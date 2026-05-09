# Operad 1.0 Internal Release Readiness

Operad `1.0.0` is intended as an internal shared baseline for the game,
Fabricad/layout tools, and Orbifold. It is not a public internet release. The
goal is to give those consumers enough stable surface area to begin building on
Operad while leaving product-specific commands and domain editors in their own
crates.

## Review Criticism Response

- **Tracked integration state:** the library, docs, examples, manifest, and
  lockfile are now ordinary tracked workspace files. The crate has a library
  target, an example probe, and Cargo metadata for `1.0.0`.
- **Mutation and invalidation:** `UiDocument::nodes` is private. Consumers can
  inspect through `node`, `nodes`, and `node_count`; mutation goes through
  setters, `edit_node`, or `node_mut`, all of which either preserve paint-only
  behavior or invalidate layout conservatively.
- **Hit testing versus paint order:** pointer hit testing now uses the same
  effective z-order traversal as `PaintList`, including inherited parent
  z-order.
- **Animation semantics:** animation is paint-time state. Layout rectangles stay
  stable, while paint items carry effective opacity, translation, and scale from
  the current animation machine values.
- **Scroll routing:** wheel input finds scroll containers geometrically rather
  than requiring a pointer-enabled child under the cursor. Nested scroll areas
  get first chance, then wheel deltas propagate to outer scroll regions when an
  inner region cannot move.
- **Scroll content bounds:** scroll content size is computed from descendants,
  not just direct children, so nested panels and composed widgets can determine
  the scroll extent.
- **Canvas boundary:** core emits `PaintKind::Canvas` with stable key, rect,
  clip, z, opacity, and transform. The egui adapter exposes
  `paint_document_egui_with_canvas` so applications can register backend-local
  drawing for those keys.
- **Paint-list maturity:** `PaintList` now includes rectangles, text, canvas
  placeholders, lines, circles, polygons, and image handles without exposing
  backend-specific types.
- **Text fidelity:** public text styles remain Operad-owned. The egui adapter
  now at least preserves monospace versus proportional family choice, while
  real measurement stays behind `text-cosmic`.
- **Text input:** the widget layer includes a first-class `TextInputState`,
  edit phases, caret movement, selection, insert/delete/backspace, multiline
  enter behavior, commit, and cancel.
- **Virtual lists and tables:** the widget layer includes `VirtualListSpec`,
  `virtual_list`, `TableColumn`, and `table_header` so large data views can
  render visible rows with spacer-backed scroll extents.
- **Audit strength:** `audit_layout` now checks non-finite rects, invisible
  interactive nodes, empty clips, too-small hit targets, duplicate node names,
  clipped text, nodes outside the root without scroll/canvas intent, and paint
  items with empty clips.
- **Layout API ergonomics:** the `layout` helper module gives consumers common
  row, column, fixed, fill, percent, margin, and padding helpers so product code
  does not need raw Taffy construction everywhere.

## 1.0 Consumer Surface

Core:

- `UiDocument`, `UiNode`, stable `UiNodeId`, geometry primitives, colors,
  strokes, visuals, clipping, input behavior, focus, scroll, animation, layout
  snapshots, and audit warnings.
- Neutral text types plus `ApproxTextMeasurer` by default.
- Optional `CosmicTextMeasurer` behind `text-cosmic`.
- `PaintList` with shape/editor primitives and per-item transform/opacity.
- Canvas escape hatches for app-owned renderers.

Widgets:

- Button, label, checkbox, text input, slider, combo box, scroll area,
  scrollbar thumb geometry, virtual list, and table header helpers.
- Widgets build normal Operad nodes and expose neutral state/action helpers.
  They do not own game, DAW, Fabricad, audio, MIDI, semiconductor, or undo
  semantics.

Examples and tests:

- `examples/three_consumer_probe.rs` exercises a game HUD, Fabricad-style
  scroll panel, and Orbifold editor shell without opening a window.
- Unit tests cover layout, text measurement, cache invalidation, clipping,
  z-order hit testing, scroll behavior, descendant scroll bounds, paint-list
  primitives, focus, animation paint output, text input, and virtual lists.

## Known Post-1.0 Work

- Shortcut scopes and command/menu registries.
- Text selection/caret painting, IME platform integration, and clipboard
  service adapters.
- Full table body helpers with sorting, filtering, column resizing, sticky
  headers, and keyboard navigation.
- Splitter/docking state helpers.
- Stronger serialized audit snapshots and optional screenshot backends.
- Native GPU renderer work that consumes the same `PaintList` boundary.
