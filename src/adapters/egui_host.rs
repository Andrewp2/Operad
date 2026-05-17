//! Feature-gated egui host adapters.
//!
//! This module keeps egui integration at the boundary of the crate by
//! translating egui input into Operad's renderer-neutral raw input contracts.

use std::fmt;

use crate::accessibility::{
    AccessibilityAdapterRequest, AccessibilityAdapterRequestPlan, AccessibilityAdapterResponse,
    AccessibilityCapabilities,
};
use crate::commands::CommandRegistry;
use crate::host::{
    HostAdapter, HostAdapterError, HostCommandDispatch, HostDocumentFrameOutput, HostFrameOutput,
    HostFrameRequest, HostInteractionState,
};
use crate::input::{
    PointerButton, PointerButtons, PointerEventKind, RawInputEvent, RawKeyboardEvent,
    RawPointerEvent, RawTextInputEvent, RawWheelEvent,
};
use crate::platform::{
    BackendAdapterKind, BackendCapabilities, ClipboardRequest, CursorGrabMode, CursorRequest,
    CursorShape, InputCapabilities, LayerCapabilities, PlatformRequest, PlatformRequestIdAllocator,
    PlatformResponse, PlatformServiceCapabilities, PlatformServiceRequest, PlatformServiceResponse,
    RenderingCapabilities, RepaintRequest, ResourceCapabilities, ResourceDomain, ResourceHandle,
    ResourceKind,
};
use crate::renderer::{PixelRect, ResourceFormat, ResourceUpdate};
use crate::{FocusDirection, KeyCode, KeyModifiers, UiPoint, UiSize};

#[derive(Debug, Clone)]
pub struct EguiHostAdapter {
    input: EguiInputAdapter,
    commands: CommandRegistry,
    wheel_line_size: f32,
    capabilities: BackendCapabilities,
}

impl EguiHostAdapter {
    pub fn new() -> Self {
        Self {
            input: EguiInputAdapter::new(),
            commands: CommandRegistry::new(),
            wheel_line_size: 16.0,
            capabilities: egui_host_capabilities(),
        }
    }

    pub fn with_command_registry(mut self, commands: CommandRegistry) -> Self {
        self.commands = commands;
        self
    }

    pub fn with_wheel_line_size(mut self, wheel_line_size: f32) -> Self {
        if wheel_line_size.is_finite() && wheel_line_size > 0.0 {
            self.wheel_line_size = wheel_line_size;
        }
        self
    }

    pub fn input_adapter(&self) -> &EguiInputAdapter {
        &self.input
    }

    pub fn input_adapter_mut(&mut self) -> &mut EguiInputAdapter {
        &mut self.input
    }

    pub fn command_registry(&self) -> &CommandRegistry {
        &self.commands
    }

    pub fn command_registry_mut(&mut self) -> &mut CommandRegistry {
        &mut self.commands
    }

    pub fn translate_host_frame_request<'a>(
        &mut self,
        viewport: UiSize,
        state: HostInteractionState,
        events: impl IntoIterator<Item = &'a egui::Event>,
        timestamp_millis: u64,
    ) -> HostFrameRequest {
        let raw_input = self.input.translate_events(events, timestamp_millis);
        let mut request = HostFrameRequest::new(viewport, state);
        request.raw_input = raw_input;
        request
    }

    pub fn process_egui_events<'a>(
        &mut self,
        viewport: UiSize,
        state: HostInteractionState,
        events: impl IntoIterator<Item = &'a egui::Event>,
        timestamp_millis: u64,
    ) -> Result<HostFrameOutput, HostAdapterError> {
        let request = self.translate_host_frame_request(viewport, state, events, timestamp_millis);
        self.process_frame(request)
    }
}

impl Default for EguiHostAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl HostAdapter for EguiHostAdapter {
    fn capabilities(&self) -> BackendCapabilities {
        self.capabilities.clone()
    }

    fn process_frame(
        &mut self,
        request: HostFrameRequest,
    ) -> Result<HostFrameOutput, HostAdapterError> {
        let mut state = request.state;
        let mut output = HostFrameOutput::new(state.clone());

        for response in request.platform_responses {
            if let PlatformResponse::TextIme(response) = &response.response {
                state.apply_text_ime_response(response);
            }
            output.platform_responses.push(response);
        }

        for event in request.raw_input {
            if let Some(ui_event) =
                event.to_ui_input_event_with_wheel_scale(self.wheel_line_size, request.viewport)
            {
                output.ui_events.push(ui_event);
            }

            if let RawInputEvent::Keyboard(keyboard) = event {
                if keyboard.pressed {
                    let route = state.route_key(keyboard.key, keyboard.modifiers, &self.commands);
                    if let Some(command) = route.command.clone() {
                        output.commands.push(HostCommandDispatch {
                            command,
                            shortcut: route.shortcut,
                            target: route.target,
                        });
                    }
                }
            }
        }

        output.state = state;
        Ok(output)
    }
}

