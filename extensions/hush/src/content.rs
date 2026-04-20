//! Content-script Rust runtime (Stage 5).
//!
//! The former JS content script was a ~460-line module that handled
//! three concerns simultaneously:
//!
//! 1. **Layer application** - apply Remove (delete matching DOM
//!    nodes) + Hide (inject `display: none !important` CSS) on every
//!    matched site.
//! 2. **Behavioral detection** - scan hidden iframes, sticky
//!    overlays, and the `PerformanceObserver` resource stream; emit
//!    the results back to the service worker so the Rust engine can
//!    compute suggestions.
//! 3. **Main-world bridge** - buffer `__hush_call__` events dispatched
//!    from `mainworld.js` and flush them in batches.
//!
//! All three are now here in Rust. The JS side survives as a 20-line
//! bootstrap that reads three `chrome.storage.local` keys and hands
//! them to [`hush_content_main`].

use crate::types::{
    Allowlist, Config, IframeHit, JsCall, Resource, SignalPayload, StickyHit, StickyRect,
};
use js_sys::{Function, Object, Reflect};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, HtmlElement, HtmlIFrameElement, HtmlStyleElement, MutationObserver,
    MutationObserverInit, PerformanceEntry, PerformanceObserver, PerformanceObserverEntryList,
    PerformanceObserverInit, PerformanceResourceTiming, Window,
};

// ---------------------------------------------------------------------------
// Constants (mirror the JS content script).

const MAX_LOCAL_REMOVED: usize = 200;
const MAX_BUFFERED_RESOURCES: usize = 500;
const MAX_LOCAL_JS_CALLS: usize = 300;
const JS_CALL_SEND_DELAY_MS: i32 = 500;
const STATS_SEND_DELAY_MS: i32 = 500;
const SCAN_IDLE_DELAY_MS: i32 = 5000;

// ---------------------------------------------------------------------------
// Per-tab content state. Held in a `thread_local!` RefCell because
// every closure we hand to JS (MutationObserver, PerformanceObserver,
// message handler, event listener) needs mutable access.

thread_local! {
    static STATE: RefCell<Option<ContentState>> = const { RefCell::new(None) };

    // Pin closures for their tab lifetime. WASM is single-threaded so
    // TLS + RefCell is the right primitive.
    static LIVE_CLOSURES: RefCell<Vec<Box<dyn std::any::Any>>> =
        const { RefCell::new(Vec::new()) };
}

fn keep_closure<T: std::any::Any>(c: T) {
    LIVE_CLOSURES.with(|cell| cell.borrow_mut().push(Box::new(c)));
}

struct ContentState {
    debug: bool,
    detector_enabled: bool,
    allowlist: Allowlist,
    matched_domain: Option<String>,
    remove_selectors: Vec<String>,
    hide_selectors: Vec<String>,
    hide_counts: Vec<u32>,
    remove_counts: Vec<u32>,
    pending_removed: Vec<RemovedEvent>,
    collected_resources: Vec<Resource>,
    js_calls: Vec<JsCall>,
    js_call_timer_scheduled: bool,
    stats_timer_scheduled: bool,
}

#[derive(Debug, Clone, Serialize)]
struct RemovedEvent {
    t: String,
    selector: String,
    el: String,
}

// Snapshot handed in by the JS bootstrap.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ContentSnapshot {
    config: Config,
    options: OptionsSnapshot,
    allowlist: Allowlist,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct OptionsSnapshot {
    debug: bool,
    #[serde(rename = "suggestionsEnabled")]
    suggestions_enabled: bool,
}

// ---------------------------------------------------------------------------
// Public entry point.

