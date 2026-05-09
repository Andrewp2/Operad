# Orbifold Wishlist For Operad 3.0

This is a forward-looking wishlist from the Orbifold side after starting the
Operad 1.0 integration and looking at the Operad 2.0 direction. Orbifold can keep
moving on 1.0/2.0 for custom scene surfaces, but building a polished DAW-style UI
similar to the current visual target needs more from Operad than layout, widgets,
and neutral paint primitives.

The target is a dense music workstation UI: transport at the top, project and
asset browsers on the left, track list and arrangement in the center, piano roll
and automation editors below, inspector/helper panels on the right, and lots of
small controls that must stay readable for long sessions.

## Guiding Boundary

Operad should not become Orbifold's music engine. Orbifold should still own:

- Transport and playback state.
- Clips, notes, automation, tracks, scales, tunings, MIDI, audio, and undo.
- Product-specific commands such as quantize, record, duplicate note, audition
  chord, load scale, route MIDI, and save project.

Operad should own the reusable UI machinery:

- The app shell, panels, docking, menus, widgets, popovers, and list mechanics.
- Renderer-neutral visual primitives and theme tokens.
- Focus, keyboard, pointer gestures, drag capture, accessibility metadata, and
  testable layout/paint snapshots.
- Editor-surface helpers that turn domain coordinates into view geometry without
  owning the domain model.

The best API shape remains:

```text
Orbifold snapshot -> Operad document/editor surfaces -> typed Orbifold commands
```

## 1. Theme System And Design Tokens

Orbifold needs a first-class theme layer. The reference UI depends on coherent
semantic color and spacing, not arbitrary per-widget colors.

Needed:

- Theme structs for color, spacing, typography, radius, stroke, shadow, opacity,
  and motion.
- Semantic roles: app background, panel, panel raised, panel inset, divider,
  active outline, focus ring, muted text, strong text, warning, record, transport
  active, meter active, clip selected, clip muted, grid major, grid minor, editor
  playhead, scale root, scale degree, and disabled state.
- Component tokens for buttons, tabs, search fields, track headers, clip blocks,
  piano roll lanes, velocity bars, property rows, menu rows, and transport
  controls.
- Theme inheritance or scoped themes so editor surfaces can use musical colors
  while shell widgets keep consistent UI colors.
- Light/dark support eventually, but the first requirement is one excellent dark
  theme with semantic roles.

Why this matters: Orbifold will have dozens of small surfaces. Without theme
tokens the UI will either drift visually or require hard-coded colors in every
panel.

## 2. Richer Paint Primitives

Operad 1.0 scene primitives are enough to start, but the target UI needs richer
renderer-neutral paint output.

Needed:

- Rounded rectangles as first-class paint primitives, not polygon approximations.
- Stroke alignment: inside, center, outside.
- Linear gradients and simple multi-stop gradients.
- Layer shadows and glows with stable semantics, even if some backends approximate
  them.
- Inner shadows or inset borders for dense panels and editors.
- Text clipping, elision, alignment, and baseline positioning.
- Image/icon handles with explicit sizing, tinting, and alignment.
- Path primitives for automation curves and waveform-like displays.
- Per-layer opacity and clipping groups.
- Pixel snapping policy for hairlines and grid lines.

Nice later:

- Blur/material effects behind panels.
- Cached display lists for static editor backgrounds.
- Backdrop filters for floating popovers.

The immediate priority is not visual flourish; it is making the reference UI
possible without drawing every surface as a special-case egui escape hatch.

## 3. Icon And Asset System

The target UI uses many compact icon buttons. Orbifold should not hand-roll icon
paths inside app code.

Needed:

- Renderer-neutral icon handles.
- A small built-in icon registry for common UI actions: play, stop, record,
  loop, metronome, settings, add, search, folder, collapse, expand, mute, solo,
  lock, snap, zoom, link, scissors, pencil, pointer, grid, and more.
- Icon-button widgets with tooltip/accessibility labels.
- Icon tint states: normal, hover, pressed, active, disabled, warning.
- Optional app-provided icon/image registry for Orbifold-specific icons such as
  track types, scale/tuning markers, Lumatone, and instrument categories.

## 4. App Shell And Workspace Composition

Operad 2.0 has split/dock direction. Orbifold needs those pieces to become a
smooth shell API that matches workstation layout.

Needed:

- Persistent multi-region workspace:
  - top transport/menu bar
  - left project/scale/asset browser
  - track list
  - central arrangement timeline
  - bottom editor area
  - right helper/scale-lab/inspector panels
  - bottom status strip
- Split pane state with persisted sizes and collapse/restore.
- Dock panel visibility, min/max sizes, resize handles, and keyboard accessible
  resizing.
- Tab strips for secondary panels: helper, scale lab, notes; piano roll, hex
  view, automation, mixer.
- Scroll synchronization between track list and arrangement lanes.
- Layout snapshots that make it easy to assert "the piano roll occupies this
  rectangle" and "the inspector is visible at this size."

The shell should make common DAW layouts easy, while still allowing Orbifold to
own the actual track/clip/editor contents.

