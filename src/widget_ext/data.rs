//! Data navigation widgets: property grids, tables, trees, and tabs.

use std::collections::HashSet;
use std::ops::Range;

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentageAuto,
    Size as TaffySize, Style,
};

use crate::{
    commands::CommandEffect,
    platform::{ClipboardRequest, DragOperation, DragPayload},
    AccessibilityAction, AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole,
    ClipBehavior, ColorRgba, CommandId, DragDropSurfaceKind, DragSourceDescriptor, DragSourceId,
    DropPayloadFilter, DropTargetDescriptor, DropTargetId, ImageContent, InputBehavior, ScrollAxes,
    ShaderEffect, StrokeStyle, TextStyle, TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle,
    UiPoint, UiRect, UiVisual,
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

/// Domain-neutral state carried by one property inspector row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PropertyRowStatus {
    pub invalid: Option<String>,
    pub error: Option<String>,
    pub warning: Option<String>,
    pub help: Option<String>,
    pub changed: bool,
    pub pending: bool,
}

impl PropertyRowStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn invalid(mut self, reason: impl Into<String>) -> Self {
        self.invalid = Some(reason.into());
        self
    }

    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }

    pub fn warning(mut self, message: impl Into<String>) -> Self {
        self.warning = Some(message.into());
        self
    }

    pub fn help(mut self, message: impl Into<String>) -> Self {
        self.help = Some(message.into());
        self
    }

    pub fn changed(mut self) -> Self {
        self.changed = true;
        self
    }

    pub fn pending(mut self) -> Self {
        self.pending = true;
        self
    }

    pub fn has_visual_status(&self) -> bool {
        self.invalid.is_some()
            || self.error.is_some()
            || self.warning.is_some()
            || self.changed
            || self.pending
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
    pub disabled: bool,
    pub status: PropertyRowStatus,
    pub leading_image: Option<ImageContent>,
}

impl PropertyGridRow {
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: value.into(),
            value_kind: PropertyValueKind::Text,
            editable: true,
            disabled: false,
            status: PropertyRowStatus::default(),
            leading_image: None,
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

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn with_status(mut self, status: PropertyRowStatus) -> Self {
        self.status = status;
        self
    }

    pub fn invalid(mut self, reason: impl Into<String>) -> Self {
        self.status = self.status.invalid(reason);
        self
    }

    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.status = self.status.error(message);
        self
    }

    pub fn warning(mut self, message: impl Into<String>) -> Self {
        self.status = self.status.warning(message);
        self
    }

    pub fn help(mut self, message: impl Into<String>) -> Self {
        self.status = self.status.help(message);
        self
    }

    pub fn changed(mut self) -> Self {
        self.status = self.status.changed();
        self
    }

    pub fn pending(mut self) -> Self {
        self.status = self.status.pending();
        self
    }

    pub fn with_leading_image(mut self, image: ImageContent) -> Self {
        self.leading_image = Some(image);
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
    pub focused_index: Option<usize>,
    pub background_visual: UiVisual,
    pub row_visual: UiVisual,
    pub selected_row_visual: UiVisual,
    pub status_row_visual: UiVisual,
    pub selected_row_shader: Option<ShaderEffect>,
    pub focused_row_shader: Option<ShaderEffect>,
    pub status_row_shader: Option<ShaderEffect>,
    pub label_style: TextStyle,
    pub value_style: TextStyle,
    pub read_only_value_style: TextStyle,
    pub leading_image_size: f32,
    pub accessibility_label: Option<String>,
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
            focused_index: None,
            background_visual: UiVisual::panel(
                ColorRgba::new(20, 24, 30, 255),
                Some(StrokeStyle::new(ColorRgba::new(62, 72, 88, 255), 1.0)),
                4.0,
            ),
            row_visual: UiVisual::TRANSPARENT,
            selected_row_visual: UiVisual::panel(ColorRgba::new(43, 62, 86, 255), None, 0.0),
            status_row_visual: UiVisual::TRANSPARENT,
            selected_row_shader: None,
            focused_row_shader: None,
            status_row_shader: None,
            label_style: muted_text_style(),
            value_style: TextStyle::default(),
            read_only_value_style: muted_text_style(),
            leading_image_size: 16.0,
            accessibility_label: None,
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
        .with_visual(options.background_visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Grid)
                .label(accessibility_label_or_name(
                    &options.accessibility_label,
                    &name,
                ))
                .value(format!("{} properties", rows.len())),
        ),
    );

    for (index, row) in rows.iter().enumerate() {
        let selected = options.selected_index == Some(index);
        let focused = options.focused_index == Some(index);
        let visual = property_row_visual(row, selected, &options);
        let shader = property_row_shader(row, selected, focused, &options);
        let row_node = with_optional_shader(
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
            .with_input(if row.disabled {
                InputBehavior::NONE
            } else {
                InputBehavior::BUTTON
            })
            .with_visual(visual)
            .with_accessibility(property_row_accessibility(
                row,
                index,
                rows.len(),
                selected,
                focused,
            )),
            shader.as_ref(),
        );
        let row_node = document.add_child(root, row_node);

        if let Some(image) = row.leading_image.clone() {
            document.add_child(
                row_node,
                leading_image_node(
                    format!("{name}.row.{}.image", row.id),
                    image,
                    options.leading_image_size,
                    Some(row.label.clone()),
                ),
            );
        }

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
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Label).label(row.label.clone()),
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
                if row.disabled {
                    InputBehavior::NONE
                } else {
                    InputBehavior::BUTTON
                }
            } else {
                InputBehavior::NONE
            })
            .with_accessibility(property_value_accessibility(row, selected, focused)),
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

/// Why a dense data view has no rows to present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataViewEmptyReason {
    NoRows,
    NoMatches,
    NoVisibleRows,
}

impl Default for DataViewEmptyReason {
    fn default() -> Self {
        Self::NoRows
    }
}

/// Renderer-neutral empty-state copy and metadata for dense lists and tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataViewEmptyState {
    pub reason: DataViewEmptyReason,
    pub title: String,
    pub message: Option<String>,
    pub action_label: Option<String>,
    pub query: Option<String>,
}

impl DataViewEmptyState {
    pub fn new(reason: DataViewEmptyReason, title: impl Into<String>) -> Self {
        Self {
            reason,
            title: title.into(),
            message: None,
            action_label: None,
            query: None,
        }
    }

    pub fn no_rows(title: impl Into<String>) -> Self {
        Self::new(DataViewEmptyReason::NoRows, title)
    }

    pub fn no_matches(query: impl Into<String>, title: impl Into<String>) -> Self {
        Self::new(DataViewEmptyReason::NoMatches, title).query(query)
    }

    pub fn no_visible_rows(title: impl Into<String>) -> Self {
        Self::new(DataViewEmptyReason::NoVisibleRows, title)
    }

    pub fn for_counts(
        total_row_count: usize,
        visible_row_count: usize,
        query: impl AsRef<str>,
    ) -> Option<Self> {
        if visible_row_count > 0 {
            return None;
        }

        let query = query.as_ref().trim();
        if total_row_count == 0 {
            Some(Self::no_rows("No rows"))
        } else if query.is_empty() {
            Some(Self::no_visible_rows("No visible rows"))
        } else {
            Some(Self::no_matches(query, "No matching rows"))
        }
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn action_label(mut self, action_label: impl Into<String>) -> Self {
        self.action_label = Some(action_label.into());
        self
    }

    pub fn query(mut self, query: impl Into<String>) -> Self {
        let query = query.into();
        self.query = (!query.is_empty()).then_some(query);
        self
    }

    pub fn is_filter_empty(&self) -> bool {
        self.reason == DataViewEmptyReason::NoMatches
    }

    pub fn accessibility_value(&self) -> String {
        let mut value = vec![data_view_empty_reason_label(self.reason).to_owned()];
        if let Some(query) = &self.query {
            value.push(format!("query {query}"));
        }
        if let Some(message) = &self.message {
            value.push(message.clone());
        }
        if let Some(action) = &self.action_label {
            value.push(format!("action {action}"));
        }
        value.join("; ")
    }

    pub fn accessibility(&self) -> AccessibilityMeta {
        let meta = AccessibilityMeta::new(AccessibilityRole::Status)
            .label(self.title.clone())
            .value(self.accessibility_value());
        if self.is_filter_empty() {
            meta.live_region(AccessibilityLiveRegion::Polite)
        } else {
            meta
        }
    }
}

/// Section metadata that can be inserted into a dense list/table row stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataViewSectionHeader {
    pub id: String,
    pub label: String,
    pub row_count: usize,
    pub collapsed: bool,
}

impl DataViewSectionHeader {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            row_count: 0,
            collapsed: false,
        }
    }

    pub fn with_row_count(mut self, row_count: usize) -> Self {
        self.row_count = row_count;
        self
    }

    pub fn collapsed(mut self) -> Self {
        self.collapsed = true;
        self
    }

    pub fn accessibility(&self, section_index: usize, section_count: usize) -> AccessibilityMeta {
        let mut value = vec![
            format!("section {} of {}", section_index + 1, section_count),
            format!("{} rows", self.row_count),
        ];
        push_state(&mut value, "collapsed", self.collapsed);

        AccessibilityMeta::new(AccessibilityRole::RowHeader)
            .label(self.label.clone())
            .value(value.join("; "))
            .expanded(!self.collapsed)
    }
}

/// Stable identity for a row after filtering, sorting, or section flattening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataViewRow {
    pub id: String,
    pub source_index: usize,
    pub section_id: Option<String>,
}

impl DataViewRow {
    pub fn new(id: impl Into<String>, source_index: usize) -> Self {
        Self {
            id: id.into(),
            source_index,
            section_id: None,
        }
    }

    pub fn in_section(mut self, section_id: impl Into<String>) -> Self {
        self.section_id = Some(section_id.into());
        self
    }
}

/// One entry in a flattened dense data view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataViewEntry {
    SectionHeader(DataViewSectionHeader),
    Row(DataViewRow),
}

impl DataViewEntry {
    pub fn id(&self) -> &str {
        match self {
            Self::SectionHeader(section) => section.id.as_str(),
            Self::Row(row) => row.id.as_str(),
        }
    }

    pub fn is_section_header(&self) -> bool {
        matches!(self, Self::SectionHeader(_))
    }

    pub fn is_row(&self) -> bool {
        matches!(self, Self::Row(_))
    }

    pub fn row(&self) -> Option<&DataViewRow> {
        match self {
            Self::Row(row) => Some(row),
            Self::SectionHeader(_) => None,
        }
    }

    pub fn section_header(&self) -> Option<&DataViewSectionHeader> {
        match self {
            Self::SectionHeader(section) => Some(section),
            Self::Row(_) => None,
        }
    }
}

