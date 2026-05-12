# Operad 5.0 Roadmap

Operad `5.0.0` should turn the v4 renderer milestone into a complete
interaction runtime. V4 can render real UI quickly; v5 should make UI behavior
first-class, predictable, and easy to wire into applications.

Downstream applications still own product semantics. Operad should own the
reusable interaction, host, accessibility, rendering, and testing substrate.

The release-readiness status for each gate is tracked in
`docs/v5_0_completion_audit.md`; this roadmap remains the product/engineering
target list rather than the source of truth for what has landed.

## Release Goal

V5 should make Operad feel like a usable app UI toolkit rather than a document
and renderer layer. The main bar is that buttons, text fields, lists, popups,
drag interactions, canvas viewports, and shell surfaces can be built with one
consistent event/action model and one reusable host loop.

## 1. Widget Action And Command Events

Buttons and widgets currently expose focus, pressed, and clicked state, but app
code has to inspect `UiInputResult` directly. V5 should add a renderer-neutral
widget action/event model.

Targets:

- Add a first-class widget event queue for activation, selection, value preview,
  value commit, cancellation, open/close, drag begin/update/commit, and focus
  changes.
- Let widgets bind to neutral `CommandId`s or app-owned opaque action IDs without
  storing product-specific semantics in Operad.
- Make `widgets::button(...)` able to emit an activation event or command
  dispatch without custom hit-result plumbing at every call site.
- Route keyboard activation (`Space`, `Enter`) through the same activation path
  as pointer clicks.
- Add tests showing button activation, disabled suppression, keyboard activation,
  command routing, and action ordering across nested widgets.

## 2. Integrated Gesture And Drag Routing

V4 has `PointerGestureTracker`, drag/drop descriptors, and host interaction
state, but raw gesture routing is not yet integrated into the normal document
frame path.

Targets:

- Route raw pointer events through `PointerGestureTracker` inside the host frame
  pipeline.
- Preserve pointer capture across move/up/cancel even when the pointer leaves the
  original node.
- Emit drag begin/update/commit/cancel widget events with stable edit phases.
- Connect drag-source descriptors to platform drag requests and drop-target
  resolution.
- Add widget helpers for draggable rows, split panes, scrollbar thumbs, sliders,
  timeline/range editors, and canvas hit targets.
- Add regression tests for drag threshold, capture, cancellation, disabled
  targets, nested scroll/drag conflicts, and one-commit edit behavior.

## 3. Native Host Runtime

V4 includes WGPU rendering and host contracts, but consumers still need to wire a
lot of runtime behavior themselves. V5 should provide a reusable native host
runtime layer.

Targets:

- Provide a canonical app loop that handles window events, raw input conversion,
  document frames, render frames, repaint scheduling, and surface resize.
- Integrate `WgpuSurfaceRenderer` into a window-backed example/runtime path.
- Own platform service loops for cursor shape, cursor confinement, pointer lock,
  clipboard, text IME, drag/drop, repaint, and accessibility requests.
- Preserve deterministic headless tests while making the native runtime optional
  and feature-gated.
- Add a small example app that demonstrates button actions, text input, popup
  menus, drag, a canvas viewport, and WGPU window rendering.

## 4. Production Text Editing

Text input exists, but long editing sessions still need more complete behavior.

Targets:

- Add undo/redo primitives and edit history boundaries.
- Harden IME composition, commit, cancellation, and focus transfer.
- Improve multiline selection, word/line movement, selection extension, and
  pointer selection behavior.
- Cover clipboard edge cases: paste sanitization, multiline copy/cut, selection
  replacement, and disabled/read-only fields.
- Support validation, pending edits, commit/cancel phases, and accessible error
  summaries for form fields.
- Add performance tests for high-frequency text editing and large text fields.

## 5. State Binding And Widget Lifecycle

Operad should give applications a predictable state boundary. Widgets should be
easy to rebuild every frame without losing focus, scroll, animation, overlay, or
edit state, and without forcing app-specific state management into Operad.

Targets:

- Define controlled vs uncontrolled widget patterns for text fields, selects,
  sliders, forms, tables, trees, and popups.
