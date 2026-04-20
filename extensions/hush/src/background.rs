//! Background service worker Rust runtime (Stage 6 / post-Stage-5).
//!
//! The 988-line `background.js` replaced by a static-import wasm
//! bootstrap that calls [`hush_background_main`]. This module owns
//! every listener the service worker installs (onInstalled, onStartup,
//! onMessage, storage.onChanged, webNavigation.onCommitted,
//! tabs.onRemoved, declarativeNetRequest.onRuleMatchedDebug) plus
//! every handler, plus the DNR rule lifecycle, plus the per-tab
//! state the popup reads via `hush:get-tab-stats` / `hush:get-suggestions`
//! / `hush:get-rule-diagnostics` / `hush:get-debug-info`.
//!
//! State is held in a `thread_local! RefCell<BackgroundState>` because
//! wasm is single-threaded and every closure we hand to chrome.* APIs
//! needs mutable access. SW cold-wake wipes this state - the hydrate
//! path on the onStartup branch restores it from `chrome.storage.session`.

#![allow(clippy::too_many_arguments)]

use crate::canon::pattern_keyword;
use crate::compute::compute_suggestions as rust_compute_suggestions;
use crate::types::{
    Allowlist, BehaviorState, BlockedUrl, Config, IframeHit, JsCall, RemovedElement, Resource,
    StickyHit, Suggestion,
};
use indexmap::IndexMap;
use js_sys::{Array, Function, Object, Promise, Reflect};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

// ---------------------------------------------------------------------------
// Constants mirror the JS side.

const MAX_LOG_ENTRIES: usize = 300;
const MAX_EVIDENCE: usize = 50;
const MAX_SEEN_RESOURCES: usize = 500;
const MAX_JS_CALLS: usize = 500;
const STORAGE_KEY: &str = "config";
const OPTIONS_KEY: &str = "options";
const ALLOWLIST_KEY: &str = "allowlist";
const SESSION_TABSTATS_KEY: &str = "tabStats";
const SESSION_BEHAVIOR_KEY: &str = "tabBehavior";

// ---------------------------------------------------------------------------
// State.

thread_local! {
    static STATE: RefCell<BackgroundState> = RefCell::new(BackgroundState::default());
    static LIVE_CLOSURES: RefCell<Vec<Box<dyn std::any::Any>>> =
        const { RefCell::new(Vec::new()) };
}

fn keep<T: std::any::Any>(c: T) {
    LIVE_CLOSURES.with(|cell| cell.borrow_mut().push(Box::new(c)));
}

#[derive(Default)]
struct BackgroundState {
    debug_logging: bool,
    log_buffer: VecDeque<LogEntry>,
    rule_patterns: HashMap<i32, RuleMeta>,
    rule_fire_count: HashMap<i32, u32>,
    tab_stats: HashMap<i32, TabStatsEntry>,
    tab_behavior: HashMap<i32, BehaviorState>,
    allowlist_cache: Allowlist,
    persist_stats_scheduled: bool,
    persist_behavior_scheduled: bool,
    sync_in_flight: bool,
    sync_pending: bool,
}

#[derive(Clone, Serialize)]
struct LogEntry {
    t: String,
    level: String,
    source: String,
    msg: String,
}

#[derive(Clone)]
struct RuleMeta {
    pattern: String,
    domain: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct TabStatsEntry {
    matched_domain: Option<String>,
    hide: IndexMap<String, u32>,
    remove: IndexMap<String, u32>,
    block: u32,
    blocked_urls: Vec<BlockedUrl>,
    removed_elements: Vec<RemovedElement>,
}

// ---------------------------------------------------------------------------
// Entry point.

/// Called once per SW wake by `background.js` after `await init()`
/// and `initEngine()`. Installs every listener and kicks off the
/// initial hydrate + DNR-rule re-sync path.
#[wasm_bindgen(js_name = "hushBackgroundMain")]
pub fn hush_background_main() -> Result<(), JsValue> {
    install_on_installed()?;
    install_on_startup()?;
    install_storage_on_changed()?;
    install_web_navigation_committed()?;
    install_tabs_on_removed()?;
    install_dnr_on_rule_matched_debug()?;
    install_runtime_on_message()?;

    // Also run the "woke up" bootstrap: refresh flags, load allowlist,
    // hydrate per-tab state, rehydrate rule patterns. Mirrors the
    // IIFE at the bottom of the original background.js.
    spawn_local(async {
        if let Err(e) = bg_woke_up().await {
            log_error(&format!("bg_woke_up failed: {:?}", e));
        }
    });
    Ok(())
}

async fn bg_woke_up() -> Result<(), JsValue> {
    refresh_debug_flag().await?;
    load_allowlist().await?;
    hydrate_tab_stats().await;
    hydrate_tab_behavior().await;
    rehydrate_rule_patterns().await;
    log("service worker started / woke up");
    Ok(())
}

// ---------------------------------------------------------------------------
// Logging.

fn push_log(level: &str, source: &str, msg: String) {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.log_buffer.push_back(LogEntry {
            t: iso_now(),
            level: level.to_string(),
            source: source.to_string(),
            msg,
        });
        while state.log_buffer.len() > MAX_LOG_ENTRIES {
            state.log_buffer.pop_front();
        }
    });
}

fn log(msg: &str) {
    push_log("info", "bg", msg.to_string());
    let debug = STATE.with(|s| s.borrow().debug_logging);
    if debug {
        web_sys::console::log_2(&JsValue::from_str("[Hush bg]"), &JsValue::from_str(msg));
    }
}

fn log_error(msg: &str) {
    push_log("error", "bg", msg.to_string());
    web_sys::console::error_2(&JsValue::from_str("[Hush bg]"), &JsValue::from_str(msg));
}

// ---------------------------------------------------------------------------
// Storage helpers (async).

async fn storage_local_get_one(key: &str) -> Result<JsValue, JsValue> {
    let local = chrome_storage_local()?;
    let get_fn = get_fn_from(&local, "get")?;
    let promise: Promise = get_fn
        .call1(&local, &JsValue::from_str(key))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("storage.local.get did not return a Promise"))?;
    let reply = JsFuture::from(promise).await?;
    Ok(Reflect::get(&reply, &JsValue::from_str(key))?)
}

async fn storage_local_set_one(key: &str, value: &JsValue) -> Result<(), JsValue> {
    let local = chrome_storage_local()?;
    let set_fn = get_fn_from(&local, "set")?;
    let payload = Object::new();
    Reflect::set(&payload, &JsValue::from_str(key), value)?;
    let promise: Promise = set_fn
        .call1(&local, &payload.into())?
        .dyn_into()
        .map_err(|_| JsValue::from_str("storage.local.set did not return a Promise"))?;
    JsFuture::from(promise).await?;
    Ok(())
}

async fn storage_session_get_one(key: &str) -> Result<JsValue, JsValue> {
    let session = chrome_storage_session()?;
    let get_fn = get_fn_from(&session, "get")?;
    let promise: Promise = get_fn
        .call1(&session, &JsValue::from_str(key))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("storage.session.get did not return a Promise"))?;
    let reply = JsFuture::from(promise).await?;
    Ok(Reflect::get(&reply, &JsValue::from_str(key))?)
}

async fn storage_session_set_one(key: &str, value: &JsValue) -> Result<(), JsValue> {
    let session = chrome_storage_session()?;
    let set_fn = get_fn_from(&session, "set")?;
    let payload = Object::new();
    Reflect::set(&payload, &JsValue::from_str(key), value)?;
    let promise: Promise = set_fn
        .call1(&session, &payload.into())?
        .dyn_into()
        .map_err(|_| JsValue::from_str("storage.session.set did not return a Promise"))?;
    JsFuture::from(promise).await?;
    Ok(())
}

/// Global scope accessor that works in both Window and
/// ServiceWorkerGlobalScope contexts. `web_sys::window()` returns
/// `None` inside a service worker - `js_sys::global()` is the
/// cross-context fallback.
fn global_scope() -> JsValue {
    js_sys::global().into()
}

fn chrome_root() -> Result<JsValue, JsValue> {
    let g = global_scope();
    Ok(Reflect::get(&g, &JsValue::from_str("chrome"))?)
}

fn chrome_storage_local() -> Result<JsValue, JsValue> {
    let chrome = chrome_root()?;
    let storage = Reflect::get(&chrome, &JsValue::from_str("storage"))?;
    Ok(Reflect::get(&storage, &JsValue::from_str("local"))?)
}

