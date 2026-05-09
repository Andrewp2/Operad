# Operad 2.0 Widget Roadmap

Operad `2.0.0` builds from the internal `1.0.0` baseline and turns the toolkit
into a richer shared widget surface for the game, Fabricad layout tools, and
Orbifold. The 2.0 widgets are renderer-neutral and application-command-neutral:
they use IDs, selection indexes, state structs, and Operad nodes rather than
knowing product-specific command enums or domain state.

## Implemented Widgets

1. Dropdown/select menu with keyboard navigation and popup placement.
2. Context menu and menu bar model for app command surfaces.
3. Calendar/date picker for scheduling, logs, history, and metadata panels.
4. Color picker with swatches, HSV/RGBA helpers, alpha, and palette slots.
5. Numeric input/drag value with ranges, precision, speed, and edit phases.
6. Property inspector grid with labels, value kinds, and selectable rows.
7. Virtualized data table with sticky headers, row selection, and column sizing.
8. Tree view/outliner with expand/collapse, selection, and indentation guides.
9. Tabs and tab groups for dense workspaces and inspectors.
10. Split pane and docking helpers with persisted panel sizes.
11. Modal/dialog/popover foundation with dismissal and placement rules.
12. Command palette with filtering, ranking, shortcuts, and dispatch IDs.
13. Toast/notification stack with severity, timeout, and action buttons.
14. Timeline/ruler widget for DAW, simulation, and editor views.
15. File/path picker with breadcrumbs, favorites, and recent paths.

## Module Map

- `src/widget_ext/menu.rs`: popup placement, dropdown/select, context menus,
  menu bars, and command palette.
- `src/widget_ext/pickers.rs`: calendar/date picker, color picker, numeric
  input/drag values, and path picker.
- `src/widget_ext/data.rs`: property inspector grid, virtual data table, tree
  view/outliner, and tabs.
- `src/widget_ext/surfaces.rs`: split panes, docking workspace, dialog/popover
  state, toast stack, and timeline/ruler.

These modules are re-exported through the existing `widgets` feature so
consumers can opt into one widget surface without pulling renderer or text
backend dependencies into core.
