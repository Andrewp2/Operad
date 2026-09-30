//! Host adapter contracts for pre-paint interaction state.
//!
//! Backends such as winit/wgpu, test harnesses, or app-owned hosts
//! can use these data contracts to feed hover, press, focus, drag capture,
//! wheel targeting, text/IME, and shortcut routing state into Operad before a
//! document is painted.

use std::fmt;

use crate::accessibility::{
    push_supported_accessibility_request, AccessibilityAdapterRequest,
    AccessibilityAnnouncementQueue, AccessibilityCapabilities, AccessibilityLiveRegionSnapshot,
    AccessibilityPreferences, FocusRestoreTarget,
};
use crate::actions::{
    action_target_accepts_pointer_click, action_target_enabled, resolve_action_target,
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
    AccessibilityTree, DirtyFlags, KeyCode, KeyModifiers, LayoutSnapshot, TextMeasurer, UiDocument,
    UiFocusState, UiInputEvent, UiInputResult, UiNodeId, UiPoint, UiRect, UiSize, WidgetAction,
    WidgetActionBinding, WidgetActionQueue, WidgetValueEditPhase,
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

/// Composition lifetime from ordered input, independent of the last frame's
/// rendered snapshot. An empty preedit can precede a valid native commit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HostTextCompositionState {
    /// No composition is awaiting completion.
    #[default]
    Inactive,
    /// A nonempty draft is active or has been published by the application.
    Preedit,
    /// An empty preedit was routed; a valid commit may still follow it.
    EmptyPreedit,
}

impl HostTextCompositionState {
    pub const fn is_active(self) -> bool {
        !matches!(self, Self::Inactive)
    }

    pub(crate) fn from_session(session: &TextImeSession) -> Self {
        match &session.composition {
            Some(range) if range.start == range.end => Self::EmptyPreedit,
            Some(_) => Self::Preedit,
            None => Self::Inactive,
        }
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
    /// Ordered input lifetime; the IME snapshot can still describe the frame
    /// before the application applies the latest composition events.
    pub text_composition: HostTextCompositionState,
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
        self.text_composition = HostTextCompositionState::from_session(&session);
        self.text_ime = Some(session.clone());
        PlatformRequest::TextIme(TextImeRequest::Activate(session))
    }

    pub fn activate_text_ime_for(
        &mut self,
        target: UiNodeId,
        session: TextImeSession,
    ) -> PlatformRequest {
        self.text_target = Some(target);
        self.text_composition = HostTextCompositionState::from_session(&session);
        self.text_ime = Some(session.clone());
        PlatformRequest::TextIme(TextImeRequest::Activate(session))
    }

    pub fn update_text_ime(&mut self, session: TextImeSession) -> PlatformRequest {
        self.text_composition = HostTextCompositionState::from_session(&session);
        self.text_target = self
            .text_target
            .or_else(|| text_target_from_input(&session.input));
        self.text_ime = Some(session.clone());
        PlatformRequest::TextIme(TextImeRequest::Update(session))
    }

