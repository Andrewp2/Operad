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
