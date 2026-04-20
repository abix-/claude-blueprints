// Background service worker bootstrap. All the listener plumbing, DNR
// rule sync, per-tab state, and message handling lives in Rust now
// (src/background.rs). This file is a module SW so top-level static
// `import` works. On cold wake, Chrome re-runs the SW script from the
// top; `init()` re-instantiates WASM each time, and
// `hushBackgroundMain` re-registers every chrome.* listener from Rust.

import initWasm, { initEngine, hushBackgroundMain } from "./dist/pkg/hush.js";

(async () => {
  try {
    await initWasm({ module_or_path: "./dist/pkg/hush_bg.wasm" });
    try { initEngine(); } catch (e) { console.error("[Hush bg] initEngine failed", e); }
    hushBackgroundMain();
  } catch (e) {
    console.error("[Hush bg] bootstrap failed", e);
  }
})();
