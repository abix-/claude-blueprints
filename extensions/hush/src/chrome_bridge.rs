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
