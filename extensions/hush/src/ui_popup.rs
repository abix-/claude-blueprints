//! Popup UI (Stage 4).
//!
//! Leptos CSR component tree for `popup.html`. `popup.js` queries
//! chrome.tabs + chrome.runtime for an initial snapshot and hands it
//! to [`mount_popup`]. Leptos owns the matched-site header, the
//! activity summary, and the suggestions list (including the
//! Add / Dismiss / Allow actions, which call chrome.runtime.sendMessage
//! via [`crate::chrome_bridge`] directly). The per-section JS
//! renderers for blocked URLs, removed-element evidence, and block
//! diagnostics still live in popup.js and get ported in follow-up
//! iterations.

use crate::chrome_bridge;
use crate::types::{
    BlockDiagnostic, BlockedUrl, FirewallEvent, FirewallEvidence, RemovedElement, SiteConfig,
    Suggestion, SuggestionDiag, SuggestionLayer, GLOBAL_SCOPE_KEY,
};
use indexmap::IndexMap;
use js_sys::Reflect;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Fire-and-forget clipboard write. Logs the failure and moves on
/// instead of propagating the error back to the UI thread - the
/// button already shows "Copy failed" visual feedback through the
/// button-label swap in [`CopyButton`].
fn clipboard_write(text: &str) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    // web_sys exposes Navigator::clipboard() behind a Clipboard feature.
    // Use Reflect to avoid panicking on browsers that don't expose it.
    let clipboard = Reflect::get(&navigator, &JsValue::from_str("clipboard"))
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null());
    let Some(clipboard) = clipboard else {
        return false;
    };
    let write = match Reflect::get(&clipboard, &JsValue::from_str("writeText")) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let Ok(func) = write.dyn_into::<js_sys::Function>() else {
        return false;
    };
    func.call1(&clipboard, &JsValue::from_str(text)).is_ok()
}

// Handle to the popup's top-level suggestions signal + its tab id.
// Populated on mount, consumed by [`refresh_popup_suggestions`] when
// JS-side buttons (Enable / Scan once / Rescan) want to re-trigger a
// fetch without re-mounting the component tree. Single-threaded WASM
// so a `thread_local!(RefCell<Option<_>>)` is the right primitive.
thread_local! {
    static POPUP_HANDLE: RefCell<Option<PopupHandle>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct PopupHandle {
    suggestions: RwSignal<Vec<Suggestion>>,
    tab_id: Option<i32>,
}

/// Called by `popup.js` after enable/scan/rescan clicks so the Leptos
/// suggestion list refreshes without tearing down the whole component
/// tree. No-op if no popup is currently mounted or we have no tab id.
#[wasm_bindgen(js_name = "refreshPopupSuggestions")]
pub fn refresh_popup_suggestions() {
    let Some(handle) = POPUP_HANDLE.with(|h| h.borrow().clone()) else {
        return;
    };
    let Some(tab_id) = handle.tab_id else {
        return;
    };
    spawn_local(async move {
        match chrome_bridge::get_suggestions(tab_id).await {
            Ok(next) => handle.suggestions.set(next),
            Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!(
                "[Hush popup] refresh_popup_suggestions: {:?}",
                e
            ))),
        }
    });
}

/// Snapshot popup.js hands in at mount time.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PopupSnapshot {
    pub hostname: String,
    pub matched_domain: Option<String>,
    pub block_count: u32,
    pub remove_count: u32,
    pub hide_count: u32,
    pub suggestion_count: u32,
    /// Active tab id. Needed for per-tab dismiss + re-fetch calls.
    /// `None` when the popup opens outside a normal tab context.
    pub tab_id: Option<i32>,
    /// Initial suggestions list. Leptos re-fetches after each action
    /// mutation, but the initial render avoids the round-trip.
    pub suggestions: Vec<Suggestion>,
    /// Whether the behavioral detector is enabled in user options.
    /// Affects the "enable detector" CTA copy.
    pub detector_enabled: bool,
    /// True when a site config matched this tab's hostname. Hides the
    /// per-section diagnostic panels when there's no config to
    /// interpret them against.
    #[serde(default)]
    pub is_matched: bool,
    /// Recent DNR-rule fires for this tab. Rendered as a by-pattern
    /// summary plus a collapsible URL list.
    #[serde(default)]
    pub blocked_urls: Vec<BlockedUrl>,
    /// Per-rule diagnostic rows for the Blocked section. Each entry
    /// represents one configured block rule with its fire count,
    /// status, and any broken-pattern hint.
    #[serde(default)]
    pub block_diagnostics: Vec<BlockDiagnostic>,
    /// Remove-layer selectors observed on this tab, mapped to their
    /// match counts. `IndexMap` preserves the insertion order the
    /// content script reported so the popup's list matches the order
    /// the user authored the selectors in the site config.
    #[serde(default)]
    pub remove_selectors: IndexMap<String, u32>,
    /// Hide-layer selectors observed on this tab, mapped to their
    /// match counts. Same ordering guarantee as `remove_selectors`.
    #[serde(default)]
    pub hide_selectors: IndexMap<String, u32>,
    /// Recently-removed DOM elements reported by the content script.
    /// Rendered as a collapsible evidence panel under the Removed
    /// section.
    #[serde(default)]
    pub removed_elements: Vec<RemovedElement>,
    /// The active tab's full URL. Used by the Debug button to include
    /// it in the clipboard payload. Empty when there's no active tab.
    #[serde(default)]
    pub tab_url: String,
    /// Unified firewall-log event buffer for the active tab. Fed to
    /// the `FirewallLog` component so it can aggregate by rule_id
    /// and render Palo-Alto-style per-rule hit counts + recent
    /// evidence.
    #[serde(default)]
    pub events: Vec<FirewallEvent>,
    /// Global-scope rules active on this tab (from the reserved
    /// `__global__` config entry, if any). Rendered alongside
    /// site-scoped rules in the firewall log.
    #[serde(default)]
    pub global_rules: SiteConfig,
    /// Site-scoped rules for the matched domain (empty when no
    /// site config matched).
    #[serde(default)]
    pub site_rules: SiteConfig,
}

/// WASM entry. Called by `popup.js` once per popup open.
#[wasm_bindgen(js_name = "mountPopup")]
pub fn mount_popup(snapshot: JsValue) -> Result<(), JsValue> {
    let snap: PopupSnapshot = serde_wasm_bindgen::from_value(snapshot)
        .map_err(|e| JsValue::from_str(&format!("mountPopup: {e}")))?;
    mount_popup_inner(snap)
}

fn mount_popup_inner(snap: PopupSnapshot) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let root = document
        .get_element_by_id("rust-popup-root")
        .ok_or_else(|| JsValue::from_str("no #rust-popup-root in popup.html"))?;
    let root_el: web_sys::HtmlElement = root
        .dyn_into::<web_sys::HtmlElement>()
        .map_err(|_| JsValue::from_str("#rust-popup-root is not an HtmlElement"))?;

    std::mem::forget(leptos::mount::mount_to(root_el, move || {
        view! { <PopupRoot snap=snap.clone() /> }
    }));
    Ok(())
}

