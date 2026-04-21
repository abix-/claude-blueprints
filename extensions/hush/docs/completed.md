# Current user-facing features

Snapshot of what's shipped in Hush right now. Systems list, not
changelog (see [CHANGELOG.md](../CHANGELOG.md) for dated rollout
notes and [history.md](history.md) for retired stage narrative).

## Seven rule actions

Every rule is a `(scope, action, match)` triple. Seven actions
available:

- **Block** (network) — URL pattern; matching requests fail with
  `net::ERR_BLOCKED_BY_CLIENT` via `chrome.declarativeNetRequest`.
- **Allow** (exception) — counter-rule to Block. DNR priority
  override at the network layer; CSS-selector exclusion at the
  DOM layer. Lets a site-scoped rule carve an exception out of a
  broader global rule.
- **Neuter** (script capture) — script-origin URL pattern. At
  `document_start`, denies `addEventListener` registrations for
  interaction events (click/keydown/mouse/scroll/touch) from
  matching origins. Upstream defense for session-replay vendors.
- **Silence** (script exfil) — script-origin URL pattern.
  Intercepts outbound `fetch` / `XMLHttpRequest` /
  `navigator.sendBeacon` from matching origins and fake-succeeds
  them (204 / state 4 / true). Fallback for bundled replay
  libraries where Neuter can't match cleanly by origin.
- **Remove** (DOM) — CSS selector; matching elements are
  physically deleted via `element.remove()`. A MutationObserver
  re-applies on every mutation.
- **Hide** (CSS) — CSS selector; matching elements get
  `display: none !important` via a user stylesheet at
  `document_start`. Leaves the element in the DOM.
- **Spoof** (fingerprint) — kind tag; returns bland constants
  from the named fingerprint API instead of real values.
  Supported kinds: `webgl-unmasked`, `canvas`, `audio`,
  `font-enum`.

Rules evaluate **first-match-wins within each action**. The
cross-action case (Allow overriding Block) goes through DNR
priority, not table ordering.

## Rule metadata

Every rule is a `RuleEntry { value, disabled, tags, comment }`,
not a bare string. Metadata lives next to the match value.

- `disabled` — boolean; evaluator skips the rule without deleting
  the row (DNR sync excludes, content-applier skips, suggestion
  dedup ignores).
- `tags` — free-form labels. Detector-origin tags use the
  `auto:<kind>` prefix (`auto:canvas-fp`, `auto:replay-vendor`,
  etc.) so the firewall log can distinguish hand-authored from
  detector-derived rules.
- `comment` — author's note.

## Scopes

- **Site-scoped** — rules live under a hostname key
  (`reddit.com`). Apply when the tab's hostname exactly matches
  or is a suffix of the key (`reddit.com` covers
  `www.reddit.com`, `sh.reddit.com`, etc).
- **Global** — reserved `__global__` key. Rules apply on every
  tab, layered underneath site-scoped rules. No schema migration;
  underscore-prefixed keys can't collide with real hostnames.

## Options UI: flat firewall-style rule table

Single sortable table — one row per rule across every scope and
every action. Columns: `On | # | Scope | Action | Match | Tags |
Comment | ↑ ↓ ×`. Scope and action are inline `<select>` cells;
changing them relocates the rule between storage buckets without
leaving the row.

- Filter bar above the table: scope filter, action filter,
  free-text search over value / tags / comment / scope.
- **+ Add rule** appends a row with defaults (`Global` /
  `Block` / empty Match).
- **+ New site...** option in the scope dropdown prompts for a
  hostname and creates the entry lazily.
- Per-row up/down reorder within the `(scope, action)` bucket.
- Per-row enable checkbox, tags field, comment field, delete.
- **Status dot** in the `#` column indicates rule health
  (disabled / broken / shadowed / firing / no-hits) with a
  `title` tooltip carrying the human-readable reason.

## Firewall log (popup)

Every rule match emits one `FirewallEvent` into a persistent
global FIFO buffer (`chrome.storage.session["firewall_log"]`,
10k cap). Survives SW restarts, popup close, tab navigation;
clears on browser restart.

Popup's Firewall Log section aggregates events by `rule_id`
(`"{action}::{scope}::{match}"`). Each row shows action badge,
scope tag, match, hit count, last-hit time. Click-to-expand for
recent per-event evidence (last 20). Filters: tab (This tab /
All tabs), action (block / allow / remove / hide / spoof / …),
tag chips, substring search.

Inline rule-health annotations per row:

- **shadowed by allow** — block rule covered by an earlier allow
  (DNR-unreachable).
- **no DOM match on this tab** — remove/hide selector with count
  0 on the current tab.
- **invalid selector (threw on querySelectorAll)** — selector
  rejected by the browser's CSS parser; content-script flagged
  it on a recent pass.

Header roll-up: `N rules · X hits · Y shadowed · Z zero-match ·
W broken`.

## Behavioral detector

Opt-in (off by default). Watches live page behavior and emits
suggestions to the popup with per-signal teaching text and
raw-observation evidence.

Signals currently emitted:

**Network-layer** (via Resource Timing):

