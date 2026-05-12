# Operad 4.0 Release Checklist

Goal: make a production-ready 4.0 Operad library release with explicit
v3->v4 transition evidence. Downstream application migrations are tracked by
those applications and are not Operad release blockers.

## v4 Scope Items

1. **Renderer expansion**
- [x] Add optional `wgpu` feature and renderer module wiring.
  - Implemented in [Cargo.toml](/home/andrew-peterson/code/operad/Cargo.toml)
  - Implemented in [src/lib.rs](/home/andrew-peterson/code/operad/src/lib.rs)
  - Validated by `cargo check --features wgpu`
- [x] Add WGPU renderer tests and parity coverage for snapshot output.
  - Implemented in [src/wgpu_renderer.rs](/home/andrew-peterson/code/operad/src/wgpu_renderer.rs)
  - Implemented in [tests/wgpu_snapshot_parity.rs](/home/andrew-peterson/code/operad/tests/wgpu_snapshot_parity.rs)
  - Verified by:
    - `cargo test --features wgpu --test wgpu_snapshot_parity -- --nocapture`
    - `wgpu_snapshot_matches_cpu_snapshot` passes
- [x] Add native viewport/path-conformance test beyond snapshot parity.
  - Implemented in [tests/perf_smoke.rs](/home/andrew-peterson/code/operad/tests/perf_smoke.rs)
    - `scenario_harness_multi_frame_render_smoke_stays_under_budget`
    - `wgpu_text_cache_window_render_stays_under_budget_without_readback`
    - `wgpu_mixed_changing_ui_window_render_stays_under_budget_without_readback`
  - Verified by:
    - `cargo test --features widgets,wgpu --test perf_smoke -- --nocapture`
    - `cargo test --release --features widgets,wgpu --test perf_smoke -- --nocapture`
    - `scenario_harness_multi_frame_render_smoke_stays_under_budget` and WGPU no-readback render budget tests pass
  - `tests/perf_smoke.rs` serializes benchmark tests through a shared lock so
    render budget gates are not polluted by concurrent perf tests.
- [x] Split egui host compatibility from egui renderer compatibility.
  - Implemented in [Cargo.toml](/home/andrew-peterson/code/operad/Cargo.toml)
  - Implemented in [src/lib.rs](/home/andrew-peterson/code/operad/src/lib.rs)
  - `egui` keeps host/input/platform adapters; legacy `paint_document_egui*`
    helpers now require `egui-renderer-compat`.

2. **Downstream migration boundary**
- [x] Document that downstream app probes are adoption work, not Operad release blockers.
  - [docs/v4_0_roadmap.md](/home/andrew-peterson/code/operad/docs/v4_0_roadmap.md)
  - [docs/v4_0_migration_guide.md](/home/andrew-peterson/code/operad/docs/v4_0_migration_guide.md)
- [x] Preserve migration compatibility for common legacy `taffy::Style` call sites.
  - `LayoutStyle::from_taffy_style(...)`
  - `From<taffy::Style> for LayoutStyle`
  - `From<taffy::Style> for UiNodeStyle`
  - node/widget constructors that accept `impl Into<LayoutStyle>` or `impl Into<UiNodeStyle>`.
- [x] Record current downstream probe status as non-blocking evidence.
  - Orbifold and Fabricad/layout have local path-based probe evidence.
  - Game migration work remains downstream-owned and should be completed in the game repository.

3. **Migration and release preparation**
- [x] Add v4 migration guide.
  - [docs/v4_0_migration_guide.md](/home/andrew-peterson/code/operad/docs/v4_0_migration_guide.md)
- [x] Add constructor compatibility for legacy `taffy::Style` usage.
  - `src/lib.rs`
  - `src/lib.rs` tests:
    - `ui_node_factories_accept_legacy_taffy_styles`
    - `document_accepts_legacy_taffy_root_and_style_updates`
    - `widget_apis_accept_legacy_taffy_layout_inputs` (gated behind `cfg(feature = "widgets")`)
  - `widgets` label/scroll_area `layout` args and options `with_layout(...)` methods now accept legacy layout types.
- [x] Bump crate version metadata.
  - [Cargo.toml](/home/andrew-peterson/code/operad/Cargo.toml)
  - [Cargo.lock](/home/andrew-peterson/code/operad/Cargo.lock)
- [x] Capture and publish release notes/changelog entry for 4.0.
  - [CHANGELOG.md](/home/andrew-peterson/code/operad/CHANGELOG.md)

4. **Release gate deliverables**
- [x] At least one renderer/native-viewport conformance test present.
  - `tests/perf_smoke.rs`
  - `scenario_harness_multi_frame_render_smoke_stays_under_budget` with feature set `widgets,wgpu`
- [x] Downstream compatibility guidance present.
  - Downstream probes are non-blocking adoption checks for each app.
  - Operad v4 retains compatibility helpers for common v3 layout migration paths.
- [x] Migration guide present.
- [x] Documented decision on remaining public backend-type leakage.
  - [v4 release checklist](/home/andrew-peterson/code/operad/docs/v4_0_release_checklist.md) (decision log)
- [x] Release checklist includes accessibility and performance sign-off with evidence.
  - Compile sign-off: `cargo check --no-default-features`, `cargo check --all-features`
  - Library sign-off: `cargo test --lib`, `cargo test --all-features --lib`
  - Performance sign-off: `cargo test --features widgets,wgpu --test perf_smoke -- --nocapture`, `cargo test --release --features widgets,wgpu --test perf_smoke -- --nocapture`
  - Renderer sign-off: `cargo test --features wgpu --test wgpu_snapshot_parity -- --nocapture`
  - Accessibility sign-off: covered by `cargo test --lib` and `cargo test --all-features --lib`, including host request sequencing, accessibility summaries, audits, and focus-trap lifecycle tests.
- [x] Host accessibility request behavior coverage added for full supported-kind sequencing and focus trap lifecycle.
  - `host.rs` unit test `host_accessibility_requests_round_trip_all_supported_kinds_with_focus_trap`
  - Evidence: publish-tree, preference application, announcement, focus trap set, move-focus, restore-focus, and clear-trap responses handled in sequence.
- [x] Real-host accessibility adapter parity classified as downstream integration work.
  - Operad v4 covers accessibility metadata, host request sequencing, focus-trap lifecycle, and preference propagation contracts.
  - Platform screen-reader backends and host-window focus-trap preferences remain app/host integration responsibilities.

## Decision Log

- A minimal constructor-compatibility layer was added so legacy layout inputs can
  flow into `LayoutStyle`/`UiNodeStyle` without forcing immediate downstream
  signature edits; full backend-type abstraction cleanup remains deferred.
- Legacy egui painting is no longer part of the ordinary `egui` feature. It is
  quarantined behind `egui-renderer-compat` so WGPU is the renderer path under
  active validation while egui remains available as host compatibility glue.
- This checklist is intentionally explicit; the objective is to make blockers visible,
  not to hide incomplete areas behind a version bump alone.
- Operad v4's release gate is library-owned. Downstream app migrations are valuable
  validation but should not block publishing the Operad crate once the library
  checks above are green.
