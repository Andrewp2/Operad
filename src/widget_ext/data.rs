//! Data navigation widgets: property grids, tables, trees, and tabs.

use std::collections::HashSet;
use std::ops::Range;

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, Size as TaffySize, Style,
};

use crate::{
    ClipBehavior, ColorRgba, InputBehavior, ScrollAxes, StrokeStyle, TextStyle, TextWrap,
    UiDocument, UiNode, UiNodeId, UiNodeStyle, UiPoint, UiVisual,
};

/// Semantic hint for property value rendering and editing owned by the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyValueKind {
    Text,
    Number,
    Boolean,
    Choice,
    Color,
    Custom,
}

impl Default for PropertyValueKind {
    fn default() -> Self {
        Self::Text
    }
}

/// One row in a renderer-neutral property inspector grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyGridRow {
    pub id: String,
    pub label: String,
    pub value: String,
    pub value_kind: PropertyValueKind,
    pub editable: bool,
}

impl PropertyGridRow {
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: value.into(),
            value_kind: PropertyValueKind::Text,
            editable: true,
        }
    }

    pub fn with_kind(mut self, value_kind: PropertyValueKind) -> Self {
        self.value_kind = value_kind;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.editable = false;
        self
    }
}

/// Layout and styling knobs for [`property_inspector_grid`].
#[derive(Debug, Clone)]
pub struct PropertyInspectorOptions {
    pub layout: Style,
    pub label_width: f32,
    pub row_height: f32,
    pub selected_index: Option<usize>,
    pub background_visual: UiVisual,
    pub row_visual: UiVisual,
    pub selected_row_visual: UiVisual,
    pub label_style: TextStyle,
    pub value_style: TextStyle,
    pub read_only_value_style: TextStyle,
}

impl Default for PropertyInspectorOptions {
    fn default() -> Self {
        Self {
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
            label_width: 140.0,
            row_height: 28.0,
            selected_index: None,
            background_visual: UiVisual::panel(
                ColorRgba::new(20, 24, 30, 255),
                Some(StrokeStyle::new(ColorRgba::new(62, 72, 88, 255), 1.0)),
                4.0,
            ),
            row_visual: UiVisual::TRANSPARENT,
            selected_row_visual: UiVisual::panel(ColorRgba::new(43, 62, 86, 255), None, 0.0),
            label_style: muted_text_style(),
            value_style: TextStyle::default(),
            read_only_value_style: muted_text_style(),
        }
    }
}

/// Build a two-column property inspector using row IDs and a selected row index.
pub fn property_inspector_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    rows: &[PropertyGridRow],
    options: PropertyInspectorOptions,
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.background_visual),
    );

    for (index, row) in rows.iter().enumerate() {
        let visual = if options.selected_index == Some(index) {
            options.selected_row_visual
        } else {
            options.row_visual
        };
        let row_node = document.add_child(
            root,
            UiNode::container(
                format!("{name}.row.{}", row.id),
                UiNodeStyle {
                    layout: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: px(options.row_height),
                        },
                        ..Default::default()
                    },
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON)
            .with_visual(visual),
        );

        document.add_child(
            row_node,
            UiNode::text(
                format!("{name}.row.{}.label", row.id),
                &row.label,
                options.label_style.clone(),
                Style {
                    size: TaffySize {
                        width: px(options.label_width),
                        height: Dimension::percent(1.0),
                    },
                    padding: taffy::prelude::Rect::length(6.0),
                    ..Default::default()
                },
            ),
        );

        let value_style = if row.editable {
            options.value_style.clone()
        } else {
            options.read_only_value_style.clone()
        };
        document.add_child(
            row_node,
            UiNode::text(
                format!("{name}.row.{}.value", row.id),
                &row.value,
                value_style,
                Style {
                    flex_grow: 1.0,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::percent(1.0),
                    },
                    padding: taffy::prelude::Rect::length(6.0),
                    ..Default::default()
                },
            )
            .with_input(if row.editable {
                InputBehavior::BUTTON
            } else {
                InputBehavior::NONE
            }),
        );
    }

    root
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataCellAlignment {
    Start,
    Center,
    End,
}

impl Default for DataCellAlignment {
    fn default() -> Self {
        Self::Start
    }
}

