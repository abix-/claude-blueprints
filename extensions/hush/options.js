// Options page bootstrap. The full UI (preference toggles, config
// editor, allowlist editor, JSON editor, export/reset toolbar, and
// status banner) is rendered by Leptos components in
// `src/ui_options.rs`. This file just boots wasm and hands the
// initial storage snapshot to `mountOptions`.

import initWasm, { initEngine, mountOptions } from "./dist/pkg/hush.js";

const wasmReady = initWasm().then(() => {
  try { initEngine(); } catch (e) { console.error("[Hush options] initEngine failed", e); }
}).catch(e => console.error("[Hush options] wasm init failed", e));

async function main() {
  const data = await chrome.storage.local.get(["config", "options", "allowlist"]);
  const config = data.config || {};
  const opts = data.options || {};
  const al = data.allowlist || { iframes: [], overlays: [], suggestions: [] };

  try {
    await wasmReady;
    mountOptions({
      debug: !!opts.debug,
      suggestionsEnabled: !!opts.suggestionsEnabled,
      allowlist: {
        iframes: al.iframes || [],
        overlays: al.overlays || [],
        suggestions: al.suggestions || [],
      },
      config,
    });
  } catch (e) { console.error("[Hush options] mountOptions failed", e); }
}

main();
