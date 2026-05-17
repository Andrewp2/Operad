//! Browser runtime for WASM/WebGPU applications.
//!
//! This is the web counterpart to the native-window runner: applications
//! provide state, an update function, and a view function, while Operad owns the
//! host loop, input conversion, layout, action dispatch, runtime state
//! persistence, and WGPU surface presentation.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::host::{collect_document_widget_actions, process_host_frame_input_with_target_resolver};
use crate::platform::{
    BackendAdapterKind, BackendCapabilities, ClipboardRequest, ClipboardResponse, CursorGrabMode,
    CursorRequest, CursorResponse, CursorShape, LayerCapabilities, OpenUrlResponse, PixelSize,
    PlatformErrorCode, PlatformRequest, PlatformRequestId, PlatformRequestIdAllocator,
    PlatformResponse, PlatformServiceCapabilities, PlatformServiceError, PlatformServiceRequest,
    PlatformServiceResponse, RenderingCapabilities, RepaintRequest, RepaintResponse,
    ResourceCapabilities,
};
use crate::{
    AccessibilityCapabilities, AnimationMachine, ApproxTextMeasurer, EmptyResourceResolver,
    HostDocumentFrameState, HostFrameOutput, HostInteractionState, InputCapabilities, KeyCode,
    KeyModifiers, PointerButton, PointerButtons, PointerEventKind, RawInputEvent, RawKeyboardEvent,
    RawPointerEvent, RawTextInputEvent, RawWheelEvent, RenderTarget, RendererAdapter, UiDocument,
    UiFocusState, UiNodeId, UiPoint, UiSize, WgpuSurfaceRenderer, WheelDeltaUnit, WheelPhase,
    WidgetAction, WidgetActionBinding,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WebRuntimeMetrics {
    pub viewport: UiSize,
    pub pixel_size: PixelSize,
    pub dpi_scale: f32,
    pub timestamp_ms: f64,
}

pub struct WebRuntimeHooks<State> {
    before_render: Option<Box<dyn FnMut(&mut State, WebRuntimeMetrics)>>,
    title: Option<Box<dyn FnMut(&State) -> String>>,
    platform_requests:
        Option<Box<dyn FnMut(&mut State, WebRuntimeMetrics) -> Vec<PlatformRequest>>>,
    platform_service_requests:
        Option<Box<dyn FnMut(&mut State, WebRuntimeMetrics) -> Vec<PlatformServiceRequest>>>,
    platform_responses: Option<Box<dyn FnMut(&mut State, &[PlatformServiceResponse])>>,
}

impl<State> WebRuntimeHooks<State> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_before_render(
        mut self,
        hook: impl FnMut(&mut State, WebRuntimeMetrics) + 'static,
    ) -> Self {
        self.before_render = Some(Box::new(hook));
        self
    }

    pub fn with_title(mut self, title: impl FnMut(&State) -> String + 'static) -> Self {
        self.title = Some(Box::new(title));
        self
    }

    pub fn with_platform_requests(
        mut self,
        hook: impl FnMut(&mut State, WebRuntimeMetrics) -> Vec<PlatformRequest> + 'static,
    ) -> Self {
        self.platform_requests = Some(Box::new(hook));
        self
    }

    pub fn with_platform_service_requests(
        mut self,
        hook: impl FnMut(&mut State, WebRuntimeMetrics) -> Vec<PlatformServiceRequest> + 'static,
    ) -> Self {
        self.platform_service_requests = Some(Box::new(hook));
        self
    }

    pub fn with_platform_responses(
        mut self,
        hook: impl FnMut(&mut State, &[PlatformServiceResponse]) + 'static,
    ) -> Self {
        self.platform_responses = Some(Box::new(hook));
        self
    }
}

impl<State> Default for WebRuntimeHooks<State> {
    fn default() -> Self {
        Self {
            before_render: None,
            title: None,
            platform_requests: None,
            platform_service_requests: None,
            platform_responses: None,
        }
    }
}

