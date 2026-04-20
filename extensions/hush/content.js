// Hush content-script bootstrap. All of the former JS runtime
// (layer application, MutationObserver, PerformanceObserver, scan
// functions, __hush_call__ bridge) now lives in Rust - see
// `src/content.rs`. This file exists only because MV3 content
// scripts can't static-import wasm glue; it dynamically imports
// the bundle via chrome.runtime.getURL, reads the three
// chrome.storage.local keys the Rust side needs, and hands them
// to `hushContentMain`.

(async function () {
  try {
    const [{ default: initWasm, initEngine, hushContentMain }, data] =
      await Promise.all([
        import(chrome.runtime.getURL("dist/pkg/hush.js")),
        chrome.storage.local.get(["config", "options", "allowlist"]),
      ]);
    await initWasm(chrome.runtime.getURL("dist/pkg/hush_bg.wasm"));
    try { initEngine(); } catch (e) { /* panic hook already installed if enabled */ }
    const al = data.allowlist || {};
    hushContentMain({
      config: data.config || {},
      options: data.options || {},
      allowlist: {
        iframes: Array.isArray(al.iframes) ? al.iframes : [],
        overlays: Array.isArray(al.overlays) ? al.overlays : [],
        suggestions: Array.isArray(al.suggestions) ? al.suggestions : [],
      },
    });
  } catch (e) {
    console.error("[Hush content] bootstrap failed", e);
  }
})();
