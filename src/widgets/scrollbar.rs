use std::collections::HashMap;

use super::*;

#[derive(Debug, Clone)]
pub struct ScrollbarOptions {
    pub layout: LayoutStyle,
    pub track_size: UiSize,
    pub track_visual: UiVisual,
    pub thumb_visual: UiVisual,
    pub disabled_thumb_visual: UiVisual,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
}

impl Default for ScrollbarOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::size(8.0, 120.0),
            track_size: UiSize::new(8.0, 120.0),
            track_visual: UiVisual::panel(
                ColorRgba::new(28, 34, 42, 255),
                Some(StrokeStyle::new(ColorRgba::new(54, 65, 81, 255), 1.0)),
                4.0,
            ),
            thumb_visual: UiVisual::panel(ColorRgba::new(103, 119, 143, 255), None, 4.0),
            disabled_thumb_visual: UiVisual::panel(ColorRgba::new(72, 82, 98, 120), None, 4.0),
            action: None,
            accessibility_label: None,
        }
    }
}

impl ScrollbarOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_track_size(mut self, size: UiSize) -> Self {
        self.track_size = size;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }
}

pub fn scrollbar(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    scroll: ScrollState,
    axis: ScrollAxis,
    options: ScrollbarOptions,
) -> UiNodeId {
    let name = name.into();
    let track = UiRect::new(
        0.0,
        0.0,
        options.track_size.width.max(0.0),
        options.track_size.height.max(0.0),
    );
    let thumb = scrollbar_thumb(scroll, track, axis);
    let max_offset = axis.value(scroll.max_offset());
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_visual(options.track_visual)
    .with_accessibility(scrollbar_accessibility(
        options
            .accessibility_label
            .clone()
            .unwrap_or_else(|| format!("{name} {}", axis.label())),
        scroll,
        axis,
    ));
    if max_offset > f32::EPSILON {
        node = node.with_input(InputBehavior::BUTTON);
        if let Some(action) = options.action.clone() {
            node = node.with_pointer_edit_action(action);
        }
    }
    let root = document.add_child(parent, node);
    document.add_child(
        root,
        UiNode::container(format!("{name}.thumb"), LayoutStyle::absolute_rect(thumb)).with_visual(
            if max_offset > f32::EPSILON {
                options.thumb_visual
            } else {
                options.disabled_thumb_visual
            },
        ),
    );
    root
}

pub fn scrollbar_thumb(scroll: ScrollState, track: UiRect, axis: ScrollAxis) -> UiRect {
    match axis {
        ScrollAxis::Vertical => {
            if track.height <= f32::EPSILON || track.width <= f32::EPSILON {
                return UiRect::new(track.x, track.y, 0.0, 0.0);
            }
            let ratio =
                scrollbar_viewport_ratio(scroll.viewport_size.height, scroll.content_size.height);
            let height = track.height * ratio;
            let max_offset = scroll.max_offset().y;
            let offset_ratio = if max_offset <= f32::EPSILON {
                0.0
            } else {
                (scroll.offset.y / max_offset).clamp(0.0, 1.0)
            };
            let y = track.y + (track.height - height) * offset_ratio;
            UiRect::new(track.x, y, track.width, height)
        }
        ScrollAxis::Horizontal => {
            if track.width <= f32::EPSILON || track.height <= f32::EPSILON {
                return UiRect::new(track.x, track.y, 0.0, 0.0);
            }
            let ratio =
                scrollbar_viewport_ratio(scroll.viewport_size.width, scroll.content_size.width);
            let width = track.width * ratio;
            let max_offset = scroll.max_offset().x;
            let offset_ratio = if max_offset <= f32::EPSILON {
                0.0
            } else {
                (scroll.offset.x / max_offset).clamp(0.0, 1.0)
            };
            let x = track.x + (track.width - width) * offset_ratio;
            UiRect::new(x, track.y, width, track.height)
        }
    }
}

pub fn scrollbar_accessibility(
    label: impl Into<String>,
    scroll: ScrollState,
    axis: ScrollAxis,
) -> AccessibilityMeta {
    let (offset, max_offset) = match axis {
        ScrollAxis::Vertical => (scroll.offset.y, scroll.max_offset().y),
        ScrollAxis::Horizontal => (scroll.offset.x, scroll.max_offset().x),
    };
    let percent = if max_offset <= f32::EPSILON {
        100.0
    } else {
        (offset / max_offset * 100.0).clamp(0.0, 100.0)
    };
    let accessibility = AccessibilityMeta::new(AccessibilityRole::Slider)
        .label(label)
        .value(format!("{percent:.0}%"))
        .value_range(AccessibilityValueRange::new(
            0.0,
            max_offset.max(0.0) as f64,
        ))
        .action(AccessibilityAction::new(
            "scroll_backward",
            "Scroll backward",
        ))
        .action(AccessibilityAction::new("scroll_forward", "Scroll forward"));
    if max_offset <= f32::EPSILON {
        accessibility.disabled()
    } else {
        accessibility.focusable()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarDragState {
    pub axis: ScrollAxis,
    pub track: UiRect,
    pub thumb: UiRect,
    pub pointer_start: UiPoint,
    pub offset_start: UiPoint,
    pub max_offset: UiPoint,
}

#[derive(Debug, Clone, Default)]
pub struct ScrollbarControllerState {
    drags: HashMap<String, ScrollbarDragState>,
}

impl ScrollbarControllerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self, id: &str) {
        self.drags.remove(id);
    }

    pub fn apply_drag(
        &mut self,
        id: impl Into<String>,
        scroll: ScrollState,
        track: UiRect,
        axis: ScrollAxis,
        edit: WidgetPointerEdit,
    ) -> UiPoint {
        let id = id.into();
        match edit.phase.edit_phase() {
            EditPhase::Preview => scroll.offset,
            EditPhase::BeginEdit => {
                if let Some(drag) = ScrollbarDragState::new(scroll, track, axis, edit.position) {
                    self.drags.insert(id, drag);
                    drag.offset_for_pointer(edit.position)
                } else {
                    scroll.offset
                }
            }
            EditPhase::UpdateEdit | EditPhase::CommitEdit => {
                let drag = self
                    .drags
                    .get(&id)
                    .copied()
                    .or_else(|| ScrollbarDragState::new(scroll, track, axis, edit.position));
                let offset = drag
                    .map(|drag| drag.offset_for_pointer(edit.position))
                    .unwrap_or(scroll.offset);
                if edit.phase.edit_phase() == EditPhase::CommitEdit {
                    self.drags.remove(&id);
                }
                offset
            }
            EditPhase::CancelEdit => {
                self.drags.remove(&id);
                scroll.offset
            }
        }
    }

    pub fn apply_drag_for_target_rect(
        &mut self,
        id: impl Into<String>,
        scroll: ScrollState,
        axis: ScrollAxis,
        edit: WidgetPointerEdit,
    ) -> UiPoint {
        self.apply_drag(
            id,
            scroll,
            UiRect::new(0.0, 0.0, edit.target_rect.width, edit.target_rect.height),
            axis,
            edit,
        )
    }
}