pub fn web_runtime_capabilities() -> BackendCapabilities {
    BackendCapabilities::new("web-runtime")
        .adapter(BackendAdapterKind::Wgpu)
        .input(InputCapabilities {
            pointer_move: true,
            pointer_button: true,
            pointer_wheel: true,
            wheel_phase: false,
            high_resolution_wheel: true,
            keyboard_press: true,
            keyboard_release: true,
            text_input: true,
            text_ime: false,
            modifiers: true,
            raw_mouse_motion: false,
            pointer_lock: false,
            gamepad: false,
            canvas_local_input: true,
        })
        .resources(ResourceCapabilities {
            images: true,
            icons: true,
            textures: true,
            thumbnails: true,
            tinted_icons: true,
            partial_texture_updates: true,
        })
        .layers(LayerCapabilities::STANDARD)
        .services(PlatformServiceCapabilities {
            clipboard_read: true,
            clipboard_write: true,
            open_url: true,
            cursor_shape: true,
            cursor_visible: true,
            repaint: true,
            ..PlatformServiceCapabilities::NONE
        })
        .rendering(RenderingCapabilities {
            high_dpi: true,
            offscreen: false,
            deterministic_snapshots: false,
            partial_updates: true,
            webgpu_surface: true,
            native_child_windows: false,
            platform_overlays: false,
        })
        .accessibility(AccessibilityCapabilities::NONE)
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
        move |_state: &(), viewport| view(viewport),
    )
    .await
}

pub async fn run_app<State>(
    title: impl Into<String>,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize) -> UiDocument + 'static,
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
    view: impl FnMut(&State, UiSize) -> UiDocument + 'static,
) -> Result<(), JsValue>
where
    State: 'static,
{
    run_app_with_hooks(options, state, update, view, WebRuntimeHooks::default()).await
}

pub async fn run_app_with_hooks<State>(
    options: WebRuntimeOptions,
    state: State,
    update: impl FnMut(&mut State, WidgetAction) + 'static,
    view: impl FnMut(&State, UiSize) -> UiDocument + 'static,
    hooks: WebRuntimeHooks<State>,
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

    let app = match WebRuntimeApp::new(options, state, update, view, hooks, canvas).await {
        Ok(app) => Rc::new(RefCell::new(app)),
        Err(error) => {
            publish_web_startup_error(&startup_options, &error);
            return Err(error);
        }
    };
    register_pointer_events(app.borrow().canvas(), app.clone())?;
    register_wheel_events(app.borrow().canvas(), app.clone())?;
    register_keyboard_events(&browser_window()?, app.clone())?;
    start_animation_loop(app)?;
    Ok(())
}

struct WebRuntimeApp<State, Update, View> {
    options: WebRuntimeOptions,
    state: State,
    update: Update,
    view: View,
    hooks: WebRuntimeHooks<State>,
    frame_state: HostDocumentFrameState,
    platform_request_ids: PlatformRequestIdAllocator,
    pending_platform_responses: Vec<PlatformServiceResponse>,
    async_platform_responses: Rc<RefCell<Vec<PlatformServiceResponse>>>,
    renderer: WgpuSurfaceRenderer<'static>,
    canvas: web_sys::HtmlCanvasElement,
    text_measurer: ApproxTextMeasurer,
    pending_input: Vec<RawInputEvent>,
    cursor: Option<UiPoint>,
    buttons: PointerButtons,
    modifiers: KeyModifiers,
    scroll_offsets: HashMap<String, UiPoint>,
    animation_states: HashMap<String, AnimationMachine>,
    dpi_scale: f32,
    last_tick_ms: Option<f64>,
    last_animation_ms: Option<f64>,
}

