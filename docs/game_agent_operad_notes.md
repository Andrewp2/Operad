# Codex Notes On Operad

Operad should be the reusable UI substrate, not a place where any single
application's UI leaks in. The game, Orbifold, and semiconductor tools will all
need different visual language and different domain widgets, but they share a
need for predictable layout, input routing, focus, animation, clipping,
scrolling, text measurement, and efficient rendering.

From a coding-agent point of view, the most useful version of Operad is a small
library with stable boundaries and strong tests. I should be able to improve a
control, fix input behavior, or optimize rendering once in Operad and then
reuse that work across projects without rediscovering each application's UI
rules.

## What I Need From Operad

I need the core to be deterministic and easy to exercise headlessly. Most UI
bugs I can fix fastest when I can build a document, feed it synthetic events,
compute layout at a known viewport size, and assert the resulting geometry,
focus, hit target, scroll offset, or emitted command without opening a real app.

The library should keep examples and benchmark probes close to the code. A
100x100 button grid, a scrollable panel with dropdowns, a text-heavy form, and a
mixed overlay scene would tell me quickly whether a change improved or damaged
behavior. Those probes should be renderer-aware where needed, but the core
layout and input tests should not require a window or GPU.

Operad should expose enough instrumentation to explain performance. I want to
know whether a frame spent time in tree rebuilds, layout, text shaping, hit
testing, paint-list generation, batching, or backend upload. If a UI becomes
laggy, "the UI is slow" is not enough information; the library should make the
hot path visible.

## Right Abstraction Boundaries

The core crate should own:

- Geometry primitives, colors, rectangles, clipping, z order, opacity, and
  stable node IDs.
- Retained document/tree state and explicit dirty flags.
- Layout integration and a layout-facing style model.
- Hit testing, pointer capture, keyboard focus, tab order, and event routing.
- Text measurement interfaces and text layout caching.
- Generic animation/state transitions.
- Scroll containers, popup positioning, and modal/layer behavior as generic
  mechanisms.
- A renderer-neutral paint list that backends can consume efficiently.

Generic widgets can live in Operad if they are domain-neutral:

- Button, icon button, checkbox, radio/segmented control.
- Slider, numeric input, text input.
- Dropdown/select, menu, tooltip, popover.
- Scrollbar and scroll area.
- Tabs, split panels, list/grid virtualization.

Application crates should own:

- Game hotbars, gear panels, map tools, chat behavior, inventory semantics, and
  any input binding that knows about game actions.
- Orbifold-specific music widgets, tuning systems, piano rolls, modulation
  graphs, meters, and transport semantics.
- Semiconductor-specific viewers, process-flow editors, wafer maps, device
  inspectors, and domain models.
- Debug menus and engine panels. They can use Operad later if desired, but they
  should not define Operad's core shape.

Renderer backends should be adapters, not the foundation. The core should not
depend on egui, wgpu, winit, or a particular game renderer by default. Optional
features such as `egui` can provide compatibility painting, and later backends
can target a custom wgpu renderer or CPU snapshot renderer. The boundary should
be a paint list plus font/text measurement traits, not direct calls from core UI
logic into a rendering framework.

## Shape I Would Like The Crate To Grow Toward

A good module split would be:

- `core`: IDs, document, nodes, style, geometry, colors.
- `layout`: taffy integration, layout cache, dirty propagation.
- `input`: event types, hit testing, focus, capture, command emission.
- `text`: measurement traits, glyphon/cosmic adapters, shaping cache.
- `animation`: transition machinery and per-node animated values.
- `widgets`: reusable controls built on the core.
- `paint`: renderer-neutral primitives and batching hints.
- `backends::egui`: optional egui painter/input bridge.
- `backends::wgpu`: optional future renderer that does not pull in game code.
- `testing`: synthetic input helpers, layout assertions, snapshot utilities.

The public API should prefer explicit data flow:

- Build or update a document from application state.
- Compute layout for a viewport.
- Feed input events into the document.
- Receive typed UI responses/commands.
- Generate a paint list.
- Let a backend render that paint list.

That keeps application state outside Operad while still making controls
testable. It also makes it easier for an agent to reason about changes because
state changes and side effects are visible at the API boundary.

## Performance Expectations

Operad should be designed around large operational interfaces, not just small
game HUDs. A 100x100 grid of buttons at 120 FPS is a useful baseline because it
forces the library to avoid accidental per-frame tree churn, repeated text
shaping, excessive allocation, and one draw call per rectangle.

Important performance rules:

- Reuse node identity and cached layout wherever possible.
- Separate visual changes from structural changes.
- Do not recompute layout for pointer hover unless layout-affecting state
  actually changed.
- Cache shaped text and measured sizes by text/style/constraint.
- Produce batchable paint primitives.
- Support virtualization for long lists and large grids.
- Keep popups/dropdowns out of normal flow so opening them does not reshape the
  whole document unexpectedly.

## What Would Make Operad Most Useful To Me

The most useful thing is a library where problems can be isolated. If a
dropdown opens in the wrong place, there should be a small Operad test for popup
placement. If scroll-wheel selection competes with game input, there should be a
testable input propagation rule. If a menu draws over a debug panel, the layer
and z-order policy should be explicit.

I also need the library to stay honest about what it does not own. Operad should
not know what a "hand tool" is, what a debug frame graph is, or how Orbifold
represents a tuning. It should provide the controls, event routing, layout, and
painting substrate that make those UI surfaces reliable.

In short: Operad should be boring, fast, testable infrastructure. The
interesting product decisions should stay in the applications.