pub fn egui_host_capabilities() -> BackendCapabilities {
    BackendCapabilities::new("egui-host")
        .adapter(BackendAdapterKind::Egui)
        .resources(ResourceCapabilities {
            images: true,
            icons: true,
            textures: true,
            thumbnails: true,
            tinted_icons: true,
            partial_texture_updates: true,
        })
        .layers(LayerCapabilities::STANDARD)
        .input(InputCapabilities::STANDARD)
        .services(PlatformServiceCapabilities {
            clipboard_write: true,
            open_url: true,
            cursor_shape: true,
            cursor_visible: true,
            cursor_grab: true,
            cursor_confine: true,
            repaint: true,
            ..PlatformServiceCapabilities::NONE
        })
        .rendering(RenderingCapabilities::STANDARD)
        .accessibility(AccessibilityCapabilities {
            clipboard: true,
            text_ime: true,
            ..AccessibilityCapabilities::NONE
        })
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EguiAccessibilityOutputPlan {
    pub plan: AccessibilityAdapterRequestPlan,
}

impl EguiAccessibilityOutputPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_requests<'a>(
        requests: impl IntoIterator<Item = &'a AccessibilityAdapterRequest>,
    ) -> Self {
        let mut plan = Self::new();
        for request in requests {
            plan.push_request(request);
        }
        plan
    }

    pub fn from_document_frame_output(output: &HostDocumentFrameOutput) -> Self {
        Self::from_requests(output.accessibility_requests.iter())
    }

    pub fn push_request(&mut self, request: &AccessibilityAdapterRequest) -> bool {
        self.plan
            .push_request(request, egui_host_capabilities().accessibility)
    }

    pub fn is_fully_supported(&self) -> bool {
        self.plan.is_fully_supported()
    }

    pub fn supported_requests(&self) -> &[AccessibilityAdapterRequest] {
        &self.plan.supported_requests
    }

    pub fn responses(&self) -> &[AccessibilityAdapterResponse] {
        &self.plan.unsupported_responses
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EguiInputAdapter {
    buttons: PointerButtons,
    last_pointer_pos: Option<UiPoint>,
    modifiers: KeyModifiers,
}

impl EguiInputAdapter {
    pub const fn new() -> Self {
        Self {
            buttons: PointerButtons::NONE,
            last_pointer_pos: None,
            modifiers: KeyModifiers::NONE,
        }
    }

    pub const fn buttons(&self) -> PointerButtons {
        self.buttons
    }

    pub const fn last_pointer_pos(&self) -> Option<UiPoint> {
        self.last_pointer_pos
    }

    pub const fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    pub fn translate_events<'a>(
        &mut self,
        events: impl IntoIterator<Item = &'a egui::Event>,
        timestamp_millis: u64,
    ) -> Vec<RawInputEvent> {
        events
            .into_iter()
            .flat_map(|event| self.translate_event(event, timestamp_millis))
            .collect()
    }

    pub fn translate_event(
        &mut self,
        event: &egui::Event,
        timestamp_millis: u64,
    ) -> Vec<RawInputEvent> {
        match event {
            egui::Event::PointerMoved(pos) => {
                let position = ui_point(*pos);
                self.last_pointer_pos = Some(position);
                vec![RawInputEvent::Pointer(
                    RawPointerEvent::new(PointerEventKind::Move, position, timestamp_millis)
                        .buttons(self.buttons)
                        .modifiers(self.modifiers),
                )]
            }
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } => {
                let position = ui_point(*pos);
                let button = egui_pointer_button(*button);
                self.last_pointer_pos = Some(position);
                self.modifiers = egui_modifiers(*modifiers);
                self.buttons = if *pressed {
                    self.buttons.with(button)
                } else {
                    self.buttons.without(button)
                };
                let kind = if *pressed {
                    PointerEventKind::Down(button)
                } else {
                    PointerEventKind::Up(button)
                };
                vec![RawInputEvent::Pointer(
                    RawPointerEvent::new(kind, position, timestamp_millis)
                        .buttons(self.buttons)
                        .modifiers(self.modifiers),
                )]
            }
            egui::Event::PointerGone => {
                let Some(position) = self.last_pointer_pos.take() else {
                    return Vec::new();
                };
                let event = RawInputEvent::Pointer(
                    RawPointerEvent::new(PointerEventKind::Cancel, position, timestamp_millis)
                        .buttons(self.buttons)
                        .modifiers(self.modifiers),
                );
                self.buttons = PointerButtons::NONE;
                vec![event]
            }
            egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
            } => {
                self.modifiers = egui_modifiers(*modifiers);
                let position = self.last_pointer_pos.unwrap_or(UiPoint::new(0.0, 0.0));
                vec![RawInputEvent::Wheel(
                    egui_wheel_event(*unit, position, *delta, timestamp_millis)
                        .modifiers(self.modifiers),
                )]
            }
            egui::Event::Key {
                key,
                pressed,
                repeat,
                modifiers,
                ..
            } => {
                self.modifiers = egui_modifiers(*modifiers);
                if *pressed && *key == egui::Key::Tab {
                    let direction = if modifiers.shift {
                        FocusDirection::Previous
                    } else {
                        FocusDirection::Next
                    };
                    return vec![RawInputEvent::Focus(direction)];
                }
                let Some(key) = egui_key(*key) else {
                    return Vec::new();
                };
                let event = if *pressed {
                    RawKeyboardEvent::press(key, self.modifiers, timestamp_millis)
                } else {
                    RawKeyboardEvent::release(key, self.modifiers, timestamp_millis)
                }
                .repeat(*repeat);
                vec![RawInputEvent::Keyboard(event)]
            }
            egui::Event::Text(text) | egui::Event::Paste(text) => {
                vec![RawInputEvent::Text(RawTextInputEvent::new(
                    text.clone(),
                    timestamp_millis,
                ))]
            }
            egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                vec![RawInputEvent::Text(RawTextInputEvent::new(
                    text.clone(),
                    timestamp_millis,
                ))]
            }
            _ => Vec::new(),
        }
    }
}

impl Default for EguiInputAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct EguiPlatformOutputPlan {
    pub platform_output: egui::PlatformOutput,
    pub viewport_commands: Vec<egui::ViewportCommand>,
    pub repaint_requests: Vec<RepaintRequest>,
    pub unsupported_requests: Vec<PlatformRequest>,
    pub unsupported_service_requests: Vec<PlatformServiceRequest>,
}

impl EguiPlatformOutputPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_requests<'a>(requests: impl IntoIterator<Item = &'a PlatformRequest>) -> Self {
        let mut plan = Self::new();
        for request in requests {
            plan.push_request(request);
        }
        plan
    }

    pub fn from_service_requests<'a>(
        requests: impl IntoIterator<Item = &'a PlatformServiceRequest>,
    ) -> Self {
        let mut plan = Self::new();
        for request in requests {
            plan.push_service_request(request);
        }
        plan
    }

    pub fn from_document_frame_output(
        output: &HostDocumentFrameOutput,
        allocator: &mut PlatformRequestIdAllocator,
    ) -> Self {
        let requests = output.platform_service_requests(allocator);
        Self::from_service_requests(requests.iter())
    }

    pub fn push_document_frame_output(
        &mut self,
        output: &HostDocumentFrameOutput,
        allocator: &mut PlatformRequestIdAllocator,
    ) {
        for request in output.platform_service_requests(allocator) {
            self.push_service_request(&request);
        }
    }

    pub fn push_service_request(&mut self, request: &PlatformServiceRequest) -> bool {
        let supported = self.push_request(&request.request);
        if !supported {
            self.unsupported_service_requests.push(request.clone());
        }
        supported
    }

    pub fn push_request(&mut self, request: &PlatformRequest) -> bool {
        match request {
            PlatformRequest::Clipboard(ClipboardRequest::WriteText(text)) => {
                self.platform_output
                    .commands
                    .push(egui::OutputCommand::CopyText(text.clone()));
                true
            }
            PlatformRequest::Clipboard(ClipboardRequest::Clear) => {
                self.platform_output
                    .commands
                    .push(egui::OutputCommand::CopyText(String::new()));
                true
            }
            PlatformRequest::OpenUrl(request) => {
                self.platform_output
                    .commands
                    .push(egui::OutputCommand::OpenUrl(egui::OpenUrl {
                        url: request.url.clone(),
                        new_tab: request.new_window,
                    }));
                true
            }
            PlatformRequest::Cursor(CursorRequest::SetShape(shape)) => {
                self.platform_output.cursor_icon = egui_cursor_icon(*shape);
                true
            }
            PlatformRequest::Cursor(CursorRequest::SetVisible(visible)) => {
                self.platform_output.cursor_icon = if *visible {
                    egui::CursorIcon::Default
                } else {
                    egui::CursorIcon::None
                };
                self.viewport_commands
                    .push(egui::ViewportCommand::CursorVisible(*visible));
                true
            }
            PlatformRequest::Cursor(CursorRequest::SetGrab(mode)) => {
                self.viewport_commands
                    .push(egui::ViewportCommand::CursorGrab(egui_cursor_grab(*mode)));
                true
            }
            PlatformRequest::Cursor(CursorRequest::Confine(_)) => {
                self.viewport_commands
                    .push(egui::ViewportCommand::CursorGrab(
                        egui::CursorGrab::Confined,
                    ));
                true
            }
            PlatformRequest::Cursor(CursorRequest::ReleaseConfine) => {
                self.viewport_commands
                    .push(egui::ViewportCommand::CursorGrab(egui::CursorGrab::None));
                true
            }
            PlatformRequest::Repaint(request) => {
                self.repaint_requests.push(request.clone());
                true
            }
            _ => {
                self.unsupported_requests.push(request.clone());
                false
            }
        }
    }

    pub fn is_fully_supported(&self) -> bool {
        self.unsupported_requests.is_empty()
    }

    pub fn unsupported_service_responses(&self) -> Vec<PlatformServiceResponse> {
        self.unsupported_service_requests
            .iter()
            .map(PlatformServiceRequest::unsupported_response)
            .collect()
    }
}

