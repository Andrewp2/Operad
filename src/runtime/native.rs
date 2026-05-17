//! Native window runner for the default WGPU/winit path.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::host::process_host_frame_input_with_target_resolver;
use crate::input::{
    PointerButton, PointerButtons, PointerEventKind, RawInputEvent, RawKeyboardEvent,
    RawPointerEvent, RawTextInputEvent, RawWheelEvent,
};
use crate::platform::{
    CursorGrabMode, CursorRequest, CursorResponse, CursorShape, PixelSize, PlatformErrorCode,
    PlatformRequest, PlatformRequestIdAllocator, PlatformResponse, PlatformServiceRequest,
    PlatformServiceResponse, RepaintRequest, RepaintResponse,
};
use crate::{
    process_document_frame, AccessibilityRole, AnimationMachine, CanvasContent,
    CanvasHostCaptureId, CanvasRenderOutput, CanvasRenderReport, CanvasRenderRequest,
    CosmicTextMeasurer, DirtyRegionSet, EmptyResourceResolver, HostDocumentFrameState,
    HostFrameOutput, HostNodeInteraction, KeyCode, KeyModifiers, RenderError, RenderTarget,
    RendererAdapter, UiContent, UiDocument, UiFocusState, UiInputEvent, UiNodeId, UiPoint, UiRect,
    UiSize, WgpuCanvasContext, WgpuSurfaceRenderer, WheelDeltaUnit, WheelPhase, WidgetAction,
    WidgetActionBinding, WidgetActionQueue, WidgetValueEditPhase,
};

pub type NativeWindowResult<T = ()> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
struct NativeWindowRunError {
    title: String,
    phase: &'static str,
    message: String,
    app_error: Option<String>,
    last_frame: Option<NativeFrameTimingReport>,
}

impl NativeWindowRunError {
    fn new(
        title: impl Into<String>,
        phase: &'static str,
        message: impl Into<String>,
        app_error: Option<String>,
        last_frame: Option<NativeFrameTimingReport>,
    ) -> Self {
        Self {
            title: title.into(),
            phase,
            message: message.into(),
            app_error,
            last_frame,
        }
    }
}

impl fmt::Display for NativeWindowRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "native window {:?} failed while {}: {}",
            self.title, self.phase, self.message
        )?;
        if let Some(app_error) = &self.app_error {
            write!(f, "\nlast application error: {app_error}")?;
        }
        if let Some(last_frame) = self.last_frame {
            write!(f, "\nlast completed frame: {last_frame}")?;
        }
        Ok(())
    }
}

impl Error for NativeWindowRunError {}

#[derive(Debug, Clone, Copy)]
struct NativeFrameTimingReport {
    viewport: UiSize,
    nodes: usize,
    paint_items: usize,
    widget_actions: usize,
    build_document: Duration,
    host_input: Duration,
    document_frame: Duration,
    action_rebuild: Option<Duration>,
    canvas_render: Duration,
    surface_render: Duration,
    total: Duration,
}

impl fmt::Display for NativeFrameTimingReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "viewport={:.0}x{:.0}, nodes={}, paint_items={}, actions={}, build_document={:?}, host_input={:?}, document_frame={:?}",
            self.viewport.width,
            self.viewport.height,
            self.nodes,
            self.paint_items,
            self.widget_actions,
            self.build_document,
            self.host_input,
            self.document_frame,
        )?;
        if let Some(action_rebuild) = self.action_rebuild {
            write!(f, ", action_rebuild={action_rebuild:?}")?;
        }
        write!(
            f,
            ", canvas_render={:?}, surface_render={:?}, total={:?}",
            self.canvas_render, self.surface_render, self.total
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeWindowMetrics {
    pub physical_size: PixelSize,
    pub viewport: UiSize,
    pub scale_factor: f32,
    pub dpi_scale: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeKeyboardInput {
    pub logical_key: winit::keyboard::Key,
    pub physical_key: winit::keyboard::PhysicalKey,
    pub key_code: Option<KeyCode>,
    pub modifiers: winit::keyboard::ModifiersState,
    pub state: winit::event::ElementState,
    pub pressed: bool,
    pub repeat: bool,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NativeRawMouseMotion {
    pub device_id: winit::event::DeviceId,
    pub delta: (f64, f64),
    pub timestamp_millis: u64,
    pub captured_canvas: Option<CanvasHostCaptureId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeCanvasInput {
    pub node: UiNodeId,
    pub key: String,
    pub rect: UiRect,
    pub local_position: Option<UiPoint>,
    pub input: RawInputEvent,
}

type InitialSizeHook = Box<dyn Fn(&winit::event_loop::ActiveEventLoop) -> UiSize>;
type TitleHook<State> = Box<dyn Fn(&State) -> String>;
type ScaleFactorHook<State> = Box<dyn Fn(&State, NativeWindowMetrics) -> f32>;
type CloseHook<State> = Box<dyn FnMut(&mut State) -> bool>;
type KeyboardHook<State> = Box<dyn FnMut(&mut State, NativeKeyboardInput) -> bool>;
type RawMouseMotionHook<State> = Box<dyn FnMut(&mut State, NativeRawMouseMotion) -> bool>;
type CanvasInputHook<State> = Box<dyn FnMut(&mut State, NativeCanvasInput) -> bool>;
type PlatformRequestsHook<State> =
    Box<dyn FnMut(&mut State, NativeWindowMetrics) -> Vec<PlatformRequest>>;
type BeforeRenderHook<State> = Box<dyn FnMut(&mut State, NativeWindowMetrics)>;
type IdleRedrawHook<State> = Box<dyn Fn(&State) -> bool>;

pub struct NativeWindowHooks<State> {
    pub initial_size: Option<InitialSizeHook>,
    pub title: Option<TitleHook<State>>,
    pub scale_factor: Option<ScaleFactorHook<State>>,
    pub close_requested: Option<CloseHook<State>>,
    pub keyboard_input: Option<KeyboardHook<State>>,
    pub raw_mouse_motion: Option<RawMouseMotionHook<State>>,
    pub canvas_input: Option<CanvasInputHook<State>>,
    pub platform_requests: Option<PlatformRequestsHook<State>>,
    pub before_render: Option<BeforeRenderHook<State>>,
    pub idle_redraw: Option<IdleRedrawHook<State>>,
}

impl<State> NativeWindowHooks<State> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_initial_size(
        mut self,
        initial_size: impl Fn(&winit::event_loop::ActiveEventLoop) -> UiSize + 'static,
    ) -> Self {
        self.initial_size = Some(Box::new(initial_size));
        self
    }

    pub fn with_title(mut self, title: impl Fn(&State) -> String + 'static) -> Self {
        self.title = Some(Box::new(title));
        self
    }

    pub fn with_scale_factor(
        mut self,
        scale_factor: impl Fn(&State, NativeWindowMetrics) -> f32 + 'static,
    ) -> Self {
        self.scale_factor = Some(Box::new(scale_factor));
        self
    }

    pub fn with_close_requested(
        mut self,
        close_requested: impl FnMut(&mut State) -> bool + 'static,
    ) -> Self {
        self.close_requested = Some(Box::new(close_requested));
        self
    }

    pub fn with_keyboard_input(
        mut self,
        keyboard_input: impl FnMut(&mut State, NativeKeyboardInput) -> bool + 'static,
    ) -> Self {
        self.keyboard_input = Some(Box::new(keyboard_input));
        self
    }

    pub fn with_raw_mouse_motion(
        mut self,
        raw_mouse_motion: impl FnMut(&mut State, NativeRawMouseMotion) -> bool + 'static,
    ) -> Self {
        self.raw_mouse_motion = Some(Box::new(raw_mouse_motion));
        self
    }

    pub fn with_canvas_input(
        mut self,
        canvas_input: impl FnMut(&mut State, NativeCanvasInput) -> bool + 'static,
    ) -> Self {
        self.canvas_input = Some(Box::new(canvas_input));
        self
    }

    pub fn with_platform_requests(
        mut self,
        platform_requests: impl FnMut(&mut State, NativeWindowMetrics) -> Vec<PlatformRequest> + 'static,
    ) -> Self {
        self.platform_requests = Some(Box::new(platform_requests));
        self
    }

    pub fn with_before_render(
        mut self,
        before_render: impl FnMut(&mut State, NativeWindowMetrics) + 'static,
    ) -> Self {
        self.before_render = Some(Box::new(before_render));
        self
    }

    pub fn with_idle_redraw(mut self, idle_redraw: impl Fn(&State) -> bool + 'static) -> Self {
        self.idle_redraw = Some(Box::new(idle_redraw));
        self
    }
}

impl<State> Default for NativeWindowHooks<State> {
    fn default() -> Self {
        Self {
            initial_size: None,
            title: None,
            scale_factor: None,
            close_requested: None,
            keyboard_input: None,
            raw_mouse_motion: None,
            canvas_input: None,
            platform_requests: None,
            before_render: None,
            idle_redraw: None,
        }
    }
}

#[derive(Debug)]
pub struct NativeWgpuCanvasRenderContext<'a> {
    pub request: &'a CanvasRenderRequest,
    pub scale_factor: f32,
    pub dirty_regions: &'a DirtyRegionSet,
    pub interaction: HostNodeInteraction,
    pub surface: WgpuCanvasContext<'a>,
}

impl NativeWgpuCanvasRenderContext<'_> {
    pub fn is_dirty(&self) -> bool {
        self.dirty_regions.is_empty() || self.dirty_regions.covers(self.request.rect)
    }

    pub fn surface_size(&self) -> crate::platform::PixelSize {
        self.surface.size()
    }
}

pub trait NativeWgpuCanvasRenderHandler<State> {
    fn render_canvas(
        &mut self,
        state: &mut State,
        context: NativeWgpuCanvasRenderContext<'_>,
    ) -> Result<CanvasRenderOutput, RenderError>;
}

impl<State, F> NativeWgpuCanvasRenderHandler<State> for F
where
    F: for<'a> FnMut(
        &mut State,
        NativeWgpuCanvasRenderContext<'a>,
    ) -> Result<CanvasRenderOutput, RenderError>,
{
    fn render_canvas(
        &mut self,
        state: &mut State,
        context: NativeWgpuCanvasRenderContext<'_>,
    ) -> Result<CanvasRenderOutput, RenderError> {
        self(state, context)
    }
}

