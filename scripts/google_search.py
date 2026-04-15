#!/usr/bin/env python3
"""
Google search via a real visible Chrome window, driven by Selenium.

Usage:
    python google_search.py "my query"
    python google_search.py "my query" --num 20
    python google_search.py "my query" --json
    python google_search.py "my query" --keep-open
    python google_search.py "my query" --profile "C:/path/to/profile"

Plain text is the default output. Use --json for machine-readable output.

Exit codes:
    0  success
    1  driver/fetch failure
    2  google served captcha (/sorry/)
    3  results selector did not match within timeout
    4  selenium not installed
    130 KeyboardInterrupt

Dependency:
    pip install selenium

ChromeDriver is fetched automatically on first run by Selenium Manager; no
separate install step is required.
"""

import argparse
import json
import sys


def _import_selenium():
    try:
        from selenium import webdriver
        from selenium.webdriver.chrome.options import Options
        from selenium.webdriver.common.by import By
        from selenium.webdriver.support import expected_conditions as EC
        from selenium.webdriver.support.ui import WebDriverWait
        from selenium.common.exceptions import (
            TimeoutException,
            WebDriverException,
            SessionNotCreatedException,
        )
        return {
            "webdriver": webdriver, "Options": Options, "By": By, "EC": EC,
            "WebDriverWait": WebDriverWait,
            "TimeoutException": TimeoutException,
            "WebDriverException": WebDriverException,
            "SessionNotCreatedException": SessionNotCreatedException,
        }
    except ImportError:
        print(
            "selenium not installed. run:\n    pip install selenium",
            file=sys.stderr,
        )
        sys.exit(4)


def build_options(sel, profile: str | None):
    opts = sel["Options"]()
    opts.add_argument("--lang=en-US")
    opts.add_argument("--disable-blink-features=AutomationControlled")
    opts.add_experimental_option("excludeSwitches", ["enable-automation"])
    opts.add_experimental_option("useAutomationExtension", False)
    if profile:
        opts.add_argument(f"--user-data-dir={profile}")
    return opts


def dismiss_consent(sel, driver):
    """Click 'Accept all' / 'I agree' if a consent banner appears. Silent on miss."""
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
        btn = WebDriverWait(driver, 3).until(
            EC.element_to_be_clickable((By.XPATH, xpath))
        )
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

        # Snippet: text of the full result wrapper minus the title/URL noise.
        # Modern SERP wraps each organic hit in div.MjjYud; fall back to any ancestor div.
        snippet = ""
        try:
            container = a.find_element(
                By.XPATH,
                "./ancestor::div[contains(@class,'MjjYud') or contains(@class,'N54PNb')][1]",
            )
            full = container.text or ""
            lines = [ln.strip() for ln in full.splitlines() if ln.strip()]
            # Drop lines that are the title, a URL breadcrumb, or boilerplate
            skip = {"Web results", title}
            filtered = []
            for ln in lines:
                if ln in skip:
                    continue
                if ln.startswith("http://") or ln.startswith("https://"):
                    continue
                # URL breadcrumb lines contain the domain with "›" separators
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
    ap = argparse.ArgumentParser(description="Google search via visible Chrome (Selenium)")
    ap.add_argument("query", help="search query")
    ap.add_argument("--num", type=int, default=10, help="max results (default 10)")
    ap.add_argument("--json", action="store_true", help="output as JSON")
    ap.add_argument("--keep-open", action="store_true",
                    help="leave the browser open; script waits for Enter before closing")
    ap.add_argument("--profile", default=None,
                    help="path to a persistent Chrome user-data-dir (optional)")
    args = ap.parse_args()

    sel = _import_selenium()
    opts = build_options(sel, args.profile)

    driver = None
    try:
        driver = sel["webdriver"].Chrome(options=opts)
    except sel["SessionNotCreatedException"] as e:
        print(f"chrome/driver mismatch: {e}", file=sys.stderr)
        sys.exit(1)
    except sel["WebDriverException"] as e:
        print(f"failed to start chrome: {e}", file=sys.stderr)
        sys.exit(1)

    try:
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

        if args.keep_open:
            try:
                input("Press Enter to close the browser... ")
            except EOFError:
                pass

    except KeyboardInterrupt:
        print("interrupted", file=sys.stderr)
        sys.exit(130)
    except sel["WebDriverException"] as e:
        print(f"webdriver error: {e}", file=sys.stderr)
        sys.exit(1)
    finally:
        if driver is not None:
            try:
                driver.quit()
            except Exception:
                pass


if __name__ == "__main__":
    main()