/// Called once per frame by the JS content-script bootstrap after it
/// has read `chrome.storage.local`. Installs every observer + event
/// listener the old JS content script owned and runs the initial
/// Remove/Hide pass if the current hostname matches a configured
/// site. Returns `Err` (which JS ignores) only for snapshot shape
/// errors - transient web API failures log and continue.
#[wasm_bindgen(js_name = "hushContentMain")]
pub fn hush_content_main(snapshot: JsValue) -> Result<(), JsValue> {
    let snap: ContentSnapshot = serde_wasm_bindgen::from_value(snapshot)
        .map_err(|e| JsValue::from_str(&format!("hushContentMain: {e}")))?;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    let host = window.location().hostname().unwrap_or_default();
    let (matched_domain, site_cfg) = match find_config_entry(&snap.config, &host) {
        Some((k, cfg)) => (Some(k), Some(cfg.clone())),
        None => (None, None),
    };
    let remove_selectors = site_cfg.as_ref().map(|c| c.remove.clone()).unwrap_or_default();
    let hide_selectors = site_cfg.as_ref().map(|c| c.hide.clone()).unwrap_or_default();
    let remove_counts = vec![0u32; remove_selectors.len()];
    let hide_counts = vec![0u32; hide_selectors.len()];

    let debug = snap.options.debug;
    let detector_enabled = snap.options.suggestions_enabled;

    STATE.with(|s| {
        *s.borrow_mut() = Some(ContentState {
            debug,
            detector_enabled,
            allowlist: snap.allowlist,
            matched_domain: matched_domain.clone(),
            remove_selectors,
            hide_selectors,
            hide_counts,
            remove_counts,
            pending_removed: Vec::new(),
            collected_resources: Vec::new(),
            js_calls: Vec::new(),
            js_call_timer_scheduled: false,
            stats_timer_scheduled: false,
        });
    });

    install_js_call_listener(&document);
    install_message_listener();
    install_resource_observer();

    // Matched-site behavior: initial Remove/Hide pass + Mutation
    // observer + send initial stats.
    if matched_domain.is_some() {
        log(&format!(
            "{} - matched: {:?}",
            host,
            matched_domain.as_deref().unwrap_or("?")
        ));
        pass();
        inject_hide_css(&document);
        install_mutation_observer(&document);
        send_stats();
    } else if STATE.with(|s| s.borrow().as_ref().map(|c| c.debug).unwrap_or(false)) {
        log(&format!("{} - no matching site config", host));
    }

    // Continuous scans only when detector is enabled.
    if detector_enabled {
        if document.ready_state() == "loading" {
            let cb = Closure::<dyn Fn()>::new(|| run_scan("dom-content-loaded"));
            let _ = document.add_event_listener_with_callback_and_add_event_listener_options(
                "DOMContentLoaded",
                cb.as_ref().unchecked_ref(),
                #[allow(deprecated)]
                web_sys::AddEventListenerOptions::new().once(true),
            );
            keep_closure(cb);
        } else {
            run_scan("dom-content-loaded");
        }
        set_timeout(
            || run_scan("post-load-idle"),
            SCAN_IDLE_DELAY_MS,
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Config matching.

fn find_config_entry<'a>(
    config: &'a Config,
    host: &str,
) -> Option<(String, &'a crate::types::SiteConfig)> {
    if let Some(cfg) = config.get(host) {
        return Some((host.to_string(), cfg));
    }
    for (key, cfg) in config.iter() {
        if host == key || host.ends_with(&format!(".{key}")) {
            return Some((key.clone(), cfg));
        }
    }
    None
}

fn host_of(url: &str) -> String {
    let base = web_sys::window()
        .and_then(|w| w.location().href().ok())
        .unwrap_or_default();
    url::Url::parse(url)
        .ok()
        .or_else(|| {
            url::Url::parse(&base)
                .ok()
                .and_then(|b| b.join(url).ok())
        })
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Remove + Hide layer application.

fn pass() {
    let a = apply_remove();
    let b = recount_hide();
    if a || b {
        schedule_stats_send();
    }
}

fn apply_remove() -> bool {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return false;
    };
    let mut any_changed = false;
    let selectors: Vec<String> =
        STATE.with(|s| s.borrow().as_ref().map(|c| c.remove_selectors.clone()).unwrap_or_default());
    for (idx, sel) in selectors.iter().enumerate() {
        let nodes = match document.query_selector_all(sel) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let len = nodes.length();
        if len == 0 {
            continue;
        }
        let mut removed_descriptions = Vec::with_capacity(len as usize);
        let now = iso_now();
        for i in 0..len {
            let Some(node) = nodes.get(i) else { continue };
            let Ok(element) = node.dyn_into::<Element>() else {
                continue;
            };
            removed_descriptions.push(describe_element(&element));
            element.remove();
        }
        STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                if idx < state.remove_counts.len() {
                    state.remove_counts[idx] =
                        state.remove_counts[idx].saturating_add(len as u32);
                }
                for desc in removed_descriptions {
                    state.pending_removed.push(RemovedEvent {
                        t: now.clone(),
                        selector: sel.clone(),
                        el: desc,
                    });
                }
                if state.pending_removed.len() > MAX_LOCAL_REMOVED {
                    let drop = state.pending_removed.len() - MAX_LOCAL_REMOVED;
                    state.pending_removed.drain(..drop);
                }
            }
        });
        any_changed = true;
    }
    any_changed
}

