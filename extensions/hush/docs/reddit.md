# Reddit rules - case study

Reddit is the clearest demonstration of what Hush is for. uBlock Origin Lite's curated filter lists catch generic third-party trackers, but Reddit's site-specific anti-user patterns are first-party or use custom web components that lists don't know about:

- Their own `w3-reporting.reddit.com` telemetry endpoint (first-party, no list blocks first-party domains by default)
- Custom elements they author themselves (`games-section-badge-controller`, `shreddit-brand-affiliate-tag`, etc.) that only exist on Reddit
- Algorithmic insertions into the main feed disguised as community recommendations

Everything below was discovered by inspecting Reddit's DOM (F12 -> right-click the offending element -> Inspect) and writing the smallest stable selector. The Hush behavioral suggestion feature proposed some of these; others needed manual inspection.

Paste this block into Hush as-is, or into the raw-JSON editor:

```json
{
  "reddit.com": {
    "remove": [
      "games-section-badge-controller",
      "article[data-post-id]:has([is-post-commercial-communication])",
      "faceplate-partial[name^=\"RelatedCommunityRecommendations\"]"
    ],
    "block": [
      "||w3-reporting.reddit.com^"
    ]
  }
}
```

Reddit's site entry covers `reddit.com` and all its subdomains (`www.reddit.com`, `sh.reddit.com`, `old.reddit.com`, etc.) via Hush's exact-or-suffix domain match. Each rule is documented below.

---

## Block: `||w3-reporting.reddit.com^`

**Layer:** Block (network)

**What it catches:** every request to the `w3-reporting.reddit.com` subdomain, which is Reddit's first-party event telemetry pipeline. The "w3" refers to Reddit's rewritten analytics stack. `navigator.sendBeacon` fires events there throughout normal usage - typically four beacons per page load - carrying:

- Which posts you scrolled past (impression tracking)
- How long each post was on screen (dwell time)
- Click vs ignore for each visible post
- Feed position and ranking-signal feedback
- Ad/promo impression counts for advertisers

**How it was discovered:** Hush's behavioral suggestion detector flagged it as a `sendBeacon` target (confidence 95). `sendBeacon` is the classic telemetry API - it's fire-and-forget, purpose-built to survive page-unload, and has no legitimate non-tracking use case.

**Why it's safe:** Reddit's user-facing features run through completely different endpoints:

- `gql.reddit.com` - GraphQL queries for feed/post data
- `oauth.reddit.com` and `www.reddit.com/api/*` - authenticated actions (vote, comment, submit, message)
- `www.redditstatic.com` - static assets and chunked JS bundles

`w3-reporting` is strictly telemetry. Nothing you interact with routes through it.

**Second-order effects:**

- Reddit's recommendation engine gets slightly less training data from your sessions. Many users consider this a feature.
- Subreddit mod analytics may lose data on your own views of subs you moderate.
- Ad targeting becomes more generic for your account.

---

## Remove: commercial posts

Pick ONE of these two rules - they overlap, so using both is redundant (the second would try to match elements the first already removed).

### Aggressive (recommended): `article[data-post-id]:has([is-post-commercial-communication])`

**What it catches:** every feed post Reddit flags as commercial. `is-post-commercial-communication` is an attribute Reddit sets on `<shreddit-post-overflow-menu>` for promoted posts, Brand Affiliate posts, in-feed ads, and any new commercial variant Reddit ships.

**Risk:** broader rule = broader blast radius if Reddit ever repurposes the attribute (unlikely but possible). If you see legitimate posts disappearing, swap to the conservative rule below.

### Conservative: `article[data-post-id]:has(shreddit-brand-affiliate-tag)`

**What it catches:** only "Brand Affiliate" posts - where a real user is paid to promote something. Reddit tags these with a dedicated custom element `<shreddit-brand-affiliate-tag>` that renders as "Brand Affiliate" in the credit bar. Regular promoted ads are NOT caught by this rule.

**Risk:** low. The custom element's tag name is unambiguously named and dedicated to this one feature. If you want narrow, proven-safe behavior at the cost of leaving regular ads alone, use this.

### Selector anatomy (applies to both)

- `article[data-post-id]` - every feed row is wrapped in `<article data-post-id="t3_...">`. The attribute-exists selector confines us to real post rows, not stray `<article>` elements elsewhere on the page.
- `:has(...)` - parent selector. CSS `:has()` matches the article only if it contains a matching descendant.

### How they were discovered

User spotted the "Brand Affiliate" text in a post's credit bar and inspected the element. Walking up the DOM tree found `<shreddit-brand-affiliate-tag>` as a dedicated custom element, and noticed `is-post-commercial-communication` as an attribute on a sibling element of the same post. Tag-name selectors and attribute-existence selectors are far more stable than utility-class chains (which change with every framework update).

### Why Remove over Hide

- Reddit wraps posts in `<faceplate-tracker>` which fires view/impression events as soon as a post enters the viewport. `display: none` does NOT always prevent that - some trackers fire on intersection, some on hydration. `.remove()` guarantees the element never exists to have its tracker fire.
- Infinite scroll: Hush's MutationObserver catches each newly-loaded post as it appears. Commercial posts never flash on screen; they're removed in the same tick they're added.
- Removing the element also frees any event listeners Reddit's framework attached to it.

---

## Remove: `faceplate-partial[name^="RelatedCommunityRecommendations"]`

