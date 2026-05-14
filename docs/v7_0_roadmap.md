# Operad 7.0 Roadmap

V7 should turn Operad from a usable native Rust UI toolkit into a toolkit that is
comfortable for building real application screens. V6 proved the native host,
widget showcase, WGPU canvas path, and module ownership direction. V7 should
finish the structural work that did not make the v6 release, close the obvious
widget parity gaps, and make performance easier to understand.

There was no standalone `docs/v6_0_roadmap.md` when this document was written.
The v6 carryover below comes from `docs/v6_0_module_organization.md`,
`docs/v6_0_release_checklist.md`, `docs/widget_inventory.md`, and the older
consumer gap notes in `docs/fabricad_operad_v3_ui_gaps.md` and
`docs/game_agent_operad_notes.md`.

External parity references:

- egui 0.34.2 widgets and `WidgetType`:
  <https://docs.rs/egui/latest/egui/widgets/> and
  <https://docs.rs/egui/latest/egui/enum.WidgetType.html>
- egui `Ui` methods and container helpers:
  <https://docs.rs/egui/latest/egui/struct.Ui.html>
- iced 0.14 widget module:
  <https://docs.rs/iced/latest/iced/widget/>

## V7 Product Goal

Operad should be easy to evaluate and easy to adopt:

- A customer should be able to open the README, run the showcase, and understand
  how to build a normal app without reading internal planning docs.
- Common widgets should have normal defaults: spacing, text alignment, hover,
  pressed, disabled, focus, keyboard, clipboard, scrolling, and accessibility
  behavior should work without per-example patching.
- Overlay and floating surfaces should use one ordering model for paint, hit
  testing, focus, popups, and accessibility.
- Performance problems should be explainable from instrumentation instead of
  guessed from how laggy the showcase feels.
- Examples should teach public APIs. Tests, stress probes, and visual regression
  scenarios should live in test/diagnostic infrastructure, not in the showcase.

## V6 Carryover

These are the items that should move from the v6 planning notes into v7 work.

1. Split `src/lib.rs`
   - Current state: `src/lib.rs` is still a large implementation file.
   - V7 target: `lib.rs` is only crate docs, module declarations, and public
     re-exports.
   - Move out geometry primitives, visual/style/text primitives, the document
     tree, layout/focus/scroll integration, widget helpers, and module-local
     tests.

2. Add a public prelude
   - V6 deferred `prelude.rs` until the document/widget split was clearer.
   - V7 should add `operad::prelude` after deciding the stable import set.
   - The prelude should be small: common document, widget, style, action, and
     runtime types only.

3. Decide compatibility aliases
   - V6 kept old flat paths as compatibility aliases in several areas.
   - V7 should decide which aliases are deprecated for one cycle, which remain
     stable, and which are removed as breaking changes.
   - Every removed or deprecated path needs a migration-guide entry.

4. Finish module ownership
   - Backend-neutral contracts stay out of adapter modules.
   - Optional WGPU, winit, egui, AccessKit, glyphon, and clipboard integration
     stay behind feature-gated adapter/runtime boundaries.
   - Widget code should not live partly in `lib.rs` and partly in
     `src/widgets`.

5. Stabilize canvas ownership
   - V6 moved from shader-file demos toward app-owned WGPU canvas rendering.
   - V7 should document and harden the intended model: the app owns renderer
     state, records as many WGPU passes as it needs, and Operad owns canvas
     placement, input routing, accessibility metadata, and composition.

6. Keep release docs current
   - Replace v6-specific checklist language with v7 release gates when work
     starts.
   - Keep the README focused on the current library, not version archaeology.
   - Archive or clearly label old v3-v6 internal planning notes so they do not
     read like current product guidance.

## V7 Themes

### Alpha Progress

- Alpha 1 moved the retained document implementation out of `src/lib.rs`, added
  `operad::prelude`, and recorded the public path policy.
- Alpha 2 has started widget normalization by adding backend-neutral builders
  for text-style labels, standalone images, separators/spacers, spinners, radio
  buttons/groups, toggle switches, visual drag values, generic grids, and panel
  containers.
- Alpha 3 has started performance instrumentation with named frame pipeline
  stages and cache diagnostics for hit, miss, eviction, and retained-byte
  reporting.
- Alpha 4 has started overlay/widget parity by adding backend-neutral builders
  for collapsing headers, tooltip boxes, and modal dialog surfaces on top of the
  existing accessibility and overlay contracts.
- Alpha 5 has started text/widget parity by adding link, hyperlink, and
  selectable-label builders distinct from selectable read-only text.
