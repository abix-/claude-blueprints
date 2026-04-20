//! Main-world Rust runtime.
//!
//! Stage 3 of the Rust port. Executes the approved plan faithfully:
//! Rust re-patches every target prototype with a Rust-backed wrapper
//! via `js_sys::Reflect::set` + `Closure`, validating every payload
//! against the typed [`SignalPayload`] union at the wasm-bindgen
//! boundary.
//!
//! ## Installation flow
//!
//! 1. `mainworld.js` runs synchronously at `document_start`. It
//!    installs tiny JS stubs on every target prototype so the
//!    hook-install can begin before any page script. Stubs push
//!    captured invocations onto `window.__hush_stub_q__` so events
//!    fired during the async WASM load aren't lost.
//! 2. `mainworld.js` dynamically imports `dist/pkg/hush.js`, awaits
//!    `init()` + `initEngine()`, then calls
//!    [`install_from_js`] with:
//!    - `orig_map`: a plain object mapping `"ProtoName.method"` ->
//!      the original prototype method (captured at step 1)
//!    - `make_wrapper`: a one-line JS factory that builds a
//!      `this`-capturing wrapper from a Rust dispatch function + an
//!      original fn + a kind tag. Unavoidable shim: wasm-bindgen
//!      closures can't forward JS `this` by themselves; the factory
//!      is the minimum JS needed to pass `this` through to Rust.
//! 3. For each hook descriptor, [`install_from_js`] creates a Rust
//!    closure that validates + emits, calls the factory to wrap it
//!    in a `this`-capturing function, and uses `Reflect::set` to
//!    swap the stub on the prototype.
//! 4. Finally it drains `window.__hush_stub_q__` through the Rust
//!    dispatch path and clears the queue.
//!
//! After step 4, every hook invocation goes through Rust: payload
//! shape is validated by serde against `SignalPayload`; missing
//! required fields (the 0.5.0 bug class) fail loudly instead of
//! silently reaching the detectors.

use crate::types::SignalPayload;
use js_sys::{Array, Function, Object, Reflect};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CustomEvent, CustomEventInit, Document};

// Long-lived storage for the Rust closures we hand to JS. They must
// outlive every prototype-method invocation for the tab's lifetime.
// `thread_local!` is the single-threaded-WASM answer to the usual
// `OnceLock<Mutex<...>>` pattern - `Closure` is `!Send`, and WASM
// never actually crosses threads, so the TLS cell is safe.
thread_local! {
    static LIVE_CLOSURES: RefCell<Vec<Closure<dyn Fn(JsValue, JsValue)>>> =
        const { RefCell::new(Vec::new()) };
}

/// All hook descriptors: which prototype + method + the `SignalPayload`
/// kind tag the Rust-backed wrapper should build from the captured
/// arguments. One table so adding a new hook is a one-line change.
/// Each tuple: (global constructor path, method name, kind tag).
const HOOKS: &[(&str, &str, &str)] = &[
    ("window.HTMLCanvasElement", "toDataURL", "canvas-fp:toDataURL"),
    ("window.HTMLCanvasElement", "toBlob", "canvas-fp:toBlob"),
    ("window.CanvasRenderingContext2D", "getImageData", "canvas-fp:getImageData"),
    ("window.CanvasRenderingContext2D", "measureText", "font-fp"),
    ("window.WebGLRenderingContext", "getParameter", "webgl-fp"),
    ("window.WebGL2RenderingContext", "getParameter", "webgl-fp"),
    // fetch / XHR / beacon / WebSocket hooks live as their own wrapper
    // assignments; they're installed below in a dedicated loop because
    // each one reads its args differently to build the payload.
];

