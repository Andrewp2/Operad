# Operad 8.0 Completion Audit

This audit maps the v8 roadmap to current repository artifacts. It is the
release-readiness checklist for the v8 branch.

Status key:

- `Done`: reusable API, documentation, and regression coverage exist for the
  roadmap item.
- `Partial`: reusable pieces exist, but the full roadmap behavior still needs
  more product integration or examples.
- `Gap`: no complete Operad-owned artifact exists yet.

## Roadmap Themes

| Theme | Status | Current artifact | Remaining gap |
| --- | --- | --- | --- |
| Live layout inspector | Done | `DebugInspectorSnapshot`, `debug_inspector_panel`, layout/paint/intrinsic-size/debug hit traces, host input-consumption state, accessibility summaries, and the showcase Diagnostics window. | Keep adding specialized rows as new layout primitives land. |
| Animation state-machine inspector | Done | `AnimationMachine`, blend inputs, topology morph values, `DebugAnimationGraph`, animation graph panels, reusable animation inspector controls for pause/step/scrub/input/trigger actions, showcase timed/scrubbed/bool/interaction examples, Diagnostics-owned control mutation state, and layout/paint/replay tests. | Keep adding specialized inspectors as new animation primitives land. |
| Command palette and hotkeys | Done | `CommandRegistry`, scoped shortcuts, `ShortcutRemap`, command palette history, command diagnostics panel, direct command replay, and showcase examples. | Downstream apps still own command namespaces and product command effects. |
| Canvas input capture | Done | Key release/raw motion/canvas capture state, cursor requests/responses, canvas capture diagnostics, native hooks, WGPU canvas callbacks, and replay coverage. | OS support remains backend-specific and must be surfaced by host adapters. |
| Host capability matrix | Done | `BackendCapabilities::native_window()`, `BackendCapabilities::web_runtime()`, `test_host_capabilities()`, product-level `BackendCapabilityProfile` diagnostics, and matrix tests cover native, web, and deterministic test-host support/fallback decisions for hotkeys, text editing, canvas editing, flycam, dock workspaces, OS/platform drag-drop, and accessibility. Native IME requests now route through winit instead of being advertised as supported but returned as `Unsupported`; OS data drag-drop remains a separate unsupported capability until a real backend adapter exists. | Backend adapters still need to report OS-specific denial responses at runtime. |
| Runtime errors and recovery | Done | `ErrorReport`, render/platform classifiers, `classify_platform_service_response`, `PlatformAssertions` failure messages, runtime error overlays, native startup reports, web startup status, invalid-node panic context, and smoke tests cover unsupported, denied, recoverable, and fatal runtime failures. | Keep adding classifiers for new backend error families as they are introduced. |
| Interaction recorder | Done | `InteractionRecorder`, `EventReplay`, command/platform-response/window-resize replay, long-wheel helpers, input-consumption assertions for topmost wheel routing, `ScenarioHarness`, and showcase regression tests outside `examples/showcase.rs`. | Persisted replay file formats remain app-owned. |
| Theme and style editor | Done | `DebugThemeSnapshot`, `theme_editor_panel`, `ThemePatchExport`, component-state rows, contrast/preference audits, and showcase Diagnostics examples. | Product-specific theme presets remain app-owned. |
| Virtualized tree and table 2.0 | Done | Shared virtualization planner, diagnostics/budgets, virtualized tree view, stable tree focus-preservation records, showcase tree-table pattern, virtual data table selection/export/sort/filter/resize/sticky metadata, showcase sort/filter/selection/scroll/resize flows, and performance tests. | Product-specific row models still own their filtering and persistence policies. |
| Layout animations | Done | `layout_animation_transitions` computes post-layout visual interpolation records while keeping layout authoritative, `apply_layout_animation_transitions_to_paint_list` applies them to paint output, and document frames now carry previous/current layout snapshots plus reduced-motion-aware transition output. | Keep adding higher-level easing presets for specific product transitions. |
| Accessibility debug overlay | Done | `accessibility_debug_overlay`, accessibility overlay panel, keyboard navigation trace, focus/action/relation diagnostics, contrast audits, and showcase Diagnostics examples. | Backend publication remains adapter-specific. |
| Docking workspace | Done | `dock_workspace`, split panes, persisted `DockWorkspaceLayoutSnapshot`, panel resize handles, drawer visibility state, drawer rail widgets, panel reorder targets, domain-neutral dock panel drag/drop targets, drop-hit commit helpers on `DockWorkspaceState`, host-style drag source/drop target tests, and a showcase layout window with docked inspector/assets panels, tabbed center content, drawer toggles, reorder target previews, and a floating inspector flow. Dock-workspace capability now depends on local pointer/overlay support rather than OS data-drag support. | Richer OS-integrated drag sessions remain backend-specific, but the toolkit-owned dock state and target contract are covered. |