- `sendBeacon` targets (confidence 95)
- Tracking pixels (confidence 85)
- First-party telemetry subdomains (confidence 70)
- Polling endpoints (confidence 75)
- Hidden iframes (confidence 80, with allowlist for
  captcha/OAuth/payment)
- Sticky overlays (confidence 55)

**Main-world fingerprinting** (via API hooks):

- Canvas fingerprinting (toDataURL / toBlob / getImageData) —
  confidence 90
- WebGL `UNMASKED_RENDERER_WEBGL` / `UNMASKED_VENDOR_WEBGL`
  reads — confidence 95
- WebGL general `getParameter` density (8+) — confidence 75
- Audio fingerprinting (`OfflineAudioContext` construction) —
  confidence 90
- Font enumeration via `measureText` (20+ distinct font
  families) — confidence 85

**Session replay**:

- Vendor global poll (Hotjar, FullStory, Clarity, LogRocket,
  Smartlook, Mouseflow, PostHog) — confidence 95
- Listener density on document/window/body (12+ interaction
  listeners from one origin within 60s) → suggests **Neuter**
  rather than URL block — confidence 80

**Attention tracking**:

- Attention / page-lifecycle listener density (4+
  `visibilitychange` / `focus` / `blur` / `pagehide` /
  `pageshow` / `beforeunload` listeners across 3+ distinct
  types from one origin within 60s) → suggests **Neuter** —
  confidence 75. Catches engagement analytics and
  session-replay dwell-time hooks that Brave Shields doesn't
  specifically target.

**Clipboard read**:

- Any `navigator.clipboard.readText()` call from a page script
  → suggests **Block** for the calling origin — confidence 95.
  Chrome gesture-gates the API but legit page-script use is
  near-zero (password managers and clipboard inspectors run as
  extensions, not page scripts), so one call is enough signal.
  Catches coupon / competitor-URL sniffing and paste-in
  tracking that Brave doesn't hook.

**Hardware device-API probes**:

- Any call to `Bluetooth.requestDevice` / `USB.requestDevice` /
  `HID.requestDevice` / `Serial.requestPort` from a page script
  → suggests **Block** for the calling origin — confidence 90.
  Legit uses are rare (maker-space, industrial, dev-tool
  contexts) and always clearly user-initiated. Random web
  pages calling these are fingerprint probes — the permission
  prompt itself is an entropy signal. Brave doesn't hook any
  of these APIs. `navigator.share` is excluded because legit
  share-button use is common.

**Invisible animation loop**:

- Hot 2D canvas draw ops sample target-canvas visibility once
  per 100ms per canvas. Sustained invisible-canvas draws → block
  suggestion for the script origin. Confidence 70.

## Per-suggestion actions

Every suggestion has three buttons:

- **+ Add** writes the rule. Opens a scope picker (site vs
  global) so the user chooses where the rule lives.
- **Dismiss** suppresses the suggestion for the current tab
  session.
- **Allow** persists the suggestion's key in
  `allowlist.suggestions`, filtering it out on every site
  forever. Revocable from the options allowlist editor.

Plus two diagnostic panels:

- **Why?** — dedup diagnostic: tab hostname vs matched config
  key, existing rule count + sample, dedup outcome.
- **Evidence** — raw observations (URLs, sizes, timestamps,
  outerHTML snippets, stack traces) with a clipboard copy
  button.

Accepted suggestions inherit an `auto:<signal-kind>` tag on the
resulting rule so the firewall log can filter by detector origin.

## Rule simulate (test-match)

Collapsible panel on the options page. Paste a URL (and
optionally a site hostname); the pure
`simulate::simulate_url(config, site, url)` walks the active
config and returns every matching rule across Global + site
scopes with action / scope / value / priority. DNR winner is
flagged with a checkmark. Disabled rules surface with
strikethrough. Shares code with the real evaluator so simulation
can't diverge from enforcement.

## Rule profiles (import/export)

Options page has **Import profile...** and **Export as
profile...** buttons.

- Export wraps the current config with a user-supplied name +
  description inside a `hushProfile` header and triggers a
  download.
- Import parses the profile JSON, previews the merge (added /
  skipped counts), confirms, then unions the profile's rules
  into the live config. Existing rule metadata (disabled /
  tags / comment) is preserved; new rules append to the end of
  their bucket.

## Allowlists

Three sections, user-editable from the options page:

- **iframes** — URL substrings. Hidden iframes whose src
  contains any entry are skipped at detection time. Defaults
  cover captcha, OAuth, payment widgets.
- **overlays** — CSS selectors. Sticky/fixed elements matching
  any selector are skipped.
- **suggestions** — full suggestion keys. Populated by the
  Allow button.

## Popup debug payload

One-click clipboard copy of a JSON snapshot: manifest version,
active config, dynamic DNR rules + per-rule fire counts, per-tab
activity + behavior summary, recent blocked URLs + removed
elements, dismissed suggestions, recent log ring buffer. Drop-in
for bug reports.

## Case studies

- [reddit.md](reddit.md)
- [amazon.md](amazon.md)
- [github.md](github.md)
