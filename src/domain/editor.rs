//! Renderer-neutral editor surface geometry and interaction helpers.
//!
//! These contracts are for custom lane timelines, range items, curve editors,
//! value-lane grids, and canvas-like editor surfaces. Consumers own the domain
//! model; Operad provides reusable transforms, hit testing, snapping, marquee
//! state, cursor/tool metadata, and overlay ordering.

use crate::input::{DragGesture, GestureEvent, GesturePhase, PointerCapture};
use crate::platform::{LayerOrder, UiLayer};
use crate::{
    AccessibilityAction, AccessibilityMeta, AccessibilityRole, AccessibilitySummary,
    AccessibilityValueRange, KeyModifiers, UiNodeId, UiPoint, UiRect,
};

const MIN_SCALE: f32 = 0.0001;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorSurfaceId(String);

impl EditorSurfaceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for EditorSurfaceId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for EditorSurfaceId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EditorHitId(String);

impl EditorHitId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for EditorHitId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for EditorHitId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorToolId(String);

impl EditorToolId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for EditorToolId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for EditorToolId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EditorCursor {
    Default,
    Pointer,
    Crosshair,
    Grab,
    Grabbing,
    Text,
    ResizeHorizontal,
    ResizeVertical,
    ResizeBoth,
    Custom(String),
}

impl Default for EditorCursor {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorToolMode {
    pub id: EditorToolId,
    pub label: String,
    pub cursor: EditorCursor,
    pub snapping: bool,
    pub marquee_selection: bool,
}

impl EditorToolMode {
    pub fn new(id: impl Into<EditorToolId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            cursor: EditorCursor::Default,
            snapping: true,
            marquee_selection: false,
        }
    }

    pub fn cursor(mut self, cursor: EditorCursor) -> Self {
        self.cursor = cursor;
        self
    }

    pub const fn snapping(mut self, snapping: bool) -> Self {
        self.snapping = snapping;
        self
    }