/// One-shot popup bootstrap. Queries the active tab, fetches every
/// piece of state the Leptos tree needs (per-tab stats, suggestions,
/// rule diagnostics, persisted config + options), computes the
/// `matched_domain` via hostname-suffix lookup, and mounts the tree.
/// The JS bootstrap is now just `await initWasm(); hushPopupMain();`.
#[wasm_bindgen(js_name = "hushPopupMain")]
pub async fn hush_popup_main() -> Result<(), JsValue> {
    let tab = chrome_bridge::get_active_tab().await?;
    let hostname = safe_hostname(&tab.url);

    // No tab (devtools / extension page): mount a minimal empty
    // snapshot so the UI still renders without the per-tab sections.
    let Some(tab_id) = tab.tab_id else {
        return mount_popup_inner(PopupSnapshot {
            hostname: hostname.clone(),
            tab_url: tab.url.clone(),
            ..PopupSnapshot::default()
        });
    };
    let _ = hostname; // remains valid below; above branch took a copy.

    // Sequential fetch: stats, suggestions, diagnostics, stored
    // options + config. Each branch degrades independently so a dead
    // background service worker never prevents the popup from
    // rendering something. The per-branch cost is ~1-3ms of in-process
    // message round-trips; parallelizing would shave ~10ms and isn't
    // worth the unsafe-pin-projection machinery it requires.
    let stats = chrome_bridge::get_tab_stats(tab_id).await.unwrap_or_default();
    let suggestions = chrome_bridge::get_suggestions(tab_id)
        .await
        .unwrap_or_default();
    let diagnostics = chrome_bridge::get_rule_diagnostics(tab_id, &hostname)
        .await
        .unwrap_or_default();
    let storage = chrome_bridge::get_popup_storage().await.unwrap_or(
        chrome_bridge::PopupStorage {
            detector_enabled: false,
            config: crate::types::Config::default(),
        },
    );
    let events = chrome_bridge::get_firewall_events(tab_id)
        .await
        .unwrap_or_default();

    // matched_domain resolves from the tab's running stats first, then
    // falls back to a hostname-suffix lookup in the user config. This
    // mirrors the JS popup's `stats.matchedDomain || configMatch.key`.
    let config_match_key = find_config_entry_key(&storage.config, &hostname);
    let matched_domain = stats.matched_domain.clone().or(config_match_key);
    let is_matched = matched_domain.is_some();

    let block_count = stats.block;
    let remove_count: u32 = stats.remove.values().sum();
    let hide_count: u32 = stats.hide.values().sum();
    let suggestion_count = suggestions.len() as u32;

    let global_rules = storage
        .config
        .get(GLOBAL_SCOPE_KEY)
        .cloned()
        .unwrap_or_default();
    let site_rules = matched_domain
        .as_ref()
        .and_then(|d| storage.config.get(d))
        .cloned()
        .unwrap_or_default();
    let snap = PopupSnapshot {
        hostname,
        matched_domain,
        block_count,
        remove_count,
        hide_count,
        suggestion_count,
        tab_id: Some(tab_id),
        suggestions,
        detector_enabled: storage.detector_enabled,
        is_matched,
        blocked_urls: stats.blocked_urls,
        block_diagnostics: diagnostics,
        events,
        global_rules,
        site_rules,
        remove_selectors: stats.remove,
        hide_selectors: stats.hide,
        removed_elements: stats.removed_elements,
        tab_url: tab.url,
    };
    mount_popup_inner(snap)
}

fn safe_hostname(url: &str) -> String {
    if url.is_empty() {
        return String::new();
    }
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_default()
}

fn find_config_entry_key(config: &crate::types::Config, host: &str) -> Option<String> {
    if config.contains_key(host) {
        return Some(host.to_string());
    }
    for key in config.keys() {
        if host == key || host.ends_with(&format!(".{key}")) {
            return Some(key.clone());
        }
    }
    None
}

#[component]
fn PopupRoot(snap: PopupSnapshot) -> impl IntoView {
    // Suggestions are the only reactive state on the popup today.
    // Other fields are init-only; we read them once.
    let suggestions = RwSignal::new(snap.suggestions.clone());
    let tab_id = snap.tab_id;
    let hostname = snap.hostname.clone();

    // Expose the signal to JS-side buttons via refresh_popup_suggestions.
    POPUP_HANDLE.with(|h| {
        *h.borrow_mut() = Some(PopupHandle { suggestions, tab_id });
    });

    view! {
        <MatchedSite
            hostname=snap.hostname.clone()
            matched_domain=snap.matched_domain.clone()
        />
        <ActivitySummary
            block_count=snap.block_count
            remove_count=snap.remove_count
            hide_count=snap.hide_count
            suggestion_count=snap.suggestion_count
        />
        {if !snap.is_matched {
            view! {
                <UnmatchedBanner hostname=snap.hostname.clone() />
            }.into_any()
        } else { view! { <div /> }.into_any() }}
        <SuggestionsList
            suggestions=suggestions
            hostname=hostname
            tab_id=tab_id
        />
        <DetectorCta
            detector_enabled=snap.detector_enabled
            tab_id=tab_id
        />
        {
            let any_rules = !snap.global_rules.block.is_empty()
                || !snap.global_rules.remove.is_empty()
                || !snap.global_rules.hide.is_empty()
                || !snap.global_rules.spoof.is_empty()
                || !snap.site_rules.block.is_empty()
                || !snap.site_rules.remove.is_empty()
                || !snap.site_rules.hide.is_empty()
                || !snap.site_rules.spoof.is_empty();
            if any_rules || !snap.events.is_empty() {
                view! {
                    <FirewallLog
                        events=snap.events.clone()
                        global_rules=snap.global_rules.clone()
                        site_rules=snap.site_rules.clone()
                        site_scope=snap.matched_domain.clone()
                            .unwrap_or_else(|| snap.hostname.clone())
                    />
                }.into_any()
            } else { view! { <div /> }.into_any() }
        }
        {if snap.is_matched {
            view! {
                <div class="rust-sections" style="padding: 4px 0;">
                    <BlockedSection
                        blocked_urls=snap.blocked_urls.clone()
                        diagnostics=snap.block_diagnostics.clone()
                        block_count=snap.block_count
                    />
                    <RemovedSection
                        selectors=snap.remove_selectors.clone()
                        removed_elements=snap.removed_elements.clone()
                    />
                    <HiddenSection
                        selectors=snap.hide_selectors.clone()
                        remove_selectors=snap.remove_selectors.clone()
                    />
                </div>
            }.into_any()
        } else { view! { <div /> }.into_any() }}
        <FooterButtons
            tab_id=tab_id
            tab_url=snap.tab_url.clone()
            hostname=snap.hostname.clone()
        />
    }
}

/// "No config matched for this site" placeholder with a Create-site
/// button that seeds an empty triple under the current hostname and
/// re-runs the popup bootstrap. Replaces the `#unmatched` +
/// `#create-site` DOM that used to live in `popup.html`.
#[component]
fn UnmatchedBanner(hostname: String) -> impl IntoView {
    let busy = RwSignal::new(false);
    let on_create = move |_| {
        if busy.get() || hostname.is_empty() {
            return;
        }
        busy.set(true);
        let hostname = hostname.clone();
        spawn_local(async move {
            // Read current config, insert an empty triple under
            // hostname if absent, write it back, then re-run
            // hush_popup_main so the Leptos tree re-mounts with the
            // new matched state.
            match chrome_bridge::get_popup_storage().await {
                Ok(storage) => {
                    let mut cfg = storage.config;
                    if !cfg.contains_key(&hostname) {
                        cfg.insert(
                            hostname.clone(),
                            crate::types::SiteConfig::default(),
                        );
                        let _ = chrome_bridge::set_config(&cfg).await;
                    }
                    // Clear the existing Leptos tree by blanking the
                    // root, then re-run the bootstrap. Simpler than
                    // plumbing a refresh signal through every sub-tree.
                    if let Some(document) =
                        web_sys::window().and_then(|w| w.document())
                    {
                        if let Some(root) =
                            document.get_element_by_id("rust-popup-root")
                        {
                            root.set_inner_html("");
                        }
                    }
                    let _ = hush_popup_main().await;
                }
                Err(e) => {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "[Hush popup] create-site: {:?}",
                        e
                    )));
                    busy.set(false);
                }
            }
        });
    };

    view! {
        <div class="rust-unmatched"
             style="padding: 12px 14px; color: #666; text-align: center;
                    background: #fafafa; border-bottom: 1px solid #eee;">
            "No config matched for this site."
            <div style="margin-top: 8px;">
                <button on:click=on_create
                        disabled=move || busy.get()
                        style="padding: 6px 14px; font-size: 12px;
                               background: #fff; border: 1px solid #ccc;
                               border-radius: 5px; cursor: pointer;">
                    "Create config for this site"
                </button>
            </div>
        </div>
    }
}

