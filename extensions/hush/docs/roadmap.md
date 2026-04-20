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

Stages 1, 2, and 3: [x] Complete (see [history.md](history.md))

**Current Sprint (priority order):**

1. Stage 4 popup UI in Leptos
2. Then Stage 5 options UI + content script cleanup

### Stage 3: Main-world hooks in Rust [x] Complete

Shipped in 0.10.0. Hybrid bootstrap: synchronous JS stubs capture hooks
at document_start into `window.__hush_stub_q__`; WASM loads
asynchronously; once ready, the queue is drained through
`drainStubQueue` and subsequent hook invocations go through
`dispatchHook` directly. Every payload is validated by serde against
the typed `SignalPayload` discriminated union in Rust - missing
required fields (the 0.5.0 bug class) fail loudly instead of silently
dropping.

The "Rust installs prototype hooks" shape from the original plan
didn't survive contact with wasm-bindgen's `this`-binding limitation.
JS owns the physically-required prototype assignment; Rust owns every
step after the capture. See [history.md](history.md) for the rollout
notes and design rationale.

### Stage 4: Popup UI in Rust (Leptos)

*Done when: the popup renders via Leptos components compiled to WASM.
`popup.js` becomes a ~15-line bootstrap. Activity section, block
diagnostics, suggestions list, and the Add / Dismiss / Allow actions
all work identically to the current JS popup. Performance budget: cold
popup open under 100 ms.*

- [x] Pick framework: Leptos 0.8 (smallest bundle of the Rust WASM
      frameworks; fine-grained signals match the popup's mutation
      patterns)
- [x] Add Leptos dep + wasm-bindgen-futures + chrome_bridge module
- [x] Port matched-site header (`MatchedSite` component)
- [x] Port activity summary pills (`ActivitySummary` component)
- [x] Port suggestions list with Add / Dismiss / Allow actions
      (`SuggestionsList` + `SuggestionRow` + async
      `chrome_bridge::accept_suggestion` / `dismiss_suggestion` /
      `allowlist_suggestion`)
- [x] Port Why? / Evidence expandable panels (`WhyPanel` +
      `EvidencePanel` + clipboard copy via `navigator.clipboard.writeText`)
- [x] Port detector CTA (`DetectorCta`: Enable / Scan-once / Rescan)
      with `chrome_bridge::enable_detector` + `scan_once`
- [x] Expose `refreshPopupSuggestions` wasm-bindgen export so button
      actions can refresh the Leptos signal without remounting
- [x] Port the blocked-URL list and block-rule diagnostics panel
      (`BlockedSection` component, new `BlockedUrl` + `BlockDiagnostic`
      types)
- [x] Port the Remove + Hide selector lists and the removed-element
      evidence panel (`RemovedSection` + `RemovedEvidence` +
      `HiddenSection` components, new `RemovedElement` type,
      `IndexMap<String, u32>` selector maps). `#sections` div deleted
      from `popup.html`.
- [ ] Verify cold-open render time in DevTools Performance against the
      100ms budget

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
