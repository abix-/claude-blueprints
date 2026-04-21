# Roadmap

Forward-only. What's left to build, in priority order, **for a
Brave (latest) user**. Move items up/down as you re-prioritize.
Mark shipped items by deleting them from this file — present-tense
state lives in [architecture.md](architecture.md) and the feature
snapshot in [completed.md](completed.md); rollout notes land in
[CHANGELOG.md](../CHANGELOG.md) and [history.md](history.md).

## Context: Hush in a Brave stack

Hush's maintainer uses **Brave (latest)** as the base browser.
Verified against Brave's current docs (privacy-features page,
Shields wiki, privacy-updates blog), Brave Shields already ships
all of this on-by-default at the browser layer:

- EasyList / EasyPrivacy / uBO lists + Brave-curated lists.
  Standard mode = third-party only; Aggressive mode = first-party too.
- Fingerprint **farbling** — per-session per-eTLD+1 seed
  randomizes canvas, audio, WebGL, font-enum, and many
  navigator/screen properties. Stronger than any extension spoof.
- HTTPS upgrade (Strict mode shows interstitial on failure).
- **CNAME-cloaked tracker unmasking** — MV3 extensions have no
  DNS API and can't replicate.
- Third-party storage partitioning + ephemeral storage.
- URL query-parameter stripping (curated global list).
- Referrer header rewriting (policy-level).
- **Debouncing** — skips known click-redirectors entirely.
- Social-media widget blocking.
- **Resource replacement** — GA4 / Meta Pixel scripts replaced
  with no-op stubs.
- Global Privacy Control header, De-AMP, language-fingerprinting
  reduction, Client-Hints limiting, Auto Shred.

Given that footprint, the priority rule becomes: **promote items
that do things Brave doesn't, demote items that duplicate Brave
at a narrower scope**. Specifically, per-site overrides of Brave's
global decisions (strip, referrer, redirector detection) are
useful but not priority — they cover edge cases a default-on
Brave install already handles.

## Priority 1 — pure detection gaps Brave doesn't cover

These are behavioral signals Brave doesn't specifically target.
They're the cleanest fit for Hush's thesis (per-tab behavioral
observation with evidence-first suggestions) and add value that
no amount of Brave-tuning replicates.

### Attention-tracking detector

Session-replay vendors, A/B-test frameworks, and "engagement
analytics" hook the Page Visibility API + `focus` / `blur` /
`pagehide` / `beforeunload` to measure how long your attention is
on the tab. Brave doesn't neutralize this.

**Detection**: main-world hook on `addEventListener`. Count
registrations for `visibilitychange`, `focus`, `blur`, `pagehide`,
`pageshow`, `beforeunload` per script origin. Flag at 4+ on the
same origin within the first 3 seconds of load.

**Output**: Neuter suggestion (deny the listener registration)
rather than URL block — same pattern as the existing replay-
listener detector.

### Clipboard API monitoring

Some sites monitor `navigator.clipboard.readText()` (gesture-
gated but many sites probe) or wrap paste events to sniff
clipboard content for coupon codes / competitor URLs. Brave
doesn't hook this.

**Detection**: hook `navigator.clipboard.readText` /
`navigator.clipboard.writeText` in main-world. Flag any
`readText` call as high-signal (very few legit use cases).
Output: block suggestion for the reading script origin.

### New-Web-API permission probes

`Bluetooth.requestDevice` / `USB.requestDevice` /
`HID.requestDevice` / `Serial.requestPort` /
`navigator.share` are device-fingerprinting vectors. Legit uses
are rare and always user-initiated. Sites that merely *probe*
(check for API existence) are also suspicious. Brave may limit
some but doesn't detect/report probes.

**Detection**: hook the five new-API entry points. Any call from
a non-user-initiated context is high-signal. Output: Neuter
suggestion for the script origin.

### Tier 3 navigator/screen property fingerprint **detection**

Fingerprinters read 15-30 property accessors on `navigator`,
`screen`, and `window` in rapid succession. Brave **farbles** the
values (defense), but silently — the user never learns which
sites attempted. Hush's value here is the **transparency layer**.

**Detection strategy**: monkey-patch property getters on
`navigator` / `screen` prototypes via `Object.defineProperty`.
Count reads per-origin from stack traces. Flag when a single
origin reads 10+ properties within 3 seconds of load.

**Properties to monitor**:

- `navigator`: userAgent, platform, language, languages,
  hardwareConcurrency, deviceMemory, maxTouchPoints,
  cookieEnabled, doNotTrack, plugins, mimeTypes, vendor,
  webdriver, connection, onLine
- `screen`: width, height, availWidth, availHeight, colorDepth,
  pixelDepth
- `window`: devicePixelRatio, innerWidth, innerHeight, screenX,
  screenY

**Tricky bits**: legit code reads `navigator.userAgent` for
browser detection. A 10+ DIFFERENT-property threshold screens
most single-purpose sniffs. Output: informational firewall-log
entry (since Brave already farbles, a Block suggestion is
redundant).

### Seeded profiles + brave-supplement.json

Import/export profile merge is shipped. Missing is curated
starter content.

Author three seed profiles:

