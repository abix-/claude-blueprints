---
name: hush
description: Hush -- a firewall-style rule engine for Chrome (MV3) with a Rust/WASM detection core and a Leptos UI. Per-site (or global) rules over seven actions: block/allow (network), neuter/silence (script capture + exfil), remove/hide (DOM), spoof (fingerprint APIs). Lives in `abix-/chromium-extensions` repo. Use when modifying Hush detectors, rules, the rule schema, the Rust engine, the popup, the options page, or the seeded site profiles.
user-invocable: false
version: "1.0"
updated: "2026-05-11"
---
# Hush -- Chrome firewall-style rule engine

Hush is a Chrome MV3 extension shaped like a software firewall: every
request, DOM element, and fingerprint probe a page makes is checked
against user-authored rules; matched rules fire an action; each action
emits an evidence-carrying log entry. The mental model is borrowed
from enterprise network firewalls; the enforcement points live inside
Chrome (DNR, DOM, CSS, prototype hooks).

The detection engine and rule logic are Rust compiled to WASM
(`wasm-pack --target web`). Chrome API glue stays in thin JS shims
(`background.js`, `content.js`, `mainworld.js`). Options and popup
UIs are Leptos. Public repo: `abix-/chromium-extensions`, extension
sub-tree `hush/`.

## Repo + where things live

| Path (repo-relative)                  | What lives there                                      |
| ------------------------------------- | ----------------------------------------------------- |
| `hush/manifest.json`                  | MV3 manifest -- permissions, scripts, CSP, key        |
| `hush/background.js`                  | Service worker shim; bridges chrome.* into the engine |
| `hush/content.js`                     | Content-script shim; injects mainworld + applies DOM passes |
| `hush/mainworld.js`                   | `document_start` page-context hooks (fetch/XHR/beacon/WS, addEventListener, fingerprint APIs) |
| `hush/popup.html` + `popup.js`        | Toolbar popup entry; mounts Leptos popup UI           |
| `hush/options.html` + `options.js`    | Options page entry; mounts Leptos options table       |
| `hush/sites.json`                     | Seeded example rules (generic, not user-specific)     |
| `hush/allowlist.defaults.json`        | Default iframe + sticky-overlay allowlists            |
| `hush/profiles/`                      | Importable profile bundles                            |
| `hush/migrate_config.mjs`             | Schema migration runner (storage v1 -> vN)            |
| `hush/Cargo.toml`                     | Rust crate (cdylib + rlib); wasm-bindgen + leptos     |
| `hush/src/lib.rs`                     | Crate root; wasm-bindgen exports                      |
| `hush/src/types.rs`                   | Rule schema, scopes, actions, RuleEntry, SignalPayload|
| `hush/src/detectors.rs`               | Detector trait + per-tier detector impls              |
| `hush/src/compute.rs`                 | `compute_suggestions` -- core engine entry point      |
| `hush/src/bg_logic.rs`                | Service-worker logic ported off background.js         |
| `hush/src/main_world.rs`              | Page-context hook installer + dispatcher              |
| `hush/src/canon.rs`                   | URL canonicalization, host extraction                 |
| `hush/src/chrome_bridge.rs`           | Typed wrappers over `chrome.*` + storage.local        |
| `hush/src/suggestion.rs`              | Suggestion ranking, dedup, scope picker model         |
| `hush/src/allowlist.rs`               | iframe + sticky allowlist logic                       |
| `hush/src/learn.rs`                   | Auto-tag + accept-flow state                          |
| `hush/src/simulate.rs`                | Rule simulate / test-match logic (options page tool)  |
| `hush/src/stack.rs`                   | Initiator-stack parsing (V8 stack format detection)   |
| `hush/src/lint.rs`                    | Pattern lint (broken/too-narrow detection)            |
| `hush/src/ui_popup.rs`                | Leptos popup components                               |
| `hush/src/ui_options.rs`              | Leptos options components                             |
| `hush/benches/compute_suggestions.rs` | Criterion baseline vs the pre-port JS engine          |
| `hush/test/`                          | Pure-Rust unit tests (cargo test against rlib)        |
| `hush/docs/architecture.md`           | Authoritative threat model + rule taxonomy            |
| `hush/docs/roadmap.md`                | Stage planning + brave-stack gap analysis             |
| `hush/docs/completed.md`              | Shipped detector inventory                            |
| `hush/docs/benchmarks.md`             | Criterion runs (rust vs js baseline)                  |
| `hush/docs/{amazon,reddit,github}.md` | Per-site case studies (rule rationale, observed evidence) |
| `hush/docs/reddit-undisclosed-shilling*.md` | Research + implementation plan for shill detection |
| `hush/docs/comparison.md`             | Hush vs uBlock vs Brave vs Privacy Badger             |
| `hush/CHANGELOG.md`                   | Per-version log                                       |

