use std::ops::Range;

use crate::actions::{action_target_enabled, keyboard_activation_key};
use crate::core::document::rect_is_finite;
use crate::platform::{
    ClipboardRequest, LogicalRect, PlatformRequest, TextImeRequest, TextImeResponse,
    TextImeSession, TextInputId, TextRange,
};
use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentage,
    LengthPercentageAuto, Size as TaffySize, Style,
};

use super::*;

pub mod button;
pub mod canvas;
pub mod checkbox;
pub mod collapsing;
pub mod combo_box;
pub mod container;
pub mod drag_drop;
pub mod drag_value;
pub mod ext;
pub mod form;
pub mod grid;
pub mod helpers;
pub mod image;
pub mod label;
pub mod modal;
pub mod panel;
pub mod radio;
pub mod scroll_area;
pub mod scrollbar;
pub mod separator;
pub mod slider;
pub mod spinner;
pub mod table;
#[cfg(test)]
mod tests;
pub mod text_input;
pub mod theme_preference;
pub mod toggle;
pub mod tooltip;
pub mod virtual_list;

pub use button::{
    button, icon_button, image_button, reset_button, small_button, toggle_button, ButtonOptions,
};
pub use canvas::{canvas, CanvasOptions};
pub use checkbox::{checkbox, checkbox_with_state, CheckboxOptions, CheckboxState};
pub use collapsing::{collapsing_header, CollapsingHeaderNodes, CollapsingHeaderOptions};
pub use combo_box::{combo_box, ComboBoxOptions};
pub use container::{
    columns, frame, group, indented_section, resize_container, scene, sides, ColumnsNodes,
    ColumnsOptions, FrameOptions, IndentOptions, ResizeContainerNodes, ResizeContainerOptions,
    SceneOptions, SidesNodes, SidesOptions,
};
pub use drag_drop::{
    dnd_drag_source, dnd_drop_zone, DragImagePolicy, DragSourceNodes, DragSourceOptions,
    DropZoneNodes, DropZoneOptions,
};
pub use drag_value::{drag_angle, drag_angle_tau, drag_value_input, DragValueOptions};
pub use ext::dialog::{DialogDismissReason, DialogDismissal};
pub use ext::numeric_input::{NumericPrecision, NumericRange, NumericUnitFormat};
pub use ext::toggle_control::ToggleValue;
pub use form::{
    field_help_text, field_label, field_validation_message, form_action_buttons,
    form_error_summary, form_field_order, form_has_errors, form_row, form_section, next_form_field,
    previous_form_field, validation_text_style, FieldHelpOptions, FieldLabelOptions,
    FormActionAvailability, FormActionButtonNodes, FormActionButtonsOptions, FormActionKind,
    FormActionLabels, FormErrorSummaryNodes, FormErrorSummaryOptions, FormRowOptions,
    FormSectionNodes, FormSectionOptions, ValidationMessageOptions,
};
pub use helpers::{
    add_enabled, add_enabled_ui, add_visible, add_visible_ui, scroll_to_cursor, scroll_to_rect,
    scroll_to_rect_with_options, set_subtree_enabled, set_subtree_visible,
};
pub use image::{image, ImageOptions};
pub use label::{
    code_label, code_text_style, colored_label, colored_text_style, heading_label,
    heading_text_style, hyperlink, label, link, localized_label, monospace_label,
    monospace_text_style, selectable_label, selectable_value, small_label, small_text_style,
    strong_label, strong_text_style, weak_label, weak_text_style, wrapped_label, LinkOptions,
    SelectableLabelOptions,
};
pub use modal::{modal_dialog, ModalDialogNodes, ModalDialogOptions};
pub use panel::{
    bottom_panel, central_panel, group_panel, left_panel, panel, right_panel, side_panel,
    top_panel, PanelKind, PanelOptions, SidePanelSide,
};
pub use radio::{radio_button, radio_group, RadioButtonOptions, RadioGroupOptions, RadioOption};
pub use scroll_area::{
    scroll_area, scroll_container, ScrollContainerNodes, ScrollContainerOptions,
};
pub use separator::{separator, spacer, SeparatorOptions, SeparatorOrientation};
pub use slider::{slider, SliderClamping, SliderOptions, SliderThumbShape, SliderValueSpec};
pub use spinner::{spinner, SpinnerOptions};
pub use table::{table_header, TableColumn};
pub use text_input::{
    code_editor, multiline_text_input, password_input, search_input, selectable_text,
    singleline_text_input, text_area, text_input, TextInputInteractionPolicy, TextInputOptions,
    TextInputState,
};
pub use theme_preference::{
    theme_preference_buttons, theme_preference_switch, ThemePreference, ThemePreferenceButtonNodes,
    ThemePreferenceButtonsOptions, ThemePreferenceLabels, ThemePreferenceSwitchOptions,
};
pub use toggle::{toggle_switch, ToggleSwitchOptions};
pub use tooltip::{tooltip_box, TooltipBoxOptions, TooltipTriggerMode, TooltipTriggerOptions};
pub use virtual_list::{virtual_list, VirtualListSpec};