pub struct NativeWgpuCanvasRenderRegistry<State> {
    handlers: HashMap<String, Box<dyn NativeWgpuCanvasRenderHandler<State>>>,
}

impl<State> NativeWgpuCanvasRenderRegistry<State> {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        key: impl Into<String>,
        handler: impl NativeWgpuCanvasRenderHandler<State> + 'static,
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

    fn render_frame_canvases(
        &mut self,
        state: &mut State,
        renderer: &mut WgpuSurfaceRenderer<'static>,
        request: &crate::RenderFrameRequest,
    ) -> CanvasRenderReport {
        let mut report = CanvasRenderReport::default();
        for canvas_request in request.canvas_requests() {
            let Some(handler) = self.handlers.get_mut(&canvas_request.canvas.key) else {
                continue;
            };
            let Some(size) = canvas_pixel_size(
                canvas_request.rect.width,
                canvas_request.rect.height,
                request.options.scale_factor
                    * normalized_native_scale(canvas_request.transform.scale),
            ) else {
                report.outcomes.push(crate::CanvasRenderOutcome::Failed {
                    request: canvas_request,
                    error: RenderError::Backend(
                        "canvas surface must have a positive finite size".to_string(),
                    ),
                });
                continue;
            };
            let outcome = match renderer.get_gpu_context(&canvas_request.canvas, size) {
                Ok(surface) => handler.render_canvas(
                    state,
                    NativeWgpuCanvasRenderContext {
                        request: &canvas_request,
                        scale_factor: request.options.scale_factor,
                        dirty_regions: &request.dirty_regions,
                        interaction: request.interaction_for(canvas_request.node),
                        surface,
                    },
                ),
                Err(error) => Err(error),
            };
            match outcome {
                Ok(output) => report.outcomes.push(crate::CanvasRenderOutcome::Rendered {
                    request: canvas_request,
                    output,
                }),
                Err(error) => report.outcomes.push(crate::CanvasRenderOutcome::Failed {
                    request: canvas_request,
                    error,
                }),
            }
        }
        report
    }
}

impl<State> Default for NativeWgpuCanvasRenderRegistry<State> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct NativeWindowOptions {
    pub title: String,
    pub size: UiSize,
    pub min_size: Option<UiSize>,
    pub ui_scale: f32,
    pub tick_action: Option<WidgetActionBinding>,
    pub tick_interval: Duration,
}

impl NativeWindowOptions {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = UiSize::new(width, height);
        self
    }

    pub fn with_min_size(mut self, width: f32, height: f32) -> Self {
        self.min_size = Some(UiSize::new(width, height));
        self
    }

    pub fn with_ui_scale(mut self, ui_scale: f32) -> Self {
        self.ui_scale = if ui_scale.is_finite() && ui_scale > 0.0 {
            ui_scale
        } else {
            1.0
        };
        self
    }

    pub fn with_tick_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.tick_action = Some(action.into());
        self
    }

    pub fn with_tick_rate_hz(mut self, rate_hz: f32) -> Self {
        let seconds = if rate_hz.is_finite() && rate_hz > 0.0 {
            (1.0 / rate_hz).max(0.001)
        } else {
            1.0 / 60.0
        };
        self.tick_interval = Duration::from_secs_f32(seconds);
        self
    }
}

impl Default for NativeWindowOptions {
    fn default() -> Self {
        Self {
            title: "operad".to_string(),
            size: UiSize::new(1024.0, 720.0),
            min_size: Some(UiSize::new(480.0, 320.0)),
            ui_scale: 1.0,
            tick_action: None,
            tick_interval: Duration::from_millis(16),
        }
    }
}

pub fn run_ui_document(
    title: impl Into<String>,
    view: impl FnMut(UiSize) -> UiDocument + 'static,
) -> NativeWindowResult {
    run_ui_document_with(NativeWindowOptions::new(title), view)
}

pub fn run_ui_document_with(
    options: NativeWindowOptions,
    mut view: impl FnMut(UiSize) -> UiDocument + 'static,
) -> NativeWindowResult {
    run_app_with(
        options,
        (),
        |_state: &mut (), _action: WidgetAction| {},
        move |_state: &(), viewport| view(viewport),
    )
}

pub fn run_ui_document_with_canvas_renderers(
    options: NativeWindowOptions,
    mut view: impl FnMut(UiSize) -> UiDocument + 'static,
    canvas_renderers: NativeWgpuCanvasRenderRegistry<()>,
) -> NativeWindowResult {
    run_app_with_canvas_renderers(
        options,
        (),
        |_state: &mut (), _action: WidgetAction| {},
        move |_state: &(), viewport| view(viewport),
        canvas_renderers,
    )
}

pub fn run_app<State>(
    title: impl Into<String>,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize) -> UiDocument + 'static,
) -> NativeWindowResult
where
    State: 'static,
{
    run_app_with(NativeWindowOptions::new(title), state, update, view)
}

pub fn run_app_with<State>(
    options: NativeWindowOptions,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize) -> UiDocument + 'static,
) -> NativeWindowResult
where
    State: 'static,
{
    run_app_with_canvas_renderers(
        options,
        state,
        update,
        view,
        NativeWgpuCanvasRenderRegistry::new(),
    )
}

pub fn run_app_with_canvas_renderers<State>(
    options: NativeWindowOptions,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize) -> UiDocument + 'static,
    canvas_renderers: NativeWgpuCanvasRenderRegistry<State>,
) -> NativeWindowResult
where
    State: 'static,
{
    run_app_with_canvas_renderers_and_hooks(
        options,
        state,
        update,
        view,
        canvas_renderers,
        NativeWindowHooks::default(),
    )
}

pub fn run_app_with_canvas_renderers_and_hooks<State>(
    options: NativeWindowOptions,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize) -> UiDocument + 'static,
    canvas_renderers: NativeWgpuCanvasRenderRegistry<State>,
    hooks: NativeWindowHooks<State>,
) -> NativeWindowResult
where
    State: 'static,
{
    let title = options.title.clone();
    let event_loop = winit::event_loop::EventLoop::new().map_err(|error| {
        NativeWindowRunError::new(
            title.clone(),
            "creating the platform event loop",
            error.to_string(),
            None,
            None,
        )
    })?;
    let mut app = NativeWindowApp::new(options, state, update, view, canvas_renderers, hooks);
    if let Err(error) = event_loop.run_app(&mut app) {
        return Err(NativeWindowRunError::new(
            app.options.title.clone(),
            "running the platform event loop",
            error.to_string(),
            app.error.take(),
            app.last_frame_report,
        )
        .into());
    }
    if let Some(error) = app.error {
        Err(NativeWindowRunError::new(
            app.options.title.clone(),
            "rendering a frame",
            error,
            None,
            app.last_frame_report,
        )
        .into())
    } else {
        Ok(())
    }
}

