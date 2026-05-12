//! Backend-neutral contracts for advanced scrolling behavior.
//!
//! These helpers model v5 scrolling decisions without coupling them to a
//! renderer or host event loop. Existing document scrolling can adopt the
//! contracts incrementally while preserving deterministic tests.

use std::cmp::Ordering;

use crate::{ScrollAxes, ScrollState, UiNodeId, UiPoint, UiRect, UiSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollAxis {
    Horizontal,
    Vertical,
}

impl ScrollAxis {
    pub const fn value(self, point: UiPoint) -> f32 {
        match self {
            Self::Horizontal => point.x,
            Self::Vertical => point.y,
        }
    }

    pub const fn size_value(self, size: UiSize) -> f32 {
        match self {
            Self::Horizontal => size.width,
            Self::Vertical => size.height,
        }
    }

    pub const fn rect_start(self, rect: UiRect) -> f32 {
        match self {
            Self::Horizontal => rect.x,
            Self::Vertical => rect.y,
        }
    }

    pub fn rect_end(self, rect: UiRect) -> f32 {
        match self {
            Self::Horizontal => rect.right(),
            Self::Vertical => rect.bottom(),
        }
    }

    pub const fn with_value(self, point: UiPoint, value: f32) -> UiPoint {
        match self {
            Self::Horizontal => UiPoint::new(value, point.y),
            Self::Vertical => UiPoint::new(point.x, value),
        }
    }

    pub const fn enabled_by(self, axes: ScrollAxes) -> bool {
        match self {
            Self::Horizontal => axes.horizontal,
            Self::Vertical => axes.vertical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl ScrollInsets {
    pub const ZERO: Self = Self::uniform(0.0);

    pub const fn uniform(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn expand_rect(self, rect: UiRect) -> UiRect {
        UiRect::new(
            rect.x - self.left.max(0.0),
            rect.y - self.top.max(0.0),
            rect.width + self.left.max(0.0) + self.right.max(0.0),
            rect.height + self.top.max(0.0) + self.bottom.max(0.0),
        )
    }
}

impl Default for ScrollInsets {
    fn default() -> Self {
        Self::ZERO
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverscrollBehavior {
    Auto,
    Contain,
    None,
}

impl OverscrollBehavior {
    pub const fn allows_chain(self) -> bool {
        matches!(self, Self::Auto)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverscrollPolicy {
    pub horizontal: OverscrollBehavior,
    pub vertical: OverscrollBehavior,
}

impl OverscrollPolicy {
    pub const AUTO: Self = Self {
        horizontal: OverscrollBehavior::Auto,
        vertical: OverscrollBehavior::Auto,
    };

    pub const CONTAIN: Self = Self {
        horizontal: OverscrollBehavior::Contain,
        vertical: OverscrollBehavior::Contain,
    };

    pub const NONE: Self = Self {
        horizontal: OverscrollBehavior::None,
        vertical: OverscrollBehavior::None,
    };

    pub const fn new(horizontal: OverscrollBehavior, vertical: OverscrollBehavior) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    pub const fn behavior(self, axis: ScrollAxis) -> OverscrollBehavior {
        match axis {
            ScrollAxis::Horizontal => self.horizontal,
            ScrollAxis::Vertical => self.vertical,
        }
    }
}

impl Default for OverscrollPolicy {
    fn default() -> Self {
        Self::AUTO
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollDeltaResolution {
    pub offset_before: UiPoint,
    pub offset_after: UiPoint,
    pub consumed_delta: UiPoint,
    pub overscroll_delta: UiPoint,
}

pub fn resolve_scroll_delta(state: ScrollState, delta: UiPoint) -> ScrollDeltaResolution {
    let delta = finite_point(delta);
    let offset_before = finite_point(state.offset);
    let max_offset = state.max_offset();
    let desired = UiPoint::new(
        if state.axes.horizontal {
            offset_before.x + delta.x
        } else {
            offset_before.x
        },
        if state.axes.vertical {
            offset_before.y + delta.y
        } else {
            offset_before.y
        },
    );
    let offset_after = UiPoint::new(
        clamp_axis(desired.x, 0.0, max_offset.x),
        clamp_axis(desired.y, 0.0, max_offset.y),
    );
    let consumed_delta = UiPoint::new(
        offset_after.x - offset_before.x,
        offset_after.y - offset_before.y,
    );
    ScrollDeltaResolution {
        offset_before,
        offset_after,
        consumed_delta,
        overscroll_delta: UiPoint::new(delta.x - consumed_delta.x, delta.y - consumed_delta.y),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NestedScrollCandidate {
    pub node: UiNodeId,
    pub state: ScrollState,
    pub overscroll: OverscrollPolicy,
}

impl NestedScrollCandidate {
    pub const fn new(node: UiNodeId, state: ScrollState) -> Self {
        Self {
            node,
            state,
            overscroll: OverscrollPolicy::AUTO,
        }
    }

    pub const fn overscroll(mut self, overscroll: OverscrollPolicy) -> Self {
        self.overscroll = overscroll;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NestedScrollStep {
    pub node: UiNodeId,
    pub input_delta: UiPoint,
    pub consumed_delta: UiPoint,
    pub overscroll_delta: UiPoint,
    pub offset_before: UiPoint,
    pub offset_after: UiPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NestedScrollPlan {
    pub steps: Vec<NestedScrollStep>,
    pub remaining_delta: UiPoint,
    pub blocked_delta: UiPoint,
}

impl NestedScrollPlan {
    pub fn changed_nodes(&self) -> Vec<UiNodeId> {
        self.steps
            .iter()
            .filter(|step| point_has_delta(step.consumed_delta))
            .map(|step| step.node)
            .collect()
    }

    pub fn offset_after(&self, node: UiNodeId) -> Option<UiPoint> {
        self.steps
            .iter()
            .find(|step| step.node == node)
            .map(|step| step.offset_after)
    }
}

pub fn arbitrate_nested_scroll(
    delta: UiPoint,
    candidates: &[NestedScrollCandidate],
) -> NestedScrollPlan {
    let mut remaining_delta = finite_point(delta);
    let mut blocked_delta = UiPoint::new(0.0, 0.0);
    let mut steps = Vec::new();

    for candidate in candidates {
        if !point_has_delta(remaining_delta) {
            break;
        }
        let resolution = resolve_scroll_delta(candidate.state, remaining_delta);
        let mut chained_delta = UiPoint::new(0.0, 0.0);
        let mut locally_blocked = UiPoint::new(0.0, 0.0);

        for axis in [ScrollAxis::Horizontal, ScrollAxis::Vertical] {
            let overscroll = axis.value(resolution.overscroll_delta);
            if overscroll.abs() <= f32::EPSILON {
                continue;
            }
            if candidate.overscroll.behavior(axis).allows_chain() {
                chained_delta = axis.with_value(chained_delta, overscroll);
            } else {
                locally_blocked = axis.with_value(locally_blocked, overscroll);
            }
        }

        blocked_delta = UiPoint::new(
            blocked_delta.x + locally_blocked.x,
            blocked_delta.y + locally_blocked.y,
        );
        steps.push(NestedScrollStep {
            node: candidate.node,
            input_delta: remaining_delta,
            consumed_delta: resolution.consumed_delta,
            overscroll_delta: resolution.overscroll_delta,
            offset_before: resolution.offset_before,
            offset_after: resolution.offset_after,
        });
        remaining_delta = chained_delta;
    }

    NestedScrollPlan {
        steps,
        remaining_delta,
        blocked_delta,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RevealAlignment {
    Nearest,
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgrammaticScrollBehavior {
    Instant,
    Smooth { duration_ms: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RevealOptions {
    pub inline: RevealAlignment,
    pub block: RevealAlignment,
    pub margin: ScrollInsets,
    pub behavior: ProgrammaticScrollBehavior,
}

impl RevealOptions {
    pub const fn nearest() -> Self {
        Self {
            inline: RevealAlignment::Nearest,
            block: RevealAlignment::Nearest,
            margin: ScrollInsets::ZERO,
            behavior: ProgrammaticScrollBehavior::Instant,
        }
    }

    pub const fn align(inline: RevealAlignment, block: RevealAlignment) -> Self {
        Self {
            inline,
            block,
            margin: ScrollInsets::ZERO,
            behavior: ProgrammaticScrollBehavior::Instant,
        }
    }

    pub const fn margin(mut self, margin: ScrollInsets) -> Self {
        self.margin = margin;
        self
    }

    pub const fn behavior(mut self, behavior: ProgrammaticScrollBehavior) -> Self {
        self.behavior = behavior;
        self
    }
}

impl Default for RevealOptions {
    fn default() -> Self {
        Self::nearest()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RevealScrollPlan {
    pub offset_before: UiPoint,
    pub offset_after: UiPoint,
    pub delta: UiPoint,
    pub behavior: ProgrammaticScrollBehavior,
}

impl RevealScrollPlan {
    pub fn changed(self) -> bool {
        point_has_delta(self.delta)
    }
}

pub fn reveal_rect_into_view(
    scroll: ScrollState,
    target_rect: UiRect,
    options: RevealOptions,
) -> RevealScrollPlan {
    let target = options.margin.expand_rect(target_rect);
    let max = scroll.max_offset();
    let mut offset = finite_point(scroll.offset);

    if scroll.axes.horizontal {
        offset.x = reveal_axis_offset(
            offset.x,
            scroll.viewport_size.width,
            max.x,
            target.x,
            target.right(),
            options.inline,
        );
    }
    if scroll.axes.vertical {
        offset.y = reveal_axis_offset(
            offset.y,
            scroll.viewport_size.height,
            max.y,
            target.y,
            target.bottom(),
            options.block,
        );
    }

    let offset_after = UiPoint::new(
        clamp_axis(offset.x, 0.0, max.x),
        clamp_axis(offset.y, 0.0, max.y),
    );
    let offset_before = finite_point(scroll.offset);
    RevealScrollPlan {
        offset_before,
        offset_after,
        delta: UiPoint::new(
            offset_after.x - offset_before.x,
            offset_after.y - offset_before.y,
        ),
        behavior: options.behavior,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollAnchorCandidate {
    pub node: UiNodeId,
    pub rect_before: UiRect,
    pub rect_after: UiRect,
    pub priority: i32,
    pub order: usize,
    pub enabled: bool,
}

impl ScrollAnchorCandidate {
    pub const fn new(node: UiNodeId, rect_before: UiRect, rect_after: UiRect) -> Self {
        Self {
            node,
            rect_before,
            rect_after,
            priority: 0,
            order: 0,
            enabled: true,
        }
    }

    pub const fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub const fn order(mut self, order: usize) -> Self {
        self.order = order;
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollAnchorAdjustment {
    pub anchor: Option<UiNodeId>,
    pub offset_before: UiPoint,
    pub offset_after: UiPoint,
    pub delta: UiPoint,
}

pub fn apply_scroll_anchor(
    scroll: ScrollState,
    candidates: &[ScrollAnchorCandidate],
) -> ScrollAnchorAdjustment {
    let offset_before = finite_point(scroll.offset);
    let Some(anchor) = select_scroll_anchor(scroll, candidates) else {
        return ScrollAnchorAdjustment {
            anchor: None,
            offset_before,
            offset_after: offset_before,
            delta: UiPoint::new(0.0, 0.0),
        };
    };

    let mut delta = UiPoint::new(0.0, 0.0);
    if scroll.axes.horizontal {
        delta.x = anchor.rect_after.x - anchor.rect_before.x;
    }
    if scroll.axes.vertical {
        delta.y = anchor.rect_after.y - anchor.rect_before.y;
    }
    let max = scroll.max_offset();
    let offset_after = UiPoint::new(
        clamp_axis(offset_before.x + delta.x, 0.0, max.x),
        clamp_axis(offset_before.y + delta.y, 0.0, max.y),
    );

    ScrollAnchorAdjustment {
        anchor: Some(anchor.node),
        offset_before,
        offset_after,
        delta: UiPoint::new(
            offset_after.x - offset_before.x,
            offset_after.y - offset_before.y,
        ),
    }
}

pub fn select_scroll_anchor<'a>(
    scroll: ScrollState,
    candidates: &'a [ScrollAnchorCandidate],
) -> Option<&'a ScrollAnchorCandidate> {
    let viewport = content_viewport_rect(scroll);
    candidates
        .iter()
        .filter(|candidate| candidate.enabled)
        .filter_map(|candidate| {
            let visible_area = candidate
                .rect_before
                .intersection(viewport)
                .map(rect_area)
                .unwrap_or(0.0);
            (visible_area > f32::EPSILON).then_some((candidate, visible_area))
        })
        .max_by(|left, right| compare_anchor_candidate(*left, *right))
        .map(|(candidate, _)| candidate)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickyEdges {
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,
}

impl StickyEdges {
    pub const NONE: Self = Self {
        top: None,
        right: None,
        bottom: None,
        left: None,
    };

    pub const fn top(value: f32) -> Self {
        Self {
            top: Some(value),
            right: None,
            bottom: None,
            left: None,
        }
    }

    pub const fn left(value: f32) -> Self {
        Self {
            top: None,
            right: None,
            bottom: None,
            left: Some(value),
        }
    }
}

impl Default for StickyEdges {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickyConstraints {
    pub scrollport: UiRect,
    pub containing_block: UiRect,
    pub edges: StickyEdges,
}

impl StickyConstraints {
    pub const fn new(scrollport: UiRect, containing_block: UiRect, edges: StickyEdges) -> Self {
        Self {
            scrollport,
            containing_block,
            edges,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedConstraints {
    pub viewport: UiRect,
    pub viewport_rect: UiRect,
}

impl FixedConstraints {
    pub const fn new(viewport: UiRect, viewport_rect: UiRect) -> Self {
        Self {
            viewport,
            viewport_rect,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollAttachment {
    Flow,
    Sticky(StickyConstraints),
    Fixed(FixedConstraints),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollAttachmentMode {
    Flow,
    Stuck,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollAttachmentResolution {
    pub mode: ScrollAttachmentMode,
    pub rect: UiRect,
    pub delta: UiPoint,
}

pub fn resolve_scroll_attachment(
    normal_rect: UiRect,
    attachment: ScrollAttachment,
) -> ScrollAttachmentResolution {
    match attachment {
        ScrollAttachment::Flow => ScrollAttachmentResolution {
            mode: ScrollAttachmentMode::Flow,
            rect: normal_rect,
            delta: UiPoint::new(0.0, 0.0),
        },
        ScrollAttachment::Sticky(constraints) => resolve_sticky_rect(normal_rect, constraints),
        ScrollAttachment::Fixed(constraints) => {
            let rect = UiRect::new(
                constraints.viewport.x + constraints.viewport_rect.x,
                constraints.viewport.y + constraints.viewport_rect.y,
                constraints.viewport_rect.width,
                constraints.viewport_rect.height,
            );
            ScrollAttachmentResolution {
                mode: ScrollAttachmentMode::Fixed,
                rect,
                delta: UiPoint::new(rect.x - normal_rect.x, rect.y - normal_rect.y),
            }
        }
    }
}

pub fn resolve_sticky_rect(
    normal_rect: UiRect,
    constraints: StickyConstraints,
) -> ScrollAttachmentResolution {
    let mut x = normal_rect.x;
    let mut y = normal_rect.y;
    if let Some(left) = constraints.edges.left {
        x = x.max(constraints.scrollport.x + left.max(0.0));
    }
    if let Some(right) = constraints.edges.right {
        x = x.min(constraints.scrollport.right() - right.max(0.0) - normal_rect.width);
    }
    if let Some(top) = constraints.edges.top {
        y = y.max(constraints.scrollport.y + top.max(0.0));
    }
    if let Some(bottom) = constraints.edges.bottom {
        y = y.min(constraints.scrollport.bottom() - bottom.max(0.0) - normal_rect.height);
    }

    x = clamp_axis(
        x,
        constraints.containing_block.x,
        constraints.containing_block.right() - normal_rect.width,
    );
    y = clamp_axis(
        y,
        constraints.containing_block.y,
        constraints.containing_block.bottom() - normal_rect.height,
    );

    let rect = UiRect::new(x, y, normal_rect.width, normal_rect.height);
    let delta = UiPoint::new(rect.x - normal_rect.x, rect.y - normal_rect.y);
    ScrollAttachmentResolution {
        mode: if point_has_delta(delta) {
            ScrollAttachmentMode::Stuck
        } else {
            ScrollAttachmentMode::Flow
        },
        rect,
        delta,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollbarVisibility {
    Auto,
    Always,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarPolicy {
    pub visibility: ScrollbarVisibility,
    pub thickness: f32,
    pub min_thumb_length: f32,
}

impl ScrollbarPolicy {
    pub const DEFAULT: Self = Self {
        visibility: ScrollbarVisibility::Auto,
        thickness: 10.0,
        min_thumb_length: 18.0,
    };
}

impl Default for ScrollbarPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub fn scrollbar_thumb_rect(
    scroll: ScrollState,
    track: UiRect,
    axis: ScrollAxis,
    policy: ScrollbarPolicy,
) -> Option<UiRect> {
    if matches!(policy.visibility, ScrollbarVisibility::Hidden) {
        return None;
    }
    let viewport = axis.size_value(scroll.viewport_size);
    let content = axis.size_value(scroll.content_size);
    if content <= viewport && matches!(policy.visibility, ScrollbarVisibility::Auto) {
        return None;
    }
    let track_length = axis.rect_end(track) - axis.rect_start(track);
    if track_length <= f32::EPSILON {
        return None;
    }
    let max_offset = axis.value(scroll.max_offset());
    let ratio = if viewport <= f32::EPSILON || content <= viewport {
        1.0
    } else {
        (viewport / content).clamp(0.0, 1.0)
    };
    let thumb_length = (track_length * ratio).max(policy.min_thumb_length.min(track_length));
    let travel = (track_length - thumb_length).max(0.0);
    let offset_ratio = if max_offset <= f32::EPSILON {
        0.0
    } else {
        (axis.value(scroll.offset) / max_offset).clamp(0.0, 1.0)
    };
    match axis {
        ScrollAxis::Horizontal => Some(UiRect::new(
            track.x + travel * offset_ratio,
            track.y,
            thumb_length,
            policy.thickness.min(track.height),
        )),
        ScrollAxis::Vertical => Some(UiRect::new(
            track.x,
            track.y + travel * offset_ratio,
            policy.thickness.min(track.width),
            thumb_length,
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KineticScrollConfig {
    pub deceleration: f32,
    pub min_velocity: f32,
}

impl KineticScrollConfig {
    pub const DEFAULT: Self = Self {
        deceleration: 2800.0,
        min_velocity: 4.0,
    };
}

impl Default for KineticScrollConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KineticScrollState {
    pub velocity: UiPoint,
    pub active: bool,
}

impl KineticScrollState {
    pub const IDLE: Self = Self {
        velocity: UiPoint::new(0.0, 0.0),
        active: false,
    };

    pub const fn new(velocity: UiPoint) -> Self {
        Self {
            velocity,
            active: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KineticScrollStep {
    pub state: KineticScrollState,
    pub delta: UiPoint,
}

pub fn step_kinetic_scroll(
    state: KineticScrollState,
    elapsed_seconds: f32,
    config: KineticScrollConfig,
) -> KineticScrollStep {
    if !state.active || elapsed_seconds <= 0.0 || !elapsed_seconds.is_finite() {
        return KineticScrollStep {
            state,
            delta: UiPoint::new(0.0, 0.0),
        };
    }
    let next_velocity = UiPoint::new(
        decay_velocity(state.velocity.x, elapsed_seconds, config),
        decay_velocity(state.velocity.y, elapsed_seconds, config),
    );
    let delta = UiPoint::new(
        (state.velocity.x + next_velocity.x) * 0.5 * elapsed_seconds,
        (state.velocity.y + next_velocity.y) * 0.5 * elapsed_seconds,
    );
    let active =
        next_velocity.x.abs() > config.min_velocity || next_velocity.y.abs() > config.min_velocity;
    KineticScrollStep {
        state: KineticScrollState {
            velocity: if active {
                next_velocity
            } else {
                UiPoint::new(0.0, 0.0)
            },
            active,
        },
        delta,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollSyncMode {
    ExactOffset,
    Proportional,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollSyncSurface {
    pub node: UiNodeId,
    pub state: ScrollState,
}

impl ScrollSyncSurface {
    pub const fn new(node: UiNodeId, state: ScrollState) -> Self {
        Self { node, state }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollSyncUpdate {
    pub node: UiNodeId,
    pub offset_before: UiPoint,
    pub offset_after: UiPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScrollSyncPlan {
    pub updates: Vec<ScrollSyncUpdate>,
}

pub fn synchronize_scroll_surfaces(
    source: ScrollSyncSurface,
    followers: &[ScrollSyncSurface],
    axes: ScrollAxes,
    mode: ScrollSyncMode,
) -> ScrollSyncPlan {
    let source_max = source.state.max_offset();
    let source_progress_x = scroll_progress(source.state.offset.x, source_max.x);
    let source_progress_y = scroll_progress(source.state.offset.y, source_max.y);
    let updates = followers
        .iter()
        .filter_map(|follower| {
            let follower_max = follower.state.max_offset();
            let mut offset = follower.state.offset;
            if axes.horizontal && follower.state.axes.horizontal {
                offset.x = match mode {
                    ScrollSyncMode::ExactOffset => source.state.offset.x,
                    ScrollSyncMode::Proportional => follower_max.x * source_progress_x,
                };
            }
            if axes.vertical && follower.state.axes.vertical {
                offset.y = match mode {
                    ScrollSyncMode::ExactOffset => source.state.offset.y,
                    ScrollSyncMode::Proportional => follower_max.y * source_progress_y,
                };
            }
            offset = UiPoint::new(
                clamp_axis(offset.x, 0.0, follower_max.x),
                clamp_axis(offset.y, 0.0, follower_max.y),
            );
            (offset != follower.state.offset).then_some(ScrollSyncUpdate {
                node: follower.node,
                offset_before: follower.state.offset,
                offset_after: offset,
            })
        })
        .collect();
    ScrollSyncPlan { updates }
}

pub fn content_viewport_rect(scroll: ScrollState) -> UiRect {
    UiRect::new(
        finite_value(scroll.offset.x),
        finite_value(scroll.offset.y),
        scroll.viewport_size.width.max(0.0),
        scroll.viewport_size.height.max(0.0),
    )
}

fn reveal_axis_offset(
    current: f32,
    viewport_length: f32,
    max_offset: f32,
    target_start: f32,
    target_end: f32,
    alignment: RevealAlignment,
) -> f32 {
    let viewport_end = current + viewport_length;
    let offset = match alignment {
        RevealAlignment::Nearest => {
            if target_start < current {
                target_start
            } else if target_end > viewport_end {
                target_end - viewport_length
            } else {
                current
            }
        }
        RevealAlignment::Start => target_start,
        RevealAlignment::Center => (target_start + target_end - viewport_length) * 0.5,
        RevealAlignment::End => target_end - viewport_length,
    };
    clamp_axis(offset, 0.0, max_offset)
}

fn compare_anchor_candidate(
    left: (&ScrollAnchorCandidate, f32),
    right: (&ScrollAnchorCandidate, f32),
) -> Ordering {
    left.0
        .priority
        .cmp(&right.0.priority)
        .then_with(|| left.1.total_cmp(&right.1))
        .then_with(|| right.0.order.cmp(&left.0.order))
}

fn decay_velocity(value: f32, elapsed_seconds: f32, config: KineticScrollConfig) -> f32 {
    if value.abs() <= config.min_velocity {
        return 0.0;
    }
    let deceleration = config.deceleration.max(0.0) * elapsed_seconds;
    let decayed = (value.abs() - deceleration).max(0.0);
    value.signum() * decayed
}

fn scroll_progress(offset: f32, max_offset: f32) -> f32 {
    if max_offset <= f32::EPSILON {
        0.0
    } else {
        (offset / max_offset).clamp(0.0, 1.0)
    }
}

fn rect_area(rect: UiRect) -> f32 {
    rect.width.max(0.0) * rect.height.max(0.0)
}

fn clamp_axis(value: f32, min: f32, max: f32) -> f32 {
    let value = finite_value(value);
    let min = finite_value(min);
    let max = finite_value(max);
    if max < min {
        return min;
    }
    value.clamp(min, max)
}

fn finite_point(point: UiPoint) -> UiPoint {
    UiPoint::new(finite_value(point.x), finite_value(point.y))
}

fn finite_value(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

fn point_has_delta(point: UiPoint) -> bool {
    point.x.abs() > f32::EPSILON || point.y.abs() > f32::EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scroll_state(
        axes: ScrollAxes,
        offset: UiPoint,
        viewport_size: UiSize,
        content_size: UiSize,
    ) -> ScrollState {
        ScrollState {
            axes,
            offset,
            viewport_size,
            content_size,
        }
    }

    #[test]
    fn nested_scroll_transfer_chains_boundary_delta_to_parent() {
        let inner = NestedScrollCandidate::new(
            UiNodeId(2),
            scroll_state(
                ScrollAxes::VERTICAL,
                UiPoint::new(0.0, 80.0),
                UiSize::new(100.0, 100.0),
                UiSize::new(100.0, 200.0),
            ),
        );
        let outer = NestedScrollCandidate::new(
            UiNodeId(1),
            scroll_state(
                ScrollAxes::VERTICAL,
                UiPoint::new(0.0, 10.0),
                UiSize::new(100.0, 100.0),
                UiSize::new(100.0, 300.0),
            ),
        );

        let plan = arbitrate_nested_scroll(UiPoint::new(0.0, 70.0), &[inner, outer]);

        assert_eq!(plan.changed_nodes(), vec![UiNodeId(2), UiNodeId(1)]);
        assert_eq!(plan.steps[0].consumed_delta, UiPoint::new(0.0, 20.0));
        assert_eq!(plan.steps[0].overscroll_delta, UiPoint::new(0.0, 50.0));
        assert_eq!(plan.steps[1].consumed_delta, UiPoint::new(0.0, 50.0));
        assert_eq!(
            plan.offset_after(UiNodeId(2)),
            Some(UiPoint::new(0.0, 100.0))
        );
        assert_eq!(
            plan.offset_after(UiNodeId(1)),
            Some(UiPoint::new(0.0, 60.0))
        );
        assert_eq!(plan.remaining_delta, UiPoint::new(0.0, 0.0));
    }

    #[test]
    fn nested_scroll_transfer_respects_contained_overscroll() {
        let inner = NestedScrollCandidate::new(
            UiNodeId(2),
            scroll_state(
                ScrollAxes::VERTICAL,
                UiPoint::new(0.0, 100.0),
                UiSize::new(100.0, 100.0),
                UiSize::new(100.0, 200.0),
            ),
        )
        .overscroll(OverscrollPolicy::CONTAIN);
        let outer = NestedScrollCandidate::new(
            UiNodeId(1),
            scroll_state(
                ScrollAxes::VERTICAL,
                UiPoint::new(0.0, 0.0),
                UiSize::new(100.0, 100.0),
                UiSize::new(100.0, 300.0),
            ),
        );

        let plan = arbitrate_nested_scroll(UiPoint::new(0.0, 40.0), &[inner, outer]);

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.blocked_delta, UiPoint::new(0.0, 40.0));
        assert_eq!(plan.remaining_delta, UiPoint::new(0.0, 0.0));
        assert_eq!(plan.offset_after(UiNodeId(1)), None);
    }

    #[test]
    fn scroll_anchoring_preserves_visible_anchor_position() {
        let scroll = scroll_state(
            ScrollAxes::VERTICAL,
            UiPoint::new(0.0, 100.0),
            UiSize::new(200.0, 80.0),
            UiSize::new(200.0, 600.0),
        );
        let anchor = ScrollAnchorCandidate::new(
            UiNodeId(4),
            UiRect::new(0.0, 130.0, 100.0, 20.0),
            UiRect::new(0.0, 180.0, 100.0, 20.0),
        )
        .priority(10);

        let adjustment = apply_scroll_anchor(scroll, &[anchor]);

        assert_eq!(adjustment.anchor, Some(UiNodeId(4)));
        assert_eq!(adjustment.offset_before, UiPoint::new(0.0, 100.0));
        assert_eq!(adjustment.offset_after, UiPoint::new(0.0, 150.0));
        assert_eq!(adjustment.delta, UiPoint::new(0.0, 50.0));
    }

    #[test]
    fn sticky_and_fixed_content_resolve_against_scrollport_and_viewport() {
        let normal = UiRect::new(12.0, 80.0, 120.0, 30.0);
        let sticky = resolve_scroll_attachment(
            normal,
            ScrollAttachment::Sticky(StickyConstraints::new(
                UiRect::new(0.0, 100.0, 200.0, 80.0),
                UiRect::new(0.0, 0.0, 200.0, 240.0),
                StickyEdges::top(8.0),
            )),
        );

        assert_eq!(sticky.mode, ScrollAttachmentMode::Stuck);
        assert_eq!(sticky.rect, UiRect::new(12.0, 108.0, 120.0, 30.0));
        assert_eq!(sticky.delta, UiPoint::new(0.0, 28.0));

        let fixed = resolve_scroll_attachment(
            normal,
            ScrollAttachment::Fixed(FixedConstraints::new(
                UiRect::new(20.0, 40.0, 300.0, 200.0),
                UiRect::new(8.0, 12.0, 120.0, 30.0),
            )),
        );

        assert_eq!(fixed.mode, ScrollAttachmentMode::Fixed);
        assert_eq!(fixed.rect, UiRect::new(28.0, 52.0, 120.0, 30.0));
    }

    #[test]
    fn reveal_and_sync_helpers_plan_deterministic_offsets() {
        let scroll = scroll_state(
            ScrollAxes::BOTH,
            UiPoint::new(10.0, 20.0),
            UiSize::new(100.0, 80.0),
            UiSize::new(300.0, 240.0),
        );

        let reveal = reveal_rect_into_view(
            scroll,
            UiRect::new(150.0, 130.0, 20.0, 30.0),
            RevealOptions::nearest().margin(ScrollInsets::uniform(4.0)),
        );

        assert_eq!(reveal.offset_after, UiPoint::new(74.0, 84.0));
        assert!(reveal.changed());

        let follower = ScrollSyncSurface::new(
            UiNodeId(9),
            scroll_state(
                ScrollAxes::VERTICAL,
                UiPoint::new(0.0, 0.0),
                UiSize::new(100.0, 50.0),
                UiSize::new(100.0, 250.0),
            ),
        );
        let plan = synchronize_scroll_surfaces(
            ScrollSyncSurface::new(UiNodeId(8), scroll),
            &[follower],
            ScrollAxes::VERTICAL,
            ScrollSyncMode::Proportional,
        );

        assert_eq!(plan.updates.len(), 1);
        assert_eq!(plan.updates[0].offset_after, UiPoint::new(0.0, 25.0));
    }
}
