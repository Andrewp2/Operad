//! Scale-aware popup placement shared by layout constraints and widgets.

use crate::{UiRect, UiSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupSide {
    Top,
    Bottom,
    Left,
    Right,
}

impl PopupSide {
    pub const fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupAlign {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupPlacement {
    pub side: PopupSide,
    pub align: PopupAlign,
    pub offset: f32,
    pub viewport_margin: f32,
    pub flip: bool,
    pub constrain_to_viewport: bool,
}

impl PopupPlacement {
    pub const fn new(side: PopupSide, align: PopupAlign) -> Self {
        Self {
            side,
            align,
            offset: 4.0,
            viewport_margin: 4.0,
            flip: true,
            constrain_to_viewport: true,
        }
    }

    pub const fn with_offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    pub const fn with_viewport_margin(mut self, margin: f32) -> Self {
        self.viewport_margin = margin;
        self
    }

    pub const fn with_flip(mut self, flip: bool) -> Self {
        self.flip = flip;
        self
    }

    pub const fn with_viewport_constraint(mut self, constrain: bool) -> Self {
        self.constrain_to_viewport = constrain;
        self
    }
}

impl Default for PopupPlacement {
    fn default() -> Self {
        Self::new(PopupSide::Bottom, PopupAlign::Start)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupLayout {
    pub rect: UiRect,
    pub side: PopupSide,
    pub flipped: bool,
    pub primary_rect: UiRect,
    pub unconstrained_rect: UiRect,
    pub overflow_before_constrain: f32,
    pub overflow_after_constrain: f32,
    pub constrained: bool,
}

impl PopupLayout {
    pub fn diagnostic_summary(&self) -> String {
        format!(
            "popup placement side={:?} flipped={} constrained={} primary={:?} unconstrained={:?} final={:?} overflow_before={:.2} overflow_after={:.2}",
            self.side,
            self.flipped,
            self.constrained,
            self.primary_rect,
            self.unconstrained_rect,
            self.rect,
            self.overflow_before_constrain,
            self.overflow_after_constrain
        )
    }
}

/// Popup geometry in logical coordinates relative to the destination portal.
///
/// For app-overlay portals, `anchor` and `viewport` use window coordinates.
/// Parent and named portals use coordinates local to their layout host. Widget
/// sizes, placement offsets, and viewport margins use authored UI units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnchoredPopup {
    pub anchor: UiRect,
    pub viewport: UiRect,
    pub placement: PopupPlacement,
}

impl AnchoredPopup {
    pub const fn new(anchor: UiRect, viewport: UiRect, placement: PopupPlacement) -> Self {
        Self {
            anchor,
            viewport,
            placement,
        }
    }

    pub(crate) fn layout_rect(self, scale: f32, size: UiSize) -> UiRect {
        let rect = place_popup(
            self.anchor,
            UiSize::new(size.width * scale, size.height * scale),
            self.viewport,
            PopupPlacement {
                offset: self.placement.offset * scale,
                viewport_margin: self.placement.viewport_margin * scale,
                ..self.placement
            },
        )
        .rect;
        // Layout applies UI scaling, so return authored units after placing
        // and constraining the popup using its actual logical dimensions.
        UiRect::new(
            rect.x / scale,
            rect.y / scale,
            rect.width / scale,
            rect.height / scale,
        )
    }
}

/// Place a popup with every length expressed in the same coordinate units.
pub fn place_popup(
    anchor: UiRect,
    popup_size: UiSize,
    viewport: UiRect,
    placement: PopupPlacement,
) -> PopupLayout {
    let inner_viewport = super::inset_rect(viewport, placement.viewport_margin.max(0.0));
    let primary = popup_rect_for_anchor(anchor, popup_size, placement.side, placement);
    let mut rect = primary;
    let mut side = placement.side;
    let mut flipped = false;

    if placement.flip {
        let opposite_side = placement.side.opposite();
        let opposite = popup_rect_for_anchor(anchor, popup_size, opposite_side, placement);
        if super::rect_overflow_amount(opposite, inner_viewport)
            < super::rect_overflow_amount(primary, inner_viewport)
        {
            rect = opposite;
            side = opposite_side;
            flipped = true;
        }
    }

    let unconstrained_rect = rect;
    let overflow_before_constrain = super::rect_overflow_amount(unconstrained_rect, inner_viewport);
    if placement.constrain_to_viewport {
        rect = super::contain_rect(rect, inner_viewport, UiSize::ZERO);
    }
    let overflow_after_constrain = super::rect_overflow_amount(rect, inner_viewport);

    PopupLayout {
        rect,
        side,
        flipped,
        primary_rect: primary,
        unconstrained_rect,
        overflow_before_constrain,
        overflow_after_constrain,
        constrained: rect != unconstrained_rect,
    }
}

pub fn centered_popup_rect(viewport: UiRect, popup_size: UiSize, viewport_margin: f32) -> UiRect {
    let inner = super::inset_rect(viewport, viewport_margin.max(0.0));
    super::contain_rect(
        UiRect::new(
            inner.x + (inner.width - popup_size.width) * 0.5,
            inner.y + (inner.height - popup_size.height) * 0.5,
            popup_size.width,
            popup_size.height,
        ),
        inner,
        UiSize::ZERO,
    )
}

fn popup_rect_for_anchor(
    anchor: UiRect,
    popup_size: UiSize,
    side: PopupSide,
    placement: PopupPlacement,
) -> UiRect {
    let offset = placement.offset.max(0.0);
    match side {
        PopupSide::Top => UiRect::new(
            aligned_x(anchor, popup_size.width, placement.align),
            anchor.y - popup_size.height - offset,
            popup_size.width,
            popup_size.height,
        ),
        PopupSide::Bottom => UiRect::new(
            aligned_x(anchor, popup_size.width, placement.align),
            anchor.bottom() + offset,
            popup_size.width,
            popup_size.height,
        ),
        PopupSide::Left => UiRect::new(
            anchor.x - popup_size.width - offset,
            aligned_y(anchor, popup_size.height, placement.align),
            popup_size.width,
            popup_size.height,
        ),
        PopupSide::Right => UiRect::new(
            anchor.right() + offset,
            aligned_y(anchor, popup_size.height, placement.align),
            popup_size.width,
            popup_size.height,
        ),
    }
}

fn aligned_x(anchor: UiRect, width: f32, align: PopupAlign) -> f32 {
    match align {
        PopupAlign::Start => anchor.x,
        PopupAlign::Center => anchor.x + (anchor.width - width) * 0.5,
        PopupAlign::End => anchor.right() - width,
    }
}

fn aligned_y(anchor: UiRect, height: f32, align: PopupAlign) -> f32 {
    match align {
        PopupAlign::Start => anchor.y,
        PopupAlign::Center => anchor.y + (anchor.height - height) * 0.5,
        PopupAlign::End => anchor.bottom() - height,
    }
}