struct NativeWindowApp<State, Update, View> {
    options: NativeWindowOptions,
    state: State,
    update: Update,
    view: View,
    window: Option<Arc<winit::window::Window>>,
    window_id: Option<winit::window::WindowId>,
    renderer: Option<WgpuSurfaceRenderer<'static>>,
    canvas_renderers: NativeWgpuCanvasRenderRegistry<State>,
    hooks: NativeWindowHooks<State>,
    frame_state: HostDocumentFrameState,
    platform_request_ids: PlatformRequestIdAllocator,
    pending_platform_responses: Vec<PlatformServiceResponse>,
    scroll_offsets: HashMap<String, UiPoint>,
    animation_states: HashMap<String, AnimationMachine>,
    text_measurer: CosmicTextMeasurer,
    pending_input: Vec<RawInputEvent>,
    cursor: Option<UiPoint>,
    modifiers: KeyModifiers,
    buttons: PointerButtons,
    scheduled_redraw_at: Option<Instant>,
    continuous_redraw: bool,
    start: Instant,
    last_tick: Instant,
    last_animation_tick: Instant,
    last_frame_report: Option<NativeFrameTimingReport>,
    error: Option<String>,
}

impl<State, Update, View> NativeWindowApp<State, Update, View> {
    fn new(
        options: NativeWindowOptions,
        state: State,
        update: Update,
        view: View,
        canvas_renderers: NativeWgpuCanvasRenderRegistry<State>,
        hooks: NativeWindowHooks<State>,
    ) -> Self {
        Self {
            options,
            state,
            update,
            view,
            window: None,
            window_id: None,
            renderer: None,
            canvas_renderers,
            hooks,
            frame_state: HostDocumentFrameState::new(),
            platform_request_ids: PlatformRequestIdAllocator::default(),
            pending_platform_responses: Vec::new(),
            scroll_offsets: HashMap::new(),
            animation_states: HashMap::new(),
            text_measurer: CosmicTextMeasurer::new(),
            pending_input: Vec::new(),
            cursor: None,
            modifiers: KeyModifiers::NONE,
            buttons: PointerButtons::NONE,
            scheduled_redraw_at: None,
            continuous_redraw: false,
            start: Instant::now(),
            last_tick: Instant::now(),
            last_animation_tick: Instant::now(),
            last_frame_report: None,
            error: None,
        }
    }