## 5. Transport And Toolbar Widgets

Orbifold needs dense, professional controls at the top of the app.

Needed components:

- Transport cluster: rewind, play/pause, stop, record, loop.
- Numeric readouts with labels and small sublabels: BPM, time signature, key/root,
  scale, buffer, CPU, voices, disk, MIDI in.
- Dropdown-select controls compact enough for scale/tuning selection.
- Toggle buttons with active indicator and accessible state.
- Segmented controls for editing modes.
- Mini meters, activity dots, and connection indicators.
- Toolbar overflow behavior for smaller widths.
- Tooltips with shortcut hints.

Interaction details:

- Buttons need pressed/latched/disabled states.
- Numeric controls need wheel, drag, text entry, min/max, precision, and commit
  phases.
- Transport controls need keyboard shortcuts and command routing, but Operad
  should only emit command IDs.

## 6. Track List And Arrangement Helpers

The center of the reference UI is a track list plus timeline arrangement. Operad
should provide reusable geometry and interaction helpers, not Orbifold-specific
music models.

Needed:

- Track header widget:
  - color/icon strip
  - name and subtitle
  - mute/solo/arm buttons
  - volume/pan mini controls
  - selected/hover/active states
- Arrangement grid helpers:
  - bar/beat ruler
  - measure subdivisions
  - snap lines
  - vertical track lanes
  - playhead
  - loop/selection regions
- Clip block primitives:
  - rounded colored clip blocks
  - selected/hovered/muted states
  - text labels
  - miniature MIDI note/waveform/automation previews
  - resize handles
  - drag ghosts
- Coordinate transforms:
  - beat/time to x
  - track index to y
  - zoom/pan
  - snap-to-grid helpers

Orbifold should pass track/clip snapshots and receive gestures or command intents
such as select clip, move clip, resize clip, split clip, duplicate clip, open
clip editor.

## 7. Piano Roll, Automation, And Editor Surface Infrastructure

Orbifold's current piano roll is the most important custom editor. Operad should
provide the boring reusable machinery around it.

Needed:

- Editor surface abstraction with:
  - world/view transform
  - clipping
  - hit testing
  - hover state
  - drag capture
  - marquee selection
  - snapping
  - cursor override
  - tool mode
  - overlay layers
- Piano roll helpers:
  - pitch lane geometry
  - scale-degree lane coloring
  - keyboard/pitch labels
  - bar/beat grid
  - note rectangle hit testing
  - note resize handles
  - velocity lane geometry
  - selected-note overlays
- Automation helpers:
  - point and curve rendering
  - handle hit testing
  - segment insertion
  - snapping and value scaling
  - interpolation display
- Ruler and playhead helpers that can be reused between arrangement and editor.

This is where Operad should make editor code less fragile. Orbifold should still
own the musical commands and undo grouping.

## 8. Gesture And Edit Phase Model

Orbifold needs a serious pointer/keyboard interaction model for editing music.

Needed:

- Pointer capture with explicit gesture IDs.
- Drag threshold and cancellation.
- Double-click/triple-click timing and hit classification.
- Hover, pressed, dragging, committed, cancelled states.
- Modifier-aware gestures.
- Touchpad/wheel handling with high-resolution deltas.
- Edit phases:
  - preview
  - begin edit
  - update edit
  - commit edit
  - cancel edit
- Coalescing hooks so a slider drag or note drag becomes one undoable edit in
  Orbifold.

Examples:

- Dragging a MIDI note should preview movement continuously but commit once.
- Resizing a clip should show snap feedback and commit once.
- Moving a synth slider should update audio continuously but produce one
  undo/persistence event.

## 9. Command Routing And Shortcut Scopes

Orbifold needs keyboard-heavy workflows. Operad should make scopes explicit.

Needed:

- Command registry with opaque app-owned command IDs.
- Shortcut bindings with platform-aware modifiers.
- Scope hierarchy:
  - global app
  - active workspace
  - focused dock panel
  - focused editor surface
  - focused text/numeric field
  - menu/popover modal scope
- Conflict detection and debug output.
- Command palette integration.
- Menu item integration.
- Tooltip shortcut display.
- Event replay support for command routing tests.

Operad should not implement "quantize clip"; it should route "orbifold.quantize"
to Orbifold when the focused scope allows it.

## 10. Text, Numeric, And Property Editing

The right inspector and helper panels need high-quality compact form controls.

Needed:

- Property row layout with label/value/help/error slots.
- Numeric fields with:
  - drag-to-adjust
  - wheel-to-adjust
  - direct text entry
  - unit suffixes
  - step sizes
  - precision
  - validation
  - commit/cancel phases
- Text input with selection painting, caret painting, clipboard, IME path, and
  platform text services.
- Dropdowns and combo boxes with search/filter for large scale/tuning lists.
- Color swatches and palette rows.
- Disabled, read-only, invalid, changed, and pending states.

This is necessary for synth parameters, clip properties, scale settings, device
selection, and future routing/mixer panels.

## 11. Data Views, Trees, And Browsers

Orbifold's left side needs project lists, scale lists, asset folders, search, and
event logs.

Needed:

- Tree view with folders, disclosure arrows, icons, selection, drag/drop, and
  keyboard navigation.
- Search field with clear button and delayed filtering hooks.
- Virtualized lists with section headers.
- Multi-column compact tables for MIDI logs and mapping capture.
- Row actions and context menus.
- Empty states that are compact and not marketing-like.
- Stable row IDs so selection survives filtering and sorting.
- Drag/drop surface metadata for importing assets and moving clips/tracks later.

## 12. Menus, Popovers, And Inspectors

Operad 2.0 appears to have strong menu/popover direction. Orbifold still needs
these to become practical in an app shell.

Needed:

- Menu bar that can replace the current egui menu bar.
- Context menus over editors, tracks, clips, notes, and browser rows.
- Popovers anchored to controls, with placement and clipping that works inside
  docked panels.
- Inspector sections with collapsible groups and compact headers.
- Keyboard navigation through menus and popovers.
- Escape/outside-click dismissal policy.
- Accessibility metadata and focus restore after dismissal.

## 13. Visual State Debugging And Tooling

Dense UI work needs strong inspection tools.

Needed:

- Debug overlay showing:
  - layout bounds
  - clip rects
  - z-order
  - hovered/pressed/focused nodes
  - active gesture
  - active command scope
  - repaint reason
- Paint-list dump with stable node names and primitive counts.
- Layout snapshot serialization.
- Theme token inspector.
- Hit-test trace for a point.
- Frame timing sections: snapshot, layout, paint build, render, input dispatch.

This should be available without adding one-off debug code to Orbifold panels.

## 14. Screenshot And Interaction Tests

Orbifold will need visual regression coverage before the UI becomes ambitious.

Needed:

- Offscreen renderer or deterministic screenshot backend.
- Golden screenshot harness that can render a document at fixed sizes.
- Pixel-diff tooling with tolerances.
- Event replay for:
  - opening menus
  - selecting browser rows
  - dragging notes
  - resizing clips
  - scrolling lists
  - keyboard shortcuts
- Layout assertions by node name.
- Paint assertions for important editor primitives.

The current Orbifold screenshot smoke test catches only catastrophic failures.
That is not enough for a polished workstation UI.

## 15. Performance And Incremental Rendering

The reference UI has many tracks, clips, rows, meters, and editor primitives.
Operad needs predictable performance.

Needed:

- Dirty flags: layout-dirty, paint-dirty, input-dirty, theme-dirty.
- Retained display lists for static editor backgrounds.
- Virtualized rows and clip lanes.
- Cheap updates for playhead and meters.
- Avoiding full text measurement when only the playhead moves.
- Primitive batching guidance for render backends.
- Stable allocation patterns for per-frame scene generation.

Orbifold should be able to update playhead/meter state at high frequency without
rebuilding every panel and every list.

## 16. Accessibility Baseline

Orbifold is an expert app, but accessibility should not be bolted on later.

Needed:

- Roles for buttons, toggles, sliders, tabs, menu items, tree rows, table cells,
  editor surfaces, and splitters.
- Names, values, hints, disabled/selected/checked states.
- Focus order and focus traps for modal surfaces.
- Keyboard access to splitters, tabs, menus, and property controls.
- Screen-reader summaries for custom editor surfaces.
- Reduced-motion and high-contrast hooks eventually.

## 17. Platform Services

Operad does not need to own the platform, but it should define clean service
boundaries.

Needed service traits or adapters:

- Clipboard.
- File dialogs.
- Drag/drop payloads.
- Cursor/icon changes.
- Open URL.
- Native notifications or toast fallback.
- Text input/IME.
- Screenshot capture.

Orbifold can provide implementations through eframe/winit/native APIs, but Operad
should give UI components a consistent way to request services without importing
Orbifold.

## 18. Immediate Priorities For V3

If Operad 3.0 has to choose, I would prioritize in this order:

1. Theme tokens and component visual states.
2. Rich paint primitives: rounded rects, gradients, shadows, text alignment and
   elision, image/icon handles.
3. Command routing and shortcut scopes.
4. Editor gesture phases and hit-test helpers.
5. App shell helpers: persisted dock/split/tab layout.
6. Property/numeric controls with commit/cancel semantics.
7. Tree/list/table polish for browsers and inspectors.
8. Screenshot/layout/interaction test harness.

Those eight areas would let Orbifold replace large egui surfaces with Operad
without waiting for Operad to know anything about music.

## What Orbifold Can Do Meanwhile

Orbifold can keep integrating the current stable Operad baseline by:

- Moving custom drawing surfaces to Operad scene primitives.
- Keeping egui as a temporary backend for text overlays and shell widgets.
- Defining Orbifold view-model snapshots and command enums.
- Extracting editor geometry and hit testing into backend-neutral code.
- Feeding concrete pain points back into Operad 3.0 rather than asking Operad to
  guess from abstract requirements.

The main thing I want from Operad 3.0 is not "a DAW framework." I want a robust,
themeable, testable UI toolkit where Orbifold can build a DAW without fighting
layout, rendering, focus, and gesture infrastructure every step of the way.