fn inject_hide_css(document: &Document) {
    let selectors: Vec<String> =
        STATE.with(|s| s.borrow().as_ref().map(|c| c.hide_selectors.clone()).unwrap_or_default());
    if selectors.is_empty() {
        return;
    }
    let css = selectors
        .iter()
        .map(|s| format!("{s} {{ display: none !important; }}"))
        .collect::<Vec<_>>()
        .join("\n");
    let Ok(style_el) = document.create_element("style") else {
        return;
    };
    let Ok(style) = style_el.dyn_into::<HtmlStyleElement>() else {
        return;
    };
    style.set_text_content(Some(&css));
    let _ = style.set_attribute("data-hush", "hide");
    let parent: web_sys::Element = document
        .head()
        .map(Into::into)
        .unwrap_or_else(|| document.document_element().unwrap());
    let _ = parent.append_child(&style);
}

fn recount_hide() -> bool {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return false;
    };
    let selectors: Vec<String> =
        STATE.with(|s| s.borrow().as_ref().map(|c| c.hide_selectors.clone()).unwrap_or_default());
    let mut any_changed = false;
    for (idx, sel) in selectors.iter().enumerate() {
        let Ok(nodes) = document.query_selector_all(sel) else {
            continue;
        };
        let n = nodes.length();
        STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                if idx < state.hide_counts.len() && state.hide_counts[idx] != n {
                    state.hide_counts[idx] = n;
                    any_changed = true;
                }
            }
        });
    }
    any_changed
}

/// Rich human-readable description of a DOM element. Mirrors the JS
/// `describeElement`: tag + first two classes + id, then up to three
/// "interesting" attributes, then a text snippet.
fn describe_element(el: &Element) -> String {
    let tag = el.tag_name().to_lowercase();
    let classes = el
        .get_attribute("class")
        .unwrap_or_default()
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
    let id = el.id();
    let mut base = tag.clone();
    if !classes.is_empty() {
        base.push('.');
        base.push_str(&classes);
    }
    if !id.is_empty() {
        base.push('#');
        base.push_str(&id);
    }

    const INTERESTING: &[&str] = &[
        "name",
        "data-testid",
        "data-post-id",
        "aria-label",
        "post-title",
        "post-type",
        "subreddit-prefixed-name",
        "author",
        "post-id",
        "data-promoted",
        "data-ad",
        "data-ad-type",
        "src",
        "href",
        "title",
        "alt",
    ];
    let mut attr_parts: Vec<String> = Vec::new();
    for attr in INTERESTING {
        let Some(mut v) = el.get_attribute(attr) else {
            continue;
        };
        if v.is_empty() {
            continue;
        }
        if v.len() > 70 {
            v.truncate(67);
            v.push_str("...");
        }
        attr_parts.push(format!(r#"{}="{}""#, attr, v));
        if attr_parts.len() >= 3 {
            break;
        }
    }

    let text_snippet = el
        .text_content()
        .map(|t| {
            let mut s: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
            if s.len() > 80 {
                s.truncate(77);
                s.push_str("...");
            }
            s
        })
        .unwrap_or_default();

    let mut parts = vec![base];
    if !attr_parts.is_empty() {
        parts.push(attr_parts.join(" "));
    }
    if !text_snippet.is_empty() {
        parts.push(format!(r#""{}""#, text_snippet));
    }
    parts.join("  |  ")
}

// ---------------------------------------------------------------------------
// Behavioral detector: iframe + sticky + resource scans.

fn scan_hidden_iframes(document: &Document) -> Vec<IframeHit> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return Vec::new(),
    };
    let vw = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    let vh = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);

    let Ok(frames) = document.query_selector_all("iframe") else {
        return Vec::new();
    };
    let mut hits = Vec::new();
    for i in 0..frames.length() {
        let Some(node) = frames.get(i) else { continue };
        let Ok(frame) = node.dyn_into::<HtmlIFrameElement>() else {
            continue;
        };
        let Ok(Some(cs)) = window.get_computed_style(&frame) else {
            continue;
        };
        let rect = frame.get_bounding_client_rect();
        let mut reasons: Vec<String> = Vec::new();
        if cs.get_property_value("display").unwrap_or_default() == "none" {
            reasons.push("display:none".into());
        }
        if cs.get_property_value("visibility").unwrap_or_default() == "hidden" {
            reasons.push("visibility:hidden".into());
        }
        if cs
            .get_property_value("opacity")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0)
            == 0.0
        {
            reasons.push("opacity:0".into());
        }
        if rect.width() <= 1.0 || rect.height() <= 1.0 {
            reasons.push("1x1 size".into());
        }
        if rect.right() < 0.0 || rect.bottom() < 0.0 || rect.left() > vw || rect.top() > vh {
            reasons.push("offscreen".into());
        }
        if reasons.is_empty() {
            continue;
        }
        let src = frame.src();
        let src = if src.is_empty() {
            frame.get_attribute("src").unwrap_or_default()
        } else {
            src
        };
        let mut outer = frame.outer_html();
        if outer.len() > 300 {
            outer.truncate(300);
        }
        hits.push(IframeHit {
            host: host_of(&src),
            src,
            reasons,
            width: rect.width() as i64,
            height: rect.height() as i64,
            outer_html_preview: outer,
            reporter_frame: None,
        });
    }
    hits
}