    fn init_window(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> NativeWindowResult {
        let mut attributes = winit::window::Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_visible(true);
        let initial_size = self
            .hooks
            .initial_size
            .as_ref()
            .map(|initial_size| initial_size(event_loop))
            .unwrap_or(self.options.size);
        attributes = attributes.with_inner_size(logical_size(initial_size));
        if let Some(min_size) = self.options.min_size {
            attributes = attributes.with_min_inner_size(logical_size(min_size));
        }
        let window = Arc::new(event_loop.create_window(attributes)?);
        let size = nonzero_window_size(window.inner_size());

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
        }))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("native-window-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            }))?;
        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or("adapter does not support the native window surface")?;

        self.window_id = Some(window.id());
        self.renderer = Some(WgpuSurfaceRenderer::new(
            surface,
            device,
            queue,
            surface_config,
        )?);
        self.window = Some(window);
        Ok(())
    }

    fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn viewport(&self) -> Option<UiSize> {
        let size = self.window.as_ref()?.inner_size();
        if size.width == 0 || size.height == 0 {
            None
        } else {
            let scale = self.scale_factor_for_size(size);
            Some(UiSize::new(
                size.width as f32 / scale,
                size.height as f32 / scale,
            ))
        }
    }

    fn dpi_scale(&self) -> f32 {
        self.window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .filter(|scale| scale.is_finite() && *scale > 0.0)
            .unwrap_or(1.0)
    }

    fn scale_factor(&self) -> f32 {
        self.window
            .as_ref()
            .map(|window| self.scale_factor_for_size(window.inner_size()))
            .unwrap_or(1.0)
    }

    fn scale_factor_for_size(&self, size: winit::dpi::PhysicalSize<u32>) -> f32 {
        let dpi_scale = self.dpi_scale();
        let dpi_viewport = UiSize::new(
            size.width.max(1) as f32 / dpi_scale,
            size.height.max(1) as f32 / dpi_scale,
        );
        let metrics = NativeWindowMetrics {
            physical_size: PixelSize::new(size.width.max(1), size.height.max(1)),
            viewport: dpi_viewport,
            scale_factor: dpi_scale,
            dpi_scale,
        };
        self.hooks
            .scale_factor
            .as_ref()
            .map(|scale_factor| normalized_native_scale(scale_factor(&self.state, metrics)))
            .unwrap_or(dpi_scale)
    }

    fn metrics_for_viewport(&self, viewport: UiSize) -> NativeWindowMetrics {
        let size = self
            .window
            .as_ref()
            .map(|window| window.inner_size())
            .unwrap_or_else(|| winit::dpi::PhysicalSize::new(1, 1));
        NativeWindowMetrics {
            physical_size: PixelSize::new(size.width.max(1), size.height.max(1)),
            viewport,
            scale_factor: self.scale_factor_for_size(size),
            dpi_scale: self.dpi_scale(),
        }
    }

    fn timestamp_millis(&self) -> u64 {
        self.start
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }

    fn push_input(&mut self, event: RawInputEvent) {
        self.pending_input.push(event);
        self.request_redraw();
    }

    fn dispatch_canvas_input_hooks(
        &mut self,
        document: &UiDocument,
        raw_input: Vec<RawInputEvent>,
    ) -> Vec<RawInputEvent> {
        if self.hooks.canvas_input.is_none() {
            return raw_input;
        }

        let mut remaining = Vec::with_capacity(raw_input.len());
        for event in raw_input {
            let handled =
                native_canvas_input_for_raw_event(document, &self.frame_state.interaction, &event)
                    .is_some_and(|canvas_input| {
                        self.hooks
                            .canvas_input
                            .as_mut()
                            .is_some_and(|hook| hook(&mut self.state, canvas_input))
                    });
            if handled {
                self.request_redraw();
            } else {
                remaining.push(event);
            }
        }
        remaining
    }

    fn apply_platform_service_requests(&mut self, frame: &crate::HostDocumentFrameOutput) {
        let requests = frame.platform_service_requests(&mut self.platform_request_ids);
        if requests.is_empty() {
            return;
        }
        let responses = requests
            .into_iter()
            .map(|request| self.apply_platform_service_request(request))
            .collect::<Vec<_>>();
        self.pending_platform_responses.extend(responses);
    }

    fn apply_platform_service_request(
        &mut self,
        request: PlatformServiceRequest,
    ) -> PlatformServiceResponse {
        let PlatformServiceRequest { id, request } = request;
        let response = self.apply_platform_request(request);
        PlatformServiceResponse::new(id, response)
    }

    fn apply_platform_request(&mut self, request: PlatformRequest) -> PlatformResponse {
        match request {
            PlatformRequest::Cursor(request) => {
                PlatformResponse::Cursor(self.apply_cursor_request(request))
            }
            PlatformRequest::Repaint(request) => {
                PlatformResponse::Repaint(self.apply_repaint_request(request))
            }
            request => PlatformResponse::unsupported(request.kind()),
        }
    }

    fn apply_hook_platform_requests(&mut self, metrics: NativeWindowMetrics) {
        let Some(platform_requests) = self.hooks.platform_requests.as_mut() else {
            return;
        };
        let requests = platform_requests(&mut self.state, metrics);
        for request in requests {
            self.apply_platform_request(request);
        }
    }

    fn apply_cursor_request(&self, request: CursorRequest) -> CursorResponse {
        let Some(window) = self.window.as_ref() else {
            return CursorResponse::Unsupported;
        };
        match request {
            CursorRequest::SetShape(shape) => {
                window.set_cursor(native_cursor_icon(shape));
                CursorResponse::Applied
            }
            CursorRequest::SetVisible(visible) => {
                window.set_cursor_visible(visible);
                CursorResponse::Applied
            }
            CursorRequest::SetPosition(point) => window
                .set_cursor_position(winit::dpi::LogicalPosition::new(
                    point.x as f64,
                    point.y as f64,
                ))
                .map(|_| CursorResponse::Applied)
                .unwrap_or_else(cursor_error),
            CursorRequest::SetGrab(mode) => {
                set_cursor_grab(window, mode).unwrap_or_else(cursor_error)
            }
            CursorRequest::Confine(_) => window
                .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                .or_else(|_| window.set_cursor_grab(winit::window::CursorGrabMode::Locked))
                .map(|_| CursorResponse::Applied)
                .unwrap_or_else(cursor_error),
            CursorRequest::ReleaseConfine => window
                .set_cursor_grab(winit::window::CursorGrabMode::None)
                .map(|_| CursorResponse::Applied)
                .unwrap_or_else(cursor_error),
        }
    }

    fn apply_repaint_request(&mut self, request: RepaintRequest) -> RepaintResponse {
        match request {
            RepaintRequest::NextFrame | RepaintRequest::Area(_) => {
                self.request_redraw();
                RepaintResponse::Scheduled {
                    delay: Duration::ZERO,
                }
            }
            RepaintRequest::After(delay) => {
                let wake_at = Instant::now() + delay;
                self.scheduled_redraw_at = Some(
                    self.scheduled_redraw_at
                        .map(|scheduled| scheduled.min(wake_at))
                        .unwrap_or(wake_at),
                );
                RepaintResponse::Scheduled { delay }
            }
            RepaintRequest::Continuous { active } => {
                self.continuous_redraw = active;
                if active {
                    self.request_redraw();
                    RepaintResponse::Scheduled {
                        delay: Duration::ZERO,
                    }
                } else {
                    RepaintResponse::Coalesced
                }
            }
        }
    }

    fn render(&mut self) -> NativeWindowResult
    where
        Update: FnMut(&mut State, WidgetAction),
        View: FnMut(&State, UiSize) -> UiDocument,
    {
        let Some(viewport) = self.viewport() else {
            return Ok(());
        };
        let frame_started = Instant::now();
        let metrics = self.metrics_for_viewport(viewport);
        if let Some(before_render) = self.hooks.before_render.as_mut() {
            before_render(&mut self.state, metrics);
        }
        if let (Some(window), Some(title)) = (self.window.as_ref(), self.hooks.title.as_ref()) {
            window.set_title(&title(&self.state));
        }
        self.apply_hook_platform_requests(metrics);
        self.dispatch_tick_if_due();
        let raw_input = std::mem::take(&mut self.pending_input);
        let animation_dt = self.animation_delta_seconds();

        let build_started = Instant::now();
        let mut document = self.build_document(viewport)?;
        let mut build_document = build_started.elapsed();
        let mut nodes = document.node_count();
        document.tick_animations(animation_dt);
        let raw_input = self.dispatch_canvas_input_hooks(&document, raw_input);
        let mut host_request = self.frame_state.host_frame_request(viewport);
        host_request.raw_input = raw_input;
        host_request.platform_responses = std::mem::take(&mut self.pending_platform_responses);
        let host_input_started = Instant::now();
        let host_output =
            process_host_frame_input_with_target_resolver(host_request, |event, state| {
                resolve_target(event, state, &document)
            });
        let host_input = host_input_started.elapsed();
        let frame_request = self.frame_state.document_frame_request(
            viewport,
            RenderTarget::window(self.options.title.clone(), viewport),
            host_output,
        );
        let document_frame_started = Instant::now();
        let frame = process_document_frame(&mut document, &mut self.text_measurer, frame_request)?;
        let mut document_frame = document_frame_started.elapsed();
        self.capture_document_runtime_state(&document);
        let actions = collect_widget_actions(&document, &frame);
        let actions_count = actions.len();
        self.frame_state.apply_document_frame_output(&frame);
        self.apply_platform_service_requests(&frame);

        let mut action_rebuild = None;
        let frame = if actions.is_empty() {
            frame
        } else {
            let action_started = Instant::now();
            for action in actions {
                (self.update)(&mut self.state, action);
            }
            let rebuild_started = Instant::now();
            let mut document = self.build_document(viewport)?;
            build_document += rebuild_started.elapsed();
            nodes = document.node_count();
            let frame_request = self.frame_state.document_frame_request(
                viewport,
                RenderTarget::window(self.options.title.clone(), viewport),
                HostFrameOutput::new(self.frame_state.interaction.clone()),
            );
            let document_frame_started = Instant::now();
            let frame =
                process_document_frame(&mut document, &mut self.text_measurer, frame_request)?;
            document_frame += document_frame_started.elapsed();
            self.capture_document_runtime_state(&document);
            self.frame_state.apply_document_frame_output(&frame);
            self.apply_platform_service_requests(&frame);
            action_rebuild = Some(action_started.elapsed());
            frame
        };

        let Some(renderer) = self.renderer.as_mut() else {
            self.last_frame_report = Some(NativeFrameTimingReport {
                viewport,
                nodes,
                paint_items: frame.render_request.paint.items.len(),
                widget_actions: actions_count,
                build_document,
                host_input,
                document_frame,
                action_rebuild,
                canvas_render: Duration::ZERO,
                surface_render: Duration::ZERO,
                total: frame_started.elapsed(),
            });
            return Ok(());
        };
        let paint_items = frame.render_request.paint.items.len();
        let canvas_started = Instant::now();
        let canvas_report = self.canvas_renderers.render_frame_canvases(
            &mut self.state,
            renderer,
            &frame.render_request,
        );
        let canvas_render = canvas_started.elapsed();
        if let Some(error) = canvas_report.first_failure().cloned() {
            return Err(error.into());
        }
        let repaint_requested = canvas_report.repaint_requested();
        let surface_started = Instant::now();
        renderer.render_frame(frame.render_request, &EmptyResourceResolver)?;
        let surface_render = surface_started.elapsed();
        self.last_frame_report = Some(NativeFrameTimingReport {
            viewport,
            nodes,
            paint_items,
            widget_actions: actions_count,
            build_document,
            host_input,
            document_frame,
            action_rebuild,
            canvas_render,
            surface_render,
            total: frame_started.elapsed(),
        });
        if repaint_requested {
            self.request_redraw();
        }
        Ok(())
    }

    fn dispatch_tick_if_due(&mut self)
    where
        Update: FnMut(&mut State, WidgetAction),
    {
        let Some(action) = self.options.tick_action.clone() else {
            return;
        };
        let now = Instant::now();
        if now.duration_since(self.last_tick) < self.options.tick_interval {
            return;
        }
        self.last_tick = now;
        (self.update)(&mut self.state, WidgetAction::activate(UiNodeId(0), action));
    }

    fn build_document(&mut self, viewport: UiSize) -> Result<UiDocument, taffy::TaffyError>
    where
        View: FnMut(&State, UiSize) -> UiDocument,
    {
        let mut document = (self.view)(&self.state, viewport);
        document.set_ui_scale(self.options.ui_scale);
        document.set_dpi_scale(self.scale_factor());
        restore_scroll_offsets(&mut document, &self.scroll_offsets);
        self.restore_animation_states(&mut document);
        let mut focus = UiFocusState {
            hovered: self.frame_state.interaction.hovered,
            pressed: self.frame_state.interaction.pressed,
            focused: self.frame_state.interaction.focused,
        };
        if let Some(cursor) = self.cursor {
            focus.hovered = document.hit_test(cursor);
            if self.buttons == PointerButtons::NONE {
                focus.pressed = None;
            }
        }
        document.set_focus_state(focus);
        document.compute_layout(viewport, &mut self.text_measurer)?;
        if let Some(cursor) = self.cursor {
            let mut focus = document.focus.clone();
            focus.hovered = document.hit_test(cursor);
            if self.buttons == PointerButtons::NONE {
                focus.pressed = None;
            }
            document.set_focus_state(focus);
        }
        if document.clamp_scroll_offsets() {
            document.compute_layout(viewport, &mut self.text_measurer)?;
            if let Some(cursor) = self.cursor {
                let mut focus = document.focus.clone();
                focus.hovered = document.hit_test(cursor);
                if self.buttons == PointerButtons::NONE {
                    focus.pressed = None;
                }
                document.set_focus_state(focus);
            }
        }
        Ok(document)
    }

    fn animation_delta_seconds(&mut self) -> f32 {
        let now = Instant::now();
        let dt = now
            .checked_duration_since(self.last_animation_tick)
            .unwrap_or(Duration::ZERO);
        self.last_animation_tick = now;
        dt.as_secs_f32().clamp(0.0, 0.1)
    }

    fn capture_document_runtime_state(&mut self, document: &UiDocument) {
        self.capture_scroll_offsets(document);
        self.capture_animation_states(document);
        if document.animations_active() {
            self.request_redraw();
        }
    }

    fn capture_scroll_offsets(&mut self, document: &UiDocument) {
        self.scroll_offsets.clear();
        for index in 0..document.node_count() {
            let id = UiNodeId(index);
            let Some(scroll) = document.scroll_state(id) else {
                continue;
            };
            if scroll_offset_is_zero(scroll.offset) {
                continue;
            }
            self.scroll_offsets
                .insert(node_path_key(document, id), scroll.offset);
        }
    }

    fn restore_animation_states(&self, document: &mut UiDocument) -> bool {
        if self.animation_states.is_empty() {
            return false;
        }
        let mut restored = Vec::new();
        for index in 0..document.node_count() {
            let id = UiNodeId(index);
            let Some(animation) = document.node(id).animation.as_ref() else {
                continue;
            };
            let Some(stored) = self.animation_states.get(&node_path_key(document, id)) else {
                continue;
            };
            if animation.has_same_definition(stored) {
                let mut restored_animation = animation.clone();
                restored_animation.retain_runtime_from(stored);
                restored.push((id, restored_animation));
            }
        }

        let changed = !restored.is_empty();
        for (id, animation) in restored {
            document.node_mut(id).animation = Some(animation);
        }
        changed
    }

    fn capture_animation_states(&mut self, document: &UiDocument) {
        self.animation_states.clear();
        for index in 0..document.node_count() {
            let id = UiNodeId(index);
            let Some(animation) = document.node(id).animation.as_ref() else {
                continue;
            };
            self.animation_states
                .insert(node_path_key(document, id), animation.clone());
        }
    }

    fn fail_and_exit(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        error: impl ToString,
    ) {
        self.error = Some(error.to_string());
        event_loop.exit();
    }
}