- Add stable widget identity rules for focus persistence, animation persistence,
  overlay persistence, and scroll retention across rebuilds.
- Provide state handles or state maps for common widgets without requiring apps
  to inspect node internals.
- Connect widget state changes to dirty flags and retained display-list
  invalidation.
- Define synchronization rules between app state, widget-local transient state,
  preview edits, committed edits, and canceled edits.
- Add tests for rebuild stability, focus retention, scroll retention, animation
  continuity, and overlay lifecycle across document reconstruction.

## 6. Selection Models And Edit Transactions

Lists, tables, trees, timelines, forms, and canvas editors all need consistent
selection and transaction behavior.

Targets:

- Add reusable single, multi, range, active-item, anchor-item, and roving-index
  selection models.
- Support keyboard and pointer selection semantics for lists, trees, tables,
  menus, timelines, and canvas hit targets.
- Add edit transactions beyond text: drag edits, slider edits, numeric edits,
  property edits, table edits, selection changes, and command palette actions.
- Standardize preview/update/commit/cancel boundaries so high-frequency edits can
  preview continuously but commit once.
- Make undo/redo integration work against committed transactions, not every
  pointer move.
- Add tests for shift range selection, ctrl/cmd toggle selection, active-row
  movement, drag preview/commit, and canceled edits.

## 7. Forms And Validation

V4 has controls and form metadata. V5 should make forms a coherent workflow
instead of a collection of individual widgets.

Targets:

- Add field grouping, form sections, labels, descriptions, hints, errors,
  required state, dirty state, pending state, and disabled/read-only state.
- Add submit/apply/cancel/reset workflows with explicit validation phases.
- Support field-level and form-level validation messages with accessible error
  summaries.
- Add one-commit edit phases for property forms and inspector panels.
- Add reusable form navigation and keyboard semantics, including Enter/Escape
  behavior where appropriate.
- Add tests for validation lifecycle, dirty tracking, error summary generation,
  pending/apply/cancel flows, and accessibility metadata.

## 8. Unified Keyboard Navigation

Focus next/previous is not enough for dense app UI. V5 should define keyboard
navigation patterns that widgets can share.

Targets:

- Add roving tabindex/active-descendant behavior for menus, listboxes, tabs,
  trees, tables, radio groups, and toolbars.
- Define arrow-key behavior for horizontal, vertical, grid, table, tree, and
  menu navigation.
- Standardize Escape, Enter, Space, Tab, Shift-Tab, Home, End, PageUp, PageDown,
  and typeahead semantics.
- Integrate keyboard navigation with overlay dismissal, focus restore, selection
  models, and command scopes.
- Add tests for keyboard loops, disabled item skipping, nested popup escape
  behavior, table/grid navigation, and focus restoration.

## 9. Overlay, Popup, Menu, And Listbox System

Menus, selects, popovers, dialogs, and command palettes exist as pieces. V5
should make overlays one coherent runtime system.

Targets:

- Centralize overlay stack ownership, z-order, clipping, focus restore, and
  dismissal policy.
- Support outside-click dismissal, Escape dismissal, modal focus traps,
  non-modal popovers, nested menus, and command palettes through one model.
- Add active-descendant routing for listboxes, combo boxes, selects, and command
  palettes.
- Improve popup placement, flipping, clamping, scroll-container awareness, and
  viewport edge behavior.
- Add virtualized option lists, typeahead, keyboard search, and screen-reader
  metadata for options.
- Add tests for nested menus, popover focus restore, listbox keyboard routing,
  scroll clipping, and overlay/modal interactions.

## 10. Real Accessibility Adapters

Operad has strong metadata, audits, and adapter request contracts. V5 should
connect those contracts to real platform behavior where possible.

Targets:

- Implement at least one platform-backed screen-reader publication path.
- Deliver live-region announcements through host adapters that support them.
- Integrate focus trap/restore behavior with host windows and overlays.
- Apply high contrast, forced colors, reduced motion, reduced transparency, and
  text scale preferences consistently across widgets, themes, and renderers.
