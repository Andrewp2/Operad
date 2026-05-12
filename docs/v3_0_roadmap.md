# Operad 3.0 Release Scope

Operad `3.0.0` is the workstation-grade UI infrastructure release for the game,
Orbifold, and Fabricad/layout consumers. It is intentionally breaking relative
to `2.0.0`; the main goal is to give downstream apps reusable, renderer-neutral
contracts for accessibility, host frames, command routing, custom editor
geometry, dense widgets, snapshots, performance checks, and Operad-owned layout
style storage.

Future work that did not fit in `3.0.0` has been moved to
[`docs/v4_0_roadmap.md`](v4_0_roadmap.md). Consumer migration guidance is in
[`docs/v3_0_migration_guide.md`](v3_0_migration_guide.md).

## Release Gate

The release branch is expected to pass:

- `cargo test --all-features`
- `cargo test --no-default-features`
- `cargo check --examples --all-features`
- `cargo clippy --all-features --all-targets -- -D warnings`
- `cargo fmt -- --check`
- `git diff --check`

## Cross-App Reuse Rule

The v3 API surface keeps product semantics in the applications. Operad owns
generic UI machinery: layout, input routing, accessibility, paint contracts,
commands, shell state, editor geometry, data widgets, testing, and host/backend
adapters. Game, Orbifold, and Fabricad should pass app-owned IDs, labels,
commands, resources, and domain hit targets into those neutral primitives.

## Current V3 Baseline

The branch starts from Operad 2.0.0 plus:

- `Cargo.toml` version bumped to `3.0.0`.
- Expanded accessibility primitives in core.
- Scoped theme registry contracts in `src/theme.rs` for shell, panel, editor,
  overlay, menu, and tooltip theme scopes with inherited token patches and
  derived component-token rebuilding.
- `UiDocument::accessibility_snapshot()` with nodes, focus order, and modal
  scope.
- Accessibility tree helpers for nearest accessible parents, focusable nodes,
  effective modal focus order, live regions, and modal/focus-trap traversal.
- Accessibility focus helpers now turn effective tree or focus-trap navigation
  into backend-facing move-focus/restore-focus adapter requests.
- Accessibility name, description, and screen-reader text resolvers that fold
  direct labels, summaries, labelled-by/described-by relations, values, states,
  and shortcuts into backend-facing text.
- Screen-reader summary payloads for custom editor surfaces, with structured
  title, description, key/value items, instructions, and tree lookup helpers.
- Backend-facing accessibility adapter request/response contracts and host
  preference flags for screen reader, reduced motion, high contrast, forced
  colors, transparency, and text scaling.
- Host document frames diff accessibility trees, focused node state, live
  regions, and accessibility preferences so backend adapters receive
  capability-gated publish-tree, preference-apply, and announcement requests.
- Host document-frame state snapshots let consumers carry reusable interaction
  and previous accessibility inputs across host/document frames without
  rebuilding previous-frame plumbing by hand.
- Renderer-neutral scenario testing now combines input replay, document-frame
  processing, paint recording, platform request collection, and reusable
  assertions without depending on egui harness types.
- Test render frames now report batch and render timing sections through the
  shared `RenderFrameOutput` timing surface.
- Frame timing series helpers aggregate per-frame sections into reusable
  performance samples and section-level budget assertions.
- Scenario reports expose renderer-neutral timing sections for pre-input layout,
  input replay, document-frame processing, rendering, and platform request
  collection.
- Perf smoke coverage now runs multi-frame scenario harness rendering with
  timing-series section assertions, percentile budget checks, and paint-list
  content checks.
- Display-list reuse series helpers aggregate retained display-list hit/miss
  outcomes across frames and assert reuse rates, eviction absence, and per-key
  outcome counts.
- Document keyboard focus traversal now follows accessibility focus ordering and
  modal-scope containment before falling back to input-only document order.
- Accessibility preference resolution in `src/theme.rs`, `src/renderer.rs`,
  and `src/host.rs` for text scaling, reduced motion, high contrast/forced
  colors, reduced transparency, and render-option propagation from host frame
  requests.
- Existing core widgets and major widget families wired to richer accessibility
  states where the current APIs already expose that information.
- Text input state now maps pointer coordinates to caret positions and drag
  selections through renderer-neutral layout metrics, and can update IME cursor
  sessions from those pointer-driven caret changes.
- Orbifold, game-agent, and Fabricad/Rust-layout v3 migration notes preserved
  under `docs/`.
- Cross-application reuse criteria in this roadmap now require new primitives
  to be named and tested as neutral toolkit mechanics, with product-specific
  concepts passed in as app-owned data rather than embedded in Operad APIs.
