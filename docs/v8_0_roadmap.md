# Operad 8.0 Roadmap

V8 should make Operad feel like a serious application UI toolkit: explainable,
instrumented, editor-friendly, and ready for large native app surfaces. V7 made
the widget showcase, layout pipeline, scroll behavior, overlays, animation
runtime, and release gates much stronger. V8 should build on that by giving
developers the tools to understand, debug, automate, and compose complex UI
systems.

## Product Goal

Operad should be the UI substrate a real app can trust when the screen gets
dense:

- Layout failures should be inspectable from computed size, position, clipping,
  scroll, z-order, and accessibility data.
- Interactive surfaces should expose commands, hotkeys, canvas input capture,
  accessibility metadata, and test replay through the same retained document
  model.
- Animation should be state-machine driven, inspectable, and useful for real
  interaction feedback instead of limited to decorative demos.
- Showcase should remain a teaching surface, while deeper validation lives in
  diagnostic tools, replay harnesses, and state-matrix tests.

## V8 Themes

### Alpha Implementation Map

The first v8 slices are now available as reusable, domain-neutral APIs rather
than showcase-only code:

- Live layout, paint, intrinsic-size, scroll, accessibility, audit, and
  animation data are collected by `DebugInspectorSnapshot`.
- `debug_inspector_panel`, `accessibility_overlay_panel`, and
  `accessibility_debug_overlay` expose that snapshot through reusable inspector
  and visual overlay widgets.
- `AccessibilityKeyboardNavigationTrace` records focus movement, blocked focus
  boundaries, focused accessibility action activation, and command shortcut
  dispatch for keyboard-navigation diagnostics.
- Animation machines expose states, transitions, active transition, inputs, and
  blend bindings for inspector UIs.
- `DebugAnimationGraph` and `animation_state_graph_panel` render compact
  state-machine graphs, including transition conditions, active progress, and
  blend bindings.
- `animation_inspector_controls_panel` renders reusable pause/resume, step,
  scrub, bool toggle, number edit, and trigger-fire controls from animation
  inspector snapshots, leaving actual app mutation to action handlers.
- Showcase Diagnostics now owns those action handlers for pause, step, scrub,
  bool, number, and trigger inputs, with replay coverage outside
  `examples/showcase.rs`.
- Command palette registry adapters are complemented by
  `command_diagnostics_panel`, which shows command metadata, shortcuts, disabled
  states, and shortcut conflicts.
- `CommandPaletteHistory` lets applications keep recent commands and feed that
  recency back into registry-built palette items without Operad owning the
  command namespace.
- `ShortcutRemap` and `CommandRegistry::rebind_shortcut` provide atomic
  shortcut preference updates, conflict reporting, and binding inspection for
  hotkey settings UIs.
- Canvas raw mouse motion includes the captured canvas ID when pointer lock is
  active, so native hosts can route flycam/editor deltas to the owning canvas.
- `CanvasHostCaptureDiagnosticReport` explains active, acquired, updated,
  released, unavailable, and denied capture lifecycles with matching cursor
  platform requests and responses.
- `InteractionRecorder` and `EventReplay::long_wheel_scroll` preserve the long
  scroll regression cases outside showcase source.
- Event replay can now include direct command steps, so command palette
  selections and hotkey-routed commands can be asserted through the same replay
  report.
- Event replay can also record platform responses, letting scenario harnesses
  replay host service outcomes alongside pointer, keyboard, scroll, and command
  input.
- Event replay includes window-resize steps, and `ScenarioHarness` applies the
  final replayed viewport before layout/render for responsive regression tests.
- `theme_editor_panel` renders theme tokens and component-state rows from
  `DebugThemeSnapshot`, and `ThemePatchExport` can diff edited themes into a
  typed `ThemePatch`, changed-token list, and Rust builder snippet.
- `ThemeAccessibilityAudit` checks component text/icon contrast and validates
  reduced-motion, forced-colors, reduced-transparency, and text-scale policy
  application against a baseline theme.
- `layout_animation_transitions` diffs previous and current layout snapshots
  after layout and returns visual interpolation records without feeding animated
  positions back into layout.
- `HostDocumentFrameRequest` can carry the previous layout snapshot plus
  layout-animation options, and `HostDocumentFrameOutput` now publishes current
  layout snapshots and reduced-motion-aware layout transition records.
- `apply_layout_animation_transitions_to_paint_list` applies those transitions
  to paint output after layout, and document frames now feed the transformed
  paint list to render requests while keeping computed layout authoritative.
- `DockWorkspaceLayoutSnapshot` captures and restores dock panel side, size, and
  visibility so editor workspaces can persist layouts outside the renderer.
- Dock workspaces publish domain-neutral panel drag payloads, drag sources,
  drop zones, reorder targets, drawer visibility state, drawer rail widgets,
  and floating-panel state helpers for side docking, center docking,
  undock/floating-panel targets, and panel ordering.
- Dock workspace state can commit host-resolved drop hits back into dock,
  float, and reorder state, so platform drag sessions and tests share the same
  target contract.
- Showcase uses the public dock workspace surface for the Layout widgets window,
  including docked inspector/assets panels, tabbed center content, drop-target
  previews, drawer toggles, reorder target previews, and a floating inspector
  flow.
- `virtualized_tree_view` uses the shared virtualization planner to materialize
  only the current tree viewport, with stable tree row accessibility and
  scroll-spacer content sizing.
- Virtualized trees can publish focus-preservation records from stable item IDs
  across filtering/expansion, and showcase includes a tree-table pattern built
  from tree-flattening plus the public virtualized table surface.
