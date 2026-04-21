# Roadmap

Forward-only. What's left to build, in rough priority order. Move
items up/down as you re-prioritize. Mark shipped items by deleting
them from this file — present-tense state lives in
[architecture.md](architecture.md) and the feature snapshot in
[completed.md](completed.md); rollout notes land in
[CHANGELOG.md](../CHANGELOG.md) and [history.md](history.md).

## Context: Hush in a Brave stack

Hush's maintainer uses Brave as the base browser. Brave Shields
already handles filter-list-based ad/tracker blocking, fingerprint
farbling at the browser layer (stronger than any extension spoof),
HTTPS upgrade, third-party storage partitioning, query-parameter
stripping, referrer rewriting, bounce-tracking protection, and
CNAME-cloaked tracker unmasking.

So Hush's value-add in a Brave world is **what Brave Shields
doesn't see**:

- per-site surgical DOM cleanup (Remove/Hide custom elements that
  no cosmetic list targets)
- first-party telemetry subdomains (`collector.*`,
  `w3-reporting.*`, `unagi.*`)
- session-replay listener density (Neuter / Silence)
- behavioral fingerprint detection (even with Brave farbling, the
  evidence log tells you which sites tried)
- new Web-API permission probes (Bluetooth / USB / HID / Serial)
- attention tracking (Visibility API + focus/blur density)
- clipboard read/write monitoring
- fingerprinting via battery / connection / screen orientation

Priorities below are tuned for that world. The `spoof` action
stays (non-Brave users still benefit) but is not a priority
because Brave covers it better at the browser layer.

## Priority 1

### `strip` action — URL query parameter removal

Brave strips `utm_source` / `gclid` / `fbclid` / `msclkid` / etc.
globally. Hush should expose a per-site **strip** action for the
params a site-specific tracker uses that general-purpose strip
lists miss. Fits the firewall model (one more (scope, action,
match) entry per row).

**Implementation**: new action type; each entry is a param name or
pattern. DNR supports this via `redirect` +
`transform.queryTransform.removeParams`. No content-script work
needed — the redirect happens at the network layer before the
request goes out.

**Evaluation order**: strip runs at the DNR layer alongside Block /
Allow; a Block beats Strip (request never went anyway), Allow
beats Strip (pass-through by user intent).

### `referrer` action — Referer header rewriting

Per-site Referer policy: strip, trim to origin, or set to a fixed
value on outbound requests. Brave does this globally; Hush should
let the user tune per-scope.

**Implementation**: DNR `modifyHeaders` rule. Small.

### Bounce-tracking / redirector detector

Click a link → hits `redirector.com/out?url=X` → 302s to the real
destination. The redirector exists to log the click. Brave catches
the common patterns; Hush can catch site-specific ones by
observation.

**Detection**: Resource Timing + navigation correlation. A
resource fetch immediately followed by a same-click navigation to
a different host, with the intermediate host serving only a 3xx,
is the pattern. Emit block suggestion for the redirector.

### Attention-tracking detector

Session-replay vendors, A/B-test frameworks, and "engagement
analytics" hook the Page Visibility API + `focus` / `blur` /
`pagehide` / `beforeunload` to measure how long your attention is
on the tab.

**Detection**: main-world hook on `addEventListener`. Count
registrations for `visibilitychange`, `focus`, `blur`, `pagehide`,
`pageshow`, `beforeunload` per script origin. Flag at 4+ on the
same origin within the first 3 seconds of load.

**Output**: Neuter suggestion (deny the listener registration)
rather than URL block — same pattern as the existing replay-
listener detector.

### Seeded profiles for Stage 10

Import/export profile merge is shipped. What's missing is curated
starter content under `profiles/` so new users have a reasonable
baseline to merge in.

Author three seed profiles:

- **news-site-baseline.json** — first-party telemetry beacon
  blocks, social-widget iframe removes, newsletter / cookie-banner
  overlay kills.
- **brave-supplement.json** — for Brave users: the site-specific
  Remove/Hide/Neuter rules that Shields can't reach. Smaller and
  tighter than a general-purpose profile; focused on
  high-traffic sites where behavioral detection has caught
  concrete patterns (reddit, amazon, github).
- **social-media-declutter.json** — Reddit promoted-post removes,
  algorithmic community recommendation removes, Twitter/X trending
  hide rules.

## Priority 2

### Tier 3: navigator / screen property fingerprinting

Beyond canvas/WebGL/audio, fingerprinters read 15-30 property
accessors on `navigator`, `screen`, and `window` in rapid
succession. The combination of values uniquely identifies 90%+ of
browser sessions.

**Detection strategy**: monkey-patch property getters on
`navigator` / `screen` prototypes via `Object.defineProperty`.
Count reads per-origin from stack traces. Flag when a single origin
reads 10+ of these properties within 3 seconds of load.

