#!/usr/bin/env python3
"""
Chrome CPU profiler via Selenium + CDP.

Attaches to a running Chrome started with --remote-debugging-port=9222,
picks a tab, records a CPU profile for N seconds, and prints the top
functions by self time with file:line locations.

Launch Chrome first (close all existing Chrome windows first):

    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe" ^
        --remote-debugging-port=9222 ^
        --user-data-dir="C:\\Users\\Abix\\AppData\\Local\\Google\\Chrome\\User Data"

Or point --user-data-dir at a scratch folder if you don't want to touch
your real profile; then open the problem site in that new window.

Usage:
    python chrome_cpu_profile.py                     # list tabs
    python chrome_cpu_profile.py --match youtube     # profile first tab matching substring
    python chrome_cpu_profile.py --index 2           # profile tab #2 from the list
    python chrome_cpu_profile.py --match foo --seconds 15 --top 30 --raw out.cpuprofile
"""

import argparse
import json
import socket
import sys
import time
from collections import defaultdict

DEBUG_PORT = 9222


def _import_selenium():
    try:
        from selenium import webdriver
        from selenium.webdriver.chrome.options import Options
        from selenium.common.exceptions import WebDriverException
        return {
            "webdriver": webdriver,
            "Options": Options,
            "WebDriverException": WebDriverException,
        }
    except ImportError:
        print("selenium not installed. run:\n    pip install selenium", file=sys.stderr)
        sys.exit(4)


def port_open(port, timeout=0.3):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=timeout):
            return True
    except OSError:
        return False


def connect(sel):
    if not port_open(DEBUG_PORT):
        print(
            f"chrome is not listening on port {DEBUG_PORT}.\n"
            f"close all chrome windows, then launch chrome with:\n"
            f'  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe" '
            f"--remote-debugging-port={DEBUG_PORT} "
            f'--user-data-dir="C:\\Users\\Abix\\AppData\\Local\\Google\\Chrome\\User Data"',
            file=sys.stderr,
        )
        sys.exit(1)
    opts = sel["Options"]()
    opts.debugger_address = f"127.0.0.1:{DEBUG_PORT}"
    try:
        return sel["webdriver"].Chrome(options=opts)
    except sel["WebDriverException"] as e:
        print(f"failed to attach: {e}", file=sys.stderr)
        sys.exit(1)


def list_tabs(driver):
    tabs = []
    for h in driver.window_handles:
        try:
            driver.switch_to.window(h)
            tabs.append({"handle": h, "title": driver.title, "url": driver.current_url})
        except Exception as e:
            tabs.append({"handle": h, "title": f"<unreadable: {e}>", "url": ""})
    return tabs


def pick_tab(tabs, match=None, index=None):
    if index is not None:
        if index < 1 or index > len(tabs):
            print(f"tab index {index} out of range (1..{len(tabs)})", file=sys.stderr)
            sys.exit(1)
        return tabs[index - 1]
    if match:
        lower = match.lower()
        for t in tabs:
            if lower in (t["title"] or "").lower() or lower in (t["url"] or "").lower():
                return t
        print(f"no tab matched {match!r}", file=sys.stderr)
        sys.exit(1)
    return None


def show_tabs(tabs):
    print("tabs:")
    for i, t in enumerate(tabs, 1):
        title = (t["title"] or "").strip() or "(untitled)"
        print(f"  {i}. {title[:80]}")
        print(f"     {(t['url'] or '')[:120]}")


def profile(driver, seconds):
    driver.execute_cdp_cmd("Profiler.enable", {})
    driver.execute_cdp_cmd("Profiler.setSamplingInterval", {"interval": 200})
    driver.execute_cdp_cmd("Profiler.start", {})
    print(f"profiling {seconds}s...", file=sys.stderr)
    time.sleep(seconds)
    result = driver.execute_cdp_cmd("Profiler.stop", {})
    return result.get("profile", {})


def analyze(prof, top_n=25):
    nodes = prof.get("nodes", [])
    samples = prof.get("samples", []) or []
    deltas = prof.get("timeDeltas", []) or []
    start = prof.get("startTime", 0)
    end = prof.get("endTime", 0)

    if not nodes or not samples:
        print("profile was empty (no samples captured)", file=sys.stderr)
        return

    by_id = {n["id"]: n for n in nodes}
    self_us_by_id = defaultdict(int)
    for d, sid in zip(deltas, samples):
        self_us_by_id[sid] += d

    agg = defaultdict(int)
    for nid, us in self_us_by_id.items():
        n = by_id.get(nid)
        if not n:
            continue
        cf = n.get("callFrame", {})
        key = (
            cf.get("functionName") or "(anonymous)",
            cf.get("url") or "",
            cf.get("lineNumber", -1),
            cf.get("columnNumber", -1),
        )
        agg[key] += us

    total_self = sum(agg.values()) or 1
    wall_ms = (end - start) / 1000.0
    cpu_ms = total_self / 1000.0
    pct_core = 100.0 * cpu_ms / wall_ms if wall_ms else 0.0

    print()
    print(f"profile: {wall_ms:.0f} ms wall, {cpu_ms:.0f} ms CPU ({pct_core:.0f}% of one core)")
    print()
    print(f"  {'%':>6}  {'self_ms':>8}  function / location")
    print(f"  {'-'*6}  {'-'*8}  {'-'*90}")

    rows = sorted(agg.items(), key=lambda kv: kv[1], reverse=True)[:top_n]
    for (fn, url, line, col), us in rows:
        pct = 100.0 * us / total_self
        if url:
            short = url.rsplit("/", 1)[-1][:60]
            loc = f"[{short}:{line + 1}:{col + 1}]"
        else:
            loc = "[native]"
        print(f"  {pct:>5.1f}%  {us/1000:>7.1f}   {fn[:50]:<50} {loc}")


def main():
    ap = argparse.ArgumentParser(description="Chrome CPU profiler via Selenium + CDP")
    ap.add_argument("--match", help="pick tab whose title or url contains this substring")
    ap.add_argument("--index", type=int, help="pick tab by 1-based index from the list")
    ap.add_argument("--seconds", type=int, default=10, help="profile duration in seconds")
    ap.add_argument("--top", type=int, default=25, help="show top N functions")
    ap.add_argument("--raw", help="write raw .cpuprofile JSON here for further analysis")
    args = ap.parse_args()

    sel = _import_selenium()
    driver = connect(sel)

    try:
        tabs = list_tabs(driver)
        if not tabs:
            print("no tabs found", file=sys.stderr)
            sys.exit(1)

        if args.match is None and args.index is None:
            show_tabs(tabs)
            print()
            print("rerun with --match <substring> or --index <n> to profile a tab")
            return

        tab = pick_tab(tabs, match=args.match, index=args.index)
        driver.switch_to.window(tab["handle"])
        print(f"profiling: {tab['title']!r}", file=sys.stderr)
        print(f"  url: {tab['url']}", file=sys.stderr)

        prof = profile(driver, args.seconds)

        if args.raw:
            with open(args.raw, "w") as f:
                json.dump(prof, f)
            print(f"raw profile written to {args.raw}", file=sys.stderr)

        analyze(prof, top_n=args.top)

    finally:
        try:
            driver.quit()
        except Exception:
            pass


if __name__ == "__main__":
    main()
