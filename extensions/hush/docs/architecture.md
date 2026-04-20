# Hush architecture — firewall-style rule engine for Chrome

Hush is **modeled after a software firewall**. It isn't one at the OS
level — it doesn't gate packets and it lives entirely inside the
browser process. But the mental model it borrows from network
firewalls like Palo Alto is the right one for what it does: every
request, element, and fingerprint probe a page makes is checked
against user-authored rules, matched rules fire an action, and each
action emits an evidence-carrying log entry. This doc explains that
model, and it's the model every future feature should be designed
against.

## Threat model

Every website you load is running code that is not on your side.

- Ad networks, analytics vendors, and session-replay companies embed
  scripts that exfiltrate what you do on the page — mouse movements,
  scroll timing, keystrokes, which elements you hovered.
- First-party "telemetry" pipelines (sites' own `collector.*`,
  `w3-reporting.*`, `unagi.*` subdomains) fire `sendBeacon` events
  throughout a normal session. They are not on any public blocklist
  because public lists target cross-site trackers, not first-party
  subdomains. Hush's mandate is to close exactly that gap.
- Fingerprinting scripts read GPU model, canvas pixel signatures,
  installed-font lists, WebGL parameters — combining three or four
  of those uniquely identifies 90%+ of browser sessions regardless
  of cookies, incognito, or VPN.
- Heavy UI elements (sticky promos, hidden iframes, background
  animation loops) sit on the page running CPU and network whether
  you look at them or not.

The page author's interests and the reader's interests are in
direct conflict. A firewall-style rule engine is the right shape
for the tool: something that sits between the page and the user
and decides, per rule, what gets through.

## Rule model

A Hush rule is a **(scope, action, match)** triple:

| Dimension | Values | What it means |
|---|---|---|
| Scope | `global` \| `<domain>` | Which tabs the rule evaluates on |
| Action | `block` \| `allow` \| `remove` \| `hide` \| `spoof` | What the rule does when it matches |
| Match | URL pattern, CSS selector, or fingerprint kind | What the rule looks at |

Rules also carry optional metadata used by the log and the editor:

| Field | Purpose |
|---|---|
| `disabled` | Boolean; skipped by the evaluator without deleting the row |
| `tags` | Free-form labels (`analytics`, `session-replay`, `auto:fp`). Drives log filters |
| `comment` | Author's note to future self. Shown in the options editor |

### Actions (what a rule does)

1. **Block (network)** — registered with
   `chrome.declarativeNetRequest`. Matching requests are rejected
   before DNS resolution, TCP connection, or TLS handshake. The
   initiating `fetch()` or iframe load fails locally with
   `net::ERR_BLOCKED_BY_CLIENT`. No bytes reach the network.
2. **Allow (exception)** — the counter-rule to Block. Matching
   requests pass through even if a broader Block rule would cover
   them. DNR handles the override by giving Allow rules a higher
   `priority` than Block rules so DNR's own first-match-wins
   resolution picks Allow. For Remove/Hide, an Allow rule is a
   selector exclusion: nodes matching an Allow selector are skipped
   by the content-script applier. This is the primitive that lets a
   user write "globally block `||doubleclick.net`, but allow
   `doubleclick.net/adx/` on this one site" — impossible in earlier
   versions.
3. **Remove (DOM)** — CSS selectors whose matching elements are
   physically deleted via `element.remove()`. A `MutationObserver`
   re-applies on every DOM mutation so SPA routers and infinite-scroll
   insertions can't sneak the element back in.
4. **Hide (CSS)** — CSS selectors applied with
   `display: none !important` via a user stylesheet injected at
   `document_start`. The element stays in the DOM; it doesn't
   render. Mildest action.
5. **Spoof (fingerprint)** — kind tags (e.g. `webgl-unmasked`) that
   swap the real value returned by a fingerprinting API for a
   bland identical-across-users default. First case: WebGL
   `UNMASKED_VENDOR_WEBGL` / `UNMASKED_RENDERER_WEBGL` returning
   `"Google Inc."` / `"ANGLE (Generic)"` instead of your actual
   GPU identity. Lets the page keep rendering while killing the
   fingerprint's entropy contribution.

### Evaluation order

Rules are evaluated **first-match-wins within each action**,
top-down in authoring order. The options editor lets the user
reorder rows; order is persisted verbatim in `chrome.storage.local`.

