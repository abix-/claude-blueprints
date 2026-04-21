# Roadmap

Forward-only. What's left to build, in rough priority order. Move
items up/down as you re-prioritize. Mark shipped items by deleting
them from this file — present-tense state lives in
[architecture.md](architecture.md) and the feature snapshot in
[completed.md](completed.md); rollout notes land in
[CHANGELOG.md](../CHANGELOG.md) and [history.md](history.md).

## Context: Hush in a Brave stack

Hush's maintainer uses Brave as the base browser. Verified
against Brave's current docs (privacy-features page, Shields
wiki, privacy-updates blog), Brave Shields already ships all of
this on-by-default at the browser layer:

- EasyList / EasyPrivacy / uBO lists + Brave-curated lists (ad
  and tracker blocking). Standard mode = third-party only;
  Aggressive mode = first-party too.
- Fingerprint **farbling** — per-session per-eTLD+1 seed
  randomizes canvas, audio, WebGL, font-enum, and many
  navigator/screen properties. Stronger than any extension spoof
  because it operates before the JS returns.
- HTTPS upgrade (Strict mode shows interstitial on failure).
- **CNAME-cloaked tracker unmasking** (Brave was first browser to
  ship this, 1.17.73). MV3 extensions have no DNS API and can't
  replicate.
- Third-party storage partitioning + ephemeral storage.
- URL query-parameter stripping (curated global list).
- Referrer header rewriting (policy-level).
- **Debouncing** (1.32+) — skips known click-redirectors
  entirely.
- Social-media widget blocking (Facebook Connect, Twitter embed,
  LinkedIn Share, Google Sign-In).
- **Resource replacement** — GA4 / Meta Pixel scripts replaced
  with no-op stubs at load time.
- Global Privacy Control header, De-AMP, language-fingerprinting
  reduction, Client Hints limiting, Auto Shred.

That's a wide feature footprint. Hush's value-add shrinks to
what Brave can't see or can't target:

**Gaps worth closing** (priority order):

1. **Site-specific surgical DOM cleanup** (Remove/Hide for custom
   elements). Shipped; Hush's core.
2. **First-party telemetry subdomain detection**. Shipped;
   Hush's core.
3. **Session-replay listener-density detection** — catches
   bundled first-party replay that doesn't ship from known vendor
   domains. Shipped; unique to Hush.
4. **Attention-tracking detector** (Visibility API + focus/blur
   density). Not shipped; Brave doesn't specifically target this.
5. **Clipboard API monitoring** (`readText` / `writeText` hooks).
   Not shipped; Brave doesn't hook this.
6. **New-Web-API permission probes** (Bluetooth / USB / HID /
   Serial). Not shipped; Brave may limit but doesn't detect.
7. **Per-site `strip` / `referrer` / `replace` overrides** on top
   of Brave's global decisions. Not shipped; complementary.
8. **Navigator-property fingerprint detection** (Tier 3 — not
   spoofing; Brave farbles). Not shipped; transparency value
   only.
9. **Unknown-redirector detection** — long-tail bounce trackers
   Brave's curated list hasn't caught yet. Not shipped.

The `spoof` action stays (non-Brave users benefit; Brave users
who want constant-return aggressive defense on specific sites can
opt in) but isn't a priority.

Priorities below are tuned to this gap list.

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

### Site-specific redirector detector

Brave already has **two** layers of bounce-tracking defense: a
curated debouncing list (1.32+, skips known redirectors
entirely) plus filter-list blocks. Both are curated, so
long-tail site-specific redirectors slip through.

Hush's angle: detect **unknown** redirector patterns on sites
the user is active on, surface them as suggestions. Users can
block locally or submit upstream to Brave / filter-list
maintainers.

**Detection**: Resource Timing + navigation correlation. A
resource fetch immediately followed by a same-click navigation
to a different host, with the intermediate host serving only a
3xx, is the pattern. Emit block suggestion for the redirector.
Redundant with Brave's curated list when the redirector is
already known; unique value is the long tail.

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

### `replace` action — substitute scripts with no-op stubs

Brave already does this for Google Analytics (gtag.js) and Meta
Pixel (fbevents.js) — the script loads, page code calling
`gtag()` / `fbq()` doesn't error, but the calls are no-ops.

Hush's extension: a **per-site, user-authored `replace` rule**
that substitutes a matched URL with an extension-packaged stub.
Covers site-specific analytics libraries that Brave's curated
replacement list doesn't target.

**Implementation**: DNR `redirect` with `redirect.extensionPath`
pointing to a bundled stub file. Stubs live under
`stubs/ga.js` / `stubs/fbq.js` / etc. and expose a tiny no-op
API. User specifies `{ match: "||example.com/analytics.js",
stub: "noop" }` per rule.

**Why separate from Block**: Block fails the request entirely,
which can crash page code that checks for the script's globals.
Replace preserves the global surface, kills the tracking.

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
