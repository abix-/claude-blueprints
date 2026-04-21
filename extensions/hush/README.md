# Hush

**Firewall-style rule engine for Chrome.** Every website you load is
running code that is not on your side — ad networks, analytics vendors,
session-replay capture, first-party telemetry pipelines, fingerprinters
reading your GPU model and installed fonts. Public blocklists catch the
cross-site cases, but site-specific anti-user behavior is a gap public
lists can't fill. Hush is the tool for that gap: a per-site (and soon
global) rule engine that decides what the page is allowed to do. The
mental model — **rules, scopes, actions, hit logs** — is borrowed from
enterprise network firewalls, even though the enforcement points
(DNR, DOM, CSS, prototype hooks) are all inside Chrome.

Read [docs/architecture.md](docs/architecture.md) for the full mental
model, threat model, and rule taxonomy. The short version:

## The rule model

A Hush rule is a **(scope, action, match)** triple.

- **Scope**: `Global` (the reserved `__global__` key — applies to every
  tab) or a hostname (applies to that hostname and its subdomains via
  exact-or-suffix matching).
- **Action**: `block`, `allow`, `neuter`, `silence`, `remove`, `hide`,
  or `spoof`.
- **Match**: URL pattern, CSS selector, script-origin URL pattern, or
  fingerprint kind tag, depending on the action.

The seven actions:

1. **Block (network)** — URL patterns registered with
   `chrome.declarativeNetRequest`. Matching requests are rejected by the
   browser before DNS resolution, TCP connection, or TLS handshake. No
   bytes reach the network; the initiating `fetch()` or iframe load
   fails locally with `net::ERR_BLOCKED_BY_CLIENT`.

2. **Allow (exception)** — the counter-rule to Block. Matching requests
   pass through even when a broader Block rule would cover them (DNR
   priority override). For Remove/Hide, an Allow selector excludes
   matching nodes from the DOM passes. Use it to carve an exception out
   of a global rule on a single site.

3. **Neuter (script capture)** — script-origin URL patterns. At
   `document_start`, main-world denies `addEventListener` calls for
   interaction events (click/keydown/mouse/scroll/touch) from matching
   script origins. No listener, no capture, no exfil. Upstream defense
   for session-replay vendors.

4. **Silence (script exfil)** — script-origin URL patterns. Main-world
   intercepts outbound `fetch` / `XMLHttpRequest.send` /
   `navigator.sendBeacon` from matching script origins and fake-succeeds
   them (204 No Content / XHR state 4 / beacon true). Fallback for
   bundled first-party replay libraries where Neuter can't match cleanly
   by origin.

5. **Remove (DOM)** — CSS selectors whose matching elements are
   physically deleted via `element.remove()`. A `MutationObserver`
   re-applies on every DOM mutation so SPA routers and infinite-scroll
   insertions can't sneak the element back in.

6. **Hide (CSS)** — CSS selectors applied with
   `display: none !important` via a user stylesheet injected at
   `document_start`. Skips layout, paint, and compositing. Mildest
   layer; leaves the element in the DOM and its JavaScript running.

7. **Spoof (fingerprint)** — intercept specific fingerprinting APIs and
   return bland identical-across-users values instead of blocking the
   site. Supported kinds:
   - `webgl-unmasked` — `getParameter(UNMASKED_VENDOR_WEBGL)` /
     `getParameter(UNMASKED_RENDERER_WEBGL)` return `"Google Inc."` /
     `"ANGLE (Generic)"` instead of the real GPU identity.
   - `canvas` — `toDataURL` / `toBlob` return a constant 1x1 PNG;
     `getImageData` returns a zero-initialized `ImageData`. Kills
     subpixel-rendering fingerprints; opt-in per site since legit
     canvas uses will break.
   - `audio` — `OfflineAudioContext.startRendering` resolves to a
     silent `AudioBuffer`, neutralizing audio-rendering-divergence
     fingerprints.
   - `font-enum` — `measureText` returns synthetic metrics whose
     width depends only on text length, killing installed-font probes.

Rules are evaluated **first-match-wins within each action**, top-down
in authoring order. The options UI is a single flat table that lets you
reorder rows; order is persisted verbatim in `chrome.storage.local`.
The Block↔Allow cross-action override runs through DNR priority, not
table ordering. Other actions are orthogonal (Block gates the network,
Remove/Hide touch the DOM, Spoof touches fingerprint APIs), so there
is no cross-action ordering to worry about.