fn scan_sticky_overlays(document: &Document) -> Vec<StickyHit> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return Vec::new(),
    };
    let vw = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    let vh = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    let viewport_area = (vw * vh).max(1.0);
    let Ok(all) = document.query_selector_all("body *") else {
        return Vec::new();
    };
    if all.length() > 20_000 {
        return Vec::new();
    }
    let overlays = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .map(|c| c.allowlist.overlays.clone())
            .unwrap_or_default()
    });

    let mut hits = Vec::new();
    let mut checked = 0;
    for i in 0..all.length() {
        if checked >= 5000 {
            break;
        }
        checked += 1;
        let Some(node) = all.get(i) else { continue };
        let Ok(el) = node.dyn_into::<Element>() else {
            continue;
        };
        let Ok(Some(cs)) = window.get_computed_style(&el) else {
            continue;
        };
        let position = cs.get_property_value("position").unwrap_or_default();
        if position != "fixed" && position != "sticky" {
            continue;
        }
        let z_str = cs.get_property_value("zIndex").unwrap_or_default();
        let Ok(z) = z_str.parse::<i64>() else {
            continue;
        };
        if z < 100 {
            continue;
        }
        let rect = el.get_bounding_client_rect();
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            continue;
        }
        let coverage = (rect.width() * rect.height()) / viewport_area;
        if coverage < 0.25 {
            continue;
        }
        if matches_allowlist(&el, &overlays) {
            continue;
        }
        let tag = el.tag_name().to_lowercase();
        let classes = el
            .get_attribute("class")
            .unwrap_or_default()
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(".");
        let id = el.id();
        let mut selector = tag;
        if !classes.is_empty() {
            selector.push('.');
            selector.push_str(&classes);
        }
        if !id.is_empty() {
            selector.push('#');
            selector.push_str(&id);
        }
        hits.push(StickyHit {
            selector,
            coverage: (coverage * 100.0).round() as u32,
            z_index: z,
            rect: StickyRect {
                w: rect.width().round() as i64,
                h: rect.height().round() as i64,
            },
            reporter_frame: None,
        });
    }
    hits
}

fn matches_allowlist(el: &Element, selectors: &[String]) -> bool {
    for sel in selectors {
        if sel.is_empty() {
            continue;
        }
        if el.matches(sel).unwrap_or(false) {
            return true;
        }
    }
    false
}

