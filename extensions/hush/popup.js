// Popup bootstrap. The entire popup UI - matched-site header,
// activity summary, suggestions list, detector CTA, three diagnostic
// sections, unmatched banner, and footer buttons - renders via
// Leptos components inside the WASM bundle. See `src/ui_popup.rs`.
//
// `hushPopupMain` queries the active tab, fetches stats /
// suggestions / rule diagnostics / persisted config, then mounts the
// Leptos tree with the assembled snapshot.

import initWasm, { initEngine, hushPopupMain } from "./dist/pkg/hush.js";

(async () => {
  try {
    await initWasm();
    try { initEngine(); } catch (e) { console.error("[Hush popup] initEngine failed", e); }
    await hushPopupMain();
  } catch (e) {
    console.error("[Hush popup] bootstrap failed", e);
  }
})();