## Install

1. Clone or download this folder.
2. Open `chrome://extensions/` and enable **Developer mode** (top right).
3. Click **Load unpacked** and select the `hush/` folder.
4. Click the Hush toolbar icon to open the popup, then **Open options** to
   configure sites.

## Configure

The options page is a **flat firewall-style rule table**. Every rule is
one row; scope and action are inline dropdowns on each row:

| On | # | Scope | Action | Match | Tags | Comment |
|---|---|---|---|---|---|---|

- **Scope**: `Global` (reserved `__global__` key, applies to every tab)
  or any hostname you've added. Changing the dropdown moves the rule
  between scopes; `+ New site...` prompts for a hostname and creates the
  entry lazily.
- **Action**: one of the seven action types (Block, Allow, Neuter,
  Silence, Remove, Hide, Spoof). Changing the dropdown moves the rule
  between action buckets.
- **Match**: URL pattern, CSS selector, script-origin pattern, or
  fingerprint kind tag, depending on action.
- Up/down buttons reorder within the row's `(scope, action)` bucket —
  rules evaluate first-match-wins per action, so bucket order is what
  matters. Cross-bucket ordering is meaningless (DNR priority handles
  allow-over-block; other actions are orthogonal).

A filter bar above the table narrows by scope, action, or substring.
`+ Add rule` at the bottom appends a row with defaults (Global / Block /
empty); fill in the Match cell to activate it.

Below the table: a **How Hush works** section with detailed notes on
each layer, pattern syntax, and runtime order of operations.

At the bottom: an **Advanced: edit raw JSON** section for bulk edits or
copy-paste between machines. The UI and the JSON view edit the same
storage.

### Config format

Under the covers, rules are stored keyed by scope. The `__global__`
entry applies to every tab; any hostname key applies to that hostname
and its subdomains. Each scope has seven optional arrays (one per
action). Rule entries are `{ "value": ..., "disabled": ..., "tags":
[...], "comment": "..." }`; a bare string still parses as a
value-only entry.

```json
{
  "__global__": {
    "block": [{"value": "||doubleclick.net"}]
  },
  "example.com": {
    "block":   [{"value": "||ads.example.com"}],
    "allow":   [{"value": "||ads.example.com/partner/"}],
    "neuter":  [{"value": "||hotjar.com"}],
    "silence": [{"value": "||replay-vendor.example/api/"}],
    "remove":  [{"value": ".modal-overlay"}],
    "hide":    [{"value": ".popup"}, {"value": "[class*=\"AdBanner\"]"}],
    "spoof":   [{"value": "webgl-unmasked"}]
  }
}
```

- `block` — uBlock-style URL patterns (`||domain.com`,
  `*.cdn.example.com`, path wildcards, etc.). Rules are keyed by scope
  in your config but applied as **global URL-pattern matches** at the
  network layer. A rule under `reddit.com` blocks its target URL
  wherever that URL is requested — including from embedded third-party
  iframes loaded inside Reddit. Chrome's DNR `initiatorDomains` condition
  only matches the initiating frame's origin, which would fail for
  cross-origin iframe traffic, so we don't use it. For a URL blocked
  only on a specific site, make the pattern itself more specific.
- `allow` — uBlock-style URL patterns (for network) or CSS selectors
  (for DOM). A matching URL passes DNR even if a broader `block` rule
  would cover it; a matching selector excludes nodes from the Remove
  and Hide passes.
- `neuter` — script-origin URL patterns. Interaction-event
  `addEventListener` calls from matching script origins are silently
  denied at `document_start`.
- `silence` — script-origin URL patterns. Outbound fetch/XHR/beacon
  calls from matching script origins are intercepted and fake-succeeded.
- `remove` — CSS selectors. Matching elements are physically deleted
  from the DOM. Applied per-frame when the frame's hostname matches the
  scope.
- `hide` — CSS selectors. Matching elements get `display: none
  !important` via a user stylesheet. Per-frame application, same as
  `remove`.
- `spoof` — kind tags identifying fingerprint signals to neutralize.
  Currently supported: `webgl-unmasked`. Applied via main-world hook;
  the site's JS still sees a string (not a thrown error), just a
  uselessly-bland one.

