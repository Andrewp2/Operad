//! Browser runtime for WASM/WebGPU applications.
//!
//! This is the web counterpart to the native-window runner: applications
//! provide state, an update function, and a view function, while Operad owns the
//! host loop, input conversion, layout, action dispatch, runtime state
//! persistence, and WGPU surface presentation.

use std::cell::{BorrowMutError, RefCell};
use std::collections::HashMap;
use std::future::{poll_fn, Future};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::time::Duration;

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

// Pointer events expose fractional CSS coordinates. The inherited web-sys
// MouseEvent getters return i32, which can move a hit across a control boundary.
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(extends = web_sys::MouseEvent)]
    type BrowserPointerCoordinates;

    #[wasm_bindgen::prelude::wasm_bindgen(method, getter, structural, js_name = clientX)]
    fn precise_client_x(this: &BrowserPointerCoordinates) -> f64;

    #[wasm_bindgen::prelude::wasm_bindgen(method, getter, structural, js_name = clientY)]
    fn precise_client_y(this: &BrowserPointerCoordinates) -> f64;
}

mod frame_driver;
mod ime;
use super::integration::{RawMouseMotion, RuntimeHooks, RuntimeMetrics, RuntimeObservation};
use frame_driver::WebFrameDriver;

use crate::host::{collect_document_widget_actions, HostDocumentFrameOutput, HostFrameOutput};
use crate::input::{
    PointerId, PointerKind, RawInputEvent, RawKeyboardEvent, RawPointerEvent, RawWheelEvent,
    WheelDeltaUnit, WheelPhase,
};
use crate::platform::{
    BackendCapabilities, ClipboardRequest, ClipboardResponse, CursorGrabMode, CursorRequest,
    CursorResponse, CursorShape, OpenUrlResponse, PixelSize, PlatformErrorCode, PlatformRequest,
    PlatformRequestId, PlatformRequestIdAllocator, PlatformResponse, PlatformServiceError,
    PlatformServiceRequest, PlatformServiceResponse, RepaintRequest, RepaintResponse,
    TextImeRequest, TextImeResponse,
};
use crate::renderer::EmptyResourceResolver;
use crate::renderer::{RenderError, RenderTarget, RendererAdapter};
use crate::wgpu_renderer::WgpuSurfaceRenderer;
use crate::{
    CosmicTextMeasurer, KeyCode, KeyModifiers, PointerButton, PointerButtons, PointerEventKind,
    UiDocument, UiNodeId, UiPoint, UiRect, UiSize, WidgetAction, WidgetActionBinding,
};

#[derive(Debug, Clone)]
pub struct WebRuntimeOptions {
    pub title: String,
    pub canvas_id: String,
    pub status_id: Option<String>,
    pub target_name: String,
    pub ui_scale: f32,
    pub background: String,
    pub install_document_chrome: bool,
    pub tick_action: Option<WidgetActionBinding>,
    pub tick_interval: Duration,
}

impl WebRuntimeOptions {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn with_canvas_id(mut self, canvas_id: impl Into<String>) -> Self {
        self.canvas_id = canvas_id.into();
        self
    }

    pub fn with_status_id(mut self, status_id: impl Into<String>) -> Self {
        self.status_id = Some(status_id.into());
        self
    }

    pub fn without_status(mut self) -> Self {
        self.status_id = None;
        self
    }

