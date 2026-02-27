# Performance

## Scope

Primary metric: request-time query-filtering latency (p95).

## Harness

A synthetic harness is available at:

- `crates/integration-tests/tests/perf_filter_logic.rs`

Run:

```bash
cargo test -p integration-tests --test perf_filter_logic -- --nocapture
```

The harness prints p50/p95 timings across repeated iterations of representative query-filtering logic.

Example output (from one local run; values are machine-dependent):

```text
p50=600ns
p95=624ns
```

## Implementation Note

The current implementation includes an identity fast path in variable evaluation:

- when the filter is `initial all;` with no include/exclude rules, input args are returned directly without per-segment filtering.

This reduces avoidable scanning and branching for passthrough configurations.

## Notes

- Use this harness for relative comparisons across code changes on the same environment.
