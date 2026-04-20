# Hush case studies

Real-world per-site rule sets showing how Hush solves problems that generic ad/tracker blockers can't.

Each case study documents:

- The rules themselves, copy-pasteable into Hush's config
- What each rule catches on the page
- Why the specific selector and layer were chosen
- How the rule was discovered (behavioral detection vs manual DOM inspection)
- What breaks if the rule is applied, and what to watch for as the site evolves

## Available case studies

- [reddit.md](reddit.md) - Reddit's telemetry beacons, Brand Affiliate posts, algorithmic community recommendations, and sidebar widgets. Demonstrates `:has()` parent selectors, attribute-based matching, and the hide-vs-remove decision for framework components.
- [amazon.md](amazon.md) - Amazon homepage ad iframes. Narrow scope, only documents what has been observed in Hush's suggestion output on the homepage - no speculation about other Amazon surfaces.
- [github.md](github.md) - GitHub's first-party telemetry collector (`collector.github.com`). Demonstrates how the sendBeacon detector catches first-party-owned analytics that curated filter lists don't reach.

## Planning

- [roadmap.md](roadmap.md) - stage-based development roadmap. First unchecked stage is the current sprint. Follows the endless repo pattern: numbered stages with "Done when:" acceptance criteria.
- [completed.md](completed.md) - snapshot of current user-facing features.
- [history.md](history.md) - retired rollout notes.
- [heuristic-roadmap.md](heuristic-roadmap.md) - gap analysis of detection signals. Source material for future roadmap stages.

## Contributing your own

Good case studies:

1. Describe a site-specific pattern curated lists don't catch
2. Use stable selectors (custom-element tags, stable attributes) rather than utility classes
3. Explain *why* each rule is safe (what it does and does not affect)
4. Note any caveats or expected breakage if the site's DOM changes

Add new files alongside the existing ones and link them from this index.
