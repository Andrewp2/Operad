# Changelog

## Unreleased

- Share document preparation, input routing, frame processing, and runtime state
  retention between native and web hosts. Focus, gestures, IME targets, and
  canvas captures follow stable node paths across rebuilds; removing targets
  cancels their runtime ownership.
- Retain documents and layout across redraws, rebuild after application updates
  and viewport changes, and consume resource uploads only after presentation.
  Frame-owned tooltips are removed before retaining the authored document.
- Move invalidation and timing primitives into `core`, and make subsystem
  modules own their implementations instead of re-exporting root declarations.
- Make `diagnostics`, `inspector`, and `test-support` opt-in features. The native
  showcase now requires `--features inspector`; `web-showcase` enables it.
- Consolidate equivalent inspector option and node types into shared panel
  contracts. Panel functions now return `DiagnosticPanelNodes` where they expose
  a root and a row container, and use common timeline, source, record, candidate,
  text, property, node, change, and issue options where their contracts match.
- Import `EmptyResourceResolver` from `renderer`, timing primitives from `core`
  or the crate root, and theme stability metadata from `theme::stability`.

## 9.0.1 - 2026-05-22

- Added direct app-owned WGPU view composition with explicit load/clear
  behavior, capability reporting, caller-owned GPU timing tokens, and
  discard-path documentation for legacy encoder rendering.
- Added shared font injection for the WGPU renderer and `CosmicTextMeasurer`
  through a backend-neutral font library so layout and rendered glyphs can use
  the same app fonts.
- Added a public empty resource resolver for rect/text-only render paths and
  documented resolver/update requirements for images, textures, thumbnails, and
  canvas resources.
- Fixed Shader Lab material-effect layout so painted outsets do not overlap
  controls, shortened visible showcase copy, restored canvas demo dragging, and
  added default split-pane drag-handle cross-axis margins.

## 9.0.0 - 2026-05-21

- Added element material and shader-effect support for UI surfaces, including
  shader-backed canvas, frame, and button previews, paint outsets, clip and hit
  shape controls, geometry-effect declarations, and renderer-safe fallback
  reporting for invalid WGSL.
- Added the Shader Lab showcase for editable WGSL programs, program presets,
  material contract controls, canvas/frame/button targets, and live validation
  without crashing the host when shader compilation fails.
- Hardened popup, dropdown, menu, tooltip, modal, and portal stacking so
  anchored surfaces are layered relative to their owning window and stay above
  later siblings without escaping unrelated floating windows.
- Reworked scroll containers and scrollbars with automatic visibility,
  reserved scrollbar gutters, hover/active visual states, draggable thumbs,
  ctrl/shift horizontal scrolling, end-position tests, and range epsilon fixes.
- Improved text editing and selection ownership, including single active text
  selection, tighter caret and selection geometry, focus blur behavior,
  multiline newline handling, and scrollable code/text editor regions.
- Expanded media and image resources with user-supplied image handles, decoded
  PNG/JPEG/BMP assets, clearer missing-image rendering, icon image variants,
  and showcase coverage for image-backed checkbox marks and media grids.
- Added indeterminate checkbox state, richer radio/toggle customization,
  editable numeric inputs with unit-specific ranges, date range picking,
  progress logs, theme switching, panel resizing, timeline scrolling, editable
  trees, and easing demos.
- Refined showcase layout and organization behavior so windows start closer to
  content size, can be packed/minimized when organized, and keep regressions in
  external tests instead of `examples/showcase.rs`.
- Cleaned up all-target Cargo warnings and extended visual/layout tests for the
  v9 widget surfaces.

## 8.0.1 - 2026-05-18

- Fixed text input selection geometry on the web by using the same non-wrapping
  shaped text layout for paint, caret placement, and selection highlights.
- Aligned the direct `cosmic-text` dependency with `glyphon` so browser and
  native text measurement use the same shaping engine.

## 8.0.0 - 2026-05-18

- Added application-shell APIs for command registries, scoped hotkeys,
  shortcut remapping, command diagnostics, and command palette examples.
- Added the animation state-machine runtime with inputs, triggers, blend
  bindings, topology morph values, debug graph views, and showcase examples for
  timed, scrubbed, boolean, and interaction-driven animation.
- Added host capability profiles and backend diagnostics for native, web, and
  test hosts so applications can gate hotkeys, text editing, canvas editing,
  flycam input, docking, drag/drop, and accessibility behavior explicitly.
- Expanded canvas input support with key release, raw mouse motion, cursor
  capture requests, host-capture diagnostics, WGPU canvas callbacks, and replay
  coverage for editor-style and flycam-style interactions.
