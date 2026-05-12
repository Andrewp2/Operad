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
- A backend-neutral paint list so WGPU, CPU snapshot renderers, and tests
  consume the same display boundary; egui painting is kept only as explicit
  migration compatibility.
- Layout/audit snapshots for debugging panel bounds, clipping, focusability, and
  unreachable interactive controls.

The default feature set intentionally avoids `egui`, `glyphon`, `wgpu`, `winit`,
`accesskit`, and `cosmic-text`. With `--no-default-features`, the normal
dependency tree is currently `taffy`, `lyon_tessellation`, and their small
layout/geometry dependencies.

## Optional Layers

- `text-cosmic`: provides `CosmicTextMeasurer` for real text shaping and
  measurement while keeping public text styles backend-neutral.
- `egui`: provides host/input/platform compatibility for existing egui apps.
- `egui-renderer-compat`: keeps the legacy egui `PaintList` painter available
  for migrations that still need it; new renderer validation should use WGPU.
- `wgpu`: provides the native renderer path, including glyphon-backed text,
  lyon-backed geometry upload, and GPU compositing validation.
- `native-window`: adds winit/WGPU native-surface example support.
- `accesskit-winit`: provides the optional AccessKit publication bridge for
  winit hosts.
- `widgets`: provides domain-neutral builders and helpers for buttons, labels,
  checkboxes, sliders, editable text inputs, read-only selectable text,
  combo boxes, scroll areas, scrollbar thumb geometry, virtual lists, context
  menus, and table headers.
- `audit`: keeps the headless layout and paint audit surface available for
  consumers that want stricter checks around clipped text, duplicate names,
  unusable hit targets, and empty paint clips.

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

## Version 1.0 Scope

Operad `1.0.0` is an internal consumer baseline, not a promise that the toolkit
is complete. The stable expectation is that consumers can start building against
the retained document, neutral style/text/input types, scroll model, paint-list
boundary, audit hooks, optional egui/text adapters, and the initial widget/data
view helpers while still owning product-specific commands and domain editors.

The next milestones after `1.0.0` are richer shortcut scopes, menu/command
models, text selection painting, native clipboard adapters, more complete table
interactions, docking/splitter state helpers, and stronger serialized audit
export. Those should build on the same document, input, scroll, and paint-list
boundaries rather than adding app-specific behavior to core.
