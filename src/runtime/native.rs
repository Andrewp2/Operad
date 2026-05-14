//! Native window runner for the default WGPU/winit path.

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::host::process_host_frame_input_with_target_resolver;
use crate::input::{
    PointerButton, PointerButtons, PointerEventKind, RawInputEvent, RawKeyboardEvent,
    RawPointerEvent, RawTextInputEvent, RawWheelEvent,
};
use crate::platform::PixelSize;
use crate::{
    process_document_frame, AccessibilityRole, ApproxTextMeasurer, CanvasRenderOutput,
    CanvasRenderReport, CanvasRenderRequest, DirtyRegionSet, EmptyResourceResolver,
    HostDocumentFrameState, HostFrameOutput, HostNodeInteraction, KeyCode, KeyModifiers,
    RenderError, RenderTarget, RendererAdapter, UiDocument, UiFocusState, UiInputEvent, UiNodeId,
    UiPoint, UiSize, WgpuCanvasContext, WgpuSurfaceRenderer, WheelDeltaUnit, WheelPhase,
    WidgetAction, WidgetActionBinding, WidgetActionQueue, WidgetValueEditPhase,
};

pub type NativeWindowResult<T = ()> = Result<T, Box<dyn Error>>;

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
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = NativeWindowApp::new(options, state, update, view, canvas_renderers);
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        Err(error.into())
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
    frame_state: HostDocumentFrameState,
    scroll_offsets: HashMap<String, UiPoint>,
    text_measurer: ApproxTextMeasurer,
    pending_input: Vec<RawInputEvent>,
    cursor: Option<UiPoint>,
    modifiers: KeyModifiers,
    buttons: PointerButtons,
    start: Instant,
    last_tick: Instant,
    error: Option<String>,
}

impl<State, Update, View> NativeWindowApp<State, Update, View> {
    fn new(
        options: NativeWindowOptions,
        state: State,
        update: Update,
        view: View,
        canvas_renderers: NativeWgpuCanvasRenderRegistry<State>,
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
            frame_state: HostDocumentFrameState::new(),
            scroll_offsets: HashMap::new(),
            text_measurer: ApproxTextMeasurer,
            pending_input: Vec::new(),
            cursor: None,
            modifiers: KeyModifiers::NONE,
            buttons: PointerButtons::NONE,
            start: Instant::now(),
            last_tick: Instant::now(),
            error: None,
        }
    }

    fn init_window(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> NativeWindowResult {
        let mut attributes = winit::window::Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_inner_size(logical_size(self.options.size))
            .with_visible(true);
        if let Some(min_size) = self.options.min_size {
            attributes = attributes.with_min_inner_size(logical_size(min_size));
        }
        let window = Arc::new(event_loop.create_window(attributes)?);
        let size = nonzero_window_size(window.inner_size());

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
        }))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("native-window-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
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
            let scale = self.dpi_scale();
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

    fn render(&mut self) -> NativeWindowResult
    where
        Update: FnMut(&mut State, WidgetAction),
        View: FnMut(&State, UiSize) -> UiDocument,
    {
        let Some(viewport) = self.viewport() else {
            return Ok(());
        };
        self.dispatch_tick_if_due();
        let raw_input = std::mem::take(&mut self.pending_input);
        let mut document = self.build_document(viewport)?;
        let mut host_request = self.frame_state.host_frame_request(viewport);
        host_request.raw_input = raw_input;
        let host_output =
            process_host_frame_input_with_target_resolver(host_request, |event, state| {
                resolve_target(event, state, &document)
            });
        let frame_request = self.frame_state.document_frame_request(
            viewport,
            RenderTarget::window(self.options.title.clone(), viewport),
            host_output,
        );
        let frame = process_document_frame(&mut document, &mut self.text_measurer, frame_request)?;
        self.capture_scroll_offsets(&document);
        let actions = collect_widget_actions(&document, &frame);
        self.frame_state.apply_document_frame_output(&frame);

        let frame = if actions.is_empty() {
            frame
        } else {
            for action in actions {
                (self.update)(&mut self.state, action);
            }
            let mut document = self.build_document(viewport)?;
            let frame_request = self.frame_state.document_frame_request(
                viewport,
                RenderTarget::window(self.options.title.clone(), viewport),
                HostFrameOutput::new(self.frame_state.interaction.clone()),
            );
            let frame =
                process_document_frame(&mut document, &mut self.text_measurer, frame_request)?;
            self.capture_scroll_offsets(&document);
            self.frame_state.apply_document_frame_output(&frame);
            frame
        };

        let Some(renderer) = self.renderer.as_mut() else {
            return Ok(());
        };
        let canvas_report = self.canvas_renderers.render_frame_canvases(
            &mut self.state,
            renderer,
            &frame.render_request,
        );
        if let Some(error) = canvas_report.first_failure().cloned() {
            return Err(error.into());
        }
        let repaint_requested = canvas_report.repaint_requested();
        renderer.render_frame(frame.render_request, &EmptyResourceResolver)?;
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
        document.set_dpi_scale(self.dpi_scale());
        restore_scroll_offsets(&mut document, &self.scroll_offsets);
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
                .insert(scroll_offset_key(document, id), scroll.offset);
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
            winit::event::WindowEvent::CloseRequested | winit::event::WindowEvent::Destroyed => {
                event_loop.exit();
            }
            winit::event::WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.request_redraw();
                }
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                let scale = self.dpi_scale();
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
                    let scale = self.dpi_scale();
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
                if event.state != winit::event::ElementState::Pressed {
                    return;
                }
                if let Some(key) = key_code(&event, self.modifiers) {
                    self.push_input(RawInputEvent::Keyboard(
                        RawKeyboardEvent::press(key, self.modifiers, self.timestamp_millis())
                            .repeat(event.repeat),
                    ));
                }
                if !self.modifiers.ctrl && !self.modifiers.meta {
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

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.options.tick_action.is_none() {
            return;
        }
        let next_tick = self.last_tick + self.options.tick_interval;
        let now = Instant::now();
        if now >= next_tick {
            self.request_redraw();
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(next_tick));
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

fn logical_size(size: UiSize) -> winit::dpi::LogicalSize<f64> {
    winit::dpi::LogicalSize::new(size.width.max(1.0) as f64, size.height.max(1.0) as f64)
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
        let Some(offset) = offsets.get(&scroll_offset_key(document, id)).copied() else {
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

fn scroll_offset_key(document: &UiDocument, id: UiNodeId) -> String {
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
        offsets.insert(
            scroll_offset_key(&document, scroll),
            UiPoint::new(0.0, 120.0),
        );

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
}