- Alpha 6 has started text-editing convenience APIs by adding single-line,
  multiline, text-area, code-editor, search, and password-input builders backed
  by the existing text input state model.
- Alpha 7 has started generic drag/drop widget parity by adding reusable
  `dnd_drag_source` and `dnd_drop_zone` builders on top of the existing
  renderer-neutral drag/drop descriptors.
- Alpha 8 has started button convenience parity by adding small, icon, image,
  toggle, and reset button builders on top of the default button primitive.

### 1. Architecture And API Cleanup

- Move the remaining inline implementation out of `src/lib.rs`.
- Publish `operad::core`, `operad::interaction`, `operad::render`,
  `operad::runtime`, `operad::adapters`, `operad::accessibility`,
  `operad::widgets`, `operad::theme`, `operad::diagnostics`, and
  `operad::prelude` as the preferred public shape.
- Add a compatibility matrix that says which v5/v6 paths still work.
- Audit feature flags so backend-neutral builds do not pull native/rendering
  dependencies.
- Add public builder patterns where examples currently need low-level node
  construction.
- Add doc comments and examples on each public widget builder.

### 2. Widget Completeness

- Close the largest egui/iced parity gaps first: radio buttons, toggles, visual
  numeric drag values, images, separators, spinners, collapsing headers, panels,
  generic grids, tooltips, modal/dialog surfaces, and generic drag/drop
  surfaces.
- Turn partially implemented contracts into normal visual widgets when they are
  customer-facing, for example scrollbars, property inspectors, command
  palette, tree views, and context menus.
- Make every widget expose state/action helpers instead of asking examples to
  hand-roll editing, hit testing, hover, pressed, selection, or popup behavior.
- Add examples for all supported states: normal, hovered, pressed, toggled,
  disabled, focused, selected, error, loading, and reduced-motion.

### 3. Text, Editing, And Forms

- Harden single-line text input: caret placement, glyph metrics, selection,
  clipboard, undo/redo boundaries, keyboard navigation, IME, placeholders, and
  validation.
- Add multiline text editing and a code editor convenience widget.
- Add form primitives: field labels, help text, validation messages, field
  groups, dirty state, submit/cancel/apply actions, and keyboard traversal.
- Add numeric entry primitives: integer, decimal, stepper, drag value, unit
  suffixes, range editors, clamping modes, and transient invalid input.

### 4. Overlays, Panels, And Windows

- Centralize z-order so paint, hit testing, focus, popups, drag capture, and
  accessibility traversal agree.
- Make floating windows, popups, menus, tooltips, command palettes, modals, and
  toast overlays use the same layering rules.
- Add resize, collapse, close, drag, minimum-size, focus, and keyboard behavior
  as reusable surface primitives.
- Add panel primitives comparable to egui's central/side/top/bottom panels and
  iced's pane grid/float/overlay concepts.
- Ensure opening a combo box, submenu, tooltip, or popup does not cause layout
  shifts unless the caller explicitly requests inline layout.

### 5. Data-Heavy Application Surfaces

- Make tables, virtual lists, property inspectors, tree views, timelines, and
  scroll areas production-oriented.
- Add row selection, multi-selection, keyboard navigation, sorting, filtering,
  column resize, column reorder, sticky headers, virtualized row measurement,
  and clipboard/export hooks.
- Add chart primitives for the consumer gaps that still need egui fallbacks:
  sparklines, line charts, bar charts, histograms, range bands, and simple
  timelines.
- Add dense operational layouts with predictable spacing and no app-specific
  domain concepts.

### 6. Styling And Themes

- Promote the frame/styling controls into reusable theme/style APIs, not
  showcase-only code.
- Support per-side margins, per-corner radii, fill, stroke, shadow color,
  shadow blur/spread, and x/y shadow offset.
- Add state-specific tokens for hover, active, pressed, selected, focused,
  disabled, invalid, warning, success, and loading.
- Add high-contrast and reduced-motion theme paths.
- Add public text-style helpers: heading, strong, weak, small, monospace, code,
  colored label, and wrapped label.

### 7. Rendering, Canvas, And Media

- Keep the canvas API app-owned: apps should be able to build renderers, record
  multiple passes, and render into the Operad canvas target.
- Add offscreen capture and screenshot hooks outside the showcase.
- Add standalone image and SVG widgets with loading/error states and sizing
  policies.
- Add texture/resource lifecycle diagnostics for missing, stale, and failed
  assets.
