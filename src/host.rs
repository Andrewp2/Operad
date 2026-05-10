//! Host adapter contracts for pre-paint interaction state.
//!
//! Backends such as egui, winit/wgpu, CPU test harnesses, or app-owned hosts
//! can use these data contracts to feed hover, press, focus, drag capture,
//! wheel targeting, text/IME, and shortcut routing state into Operad before a
//! document is painted.

use std::fmt;

use crate::accessibility::{
    AccessibilityAdapterRequest, AccessibilityAnnouncementQueue, AccessibilityCapabilities,
    AccessibilityLiveRegionSnapshot,
};
use crate::commands::{CommandId, CommandRegistry, CommandScope, Shortcut};
use crate::input::{GestureEvent, GesturePhase, PointerCapture, RawInputEvent};
use crate::platform::{
    BackendCapabilities, PlatformRequest, PlatformRequestId, PlatformResponse,
    PlatformServiceRequest, PlatformServiceResponse, RepaintRequest, TextImeRequest,
    TextImeResponse, TextImeSession, TextInputId,
};
use crate::renderer::{RenderFrameRequest, RenderOptions, RenderTarget};
use crate::{
    AccessibilityTree, DirtyFlags, KeyCode, KeyModifiers, TextMeasurer, UiDocument, UiInputEvent,
    UiInputResult, UiNodeId, UiSize,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostShortcutRoute {
    pub shortcut: Shortcut,
    pub active_scopes: Vec<CommandScope>,
    pub target: Option<UiNodeId>,
    pub command: Option<CommandId>,
}

impl HostShortcutRoute {
    pub fn is_routed(&self) -> bool {
        self.command.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCommandDispatch {
    pub command: CommandId,
    pub shortcut: Shortcut,
    pub target: Option<UiNodeId>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HostNodeInteraction {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub drag_captured: bool,
    pub text_editing: bool,
    pub wheel_targeted: bool,
    pub shortcut_targeted: bool,
}

impl HostNodeInteraction {
    pub const fn any(self) -> bool {
        self.hovered
            || self.pressed
            || self.focused
            || self.drag_captured
            || self.text_editing
            || self.wheel_targeted
            || self.shortcut_targeted
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostInteractionState {
    pub hovered: Option<UiNodeId>,
    pub pressed: Option<UiNodeId>,
    pub focused: Option<UiNodeId>,
    pub drag_capture: Option<PointerCapture>,
    pub text_ime: Option<TextImeSession>,
    pub text_target: Option<UiNodeId>,
    pub wheel_target: Option<UiNodeId>,
    pub active_shortcut_scopes: Vec<CommandScope>,
    pub shortcut_route: Option<HostShortcutRoute>,
}

impl HostInteractionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_input_result(result: UiInputResult) -> Self {
        let mut state = Self::new();
        state.apply_input_result(result);
        state
    }

    pub fn apply_input_result(&mut self, result: UiInputResult) {
        self.hovered = result.hovered;
        self.focused = result.focused;
        self.pressed = result.pressed;
        self.wheel_target = result.scrolled;
    }

    pub fn apply_gesture(&mut self, event: &GestureEvent) {
        match event {
            GestureEvent::Hover { target, .. } => {
                self.hovered = *target;
            }
            GestureEvent::Press { target, .. } => {
                self.pressed = *target;
            }
            GestureEvent::Drag(gesture) => {
                self.hovered = Some(gesture.target);
                match gesture.phase {
                    GesturePhase::Preview | GesturePhase::Begin | GesturePhase::Update => {
                        self.pressed = Some(gesture.target);
                        self.drag_capture = Some(PointerCapture::new(
                            gesture.pointer_id,
                            gesture.target,
                            gesture.origin,
                            0.0,
                            gesture.modifiers,
                        ));
                    }
                    GesturePhase::Commit | GesturePhase::Cancel => {
                        self.pressed = None;
                        self.clear_drag_capture(gesture.pointer_id);
                    }
                }
            }
            GestureEvent::Click(click) => {
                self.hovered = Some(click.target);
                self.pressed = None;
            }
            GestureEvent::WheelTargeted { target, .. } => {
                self.wheel_target = *target;
            }
            GestureEvent::Cancel { pointer_id, .. } => {
                self.pressed = None;
                self.clear_drag_capture(*pointer_id);
            }
        }
    }

    pub fn clear_drag_capture(&mut self, pointer_id: crate::PointerId) -> bool {
        if self
            .drag_capture
            .is_some_and(|capture| capture.pointer_id == pointer_id)
        {
            self.drag_capture = None;
            true
        } else {
            false
        }
    }

    pub fn set_active_shortcut_scopes(&mut self, scopes: impl IntoIterator<Item = CommandScope>) {
        self.active_shortcut_scopes = scopes.into_iter().collect();
    }

    pub fn with_active_shortcut_scope(mut self, scope: CommandScope) -> Self {
        self.active_shortcut_scopes.push(scope);
        self
    }

    pub fn route_shortcut(
        &mut self,
        shortcut: Shortcut,
        registry: &CommandRegistry,
    ) -> HostShortcutRoute {
        let command = registry.resolve(shortcut, &self.active_shortcut_scopes);
        let route = HostShortcutRoute {
            shortcut,
            active_scopes: self.active_shortcut_scopes.clone(),
            target: self.focused,
            command,
        };
        self.shortcut_route = Some(route.clone());
        route
    }

    pub fn route_key(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
        registry: &CommandRegistry,
    ) -> HostShortcutRoute {
        self.route_shortcut(Shortcut::new(key, modifiers), registry)
    }

    pub fn activate_text_ime(&mut self, session: TextImeSession) -> PlatformRequest {
        self.text_target = text_target_from_input(&session.input);
        self.text_ime = Some(session.clone());
        PlatformRequest::TextIme(TextImeRequest::Activate(session))
    }

    pub fn activate_text_ime_for(
        &mut self,
        target: UiNodeId,
        session: TextImeSession,
    ) -> PlatformRequest {
        self.text_target = Some(target);
        self.text_ime = Some(session.clone());
        PlatformRequest::TextIme(TextImeRequest::Activate(session))
    }

    pub fn update_text_ime(&mut self, session: TextImeSession) -> PlatformRequest {
        self.text_target = self
            .text_target
            .or_else(|| text_target_from_input(&session.input));
        self.text_ime = Some(session.clone());
        PlatformRequest::TextIme(TextImeRequest::Update(session))
    }

    pub fn deactivate_text_ime(&mut self, input: TextInputId) -> PlatformRequest {
        self.text_ime = None;
        self.text_target = None;
        PlatformRequest::TextIme(TextImeRequest::Deactivate { input })
    }

    pub fn apply_text_ime_response(&mut self, response: &TextImeResponse) {
        if let TextImeResponse::Deactivated { input } = response {
            if self
                .text_ime
                .as_ref()
                .is_some_and(|session| session.input == *input)
            {
                self.text_ime = None;
                self.text_target = None;
            }
        }
    }

    pub fn node_state(&self, node: UiNodeId) -> HostNodeInteraction {
        HostNodeInteraction {
            hovered: self.hovered == Some(node),
            pressed: self.pressed == Some(node),
            focused: self.focused == Some(node),
            drag_captured: self
                .drag_capture
                .is_some_and(|capture| capture.target == node),
            text_editing: self.text_target == Some(node),
            wheel_targeted: self.wheel_target == Some(node),
            shortcut_targeted: self
                .shortcut_route
                .as_ref()
                .is_some_and(|route| route.target == Some(node) && route.is_routed()),
        }
    }
}

pub fn text_input_id_for_node(node: UiNodeId) -> TextInputId {
    TextInputId::new(format!("node:{}", node.0))
}

fn text_target_from_input(input: &TextInputId) -> Option<UiNodeId> {
    input
        .0
        .strip_prefix("node:")
        .and_then(|index| index.parse::<usize>().ok())
        .map(UiNodeId)
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostFrameRequest {
    pub viewport: UiSize,
    pub state: HostInteractionState,
    pub raw_input: Vec<RawInputEvent>,
    pub platform_responses: Vec<PlatformServiceResponse>,
}

impl HostFrameRequest {
    pub fn new(viewport: UiSize, state: HostInteractionState) -> Self {
        Self {
            viewport,
            state,
            raw_input: Vec::new(),
            platform_responses: Vec::new(),
        }
    }

    pub fn raw_event(mut self, event: RawInputEvent) -> Self {
        self.raw_input.push(event);
        self
    }

    pub fn platform_response(mut self, response: PlatformServiceResponse) -> Self {
        self.platform_responses.push(response);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostFrameOutput {
    pub state: HostInteractionState,
    pub ui_events: Vec<UiInputEvent>,
    pub gestures: Vec<GestureEvent>,
    pub commands: Vec<HostCommandDispatch>,
    pub platform_requests: Vec<PlatformServiceRequest>,
    pub platform_responses: Vec<PlatformServiceResponse>,
}

impl HostFrameOutput {
    pub fn new(state: HostInteractionState) -> Self {
        Self {
            state,
            ui_events: Vec::new(),
            gestures: Vec::new(),
            commands: Vec::new(),
            platform_requests: Vec::new(),
            platform_responses: Vec::new(),
        }
    }

    pub fn request(mut self, id: PlatformRequestId, request: PlatformRequest) -> Self {
        self.platform_requests
            .push(PlatformServiceRequest::new(id, request));
        self
    }

    pub fn repaint_next_frame(mut self, id: PlatformRequestId) -> Self {
        self.platform_requests.push(PlatformServiceRequest::new(
            id,
            PlatformRequest::Repaint(RepaintRequest::NextFrame),
        ));
        self
    }

    pub fn response(mut self, id: PlatformRequestId, response: PlatformResponse) -> Self {
        self.platform_responses
            .push(PlatformServiceResponse::new(id, response));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostAdapterError {
    UnsupportedInput(String),
    UnsupportedPlatformRequest(String),
    Backend(String),
}

impl fmt::Display for HostAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedInput(reason) => write!(formatter, "unsupported host input: {reason}"),
            Self::UnsupportedPlatformRequest(reason) => {
                write!(formatter, "unsupported platform request: {reason}")
            }
            Self::Backend(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for HostAdapterError {}

pub trait HostAdapter {
    fn capabilities(&self) -> BackendCapabilities;

    fn process_frame(
        &mut self,
        request: HostFrameRequest,
    ) -> Result<HostFrameOutput, HostAdapterError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostDocumentFrameRequest {
    pub viewport: UiSize,
    pub target: RenderTarget,
    pub host_output: HostFrameOutput,
    pub previous_live_regions: Option<AccessibilityLiveRegionSnapshot>,
    pub accessibility_capabilities: AccessibilityCapabilities,
    pub render_options: RenderOptions,
    pub dirty_flags: DirtyFlags,
}

impl HostDocumentFrameRequest {
    pub fn new(viewport: UiSize, target: RenderTarget, host_output: HostFrameOutput) -> Self {
        Self {
            viewport,
            target,
            host_output,
            previous_live_regions: None,
            accessibility_capabilities: AccessibilityCapabilities::NONE,
            render_options: RenderOptions::default(),
            dirty_flags: DirtyFlags::ALL,
        }
    }

    pub fn previous_live_regions(mut self, previous: AccessibilityLiveRegionSnapshot) -> Self {
        self.previous_live_regions = Some(previous);
        self
    }

    pub const fn accessibility_capabilities(
        mut self,
        capabilities: AccessibilityCapabilities,
    ) -> Self {
        self.accessibility_capabilities = capabilities;
        self
    }

    pub const fn render_options(mut self, options: RenderOptions) -> Self {
        self.render_options = options;
        self
    }

    pub const fn dirty_flags(mut self, dirty_flags: DirtyFlags) -> Self {
        self.dirty_flags = dirty_flags;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostDocumentFrameOutput {
    pub host_output: HostFrameOutput,
    pub input_results: Vec<UiInputResult>,
    pub render_request: RenderFrameRequest,
    pub accessibility_tree: AccessibilityTree,
    pub live_regions: AccessibilityLiveRegionSnapshot,
    pub announcements: AccessibilityAnnouncementQueue,
    pub accessibility_requests: Vec<AccessibilityAdapterRequest>,
}

pub fn process_document_frame(
    document: &mut UiDocument,
    measurer: &mut impl TextMeasurer,
    request: HostDocumentFrameRequest,
) -> Result<HostDocumentFrameOutput, taffy::TaffyError> {
    let HostDocumentFrameRequest {
        viewport,
        target,
        mut host_output,
        previous_live_regions,
        accessibility_capabilities,
        render_options,
        dirty_flags,
    } = request;

    let mut state = host_output.state.clone();
    let mut input_results = Vec::with_capacity(host_output.ui_events.len());
    for event in host_output.ui_events.iter().cloned() {
        let result = document.handle_input(event);
        state.apply_input_result(result.clone());
        input_results.push(result);
    }
    host_output.state = state.clone();

    document.compute_layout(viewport, measurer)?;

    let accessibility_tree = document.accessibility_snapshot();
    let live_regions = AccessibilityLiveRegionSnapshot::from_tree(&accessibility_tree);
    let previous_live_regions = previous_live_regions.unwrap_or_default();
    let announcements = AccessibilityAnnouncementQueue::from_live_region_diff(
        &previous_live_regions,
        &live_regions,
    );
    let accessibility_requests = announcements.supported_requests(accessibility_capabilities);

    let paint = document.paint_list();
    let mut node_interactions = paint
        .items
        .iter()
        .map(|item| (item.node, state.node_state(item.node)))
        .collect::<Vec<_>>();
    node_interactions.extend(
        accessibility_tree
            .nodes
            .iter()
            .map(|node| (node.id, state.node_state(node.id))),
    );
    let render_request = RenderFrameRequest::new(target, viewport, paint)
        .node_interactions(node_interactions)
        .dirty_flags(dirty_flags)
        .options(render_options);

    Ok(HostDocumentFrameOutput {
        host_output,
        input_results,
        render_request,
        accessibility_tree,
        live_regions,
        announcements,
        accessibility_requests,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, CommandMeta};
    use crate::input::{
        DragGesture, PointerButton, PointerId, RawKeyboardEvent, RawWheelEvent, WheelPhase,
    };
    use crate::platform::{
        BackendAdapterKind, LogicalRect, PlatformRequestId, PlatformServiceCapabilities,
        RepaintResponse, TextRange,
    };
    use crate::{
        length, AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole, ApproxTextMeasurer,
        InputBehavior, UiDocument, UiNode, UiNodeStyle, UiPoint,
    };
    use taffy::prelude::{Size as TaffySize, Style};

    fn fixed_style(width: f32, height: f32) -> UiNodeStyle {
        UiNodeStyle {
            layout: Style {
                size: TaffySize {
                    width: length(width),
                    height: length(height),
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn drag(target: UiNodeId, phase: GesturePhase) -> GestureEvent {
        GestureEvent::Drag(DragGesture {
            pointer_id: PointerId::MOUSE,
            target,
            phase,
            origin: UiPoint::new(4.0, 4.0),
            current: UiPoint::new(12.0, 8.0),
            previous: UiPoint::new(8.0, 6.0),
            delta: UiPoint::new(4.0, 2.0),
            total_delta: UiPoint::new(8.0, 4.0),
            button: PointerButton::Primary,
            modifiers: KeyModifiers::NONE,
            captured: true,
            timestamp_millis: 16,
        })
    }

    #[test]
    fn host_state_folds_input_results_and_gestures_before_paint() {
        let hovered = UiNodeId(1);
        let focused = UiNodeId(2);
        let scrolled = UiNodeId(3);
        let dragged = UiNodeId(4);
        let mut state = HostInteractionState::from_input_result(UiInputResult {
            hovered: Some(hovered),
            focused: Some(focused),
            pressed: Some(hovered),
            clicked: None,
            scrolled: Some(scrolled),
        });

        assert!(state.node_state(hovered).hovered);
        assert!(state.node_state(focused).focused);
        assert!(state.node_state(scrolled).wheel_targeted);

        state.apply_gesture(&drag(dragged, GesturePhase::Begin));
        let drag_state = state.node_state(dragged);
        assert!(drag_state.hovered);
        assert!(drag_state.pressed);
        assert!(drag_state.drag_captured);

        state.apply_gesture(&drag(dragged, GesturePhase::Commit));
        assert!(!state.node_state(dragged).drag_captured);
        assert!(state.drag_capture.is_none());
    }

    #[test]
    fn shortcut_routing_records_scopes_focused_target_and_command() {
        let mut registry = CommandRegistry::new();
        registry
            .register(Command::new(CommandMeta::new(
                "global.duplicate",
                "Duplicate",
            )))
            .unwrap();
        registry
            .register(Command::new(CommandMeta::new(
                "editor.duplicate",
                "Duplicate Note",
            )))
            .unwrap();
        registry
            .bind_shortcut(
                CommandScope::Global,
                Shortcut::ctrl('d'),
                "global.duplicate",
            )
            .unwrap();
        registry
            .bind_shortcut(
                CommandScope::Editor,
                Shortcut::ctrl('d'),
                "editor.duplicate",
            )
            .unwrap();

        let focused = UiNodeId(9);
        let mut state = HostInteractionState {
            focused: Some(focused),
            active_shortcut_scopes: vec![CommandScope::Workspace, CommandScope::Editor],
            ..HostInteractionState::default()
        };
        let route = state.route_shortcut(Shortcut::ctrl('D'), &registry);

        assert_eq!(route.command, Some(CommandId::new("editor.duplicate")));
        assert_eq!(route.target, Some(focused));
        assert_eq!(
            state.shortcut_route.as_ref().unwrap().active_scopes,
            vec![CommandScope::Workspace, CommandScope::Editor]
        );
        assert!(state.node_state(focused).shortcut_targeted);
    }

    #[test]
    fn text_ime_requests_update_host_state_and_platform_contracts() {
        let input = TextInputId::new("search");
        let session = TextImeSession::new(input.clone(), LogicalRect::new(10.0, 20.0, 1.0, 18.0))
            .surrounding_text("scale", TextRange::caret(5));
        let mut state = HostInteractionState::default();

        let request = state.activate_text_ime_for(UiNodeId(12), session.clone());
        assert!(matches!(
            request,
            PlatformRequest::TextIme(TextImeRequest::Activate(_))
        ));
        assert_eq!(state.text_ime, Some(session.clone()));
        assert!(state.node_state(UiNodeId(12)).text_editing);

        let updated = session.surrounding_text("scale mode", TextRange::new(6, 10));
        let request = state.update_text_ime(updated.clone());
        assert!(matches!(
            request,
            PlatformRequest::TextIme(TextImeRequest::Update(_))
        ));
        assert_eq!(state.text_ime, Some(updated));

        state.apply_text_ime_response(&TextImeResponse::Deactivated {
            input: input.clone(),
        });
        assert!(state.text_ime.is_none());

        let request = state.deactivate_text_ime(input);
        assert!(matches!(
            request,
            PlatformRequest::TextIme(TextImeRequest::Deactivate { .. })
        ));
    }

    #[test]
    fn node_text_input_ids_can_map_ime_sessions_back_to_nodes() {
        let input = text_input_id_for_node(UiNodeId(7));
        let session = TextImeSession::new(input.clone(), LogicalRect::new(0.0, 0.0, 1.0, 18.0));
        let mut state = HostInteractionState::default();

        state.activate_text_ime(session);
        assert_eq!(state.text_target, Some(UiNodeId(7)));
        assert!(state.node_state(UiNodeId(7)).text_editing);
        assert_eq!(input.0, "node:7");
    }

    #[test]
    fn document_frame_processes_input_render_and_accessibility_announcements() {
        let viewport = UiSize::new(240.0, 120.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(240.0, 120.0));
        let root = document.root;
        let button = document.add_child(
            root,
            UiNode::container("apply", fixed_style(80.0, 28.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Apply")
                        .focusable(),
                ),
        );
        let status = document.add_child(
            root,
            UiNode::container("status", fixed_style(140.0, 24.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Status)
                    .label("Status")
                    .value("Ready")
                    .live_region(AccessibilityLiveRegion::Polite),
            ),
        );
        document
            .compute_layout(viewport, &mut measurer)
            .expect("initial layout");
        let previous_live_regions =
            AccessibilityLiveRegionSnapshot::from_tree(&document.accessibility_snapshot());
        document
            .node_mut(status)
            .accessibility
            .as_mut()
            .expect("status accessibility")
            .value = Some("Running".to_string());

        let mut host_output = HostFrameOutput::new(HostInteractionState::default());
        host_output
            .ui_events
            .push(UiInputEvent::PointerDown(UiPoint::new(4.0, 4.0)));
        let frame = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                host_output,
            )
            .previous_live_regions(previous_live_regions)
            .accessibility_capabilities(AccessibilityCapabilities::SCREEN_READER),
        )
        .expect("document frame");

        assert_eq!(frame.input_results[0].focused, Some(button));
        assert_eq!(frame.host_output.state.focused, Some(button));
        assert_eq!(frame.render_request.viewport, viewport);
        assert!(frame.render_request.interaction_for(button).focused);
        assert_eq!(
            frame
                .accessibility_tree
                .node(status)
                .unwrap()
                .value
                .as_deref(),
            Some("Running")
        );
        assert_eq!(frame.announcements.pending.len(), 1);
        assert_eq!(frame.announcements.pending[0].message, "Status: Running");
        assert_eq!(frame.accessibility_requests.len(), 1);
        assert!(matches!(
            frame.accessibility_requests[0],
            AccessibilityAdapterRequest::Announce(_)
        ));
    }

    #[derive(Debug)]
    struct RecordingHost {
        capabilities: BackendCapabilities,
        registry: CommandRegistry,
    }

    impl HostAdapter for RecordingHost {
        fn capabilities(&self) -> BackendCapabilities {
            self.capabilities.clone()
        }

        fn process_frame(
            &mut self,
            request: HostFrameRequest,
        ) -> Result<HostFrameOutput, HostAdapterError> {
            let mut state = request.state;
            let mut output = HostFrameOutput::new(state.clone());
            output.platform_responses = request.platform_responses;

            for event in request.raw_input {
                if let Some(ui_event) =
                    event.to_ui_input_event_with_wheel_scale(16.0, request.viewport)
                {
                    output.ui_events.push(ui_event);
                }
                if let RawInputEvent::Keyboard(keyboard) = event {
                    let route = state.route_key(keyboard.key, keyboard.modifiers, &self.registry);
                    if let Some(command) = route.command.clone() {
                        output.commands.push(HostCommandDispatch {
                            command,
                            shortcut: route.shortcut,
                            target: route.target,
                        });
                    }
                }
            }

            output.state = state;
            Ok(output.repaint_next_frame(PlatformRequestId::new(77)))
        }
    }

    #[test]
    fn host_adapter_trait_processes_raw_input_commands_and_repaint_requests() {
        let mut registry = CommandRegistry::new();
        registry
            .register(Command::new(CommandMeta::new("save", "Save")))
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('s'), "save")
            .unwrap();

        let mut adapter = RecordingHost {
            capabilities: BackendCapabilities::new("recording-host")
                .adapter(BackendAdapterKind::Test)
                .services(PlatformServiceCapabilities {
                    repaint: true,
                    text_ime: true,
                    ..PlatformServiceCapabilities::NONE
                }),
            registry,
        };
        let response = PlatformServiceResponse::new(
            PlatformRequestId::new(1),
            PlatformResponse::Repaint(RepaintResponse::Coalesced),
        );
        let request = HostFrameRequest::new(
            UiSize::new(320.0, 180.0),
            HostInteractionState {
                focused: Some(UiNodeId(2)),
                active_shortcut_scopes: vec![CommandScope::Editor],
                ..HostInteractionState::default()
            },
        )
        .raw_event(RawInputEvent::Keyboard(RawKeyboardEvent::press(
            KeyCode::Character('S'),
            KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
            10,
        )))
        .raw_event(RawInputEvent::Wheel(
            RawWheelEvent::pixels(UiPoint::new(20.0, 10.0), UiPoint::new(0.0, -8.0), 11)
                .phase(WheelPhase::Moved),
        ))
        .platform_response(response.clone());

        let output = adapter.process_frame(request).expect("host frame output");
        assert_eq!(adapter.capabilities().adapter, BackendAdapterKind::Test);
        assert_eq!(output.commands[0].command, CommandId::new("save"));
        assert_eq!(output.commands[0].target, Some(UiNodeId(2)));
        assert_eq!(output.ui_events.len(), 2);
        assert_eq!(output.platform_responses, vec![response]);
        assert!(matches!(
            output.platform_requests[0].request,
            PlatformRequest::Repaint(RepaintRequest::NextFrame)
        ));
    }
}
