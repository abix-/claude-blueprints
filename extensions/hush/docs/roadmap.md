# Roadmap

Forward-only. What's left to build, in rough priority order. Move
items up/down as you re-prioritize. Mark shipped items by deleting
them from this file — present-tense state lives in
[architecture.md](architecture.md) and the feature snapshot in
[completed.md](completed.md); rollout notes land in
[CHANGELOG.md](../CHANGELOG.md) and [history.md](history.md).

## Priority 1

### Seeded profiles for Stage 10

Import/export profile merge is shipped. What's missing is curated
starter content under `profiles/` so new users have a reasonable
baseline to merge in.

Author three seed profiles:

- **news-site-baseline.json** — first-party telemetry beacon
  blocks, social-widget iframe removes, newsletter / cookie-banner
  overlay kills.
- **developer-baseline.json** — session-replay vendor blocks
  (hotjar, fullstory, clarity, logrocket, smartlook), WebGL +
  canvas + font-enum spoof opt-ins for common dev-tool sites.
- **social-media-declutter.json** — Reddit promoted-post removes,
  algorithmic community recommendation removes, Twitter/X trending
  hide rules.

Each profile carries the `hushProfile` header (name, description,
version) the importer already expects.

### Conflict-resolution dialog for profile import

Current import is additive union with dedup by value (existing
rules keep metadata; new rules append to their bucket). That's the
safe default but loses the chance to overwrite stale metadata or
rename a scope. Expand the import flow:

- Per-rule conflict row: value-match on a rule with different
  `tags` / `comment` / `disabled` offers "keep mine" / "use
  imported" / "merge tags".
- Per-scope conflict: if a profile ships `reddit.com` rules and
  the user already has `reddit.com` entries, offer "merge into
  existing" / "rename profile scope".

### Tier 3: navigator / screen property fingerprinting

Beyond canvas/WebGL/audio, fingerprinters read 15-30 property
accessors on `navigator`, `screen`, and `window` in rapid
succession. The combination of values uniquely identifies 90%+ of
browser sessions.

**Detection strategy**: monkey-patch property getters on
`navigator` / `screen` prototypes via `Object.defineProperty`.
Count reads per-origin from stack traces. Flag when a single origin
reads 10+ of these properties within 3 seconds of load.

**Properties to monitor** (from Brave's Shields design + academic
surveys):

- `navigator`: userAgent, platform, language, languages,
  hardwareConcurrency, deviceMemory, maxTouchPoints, cookieEnabled,
  doNotTrack, plugins, mimeTypes, vendor, webdriver, connection,
  onLine
- `screen`: width, height, availWidth, availHeight, colorDepth,
  pixelDepth
- `window`: devicePixelRatio, innerWidth, innerHeight, screenX,
  screenY

**Tricky bits**: legit code reads `navigator.userAgent` for
browser detection. A 10+ DIFFERENT-property threshold screens most
single-purpose sniffs (which read 1-2). Output: block suggestion
for the stack-origin script.

## Priority 2

### Tier 4: storage-based supercookies

Tracking IDs persisted in `localStorage`, `sessionStorage`,
`IndexedDB`, Cache API, ETag/Last-Modified headers. Third-party
iframes with storage access can set a user ID once and read it on
every subsequent visit to any embedding site.

**Detection strategy**:

1. Hook `localStorage.setItem` / `sessionStorage.setItem` /
   `indexedDB.open` from iframes whose origin differs from the
   tab's top-frame origin. Any write by a third-party iframe to
   persistent storage is a supercookie flag.
2. Count unique storage keys written per-origin. First-party sites
   write many small settings; third-party iframes usually write a
   single identifier.
3. Hook `document.cookie` reads from third-party iframes (reading
   an implicitly-set value is a cross-frame coordination hint).

**Plumbing**: content script already runs in every frame
(`all_frames: true`). Each frame checks `window.top === window` to
detect third-party iframe context. Hooks live in `mainworld.js`.
Suggestion: Remove the third-party iframe.

### compute_suggestions Criterion bench

Run the before/after Criterion bench for the first-match-wins
changes from Stage 9 and the detector additions since. Expected no
regression. Needs local run outside k3s (Criterion needs real
hardware baselines).

### Popup cold-open perf verification

The stated Stage 4 performance budget is cold popup open < 100 ms.
Verify in DevTools Performance. If we're over budget, prioritize
the slow path (likely the Leptos mount + async-fetch waterfall).

## Priority 3

### Tier 6: service worker registration tracking

Hook `navigator.serviceWorker.register()` in `mainworld.js`. Report
each registration with scope and script URL. Not automatically
suspicious (PWAs, offline docs, chat apps legitimately use SWs) —
surface as disclosure in a popup "Service Workers registered"
panel. Lets the user see what's running in the background for a
given site without forcing a rule.

### Cross-bucket rule reorder

Flat rules table currently greys out up/down arrows at bucket
edges. A single "move to scope X / action Y" dropdown per row
would let users relocate rules without separately editing scope +
action. Nice-to-have.

### Profile export: rule-subset picker

Profile export currently dumps the whole config. Let the user pick
a subset before export — by scope (tabs for "just `reddit.com`"),
by tag ("just `auto:canvas-fp` rules"), or by action. Builds a
smaller, more shareable JSON.

### Manifest / Cargo version drift cleanup

`manifest.json` says 0.10.0; `Cargo.toml` says 0.12.0. Bump the
manifest to match Cargo and cut a release. Mechanical, blocks any
user-facing "what version am I running" answer.

## Out of scope

Not planned. Pulled back in via new entries if they become
blocking.

- **Filter-list engine** (EasyList / EasyPrivacy). uBlock Origin
  Lite already does this. Hush's mandate is per-site surgical
  cleanup plus behavioral detection of what lists miss.
- **Cross-site tracker correlation** (Privacy Badger's core "3+
  sites" algorithm). Needs persistent stateful detection; big
  architectural load.
- **API randomization for anti-fingerprint** (Brave's Shields
  approach). Requires deeper browser integration than MV3
  extensions can replicate. Spoof action is the Hush-sized
  version — replaces entropy signals with identical-across-users
  constants rather than per-session randomized noise.
- **Shared-core builds** (Tauri desktop app, native CLI HAR
  analyzer, mobile via `uniffi`). Attractive but out of scope
  until Hush is feature-complete in the browser.
- **Cookie-consent banner auto-dismissal** (I Don't Care About
  Cookies, Brave's Cookiecrumbler). Different problem from
  behavioral tracking; could be a separate companion extension.
