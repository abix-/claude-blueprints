//! Thin async wrappers around `chrome.runtime.sendMessage`.
//!
//! Leptos components in `src/ui_popup.rs` use these to fetch and mutate
//! state without hopping back to JS glue. Each call:
//!
//! 1. Resolves `window.chrome.runtime.sendMessage` via `js_sys::Reflect`
//! 2. Serializes the message struct via `serde-wasm-bindgen`
//! 3. Awaits the returned Promise via `wasm-bindgen-futures::JsFuture`
//! 4. Deserializes the response into the expected type
//!
//! Errors are `JsValue` (typically strings) so they can propagate
//! through Leptos's `Action` / `spawn_local` machinery without extra
//! wrapping.

use crate::types::Suggestion;
use js_sys::{Promise, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

fn send_fn() -> Result<(JsValue, js_sys::Function), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let chrome = Reflect::get(&window, &JsValue::from_str("chrome"))?;
    let runtime = Reflect::get(&chrome, &JsValue::from_str("runtime"))?;
    let send = Reflect::get(&runtime, &JsValue::from_str("sendMessage"))?;
    let func: js_sys::Function = send
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.runtime.sendMessage is not a function"))?;
    Ok((runtime, func))
}

/// Generic: send a message object, await the reply, deserialize into R.
async fn send<M, R>(msg: &M) -> Result<R, JsValue>
where
    M: Serialize + ?Sized,
    R: for<'de> Deserialize<'de>,
{
    let payload =
        serde_wasm_bindgen::to_value(msg).map_err(|e| JsValue::from_str(&format!("sendMessage serialize: {e}")))?;
    let (runtime, func) = send_fn()?;
    let promise: Promise = func
        .call1(&runtime, &payload)?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.runtime.sendMessage did not return a Promise"))?;
    let reply = JsFuture::from(promise).await?;
    serde_wasm_bindgen::from_value(reply)
        .map_err(|e| JsValue::from_str(&format!("sendMessage deserialize: {e}")))
}

/// Request / response types for the three suggestion-mutating messages
/// the popup's action row cares about, plus the re-fetch after each.

#[derive(Serialize)]
struct AcceptSuggestionMsg<'a> {
    #[serde(rename = "type")]
    type_: &'static str,
    hostname: &'a str,
    layer: &'a str,
    value: &'a str,
}

#[derive(Serialize)]
struct DismissSuggestionMsg<'a> {
    #[serde(rename = "type")]
    type_: &'static str,
    #[serde(rename = "tabId")]
    tab_id: i32,
    key: &'a str,
}

#[derive(Serialize)]
struct AllowlistAddMsg<'a> {
    #[serde(rename = "type")]
    type_: &'static str,
    key: &'a str,
}

#[derive(Serialize)]
struct GetSuggestionsMsg {
    #[serde(rename = "type")]
    type_: &'static str,
    #[serde(rename = "tabId")]
    tab_id: i32,
}

#[derive(Deserialize, Default)]
pub struct GetSuggestionsResp {
    #[serde(default)]
    pub suggestions: Vec<Suggestion>,
    /// pageHost field in the wire response. Popup doesn't read it; kept
    /// here so the JSON shape stays validated end-to-end.
    #[serde(rename = "pageHost", default)]
    #[allow(dead_code)]
    pub page_host: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OkResp {
    ok: bool,
}

/// POST the accept-suggestion action, then refetch.
pub async fn accept_suggestion(
    hostname: &str,
    layer: &str,
    value: &str,
) -> Result<(), JsValue> {
    let _: OkResp = send(&AcceptSuggestionMsg {
        type_: "hush:accept-suggestion",
        hostname,
        layer,
        value,
    })
    .await?;
    Ok(())
}

pub async fn dismiss_suggestion(tab_id: i32, key: &str) -> Result<(), JsValue> {
    let _: OkResp = send(&DismissSuggestionMsg {
        type_: "hush:dismiss-suggestion",
        tab_id,
        key,
    })
    .await?;
    Ok(())
}

pub async fn allowlist_suggestion(key: &str) -> Result<(), JsValue> {
    let _: OkResp = send(&AllowlistAddMsg {
        type_: "hush:allowlist-add-suggestion",
        key,
    })
    .await?;
    Ok(())
}

pub async fn get_suggestions(tab_id: i32) -> Result<Vec<Suggestion>, JsValue> {
    let resp: GetSuggestionsResp = send(&GetSuggestionsMsg {
        type_: "hush:get-suggestions",
        tab_id,
    })
    .await?;
    Ok(resp.suggestions)
}

/// Merge a single boolean field into `chrome.storage.local["options"]`.
/// Read-modify-write so other option fields are preserved. Errors
/// propagate the underlying `JsValue` so callers can surface them.
pub async fn set_option_bool(key: &str, value: bool) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let chrome = Reflect::get(&window, &JsValue::from_str("chrome"))?;
    let storage = Reflect::get(&chrome, &JsValue::from_str("storage"))?;
    let local = Reflect::get(&storage, &JsValue::from_str("local"))?;