fn run_scan(reason: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let hostname = window.location().hostname().unwrap_or_default();
    let iframes = scan_hidden_iframes(&document);
    let stickies = scan_sticky_overlays(&document);
    let resources: Vec<Resource> = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .map(|c| c.collected_resources.clone())
            .unwrap_or_default()
    });
    let n_res = resources.len();
    let n_if = iframes.len();
    let n_st = stickies.len();

    let scan = ScanMessage {
        type_: "hush:scan",
        hostname,
        observed_at: iso_now(),
        reason: reason.to_string(),
        resources,
        iframes,
        stickies,
    };
    send_message(&scan);
    log(&format!(
        "scan: {reason} resources {n_res} iframes {n_if} stickies {n_st}"
    ));
}

#[derive(Serialize)]
struct ScanMessage {
    #[serde(rename = "type")]
    type_: &'static str,
    hostname: String,
    #[serde(rename = "observedAt")]
    observed_at: String,
    reason: String,
    resources: Vec<Resource>,
    iframes: Vec<IframeHit>,
    stickies: Vec<StickyHit>,
}

// ---------------------------------------------------------------------------
// PerformanceObserver (resource stream).

fn install_resource_observer() {
    let cb = Closure::<dyn Fn(PerformanceObserverEntryList, PerformanceObserver)>::new(
        move |list: PerformanceObserverEntryList, _obs: PerformanceObserver| {
            let entries = list.get_entries();
            STATE.with(|s| {
                if let Some(state) = s.borrow_mut().as_mut() {
                    for i in 0..entries.length() {
                        let entry = entries.get(i);
                        if let Some(res) = convert_resource_entry(&entry) {
                            state.collected_resources.push(res);
                        }
                    }
                    if state.collected_resources.len() > MAX_BUFFERED_RESOURCES {
                        let drop = state.collected_resources.len() - MAX_BUFFERED_RESOURCES;
                        state.collected_resources.drain(..drop);
                    }
                }
            });
        },
    );
    let obs = match PerformanceObserver::new(cb.as_ref().unchecked_ref()) {
        Ok(o) => o,
        Err(e) => {
            log(&format!("PerformanceObserver not available: {:?}", e));
            return;
        }
    };
    let init = PerformanceObserverInit::new(
        &js_sys::Array::of1(&JsValue::from_str("resource")).into(),
    );
    init.set_buffered(true);
    let _ = obs.observe(&init);
    keep_closure(cb);
    keep_closure(obs);
}

fn convert_resource_entry(entry: &JsValue) -> Option<Resource> {
    let perf: &PerformanceEntry = entry.unchecked_ref();
    let name = perf.name();
    let host = host_of(&name);
    let initiator_type = entry
        .unchecked_ref::<PerformanceResourceTiming>()
        .initiator_type();
    let transfer_size = entry
        .unchecked_ref::<PerformanceResourceTiming>()
        .transfer_size() as i64;
    let duration = perf.duration().round() as i64;
    let start_time = perf.start_time().round() as i64;
    Some(Resource {
        url: name,
        host,
        initiator_type,
        transfer_size,
        duration,
        start_time,
        reporter_frame: None,
    })
}

// ---------------------------------------------------------------------------
// MutationObserver.

fn install_mutation_observer(document: &Document) {
    // Only install when there's work to do.
    let any = STATE.with(|s| {
        s.borrow().as_ref().map_or(false, |c| {
            !c.remove_selectors.is_empty() || !c.hide_selectors.is_empty()
        })
    });
    if !any {
        return;
    }
    let cb = Closure::<dyn Fn(JsValue, JsValue)>::new(|_records: JsValue, _obs: JsValue| {
        pass();
    });
    let Ok(observer) = MutationObserver::new(cb.as_ref().unchecked_ref()) else {
        return;
    };
    let init = MutationObserverInit::new();
    init.set_child_list(true);
    init.set_subtree(true);
    let Some(root) = document.document_element() else {
        return;
    };
    let _ = observer.observe_with_options(&root, &init);
    keep_closure(cb);
    keep_closure(observer);

    // Match the JS behavior of logging the post-DOMContentLoaded state.
    let log_cb = Closure::<dyn Fn()>::new(|| {
        let (remove_counts, hide_counts) = STATE.with(|s| {
            s.borrow()
                .as_ref()
                .map(|c| (c.remove_counts.clone(), c.hide_counts.clone()))
                .unwrap_or_default()
        });
        log(&format!(
            "post-DOMContentLoaded pass: remove {:?} hide {:?}",
            remove_counts, hide_counts
        ));
    });
    if document.ready_state() == "loading" {
        let _ = document.add_event_listener_with_callback_and_add_event_listener_options(
            "DOMContentLoaded",
            log_cb.as_ref().unchecked_ref(),
            #[allow(deprecated)]
                web_sys::AddEventListenerOptions::new().once(true),
        );
    } else {
        log_cb
            .as_ref()
            .unchecked_ref::<Function>()
            .call0(&JsValue::NULL)
            .ok();
    }
    keep_closure(log_cb);
}