fn chrome_storage_session() -> Result<JsValue, JsValue> {
    let chrome = chrome_root()?;
    let storage = Reflect::get(&chrome, &JsValue::from_str("storage"))?;
    Ok(Reflect::get(&storage, &JsValue::from_str("session"))?)
}

fn get_fn_from(parent: &JsValue, name: &str) -> Result<Function, JsValue> {
    Reflect::get(parent, &JsValue::from_str(name))?
        .dyn_into::<Function>()
        .map_err(|_| JsValue::from_str(&format!("{name} is not a function")))
}

async fn load_config() -> Result<Config, JsValue> {
    let v = storage_local_get_one(STORAGE_KEY).await?;
    if v.is_undefined() || v.is_null() {
        return Ok(Config::default());
    }
    Ok(serde_wasm_bindgen::from_value(v).unwrap_or_default())
}

async fn load_options_raw() -> Result<JsValue, JsValue> {
    let v = storage_local_get_one(OPTIONS_KEY).await?;
    if v.is_undefined() || v.is_null() {
        return Ok(Object::new().into());
    }
    Ok(v)
}

async fn load_allowlist() -> Result<(), JsValue> {
    let v = storage_local_get_one(ALLOWLIST_KEY).await?;
    let al: Allowlist = if v.is_undefined() || v.is_null() {
        Allowlist::default()
    } else {
        serde_wasm_bindgen::from_value(v).unwrap_or_default()
    };
    STATE.with(|s| s.borrow_mut().allowlist_cache = al);
    Ok(())
}

async fn refresh_debug_flag() -> Result<(), JsValue> {
    let opts = load_options_raw().await?;
    let debug = Reflect::get(&opts, &JsValue::from_str("debug"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    STATE.with(|s| s.borrow_mut().debug_logging = debug);
    Ok(())
}

async fn seed_config_if_empty() -> Result<(), JsValue> {
    let existing = storage_local_get_one(STORAGE_KEY).await?;
    if !existing.is_undefined() && !existing.is_null() {
        return Ok(());
    }
    let url_fn = chrome_runtime_get_url()?;
    let url = url_fn
        .call1(&chrome_root()?.dyn_into::<Object>()?.into(), &JsValue::from_str("sites.json"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("chrome.runtime.getURL returned non-string"))?;
    let g = global_scope();
    let fetch_fn: Function = Reflect::get(&g, &JsValue::from_str("fetch"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("fetch is not a function"))?;
    let fetch_promise: Promise = fetch_fn
        .call1(&g, &JsValue::from_str(&url))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("fetch did not return a Promise"))?;
    let response = JsFuture::from(fetch_promise).await?;
    let json_fn: Function = Reflect::get(&response, &JsValue::from_str("json"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("Response.json is not a function"))?;
    let json_promise: Promise = json_fn
        .call0(&response)?
        .dyn_into()
        .map_err(|_| JsValue::from_str("Response.json did not return a Promise"))?;
    let seed = JsFuture::from(json_promise).await?;
    storage_local_set_one(STORAGE_KEY, &seed).await?;
    Ok(())
}

async fn seed_allowlist_if_empty() -> Result<(), JsValue> {
    let existing = storage_local_get_one(ALLOWLIST_KEY).await?;
    if !existing.is_undefined() && !existing.is_null() {
        return Ok(());
    }
    // Fetch allowlist.defaults.json and write. Falls back to empty on failure.
    let url_fn = chrome_runtime_get_url()?;
    let url = url_fn
        .call1(
            &chrome_root()?.dyn_into::<Object>()?.into(),
            &JsValue::from_str("allowlist.defaults.json"),
        )?
        .as_string()
        .unwrap_or_default();
    let g = global_scope();
    let fetch_fn: Function = Reflect::get(&g, &JsValue::from_str("fetch"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("fetch is not a function"))?;
    let fetched = async {
        let p: Promise = fetch_fn
            .call1(&g, &JsValue::from_str(&url))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("fetch did not return a Promise"))?;
        let resp = JsFuture::from(p).await?;
        let json_fn: Function = Reflect::get(&resp, &JsValue::from_str("json"))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("Response.json is not a function"))?;
        let jp: Promise = json_fn
            .call0(&resp)?
            .dyn_into()
            .map_err(|_| JsValue::from_str("Response.json did not return a Promise"))?;
        JsFuture::from(jp).await
    }
    .await;
    let value = fetched.unwrap_or_else(|_| {
        // Empty default
        let obj = Object::new();
        let empty = Array::new();
        Reflect::set(&obj, &JsValue::from_str("iframes"), &empty).ok();
        Reflect::set(&obj, &JsValue::from_str("overlays"), &empty).ok();
        obj.into()
    });
    storage_local_set_one(ALLOWLIST_KEY, &value).await?;
    Ok(())
}

fn chrome_runtime_get_url() -> Result<Function, JsValue> {
    let chrome = chrome_root()?;
    let runtime = Reflect::get(&chrome, &JsValue::from_str("runtime"))?;
    Reflect::get(&runtime, &JsValue::from_str("getURL"))?
        .dyn_into::<Function>()
        .map_err(|_| JsValue::from_str("chrome.runtime.getURL is not a function"))
}

// ---------------------------------------------------------------------------
// DNR rule sync.

async fn sync_dynamic_rules() -> Result<(), JsValue> {
    // Chain: serialize so two onChanged bursts don't interleave.
    let already = STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.sync_in_flight {
            st.sync_pending = true;
            true
        } else {
            st.sync_in_flight = true;
            false
        }
    });
    if already {
        return Ok(());
    }
    let res = do_sync_dynamic_rules().await;
    // Drain any pending sync requests.
    loop {
        let pending = STATE.with(|s| {
            let mut st = s.borrow_mut();
            if st.sync_pending {
                st.sync_pending = false;
                true
            } else {
                st.sync_in_flight = false;
                false
            }
        });
        if !pending {
            break;
        }
        let _ = do_sync_dynamic_rules().await;
    }
    res
}

async fn do_sync_dynamic_rules() -> Result<(), JsValue> {
    let config = load_config().await?;
    let dnr = chrome_dnr()?;
    let get_rules_fn = get_fn_from(&dnr, "getDynamicRules")?;
    let get_promise: Promise = get_rules_fn
        .call0(&dnr)?
        .dyn_into()
        .map_err(|_| JsValue::from_str("getDynamicRules did not return a Promise"))?;
    let existing = JsFuture::from(get_promise).await?;
    let existing: Array = existing
        .dyn_into()
        .map_err(|_| JsValue::from_str("getDynamicRules did not return an Array"))?;
    let remove_ids = Array::new();
    for i in 0..existing.length() {
        let rule = existing.get(i);
        if let Ok(id_val) = Reflect::get(&rule, &JsValue::from_str("id")) {
            remove_ids.push(&id_val);
        }
    }

    let add_rules = Array::new();
    let mut patterns: HashMap<i32, RuleMeta> = HashMap::new();
    let mut next_id: i32 = 1;
    for (domain, cfg) in config.iter() {
        for pattern in &cfg.block {
            let pattern = pattern.trim();
            if pattern.is_empty() {
                continue;
            }
            let id = next_id;
            next_id += 1;
            patterns.insert(
                id,
                RuleMeta {
                    pattern: pattern.to_string(),
                    domain: domain.clone(),
                },
            );
            let rule = Object::new();
            Reflect::set(&rule, &JsValue::from_str("id"), &JsValue::from_f64(id as f64))?;
            Reflect::set(&rule, &JsValue::from_str("priority"), &JsValue::from_f64(1.0))?;
            let action = Object::new();
            Reflect::set(&action, &JsValue::from_str("type"), &JsValue::from_str("block"))?;
            Reflect::set(&rule, &JsValue::from_str("action"), &action)?;
            let condition = Object::new();
            Reflect::set(
                &condition,
                &JsValue::from_str("urlFilter"),
                &JsValue::from_str(pattern),
            )?;
            Reflect::set(&rule, &JsValue::from_str("condition"), &condition)?;
            add_rules.push(&rule);
        }
    }

    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.rule_patterns = patterns;
        st.rule_fire_count.clear();
    });

    let update_fn = get_fn_from(&dnr, "updateDynamicRules")?;
    let arg = Object::new();
    Reflect::set(&arg, &JsValue::from_str("removeRuleIds"), &remove_ids)?;
    Reflect::set(&arg, &JsValue::from_str("addRules"), &add_rules)?;
    let promise: Promise = update_fn
        .call1(&dnr, &arg.into())?
        .dyn_into()
        .map_err(|_| JsValue::from_str("updateDynamicRules did not return a Promise"))?;
    JsFuture::from(promise).await?;
    log(&format!(
        "synced dynamic rules: removed {} added {}",
        remove_ids.length(),
        add_rules.length()
    ));
    Ok(())
}

