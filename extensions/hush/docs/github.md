# GitHub rules - case study

This doc documents rules observed and verified in Hush's suggestion output
while browsing `github.com` (the web UI, not the API). Extend as you
encounter more patterns.

## What was observed

Browsing `github.com` with behavioral suggestions enabled surfaced a
**sendBeacon target** suggestion after roughly a minute of normal use
(clicking between repos, scrolling a feed):

- **Suggested rule:** `||collector.github.com`
- **Layer:** Block
- **Confidence:** 95
- **Reason:** sendBeacon target (12 beacons sent)
- **Source frame:** `github.com` (top frame, not an iframe)

Inspecting the captured evidence in the popup's Evidence panel, every
observed beacon target was the same endpoint:

```
https://collector.github.com/github/collect
https://collector.github.com/github/collect
https://collector.github.com/github/collect
https://collector.github.com/github/collect
https://collector.github.com/github/collect
```

All twelve calls went through `navigator.sendBeacon()` from the main
`github.com` frame. The dedup diagnostic ("Why?") confirmed no existing
block rule matched and the tab had no site config yet.

## Why it's worth blocking

`navigator.sendBeacon()` is a purpose-built browser API for fire-and-forget
telemetry. Its defining property is that it's delivered even while the
page is unloading - the browser hands the request off to the network
service and commits to sending it whether the tab is closed, the user
navigates away, or the process is torn down. There is **no legitimate
non-tracking use case** at scale. It exists to transmit analytics without
the reliability problems of `fetch()` during page teardown.

`collector.github.com` is GitHub's internal-only telemetry endpoint, which
is why no curated filter list (EasyPrivacy, Disconnect, AdGuard) blocks it
- the third-party lists target cross-site trackers, not first-party-owned
analytics subdomains. That is exactly the gap Hush's behavioral detector
is designed to close.

## Suggested rule

```json
{
  "github.com": {
    "block": ["||collector.github.com"]
  }
}
```

The `||` prefix is Chrome DNR's domain anchor - it matches any scheme and
any subdomain prefix (so it also handles `https://collector.github.com`,
`http://collector.github.com`, and any future scheme rewrites). The rule
is declared under `github.com` in the site config, but because Hush v0.4.0
dropped DNR `initiatorDomains` restrictions, the pattern fires globally -
if any other tab ever makes a request to `collector.github.com`, it's
blocked there too. This is intentional: GitHub's telemetry collector
should not be reached from any context, and the per-site declaration is
purely organizational.

## What breaks if applied

Nothing visible has been observed to break. The site renders and behaves
normally. The `sendBeacon()` call in the page's code returns `false`
(beacon rejected) instead of `true`, but GitHub's page code does not
appear to do anything user-visible with the return value - the telemetry
is genuinely fire-and-forget.

If you rely on anything GitHub analytics-powered (repo insight
contribution graphs derived from client telemetry, in-product A/B flags
that phone home), you may want to allow the rule. None of these have been
observed to use `collector.github.com` specifically.

## How to apply

- **From the popup:** click `+ Add` on the sendBeacon suggestion next time
  you see it on `github.com`. Hush writes the rule into your config
  immediately and reloads the matched site.
- **Manually:** open Options, add a new site `github.com`, then under the
  **Block (network)** section add `||collector.github.com`.
- **From raw JSON:** paste the snippet above into the Advanced JSON
  editor on the Options page.

## Related

The same browser API (`navigator.sendBeacon`) is used by many
first-party-operated telemetry subdomains across the web. If you see
suggestions like `||track.*.com`, `||metrics.*.com`, `||log.*.com`, or
`||*.doubleclick.net` with high confidence and a sendBeacon evidence
set, the same blocking rationale applies.