For the Block vs. Allow cross-action case, DNR's own
priority-resolution takes over: Allow rules get a higher numeric
`priority` than Block rules, so DNR returns the first matching
Allow before any Block has a chance to fire. For Remove/Hide,
the content-script applier walks rules in order and excludes
nodes matched by an Allow selector from the subsequent
Remove/Hide passes.

Actions are otherwise orthogonal — Block gates the network,
Remove touches the DOM, Spoof touches fingerprint APIs — so
there is no cross-action ordering beyond the Allow/Block override.

### Scopes (where a rule applies)

- **Site-scoped** (current default) — the rule lives under a site
  config key (e.g. `reddit.com`) and evaluates on any tab whose
  hostname exactly matches or is a suffix of that key. A rule
  under `reddit.com` also applies on `www.reddit.com`,
  `sh.reddit.com`, and `old.reddit.com`.
- **Global** (planned, not yet shipped) — the rule evaluates on
  every tab. Useful for blanket bans on known-bad ad-network
  hosts or session-replay vendors the user always wants killed.
  Tracked as a separate stage in [roadmap.md](roadmap.md).

### Match types

- **URL pattern** (for Block): uBlock-style (`||host.example.com`,
  `||foo.com^`, `*.cdn.example.com`, path wildcards). Handed to
  `chrome.declarativeNetRequest`'s `urlFilter` condition.
- **CSS selector** (for Remove + Hide): any selector `querySelectorAll`
  accepts, including `:has(...)` and attribute-starts-with matches.
  The stable-selector heuristics used in the case studies
  ([reddit](reddit.md), [amazon](amazon.md), [github](github.md))
  apply: prefer custom-element tag names and stable data attributes
  over utility-class chains.
- **Kind tag** (for Spoof): a short string identifying which
  fingerprint signal to neutralize. Currently supported:
  `webgl-unmasked`. Future: `canvas`, `audio`, `font-enum`.

### The rule-hit event

Every rule match emits one [`FirewallEvent`](../src/types.rs)
(`{t, rule_id, action, scope, match, tags, disposition, evidence}`)
into the firewall-log buffer. The popup's **Firewall log** section
reads the buffer, applies the current filter set (action / tag /
tab / search substring), and aggregates by `rule_id`, showing each
rule's hit count + last-hit timestamp + expandable recent-evidence
list.

| Action | Event shape | Evidence | Status |
|---|---|---|---|
| Block | `FirewallEvent` | `{url, resourceType}` | shipped |
| Allow | `FirewallEvent` | `{url, resourceType}` (action=allow) | shipped |
| Remove | `FirewallEvent` | `{el}` (element description) | shipped |
| Hide | `FirewallEvent` | `None` (count-only, no per-element events) | next |
| Spoof | `FirewallEvent` | `None` / kind-specific | next |

`rule_id` is derived as `"{action}::{scope}::{match}"` (see
`src/types.rs::rule_id`) — the same format as suggestion keys, so
an accepted suggestion's key matches the resulting rule's ID in
the log. `tags` are copied from the matching `RuleEntry` at emit
time so the log can be filtered by category (e.g. all
session-replay blocks) without re-reading config. `disposition` is
`"block"` or `"allow"` and records whether an Allow rule
overrode a Block; an Allow match records the overridden rule's
`rule_id` in evidence so the log shows the exception chain.

### Log persistence

The firewall log lives in `chrome.storage.session` under
`"firewall_log"` as a single FIFO buffer of [`FirewallEvent`]
objects, capped at 10k entries (≈2MB, well under the 10MB quota).
Every event carries `tabId` so the popup can pivot between
"This tab" and "All tabs" views. The session-storage backing
means the log survives popup close and tab reload but is
cleared when the browser restarts — aligning with the
privacy-preserving "no persistent behavioral history" principle
(the log records user-authored-rule hits, not raw behavior).

## Runtime data flow

```
 page load
    │
    ▼
 [DNR dynamic rules]   ← rebuilt from config on onInstalled /
    │                    onStartup / storage.onChanged
    │
    ▼
 network request ──── BLOCK? ──── request fails locally ──┐
    │ pass                                                 │
    ▼                                                      │
 frame commit                                              │
    │                                                      │
    ▼                                                      │
 content script @ document_start                           │
    ├─ read matched site config                            │
    ├─ inject hide-layer <style>                           │
    ├─ write data-hush-spoof on <html> if spoof enabled    │
    └─ install MutationObserver for remove layer           │
          │                                                │
          ▼                                                │
     DOM mutations ─── remove matches ─── element deleted  │
                                                           │
 main-world (isolated script, document_start):             │
   WebGL.getParameter intercepted                          │
     ├─ emit webgl-fp observation to detector              │
     └─ if spoof tag present, return bland string ─┐       │
                                                    │       │
                                                    ▼       ▼
                            service worker receives events / stats
                                   │
                                   ▼
                            popup reads per-tab state
                            firewall log accumulates
```

