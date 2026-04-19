# Amazon rules - case study

This doc only documents rules that have been observed and verified in Hush's
suggestion output on the Amazon homepage. No speculation, no "these other
domains are probably also ads" guesses. Extend this doc yourself as you
encounter and verify more patterns.

## What was observed

Visiting the Amazon homepage (`www.amazon.com`) with Hush's behavioral
suggestions enabled surfaced a **hidden iframe** suggestion with the
following evidence:

- **Suggested rule:** `iframe[src*="m.media-amazon.com"]`
- **Layer:** Remove
- **Reason:** hidden iframe (`visibility: hidden`, 1x1 size)

Inspecting one of the flagged iframes:

```html
<iframe id="ape_Gateway_desktop-homepage-btf-left_desktop_iframe"
        name="{...aaxImpPixelUrl:.../ad-events/loaded/...?publisherId=stores...}"
        src="..."></iframe>
```

And another:

```html
<iframe id="ape_Gateway_right-7_desktop_iframe"
        name="{...aaxImpPixelUrl:.../ad-events/loaded/...}"
        src="..."></iframe>
```

## Observed patterns

What the iframe DOM we captured actually tells us:

- **IDs start with `ape_`** on both observed iframes. Looks like a stable
  naming convention for ad slots on Amazon's homepage.
- **IDs contain `btf`** and `right-N` slot-position suffixes - classic
  ad-slot naming (`btf` = below-the-fold).
- **The `name` attribute carries JSON metadata** with `aaxImpPixelUrl` /
  `aaxInstrPixelUrl` fields pointing to Amazon's ad infrastructure.
  AAX = Amazon Advertising eXchange.

Combined with the hidden-iframe behavioral detection (1x1 size,
`visibility: hidden`) and the above naming, these are ad containers.

## Second observation: programmatic ad iframes

A subsequent scan surfaced another hidden-iframe suggestion:

- **Suggested rule:** `iframe[src*="s.amazon-adsystem.com"]`
- **Layer:** Remove
- **Reason:** hidden iframe

Iframes loaded from `s.amazon-adsystem.com` - Amazon's programmatic ad-serving
subdomain. Distinct from the `m.media-amazon.com` iframes observed earlier;
both appear as hidden iframe suggestions but load from different CDNs. Keep
both rules to catch both patterns.

## Third observation: CSM telemetry beacons

A subsequent scan surfaced a second Hush suggestion with this evidence:

- **Suggested rule:** `||unagi.amazon.com^`
- **Layer:** Block
- **Reason:** sendBeacon target (5 beacons sent)

Observed URLs:

```
https://unagi.amazon.com/1/events/com.amazon.csm.csa.prod
https://unagi.amazon.com/1/events/com.amazon.csm.customsg.prod
https://unagi.amazon.com/1/events/com.amazon.csm.nexusclient.prod
```

What the evidence confirms:

- Host `unagi.amazon.com` is Amazon's Client-Side Monitoring (CSM) telemetry ingest.
  The path segment `com.amazon.csm.*` is Amazon's internal event-namespace convention.
- All five requests used `navigator.sendBeacon`, which exists only to send
  fire-and-forget telemetry. No feature behavior depends on the response.
- `nexusclient` is one of Amazon's telemetry channels; `csa`/`customsg` are
  others. All `.prod` environments.

Safe to block. Same class of target as Reddit's `w3-reporting.reddit.com`.

## Rules

Based only on observed Hush suggestions plus DOM confirmation:

```json
{
  "amazon.com": {
    "remove": [
      "iframe[id^=\"ape_\"]",
      "iframe[src*=\"s.amazon-adsystem.com\"]"
    ],
    "hide": [],
    "block": [
      "||unagi.amazon.com^"
    ]
  }
}
```

### Remove: `iframe[id^="ape_"]`

Slightly stricter than Hush's original `iframe[src*="m.media-amazon.com"]`
suggestion - targets the `ape_` ID convention that both observed ad iframes
shared. Catches the homepage ad slots regardless of which CDN they load from.

### Remove: `iframe[src*="s.amazon-adsystem.com"]`

Catches iframes served from Amazon's programmatic ad-serving subdomain.
Observed separately from the `ape_`-prefixed iframes - some ad iframes may
be on a different CDN or carry different IDs, so this rule is the backstop.

### Block: `||unagi.amazon.com^`

Kills the CSM telemetry ingest at the network layer. Since these are
`sendBeacon` calls fired by Amazon's page JS (not a removable DOM element),
network-block is the only mechanism that can stop them - Remove wouldn't
help.

## What this doc does NOT claim

We did **not** observe in Hush's output or verify via the Network tab:

- Block rules targeting `amazon-adsystem.com` or any similar AAX backend host.
  Those may exist in Amazon's infrastructure but have not been seen in the
  parent page's traffic in our session.
- Block rules targeting `advertising.amazon.dev`. The URL appears in the
  iframe's `name` attribute as metadata, but we have not observed the parent
  page fetching it. (It's fetched by code running INSIDE the ad iframe,
  which Hush's content script doesn't scan - content scripts run in the top
  frame only, with `all_frames: false`.)
- Any rules beyond the homepage. Product pages, search results, and Prime
  Video may have different ad conventions that need their own inspection.

If you want to block traffic originating INSIDE ad iframes rather than just
remove the iframe container itself, that needs a different approach:
enabling `all_frames: true` in Hush's manifest so the content script runs
inside sub-frames too. That's a future enhancement, not a rule you can add.

## Discovering more Amazon patterns

Follow the same method as the Reddit case study:

1. Turn on behavioral suggestions, reload an Amazon page.
2. Check Hush popup - accept/dismiss what it surfaces.
3. For anything Hush didn't catch: inspect the DOM manually, look for
   stable `id`/`name`/data-attribute patterns, not utility class chains.
4. Document what you added here, with enough evidence that someone else
   can verify it themselves.
