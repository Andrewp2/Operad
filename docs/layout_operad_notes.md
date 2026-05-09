# Codex Notes on Operad

These notes describe what I would want from Operad as a shared UI foundation for
Fabricad, the DAW, and the game. I intentionally wrote this from the current
code shape and our product needs, without reading any other design note in this
directory.

## Short Version

Operad should be a native Rust UI model and rendering boundary, not a wrapper
around any one UI toolkit. Its most useful role is to give our apps a retained,
testable, renderer-agnostic UI document that can eventually be painted by egui,
wgpu, or another backend.

The core crate should stay small and boring:

- Document tree
- Layout inputs and computed layout
- Hit testing
- Focus and input routing
- Scroll state
- Basic animation state
- Neutral style types
- Renderer-independent paint/display lists
- Testing and screenshot-audit hooks

Everything backend-specific should sit outside the core: egui, wgpu, glyphon,
cosmic-text, platform windows, app command systems, and product-specific widgets.

## Why Operad Is Worth Having

The common problem across the three apps is not just that egui looks rough. It is
that immediate-mode UI makes it easy to accidentally create layouts that are hard
to audit, hard to virtualize, hard to render outside egui, and hard to make feel
like a serious native application.

Fabricad especially needs UI that can be inspected and verified. We need to know
that every view fits at multiple aspect ratios, that no text falls off a panel,
that scroll regions are real, that tables paginate or virtualize correctly, and
that viewport overlays do not fight application chrome. A retained document model
is the right foundation for that.

For the DAW, Operad could give us predictable native app controls and a better
foundation for piano-roll and timeline overlays. For the game, Operad can keep
menus and HUDs independent from egui while still allowing an egui adapter during
the migration.

## Core Abstraction Boundaries

### `operad-core`

This should be the minimal, renderer-independent layer.

It should own:

- `UiDocument`, `UiNode`, stable node ids, tree mutation
- Geometry types: point, size, rect, insets
- Neutral style types: color, stroke, radius, opacity, text style, layout style
- Layout execution through a backend abstraction, probably Taffy internally
- Computed layout cache and invalidation
- Hit testing and z ordering
- Focus traversal and focus scopes
- Pointer capture, keyboard focus, and event routing
- Scroll containers and scroll state
- Basic transition/animation state
- Accessibility metadata as data, even if backends ignore it initially
- A renderer-neutral paint list or display list

It should not depend on:

- egui
- glyphon
- wgpu
- winit
- eframe
- product crates
- app-specific command enums

Taffy is acceptable in core for now because layout is a central job of the core.
If we want to be stricter, we can wrap Taffy behind our own layout style types so
consumers do not build directly against Taffy APIs.

### `operad-egui`

This adapter should let us embed Operad documents inside existing egui apps.

It should own:

- Conversion from egui input into Operad input events
- Painting an Operad display list through egui
- Temporary bridging for egui fonts and text rendering
- Debug overlays for layout boxes, focus, clipping, and hit regions

It should not define core UI behavior. If button behavior, scroll behavior, text
editing, or table virtualization lives only in the egui adapter, then Operad has
not actually solved the portability problem.

### `operad-text`

Text should be optional and carefully separated.

The core should define neutral text concepts:

- Font family preference
- Size and line height
- Weight
- Italic/style
- Wrap mode
- Alignment
- Color

The core should expose a `TextMeasurer` trait and include a cheap approximate
measurer for tests and simple tools.

A separate text crate or feature can provide real shaping and measurement. I
would prefer `cosmic-text` for this layer. I would not put `glyphon` in core,
because glyphon is primarily a wgpu text renderer and brings graphics
dependencies into places that only need measurement.

### `operad-wgpu`

This should be the eventual native renderer.

It should own:

- Rect, stroke, image, and mesh rendering
- Text rendering, likely through glyphon or another wgpu text path
- Texture atlas/image resource management
- Clipping/scissor management
- GPU batching
- Render-target integration

The wgpu renderer should consume the same display list that the egui adapter
consumes. That keeps core behavior testable without a GPU.

### Widget Layer

I would keep widgets separate from the document core.

Possible shape:

- `operad-widgets` for reusable controls
- Product crates compose those widgets into domain views
- Apps can still build nodes directly when needed

Widgets should emit semantic actions rather than owning application state. For
example, a command palette widget should emit `CommandSelected(id)`, not know
about Fabricad's command enum directly.

## What I Need Before Using It Seriously

### Real Scroll Containers

This is the first hard requirement for Fabricad.

Needed behavior:

- Vertical and horizontal scroll offsets
- Wheel and trackpad input
- Scrollbar thumb geometry
- Dragging scrollbar thumbs
- Programmatic scroll-to-node
- Nested scroll region behavior
- Clipping that matches hit testing and painting

Without this, Operad cannot replace the egui panels that currently fail when
content is taller than the window.

### Text Input

Text editing has to be a core interaction, not a page-specific hack.

Needed behavior:

- Single-line and multiline editing
- Caret movement
- Selection
- Backspace/delete
- Home/end and word movement
- Clipboard copy/paste/cut through platform adapters
- IME composition hooks
- Mouse selection
- Focus loss/commit behavior

Fabricad uses filters, name fields, markdown editing, property editors, and
command palette input. The DAW also needs project names, numeric entry, search,
and possibly lyrics/notes later.

### Virtual Lists and Tables

Fabricad has many tables and lists where "render every row" is the wrong model.

Needed behavior:

- Fixed-height virtual list
- Variable-height virtual list eventually
- Sticky headers
- Column sizing
- Sort/filter state hooks
- Row selection
- Keyboard navigation
- Pagination helpers for small datasets
- Displaying "showing N-M of K" as a standard pattern

This is where Operad can become more useful than egui for us. It should make the
correct path easy for large data views.

### Command Palette and Menu Model

The command palette should be a reusable system:

- Command registry
- Search/filter
- Keyboard navigation
- Recent/frequent ranking later
- Shortcut display
- Action dispatch through app-provided callbacks

Menus should also be data-driven enough that Fabricad, the DAW, and the game can
share the same behavior while rendering different command sets.

### Docking, Splitters, and Panels

Fabricad wants professional application chrome:

- Pinned navigation rail
- Optional panels
- Docked side panels
- Floating panels
- Split panes
- Resizable panels
- Responsive collapse rules

This does not all need to be in v1, but the core layout model should not make it
hard to add.

### Canvas and Viewport Overlays

Operad should not try to own every high-performance canvas immediately.

For Fabricad layout/3D and the DAW piano roll, I want a clear escape hatch:

- App owns the domain renderer/canvas
- Operad owns chrome and overlays
- Hit testing can decide whether UI or canvas receives input
- Viewport fullscreen can be modeled as layout state, not an egui special case

This lets us migrate app chrome without blocking on rewriting every custom
renderer.

## Display List Boundary

The right renderer boundary is probably a display list.

Core produces something like:

- Rect fill
- Rect stroke
- Text run
- Image
- Custom mesh or custom paint callback
- Clip push/pop or clip rect per item
- Z order
- Transform/opacity

Backends consume this list:

- `operad-egui` paints it through egui
- `operad-wgpu` batches and renders it natively
- A test backend can inspect it without a window

This is also useful for audit tooling. We can assert that nodes are inside their
clips, that text has finite measured bounds, and that no interactive node is
unreachable.

## Testing and Audit Hooks

This is where Operad can be especially useful to me.

I want a standard harness that can:

- Build each product view at named viewport sizes
- Compute layout without opening a window
- Export a layout tree as JSON
- Render screenshots through a deterministic backend
- Check for text overflow
- Check for clipped interactive controls
- Check for missing scroll containers
- Check for unreachable focusable elements
- Compare screenshots to baselines where useful

Fabricad needs this badly. The recent UI issues were often not logic bugs; they
were layout/audit failures.

## Suggested Crate Layout

A practical structure:

```text
operad/
  crates/
    operad-core/
    operad-egui/
    operad-text-cosmic/
    operad-wgpu/
    operad-widgets/
    operad-audit/
  examples/
    egui_shell.rs
    scroll_table.rs
    command_palette.rs
    daw_transport.rs
    fabricad_panel.rs
```

If we keep a single crate for now, use features with strict boundaries:

```text
default = ["egui"]
egui = ["dep:egui"]
text-cosmic = ["dep:cosmic-text"]
wgpu = ["dep:wgpu", "dep:glyphon"]
widgets = []
audit = ["serde"]
```

The important rule is that disabling default features should not pull wgpu.

## API Shape I Would Prefer

The core should make app code read like it is building a document, not like it is
manually pushing rectangles.

Example direction:

```rust
let root = document.root();
let panel = document.panel(root, Panel::new().layout(fill()).scroll_y());
let search = document.text_input(panel, TextInput::new("Search").placeholder("recipe, lot, rule"));
let table = document.virtual_table(panel, Table::new(columns).row_count(rows.len()));
```

But it should still allow low-level node construction for custom views.

Actions should be typed outside the core:

```rust
enum AppAction {
    RunCommand(CommandId),
    SelectRow(RowId),
    EditLayerName(LayerId, String),
}
```

Widgets can return neutral event payloads or user-supplied action ids. Core
should not know product-specific action enums.

## Adoption Strategy

I would not rewrite Fabricad first.

Better order:

1. Make Operad core dependency-light and split egui/text boundaries.
2. Port the game menu to consume Operad through the new package cleanly.
3. Port one DAW surface, probably transport plus side panels.
4. Add real scroll containers and text input.
5. Port a Fabricad small panel with known pain, such as Sidebar Modules or
   command palette.
6. Add virtual tables.
7. Port Fabricad Notebook or Layout Diff controls.
8. Only then consider replacing broad Fabricad chrome.

This order keeps risk down and gives the library real consumers quickly.

## Non-Goals

Operad should not become:

- A Tauri/webview abstraction
- A thin wrapper around egui
- A game HUD-only library
- A Fabricad-only design system
- A renderer that requires wgpu just to compute layout
- A place for application domain state

## Current Concerns

The current extracted crate is useful, but these should be addressed early:

- `glyphon` is in the core dependency list, which pulls wgpu even without egui.
- `TextStyle` exposes backend-specific text types.
- Input events are too small for serious app UI.
- Scroll containers are not core behavior yet.
- There is no widget layer yet.
- There is no display-list boundary yet.
- The egui renderer is inside the same source file as core behavior.
- The public API exposes `taffy::Style` directly, which may make future layout
  abstraction harder.

None of these are blockers for the first extraction, but they are the next set of
decisions that will determine whether Operad becomes a reusable foundation or
just a shared copy of the game UI experiment.

## Bottom Line

I want Operad to be the place where layout, input routing, scroll behavior,
focus, and view auditability become reliable across our native Rust apps.

The best version of Operad is not "our own egui" and not "a wgpu renderer." It is
the stable UI document and interaction model that can survive multiple renderers.
That boundary would make it useful immediately through an egui adapter and much
more valuable later through a native wgpu backend.
