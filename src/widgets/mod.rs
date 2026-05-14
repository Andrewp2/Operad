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
pub mod combo_box;
pub mod label;
pub mod scroll_area;
pub mod scrollbar;
pub mod slider;
pub mod table;
pub mod text_input;
pub mod virtual_list;

pub use button::*;
pub use canvas::*;
pub use checkbox::*;
pub use combo_box::*;
pub use label::*;
pub use scroll_area::*;
pub use scrollbar::{
    scrollbar, scrollbar_accessibility, scrollbar_thumb, ScrollAxis, ScrollbarControllerState,
    ScrollbarDragState, ScrollbarOptions,
};
pub use slider::*;
pub use table::*;
pub use text_input::*;
pub use virtual_list::*;
