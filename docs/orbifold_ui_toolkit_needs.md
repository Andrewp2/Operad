# Orbifold UI Toolkit Needs

This note is written from the perspective of replacing Orbifold's primary egui UI with a toolkit that fits the app's domain. Orbifold is not a generic CRUD app. It is a low-latency musical workstation with dense controls, custom musical editors, live MIDI input, audio feedback, and scale/key-map visualizations that need to stay readable while state changes continuously.

The useful toolkit is therefore not just a widget library. It should be a boundary between Orbifold's real-time musical model and a deterministic, inspectable, testable interactive surface.

## Current UI Surface

Orbifold currently uses egui for both ordinary UI and custom drawing:

- Application shell: menu bar, toolbar, status bar, side panels, central workspace, and bottom clip panel.
- Configuration surfaces: audio output, MIDI input, scale tuning, synth parameters, key-map loading, view toggles, and persistence commands.
- Browsers and inspectors: scale library, audio asset browser, MIDI log, mapping capture panel, and selected event details.
- Musical editors: transport controls, clip note controls, piano-roll grid, wrapped loop note drawing, playhead, pitch lanes, and click/double-click selection/creation.
- Instrument visualizations: Lumatone board layout, hex keys, active-note highlights, capture-order markers, labels, and colors loaded from key-map data.
- Platform hooks: file dialogs, screenshots, close-window command, keyboard shortcuts, and repaint scheduling.

That mix is the main pressure. Orbifold needs commodity controls, but its important UI is custom editor geometry. A replacement toolkit should make custom surfaces first-class rather than forcing every editor through a widget escape hatch.

## Design Goals

The toolkit should make it easy to build UI that is:

- Low latency enough for musical feedback. MIDI note state, transport position, recording state, and active voices should feel live.
- Deterministic enough to test with snapshots, screenshots, event replay, and layout assertions.
- Explicit about ownership. UI rendering should not casually hold locks on audio, MIDI, or project state.
- Good at dense, professional tool layouts. Orbifold needs panes, tables, inspectors, menus, timelines, meters, editors, and compact controls more than landing-page style composition.
- Good at custom 2D drawing and hit testing. Piano rolls, scale views, modulation lanes, Lumatone maps, and future sequencers need exact geometry.
- Portable across windowing/rendering backends where practical. Orbifold should not tie its musical UI model to one immediate-mode renderer.
- Incrementally adoptable. Orbifold should be able to move one panel or editor at a time without rewriting the audio and project model.

## Non-Goals

The toolkit does not need to solve every UI problem immediately.

- It does not need a browser DOM or CSS clone.
- It does not need a large general-purpose component catalog before Orbifold can use it.
- It does not need animation-heavy marketing UI.
- It should not put audio/MIDI domain logic inside the toolkit.
- It should not require the real-time audio path to wait on layout, rendering, filesystem work, image encoding, or font shaping.

## The Most Important Boundary

The key boundary should be:

```text
Orbifold domain state -> UI snapshot -> UI tree/editor surfaces -> commands back to Orbifold
```

The UI should read stable snapshots and emit typed commands. It should not own the music project, synth, MIDI connection, or file formats.

For example, the piano roll should receive a `ClipViewModel` containing notes, loop length, selected note, pitch range, scale labels, quantize grid, playback beat, and recording state. It should emit commands such as `SelectNote`, `AddNoteAt`, `MoveNote`, `ResizeNote`, `SetVelocity`, `SetLoopLength`, and `SetQuantizeGrid`. Orbifold's app layer decides how those commands mutate project state, update undo history, or talk to the synth.

This keeps the toolkit useful beyond Orbifold while still allowing Orbifold-specific editors to be written cleanly.

## Runtime And Platform Layer

Orbifold needs a platform layer that owns the event loop and window integration, but does not leak everywhere.

It should provide:

- Window lifecycle: create, resize, scale-factor changes, close requests, fullscreen if needed later.
- Input events: pointer, wheel, keyboard, text input, modifiers, focus, drag capture, hover, and double-click timing.
- Clipboard and file dialogs through narrow services.
- Screenshot capture for regression tests and user-visible export.
- Repaint scheduling with explicit reasons: animation tick, transport tick, input event, async data update, or forced test frame.
- Time source injection so tests can render deterministic transport/playhead frames.

The toolkit should distinguish "needs another frame because the transport is moving" from "needs another frame because layout changed." Orbifold should be able to repaint the piano roll/playhead often without redoing unrelated expensive work.

## Layout Layer

The layout system should be retained enough to inspect and test, even if parts of the API feel immediate from the caller's perspective.

Needed primitives:

- Docked regions: top menu, toolbar, bottom status/clip panels, left browser, right inspector, central workspace.
- Resizable panes with persisted sizes.
- Scroll areas with stable coordinate spaces.
- Tables/grids for MIDI logs, device lists, tuning settings, and asset browsers.
- Toolbars with wrapping or overflow behavior.
- Popovers/menus with keyboard navigation.
- Modal and modeless dialogs where native dialogs are not appropriate.
- Fixed-format custom canvases with well-defined local coordinates.

The layout engine should expose final rectangles by stable IDs. That matters for editor hit testing, screenshot assertions, and debugging "why is this panel clipped?" issues.

## Rendering Layer

Rendering should be a separate layer from layout and widgets. Orbifold's custom editors need a small, dependable 2D scene API:

- Rectangles, rounded rectangles, lines, polylines, polygons, circles, text, images, clipping, and transforms.
- Stable color and stroke types.
- Pixel snapping or deliberate subpixel policy.
- Text measurement before layout finalization.
- Font fallback and monospace/proportional font selection.
- Z ordering that is explicit enough for editor overlays.
- Optional retained display lists for unchanged panels.

The piano roll and Lumatone grid should be expressible as normal drawing code, not as special cases. The renderer should also make it easy to render offscreen for tests and screenshots.

GPU acceleration is useful, but the first priority is a clean scene abstraction. Orbifold should not have to thread raw GPU concepts through every panel. A backend can lower scene primitives to wgpu, softbuffer, pixels, tiny-skia, or another renderer later.

## Widgets And Components

Orbifold needs a compact set of boring, reliable controls:

- Buttons, icon buttons, checkboxes, radio groups, segmented controls, sliders, drag values, text inputs, combo boxes, menus, list rows, tabs, splitters, and scroll bars.
- Numeric controls that understand units, ranges, precision, and step size.
- Disabled state, hover state, active state, focus ring, validation state, and tooltips.
- Keyboard activation and focus traversal.
- A command-routing model for shortcuts.

The widgets should not directly mutate arbitrary application fields. Prefer typed value bindings or command emission. For example, a synth gain slider can produce `SetSynthParam { parameter: MasterGain, value }`, while Orbifold decides whether to apply it immediately, persist it, coalesce it into undo, or reject it.

## Custom Musical Editors

The toolkit should treat editor surfaces as a core use case.

For the piano roll, useful support includes:

- World-to-screen transforms for beat and pitch axes.
- Hit testing against notes, handles, lanes, grid lines, and empty space.
- Drag gestures with capture, thresholds, snapping, cancellation, and modifier behavior.
- Multi-selection and marquee selection.
- Loop wrapping, so notes that cross the loop boundary can render and hit-test as multiple rects while remaining one musical note.
- High-density labels that can be hidden, clipped, or elided based on zoom.
- Playhead overlays that update independently of note layout.

For Lumatone and other isomorphic layouts:

- Polygon hit testing.
- Hex-grid coordinate helpers.
- Per-key fill/stroke/text layers.
- Active-note and captured-note overlays.
- Label contrast helpers.
- Efficient redraw when only active notes change.

Future editors will likely need automation lanes, scale-degree maps, tuning curves, modulation graphs, sample browsers, and routing views. The toolkit should make those feel like first-class editor canvases, not hacks beside the widget system.

## State, Commands, And Undo

The UI toolkit should not own Orbifold's undo stack, but it should help produce commands that are easy to coalesce.

Useful event phases:

- `Preview`: pointer hover, pending drag, scrub feedback, tooltip data.
- `BeginEdit`: start of slider drag, note move, resize, recording arm, or text edit.
- `UpdateEdit`: continuous value changes.
- `CommitEdit`: final value for undo history and persistence.
- `CancelEdit`: escape key, focus loss, or failed validation.

This distinction matters for audio tools. A slider may update the synth continuously, but undo should record one edit when the drag ends. A note drag may audition notes while moving, but the project history should remain coherent.

## Real-Time Safety

The audio callback and MIDI callback paths should never depend on UI progress.

Toolkit-facing rules:

- UI reads snapshots, not live structures that require long-held locks.
- Commands back to Orbifold are queued or applied on the app thread.
- Expensive operations are explicit tasks: file IO, image decode, audio asset scan, screenshot encode, font load, and project save.
- Live indicators use atomics or small snapshots where practical.
- Rendering never calls into synth code that can block.

Orbifold currently has shared state protected by mutexes for scale, MIDI log/capture, Lumatone map, synth settings, and music project data. A toolkit migration is a good opportunity to centralize snapshot creation so draw code does not scatter lock acquisition across every panel.

## Styling And Theme

