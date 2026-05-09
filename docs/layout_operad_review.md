# Layout Agent Review of Current Operad Changes

This note captures the main concerns I saw after reviewing the current Operad
changes that integrated feedback from the game, Orbifold, and Fabricad/layout
consumers.

## Overall Assessment

The direction is substantially better than the first extraction. Operad now looks
more like a shared UI foundation and less like copied game UI code.

The strongest changes are:

- The default feature set is empty.
- `glyphon` and direct `wgpu` dependencies are gone from core.
- `cosmic-text` is optional behind `text-cosmic`.
- Public text style types are Operad-owned instead of backend-owned.
- A backend-neutral `PaintList` exists.
- Scroll state, wheel events, layout snapshots, audit warnings, canvas nodes,
  and an initial widget module have been added.

Those changes are aligned with the right long-term boundary: core document,
layout, input, and paint-list behavior in Operad; renderers, text engines,
platforms, and product state outside the core.

## Critical Consumer Break

The game integration currently looks mismatched.

In the game repo, the path dependency enables only:

```toml
operad = { path = "../operad", features = ["egui"] }
```

But game code still imports `CosmicTextMeasurer` through `src/ui/player/mod.rs`.
Since `CosmicTextMeasurer` is now gated behind `text-cosmic`, the game needs one
of these changes:

```toml
operad = { path = "../operad", features = ["egui", "text-cosmic"] }
```

or the game needs to switch those call sites to `ApproxTextMeasurer`.

This should be fixed before treating the extraction as integrated.

## Scroll Routing Is Too Narrow

Wheel input currently finds a scroll target by hit-testing the pointer position
and walking up to a scrollable ancestor.

That is a good start, but `hit_test` only returns nodes with `InputBehavior`
where `pointer == true`. This means blank space inside a scroll area may fail to
scroll unless some pointer-enabled child is under the cursor.

For Fabricad-style panels, scroll regions need to scroll when the pointer is
inside the scroll viewport, even over empty rows, padding, or whitespace.

Recommended fix:

- Add a separate hit-test mode for scroll containers.
- Or make wheel routing search visible scroll nodes whose `clip_rect` contains
  the pointer, choosing the topmost/deepest scroll region.
- Keep pointer click hit testing separate from wheel scroll hit testing.

## Scroll Content Measurement Is Shallow

Scroll content sizing currently appears to derive content bounds from direct
children after layout.

That is enough for simple row lists, but not enough for nested panels, tables,
wrapped content, or composed widgets. Fabricad and Orbifold will both create
nested layouts where the scrollable content bounds are determined by descendants,
not just immediate children.

Recommended fix:

- Compute scroll content bounds from descendant layout bounds in the scroll
  node's local content space.
- Include only descendants that participate in layout and visibility.
- Be explicit about whether absolutely positioned descendants count.

## Canvas Content Needs A Callback Boundary

`CanvasContent` is a useful addition because it gives Fabricad layout/3D and
Orbifold editor surfaces a first-class escape hatch.

However, the egui adapter currently emits no drawing for `PaintKind::Canvas`.
That is understandable for core, but the abstraction needs a follow-up boundary
soon.

Recommended shape:

- Core emits `PaintKind::Canvas(CanvasContent)` with stable key, rect, clip,
  opacity, and z order.
- Renderer adapters accept a user-provided callback or registry for canvas keys.
- The callback receives rect, clip, scale factor, input/focus state if needed,
  and a backend-specific drawing context.

Without that, canvas nodes are only layout placeholders.

## Taffy Types Leak Through Core API

The public API still takes and stores `taffy::Style` directly in `UiNodeStyle`
and widget options.

This is acceptable for the current spike, but it couples every consumer to
Taffy's exact style model. If we later want Operad-owned layout types, every
consumer view may need to be rewritten.

Recommended options:

1. Keep exposing Taffy for the near term, but document that it is provisional.
2. Add Operad-owned layout builder helpers around common cases.
3. Eventually migrate public APIs to Operad layout types and translate to Taffy
   internally.

The main risk is not Taffy itself. The risk is that product code starts depending
on Taffy details everywhere.

## Text Input Is Still Missing

The text style and measurement boundary is improved, but text editing remains a
major missing primitive.

