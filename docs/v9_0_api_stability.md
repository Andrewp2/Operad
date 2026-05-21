# Operad 9.0 API Stability

Operad v9 turns the v8 showcase and diagnostics work into a broader product
surface for shader-backed UI, stronger text editing, anchored overlays,
scrolling, media resources, and richer widget composition. This guide separates
stable product APIs from diagnostics, tests, backend-specific hooks, and
migration-only surfaces.

The stability categories are the labels exposed by `ApiStability`,
`StabilityNote`, and `FeatureStability`:

- Stable: semver-protected public API for compatible v9 releases.
- Experimental: public integration points that may add fields or presets during
  v9 as renderer and devtool behavior matures.
- Backend-specific: public API whose behavior depends on renderer, native
  window, browser, OS, or available feature flags.
- Migration-only: compatibility support for older integrations, not the
  preferred v9 surface for new code.

## Stable Product APIs

These surfaces are intended for application code and should be treated as the
preferred v9 product API unless a type documents otherwise.

- Retained document, layout, intrinsic sizing, input, actions, focus,
  accessibility metadata, paint lists, render-frame contracts, and the
  size-position-draw layout pipeline.
- Widget composition APIs under `operad::widgets`, including buttons, labels,
  text inputs, numeric inputs, sliders, checkboxes, radio buttons, toggles,
  scroll containers, overlays, menus, forms, color pickers, tree views, data
  tables, panels, timeline controls, media widgets, and reusable shell widgets.
- Unified popup stacking and portal ownership for dropdowns, menus, tooltips,
  modals, popups, and anchored overlays. Popups should be positioned relative
  to their anchor and stacked in the nearest owning window or app overlay.
- Scroll Containers and scrollbar behavior, including automatic visibility,
  reserved gutter measurement, hover and active scrollbar states, draggable
  thumbs, ctrl/shift horizontal scroll routing, and content-size based range
  calculation.
- Text Input And Selection behavior, including single active selection
  ownership, caret geometry, selectable read-only text, multiline editing,
  newline handling, text focus/blur, scrollable editors, and copy/paste
  semantics.
- Image And Media Resources, including stable resource keys, built-in icons,
  user-supplied image handles, decoded PNG/JPEG/BMP images, image-backed
  checkbox marks, missing-image rendering, and tintable image widgets.
- Theme and design-token model, including scoped themes, component state
  resolution, checkbox/checkmark contrast, accessible theme preference controls,
  theme patching, and app-wide theme switching.
- Element material and shader-effect declarations for UI surfaces, including
  material shader presets, clip/hit shape descriptions, paint outsets, geometry
  effect metadata, and shader-backed canvas/frame/button paint records.
- Command model: `CommandRegistry`, `Command`, `CommandMeta`, command effects,
  shortcut scopes, shortcut conflict checks, `ShortcutRemap`, and command
  bindings.
- Animation runtime data model: `AnimationMachine`, inputs, transitions,
  triggers, blend bindings, animated values, topology morph values, easing
  presets, and state-machine driven interaction glue.
- Canvas content and interaction policy records, including host-capture plans,
  raw pointer/key/wheel input shape, aspect-ratio sizing, and renderer-neutral
  canvas render requests.
- Host capability records and product-level `BackendCapabilityProfile` checks,
  so apps can gate command hotkeys, text editing, canvas editing, flycam input,
  dock workspaces, OS/platform drag-drop, and accessibility support.
- Runtime platform service APIs: `PlatformRequest`, `PlatformServiceRequest`,
  `PlatformServiceResponse`, `PlatformServiceClient`, and native/web runtime
  hooks for draining app-owned requests and delivering clipboard, open-URL,
  cursor, repaint, and other service responses.
- Virtualization planner records for list, table, tree, and grid surfaces,
  including virtual ranges, measured extents, sticky regions, focus/selection
  preservation, stable tree focus by item ID, scroll-anchor adjustment, and
  `VirtualizationBudget`.
