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
| Button activation and command/action routing work through pointer and keyboard input. | Partial | `src/commands.rs`, `src/host.rs`, widget button helpers, command replay tests. | No first-class widget event queue unifying pointer and keyboard activation. |
| Raw pointer gestures route through the host frame path and drive drag widgets without app-local plumbing. | Partial | `src/input.rs`, `src/drag_drop.rs`, host gesture state tests. | Drag begin/update/commit widget events and full host routing are incomplete. |
| Native host runtime example opens a WGPU window and exercises core widgets. | Gap | `src/wgpu_renderer.rs`, `src/egui_host.rs`, host contracts. | No canonical native window runtime example. |
| Text editing has undo/redo, robust IME lifecycle, multiline selection, and clipboard tests. | Partial | Widget text input state, IME host contracts, clipboard platform request types. | Undo/redo and full editing lifecycle coverage are incomplete. |
| Widget identity and state binding preserve focus, overlays, scroll, animation, and edit state across rebuilds. | Partial | Retained document IDs, scroll state, animation machines, shell state. | No general widget identity/state map contract. |
| Shared selection models and edit transactions cover lists, trees, tables, timelines, forms, sliders, and canvas hit targets. | Partial | Editor selection geometry, data widgets, `EditPhase`. | No unified transaction model across widget families. |
| Form validation, dirty tracking, pending/apply/cancel, and accessible error summaries are covered by tests. | Partial | Accessibility metadata and audits support required/invalid state. | Form workflow and validation lifecycle are not centralized. |
| Unified keyboard navigation covers roving focus, active descendants, menus, listboxes, tables, trees, toolbars, and Escape/Enter/Space semantics. | Partial | Basic focus order and accessibility active-descendant metadata. | Roving navigation and overlay-aware keyboard routing remain gaps. |
| Overlay stack behavior is centralized and covered by nested popup/menu tests. | Partial | Widget extension menu/surface modules have overlay-like helpers. | One shared overlay runtime stack is not complete. |
| At least one real accessibility adapter path is proven beyond metadata-only assertions. | Partial | Accessibility adapter request contracts and host publication tests. | Platform-backed screen-reader publication remains unproven. |
| Public layout APIs have an Operad-owned path that avoids direct Taffy use for common cases. | Done | `src/layout.rs` facade and conversion tests. | Internals and legacy helpers still expose Taffy for migration and advanced use. |
| Localization, RTL/text-direction, and dynamic labels have an explicit support path and regression coverage. | Partial | `src/i18n.rs` policy and unit tests. | Widgets/renderers do not yet apply these policies end to end. |
| Font, icon, image, and texture lifecycle behavior is documented and tested. | Partial | `src/assets.rs`, renderer resource descriptors, WGPU resource paths. | Font registry and texture lifecycle policy are incomplete. |
| Renderer/performance tests cover interaction-heavy frames, large resources, and native-surface rendering where available. | Partial | `tests/perf_smoke.rs`, CPU snapshots, WGPU parity tests. | Native-surface and large resource stress coverage remain limited. |
| Advanced scrolling handles nested arbitration, anchoring, sticky/fixed content, kinetic behavior, scrollbars, reveal-into-view, and synchronized surfaces. | Partial | `ScrollState`, reveal-into-view helpers, shell scroll sync. | Nested arbitration, anchoring, kinetic scrolling, sticky/fixed semantics, and scrollbars remain gaps. |
| Layering/compositing semantics are explicit for stacking contexts, transforms, opacity groups, offscreen layers, clipping, masks, and transformed hit testing. | Partial | Platform layer order, z-index ordering, simple animation transform hit testing. | Stacking contexts, opacity groups, masks, and transformed clipping are not complete. |
| Compositor-quality rendering covers shadows, rounded clipping, borders, gradients, masks, filters, subpixel text, and parity for composited content. | Partial | Paint primitives, gradients, paths, CPU/WGPU renderers. | Shadows, masks, filters, rounded clipping parity, and color policy need hardening. |
| Diagnostics can explain input routing, widget actions, overlay state, accessibility output, and render timing in one debug surface. | Partial | `src/debug.rs`, testing assertions, render timing data. | Widget actions and overlay state are not yet part of one debug surface. |
| Theme/design-token APIs and feature stability are documented for v5 consumers. | Partial | `src/theme.rs`, `src/versioning.rs`. | Stability annotations and feature policy docs are incomplete. |
| Async task state, loading/progress, cancellation, async validation, and repaint scheduling are covered by tests. | Gap | Platform repaint request contracts. | No Operad-owned async task model. |
| Virtualized list/table/tree/grid behavior handles huge datasets, measured row heights, sticky regions, selection, focus, and accessibility. | Partial | Data widget virtual table helpers. | General virtualization model and accessibility stability are incomplete. |
| Multi-window and multi-document routing is proven for focus, IME, overlays, cursor, accessibility, and render surfaces. | Gap | Host request/response contracts are document-scoped. | No multi-window runtime/state routing proof. |
| Touch, stylus, and gamepad input have explicit routing and regression tests. | Partial | Pointer kind metadata exists. | Touch gesture, stylus metadata, and gamepad routing are not complete. |
| Tooltip, help, and context menu policy is centralized and accessible. | Partial | `src/tooltips.rs`, menu extension helpers. | Context menu policy and overlay integration remain incomplete. |
| Scheduler/frame lifecycle behavior is deterministic in tests and prevents unbounded repaint loops. | Partial | Host frame request/output and testing harness timing. | Full scheduler, timers, idle work, and repaint coalescing policy are gaps. |
| Error boundaries, resource limits, and malformed input handling are documented and tested. | Partial | Renderer/resource validation paths and audit warnings. | Resource budget/security policy and local widget failure boundaries are incomplete. |
| CI/release automation covers feature matrix, docs, semver review, and package dry runs. | Gap | Cargo feature flags and tests exist. | No release automation or semver review automation in repo. |
| API docs explain core concepts, lifecycle, ownership, and migration path. | Partial | Existing roadmap and migration docs plus this audit. | Concept reference docs still need to be written and linked. |

## Audit Notes

- The first slice intentionally avoided actions, compositor internals, scrolling
  behavior changes, and host runtime internals.
- `LayoutStyle` remains the compatibility bridge to Taffy. New public code
  should prefer `operad::layout::Layout` and related Operad-owned primitives for
  common layout construction.
- The v5 release gate remains mostly partial. The new artifacts make the public
  API/docs/versioning direction explicit, but they do not complete the broader
  interaction runtime.