// ---------------------------------------------------------------------------
// Main-world bridge: `__hush_call__` event listener + batch send.

fn install_js_call_listener(document: &Document) {
    let cb = Closure::<dyn Fn(web_sys::Event)>::new(|ev: web_sys::Event| {
        // Skip cheaply if detector is off.
        if !STATE.with(|s| s.borrow().as_ref().map_or(false, |c| c.detector_enabled)) {
            return;
        }
        let custom_event: &web_sys::CustomEvent = ev.unchecked_ref();
        let detail = custom_event.detail();
        if !detail.is_object() {
            return;
        }
        // Validate against the typed SignalPayload union. Invalid
        // shapes get dropped loudly (the 0.5.0 bug class).
        let payload: SignalPayload = match serde_wasm_bindgen::from_value(detail.clone()) {
            Ok(p) => p,
            Err(e) => {
                web_sys::console::warn_1(&JsValue::from_str(&format!(
                    "[Hush] __hush_call__ shape mismatch: {e}"
                )));
                return;
            }
        };
        let call = signal_to_js_call(payload);
        STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                state.js_calls.push(call);
                if state.js_calls.len() > MAX_LOCAL_JS_CALLS {
                    let drop = state.js_calls.len() - MAX_LOCAL_JS_CALLS;
                    state.js_calls.drain(..drop);
                }
            }
        });
        schedule_js_call_send();
    });
    let _ = document.add_event_listener_with_callback("__hush_call__", cb.as_ref().unchecked_ref());
    keep_closure(cb);
}

/// Flatten the typed SignalPayload into the shape `background.js`
/// expects on `hush:js-calls` entries. Kind-specific fields are
/// left as `None`/empty when the variant doesn't carry them.
fn signal_to_js_call(p: SignalPayload) -> JsCall {
    let now = iso_now();
    let mut out = JsCall {
        kind: String::new(),
        t: now,
        stack: Vec::new(),
        url: None,
        method: None,
        body_preview: None,
        param: None,
        hot_param: false,
        font: None,
        text: None,
        event_type: None,
        vendors: Vec::new(),
        op: None,
        visible: None,
        canvas_sel: None,
        reporter_frame: None,
    };
    match p {
        SignalPayload::Fetch { url, method, body_preview, stack } => {
            out.kind = "fetch".into();
            out.url = Some(url);
            out.method = Some(method);
            out.body_preview = body_preview;
            out.stack = stack;
        }
        SignalPayload::Xhr { url, method, body_preview, stack } => {
            out.kind = "xhr".into();
            out.url = Some(url);
            out.method = Some(method);
            out.body_preview = body_preview;
            out.stack = stack;
        }
        SignalPayload::Beacon { url, method, body_preview, stack } => {
            out.kind = "beacon".into();
            out.url = Some(url);
            out.method = Some(method);
            out.body_preview = body_preview;
            out.stack = stack;
        }
        SignalPayload::WsSend { url, method, body_preview, stack } => {
            out.kind = "ws-send".into();
            out.url = Some(url);
            out.method = Some(method);
            out.body_preview = body_preview;
            out.stack = stack;
        }
        SignalPayload::CanvasFp { method, stack } => {
            out.kind = "canvas-fp".into();
            out.method = Some(method);
            out.stack = stack;
        }
        SignalPayload::FontFp { font, text, stack } => {
            out.kind = "font-fp".into();
            out.font = Some(font);
            out.text = Some(text);
            out.stack = stack;
        }
        SignalPayload::WebglFp { param, hot_param, stack } => {
            out.kind = "webgl-fp".into();
            out.param = Some(param);
            out.hot_param = hot_param;
            out.stack = stack;
        }
        SignalPayload::AudioFp { method, stack } => {
            out.kind = "audio-fp".into();
            out.method = Some(method);
            out.stack = stack;
        }
        SignalPayload::ListenerAdded { event_type, stack } => {
            out.kind = "listener-added".into();
            out.event_type = Some(event_type);
            out.stack = stack;
        }
        SignalPayload::ReplayGlobal { vendors } => {
            out.kind = "replay-global".into();
            out.vendors = vendors;
        }
        SignalPayload::CanvasDraw { op, visible, canvas_sel, stack } => {
            out.kind = "canvas-draw".into();
            out.op = Some(op);
            out.visible = Some(visible);
            out.canvas_sel = Some(canvas_sel);
            out.stack = stack;
        }
    }
    out
}