The toolkit should provide a small design-token system rather than hard-coded colors in every editor:

- Surface, raised surface, panel border, grid line, active line, text, muted text, warning, record, selection, note fill, root lane, black-key lane, and accent colors.
- Spacing, control height, panel padding, row height, corner radius, and stroke widths.
- Font roles: UI, monospace data, compact label, editor label, title.

Orbifold has dense musical data. The theme should optimize for contrast, scanability, and long sessions, not visual novelty. It should also allow editor-specific semantic colors, because a scale degree, selected clip note, active MIDI key, and armed recording state should not all compete for one generic accent.

## Accessibility And Input Completeness

Even if accessibility is not the first implementation milestone, the abstraction should not rule it out.

Needed foundations:

- Stable semantic IDs.
- Labels and roles for controls.
- Focus order.
- Keyboard equivalents for commands.
- Screen-reader text for controls where possible.
- Respect for platform text scale if practical.

Orbifold also needs serious keyboard handling. Global commands like play/stop, record, undo, save, note delete, duplication, nudging, and pitch changes must coexist with focused text/numeric fields. The toolkit should make shortcut scopes explicit: global, panel, editor surface, focused control, and text input.

## Testing And Debuggability

This is where a custom toolkit can be more useful than egui for Orbifold.

I would want:

- Deterministic layout tests that assert rectangles for important controls and editor regions.
- Golden screenshot tests for the app shell, piano roll, empty clip, selected note, Lumatone map, active notes, and capture mode.
- Event replay tests for note selection, double-click creation, note dragging, shortcut routing, and menu commands.
- A debug overlay showing layout bounds, IDs, focus, hovered item, active gesture, repaint reason, and command log.
- A way to dump the UI tree/display list as text.
- A frame budget trace: snapshot time, layout time, paint list build, GPU upload, render, and event handling.

For a DAW-like app, testing interaction geometry is as important as testing widgets. A bug in note hit testing or drag snapping is a real product bug.

## Incremental Migration Strategy

The first target should not be the whole app. The lowest-risk migration path is:

1. Define Orbifold-facing view models and command enums while still rendering with egui.
2. Extract custom editor geometry into backend-neutral structs: piano-roll layout, Lumatone layout, hit-test results, and draw primitives.
3. Add an Operad-backed custom canvas for one editor, probably the Lumatone grid or piano roll.
4. Move shell layout after the custom editor path is proven.
5. Replace menu/panel/widgets once command routing, focus, and screenshot testing are solid.

This avoids mixing "new toolkit design" with "rewrite Orbifold's project/audio model." The UI boundary should improve the code even before egui is fully removed.

## API Shape I Would Want

A useful Orbifold-facing API might look conceptually like this:

```rust
fn orbifold_ui(ui: &mut Ui, model: &OrbifoldViewModel) -> Vec<OrbifoldCommand> {
    ui.shell("orbifold_shell", |shell| {
        shell.top(menu_bar);
        shell.top(toolbar);
        shell.left_resizable("scale_library", model.show_scale_library, scale_library);
        shell.right_resizable("inspector", model.show_inspector, inspector);
        shell.center(workspace);
        shell.bottom(clip_panel);
        shell.bottom(status_bar);
    })
}
```

For custom editors:

```rust
ui.canvas("piano_roll", desired_size, |canvas, input| {
    let layout = PianoRollLayout::new(canvas.rect(), model.clip, model.scale);
    draw_piano_roll(canvas, layout, model);
    piano_roll_interaction(input, layout, model)
})
```

The important part is not this exact syntax. The important part is that drawing, layout, input, hit testing, and command emission are separable enough to test individually.

## Open Questions

- Should the toolkit be immediate-mode at the API level, retained internally, or retained at both levels?
- Should layout be custom, Taffy-like, or a small domain-specific pane/grid system first?
- Should rendering start CPU-backed for correctness and screenshots, then add GPU, or start with wgpu immediately?
- How much text editing is needed inside Orbifold soon?
- Should native menus be supported, or should Orbifold draw all menus itself for portability and screenshot consistency?
- What is the minimum accessibility target for the first usable version?
- How should async tasks report progress and errors into the UI command/status system?

## Practical First Deliverable

The most useful first deliverable for Orbifold would be a small toolkit slice that can render and test one custom editor:

- Window/input wrapper.
- Basic shell layout.
- Canvas primitive.
- Text and shape rendering.
- Stable IDs and final rect inspection.
- Pointer hit testing and gesture capture.
- Screenshot test harness.
- Command emission back to Orbifold.

If that slice can replace the Lumatone grid or piano roll without making the app architecture worse, the toolkit will be pointed in the right direction.
