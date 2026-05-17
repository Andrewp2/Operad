//! Shared data primitives for extended data widgets.

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

#[cfg(test)]
mod tests {
    use taffy::prelude::{Dimension, Display, FlexDirection, Size as TaffySize, Style};

    use super::super::data_table::data_table_header_accessibility;
    use super::super::{
        data_table::*, editable_form::*, property_inspector::*, tab_group::*, toggle_control::*,
        tree_view::*,
    };
    use super::*;
    use crate::{
        platform::{ClipboardRequest, DragOperation, DragPayload},
        *,
    };

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
    fn toggle_control_state_tracks_values_phases_and_accessibility() {
        let mut toggle = ToggleControlState::mixed();
        let mixed_meta = toggle.accessibility_meta("Filter enabled", ToggleControlRole::Switch);
        assert_eq!(mixed_meta.role, AccessibilityRole::Switch);
        assert_eq!(mixed_meta.value.as_deref(), Some("mixed"));
        assert_eq!(mixed_meta.checked, Some(crate::AccessibilityChecked::Mixed));

        let update = toggle.toggle();
        assert_eq!(update.previous, ToggleValue::Mixed);
        assert_eq!(update.value, ToggleValue::On);
        assert_eq!(update.phase, EditPhase::UpdateEdit);
        assert!(update.changed);

        let button_meta = toggle.accessibility_meta("Pin panel", ToggleControlRole::ToggleButton);
        assert_eq!(button_meta.role, AccessibilityRole::ToggleButton);
        assert_eq!(button_meta.pressed, Some(true));

        let commit = toggle.commit();
        assert_eq!(commit.phase, EditPhase::CommitEdit);
        assert!(!commit.changed);

        let cancel = toggle.cancel_to(ToggleValue::Off);
        assert_eq!(cancel.phase, EditPhase::CancelEdit);
        assert_eq!(cancel.value, ToggleValue::Off);

        let disabled = ToggleControlState::new(true).disabled();
        let disabled_meta = disabled.accessibility_meta("MES sync", ToggleControlRole::Checkbox);
        assert!(!disabled_meta.enabled);
        assert!(!disabled_meta.focusable);
        assert_eq!(
            disabled_meta.checked,
            Some(crate::AccessibilityChecked::True)
        );
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
                action_prefix: Some("props.edit".to_owned()),
                ..Default::default()
            },
        );

        assert_eq!(doc.node(grid).children.len(), 2);
        let first_value = doc.node(doc.node(doc.node(grid).children[0]).children[1]);
        assert!(!first_value.input.pointer);
        let selected_row = doc.node(doc.node(grid).children[1]);
        assert_eq!(selected_row.visual.fill, ColorRgba::new(43, 62, 86, 255));
        assert_eq!(
            selected_row
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("props.edit.row.gain")
        );
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
    fn editable_form_state_tracks_focus_edit_commit_cancel_and_picker() {
        let fields = vec![
            EditableFormField::new("recipe", "Recipe", "A1", EditableFormFieldKind::Text)
                .with_command(EditableFormCommand::commit("form.commit.recipe")),
            EditableFormField::new("mode", "Mode", "Auto", EditableFormFieldKind::Select)
                .with_command(EditableFormCommand::open_picker("form.mode.open")),
            EditableFormField::new("locked", "Locked", "Yes", EditableFormFieldKind::ReadOnly)
                .read_only(),
            EditableFormField::new("disabled", "Disabled", "No", EditableFormFieldKind::Boolean)
                .disabled(),
        ];
        let mut state = EditableFormState::new();

        let outcome = state.move_focus(&fields, FocusDirection::Next);
        assert_eq!(outcome.focused_field.as_deref(), Some("recipe"));

        let outcome = state.begin_edit(&fields);
        assert_eq!(outcome.began_editing.as_deref(), Some("recipe"));
        assert_eq!(state.editing_field.as_deref(), Some("recipe"));

        let outcome = state.commit();
        assert_eq!(outcome.committed_field.as_deref(), Some("recipe"));
        assert!(state.editing_field.is_none());

        let outcome = state.move_focus(&fields, FocusDirection::Next);
        assert_eq!(outcome.focused_field.as_deref(), Some("mode"));
        let outcome = state.open_picker(&fields);
        assert_eq!(outcome.opened_picker.as_deref(), Some("mode"));

        let outcome = state.move_focus(&fields, FocusDirection::Next);
        assert_eq!(outcome.focused_field.as_deref(), Some("recipe"));

        state.focus_field(&fields, "mode");
        state.begin_edit(&fields);
        let outcome = state.cancel();
        assert_eq!(outcome.canceled_field.as_deref(), Some("mode"));
    }

    #[test]
    fn editable_form_contract_exports_status_commands_and_accessibility() {
        let fields = vec![
            EditableFormField::new("name", "Name", "Recipe A", EditableFormFieldKind::Text)
                .required()
                .changed()
                .with_command(EditableFormCommand::commit("form.name.commit"))
                .with_command(EditableFormCommand::cancel("form.name.cancel").disabled()),
            EditableFormField::new("gain", "Gain", "120", EditableFormFieldKind::Number)
                .invalid("Out of range")
                .pending(),
            EditableFormField::new("mode", "Mode", "Auto", EditableFormFieldKind::Select)
                .with_command(EditableFormCommand::open_picker("form.mode.open")),
            EditableFormField::from_property(
                &PropertyGridRow::new("locked", "Locked", "Yes").read_only(),
            ),
        ];
        let state = EditableFormState::new().editing("name");

        let contract = editable_form_contract("recipe.form", "Recipe form", &fields, &state);

        assert_eq!(contract.field_count, 4);
        assert_eq!(contract.editable_count, 3);
        assert_eq!(contract.invalid_count, 1);
        assert_eq!(contract.changed_count, 1);
        assert_eq!(contract.pending_count, 1);
        assert_eq!(contract.accessibility.role, AccessibilityRole::Group);
        assert!(contract
            .accessibility
            .value
            .as_deref()
            .unwrap()
            .contains("1 invalid"));

        let name = &contract.fields[0];
        assert!(name.focused);
        assert!(name.editing);
        assert_eq!(name.accessibility.role, AccessibilityRole::TextBox);
        assert!(name.accessibility.required);
        assert_eq!(name.accessibility.actions.len(), 1);
        assert_eq!(name.accessibility.actions[0].id, "form.name.commit");

        let gain = &contract.fields[1];
        assert_eq!(gain.accessibility.role, AccessibilityRole::SpinButton);
        assert_eq!(gain.accessibility.invalid.as_deref(), Some("Out of range"));
        assert_eq!(
            gain.accessibility.live_region,
            AccessibilityLiveRegion::Polite
        );

        let mode = &contract.fields[2];
        assert_eq!(mode.accessibility.role, AccessibilityRole::ComboBox);
        assert_eq!(mode.accessibility.actions[0].id, "form.mode.open");

        let locked = &contract.fields[3];
        assert!(locked.accessibility.read_only);
        assert!(!locked.accessibility.focusable);
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
    fn data_table_column_sort_filter_metadata_feeds_header_accessibility() {
        let column = DataTableColumn::new("track", "Track", 160.0)
            .with_sort(DataTableSortState::descending().with_priority(2))
            .sortable("table.sort.track")
            .with_filter(DataTableFilterState::active("contains").with_value("audio"))
            .filterable("table.filter.track")
            .resize_command("table.resize.track");

        assert_eq!(
            DataTableSortDirection::Ascending.toggled(),
            DataTableSortDirection::Descending
        );
        assert_eq!(
            column.sort.as_ref().unwrap().direction.accessibility_sort(),
            AccessibilitySortDirection::Descending
        );
        assert_eq!(
            column.filter.as_ref().unwrap().accessibility_value(),
            "filtered; contains; value audio"
        );

        let accessibility = data_table_header_accessibility(&column, 0, 3);
        assert_eq!(accessibility.role, AccessibilityRole::ColumnHeader);
        assert_eq!(accessibility.sort, AccessibilitySortDirection::Descending);
        let value = accessibility.value.as_deref().unwrap();
        assert!(value.contains("column 1 of 3"));
        assert!(value.contains("resizable"));
        assert!(value.contains("sorted descending priority 2"));
        assert!(value.contains("filtered; contains; value audio"));
        assert_eq!(
            accessibility
                .actions
                .iter()
                .map(|action| action.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "table.sort.track",
                "table.filter.track",
                "table.resize.track"
            ]
        );

        let fixed = DataTableColumn::new("locked", "Locked", 80.0)
            .fixed()
            .sortable("table.sort.locked")
            .filterable("table.filter.locked")
            .resize_command("table.resize.locked");
        let accessibility = data_table_header_accessibility(&fixed, 1, 3);
        assert_eq!(accessibility.sort, AccessibilitySortDirection::None);
        assert!(accessibility
            .value
            .as_deref()
            .unwrap()
            .contains("filter available"));
        assert_eq!(
            accessibility
                .actions
                .iter()
                .map(|action| action.id.as_str())
                .collect::<Vec<_>>(),
            vec!["table.sort.locked", "table.filter.locked"]
        );
    }

    #[test]
    fn data_table_row_and_cell_meta_expose_actions_and_context_commands() {
        let row = DataTableRowMeta::new(4)
            .with_row_id("clip.4")
            .with_actions([
                DataTableAction::new("rename", "Rename"),
                DataTableAction::new("delete", "Delete")
                    .destructive()
                    .disabled(),
            ])
            .with_context_menu_commands(["duplicate", "reveal"]);
        let enabled = row.enabled_actions();

        assert_eq!(row.row, 4);
        assert_eq!(row.row_id.as_deref(), Some("clip.4"));
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id.as_str(), "rename");
        assert!(row.has_context_menu());
        assert_eq!(
            enabled[0].accessibility_action(),
            AccessibilityAction::new("rename", "Rename")
        );

        let cell = DataTableCellMeta::new(DataTableCellIndex::new(4, 2))
            .with_action(DataTableAction::new("copy", "Copy value"))
            .with_action(DataTableAction::new("clear", "Clear value").disabled())
            .with_context_menu_command("cell.context");

        assert_eq!(cell.cell, DataTableCellIndex::new(4, 2));
        assert_eq!(cell.enabled_actions().len(), 1);
        assert!(cell.has_context_menu());
        assert_eq!(cell.context_menu_commands[0].as_str(), "cell.context");
    }

    #[test]
    fn data_table_row_meta_builds_drag_and_drop_descriptors() {
        let policy = DataTableRowDropPolicy::new(DropPayloadFilter::empty().text())
            .accepted_operations([DragOperation::Move])
            .placements([
                DataTableRowDropPlacement::Before,
                DataTableRowDropPlacement::On,
                DataTableRowDropPlacement::After,
            ]);
        let row = DataTableRowMeta::new(7)
            .with_row_id("clip.7")
            .draggable(true)
            .with_drop_policy(policy);
        let bounds = UiRect::new(20.0, 40.0, 200.0, 32.0);

        let source = row
            .drag_source(bounds, DragPayload::text("clip.7"), [DragOperation::Move])
            .expect("drag source");
        assert_eq!(source.id, DragSourceId::new("data_table.row.clip.7"));
        assert_eq!(source.kind, DragDropSurfaceKind::TableRow);
        assert_eq!(source.label.as_deref(), Some("clip.7"));
        assert!(source.can_start());

        let targets = row.drop_targets(bounds);
        assert_eq!(targets.len(), 3);
        assert_eq!(
            targets[0].id,
            DropTargetId::new("data_table.row.clip.7.before")
        );
        assert_eq!(targets[0].bounds, UiRect::new(20.0, 40.0, 200.0, 8.0));
        assert_eq!(targets[1].id, DropTargetId::new("data_table.row.clip.7.on"));
        assert_eq!(targets[1].bounds, bounds);
        assert_eq!(
            targets[2].id,
            DropTargetId::new("data_table.row.clip.7.after")
        );
        assert_eq!(targets[2].bounds, UiRect::new(20.0, 64.0, 200.0, 8.0));
        assert_eq!(
            targets[1].resolve_operation(&DragPayload::text("clip.7"), &[DragOperation::Move]),
            Some(DragOperation::Move)
        );

        assert!(DataTableRowMeta::new(8)
            .draggable(true)
            .disabled()
            .drag_source(bounds, DragPayload::text("clip.8"), [DragOperation::Move])
            .is_none());
        assert!(!DataTableRowDropPolicy::new(DropPayloadFilter::empty()).enabled());
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
                        LayoutStyle::from_taffy_style(Style {
                            size: TaffySize {
                                width: Dimension::auto(),
                                height: Dimension::auto(),
                            },
                            ..Default::default()
                        }),
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
    fn virtualized_data_table_header_shares_horizontal_scroll_contract() {
        let mut doc = test_root();
        let root = doc.root;
        let columns = vec![
            DataTableColumn::new("name", "Name", 120.0),
            DataTableColumn::new("status", "Status", 90.0),
            DataTableColumn::new("value", "Value", 80.0),
        ];

        virtualized_data_table(
            &mut doc,
            root,
            "table",
            &columns,
            VirtualDataTableSpec {
                row_count: 4,
                row_height: 20.0,
                viewport_width: 200.0,
                viewport_height: 40.0,
                scroll_offset: UiPoint::new(40.0, 0.0),
                overscan_rows: 0,
            },
            DataTableOptions::default(),
            |document, parent, cell| {
                document.add_child(
                    parent,
                    UiNode::text(
                        format!("cell.{}.{}", cell.row, cell.column),
                        format!("{}:{}", cell.row, cell.column),
                        TextStyle::default(),
                        LayoutStyle::new(),
                    ),
                );
            },
        );

        let header = node_named(&doc, "table.header");
        let header_scroll = doc.scroll_state(header).expect("header scroll state");
        assert_eq!(header_scroll.axes, ScrollAxes::HORIZONTAL);
        assert_eq!(header_scroll.offset.x, 40.0);

        doc.compute_layout(UiSize::new(640.0, 480.0), &mut ApproxTextMeasurer)
            .expect("layout");
        assert!(
            !doc.audit_layout().iter().any(|warning| matches!(
                warning,
                AuditWarning::TextClipped { name, .. }
                    if name == "table.header.value.label"
            )),
            "horizontally scrollable header text should not be treated as hard clipping"
        );
    }

    #[test]
    fn virtualized_data_table_exposes_header_commands_and_scroll_action() {
        let mut doc = test_root();
        let root = doc.root;
        let columns = vec![
            DataTableColumn::new("name", "Name", 120.0)
                .with_sort(DataTableSortState::ascending())
                .sortable("table.sort.name"),
            DataTableColumn::new("status", "Status", 90.0)
                .with_filter(DataTableFilterState::active("state").with_value("Ready"))
                .filterable("table.filter.status"),
            DataTableColumn::new("value", "Value", 80.0).resize_command("table.resize.value"),
        ];
        virtualized_data_table(
            &mut doc,
            root,
            "table",
            &columns,
            VirtualDataTableSpec {
                row_count: 20,
                row_height: 20.0,
                viewport_width: 240.0,
                viewport_height: 60.0,
                scroll_offset: UiPoint::new(0.0, 20.0),
                overscan_rows: 0,
            },
            DataTableOptions::default().with_scroll_action("table.scroll"),
            |_, _, _| {},
        );

        let name_header = doc.node(node_named(&doc, "table.header.name"));
        assert_eq!(
            name_header
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("table.sort.name")
        );
        assert!(name_header.input.pointer);

        let status_header = doc.node(node_named(&doc, "table.header.status"));
        assert_eq!(
            status_header
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("table.filter.status")
        );

        let resize = doc.node(node_named(&doc, "table.header.value.resize"));
        assert_eq!(
            resize
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("table.resize.value")
        );
        assert_eq!(resize.action_mode, WidgetActionMode::PointerEditParentRect);
        assert!(resize.input.pointer);

        let body = doc.node(node_named(&doc, "table.body"));
        assert_eq!(
            body.action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("table.scroll")
        );
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
    fn virtualized_tree_view_materializes_visible_rows_and_spacers() {
        let mut doc = test_root();
        let root = doc.root;
        let children = (0..20)
            .map(|index| TreeItem::new(format!("child-{index}"), format!("Child {index}")))
            .collect::<Vec<_>>();
        let roots = vec![TreeItem::new("project", "Project").with_children(children)];
        let mut state = TreeViewState::expanded(["project"]);
        state.select(Some(6));
        let nodes = virtualized_tree_view(
            &mut doc,
            root,
            "tree",
            &roots,
            &state,
            VirtualTreeViewSpec::new(20.0, 60.0)
                .scroll_offset(100.0)
                .overscan_rows(1),
            TreeViewOptions {
                selected_row_shader: Some(ShaderEffect::new("ui.tree_selected")),
                ..Default::default()
            },
        );

        assert_eq!(nodes.plan.visible_range, 5..8);
        assert_eq!(nodes.plan.materialized_range, 4..9);
        assert_eq!(nodes.rows.len(), 5);
        assert!(nodes.top_spacer.is_some());
        assert!(nodes.bottom_spacer.is_some());
        assert_eq!(doc.node(nodes.body).children.len(), 7);
        assert!(doc
            .node(node_named(&doc, "tree.row.child-3"))
            .name
            .ends_with("child-3"));
        assert_eq!(
            doc.node(node_named(&doc, "tree.row.child-5"))
                .shader
                .as_ref()
                .unwrap()
                .key,
            "ui.tree_selected"
        );
        assert!(doc
            .node(nodes.root)
            .accessibility
            .as_ref()
            .unwrap()
            .value
            .as_deref()
            .unwrap()
            .contains("showing 6-8"));

        doc.compute_layout(UiSize::new(640.0, 480.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let scroll = doc.scroll_state(nodes.body).expect("tree scroll state");
        assert_eq!(scroll.offset.y, 100.0);
        assert_eq!(scroll.content_size.height, 420.0);
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
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: TaffySize {
                        width: length(320.0),
                        height: length(180.0),
                    },
                    ..Default::default()
                }),
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
                        LayoutStyle::from_taffy_style(Style {
                            size: TaffySize {
                                width: Dimension::auto(),
                                height: Dimension::auto(),
                            },
                            ..Default::default()
                        }),
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

    #[test]
    fn tab_group_publishes_strip_minimum_from_all_tabs() {
        let mut doc = UiDocument::new(root_style(240.0, 160.0));
        let root = doc.root;
        let tabs = vec![
            TabItem::new("one", "One"),
            TabItem::new("two", "Two"),
            TabItem::new("three", "Three"),
        ];
        let group = tab_group(
            &mut doc,
            root,
            "tabs",
            &tabs,
            TabGroupState::selected(0),
            TabGroupOptions {
                min_tab_width: 96.0,
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(120.0),
                    },
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, _, _| {},
        );
        doc.compute_layout(UiSize::new(240.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let strip = doc.node(group).children[0];
        let strip_rect = doc.node(strip).layout.rect;
        assert!(strip_rect.width >= 96.0 * 3.0, "strip_rect={strip_rect:?}");
    }
}
