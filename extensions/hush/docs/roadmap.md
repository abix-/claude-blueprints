# Roadmap

Target: single-crate Rust extension with the detection engine, main-world
hooks, and popup/options UI all compiled to WASM. JS reduced to
chrome-API glue and bootstrap shims.

## How to maintain this roadmap

1. **Stages are the priority.** Read top-down. First unchecked stage is
   the current sprint.
2. **No duplication.** Each work item lives in exactly one place. Stages
   hold future work. [completed.md](completed.md) is the user-facing
   feature snapshot. [history.md](history.md) keeps retired rollout
   notes. [heuristic-roadmap.md](heuristic-roadmap.md) has detection-tier
   research and is the source material for future stages.
3. **Completed checkboxes are accomplishments.** Never delete them. When
   a stage is done, move rollout narrative to
   [history.md](history.md) and current behavior into the matching
   feature doc.
4. **"Done when" sentences don't change** unless the product changes.
5. **New features** go in the appropriate future stage.
6. **Describe current state, not history.** Present-tense behavior lives
   in feature docs; chronology lives in [history.md](history.md) or
   [CHANGELOG.md](../CHANGELOG.md).

## Completed

See [completed.md](completed.md) for the user-facing feature snapshot
and [history.md](history.md) for retired rollout notes.

## Stages

Stages 1 and 2: [x] Complete (see [history.md](history.md))

**Current Sprint (priority order):**

1. Stage 3 main-world hooks in Rust
2. Then Stage 4 popup UI in Leptos
3. Then Stage 5 options UI + content script cleanup

### Stage 3: Main-world hooks in Rust

*Done when: every `mainworld.js` prototype hook is installed from a
Rust closure via `wasm_bindgen::Closure` + `js_sys::Reflect::set`, with
`mainworld.js` reduced to a ~20-line bootstrap that loads WASM and
calls `install()`. `test/emit_contract.test.mjs` passes against the
Rust-installed hooks without modification.*

- [ ] Add `src/main_world.rs` with an exported `install()` function
- [ ] Port fetch / XHR / sendBeacon / WebSocket.send hooks
- [ ] Port canvas fingerprinting hooks (toDataURL / toBlob /
      getImageData / measureText)
- [ ] Port WebGL / WebGL2 getParameter hook
- [ ] Port OfflineAudioContext constructor hook
- [ ] Port `addEventListener` density hook
- [ ] Port replay-global poll
- [ ] Port canvas draw-op visibility sampler (throttled per canvas)
- [ ] Reduce `mainworld.js` to a bootstrap shim that loads
      `dist/pkg/hush.js` and calls `install()`
- [ ] Add `web_accessible_resources` entries for `dist/pkg/*` so the
      MAIN world can fetch the WASM bundle
- [ ] Update `test/emit_contract.test.mjs` if the hook-installation
      path needs new stubs; existing assertions stay

### Stage 4: Popup UI in Rust (Leptos)

*Done when: the popup renders via Leptos components compiled to WASM.
`popup.js` becomes a ~15-line bootstrap. Activity section, block
diagnostics, suggestions list, and the Add / Dismiss / Allow actions
all work identically to the current JS popup. Performance budget: cold
popup open under 100 ms.*

- [ ] Pick framework: Leptos (recommended for bundle size) or Yew
- [ ] Add Leptos dep + compile target
- [ ] Port popup component tree: match header, activity section, block
      diagnostics, suggestions list, action row
- [ ] Wire messaging from popup to service worker (same
      `chrome.runtime.sendMessage` envelopes, typed via shared types)
- [ ] Port `.sugg-learn` + `.sugg-actions` styling (can stay CSS)
- [ ] Replace `popup.js` with bootstrap that loads WASM and mounts
      the component tree
- [ ] Verify cold-open render time in DevTools Performance

### Stage 5: Options UI + content script cleanup

*Done when: the options page is a Leptos component, the content
script's DOM scans (hidden iframes, sticky overlays) run through
`web_sys` bindings with the same thresholds, and the remaining JS
totals under 100 lines across all bootstrap shims.*

- [ ] Port options page to Leptos (site list, per-site editor, three
      allowlist sections, raw JSON editor, debug/suggestions toggles)
- [ ] Port `content.js` DOM scans via `web_sys::Document` and
      `web_sys::Element` + `getComputedStyle`
- [ ] Port `MutationObserver` installation via
      `web_sys::MutationObserver`
- [ ] Port `PerformanceObserver` + resource stream via
      `web_sys::PerformanceObserver`
- [ ] Delete the JS copies after each port lands
- [ ] Final pass: confirm total JS LOC under 100 across all bootstrap
      shims (`background.js`, `content.js`, `mainworld.js`, `popup.js`,
      `options.js`)

## Out of scope (for now)

Explicitly not in the current stage list. Pulled back in via new stages
when any of them become the blocking work:

- Filter-list engine (EasyList / EasyPrivacy matching). uBlock Origin
  Lite already does this; Hush's mandate is per-site surgical cleanup
  plus behavioral detection of what lists miss. See
  [heuristic-roadmap.md](heuristic-roadmap.md) "Out of scope" section.
- Cross-site correlation (Privacy-Badger-style "3+ sites" threshold).
  Needs persistent stateful detection; big architectural load.
- Additional detection tiers beyond what's already shipped: Tier 3
  navigator/screen property reads, Tier 4 supercookies, Tier 6 service
  worker registrations. See [heuristic-roadmap.md](heuristic-roadmap.md).
- Shared-core builds: Tauri desktop app, native CLI HAR analyzer,
  mobile via `uniffi`. Covered in the max-Rust thesis but not in the
  near-term stage list.

## Related

- [heuristic-roadmap.md](heuristic-roadmap.md) - detection-tier research
  and gap analysis that drives future stages.
- [completed.md](completed.md) - user-facing feature snapshot.
- [history.md](history.md) - retired rollout notes.
- [CHANGELOG.md](../CHANGELOG.md) - dated release notes.