Your personal config lives in `chrome.storage.local` and never lands on disk
as text. The seeded `sites.json` is a generic example only.

## Popup (per-tab activity)

Clicking the toolbar icon opens a compact popup showing, for the current tab:

- Matched site (or "no config matched" if none applies)
- Counts and per-pattern detail for each action layer
- An expandable list of every blocked URL with timestamp and resource type
- An expandable list of every removed element with tag + class signature,
  distinguishing attributes (`name`, `data-testid`, `post-title`, etc.), and a
  short text-content preview — enough context to see *what* was killed, not
  just that something was killed
- Badge on the icon:
    - **Yellow `!`** when the current tab has pending behavioral suggestions (needs your review)
    - **Grey `N`** when no suggestions pending but Hush is actively blocking, removing, or hiding things
    - No badge when nothing is happening

The popup footer has:

- **Open options** — jump to the config editor
- **Reload tab** — apply config changes to the current page
- **Debug** — copy a JSON snapshot of the extension's state (config, dynamic
  rules, tab stats, recent logs) to the clipboard for troubleshooting

## Architecture

See [docs/architecture.md](docs/architecture.md) for the rule model,
runtime data flow, rule lifecycle, detector-to-rule pipeline, and the
per-file tour of the codebase. The short version: all logic lives in
Rust compiled to WASM (`src/`). The JS shims are 18–34 line bootstraps
(`background.js`, `content.js`, `popup.js`, `options.js`) plus
`mainworld.js` (main-world hook stubs, which can't load WASM because
strict-CSP pages block WebAssembly compilation in content-script
context).

## Behavioral suggestions (opt-in)

Hush can observe what a page is doing and suggest rules to kill anti-user behavior
that curated blocklists (uBlock Origin, AdGuard, Privacy Badger) can't catch because
it's site-specific. **Off by default** — the user opts in knowing there's a small
per-scan CPU cost.

Enable the feature at the bottom of the options page. On the next tab reload, Hush
begins scanning. Findings appear in the popup's **Suggestions** panel with inline
`+ Add` / `Dismiss` / `Evidence` buttons. Adding a suggestion writes it straight to
the matched site's config. Nothing is applied automatically.

### Signals used

All observations come from APIs the browser already exposes. No filter list is
fetched; no observations leave the machine.

- **`sendBeacon` targets** — `navigator.sendBeacon` is purpose-built for telemetry;
  any third-party host receiving beacons gets a block suggestion.
- **Tracking pixels** — third-party `<img>` responses smaller than 200 bytes are
  the classic 1x1 pixel-tracker pattern.
- **First-party telemetry subdomains** — subdomains of the current site whose
  observed responses are all tiny (median < 1 KB) are almost always internal
  tracking/widget endpoints that public lists can't know about.
- **Polling endpoints** — the same canonical URL (with noise query params stripped)
  fetched four or more times within seconds, with tiny responses.
- **Hidden iframes** — iframes with `display:none`, `visibility:hidden`, 1x1 size,
  opacity 0, or positioned off-screen get a remove suggestion. Known-legit
  hidden iframes are filtered out automatically (see allowlist below).
- **Sticky overlays** — fixed/sticky-position elements with z-index ≥ 100 covering
  ≥ 25 % of the viewport get a hide suggestion.
- **Canvas fingerprinting** — `HTMLCanvasElement.toDataURL` / `toBlob` /
  `getImageData` hooks. 3+ calls from one script origin produces a block
  suggestion for that script. Confidence 90.
- **WebGL fingerprinting** — `WebGLRenderingContext.getParameter` hook.
  Reading `UNMASKED_RENDERER_WEBGL` or `UNMASKED_VENDOR_WEBGL` (the
  GPU-model-identifying parameters) produces a block suggestion at
  confidence 95. General getParameter flurry (8+) at confidence 75.
- **Audio fingerprinting** — any `OfflineAudioContext` construction is
  flagged (it has essentially no legitimate non-fingerprinting use).
  Confidence 90.
- **Font enumeration** — `measureText` with 20+ distinct font families in
  one session indicates installed-font probing. Confidence 85.