/// Column metadata for virtualized data tables.
#[derive(Debug, Clone, PartialEq)]
pub struct DataTableColumn {
    pub id: String,
    pub label: String,
    pub width: f32,
    pub min_width: f32,
    pub alignment: DataCellAlignment,
    pub resizable: bool,
}

impl DataTableColumn {
    pub fn new(id: impl Into<String>, label: impl Into<String>, width: f32) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            width,
            min_width: 24.0,
            alignment: DataCellAlignment::Start,
            resizable: true,
        }
    }

    pub fn with_min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width.max(1.0);
        self
    }

    pub fn with_alignment(mut self, alignment: DataCellAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn fixed(mut self) -> Self {
        self.resizable = false;
        self
    }

    pub fn resolved_width(&self) -> f32 {
        self.width.max(self.min_width)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataTableCellIndex {
    pub row: usize,
    pub column: usize,
}

impl DataTableCellIndex {
    pub const fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataTableSelection {
    pub selected_rows: Vec<usize>,
    pub active_cell: Option<DataTableCellIndex>,
}

impl DataTableSelection {
    pub fn single_row(row: usize) -> Self {
        Self {
            selected_rows: vec![row],
            active_cell: None,
        }
    }

    pub fn with_active_cell(mut self, active_cell: DataTableCellIndex) -> Self {
        self.active_cell = Some(active_cell);
        self
    }

    pub fn contains_row(&self, row: usize) -> bool {
        self.selected_rows.contains(&row)
    }

    pub fn is_active_cell(&self, cell: DataTableCellIndex) -> bool {
        self.active_cell == Some(cell)
    }
}

/// Virtualization inputs for a data table body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualDataTableSpec {
    pub row_count: usize,
    pub row_height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub scroll_offset: UiPoint,
    pub overscan_rows: usize,
}

impl VirtualDataTableSpec {
    pub fn visible_rows(self) -> Range<usize> {
        if self.row_count == 0 || self.row_height <= f32::EPSILON {
            return 0..0;
        }
        let first = (self.scroll_offset.y.max(0.0) / self.row_height).floor() as usize;
        let visible = (self.viewport_height.max(0.0) / self.row_height).ceil() as usize + 1;
        let start = first.saturating_sub(self.overscan_rows).min(self.row_count);
        let end = (first + visible + self.overscan_rows).min(self.row_count);
        start..end
    }

    pub fn total_height(self) -> f32 {
        self.row_count as f32 * self.row_height.max(0.0)
    }

    pub fn row_at_viewport_y(self, y: f32) -> Option<usize> {
        if self.row_count == 0
            || self.row_height <= f32::EPSILON
            || self.viewport_height <= f32::EPSILON
            || y < 0.0
            || y >= self.viewport_height
        {
            return None;
        }
        let row = ((self.scroll_offset.y.max(0.0) + y) / self.row_height).floor() as usize;
        (row < self.row_count).then_some(row)
    }
}

#[derive(Debug, Clone)]
pub struct DataTableOptions {
    pub layout: Style,
    pub header_height: f32,
    pub selection: DataTableSelection,
    pub background_visual: UiVisual,
    pub header_visual: UiVisual,
    pub row_visual: UiVisual,
    pub selected_row_visual: UiVisual,
    pub active_cell_visual: UiVisual,
    pub header_text_style: TextStyle,
    pub cell_text_style: TextStyle,
}

impl Default for DataTableOptions {
    fn default() -> Self {
        Self {
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
            header_height: 30.0,
            selection: DataTableSelection::default(),
            background_visual: UiVisual::panel(
                ColorRgba::new(17, 21, 27, 255),
                Some(StrokeStyle::new(ColorRgba::new(62, 72, 88, 255), 1.0)),
                4.0,
            ),
            header_visual: UiVisual::panel(ColorRgba::new(32, 39, 49, 255), None, 0.0),
            row_visual: UiVisual::TRANSPARENT,
            selected_row_visual: UiVisual::panel(ColorRgba::new(38, 58, 84, 255), None, 0.0),
            active_cell_visual: UiVisual::panel(
                ColorRgba::new(50, 72, 104, 255),
                Some(StrokeStyle::new(ColorRgba::new(108, 180, 255, 255), 1.0)),
                0.0,
            ),
            header_text_style: muted_text_style(),
            cell_text_style: TextStyle::default(),
        }
    }
}

/// Build a virtualized table with fixed-width columns and app-owned cells.
pub fn virtualized_data_table(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    columns: &[DataTableColumn],
    spec: VirtualDataTableSpec,
    options: DataTableOptions,
    mut build_cell: impl FnMut(&mut UiDocument, UiNodeId, DataTableCellIndex),
) -> UiNodeId {
    let name = name.into();
    let table_width = data_table_width(columns).max(spec.viewport_width);
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.background_visual),
    );

    data_table_header(document, root, format!("{name}.header"), columns, &options);

    let body = document.add_child(
        root,
        UiNode::container(
            format!("{name}.body"),
            UiNodeStyle {
                layout: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: TaffySize {
                        width: px(spec.viewport_width),
                        height: px(spec.viewport_height),
                    },
                    ..Default::default()
                },
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_scroll(ScrollAxes::BOTH),
    );

    if let Some(scroll) = &mut document.node_mut(body).scroll {
        scroll.offset = UiPoint::new(spec.scroll_offset.x.max(0.0), spec.scroll_offset.y.max(0.0));
    }

    let visible_rows = spec.visible_rows();
    let top = visible_rows.start as f32 * spec.row_height;
    if top > 0.0 {
        document.add_child(
            body,
            vertical_spacer(format!("{name}.top_spacer"), table_width, top),
        );
    }

    for row in visible_rows.clone() {
        let visual = if options.selection.contains_row(row) {
            options.selected_row_visual
        } else {
            options.row_visual
        };
        let row_node = document.add_child(
            body,
            UiNode::container(
                format!("{name}.row.{row}"),
                UiNodeStyle {
                    layout: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        size: TaffySize {
                            width: px(table_width),
                            height: px(spec.row_height),
                        },
                        flex_shrink: 0.0,
                        ..Default::default()
                    },
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON)
            .with_visual(visual),
        );

        for (column_index, column) in columns.iter().enumerate() {
            let cell_index = DataTableCellIndex::new(row, column_index);
            let mut cell = UiNode::container(
                format!("{name}.row.{row}.cell.{}", column.id),
                UiNodeStyle {
                    layout: Style {
                        display: Display::Flex,
                        align_items: Some(AlignItems::Center),
                        justify_content: Some(justify_content(column.alignment)),
                        size: TaffySize {
                            width: px(column.resolved_width()),
                            height: Dimension::percent(1.0),
                        },
                        padding: taffy::prelude::Rect::length(6.0),
                        flex_shrink: 0.0,
                        ..Default::default()
                    },
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON);

            if options.selection.is_active_cell(cell_index) {
                cell = cell.with_visual(options.active_cell_visual);
            }

            let cell_node = document.add_child(row_node, cell);
            build_cell(document, cell_node, cell_index);
        }
    }

    let bottom = spec.row_count.saturating_sub(visible_rows.end) as f32 * spec.row_height;
    if bottom > 0.0 {
        document.add_child(
            body,
            vertical_spacer(format!("{name}.bottom_spacer"), table_width, bottom),
        );
    }

    root
}

pub fn data_table_width(columns: &[DataTableColumn]) -> f32 {
    columns.iter().map(DataTableColumn::resolved_width).sum()
}

pub fn data_table_column_at_x(columns: &[DataTableColumn], x: f32) -> Option<usize> {
    if x < 0.0 {
        return None;
    }
    let mut cursor = 0.0;
    for (index, column) in columns.iter().enumerate() {
        cursor += column.resolved_width();
        if x < cursor {
            return Some(index);
        }
    }
    None
}

pub fn data_table_cell_at_point(
    columns: &[DataTableColumn],
    spec: VirtualDataTableSpec,
    point: UiPoint,
) -> Option<DataTableCellIndex> {
    if spec.viewport_width <= f32::EPSILON || point.x < 0.0 || point.x >= spec.viewport_width {
        return None;
    }
    let row = spec.row_at_viewport_y(point.y)?;
    let column = data_table_column_at_x(columns, spec.scroll_offset.x.max(0.0) + point.x)?;
    Some(DataTableCellIndex::new(row, column))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeItem {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeItem>,
    pub disabled: bool,
}

impl TreeItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            disabled: false,
        }
    }

    pub fn with_children(mut self, children: Vec<TreeItem>) -> Self {
        self.children = children;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeVisibleItem {
    pub index: usize,
    pub id: String,
    pub label: String,
    pub depth: usize,
    pub parent_id: Option<String>,
    pub child_count: usize,
    pub expanded: bool,
    pub disabled: bool,
}

impl TreeVisibleItem {
    pub fn has_children(&self) -> bool {
        self.child_count > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TreeViewState {
    pub expanded_ids: Vec<String>,
    pub selected_index: Option<usize>,
}

impl TreeViewState {
    pub fn expanded(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            expanded_ids: ids.into_iter().map(Into::into).collect(),
            selected_index: None,
        }
    }

    pub fn is_expanded(&self, id: &str) -> bool {
        self.expanded_ids.iter().any(|expanded| expanded == id)
    }

    pub fn set_expanded(&mut self, id: impl Into<String>, expanded: bool) {
        let id = id.into();
        if expanded {
            if !self.expanded_ids.iter().any(|existing| existing == &id) {
                self.expanded_ids.push(id);
            }
        } else {
            self.expanded_ids.retain(|existing| existing != &id);
        }
    }

    pub fn toggle_expanded(&mut self, id: impl Into<String>) -> bool {
        let id = id.into();
        let expanded = !self.is_expanded(&id);
        self.set_expanded(id, expanded);
        expanded
    }

    pub fn select(&mut self, selected_index: Option<usize>) {
        self.selected_index = selected_index;
    }

    pub fn visible_items(&self, roots: &[TreeItem]) -> Vec<TreeVisibleItem> {
        let expanded: HashSet<&str> = self.expanded_ids.iter().map(String::as_str).collect();
        let mut visible = Vec::new();
        flatten_tree_items(roots, &expanded, 0, None, &mut visible);
        visible
    }

    pub fn selected_visible_item(&self, roots: &[TreeItem]) -> Option<TreeVisibleItem> {
        let selected_index = self.selected_index?;
        self.visible_items(roots)
            .into_iter()
            .find(|item| item.index == selected_index)
    }
}

#[derive(Debug, Clone)]
pub struct TreeViewOptions {
    pub layout: Style,
    pub row_height: f32,
    pub indent_width: f32,
    pub disclosure_width: f32,
    pub background_visual: UiVisual,
    pub row_visual: UiVisual,
    pub selected_row_visual: UiVisual,
    pub text_style: TextStyle,
    pub muted_text_style: TextStyle,
}

impl Default for TreeViewOptions {
    fn default() -> Self {
        Self {
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
            row_height: 26.0,
            indent_width: 16.0,
            disclosure_width: 18.0,
            background_visual: UiVisual::panel(
                ColorRgba::new(18, 22, 28, 255),
                Some(StrokeStyle::new(ColorRgba::new(58, 69, 84, 255), 1.0)),
                4.0,
            ),
            row_visual: UiVisual::TRANSPARENT,
            selected_row_visual: UiVisual::panel(ColorRgba::new(41, 59, 82, 255), None, 0.0),
            text_style: TextStyle::default(),
            muted_text_style: muted_text_style(),
        }
    }
}

/// Build a tree view/outliner from nested IDs and a visible selection index.
pub fn tree_view(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    roots: &[TreeItem],
    state: &TreeViewState,
    options: TreeViewOptions,
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.background_visual),
    );

    for item in state.visible_items(roots) {
        let visual = if state.selected_index == Some(item.index) {
            options.selected_row_visual
        } else {
            options.row_visual
        };
        let row = document.add_child(
            root,
            UiNode::container(
                format!("{name}.row.{}", item.id),
                UiNodeStyle {
                    layout: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: px(options.row_height),
                        },
                        ..Default::default()
                    },
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(if item.disabled {
                InputBehavior::NONE
            } else {
                InputBehavior::BUTTON
            })
            .with_visual(visual),
        );

        if item.depth > 0 {
            document.add_child(
                row,
                UiNode::container(
                    format!("{name}.row.{}.indent", item.id),
                    UiNodeStyle {
                        layout: Style {
                            size: TaffySize {
                                width: px(item.depth as f32 * options.indent_width),
                                height: Dimension::percent(1.0),
                            },
                            flex_shrink: 0.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
            );
        }

        let disclosure = if item.has_children() {
            if item.expanded {
                "v"
            } else {
                ">"
            }
        } else {
            ""
        };
        document.add_child(
            row,
            UiNode::text(
                format!("{name}.row.{}.disclosure", item.id),
                disclosure,
                options.muted_text_style.clone(),
                Style {
                    size: TaffySize {
                        width: px(options.disclosure_width),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                },
            ),
        );

        let style = if item.disabled {
            options.muted_text_style.clone()
        } else {
            options.text_style.clone()
        };
        document.add_child(
            row,
            UiNode::text(
                format!("{name}.row.{}.label", item.id),
                &item.label,
                style,
                Style {
                    flex_grow: 1.0,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                },
            ),
        );
    }

    root
}

pub fn outliner(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    roots: &[TreeItem],
    state: &TreeViewState,
    options: TreeViewOptions,
) -> UiNodeId {
    tree_view(document, parent, name, roots, state, options)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub disabled: bool,
    pub closable: bool,
    pub dirty: bool,
}

impl TabItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            disabled: false,
            closable: false,
            dirty: false,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }

    pub fn dirty(mut self) -> Self {
        self.dirty = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TabGroupState {
    pub selected_index: Option<usize>,
}

impl TabGroupState {
    pub const fn selected(selected_index: usize) -> Self {
        Self {
            selected_index: Some(selected_index),
        }
    }

    pub fn clamped_selected_index(self, tabs: &[TabItem]) -> Option<usize> {
        let selected = self.selected_index?;
        (selected < tabs.len()).then_some(selected)
    }

    pub fn selected_tab<'a>(self, tabs: &'a [TabItem]) -> Option<&'a TabItem> {
        tabs.get(self.clamped_selected_index(tabs)?)
    }

    pub fn selected_tab_id<'a>(self, tabs: &'a [TabItem]) -> Option<&'a str> {
        Some(self.selected_tab(tabs)?.id.as_str())
    }

    pub fn select_next(&mut self, tabs: &[TabItem]) -> Option<usize> {
        if tabs.is_empty() {
            self.selected_index = None;
            return None;
        }
        let start = self
            .selected_index
            .map(|index| (index.min(tabs.len() - 1) + 1) % tabs.len())
            .unwrap_or(0);
        for offset in 0..tabs.len() {
            let index = (start + offset) % tabs.len();
            if !tabs[index].disabled {
                self.selected_index = Some(index);
                return Some(index);
            }
        }
        None
    }

    pub fn select_previous(&mut self, tabs: &[TabItem]) -> Option<usize> {
        if tabs.is_empty() {
            self.selected_index = None;
            return None;
        }
        let start = self
            .selected_index
            .map(|index| (index.min(tabs.len() - 1) + tabs.len() - 1) % tabs.len())
            .unwrap_or(tabs.len() - 1);
        for offset in 0..tabs.len() {
            let index = (start + tabs.len() - offset) % tabs.len();
            if !tabs[index].disabled {
                self.selected_index = Some(index);
                return Some(index);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct TabGroupOptions {
    pub layout: Style,
    pub tab_strip_height: f32,
    pub min_tab_width: f32,
    pub background_visual: UiVisual,
    pub tab_visual: UiVisual,
    pub selected_tab_visual: UiVisual,
    pub panel_visual: UiVisual,
    pub text_style: TextStyle,
    pub muted_text_style: TextStyle,
}

impl Default for TabGroupOptions {
    fn default() -> Self {
        Self {
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            },
            tab_strip_height: 32.0,
            min_tab_width: 96.0,
            background_visual: UiVisual::panel(
                ColorRgba::new(16, 20, 26, 255),
                Some(StrokeStyle::new(ColorRgba::new(58, 69, 84, 255), 1.0)),
                4.0,
            ),
            tab_visual: UiVisual::panel(ColorRgba::new(28, 34, 43, 255), None, 0.0),
            selected_tab_visual: UiVisual::panel(ColorRgba::new(43, 52, 65, 255), None, 0.0),
            panel_visual: UiVisual::TRANSPARENT,
            text_style: TextStyle::default(),
            muted_text_style: muted_text_style(),
        }
    }
}

/// Build a tab group and call `build_panel` for the selected tab index.
pub fn tab_group(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    tabs: &[TabItem],
    state: TabGroupState,
    options: TabGroupOptions,
    mut build_panel: impl FnMut(&mut UiDocument, UiNodeId, usize),
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.background_visual),
    );

    let strip = document.add_child(
        root,
        UiNode::container(
            format!("{name}.strip"),
            UiNodeStyle {
                layout: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: px(options.tab_strip_height),
                    },
                    ..Default::default()
                },
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );

    let selected_index = state.clamped_selected_index(tabs);
    for (index, tab) in tabs.iter().enumerate() {
        let selected = selected_index == Some(index);
        let style = if tab.disabled {
            options.muted_text_style.clone()
        } else {
            options.text_style.clone()
        };
        let tab_node = document.add_child(
            strip,
            UiNode::container(
                format!("{name}.tab.{}", tab.id),
                UiNodeStyle {
                    layout: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        justify_content: Some(JustifyContent::Center),
                        size: TaffySize {
                            width: px(options.min_tab_width),
                            height: Dimension::percent(1.0),
                        },
                        padding: taffy::prelude::Rect::length(6.0),
                        flex_shrink: 0.0,
                        ..Default::default()
                    },
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(if tab.disabled {
                InputBehavior::NONE
            } else {
                InputBehavior::BUTTON
            })
            .with_visual(if selected {
                options.selected_tab_visual
            } else {
                options.tab_visual
            }),
        );

        let label = if tab.dirty {
            format!("{} *", tab.label)
        } else {
            tab.label.clone()
        };
        document.add_child(
            tab_node,
            UiNode::text(
                format!("{name}.tab.{}.label", tab.id),
                label,
                style,
                Style {
                    flex_grow: 1.0,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            ),
        );

        if tab.closable {
            document.add_child(
                tab_node,
                UiNode::text(
                    format!("{name}.tab.{}.close", tab.id),
                    "x",
                    options.muted_text_style.clone(),
                    Style {
                        size: TaffySize {
                            width: px(16.0),
                            height: Dimension::percent(1.0),
                        },
                        ..Default::default()
                    },
                )
                .with_input(InputBehavior::BUTTON),
            );
        }
    }

    let panel = document.add_child(
        root,
        UiNode::container(
            format!("{name}.panel"),
            UiNodeStyle {
                layout: Style {
                    flex_grow: 1.0,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                },
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.panel_visual),
    );

    if let Some(index) = selected_index {
        build_panel(document, panel, index);
    }

    root
}

fn data_table_header(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    columns: &[DataTableColumn],
    options: &DataTableOptions,
) -> UiNodeId {
    let name = name.into();
    let header = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: px(options.header_height),
                    },
                    ..Default::default()
                },
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.header_visual),
    );

    for column in columns {
        document.add_child(
            header,
            UiNode::text(
                format!("{name}.{}", column.id),
                &column.label,
                options.header_text_style.clone(),
                Style {
                    display: Display::Flex,
                    align_items: Some(AlignItems::Center),
                    justify_content: Some(justify_content(column.alignment)),
                    size: TaffySize {
                        width: px(column.resolved_width()),
                        height: Dimension::percent(1.0),
                    },
                    padding: taffy::prelude::Rect::length(6.0),
                    flex_shrink: 0.0,
                    ..Default::default()
                },
            ),
        );
    }

    header
}

fn vertical_spacer(name: impl Into<String>, width: f32, height: f32) -> UiNode {
    UiNode::container(
        name,
        UiNodeStyle {
            layout: Style {
                size: TaffySize {
                    width: px(width),
                    height: px(height),
                },
                flex_shrink: 0.0,
                ..Default::default()
            },
            ..Default::default()
        },
    )
}

fn flatten_tree_items(
    items: &[TreeItem],
    expanded: &HashSet<&str>,
    depth: usize,
    parent_id: Option<&str>,
    visible: &mut Vec<TreeVisibleItem>,
) {
    for item in items {
        let has_children = !item.children.is_empty();
        let is_expanded = has_children && expanded.contains(item.id.as_str());
        let index = visible.len();
        visible.push(TreeVisibleItem {
            index,
            id: item.id.clone(),
            label: item.label.clone(),
            depth,
            parent_id: parent_id.map(str::to_owned),
            child_count: item.children.len(),
            expanded: is_expanded,
            disabled: item.disabled,
        });
        if is_expanded {
            flatten_tree_items(&item.children, expanded, depth + 1, Some(&item.id), visible);
        }
    }
}

fn justify_content(alignment: DataCellAlignment) -> JustifyContent {
    match alignment {
        DataCellAlignment::Start => JustifyContent::FlexStart,
        DataCellAlignment::Center => JustifyContent::Center,
        DataCellAlignment::End => JustifyContent::FlexEnd,
    }
}

fn muted_text_style() -> TextStyle {
    TextStyle {
        color: ColorRgba::new(151, 162, 178, 255),
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn px(value: f32) -> Dimension {
    Dimension::length(value.max(0.0))
}

#[cfg(test)]
mod tests {
    use taffy::prelude::Size as TaffySize;

    use super::*;
    use crate::{length, ApproxTextMeasurer, UiContent, UiSize};

    fn test_root() -> UiDocument {
        UiDocument::new(crate::root_style(640.0, 480.0))
    }

    #[test]
    fn property_inspector_grid_builds_selectable_rows() {
        let mut doc = test_root();
        let rows = vec![
            PropertyGridRow::new("name", "Name", "Lead").read_only(),
            PropertyGridRow::new("gain", "Gain", "-3 dB").with_kind(PropertyValueKind::Number),
        ];
        let root = doc.root;
        let grid = property_inspector_grid(
            &mut doc,
            root,
            "props",
            &rows,
            PropertyInspectorOptions {
                selected_index: Some(1),
                ..Default::default()
            },
        );

        assert_eq!(doc.node(grid).children.len(), 2);
        let first_value = doc.node(doc.node(doc.node(grid).children[0]).children[1]);
        assert!(!first_value.input.pointer);
        let selected_row = doc.node(doc.node(grid).children[1]);
        assert_eq!(selected_row.visual.fill, ColorRgba::new(43, 62, 86, 255));
    }

    #[test]
    fn virtualized_data_table_ranges_and_hit_testing_use_scroll_offsets() {
        let columns = vec![
            DataTableColumn::new("name", "Name", 120.0),
            DataTableColumn::new("value", "Value", 80.0).with_alignment(DataCellAlignment::End),
        ];
        let spec = VirtualDataTableSpec {
            row_count: 100,
            row_height: 20.0,
            viewport_width: 160.0,
            viewport_height: 60.0,
            scroll_offset: UiPoint::new(50.0, 200.0),
            overscan_rows: 1,
        };

        assert_eq!(spec.visible_rows(), 9..15);
        assert_eq!(spec.row_at_viewport_y(5.0), Some(10));
        assert_eq!(
            data_table_cell_at_point(&columns, spec, UiPoint::new(80.0, 5.0)),
            Some(DataTableCellIndex::new(10, 1))
        );
        assert_eq!(
            data_table_cell_at_point(&columns, spec, UiPoint::new(-1.0, 5.0)),
            None
        );
        assert_eq!(
            data_table_cell_at_point(&columns, spec, UiPoint::new(80.0, 61.0)),
            None
        );
    }

    #[test]
    fn virtualized_data_table_builds_header_visible_rows_and_spacers() {
        let mut doc = test_root();
        let root = doc.root;
        let columns = vec![
            DataTableColumn::new("name", "Name", 120.0),
            DataTableColumn::new("value", "Value", 80.0),
        ];
        let spec = VirtualDataTableSpec {
            row_count: 100,
            row_height: 20.0,
            viewport_width: 180.0,
            viewport_height: 60.0,
            scroll_offset: UiPoint::new(0.0, 200.0),
            overscan_rows: 1,
        };
        let mut built_cells = Vec::new();
        let table = virtualized_data_table(
            &mut doc,
            root,
            "table",
            &columns,
            spec,
            DataTableOptions {
                selection: DataTableSelection::single_row(10)
                    .with_active_cell(DataTableCellIndex::new(10, 1)),
                ..Default::default()
            },
            |document, parent, cell| {
                built_cells.push(cell);
                document.add_child(
                    parent,
                    UiNode::text(
                        format!("cell.{}.{}", cell.row, cell.column),
                        format!("{}:{}", cell.row, cell.column),
                        TextStyle::default(),
                        Style {
                            size: TaffySize {
                                width: Dimension::auto(),
                                height: Dimension::auto(),
                            },
                            ..Default::default()
                        },
                    ),
                );
            },
        );

        let header = doc.node(table).children[0];
        let body = doc.node(table).children[1];
        assert_eq!(doc.node(header).children.len(), 2);
        assert_eq!(doc.node(body).children.len(), 8);
        assert_eq!(built_cells.len(), 12);

        doc.compute_layout(UiSize::new(640.0, 480.0), &mut ApproxTextMeasurer)
            .expect("layout");
        assert_eq!(doc.scroll_state(body).unwrap().content_size.height, 2000.0);
    }

    #[test]
    fn tree_view_state_flattens_expanded_items() {
        let roots = vec![TreeItem::new("project", "Project").with_children(vec![
            TreeItem::new("src", "src").with_children(vec![TreeItem::new("main", "main.rs")]),
            TreeItem::new("readme", "README.md"),
        ])];
        let mut state = TreeViewState::expanded(["project"]);
        state.select(Some(1));

        let visible = state.visible_items(&roots);
        assert_eq!(
            visible
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["project", "src", "readme"]
        );
        assert_eq!(visible[1].depth, 1);
        assert_eq!(state.selected_visible_item(&roots).unwrap().id, "src");

        assert!(state.toggle_expanded("src"));
        let visible = state.visible_items(&roots);
        assert_eq!(visible[2].id, "main");
        assert_eq!(visible[2].parent_id.as_deref(), Some("src"));
    }

    #[test]
    fn tree_view_builds_rows_with_disclosure_and_selection() {
        let mut doc = test_root();
        let root = doc.root;
        let roots =
            vec![TreeItem::new("project", "Project")
                .with_children(vec![TreeItem::new("src", "src")])];
        let mut state = TreeViewState::expanded(["project"]);
        state.select(Some(0));

        let tree = tree_view(
            &mut doc,
            root,
            "tree",
            &roots,
            &state,
            TreeViewOptions::default(),
        );

        assert_eq!(doc.node(tree).children.len(), 2);
        let first_row = doc.node(tree).children[0];
        assert_eq!(
            doc.node(first_row).visual.fill,
            ColorRgba::new(41, 59, 82, 255)
        );
        let disclosure = doc.node(doc.node(first_row).children[0]);
        assert!(matches!(&disclosure.content, UiContent::Text(text) if text.text == "v"));
    }

    #[test]
    fn tab_group_state_skips_disabled_tabs() {
        let tabs = vec![
            TabItem::new("one", "One"),
            TabItem::new("two", "Two").disabled(),
            TabItem::new("three", "Three"),
        ];
        let mut state = TabGroupState::selected(0);

        assert_eq!(state.select_next(&tabs), Some(2));
        assert_eq!(state.selected_tab_id(&tabs), Some("three"));
        assert_eq!(state.select_previous(&tabs), Some(0));

        let mut unselected = TabGroupState::default();
        assert_eq!(unselected.select_next(&tabs), Some(0));
        let mut unselected = TabGroupState::default();
        assert_eq!(unselected.select_previous(&tabs), Some(2));
    }

    #[test]
    fn tab_group_builds_strip_and_selected_panel() {
        let mut doc = test_root();
        let root = doc.root;
        let tabs = vec![
            TabItem::new("inspect", "Inspect").closable(),
            TabItem::new("history", "History").dirty(),
        ];
        let group = tab_group(
            &mut doc,
            root,
            "tabs",
            &tabs,
            TabGroupState::selected(1),
            TabGroupOptions {
                layout: Style {
                    size: TaffySize {
                        width: length(320.0),
                        height: length(180.0),
                    },
                    ..TabGroupOptions::default().layout
                },
                ..Default::default()
            },
            |document, panel, selected_index| {
                document.add_child(
                    panel,
                    UiNode::text(
                        "selected_panel",
                        format!("tab {selected_index}"),
                        TextStyle::default(),
                        Style {
                            size: TaffySize {
                                width: Dimension::auto(),
                                height: Dimension::auto(),
                            },
                            ..Default::default()
                        },
                    ),
                );
            },
        );

        let strip = doc.node(group).children[0];
        let panel = doc.node(group).children[1];
        assert_eq!(doc.node(strip).children.len(), 2);
        assert_eq!(doc.node(panel).children.len(), 1);
        let selected_tab = doc.node(strip).children[1];
        assert_eq!(
            doc.node(selected_tab).visual.fill,
            ColorRgba::new(43, 52, 65, 255)
        );
    }
}
