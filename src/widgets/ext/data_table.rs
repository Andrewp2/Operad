//! Data table widget implementation.

use std::collections::HashSet;
use std::ops::Range;

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentageAuto, Position,
    Size as TaffySize, Style,
};

use crate::{
    commands::CommandEffect,
    drag_drop::{DragSourceDescriptor, DragSourceId, DropTargetDescriptor, DropTargetId},
    platform::{ClipboardRequest, DragOperation, DragPayload},
    AccessibilityAction, AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole,
    AccessibilitySortDirection, ClipBehavior, ColorRgba, CommandId, DragDropSurfaceKind,
    DropPayloadFilter, ImageContent, InputBehavior, LayoutStyle, ScrollAxes, ShaderEffect,
    StrokeStyle, TextStyle, TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiPoint, UiRect,
    UiVisual, WidgetActionMode,
};

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
    pub sort: Option<DataTableSortState>,
    pub filter: Option<DataTableFilterState>,
    pub sort_command: Option<CommandId>,
    pub filter_command: Option<CommandId>,
    pub resize_command: Option<CommandId>,
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
            sort: None,
            filter: None,
            sort_command: None,
            filter_command: None,
            resize_command: None,
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

    pub fn with_sort(mut self, sort: DataTableSortState) -> Self {
        self.sort = Some(sort);
        self
    }

    pub fn sortable(mut self, command: impl Into<CommandId>) -> Self {
        self.sort_command = Some(command.into());
        self
    }

    pub fn with_filter(mut self, filter: DataTableFilterState) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn filterable(mut self, command: impl Into<CommandId>) -> Self {
        self.filter_command = Some(command.into());
        self
    }

    pub fn resize_command(mut self, command: impl Into<CommandId>) -> Self {
        self.resize_command = Some(command.into());
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTableSortDirection {
    Ascending,
    Descending,
}

impl DataTableSortDirection {
    pub const fn toggled(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    pub const fn accessibility_sort(self) -> AccessibilitySortDirection {
        match self {
            Self::Ascending => AccessibilitySortDirection::Ascending,
            Self::Descending => AccessibilitySortDirection::Descending,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Ascending => "ascending",
            Self::Descending => "descending",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableSortState {
    pub direction: DataTableSortDirection,
    pub priority: usize,
}

impl DataTableSortState {
    pub const fn new(direction: DataTableSortDirection) -> Self {
        Self {
            direction,
            priority: 0,
        }
    }

    pub const fn ascending() -> Self {
        Self::new(DataTableSortDirection::Ascending)
    }

    pub const fn descending() -> Self {
        Self::new(DataTableSortDirection::Descending)
    }

    pub const fn with_priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }

    pub fn accessibility_value(&self) -> String {
        if self.priority == 0 {
            format!("sorted {}", self.direction.label())
        } else {
            format!(
                "sorted {} priority {}",
                self.direction.label(),
                self.priority
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableFilterState {
    pub active: bool,
    pub label: Option<String>,
    pub value: Option<String>,
}

impl DataTableFilterState {
    pub fn inactive() -> Self {
        Self {
            active: false,
            label: None,
            value: None,
        }
    }

    pub fn active(label: impl Into<String>) -> Self {
        Self {
            active: true,
            label: Some(label.into()),
            value: None,
        }
    }

    pub fn value(value: impl Into<String>) -> Self {
        Self {
            active: true,
            label: None,
            value: Some(value.into()),
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.active = true;
        self.value = Some(value.into());
        self
    }

    pub fn accessibility_value(&self) -> String {
        if !self.active {
            return "filter available".to_owned();
        }

        let mut value = vec!["filtered".to_owned()];
        if let Some(label) = &self.label {
            value.push(label.clone());
        }
        if let Some(filter_value) = &self.value {
            value.push(format!("value {filter_value}"));
        }
        value.join("; ")
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

/// Renderer-neutral action metadata for table rows and cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableAction {
    pub id: CommandId,
    pub label: String,
    pub disabled: bool,
    pub destructive: bool,
    pub leading_image: Option<ImageContent>,
}

impl DataTableAction {
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

/// Row-level action and context-menu metadata for a data table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableRowMeta {
    pub row: usize,
    pub row_id: Option<String>,
    pub disabled: bool,
    pub actions: Vec<DataTableAction>,
    pub context_menu_commands: Vec<CommandId>,
    pub draggable: bool,
    pub drop_policy: Option<DataTableRowDropPolicy>,
}

impl DataTableRowMeta {
    pub fn new(row: usize) -> Self {
        Self {
            row,
            row_id: None,
            disabled: false,
            actions: Vec::new(),
            context_menu_commands: Vec::new(),
            draggable: false,
            drop_policy: None,
        }
    }

    pub fn with_row_id(mut self, row_id: impl Into<String>) -> Self {
        self.row_id = Some(row_id.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn with_action(mut self, action: DataTableAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_actions(mut self, actions: impl IntoIterator<Item = DataTableAction>) -> Self {
        self.actions.extend(actions);
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

    pub fn with_drop_policy(mut self, policy: DataTableRowDropPolicy) -> Self {
        self.drop_policy = Some(policy);
        self
    }

    pub fn enabled_actions(&self) -> Vec<&DataTableAction> {
        self.actions
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
                DragSourceId::new(format!("data_table.row.{}", self.descriptor_id())),
                DragDropSurfaceKind::TableRow,
                bounds,
                payload,
            )
            .allowed_operations(allowed_operations)
            .label(self.accessibility_label())
        })
    }

    pub fn drop_target(
        &self,
        bounds: UiRect,
        placement: DataTableRowDropPlacement,
    ) -> Option<DropTargetDescriptor> {
        let policy = self.drop_policy.as_ref()?;
        policy.allows_placement(placement).then(|| {
            DropTargetDescriptor::new(
                DropTargetId::new(format!(
                    "data_table.row.{}.{}",
                    self.descriptor_id(),
                    placement.suffix()
                )),
                DragDropSurfaceKind::TableRow,
                placement.bounds(bounds),
            )
            .accepted_payload(policy.accepted_payload.clone())
            .accepted_operations(policy.accepted_operations.clone())
            .label(format!(
                "{} {}",
                self.accessibility_label(),
                placement.label()
            ))
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

    fn descriptor_id(&self) -> String {
        self.row_id.clone().unwrap_or_else(|| self.row.to_string())
    }

    fn accessibility_label(&self) -> String {
        self.row_id
            .as_ref()
            .cloned()
            .unwrap_or_else(|| format!("Row {}", self.row + 1))
    }
}

/// Cell-level action and context-menu metadata for a data table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableCellMeta {
    pub cell: DataTableCellIndex,
    pub disabled: bool,
    pub actions: Vec<DataTableAction>,
    pub context_menu_commands: Vec<CommandId>,
}

impl DataTableCellMeta {
    pub fn new(cell: DataTableCellIndex) -> Self {
        Self {
            cell,
            disabled: false,
            actions: Vec::new(),
            context_menu_commands: Vec::new(),
        }
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn with_action(mut self, action: DataTableAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_actions(mut self, actions: impl IntoIterator<Item = DataTableAction>) -> Self {
        self.actions.extend(actions);
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

    pub fn enabled_actions(&self) -> Vec<&DataTableAction> {
        self.actions
            .iter()
            .filter(|action| !action.disabled)
            .collect()
    }

    pub fn has_context_menu(&self) -> bool {
        !self.context_menu_commands.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataTableRowDropPlacement {
    Before,
    On,
    After,
}

impl DataTableRowDropPlacement {
    pub const ALL: [Self; 3] = [Self::Before, Self::On, Self::After];

    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::On => "on",
            Self::After => "after",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::On => "on",
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
            Self::On => row_bounds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableRowDropPolicy {
    pub accepted_payload: DropPayloadFilter,
    pub accepted_operations: Vec<DragOperation>,
    pub placements: Vec<DataTableRowDropPlacement>,
    pub disabled: bool,
}

impl DataTableRowDropPolicy {
    pub fn new(accepted_payload: DropPayloadFilter) -> Self {
        Self {
            accepted_payload,
            accepted_operations: vec![
                DragOperation::Copy,
                DragOperation::Move,
                DragOperation::Link,
            ],
            placements: vec![DataTableRowDropPlacement::On],
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

    pub fn placements(
        mut self,
        placements: impl IntoIterator<Item = DataTableRowDropPlacement>,
    ) -> Self {
        self.placements = placements.into_iter().collect();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn allows_placement(&self, placement: DataTableRowDropPlacement) -> bool {
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
    pub layout: LayoutStyle,
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
    pub row_action_prefix: Option<String>,
    pub cell_action_prefix: Option<String>,
    pub scroll_action: Option<String>,
}

impl Default for DataTableOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
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
            row_action_prefix: None,
            cell_action_prefix: None,
            scroll_action: None,
        }
    }
}

impl DataTableOptions {
    pub fn with_row_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.row_action_prefix = Some(prefix.into());
        self
    }

    pub fn with_cell_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.cell_action_prefix = Some(prefix.into());
        self
    }

    pub fn with_scroll_action(mut self, action: impl Into<String>) -> Self {
        self.scroll_action = Some(action.into());
        self
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
                layout: options.layout.style.clone(),
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

    data_table_header(
        document,
        root,
        format!("{name}.header"),
        columns,
        &options,
        scroll_offset.x,
    );

    let body = document.add_child(
        root,
        UiNode::container(
            format!("{name}.body"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: px(spec.viewport_height),
                    },
                    min_size: TaffySize {
                        width: px(0.0),
                        height: px(0.0),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_scroll(ScrollAxes::BOTH),
    );

    if let Some(scroll) = &mut document.node_mut(body).scroll {
        scroll.offset = scroll_offset;
    }
    if let Some(action) = options.scroll_action.clone() {
        document.node_mut(body).action = Some(action.into());
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
        let mut row_node = UiNode::container(
            format!("{name}.row.{row}"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    size: TaffySize {
                        width: px(table_width),
                        height: px(spec.row_height),
                    },
                    flex_shrink: 0.0,
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(visual)
        .with_accessibility(data_table_row_accessibility(row, spec.row_count, selected));
        if let Some(prefix) = options.row_action_prefix.as_deref() {
            row_node = row_node.with_action(format!("{prefix}.row.{row}"));
        }
        let row_node = with_optional_shader(
            row_node,
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
                    layout: LayoutStyle::from_taffy_style(Style {
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
                    })
                    .style,
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
            if let Some(prefix) = options.cell_action_prefix.as_deref() {
                cell = cell.with_action(format!(
                    "{prefix}.cell.{}.{}",
                    cell_index.row, cell_index.column
                ));
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

fn data_table_header(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    columns: &[DataTableColumn],
    options: &DataTableOptions,
    scroll_x: f32,
) -> UiNodeId {
    let name = name.into();
    let header = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: px(options.header_height),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.header_visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::ListItem)
                .label("Column headers")
                .value(format!("{} columns", columns.len())),
        )
        .with_scroll(ScrollAxes::HORIZONTAL),
    );
    if let Some(scroll) = &mut document.node_mut(header).scroll {
        scroll.offset.x = finite_nonnegative(scroll_x);
    }

    for (column_index, column) in columns.iter().enumerate() {
        let mut cell_node = UiNode::container(
            format!("{name}.{}", column.id),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
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
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_accessibility(data_table_header_accessibility(
            column,
            column_index,
            columns.len(),
        ));
        if let Some(action) = column
            .sort_command
            .as_ref()
            .or(column.filter_command.as_ref())
        {
            cell_node = cell_node
                .with_input(InputBehavior::BUTTON)
                .with_action(action.as_str());
        }
        let cell = document.add_child(header, cell_node);
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
                LayoutStyle::from_taffy_style(Style {
                    flex_grow: 1.0,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            ),
        );
        if column.resizable {
            if let Some(command) = &column.resize_command {
                document.add_child(
                    cell,
                    UiNode::container(
                        format!("{name}.{}.resize", column.id),
                        UiNodeStyle {
                            layout: LayoutStyle::from_taffy_style(Style {
                                position: Position::Absolute,
                                inset: taffy::prelude::Rect {
                                    left: LengthPercentageAuto::auto(),
                                    right: LengthPercentageAuto::length(0.0),
                                    top: LengthPercentageAuto::length(0.0),
                                    bottom: LengthPercentageAuto::length(0.0),
                                },
                                size: TaffySize {
                                    width: px(8.0),
                                    height: Dimension::percent(1.0),
                                },
                                ..Default::default()
                            })
                            .style,
                            ..Default::default()
                        },
                    )
                    .with_input(InputBehavior::BUTTON)
                    .with_action(command.as_str())
                    .with_action_mode(WidgetActionMode::PointerEditParentRect)
                    .with_visual(UiVisual::panel(
                        ColorRgba::new(108, 122, 144, 110),
                        None,
                        0.0,
                    ))
                    .with_accessibility(
                        AccessibilityMeta::new(AccessibilityRole::Button)
                            .label(format!("Resize {}", column.label))
                            .focusable(),
                    ),
                );
            }
        }
    }

    header
}

fn vertical_spacer(name: impl Into<String>, width: f32, height: f32) -> UiNode {
    UiNode::container(
        name,
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: px(width),
                    height: px(height),
                },
                flex_shrink: 0.0,
                ..Default::default()
            })
            .style,
            ..Default::default()
        },
    )
}

fn justify_content(alignment: DataCellAlignment) -> JustifyContent {
    match alignment {
        DataCellAlignment::Start => JustifyContent::FlexStart,
        DataCellAlignment::Center => JustifyContent::Center,
        DataCellAlignment::End => JustifyContent::FlexEnd,
    }
}

pub(super) fn data_table_header_accessibility(
    column: &DataTableColumn,
    column_index: usize,
    column_count: usize,
) -> AccessibilityMeta {
    let mut value = vec![
        format!("column {} of {}", column_index + 1, column_count),
        if column.resizable {
            "resizable"
        } else {
            "fixed"
        }
        .to_owned(),
    ];
    if let Some(sort) = &column.sort {
        value.push(sort.accessibility_value());
    } else if column.sort_command.is_some() {
        value.push("sortable".to_owned());
    }
    if let Some(filter) = &column.filter {
        value.push(filter.accessibility_value());
    } else if column.filter_command.is_some() {
        value.push("filter available".to_owned());
    }

    let mut meta = AccessibilityMeta::new(AccessibilityRole::ColumnHeader)
        .label(column.label.clone())
        .value(value.join("; "))
        .sort(
            column
                .sort
                .as_ref()
                .map(|sort| sort.direction.accessibility_sort())
                .unwrap_or(AccessibilitySortDirection::None),
        );
    if let Some(command) = &column.sort_command {
        meta = meta.action(AccessibilityAction::new(
            command.as_str(),
            format!("Sort {}", column.label),
        ));
    }
    if let Some(command) = &column.filter_command {
        meta = meta.action(AccessibilityAction::new(
            command.as_str(),
            format!("Filter {}", column.label),
        ));
    }
    if column.resizable {
        if let Some(command) = &column.resize_command {
            meta = meta.action(AccessibilityAction::new(
                command.as_str(),
                format!("Resize {}", column.label),
            ));
        }
    }
    meta
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

fn data_view_empty_reason_label(reason: DataViewEmptyReason) -> &'static str {
    match reason {
        DataViewEmptyReason::NoRows => "no rows",
        DataViewEmptyReason::NoMatches => "no matches",
        DataViewEmptyReason::NoVisibleRows => "no visible rows",
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

fn leading_image_node(
    name: impl Into<String>,
    image: ImageContent,
    size: f32,
    label: Option<String>,
) -> UiNode {
    let node = UiNode::image(
        name,
        image,
        LayoutStyle::from_taffy_style(Style {
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
        }),
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