impl fmt::Debug for EguiPlatformOutputPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EguiPlatformOutputPlan")
            .field("commands", &self.platform_output.commands)
            .field("cursor_icon", &self.platform_output.cursor_icon)
            .field("viewport_commands", &self.viewport_commands)
            .field("repaint_requests", &self.repaint_requests)
            .field("unsupported_requests", &self.unsupported_requests)
            .field(
                "unsupported_service_requests",
                &self.unsupported_service_requests,
            )
            .finish()
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct EguiTextureDeltaPlan {
    pub textures_delta: egui::TexturesDelta,
    pub rejected_updates: Vec<ResourceUpdate>,
}

impl EguiTextureDeltaPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_resource_updates<'a>(
        updates: impl IntoIterator<Item = &'a ResourceUpdate>,
        options: egui::TextureOptions,
    ) -> Self {
        let mut plan = Self::new();
        for update in updates {
            plan.push_update(update, options);
        }
        plan
    }

    pub fn push_update(&mut self, update: &ResourceUpdate, options: egui::TextureOptions) -> bool {
        let Some(image) = egui_color_image_for_update(update) else {
            self.rejected_updates.push(update.clone());
            return false;
        };
        let texture = egui_texture_id_for_resource(&update.descriptor.handle);
        let image = egui::ImageData::Color(std::sync::Arc::new(image));
        let delta = match update.dirty_rect {
            Some(rect) => egui::epaint::ImageDelta::partial(
                [
                    usize::try_from(rect.x).unwrap_or(usize::MAX),
                    usize::try_from(rect.y).unwrap_or(usize::MAX),
                ],
                image,
                options,
            ),
            None => egui::epaint::ImageDelta::full(image, options),
        };
        self.textures_delta.set.push((texture, delta));
        true
    }

    pub fn is_fully_supported(&self) -> bool {
        self.rejected_updates.is_empty()
    }
}

impl fmt::Debug for EguiTextureDeltaPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EguiTextureDeltaPlan")
            .field("textures_delta", &self.textures_delta)
            .field("rejected_updates", &self.rejected_updates)
            .finish()
    }
}

pub fn egui_texture_id_for_resource(handle: &ResourceHandle) -> egui::TextureId {
    egui::TextureId::User(stable_resource_hash(handle))
}

pub fn egui_cursor_icon(shape: CursorShape) -> egui::CursorIcon {
    match shape {
        CursorShape::Default => egui::CursorIcon::Default,
        CursorShape::Pointer => egui::CursorIcon::PointingHand,
        CursorShape::Text => egui::CursorIcon::Text,
        CursorShape::Crosshair => egui::CursorIcon::Crosshair,
        CursorShape::Grab => egui::CursorIcon::Grab,
        CursorShape::Grabbing => egui::CursorIcon::Grabbing,
        CursorShape::Move => egui::CursorIcon::Move,
        CursorShape::NotAllowed => egui::CursorIcon::NotAllowed,
        CursorShape::Wait => egui::CursorIcon::Wait,
        CursorShape::Progress => egui::CursorIcon::Progress,
        CursorShape::ResizeHorizontal => egui::CursorIcon::ResizeHorizontal,
        CursorShape::ResizeVertical => egui::CursorIcon::ResizeVertical,
        CursorShape::ResizeNorthEastSouthWest => egui::CursorIcon::ResizeNeSw,
        CursorShape::ResizeNorthWestSouthEast => egui::CursorIcon::ResizeNwSe,
        CursorShape::ZoomIn => egui::CursorIcon::ZoomIn,
        CursorShape::ZoomOut => egui::CursorIcon::ZoomOut,
    }
}

pub fn egui_cursor_grab(mode: CursorGrabMode) -> egui::CursorGrab {
    match mode {
        CursorGrabMode::None => egui::CursorGrab::None,
        CursorGrabMode::Confined => egui::CursorGrab::Confined,
        CursorGrabMode::Locked => egui::CursorGrab::Locked,
    }
}

fn egui_color_image_for_update(update: &ResourceUpdate) -> Option<egui::ColorImage> {
    if !update.has_expected_byte_len() || !update.dirty_rect_is_valid() {
        return None;
    }
    let size = update.dirty_rect.map(pixel_rect_size).unwrap_or_else(|| {
        pixel_size(update.descriptor.size.width, update.descriptor.size.height)
    })?;
    if size[0] == 0 || size[1] == 0 {
        return None;
    }
    match update.descriptor.format {
        ResourceFormat::Rgba8 => Some(egui::ColorImage::from_rgba_unmultiplied(
            size,
            &update.bytes,
        )),
        ResourceFormat::Bgra8 => {
            let mut rgba = Vec::with_capacity(update.bytes.len());
            for bgra in update.bytes.chunks_exact(4) {
                rgba.extend_from_slice(&[bgra[2], bgra[1], bgra[0], bgra[3]]);
            }
            Some(egui::ColorImage::from_rgba_unmultiplied(size, &rgba))
        }
        ResourceFormat::Alpha8 => {
            let pixels = update
                .bytes
                .iter()
                .map(|alpha| egui::Color32::from_rgba_unmultiplied(255, 255, 255, *alpha))
                .collect::<Vec<_>>();
            Some(egui::ColorImage::new(size, pixels))
        }
    }
}

fn pixel_rect_size(rect: PixelRect) -> Option<[usize; 2]> {
    pixel_size(rect.width, rect.height)
}

fn pixel_size(width: u32, height: u32) -> Option<[usize; 2]> {
    Some([usize::try_from(width).ok()?, usize::try_from(height).ok()?])
}

fn stable_resource_hash(handle: &ResourceHandle) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    hash = fnv1a(hash, &[resource_domain_byte(handle.id().domain)]);
    hash = fnv1a(hash, &[resource_kind_byte(handle.kind())]);
    fnv1a(hash, handle.id().key.as_bytes())
}