    pub fn with_target_name(mut self, target_name: impl Into<String>) -> Self {
        self.target_name = target_name.into();
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

    pub fn with_background(mut self, background: impl Into<String>) -> Self {
        self.background = background.into();
        self
    }

    pub fn with_document_chrome(mut self, install_document_chrome: bool) -> Self {
        self.install_document_chrome = install_document_chrome;
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

    fn tick_interval_ms(&self) -> f64 {
        self.tick_interval.as_secs_f64() * 1000.0
    }
}

impl Default for WebRuntimeOptions {
    fn default() -> Self {
        Self {
            title: "operad".to_string(),
            canvas_id: "operad-canvas".to_string(),
            status_id: Some("operad-status".to_string()),
            target_name: "main".to_string(),
            ui_scale: 1.0,
            background: "#0d1117".to_string(),
            install_document_chrome: true,
            tick_action: None,
            tick_interval: Duration::from_millis(16),
        }
    }
}

pub fn web_runtime_capabilities() -> BackendCapabilities {
    BackendCapabilities::web_runtime()
}

pub async fn run(
    title: impl Into<String>,
    view: impl FnMut(UiSize) -> UiDocument + 'static,
) -> Result<(), JsValue> {
    run_ui_document(title, view).await
}

pub async fn run_ui_document(
    title: impl Into<String>,
    view: impl FnMut(UiSize) -> UiDocument + 'static,
) -> Result<(), JsValue> {
    run_ui_document_with(WebRuntimeOptions::new(title), view).await
}

pub async fn run_ui_document_with(
    options: WebRuntimeOptions,
    mut view: impl FnMut(UiSize) -> UiDocument + 'static,
) -> Result<(), JsValue> {
    run_app_with(
        options,
        (),
        |_state: &mut (), _action: WidgetAction| {},
        move |_state: &(), viewport, _views| view(viewport),
    )
    .await
}

pub async fn run_app<State>(
    title: impl Into<String>,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
) -> Result<(), JsValue>
where
    State: 'static,
{
    run_app_with(WebRuntimeOptions::new(title), state, update, view).await
}

pub async fn run_app_with<State>(
    options: WebRuntimeOptions,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
) -> Result<(), JsValue>
where
    State: 'static,
{
    run_app_with_hooks(options, state, update, view, RuntimeHooks::default()).await
}

pub async fn run_app_with_hooks<State>(
    options: WebRuntimeOptions,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
    hooks: RuntimeHooks<State>,
) -> Result<(), JsValue>
where
    State: 'static,
{
    console_error_panic_hook::set_once();
    if options.install_document_chrome {
        install_document_chrome(&options)?;
    }
    let startup_options = options.clone();
    let canvas = canvas_element(&options.canvas_id)?;
    canvas.set_tab_index(0);
    let _ = canvas.focus();

    let (app, device_lost) =
        match WebRuntimeApp::new(options, state, update, view, hooks, canvas).await {
            Ok((app, device_lost)) => (Rc::new(RefCell::new(app)), device_lost),
            Err(error) => {
                publish_web_startup_error(&startup_options, &error);
                return Err(error);
            }
        };
    register_pointer_events(app.borrow().canvas(), app.clone())?;
    register_wheel_events(app.borrow().canvas(), app.clone())?;
    register_keyboard_events(&browser_window()?, app.clone())?;
    register_window_events(&browser_window()?, app.clone())?;
    {
        let app = app.borrow();
        app.frame_driver.observe_canvas(app.canvas())?;
    }
    install_uat_hooks(app.clone())?;
    // Install after the driver callback exists so startup completions cannot
    // consume their only wakeup before requestAnimationFrame is available.
    frame_driver::start_animation_loop(app.clone())?;
    let driver = Rc::downgrade(&app.borrow().frame_driver);
    app.borrow_mut().hooks.set_task_waker(move || {
        if let Some(driver) = driver.upgrade() {
            driver.wake();
        }
    });
    let app = Rc::downgrade(&app);
    wasm_bindgen_futures::spawn_local(async move {
        let message = device_lost.await;
        let Some(app) = app.upgrade() else {
            return;
        };
        stop_web_runtime(
            &app,
            &web_error(message),
            "Rendering stopped because the graphics device was lost. Reload the page to restart.",
        );
    });
    Ok(())
}

/// Both device loss and terminal frame errors end the same browser lifetime.
/// Call outside an app borrow: DOM ownership releases can reenter event handlers.
fn stop_web_runtime<State, Update, View>(
    app: &RefCell<WebRuntimeApp<State, Update, View>>,
    error: &JsValue,
    status_message: &str,
) {
    let (canvas, pointers, locked, status_id) = {
        let mut app = app.borrow_mut();
        if app.frame_driver.is_stopped() {
            return;
        }
        app.frame_driver.stop();
        // Drop task receivers and inactive callbacks, retaining only the
        // application's unsaved-change protection for page close.
        let close_requested = app.hooks.close_requested.take();
        app.hooks = RuntimeHooks::default();
        app.hooks.close_requested = close_requested;
        app.pending_input.borrow_mut().clear();
        app.pending_platform_responses.clear();
        app.async_platform_responses.borrow_mut().clear();
        app.pointer_lock_requests.clear();
        app.pointer_lock_desired = false;
        app.text_input.shutdown();
        let locked = app.pointer_locked();
        let pointers = app
            .active_pointers
            .drain()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        (
            app.canvas.clone(),
            pointers,
            locked,
            app.options.status_id.clone(),
        )
    };
    // Releasing browser ownership can synchronously dispatch input events.
    for pointer in pointers {
        if canvas.has_pointer_capture(pointer) {
            let _ = canvas.release_pointer_capture(pointer);
        }
    }
    if locked {
        if let Ok(document) = browser_document() {
            document.exit_pointer_lock();
        }
    }
    let _ = canvas.blur();
    let _ = canvas.style().set_property("cursor", "default");
    let _ = canvas.style().set_property("touch-action", "auto");
    web_sys::console::error_1(error);
    if let Some(status_id) = status_id {
        set_status(&status_id, status_message);
    }
}

fn stop_web_frame_error<State, Update, View>(
    app: &RefCell<WebRuntimeApp<State, Update, View>>,
    error: &JsValue,
) {
    stop_web_runtime(
        app,
        error,
        &format!(
            "Rendering stopped: {}. Reload the page to restart.",
            web_message(error)
        ),
    );
}

// Keep the renderer's recovery contract until the frame driver decides whether
// another frame can make progress. JavaScript/browser and layout errors stop.
enum WebFrameError {
    Browser(JsValue),
    Renderer(RenderError),
}

impl WebFrameError {
    fn can_retry(&self) -> bool {
        matches!(self, Self::Renderer(RenderError::SurfaceUnavailable(_)))
    }

    fn into_js(self) -> JsValue {
        match self {
            Self::Browser(error) => error,
            Self::Renderer(error) => web_error(format!("render failed: {error}")),
        }
    }
}

impl From<JsValue> for WebFrameError {
    fn from(error: JsValue) -> Self {
        Self::Browser(error)
    }
}

impl From<RenderError> for WebFrameError {
    fn from(error: RenderError) -> Self {
        Self::Renderer(error)
    }
}

fn device_loss_notification(device: &wgpu::Device) -> impl Future<Output = String> + 'static {
    // wgpu requires a Send callback, even on the browser's UI thread. Pass only
    // owned data and a task waker across that boundary, never the Rc-owned app.
    let notification = Arc::new(Mutex::new((None, None::<Waker>)));
    let callback_notification = notification.clone();
    device.set_device_lost_callback(move |reason, message| {
        let wake = {
            let mut notification = callback_notification
                .lock()
                .expect("device loss notification poisoned");
            notification.0 = Some(format!("WebGPU device lost ({reason:?}): {message}"));
            notification.1.take()
        };
        if let Some(wake) = wake {
            wake.wake();
        }
    });
    poll_fn(move |cx| {
        let mut notification = notification
            .lock()
            .expect("device loss notification poisoned");
        if let Some(message) = notification.0.take() {
            Poll::Ready(message)
        } else {
            notification.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    })
}

struct WebRuntimeApp<State, Update, View> {
    options: WebRuntimeOptions,
    state: State,
    update: Update,
    view: View,
    hooks: RuntimeHooks<State>,
    session: super::session::RuntimeSession,
    platform_request_ids: PlatformRequestIdAllocator,
    pending_platform_responses: Vec<PlatformServiceResponse>,
    async_platform_responses: Rc<RefCell<Vec<PlatformServiceResponse>>>,
    renderer: WgpuSurfaceRenderer<'static>,
    canvas: web_sys::HtmlCanvasElement,
    text_measurer: CosmicTextMeasurer,
    pending_input: Rc<RefCell<Vec<RawInputEvent>>>,
    text_input: ime::WebTextInput,
    cursor: Option<UiPoint>,
    active_pointers: HashMap<i32, RawPointerEvent>,
    pointer_lock_requests: Vec<PlatformRequestId>,
    pointer_lock_desired: bool,
    modifiers: KeyModifiers,
    dpi_scale: f32,
    scale_factor: f32,
    last_metrics: Option<RuntimeMetrics>,
    last_tick_ms: Option<f64>,
    last_animation_ms: Option<f64>,
    frame_driver: Rc<WebFrameDriver>,
}

impl<State, Update, View> WebRuntimeApp<State, Update, View> {
    async fn new(
        options: WebRuntimeOptions,
        state: State,
        update: Update,
        view: View,
        hooks: RuntimeHooks<State>,
        canvas: web_sys::HtmlCanvasElement,
    ) -> Result<(Self, impl Future<Output = String> + 'static), JsValue> {
        let (_viewport, pixel_size, dpi_scale) = canvas_metrics(&canvas)?;
        canvas.set_width(pixel_size.width);
        canvas.set_height(pixel_size.height);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|error| {
                web_startup_error(
                    "creating the WebGPU surface",
                    error,
                    "Use a browser with WebGPU enabled and confirm the canvas element can create a GPU surface.",
                )
            })?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|error| {
                web_startup_error(
                    "requesting a WebGPU adapter",
                    error,
                    "Enable WebGPU in the browser and use a GPU/driver combination supported by wgpu.",
                )
            })?;
        let adapter_features = adapter.features();
        let required_features = if adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            wgpu::Features::TIMESTAMP_QUERY
        } else {
            wgpu::Features::empty()
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("operad-web-device"),
                required_features,
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(|error| {
                web_startup_error(
                    "requesting a WebGPU device",
                    error,
                    "Lower required WebGPU features or update the browser and GPU driver.",
                )
            })?;
        let device_lost = device_loss_notification(&device);
        let surface_config = surface
            .get_default_config(&adapter, pixel_size.width, pixel_size.height)
            .ok_or_else(|| {
                web_startup_error(
                    "selecting the WebGPU surface configuration",
                    "no compatible surface configuration was reported",
                    "Use a browser with WebGPU enabled and a compatible GPU, or try another browser channel.",
                )
            })?;
        let renderer =
            WgpuSurfaceRenderer::new(surface, device, queue, surface_config).map_err(|error| {
                web_startup_error(
                    "initializing the WebGPU renderer",
                    error,
                    "Check surface format support and renderer initialization logs.",
                )
            })?;

        let frame_driver = Rc::new(WebFrameDriver::new());
        let pending_input = Rc::new(RefCell::new(Vec::new()));
        let text_input = ime::WebTextInput::new(&canvas, pending_input.clone(), &frame_driver);
        Ok((
            Self {
                options,
                state,
                update,
                view,
                hooks,
                session: super::session::RuntimeSession::new(),
                platform_request_ids: PlatformRequestIdAllocator::new(1),
                pending_platform_responses: Vec::new(),
                async_platform_responses: Rc::new(RefCell::new(Vec::new())),
                renderer,
                canvas,
                text_measurer: CosmicTextMeasurer::new(),
                pending_input,
                text_input,
                cursor: None,
                active_pointers: HashMap::new(),
                pointer_lock_requests: Vec::new(),
                pointer_lock_desired: false,
                modifiers: KeyModifiers::NONE,
                dpi_scale,
                scale_factor: dpi_scale,
                last_metrics: None,
                last_tick_ms: None,
                last_animation_ms: None,
                frame_driver,
            },
            device_lost,
        ))
    }

    fn canvas(&self) -> &web_sys::HtmlCanvasElement {
        &self.canvas
    }

    // Shared by scheduled frames and synchronous composition-key flushing.
    // Only temporary surface acquisition failures can make progress by retrying.
    fn render(&mut self) -> Result<(), JsValue>
    where
        Update: FnMut(&mut State, WidgetAction),
        View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument,
    {
        if let Err(error) = self.render_frame() {
            self.session.frame_failed(self.frame_driver.now());
            let can_retry = error.can_retry();
            let error = error.into_js();
            if !can_retry {
                return Err(error);
            }
            web_sys::console::error_1(&error);
            if let Some(status_id) = self.options.status_id.as_deref() {
                set_status(
                    status_id,
                    &format!("Render failed: {}", web_message(&error)),
                );
            }
        }
        Ok(())
    }

