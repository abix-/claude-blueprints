# google_search.py

Google search via a persistent Chrome window, driven by Selenium. Built as a
fallback for when the built-in `WebSearch` tool is unavailable (e.g. AWS
Bedrock provider) or returns nothing useful.

## What it does

1. Checks whether Chrome is already listening on `127.0.0.1:9222` (the
   remote-debugging port).
2. If not, launches Chrome with a dedicated profile at
   `~/.cache/claude-google-search/profile` and waits up to 10s for port 9222
   to open.
3. Attaches Selenium to that Chrome via `debugger_address`.
4. Opens or reuses a tab (see "Tab behavior" below), navigates to
   `https://www.google.com/search?q=<query>`, dismisses the consent banner
   if present, waits for the results to render.
5. Parses the DOM for `h3` + parent anchor, walks up to the `div.MjjYud`
   result wrapper, extracts title, url, and a best-effort snippet.
6. Prints plain text (default) or JSON (`--json`) to stdout.

Each search closes its tab when done, so Chrome exits when the last tab
closes. Every invocation launches a fresh Chrome (~5-6s per call).

## Tab behavior

- **Cold launch** (Chrome was not running): reuse the default new-tab that
  Chrome opens at startup. Navigate it to Google. Close it when done.
  Closing the last tab causes Chrome to exit.
- **Warm call** (Chrome was already running): open a new disposable tab,
  run the search, close that tab when done. If it was the only tab, Chrome
  exits.

Every search closes its tab when done. No tabs are left behind.

## CLI

```
python google_search.py "<query>" [--num N] [--json] [--profile PATH]
```

- `query` (required): the search string.
- `--num N`: max results to return (default 10). Google's SERP typically
  returns ~10 per page; higher values request a larger page.
- `--json`: emit a single JSON object `{query, results: [{title, url, snippet}]}`
  instead of the plain-text numbered listing.
- `--profile PATH`: override the user-data-dir used when auto-launching
  Chrome. Ignored if Chrome is already running on port 9222 (you get
  whatever profile that Chrome was started with).

## Output

**Plain text** (default) — one numbered block per result, blank line between:

```
1. <title>
   <url>
   <snippet>
```

Snippet line omitted if empty. Errors go to stderr.

**JSON** (`--json`):

```json
{
  "query": "<query>",
  "results": [
    {"title": "...", "url": "https://...", "snippet": "..."}
  ]
}
```

Snippet is always a string (`""` if none extracted).

## Exit codes

- `0` — success
- `1` — Chrome launch or Selenium attach failure
- `2` — Google served a captcha / `/sorry/` redirect
- `3` — results selector did not match within the 10s timeout
- `4` — `selenium` not installed (`pip install selenium`)
- `130` — KeyboardInterrupt

## Dependencies

- Python 3.8+ (uses `str | None` annotation style).
- `pip install selenium` (Selenium 4.6+). ChromeDriver is auto-downloaded
  by Selenium Manager on first run and cached under `~/.cache/selenium`.
- Google Chrome. Path defaults to
  `C:\Program Files\Google\Chrome\Application\chrome.exe`; override with
  the `CHROME_EXE` environment variable if installed elsewhere.

## Profile location

`~/.cache/claude-google-search/profile`

Separate from your normal Chrome profile, so logging in there (or
accumulating cookies) doesn't pollute your everyday browsing. Google's
consent banner is dismissed once and stays dismissed because the cookie
persists in this profile.

## Known caveats

- Chrome enforces a single-instance lock on a user-data-dir. Two parallel
  invocations against the same profile will fail with a profile-in-use
  error. Sequential calls are fine.
- The "Chrome is being controlled by automated test software" banner is
  hidden via `--disable-blink-features=AutomationControlled` +
  `excludeSwitches=enable-automation`.
- If Chrome is killed externally, the next invocation auto-relaunches
  (the port-closed path).
