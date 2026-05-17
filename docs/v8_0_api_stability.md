# Operad 8.0 API Stability

Operad v8 adds application-shell, diagnostics, animation, and dense-data
surfaces on top of the existing retained document, layout, paint, theme, and
runtime contracts. This document separates the surfaces intended as product API
from the helpers that are deliberately diagnostic, testing, backend-specific, or
early integration points.

The stability categories are the same labels exposed by
`ApiStability`, `StabilityNote`, and `FeatureStability`:

- Stable: semver-protected public API for compatible v8 releases.
- Experimental: available to build tools and early integrations, but field sets
  and widget composition may expand during v8.
- Backend-specific: public API whose behavior depends on renderer, native
  window, OS, or feature support.
- Migration-only: compatibility support for older app integrations, not the
  preferred v8 surface for new code.

## Stable Product APIs

These surfaces are intended for application code and should be treated as the
preferred v8 product API unless a specific type documents otherwise.

- Retained document, layout, intrinsic sizing, input, actions, focus,
  accessibility metadata, paint lists, and render-frame contracts.
- Theme and design-token model, including scoped themes, component state
  resolution, accessibility preference adjustments, and theme patching.
- Command model: `CommandRegistry`, `Command`, `CommandMeta`, command effects,
  shortcut scopes, shortcut conflict checks, `ShortcutRemap`, and command
  bindings.
- Widget composition APIs under `operad::widgets`, including buttons, labels,
  inputs, sliders, scroll containers, overlays, menus, color pickers, tree
  views, data tables, docking surfaces, and reusable shell widgets.
- Animation runtime data model: `AnimationMachine`, inputs, transitions,
  triggers, blend bindings, animated values, and topology morph values.
- Canvas content and interaction policy records, including host-capture plans,
  raw pointer/key/wheel input shape, and renderer-neutral canvas render
  requests.
- Host capability records and product-level `BackendCapabilityProfile` checks,
  so apps can gate command hotkeys, text editing, canvas editing, flycam input,
  dock workspaces, OS/platform drag-drop, and accessibility support without
  hand-copying low-level requirements. The built-in native, web, and test-host
  capability profiles are part of this stable contract.
- Runtime platform service APIs: `PlatformRequest`, `PlatformServiceRequest`,
  `PlatformServiceResponse`, `PlatformServiceClient`, and native/web runtime
  hooks for draining app-owned requests and delivering clipboard, open-URL,
  cursor, repaint, and other service responses.
- Virtualization planner records for list, table, tree, and grid surfaces,
  including virtual ranges, measured extents, sticky regions, focus/selection
  preservation, stable tree focus preservation by item ID, scroll-anchor
  adjustment, and `VirtualizationBudget`.
- Dock workspace state and layout snapshot records for side, size, visibility,
  split panes, drawers, drawer rails, panel reorder targets, focus restoration,
  persisted editor layouts, host-resolved drop-hit commit helpers, and
  domain-neutral dock panel drag/drop targets and floating-panel state.

Stable does not mean that default colors, exact spacing values, text metrics,
or backend pixels are frozen. It means the public concepts and record fields are
part of the v8 contract and should evolve compatibly.

## Experimental Diagnostics And Devtools

These APIs are public so applications can build inspectors, audits, and replay
harnesses, but they should not be exposed as downstream product data contracts
without a local compatibility wrapper.

- `DebugInspectorSnapshot`, layout/paint/intrinsic-size snapshots, hit traces,
  audit warnings, scroll and scrollbar-range diagnostics, and inspector panel
  widgets.
- `HostInteractionState` and `HostNodeInteraction` diagnostics for hovered,
  focused, captured, wheel-targeted, shortcut-targeted, and input-consumed
  state.
- `accessibility_debug_overlay`, `accessibility_overlay_panel`, and keyboard
  navigation trace records used to inspect roles, labels, focus order, modal
  scopes, actions, relations, and command shortcut routing.
- `DebugAnimationGraph`, animation graph panels, active transition inspection,
  blend-binding views, and generic animation inspector controls for transport,
  scrub, input editing, and trigger-fire action surfaces.
- `command_diagnostics_panel` and command registry diagnostic views for
  conflicts, disabled commands, shortcut displays, and command metadata.
- `ThemePatchExport`, theme editor panels, theme accessibility audit summaries,
  and debug theme snapshots.
- `CanvasHostCaptureDiagnosticReport`, capture lifecycle diagnostics, denied
  cursor-response reports, and host capability explanations.
- `ErrorReport`, render/platform error classifiers, platform-service response
  classifiers, and `runtime_error_overlay` for turning recoverable or fatal
  runtime failures into actionable in-app debug surfaces.
- `PopupLayout::diagnostic_summary` and its placement fields for inspecting
  overlay side, flip, clamp, and viewport-overflow decisions.
- `VirtualizationDiagnostics`, materialization budget reports, overscan/range
  summaries, and large-data performance risk messages.
- `layout_animation_transitions`,
  `apply_layout_animation_transitions_to_paint_list`, and layout animation
  transition records. Layout remains authoritative; these records are visual
  interpolation diagnostics and presentation helpers. Document-frame layout
  snapshot carry-forward, reduced-motion-aware transition output, and paint-list
  application are also experimental integration points while renderers adopt
  them.

Experimental diagnostics may add fields or richer classifications in minor v8
releases. Prefer to persist app-owned settings or domain state, not raw
diagnostic snapshots.

## Testing And Replay Helpers

The replay and scenario utilities are public because they are valuable to
consumers, but they are primarily test infrastructure:

- `InteractionRecorder`, `EventReplay`, direct command replay, long wheel
  scroll helpers, topmost input-consumption assertions, platform-response
  replay, and window-resize replay.
- `ScenarioHarness`, frame reports, render assertions, layout assertions,
  scroll/focus assertions, snapshot helpers, and performance sample assertions.
- Widget state matrix helpers and showcase regression tests outside
  `examples/showcase.rs`.

These APIs are expected to remain source-compatible where practical, but their
assertion coverage and report fields may grow as new regressions are preserved.
Downstream libraries should avoid treating replay report structs as persistent
file formats.

## Backend-Specific APIs

Backend-specific APIs are public only when their features are enabled and when
the host environment supports them:

- `native-window`, winit event-loop integration, native hooks, cursor grab,
  cursor visibility, raw device motion, window metrics, and app tick/redraw
  hooks.
- `wgpu`, WGPU canvas render passes, surface rendering, glyphon text paths,
  GPU resource uploads, timestamp query reports, and adapter-dependent feature
  fallbacks.
- `accesskit-winit`, AccessKit tree publication, platform focus, and host-owned
  accessibility action routing.
- `egui` and `egui-renderer-compat`, which remain compatibility surfaces for
  older renderer integrations rather than the preferred v8 renderer path.

Backend-specific APIs should be wrapped by app host adapters. Product logic
should depend on renderer-neutral records where possible.

## Migration-Only Surfaces

Migration-only APIs exist to keep existing applications moving while the v8
contract settles. New integrations should not add new product dependencies on
legacy renderer compatibility, direct Taffy compatibility shims, or older egui
adapter behavior unless they are isolated behind local adapters.

## Consumer Guidance

Use stable product APIs for app state and public integration points. Use
experimental diagnostics to debug, inspect, test, and build devtools. Use
testing and replay helpers to preserve regressions, not as production event
logs. Keep backend-specific behavior in host adapters, and avoid persisting raw
diagnostic reports unless the application owns a versioned wrapper.