- Expose canvas/editor hit target summaries as navigable accessibility items.
- Add end-to-end adapter tests in addition to metadata/audit tests.

## 11. Layout Abstraction Cleanup

V4 still exposes Taffy concepts in public layout APIs. V5 should reduce backend
type leakage without making layout cumbersome.

Targets:

- Add Operad-owned dimension, inset, spacing, alignment, display, position, and
  flex types for public APIs.
- Keep conversion to and from Taffy for migration and advanced users.
- Add public query and builder methods for common layout inspection/mutation.
- Add layout serialization/debug output that does not expose backend internals.
- Keep the backend translation layer explicit so Taffy remains replaceable.
- Migrate widget options and tests toward Operad-owned layout types.

## 12. Localization, Text Direction, And Internationalization

Operad should not bake English-only or left-to-right assumptions into widgets,
layout, text editing, or accessibility.

Targets:

- Define string ownership and dynamic label update patterns for widgets and
  accessibility metadata.
- Support text direction, bidirectional text, and layout mirroring where the
  renderer and text stack allow it.
- Add locale-aware formatting hooks for numbers, units, dates, times, and
  validation messages without owning product-specific localization.
- Ensure keyboard navigation and popup placement handle right-to-left layouts.
- Add tests for dynamic labels, mirrored layout, bidi text measurement/rendering
  paths, and accessible names/descriptions.

## 13. Asset, Icon, Font, And Texture Pipeline

V5 should make assets and fonts predictable across CPU snapshots, WGPU, and host
renderers.

Targets:

- Add font loading, font fallback stacks, and explicit font registry lifecycle.
- Define DPI-aware icon/image variants and asset lookup policy.
- Expose texture cache policy, eviction, and versioning for WGPU resources.
- Improve built-in icon coverage and make icon-only controls easy to build
  accessibly.
- Add tests for font fallback, missing assets, high-DPI assets, partial texture
  updates, and texture cache eviction.

## 14. Renderer And Performance Hardening

The WGPU renderer is real and fast enough for v4. V5 should broaden coverage and
make performance easier to explain.

Targets:

- Add real native-surface examples and tests where available.
- Expand performance baselines for hover, drag, scroll, text edit,
  high-frequency playhead/meter updates, and large resource updates.
- Add partial-update/damage-region tests and document when a full repaint is
  required.
- Add large image/texture upload and partial texture update stress coverage.
- Add performance instrumentation that separates layout, input, paint build,
  resource upload, CPU render orchestration, and GPU render pass time.
- Keep CI thresholds stable while preserving a local high-confidence performance
  path for GPU-equipped machines.

## 15. Advanced Scrolling

Scrolling needs to behave predictably in nested, virtualized, editor-heavy
surfaces.

Targets:

- Define nested scroll arbitration, including when wheel, touchpad, drag, and
  keyboard scroll input transfer between parent and child regions.
- Add scroll anchoring so content insertion, removal, and lazy loading do not
  cause avoidable viewport jumps.
- Support sticky and fixed-position content inside scroll containers with clear
  paint, hit-test, and clipping semantics.
- Add kinetic scrolling, overscroll policy, scrollbar policy, and platform-aware
  input tuning.
- Cover scroll state persistence, programmatic scrolling, reveal-into-view, and
  synchronized scroll surfaces.
- Add deterministic tests for nested scrolling, virtualized lists, scrollbars,
  sticky headers, and text-editor scrolling.

## 16. Layering And Compositing Model

V5 should define how paint, hit testing, transforms, opacity, clipping, and
offscreen effects compose instead of leaving those rules implicit.

Targets:

- Add explicit stacking contexts, z-order rules, and layer promotion heuristics.
- Define transform origin, transformed bounds, transformed clipping, and
  transformed hit testing.
- Add opacity groups and offscreen layers for effects that require isolated
  composition.
- Define clip, mask, and scissor hierarchy semantics shared by CPU and WGPU
  backends.
- Ensure input routing, accessibility bounds, and diagnostics use the same
  effective geometry as the compositor.