fn resource_domain_byte(domain: ResourceDomain) -> u8 {
    match domain {
        ResourceDomain::BuiltIn => 1,
        ResourceDomain::App => 2,
        ResourceDomain::Host => 3,
    }
}

fn resource_kind_byte(kind: ResourceKind) -> u8 {
    match kind {
        ResourceKind::Image => 1,
        ResourceKind::Icon => 2,
        ResourceKind::Texture => 3,
        ResourceKind::Thumbnail => 4,
    }
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    const PRIME: u64 = 0x100000001b3;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

pub fn egui_modifiers(modifiers: egui::Modifiers) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.shift,
        ctrl: modifiers.ctrl || (modifiers.command && !modifiers.mac_cmd),
        alt: modifiers.alt,
        meta: modifiers.mac_cmd,
    }
}

pub fn egui_pointer_button(button: egui::PointerButton) -> PointerButton {
    match button {
        egui::PointerButton::Primary => PointerButton::Primary,
        egui::PointerButton::Secondary => PointerButton::Secondary,
        egui::PointerButton::Middle => PointerButton::Auxiliary,
        egui::PointerButton::Extra1 => PointerButton::Back,
        egui::PointerButton::Extra2 => PointerButton::Forward,
    }
}

pub fn egui_key(key: egui::Key) -> Option<KeyCode> {
    Some(match key {
        egui::Key::ArrowDown => KeyCode::ArrowDown,
        egui::Key::ArrowLeft => KeyCode::ArrowLeft,
        egui::Key::ArrowRight => KeyCode::ArrowRight,
        egui::Key::ArrowUp => KeyCode::ArrowUp,
        egui::Key::Escape => KeyCode::Escape,
        egui::Key::Tab => KeyCode::Tab,
        egui::Key::Backspace => KeyCode::Backspace,
        egui::Key::Enter => KeyCode::Enter,
        egui::Key::Delete => KeyCode::Delete,
        egui::Key::Home => KeyCode::Home,
        egui::Key::End => KeyCode::End,
        egui::Key::F10 => KeyCode::F10,
        egui::Key::Space => KeyCode::Character(' '),
        egui::Key::Colon => KeyCode::Character(':'),
        egui::Key::Comma => KeyCode::Character(','),
        egui::Key::Backslash => KeyCode::Character('\\'),
        egui::Key::Slash => KeyCode::Character('/'),
        egui::Key::Pipe => KeyCode::Character('|'),
        egui::Key::Questionmark => KeyCode::Character('?'),
        egui::Key::Exclamationmark => KeyCode::Character('!'),
        egui::Key::OpenBracket => KeyCode::Character('['),
        egui::Key::CloseBracket => KeyCode::Character(']'),
        egui::Key::OpenCurlyBracket => KeyCode::Character('{'),
        egui::Key::CloseCurlyBracket => KeyCode::Character('}'),
        egui::Key::Backtick => KeyCode::Character('`'),
        egui::Key::Minus => KeyCode::Character('-'),
        egui::Key::Period => KeyCode::Character('.'),
        egui::Key::Plus => KeyCode::Character('+'),
        egui::Key::Equals => KeyCode::Character('='),
        egui::Key::Semicolon => KeyCode::Character(';'),
        egui::Key::Quote => KeyCode::Character('\''),
        egui::Key::Num0 => KeyCode::Character('0'),
        egui::Key::Num1 => KeyCode::Character('1'),
        egui::Key::Num2 => KeyCode::Character('2'),
        egui::Key::Num3 => KeyCode::Character('3'),
        egui::Key::Num4 => KeyCode::Character('4'),
        egui::Key::Num5 => KeyCode::Character('5'),
        egui::Key::Num6 => KeyCode::Character('6'),
        egui::Key::Num7 => KeyCode::Character('7'),
        egui::Key::Num8 => KeyCode::Character('8'),
        egui::Key::Num9 => KeyCode::Character('9'),
        egui::Key::A => KeyCode::Character('a'),
        egui::Key::B => KeyCode::Character('b'),
        egui::Key::C => KeyCode::Character('c'),
        egui::Key::D => KeyCode::Character('d'),
        egui::Key::E => KeyCode::Character('e'),
        egui::Key::F => KeyCode::Character('f'),
        egui::Key::G => KeyCode::Character('g'),
        egui::Key::H => KeyCode::Character('h'),
        egui::Key::I => KeyCode::Character('i'),
        egui::Key::J => KeyCode::Character('j'),
        egui::Key::K => KeyCode::Character('k'),
        egui::Key::L => KeyCode::Character('l'),
        egui::Key::M => KeyCode::Character('m'),
        egui::Key::N => KeyCode::Character('n'),
        egui::Key::O => KeyCode::Character('o'),
        egui::Key::P => KeyCode::Character('p'),
        egui::Key::Q => KeyCode::Character('q'),
        egui::Key::R => KeyCode::Character('r'),
        egui::Key::S => KeyCode::Character('s'),
        egui::Key::T => KeyCode::Character('t'),
        egui::Key::U => KeyCode::Character('u'),
        egui::Key::V => KeyCode::Character('v'),
        egui::Key::W => KeyCode::Character('w'),
        egui::Key::X => KeyCode::Character('x'),
        egui::Key::Y => KeyCode::Character('y'),
        egui::Key::Z => KeyCode::Character('z'),
        _ => return None,
    })
}

fn egui_wheel_event(
    unit: egui::MouseWheelUnit,
    position: UiPoint,
    delta: egui::Vec2,
    timestamp_millis: u64,
) -> RawWheelEvent {
    let delta = UiPoint::new(delta.x, delta.y);
    match unit {
        egui::MouseWheelUnit::Point => RawWheelEvent::pixels(position, delta, timestamp_millis),
        egui::MouseWheelUnit::Line => RawWheelEvent::lines(position, delta, timestamp_millis),
        egui::MouseWheelUnit::Page => RawWheelEvent::pages(position, delta, timestamp_millis),
    }
}