fn schedule_js_call_send() {
    let already = STATE.with(|s| {
        s.borrow().as_ref().map_or(false, |c| c.js_call_timer_scheduled)
    });
    if already {
        return;
    }
    STATE.with(|s| {
        if let Some(state) = s.borrow_mut().as_mut() {
            state.js_call_timer_scheduled = true;
        }
    });
    set_timeout(
        || {
            let batch: Vec<JsCall> = STATE.with(|s| {
                s.borrow_mut()
                    .as_mut()
                    .map(|state| {
                        state.js_call_timer_scheduled = false;
                        std::mem::take(&mut state.js_calls)
                    })
                    .unwrap_or_default()
            });
            if batch.is_empty() {
                return;
            }
            #[derive(Serialize)]
            struct Msg {
                #[serde(rename = "type")]
                type_: &'static str,
                calls: Vec<JsCall>,
            }
            send_message(&Msg {
                type_: "hush:js-calls",
                calls: batch,
            });
        },
        JS_CALL_SEND_DELAY_MS,
    );
}

// ---------------------------------------------------------------------------
// Hide/remove stats send.

fn schedule_stats_send() {
    let already = STATE.with(|s| {
        s.borrow().as_ref().map_or(false, |c| c.stats_timer_scheduled)
    });
    if already {
        return;
    }
    STATE.with(|s| {
        if let Some(state) = s.borrow_mut().as_mut() {
            state.stats_timer_scheduled = true;
        }
    });
    set_timeout(
        || {
            STATE.with(|s| {
                if let Some(state) = s.borrow_mut().as_mut() {
                    state.stats_timer_scheduled = false;
                }
            });
            send_stats();
        },
        STATS_SEND_DELAY_MS,
    );
}

fn send_stats() {
    #[derive(Serialize)]
    struct StatsMessage<'a> {
        #[serde(rename = "type")]
        type_: &'static str,
        #[serde(rename = "matchedDomain")]
        matched_domain: Option<&'a str>,
        hide: std::collections::BTreeMap<&'a str, u32>,
        remove: std::collections::BTreeMap<&'a str, u32>,
        #[serde(rename = "newRemovedElements")]
        new_removed: Vec<RemovedEvent>,
    }
    STATE.with(|s| {
        if let Some(state) = s.borrow_mut().as_mut() {
            let mut hide = std::collections::BTreeMap::new();
            for (i, sel) in state.hide_selectors.iter().enumerate() {
                hide.insert(sel.as_str(), *state.hide_counts.get(i).unwrap_or(&0));
            }
            let mut remove = std::collections::BTreeMap::new();
            for (i, sel) in state.remove_selectors.iter().enumerate() {
                remove.insert(sel.as_str(), *state.remove_counts.get(i).unwrap_or(&0));
            }
            let new_removed = std::mem::take(&mut state.pending_removed);
            let msg = StatsMessage {
                type_: "hush:stats",
                matched_domain: state.matched_domain.as_deref(),
                hide,
                remove,
                new_removed,
            };
            send_message(&msg);
        }
    });
}

// ---------------------------------------------------------------------------
// chrome.runtime.onMessage listener for `hush:scan-once`.