- Add layer-tree inspection output and parity tests for overlapping,
  transformed, clipped, and semi-transparent UI.

## 17. Compositor-Quality Rendering Features

The renderer should cover normal application polish without forcing bespoke
canvas escapes for common effects.

Targets:

- Add high-quality shadows, elevation treatment, rounded clipping, and
  anti-aliased borders across CPU and WGPU paths.
- Improve gradients, strokes, joins, caps, and path rendering enough for
  production controls and charts.
- Define mask, blur, backdrop, and filter support levels, including explicit
  fallbacks where a backend cannot support an effect efficiently; v5 disables
  true backdrop filters until a backend can sample the composited framebuffer.
- Tighten pixel snapping, grayscale subpixel text positioning, and glyph cache
  behavior so scrolling and animation stay visually stable.
- Decide the color-management policy for sRGB, alpha blending, and future
  wide-gamut support.
- Add CPU snapshot and WGPU parity coverage for shadows, clipping, transforms,
  gradients, masks, and text over composited layers.

## 18. Visual System And Motion Polish

V4 has themes, state visuals, shaders, and animation primitives. V5 should make
them coherent across the widget set.

Targets:

- Make image/icon buttons first-class, including icon-only, icon+label,
  toggleable, disabled, selected, and destructive variants.
- Define consistent visual states for disabled, hovered, focused, pressed,
  selected, invalid, warning, pending, active, and loading.
- Add animation presets for hover, press, open, close, selection, validation,
  loading, and reduced-motion fallback.
- Clarify shader/effect fallback behavior across CPU snapshots and WGPU.
- Add denser theme variants for operational tools, editor surfaces, and game
  HUDs.
- Expand visual snapshot coverage for common widget states.

## 19. Diagnostics, Devtools, And Inspection

Operad should make UI failures explainable without app-specific debug code.

Targets:

- Add one debug surface for hit testing, focus, keyboard navigation, widget
  events, command/action dispatch, overlay stack state, drag capture, and scroll
  routing.
- Inspect layout, paint order, render batches, resource updates, GPU timings,
  accessibility tree output, and dirty regions in one place.
- Add structured traces for input-to-action-to-render flow.
- Provide snapshot-friendly debug dumps for CI failures.
- Add tests that assert diagnostics are present and stable for common failure
  modes.

## 20. Theme And Design-Token API Stability

Themes should be extensible without forcing applications to fork widgets.

Targets:

- Define stable public design tokens for surfaces, text, icons, focus rings,
  warnings, errors, selection, disabled state, pending state, and motion.
- Let apps extend token scopes for domain surfaces without changing Operad core.
- Document which theme APIs are stable, experimental, or migration-only.
- Add compatibility tests for token fallback, scoped overrides, missing tokens,
  and preference-adjusted themes.

## 21. Versioning And API Stability Policy

V5 should make it explicit which APIs consumers can depend on.

Targets:

- Classify public APIs as stable, experimental, backend-specific, or migration
  compatibility.
- Add deprecation paths for migration helpers that should not become permanent
  architecture.
- Document feature-flag stability for `wgpu`, host runtimes, accessibility
  adapters, and compatibility renderers.
- Add release checklist items for semver review, migration guide updates, and
  deprecated API audits.

## 22. Async Tasks, Loading, And Progress

Applications need background work without making every widget invent its own
loading, cancellation, and repaint behavior.

Targets:

- Define task handles, loading state, progress state, cancellation, completion,
  and failure reporting.
- Support async validation for forms and fields.
- Add repaint requests when task state changes without requiring app-specific
  polling.
- Provide accessible status updates for loading, progress, completion, and
  failures.
- Add tests for task completion, cancellation, async validation, stale-result
  suppression, and progress announcements.

## 23. Virtualization At Scale

Large data views need more than basic visible-row helpers.

Targets:

- Add list, table, tree, and grid virtualization that supports huge datasets.
- Support measured row heights, estimated row heights, row recycling, overscan,
  sticky headers, sticky columns, and incremental layout.
- Keep selection, focus, active descendants, drag targets, and accessibility
  stable across virtualized rebuilds.
