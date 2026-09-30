//! Native window runner for the default WGPU/winit path.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod clipboard;
mod ime;

use clipboard::NativeClipboard;

use crate::input::{
    text_input_for_key_event_text, PointerButton, PointerButtons, PointerEventKind, RawInputEvent,
    RawKeyboardEvent, RawPointerEvent, RawTextInputEvent, RawWheelEvent, WheelDeltaUnit,
    WheelPhase,
};
use crate::platform::{
    BackendCapabilities, CursorGrabMode, CursorRequest, CursorResponse, CursorShape, LogicalRect,
    OpenUrlResponse, PixelSize, PlatformErrorCode, PlatformRequest, PlatformRequestIdAllocator,
    PlatformResponse, PlatformServiceError, PlatformServiceRequest, PlatformServiceResponse,
    RepaintRequest, RepaintResponse, TextImeRequest, TextImeResponse,
};
use crate::renderer::EmptyResourceResolver;
use crate::renderer::{
    CanvasRenderOutcome, CanvasRenderOutput, CanvasRenderReport, CanvasRenderRequest,
    DirtyRegionSet, RenderError, RenderFrameRequest, RenderTarget, RendererAdapter,
};
use crate::wgpu_renderer::{WgpuCanvasContext, WgpuSurfaceRenderer};
use crate::{
    errors::{
        classify_render_error, ErrorKind, ErrorReport, FallbackAction, FallbackDecision,
        RendererErrorKind, RuntimeErrorKind,
    },
    host::{HostDocumentFrameOutput, HostFrameOutput, HostNodeInteraction},
};
use crate::{
    CosmicTextMeasurer, KeyCode, KeyModifiers, UiDocument, UiNodeId, UiPoint, UiSize, WidgetAction,
    WidgetActionBinding,
};

#[cfg(test)]
use super::integration::input::canvas_input_for_raw_event;
use super::integration::input::captured_raw_mouse_canvas;
use super::{RawMouseMotion, RuntimeHooks, RuntimeMetrics, RuntimeObservation};
#[cfg(test)]
use crate::host::HostInteractionState;
#[cfg(test)]
use crate::renderer::{CanvasHostCaptureId, CanvasHostCapturePlan};
#[cfg(test)]
use crate::{CanvasContent, UiContent};

pub type NativeWindowResult<T = ()> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
struct NativeWindowRunError {
    title: String,
    phase: &'static str,
    message: String,
    report: Option<ErrorReport>,
    app_error: Option<NativeRuntimeFailure>,
    last_frame: Option<NativeFrameTimingReport>,
}

impl NativeWindowRunError {
    fn new(
        title: impl Into<String>,
        phase: &'static str,
        message: impl Into<String>,
        app_error: Option<NativeRuntimeFailure>,
        last_frame: Option<NativeFrameTimingReport>,
    ) -> Self {
        Self {
            title: title.into(),
            phase,
            message: message.into(),
            report: None,
            app_error,
            last_frame,
        }
    }

    #[cfg(test)]
    fn from_report(
        title: impl Into<String>,
        phase: &'static str,
        report: ErrorReport,
        app_error: Option<NativeRuntimeFailure>,
        last_frame: Option<NativeFrameTimingReport>,
    ) -> Self {
        Self {
            title: title.into(),
            phase,
            message: report.message.clone(),
            report: Some(report),
            app_error,
            last_frame,
        }
    }

    fn from_failure(
        title: impl Into<String>,
        failure: NativeRuntimeFailure,
        last_frame: Option<NativeFrameTimingReport>,
    ) -> Self {
        Self {
            title: title.into(),
            phase: failure.phase,
            message: failure.message,
            report: failure.report,
            app_error: None,
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
        if let Some(report) = &self.report {
            write!(f, "\nerror report: {report}")?;
        }
        if let Some(app_error) = &self.app_error {
            write!(f, "\nlast application error: {app_error}")?;
        }
        if let Some(last_frame) = self.last_frame {
            write!(f, "\nlast completed frame: {last_frame}")?;
        }
        Ok(())
    }
}

impl Error for NativeWindowRunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.report.as_ref().map(|report| report as &dyn Error)
    }
}

#[derive(Debug, Clone)]
struct NativeRuntimeFailure {
    phase: &'static str,
    message: String,
    report: Option<ErrorReport>,
}

impl NativeRuntimeFailure {
    fn from_error(phase: &'static str, error: Box<dyn Error>) -> Self {
        let message = error.to_string();
        let report = error.downcast_ref::<ErrorReport>().cloned().or_else(|| {
            error
                .downcast_ref::<RenderError>()
                .map(classify_render_error)
        });
        Self {
            phase,
            message,
            report,
        }
    }
}