/// Footer buttons: Options (opens the options page), Reload (reloads
/// the active tab), Debug (copies a JSON debug snapshot to the
/// clipboard). Replaces the `<footer>` block that used to live in
/// `popup.html`.
#[component]
fn FooterButtons(
    tab_id: Option<i32>,
    tab_url: String,
    hostname: String,
) -> impl IntoView {
    let debug_label = RwSignal::new("Debug");

    let on_options = move |_| {
        if let Err(e) = chrome_bridge::open_options_page() {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "[Hush popup] openOptionsPage: {:?}",
                e
            )));
        }
    };

    let on_reload = move |_| {
        if let Some(tid) = tab_id {
            if let Err(e) = chrome_bridge::reload_tab(tid) {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "[Hush popup] reload_tab: {:?}",
                    e
                )));
            }
        }
    };

    let on_debug = {
        let tab_url = tab_url.clone();
        let hostname = hostname.clone();
        move |_| {
            let tab_url = tab_url.clone();
            let hostname = hostname.clone();
            spawn_local(async move {
                let debug_info = chrome_bridge::get_debug_info(tab_id)
                    .await
                    .unwrap_or(JsValue::NULL);
                // Build a payload object: spread debug_info, add
                // url + hostname. Matches the JS popup's shape.
                let payload = js_sys::Object::new();
                if debug_info.is_object() {
                    let _ = js_sys::Object::assign(&payload, debug_info.unchecked_ref());
                }
                let _ = js_sys::Reflect::set(
                    &payload,
                    &JsValue::from_str("url"),
                    &JsValue::from_str(&tab_url),
                );
                let _ = js_sys::Reflect::set(
                    &payload,
                    &JsValue::from_str("hostname"),
                    &JsValue::from_str(&hostname),
                );
                let json = js_sys::JSON::stringify_with_replacer_and_space(
                    &payload,
                    &JsValue::NULL,
                    &JsValue::from_f64(2.0),
                )
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
                let ok = clipboard_write(&json);
                debug_label.set(if ok { "Copied!" } else { "Copy failed" });
                // Revert after 2s.
                if let Some(window) = web_sys::window() {
                    let cb = Closure::<dyn Fn()>::new(move || debug_label.set("Debug"));
                    let _ = window
                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                            cb.as_ref().unchecked_ref(),
                            2000,
                        );
                    cb.forget();
                }
            });
        }
    };

    view! {
        <footer style="display: flex; gap: 8px; padding: 10px 14px;
                       border-top: 1px solid #eee; background: #fafafa;">
            <button on:click=on_options
                    style="flex: 1; padding: 6px 10px; font-size: 12px;
                           border: 1px solid #ccc; background: #fff;
                           border-radius: 5px; cursor: pointer;">
                "Open options"
            </button>
            <button on:click=on_reload
                    disabled=move || tab_id.is_none()
                    style="flex: 1; padding: 6px 10px; font-size: 12px;
                           border: 1px solid #ccc; background: #fff;
                           border-radius: 5px; cursor: pointer;">
                "Reload tab"
            </button>
            <button on:click=on_debug
                    title="Copy debug info to clipboard"
                    style="flex: 1; padding: 6px 10px; font-size: 12px;
                           border: 1px solid #ccc; background: #fff;
                           border-radius: 5px; cursor: pointer;">
                {move || debug_label.get()}
            </button>
        </footer>
    }
}

/// Firewall log — unified per-rule view. Enumerates every rule in
/// the tab's active policy (global + site-scoped), joins in any
/// matching [`FirewallEvent`]s to show hit counts + most-recent
/// evidence, and renders one row per rule. Palo-Alto-flavored:
/// rules that have fired surface with a hit count; rules that
/// haven't show `no traffic` / `no hits` so the user can spot a
/// rule that's never catching.
#[component]
fn FirewallLog(
    events: Vec<FirewallEvent>,
    global_rules: SiteConfig,
    site_rules: SiteConfig,
    site_scope: String,
) -> impl IntoView {
    use std::collections::HashMap;
    // Group events by rule_id so each rule gets its aggregated hit
    // count + recent-evidence list. Newest events appended last by
    // the background ring buffer, so we don't need to re-sort here.
    let mut by_rule: HashMap<String, Vec<FirewallEvent>> = HashMap::new();
    for ev in events {
        by_rule.entry(ev.rule_id.clone()).or_default().push(ev);
    }

    // Enumerate every configured rule (scope × action × match). Rule
    // order: block > remove > hide > spoof within each scope; global
    // scope before site scope. This matches the aggressiveness
    // ordering used elsewhere in the UI.
    let mut rows: Vec<RuleRow> = Vec::new();
    fn emit_rows(
        rows: &mut Vec<RuleRow>,
        scope: &str,
        cfg: &SiteConfig,
        by_rule: &HashMap<String, Vec<FirewallEvent>>,
    ) {
        let blocks_iter = cfg.block.iter().map(|m| ("block", m.as_str()));
        let removes_iter = cfg.remove.iter().map(|m| ("remove", m.as_str()));
        let hides_iter = cfg.hide.iter().map(|m| ("hide", m.as_str()));
        let spoofs_iter = cfg.spoof.iter().map(|m| ("spoof", m.as_str()));
        for (action, m) in blocks_iter
            .chain(removes_iter)
            .chain(hides_iter)
            .chain(spoofs_iter)
        {
            let id = crate::types::rule_id(action, scope, m);
            let hits = by_rule.get(&id).map(|v| v.len() as u32).unwrap_or(0);
            let last_t = by_rule
                .get(&id)
                .and_then(|v| v.last())
                .map(|e| e.t.clone());
            rows.push(RuleRow {
                rule_id: id,
                action: action.to_string(),
                scope: scope.to_string(),
                match_: m.to_string(),
                hits,
                last_t,
            });
        }
    }
    emit_rows(&mut rows, GLOBAL_SCOPE_KEY, &global_rules, &by_rule);
    if !site_scope.is_empty() {
        emit_rows(&mut rows, &site_scope, &site_rules, &by_rule);
    }

    // Append "orphan" rule rows for events whose rule_id isn't in
    // the configured rules. Shouldn't normally happen, but guards
    // against config-drifted events showing up nowhere. Rare enough
    // that we don't bother prettifying.
    let configured_ids: std::collections::HashSet<String> =
        rows.iter().map(|r| r.rule_id.clone()).collect();
    for (rid, evs) in &by_rule {
        if configured_ids.contains(rid) {
            continue;
        }
        if let Some(first) = evs.first() {
            rows.push(RuleRow {
                rule_id: rid.clone(),
                action: first.action.clone(),
                scope: first.scope.clone(),
                match_: first.match_.clone(),
                hits: evs.len() as u32,
                last_t: evs.last().map(|e| e.t.clone()),
            });
        }
    }

    // Sort: rows with hits first (DESC), then unfired rules (by
    // action-then-scope-then-match lexicographic). Matches how a
    // firewall UI surfaces "things that fired" over "things that
    // didn't yet".
    rows.sort_by(|a, b| b.hits.cmp(&a.hits).then(a.rule_id.cmp(&b.rule_id)));

    let total_hits: u32 = rows.iter().map(|r| r.hits).sum();
    let rule_count = rows.len();

    view! {
        <details class="section firewall-log" open=true>
            <summary class="section-head">
                <span class="section-name">"Firewall log"</span>
                <span class="count" style="background: #2d4d8a; color: #fff;">
                    {total_hits}
                </span>
            </summary>
            <div style="color: #888; font-size: 10px; margin-top: 4px;">
                {rule_count} " rule" {if rule_count == 1 { "" } else { "s" }}
                " active on this tab (" {total_hits} " total hits)"
            </div>
            {if rows.is_empty() {
                view! {
                    <div class="no-sels">"No rules configured for this tab"</div>
                }.into_any()
            } else {
                rows.into_iter().map(|row| {
                    let events_for_row = by_rule.get(&row.rule_id).cloned().unwrap_or_default();
                    view! {
                        <FirewallLogRow row=row events=events_for_row />
                    }
                }).collect::<Vec<_>>().into_any()
            }}
        </details>
    }
}