impl<State, Update, View> WebRuntimeApp<State, Update, View> {
    async fn new(
        options: WebRuntimeOptions,
        state: State,
        update: Update,
        view: View,
        hooks: WebRuntimeHooks<State>,
        canvas: web_sys::HtmlCanvasElement,
    ) -> Result<Self, JsValue> {
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

        Ok(Self {
            options,
            state,
            update,
            view,
            hooks,
            frame_state: HostDocumentFrameState::new(),
            platform_request_ids: PlatformRequestIdAllocator::new(1),
            pending_platform_responses: Vec::new(),
            async_platform_responses: Rc::new(RefCell::new(Vec::new())),
            renderer,
            canvas,
            text_measurer: ApproxTextMeasurer,
            pending_input: Vec::new(),
            cursor: None,
            buttons: PointerButtons::NONE,
            modifiers: KeyModifiers::NONE,
            scroll_offsets: HashMap::new(),
            animation_states: HashMap::new(),
            dpi_scale,
            last_tick_ms: None,
            last_animation_ms: None,
        })
    }

    fn canvas(&self) -> &web_sys::HtmlCanvasElement {
        &self.canvas
    }

    fn render(&mut self, timestamp_ms: f64) -> Result<(), JsValue>
    where
        Update: FnMut(&mut State, WidgetAction),
        View: FnMut(&State, UiSize) -> UiDocument,
    {
        let (viewport, pixel_size, dpi_scale) = canvas_metrics(&self.canvas)?;
        if self.canvas.width() != pixel_size.width {
            self.canvas.set_width(pixel_size.width);
        }
        if self.canvas.height() != pixel_size.height {
            self.canvas.set_height(pixel_size.height);
        }
        self.dpi_scale = dpi_scale;

        let metrics = WebRuntimeMetrics {
            viewport,
            pixel_size,
            dpi_scale,
            timestamp_ms,
        };
        if let Some(before_render) = self.hooks.before_render.as_mut() {
            before_render(&mut self.state, metrics);
        }
        if let Some(title) = self.hooks.title.as_mut() {
            if let Ok(document) = browser_document() {
                document.set_title(&title(&self.state));
            }
        }
        self.apply_hook_platform_requests(metrics);
        self.dispatch_tick(timestamp_ms);
        let animation_dt = self.animation_delta_seconds(timestamp_ms);

        let mut document = self.build_document(viewport).map_err(layout_web_error)?;
        document.tick_animations(animation_dt);
        let raw_input = std::mem::take(&mut self.pending_input);
        self.drain_async_platform_responses();
        let mut host_request = self.frame_state.host_frame_request(viewport);
        host_request.raw_input = raw_input;
        host_request.platform_responses = std::mem::take(&mut self.pending_platform_responses);
        let host_output =
            process_host_frame_input_with_target_resolver(host_request, |event, state| {
                resolve_target(event, state, &document)
            });
        let frame_request = self.frame_state.document_frame_request(
            viewport,
            RenderTarget::window(self.options.target_name.clone(), viewport),
            host_output,
        );
        let frame =
            crate::process_document_frame(&mut document, &mut self.text_measurer, frame_request)
                .map_err(layout_web_error)?;
        self.capture_document_runtime_state(&document);
        let actions = collect_document_widget_actions(&document, &frame);
        self.frame_state.apply_document_frame_output(&frame);
        self.apply_platform_service_requests(&frame);

        let frame = if actions.is_empty() {
            frame
        } else {
            for action in actions {
                (self.update)(&mut self.state, action);
            }
            let mut document = self.build_document(viewport).map_err(layout_web_error)?;
            let frame_request = self.frame_state.document_frame_request(
                viewport,
                RenderTarget::window(self.options.target_name.clone(), viewport),
                HostFrameOutput::new(self.frame_state.interaction.clone()),
            );
            let frame = crate::process_document_frame(
                &mut document,
                &mut self.text_measurer,
                frame_request,
            )
            .map_err(layout_web_error)?;
            self.capture_document_runtime_state(&document);
            self.frame_state.apply_document_frame_output(&frame);
            self.apply_platform_service_requests(&frame);
            frame
        };

        self.renderer
            .render_frame(frame.render_request, &EmptyResourceResolver)
            .map_err(render_web_error)?;
        Ok(())
    }