**Layer:** Remove (DOM)

**What it catches:** the "Related communities you might like" blocks Reddit injects between posts in the main feed. These aren't marked as ads or commercial - they're Reddit's algorithmic cross-promotion for other subreddits, appearing as `<faceplate-partial name="RelatedCommunityRecommendations_XXXXX">` where `XXXXX` is a randomized tracking ID that changes per session.

**The selector:**

- `faceplate-partial` - the Reddit framework's lazy-load container element.
- `[name^="RelatedCommunityRecommendations"]` - attribute starts-with match. The random suffix rotates, so we anchor on the stable prefix.

**How it was discovered:** inspecting the DOM around a feed post showed that after each post, Reddit inserts a `<faceplate-partial>` pointing to `/svc/shreddit/partial/XXXXX/related-community-recommendations`. Recognizable by the stable `name` attribute prefix.

**Why Remove:** these are SPA-injected after the initial feed load, so we need MutationObserver to catch them as they appear. They also lazy-load their content over the network, so removing the element before it hydrates also cancels the pending request.

**What the evidence looks like in the popup:** Reddit injects several of these in a single feed chunk, so the Removed section will show multiple entries at the same timestamp. Each entry carries the unique `name` attribute (e.g. `name="RelatedCommunityRecommendations_qmZmnB"`) so they're distinguishable in the evidence expansion even though the tag + class signature is identical.

---

## Remove: `games-section-badge-controller`

**Layer:** Remove (DOM)

**What it catches:** Reddit's "Games" section in the left navigation sidebar. Click-to-expand widget showing featured Reddit games, nudging you to play them.

**Why Remove over Hide:** Hide (`display: none`) only stops rendering. The widget's JavaScript is still alive and could be polling for new featured games, fetching badge counts, or firing impression beacons in the background. Remove physically deletes the element, which stops any polling/fetching tied to its lifecycle.

**Framework-re-render concern:** the sidebar is a Lit-based web component and its parent nav might try to re-render the Games widget on state changes. That's fine - Hush's MutationObserver catches each re-add in the next tick and removes it again. No loop, since frameworks don't immediately re-render after arbitrary DOM mutations (they wait for their own state changes).

**How it was discovered:** the user described the pain point ("i don't need games on reddit"), inspected the widget, and found the custom-element tag name in the DOM path. Custom-element tags are far more stable than descendant class chains.

---

## What uBlock Origin Lite misses, and why Hush catches it

All three of the remove rules above target Reddit's own custom elements (`shreddit-brand-affiliate-tag`, `faceplate-partial`). Curated filter lists like EasyList and EasyPrivacy don't have entries for these because they're:

1. **Site-specific** - these custom elements only exist on Reddit; a global list would be noise
2. **Framework-level** - the tag names aren't inherently "ads" or "trackers"; their role is determined by how Reddit uses them
3. **Moving targets** - Reddit's ad-adjacent custom elements can change with frontend updates, and a curated list would need a Reddit-specific maintainer

The `w3-reporting.reddit.com` block is similar - curated lists are conservative about blanket-blocking first-party subdomains of major sites, because those subdomains sometimes serve real functionality. Hush's behavioral detector looked at what the subdomain actually does (`sendBeacon` target, no visible effect) and recommended the block based on behavior, not list membership.

This is the "per-site surgical cleanup" value Hush is designed for.

---

## Discovering more Reddit rules

Workflow that found the rules above:

1. Turn on Hush's behavioral suggestions (Options -> bottom of page -> checkbox). Reload Reddit.
2. Click the Hush popup. Review any Suggestions. Click the **Evidence** button on each to see the raw data - for block suggestions, the URLs; for remove suggestions, the hidden-iframe signatures and outerHTML snippets.
3. Accept suggestions that look right (the sendBeacon targets are always safe; hidden iframes usually are).
4. For cosmetic nuisances the detector can't see (like Brand Affiliate posts or sidebar widgets): right-click the element -> Inspect. Walk up the DOM tree looking for:
   - **Custom element tag names** (`<shreddit-foo>`, `<faceplate-bar>`) - the most stable hook
   - **Stable attributes** (`data-post-id`, `is-post-commercial-communication`) - next most stable
   - **Descendant chains involving a custom element** via `:has()` - good for wrapping an anonymous outer container
5. Avoid these selector choices - they break on the next frontend deploy:
   - Utility-class chains like `.bg-neutral-background.focus-within:bg-neutral-background-hover` - change frequently
   - Random-hash IDs or classes like `.abcdef12345`
   - Long descendant paths like `#left-sidebar > nav > foo > bar > baz` - fragile
6. Prefer remove by default. Hide only when removing the element breaks the site's layout (empty flex slots, height collapse on a container that the framework measures). For most unwanted widgets, remove is better because it stops any background polling/telemetry the widget's JS might be doing.

---

## Known limits

- **Reddit's app redirect banner** ("View in app") is injected via their framework; the selector can be slippery. Not included here yet.
- **Promoted posts in other surfaces** (user profile feeds, subreddit feeds) may use slightly different attributes than the main feed. The commercial-communication rule should cover them but has been verified only on the home feed.
- **Old Reddit** (`old.reddit.com`) has a completely different DOM. These rules target new Reddit (`www.reddit.com`, `sh.reddit.com`). If you use old Reddit, you'd want a separate site entry for `old.reddit.com` with its own selectors.