fn chrome_dnr() -> Result<JsValue, JsValue> {
    let chrome = chrome_root()?;
    Ok(Reflect::get(&chrome, &JsValue::from_str("declarativeNetRequest"))?)
}

async fn rehydrate_rule_patterns() {
    let Ok(dnr) = chrome_dnr() else { return };
    let Ok(get_fn) = get_fn_from(&dnr, "getDynamicRules") else {
        return;
    };
    let promise: Promise = match get_fn.call0(&dnr).and_then(|v| {
        v.dyn_into()
            .map_err(|_| JsValue::from_str("getDynamicRules did not return a Promise"))
    }) {
        Ok(p) => p,
        Err(_) => return,
    };
    let Ok(existing) = JsFuture::from(promise).await else {
        return;
    };
    let Ok(existing) = existing.dyn_into::<Array>() else {
        return;
    };

    let config = load_config().await.unwrap_or_default();
    // Build pattern -> source_domain reverse map, tolerating a trailing `^`.
    let mut pattern_to_source: HashMap<String, String> = HashMap::new();
    for (domain, cfg) in config.iter() {
        for raw in &cfg.block {
            let normalized = raw.strip_suffix('^').unwrap_or(raw).to_string();
            pattern_to_source
                .entry(normalized)
                .or_insert_with(|| domain.clone());
        }
    }
    let mut patterns: HashMap<i32, RuleMeta> = HashMap::new();
    for i in 0..existing.length() {
        let rule = existing.get(i);
        let Some(id) = Reflect::get(&rule, &JsValue::from_str("id"))
            .ok()
            .and_then(|v| v.as_f64())
            .map(|f| f as i32)
        else {
            continue;
        };
        let pattern = Reflect::get(&rule, &JsValue::from_str("condition"))
            .ok()
            .and_then(|c| Reflect::get(&c, &JsValue::from_str("urlFilter")).ok())
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let domain = pattern_to_source.get(&pattern).cloned().unwrap_or_default();
        patterns.insert(id, RuleMeta { pattern, domain });
    }
    let n = patterns.len();
    STATE.with(|s| s.borrow_mut().rule_patterns = patterns);
    log(&format!("rehydrated rulePatterns for {} rule(s)", n));
}

// ---------------------------------------------------------------------------
// Per-tab stats + badge + persistence.

fn get_stats_mut<R>(tab_id: i32, f: impl FnOnce(&mut TabStatsEntry) -> R) -> R {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let entry = st.tab_stats.entry(tab_id).or_default();
        f(entry)
    })
}

fn reset_stats(tab_id: i32) {
    STATE.with(|s| {
        s.borrow_mut()
            .tab_stats
            .insert(tab_id, TabStatsEntry::default());
    });
    update_badge(tab_id);
    schedule_persist_stats();
}

fn total_activity(stats: &TabStatsEntry) -> u32 {
    let hide: u32 = stats.hide.values().sum();
    let remove: u32 = stats.remove.values().sum();
    hide + remove + stats.block
}

fn update_badge(tab_id: i32) {
    let (total, sugg_count) = STATE.with(|s| {
        let st = s.borrow();
        let total = st.tab_stats.get(&tab_id).map(total_activity).unwrap_or(0);
        let sugg = st
            .tab_behavior
            .get(&tab_id)
            .map(|b| b.suggestions.len() as u32)
            .unwrap_or(0);
        (total, sugg)
    });
    if sugg_count > 0 {
        set_badge(tab_id, "!", "#e8a200");
    } else {
        let text = if total > 0 { total.to_string() } else { String::new() };
        set_badge(tab_id, &text, "#666");
    }
}

fn set_badge(tab_id: i32, text: &str, color: &str) {
    let Ok(chrome) = chrome_root() else { return };
    let Ok(action) = Reflect::get(&chrome, &JsValue::from_str("action")) else {
        return;
    };
    if let Ok(set_text) = get_fn_from(&action, "setBadgeText") {
        let arg = Object::new();
        let _ = Reflect::set(&arg, &JsValue::from_str("tabId"), &JsValue::from_f64(tab_id as f64));
        let _ = Reflect::set(&arg, &JsValue::from_str("text"), &JsValue::from_str(text));
        let _ = set_text.call1(&action, &arg.into());
    }
    if let Ok(set_color) = get_fn_from(&action, "setBadgeBackgroundColor") {
        let arg = Object::new();
        let _ = Reflect::set(&arg, &JsValue::from_str("tabId"), &JsValue::from_f64(tab_id as f64));
        let _ = Reflect::set(&arg, &JsValue::from_str("color"), &JsValue::from_str(color));
        let _ = set_color.call1(&action, &arg.into());
    }
}

fn schedule_persist_stats() {
    let already = STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.persist_stats_scheduled {
            true
        } else {
            st.persist_stats_scheduled = true;
            false
        }
    });
    if already {
        return;
    }
    set_timeout(
        || {
            STATE.with(|s| s.borrow_mut().persist_stats_scheduled = false);
            spawn_local(async {
                let snapshot: HashMap<String, TabStatsEntry> = STATE.with(|s| {
                    s.borrow()
                        .tab_stats
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.clone()))
                        .collect()
                });
                match serde_wasm_bindgen::to_value(&snapshot) {
                    Ok(v) => {
                        if let Err(e) = storage_session_set_one(SESSION_TABSTATS_KEY, &v).await {
                            log_error(&format!("persist tabStats failed: {:?}", e));
                        }
                    }
                    Err(e) => log_error(&format!("persist tabStats serialize failed: {}", e)),
                }
            });
        },
        500,
    );
}

async fn hydrate_tab_stats() {
    let v = match storage_session_get_one(SESSION_TABSTATS_KEY).await {
        Ok(v) if !v.is_undefined() && !v.is_null() => v,
        _ => return,
    };
    let Ok(map): Result<HashMap<String, TabStatsEntry>, _> = serde_wasm_bindgen::from_value(v)
    else {
        return;
    };
    let n = map.len();
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        for (k, v) in map {
            if let Ok(id) = k.parse::<i32>() {
                st.tab_stats.insert(id, v);
            }
        }
    });
    log(&format!("hydrated tabStats for {} tab(s) from session storage", n));
}

// ---------------------------------------------------------------------------
// Per-tab behavior + persistence.

fn get_behavior_mut<R>(tab_id: i32, f: impl FnOnce(&mut BehaviorState) -> R) -> R {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let entry = st.tab_behavior.entry(tab_id).or_default();
        f(entry)
    })
}

fn reset_behavior(tab_id: i32) {
    STATE.with(|s| {
        s.borrow_mut()
            .tab_behavior
            .insert(tab_id, BehaviorState::default());
    });
    schedule_persist_behavior();
}

fn schedule_persist_behavior() {
    let already = STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.persist_behavior_scheduled {
            true
        } else {
            st.persist_behavior_scheduled = true;
            false
        }
    });
    if already {
        return;
    }
    set_timeout(
        || {
            STATE.with(|s| s.borrow_mut().persist_behavior_scheduled = false);
            spawn_local(async {
                let snapshot: HashMap<String, BehaviorState> = STATE.with(|s| {
                    s.borrow()
                        .tab_behavior
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.clone()))
                        .collect()
                });
                match serde_wasm_bindgen::to_value(&snapshot) {
                    Ok(v) => {
                        if let Err(e) = storage_session_set_one(SESSION_BEHAVIOR_KEY, &v).await {
                            log_error(&format!("persist behavior failed: {:?}", e));
                        }
                    }
                    Err(e) => log_error(&format!("persist behavior serialize failed: {}", e)),
                }
            });
        },
        500,
    );
}

async fn hydrate_tab_behavior() {
    let v = match storage_session_get_one(SESSION_BEHAVIOR_KEY).await {
        Ok(v) if !v.is_undefined() && !v.is_null() => v,
        _ => return,
    };
    let Ok(map): Result<HashMap<String, BehaviorState>, _> = serde_wasm_bindgen::from_value(v)
    else {
        return;
    };
    let n = map.len();
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        for (k, v) in map {
            if let Ok(id) = k.parse::<i32>() {
                st.tab_behavior.insert(id, v);
            }
        }
    });
    log(&format!("hydrated behavior for {} tab(s)", n));
}

// ---------------------------------------------------------------------------
// Diagnostics.

