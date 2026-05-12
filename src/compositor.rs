//! Backend-neutral compositor and layer contracts.
//!
//! The types in this module describe stacking, isolation, transforms, clips,
//! masks, hit testing, bounds, and renderer feature fallback planning. They do
//! not prescribe a GPU backend.

use crate::platform::LayerOrder;
use crate::{ColorRgba, CornerRadii, UiPoint, UiRect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AffineTransform {
    pub m11: f32,
    pub m12: f32,
    pub m21: f32,
    pub m22: f32,
    pub dx: f32,
    pub dy: f32,
}

impl AffineTransform {
    pub const IDENTITY: Self = Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    pub const fn new(m11: f32, m12: f32, m21: f32, m22: f32, dx: f32, dy: f32) -> Self {
        Self {
            m11,
            m12,
            m21,
            m22,
            dx,
            dy,
        }
    }

    pub const fn translation(x: f32, y: f32) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    pub const fn scale(x: f32, y: f32) -> Self {
        Self::new(x, 0.0, 0.0, y, 0.0, 0.0)
    }

    pub fn rotation(radians: f32) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self::new(cos, sin, -sin, cos, 0.0, 0.0)
    }

    pub fn then(self, next: Self) -> Self {
        Self {
            m11: next.m11 * self.m11 + next.m21 * self.m12,
            m12: next.m12 * self.m11 + next.m22 * self.m12,
            m21: next.m11 * self.m21 + next.m21 * self.m22,
            m22: next.m12 * self.m21 + next.m22 * self.m22,
            dx: next.m11 * self.dx + next.m21 * self.dy + next.dx,
            dy: next.m12 * self.dx + next.m22 * self.dy + next.dy,
        }
    }

    pub fn transform_point(self, point: UiPoint) -> UiPoint {
        UiPoint::new(
            self.m11 * point.x + self.m21 * point.y + self.dx,
            self.m12 * point.x + self.m22 * point.y + self.dy,
        )
    }

    pub fn transform_rect_bounds(self, rect: UiRect) -> UiRect {
        let points = [
            self.transform_point(UiPoint::new(rect.x, rect.y)),
            self.transform_point(UiPoint::new(rect.right(), rect.y)),
            self.transform_point(UiPoint::new(rect.right(), rect.bottom())),
            self.transform_point(UiPoint::new(rect.x, rect.bottom())),
        ];
        bounds_from_points(&points)
    }

    pub fn inverse(self) -> Option<Self> {
        let determinant = self.m11 * self.m22 - self.m21 * self.m12;
        if determinant.abs() <= f32::EPSILON || !determinant.is_finite() {
            return None;
        }
        Some(Self {
            m11: self.m22 / determinant,
            m12: -self.m12 / determinant,
            m21: -self.m21 / determinant,
            m22: self.m11 / determinant,
            dx: (self.m21 * self.dy - self.m22 * self.dx) / determinant,
            dy: (self.m12 * self.dx - self.m11 * self.dy) / determinant,
        })
    }

    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }
}

impl Default for AffineTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StackingContextId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CompositorLayerId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OffscreenReason {
    Explicit,
    Opacity,
    Mask,
    Filter,
    BlendMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OffscreenIsolation {
    Auto,
    Required(OffscreenReason),
    Disabled,
}

impl OffscreenIsolation {
    pub const fn is_required(self) -> bool {
        matches!(self, Self::Required(_))
    }
}

impl Default for OffscreenIsolation {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompositorClip {
    Rect(UiRect),
    RoundedRect { rect: UiRect, radii: CornerRadii },
}

impl CompositorClip {
    pub const fn rect(rect: UiRect) -> Self {
        Self::Rect(rect)
    }

    pub const fn rounded_rect(rect: UiRect, radii: CornerRadii) -> Self {
        Self::RoundedRect { rect, radii }
    }

    pub const fn bounds(&self) -> UiRect {
        match self {
            Self::Rect(rect) | Self::RoundedRect { rect, .. } => *rect,
        }
    }

