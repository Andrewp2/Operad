# Operad 5.0 Migration Guide

Operad `5.0.0` establishes the v5 public API, docs, and release-gate foundation.
It does not claim the full interaction-runtime roadmap is complete; see
`docs/v5_0_completion_audit.md` for current done, partial, and gap status.

## Upgrade from `4.0.0` to `5.0.0`

1. Update dependency metadata.

```toml
operad = { git = "https://github.com/Andrewp2/Operad.git", tag = "v5.0.0", features = ["widgets", "wgpu"] }
```

Use a local path dependency while validating unreleased downstream changes.

2. Prefer Operad-owned layout primitives for new public code.

New v5-facing code should use `operad::layout` helpers and public layout facade
types for common construction and inspection. Existing `LayoutStyle` and Taffy
conversion paths remain available for migration and advanced layout cases.

3. Annotate internationalization behavior explicitly where product code depends
   on it.

The v5 public surface includes locale identity, text direction, bidi policy,
layout mirroring, localization policy, and dynamic label metadata. Widgets and
renderers do not yet apply every policy end to end, so downstream applications
should treat these as explicit contracts to wire through product UI state.

4. Use API stability markers when consuming new v5 surfaces.

`operad::versioning` classifies APIs as stable, experimental,
backend-specific, or migration-only. Prefer stable APIs for product-facing code
and quarantine experimental/backend-specific calls behind local adapters.
Theme and design-token stability policy is recorded in
`docs/v5_0_theme_and_stability.md` and mirrored by
`operad::theme_stability`.

5. Keep feature gates intentional.

- `widgets` enables the higher-level widget helpers and widget extension tests.
- `wgpu` enables `WgpuRenderer`, `WgpuSurfaceRenderer`, and GPU validation paths.
- `egui` remains host/input/platform compatibility.
- `egui-renderer-compat` remains the legacy egui painter compatibility path.
- `text-cosmic` enables the optional Cosmic Text measurer.

6. Move shared host and renderer bookkeeping toward v5 contracts.

Use `RuntimeLoopState`, `WidgetActionQueue`, `RetainedWidgetStateStore`,
`TextEditHistory`, `EffectiveGeometry`, `ResourceCache`, and the scrolling and
compositor policy modules for new integration work. These are backend-neutral
contracts; some existing widget and renderer paths still need incremental
rewiring, so prefer thin local adapters while migrating downstream code.

7. Move async task and form state through explicit lifecycle records.

`TaskRegistry` and `FormState` provide deterministic state transitions for
progress, cancellation, stale result rejection, validation generations,
apply/submit/cancel/reset phases, and accessible error summaries. Hosts still
own their executor and product validators; Operad owns the UI-side contracts.

8. Treat accessibility adapters as explicit host state.

`HeadlessAccessibilityAdapter` is available for tests and non-platform
integration. Platform adapters should consume the same request/report types and
can publish canvas/editor target summaries through
`AccessibilityAdapterTargetSummary`.

## Validation

Run the required gates in `docs/v5_0_release_checklist.md`. Basic CI compiles
all feature combinations and enumerates all-feature tests, but WGPU parity and
perf smoke tests should run only on machines with suitable adapters and stable
timing characteristics.