- Add zoom/pan viewport helpers for canvas-heavy applications.
- Keep WGPU-specific types out of backend-neutral core APIs.

### 8. Performance And Instrumentation

- Add a frame timeline that reports tree rebuild, diffing, layout, text
  shaping, hit testing, paint-list generation, batching, uploads, and backend
  draw time.
- Track allocations and cache hit rates for layout, shaped text, images,
  canvas textures, and display-list reuse.
- Add stress probes outside the showcase: button grids, text-heavy forms,
  scroll-heavy tables, overlapping overlays, animated progress, and canvas
  scenes.
- Avoid layout recomputation for pointer hover unless the hover state changes a
  layout-affecting property.
- Add a performance budget for the showcase and for synthetic test scenes.

### 9. Accessibility And Input

- Add reusable accessibility semantics for every public widget.
- Add focus traps for modal/dialog surfaces.
- Add roving focus helpers for menus, tab lists, tree views, tables, and radio
  groups.
- Add keyboard shortcut, command, and menu semantics consistently.
- Expand pointer/touch/stylus support and make capture/cancel behavior explicit.
- Add screen-reader-friendly labels, value ranges, selected states, expanded
  states, disabled reasons, and live regions.

## Widget And Feature Backlog

This is intentionally larger than what must ship in `7.0.0`. The goal is to
make the missing surface area explicit so v7 work can be planned in batches.

### Text And Labels

- `heading`
- `colored_label`
- `strong`
- `weak`
- `small`
- `monospace`
- `code`
- `rich_text`
- wrapped label
- selectable label
- hyperlink
- link
- keyboard shortcut label
- badge/chip/tag text
- icon plus text label
- markdown viewer
- code editor

### Buttons And Binary Controls

- icon button
- small button
- split button
- menu button
- image button or image-in-button composition
- reset-to-default button
- radio button
- radio group
- toggle switch
- toggle button
- checkbox group
- tri-state checkbox
- segmented control
- disclosure button

### Numeric And Slider Controls

- visual drag value
- integer input
- decimal input
- unit input
- stepper
- angle drag control
- two-dimensional drag pad
- range slider
- dual-thumb range slider
- vertical slider
- logarithmic slider builder
- stepped slider builder
- editable slider value
- smart aim configuration
- clamping configuration
- slider with trailing fill
- slider with custom thumb shape

### Selection Controls

- searchable combo box
- pick list
- list box
- multi-select list
- autocomplete field
- command-backed picker
- grouped select menu
- disabled-option reasons
- select menu with icons
- select menu with keyboard typeahead
- popover picker

### Text Input And Forms

- single-line text input
- multiline text input
- password input
- search input
- validated input
- masked input
- text area
- field label
- field help text
- field error text
- form row
- form section
- property editor row
- apply/cancel form footer
- dirty-state indicator
- validation summary

### Color And Style Editing

- compact color button
- color swatch button
- RGB editor
- RGBA editor
- SRGB editor
- SRGBA editor
- HSVA editor
- OKLCH editor
- premultiplied alpha editor
- unmultiplied alpha editor
- two-dimensional color picker
- palette editor
- color history
- color format copy buttons
- eyedropper request contract
- frame editor
- shadow editor
- stroke editor
- fill editor

### Containers, Layout, And Surfaces

- frame
- group
- separator/rule
- spacer/space
- scroll bar
- scroll area
- collapsing header
- accordion
- tooltip
- popup
- popover
- modal
- dialog
- area
- floating window
- resize handle
- resize container
- central panel
- side panel
- top panel
- bottom panel
- pane grid
- dock workspace
- split pane
- stack
- overlay layer
- responsive grid
- columns
- indented section
- scene or zoomable area

### Menus, Toolbars, And Commands

- menu bar
- menu button
- submenu
- submenu button
- context menu
- command palette
- toolbar
- status bar
- breadcrumb
- navigation rail
- command search result
- checkable menu item
- radio menu item
- menu item with shortcut
- disabled menu item with reason

### Data And Navigation

- table
- data grid
- virtual list
- tree view
- tree table
- outline view
- property inspector
- key-value inspector
- sortable header
- filter row
- pagination controls
- row details
- cell editor
- row reorder handle
- drag/drop list
- timeline
- ruler
- breadcrumb
- tabs
- dock tabs

### Feedback And Status

- progress bar
- progress ring
- spinner
- loading skeleton
- toast
- alert
- banner
- inline warning
- empty state
- retry panel
- validation summary
- live status region
- notification center

### Media, Canvas, And Domain-Neutral Drawing

