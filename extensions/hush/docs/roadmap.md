# Roadmap

Target: a firewall-style rule engine for the browser. Rules are
(scope, action, match) triples with five actions — block, allow,
remove, hide, spoof — evaluated first-match-wins per action. Every
rule hit emits into a persistent searchable log. A behavioral
detector proposes new rules from observed page behavior so the
user's rule set grows against the sites it's actually fighting.
Engine in Rust/WASM; JS is chrome-API glue and bootstrap shims.

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

Stages 1-7, 9: [x] Complete. See [history.md](history.md) for
rollout notes and [completed.md](completed.md) for the current
feature snapshot.

Stage 8: More spoof kinds (`canvas`, `audio`, `font-enum`).
Stage 10: Rule import/export profiles.
Stage 11: **Auto-tags shipped** — accepted suggestions inherit
the originating detector's signal kind as an `auto:<kind>` tag
on the resulting rule. Popup firewall log gains a tag-chip
filter row built from the distinct tags across every rule.
Stage 12: **Rule lint (phase A shipped)** — shadow detection for
block rules and zero-match detection for remove/hide selectors,
both surfaced as inline badges in the popup firewall log + a
rule-health roll-up in the log header. Pure-Rust `src/lint.rs`
module with 9 unit tests for the shadow heuristic. Phase B
(options-editor per-rule badges, broken-selector detection,
dead-vs-no-hits distinction) is deferred.
Stage 13: **Rule simulate** — test-match UI. Given a URL or a
DOM snippet, show which rule would fire (block / allow / remove
/ hide / spoof / neuter / silence) and why.
Stage 14: **Neuter + silence shipped** — two new main-world
actions for session-replay neutralization. Neuter denies the
capture surface (addEventListener) at document_start; silence
intercepts exfil (fetch/XHR/beacon) with fake-success. Replay-
listener detector upgraded to suggest neuter instead of URL
block.

## Next up (priority order)

1. **Stage 13** — rule simulate / test-match UI. Useful once
   rule sets grow past a dozen entries across global + per-site
   scopes.
2. **Stage 12 phase B** — options-editor rule-health badges,
   broken-selector detection, and a dead-rule category
   computed by walking the persistent firewall log.
3. **Stage 8** — canvas / audio / font-enum spoofs. Fingerprint
   coverage expansion, orthogonal to the audit stages.
4. **Stage 10** — rule import/export profiles. UX polish on top
   of everything above.

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

- [x] Port options page to Leptos. All UI (preference toggles, site
      list + per-site editor, three allowlist sections, raw JSON
      editor, Export/Reset toolbar) renders via Leptos components in
      `src/ui_options.rs`. `options.js` is a 34-line bootstrap that
      reads three storage keys and calls `mountOptions(snapshot)`.
    - [x] Scaffold: `src/ui_options.rs` + `mountOptions` +
          `<div id="rust-options-root">` + `options.js` module
          conversion; preference toggles (`SettingsToggles`) and
          shared status banner (`StatusBanner`) ported. Exported
          `setOptionsStatus` so the remaining JS handlers surface
          feedback through the same banner.
    - [x] Port Export JSON + Reset to defaults buttons
          (`ConfigToolbar` component; `chrome_bridge::get_config_json`
          + `reset_config_to_defaults` helpers). Reset reloads the
          page so the JS-owned site list and JSON editor re-read
          `chrome.storage.local`.
    - [x] Port JSON editor (`JsonEditor` component;
          `chrome_bridge::set_config_from_json` +
          `chrome_bridge::get_config_json`). Apply reloads the page
          so the JS-owned site list re-reads `chrome.storage.local`.
    - [x] Port allowlist textareas (`AllowlistEditor` component;
          `chrome_bridge::set_allowlist` +
          `chrome_bridge::get_default_allowlist`). Mounts at a
          second root `#rust-allowlist-root` inside the existing
          `<details>` wrapper.
    - [x] Port site list + per-site editor (`ConfigEditor` +
          `SiteList` + `SiteListRow` + `SiteDetail` + `SiteDetailBody`
          + `LayerSection` components). The full `Config` lives in a
          single `RwSignal<IndexMap<String, SiteConfig>>` that the
          editor mutates in place and persists via
          `chrome_bridge::set_config` on every change.
