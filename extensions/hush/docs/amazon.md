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

## Rule

Based only on what Hush suggested and what the DOM confirms:

```json
{
  "amazon.com": {
    "remove": [
      "iframe[id^=\"ape_\"]"
    ],
    "hide": [],
    "block": []
  }
}
```

The `iframe[id^="ape_"]` rule is slightly stricter than Hush's suggested
`iframe[src*="m.media-amazon.com"]` - it targets the `ape_` ID convention
that both observed iframes share. Trade-off:

- `iframe[src*="m.media-amazon.com"]`: Hush's suggestion. May match any iframe
  loaded from `m.media-amazon.com` regardless of role. If Amazon ever uses
  that CDN for non-ad iframes, this rule catches those too (false positives).
- `iframe[id^="ape_"]`: more surgical. Matches only iframes that follow the
  `ape_` naming convention (which both observed ad iframes do).

Either is fine for starting out. If you see non-ad iframes disappearing,
switch to the `id^="ape_"` form.

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