- `VirtualizationDiagnostics` and `VirtualizationBudget` summarize visible,
  materialized, overscan, sticky, and materialization-ratio metrics for
  list/table/tree/grid surfaces and flag large-data budget risks.
- Virtualized data table headers expose column sort/filter/resize command
  metadata as pointer and accessibility actions, and showcase demonstrates
  public sort, filter, selection, scroll, and column-width reset/resize flows.
- Showcase includes a Diagnostics window that demonstrates the v8 inspector,
  accessibility, command, and theme surfaces using public APIs.

### 1. Live Layout Inspector

Add an in-app inspector that can target any node and show the data needed to
debug layout without guessing:

- Computed minimum, preferred, and final size.
- Margins, padding, flex basis, axis, gaps, and alignment.
- Parent constraints, intrinsic text measurement, and canvas/aspect constraints.
- Position, visible rect, clip chain, scroll viewport, content size, and scroll
  offset.
- Paint z, platform layer, hit-test eligibility, focus state, and pointer target.
- Accessibility role, label, value, actions, focus order, and relation metadata.

The inspector should be reusable outside showcase so applications can mount it
behind their own debug UI.

### 2. Animation State Machine Inspector

Make the Rive-inspired animation runtime visible and debuggable:

- Show states, transitions, active transition, elapsed time, current values, and
  current blend inputs.
- Allow toggling bool inputs, editing number inputs, firing triggers, pausing,
  stepping, and scrubbing time.
- Visualize topology morphing and property blending for scene primitives.
- Add a compact state-machine graph view for common cases.

The goal is that animation bugs can be inspected through runtime state rather
than inferred from the rendered frame.

### 3. Command Palette And Hotkeys

Add a first-class command system suitable for application shells:

- Global and scoped command registry.
- Keyboard shortcut registration, conflict detection, platform modifier
  formatting, and remapping hooks.
- Searchable command palette widget with disabled states, descriptions,
  shortcuts, categories, and recent commands.
- Integration with buttons, menu items, context menus, accessibility actions,
  and replay tests.

Commands should remain application-owned; Operad should own routing,
discoverability, diagnostics, and widget glue.

### 4. Canvas Input Capture

Make editor and flycam-style canvases first-class:

- Key press and release delivery.
- Raw mouse motion and local pointer deltas.
- Wheel phases, button state, modifiers, and local canvas coordinates.
- Cursor grab, cursor visibility, cursor icon, and pointer-lock style platform
  requests.
- Capture lifecycle diagnostics so apps can tell why capture was denied,
  released, or unavailable.

This should support 2D editors, 3D flycams, custom drawing tools, and future
hotkey-heavy canvases without app-specific host hacks.

### 5. Interaction Recorder

Add a recorder that can turn real UI interaction into deterministic tests:

- Record pointer, keyboard, text, scroll, window resize, focus, command, and
  platform-response events.
- Replay against a document/app harness with stable timing.
- Assert final layout, scroll offsets, focused node, command output,
  accessibility tree, paint counts, and selected snapshots.
- Provide helpers for stress cases, such as synthetic long scrolls to verify
  scrollbar end positions.

This should make regressions from showcase bugs easy to preserve without putting
tests in showcase itself.

### 6. Theme And Style Editor

Build a reusable style editor for live-tuning visual systems:

- Edit colors, typography, spacing, radii, shadows, borders, opacity, focus
  rings, and widget state visuals.
- Preview changes against real widgets and dense app-like panels.
- Export a theme patch or Rust theme builder snippet.
- Include contrast, reduced-motion, forced-colors, and text-scale checks.

The editor should help avoid one-off showcase knobs and make theming an
application feature.

### 7. Virtualized Tree And Table 2.0

Invest in dense data widgets that applications actually need:

- Sticky headers, column resize, sort, filter, selection, keyboard navigation,
  row actions, and accessible row/column metadata.
- Lazy row materialization with measured row heights and stable scroll anchors.
- Tree-table patterns, expandable rows, and focus preservation across
  filtering/virtualization.
- Performance diagnostics for large data sets.

This is less flashy than animation work, but it is central for serious tools.

### 8. Layout Animations

Add animation for layout changes, not only paint properties:

- Animate between previous and current layout rects when widgets move, resize,
  expand, collapse, reorder, or appear/disappear.
- Keep layout authoritative: size and position are still computed first, then
  transitions interpolate visual presentation.
- Respect reduced-motion preferences and expose animation state to diagnostics.

This should make collapsible headers, docking panels, lists, and popup
transitions feel coherent without compromising layout correctness.

### 9. Accessibility Debug Overlay

Add a visual overlay for accessibility structure:

- Tab order, focus traps, modal scopes, labels, roles, names, actions, and
  relations.
- Contrast warnings, missing accessible names/actions, and hidden/focusable
  mismatches.
- Screen-reader tree preview with stable node names.
- Keyboard navigation trace for focus movement and command activation.

This should make accessibility defects as visible as layout defects.

### 10. Docking Workspace

Add an editor-grade workspace system:

- Split panes, tabbed panels, dock/undock, floating panels, drawers, sidebars,
  and persisted layouts.
- Panel drag targets, resize handles, collapse/restore, focus restoration, and
  keyboard navigation.
- Integration with overlays, command palette, scroll containers, and layout
  inspector.

This would make Operad immediately useful for editor-style applications and
large operational tools.

## Release Gates

V8 should not ship these as isolated demos. The release should include:

- Showcase examples for each major feature, using public APIs only.
- Replay/state-matrix tests for interactive behavior.
- Layout/a11y/performance diagnostics for the new surfaces.
- No tests embedded in showcase source.
- No app-specific domain semantics in public APIs.
- Package docs that distinguish stable v8 APIs from experimental diagnostic
  helpers.