- **Session replay tools** — known vendor globals polled (`window._hjSettings`,
  `window.FS`, `window.clarity`, `window.LogRocket`, `window.smartlook`,
  `window.mouseflow`, `window.__posthog`). Presence produces a block
  suggestion for the vendor's canonical domain at confidence 95.
- **Session replay listener density** — 12+ `mousemove`/`keydown`/`click`/
  `scroll`/etc listeners attached to document/window/body in the first
  minute from a single script origin suggests replay-style capture.
  Confidence 80.
- **Invisible animation loop** — hot 2D canvas draw ops (`fillRect`,
  `drawImage`, `fill`, `stroke`, etc.) are hooked and sample the target
  canvas's visibility (viewport intersection + `display:none` /
  `visibility:hidden` / `opacity:0` / sub-2px dimensions) at most once per
  100ms per canvas. If one script origin sustains 20+ invisible-canvas
  draws over a window of at least 3 seconds with a >= 80% invisibility
  ratio, a block suggestion is emitted. Catches Lottie-style animations
  running inside collapsed panels or hidden widgets. Confidence 70.

Each suggestion carries a confidence score (sendBeacon = 95, pixels = 85, polling =
75, first-party telemetry = 70, sticky overlays = 55) and lists the raw evidence
(URLs, sizes, timestamps, outerHTML snippets) so the user can verify before
accepting.

### Hidden-iframe allowlist

Many legitimate features run in hidden iframes by design — captcha challenges,
OAuth popups, payment processor widgets. Hush skips these automatically so
they never surface as remove suggestions. Current allowlist:

- **Captcha:** `google.com/recaptcha`, `gstatic.com/recaptcha`, `hcaptcha.com`,
  `challenges.cloudflare.com`, `turnstile.cloudflare.com`
- **Payment:** `stripe.com`, `paypal.com`, `paypalobjects.com`, `braintreegateway.com`,
  `braintree-api.com`, `adyen.com`, `squareup.com`, `squarecdn.com`
- **OAuth / auth:** `accounts.google.com`, `appleid.apple.com`,
  `login.microsoftonline.com`, `login.live.com`, `*.firebaseapp.com`, `auth0.com`, `okta.com`

If Hush ever does surface a suggestion you believe is legit (a new
captcha provider we haven't allowlisted, a real hidden widget you use,
etc.), you have two options:

- **Dismiss** - per-tab-session only. A fresh page load restarts detection
  and the suggestion comes back.
- **Allow** - permanent. The suggestion key is written to
  `allowlist.suggestions` and filtered out on every site until you
  remove it from the Suggestion allowlist editor on the options page.
  Use this for false positives you never want to see again.

Every suggestion carries both buttons regardless of its layer (block,
remove, or hide) or detection tier (fingerprinting, session replay,
animation loop, etc.).

### Scan timing

When the feature is on, Hush scans:

- Once at `DOMContentLoaded`
- Once 5 s after load (SPAs deferred-load)
- On explicit **Rescan now** click in the popup

When the feature is off, the detector code is not installed. No listeners, no
timers, no DOM walks. The **Scan this tab now** button in the popup's Suggestions
panel runs a single one-shot scan without enabling the feature — useful for ad-hoc
inspection.

### Badge signal

When any tab has pending suggestions, the Hush toolbar icon shows a **yellow `!`**
badge on that tab. Clicking it opens the popup with the Suggestions panel ready to
review. After you've accepted or dismissed everything, the badge reverts to the
grey activity count (or disappears if no activity).

### Deep inspection via main-world hooks

When behavioral suggestions are enabled, Hush additionally injects a small
script into the page's own JavaScript context (via `content_scripts` with
`"world": "MAIN"`) that monkey-patches:

- `window.fetch`
- `XMLHttpRequest.prototype.open` and `.send`
- `navigator.sendBeacon`
- `WebSocket.prototype.send`

For every call the hook captures the URL, method, a truncated body preview,
and a short JS stack trace (top 6 frames). These observations are forwarded
to the background service worker and attached to the per-tab behavior
state. This gives Hush answers to questions the Resource Timing API alone
can't provide:

- "Which script fired this beacon?" (stack trace shows the calling file)
- "What data was sent in that POST body?" (body preview)
- "What messages are being streamed over this WebSocket?" (send hook)

The hook passes through to the original API immediately, so site behavior
is unchanged. If a hook fails to install or dispatch (strict CSP, etc.),
the network request still goes through normally.

