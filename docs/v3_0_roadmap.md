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
- Component tokens for buttons, tabs, search fields, track headers, clip blocks,
  piano-roll lanes, property rows, menu rows, and transport controls.
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
- Drag capture for sliders, splitters, note handles, clip handles, automation
  curves, and host-embedded editor surfaces.

## Shell And Editor Track

Make common DAW layouts easier to assemble:

- Persistent top, left, track, arrangement, editor, right, and status regions.
- Higher-level shell host for menu/transport/status bars, left/right/bottom
  resizable panels, central workspace, scroll containers, visibility, docking,
  saved layout state, and persisted offsets.
- Split pane collapse/restore, min/max sizes, and keyboard accessible resizing.
- Dock panel visibility and persisted size state.
- Tab strips for inspector/editor panels.
- Track list and arrangement scroll synchronization.
- Editor surfaces with world/view transforms, hover, hit testing, drag capture,
  marquee selection, snapping, cursor override, tool mode, and overlay layers.
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
- Screen-reader summary payloads for custom editor surfaces, with structured
  title, description, key/value items, instructions, and tree lookup helpers.
- Backend-facing accessibility adapter request/response contracts and host
  preference flags for screen reader, reduced motion, high contrast, forced
  colors, transparency, and text scaling.
- Existing core widgets and major widget families wired to richer accessibility
  states where the current APIs already expose that information.
- Orbifold and game-agent v3 wishlist documents preserved under `docs/`.
- Renderer-neutral paint extensions in `src/paint.rs` for gradient brushes,
  stroke alignment, corner radii, shadows/glows/inset shadows, anchored scene
  text with alignment and overflow policy, image placement, and path primitives.
- Renderer-neutral raw input and gesture contracts in `src/input.rs` for pointer
  identity/buttons, high-resolution wheel units, keyboard/text conversion,
  pointer capture, drag thresholds, double-click counting, cancellation, and
  gesture-to-edit phases.
- Persistable app-shell state contracts in `src/shell.rs` for docked/floating
  panel visibility, saved extents, collapse/restore, keyboard-resizable split
  state, active tabs, focus restore, and synchronized track/arrangement scroll
  offsets.
- Renderer-neutral testing helpers in `src/testing.rs` for event replay, raw
  input conversion checks, platform-output assertions, stable-name layout
  assertions, paint-list assertions, RGBA pixel diffs with tolerances, dirty
  flags, and frame timing sections.
- Renderer-neutral debug snapshots in `src/debug.rs` for layout bounds, clip
  rects, paint primitive counts, z ranges, host interaction flags, command
  scopes, active gestures, repaint reasons, frame timings, and hit-test traces.
- Renderer/backend adapter contracts in `src/renderer.rs` for render targets,
  resource deltas, dirty regions, paint batching, deterministic snapshots, and
  backend capability negotiation.
- Host adapter contracts in `src/host.rs` for hover, pressed, focused,
  drag-captured, text/IME, wheel-targeted, shortcut-routed, command dispatch,
  and repaint/platform-service state before paint.
- Command effect hooks in `src/commands.rs` for mapping enabled app commands to
  platform service requests or opaque app-owned effects, including clipboard,
  file dialog, screenshot, repaint, close-window, and quit requests.
- Editor-surface helpers in `src/editor.rs` for world/view transforms, hit
  testing, snapping, cursor/tool modes, marquee selection, drag capture, and
  overlay ordering.
- Retained display-list cache contracts in `src/display.rs` for static editor
  backgrounds, snapshot/display-list reuse, dirty-flag invalidation, and bounded
  cache eviction.
- Asset registry contracts in `src/assets.rs` for built-in common action icons,
  app-provided icon/image descriptors, sizing, tinting, alignment, compact
  icon-button metadata, tooltip text, and accessibility labels.
- Tooltip and shortcut-display contracts in `src/tooltips.rs` for platform-aware
  shortcut labels, command metadata tooltips, disabled reasons, and
  renderer-neutral tooltip requests.
- Data-table export helpers in `src/widget_ext/data.rs` for selected rows,
  visible rows, active cells, and ranges, with TSV/CSV formatting and clipboard
  command-effect integration.