- Dock workspace state and layout snapshot records for side, size, visibility,
  split panes, panel resize handles, panel reorder targets, focus restoration,
  persisted editor layouts, host-resolved drop-hit commit helpers,
  domain-neutral dock panel drag/drop targets, and floating-panel state.

Stable does not freeze exact pixels, colors, font metrics, or backend-specific
fallback choices. It means the public concepts and record fields are part of
the v9 contract and should evolve compatibly.

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
  navigation trace records for inspecting roles, labels, focus order, modal
  scopes, actions, relations, and command shortcut routing.
- Shader Lab And Element Materials diagnostics, including WGSL source previews,
  shader compilation status, material preview panels, shader preset selection,
  geometry-effect visualization, and backend fallback messages.
- `DebugAnimationGraph`, animation graph panels, active transition inspection,
  blend-binding views, easing graph views, and generic animation inspector
  controls for transport, scrub, input editing, and trigger-fire action
  surfaces.
- `command_diagnostics_panel` and command registry diagnostic views for
  conflicts, disabled commands, shortcut displays, and command metadata.
- `ThemePatchExport`, theme editor panels, theme accessibility audit summaries,
  and debug theme snapshots.
- `CanvasHostCaptureDiagnosticReport`, capture lifecycle diagnostics, denied
  cursor-response reports, and host capability explanations.
- `ErrorReport`, render/platform error classifiers, platform-service response
  classifiers, web startup reports, and runtime error overlays.
- `PopupLayout::diagnostic_summary` and placement fields for inspecting overlay
  side, flip, clamp, and viewport-overflow decisions.
- `VirtualizationDiagnostics`, materialization budget reports, overscan/range
  summaries, and large-data performance risk messages.
- `layout_animation_transitions`,
  `apply_layout_animation_transitions_to_paint_list`, and layout animation
  transition records.

Experimental diagnostics may add fields or richer classifications in minor v9
releases. Prefer to persist app-owned settings or domain state, not raw
diagnostic snapshots.

## Testing And Replay Helpers

The replay and scenario utilities are public because they are valuable to
consumers, but they are primarily test infrastructure:

- `InteractionRecorder`, `EventReplay`, direct command replay, long wheel
  scroll helpers, topmost input-consumption assertions, platform-response
  replay, and window-resize replay.
- `ScenarioHarness`, frame reports, render assertions, layout assertions,
  scroll/focus assertions, snapshot helpers, visual snapshot helpers, and
  performance sample assertions.
- Widget state matrix helpers, no-clipping checks, synthetic scroll tests,
  showcase regression tests outside `examples/showcase.rs`, and browser smoke
  checks for the GitHub Pages showcase.

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
- `wgpu`, WGPU canvas render passes, shader-backed element materials, surface
  rendering, glyphon text paths, GPU resource uploads, timestamp query reports,
  and adapter-dependent feature fallbacks.
- `web-runtime`, WebGPU startup, browser canvas integration, panic/error
  reporting, and Pages-hosted showcase bootstrapping.
- `image-decode`, decoded PNG/JPEG/BMP resource loading through the optional
  image feature.
- `accesskit-winit`, AccessKit tree publication, platform focus, and host-owned
  accessibility action routing.
- `egui` and `egui-renderer-compat`, which remain compatibility surfaces for
  older renderer integrations rather than the preferred v9 renderer path.

Backend-specific APIs should be wrapped by app host adapters. Product logic
should depend on renderer-neutral records where possible.

## Migration-Only Surfaces

Migration-only APIs exist to keep existing applications moving while the v9
contract settles. New integrations should not add product dependencies on
legacy renderer compatibility, direct Taffy compatibility shims, or older egui
adapter behavior unless those dependencies are isolated behind local adapters.

## Consumer Guidance

Use stable product APIs for app state and public integration points. Use
experimental diagnostics to debug, inspect, test, and build devtools. Use
testing and replay helpers to preserve regressions, not as production event
logs. Keep backend-specific behavior in host adapters, and avoid persisting raw
diagnostic reports unless the application owns a versioned wrapper.
