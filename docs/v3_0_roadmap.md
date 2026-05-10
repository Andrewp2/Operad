# Operad 3.0 Roadmap

This roadmap translates the Orbifold and game-agent 3.0 wishlists into reusable
Operad work. Consumers should continue to own product semantics; Operad should
own the themeable, testable, accessible UI machinery that projects app snapshots
into documents, editor surfaces, backend paint, and command intents.

## Release Direction

Operad 3.0 should focus on workstation-grade UI infrastructure:

1. Accessibility foundation and platform adapter contracts.
2. Theme tokens and component visual states.
3. Rich paint primitives and icon/image handles.
4. Command routing, shortcut scopes, and tooltip integration.
5. Gesture phases, pointer capture, and editor-surface hit testing.
6. App shell helpers for persisted split, dock, tab, and scroll-sync state.
7. Property, numeric, text, tree, table, and browser polish.
8. Backend-neutral input, platform output, image handles, and layer policy.
9. Screenshot, layout, interaction, and performance tooling.

## Cross-App Reuse Gate

Every v3 slice should be reviewed as shared toolkit infrastructure before it is
implemented. A feature belongs in Operad when it can be described without game,
music, semiconductor, or application-specific nouns and when consumers can bind
their own state, commands, and drawing to the same primitive.

Use neutral API names for reusable mechanics:

- Prefer `TimelineGeometry`, `LaneGeometry`, `RangeItem`, `CurvePoint`,
  `Overlay`, `CommandId`, `CanvasContent`, `DataTable`, `EditableForm`, and
  `ShellRegion`.
- Avoid public APIs named after product concepts such as inventory slots, MIDI
  notes, wafer dies, fabrication recipes, synth parameters, or game debug tools
  unless the type is explicitly only test/sample data.
- Keep examples free to mention Orbifold clips, Fabricad wafers, or game HUDs,
  but keep core structs and widget contracts app-neutral.

Before landing a nontrivial v3 primitive, it should have a short reuse check:

- **Game:** does this remove egui coupling, improve renderer-neutral input,
  layering, assets, debugging, or reusable menus/HUD/editor tools?
- **Orbifold:** does this support dense workstation shell, timeline/lane/range
  editing, commands, text/list inputs, or custom editor surfaces without owning
  musical state?
- **Fabricad/layout:** does this support inspectable panels, forms, data grids,
  canvas/viewports, charts, auditability, accessibility, or custom domain hit
  targets without owning semiconductor state?

If a slice only benefits one application, keep it in that application unless it
can be reduced to a neutral primitive with typed app-owned payloads.

## Accessibility Track

The first v3 commit expands the core accessibility contract beyond v2 metadata:

- More roles for toolbars, toggle buttons, search boxes, spin buttons, splitters,
  status/alert/live regions, editor surfaces, table headers, rows, rulers, and
  meters.
- State flags for selected, checked, expanded, pressed, read-only, required,
  invalid, modal, and hidden.
- Value ranges for sliders, scrollbars, numeric controls, rulers, and color
  channels.
- Relationships for labelled-by, described-by, controls, owns, and active
  descendant.
- Actions, keyboard shortcuts, focus ordering, modal-scope detection, and live
  region priority.

Next accessibility work should define backend/platform adapter traits for
screen-reader trees, focus restore, focus traps, reduced motion, high contrast,
clipboard, text/IME, drag/drop, and screenshots.

The branch now includes `src/accessibility.rs` as the backend-facing
accessibility contract: screen-reader tree publishing requests, focus movement,
focus traps, focus restore targets, live announcements, host preference flags,
and accessibility capabilities integrated into backend capability descriptors.

## Theme Track

Add a first-class theme model with semantic tokens:

- Color, typography, spacing, radius, stroke, shadow, opacity, and motion tokens.
- Component tokens for buttons, tabs, search fields, lane headers, range items,
  editor lanes, property rows, menu rows, and transport controls.
- Scoped theme inheritance so editor surfaces can use musical colors while shell
  widgets remain visually consistent.
- One excellent dark theme before broad light-theme work.
- High-density visual variants for toolbars, lists, data grids, and DAW/editor
  controls.
- Stable active, hover, pressed, disabled, invalid, warning, changed, pending,
  selected, and focused state tokens.

## Paint And Asset Track

Add renderer-neutral primitives and resource handles that make dense app
surfaces possible without leaking backend types into consumer state:

- Rounded rectangles with stroke alignment.
- Linear gradients and simple multi-stop gradients.
- Shadows, glows, inset borders, and clear fallback semantics.
- Text alignment, baseline positioning, clipping, and elision.
- Text primitives inside custom scene/display-list surfaces, including anchored
  text at a point or rect, multiline labels, contrast-aware color selection, and
  snapshot coverage.
- Icon/image registry handles with sizing, tint, and alignment.
- App-owned image, icon, texture, and thumbnail handles that can be resolved by
  egui, wgpu, a game renderer, or an offscreen snapshot renderer.
- Canvas/native-viewport embedding and render callbacks for custom GPU/tiled
  surfaces, wafer maps, charts, sparklines, and domain hit targets.
- Paths for automation curves, waveforms, and custom editor display lists.
- Pixel snapping policy for hairlines and grids.

## Backend And Platform Track

Make egui one optional adapter rather than a type that leaks into consumer UI
models, tests, input conversion, texture handles, and styling:

- Renderer-neutral raw input events, platform-output responses, cursor changes,
  repaint requests, file dialogs, clipboard, open-URL, screenshots, text/IME,
  and drag/drop service requests.
- Backend adapter traits for egui, future wgpu, CPU snapshot rendering, and
  app-owned renderers.
- Host adapter contracts for hover, pressed, focused, drag-captured, text/IME,
  wheel-targeted, and shortcut-routed state before paint.
