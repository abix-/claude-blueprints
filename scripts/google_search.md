# google_search.py

Google search via a persistent Chrome window, driven by Selenium. Built as a
fallback for when the built-in `WebSearch` tool is unavailable (e.g. AWS
Bedrock provider) or returns nothing useful.

## How it works

1. If Chrome isn't already listening on `127.0.0.1:9222`, launch it with a
   dedicated profile at `~/.cache/claude-google-search/profile` and wait up
   to 10s for the port to open. Chrome's initial new-tab page is left
   untouched — the script never uses or closes it.
2. Attach Selenium via `debugger_address`.
3. Open a fresh tab, navigate to `https://www.google.com/search?q=<query>`,
   dismiss the consent banner if present, wait for results to render.
4. Parse the DOM: `h3` + parent anchor, walk up to the `div.MjjYud` result
   wrapper, extract title, url, and a best-effort snippet.
5. Print plain text (default) or JSON (`--json`) to stdout.
6. Close the tab that was opened in step 3.

~5-6s per call on cold Chrome, ~2-3s on warm Chrome.

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

- Python 3.8+.
- `pip install selenium` (Selenium 4.6+). ChromeDriver is auto-downloaded
  by Selenium Manager on first run and cached under `~/.cache/selenium`.
- Google Chrome. Path defaults to
  `C:\Program Files\Google\Chrome\Application\chrome.exe`; override with
  the `CHROME_EXE` environment variable if installed elsewhere.

## Profile

`~/.cache/claude-google-search/profile` — separate from your normal Chrome
profile, so logging in or accumulating cookies here doesn't pollute
everyday browsing. Google's consent banner is dismissed once and stays
dismissed because the cookie persists.

## Caveats

- Chrome enforces a single-instance lock on a user-data-dir. Two parallel
  invocations against the same profile will fail with a profile-in-use
  error. Sequential calls are fine.
- The "Chrome is being controlled by automated test software" banner is
  hidden via `--disable-blink-features=AutomationControlled` +
  `excludeSwitches=enable-automation`.
- If Chrome is killed externally, the next invocation auto-relaunches.