    fn render_frame(&mut self) -> Result<(), WebFrameError>
    where
        Update: FnMut(&mut State, WidgetAction),
        View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument,
    {
        if self.frame_driver.is_stopped() {
            return Ok(());
        }
        self.session
            .apply_task_completions(&mut self.hooks, &mut self.state);
        let (dpi_viewport, pixel_size, dpi_scale) = canvas_metrics(&self.canvas)?;
        let elapsed = self.frame_driver.now();
        // Scheduled and synchronous input frames share the runtime's origin.
        // Page-relative timestamps would advance tick/animation state into the
        // future when an embedded runtime starts after the page loads.
        let timestamp_ms = elapsed.as_secs_f64() * 1000.0;
        self.session.begin_frame(elapsed);
        let dpi_metrics = RuntimeMetrics {
            physical_size: pixel_size,
            viewport: dpi_viewport,
            scale_factor: dpi_scale,
            dpi_scale,
            elapsed,
        };
        let scale_factor = self
            .hooks
            .scale_factor
            .as_ref()
            .map(|hook| normalized_web_scale(hook(&self.state, dpi_metrics)))
            .unwrap_or(dpi_scale);
        let viewport = UiSize::new(
            pixel_size.width as f32 / scale_factor,
            pixel_size.height as f32 / scale_factor,
        );
        if self.canvas.width() != pixel_size.width {
            self.canvas.set_width(pixel_size.width);
        }
        if self.canvas.height() != pixel_size.height {
            self.canvas.set_height(pixel_size.height);
        }
        self.dpi_scale = dpi_scale;
        self.scale_factor = scale_factor;

        let metrics = RuntimeMetrics {
            physical_size: pixel_size,
            viewport,
            scale_factor,
            dpi_scale,
            elapsed,
        };
        self.session
            .apply_before_render(&mut self.hooks, &mut self.state, metrics);
        if let Some(title) = self.hooks.title.as_mut() {
            if let Ok(document) = browser_document() {
                document.set_title(&title(&self.state));
            }
        }
        self.drain_async_platform_responses();
        self.dispatch_tick(timestamp_ms);
        self.apply_hook_platform_requests(metrics);
        let animation_dt = self.animation_delta_seconds(timestamp_ms);

        let mut document = self.build_document(viewport).map_err(layout_web_error)?;
        document.tick_animations(animation_dt);
        let raw_input = std::mem::take(&mut *self.pending_input.borrow_mut());
        let responses = std::mem::take(&mut self.pending_platform_responses);
        let host_output = self
            .session
            .process_input_with_hooks(
                &mut document,
                viewport,
                &raw_input,
                &responses,
                &mut self.hooks,
                &mut self.state,
                &mut self.text_measurer,
            )
            .map_err(layout_web_error)?;
        let frame = self
            .session
            .finish_frame(
                &mut document,
                viewport,
                RenderTarget::window(self.options.target_name.clone(), viewport),
                host_output,
                &mut self.text_measurer,
                &mut self.platform_request_ids,
            )
            .map_err(layout_web_error)?;
        let actions = collect_document_widget_actions(&frame);
        self.apply_platform_service_requests(&frame);

        let frame = if actions.is_empty() && !self.session.view_needs_rebuild() {
            frame
        } else {
            for action in actions {
                (self.update)(&mut self.state, action);
                self.session.invalidate_view();
            }
            self.apply_hook_platform_requests(metrics);
            self.session.retain_document(document);
            document = self.build_document(viewport).map_err(layout_web_error)?;
            let frame = self
                .session
                .finish_frame(
                    &mut document,
                    viewport,
                    RenderTarget::window(self.options.target_name.clone(), viewport),
                    HostFrameOutput::new(self.session.interaction().clone()),
                    &mut self.text_measurer,
                    &mut self.platform_request_ids,
                )
                .map_err(layout_web_error)?;
            self.apply_platform_service_requests(&frame);
            frame
        };

        if self.session.view_needs_rebuild() {
            // A response to the second document pass may update application
            // state again. Defer that work without losing the wakeup.
            let now = self.frame_driver.now();
            self.session.request_repaint(now, RepaintRequest::NextFrame);
        }
        if let Some(session) = self.session.interaction().text_ime.as_ref() {
            self.text_input
                .sync(session, &self.canvas, self.scale_factor / self.dpi_scale);
        }
        self.hooks.observe(
            &self.state,
            RuntimeObservation::new(metrics, &document, &frame, self.session.view_build_stats()),
        );
        self.last_metrics = Some(metrics);
        self.session.retain_document(document);
        // Composition-key flushing still prepares current surrounding text while
        // the surface is unavailable, but must not attempt GPU work early.
        if self
            .session
            .frame_retry_delay(self.frame_driver.now())
            .is_some()
        {
            self.session.frame_deferred();
            return Ok(());
        }
        self.renderer
            .render_frame(frame.render_request, &EmptyResourceResolver)?;
        self.session.frame_presented();
        Ok(())
    }