    pub const fn marquee_selection(mut self, marquee_selection: bool) -> Self {
        self.marquee_selection = marquee_selection;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorTransform {
    pub viewport: UiRect,
    pub world_origin: UiPoint,
    pub scale: UiPoint,
}

impl EditorTransform {
    pub fn new(viewport: UiRect) -> Self {
        Self {
            viewport,
            world_origin: UiPoint::new(0.0, 0.0),
            scale: UiPoint::new(1.0, 1.0),
        }
    }

    pub fn with_world_origin(mut self, origin: UiPoint) -> Self {
        if point_is_finite(origin) {
            self.world_origin = origin;
        }
        self
    }

    pub fn with_scale(mut self, scale: UiPoint) -> Self {
        self.scale = sanitize_scale(scale);
        self
    }

    pub fn world_to_view_point(self, point: UiPoint) -> UiPoint {
        UiPoint::new(
            self.viewport.x + (point.x - self.world_origin.x) * self.scale.x,
            self.viewport.y + (point.y - self.world_origin.y) * self.scale.y,
        )
    }

    pub fn view_to_world_point(self, point: UiPoint) -> UiPoint {
        UiPoint::new(
            self.world_origin.x + (point.x - self.viewport.x) / self.scale.x,
            self.world_origin.y + (point.y - self.viewport.y) / self.scale.y,
        )
    }

    pub fn world_to_view_rect(self, rect: UiRect) -> UiRect {
        let top_left = self.world_to_view_point(UiPoint::new(rect.x, rect.y));
        UiRect::new(
            top_left.x,
            top_left.y,
            rect.width * self.scale.x,
            rect.height * self.scale.y,
        )
    }

    pub fn view_to_world_rect(self, rect: UiRect) -> UiRect {
        let top_left = self.view_to_world_point(UiPoint::new(rect.x, rect.y));
        UiRect::new(
            top_left.x,
            top_left.y,
            rect.width / self.scale.x,
            rect.height / self.scale.y,
        )
    }

    pub fn visible_world_rect(self) -> UiRect {
        self.view_to_world_rect(self.viewport)
    }

    pub fn pan_by_view_delta(&mut self, delta: UiPoint) {
        if !point_is_finite(delta) {
            return;
        }
        self.world_origin.x -= delta.x / self.scale.x;
        self.world_origin.y -= delta.y / self.scale.y;
    }

    pub fn zoom_around_view_point(&mut self, anchor: UiPoint, factor: f32) {
        if !point_is_finite(anchor) || !factor.is_finite() || factor <= f32::EPSILON {
            return;
        }
        let before = self.view_to_world_point(anchor);
        self.scale = sanitize_scale(UiPoint::new(self.scale.x * factor, self.scale.y * factor));
        let after = self.view_to_world_point(anchor);
        self.world_origin.x += before.x - after.x;
        self.world_origin.y += before.y - after.y;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapGrid {
    pub enabled: bool,
    pub step: UiPoint,
    pub origin: UiPoint,
    pub tolerance: Option<UiPoint>,
}

impl SnapGrid {
    pub const NONE: Self = Self {
        enabled: false,
        step: UiPoint::new(1.0, 1.0),
        origin: UiPoint::new(0.0, 0.0),
        tolerance: None,
    };

    pub fn new(step: UiPoint) -> Self {
        Self {
            enabled: true,
            step: sanitize_step(step),
            origin: UiPoint::new(0.0, 0.0),
            tolerance: None,
        }
    }

    pub fn origin(mut self, origin: UiPoint) -> Self {
        if point_is_finite(origin) {
            self.origin = origin;
        }
        self
    }

    pub fn tolerance(mut self, tolerance: UiPoint) -> Self {
        if point_is_finite(tolerance) {
            self.tolerance = Some(UiPoint::new(tolerance.x.max(0.0), tolerance.y.max(0.0)));
        }
        self
    }

    pub fn snap_point(self, point: UiPoint) -> UiPoint {
        if !self.enabled || !point_is_finite(point) {
            return point;
        }
        UiPoint::new(
            snap_axis(
                point.x,
                self.origin.x,
                self.step.x,
                self.tolerance.map(|value| value.x),
            ),
            snap_axis(
                point.y,
                self.origin.y,
                self.step.y,
                self.tolerance.map(|value| value.y),
            ),
        )
    }

    pub fn snap_rect(self, rect: UiRect) -> UiRect {
        let snapped = self.snap_point(UiPoint::new(rect.x, rect.y));
        UiRect::new(snapped.x, snapped.y, rect.width, rect.height)
    }

    pub fn snap_delta(self, origin: UiPoint, current: UiPoint) -> UiPoint {
        let snapped = self.snap_point(current);
        UiPoint::new(snapped.x - origin.x, snapped.y - origin.y)
    }
}

impl Default for SnapGrid {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorAxisRange {
    pub start: f32,
    pub end: f32,
}

impl EditorAxisRange {
    pub fn new(start: f32, end: f32) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    pub fn length(self) -> f32 {
        (self.end - self.start).max(0.0)
    }

    pub fn is_empty(self) -> bool {
        self.length() <= f32::EPSILON
    }

    pub fn contains(self, value: f32) -> bool {
        value >= self.start && value <= self.end
    }

    pub fn intersects(self, other: Self) -> bool {
        self.start < other.end && self.end > other.start
    }

    pub fn padded(self, amount: f32) -> Self {
        let amount = if amount.is_finite() {
            amount.max(0.0)
        } else {
            0.0
        };
        Self {
            start: self.start - amount,
            end: self.end + amount,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineGeometry {
    pub transform: EditorTransform,
}

impl TimelineGeometry {
    pub const fn new(transform: EditorTransform) -> Self {
        Self { transform }
    }

    pub fn unit_to_view_x(self, unit: f32) -> f32 {
        self.transform
            .world_to_view_point(UiPoint::new(unit, 0.0))
            .x
    }

    pub fn view_x_to_unit(self, x: f32) -> f32 {
        self.transform.view_to_world_point(UiPoint::new(x, 0.0)).x
    }

    pub fn span_to_view_width(self, span: f32) -> f32 {
        span * self.transform.scale.x
    }

    pub fn view_width_to_span(self, width: f32) -> f32 {
        width / self.transform.scale.x
    }

    pub fn visible_units(self) -> EditorAxisRange {
        let visible = self.transform.visible_world_rect();
        EditorAxisRange::new(visible.x, visible.right())
    }

    pub fn snap_unit(self, unit: f32, grid: SnapGrid) -> f32 {
        grid.snap_point(UiPoint::new(unit, 0.0)).x
    }

    pub fn snap_range(self, range: EditorAxisRange, grid: SnapGrid) -> EditorAxisRange {
        EditorAxisRange::new(
            self.snap_unit(range.start, grid),
            self.snap_unit(range.end, grid),
        )
    }

    pub fn playhead_rect(
        self,
        unit: f32,
        world_y: f32,
        world_height: f32,
        width_px: f32,
    ) -> UiRect {
        let x = self.unit_to_view_x(unit) - width_px.max(1.0) * 0.5;
        let top = self
            .transform
            .world_to_view_point(UiPoint::new(unit, world_y))
            .y;
        UiRect::new(
            x,
            top,
            width_px.max(1.0),
            world_height * self.transform.scale.y,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleLaneRange {
    pub start_index: usize,
    pub end_index: usize,
}

impl VisibleLaneRange {
    pub const fn new(start_index: usize, end_index: usize) -> Self {
        Self {
            start_index,
            end_index,
        }
    }

    pub fn len(self) -> usize {
        self.end_index.saturating_sub(self.start_index)
    }

    pub fn is_empty(self) -> bool {
        self.start_index >= self.end_index
    }

    pub fn contains(self, index: usize) -> bool {
        index >= self.start_index && index < self.end_index
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneGeometry {
    pub origin_y: f32,
    pub lane_height: f32,
    pub lane_gap: f32,
    pub lane_count: usize,
}

impl LaneGeometry {
    pub fn new(lane_height: f32, lane_count: usize) -> Self {
        Self {
            origin_y: 0.0,
            lane_height: sanitize_positive(lane_height),
            lane_gap: 0.0,
            lane_count,
        }
    }

    pub fn with_origin_y(mut self, origin_y: f32) -> Self {
        if origin_y.is_finite() {
            self.origin_y = origin_y;
        }
        self
    }

    pub fn with_lane_gap(mut self, lane_gap: f32) -> Self {
        if lane_gap.is_finite() {
            self.lane_gap = lane_gap.max(0.0);
        }
        self
    }

    pub fn lane_pitch(self) -> f32 {
        self.lane_height + self.lane_gap
    }

    pub fn lane_y(self, index: usize) -> f32 {
        self.origin_y + index as f32 * self.lane_pitch()
    }

    pub fn lane_rect(self, index: usize, x_range: EditorAxisRange) -> Option<UiRect> {
        if index >= self.lane_count || x_range.is_empty() {
            return None;
        }
        Some(UiRect::new(
            x_range.start,
            self.lane_y(index),
            x_range.length(),
            self.lane_height,
        ))
    }

    pub fn index_at_y(self, y: f32) -> Option<usize> {
        if !y.is_finite() || self.lane_count == 0 {
            return None;
        }
        let offset = y - self.origin_y;
        if offset < 0.0 {
            return None;
        }
        let pitch = self.lane_pitch();
        let index = (offset / pitch).floor() as usize;
        if index >= self.lane_count {
            return None;
        }
        let lane_offset = offset - index as f32 * pitch;
        if lane_offset <= self.lane_height {
            Some(index)
        } else {
            None
        }
    }

    pub fn visible_lanes(self, world_rect: UiRect) -> VisibleLaneRange {
        if self.lane_count == 0 || world_rect.height <= 0.0 {
            return VisibleLaneRange::new(0, 0);
        }
        if world_rect.bottom() <= self.origin_y {
            return VisibleLaneRange::new(0, 0);
        }
        if world_rect.y >= self.origin_y + self.total_height() {
            return VisibleLaneRange::new(self.lane_count, self.lane_count);
        }

        let pitch = self.lane_pitch();
        let mut first = ((world_rect.y - self.origin_y) / pitch).floor().max(0.0) as usize;
        first = first.min(self.lane_count);
        while first < self.lane_count && self.lane_y(first) + self.lane_height <= world_rect.y {
            first += 1;
        }
        if first >= self.lane_count || self.lane_y(first) >= world_rect.bottom() {
            return VisibleLaneRange::new(first, first);
        }

        let mut last = ((world_rect.bottom() - self.origin_y) / pitch)
            .floor()
            .max(0.0) as usize
            + 1;
        last = last.min(self.lane_count);
        while last > first && self.lane_y(last - 1) >= world_rect.bottom() {
            last -= 1;
        }

        VisibleLaneRange::new(first.min(self.lane_count), last.min(self.lane_count))
    }

    pub fn total_height(self) -> f32 {
        if self.lane_count == 0 {
            0.0
        } else {
            self.lane_count as f32 * self.lane_height
                + self.lane_count.saturating_sub(1) as f32 * self.lane_gap
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneTimelineGeometry {
    pub timeline: TimelineGeometry,
    pub lanes: LaneGeometry,
}

impl LaneTimelineGeometry {
    pub const fn new(transform: EditorTransform, lanes: LaneGeometry) -> Self {
        Self {
            timeline: TimelineGeometry::new(transform),
            lanes,
        }
    }

    pub fn visible_units(self) -> EditorAxisRange {
        self.timeline.visible_units()
    }

    pub fn visible_lanes(self) -> VisibleLaneRange {
        self.lanes
            .visible_lanes(self.timeline.transform.visible_world_rect())
    }

    pub fn world_range_rect(self, lane_index: usize, range: EditorAxisRange) -> Option<UiRect> {
        self.lanes.lane_rect(lane_index, range)
    }

    pub fn view_range_rect(self, lane_index: usize, range: EditorAxisRange) -> Option<UiRect> {
        self.world_range_rect(lane_index, range)
            .map(|rect| self.timeline.transform.world_to_view_rect(rect))
    }

    pub fn lane_at_view_y(self, y: f32) -> Option<usize> {
        let world_y = self
            .timeline
            .transform
            .view_to_world_point(UiPoint::new(0.0, y))
            .y;
        self.lanes.index_at_y(world_y)
    }

    pub fn range_at_view_rect(self, rect: UiRect) -> EditorAxisRange {
        let world = self.timeline.transform.view_to_world_rect(rect);
        EditorAxisRange::new(world.x, world.right())
    }

    pub fn loop_overlay_rect(self, range: EditorAxisRange) -> UiRect {
        let top = self.lanes.origin_y;
        let bottom = self.lanes.origin_y + self.lanes.total_height();
        self.timeline.transform.world_to_view_rect(UiRect::new(
            range.start,
            top,
            range.length(),
            bottom - top,
        ))
    }

    pub fn selection_rect(
        self,
        lane_range: VisibleLaneRange,
        unit_range: EditorAxisRange,
    ) -> UiRect {
        let start_y = self.lanes.lane_y(lane_range.start_index);
        let end_y = if lane_range.is_empty() {
            start_y
        } else {
            self.lanes.lane_y(lane_range.end_index - 1) + self.lanes.lane_height
        };
        self.timeline.transform.world_to_view_rect(UiRect::new(
            unit_range.start,
            start_y,
            unit_range.length(),
            end_y - start_y,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineRangeItemEdge {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineRangeItem {
    pub id: EditorHitId,
    pub lane_index: usize,
    pub range: EditorAxisRange,
    pub selected: bool,
    pub disabled: bool,
    pub dragging: bool,
}

impl TimelineRangeItem {
    pub fn new(
        id: impl Into<EditorHitId>,
        lane_index: usize,
        start_unit: f32,
        duration: f32,
    ) -> Self {
        let start_unit = if start_unit.is_finite() {
            start_unit
        } else {
            0.0
        };
        let duration = if duration.is_finite() {
            duration.max(0.0)
        } else {
            0.0
        };
        Self {
            id: id.into(),
            lane_index,
            range: EditorAxisRange::new(start_unit, start_unit + duration),
            selected: false,
            disabled: false,
            dragging: false,
        }
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub const fn dragging(mut self, dragging: bool) -> Self {
        self.dragging = dragging;
        self
    }

    pub const fn with_range(mut self, range: EditorAxisRange) -> Self {
        self.range = range;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRangeItemGeometry {
    pub lane_timeline: LaneTimelineGeometry,
    pub resize_handle_width_px: f32,
}

impl TimelineRangeItemGeometry {
    pub const fn new(lane_timeline: LaneTimelineGeometry) -> Self {
        Self {
            lane_timeline,
            resize_handle_width_px: 6.0,
        }
    }

    pub fn with_resize_handle_width_px(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.resize_handle_width_px = width;
        }
        self
    }

    pub fn item_world_rect(self, item: &TimelineRangeItem) -> Option<UiRect> {
        if item.range.is_empty() {
            return None;
        }
        self.lane_timeline
            .world_range_rect(item.lane_index, item.range)
    }

    pub fn item_view_rect(self, item: &TimelineRangeItem) -> Option<UiRect> {
        self.item_world_rect(item).map(|rect| {
            self.lane_timeline
                .timeline
                .transform
                .world_to_view_rect(rect)
        })
    }

    pub fn edge_world_rect(
        self,
        item: &TimelineRangeItem,
        edge: TimelineRangeItemEdge,
    ) -> Option<UiRect> {
        let rect = self.item_world_rect(item)?;
        let width = self
            .lane_timeline
            .timeline
            .view_width_to_span(self.resize_handle_width_px)
            .max(MIN_SCALE);
        Some(edge_handle_rect(rect, width, edge))
    }

    pub fn hit_targets(self, item: &TimelineRangeItem) -> Vec<EditorHitTarget> {
        let Some(rect) = self.item_world_rect(item) else {
            return Vec::new();
        };
        let body_z = if item.disabled {
            0.0
        } else if item.dragging {
            30.0
        } else if item.selected {
            20.0
        } else {
            10.0
        };
        let mut body = EditorHitTarget::new(item.id.clone(), EditorHitKind::Item, rect)
            .z_index(body_z)
            .selectable(!item.disabled)
            .draggable(!item.disabled);
        if !item.disabled {
            body = body.cursor(if item.dragging {
                EditorCursor::Grabbing
            } else {
                EditorCursor::Grab
            });
        }
        let mut targets = vec![body];
        if item.disabled {
            return targets;
        }
        for edge in [TimelineRangeItemEdge::Start, TimelineRangeItemEdge::End] {
            if let Some(edge_rect) = self.edge_world_rect(item, edge) {
                targets.push(
                    EditorHitTarget::new(
                        format!("{}.{}", item.id.as_str(), edge.hit_suffix()),
                        EditorHitKind::ResizeHandle,
                        edge_rect,
                    )
                    .z_index(body_z + 1.0)
                    .cursor(EditorCursor::ResizeHorizontal)
                    .selectable(false),
                );
            }
        }
        targets
    }

    pub fn translated_item(
        self,
        item: &TimelineRangeItem,
        unit_delta: f32,
        lane_delta: isize,
        grid: SnapGrid,
    ) -> Option<TimelineRangeItem> {
        if item.disabled {
            return None;
        }
        let lane_index = if lane_delta < 0 {
            item.lane_index.checked_sub(lane_delta.unsigned_abs())?
        } else {
            item.lane_index.checked_add(lane_delta as usize)?
        };
        if lane_index >= self.lane_timeline.lanes.lane_count || !unit_delta.is_finite() {
            return None;
        }
        let duration = item.range.length();
        let start = item.range.start + unit_delta;
        let start = if grid.enabled {
            self.lane_timeline.timeline.snap_unit(start, grid)
        } else {
            start
        };
        Some(TimelineRangeItem {
            lane_index,
            range: EditorAxisRange::new(start, start + duration),
            dragging: true,
            ..item.clone()
        })
    }

    pub fn drag_preview_world_rect(
        self,
        item: &TimelineRangeItem,
        unit_delta: f32,
        lane_delta: isize,
        grid: SnapGrid,
    ) -> Option<UiRect> {
        self.translated_item(item, unit_delta, lane_delta, grid)
            .and_then(|preview| self.item_world_rect(&preview))
    }

    pub fn resized_range(
        self,
        item: &TimelineRangeItem,
        edge: TimelineRangeItemEdge,
        unit: f32,
        min_duration: f32,
        grid: SnapGrid,
    ) -> EditorAxisRange {
        if item.disabled || !unit.is_finite() {
            return item.range;
        }
        let unit = if grid.enabled {
            self.lane_timeline.timeline.snap_unit(unit, grid)
        } else {
            unit
        };
        let min_duration = if min_duration.is_finite() {
            min_duration.max(MIN_SCALE)
        } else {
            MIN_SCALE
        };
        match edge {
            TimelineRangeItemEdge::Start => {
                let start = unit.min(item.range.end - min_duration);
                EditorAxisRange::new(start, item.range.end)
            }
            TimelineRangeItemEdge::End => {
                let end = unit.max(item.range.start + min_duration);
                EditorAxisRange::new(item.range.start, end)
            }
        }
    }
}

impl TimelineRangeItemEdge {
    fn hit_suffix(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveInterpolation {
    Linear,
    Hold,
    Step,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurvePoint {
    pub id: EditorHitId,
    pub unit: f32,
    pub value: f32,
    pub selected: bool,
    pub disabled: bool,
    pub dragging: bool,
}

impl CurvePoint {
    pub fn new(id: impl Into<EditorHitId>, unit: f32, value: f32) -> Self {
        Self {
            id: id.into(),
            unit: finite_or_zero(unit),
            value: finite_or_zero(value),
            selected: false,
            disabled: false,
            dragging: false,
        }
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub const fn dragging(mut self, dragging: bool) -> Self {
        self.dragging = dragging;
        self
    }

    pub fn with_unit_value(mut self, unit: f32, value: f32) -> Self {
        if unit.is_finite() {
            self.unit = unit;
        }
        if value.is_finite() {
            self.value = value;
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveSegment {
    pub from: UiPoint,
    pub to: UiPoint,
}

impl CurveSegment {
    pub const fn new(from: UiPoint, to: UiPoint) -> Self {
        Self { from, to }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveEditorGeometry {
    pub timeline: TimelineGeometry,
    pub value_range: EditorAxisRange,
    pub view_rect: UiRect,
    pub point_radius_px: f32,
}

impl CurveEditorGeometry {
    pub fn new(
        timeline: TimelineGeometry,
        value_range: EditorAxisRange,
        view_rect: UiRect,
    ) -> Self {
        Self {
            timeline,
            value_range: sanitize_axis_range(value_range),
            view_rect,
            point_radius_px: 5.0,
        }
    }

    pub fn with_point_radius_px(mut self, radius: f32) -> Self {
        if radius.is_finite() && radius > 0.0 {
            self.point_radius_px = radius;
        }
        self
    }

    pub fn unit_to_view_x(self, unit: f32) -> f32 {
        self.timeline.unit_to_view_x(unit)
    }

    pub fn view_x_to_unit(self, x: f32) -> f32 {
        self.timeline.view_x_to_unit(x)
    }

    pub fn value_to_view_y(self, value: f32) -> f32 {
        let span = self.value_range.length();
        if span <= MIN_SCALE {
            return self.view_rect.bottom();
        }
        let normalized =
            (clamp_to_axis_range(value, self.value_range) - self.value_range.start) / span;
        self.view_rect.bottom() - normalized * self.view_rect.height
    }

    pub fn view_y_to_value(self, y: f32) -> f32 {
        if !y.is_finite() {
            return self.value_range.start;
        }
        let height = self.view_rect.height.max(MIN_SCALE);
        let normalized = ((self.view_rect.bottom() - y) / height).clamp(0.0, 1.0);
        clamp_to_axis_range(
            self.value_range.start + normalized * self.value_range.length(),
            self.value_range,
        )
    }

    pub fn point_view_position(self, point: &CurvePoint) -> UiPoint {
        UiPoint::new(
            self.unit_to_view_x(point.unit),
            self.value_to_view_y(point.value),
        )
    }

    pub fn point_world_rect(self, point: &CurvePoint) -> UiRect {
        let center = self.point_view_position(point);
        let radius = self.point_radius_px.max(MIN_SCALE);
        self.timeline.transform.view_to_world_rect(UiRect::new(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        ))
    }

    pub fn point_hit_target(self, point: &CurvePoint) -> EditorHitTarget {
        let z_index = if point.disabled {
            0.0
        } else if point.dragging {
            30.0
        } else if point.selected {
            20.0
        } else {
            10.0
        };
        let mut target = EditorHitTarget::new(
            point.id.clone(),
            EditorHitKind::Item,
            self.point_world_rect(point),
        )
        .z_index(z_index)
        .selectable(!point.disabled)
        .draggable(!point.disabled);
        if !point.disabled {
            target = target.cursor(if point.dragging {
                EditorCursor::Grabbing
            } else {
                EditorCursor::Grab
            });
        }
        target
    }

    pub fn hit_targets(self, points: &[CurvePoint]) -> Vec<EditorHitTarget> {
        points
            .iter()
            .map(|point| self.point_hit_target(point))
            .collect()
    }

    pub fn segment_view_points(self, points: &[CurvePoint]) -> Vec<CurveSegment> {
        let sorted = sorted_curve_points(points);
        sorted
            .windows(2)
            .map(|window| {
                CurveSegment::new(
                    self.point_view_position(window[0]),
                    self.point_view_position(window[1]),
                )
            })
            .collect()
    }

    pub fn segment_view_path(
        self,
        points: &[CurvePoint],
        interpolation: CurveInterpolation,
    ) -> Vec<UiPoint> {
        let sorted = sorted_curve_points(points);
        let Some(first) = sorted.first() else {
            return Vec::new();
        };
        let mut path = vec![self.point_view_position(first)];
        for window in sorted.windows(2) {
            let from = self.point_view_position(window[0]);
            let to = self.point_view_position(window[1]);
            match interpolation {
                CurveInterpolation::Linear => path.push(to),
                CurveInterpolation::Hold => {
                    path.push(UiPoint::new(to.x, from.y));
                    path.push(to);
                }
                CurveInterpolation::Step => {
                    let midpoint_x = from.x + (to.x - from.x) * 0.5;
                    path.push(UiPoint::new(midpoint_x, from.y));
                    path.push(UiPoint::new(midpoint_x, to.y));
                    path.push(to);
                }
            }
        }
        path
    }

    pub fn translated_point(
        self,
        point: &CurvePoint,
        unit_delta: f32,
        value_delta: f32,
        unit_grid: SnapGrid,
    ) -> Option<CurvePoint> {
        if point.disabled || !unit_delta.is_finite() || !value_delta.is_finite() {
            return None;
        }
        let unit = point.unit + unit_delta;
        let unit = if unit_grid.enabled {
            self.timeline.snap_unit(unit, unit_grid)
        } else {
            unit
        };
        Some(CurvePoint {
            unit,
            value: clamp_to_axis_range(point.value + value_delta, self.value_range),
            dragging: true,
            ..point.clone()
        })
    }

    pub fn point_at_view_position(
        self,
        id: impl Into<EditorHitId>,
        position: UiPoint,
    ) -> CurvePoint {
        CurvePoint::new(
            id,
            self.view_x_to_unit(position.x),
            self.view_y_to_value(position.y),
        )
    }
}

fn sorted_curve_points(points: &[CurvePoint]) -> Vec<&CurvePoint> {
    let mut sorted = points.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        left.unit
            .total_cmp(&right.unit)
            .then_with(|| left.id.cmp(&right.id))
    });
    sorted
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaneValueRange {
    pub min_value: i16,
    pub max_value: i16,
}

impl LaneValueRange {
    pub const fn new(min_value: i16, max_value: i16) -> Self {
        if min_value <= max_value {
            Self {
                min_value,
                max_value,
            }
        } else {
            Self {
                min_value: max_value,
                max_value: min_value,
            }
        }
    }

    pub fn value_count(self) -> usize {
        (i32::from(self.max_value) - i32::from(self.min_value) + 1).max(0) as usize
    }

    pub const fn contains(self, value: i16) -> bool {
        value >= self.min_value && value <= self.max_value
    }

    pub fn lane_index_for_value(self, value: i16) -> Option<usize> {
        if self.contains(value) {
            Some((i32::from(self.max_value) - i32::from(value)) as usize)
        } else {
            None
        }
    }

    pub fn value_for_lane_index(self, lane_index: usize) -> Option<i16> {
        if lane_index < self.value_count() {
            Some((i32::from(self.max_value) - lane_index as i32) as i16)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaneValueRangeItem {
    pub id: EditorHitId,
    pub value: i16,
    pub range: EditorAxisRange,
    pub magnitude: f32,
    pub selected: bool,
}

impl LaneValueRangeItem {
    pub fn new(id: impl Into<EditorHitId>, value: i16, start_unit: f32, duration: f32) -> Self {
        let start_unit = if start_unit.is_finite() {
            start_unit
        } else {
            0.0
        };
        let duration = if duration.is_finite() {
            duration.max(0.0)
        } else {
            0.0
        };
        Self {
            id: id.into(),
            value,
            range: EditorAxisRange::new(start_unit, start_unit + duration),
            magnitude: 1.0,
            selected: false,
        }
    }

    pub fn magnitude(mut self, magnitude: f32) -> Self {
        if magnitude.is_finite() {
            self.magnitude = magnitude.clamp(0.0, 1.0);
        }
        self
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneValueGeometry {
    pub lane_timeline: LaneTimelineGeometry,
    pub values: LaneValueRange,
    pub resize_handle_width_px: f32,
}

impl LaneValueGeometry {
    pub fn new(transform: EditorTransform, values: LaneValueRange, lane_height: f32) -> Self {
        Self {
            lane_timeline: LaneTimelineGeometry::new(
                transform,
                LaneGeometry::new(lane_height, values.value_count()),
            ),
            values,
            resize_handle_width_px: 6.0,
        }
    }

    pub const fn with_lane_timeline(
        lane_timeline: LaneTimelineGeometry,
        values: LaneValueRange,
    ) -> Self {
        Self {
            lane_timeline,
            values,
            resize_handle_width_px: 6.0,
        }
    }

    pub fn with_resize_handle_width_px(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.resize_handle_width_px = width;
        }
        self
    }

    pub fn value_lane_index(self, value: i16) -> Option<usize> {
        self.values.lane_index_for_value(value)
    }

    pub fn value_at_view_y(self, view_y: f32) -> Option<i16> {
        self.lane_timeline
            .lane_at_view_y(view_y)
            .and_then(|index| self.values.value_for_lane_index(index))
    }

    pub fn value_lane_world_rect(self, value: i16, range: EditorAxisRange) -> Option<UiRect> {
        self.value_lane_index(value)
            .and_then(|lane| self.lane_timeline.world_range_rect(lane, range))
    }

    pub fn value_lane_view_rect(self, value: i16, range: EditorAxisRange) -> Option<UiRect> {
        self.value_lane_world_rect(value, range).map(|rect| {
            self.lane_timeline
                .timeline
                .transform
                .world_to_view_rect(rect)
        })
    }

    pub fn item_world_rect(self, item: &LaneValueRangeItem) -> Option<UiRect> {
        if item.range.is_empty() {
            return None;
        }
        self.value_lane_world_rect(item.value, item.range)
    }

    pub fn item_view_rects(
        self,
        item: &LaneValueRangeItem,
        loop_range: Option<EditorAxisRange>,
    ) -> Vec<UiRect> {
        self.item_world_rects(item, loop_range)
            .into_iter()
            .map(|rect| {
                self.lane_timeline
                    .timeline
                    .transform
                    .world_to_view_rect(rect)
            })
            .collect()
    }

    pub fn item_world_rects(
        self,
        item: &LaneValueRangeItem,
        loop_range: Option<EditorAxisRange>,
    ) -> Vec<UiRect> {
        let Some(lane_index) = self.value_lane_index(item.value) else {
            return Vec::new();
        };
        wrapped_item_ranges(item.range, loop_range)
            .into_iter()
            .filter_map(|range| self.lane_timeline.world_range_rect(lane_index, range))
            .collect()
    }

    pub fn item_hit_targets(
        self,
        item: &LaneValueRangeItem,
        loop_range: Option<EditorAxisRange>,
    ) -> Vec<EditorHitTarget> {
        let rects = self.item_world_rects(item, loop_range);
        if rects.is_empty() {
            return Vec::new();
        }

        let handle_width = self
            .lane_timeline
            .timeline
            .view_width_to_span(self.resize_handle_width_px)
            .max(MIN_SCALE);
        let mut targets = Vec::with_capacity(rects.len() + 2);
        let body_z = if item.selected { 20.0 } else { 10.0 };

        for (index, rect) in rects.iter().copied().enumerate() {
            targets.push(
                EditorHitTarget::new(
                    if index == 0 {
                        item.id.clone()
                    } else {
                        EditorHitId::new(format!("{}.wrap.{index}", item.id.as_str()))
                    },
                    EditorHitKind::Item,
                    rect,
                )
                .z_index(body_z)
                .cursor(EditorCursor::Grab),
            );
        }

        let start_rect = edge_handle_rect(rects[0], handle_width, TimelineRangeItemEdge::Start);
        targets.push(
            EditorHitTarget::new(
                format!("{}.start", item.id.as_str()),
                EditorHitKind::ResizeHandle,
                start_rect,
            )
            .z_index(body_z + 1.0)
            .cursor(EditorCursor::ResizeHorizontal),
        );

        let end_rect = edge_handle_rect(
            rects[rects.len() - 1],
            handle_width,
            TimelineRangeItemEdge::End,
        );
        targets.push(
            EditorHitTarget::new(
                format!("{}.end", item.id.as_str()),
                EditorHitKind::ResizeHandle,
                end_rect,
            )
            .z_index(body_z + 1.0)
            .cursor(EditorCursor::ResizeHorizontal),
        );

        targets
    }

    pub fn magnitude_bar_rect(
        self,
        item: &LaneValueRangeItem,
        magnitude_lane_view_rect: UiRect,
        bar_width_px: f32,
    ) -> Option<UiRect> {
        if !self.values.contains(item.value) || item.range.is_empty() {
            return None;
        }
        let center_x = self.lane_timeline.timeline.unit_to_view_x(item.range.start);
        let width = bar_width_px.max(1.0);
        let height = magnitude_lane_view_rect.height * item.magnitude.clamp(0.0, 1.0);
        Some(UiRect::new(
            center_x - width * 0.5,
            magnitude_lane_view_rect.bottom() - height,
            width,
            height,
        ))
    }
}

fn edge_handle_rect(rect: UiRect, width: f32, edge: TimelineRangeItemEdge) -> UiRect {
    let width = width.min(rect.width.max(0.0) * 0.5).max(MIN_SCALE);
    let x = match edge {
        TimelineRangeItemEdge::Start => rect.x,
        TimelineRangeItemEdge::End => rect.right() - width,
    };
    UiRect::new(x, rect.y, width, rect.height)
}

fn wrapped_item_ranges(
    range: EditorAxisRange,
    loop_range: Option<EditorAxisRange>,
) -> Vec<EditorAxisRange> {
    let Some(loop_range) = loop_range else {
        return vec![range];
    };
    let loop_length = loop_range.length();
    let duration = range.length();
    if loop_length <= MIN_SCALE || duration <= MIN_SCALE {
        return Vec::new();
    }
    if duration >= loop_length {
        return vec![loop_range];
    }

    let start = wrap_unit(range.start, loop_range);
    let end = start + duration;
    if end <= loop_range.end {
        vec![EditorAxisRange::new(start, end)]
    } else {
        vec![
            EditorAxisRange::new(start, loop_range.end),
            EditorAxisRange::new(loop_range.start, loop_range.start + (end - loop_range.end)),
        ]
    }
}

fn wrap_unit(unit: f32, loop_range: EditorAxisRange) -> f32 {
    let length = loop_range.length();
    if length <= MIN_SCALE {
        return unit;
    }
    loop_range.start + (unit - loop_range.start).rem_euclid(length)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RulerTick {
    pub unit: f32,
    pub view_x: f32,
    pub major: bool,
    pub index: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RulerTickConfig {
    pub major_step: f32,
    pub minor_divisions: u16,
}

impl RulerTickConfig {
    pub fn new(major_step: f32) -> Self {
        Self {
            major_step: sanitize_positive(major_step),
            minor_divisions: 1,
        }
    }

    pub fn with_minor_divisions(mut self, minor_divisions: u16) -> Self {
        self.minor_divisions = minor_divisions.max(1);
        self
    }

    pub fn minor_step(self) -> f32 {
        self.major_step / self.minor_divisions as f32
    }
}

pub fn generate_ruler_ticks(
    timeline: TimelineGeometry,
    config: RulerTickConfig,
    visible_units: EditorAxisRange,
) -> Vec<RulerTick> {
    let minor_step = config.minor_step();
    if minor_step <= MIN_SCALE {
        return Vec::new();
    }
    let first = (visible_units.start / minor_step).floor() as i32;
    let last = (visible_units.end / minor_step).ceil() as i32;
    (first..=last)
        .map(|index| {
            let unit = index as f32 * minor_step;
            RulerTick {
                unit,
                view_x: timeline.unit_to_view_x(unit),
                major: index.rem_euclid(config.minor_divisions as i32) == 0,
                index,
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EditorHitKind {
    Surface,
    Item,
    ResizeHandle,
    Ruler,
    GridLine,
    Overlay,
    Custom(String),
}

impl EditorHitKind {
    pub const fn accessibility_role(&self) -> AccessibilityRole {
        match self {
            Self::Surface => AccessibilityRole::EditorSurface,
            Self::Item => AccessibilityRole::ListItem,
            Self::ResizeHandle => AccessibilityRole::Slider,
            Self::Ruler => AccessibilityRole::Ruler,
            Self::GridLine => AccessibilityRole::Separator,
            Self::Overlay | Self::Custom(_) => AccessibilityRole::Group,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditorHitTarget {
    pub id: EditorHitId,
    pub kind: EditorHitKind,
    pub world_rect: UiRect,
    pub z_index: f32,
    pub cursor: Option<EditorCursor>,
    pub selectable: bool,
    pub draggable: bool,
}

impl EditorHitTarget {
    pub fn new(id: impl Into<EditorHitId>, kind: EditorHitKind, world_rect: UiRect) -> Self {
        Self {
            id: id.into(),
            kind,
            world_rect,
            z_index: 0.0,
            cursor: None,
            selectable: true,
            draggable: true,
        }
    }

    pub const fn z_index(mut self, z_index: f32) -> Self {
        self.z_index = z_index;
        self
    }

    pub fn cursor(mut self, cursor: EditorCursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub const fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    pub const fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    pub fn contains_world_point(&self, point: UiPoint) -> bool {
        self.world_rect.contains_point(point)
    }

    pub fn accessible_target(&self, label: impl Into<String>) -> EditorAccessibleTarget {
        EditorAccessibleTarget::from_hit_target(self, label)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditorAccessibleTarget {
    pub id: EditorHitId,
    pub role: AccessibilityRole,
    pub label: String,
    pub value: Option<String>,
    pub hint: Option<String>,
    pub selected: Option<bool>,
    pub disabled: bool,
    pub read_only: bool,
    pub focusable: bool,
    pub value_range: Option<AccessibilityValueRange>,
    pub key_shortcuts: Vec<String>,
    pub actions: Vec<AccessibilityAction>,
}

impl EditorAccessibleTarget {
    pub fn new(
        id: impl Into<EditorHitId>,
        role: AccessibilityRole,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            role,
            label: label.into(),
            value: None,
            hint: None,
            selected: None,
            disabled: false,
            read_only: false,
            focusable: true,
            value_range: None,
            key_shortcuts: Vec::new(),
            actions: Vec::new(),
        }
    }

    pub fn from_hit_target(target: &EditorHitTarget, label: impl Into<String>) -> Self {
        Self {
            focusable: target.selectable || target.draggable,
            ..Self::new(target.id.clone(), target.kind.accessibility_role(), label)
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub const fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    pub const fn value_range(mut self, range: AccessibilityValueRange) -> Self {
        self.value_range = Some(range);
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.key_shortcuts.push(shortcut.into());
        self
    }

    pub fn action(mut self, action: AccessibilityAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(self.role).label(self.label.clone());
        if let Some(value) = &self.value {
            meta = meta.value(value.clone());
        }
        if let Some(hint) = &self.hint {
            meta = meta.hint(hint.clone());
        }
        if self.disabled {
            meta = meta.disabled();
        } else if self.focusable {
            meta = meta.focusable();
        }
        if let Some(selected) = self.selected {
            meta = meta.selected(selected);
        }
        if self.read_only {
            meta = meta.read_only();
        }
        if let Some(range) = self.value_range {
            meta = meta.value_range(range);
        }
        for shortcut in &self.key_shortcuts {
            meta = meta.shortcut(shortcut.clone());
        }
        for action in &self.actions {
            meta = meta.action(action.clone());
        }
        meta
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditorSurfaceAccessibility {
    pub label: String,
    pub description: Option<String>,
    pub visible_units: Option<EditorAxisRange>,
    pub visible_lanes: Option<VisibleLaneRange>,
    pub target_count: Option<usize>,
    pub selected_count: Option<usize>,
    pub active: Option<EditorHitId>,
    pub instructions: Vec<String>,
    pub actions: Vec<AccessibilityAction>,
}

impl EditorSurfaceAccessibility {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            visible_units: None,
            visible_lanes: None,
            target_count: None,
            selected_count: None,
            active: None,
            instructions: Vec::new(),
            actions: Vec::new(),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub const fn visible_units(mut self, range: EditorAxisRange) -> Self {
        self.visible_units = Some(range);
        self
    }

    pub const fn visible_lanes(mut self, range: VisibleLaneRange) -> Self {
        self.visible_lanes = Some(range);
        self
    }

    pub const fn target_count(mut self, count: usize) -> Self {
        self.target_count = Some(count);
        self
    }

    pub const fn selected_count(mut self, count: usize) -> Self {
        self.selected_count = Some(count);
        self
    }

    pub fn active(mut self, id: impl Into<EditorHitId>) -> Self {
        self.active = Some(id.into());
        self
    }

    pub fn instruction(mut self, instruction: impl Into<String>) -> Self {
        self.instructions.push(instruction.into());
        self
    }

    pub fn action(mut self, action: AccessibilityAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn summary(&self) -> AccessibilitySummary {
        let mut summary = AccessibilitySummary::new(self.label.clone());
        if let Some(description) = &self.description {
            summary = summary.description(description.clone());
        }
        if let Some(range) = self.visible_units {
            summary = summary.item("Visible units", format_axis_range(range));
        }
        if let Some(range) = self.visible_lanes {
            summary = summary.item("Visible lanes", format_visible_lanes(range));
        }
        if let Some(count) = self.target_count {
            summary = summary.item("Targets", count.to_string());
        }
        if let Some(count) = self.selected_count {
            summary = summary.item("Selected", count.to_string());
        }
        if let Some(active) = &self.active {
            summary = summary.item("Active", active.as_str());
        }
        for instruction in &self.instructions {
            summary = summary.instruction(instruction.clone());
        }
        summary
    }

    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(AccessibilityRole::EditorSurface)
            .label(self.label.clone())
            .focusable()
            .summary(self.summary());
        if let Some(count) = self.target_count {
            let selected = self.selected_count.unwrap_or(0);
            meta = meta.value(format!("{count} targets, {selected} selected"));
        }
        for action in &self.actions {
            meta = meta.action(action.clone());
        }
        meta
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditorHitTest {
    pub view_point: UiPoint,
    pub world_point: UiPoint,
    pub target: Option<EditorHitTarget>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditorHitTester {
    pub targets: Vec<EditorHitTarget>,
}

impl EditorHitTester {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn target(mut self, target: EditorHitTarget) -> Self {
        self.targets.push(target);
        self
    }

    pub fn hit_test(&self, transform: EditorTransform, view_point: UiPoint) -> EditorHitTest {
        let world_point = transform.view_to_world_point(view_point);
        let target = if transform.viewport.contains_point(view_point) {
            self.targets
                .iter()
                .enumerate()
                .filter(|(_, target)| target.contains_world_point(world_point))
                .max_by(|left, right| {
                    left.1
                        .z_index
                        .total_cmp(&right.1.z_index)
                        .then_with(|| left.0.cmp(&right.0))
                })
                .map(|(_, target)| target.clone())
        } else {
            None
        };
        EditorHitTest {
            view_point,
            world_point,
            target,
        }
    }

    pub fn selectable_in_rect(&self, world_rect: UiRect) -> Vec<EditorHitId> {
        let mut hits = self
            .targets
            .iter()
            .filter(|target| target.selectable && target.world_rect.intersects(world_rect))
            .map(|target| target.id.clone())
            .collect::<Vec<_>>();
        hits.sort();
        hits
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarqueeSelection {
    pub origin: UiPoint,
    pub current: UiPoint,
    pub phase: GesturePhase,
    pub modifiers: KeyModifiers,
}

impl MarqueeSelection {
    pub const fn new(origin: UiPoint, current: UiPoint, modifiers: KeyModifiers) -> Self {
        Self {
            origin,
            current,
            phase: GesturePhase::Begin,
            modifiers,
        }
    }

    pub const fn phase(mut self, phase: GesturePhase) -> Self {
        self.phase = phase;
        self
    }

    pub fn from_drag(transform: EditorTransform, drag: DragGesture) -> Self {
        Self {
            origin: transform.view_to_world_point(drag.origin),
            current: transform.view_to_world_point(drag.current),
            phase: drag.phase,
            modifiers: drag.modifiers,
        }
    }

    pub fn world_rect(self) -> UiRect {
        rect_from_points(self.origin, self.current)
    }

    pub fn view_rect(self, transform: EditorTransform) -> UiRect {
        transform.world_to_view_rect(self.world_rect())
    }

    pub fn is_finished(self) -> bool {
        matches!(self.phase, GesturePhase::Commit | GesturePhase::Cancel)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditorOverlay {
    pub id: EditorHitId,
    pub layer: LayerOrder,
    pub world_rect: UiRect,
    pub hit_testable: bool,
    pub cursor: Option<EditorCursor>,
    pub label: Option<String>,
}

impl EditorOverlay {
    pub fn new(id: impl Into<EditorHitId>, world_rect: UiRect) -> Self {
        Self {
            id: id.into(),
            layer: LayerOrder::new(UiLayer::AppOverlay, 0.0),
            world_rect,
            hit_testable: false,
            cursor: None,
            label: None,
        }
    }

    pub const fn layer(mut self, layer: LayerOrder) -> Self {
        self.layer = layer;
        self
    }

    pub const fn hit_testable(mut self, hit_testable: bool) -> Self {
        self.hit_testable = hit_testable;
        self
    }

    pub fn cursor(mut self, cursor: EditorCursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditorOverlayStack {
    pub overlays: Vec<EditorOverlay>,
}

impl EditorOverlayStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, overlay: EditorOverlay) {
        self.overlays.push(overlay);
    }

    pub fn ordered(&self) -> Vec<&EditorOverlay> {
        let mut overlays = self.overlays.iter().collect::<Vec<_>>();
        overlays.sort_by(|left, right| {
            left.layer
                .cmp(&right.layer)
                .then_with(|| left.id.cmp(&right.id))
        });
        overlays
    }

    pub fn hit_targets(&self) -> Vec<EditorHitTarget> {
        self.overlays
            .iter()
            .filter(|overlay| overlay.hit_testable)
            .map(|overlay| {
                let mut target = EditorHitTarget::new(
                    overlay.id.clone(),
                    EditorHitKind::Overlay,
                    overlay.world_rect,
                )
                .z_index(overlay.layer.local_z)
                .selectable(false)
                .draggable(false);
                if let Some(cursor) = overlay.cursor.clone() {
                    target = target.cursor(cursor);
                }
                target
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditorSurfaceState {
    pub id: EditorSurfaceId,
    pub node: UiNodeId,
    pub transform: EditorTransform,
    pub tool: EditorToolMode,
    pub snap_grid: SnapGrid,
    pub hovered: Option<EditorHitId>,
    pub active: Option<EditorHitId>,
    pub drag_capture: Option<PointerCapture>,
    pub marquee: Option<MarqueeSelection>,
    pub cursor_override: Option<EditorCursor>,
}

impl EditorSurfaceState {
    pub fn new(
        id: impl Into<EditorSurfaceId>,
        node: UiNodeId,
        transform: EditorTransform,
        tool: EditorToolMode,
    ) -> Self {
        Self {
            id: id.into(),
            node,
            transform,
            tool,
            snap_grid: SnapGrid::NONE,
            hovered: None,
            active: None,
            drag_capture: None,
            marquee: None,
            cursor_override: None,
        }
    }

    pub fn with_snap_grid(mut self, snap_grid: SnapGrid) -> Self {
        self.snap_grid = snap_grid;
        self
    }

    pub fn cursor(&self) -> EditorCursor {
        self.cursor_override
            .clone()
            .unwrap_or_else(|| self.tool.cursor.clone())
    }

    pub fn apply_hit_test(&mut self, hit: &EditorHitTest) {
        self.hovered = hit.target.as_ref().map(|target| target.id.clone());
        self.cursor_override = hit.target.as_ref().and_then(|target| target.cursor.clone());
    }

    pub fn apply_gesture(&mut self, event: &GestureEvent, hit: Option<&EditorHitTarget>) {
        match event {
            GestureEvent::Hover { .. } => {
                if let Some(hit) = hit {
                    self.hovered = Some(hit.id.clone());
                    self.cursor_override = hit.cursor.clone();
                }
            }
            GestureEvent::Press {
                target,
                pointer_id,
                position,
                modifiers,
                ..
            } => {
                self.active = hit.map(|target| target.id.clone());
                self.drag_capture = if hit.is_some()
                    || (self.tool.marquee_selection && *target == Some(self.node))
                {
                    Some(PointerCapture::new(
                        *pointer_id,
                        self.node,
                        *position,
                        0.0,
                        *modifiers,
                    ))
                } else {
                    None
                };
            }
            GestureEvent::Drag(drag) if self.tool.marquee_selection && self.active.is_none() => {
                self.marquee = Some(MarqueeSelection::from_drag(self.transform, *drag));
                if drag.phase == GesturePhase::Commit || drag.phase == GesturePhase::Cancel {
                    self.drag_capture = None;
                }
            }
            GestureEvent::Drag(drag) => {
                if drag.phase == GesturePhase::Commit || drag.phase == GesturePhase::Cancel {
                    self.drag_capture = None;
                    self.active = None;
                }
            }
            GestureEvent::Click(click) => {
                self.hovered = hit.map(|target| target.id.clone());
                self.active = hit.map(|target| target.id.clone());
                self.cursor_override = hit.and_then(|target| target.cursor.clone());
                self.drag_capture = None;
                self.marquee = None;
                if click.target != self.node {
                    self.active = None;
                }
            }
            GestureEvent::WheelTargeted { .. } | GestureEvent::Cancel { .. } => {
                self.drag_capture = None;
            }
        }
    }
}

fn sanitize_scale(scale: UiPoint) -> UiPoint {
    UiPoint::new(sanitize_positive(scale.x), sanitize_positive(scale.y))
}

fn sanitize_axis_range(range: EditorAxisRange) -> EditorAxisRange {
    EditorAxisRange::new(finite_or_zero(range.start), finite_or_zero(range.end))
}

fn sanitize_step(step: UiPoint) -> UiPoint {
    UiPoint::new(sanitize_positive(step.x), sanitize_positive(step.y))
}

fn sanitize_positive(value: f32) -> f32 {
    if value.is_finite() && value > MIN_SCALE {
        value
    } else {
        1.0
    }
}

fn clamp_to_axis_range(value: f32, range: EditorAxisRange) -> f32 {
    if !value.is_finite() {
        return range.start;
    }
    value.clamp(range.start, range.end)
}

fn format_axis_range(range: EditorAxisRange) -> String {
    format!(
        "{} to {}",
        format_f32_compact(range.start),
        format_f32_compact(range.end)
    )
}

fn format_visible_lanes(range: VisibleLaneRange) -> String {
    if range.is_empty() {
        "none".to_string()
    } else {
        format!(
            "{} to {}",
            range.start_index,
            range.end_index.saturating_sub(1)
        )
    }
}

fn format_f32_compact(value: f32) -> String {
    if value.is_finite() {
        let rounded = value.round();
        if (value - rounded).abs() <= 0.0001 {
            format!("{rounded:.0}")
        } else {
            format!("{value:.3}")
        }
    } else {
        "0".to_string()
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

fn snap_axis(value: f32, origin: f32, step: f32, tolerance: Option<f32>) -> f32 {
    if !value.is_finite() || !origin.is_finite() || !step.is_finite() || step <= MIN_SCALE {
        return value;
    }
    let snapped = origin + ((value - origin) / step).round() * step;
    if tolerance.is_some_and(|tolerance| (value - snapped).abs() > tolerance) {
        value
    } else {
        snapped
    }
}

fn point_is_finite(point: UiPoint) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn rect_from_points(a: UiPoint, b: UiPoint) -> UiRect {
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = a.x.max(b.x);
    let bottom = a.y.max(b.y);
    UiRect::new(left, top, right - left, bottom - top)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{PointerButton, PointerId};

    fn transform() -> EditorTransform {
        EditorTransform::new(UiRect::new(10.0, 20.0, 400.0, 200.0))
            .with_world_origin(UiPoint::new(100.0, 50.0))
            .with_scale(UiPoint::new(2.0, 4.0))
    }

    #[test]
    fn editor_transform_round_trips_points_and_preserves_zoom_anchor() {
        let mut transform = transform();
        let world = UiPoint::new(150.0, 75.0);
        let view = transform.world_to_view_point(world);

        assert_eq!(view, UiPoint::new(110.0, 120.0));
        assert_eq!(transform.view_to_world_point(view), world);
        assert_eq!(
            transform.visible_world_rect(),
            UiRect::new(100.0, 50.0, 200.0, 50.0)
        );

        let anchor = UiPoint::new(210.0, 120.0);
        let before = transform.view_to_world_point(anchor);
        transform.zoom_around_view_point(anchor, 2.0);
        assert_eq!(transform.view_to_world_point(anchor), before);

        transform.pan_by_view_delta(UiPoint::new(20.0, -8.0));
        assert_eq!(
            transform.world_origin,
            UiPoint::new(before.x - 50.0 - 5.0, before.y - 12.5 + 1.0)
        );
    }

    #[test]
    fn snap_grid_snaps_points_rects_and_deltas_with_tolerance() {
        let grid = SnapGrid::new(UiPoint::new(0.25, 10.0))
            .origin(UiPoint::new(0.0, 2.0))
            .tolerance(UiPoint::new(0.05, 3.0));

        assert_eq!(
            grid.snap_point(UiPoint::new(1.02, 23.2)),
            UiPoint::new(1.0, 22.0)
        );
        assert_eq!(
            grid.snap_point(UiPoint::new(1.08, 26.2)),
            UiPoint::new(1.08, 26.2)
        );
        assert_eq!(
            grid.snap_rect(UiRect::new(1.02, 23.2, 4.0, 5.0)),
            UiRect::new(1.0, 22.0, 4.0, 5.0)
        );
        assert_eq!(
            grid.snap_delta(UiPoint::new(0.5, 2.0), UiPoint::new(1.02, 23.2)),
            UiPoint::new(0.5, 20.0)
        );
    }

    #[test]
    fn timeline_geometry_converts_visible_ranges_and_snap_units() {
        let timeline = TimelineGeometry::new(transform());

        assert_eq!(timeline.unit_to_view_x(125.0), 60.0);
        assert_eq!(timeline.view_x_to_unit(60.0), 125.0);
        assert_eq!(timeline.span_to_view_width(8.0), 16.0);
        assert_eq!(timeline.view_width_to_span(16.0), 8.0);
        assert_eq!(timeline.visible_units(), EditorAxisRange::new(100.0, 300.0));

        let grid = SnapGrid::new(UiPoint::new(0.25, 1.0));
        assert_eq!(timeline.snap_unit(12.37, grid), 12.25);
        assert_eq!(
            timeline.snap_range(EditorAxisRange::new(4.13, 8.88), grid),
            EditorAxisRange::new(4.25, 9.0)
        );

        let playhead = timeline.playhead_rect(125.0, 50.0, 20.0, 3.0);
        assert_eq!(playhead, UiRect::new(58.5, 20.0, 3.0, 80.0));
    }

    #[test]
    fn lane_geometry_maps_indices_and_visible_lanes() {
        let lanes = LaneGeometry::new(10.0, 8)
            .with_origin_y(50.0)
            .with_lane_gap(2.0);

        assert_eq!(lanes.lane_pitch(), 12.0);
        assert_eq!(lanes.lane_y(3), 86.0);
        assert_eq!(lanes.index_at_y(50.0), Some(0));
        assert_eq!(lanes.index_at_y(61.0), None);
        assert_eq!(lanes.index_at_y(86.0), Some(3));
        assert_eq!(lanes.index_at_y(200.0), None);
        assert_eq!(lanes.total_height(), 94.0);

        assert_eq!(
            lanes.lane_rect(2, EditorAxisRange::new(12.0, 20.0)),
            Some(UiRect::new(12.0, 74.0, 8.0, 10.0))
        );
        assert_eq!(
            lanes.visible_lanes(UiRect::new(0.0, 59.0, 100.0, 29.0)),
            VisibleLaneRange::new(0, 4)
        );
        assert_eq!(
            lanes.visible_lanes(UiRect::new(0.0, 0.0, 100.0, 30.0)),
            VisibleLaneRange::new(0, 0)
        );
        assert_eq!(
            lanes.visible_lanes(UiRect::new(0.0, 60.25, 100.0, 1.5)),
            VisibleLaneRange::new(1, 1)
        );
        assert_eq!(
            lanes.visible_lanes(UiRect::new(0.0, 200.0, 100.0, 10.0)),
            VisibleLaneRange::new(8, 8)
        );
    }

    #[test]
    fn lane_timeline_geometry_builds_range_selection_and_overlay_rects() {
        let lane_timeline = LaneTimelineGeometry::new(
            transform(),
            LaneGeometry::new(5.0, 6)
                .with_origin_y(50.0)
                .with_lane_gap(1.0),
        );

        assert_eq!(lane_timeline.visible_lanes(), VisibleLaneRange::new(0, 6));
        assert_eq!(lane_timeline.lane_at_view_y(20.0), Some(0));
        assert_eq!(lane_timeline.lane_at_view_y(42.0), None);
        assert_eq!(
            lane_timeline.range_at_view_rect(UiRect::new(10.0, 20.0, 20.0, 10.0)),
            EditorAxisRange::new(100.0, 110.0)
        );

        assert_eq!(
            lane_timeline.world_range_rect(2, EditorAxisRange::new(120.0, 132.0)),
            Some(UiRect::new(120.0, 62.0, 12.0, 5.0))
        );
        assert_eq!(
            lane_timeline.view_range_rect(2, EditorAxisRange::new(120.0, 132.0)),
            Some(UiRect::new(50.0, 68.0, 24.0, 20.0))
        );
        assert_eq!(
            lane_timeline.loop_overlay_rect(EditorAxisRange::new(120.0, 132.0)),
            UiRect::new(50.0, 20.0, 24.0, 140.0)
        );
        assert_eq!(
            lane_timeline.selection_rect(
                VisibleLaneRange::new(1, 4),
                EditorAxisRange::new(110.0, 120.0)
            ),
            UiRect::new(30.0, 44.0, 20.0, 68.0)
        );
    }

    #[test]
    fn timeline_range_items_build_rects_and_hit_targets() {
        let geometry = TimelineRangeItemGeometry::new(LaneTimelineGeometry::new(
            transform(),
            LaneGeometry::new(5.0, 6)
                .with_origin_y(50.0)
                .with_lane_gap(1.0),
        ))
        .with_resize_handle_width_px(6.0);
        let item = TimelineRangeItem::new("range.1", 2, 120.0, 12.0).selected(true);

        assert_eq!(
            geometry.item_world_rect(&item),
            Some(UiRect::new(120.0, 62.0, 12.0, 5.0))
        );
        assert_eq!(
            geometry.item_view_rect(&item),
            Some(UiRect::new(50.0, 68.0, 24.0, 20.0))
        );
        assert_eq!(
            geometry.edge_world_rect(&item, TimelineRangeItemEdge::Start),
            Some(UiRect::new(120.0, 62.0, 3.0, 5.0))
        );
        assert_eq!(
            geometry.edge_world_rect(&item, TimelineRangeItemEdge::End),
            Some(UiRect::new(129.0, 62.0, 3.0, 5.0))
        );

        let targets = geometry.hit_targets(&item);
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].id.as_str(), "range.1");
        assert_eq!(targets[0].kind, EditorHitKind::Item);
        assert_eq!(targets[0].z_index, 20.0);
        assert!(targets[0].selectable);
        assert!(targets[0].draggable);
        assert_eq!(targets[1].id.as_str(), "range.1.start");
        assert_eq!(targets[1].kind, EditorHitKind::ResizeHandle);
        assert_eq!(targets[1].z_index, 21.0);
        assert!(!targets[1].selectable);
        assert_eq!(targets[2].id.as_str(), "range.1.end");

        let disabled = TimelineRangeItem::new("disabled", 2, 120.0, 12.0).disabled(true);
        let disabled_targets = geometry.hit_targets(&disabled);
        assert_eq!(disabled_targets.len(), 1);
        assert!(!disabled_targets[0].selectable);
        assert!(!disabled_targets[0].draggable);
        assert!(disabled_targets[0].cursor.is_none());
    }

    #[test]
    fn timeline_range_items_translate_preview_and_resize_neutrally() {
        let geometry = TimelineRangeItemGeometry::new(LaneTimelineGeometry::new(
            transform(),
            LaneGeometry::new(5.0, 6)
                .with_origin_y(50.0)
                .with_lane_gap(1.0),
        ));
        let item = TimelineRangeItem::new("range.2", 2, 120.0, 12.0);
        let grid = SnapGrid::new(UiPoint::new(0.25, 1.0));

        let preview = geometry
            .translated_item(&item, 1.13, 1, grid)
            .expect("translated item");
        assert_eq!(preview.lane_index, 3);
        assert_eq!(preview.range, EditorAxisRange::new(121.25, 133.25));
        assert!(preview.dragging);
        assert_eq!(
            geometry.drag_preview_world_rect(&item, 1.13, 1, grid),
            Some(UiRect::new(121.25, 68.0, 12.0, 5.0))
        );
        assert!(geometry
            .translated_item(&item.clone().disabled(true), 1.0, 0, grid)
            .is_none());
        assert!(geometry.translated_item(&item, 1.0, -3, grid).is_none());
        assert!(geometry.translated_item(&item, 1.0, 99, grid).is_none());

        assert_eq!(
            geometry.resized_range(&item, TimelineRangeItemEdge::Start, 123.12, 2.0, grid),
            EditorAxisRange::new(123.0, 132.0)
        );
        assert_eq!(
            geometry.resized_range(&item, TimelineRangeItemEdge::End, 120.2, 2.0, grid),
            EditorAxisRange::new(120.0, 122.0)
        );
        assert_eq!(
            geometry.resized_range(
                &item.clone().disabled(true),
                TimelineRangeItemEdge::End,
                140.0,
                2.0,
                grid
            ),
            item.range
        );
    }

    #[test]
    fn curve_editor_geometry_maps_points_segments_and_hit_targets() {
        let geometry = CurveEditorGeometry::new(
            TimelineGeometry::new(transform()),
            EditorAxisRange::new(0.0, 1.0),
            UiRect::new(10.0, 120.0, 200.0, 80.0),
        )
        .with_point_radius_px(5.0);
        let point = CurvePoint::new("curve.1", 125.0, 0.75).selected(true);

        assert_eq!(geometry.unit_to_view_x(125.0), 60.0);
        assert_eq!(geometry.value_to_view_y(0.75), 140.0);
        assert_eq!(
            geometry.point_view_position(&point),
            UiPoint::new(60.0, 140.0)
        );
        assert_eq!(
            geometry.point_world_rect(&point),
            UiRect::new(122.5, 78.75, 5.0, 2.5)
        );

        let targets = geometry.hit_targets(std::slice::from_ref(&point));
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id.as_str(), "curve.1");
        assert_eq!(targets[0].kind, EditorHitKind::Item);
        assert_eq!(targets[0].z_index, 20.0);
        assert_eq!(targets[0].cursor, Some(EditorCursor::Grab));
        assert!(targets[0].selectable);
        assert!(targets[0].draggable);

        let disabled = CurvePoint::new("curve.disabled", 125.0, 0.75).disabled(true);
        let disabled_target = geometry.point_hit_target(&disabled);
        assert_eq!(disabled_target.z_index, 0.0);
        assert!(disabled_target.cursor.is_none());
        assert!(!disabled_target.selectable);
        assert!(!disabled_target.draggable);

        let later = CurvePoint::new("curve.2", 130.0, 0.25);
        let points = vec![later, point];
        assert_eq!(
            geometry.segment_view_points(&points),
            vec![CurveSegment::new(
                UiPoint::new(60.0, 140.0),
                UiPoint::new(70.0, 180.0)
            )]
        );
        assert_eq!(
            geometry.segment_view_path(&points, CurveInterpolation::Linear),
            vec![UiPoint::new(60.0, 140.0), UiPoint::new(70.0, 180.0)]
        );
        assert_eq!(
            geometry.segment_view_path(&points, CurveInterpolation::Hold),
            vec![
                UiPoint::new(60.0, 140.0),
                UiPoint::new(70.0, 140.0),
                UiPoint::new(70.0, 180.0)
            ]
        );
        assert_eq!(
            geometry.segment_view_path(&points, CurveInterpolation::Step),
            vec![
                UiPoint::new(60.0, 140.0),
                UiPoint::new(65.0, 140.0),
                UiPoint::new(65.0, 180.0),
                UiPoint::new(70.0, 180.0)
            ]
        );
    }

    #[test]
    fn curve_editor_geometry_translates_points_with_snapping_and_clamping() {
        let geometry = CurveEditorGeometry::new(
            TimelineGeometry::new(transform()),
            EditorAxisRange::new(0.0, 1.0),
            UiRect::new(10.0, 120.0, 200.0, 80.0),
        );
        let point = CurvePoint::new("curve.drag", 125.0, 0.75);
        let grid = SnapGrid::new(UiPoint::new(0.25, 1.0));

        let preview = geometry
            .translated_point(&point, 0.13, 0.5, grid)
            .expect("translated point");
        assert_eq!(preview.unit, 125.25);
        assert_eq!(preview.value, 1.0);
        assert!(preview.dragging);
        assert!(geometry
            .translated_point(&point.clone().disabled(true), 0.13, 0.5, grid)
            .is_none());

        assert_eq!(geometry.view_y_to_value(120.0), 1.0);
        assert_eq!(geometry.view_y_to_value(200.0), 0.0);
        assert_eq!(geometry.view_y_to_value(80.0), 1.0);
        assert_eq!(geometry.view_y_to_value(240.0), 0.0);

        let created = geometry.point_at_view_position("curve.new", UiPoint::new(60.0, 140.0));
        assert_eq!(created.id.as_str(), "curve.new");
        assert_eq!(created.unit, 125.0);
        assert_eq!(created.value, 0.75);
    }

    #[test]
    fn editor_accessibility_helpers_describe_targets_and_surfaces() {
        let handle = EditorHitTarget::new(
            "range.1.start",
            EditorHitKind::ResizeHandle,
            UiRect::new(120.0, 62.0, 3.0, 5.0),
        )
        .selectable(false)
        .cursor(EditorCursor::ResizeHorizontal);
        let target = handle
            .accessible_target("Range start")
            .value("Start 120")
            .hint("Resize with arrow keys or drag")
            .selected(true)
            .value_range(AccessibilityValueRange::new(0.0, 240.0).with_step(0.25))
            .shortcut("Left")
            .action(AccessibilityAction::new("nudge.left", "Nudge left").shortcut("Left"));
        let meta = target.accessibility_meta();

        assert_eq!(handle.kind.accessibility_role(), AccessibilityRole::Slider);
        assert_eq!(meta.role, AccessibilityRole::Slider);
        assert_eq!(meta.label.as_deref(), Some("Range start"));
        assert_eq!(meta.value.as_deref(), Some("Start 120"));
        assert_eq!(meta.hint.as_deref(), Some("Resize with arrow keys or drag"));
        assert_eq!(meta.selected, Some(true));
        assert!(meta.focusable);
        assert_eq!(
            meta.value_range,
            Some(AccessibilityValueRange::new(0.0, 240.0).with_step(0.25))
        );
        assert_eq!(meta.key_shortcuts, vec!["Left".to_string()]);
        assert_eq!(meta.actions[0].id, "nudge.left");

        let disabled_meta = EditorHitTarget::new(
            "grid.1",
            EditorHitKind::GridLine,
            UiRect::new(128.0, 50.0, 0.5, 120.0),
        )
        .selectable(false)
        .draggable(false)
        .accessible_target("Grid line")
        .disabled(true)
        .read_only(true)
        .accessibility_meta();
        assert_eq!(disabled_meta.role, AccessibilityRole::Separator);
        assert!(!disabled_meta.enabled);
        assert!(!disabled_meta.focusable);
        assert!(disabled_meta.read_only);

        let surface = EditorSurfaceAccessibility::new("Timeline editor")
            .description("Custom editor surface")
            .visible_units(EditorAxisRange::new(100.0, 132.5))
            .visible_lanes(VisibleLaneRange::new(2, 6))
            .target_count(8)
            .selected_count(2)
            .active("range.1")
            .instruction("Use arrow keys to move selected items")
            .action(AccessibilityAction::new("delete", "Delete selected"));
        let surface_meta = surface.accessibility_meta();
        let summary_text = surface.summary().screen_reader_text();

        assert_eq!(surface_meta.role, AccessibilityRole::EditorSurface);
        assert_eq!(surface_meta.label.as_deref(), Some("Timeline editor"));
        assert_eq!(surface_meta.value.as_deref(), Some("8 targets, 2 selected"));
        assert!(surface_meta.focusable);
        assert_eq!(surface_meta.actions[0].id, "delete");
        assert!(summary_text.contains("Visible units: 100 to 132.500"));
        assert!(summary_text.contains("Visible lanes: 2 to 5"));
        assert!(summary_text.contains("Active: range.1"));
    }

    #[test]
    fn lane_value_geometry_maps_values_lanes_and_items() {
        let lane_grid = LaneValueGeometry::with_lane_timeline(
            LaneTimelineGeometry::new(
                transform(),
                LaneGeometry::new(5.0, 12)
                    .with_origin_y(50.0)
                    .with_lane_gap(1.0),
            ),
            LaneValueRange::new(60, 71),
        );
        let item = LaneValueRangeItem::new("item.1", 69, 120.0, 4.0);

        assert_eq!(lane_grid.values.value_count(), 12);
        assert_eq!(lane_grid.value_lane_index(71), Some(0));
        assert_eq!(lane_grid.value_lane_index(60), Some(11));
        assert_eq!(lane_grid.value_at_view_y(68.0), Some(69));
        assert_eq!(lane_grid.value_at_view_y(66.0), None);
        assert_eq!(
            lane_grid.item_world_rect(&item),
            Some(UiRect::new(120.0, 62.0, 4.0, 5.0))
        );
        assert_eq!(
            lane_grid.item_view_rects(&item, None),
            vec![UiRect::new(50.0, 68.0, 8.0, 20.0)]
        );
    }

    #[test]
    fn lane_value_geometry_wraps_items_across_loop_boundaries() {
        let lane_grid = LaneValueGeometry::with_lane_timeline(
            LaneTimelineGeometry::new(
                transform(),
                LaneGeometry::new(5.0, 12)
                    .with_origin_y(50.0)
                    .with_lane_gap(1.0),
            ),
            LaneValueRange::new(60, 71),
        );
        let item = LaneValueRangeItem::new("item.wrap", 69, 114.0, 6.0);

        assert_eq!(
            lane_grid.item_world_rects(&item, Some(EditorAxisRange::new(100.0, 116.0))),
            vec![
                UiRect::new(114.0, 62.0, 2.0, 5.0),
                UiRect::new(100.0, 62.0, 4.0, 5.0)
            ]
        );
        assert_eq!(
            lane_grid.item_view_rects(&item, Some(EditorAxisRange::new(100.0, 116.0))),
            vec![
                UiRect::new(38.0, 68.0, 4.0, 20.0),
                UiRect::new(10.0, 68.0, 8.0, 20.0)
            ]
        );
    }

    #[test]
    fn lane_value_geometry_hit_targets_distinguish_body_and_resize_handles() {
        let lane_grid = LaneValueGeometry::with_lane_timeline(
            LaneTimelineGeometry::new(
                transform(),
                LaneGeometry::new(5.0, 12)
                    .with_origin_y(50.0)
                    .with_lane_gap(1.0),
            ),
            LaneValueRange::new(60, 71),
        )
        .with_resize_handle_width_px(6.0);
        let item = LaneValueRangeItem::new("item.hit", 69, 114.0, 6.0).selected(true);
        let targets = lane_grid.item_hit_targets(&item, Some(EditorAxisRange::new(100.0, 116.0)));

        assert_eq!(targets.len(), 4);
        assert_eq!(targets[0].id.as_str(), "item.hit");
        assert_eq!(targets[0].kind, EditorHitKind::Item);
        assert_eq!(targets[2].id.as_str(), "item.hit.start");
        assert_eq!(targets[2].kind, EditorHitKind::ResizeHandle);
        assert_eq!(targets[2].world_rect, UiRect::new(114.0, 62.0, 1.0, 5.0));
        assert_eq!(targets[3].id.as_str(), "item.hit.end");
        assert_eq!(targets[3].world_rect, UiRect::new(102.0, 62.0, 2.0, 5.0));

        let tester = targets
            .into_iter()
            .fold(EditorHitTester::new(), |tester, target| {
                tester.target(target)
            });
        let start_hit = tester.hit_test(
            transform(),
            transform().world_to_view_point(UiPoint::new(114.5, 63.0)),
        );
        assert_eq!(
            start_hit.target.as_ref().map(|target| target.id.as_str()),
            Some("item.hit.start")
        );
        let end_hit = tester.hit_test(
            transform(),
            transform().world_to_view_point(UiPoint::new(103.0, 63.0)),
        );
        assert_eq!(
            end_hit.target.as_ref().map(|target| target.id.as_str()),
            Some("item.hit.end")
        );
        let body_hit = tester.hit_test(
            transform(),
            transform().world_to_view_point(UiPoint::new(101.0, 63.0)),
        );
        assert_eq!(
            body_hit.target.as_ref().map(|target| target.id.as_str()),
            Some("item.hit.wrap.1")
        );
    }

    #[test]
    fn lane_value_geometry_magnitude_bars_use_item_magnitude_and_timeline_position() {
        let lane_grid = LaneValueGeometry::with_lane_timeline(
            LaneTimelineGeometry::new(
                transform(),
                LaneGeometry::new(5.0, 12)
                    .with_origin_y(50.0)
                    .with_lane_gap(1.0),
            ),
            LaneValueRange::new(60, 71),
        );
        let item = LaneValueRangeItem::new("item.magnitude", 69, 120.0, 4.0).magnitude(0.75);

        assert_eq!(
            lane_grid.magnitude_bar_rect(&item, UiRect::new(10.0, 180.0, 200.0, 40.0), 6.0),
            Some(UiRect::new(47.0, 190.0, 6.0, 30.0))
        );
        assert!(lane_grid
            .magnitude_bar_rect(
                &LaneValueRangeItem::new("outside", 10, 120.0, 4.0),
                UiRect::new(10.0, 180.0, 200.0, 40.0),
                6.0,
            )
            .is_none());
    }

    #[test]
    fn ruler_ticks_include_major_and_minor_positions() {
        let timeline = TimelineGeometry::new(transform());
        let ticks = generate_ruler_ticks(
            timeline,
            RulerTickConfig::new(4.0).with_minor_divisions(4),
            EditorAxisRange::new(101.25, 104.25),
        );

        assert_eq!(ticks.len(), 5);
        assert_eq!(ticks[0].unit, 101.0);
        assert_eq!(ticks[0].view_x, 12.0);
        assert!(!ticks[0].major);
        assert_eq!(ticks[3].unit, 104.0);
        assert!(ticks[3].major);
        assert_eq!(ticks[4].unit, 105.0);
    }

    #[test]
    fn hit_tester_returns_topmost_target_and_marquee_selection() {
        let transform = transform();
        let range_item = EditorHitTarget::new(
            "range-item",
            EditorHitKind::Item,
            UiRect::new(120.0, 60.0, 30.0, 10.0),
        )
        .z_index(2.0)
        .cursor(EditorCursor::Grab);
        let handle = EditorHitTarget::new(
            "range-item.resize",
            EditorHitKind::ResizeHandle,
            UiRect::new(145.0, 60.0, 5.0, 10.0),
        )
        .z_index(4.0)
        .cursor(EditorCursor::ResizeHorizontal);
        let tester = EditorHitTester::new()
            .target(range_item.clone())
            .target(handle.clone());

        let hit = tester.hit_test(
            transform,
            transform.world_to_view_point(UiPoint::new(146.0, 64.0)),
        );
        assert_eq!(
            hit.target.as_ref().map(|target| &target.id),
            Some(&handle.id)
        );
        assert_eq!(
            hit.target.unwrap().cursor,
            Some(EditorCursor::ResizeHorizontal)
        );

        let selected = tester.selectable_in_rect(UiRect::new(118.0, 58.0, 40.0, 16.0));
        assert_eq!(selected, vec![range_item.id, handle.id]);
    }

    #[test]
    fn marquee_selection_converts_drag_gestures_to_world_rects() {
        let transform = transform();
        let drag = DragGesture {
            pointer_id: PointerId::MOUSE,
            target: UiNodeId(7),
            phase: GesturePhase::Update,
            origin: transform.world_to_view_point(UiPoint::new(120.0, 60.0)),
            current: transform.world_to_view_point(UiPoint::new(160.0, 90.0)),
            previous: transform.world_to_view_point(UiPoint::new(150.0, 80.0)),
            delta: UiPoint::new(20.0, 40.0),
            total_delta: UiPoint::new(80.0, 120.0),
            button: PointerButton::Primary,
            modifiers: KeyModifiers {
                shift: true,
                ..KeyModifiers::NONE
            },
            captured: true,
            timestamp_millis: 20,
        };

        let marquee = MarqueeSelection::from_drag(transform, drag);
        assert_eq!(marquee.world_rect(), UiRect::new(120.0, 60.0, 40.0, 30.0));
        assert_eq!(
            marquee.view_rect(transform),
            transform.world_to_view_rect(UiRect::new(120.0, 60.0, 40.0, 30.0))
        );
        assert!(!marquee.is_finished());
    }

    #[test]
    fn overlay_stack_orders_layers_and_exports_hit_targets() {
        let mut stack = EditorOverlayStack::new();
        stack.push(
            EditorOverlay::new("playhead", UiRect::new(10.0, 0.0, 1.0, 100.0))
                .layer(LayerOrder::new(UiLayer::AppOverlay, 20.0))
                .hit_testable(true)
                .cursor(EditorCursor::ResizeHorizontal),
        );
        stack.push(
            EditorOverlay::new("debug.bounds", UiRect::new(0.0, 0.0, 100.0, 100.0))
                .layer(LayerOrder::new(UiLayer::DebugOverlay, 0.0)),
        );

        let ordered = stack.ordered();
        assert_eq!(ordered[0].id.as_str(), "playhead");
        assert_eq!(ordered[1].id.as_str(), "debug.bounds");

        let hit_targets = stack.hit_targets();
        assert_eq!(hit_targets.len(), 1);
        assert_eq!(hit_targets[0].id.as_str(), "playhead");
        assert_eq!(hit_targets[0].kind, EditorHitKind::Overlay);
        assert_eq!(hit_targets[0].cursor, Some(EditorCursor::ResizeHorizontal));
    }

    #[test]
    fn editor_surface_state_tracks_hover_cursor_drag_and_marquee() {
        let node = UiNodeId(3);
        let transform = transform();
        let tool = EditorToolMode::new("select", "Select")
            .cursor(EditorCursor::Pointer)
            .marquee_selection(true);
        let mut state = EditorSurfaceState::new("lane_grid-roll", node, transform, tool)
            .with_snap_grid(SnapGrid::new(UiPoint::new(0.25, 1.0)));
        let target = EditorHitTarget::new(
            "item.1",
            EditorHitKind::Item,
            UiRect::new(120.0, 60.0, 10.0, 4.0),
        )
        .cursor(EditorCursor::Grab);
        let hit = EditorHitTest {
            view_point: transform.world_to_view_point(UiPoint::new(121.0, 61.0)),
            world_point: UiPoint::new(121.0, 61.0),
            target: Some(target.clone()),
        };

        state.apply_hit_test(&hit);
        assert_eq!(
            state.hovered.as_ref().map(EditorHitId::as_str),
            Some("item.1")
        );
        assert_eq!(state.cursor(), EditorCursor::Grab);

        state.apply_gesture(
            &GestureEvent::Press {
                target: Some(node),
                pointer_id: PointerId::MOUSE,
                position: hit.view_point,
                button: PointerButton::Primary,
                modifiers: KeyModifiers::NONE,
            },
            Some(&target),
        );
        assert_eq!(
            state.active.as_ref().map(EditorHitId::as_str),
            Some("item.1")
        );
        assert!(state.drag_capture.is_some());

        state.active = None;
        state.apply_gesture(
            &GestureEvent::Press {
                target: Some(node),
                pointer_id: PointerId::MOUSE,
                position: transform.world_to_view_point(UiPoint::new(100.0, 50.0)),
                button: PointerButton::Primary,
                modifiers: KeyModifiers::NONE,
            },
            None,
        );
        assert!(state.drag_capture.is_some());

        state.apply_gesture(
            &GestureEvent::Drag(DragGesture {
                pointer_id: PointerId::MOUSE,
                target: node,
                phase: GesturePhase::Begin,
                origin: transform.world_to_view_point(UiPoint::new(100.0, 50.0)),
                current: transform.world_to_view_point(UiPoint::new(110.0, 55.0)),
                previous: transform.world_to_view_point(UiPoint::new(100.0, 50.0)),
                delta: UiPoint::new(20.0, 20.0),
                total_delta: UiPoint::new(20.0, 20.0),
                button: PointerButton::Primary,
                modifiers: KeyModifiers::NONE,
                captured: true,
                timestamp_millis: 32,
            }),
            None,
        );
        assert_eq!(
            state.marquee.unwrap().world_rect(),
            UiRect::new(100.0, 50.0, 10.0, 5.0)
        );
    }
}
