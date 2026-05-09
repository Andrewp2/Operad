# Review: Integrated Consumer Feedback Changes

This is a review of the current Operad worktree after the integrated feedback pass
from the game, layout/Fabricad, and Orbifold consumers. It is intentionally a
critique document, not a patch plan. The code review was read-only except for
adding this note.

## High-Level Take

The direction is good. The work moves Operad away from a throwaway binary and
toward a reusable library with a retained document tree, Taffy layout, neutral
geometry, backend-neutral paint items, optional text/egui/widget features, and a
small three-consumer probe. That is the right foundation for the consumers I care
about: game HUDs, dense layout tooling, and Orbifold's musical editors.

I would still treat this as a promising first spike rather than a stable toolkit
core. The main risks are not in the concept; they are in places where the current
implementation can silently disagree with itself: mutation versus layout caching,
animation state versus rendered output, hit testing versus paint order, and text
measurement versus text rendering.

## What Looks Strong

- The crate is now a library with optional backend features instead of a binary.
- The default feature set is light: practically just `taffy` and its small layout
  dependencies.
- The retained `UiDocument` model gives consumers something inspectable,
  testable, and serializable enough for layout snapshots and debugging.
- The neutral `PaintList` is a good boundary. It lets egui, future GPU renderers,
  CPU snapshot tests, and custom editor surfaces consume the same display model.
- The `examples/three_consumer_probe.rs` file is useful because it checks that
  the same primitives can represent a game HUD, a Fabricad-style panel, and an
  Orbifold editor shell.
- The tests cover important early surfaces: layout, text measurement, clipping,
  scrolling, paint list generation, focus, and animation state machines.

## Main Concerns

### 1. Mutation And Layout Cache Invalidation Are Too Easy To Misuse

`UiDocument` exposes both `pub nodes: Vec<UiNode>` and `node_mut`. Layout caching
is invalidated by explicit methods such as `add_child` and `set_scroll_offset`,
but direct mutation through `nodes` or `node_mut` can change layout-affecting
state without bumping the revision. After that, `compute_layout` can return early
from the old cache.

Relevant code:

- `src/lib.rs:767` defines `UiDocument`.
- `src/lib.rs:769` exposes `pub nodes`.
- `src/lib.rs:800` exposes `node_mut`.
- `src/lib.rs:865` defines explicit invalidation.
- `src/lib.rs:880` returns early when the cached layout key matches.

This is the largest correctness risk. A retained UI toolkit has to make stale
layout hard to produce. If consumers start building real panels on top of this,
this will become a source of confusing bugs.

Possible fixes:

- Make `nodes` private.
- Replace broad mutable access with methods that know whether they invalidate
  layout, paint, input, or only metadata.
- If broad mutation remains necessary, expose a mutation closure like
  `edit_node(id, |node| ...)` that invalidates conservatively.
- Split dirty state into layout-dirty, paint-dirty, and input-dirty once the
  toolkit needs more performance.

### 2. Animation State Is Not Connected To Rendered Output

The animation machine itself works, but document-level animation currently does
not reliably affect what gets painted.

`tick_animations` advances animation state, but it does not invalidate layout.
Opacity is copied into `ComputedLayout` during layout, so a later animation tick
can leave paint using stale opacity. The `translate` and `scale` fields exist in
`AnimatedValues`, but they are not applied to layout, paint items, or hit testing.

Relevant code:

- `src/lib.rs:985` reads animation opacity during layout.
- `src/lib.rs:1137` ticks animations.
- `src/lib.rs:1467` defines animated opacity/translate/scale.
- `src/lib.rs:1145` builds paint items from cached computed layout.

This should be resolved before animation becomes part of the public story.
Operad should decide whether animation transforms are paint-only, layout-affecting,
or both. For most UI work, I would start with paint-only transforms and make the
paint list carry effective opacity and transform directly from current animation
values. Hit testing then needs an explicit policy: either use untransformed layout
rects or inverse-transform pointer coordinates.

### 3. Hit Testing And Paint Order Can Disagree

Painting uses `effective_z_indexes`, where children can inherit a parent's
z-order. Hit testing uses `node.style.z_index` directly. That means visual stacking
and click targeting can diverge.

Relevant code:

- `src/lib.rs:1027` implements hit testing.
- `src/lib.rs:1037` compares raw `node.style.z_index`.
- `src/lib.rs:1147` uses effective z-indexes for paint order.
- `src/lib.rs:1202` computes effective z-indexes.

Click targeting should use the same ordering model as paint. For a UI toolkit,
"the thing visually on top is the thing that receives the click" is the default
expectation unless there is an explicit input override.

### 4. Wheel Scrolling Requires A Pointer-Enabled Hit Target

Wheel routing starts with `hit_test(position)`. But `hit_test` ignores nodes that
do not have pointer input enabled. As a result, a scroll area with noninteractive
text or passive content may not scroll when the cursor is over that content.

Relevant code:

- `src/lib.rs:1027` implements hit testing.
- `src/lib.rs:1031` filters out non-pointer nodes.
- `src/lib.rs:1066` handles wheel events by calling `hit_test`.
- `src/lib.rs:1091` walks to a scrollable ancestor.

Wheel routing probably needs a separate geometry query that finds the deepest
visible node under the cursor regardless of whether it is pointer-interactive,
then walks upward to a scroll container.

### 5. Text Measurement And Egui Rendering Do Not Match

The data model has font family, weight, style, stretch, line height, and wrap.
`CosmicTextMeasurer` attempts to measure those. The egui renderer, however,
always renders text with `egui::FontId::proportional(text.style.font_size)` and
does not apply family, weight, style, stretch, line height, or wrapping.

Relevant code:

- `src/lib.rs:240` defines `TextStyle`.
- `src/lib.rs:591` implements `CosmicTextMeasurer`.
- `src/lib.rs:1738` renders egui text.
- `src/lib.rs:1744` always uses proportional egui text.

For Orbifold, this matters quickly. Dense musical editors need monospace labels,
small numeric data, and predictable measured text. If measured layout and rendered
text disagree, snapshot tests and real UI will drift.

### 6. Scroll Semantics Are Still Minimal

The scroll model is a good start, but it is not yet enough for production UI:

- No scrollbar thumb interaction.
- No trackpad phase or high-resolution scroll policy.
- No nested scroll handoff policy.
- No explicit coordinate conversion helpers for content versus viewport space.
- `scroll_to_node` depends on current layout and may be surprising if called
  before layout has been computed.

This is acceptable for the current spike, but it should be called out as an early
area to harden.

### 7. Widget Layer Is Only A Seed

The optional `widgets` feature currently provides basic builders for buttons,
labels, and scroll areas. That is fine as a first slice, but consumers should not
assume the widget system has the command model, text input, shortcut scopes,
menus, numeric controls, tables, docking, or validation behavior they will need.

For Orbifold specifically, numeric controls and command phases are important:
continuous preview while dragging, one undoable edit on commit, and clear
cancellation behavior.

## Verification Results

These commands passed:

```text
cargo test
cargo test --all-features
cargo run --example three_consumer_probe
cargo check --examples --all-features
cargo fmt --check
```

This command failed:

```text
cargo clippy --all-features --all-targets -- -D warnings
```

The failure was a test-only style issue: field reassignment after
`TextStyle::default()` in `src/lib.rs:1924`. That is not a product bug, but it is
worth fixing if CI will run Clippy with warnings denied.

## Suggested Next Moves

1. Lock down mutation and invalidation before more API grows around `UiDocument`.
   This is the first thing I would fix.
2. Make hit testing use the same effective ordering as paint.
3. Split "node under cursor" from "pointer-interactive hit target" so scrolling
   works over passive content.
4. Decide animation semantics and make paint output reflect current animation
   values without requiring accidental layout recomputation.
5. Improve egui text rendering or document clearly that the egui backend is only
   a migration/debug renderer with approximate text fidelity.
6. Add tests for the failure modes above: stale layout after mutation, inherited
   z-index hit testing, scroll over passive text, and rendered animation opacity.

## Bottom Line

The integrated changes point Operad in the right direction. The architecture has
the right boundaries for the three consumers: retained inspectable layout,
backend-neutral paint, optional render/text layers, and consumer-owned domain
commands.

The critique is that a UI toolkit succeeds or fails on boring consistency. The
current version has a few places where internal models can disagree. Fixing those
early will make the rest of the toolkit much easier to trust.