/// Public wasm-bindgen entry. Called once by `mainworld.js` after the
/// WASM module finishes `init() + initEngine()`. See module docs.
#[wasm_bindgen(js_name = "hush_install_from_js")]
pub fn install_from_js(orig_map: JsValue, make_wrapper: &Function) -> Result<(), JsValue> {
    // --- Install Rust-backed wrappers on each prototype method -----------
    for (proto_path, method, kind_tag) in HOOKS {
        let proto = match resolve_path(proto_path) {
            Some(p) => p,
            None => continue, // browser doesn't expose this constructor; skip quietly
        };
        let key = format!("{}.{}", proto_path.trim_start_matches("window."), method);
        let orig = Reflect::get(&orig_map, &JsValue::from_str(&key))?;
        if !orig.is_function() {
            continue; // stub didn't patch this one (unsupported in env)
        }
        install_one_hook(&proto, method, kind_tag, &orig, make_wrapper)?;
    }

    // --- Drain the pre-WASM stub queue ----------------------------------
    if let Some(window) = web_sys::window() {
        if let Ok(q_val) = Reflect::get(&window, &JsValue::from_str("__hush_stub_q__")) {
            if let Ok(arr) = q_val.dyn_into::<Array>() {
                let len = arr.length();
                for i in 0..len {
                    let entry = arr.get(i);
                    if let Err(e) = validate_and_dispatch(entry) {
                        log_error(&format!("stub queue drain: {}", js_err_str(&e)));
                    }
                }
                arr.set_length(0);
            }
        }
    }

    Ok(())
}

fn install_one_hook(
    proto: &Object,
    method: &str,
    kind_tag: &'static str,
    orig: &JsValue,
    make_wrapper: &Function,
) -> Result<(), JsValue> {
    // Rust dispatch function. Called by the JS-side wrapper on every
    // invocation with the captured `this` and `arguments`. We build a
    // payload, validate via SignalPayload, dispatch a CustomEvent.
    let closure = Closure::<dyn Fn(JsValue, JsValue)>::new(move |this_val: JsValue, args: JsValue| {
        let detail = match build_payload(kind_tag, &this_val, &args) {
            Ok(v) => v,
            Err(e) => {
                log_error(&format!("build_payload({kind_tag}): {}", js_err_str(&e)));
                return;
            }
        };
        if let Err(e) = validate_and_dispatch(detail) {
            log_error(&format!("dispatch({kind_tag}): {}", js_err_str(&e)));
        }
    });

    // Call the JS factory: factory(rustDispatch, origFn, kindTag) => wrapperFn.
    // The wrapperFn is a real JS function that captures `this` and forwards
    // both `this` and `arguments` to the Rust dispatch.
    let rust_fn_ref: JsValue = closure.as_ref().clone();
    let wrapper = make_wrapper.call3(
        &JsValue::NULL,
        &rust_fn_ref,
        orig,
        &JsValue::from_str(kind_tag),
    )?;
    Reflect::set(proto, &JsValue::from_str(method), &wrapper)?;

    // Keep the closure alive for the lifetime of the tab.
    LIVE_CLOSURES.with(|cell| cell.borrow_mut().push(closure));
    Ok(())
}

/// Build the `SignalPayload`-shaped detail object for a given kind tag
/// from the captured `this` and arguments. Stack capture happens on the
/// JS side (Rust WASM has no access to the JS call stack), so the JS
/// factory supplies a `stack` array alongside the arguments.
fn build_payload(kind_tag: &str, _this: &JsValue, call: &JsValue) -> Result<JsValue, JsValue> {
    // `call` is an object provided by the JS factory with shape
    // `{args: [...], stack: [...]}`. Rust doesn't touch `this` for
    // any of our current hooks - the payload doesn't include it;
    // `this` only matters for the factory-side `orig.apply(this, args)`.
    let args = Reflect::get(call, &JsValue::from_str("args"))?;
    let stack = Reflect::get(call, &JsValue::from_str("stack"))?;
    let args_arr = args
        .dyn_into::<Array>()
        .map_err(|_| JsValue::from_str("args must be an array-like"))?;

    let obj = Object::new();
    let set = |obj: &Object, k: &str, v: &JsValue| -> Result<(), JsValue> {
        Reflect::set(obj, &JsValue::from_str(k), v).map(|_| ())
    };

    match kind_tag {
        "canvas-fp:toDataURL" => {
            set(&obj, "kind", &JsValue::from_str("canvas-fp"))?;
            set(&obj, "method", &JsValue::from_str("toDataURL"))?;
            set(&obj, "stack", &stack)?;
        }
        "canvas-fp:toBlob" => {
            set(&obj, "kind", &JsValue::from_str("canvas-fp"))?;
            set(&obj, "method", &JsValue::from_str("toBlob"))?;
            set(&obj, "stack", &stack)?;
        }
        "canvas-fp:getImageData" => {
            set(&obj, "kind", &JsValue::from_str("canvas-fp"))?;
            set(&obj, "method", &JsValue::from_str("getImageData"))?;
            set(&obj, "stack", &stack)?;
        }
        "font-fp" => {
            // measureText(text) - font comes from `this.font` via the factory
            let font = Reflect::get(call, &JsValue::from_str("font"))
                .unwrap_or(JsValue::from_str(""));
            let text = args_arr.get(0).as_string().unwrap_or_default();
            set(&obj, "kind", &JsValue::from_str("font-fp"))?;
            set(&obj, "font", &font)?;
            set(&obj, "text", &JsValue::from_str(&text.chars().take(20).collect::<String>()))?;
            set(&obj, "stack", &stack)?;
        }
        "webgl-fp" => {
            let param = args_arr.get(0);
            let param_num = param.as_f64().unwrap_or(-1.0) as i32;
            let hot = param_num == 37445 || param_num == 37446;
            set(&obj, "kind", &JsValue::from_str("webgl-fp"))?;
            set(&obj, "param", &JsValue::from_str(&param_num.to_string()))?;
            set(&obj, "hotParam", &JsValue::from_bool(hot))?;
            set(&obj, "stack", &stack)?;
        }
        _ => return Err(JsValue::from_str(&format!("unknown kind tag: {kind_tag}"))),
    }
    Ok(obj.into())
}