## The rule model (mandatory shared vocabulary)

A rule is a **(scope, action, match)** triple, with optional `disabled`,
`tags`, `comment`. Storage is keyed by scope under
`chrome.storage.local` (NOT on disk as text).

```jsonc
{
  "__global__": { "block": [{"value": "||doubleclick.net"}] },
  "example.com": {
    "block":   [{"value": "||ads.example.com"}],
    "allow":   [{"value": "||ads.example.com/partner/"}],
    "neuter":  [{"value": "||hotjar.com"}],
    "silence": [{"value": "||replay-vendor.example/api/"}],
    "remove":  [{"value": ".modal-overlay"}],
    "hide":    [{"value": ".popup"}],
    "spoof":   [{"value": "webgl-unmasked"}]
  }
}
```

`__global__` is the reserved scope key. Hostname keys match the host
**and its subdomains** (exact-or-suffix).

### The seven actions

| Action      | Layer      | Enforcement point                                                              |
| ----------- | ---------- | ------------------------------------------------------------------------------ |
| **block**   | network    | `chrome.declarativeNetRequest` rules (URL patterns; uBlock-style syntax)       |
| **allow**   | network/DOM| DNR priority override; for remove/hide it's a selector exclusion               |
| **neuter**  | script     | mainworld wraps `addEventListener`; denies interaction-event regs from matching script origins |
| **silence** | script     | mainworld intercepts `fetch` / `XHR.send` / `sendBeacon` from matching origins; fake-succeed (204 / state 4 / true) |
| **remove**  | DOM        | `element.remove()` + `MutationObserver` per-frame                              |
| **hide**    | CSS        | `display: none !important` via user stylesheet at `document_start`             |
| **spoof**   | fingerprint| mainworld intercepts specific APIs; returns bland identical-across-users values|

**Block rules apply as global URL patterns** at the network layer
(no `initiatorDomains` -- broken for cross-origin iframes). A rule
keyed by `reddit.com` blocks its target URL wherever requested.
Per-site BLOCKING is done by making the pattern itself more
specific, not via DNR `initiatorDomains`.

Evaluation is **first-match-wins within each action**, top-down in
authoring order. Cross-action ordering is meaningless (Block gates
network, Remove/Hide touch DOM, Spoof touches fingerprint APIs).
Block↔Allow override runs through DNR priority, not table position.

### Spoof kinds (current)

| Kind             | What it returns                                                                 |
| ---------------- | ------------------------------------------------------------------------------- |
| `webgl-unmasked` | `UNMASKED_VENDOR_WEBGL` / `UNMASKED_RENDERER_WEBGL` → `"Google Inc."` / `"ANGLE (Generic)"` |
| `canvas`         | `toDataURL` / `toBlob` → constant 1×1 PNG; `getImageData` → zero-init ImageData |
| `audio`          | `OfflineAudioContext.startRendering` → silent AudioBuffer                       |
| `font-enum`      | `measureText` → synthetic metrics (width depends only on text length)           |

## Threat model (one paragraph)

Every site runs code not on the user's side: ad/analytics/session-
replay vendors exfil mouse/scroll/keys; first-party telemetry
subdomains (`collector.*`, `unagi.*`, `w3-reporting.*`) fire
sendBeacon throughout a session and are NOT on public blocklists;
fingerprinters read GPU / canvas / fonts / WebGL params (3-4 of
those uniquely identify 90%+ of sessions regardless of cookies,
incognito, VPN); heavy UI elements burn CPU/network in the
background. Public blocklists cover cross-site; Hush is the tool
for the **per-site first-party** gap.

## Architecture: where work happens

- **Service worker (Rust/WASM, glued by `background.js`)** -- owns
  DNR ruleset registration, the firewall event log (rule_id +
  per-tab ring buffer + persistent searchable log), rule-pattern
  rehydration on SW wake, suggestion compute, options + popup
  messaging. Engine entry point: `hush::compute::compute_suggestions`.
- **Content script (Rust/WASM via `content.js`)** -- applies
  Remove/Hide DOM passes per-frame; runs `MutationObserver`;
  emits removal evidence (tag + class signature + distinguishing
  attrs + text preview).
