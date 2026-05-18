//! Host adapter contracts for pre-paint interaction state.
//!
//! Backends such as egui, winit/wgpu, test harnesses, or app-owned hosts
//! can use these data contracts to feed hover, press, focus, drag capture,
//! wheel targeting, text/IME, and shortcut routing state into Operad before a
//! document is painted.

use std::fmt;

use crate::accessibility::{
    push_supported_accessibility_request, AccessibilityAdapterRequest,
    AccessibilityAnnouncementQueue, AccessibilityCapabilities, AccessibilityLiveRegionSnapshot,
    AccessibilityPreferences, FocusRestoreTarget,
};
use crate::commands::{CommandId, CommandRegistry, CommandScope, Shortcut};
use crate::input::{
    GestureEvent, GesturePhase, PointerCapture, PointerEventKind, PointerGestureTracker,
    RawInputEvent,
};
use crate::layout_animation::{
    apply_layout_animation_transitions_to_paint_list, layout_animation_transitions,
    LayoutAnimationOptions, LayoutAnimationTransition,
};
use crate::platform::{
    BackendCapabilities, BackendCapabilityDiagnostic, BackendCapabilityRequirement,
    CapabilityFallback, PlatformRequest, PlatformRequestId, PlatformRequestIdAllocator,
    PlatformResponse, PlatformServiceRequest, PlatformServiceResponse, RepaintRequest,
    TextImeRequest, TextImeResponse, TextImeSession, TextInputId,
};
use crate::renderer::{
    CanvasHostCaptureState, CanvasHostCaptureTransition, RenderFrameRequest, RenderOptions,
    RenderTarget,
};
use crate::shell::{ShellLayoutPlan, ShellWorkspaceState};
use crate::{
    AccessibilityRole, AccessibilityTree, DirtyFlags, KeyCode, KeyModifiers, LayoutSnapshot,
    TextMeasurer, UiDocument, UiInputEvent, UiInputResult, UiNodeId, UiPoint, UiRect, UiSize,
    WidgetAction, WidgetActionBinding, WidgetActionQueue, WidgetValueEditPhase,
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
    pub input_consumed: bool,
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
            || self.input_consumed
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostInteractionState {
    pub hovered: Option<UiNodeId>,
    pub pressed: Option<UiNodeId>,
    pub focused: Option<UiNodeId>,
    pub drag_capture: Option<PointerCapture>,
    pub gesture_tracker: PointerGestureTracker,
    pub text_ime: Option<TextImeSession>,
    pub text_target: Option<UiNodeId>,
    pub wheel_target: Option<UiNodeId>,
    pub input_consumed: bool,
    pub input_consumed_by: Option<UiNodeId>,
    pub active_shortcut_scopes: Vec<CommandScope>,
    pub shortcut_route: Option<HostShortcutRoute>,
    pub canvas_host_capture: CanvasHostCaptureState,
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
        self.input_consumed = result.consumed;
        self.input_consumed_by = result.consumed_by;
    }

    pub fn apply_gesture(&mut self, event: &GestureEvent) {
        match event {
            GestureEvent::Hover { target, .. } => {
                self.hovered = *target;
            }
            GestureEvent::Press {
                target,
                pointer_id,
                position,
                modifiers,
                ..
            } => {
                self.hovered = *target;
                self.pressed = *target;
                self.drag_capture = (*target).map(|target| {
                    PointerCapture::new(*pointer_id, target, *position, 0.0, *modifiers)
                });
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
                self.clear_drag_capture(click.pointer_id);
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
            input_consumed: self.input_consumed_by == Some(node),
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

pub fn process_host_frame_input(request: HostFrameRequest) -> HostFrameOutput {
    process_host_frame_input_with_target_resolver(request, default_host_frame_target)
}

pub fn process_host_frame_input_with_target_resolver(
    request: HostFrameRequest,
    resolve_target: impl FnMut(&RawInputEvent, &HostInteractionState) -> Option<UiNodeId>,
) -> HostFrameOutput {
    process_host_frame_input_with_wheel_scale_and_target_resolver(request, 16.0, resolve_target)
}

pub fn process_host_frame_input_with_wheel_scale_and_target_resolver(
    request: HostFrameRequest,
    wheel_line_size: f32,
    mut resolve_target: impl FnMut(&RawInputEvent, &HostInteractionState) -> Option<UiNodeId>,
) -> HostFrameOutput {
    let HostFrameRequest {
        viewport,
        mut state,
        raw_input,
        platform_responses,
    } = request;
    let mut output = HostFrameOutput::new(state.clone());
    output.platform_responses = platform_responses;

    for event in raw_input {
        if let Some(ui_event) = event.to_ui_input_event_with_wheel_scale(wheel_line_size, viewport)
        {
            output.ui_events.push(ui_event);
        }

        let target = match event {
            RawInputEvent::Pointer(_) | RawInputEvent::Wheel(_) => resolve_target(&event, &state),
            RawInputEvent::Keyboard(_) | RawInputEvent::Text(_) | RawInputEvent::Focus(_) => None,
        };
        if let Some(gesture) = host_frame_gesture_for_event(&mut state, &event, target) {
            apply_host_frame_gesture(&mut state, &gesture);
            output.gestures.push(gesture);
        } else {
            clear_host_frame_capture_after_terminal_event(&mut state, &event);
        }
    }

    output.state = state;
    output
}

fn default_host_frame_target(
    event: &RawInputEvent,
    state: &HostInteractionState,
) -> Option<UiNodeId> {
    match event {
        RawInputEvent::Pointer(pointer) => state
            .drag_capture
            .filter(|capture| {
                capture.pointer_id == pointer.pointer_id
                    && matches!(
                        pointer.kind,
                        PointerEventKind::Move | PointerEventKind::Cancel
                    )
            })
            .map(|capture| capture.target)
            .or(state.hovered),
        RawInputEvent::Wheel(_) => state.wheel_target.or(state.hovered),
        RawInputEvent::Keyboard(_) | RawInputEvent::Text(_) | RawInputEvent::Focus(_) => None,
    }
}

fn host_frame_gesture_for_event(
    state: &mut HostInteractionState,
    event: &RawInputEvent,
    target: Option<UiNodeId>,
) -> Option<GestureEvent> {
    match event {
        RawInputEvent::Pointer(pointer) => match pointer.kind {
            PointerEventKind::Down(_) => Some(state.gesture_tracker.pointer_down(target, *pointer)),
            PointerEventKind::Move => state.gesture_tracker.pointer_move(target, *pointer),
            PointerEventKind::Up(_) => state.gesture_tracker.pointer_up(target, *pointer),
            PointerEventKind::Cancel => state
                .gesture_tracker
                .pointer_cancel(pointer.pointer_id, pointer.position),
        },
        RawInputEvent::Wheel(wheel) => Some(PointerGestureTracker::wheel(target, *wheel)),
        RawInputEvent::Keyboard(_) | RawInputEvent::Text(_) | RawInputEvent::Focus(_) => None,
    }
}

fn apply_host_frame_gesture(state: &mut HostInteractionState, gesture: &GestureEvent) {
    state.apply_gesture(gesture);
    match gesture {
        GestureEvent::Press { pointer_id, .. } => {
            sync_host_frame_capture_from_tracker(state, *pointer_id);
        }
        GestureEvent::Drag(drag)
            if matches!(
                drag.phase,
                GesturePhase::Preview | GesturePhase::Begin | GesturePhase::Update
            ) =>
        {
            sync_host_frame_capture_from_tracker(state, drag.pointer_id);
        }
        _ => {}
    }
}

fn sync_host_frame_capture_from_tracker(
    state: &mut HostInteractionState,
    pointer_id: crate::PointerId,
) {
    if let Some(capture) = state.gesture_tracker.active_capture(pointer_id) {
        state.drag_capture = Some(capture);
    }
}

fn clear_host_frame_capture_after_terminal_event(
    state: &mut HostInteractionState,
    event: &RawInputEvent,
) {
    let RawInputEvent::Pointer(pointer) = event else {
        return;
    };
    if matches!(
        pointer.kind,
        PointerEventKind::Up(_) | PointerEventKind::Cancel
    ) && state.clear_drag_capture(pointer.pointer_id)
    {
        state.pressed = None;
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostAccessibilityState {
    pub tree: Option<AccessibilityTree>,
    pub focused: Option<Option<UiNodeId>>,
    pub live_regions: Option<AccessibilityLiveRegionSnapshot>,
    pub preferences: Option<AccessibilityPreferences>,
}

impl HostAccessibilityState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(
        tree: AccessibilityTree,
        focused: Option<UiNodeId>,
        live_regions: AccessibilityLiveRegionSnapshot,
        preferences: AccessibilityPreferences,
    ) -> Self {
        Self {
            tree: Some(tree),
            focused: Some(focused),
            live_regions: Some(live_regions),
            preferences: Some(preferences),
        }
    }

    pub fn tree(mut self, tree: AccessibilityTree) -> Self {
        self.tree = Some(tree);
        self
    }

    pub const fn focused(mut self, focused: Option<UiNodeId>) -> Self {
        self.focused = Some(focused);
        self
    }

    pub fn live_regions(mut self, live_regions: AccessibilityLiveRegionSnapshot) -> Self {
        self.live_regions = Some(live_regions);
        self
    }

    pub const fn preferences(mut self, preferences: AccessibilityPreferences) -> Self {
        self.preferences = Some(preferences);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostDocumentFrameRequest {
    pub viewport: UiSize,
    pub target: RenderTarget,
    pub host_output: HostFrameOutput,
    pub previous_live_regions: Option<AccessibilityLiveRegionSnapshot>,
    pub previous_accessibility_tree: Option<AccessibilityTree>,
    pub previous_focused: Option<Option<UiNodeId>>,
    pub previous_accessibility_preferences: Option<AccessibilityPreferences>,
    pub previous_layout_snapshot: Option<LayoutSnapshot>,
    pub layout_animation_options: Option<LayoutAnimationOptions>,
    pub accessibility_capabilities: AccessibilityCapabilities,
    pub accessibility_preferences: AccessibilityPreferences,
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
            previous_accessibility_tree: None,
            previous_focused: None,
            previous_accessibility_preferences: None,
            previous_layout_snapshot: None,
            layout_animation_options: None,
            accessibility_capabilities: AccessibilityCapabilities::NONE,
            accessibility_preferences: AccessibilityPreferences::DEFAULT,
            render_options: RenderOptions::default(),
            dirty_flags: DirtyFlags::ALL,
        }
    }

    pub fn previous_live_regions(mut self, previous: AccessibilityLiveRegionSnapshot) -> Self {
        self.previous_live_regions = Some(previous);
        self
    }

    pub fn previous_accessibility_tree(mut self, previous: AccessibilityTree) -> Self {
        self.previous_accessibility_tree = Some(previous);
        self
    }

    pub const fn previous_focused(mut self, previous: Option<UiNodeId>) -> Self {
        self.previous_focused = Some(previous);
        self
    }

    pub const fn previous_accessibility_preferences(
        mut self,
        previous: AccessibilityPreferences,
    ) -> Self {
        self.previous_accessibility_preferences = Some(previous);
        self
    }

    pub fn previous_accessibility_state(mut self, previous: HostAccessibilityState) -> Self {
        self.previous_accessibility_tree = previous.tree;
        self.previous_focused = previous.focused;
        self.previous_live_regions = previous.live_regions;
        self.previous_accessibility_preferences = previous.preferences;
        self
    }

    pub fn previous_layout_snapshot(mut self, previous: LayoutSnapshot) -> Self {
        self.previous_layout_snapshot = Some(previous);
        self
    }

    pub fn with_previous_layout_snapshot(mut self, previous: Option<LayoutSnapshot>) -> Self {
        self.previous_layout_snapshot = previous;
        self
    }

    pub const fn layout_animation_options(mut self, options: LayoutAnimationOptions) -> Self {
        self.layout_animation_options = Some(options);
        self
    }

    pub const fn accessibility_capabilities(
        mut self,
        capabilities: AccessibilityCapabilities,
    ) -> Self {
        self.accessibility_capabilities = capabilities;
        self
    }

    pub const fn accessibility_preferences(
        mut self,
        preferences: AccessibilityPreferences,
    ) -> Self {
        self.accessibility_preferences = preferences;
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
    pub accessibility_state: HostAccessibilityState,
    pub canvas_host_capture_transition: CanvasHostCaptureTransition,
    pub layout_snapshot: LayoutSnapshot,
    pub layout_animation_transitions: Vec<LayoutAnimationTransition>,
}

impl HostDocumentFrameOutput {
    pub fn platform_requests(&self) -> Vec<PlatformRequest> {
        let mut requests = self
            .host_output
            .platform_requests
            .iter()
            .map(|request| request.request.clone())
            .collect::<Vec<_>>();
        requests.extend(self.canvas_host_capture_transition.platform_requests());
        requests
    }

    pub fn platform_service_requests(
        &self,
        allocator: &mut PlatformRequestIdAllocator,
    ) -> Vec<PlatformServiceRequest> {
        let mut requests = self.host_output.platform_requests.clone();
        requests.extend(
            self.canvas_host_capture_transition
                .platform_service_requests(allocator),
        );
        requests
    }

    pub fn platform_request_capability_diagnostics(
        &self,
        backend: &BackendCapabilities,
        fallback: CapabilityFallback,
    ) -> Vec<BackendCapabilityDiagnostic> {
        self.platform_requests()
            .into_iter()
            .map(|request| {
                backend.diagnose_requirement(
                    BackendCapabilityRequirement::PlatformRequest(request),
                    fallback,
                )
            })
            .collect()
    }

    pub fn canvas_host_capture_capability_diagnostics(
        &self,
        backend: &BackendCapabilities,
        fallback: CapabilityFallback,
    ) -> Vec<BackendCapabilityDiagnostic> {
        self.render_request
            .canvas_host_capture_plans()
            .into_iter()
            .flat_map(|plan| plan.capability_requirements())
            .map(|requirement| backend.diagnose_requirement(requirement, fallback))
            .collect()
    }

    pub fn host_capability_diagnostics(
        &self,
        backend: &BackendCapabilities,
        fallback: CapabilityFallback,
    ) -> Vec<BackendCapabilityDiagnostic> {
        let mut diagnostics = self.canvas_host_capture_capability_diagnostics(backend, fallback);
        diagnostics.extend(self.platform_request_capability_diagnostics(backend, fallback));
        diagnostics
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostDocumentFrameState {
    pub interaction: HostInteractionState,
    pub accessibility: HostAccessibilityState,
    pub layout: Option<LayoutSnapshot>,
}

impl HostDocumentFrameState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_parts(
        interaction: HostInteractionState,
        accessibility: HostAccessibilityState,
    ) -> Self {
        Self {
            interaction,
            accessibility,
            layout: None,
        }
    }

    pub fn with_interaction(mut self, interaction: HostInteractionState) -> Self {
        self.interaction = interaction;
        self
    }

    pub fn with_accessibility(mut self, accessibility: HostAccessibilityState) -> Self {
        self.accessibility = accessibility;
        self
    }

    pub fn host_frame_request(&self, viewport: UiSize) -> HostFrameRequest {
        HostFrameRequest::new(viewport, self.interaction.clone())
    }

    pub fn document_frame_request(
        &self,
        viewport: UiSize,
        target: RenderTarget,
        host_output: HostFrameOutput,
    ) -> HostDocumentFrameRequest {
        HostDocumentFrameRequest::new(viewport, target, host_output)
            .previous_accessibility_state(self.accessibility.clone())
            .with_previous_layout_snapshot(self.layout.clone())
    }

    pub fn apply_host_frame_output(&mut self, output: &HostFrameOutput) {
        self.interaction = output.state.clone();
    }

    pub fn apply_document_frame_output(&mut self, output: &HostDocumentFrameOutput) {
        self.interaction = output.host_output.state.clone();
        self.accessibility = output.accessibility_state.clone();
        self.layout = Some(output.layout_snapshot.clone());
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HostShellEvent {
    ResizePanel {
        panel_id: String,
        delta: f32,
    },
    SetPanelExtent {
        panel_id: String,
        extent: f32,
    },
    CollapsePanel {
        panel_id: String,
    },
    RestorePanel {
        panel_id: String,
    },
    FocusPanel {
        panel_id: String,
        restore: FocusRestoreTarget,
    },
    ScrollPanel {
        panel_id: String,
        offset: UiPoint,
    },
}

impl HostShellEvent {
    pub fn resize_panel(panel_id: impl Into<String>, delta: f32) -> Self {
        Self::ResizePanel {
            panel_id: panel_id.into(),
            delta,
        }
    }

    pub fn set_panel_extent(panel_id: impl Into<String>, extent: f32) -> Self {
        Self::SetPanelExtent {
            panel_id: panel_id.into(),
            extent,
        }
    }

    pub fn collapse_panel(panel_id: impl Into<String>) -> Self {
        Self::CollapsePanel {
            panel_id: panel_id.into(),
        }
    }

    pub fn restore_panel(panel_id: impl Into<String>) -> Self {
        Self::RestorePanel {
            panel_id: panel_id.into(),
        }
    }

    pub fn focus_panel(panel_id: impl Into<String>, restore: FocusRestoreTarget) -> Self {
        Self::FocusPanel {
            panel_id: panel_id.into(),
            restore,
        }
    }

    pub fn scroll_panel(panel_id: impl Into<String>, offset: UiPoint) -> Self {
        Self::ScrollPanel {
            panel_id: panel_id.into(),
            offset,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostShellFrameRequest {
    pub viewport: UiRect,
    pub workspace: ShellWorkspaceState,
    pub events: Vec<HostShellEvent>,
}

impl HostShellFrameRequest {
    pub fn new(viewport: UiRect, workspace: ShellWorkspaceState) -> Self {
        Self {
            viewport,
            workspace,
            events: Vec::new(),
        }
    }

    pub fn event(mut self, event: HostShellEvent) -> Self {
        self.events.push(event);
        self
    }

    pub fn events(mut self, events: impl IntoIterator<Item = HostShellEvent>) -> Self {
        self.events.extend(events);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostShellFrameOutput {
    pub workspace: ShellWorkspaceState,
    pub layout: ShellLayoutPlan,
    pub changed: bool,
}

pub fn process_shell_frame(request: HostShellFrameRequest) -> HostShellFrameOutput {
    let HostShellFrameRequest {
        viewport,
        mut workspace,
        events,
    } = request;

    let mut changed = false;
    for event in events {
        changed |= apply_shell_event(&mut workspace, event);
    }
    let layout = workspace.layout(viewport);

    HostShellFrameOutput {
        workspace,
        layout,
        changed,
    }
}

fn apply_shell_event(workspace: &mut ShellWorkspaceState, event: HostShellEvent) -> bool {
    match event {
        HostShellEvent::ResizePanel { panel_id, delta } => workspace
            .panel_mut(&panel_id)
            .is_some_and(|panel| panel.resize_by(delta)),
        HostShellEvent::SetPanelExtent { panel_id, extent } => workspace
            .panel_mut(&panel_id)
            .is_some_and(|panel| panel.set_extent(extent)),
        HostShellEvent::CollapsePanel { panel_id } => workspace
            .panel_mut(&panel_id)
            .is_some_and(|panel| panel.collapse()),
        HostShellEvent::RestorePanel { panel_id } => workspace
            .panel_mut(&panel_id)
            .is_some_and(|panel| panel.restore()),
        HostShellEvent::FocusPanel { panel_id, restore } => {
            let Some(panel) = workspace.panel(&panel_id) else {
                return false;
            };
            if !panel.visible {
                return false;
            }
            let changed = workspace.focused_panel.as_deref() != Some(panel_id.as_str())
                || workspace.restored_focus != Some(restore);
            workspace.set_focused_panel(panel_id, restore);
            changed
        }
        HostShellEvent::ScrollPanel { panel_id, offset } => workspace
            .panel_mut(&panel_id)
            .is_some_and(|panel| panel.set_scroll_offset(offset)),
    }
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
        previous_accessibility_tree,
        previous_focused,
        previous_accessibility_preferences,
        previous_layout_snapshot,
        layout_animation_options,
        accessibility_capabilities,
        accessibility_preferences,
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
    let layout_snapshot = document.layout_snapshot();
    let layout_animation_transitions = if accessibility_preferences.should_reduce_motion() {
        Vec::new()
    } else if let (Some(previous), Some(options)) =
        (previous_layout_snapshot.as_ref(), layout_animation_options)
    {
        layout_animation_transitions(previous, &layout_snapshot, options)
    } else {
        Vec::new()
    };

    let accessibility_tree = document.accessibility_snapshot();
    let live_regions = AccessibilityLiveRegionSnapshot::from_tree(&accessibility_tree);
    let previous_live_regions = previous_live_regions.unwrap_or_default();
    let announcements = AccessibilityAnnouncementQueue::from_live_region_diff(
        &previous_live_regions,
        &live_regions,
    );
    let mut accessibility_requests = Vec::new();
    if previous_accessibility_tree
        .as_ref()
        .is_none_or(|previous| previous != &accessibility_tree)
        || previous_focused.is_some_and(|previous| previous != state.focused)
    {
        push_supported_accessibility_request(
            &mut accessibility_requests,
            accessibility_capabilities,
            AccessibilityAdapterRequest::PublishTree {
                tree: accessibility_tree.clone(),
                focused: state.focused,
                preferences: accessibility_preferences,
            },
        );
    }
    if previous_accessibility_preferences != Some(accessibility_preferences) {
        push_supported_accessibility_request(
            &mut accessibility_requests,
            accessibility_capabilities,
            AccessibilityAdapterRequest::ApplyPreferences(accessibility_preferences),
        );
    }
    for announcement in &announcements.pending {
        push_supported_accessibility_request(
            &mut accessibility_requests,
            accessibility_capabilities,
            AccessibilityAdapterRequest::Announce(announcement.clone()),
        );
    }
    let accessibility_state = HostAccessibilityState::current(
        accessibility_tree.clone(),
        state.focused,
        live_regions.clone(),
        accessibility_preferences,
    );

    let mut paint = document.paint_list();
    apply_layout_animation_transitions_to_paint_list(&mut paint, &layout_animation_transitions);
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
    let mut render_options = render_options;
    render_options.accessibility_preferences = accessibility_preferences;
    render_options.scale_factor =
        normalized_host_scale(render_options.scale_factor) * document.dpi_scale();

    let render_request = RenderFrameRequest::new(target, viewport, paint)
        .node_interactions(node_interactions)
        .dirty_flags(dirty_flags)
        .options(render_options);
    let canvas_host_capture_transition = state
        .canvas_host_capture
        .sync(render_request.canvas_host_capture_plans());
    host_output.state = state.clone();

    Ok(HostDocumentFrameOutput {
        host_output,
        input_results,
        render_request,
        accessibility_tree,
        live_regions,
        announcements,
        accessibility_requests,
        accessibility_state,
        canvas_host_capture_transition,
        layout_snapshot,
        layout_animation_transitions,
    })
}

pub fn collect_document_widget_actions(
    document: &UiDocument,
    frame: &HostDocumentFrameOutput,
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
                    .unwrap_or_else(|| UiRect::new(0.0, 0.0, 0.0, 0.0));
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
                    .unwrap_or_else(|| UiRect::new(0.0, 0.0, 0.0, 0.0));
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
    frame: &HostDocumentFrameOutput,
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
            return Some((target, WidgetValueEditPhase::Commit, *point, true));
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
            Some((WidgetValueEditPhase::Commit, *point, true))
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

fn normalized_host_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{
        AccessibilityAdapter, AccessibilityAdapterRequest, AccessibilityAdapterResponse,
        AccessibilityAnnouncement, AccessibilityCapabilities, AccessibilityPreferences,
        AccessibilityRequestKind, FocusRestoreTarget, FocusTrap,
    };
    use crate::commands::{Command, CommandMeta};
    use crate::diagnostics::{DiagnosticCategory, DiagnosticReport};
    use crate::input::{
        DragGesture, PointerButton, PointerEventKind, PointerId, RawKeyboardEvent, RawPointerEvent,
        RawTextInputEvent, RawWheelEvent, WheelPhase,
    };
    use crate::platform::{
        BackendAdapterKind, CapabilityDecision, CursorGrabMode, CursorRequest, InputCapabilities,
        InputCapabilityKind, LogicalRect, PlatformRequestId, PlatformRequestIdAllocator,
        PlatformServiceCapabilities, RepaintResponse, TextRange,
    };
    use crate::shell::{ShellPanelState, ShellRegion};
    use crate::{
        length, AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole, ApproxTextMeasurer,
        CanvasContent, CanvasInteractionPolicy, CanvasRenderMode, ColorRgba, InputBehavior,
        LayoutStyle, StrokeStyle, UiContent, UiDocument, UiNode, UiNodeStyle, UiPoint, UiVisual,
    };
    use taffy::prelude::{Size as TaffySize, Style};

    fn fixed_style(width: f32, height: f32) -> UiNodeStyle {
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(width),
                    height: length(height),
                },
                ..Default::default()
            })
            .style,
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

    fn raw_pointer(kind: PointerEventKind, x: f32, y: f32, timestamp: u64) -> RawInputEvent {
        RawInputEvent::Pointer(RawPointerEvent::new(kind, UiPoint::new(x, y), timestamp))
    }

    #[derive(Debug, Default)]
    struct TestHostAccessibilityAdapter {
        capabilities: AccessibilityCapabilities,
        handled: Vec<AccessibilityAdapterRequest>,
        published_focus: Option<UiNodeId>,
        preferences: Option<AccessibilityPreferences>,
        trap: Option<FocusTrap>,
        announced: Vec<AccessibilityAnnouncement>,
        focused: Option<UiNodeId>,
    }

    impl TestHostAccessibilityAdapter {
        fn new(capabilities: AccessibilityCapabilities) -> Self {
            Self {
                capabilities,
                ..Self::default()
            }
        }
    }

    impl AccessibilityAdapter for TestHostAccessibilityAdapter {
        fn accessibility_capabilities(&self) -> AccessibilityCapabilities {
            self.capabilities
        }

        fn handle_accessibility_request(
            &mut self,
            request: AccessibilityAdapterRequest,
        ) -> AccessibilityAdapterResponse {
            if !self.capabilities.supports(request.kind()) {
                return AccessibilityAdapterResponse::Unsupported(request.kind());
            }

            self.handled.push(request.clone());
            match request {
                AccessibilityAdapterRequest::PublishTree {
                    focused,
                    preferences,
                    ..
                } => {
                    self.published_focus = focused;
                    self.preferences = Some(preferences);
                    AccessibilityAdapterResponse::Applied
                }
                AccessibilityAdapterRequest::MoveFocus { target, .. } => {
                    self.focused = Some(target);
                    AccessibilityAdapterResponse::FocusChanged(Some(target))
                }
                AccessibilityAdapterRequest::SetFocusTrap(trap) => {
                    self.trap = Some(trap);
                    AccessibilityAdapterResponse::Applied
                }
                AccessibilityAdapterRequest::ClearFocusTrap { restore } => {
                    self.trap = None;
                    AccessibilityAdapterResponse::FocusChanged(match restore {
                        FocusRestoreTarget::Node(node) => Some(node),
                        FocusRestoreTarget::Previous => self.focused,
                        FocusRestoreTarget::None => None,
                    })
                }
                AccessibilityAdapterRequest::RestoreFocus(restore) => {
                    AccessibilityAdapterResponse::FocusChanged(match restore {
                        FocusRestoreTarget::Node(node) => Some(node),
                        FocusRestoreTarget::Previous => self.focused,
                        FocusRestoreTarget::None => None,
                    })
                }
                AccessibilityAdapterRequest::Announce(announcement) => {
                    self.announced.push(announcement);
                    AccessibilityAdapterResponse::Applied
                }
                AccessibilityAdapterRequest::ApplyPreferences(preferences) => {
                    self.preferences = Some(preferences);
                    AccessibilityAdapterResponse::PreferencesChanged(preferences)
                }
            }
        }
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
            consumed: true,
            consumed_by: Some(scrolled),
        });

        assert!(state.node_state(hovered).hovered);
        assert!(state.node_state(focused).focused);
        assert!(state.node_state(scrolled).wheel_targeted);
        assert!(state.node_state(scrolled).input_consumed);
        assert!(state.input_consumed);
        assert_eq!(state.input_consumed_by, Some(scrolled));

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
    fn host_frame_gesture_tracker_respects_drag_threshold_across_frames() {
        let viewport = UiSize::new(200.0, 120.0);
        let target = UiNodeId(7);
        let outside = UiNodeId(8);
        let first = process_host_frame_input_with_target_resolver(
            HostFrameRequest::new(viewport, HostInteractionState::default()).raw_event(
                raw_pointer(
                    PointerEventKind::Down(PointerButton::Primary),
                    10.0,
                    10.0,
                    1,
                ),
            ),
            |_, _| Some(target),
        );

        assert_eq!(
            first.ui_events,
            vec![UiInputEvent::PointerDown(UiPoint::new(10.0, 10.0))]
        );
        assert!(matches!(
            &first.gestures[..],
            [GestureEvent::Press {
                target: Some(actual),
                ..
            }] if *actual == target
        ));
        assert_eq!(first.state.drag_capture.unwrap().target, target);

        let under_threshold = process_host_frame_input_with_target_resolver(
            HostFrameRequest::new(viewport, first.state).raw_event(raw_pointer(
                PointerEventKind::Move,
                12.0,
                13.0,
                2,
            )),
            |_, _| Some(outside),
        );
        assert!(under_threshold.gestures.is_empty());
        assert_eq!(
            under_threshold.ui_events,
            vec![UiInputEvent::PointerMove(UiPoint::new(12.0, 13.0))]
        );
        assert_eq!(under_threshold.state.drag_capture.unwrap().target, target);

        let drag_begin = process_host_frame_input_with_target_resolver(
            HostFrameRequest::new(viewport, under_threshold.state).raw_event(raw_pointer(
                PointerEventKind::Move,
                20.0,
                14.0,
                3,
            )),
            |_, _| Some(outside),
        );
        let [GestureEvent::Drag(begin)] = &drag_begin.gestures[..] else {
            panic!("expected one drag begin gesture");
        };
        assert_eq!(begin.target, target);
        assert_eq!(begin.phase, GesturePhase::Begin);
        assert_eq!(begin.total_delta, UiPoint::new(10.0, 4.0));
    }

    #[test]
    fn host_frame_preserves_capture_across_outside_move_and_up() {
        let viewport = UiSize::new(200.0, 120.0);
        let target = UiNodeId(3);
        let outside = UiNodeId(4);
        let pressed = process_host_frame_input_with_target_resolver(
            HostFrameRequest::new(viewport, HostInteractionState::default()).raw_event(
                raw_pointer(PointerEventKind::Down(PointerButton::Primary), 4.0, 4.0, 1),
            ),
            |_, _| Some(target),
        );
        let dragging = process_host_frame_input_with_target_resolver(
            HostFrameRequest::new(viewport, pressed.state).raw_event(raw_pointer(
                PointerEventKind::Move,
                24.0,
                6.0,
                2,
            )),
            |_, _| Some(outside),
        );
        let [GestureEvent::Drag(begin)] = &dragging.gestures[..] else {
            panic!("expected drag begin");
        };
        assert_eq!(begin.target, target);
        assert_eq!(begin.phase, GesturePhase::Begin);
        assert_eq!(dragging.state.drag_capture.unwrap().target, target);

        let committed = process_host_frame_input_with_target_resolver(
            HostFrameRequest::new(viewport, dragging.state).raw_event(raw_pointer(
                PointerEventKind::Up(PointerButton::Primary),
                40.0,
                10.0,
                3,
            )),
            |_, _| Some(outside),
        );
        let [GestureEvent::Drag(commit)] = &committed.gestures[..] else {
            panic!("expected drag commit");
        };
        assert_eq!(commit.target, target);
        assert_eq!(commit.phase, GesturePhase::Commit);
        assert!(committed.state.drag_capture.is_none());
        assert!(committed.state.pressed.is_none());
        assert_eq!(
            committed
                .state
                .gesture_tracker
                .active_capture(PointerId::MOUSE),
            None
        );
    }

    #[test]
    fn host_frame_cancel_clears_capture() {
        let viewport = UiSize::new(200.0, 120.0);
        let target = UiNodeId(5);
        let pressed = process_host_frame_input_with_target_resolver(
            HostFrameRequest::new(viewport, HostInteractionState::default()).raw_event(
                raw_pointer(
                    PointerEventKind::Down(PointerButton::Primary),
                    10.0,
                    10.0,
                    1,
                ),
            ),
            |_, _| Some(target),
        );
        let dragging = process_host_frame_input_with_target_resolver(
            HostFrameRequest::new(viewport, pressed.state).raw_event(raw_pointer(
                PointerEventKind::Move,
                20.0,
                10.0,
                2,
            )),
            |_, _| Some(target),
        );

        let cancelled = process_host_frame_input_with_target_resolver(
            HostFrameRequest::new(viewport, dragging.state).raw_event(raw_pointer(
                PointerEventKind::Cancel,
                20.0,
                10.0,
                3,
            )),
            |_, _| Some(target),
        );
        let [GestureEvent::Drag(cancel)] = &cancelled.gestures[..] else {
            panic!("expected drag cancel");
        };
        assert_eq!(cancel.target, target);
        assert_eq!(cancel.phase, GesturePhase::Cancel);
        assert!(cancelled.state.drag_capture.is_none());
        assert!(cancelled.state.pressed.is_none());
        assert_eq!(
            cancelled
                .state
                .gesture_tracker
                .active_capture(PointerId::MOUSE),
            None
        );
    }

    #[test]
    fn host_frame_wheel_emits_targeted_gesture_and_legacy_event() {
        let viewport = UiSize::new(200.0, 120.0);
        let target = UiNodeId(9);
        let wheel = RawWheelEvent::pixels(UiPoint::new(18.0, 12.0), UiPoint::new(0.0, -4.0), 10)
            .phase(WheelPhase::Moved);
        let state = HostInteractionState {
            hovered: Some(target),
            ..HostInteractionState::default()
        };
        let output = process_host_frame_input(
            HostFrameRequest::new(viewport, state).raw_event(RawInputEvent::Wheel(wheel)),
        );

        assert_eq!(output.ui_events.len(), 1);
        assert!(matches!(
            &output.gestures[..],
            [GestureEvent::WheelTargeted {
                target: Some(actual),
                event
            }] if *actual == target && *event == wheel
        ));
        assert_eq!(output.state.wheel_target, Some(target));
    }

    #[test]
    fn host_frame_preserves_legacy_ui_event_conversion() {
        let viewport = UiSize::new(320.0, 180.0);
        let target = UiNodeId(1);
        let raw_events = vec![
            raw_pointer(PointerEventKind::Down(PointerButton::Primary), 6.0, 8.0, 1),
            RawInputEvent::Wheel(
                RawWheelEvent::lines(UiPoint::new(12.0, 10.0), UiPoint::new(0.0, -2.0), 2)
                    .phase(WheelPhase::Started),
            ),
            RawInputEvent::Keyboard(RawKeyboardEvent::press(
                KeyCode::Character('A'),
                KeyModifiers::NONE,
                3,
            )),
            RawInputEvent::Text(RawTextInputEvent::new("a", 4)),
            RawInputEvent::Focus(crate::FocusDirection::Next),
        ];
        let expected_ui_events: Vec<_> = raw_events
            .iter()
            .filter_map(|event| event.to_ui_input_event_with_wheel_scale(20.0, viewport))
            .collect();
        let mut request = HostFrameRequest::new(viewport, HostInteractionState::default());
        for event in raw_events {
            request = request.raw_event(event);
        }

        let output = process_host_frame_input_with_wheel_scale_and_target_resolver(
            request,
            20.0,
            |event, _| match event {
                RawInputEvent::Pointer(_) | RawInputEvent::Wheel(_) => Some(target),
                _ => None,
            },
        );

        assert_eq!(output.ui_events, expected_ui_events);
        assert!(matches!(
            output.gestures.first(),
            Some(GestureEvent::Press {
                target: Some(actual),
                ..
            }) if *actual == target
        ));
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
        assert_eq!(input.as_str(), "node:7");
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
            .accessibility_capabilities(AccessibilityCapabilities::SCREEN_READER)
            .accessibility_preferences(
                AccessibilityPreferences::DEFAULT
                    .reduced_motion(true)
                    .text_scale(1.25),
            ),
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
        assert_eq!(
            frame
                .accessibility_requests
                .iter()
                .map(AccessibilityAdapterRequest::kind)
                .collect::<Vec<_>>(),
            vec![
                AccessibilityRequestKind::PublishTree,
                AccessibilityRequestKind::ApplyPreferences,
                AccessibilityRequestKind::Announce,
            ]
        );
        assert_eq!(
            frame.render_request.options.accessibility_preferences,
            AccessibilityPreferences::DEFAULT
                .reduced_motion(true)
                .text_scale(1.25)
        );
        assert!(matches!(
            frame.accessibility_requests[2],
            AccessibilityAdapterRequest::Announce(_)
        ));
    }

    #[test]
    fn document_frame_combines_document_dpi_with_render_scale() {
        let viewport = UiSize::new(120.0, 80.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(120.0, 80.0));
        document.set_dpi_scale(2.0);
        document
            .compute_layout(viewport, &mut measurer)
            .expect("layout");

        let frame = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            )
            .render_options(RenderOptions {
                scale_factor: 1.5,
                ..Default::default()
            }),
        )
        .expect("document frame");

        assert_eq!(frame.render_request.options.scale_factor, 3.0);
    }

    #[test]
    fn host_accessibility_requests_round_trip_all_supported_kinds_with_focus_trap() {
        let viewport = UiSize::new(240.0, 120.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(240.0, 120.0));
        let root = document.root;
        let play = document.add_child(
            root,
            UiNode::container("play", fixed_style(80.0, 28.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Play")
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

        let frame = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            )
            .previous_live_regions(previous_live_regions)
            .accessibility_capabilities(AccessibilityCapabilities::SCREEN_READER)
            .accessibility_preferences(AccessibilityPreferences::DEFAULT.high_contrast(true)),
        )
        .expect("document frame");

        let focus_trap = FocusTrap::new(root).restore_focus(FocusRestoreTarget::Node(play));
        let mut requests = frame.accessibility_requests;
        requests.extend([
            AccessibilityAdapterRequest::SetFocusTrap(focus_trap),
            AccessibilityAdapterRequest::MoveFocus {
                target: play,
                restore: FocusRestoreTarget::Previous,
            },
            AccessibilityAdapterRequest::RestoreFocus(FocusRestoreTarget::Node(status)),
            AccessibilityAdapterRequest::ClearFocusTrap {
                restore: FocusRestoreTarget::Node(play),
            },
        ]);

        let mut adapter =
            TestHostAccessibilityAdapter::new(AccessibilityCapabilities::SCREEN_READER);
        let mut responses = Vec::with_capacity(requests.len());
        for request in requests {
            responses.push(adapter.handle_accessibility_request(request));
        }

        assert_eq!(
            responses,
            vec![
                AccessibilityAdapterResponse::Applied,
                AccessibilityAdapterResponse::PreferencesChanged(
                    AccessibilityPreferences::DEFAULT.high_contrast(true),
                ),
                AccessibilityAdapterResponse::Applied,
                AccessibilityAdapterResponse::Applied,
                AccessibilityAdapterResponse::FocusChanged(Some(play)),
                AccessibilityAdapterResponse::FocusChanged(Some(status)),
                AccessibilityAdapterResponse::FocusChanged(Some(play)),
            ]
        );
        assert_eq!(adapter.announced.len(), 1);
        assert_eq!(adapter.announced[0].message, "Status: Running");
        assert_eq!(adapter.published_focus, None);
        assert_eq!(
            adapter.preferences,
            Some(AccessibilityPreferences::DEFAULT.high_contrast(true))
        );
        assert_eq!(adapter.trap, None);
        assert_eq!(adapter.focused, Some(play));
    }

    #[test]
    fn document_frame_publishes_screen_reader_tree_when_supported() {
        let viewport = UiSize::new(180.0, 80.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(180.0, 80.0));
        let button = document.add_child(
            document.root,
            UiNode::container("play", fixed_style(80.0, 28.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Play")
                        .focusable(),
                ),
        );
        let host_output = HostFrameOutput::new(HostInteractionState {
            focused: Some(button),
            ..HostInteractionState::default()
        });

        let frame = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                host_output,
            )
            .accessibility_capabilities(AccessibilityCapabilities::SCREEN_READER),
        )
        .expect("document frame");

        assert_eq!(
            frame.accessibility_requests[0].kind(),
            AccessibilityRequestKind::PublishTree
        );
        let AccessibilityAdapterRequest::PublishTree {
            tree,
            focused,
            preferences,
        } = &frame.accessibility_requests[0]
        else {
            panic!("expected PublishTree");
        };
        assert_eq!(*focused, Some(button));
        assert_eq!(*preferences, AccessibilityPreferences::DEFAULT);
        assert_eq!(tree.node(button).unwrap().label.as_deref(), Some("Play"));
        assert_eq!(frame.accessibility_state.focused, Some(Some(button)));
        assert_eq!(
            frame.accessibility_state.preferences,
            Some(AccessibilityPreferences::DEFAULT)
        );
        assert_eq!(
            frame.accessibility_state.live_regions.as_ref(),
            Some(&frame.live_regions)
        );
    }

    #[test]
    fn host_accessibility_state_groups_previous_frame_inputs() {
        let focused = UiNodeId(3);
        let preferences = AccessibilityPreferences::DEFAULT
            .screen_reader_active(true)
            .text_scale(1.4);
        let tree = AccessibilityTree {
            nodes: Vec::new(),
            focus_order: vec![focused],
            modal_scope: None,
        };
        let live_regions = AccessibilityLiveRegionSnapshot::default();
        let state = HostAccessibilityState::new()
            .tree(tree.clone())
            .focused(Some(focused))
            .live_regions(live_regions.clone())
            .preferences(preferences);

        let request = HostDocumentFrameRequest::new(
            UiSize::new(100.0, 50.0),
            RenderTarget::window("main", UiSize::new(100.0, 50.0)),
            HostFrameOutput::new(HostInteractionState::default()),
        )
        .previous_accessibility_state(state);

        assert_eq!(request.previous_accessibility_tree, Some(tree));
        assert_eq!(request.previous_focused, Some(Some(focused)));
        assert_eq!(request.previous_live_regions, Some(live_regions));
        assert_eq!(
            request.previous_accessibility_preferences,
            Some(preferences)
        );
    }

    #[test]
    fn host_document_frame_state_builds_requests_and_carries_outputs() {
        let viewport = UiSize::new(180.0, 80.0);
        let focused = UiNodeId(4);
        let interaction = HostInteractionState {
            focused: Some(focused),
            ..HostInteractionState::default()
        };
        let accessibility = HostAccessibilityState::new().focused(Some(focused));
        let mut state =
            HostDocumentFrameState::from_parts(interaction.clone(), accessibility.clone());

        let host_request = state.host_frame_request(viewport);
        assert_eq!(host_request.viewport, viewport);
        assert_eq!(host_request.state, interaction);

        let host_output = HostFrameOutput::new(host_request.state.clone());
        state.apply_host_frame_output(&host_output);
        assert_eq!(state.interaction, host_output.state);

        let frame_request = state.document_frame_request(
            viewport,
            RenderTarget::window("main", viewport),
            host_output,
        );
        assert_eq!(frame_request.previous_focused, Some(Some(focused)));

        let mut document = UiDocument::new(fixed_style(180.0, 80.0));
        let button = document.add_child(
            document.root,
            UiNode::container("button", fixed_style(80.0, 28.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Button")
                        .focusable(),
                ),
        );
        let mut measurer = ApproxTextMeasurer;
        let frame =
            process_document_frame(&mut document, &mut measurer, frame_request).expect("frame");
        state.apply_document_frame_output(&frame);

        assert_eq!(state.interaction, frame.host_output.state);
        assert_eq!(state.accessibility, frame.accessibility_state);
        assert!(state
            .layout
            .as_ref()
            .is_some_and(|layout| layout.children.iter().any(|child| child.name == "button")));
        assert!(state
            .accessibility
            .tree
            .as_ref()
            .is_some_and(|tree| tree.node(button).is_some()));
        let next_request = state.document_frame_request(
            viewport,
            RenderTarget::window("main", viewport),
            HostFrameOutput::new(state.interaction.clone()),
        );
        assert!(next_request.previous_layout_snapshot.is_some());
    }

    #[test]
    fn document_frame_emits_layout_animation_transitions_from_previous_snapshot() {
        let viewport = UiSize::new(220.0, 100.0);
        let mut measurer = ApproxTextMeasurer;
        let mut previous = UiDocument::new(fixed_style(220.0, 100.0));
        previous.add_child(
            previous.root,
            UiNode::container("panel", fixed_style(80.0, 32.0)).with_visual(UiVisual::panel(
                ColorRgba::new(24, 30, 36, 255),
                Some(StrokeStyle::new(ColorRgba::new(90, 100, 120, 255), 1.0)),
                4.0,
            )),
        );
        previous.compute_layout(viewport, &mut measurer).unwrap();
        let previous_snapshot = previous.layout_snapshot();

        let mut current = UiDocument::new(fixed_style(220.0, 100.0));
        current.add_child(
            current.root,
            UiNode::container("panel", fixed_style(140.0, 52.0)).with_visual(UiVisual::panel(
                ColorRgba::new(24, 30, 36, 255),
                Some(StrokeStyle::new(ColorRgba::new(90, 100, 120, 255), 1.0)),
                4.0,
            )),
        );
        let frame = process_document_frame(
            &mut current,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            )
            .previous_layout_snapshot(previous_snapshot)
            .layout_animation_options(LayoutAnimationOptions {
                progress: 0.5,
                ..Default::default()
            }),
        )
        .expect("frame");

        assert_eq!(frame.layout_animation_transitions.len(), 1);
        let transition = &frame.layout_animation_transitions[0];
        assert_eq!(transition.name, "panel");
        assert_eq!(transition.visual_rect.width, 110.0);
        assert_eq!(transition.visual_rect.height, 42.0);
        assert_eq!(transition.to_rect.width, 140.0);
        let painted_panel = frame
            .render_request
            .paint
            .items
            .iter()
            .find(|item| item.node == transition.node)
            .expect("painted animated panel");
        assert_eq!(painted_panel.transform, transition.transform);
    }

    #[test]
    fn document_frame_suppresses_layout_animation_when_reduced_motion_is_requested() {
        let viewport = UiSize::new(220.0, 100.0);
        let mut measurer = ApproxTextMeasurer;
        let mut previous = UiDocument::new(fixed_style(220.0, 100.0));
        previous.add_child(
            previous.root,
            UiNode::container("panel", fixed_style(80.0, 32.0)),
        );
        previous.compute_layout(viewport, &mut measurer).unwrap();

        let mut current = UiDocument::new(fixed_style(220.0, 100.0));
        current.add_child(
            current.root,
            UiNode::container("panel", fixed_style(140.0, 52.0)),
        );
        let frame = process_document_frame(
            &mut current,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            )
            .previous_layout_snapshot(previous.layout_snapshot())
            .layout_animation_options(LayoutAnimationOptions {
                progress: 0.5,
                ..Default::default()
            })
            .accessibility_preferences(AccessibilityPreferences::DEFAULT.reduced_motion(true)),
        )
        .expect("frame");

        assert!(frame.layout_animation_transitions.is_empty());
    }

    #[test]
    fn document_frame_applies_changed_accessibility_preferences() {
        let viewport = UiSize::new(180.0, 80.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(180.0, 80.0));
        document.add_child(
            document.root,
            UiNode::container("status", fixed_style(100.0, 24.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Status).label("Status"),
            ),
        );

        let first = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            )
            .accessibility_capabilities(AccessibilityCapabilities::SCREEN_READER)
            .accessibility_preferences(AccessibilityPreferences::DEFAULT),
        )
        .expect("first frame");
        let updated_preferences = AccessibilityPreferences::DEFAULT
            .high_contrast(true)
            .text_scale(1.35);

        let second = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(first.host_output.state),
            )
            .previous_accessibility_state(first.accessibility_state)
            .accessibility_capabilities(AccessibilityCapabilities::SCREEN_READER)
            .accessibility_preferences(updated_preferences),
        )
        .expect("second frame");

        assert_eq!(
            second
                .accessibility_requests
                .iter()
                .map(AccessibilityAdapterRequest::kind)
                .collect::<Vec<_>>(),
            vec![AccessibilityRequestKind::ApplyPreferences]
        );
        assert_eq!(
            second.accessibility_requests[0],
            AccessibilityAdapterRequest::ApplyPreferences(updated_preferences)
        );
    }

    #[test]
    fn document_frame_skips_tree_and_preferences_when_capabilities_are_missing() {
        let viewport = UiSize::new(180.0, 80.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(180.0, 80.0));
        document.add_child(
            document.root,
            UiNode::container("status", fixed_style(100.0, 24.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Status)
                    .label("Status")
                    .value("Ready")
                    .live_region(AccessibilityLiveRegion::Polite),
            ),
        );

        let frame = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            )
            .accessibility_capabilities(AccessibilityCapabilities::NONE)
            .accessibility_preferences(AccessibilityPreferences::DEFAULT.high_contrast(true)),
        )
        .expect("document frame");

        assert!(frame.accessibility_requests.is_empty());
        assert_eq!(frame.announcements.pending.len(), 1);
    }

    #[test]
    fn document_frame_does_not_republish_unchanged_tree_when_previous_state_matches() {
        let viewport = UiSize::new(180.0, 80.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(180.0, 80.0));
        let button = document.add_child(
            document.root,
            UiNode::container("play", fixed_style(80.0, 28.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Play")
                        .focusable(),
                ),
        );
        let state = HostInteractionState {
            focused: Some(button),
            ..HostInteractionState::default()
        };
        let first = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(state),
            )
            .accessibility_capabilities(AccessibilityCapabilities::SCREEN_READER),
        )
        .expect("first frame");

        let second = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(first.host_output.state.clone()),
            )
            .previous_accessibility_state(first.accessibility_state)
            .accessibility_capabilities(AccessibilityCapabilities::SCREEN_READER),
        )
        .expect("second frame");

        assert!(second.accessibility_requests.is_empty());
    }

    #[test]
    fn document_frame_republishes_tree_when_focus_changes() {
        let viewport = UiSize::new(180.0, 80.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(180.0, 80.0));
        let button = document.add_child(
            document.root,
            UiNode::container("play", fixed_style(80.0, 28.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Play")
                        .focusable(),
                ),
        );
        let first = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            )
            .accessibility_capabilities(AccessibilityCapabilities::SCREEN_READER),
        )
        .expect("first frame");
        let focused_state = HostInteractionState {
            focused: Some(button),
            ..first.host_output.state
        };

        let second = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(focused_state),
            )
            .previous_accessibility_state(first.accessibility_state)
            .accessibility_capabilities(AccessibilityCapabilities::SCREEN_READER),
        )
        .expect("second frame");

        assert_eq!(
            second
                .accessibility_requests
                .iter()
                .map(AccessibilityAdapterRequest::kind)
                .collect::<Vec<_>>(),
            vec![AccessibilityRequestKind::PublishTree]
        );
        let AccessibilityAdapterRequest::PublishTree { focused, .. } =
            &second.accessibility_requests[0]
        else {
            panic!("expected PublishTree");
        };
        assert_eq!(*focused, Some(button));
    }

    fn canvas_document(interaction: CanvasInteractionPolicy) -> (UiDocument, UiNodeId) {
        let mut document = UiDocument::new(fixed_style(320.0, 200.0));
        let canvas = document.add_child(
            document.root,
            UiNode::canvas(
                "viewport",
                "app.viewport",
                crate::LayoutStyle::from_taffy_style(fixed_style(160.0, 96.0).layout),
            ),
        );
        document.set_node_content(
            canvas,
            UiContent::Canvas(
                CanvasContent::new("app.viewport")
                    .native_viewport()
                    .interaction(interaction),
            ),
        );
        (document, canvas)
    }

    #[test]
    fn document_frame_carries_canvas_capture_state_across_frames() {
        let viewport = UiSize::new(320.0, 200.0);
        let mut measurer = ApproxTextMeasurer;
        let (mut document, canvas) = canvas_document(CanvasInteractionPolicy::NATIVE_VIEWPORT);

        let first = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            ),
        )
        .expect("first frame");

        assert_eq!(first.render_request.canvas_requests().len(), 1);
        assert_eq!(
            first.render_request.canvas_requests()[0].canvas.render_mode,
            CanvasRenderMode::NativeViewport
        );
        assert_eq!(
            first.host_output.state.canvas_host_capture.active_plans()[0].node,
            canvas
        );
        assert_eq!(
            first.canvas_host_capture_transition.platform_requests(),
            vec![
                PlatformRequest::Cursor(CursorRequest::SetGrab(CursorGrabMode::Locked)),
                PlatformRequest::Cursor(CursorRequest::SetVisible(false)),
            ]
        );

        let second = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(first.host_output.state),
            ),
        )
        .expect("second frame");

        assert!(second.canvas_host_capture_transition.is_empty());
        assert_eq!(
            second.host_output.state.canvas_host_capture.active_plans()[0].node,
            canvas
        );
    }

    #[test]
    fn document_frame_merges_host_and_generated_platform_service_requests() {
        let viewport = UiSize::new(320.0, 200.0);
        let mut measurer = ApproxTextMeasurer;
        let (mut document, _) = canvas_document(CanvasInteractionPolicy::NATIVE_VIEWPORT);
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
        .expect("frame");

        let mut allocator = PlatformRequestIdAllocator::new(20);
        let requests = frame.platform_service_requests(&mut allocator);

        assert_eq!(
            requests
                .iter()
                .map(|request| request.id)
                .collect::<Vec<_>>(),
            vec![
                PlatformRequestId::new(7),
                PlatformRequestId::new(20),
                PlatformRequestId::new(21),
            ]
        );
        assert_eq!(
            requests[0].request,
            PlatformRequest::Repaint(RepaintRequest::NextFrame)
        );
        assert_eq!(
            requests[1].request,
            PlatformRequest::Cursor(CursorRequest::SetGrab(CursorGrabMode::Locked))
        );
        assert_eq!(
            requests[2].request,
            PlatformRequest::Cursor(CursorRequest::SetVisible(false))
        );
        assert_eq!(allocator.next_value(), 22);

        let backend = BackendCapabilities::new("limited-host")
            .input(InputCapabilities::STANDARD)
            .services(PlatformServiceCapabilities {
                repaint: true,
                cursor_visible: true,
                ..PlatformServiceCapabilities::NONE
            });
        let platform_requests = frame.platform_requests();
        assert_eq!(
            platform_requests,
            vec![
                PlatformRequest::Repaint(RepaintRequest::NextFrame),
                PlatformRequest::Cursor(CursorRequest::SetGrab(CursorGrabMode::Locked)),
                PlatformRequest::Cursor(CursorRequest::SetVisible(false)),
            ]
        );

        let diagnostics = frame
            .platform_request_capability_diagnostics(&backend, CapabilityFallback::EmitDiagnostic);

        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.decision)
                .collect::<Vec<_>>(),
            vec![
                CapabilityDecision::UseFeature,
                CapabilityDecision::EmitDiagnostic,
                CapabilityDecision::UseFeature,
            ]
        );
        assert!(diagnostics.iter().any(|diagnostic| {
            !diagnostic.supported && diagnostic.summary.contains("cursor grab Locked")
        }));

        let host_diagnostics =
            frame.host_capability_diagnostics(&backend, CapabilityFallback::EmitDiagnostic);
        assert!(host_diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.requirement,
            BackendCapabilityRequirement::Input(InputCapabilityKind::RawMouseMotion)
        ) && diagnostic.decision
            == CapabilityDecision::EmitDiagnostic));
        assert!(host_diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.requirement,
            BackendCapabilityRequirement::Input(InputCapabilityKind::PointerLock)
        ) && diagnostic.decision
            == CapabilityDecision::EmitDiagnostic));
        assert!(host_diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.requirement,
            BackendCapabilityRequirement::PlatformRequest(PlatformRequest::Cursor(
                CursorRequest::SetGrab(CursorGrabMode::Locked)
            ))
        ) && diagnostic.decision
            == CapabilityDecision::EmitDiagnostic));

        let mut report = DiagnosticReport::new();
        report.host_document_frame_capabilities(
            &frame,
            &backend,
            CapabilityFallback::EmitDiagnostic,
        );
        assert!(report.summaries.iter().any(|summary| {
            summary.category == DiagnosticCategory::HostCapability
                && summary.label == "input:raw mouse motion"
        }));
    }

    #[test]
    fn document_frame_releases_canvas_capture_when_canvas_disappears() {
        let viewport = UiSize::new(320.0, 200.0);
        let mut measurer = ApproxTextMeasurer;
        let (mut document, canvas) = canvas_document(CanvasInteractionPolicy::NATIVE_VIEWPORT);
        let first = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            ),
        )
        .expect("first frame");

        document.set_node_content(canvas, UiContent::Empty);
        let released = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(first.host_output.state),
            ),
        )
        .expect("released frame");

        assert!(released
            .host_output
            .state
            .canvas_host_capture
            .active_plans()
            .is_empty());
        assert_eq!(
            released.canvas_host_capture_transition.platform_requests(),
            vec![
                PlatformRequest::Cursor(CursorRequest::SetGrab(CursorGrabMode::None)),
                PlatformRequest::Cursor(CursorRequest::SetVisible(true)),
            ]
        );
    }

    #[test]
    fn document_frame_tracks_editor_canvas_capture_without_cursor_requests() {
        let viewport = UiSize::new(320.0, 200.0);
        let mut measurer = ApproxTextMeasurer;
        let (mut document, canvas) = canvas_document(CanvasInteractionPolicy::EDITOR);

        let frame = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            ),
        )
        .expect("frame");

        assert_eq!(
            frame.host_output.state.canvas_host_capture.active_plans()[0].node,
            canvas
        );
        assert!(frame
            .canvas_host_capture_transition
            .platform_requests()
            .is_empty());
    }

    #[test]
    fn host_shell_frame_resizes_panel_and_returns_updated_layout() {
        let mut workspace = ShellWorkspaceState::new();
        workspace.upsert_panel(
            ShellPanelState::new("inspector", "Inspector", ShellRegion::RightPanel, 200.0)
                .with_limits(120.0, Some(400.0))
                .resizable(true),
        );

        let output = process_shell_frame(
            HostShellFrameRequest::new(UiRect::new(0.0, 0.0, 800.0, 600.0), workspace)
                .event(HostShellEvent::resize_panel("inspector", 75.0)),
        );

        assert!(output.changed);
        assert_eq!(
            output.workspace.panel("inspector").unwrap().extent.current,
            275.0
        );
        assert_eq!(
            output.layout.panel_rect("inspector"),
            Some(UiRect::new(525.0, 0.0, 275.0, 600.0))
        );
    }

    #[test]
    fn host_shell_frame_ignores_non_resizable_and_missing_panel_resize() {
        let mut workspace = ShellWorkspaceState::new();
        workspace.upsert_panel(ShellPanelState::new(
            "inspector",
            "Inspector",
            ShellRegion::RightPanel,
            200.0,
        ));

        let output = process_shell_frame(
            HostShellFrameRequest::new(UiRect::new(0.0, 0.0, 800.0, 600.0), workspace).events([
                HostShellEvent::resize_panel("missing", 50.0),
                HostShellEvent::resize_panel("inspector", 50.0),
            ]),
        );

        assert!(!output.changed);
        assert_eq!(
            output.workspace.panel("inspector").unwrap().extent.current,
            200.0
        );
        assert_eq!(
            output.layout.panel_rect("inspector"),
            Some(UiRect::new(600.0, 0.0, 200.0, 600.0))
        );
    }

    #[test]
    fn host_shell_frame_focus_scroll_and_collapse_update_workspace_state() {
        let mut drawer = ShellPanelState::new("drawer", "Drawer", ShellRegion::RightPanel, 220.0);
        drawer.collapsed_extent = 36.0;
        let mut workspace = ShellWorkspaceState::new();
        workspace.upsert_panel(drawer);

        let output = process_shell_frame(
            HostShellFrameRequest::new(UiRect::new(10.0, 20.0, 640.0, 360.0), workspace).events([
                HostShellEvent::focus_panel("drawer", FocusRestoreTarget::Node(UiNodeId(9))),
                HostShellEvent::scroll_panel("drawer", UiPoint::new(0.0, 128.0)),
                HostShellEvent::collapse_panel("drawer"),
            ]),
        );

        let drawer = output.workspace.panel("drawer").unwrap();
        assert!(output.changed);
        assert_eq!(output.workspace.focused_panel.as_deref(), Some("drawer"));
        assert_eq!(
            output.workspace.restored_focus,
            Some(FocusRestoreTarget::Node(UiNodeId(9)))
        );
        assert_eq!(drawer.scroll_offset, UiPoint::new(0.0, 128.0));
        assert!(drawer.collapsed);
        assert_eq!(drawer.extent.current, 36.0);
        assert_eq!(
            output.layout.panel_rect("drawer"),
            Some(UiRect::new(614.0, 20.0, 36.0, 360.0))
        );
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

    #[test]
    fn text_pointer_edit_event_preserves_drag_selection_on_pointer_up() {
        let point = UiPoint::new(42.0, 8.0);
        let (phase, position, selecting) =
            text_pointer_edit_event(&UiInputEvent::PointerUp(point), true)
                .expect("pressed text input should commit pointer edits");

        assert_eq!(phase, WidgetValueEditPhase::Commit);
        assert_eq!(position, point);
        assert!(selecting);
    }
}