## Release Gates

| Release gate | Status | Current artifact | Remaining gap |
| --- | --- | --- | --- |
| Showcase examples for each major feature, using public APIs only | Done | Showcase has Diagnostics, Animation, Command palette, Trees, Canvas, Styling, virtualized table sort/filter/resize flows, tree-table/focus-preservation coverage, a stronger dock workspace with drawers/reorder targets/floating state, and widget regression windows. | Keep future showcase regressions covered by external tests. |
| Browser showcase runs as WASM/WebGPU | Done | The Pages workflow builds `showcase_web`, assembles `/` and `/showcase/` artifacts, serves the artifact locally, and runs `scripts/check-web-showcase.mjs` against both routes in headless Chrome with WebGPU enabled to catch startup panics, console errors, missing assets, and shader warnings. The web runtime uses the same cosmic-text measurement path as the native runner so browser layout is not based on approximate text widths. | Keep the smoke script focused on browser-only failures that cargo checks cannot see. |
| Replay/state-matrix tests for interactive behavior | Done | `tests/showcase_layout.rs`, widget state matrix tests, the per-window showcase audit matrix for normal, collapsed, scrolled, focused, hovered, and narrow states, replay helpers, topmost input-consumption checks, popup placement diagnostics for flip/clamp decisions, visible-scrollbar-without-range audit diagnostics, Just Work audit summaries with reasons/measurements/hints, and scenario harness tests cover scroll, resize, command, text input, animation, and performance regressions. CI runs the focused showcase regression filter so these gates are enforced without requiring a full all-feature test sweep. | Continue adding focused replay cases for every bug found in showcase. |
| Layout/a11y/performance diagnostics for new surfaces | Done | Debug inspector, accessibility overlay, virtualization diagnostics, dominant frame-timing summaries, performance snapshots, and perf smoke tests cover the v8 diagnostic surfaces. CI runs the non-WGPU performance smoke target, while WGPU/native-host perf evidence remains opt-in for environments with a compatible device/display. | Native-host perf evidence remains opt-in because CI may be headless. |
| No tests embedded in showcase source | Done | Showcase regression tests live in `tests/showcase_layout.rs`; `examples/showcase.rs` remains a teaching example. | Keep new showcase regressions out of the example file. |
| No app-specific domain semantics in public APIs | Done | New v8 APIs use command, layout, animation, canvas, theme, data, docking, and diagnostic terms rather than product-specific domains. | Audit new examples when adding richer docking/table demos. |
| Package docs distinguish stable APIs from diagnostics | Done | `docs/v8_0_api_stability.md` classifies stable product APIs, experimental diagnostics/devtools, testing/replay helpers, backend-specific APIs, and migration-only surfaces. | Keep the guide current as partial roadmap areas become stable. |

## Next Work

The v8 roadmap gates are covered. Remaining work should be treated as release
polish: run the full suite, audit the public API naming surface, and keep any
new showcase regressions in external tests.
