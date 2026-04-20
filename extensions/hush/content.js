// Hush content-script bootstrap. All the former JS runtime
// (layer application, MutationObserver, PerformanceObserver, scan
// functions, __hush_call__ bridge) lives in Rust - see
// `src/content.rs`. This file exists only because MV3 content
// scripts can't static-import wasm glue.
//
// Two important details:
//   1. We pre-fetch the WASM bytes via chrome.runtime.getURL +
//      fetch + arrayBuffer, then pass them to initWasm as a
//      BufferSource. This avoids WebAssembly.instantiateStreaming
//      which the page's CSP can block (pages like reddit.com ship
//      script-src rules that reject streaming instantiation).
//   2. wasm-bindgen >= 0.2.100 deprecates passing a URL directly;
//      the new signature is initWasm({ module_or_path }).

(async function () {
  try {
    const [{ default: initWasm, initEngine, hushContentMain }, data, wasmBytes] =
      await Promise.all([
        import(chrome.runtime.getURL("dist/pkg/hush.js")),
        chrome.storage.local.get(["config", "options", "allowlist"]),
        fetch(chrome.runtime.getURL("dist/pkg/hush_bg.wasm")).then(r => r.arrayBuffer()),
      ]);
    await initWasm({ module_or_path: wasmBytes });
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
