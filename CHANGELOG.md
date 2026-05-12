# Changelog

## 5.0.0

- Added Operad-owned public layout primitives for common API use, with conversion
  paths back to `LayoutStyle` and Taffy for migration and advanced cases.
- Added localization and internationalization policy types for locale identity,
  text direction, bidi behavior, layout mirroring, and dynamic label metadata.
- Added public API stability/versioning marker types so v5 consumers can
  distinguish stable, experimental, backend-specific, and migration-only APIs.
- Added backend-neutral runtime/frame lifecycle contracts, widget action queues,
  retained widget state lifecycle, edit transactions, and selection/history
  helpers for interaction-oriented hosts.
- Added core widget action routing helpers and action bindings for buttons,
  checkboxes, sliders, and text inputs.
- Wired widget text input edits into `TextEditHistory`, including committed
  transactions and keyboard undo/redo for text input state.
- Added async task lifecycle and form validation contracts for progress,
  cancellation, stale async results, dirty/pending state, submit/apply/cancel
  workflows, and accessible error summaries.
- Added shared effective-geometry, advanced scrolling, compositor feature, and
  resource cache lifecycle contracts for renderer and host integration.
- Added font lifecycle contracts for fallback stacks, loaded/missing/failed
  states, generation checks, cache byte accounting, and eviction planning.
- Added headless accessibility adapter contracts, accessibility target
  publication records, error classification, resource/input limits, and release
  guardrails for adapter and renderer failures.
- Added touch/stylus/gamepad routing, multi-window routing, navigation/overlay
  contracts, virtualization planning, tooltip/help/context menu policy, unified
  diagnostics, and theme/design-token stability documentation.
- Updated consumer-style probes and render tests to use the new public
  conversion helpers where direct backend layout fields are not required.
- Documented the v5 completion audit, migration posture, release checklist, and
  CI/release gates for fmt, feature-matrix checks, docs, examples, package
  verification, WGPU validation, perf smoke, and semver review.

## 4.0.0

- Added optional `wgpu` rendering support behind the `wgpu` feature.
- Added `WgpuRenderer` and `WgpuSurfaceRenderer` exports under the `wgpu` feature.
- Added GPU snapshot parity test coverage in `tests/wgpu_snapshot_parity.rs`,
  covering CPU parity, texture upload, SDF rounded rectangles, glyphon text, and
  paint order across text and geometry.
- Added WGPU no-readback perf coverage for cached text and mixed changing UI
  scenes, with a release-mode 1 ms p95 render budget.
- Added opt-in GPU render-pass timestamp timing via `RenderOptions::collect_gpu_timing`.
- Added glyphon text chunk caching so changing one text run does not force
  preparing every visible text surface each frame.
- Split the legacy egui painter into `egui-renderer-compat`; the `egui`
  feature now represents host/input/platform compatibility rather than the
  renderer backend path.
- Added initial v4 migration guidance and release checklist.
- Added v4 migration-compat constructors so legacy `taffy::Style` inputs can be passed
  into common node/style constructors (`UiNode::container`, `UiNode::text`, etc.) during
  downstream upgrades.
- Expanded migration compatibility in widget entry points by allowing
  `label`/`scroll_area` to accept legacy layout styles and adding `with_layout(...)`
  to widget option structs (`ButtonOptions`, `CheckboxOptions`, `SliderOptions`,
  `TextInputOptions`, `ComboBoxOptions`).

## 3.0.0

- Breaking layout API updates (LayoutStyle ownership migration) and host/rendering
  modernization for v3 contracts.
