# Operad 2.0 Widget Roadmap

Operad `2.0.0` should build from the internal `1.0.0` baseline and focus on
turning the toolkit into a richer shared widget surface for the game, Fabricad
layout tools, and Orbifold. The current `main` branch does not yet contain the
`v1.0.0` branch commit, so v2 implementation should first merge or branch from
`v1.0.0`.

## Widget Ideas

1. Dropdown/select menu with keyboard navigation and popup placement.
2. Context menu and menu bar model for app command surfaces.
3. Calendar/date picker for scheduling, logs, history, and metadata panels.
4. Color picker with swatches, RGB/HSL controls, alpha, and palette slots.
5. Numeric input/drag value with units, ranges, precision, and commit phases.
6. Property inspector grid with labels, validation states, and mixed controls.
7. Virtualized data table with sticky headers, row selection, and column sizing.
8. Tree view/outliner with expand/collapse, selection, and indentation guides.
9. Tabs and tab groups for dense workspaces and inspectors.
10. Split pane and docking helpers with persisted panel sizes.
11. Modal/dialog/popover foundation with focus trapping and dismissal rules.
12. Command palette with filtering, shortcuts, recent commands, and dispatch IDs.
13. Toast/notification stack with severity, timeout, and action buttons.
14. Timeline/ruler widget for DAW, simulation, and editor views.
15. File/path picker with breadcrumbs, favorites, and recent paths.

## Suggested Build Order

1. Popup/menu foundation.
2. Dropdown/select and context/menu bar widgets.
3. Numeric input and property inspector grid.
4. Color picker and calendar/date picker.
5. Virtualized table and tree view.
6. Tabs, split panes, and docking helpers.
7. Command palette, toasts, timeline/ruler, and file/path picker.

This order creates shared infrastructure first, then builds increasingly
specialized widgets on top without leaking product-specific command enums or
domain state into Operad.
