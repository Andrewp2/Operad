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

## Theme Track

Add a first-class theme model with semantic tokens:

- Color, typography, spacing, radius, stroke, shadow, opacity, and motion tokens.
- Component tokens for buttons, tabs, search fields, track headers, clip blocks,
  piano-roll lanes, property rows, menu rows, and transport controls.
- Scoped theme inheritance so editor surfaces can use musical colors while shell
  widgets remain visually consistent.
- One excellent dark theme before broad light-theme work.

## Paint And Asset Track

Add renderer-neutral primitives and resource handles that make dense app
surfaces possible without leaking backend types into consumer state:

- Rounded rectangles with stroke alignment.
- Linear gradients and simple multi-stop gradients.
- Shadows, glows, inset borders, and clear fallback semantics.
- Text alignment, baseline positioning, clipping, and elision.
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
- Pointer capture, drag thresholds, double-click timing, cancellation, modifiers,
  high-resolution wheel deltas, and edit phase coalescing.

## Shell And Editor Track

Make common DAW layouts easier to assemble:

- Persistent top, left, track, arrangement, editor, right, and status regions.
- Split pane collapse/restore, min/max sizes, and keyboard accessible resizing.
- Dock panel visibility and persisted size state.
- Tab strips for inspector/editor panels.
- Track list and arrangement scroll synchronization.
- Editor surfaces with world/view transforms, hover, hit testing, drag capture,
  marquee selection, snapping, cursor override, tool mode, and overlay layers.

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
- `UiDocument::accessibility_snapshot()` with nodes, focus order, and modal
  scope.
- Existing core widgets and major widget families wired to richer accessibility
  states where the current APIs already expose that information.
- Orbifold and game-agent v3 wishlist documents preserved under `docs/`.
