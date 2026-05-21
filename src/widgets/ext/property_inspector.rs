//! Property inspector widget implementation.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, LengthPercentageAuto, Size as TaffySize, Style,
};

use crate::widgets::{
    inline_intrinsic_base_size, publish_inline_intrinsic_size, single_line_text_style,
};
use crate::{
    AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole, ClipBehavior, ColorRgba,
    ImageContent, InputBehavior, LayoutStyle, ShaderEffect, StrokeStyle, TextStyle, TextWrap,
    UiDocument, UiNode, UiNodeId, UiNodeStyle, UiVisual,
};

use super::data::{PropertyRowStatus, PropertyValueKind};

const PROPERTY_CELL_PADDING: f32 = 6.0;

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
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub action_prefix: Option<String>,
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
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 140.0,
            row_height: 28.0,
            action_prefix: None,
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
            value_style: TextStyle {
                wrap: TextWrap::None,
                ..Default::default()
            },
            read_only_value_style: read_only_value_text_style(),
            leading_image_size: 16.0,
            accessibility_label: None,
        }
    }
}

impl PropertyInspectorOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
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
                .value(format!("{} properties", rows.len())),
        ),
    );

    for (index, row) in rows.iter().enumerate() {
        let selected = options.selected_index == Some(index);
        let focused = options.focused_index == Some(index);
        let visual = property_row_visual(row, selected, &options);
        let shader = property_row_shader(row, selected, focused, &options);
        let row_layout = Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            size: TaffySize {
                width: Dimension::percent(1.0),
                height: px(options.row_height),
            },
            ..Default::default()
        };
        let row_node = with_optional_action(
            with_optional_shader(
                UiNode::container(
                    format!("{name}.row.{}", row.id),
                    UiNodeStyle {
                        layout: LayoutStyle::from_taffy_style(row_layout.clone()).style,
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
            ),
            row_action(&options.action_prefix, row),
        );
        let row_node = document.add_child(root, row_node);
        let mut fixed_intrinsic_items = Vec::<Style>::new();

        if let Some(image) = row.leading_image.clone() {
            fixed_intrinsic_items.push(leading_image_style(options.leading_image_size));
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

        let label_node = document.add_child(
            row_node,
            UiNode::text(
                format!("{name}.row.{}.label", row.id),
                &row.label,
                single_line_text_style(options.label_style.clone()),
                LayoutStyle::from_taffy_style(Style {
                    flex_shrink: 0.0,
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::percent(1.0),
                    },
                    min_size: TaffySize {
                        width: px(options.label_width),
                        height: Dimension::auto(),
                    },
                    padding: taffy::prelude::Rect::length(PROPERTY_CELL_PADDING),
                    ..Default::default()
                }),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Label).label(row.label.clone()),
            ),
        );

        let value_style = if row.editable {
            single_line_text_style(options.value_style.clone())
        } else {
            single_line_text_style(options.read_only_value_style.clone())
        };
        let value_node = document.add_child(
            row_node,
            UiNode::text(
                format!("{name}.row.{}.value", row.id),
                &row.value,
                value_style,
                LayoutStyle::from_taffy_style(Style {
                    flex_grow: 1.0,
                    flex_shrink: 0.0,
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::percent(1.0),
                    },
                    padding: taffy::prelude::Rect::length(PROPERTY_CELL_PADDING),
                    ..Default::default()
                }),
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
        let fixed_items = fixed_intrinsic_items.iter().collect::<Vec<_>>();
        let mut intrinsic_base =
            inline_intrinsic_base_size(&row_layout, &fixed_items, fixed_items.len() + 2);
        intrinsic_base.width += PROPERTY_CELL_PADDING * 4.0;
        publish_inline_intrinsic_size(document, row_node, [label_node, value_node], intrinsic_base);
    }

    root
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

fn leading_image_node(
    name: impl Into<String>,
    image: ImageContent,
    size: f32,
    label: Option<String>,
) -> UiNode {
    let node = UiNode::image(
        name,
        image,
        LayoutStyle::from_taffy_style(leading_image_style(size)),
    );
    if let Some(label) = label {
        node.with_accessibility(AccessibilityMeta::new(AccessibilityRole::Image).label(label))
    } else {
        node
    }
}

fn leading_image_style(size: f32) -> Style {
    Style {
        size: TaffySize {
            width: px(size),
            height: px(size),
        },
        margin: taffy::prelude::Rect {
            right: LengthPercentageAuto::length(6.0),
            ..taffy::prelude::Rect::length(0.0_f32)
        },
        flex_shrink: 0.0,
        ..Default::default()
    }
}

fn with_optional_shader(mut node: UiNode, shader: Option<&ShaderEffect>) -> UiNode {
    if let Some(shader) = shader {
        node = node.with_shader(shader.clone());
    }
    node
}

fn with_optional_action(mut node: UiNode, action: Option<String>) -> UiNode {
    if let Some(action) = action {
        node = node.with_action(action);
    }
    node
}

fn row_action(prefix: &Option<String>, row: &PropertyGridRow) -> Option<String> {
    if row.disabled {
        None
    } else {
        prefix
            .as_ref()
            .map(|prefix| format!("{prefix}.row.{}", row.id))
    }
}

fn accessibility_label_or_name(label: &Option<String>, name: &str) -> String {
    label.clone().unwrap_or_else(|| name.to_owned())
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

fn muted_text_style() -> TextStyle {
    TextStyle {
        color: ColorRgba::new(151, 162, 178, 255),
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn read_only_value_text_style() -> TextStyle {
    TextStyle {
        wrap: TextWrap::None,
        ..muted_text_style()
    }
}

fn px(value: f32) -> Dimension {
    Dimension::length(value.max(0.0))
}