Hooks also run inside cross-origin iframes (`all_frames: true`), which
means ad-iframe internal telemetry is visible too. This is the main way
to see what an ad iframe is actually exfiltrating to its own servers.

### Privacy posture

- No network calls from Hush for detection (no filter-list fetch, no telemetry
  back to anyone)
- Behavioral state is stored only in `chrome.storage.session` (cleared on browser
  restart) and is scoped per-tab
- Tab's behavioral state is wiped on full-page navigation (`webNavigation.onCommitted`)

## Debug logging

Off by default. Toggle **Enable verbose console logging** in options to turn
on. When on:

- Content script writes `[Hush] ...` entries to the page's DevTools console.
- Background service worker writes `[Hush bg] ...` entries to its own console
  (open via `chrome://extensions/` → Hush → "service worker" link).
- All entries (both content and background) are captured in an in-memory ring
  buffer of the last 300 log lines, accessible via the popup's **Debug** button.

Errors are always logged regardless of the toggle.

## How the action layers interact

- **Allow over Block:** An `allow` URL pattern overrides a broader
  `block` via DNR priority. An `allow` selector excludes matching
  nodes from Remove/Hide passes. Use it to carve a narrow exception
  out of a blanket rule.

- **Neuter / Silence vs. Block:** Block kills the request before
  connection. Neuter kills the `addEventListener` registration at
  `document_start` so replay vendors never get a chance to capture.
  Silence intercepts the outbound fetch/XHR/beacon call after it's
  made but before the network write. Pick Neuter when you can match
  the vendor's script origin cleanly; Silence when the capture is
  bundled into the site's own JS.

- **Remove and Hide on the same selector:** Remove runs first on
  every MutationObserver pass. By the time the hide-counter looks,
  matching elements are gone. The popup shows such selectors as
  `- (removed)` in the Hidden section.

- **Block plus Hide/Remove:** the request is killed at the network
  layer; if a site's JS still creates a DOM element whose src just
  failed (a dead iframe, for example), Hide or Remove cleans up
  the shell.

- **Runtime order for any given element:**
  1. Block / Allow decide whether the request exists at all.
  2. Neuter decides whether capture listeners ever register.
  3. Silence intercepts any capture that still made it to the wire.
  4. Remove decides whether the element gets to stay in the DOM.
  5. Hide decides whether what remains actually renders.
  6. Spoof returns bland values from fingerprinting reads.

## Limitations

- Network blocking relies on `declarativeNetRequest` dynamic rules under MV3.
  Chrome caps the total number of dynamic rules (currently 30000 site-wide);
  unlikely to matter for personal use.

- `chrome.declarativeNetRequest.onRuleMatchedDebug` only fires for unpacked
  extensions. That's how the popup counts blocked requests. If you ever pack
  Hush as a CRX, the block layer still works but the per-URL evidence
  disappears.

- Content scripts run with `<all_urls>` host permissions so the options UI
  can apply to any site you configure. On sites with no config entry the
  content script exits within a few microseconds.

- Changes to `block` rules take effect at the next request. Changes to `hide`
  and `remove` apply when the tab is reloaded.

- Domain matching is exact-or-suffix on the hostname. There's no regex or
  path matching for the domain-key match itself; use `block` patterns if you
  need URL-path filtering.

## Case studies

Worked examples with full rule sets and reasoning live under [`docs/`](docs/README.md).

- [Reddit](docs/reddit.md) - telemetry beacons, Brand Affiliate posts, algorithmic community recs, sidebar widgets
- [Amazon](docs/amazon.md) - homepage ad iframes (narrow scope, observed-only)
- [GitHub](docs/github.md) - first-party sendBeacon telemetry collector (`collector.github.com`)

## How Hush compares to other blockers

Honest side-by-side with uBlock Origin, Privacy Badger, Ghostery, Brave
Shields, NoScript, and others lives in
[docs/comparison.md](docs/comparison.md). Short version: Hush is not a
replacement for uBlock Origin — it's a per-site surgical tool that
catches first-party telemetry subdomains, site-specific custom elements,
session-replay listener density, and fingerprint-API reads that curated
filter lists can't see. Use it alongside a general blocker.

## License

GPL-3.0-or-later. See [LICENSE](../../LICENSE) at the repo root.
