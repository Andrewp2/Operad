# Operad 7.0 Performance Gates

V7 performance checks live outside customer-facing examples. The showcase should
remain a learning surface; repeatable stress probes belong in `tests/`.

## Stress Probes

`tests/perf_smoke.rs` is the v7 performance smoke suite. It covers:

- virtualized table layout and paint
- command palette filtering, build, and paint
- interaction-heavy widget frames
- retained display-list reuse
- editor geometry scene build and paint
- multi-frame scenario harness rendering
- WGPU text cache, mixed UI, GPU timestamp, and large-resource paths when the
  `wgpu` feature is enabled

## Commands

Run the backend-neutral widget probes:

```bash
cargo test --locked --no-default-features --features widgets --test perf_smoke -- --nocapture
```

Run the WGPU probes on machines with a compatible adapter:

```bash
cargo test --locked --features widgets,wgpu --test perf_smoke -- --nocapture
```

For release sign-off, also run the same commands with `--release` on the release
machine class and record intentional budget changes in the release notes.

## Budgets

The smoke suite owns the numeric budgets next to each scenario so reviewers can
see which workload changed. Current budget categories include:

- total and average CPU build/paint time for widget-heavy scenarios
- display-list cache hit, miss, eviction, and reuse accounting
- scenario frame totals and per-section render-frame budgets
- no-readback WGPU window-target render budgets
- GPU render-pass percentile budgets when timestamp queries are available

`PerformanceSnapshot`, `required_pipeline_stages`, and
`required_cache_diagnostic_kinds` are the diagnostic surface for explaining
failures. Stress probes should use those contracts when a failed budget needs to
report whether time went to tree rebuild, diffing, layout, text shaping, hit
testing, paint-list generation, batching, uploads, backend draw, or cache reuse.