    pub fn contains_local_point(&self, point: UiPoint) -> bool {
        self.bounds().contains_point(point)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaskMode {
    Alpha,
    Luminance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositorMask {
    pub bounds: UiRect,
    pub mode: MaskMode,
}

impl CompositorMask {
    pub const fn new(bounds: UiRect, mode: MaskMode) -> Self {
        Self { bounds, mode }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompositorFilterKind {
    Blur,
    Brightness,
    Contrast,
    Saturate,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositorFilter {
    pub kind: CompositorFilterKind,
    pub amount: f32,
}

impl CompositorFilter {
    pub const fn new(kind: CompositorFilterKind, amount: f32) -> Self {
        Self { kind, amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StackingContext {
    pub id: StackingContextId,
    pub parent: Option<StackingContextId>,
    pub layer_order: LayerOrder,
    pub z_index: i32,
    pub order: usize,
    pub transform: AffineTransform,
    pub opacity: f32,
    pub isolation: OffscreenIsolation,
    pub clip: Option<CompositorClip>,
    pub mask: Option<CompositorMask>,
}

impl StackingContext {
    pub fn new(id: StackingContextId) -> Self {
        Self {
            id,
            parent: None,
            layer_order: LayerOrder::DEFAULT,
            z_index: 0,
            order: 0,
            transform: AffineTransform::IDENTITY,
            opacity: 1.0,
            isolation: OffscreenIsolation::Auto,
            clip: None,
            mask: None,
        }
    }

    pub fn root() -> Self {
        Self::new(StackingContextId(0))
    }

    pub const fn parent(mut self, parent: StackingContextId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub const fn layer_order(mut self, layer_order: LayerOrder) -> Self {
        self.layer_order = layer_order;
        self
    }

    pub const fn z_index(mut self, z_index: i32) -> Self {
        self.z_index = z_index;
        self
    }

    pub const fn order(mut self, order: usize) -> Self {
        self.order = order;
        self
    }

    pub const fn transform(mut self, transform: AffineTransform) -> Self {
        self.transform = transform;
        self
    }

    pub const fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub const fn isolation(mut self, isolation: OffscreenIsolation) -> Self {
        self.isolation = isolation;
        self
    }

    pub fn clip(mut self, clip: CompositorClip) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn mask(mut self, mask: CompositorMask) -> Self {
        self.mask = Some(mask);
        self
    }

    pub fn creates_isolated_group(&self) -> bool {
        self.isolation.is_required()
            || self.opacity < 1.0
            || self.mask.is_some()
            || !self.transform.is_identity()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositorLayer {
    pub id: CompositorLayerId,
    pub context: StackingContextId,
    pub z_index: i32,
    pub order: usize,
    pub bounds: UiRect,
    pub transform: AffineTransform,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub isolation: OffscreenIsolation,
    pub clip: Option<CompositorClip>,
    pub mask: Option<CompositorMask>,
    pub filters: Vec<CompositorFilter>,
    pub hit_testable: bool,
}

impl CompositorLayer {
    pub fn new(id: CompositorLayerId, bounds: UiRect) -> Self {
        Self {
            id,
            context: StackingContextId(0),
            z_index: 0,
            order: 0,
            bounds,
            transform: AffineTransform::IDENTITY,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            isolation: OffscreenIsolation::Auto,
            clip: None,
            mask: None,
            filters: Vec::new(),
            hit_testable: true,
        }
    }

    pub const fn context(mut self, context: StackingContextId) -> Self {
        self.context = context;
        self
    }

    pub const fn z_index(mut self, z_index: i32) -> Self {
        self.z_index = z_index;
        self
    }

    pub const fn order(mut self, order: usize) -> Self {
        self.order = order;
        self
    }

    pub const fn transform(mut self, transform: AffineTransform) -> Self {
        self.transform = transform;
        self
    }

    pub const fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub const fn blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    pub const fn isolation(mut self, isolation: OffscreenIsolation) -> Self {
        self.isolation = isolation;
        self
    }

    pub fn clip(mut self, clip: CompositorClip) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn mask(mut self, mask: CompositorMask) -> Self {
        self.mask = Some(mask);
        self
    }

    pub fn filter(mut self, filter: CompositorFilter) -> Self {
        self.filters.push(filter);
        self
    }

    pub const fn hit_testable(mut self, hit_testable: bool) -> Self {
        self.hit_testable = hit_testable;
        self
    }

    pub fn requires_offscreen(&self) -> bool {
        self.isolation.is_required()
            || self.opacity < 1.0
            || self.mask.is_some()
            || !self.filters.is_empty()
            || self.blend_mode != BlendMode::Normal
    }

    pub fn local_visible_bounds(&self) -> Option<UiRect> {
        let mut bounds = self.bounds;
        if let Some(clip) = &self.clip {
            bounds = bounds.intersection(clip.bounds())?;
        }
        if let Some(mask) = &self.mask {
            bounds = bounds.intersection(mask.bounds)?;
        }
        Some(bounds)
    }

    pub fn transformed_bounds(&self) -> Option<UiRect> {
        self.local_visible_bounds()
            .map(|bounds| self.transform.transform_rect_bounds(bounds))
    }

    pub fn world_transform(&self, contexts: &[StackingContext]) -> AffineTransform {
        let mut transform = self.transform;
        for context in context_chain(self.context, contexts).into_iter().rev() {
            transform = transform.then(context.transform);
        }
        transform
    }

    pub fn world_bounds(&self, contexts: &[StackingContext]) -> Option<UiRect> {
        self.local_visible_bounds()
            .map(|bounds| self.world_transform(contexts).transform_rect_bounds(bounds))
    }

    pub fn hit_test(&self, point: UiPoint) -> bool {
        self.hit_test_with_transform(point, self.transform)
    }

    pub fn world_hit_test(&self, point: UiPoint, contexts: &[StackingContext]) -> bool {
        self.hit_test_with_transform(point, self.world_transform(contexts))
    }

    fn hit_test_with_transform(&self, point: UiPoint, transform: AffineTransform) -> bool {
        if !self.hit_testable || self.opacity <= 0.0 {
            return false;
        }
        let Some(inverse) = transform.inverse() else {
            return false;
        };
        let local = inverse.transform_point(point);
        if !self.bounds.contains_point(local) {
            return false;
        }
        if self
            .clip
            .as_ref()
            .is_some_and(|clip| !clip.contains_local_point(local))
        {
            return false;
        }
        if self
            .mask
            .as_ref()
            .is_some_and(|mask| !mask.bounds.contains_point(local))
        {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositorScene {
    pub contexts: Vec<StackingContext>,
    pub layers: Vec<CompositorLayer>,
}

impl CompositorScene {
    pub fn new() -> Self {
        Self {
            contexts: vec![StackingContext::root()],
            layers: Vec::new(),
        }
    }

    pub fn context(mut self, context: StackingContext) -> Self {
        self.contexts.push(context);
        self
    }

    pub fn layer(mut self, layer: CompositorLayer) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn sorted_layer_ids(&self) -> Vec<CompositorLayerId> {
        sort_layers_for_paint(&self.layers, &self.contexts)
    }

    pub fn topmost_layer_at(&self, point: UiPoint) -> Option<CompositorLayerId> {
        topmost_layer_at(&self.layers, &self.contexts, point)
    }
}

impl Default for CompositorScene {
    fn default() -> Self {
        Self::new()
    }
}

pub fn sort_layers_for_paint(
    layers: &[CompositorLayer],
    contexts: &[StackingContext],
) -> Vec<CompositorLayerId> {
    let mut indexed = layers.iter().enumerate().collect::<Vec<_>>();
    indexed.sort_by(|left, right| {
        layer_sort_key(left.1, contexts)
            .cmp(&layer_sort_key(right.1, contexts))
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.1.id.cmp(&right.1.id))
    });
    indexed.into_iter().map(|(_, layer)| layer.id).collect()
}

pub fn topmost_layer_at(
    layers: &[CompositorLayer],
    contexts: &[StackingContext],
    point: UiPoint,
) -> Option<CompositorLayerId> {
    let sorted = sort_layers_for_paint(layers, contexts);
    sorted.into_iter().rev().find(|id| {
        layers
            .iter()
            .find(|layer| layer.id == *id)
            .is_some_and(|layer| layer.world_hit_test(point, contexts))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FeatureSupportLevel {
    Unsupported,
    Fallback,
    Basic,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SubpixelTextPolicy {
    /// Text is not rasterized with subpixel positioning or component masks.
    Disabled,
    /// Grayscale alpha glyph masks with fractional-position placement.
    Grayscale,
    /// RGB/LCD component subpixel masks.
    Subpixel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ColorManagementLevel {
    SrgbOnly,
    DisplayP3,
    ExtendedLinear,
    IccProfiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderFeature {
    Shadows,
    RoundedClipping,
    Borders,
    Gradients,
    Masks,
    Filters,
    BackdropFilters,
    SubpixelText,
    ColorManagement,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderFeatureSupport {
    pub shadows: FeatureSupportLevel,
    pub rounded_clipping: FeatureSupportLevel,
    pub borders: FeatureSupportLevel,
    pub gradients: FeatureSupportLevel,
    pub masks: FeatureSupportLevel,
    pub filters: FeatureSupportLevel,
    pub backdrop_filters: FeatureSupportLevel,
    pub subpixel_text: SubpixelTextPolicy,
    pub color_management: ColorManagementLevel,
    pub max_shadow_blur: f32,
    pub max_border_width: f32,
    pub max_gradient_stops: usize,
    pub max_backdrop_blur: f32,
}

impl RenderFeatureSupport {
    pub const NONE: Self = Self {
        shadows: FeatureSupportLevel::Unsupported,
        rounded_clipping: FeatureSupportLevel::Unsupported,
        borders: FeatureSupportLevel::Unsupported,
        gradients: FeatureSupportLevel::Unsupported,
        masks: FeatureSupportLevel::Unsupported,
        filters: FeatureSupportLevel::Unsupported,
        backdrop_filters: FeatureSupportLevel::Unsupported,
        subpixel_text: SubpixelTextPolicy::Disabled,
        color_management: ColorManagementLevel::SrgbOnly,
        max_shadow_blur: 0.0,
        max_border_width: 0.0,
        max_gradient_stops: 0,
        max_backdrop_blur: 0.0,
    };

    pub const STANDARD: Self = Self {
        shadows: FeatureSupportLevel::Full,
        rounded_clipping: FeatureSupportLevel::Full,
        borders: FeatureSupportLevel::Full,
        gradients: FeatureSupportLevel::Full,
        masks: FeatureSupportLevel::Full,
        filters: FeatureSupportLevel::Full,
        backdrop_filters: FeatureSupportLevel::Full,
        subpixel_text: SubpixelTextPolicy::Subpixel,
        color_management: ColorManagementLevel::DisplayP3,
        max_shadow_blur: 256.0,
        max_border_width: 256.0,
        max_gradient_stops: 16,
        max_backdrop_blur: 128.0,
    };

    pub const CPU_SNAPSHOT_QUALITY: Self = Self {
        shadows: FeatureSupportLevel::Basic,
        rounded_clipping: FeatureSupportLevel::Basic,
        borders: FeatureSupportLevel::Basic,
        gradients: FeatureSupportLevel::Basic,
        masks: FeatureSupportLevel::Basic,
        filters: FeatureSupportLevel::Basic,
        backdrop_filters: FeatureSupportLevel::Unsupported,
        subpixel_text: SubpixelTextPolicy::Disabled,
        color_management: ColorManagementLevel::SrgbOnly,
        max_shadow_blur: 64.0,
        max_border_width: 64.0,
        max_gradient_stops: 8,
        max_backdrop_blur: 0.0,
    };

    pub const WGPU_SNAPSHOT_QUALITY: Self = Self {
        shadows: FeatureSupportLevel::Basic,
        rounded_clipping: FeatureSupportLevel::Basic,
        borders: FeatureSupportLevel::Basic,
        gradients: FeatureSupportLevel::Basic,
        masks: FeatureSupportLevel::Basic,
        filters: FeatureSupportLevel::Basic,
        backdrop_filters: FeatureSupportLevel::Unsupported,
        subpixel_text: SubpixelTextPolicy::Grayscale,
        color_management: ColorManagementLevel::SrgbOnly,
        max_shadow_blur: 64.0,
        max_border_width: 64.0,
        max_gradient_stops: 2,
        max_backdrop_blur: 0.0,
    };
}

impl Default for RenderFeatureSupport {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderFeatureRequirement {
    Shadow { blur_radius: f32 },
    RoundedClip { radii: CornerRadii },
    Border { width: f32 },
    Gradient { stops: usize, fallback: ColorRgba },
    Mask,
    Filter { kind: CompositorFilterKind },
    BackdropFilter { kind: CompositorFilterKind },
    SubpixelText { policy: SubpixelTextPolicy },
    ColorManagement { level: ColorManagementLevel },
}

impl RenderFeatureRequirement {
    pub const fn feature(&self) -> RenderFeature {
        match self {
            Self::Shadow { .. } => RenderFeature::Shadows,
            Self::RoundedClip { .. } => RenderFeature::RoundedClipping,
            Self::Border { .. } => RenderFeature::Borders,
            Self::Gradient { .. } => RenderFeature::Gradients,
            Self::Mask => RenderFeature::Masks,
            Self::Filter { .. } => RenderFeature::Filters,
            Self::BackdropFilter { .. } => RenderFeature::BackdropFilters,
            Self::SubpixelText { .. } => RenderFeature::SubpixelText,
            Self::ColorManagement { .. } => RenderFeature::ColorManagement,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureFallbackAction {
    Native,
    Approximate,
    RasterizeOffscreen,
    FlattenToSolidColor,
    ConvertToSrgb,
    Disable,
    UseGrayscaleText,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFeaturePlanItem {
    pub requirement: RenderFeatureRequirement,
    pub feature: RenderFeature,
    pub support: FeatureSupportLevel,
    pub action: FeatureFallbackAction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFeaturePlan {
    pub items: Vec<RenderFeaturePlanItem>,
}

impl RenderFeaturePlan {
    pub fn fallback_items(&self) -> Vec<&RenderFeaturePlanItem> {
        self.items
            .iter()
            .filter(|item| item.action != FeatureFallbackAction::Native)
            .collect()
    }

    pub fn action_for(&self, feature: RenderFeature) -> Option<FeatureFallbackAction> {
        self.items
            .iter()
            .find(|item| item.feature == feature)
            .map(|item| item.action)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompositorParityExpectation {
    PixelExact,
    ChannelTolerance { max_channel_delta: u8 },
    FallbackActionParity,
    CpuAuthoritativeFallback,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositorQualityRecord {
    pub requirement: RenderFeatureRequirement,
    pub feature: RenderFeature,
    pub cpu_support: FeatureSupportLevel,
    pub wgpu_support: FeatureSupportLevel,
    pub cpu_action: FeatureFallbackAction,
    pub wgpu_action: FeatureFallbackAction,
    pub parity_expectation: CompositorParityExpectation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositorQualityPlan {
    pub records: Vec<CompositorQualityRecord>,
}

impl CompositorQualityPlan {
    pub fn record_for(&self, feature: RenderFeature) -> Option<&CompositorQualityRecord> {
        self.records.iter().find(|record| record.feature == feature)
    }

    pub fn wgpu_fallback_records(&self) -> Vec<&CompositorQualityRecord> {
        self.records
            .iter()
            .filter(|record| record.wgpu_action != FeatureFallbackAction::Native)
            .collect()
    }
}

pub fn compositor_quality_requirements() -> Vec<RenderFeatureRequirement> {
    vec![
        RenderFeatureRequirement::Shadow { blur_radius: 16.0 },
        RenderFeatureRequirement::RoundedClip {
            radii: CornerRadii::uniform(8.0),
        },
        RenderFeatureRequirement::Border { width: 2.0 },
        RenderFeatureRequirement::Gradient {
            stops: 4,
            fallback: ColorRgba::BLACK,
        },
        RenderFeatureRequirement::Mask,
        RenderFeatureRequirement::Filter {
            kind: CompositorFilterKind::Blur,
        },
        RenderFeatureRequirement::BackdropFilter {
            kind: CompositorFilterKind::Blur,
        },
        RenderFeatureRequirement::ColorManagement {
            level: ColorManagementLevel::SrgbOnly,
        },
        RenderFeatureRequirement::SubpixelText {
            policy: SubpixelTextPolicy::Grayscale,
        },
    ]
}

pub fn plan_compositor_quality(
    requirements: &[RenderFeatureRequirement],
    cpu_support: RenderFeatureSupport,
    wgpu_support: RenderFeatureSupport,
) -> CompositorQualityPlan {
    let cpu_plan = plan_render_feature_fallbacks(requirements, cpu_support);
    let wgpu_plan = plan_render_feature_fallbacks(requirements, wgpu_support);
    let records = cpu_plan
        .items
        .into_iter()
        .zip(wgpu_plan.items)
        .map(|(cpu, wgpu)| CompositorQualityRecord {
            requirement: cpu.requirement,
            feature: cpu.feature,
            cpu_support: cpu.support,
            wgpu_support: wgpu.support,
            cpu_action: cpu.action,
            wgpu_action: wgpu.action,
            parity_expectation: parity_expectation_for(cpu.feature, cpu.action, wgpu.action),
        })
        .collect();
    CompositorQualityPlan { records }
}

pub fn plan_render_feature_fallbacks(
    requirements: &[RenderFeatureRequirement],
    support: RenderFeatureSupport,
) -> RenderFeaturePlan {
    let items = requirements
        .iter()
        .cloned()
        .map(|requirement| {
            let feature = requirement.feature();
            let (support_level, action) = match &requirement {
                RenderFeatureRequirement::Shadow { blur_radius } => {
                    let action = match support.shadows {
                        FeatureSupportLevel::Full => {
                            if *blur_radius <= support.max_shadow_blur {
                                FeatureFallbackAction::Native
                            } else {
                                FeatureFallbackAction::RasterizeOffscreen
                            }
                        }
                        FeatureSupportLevel::Basic => {
                            if *blur_radius <= support.max_shadow_blur {
                                FeatureFallbackAction::Native
                            } else {
                                FeatureFallbackAction::Approximate
                            }
                        }
                        FeatureSupportLevel::Fallback => FeatureFallbackAction::RasterizeOffscreen,
                        FeatureSupportLevel::Unsupported => FeatureFallbackAction::Disable,
                    };
                    (support.shadows, action)
                }
                RenderFeatureRequirement::RoundedClip { .. } => (
                    support.rounded_clipping,
                    action_for_level(
                        support.rounded_clipping,
                        FeatureFallbackAction::Disable,
                        FeatureFallbackAction::RasterizeOffscreen,
                    ),
                ),
                RenderFeatureRequirement::Border { width } => {
                    let action = match support.borders {
                        FeatureSupportLevel::Full => {
                            if *width <= support.max_border_width {
                                FeatureFallbackAction::Native
                            } else {
                                FeatureFallbackAction::RasterizeOffscreen
                            }
                        }
                        FeatureSupportLevel::Basic => {
                            if *width <= support.max_border_width {
                                FeatureFallbackAction::Native
                            } else {
                                FeatureFallbackAction::Approximate
                            }
                        }
                        FeatureSupportLevel::Fallback => FeatureFallbackAction::RasterizeOffscreen,
                        FeatureSupportLevel::Unsupported => FeatureFallbackAction::Disable,
                    };
                    (support.borders, action)
                }
                RenderFeatureRequirement::Gradient { stops, .. } => {
                    let action = match support.gradients {
                        FeatureSupportLevel::Full => FeatureFallbackAction::Native,
                        FeatureSupportLevel::Basic => {
                            if *stops <= support.max_gradient_stops {
                                FeatureFallbackAction::Native
                            } else {
                                FeatureFallbackAction::Approximate
                            }
                        }
                        FeatureSupportLevel::Fallback | FeatureSupportLevel::Unsupported => {
                            FeatureFallbackAction::FlattenToSolidColor
                        }
                    };
                    (support.gradients, action)
                }
                RenderFeatureRequirement::Mask => (
                    support.masks,
                    action_for_level(
                        support.masks,
                        FeatureFallbackAction::Disable,
                        FeatureFallbackAction::RasterizeOffscreen,
                    ),
                ),
                RenderFeatureRequirement::Filter { .. } => (
                    support.filters,
                    action_for_level(
                        support.filters,
                        FeatureFallbackAction::Disable,
                        FeatureFallbackAction::RasterizeOffscreen,
                    ),
                ),
                RenderFeatureRequirement::BackdropFilter { kind } => {
                    let action = match support.backdrop_filters {
                        FeatureSupportLevel::Full => FeatureFallbackAction::Native,
                        FeatureSupportLevel::Basic => {
                            if *kind == CompositorFilterKind::Blur
                                && support.max_backdrop_blur > 0.0
                            {
                                FeatureFallbackAction::Native
                            } else {
                                FeatureFallbackAction::Approximate
                            }
                        }
                        FeatureSupportLevel::Fallback => FeatureFallbackAction::RasterizeOffscreen,
                        FeatureSupportLevel::Unsupported => FeatureFallbackAction::Disable,
                    };
                    (support.backdrop_filters, action)
                }
                RenderFeatureRequirement::SubpixelText { policy } => {
                    let action = if support.subpixel_text >= *policy {
                        FeatureFallbackAction::Native
                    } else if support.subpixel_text >= SubpixelTextPolicy::Grayscale {
                        FeatureFallbackAction::UseGrayscaleText
                    } else {
                        FeatureFallbackAction::Disable
                    };
                    (subpixel_support_level(support.subpixel_text), action)
                }
                RenderFeatureRequirement::ColorManagement { level } => {
                    let action = if support.color_management >= *level {
                        FeatureFallbackAction::Native
                    } else {
                        FeatureFallbackAction::ConvertToSrgb
                    };
                    (color_support_level(support.color_management), action)
                }
            };
            RenderFeaturePlanItem {
                requirement,
                feature,
                support: support_level,
                action,
            }
        })
        .collect();
    RenderFeaturePlan { items }
}

fn parity_expectation_for(
    feature: RenderFeature,
    cpu_action: FeatureFallbackAction,
    wgpu_action: FeatureFallbackAction,
) -> CompositorParityExpectation {
    if wgpu_action == FeatureFallbackAction::Disable {
        return CompositorParityExpectation::Unsupported;
    }
    if cpu_action == wgpu_action && wgpu_action != FeatureFallbackAction::Native {
        return CompositorParityExpectation::FallbackActionParity;
    }
    if cpu_action != FeatureFallbackAction::Native || wgpu_action != FeatureFallbackAction::Native {
        return CompositorParityExpectation::CpuAuthoritativeFallback;
    }
    match feature {
        RenderFeature::Shadows => CompositorParityExpectation::ChannelTolerance {
            max_channel_delta: 8,
        },
        RenderFeature::Filters => CompositorParityExpectation::ChannelTolerance {
            max_channel_delta: 8,
        },
        RenderFeature::SubpixelText => CompositorParityExpectation::ChannelTolerance {
            max_channel_delta: 2,
        },
        _ => CompositorParityExpectation::PixelExact,
    }
}

fn action_for_level(
    level: FeatureSupportLevel,
    unsupported: FeatureFallbackAction,
    fallback: FeatureFallbackAction,
) -> FeatureFallbackAction {
    match level {
        FeatureSupportLevel::Full | FeatureSupportLevel::Basic => FeatureFallbackAction::Native,
        FeatureSupportLevel::Fallback => fallback,
        FeatureSupportLevel::Unsupported => unsupported,
    }
}

fn subpixel_support_level(policy: SubpixelTextPolicy) -> FeatureSupportLevel {
    match policy {
        SubpixelTextPolicy::Disabled => FeatureSupportLevel::Unsupported,
        SubpixelTextPolicy::Grayscale => FeatureSupportLevel::Basic,
        SubpixelTextPolicy::Subpixel => FeatureSupportLevel::Full,
    }
}

fn color_support_level(level: ColorManagementLevel) -> FeatureSupportLevel {
    match level {
        ColorManagementLevel::SrgbOnly => FeatureSupportLevel::Basic,
        ColorManagementLevel::DisplayP3 | ColorManagementLevel::ExtendedLinear => {
            FeatureSupportLevel::Full
        }
        ColorManagementLevel::IccProfiles => FeatureSupportLevel::Full,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SortComponent {
    plane_z: i32,
    z_index: i32,
    order: usize,
    kind: u8,
}

impl SortComponent {
    fn context(context: &StackingContext) -> Self {
        Self {
            plane_z: context.layer_order.resolved_z(),
            z_index: context.z_index,
            order: context.order,
            kind: 0,
        }
    }

    const fn layer(layer: &CompositorLayer) -> Self {
        Self {
            plane_z: 0,
            z_index: layer.z_index,
            order: layer.order,
            kind: 1,
        }
    }
}

fn layer_sort_key(layer: &CompositorLayer, contexts: &[StackingContext]) -> Vec<SortComponent> {
    let mut key = context_chain(layer.context, contexts)
        .into_iter()
        .map(SortComponent::context)
        .collect::<Vec<_>>();
    key.push(SortComponent::layer(layer));
    key
}

fn context_chain(
    context_id: StackingContextId,
    contexts: &[StackingContext],
) -> Vec<&StackingContext> {
    let mut current = Some(context_id);
    let mut seen = Vec::new();
    let mut chain = Vec::new();
    while let Some(id) = current {
        if seen.contains(&id) {
            break;
        }
        seen.push(id);
        let Some(context) = contexts.iter().find(|context| context.id == id) else {
            break;
        };
        chain.push(context);
        current = context.parent;
    }
    chain.reverse();
    chain
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
    use crate::platform::{UiLayer, LAYER_LOCAL_Z_MIN};

    fn assert_rect_near(actual: UiRect, expected: UiRect) {
        assert!((actual.x - expected.x).abs() < 0.001, "{actual:?}");
        assert!((actual.y - expected.y).abs() < 0.001, "{actual:?}");
        assert!((actual.width - expected.width).abs() < 0.001, "{actual:?}");
        assert!(
            (actual.height - expected.height).abs() < 0.001,
            "{actual:?}"
        );
    }

    #[test]
    fn layer_sorting_respects_context_z_order_and_platform_layers() {
        let root = StackingContext::root();
        let modal = StackingContext::new(StackingContextId(10))
            .parent(StackingContextId(0))
            .z_index(20)
            .order(1);
        let debug = StackingContext::new(StackingContextId(20))
            .parent(StackingContextId(0))
            .layer_order(LayerOrder::new(UiLayer::DebugOverlay, LAYER_LOCAL_Z_MIN))
            .order(2);
        let layers = vec![
            CompositorLayer::new(CompositorLayerId(1), UiRect::new(0.0, 0.0, 100.0, 100.0))
                .z_index(-10)
                .order(0),
            CompositorLayer::new(CompositorLayerId(2), UiRect::new(0.0, 0.0, 100.0, 100.0))
                .z_index(0)
                .order(1),
            CompositorLayer::new(CompositorLayerId(3), UiRect::new(0.0, 0.0, 100.0, 100.0))
                .context(StackingContextId(10))
                .order(0),
            CompositorLayer::new(CompositorLayerId(4), UiRect::new(0.0, 0.0, 100.0, 100.0))
                .context(StackingContextId(20))
                .order(0),
        ];

        let sorted = sort_layers_for_paint(&layers, &[root, modal, debug]);

        assert_eq!(
            sorted,
            vec![
                CompositorLayerId(1),
                CompositorLayerId(2),
                CompositorLayerId(3),
                CompositorLayerId(4)
            ]
        );
    }

    #[test]
    fn transformed_hit_testing_and_bounds_use_inverse_transform_and_clips() {
        let layer = CompositorLayer::new(CompositorLayerId(7), UiRect::new(0.0, 0.0, 20.0, 10.0))
            .transform(AffineTransform::new(2.0, 0.0, 0.0, 3.0, 10.0, 5.0))
            .clip(CompositorClip::rect(UiRect::new(0.0, 0.0, 10.0, 10.0)));

        assert!(layer.hit_test(UiPoint::new(25.0, 20.0)));
        assert!(!layer.hit_test(UiPoint::new(35.5, 20.0)));
        assert_rect_near(
            layer.transformed_bounds().expect("bounds"),
            UiRect::new(10.0, 5.0, 20.0, 30.0),
        );

        let rotated = AffineTransform::rotation(std::f32::consts::FRAC_PI_2)
            .transform_rect_bounds(UiRect::new(0.0, 0.0, 10.0, 20.0));
        assert_rect_near(rotated, UiRect::new(-20.0, 0.0, 20.0, 10.0));
    }

    #[test]
    fn topmost_layer_hit_testing_walks_sorted_layers_back_to_front() {
        let scene = CompositorScene::new()
            .layer(CompositorLayer::new(
                CompositorLayerId(1),
                UiRect::new(0.0, 0.0, 40.0, 40.0),
            ))
            .layer(
                CompositorLayer::new(CompositorLayerId(2), UiRect::new(0.0, 0.0, 40.0, 40.0))
                    .z_index(5)
                    .transform(AffineTransform::translation(20.0, 0.0)),
            );

        assert_eq!(
            scene.topmost_layer_at(UiPoint::new(25.0, 10.0)),
            Some(CompositorLayerId(2))
        );
        assert_eq!(
            scene.topmost_layer_at(UiPoint::new(5.0, 10.0)),
            Some(CompositorLayerId(1))
        );
    }

    #[test]
    fn feature_fallback_planning_reports_backend_neutral_actions() {
        let support = RenderFeatureSupport {
            shadows: FeatureSupportLevel::Basic,
            rounded_clipping: FeatureSupportLevel::Full,
            borders: FeatureSupportLevel::Full,
            gradients: FeatureSupportLevel::Basic,
            masks: FeatureSupportLevel::Unsupported,
            filters: FeatureSupportLevel::Fallback,
            backdrop_filters: FeatureSupportLevel::Unsupported,
            subpixel_text: SubpixelTextPolicy::Grayscale,
            color_management: ColorManagementLevel::SrgbOnly,
            max_shadow_blur: 4.0,
            max_border_width: 8.0,
            max_gradient_stops: 2,
            max_backdrop_blur: 0.0,
        };
        let requirements = vec![
            RenderFeatureRequirement::Shadow { blur_radius: 12.0 },
            RenderFeatureRequirement::Border { width: 2.0 },
            RenderFeatureRequirement::Gradient {
                stops: 4,
                fallback: ColorRgba::BLACK,
            },
            RenderFeatureRequirement::Filter {
                kind: CompositorFilterKind::Blur,
            },
            RenderFeatureRequirement::BackdropFilter {
                kind: CompositorFilterKind::Blur,
            },
            RenderFeatureRequirement::Mask,
            RenderFeatureRequirement::SubpixelText {
                policy: SubpixelTextPolicy::Subpixel,
            },
            RenderFeatureRequirement::ColorManagement {
                level: ColorManagementLevel::DisplayP3,
            },
        ];

        let plan = plan_render_feature_fallbacks(&requirements, support);

        assert_eq!(
            plan.action_for(RenderFeature::Shadows),
            Some(FeatureFallbackAction::Approximate)
        );
        assert_eq!(
            plan.action_for(RenderFeature::Gradients),
            Some(FeatureFallbackAction::Approximate)
        );
        assert_eq!(
            plan.action_for(RenderFeature::Borders),
            Some(FeatureFallbackAction::Native)
        );
        assert_eq!(
            plan.action_for(RenderFeature::Filters),
            Some(FeatureFallbackAction::RasterizeOffscreen)
        );
        assert_eq!(
            plan.action_for(RenderFeature::BackdropFilters),
            Some(FeatureFallbackAction::Disable)
        );
        assert_eq!(
            plan.action_for(RenderFeature::Masks),
            Some(FeatureFallbackAction::Disable)
        );
        assert_eq!(
            plan.action_for(RenderFeature::SubpixelText),
            Some(FeatureFallbackAction::UseGrayscaleText)
        );
        assert_eq!(
            plan.action_for(RenderFeature::ColorManagement),
            Some(FeatureFallbackAction::ConvertToSrgb)
        );
        assert_eq!(plan.fallback_items().len(), 7);
    }

    #[test]
    fn compositor_quality_requirements_cover_release_gate_features() {
        let features = compositor_quality_requirements()
            .into_iter()
            .map(|requirement| requirement.feature())
            .collect::<Vec<_>>();

        assert!(features.contains(&RenderFeature::Shadows));
        assert!(features.contains(&RenderFeature::RoundedClipping));
        assert!(features.contains(&RenderFeature::Borders));
        assert!(features.contains(&RenderFeature::Gradients));
        assert!(features.contains(&RenderFeature::Masks));
        assert!(features.contains(&RenderFeature::Filters));
        assert!(features.contains(&RenderFeature::BackdropFilters));
        assert!(features.contains(&RenderFeature::SubpixelText));
        assert!(features.contains(&RenderFeature::ColorManagement));
        assert_eq!(features.len(), 9);
    }

    #[test]
    fn compositor_quality_plan_records_current_snapshot_fallback_expectations() {
        let requirements = compositor_quality_requirements();
        let plan = plan_compositor_quality(
            &requirements,
            RenderFeatureSupport::CPU_SNAPSHOT_QUALITY,
            RenderFeatureSupport::WGPU_SNAPSHOT_QUALITY,
        );

        let rounded = plan
            .record_for(RenderFeature::RoundedClipping)
            .expect("rounded clipping record");
        assert_eq!(rounded.wgpu_action, FeatureFallbackAction::Native);
        assert_eq!(
            rounded.parity_expectation,
            CompositorParityExpectation::PixelExact
        );

        let border = plan
            .record_for(RenderFeature::Borders)
            .expect("border record");
        assert_eq!(border.wgpu_action, FeatureFallbackAction::Native);
        assert_eq!(
            border.parity_expectation,
            CompositorParityExpectation::PixelExact
        );

        let gradient = plan
            .record_for(RenderFeature::Gradients)
            .expect("gradient record");
        assert_eq!(gradient.wgpu_action, FeatureFallbackAction::Approximate);
        assert_eq!(
            gradient.parity_expectation,
            CompositorParityExpectation::CpuAuthoritativeFallback
        );

        let shadow = plan
            .record_for(RenderFeature::Shadows)
            .expect("shadow record");
        assert_eq!(shadow.wgpu_action, FeatureFallbackAction::Native);
        assert_eq!(
            shadow.parity_expectation,
            CompositorParityExpectation::ChannelTolerance {
                max_channel_delta: 8
            }
        );

        let mask = plan.record_for(RenderFeature::Masks).expect("mask record");
        assert_eq!(mask.wgpu_action, FeatureFallbackAction::Native);
        assert_eq!(
            mask.parity_expectation,
            CompositorParityExpectation::PixelExact
        );

        let filter = plan
            .record_for(RenderFeature::Filters)
            .expect("filter record");
        assert_eq!(filter.wgpu_action, FeatureFallbackAction::Native);
        assert_eq!(
            filter.parity_expectation,
            CompositorParityExpectation::ChannelTolerance {
                max_channel_delta: 8
            }
        );

        let backdrop = plan
            .record_for(RenderFeature::BackdropFilters)
            .expect("backdrop filter record");
        assert_eq!(backdrop.wgpu_action, FeatureFallbackAction::Disable);
        assert_eq!(
            backdrop.parity_expectation,
            CompositorParityExpectation::Unsupported
        );

        let color = plan
            .record_for(RenderFeature::ColorManagement)
            .expect("color management record");
        assert_eq!(color.wgpu_action, FeatureFallbackAction::Native);
        assert_eq!(
            color.parity_expectation,
            CompositorParityExpectation::PixelExact
        );

        let text = plan
            .record_for(RenderFeature::SubpixelText)
            .expect("subpixel text record");
        assert_eq!(text.wgpu_action, FeatureFallbackAction::Native);
        assert_eq!(
            text.parity_expectation,
            CompositorParityExpectation::CpuAuthoritativeFallback
        );
        assert_eq!(plan.wgpu_fallback_records().len(), 2);
    }

    #[test]
    fn border_fallback_planning_tracks_native_limits() {
        let support = RenderFeatureSupport {
            borders: FeatureSupportLevel::Basic,
            max_border_width: 1.0,
            ..RenderFeatureSupport::NONE
        };
        let requirements = vec![RenderFeatureRequirement::Border { width: 4.0 }];
        let plan = plan_render_feature_fallbacks(&requirements, support);

        assert_eq!(
            plan.action_for(RenderFeature::Borders),
            Some(FeatureFallbackAction::Approximate)
        );
    }
}