/// A flattened table/list projection with stable row IDs and optional headers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataViewProjection {
    pub entries: Vec<DataViewEntry>,
    pub total_row_count: usize,
}

impl DataViewProjection {
    pub fn new(entries: Vec<DataViewEntry>, total_row_count: usize) -> Self {
        Self {
            entries,
            total_row_count,
        }
    }

    pub fn from_rows(rows: impl IntoIterator<Item = DataViewRow>) -> Self {
        let rows = rows.into_iter().collect::<Vec<_>>();
        Self {
            total_row_count: rows.len(),
            entries: rows.into_iter().map(DataViewEntry::Row).collect(),
        }
    }

    pub fn from_sections(
        sections: impl IntoIterator<Item = (DataViewSectionHeader, Vec<DataViewRow>)>,
    ) -> Self {
        let mut entries = Vec::new();
        let mut total_row_count = 0;
        for (mut section, rows) in sections {
            if section.row_count == 0 {
                section.row_count = rows.len();
            }
            total_row_count += section.row_count;
            let collapsed = section.collapsed;
            entries.push(DataViewEntry::SectionHeader(section));
            if !collapsed {
                entries.extend(rows.into_iter().map(DataViewEntry::Row));
            }
        }
        Self {
            entries,
            total_row_count,
        }
    }

    pub fn visible_row_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.is_row()).count()
    }

    pub fn section_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.is_section_header())
            .count()
    }

    pub fn empty_state(&self, query: impl AsRef<str>) -> Option<DataViewEmptyState> {
        DataViewEmptyState::for_counts(self.total_row_count, self.visible_row_count(), query)
    }

    pub fn row_identity(&self) -> DataViewRowIdentity {
        DataViewRowIdentity::new(self.entries.iter().filter_map(|entry| match entry {
            DataViewEntry::Row(row) => Some(row.id.clone()),
            DataViewEntry::SectionHeader(_) => None,
        }))
    }

    pub fn row_at_visible_index(&self, visible_row_index: usize) -> Option<&DataViewRow> {
        self.entries
            .iter()
            .filter_map(DataViewEntry::row)
            .nth(visible_row_index)
    }

    pub fn row_index_for_id(&self, id: &str) -> Option<usize> {
        self.entries
            .iter()
            .filter_map(DataViewEntry::row)
            .position(|row| row.id == id)
    }

    pub fn source_index_for_id(&self, id: &str) -> Option<usize> {
        self.entries
            .iter()
            .filter_map(DataViewEntry::row)
            .find(|row| row.id == id)
            .map(|row| row.source_index)
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
    pub leading_image: Option<ImageContent>,
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
            leading_image: None,
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

    pub fn with_leading_image(mut self, image: ImageContent) -> Self {
        self.leading_image = Some(image);
        self
    }

    pub fn resolved_width(&self) -> f32 {
        self.width.max(self.min_width)
    }
}

/// Sticky header/leading-column contract for renderer-specific table layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DataTableStickySpec {
    pub header: bool,
    pub leading_columns: usize,
}

impl DataTableStickySpec {
    pub const NONE: Self = Self {
        header: false,
        leading_columns: 0,
    };

    pub const HEADER: Self = Self {
        header: true,
        leading_columns: 0,
    };

    pub const fn new() -> Self {
        Self::NONE
    }

    pub const fn header() -> Self {
        Self::HEADER
    }

    pub const fn leading_columns(leading_columns: usize) -> Self {
        Self {
            header: false,
            leading_columns,
        }
    }

    pub const fn with_header(mut self, header: bool) -> Self {
        self.header = header;
        self
    }

    pub const fn with_leading_columns(mut self, leading_columns: usize) -> Self {
        self.leading_columns = leading_columns;
        self
    }

    pub fn clamped(self, column_count: usize) -> Self {
        Self {
            header: self.header,
            leading_columns: self.leading_columns.min(column_count),
        }
    }

    pub fn has_sticky_columns(self, column_count: usize) -> bool {
        self.clamped(column_count).leading_columns > 0
    }

    pub fn column_region(
        self,
        column_index: usize,
        column_count: usize,
    ) -> Option<DataTableColumnRegion> {
        if column_index >= column_count {
            return None;
        }
        if column_index < self.clamped(column_count).leading_columns {
            Some(DataTableColumnRegion::StickyLeading)
        } else {
            Some(DataTableColumnRegion::Scrollable)
        }
    }

