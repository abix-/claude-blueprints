# Hush vs. other blockers

Honest comparison of what Hush does, what it deliberately doesn't
do, and where it overlaps with the mainstream ad / tracker /
privacy extensions. Hush is a narrow tool with a strong thesis,
not a replacement for everything.

## TL;DR

**Hush is not a replacement for uBlock Origin.** It's a per-site
surgical tool with a firewall-style rule model that catches
behaviors public blocklists can't see:

- first-party telemetry subdomains (`collector.github.com`,
  `unagi.amazon.com`, etc.)
- site-specific custom elements (`shreddit-async-loader`,
  `faceplate-carousel`)
- session-replay listener density (Hotjar, FullStory, Clarity
  attaching 20+ interaction listeners)
- fingerprinting API reads (canvas, WebGL, audio, font-enum)

**Use Hush alongside a general blocker.** uBlock Origin Lite
handles EasyList and the broad case; Hush handles everything
site-specific and behavioral that the lists miss.

## Mental models at a glance

| Tool | How it decides | What it scales with |
|---|---|---|
| uBlock Origin Lite / uBO | Curated filter lists (EasyList, EasyPrivacy) | Quality + size of the community lists |
| AdBlock / AdBlock Plus | Curated lists (with optional "Acceptable Ads") | Same lists, different policies |
| Ghostery | Curated tracker database + cosmetic filters | Ghostery's tracker DB + WhoTracks.me |
| Privacy Badger | **Behavioral** — observes trackers across every site you visit, auto-blocks after 3+ appearances | Your own browsing time |
| Brave Shields | Filter lists + **browser-level** API randomization (canvas/audio farbling, font enumeration limits) | Browser integration (not an extension) |
| DuckDuckGo Privacy Essentials | DDG's Tracker Radar dataset + HTTPS upgrade | DDG's research team |
| NoScript | **Default-deny JavaScript**, whitelisted per-origin | Explicit user whitelist |
| Decentraleyes / LocalCDN | Cached copies of common CDN libraries | Curated CDN map |
| **Hush** | **User-authored firewall rules, seeded by a behavioral detector** | **Your own rule set** |

## Per-extension breakdown

### uBlock Origin (MV2) / uBlock Origin Lite (MV3)