    fn build_document(&mut self, viewport: UiSize) -> Result<UiDocument, taffy::TaffyError>
    where
        View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument,
    {
        let scale = crate::UiDocumentScale::new(self.options.ui_scale, self.scale_factor);
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
            // A removed or disabled owner can cancel an application edit while
            // preparing this document. Include the resulting state in this frame.
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

    fn uat_snapshot(&self) -> Result<JsValue, JsValue> {
        let document = self
            .session
            .document()
            .ok_or_else(|| web_error("runtime has not computed its first frame"))?;
        let metrics = self
            .last_metrics
            .ok_or_else(|| web_error("runtime has not computed its first frame"))?;
        let viewport = metrics.viewport;
        let pixel_size = metrics.physical_size;
        let dpi_scale = metrics.dpi_scale;

        let snapshot = Object::new();
        set_js_string(&snapshot, "target", &self.options.target_name)?;
        set_js_number(&snapshot, "nodeCount", document.node_count() as f64)?;
        set_js_number(&snapshot, "dpiScale", dpi_scale as f64)?;
        set_js_number(&snapshot, "scaleFactor", metrics.scale_factor as f64)?;
        set_js_object(&snapshot, "viewport", size_js_object(viewport)?.as_ref())?;
        let pixel = Object::new();
        set_js_number(&pixel, "width", pixel_size.width as f64)?;
        set_js_number(&pixel, "height", pixel_size.height as f64)?;
        set_js_object(&snapshot, "pixelSize", pixel.as_ref())?;

        let focus = Object::new();
        set_optional_node_name(&focus, "hovered", &document, document.focus_state().hovered)?;
        set_optional_node_name(&focus, "focused", &document, document.focus_state().focused)?;
        set_optional_node_name(&focus, "pressed", &document, document.focus_state().pressed)?;
        set_js_object(&snapshot, "focus", focus.as_ref())?;

        let nodes = Array::new();
        for (index, node) in document.nodes().iter().enumerate() {
            let item = Object::new();
            set_js_number(&item, "index", index as f64)?;
            set_js_string(&item, "name", node.name())?;
            set_js_bool(&item, "visible", node.layout().visible)?;
            set_js_object(&item, "rect", rect_js_object(node.layout().rect)?.as_ref())?;
            set_js_object(
                &item,
                "clipRect",
                rect_js_object(node.layout().clip_rect)?.as_ref(),
            )?;
            let input = node.input();
            set_js_bool(&item, "pointer", input.pointer)?;
            set_js_bool(&item, "focusable", input.focusable)?;
            set_js_bool(&item, "keyboard", input.keyboard)?;
            set_js_bool(&item, "autoScrollbar", node.has_auto_scrollbar())?;
            if let Some(action) = node.action() {
                if let Some(id) = action.action_id() {
                    set_js_string(&item, "action", id.as_str())?;
                }
                if let Some(id) = action.command_id() {
                    set_js_string(&item, "command", id.as_str())?;
                }
            }
            if let Some(scroll) = node.scroll() {
                set_js_object(&item, "scroll", scroll_js_object(scroll)?.as_ref())?;
            }
            if let Some(meta) = node.accessibility() {
                let accessibility = Object::new();
                set_js_string(&accessibility, "role", &format!("{:?}", meta.role))?;
                if let Some(label) = meta.label.as_deref() {
                    set_js_string(&accessibility, "label", label)?;
                }
                if let Some(value) = meta.value.as_deref() {
                    set_js_string(&accessibility, "value", value)?;
                }
                set_js_bool(
                    &accessibility,
                    "enabled",
                    document.node_is_enabled(UiNodeId(index)),
                )?;
                set_js_bool(&accessibility, "focusable", meta.focusable)?;
                set_js_object(&item, "accessibility", accessibility.as_ref())?;
            }
            nodes.push(&item);
        }
        set_js_array(&snapshot, "nodes", &nodes)?;

        let warnings = Array::new();
        for warning in document.audit_layout() {
            warnings.push(&JsValue::from_str(&format!("{warning:?}")));
        }
        set_js_array(&snapshot, "warnings", &warnings)?;

        Ok(snapshot.into())
    }

    fn dispatch_tick(&mut self, timestamp_ms: f64)
    where
        Update: FnMut(&mut State, WidgetAction),
    {
        let Some(action) = self.options.tick_action.clone() else {
            return;
        };
        let interval = self.options.tick_interval_ms().max(1.0);
        let mut last_tick = self.last_tick_ms.unwrap_or(timestamp_ms);
        let mut ticks = 0;
        while timestamp_ms - last_tick >= interval && ticks < 4 {
            self.session.invalidate_view();
            (self.update)(
                &mut self.state,
                WidgetAction::activate(UiNodeId(0), action.clone()),
            );
            last_tick += interval;
            ticks += 1;
        }
        // Do not spend subsequent frames catching up an unbounded background-tab backlog.
        if timestamp_ms - last_tick >= interval {
            last_tick = timestamp_ms;
        }
        self.last_tick_ms = Some(last_tick);
    }

    fn animation_delta_seconds(&mut self, timestamp_ms: f64) -> f32 {
        let dt = self
            .last_animation_ms
            .map(|last| ((timestamp_ms - last) / 1000.0).max(0.0))
            .unwrap_or(0.0);
        self.last_animation_ms = Some(timestamp_ms);
        (dt as f32).clamp(0.0, 0.1)
    }

    fn timestamp_millis(&self) -> u64 {
        self.frame_driver
            .now()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }

    fn push_pointer(&mut self, event: &web_sys::PointerEvent, mut kind: PointerEventKind) {
        self.modifiers = pointer_modifiers(event);
        let buttons = web_pointer_buttons(event.buttons());
        // Browsers report a chord's intermediate button changes as pointermove.
        // Preserve native-style button events so releasing the captured button
        // ends its edit even while another mouse button remains held.
        if kind == PointerEventKind::Move && event.button() >= 0 {
            if let Some(previous) = self.active_pointers.get(&event.pointer_id()) {
                let button = pointer_button(event.button());
                if previous.buttons.contains(button) != buttons.contains(button) {
                    kind = if buttons.contains(button) {
                        PointerEventKind::Down(button)
                    } else {
                        PointerEventKind::Up(button)
                    };
                }
            }
        }
        let coordinates: &BrowserPointerCoordinates = event.unchecked_ref();
        let point = self.input_position(
            coordinates.precise_client_x(),
            coordinates.precise_client_y(),
        );
        self.cursor = Some(point);
        let raw = RawPointerEvent::new(kind, point, self.timestamp_millis())
            .pointer_id(PointerId::new(event.pointer_id() as u64))
            .pointer_kind(match event.pointer_type().as_str() {
                "mouse" => PointerKind::Mouse,
                "touch" => PointerKind::Touch,
                "pen" => PointerKind::Pen,
                _ => PointerKind::Unknown,
            })
            .buttons(buttons)
            .modifiers(self.modifiers);
        match kind {
            PointerEventKind::Down(_) => {
                self.active_pointers.insert(event.pointer_id(), raw);
            }
            PointerEventKind::Move => {
                if let Some(active) = self.active_pointers.get_mut(&event.pointer_id()) {
                    *active = raw;
                }
            }
            PointerEventKind::Up(_) if buttons != PointerButtons::NONE => {
                self.active_pointers.insert(event.pointer_id(), raw);
            }
            PointerEventKind::Up(_) | PointerEventKind::Cancel => {
                self.active_pointers.remove(&event.pointer_id());
            }
        }
        self.pending_input
            .borrow_mut()
            .push(RawInputEvent::Pointer(raw));
    }

    fn cancel_pointer(&mut self, pointer_id: i32, timestamp_millis: u64) {
        let Some(mut pointer) = self.active_pointers.remove(&pointer_id) else {
            // Browsers release DOM capture after pointerup/cancel. That terminal
            // event already ended the edit and must not produce a second cancel.
            return;
        };
        pointer.kind = PointerEventKind::Cancel;
        pointer.buttons = PointerButtons::NONE;
        pointer.timestamp_millis = timestamp_millis;
        self.pending_input
            .borrow_mut()
            .push(RawInputEvent::Pointer(pointer));
    }

    fn cancel_pointers(&mut self, timestamp_millis: u64) -> Vec<i32> {
        let pointers = self.active_pointers.keys().copied().collect::<Vec<_>>();
        for pointer_id in &pointers {
            self.cancel_pointer(*pointer_id, timestamp_millis);
        }
        self.modifiers = KeyModifiers::NONE;
        pointers
    }

    fn input_position(&self, client_x: f64, client_y: f64) -> UiPoint {
        let point = pointer_position(&self.canvas, client_x, client_y);
        let css_to_ui = self.dpi_scale / self.scale_factor;
        UiPoint::new(point.x * css_to_ui, point.y * css_to_ui)
    }

    fn pointer_locked(&self) -> bool {
        browser_document()
            .ok()
            .and_then(|document| document.pointer_lock_element())
            .is_some_and(|element| element == self.canvas.clone().into())
    }

    fn push_raw_mouse_motion(&mut self, event: &web_sys::MouseEvent) {
        if !self.pointer_locked() {
            return;
        }
        let timestamp_millis = self.timestamp_millis();
        let Some(hook) = self.hooks.raw_mouse_motion.as_mut() else {
            return;
        };
        let motion = RawMouseMotion {
            delta: (event.movement_x() as f64, event.movement_y() as f64),
            timestamp_millis,
            captured_canvas: super::integration::input::captured_raw_mouse_canvas(
                self.session.interaction(),
            ),
        };
        self.session.invalidate_view();
        hook(&mut self.state, motion);
    }

    fn push_wheel(&mut self, event: web_sys::WheelEvent) {
        self.modifiers = wheel_modifiers(&event);
        let coordinates: &BrowserPointerCoordinates = event.unchecked_ref();
        let point = self.input_position(
            coordinates.precise_client_x(),
            coordinates.precise_client_y(),
        );
        self.cursor = Some(point);
        let (mut delta, unit) = wheel_delta(&event);
        if unit == WheelDeltaUnit::Pixel {
            let css_to_ui = self.dpi_scale / self.scale_factor;
            delta = UiPoint::new(delta.x * css_to_ui, delta.y * css_to_ui);
        }
        self.pending_input
            .borrow_mut()
            .push(RawInputEvent::Wheel(RawWheelEvent {
                position: point,
                delta,
                unit,
                phase: WheelPhase::Moved,
                modifiers: self.modifiers,
                timestamp_millis: self.timestamp_millis(),
            }));
    }

    fn push_key(&mut self, event: web_sys::KeyboardEvent, pressed: bool) {
        self.modifiers = key_modifiers(&event);
        let Some(key) = key_code(&event) else {
            return;
        };
        let timestamp = self.timestamp_millis();
        let mut raw = if pressed {
            RawKeyboardEvent::press(key, self.modifiers, timestamp).repeat(event.repeat())
        } else {
            RawKeyboardEvent::release(key, self.modifiers, timestamp)
        };
        if pressed {
            raw.text = text_input_for_key(&event);
        }
        self.pending_input
            .borrow_mut()
            .push(RawInputEvent::Keyboard(raw));
    }

    fn apply_platform_service_requests(&mut self, frame: &HostDocumentFrameOutput) {
        let requests = frame.platform_service_requests(&mut self.platform_request_ids);
        if requests.is_empty() {
            return;
        }
        let responses = requests
            .into_iter()
            .filter_map(|request| self.apply_platform_service_request(request))
            .collect::<Vec<_>>();
        self.dispatch_platform_responses(&responses);
        self.pending_platform_responses.extend(responses);
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
            .filter_map(|request| self.apply_platform_service_request(request))
            .collect::<Vec<_>>();
        self.dispatch_platform_responses(&responses);
        self.pending_platform_responses.extend(responses);
    }

    fn apply_platform_service_request(
        &mut self,
        request: PlatformServiceRequest,
    ) -> Option<PlatformServiceResponse> {
        let PlatformServiceRequest { id, request } = request;
        match request {
            PlatformRequest::Clipboard(request) => self.apply_web_clipboard_request(id, request),
            PlatformRequest::Cursor(CursorRequest::SetGrab(CursorGrabMode::Locked)) => {
                self.pointer_lock_desired = true;
                if self.pointer_locked() {
                    Some(PlatformServiceResponse::new(
                        id,
                        PlatformResponse::Cursor(CursorResponse::Applied),
                    ))
                } else {
                    self.pointer_lock_requests.push(id);
                    if self.pointer_lock_requests.len() == 1 {
                        self.canvas.request_pointer_lock();
                    }
                    None
                }
            }
            request => Some(PlatformServiceResponse::new(
                id,
                self.apply_platform_request(request),
            )),
        }
    }

    fn apply_web_clipboard_request(
        &mut self,
        id: PlatformRequestId,
        request: ClipboardRequest,
    ) -> Option<PlatformServiceResponse> {
        match request {
            ClipboardRequest::ReadText => {
                let responses = self.async_platform_responses.clone();
                let driver = self.frame_driver.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let response = match web_clipboard_read_text().await {
                        Ok(text) => ClipboardResponse::Text(text),
                        Err(error) => web_clipboard_error(error),
                    };
                    push_async_platform_response(
                        &responses,
                        &driver,
                        PlatformServiceResponse::new(id, PlatformResponse::Clipboard(response)),
                        "clipboard read response",
                    );
                });
                None
            }
            ClipboardRequest::WriteText(text) => {
                let responses = self.async_platform_responses.clone();
                let driver = self.frame_driver.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let response = match web_clipboard_write_text(&text).await {
                        Ok(()) => ClipboardResponse::Completed,
                        Err(error) => web_clipboard_error(error),
                    };
                    push_async_platform_response(
                        &responses,
                        &driver,
                        PlatformServiceResponse::new(id, PlatformResponse::Clipboard(response)),
                        "clipboard write response",
                    );
                });
                None
            }
            ClipboardRequest::Clear => {
                let responses = self.async_platform_responses.clone();
                let driver = self.frame_driver.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let response = match web_clipboard_write_text("").await {
                        Ok(()) => ClipboardResponse::Completed,
                        Err(error) => web_clipboard_error(error),
                    };
                    push_async_platform_response(
                        &responses,
                        &driver,
                        PlatformServiceResponse::new(id, PlatformResponse::Clipboard(response)),
                        "clipboard clear response",
                    );
                });
                None
            }
            ClipboardRequest::ReadFiles | ClipboardRequest::WriteFiles(_) => {
                Some(PlatformServiceResponse::new(
                    id,
                    PlatformResponse::Clipboard(ClipboardResponse::Unsupported),
                ))
            }
        }
    }

    fn drain_async_platform_responses(&mut self) {
        let responses = match self.async_platform_responses.try_borrow_mut() {
            Ok(mut responses) => responses.drain(..).collect::<Vec<_>>(),
            Err(error) => {
                log_web_runtime_reentry("async platform response drain", &error);
                return;
            }
        };
        self.dispatch_platform_responses(&responses);
        self.pending_platform_responses.extend(responses);
    }

    fn dispatch_platform_responses(&mut self, responses: &[PlatformServiceResponse]) {
        self.session
            .apply_platform_responses(&mut self.hooks, &mut self.state, responses);
    }

    fn apply_platform_request(&mut self, request: PlatformRequest) -> PlatformResponse {
        if let PlatformRequest::TextIme(request) = &request {
            self.session.apply_text_ime_request(request);
        }
        match request {
            PlatformRequest::OpenUrl(request) => {
                let target = if request.new_window {
                    "_blank"
                } else {
                    "_self"
                };
                match browser_window()
                    .and_then(|window| window.open_with_url_and_target(&request.url, target))
                {
                    Ok(Some(_)) => PlatformResponse::OpenUrl(OpenUrlResponse::Opened),
                    Ok(None) => PlatformResponse::OpenUrl(OpenUrlResponse::Blocked),
                    Err(error) => PlatformResponse::OpenUrl(OpenUrlResponse::Error(
                        PlatformServiceError::new(PlatformErrorCode::Failed, web_message(&error)),
                    )),
                }
            }
            PlatformRequest::TextIme(request) => PlatformResponse::TextIme(match request {
                TextImeRequest::Activate(session) | TextImeRequest::Update(session) => {
                    self.text_input.sync(
                        &session,
                        &self.canvas,
                        self.scale_factor / self.dpi_scale,
                    );
                    TextImeResponse::Activated {
                        input: session.input,
                    }
                }
                TextImeRequest::Deactivate { input } | TextImeRequest::HideKeyboard { input } => {
                    self.text_input.deactivate(&input);
                    TextImeResponse::Deactivated { input }
                }
                TextImeRequest::ShowKeyboard { input } => {
                    if self
                        .session
                        .interaction()
                        .text_ime
                        .as_ref()
                        .is_some_and(|session| session.input == input)
                    {
                        self.text_input.focus();
                    }
                    TextImeResponse::Activated { input }
                }
            }),
            PlatformRequest::Cursor(request) => {
                PlatformResponse::Cursor(self.apply_cursor_request(request))
            }
            PlatformRequest::Repaint(request) => {
                PlatformResponse::Repaint(self.apply_repaint_request(request))
            }
            request => PlatformResponse::unsupported(request.kind()),
        }
    }

    fn apply_cursor_request(&mut self, request: CursorRequest) -> CursorResponse {
        let style = self.canvas.style();
        match request {
            CursorRequest::SetShape(shape) => style
                .set_property("cursor", css_cursor(shape))
                .map(|_| CursorResponse::Applied)
                .unwrap_or_else(cursor_error),
            CursorRequest::SetVisible(visible) => {
                let cursor = if visible { "auto" } else { "none" };
                style
                    .set_property("cursor", cursor)
                    .map(|_| CursorResponse::Applied)
                    .unwrap_or_else(cursor_error)
            }
            // Lock requests are asynchronous and handled with their request ID.
            CursorRequest::SetGrab(CursorGrabMode::Locked) => CursorResponse::Unsupported,
            CursorRequest::SetGrab(CursorGrabMode::None) => {
                self.pointer_lock_desired = false;
                self.complete_pointer_lock_requests(CursorResponse::Error(
                    PlatformServiceError::new(
                        PlatformErrorCode::Failed,
                        "pointer lock request was cancelled",
                    ),
                ));
                match browser_document() {
                    Ok(document) => {
                        document.exit_pointer_lock();
                        CursorResponse::Applied
                    }
                    Err(error) => cursor_error(error),
                }
            }
            CursorRequest::SetPosition(_)
            | CursorRequest::SetGrab(CursorGrabMode::Confined)
            | CursorRequest::Confine(_)
            | CursorRequest::ReleaseConfine => CursorResponse::Unsupported,
        }
    }

    fn complete_pointer_lock_requests(&mut self, response: CursorResponse) {
        let responses = std::mem::take(&mut self.pointer_lock_requests)
            .into_iter()
            .map(|id| PlatformServiceResponse::new(id, PlatformResponse::Cursor(response.clone())))
            .collect::<Vec<_>>();
        self.dispatch_platform_responses(&responses);
        self.pending_platform_responses.extend(responses);
    }

    fn apply_repaint_request(&mut self, request: RepaintRequest) -> RepaintResponse {
        self.session
            .request_repaint(self.frame_driver.now(), request)
    }

    fn next_frame_delay(&mut self) -> Option<Duration> {
        let now = self.frame_driver.now();
        if let Some(delay) = self.session.frame_retry_delay(now) {
            return Some(delay);
        }
        let repaint = self.session.next_frame_delay(now);
        let tick = self.options.tick_action.as_ref().map(|_| {
            let interval = self.options.tick_interval_ms().max(1.0);
            let elapsed = now.as_secs_f64() * 1000.0;
            let deadline = self.last_tick_ms.unwrap_or(elapsed) + interval;
            Duration::from_secs_f64(((deadline - elapsed) / 1000.0).max(0.0))
        });
        let idle = if self
            .hooks
            .idle_redraw
            .as_ref()
            .is_some_and(|hook| hook(&self.state))
        {
            // Match the native host: explicitly requested idle frames refresh
            // views that read time or other application-owned external state.
            self.session.invalidate_view();
            Some(Duration::ZERO)
        } else {
            None
        };
        repaint.into_iter().chain(tick).chain(idle).min()
    }
}