- Public theme, shell, editor, and drag/drop APIs now use lane, timeline, range
  item, editor lane, and lane-value terminology instead of app-specific track,
  clip, note, piano-roll, or arrangement names.
- Renderer-neutral paint extensions in `src/paint.rs` for gradient brushes,
  stroke alignment, corner radii, shadows/glows/inset shadows, anchored scene
  text with alignment and overflow policy, image placement, path primitives,
  and pixel-snapping policy for rect edges, hairline line centers, paths, and
  stroke widths.
- Renderer-neutral raw input and gesture contracts in `src/input.rs` for pointer
  identity/buttons, high-resolution wheel units, keyboard/text conversion,
  pointer capture, drag thresholds, double-click counting, cancellation, and
  gesture-to-edit phases.
- Document-facing wheel events now preserve wheel unit and phase metadata from
  raw input conversion, and document scroll handling mutates offsets only for
  moved or momentum wheel phases.
- Persistable app-shell state contracts in `src/shell.rs` for docked/floating
  panel visibility, saved extents, collapse/restore, keyboard-resizable split
  state, active tabs, focus restore, and synchronized lane/timeline scroll
  offsets.
- Shell layout planning in `src/shell.rs` for top/menu/transport/tool/status
  bars, left/right/bottom dock regions, central workspace, lane list,
  timeline, editor, hidden panels, floating panels, and persisted panel
  scroll offsets.
- Shell layout document bridges in `src/shell.rs` for turning persisted shell
  plans into stable region, panel, panel-content, hidden-panel, and accessible
  resize-handle nodes inside a `UiDocument` without depending on a renderer.
- Shell bar and transport planning in `src/shell.rs` for command/toggle/readout
  item metadata, enabled/active/pressed state, width priorities, cluster spacing,
  deterministic overflow plans, and renderer-neutral accessibility metadata.
- Renderer-neutral testing helpers in `src/testing.rs` for event replay with
  click/drag/wheel/key/focus builders and interaction assertions, raw input
  conversion checks, platform-output assertions, stable-name layout assertions,
  audit warning assertions, paint-list kind/node/shader assertions, RGBA pixel
  diffs with tolerances, dirty flags, deterministic snapshot hash/content
  assertions, frame timing section/budget assertions, performance sample budget
  assertions, command-aware shortcut replay assertions, and stable-name
  accessibility assertions for roles, labels, resolved names/descriptions,
  screen-reader text, action IDs/labels/shortcuts, key shortcuts, values,
  summaries, live regions, focus order, active descendants, document-frame generated
  platform-service requests, request/response coverage, correlated unsupported
  platform-service responses, render-frame conformance assertions for
  canvas/image handler coverage, dirty regions, host input capture, and
  per-node interaction state, canvas hit-report assertions for target ids,
  topmost hits, accessibility labels, disabled targets, and metadata,
  display-list reuse/invalidation assertions and multi-frame reuse-series
  budgets, percentile performance budgets, plus render-output snapshot, batch,
  painted-item, and timing assertions.
- Public renderer-neutral testing helpers in `src/testing.rs` give consumers a
  deterministic paint-recording adapter and document-frame path for E2E checks
  without depending on egui or Operad's private integration-test harness.
- E2E snapshot coverage in `tests/e2e_render.rs` now includes a reusable editor
  surface scene built from timeline range-item, lane, ruler, playhead, curve
  point, interpolation path, and resize-handle primitives.
- Performance smoke coverage in `tests/perf_smoke.rs` now exercises reusable
  editor geometry, hit-target construction, curve segments, scene paint-list
  generation, retained display-list reuse rates, and deterministic paint
  fingerprints under a fixed budget.
- Layout audit checks in `src/lib.rs` now cover duplicate node names,
  non-finite rects, invisible or too-small interactive nodes, text clipping,
  low-contrast text and scene text against effective background fills, nodes
  outside the root, empty paint clips, interactive controls missing visible
  accessibility metadata, focusable controls missing from the accessibility
  traversal, and accessible-name/action/action-id/action-label/duplicate-action/
  state/value/missing-or-invalid-value-range/relation-target gaps, including
  meter ranges.
- Public `ColorRgba` contrast helpers compute alpha compositing, relative
  luminance, contrast ratios, contrast pass/fail checks, and highest-contrast
  foreground selection for theme, widget, audit, and snapshot code.
- `UiNodeStyle` and widget option structs now store Operad-owned `LayoutStyle`
  values. Core translates that owned type to Taffy internally, and public node
  constructors plus screenshot/perf consumer probes use the `layout::*` helper
  surface instead of raw Taffy style literals.