## Rule lifecycle

1. **Authoring**. Rules come from one of three sources:
   - User types a rule into the options editor (site list +
     per-site editor, with a section per action).
   - User clicks **Add** on a behavioral suggestion surfaced by
     the detector (see below). The suggestion's `layer` + `value`
     become a rule.
   - User pastes JSON into the raw-JSON editor.
2. **Persistence**. Rules live in `chrome.storage.local["config"]`
   as a `{ domain: { block: [...], remove: [...], hide: [...],
   spoof: [...] } }` tree. Plain JSON, inspectable, exportable.
3. **Install**. On every `chrome.storage.onChanged`, the service
   worker (`src/background.rs`) re-syncs dynamic DNR rules for the
   Block action. Content scripts re-read on tab reload for Remove
   / Hide / Spoof.
4. **Evaluation**. Per request or per DOM mutation — see flow diagram.
5. **Logging**. Matched-rule events accumulate in per-tab state
   for the popup. See "The rule-hit event" above.

## Detector → rule pipeline

Hush's **behavioral suggestions** feature (opt-in, off by default)
watches live page behavior and emits proposed rules. Signals are
listed in the main [README](../README.md#signals-used). Every
suggestion is a proposed **(scope, action, match)** triple — the
same shape as a stored rule. Clicking **Add** promotes the
proposal to a stored rule; **Dismiss** drops it per-tab-session;
**Allow** records the proposal key in
`chrome.storage.local["allowlist"].suggestions` so it never
resurfaces.

Every suggestion carries raw evidence (URLs, sizes, timestamps,
outerHTML snippets, stack traces) so the user can verify before
accepting. This is the firewall's equivalent of "suggested rules
from IDS alerts".

## Allowlists

Three independent user-editable allowlists live in
`chrome.storage.local["allowlist"]`:

- **`iframes`** — URL substrings. A hidden iframe whose `src`
  matches any entry is skipped by the detector (captcha, OAuth,
  payment, bot-management by default).
- **`overlays`** — CSS selectors. A sticky overlay that matches
  any selector is skipped by the detector (React Portals, modal
  roots, framework shells).
- **`suggestions`** — full suggestion keys (e.g.
  `block::||example.com::canvas-fp`). Populated by the **Allow**
  button. Any listed key is filtered out at emit time.

Allowlists are **detector-scoped**, not rule-scoped. They suppress
*suggestions*, not stored rules. If you want a stored rule to
stop firing, delete the rule; if you want a suggestion to stop
appearing, Allow it.

## Where each piece lives

```
extensions/hush/
  manifest.json            MV3 manifest. Permissions +
                           content_security_policy + key.
  background.js            18-line wasm bootstrap + log-sink relay.
  content.js               Pure-JS content script (runs on every
                           matched frame at document_start, applies
                           remove + hide + spoof-marker; can't run
                           wasm because page CSP can block it on
                           strict sites like reddit).
  mainworld.js             Main-world hooks (fetch/XHR/beacon/
                           WebSocket/canvas/WebGL/audio/font/
                           listener monkey-patching).
  popup.js                 20-line wasm bootstrap.
  options.js               34-line wasm bootstrap.
  src/                     Rust/WASM engine:
    types.rs                  SiteConfig, Allowlist, Suggestion,
                              SignalPayload — the schema contracts.
    background.rs             Service-worker runtime: listeners,
                              DNR sync, per-tab state, message
                              handlers.
    ui_popup.rs               Leptos popup components.
    ui_options.rs             Leptos options components.
    chrome_bridge.rs          Reflect + wasm-bindgen wrappers for
                              chrome.* APIs.
    compute.rs                compute_suggestions — pure detection
                              engine.
    detectors.rs              Per-signal detector implementations.
    allowlist.rs              Allowlist matching helpers.
    canon.rs                  URL canonicalization + pattern_keyword.
    suggestion.rs             build_suggestion + dedup diagnostic.
    learn.rs                  Per-signal teaching text.
    main_world.rs             Rust runtime scaffolding (not loaded in
                              practice — main world has no
                              chrome.runtime.getURL for wasm).
    stack.rs                  Script-origin extraction from JS stacks.
  docs/
    architecture.md          This file.
    reddit.md                Case study: reddit.com rules.
    amazon.md                Case study: amazon.com rules.
    github.md                Case study: github.com rules.
    heuristic-roadmap.md     Per-signal research notes.
    roadmap.md               Stage-by-stage plan.
    history.md               Completed-stage rollout notes.
    completed.md             Current feature snapshot.
    benchmarks.md            compute_suggestions perf.
  sites.json                 Seed rules — case studies only.
  tools/log-server.mjs       Local dev HTTP log sink.
```

## Future stages

Tracked in [roadmap.md](roadmap.md).

**Shipped**: global scope (reserved `__global__` key), stable
per-rule IDs, unified `FirewallEvent` ring buffer, Palo-Alto-style
firewall-log popup section aggregating by `rule_id`.

**Stage 7 closeout** (quick wins that finish the current surface):

- **Hide + Spoof event emission.** Hide is count-based today
  (CSS selector hit counts, not per-element events); Spoof is
  silent. Wire a main-world → content → bg event path so both
  actions show up in the firewall log uniformly alongside
  block/remove.
- **Per-rule disable toggle.** Each rule row gets a switch so the
  user can mute a rule temporarily without deleting it. DNR sync
  excludes disabled rules; content-script applier skips them;
  `compute_suggestions` ignores them for dedup.

**Stage 8** (fingerprint coverage):

- **More spoof kinds.** `canvas` (return a fixed hash from
  `toDataURL` / `getImageData`), `audio` (stub
  `OfflineAudioContext`), `font-enum` (return a fixed allowlist
  from `measureText`). Each follows the same content-script →
  dataset → main-world-hook pattern the WebGL spoof uses.

**Stage 9 — PA primitives** (the shape-change that makes Hush a
real firewall, not a blocker list):

- **Allow action** with DNR priority override + content-script
  selector exclusion. Enables write-through exceptions to broader
  Block/Remove rules.
- **First-match-wins ordering within each action.** Rules become
  an ordered list; options editor gains up/down reordering.
- **Persistent searchable firewall log.** Migrate the per-tab
  ring buffer to a single `chrome.storage.session` buffer
  (10k cap). Popup gains action / tag / tab / search filters.
- **Rule tags.** Each `RuleEntry` carries a `tags: Vec<String>`.
  Detector-origin tags (from accepted suggestions) are prefixed
  `auto:` so the log can distinguish hand-authored vs. derived
  rules.

**Stage 10** (serialization on top of the Stage 9 primitives):

- **Rule import/export profiles.** Named rule bundles — e.g.
  "news-site baseline" (telemetry beacons + generic overlay
  killers), "developer baseline" (session-replay vendors + GPU
  fingerprint spoof) — users can merge in on a per-machine basis.
  Deferred to after Stage 9 because a profile is only interesting
  once ordering, tags, and allow-overrides exist to serialize.

## Design principles

These are the rules of thumb every change should preserve.

- **Nothing hardcoded in user-visible behavior**. Every blocklist,
  allowlist, and rule is user-editable in the options UI. The
  shipped `sites.json` is only a seed; the user owns everything.
- **Evidence-first UI**. Every suggestion carries the raw
  observations that triggered it. Every rule hit carries enough
  context (URL, selector, element description, timestamp) to
  explain what was killed, not just that something was killed.
- **No network egress from the extension itself**. No filter-list
  fetches, no analytics, no phone-home. Behavioral state lives
  in `chrome.storage.session` only.
- **Surgical cleanup beats broad blocking**. Public blocklists
  already cover the easy cases. Hush's value is per-site, per-
  signal precision on things those lists can't see — first-party
  telemetry subdomains, site-specific custom elements, fingerprint
  APIs. The Kovarex rule: if a rule would be a one-line addition
  to EasyList, it doesn't belong in Hush.
- **Performance budget**. Popup cold-open < 100ms. Content-script
  DOM scans capped at 5000 elements. MutationObserver only watches
  childList + subtree, not attributes. The detector's opt-in because
  even small per-scan CPU costs compound at reddit-scroll rates.