- Added reusable diagnostics for layout, paint, intrinsic sizing, accessibility,
  command routing, theme editing, animation, runtime errors, platform responses,
  virtualization, and performance summaries.
- Added interaction recording and replay infrastructure, scenario harnesses,
  long-wheel helpers, topmost input-consumption assertions, and external
  showcase regression tests.
- Added virtualized tree and data-table APIs with stable focus preservation,
  selection/export/sort/filter/resize metadata, sticky regions, diagnostics,
  and showcase coverage.
- Added docking workspace primitives with split panes, drawers, drawer rails,
  persisted layout snapshots, panel reorder targets, floating panel state, and
  domain-neutral dock drag/drop contracts.
- Added layout animation transition records and reduced-motion-aware paint-list
  interpolation while keeping layout output authoritative.
- Hardened widget layout, scroll, overlay, text input, selection, scrollbar,
  color picker, styling, forms, media, drag/drop, and showcase behavior through
  generic primitive fixes instead of showcase-only sizing patches.
- Added the WASM/WebGPU showcase path for GitHub Pages, including root and
  `/showcase/` artifacts, favicon packaging, browser smoke checks, and web
  startup error reporting.
- Documented v8 API stability categories and the reusable release process for
  future Operad releases.

## 7.0.0 - 2026-05-14

- Started the v7 source-organization pass by moving retained document
  primitives into `src/core/document.rs` and widget module wiring into
  `src/widgets/mod.rs`.
- Added `operad::prelude` as the preferred broad import surface for application
  code.
- Added v7 roadmap, migration-guide, and release-checklist drafts so API moves,
  compatibility aliases, widget parity work, and release gates are tracked as
  work happens.
- Carried the released `6.1.0` WGPU baseline forward, including `wgpu` 29.0.3
  and `glyphon` 0.11.0.
- Moved widget-gated tests out of `core::document` and into the widget module
  so the first source split also moves module ownership for widget behavior.
- Added v7 widget builders for text-style labels, standalone images,
  separators/spacers, spinners, radio buttons/groups, toggle switches, visual
  drag values, generic grids, and panel containers so examples can use normal
  Operad primitives instead of hand-building those nodes.
- Added renderer-neutral performance diagnostics that name frame pipeline stages
  and aggregate cache hit, miss, and eviction rates from display-list reuse
  reports.
- Added v7 overlay/widget builders for collapsing headers, tooltip boxes, and
  modal dialogs so examples and applications can use normal Operad surfaces
  instead of hand-building those nodes from low-level containers.
- Added link, hyperlink, and selectable-label builders so interactive text
  controls are available separately from selectable read-only text input.
- Added text-input convenience builders for single-line input, multiline input,
  text areas, code editors, search boxes, and password fields.
- Added generic drag/drop source and drop-zone builders backed by the existing
  drag/drop descriptors and platform drag-start requests.
- Added small, icon, image, toggle, and reset button convenience builders on top
  of the default button primitive.
- Added form section, row, field label, help text, validation message, and error
  summary widget helpers backed by the existing form validation contracts.
- Added container helpers for panels, frames, groups, sides, columns,
  indentation, resize handles, and resize containers.
- Added compact color button, color swatch button, color-format display, and
  RGB/RGBA/SRGB/SRGBA/HSVA/OKLCH color-edit button helpers.
- Added menu-button state, trigger builders, image menu buttons,
  image-and-text menu buttons, submenu item helpers, and anchored submenu popup
  composition.
- Added selectable-value and angle-drag helper builders over the existing
  selectable-label and drag-value primitives.
- Added color conveniences for premultiplied/unmultiplied RGBA and
  SRGBA buttons, `Color32`-style color buttons, and standalone HSV 2D picker
  fields.
- Added `area` and `scene` widget helpers so absolute-positioned regions and
  scene primitives can be built through the normal public widget API.
- Added widget interaction helpers for visible/enabled UI blocks, fixed/minimum
  allocations, scene painter allocations, and programmatic scrolling.
- Added global theme preference button-group and switch builders so applications
  can expose System/Light/Dark theme controls without showcase-specific code.
- Added form action buttons plus field-order traversal helpers for
  submit/apply/cancel/reset workflows.
- Added tooltip trigger resolution and tooltip animation presets to the widget
  layer.
- Added modal dialog dismissal policy, focus-trap helpers, and overlay-frame
  event helpers to the visual modal builder.
- Added drag-image policy and drop-preview state helpers to the drag/drop widget
  layer.