    pub fn deactivate_text_ime(&mut self, input: TextInputId) -> PlatformRequest {
        self.text_ime = None;
        self.text_target = None;
        self.text_composition = HostTextCompositionState::Inactive;
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
                self.text_composition = HostTextCompositionState::Inactive;
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

/// An ordered document event and any gesture derived from platform input.
/// Keeping them together preserves ordering and lets document-owned interactions
/// (such as automatic scrollbars) suppress generic widget actions.
#[derive(Debug, Clone, PartialEq)]
pub struct HostInputEvent {
    pub ui_event: Option<UiInputEvent>,
    pub gesture: Option<GestureEvent>,
    /// Position before this raw pointer event, when a gesture is active. This
    /// survives rebuilt documents and distinguishes motion from a stationary
    /// release even if the displayed field moved or changed font after press.
    pub previous_pointer_position: Option<UiPoint>,
    /// Set when this event has been applied to a document. Its actions retain
    /// the geometry and values at that event, even after later input changes them.
    pub document_result: Option<HostDocumentInputResult>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostDocumentInputResult {
    pub previous_focus: UiFocusState,
    pub input: Option<UiInputResult>,
    pub actions: Vec<WidgetAction>,
    gesture_action: Option<usize>,
}

impl HostDocumentInputResult {
    pub(crate) fn gesture_action(&self) -> Option<&WidgetAction> {
        self.gesture_action
            .and_then(|index| self.actions.get(index))
    }
}

impl HostInputEvent {
    pub fn new(ui_event: Option<UiInputEvent>, gesture: Option<GestureEvent>) -> Self {
        Self {
            ui_event,
            gesture,
            previous_pointer_position: None,
            document_result: None,
        }
    }
}

impl From<UiInputEvent> for HostInputEvent {
    fn from(event: UiInputEvent) -> Self {
        Self::new(Some(event), None)
    }
}

impl From<GestureEvent> for HostInputEvent {
    fn from(gesture: GestureEvent) -> Self {
        Self::new(None, Some(gesture))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostFrameOutput {
    pub state: HostInteractionState,
    pub events: Vec<HostInputEvent>,
    pub commands: Vec<HostCommandDispatch>,
    pub platform_requests: Vec<PlatformServiceRequest>,
    pub platform_responses: Vec<PlatformServiceResponse>,
}

impl HostFrameOutput {
    pub fn new(state: HostInteractionState) -> Self {
        Self {
            state,
            events: Vec::new(),
            commands: Vec::new(),
            platform_requests: Vec::new(),
            platform_responses: Vec::new(),
        }
    }

    pub fn ui_events(&self) -> impl DoubleEndedIterator<Item = &UiInputEvent> {
        self.events
            .iter()
            .filter_map(|event| event.ui_event.as_ref())
    }

    pub fn gestures(&self) -> impl DoubleEndedIterator<Item = &GestureEvent> {
        self.events
            .iter()
            .filter_map(|event| event.gesture.as_ref())
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
    resolve_target: impl FnMut(&RawInputEvent, &HostInteractionState) -> Option<UiNodeId>,
) -> HostFrameOutput {
    process_host_frame_input_with_filter(request, wheel_line_size, resolve_target, |_, _| true)
}

/// Run interception against the state produced by all preceding events. Keeping
/// interception here preserves gesture order and keyboard/text pairing.
pub(crate) fn process_host_frame_input_with_filter(
    request: HostFrameRequest,
    wheel_line_size: f32,
    resolve_target: impl FnMut(&RawInputEvent, &HostInteractionState) -> Option<UiNodeId>,
    accepts_event: impl FnMut(&RawInputEvent, &HostInteractionState) -> bool,
) -> HostFrameOutput {
    let mut routing = RawInputRouting {
        resolve_target,
        accepts_event,
    };
    let wheel_scale = (wheel_line_size, request.viewport);
    match process_host_input(request, wheel_scale, &mut routing) {
        Ok(output) => output,
        Err(never) => match never {},
    }
}

trait HostInputRouting {
    type Error;

    fn accepts_event(
        &mut self,
        event: &RawInputEvent,
        state: &HostInteractionState,
    ) -> Result<bool, Self::Error>;
    fn resolve_target(
        &mut self,
        event: &RawInputEvent,
        state: &HostInteractionState,
    ) -> Option<UiNodeId>;
    fn process_event(&mut self, _event: &mut HostInputEvent, _state: &mut HostInteractionState) {}
}

struct RawInputRouting<Resolve, Accept> {
    resolve_target: Resolve,
    accepts_event: Accept,
}

impl<Resolve, Accept> HostInputRouting for RawInputRouting<Resolve, Accept>
where
    Resolve: FnMut(&RawInputEvent, &HostInteractionState) -> Option<UiNodeId>,
    Accept: FnMut(&RawInputEvent, &HostInteractionState) -> bool,
{
    type Error = std::convert::Infallible;

    fn accepts_event(
        &mut self,
        event: &RawInputEvent,
        state: &HostInteractionState,
    ) -> Result<bool, Self::Error> {
        Ok((self.accepts_event)(event, state))
    }

    fn resolve_target(
        &mut self,
        event: &RawInputEvent,
        state: &HostInteractionState,
    ) -> Option<UiNodeId> {
        (self.resolve_target)(event, state)
    }
}

struct DocumentInputRouting<'a, M, Accept> {
    document: &'a mut UiDocument,
    measurer: &'a mut M,
    viewport: UiSize,
    accepts_event: Accept,
}

impl<M, Accept> HostInputRouting for DocumentInputRouting<'_, M, Accept>
where
    M: TextMeasurer,
    Accept: FnMut(&UiDocument, &RawInputEvent, &HostInteractionState) -> bool,
{
    type Error = taffy::TaffyError;

    fn accepts_event(
        &mut self,
        event: &RawInputEvent,
        state: &HostInteractionState,
    ) -> Result<bool, Self::Error> {
        // Unchanged geometry takes the document's cached fast path. Scroll and
        // interaction styles become visible to the next event's hit test.
        self.document.compute_layout(self.viewport, self.measurer)?;
        Ok((self.accepts_event)(self.document, event, state))
    }

    fn resolve_target(
        &mut self,
        event: &RawInputEvent,
        state: &HostInteractionState,
    ) -> Option<UiNodeId> {
        document_input_target(event, state, self.document)
    }

    fn process_event(&mut self, event: &mut HostInputEvent, state: &mut HostInteractionState) {
        let previous_focus = self.document.focus.clone();
        process_document_input_event(self.document, event, state, previous_focus);
    }
}

pub(crate) fn process_document_input_with_filter(
    document: &mut UiDocument,
    measurer: &mut impl TextMeasurer,
    request: HostFrameRequest,
    wheel_scale: (f32, UiSize),
    accepts_event: impl FnMut(&UiDocument, &RawInputEvent, &HostInteractionState) -> bool,
) -> Result<HostFrameOutput, taffy::TaffyError> {
    let viewport = request.viewport;
    process_host_input(
        request,
        wheel_scale,
        &mut DocumentInputRouting {
            document,
            measurer,
            viewport,
            accepts_event,
        },
    )
}

fn record_host_input(
    output: &mut HostFrameOutput,
    state: &mut HostInteractionState,
    routing: &mut impl HostInputRouting,
    mut event: HostInputEvent,
) {
    routing.process_event(&mut event, state);
    output.events.push(event);
}

fn process_host_input<R: HostInputRouting>(
    request: HostFrameRequest,
    wheel_scale: (f32, UiSize),
    routing: &mut R,
) -> Result<HostFrameOutput, R::Error> {
    let HostFrameRequest {
        viewport: _,
        mut state,
        raw_input,
        platform_responses,
    } = request;
    let mut output = HostFrameOutput::new(state.clone());
    output.platform_responses = platform_responses;

    for event in raw_input {
        if let RawInputEvent::Composition(composition) = &event {
            if !state
                .text_ime
                .as_ref()
                .is_some_and(|session| session.input == composition.input)
            {
                continue;
            }
        }
        // The document has one press owner. Raw hooks still see other pointers
        // and buttons, but they cannot update or finish that widget gesture.
        let accepts_pointer_event = match &event {
            RawInputEvent::Pointer(pointer) => {
                state
                    .drag_capture
                    .is_none_or(|capture| capture.pointer_id == pointer.pointer_id)
                    && state.gesture_tracker.accepts_pointer_event(*pointer)
            }
            _ => true,
        };
        if !routing.accepts_event(&event, &state)? {
            if let RawInputEvent::Pointer(pointer) = &event {
                if accepts_pointer_event
                    && matches!(
                        pointer.kind,
                        PointerEventKind::Up(_) | PointerEventKind::Cancel
                    )
                {
                    // A hook may intercept release after allowing the press.
                    // End widget ownership without committing or clicking.
                    let gesture = state
                        .gesture_tracker
                        .pointer_cancel(pointer.pointer_id, pointer.position);
                    if let Some(cancel) = &gesture {
                        apply_host_frame_gesture(&mut state, cancel);
                    }
                    record_host_input(
                        &mut output,
                        &mut state,
                        routing,
                        HostInputEvent::new(Some(UiInputEvent::PointerCancel), gesture),
                    );
                    clear_host_frame_capture_after_terminal_event(&mut state, &event);
                }
            }
            continue;
        }
        if !accepts_pointer_event {
            continue;
        }
        if let RawInputEvent::Composition(composition) = &event {
            if let Some(target) = state.text_target {
                record_host_input(
                    &mut output,
                    &mut state,
                    routing,
                    UiInputEvent::Composition {
                        target: Some(target),
                        event: composition.event.clone(),
                    }
                    .into(),
                );
            }
            continue;
        }
        let mut ui_events = event.to_ui_input_events_with_wheel_scale(wheel_scale.0, wheel_scale.1);
        let ui_event = ui_events.next();

        let target = match event {
            RawInputEvent::Pointer(_) | RawInputEvent::Wheel(_) => {
                routing.resolve_target(&event, &state)
            }
            RawInputEvent::Keyboard(_)
            | RawInputEvent::Text(_)
            | RawInputEvent::Composition(_)
            | RawInputEvent::Focus(_) => None,
        };
        let previous_pointer_position = match &event {
            RawInputEvent::Pointer(pointer) => {
                state.gesture_tracker.pointer_position(pointer.pointer_id)
            }
            _ => None,
        };
        let gesture = host_frame_gesture_for_event(&mut state, &event, target);
        if let Some(gesture) = &gesture {
            apply_host_frame_gesture(&mut state, gesture);
        } else {
            clear_host_frame_capture_after_terminal_event(&mut state, &event);
        }
        if ui_event.is_some() || gesture.is_some() {
            let mut input = HostInputEvent::new(ui_event, gesture);
            input.previous_pointer_position = previous_pointer_position;
            record_host_input(&mut output, &mut state, routing, input);
        }
        for ui_event in ui_events {
            record_host_input(&mut output, &mut state, routing, ui_event.into());
        }
    }

    output.state = state;
    Ok(output)
}

pub(crate) fn document_input_target(
    event: &RawInputEvent,
    state: &HostInteractionState,
    document: &UiDocument,
) -> Option<UiNodeId> {
    // Release must hit-test for clicks. Active drags already retain their target
    // in the gesture tracker, and canvas hooks retain their own pointer capture.
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
            .or_else(|| {
                document
                    .pointer_input_hit(pointer.position)
                    .0
                    .and_then(crate::HitTestResult::target)
            }),
        RawInputEvent::Wheel(wheel) => document.hit_test(wheel.position),
        RawInputEvent::Keyboard(_)
        | RawInputEvent::Text(_)
        | RawInputEvent::Composition(_)
        | RawInputEvent::Focus(_) => None,
    }
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
        RawInputEvent::Keyboard(_)
        | RawInputEvent::Text(_)
        | RawInputEvent::Composition(_)
        | RawInputEvent::Focus(_) => None,
    }
}

fn host_frame_gesture_for_event(
    state: &mut HostInteractionState,
    event: &RawInputEvent,
    target: Option<UiNodeId>,
) -> Option<GestureEvent> {
    match event {
        RawInputEvent::Pointer(pointer) => match pointer.kind {
            PointerEventKind::Down(_) => state.gesture_tracker.pointer_down(target, *pointer),
            PointerEventKind::Move => state.gesture_tracker.pointer_move(target, *pointer),
            PointerEventKind::Up(_) => state.gesture_tracker.pointer_up(target, *pointer),
            PointerEventKind::Cancel => state
                .gesture_tracker
                .pointer_cancel(pointer.pointer_id, pointer.position),
        },
        RawInputEvent::Wheel(wheel) => Some(PointerGestureTracker::wheel(target, *wheel)),
        RawInputEvent::Keyboard(_)
        | RawInputEvent::Text(_)
        | RawInputEvent::Composition(_)
        | RawInputEvent::Focus(_) => None,
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
    /// Focused node before this document frame's UI events were applied.
    pub previous_focused: Option<UiNodeId>,
    /// Press owner before this document frame's UI events were applied.
    pub previous_pressed: Option<UiNodeId>,
    pub host_output: HostFrameOutput,
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
    pub fn input_results(&self) -> impl Iterator<Item = &UiInputResult> {
        self.input_events().filter_map(|(_, input)| input)
    }

    pub(crate) fn input_events(
        &self,
    ) -> impl Iterator<Item = (&HostInputEvent, Option<&UiInputResult>)> {
        self.host_output.events.iter().map(|event| {
            (
                event,
                event
                    .document_result
                    .as_ref()
                    .and_then(|result| result.input.as_ref()),
            )
        })
    }

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
    if let Some(focused) = document.focus.focused {
        state.focused = Some(focused);
    }
    let initial_focus = host_output
        .events
        .iter()
        .find_map(|event| {
            event
                .document_result
                .as_ref()
                .map(|result| result.previous_focus.clone())
        })
        .unwrap_or_else(|| document.focus.clone());
    let authored_focused = initial_focus.focused;
    let previous_focused_for_actions =
        authored_focused.or_else(|| previous_focused.unwrap_or(state.focused));
    let previous_pressed = initial_focus.pressed;
    if host_output.ui_events().next().is_none()
        && host_output
            .events
            .iter()
            .all(|event| event.document_result.is_none())
    {
        let mut actions = WidgetActionQueue::new();
        push_focus_transition_actions(
            document,
            &mut actions,
            previous_focused_for_actions,
            state.focused,
        );
        let actions = actions.into_vec();
        if !actions.is_empty() {
            let mut event = HostInputEvent::new(None, None);
            event.document_result = Some(HostDocumentInputResult {
                previous_focus: initial_focus,
                input: None,
                actions,
                gesture_action: None,
            });
            host_output.events.insert(0, event);
        }
    }
    let mut action_focus = document.focus.clone();
    action_focus.focused = previous_focused_for_actions;
    for event in &mut host_output.events {
        if event.document_result.is_none() {
            document.compute_layout(viewport, measurer)?;
            process_document_input_event(document, event, &mut state, action_focus);
        }
        action_focus = document.focus.clone();
    }
    host_output.state = state.clone();

    document.compute_layout(viewport, measurer)?;
    #[cfg(feature = "widgets")]
    {
        let cursor = document.pointer_position;
        if crate::widgets::tooltip::add_active_node_tooltip(document, viewport, cursor).is_some() {
            document.compute_layout(viewport, measurer)?;
        }
    }
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
        .resource_updates(document.resource_updates().iter().cloned())
        .node_interactions(node_interactions)
        .dirty_flags(dirty_flags)
        .options(render_options);
    let canvas_host_capture_transition = state
        .canvas_host_capture
        .sync(render_request.canvas_host_capture_plans());
    host_output.state = state.clone();

    Ok(HostDocumentFrameOutput {
        previous_focused: previous_focused_for_actions,
        previous_pressed,
        host_output,
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

/// Return actions captured against the document state at each input event.
pub fn collect_document_widget_actions(frame: &HostDocumentFrameOutput) -> Vec<WidgetAction> {
    frame
        .host_output
        .events
        .iter()
        .filter_map(|event| event.document_result.as_ref())
        .flat_map(|result| result.actions.iter().cloned())
        .collect()
}

pub(crate) fn process_document_input_event(
    document: &mut UiDocument,
    event: &mut HostInputEvent,
    state: &mut HostInteractionState,
    previous_focus: UiFocusState,
) {
    let text_pointer_changed = match &event.ui_event {
        Some(UiInputEvent::PointerMove(point) | UiInputEvent::PointerUp(point)) => {
            event
                .previous_pointer_position
                .or(document.pointer_position)
                != Some(*point)
        }
        _ => true,
    };
    let click_target = match &event.gesture {
        Some(GestureEvent::Click(click)) => {
            action_target_accepts_pointer_click(document, click.target, click.button)
                .then_some(click.target)
        }
        _ => None,
    };
    let input = event.ui_event.as_ref().map(|event| {
        let input = document.handle_input_with_click_target(event.clone(), click_target);
        state.apply_input_result(input.clone());
        input
    });
    let mut queue = WidgetActionQueue::new();
    let scrollbar_handled = input
        .as_ref()
        .is_some_and(|input| input.scrollbar_target.is_some());
    if let (Some(event), Some(input)) = (&event.ui_event, &input) {
        // A valid cancellation must reach the original model even though it
        // revokes the session before subsequent events are routed.
        let obsolete_composition = matches!(event, UiInputEvent::Composition { target, .. }
            if state.text_ime.is_some() && *target != state.text_target);
        if let Some(target) = state.text_target {
            let pointer_target = if matches!(event, UiInputEvent::PointerDown(_)) {
                input.pressed
            } else {
                previous_focus.pressed
            };
            let pointer_selection = state.text_composition.is_active()
                && !scrollbar_handled
                && text_pointer_changed
                && text_pointer_edit_target(document, pointer_target, event).is_some_and(
                    |(owner, _, point, _)| {
                        owner == target
                            && document.text_input_pointer_geometry(owner, point).is_some()
                    },
                );
            let explicit_cancel = matches!(
                event,
                UiInputEvent::Key {
                    key: KeyCode::Escape,
                    ..
                }
            ) || matches!(event, UiInputEvent::Composition {
                    target: Some(owner), event: crate::TextCompositionEvent::Cancel,
                } if *owner == target);
            let cancels_draft =
                state.text_composition.is_active() && (pointer_selection || explicit_cancel);
            if input.focused != Some(target) || cancels_draft {
                // Revoke routing immediately, including later events in this
                // batch. Runtime sync deactivates the retained platform session
                // before activating a fresh input ID.
                state.text_target = None;
                state.text_composition = HostTextCompositionState::Inactive;
            }
        }
        if let UiInputEvent::Composition { target, event } = event {
            if target.is_some() && *target == state.text_target && *target == input.focused {
                // Empty preedit can precede a queued native commit, including
                // across a frame boundary with no visible draft.
                state.text_composition = match event {
                    crate::TextCompositionEvent::Preedit { text, .. } if text.is_empty() => {
                        HostTextCompositionState::EmptyPreedit
                    }
                    crate::TextCompositionEvent::Preedit { .. } => {
                        HostTextCompositionState::Preedit
                    }
                    _ => HostTextCompositionState::Inactive,
                };
            }
        }
        push_focus_transition_actions(document, &mut queue, previous_focus.focused, input.focused);
        if !scrollbar_handled && !obsolete_composition {
            push_document_input_actions(
                document,
                &mut queue,
                event,
                input.focused,
                input.pressed,
                previous_focus.pressed,
                text_pointer_changed,
            );
        }
        if let Some(target) = input.scrolled {
            if let (Some(binding), Some(scroll)) = (
                action_binding(document, target),
                document.scroll_state(target),
            ) {
                queue.push(WidgetAction::scroll(target, binding, scroll));
            }
        }
    }
    let click_matches_document = match (&event.ui_event, &event.gesture) {
        (Some(UiInputEvent::PointerUp(_)), Some(GestureEvent::Click(click))) => input
            .as_ref()
            .is_some_and(|input| input.clicked == Some(click.target)),
        _ => true,
    };
    let mut gesture_action = None;
    if !scrollbar_handled && click_matches_document {
        if let Some(gesture) = &event.gesture {
            if let Some(action) =
                WidgetAction::from_gesture_event_for_document(document, gesture, |id| {
                    action_binding(document, id)
                })
            {
                gesture_action = Some(queue.len());
                queue.push(action);
            }
            if let Some(action) = drop_target_drag_action_from_gesture(document, gesture) {
                queue.push(action);
            }
        }
    }
    event.document_result = Some(HostDocumentInputResult {
        previous_focus,
        input,
        actions: queue.into_vec(),
        gesture_action,
    });
}

fn push_document_input_actions(
    document: &UiDocument,
    queue: &mut WidgetActionQueue,
    event: &UiInputEvent,
    focused: Option<UiNodeId>,
    pressed: Option<UiNodeId>,
    previous_pressed: Option<UiNodeId>,
    text_pointer_changed: bool,
) {
    // Focus transitions were already dispatched. Navigation must not become a
    // text edit on the newly focused control.
    if event.focus_direction().is_some() {
        return;
    }
    if let UiInputEvent::Composition {
        target,
        event: composition,
    } = event
    {
        if let Some(target) = target.filter(|target| Some(*target) == focused) {
            if let Some(binding) = action_binding(document, target) {
                queue.push(WidgetAction::text_edit(
                    target,
                    binding,
                    UiInputEvent::Composition {
                        target: Some(target),
                        event: composition.clone(),
                    },
                ));
            }
        }
        return;
    }
    let pointer_target = if matches!(event, UiInputEvent::PointerDown(_)) {
        pressed
    } else {
        previous_pressed
    };
    if let Some((target, phase, position, selecting)) =
        text_pointer_edit_target(document, pointer_target, event)
    {
        // A press already placed the caret. Reinterpreting an unchanged point
        // after focus restyles the field would create a selection on release.
        if !text_pointer_changed {
            return;
        }
        if let Some(binding) = action_binding(document, target) {
            let target_rect = document
                .nodes()
                .get(target.0)
                .map(|node| node.layout.rect)
                .unwrap_or_else(|| UiRect::new(0.0, 0.0, 0.0, 0.0));
            let Some(geometry) = document.text_input_pointer_geometry(target, position) else {
                return;
            };
            let mut action = WidgetAction::text_pointer_edit(
                target,
                binding,
                event.clone(),
                phase,
                position,
                target_rect,
                selecting,
            );
            if let crate::WidgetActionKind::TextEdit(edit) = &mut action.kind {
                edit.geometry = Some(geometry);
            }
            queue.push(action);
            return;
        }
    }
    let Some(target) = focused else {
        return;
    };
    let Some(binding) = action_binding(document, target) else {
        return;
    };
    if document.node_is_text_control(target)
        && matches!(event, UiInputEvent::TextInput(_) | UiInputEvent::Key { .. })
    {
        queue.push(WidgetAction::text_edit(target, binding, event.clone()));
        return;
    }
    if let UiInputEvent::Key { key, modifiers } = event {
        queue.push_key_activation(target, binding, *key, *modifiers);
    }
}

pub(crate) fn document_focus_transition_event(
    document: &UiDocument,
    previous: UiFocusState,
    current: Option<UiNodeId>,
) -> Option<HostInputEvent> {
    let mut queue = WidgetActionQueue::new();
    push_focus_transition_actions(document, &mut queue, previous.focused, current);
    if queue.is_empty() {
        return None;
    }
    Some(HostInputEvent {
        ui_event: None,
        gesture: None,
        previous_pointer_position: None,
        document_result: Some(HostDocumentInputResult {
            previous_focus: previous,
            input: None,
            actions: queue.into_vec(),
            gesture_action: None,
        }),
    })
}

fn push_focus_transition_actions(
    document: &UiDocument,
    queue: &mut WidgetActionQueue,
    previous: Option<UiNodeId>,
    current: Option<UiNodeId>,
) {
    if previous == current {
        return;
    }
    if let Some(previous) = previous {
        if document.node_is_text_control(previous) {
            if let Some(binding) = action_binding(document, previous) {
                queue.push(WidgetAction::text_edit(
                    previous,
                    binding.clone(),
                    UiInputEvent::Composition {
                        target: Some(previous),
                        event: crate::TextCompositionEvent::Cancel,
                    },
                ));
                queue.focus(previous, binding, false);
            }
        }
    }
    if let Some(current) = current {
        if document.node_is_text_control(current) {
            if let Some(binding) = action_binding(document, current) {
                queue.focus(current, binding, true);
            }
        }
    }
}

fn text_pointer_edit_target(
    document: &UiDocument,
    pressed: Option<UiNodeId>,
    event: &UiInputEvent,
) -> Option<(UiNodeId, WidgetValueEditPhase, UiPoint, bool)> {
    let target = pressed.filter(|target| {
        document.node_is_text_control(*target) && action_target_enabled(document, *target)
    })?;
    let (phase, position, selecting) = match event {
        UiInputEvent::PointerDown(point) => (WidgetValueEditPhase::Begin, *point, false),
        UiInputEvent::PointerMove(point) => (WidgetValueEditPhase::Update, *point, true),
        UiInputEvent::PointerUp(point) => (WidgetValueEditPhase::Commit, *point, true),
        _ => return None,
    };
    Some((target, phase, position, selecting))
}

fn action_binding(document: &UiDocument, id: UiNodeId) -> Option<WidgetActionBinding> {
    document
        .nodes()
        .get(id.0)
        .and_then(|node| node.action.clone())
}

fn drop_target_drag_action_from_gesture(
    document: &UiDocument,
    event: &GestureEvent,
) -> Option<WidgetAction> {
    let GestureEvent::Drag(gesture) = event else {
        return None;
    };
    let source = drag_source_action_target_for_hit(document, gesture.target)?;
    let hit = document.hit_test(gesture.current)?;
    let target = drop_action_target_for_hit(document, hit)?;
    if document.node_is_logical_descendant_or_self(source, target) {
        return None;
    }
    let binding = action_binding(document, target)?;
    let mut drag = *gesture;
    drag.target = target;
    WidgetAction::drag_from_gesture(&drag, binding)
}

fn drag_source_action_target_for_hit(document: &UiDocument, hit: UiNodeId) -> Option<UiNodeId> {
    action_target_for_accessibility_action(document, hit, "drag.start")
}

fn drop_action_target_for_hit(document: &UiDocument, hit: UiNodeId) -> Option<UiNodeId> {
    action_target_for_accessibility_action(document, hit, "drop.accept")
}

fn action_target_for_accessibility_action(
    document: &UiDocument,
    hit: UiNodeId,
    accessibility_action_id: &str,
) -> Option<UiNodeId> {
    resolve_action_target(document, hit, |id| {
        let node = document.nodes().get(id.0)?;
        let has_action = node.accessibility.as_ref().is_some_and(|accessibility| {
            accessibility
                .actions
                .iter()
                .any(|action| action.id == accessibility_action_id)
        });
        (has_action && node.action.is_some()).then_some(())
    })
    .map(|(target, _, _)| target)
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
        AccessibilityAdapterRequest, AccessibilityCapabilities, AccessibilityPreferences,
        AccessibilityRequestKind, FocusRestoreTarget,
    };
    use crate::commands::{Command, CommandMeta};
    use crate::diagnostics::{DiagnosticCategory, DiagnosticReport};
    use crate::input::{
        DragGesture, PointerButton, PointerClick, PointerEventKind, PointerId, RawKeyboardEvent,
        RawPointerEvent, RawTextInputEvent, RawWheelEvent, WheelPhase,
    };
    use crate::platform::{
        CapabilityDecision, CursorGrabMode, CursorRequest, InputCapabilities, InputCapabilityKind,
        LogicalRect, PlatformRequestId, PlatformRequestIdAllocator, PlatformServiceCapabilities,
        TextRange,
    };
    use crate::shell::{ShellPanelState, ShellRegion};
    use crate::{
        length, AccessibilityAction, AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole,
        ApproxTextMeasurer, CanvasContent, CanvasInteractionPolicy, CanvasRenderMode, ColorRgba,
        InputBehavior, KeyModifiers, LayoutStyle, StrokeStyle, UiContent, UiDocument, UiNode,
        UiNodeStyle, UiPoint, UiVisual, WidgetActionKind, WidgetDragPhase,
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
            scrollbar_target: None,
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
            first.ui_events().cloned().collect::<Vec<_>>(),
            vec![UiInputEvent::PointerDown(UiPoint::new(10.0, 10.0))]
        );
        assert!(matches!(
            &first.gestures().cloned().collect::<Vec<_>>()[..],
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
        assert!(under_threshold.gestures().next().is_none());
        assert_eq!(
            under_threshold.ui_events().cloned().collect::<Vec<_>>(),
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
        let [GestureEvent::Drag(begin)] = &drag_begin.gestures().cloned().collect::<Vec<_>>()[..]
        else {
            panic!("expected one drag begin gesture");
        };
        assert_eq!(begin.target, target);
        assert_eq!(begin.phase, GesturePhase::Begin);
        assert_eq!(begin.total_delta, UiPoint::new(10.0, 4.0));
    }

    #[test]
    fn pointer_chords_cannot_replace_or_release_the_press_owner() {
        for owner in [PointerButton::Primary, PointerButton::Secondary] {
            let other = if owner == PointerButton::Primary {
                PointerButton::Secondary
            } else {
                PointerButton::Primary
            };
            for dragging in [false, true] {
                for intercepted in [false, true] {
                    let target = UiNodeId(3);
                    let neighbor = UiNodeId(4);
                    let viewport = UiSize::new(200.0, 120.0);
                    let pressed = process_host_frame_input_with_target_resolver(
                        HostFrameRequest::new(viewport, HostInteractionState::default())
                            .raw_event(raw_pointer(PointerEventKind::Down(owner), 4.0, 4.0, 1)),
                        |_, _| Some(target),
                    );
                    let state =
                        if dragging {
                            process_host_frame_input_with_target_resolver(
                                HostFrameRequest::new(viewport, pressed.state)
                                    .raw_event(raw_pointer(PointerEventKind::Move, 24.0, 4.0, 2)),
                                |_, _| Some(target),
                            )
                            .state
                        } else {
                            pressed.state
                        };
                    let capture = state.drag_capture;
                    // Include an unmatched release as well as a full chord.
                    let mut request = HostFrameRequest::new(viewport, state);
                    for kind in [
                        PointerEventKind::Up(other),
                        PointerEventKind::Down(other),
                        PointerEventKind::Up(other),
                    ] {
                        request.raw_input.push(raw_pointer(kind, 24.0, 4.0, 3));
                    }
                    let mut seen = 0;
                    let chord = process_host_frame_input_with_filter(
                        request,
                        16.0,
                        |_, _| Some(neighbor),
                        |_, _| {
                            seen += 1;
                            !intercepted
                        },
                    );
                    assert_eq!(seen, 3, "raw hooks must still receive button chords");
                    assert!(
                        chord.gestures().next().is_none(),
                        "{owner:?}, dragging={dragging}, intercepted={intercepted}: {:?}",
                        chord.gestures().cloned().collect::<Vec<_>>()
                    );
                    assert!(chord.ui_events().next().is_none());
                    assert_eq!(chord.state.drag_capture, capture);
                    assert_eq!(chord.state.pressed, Some(target));
                    let released = process_host_frame_input_with_target_resolver(
                        HostFrameRequest::new(viewport, chord.state).raw_event(raw_pointer(
                            PointerEventKind::Up(owner),
                            if dragging { 24.0 } else { 4.0 },
                            4.0,
                            4,
                        )),
                        |_, _| Some(target),
                    );
                    assert!(
                        matches!(released.gestures().cloned().collect::<Vec<_>>().as_slice(), [GestureEvent::Drag(drag)]
                            if dragging && drag.target == target && drag.button == owner && drag.phase == GesturePhase::Commit)
                            || matches!(released.gestures().cloned().collect::<Vec<_>>().as_slice(), [GestureEvent::Click(click)]
                                if !dragging && click.target == target && click.button == owner)
                    );
                    assert!(released.state.drag_capture.is_none());
                    assert!(released.state.pressed.is_none());
                }
            }
        }
    }

    #[test]
    fn other_pointers_cannot_steal_or_cancel_the_document_press_owner() {
        for intercepted in [false, true] {
            let viewport = UiSize::new(200.0, 120.0);
            let target = UiNodeId(3);
            let neighbor = UiNodeId(4);
            let pressed = process_host_frame_input_with_target_resolver(
                HostFrameRequest::new(viewport, HostInteractionState::default())
                    .raw_event(raw_pointer(
                        PointerEventKind::Down(PointerButton::Primary),
                        4.0,
                        4.0,
                        1,
                    ))
                    .raw_event(raw_pointer(PointerEventKind::Move, 24.0, 4.0, 2)),
                |_, _| Some(target),
            );
            let capture = pressed.state.drag_capture;
            let mut request = HostFrameRequest::new(viewport, pressed.state);
            for kind in [
                PointerEventKind::Up(PointerButton::Primary),
                PointerEventKind::Cancel,
                PointerEventKind::Down(PointerButton::Primary),
                PointerEventKind::Move,
                PointerEventKind::Up(PointerButton::Primary),
            ] {
                request.raw_input.push(RawInputEvent::Pointer(
                    RawPointerEvent::new(kind, UiPoint::new(140.0, 40.0), 3)
                        .pointer_id(PointerId::new(2))
                        .pointer_kind(crate::input::PointerKind::Touch),
                ));
            }
            let mut seen = 0;
            let other = process_host_frame_input_with_filter(
                request,
                16.0,
                |_, _| Some(neighbor),
                |_, _| {
                    seen += 1;
                    !intercepted
                },
            );
            assert_eq!(
                seen, 5,
                "custom raw hooks must still receive other pointers"
            );
            assert!(
                other.ui_events().next().is_none(),
                "intercepted={intercepted}: {:?}",
                other.ui_events().cloned().collect::<Vec<_>>()
            );
            assert!(other.gestures().next().is_none());
            assert_eq!(other.state.drag_capture, capture);
            assert_eq!(other.state.pressed, Some(target));
            let released = process_host_frame_input_with_target_resolver(
                HostFrameRequest::new(viewport, other.state).raw_event(raw_pointer(
                    PointerEventKind::Up(PointerButton::Primary),
                    24.0,
                    4.0,
                    4,
                )),
                |_, _| Some(neighbor),
            );
            assert!(
                matches!(released.gestures().cloned().collect::<Vec<_>>().as_slice(), [GestureEvent::Drag(drag)]
                    if drag.target == target && drag.phase == GesturePhase::Commit)
            );
            assert!(released.state.drag_capture.is_none());
        }
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
        let [GestureEvent::Drag(begin)] = &dragging.gestures().cloned().collect::<Vec<_>>()[..]
        else {
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
        let [GestureEvent::Drag(commit)] = &committed.gestures().cloned().collect::<Vec<_>>()[..]
        else {
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
        let [GestureEvent::Drag(cancel)] = &cancelled.gestures().cloned().collect::<Vec<_>>()[..]
        else {
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

        assert_eq!(output.ui_events().count(), 1);
        assert!(matches!(
            &output.gestures().cloned().collect::<Vec<_>>()[..],
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
        let expected_ui_events = vec![
            UiInputEvent::PointerDown(UiPoint::new(6.0, 8.0)),
            UiInputEvent::Wheel(
                crate::UiWheelEvent::pixels(UiPoint::new(12.0, 10.0), UiPoint::new(0.0, -40.0))
                    .unit(crate::input::WheelDeltaUnit::Line)
                    .phase(WheelPhase::Started),
            ),
            UiInputEvent::Key {
                key: KeyCode::Character('A'),
                modifiers: KeyModifiers::NONE,
            },
            UiInputEvent::TextInput("a".to_string()),
            UiInputEvent::Focus(crate::FocusDirection::Next),
        ];
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

        assert_eq!(
            output.ui_events().cloned().collect::<Vec<_>>(),
            expected_ui_events
        );
        assert!(matches!(
            output.gestures().next(),
            Some(GestureEvent::Press {
                target: Some(actual),
                ..
            }) if *actual == target
        ));
    }

    #[test]
    fn host_frame_deduplicates_enter_key_and_generated_newline_text() {
        let viewport = UiSize::new(320.0, 180.0);
        let output = process_host_frame_input(
            HostFrameRequest::new(viewport, HostInteractionState::default()).raw_event(
                RawInputEvent::Keyboard(
                    RawKeyboardEvent::press(KeyCode::Enter, KeyModifiers::NONE, 10).with_text("\r"),
                ),
            ),
        );

        assert_eq!(
            output.ui_events().cloned().collect::<Vec<_>>(),
            vec![UiInputEvent::Key {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            }]
        );

        let text_only = process_host_frame_input(
            HostFrameRequest::new(viewport, HostInteractionState::default())
                .raw_event(RawInputEvent::Text(RawTextInputEvent::new("\n", 11))),
        );
        assert_eq!(
            text_only.ui_events().cloned().collect::<Vec<_>>(),
            vec![UiInputEvent::TextInput("\n".to_string())]
        );
    }

    #[test]
    fn enter_does_not_consume_independent_newline_text() {
        let output = process_host_frame_input(
            HostFrameRequest::new(UiSize::new(320.0, 180.0), HostInteractionState::default())
                .raw_event(RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    KeyCode::Enter,
                    KeyModifiers::NONE,
                    10,
                )))
                .raw_event(RawInputEvent::Text(RawTextInputEvent::new("\n", 11))),
        );
        assert!(
            matches!(output.ui_events().cloned().collect::<Vec<_>>().as_slice(),
            [UiInputEvent::Key { key: KeyCode::Enter, .. }, UiInputEvent::TextInput(text)] if text == "\n")
        );
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
            .events
            .push(UiInputEvent::PointerDown(UiPoint::new(4.0, 4.0)).into());
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

        assert_eq!(frame.input_results().next().unwrap().focused, Some(button));
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
        let announcement = &frame.announcements.pending[0].message;
        assert!(announcement.contains("Status"));
        assert!(announcement.contains("Running"));
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
    }

    #[test]
    fn document_frame_carries_app_image_resource_updates_to_renderer_request() {
        let viewport = UiSize::new(80.0, 80.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(80.0, 80.0));
        let update = crate::renderer::ResourceUpdate::rgba8_image(
            crate::platform::ImageHandle::app("user.avatar"),
            crate::platform::PixelSize::new(1, 1),
            vec![10, 20, 30, 255],
        );
        let pixel_buffer = update.bytes.as_ptr();
        document.add_resource_update(update.clone());

        let frame = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                HostFrameOutput::new(HostInteractionState::default()),
            ),
        )
        .expect("document frame");

        assert_eq!(frame.render_request.resource_updates, vec![update]);
        assert_eq!(
            frame.render_request.resource_updates[0].bytes.as_ptr(),
            pixel_buffer,
            "frame preparation must share pixel storage with pending uploads"
        );
    }

    #[test]
    fn document_widget_actions_route_drag_lifecycle_to_drop_target_under_cursor() {
        let viewport = UiSize::new(260.0, 120.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(260.0, 120.0));
        let root = document.root;
        let source = document.add_child(
            root,
            UiNode::container(
                "drag.source",
                LayoutStyle::absolute_rect(UiRect::new(8.0, 12.0, 72.0, 28.0)),
            )
            .with_input(InputBehavior::BUTTON)
            .with_action("drag.source")
            .with_action_mode(crate::WidgetActionMode::Drag)
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::ListItem)
                    .label("Drag source")
                    .action(AccessibilityAction::new("drag.start", "Start drag")),
            ),
        );
        let target = document.add_child(
            root,
            UiNode::container(
                "drop.target",
                LayoutStyle::absolute_rect(UiRect::new(132.0, 24.0, 96.0, 44.0)),
            )
            .with_input(InputBehavior::BUTTON)
            .with_action("drop.target")
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Group)
                    .label("Drop target")
                    .action(AccessibilityAction::new("drop.accept", "Accept drop")),
            ),
        );

        let mut host_output = HostFrameOutput::new(HostInteractionState::default());
        host_output.events.push(
            GestureEvent::Drag(DragGesture {
                pointer_id: PointerId::MOUSE,
                target: source,
                phase: GesturePhase::Commit,
                origin: UiPoint::new(12.0, 16.0),
                current: UiPoint::new(150.0, 40.0),
                previous: UiPoint::new(88.0, 28.0),
                delta: UiPoint::new(62.0, 12.0),
                total_delta: UiPoint::new(138.0, 24.0),
                button: PointerButton::Primary,
                modifiers: KeyModifiers::NONE,
                captured: true,
                timestamp_millis: 24,
            })
            .into(),
        );

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

        let actions = collect_document_widget_actions(&frame);
        assert_eq!(actions.len(), 2, "{actions:#?}");
        assert_eq!(actions[0].target, source);
        assert_eq!(
            actions[0].binding.action_id().map(|id| id.as_str()),
            Some("drag.source")
        );
        assert_eq!(actions[1].target, target);
        assert_eq!(
            actions[1].binding.action_id().map(|id| id.as_str()),
            Some("drop.target")
        );
        assert!(matches!(
            actions[1].kind,
            WidgetActionKind::Drag(drag) if drag.phase == WidgetDragPhase::Commit
        ));
    }

    #[test]
    fn drag_drop_actions_follow_owners_and_reject_disabled_hits_and_self_drops() {
        use crate::UiPortalTarget;

        for (source_portal, drop_portal, source_owner, drop_owner) in [
            (
                UiPortalTarget::Parent,
                UiPortalTarget::Parent,
                "source",
                "drop",
            ),
            (
                UiPortalTarget::AppOverlay,
                UiPortalTarget::Parent,
                "source",
                "drop",
            ),
            (
                UiPortalTarget::Parent,
                UiPortalTarget::AppOverlay,
                "source",
                "drop",
            ),
            (
                UiPortalTarget::named("host"),
                UiPortalTarget::named("host"),
                "source",
                "drop",
            ),
            (
                UiPortalTarget::global_named("host"),
                UiPortalTarget::Parent,
                "host",
                "drop",
            ),
            (
                UiPortalTarget::Parent,
                UiPortalTarget::global_named("host"),
                "source",
                "host",
            ),
            (
                UiPortalTarget::global_named("host"),
                UiPortalTarget::global_named("host"),
                "host",
                "host",
            ),
            (
                UiPortalTarget::GlobalAppOverlay,
                UiPortalTarget::Parent,
                "none",
                "drop",
            ),
            (
                UiPortalTarget::Parent,
                UiPortalTarget::GlobalAppOverlay,
                "source",
                "none",
            ),
        ] {
            for drop_inside_source in [false, true] {
                for disabled in ["none", "source-hit", "drop-hit"] {
                    for phase in [
                        GesturePhase::Begin,
                        GesturePhase::Update,
                        GesturePhase::Commit,
                        GesturePhase::Cancel,
                    ] {
                        let viewport = UiSize::new(480.0, 240.0);
                        let mut document =
                            UiDocument::new(fixed_style(viewport.width, viewport.height));
                        let root = document.root();
                        let source = document.add_child(
                            root,
                            UiNode::container(
                                "source",
                                LayoutStyle::absolute_rect(UiRect::new(0.0, 0.0, 100.0, 60.0)),
                            )
                            .with_action("source")
                            .with_action_mode(crate::WidgetActionMode::Drag)
                            .with_accessibility(
                                AccessibilityMeta::new(AccessibilityRole::ListItem)
                                    .action(AccessibilityAction::new("drag.start", "Drag")),
                            ),
                        );
                        let drop = document.add_child(
                            if drop_inside_source { source } else { root },
                            UiNode::container(
                                "drop",
                                LayoutStyle::absolute_rect(UiRect::new(180.0, 60.0, 100.0, 60.0)),
                            )
                            .with_action("drop")
                            .with_accessibility(
                                AccessibilityMeta::new(AccessibilityRole::Group)
                                    .action(AccessibilityAction::new("drop.accept", "Drop")),
                            ),
                        );
                        let host = document.add_child(
                            root,
                            UiNode::container(
                                "host",
                                LayoutStyle::absolute_rect(UiRect::new(0.0, 0.0, 480.0, 240.0)),
                            )
                            .with_action("host")
                            .with_action_mode(crate::WidgetActionMode::Drag)
                            .with_accessibility(
                                AccessibilityMeta::new(AccessibilityRole::Group)
                                    .action(AccessibilityAction::new("drag.start", "Drag"))
                                    .action(AccessibilityAction::new("drop.accept", "Drop")),
                            ),
                        );
                        document.register_portal_host("host", host);
                        let source_hit = document.add_portal_child(
                            source,
                            source_portal.clone(),
                            UiNode::container(
                                "source-hit",
                                LayoutStyle::absolute_rect(UiRect::new(10.0, 10.0, 20.0, 20.0)),
                            )
                            .with_input(InputBehavior::BUTTON),
                        );
                        let drop_hit = document.add_portal_child(
                            drop,
                            drop_portal.clone(),
                            UiNode::container(
                                "drop-hit",
                                LayoutStyle::absolute_rect(UiRect::new(40.0, 20.0, 20.0, 20.0)),
                            )
                            .with_input(InputBehavior::BUTTON),
                        );
                        if disabled == "source-hit" {
                            document.set_node_enabled(source_hit, false);
                        }
                        if disabled == "drop-hit" {
                            document.set_node_enabled(drop_hit, false);
                        }
                        document
                            .compute_layout(viewport, &mut ApproxTextMeasurer)
                            .unwrap();
                        let rect = document.node(drop_hit).layout().rect;
                        let current = UiPoint::new(rect.x + 5.0, rect.y + 5.0);
                        assert_eq!(
                            document.hit_test(current),
                            (disabled != "drop-hit").then_some(drop_hit)
                        );
                        let mut host_output = HostFrameOutput::new(HostInteractionState::default());
                        host_output.events.push(
                            GestureEvent::Drag(DragGesture {
                                pointer_id: PointerId::MOUSE,
                                target: source_hit,
                                phase,
                                origin: UiPoint::new(15.0, 15.0),
                                current,
                                previous: current,
                                delta: UiPoint::new(0.0, 0.0),
                                total_delta: UiPoint::new(current.x - 15.0, current.y - 15.0),
                                button: PointerButton::Primary,
                                modifiers: KeyModifiers::NONE,
                                captured: true,
                                timestamp_millis: 12,
                            })
                            .into(),
                        );
                        let frame = process_document_frame(
                            &mut document,
                            &mut ApproxTextMeasurer,
                            HostDocumentFrameRequest::new(
                                viewport,
                                RenderTarget::window("main", viewport),
                                host_output,
                            ),
                        )
                        .unwrap();
                        let actions = collect_document_widget_actions(&frame);
                        let mut expected = Vec::new();
                        if disabled != "source-hit" && source_owner != "none" {
                            expected.push(if source_owner == "source" {
                                source
                            } else {
                                host
                            });
                            let self_drop = source_owner == drop_owner
                                || (drop_inside_source
                                    && source_owner == "source"
                                    && drop_owner == "drop");
                            if disabled != "drop-hit" && drop_owner != "none" && !self_drop {
                                expected.push(if drop_owner == "drop" { drop } else { host });
                            }
                        }
                        let targets: Vec<_> = actions.iter().map(|action| action.target).collect();
                        assert_eq!(targets, expected,
                            "source={source_portal:?}, drop={drop_portal:?}, nested={drop_inside_source}, disabled={disabled}, phase={phase:?}");
                        assert!(actions.iter().all(|action| matches!(&action.kind,
                            WidgetActionKind::Drag(drag) if drag.phase == WidgetDragPhase::try_from(phase).unwrap())));
                    }
                }
            }
        }
    }

    #[test]
    fn document_frame_requires_paired_clicks_to_match_the_press_and_release() {
        for release_over_pressed in [false, true] {
            for claimed in [None, Some(0), Some(1)] {
                let viewport = UiSize::new(240.0, 80.0);
                let mut document = UiDocument::new(fixed_style(240.0, 80.0));
                let controls: Vec<_> = (0..2)
                    .map(|index| {
                        document.add_child(
                            document.root(),
                            UiNode::container(
                                format!("control.{index}"),
                                LayoutStyle::absolute_rect(UiRect::new(
                                    index as f32 * 120.0,
                                    0.0,
                                    100.0,
                                    40.0,
                                )),
                            )
                            .with_input(InputBehavior::BUTTON)
                            .with_action(format!("activate.{index}")),
                        )
                    })
                    .collect();
                let release = UiPoint::new(if release_over_pressed { 4.0 } else { 124.0 }, 4.0);
                let mut host_output = HostFrameOutput::new(HostInteractionState::default());
                host_output
                    .events
                    .push(UiInputEvent::PointerDown(UiPoint::new(4.0, 4.0)).into());
                host_output.events.push(HostInputEvent::new(
                    Some(UiInputEvent::PointerUp(release)),
                    claimed.map(|index| {
                        GestureEvent::Click(PointerClick {
                            pointer_id: PointerId::MOUSE,
                            target: controls[index],
                            position: release,
                            button: PointerButton::Primary,
                            count: 1,
                            modifiers: KeyModifiers::NONE,
                            timestamp_millis: 1,
                        })
                    }),
                ));
                let frame = process_document_frame(
                    &mut document,
                    &mut ApproxTextMeasurer,
                    HostDocumentFrameRequest::new(
                        viewport,
                        RenderTarget::window("test", viewport),
                        host_output,
                    ),
                )
                .unwrap();
                let expected = (release_over_pressed && claimed == Some(0)).then_some(controls[0]);
                assert_eq!(
                    frame.input_results().last().unwrap().clicked,
                    expected,
                    "release_over_pressed={release_over_pressed}, claimed={claimed:?}"
                );
                let actions = collect_document_widget_actions(&frame);
                assert_eq!(actions.len(), usize::from(expected.is_some()));
                if let Some(target) = expected {
                    assert_eq!(actions[0].target, target);
                    assert!(matches!(actions[0].kind, WidgetActionKind::Activate(_)));
                }
                assert!(frame.host_output.state.pressed.is_none());
            }
        }
    }

    #[cfg(feature = "widgets")]
    #[test]
    fn text_pointer_selection_does_not_activate_its_edit_binding() {
        for control in [
            "button",
            "editable",
            "read_only",
            "selectable",
            "search",
            "ime",
        ] {
            for count in [1, 2] {
                let viewport = UiSize::new(240.0, 80.0);
                let mut document = UiDocument::new(fixed_style(240.0, 80.0));
                let parent = document.root();
                let text_control = control != "button";
                let target = if matches!(control, "editable" | "read_only" | "selectable") {
                    let options =
                        crate::widgets::TextInputOptions::default().with_edit_action("edit.name");
                    let options = if control == "read_only" {
                        options.read_only()
                    } else {
                        options
                    };
                    let builder = if control == "selectable" {
                        crate::widgets::selectable_text
                    } else {
                        crate::widgets::singleline_text_input
                    };
                    builder(
                        &mut document,
                        parent,
                        "name",
                        &crate::widgets::TextInputState::new("Track"),
                        options,
                    )
                } else if control == "ime" {
                    document.add_child(
                        parent,
                        UiNode::container("ime", LayoutStyle::size(180.0, 30.0))
                            .with_input(InputBehavior::BUTTON)
                            .with_text_input(crate::TextInputSnapshot::new(
                                "Track",
                                0..0,
                                UiRect::new(0.0, 0.0, 1.0, 20.0),
                            ))
                            .with_action("edit.ime"),
                    )
                } else if control == "search" {
                    // Custom text controls can route selection without owning an IME session.
                    document.add_child(
                        parent,
                        UiNode::container("search", LayoutStyle::size(180.0, 30.0))
                            .with_input(InputBehavior::BUTTON)
                            .with_accessibility(AccessibilityMeta::new(
                                AccessibilityRole::SearchBox,
                            ))
                            .with_action("edit.search"),
                    )
                } else {
                    document.add_child(
                        parent,
                        UiNode::container("button", LayoutStyle::size(180.0, 30.0))
                            .with_input(InputBehavior::BUTTON)
                            .with_action("button.activate"),
                    )
                };
                let point = UiPoint::new(12.0, 12.0);
                let mut host_output = HostFrameOutput::new(HostInteractionState::default());
                host_output
                    .events
                    .push(UiInputEvent::PointerDown(point).into());
                host_output.events.push(HostInputEvent::new(
                    Some(UiInputEvent::PointerUp(point)),
                    Some(GestureEvent::Click(PointerClick {
                        pointer_id: PointerId::MOUSE,
                        target,
                        position: point,
                        button: PointerButton::Primary,
                        count,
                        modifiers: KeyModifiers::NONE,
                        timestamp_millis: 1,
                    })),
                ));
                let frame = process_document_frame(
                    &mut document,
                    &mut ApproxTextMeasurer,
                    HostDocumentFrameRequest::new(
                        viewport,
                        RenderTarget::window("test", viewport),
                        host_output,
                    ),
                )
                .unwrap();
                let actions = collect_document_widget_actions(&frame);
                assert_eq!(
                    actions
                        .iter()
                        .filter(|action| matches!(
                            action.kind,
                            crate::WidgetActionKind::Activate(_)
                        ))
                        .count(),
                    usize::from(!text_control),
                    "control={control}, clicks={count}: {actions:?}"
                );
                assert_eq!(
                    actions
                        .iter()
                        .any(|action| matches!(action.kind, crate::WidgetActionKind::TextEdit(_))),
                    text_control
                );
                let input = frame.input_results().last().unwrap();
                assert_eq!(
                    crate::WidgetAction::activation_from_input_result_for_document(
                        &document,
                        input,
                        |id| document.node(id).action().cloned()
                    )
                    .is_some(),
                    !text_control
                );
            }
        }
    }

    #[test]
    fn document_any_button_actions_dispatch_secondary_clicks_but_not_wheel_activations() {
        let viewport = UiSize::new(220.0, 120.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(220.0, 120.0));
        let target = document.add_child(
            document.root,
            UiNode::container(
                "grid.cycle",
                LayoutStyle::absolute_rect(UiRect::new(8.0, 12.0, 96.0, 32.0)),
            )
            .with_input(InputBehavior::BUTTON)
            .with_action("grid.cycle")
            .with_action_mode(crate::WidgetActionMode::ActivateAnyButton),
        );
        let wheel = RawWheelEvent::pixels(UiPoint::new(24.0, 24.0), UiPoint::new(0.0, -32.0), 12);
        let mut host_output = HostFrameOutput::new(HostInteractionState::default());
        host_output.events.push(
            GestureEvent::Click(PointerClick {
                pointer_id: PointerId::MOUSE,
                target,
                position: UiPoint::new(24.0, 24.0),
                button: PointerButton::Secondary,
                count: 1,
                modifiers: KeyModifiers::NONE,
                timestamp_millis: 10,
            })
            .into(),
        );
        host_output.events.push(
            GestureEvent::WheelTargeted {
                target: Some(target),
                event: wheel,
            }
            .into(),
        );

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

        let actions = collect_document_widget_actions(&frame);
        assert_eq!(actions.len(), 1, "{actions:#?}");
        assert!(matches!(
            actions[0].kind,
            WidgetActionKind::Activate(activation)
                if activation.pointer_button() == Some(PointerButton::Secondary)
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
    fn document_widget_actions_publish_focus_loss_when_input_blurs() {
        let viewport = UiSize::new(220.0, 100.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(220.0, 100.0));
        let input = document.add_child(
            document.root,
            UiNode::container("search", fixed_style(120.0, 28.0))
                .with_input(InputBehavior::BUTTON)
                .with_action("search.edit")
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::TextBox)
                        .label("Search")
                        .focusable(),
                ),
        );
        document
            .compute_layout(viewport, &mut measurer)
            .expect("initial layout");

        let mut host_output = HostFrameOutput::new(HostInteractionState {
            focused: Some(input),
            ..HostInteractionState::default()
        });
        host_output
            .events
            .push(UiInputEvent::PointerDown(UiPoint::new(180.0, 70.0)).into());
        let frame = process_document_frame(
            &mut document,
            &mut measurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                host_output,
            )
            .previous_focused(Some(input)),
        )
        .expect("document frame");

        assert_eq!(frame.previous_focused, Some(input));
        assert_eq!(frame.host_output.state.focused, None);
        let actions = collect_document_widget_actions(&frame);
        assert!(
            actions.iter().any(|action| {
                action.target == input
                    && action.binding.action_id().map(|id| id.as_str()) == Some("search.edit")
                    && matches!(action.kind, WidgetActionKind::Focus(change) if !change.focused)
            }),
            "expected focus lost action for blurred text field, got {actions:#?}"
        );
    }

    #[test]
    fn document_widget_actions_do_not_reuse_button_activation_binding_for_focus() {
        let viewport = UiSize::new(220.0, 100.0);
        let mut measurer = ApproxTextMeasurer;
        let mut document = UiDocument::new(fixed_style(220.0, 100.0));
        let button = document.add_child(
            document.root,
            UiNode::container("close", fixed_style(80.0, 28.0))
                .with_input(InputBehavior::BUTTON)
                .with_action("window.close")
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Close")
                        .focusable(),
                ),
        );
        document
            .compute_layout(viewport, &mut measurer)
            .expect("initial layout");

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
            ),
        )
        .expect("document frame");

        let actions = collect_document_widget_actions(&frame);

        assert!(
            actions
                .iter()
                .all(|action| !matches!(action.kind, WidgetActionKind::Focus(_))),
            "ordinary button focus must not emit focus actions with activation bindings: {actions:#?}"
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
}