- **brave-supplement.json** — the Hush-specific bits a Brave
  user benefits from: site-specific Remove/Hide rules,
  first-party telemetry subdomain blocks, Neuter rules for
  bundled session-replay libraries. Tight and focused; pairs
  cleanly with Brave Shields.
- **news-site-baseline.json** — first-party telemetry beacon
  blocks, social-widget iframe removes, newsletter / cookie-
  banner overlay kills.
- **social-media-declutter.json** — Reddit promoted-post
  removes, algorithmic community recommendation removes,
  Twitter/X trending hide rules.

## Priority 2 — polish + specialized detection

### Crypto-mining heuristic

Sustained main-thread Worker activity with `performance.now`-loop
patterns that look like hash rounds (tight busy loops, not
responsive to `setTimeout(0)` yields). Rare but high-confidence
when it fires. Brave doesn't specifically catch this.

**Detection**: sample Worker message rate + main-thread busy
ratio from `PerformanceObserver('longtask')`. Output: block
suggestion for the worker script origin.

### Tier 4 first-party supercookies

Brave's third-party storage partitioning handles the worst case
(third-party iframes setting persistent IDs). First-party
supercookies — the tab host itself writing a user ID to
`localStorage` then exfiltrating it — still work.

**Detection**: hook `localStorage.setItem` /
`sessionStorage.setItem` in main-world. Count unique keys per
origin. Correlate with outbound fetch/beacon hitting the same
origin that reads the key back.

### Tier 6 service-worker registration disclosure

Hook `navigator.serviceWorker.register()` in `mainworld.js`.
Report each registration with scope and script URL. Not
automatically suspicious — surface as disclosure in a popup
"Service Workers registered" panel. Lets the user see what's
running in the background for a given site.

### Conflict-resolution dialog for profile import

Current import is additive union with dedup by value (existing
rules keep metadata; new rules append). Expand:

- Per-rule conflict row: value-match on a rule with different
  `tags` / `comment` / `disabled` offers "keep mine" / "use
  imported" / "merge tags".
- Per-scope conflict: offer "merge into existing" / "rename
  profile scope".

### compute_suggestions Criterion bench

Run before/after Criterion bench for the recent detector
additions. Expected no regression. Needs local run outside k3s.

### Popup cold-open perf verification

Stage 4 performance budget is cold popup open < 100 ms. Verify
in DevTools Performance. If over budget, prioritize the slow path.

## Priority 3 — edge cases + drift cleanup

### `strip` action — per-site URL query param override

Brave's curated global list covers `utm_source` / `gclid` /
`fbclid` / `msclkid` etc. Per-site override matters only for
site-specific tracking params Brave doesn't know about. Rare.

**Implementation**: new action type; DNR `redirect` +
`transform.queryTransform.removeParams`.

### `referrer` action — per-scope Referer rewriting

Brave does global policy-level Referer reduction. Per-site
override matters for paywall-debugging and referral-testing edge
cases. Rare.

**Implementation**: DNR `modifyHeaders` rule.

### Site-specific redirector detection

Brave has **two** layers of bounce-tracking defense already
(curated debouncing list + filter lists). Long-tail value here
is small — only useful for site-specific redirectors Brave
hasn't caught. Demoted from earlier P1 since the practical
overlap with Brave's existing coverage makes this rarely fire in
practice.

### `replace` action — substitute scripts with no-op stubs

Brave does this for GA4 / Meta Pixel already. Per-site user-
authored version would cover site-specific analytics Brave's
curated list doesn't target.

**Implementation**: DNR `redirect` with `redirect.extensionPath`
pointing to a bundled stub file. Stubs live under `stubs/` and
expose tiny no-op APIs.

### Battery / Connection API read detection

`navigator.getBattery()` and `navigator.connection.*` are stable-
ish signals. Detection only (Brave spoofs the values); emit an
informational entry in the firewall log.

### Cross-bucket rule reorder

Flat rules table currently greys out up/down arrows at bucket
edges. A "move to scope X / action Y" dropdown per row would
allow relocation without separately editing scope + action.

### Profile export: rule-subset picker

Profile export currently dumps the whole config. Let the user
pick a subset before export — by scope, by tag, or by action.

### Manifest / Cargo version drift cleanup

`manifest.json` says 0.10.0; `Cargo.toml` says 0.12.0. Bump the
manifest to match Cargo and cut a release. Mechanical.

## Out of scope

Not planned. Pulled back in via new entries if they become
blocking.

- **Filter-list engine** (EasyList / EasyPrivacy). uBO and Brave
  already do this.
- **Cross-site tracker correlation** (Privacy Badger's "3+ sites"
  algorithm). Needs persistent stateful detection.
- **API randomization / farbling for anti-fingerprint** (Brave's
  approach). Requires deeper browser integration than MV3 can
  replicate. Hush's `spoof` action is the extension-sized
  fallback for non-Brave users.
- **HTTPS upgrade default-on**. Brave does this.
- **CNAME-cloaked tracker unmasking**. Brave does this; MV3
  extensions have no DNS API.
- **Global Privacy Control header, De-AMP, Auto Shred, Client
  Hints limiting**. All browser-core; extensions can't meaningfully
  add.
- **Shared-core builds** (Tauri, native CLI, uniffi mobile).
- **Cookie-consent banner auto-dismissal**. Different problem;
  separate companion extension if ever.