- Added a composed scroll area with aligned vertical/horizontal scrollbar
  helpers.
- Added required frame-stage and cache-domain performance diagnostics to the
  unified diagnostic report surface.
- Documented v7 performance stress probes, budgets, and repeatable smoke
  commands outside the showcase.
- Trimmed package contents so old internal planning docs are not shipped as
  current crate guidance.
- Added a v7 showcase audit covering public API use, hidden harness removal,
  widget state coverage, text input, scrollbar, and overlay validation.

## 6.1.0

- Updated the optional WGPU stack to `wgpu` 29.0.3 and `glyphon` 0.11.0 so
  native-window and GPU canvas consumers can share the same WGPU version as
  downstream renderers that have already moved past WGPU 25.
- Adjusted the WGPU adapter for WGPU 29 surface acquisition, pipeline layout,
  render pass, polling, sampler, and glyphon text-buffer APIs.

## 6.0.0

- Added a native window runner so simple apps can open a window with
  `run_ui_document` or `run_app` instead of wiring winit and WGPU by hand.
- Added a widget library for common controls, including buttons, checkboxes,
  sliders, text input, selection controls, menus, date and color pickers,
  lists, tables, trees, floating windows, toasts, popup panels, and canvas
  surfaces.
- Added app-owned WGPU canvas rendering. Applications can declare a GPU canvas
  in the UI tree, register a renderer with `NativeWgpuCanvasRenderRegistry`,
  and record their own WGPU work into the canvas texture before Operad
  composites the UI.
- Added `WgpuCanvasContext` and `WgpuCanvasRenderPass` under the `wgpu` feature.
  `WgpuCanvasContext::render_pass` remains a convenience helper for simple WGSL
  passes; apps can also create command encoders and render passes directly.
- Added OKLCH support to the color picker and fixed hue endpoint handling so
  dragging hue to the far right no longer snaps the control back to the left.
- Removed the transient WebGL/WebGL2 naming from the canvas API; the public
  surface uses native GPU and WGPU terminology.
- Moved the source tree toward the v6 module organization while preserving
  common v5 public compatibility paths for downstream consumers.
- Updated README, migration notes, and release validation guidance for the v6
  public API.

## 5.0.0

- Added Operad-owned public layout primitives for common API use, with conversion
  paths back to `LayoutStyle` and Taffy for migration and advanced cases.
- Added localization and internationalization policy types for locale identity,
  text direction, bidi behavior, layout mirroring, and dynamic label metadata.
- Carried localization policy through text content, localized labels, paint
  output, and accessibility metadata.
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
- Added an optional `accesskit-winit` adapter that converts Operad
  `AccessibilityTree` output into AccessKit tree updates and publishes it
  through winit hosts.
- Added touch/stylus/gamepad routing, multi-window routing, navigation/overlay
  contracts, virtualization planning, tooltip/help/context menu policy, unified
  diagnostics, and theme/design-token stability documentation.
- Updated consumer-style probes and render tests to use the new public
  conversion helpers where direct backend layout fields are not required.
- Documented the v5 completion audit, migration posture, release checklist, and
  CI/release gates for fmt, feature-matrix checks, docs, examples, package
  verification, WGPU validation, perf smoke, and semver review.
- Added a bounded native WGPU window smoke mode to `native_wgpu_host` so release
  validation can prove OS-surface presentation without snapshot readback while
  keeping display-dependent execution opt-in.
- Added CPU/WGPU rich-rect gradient rendering coverage and corrected compositor
  quality profiles so fallback and unsupported effects are not overclaimed.
- Added explicit composited paint layers with CPU snapshot reference rendering
  and WGPU render-to-texture composition for rounded clips, rectangular masks,
  opacity, and basic brightness/contrast/saturate/blur filters.
- Added CPU/WGPU soft rich-rect shadow falloff coverage and compositor quality
  planning for basic native shadow blur limits.
- Added WGPU parity coverage for glyphon text inside composited paint layers
  and release-gated the sRGB-only snapshot color-management policy.
- Added explicit compositor quality fallback records for backdrop filters, which
  remain disabled until a backend samples the already-composited framebuffer.
- Added lyon-backed path fill/stroke tessellation with even-odd holes, curved
  concave fills, configurable stroke caps/joins, and WGPU parity coverage.
- Added WGPU parity coverage for fractional grayscale glyph positioning without
  claiming RGB/LCD component subpixel masks for v5.
- Expanded the native WGPU host example document to include buttons, text input,
  popup/menu items, a drag-handle target, and a canvas viewport.

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
- Removed the legacy compatibility painter path from the active renderer
  backend surface.
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
