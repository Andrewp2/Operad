//! Renderer-neutral testing helpers for Operad documents.
//!
//! These utilities are intended for consumers as well as Operad's own tests:
//! replay input without an egui harness, assert layout by stable node names,
//! inspect paint lists, diff rgba snapshots with tolerances, and track simple
//! frame timing sections.

use std::borrow::Cow;
use std::fmt;
use std::time::Duration;

use crate::accessibility::{
    AccessibilityAdapterRequest, AccessibilityAnnouncement, AccessibilityPreferences,
    AccessibilityRequestKind,
};
use crate::commands::{CommandId, CommandRegistry};
use crate::host::{
    HostCommandDispatch, HostDocumentFrameOutput, HostFrameOutput, HostInteractionState,
    HostNodeInteraction, HostShortcutRoute,
};
use crate::platform::{
    AppLifecycleResponse, ClipboardResponse, CursorResponse, DragDropResponse, FileDialogResponse,
    NotificationResponse, OpenUrlResponse, PlatformRequestIdAllocator, PlatformResponse,
    PlatformServiceError, PlatformServiceKind, PlatformServiceRequest, PlatformServiceResponse,
    RepaintResponse, ScreenshotResponse, TextImeResponse,
};
use crate::renderer::{
    CanvasRenderRegistry, CanvasRenderRequest, ImageRenderRegistry, ImageRenderRequest,
    RenderFrameRequest,
};
use crate::{
    AccessibilityLiveRegion, AccessibilityNode, AccessibilityRole, AccessibilityTree, PaintItem,
    PaintKind, PaintList, RawInputEvent, UiDocument, UiInputEvent, UiInputResult, UiNode, UiNodeId,
    UiRect, UiSize,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestFailure {
    pub message: String,
}

impl TestFailure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TestFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TestFailure {}

pub type TestResult<T = ()> = Result<T, TestFailure>;

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayInput {
    Ui(UiInputEvent),
    Raw {
        event: RawInputEvent,
        line_size: f32,
        page_size: UiSize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventReplayStep {
    pub label: String,
    pub input: ReplayInput,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventReplayStepResult {
    pub label: String,
    pub input: ReplayInput,
    pub converted: Option<UiInputEvent>,
    pub result: Option<UiInputResult>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventReplay {
    pub steps: Vec<EventReplayStep>,
}

impl EventReplay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(mut self, label: impl Into<String>, event: UiInputEvent) -> Self {
        self.steps.push(EventReplayStep {
            label: label.into(),
            input: ReplayInput::Ui(event),
        });
        self
    }

    pub fn raw(mut self, label: impl Into<String>, event: RawInputEvent) -> Self {
        self.steps.push(EventReplayStep {
            label: label.into(),
            input: ReplayInput::Raw {
                event,
                line_size: 16.0,
                page_size: UiSize::new(800.0, 600.0),
            },
        });
        self
    }

    pub fn raw_scaled(
        mut self,
        label: impl Into<String>,
        event: RawInputEvent,
        line_size: f32,
        page_size: UiSize,
    ) -> Self {
        self.steps.push(EventReplayStep {
            label: label.into(),
            input: ReplayInput::Raw {
                event,
                line_size,
                page_size,
            },
        });
        self
    }

    pub fn run(&self, document: &mut UiDocument) -> EventReplayReport {
        let mut steps = Vec::with_capacity(self.steps.len());
        for step in &self.steps {
            let converted = match &step.input {
                ReplayInput::Ui(event) => Some(event.clone()),
                ReplayInput::Raw {
                    event,
                    line_size,
                    page_size,
                } => event.to_ui_input_event_with_wheel_scale(*line_size, *page_size),
            };
            let result = converted.clone().map(|event| document.handle_input(event));
            steps.push(EventReplayStepResult {
                label: step.label.clone(),
                input: step.input.clone(),
                converted,
                result,
            });
        }
        EventReplayReport { steps }
    }

    pub fn run_with_commands(
        &self,
        document: &mut UiDocument,
        state: HostInteractionState,
        registry: &CommandRegistry,
    ) -> CommandReplayReport {
        let mut state = state;
        let mut steps = Vec::with_capacity(self.steps.len());
        for step in &self.steps {
            let converted = match &step.input {
                ReplayInput::Ui(event) => Some(event.clone()),
                ReplayInput::Raw {
                    event,
                    line_size,
                    page_size,
                } => event.to_ui_input_event_with_wheel_scale(*line_size, *page_size),
            };
            let result = converted.clone().map(|event| document.handle_input(event));
            let updates_host_state = converted
                .as_ref()
                .is_some_and(replay_input_updates_host_state);
            if let Some(result) = result.clone().filter(|_| updates_host_state) {
                state.apply_input_result(result);
            }

            let shortcut_route = converted.as_ref().and_then(|event| match event {
                UiInputEvent::Key { key, modifiers } => {
                    Some(state.route_key(*key, *modifiers, registry))
                }
                _ => None,
            });
            let dispatch = shortcut_route.as_ref().and_then(|route| {
                route.command.clone().map(|command| HostCommandDispatch {
                    command,
                    shortcut: route.shortcut,
                    target: route.target,
                })
            });

            steps.push(CommandReplayStepResult {
                label: step.label.clone(),
                input: step.input.clone(),
                converted,
                result,
                shortcut_route,
                dispatch,
            });
        }
        CommandReplayReport { steps, state }
    }
}

fn replay_input_updates_host_state(event: &UiInputEvent) -> bool {
    !matches!(event, UiInputEvent::Key { .. } | UiInputEvent::TextInput(_))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventReplayReport {
    pub steps: Vec<EventReplayStepResult>,
}

impl EventReplayReport {
    pub fn clicked_nodes(&self) -> Vec<UiNodeId> {
        self.steps
            .iter()
            .filter_map(|step| step.result.as_ref()?.clicked)
            .collect()
    }

    pub fn focused_nodes(&self) -> Vec<UiNodeId> {
        self.steps
            .iter()
            .filter_map(|step| step.result.as_ref()?.focused)
            .collect()
    }

    pub fn scrolled_nodes(&self) -> Vec<UiNodeId> {
        self.steps
            .iter()
            .filter_map(|step| step.result.as_ref()?.scrolled)
            .collect()
    }

    pub fn require_all_converted(&self) -> TestResult {
        if let Some(step) = self.steps.iter().find(|step| step.converted.is_none()) {
            Err(TestFailure::new(format!(
                "event replay step `{}` did not convert to UiInputEvent",
                step.label
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandReplayStepResult {
    pub label: String,
    pub input: ReplayInput,
    pub converted: Option<UiInputEvent>,
    pub result: Option<UiInputResult>,
    pub shortcut_route: Option<HostShortcutRoute>,
    pub dispatch: Option<HostCommandDispatch>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CommandReplayReport {
    pub steps: Vec<CommandReplayStepResult>,
    pub state: HostInteractionState,
}

impl CommandReplayReport {
    pub fn routes(&self) -> impl Iterator<Item = &HostShortcutRoute> {
        self.steps
            .iter()
            .filter_map(|step| step.shortcut_route.as_ref())
    }

    pub fn dispatches(&self) -> impl Iterator<Item = &HostCommandDispatch> {
        self.steps.iter().filter_map(|step| step.dispatch.as_ref())
    }

    pub fn dispatched_commands(&self) -> Vec<CommandId> {
        self.dispatches()
            .map(|dispatch| dispatch.command.clone())
            .collect()
    }

    pub fn require_command_dispatched(&self, command: impl Into<CommandId>) -> TestResult {
        let command = command.into();
        if self
            .dispatches()
            .any(|dispatch| dispatch.command == command)
        {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected command `{command}` to dispatch, got {:?}",
                self.dispatched_commands()
            )))
        }
    }

    pub fn require_no_commands(&self) -> TestResult {
        if let Some(dispatch) = self.dispatches().next() {
            Err(TestFailure::new(format!(
                "expected no command dispatches, got `{}`",
                dispatch.command
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutAssertions<'a> {
    document: &'a UiDocument,
}

impl<'a> LayoutAssertions<'a> {
    pub const fn new(document: &'a UiDocument) -> Self {
        Self { document }
    }

    pub fn node(&self, name: &str) -> TestResult<(UiNodeId, &'a UiNode)> {
        self.document
            .nodes()
            .iter()
            .enumerate()
            .find(|(_, node)| node.name == name)
            .map(|(index, node)| (UiNodeId(index), node))
            .ok_or_else(|| TestFailure::new(format!("missing node `{name}`")))
    }

    pub fn rect(&self, name: &str) -> TestResult<UiRect> {
        Ok(self.node(name)?.1.layout.rect)
    }

    pub fn require_visible(&self, name: &str) -> TestResult {
        let (_, node) = self.node(name)?;
        if node.layout.visible {
            Ok(())
        } else {
            Err(TestFailure::new(format!("node `{name}` is not visible")))
        }
    }

    pub fn require_min_size(&self, name: &str, min_size: UiSize) -> TestResult {
        let rect = self.rect(name)?;
        if rect.width >= min_size.width && rect.height >= min_size.height {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected at least {}x{}, got {}x{}",
                min_size.width, min_size.height, rect.width, rect.height
            )))
        }
    }

    pub fn require_contains(&self, outer: &str, inner: &str) -> TestResult {
        let outer_rect = self.rect(outer)?;
        let inner_rect = self.rect(inner)?;
        if outer_rect.contains_rect(inner_rect) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{outer}` does not contain `{inner}`"
            )))
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessibilityAssertions<'a> {
    document: &'a UiDocument,
    tree: AccessibilityTree,
}

impl<'a> AccessibilityAssertions<'a> {
    pub fn new(document: &'a UiDocument) -> Self {
        Self {
            document,
            tree: document.accessibility_snapshot(),
        }
    }

    pub fn tree(&self) -> &AccessibilityTree {
        &self.tree
    }

    pub fn node(&self, name: &str) -> TestResult<&AccessibilityNode> {
        let (id, _) = LayoutAssertions::new(self.document).node(name)?;
        self.tree
            .nodes
            .iter()
            .find(|node| node.id == id)
            .ok_or_else(|| TestFailure::new(format!("node `{name}` has no accessibility node")))
    }

    pub fn require_role(&self, name: &str, role: AccessibilityRole) -> TestResult {
        let node = self.node(name)?;
        if node.role == role {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected accessibility role {role:?}, got {:?}",
                node.role
            )))
        }
    }

    pub fn require_label(&self, name: &str, label: &str) -> TestResult {
        let node = self.node(name)?;
        if node.label.as_deref() == Some(label) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected accessibility label `{label}`, got {:?}",
                node.label
            )))
        }
    }

    pub fn require_value_contains(&self, name: &str, text: &str) -> TestResult {
        let node = self.node(name)?;
        if node
            .value
            .as_deref()
            .is_some_and(|value| value.contains(text))
        {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected accessibility value containing `{text}`, got {:?}",
                node.value
            )))
        }
    }

    pub fn require_summary_contains(&self, name: &str, text: &str) -> TestResult {
        let node = self.node(name)?;
        let screen_reader_text = node
            .summary
            .as_ref()
            .map(|summary| summary.screen_reader_text());
        if screen_reader_text
            .as_deref()
            .is_some_and(|summary| summary.contains(text))
        {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected accessibility summary containing `{text}`, got {screen_reader_text:?}"
            )))
        }
    }

    pub fn require_live_region(
        &self,
        name: &str,
        live_region: AccessibilityLiveRegion,
    ) -> TestResult {
        let node = self.node(name)?;
        if node.live_region == live_region {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected live region {live_region:?}, got {:?}",
                node.live_region
            )))
        }
    }

    pub fn require_active_descendant(&self, owner: &str, descendant: &str) -> TestResult {
        let owner_node = self.node(owner)?;
        let descendant_id = LayoutAssertions::new(self.document).node(descendant)?.0;
        if owner_node.relations.active_descendant == Some(descendant_id) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{owner}` expected active descendant `{descendant}`, got {:?}",
                owner_node.relations.active_descendant
            )))
        }
    }

    pub fn require_focus_order(&self, names: &[&str]) -> TestResult {
        let expected = names
            .iter()
            .map(|name| {
                LayoutAssertions::new(self.document)
                    .node(name)
                    .map(|(id, _)| id)
            })
            .collect::<TestResult<Vec<_>>>()?;
        if self.tree.focus_order == expected {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected accessibility focus order {expected:?}, got {:?}",
                self.tree.focus_order
            )))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AccessibilityRequestAssertions<'a> {
    requests: &'a [AccessibilityAdapterRequest],
}

impl<'a> AccessibilityRequestAssertions<'a> {
    pub const fn new(requests: &'a [AccessibilityAdapterRequest]) -> Self {
        Self { requests }
    }

    pub fn from_document_frame(output: &'a HostDocumentFrameOutput) -> Self {
        Self::new(&output.accessibility_requests)
    }

    pub const fn requests(&self) -> &'a [AccessibilityAdapterRequest] {
        self.requests
    }

    pub fn request_count(&self, kind: AccessibilityRequestKind) -> usize {
        self.requests
            .iter()
            .filter(|request| request.kind() == kind)
            .count()
    }

    pub fn require_request_kind(
        &self,
        kind: AccessibilityRequestKind,
    ) -> TestResult<&'a AccessibilityAdapterRequest> {
        self.requests
            .iter()
            .find(|request| request.kind() == kind)
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing accessibility request kind {kind:?}; available requests: {:?}",
                    self.request_kinds()
                ))
            })
    }

    pub fn require_publish_tree(
        &self,
    ) -> TestResult<(
        &'a AccessibilityTree,
        Option<UiNodeId>,
        AccessibilityPreferences,
    )> {
        self.requests
            .iter()
            .find_map(|request| {
                if let AccessibilityAdapterRequest::PublishTree {
                    tree,
                    focused,
                    preferences,
                } = request
                {
                    Some((tree, *focused, *preferences))
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing accessibility PublishTree request; available requests: {:?}",
                    self.request_kinds()
                ))
            })
    }

    pub fn require_apply_preferences(
        &self,
        preferences: AccessibilityPreferences,
    ) -> TestResult<&'a AccessibilityAdapterRequest> {
        self.requests
            .iter()
            .find(|request| {
                matches!(
                    request,
                    AccessibilityAdapterRequest::ApplyPreferences(actual)
                        if *actual == preferences
                )
            })
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing accessibility ApplyPreferences request for {preferences:?}; available requests: {:?}",
                    self.request_kinds()
                ))
            })
    }

    pub fn require_announcement_contains(
        &self,
        text: &str,
    ) -> TestResult<&'a AccessibilityAnnouncement> {
        self.requests
            .iter()
            .find_map(|request| {
                if let AccessibilityAdapterRequest::Announce(announcement) = request {
                    announcement.message.contains(text).then_some(announcement)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing accessibility announcement containing `{text}`; available requests: {:?}",
                    self.request_kinds()
                ))
            })
    }

    fn request_kinds(&self) -> Vec<AccessibilityRequestKind> {
        self.requests.iter().map(|request| request.kind()).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaintKindSelector {
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

impl PaintKindSelector {
    pub const fn matches(self, kind: &PaintKind) -> bool {
        matches!(
            (self, kind),
            (Self::Rect, PaintKind::Rect { .. })
                | (Self::Text, PaintKind::Text(_))
                | (Self::Canvas, PaintKind::Canvas(_))
                | (Self::Line, PaintKind::Line { .. })
                | (Self::Circle, PaintKind::Circle { .. })
                | (Self::Polygon, PaintKind::Polygon { .. })
                | (Self::Image, PaintKind::Image { .. })
                | (Self::RichRect, PaintKind::RichRect(_))
                | (Self::SceneText, PaintKind::SceneText(_))
                | (Self::Path, PaintKind::Path(_))
                | (Self::ImagePlacement, PaintKind::ImagePlacement(_))
        )
    }
}

#[derive(Debug, Clone)]
pub struct PaintAssertions<'a> {
    document: &'a UiDocument,
    paint: PaintList,
}

impl<'a> PaintAssertions<'a> {
    pub fn new(document: &'a UiDocument) -> Self {
        Self {
            document,
            paint: document.paint_list(),
        }
    }

    pub fn paint(&self) -> &PaintList {
        &self.paint
    }

    pub fn count_kind(&self, selector: PaintKindSelector) -> usize {
        self.paint
            .items
            .iter()
            .filter(|item| selector.matches(&item.kind))
            .count()
    }

    pub fn require_node_kind(
        &self,
        node_name: &str,
        selector: PaintKindSelector,
    ) -> TestResult<&PaintItem> {
        let (id, _) = LayoutAssertions::new(self.document).node(node_name)?;
        self.paint
            .items
            .iter()
            .find(|item| item.node == id && selector.matches(&item.kind))
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "node `{node_name}` has no paint item of kind {selector:?}"
                ))
            })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderAssertions<'a> {
    request: &'a RenderFrameRequest,
}

