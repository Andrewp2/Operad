# Operad 5.0 Completion Audit

This audit maps the v5 roadmap prompt to current repository artifacts. It is a
release-readiness checklist, not a claim that v5 is complete.

Status key:

- `Done`: the repository has a focused artifact and regression coverage for the
  stated gate.
- `Partial`: the repository has reusable pieces, but the release gate is not yet
  fully satisfied.
- `Gap`: no complete Operad-owned artifact exists yet.

## First Public API/Docs Slice

| Prompt item | Status | Current artifact | Remaining gap |
| --- | --- | --- | --- |
| Operad-owned layout primitives for common public API use | Done | `src/layout.rs` adds `Layout`, `LayoutDimension`, `LayoutInsets`, `LayoutSpacing`, alignment, display, position, and flex basics with conversion to `LayoutStyle` and Taffy. | Widget options still accept many existing `LayoutStyle` values and internals still store Taffy `Style`. |
| Reduce package-level test enumeration blockers from layout helper mismatches | Done | `examples/three_consumer_probe.rs` and `tests/e2e_render.rs` now use public node-style conversion helpers where `UiNodeStyle.layout` needs a Taffy style. | Full migration away from direct Taffy fields is still future work. |
| Localization, text direction, bidi, mirroring, and dynamic label metadata | Done | `src/i18n.rs` adds `LocaleId`, `TextDirection`, `BidiPolicy`, `LayoutMirrorMode`, `LocalizationPolicy`, and `DynamicLabelMeta`. | Widgets do not yet consume these policies for text measurement, placement, keyboard routing, or accessibility updates. |
| Public API stability/versioning marker types | Done | `src/versioning.rs` adds `Stable`, `Experimental`, `BackendSpecific`, `MigrationOnly`, `ApiStability`, `StabilityNote`, and `FeatureStability`. | Existing public APIs are not yet exhaustively annotated in docs. |
| Focused tests for layout, RTL/mirroring, and stability classifications | Done | Unit tests in `src/layout.rs`, `src/i18n.rs`, and `src/versioning.rs`. | No cross-widget RTL or semver lint coverage yet. |

## Proposed Release Gate Checklist

