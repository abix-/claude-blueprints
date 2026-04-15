#!/usr/bin/env python3
"""
Google search via a reused visible Chrome, driven by Selenium.

Launches Chrome once (with --remote-debugging-port=9222) and reuses the same
browser process across subsequent invocations. First call ~5-6s; later calls
~1-2s because Chrome is already warm.

Usage:
    python google_search.py "my query"
    python google_search.py "my query" --num 20
    python google_search.py "my query" --json

Exit codes:
    0  success
    1  driver/chrome failure
    2  google served captcha (/sorry/)
    3  results selector did not match within timeout
    4  selenium not installed
    130 KeyboardInterrupt
"""

import argparse
import json
import os
import socket
import subprocess
import sys
import time


DEBUG_PORT = 9222
DEFAULT_PROFILE = os.path.expanduser("~/.cache/claude-google-search/profile")
CHROME_EXE = os.environ.get("CHROME_EXE", r"C:\Program Files\Google\Chrome\Application\chrome.exe")


def _import_selenium():
    try:
        from selenium import webdriver
        from selenium.webdriver.chrome.options import Options
        from selenium.webdriver.common.by import By
        from selenium.webdriver.support import expected_conditions as EC
        from selenium.webdriver.support.ui import WebDriverWait
        from selenium.common.exceptions import TimeoutException, WebDriverException
        return {
            "webdriver": webdriver, "Options": Options, "By": By, "EC": EC,
            "WebDriverWait": WebDriverWait,
            "TimeoutException": TimeoutException,
            "WebDriverException": WebDriverException,
        }
    except ImportError:
        print("selenium not installed. run:\n    pip install selenium", file=sys.stderr)
        sys.exit(4)


def port_open(port: int, timeout: float = 0.3) -> bool:
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=timeout):
            return True
    except OSError:
        return False


def ensure_chrome_running(profile: str) -> bool:
    """Returns True if we had to launch Chrome, False if it was already running."""
    if port_open(DEBUG_PORT):
        return False
    os.makedirs(profile, exist_ok=True)
    if not os.path.exists(CHROME_EXE):
        print(f"chrome not found at {CHROME_EXE} (set CHROME_EXE env var to override)", file=sys.stderr)
        sys.exit(1)
    args = [
        CHROME_EXE,
        f"--remote-debugging-port={DEBUG_PORT}",
        f"--user-data-dir={profile}",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-blink-features=AutomationControlled",
        "--lang=en-US",
    ]
    creationflags = 0
    if sys.platform == "win32":
        creationflags = subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP
    subprocess.Popen(
        args,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, stdin=subprocess.DEVNULL,
        close_fds=True, creationflags=creationflags,
    )
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if port_open(DEBUG_PORT):
            return True
        time.sleep(0.1)
    print(f"chrome did not open port {DEBUG_PORT} within 10s", file=sys.stderr)
    sys.exit(1)


def dismiss_consent(sel, driver):
    By = sel["By"]
    WebDriverWait = sel["WebDriverWait"]
    EC = sel["EC"]
    TimeoutException = sel["TimeoutException"]
    xpath = (
        "//button["
        ".//div[contains(., 'Accept all') or contains(., 'I agree')] "
        "or contains(., 'Accept all') "
        "or contains(., 'I agree')"
        "]"
    )
    try:
        btn = WebDriverWait(driver, 1).until(EC.element_to_be_clickable((By.XPATH, xpath)))
        btn.click()
    except TimeoutException:
        pass