/// One row in the firewall-log table. State is local (each row has
/// its own expand/collapse signal) so expanding a busy rule doesn't
/// re-render the whole log.
#[component]
fn FirewallLogRow(row: RuleRow, events: Vec<FirewallEvent>) -> impl IntoView {
    let open = RwSignal::new(false);
    let can_expand = !events.is_empty();

    let (badge_bg, badge_fg, badge_label) = match row.action.as_str() {
        "block" => ("#d85c4f", "#fff", "BLOCK"),
        "remove" => ("#d89a4f", "#fff", "REMOVE"),
        "hide" => ("#6b8ad4", "#fff", "HIDE"),
        "spoof" => ("#8a4fc3", "#fff", "SPOOF"),
        _ => ("#666", "#fff", "?"),
    };
    let scope_label = if row.scope == GLOBAL_SCOPE_KEY {
        "global".to_string()
    } else {
        row.scope.clone()
    };
    let status_text = if row.hits > 0 {
        format!("{} hits", row.hits)
    } else {
        "no hits".to_string()
    };
    let status_color = if row.hits > 0 { "#125a12" } else { "#999" };
    let last_hit = row
        .last_t
        .as_deref()
        .map(time_only)
        .unwrap_or_default();

    view! {
        <div class="firewall-row"
             style="padding: 6px 0; border-bottom: 1px dotted #e8e8e8;">
            <div style="display: flex; align-items: center; gap: 6px;
                        font-size: 10px;">
                <span style=format!(
                    "display:inline-block; padding: 1px 6px; background: {};
                     color: {}; border-radius: 3px; font-weight: 600;
                     font-family: ui-monospace, monospace;",
                     badge_bg, badge_fg
                )>
                    {badge_label}
                </span>
                <span style="font-size: 10px; color: #555;
                             padding: 1px 5px; background: #eef1f6;
                             border-radius: 3px;
                             font-family: ui-monospace, monospace;">
                    {scope_label}
                </span>
                <span style=format!("margin-left: auto; color: {};", status_color)>
                    {status_text}
                </span>
                {if !last_hit.is_empty() {
                    view! {
                        <span style="color: #aaa;
                                     font-family: ui-monospace, monospace;">
                            {last_hit}
                        </span>
                    }.into_any()
                } else { view! { <span /> }.into_any() }}
            </div>
            <div style="font-family: ui-monospace, monospace; font-size: 11px;
                        color: #333; margin-top: 2px; word-break: break-all;">
                {row.match_.clone()}
            </div>
            {if can_expand {
                view! {
                    <div style="margin-top: 3px;">
                        <span class="evidence-toggle"
                              on:click=move |_| open.update(|v| *v = !*v)>
                            {move || if open.get() {
                                "Hide evidence"
                            } else {
                                "Show recent evidence"
                            }}
                        </span>
                        {move || if open.get() {
                            view! {
                                <FirewallEvidence events=events.clone() />
                            }.into_any()
                        } else { view! { <span /> }.into_any() }}
                    </div>
                }.into_any()
            } else { view! { <div /> }.into_any() }}
        </div>
    }
}

/// Expanded per-rule evidence list. Shows each event's timestamp +
/// action-appropriate detail (URL for block, element description
/// for remove, placeholder for hide/spoof/none). Newest first.
#[component]
fn FirewallEvidence(events: Vec<FirewallEvent>) -> impl IntoView {
    let rows: Vec<_> = events
        .into_iter()
        .rev()
        .take(20)
        .map(|ev| {
            let ts = time_only(&ev.t);
            let body = match &ev.evidence {
                FirewallEvidence::Block { url, resource_type } => {
                    let rt = resource_type
                        .as_deref()
                        .map(|t| format!(" [{}]", t))
                        .unwrap_or_default();
                    format!("{}{}", url, rt)
                }
                FirewallEvidence::Remove { el } => el.clone(),
                FirewallEvidence::None {} => String::new(),
            };
            view! {
                <li>
                    <span class="ts">{ts}</span>
                    <span>{body}</span>
                </li>
            }
        })
        .collect();
    view! {
        <ul class="evidence-list">{rows}</ul>
    }
}

/// One aggregated rule row in the firewall log.
#[derive(Clone)]
struct RuleRow {
    rule_id: String,
    action: String,
    scope: String,
    match_: String,
    hits: u32,
    last_t: Option<String>,
}