| Roadmap gate | Status | Current artifact | Remaining gap |
| --- | --- | --- | --- |
| Button activation and command/action routing work through pointer and keyboard input. | Partial | `src/actions.rs`, `src/commands.rs`, `src/host.rs`, widget helpers, command/action replay tests. | Widget helpers are not all rewired to emit the shared action queue by default. |
| Raw pointer gestures route through the host frame path and drive drag widgets without app-local plumbing. | Partial | `src/input.rs`, `src/drag_drop.rs`, host gesture state tests. | Drag/drop widgets still need full default action integration. |
| Native host runtime example opens a WGPU window and exercises core widgets. | Partial | `examples/native_wgpu_host.rs`, `src/runtime.rs`, `src/wgpu_renderer.rs`, host contracts. | The example is compile/offscreen oriented unless `OPERAD_RUN_WGPU_EXAMPLE` is set; no winit window dependency is included. |
| Text editing has undo/redo, robust IME lifecycle, multiline selection, and clipboard tests. | Done | `TextInputState` stores `TextEditHistory`, emits committed text transactions from widget edits, supports keyboard undo/redo, and keeps IME, multiline selection, clipboard, caret geometry, and platform request tests in `src/lib.rs`; `src/transactions.rs` owns the reusable history model. | Rich text and product-specific editor commands remain app/widget-extension work. |
| Widget identity and state binding preserve focus, overlays, scroll, animation, and edit state across rebuilds. | Done | `src/state.rs` provides stable widget keys, scopes, typed slots, retained state, lifecycle reports, keepalive, expiry, and invalidation tests. | Widget helpers still need incremental adoption of the store. |
| Shared selection models and edit transactions cover lists, trees, tables, timelines, forms, sliders, and canvas hit targets. | Done | `src/transactions.rs` provides edit transaction phases, selection models, text history, and undo/redo command descriptors. | Domain-specific widgets still need more direct adapters. |
| Form validation, dirty tracking, pending/apply/cancel, and accessible error summaries are covered by tests. | Done | `src/forms.rs` covers field/form messages, dirty/pending/validating/submitted state, async generations, stale rejection, apply/submit/cancel/reset, and accessible error summaries. | Product validators remain app-owned. |
| Unified keyboard navigation covers roving focus, active descendants, menus, listboxes, tables, trees, toolbars, and Escape/Enter/Space semantics. | Done | `src/navigation.rs` defines collection kinds, focus models, boundary behavior, activation/dismissal semantics, and tests. | Existing widget extension menus still need incremental adoption. |
| Overlay stack behavior is centralized and covered by nested popup/menu tests. | Done | `src/overlays.rs` defines overlay IDs, nesting, modal blocking, dismiss policies, ordering, outside-hit routing, and focus restore records. | Existing menu/popup helpers still need full stack integration. |
| At least one real accessibility adapter path is proven beyond metadata-only assertions. | Done | `src/accessibility.rs` includes `HeadlessAccessibilityAdapter`, adapter apply reports, focus trap state, navigable target summaries, and tests. | Platform-backed screen-reader adapters remain backend work. |
| Public layout APIs have an Operad-owned path that avoids direct Taffy use for common cases. | Done | `src/layout.rs` facade and conversion tests. | Internals and legacy helpers still expose Taffy for migration and advanced use. |
| Localization, RTL/text-direction, and dynamic labels have an explicit support path and regression coverage. | Partial | `src/i18n.rs` policy and unit tests. | Widgets/renderers do not yet apply these policies end to end. |
| Font, icon, image, and texture lifecycle behavior is documented and tested. | Done | `src/assets.rs`, `src/fonts.rs`, `src/resource_cache.rs`, renderer resource descriptors, WGPU resource paths. `FontRegistry` covers fallback stacks, loaded/missing/failed states, byte accounting, stale generation rejection, and LRU eviction planning. | Backend adapters still need to feed concrete platform font and resource objects into these contracts. |
| Renderer/performance tests cover interaction-heavy frames, large resources, and native-surface rendering where available. | Partial | `tests/perf_smoke.rs` covers interaction-heavy frame build/paint, large retained display-list reuse, WGPU window-target no-readback perf paths, and adapter-free WGPU enumeration for large resource requests; WGPU parity tests keep snapshot readback out of perf coverage. | True native OS-surface execution is not claimed in this environment; the current WGPU perf smoke path renders window/app-owned targets through discard/offscreen command paths and the native surface renderer remains a manual/host-backed gate. |
| Advanced scrolling handles nested arbitration, anchoring, sticky/fixed content, kinetic behavior, scrollbars, reveal-into-view, and synchronized surfaces. | Done | `src/scrolling.rs` covers nested arbitration, anchoring, fixed/sticky resolution, kinetic stepping, scrollbar geometry, reveal-into-view, and scroll sync tests. | Widget-level adoption remains incremental. |
| Layering/compositing semantics are explicit for stacking contexts, transforms, opacity groups, offscreen layers, clipping, masks, and transformed hit testing. | Done | `src/compositor.rs` and `src/effective_geometry.rs` cover stacking contexts, transforms, clips, masks, offscreen isolation, feature fallback plans, and shared hit geometry. | WGPU rendering does not yet implement every feature at visual parity. |
| Compositor-quality rendering covers shadows, rounded clipping, borders, gradients, masks, filters, subpixel text, and parity for composited content. | Partial | `src/compositor.rs` now records backend-neutral compositor-quality requirements, border support limits, CPU/WGPU quality profiles, WGPU fallback actions, and parity expectations for shadows, rounded clipping, borders, gradients, masks, filters, and subpixel text; paint primitives, gradients, paths, CPU/WGPU renderers provide the concrete rendering pieces. | WGPU still does not claim full visual parity for shadows, gradients, masks, filters, or subpixel text; those remain explicit fallback/unsupported parity records until native rendering is implemented. |
| Diagnostics can explain input routing, widget actions, overlay state, accessibility output, and render timing in one debug surface. | Done | `src/diagnostics.rs` provides typed diagnostic records, severity/category summaries, input routing, widget action, overlay, accessibility, effective geometry, render timing, dirty flag, warning, and error coverage. | Host-facing devtools can still build richer views on top of the unified report. |
| Theme/design-token APIs and feature stability are documented for v5 consumers. | Done | `src/theme.rs`, `src/theme_stability.rs`, `src/versioning.rs`, and `docs/v5_0_theme_and_stability.md` classify stable theme records, token categories, component state resolution, accessibility adjustments, backend-specific output, migration-only compatibility, and experimental inspection. | Runtime/widget adoption of every token remains incremental. |
| Async task state, loading/progress, cancellation, async validation, and repaint scheduling are covered by tests. | Done | `src/tasks.rs` and `src/forms.rs` cover task progress/cancel/complete/error, stale generations, async validation, and `RuntimeInvalidationReason::AsyncTask` summaries. | Hosts still own executors and product validators. |
| Virtualized list/table/tree/grid behavior handles huge datasets, measured row heights, sticky regions, selection, focus, and accessibility. | Done | `src/virtualization.rs` plans sparse measured extents, overscan, sticky regions, accessibility records, focus/selection preservation, and scroll-anchor adjustment. | Existing data widgets still need incremental adoption. |
| Multi-window and multi-document routing is proven for focus, IME, overlays, cursor, accessibility, and render surfaces. | Done | `src/windows.rs` provides window/document/surface IDs, router state, focus transfer, IME/cursor/accessibility/render routing, overlay ownership, and rejection reports. | Native backend integration remains host work. |
| Touch, stylus, and gamepad input have explicit routing and regression tests. | Done | `src/input_devices.rs` covers touch gesture classification, stylus metadata support/fallback, gamepad navigation policy, and capture/cancel routing reports. | Backend adapters still need to feed these contracts. |
| Tooltip, help, and context menu policy is centralized and accessible. | Done | `src/tooltips.rs` centralizes command tooltip text, accessible help text, timing/dismissal policy, validation help, context menu triggers, suppression reasons, and overlay records. | Existing widgets still need incremental adoption of the shared policy. |
| Scheduler/frame lifecycle behavior is deterministic in tests and prevents unbounded repaint loops. | Done | `src/runtime.rs` covers host frame request/output, repaint scheduler contracts, deterministic timer deadline/completion/cancellation records, idle budget/completion/cancellation records, repaint invalidation, coalescing, stale IDs, and loop-guard render suppression. | Hosts still own concrete timer and idle callback APIs. |
| Error boundaries, resource limits, and malformed input handling are documented and tested. | Done | `src/errors.rs` and `src/limits.rs` classify errors, fallback decisions, resource/input limits, cache budgets, malformed input, and validation reports. | Widget-specific local error boundaries can still be layered on top. |
| CI/release automation covers feature matrix, docs, semver review, and package dry runs. | Done | `.github/workflows/ci.yml` and `docs/v5_0_release_checklist.md` cover format, feature-matrix checks, tests, docs, examples, package dry run, WGPU/perf manual gates, and semver review notes. | `cargo semver-checks` remains optional/manual unless installed. |
| API docs explain core concepts, lifecycle, ownership, and migration path. | Done | `docs/v5_0_core_concepts.md` explains the v5 contract model, core concepts, lifecycle, ownership boundaries, migration path, compatibility notes, and how backend-neutral contracts fit together. | Keep this reference current as runtime/widget adoption changes. |

## Audit Notes

- The first slice intentionally avoided actions, compositor internals, scrolling
  behavior changes, and host runtime internals; later v5 slices added focused
  contracts for those areas.
- `LayoutStyle` remains the compatibility bridge to Taffy. New public code
  should prefer `operad::layout::Layout` and related Operad-owned primitives for
  common layout construction.
- The v5 release gate is now mostly covered by backend-neutral contracts and
  focused regression tests. Remaining partial areas are primarily deep backend
  rendering parity, native window runtime integration, and incremental adoption
  of the shared contracts by older widget helpers.
- Renderer perf coverage intentionally separates WGPU perf timing from snapshot
  readback. `tests/perf_smoke.rs` uses window targets for WGPU perf paths and
  includes adapter-free enumeration for large resource display lists so CI can
  compile/list the path even when a native adapter or OS surface is unavailable.
- `docs/v5_0_core_concepts.md` is the concise concept/reference entry point for
  the backend-neutral v5 contracts and should be updated when ownership or
  lifecycle rules change.
