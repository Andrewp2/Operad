//! Tooltip placement shared by document layout and widget builders.

use crate::{UiPoint, UiRect, UiSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TooltipPlacement {
    Above,
    Below,
    Left,
    Right,
    Cursor,
}

impl Default for TooltipPlacement {
    fn default() -> Self {
        Self::Above
    }
}

// Placement happens in window coordinates. Convert the result back to authored
// layout units so the document applies UI scaling only once.
pub(crate) fn tooltip_layout_rect(
    scale: f32,
    anchor: UiRect,
    size: UiSize,
    viewport: UiRect,
    placement: TooltipPlacement,
    offset: f32,
    cursor: Option<UiPoint>,
) -> UiRect {
    let rect = tooltip_rect(
        anchor,
        UiSize::new(size.width * scale, size.height * scale),
        viewport,
        placement,
        offset * scale,
        cursor,
    );
    UiRect::new(
        rect.x / scale,
        rect.y / scale,
        rect.width / scale,
        rect.height / scale,
    )
}

/// Place a tooltip with every length expressed in the same coordinate units.
pub fn tooltip_rect(
    anchor: UiRect,
    tooltip_size: UiSize,
    viewport: UiRect,
    placement: TooltipPlacement,
    offset: f32,
    cursor: Option<UiPoint>,
) -> UiRect {
    let offset = finite_or(offset, 0.0).max(0.0);
    let tooltip_size = UiSize::new(
        finite_or(tooltip_size.width, 0.0).max(0.0),
        finite_or(tooltip_size.height, 0.0).max(0.0),
    );
    let origin = tooltip_origin(anchor, tooltip_size, viewport, placement, offset, cursor);
    UiRect::new(
        clamp_tooltip_axis(origin.x, tooltip_size.width, viewport.x, viewport.right()),
        clamp_tooltip_axis(origin.y, tooltip_size.height, viewport.y, viewport.bottom()),
        tooltip_size.width,
        tooltip_size.height,
    )
}

fn tooltip_origin(
    anchor: UiRect,
    tooltip_size: UiSize,
    viewport: UiRect,
    placement: TooltipPlacement,
    offset: f32,
    cursor: Option<UiPoint>,
) -> UiPoint {
    match placement {
        TooltipPlacement::Above => {
            let above = anchor.y - tooltip_size.height - offset;
            let below = anchor.bottom() + offset;
            let above_space = tooltip_side_space(viewport.y, anchor.y, offset);
            let below_space = tooltip_side_space(anchor.bottom(), viewport.bottom(), offset);
            if above_space < tooltip_size.height && below_space > above_space {
                UiPoint::new(anchor.x, below)
            } else {
                UiPoint::new(anchor.x, above)
            }
        }
        TooltipPlacement::Below => {
            let below = anchor.bottom() + offset;
            let above = anchor.y - tooltip_size.height - offset;
            let below_space = tooltip_side_space(anchor.bottom(), viewport.bottom(), offset);
            let above_space = tooltip_side_space(viewport.y, anchor.y, offset);
            if below_space < tooltip_size.height && above_space > below_space {
                UiPoint::new(anchor.x, above)
            } else {
                UiPoint::new(anchor.x, below)
            }
        }
        TooltipPlacement::Left => {
            let left = anchor.x - tooltip_size.width - offset;
            let right = anchor.right() + offset;
            let left_space = tooltip_side_space(viewport.x, anchor.x, offset);
            let right_space = tooltip_side_space(anchor.right(), viewport.right(), offset);
            if left_space < tooltip_size.width && right_space > left_space {
                UiPoint::new(right, anchor.y)
            } else {
                UiPoint::new(left, anchor.y)
            }
        }
        TooltipPlacement::Right => {
            let right = anchor.right() + offset;
            let left = anchor.x - tooltip_size.width - offset;
            let right_space = tooltip_side_space(anchor.right(), viewport.right(), offset);
            let left_space = tooltip_side_space(viewport.x, anchor.x, offset);
            if right_space < tooltip_size.width && left_space > right_space {
                UiPoint::new(left, anchor.y)
            } else {
                UiPoint::new(right, anchor.y)
            }
        }
        TooltipPlacement::Cursor => cursor
            .map(|point| UiPoint::new(point.x + offset, point.y + offset))
            .unwrap_or_else(|| UiPoint::new(anchor.right() + offset, anchor.bottom() + offset)),
    }
}

fn tooltip_side_space(start: f32, end: f32, offset: f32) -> f32 {
    (end - start - offset).max(0.0)
}

fn clamp_tooltip_axis(value: f32, extent: f32, min: f32, max: f32) -> f32 {
    let min = finite_or(min, 0.0);
    let max = finite_or(max, min).max(min);
    let extent = finite_or(extent, 0.0).max(0.0);
    let upper = (max - extent).max(min);
    finite_or(value, min).clamp(min, upper)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}