fn install_uat_hooks<State, Update, View>(
    app: Rc<RefCell<WebRuntimeApp<State, Update, View>>>,
) -> Result<(), JsValue>
where
    State: 'static,
    Update: 'static,
    View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
{
    if !web_uat_hooks_enabled() {
        return Ok(());
    }
    let window = browser_window()?;
    let api = Object::new();
    let snapshot_app = app.clone();
    let snapshot = Closure::<dyn FnMut() -> JsValue>::wrap(Box::new(move || {
        match snapshot_app.try_borrow() {
            Ok(app) => app
                .uat_snapshot()
                .unwrap_or_else(|error| uat_error_value("snapshot", error)),
            Err(error) => uat_error_value(
                "snapshot",
                JsValue::from_str(&format!(
                    "runtime state was already borrowed while collecting UAT snapshot: {error}"
                )),
            ),
        }
    }));
    Reflect::set(
        api.as_ref(),
        &JsValue::from_str("snapshot"),
        snapshot.as_ref(),
    )?;
    snapshot.forget();
    Reflect::set(
        window.as_ref(),
        &JsValue::from_str("__OPERAD_UAT__"),
        api.as_ref(),
    )?;
    Ok(())
}

fn web_uat_hooks_enabled() -> bool {
    let Ok(window) = browser_window() else {
        return false;
    };
    Reflect::get(window.as_ref(), &JsValue::from_str("__OPERAD_ENABLE_UAT__"))
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn uat_error_value(context: &str, error: JsValue) -> JsValue {
    let object = Object::new();
    let _ = set_js_string(&object, "error", context);
    let _ = set_js_string(&object, "message", &web_message(&error));
    object.into()
}

fn set_optional_node_name(
    object: &Object,
    name: &str,
    document: &UiDocument,
    node: Option<UiNodeId>,
) -> Result<(), JsValue> {
    if let Some(node) = node {
        if node.index() < document.node_count() {
            set_js_string(object, name, document.node(node).name())?;
            return Ok(());
        }
    }
    Reflect::set(object.as_ref(), &JsValue::from_str(name), &JsValue::NULL)?;
    Ok(())
}

fn rect_js_object(rect: UiRect) -> Result<Object, JsValue> {
    let object = Object::new();
    set_js_number(&object, "x", rect.x as f64)?;
    set_js_number(&object, "y", rect.y as f64)?;
    set_js_number(&object, "width", rect.width as f64)?;
    set_js_number(&object, "height", rect.height as f64)?;
    set_js_number(&object, "right", rect.right() as f64)?;
    set_js_number(&object, "bottom", rect.bottom() as f64)?;
    Ok(object)
}

fn size_js_object(size: UiSize) -> Result<Object, JsValue> {
    let object = Object::new();
    set_js_number(&object, "width", size.width as f64)?;
    set_js_number(&object, "height", size.height as f64)?;
    Ok(object)
}

fn point_js_object(point: UiPoint) -> Result<Object, JsValue> {
    let object = Object::new();
    set_js_number(&object, "x", point.x as f64)?;
    set_js_number(&object, "y", point.y as f64)?;
    Ok(object)
}

fn scroll_js_object(scroll: &crate::ScrollState) -> Result<Object, JsValue> {
    let object = Object::new();
    let axes = scroll.axes();
    set_js_bool(&object, "horizontal", axes.horizontal)?;
    set_js_bool(&object, "vertical", axes.vertical)?;
    set_js_object(
        &object,
        "offset",
        point_js_object(scroll.offset())?.as_ref(),
    )?;
    set_js_object(
        &object,
        "maxOffset",
        point_js_object(scroll.max_offset())?.as_ref(),
    )?;
    set_js_object(
        &object,
        "viewportSize",
        size_js_object(scroll.viewport_size())?.as_ref(),
    )?;
    set_js_object(
        &object,
        "contentSize",
        size_js_object(scroll.content_size())?.as_ref(),
    )?;
    Ok(object)
}

fn set_js_string(object: &Object, name: &str, value: &str) -> Result<(), JsValue> {
    Reflect::set(
        object.as_ref(),
        &JsValue::from_str(name),
        &JsValue::from_str(value),
    )?;
    Ok(())
}

fn set_js_number(object: &Object, name: &str, value: f64) -> Result<(), JsValue> {
    Reflect::set(
        object.as_ref(),
        &JsValue::from_str(name),
        &JsValue::from_f64(value),
    )?;
    Ok(())
}

fn set_js_bool(object: &Object, name: &str, value: bool) -> Result<(), JsValue> {
    Reflect::set(
        object.as_ref(),
        &JsValue::from_str(name),
        &JsValue::from_bool(value),
    )?;
    Ok(())
}

fn set_js_object(object: &Object, name: &str, value: &JsValue) -> Result<(), JsValue> {
    Reflect::set(object.as_ref(), &JsValue::from_str(name), value)?;
    Ok(())
}

fn set_js_array(object: &Object, name: &str, value: &Array) -> Result<(), JsValue> {
    Reflect::set(object.as_ref(), &JsValue::from_str(name), value.as_ref())?;
    Ok(())
}

fn register_pointer_events<State, Update, View>(
    canvas: &web_sys::HtmlCanvasElement,
    app: Rc<RefCell<WebRuntimeApp<State, Update, View>>>,
) -> Result<(), JsValue>
where
    State: 'static,
    Update: FnMut(&mut State, WidgetAction) + 'static,
    View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
{
    // Native touch scrolling would cancel an application-owned drag.
    canvas.style().set_property("touch-action", "none")?;
    for event_name in [
        "pointerdown",
        "pointermove",
        "pointerup",
        "pointercancel",
        "lostpointercapture",
    ] {
        let target = canvas.clone();
        let app = app.clone();
        let driver = app.borrow().frame_driver.clone();
        let closure = Closure::<dyn FnMut(web_sys::PointerEvent)>::wrap(Box::new(
            move |event: web_sys::PointerEvent| {
                if driver.is_stopped() {
                    return;
                }
                if event.type_() == "lostpointercapture" {
                    with_web_runtime_app_mut(&app, "lost pointer capture", |app| {
                        app.cancel_pointer(event.pointer_id(), app.timestamp_millis());
                    });
                    return;
                }
                event.prevent_default();
                if event.type_() == "pointerdown" {
                    let _ = target.focus();
                    if let Err(error) = target.set_pointer_capture(event.pointer_id()) {
                        web_sys::console::warn_1(&error);
                    }
                }
                let kind = match event.type_().as_str() {
                    "pointerdown" => PointerEventKind::Down(pointer_button(event.button())),
                    "pointerup" => PointerEventKind::Up(pointer_button(event.button())),
                    "pointercancel" => PointerEventKind::Cancel,
                    _ => PointerEventKind::Move,
                };
                with_web_runtime_app_mut(&app, "pointer event", |app| {
                    app.push_pointer(&event, kind);
                });
                if matches!(kind, PointerEventKind::Up(_) | PointerEventKind::Cancel)
                    && target.has_pointer_capture(event.pointer_id())
                {
                    let _ = target.release_pointer_capture(event.pointer_id());
                }
            },
        ));
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    Ok(())
}

fn register_window_events<State, Update, View>(
    window: &web_sys::Window,
    app: Rc<RefCell<WebRuntimeApp<State, Update, View>>>,
) -> Result<(), JsValue>
where
    State: 'static,
    Update: FnMut(&mut State, WidgetAction) + 'static,
    View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
{
    let blur_app = app.clone();
    let blur_driver = app.borrow().frame_driver.clone();
    let blur =
        Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event: web_sys::Event| {
            if blur_driver.is_stopped() {
                return;
            }
            // Release DOM capture outside the RefCell borrow: browsers can dispatch
            // lostpointercapture while releasing it.
            let release = match blur_app.try_borrow_mut() {
                Ok(mut app) => {
                    let timestamp_millis = app.timestamp_millis();
                    let pointers = app.cancel_pointers(timestamp_millis);
                    app.frame_driver.wake();
                    Some((app.canvas.clone(), pointers))
                }
                Err(error) => {
                    log_web_runtime_reentry("window blur", &error);
                    None
                }
            };
            if let Some((canvas, pointers)) = release {
                for pointer in pointers {
                    if canvas.has_pointer_capture(pointer) {
                        let _ = canvas.release_pointer_capture(pointer);
                    }
                }
            }
        }));
    window.add_event_listener_with_callback("blur", blur.as_ref().unchecked_ref())?;
    blur.forget();

    let motion_app = app.clone();
    let motion = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(move |event| {
        // Pointer-lock movement is device motion; ordinary pointer events still
        // flow through the shared ordered input dispatcher.
        if motion_app
            .try_borrow()
            .is_ok_and(|app| app.pointer_locked())
        {
            with_web_runtime_app_mut(&motion_app, "raw mouse motion", |app| {
                app.push_raw_mouse_motion(&event)
            });
        }
    }));
    window.add_event_listener_with_callback("mousemove", motion.as_ref().unchecked_ref())?;
    motion.forget();

    for event_name in ["pointerlockchange", "pointerlockerror"] {
        let lock_app = app.clone();
        let driver = app.borrow().frame_driver.clone();
        let canvas = app.borrow().canvas.clone();
        let lock =
            Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |event: web_sys::Event| {
                if driver.is_stopped() {
                    // A request issued before failure may acquire the lock later.
                    if let Ok(document) = browser_document() {
                        if document
                            .pointer_lock_element()
                            .is_some_and(|element| element == canvas.clone().into())
                        {
                            document.exit_pointer_lock();
                        }
                    }
                    return;
                }
                with_web_runtime_app_mut(&lock_app, "pointer lock", |app| {
                    if event.type_() == "pointerlockerror" {
                        app.pointer_lock_desired = false;
                        app.complete_pointer_lock_requests(CursorResponse::Error(
                            PlatformServiceError::new(
                                PlatformErrorCode::Failed,
                                "browser denied pointer lock",
                            ),
                        ));
                    } else if app.pointer_locked() {
                        if app.pointer_lock_desired {
                            app.complete_pointer_lock_requests(CursorResponse::Applied);
                        } else if let Ok(document) = browser_document() {
                            document.exit_pointer_lock();
                        }
                    } else {
                        app.cancel_pointers(app.timestamp_millis());
                    }
                });
            }));
        browser_document()?
            .add_event_listener_with_callback(event_name, lock.as_ref().unchecked_ref())?;
        lock.forget();
    }

    if app.borrow().hooks.close_requested.is_some() {
        let close_app = app.clone();
        let close = Closure::<dyn FnMut(web_sys::BeforeUnloadEvent)>::wrap(Box::new(
            move |event: web_sys::BeforeUnloadEvent| {
                // A failed renderer must not disable unsaved-change protection.
                match close_app.try_borrow_mut() {
                    Ok(mut app) => {
                        let app = &mut *app;
                        if let Some(hook) = app.hooks.close_requested.as_mut() {
                            app.session.invalidate_view();
                            if !hook(&mut app.state) {
                                // The browser owns the confirmation dialog and may
                                // suppress it when the page has no user activation.
                                event.prevent_default();
                                event.set_return_value("");
                            }
                        }
                        app.frame_driver.wake();
                    }
                    Err(error) => log_web_runtime_reentry("close request", &error),
                }
            },
        ));
        window.add_event_listener_with_callback("beforeunload", close.as_ref().unchecked_ref())?;
        close.forget();
    }
    Ok(())
}