impl<'a> RenderAssertions<'a> {
    pub const fn new(request: &'a RenderFrameRequest) -> Self {
        Self { request }
    }

    pub const fn request(&self) -> &'a RenderFrameRequest {
        self.request
    }

    pub fn canvas_requests(&self) -> Vec<CanvasRenderRequest> {
        self.request.canvas_requests()
    }

    pub fn image_requests(&self) -> Vec<ImageRenderRequest> {
        self.request.image_requests()
    }

    pub fn require_canvas(&self, key: &str) -> TestResult<CanvasRenderRequest> {
        self.canvas_requests()
            .into_iter()
            .find(|request| request.canvas.key == key)
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing canvas render request `{key}`; available canvases: {:?}",
                    self.canvas_keys()
                ))
            })
    }

    pub fn require_image(&self, key: &str) -> TestResult<ImageRenderRequest> {
        self.image_requests()
            .into_iter()
            .find(|request| request.key() == key)
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing image render request `{key}`; available images: {:?}",
                    self.image_keys()
                ))
            })
    }

    pub fn require_canvas_host_capture(&self, key: &str) -> TestResult {
        let request = self.require_canvas(key)?;
        if request.requires_host_input_capture() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "canvas `{key}` does not require host input capture"
            )))
        }
    }

    pub fn require_canvas_dirty(&self, key: &str) -> TestResult {
        let request = self.require_canvas(key)?;
        if self.request.dirty_regions.is_empty() || self.request.dirty_regions.covers(request.rect)
        {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "canvas `{key}` rect {:?} is not covered by dirty regions {:?}",
                request.rect, self.request.dirty_regions.regions
            )))
        }
    }

    pub fn require_node_interaction(
        &self,
        node: UiNodeId,
        expected: HostNodeInteraction,
    ) -> TestResult {
        let actual = self.request.interaction_for(node);
        if actual == expected {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node {node:?} expected render interaction {expected:?}, got {actual:?}"
            )))
        }
    }

    pub fn missing_canvas_handlers<B>(&self, registry: &CanvasRenderRegistry<B>) -> Vec<String> {
        self.canvas_requests()
            .into_iter()
            .filter(|request| !registry.contains(&request.canvas.key))
            .map(|request| request.canvas.key)
            .collect()
    }

    pub fn missing_image_handlers<B>(&self, registry: &ImageRenderRegistry<B>) -> Vec<String> {
        self.image_requests()
            .into_iter()
            .filter(|request| !registry.contains(request.key()))
            .map(|request| request.image.key)
            .collect()
    }

    pub fn require_all_canvas_handlers<B>(&self, registry: &CanvasRenderRegistry<B>) -> TestResult {
        let missing = self.missing_canvas_handlers(registry);
        if missing.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "missing canvas render handlers for {missing:?}"
            )))
        }
    }

    pub fn require_all_image_handlers<B>(&self, registry: &ImageRenderRegistry<B>) -> TestResult {
        let missing = self.missing_image_handlers(registry);
        if missing.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "missing image render handlers for {missing:?}"
            )))
        }
    }

    fn canvas_keys(&self) -> Vec<String> {
        self.canvas_requests()
            .into_iter()
            .map(|request| request.canvas.key)
            .collect()
    }

    fn image_keys(&self) -> Vec<String> {
        self.image_requests()
            .into_iter()
            .map(|request| request.image.key)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct PlatformAssertions<'a> {
    requests: Cow<'a, [PlatformServiceRequest]>,
    responses: Cow<'a, [PlatformServiceResponse]>,
}

