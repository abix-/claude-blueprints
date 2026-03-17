---
description: Run Criterion system benchmarks and record results to docs/performance.md
disable-model-invocation: true
allowed-tools: Bash, Read, Edit, Write, Glob, Grep
version: "1.0"
---
## Steps

1. **Run benchmarks**: Run `k3s-claude cargo-lock bench --bench system_bench 2>&1 | tee /c/code/endless/rust/bench_results.txt` and wait for completion.

2. **Read results**: Read `rust/bench_results.txt` and extract the timing data for each system at each entity count.

3. **Read docs/performance.md**: Find the `## Benchmark History` section.

4. **Append new entry**: Add a new dated entry to the Benchmark History section with a markdown table showing all system timings. Format:

```
### YYYY-MM-DD — <commit hash (short)>

| System | 1K | 50K |
|--------|-----|-----|
| decision | Xus | Xus |
| damage | Xus | Xus |
| healing | Xus | Xus |
| attack | Xus | Xus |

Combined 50K: X.Xms (X.X% of 16ms budget)
```

5. **Summarize**: Print a short analysis comparing to the previous entry (if any) — which systems improved, regressed, or stayed the same. Include the Factorio-style budget math.

## Rules

- Always write raw output to `rust/bench_results.txt` (overwritten each run)
- Round timings to nearest whole µs in the table
- Use the median value (middle of the `[low median high]` range) from Criterion output
- If a previous benchmark entry exists, note regressions >5% or improvements >5%
- Don't modify anything in performance.md outside the Benchmark History section
- If $ARGUMENTS is provided, pass it through to `cargo bench` (e.g. `-- decision` to filter)