- [x] Port `content.js` DOM scans via `web_sys::Document` and
      `web_sys::Element` + `getComputedStyle` (`src/content.rs`
      `scan_hidden_iframes` + `scan_sticky_overlays` +
      `describe_element`).
- [x] Port `MutationObserver` installation via
      `web_sys::MutationObserver` (`src/content.rs`
      `install_mutation_observer`).
- [x] Port `PerformanceObserver` + resource stream via
      `web_sys::PerformanceObserver` (`src/content.rs`
      `install_resource_observer` +
      `convert_resource_entry`).
- [x] Delete the JS copies: `content.js` collapsed from 464 LOC to
      a 32-line wasm bootstrap. `options.js` is 34 LOC, `popup.js`
      is 148 LOC.
- [x] Final pass: total JS LOC under 100 across popup + options +
      content bootstrap shims. popup.js 20 + options.js 34 +
      content.js 32 = 86. `mainworld.js` (419) stays larger because
      of the physically-required synchronous document_start hook
      stubs; `background.js` (988) is the service worker, out of
      Stage 5 scope.

### Stage 7: Firewall-style rule engine surface

*Done when: rules have explicit scope (global OR site-specific),
every rule hit emits a uniform firewall-log event (`rule_id`,
`action`, `scope`, `match`, `observed_at`, `evidence`), and the
popup / options UI exposes per-rule hit counts and a sortable event
history. The firewall-log mental model is the
user-visible interface shape; see [architecture.md](architecture.md)
for the background and rationale.*

- [x] Add a `global` scope to the config schema. Shipped as the
      reserved `__global__` hostname key in the existing
      `IndexMap<String, SiteConfig>` (no schema migration needed;
      underscore-prefixed domains can't collide with real
      hostnames). Content script merges global + site-scoped rules
      at evaluation time; dedup in `compute_suggestions` checks
      both scopes (`src/types.rs::merged_site_config`).
- [x] Stable `rule_id` for every rule: deterministic
      `"{action}::{scope}::{match}"` format
      (`src/types.rs::rule_id`). Matches the suggestion-key
      format so accepted suggestions and their resulting
      firewall-log rows cross-reference cleanly. No persisted
      ID migration because the format is purely derived.
- [x] Unified firewall-log event shape
      (`src/types.rs::FirewallEvent` with action-tagged
      `FirewallEvidence` variants) + per-tab ring buffer (cap
      500 events/tab) in `BackgroundState.tab_events`. Block hits
      (DNR `onRuleMatchedDebug`) and Remove hits (`hush:stats`
      `newRemovedElements`) both emit into the buffer; Remove
      events carry their originating scope so the log row
      matches the rule row.
- [x] Popup Firewall Log section (`FirewallLog` + `FirewallLogRow`
      + `FirewallEvidence` components in `src/ui_popup.rs`).
      Aggregates events by `rule_id`; one row per rule with
      action badge, scope tag, match, hit count, last-hit time;
      click-to-expand recent evidence (last 20 events per rule).
- [x] Spoof/hide event emission. Hide: content.js emits one
      event per selector the first time its match count goes
      >0 on a page (piggybacks on `hush:stats`). Spoof:
      mainworld dispatches `__hush_spoof_hit__` on the first
      bland-value return per kind per page; content.js relays
      as `hush:spoof-hit`; background emits a FirewallEvent
      with action="spoof". Every action now flows through the
      unified log.
- [x] Per-rule disable toggle in the options editor. Enable
      checkbox on every rule row; disabled rows render
      strikethrough. Evaluator skip: DNR sync excludes them,
      `toValueList` in content.js skips them, compute_suggestions
      ignores them for dedup (so detector keeps surfacing
      matches while a rule is parked). Regression test locks
      the dedup behaviour.

### Stage 8: More spoof kinds

*Done when: Canvas, Audio, and Font-enumeration fingerprint signals
have spoof implementations alongside the existing `webgl-unmasked`
kind. Each follows the same content-script → dataset marker →
main-world-hook pattern the WebGL case established.*