- Operad-owned layout helper APIs in `src/lib.rs` cover common fixed, fill,
  row/column flex, centered/flex-start children, absolute, gap, margin, padding,
  min/max size, flex item, and clipped node-style shapes.
- Accessibility adapter contracts in `src/accessibility.rs` now include
  deterministic live-region snapshots, live-region diffing, and announcement
  queues that can be converted into supported screen-reader adapter requests,
  plus batch request plans that split supported adapter requests from explicit
  unsupported responses.
- Live-region snapshots and announcement queues now prioritize assertive
  announcements ahead of polite status updates while preserving deterministic
  ordering within each priority.
- Renderer-neutral debug snapshots in `src/debug.rs` for layout bounds, clip
  rects, paint primitive counts, local and resolved z ranges, host interaction
  flags, command scopes, active gestures, repaint reasons, frame timings, theme
  token inspection, resolved component state previews, and hit-test traces.
- Renderer/backend adapter contracts in `src/renderer.rs` for render targets,
  resource deltas, dirty regions, paint batching, deterministic snapshots, and
  backend capability negotiation, with renderer-facing image extraction and
  image callback registries for app-owned icon/image/texture draw paths.
- Document paint output now carries `platform::LayerOrder` through node styles,
  hit testing, paint-list generation, renderer batch keys, image/canvas render
  requests, and debug dumps so host, app, overlay, debug, and system surfaces
  sort consistently before local z-index is applied.
- Document hit testing and wheel-scroll targeting now use the same
  renderer-neutral `PaintTransform` geometry as paint output, so animated or
  scaled controls receive input where they are painted instead of in stale
  layout-space bounds.
- Egui paint callback hooks in `src/lib.rs` for forwarding renderer-neutral
  image, image-placement, and canvas paint items to app-owned egui bridge code
  instead of silently dropping asset-backed primitives.
- Feature-gated egui host input adapter in `src/egui_host.rs` for translating
  egui pointer, wheel, keyboard, focus navigation, text, paste, and IME commit
  events into Operad raw input without leaking egui types into app UI models.
- Feature-gated egui host adapter in `src/egui_host.rs` implements the
  backend-neutral `HostAdapter` trait, owns egui input translation and command
  routing, applies correlated platform responses, and advertises egui host
  capabilities through the shared backend capability descriptor, including the
  accessibility-adjacent clipboard and text/IME bridge surface egui owns today.
- Feature-gated egui accessibility output plans in `src/egui_host.rs` split
  document-frame accessibility adapter requests through the shared capability
  contract, producing explicit unsupported responses for screen-reader services
  egui does not implement directly yet.
- Feature-gated egui platform-output plans in `src/egui_host.rs` for mapping
  supported Operad clipboard-write, open-URL, cursor, and repaint requests into
  egui-compatible output while reporting unsupported host services explicitly,
  including service-request IDs when consumers pass correlated platform
  requests through the egui adapter, consuming merged host document-frame
  service requests directly, and producing correlated unsupported responses for
  backend services egui cannot handle directly.
- Feature-gated egui texture-delta plans in `src/egui_host.rs` for converting
  renderer-neutral resource updates into stable egui user texture deltas without
  storing egui texture handles in app-owned UI state.
- Embedded canvas/native viewport contracts for callback, texture, and
  native-viewport render modes, host input capture policies, pointer-lock
  requests, domain hit-testing flags, renderer-facing canvas extraction, and
  renderer-neutral callback registries that pass rects, clips, scale factors,
  dirty regions, and per-node host interaction state to app-owned canvas
  renderers.
- Canvas render handlers can return neutral hit-target metadata in render
  outputs, with per-canvas hit collections aggregated by render reports so apps
  can map their own domain IDs to viewport/editor selections without Operad
  naming those domains, and equal-z hit ties preserve reported target order so
  app-owned canvas renderers can use their draw/hit ordering deterministically.
- Canvas hit targets and collections expose accessibility metadata and
  screen-reader summaries for app-owned canvas/editor/viewport surfaces.
- Canvas host-capture plans in `src/renderer.rs` convert canvas interaction
  policies into renderer-neutral host capture metadata and cursor
  confine/visibility platform requests for pointer-locked surfaces.
- Canvas host-capture lifecycle state in `src/renderer.rs` and `src/host.rs`
  diffs app-owned canvas plans across frames so hosts can acquire, update, and
  release pointer-lock/cursor-capture requests deterministically.