- image
- SVG
- animated image policy
- QR code
- canvas
- WGPU canvas target
- app-owned render graph hook
- zoom/pan canvas viewport
- ruler overlay
- mini-map
- sparkline
- line chart
- bar chart
- histogram
- heat map
- curve editor
- node graph shell

### Interaction Helpers

- drag source
- drop zone
- drag handle
- reorderable list
- hover region
- mouse area
- opaque input region
- focus scope
- roving focus group
- keyboard shortcut scope
- visible wrapper
- enabled wrapper
- sized widget helper
- exact allocation helper
- manual placement helper
- scroll-to-rect helper
- scroll-to-cursor helper
- animated scroll helper

## Milestones

### 7.0 Alpha 1: Structure

- Split `src/lib.rs` below 1,000 lines. Current structure pass: done.
- Add `operad::prelude`. Current structure pass: done.
- Add compatibility/deprecation policy and migration guide. Current structure
  pass: started in `docs/v7_0_migration_guide.md`.
- Carry the published `6.1.0` WGPU 29/Glyphon 0.11 baseline forward so v7 does
  not regress released GPU compatibility.
- Move tests next to the modules they exercise.
- Keep all v6 release checks green after the split.

### 7.0 Alpha 2: Widget Normalization

- Promote remaining showcase-only behavior into widget primitives.
- Add radio, toggle, drag value, image, separator, spinner, collapsing header,
  tooltip, modal/dialog, and generic grid builders.
- Add text-style helpers and form primitives.
- Add widget doc examples for common states.

### 7.0 Alpha 3: Surfaces And Overlays

- Centralize z-order and hit-test ordering.
- Stabilize floating windows, popups, menus, tooltips, toasts, command
  palette, and modal layers.
- Add panel primitives and pane-grid style layout.
- Add overlay visual regression coverage.

### 7.0 Beta 1: Data And Forms

- Upgrade tables, virtual lists, property inspectors, tree views, and timelines.
- Add row/cell selection, sorting/filtering, column resize, and keyboard
  navigation.
- Add multiline text editing, numeric input, validation, and form helpers.
- Add chart/sparkline primitives needed by consumer migration notes.

### 7.0 Beta 2: Performance And Rendering

- Add frame instrumentation and cache diagnostics.
- Add stress probes outside examples.
- Harden WGPU canvas rendering, offscreen capture, image/SVG loading, and
  resource lifecycle reporting.
- Define release performance budgets.

### 7.0 Release Candidate

- Rewrite README and migration docs for the v7 public API.
- Keep the showcase as a clean learning example with one window per widget
  family and no hidden test harness behavior.
- Run the visual inspection matrix across default, interaction, min-size,
  max-size, overlay, and high-DPI states.
- Run package, docs, no-default, all-features, examples, and WGPU checks.

## Release Gates

V7 should not ship until these are true:

- `src/lib.rs` is below 1,000 lines and mostly declarative.
- `operad::prelude` exists and is documented.
- Every intentionally changed public path is documented in the migration guide.
- The README describes Operad as the current library, not as "v7 of v6".
- Feature flags are documented and no-default builds stay backend-neutral.
- The showcase contains only public API usage intended for customers to learn
  from.
- Widget visual inspection covers normal, hover, pressed, toggled, disabled,
  focused, selected, min-size, max-size, popup-open, and overlap states where
  applicable.
- Paint ordering, hit testing, focus, and accessibility traversal use the same
  effective ordering model.
- Text input supports caret placement, selection, clipboard, keyboard
  navigation, placeholder sizing, and deletion without renderer artifacts.
- Scrollbars can be clicked, dragged, and kept visually aligned with the
  content they control.
- Performance instrumentation can explain a laggy frame.
- These commands pass:

```bash
cargo fmt --all -- --check
cargo check --locked --no-default-features --all-targets
cargo test --locked --no-default-features --lib
cargo check --locked --all-features --all-targets
cargo test --locked --all-features -- --list
cargo check --locked --all-features --examples
cargo doc --locked --all-features --no-deps
cargo package --locked
```

## Non-Goals

- Do not turn Operad into a product-specific UI framework for any one app.
- Do not put tests or stress harnesses into customer-facing examples.
- Do not split into multiple crates until the internal module boundaries are
  stable inside one crate.
- Do not make WGPU, winit, egui, AccessKit, glyphon, or clipboard dependencies
  mandatory for backend-neutral consumers.
- Do not promise every backlog widget for `7.0.0`; treat the backlog as v7.x
  planning input unless a milestone explicitly adopts it.