impl<State, Update, View> winit::application::ApplicationHandler
    for NativeWindowApp<State, Update, View>
where
    State: 'static,
    Update: FnMut(&mut State, WidgetAction) + 'static,
    View: FnMut(&State, UiSize) -> UiDocument + 'static,
{
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        if let Err(error) = self.init_window(event_loop) {
            self.fail_and_exit(event_loop, error);
            return;
        }
        self.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }

        match event {
            winit::event::WindowEvent::CloseRequested => {
                let should_exit = self
                    .hooks
                    .close_requested
                    .as_mut()
                    .map(|close_requested| close_requested(&mut self.state))
                    .unwrap_or(true);
                if should_exit {
                    event_loop.exit();
                } else {
                    self.request_redraw();
                }
            }
            winit::event::WindowEvent::Destroyed => {
                event_loop.exit();
            }
            winit::event::WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.request_redraw();
                }
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                let scale = self.scale_factor();
                let point = UiPoint::new(position.x as f32 / scale, position.y as f32 / scale);
                self.cursor = Some(point);
                self.push_input(RawInputEvent::Pointer(
                    RawPointerEvent::new(PointerEventKind::Move, point, self.timestamp_millis())
                        .buttons(self.buttons)
                        .modifiers(self.modifiers),
                ));
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                let Some(point) = self.cursor else {
                    return;
                };
                let Some(button) = pointer_button(button) else {
                    return;
                };
                let kind = match state {
                    winit::event::ElementState::Pressed => {
                        self.buttons = self.buttons.with(button);
                        PointerEventKind::Down(button)
                    }
                    winit::event::ElementState::Released => {
                        self.buttons = self.buttons.without(button);
                        PointerEventKind::Up(button)
                    }
                };
                self.push_input(RawInputEvent::Pointer(
                    RawPointerEvent::new(kind, point, self.timestamp_millis())
                        .buttons(self.buttons)
                        .modifiers(self.modifiers),
                ));
            }
            winit::event::WindowEvent::MouseWheel { delta, phase, .. } => {
                let position = self.cursor.unwrap_or(UiPoint::new(0.0, 0.0));
                let (delta, unit) = wheel_delta(delta);
                let delta = if unit == WheelDeltaUnit::Pixel {
                    let scale = self.scale_factor();
                    UiPoint::new(delta.x / scale, delta.y / scale)
                } else {
                    delta
                };
                self.push_input(RawInputEvent::Wheel(RawWheelEvent {
                    position,
                    delta,
                    unit,
                    phase: wheel_phase(phase),
                    modifiers: self.modifiers,
                    timestamp_millis: self.timestamp_millis(),
                }));
            }
            winit::event::WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = key_modifiers(modifiers.state());
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                let key = key_code(&event, self.modifiers);
                let handled = self
                    .hooks
                    .keyboard_input
                    .as_mut()
                    .is_some_and(|keyboard_input| {
                        keyboard_input(
                            &mut self.state,
                            NativeKeyboardInput {
                                logical_key: event.logical_key.clone(),
                                physical_key: event.physical_key,
                                key_code: key,
                                modifiers: winit_modifiers(self.modifiers),
                                state: event.state,
                                pressed: matches!(event.state, winit::event::ElementState::Pressed),
                                repeat: event.repeat,
                                text: event.text.as_ref().map(|text| text.to_string()),
                            },
                        )
                    });
                if handled {
                    self.request_redraw();
                    return;
                }
                if let Some(key) = key {
                    let event = match event.state {
                        winit::event::ElementState::Pressed => {
                            RawKeyboardEvent::press(key, self.modifiers, self.timestamp_millis())
                                .repeat(event.repeat)
                        }
                        winit::event::ElementState::Released => {
                            RawKeyboardEvent::release(key, self.modifiers, self.timestamp_millis())
                        }
                    };
                    self.push_input(RawInputEvent::Keyboard(event));
                }
                if event.state == winit::event::ElementState::Pressed
                    && !self.modifiers.ctrl
                    && !self.modifiers.meta
                {
                    if let Some(text) = event.text.as_ref().filter(|text| !text.is_empty()) {
                        self.push_input(RawInputEvent::Text(RawTextInputEvent::new(
                            text.as_str(),
                            self.timestamp_millis(),
                        )));
                    }
                }
            }
            winit::event::WindowEvent::RedrawRequested => {
                if let Err(error) = self.render() {
                    self.fail_and_exit(event_loop, error);
                }
            }
            winit::event::WindowEvent::ScaleFactorChanged { .. } => {
                self.request_redraw();
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let winit::event::DeviceEvent::MouseMotion { delta } = event else {
            return;
        };
        let timestamp_millis = self.timestamp_millis();
        let handled = self
            .hooks
            .raw_mouse_motion
            .as_mut()
            .is_some_and(|raw_mouse_motion| {
                raw_mouse_motion(
                    &mut self.state,
                    NativeRawMouseMotion {
                        device_id,
                        delta,
                        timestamp_millis,
                        captured_canvas: captured_raw_mouse_canvas(&self.frame_state.interaction),
                    },
                )
            });
        if handled {
            self.request_redraw();
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self
            .hooks
            .idle_redraw
            .as_ref()
            .is_some_and(|idle_redraw| idle_redraw(&self.state))
        {
            self.request_redraw();
            return;
        }

        if self.continuous_redraw {
            self.request_redraw();
            return;
        }

        let now = Instant::now();
        if self
            .scheduled_redraw_at
            .is_some_and(|scheduled| now >= scheduled)
        {
            self.scheduled_redraw_at = None;
            self.request_redraw();
            return;
        }

        let mut wake_at = self.scheduled_redraw_at;
        if self.options.tick_action.is_some() {
            let next_tick = self.last_tick + self.options.tick_interval;
            if now >= next_tick {
                self.request_redraw();
                return;
            }
            wake_at = Some(
                wake_at
                    .map(|wake_at| wake_at.min(next_tick))
                    .unwrap_or(next_tick),
            );
        }

        if let Some(wake_at) = wake_at {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(wake_at));
        }
    }
}