- Platform request ID allocation turns renderer-neutral platform requests into
  deterministic service requests that host adapters can correlate with
  responses, including canvas host-capture transitions.
- Host document-frame output can merge adapter-emitted service requests with
  generated canvas-capture service requests so consumers can submit one
  correlated host-service stream per frame.
- Text input routing helpers that bridge document focus, editable text state,
  clipboard service requests, and IME activation/update/deactivation without
  requiring consumers to hand-assemble platform plumbing for each field.
- Searchable select/listbox contracts in `src/widget_ext/menu.rs` compose
  filtering, active descendant metadata, bounded visible rows, selected/active
  row accessibility, escape close, and outside-dismiss outcomes for combo and
  filter picker workflows.
- Overlay frame contracts in `src/widget_ext/surfaces.rs` for dialog/popover
  open, close, toggle, Escape/outside dismissal, dismissed overlay reporting,
  focus trap state, and backend-gated accessibility focus-trap requests.
- Chart, sparkline, and dense grid-map geometry helpers in `src/charts.rs` for
  numeric range mapping, path generation, cell rectangles, hit testing, and
  visible-cell queries, with screen-reader summaries, axis metadata, overlay
  layers, selection summaries, and hit metadata for chart series, sparklines,
  and grid-map surfaces.
- Grid-map cell metadata in `src/charts.rs` for masked dense analytic surfaces,
  domain cell IDs, labels, values, disabled/non-selectable cells, and hit
  collections that skip out-of-bounds cells.
- Chart hit accessibility helpers in `src/charts.rs` for exposing samples,
  grid cells, overlays, axes, labels, and custom hit targets as speakable
  accessibility metadata and summaries.
- Dense table metadata in `src/widget_ext/data.rs` for sortable/filterable
  columns, app-owned sort/filter/resize commands, accessibility sort state,
  active-cell copy/export contracts, row/cell action metadata, sticky column
  partitions, and row drag/drop descriptors.
- Toggle-control state contracts in `src/widget_ext/data.rs` for switch,
  checkbox, and toggle-button roles, including mixed state, disabled state,
  edit phases, typed outcomes, and accessibility metadata.
- Host adapter contracts in `src/host.rs` for hover, pressed, focused,
  drag-captured, text/IME, wheel-targeted, shortcut-routed, command dispatch,
  and repaint/platform-service state before paint, plus a document-frame
  coordinator that applies host UI events, recomputes layout, builds render
  requests with node interaction state, snapshots accessibility, and emits
  live-region announcement requests.
- Host shell frame contracts in `src/host.rs` for applying renderer-neutral
  shell events to `ShellWorkspaceState`, including panel resize/extent,
  collapse/restore, focus restore targets, panel scroll offsets, and updated
  shell layout plans.
- Command effect hooks in `src/commands.rs` for mapping enabled app commands to
  platform service requests or opaque app-owned effects, including clipboard,
  file dialog, screenshot, repaint, close-window, and quit requests.
- Editor-surface helpers in `src/editor.rs` for world/view transforms, hit
  testing, snapping, cursor/tool modes, marquee selection, drag capture, and
  overlay ordering.
- Editor accessibility helpers in `src/editor.rs` for mapping generic hit
  targets, resize handles, rulers, overlays, active selections, visible ranges,
  and keyboard actions into screen-reader metadata for app-owned custom
  surfaces.
- Lane timeline geometry helpers in `src/editor.rs` for unit-to-x timelines,
  lane y/index mapping, visible ranges, range/selection/playhead rectangles,
  ruler ticks, and grid snapping.
- Timeline range-item geometry helpers in `src/editor.rs` for app-owned spans
  on lane/timeline surfaces, including body and resize hit targets, snapped drag
  previews, constrained resizing, and selected/disabled/dragging interaction
  state without naming product-specific clips, wafers, or game timeline items.
- Curve editor geometry helpers in `src/editor.rs` for app-owned points and
  paths, including normalized value mapping, point hit targets, sorted segment
  view geometry, interpolation paths, snapped point translation, and clamped
  value edits without naming product-specific parameters, measurements, or
  gameplay curves.
- Lane-value geometry helpers in `src/editor.rs` for value-to-lane mapping,
  range-item rectangles, loop-wrapped item segments, body versus resize-handle
  hit targets, and magnitude-bar geometry.
- Retained display-list cache contracts in `src/display.rs` for static editor
  backgrounds, snapshot/display-list reuse, dirty-flag invalidation, and bounded
  cache eviction, with observable reuse/miss/eviction/invalidation reports for
  scenario and performance tests.