fn with_web_runtime_app_mut<State, Update, View>(
    app: &Rc<RefCell<WebRuntimeApp<State, Update, View>>>,
    context: &str,
    apply: impl FnOnce(&mut WebRuntimeApp<State, Update, View>),
) {
    match app.try_borrow_mut() {
        Ok(mut app) => {
            if app.frame_driver.is_stopped() {
                return;
            }
            apply(&mut app);
            app.frame_driver.wake();
        }
        Err(error) => log_web_runtime_reentry(context, &error),
    }
}

fn push_async_platform_response(
    responses: &Rc<RefCell<Vec<PlatformServiceResponse>>>,
    driver: &WebFrameDriver,
    response: PlatformServiceResponse,
    context: &str,
) {
    if driver.is_stopped() {
        return;
    }
    match responses.try_borrow_mut() {
        Ok(mut responses) => {
            responses.push(response);
            driver.wake();
        }
        Err(error) => log_web_runtime_reentry(context, &error),
    }
}

fn log_web_runtime_reentry(context: &str, error: &BorrowMutError) {
    web_sys::console::warn_1(&JsValue::from_str(&format!(
        "Operad web runtime skipped {context}: callback re-entered while runtime state was already borrowed ({error})"
    )));
}

fn register_wheel_events<State, Update, View>(
    canvas: &web_sys::HtmlCanvasElement,
    app: Rc<RefCell<WebRuntimeApp<State, Update, View>>>,
) -> Result<(), JsValue>
where
    State: 'static,
    Update: FnMut(&mut State, WidgetAction) + 'static,
    View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
{
    let driver = app.borrow().frame_driver.clone();
    let closure = Closure::<dyn FnMut(web_sys::WheelEvent)>::wrap(Box::new(
        move |event: web_sys::WheelEvent| {
            if driver.is_stopped() {
                return;
            }
            event.prevent_default();
            with_web_runtime_app_mut(&app, "wheel event", |app| {
                app.push_wheel(event);
            });
        },
    ));
    canvas.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}