impl ScrollbarDragState {
    pub fn new(
        scroll: ScrollState,
        track: UiRect,
        axis: ScrollAxis,
        pointer_start: UiPoint,
    ) -> Option<Self> {
        let thumb = scrollbar_thumb(scroll, track, axis);
        let max_offset = scroll.max_offset();
        let travel = scrollbar_thumb_travel(track, thumb, axis);
        let axis_max_offset = axis.value(max_offset);
        (travel > f32::EPSILON && axis_max_offset > f32::EPSILON).then_some(Self {
            axis,
            track,
            thumb,
            pointer_start,
            offset_start: scroll.offset,
            max_offset,
        })
    }

    pub fn offset_for_pointer(self, pointer: UiPoint) -> UiPoint {
        let travel = scrollbar_thumb_travel(self.track, self.thumb, self.axis);
        if travel <= f32::EPSILON {
            return self.offset_start;
        }
        let pointer_delta = self.axis.value(pointer) - self.axis.value(self.pointer_start);
        let max_axis_offset = self.axis.value(self.max_offset);
        let offset_delta = pointer_delta / travel * max_axis_offset;
        let offset = self.axis.with_value(
            self.offset_start,
            self.axis.value(self.offset_start) + offset_delta,
        );
        UiPoint::new(
            offset.x.clamp(0.0, self.max_offset.x),
            offset.y.clamp(0.0, self.max_offset.y),
        )
    }

    pub fn scroll_state_for_pointer(
        self,
        mut scroll: ScrollState,
        pointer: UiPoint,
    ) -> ScrollState {
        scroll.offset = scroll.clamp_offset(self.offset_for_pointer(pointer));
        scroll
    }
}

fn scrollbar_thumb_travel(track: UiRect, thumb: UiRect, axis: ScrollAxis) -> f32 {
    match axis {
        ScrollAxis::Vertical => (track.height - thumb.height).max(0.0),
        ScrollAxis::Horizontal => (track.width - thumb.width).max(0.0),
    }
}

fn scrollbar_viewport_ratio(viewport: f32, content: f32) -> f32 {
    if viewport <= f32::EPSILON || content <= viewport {
        1.0
    } else {
        (viewport / content).clamp(0.05, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxis {
    Vertical,
    Horizontal,
}

impl ScrollAxis {
    pub const fn value(self, point: UiPoint) -> f32 {
        match self {
            Self::Vertical => point.y,
            Self::Horizontal => point.x,
        }
    }

    pub const fn with_value(self, point: UiPoint, value: f32) -> UiPoint {
        match self {
            Self::Vertical => UiPoint::new(point.x, value),
            Self::Horizontal => UiPoint::new(value, point.y),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Vertical => "vertical scrollbar",
            Self::Horizontal => "horizontal scrollbar",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(phase: WidgetValueEditPhase, y: f32, target_height: f32) -> WidgetPointerEdit {
        WidgetPointerEdit::new(
            phase,
            UiPoint::new(4.0, y),
            UiPoint::new(4.0, y),
            UiRect::new(0.0, 0.0, 8.0, target_height),
        )
    }

    #[test]
    fn scrollbar_controller_tracks_pointer_drag_by_id() {
        let scroll = ScrollState {
            axes: ScrollAxes::VERTICAL,
            offset: UiPoint::new(0.0, 0.0),
            viewport_size: UiSize::new(8.0, 50.0),
            content_size: UiSize::new(8.0, 100.0),
        };
        let mut controller = ScrollbarControllerState::new();

        assert_eq!(
            controller.apply_drag_for_target_rect(
                "list",
                scroll,
                ScrollAxis::Vertical,
                edit(WidgetValueEditPhase::Begin, 0.0, 100.0),
            ),
            UiPoint::new(0.0, 0.0)
        );
        assert_eq!(
            controller.apply_drag_for_target_rect(
                "list",
                scroll,
                ScrollAxis::Vertical,
                edit(WidgetValueEditPhase::Update, 25.0, 100.0),
            ),
            UiPoint::new(0.0, 25.0)
        );
        assert_eq!(
            controller.apply_drag_for_target_rect(
                "list",
                scroll,
                ScrollAxis::Vertical,
                edit(WidgetValueEditPhase::Commit, 50.0, 100.0),
            ),
            UiPoint::new(0.0, 50.0)
        );
    }
}
