use std::ops::Range;

use crate::core::document::rect_is_finite;
use crate::platform::{
    ClipboardRequest, LogicalRect, PlatformRequest, TextImeRequest, TextImeResponse,
    TextImeSession, TextInputId, TextRange,
};
use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, Size as TaffySize, Style,
};

use super::*;

pub use crate::widget_ext::*;

pub mod button;
pub mod canvas;
pub mod checkbox;
pub mod collapsing;
pub mod combo_box;
pub mod drag_value;
pub mod grid;
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
pub mod toggle;
pub mod tooltip;
pub mod virtual_list;

pub use button::*;
pub use canvas::*;
pub use checkbox::*;
pub use collapsing::*;
pub use combo_box::*;
pub use drag_value::*;
pub use grid::*;
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
pub use toggle::*;
pub use tooltip::*;
pub use virtual_list::*;
