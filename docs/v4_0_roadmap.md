# Operad 4.0 Roadmap

Operad `4.0.0` picks up the renderer and release-hardening work intentionally
deferred from `3.0.0`. Downstream applications should validate their own
adoption paths, but those migrations are not Operad release blockers once the
library-owned gates are green.

## Release Goal

V4 should turn the v3 contracts into a more complete production UI platform:
less residual backend leakage, stronger native/canvas integration, richer
editing widgets, real accessibility adapter paths, and renderer-level
performance coverage.

## 1. Finish Operad-Owned Layout Types

V3 moved stored node/widget layout styles behind `LayoutStyle`, but some helper
APIs still expose Taffy dimension and spacing types.

V4 should consider:

- Operad-owned dimension, inset, spacing, alignment, and flex enums.
- Public query/builder methods for common layout inspection and mutation.
- Layout serialization/debug output that does not expose backend internals.
- A backend translation layer that keeps Taffy replaceable.
- Additional helpers only when they map to real shared consumer needs.

## 2. Production Canvas And Native Viewports

V3 defines renderer-neutral canvas contracts, hit targets, and host-capture
state. V4 should prove them in realistic app surfaces.

Targets:

- WGPU/native viewport examples for Fabricad 2D mask layout, 3D viewport, wafer
  maps, and cross-section painters.
- Orbifold editor surfaces with playhead, note/range hit targets, pointer
  capture, and keyboard focus.
- Game renderer callbacks for HUD thumbnails, inventory/menus, and debug
  overlays.
- Domain hit-target accessibility summaries for canvas surfaces.
- Pointer-lock, cursor confinement, focus, and IME behavior across host adapters.

## 3. Renderer-Level Performance Tests

Current v3 performance smoke tests rely on deterministic test render output. V4
should add renderer/backend conformance and speed tests closer to production.

Targets:

- WGPU or GPU-adjacent batch/render benchmarks.
- Paint-batch reuse and retained display-list invalidation under real editor
  workloads.
- Large canvas/image resource-update scenarios.
- Frame budget baselines for hover, drag, scroll, text edit, and high-frequency
  playhead/meter updates.
- CI-friendly thresholds that avoid flaky hardware-specific assertions.

## 4. Accessibility Adapter Implementation

V3 has strong metadata, audits, and adapter request contracts. V4 should connect
more of that to real platform behavior.

Targets:

- Screen-reader tree publication for supported hosts.
- Live-region announcement delivery where the backend can support it.
- Focus trap/restore integration with host windows and overlays.
- High contrast, forced colors, reduced motion, reduced transparency, and text
  scale policies applied consistently across themes and renderers.
- Canvas/editor hit target summaries exposed as navigable accessibility items.

## 5. Text, Lists, And Forms To Production Quality

V3 text input, searchable selects, data tables, trees, and form metadata are
usable, but v4 should harden them for long editing sessions.

Targets:

- Text field undo/redo, robust IME composition, multiline selection polish, and
  host clipboard edge cases.
- Listbox/combo popup positioning, virtualized option lists, active-descendant
  focus routing, and keyboard typeahead across overlays.
- Numeric controls with richer drag/keyboard acceleration, validation, unit
  formatting, and one-commit edit phases.
- Property forms with field-level validation, pending/apply/cancel flows, and
  accessible error summaries.
- Copy/export support across tree, table, form, and editor selections.

## 6. Widget Visual And Motion Quality

The v3 widget set is broad. V4 should make it feel more like a coherent product
toolkit.

Targets:

- Image/icon buttons as first-class controls.
- Shader/effect hooks with clear fallback behavior.
- Animation state-machine presets for common hover, press, open, close,
  selection, validation, and loading states.
- Denser theme variants for operational tools, editor surfaces, and game HUDs.
- Stronger visual snapshots for disabled, focused, invalid, warning, selected,
  pending, and active states.

## 7. App Shell Runtime

V3 includes shell state, layout planning, and shell document bridges. V4 should
move closer to a reusable host/runtime layer.

Targets:

- Persistent shell host that can replace more egui `TopBottomPanel`,
  `SidePanel`, `CentralPanel`, and floating-window usage.
- Menu/toolbar/status/transport bars with command routing and overflow.
- Dock, tab, split, collapse, restore, scroll-sync, and focus restore as one
  coherent workflow.
- Overlay stack coordination for dialogs, popovers, command palette, and
  listbox popups.

## 8. Downstream Reference Migrations

Downstream migrations are adoption work owned by each application. They are
still useful validation targets for future Operad work, but they are not part
of the Operad `4.0.0` release gate.

Reference goals:

- Game: move more HUD/menu/editor surfaces off direct egui assumptions.
- Orbifold: migrate main menu, dense numeric controls, custom editor labels, and
  shell panels through Operad-owned primitives.
- Fabricad/layout: migrate more inspector/drawer/control surfaces, prove wafer
  map or cross-section canvas integration, and reduce fallback egui widgets.

## V4 Release Gate

V4 should keep the v3 quality gate and add:

- At least one renderer/native-viewport conformance test.
- Clear downstream migration guidance and compatibility notes.
- A migration guide from v3 to v4.
- A documented decision on any remaining public backend type leakage.
- A release checklist that includes accessibility and performance sign-off.

The status and evidence for each gate is tracked in:

- [v4 release checklist](/home/andrew-peterson/code/operad/docs/v4_0_release_checklist.md)

Follow-up interaction-runtime work is tracked in:

- [v5 roadmap](/home/andrew-peterson/code/operad/docs/v5_0_roadmap.md)