- Add performance tests for large scrolling datasets and mixed row heights.
- Add debug output explaining visible ranges, overscan, recycled rows, and
  measured-size cache behavior.

## 24. Multi-Window And Multi-Document Runtime

V5 should support apps with multiple windows, documents, surfaces, and detached
panels.

Targets:

- Define per-window and per-document state boundaries for focus, hover, pressed,
  IME, cursor, drag capture, accessibility, overlays, and render surfaces.
- Support detached panels and multiple app-owned/native surfaces.
- Route platform requests and responses to the correct window/document.
- Add tests for focus transfer between windows, per-window IME state, overlay
  isolation, and multi-surface rendering.

## 25. Touch, Stylus, And Gamepad Input

Pointer input is currently mouse-centered. V5 should broaden the input model for
touch, pen, and controller-heavy apps.

Targets:

- Add touch gesture primitives for tap, long press, pan, pinch, and multi-touch
  cancellation.
- Add stylus pressure, tilt, barrel button, eraser, and hover metadata where
  hosts can provide it.
- Add gamepad/controller navigation for focus movement, activation, cancellation,
  sliders, lists, and menus.
- Define conflict policy between touch scrolling, drag editing, selection, and
  canvas gestures.
- Add tests for touch long-press context menus, pinch/pan canvas gestures, stylus
  metadata propagation, and gamepad focus loops.

## 26. Tooltip, Help, And Context Menu Policy

Tooltip and context menu behavior should be a toolkit policy rather than a set
of one-off widgets.

Targets:

- Add delayed hover/focus tooltips, rich command tooltips, validation help, and
  accessible tooltip relationships.
- Route right-click, long-press, keyboard menu key, and command-based context
  menu requests through one policy.
- Integrate tooltips and context menus with overlays, focus restore, dismissal,
  accessibility metadata, and command routing.
- Add tests for tooltip delay, tooltip dismissal, disabled command tooltips,
  right-click context menus, long-press menus, and keyboard menu invocation.

## 27. Scheduler And Frame Lifecycle

The runtime needs explicit time ownership for animation, timers, async work,
idle work, and deterministic tests.

Targets:

- Define frame phases for input, task polling, layout, action dispatch, render,
  platform requests, and post-frame cleanup.
- Add animation ticks, timers, delayed actions, throttling, debouncing,
  coalesced repaint, and idle work primitives.
- Let tests control time deterministically.
- Prevent unbounded repaint loops and document repaint scheduling policy.
- Add tests for animation ticks, timer cancellation, debounced search, idle work,
  coalesced repaint, and deterministic time advancement.

## 28. Error Boundaries And Recovery

Widget, host, renderer, and resource failures should be diagnosable and local
where possible.

Targets:

- Define recoverable vs fatal errors for widgets, renderers, resources, host
  services, accessibility adapters, and async tasks.
- Allow failed widgets/canvases/resources to report fallback output and
  diagnostics without poisoning the whole frame when safe.
- Add structured error reports to debug tooling and test assertions.
- Add tests for missing resources, renderer failures, task failures, adapter
  failures, and local fallback rendering.

## 29. Resource Limits And Security

The toolkit should behave predictably with malformed input, untrusted assets,
and large resource requests.

Targets:

- Define limits for texture sizes, image dimensions, font files, text input,
  paste size, virtualized row counts, and resource cache memory.
- Validate untrusted images, fonts, clipboard data, drag/drop payloads, and
  malformed text input before they reach renderer internals.
- Add graceful failure paths for over-limit resources and invalid payloads.
- Add tests for malformed assets, oversized textures, huge paste payloads,
  invalid drag payloads, and resource budget eviction.

## 30. Packaging And Release Automation

V5 should make releases reproducible rather than manually assembled.

Targets:

- Add CI coverage for no-default, all-features, widgets, wgpu, egui compat,
  accesskit-winit, docs, formatting, clippy where appropriate, and release perf
  smoke tests.
- Automate changelog, migration guide, release checklist, and semver review
  checks where possible.