- **Main world (`mainworld.js` + Rust hooks)** -- installed at
  `document_start` via `web_accessible_resources`. Wraps fetch/
  XHR/sendBeacon/WebSocket + `addEventListener` + fingerprint
  APIs. Communicates with the content script via
  `CustomEvent` (typed `SignalPayload`). CSP fallback for strict
  sites: wasm in content scripts is blocked by some CSPs, so
  Hush ships the page-context hooks as plain JS that calls into
  wasm only from the content script.

## The detector pipeline + suggestions

`detectors.rs` defines a `Detector` trait + per-tier impls. Each
detector inspects DOM/CSS/network/main-world signals and proposes
**suggestions** -- not auto-applied rules. The user sees them as a
yellow `!` badge in the popup; each suggestion has Why? / Evidence
expandable panels and one-click Add / Dismiss / Allow.

Tiers (see `docs/completed.md` for the full inventory):
1. Network -- block/allow seed defaults
2. DOM -- remove/hide candidates (overlays, sticky promos, hidden iframes)
3. Script capture / exfil -- neuter / silence candidates from initiator-stack analysis
4. Fingerprinting -- canvas / webgl / audio / font-enum / installed-font probes
5. Behavioral -- invisible animation loops, rAF burn, hidden iframes
6. Brave-stack baseline gaps (clipboard-read, attention-tracking, hardware device-api, navigator-fp)
7. Session-replay vendor heuristics
8. Per-site research (reddit shill detection in flight)

`compute_suggestions` is the single entry; it dedups against
existing rules (`DetectCtx::is_covered`), per scope including
`__global__`, and ranks by tier + signal strength. Benchmarked
against the pre-port JS engine with Criterion
(`benches/compute_suggestions.rs`).

## Schema migration

Storage schema is versioned. `migrate_config.mjs` is the chain
runner; `migrateConfigSchema` in `hush/src/...` (port target)
runs each step. The chain has an anchor at v3 (rule entries are
`{value, disabled, tags, comment}` instead of bare strings; bare
strings still parse for back-compat).

Add a migration step when changing schema: bump the version,
append a step that converts vN → vN+1, never rewrite past steps
(forward-only chain).

## Runtime self-tests

The engine runs a small self-test at SW startup:
- V8 `Error().stack` format detection (different across Chromium
  versions; the parser branches on the detected format).
- Spoof tag exact-match enforcement (each spoof writes a tag that
  page-side detectors check; tag must match exactly, not by
  prefix, so a spoof of one kind can't mask another).

Self-tests log failures via `console.error`; they're cheap (one
synthetic invocation each at SW init).

## Build + load

```bash
# from hush/
wasm-pack build --target web --release
# then in chrome://extensions/, Developer Mode on, Load Unpacked, pick hush/
```

`wasm-pack` profile is pinned in `Cargo.toml`:
- `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`,
  `panic = "abort"`, symbols stripped.
- `wasm-opt = false` (bundled wasm-opt in `wasm-pack` 0.14 doesn't
  validate rustc 1.95's `i64.trunc_sat_f64_s`).
- ~1.0-1.5 MB bundle; runtime perf chosen over bundle size --
  bundle loads once per tab session, hot paths run thousands of
  times.

## Test surface

- `cargo test` -- pure-Rust unit tests via rlib (no wasm needed).
- `cargo bench` -- Criterion runs vs the pre-port JS baseline.
- `hush/test/` -- JS-side integration tests; expose internal
  helpers via `__hush_mainworld__` for test-only access (e.g.
  `matchesHostPattern`).
- `clippy -D warnings` is enforced in CI; do not regress.
- eslint 10 flat-config for the JS shims.

## Sibling extensions (same repo)

`chromium-extensions/` also houses:
- **zoom-extension** -- YouTube fullscreen + scroll-to-preview
  blocker. CSS-injected at `document_start`. Notes in its own
  `README.md` + `CHANGELOG.md`.
- **filter-anything-everywhere** -- MIT fork; less-actively
  maintained, see its README.

No skill for either today; if work concentrates there, file a
follow-up to split.

## Session etiquette

- Public repo. Generic code only -- no personal seeds in
  `sites.json` (the seeded examples must be generic and
  reproducible). User-specific rules belong in the user's own
  `chrome.storage.local`, not in the repo.
- Read `hush/docs/architecture.md` before changing the rule
  model. It is the contract.
- New detectors land with: one impl in `detectors.rs`, one
  `compute_suggestions` wire-up, one entry in `completed.md`,
  one bench comparison against baseline.
- `hush/docs/roadmap.md` carries the priority list. Brave-stack
  gap closures rank above novel detectors.
- ASCII source/docs/commits. Commits lowercase, push
  immediately.