/// Blocked (network) section. Replaces the `#block-count` / `#block-list`
/// / `#block-evidence` / `#block-diagnostics` renderers that previously
/// lived in `popup.js`. Groups recent blocked URLs by pattern, shows a
/// collapsible per-URL evidence list, and renders the per-rule
/// diagnostic panel with "firing" / "no traffic" / "pattern broken"
/// status badges plus a broken-pattern hint when present.
#[component]
fn BlockedSection(
    blocked_urls: Vec<BlockedUrl>,
    diagnostics: Vec<BlockDiagnostic>,
    block_count: u32,
) -> impl IntoView {
    let evidence_open = RwSignal::new(false);
    let copy_label = RwSignal::new("Copy");

    // Group URLs by pattern for the summary list.
    let mut by_pattern: Vec<(String, u32)> = Vec::new();
    {
        use std::collections::BTreeMap;
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        for b in &blocked_urls {
            let key = if b.pattern.is_empty() {
                "(unknown rule)".to_string()
            } else {
                b.pattern.clone()
            };
            *map.entry(key).or_insert(0) += 1;
        }
        for (k, v) in map {
            by_pattern.push((k, v));
        }
    }

    let has_patterns = !by_pattern.is_empty();
    let has_urls = !blocked_urls.is_empty();
    let has_diag = !diagnostics.is_empty();
    let blocked_len = blocked_urls.len();

    let count_class = if block_count == 0 { "count zero" } else { "count" };

    // Evidence copy payload: reverse order so newest first, join with newlines.
    let copy_payload: String = blocked_urls
        .iter()
        .rev()
        .map(|b| {
            format!(
                "{}\t[{}]\t{}\t(pattern: {})",
                time_only(&b.t),
                b.resource_type.clone().unwrap_or_else(|| "?".into()),
                b.url,
                if b.pattern.is_empty() { "?".into() } else { b.pattern.clone() },
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let on_copy = move |_| {
        let ok = clipboard_write(&copy_payload);
        copy_label.set(if ok { "Copied" } else { "Failed" });
        let label = copy_label;
        if let Some(window) = web_sys::window() {
            let cb = Closure::<dyn Fn()>::new(move || label.set("Copy"));
            let _ = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    1500,
                );
            cb.forget();
        }
    };

    // Newest first for the visible evidence list too.
    let evidence_rows: Vec<_> = blocked_urls
        .iter()
        .rev()
        .map(|b| {
            let ts = time_only(&b.t);
            let rtype_suffix = b
                .resource_type
                .as_ref()
                .map(|t| format!(" [{}]", t))
                .unwrap_or_default();
            let title = b.url.clone();
            let body = format!("{}{}", b.url, rtype_suffix);
            view! {
                <li>
                    <span class="ts">{ts}</span>
                    <span title=title>{body}</span>
                </li>
            }
        })
        .collect();

    view! {
        <details class="section" open=true>
            <summary class="section-head">
                <span class="section-name">"Blocked (network)"</span>
                <span class=count_class>{block_count}</span>
            </summary>
            <ul>
                {if has_patterns {
                    by_pattern.into_iter().map(|(pattern, n)| {
                        let title = pattern.clone();
                        view! {
                            <li>
                                <span class="sel" title=title>{pattern}</span>
                                <span class="n">{n}</span>
                            </li>
                        }
                    }).collect::<Vec<_>>().into_any()
                } else {
                    let msg = if block_count > 0 {
                        "Blocked, but URL evidence not yet captured (try reloading)"
                    } else {
                        "No network blocks yet"
                    };
                    view! {
                        <div class="no-sels">{msg}</div>
                    }.into_any()
                }}
            </ul>
            {if has_urls {
                let toggle_text = move || {
                    let plural = if blocked_len == 1 { "" } else { "s" };
                    let verb = if evidence_open.get() { "Hide" } else { "Show" };
                    format!("{} {} blocked URL{}", verb, blocked_len, plural)
                };
                view! {
                    <div class="evidence">
                        <div style="display:flex;align-items:center;gap:8px;">
                            <span class="evidence-toggle"
                                  on:click=move |_| evidence_open.update(|v| *v = !*v)>
                                {toggle_text}
                            </span>
                            <button on:click=on_copy
                                    title="Copy evidence to clipboard"
                                    style="flex:0 0 auto;padding:2px 10px;font-size:10px;cursor:pointer;border:1px solid #ccc;background:#fff;border-radius:4px;">
                                {move || copy_label.get()}
                            </button>
                        </div>
                        {move || if evidence_open.get() {
                            view! {
                                <ul class="evidence-list">
                                    {evidence_rows.clone()}
                                </ul>
                            }.into_any()
                        } else { view! { <span /> }.into_any() }}
                    </div>
                }.into_any()
            } else { view! { <div /> }.into_any() }}
            {if has_diag {
                let diag_len = diagnostics.len();
                let rows: Vec<_> = diagnostics.into_iter().map(|d| {
                    view! { <RuleRow diag=d /> }
                }).collect();
                view! {
                    <div class="diagnostics">
                        <div class="diagnostics-title">
                            "Block rules (" {diag_len} ")"
                        </div>
                        {rows}
                    </div>
                }.into_any()
            } else { view! { <div /> }.into_any() }}
        </details>
    }
}

/// One row in the Block-rules diagnostic panel. Rendered as a standalone
/// component so the conditional hint (broken-pattern vs. no-traffic) can
/// be expressed as straight view branches without cloning the whole
/// parent context.
#[component]
fn RuleRow(diag: BlockDiagnostic) -> impl IntoView {
    let status_label = match diag.status.as_str() {
        "firing" => "FIRING",
        "no-traffic" => "no traffic",
        "pattern-broken" => "PATTERN BROKEN?",
        other => {
            // Fallback: render whatever the backend sent. Lets future
            // status values land without a Rust change.
            return view! {
                <div class="rule-row">
                    <div class="rule-pattern" title=diag.pattern.clone()>
                        {diag.pattern.clone()}
                    </div>
                    <div class="rule-meta">
                        <span class="rule-fired">
                            "fired " {diag.fired} "x  |  declared under "
                            {if diag.source_domain.is_empty() {
                                "-".to_string()
                            } else { diag.source_domain.clone() }}
                        </span>
                        <span class=format!("rule-status {other}")>
                            {other.to_string()}
                        </span>
                    </div>
                </div>
            }.into_any();
        }
    };
    let status_class = format!("rule-status {}", diag.status);
    let source = if diag.source_domain.is_empty() {
        "-".to_string()
    } else {
        diag.source_domain.clone()
    };

    let hint = if diag.status == "pattern-broken" && !diag.matching_urls.is_empty() {
        let urls: Vec<_> = diag
            .matching_urls
            .iter()
            .map(|u| {
                let title = u.clone();
                view! { <div title=title>{u.clone()}</div> }
            })
            .collect();
        view! {
            <div class="rule-hint">
                <b>"Diagnosis: "</b>
                "this page requested URLs containing "
                <code>{diag.keyword.clone()}</code>
                " but the rule never fired. Your pattern probably doesn't match. "
                "Try a simpler form - e.g., drop wildcards, or use the distinctive "
                "substring anchored with "
                <code>"||domain"</code>
                "."
                <div class="urls">
                    <div style="margin-top:6px;color:#999">
                        "URLs that should have matched:"
                    </div>
                    {urls}
                </div>
            </div>
        }.into_any()
    } else if diag.status == "no-traffic" && diag.fired == 0 {
        view! {
            <div class="rule-hint"
                 style="background:#f0f0f0;color:#666;">
                <b>"No matching traffic yet. "</b>
                "Either the site hasn't requested this URL in the current "
                "session, or a DOM Remove rule is killing the element before "
                "it can fetch. Not necessarily a bug - scroll/reload the page "
                "to generate more traffic if you want to verify."
            </div>
        }.into_any()
    } else {
        view! { <div /> }.into_any()
    };

    view! {
        <div class="rule-row">
            <div class="rule-pattern" title=diag.pattern.clone()>
                {diag.pattern.clone()}
            </div>
            <div class="rule-meta">
                <span class="rule-fired">
                    "fired " {diag.fired} "x  |  declared under " {source}
                </span>
                <span class=status_class>{status_label}</span>
            </div>
            {hint}
        </div>
    }.into_any()
}

/// Removed (DOM) section. Replaces the `#remove-count` / `#remove-list`
/// / `#remove-evidence` renderers. Lists configured Remove-layer
/// selectors with their match counts, plus a collapsible evidence
/// panel of recently-detached elements.
#[component]
fn RemovedSection(
    selectors: IndexMap<String, u32>,
    removed_elements: Vec<RemovedElement>,
) -> impl IntoView {
    let total: u32 = selectors.values().sum();
    let has_selectors = !selectors.is_empty();
    let has_evidence = !removed_elements.is_empty();
    let count_class = if total == 0 { "count zero" } else { "count" };

    let selector_rows: Vec<_> = selectors
        .into_iter()
        .map(|(sel, n)| {
            let title = sel.clone();
            view! {
                <li>
                    <span class="sel" title=title>{sel}</span>
                    <span class="n">{n}</span>
                </li>
            }
        })
        .collect();

    view! {
        <details class="section">
            <summary class="section-head">
                <span class="section-name">"Removed (DOM)"</span>
                <span class=count_class>{total}</span>
            </summary>
            <ul>
                {if has_selectors {
                    selector_rows.into_any()
                } else {
                    view! {
                        <div class="no-sels">"No remove selectors configured"</div>
                    }.into_any()
                }}
            </ul>
            {if has_evidence {
                view! { <RemovedEvidence removed_elements=removed_elements /> }.into_any()
            } else { view! { <div /> }.into_any() }}
        </details>
    }
}

/// Collapsible evidence list for the Removed section. Split into its
/// own component so the toggle/copy state is scoped to one section.
#[component]
fn RemovedEvidence(removed_elements: Vec<RemovedElement>) -> impl IntoView {
    let open = RwSignal::new(false);
    let copy_label = RwSignal::new("Copy");
    let total = removed_elements.len();

    // Newest first, joined to newline-separated TSV.
    let copy_payload: String = removed_elements
        .iter()
        .rev()
        .map(|ev| {
            format!(
                "{}\t{}\t(via {})",
                time_only(&ev.t),
                if ev.el.is_empty() { "?".to_string() } else { ev.el.clone() },
                if ev.selector.is_empty() { "?".to_string() } else { ev.selector.clone() },
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let on_copy = move |_| {
        let ok = clipboard_write(&copy_payload);
        copy_label.set(if ok { "Copied" } else { "Failed" });
        let label = copy_label;
        if let Some(window) = web_sys::window() {
            let cb = Closure::<dyn Fn()>::new(move || label.set("Copy"));
            let _ = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    1500,
                );
            cb.forget();
        }
    };

    let rows: Vec<_> = removed_elements
        .iter()
        .rev()
        .map(|ev| {
            let ts = time_only(&ev.t);
            let title = format!("{} -> {}", ev.selector, ev.el);
            let el = if ev.el.is_empty() { "?".to_string() } else { ev.el.clone() };
            let sel = if ev.selector.is_empty() {
                "?".to_string()
            } else {
                ev.selector.clone()
            };
            let body = format!("{}  (via {})", el, sel);
            view! {
                <li>
                    <span class="ts">{ts}</span>
                    <span title=title>{body}</span>
                </li>
            }
        })
        .collect();

    let toggle_text = move || {
        let plural = if total == 1 { "" } else { "s" };
        let verb = if open.get() { "Hide" } else { "Show" };
        format!("{} {} removed element{}", verb, total, plural)
    };

    view! {
        <div class="evidence">
            <div style="display:flex;align-items:center;gap:8px;">
                <span class="evidence-toggle"
                      on:click=move |_| open.update(|v| *v = !*v)>
                    {toggle_text}
                </span>
                <button on:click=on_copy
                        title="Copy evidence to clipboard"
                        style="flex:0 0 auto;padding:2px 10px;font-size:10px;cursor:pointer;border:1px solid #ccc;background:#fff;border-radius:4px;">
                    {move || copy_label.get()}
                </button>
            </div>
            {move || if open.get() {
                view! {
                    <ul class="evidence-list">{rows.clone()}</ul>
                }.into_any()
            } else { view! { <span /> }.into_any() }}
        </div>
    }
}

/// Hidden (CSS) section. Lists Hide-layer selectors with their match
/// counts. Selectors that also appear in `remove_selectors` render
/// with a "- (removed)" italic marker when their count is zero -
/// the Remove layer detached the node before the CSS rule could
/// match, which is expected overlap, not a bug.
#[component]
fn HiddenSection(
    selectors: IndexMap<String, u32>,
    remove_selectors: IndexMap<String, u32>,
) -> impl IntoView {
    let total: u32 = selectors.values().sum();
    let has_selectors = !selectors.is_empty();
    let count_class = if total == 0 { "count zero" } else { "count" };

    let remove_keys: std::collections::HashSet<String> =
        remove_selectors.keys().cloned().collect();

    let selector_rows: Vec<_> = selectors
        .into_iter()
        .map(|(sel, n)| {
            let title = sel.clone();
            let overlaps = n == 0 && remove_keys.contains(&sel);
            let n_view = if overlaps {
                view! {
                    <span class="n" style="font-style:italic;color:#999;">
                        "- (removed)"
                    </span>
                }.into_any()
            } else {
                view! { <span class="n">{n}</span> }.into_any()
            };
            view! {
                <li>
                    <span class="sel" title=title>{sel}</span>
                    {n_view}
                </li>
            }
        })
        .collect();

    view! {
        <details class="section">
            <summary class="section-head">
                <span class="section-name">"Hidden (CSS)"</span>
                <span class=count_class>{total}</span>
            </summary>
            <ul>
                {if has_selectors {
                    selector_rows.into_any()
                } else {
                    view! {
                        <div class="no-sels">"No hide selectors configured"</div>
                    }.into_any()
                }}
            </ul>
        </details>
    }
}

/// HH:MM:SS from an ISO timestamp. Uses the browser's JS Date so
/// locale + timezone match the rest of the UI. Returns an empty
/// string for unparseable input.
fn time_only(iso: &str) -> String {
    let d = js_sys::Date::new(&JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return String::new();
    }
    let s = d.to_time_string();
    // Format is "HH:MM:SS GMT...". Slice to 8 chars for time only.
    s.as_string()
        .unwrap_or_default()
        .chars()
        .take(8)
        .collect()
}

/// Call-to-action row under the suggestions list. When the behavioral
/// detector is off, shows "Enable" + "Scan this tab now". When it's
/// on, shows "Rescan now". Both paths end by calling
/// [`refresh_popup_suggestions`] so new findings surface without a
/// full popup reload.
#[component]
fn DetectorCta(detector_enabled: bool, tab_id: Option<i32>) -> impl IntoView {
    let enabled = RwSignal::new(detector_enabled);
    let busy = RwSignal::new(false);

    let enable_click = move |_| {
        if busy.get() {
            return;
        }
        let Some(tab_id) = tab_id else {
            return;
        };
        busy.set(true);
        spawn_local(async move {
            if let Err(e) = chrome_bridge::enable_detector().await {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "[Hush popup] enable_detector: {:?}",
                    e
                )));
            }
            let _ = chrome_bridge::scan_once(tab_id).await;
            // Refresh after content-script has had a moment to scan.
            set_timeout(300, move || {
                refresh_popup_suggestions();
                busy.set(false);
                enabled.set(true);
            });
        });
    };

    let scan_click = move |_| {
        if busy.get() {
            return;
        }
        let Some(tab_id) = tab_id else {
            return;
        };
        busy.set(true);
        spawn_local(async move {
            let _ = chrome_bridge::scan_once(tab_id).await;
            set_timeout(300, move || {
                refresh_popup_suggestions();
                busy.set(false);
            });
        });
    };

    view! {
        <div class="rust-detector-cta"
             style="margin-top: 12px; padding: 8px 10px;
                    background: #fffdf5; border: 1px solid #f0e4b0;
                    border-radius: 4px; font-size: 11px;">
            {move || if enabled.get() {
                view! {
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="color: #555; flex: 1;">
                            "Behavioral detector is on."
                        </span>
                        <button
                            disabled=move || busy.get()
                            on:click=scan_click
                            style="padding: 4px 10px; font-size: 11px;
                                   background: #fff; border: 1px solid #ccc;
                                   border-radius: 3px; cursor: pointer;">
                            "Rescan now"
                        </button>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div>
                        <div style="color: #8a6500; margin-bottom: 6px;">
                            "Behavioral suggestions are off (zero CPU cost)."
                        </div>
                        <div style="display: flex; gap: 6px;">
                            <button
                                disabled=move || busy.get()
                                on:click=enable_click
                                style="padding: 4px 10px; font-size: 11px;
                                       background: #2b7cff; color: #fff;
                                       border: 1px solid #2b7cff;
                                       border-radius: 3px; cursor: pointer;">
                                "Enable"
                            </button>
                            <button
                                disabled=move || busy.get()
                                on:click=scan_click
                                style="padding: 4px 10px; font-size: 11px;
                                       background: #fff; border: 1px solid #ccc;
                                       border-radius: 3px; cursor: pointer;">
                                "Scan this tab now"
                            </button>
                        </div>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

/// One-shot setTimeout wrapper. Allocates a Closure that the browser
/// fires once after `ms` milliseconds, then leaks it (it's a one-shot
/// callback so the lifetime is implicit in the timer's own bounded
/// duration). Any `.forget()` alternative ends up the same way.
fn set_timeout(ms: i32, f: impl FnOnce() + 'static) {
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

#[component]
fn MatchedSite(hostname: String, matched_domain: Option<String>) -> impl IntoView {
    let matched = matched_domain.clone();
    let hostname_owned = hostname.clone();
    let show_suffix = match (&matched, hostname.is_empty()) {
        (Some(m), false) => m != &hostname,
        _ => false,
    };

    view! {
        <div class="rust-matched-site"
             style="padding: 6px 10px; font-size: 12px;
                    background: #f5f7fb; border: 1px solid #e0e6ef;
                    border-radius: 4px; margin: 6px 0;">
            {move || match matched.clone() {
                Some(m) => view! {
                    <span>
                        "Matched: "
                        <b style="color: #2d4d8a;">{m.clone()}</b>
                        {if show_suffix {
                            view! { <span style="color:#999;"> " (" {hostname_owned.clone()} ")" </span> }.into_any()
                        } else {
                            view! { <span /> }.into_any()
                        }}
                    </span>
                }.into_any(),
                None if hostname.is_empty() => view! {
                    <span style="color:#999;">"No active tab"</span>
                }.into_any(),
                None => view! {
                    <span style="color:#666;">{hostname.clone()} " (no config matched)"</span>
                }.into_any(),
            }}
        </div>
    }
}

#[component]
fn ActivitySummary(
    block_count: u32,
    remove_count: u32,
    hide_count: u32,
    suggestion_count: u32,
) -> impl IntoView {
    let pill = |label: &'static str, n: u32, color: &'static str| {
        let bg = if n == 0 { "#f0f0f0" } else { color };
        let text = if n == 0 { "#999" } else { "#fff" };
        view! {
            <span style=format!(
                "display:inline-block; padding: 2px 8px; margin-right: 4px;
                 background: {bg}; color: {text}; border-radius: 10px;
                 font-size: 11px; font-weight: 600;"
            )>
                {label} ": " {n}
            </span>
        }
    };
    view! {
        <div class="rust-activity-summary"
             style="padding: 4px 0; margin: 6px 0;">
            {pill("block", block_count, "#d85c4f")}
            {pill("remove", remove_count, "#d89a4f")}
            {pill("hide", hide_count, "#6b8ad4")}
            {pill("suggestions", suggestion_count, "#e8a200")}
        </div>
    }
}

#[component]
fn SuggestionsList(
    suggestions: RwSignal<Vec<Suggestion>>,
    hostname: String,
    tab_id: Option<i32>,
) -> impl IntoView {
    // Refresh helper: re-fetch suggestions after any action.
    let refresh = {
        let suggestions = suggestions;
        move || {
            let Some(tab_id) = tab_id else { return };
            spawn_local(async move {
                match chrome_bridge::get_suggestions(tab_id).await {
                    Ok(next) => suggestions.set(next),
                    Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!(
                        "[Hush popup] get_suggestions failed: {:?}",
                        e
                    ))),
                }
            });
        }
    };

    view! {
        <div class="rust-suggestions"
             style="margin-top: 10px;">
            <h2 style="font-size: 12px; font-weight: 600; color: #555; margin: 0 0 6px;">
                "Suggestions"
            </h2>
            {move || {
                let list = suggestions.get();
                if list.is_empty() {
                    view! {
                        <div style="padding: 8px 10px; background: #fafafa;
                                    color: #999; font-size: 11px; font-style: italic;
                                    border-radius: 3px;">
                            "No suggestions for this tab yet."
                        </div>
                    }.into_any()
                } else {
                    let rows: Vec<_> = list.into_iter().map(|s| {
                        let refresh = refresh.clone();
                        view! {
                            <SuggestionRow
                                suggestion=s
                                hostname=hostname.clone()
                                tab_id=tab_id
                                on_mutated=refresh
                            />
                        }
                    }).collect();
                    view! { <ul style="list-style:none; padding:0; margin:0;">{rows}</ul> }.into_any()
                }
            }}
        </div>
    }
}

#[component]
fn SuggestionRow<F>(
    suggestion: Suggestion,
    hostname: String,
    tab_id: Option<i32>,
    on_mutated: F,
) -> impl IntoView
where
    F: Fn() + Clone + 'static,
{
    let layer_str = match suggestion.layer {
        SuggestionLayer::Block => "block",
        SuggestionLayer::Remove => "remove",
        SuggestionLayer::Hide => "hide",
    };
    let layer_color = match suggestion.layer {
        SuggestionLayer::Block => "#d85c4f",
        SuggestionLayer::Remove => "#d89a4f",
        SuggestionLayer::Hide => "#6b8ad4",
    };

    // Disable the row during an in-flight action so double-click
    // doesn't fire the same message twice.
    let busy = RwSignal::new(false);

    let add_action = {
        let hostname = hostname.clone();
        let layer = layer_str.to_string();
        let value = suggestion.value.clone();
        let on_mutated = on_mutated.clone();
        move |_| {
            if busy.get() {
                return;
            }
            busy.set(true);
            let hostname = hostname.clone();
            let layer = layer.clone();
            let value = value.clone();
            let on_mutated = on_mutated.clone();
            spawn_local(async move {
                if let Err(e) = chrome_bridge::accept_suggestion(&hostname, &layer, &value).await {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "[Hush popup] accept_suggestion: {:?}",
                        e
                    )));
                }
                busy.set(false);
                on_mutated();
            });
        }
    };

    let dismiss_action = {
        let key = suggestion.key.clone();
        let on_mutated = on_mutated.clone();
        move |_| {
            if busy.get() {
                return;
            }
            let Some(tab_id) = tab_id else { return };
            busy.set(true);
            let key = key.clone();
            let on_mutated = on_mutated.clone();
            spawn_local(async move {
                if let Err(e) = chrome_bridge::dismiss_suggestion(tab_id, &key).await {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "[Hush popup] dismiss_suggestion: {:?}",
                        e
                    )));
                }
                busy.set(false);
                on_mutated();
            });
        }
    };

    let allow_action = {
        let key = suggestion.key.clone();
        let on_mutated = on_mutated.clone();
        move |_| {
            if busy.get() {
                return;
            }
            busy.set(true);
            let key = key.clone();
            let on_mutated = on_mutated.clone();
            spawn_local(async move {
                if let Err(e) = chrome_bridge::allowlist_suggestion(&key).await {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "[Hush popup] allowlist_suggestion: {:?}",
                        e
                    )));
                }
                busy.set(false);
                on_mutated();
            });
        }
    };

    let value = suggestion.value.clone();
    let reason = suggestion.reason.clone();
    let learn = suggestion.learn.clone();
    let confidence = suggestion.confidence;
    let count = suggestion.count;
    let from_iframe = suggestion.from_iframe;
    let frame_host = suggestion.frame_hostname.clone();
    let evidence = suggestion.evidence.clone();
    let diag = suggestion.diag.clone();

    // Why? / Evidence panels are independently collapsible per row.
    let why_open = RwSignal::new(false);
    let evidence_open = RwSignal::new(false);

    view! {
        <li class="rust-sugg-row"
            style="padding: 8px 10px; margin-bottom: 8px;
                   background: #fff; border: 1px solid #e0e0e0;
                   border-radius: 4px;">
            <div style="display:flex; align-items:center; gap: 6px;
                        font-size: 11px; margin-bottom: 4px;">
                <span style=format!(
                    "display:inline-block; padding: 1px 8px; background: {layer_color};
                     color: #fff; border-radius: 10px; font-weight: 600;"
                )>
                    {layer_str}
                </span>
                {if from_iframe {
                    if let Some(fh) = frame_host.clone() {
                        view! {
                            <span style="font-size: 10px; color: #888; font-style: italic;"
                                  title="Observation came from an iframe on this tab">
                                "from iframe " {fh}
                            </span>
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }
                } else { view! { <span /> }.into_any() }}
                <span style="margin-left:auto; color: #999;">
                    "conf " {confidence} "  |  count " {count}
                </span>
            </div>
            <div style="font-family: ui-monospace, monospace; font-size: 11px;
                        color: #333; word-break: break-all;">
                {value}
            </div>
            <div style="font-size: 11px; color: #666; margin: 3px 0;">
                {reason}
            </div>
            {if !learn.is_empty() {
                view! {
                    <div style="font-size: 11px; line-height: 1.5; color: #555;
                                background: #fafafa; border-left: 2px solid #c7d5e9;
                                padding: 6px 9px; margin: 6px 0 2px;
                                border-radius: 3px;">
                        {learn}
                    </div>
                }.into_any()
            } else { view! { <div /> }.into_any() }}
            <div style="display:flex; gap: 6px; margin-top: 6px;">
                <button
                    disabled=move || busy.get()
                    on:click=add_action
                    style="padding: 4px 10px; font-size: 11px;
                           background: #2b7cff; color: #fff;
                           border: 1px solid #2b7cff; border-radius: 3px;
                           cursor: pointer;"
                    title="Write this suggestion into the matched site's config">
                    "+ Add"
                </button>
                <button
                    disabled=move || busy.get()
                    on:click=dismiss_action
                    style="padding: 4px 10px; font-size: 11px;
                           background: #fff; color: #999;
                           border: 1px solid #ccc; border-radius: 3px;
                           cursor: pointer;"
                    title="Hide for this tab session only">
                    "Dismiss"
                </button>
                <button
                    disabled=move || busy.get()
                    on:click=allow_action
                    style="padding: 4px 10px; font-size: 11px;
                           background: #fff; color: #2d8a3e;
                           border: 1px solid #b7d7bf; border-radius: 3px;
                           cursor: pointer;"
                    title="Permanently allow on every site. Revocable from Options.">
                    "Allow"
                </button>
                <button
                    on:click=move |_| why_open.update(|v| *v = !*v)
                    style="padding: 4px 10px; font-size: 11px;
                           background: #fff; color: #555;
                           border: 1px solid #ccc; border-radius: 3px;
                           cursor: pointer; flex: 0 0 auto;"
                    title="Why is this suggestion showing even if I have a rule?">
                    {move || if why_open.get() { "Hide why" } else { "Why?" }}
                </button>
                <button
                    on:click=move |_| evidence_open.update(|v| *v = !*v)
                    style="padding: 4px 10px; font-size: 11px;
                           background: #fff; color: #555;
                           border: 1px solid #ccc; border-radius: 3px;
                           cursor: pointer; flex: 0 0 auto;"
                    title="Raw observations that triggered this suggestion">
                    {move || if evidence_open.get() { "Hide evidence" } else { "Evidence" }}
                </button>
            </div>
            {move || if why_open.get() {
                view! { <WhyPanel diag=diag.clone() /> }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
            {move || if evidence_open.get() {
                view! { <EvidencePanel evidence=evidence.clone() /> }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
        </li>
    }
}

/// Dedup diagnostic panel. Explains why a suggestion surfaced even
/// when the user thinks they have a matching rule - drawn from the
/// engine's `SuggestionDiag` attached to every suggestion.
#[component]
fn WhyPanel(diag: SuggestionDiag) -> impl IntoView {
    let layer_str = match diag.layer {
        SuggestionLayer::Block => "block",
        SuggestionLayer::Remove => "remove",
        SuggestionLayer::Hide => "hide",
    };
    let tab_host = if diag.tab_hostname.is_empty() {
        "(unknown)".to_string()
    } else {
        diag.tab_hostname.clone()
    };
    let frame_host = if diag.frame_hostname.is_empty() {
        tab_host.clone()
    } else {
        diag.frame_hostname.clone()
    };
    let is_iframe = if diag.is_from_iframe { "yes" } else { "no" };
    let matched_key = diag
        .matched_key
        .clone()
        .unwrap_or_else(|| "(no site config matched)".into());
    let rule_count = diag.existing_block_count.to_string();
    let sample_rows: Vec<_> = diag
        .existing_block_sample
        .iter()
        .map(|entry| {
            let line = format!("{} (len={})", entry, entry.len());
            view! {
                <li style="padding-left: 12px; font-family: ui-monospace, monospace;">
                    {line}
                </li>
            }
        })
        .collect();
    let has_sample = !diag.existing_block_sample.is_empty();
    let candidate_line = format!("{} (len={})", diag.value, diag.value.len());

    view! {
        <div style="margin-top: 6px; padding: 8px 10px;
                    background: #fafafa; border: 1px solid #eee;
                    border-radius: 3px; font-size: 11px;">
            <ul style="list-style:none; padding:0; margin:0;">
                <li><b>"Checked value: "</b> {diag.value.clone()}</li>
                <li><b>"Tab hostname (used for config match): "</b> {tab_host}</li>
                <li><b>"Observed from frame: "</b> {frame_host}</li>
                <li><b>"From iframe?: "</b> {is_iframe}</li>
                <li><b>"Matched config key: "</b> {matched_key}</li>
                <li><b>"Existing " {layer_str} " rules count: "</b> {rule_count}</li>
                <li><b>"Dedup result: "</b> {diag.dedup_result.clone()}</li>
                {if has_sample {
                    view! {
                        <li style="margin-top: 4px;"><b>"Existing rules sample (first 10):"</b></li>
                        {sample_rows}
                        <li style="margin-top: 4px;"><b>"Candidate value: "</b> {candidate_line}</li>
                    }.into_any()
                } else { view! { <li /> }.into_any() }}
            </ul>
        </div>
    }
}

/// Evidence panel: the raw observations the engine used to justify
/// the suggestion. Shows one line per entry with a Copy button that
/// writes all entries newline-joined to the clipboard.
#[component]
fn EvidencePanel(evidence: Vec<String>) -> impl IntoView {
    let copy_label = RwSignal::new("Copy");
    let joined = evidence.join("\n");
    let on_copy = move |_| {
        let ok = clipboard_write(&joined);
        copy_label.set(if ok { "Copied" } else { "Failed" });
        let label = copy_label;
        // Revert after 1.5s. Leptos has no built-in setTimeout wrapper
        // that's tiny; use gloo-less Window.setTimeout via web_sys.
        if let Some(window) = web_sys::window() {
            let cb = Closure::<dyn Fn()>::new(move || label.set("Copy"));
            let _ = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    1500,
                );
            cb.forget();
        }
    };
    let entries = evidence.clone();
    let empty = entries.is_empty();

    view! {
        <div style="margin-top: 6px; padding: 8px 10px;
                    background: #fafafa; border: 1px solid #eee;
                    border-radius: 3px; font-size: 11px;">
            <div style="display:flex; justify-content:flex-end; margin-bottom: 4px;">
                <button on:click=on_copy
                        disabled=move || empty
                        style="padding: 2px 10px; font-size: 10px;
                               border: 1px solid #ccc; background: #fff;
                               border-radius: 4px; cursor: pointer;"
                        title="Copy all evidence to clipboard">
                    {move || copy_label.get()}
                </button>
            </div>
            <ul style="list-style:none; padding:0; margin:0;">
                {if empty {
                    view! {
                        <li style="font-style: italic; color: #999;">
                            "(no captured evidence)"
                        </li>
                    }.into_any()
                } else {
                    entries.into_iter().map(|e| view! {
                        <li style="font-family: ui-monospace, monospace;
                                   word-break: break-all; margin-bottom: 2px;">
                            {e}
                        </li>
                    }).collect::<Vec<_>>().into_any()
                }}
            </ul>
        </div>
    }
}