fn collect_widget_actions(
    document: &UiDocument,
    frame: &crate::HostDocumentFrameOutput,
) -> Vec<WidgetAction> {
    let mut queue = WidgetActionQueue::new();
    for event in &frame.host_output.ui_events {
        if let Some((target, phase, position, selecting)) =
            text_pointer_edit_target(document, frame, event)
        {
            if let Some(binding) = action_binding(document, target) {
                let target_rect = document
                    .nodes()
                    .get(target.0)
                    .map(|node| node.layout.rect)
                    .unwrap_or_else(|| crate::UiRect::new(0.0, 0.0, 0.0, 0.0));
                queue.push(WidgetAction::text_pointer_edit(
                    target,
                    binding,
                    event.clone(),
                    phase,
                    position,
                    target_rect,
                    selecting,
                ));
                continue;
            }
        }
        let Some(target) = document.focus.focused else {
            continue;
        };
        let Some(binding) = action_binding(document, target) else {
            continue;
        };
        if text_edit_target(document, target)
            && matches!(event, UiInputEvent::TextInput(_) | UiInputEvent::Key { .. })
        {
            queue.push(WidgetAction::text_edit(target, binding, event.clone()));
            continue;
        }
        if text_edit_target(document, target) {
            if let Some((phase, position, selecting)) =
                text_pointer_edit_event(event, frame.host_output.state.pressed == Some(target))
            {
                let target_rect = document
                    .nodes()
                    .get(target.0)
                    .map(|node| node.layout.rect)
                    .unwrap_or_else(|| crate::UiRect::new(0.0, 0.0, 0.0, 0.0));
                queue.push(WidgetAction::text_pointer_edit(
                    target,
                    binding,
                    event.clone(),
                    phase,
                    position,
                    target_rect,
                    selecting,
                ));
                continue;
            }
        }
        if let UiInputEvent::Key { key, modifiers } = event {
            queue.push_key_activation(target, binding, *key, *modifiers);
        }
    }
    for gesture in &frame.host_output.gestures {
        queue.push_gesture_event_for_document(document, gesture, |id| action_binding(document, id));
    }
    for input in &frame.input_results {
        let Some(target) = input.scrolled else {
            continue;
        };
        let Some(binding) = action_binding(document, target) else {
            continue;
        };
        if let Some(scroll) = document.scroll_state(target) {
            queue.push(WidgetAction::scroll(target, binding, scroll));
        }
    }
    queue.into_vec()
}

fn text_pointer_edit_target(
    document: &UiDocument,
    frame: &crate::HostDocumentFrameOutput,
    event: &UiInputEvent,
) -> Option<(UiNodeId, WidgetValueEditPhase, UiPoint, bool)> {
    let (phase, position, selecting) = match event {
        UiInputEvent::PointerDown(point) => (WidgetValueEditPhase::Begin, *point, false),
        UiInputEvent::PointerMove(point) => {
            let target = frame.host_output.state.pressed?;
            if !text_edit_target(document, target) {
                return None;
            }
            return Some((target, WidgetValueEditPhase::Update, *point, true));
        }
        UiInputEvent::PointerUp(point) => {
            let target = frame.host_output.state.pressed.or(document.focus.pressed)?;
            if !text_edit_target(document, target) {
                return None;
            }
            return Some((target, WidgetValueEditPhase::Commit, *point, false));
        }
        _ => return None,
    };
    let target = document.hit_test(position)?;
    text_edit_target(document, target).then_some((target, phase, position, selecting))
}

fn text_pointer_edit_event(
    event: &UiInputEvent,
    pressed: bool,
) -> Option<(WidgetValueEditPhase, UiPoint, bool)> {
    match event {
        UiInputEvent::PointerDown(point) => Some((WidgetValueEditPhase::Begin, *point, false)),
        UiInputEvent::PointerMove(point) if pressed => {
            Some((WidgetValueEditPhase::Update, *point, true))
        }
        UiInputEvent::PointerUp(point) if pressed => {
            Some((WidgetValueEditPhase::Commit, *point, false))
        }
        _ => None,
    }
}

fn text_edit_target(document: &UiDocument, target: UiNodeId) -> bool {
    document
        .nodes()
        .get(target.0)
        .and_then(|node| node.accessibility.as_ref())
        .is_some_and(|accessibility| {
            matches!(
                accessibility.role,
                AccessibilityRole::TextBox | AccessibilityRole::SearchBox
            )
        })
}

fn action_binding(document: &UiDocument, id: UiNodeId) -> Option<WidgetActionBinding> {
    document
        .nodes()
        .get(id.0)
        .and_then(|node| node.action.clone())
}

fn resolve_target(
    event: &RawInputEvent,
    state: &crate::HostInteractionState,
    document: &UiDocument,
) -> Option<UiNodeId> {
    match event {
        RawInputEvent::Pointer(pointer) => state
            .drag_capture
            .filter(|capture| {
                capture.pointer_id == pointer.pointer_id
                    && matches!(
                        pointer.kind,
                        PointerEventKind::Move | PointerEventKind::Up(_) | PointerEventKind::Cancel
                    )
            })
            .map(|capture| capture.target)
            .or_else(|| document.hit_test(pointer.position)),
        RawInputEvent::Wheel(wheel) => document.hit_test(wheel.position),
        RawInputEvent::Keyboard(_) | RawInputEvent::Text(_) | RawInputEvent::Focus(_) => None,
    }
}

fn native_canvas_input_for_raw_event(
    document: &UiDocument,
    state: &crate::HostInteractionState,
    event: &RawInputEvent,
) -> Option<NativeCanvasInput> {
    match event {
        RawInputEvent::Pointer(pointer) => {
            let target = resolve_target(event, state, document)?;
            let (node, canvas, rect) = canvas_target(document, target)?;
            (canvas.interaction.pointer_capture || canvas.interaction.pointer_lock).then(|| {
                native_canvas_input(node, canvas, rect, Some(pointer.position), event.clone())
            })
        }
        RawInputEvent::Wheel(wheel) => resolve_target(event, state, document)
            .and_then(|target| canvas_target(document, target))
            .filter(|(_, canvas, _)| canvas.interaction.wheel_capture)
            .or_else(|| active_canvas_capture(document, state, |plan| plan.wheel_capture))
            .map(|(node, canvas, rect)| {
                native_canvas_input(node, canvas, rect, Some(wheel.position), event.clone())
            }),
        RawInputEvent::Keyboard(_) | RawInputEvent::Text(_) => state
            .focused
            .and_then(|focused| canvas_target(document, focused))
            .filter(|(_, canvas, _)| canvas.interaction.keyboard_capture)
            .or_else(|| active_canvas_capture(document, state, |plan| plan.keyboard_capture))
            .map(|(node, canvas, rect)| {
                native_canvas_input(node, canvas, rect, None, event.clone())
            }),
        RawInputEvent::Focus(_) => None,
    }
}

fn canvas_target(
    document: &UiDocument,
    target: UiNodeId,
) -> Option<(UiNodeId, &CanvasContent, UiRect)> {
    let mut current = Some(target);
    while let Some(id) = current {
        let node = document.nodes().get(id.0)?;
        if let UiContent::Canvas(canvas) = &node.content {
            return Some((id, canvas, node.layout.rect));
        }
        current = node.parent;
    }
    None
}

fn active_canvas_capture<'a>(
    document: &'a UiDocument,
    state: &crate::HostInteractionState,
    accepts: impl Fn(&crate::CanvasHostCapturePlan) -> bool,
) -> Option<(UiNodeId, &'a CanvasContent, UiRect)> {
    state
        .canvas_host_capture
        .active_plans()
        .iter()
        .find(|plan| accepts(plan))
        .and_then(|plan| canvas_target(document, plan.node))
}

fn captured_raw_mouse_canvas(state: &crate::HostInteractionState) -> Option<CanvasHostCaptureId> {
    state
        .canvas_host_capture
        .active_plans()
        .iter()
        .find(|plan| plan.pointer_lock)
        .map(CanvasHostCaptureId::from_plan)
}

fn native_canvas_input(
    node: UiNodeId,
    canvas: &CanvasContent,
    rect: UiRect,
    position: Option<UiPoint>,
    input: RawInputEvent,
) -> NativeCanvasInput {
    NativeCanvasInput {
        node,
        key: canvas.key.clone(),
        rect,
        local_position: position
            .map(|position| UiPoint::new(position.x - rect.x, position.y - rect.y)),
        input,
    }
}

fn logical_size(size: UiSize) -> winit::dpi::LogicalSize<f64> {
    winit::dpi::LogicalSize::new(size.width.max(1.0) as f64, size.height.max(1.0) as f64)
}

fn native_cursor_icon(shape: CursorShape) -> winit::window::CursorIcon {
    match shape {
        CursorShape::Default => winit::window::CursorIcon::Default,
        CursorShape::Pointer => winit::window::CursorIcon::Pointer,
        CursorShape::Text => winit::window::CursorIcon::Text,
        CursorShape::Crosshair => winit::window::CursorIcon::Crosshair,
        CursorShape::Grab => winit::window::CursorIcon::Grab,
        CursorShape::Grabbing => winit::window::CursorIcon::Grabbing,
        CursorShape::Move => winit::window::CursorIcon::Move,
        CursorShape::NotAllowed => winit::window::CursorIcon::NotAllowed,
        CursorShape::Wait => winit::window::CursorIcon::Wait,
        CursorShape::Progress => winit::window::CursorIcon::Progress,
        CursorShape::ResizeHorizontal => winit::window::CursorIcon::EwResize,
        CursorShape::ResizeVertical => winit::window::CursorIcon::NsResize,
        CursorShape::ResizeNorthEastSouthWest => winit::window::CursorIcon::NeswResize,
        CursorShape::ResizeNorthWestSouthEast => winit::window::CursorIcon::NwseResize,
        CursorShape::ZoomIn => winit::window::CursorIcon::ZoomIn,
        CursorShape::ZoomOut => winit::window::CursorIcon::ZoomOut,
    }
}