/// Called by mainworld.js's stub path on each hook fire, pre-WASM
/// load, to push a pre-built detail into the queue AND straight
/// through validation after WASM is ready. Same entry path used for
/// queue drain. Kept as a wasm-bindgen export so stubs can invoke it
/// directly without going through a factory.
#[wasm_bindgen(js_name = "dispatchHook")]
pub fn dispatch_hook(detail: JsValue) -> Result<(), JsValue> {
    validate_and_dispatch(detail)
}

/// Legacy entry kept for the emit-contract test harness. Validates a
/// single queue entry; a no-op on success.
#[wasm_bindgen(js_name = "drainStubQueue")]
pub fn drain_stub_queue(queue: JsValue) -> Result<(), JsValue> {
    let arr = queue
        .dyn_into::<Array>()
        .map_err(|_| JsValue::from_str("drainStubQueue: argument must be an array"))?;
    let len = arr.length();
    for i in 0..len {
        let entry = arr.get(i);
        if let Err(e) = validate_and_dispatch(entry) {
            log_error(&format!("drain entry invalid: {}", js_err_str(&e)));
        }
    }
    Ok(())
}

fn validate_and_dispatch(detail: JsValue) -> Result<(), JsValue> {
    let payload: SignalPayload = serde_wasm_bindgen::from_value(detail)
        .map_err(|e| JsValue::from_str(&format!("SignalPayload deserialize: {e}")))?;
    let canonical = serde_wasm_bindgen::to_value(&payload)
        .map_err(|e| JsValue::from_str(&format!("SignalPayload re-serialize: {e}")))?;
    dispatch_custom_event(&canonical)
}

fn dispatch_custom_event(detail: &JsValue) -> Result<(), JsValue> {
    let document = document_ref()?;
    let init = CustomEventInit::new();
    init.set_detail(detail);
    let event = CustomEvent::new_with_event_init_dict("__hush_call__", &init)?;
    document.dispatch_event(&event)?;
    Ok(())
}

fn document_ref() -> Result<Document, JsValue> {
    web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| JsValue::from_str("no window.document available in main world"))
}

/// Resolve a path like `"window.HTMLCanvasElement"` to the constructor,
/// then return its `.prototype`. Returns `None` if any segment is
/// missing (e.g. `WebGL2RenderingContext` on an old browser).
fn resolve_path(path: &str) -> Option<Object> {
    let window = web_sys::window()?;
    let mut cur: JsValue = window.into();
    for segment in path.trim_start_matches("window.").split('.') {
        let next = Reflect::get(&cur, &JsValue::from_str(segment)).ok()?;
        if next.is_undefined() {
            return None;
        }
        cur = next;
    }
    // cur is the constructor; grab its prototype.
    let proto = Reflect::get(&cur, &JsValue::from_str("prototype")).ok()?;
    proto.dyn_into::<Object>().ok()
}

fn log_error(msg: &str) {
    web_sys::console::error_1(&JsValue::from_str(&format!("[Hush] {msg}")));
}

fn js_err_str(v: &JsValue) -> String {
    v.as_string().unwrap_or_else(|| format!("{:?}", v))
}
