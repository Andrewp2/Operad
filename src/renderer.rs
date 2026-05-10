//! Renderer/backend adapter contracts for Operad paint lists.
//!
//! This module sits between `PaintList` and concrete backends such as egui,
//! wgpu, CPU snapshot renderers, or app-owned renderers. It keeps batching,
//! resource updates, dirty regions, deterministic snapshots, and adapter
//! capabilities out of product state.

use std::collections::HashMap;

use crate::accessibility::AccessibilityPreferences;
use crate::host::HostNodeInteraction;
use crate::platform::{
    BackendCapabilities, CursorRequest, LayerOrder, LogicalRect, PixelSize, PlatformRequest,
    ResourceHandle, ResourceId, ResourceKind,
};
use crate::{
    CanvasContent, ColorRgba, DirtyFlags, FrameTiming, PaintImage, PaintItem, PaintKind, PaintList,
    PaintTransform, ShaderEffect, UiNodeId, UiRect, UiSize,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceFormat {
    Rgba8,
    Bgra8,
    Alpha8,
}

impl ResourceFormat {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba8 | Self::Bgra8 => 4,
            Self::Alpha8 => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub const fn right(self) -> u32 {
        self.x.saturating_add(self.width)
    }

    pub const fn bottom(self) -> u32 {
        self.y.saturating_add(self.height)
    }

    pub const fn contains(self, size: PixelSize) -> bool {
        self.right() <= size.width && self.bottom() <= size.height
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDescriptor {
    pub handle: ResourceHandle,
    pub size: PixelSize,
    pub format: ResourceFormat,
    pub version: u64,
}

impl ResourceDescriptor {
    pub fn new(handle: ResourceHandle, size: PixelSize, format: ResourceFormat) -> Self {
        Self {
            handle,
            size,
            format,
            version: 0,
        }
    }

    pub fn version(mut self, version: u64) -> Self {
        self.version = version;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUpdate {
    pub descriptor: ResourceDescriptor,
    pub dirty_rect: Option<PixelRect>,
    pub bytes: Vec<u8>,
}

impl ResourceUpdate {
    pub fn full(descriptor: ResourceDescriptor, bytes: Vec<u8>) -> Self {
        Self {
            descriptor,
            dirty_rect: None,
            bytes,
        }
    }

    pub fn partial(descriptor: ResourceDescriptor, dirty_rect: PixelRect, bytes: Vec<u8>) -> Self {
        Self {
            descriptor,
            dirty_rect: Some(dirty_rect),
            bytes,
        }
    }

    pub fn is_partial(&self) -> bool {
        self.dirty_rect.is_some()
    }

    pub fn expected_byte_len(&self) -> Option<usize> {
        let pixels = match self.dirty_rect {
            Some(rect) => usize::try_from(rect.width)
                .ok()?
                .checked_mul(usize::try_from(rect.height).ok()?)?,
            None => usize::try_from(self.descriptor.size.width)
                .ok()?
                .checked_mul(usize::try_from(self.descriptor.size.height).ok()?)?,
        };
        pixels.checked_mul(self.descriptor.format.bytes_per_pixel())
    }

    pub fn has_expected_byte_len(&self) -> bool {
        self.expected_byte_len()
            .is_some_and(|expected| expected == self.bytes.len())
    }

    pub fn dirty_rect_is_valid(&self) -> bool {
        self.dirty_rect
            .map(|rect| !rect.is_empty() && rect.contains(self.descriptor.size))
            .unwrap_or(true)
    }
}

pub trait ResourceResolver {
    fn resolve_resource(&self, id: &ResourceId) -> Option<ResourceDescriptor>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderTargetKind {
    Window,
    Offscreen,
    Snapshot,
    AppOwned,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderTarget {
    Window {
        id: String,
        size: UiSize,
    },
    Offscreen {
        label: Option<String>,
        size: PixelSize,
    },
    Snapshot {
        label: Option<String>,
        size: PixelSize,
    },
    AppOwned {
        id: String,
        size: UiSize,
    },
}

impl RenderTarget {
    pub fn window(id: impl Into<String>, size: UiSize) -> Self {
        Self::Window {
            id: id.into(),
            size,
        }
    }

    pub fn offscreen(size: PixelSize) -> Self {
        Self::Offscreen { label: None, size }
    }

    pub fn snapshot(size: PixelSize) -> Self {
        Self::Snapshot { label: None, size }
    }

    pub fn app_owned(id: impl Into<String>, size: UiSize) -> Self {
        Self::AppOwned {
            id: id.into(),
            size,
        }
    }

    pub const fn kind(&self) -> RenderTargetKind {
        match self {
            Self::Window { .. } => RenderTargetKind::Window,
            Self::Offscreen { .. } => RenderTargetKind::Offscreen,
            Self::Snapshot { .. } => RenderTargetKind::Snapshot,
            Self::AppOwned { .. } => RenderTargetKind::AppOwned,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DirtyRegionSet {
    pub regions: Vec<UiRect>,
}

impl DirtyRegionSet {
    pub fn empty() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    pub fn full(viewport: UiSize) -> Self {
        Self {
            regions: vec![UiRect::new(0.0, 0.0, viewport.width, viewport.height)],
        }
    }

    pub fn push(&mut self, region: UiRect) -> bool {
        if !rect_is_finite(region) || region.width <= 0.0 || region.height <= 0.0 {
            return false;
        }
        if self.regions.contains(&region) {
            return false;
        }
        self.regions.push(region);
        true
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    pub fn covers(&self, rect: UiRect) -> bool {
        self.regions.iter().any(|region| region.contains_rect(rect))
    }
}

impl Default for DirtyRegionSet {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    pub scale_factor: f32,
    pub deterministic: bool,
    pub allow_partial_updates: bool,
    pub clear_color: ColorRgba,
    pub accessibility_preferences: AccessibilityPreferences,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            scale_factor: 1.0,
            deterministic: false,
            allow_partial_updates: true,
            clear_color: ColorRgba::TRANSPARENT,
            accessibility_preferences: AccessibilityPreferences::DEFAULT,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFrameRequest {
    pub target: RenderTarget,
    pub viewport: UiSize,
    pub paint: PaintList,
    pub dirty_regions: DirtyRegionSet,
    pub resource_updates: Vec<ResourceUpdate>,
    pub node_interactions: HashMap<UiNodeId, HostNodeInteraction>,
    pub dirty_flags: DirtyFlags,
    pub options: RenderOptions,
}

impl RenderFrameRequest {
    pub fn new(target: RenderTarget, viewport: UiSize, paint: PaintList) -> Self {
        Self {
            target,
            viewport,
            paint,
            dirty_regions: DirtyRegionSet::full(viewport),
            resource_updates: Vec::new(),
            node_interactions: HashMap::new(),
            dirty_flags: DirtyFlags::ALL,
            options: RenderOptions::default(),
        }
    }

    pub fn dirty_regions(mut self, dirty_regions: DirtyRegionSet) -> Self {
        self.dirty_regions = dirty_regions;
        self
    }

    pub fn resource_update(mut self, update: ResourceUpdate) -> Self {
        self.resource_updates.push(update);
        self
    }

    pub fn node_interaction(mut self, node: UiNodeId, interaction: HostNodeInteraction) -> Self {
        self.node_interactions.insert(node, interaction);
        self
    }

    pub fn node_interactions(
        mut self,
        interactions: impl IntoIterator<Item = (UiNodeId, HostNodeInteraction)>,
    ) -> Self {
        self.node_interactions.extend(interactions);
        self
    }

    pub fn interaction_for(&self, node: UiNodeId) -> HostNodeInteraction {
        self.node_interactions
            .get(&node)
            .copied()
            .unwrap_or_default()
    }

    pub fn dirty_flags(mut self, dirty_flags: DirtyFlags) -> Self {
        self.dirty_flags = dirty_flags;
        self
    }

    pub fn options(mut self, options: RenderOptions) -> Self {
        self.options = options;
        self
    }

    pub fn batches(&self) -> Vec<PaintBatch> {
        PaintBatcher::default().batch(&self.paint)
    }

    pub fn canvas_requests(&self) -> Vec<CanvasRenderRequest> {
        self.paint
            .items
            .iter()
            .filter_map(CanvasRenderRequest::from_paint_item)
            .collect()
    }

    pub fn canvas_host_capture_plans(&self) -> Vec<CanvasHostCapturePlan> {
        self.canvas_requests()
            .into_iter()
            .filter(|request| request.requires_host_input_capture())
            .map(|request| request.host_capture_plan())
            .collect()
    }

    pub fn canvas_platform_requests(&self) -> Vec<PlatformRequest> {
        self.canvas_host_capture_plans()
            .into_iter()
            .flat_map(|plan| plan.platform_requests())
            .collect()
    }

    pub fn image_requests(&self) -> Vec<ImageRenderRequest> {
        self.paint
            .items
            .iter()
            .filter_map(ImageRenderRequest::from_paint_item)
            .collect()
    }

    pub fn requires_full_repaint(&self) -> bool {
        self.dirty_regions.is_empty()
            || self.dirty_flags.layout
            || self.dirty_flags.theme
            || self.dirty_flags.text_measurement
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasRenderRequest {
    pub node: UiNodeId,
    pub canvas: CanvasContent,
    pub rect: UiRect,
    pub clip_rect: UiRect,
    pub z_index: i16,
    pub layer_order: LayerOrder,
    pub opacity: f32,
    pub transform: PaintTransform,
}

impl CanvasRenderRequest {
    pub fn from_paint_item(item: &PaintItem) -> Option<Self> {
        let PaintKind::Canvas(canvas) = &item.kind else {
            return None;
        };
        Some(Self {
            node: item.node,
            canvas: canvas.clone(),
            rect: item.rect,
            clip_rect: item.clip_rect,
            z_index: item.z_index,
            layer_order: item.layer_order,
            opacity: item.opacity,
            transform: item.transform,
        })
    }

    pub const fn requires_host_input_capture(&self) -> bool {
        self.canvas.requires_host_input_capture()
    }

    pub fn host_capture_plan(&self) -> CanvasHostCapturePlan {
        CanvasHostCapturePlan::from_request(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasHostCapturePlan {
    pub node: UiNodeId,
    pub key: String,
    pub rect: UiRect,
    pub pointer_capture: bool,
    pub keyboard_capture: bool,
    pub wheel_capture: bool,
    pub pointer_lock: bool,
    pub domain_hit_testing: bool,
}

impl CanvasHostCapturePlan {
    pub fn from_request(request: &CanvasRenderRequest) -> Self {
        let interaction = request.canvas.interaction;
        Self {
            node: request.node,
            key: request.canvas.key.clone(),
            rect: request.rect,
            pointer_capture: interaction.pointer_capture,
            keyboard_capture: interaction.keyboard_capture,
            wheel_capture: interaction.wheel_capture,
            pointer_lock: interaction.pointer_lock,
            domain_hit_testing: interaction.domain_hit_testing,
        }
    }

    pub const fn requires_host_capture(&self) -> bool {
        self.pointer_capture || self.keyboard_capture || self.wheel_capture || self.pointer_lock
    }

    pub fn cursor_confine_rect(&self) -> Option<LogicalRect> {
        self.pointer_lock.then(|| ui_rect_to_logical(self.rect))
    }

    pub fn platform_requests(&self) -> Vec<PlatformRequest> {
        let Some(rect) = self.cursor_confine_rect() else {
            return Vec::new();
        };
        vec![
            PlatformRequest::Cursor(CursorRequest::Confine(rect)),
            PlatformRequest::Cursor(CursorRequest::SetVisible(false)),
        ]
    }

    pub fn release_platform_requests(&self) -> Vec<PlatformRequest> {
        if self.pointer_lock {
            vec![
                PlatformRequest::Cursor(CursorRequest::ReleaseConfine),
                PlatformRequest::Cursor(CursorRequest::SetVisible(true)),
            ]
        } else {
            Vec::new()
        }
    }
}

#[derive(Debug)]
pub struct CanvasRenderContext<'a, B> {
    pub request: &'a CanvasRenderRequest,
    pub scale_factor: f32,
    pub dirty_regions: &'a DirtyRegionSet,
    pub interaction: HostNodeInteraction,
    pub backend: &'a mut B,
}

impl<B> CanvasRenderContext<'_, B> {
    pub fn is_dirty(&self) -> bool {
        self.dirty_regions.is_empty() || self.dirty_regions.covers(self.request.rect)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasRenderOutput {
    pub dirty_region: Option<UiRect>,
    pub resource_updates: Vec<ResourceUpdate>,
    pub repaint_requested: bool,
}

impl CanvasRenderOutput {
    pub fn new() -> Self {
        Self {
            dirty_region: None,
            resource_updates: Vec::new(),
            repaint_requested: false,
        }
    }

    pub fn dirty_region(mut self, dirty_region: UiRect) -> Self {
        self.dirty_region = Some(dirty_region);
        self
    }

    pub fn resource_update(mut self, update: ResourceUpdate) -> Self {
        self.resource_updates.push(update);
        self
    }

    pub fn repaint_requested(mut self, repaint_requested: bool) -> Self {
        self.repaint_requested = repaint_requested;
        self
    }
}

impl Default for CanvasRenderOutput {
    fn default() -> Self {
        Self::new()
    }
}

pub trait CanvasRenderHandler<B> {
    fn render_canvas(
        &mut self,
        context: CanvasRenderContext<'_, B>,
    ) -> Result<CanvasRenderOutput, RenderError>;
}

impl<B, F> CanvasRenderHandler<B> for F
where
    F: for<'a> FnMut(CanvasRenderContext<'a, B>) -> Result<CanvasRenderOutput, RenderError>,
{
    fn render_canvas(
        &mut self,
        context: CanvasRenderContext<'_, B>,
    ) -> Result<CanvasRenderOutput, RenderError> {
        self(context)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CanvasRenderOutcome {
    Rendered {
        request: CanvasRenderRequest,
        output: CanvasRenderOutput,
    },
    Missing {
        request: CanvasRenderRequest,
    },
    Failed {
        request: CanvasRenderRequest,
        error: RenderError,
    },
}

impl CanvasRenderOutcome {
    pub const fn request(&self) -> &CanvasRenderRequest {
        match self {
            Self::Rendered { request, .. }
            | Self::Missing { request }
            | Self::Failed { request, .. } => request,
        }
    }

    pub const fn is_rendered(&self) -> bool {
        matches!(self, Self::Rendered { .. })
    }

    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing { .. })
    }

    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CanvasRenderReport {
    pub outcomes: Vec<CanvasRenderOutcome>,
}

impl CanvasRenderReport {
    pub fn rendered_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.is_rendered())
            .count()
    }

    pub fn missing_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.is_missing())
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.is_failed())
            .count()
    }

    pub fn repaint_requested(&self) -> bool {
        self.outcomes.iter().any(|outcome| {
            matches!(
                outcome,
                CanvasRenderOutcome::Rendered {
                    output: CanvasRenderOutput {
                        repaint_requested: true,
                        ..
                    },
                    ..
                }
            )
        })
    }

    pub fn first_failure(&self) -> Option<&RenderError> {
        self.outcomes.iter().find_map(|outcome| match outcome {
            CanvasRenderOutcome::Failed { error, .. } => Some(error),
            _ => None,
        })
    }

    pub fn first_missing(&self) -> Option<&CanvasRenderRequest> {
        self.outcomes.iter().find_map(|outcome| match outcome {
            CanvasRenderOutcome::Missing { request } => Some(request),
            _ => None,
        })
    }

    pub fn resource_updates(&self) -> Vec<ResourceUpdate> {
        let mut updates = Vec::new();
        for outcome in &self.outcomes {
            if let CanvasRenderOutcome::Rendered { output, .. } = outcome {
                updates.extend(output.resource_updates.iter().cloned());
            }
        }
        updates
    }

    pub fn into_strict_result(self) -> Result<Self, RenderError> {
        if let Some(error) = self.first_failure().cloned() {
            return Err(error);
        }
        if let Some(missing) = self.first_missing() {
            return Err(RenderError::MissingCanvasRenderer(
                missing.canvas.key.clone(),
            ));
        }
        Ok(self)
    }
}

pub struct CanvasRenderRegistry<B> {
    handlers: HashMap<String, Box<dyn CanvasRenderHandler<B>>>,
}

impl<B> CanvasRenderRegistry<B> {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        key: impl Into<String>,
        handler: impl CanvasRenderHandler<B> + 'static,
    ) -> bool {
        self.handlers
            .insert(key.into(), Box::new(handler))
            .is_some()
    }

    pub fn unregister(&mut self, key: &str) -> bool {
        self.handlers.remove(key).is_some()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.handlers.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub fn render_frame_canvases(
        &mut self,
        request: &RenderFrameRequest,
        backend: &mut B,
    ) -> CanvasRenderReport {
        let mut report = CanvasRenderReport::default();
        for canvas_request in request.canvas_requests() {
            let Some(handler) = self.handlers.get_mut(&canvas_request.canvas.key) else {
                report.outcomes.push(CanvasRenderOutcome::Missing {
                    request: canvas_request,
                });
                continue;
            };
            let outcome = match handler.render_canvas(CanvasRenderContext {
                scale_factor: request.options.scale_factor,
                dirty_regions: &request.dirty_regions,
                interaction: request.interaction_for(canvas_request.node),
                request: &canvas_request,
                backend,
            }) {
                Ok(output) => CanvasRenderOutcome::Rendered {
                    request: canvas_request,
                    output,
                },
                Err(error) => CanvasRenderOutcome::Failed {
                    request: canvas_request,
                    error,
                },
            };
            report.outcomes.push(outcome);
        }
        report
    }

    pub fn render_frame_canvases_strict(
        &mut self,
        request: &RenderFrameRequest,
        backend: &mut B,
    ) -> Result<CanvasRenderReport, RenderError> {
        self.render_frame_canvases(request, backend)
            .into_strict_result()
    }
}

impl<B> Default for CanvasRenderRegistry<B> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageRenderKind {
    NodeImage,
    ImagePlacement,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageRenderRequest {
    pub node: UiNodeId,
    pub kind: ImageRenderKind,
    pub image: PaintImage,
    pub clip_rect: UiRect,
    pub z_index: i16,
    pub layer_order: LayerOrder,
    pub opacity: f32,
    pub transform: PaintTransform,
}

impl ImageRenderRequest {
    pub fn from_paint_item(item: &PaintItem) -> Option<Self> {
        let (kind, image) = match &item.kind {
            PaintKind::Image { key, tint } => {
                let mut image = PaintImage::new(key.clone(), item.rect);
                if let Some(tint) = *tint {
                    image = image.tinted(tint);
                }
                (ImageRenderKind::NodeImage, image)
            }
            PaintKind::ImagePlacement(image) => (ImageRenderKind::ImagePlacement, image.clone()),
            _ => return None,
        };
        Some(Self {
            node: item.node,
            kind,
            image,
            clip_rect: item.clip_rect,
            z_index: item.z_index,
            layer_order: item.layer_order,
            opacity: item.opacity,
            transform: item.transform,
        })
    }

    pub fn key(&self) -> &str {
        &self.image.key
    }
}

#[derive(Debug)]
pub struct ImageRenderContext<'a, B> {
    pub request: &'a ImageRenderRequest,
    pub scale_factor: f32,
    pub dirty_regions: &'a DirtyRegionSet,
    pub interaction: HostNodeInteraction,
    pub backend: &'a mut B,
}

impl<B> ImageRenderContext<'_, B> {
    pub fn is_dirty(&self) -> bool {
        self.dirty_regions.is_empty() || self.dirty_regions.covers(self.request.image.rect)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageRenderOutput {
    pub dirty_region: Option<UiRect>,
    pub resource_updates: Vec<ResourceUpdate>,
    pub repaint_requested: bool,
}

impl ImageRenderOutput {
    pub fn new() -> Self {
        Self {
            dirty_region: None,
            resource_updates: Vec::new(),
            repaint_requested: false,
        }
    }

    pub fn dirty_region(mut self, dirty_region: UiRect) -> Self {
        self.dirty_region = Some(dirty_region);
        self
    }

    pub fn resource_update(mut self, update: ResourceUpdate) -> Self {
        self.resource_updates.push(update);
        self
    }

    pub fn repaint_requested(mut self, repaint_requested: bool) -> Self {
        self.repaint_requested = repaint_requested;
        self
    }
}

impl Default for ImageRenderOutput {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ImageRenderHandler<B> {
    fn render_image(
        &mut self,
        context: ImageRenderContext<'_, B>,
    ) -> Result<ImageRenderOutput, RenderError>;
}

impl<B, F> ImageRenderHandler<B> for F
where
    F: for<'a> FnMut(ImageRenderContext<'a, B>) -> Result<ImageRenderOutput, RenderError>,
{
    fn render_image(
        &mut self,
        context: ImageRenderContext<'_, B>,
    ) -> Result<ImageRenderOutput, RenderError> {
        self(context)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageRenderOutcome {
    Rendered {
        request: ImageRenderRequest,
        output: ImageRenderOutput,
    },
    Missing {
        request: ImageRenderRequest,
    },
    Failed {
        request: ImageRenderRequest,
        error: RenderError,
    },
}

impl ImageRenderOutcome {
    pub const fn request(&self) -> &ImageRenderRequest {
        match self {
            Self::Rendered { request, .. }
            | Self::Missing { request }
            | Self::Failed { request, .. } => request,
        }
    }

    pub const fn is_rendered(&self) -> bool {
        matches!(self, Self::Rendered { .. })
    }

    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing { .. })
    }

    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageRenderReport {
    pub outcomes: Vec<ImageRenderOutcome>,
}

impl ImageRenderReport {
    pub fn rendered_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.is_rendered())
            .count()
    }

    pub fn missing_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.is_missing())
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.is_failed())
            .count()
    }

    pub fn repaint_requested(&self) -> bool {
        self.outcomes.iter().any(|outcome| {
            matches!(
                outcome,
                ImageRenderOutcome::Rendered {
                    output: ImageRenderOutput {
                        repaint_requested: true,
                        ..
                    },
                    ..
                }
            )
        })
    }

    pub fn first_failure(&self) -> Option<&RenderError> {
        self.outcomes.iter().find_map(|outcome| match outcome {
            ImageRenderOutcome::Failed { error, .. } => Some(error),
            _ => None,
        })
    }

    pub fn first_missing(&self) -> Option<&ImageRenderRequest> {
        self.outcomes.iter().find_map(|outcome| match outcome {
            ImageRenderOutcome::Missing { request } => Some(request),
            _ => None,
        })
    }

    pub fn resource_updates(&self) -> Vec<ResourceUpdate> {
        let mut updates = Vec::new();
        for outcome in &self.outcomes {
            if let ImageRenderOutcome::Rendered { output, .. } = outcome {
                updates.extend(output.resource_updates.iter().cloned());
            }
        }
        updates
    }

    pub fn into_strict_result(self) -> Result<Self, RenderError> {
        if let Some(error) = self.first_failure().cloned() {
            return Err(error);
        }
        if let Some(missing) = self.first_missing() {
            return Err(RenderError::MissingImageRenderer(missing.image.key.clone()));
        }
        Ok(self)
    }
}

pub struct ImageRenderRegistry<B> {
    handlers: HashMap<String, Box<dyn ImageRenderHandler<B>>>,
}

impl<B> ImageRenderRegistry<B> {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        key: impl Into<String>,
        handler: impl ImageRenderHandler<B> + 'static,
    ) -> bool {
        self.handlers
            .insert(key.into(), Box::new(handler))
            .is_some()
    }

    pub fn unregister(&mut self, key: &str) -> bool {
        self.handlers.remove(key).is_some()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.handlers.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub fn render_frame_images(
        &mut self,
        request: &RenderFrameRequest,
        backend: &mut B,
    ) -> ImageRenderReport {
        let mut report = ImageRenderReport::default();
        for image_request in request.image_requests() {
            let Some(handler) = self.handlers.get_mut(&image_request.image.key) else {
                report.outcomes.push(ImageRenderOutcome::Missing {
                    request: image_request,
                });
                continue;
            };
            let outcome = match handler.render_image(ImageRenderContext {
                scale_factor: request.options.scale_factor,
                dirty_regions: &request.dirty_regions,
                interaction: request.interaction_for(image_request.node),
                request: &image_request,
                backend,
            }) {
                Ok(output) => ImageRenderOutcome::Rendered {
                    request: image_request,
                    output,
                },
                Err(error) => ImageRenderOutcome::Failed {
                    request: image_request,
                    error,
                },
            };
            report.outcomes.push(outcome);
        }
        report
    }

    pub fn render_frame_images_strict(
        &mut self,
        request: &RenderFrameRequest,
        backend: &mut B,
    ) -> Result<ImageRenderReport, RenderError> {
        self.render_frame_images(request, backend)
            .into_strict_result()
    }
}

impl<B> Default for ImageRenderRegistry<B> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaintBatchKind {
    Rect,
    Text,
    Canvas,
    Line,
    Circle,
    Polygon,
    Image,
    RichRect,
    SceneText,
    Path,
    ImagePlacement,
}

impl PaintBatchKind {
    pub const fn from_kind(kind: &PaintKind) -> Self {
        match kind {
            PaintKind::Rect { .. } => Self::Rect,
            PaintKind::Text(_) => Self::Text,
            PaintKind::Canvas(_) => Self::Canvas,
            PaintKind::Line { .. } => Self::Line,
            PaintKind::Circle { .. } => Self::Circle,
            PaintKind::Polygon { .. } => Self::Polygon,
            PaintKind::Image { .. } => Self::Image,
            PaintKind::RichRect(_) => Self::RichRect,
            PaintKind::SceneText(_) => Self::SceneText,
            PaintKind::Path(_) => Self::Path,
            PaintKind::ImagePlacement(_) => Self::ImagePlacement,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaintBatchKey {
    pub kind: PaintBatchKind,
    pub z_index: i16,
    pub layer_order: LayerOrder,
    pub clip_rect: UiRect,
    pub shader: Option<ShaderEffect>,
}

impl PaintBatchKey {
    pub fn from_item(item: &PaintItem) -> Self {
        Self {
            kind: PaintBatchKind::from_kind(&item.kind),
            z_index: item.z_index,
            layer_order: item.layer_order,
            clip_rect: item.clip_rect,
            shader: item.shader.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaintBatch {
    pub key: PaintBatchKey,
    pub item_indices: Vec<usize>,
    pub bounds: UiRect,
}

impl PaintBatch {
    fn new(index: usize, item: &PaintItem) -> Self {
        Self {
            key: PaintBatchKey::from_item(item),
            item_indices: vec![index],
            bounds: item.rect,
        }
    }

    fn try_push(&mut self, index: usize, item: &PaintItem) -> bool {
        if self.key != PaintBatchKey::from_item(item) {
            return false;
        }
        self.item_indices.push(index);
        self.bounds = union_rect(self.bounds, item.rect);
        true
    }

    pub fn len(&self) -> usize {
        self.item_indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.item_indices.is_empty()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PaintBatcher {
    pub preserve_order: bool,
}

impl PaintBatcher {
    pub fn batch(self, paint: &PaintList) -> Vec<PaintBatch> {
        let mut batches = Vec::<PaintBatch>::new();
        for (index, item) in paint.items.iter().enumerate() {
            if self.preserve_order
                || !batches
                    .last_mut()
                    .is_some_and(|batch| batch.try_push(index, item))
            {
                batches.push(PaintBatch::new(index, item));
            }
        }
        batches
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    UnsupportedTarget(RenderTargetKind),
    UnsupportedResource(ResourceKind),
    MissingResource(ResourceId),
    MissingCanvasRenderer(String),
    MissingImageRenderer(String),
    InvalidResourceUpdate(String),
    Backend(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedTarget(target) => {
                write!(formatter, "unsupported render target {target:?}")
            }
            Self::UnsupportedResource(resource) => {
                write!(formatter, "unsupported render resource {resource:?}")
            }
            Self::MissingResource(resource) => {
                write!(formatter, "missing render resource {:?}", resource.key)
            }
            Self::MissingCanvasRenderer(key) => {
                write!(formatter, "missing canvas renderer for {key:?}")
            }
            Self::MissingImageRenderer(key) => {
                write!(formatter, "missing image renderer for {key:?}")
            }
            Self::InvalidResourceUpdate(reason) => {
                write!(formatter, "invalid render resource update: {reason}")
            }
            Self::Backend(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for RenderError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedImage {
    pub size: PixelSize,
    pub format: ResourceFormat,
    pub pixels: Vec<u8>,
}

impl RenderedImage {
    pub fn new(size: PixelSize, format: ResourceFormat, pixels: Vec<u8>) -> Self {
        Self {
            size,
            format,
            pixels,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFrameOutput {
    pub target: RenderTarget,
    pub painted_items: usize,
    pub batches: Vec<PaintBatch>,
    pub dirty_regions: DirtyRegionSet,
    pub timings: FrameTiming,
    pub snapshot: Option<RenderedImage>,
}

impl RenderFrameOutput {
    pub fn new(target: RenderTarget) -> Self {
        Self {
            target,
            painted_items: 0,
            batches: Vec::new(),
            dirty_regions: DirtyRegionSet::default(),
            timings: FrameTiming::default(),
            snapshot: None,
        }
    }
}

pub trait RendererAdapter {
    fn capabilities(&self) -> BackendCapabilities;

    fn render_frame(
        &mut self,
        request: RenderFrameRequest,
        resolver: &dyn ResourceResolver,
    ) -> Result<RenderFrameOutput, RenderError>;
}

fn rect_is_finite(rect: UiRect) -> bool {
    rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite()
}

fn ui_rect_to_logical(rect: UiRect) -> LogicalRect {
    LogicalRect::new(rect.x, rect.y, rect.width, rect.height)
}

fn union_rect(a: UiRect, b: UiRect) -> UiRect {
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = a.right().max(b.right());
    let bottom = a.bottom().max(b.bottom());
    UiRect::new(left, top, right - left, bottom - top)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::platform::{
        BackendAdapterKind, ImageHandle, RenderingCapabilities, ResourceCapabilities,
        ResourceDomain,
    };
    use crate::{
        CanvasContent, CanvasInteractionPolicy, CanvasRenderMode, ImageAlignment, ImageFit,
        PaintImage, PaintTransform, ShaderEffect, StrokeStyle, TextContent, TextStyle, UiNodeId,
    };

    fn paint_item(index: usize, rect: UiRect, kind: PaintKind) -> PaintItem {
        PaintItem {
            node: UiNodeId(index),
            rect,
            clip_rect: UiRect::new(0.0, 0.0, 200.0, 100.0),
            z_index: 0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            kind,
        }
    }

    #[test]
    fn render_options_default_to_neutral_accessibility_preferences() {
        assert_eq!(
            RenderOptions::default().accessibility_preferences,
            AccessibilityPreferences::DEFAULT
        );
    }

    #[test]
    fn resource_updates_validate_full_and_partial_texture_deltas() {
        let descriptor = ResourceDescriptor::new(
            ResourceHandle::Image(ImageHandle::app("menu.thumbnail")),
            PixelSize::new(4, 4),
            ResourceFormat::Rgba8,
        )
        .version(7);

        let full = ResourceUpdate::full(descriptor.clone(), vec![0; 4 * 4 * 4]);
        assert!(!full.is_partial());
        assert_eq!(full.expected_byte_len(), Some(64));
        assert!(full.has_expected_byte_len());
        assert!(full.dirty_rect_is_valid());

        let partial = ResourceUpdate::partial(
            descriptor.clone(),
            PixelRect::new(1, 1, 2, 2),
            vec![255; 2 * 2 * 4],
        );
        assert!(partial.is_partial());
        assert_eq!(partial.expected_byte_len(), Some(16));
        assert!(partial.has_expected_byte_len());
        assert!(partial.dirty_rect_is_valid());

        let invalid =
            ResourceUpdate::partial(descriptor, PixelRect::new(3, 3, 2, 2), vec![0; 2 * 2 * 4]);
        assert!(!invalid.dirty_rect_is_valid());
    }

    #[test]
    fn paint_batcher_groups_contiguous_items_by_kind_clip_z_and_shader() {
        let rect = PaintKind::Rect {
            fill: ColorRgba::new(20, 20, 20, 255),
            stroke: Some(StrokeStyle::new(ColorRgba::new(90, 90, 90, 255), 1.0)),
            corner_radius: 4.0,
        };
        let mut paint = PaintList::default();
        paint.items.push(paint_item(
            0,
            UiRect::new(0.0, 0.0, 20.0, 20.0),
            rect.clone(),
        ));
        paint.items.push(paint_item(
            1,
            UiRect::new(16.0, 0.0, 20.0, 20.0),
            rect.clone(),
        ));
        let mut debug_overlay_item = paint_item(3, UiRect::new(24.0, 0.0, 20.0, 20.0), rect);
        debug_overlay_item.layer_order = LayerOrder::new(crate::platform::UiLayer::DebugOverlay, 0);
        paint.items.push(debug_overlay_item);
        let mut shader_item = paint_item(
            2,
            UiRect::new(40.0, 0.0, 20.0, 20.0),
            PaintKind::Text(TextContent::new("A", TextStyle::default())),
        );
        shader_item.shader = Some(ShaderEffect::new("text.glow"));
        paint.items.push(shader_item);

        let batches = PaintBatcher::default().batch(&paint);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].key.kind, PaintBatchKind::Rect);
        assert_eq!(batches[0].key.layer_order, LayerOrder::DEFAULT);
        assert_eq!(batches[0].item_indices, vec![0, 1]);
        assert_eq!(batches[0].bounds, UiRect::new(0.0, 0.0, 36.0, 20.0));
        assert_eq!(
            batches[1].key.layer_order,
            LayerOrder::new(crate::platform::UiLayer::DebugOverlay, 0)
        );
        assert_eq!(batches[2].key.kind, PaintBatchKind::Text);
        assert_eq!(batches[2].key.shader.as_ref().unwrap().key, "text.glow");

        let unbatched = PaintBatcher {
            preserve_order: true,
        }
        .batch(&paint);
        assert_eq!(unbatched.len(), 4);
    }

    #[test]
    fn render_request_tracks_dirty_regions_batches_and_full_repaint_policy() {
        let mut paint = PaintList::default();
        paint.items.push(paint_item(
            0,
            UiRect::new(10.0, 10.0, 20.0, 20.0),
            PaintKind::Image {
                key: "icons.play".to_string(),
                tint: None,
            },
        ));
        let mut dirty = DirtyRegionSet::empty();
        assert!(dirty.push(UiRect::new(8.0, 8.0, 24.0, 24.0)));

        let request = RenderFrameRequest::new(
            RenderTarget::snapshot(PixelSize::new(128, 64)),
            UiSize::new(128.0, 64.0),
            paint,
        )
        .dirty_regions(dirty.clone())
        .dirty_flags(DirtyFlags {
            paint: true,
            ..DirtyFlags::NONE
        });

        assert!(!request.requires_full_repaint());
        assert!(request
            .dirty_regions
            .covers(UiRect::new(10.0, 10.0, 20.0, 20.0)));
        assert_eq!(request.batches().len(), 1);

        let full = request.clone().dirty_flags(DirtyFlags {
            layout: true,
            ..DirtyFlags::NONE
        });
        assert!(full.requires_full_repaint());
    }

    #[test]
    fn render_request_extracts_embedded_canvas_requests() {
        let canvas = CanvasContent::new("fabricad.mask.viewport")
            .native_viewport()
            .interaction(CanvasInteractionPolicy::NATIVE_VIEWPORT);
        let mut paint = PaintList::default();
        paint.items.push(paint_item(
            7,
            UiRect::new(12.0, 16.0, 320.0, 180.0),
            PaintKind::Canvas(canvas),
        ));

        let request = RenderFrameRequest::new(
            RenderTarget::app_owned("main", UiSize::new(640.0, 480.0)),
            UiSize::new(640.0, 480.0),
            paint,
        );
        let canvases = request.canvas_requests();

        assert_eq!(canvases.len(), 1);
        assert_eq!(canvases[0].node, UiNodeId(7));
        assert_eq!(canvases[0].canvas.key, "fabricad.mask.viewport");
        assert_eq!(
            canvases[0].canvas.render_mode,
            CanvasRenderMode::NativeViewport
        );
        assert!(canvases[0].requires_host_input_capture());
        assert!(canvases[0].canvas.interaction.pointer_lock);
        assert!(canvases[0].canvas.interaction.domain_hit_testing);
        assert_eq!(canvases[0].rect, UiRect::new(12.0, 16.0, 320.0, 180.0));
    }

    #[test]
    fn canvas_host_capture_plan_maps_pointer_lock_to_cursor_requests() {
        let canvas = CanvasContent::new("fabricad.mask.viewport")
            .native_viewport()
            .interaction(CanvasInteractionPolicy::NATIVE_VIEWPORT);
        let mut paint = PaintList::default();
        paint.items.push(paint_item(
            7,
            UiRect::new(12.0, 16.0, 320.0, 180.0),
            PaintKind::Canvas(canvas),
        ));

        let request = RenderFrameRequest::new(
            RenderTarget::app_owned("main", UiSize::new(640.0, 480.0)),
            UiSize::new(640.0, 480.0),
            paint,
        );
        let plans = request.canvas_host_capture_plans();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].node, UiNodeId(7));
        assert_eq!(plans[0].key, "fabricad.mask.viewport");
        assert_eq!(plans[0].rect, UiRect::new(12.0, 16.0, 320.0, 180.0));
        assert!(plans[0].pointer_capture);
        assert!(plans[0].keyboard_capture);
        assert!(plans[0].wheel_capture);
        assert!(plans[0].pointer_lock);
        assert!(plans[0].domain_hit_testing);
        assert!(plans[0].requires_host_capture());
        assert_eq!(
            plans[0].cursor_confine_rect(),
            Some(LogicalRect::new(12.0, 16.0, 320.0, 180.0))
        );
        assert_eq!(
            request.canvas_platform_requests(),
            vec![
                PlatformRequest::Cursor(CursorRequest::Confine(LogicalRect::new(
                    12.0, 16.0, 320.0, 180.0
                ))),
                PlatformRequest::Cursor(CursorRequest::SetVisible(false)),
            ]
        );
        assert_eq!(
            plans[0].release_platform_requests(),
            vec![
                PlatformRequest::Cursor(CursorRequest::ReleaseConfine),
                PlatformRequest::Cursor(CursorRequest::SetVisible(true)),
            ]
        );
    }

    #[test]
    fn canvas_host_capture_plan_keeps_editor_capture_without_pointer_lock_requests() {
        let canvas = CanvasContent::new("orbifold.curve.editor")
            .interaction(CanvasInteractionPolicy::EDITOR);
        let mut paint = PaintList::default();
        paint.items.push(paint_item(
            11,
            UiRect::new(24.0, 32.0, 400.0, 240.0),
            PaintKind::Canvas(canvas),
        ));

        let request = RenderFrameRequest::new(
            RenderTarget::window("main", UiSize::new(800.0, 600.0)),
            UiSize::new(800.0, 600.0),
            paint,
        );
        let plans = request.canvas_host_capture_plans();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].node, UiNodeId(11));
        assert_eq!(plans[0].key, "orbifold.curve.editor");
        assert!(plans[0].pointer_capture);
        assert!(plans[0].keyboard_capture);
        assert!(plans[0].wheel_capture);
        assert!(!plans[0].pointer_lock);
        assert!(plans[0].domain_hit_testing);
        assert_eq!(plans[0].cursor_confine_rect(), None);
        assert!(plans[0].platform_requests().is_empty());
        assert!(plans[0].release_platform_requests().is_empty());
        assert!(request.canvas_platform_requests().is_empty());
    }

    #[derive(Debug, Default)]
    struct CanvasBackend {
        rendered: Vec<String>,
        scale_factors: Vec<f32>,
        focused: Vec<bool>,
        dirty: Vec<bool>,
    }

    #[derive(Debug)]
    struct RecordingCanvasHandler;

    impl CanvasRenderHandler<CanvasBackend> for RecordingCanvasHandler {
        fn render_canvas(
            &mut self,
            context: CanvasRenderContext<'_, CanvasBackend>,
        ) -> Result<CanvasRenderOutput, RenderError> {
            context
                .backend
                .rendered
                .push(context.request.canvas.key.clone());
            context.backend.scale_factors.push(context.scale_factor);
            context.backend.focused.push(context.interaction.focused);
            context.backend.dirty.push(context.is_dirty());
            Ok(CanvasRenderOutput::new()
                .dirty_region(context.request.rect)
                .repaint_requested(context.interaction.focused))
        }
    }

    #[test]
    fn canvas_render_registry_dispatches_requests_with_context() {
        let canvas = CanvasContent::new("fabricad.mask.viewport")
            .callback()
            .pointer_capture(true)
            .keyboard_capture(true);
        let mut paint = PaintList::default();
        paint.items.push(paint_item(
            7,
            UiRect::new(12.0, 16.0, 320.0, 180.0),
            PaintKind::Canvas(canvas),
        ));
        let request = RenderFrameRequest::new(
            RenderTarget::window("main", UiSize::new(640.0, 480.0)),
            UiSize::new(640.0, 480.0),
            paint,
        )
        .options(RenderOptions {
            scale_factor: 2.0,
            ..RenderOptions::default()
        })
        .node_interaction(
            UiNodeId(7),
            HostNodeInteraction {
                focused: true,
                ..HostNodeInteraction::default()
            },
        );
        let mut backend = CanvasBackend::default();
        let mut registry = CanvasRenderRegistry::new();
        assert!(!registry.register("fabricad.mask.viewport", RecordingCanvasHandler));

        let report = registry
            .render_frame_canvases_strict(&request, &mut backend)
            .expect("canvas dispatch");

        assert_eq!(report.rendered_count(), 1);
        assert_eq!(report.missing_count(), 0);
        assert_eq!(report.failed_count(), 0);
        assert!(report.repaint_requested());
        assert_eq!(backend.rendered, vec!["fabricad.mask.viewport".to_string()]);
        assert_eq!(backend.scale_factors, vec![2.0]);
        assert_eq!(backend.focused, vec![true]);
        assert_eq!(backend.dirty, vec![true]);
        assert_eq!(report.outcomes[0].request().node, UiNodeId(7));
        assert_eq!(report.resource_updates(), Vec::<ResourceUpdate>::new());
    }

    #[test]
    fn canvas_render_registry_reports_missing_handlers() {
        let mut paint = PaintList::default();
        paint.items.push(paint_item(
            4,
            UiRect::new(0.0, 0.0, 120.0, 80.0),
            PaintKind::Canvas(CanvasContent::new("missing.viewport").native_viewport()),
        ));
        let request = RenderFrameRequest::new(
            RenderTarget::app_owned("main", UiSize::new(640.0, 480.0)),
            UiSize::new(640.0, 480.0),
            paint,
        );
        let mut backend = CanvasBackend::default();
        let mut registry = CanvasRenderRegistry::new();

        let report = registry.render_frame_canvases(&request, &mut backend);
        assert_eq!(report.rendered_count(), 0);
        assert_eq!(report.missing_count(), 1);
        assert_eq!(
            report.first_missing().unwrap().canvas.key,
            "missing.viewport"
        );
        assert_eq!(
            report.into_strict_result().unwrap_err(),
            RenderError::MissingCanvasRenderer("missing.viewport".to_string())
        );
    }

    #[test]
    fn render_request_extracts_image_requests_for_nodes_and_scene_placements() {
        let mut paint = PaintList::default();
        paint.items.push(paint_item(
            2,
            UiRect::new(8.0, 10.0, 24.0, 18.0),
            PaintKind::Image {
                key: "icons.play".to_string(),
                tint: Some(ColorRgba::new(120, 180, 255, 255)),
            },
        ));
        paint.items.push(paint_item(
            3,
            UiRect::new(40.0, 12.0, 64.0, 48.0),
            PaintKind::ImagePlacement(
                PaintImage::new("covers.scale", UiRect::new(40.0, 12.0, 64.0, 48.0))
                    .fit(ImageFit::Contain)
                    .align(ImageAlignment::End, ImageAlignment::Start),
            ),
        ));
        let request = RenderFrameRequest::new(
            RenderTarget::window("main", UiSize::new(160.0, 90.0)),
            UiSize::new(160.0, 90.0),
            paint,
        );

        let images = request.image_requests();

        assert_eq!(images.len(), 2);
        assert_eq!(images[0].kind, ImageRenderKind::NodeImage);
        assert_eq!(images[0].key(), "icons.play");
        assert_eq!(images[0].image.rect, UiRect::new(8.0, 10.0, 24.0, 18.0));
        assert_eq!(
            images[0].image.tint,
            Some(ColorRgba::new(120, 180, 255, 255))
        );
        assert_eq!(images[1].kind, ImageRenderKind::ImagePlacement);
        assert_eq!(images[1].key(), "covers.scale");
        assert_eq!(images[1].image.fit, ImageFit::Contain);
        assert_eq!(images[1].image.horizontal_align, ImageAlignment::End);
    }

    #[derive(Debug, Default)]
    struct ImageBackend {
        rendered: Vec<String>,
        scale_factors: Vec<f32>,
        hovered: Vec<bool>,
        dirty: Vec<bool>,
    }

    #[derive(Debug)]
    struct RecordingImageHandler;

    impl ImageRenderHandler<ImageBackend> for RecordingImageHandler {
        fn render_image(
            &mut self,
            context: ImageRenderContext<'_, ImageBackend>,
        ) -> Result<ImageRenderOutput, RenderError> {
            context
                .backend
                .rendered
                .push(context.request.image.key.clone());
            context.backend.scale_factors.push(context.scale_factor);
            context.backend.hovered.push(context.interaction.hovered);
            context.backend.dirty.push(context.is_dirty());
            Ok(ImageRenderOutput::new()
                .dirty_region(context.request.image.rect)
                .repaint_requested(context.interaction.hovered))
        }
    }

    #[test]
    fn image_render_registry_dispatches_requests_with_context() {
        let mut paint = PaintList::default();
        paint.items.push(paint_item(
            5,
            UiRect::new(10.0, 12.0, 32.0, 32.0),
            PaintKind::Image {
                key: "icons.record".to_string(),
                tint: None,
            },
        ));
        let request = RenderFrameRequest::new(
            RenderTarget::window("main", UiSize::new(160.0, 90.0)),
            UiSize::new(160.0, 90.0),
            paint,
        )
        .options(RenderOptions {
            scale_factor: 1.5,
            ..RenderOptions::default()
        })
        .node_interaction(
            UiNodeId(5),
            HostNodeInteraction {
                hovered: true,
                ..HostNodeInteraction::default()
            },
        );
        let mut backend = ImageBackend::default();
        let mut registry = ImageRenderRegistry::new();
        assert!(!registry.register("icons.record", RecordingImageHandler));

        let report = registry
            .render_frame_images_strict(&request, &mut backend)
            .expect("image dispatch");

        assert_eq!(report.rendered_count(), 1);
        assert_eq!(report.missing_count(), 0);
        assert_eq!(report.failed_count(), 0);
        assert!(report.repaint_requested());
        assert_eq!(backend.rendered, vec!["icons.record".to_string()]);
        assert_eq!(backend.scale_factors, vec![1.5]);
        assert_eq!(backend.hovered, vec![true]);
        assert_eq!(backend.dirty, vec![true]);
        assert_eq!(report.outcomes[0].request().node, UiNodeId(5));
        assert_eq!(report.resource_updates(), Vec::<ResourceUpdate>::new());
    }

    #[test]
    fn image_render_registry_reports_missing_handlers() {
        let mut paint = PaintList::default();
        paint.items.push(paint_item(
            8,
            UiRect::new(0.0, 0.0, 120.0, 80.0),
            PaintKind::Image {
                key: "missing.image".to_string(),
                tint: None,
            },
        ));
        let request = RenderFrameRequest::new(
            RenderTarget::app_owned("main", UiSize::new(640.0, 480.0)),
            UiSize::new(640.0, 480.0),
            paint,
        );
        let mut backend = ImageBackend::default();
        let mut registry = ImageRenderRegistry::new();

        let report = registry.render_frame_images(&request, &mut backend);
        assert_eq!(report.rendered_count(), 0);
        assert_eq!(report.missing_count(), 1);
        assert_eq!(report.first_missing().unwrap().image.key, "missing.image");
        assert_eq!(
            report.into_strict_result().unwrap_err(),
            RenderError::MissingImageRenderer("missing.image".to_string())
        );
    }

    #[derive(Debug, Default)]
    struct TestResolver {
        descriptor: Option<ResourceDescriptor>,
    }

    impl ResourceResolver for TestResolver {
        fn resolve_resource(&self, id: &ResourceId) -> Option<ResourceDescriptor> {
            self.descriptor
                .clone()
                .filter(|descriptor| descriptor.handle.id() == id)
        }
    }

    #[derive(Debug)]
    struct RecordingRenderer {
        capabilities: BackendCapabilities,
        resolved: Vec<ResourceId>,
    }

    impl RendererAdapter for RecordingRenderer {
        fn capabilities(&self) -> BackendCapabilities {
            self.capabilities.clone()
        }

        fn render_frame(
            &mut self,
            request: RenderFrameRequest,
            resolver: &dyn ResourceResolver,
        ) -> Result<RenderFrameOutput, RenderError> {
            if matches!(request.target.kind(), RenderTargetKind::Snapshot)
                && !self.capabilities.rendering.deterministic_snapshots
            {
                return Err(RenderError::UnsupportedTarget(request.target.kind()));
            }

            for update in &request.resource_updates {
                if !self
                    .capabilities
                    .supports_resource(update.descriptor.handle.kind())
                {
                    return Err(RenderError::UnsupportedResource(
                        update.descriptor.handle.kind(),
                    ));
                }
                if !update.has_expected_byte_len() || !update.dirty_rect_is_valid() {
                    return Err(RenderError::InvalidResourceUpdate(
                        update.descriptor.handle.id().key.clone(),
                    ));
                }
                let id = update.descriptor.handle.id().clone();
                resolver
                    .resolve_resource(&id)
                    .ok_or_else(|| RenderError::MissingResource(id.clone()))?;
                self.resolved.push(id);
            }

            let batches = request.batches();
            let mut output = RenderFrameOutput::new(request.target);
            output.painted_items = request.paint.items.len();
            output.batches = batches;
            output.dirty_regions = request.dirty_regions;
            output.timings = FrameTiming::new().section("paint-build", Duration::from_millis(1));
            Ok(output)
        }
    }

    #[test]
    fn renderer_adapter_trait_receives_resources_batches_and_timings() {
        let handle = ResourceHandle::Image(ImageHandle::app("cover"));
        let descriptor =
            ResourceDescriptor::new(handle.clone(), PixelSize::new(2, 2), ResourceFormat::Rgba8);
        let update = ResourceUpdate::full(descriptor.clone(), vec![128; 2 * 2 * 4]);
        let resolver = TestResolver {
            descriptor: Some(descriptor),
        };
        let paint = PaintList {
            items: vec![paint_item(
                0,
                UiRect::new(0.0, 0.0, 16.0, 16.0),
                PaintKind::Image {
                    key: "cover".to_string(),
                    tint: None,
                },
            )],
        };
        let request = RenderFrameRequest::new(
            RenderTarget::snapshot(PixelSize::new(64, 64)),
            UiSize::new(64.0, 64.0),
            paint,
        )
        .resource_update(update)
        .options(RenderOptions {
            deterministic: true,
            ..RenderOptions::default()
        });
        let mut renderer = RecordingRenderer {
            capabilities: BackendCapabilities::new("recording")
                .adapter(BackendAdapterKind::Test)
                .resources(ResourceCapabilities {
                    images: true,
                    partial_texture_updates: true,
                    ..ResourceCapabilities::NONE
                })
                .rendering(RenderingCapabilities {
                    deterministic_snapshots: true,
                    offscreen: true,
                    partial_updates: true,
                    high_dpi: true,
                }),
            resolved: Vec::new(),
        };

        let output = renderer
            .render_frame(request, &resolver)
            .expect("render output");
        assert_eq!(output.painted_items, 1);
        assert_eq!(output.batches.len(), 1);
        assert_eq!(
            renderer.resolved,
            vec![ResourceId::new(ResourceDomain::App, "cover")]
        );
        assert_eq!(
            output.timings.duration("paint-build"),
            Some(Duration::from_millis(1))
        );
    }
}