impl<'a> PlatformAssertions<'a> {
    pub fn new(
        requests: &'a [PlatformServiceRequest],
        responses: &'a [PlatformServiceResponse],
    ) -> Self {
        Self {
            requests: Cow::Borrowed(requests),
            responses: Cow::Borrowed(responses),
        }
    }

    pub fn from_host_frame(output: &'a HostFrameOutput) -> Self {
        Self::new(&output.platform_requests, &output.platform_responses)
    }

    pub fn from_document_frame(
        output: &HostDocumentFrameOutput,
        allocator: &mut PlatformRequestIdAllocator,
    ) -> Self {
        Self {
            requests: Cow::Owned(output.platform_service_requests(allocator)),
            responses: Cow::Owned(output.host_output.platform_responses.clone()),
        }
    }

    pub fn requests(&self) -> &[PlatformServiceRequest] {
        self.requests.as_ref()
    }

    pub fn responses(&self) -> &[PlatformServiceResponse] {
        self.responses.as_ref()
    }

    pub fn request_count(&self, kind: PlatformServiceKind) -> usize {
        self.requests
            .iter()
            .filter(|request| request.kind() == kind)
            .count()
    }

    pub fn response_count(&self, kind: PlatformServiceKind) -> usize {
        self.responses
            .iter()
            .filter(|response| response.kind() == kind)
            .count()
    }