    pub fn partition(self, columns: &[DataTableColumn]) -> DataTableStickyColumns {
        let clamped = self.clamped(columns.len());
        let leading_columns = 0..clamped.leading_columns;
        let scrollable_columns = clamped.leading_columns..columns.len();
        let leading_width = data_table_width(&columns[leading_columns.clone()]);
        let scrollable_width = data_table_width(&columns[scrollable_columns.clone()]);

        DataTableStickyColumns {
            header: clamped.header,
            leading_columns,
            scrollable_columns,
            leading_width,
            scrollable_width,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTableColumnRegion {
    StickyLeading,
    Scrollable,
}

/// Resolved sticky column partition with widths ready for layout/renderers.
#[derive(Debug, Clone, PartialEq)]
pub struct DataTableStickyColumns {
    pub header: bool,
    pub leading_columns: Range<usize>,
    pub scrollable_columns: Range<usize>,
    pub leading_width: f32,
    pub scrollable_width: f32,
}

impl DataTableStickyColumns {
    pub fn has_sticky_columns(&self) -> bool {
        !self.leading_columns.is_empty()
    }

    pub fn total_width(&self) -> f32 {
        self.leading_width + self.scrollable_width
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

    pub fn selected_rows_clamped(&self, row_count: usize) -> Vec<usize> {
        sorted_unique_indices(self.selected_rows.iter().copied(), row_count)
    }

    pub fn set_active_cell_clamped(
        &mut self,
        row_count: usize,
        column_count: usize,
        cell: DataTableCellIndex,
    ) -> Option<DataTableCellIndex> {
        if row_count == 0 || column_count == 0 {
            self.active_cell = None;
            return None;
        }

        let cell = DataTableCellIndex::new(
            cell.row.min(row_count - 1),
            cell.column.min(column_count - 1),
        );
        self.active_cell = Some(cell);
        self.selected_rows = vec![cell.row];
        Some(cell)
    }

    pub fn move_active_cell_by(
        &mut self,
        row_count: usize,
        column_count: usize,
        row_delta: isize,
        column_delta: isize,
    ) -> Option<DataTableCellIndex> {
        if row_count == 0 || column_count == 0 {
            self.active_cell = None;
            return None;
        }

        let base = self.active_cell.unwrap_or_else(|| {
            DataTableCellIndex::new(self.selected_rows.first().copied().unwrap_or(0), 0)
        });
        self.set_active_cell_clamped(
            row_count,
            column_count,
            DataTableCellIndex::new(
                clamp_index_delta(base.row, row_delta, row_count),
                clamp_index_delta(base.column, column_delta, column_count),
            ),
        )
    }
}

/// Stable visible-row ID list used to remap index-based table selection.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataViewRowIdentity {
    pub row_ids: Vec<String>,
}

/// Table-oriented alias for [`DataViewRowIdentity`].
pub type DataTableRowIdentity = DataViewRowIdentity;

impl DataViewRowIdentity {
    pub fn new(row_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            row_ids: row_ids.into_iter().map(Into::into).collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.row_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.row_ids.is_empty()
    }

    pub fn id_at(&self, row_index: usize) -> Option<&str> {
        self.row_ids.get(row_index).map(String::as_str)
    }

    pub fn index_of(&self, row_id: &str) -> Option<usize> {
        self.row_ids.iter().position(|id| id == row_id)
    }

    pub fn contains_id(&self, row_id: &str) -> bool {
        self.index_of(row_id).is_some()
    }

    pub fn duplicate_ids(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut duplicate_ids = Vec::new();
        for id in &self.row_ids {
            if !seen.insert(id.as_str()) && !duplicate_ids.iter().any(|duplicate| duplicate == id) {
                duplicate_ids.push(id.clone());
            }
        }
        duplicate_ids
    }

    pub fn has_unique_ids(&self) -> bool {
        self.duplicate_ids().is_empty()
    }

    pub fn selected_row_ids(&self, selection: &DataTableSelection) -> Vec<String> {
        selection
            .selected_rows_clamped(self.len())
            .into_iter()
            .filter_map(|row| self.id_at(row).map(str::to_owned))
            .collect()
    }

    pub fn active_row_id<'a>(&'a self, selection: &DataTableSelection) -> Option<&'a str> {
        self.id_at(selection.active_cell?.row)
    }

    pub fn selection_from_row_ids(
        &self,
        row_ids: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> DataTableSelection {
        let rows = row_ids
            .into_iter()
            .filter_map(|id| self.index_of(id.as_ref()))
            .collect::<Vec<_>>();
        DataTableSelection {
            selected_rows: sorted_unique_indices(rows, self.len()),
            active_cell: None,
        }
    }

    pub fn selection_from_row_ids_with_active_cell(
        &self,
        row_ids: impl IntoIterator<Item = impl AsRef<str>>,
        active_row_id: Option<&str>,
        active_column: usize,
    ) -> DataTableSelection {
        let mut selection = self.selection_from_row_ids(row_ids);
        selection.active_cell = active_row_id
            .and_then(|row_id| self.index_of(row_id))
            .map(|row| DataTableCellIndex::new(row, active_column));
        selection
    }

    pub fn remap_selection_from(
        &self,
        previous: &DataViewRowIdentity,
        selection: &DataTableSelection,
    ) -> DataTableSelection {
        let selected_ids = previous.selected_row_ids(selection);
        let active_row_id = previous.active_row_id(selection);
        let active_column = selection.active_cell.map(|cell| cell.column).unwrap_or(0);
        self.selection_from_row_ids_with_active_cell(selected_ids, active_row_id, active_column)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTableExportFormat {
    Tsv,
    Csv,
}

impl DataTableExportFormat {
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Tsv => "text/tab-separated-values",
            Self::Csv => "text/csv",
        }
    }

    pub const fn file_extension(self) -> &'static str {
        match self {
            Self::Tsv => "tsv",
            Self::Csv => "csv",
        }
    }
}

impl Default for DataTableExportFormat {
    fn default() -> Self {
        Self::Tsv
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataTableExportScope {
    AllRows,
    VisibleRows(Range<usize>),
    SelectedRows,
    ActiveCell,
    Rows(Vec<usize>),
    CellRange {
        rows: Range<usize>,
        columns: Range<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableExportOptions {
    pub format: DataTableExportFormat,
    pub scope: DataTableExportScope,
    pub include_headers: bool,
    pub line_ending: String,
}

impl DataTableExportOptions {
    pub fn new(scope: DataTableExportScope) -> Self {
        Self {
            scope,
            ..Default::default()
        }
    }

    pub fn format(mut self, format: DataTableExportFormat) -> Self {
        self.format = format;
        self
    }

    pub fn include_headers(mut self, include_headers: bool) -> Self {
        self.include_headers = include_headers;
        self
    }

    pub fn line_ending(mut self, line_ending: impl Into<String>) -> Self {
        self.line_ending = line_ending.into();
        self
    }
}

impl Default for DataTableExportOptions {
    fn default() -> Self {
        Self {
            format: DataTableExportFormat::Tsv,
            scope: DataTableExportScope::SelectedRows,
            include_headers: true,
            line_ending: "\n".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableExport {
    pub text: String,
    pub format: DataTableExportFormat,
    pub row_count: usize,
    pub column_count: usize,
}

impl DataTableExport {
    pub fn mime_type(&self) -> &'static str {
        self.format.mime_type()
    }

    pub fn file_extension(&self) -> &'static str {
        self.format.file_extension()
    }

    pub fn clipboard_request(&self) -> ClipboardRequest {
        ClipboardRequest::WriteText(self.text.clone())
    }

    pub fn clipboard_effect(&self) -> CommandEffect {
        CommandEffect::clipboard(self.clipboard_request())
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
        if self.row_count == 0
            || !self.row_height.is_finite()
            || self.row_height <= f32::EPSILON
            || !self.viewport_height.is_finite()
            || self.viewport_height <= f32::EPSILON
        {
            return 0..0;
        }
        let first = (self.clamped_scroll_offset(0.0).y / self.row_height).floor() as usize;
        let visible = (self.viewport_height.max(0.0) / self.row_height).ceil() as usize + 1;
        let start = first.saturating_sub(self.overscan_rows).min(self.row_count);
        let end = first
            .saturating_add(visible)
            .saturating_add(self.overscan_rows)
            .min(self.row_count);
        start..end
    }

    pub fn total_height(self) -> f32 {
        if !self.row_height.is_finite() {
            return 0.0;
        }
        self.row_count as f32 * self.row_height.max(0.0)
    }

    pub fn clamped_scroll_offset(self, content_width: f32) -> UiPoint {
        let viewport_width = finite_nonnegative(self.viewport_width);
        let viewport_height = finite_nonnegative(self.viewport_height);
        let content_width = finite_nonnegative(content_width);
        let total_height = finite_nonnegative(self.total_height());
        let max_x = (content_width - viewport_width).max(0.0);
        let max_y = (total_height - viewport_height).max(0.0);

        UiPoint::new(
            finite_nonnegative(self.scroll_offset.x).min(max_x),
            finite_nonnegative(self.scroll_offset.y).min(max_y),
        )
    }

    pub fn row_at_viewport_y(self, y: f32) -> Option<usize> {
        if self.row_count == 0
            || !self.row_height.is_finite()
            || self.row_height <= f32::EPSILON
            || !self.viewport_height.is_finite()
            || self.viewport_height <= f32::EPSILON
            || !y.is_finite()
            || y < 0.0
            || y >= self.viewport_height
        {
            return None;
        }
        let row = ((self.clamped_scroll_offset(0.0).y + y) / self.row_height).floor() as usize;
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
    pub selected_row_shader: Option<ShaderEffect>,
    pub active_cell_shader: Option<ShaderEffect>,
    pub header_text_style: TextStyle,
    pub cell_text_style: TextStyle,
    pub leading_image_size: f32,
    pub accessibility_label: Option<String>,
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
            selected_row_shader: None,
            active_cell_shader: None,
            header_text_style: muted_text_style(),
            cell_text_style: TextStyle::default(),
            leading_image_size: 16.0,
            accessibility_label: None,
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
    let scroll_offset = spec.clamped_scroll_offset(table_width);
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
        .with_visual(options.background_visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Grid)
                .label(accessibility_label_or_name(
                    &options.accessibility_label,
                    &name,
                ))
                .value(format!(
                    "{} rows; {} columns",
                    spec.row_count,
                    columns.len()
                ))
                .focusable(),
        ),
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
        scroll.offset = scroll_offset;
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
        let selected = options.selection.contains_row(row);
        let visual = if selected {
            options.selected_row_visual
        } else {
            options.row_visual
        };
        let row_node = with_optional_shader(
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
            .with_visual(visual)
            .with_accessibility(data_table_row_accessibility(row, spec.row_count, selected)),
            selected
                .then_some(())
                .and(options.selected_row_shader.as_ref()),
        );
        let row_node = document.add_child(body, row_node);

        for (column_index, column) in columns.iter().enumerate() {
            let cell_index = DataTableCellIndex::new(row, column_index);
            let active = options.selection.is_active_cell(cell_index);
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
            .with_input(InputBehavior::BUTTON)
            .with_accessibility(data_table_cell_accessibility(
                cell_index,
                spec.row_count,
                columns,
                active,
            ));

            if active {
                cell = cell.with_visual(options.active_cell_visual);
                cell = with_optional_shader(cell, options.active_cell_shader.as_ref());
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
    if !spec.viewport_width.is_finite()
        || spec.viewport_width <= f32::EPSILON
        || !point.x.is_finite()
        || point.x < 0.0
        || point.x >= spec.viewport_width
    {
        return None;
    }
    let row = spec.row_at_viewport_y(point.y)?;
    let column = data_table_column_at_x(
        columns,
        spec.clamped_scroll_offset(data_table_width(columns)).x + point.x,
    )?;
    Some(DataTableCellIndex::new(row, column))
}

pub fn export_data_table_text(
    columns: &[DataTableColumn],
    row_count: usize,
    selection: &DataTableSelection,
    options: DataTableExportOptions,
    mut cell_text: impl FnMut(DataTableCellIndex) -> String,
) -> DataTableExport {
    let selected = data_table_export_indices(columns, row_count, selection, &options.scope);
    let rows = selected.rows;
    let column_indices = selected.columns;
    let mut lines = Vec::new();

    if options.include_headers && !column_indices.is_empty() {
        lines.push(format_data_table_row(
            options.format,
            column_indices
                .iter()
                .filter_map(|column| columns.get(*column).map(|column| column.label.as_str())),
        ));
    }

    for row in &rows {
        lines.push(format_data_table_row(
            options.format,
            column_indices
                .iter()
                .map(|column| cell_text(DataTableCellIndex::new(*row, *column))),
        ));
    }

    DataTableExport {
        text: lines.join(&options.line_ending),
        format: options.format,
        row_count: rows.len(),
        column_count: column_indices.len(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DataTableExportIndices {
    rows: Vec<usize>,
    columns: Vec<usize>,
}

fn data_table_export_indices(
    columns: &[DataTableColumn],
    row_count: usize,
    selection: &DataTableSelection,
    scope: &DataTableExportScope,
) -> DataTableExportIndices {
    let column_count = columns.len();
    let all_columns = (0..column_count).collect::<Vec<_>>();
    match scope {
        DataTableExportScope::AllRows => DataTableExportIndices {
            rows: (0..row_count).collect(),
            columns: all_columns,
        },
        DataTableExportScope::VisibleRows(rows) => DataTableExportIndices {
            rows: clamp_range_to_indices(rows.clone(), row_count),
            columns: all_columns,
        },
        DataTableExportScope::SelectedRows => {
            let mut rows = selection.selected_rows_clamped(row_count);
            if rows.is_empty() {
                rows = selection
                    .active_cell
                    .map(|cell| vec![cell.row])
                    .unwrap_or_default();
                rows = sorted_unique_indices(rows, row_count);
            }
            DataTableExportIndices {
                rows,
                columns: all_columns,
            }
        }
        DataTableExportScope::ActiveCell => {
            let (rows, columns) = selection
                .active_cell
                .filter(|cell| cell.row < row_count && cell.column < column_count)
                .map(|cell| (vec![cell.row], vec![cell.column]))
                .unwrap_or_default();
            DataTableExportIndices { rows, columns }
        }
        DataTableExportScope::Rows(rows) => DataTableExportIndices {
            rows: sorted_unique_indices(rows.iter().copied(), row_count),
            columns: all_columns,
        },
        DataTableExportScope::CellRange { rows, columns } => DataTableExportIndices {
            rows: clamp_range_to_indices(rows.clone(), row_count),
            columns: clamp_range_to_indices(columns.clone(), column_count),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeItem {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeItem>,
    pub disabled: bool,
    pub leading_image: Option<ImageContent>,
    pub row_actions: Vec<TreeRowAction>,
    pub context_menu_commands: Vec<CommandId>,
    pub draggable: bool,
    pub drop_policy: Option<TreeItemDropPolicy>,
}

impl TreeItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            disabled: false,
            leading_image: None,
            row_actions: Vec::new(),
            context_menu_commands: Vec::new(),
            draggable: false,
            drop_policy: None,
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

    pub fn with_leading_image(mut self, image: ImageContent) -> Self {
        self.leading_image = Some(image);
        self
    }

    pub fn with_row_action(mut self, action: TreeRowAction) -> Self {
        self.row_actions.push(action);
        self
    }

    pub fn with_row_actions(mut self, actions: impl IntoIterator<Item = TreeRowAction>) -> Self {
        self.row_actions.extend(actions);
        self
    }

    pub fn with_context_menu_command(mut self, command: impl Into<CommandId>) -> Self {
        self.context_menu_commands.push(command.into());
        self
    }

    pub fn with_context_menu_commands(
        mut self,
        commands: impl IntoIterator<Item = impl Into<CommandId>>,
    ) -> Self {
        self.context_menu_commands
            .extend(commands.into_iter().map(Into::into));
        self
    }

    pub const fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    pub fn with_drop_policy(mut self, policy: TreeItemDropPolicy) -> Self {
        self.drop_policy = Some(policy);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRowAction {
    pub id: CommandId,
    pub label: String,
    pub disabled: bool,
    pub destructive: bool,
    pub leading_image: Option<ImageContent>,
}

impl TreeRowAction {
    pub fn new(id: impl Into<CommandId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            disabled: false,
            destructive: false,
            leading_image: None,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    pub fn with_leading_image(mut self, image: ImageContent) -> Self {
        self.leading_image = Some(image);
        self
    }

    pub fn accessibility_action(&self) -> AccessibilityAction {
        AccessibilityAction::new(self.id.as_str(), self.label.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TreeDropPlacement {
    Before,
    On,
    Inside,
    After,
}

impl TreeDropPlacement {
    pub const ALL: [Self; 4] = [Self::Before, Self::On, Self::Inside, Self::After];

    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::On => "on",
            Self::Inside => "inside",
            Self::After => "after",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::On => "on",
            Self::Inside => "inside",
            Self::After => "after",
        }
    }

    pub fn bounds(self, row_bounds: UiRect) -> UiRect {
        let edge_height = (row_bounds.height * 0.25).max(1.0).min(row_bounds.height);
        match self {
            Self::Before => UiRect::new(row_bounds.x, row_bounds.y, row_bounds.width, edge_height),
            Self::After => UiRect::new(
                row_bounds.x,
                row_bounds.bottom() - edge_height,
                row_bounds.width,
                edge_height,
            ),
            Self::On | Self::Inside => row_bounds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeItemDropPolicy {
    pub accepted_payload: DropPayloadFilter,
    pub accepted_operations: Vec<DragOperation>,
    pub placements: Vec<TreeDropPlacement>,
    pub disabled: bool,
}

impl TreeItemDropPolicy {
    pub fn new(accepted_payload: DropPayloadFilter) -> Self {
        Self {
            accepted_payload,
            accepted_operations: vec![
                DragOperation::Copy,
                DragOperation::Move,
                DragOperation::Link,
            ],
            placements: vec![TreeDropPlacement::On],
            disabled: false,
        }
    }

    pub fn any_payload() -> Self {
        Self::new(DropPayloadFilter::any())
    }

    pub fn accepted_operations(
        mut self,
        operations: impl IntoIterator<Item = DragOperation>,
    ) -> Self {
        self.accepted_operations = operations.into_iter().collect();
        self
    }

    pub fn placements(mut self, placements: impl IntoIterator<Item = TreeDropPlacement>) -> Self {
        self.placements = placements.into_iter().collect();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn allows_placement(&self, placement: TreeDropPlacement) -> bool {
        !self.disabled
            && self.placements.contains(&placement)
            && !self.accepted_operations.is_empty()
            && !self.accepted_payload.is_empty()
    }

    pub fn enabled(&self) -> bool {
        !self.disabled
            && self
                .placements
                .iter()
                .any(|placement| self.allows_placement(*placement))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeVisibleItem {
    pub index: usize,
    pub id: String,
    pub label: String,
    pub leading_image: Option<ImageContent>,
    pub depth: usize,
    pub parent_id: Option<String>,
    pub child_count: usize,
    pub expanded: bool,
    pub disabled: bool,
    pub row_actions: Vec<TreeRowAction>,
    pub context_menu_commands: Vec<CommandId>,
    pub draggable: bool,
    pub drop_policy: Option<TreeItemDropPolicy>,
}

impl TreeVisibleItem {
    pub fn has_children(&self) -> bool {
        self.child_count > 0
    }

    pub fn enabled_row_actions(&self) -> Vec<&TreeRowAction> {
        self.row_actions
            .iter()
            .filter(|action| !action.disabled)
            .collect()
    }

    pub fn has_context_menu(&self) -> bool {
        !self.context_menu_commands.is_empty()
    }

    pub fn drag_source(
        &self,
        bounds: UiRect,
        payload: DragPayload,
        allowed_operations: impl IntoIterator<Item = DragOperation>,
    ) -> Option<DragSourceDescriptor> {
        (!self.disabled && self.draggable).then(|| {
            DragSourceDescriptor::new(
                DragSourceId::new(format!("tree.item.{}", self.id)),
                DragDropSurfaceKind::TreeItem,
                bounds,
                payload,
            )
            .allowed_operations(allowed_operations)
            .label(self.label.clone())
        })
    }

    pub fn drop_target(
        &self,
        bounds: UiRect,
        placement: TreeDropPlacement,
    ) -> Option<DropTargetDescriptor> {
        let policy = self.drop_policy.as_ref()?;
        policy.allows_placement(placement).then(|| {
            DropTargetDescriptor::new(
                DropTargetId::new(format!("tree.item.{}.{}", self.id, placement.suffix())),
                DragDropSurfaceKind::TreeItem,
                placement.bounds(bounds),
            )
            .accepted_payload(policy.accepted_payload.clone())
            .accepted_operations(policy.accepted_operations.clone())
            .label(format!("{} {}", self.label, placement.label()))
        })
    }

    pub fn drop_targets(&self, bounds: UiRect) -> Vec<DropTargetDescriptor> {
        self.drop_policy
            .as_ref()
            .map(|policy| {
                policy
                    .placements
                    .iter()
                    .filter_map(|placement| self.drop_target(bounds, *placement))
                    .collect()
            })
            .unwrap_or_default()
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

    pub fn select_next_visible(&mut self, roots: &[TreeItem]) -> Option<usize> {
        let visible = self.visible_items(roots);
        let current = self.selected_index;
        let index = next_enabled_visible_index(&visible, current)?;
        self.select(Some(index));
        Some(index)
    }

    pub fn select_previous_visible(&mut self, roots: &[TreeItem]) -> Option<usize> {
        let visible = self.visible_items(roots);
        let current = self.selected_index;
        let index = previous_enabled_visible_index(&visible, current)?;
        self.select(Some(index));
        Some(index)
    }

    pub fn toggle_selected_expansion(&mut self, roots: &[TreeItem]) -> Option<bool> {
        let selected = self.selected_visible_item(roots)?;
        selected
            .has_children()
            .then(|| self.toggle_expanded(selected.id))
    }
}

#[derive(Debug, Clone)]
pub struct TreeViewOptions {
    pub layout: Style,
    pub row_height: f32,
    pub indent_width: f32,
    pub disclosure_width: f32,
    pub focused_index: Option<usize>,
    pub background_visual: UiVisual,
    pub row_visual: UiVisual,
    pub selected_row_visual: UiVisual,
    pub selected_row_shader: Option<ShaderEffect>,
    pub focused_row_shader: Option<ShaderEffect>,
    pub text_style: TextStyle,
    pub muted_text_style: TextStyle,
    pub leading_image_size: f32,
    pub accessibility_label: Option<String>,
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
            focused_index: None,
            background_visual: UiVisual::panel(
                ColorRgba::new(18, 22, 28, 255),
                Some(StrokeStyle::new(ColorRgba::new(58, 69, 84, 255), 1.0)),
                4.0,
            ),
            row_visual: UiVisual::TRANSPARENT,
            selected_row_visual: UiVisual::panel(ColorRgba::new(41, 59, 82, 255), None, 0.0),
            selected_row_shader: None,
            focused_row_shader: None,
            text_style: TextStyle::default(),
            muted_text_style: muted_text_style(),
            leading_image_size: 16.0,
            accessibility_label: None,
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
        .with_visual(options.background_visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Tree)
                .label(accessibility_label_or_name(
                    &options.accessibility_label,
                    &name,
                ))
                .value(format!(
                    "{} visible items",
                    state.visible_items(roots).len()
                ))
                .focusable(),
        ),
    );

    let visible_items = state.visible_items(roots);
    let visible_count = visible_items.len();
    for item in visible_items {
        let selected = state.selected_index == Some(item.index);
        let focused = options.focused_index == Some(item.index);
        let visual = if selected {
            options.selected_row_visual
        } else {
            options.row_visual
        };
        let row = with_optional_shader(
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
            .with_visual(visual)
            .with_accessibility(tree_item_accessibility(
                &item,
                visible_count,
                selected,
                focused,
            )),
            if selected {
                options.selected_row_shader.as_ref()
            } else if focused {
                options.focused_row_shader.as_ref()
            } else {
                None
            },
        );
        let row = document.add_child(root, row);

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

        if let Some(image) = item.leading_image.clone() {
            document.add_child(
                row,
                leading_image_node(
                    format!("{name}.row.{}.image", item.id),
                    image,
                    options.leading_image_size,
                    Some(item.label.clone()),
                ),
            );
        }

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
    pub leading_image: Option<ImageContent>,
}

impl TabItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            disabled: false,
            closable: false,
            dirty: false,
            leading_image: None,
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

    pub fn with_leading_image(mut self, image: ImageContent) -> Self {
        self.leading_image = Some(image);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TabGroupState {
    pub selected_index: Option<usize>,
    pub focused_index: Option<usize>,
}

impl TabGroupState {
    pub const fn selected(selected_index: usize) -> Self {
        Self {
            selected_index: Some(selected_index),
            focused_index: Some(selected_index),
        }
    }

    pub fn clamped_selected_index(self, tabs: &[TabItem]) -> Option<usize> {
        let selected = self.selected_index?;
        (selected < tabs.len()).then_some(selected)
    }

    pub fn selected_tab(self, tabs: &[TabItem]) -> Option<&TabItem> {
        tabs.get(self.clamped_selected_index(tabs)?)
    }

    pub fn selected_tab_id(self, tabs: &[TabItem]) -> Option<&str> {
        Some(self.selected_tab(tabs)?.id.as_str())
    }

    pub fn clamped_focused_index(self, tabs: &[TabItem]) -> Option<usize> {
        let focused = self.focused_index?;
        (focused < tabs.len()).then_some(focused)
    }

    pub fn focus_next(&mut self, tabs: &[TabItem]) -> Option<usize> {
        let index = next_enabled_tab_index(tabs, self.focused_index.or(self.selected_index))?;
        self.focused_index = Some(index);
        Some(index)
    }

    pub fn focus_previous(&mut self, tabs: &[TabItem]) -> Option<usize> {
        let index = previous_enabled_tab_index(tabs, self.focused_index.or(self.selected_index))?;
        self.focused_index = Some(index);
        Some(index)
    }

    pub fn select_focused(&mut self, tabs: &[TabItem]) -> Option<usize> {
        let focused = self.clamped_focused_index(tabs)?;
        if tabs[focused].disabled {
            return None;
        }
        self.selected_index = Some(focused);
        Some(focused)
    }

    pub fn select_next(&mut self, tabs: &[TabItem]) -> Option<usize> {
        if tabs.is_empty() {
            self.selected_index = None;
            self.focused_index = None;
            return None;
        }
        let index = self.focus_next(tabs)?;
        self.selected_index = Some(index);
        Some(index)
    }

    pub fn select_previous(&mut self, tabs: &[TabItem]) -> Option<usize> {
        if tabs.is_empty() {
            self.selected_index = None;
            self.focused_index = None;
            return None;
        }
        let index = self.focus_previous(tabs)?;
        self.selected_index = Some(index);
        Some(index)
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
    pub selected_tab_shader: Option<ShaderEffect>,
    pub focused_tab_shader: Option<ShaderEffect>,
    pub panel_shader: Option<ShaderEffect>,
    pub text_style: TextStyle,
    pub muted_text_style: TextStyle,
    pub leading_image_size: f32,
    pub accessibility_label: Option<String>,
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
            selected_tab_shader: None,
            focused_tab_shader: None,
            panel_shader: None,
            text_style: TextStyle::default(),
            muted_text_style: muted_text_style(),
            leading_image_size: 16.0,
            accessibility_label: None,
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
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::TabList)
                .label(accessibility_label_or_name(
                    &options.accessibility_label,
                    &name,
                ))
                .value(format!("{} tabs", tabs.len()))
                .focusable(),
        ),
    );

    let selected_index = state.clamped_selected_index(tabs);
    let focused_index = state.clamped_focused_index(tabs);
    for (index, tab) in tabs.iter().enumerate() {
        let selected = selected_index == Some(index);
        let focused = focused_index == Some(index);
        let style = if tab.disabled {
            options.muted_text_style.clone()
        } else {
            options.text_style.clone()
        };
        let tab_node = with_optional_shader(
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
            })
            .with_accessibility(tab_accessibility(
                tab,
                index,
                tabs.len(),
                selected,
                focused,
            )),
            if selected {
                options.selected_tab_shader.as_ref()
            } else if focused {
                options.focused_tab_shader.as_ref()
            } else {
                None
            },
        );
        let tab_node = document.add_child(strip, tab_node);

        let label = if tab.dirty {
            format!("{} *", tab.label)
        } else {
            tab.label.clone()
        };
        if let Some(image) = tab.leading_image.clone() {
            document.add_child(
                tab_node,
                leading_image_node(
                    format!("{name}.tab.{}.image", tab.id),
                    image,
                    options.leading_image_size,
                    Some(tab.label.clone()),
                ),
            );
        }
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
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label(format!("Close {}", tab.label))
                        .focusable(),
                ),
            );
        }
    }

    let panel = with_optional_shader(
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
        .with_visual(options.panel_visual)
        .with_accessibility(tab_panel_accessibility(tabs, selected_index, &name)),
        options.panel_shader.as_ref(),
    );
    let panel = document.add_child(root, panel);

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
        .with_visual(options.header_visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::ListItem)
                .label("Column headers")
                .value(format!("{} columns", columns.len())),
        ),
    );

    for (column_index, column) in columns.iter().enumerate() {
        let cell = document.add_child(
            header,
            UiNode::container(
                format!("{name}.{}", column.id),
                UiNodeStyle {
                    layout: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
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
            .with_accessibility(data_table_header_accessibility(
                column,
                column_index,
                columns.len(),
            )),
        );
        if let Some(image) = column.leading_image.clone() {
            document.add_child(
                cell,
                leading_image_node(
                    format!("{name}.{}.image", column.id),
                    image,
                    options.leading_image_size,
                    Some(column.label.clone()),
                ),
            );
        }
        document.add_child(
            cell,
            UiNode::text(
                format!("{name}.{}.label", column.id),
                &column.label,
                options.header_text_style.clone(),
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
            leading_image: item.leading_image.clone(),
            depth,
            parent_id: parent_id.map(str::to_owned),
            child_count: item.children.len(),
            expanded: is_expanded,
            disabled: item.disabled,
            row_actions: item.row_actions.clone(),
            context_menu_commands: item.context_menu_commands.clone(),
            draggable: item.draggable,
            drop_policy: item.drop_policy.clone(),
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

fn property_row_visual(
    row: &PropertyGridRow,
    selected: bool,
    options: &PropertyInspectorOptions,
) -> UiVisual {
    if selected {
        options.selected_row_visual
    } else if row.status.has_visual_status() {
        options.status_row_visual
    } else {
        options.row_visual
    }
}

fn property_row_shader(
    row: &PropertyGridRow,
    selected: bool,
    focused: bool,
    options: &PropertyInspectorOptions,
) -> Option<ShaderEffect> {
    let shader = if selected {
        options.selected_row_shader.as_ref()
    } else if focused {
        options.focused_row_shader.as_ref()
    } else if row.status.has_visual_status() {
        options.status_row_shader.as_ref()
    } else {
        None
    };

    shader.map(|shader| property_status_shader(shader.clone(), &row.status))
}

fn property_status_shader(mut shader: ShaderEffect, status: &PropertyRowStatus) -> ShaderEffect {
    if status.has_visual_status() || status.help.is_some() {
        shader = shader
            .uniform(
                "property_status_invalid",
                status.invalid.is_some() as u8 as f32,
            )
            .uniform("property_status_error", status.error.is_some() as u8 as f32)
            .uniform(
                "property_status_warning",
                status.warning.is_some() as u8 as f32,
            )
            .uniform("property_status_changed", status.changed as u8 as f32)
            .uniform("property_status_pending", status.pending as u8 as f32)
            .uniform("property_status_help", status.help.is_some() as u8 as f32);
    }
    shader
}

fn property_row_accessibility(
    row: &PropertyGridRow,
    index: usize,
    total_rows: usize,
    selected: bool,
    focused: bool,
) -> AccessibilityMeta {
    let mut value = vec![
        format!("row {} of {}", index + 1, total_rows),
        property_value_kind_label(row.value_kind).to_owned(),
        if row.editable {
            "editable"
        } else {
            "read only"
        }
        .to_owned(),
    ];
    push_state(&mut value, "selected", selected);
    push_state(&mut value, "focused", focused);
    push_state(&mut value, "disabled", row.disabled);
    push_property_status_value(&mut value, &row.status);

    let mut meta = AccessibilityMeta::new(AccessibilityRole::ListItem)
        .label(row.label.clone())
        .value(value.join("; "))
        .selected(selected)
        .focusable();
    meta = apply_property_status_accessibility(meta, &row.status);
    apply_enabled(meta, !row.disabled)
}

fn property_value_accessibility(
    row: &PropertyGridRow,
    selected: bool,
    focused: bool,
) -> AccessibilityMeta {
    let mut value = vec![
        row.value.clone(),
        property_value_kind_label(row.value_kind).to_owned(),
    ];
    push_state(&mut value, "selected row", selected);
    push_state(&mut value, "focused row", focused);
    push_state(&mut value, "read only", !row.editable);
    push_state(&mut value, "disabled", row.disabled);
    push_property_status_value(&mut value, &row.status);

    let mut meta = AccessibilityMeta::new(AccessibilityRole::GridCell)
        .label(format!("{} value", row.label))
        .value(value.join("; "))
        .selected(selected);
    if !row.editable {
        meta = meta.read_only();
    }
    if row.editable && !row.disabled {
        meta = meta.focusable();
    }
    meta = apply_property_status_accessibility(meta, &row.status);
    apply_enabled(meta, !row.disabled)
}

fn data_table_header_accessibility(
    column: &DataTableColumn,
    column_index: usize,
    column_count: usize,
) -> AccessibilityMeta {
    AccessibilityMeta::new(AccessibilityRole::ColumnHeader)
        .label(column.label.clone())
        .value(format!(
            "column {} of {}; {}",
            column_index + 1,
            column_count,
            if column.resizable {
                "resizable"
            } else {
                "fixed"
            }
        ))
}

fn data_table_row_accessibility(row: usize, row_count: usize, selected: bool) -> AccessibilityMeta {
    let mut value = vec![format!("row {} of {}", row + 1, row_count)];
    push_state(&mut value, "selected", selected);
    AccessibilityMeta::new(AccessibilityRole::ListItem)
        .label(format!("Row {}", row + 1))
        .value(value.join("; "))
        .selected(selected)
        .focusable()
}

fn data_table_cell_accessibility(
    cell: DataTableCellIndex,
    row_count: usize,
    columns: &[DataTableColumn],
    active: bool,
) -> AccessibilityMeta {
    let column_label = columns
        .get(cell.column)
        .map(|column| column.label.as_str())
        .unwrap_or("Column");
    let mut value = vec![
        format!("row {} of {}", cell.row + 1, row_count),
        format!("column {} of {}", cell.column + 1, columns.len()),
    ];
    push_state(&mut value, "active", active);

    AccessibilityMeta::new(AccessibilityRole::GridCell)
        .label(format!("Row {}, {}", cell.row + 1, column_label))
        .value(value.join("; "))
        .selected(active)
        .focusable()
}

fn tree_item_accessibility(
    item: &TreeVisibleItem,
    visible_count: usize,
    selected: bool,
    focused: bool,
) -> AccessibilityMeta {
    let mut value = vec![
        format!("item {} of {}", item.index + 1, visible_count),
        format!("level {}", item.depth + 1),
        if item.has_children() {
            format!(
                "{}; {} children",
                if item.expanded {
                    "expanded"
                } else {
                    "collapsed"
                },
                item.child_count
            )
        } else {
            "leaf".to_owned()
        },
    ];
    push_state(&mut value, "selected", selected);
    push_state(&mut value, "focused", focused);
    push_state(&mut value, "disabled", item.disabled);
    push_state(&mut value, "draggable", item.draggable);
    push_state(
        &mut value,
        "drop target",
        item.drop_policy
            .as_ref()
            .is_some_and(TreeItemDropPolicy::enabled),
    );
    if !item.row_actions.is_empty() {
        value.push(format!("{} actions", item.enabled_row_actions().len()));
    }
    push_state(&mut value, "context menu", item.has_context_menu());

    let mut meta = AccessibilityMeta::new(AccessibilityRole::TreeItem)
        .label(item.label.clone())
        .value(value.join("; "))
        .selected(selected)
        .expanded(item.expanded)
        .focusable();
    for action in item.enabled_row_actions() {
        meta = meta.action(action.accessibility_action());
    }
    if item.has_context_menu() {
        meta = meta.action(AccessibilityAction::new(
            "context_menu.open",
            "Open context menu",
        ));
    }
    if item.draggable && !item.disabled {
        meta = meta.action(AccessibilityAction::new("drag.start", "Start drag"));
    }
    if item
        .drop_policy
        .as_ref()
        .is_some_and(TreeItemDropPolicy::enabled)
    {
        meta = meta.action(AccessibilityAction::new("drop.accept", "Accept drop"));
    }
    apply_enabled(meta, !item.disabled)
}

fn tab_accessibility(
    tab: &TabItem,
    index: usize,
    tab_count: usize,
    selected: bool,
    focused: bool,
) -> AccessibilityMeta {
    let mut value = vec![format!("tab {} of {}", index + 1, tab_count)];
    push_state(&mut value, "selected", selected);
    push_state(&mut value, "focused", focused);
    push_state(&mut value, "dirty", tab.dirty);
    push_state(&mut value, "closable", tab.closable);
    push_state(&mut value, "disabled", tab.disabled);

    apply_enabled(
        AccessibilityMeta::new(AccessibilityRole::Tab)
            .label(tab.label.clone())
            .value(value.join("; "))
            .selected(selected)
            .focusable(),
        !tab.disabled,
    )
}

fn tab_panel_accessibility(
    tabs: &[TabItem],
    selected_index: Option<usize>,
    group_name: &str,
) -> AccessibilityMeta {
    let selected = selected_index.and_then(|index| tabs.get(index));
    let label = selected
        .map(|tab| format!("{} panel", tab.label))
        .unwrap_or_else(|| format!("{group_name} panel"));
    let value = selected
        .map(|tab| format!("selected tab {}", tab.id))
        .unwrap_or_else(|| "no selected tab".to_owned());
    AccessibilityMeta::new(AccessibilityRole::TabPanel)
        .label(label)
        .value(value)
}

fn leading_image_node(
    name: impl Into<String>,
    image: ImageContent,
    size: f32,
    label: Option<String>,
) -> UiNode {
    let node = UiNode::image(
        name,
        image,
        Style {
            size: TaffySize {
                width: px(size),
                height: px(size),
            },
            margin: taffy::prelude::Rect {
                right: LengthPercentageAuto::length(6.0),
                ..taffy::prelude::Rect::length(0.0)
            },
            flex_shrink: 0.0,
            ..Default::default()
        },
    );
    if let Some(label) = label {
        node.with_accessibility(AccessibilityMeta::new(AccessibilityRole::Image).label(label))
    } else {
        node
    }
}

fn with_optional_shader(mut node: UiNode, shader: Option<&ShaderEffect>) -> UiNode {
    if let Some(shader) = shader {
        node = node.with_shader(shader.clone());
    }
    node
}

fn accessibility_label_or_name(label: &Option<String>, name: &str) -> String {
    label.clone().unwrap_or_else(|| name.to_owned())
}

fn data_view_empty_reason_label(reason: DataViewEmptyReason) -> &'static str {
    match reason {
        DataViewEmptyReason::NoRows => "no rows",
        DataViewEmptyReason::NoMatches => "no matches",
        DataViewEmptyReason::NoVisibleRows => "no visible rows",
    }
}

fn property_value_kind_label(kind: PropertyValueKind) -> &'static str {
    match kind {
        PropertyValueKind::Text => "text",
        PropertyValueKind::Number => "number",
        PropertyValueKind::Boolean => "boolean",
        PropertyValueKind::Choice => "choice",
        PropertyValueKind::Color => "color",
        PropertyValueKind::Custom => "custom",
    }
}

fn push_property_status_value(values: &mut Vec<String>, status: &PropertyRowStatus) {
    push_state(values, "changed", status.changed);
    push_state(values, "pending", status.pending);
    if status.invalid.is_some() {
        values.push("invalid".to_owned());
    }
    if status.error.is_some() {
        values.push("error".to_owned());
    }
    if status.warning.is_some() {
        values.push("warning".to_owned());
    }
    if status.help.is_some() {
        values.push("help available".to_owned());
    }
}

fn apply_property_status_accessibility(
    mut meta: AccessibilityMeta,
    status: &PropertyRowStatus,
) -> AccessibilityMeta {
    if let Some(message) = property_status_invalid_message(status) {
        meta = meta.invalid(message);
    }
    if let Some(hint) = property_status_hint(status) {
        meta = meta.hint(hint);
    }
    if status.error.is_some() {
        meta = meta.live_region(AccessibilityLiveRegion::Assertive);
    } else if status.pending {
        meta = meta.live_region(AccessibilityLiveRegion::Polite);
    }
    meta
}

fn property_status_invalid_message(status: &PropertyRowStatus) -> Option<String> {
    status.error.clone().or_else(|| status.invalid.clone())
}

fn property_status_hint(status: &PropertyRowStatus) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(error) = &status.error {
        parts.push(format!("Error: {error}"));
    }
    if let Some(invalid) = &status.invalid {
        parts.push(format!("Invalid: {invalid}"));
    }
    if let Some(warning) = &status.warning {
        parts.push(format!("Warning: {warning}"));
    }
    if let Some(help) = &status.help {
        parts.push(format!("Help: {help}"));
    }
    if status.pending {
        parts.push("Pending".to_owned());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

fn apply_enabled(meta: AccessibilityMeta, enabled: bool) -> AccessibilityMeta {
    if enabled {
        meta
    } else {
        meta.disabled()
    }
}

fn push_state(values: &mut Vec<String>, label: &str, active: bool) {
    if active {
        values.push(label.to_owned());
    }
}

fn sorted_unique_indices(
    indices: impl IntoIterator<Item = usize>,
    upper_bound: usize,
) -> Vec<usize> {
    let mut indices = indices
        .into_iter()
        .filter(|index| *index < upper_bound)
        .collect::<Vec<_>>();
    indices.sort_unstable();
    indices.dedup();
    indices
}

fn clamp_range_to_indices(range: Range<usize>, upper_bound: usize) -> Vec<usize> {
    let start = range.start.min(upper_bound);
    let end = range.end.min(upper_bound);
    (start..end).collect()
}

fn format_data_table_row(
    format: DataTableExportFormat,
    cells: impl IntoIterator<Item = impl AsRef<str>>,
) -> String {
    let cells = cells
        .into_iter()
        .map(|cell| format_data_table_cell(format, cell.as_ref()))
        .collect::<Vec<_>>();
    match format {
        DataTableExportFormat::Tsv => cells.join("\t"),
        DataTableExportFormat::Csv => cells.join(","),
    }
}

fn format_data_table_cell(format: DataTableExportFormat, text: &str) -> String {
    match format {
        DataTableExportFormat::Tsv => text
            .chars()
            .map(|character| match character {
                '\t' | '\r' | '\n' => ' ',
                other => other,
            })
            .collect(),
        DataTableExportFormat::Csv => {
            if text.contains([',', '"', '\r', '\n']) {
                format!("\"{}\"", text.replace('"', "\"\""))
            } else {
                text.to_owned()
            }
        }
    }
}

fn next_enabled_visible_index(
    visible: &[TreeVisibleItem],
    current: Option<usize>,
) -> Option<usize> {
    let start = current.and_then(|index| index.checked_add(1)).unwrap_or(0);
    visible
        .iter()
        .find(|item| item.index >= start && !item.disabled)
        .or_else(|| visible.iter().rev().find(|item| !item.disabled))
        .map(|item| item.index)
}

fn previous_enabled_visible_index(
    visible: &[TreeVisibleItem],
    current: Option<usize>,
) -> Option<usize> {
    match current {
        Some(current) => visible
            .iter()
            .rev()
            .find(|item| item.index < current && !item.disabled)
            .or_else(|| visible.iter().find(|item| !item.disabled))
            .map(|item| item.index),
        None => visible
            .iter()
            .rev()
            .find(|item| !item.disabled)
            .map(|item| item.index),
    }
}

fn next_enabled_tab_index(tabs: &[TabItem], current: Option<usize>) -> Option<usize> {
    if tabs.is_empty() {
        return None;
    }
    let start = current
        .map(|index| (index.min(tabs.len() - 1) + 1) % tabs.len())
        .unwrap_or(0);
    for offset in 0..tabs.len() {
        let index = (start + offset) % tabs.len();
        if !tabs[index].disabled {
            return Some(index);
        }
    }
    None
}

fn previous_enabled_tab_index(tabs: &[TabItem], current: Option<usize>) -> Option<usize> {
    if tabs.is_empty() {
        return None;
    }
    let start = current
        .map(|index| (index.min(tabs.len() - 1) + tabs.len() - 1) % tabs.len())
        .unwrap_or(tabs.len() - 1);
    for offset in 0..tabs.len() {
        let index = (start + tabs.len() - offset) % tabs.len();
        if !tabs[index].disabled {
            return Some(index);
        }
    }
    None
}

fn clamp_index_delta(index: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    ((index as i128) + (delta as i128)).clamp(0, (len - 1) as i128) as usize
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
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

    fn node_named(doc: &UiDocument, name: &str) -> UiNodeId {
        doc.nodes()
            .iter()
            .position(|node| node.name == name)
            .map(UiNodeId)
            .unwrap_or_else(|| panic!("missing node {name}"))
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
    fn property_inspector_grid_exports_accessibility_images_and_shader_state() {
        let mut doc = test_root();
        let rows = vec![
            PropertyGridRow::new("name", "Name", "Lead")
                .with_leading_image(ImageContent::new("icons.text")),
            PropertyGridRow::new("locked", "Locked", "Yes").disabled(),
        ];
        let root = doc.root;
        let grid = property_inspector_grid(
            &mut doc,
            root,
            "props",
            &rows,
            PropertyInspectorOptions {
                selected_index: Some(0),
                focused_index: Some(0),
                selected_row_shader: Some(ShaderEffect::new("ui.selected")),
                accessibility_label: Some("Inspector".to_owned()),
                ..Default::default()
            },
        );

        assert_eq!(
            doc.node(grid).accessibility.as_ref().unwrap().role,
            AccessibilityRole::Grid
        );
        assert_eq!(
            doc.node(grid)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Inspector")
        );

        let selected_row = doc.node(node_named(&doc, "props.row.name"));
        assert_eq!(selected_row.shader.as_ref().unwrap().key, "ui.selected");
        let row_meta = selected_row.accessibility.as_ref().unwrap();
        assert_eq!(row_meta.role, AccessibilityRole::ListItem);
        assert!(row_meta.value.as_deref().unwrap().contains("selected"));
        assert!(row_meta.value.as_deref().unwrap().contains("focused"));

        let image = doc.node(node_named(&doc, "props.row.name.image"));
        assert!(matches!(&image.content, UiContent::Image(image) if image.key == "icons.text"));

        let disabled_row = doc.node(node_named(&doc, "props.row.locked"));
        assert!(!disabled_row.input.pointer);
        assert!(!disabled_row.accessibility.as_ref().unwrap().enabled);
    }

    #[test]
    fn property_inspector_grid_maps_status_metadata_to_accessibility() {
        let mut doc = test_root();
        let rows = vec![
            PropertyGridRow::new("gain", "Gain", "12 dB")
                .invalid("Out of range")
                .warning("May clip")
                .help("Use a lower gain")
                .changed()
                .pending(),
            PropertyGridRow::new("mode", "Mode", "Auto").error("Unsupported mode"),
        ];
        let root = doc.root;
        property_inspector_grid(
            &mut doc,
            root,
            "props",
            &rows,
            PropertyInspectorOptions::default(),
        );

        let gain_row = doc.node(node_named(&doc, "props.row.gain"));
        let gain_meta = gain_row.accessibility.as_ref().unwrap();
        let gain_value = gain_meta.value.as_deref().unwrap();
        assert!(gain_value.contains("changed"));
        assert!(gain_value.contains("pending"));
        assert!(gain_value.contains("invalid"));
        assert!(gain_value.contains("warning"));
        assert!(gain_value.contains("help available"));
        assert_eq!(gain_meta.invalid.as_deref(), Some("Out of range"));
        assert_eq!(gain_meta.live_region, AccessibilityLiveRegion::Polite);
        assert!(gain_meta
            .hint
            .as_deref()
            .unwrap()
            .contains("Warning: May clip"));

        let gain_value_node = doc.node(node_named(&doc, "props.row.gain.value"));
        let gain_value_meta = gain_value_node.accessibility.as_ref().unwrap();
        assert_eq!(gain_value_meta.invalid.as_deref(), Some("Out of range"));
        assert!(gain_value_meta
            .hint
            .as_deref()
            .unwrap()
            .contains("Help: Use a lower gain"));

        let mode_row = doc.node(node_named(&doc, "props.row.mode"));
        let mode_meta = mode_row.accessibility.as_ref().unwrap();
        assert_eq!(mode_meta.invalid.as_deref(), Some("Unsupported mode"));
        assert_eq!(mode_meta.live_region, AccessibilityLiveRegion::Assertive);
    }

    #[test]
    fn property_inspector_grid_maps_status_to_visual_and_shader_hooks() {
        let mut doc = test_root();
        let rows = vec![
            PropertyGridRow::new("dirty", "Dirty", "Yes").changed(),
            PropertyGridRow::new("warn", "Warning", "High").warning("Near limit"),
        ];
        let status_visual = UiVisual::panel(
            ColorRgba::new(28, 34, 43, 255),
            Some(StrokeStyle::new(ColorRgba::new(214, 158, 46, 255), 1.0)),
            0.0,
        );
        let root = doc.root;
        property_inspector_grid(
            &mut doc,
            root,
            "props",
            &rows,
            PropertyInspectorOptions {
                selected_index: Some(0),
                selected_row_shader: Some(ShaderEffect::new("ui.selected")),
                status_row_visual: status_visual,
                status_row_shader: Some(ShaderEffect::new("ui.status")),
                ..Default::default()
            },
        );

        let selected_row = doc.node(node_named(&doc, "props.row.dirty"));
        assert_eq!(selected_row.visual.fill, ColorRgba::new(43, 62, 86, 255));
        let selected_shader = selected_row.shader.as_ref().unwrap();
        assert_eq!(selected_shader.key, "ui.selected");
        assert!(selected_shader.uniforms.iter().any(|uniform| {
            uniform.name == "property_status_changed" && (uniform.value - 1.0).abs() < f32::EPSILON
        }));

        let warning_row = doc.node(node_named(&doc, "props.row.warn"));
        assert_eq!(warning_row.visual, status_visual);
        let warning_shader = warning_row.shader.as_ref().unwrap();
        assert_eq!(warning_shader.key, "ui.status");
        assert!(warning_shader.uniforms.iter().any(|uniform| {
            uniform.name == "property_status_warning" && (uniform.value - 1.0).abs() < f32::EPSILON
        }));
    }

    #[test]
    fn data_view_empty_state_distinguishes_source_filter_and_view_empty() {
        assert_eq!(
            DataViewEmptyState::for_counts(0, 0, ""),
            Some(DataViewEmptyState::no_rows("No rows"))
        );
        assert_eq!(
            DataViewEmptyState::for_counts(4, 0, ""),
            Some(DataViewEmptyState::no_visible_rows("No visible rows"))
        );
        assert_eq!(DataViewEmptyState::for_counts(4, 2, "lead"), None);

        let filtered = DataViewEmptyState::for_counts(4, 0, "lead")
            .unwrap()
            .message("Adjust the filter")
            .action_label("Clear filter");
        assert!(filtered.is_filter_empty());
        assert!(filtered.accessibility_value().contains("query lead"));
        assert!(filtered.accessibility_value().contains("Adjust the filter"));

        let accessibility = filtered.accessibility();
        assert_eq!(accessibility.role, AccessibilityRole::Status);
        assert_eq!(accessibility.live_region, AccessibilityLiveRegion::Polite);
        assert_eq!(accessibility.label.as_deref(), Some("No matching rows"));
    }

    #[test]
    fn data_view_projection_flattens_section_headers_and_row_identity() {
        let projection = DataViewProjection::from_sections(vec![
            (
                DataViewSectionHeader::new("audio", "Audio").with_row_count(2),
                vec![
                    DataViewRow::new("kick", 3).in_section("audio"),
                    DataViewRow::new("snare", 7).in_section("audio"),
                ],
            ),
            (
                DataViewSectionHeader::new("hidden", "Hidden")
                    .with_row_count(1)
                    .collapsed(),
                vec![DataViewRow::new("ghost", 9).in_section("hidden")],
            ),
        ]);

        assert_eq!(projection.total_row_count, 3);
        assert_eq!(projection.visible_row_count(), 2);
        assert_eq!(projection.section_count(), 2);
        assert_eq!(
            projection
                .entries
                .iter()
                .map(DataViewEntry::id)
                .collect::<Vec<_>>(),
            vec!["audio", "kick", "snare", "hidden"]
        );
        assert_eq!(projection.row_index_for_id("snare"), Some(1));
        assert_eq!(projection.source_index_for_id("snare"), Some(7));
        assert_eq!(
            projection
                .row_at_visible_index(0)
                .unwrap()
                .section_id
                .as_deref(),
            Some("audio")
        );
        assert_eq!(
            projection.row_identity().row_ids,
            vec!["kick".to_owned(), "snare".to_owned()]
        );

        let hidden = projection.entries[3].section_header().unwrap();
        let accessibility = hidden.accessibility(1, 2);
        assert_eq!(accessibility.role, AccessibilityRole::RowHeader);
        assert_eq!(accessibility.expanded, Some(false));
        assert!(accessibility
            .value
            .as_deref()
            .unwrap()
            .contains("collapsed"));
    }

    #[test]
    fn data_table_sticky_spec_partitions_leading_columns() {
        let columns = vec![
            DataTableColumn::new("name", "Name", 120.0),
            DataTableColumn::new("state", "State", 80.0),
            DataTableColumn::new("note", "Note", 40.0).with_min_width(60.0),
        ];

        let spec = DataTableStickySpec::leading_columns(2).with_header(true);
        let partition = spec.partition(&columns);

        assert!(partition.header);
        assert_eq!(partition.leading_columns, 0..2);
        assert_eq!(partition.scrollable_columns, 2..3);
        assert_eq!(partition.leading_width, 200.0);
        assert_eq!(partition.scrollable_width, 60.0);
        assert_eq!(partition.total_width(), data_table_width(&columns));
        assert!(partition.has_sticky_columns());
        assert_eq!(
            spec.column_region(1, columns.len()),
            Some(DataTableColumnRegion::StickyLeading)
        );
        assert_eq!(
            spec.column_region(2, columns.len()),
            Some(DataTableColumnRegion::Scrollable)
        );
        assert_eq!(
            DataTableStickySpec::leading_columns(99)
                .partition(&columns)
                .scrollable_columns,
            3..3
        );
    }

    #[test]
    fn data_view_row_identity_remaps_selection_after_filtering_and_sorting() {
        let previous = DataViewRowIdentity::new(["bravo", "alpha", "charlie", "delta"]);
        let current = DataViewRowIdentity::new(["delta", "charlie", "bravo"]);
        let selection = DataTableSelection {
            selected_rows: vec![2, 0, 2, 99],
            active_cell: Some(DataTableCellIndex::new(2, 4)),
        };

        assert_eq!(
            previous.selected_row_ids(&selection),
            vec!["bravo".to_owned(), "charlie".to_owned()]
        );
        assert_eq!(previous.active_row_id(&selection), Some("charlie"));

        let remapped = current.remap_selection_from(&previous, &selection);
        assert_eq!(remapped.selected_rows, vec![1, 2]);
        assert_eq!(remapped.active_cell, Some(DataTableCellIndex::new(1, 4)));

        let filtered = DataViewRowIdentity::new(["delta"]);
        let remapped = filtered.remap_selection_from(&previous, &selection);
        assert!(remapped.selected_rows.is_empty());
        assert_eq!(remapped.active_cell, None);

        let duplicate = DataViewRowIdentity::new(["one", "two", "one", "two", "one"]);
        assert_eq!(
            duplicate.duplicate_ids(),
            vec!["one".to_owned(), "two".to_owned()]
        );
        assert!(!duplicate.has_unique_ids());
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
    fn virtualized_data_table_clamps_edge_offsets_and_keyboard_moves() {
        let columns = vec![
            DataTableColumn::new("name", "Name", 100.0),
            DataTableColumn::new("value", "Value", 100.0),
        ];
        let spec = VirtualDataTableSpec {
            row_count: 10,
            row_height: 10.0,
            viewport_width: 50.0,
            viewport_height: 30.0,
            scroll_offset: UiPoint::new(999.0, 999.0),
            overscan_rows: 0,
        };

        assert_eq!(
            spec.clamped_scroll_offset(data_table_width(&columns)),
            UiPoint::new(150.0, 70.0)
        );
        assert_eq!(spec.visible_rows(), 7..10);
        assert_eq!(spec.row_at_viewport_y(0.0), Some(7));
        assert_eq!(spec.row_at_viewport_y(29.0), Some(9));
        assert_eq!(
            data_table_cell_at_point(&columns, spec, UiPoint::new(10.0, 0.0)),
            Some(DataTableCellIndex::new(7, 1))
        );
        assert_eq!(
            VirtualDataTableSpec {
                viewport_height: 0.0,
                ..spec
            }
            .visible_rows(),
            0..0
        );

        let mut selection = DataTableSelection::default();
        assert_eq!(
            selection.set_active_cell_clamped(10, 2, DataTableCellIndex::new(100, 10)),
            Some(DataTableCellIndex::new(9, 1))
        );
        assert_eq!(
            selection.move_active_cell_by(10, 2, -20, -20),
            Some(DataTableCellIndex::new(0, 0))
        );
        assert_eq!(selection.selected_rows, vec![0]);
        assert_eq!(selection.move_active_cell_by(0, 2, 1, 0), None);
    }

    #[test]
    fn data_table_export_formats_selected_rows_and_clipboard_effects() {
        let columns = vec![
            DataTableColumn::new("name", "Name", 120.0),
            DataTableColumn::new("value", "Value", 80.0),
        ];
        let selection = DataTableSelection {
            selected_rows: vec![3, 1, 3, 99],
            active_cell: None,
        };
        let export = export_data_table_text(
            &columns,
            4,
            &selection,
            DataTableExportOptions::new(DataTableExportScope::SelectedRows),
            |cell| match cell.column {
                0 => format!("clip\t{}", cell.row),
                _ => format!("bar\n{}", cell.row + 1),
            },
        );

        assert_eq!(selection.selected_rows_clamped(4), vec![1, 3]);
        assert_eq!(export.format, DataTableExportFormat::Tsv);
        assert_eq!(export.mime_type(), "text/tab-separated-values");
        assert_eq!(export.file_extension(), "tsv");
        assert_eq!(export.row_count, 2);
        assert_eq!(export.column_count, 2);
        assert_eq!(export.text, "Name\tValue\nclip 1\tbar 2\nclip 3\tbar 4");
        assert_eq!(
            export.clipboard_request(),
            ClipboardRequest::WriteText(export.text.clone())
        );
        assert_eq!(
            export.clipboard_effect(),
            CommandEffect::clipboard(ClipboardRequest::WriteText(export.text.clone()))
        );
    }

    #[test]
    fn data_table_export_supports_csv_active_cells_and_ranges() {
        let columns = vec![
            DataTableColumn::new("name", "Name", 120.0),
            DataTableColumn::new("note", "Note", 200.0),
        ];
        let selection =
            DataTableSelection::single_row(0).with_active_cell(DataTableCellIndex::new(2, 1));
        let active = export_data_table_text(
            &columns,
            8,
            &selection,
            DataTableExportOptions::new(DataTableExportScope::ActiveCell)
                .format(DataTableExportFormat::Csv)
                .include_headers(false),
            |cell| format!("row {}, \"quoted\"", cell.row),
        );

        assert_eq!(active.text, "\"row 2, \"\"quoted\"\"\"");
        assert_eq!(active.mime_type(), "text/csv");
        assert_eq!(active.row_count, 1);
        assert_eq!(active.column_count, 1);

        let range = export_data_table_text(
            &columns,
            8,
            &selection,
            DataTableExportOptions::new(DataTableExportScope::CellRange {
                rows: 1..3,
                columns: 0..2,
            })
            .format(DataTableExportFormat::Csv)
            .line_ending("\r\n"),
            |cell| format!("r{}c{}", cell.row, cell.column),
        );

        assert_eq!(range.text, "Name,Note\r\nr1c0,r1c1\r\nr2c0,r2c1");
        assert_eq!(range.row_count, 2);
        assert_eq!(range.column_count, 2);
    }

    #[test]
    fn virtualized_data_table_builds_header_visible_rows_and_spacers() {
        let mut doc = test_root();
        let root = doc.root;
        let columns = vec![
            DataTableColumn::new("name", "Name", 120.0)
                .with_leading_image(ImageContent::new("icons.name")),
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
                selected_row_shader: Some(ShaderEffect::new("ui.row_selected")),
                active_cell_shader: Some(ShaderEffect::new("ui.cell_active")),
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
        assert!(matches!(
            &doc.node(node_named(&doc, "table.header.name.image")).content,
            UiContent::Image(image) if image.key == "icons.name"
        ));
        assert_eq!(
            doc.node(node_named(&doc, "table.row.10"))
                .shader
                .as_ref()
                .unwrap()
                .key,
            "ui.row_selected"
        );
        let active_cell = doc.node(node_named(&doc, "table.row.10.cell.value"));
        assert_eq!(active_cell.shader.as_ref().unwrap().key, "ui.cell_active");
        assert_eq!(
            active_cell.accessibility.as_ref().unwrap().role,
            AccessibilityRole::GridCell
        );
        assert!(active_cell
            .accessibility
            .as_ref()
            .unwrap()
            .value
            .as_deref()
            .unwrap()
            .contains("active"));

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
    fn tree_view_state_navigates_enabled_visible_items() {
        let roots = vec![TreeItem::new("project", "Project").with_children(vec![
            TreeItem::new("src", "src").disabled(),
            TreeItem::new("readme", "README.md"),
        ])];
        let mut state = TreeViewState::expanded(["project"]);
        state.select(Some(0));

        assert_eq!(state.select_next_visible(&roots), Some(2));
        assert_eq!(state.selected_visible_item(&roots).unwrap().id, "readme");
        assert_eq!(state.select_previous_visible(&roots), Some(0));
        assert_eq!(state.toggle_selected_expansion(&roots), Some(false));
        assert_eq!(
            state
                .visible_items(&roots)
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["project"]
        );
    }

    #[test]
    fn tree_view_builds_rows_with_disclosure_and_selection() {
        let mut doc = test_root();
        let root = doc.root;
        let roots = vec![TreeItem::new("project", "Project")
            .with_leading_image(ImageContent::new("icons.folder"))
            .with_children(vec![TreeItem::new("src", "src")])];
        let mut state = TreeViewState::expanded(["project"]);
        state.select(Some(0));

        let tree = tree_view(
            &mut doc,
            root,
            "tree",
            &roots,
            &state,
            TreeViewOptions {
                selected_row_shader: Some(ShaderEffect::new("ui.tree_selected")),
                ..Default::default()
            },
        );

        assert_eq!(doc.node(tree).children.len(), 2);
        let first_row = doc.node(tree).children[0];
        assert_eq!(
            doc.node(first_row).visual.fill,
            ColorRgba::new(41, 59, 82, 255)
        );
        assert_eq!(
            doc.node(first_row).shader.as_ref().unwrap().key,
            "ui.tree_selected"
        );
        assert_eq!(
            doc.node(tree).accessibility.as_ref().unwrap().role,
            AccessibilityRole::Tree
        );
        assert_eq!(
            doc.node(first_row).accessibility.as_ref().unwrap().role,
            AccessibilityRole::TreeItem
        );
        assert!(doc
            .node(first_row)
            .accessibility
            .as_ref()
            .unwrap()
            .value
            .as_deref()
            .unwrap()
            .contains("expanded"));
        assert!(matches!(
            &doc.node(node_named(&doc, "tree.row.project.image")).content,
            UiContent::Image(image) if image.key == "icons.folder"
        ));
        let disclosure = doc.node(doc.node(first_row).children[0]);
        assert!(matches!(&disclosure.content, UiContent::Text(text) if text.text == "v"));
    }

    #[test]
    fn tree_visible_items_expose_actions_context_and_drag_drop_descriptors() {
        let policy = TreeItemDropPolicy::new(DropPayloadFilter::empty().files())
            .accepted_operations([DragOperation::Move])
            .placements([TreeDropPlacement::Before, TreeDropPlacement::Inside]);
        let roots = vec![TreeItem::new("track", "Track")
            .with_row_actions([
                TreeRowAction::new("rename", "Rename"),
                TreeRowAction::new("remove", "Remove").disabled(),
            ])
            .with_context_menu_commands(["duplicate", "delete"])
            .draggable(true)
            .with_drop_policy(policy.clone())];
        let visible = TreeViewState::default().visible_items(&roots);
        let item = &visible[0];

        assert_eq!(item.enabled_row_actions().len(), 1);
        assert!(item.has_context_menu());
        assert!(item.draggable);
        assert_eq!(item.drop_policy.as_ref(), Some(&policy));

        let bounds = UiRect::new(10.0, 20.0, 100.0, 24.0);
        let source = item
            .drag_source(bounds, DragPayload::text("track"), [DragOperation::Move])
            .expect("drag source");
        assert_eq!(source.id, DragSourceId::new("tree.item.track"));
        assert_eq!(source.kind, DragDropSurfaceKind::TreeItem);
        assert!(source.can_start());

        let targets = item.drop_targets(bounds);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].id, DropTargetId::new("tree.item.track.before"));
        assert_eq!(targets[0].bounds, UiRect::new(10.0, 20.0, 100.0, 6.0));
        assert_eq!(targets[1].id, DropTargetId::new("tree.item.track.inside"));
        assert_eq!(targets[1].bounds, bounds);
        assert_eq!(
            targets[1]
                .resolve_operation(&DragPayload::files(["track.wav"]), &[DragOperation::Move]),
            Some(DragOperation::Move)
        );
    }

    #[test]
    fn tree_view_accessibility_includes_row_actions_context_and_drag_drop() {
        let mut doc = test_root();
        let root = doc.root;
        let roots = vec![TreeItem::new("clip", "Clip")
            .with_row_action(TreeRowAction::new("rename", "Rename"))
            .with_context_menu_command("clip.context")
            .draggable(true)
            .with_drop_policy(TreeItemDropPolicy::any_payload())];

        tree_view(
            &mut doc,
            root,
            "tree",
            &roots,
            &TreeViewState::default(),
            TreeViewOptions::default(),
        );

        let row = node_named(&doc, "tree.row.clip");
        let accessibility = doc.node(row).accessibility.as_ref().unwrap();
        let value = accessibility.value.as_deref().unwrap();
        assert!(value.contains("draggable"));
        assert!(value.contains("drop target"));
        assert!(value.contains("1 actions"));
        assert!(value.contains("context menu"));
        assert!(accessibility
            .actions
            .iter()
            .any(|action| action.id == "rename"));
        assert!(accessibility
            .actions
            .iter()
            .any(|action| action.id == "context_menu.open"));
        assert!(accessibility
            .actions
            .iter()
            .any(|action| action.id == "drag.start"));
        assert!(accessibility
            .actions
            .iter()
            .any(|action| action.id == "drop.accept"));
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

        let mut focus_only = TabGroupState::selected(0);
        assert_eq!(focus_only.focus_next(&tabs), Some(2));
        assert_eq!(focus_only.selected_tab_id(&tabs), Some("one"));
        assert_eq!(focus_only.select_focused(&tabs), Some(2));
        assert_eq!(focus_only.selected_tab_id(&tabs), Some("three"));
    }

    #[test]
    fn tab_group_builds_strip_and_selected_panel() {
        let mut doc = test_root();
        let root = doc.root;
        let tabs = vec![
            TabItem::new("inspect", "Inspect")
                .with_leading_image(ImageContent::new("icons.inspect"))
                .closable(),
            TabItem::new("history", "History").dirty(),
        ];
        let group = tab_group(
            &mut doc,
            root,
            "tabs",
            &tabs,
            TabGroupState {
                selected_index: Some(1),
                focused_index: Some(0),
            },
            TabGroupOptions {
                layout: Style {
                    size: TaffySize {
                        width: length(320.0),
                        height: length(180.0),
                    },
                    ..TabGroupOptions::default().layout
                },
                selected_tab_shader: Some(ShaderEffect::new("ui.tab_selected")),
                focused_tab_shader: Some(ShaderEffect::new("ui.tab_focused")),
                panel_shader: Some(ShaderEffect::new("ui.panel")),
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
        assert_eq!(
            doc.node(selected_tab).shader.as_ref().unwrap().key,
            "ui.tab_selected"
        );
        let focused_tab = doc.node(strip).children[0];
        assert_eq!(
            doc.node(focused_tab).shader.as_ref().unwrap().key,
            "ui.tab_focused"
        );
        assert_eq!(
            doc.node(strip).accessibility.as_ref().unwrap().role,
            AccessibilityRole::TabList
        );
        assert_eq!(
            doc.node(panel).accessibility.as_ref().unwrap().role,
            AccessibilityRole::TabPanel
        );
        assert_eq!(doc.node(panel).shader.as_ref().unwrap().key, "ui.panel");
        assert!(matches!(
            &doc.node(node_named(&doc, "tabs.tab.inspect.image")).content,
            UiContent::Image(image) if image.key == "icons.inspect"
        ));
    }
}