pub(crate) fn single_line_text_style(mut style: TextStyle) -> TextStyle {
    style.wrap = TextWrap::None;
    style
}

pub(crate) fn inline_intrinsic_chrome_size(style: &Style, inline_items: usize) -> UiSize {
    UiSize::new(
        horizontal_padding_width(style.padding)
            + horizontal_gap_width(style.gap.width, inline_items),
        vertical_padding_height(style.padding),
    )
}

pub(crate) fn inline_intrinsic_base_size(
    style: &Style,
    fixed_items: &[&Style],
    inline_items: usize,
) -> UiSize {
    let mut size = inline_intrinsic_chrome_size(style, inline_items);
    for fixed_item in fixed_items {
        let item_size = inline_item_outer_size(fixed_item);
        size.width += item_size.width;
        size.height = size.height.max(item_size.height);
    }
    if let Some(height) =
        dimension_points(style.min_size.height).or_else(|| dimension_points(style.size.height))
    {
        size.height = size.height.max(height);
    }
    size
}

pub(crate) fn publish_inline_intrinsic_size(
    document: &mut UiDocument,
    target: UiNodeId,
    sources: impl Into<Vec<UiNodeId>>,
    min_size: UiSize,
) {
    document.node_mut(target).layout_constraint =
        Some(UiNodeLayoutConstraint::InlineIntrinsicSize {
            sources: sources.into(),
            min_size,
        });
}

fn inline_item_outer_size(style: &Style) -> UiSize {
    UiSize::new(
        dimension_points(style.min_size.width)
            .or_else(|| dimension_points(style.size.width))
            .unwrap_or(0.0)
            + horizontal_margin_width(style.margin),
        dimension_points(style.min_size.height)
            .or_else(|| dimension_points(style.size.height))
            .unwrap_or(0.0)
            + vertical_margin_height(style.margin),
    )
}

fn horizontal_gap_width(gap: LengthPercentage, inline_items: usize) -> f32 {
    if inline_items <= 1 {
        0.0
    } else {
        length_percentage_points(gap) * (inline_items - 1) as f32
    }
}

fn horizontal_padding_width(
    padding: taffy::prelude::Rect<taffy::prelude::LengthPercentage>,
) -> f32 {
    length_percentage_points(padding.left) + length_percentage_points(padding.right)
}

fn vertical_padding_height(padding: taffy::prelude::Rect<taffy::prelude::LengthPercentage>) -> f32 {
    length_percentage_points(padding.top) + length_percentage_points(padding.bottom)
}

fn horizontal_margin_width(margin: taffy::prelude::Rect<LengthPercentageAuto>) -> f32 {
    length_percentage_auto_points(margin.left) + length_percentage_auto_points(margin.right)
}

fn vertical_margin_height(margin: taffy::prelude::Rect<LengthPercentageAuto>) -> f32 {
    length_percentage_auto_points(margin.top) + length_percentage_auto_points(margin.bottom)
}

fn dimension_points(value: Dimension) -> Option<f32> {
    let raw = value.into_raw();
    (raw.tag() == taffy::prelude::CompactLength::LENGTH_TAG).then_some(raw.value())
}

fn length_percentage_points(value: taffy::prelude::LengthPercentage) -> f32 {
    let raw = value.into_raw();
    if raw.tag() == taffy::prelude::CompactLength::LENGTH_TAG {
        raw.value().max(0.0)
    } else {
        0.0
    }
}

fn length_percentage_auto_points(value: LengthPercentageAuto) -> f32 {
    let raw = value.into_raw();
    if raw.tag() == taffy::prelude::CompactLength::LENGTH_TAG {
        raw.value().max(0.0)
    } else {
        0.0
    }
}