fn ui_point(pos: egui::Pos2) -> UiPoint {
    UiPoint::new(pos.x, pos.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{
        AccessibilityAdapterRequest, AccessibilityAnnouncement, AccessibilityPreferences,
        AccessibilityRequestKind,
    };
    use crate::commands::{Command, CommandMeta, CommandScope, Shortcut};
    use crate::input::{WheelDeltaUnit, WheelPhase};
    use crate::platform::{
        ClipboardRequest, CursorRequest, CursorShape, FileDialogMode, FileDialogRequest,
        ImageHandle, LogicalPoint, LogicalRect, OpenUrlRequest, PixelSize, PlatformRequest,
        PlatformRequestId, PlatformRequestIdAllocator, PlatformResponse, PlatformServiceRequest,
        PlatformServiceResponse, RepaintRequest, ResourceHandle, TextImeResponse, TextImeSession,
        TextInputId, TextureHandle,
    };
    use crate::renderer::{PixelRect, ResourceDescriptor, ResourceFormat, ResourceUpdate};
    use crate::testing::AccessibilityResponseAssertions;
    use crate::{
        process_document_frame, ApproxTextMeasurer, CanvasContent, CanvasInteractionPolicy,
        HostDocumentFrameRequest, HostFrameOutput, InputBehavior, LayoutStyle, RenderTarget,
        UiContent, UiDocument, UiInputEvent, UiNode, UiNodeId, UiNodeStyle, UiSize,
    };
    use std::time::Duration;
    use taffy::prelude::{Dimension, Display, FlexDirection, Size as TaffySize, Style};

    fn key_event(
        key: egui::Key,
        pressed: bool,
        repeat: bool,
        modifiers: egui::Modifiers,
    ) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat,
            modifiers,
        }
    }

    #[test]
    fn egui_host_adapter_processes_events_commands_and_ime_responses() {
        let mut commands = CommandRegistry::new();
        commands
            .register(Command::new(CommandMeta::new("save", "Save")))
            .unwrap();
        commands
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('s'), "save")
            .unwrap();

        let mut adapter = EguiHostAdapter::new()
            .with_command_registry(commands)
            .with_wheel_line_size(24.0);
        let mut state = HostInteractionState {
            focused: Some(UiNodeId(9)),
            ..HostInteractionState::default()
        };
        let ime_input = TextInputId::new("node:9");
        state.text_ime = Some(TextImeSession::new(
            ime_input.clone(),
            LogicalRect::new(10.0, 12.0, 2.0, 18.0),
        ));
        state.text_target = Some(UiNodeId(9));

        let response = PlatformServiceResponse::new(
            PlatformRequestId::new(11),
            PlatformResponse::TextIme(TextImeResponse::Deactivated {
                input: ime_input.clone(),
            }),
        );
        let mut request = adapter.translate_host_frame_request(
            UiSize::new(320.0, 200.0),
            state,
            [
                egui::Event::PointerMoved(egui::pos2(18.0, 22.0)),
                key_event(
                    egui::Key::S,
                    true,
                    false,
                    egui::Modifiers {
                        ctrl: true,
                        ..egui::Modifiers::default()
                    },
                ),
            ]
            .iter(),
            50,
        );
        request.platform_responses.push(response.clone());

        let output = adapter.process_frame(request).expect("egui host frame");

        assert_eq!(adapter.capabilities().adapter, BackendAdapterKind::Egui);
        assert!(adapter.capabilities().services.clipboard_write);
        assert_eq!(output.platform_responses, vec![response]);
        assert_eq!(output.ui_events.len(), 2);
        assert_eq!(output.commands.len(), 1);
        assert_eq!(output.commands[0].command.as_str(), "save");
        assert_eq!(output.commands[0].target, Some(UiNodeId(9)));
        assert_eq!(output.state.text_ime, None);
        assert_eq!(output.state.text_target, None);
    }

    #[test]
    fn egui_host_capabilities_report_accessibility_bridge_surface() {
        let capabilities = egui_host_capabilities();

        assert_eq!(capabilities.adapter, BackendAdapterKind::Egui);
        assert!(capabilities.accessibility.clipboard);
        assert!(capabilities.accessibility.text_ime);
        assert!(!capabilities.supports_accessibility(AccessibilityRequestKind::PublishTree));
        assert!(!capabilities.supports_accessibility(AccessibilityRequestKind::SetFocusTrap));
        assert!(!capabilities.accessibility.announcements);
    }

    #[test]
    fn egui_accessibility_output_plan_reports_unsupported_adapter_requests() {
        let mut document = UiDocument::new(fixed_style(120.0, 60.0));
        let button = document.add_child(
            document.root,
            UiNode::container("button", fixed_style(80.0, 32.0)).with_accessibility(
                crate::AccessibilityMeta::new(crate::AccessibilityRole::Button).label("Button"),
            ),
        );
        document
            .compute_layout(UiSize::new(120.0, 60.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let tree = document.accessibility_snapshot();
        let requests = vec![
            AccessibilityAdapterRequest::PublishTree {
                tree,
                focused: Some(button),
                preferences: AccessibilityPreferences::DEFAULT,
            },
            AccessibilityAdapterRequest::Announce(
                AccessibilityAnnouncement::polite("Ready").source(button),
            ),
        ];

        let plan = EguiAccessibilityOutputPlan::from_requests(&requests);

        assert!(!plan.is_fully_supported());
        assert!(plan.supported_requests().is_empty());
        assert_eq!(plan.responses().len(), 2);
        AccessibilityResponseAssertions::new(plan.responses())
            .require_unsupported(AccessibilityRequestKind::PublishTree)
            .expect("unsupported publish tree");
        AccessibilityResponseAssertions::new(plan.responses())
            .require_unsupported(AccessibilityRequestKind::Announce)
            .expect("unsupported announce");
    }

    fn fixed_style(width: f32, height: f32) -> UiNodeStyle {
        UiNodeStyle::from(LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: Dimension::length(width),
                height: Dimension::length(height),
            },
            ..Default::default()
        }))
    }

    #[test]
    fn egui_pointer_events_preserve_position_buttons_and_modifiers() {
        let mut adapter = EguiInputAdapter::new();
        let modifiers = egui::Modifiers {
            shift: true,
            ctrl: true,
            command: true,
            ..egui::Modifiers::NONE
        };
        let events = [
            egui::Event::PointerButton {
                pos: egui::Pos2::new(10.0, 20.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers,
            },
            egui::Event::PointerMoved(egui::Pos2::new(16.0, 24.0)),
            egui::Event::PointerButton {
                pos: egui::Pos2::new(16.0, 24.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers,
            },
        ];

        let translated = adapter.translate_events(events.iter(), 42);

        assert_eq!(translated.len(), 3);
        let RawInputEvent::Pointer(down) = translated[0] else {
            panic!("expected pointer down");
        };
        assert_eq!(down.kind, PointerEventKind::Down(PointerButton::Primary));
        assert_eq!(down.position, UiPoint::new(10.0, 20.0));
        assert!(down.buttons.contains(PointerButton::Primary));
        assert_eq!(
            down.modifiers,
            KeyModifiers {
                shift: true,
                ctrl: true,
                alt: false,
                meta: false,
            }
        );

        let RawInputEvent::Pointer(moved) = translated[1] else {
            panic!("expected pointer move");
        };
        assert_eq!(moved.kind, PointerEventKind::Move);
        assert_eq!(moved.position, UiPoint::new(16.0, 24.0));
        assert!(moved.buttons.contains(PointerButton::Primary));
        assert_eq!(moved.modifiers, down.modifiers);

        let RawInputEvent::Pointer(up) = translated[2] else {
            panic!("expected pointer up");
        };
        assert_eq!(up.kind, PointerEventKind::Up(PointerButton::Primary));
        assert_eq!(up.buttons, PointerButtons::NONE);
        assert_eq!(adapter.buttons(), PointerButtons::NONE);
        assert_eq!(adapter.last_pointer_pos(), Some(UiPoint::new(16.0, 24.0)));
    }

    #[test]
    fn egui_wheel_events_keep_units_and_last_pointer_position() {
        let mut adapter = EguiInputAdapter::new();
        adapter.translate_event(&egui::Event::PointerMoved(egui::Pos2::new(40.0, 50.0)), 1);
        let modifiers = egui::Modifiers {
            alt: true,
            ..egui::Modifiers::NONE
        };
        let events = [
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::Vec2::new(2.0, -4.0),
                modifiers,
            },
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::Vec2::new(1.0, -2.0),
                modifiers,
            },
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Page,
                delta: egui::Vec2::new(0.0, 1.0),
                modifiers,
            },
        ];

        let translated = adapter.translate_events(events.iter(), 9);

        let RawInputEvent::Wheel(point) = translated[0] else {
            panic!("expected point wheel");
        };
        assert_eq!(point.position, UiPoint::new(40.0, 50.0));
        assert_eq!(point.delta, UiPoint::new(2.0, -4.0));
        assert_eq!(point.unit, WheelDeltaUnit::Pixel);
        assert_eq!(point.phase, WheelPhase::Moved);
        assert!(point.modifiers.alt);

        let RawInputEvent::Wheel(line) = translated[1] else {
            panic!("expected line wheel");
        };
        assert_eq!(line.unit, WheelDeltaUnit::Line);
        assert_eq!(
            translated[1].to_ui_input_event_with_wheel_scale(12.0, UiSize::new(100.0, 80.0)),
            Some(UiInputEvent::Wheel(
                crate::UiWheelEvent::pixels(UiPoint::new(40.0, 50.0), UiPoint::new(12.0, -24.0))
                    .unit(WheelDeltaUnit::Line)
            ))
        );

        let RawInputEvent::Wheel(page) = translated[2] else {
            panic!("expected page wheel");
        };
        assert_eq!(page.unit, WheelDeltaUnit::Page);
        assert_eq!(
            translated[2].to_ui_input_event_with_wheel_scale(12.0, UiSize::new(100.0, 80.0)),
            Some(UiInputEvent::Wheel(
                crate::UiWheelEvent::pixels(UiPoint::new(40.0, 50.0), UiPoint::new(0.0, 80.0))
                    .unit(WheelDeltaUnit::Page)
            ))
        );
    }

    #[test]
    fn egui_key_events_map_repeat_release_and_focus_navigation() {
        let mut adapter = EguiInputAdapter::new();
        let command = egui::Modifiers {
            command: true,
            ..egui::Modifiers::NONE
        };

        let translated = adapter.translate_events(
            [
                key_event(egui::Key::A, true, true, command),
                key_event(egui::Key::A, false, false, command),
                key_event(egui::Key::F10, true, false, egui::Modifiers::SHIFT),
                key_event(egui::Key::Tab, true, false, egui::Modifiers::NONE),
                key_event(egui::Key::Tab, true, false, egui::Modifiers::SHIFT),
            ]
            .iter(),
            12,
        );

        let RawInputEvent::Keyboard(press) = translated[0] else {
            panic!("expected key press");
        };
        assert_eq!(press.key, KeyCode::Character('a'));
        assert!(press.pressed);
        assert!(press.repeat);
        assert!(press.modifiers.ctrl);
        assert_eq!(
            translated[0].to_ui_input_event(),
            Some(UiInputEvent::Key {
                key: KeyCode::Character('a'),
                modifiers: press.modifiers,
            })
        );

        let RawInputEvent::Keyboard(release) = translated[1] else {
            panic!("expected key release");
        };
        assert!(!release.pressed);
        assert_eq!(translated[1].to_ui_input_event(), None);

        let RawInputEvent::Keyboard(context_menu) = translated[2] else {
            panic!("expected F10 key press");
        };
        assert_eq!(context_menu.key, KeyCode::F10);
        assert!(context_menu.modifiers.shift);
        assert_eq!(
            translated[2].to_ui_input_event(),
            Some(UiInputEvent::Key {
                key: KeyCode::F10,
                modifiers: context_menu.modifiers,
            })
        );

        assert_eq!(translated[3], RawInputEvent::Focus(FocusDirection::Next));
        assert_eq!(
            translated[4],
            RawInputEvent::Focus(FocusDirection::Previous)
        );
    }

    #[test]
    fn egui_text_paste_and_ime_commit_translate_to_raw_text() {
        let mut adapter = EguiInputAdapter::new();
        let translated = adapter.translate_events(
            [
                egui::Event::Text("a".to_string()),
                egui::Event::Paste("paste".to_string()),
                egui::Event::Ime(egui::ImeEvent::Commit("ime".to_string())),
            ]
            .iter(),
            24,
        );

        assert_eq!(
            translated,
            vec![
                RawInputEvent::Text(RawTextInputEvent::new("a", 24)),
                RawInputEvent::Text(RawTextInputEvent::new("paste", 24)),
                RawInputEvent::Text(RawTextInputEvent::new("ime", 24)),
            ]
        );
    }

    #[test]
    fn egui_pointer_gone_emits_cancel_and_clears_capture_state() {
        let mut adapter = EguiInputAdapter::new();
        adapter.translate_event(
            &egui::Event::PointerButton {
                pos: egui::Pos2::new(8.0, 9.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            1,
        );

        let translated = adapter.translate_event(&egui::Event::PointerGone, 2);

        assert_eq!(translated.len(), 1);
        let RawInputEvent::Pointer(cancel) = translated[0] else {
            panic!("expected pointer cancel");
        };
        assert_eq!(cancel.kind, PointerEventKind::Cancel);
        assert_eq!(cancel.position, UiPoint::new(8.0, 9.0));
        assert!(cancel.buttons.contains(PointerButton::Primary));
        assert_eq!(adapter.buttons(), PointerButtons::NONE);
        assert_eq!(adapter.last_pointer_pos(), None);
    }

    #[test]
    fn translated_egui_events_feed_document_focus_routing() {
        let mut document =
            UiDocument::new(UiNodeStyle::from(LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                size: TaffySize {
                    width: Dimension::length(200.0),
                    height: Dimension::length(80.0),
                },
                ..Default::default()
            })));
        let first = document.add_child(
            document.root,
            UiNode::container("first", fixed_style(80.0, 40.0)).with_input(InputBehavior::BUTTON),
        );
        let second = document.add_child(
            document.root,
            UiNode::container("second", fixed_style(80.0, 40.0)).with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(200.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let mut adapter = EguiInputAdapter::new();
        let translated = adapter.translate_events(
            [
                egui::Event::PointerButton {
                    pos: egui::Pos2::new(10.0, 10.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                key_event(egui::Key::Tab, true, false, egui::Modifiers::NONE),
            ]
            .iter(),
            36,
        );

        let mut focused = None;
        for raw in translated {
            if let Some(event) = raw.to_ui_input_event() {
                focused = document.handle_input(event).focused;
            }
        }

        assert_eq!(document.node(first).name, "first");
        assert_eq!(focused, Some(second));
    }

    #[test]
    fn egui_platform_output_plan_maps_supported_platform_requests() {
        let requests = [
            PlatformRequest::Clipboard(ClipboardRequest::WriteText("copied".to_string())),
            PlatformRequest::Clipboard(ClipboardRequest::Clear),
            PlatformRequest::OpenUrl(OpenUrlRequest::new("https://example.test").new_window(true)),
            PlatformRequest::Cursor(CursorRequest::SetShape(CursorShape::Text)),
            PlatformRequest::Repaint(RepaintRequest::After(Duration::from_millis(16))),
        ];

        let plan = EguiPlatformOutputPlan::from_requests(requests.iter());

        assert!(plan.is_fully_supported());
        assert_eq!(plan.platform_output.commands.len(), 3);
        assert_eq!(
            plan.platform_output.commands[0],
            egui::OutputCommand::CopyText("copied".to_string())
        );
        assert_eq!(
            plan.platform_output.commands[1],
            egui::OutputCommand::CopyText(String::new())
        );
        assert_eq!(
            plan.platform_output.commands[2],
            egui::OutputCommand::OpenUrl(egui::OpenUrl {
                url: "https://example.test".to_string(),
                new_tab: true,
            })
        );
        assert_eq!(plan.platform_output.cursor_icon, egui::CursorIcon::Text);
        assert_eq!(
            plan.repaint_requests,
            vec![RepaintRequest::After(Duration::from_millis(16))]
        );
        assert!(plan.unsupported_requests.is_empty());
    }

    #[test]
    fn egui_platform_output_plan_reports_unsupported_host_requests() {
        let requests = [
            PlatformRequest::Clipboard(ClipboardRequest::ReadText),
            PlatformRequest::Cursor(CursorRequest::SetPosition(LogicalPoint::new(1.0, 2.0))),
            PlatformRequest::FileDialog(FileDialogRequest::new(FileDialogMode::OpenFile)),
        ];

        let plan = EguiPlatformOutputPlan::from_requests(requests.iter());

        assert!(!plan.is_fully_supported());
        assert!(plan.platform_output.commands.is_empty());
        assert_eq!(plan.platform_output.cursor_icon, egui::CursorIcon::Default);
        assert_eq!(plan.unsupported_requests, requests.to_vec());
    }

    #[test]
    fn egui_platform_output_plan_maps_service_requests_and_keeps_unsupported_ids() {
        let requests = [
            PlatformServiceRequest::new(
                PlatformRequestId::new(3),
                PlatformRequest::Clipboard(ClipboardRequest::WriteText("copied".to_string())),
            ),
            PlatformServiceRequest::new(
                PlatformRequestId::new(4),
                PlatformRequest::FileDialog(FileDialogRequest::new(FileDialogMode::OpenFile)),
            ),
            PlatformServiceRequest::new(
                PlatformRequestId::new(5),
                PlatformRequest::Cursor(CursorRequest::Confine(crate::platform::LogicalRect::new(
                    0.0, 0.0, 100.0, 80.0,
                ))),
            ),
        ];

        let plan = EguiPlatformOutputPlan::from_service_requests(requests.iter());

        assert!(!plan.is_fully_supported());
        assert_eq!(
            plan.platform_output.commands,
            vec![egui::OutputCommand::CopyText("copied".to_string())]
        );
        assert_eq!(
            plan.viewport_commands,
            vec![egui::ViewportCommand::CursorGrab(
                egui::CursorGrab::Confined
            )]
        );
        assert_eq!(plan.unsupported_requests, vec![requests[1].request.clone()]);
        assert_eq!(plan.unsupported_service_requests, vec![requests[1].clone()]);
        assert_eq!(
            plan.unsupported_service_responses(),
            vec![requests[1].unsupported_response()]
        );
    }

    #[test]
    fn egui_platform_output_plan_maps_document_frame_platform_requests() {
        let viewport = UiSize::new(320.0, 200.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(320.0, 200.0));
        let root = document.root;
        let mut canvas = UiNode::canvas(
            "native.viewport",
            "native.viewport",
            fixed_style(160.0, 96.0).layout,
        );
        canvas.content = UiContent::Canvas(
            CanvasContent::new("native.viewport")
                .native_viewport()
                .interaction(CanvasInteractionPolicy::NATIVE_VIEWPORT),
        );
        document.add_child(root, canvas);
        let host_output = HostFrameOutput::new(HostInteractionState::default())
            .repaint_next_frame(PlatformRequestId::new(7));
        let frame = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                host_output,
            ),
        )
        .expect("document frame");

        let mut allocator = PlatformRequestIdAllocator::new(20);
        let plan = EguiPlatformOutputPlan::from_document_frame_output(&frame, &mut allocator);

        assert!(plan.is_fully_supported());
        assert_eq!(plan.repaint_requests, vec![RepaintRequest::NextFrame]);
        assert_eq!(
            plan.viewport_commands,
            vec![
                egui::ViewportCommand::CursorGrab(egui::CursorGrab::Locked),
                egui::ViewportCommand::CursorVisible(false),
            ]
        );
        assert_eq!(plan.platform_output.cursor_icon, egui::CursorIcon::None);
        assert_eq!(allocator.next_value(), 22);
    }

    #[test]
    fn egui_platform_output_plan_keeps_document_frame_unsupported_service_ids() {
        let viewport = UiSize::new(240.0, 120.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(240.0, 120.0));
        let request = PlatformServiceRequest::new(
            PlatformRequestId::new(44),
            PlatformRequest::FileDialog(FileDialogRequest::new(FileDialogMode::OpenFile)),
        );
        let host_output = HostFrameOutput::new(HostInteractionState::default())
            .request(request.id, request.request.clone());
        let frame = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                host_output,
            ),
        )
        .expect("document frame");

        let mut allocator = PlatformRequestIdAllocator::new(90);
        let mut plan = EguiPlatformOutputPlan::new();
        plan.push_document_frame_output(&frame, &mut allocator);

        assert!(!plan.is_fully_supported());
        assert_eq!(plan.unsupported_service_requests, vec![request.clone()]);
        assert_eq!(
            plan.unsupported_service_responses(),
            vec![request.unsupported_response()]
        );
        assert_eq!(allocator.next_value(), 90);
    }

    #[test]
    fn egui_platform_output_plan_maps_cursor_visibility_and_shapes() {
        assert_eq!(
            egui_cursor_icon(CursorShape::Pointer),
            egui::CursorIcon::PointingHand
        );
        assert_eq!(
            egui_cursor_icon(CursorShape::ResizeNorthEastSouthWest),
            egui::CursorIcon::ResizeNeSw
        );
        assert_eq!(
            egui_cursor_icon(CursorShape::ResizeNorthWestSouthEast),
            egui::CursorIcon::ResizeNwSe
        );

        let mut plan = EguiPlatformOutputPlan::new();
        assert!(
            plan.push_request(&PlatformRequest::Cursor(CursorRequest::SetShape(
                CursorShape::Grab,
            )))
        );
        assert_eq!(plan.platform_output.cursor_icon, egui::CursorIcon::Grab);
        assert!(plan.push_request(&PlatformRequest::Cursor(CursorRequest::SetVisible(false,))));
        assert_eq!(plan.platform_output.cursor_icon, egui::CursorIcon::None);
        assert!(plan.push_request(&PlatformRequest::Cursor(CursorRequest::SetVisible(true,))));
        assert_eq!(plan.platform_output.cursor_icon, egui::CursorIcon::Default);
    }

    #[test]
    fn egui_platform_output_plan_maps_cursor_confinement_to_viewport_commands() {
        assert_eq!(
            egui_cursor_grab(CursorGrabMode::Locked),
            egui::CursorGrab::Locked
        );

        let requests = [
            PlatformRequest::Cursor(CursorRequest::SetGrab(CursorGrabMode::Locked)),
            PlatformRequest::Cursor(CursorRequest::Confine(crate::platform::LogicalRect::new(
                0.0, 0.0, 100.0, 80.0,
            ))),
            PlatformRequest::Cursor(CursorRequest::ReleaseConfine),
            PlatformRequest::Cursor(CursorRequest::SetGrab(CursorGrabMode::None)),
        ];

        let plan = EguiPlatformOutputPlan::from_requests(requests.iter());

        assert!(plan.is_fully_supported());
        assert_eq!(
            plan.viewport_commands,
            vec![
                egui::ViewportCommand::CursorGrab(egui::CursorGrab::Locked),
                egui::ViewportCommand::CursorGrab(egui::CursorGrab::Confined),
                egui::ViewportCommand::CursorGrab(egui::CursorGrab::None),
                egui::ViewportCommand::CursorGrab(egui::CursorGrab::None),
            ]
        );
        assert!(plan.unsupported_requests.is_empty());
    }

    #[test]
    fn egui_texture_delta_plan_maps_full_rgba_updates() {
        let handle = ResourceHandle::Texture(TextureHandle::app("menu.thumbnail"));
        let descriptor =
            ResourceDescriptor::new(handle.clone(), PixelSize::new(2, 1), ResourceFormat::Rgba8)
                .version(7);
        let update = ResourceUpdate::full(descriptor, vec![10, 20, 30, 40, 50, 60, 70, 80]);

        let plan =
            EguiTextureDeltaPlan::from_resource_updates([&update], egui::TextureOptions::NEAREST);

        assert!(plan.is_fully_supported());
        assert!(plan.rejected_updates.is_empty());
        assert_eq!(plan.textures_delta.set.len(), 1);
        assert_eq!(
            plan.textures_delta.set[0].0,
            egui_texture_id_for_resource(&handle)
        );
        let delta = &plan.textures_delta.set[0].1;
        assert_eq!(delta.pos, None);
        assert_eq!(delta.options, egui::TextureOptions::NEAREST);
        let egui::ImageData::Color(image) = &delta.image;
        assert_eq!(image.size, [2, 1]);
        assert_eq!(
            image.pixels,
            vec![
                egui::Color32::from_rgba_unmultiplied(10, 20, 30, 40),
                egui::Color32::from_rgba_unmultiplied(50, 60, 70, 80),
            ]
        );
    }

    #[test]
    fn egui_texture_delta_plan_maps_partial_bgra_and_alpha_updates() {
        let bgra_handle = ResourceHandle::Image(ImageHandle::app("cover.partial"));
        let bgra = ResourceUpdate::partial(
            ResourceDescriptor::new(
                bgra_handle.clone(),
                PixelSize::new(4, 4),
                ResourceFormat::Bgra8,
            ),
            PixelRect::new(1, 2, 2, 1),
            vec![30, 20, 10, 40, 70, 60, 50, 80],
        );
        let alpha_handle = ResourceHandle::Texture(TextureHandle::app("mask.alpha"));
        let alpha = ResourceUpdate::full(
            ResourceDescriptor::new(alpha_handle, PixelSize::new(1, 2), ResourceFormat::Alpha8),
            vec![64, 192],
        );

        let plan = EguiTextureDeltaPlan::from_resource_updates(
            [&bgra, &alpha],
            egui::TextureOptions::LINEAR,
        );

        assert!(plan.is_fully_supported());
        assert_eq!(plan.textures_delta.set.len(), 2);
        assert_eq!(
            plan.textures_delta.set[0].0,
            egui_texture_id_for_resource(&bgra_handle)
        );
        assert_eq!(plan.textures_delta.set[0].1.pos, Some([1, 2]));
        let egui::ImageData::Color(bgra_image) = &plan.textures_delta.set[0].1.image;
        assert_eq!(bgra_image.size, [2, 1]);
        assert_eq!(
            bgra_image.pixels,
            vec![
                egui::Color32::from_rgba_unmultiplied(10, 20, 30, 40),
                egui::Color32::from_rgba_unmultiplied(50, 60, 70, 80),
            ]
        );

        let egui::ImageData::Color(alpha_image) = &plan.textures_delta.set[1].1.image;
        assert_eq!(alpha_image.size, [1, 2]);
        assert_eq!(
            alpha_image.pixels,
            vec![
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 64),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 192),
            ]
        );
    }

    #[test]
    fn egui_texture_delta_plan_rejects_invalid_resource_updates() {
        let invalid_len = ResourceUpdate::full(
            ResourceDescriptor::new(
                ResourceHandle::Image(ImageHandle::app("bad.len")),
                PixelSize::new(2, 2),
                ResourceFormat::Rgba8,
            ),
            vec![0; 3],
        );
        let invalid_rect = ResourceUpdate::partial(
            ResourceDescriptor::new(
                ResourceHandle::Image(ImageHandle::app("bad.rect")),
                PixelSize::new(2, 2),
                ResourceFormat::Rgba8,
            ),
            PixelRect::new(2, 2, 1, 1),
            vec![0; 4],
        );

        let plan = EguiTextureDeltaPlan::from_resource_updates(
            [&invalid_len, &invalid_rect],
            egui::TextureOptions::LINEAR,
        );

        assert!(!plan.is_fully_supported());
        assert!(plan.textures_delta.set.is_empty());
        assert_eq!(plan.rejected_updates, vec![invalid_len, invalid_rect]);
    }

    #[test]
    fn egui_texture_ids_are_stable_across_resource_domains_and_kinds() {
        let app_texture = ResourceHandle::Texture(TextureHandle::app("shared"));
        let same_app_texture = ResourceHandle::Texture(TextureHandle::app("shared"));
        let image = ResourceHandle::Image(ImageHandle::app("shared"));
        let host_texture = ResourceHandle::Texture(TextureHandle::host("shared"));

        assert_eq!(
            egui_texture_id_for_resource(&app_texture),
            egui_texture_id_for_resource(&same_app_texture)
        );
        assert_ne!(
            egui_texture_id_for_resource(&app_texture),
            egui_texture_id_for_resource(&image)
        );
        assert_ne!(
            egui_texture_id_for_resource(&app_texture),
            egui_texture_id_for_resource(&host_texture)
        );
    }
}