    fn build_document(&mut self, viewport: UiSize) -> Result<UiDocument, taffy::TaffyError>
    where
        View: FnMut(&State, UiSize) -> UiDocument,
    {
        let mut document = (self.view)(&self.state, viewport);
        document.set_ui_scale(self.options.ui_scale);
        document.set_dpi_scale(self.dpi_scale);
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

    fn capture_document_runtime_state(&mut self, document: &UiDocument) {
        self.capture_scroll_offsets(document);
        self.capture_animation_states(document);
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

    fn dispatch_tick(&mut self, timestamp_ms: f64)
    where
        Update: FnMut(&mut State, WidgetAction),
    {
        let Some(action) = self.options.tick_action.clone() else {
            return;
        };
        let interval = self.options.tick_interval_ms();
        let mut last_tick = self.last_tick_ms.unwrap_or(timestamp_ms);
        let mut ticks = 0;
        while timestamp_ms - last_tick >= interval && ticks < 4 {
            (self.update)(
                &mut self.state,
                WidgetAction::activate(UiNodeId(0), action.clone()),
            );
            last_tick += interval;
            ticks += 1;
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

    fn push_pointer(&mut self, event: web_sys::PointerEvent, kind: PointerEventKind) {
        self.modifiers = pointer_modifiers(&event);
        self.buttons = match kind {
            PointerEventKind::Down(button) => self.buttons.with(button),
            PointerEventKind::Up(button) => self.buttons.without(button),
            PointerEventKind::Move | PointerEventKind::Cancel => {
                web_pointer_buttons(event.buttons())
            }
        };
        let point = pointer_position(&self.canvas, event.client_x(), event.client_y());
        self.cursor = Some(point);
        self.pending_input.push(RawInputEvent::Pointer(
            RawPointerEvent::new(kind, point, event.time_stamp() as u64)
                .buttons(self.buttons)
                .modifiers(self.modifiers),
        ));
    }

    fn push_wheel(&mut self, event: web_sys::WheelEvent) {
        self.modifiers = wheel_modifiers(&event);
        let point = pointer_position(&self.canvas, event.client_x(), event.client_y());
        self.cursor = Some(point);
        let (delta, unit) = wheel_delta(&event);
        self.pending_input.push(RawInputEvent::Wheel(RawWheelEvent {
            position: point,
            delta,
            unit,
            phase: WheelPhase::Moved,
            modifiers: self.modifiers,
            timestamp_millis: event.time_stamp() as u64,
        }));
    }

    fn push_key(&mut self, event: web_sys::KeyboardEvent, pressed: bool) {
        self.modifiers = key_modifiers(&event);
        let Some(key) = key_code(&event) else {
            return;
        };
        let timestamp = event.time_stamp() as u64;
        let raw = if pressed {
            RawKeyboardEvent::press(key, self.modifiers, timestamp).repeat(event.repeat())
        } else {
            RawKeyboardEvent::release(key, self.modifiers, timestamp)
        };
        self.pending_input.push(RawInputEvent::Keyboard(raw));
        if pressed && !self.modifiers.ctrl && !self.modifiers.meta {
            if let Some(text) = text_input_for_key(&event) {
                self.pending_input
                    .push(RawInputEvent::Text(RawTextInputEvent::new(text, timestamp)));
            }
        }
    }

    fn apply_platform_service_requests(&mut self, frame: &crate::HostDocumentFrameOutput) {
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

    fn apply_hook_platform_requests(&mut self, metrics: WebRuntimeMetrics) {
        if self.hooks.platform_requests.is_none() && self.hooks.platform_service_requests.is_none()
        {
            return;
        }

        let mut requests = Vec::new();
        if let Some(platform_requests) = self.hooks.platform_requests.as_mut() {
            requests.extend(
                self.platform_request_ids
                    .allocate_all(platform_requests(&mut self.state, metrics)),
            );
        }
        if let Some(platform_service_requests) = self.hooks.platform_service_requests.as_mut() {
            requests.extend(platform_service_requests(&mut self.state, metrics));
        }
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
                wasm_bindgen_futures::spawn_local(async move {
                    let response = match web_clipboard_read_text().await {
                        Ok(text) => ClipboardResponse::Text(text),
                        Err(error) => web_clipboard_error(error),
                    };
                    responses.borrow_mut().push(PlatformServiceResponse::new(
                        id,
                        PlatformResponse::Clipboard(response),
                    ));
                });
                None
            }
            ClipboardRequest::WriteText(text) => {
                let responses = self.async_platform_responses.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let response = match web_clipboard_write_text(&text).await {
                        Ok(()) => ClipboardResponse::Completed,
                        Err(error) => web_clipboard_error(error),
                    };
                    responses.borrow_mut().push(PlatformServiceResponse::new(
                        id,
                        PlatformResponse::Clipboard(response),
                    ));
                });
                None
            }
            ClipboardRequest::Clear => {
                let responses = self.async_platform_responses.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let response = match web_clipboard_write_text("").await {
                        Ok(()) => ClipboardResponse::Completed,
                        Err(error) => web_clipboard_error(error),
                    };
                    responses.borrow_mut().push(PlatformServiceResponse::new(
                        id,
                        PlatformResponse::Clipboard(response),
                    ));
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
        let responses = self
            .async_platform_responses
            .borrow_mut()
            .drain(..)
            .collect::<Vec<_>>();
        self.dispatch_platform_responses(&responses);
        self.pending_platform_responses.extend(responses);
    }

    fn dispatch_platform_responses(&mut self, responses: &[PlatformServiceResponse]) {
        if responses.is_empty() {
            return;
        }
        if let Some(platform_responses) = self.hooks.platform_responses.as_mut() {
            platform_responses(&mut self.state, responses);
        }
    }

    fn apply_platform_request(&mut self, request: PlatformRequest) -> PlatformResponse {
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
            PlatformRequest::Cursor(request) => {
                PlatformResponse::Cursor(self.apply_cursor_request(request))
            }
            PlatformRequest::Repaint(request) => {
                PlatformResponse::Repaint(self.apply_repaint_request(request))
            }
            request => PlatformResponse::unsupported(request.kind()),
        }
    }

    fn apply_cursor_request(&self, request: CursorRequest) -> CursorResponse {
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
            CursorRequest::SetPosition(_)
            | CursorRequest::SetGrab(CursorGrabMode::Locked | CursorGrabMode::Confined)
            | CursorRequest::Confine(_)
            | CursorRequest::ReleaseConfine => CursorResponse::Unsupported,
            CursorRequest::SetGrab(CursorGrabMode::None) => CursorResponse::Applied,
        }
    }

    fn apply_repaint_request(&self, request: RepaintRequest) -> RepaintResponse {
        match request {
            RepaintRequest::NextFrame
            | RepaintRequest::Area(_)
            | RepaintRequest::Continuous { active: true } => RepaintResponse::Scheduled {
                delay: Duration::ZERO,
            },
            RepaintRequest::After(delay) => RepaintResponse::Scheduled { delay },
            RepaintRequest::Continuous { active: false } => RepaintResponse::Coalesced,
        }
    }
}

fn register_pointer_events<State, Update, View>(
    canvas: &web_sys::HtmlCanvasElement,
    app: Rc<RefCell<WebRuntimeApp<State, Update, View>>>,
) -> Result<(), JsValue>
where
    State: 'static,
    Update: FnMut(&mut State, WidgetAction) + 'static,
    View: FnMut(&State, UiSize) -> UiDocument + 'static,
{
    for event_name in ["pointerdown", "pointermove", "pointerup", "pointercancel"] {
        let target = canvas.clone();
        let app = app.clone();
        let closure = Closure::<dyn FnMut(web_sys::PointerEvent)>::wrap(Box::new(
            move |event: web_sys::PointerEvent| {
                event.prevent_default();
                if event.type_() == "pointerdown" {
                    let _ = target.focus();
                }
                let kind = match event.type_().as_str() {
                    "pointerdown" => PointerEventKind::Down(pointer_button(event.button())),
                    "pointerup" => PointerEventKind::Up(pointer_button(event.button())),
                    "pointercancel" => PointerEventKind::Cancel,
                    _ => PointerEventKind::Move,
                };
                app.borrow_mut().push_pointer(event, kind);
            },
        ));
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    Ok(())
}

fn register_wheel_events<State, Update, View>(
    canvas: &web_sys::HtmlCanvasElement,
    app: Rc<RefCell<WebRuntimeApp<State, Update, View>>>,
) -> Result<(), JsValue>
where
    State: 'static,
    Update: FnMut(&mut State, WidgetAction) + 'static,
    View: FnMut(&State, UiSize) -> UiDocument + 'static,
{
    let closure = Closure::<dyn FnMut(web_sys::WheelEvent)>::wrap(Box::new(
        move |event: web_sys::WheelEvent| {
            event.prevent_default();
            app.borrow_mut().push_wheel(event);
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
    View: FnMut(&State, UiSize) -> UiDocument + 'static,
{
    for (event_name, pressed) in [("keydown", true), ("keyup", false)] {
        let app = app.clone();
        let closure = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
            move |event: web_sys::KeyboardEvent| {
                if key_code(&event).is_some() {
                    event.prevent_default();
                }
                app.borrow_mut().push_key(event, pressed);
            },
        ));
        window.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    Ok(())
}

fn start_animation_loop<State, Update, View>(
    app: Rc<RefCell<WebRuntimeApp<State, Update, View>>>,
) -> Result<(), JsValue>
where
    State: 'static,
    Update: FnMut(&mut State, WidgetAction) + 'static,
    View: FnMut(&State, UiSize) -> UiDocument + 'static,
{
    let callback = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
    let next_callback = callback.clone();
    *next_callback.borrow_mut() = Some(Closure::<dyn FnMut(f64)>::wrap(Box::new(
        move |timestamp_ms| {
            if let Err(error) = app.borrow_mut().render(timestamp_ms) {
                web_sys::console::error_1(&error);
                let options = &app.borrow().options;
                if let Some(status_id) = options.status_id.as_deref() {
                    set_status(
                        status_id,
                        &format!("Render failed: {}", web_message(&error)),
                    );
                }
            }
            if let Some(callback) = callback.borrow().as_ref() {
                let _ = request_animation_frame(callback);
            }
        },
    )));
    if let Some(callback) = next_callback.borrow().as_ref() {
        request_animation_frame(callback)?;
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
    let width = rect
        .width()
        .max(window.inner_width()?.as_f64().unwrap_or(900.0))
        .max(1.0) as f32;
    let height = rect
        .height()
        .max(window.inner_height()?.as_f64().unwrap_or(760.0))
        .max(1.0) as f32;
    let dpi_scale = window.device_pixel_ratio().max(1.0) as f32;
    let pixel_size = PixelSize::new(
        (width * dpi_scale).ceil().max(1.0) as u32,
        (height * dpi_scale).ceil().max(1.0) as u32,
    );
    Ok((UiSize::new(width, height), pixel_size, dpi_scale))
}

fn pointer_position(canvas: &web_sys::HtmlCanvasElement, client_x: i32, client_y: i32) -> UiPoint {
    let rect = canvas.get_bounding_client_rect();
    UiPoint::new(
        client_x as f32 - rect.left() as f32,
        client_y as f32 - rect.top() as f32,
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
    let delta = UiPoint::new(-event.delta_x() as f32, -event.delta_y() as f32);
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

fn resolve_target(
    event: &RawInputEvent,
    state: &HostInteractionState,
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

fn request_animation_frame(callback: &Closure<dyn FnMut(f64)>) -> Result<i32, JsValue> {
    browser_window()?.request_animation_frame(callback.as_ref().unchecked_ref())
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
    if let Some(status) = document.get_element_by_id(status_id) {
        status.set_text_content(Some(message));
    }
}

fn layout_web_error(error: taffy::TaffyError) -> JsValue {
    web_error(format!("layout failed: {error}"))
}

fn render_web_error(error: crate::RenderError) -> JsValue {
    web_error(format!("render failed: {error}"))
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