Fabricad needs this for command palette search, filters, layer names, markdown
editing, property fields, and numeric editors. Orbifold will need it for project
metadata, numeric input, search, and device/config panels.

Needed capabilities:

- Single-line text input
- Multiline text input
- Caret and selection
- Keyboard movement
- Delete/backspace
- Clipboard hooks
- IME composition hooks
- Commit/cancel behavior
- Focus integration

This should be a core interaction model or a first-class widget layer feature,
not app-specific code.

## Virtual Lists And Tables Are Not Present Yet

The new scroll primitives help, but they do not solve large data views.

Fabricad has tables and issue lists where rendering every row is the wrong
default. The toolkit needs an opinionated path for:

- Virtual rows
- Sticky headers
- Column sizing
- Row selection
- Keyboard navigation
- Sort/filter state hooks
- "Showing N-M of K" status patterns

This is one of the areas where Operad can become more useful than egui for our
apps, but it is not there yet.

## Audit System Is A Good Start But Too Weak

`layout_snapshot` and `audit_layout` are the right primitives to add early.

The current warnings are basic: non-finite rects, invisible interactive nodes,
and empty interactive clips. Useful, but not enough for the failures we have
actually seen in Fabricad.

Recommended next audit checks:

- Text overflow beyond node rect or clip rect.
- Interactive node clipped below a minimum usable size.
- Scrollable content without a reachable scroll mechanism.
- Focusable nodes missing from traversal.
- Nodes extending outside root without scroll/canvas/fullscreen intent.
- Paint items with empty clips.
- Duplicate or unstable semantic names where names are used for snapshots.

The goal should be a headless audit harness that can build each product view at
multiple viewport sizes and fail on real layout regressions.

## Widget Layer Is Only A Seed

The `widgets` feature is useful as a marker, but it currently has only basic
builders. That is fine at this stage.

The important rule is that widgets should emit neutral events or consumer-defined
action IDs. They should not grow game, DAW, or Fabricad semantics.

Near-term useful widgets:

- Button
- Label
- Checkbox
- Text input
- Combo box
- Slider / drag value
- Scroll area with scrollbar
- Splitter
- Command palette list
- Virtual list

## Feature Boundaries Look Better

The feature split is much healthier now:

```toml
default = []
egui = ["dep:egui"]
text-cosmic = ["dep:cosmic-text"]
widgets = []
audit = []
```

This is the right basic shape. The main thing to preserve is that default
Operad stays free of renderer and text-renderer dependencies.

I verified that the default dependency tree is just `taffy` plus its small
layout dependencies. With `text-cosmic`, `cosmic-text` adds expected font shaping
dependencies, including indirect `bytemuck`, but no direct `wgpu`.

## Verification Performed

I verified the current crate read-only with an external target directory:

```bash
cargo check --target-dir /tmp/codex-operad-check
cargo check --target-dir /tmp/codex-operad-check --features text-cosmic,egui,widgets,audit
cargo test --target-dir /tmp/codex-operad-check
cargo test --target-dir /tmp/codex-operad-check --features text-cosmic,egui,widgets,audit
cargo check --target-dir /tmp/codex-operad-check --examples
cargo check --target-dir /tmp/codex-operad-check --examples --features widgets
cargo fmt --check
```

All passed.

## Recommended Fix Order

1. Fix the game feature mismatch by enabling `text-cosmic` or changing the game
   to use `ApproxTextMeasurer`.
2. Improve wheel routing so blank space inside scroll regions scrolls.
3. Make scroll content bounds descendant-aware.
4. Define the canvas callback/registry boundary for renderer adapters.
5. Add text input as a real primitive.
6. Add a virtual list/table primitive before porting any serious Fabricad table.
7. Strengthen audit checks against the actual Fabricad layout failures.
8. Add Operad-owned layout helper APIs so app code does not become saturated
   with raw Taffy style construction.

## Bottom Line

The current Operad changes are directionally right and materially better than
the first extraction. The crate now has the beginning of the correct core:
document, layout, input, scroll, paint list, and audit surfaces.

The next risk is mistaking these seeds for finished systems. Scroll, text input,
canvas integration, virtual lists, and audit tooling need to become real before
Operad can safely carry Fabricad or Orbifold UI at scale.
