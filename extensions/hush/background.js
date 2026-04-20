// Background service worker bootstrap + debug-log relay.

import initWasm, { initEngine, hushBackgroundMain } from "./dist/pkg/hush.js";

const LOG_SINK = "http://127.0.0.1:8765/log";
const postLog = (text) => {
  try {
    fetch(LOG_SINK, { method: "POST", body: text, keepalive: true }).catch(() => {});
  } catch {}
};

// Relay content-script debug messages to the localhost log sink.
// Content scripts can't fetch localhost on most sites (page CSP
// blocks it), but the service worker can because its fetch isn't
// subject to page CSP.
chrome.runtime.onMessage.addListener((msg, sender) => {
  if (!msg || typeof msg !== "object" || typeof msg.type !== "string") return;
  const host = sender && sender.tab && sender.tab.url
    ? (new URL(sender.tab.url).hostname)
    : (sender && sender.url ? (new URL(sender.url).hostname) : "?");
  let summary;
  if (msg.type === "hush:debug-log" && typeof msg.text === "string") {
    summary = msg.text;
  } else if (msg.type === "hush:log") {
    summary = `log ${host}: ${(msg.args || []).join(" ")}`;
  } else if (msg.type === "hush:stats") {
    const h = Object.values(msg.hide || {}).reduce((a,b)=>a+(+b||0),0);
    const r = Object.values(msg.remove || {}).reduce((a,b)=>a+(+b||0),0);
    summary = `stats ${host}: matched=${msg.matchedDomain} hide=${h} remove=${r} newRemoved=${(msg.newRemovedElements||[]).length}`;
  } else if (msg.type === "hush:scan") {
    summary = `scan ${host}: reason=${msg.reason} resources=${(msg.resources||[]).length} iframes=${(msg.iframes||[]).length} stickies=${(msg.stickies||[]).length}`;
  } else if (msg.type === "hush:js-calls") {
    summary = `js-calls ${host}: ${(msg.calls||[]).length} calls`;
  } else {
    summary = `${msg.type} ${host}`;
  }
  postLog(summary);
});

(async () => {
  postLog("bg: bootstrap start");
  try {
    await initWasm({ module_or_path: "./dist/pkg/hush_bg.wasm" });
    postLog("bg: wasm init ok");
    try { initEngine(); } catch (e) { postLog(`bg: initEngine threw ${e}`); }
    hushBackgroundMain();
    postLog("bg: hushBackgroundMain ok");
  } catch (e) {
    postLog(`bg: bootstrap failed ${e && e.message ? e.message : e}`);
    console.error("[Hush bg] bootstrap failed", e);
  }
})();