- Asset registry contracts in `src/assets.rs` for built-in common action icons,
  app-provided icon/image descriptors, sizing, tinting, alignment, compact
  icon-button metadata, tooltip text, accessibility labels, and deterministic
  vector fallback paths for built-in icons when no texture/image handler is
  registered.
- Tooltip and shortcut-display contracts in `src/tooltips.rs` for platform-aware
  shortcut labels, command metadata tooltips, disabled reasons, and
  renderer-neutral tooltip requests.
- Menu and command-palette helpers in `src/widget_ext/menu.rs` for building
  items from the command registry, preserving enabled/disabled command state,
  displaying scoped shortcuts, and returning typed `CommandId` selections.
- Nested menu navigation state in `src/widget_ext/menu.rs` for submenu
  open/close, scoped typeahead, arrow traversal, and stable command selection
  paths without requiring consumers to inspect node names.
- Active-descendant accessibility relationships for select menus, menu lists,
  menu bars, dropdown popups, and command palette result navigation.
- Searchable select/listbox state in `src/widget_ext/menu.rs` for filtered
  option indices, active enabled option navigation, empty-state metadata,
  accessibility values, and selection results using existing `SelectSelection`
  shapes.
- Shared search-field state in `src/widget_ext/menu.rs` for listbox and command
  palette filtering, clear-button metadata, debounced filter requests,
  keyboard clear/close behavior, and polite accessible result status text.
- Progress and meter indicator helpers in `src/widget_ext/surfaces.rs` for
  bounded or indeterminate values, normalized fill geometry, accessible
  progress/meter metadata, and renderer-neutral fill nodes.
- Numeric parameter contracts in `src/widget_ext/pickers.rs` for unit
  prefixes/suffixes, normalized linear/logarithmic value mapping, formatted
  parameter accessibility metadata, and parameter-aware commit/cancel text
  editing.
- Numeric slider interaction contracts in `src/widget_ext/pickers.rs` for
  horizontal/vertical geometry, fill/thumb rects, pointer drag phases,
  keyboard stepping, quantized parameter values, and slider accessibility
  actions.
- Text-input editing helpers in `src/lib.rs` for selected text, Unicode-safe
  caret line/column metadata, multiline line-start/line-end movement, and
  vertical caret movement with shift selection.
- Text-input rendering contracts in `src/lib.rs` for deterministic caret rects,
  multiline selection rects, scene paint plans, and accessibility summaries that
  expose caret and selection position to backend adapters.
- Text-input platform helpers in `src/lib.rs` for deriving IME sessions from
  caret geometry, producing activate/update/deactivate and keyboard
  show/hide requests, mapping clipboard outcomes to platform clipboard
  requests, applying IME commit/preedit/delete responses, and target-checking
  text IME responses before mutating a focused field.
- Programmatic scroll helpers in `src/lib.rs` for bringing explicit document
  rects or nested target nodes into scroll-container view with axis-aware
  clamping.
- Scrollbar drag contracts in `src/lib.rs` for mapping vertical and horizontal
  thumb pointer deltas to clamped scroll offsets without backend-specific
  widget state.
- Data-table export helpers in `src/widget_ext/data.rs` for selected rows,
  visible rows, active cells, and ranges, with TSV/CSV formatting and clipboard
  command-effect integration.
- Data-table row and cell metadata in `src/widget_ext/data.rs` for row/cell
  actions, context-menu command IDs, draggable row sources, per-row drop
  policies, and drop-placement target descriptors.
- Property-inspector status contracts in `src/widget_ext/data.rs` for invalid,
  error, warning, help, changed, and pending row metadata, accessibility
  hints/live regions, and optional status visual/shader hooks.
- Editable form/inspector contracts in `src/widget_ext/data.rs` for field
  kinds, required/read-only/disabled state, validation/changed/pending state,
  focus traversal, commit/cancel/picker outcomes, command actions, and field
  accessibility metadata.
- Dense data-view contracts in `src/widget_ext/data.rs` for empty states,
  section headers, flattened row projections, stable row identity remapping
  across filtering/sorting, and sticky header/leading-column partitions.
- Drag/drop surface metadata in `src/drag_drop.rs` for stable draggable source
  ids, drop target ids, payload acceptance policies, operation resolution,
  topmost target hit testing, platform drag-start request construction, and
  accessibility summaries for editor, data, tree, asset, and canvas surfaces.
- Tree-view row action and context-menu metadata in `src/widget_ext/data.rs`,
  with draggable row source descriptors, per-item drop policies, drop-placement
  target descriptors, and accessibility actions for keyboard/screen-reader
  access to row commands and drag/drop affordances.