- Add docs generation and example build checks.
- Add feature-matrix checks to prevent accidental dependency creep.
- Add release scripts for branch/tag creation, crate packaging, and publish dry
  runs.

## 31. API Docs And Concept Reference

Examples are not enough for a toolkit. V5 needs stable conceptual docs.

Targets:

- Add reference docs for document trees, widget state, action events, host
  runtime, renderer backends, accessibility, layout, resources, and testing.
- Document recommended ownership patterns for app state vs widget state.
- Document lifecycle order, frame phases, event propagation, and command/action
  dispatch.
- Document migration notes for v4 consumers adopting v5 runtime features.
- Keep examples linked from the relevant concept docs.

## 32. Examples, Templates, And Tooling

V5 should make the intended usage obvious through executable examples and small
templates.

Targets:

- Add canonical examples for button command handling, text input, popup/select,
  drag, canvas viewport hit targets, WGPU window rendering, and accessibility
  output.
- Add a minimal app template using the native host runtime.
- Add debug tooling that explains hit testing, focus, overlay stack state,
  widget actions, drag capture, accessibility tree output, and render timings.
- Keep examples small enough to serve as regression tests.

## Proposed Release Gate

V5 should ship when these library-owned gates are green:

- Button activation and command/action routing work through pointer and keyboard
  input.
- Raw pointer gestures are routed through the host frame path and can drive drag
  widgets without app-local gesture plumbing.
- A native host runtime example can open a WGPU window and exercise buttons,
  text input, popup/select, drag, and canvas viewport hit testing.
- Text editing has undo/redo, robust IME lifecycle coverage, multiline
  selection, and clipboard tests.
- Widget identity and state binding preserve focus, overlays, scroll, animation,
  and edit state across rebuilds.
- Shared selection models and edit transactions cover lists, trees, tables,
  timelines, forms, sliders, and canvas hit targets.
- Form validation, dirty tracking, pending/apply/cancel, and accessible error
  summaries are covered by tests.
- Unified keyboard navigation covers roving focus, active descendants, menus,
  listboxes, tables, trees, toolbars, and Escape/Enter/Space semantics.
- Overlay stack behavior is centralized and covered by nested popup/menu tests.
- At least one real accessibility adapter path is proven beyond metadata-only
  assertions.
- Public layout APIs have a clear Operad-owned path that does not require direct
  Taffy usage for common cases.
- Localization, RTL/text-direction, and dynamic labels have an explicit support
  path and regression coverage.
- Font, icon, image, and texture lifecycle behavior is documented and tested.
- Renderer/performance tests cover interaction-heavy frames, large resources,
  and native-surface rendering where available.
- Advanced scrolling handles nested arbitration, anchoring, sticky/fixed
  content, kinetic behavior, scrollbars, reveal-into-view, and synchronized
  surfaces.
- Layering/compositing semantics are explicit for stacking contexts,
  transforms, opacity groups, offscreen layers, clipping, masks, and transformed
  hit testing.
- Compositor-quality rendering covers shadows, rounded clipping, borders,
  gradients, masks, filters, backdrop fallback policy, grayscale subpixel text,
  and CPU/WGPU parity for composited content.
- Diagnostics can explain input routing, widget actions, overlay state,
  accessibility output, and render timing in one debug surface.
- Theme/design-token APIs and feature stability are documented for v5 consumers.
- Async task state, loading/progress, cancellation, async validation, and repaint
  scheduling are covered by tests.
- Virtualized list/table/tree/grid behavior handles huge datasets, measured row
  heights, sticky regions, selection, focus, and accessibility.
- Multi-window and multi-document state routing is proven for focus, IME,
  overlays, cursor, accessibility, and render surfaces.
- Touch, stylus, and gamepad input have explicit routing and regression tests.
- Tooltip, help, and context menu policy is centralized and accessible.
- Scheduler/frame lifecycle behavior is deterministic in tests and prevents
  unbounded repaint loops.
- Error boundaries, resource limits, and malformed input handling are documented
  and tested.
- CI/release automation covers the feature matrix, docs, semver review, and
  package dry runs.
- API docs explain the core concepts, lifecycle, ownership, and migration path.