    pub fn require_request_kind(
        &self,
        kind: PlatformServiceKind,
    ) -> TestResult<&PlatformServiceRequest> {
        self.requests
            .iter()
            .find(|request| request.kind() == kind)
            .ok_or_else(|| TestFailure::new(format!("missing platform request kind {kind:?}")))
    }

    pub fn require_response_kind(
        &self,
        kind: PlatformServiceKind,
    ) -> TestResult<&PlatformServiceResponse> {
        self.responses
            .iter()
            .find(|response| response.kind() == kind)
            .ok_or_else(|| TestFailure::new(format!("missing platform response kind {kind:?}")))
    }

    pub fn require_response_for(
        &self,
        request: &PlatformServiceRequest,
    ) -> TestResult<&PlatformServiceResponse> {
        self.responses
            .iter()
            .find(|response| response.is_for(request) && response.kind() == request.kind())
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing {:?} response for platform request id {}",
                    request.kind(),
                    request.id.0
                ))
            })
    }

    pub fn require_unsupported_response_for(
        &self,
        request: &PlatformServiceRequest,
    ) -> TestResult<&PlatformServiceResponse> {
        let response = self.require_response_for(request)?;
        let expected = PlatformResponse::unsupported(request.kind());
        if response.response == expected {
            Ok(response)
        } else {
            Err(TestFailure::new(format!(
                "platform response id {} kind {:?} expected unsupported response, got {:?}",
                response.id.0,
                response.kind(),
                response.response
            )))
        }
    }

    pub fn require_all_responses_match_requests(&self) -> TestResult {
        for response in self.responses.iter() {
            if !self
                .requests
                .iter()
                .any(|request| response.is_for(request) && response.kind() == request.kind())
            {
                return Err(TestFailure::new(format!(
                    "platform response id {} kind {:?} has no matching request",
                    response.id.0,
                    response.kind()
                )));
            }
        }
        Ok(())
    }

    pub fn require_no_unsupported_responses(&self) -> TestResult {
        if let Some(response) = self
            .responses
            .iter()
            .find(|response| platform_response_is_unsupported(&response.response))
        {
            Err(TestFailure::new(format!(
                "platform response id {} kind {:?} was unsupported",
                response.id.0,
                response.kind()
            )))
        } else {
            Ok(())
        }
    }

    pub fn require_no_error_responses(&self) -> TestResult {
        if let Some((response, error)) = self.responses.iter().find_map(|response| {
            platform_response_error(&response.response).map(|error| (response, error))
        }) {
            Err(TestFailure::new(format!(
                "platform response id {} kind {:?} returned {:?}: {}",
                response.id.0,
                response.kind(),
                error.code,
                error.message
            )))
        } else {
            Ok(())
        }
    }
}

fn platform_response_is_unsupported(response: &PlatformResponse) -> bool {
    response == &PlatformResponse::unsupported(response.kind())
}

