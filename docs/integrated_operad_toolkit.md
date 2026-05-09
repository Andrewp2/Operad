# Integrated Operad Toolkit Direction

These notes synthesize the game, Orbifold, and Fabricad/layout agent notes into
the crate shape implemented here. The common requirement is a deterministic,
retained UI document that can be inspected, tested, hit-tested, scrolled, and
lowered into more than one renderer.

## Shared Core

The default `operad` crate should stay renderer-independent and dependency-light.
It owns:

- Stable node IDs, a retained document tree, and computed layout rectangles.
- Neutral geometry, colors, text styles, visuals, clipping, opacity, z order,
  scroll state, focus state, hit testing, input routing, and animation state.
- A backend-neutral paint list so egui, wgpu, CPU snapshot renderers, and tests
  consume the same display boundary.
- Layout/audit snapshots for debugging panel bounds, clipping, focusability, and
  unreachable interactive controls.

The default feature set intentionally avoids `egui`, `glyphon`, `wgpu`, and
`cosmic-text`. With `--no-default-features`, the normal dependency tree is
currently just `taffy` plus its small layout dependencies.

## Optional Layers

- `text-cosmic`: provides `CosmicTextMeasurer` for real text shaping and
  measurement while keeping public text styles backend-neutral.
- `egui`: paints the neutral `PaintList` through egui for incremental migration
  inside existing apps.
- `widgets`: starts a domain-neutral widget layer with reusable button, label,
  and scroll-area builders.
- `audit`: reserved for stronger snapshot/export checks as the testing surface
  grows.

## Consumer Boundaries

Operad should not know about game actions, musical clips, synth state, MIDI
devices, wafer maps, semiconductor process models, or application undo stacks.
Consumers build or update a UI document from stable view-model snapshots, feed
synthetic or platform input into the document, inspect emitted neutral
interaction results, and render the generated paint list through their chosen
backend.

Custom editors such as game HUD overlays, Orbifold piano rolls/Lumatone maps,
and Fabricad layout viewports should use Operad for chrome, panels, clipping,
input arbitration, and stable rect inspection while keeping domain drawing and
commands in the application crate.

## Near-Term Gaps

The next toolkit milestones are text input, richer keyboard shortcut scopes,
command/menu models, virtual lists/tables, scrollbar thumb interaction,
splitters/docking, and stronger audit export. Those should build on the same
document, input, scroll, and paint-list boundaries rather than adding
app-specific behavior to core.