**Properties to monitor**:

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

### Clipboard API monitoring

Some sites monitor `navigator.clipboard.readText()` (requires user
gesture, but many sites probe) or wrap paste events to sniff
clipboard content for coupon codes / competitor URLs.

**Detection**: hook `navigator.clipboard.readText` /
`navigator.clipboard.writeText` in main-world. Flag any
`readText` call as high-signal (very few legit use cases). Output:
block suggestion for the reading script origin.

### New-Web-API permission probes

`Bluetooth.requestDevice` / `USB.requestDevice` / `HID.requestDevice`
/ `Serial.requestPort` are device-fingerprinting vectors. Legit
uses are rare and always user-initiated. Sites that merely *probe*
(check for API existence without calling) are also suspicious.

**Detection**: hook the five new-API entry points. Any call from a
non-user-initiated context is high-signal. Output: Neuter
suggestion for the script origin.

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

### Tier 4: storage-based supercookies

Lower priority in a Brave stack because Brave already partitions
third-party storage ephemerally — third-party iframes can't read
persistent IDs they set on a previous visit. Still relevant for
non-Brave users and first-party supercookies (tab-host writing an
ID to storage and exfiltrating it).

**Detection**:

1. Hook `localStorage.setItem` / `sessionStorage.setItem` /
   `indexedDB.open` from iframes whose origin differs from the
   tab's top-frame origin.
2. Count unique storage keys written per-origin. First-party
   sites write many small settings; third-party iframes usually
   write a single identifier.
3. Hook `document.cookie` reads from third-party iframes.

### Tier 6: service-worker registration disclosure

Hook `navigator.serviceWorker.register()` in `mainworld.js`. Report
each registration with scope and script URL. Not automatically
suspicious (PWAs, offline docs, chat apps legitimately use SWs) —
surface as disclosure in a popup "Service Workers registered"
panel.

### Battery / Connection API read detection

`navigator.getBattery()` (deprecated but still works in some
paths) and `navigator.connection.effectiveType` /
`.downlink` / `.rtt` are stable-ish signals. Detection only
(Brave spoofs the values); emit an informational entry in the
firewall log so the user knows which sites probed.

### Crypto-mining heuristic

Sustained main-thread Worker activity with `performance.now`-loop
patterns that look like hash rounds (tight busy loops, not
responsive to `setTimeout(0)` yields) on background tabs. Rare but
high-confidence when it fires.

**Detection**: sample Worker message rate + main-thread busy
ratio from `PerformanceObserver('longtask')`. Output: block
suggestion for the worker script origin.

### Cross-bucket rule reorder

Flat rules table currently greys out up/down arrows at bucket
edges. A single "move to scope X / action Y" dropdown per row
would let users relocate rules without separately editing scope +
action. Nice-to-have.

### Profile export: rule-subset picker

Profile export currently dumps the whole config. Let the user pick
a subset before export — by scope, by tag, or by action.

### Manifest / Cargo version drift cleanup

`manifest.json` says 0.10.0; `Cargo.toml` says 0.12.0. Bump the
manifest to match Cargo and cut a release.

## Out of scope

Not planned. Pulled back in via new entries if they become
blocking.

- **Filter-list engine** (EasyList / EasyPrivacy). uBlock Origin
  Lite and Brave Shields already do this. Hush's mandate is
  per-site surgical cleanup plus behavioral detection of what
  lists miss.
- **Cross-site tracker correlation** (Privacy Badger's core "3+
  sites" algorithm). Needs persistent stateful detection; big
  architectural load.
- **API randomization for anti-fingerprint** (Brave's Shields
  farbling approach). Requires deeper browser integration than
  MV3 extensions can replicate. Brave users get this for free at
  the browser layer; Hush's `spoof` action is the
  extension-sized fallback for non-Brave users.
- **HTTPS upgrade as default-on**. Doable via DNR redirects but
  conflicts with Hush's "nothing hardcoded" principle. Can ship
  as an opt-in seed profile rather than built-in behavior.
- **CNAME-cloaked tracker unmasking**. Brave does this by
  resolving DNS at the network stack; MV3 extensions have no
  DNS API and can't replicate it. Hush catches the same domains
  *behaviorally* (tiny responses, beacon patterns) when they
  fire.
- **Shared-core builds** (Tauri desktop app, native CLI HAR
  analyzer, mobile via `uniffi`). Attractive but out of scope
  until Hush is feature-complete in the browser.
- **Cookie-consent banner auto-dismissal** (I Don't Care About
  Cookies, Brave's Cookiecrumbler). Different problem from
  behavioral tracking; could be a separate companion extension.