fn register_keyboard_events<State, Update, View>(
    window: &web_sys::Window,
    app: Rc<RefCell<WebRuntimeApp<State, Update, View>>>,
) -> Result<(), JsValue>
where
    State: 'static,
    Update: FnMut(&mut State, WidgetAction) + 'static,
    View: FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
{
    for (event_name, pressed) in [("keydown", true), ("keyup", false)] {
        let app = app.clone();
        let driver = app.borrow().frame_driver.clone();
        let closure = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
            move |event: web_sys::KeyboardEvent| {
                if driver.is_stopped() {
                    return;
                }
                let owns_keyboard = app.try_borrow().is_ok_and(|app| {
                    app.pointer_locked()
                        || app.text_input.is_focused()
                        || browser_document()
                            .ok()
                            .and_then(|document| document.active_element())
                            .is_some_and(|element| element == app.canvas.clone().into())
                });
                // A key owned by composition can be released after another DOM
                // control takes focus. Retire that ownership without routing the
                // external control's event through Operad.
                let ime_key = (!pressed || owns_keyboard)
                    && app
                        .try_borrow()
                        .is_ok_and(|app| app.text_input.owns_key(&event, pressed));
                if owns_keyboard {
                    if ime_key {
                        // A preceding ordinary key may still be waiting for the
                        // next frame. Publish its new surrounding text before the
                        // browser starts editing its native composition surface.
                        let mut terminal_error = None;
                        with_web_runtime_app_mut(&app, "composition key", |app| {
                            if !app.text_input.is_composing()
                                && !app.pending_input.borrow().is_empty()
                            {
                                terminal_error = app.render().err();
                            }
                        });
                        if let Some(error) = terminal_error {
                            stop_web_frame_error(&app, &error);
                        }
                        // Native composition must see these keys, including Enter,
                        // arrows, and Escape. Do not also dispatch widget shortcuts.
                        return;
                    }
                    if key_code(&event).is_some() {
                        event.prevent_default();
                    }
                    with_web_runtime_app_mut(&app, "keyboard event", |app| {
                        app.push_key(event, pressed);
                    });
                }
            },
        ));
        window.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    Ok(())
}

fn install_document_chrome(options: &WebRuntimeOptions) -> Result<(), JsValue> {
    let document = browser_document()?;
    document.set_title(&options.title);
    let body = document
        .body()
        .ok_or_else(|| web_error("browser document body is unavailable"))?;
    body.style().set_property("margin", "0")?;
    body.style().set_property("overflow", "hidden")?;
    body.style()
        .set_property("background", &options.background)?;

    let canvas = canvas_element(&options.canvas_id)?;
    let style = canvas.style();
    style.set_property("display", "block")?;
    style.set_property("width", "100vw")?;
    style.set_property("height", "100vh")?;
    style.set_property("outline", "none")?;
    Ok(())
}

