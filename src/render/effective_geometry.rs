//! Backend-neutral effective geometry contracts.
//!
//! These types describe the geometry that hit testing, accessibility bounds,
//! debug inspection, and backend adapters can share after layout rectangles are
//! combined with paint transforms, clipping, and layer ordering.

use crate::platform::{LayerOrder, UiLayer};
use crate::{PaintItem, PaintKind, PaintTransform, UiNodeId, UiPoint, UiRect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveTransform {
    pub translation: UiPoint,
    pub scale: f32,
}

impl EffectiveTransform {
    pub const IDENTITY: Self = Self {
        translation: UiPoint::new(0.0, 0.0),
        scale: 1.0,
    };

    pub const fn new(translation: UiPoint, scale: f32) -> Self {
        Self { translation, scale }
    }

    pub const fn translation(x: f32, y: f32) -> Self {
        Self::new(UiPoint::new(x, y), 1.0)
    }

    pub const fn scale(scale: f32) -> Self {
        Self::new(UiPoint::new(0.0, 0.0), scale)
    }

    pub fn transform_point(self, point: UiPoint) -> UiPoint {
        UiPoint::new(
            point.x * self.scale + self.translation.x,
            point.y * self.scale + self.translation.y,
        )
    }

    pub fn inverse_transform_point(self, point: UiPoint) -> Option<UiPoint> {
        if !self.is_invertible() {
            return None;
        }
        Some(UiPoint::new(
            (point.x - self.translation.x) / self.scale,
            (point.y - self.translation.y) / self.scale,
        ))
    }

    pub fn transform_rect_bounds(self, rect: UiRect) -> UiRect {
        bounds_from_points(&[
            self.transform_point(UiPoint::new(rect.x, rect.y)),
            self.transform_point(UiPoint::new(rect.right(), rect.y)),
            self.transform_point(UiPoint::new(rect.right(), rect.bottom())),
            self.transform_point(UiPoint::new(rect.x, rect.bottom())),
        ])
    }

    pub fn is_invertible(self) -> bool {
        self.scale.is_finite()
            && self.scale.abs() > f32::EPSILON
            && point_is_finite(self.translation)
    }
}

impl Default for EffectiveTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl From<PaintTransform> for EffectiveTransform {
    fn from(transform: PaintTransform) -> Self {
        Self {
            translation: transform.translation,
            scale: transform.scale,
        }
    }
}

impl From<EffectiveTransform> for PaintTransform {
    fn from(transform: EffectiveTransform) -> Self {
        Self {
            translation: transform.translation,
            scale: transform.scale,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveClip {
    pub source: Option<UiNodeId>,
    pub rect: UiRect,
}

impl EffectiveClip {
    pub const fn new(rect: UiRect) -> Self {
        Self { source: None, rect }
    }

    pub const fn from_node(source: UiNodeId, rect: UiRect) -> Self {
        Self {
            source: Some(source),
            rect,
        }
    }

    pub fn contains_point(self, point: UiPoint) -> bool {
        self.rect.contains_point(point)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveGeometry {
    pub node: UiNodeId,
    pub original_rect: UiRect,
    pub transform: EffectiveTransform,
    pub clip_chain: Vec<EffectiveClip>,
    pub layer_order: LayerOrder,
    pub order: usize,
    pub visible: bool,
    pub hit_testable: bool,
    pub accessibility_rect: Option<UiRect>,
}

impl EffectiveGeometry {
    pub fn new(node: UiNodeId, original_rect: UiRect) -> Self {
        Self {
            node,
            original_rect,
            transform: EffectiveTransform::IDENTITY,
            clip_chain: Vec::new(),
            layer_order: LayerOrder::DEFAULT,
            order: 0,
            visible: true,
            hit_testable: true,
            accessibility_rect: None,
        }
    }

    pub fn from_paint_item(item: &PaintItem, order: usize) -> Self {
        Self::new(item.node, item.rect)
            .paint_transform(item.transform)
            .clip(EffectiveClip::new(item.clip_rect))
            .layer_order(item.layer_order)
            .order(order)
            .hit_testable(paint_item_default_hit_testable(item))
    }

    pub const fn transform(mut self, transform: EffectiveTransform) -> Self {
        self.transform = transform;
        self
    }

    pub fn paint_transform(self, transform: PaintTransform) -> Self {
        self.transform(transform.into())
    }

    pub fn clip(mut self, clip: EffectiveClip) -> Self {
        self.clip_chain.push(clip);
        self
    }

    pub fn clip_rect(self, rect: UiRect) -> Self {
        self.clip(EffectiveClip::new(rect))
    }

    pub fn clip_chain(mut self, clip_chain: Vec<EffectiveClip>) -> Self {
        self.clip_chain = clip_chain;
        self
    }

    pub const fn layer_order(mut self, layer_order: LayerOrder) -> Self {
        self.layer_order = layer_order;
        self
    }

    pub const fn order(mut self, order: usize) -> Self {
        self.order = order;
        self
    }

    pub const fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub const fn hit_testable(mut self, hit_testable: bool) -> Self {
        self.hit_testable = hit_testable;
        self
    }

    pub const fn accessibility_rect(mut self, rect: UiRect) -> Self {
        self.accessibility_rect = Some(rect);
        self
    }

    pub fn transformed_bounds(&self) -> UiRect {
        transformed_bounds(self.original_rect, self.transform)
    }

    pub fn visible_rect(&self) -> Option<UiRect> {
        self.visible
            .then(|| clipped_visible_rect(self.transformed_bounds(), &self.clip_chain))
            .flatten()
    }

    pub fn accessibility_bounds(&self) -> Option<EffectiveAccessibilityBounds> {
        accessibility_bounds(self)
    }

    pub fn hit_eligibility(&self) -> EffectiveHitEligibility {
        let mut rejections = Vec::new();
        if !self.hit_testable {
            rejections.push(EffectiveHitRejection::NotHitTestable);
        }
        if !self.visible {
            rejections.push(EffectiveHitRejection::Invisible);
        }
        if !self.transform.is_invertible() {
            rejections.push(EffectiveHitRejection::NonInvertibleTransform);
        }
        if self.visible_rect().is_none() {
            rejections.push(EffectiveHitRejection::EmptyVisibleRect);
        }
        EffectiveHitEligibility {
            eligible: rejections.is_empty(),
            rejections,
        }
    }

    pub fn contains_point(&self, point: UiPoint) -> bool {
        if !self.hit_eligibility().eligible {
            return false;
        }
        if !self
            .clip_chain
            .iter()
            .copied()
            .all(|clip| clip.contains_point(point))
        {
            return false;
        }
        self.transform
            .inverse_transform_point(point)
            .is_some_and(|local| self.original_rect.contains_point(local))
    }

    pub fn point_hit_rejections(&self, point: UiPoint) -> Vec<EffectiveHitRejection> {
        let mut rejections = self.hit_eligibility().rejections;
        if rejections.is_empty()
            && !self
                .clip_chain
                .iter()
                .copied()
                .all(|clip| clip.contains_point(point))
        {
            rejections.push(EffectiveHitRejection::OutsideClipChain);
        }
        if rejections.is_empty()
            && !self
                .transform
                .inverse_transform_point(point)
                .is_some_and(|local| self.original_rect.contains_point(local))
        {
            rejections.push(EffectiveHitRejection::OutsideOriginalRect);
        }
        rejections
    }

    pub fn diagnostic_record(&self) -> EffectiveGeometryRecord {
        self.diagnostic_record_inner(None)
    }

    pub fn diagnostic_record_for_point(&self, point: UiPoint) -> EffectiveGeometryRecord {
        self.diagnostic_record_inner(Some(point))
    }

    fn diagnostic_record_inner(&self, point: Option<UiPoint>) -> EffectiveGeometryRecord {
        let transformed_rect = self.transformed_bounds();
        let visible_rect = self.visible_rect();
        let hit_eligibility = self.hit_eligibility();
        let point_hit = point.map(|point| self.contains_point(point));
        let point_rejections = point
            .map(|point| self.point_hit_rejections(point))
            .unwrap_or_default();
        EffectiveGeometryRecord {
            node: self.node,
            original_rect: self.original_rect,
            transformed_rect,
            visible_rect,
            clip_chain: self.clip_chain.clone(),
            layer_order: self.layer_order,
            layer: self.layer_order.layer,
            local_z: self.layer_order.local_z,
            resolved_z: self.layer_order.resolved_z(),
            order: self.order,
            hit_testable: self.hit_testable,
            visible: self.visible,
            hit_eligibility,
            test_point: point,
            point_hit,
            point_rejections,
        }
    }
}

fn paint_item_default_hit_testable(item: &PaintItem) -> bool {
    !matches!(
        item.kind,
        PaintKind::Text(_)
            | PaintKind::SceneText(_)
            | PaintKind::Image { .. }
            | PaintKind::ImagePlacement(_)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectiveAccessibilityBoundsSource {
    MetadataRect,
    VisibleRect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveAccessibilityBounds {
    pub node: UiNodeId,
    pub rect: UiRect,
    pub source: EffectiveAccessibilityBoundsSource,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveHit {
    pub node: UiNodeId,
    pub index: usize,
    pub layer_order: LayerOrder,
    pub order: usize,
    pub visible_rect: UiRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectiveHitRejection {
    NotHitTestable,
    Invisible,
    NonInvertibleTransform,
    EmptyVisibleRect,
    OutsideClipChain,
    OutsideOriginalRect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveHitEligibility {
    pub eligible: bool,
    pub rejections: Vec<EffectiveHitRejection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveGeometryRecord {
    pub node: UiNodeId,
    pub original_rect: UiRect,
    pub transformed_rect: UiRect,
    pub visible_rect: Option<UiRect>,
    pub clip_chain: Vec<EffectiveClip>,
    pub layer_order: LayerOrder,
    pub layer: UiLayer,
    pub local_z: i16,
    pub resolved_z: i32,
    pub order: usize,
    pub hit_testable: bool,
    pub visible: bool,
    pub hit_eligibility: EffectiveHitEligibility,
    pub test_point: Option<UiPoint>,
    pub point_hit: Option<bool>,
    pub point_rejections: Vec<EffectiveHitRejection>,
}

pub fn transformed_bounds(rect: UiRect, transform: EffectiveTransform) -> UiRect {
    transform.transform_rect_bounds(rect)
}

pub fn clipped_visible_rect(
    transformed_rect: UiRect,
    clip_chain: &[EffectiveClip],
) -> Option<UiRect> {
    if !rect_has_area(transformed_rect) {
        return None;
    }

    let mut visible = transformed_rect;
    for clip in clip_chain {
        if !rect_has_area(clip.rect) {
            return None;
        }
        visible = visible.intersection(clip.rect)?;
        if !rect_has_area(visible) {
            return None;
        }
    }
    Some(visible)
}

pub fn accessibility_bounds(geometry: &EffectiveGeometry) -> Option<EffectiveAccessibilityBounds> {
    if !geometry.visible {
        return None;
    }

    if let Some(rect) = geometry.accessibility_rect {
        let transformed = geometry.transform.transform_rect_bounds(rect);
        if let Some(rect) = clipped_visible_rect(transformed, &geometry.clip_chain) {
            return Some(EffectiveAccessibilityBounds {
                node: geometry.node,
                rect,
                source: EffectiveAccessibilityBoundsSource::MetadataRect,
            });
        }
    }

    geometry
        .visible_rect()
        .map(|rect| EffectiveAccessibilityBounds {
            node: geometry.node,
            rect,
            source: EffectiveAccessibilityBoundsSource::VisibleRect,
        })
}

pub fn topmost_effective_hit(
    geometries: &[EffectiveGeometry],
    point: UiPoint,
) -> Option<EffectiveHit> {
    geometries
        .iter()
        .enumerate()
        .filter(|(_, geometry)| geometry.contains_point(point))
        .max_by_key(|(index, geometry)| (geometry.layer_order, geometry.order, *index))
        .and_then(|(index, geometry)| {
            geometry.visible_rect().map(|visible_rect| EffectiveHit {
                node: geometry.node,
                index,
                layer_order: geometry.layer_order,
                order: geometry.order,
                visible_rect,
            })
        })
}

pub fn effective_geometry_records(
    geometries: &[EffectiveGeometry],
) -> Vec<EffectiveGeometryRecord> {
    geometries
        .iter()
        .map(EffectiveGeometry::diagnostic_record)
        .collect()
}

pub fn effective_hit_test_records(
    geometries: &[EffectiveGeometry],
    point: UiPoint,
) -> Vec<EffectiveGeometryRecord> {
    let mut indexed = geometries.iter().enumerate().collect::<Vec<_>>();
    indexed.sort_by_key(|(index, geometry)| (geometry.layer_order, geometry.order, *index));
    indexed
        .into_iter()
        .rev()
        .map(|(_, geometry)| geometry.diagnostic_record_for_point(point))
        .collect()
}

fn rect_has_area(rect: UiRect) -> bool {
    rect_is_finite(rect) && rect.width > f32::EPSILON && rect.height > f32::EPSILON
}

fn rect_is_finite(rect: UiRect) -> bool {
    rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite()
}

fn point_is_finite(point: UiPoint) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn bounds_from_points(points: &[UiPoint; 4]) -> UiRect {
    let mut left = points[0].x;
    let mut top = points[0].y;
    let mut right = points[0].x;
    let mut bottom = points[0].y;
    for point in points.iter().copied().skip(1) {
        left = left.min(point.x);
        top = top.min(point.y);
        right = right.max(point.x);
        bottom = bottom.max(point.y);
    }
    UiRect::new(left, top, right - left, bottom - top)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::UiLayer;
    use crate::{
        ColorRgba, PaintItem, PaintKind, PaintTransform, StrokeStyle, TextContent, TextStyle,
    };

    fn assert_rect_near(actual: UiRect, expected: UiRect) {
        let epsilon = 0.001;
        assert!(
            (actual.x - expected.x).abs() <= epsilon
                && (actual.y - expected.y).abs() <= epsilon
                && (actual.width - expected.width).abs() <= epsilon
                && (actual.height - expected.height).abs() <= epsilon,
            "expected {expected:?}, got {actual:?}",
        );
    }

    fn paint_item(node: UiNodeId, kind: PaintKind) -> PaintItem {
        PaintItem {
            node,
            rect: UiRect::new(0.0, 0.0, 80.0, 24.0),
            clip_rect: UiRect::new(0.0, 0.0, 120.0, 40.0),
            z_index: 0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            kind,
        }
    }

    #[test]
    fn transformed_bounds_and_hit_testing_use_effective_transform() {
        let geometry = EffectiveGeometry::new(UiNodeId(1), UiRect::new(10.0, 20.0, 30.0, 10.0))
            .transform(EffectiveTransform::new(UiPoint::new(5.0, -10.0), 2.0));

        assert_rect_near(
            geometry.transformed_bounds(),
            UiRect::new(25.0, 30.0, 60.0, 20.0),
        );
        assert!(geometry.contains_point(UiPoint::new(45.0, 35.0)));
        assert!(!geometry.contains_point(UiPoint::new(20.0, 35.0)));
    }

    #[test]
    fn clipped_visible_rect_intersects_clip_chain() {
        let geometry = EffectiveGeometry::new(UiNodeId(2), UiRect::new(0.0, 0.0, 100.0, 100.0))
            .clip(EffectiveClip::from_node(
                UiNodeId(10),
                UiRect::new(10.0, 10.0, 50.0, 80.0),
            ))
            .clip(EffectiveClip::from_node(
                UiNodeId(11),
                UiRect::new(20.0, 0.0, 60.0, 30.0),
            ));

        assert_eq!(
            geometry.visible_rect(),
            Some(UiRect::new(20.0, 10.0, 40.0, 20.0))
        );

        let clipped_out = geometry
            .clone()
            .clip_rect(UiRect::new(200.0, 200.0, 10.0, 10.0));
        assert_eq!(clipped_out.visible_rect(), None);
    }

    #[test]
    fn topmost_effective_hit_respects_layer_z_and_order() {
        let base = EffectiveGeometry::new(UiNodeId(1), UiRect::new(0.0, 0.0, 50.0, 50.0))
            .layer_order(LayerOrder::new(UiLayer::AppContent, 10))
            .order(10);
        let higher_local_z = EffectiveGeometry::new(UiNodeId(2), UiRect::new(0.0, 0.0, 50.0, 50.0))
            .layer_order(LayerOrder::new(UiLayer::AppContent, 20))
            .order(1);
        let overlay = EffectiveGeometry::new(UiNodeId(3), UiRect::new(0.0, 0.0, 50.0, 50.0))
            .layer_order(LayerOrder::new(UiLayer::AppOverlay, -999))
            .order(0);

        let point = UiPoint::new(25.0, 25.0);
        let hit = topmost_effective_hit(&[base.clone(), higher_local_z.clone()], point)
            .expect("local z hit");
        assert_eq!(hit.node, UiNodeId(2));

        let hit =
            topmost_effective_hit(&[base, higher_local_z, overlay], point).expect("overlay hit");
        assert_eq!(hit.node, UiNodeId(3));
    }

    #[test]
    fn paint_item_effective_geometry_keeps_label_paint_from_stealing_hits() {
        let button_background = EffectiveGeometry::from_paint_item(
            &paint_item(
                UiNodeId(1),
                PaintKind::Rect {
                    fill: ColorRgba::WHITE,
                    stroke: Some(StrokeStyle::new(ColorRgba::BLACK, 1.0)),
                    corner_radius: 4.0,
                },
            ),
            0,
        );
        let button_label = EffectiveGeometry::from_paint_item(
            &paint_item(
                UiNodeId(2),
                PaintKind::Text(TextContent::new("Play", TextStyle::default())),
            ),
            1,
        );

        assert!(!button_label.hit_testable);
        assert_eq!(
            topmost_effective_hit(
                &[button_background.clone(), button_label],
                UiPoint::new(20.0, 12.0)
            )
            .map(|hit| hit.node),
            Some(button_background.node)
        );
    }

    #[test]
    fn accessibility_bounds_prefer_metadata_then_visible_geometry() {
        let geometry = EffectiveGeometry::new(UiNodeId(4), UiRect::new(0.0, 0.0, 100.0, 100.0))
            .transform(EffectiveTransform::new(UiPoint::new(5.0, 10.0), 2.0))
            .clip_rect(UiRect::new(0.0, 0.0, 80.0, 80.0))
            .accessibility_rect(UiRect::new(10.0, 5.0, 20.0, 10.0));

        let bounds = geometry.accessibility_bounds().expect("metadata bounds");
        assert_eq!(bounds.node, UiNodeId(4));
        assert_eq!(
            bounds.source,
            EffectiveAccessibilityBoundsSource::MetadataRect
        );
        assert_rect_near(bounds.rect, UiRect::new(25.0, 20.0, 40.0, 20.0));

        let fallback = EffectiveGeometry {
            accessibility_rect: None,
            ..geometry
        };
        let bounds = fallback.accessibility_bounds().expect("visible bounds");
        assert_eq!(
            bounds.source,
            EffectiveAccessibilityBoundsSource::VisibleRect
        );
        assert_rect_near(bounds.rect, UiRect::new(5.0, 10.0, 75.0, 70.0));
    }

    #[test]
    fn diagnostic_records_explain_geometry_and_hit_eligibility() {
        let geometry = EffectiveGeometry::new(UiNodeId(42), UiRect::new(2.0, 4.0, 10.0, 8.0))
            .transform(EffectiveTransform::new(UiPoint::new(4.0, 2.0), 2.0))
            .clip(EffectiveClip::from_node(
                UiNodeId(7),
                UiRect::new(0.0, 0.0, 24.0, 18.0),
            ))
            .layer_order(LayerOrder::new(UiLayer::DebugOverlay, -10))
            .order(12)
            .hit_testable(false);

        let record = geometry.diagnostic_record_for_point(UiPoint::new(20.0, 20.0));
        assert_eq!(record.node, UiNodeId(42));
        assert_eq!(record.original_rect, UiRect::new(2.0, 4.0, 10.0, 8.0));
        assert_eq!(record.transformed_rect, UiRect::new(8.0, 10.0, 20.0, 16.0));
        assert_eq!(record.visible_rect, Some(UiRect::new(8.0, 10.0, 16.0, 8.0)));
        assert_eq!(record.clip_chain.len(), 1);
        assert_eq!(record.layer, UiLayer::DebugOverlay);
        assert_eq!(record.local_z, -10);
        assert_eq!(record.resolved_z, UiLayer::DebugOverlay.base_z() - 10);
        assert!(!record.hit_eligibility.eligible);
        assert!(record
            .hit_eligibility
            .rejections
            .contains(&EffectiveHitRejection::NotHitTestable));
        assert_eq!(record.point_hit, Some(false));
    }
}