**Model**: filter-list engine. Parses community lists (EasyList,
EasyPrivacy, AdGuard, Peter Lowe's, etc.) into an in-memory matcher.
Matches every outgoing request and cosmetic selector against the
compiled rules.

**Strengths**: battle-tested, massive community lists cover the top
~95% of tracking and advertising traffic. Extremely fast matcher.
Actively maintained.

**Weaknesses** (the gaps Hush targets):

- First-party telemetry is invisible to lists because lists target
  cross-site trackers. A site's own `collector.*`,
  `w3-reporting.*`, `telemetry.*` subdomains aren't on any list.
- Site-specific custom elements (Reddit's `shreddit-async-loader`,
  Amazon's promoted carousels) aren't generic enough for EasyList
  cosmetic rules.
- Fingerprint API reads (`UNMASKED_RENDERER_WEBGL`,
  `OfflineAudioContext`, `measureText`) aren't network traffic at
  all — filter lists can't see them.
- Session-replay detection is list-based: block the vendor
  domain. Works for scripts loaded from `hotjar.com`, fails for
  bundled first-party replay libraries.

**Overlap with Hush**: network blocking (Hush's Block action
targets the same layer via DNR). If you already have uBO running,
Hush's Block rules are mostly redundant for generic ad/tracker
patterns — the value is in the per-site Remove/Hide/Spoof/Neuter
rules.

### AdBlock / AdBlock Plus

**Model**: same filter-list engine pattern as uBO. AdBlock Plus
maintains the "Acceptable Ads" allowlist, which is a paid program —
advertisers pay to have their ads allowlisted if they meet style
guidelines. Opt-out in settings.

**Overlap with Hush**: zero by design. Hush has no "acceptable
ads" allowance and no payment relationships. Both block ads, but
Hush's allowlist is 100% user-controlled.

**When to prefer uBO over AB/ABP**: uBO has no acceptable-ads
allowance and a better matcher. Recommend uBO for anyone who
doesn't specifically want the AB brand.

### Ghostery

**Model**: curated tracker database (Ghostery maintains their own
classification, powered by the WhoTracks.me dataset) plus cosmetic
filters. UI focuses on "who tracked you" — per-page breakdown by
tracker company.

**Strengths**: excellent transparency UI. The "what trackers are
on this page" view is one of the best in the category.

**Overlap with Hush**: some. Both show per-site evidence; both
focus on trackers over ads. Hush's firewall log gives the same
"what fired" view but scoped to your own rules, not Ghostery's
DB classifications.

### Privacy Badger (EFF)

**Model**: **behavioral tracker learning**. Observes every outgoing
request. If a third-party domain appears on 3+ different sites
you visit and sends tracking signals (cookies, referrer data),
Privacy Badger blocks it. Learns on your own browsing; no curated
list.

**Strengths**: catches trackers that aren't on any list because
they're new, obscure, or specifically designed to evade lists. The
learning algorithm is the main thing no other tool does.

**Weaknesses**: needs observation time before it blocks anything.
First visit to a site, new trackers pass through. Doesn't see
first-party telemetry (same-host as the site you're visiting).

**Overlap with Hush**: partial — both watch live behavior. But:

- Privacy Badger watches **across sites** ("did I see this domain
  elsewhere?"). Hush's detector is **per-tab** ("is this page
  doing something suspicious right now?"). Different axis.
- Privacy Badger operates automatically. Hush surfaces
  **suggestions**; user accepts / dismisses explicitly. Hush's
  evidence-first UI is the difference.
- Privacy Badger can't see fingerprint API reads or session-replay
  listener density. Hush does.

Using both makes sense: Privacy Badger catches cross-site
trackers, Hush catches per-page behavioral signals.

### DuckDuckGo Privacy Essentials

**Model**: DDG-maintained Tracker Radar dataset (open-sourced) +
HTTPS upgrade + basic cookie management + grade-style privacy
score per site.

**Strengths**: curated by a research team with a public dataset.
Good mainstream UX; reasonable defaults.

**Overlap with Hush**: tracker domain blocking only. No behavioral
detection, no fingerprint hardening, no session-replay detection.

### Brave Shields (built into Brave browser)

**Model**: the most comprehensive browser-layer privacy stack
shipping. Filter-list-based ad/tracker blocking (EasyList,
EasyPrivacy, uBlock Origin lists, plus Brave's own curated set) +
fingerprint randomization at the API level + HTTPS upgrade +
storage partitioning + URL cleanup + bounce-tracking redirect +
CNAME-cloak unmasking + script replacement, all on by default.

**Verified shipped features** (cross-checked against Brave's
privacy-features page + Shields wiki + privacy-updates blog as
of this writing):

- **Ad & tracker blocking**: EasyList / EasyPrivacy / uBO lists +
  Brave-curated lists. Two modes: **Standard** (third-party
  only) and **Aggressive** (first-party too).
- **HTTPS Upgrade**: on by default. Strict mode shows an
  interstitial when upgrade fails.
- **Fingerprint farbling**: per-session, per-eTLD+1 randomization
  of canvas, audio, WebGL, font enumeration, plus many
  navigator/screen properties. Same site gets consistent values
  within a session; different sites see different values;
  reseeds on browser restart.
- **CNAME cloaking unmasking**: resolves CNAME records and
  checks both the original and resolved domain against filter
  lists. Brave was the **first browser** to ship this (1.17.73,
  Oct 2020). MV3 extensions cannot replicate — no DNS API.
- **Third-party storage partitioning** + **ephemeral storage**:
  third-party iframes can't read persistent IDs across sites;
  storage clears when the last tab for a site closes.
- **Query-parameter stripping**: removes tracking params
  (utm_*, gclid, fbclid, msclkid, etc.) from URLs globally via
  a curated list.
- **Debouncing**: recognizes a curated list of
  click-redirector domains and skips them entirely — navigates
  directly to the final URL instead of bouncing through the
  tracker. Shipped in 1.32 (Oct 2021).
- **Referrer rewriting**: trims or removes the Referer header
  on cross-site requests.
- **Social-media widget blocking**: configurable at
  `brave://settings/socialBlocking`. Facebook Connect, Twitter
  embed, LinkedIn Share, Google Sign-In.
- **Resource replacement**: replaces Google Analytics / Meta
  Pixel scripts with privacy-friendly no-op stubs so site code
  calling `gtag()` / `fbq()` doesn't error but doesn't track.
  Closest analog to Hush's Silence action but at the script
  layer, not the request layer.
- **Global Privacy Control (GPC)**: sends the GPC header on all
  requests. Legal signal under CCPA and similar laws.
- **De-AMP**: bypasses Google AMP pages, navigates to canonical.
- **Auto-redirect tracking URLs**: separate from debouncing;
  catches longer-tail redirectors.
- **Language fingerprinting reduction**: only sends primary
  `navigator.language`; hides full `navigator.languages`.
- **Client hints protection**: limits User-Agent Client Hints
  headers.
- **Auto Shred**: automatic site data clearing on configurable
  schedule.

**Farbling vs Hush's spoof — different goals**:

| | Brave farbling | Hush spoof |
|---|---|---|
| Implementation | Browser core | Extension (main-world hook) |
| Scope | Every site automatically | Per-site opt-in via `spoof` rules |
| Mechanism | Randomize (per-session noise, per-eTLD+1 seed) | Replace with constant (identical across all users) |
| Goal | Prevent **cross-session linking** (different you each visit) | Prevent **per-user identification** (everyone looks the same) |
| Trade-off | Small quality loss on legit canvas use, site-wide | Breaks opted-in sites' legit canvas use entirely |
| Transparency | Silent | Hush logs each spoof firing |

Subtle point: for per-session fingerprinting (site identifies you
within one visit), Brave's randomization still produces a
unique-ish value. Hush's constant produces the **same** value for
everyone. Brave is better at defeating **tracking over time**;
Hush's spoof is more aggressive at defeating **a single-visit
identify**. In practice both are fine; this is mostly theoretical.

**When Hush adds value on top of Brave Shields**: see the "Hush +
Brave stack" section below.

### NoScript

**Model**: default-deny all JavaScript, then whitelist per-origin.
High-security model from the Firefox era.

**Strengths**: ultimate defense against client-side tracking — if
no script runs, no fingerprint is computed and no tracker fires.

**Weaknesses**: modern web is unusable without JS on ~everything.
Whitelisting takes effort per site.

**Overlap with Hush**: almost none. NoScript blocks at the script
layer globally; Hush's Neuter action blocks **specific**
listener-registration surfaces from **specific** script origins
you authored rules against. Neuter is a scalpel where NoScript is
a cleaver.

### Decentraleyes / LocalCDN

**Model**: ship bundled copies of common CDN libraries (jQuery,
GoogleFonts, Bootstrap, etc.). When a page requests one of those
from a known CDN, the extension injects the local copy so the CDN
never sees the hit.

**Strengths**: defeats CDN-based tracking (Google Fonts tracking
correlation is the classic case).

**Overlap with Hush**: none. Complementary.

## Feature overlap matrix

Columns read left-to-right: filter-list engines first, then
behavioral / heuristic, then Hush at the right edge for contrast.

| Feature | uBO / uBOL | AdBlock+ | Ghostery | Privacy Badger | Brave Shields | DDG | Hush |
|---|---|---|---|---|---|---|---|
| Generic ad blocking (EasyList) | ✔ | ✔ | ✔ | partial | ✔ | partial | ✗ |
| Cross-site tracker blocking | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | partial (via user rules) |
| First-party ad/tracker blocking (aggressive) | ✗ | ✗ | ✗ | ✗ | ✔ (Aggressive mode) | ✗ | via user rules |
| **First-party telemetry detection** | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | **✔** |
| **Site-specific custom-element cleanup** | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | **✔** |
| Session-replay vendor detection | list-based | list-based | list-based | no | list-based | ✗ | **behavioral** |
| **Session-replay listener density check** | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | **✔** |
| Canvas fingerprint defense | ✗ | ✗ | ✗ | ✗ | ✔ randomize | ✗ | ✔ spoof (opt-in) |
| WebGL fingerprint defense | ✗ | ✗ | ✗ | ✗ | ✔ randomize | ✗ | ✔ spoof (opt-in) |
| Audio fingerprint defense | ✗ | ✗ | ✗ | ✗ | ✔ randomize | ✗ | ✔ spoof (opt-in) |
| Font-enum fingerprint defense | ✗ | ✗ | ✗ | ✗ | ✔ randomize | ✗ | ✔ spoof (opt-in) |
| Navigator/screen property defense | ✗ | ✗ | ✗ | ✗ | ✔ randomize | ✗ | planned (Tier 3 detect) |
| Query-parameter stripping | via list | ✗ | ✗ | ✗ | ✔ | ✗ | planned (per-site `strip`) |
| Referrer header rewriting | via list | ✗ | ✗ | ✗ | ✔ | ✗ | planned (per-site `referrer`) |
| Bounce-tracking / debouncing | via list | ✗ | ✗ | ✗ | ✔ | ✗ | planned (site-specific detect) |
| CNAME-cloaked tracker unmasking | ✗ | ✗ | ✗ | ✗ | ✔ | ✗ | ✗ (no DNS API in MV3) |
| Third-party storage partitioning | ✗ | ✗ | ✗ | ✗ | ✔ | ✗ | ✗ (browser-level only) |
| Resource replacement (GA/Pixel stubs) | ✗ | ✗ | ✗ | ✗ | ✔ | ✗ | planned (`replace` action) |
| Global Privacy Control header | ✗ | ✗ | ✗ | ✗ | ✔ | ✗ | ✗ |
| De-AMP / canonical URL follow | ✗ | ✗ | ✗ | ✗ | ✔ | ✗ | ✗ |
| Cookie banner auto-dismiss | via list | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| HTTPS upgrade | ✗ | ✗ | ✗ | ✔ | ✔ | ✔ | ✗ |
| Per-page evidence UI | partial | ✗ | **✔ (best)** | ✔ | partial | partial | **✔ (firewall log)** |
| **User rule authoring** | limited (custom list) | ✗ | ✗ | ✗ | ✗ | ✗ | **✔ (primary model)** |
| **Behavioral suggestions** | ✗ | ✗ | ✗ | ✔ cross-site | ✗ | ✗ | **✔ per-tab, evidence-first** |
| No network egress from extension | ✗ fetches lists | ✗ | ✗ | ✗ | n/a | ✗ | **✔** |
| MV3 native | uBOL yes, uBO no | yes | yes | yes | n/a | yes | **yes** |
| Cross-site correlation | ✗ | ✗ | partial | **✔** | partial | ✗ | ✗ |

## Hush + Brave stack

Brave Shields covers most of what Hush's Block / Spoof / Allow
actions do, only better (browser layer, farbling, DNS-level CNAME
unmasking). A Brave user shouldn't expect Hush to add value in
those lanes. What **does** Hush add on top of Shields:

**Unique to Hush — not covered by Shields at any mode**:

1. **Site-specific surgical DOM cleanup** (Remove, Hide). Shields
   has cosmetic filters from EasyList but can't target custom
   elements like Reddit's `shreddit-async-loader` or Amazon's
   `ad-feedback` components.
2. **First-party telemetry subdomain detection**
   (`collector.github.com`, `unagi.amazon.com`, etc.).
   Filter-list-free; behavioral signal from Resource Timing
   (small responses, beacon types, same-host as tab).
3. **Session-replay listener density check**. Catches
   **bundled** first-party replay libraries that don't ship from
   `hotjar.com` — the Neuter action denies
   `addEventListener('mousemove' | 'keydown' | …)` at
   document_start.
4. **Evidence log of what tried to fingerprint you**. Brave
   farbles silently; Hush's firewall log tells you that
   `siteX.com` called `getParameter(UNMASKED_RENDERER_WEBGL)` 12
   times in 2 seconds. Transparency layer, not additional
   defense.
5. **Per-site rule-authoring UI** for any of the above, with
   live suggestions tied to evidence.

**Complementary to Shields — per-site overrides on top of
Brave's global decisions**:

6. **`strip` action** (planned): Brave's curated param-strip
   list is global. Hush's per-site strip lets you remove
   site-specific tracking params Brave's list doesn't know
   about — without turning off Brave's global strip.
7. **`referrer` action** (planned): Brave's Referer rewriting
   is policy-level. Hush can pin a Referer to a specific value
   on a specific scope (useful for paywalled sites that gate on
   the Referer header).
8. **Spoof action** for sites where you want **stronger**
   fingerprint defense than farbling. Trade-off: spoof returns
   a constant (everyone identical, risks breaking legit canvas
   use); farbling returns consistent-per-session noise (safer,
   less aggressive). Use spoof only on sites where you know
   the fingerprint attempt is malicious.

**Future Hush additions specifically for Brave users**
(prioritized in [roadmap.md](roadmap.md)):

- Attention-tracking detector (Visibility API + focus/blur
  density). Brave doesn't specifically target this.
- Clipboard API monitoring (`navigator.clipboard.readText`).
  Brave doesn't hook this.
- New-Web-API permission probes
  (Bluetooth/USB/HID/Serial/Web Share). Brave limits some but
  doesn't detect/report probes.
- Tier 3 navigator-property fingerprint **detection** (not
  spoofing — Brave farbles it). Transparency value.
- `replace` action — substitute matched scripts with no-op
  stubs. Brave already does this for GA4 / Meta Pixel; Hush
  would cover site-specific cases.

## When to use which

**Default recommendation for most people**: uBlock Origin Lite (or
uBlock Origin on Firefox) as the primary blocker. It's free, fast,
actively maintained, and covers 95% of generic tracking without
any configuration.

**Add Privacy Badger if** you want cross-site tracker learning on
top of lists. No configuration needed; learns as you browse.

**Add Hush if** you:

- hit sites where uBO misses something and you don't want to wait
  for a community list update,
- see a site doing first-party telemetry under its own subdomains
  (the network tab shows `beacon.theirsite.com` or similar),
- want to surgically remove site-specific UI elements (promoted
  posts, sidebar widgets) that no cosmetic list targets,
- want fingerprint-API defense without switching to Brave,
- want to neutralize session-replay scripts via listener-denial
  rather than URL blocking (so the script's site code still
  works).

**Switch to Brave browser instead of stacking extensions if** you
prioritize fingerprint randomization over extension flexibility.
Brave Shields is strictly better on fingerprint defense because
it operates at the browser layer.

**Use NoScript if** you're doing high-security browsing (research
on hostile sites, investigations) where JS-off is acceptable.

## What Hush deliberately doesn't do

Pulled forward from [roadmap.md](roadmap.md) "Out of scope" for
visibility in the comparison context:

- **Filter-list engine**. Not reimplementing EasyList. uBO exists.
- **Cross-site tracker correlation** (Privacy Badger's 3+ sites
  algorithm). Attractive but requires persistent stateful
  detection across tabs; big architectural load, and Privacy
  Badger already does it well.
- **API randomization** (Brave's farbling approach). Requires
  deeper browser integration than an MV3 extension can replicate.
  Hush's spoof action is the extension-sized version — replaces
  entropy signals with constants rather than per-session noise.
- **Cookie banner auto-dismissal** (I Don't Care About Cookies,
  Cookiecrumbler). Different problem domain; possible companion
  extension but not Hush's mandate.
- **Network egress** of any kind. No filter-list fetches, no
  telemetry, no phone-home. Every other extension in this doc
  except NoScript fetches something at install time or on a
  schedule.

## Thesis

The gap public blocklists leave is **per-site behavioral detail**.
A site's custom elements, its first-party telemetry subdomains,
its specific session-replay listener pattern, its fingerprint API
reads — these don't generalize across millions of users, so
curated lists don't have them. An extension that **observes live
behavior and proposes rules the user accepts** fills that gap
without duplicating what the lists already cover.

Hush is the implementation of that thesis. It's small on purpose:
a firewall rule engine, a behavioral detector that emits
evidence-carrying suggestions, seven action types for the kinds of
anti-user behavior browsers can expose. Everything else is
deferred to tools that already do it well.
