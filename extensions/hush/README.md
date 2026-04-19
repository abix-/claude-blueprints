# Hush

Per-site element hiding, DOM removal, and network blocking for Chrome (MV3).

Lets you write a small per-domain config and applies three independent layers of
defense against whatever a site is doing that you don't want: promo popups,
tracker scripts, heavy animated widgets, chat stickers, iframe-based widget
frameworks, etc.

## What it does

Three layers, in order of aggressiveness:

1. **Block (network)** — URL patterns registered with
   `chrome.declarativeNetRequest`. Matching requests are rejected by the browser
   before DNS resolution, TCP connection, or TLS handshake. No bytes reach the
   network; the initiating `fetch()` or iframe load fails locally with
   `net::ERR_BLOCKED_BY_CLIENT`.

2. **Remove (DOM)** — CSS selectors whose matching elements are physically
   removed from the DOM via `element.remove()`. A `MutationObserver` keeps
   watch and removes matching nodes that arrive later (SPA routers, infinite
   scroll, modals). Frees listeners and stops libraries that rely on the
   element being in the tree.

3. **Hide (CSS)** — CSS selectors applied with `display: none !important` via
   a user stylesheet injected at `document_start`. Skips layout, paint, and
   compositing. Mildest layer; leaves the element in the DOM and its
   JavaScript running.

Each site entry specifies any combination of the three. A site entry for
`example.com` also applies to `www.example.com`, `m.example.com`, and any
other subdomain via exact-or-suffix matching.

## Install

1. Clone or download this folder.
2. Open `chrome://extensions/` and enable **Developer mode** (top right).
3. Click **Load unpacked** and select the `hush/` folder.
4. Click the Hush toolbar icon to open the popup, then **Open options** to
   configure sites.

## Configure

The options page is a two-pane editor:

- Left: list of configured sites. Each entry shows counts for hide/remove/block.
- Right: the selected site's three layer sections. Add or delete entries inline.
  Rename or delete the whole site.

Below the editor: a **How Hush works** section with detailed notes on each
layer, pattern syntax, and the runtime order of operations.

At the bottom: an **Advanced: edit raw JSON** section for bulk edits or
copy-paste between machines. The UI and the JSON view edit the same storage.

### Config format

```json
{
  "example.com": {
    "block":  ["||ads.example.com^"],
    "remove": [".modal-overlay"],
    "hide":   [".popup", ".newsletter-signup", "[class*=\"AdBanner\"]"]
  }
}
```

Keys are domain names. Each site has three optional arrays:

- `block` — uBlock-style URL patterns (`||domain.com^`, `*.cdn.example.com^`,
  path wildcards, etc.). Scoped to the site key via `initiatorDomains` so a
  rule under `example.com` only fires when you're browsing example.com.
- `remove` — CSS selectors. Matching elements are physically deleted from the
  DOM.
- `hide` — CSS selectors. Matching elements get `display: none !important`
  via a user stylesheet.

Your personal config lives in `chrome.storage.local` and never lands on disk
as text. The seeded `sites.json` is a generic example only.

## Popup (per-tab activity)

Clicking the toolbar icon opens a compact popup showing, for the current tab:

- Matched site (or "no config matched" if none applies)
- Counts and per-pattern detail for each of the three layers
- An expandable list of every blocked URL with timestamp and resource type
- An expandable list of every removed element with tag + class signature
- Badge on the icon with the running total

The popup footer has:

- **Open options** — jump to the config editor
- **Reload tab** — apply config changes to the current page
- **Debug** — copy a JSON snapshot of the extension's state (config, dynamic
  rules, tab stats, recent logs) to the clipboard for troubleshooting

## Architecture

```
hush/
  manifest.json     MV3 manifest. Permissions: declarativeNetRequest,
                    declarativeNetRequestFeedback, storage, webNavigation,
                    host_permissions <all_urls>.
  background.js     Service worker. Loads config, registers dynamic DNR rules,
                    maintains a per-tab activity stats Map, listens for
                    onRuleMatchedDebug to count blocked requests, serves
                    stats/debug info to the popup. Stats are persisted to
                    chrome.storage.session so they survive SW idle restarts.
  content.js        Runs at document_start on every page. Finds the matching
                    site config, applies remove + hide layers, reports stats
                    back to background.js via runtime messages. Uses a single
                    MutationObserver for both layers.
  options.html      Two-pane editor + three-layer explainer + raw JSON
                    escape hatch.
  options.js        UI logic for the options page.
  popup.html        Per-tab activity view + debug button.
  popup.js          UI logic for the popup; queries background.js for stats.
  sites.json        Seed config bundled with the extension. Generic example
                    only; loaded into chrome.storage.local on first install.
  README.md         This file.
```

## Debug logging

Off by default. Toggle **Enable verbose console logging** in options to turn
on. When on:

- Content script writes `[Hush] ...` entries to the page's DevTools console.
- Background service worker writes `[Hush bg] ...` entries to its own console
  (open via `chrome://extensions/` → Hush → "service worker" link).
- All entries (both content and background) are captured in an in-memory ring
  buffer of the last 300 log lines, accessible via the popup's **Debug** button.

Errors are always logged regardless of the toggle.

## How the three layers interact

- **Remove and Hide on the same selector:** Remove runs first on every
  MutationObserver pass. By the time the hide-counter looks, matching
  elements are gone. The popup shows such selectors as `- (removed)` in the
  Hidden section.

- **Block plus Hide/Remove:** the request is killed at the network layer;
  if a site's JS still creates a DOM element whose src just failed
  (a dead iframe, for example), Hide or Remove cleans up the shell.

- **Runtime order for any given element:**
  1. Block decides whether the request exists at all.
  2. Remove decides whether the element gets to stay in the DOM.
  3. Hide decides whether what remains actually renders.

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

## License

MIT.