- [ ] `canvas` spoof: return a fixed hash from
      `HTMLCanvasElement.toDataURL` / `toBlob` /
      `CanvasRenderingContext2D.getImageData` when the site opts in.
      Trade-off: some sites use canvas for legitimate rendering
      (image resize, drawing apps); spoof must be opt-in per site.
- [ ] `audio` spoof: stub `OfflineAudioContext` so fingerprinting
      constructors get back a predictable-output context. Same
      opt-in caveat as canvas.
- [ ] `font-enum` spoof: cap `measureText` to returning metrics
      from a fixed allowlist (core system fonts only), neutralizing
      installed-font probing.

### Stage 9: Firewall primitives

*Done when: rules are stored as an ordered list of `RuleEntry`
objects with `disabled` / `tags` / `comment` metadata; an `allow`
action exists and overrides `block` via DNR priority and excludes
selectors from Remove/Hide; first-match-wins ordering is enforced
per action; the firewall log is persistent in
`chrome.storage.session` with search + action + tag + tab filters
in the popup. End-to-end test: a global `block ||doubleclick.net`
rule + a site-scoped `allow doubleclick.net/adx/` rule results in
adx requests succeeding on that site only, with an allow event
in the log referencing the overridden block rule's ID.*

- [x] Introduce `RuleEntry { value, disabled, tags, comment }`.
      Hard migration (no backward-compat shim in Rust): background.js
      converts legacy bare-string entries at SW bootstrap. Converted
      `SiteConfig.{block, remove, hide, spoof}` to `Vec<RuleEntry>`
      and added `allow: Vec<RuleEntry>`. Regression tests lock the
      on-disk JSON shape.
- [x] Evaluator: `background.rs` DNR sync emits `allow` rules at
      `priority: 2` and `block` rules at `priority: 1` so DNR's
      own first-match resolution picks allow. Content-script
      applier skips nodes matched by an `allow` selector in
      `applyRemove()`; hide CSS appends `:not(<allow>)` per hide
      rule so allowed nodes render. Options editor gains the
      Allow section; FirewallLog enumerates allow rules.
- [x] ~~`merged_site_config` preserves order instead of deduping
      by value.~~ Resolved differently: per-action dedup-by-value
      kept because the five actions are orthogonal (block gates
      network, remove/hide touch DOM, spoof touches fingerprint
      APIs — none race). The one cross-action case (allow
      overriding block) is handled by DNR priority, not by
      ordered evaluation. Ordering in the options editor is
      preserved for user readability and lines up with the
      on-disk JSON.
- [x] Persistent log: moved `tab_events` off the in-memory
      per-tab `HashMap` into `chrome.storage.session["firewallLog"]`.
      Single global FIFO (10k cap) tagged by `tab_id` on each
      event. Hydrates on SW cold-wake; survives tab close and
      navigation. Popup filters by `tab_id` (This tab / All tabs),
      action (block/allow/remove/hide/spoof), and substring
      search across rule_id / match / url / element description.
- [x] Options UI: up/down reorder buttons on every rule row;
      new Allow section next to Block/Remove/Hide/Spoof
      (shipped in phase 2); tag input (comma-separated);
      comment field.
- [x] Popup firewall-log UI: search box (URL / match /
      rule_id substring), action filter, "This tab" vs "All
      tabs" toggle. Tag filter chips deferred — tags are
      authored now but usage is sparse; the log is filterable
      via substring search against any tag string. Revisit if
      users accumulate enough tags that chips become useful.
- [ ] Bench: `compute_suggestions` Criterion before/after;
      expected no regression. Needs local run outside k3s.

### Stage 10: Rule import/export profiles

*Done when: users can save the current config (or a named
subset) to a JSON profile file and merge profiles back in with
conflict resolution (skip, overwrite, rename). Seeded profiles
ship in the repo: "news-site baseline", "developer baseline",
"social-media declutter".*

- [ ] Profile export UI: pick rules by scope + tag; serialize to
      portable JSON including ordering, tags, allow rules.
- [ ] Profile merge UI: import JSON; per-rule conflict dialog.
- [ ] Seed profiles in the repo under `profiles/`.

### Stage 11: Auto-tags