impl fmt::Display for NativeRuntimeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.phase, self.message)?;
        if let Some(report) = &self.report {
            write!(formatter, "\n{report}")?;
        }
        Ok(())
    }
}

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
        request: &RenderFrameRequest,
    ) -> CanvasRenderReport {
        let mut report = CanvasRenderReport::default();
        if self.handlers.is_empty() {
            return report;
        }
        for (item, canvas) in request.canvas_items() {
            let Some(handler) = self.handlers.get_mut(&canvas.key) else {
                continue;
            };
            let canvas_request = CanvasRenderRequest::from_canvas(item, canvas);
            let Some(size) = canvas_pixel_size(
                canvas_request.rect.width,
                canvas_request.rect.height,
                request.options.scale_factor
                    * normalized_native_scale(canvas_request.transform.scale),
            ) else {
                report.outcomes.push(CanvasRenderOutcome::Failed {
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
                Ok(output) => report.outcomes.push(CanvasRenderOutcome::Rendered {
                    request: canvas_request,
                    output,
                }),
                Err(error) => report.outcomes.push(CanvasRenderOutcome::Failed {
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

#[derive(Clone)]
pub struct NativeWindowOptions {
    /// Compute initial size using native monitor information when the event loop starts.
    pub initial_size: Option<Arc<dyn Fn(&winit::event_loop::ActiveEventLoop) -> UiSize>>,
    pub title: String,
    pub size: UiSize,
    pub min_size: Option<UiSize>,
    pub ui_scale: f32,
    pub tick_action: Option<WidgetActionBinding>,
    pub tick_interval: Duration,
}

impl fmt::Debug for NativeWindowOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeWindowOptions")
            .field("title", &self.title)
            .field("size", &self.size)
            .field("min_size", &self.min_size)
            .field("ui_scale", &self.ui_scale)
            .field("tick_action", &self.tick_action)
            .field("tick_interval", &self.tick_interval)
            .field(
                "initial_size",
                &self.initial_size.as_ref().map(|_| "callback"),
            )
            .finish()
    }
}

impl NativeWindowOptions {
    pub fn with_initial_size(
        mut self,
        hook: impl Fn(&winit::event_loop::ActiveEventLoop) -> UiSize + 'static,
    ) -> Self {
        self.initial_size = Some(Arc::new(hook));
        self
    }
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
            initial_size: None,
            size: UiSize::new(1024.0, 720.0),
            min_size: Some(UiSize::new(480.0, 320.0)),
            ui_scale: 1.0,
            tick_action: None,
            tick_interval: Duration::from_millis(16),
        }
    }
}

pub fn native_window_capabilities() -> BackendCapabilities {
    BackendCapabilities::native_window()
}

fn native_startup_report(
    title: &str,
    kind: RuntimeErrorKind,
    operation: &'static str,
    message: impl Into<String>,
    next_step: &'static str,
) -> ErrorReport {
    ErrorReport::fatal(ErrorKind::Runtime(kind), message)
        .context("backend", "native-window")
        .context("target", title)
        .context("operation", operation)
        .context("host_subsystem", "winit/wgpu startup")
        .context("user_visible_consequence", "the native window did not open")
        .context("next_step", next_step)
        .fallback(FallbackDecision::abort_frame(
            "native startup cannot continue without a window and WGPU surface",
        ))
}

fn native_renderer_startup_report(title: &str, error: RenderError) -> ErrorReport {
    classify_render_error(&error)
        .context("backend", "native-window")
        .context("target", title)
        .context("operation", "initializing the WGPU surface renderer")
        .context("host_subsystem", "WgpuSurfaceRenderer")
        .context("user_visible_consequence", "the native window did not open")
        .context(
            "next_step",
            "Check GPU adapter support, surface format support, and renderer initialization logs.",
        )
        .fallback(FallbackDecision::abort_frame(
            "native startup cannot continue without a renderer",
        ))
}

fn render_error_uses_cached_frame(error: &RenderError) -> bool {
    classify_render_error(error).fallback.action == FallbackAction::UseCachedFrame
}

pub fn run(
    title: impl Into<String>,
    view: impl FnMut(UiSize) -> UiDocument + 'static,
) -> NativeWindowResult {
    run_ui_document(title, view)
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
        move |_state: &(), viewport, _views| view(viewport),
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
        move |_state: &(), viewport, _views| view(viewport),
        canvas_renderers,
    )
}

pub fn run_app<State>(
    title: impl Into<String>,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
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
    view: impl FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
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
    view: impl FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
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
        RuntimeHooks::default(),
    )
}

pub fn run_app_with_canvas_renderers_and_hooks<State>(
    options: NativeWindowOptions,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
    canvas_renderers: NativeWgpuCanvasRenderRegistry<State>,
    hooks: RuntimeHooks<State>,
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
    let proxy = event_loop.create_proxy();
    let task_proxy = proxy.clone();
    let mut hooks = hooks;
    hooks.set_task_waker(move || {
        let _ = task_proxy.send_event(());
    });
    let mut app =
        NativeWindowApp::new(options, state, update, view, canvas_renderers, hooks, proxy);
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
        Err(NativeWindowRunError::from_failure(
            app.options.title.clone(),
            error,
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
    event_loop_proxy: winit::event_loop::EventLoopProxy<()>,
    device_loss: Arc<Mutex<Option<ErrorReport>>>,
    canvas_renderers: NativeWgpuCanvasRenderRegistry<State>,
    hooks: RuntimeHooks<State>,
    session: super::session::RuntimeSession,
    platform_request_ids: PlatformRequestIdAllocator,
    clipboard: NativeClipboard,
    pending_platform_responses: Vec<PlatformServiceResponse>,
    text_measurer: CosmicTextMeasurer,
    pending_input: Vec<RawInputEvent>,
    text_input: ime::NativeTextInput,
    cursor: Option<UiPoint>,
    modifiers: KeyModifiers,
    buttons: PointerButtons,
    start: Instant,
    last_tick: Instant,
    last_animation_tick: Instant,
    last_frame_report: Option<NativeFrameTimingReport>,
    error: Option<NativeRuntimeFailure>,
}

impl<State, Update, View> NativeWindowApp<State, Update, View> {
    fn new(
        options: NativeWindowOptions,
        state: State,
        update: Update,
        view: View,
        canvas_renderers: NativeWgpuCanvasRenderRegistry<State>,
        hooks: RuntimeHooks<State>,
        event_loop_proxy: winit::event_loop::EventLoopProxy<()>,
    ) -> Self {
        Self {
            options,
            state,
            update,
            view,
            window: None,
            window_id: None,
            renderer: None,
            event_loop_proxy,
            device_loss: Arc::new(Mutex::new(None)),
            canvas_renderers,
            hooks,
            session: super::session::RuntimeSession::new(),
            platform_request_ids: PlatformRequestIdAllocator::default(),
            clipboard: NativeClipboard::default(),
            pending_platform_responses: Vec::new(),
            text_measurer: CosmicTextMeasurer::new(),
            pending_input: Vec::new(),
            text_input: ime::NativeTextInput::default(),
            cursor: None,
            modifiers: KeyModifiers::NONE,
            buttons: PointerButtons::NONE,
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
            .options
            .initial_size
            .as_ref()
            .map(|initial_size| initial_size(event_loop))
            .unwrap_or(self.options.size);
        attributes = attributes.with_inner_size(logical_size(initial_size));
        if let Some(min_size) = self.options.min_size {
            attributes = attributes.with_min_inner_size(logical_size(min_size));
        }
        let window = Arc::new(event_loop.create_window(attributes).map_err(|error| {
            native_startup_report(
                &self.options.title,
                RuntimeErrorKind::WindowCreation,
                "creating the native window",
                error.to_string(),
                "Check the windowing backend, display server, and platform permissions.",
            )
        })?);
        let size = nonzero_window_size(window.inner_size());

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone()).map_err(|error| {
            native_startup_report(
                &self.options.title,
                RuntimeErrorKind::SurfaceCreation,
                "creating the WGPU surface",
                error.to_string(),
                "Verify that the native window exposes a compatible raw window/display handle.",
            )
        })?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
        }))
        .map_err(|error| {
            native_startup_report(
                &self.options.title,
                RuntimeErrorKind::AdapterRequest,
                "requesting a WGPU adapter",
                error.to_string(),
                "Install a supported GPU driver or enable a WGPU backend available on this platform.",
            )
        })?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("native-window-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .map_err(|error| {
            native_startup_report(
                &self.options.title,
                RuntimeErrorKind::DeviceRequest,
                "requesting a WGPU device",
                error.to_string(),
                "Lower required WGPU features/limits or update the GPU driver.",
            )
        })?;
        let device_loss = self.device_loss.clone();
        let proxy = self.event_loop_proxy.clone();
        let title = self.options.title.clone();
        device.set_device_lost_callback(move |reason, message| {
            *device_loss
                .lock()
                .expect("native device loss notification poisoned") = Some(
                ErrorReport::fatal(
                    ErrorKind::Renderer(RendererErrorKind::DeviceLost),
                    format!("graphics device lost ({reason:?}): {message}"),
                )
                .context("backend", "native-window")
                .context("target", title.clone())
                .context(
                    "next_step",
                    "Restart the application to recreate its graphics device.",
                )
                .fallback(FallbackDecision::abort_frame(
                    "the lost graphics device cannot present another frame",
                )),
            );
            // Device callbacks may run on another thread, even while the UI is idle.
            let _ = proxy.send_event(());
        });
        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or_else(|| {
                native_startup_report(
                    &self.options.title,
                    RuntimeErrorKind::SurfaceConfiguration,
                    "selecting the WGPU surface configuration",
                    "adapter does not support the native window surface",
                    "Try another WGPU backend or run on a GPU/display combination with surface presentation support.",
                )
            })?;

        self.window_id = Some(window.id());
        self.renderer = Some(
            WgpuSurfaceRenderer::new(surface, device, queue, surface_config)
                .map_err(|error| native_renderer_startup_report(&self.options.title, error))?,
        );
        self.window = Some(window);
        Ok(())
    }

    fn request_redraw(&self) {
        if self
            .session
            .frame_retry_delay(self.start.elapsed())
            .is_some()
        {
            return;
        }
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
        let metrics = RuntimeMetrics {
            physical_size: PixelSize::new(size.width.max(1), size.height.max(1)),
            viewport: dpi_viewport,
            scale_factor: dpi_scale,
            dpi_scale,
            elapsed: self.start.elapsed(),
        };
        self.hooks
            .scale_factor
            .as_ref()
            .map(|scale_factor| normalized_native_scale(scale_factor(&self.state, metrics)))
            .unwrap_or(dpi_scale)
    }

    fn metrics_for_viewport(&self, viewport: UiSize) -> RuntimeMetrics {
        let size = self
            .window
            .as_ref()
            .map(|window| window.inner_size())
            .unwrap_or_else(|| winit::dpi::PhysicalSize::new(1, 1));
        RuntimeMetrics {
            physical_size: PixelSize::new(size.width.max(1), size.height.max(1)),
            viewport,
            scale_factor: self.scale_factor_for_size(size),
            dpi_scale: self.dpi_scale(),
            elapsed: self.start.elapsed(),
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

    fn apply_platform_service_requests(&mut self, frame: &HostDocumentFrameOutput) {
        let requests = frame.platform_service_requests(&mut self.platform_request_ids);
        if requests.is_empty() {
            return;
        }
        let responses = requests
            .into_iter()
            .map(|request| self.apply_platform_service_request(request))
            .collect::<Vec<_>>();
        self.dispatch_platform_responses(&responses);
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
            PlatformRequest::Clipboard(request) => {
                PlatformResponse::Clipboard(self.clipboard.apply(request))
            }
            PlatformRequest::OpenUrl(request) => {
                PlatformResponse::OpenUrl(open_native_url(&request.url))
            }
            PlatformRequest::Cursor(request) => {
                PlatformResponse::Cursor(self.apply_cursor_request(request))
            }
            PlatformRequest::Repaint(request) => {
                PlatformResponse::Repaint(self.apply_repaint_request(request))
            }
            PlatformRequest::TextIme(request) => {
                PlatformResponse::TextIme(self.apply_text_ime_request(request))
            }
            request => PlatformResponse::unsupported(request.kind()),
        }
    }

    fn apply_hook_platform_requests(&mut self, metrics: RuntimeMetrics) {
        let requests = self.session.take_platform_requests(
            &mut self.hooks,
            &mut self.state,
            metrics,
            &mut self.platform_request_ids,
        );
        let responses = requests
            .into_iter()
            .map(|request| self.apply_platform_service_request(request))
            .collect::<Vec<_>>();
        self.dispatch_platform_responses(&responses);
        self.pending_platform_responses.extend(responses);
    }

    fn dispatch_platform_responses(&mut self, responses: &[PlatformServiceResponse]) {
        self.session
            .apply_platform_responses(&mut self.hooks, &mut self.state, responses);
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
        let now = self.start.elapsed();
        let response = self.session.request_repaint(now, request);
        if self.session.next_frame_delay(now) == Some(Duration::ZERO) {
            self.request_redraw();
        }
        response
    }

    fn apply_text_ime_request(&mut self, request: TextImeRequest) -> TextImeResponse {
        let Some(window) = self.window.as_ref() else {
            return TextImeResponse::Unsupported;
        };
        self.session.apply_text_ime_request(&request);
        match request {
            TextImeRequest::Activate(session) | TextImeRequest::Update(session) => {
                if self.text_input.configure(session.clone()) {
                    window.set_ime_allowed(false);
                    window.set_ime_allowed(true);
                }
                window.set_ime_purpose(if session.sensitive {
                    winit::window::ImePurpose::Password
                } else {
                    winit::window::ImePurpose::Normal
                });
                set_native_ime_cursor_area(window, session.cursor_rect, self.scale_factor());
                TextImeResponse::Activated {
                    input: session.input,
                }
            }
            TextImeRequest::Deactivate { input } | TextImeRequest::HideKeyboard { input } => {
                if self.text_input.deactivate(&input) {
                    window.set_ime_allowed(false);
                }
                TextImeResponse::Deactivated { input }
            }
            TextImeRequest::ShowKeyboard { input } => {
                if self
                    .text_input
                    .session
                    .as_ref()
                    .is_some_and(|session| session.input == input)
                {
                    window.set_ime_allowed(true);
                }
                TextImeResponse::Activated { input }
            }
        }
    }

    fn render(&mut self) -> NativeWindowResult
    where
        Update: FnMut(&mut State, WidgetAction),
        View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument,
    {
        if let Some(error) = self
            .device_loss
            .lock()
            .expect("native device loss notification poisoned")
            .take()
        {
            return Err(error.into());
        }
        // OS redraws can already be queued when presentation fails.
        if self
            .session
            .frame_retry_delay(self.start.elapsed())
            .is_some()
        {
            return Ok(());
        }
        self.session
            .apply_task_completions(&mut self.hooks, &mut self.state);
        let Some(viewport) = self.viewport() else {
            return Ok(());
        };
        let frame_started = Instant::now();
        self.session.begin_frame(self.start.elapsed());
        let metrics = self.metrics_for_viewport(viewport);
        self.session
            .apply_before_render(&mut self.hooks, &mut self.state, metrics);
        if let (Some(window), Some(title)) = (self.window.as_ref(), self.hooks.title.as_ref()) {
            window.set_title(&title(&self.state));
        }
        self.dispatch_tick_if_due();
        self.apply_hook_platform_requests(metrics);
        let raw_input = std::mem::take(&mut self.pending_input);
        let animation_dt = self.animation_delta_seconds();

        let build_started = Instant::now();
        let mut document = self.build_document(viewport)?;
        let mut build_document = build_started.elapsed();
        let mut nodes = document.node_count();
        document.tick_animations(animation_dt);
        let host_input_started = Instant::now();
        let host_output = self.session.process_input_with_hooks(
            &mut document,
            viewport,
            &raw_input,
            &std::mem::take(&mut self.pending_platform_responses),
            &mut self.hooks,
            &mut self.state,
            &mut self.text_measurer,
        )?;
        let host_input = host_input_started.elapsed();
        let document_frame_started = Instant::now();
        let frame = self.session.finish_frame(
            &mut document,
            viewport,
            RenderTarget::window(self.options.title.clone(), viewport),
            host_output,
            &mut self.text_measurer,
            &mut self.platform_request_ids,
        )?;
        let mut document_frame = document_frame_started.elapsed();
        let actions = crate::host::collect_document_widget_actions(&frame);
        let actions_count = actions.len();
        self.apply_platform_service_requests(&frame);

        let mut action_rebuild = None;
        let frame = if actions.is_empty() && !self.session.view_needs_rebuild() {
            frame
        } else {
            let action_started = Instant::now();
            for action in actions {
                (self.update)(&mut self.state, action);
                self.session.invalidate_view();
            }
            self.apply_hook_platform_requests(metrics);
            let rebuild_started = Instant::now();
            self.session.retain_document(document);
            document = self.build_document(viewport)?;
            build_document += rebuild_started.elapsed();
            nodes = document.node_count();
            let document_frame_started = Instant::now();
            let frame = self.session.finish_frame(
                &mut document,
                viewport,
                RenderTarget::window(self.options.title.clone(), viewport),
                HostFrameOutput::new(self.session.interaction().clone()),
                &mut self.text_measurer,
                &mut self.platform_request_ids,
            )?;
            document_frame += document_frame_started.elapsed();
            self.apply_platform_service_requests(&frame);
            action_rebuild = Some(action_started.elapsed());
            frame
        };

        self.session
            .reconcile_input_hooks(&document, &mut self.hooks, &mut self.state);
        if self.session.view_needs_rebuild() {
            // A response to the second document pass may update application
            // state again. Defer that work without losing the wakeup.
            let now = self.start.elapsed();
            self.session.request_repaint(now, RepaintRequest::NextFrame);
        }
        self.hooks.observe(
            &self.state,
            RuntimeObservation::new(metrics, &document, &frame, self.session.view_build_stats()),
        );
        self.session.retain_document(document);
        let Some(renderer) = self.renderer.as_mut() else {
            self.session.frame_failed(self.start.elapsed());
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
        // Unmatched registrations cannot change application state. A matching
        // callback may mutate it even when that callback returns an error.
        if !canvas_report.outcomes.is_empty() {
            self.session.invalidate_view();
        }
        let canvas_render = canvas_started.elapsed();
        if let Some(error) = self
            .device_loss
            .lock()
            .expect("native device loss notification poisoned")
            .take()
        {
            return Err(error.into());
        }
        if let Some(error) = canvas_report.first_failure().cloned() {
            return Err(error.into());
        }
        let repaint_requested = canvas_report.repaint_requested();
        let surface_started = Instant::now();
        let render_result = renderer.render_frame(frame.render_request, &EmptyResourceResolver);
        let surface_render = surface_started.elapsed();
        if let Some(error) = self
            .device_loss
            .lock()
            .expect("native device loss notification poisoned")
            .take()
        {
            return Err(error.into());
        }
        if let Err(error) = render_result {
            if render_error_uses_cached_frame(&error) {
                self.session.frame_failed(self.start.elapsed());
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
                self.request_redraw();
                return Ok(());
            }
            return Err(error.into());
        }
        self.session.frame_presented();
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
        self.session.invalidate_view();
    }

    fn build_document(&mut self, viewport: UiSize) -> Result<UiDocument, taffy::TaffyError>
    where
        View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument,
    {
        let scale = crate::UiDocumentScale::new(self.options.ui_scale, self.scale_factor());
        let mut document = self.session.build_document(
            viewport,
            scale,
            self.cursor,
            &mut self.text_measurer,
            |viewport, views| (self.view)(&self.state, viewport, views),
        )?;
        self.session
            .reconcile_input_hooks(&document, &mut self.hooks, &mut self.state);
        if self.session.view_needs_rebuild() {
            // Owner removal can cancel an application edit. Include that cleanup
            // in this frame's view before producing its final paint and observation.
            self.session.retain_document(document);
            document = self.session.build_document(
                viewport,
                scale,
                self.cursor,
                &mut self.text_measurer,
                |viewport, views| (self.view)(&self.state, viewport, views),
            )?;
            self.session
                .reconcile_input_hooks(&document, &mut self.hooks, &mut self.state);
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

    fn fail_and_exit(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        phase: &'static str,
        error: Box<dyn Error>,
    ) {
        self.error = Some(NativeRuntimeFailure::from_error(phase, error));
        event_loop.exit();
    }
}

impl<State, Update, View> winit::application::ApplicationHandler
    for NativeWindowApp<State, Update, View>
where
    State: 'static,
    Update: FnMut(&mut State, WidgetAction) + 'static,
    View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
{
    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.clipboard = NativeClipboard::default();
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, (): ()) {
        let device_loss = self
            .device_loss
            .lock()
            .expect("native device loss notification poisoned")
            .take();
        if let Some(error) = device_loss {
            self.fail_and_exit(event_loop, "handling graphics device loss", error.into());
            return;
        }
        if self
            .session
            .apply_task_completions(&mut self.hooks, &mut self.state)
            > 0
        {
            self.request_redraw();
        }
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        if let Err(error) = self.init_window(event_loop) {
            self.fail_and_exit(event_loop, "initializing native window", error);
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
                if self.hooks.close_requested.is_some() {
                    self.session.invalidate_view();
                }
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
            winit::event::WindowEvent::Focused(false) => {
                if let Some(input) = self.text_input.window_unfocused(self.timestamp_millis()) {
                    self.push_input(input);
                }
                if let Some(window) = &self.window {
                    window.set_ime_allowed(false);
                }
                self.buttons = PointerButtons::NONE;
                self.modifiers = KeyModifiers::NONE;
                self.push_input(RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Cancel,
                    self.cursor.unwrap_or(UiPoint::new(0.0, 0.0)),
                    self.timestamp_millis(),
                )));
            }
            winit::event::WindowEvent::Focused(true) => {
                if self.text_input.session.is_some() {
                    if let Some(window) = &self.window {
                        window.set_ime_allowed(true);
                    }
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
                if self
                    .text_input
                    .owns_key(event.physical_key, event.state.is_pressed())
                {
                    return;
                }
                let key = key_code(&event, self.modifiers);
                let timestamp_millis = self.timestamp_millis();
                if let Some(key) = key {
                    let mut raw = match event.state {
                        winit::event::ElementState::Pressed => {
                            RawKeyboardEvent::press(key, self.modifiers, timestamp_millis)
                                .repeat(event.repeat)
                        }
                        winit::event::ElementState::Released => {
                            RawKeyboardEvent::release(key, self.modifiers, timestamp_millis)
                        }
                    };
                    if event.state.is_pressed() {
                        raw.text = event.text.as_ref().map(|text| text.to_string());
                    }
                    self.push_input(RawInputEvent::Keyboard(raw));
                } else if event.state.is_pressed() && !self.modifiers.ctrl && !self.modifiers.meta {
                    if let Some(text) = event
                        .text
                        .as_ref()
                        .and_then(|text| text_input_for_key_event_text(text))
                    {
                        self.push_input(RawInputEvent::Text(RawTextInputEvent::new(
                            text,
                            timestamp_millis,
                        )));
                    }
                }
            }
            winit::event::WindowEvent::Ime(ime) => {
                if let Some(input) = self.text_input.event(&ime, self.timestamp_millis()) {
                    self.push_input(input);
                }
                if matches!(ime, winit::event::Ime::Enabled) {
                    if let (Some(window), Some(session)) = (&self.window, &self.text_input.session)
                    {
                        set_native_ime_cursor_area(
                            window,
                            session.cursor_rect,
                            self.scale_factor(),
                        );
                    }
                }
            }
            winit::event::WindowEvent::RedrawRequested => {
                if let Err(error) = self.render() {
                    self.fail_and_exit(event_loop, "rendering a frame", error);
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
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let winit::event::DeviceEvent::MouseMotion { delta } = event else {
            return;
        };
        let timestamp_millis = self.timestamp_millis();
        if let Some(raw_mouse_motion) = self.hooks.raw_mouse_motion.as_mut() {
            self.session.invalidate_view();
            raw_mouse_motion(
                &mut self.state,
                RawMouseMotion {
                    delta,
                    timestamp_millis,
                    captured_canvas: captured_raw_mouse_canvas(self.session.interaction()),
                },
            );
            // Returning false permits further input handling; it does not mean
            // the callback left application state unchanged.
            self.request_redraw();
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // A past WaitUntil remains installed until explicitly replaced. Reset
        // it even after a deadline was consumed or the window was minimized.
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        if self.viewport().is_none() {
            return;
        }
        let now = Instant::now();
        if let Some(delay) = self
            .session
            .frame_retry_delay(now.duration_since(self.start))
        {
            if let Some(deadline) = now.checked_add(delay) {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(deadline));
            }
            return;
        }
        if self
            .hooks
            .idle_redraw
            .as_ref()
            .is_some_and(|idle_redraw| idle_redraw(&self.state))
        {
            self.session.invalidate_view();
            self.request_redraw();
            return;
        }

        let mut delay = self
            .session
            .next_frame_delay(now.duration_since(self.start));
        if self.options.tick_action.is_some() {
            let interval = self.options.tick_interval.max(Duration::from_millis(1));
            let tick_delay = interval.saturating_sub(now.duration_since(self.last_tick));
            delay = Some(delay.map_or(tick_delay, |delay| delay.min(tick_delay)));
        }
        match delay {
            Some(Duration::ZERO) => self.request_redraw(),
            Some(delay) => {
                if let Some(deadline) = now.checked_add(delay) {
                    event_loop
                        .set_control_flow(winit::event_loop::ControlFlow::WaitUntil(deadline));
                }
            }
            None => {}
        }
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

fn set_native_ime_cursor_area(window: &winit::window::Window, rect: LogicalRect, scale: f32) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    // XIM accepts a baseline spot and winit's X11 backend ignores the area
    // height. Passing the top would place candidates over the draft itself.
    let x11 = window.window_handle().is_ok_and(|handle| {
        matches!(
            handle.as_raw(),
            RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)
        )
    });
    let y = rect.origin.y + if x11 { rect.size.height } else { 0.0 };
    window.set_ime_cursor_area(
        winit::dpi::PhysicalPosition::new((rect.origin.x * scale) as f64, (y * scale) as f64),
        winit::dpi::PhysicalSize::new(
            (rect.size.width * scale) as f64,
            (rect.size.height * scale) as f64,
        ),
    );
}

fn open_native_url(url: &str) -> OpenUrlResponse {
    if url.trim().is_empty() {
        return OpenUrlResponse::Error(PlatformServiceError::new(
            PlatformErrorCode::InvalidRequest,
            "URL is empty",
        ));
    }

    let command = native_open_url_command(url);
    match std::process::Command::new(command.program)
        .args(command.args)
        .status()
    {
        Ok(status) if status.success() => OpenUrlResponse::Opened,
        Ok(status) => OpenUrlResponse::Error(PlatformServiceError::new(
            PlatformErrorCode::Failed,
            format!("open URL command exited with status {status}"),
        )),
        Err(error) => OpenUrlResponse::Error(PlatformServiceError::new(
            PlatformErrorCode::Failed,
            error.to_string(),
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeOpenUrlCommand {
    program: &'static str,
    args: Vec<String>,
}

fn native_open_url_command(url: &str) -> NativeOpenUrlCommand {
    #[cfg(target_os = "windows")]
    {
        NativeOpenUrlCommand {
            program: "cmd",
            args: vec![
                "/C".to_string(),
                "start".to_string(),
                String::new(),
                url.to_string(),
            ],
        }
    }
    #[cfg(target_os = "macos")]
    {
        NativeOpenUrlCommand {
            program: "open",
            args: vec![url.to_string()],
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        NativeOpenUrlCommand {
            program: "xdg-open",
            args: vec![url.to_string()],
        }
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        NativeOpenUrlCommand {
            program: "",
            args: vec![url.to_string()],
        }
    }
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
    fn native_open_url_uses_platform_launcher_and_validates_empty_url() {
        assert!(matches!(
            open_native_url(""),
            OpenUrlResponse::Error(error)
                if error.code == PlatformErrorCode::InvalidRequest
                    && error.message.contains("empty")
        ));

        let command = native_open_url_command("https://example.test");
        #[cfg(target_os = "windows")]
        {
            assert_eq!(command.program, "cmd");
            assert_eq!(command.args[0], "/C");
            assert_eq!(command.args[1], "start");
            assert_eq!(command.args[3], "https://example.test");
        }
        #[cfg(target_os = "macos")]
        {
            assert_eq!(command.program, "open");
            assert_eq!(command.args, vec!["https://example.test".to_string()]);
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            assert_eq!(command.program, "xdg-open");
            assert_eq!(command.args, vec!["https://example.test".to_string()]);
        }
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

        let state = HostInteractionState::default();
        let pointer = RawInputEvent::Pointer(RawPointerEvent::new(
            PointerEventKind::Move,
            UiPoint::new(20.0, 12.0),
            1,
        ));
        let pointer_input = canvas_input_for_raw_event(&document, &state, &pointer).unwrap();
        assert_eq!(pointer_input.node, Some(canvas_id));
        assert_eq!(pointer_input.key, "viewport");
        assert_eq!(pointer_input.local_position, Some(UiPoint::new(20.0, 12.0)));
        assert_eq!(pointer_input.input, pointer);

        let wheel = RawInputEvent::Wheel(RawWheelEvent::pixels(
            UiPoint::new(24.0, 18.0),
            UiPoint::new(0.0, 10.0),
            2,
        ));
        let wheel_input = canvas_input_for_raw_event(&document, &state, &wheel).unwrap();
        assert_eq!(wheel_input.node, Some(canvas_id));
        assert_eq!(wheel_input.local_position, Some(UiPoint::new(24.0, 18.0)));

        let keyboard = RawInputEvent::Keyboard(RawKeyboardEvent::press(
            KeyCode::Character('w'),
            KeyModifiers::NONE,
            3,
        ));
        assert!(canvas_input_for_raw_event(&document, &state, &keyboard).is_none());

        let mut state = HostInteractionState {
            focused: Some(canvas_id),
            ..Default::default()
        };
        let keyboard_input = canvas_input_for_raw_event(&document, &state, &keyboard).unwrap();
        assert_eq!(keyboard_input.node, Some(canvas_id));
        assert_eq!(keyboard_input.local_position, None);

        state.focused = None;
        state.canvas_host_capture.sync([CanvasHostCapturePlan {
            node: canvas_id,
            key: "viewport".to_string(),
            rect: document.node(canvas_id).layout.rect,
            pointer_capture: true,
            keyboard_capture: true,
            wheel_capture: true,
            pointer_lock: false,
            domain_hit_testing: true,
        }]);
        let keyboard_input = canvas_input_for_raw_event(&document, &state, &keyboard).unwrap();
        assert_eq!(keyboard_input.node, Some(canvas_id));

        let mut capture_state = HostInteractionState::default();
        capture_state
            .canvas_host_capture
            .sync([CanvasHostCapturePlan {
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
            Some(CanvasHostCaptureId::new(canvas_id, "viewport"))
        );
    }

    #[test]
    fn native_window_error_includes_application_error_and_frame_timing() {
        let error = NativeWindowRunError::new(
            "showcase",
            "running the platform event loop",
            "ExitFailure(1)",
            Some(NativeRuntimeFailure {
                phase: "rendering a frame",
                message: "Io error: Broken pipe (os error 32)".to_string(),
                report: None,
            }),
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

    #[test]
    fn native_startup_report_includes_operation_consequence_and_next_step() {
        let report = native_startup_report(
            "showcase",
            RuntimeErrorKind::AdapterRequest,
            "requesting a WGPU adapter",
            "No compatible adapter was found",
            "Install a supported GPU driver.",
        );
        let error = NativeWindowRunError::from_report(
            "showcase",
            "initializing native window",
            report,
            None,
            None,
        )
        .to_string();

        assert!(error.contains("native window \"showcase\" failed"));
        assert!(error.contains("requesting a WGPU adapter"));
        assert!(error.contains("user_visible_consequence: the native window did not open"));
        assert!(error.contains("next_step: Install a supported GPU driver."));
        assert!(error.contains("fallback: AbortFrame"));
    }

    #[test]
    fn native_runtime_keeps_running_when_renderer_can_use_cached_frame() {
        assert!(render_error_uses_cached_frame(
            &RenderError::SurfaceUnavailable("surface acquire timed out".to_string())
        ));
        assert!(!render_error_uses_cached_frame(&RenderError::Backend(
            "device lost".to_string()
        )));
        assert!(!render_error_uses_cached_frame(
            &RenderError::UnsupportedTarget(crate::renderer::RenderTargetKind::Snapshot)
        ));
    }
}