fn compute_rule_diagnostics(tab_id: Option<i32>, hostname: Option<&str>) -> Vec<RuleDiagnostic> {
    STATE.with(|s| {
        let st = s.borrow();
        let behavior = tab_id.and_then(|id| st.tab_behavior.get(&id));
        let observed: Vec<&Resource> = behavior
            .map(|b| b.seen_resources.iter().collect())
            .unwrap_or_default();
        let host = hostname
            .map(|h| h.to_string())
            .or_else(|| behavior.and_then(|b| b.page_host.clone()));
        let mut out = Vec::new();
        for (id, meta) in &st.rule_patterns {
            let source_domain = meta.domain.clone();
            if let (Some(h), false) = (&host, source_domain.is_empty()) {
                let matches = h == &source_domain || h.ends_with(&format!(".{source_domain}"));
                if !matches {
                    continue;
                }
            }
            let keyword = pattern_keyword(&meta.pattern).to_string();
            let fired = *st.rule_fire_count.get(id).unwrap_or(&0);
            let matching_urls: Vec<String> = if keyword.is_empty() {
                Vec::new()
            } else {
                observed
                    .iter()
                    .filter(|r| r.url.contains(&keyword))
                    .rev()
                    .take(5)
                    .map(|r| r.url.clone())
                    .collect()
            };
            let status = if fired > 0 {
                "firing"
            } else if !matching_urls.is_empty() {
                "pattern-broken"
            } else {
                "no-traffic"
            };
            out.push(RuleDiagnostic {
                rule_id: *id,
                pattern: meta.pattern.clone(),
                source_domain,
                fired,
                keyword,
                status: status.to_string(),
                matching_urls,
            });
        }
        out
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuleDiagnostic {
    rule_id: i32,
    pattern: String,
    source_domain: String,
    fired: u32,
    keyword: String,
    status: String,
    matching_urls: Vec<String>,
}

// ---------------------------------------------------------------------------
// Suggestions bridge.

async fn compute_suggestions_for(
    state: &BehaviorState,
    config: &Config,
) -> Vec<Suggestion> {
    let allowlist = STATE.with(|s| s.borrow().allowlist_cache.clone());
    rust_compute_suggestions(state, config, &allowlist)
}

// ---------------------------------------------------------------------------
// Listeners.

fn install_on_installed() -> Result<(), JsValue> {
    let on_installed = get_event(&chrome_runtime()?, "onInstalled")?;
    let cb = Closure::<dyn Fn(JsValue)>::new(|_details: JsValue| {
        set_default_badge_color();
        spawn_local(async {
            let _ = refresh_debug_flag().await;
            let _ = seed_config_if_empty().await;
            let _ = seed_allowlist_if_empty().await;
            let _ = load_allowlist().await;
            let _ = sync_dynamic_rules().await;
        });
    });
    add_listener(&on_installed, &cb)?;
    keep(cb);
    Ok(())
}

fn install_on_startup() -> Result<(), JsValue> {
    let on_startup = get_event(&chrome_runtime()?, "onStartup")?;
    let cb = Closure::<dyn Fn()>::new(|| {
        set_default_badge_color();
        spawn_local(async {
            let _ = refresh_debug_flag().await;
            let _ = load_allowlist().await;
            let _ = sync_dynamic_rules().await;
        });
    });
    add_listener(&on_startup, &cb)?;
    keep(cb);
    Ok(())
}

fn set_default_badge_color() {
    let Ok(chrome) = chrome_root() else { return };
    let Ok(action) = Reflect::get(&chrome, &JsValue::from_str("action")) else {
        return;
    };
    if let Ok(set_color) = get_fn_from(&action, "setBadgeBackgroundColor") {
        let arg = Object::new();
        let _ = Reflect::set(&arg, &JsValue::from_str("color"), &JsValue::from_str("#666"));
        let _ = set_color.call1(&action, &arg.into());
    }
}

fn install_storage_on_changed() -> Result<(), JsValue> {
    let chrome = chrome_root()?;
    let storage = Reflect::get(&chrome, &JsValue::from_str("storage"))?;
    let on_changed = Reflect::get(&storage, &JsValue::from_str("onChanged"))?;
    let cb = Closure::<dyn Fn(JsValue, JsValue)>::new(|changes: JsValue, area: JsValue| {
        if area.as_string().as_deref() != Some("local") {
            return;
        }
        // options change -> refresh debug flag
        if !Reflect::get(&changes, &JsValue::from_str(OPTIONS_KEY))
            .map(|v| v.is_undefined() || v.is_null())
            .unwrap_or(true)
        {
            let new_val = Reflect::get(&changes, &JsValue::from_str(OPTIONS_KEY))
                .and_then(|c| Reflect::get(&c, &JsValue::from_str("newValue")))
                .ok();
            let debug = new_val
                .and_then(|v| Reflect::get(&v, &JsValue::from_str("debug")).ok())
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            STATE.with(|s| s.borrow_mut().debug_logging = debug);
            log(&format!("debug logging -> {debug}"));
        }
        // config change -> sync DNR rules
        if !Reflect::get(&changes, &JsValue::from_str(STORAGE_KEY))
            .map(|v| v.is_undefined() || v.is_null())
            .unwrap_or(true)
        {
            spawn_local(async {
                let _ = sync_dynamic_rules().await;
            });
        }
        // allowlist change -> refresh cache
        if !Reflect::get(&changes, &JsValue::from_str(ALLOWLIST_KEY))
            .map(|v| v.is_undefined() || v.is_null())
            .unwrap_or(true)
        {
            spawn_local(async {
                let _ = load_allowlist().await;
                log("allowlist updated");
            });
        }
    });
    add_listener(&on_changed, &cb)?;
    keep(cb);
    Ok(())
}

fn install_web_navigation_committed() -> Result<(), JsValue> {
    let chrome = chrome_root()?;
    let web_nav = Reflect::get(&chrome, &JsValue::from_str("webNavigation"))?;
    let on_committed = Reflect::get(&web_nav, &JsValue::from_str("onCommitted"))?;
    let cb = Closure::<dyn Fn(JsValue)>::new(|details: JsValue| {
        let frame_id = Reflect::get(&details, &JsValue::from_str("frameId"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(-1.0);
        if frame_id != 0.0 {
            return;
        }
        let tab_id = Reflect::get(&details, &JsValue::from_str("tabId"))
            .ok()
            .and_then(|v| v.as_f64())
            .map(|f| f as i32);
        let url = Reflect::get(&details, &JsValue::from_str("url"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        if let Some(id) = tab_id {
            reset_stats(id);
            reset_behavior(id);
            log(&format!("nav committed, reset tab {id} {url}"));
        }
    });
    add_listener(&on_committed, &cb)?;
    keep(cb);
    Ok(())
}

fn install_tabs_on_removed() -> Result<(), JsValue> {
    let chrome = chrome_root()?;
    let tabs = Reflect::get(&chrome, &JsValue::from_str("tabs"))?;
    let on_removed = Reflect::get(&tabs, &JsValue::from_str("onRemoved"))?;
    let cb = Closure::<dyn Fn(JsValue)>::new(|tab_id: JsValue| {
        let Some(id) = tab_id.as_f64() else { return };
        let id = id as i32;
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.tab_stats.remove(&id);
            st.tab_behavior.remove(&id);
        });
        schedule_persist_stats();
        schedule_persist_behavior();
    });
    add_listener(&on_removed, &cb)?;
    keep(cb);
    Ok(())
}

fn install_dnr_on_rule_matched_debug() -> Result<(), JsValue> {
    let dnr = chrome_dnr()?;
    let on_matched = Reflect::get(&dnr, &JsValue::from_str("onRuleMatchedDebug"))?;
    if on_matched.is_undefined() || on_matched.is_null() {
        // Not available (only fires for unpacked extensions).
        return Ok(());
    }
    let cb = Closure::<dyn Fn(JsValue)>::new(|info: JsValue| {
        let request = Reflect::get(&info, &JsValue::from_str("request")).unwrap_or(JsValue::NULL);
        let rule = Reflect::get(&info, &JsValue::from_str("rule")).unwrap_or(JsValue::NULL);
        let tab_id = Reflect::get(&request, &JsValue::from_str("tabId"))
            .ok()
            .and_then(|v| v.as_f64())
            .map(|f| f as i32);
        let rule_id = Reflect::get(&rule, &JsValue::from_str("ruleId"))
            .ok()
            .and_then(|v| v.as_f64())
            .map(|f| f as i32);
        let url = Reflect::get(&request, &JsValue::from_str("url"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let resource_type = Reflect::get(&request, &JsValue::from_str("type"))
            .ok()
            .and_then(|v| v.as_string());
        let (pattern, domain) = STATE.with(|s| {
            if let Some(id) = rule_id {
                let st = s.borrow();
                if let Some(meta) = st.rule_patterns.get(&id) {
                    return (meta.pattern.clone(), meta.domain.clone());
                }
            }
            (String::new(), String::new())
        });
        if let Some(id) = rule_id {
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                *st.rule_fire_count.entry(id).or_insert(0) += 1;
            });
        }
        log(&format!(
            "rule matched: {:?} pattern: {pattern} url: {url} tabId: {:?}",
            rule_id, tab_id
        ));
        let Some(tid) = tab_id else { return };
        if tid < 0 {
            return;
        }
        get_stats_mut(tid, |stats| {
            stats.block += 1;
            stats.blocked_urls.push(BlockedUrl {
                t: iso_now(),
                url: url.clone(),
                pattern: pattern.clone(),
                resource_type: resource_type.clone(),
            });
            if stats.blocked_urls.len() > MAX_EVIDENCE {
                let drop = stats.blocked_urls.len() - MAX_EVIDENCE;
                stats.blocked_urls.drain(..drop);
            }
            let _ = domain; // attached via rule_patterns lookup; no need to persist per-event
        });
        update_badge(tid);
        schedule_persist_stats();
    });
    add_listener(&on_matched, &cb)?;
    keep(cb);
    Ok(())
}

fn install_runtime_on_message() -> Result<(), JsValue> {
    let runtime = chrome_runtime()?;
    let on_message = Reflect::get(&runtime, &JsValue::from_str("onMessage"))?;
    let cb = Closure::<dyn Fn(JsValue, JsValue, JsValue) -> JsValue>::new(
        |msg: JsValue, sender: JsValue, send_response: JsValue| {
            handle_message(msg, sender, send_response)
        },
    );
    add_listener(&on_message, &cb)?;
    keep(cb);
    Ok(())
}

fn chrome_runtime() -> Result<JsValue, JsValue> {
    let chrome = chrome_root()?;
    Ok(Reflect::get(&chrome, &JsValue::from_str("runtime"))?)
}

fn get_event(parent: &JsValue, name: &str) -> Result<JsValue, JsValue> {
    Ok(Reflect::get(parent, &JsValue::from_str(name))?)
}

fn add_listener<F>(event: &JsValue, cb: &Closure<F>) -> Result<(), JsValue>
where
    F: ?Sized + wasm_bindgen::closure::WasmClosure,
{
    let add_fn = get_fn_from(event, "addListener")?;
    add_fn.call1(event, cb.as_ref())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Message handler.

fn handle_message(msg: JsValue, sender: JsValue, send_response: JsValue) -> JsValue {
    if !msg.is_object() {
        return JsValue::UNDEFINED;
    }
    let type_str = match Reflect::get(&msg, &JsValue::from_str("type"))
        .ok()
        .and_then(|v| v.as_string())
    {
        Some(s) => s,
        None => return JsValue::UNDEFINED,
    };
    match type_str.as_str() {
        "hush:stats" => {
            handle_stats(&msg, &sender);
            JsValue::UNDEFINED
        }
        "hush:log" => {
            handle_log(&msg, &sender);
            JsValue::UNDEFINED
        }
        "hush:js-calls" => {
            handle_js_calls(&msg, &sender);
            JsValue::UNDEFINED
        }
        "hush:scan" => {
            handle_scan(&msg, &sender);
            JsValue::UNDEFINED
        }
        "hush:get-tab-stats" => {
            handle_get_tab_stats(&msg, &sender, &send_response);
            JsValue::FALSE
        }
        "hush:get-rule-diagnostics" => {
            handle_get_rule_diagnostics(&msg, &sender, &send_response);
            JsValue::FALSE
        }
        "hush:get-suggestions" => {
            handle_get_suggestions(&msg, &sender, send_response.clone());
            JsValue::TRUE
        }
        "hush:accept-suggestion" => {
            handle_accept_suggestion(&msg, send_response.clone());
            JsValue::TRUE
        }
        "hush:allowlist-add-suggestion" => {
            handle_allowlist_add_suggestion(&msg, send_response.clone());
            JsValue::TRUE
        }
        "hush:dismiss-suggestion" => {
            handle_dismiss_suggestion(&msg, &sender, &send_response);
            JsValue::FALSE
        }
        "hush:get-debug-info" => {
            handle_get_debug_info(&msg, send_response.clone());
            JsValue::TRUE
        }
        _ => JsValue::UNDEFINED,
    }
}

fn handle_stats(msg: &JsValue, sender: &JsValue) {
    let Some(tab_id) = sender_tab_id(sender) else {
        return;
    };
    let matched = Reflect::get(msg, &JsValue::from_str("matchedDomain"))
        .ok()
        .and_then(|v| {
            if v.is_undefined() {
                None
            } else if v.is_null() {
                Some(None)
            } else {
                v.as_string().map(Some)
            }
        });
    let hide = Reflect::get(msg, &JsValue::from_str("hide"))
        .ok()
        .and_then(|v| serde_wasm_bindgen::from_value::<IndexMap<String, u32>>(v).ok());
    let remove = Reflect::get(msg, &JsValue::from_str("remove"))
        .ok()
        .and_then(|v| serde_wasm_bindgen::from_value::<IndexMap<String, u32>>(v).ok());
    let new_removed = Reflect::get(msg, &JsValue::from_str("newRemovedElements"))
        .ok()
        .and_then(|v| serde_wasm_bindgen::from_value::<Vec<RemovedElement>>(v).ok())
        .unwrap_or_default();
    get_stats_mut(tab_id, |s| {
        if let Some(m) = matched {
            s.matched_domain = m;
        }
        if let Some(h) = hide {
            s.hide = h;
        }
        if let Some(r) = remove {
            s.remove = r;
        }
        if !new_removed.is_empty() {
            s.removed_elements.extend(new_removed);
            if s.removed_elements.len() > MAX_EVIDENCE {
                let drop = s.removed_elements.len() - MAX_EVIDENCE;
                s.removed_elements.drain(..drop);
            }
        }
    });
    update_badge(tab_id);
    schedule_persist_stats();
}

fn handle_log(msg: &JsValue, sender: &JsValue) {
    let level = Reflect::get(msg, &JsValue::from_str("level"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "info".to_string());
    let args_val = Reflect::get(msg, &JsValue::from_str("args")).unwrap_or(JsValue::NULL);
    let args_str = if let Ok(arr) = args_val.dyn_into::<Array>() {
        let mut parts: Vec<String> = Vec::new();
        for i in 0..arr.length() {
            let v = arr.get(i);
            parts.push(v.as_string().unwrap_or_else(|| format!("{:?}", v)));
        }
        parts.join(" ")
    } else {
        Reflect::get(msg, &JsValue::from_str("msg"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    };
    let tid = sender_tab_id(sender)
        .map(|i| i.to_string())
        .unwrap_or_else(|| "?".to_string());
    push_log(&level, &format!("content@tab{tid}"), args_str);
}

fn handle_js_calls(msg: &JsValue, sender: &JsValue) {
    let Some(tab_id) = sender_tab_id(sender) else {
        return;
    };
    let calls = Reflect::get(msg, &JsValue::from_str("calls")).unwrap_or(JsValue::NULL);
    let Ok(calls) = calls.dyn_into::<Array>() else {
        return;
    };
    if calls.length() == 0 {
        return;
    }
    let frame = sender_frame_host(sender);
    let tab_host = sender_tab_host(sender);
    get_behavior_mut(tab_id, |state| {
        for i in 0..calls.length() {
            let c = calls.get(i);
            let Ok(mut call) = serde_wasm_bindgen::from_value::<JsCall>(c) else {
                continue;
            };
            if call.reporter_frame.is_none() {
                call.reporter_frame = frame.clone();
            }
            state.js_calls.push(call);
        }
        if state.js_calls.len() > MAX_JS_CALLS {
            let drop = state.js_calls.len() - MAX_JS_CALLS;
            state.js_calls.drain(..drop);
        }
        if let Some(h) = tab_host {
            state.page_host = Some(h);
        }
    });
    schedule_persist_behavior();
}

fn handle_scan(msg: &JsValue, sender: &JsValue) {
    let Some(tab_id) = sender_tab_id(sender) else {
        return;
    };
    let tab_host = sender_tab_host(sender);
    let frame_host = sender_frame_host(sender);
    let msg_host = Reflect::get(msg, &JsValue::from_str("hostname"))
        .ok()
        .and_then(|v| v.as_string());
    let resources = Reflect::get(msg, &JsValue::from_str("resources"))
        .ok()
        .and_then(|v| serde_wasm_bindgen::from_value::<Vec<Resource>>(v).ok());
    let iframes = Reflect::get(msg, &JsValue::from_str("iframes"))
        .ok()
        .and_then(|v| serde_wasm_bindgen::from_value::<Vec<IframeHit>>(v).ok());
    let stickies = Reflect::get(msg, &JsValue::from_str("stickies"))
        .ok()
        .and_then(|v| serde_wasm_bindgen::from_value::<Vec<StickyHit>>(v).ok());

    let reporter = frame_host.clone().or_else(|| msg_host.clone());

    get_behavior_mut(tab_id, |state| {
        if let Some(h) = tab_host {
            state.page_host = Some(h);
        } else if let Some(h) = &msg_host {
            if state.page_host.is_none() {
                state.page_host = Some(h.clone());
            }
        }
        if let Some(mut rs) = resources {
            let mut seen: std::collections::HashSet<String> = state
                .seen_resources
                .iter()
                .map(|r| format!("{}@{}", r.url, r.start_time))
                .collect();
            for r in rs.drain(..) {
                let k = format!("{}@{}", r.url, r.start_time);
                if seen.contains(&k) {
                    continue;
                }
                seen.insert(k);
                let mut r = r;
                r.reporter_frame = reporter.clone();
                state.seen_resources.push(r);
            }
            if state.seen_resources.len() > MAX_SEEN_RESOURCES {
                let drop = state.seen_resources.len() - MAX_SEEN_RESOURCES;
                state.seen_resources.drain(..drop);
            }
        }
        if let Some(mut ifs) = iframes {
            for f in ifs.iter_mut() {
                f.reporter_frame = reporter.clone();
            }
            state.latest_iframes = ifs;
        }
        if let Some(mut ss) = stickies {
            for s in ss.iter_mut() {
                s.reporter_frame = reporter.clone();
            }
            state.latest_stickies = ss;
        }
    });

    spawn_local(async move {
        let config = load_config().await.unwrap_or_default();
        let state_snapshot: BehaviorState = STATE.with(|s| {
            s.borrow()
                .tab_behavior
                .get(&tab_id)
                .cloned()
                .unwrap_or_default()
        });
        let suggestions = compute_suggestions_for(&state_snapshot, &config).await;
        let count = suggestions.len();
        STATE.with(|s| {
            if let Some(b) = s.borrow_mut().tab_behavior.get_mut(&tab_id) {
                b.suggestions = suggestions;
            }
        });
        schedule_persist_behavior();
        update_badge(tab_id);
        log(&format!(
            "scan merged for tab {tab_id} from frame {:?} - suggestions: {count}",
            reporter
        ));
    });
}

fn handle_get_tab_stats(msg: &JsValue, sender: &JsValue, send_response: &JsValue) {
    let tab_id = msg_or_sender_tab_id(msg, sender);
    let reply = Object::new();
    if let Some(id) = tab_id {
        let stats = STATE.with(|s| s.borrow().tab_stats.get(&id).cloned().unwrap_or_default());
        if let Ok(v) = serde_wasm_bindgen::to_value(&stats) {
            let _ = Reflect::set(&reply, &JsValue::from_str("stats"), &v);
        }
    } else {
        let _ = Reflect::set(&reply, &JsValue::from_str("stats"), &JsValue::NULL);
    }
    call_send_response(send_response, &reply.into());
}

fn handle_get_rule_diagnostics(msg: &JsValue, sender: &JsValue, send_response: &JsValue) {
    let tab_id = msg_or_sender_tab_id(msg, sender);
    let hostname = Reflect::get(msg, &JsValue::from_str("hostname"))
        .ok()
        .and_then(|v| v.as_string());
    let diag = compute_rule_diagnostics(tab_id, hostname.as_deref());
    let reply = Object::new();
    if let Ok(v) = serde_wasm_bindgen::to_value(&diag) {
        let _ = Reflect::set(&reply, &JsValue::from_str("diagnostics"), &v);
    }
    call_send_response(send_response, &reply.into());
}

fn handle_get_suggestions(msg: &JsValue, sender: &JsValue, send_response: JsValue) {
    let tab_id = msg_or_sender_tab_id(msg, sender);
    let Some(tab_id) = tab_id else {
        let reply = Object::new();
        let _ = Reflect::set(&reply, &JsValue::from_str("suggestions"), &Array::new());
        let _ = Reflect::set(&reply, &JsValue::from_str("pageHost"), &JsValue::NULL);
        call_send_response(&send_response, &reply.into());
        return;
    };
    spawn_local(async move {
        let state_snapshot: BehaviorState = STATE.with(|s| {
            s.borrow()
                .tab_behavior
                .get(&tab_id)
                .cloned()
                .unwrap_or_default()
        });
        let config = load_config().await.unwrap_or_default();
        let suggestions = compute_suggestions_for(&state_snapshot, &config).await;
        let page_host = state_snapshot.page_host.clone();
        STATE.with(|s| {
            if let Some(b) = s.borrow_mut().tab_behavior.get_mut(&tab_id) {
                b.suggestions = suggestions.clone();
            }
        });
        update_badge(tab_id);
        let reply = Object::new();
        if let Ok(v) = serde_wasm_bindgen::to_value(&suggestions) {
            let _ = Reflect::set(&reply, &JsValue::from_str("suggestions"), &v);
        }
        match page_host {
            Some(h) => {
                let _ = Reflect::set(
                    &reply,
                    &JsValue::from_str("pageHost"),
                    &JsValue::from_str(&h),
                );
            }
            None => {
                let _ = Reflect::set(&reply, &JsValue::from_str("pageHost"), &JsValue::NULL);
            }
        }
        call_send_response(&send_response, &reply.into());
    });
}

fn handle_accept_suggestion(msg: &JsValue, send_response: JsValue) {
    let hostname = Reflect::get(msg, &JsValue::from_str("hostname"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    let layer = Reflect::get(msg, &JsValue::from_str("layer"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    let value = Reflect::get(msg, &JsValue::from_str("value"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    if hostname.is_empty() || layer.is_empty() || value.is_empty() {
        let reply = Object::new();
        let _ = Reflect::set(&reply, &JsValue::from_str("ok"), &JsValue::FALSE);
        let _ = Reflect::set(
            &reply,
            &JsValue::from_str("error"),
            &JsValue::from_str("missing hostname/layer/value"),
        );
        call_send_response(&send_response, &reply.into());
        return;
    }
    spawn_local(async move {
        let mut config = load_config().await.unwrap_or_default();
        // Find existing matching key (exact or suffix) or insert new one.
        let target_key = if config.contains_key(&hostname) {
            hostname.clone()
        } else {
            let mut found: Option<String> = None;
            for key in config.keys() {
                if &hostname == key || hostname.ends_with(&format!(".{key}")) {
                    found = Some(key.clone());
                    break;
                }
            }
            match found {
                Some(k) => k,
                None => {
                    config.insert(hostname.clone(), crate::types::SiteConfig::default());
                    hostname.clone()
                }
            }
        };
        if let Some(entry) = config.get_mut(&target_key) {
            let arr = match layer.as_str() {
                "hide" => &mut entry.hide,
                "remove" => &mut entry.remove,
                "block" => &mut entry.block,
                _ => {
                    let reply = Object::new();
                    let _ = Reflect::set(&reply, &JsValue::from_str("ok"), &JsValue::FALSE);
                    let _ = Reflect::set(
                        &reply,
                        &JsValue::from_str("error"),
                        &JsValue::from_str(&format!("unknown layer '{layer}'")),
                    );
                    call_send_response(&send_response, &reply.into());
                    return;
                }
            };
            if !arr.iter().any(|v| v == &value) {
                arr.push(value.clone());
            }
        }
        // Write back.
        if let Ok(v) = serde_wasm_bindgen::to_value(&config) {
            if let Err(e) = storage_local_set_one(STORAGE_KEY, &v).await {
                log_error(&format!("accept-suggestion set config failed: {:?}", e));
            }
        }
        // Drop from every tab's suggestions + refresh badges.
        let accepted_key = format!("{}::{}", layer, value);
        let mutated: Vec<i32> = STATE.with(|s| {
            let mut st = s.borrow_mut();
            let mut out = Vec::new();
            for (tab_id, b) in st.tab_behavior.iter_mut() {
                let before = b.suggestions.len();
                b.suggestions.retain(|s| s.key != accepted_key);
                if b.suggestions.len() != before {
                    out.push(*tab_id);
                }
            }
            out
        });
        for id in mutated {
            update_badge(id);
        }
        schedule_persist_behavior();
        let reply = Object::new();
        let _ = Reflect::set(&reply, &JsValue::from_str("ok"), &JsValue::TRUE);
        let _ = Reflect::set(
            &reply,
            &JsValue::from_str("configKey"),
            &JsValue::from_str(&target_key),
        );
        call_send_response(&send_response, &reply.into());
    });
}

fn handle_allowlist_add_suggestion(msg: &JsValue, send_response: JsValue) {
    let key = Reflect::get(msg, &JsValue::from_str("key"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    if key.is_empty() {
        let reply = Object::new();
        let _ = Reflect::set(&reply, &JsValue::from_str("ok"), &JsValue::FALSE);
        let _ = Reflect::set(
            &reply,
            &JsValue::from_str("error"),
            &JsValue::from_str("missing key"),
        );
        call_send_response(&send_response, &reply.into());
        return;
    }
    spawn_local(async move {
        let mut al = STATE.with(|s| s.borrow().allowlist_cache.clone());
        // Re-read from storage to avoid a race where cache is stale.
        let raw = storage_local_get_one(ALLOWLIST_KEY)
            .await
            .unwrap_or(JsValue::NULL);
        if !raw.is_undefined() && !raw.is_null() {
            if let Ok(parsed) = serde_wasm_bindgen::from_value::<Allowlist>(raw) {
                al = parsed;
            }
        }
        if !al.suggestions.iter().any(|k| k == &key) {
            al.suggestions.push(key.clone());
        }
        if let Ok(v) = serde_wasm_bindgen::to_value(&al) {
            if let Err(e) = storage_local_set_one(ALLOWLIST_KEY, &v).await {
                log_error(&format!("allowlist-add set failed: {:?}", e));
            }
        }
        STATE.with(|s| s.borrow_mut().allowlist_cache = al);
        // Drop from every tab's suggestions + refresh badges.
        let mutated: Vec<i32> = STATE.with(|s| {
            let mut st = s.borrow_mut();
            let mut out = Vec::new();
            for (tab_id, b) in st.tab_behavior.iter_mut() {
                let before = b.suggestions.len();
                b.suggestions.retain(|s| s.key != key);
                if b.suggestions.len() != before {
                    out.push(*tab_id);
                }
            }
            out
        });
        for id in mutated {
            update_badge(id);
        }
        schedule_persist_behavior();
        let reply = Object::new();
        let _ = Reflect::set(&reply, &JsValue::from_str("ok"), &JsValue::TRUE);
        call_send_response(&send_response, &reply.into());
    });
}

fn handle_dismiss_suggestion(msg: &JsValue, sender: &JsValue, send_response: &JsValue) {
    let tab_id = msg_or_sender_tab_id(msg, sender);
    let key = Reflect::get(msg, &JsValue::from_str("key"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    if tab_id.is_none() || key.is_empty() {
        let reply = Object::new();
        let _ = Reflect::set(&reply, &JsValue::from_str("ok"), &JsValue::FALSE);
        call_send_response(send_response, &reply.into());
        return;
    }
    let tab_id = tab_id.unwrap();
    get_behavior_mut(tab_id, |state| {
        if !state.dismissed.iter().any(|k| k == &key) {
            state.dismissed.push(key.clone());
        }
        state.suggestions.retain(|s| s.key != key);
    });
    schedule_persist_behavior();
    update_badge(tab_id);
    let reply = Object::new();
    let _ = Reflect::set(&reply, &JsValue::from_str("ok"), &JsValue::TRUE);
    call_send_response(send_response, &reply.into());
}

fn handle_get_debug_info(msg: &JsValue, send_response: JsValue) {
    let tab_id = Reflect::get(msg, &JsValue::from_str("tabId"))
        .ok()
        .and_then(|v| v.as_f64())
        .map(|f| f as i32);
    spawn_local(async move {
        let manifest = get_manifest().unwrap_or(JsValue::NULL);
        let version = Reflect::get(&manifest, &JsValue::from_str("version"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let config = load_config().await.unwrap_or_default();
        let options = load_options_raw().await.unwrap_or(Object::new().into());
        let dynamic_rules = match chrome_dnr().and_then(|d| {
            get_fn_from(&d, "getDynamicRules")
                .and_then(|f| f.call0(&d))
                .and_then(|p| {
                    p.dyn_into::<Promise>()
                        .map_err(|_| JsValue::from_str("not a Promise"))
                })
        }) {
            Ok(p) => JsFuture::from(p).await.unwrap_or(Array::new().into()),
            Err(_) => Array::new().into(),
        };

        let (stats_for_tab, behavior_for_tab, matched_domain) = STATE.with(|s| {
            let st = s.borrow();
            let stats = tab_id.and_then(|id| st.tab_stats.get(&id).cloned());
            let behavior = tab_id.and_then(|id| st.tab_behavior.get(&id).cloned());
            let matched = stats.as_ref().and_then(|s| s.matched_domain.clone());
            (stats, behavior, matched)
        });

        // Build compact rules summary.
        let compact_rules = Array::new();
        let mut rule_count = 0;
        if let Ok(arr) = dynamic_rules.clone().dyn_into::<Array>() {
            for i in 0..arr.length() {
                let r = arr.get(i);
                let id = Reflect::get(&r, &JsValue::from_str("id"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map(|f| f as i32);
                let pattern = Reflect::get(&r, &JsValue::from_str("condition"))
                    .ok()
                    .and_then(|c| Reflect::get(&c, &JsValue::from_str("urlFilter")).ok())
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                let source_domain = id
                    .and_then(|i| STATE.with(|s| s.borrow().rule_patterns.get(&i).cloned()))
                    .map(|m| m.domain)
                    .unwrap_or_default();
                let rule = Object::new();
                let _ = Reflect::set(
                    &rule,
                    &JsValue::from_str("id"),
                    &id.map(|v| JsValue::from_f64(v as f64)).unwrap_or(JsValue::NULL),
                );
                let _ = Reflect::set(&rule, &JsValue::from_str("pattern"), &JsValue::from_str(&pattern));
                let _ = Reflect::set(
                    &rule,
                    &JsValue::from_str("sourceDomain"),
                    &JsValue::from_str(&source_domain),
                );
                compact_rules.push(&rule);
                rule_count += 1;
            }
        }

        let reply = Object::new();
        let _ = Reflect::set(&reply, &JsValue::from_str("version"), &JsValue::from_str(&version));
        let _ = Reflect::set(
            &reply,
            &JsValue::from_str("tabId"),
            &tab_id
                .map(|v| JsValue::from_f64(v as f64))
                .unwrap_or(JsValue::NULL),
        );
        let _ = Reflect::set(
            &reply,
            &JsValue::from_str("timestamp"),
            &JsValue::from_str(&iso_now()),
        );
        let _ = Reflect::set(&reply, &JsValue::from_str("options"), &options);
        let _ = Reflect::set(
            &reply,
            &JsValue::from_str("configSiteCount"),
            &JsValue::from_f64(config.len() as f64),
        );
        let keys_arr = Array::new();
        for k in config.keys() {
            keys_arr.push(&JsValue::from_str(k));
        }
        let _ = Reflect::set(&reply, &JsValue::from_str("configSites"), &keys_arr);
        let _ = Reflect::set(
            &reply,
            &JsValue::from_str("matchedDomain"),
            &matched_domain
                .as_deref()
                .map(JsValue::from_str)
                .unwrap_or(JsValue::NULL),
        );
        // matchedConfig: the per-site triple for the matched domain
        let matched_cfg = matched_domain
            .as_ref()
            .and_then(|d| config.get(d))
            .map(|c| serde_wasm_bindgen::to_value(c).unwrap_or(JsValue::NULL))
            .unwrap_or(JsValue::NULL);
        let _ = Reflect::set(&reply, &JsValue::from_str("matchedConfig"), &matched_cfg);

        // tabActivity summary.
        if let Some(stats) = stats_for_tab {
            let activity = Object::new();
            let _ = Reflect::set(
                &activity,
                &JsValue::from_str("totalBlocks"),
                &JsValue::from_f64(stats.block as f64),
            );
            let total_hide: u32 = stats.hide.values().sum();
            let total_remove: u32 = stats.remove.values().sum();
            let _ = Reflect::set(
                &activity,
                &JsValue::from_str("totalHide"),
                &JsValue::from_f64(total_hide as f64),
            );
            let _ = Reflect::set(
                &activity,
                &JsValue::from_str("totalRemove"),
                &JsValue::from_f64(total_remove as f64),
            );
            let _ = Reflect::set(
                &activity,
                &JsValue::from_str("hide"),
                &serde_wasm_bindgen::to_value(&stats.hide).unwrap_or(JsValue::NULL),
            );
            let _ = Reflect::set(
                &activity,
                &JsValue::from_str("remove"),
                &serde_wasm_bindgen::to_value(&stats.remove).unwrap_or(JsValue::NULL),
            );
            let recent_blocks_arr = Array::new();
            let start = stats.blocked_urls.len().saturating_sub(10);
            for b in &stats.blocked_urls[start..] {
                let rb = Object::new();
                let _ = Reflect::set(&rb, &JsValue::from_str("t"), &JsValue::from_str(&b.t));
                let url_trunc: String = b.url.chars().take(200).collect();
                let _ = Reflect::set(&rb, &JsValue::from_str("url"), &JsValue::from_str(&url_trunc));
                let _ = Reflect::set(
                    &rb,
                    &JsValue::from_str("pattern"),
                    &JsValue::from_str(&b.pattern),
                );
                let _ = Reflect::set(
                    &rb,
                    &JsValue::from_str("type"),
                    &b.resource_type.as_deref().map(JsValue::from_str).unwrap_or(JsValue::NULL),
                );
                recent_blocks_arr.push(&rb);
            }
            let _ = Reflect::set(
                &activity,
                &JsValue::from_str("recentBlockedUrls"),
                &recent_blocks_arr,
            );
            let recent_removed: Vec<&RemovedElement> = stats.removed_elements.iter().rev().take(10).collect();
            let recent_removed_arr = Array::new();
            for r in recent_removed.iter().rev() {
                recent_removed_arr.push(&serde_wasm_bindgen::to_value(*r).unwrap_or(JsValue::NULL));
            }
            let _ = Reflect::set(
                &activity,
                &JsValue::from_str("recentRemovedElements"),
                &recent_removed_arr,
            );
            let _ = Reflect::set(&reply, &JsValue::from_str("tabActivity"), &activity);
        } else {
            let _ = Reflect::set(&reply, &JsValue::from_str("tabActivity"), &JsValue::NULL);
        }

        if let Some(b) = behavior_for_tab {
            let summary = Object::new();
            let _ = Reflect::set(
                &summary,
                &JsValue::from_str("pageHost"),
                &b.page_host
                    .as_deref()
                    .map(JsValue::from_str)
                    .unwrap_or(JsValue::NULL),
            );
            let _ = Reflect::set(
                &summary,
                &JsValue::from_str("seenResourceCount"),
                &JsValue::from_f64(b.seen_resources.len() as f64),
            );
            let unique_hosts: std::collections::HashSet<&str> = b
                .seen_resources
                .iter()
                .map(|r| r.host.as_str())
                .filter(|h| {
                    !h.is_empty() && b.page_host.as_deref().map(|p| p != *h).unwrap_or(true)
                })
                .collect();
            let _ = Reflect::set(
                &summary,
                &JsValue::from_str("uniqueThirdPartyHostCount"),
                &JsValue::from_f64(unique_hosts.len() as f64),
            );
            let _ = Reflect::set(
                &summary,
                &JsValue::from_str("latestHiddenIframeCount"),
                &JsValue::from_f64(b.latest_iframes.len() as f64),
            );
            let _ = Reflect::set(
                &summary,
                &JsValue::from_str("latestStickyCount"),
                &JsValue::from_f64(b.latest_stickies.len() as f64),
            );
            let _ = Reflect::set(
                &summary,
                &JsValue::from_str("jsCallCount"),
                &JsValue::from_f64(b.js_calls.len() as f64),
            );
            let mut by_kind: HashMap<String, u32> = HashMap::new();
            for c in &b.js_calls {
                *by_kind.entry(c.kind.clone()).or_insert(0) += 1;
            }
            let _ = Reflect::set(
                &summary,
                &JsValue::from_str("jsCallsByKind"),
                &serde_wasm_bindgen::to_value(&by_kind).unwrap_or(JsValue::NULL),
            );
            let _ = Reflect::set(
                &summary,
                &JsValue::from_str("dismissedKeyCount"),
                &JsValue::from_f64(b.dismissed.len() as f64),
            );
            let _ = Reflect::set(
                &summary,
                &JsValue::from_str("suggestionCount"),
                &JsValue::from_f64(b.suggestions.len() as f64),
            );
            let _ = Reflect::set(
                &summary,
                &JsValue::from_str("suggestions"),
                &serde_wasm_bindgen::to_value(&b.suggestions).unwrap_or(JsValue::NULL),
            );
            let _ = Reflect::set(&reply, &JsValue::from_str("behavior"), &summary);
        } else {
            let _ = Reflect::set(&reply, &JsValue::from_str("behavior"), &JsValue::NULL);
        }

        let _ = Reflect::set(&reply, &JsValue::from_str("dynamicRules"), &compact_rules);
        let _ = Reflect::set(
            &reply,
            &JsValue::from_str("dynamicRuleCount"),
            &JsValue::from_f64(rule_count as f64),
        );
        let logs: Vec<LogEntry> = STATE.with(|s| {
            let st = s.borrow();
            let start = st.log_buffer.len().saturating_sub(40);
            st.log_buffer.iter().skip(start).cloned().collect()
        });
        let _ = Reflect::set(
            &reply,
            &JsValue::from_str("recentLogs"),
            &serde_wasm_bindgen::to_value(&logs).unwrap_or(JsValue::NULL),
        );
        call_send_response(&send_response, &reply.into());
    });
}

fn get_manifest() -> Result<JsValue, JsValue> {
    let runtime = chrome_runtime()?;
    let f: Function = Reflect::get(&runtime, &JsValue::from_str("getManifest"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("chrome.runtime.getManifest is not a function"))?;
    f.call0(&runtime)
}

// ---------------------------------------------------------------------------
// Sender helpers.

fn sender_tab_id(sender: &JsValue) -> Option<i32> {
    Reflect::get(sender, &JsValue::from_str("tab"))
        .ok()
        .and_then(|t| Reflect::get(&t, &JsValue::from_str("id")).ok())
        .and_then(|v| v.as_f64())
        .map(|f| f as i32)
}

fn sender_tab_host(sender: &JsValue) -> Option<String> {
    Reflect::get(sender, &JsValue::from_str("tab"))
        .ok()
        .and_then(|t| Reflect::get(&t, &JsValue::from_str("url")).ok())
        .and_then(|v| v.as_string())
        .and_then(|url| {
            url::Url::parse(&url)
                .ok()
                .and_then(|u| u.host_str().map(String::from))
        })
}

fn sender_frame_host(sender: &JsValue) -> Option<String> {
    Reflect::get(sender, &JsValue::from_str("url"))
        .ok()
        .and_then(|v| v.as_string())
        .and_then(|url| {
            url::Url::parse(&url)
                .ok()
                .and_then(|u| u.host_str().map(String::from))
        })
}

fn msg_or_sender_tab_id(msg: &JsValue, sender: &JsValue) -> Option<i32> {
    Reflect::get(msg, &JsValue::from_str("tabId"))
        .ok()
        .and_then(|v| v.as_f64())
        .map(|f| f as i32)
        .or_else(|| sender_tab_id(sender))
}

fn call_send_response(send_response: &JsValue, payload: &JsValue) {
    if let Ok(f) = send_response.clone().dyn_into::<Function>() {
        let _ = f.call1(&JsValue::NULL, payload);
    }
}

// ---------------------------------------------------------------------------
// Small JS helpers.

fn iso_now() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}

fn set_timeout<F: FnOnce() + 'static>(f: F, ms: i32) {
    // setTimeout exists on both Window and ServiceWorkerGlobalScope,
    // but neither interface is typed via `web_sys::window()` in SW
    // context. Call the global-scope method via Reflect instead.
    let g = global_scope();
    let Ok(set_timeout_fn) = Reflect::get(&g, &JsValue::from_str("setTimeout"))
        .and_then(|v| {
            v.dyn_into::<Function>()
                .map_err(|_| JsValue::from_str("setTimeout is not a function"))
        })
    else {
        return;
    };
    let cell = std::cell::Cell::new(Some(f));
    let cb = Closure::<dyn Fn()>::new(move || {
        if let Some(f) = cell.take() {
            f();
        }
    });
    let _ = set_timeout_fn.call2(
        &g,
        cb.as_ref(),
        &JsValue::from_f64(ms as f64),
    );
    cb.forget();
}