fn platform_response_error(response: &PlatformResponse) -> Option<&PlatformServiceError> {
    match response {
        PlatformResponse::Clipboard(ClipboardResponse::Error(error))
        | PlatformResponse::FileDialog(FileDialogResponse::Error(error))
        | PlatformResponse::OpenUrl(OpenUrlResponse::Error(error))
        | PlatformResponse::Notification(NotificationResponse::Error(error))
        | PlatformResponse::Screenshot(ScreenshotResponse::Error(error))
        | PlatformResponse::AppLifecycle(AppLifecycleResponse::Error(error))
        | PlatformResponse::TextIme(TextImeResponse::Error(error))
        | PlatformResponse::DragDrop(DragDropResponse::Error(error))
        | PlatformResponse::Cursor(CursorResponse::Error(error))
        | PlatformResponse::Repaint(RepaintResponse::Error(error)) => Some(error),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbaImageView<'a> {
    pub width: usize,
    pub height: usize,
    pub pixels: &'a [u8],
}

impl<'a> RgbaImageView<'a> {
    pub fn new(width: usize, height: usize, pixels: &'a [u8]) -> TestResult<Self> {
        let expected_len = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| TestFailure::new("rgba image dimensions overflow"))?;
        if pixels.len() != expected_len {
            return Err(TestFailure::new(format!(
                "rgba image expected {expected_len} bytes, got {}",
                pixels.len()
            )));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelDiffTolerance {
    pub max_changed_pixels: usize,
    pub max_channel_delta: u8,
    pub max_total_channel_delta: u64,
}

impl PixelDiffTolerance {
    pub const EXACT: Self = Self {
        max_changed_pixels: 0,
        max_channel_delta: 0,
        max_total_channel_delta: 0,
    };
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PixelDiffReport {
    pub width: usize,
    pub height: usize,
    pub changed_pixels: usize,
    pub max_channel_delta: u8,
    pub total_channel_delta: u64,
}

impl PixelDiffReport {
    pub const fn is_within(self, tolerance: PixelDiffTolerance) -> bool {
        self.changed_pixels <= tolerance.max_changed_pixels
            && self.max_channel_delta <= tolerance.max_channel_delta
            && self.total_channel_delta <= tolerance.max_total_channel_delta
    }
}

pub fn diff_rgba8(
    expected: RgbaImageView<'_>,
    actual: RgbaImageView<'_>,
) -> TestResult<PixelDiffReport> {
    if expected.width != actual.width || expected.height != actual.height {
        return Err(TestFailure::new(format!(
            "rgba image dimensions differ: expected {}x{}, got {}x{}",
            expected.width, expected.height, actual.width, actual.height
        )));
    }

    let mut report = PixelDiffReport {
        width: expected.width,
        height: expected.height,
        ..Default::default()
    };

    for (expected, actual) in expected
        .pixels
        .chunks_exact(4)
        .zip(actual.pixels.chunks_exact(4))
    {
        let mut pixel_changed = false;
        for channel in 0..4 {
            let delta = expected[channel].abs_diff(actual[channel]);
            if delta > 0 {
                pixel_changed = true;
                report.max_channel_delta = report.max_channel_delta.max(delta);
                report.total_channel_delta += u64::from(delta);
            }
        }
        if pixel_changed {
            report.changed_pixels += 1;
        }
    }

    Ok(report)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirtyFlags {
    pub layout: bool,
    pub paint: bool,
    pub input: bool,
    pub theme: bool,
    pub text_measurement: bool,
}

impl DirtyFlags {
    pub const NONE: Self = Self {
        layout: false,
        paint: false,
        input: false,
        theme: false,
        text_measurement: false,
    };

    pub const ALL: Self = Self {
        layout: true,
        paint: true,
        input: true,
        theme: true,
        text_measurement: true,
    };

    pub const fn any(self) -> bool {
        self.layout || self.paint || self.input || self.theme || self.text_measurement
    }

    pub const fn union(self, other: Self) -> Self {
        Self {
            layout: self.layout || other.layout,
            paint: self.paint || other.paint,
            input: self.input || other.input,
            theme: self.theme || other.theme,
            text_measurement: self.text_measurement || other.text_measurement,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::NONE;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameTimingSection {
    pub name: String,
    pub duration: Duration,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameTiming {
    pub sections: Vec<FrameTimingSection>,
}

impl FrameTiming {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn section(mut self, name: impl Into<String>, duration: Duration) -> Self {
        self.sections.push(FrameTimingSection {
            name: name.into(),
            duration,
        });
        self
    }

    pub fn total(&self) -> Duration {
        self.sections.iter().map(|section| section.duration).sum()
    }

    pub fn duration(&self, name: &str) -> Option<Duration> {
        self.sections
            .iter()
            .find(|section| section.name == name)
            .map(|section| section.duration)
    }

    pub fn within_budget(&self, budget: Duration) -> bool {
        self.total() <= budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{
        Command, CommandId, CommandMeta, CommandRegistry, CommandScope, Shortcut,
    };
    use crate::platform::{
        ClipboardRequest, ClipboardResponse, CursorRequest, LogicalRect, PlatformErrorCode,
        PlatformRequest, PlatformRequestId, PlatformRequestIdAllocator, PlatformResponse,
        PlatformServiceError, PlatformServiceKind, RepaintRequest, RepaintResponse,
    };
    use crate::{
        length, process_document_frame, root_style, AccessibilityLiveRegion, AccessibilityMeta,
        AccessibilityRole, AccessibilitySummary, ApproxTextMeasurer, CanvasContent,
        CanvasInteractionPolicy, CanvasRenderContext, CanvasRenderOutput, CanvasRenderRegistry,
        ClipBehavior, ColorRgba, DirtyRegionSet, HostDocumentFrameRequest, HostFrameOutput,
        HostInteractionState, ImageContent, ImageRenderContext, ImageRenderOutput,
        ImageRenderRegistry, InputBehavior, RawKeyboardEvent, RawPointerEvent, RawWheelEvent,
        RenderFrameRequest, RenderTarget, StrokeStyle, TextStyle, UiContent, UiDocument, UiNode,
        UiNodeStyle, UiPoint, UiVisual,
    };
    use taffy::prelude::{Dimension, Size as TaffySize, Style};

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

    #[test]
    fn event_replay_runs_raw_and_document_events() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        let root = document.root;
        let button = document.add_child(
            root,
            UiNode::container("play", fixed_style(80.0, 32.0)).with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let report = EventReplay::new()
            .raw(
                "move",
                RawInputEvent::Pointer(RawPointerEvent::new(
                    crate::PointerEventKind::Move,
                    UiPoint::new(12.0, 12.0),
                    1,
                )),
            )
            .ui("down", UiInputEvent::PointerDown(UiPoint::new(12.0, 12.0)))
            .ui("up", UiInputEvent::PointerUp(UiPoint::new(12.0, 12.0)))
            .raw(
                "key",
                RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    crate::KeyCode::Enter,
                    crate::KeyModifiers::NONE,
                    4,
                )),
            )
            .run(&mut document);

        assert_eq!(report.steps.len(), 4);
        assert_eq!(report.clicked_nodes(), vec![button]);
        assert_eq!(report.focused_nodes().last().copied(), Some(button));
        assert!(report.require_all_converted().is_ok());
    }

    #[test]
    fn event_replay_routes_raw_keyboard_shortcuts_to_commands() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        let button = document.add_child(
            document.root,
            UiNode::container("play", fixed_style(80.0, 32.0)).with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let mut registry = CommandRegistry::new();
        registry
            .register(Command::new(CommandMeta::new("global.save", "Save")))
            .unwrap();
        registry
            .register(Command::new(CommandMeta::new(
                "editor.save",
                "Save Selection",
            )))
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('s'), "global.save")
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Editor, Shortcut::ctrl('s'), "editor.save")
            .unwrap();

        let report = EventReplay::new()
            .raw(
                "save",
                RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    crate::KeyCode::Character('S'),
                    crate::KeyModifiers {
                        ctrl: true,
                        ..crate::KeyModifiers::NONE
                    },
                    1,
                )),
            )
            .run_with_commands(
                &mut document,
                HostInteractionState {
                    focused: Some(button),
                    active_shortcut_scopes: vec![CommandScope::Workspace, CommandScope::Editor],
                    ..HostInteractionState::default()
                },
                &registry,
            );

        assert_eq!(
            report.dispatched_commands(),
            vec![CommandId::new("editor.save")]
        );
        assert_eq!(
            report.steps[0].dispatch.as_ref().unwrap().target,
            Some(button)
        );
        assert_eq!(
            report.steps[0]
                .shortcut_route
                .as_ref()
                .unwrap()
                .active_scopes,
            vec![CommandScope::Workspace, CommandScope::Editor]
        );
        report
            .require_command_dispatched("editor.save")
            .expect("editor command dispatch");
    }

    #[test]
    fn command_replay_updates_state_from_document_input_before_routing() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        let button = document.add_child(
            document.root,
            UiNode::container("play", fixed_style(80.0, 32.0)).with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let mut registry = CommandRegistry::new();
        registry
            .register(Command::new(CommandMeta::new("transport.play", "Play")))
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('p'), "transport.play")
            .unwrap();

        let report = EventReplay::new()
            .ui("focus", UiInputEvent::PointerDown(UiPoint::new(12.0, 12.0)))
            .raw(
                "play",
                RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    crate::KeyCode::Character('P'),
                    crate::KeyModifiers {
                        ctrl: true,
                        ..crate::KeyModifiers::NONE
                    },
                    2,
                )),
            )
            .run_with_commands(
                &mut document,
                HostInteractionState::default().with_active_shortcut_scope(CommandScope::Global),
                &registry,
            );

        assert_eq!(report.state.focused, Some(button));
        assert_eq!(
            report.steps[1].dispatch.as_ref().unwrap().target,
            Some(button)
        );
        assert_eq!(
            report.dispatched_commands(),
            vec![CommandId::new("transport.play")]
        );
    }

    #[test]
    fn command_replay_asserts_missing_and_unrouted_commands() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let mut registry = CommandRegistry::new();
        registry
            .register(Command::new(CommandMeta::new("edit.cut", "Cut")).disabled("read only"))
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('x'), "edit.cut")
            .unwrap();

        let report = EventReplay::new()
            .raw(
                "cut",
                RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    crate::KeyCode::Character('X'),
                    crate::KeyModifiers {
                        ctrl: true,
                        ..crate::KeyModifiers::NONE
                    },
                    1,
                )),
            )
            .run_with_commands(
                &mut document,
                HostInteractionState::default().with_active_shortcut_scope(CommandScope::Global),
                &registry,
            );

        assert!(report.steps[0].shortcut_route.is_some());
        assert!(report.steps[0].dispatch.is_none());
        assert_eq!(report.dispatched_commands(), Vec::<CommandId>::new());
        report.require_no_commands().expect("no commands");
        assert!(report.require_command_dispatched("edit.cut").is_err());
    }

    #[test]
    fn layout_and_paint_assertions_use_stable_node_names() {
        let mut document = UiDocument::new(root_style(220.0, 120.0));
        let root = document.root;
        let panel = document.add_child(
            root,
            UiNode::container(
                "panel",
                UiNodeStyle {
                    clip: ClipBehavior::Clip,
                    ..fixed_style(140.0, 80.0)
                },
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(20, 24, 32, 255),
                Some(StrokeStyle::new(ColorRgba::new(80, 100, 120, 255), 1.0)),
                4.0,
            )),
        );
        document.add_child(
            panel,
            UiNode::image(
                "panel.icon",
                ImageContent::new("icons.play"),
                fixed_style(24.0, 24.0).layout,
            ),
        );
        document.add_child(
            panel,
            UiNode::text(
                "panel.label",
                "Play",
                TextStyle::default(),
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            ),
        );
        document
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let layout = LayoutAssertions::new(&document);
        layout.require_visible("panel").expect("panel visible");
        layout
            .require_min_size("panel", UiSize::new(120.0, 60.0))
            .expect("panel minimum");
        layout
            .require_contains("panel", "panel.icon")
            .expect("panel contains icon");

        let paint = PaintAssertions::new(&document);
        assert!(paint.count_kind(PaintKindSelector::Rect) >= 1);
        paint
            .require_node_kind("panel.icon", PaintKindSelector::Image)
            .expect("icon paint");
        paint
            .require_node_kind("panel.label", PaintKindSelector::Text)
            .expect("text paint");
    }

    #[test]
    fn render_assertions_check_canvas_image_handlers_and_interaction() {
        let mut document = UiDocument::new(root_style(240.0, 120.0));
        let root = document.root;
        let mut canvas = UiNode::canvas(
            "editor.viewport",
            "editor.viewport",
            fixed_style(120.0, 80.0).layout,
        );
        canvas.content = UiContent::Canvas(
            CanvasContent::new("editor.viewport")
                .pointer_capture(true)
                .keyboard_capture(true)
                .domain_hit_testing(true),
        );
        let canvas_node = document.add_child(root, canvas);
        document.add_child(
            root,
            UiNode::image(
                "editor.thumbnail",
                ImageContent::new("images.thumbnail"),
                fixed_style(48.0, 48.0).layout,
            ),
        );
        document
            .compute_layout(UiSize::new(240.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let mut dirty_regions = DirtyRegionSet::empty();
        assert!(dirty_regions.push(document.node(canvas_node).layout.rect));
        let interaction = HostNodeInteraction {
            focused: true,
            drag_captured: true,
            ..HostNodeInteraction::default()
        };
        let request = RenderFrameRequest::new(
            RenderTarget::window("main", UiSize::new(240.0, 120.0)),
            UiSize::new(240.0, 120.0),
            document.paint_list(),
        )
        .dirty_regions(dirty_regions)
        .node_interaction(canvas_node, interaction);
        let assertions = RenderAssertions::new(&request);

        let canvas_request = assertions
            .require_canvas("editor.viewport")
            .expect("canvas request");
        assert!(canvas_request.canvas.interaction.domain_hit_testing);
        assertions
            .require_canvas_host_capture("editor.viewport")
            .expect("host capture");
        assertions
            .require_canvas_dirty("editor.viewport")
            .expect("dirty canvas");
        assertions
            .require_image("images.thumbnail")
            .expect("image request");
        assertions
            .require_node_interaction(canvas_node, interaction)
            .expect("node interaction");

        let mut canvas_registry: CanvasRenderRegistry<()> = CanvasRenderRegistry::new();
        canvas_registry.register(
            "editor.viewport",
            |_context: CanvasRenderContext<'_, ()>| Ok(CanvasRenderOutput::new()),
        );
        let mut image_registry: ImageRenderRegistry<()> = ImageRenderRegistry::new();
        image_registry.register(
            "images.thumbnail",
            |_context: ImageRenderContext<'_, ()>| Ok(ImageRenderOutput::new()),
        );
        assertions
            .require_all_canvas_handlers(&canvas_registry)
            .expect("canvas handlers");
        assertions
            .require_all_image_handlers(&image_registry)
            .expect("image handlers");

        let empty_canvas_registry: CanvasRenderRegistry<()> = CanvasRenderRegistry::new();
        let missing = assertions
            .require_all_canvas_handlers(&empty_canvas_registry)
            .expect_err("missing canvas handler");
        assert!(missing.message.contains("editor.viewport"));
    }

    #[test]
    fn accessibility_assertions_use_stable_node_names() {
        let mut document = UiDocument::new(root_style(260.0, 120.0));
        let root = document.root;
        let list = document.add_child(
            root,
            UiNode::container("choices", fixed_style(160.0, 80.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::List)
                    .label("Choices")
                    .value("2 options")
                    .focusable()
                    .focus_order(0),
            ),
        );
        let first = document.add_child(
            list,
            UiNode::container("choices.alpha", fixed_style(140.0, 24.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::ListItem)
                    .label("Alpha")
                    .selected(true)
                    .focusable()
                    .focus_order(1),
            ),
        );
        document.add_child(
            list,
            UiNode::container("choices.beta", fixed_style(140.0, 24.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::ListItem)
                    .label("Beta")
                    .focusable()
                    .focus_order(2),
            ),
        );
        document.node_mut(list).accessibility = Some(
            AccessibilityMeta::new(AccessibilityRole::List)
                .label("Choices")
                .value("2 options")
                .active_descendant(first)
                .focusable()
                .focus_order(0),
        );
        document.add_child(
            root,
            UiNode::container("status", fixed_style(160.0, 24.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Status)
                    .label("Sync")
                    .value("Ready")
                    .live_region(AccessibilityLiveRegion::Polite)
                    .summary(AccessibilitySummary::new("Sync").item("State", "Ready")),
            ),
        );
        document
            .compute_layout(UiSize::new(260.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let accessibility = AccessibilityAssertions::new(&document);
        accessibility
            .require_role("choices", AccessibilityRole::List)
            .expect("list role");
        accessibility
            .require_label("choices.alpha", "Alpha")
            .expect("option label");
        accessibility
            .require_value_contains("choices", "2 options")
            .expect("list value");
        accessibility
            .require_active_descendant("choices", "choices.alpha")
            .expect("active descendant");
        accessibility
            .require_focus_order(&["choices", "choices.alpha", "choices.beta"])
            .expect("focus order");
        accessibility
            .require_live_region("status", AccessibilityLiveRegion::Polite)
            .expect("status live region");
        accessibility
            .require_summary_contains("status", "State: Ready")
            .expect("status summary");
    }

    #[test]
    fn accessibility_request_assertions_check_adapter_requests() {
        let focused = UiNodeId(7);
        let preferences = AccessibilityPreferences::DEFAULT
            .screen_reader_active(true)
            .high_contrast(true);
        let tree = AccessibilityTree {
            nodes: Vec::new(),
            focus_order: vec![focused],
            modal_scope: None,
        };
        let requests = vec![
            AccessibilityAdapterRequest::PublishTree {
                tree: tree.clone(),
                focused: Some(focused),
                preferences,
            },
            AccessibilityAdapterRequest::ApplyPreferences(preferences),
            AccessibilityAdapterRequest::Announce(
                AccessibilityAnnouncement::polite("Save complete").source(focused),
            ),
        ];

        let assertions = AccessibilityRequestAssertions::new(&requests);
        assert_eq!(
            assertions.request_count(AccessibilityRequestKind::PublishTree),
            1
        );
        assert_eq!(
            assertions.request_count(AccessibilityRequestKind::ApplyPreferences),
            1
        );
        assert_eq!(assertions.requests(), requests.as_slice());
        let (published_tree, published_focus, published_preferences) = assertions
            .require_publish_tree()
            .expect("publish tree request");
        assert_eq!(published_tree, &tree);
        assert_eq!(published_focus, Some(focused));
        assert_eq!(published_preferences, preferences);
        assertions
            .require_request_kind(AccessibilityRequestKind::Announce)
            .expect("announcement request");
        assertions
            .require_apply_preferences(preferences)
            .expect("preferences request");
        let announcement = assertions
            .require_announcement_contains("complete")
            .expect("announcement text");
        assert_eq!(announcement.source, Some(focused));
        assert!(assertions.require_announcement_contains("missing").is_err());
    }

    #[test]
    fn platform_assertions_match_requests_responses_and_errors() {
        let clipboard_id = PlatformRequestId::new(10);
        let repaint_id = PlatformRequestId::new(11);
        let output = HostFrameOutput::new(HostInteractionState::default())
            .request(
                clipboard_id,
                PlatformRequest::Clipboard(ClipboardRequest::ReadText),
            )
            .request(
                repaint_id,
                PlatformRequest::Repaint(RepaintRequest::NextFrame),
            )
            .response(
                clipboard_id,
                PlatformResponse::Clipboard(ClipboardResponse::Text(Some("copied".into()))),
            )
            .response(
                repaint_id,
                PlatformResponse::Repaint(RepaintResponse::Scheduled {
                    delay: Duration::from_millis(16),
                }),
            );

        let platform = PlatformAssertions::from_host_frame(&output);
        assert_eq!(platform.request_count(PlatformServiceKind::Clipboard), 1);
        assert_eq!(platform.response_count(PlatformServiceKind::Repaint), 1);
        let clipboard_request = platform
            .require_request_kind(PlatformServiceKind::Clipboard)
            .expect("clipboard request");
        let clipboard_response = platform
            .require_response_for(clipboard_request)
            .expect("clipboard response");
        assert!(matches!(
            clipboard_response.response,
            PlatformResponse::Clipboard(ClipboardResponse::Text(Some(_)))
        ));
        platform
            .require_all_responses_match_requests()
            .expect("matched responses");
        platform.require_no_error_responses().expect("no errors");

        let unmatched = HostFrameOutput::new(HostInteractionState::default()).response(
            PlatformRequestId::new(99),
            PlatformResponse::Repaint(RepaintResponse::Coalesced),
        );
        assert!(PlatformAssertions::from_host_frame(&unmatched)
            .require_all_responses_match_requests()
            .is_err());

        let error_output = HostFrameOutput::new(HostInteractionState::default())
            .request(
                clipboard_id,
                PlatformRequest::Clipboard(ClipboardRequest::ReadText),
            )
            .response(
                clipboard_id,
                PlatformResponse::Clipboard(ClipboardResponse::Error(PlatformServiceError::new(
                    PlatformErrorCode::Denied,
                    "clipboard blocked",
                ))),
            );
        assert!(PlatformAssertions::from_host_frame(&error_output)
            .require_no_error_responses()
            .is_err());
    }

    #[test]
    fn platform_assertions_check_unsupported_service_responses() {
        let request = PlatformServiceRequest::new(
            PlatformRequestId::new(22),
            PlatformRequest::Clipboard(ClipboardRequest::ReadText),
        );
        let unsupported = request.unsupported_response();
        let supported = PlatformServiceResponse::new(
            request.id,
            PlatformResponse::Clipboard(ClipboardResponse::Text(Some("text".into()))),
        );

        let unsupported_platform = PlatformAssertions::new(
            std::slice::from_ref(&request),
            std::slice::from_ref(&unsupported),
        );
        assert_eq!(
            unsupported_platform
                .require_unsupported_response_for(&request)
                .expect("unsupported response"),
            &unsupported
        );
        assert!(unsupported_platform
            .require_no_unsupported_responses()
            .is_err());

        let supported_platform = PlatformAssertions::new(
            std::slice::from_ref(&request),
            std::slice::from_ref(&supported),
        );
        assert!(supported_platform
            .require_unsupported_response_for(&request)
            .is_err());
        supported_platform
            .require_no_unsupported_responses()
            .expect("no unsupported responses");
    }

    #[test]
    fn platform_assertions_can_use_document_frame_generated_requests() {
        let viewport = UiSize::new(220.0, 120.0);
        let mut document = UiDocument::new(root_style(viewport.width, viewport.height));
        let canvas = document.add_child(
            document.root,
            UiNode::canvas("viewport", "app.viewport", fixed_style(120.0, 80.0).layout),
        );
        document.set_node_content(
            canvas,
            UiContent::Canvas(
                CanvasContent::new("app.viewport")
                    .native_viewport()
                    .interaction(CanvasInteractionPolicy::NATIVE_VIEWPORT),
            ),
        );
        let host_output = HostFrameOutput::new(HostInteractionState::default())
            .repaint_next_frame(PlatformRequestId::new(4));
        let frame = process_document_frame(
            &mut document,
            &mut ApproxTextMeasurer,
            HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::window("main", viewport),
                host_output,
            ),
        )
        .expect("document frame");

        let mut allocator = PlatformRequestIdAllocator::new(50);
        let platform = PlatformAssertions::from_document_frame(&frame, &mut allocator);

        assert_eq!(platform.request_count(PlatformServiceKind::Repaint), 1);
        assert_eq!(platform.request_count(PlatformServiceKind::Cursor), 2);
        assert_eq!(
            platform
                .requests()
                .iter()
                .map(|request| request.id)
                .collect::<Vec<_>>(),
            vec![
                PlatformRequestId::new(4),
                PlatformRequestId::new(50),
                PlatformRequestId::new(51),
            ]
        );
        assert_eq!(allocator.next_value(), 52);
        let cursor_request = platform
            .require_request_kind(PlatformServiceKind::Cursor)
            .expect("cursor request");
        assert_eq!(
            cursor_request.request,
            PlatformRequest::Cursor(CursorRequest::Confine(LogicalRect::new(
                0.0, 0.0, 120.0, 80.0
            )))
        );
    }

    #[test]
    fn pixel_diff_reports_tolerance_compatible_changes() {
        let expected = [0, 0, 0, 255, 10, 20, 30, 255];
        let actual = [0, 1, 0, 255, 12, 18, 31, 255];
        let report = diff_rgba8(
            RgbaImageView::new(2, 1, &expected).expect("expected view"),
            RgbaImageView::new(2, 1, &actual).expect("actual view"),
        )
        .expect("diff");

        assert_eq!(report.changed_pixels, 2);
        assert_eq!(report.max_channel_delta, 2);
        assert_eq!(report.total_channel_delta, 6);
        assert!(report.is_within(PixelDiffTolerance {
            max_changed_pixels: 2,
            max_channel_delta: 2,
            max_total_channel_delta: 6,
        }));
        assert!(!report.is_within(PixelDiffTolerance::EXACT));
    }

    #[test]
    fn dirty_flags_and_frame_timing_track_test_budget_state() {
        let flags = DirtyFlags {
            layout: true,
            input: true,
            ..DirtyFlags::NONE
        }
        .union(DirtyFlags {
            paint: true,
            ..DirtyFlags::NONE
        });
        assert!(flags.any());
        assert!(flags.layout && flags.paint && flags.input);

        let timing = FrameTiming::new()
            .section("layout", Duration::from_millis(3))
            .section("paint", Duration::from_millis(4))
            .section("render", Duration::from_millis(8));
        assert_eq!(timing.duration("paint"), Some(Duration::from_millis(4)));
        assert_eq!(timing.total(), Duration::from_millis(15));
        assert!(timing.within_budget(Duration::from_millis(16)));
        assert!(!timing.within_budget(Duration::from_millis(10)));
    }

    #[test]
    fn replay_reports_unconverted_raw_events() {
        let mut document = UiDocument::new(root_style(100.0, 50.0));
        document
            .compute_layout(UiSize::new(100.0, 50.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let report = EventReplay::new()
            .raw_scaled(
                "wheel",
                RawInputEvent::Wheel(RawWheelEvent::lines(
                    UiPoint::new(1.0, 1.0),
                    UiPoint::new(0.0, 1.0),
                    1,
                )),
                20.0,
                UiSize::new(100.0, 50.0),
            )
            .raw(
                "key-up",
                RawInputEvent::Keyboard(RawKeyboardEvent::release(
                    crate::KeyCode::Enter,
                    crate::KeyModifiers::NONE,
                    2,
                )),
            )
            .run(&mut document);

        assert_eq!(
            report.steps[0].converted,
            Some(UiInputEvent::Wheel {
                position: UiPoint::new(1.0, 1.0),
                delta: UiPoint::new(0.0, 20.0),
            })
        );
        assert!(report.steps[1].converted.is_none());
        assert!(report.require_all_converted().is_err());
    }
}