- Texture/image delta abstraction for application-owned resources such as game
  menu thumbnails.
- Explicit layer and z-order policy for mixed host/debug/app UI, so debug UI can
  stay above app UI without relying on backend-specific ordering.
- A richer paint-list/backend contract that supports batching, resource
  resolution, partial updates, and deterministic tests.

## Commands And Gestures Track

Add app-owned command routing without importing app semantics:

- Command registry with opaque IDs.
- Platform-aware shortcuts and scope hierarchy.
- Conflict detection, debug dumps, menu integration, command palette integration,
  and tooltip shortcut display.
- Menu and popup APIs should emit command IDs or typed outcomes instead of
  requiring consumers to inspect node names.
- Platform service command hooks for file dialogs, quit, screenshot, clipboard,
  and other app-owned effects.
- Pointer capture, drag thresholds, double-click timing, cancellation, modifiers,
  high-resolution wheel deltas, and edit phase coalescing.
- Drag capture for sliders, splitters, range-item handles, curve handles, and
  host-embedded editor surfaces.

## Shell And Editor Track

Make dense workstation layouts easier to assemble without baking in one
application's domain model:

- Persistent top, left, lane list, timeline/editor, right, bottom, and status
  regions.
- Higher-level shell host for menu/transport/status bars, left/right/bottom
  resizable panels, central workspace, scroll containers, visibility, docking,
  saved layout state, and persisted offsets.
- Split pane collapse/restore, min/max sizes, and keyboard accessible resizing.
- Dock panel visibility and persisted size state.
- Tab strips for inspector/editor panels.
- Lane-list and timeline/editor scroll synchronization.
- Editor surfaces with world/view transforms, hover, hit testing, drag capture,
  marquee selection, snapping, cursor override, tool mode, and overlay layers.
- Timeline range-item geometry, resize handles, lane headers, curve points, and
  segment helpers should be named as generic editor primitives; Orbifold clips
  and automation, Fabricad timeline/review ranges, and game editor ranges should
  all adapt their own domain payloads on top.
- Scene/editor text should not require an egui painter escape hatch.

## Data And Editing Track

Move beyond renderable controls toward production editing workflows:

- Numeric fields, drag values, and parameter widgets with units, prefixes,
  suffixes, clamping, fine adjustment, logarithmic scaling, keyboard precision,
  and commit versus preview phases.
- Dense data views for logs and capture tables with virtualized rows, selectable
  rows/cells, fixed/resizable columns, copy/export commands, compact monospace
  text, empty states, and sticky headers.
- Property controls should surface invalid, changed, pending, read-only, and
  disabled state consistently through theme and accessibility metadata.

## Testing And Performance Track

Build on the v2 snapshot/perf smoke harness:

- Pixel-diff tooling with tolerances.
- Event replay for menus, row selection, drag gestures, scrolling, shortcuts,
  raw input conversion, and platform-output assertions.
- Snapshot and event-test utilities that do not require egui harness types.
- Layout assertions by stable node name.
- Paint-list assertions for editor primitives.
- Dirty flags for layout, paint, input, theme, and text measurement.
- Retained display lists for static editor backgrounds.
- Frame timing sections for snapshot, layout, paint build, render, and input.

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
  live regions, and modal/focus-trap traversal.
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
- Host accessibility state snapshots let consumers carry those previous-frame
  accessibility inputs across frames as one reusable runtime object.
- Accessibility preference resolution in `src/theme.rs`, `src/renderer.rs`,
  and `src/host.rs` for text scaling, reduced motion, high contrast/forced
  colors, reduced transparency, and render-option propagation from host frame
  requests.
- Existing core widgets and major widget families wired to richer accessibility
  states where the current APIs already expose that information.
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
  topmost hits, accessibility labels, disabled targets, and metadata, plus
  render-output snapshot, batch, painted-item, and timing assertions.
- Public CPU snapshot rendering in `src/testing.rs` gives consumers a
  deterministic renderer adapter and document raster path for E2E screenshots
  without depending on egui or Operad's private integration-test harness.
- E2E snapshot coverage in `tests/e2e_render.rs` now includes a reusable editor
  surface scene built from timeline range-item, lane, ruler, playhead, curve
  point, interpolation path, and resize-handle primitives.
- Performance smoke coverage in `tests/perf_smoke.rs` now exercises reusable
  editor geometry, hit-target construction, curve segments, scene paint-list
  generation, and deterministic raster snapshots under a fixed budget.
- Layout audit checks in `src/lib.rs` now cover duplicate node names,
  non-finite rects, invisible or too-small interactive nodes, text clipping,
  nodes outside the root, empty paint clips, focusable controls missing from
  the accessibility traversal, and accessible-name/action/action-id/action-label/
  duplicate-action/state/value/value-range/relation-target gaps.
- Operad-owned layout helper APIs in `src/lib.rs` cover common fixed, fill,
  centered flex, absolute, gap, min/max size, flex item, and clipped node-style
  shapes so consumers can use fewer raw Taffy struct literals.
- Accessibility adapter contracts in `src/accessibility.rs` now include
  deterministic live-region snapshots, live-region diffing, and announcement
  queues that can be converted into supported screen-reader adapter requests.
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
  capabilities through the shared backend capability descriptor.
- Feature-gated egui platform-output plans in `src/egui_host.rs` for mapping
  supported Operad clipboard-write, open-URL, cursor, and repaint requests into
  egui-compatible output while reporting unsupported host services explicitly,
  including service-request IDs when consumers pass correlated platform
  requests through the egui adapter, and producing correlated unsupported
  responses for backend services egui cannot handle directly.
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
  naming those domains.
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
  cache eviction.
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