fn set_cursor_grab(
    window: &winit::window::Window,
    mode: CursorGrabMode,
) -> Result<CursorResponse, winit::error::ExternalError> {
    window
        .set_cursor_grab(native_cursor_grab_mode(mode))
        .map(|_| CursorResponse::Applied)
}

fn native_cursor_grab_mode(mode: CursorGrabMode) -> winit::window::CursorGrabMode {
    match mode {
        CursorGrabMode::None => winit::window::CursorGrabMode::None,
        CursorGrabMode::Confined => winit::window::CursorGrabMode::Confined,
        CursorGrabMode::Locked => winit::window::CursorGrabMode::Locked,
    }
}

fn cursor_error(error: impl ToString) -> CursorResponse {
    CursorResponse::Error(crate::platform::PlatformServiceError::new(
        PlatformErrorCode::Failed,
        error.to_string(),
    ))
}

fn canvas_pixel_size(width: f32, height: f32, scale: f32) -> Option<PixelSize> {
    let width = pixel_extent(width, scale)?;
    let height = pixel_extent(height, scale)?;
    Some(PixelSize::new(width, height))
}

fn pixel_extent(value: f32, scale: f32) -> Option<u32> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let pixels = (value * normalized_native_scale(scale)).ceil();
    if !pixels.is_finite() || pixels <= 0.0 {
        return None;
    }
    Some(pixels.min(u32::MAX as f32) as u32)
}

fn normalized_native_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

fn nonzero_window_size(size: winit::dpi::PhysicalSize<u32>) -> winit::dpi::PhysicalSize<u32> {
    winit::dpi::PhysicalSize::new(size.width.max(1), size.height.max(1))
}

fn pointer_button(button: winit::event::MouseButton) -> Option<PointerButton> {
    Some(match button {
        winit::event::MouseButton::Left => PointerButton::Primary,
        winit::event::MouseButton::Right => PointerButton::Secondary,
        winit::event::MouseButton::Middle => PointerButton::Auxiliary,
        winit::event::MouseButton::Back => PointerButton::Back,
        winit::event::MouseButton::Forward => PointerButton::Forward,
        winit::event::MouseButton::Other(value) => PointerButton::Other(value),
    })
}

fn key_modifiers(modifiers: winit::keyboard::ModifiersState) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.shift_key(),
        ctrl: modifiers.control_key(),
        alt: modifiers.alt_key(),
        meta: modifiers.super_key(),
    }
}

fn winit_modifiers(modifiers: KeyModifiers) -> winit::keyboard::ModifiersState {
    let mut state = winit::keyboard::ModifiersState::empty();
    if modifiers.shift {
        state |= winit::keyboard::ModifiersState::SHIFT;
    }
    if modifiers.ctrl {
        state |= winit::keyboard::ModifiersState::CONTROL;
    }
    if modifiers.alt {
        state |= winit::keyboard::ModifiersState::ALT;
    }
    if modifiers.meta {
        state |= winit::keyboard::ModifiersState::SUPER;
    }
    state
}

fn key_code(event: &winit::event::KeyEvent, modifiers: KeyModifiers) -> Option<KeyCode> {
    use winit::keyboard::{Key, NamedKey};

    match &event.logical_key {
        Key::Character(value) => value
            .chars()
            .next()
            .filter(|character| !character.is_control())
            .map(KeyCode::Character)
            .or_else(|| shortcut_physical_key_code(event, modifiers)),
        Key::Named(NamedKey::Backspace) => Some(KeyCode::Backspace),
        Key::Named(NamedKey::Delete) => Some(KeyCode::Delete),
        Key::Named(NamedKey::ArrowLeft) => Some(KeyCode::ArrowLeft),
        Key::Named(NamedKey::ArrowRight) => Some(KeyCode::ArrowRight),
        Key::Named(NamedKey::ArrowUp) => Some(KeyCode::ArrowUp),
        Key::Named(NamedKey::ArrowDown) => Some(KeyCode::ArrowDown),
        Key::Named(NamedKey::Home) => Some(KeyCode::Home),
        Key::Named(NamedKey::End) => Some(KeyCode::End),
        Key::Named(NamedKey::Enter) => Some(KeyCode::Enter),
        Key::Named(NamedKey::Escape) => Some(KeyCode::Escape),
        Key::Named(NamedKey::Tab) => Some(KeyCode::Tab),
        Key::Named(NamedKey::F10) => Some(KeyCode::F10),
        Key::Named(NamedKey::ContextMenu) => Some(KeyCode::ContextMenu),
        Key::Named(NamedKey::Space) => Some(KeyCode::Character(' ')),
        Key::Named(NamedKey::Copy) => Some(KeyCode::Character('c')),
        Key::Named(NamedKey::Cut) => Some(KeyCode::Character('x')),
        Key::Named(NamedKey::Paste) => Some(KeyCode::Character('v')),
        Key::Named(NamedKey::Undo) => Some(KeyCode::Character('z')),
        Key::Named(NamedKey::Redo) => Some(KeyCode::Character('y')),
        _ => shortcut_physical_key_code(event, modifiers),
    }
}

fn shortcut_physical_key_code(
    event: &winit::event::KeyEvent,
    modifiers: KeyModifiers,
) -> Option<KeyCode> {
    shortcut_physical_key_code_from_key(event.physical_key, modifiers)
}

fn shortcut_physical_key_code_from_key(
    physical_key: winit::keyboard::PhysicalKey,
    modifiers: KeyModifiers,
) -> Option<KeyCode> {
    use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};

    if !modifiers.ctrl && !modifiers.meta {
        return None;
    }

    // Some platforms report Ctrl+C/V/X as control characters; recover the
    // shortcut identity from the physical key only while a command modifier is down.
    match physical_key {
        PhysicalKey::Code(WinitKeyCode::KeyA) => Some(KeyCode::Character('a')),
        PhysicalKey::Code(WinitKeyCode::KeyC) => Some(KeyCode::Character('c')),
        PhysicalKey::Code(WinitKeyCode::KeyV) => Some(KeyCode::Character('v')),
        PhysicalKey::Code(WinitKeyCode::KeyX) => Some(KeyCode::Character('x')),
        PhysicalKey::Code(WinitKeyCode::KeyY) => Some(KeyCode::Character('y')),
        PhysicalKey::Code(WinitKeyCode::KeyZ) => Some(KeyCode::Character('z')),
        _ => None,
    }
}

fn wheel_delta(delta: winit::event::MouseScrollDelta) -> (UiPoint, WheelDeltaUnit) {
    match delta {
        winit::event::MouseScrollDelta::LineDelta(x, y) => {
            (UiPoint::new(-x, -y), WheelDeltaUnit::Line)
        }
        winit::event::MouseScrollDelta::PixelDelta(delta) => (
            UiPoint::new(-delta.x as f32, -delta.y as f32),
            WheelDeltaUnit::Pixel,
        ),
    }
}

fn wheel_phase(phase: winit::event::TouchPhase) -> WheelPhase {
    match phase {
        winit::event::TouchPhase::Started => WheelPhase::Started,
        winit::event::TouchPhase::Moved => WheelPhase::Moved,
        winit::event::TouchPhase::Ended => WheelPhase::Ended,
        winit::event::TouchPhase::Cancelled => WheelPhase::Ended,
    }
}

fn restore_scroll_offsets(document: &mut UiDocument, offsets: &HashMap<String, UiPoint>) -> bool {
    if offsets.is_empty() {
        return false;
    }
    let mut scroll_offsets = Vec::new();
    for index in 0..document.node_count() {
        let id = UiNodeId(index);
        if document.scroll_state(id).is_none() {
            continue;
        }
        let Some(offset) = offsets.get(&node_path_key(document, id)).copied() else {
            continue;
        };
        scroll_offsets.push((id, offset));
    }

    let mut changed = false;
    for (id, offset) in scroll_offsets {
        let Some(scroll) = document.node_mut(id).scroll.as_mut() else {
            continue;
        };
        let offset = UiPoint::new(offset.x.max(0.0), offset.y.max(0.0));
        if scroll.offset != offset {
            scroll.offset = offset;
            changed = true;
        }
    }
    changed
}

