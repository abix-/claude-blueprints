---
name: n
description: Auto-pick next PR/issue and start reviewing it immediately
disable-model-invocation: false
---
Run `k3sc next` to find the next item, then immediately invoke `/review` on it. No prompting, no asking -- just start the review.

1. Run `k3sc next` to get the next PR or issue
2. Parse the PR/issue number from the output
3. Invoke `/review {number}` immediately
