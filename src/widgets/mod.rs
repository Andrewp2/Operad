use std::ops::Range;

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

pub use crate::widget_ext::*;

pub mod button;
pub mod canvas;
pub mod checkbox;
pub mod collapsing;
pub mod combo_box;
pub mod container;
pub mod drag_drop;
pub mod drag_value;
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

pub use button::*;
pub use canvas::*;
pub use checkbox::*;
pub use collapsing::*;
pub use combo_box::*;
pub use container::*;
pub use drag_drop::*;
pub use drag_value::*;
pub use form::*;
pub use grid::*;
pub use helpers::*;
pub use image::*;
pub use label::*;
pub use modal::*;
pub use panel::*;
pub use radio::*;
pub use scroll_area::*;
pub use scrollbar::{
    scrollbar, scrollbar_accessibility, scrollbar_thumb, ScrollAxis, ScrollbarControllerState,
    ScrollbarDragState, ScrollbarOptions,
};
pub use separator::*;
pub use slider::*;
pub use spinner::*;
pub use table::*;
pub use text_input::*;
pub use theme_preference::*;
pub use toggle::*;
pub use tooltip::*;
pub use virtual_list::*;

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