    // Read current options object so we can merge.
    let get_fn: js_sys::Function = Reflect::get(&local, &JsValue::from_str("get"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.get is not a function"))?;
    let get_promise: Promise = get_fn
        .call1(&local, &JsValue::from_str("options"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.get did not return a Promise"))?;
    let reply = JsFuture::from(get_promise).await?;
    let opts = Reflect::get(&reply, &JsValue::from_str("options"))
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null())
        .unwrap_or_else(|| js_sys::Object::new().into());
    Reflect::set(
        &opts,
        &JsValue::from_str(key),
        &JsValue::from_bool(value),
    )?;

    // Write it back via chrome.storage.local.set({options: {...}}).
    let set_payload = js_sys::Object::new();
    Reflect::set(&set_payload, &JsValue::from_str("options"), &opts)?;
    let set_fn: js_sys::Function = Reflect::get(&local, &JsValue::from_str("set"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.set is not a function"))?;
    let set_promise: Promise = set_fn
        .call1(&local, &set_payload.into())?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.set did not return a Promise"))?;
    JsFuture::from(set_promise).await?;
    Ok(())
}

/// Enable the behavioral-suggestion detector. Thin wrapper over
/// [`set_option_bool`] so popup callers read naturally.
pub async fn enable_detector() -> Result<(), JsValue> {
    set_option_bool("suggestionsEnabled", true).await
}

/// Read the full `config` object from `chrome.storage.local` and return
/// it as a pretty-printed JSON string. Used by the options-page Export
/// button to seed the download.
pub async fn get_config_json() -> Result<String, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let chrome = Reflect::get(&window, &JsValue::from_str("chrome"))?;
    let storage = Reflect::get(&chrome, &JsValue::from_str("storage"))?;
    let local = Reflect::get(&storage, &JsValue::from_str("local"))?;
    let get_fn: js_sys::Function = Reflect::get(&local, &JsValue::from_str("get"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.get is not a function"))?;
    let get_promise: Promise = get_fn
        .call1(&local, &JsValue::from_str("config"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.get did not return a Promise"))?;
    let reply = JsFuture::from(get_promise).await?;
    let config = Reflect::get(&reply, &JsValue::from_str("config"))
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null())
        .unwrap_or_else(|| js_sys::Object::new().into());
    // `JSON.stringify(obj, null, 2)` for pretty-printed output.
    let json = js_sys::JSON::stringify_with_replacer_and_space(
        &config,
        &JsValue::NULL,
        &JsValue::from_f64(2.0),
    )?;
    Ok(json.as_string().unwrap_or_default())
}

/// Write an allowlist triple (iframes, overlays, suggestion keys) into
/// `chrome.storage.local["allowlist"]`. Replaces any previous value
/// wholesale - matches the JS options page's "save everything from
/// the textareas" semantics.
pub async fn set_allowlist(
    iframes: Vec<String>,
    overlays: Vec<String>,
    suggestions: Vec<String>,
) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let chrome = Reflect::get(&window, &JsValue::from_str("chrome"))?;
    let storage = Reflect::get(&chrome, &JsValue::from_str("storage"))?;
    let local = Reflect::get(&storage, &JsValue::from_str("local"))?;

    let allowlist = js_sys::Object::new();
    let i = js_sys::Array::new();
    for s in &iframes {
        i.push(&JsValue::from_str(s));
    }
    let o = js_sys::Array::new();
    for s in &overlays {
        o.push(&JsValue::from_str(s));
    }
    let s = js_sys::Array::new();
    for v in &suggestions {
        s.push(&JsValue::from_str(v));
    }
    Reflect::set(&allowlist, &JsValue::from_str("iframes"), &i)?;
    Reflect::set(&allowlist, &JsValue::from_str("overlays"), &o)?;
    Reflect::set(&allowlist, &JsValue::from_str("suggestions"), &s)?;

    let payload = js_sys::Object::new();
    Reflect::set(&payload, &JsValue::from_str("allowlist"), &allowlist)?;

    let set_fn: js_sys::Function = Reflect::get(&local, &JsValue::from_str("set"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.set is not a function"))?;
    let set_promise: Promise = set_fn
        .call1(&local, &payload.into())?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.set did not return a Promise"))?;
    JsFuture::from(set_promise).await?;
    Ok(())
}

/// Fetch the shipped `allowlist.defaults.json` via
/// `chrome.runtime.getURL` + `fetch`. Returns the three-field triple
/// as owned `Vec<String>`s so the caller can drop them straight into
/// signals. Any missing field degrades to an empty list.
pub async fn get_default_allowlist() -> Result<(Vec<String>, Vec<String>, Vec<String>), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let chrome = Reflect::get(&window, &JsValue::from_str("chrome"))?;
    let runtime = Reflect::get(&chrome, &JsValue::from_str("runtime"))?;
    let get_url_fn: js_sys::Function = Reflect::get(&runtime, &JsValue::from_str("getURL"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.runtime.getURL is not a function"))?;
    let url = get_url_fn
        .call1(&runtime, &JsValue::from_str("allowlist.defaults.json"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("chrome.runtime.getURL returned non-string"))?;

    let fetch_fn: js_sys::Function = Reflect::get(&window, &JsValue::from_str("fetch"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("window.fetch is not a function"))?;
    let fetch_promise: Promise = fetch_fn
        .call1(&window, &JsValue::from_str(&url))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("fetch did not return a Promise"))?;
    let response = JsFuture::from(fetch_promise).await?;
    let json_fn: js_sys::Function = Reflect::get(&response, &JsValue::from_str("json"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("Response.json is not a function"))?;
    let json_promise: Promise = json_fn
        .call0(&response)?
        .dyn_into()
        .map_err(|_| JsValue::from_str("Response.json did not return a Promise"))?;
    let seed = JsFuture::from(json_promise).await?;

    fn to_vec(value: &JsValue, key: &str) -> Vec<String> {
        let arr = match Reflect::get(value, &JsValue::from_str(key)) {
            Ok(v) if !v.is_undefined() && !v.is_null() => v,
            _ => return Vec::new(),
        };
        let arr: js_sys::Array = match arr.dyn_into() {
            Ok(a) => a,
            Err(_) => return Vec::new(),
        };
        arr.iter()
            .filter_map(|v| v.as_string())
            .collect()
    }
    Ok((
        to_vec(&seed, "iframes"),
        to_vec(&seed, "overlays"),
        to_vec(&seed, "suggestions"),
    ))
}

/// Write a full config object to `chrome.storage.local["config"]`.
/// Accepts any Serialize type so the ui_options editor can hand in a
/// typed `Config` (IndexMap) without reshaping it to JS first.
pub async fn set_config<C: serde::Serialize + ?Sized>(config: &C) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let chrome = Reflect::get(&window, &JsValue::from_str("chrome"))?;
    let storage = Reflect::get(&chrome, &JsValue::from_str("storage"))?;
    let local = Reflect::get(&storage, &JsValue::from_str("local"))?;
    let set_fn: js_sys::Function = Reflect::get(&local, &JsValue::from_str("set"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.set is not a function"))?;
    let js_config = serde_wasm_bindgen::to_value(config)
        .map_err(|e| JsValue::from_str(&format!("set_config serialize: {e}")))?;
    let payload = js_sys::Object::new();
    Reflect::set(&payload, &JsValue::from_str("config"), &js_config)?;
    let set_promise: Promise = set_fn
        .call1(&local, &payload.into())?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.set did not return a Promise"))?;
    JsFuture::from(set_promise).await?;
    Ok(())
}

/// Parse a JSON string as a config object (top-level must be a
/// non-array object; arbitrary site keys below). Writes the parsed
/// value into `chrome.storage.local["config"]`. Rejects the write
/// with a descriptive error if the top-level shape is wrong - same
/// validation the old JS handler did.
pub async fn set_config_from_json(json: &str) -> Result<(), JsValue> {
    let parsed = js_sys::JSON::parse(json)
        .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {:?}", e)))?;
    if !parsed.is_object() || parsed.is_null() || js_sys::Array::is_array(&parsed) {
        return Err(JsValue::from_str(
            "Config must be a JSON object (keys are domain names).",
        ));
    }

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let chrome = Reflect::get(&window, &JsValue::from_str("chrome"))?;
    let storage = Reflect::get(&chrome, &JsValue::from_str("storage"))?;
    let local = Reflect::get(&storage, &JsValue::from_str("local"))?;
    let set_fn: js_sys::Function = Reflect::get(&local, &JsValue::from_str("set"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.set is not a function"))?;
    let payload = js_sys::Object::new();
    Reflect::set(&payload, &JsValue::from_str("config"), &parsed)?;
    let set_promise: Promise = set_fn
        .call1(&local, &payload.into())?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.set did not return a Promise"))?;
    JsFuture::from(set_promise).await?;
    Ok(())
}

/// Fetch the shipped `sites.json` seed (via `chrome.runtime.getURL`) and
/// write it to `chrome.storage.local["config"]`, replacing whatever's
/// there. Used by the options-page "Reset to defaults" button.
pub async fn reset_config_to_defaults() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let chrome = Reflect::get(&window, &JsValue::from_str("chrome"))?;

    // chrome.runtime.getURL("sites.json") -> extension-scoped URL string.
    let runtime = Reflect::get(&chrome, &JsValue::from_str("runtime"))?;
    let get_url_fn: js_sys::Function = Reflect::get(&runtime, &JsValue::from_str("getURL"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.runtime.getURL is not a function"))?;
    let url = get_url_fn
        .call1(&runtime, &JsValue::from_str("sites.json"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("chrome.runtime.getURL returned non-string"))?;

    // fetch(url) -> Response -> json().
    let fetch_fn: js_sys::Function = Reflect::get(&window, &JsValue::from_str("fetch"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("window.fetch is not a function"))?;
    let fetch_promise: Promise = fetch_fn
        .call1(&window, &JsValue::from_str(&url))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("fetch did not return a Promise"))?;
    let response = JsFuture::from(fetch_promise).await?;
    let json_fn: js_sys::Function = Reflect::get(&response, &JsValue::from_str("json"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("Response.json is not a function"))?;
    let json_promise: Promise = json_fn
        .call0(&response)?
        .dyn_into()
        .map_err(|_| JsValue::from_str("Response.json did not return a Promise"))?;
    let seed = JsFuture::from(json_promise).await?;

    // chrome.storage.local.set({config: seed}).
    let storage = Reflect::get(&chrome, &JsValue::from_str("storage"))?;
    let local = Reflect::get(&storage, &JsValue::from_str("local"))?;
    let set_fn: js_sys::Function = Reflect::get(&local, &JsValue::from_str("set"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.set is not a function"))?;
    let payload = js_sys::Object::new();
    Reflect::set(&payload, &JsValue::from_str("config"), &seed)?;
    let set_promise: Promise = set_fn
        .call1(&local, &payload.into())?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.storage.local.set did not return a Promise"))?;
    JsFuture::from(set_promise).await?;
    Ok(())
}

/// Ask the content script in tab `tab_id` to run one behavioral scan
/// immediately. Message shape matches the JS popup's existing
/// `chrome.tabs.sendMessage(tabId, { type: "hush:scan-once" })` call.
/// Errors when the tab is closed or the content script hasn't loaded.
pub async fn scan_once(tab_id: i32) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let chrome = Reflect::get(&window, &JsValue::from_str("chrome"))?;
    let tabs = Reflect::get(&chrome, &JsValue::from_str("tabs"))?;
    let send_fn: js_sys::Function = Reflect::get(&tabs, &JsValue::from_str("sendMessage"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.tabs.sendMessage is not a function"))?;
    let msg = js_sys::Object::new();
    Reflect::set(&msg, &JsValue::from_str("type"), &JsValue::from_str("hush:scan-once"))?;
    let promise: Promise = send_fn
        .call2(&tabs, &JsValue::from_f64(tab_id as f64), &msg.into())?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.tabs.sendMessage did not return a Promise"))?;
    JsFuture::from(promise).await?;
    Ok(())
}