*Done when: every rule created by accepting a suggestion carries
an `auto:<signal-kind>` tag (e.g. `auto:replay-vendor`,
`auto:canvas-fp`, `auto:pixel`). The popup's firewall log gains
tag filter chips populated from the distinct tags across the
current rule set, so "show me all session-replay blocks" is a
click. Manually-typed tags coexist without the prefix.*

- [x] `handle_accept_suggestion` stamps the originating
      suggestion's signal kind into `RuleEntry.tags` as
      `auto:<kind>` before writing, via a new
      `RuleEntry::from_accepted_suggestion` helper. Kind comes
      from `Suggestion.kind`, populated by every detector via
      `LearnKind::tag()`.
- [x] ~~`FirewallEvent` carries the matching rule's tags so the
      log view doesn't need a config lookup to filter by tag.~~
      Resolved differently: popup pre-builds a
      `rule_id -> tags` HashMap at mount from the active
      `SiteConfig`, so the filter is a map lookup without
      bloating every event on the wire.
- [x] Popup firewall-log: tag filter chips rendered from the
      distinct tag set across every authored rule. Click a
      chip to AND it into the filter; click again to remove.
- [x] Regression test:
      `from_accepted_suggestion_stamps_auto_tag` locks the
      tag-stamping behaviour.

### Stage 12: Rule lint

*Done when: the options UI surfaces three classes of rule health
signal — **dead rules** (configured but never matched on any
tab's current session), **shadowed rules** (a block rule that an
earlier/global allow always overrides), and **zero-match
selectors** (remove/hide rules whose selector is valid CSS but
matches nothing on the tab's typical DOM). Each diagnostic is a
per-rule badge in the options editor + a roll-up panel in the
popup. Extends the Stage 7 `BlockDiagnostic` shape to the
remaining actions.*

- [x] Shadow detection: `src/lint.rs::block_shadowed_by()`
      heuristic — allow's normalized pattern is a prefix of
      block's normalized pattern (`||` and `^` stripped).
      Popup firewall-log annotates the shadowing allow on
      every block row it covers. 9 unit tests cover the
      heuristic shape.
- [x] Zero-match selector check: popup reads the per-tab
      remove/hide stats maps it already receives and flags
      selectors with count=0 on the current tab. Only active
      in This-tab view so All-tabs doesn't false-flag a
      selector that matches on a different tab.
- [x] Popup firewall-log header roll-up: "N hits · X shadowed
      · Y zero-match" alongside the rule-count summary.
- [ ] **Phase B**: Extend per-rule diagnostics beyond block:
      `RemoveDiagnostic` / `HideDiagnostic` / `SpoofDiagnostic`
      / `AllowDiagnostic` with
      `status: "firing" | "no-hits" | "shadowed" | "broken"`.
- [ ] **Phase B**: Broken-selector surfacing. content.js
      already `try/catch`es `querySelectorAll` — collect which
      selectors threw and report them back in the stats
      message.
- [ ] **Phase B**: Dead-rule distinction — walk the persistent
      firewall-log in the background handler to compute
      per-rule "never fired in this session" vs. "not fired
      on this tab yet".
- [ ] **Phase B**: Options editor: per-row health badge (dot
      indicator + tooltip) next to disabled / up / down.

### Stage 13: Rule simulate

*Done when: users can type a URL or paste a DOM snippet into a
test-match form and see which rule would fire. Shows the full
evaluation trace: which patterns matched, why priority/order
picked the winner, which rules were close misses.*

- [ ] Test-match input surface in the options page: URL field
      for block/allow, CSS-selector field for remove/hide,
      kind-tag field for spoof.
- [ ] Evaluator exposed as a pure function
      `simulate_match(url_or_selector, action, config) ->
      Vec<RuleMatch>`. Returns every rule whose pattern matches
      plus a "winner" tag (respecting allow-over-block for
      URLs). Shares code with the real evaluator so simulation
      can never diverge from enforcement.
- [ ] Output UI: ordered list of matches with rule_id, scope,
      action, priority, and a badge on the winning row. Close
      misses (same URL host but different path, same selector
      family but different specificity) shown in a secondary
      list for debugging near-matches.

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