def extract_results(sel, driver, max_num: int):
    By = sel["By"]
    anchors = driver.find_elements(By.CSS_SELECTOR, "div#search a:has(h3)")
    seen = set()
    results = []
    for a in anchors:
        href = a.get_attribute("href") or ""
        if not href.startswith("http"):
            continue
        if any(bad in href for bad in (
            "google.com/search", "google.com/url", "webcache.googleusercontent",
            "accounts.google.", "support.google.",
        )):
            continue
        if href in seen:
            continue
        seen.add(href)

        try:
            title = a.find_element(By.CSS_SELECTOR, "h3").text.strip()
        except Exception:
            title = ""
        if not title:
            continue

        snippet = ""
        try:
            container = a.find_element(
                By.XPATH,
                "./ancestor::div[contains(@class,'MjjYud') or contains(@class,'N54PNb')][1]",
            )
            full = container.text or ""
            lines = [ln.strip() for ln in full.splitlines() if ln.strip()]
            skip = {"Web results", title}
            filtered = []
            for ln in lines:
                if ln in skip:
                    continue
                if ln.startswith("http://") or ln.startswith("https://"):
                    continue
                if " > " in ln or chr(0x203A) in ln:
                    continue
                filtered.append(ln)
            snippet = " ".join(filtered)[:400]
        except Exception:
            pass

        results.append({"title": title, "url": href, "snippet": snippet})
        if len(results) >= max_num:
            break

    return results


def main():
    ap = argparse.ArgumentParser(description="Google search via reused visible Chrome")
    ap.add_argument("query", help="search query")
    ap.add_argument("--num", type=int, default=10, help="max results (default 10)")
    ap.add_argument("--json", action="store_true", help="output as JSON")
    ap.add_argument("--profile", default=DEFAULT_PROFILE,
                    help=f"user-data-dir for auto-launched Chrome (default: {DEFAULT_PROFILE})")
    args = ap.parse_args()

    sel = _import_selenium()
    ensure_chrome_running(args.profile)

    opts = sel["Options"]()
    opts.debugger_address = f"127.0.0.1:{DEBUG_PORT}"

    driver = None
    search_handle = None
    try:
        try:
            driver = sel["webdriver"].Chrome(options=opts)
        except sel["WebDriverException"] as e:
            print(f"failed to attach to chrome on port {DEBUG_PORT}: {e}", file=sys.stderr)
            sys.exit(1)

        # Always open a fresh tab; never touch Chrome's initial new-tab page.
        driver.switch_to.new_window("tab")
        search_handle = driver.current_window_handle

        import urllib.parse
        qs = urllib.parse.urlencode({
            "q": args.query, "hl": "en", "gl": "us", "num": args.num + 2,
        })
        driver.get(f"https://www.google.com/search?{qs}")

        dismiss_consent(sel, driver)

        try:
            sel["WebDriverWait"](driver, 10).until(
                sel["EC"].presence_of_element_located(
                    (sel["By"].CSS_SELECTOR, "div#search h3")
                )
            )
        except sel["TimeoutException"]:
            url = driver.current_url
            if "/sorry/" in url:
                print(f"google served a captcha: {url}", file=sys.stderr)
                sys.exit(2)
            print(f"no results selector matched; current url: {url}", file=sys.stderr)
            sys.exit(3)

        results = extract_results(sel, driver, args.num)

        if not results:
            print("results element present but extractor returned nothing", file=sys.stderr)
            sys.exit(3)

        if args.json:
            print(json.dumps({"query": args.query, "results": results}, indent=2))
        else:
            for i, r in enumerate(results, 1):
                print(f"{i}. {r['title']}")
                print(f"   {r['url']}")
                if r["snippet"]:
                    print(f"   {r['snippet']}")
                print()

    except KeyboardInterrupt:
        print("interrupted", file=sys.stderr)
        sys.exit(130)
    except sel["WebDriverException"] as e:
        print(f"webdriver error: {e}", file=sys.stderr)
        sys.exit(1)
    finally:
        if driver is not None:
            try:
                should_close = (
                    search_handle is not None
                    and search_handle in driver.window_handles
                )
                if should_close:
                    driver.switch_to.window(search_handle)
                    driver.close()
            except Exception:
                pass
            try:
                driver.quit()
            except Exception:
                pass


if __name__ == "__main__":
    main()
