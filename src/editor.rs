//! Renderer-neutral editor surface geometry and interaction helpers.
//!
//! These contracts are for custom surfaces such as piano rolls, timelines,
//! automation lanes, arrangement clips, and game editor canvases. Consumers own
//! the domain model; Operad provides reusable transforms, hit testing, snapping,
//! marquee state, cursor/tool metadata, and overlay ordering.

use crate::input::{DragGesture, GestureEvent, GesturePhase, PointerCapture};
use crate::platform::{LayerOrder, UiLayer};
use crate::{KeyModifiers, UiNodeId, UiPoint, UiRect};

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
pub struct ArrangementGeometry {
    pub timeline: TimelineGeometry,
    pub lanes: LaneGeometry,
}

impl ArrangementGeometry {
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

    pub fn world_clip_rect(self, lane_index: usize, range: EditorAxisRange) -> Option<UiRect> {
        self.lanes.lane_rect(lane_index, range)
    }

    pub fn view_clip_rect(self, lane_index: usize, range: EditorAxisRange) -> Option<UiRect> {
        self.world_clip_rect(lane_index, range)
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

#[derive(Debug, Clone, PartialEq)]
pub struct EditorHitTarget {
    pub id: EditorHitId,
    pub kind: EditorHitKind,
    pub world_rect: UiRect,
    pub z_index: i16,
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
            z_index: 0,
            cursor: None,
            selectable: true,
            draggable: true,
        }
    }

    pub const fn z_index(mut self, z_index: i16) -> Self {
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
                .max_by_key(|(index, target)| (target.z_index, *index))
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
            layer: LayerOrder::new(UiLayer::AppOverlay, 0),
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
    fn arrangement_geometry_builds_clip_selection_and_overlay_rects() {
        let arrangement = ArrangementGeometry::new(
            transform(),
            LaneGeometry::new(5.0, 6)
                .with_origin_y(50.0)
                .with_lane_gap(1.0),
        );

        assert_eq!(arrangement.visible_lanes(), VisibleLaneRange::new(0, 6));
        assert_eq!(arrangement.lane_at_view_y(20.0), Some(0));
        assert_eq!(arrangement.lane_at_view_y(42.0), None);
        assert_eq!(
            arrangement.range_at_view_rect(UiRect::new(10.0, 20.0, 20.0, 10.0)),
            EditorAxisRange::new(100.0, 110.0)
        );

        assert_eq!(
            arrangement.world_clip_rect(2, EditorAxisRange::new(120.0, 132.0)),
            Some(UiRect::new(120.0, 62.0, 12.0, 5.0))
        );
        assert_eq!(
            arrangement.view_clip_rect(2, EditorAxisRange::new(120.0, 132.0)),
            Some(UiRect::new(50.0, 68.0, 24.0, 20.0))
        );
        assert_eq!(
            arrangement.loop_overlay_rect(EditorAxisRange::new(120.0, 132.0)),
            UiRect::new(50.0, 20.0, 24.0, 140.0)
        );
        assert_eq!(
            arrangement.selection_rect(
                VisibleLaneRange::new(1, 4),
                EditorAxisRange::new(110.0, 120.0)
            ),
            UiRect::new(30.0, 44.0, 20.0, 68.0)
        );
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
        let clip = EditorHitTarget::new(
            "clip",
            EditorHitKind::Item,
            UiRect::new(120.0, 60.0, 30.0, 10.0),
        )
        .z_index(2)
        .cursor(EditorCursor::Grab);
        let handle = EditorHitTarget::new(
            "clip.resize",
            EditorHitKind::ResizeHandle,
            UiRect::new(145.0, 60.0, 5.0, 10.0),
        )
        .z_index(4)
        .cursor(EditorCursor::ResizeHorizontal);
        let tester = EditorHitTester::new()
            .target(clip.clone())
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
        assert_eq!(selected, vec![clip.id, handle.id]);
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
                .layer(LayerOrder::new(UiLayer::AppOverlay, 20))
                .hit_testable(true)
                .cursor(EditorCursor::ResizeHorizontal),
        );
        stack.push(
            EditorOverlay::new("debug.bounds", UiRect::new(0.0, 0.0, 100.0, 100.0))
                .layer(LayerOrder::new(UiLayer::DebugOverlay, 0)),
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
        let mut state = EditorSurfaceState::new("piano-roll", node, transform, tool)
            .with_snap_grid(SnapGrid::new(UiPoint::new(0.25, 1.0)));
        let target = EditorHitTarget::new(
            "note.1",
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
            Some("note.1")
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
            Some("note.1")
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
