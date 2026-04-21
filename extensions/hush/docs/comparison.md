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

**Model**: filter lists + **browser-integrated fingerprint
randomization** (canvas farbling, WebGL randomization, font
enumeration limits) + HTTPS Everywhere rules + cookie controls +
ephemeral storage for third-party iframes.

**Strengths**: the fingerprint randomization is genuine defense
that an extension cannot match — Brave perturbs the actual API
returns at the browser level. Extensions can only spoof specific
reads.

**Overlap with Hush**: conceptually close on fingerprint defense
but implemented at a different layer.

| | Brave Shields | Hush |
|---|---|---|
| Implementation | Browser core | Extension (main-world hook) |
| Scope | Every site automatically | Per-site opt-in via `spoof` rules |
| Mechanism | **Randomize** (farbling — per-session noise added to canvas/audio reads) | **Replace with constant** (bland identical-across-users values) |
| Trade-off | Small quality loss on legit canvas use site-wide | Breaks opted-in sites' legit canvas use |
| Reversibility | Toggle in Shields | Per-site rule on/off |
| Transparency | Brave tells you it's happening | Hush logs each spoof firing |

Brave Shields is strictly a better general solution for
fingerprint defense **if you're willing to use Brave**. Hush's
spoof exists because most users aren't on Brave, and a per-site
extension-level approach beats nothing.

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
| **First-party telemetry detection** | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | **✔** |
| **Site-specific custom-element cleanup** | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | **✔** |
| Session-replay vendor detection | list-based | list-based | list-based | no | ✗ | ✗ | **behavioral** |
| **Session-replay listener density check** | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | **✔** |
| Canvas fingerprint defense | ✗ | ✗ | ✗ | ✗ | ✔ randomize | ✗ | ✔ spoof (opt-in) |
| WebGL fingerprint defense | ✗ | ✗ | ✗ | ✗ | ✔ randomize | ✗ | ✔ spoof (opt-in) |
| Audio fingerprint defense | ✗ | ✗ | ✗ | ✗ | ✔ randomize | ✗ | ✔ spoof (opt-in) |
| Font-enum fingerprint defense | ✗ | ✗ | ✗ | ✗ | ✔ limit | ✗ | ✔ spoof (opt-in) |
| Cookie banner auto-dismiss | via list | ✗ | ✗ | ✗ | partial | ✗ | ✗ |
| HTTPS upgrade | ✗ | ✗ | ✗ | ✔ | ✔ | ✔ | ✗ |
| Per-page evidence UI | partial | ✗ | **✔ (best)** | ✔ | partial | partial | **✔ (firewall log)** |
| **User rule authoring** | limited (custom list) | ✗ | ✗ | ✗ | ✗ | ✗ | **✔ (primary model)** |
| **Behavioral suggestions** | ✗ | ✗ | ✗ | ✔ cross-site | ✗ | ✗ | **✔ per-tab, evidence-first** |
| No network egress from extension | ✗ fetches lists | ✗ | ✗ | ✗ | n/a | ✗ | **✔** |
| MV3 native | uBOL yes, uBO no | yes | yes | yes | n/a | yes | **yes** |
| Cross-site correlation | ✗ | ✗ | partial | **✔** | partial | ✗ | ✗ |

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