fn scroll_offset_is_zero(offset: UiPoint) -> bool {
    offset.x.abs() <= f32::EPSILON && offset.y.abs() <= f32::EPSILON
}

fn node_path_key(document: &UiDocument, id: UiNodeId) -> String {
    let mut path = Vec::new();
    let mut current = Some(id);
    while let Some(id) = current {
        let node = document.node(id);
        path.push(node.name.as_str());
        current = node.parent;
    }
    path.reverse();
    path.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ApproxTextMeasurer;

    #[test]
    fn native_shortcut_key_fallback_maps_physical_clipboard_keys_with_modifiers() {
        use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};

        let ctrl = KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        };
        let meta = KeyModifiers {
            meta: true,
            ..KeyModifiers::NONE
        };

        assert_eq!(
            shortcut_physical_key_code_from_key(PhysicalKey::Code(WinitKeyCode::KeyC), ctrl),
            Some(KeyCode::Character('c'))
        );
        assert_eq!(
            shortcut_physical_key_code_from_key(PhysicalKey::Code(WinitKeyCode::KeyV), meta),
            Some(KeyCode::Character('v'))
        );
        assert_eq!(
            shortcut_physical_key_code_from_key(
                PhysicalKey::Code(WinitKeyCode::KeyC),
                KeyModifiers::NONE
            ),
            None
        );
    }

    #[test]
    fn native_wheel_delta_uses_document_scroll_direction() {
        let (line_delta, line_unit) =
            wheel_delta(winit::event::MouseScrollDelta::LineDelta(0.0, -2.0));
        assert_eq!(line_unit, WheelDeltaUnit::Line);
        assert_eq!(line_delta, UiPoint::new(0.0, 2.0));

        let (pixel_delta, pixel_unit) = wheel_delta(winit::event::MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0.0, -48.0),
        ));
        assert_eq!(pixel_unit, WheelDeltaUnit::Pixel);
        assert_eq!(pixel_delta, UiPoint::new(0.0, 48.0));
    }

    #[test]
    fn native_cursor_helpers_map_public_cursor_contracts_to_winit() {
        assert_eq!(
            native_cursor_icon(CursorShape::Pointer),
            winit::window::CursorIcon::Pointer
        );
        assert_eq!(
            native_cursor_icon(CursorShape::ResizeNorthEastSouthWest),
            winit::window::CursorIcon::NeswResize
        );
        assert_eq!(
            native_cursor_grab_mode(CursorGrabMode::None),
            winit::window::CursorGrabMode::None
        );
        assert_eq!(
            native_cursor_grab_mode(CursorGrabMode::Locked),
            winit::window::CursorGrabMode::Locked
        );
    }

    #[test]
    fn native_canvas_input_resolves_local_pointer_wheel_and_keyboard_events() {
        let mut document = UiDocument::new(crate::LayoutStyle::size(200.0, 160.0));
        let root = document.root;
        let mut canvas = crate::UiNode::canvas(
            "viewport",
            "viewport",
            crate::LayoutStyle::size(100.0, 80.0),
        );
        canvas.content = UiContent::Canvas(
            CanvasContent::new("viewport").interaction(crate::CanvasInteractionPolicy::EDITOR),
        );
        let canvas_id = document.add_child(root, canvas);

        let mut measurer = ApproxTextMeasurer;
        document
            .compute_layout(UiSize::new(200.0, 160.0), &mut measurer)
            .unwrap();

        let state = crate::HostInteractionState::default();
        let pointer = RawInputEvent::Pointer(RawPointerEvent::new(
            PointerEventKind::Move,
            UiPoint::new(20.0, 12.0),
            1,
        ));
        let pointer_input = native_canvas_input_for_raw_event(&document, &state, &pointer).unwrap();
        assert_eq!(pointer_input.node, canvas_id);
        assert_eq!(pointer_input.key, "viewport");
        assert_eq!(pointer_input.local_position, Some(UiPoint::new(20.0, 12.0)));
        assert_eq!(pointer_input.input, pointer);

        let wheel = RawInputEvent::Wheel(RawWheelEvent::pixels(
            UiPoint::new(24.0, 18.0),
            UiPoint::new(0.0, 10.0),
            2,
        ));
        let wheel_input = native_canvas_input_for_raw_event(&document, &state, &wheel).unwrap();
        assert_eq!(wheel_input.node, canvas_id);
        assert_eq!(wheel_input.local_position, Some(UiPoint::new(24.0, 18.0)));

        let keyboard = RawInputEvent::Keyboard(RawKeyboardEvent::press(
            KeyCode::Character('w'),
            KeyModifiers::NONE,
            3,
        ));
        assert!(native_canvas_input_for_raw_event(&document, &state, &keyboard).is_none());

        let mut state = crate::HostInteractionState {
            focused: Some(canvas_id),
            ..Default::default()
        };
        let keyboard_input =
            native_canvas_input_for_raw_event(&document, &state, &keyboard).unwrap();
        assert_eq!(keyboard_input.node, canvas_id);
        assert_eq!(keyboard_input.local_position, None);

        state.focused = None;
        state
            .canvas_host_capture
            .sync([crate::CanvasHostCapturePlan {
                node: canvas_id,
                key: "viewport".to_string(),
                rect: document.node(canvas_id).layout.rect,
                pointer_capture: true,
                keyboard_capture: true,
                wheel_capture: true,
                pointer_lock: false,
                domain_hit_testing: true,
            }]);
        let keyboard_input =
            native_canvas_input_for_raw_event(&document, &state, &keyboard).unwrap();
        assert_eq!(keyboard_input.node, canvas_id);

        let mut capture_state = crate::HostInteractionState::default();
        capture_state
            .canvas_host_capture
            .sync([crate::CanvasHostCapturePlan {
                node: canvas_id,
                key: "viewport".to_string(),
                rect: document.node(canvas_id).layout.rect,
                pointer_capture: true,
                keyboard_capture: true,
                wheel_capture: true,
                pointer_lock: true,
                domain_hit_testing: true,
            }]);
        assert_eq!(
            captured_raw_mouse_canvas(&capture_state),
            Some(crate::CanvasHostCaptureId::new(canvas_id, "viewport"))
        );
    }

    #[test]
    fn native_scroll_offsets_restore_to_stable_scroll_nodes_before_layout() {
        let mut document = UiDocument::new(crate::LayoutStyle::column().with_size(100.0, 80.0));
        let scroll = document.add_child(
            document.root,
            crate::UiNode::container(
                "scroll",
                crate::LayoutStyle::column().with_size(100.0, 80.0),
            )
            .with_scroll(crate::ScrollAxes::VERTICAL),
        );
        let child = document.add_child(
            scroll,
            crate::UiNode::container(
                "content",
                crate::LayoutStyle::size(100.0, 240.0).with_flex_shrink(0.0),
            ),
        );

        let mut measurer = ApproxTextMeasurer;
        document
            .compute_layout(UiSize::new(100.0, 80.0), &mut measurer)
            .unwrap();

        let mut offsets = HashMap::new();
        offsets.insert(node_path_key(&document, scroll), UiPoint::new(0.0, 120.0));

        assert!(restore_scroll_offsets(&mut document, &offsets));
        document
            .compute_layout(UiSize::new(100.0, 80.0), &mut measurer)
            .unwrap();
        assert!(!document.clamp_scroll_offsets());

        assert_eq!(
            document.scroll_state(scroll).unwrap().offset,
            UiPoint::new(0.0, 120.0)
        );
        assert_eq!(document.node(child).layout.rect.y, -120.0);
    }

    #[test]
    fn native_window_error_includes_application_error_and_frame_timing() {
        let error = NativeWindowRunError::new(
            "showcase",
            "running the platform event loop",
            "ExitFailure(1)",
            Some("Io error: Broken pipe (os error 32)".to_string()),
            Some(NativeFrameTimingReport {
                viewport: UiSize::new(1200.0, 900.0),
                nodes: 1635,
                paint_items: 1262,
                widget_actions: 0,
                build_document: Duration::from_millis(423),
                host_input: Duration::from_millis(1),
                document_frame: Duration::from_millis(4),
                action_rebuild: None,
                canvas_render: Duration::from_millis(2),
                surface_render: Duration::from_millis(8),
                total: Duration::from_millis(438),
            }),
        )
        .to_string();

        assert!(error.contains("native window \"showcase\" failed"));
        assert!(error.contains("ExitFailure(1)"));
        assert!(error.contains("Broken pipe"));
        assert!(error.contains("last completed frame"));
        assert!(error.contains("build_document=423ms"));
        assert!(error.contains("nodes=1635"));
    }
}