fn canvas_element(canvas_id: &str) -> Result<web_sys::HtmlCanvasElement, JsValue> {
    let document = browser_document()?;
    if let Some(element) = document.get_element_by_id(canvas_id) {
        return Ok(element.dyn_into::<web_sys::HtmlCanvasElement>()?);
    }

    let canvas = document
        .create_element("canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    canvas.set_id(canvas_id);
    let body = document
        .body()
        .ok_or_else(|| web_error("browser document body is unavailable"))?;
    body.append_child(&canvas)?;
    Ok(canvas)
}

fn canvas_metrics(
    canvas: &web_sys::HtmlCanvasElement,
) -> Result<(UiSize, PixelSize, f32), JsValue> {
    let window = browser_window()?;
    let rect = canvas.get_bounding_client_rect();
    let width = rect.width().max(1.0) as f32;
    let height = rect.height().max(1.0) as f32;
    let ratio = window.device_pixel_ratio() as f32;
    let dpi_scale = if ratio.is_finite() && ratio > 0.0 {
        ratio
    } else {
        1.0
    };
    let pixel_size = PixelSize::new(
        (width * dpi_scale).ceil().max(1.0) as u32,
        (height * dpi_scale).ceil().max(1.0) as u32,
    );
    Ok((UiSize::new(width, height), pixel_size, dpi_scale))
}

fn normalized_web_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

fn pointer_position(canvas: &web_sys::HtmlCanvasElement, client_x: f64, client_y: f64) -> UiPoint {
    let rect = canvas.get_bounding_client_rect();
    UiPoint::new(
        (client_x - rect.left()) as f32,
        (client_y - rect.top()) as f32,
    )
}

fn pointer_button(button: i16) -> PointerButton {
    match button {
        0 => PointerButton::Primary,
        1 => PointerButton::Auxiliary,
        2 => PointerButton::Secondary,
        3 => PointerButton::Back,
        4 => PointerButton::Forward,
        other => PointerButton::Other(other.max(0) as u16),
    }
}

fn web_pointer_buttons(bits: u16) -> PointerButtons {
    let mut buttons = PointerButtons::NONE;
    if bits & 1 != 0 {
        buttons = buttons.with(PointerButton::Primary);
    }
    if bits & 2 != 0 {
        buttons = buttons.with(PointerButton::Secondary);
    }
    if bits & 4 != 0 {
        buttons = buttons.with(PointerButton::Auxiliary);
    }
    if bits & 8 != 0 {
        buttons = buttons.with(PointerButton::Back);
    }
    if bits & 16 != 0 {
        buttons = buttons.with(PointerButton::Forward);
    }
    buttons
}

fn wheel_delta(event: &web_sys::WheelEvent) -> (UiPoint, WheelDeltaUnit) {
    let delta = UiPoint::new(event.delta_x() as f32, event.delta_y() as f32);
    match event.delta_mode() {
        1 => (delta, WheelDeltaUnit::Line),
        2 => (delta, WheelDeltaUnit::Page),
        _ => (delta, WheelDeltaUnit::Pixel),
    }
}

fn pointer_modifiers(event: &web_sys::MouseEvent) -> KeyModifiers {
    KeyModifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key(),
    }
}

fn wheel_modifiers(event: &web_sys::WheelEvent) -> KeyModifiers {
    KeyModifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key(),
    }
}

fn key_modifiers(event: &web_sys::KeyboardEvent) -> KeyModifiers {
    KeyModifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key(),
    }
}

fn key_code(event: &web_sys::KeyboardEvent) -> Option<KeyCode> {
    match event.key().as_str() {
        "Backspace" => Some(KeyCode::Backspace),
        "Delete" => Some(KeyCode::Delete),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "Home" => Some(KeyCode::Home),
        "End" => Some(KeyCode::End),
        "Enter" => Some(KeyCode::Enter),
        "Escape" => Some(KeyCode::Escape),
        "Tab" => Some(KeyCode::Tab),
        "F10" => Some(KeyCode::F10),
        "ContextMenu" => Some(KeyCode::ContextMenu),
        value => {
            let mut chars = value.chars();
            let ch = chars.next()?;
            chars.next().is_none().then_some(KeyCode::Character(ch))
        }
    }
}

fn text_input_for_key(event: &web_sys::KeyboardEvent) -> Option<String> {
    let key = event.key();
    let mut chars = key.chars();
    let ch = chars.next()?;
    (chars.next().is_none() && !ch.is_control()).then_some(key)
}

fn css_cursor(shape: CursorShape) -> &'static str {
    match shape {
        CursorShape::Default => "auto",
        CursorShape::Pointer => "pointer",
        CursorShape::Text => "text",
        CursorShape::Crosshair => "crosshair",
        CursorShape::Grab => "grab",
        CursorShape::Grabbing => "grabbing",
        CursorShape::Move => "move",
        CursorShape::NotAllowed => "not-allowed",
        CursorShape::Wait => "wait",
        CursorShape::Progress => "progress",
        CursorShape::ResizeHorizontal => "ew-resize",
        CursorShape::ResizeVertical => "ns-resize",
        CursorShape::ResizeNorthEastSouthWest => "nesw-resize",
        CursorShape::ResizeNorthWestSouthEast => "nwse-resize",
        CursorShape::ZoomIn => "zoom-in",
        CursorShape::ZoomOut => "zoom-out",
    }
}

fn browser_window() -> Result<web_sys::Window, JsValue> {
    web_sys::window().ok_or_else(|| web_error("browser window is unavailable"))
}

fn browser_document() -> Result<web_sys::Document, JsValue> {
    browser_window()?
        .document()
        .ok_or_else(|| web_error("browser document is unavailable"))
}

fn set_status(status_id: &str, message: &str) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let status = document.get_element_by_id(status_id).or_else(|| {
        // Startup pages commonly remove their loading message once run_web returns.
        let status = document.create_element("div").ok()?;
        status.set_id(status_id);
        status.set_attribute("role", "alert").ok()?;
        status.set_attribute("style", "position:fixed;inset:16px 16px auto;padding:16px;background:#201c1c;color:#fff;font:16px system-ui;z-index:2147483647").ok()?;
        document.body()?.append_child(&status).ok()?;
        Some(status)
    });
    if let Some(status) = status {
        status.set_text_content(Some(message));
    }
}

fn layout_web_error(error: taffy::TaffyError) -> JsValue {
    web_error(format!("layout failed: {error}"))
}

async fn web_clipboard_read_text() -> Result<Option<String>, JsValue> {
    let clipboard = browser_window()?.navigator().clipboard();
    let value = wasm_bindgen_futures::JsFuture::from(clipboard.read_text()).await?;
    Ok(value.as_string())
}

async fn web_clipboard_write_text(text: &str) -> Result<(), JsValue> {
    let clipboard = browser_window()?.navigator().clipboard();
    wasm_bindgen_futures::JsFuture::from(clipboard.write_text(text)).await?;
    Ok(())
}

fn web_clipboard_error(error: JsValue) -> ClipboardResponse {
    ClipboardResponse::Error(PlatformServiceError::new(
        PlatformErrorCode::Failed,
        web_message(&error),
    ))
}

fn web_startup_error(
    operation: &'static str,
    error: impl ToString,
    next_step: &'static str,
) -> JsValue {
    web_error(format!(
        "Web runtime startup failed while {operation}: {}\n\
         Consequence: the WebGPU UI did not start.\n\
         Next step: {next_step}",
        error.to_string()
    ))
}

fn publish_web_startup_error(options: &WebRuntimeOptions, error: &JsValue) {
    web_sys::console::error_1(error);
    if let Some(status_id) = options.status_id.as_deref() {
        set_status(status_id, &web_message(error));
    }
}

fn cursor_error(error: JsValue) -> CursorResponse {
    CursorResponse::Error(PlatformServiceError::new(
        PlatformErrorCode::Failed,
        web_message(&error),
    ))
}

fn web_error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}

fn web_message(value: &JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "unknown JavaScript error".to_string())
}