fn install_message_listener() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(chrome) = Reflect::get(&window, &JsValue::from_str("chrome")) else {
        return;
    };
    let Ok(runtime) = Reflect::get(&chrome, &JsValue::from_str("runtime")) else {
        return;
    };
    let Ok(on_message) = Reflect::get(&runtime, &JsValue::from_str("onMessage")) else {
        return;
    };
    let Ok(add) = Reflect::get(&on_message, &JsValue::from_str("addListener")) else {
        return;
    };
    let Ok(add_fn) = add.dyn_into::<Function>() else {
        return;
    };
    let cb = Closure::<dyn Fn(JsValue, JsValue, JsValue) -> JsValue>::new(
        |msg: JsValue, _sender: JsValue, send_response: JsValue| {
            let Ok(type_val) = Reflect::get(&msg, &JsValue::from_str("type")) else {
                return JsValue::FALSE;
            };
            if type_val.as_string().as_deref() == Some("hush:scan-once") {
                run_scan("manual");
                if let Ok(resp_fn) = send_response.dyn_into::<Function>() {
                    let reply = Object::new();
                    let _ = Reflect::set(&reply, &JsValue::from_str("ok"), &JsValue::TRUE);
                    let _ = resp_fn.call1(&JsValue::NULL, &reply.into());
                }
                return JsValue::FALSE;
            }
            JsValue::UNDEFINED
        },
    );
    let _ = add_fn.call1(&on_message, cb.as_ref().unchecked_ref());
    keep_closure(cb);
}

// ---------------------------------------------------------------------------
// Helpers: sendMessage, setTimeout, log, iso_now.

/// Fire-and-forget `chrome.runtime.sendMessage`. Ignores the returned
/// promise. Swallows errors so a dead extension context doesn't spam
/// the page console.
fn send_message<M: Serialize>(msg: &M) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(chrome) = Reflect::get(&window, &JsValue::from_str("chrome")) else {
        return;
    };
    let Ok(runtime) = Reflect::get(&chrome, &JsValue::from_str("runtime")) else {
        return;
    };
    let Ok(send) = Reflect::get(&runtime, &JsValue::from_str("sendMessage")) else {
        return;
    };
    let Ok(send_fn) = send.dyn_into::<Function>() else {
        return;
    };
    let payload = match serde_wasm_bindgen::to_value(msg) {
        Ok(v) => v,
        Err(_) => return,
    };
    // Call sendMessage; ignore the returned Promise (which may throw
    // synchronously if the extension context is gone).
    match send_fn.call1(&runtime, &payload) {
        Ok(promise) => {
            // Attach a noop catch handler so a rejected Promise
            // doesn't surface as "Unhandled promise rejection".
            if let Ok(p) = promise.dyn_into::<js_sys::Promise>() {
                let noop = Closure::<dyn FnMut(JsValue)>::new(|_| {});
                let _ = p.catch(&noop);
                keep_closure(noop);
            }
        }
        Err(_) => {}
    }
}

fn set_timeout<F: FnOnce() + 'static>(f: F, ms: i32) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cell = std::cell::Cell::new(Some(f));
    let cb = Closure::<dyn Fn()>::new(move || {
        if let Some(f) = cell.take() {
            f();
        }
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        ms,
    );
    cb.forget();
}

/// `console.log("[Hush]", msg)` + fire-and-forget `hush:log` message
/// so background can tail content-script logs. Gated by the debug
/// flag only for the console path; background always receives them.
fn log(msg: &str) {
    let debug = STATE.with(|s| s.borrow().as_ref().map_or(false, |c| c.debug));
    if debug {
        web_sys::console::log_2(&JsValue::from_str("[Hush]"), &JsValue::from_str(msg));
    }
    #[derive(Serialize)]
    struct LogMsg<'a> {
        #[serde(rename = "type")]
        type_: &'static str,
        level: &'static str,
        args: Vec<&'a str>,
    }
    send_message(&LogMsg {
        type_: "hush:log",
        level: "info",
        args: vec![msg],
    });
}

fn iso_now() -> String {
    js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default()
}

// Silence the unused-type warning for HtmlElement which is used
// transitively through web_sys downcasts.
#[allow(dead_code)]
fn _type_used(_: HtmlElement, _: Window) {}
