# Game Agent Review Of Current Operad Changes

This note captures the main criticism from a read-only review of the current
Operad changes. It intentionally focuses on Operad's own implementation and
integration shape, not on which feature set any specific consumer should enable.

## Summary

The architectural direction is good. The current changes move Operad toward the
right abstraction boundary: a renderer-neutral retained UI document with
optional backends, optional text measurement, scroll state, canvas nodes,
paint-list output, and audit/debug hooks.

The main concern is that the implementation is still in a partial-integration
state. Several pieces are promising, but they need stronger consistency before
this should be treated as a stable shared library for the game, Orbifold, and
semiconductor tools.

## Integration State

The implementation files need to be intentionally staged/tracked before the
change is reviewed as a complete unit. During review, the most important code
was in untracked paths such as `src/lib.rs`, `docs/`, and `examples/`, while the
tracked diff mostly showed the manifest, lockfile, and deleted old binary entry
point.

That is risky because a normal commit, branch handoff, or diff review could
include the manifest changes while accidentally omitting the actual library
implementation, docs, and example probe.

## Public Mutation And Layout Invalidation

The new layout cache is a useful direction, but the invalidation model is not
yet strict enough.

`compute_layout` can skip work when the viewport and layout revision are
unchanged. That is good for performance, but `UiDocument::node_mut` exposes
direct mutable access to nodes without invalidating layout. Any caller can
change layout style, text content, scroll behavior, input flags, or other
layout-affecting fields and then get stale computed rectangles from the cache.

This needs a clearer mutation boundary. Possible fixes:

- Replace broad `node_mut` use with mutation APIs that know whether to
  invalidate layout, paint, or input state.
- Keep `node_mut`, but document it as low-level/unsafe-for-cache and require an
  explicit `invalidate_layout` after layout-affecting mutation.
- Split mutable access into narrow setters: visual-only, text, style, children,
  input behavior, scroll state.
- Track separate dirty bits for structure, layout, text measurement, paint, and
  input/hit-test data.

The key rule should be: if a public API can make computed layout stale, that API
must either invalidate automatically or make the invalidation requirement hard
to miss.

## Animation Semantics Are Incomplete

`AnimatedValues` includes opacity, translation, and scale, but only opacity is
consulted during layout application. Translation and scale are currently inert.
Even opacity can be stale because `tick_animations` updates the animation
machine without invalidating or recomputing the derived `ComputedLayout`
opacity.

This creates two issues:

- Animation state can advance without the paint output reflecting the new value.
- The type advertises transform-like behavior that the renderer does not apply.

Operad should decide whether animation values are layout-affecting,
paint-affecting, or both. A good near-term boundary would be to keep layout
rectangles stable and apply animation transforms/opacity while generating the
paint list. That would avoid forcing layout recomputation for simple hover,
focus, and open/close transitions.

## Hit Testing And Paint Ordering Should Match

`paint_list` uses effective inherited z-indexes, but `hit_test` compares only
each node's local `style.z_index`. That can create cases where an element is
painted visually on top but loses hit testing to another element that happens to
have a higher local z-index.

The hit-test path should use the same effective z-order model as paint output.
Ideally this ordering should be centralized so painting, hit testing, focus
navigation, and audit tooling all agree about the visual stack.

## Scroll Model Needs More Work Before It Is A Toolkit Feature

The scroll state addition is valuable, but it is still early. It tracks viewport
and content size, offsets child origins, and routes wheel events to scrollable
ancestors. That is a good base.

Before relying on it broadly, it needs more complete behavior:

- Clear wheel delta convention and platform adapter expectations.
- Nested scroll propagation when an inner scroll area is already at its limit.
- Scrollbar thumb geometry and dragging.
- Programmatic scroll-to-node behavior that is tested with existing offsets.
- Audit coverage for clipped but scroll-reachable interactive content.
- A paint/input story for scrollbars that does not require every app to invent
  its own version.

Scroll containers are one of the biggest reasons to have Operad, so this area
should become rigorous rather than just minimally functional.

## Paint List Is The Right Boundary, But It Needs To Mature

The renderer-neutral `PaintList` is the right abstraction. It lets egui, wgpu,
CPU snapshots, tests, and future tooling consume the same UI output.

The current paint list is still very small: rectangles, text, and canvas
placeholders. That is enough to prove the boundary, but practical consumers will
need additional primitives and metadata:

- Images or texture handles.
- Lines, paths, polygons, and circles for custom editors.
- Text alignment, elision, baseline info, and font role information.
- Transform or local coordinate information for animated/canvas content.
- Batching hints or stable item IDs for retained renderer caches.
- Debug metadata so audit views can map paint items back to layout nodes.

The important part is to keep this boundary renderer-neutral. Backend-specific
types should not leak into `PaintItem`.

## Text Abstraction Direction Is Good

Moving public text style away from renderer-specific font types is a good
abstraction improvement. The core should own neutral text concepts such as
family, weight, style, stretch, wrap, color, size, and line height.

The approximate measurer is useful for headless tests. The real text measurer
being optional is also the right shape.

The next step is to make text measurement cache behavior and invalidation more
explicit. Text-heavy UIs need predictable cache keys, bounded memory growth, and
tests for common constraints such as fixed width, auto height, wrapping, and
changed font style.

## Widget Layer Should Stay Thin And Domain-Neutral

The optional `widgets` feature currently starts with button, label, and
scroll-area builders. That is fine as a seed, but the widget layer should stay
thin and should not become an application framework.

Reusable widgets should:

- Build ordinary Operad nodes.
- Emit neutral interaction results or command IDs.
- Avoid owning application state.
- Avoid knowing about game, DAW, or semiconductor domain types.
- Be tested as plain document construction and input behavior.

This keeps product-specific UI in the product crates while allowing shared
control behavior to improve once.

## What I Would Prioritize Next

1. Make the integration state clean: tracked implementation, docs, examples,
   and lockfile all intentionally included.
2. Define the dirty/invalidation model before adding more widgets.
3. Make hit testing use the same effective z-order as painting.
4. Decide how animation affects paint and layout, then make `AnimatedValues`
   fully real or reduce it to the values currently supported.
5. Strengthen scroll tests around nested scroll areas, scroll limits, and
   scroll-to-node.
6. Add a small renderer-neutral snapshot test path around `PaintList`.

The library is heading in the right direction, but these consistency issues
matter because Operad is intended to be shared infrastructure rather than a
single app's private UI experiment.
