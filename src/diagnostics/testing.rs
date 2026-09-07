//! Renderer-neutral testing helpers for Operad documents.
//!
//! These utilities are intended for consumers as well as Operad's own tests:
//! replay input without a host harness, assert layout by stable node names,
//! inspect paint lists, diff rgba snapshots with tolerances, and track simple
//! frame timing sections.

use crate::core::timing::{FrameTiming, FrameTimingSectionSummary};
use std::borrow::Cow;
use std::fmt;
use std::time::{Duration, Instant};

use crate::accessibility::{
    AccessibilityAdapterRequest, AccessibilityAdapterResponse, AccessibilityAnnouncement,
    AccessibilityPreferences, AccessibilityRequestKind, FocusRestoreTarget,
};
use crate::commands::{CommandId, CommandRegistry};
use crate::core::document::AuditWarning;
use crate::debug::{
    DebugCaptureReport, DebugContractReport, DebugEventRouteTrace, DebugFrameAutopsyTrace,
    DebugFrameBottleneckTrace, DebugFrameBudgetTrace, DebugFrameRecorder, DebugFrameTrace,
    DebugFrameTraceContext, DebugFrameTraceStatus, DebugInspectNodeTrace, DebugInspectPointTrace,
    DebugInspectorSnapshot, DebugIssueReport, DebugOverlapKind, DebugOverlapReason,
    DebugOverlapRecord, DebugOverlapReport, DebugPaintHitMismatchRecord,
    DebugPaintHitMismatchTrace, DebugPointAutopsyTrace, DebugPointerProbe,
    DebugPointerProbeCandidate, DebugPointerRouteKind, DebugResourceReport, DebugSlowFrameTrace,
    DebugSlowNodeTrace, DebugTextFitTrace, DebugVisibilityRecord, DebugVisibilityTrace,
    DebugWhyTrace,
};
use crate::diagnostics::FrameTimingExplanation;
use crate::display::{
    DisplayListInvalidationReport, DisplayListKey, DisplayListReuseOutcome, DisplayListReuseReport,
};
use crate::host::{
    process_document_frame, HostCommandDispatch, HostDocumentFrameOutput, HostDocumentFrameState,
    HostFrameOutput, HostInteractionState, HostNodeInteraction, HostShortcutRoute,
};
use crate::input::RawInputEvent;
use crate::platform::{
    AppLifecycleResponse, BackendAdapterKind, BackendCapabilities, ClipboardResponse,
    CursorResponse, DragDropResponse, FileDialogResponse, LayerCapabilities, NotificationResponse,
    OpenUrlResponse, PlatformRequestId, PlatformRequestIdAllocator, PlatformResponse,
    PlatformServiceCapabilities, PlatformServiceError, PlatformServiceKind, PlatformServiceRequest,
    PlatformServiceResponse, RenderingCapabilities, RepaintResponse, ResourceCapabilities,
    ScreenshotResponse, TextImeResponse,
};
use crate::renderer::EmptyResourceResolver;
use crate::renderer::{
    CanvasHitCollection, CanvasHitTarget, CanvasHostCaptureDiagnosticReport, CanvasRenderRegistry,
    CanvasRenderReport, CanvasRenderRequest, ImageRenderRegistry, ImageRenderRequest, RenderError,
    RenderFrameOutput, RenderFrameRequest, RenderTarget, RenderTargetKind, RenderedImage,
    RendererAdapter, ResourceFormat, ResourceResolver,
};
use crate::{
    AccessibilityLiveRegion, AccessibilityNode, AccessibilityRelationKind, AccessibilityRole,
    AccessibilityStateKind, AccessibilityTree, AccessibilityValueRangeIssue, ApproxTextMeasurer,
    ColorRgba, FocusDirection, KeyCode, KeyModifiers, PaintItem, PaintKind, PaintList,
    TextMeasurer, UiDocument, UiInputEvent, UiInputResult, UiNode, UiNodeId, UiPoint, UiRect,
    UiSize,
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

pub fn test_host_capabilities() -> BackendCapabilities {
    BackendCapabilities::test_host()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PaintRecorderRenderer;

impl RendererAdapter for PaintRecorderRenderer {
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::new("paint-recorder")
            .adapter(BackendAdapterKind::Test)
            .resources(ResourceCapabilities::NONE)
            .layers(LayerCapabilities::STANDARD)
            .rendering(RenderingCapabilities {
                high_dpi: false,
                offscreen: false,
                deterministic_snapshots: false,
                partial_updates: true,
                ..RenderingCapabilities::NONE
            })
    }

    fn render_frame(
        &mut self,
        request: RenderFrameRequest,
        _resolver: &dyn ResourceResolver,
    ) -> Result<RenderFrameOutput, RenderError> {
        let batch_started = Instant::now();
        let batches = request.batches();
        let batch_duration = batch_started.elapsed();
        let render_started = Instant::now();
        let mut output = RenderFrameOutput::new(request.target);
        output.painted_items = request.paint.items.len();
        output.batches = batches;
        output.dirty_regions = request.dirty_regions;
        output.timings = FrameTiming::new()
            .section("batch", batch_duration)
            .section("render", render_started.elapsed());
        Ok(output)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayInput {
    Ui(UiInputEvent),
    Raw {
        event: RawInputEvent,
        line_size: f32,
        page_size: UiSize,
    },
    Command(CommandId),
    PlatformResponse(PlatformServiceResponse),
    WindowResize(UiSize),
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
    pub platform_response: Option<PlatformServiceResponse>,
    pub viewport_resize: Option<UiSize>,
    pub result: Option<UiInputResult>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventReplayPointerEvent {
    pub label: String,
    pub event: DebugPointerRouteKind,
    pub point: UiPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventReplayPointerDiagnostic<T> {
    pub label: String,
    pub event: DebugPointerRouteKind,
    pub point: UiPoint,
    pub trace: T,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventReplay {
    pub steps: Vec<EventReplayStep>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InteractionRecorder {
    replay: EventReplay,
}

impl InteractionRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_ui(&mut self, label: impl Into<String>, event: UiInputEvent) -> &mut Self {
        self.replay = std::mem::take(&mut self.replay).ui(label, event);
        self
    }

    pub fn record_raw(&mut self, label: impl Into<String>, event: RawInputEvent) -> &mut Self {
        self.replay = std::mem::take(&mut self.replay).raw(label, event);
        self
    }

    pub fn record_wheel(
        &mut self,
        label: impl Into<String>,
        position: UiPoint,
        delta: UiPoint,
    ) -> &mut Self {
        self.record_ui(label, UiInputEvent::wheel(position, delta))
    }

    pub fn record_command(
        &mut self,
        label: impl Into<String>,
        command: impl Into<CommandId>,
    ) -> &mut Self {
        self.replay = std::mem::take(&mut self.replay).command(label, command);
        self
    }

    pub fn record_platform_response(
        &mut self,
        label: impl Into<String>,
        response: PlatformServiceResponse,
    ) -> &mut Self {
        self.replay = std::mem::take(&mut self.replay).platform_response(label, response);
        self
    }

    pub fn record_window_resize(
        &mut self,
        label: impl Into<String>,
        viewport: UiSize,
    ) -> &mut Self {
        self.replay = std::mem::take(&mut self.replay).window_resize(label, viewport);
        self
    }

    pub fn replay(&self) -> &EventReplay {
        &self.replay
    }

    pub fn into_replay(self) -> EventReplay {
        self.replay
    }
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

    pub fn pointer_move(self, label: impl Into<String>, point: UiPoint) -> Self {
        self.ui(label, UiInputEvent::PointerMove(point))
    }

    pub fn pointer_down(self, label: impl Into<String>, point: UiPoint) -> Self {
        self.ui(label, UiInputEvent::PointerDown(point))
    }

    pub fn pointer_up(self, label: impl Into<String>, point: UiPoint) -> Self {
        self.ui(label, UiInputEvent::PointerUp(point))
    }

    pub fn pointer_click(self, label: impl Into<String>, point: UiPoint) -> Self {
        let label = label.into();
        self.pointer_move(format!("{label}.move"), point)
            .pointer_down(format!("{label}.down"), point)
            .pointer_up(format!("{label}.up"), point)
    }

    pub fn pointer_drag(
        self,
        label: impl Into<String>,
        start: UiPoint,
        end: UiPoint,
        intermediate_points: impl IntoIterator<Item = UiPoint>,
    ) -> Self {
        let label = label.into();
        let mut replay = self
            .pointer_move(format!("{label}.move.start"), start)
            .pointer_down(format!("{label}.down"), start);
        for (index, point) in intermediate_points.into_iter().enumerate() {
            replay = replay.pointer_move(format!("{label}.move.{index}"), point);
        }
        replay
            .pointer_move(format!("{label}.move.end"), end)
            .pointer_up(format!("{label}.up"), end)
    }

    pub fn wheel(self, label: impl Into<String>, position: UiPoint, delta: UiPoint) -> Self {
        self.ui(label, UiInputEvent::wheel(position, delta))
    }

    pub fn command(mut self, label: impl Into<String>, command: impl Into<CommandId>) -> Self {
        self.steps.push(EventReplayStep {
            label: label.into(),
            input: ReplayInput::Command(command.into()),
        });
        self
    }

    pub fn platform_response(
        mut self,
        label: impl Into<String>,
        response: PlatformServiceResponse,
    ) -> Self {
        self.steps.push(EventReplayStep {
            label: label.into(),
            input: ReplayInput::PlatformResponse(response),
        });
        self
    }

    pub fn window_resize(mut self, label: impl Into<String>, viewport: UiSize) -> Self {
        self.steps.push(EventReplayStep {
            label: label.into(),
            input: ReplayInput::WindowResize(viewport),
        });
        self
    }

    pub fn long_wheel_scroll(
        mut self,
        label: impl Into<String>,
        position: UiPoint,
        delta: UiPoint,
        steps: usize,
    ) -> Self {
        let label = label.into();
        for index in 0..steps {
            self = self.wheel(format!("{label}.{index}"), position, delta);
        }
        self
    }

    pub fn key(self, label: impl Into<String>, key: KeyCode, modifiers: KeyModifiers) -> Self {
        self.ui(label, UiInputEvent::Key { key, modifiers })
    }

    pub fn focus(self, label: impl Into<String>, direction: FocusDirection) -> Self {
        self.ui(label, UiInputEvent::Focus(direction))
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
            let converted = replay_input_to_ui_event(&step.input);
            let platform_response = replay_input_to_platform_response(&step.input);
            let result = converted.clone().map(|event| document.handle_input(event));
            steps.push(EventReplayStepResult {
                label: step.label.clone(),
                input: step.input.clone(),
                converted,
                platform_response,
                viewport_resize: replay_input_to_viewport_resize(&step.input),
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
            let converted = replay_input_to_ui_event(&step.input);
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
            let direct_command = match &step.input {
                ReplayInput::Command(command)
                    if registry
                        .command(command)
                        .is_some_and(|command| command.enabled) =>
                {
                    Some(command.clone())
                }
                ReplayInput::Command(_)
                | ReplayInput::Ui(_)
                | ReplayInput::Raw { .. }
                | ReplayInput::PlatformResponse(_)
                | ReplayInput::WindowResize(_) => None,
            };

            steps.push(CommandReplayStepResult {
                label: step.label.clone(),
                input: step.input.clone(),
                converted,
                result,
                platform_response: replay_input_to_platform_response(&step.input),
                viewport_resize: replay_input_to_viewport_resize(&step.input),
                shortcut_route,
                dispatch,
                direct_command,
            });
        }
        CommandReplayReport { steps, state }
    }
}

fn replay_input_updates_host_state(event: &UiInputEvent) -> bool {
    !matches!(event, UiInputEvent::Key { .. } | UiInputEvent::TextInput(_))
}

fn replay_input_to_ui_event(input: &ReplayInput) -> Option<UiInputEvent> {
    match input {
        ReplayInput::Ui(event) => Some(event.clone()),
        ReplayInput::Raw {
            event,
            line_size,
            page_size,
        } => event.to_ui_input_event_with_wheel_scale(*line_size, *page_size),
        ReplayInput::Command(_)
        | ReplayInput::PlatformResponse(_)
        | ReplayInput::WindowResize(_) => None,
    }
}

fn ui_input_pointer_event(event: &UiInputEvent) -> Option<(DebugPointerRouteKind, UiPoint)> {
    match event {
        UiInputEvent::PointerMove(point) => Some((DebugPointerRouteKind::Move, *point)),
        UiInputEvent::PointerDown(point) => Some((DebugPointerRouteKind::Down, *point)),
        UiInputEvent::PointerUp(point) => Some((DebugPointerRouteKind::Up, *point)),
        UiInputEvent::Wheel(wheel) => Some((DebugPointerRouteKind::Wheel, wheel.position)),
        UiInputEvent::TextInput(_) | UiInputEvent::Key { .. } | UiInputEvent::Focus(_) => None,
    }
}

fn replay_pointer_step(step: &EventReplayStepResult) -> Option<EventReplayPointerEvent> {
    let converted = step.converted.as_ref()?;
    let (event, point) = ui_input_pointer_event(converted)?;
    Some(EventReplayPointerEvent {
        label: step.label.clone(),
        event,
        point,
    })
}

fn replay_input_to_platform_response(input: &ReplayInput) -> Option<PlatformServiceResponse> {
    match input {
        ReplayInput::PlatformResponse(response) => Some(response.clone()),
        ReplayInput::Ui(_)
        | ReplayInput::Raw { .. }
        | ReplayInput::Command(_)
        | ReplayInput::WindowResize(_) => None,
    }
}

fn replay_input_to_viewport_resize(input: &ReplayInput) -> Option<UiSize> {
    match input {
        ReplayInput::WindowResize(viewport) => Some(*viewport),
        ReplayInput::Ui(_)
        | ReplayInput::Raw { .. }
        | ReplayInput::Command(_)
        | ReplayInput::PlatformResponse(_) => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioFrameReport {
    pub label: String,
    pub events: EventReplayReport,
    pub document: HostDocumentFrameOutput,
    pub render: RenderFrameOutput,
    pub timings: FrameTiming,
    pub platform_requests: Vec<PlatformServiceRequest>,
}

impl ScenarioFrameReport {
    pub fn render_assertions(&self) -> RenderOutputAssertions<'_> {
        RenderOutputAssertions::new(&self.render)
    }

    pub fn timing_assertions(&self) -> FrameTimingAssertions<'_> {
        FrameTimingAssertions::new(&self.timings)
    }

    pub fn platform_assertions(&self) -> PlatformAssertions<'_> {
        PlatformAssertions::new(
            &self.platform_requests,
            &self.document.host_output.platform_responses,
        )
    }

    pub fn canvas_capture_diagnostics(
        &self,
        capabilities: PlatformServiceCapabilities,
    ) -> CanvasHostCaptureDiagnosticReport {
        CanvasHostCaptureDiagnosticReport::from_state_transition(
            &self.document.host_output.state.canvas_host_capture,
            &self.document.canvas_host_capture_transition,
            capabilities,
        )
        .with_platform_responses(
            &self.platform_requests,
            &self.document.host_output.platform_responses,
        )
    }

    pub fn debug_event_routes(&self, document: &UiDocument) -> Vec<DebugEventRouteTrace> {
        self.events.debug_event_routes(document)
    }

    pub fn pointer_steps(&self) -> impl Iterator<Item = EventReplayPointerEvent> + '_ {
        self.events.pointer_steps()
    }

    pub fn pointer_step(&self, label: &str) -> TestResult<EventReplayPointerEvent> {
        self.events.pointer_step(label)
    }

    pub fn last_pointer_event(&self) -> Option<(DebugPointerRouteKind, UiPoint)> {
        self.events.last_pointer_event()
    }

    pub fn last_pointer_step(&self) -> Option<EventReplayPointerEvent> {
        self.events.last_pointer_step()
    }

    pub fn debug_point_autopsy(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        label: &str,
    ) -> TestResult<DebugPointAutopsyTrace> {
        self.events
            .debug_point_autopsy(document, text_measurer, label)
    }

    pub fn debug_last_point_autopsy(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Option<DebugPointAutopsyTrace> {
        self.events
            .debug_last_point_autopsy(document, text_measurer)
    }

    pub fn debug_why_point(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        label: &str,
    ) -> TestResult<DebugWhyTrace> {
        self.events.debug_why_point(document, text_measurer, label)
    }

    pub fn debug_last_why_point(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Option<DebugWhyTrace> {
        self.events.debug_last_why_point(document, text_measurer)
    }

    pub fn debug_point_autopsies(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Vec<EventReplayPointerDiagnostic<DebugPointAutopsyTrace>> {
        self.events.debug_point_autopsies(document, text_measurer)
    }

    pub fn debug_inspect_point(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        label: &str,
    ) -> TestResult<DebugInspectPointTrace> {
        self.events
            .debug_inspect_point(document, text_measurer, label)
    }

    pub fn debug_last_inspect_point(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Option<DebugInspectPointTrace> {
        self.events
            .debug_last_inspect_point(document, text_measurer)
    }

    pub fn debug_inspect_points(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Vec<EventReplayPointerDiagnostic<DebugInspectPointTrace>> {
        self.events.debug_inspect_points(document, text_measurer)
    }

    pub fn debug_frame_context(
        &self,
        document: &UiDocument,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> DebugFrameTraceContext {
        let mut context = DebugFrameTraceContext::new(self.label.clone(), self.viewport())
            .timing(self.timings.clone())
            .dirty_flags(self.document.render_request.dirty_flags)
            .routes(self.debug_event_routes(document))
            .resources(
                DebugResourceReport::from_render_frame_request_without_resolver(
                    &self.document.render_request,
                ),
            );
        if let Some(budget) = timing_budget {
            context = context.timing_budget(budget);
        }
        if let Some(selected_node) = selected_node {
            context = context.selected_node(selected_node);
        }
        context
    }

    pub fn debug_frame_trace(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> DebugFrameTrace {
        self.debug_frame_trace_with_options(document, text_measurer, None, None)
    }

    pub fn debug_frame_trace_with_options(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> DebugFrameTrace {
        DebugFrameTrace::from_document(
            document,
            text_measurer,
            self.debug_frame_context(document, timing_budget, selected_node),
        )
    }

    pub fn debug_why_frame(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> DebugWhyTrace {
        self.debug_why_frame_with_options(document, text_measurer, None, None)
    }

    pub fn debug_why_frame_with_options(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> DebugWhyTrace {
        let trace = self.debug_frame_trace_with_options(
            document,
            text_measurer,
            timing_budget,
            selected_node,
        );
        DebugWhyTrace::from_frame_trace(&trace)
    }

    pub fn record_debug_frame<'a>(
        &self,
        recorder: &'a mut DebugFrameRecorder,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Option<&'a DebugFrameTrace> {
        self.record_debug_frame_with_options(recorder, document, text_measurer, None, None, None)
    }

    pub fn record_debug_frame_with_options<'a>(
        &self,
        recorder: &'a mut DebugFrameRecorder,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        frame_index: Option<u64>,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> Option<&'a DebugFrameTrace> {
        let mut context = self.debug_frame_context(document, timing_budget, selected_node);
        if let Some(frame_index) = frame_index {
            context = context.frame_index(frame_index);
        }
        recorder.record_document(document, text_measurer, context)
    }

    pub fn debug_inspect_node_trace(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        selected_node: &str,
    ) -> Option<DebugInspectNodeTrace> {
        let route = self
            .last_pointer_event()
            .map(|(event, point)| DebugEventRouteTrace::from_pointer_event(document, event, point));
        self.debug_inspect_node_trace_with_route_context(
            document,
            text_measurer,
            selected_node,
            route.as_ref(),
        )
    }

    pub fn debug_inspect_node_trace_for_step(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        selected_node: &str,
        label: &str,
    ) -> TestResult<DebugInspectNodeTrace> {
        let step = self.pointer_step(label)?;
        let route = DebugEventRouteTrace::from_pointer_event(document, step.event, step.point);
        self.debug_inspect_node_trace_with_route_context(
            document,
            text_measurer,
            selected_node,
            Some(&route),
        )
        .ok_or_else(|| {
            TestFailure::new(format!(
                "expected inspect-node trace for `{selected_node}` in scenario `{}`",
                self.label
            ))
        })
    }

    pub fn debug_why_node(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        selected_node: &str,
    ) -> Option<DebugWhyTrace> {
        self.debug_inspect_node_trace(document, text_measurer, selected_node)
            .map(|trace| DebugWhyTrace::from_inspect_node(&trace))
    }

    pub fn debug_why_node_for_step(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        selected_node: &str,
        label: &str,
    ) -> TestResult<DebugWhyTrace> {
        let trace =
            self.debug_inspect_node_trace_for_step(document, text_measurer, selected_node, label)?;
        Ok(DebugWhyTrace::from_inspect_node(&trace))
    }

    fn debug_inspect_node_trace_with_route_context(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        selected_node: &str,
        route: Option<&DebugEventRouteTrace>,
    ) -> Option<DebugInspectNodeTrace> {
        LayoutAssertions::new(document).node(selected_node).ok()?;
        let snapshot = DebugInspectorSnapshot::from_document(document, text_measurer);
        let resources = DebugResourceReport::from_render_frame_request_without_resolver(
            &self.document.render_request,
        );
        let slow_nodes = DebugSlowNodeTrace::from_document(document, text_measurer);
        let text_fit = DebugTextFitTrace::from_document(document, text_measurer);
        DebugInspectNodeTrace::from_context_with_traces(
            &snapshot,
            Some(selected_node),
            route,
            Some(&resources),
            Some(&slow_nodes),
            Some(&text_fit),
        )
    }

    pub fn debug_capture_report(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> DebugCaptureReport {
        self.debug_capture_report_with_options(document, text_measurer, None, None)
    }

    pub fn debug_capture_report_with_options(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> DebugCaptureReport {
        let trace = self.debug_frame_trace_with_options(
            document,
            text_measurer,
            timing_budget,
            selected_node,
        );
        DebugCaptureReport::from_frame_trace(&trace)
    }

    pub fn debug_frame_budget_trace(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> DebugFrameBudgetTrace {
        self.debug_frame_budget_trace_with_options(document, text_measurer, None, None)
    }

    pub fn debug_frame_budget_trace_with_options(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> DebugFrameBudgetTrace {
        let trace = self.debug_frame_trace_with_options(
            document,
            text_measurer,
            timing_budget,
            selected_node,
        );
        DebugFrameBudgetTrace::from_trace(&trace)
    }

    pub fn debug_issue_report(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> DebugIssueReport {
        self.debug_issue_report_with_options(document, text_measurer, None, None)
    }

    pub fn debug_issue_report_with_options(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> DebugIssueReport {
        let trace = self.debug_frame_trace_with_options(
            document,
            text_measurer,
            timing_budget,
            selected_node,
        );
        DebugIssueReport::from_frame_trace(&trace)
    }

    pub fn debug_slow_frame_trace(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> DebugSlowFrameTrace {
        self.debug_slow_frame_trace_with_options(document, text_measurer, None, None)
    }

    pub fn debug_slow_frame_trace_with_options(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> DebugSlowFrameTrace {
        let trace = self.debug_frame_trace_with_options(
            document,
            text_measurer,
            timing_budget,
            selected_node,
        );
        let slow_nodes = DebugSlowNodeTrace::from_document(document, text_measurer);
        DebugSlowFrameTrace::from_trace_with_slow_nodes(&trace, &slow_nodes)
    }

    pub fn debug_frame_bottleneck_trace(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> DebugFrameBottleneckTrace {
        self.debug_frame_bottleneck_trace_with_options(document, text_measurer, None, None)
    }

    pub fn debug_frame_bottleneck_trace_with_options(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> DebugFrameBottleneckTrace {
        let trace = self.debug_frame_trace_with_options(
            document,
            text_measurer,
            timing_budget,
            selected_node,
        );
        let slow_nodes = DebugSlowNodeTrace::from_document(document, text_measurer);
        DebugFrameBottleneckTrace::from_trace_with_slow_nodes(&trace, &slow_nodes)
    }

    pub fn debug_frame_autopsy_trace(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> DebugFrameAutopsyTrace {
        self.debug_frame_autopsy_trace_with_options(document, text_measurer, None, None)
    }

    pub fn debug_frame_autopsy_trace_with_options(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> DebugFrameAutopsyTrace {
        let trace = self.debug_frame_trace_with_options(
            document,
            text_measurer,
            timing_budget,
            selected_node,
        );
        DebugFrameAutopsyTrace::from_trace(&trace)
    }

    pub fn debug_contract_report(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> DebugContractReport {
        self.debug_contract_report_with_options(document, text_measurer, None, None)
    }

    pub fn debug_contract_report_with_options(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        timing_budget: Option<Duration>,
        selected_node: Option<&str>,
    ) -> DebugContractReport {
        let trace = self.debug_frame_trace_with_options(
            document,
            text_measurer,
            timing_budget,
            selected_node,
        );
        DebugContractReport::from_frame_trace(&trace)
    }

    pub fn snapshot_assertions(
        &self,
        name: impl Into<String>,
    ) -> TestResult<SnapshotAssertions<'_>> {
        self.render_assertions().require_snapshot_rgba8(name)
    }

    pub const fn viewport(&self) -> UiSize {
        self.document.render_request.viewport
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioHarness {
    pub viewport: UiSize,
    pub target: RenderTarget,
    pub state: HostDocumentFrameState,
    pub platform_allocator: PlatformRequestIdAllocator,
}

impl ScenarioHarness {
    pub fn new(viewport: UiSize) -> Self {
        Self {
            viewport,
            target: RenderTarget::window("scenario", viewport),
            state: HostDocumentFrameState::new(),
            platform_allocator: PlatformRequestIdAllocator::new(1),
        }
    }

    pub fn target(mut self, target: RenderTarget) -> Self {
        self.target = target;
        self
    }

    pub fn state(mut self, state: HostDocumentFrameState) -> Self {
        self.state = state;
        self
    }

    pub const fn current_state(&self) -> &HostDocumentFrameState {
        &self.state
    }

    pub fn run_frame(
        &mut self,
        label: impl Into<String>,
        document: &mut UiDocument,
        replay: EventReplay,
    ) -> TestResult<ScenarioFrameReport> {
        let mut measurer = ApproxTextMeasurer;
        let mut renderer = PaintRecorderRenderer;
        self.run_frame_with_measurer_and_renderer(
            label,
            document,
            replay,
            &mut measurer,
            &mut renderer,
            &EmptyResourceResolver,
        )
    }

    pub fn run_frame_with_measurer_and_renderer(
        &mut self,
        label: impl Into<String>,
        document: &mut UiDocument,
        replay: EventReplay,
        measurer: &mut impl TextMeasurer,
        renderer: &mut impl RendererAdapter,
        resolver: &dyn ResourceResolver,
    ) -> TestResult<ScenarioFrameReport> {
        let label = label.into();
        if let Some(viewport) = replay_final_viewport(&replay) {
            self.viewport = viewport;
            self.target = render_target_with_viewport(&self.target, viewport);
        }
        let pre_input_layout_started = Instant::now();
        document
            .compute_layout(self.viewport, measurer)
            .map_err(|error| {
                TestFailure::new(format!(
                    "scenario `{label}` pre-input layout failed: {error}"
                ))
            })?;
        let pre_input_layout_duration = pre_input_layout_started.elapsed();

        let input_started = Instant::now();
        let (host_output, mut events) =
            scenario_host_output_from_replay(&replay, self.state.interaction.clone());
        let input_duration = input_started.elapsed();
        let request =
            self.state
                .document_frame_request(self.viewport, self.target.clone(), host_output);
        let document_started = Instant::now();
        let document_output =
            process_document_frame(document, measurer, request).map_err(|error| {
                TestFailure::new(format!("scenario `{label}` document frame failed: {error}"))
            })?;
        let document_duration = document_started.elapsed();
        attach_scenario_input_results(&mut events, &document_output.input_results);
        let render_request = document_output.render_request.clone();
        let render_started = Instant::now();
        let render_output = renderer
            .render_frame(render_request, resolver)
            .map_err(|error| {
                TestFailure::new(format!("scenario `{label}` render frame failed: {error}"))
            })?;
        let render_duration = render_started.elapsed();
        let platform_started = Instant::now();
        let platform_requests =
            document_output.platform_service_requests(&mut self.platform_allocator);
        let platform_duration = platform_started.elapsed();
        let timings = FrameTiming::new()
            .section("pre-input-layout", pre_input_layout_duration)
            .section("input", input_duration)
            .section("document-frame", document_duration)
            .section("render-frame", render_duration)
            .section("platform-requests", platform_duration);
        self.state.apply_document_frame_output(&document_output);
        Ok(ScenarioFrameReport {
            label,
            events,
            document: document_output,
            render: render_output,
            timings,
            platform_requests,
        })
    }
}

fn scenario_host_output_from_replay(
    replay: &EventReplay,
    state: HostInteractionState,
) -> (HostFrameOutput, EventReplayReport) {
    let mut output = HostFrameOutput::new(state);
    let mut steps = Vec::with_capacity(replay.steps.len());
    for step in &replay.steps {
        let converted = replay_input_to_ui_event(&step.input);
        if let Some(event) = converted.clone() {
            output.ui_events.push(event);
        }
        let platform_response = replay_input_to_platform_response(&step.input);
        if let Some(response) = platform_response.clone() {
            output.platform_responses.push(response);
        }
        let viewport_resize = replay_input_to_viewport_resize(&step.input);
        steps.push(EventReplayStepResult {
            label: step.label.clone(),
            input: step.input.clone(),
            converted,
            platform_response,
            viewport_resize,
            result: None,
        });
    }
    (output, EventReplayReport { steps })
}

fn replay_final_viewport(replay: &EventReplay) -> Option<UiSize> {
    replay
        .steps
        .iter()
        .filter_map(|step| replay_input_to_viewport_resize(&step.input))
        .last()
}

fn render_target_with_viewport(target: &RenderTarget, viewport: UiSize) -> RenderTarget {
    match target {
        RenderTarget::Window { id, .. } => RenderTarget::window(id.clone(), viewport),
        RenderTarget::AppOwned { id, .. } => RenderTarget::app_owned(id.clone(), viewport),
        RenderTarget::Offscreen { .. } | RenderTarget::Snapshot { .. } => target.clone(),
    }
}

fn attach_scenario_input_results(events: &mut EventReplayReport, input_results: &[UiInputResult]) {
    let mut results = input_results.iter().cloned();
    for step in &mut events.steps {
        if step.converted.is_some() {
            step.result = results.next();
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventReplayReport {
    pub steps: Vec<EventReplayStepResult>,
}

impl EventReplayReport {
    pub fn step(&self, label: &str) -> TestResult<&EventReplayStepResult> {
        self.steps
            .iter()
            .find(|step| step.label == label)
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing event replay step `{label}`; available steps: {:?}",
                    self.step_labels()
                ))
            })
    }

    pub fn platform_responses(&self) -> impl Iterator<Item = &PlatformServiceResponse> {
        self.steps
            .iter()
            .filter_map(|step| step.platform_response.as_ref())
    }

    pub fn viewport_resizes(&self) -> impl Iterator<Item = UiSize> + '_ {
        self.steps.iter().filter_map(|step| step.viewport_resize)
    }

    pub fn require_platform_response(
        &self,
        id: PlatformRequestId,
    ) -> TestResult<&PlatformServiceResponse> {
        self.platform_responses()
            .find(|response| response.id == id)
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "expected platform response `{}`; got {:?}",
                    id.0,
                    self.platform_responses()
                        .map(|response| response.id)
                        .collect::<Vec<_>>()
                ))
            })
    }

    pub fn require_viewport_resize(&self, viewport: UiSize) -> TestResult {
        if self
            .viewport_resizes()
            .any(|candidate| candidate == viewport)
        {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected viewport resize {:?}; got {:?}",
                viewport,
                self.viewport_resizes().collect::<Vec<_>>()
            )))
        }
    }

    pub fn debug_event_routes(&self, document: &UiDocument) -> Vec<DebugEventRouteTrace> {
        self.steps
            .iter()
            .filter_map(|step| {
                step.converted
                    .as_ref()
                    .and_then(|event| DebugEventRouteTrace::from_input_event(document, event))
            })
            .collect()
    }

    pub fn pointer_steps(&self) -> impl Iterator<Item = EventReplayPointerEvent> + '_ {
        self.steps.iter().filter_map(replay_pointer_step)
    }

    pub fn pointer_step(&self, label: &str) -> TestResult<EventReplayPointerEvent> {
        let step = self.step(label)?;
        replay_pointer_step(step).ok_or_else(|| {
            TestFailure::new(format!(
                "event replay step `{label}` is not a pointer event; converted: {:?}",
                step.converted
            ))
        })
    }

    pub fn pointer_events(&self) -> impl Iterator<Item = (DebugPointerRouteKind, UiPoint)> + '_ {
        self.pointer_steps().map(|step| (step.event, step.point))
    }

    pub fn last_pointer_event(&self) -> Option<(DebugPointerRouteKind, UiPoint)> {
        self.pointer_events().last()
    }

    pub fn last_pointer_step(&self) -> Option<EventReplayPointerEvent> {
        self.pointer_steps().last()
    }

    pub fn debug_point_autopsy(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        label: &str,
    ) -> TestResult<DebugPointAutopsyTrace> {
        let step = self.pointer_step(label)?;
        Ok(DebugPointAutopsyTrace::from_pointer_event(
            document,
            text_measurer,
            step.event,
            step.point,
        ))
    }

    pub fn debug_last_point_autopsy(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Option<DebugPointAutopsyTrace> {
        let (event, point) = self.last_pointer_event()?;
        Some(DebugPointAutopsyTrace::from_pointer_event(
            document,
            text_measurer,
            event,
            point,
        ))
    }

    pub fn debug_why_point(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        label: &str,
    ) -> TestResult<DebugWhyTrace> {
        let autopsy = self.debug_point_autopsy(document, text_measurer, label)?;
        Ok(DebugWhyTrace::from_point_autopsy(&autopsy))
    }

    pub fn debug_last_why_point(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Option<DebugWhyTrace> {
        let autopsy = self.debug_last_point_autopsy(document, text_measurer)?;
        Some(DebugWhyTrace::from_point_autopsy(&autopsy))
    }

    pub fn debug_point_autopsies(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Vec<EventReplayPointerDiagnostic<DebugPointAutopsyTrace>> {
        self.pointer_steps()
            .map(|step| {
                let trace = DebugPointAutopsyTrace::from_pointer_event(
                    document,
                    text_measurer,
                    step.event,
                    step.point,
                );
                EventReplayPointerDiagnostic {
                    label: step.label,
                    event: step.event,
                    point: step.point,
                    trace,
                }
            })
            .collect()
    }

    pub fn debug_inspect_point(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
        label: &str,
    ) -> TestResult<DebugInspectPointTrace> {
        let autopsy = self.debug_point_autopsy(document, text_measurer, label)?;
        Ok(DebugInspectPointTrace::from_point_autopsy(autopsy))
    }

    pub fn debug_last_inspect_point(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Option<DebugInspectPointTrace> {
        let autopsy = self.debug_last_point_autopsy(document, text_measurer)?;
        Some(DebugInspectPointTrace::from_point_autopsy(autopsy))
    }

    pub fn debug_inspect_points(
        &self,
        document: &UiDocument,
        text_measurer: &mut impl TextMeasurer,
    ) -> Vec<EventReplayPointerDiagnostic<DebugInspectPointTrace>> {
        self.debug_point_autopsies(document, text_measurer)
            .into_iter()
            .map(|diagnostic| EventReplayPointerDiagnostic {
                label: diagnostic.label,
                event: diagnostic.event,
                point: diagnostic.point,
                trace: DebugInspectPointTrace::from_point_autopsy(diagnostic.trace),
            })
            .collect()
    }

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

    pub fn consumed_nodes(&self) -> Vec<UiNodeId> {
        self.steps
            .iter()
            .filter_map(|step| step.result.as_ref()?.consumed_by)
            .collect()
    }

    pub fn require_clicked(&self, node: UiNodeId) -> TestResult {
        require_replay_node("clicked", node, self.clicked_nodes())
    }

    pub fn require_step_clicked(&self, label: &str, node: UiNodeId) -> TestResult<UiNodeId> {
        let step = self.step(label)?;
        require_replay_step_node(step, ReplayNodeOutcome::Clicked, node)
    }

    pub fn require_clicked_name(
        &self,
        document: &UiDocument,
        node_name: &str,
    ) -> TestResult<UiNodeId> {
        let (expected, _) = LayoutAssertions::new(document).node(node_name)?;
        if let Some(clicked) = self
            .clicked_nodes()
            .into_iter()
            .find(|clicked| document.node_is_descendant_or_self(expected, *clicked))
        {
            Ok(clicked)
        } else {
            Err(TestFailure::new(format_replay_named_node_failure(
                document, self, "clicked", node_name, expected,
            )))
        }
    }

    pub fn require_step_clicked_name(
        &self,
        document: &UiDocument,
        label: &str,
        node_name: &str,
    ) -> TestResult<UiNodeId> {
        let (expected, _) = LayoutAssertions::new(document).node(node_name)?;
        let step = self.step(label)?;
        if let Some(clicked) = ReplayNodeOutcome::Clicked
            .node(step)
            .filter(|clicked| document.node_is_descendant_or_self(expected, *clicked))
        {
            Ok(clicked)
        } else {
            Err(TestFailure::new(format_replay_step_named_node_failure(
                document,
                step,
                ReplayNodeOutcome::Clicked,
                node_name,
                expected,
            )))
        }
    }

    pub fn require_focused(&self, node: UiNodeId) -> TestResult {
        require_replay_node("focused", node, self.focused_nodes())
    }

    pub fn require_step_focused(&self, label: &str, node: UiNodeId) -> TestResult<UiNodeId> {
        let step = self.step(label)?;
        require_replay_step_node(step, ReplayNodeOutcome::Focused, node)
    }

    pub fn require_focused_name(
        &self,
        document: &UiDocument,
        node_name: &str,
    ) -> TestResult<UiNodeId> {
        let (expected, _) = LayoutAssertions::new(document).node(node_name)?;
        if let Some(focused) = self
            .focused_nodes()
            .into_iter()
            .find(|focused| document.node_is_descendant_or_self(expected, *focused))
        {
            Ok(focused)
        } else {
            Err(TestFailure::new(format_replay_named_node_failure(
                document, self, "focused", node_name, expected,
            )))
        }
    }

    pub fn require_step_focused_name(
        &self,
        document: &UiDocument,
        label: &str,
        node_name: &str,
    ) -> TestResult<UiNodeId> {
        let (expected, _) = LayoutAssertions::new(document).node(node_name)?;
        let step = self.step(label)?;
        if let Some(focused) = ReplayNodeOutcome::Focused
            .node(step)
            .filter(|focused| document.node_is_descendant_or_self(expected, *focused))
        {
            Ok(focused)
        } else {
            Err(TestFailure::new(format_replay_step_named_node_failure(
                document,
                step,
                ReplayNodeOutcome::Focused,
                node_name,
                expected,
            )))
        }
    }

    pub fn require_scrolled(&self, node: UiNodeId) -> TestResult {
        require_replay_node("scrolled", node, self.scrolled_nodes())
    }

    pub fn require_consumed_by(&self, node: UiNodeId) -> TestResult {
        require_replay_node("consumed by", node, self.consumed_nodes())
    }

    pub fn require_step_consumed_by(&self, label: &str, node: UiNodeId) -> TestResult {
        let step = self.step(label)?;
        require_replay_step_node(step, ReplayNodeOutcome::ConsumedBy, node).map(|_| ())
    }

    pub fn require_step_consumed_by_name(
        &self,
        document: &UiDocument,
        label: &str,
        node_name: &str,
    ) -> TestResult<UiNodeId> {
        let (expected, _) = LayoutAssertions::new(document).node(node_name)?;
        let step = self.step(label)?;
        if let Some(consumed_by) = ReplayNodeOutcome::ConsumedBy
            .node(step)
            .filter(|consumed_by| document.node_is_descendant_or_self(expected, *consumed_by))
        {
            Ok(consumed_by)
        } else {
            Err(TestFailure::new(format_replay_step_named_node_failure(
                document,
                step,
                ReplayNodeOutcome::ConsumedBy,
                node_name,
                expected,
            )))
        }
    }

    pub fn require_step_count(&self, expected: usize) -> TestResult {
        if self.steps.len() == expected {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected {expected} event replay step(s), got {}",
                self.steps.len()
            )))
        }
    }

    pub fn require_scroll_at_end(&self, document: &UiDocument, node: UiNodeId) -> TestResult {
        let Some(scroll) = document.scroll_state(node) else {
            return Err(TestFailure::new(format!(
                "expected scroll state for node {node:?}"
            )));
        };
        let max = scroll.max_offset();
        let epsilon = 0.5;
        if (scroll.offset.x - max.x).abs() <= epsilon && (scroll.offset.y - max.y).abs() <= epsilon
        {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected scroll node {node:?} at end {max:?}, got {:?}",
                scroll.offset
            )))
        }
    }

    pub fn require_no_clicks(&self) -> TestResult {
        let clicked = self.clicked_nodes();
        if clicked.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected no clicked nodes, got {clicked:?}"
            )))
        }
    }

    pub fn require_no_scrolls(&self) -> TestResult {
        let scrolled = self.scrolled_nodes();
        if scrolled.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected no scrolled nodes, got {scrolled:?}"
            )))
        }
    }

    pub fn require_no_consumption(&self) -> TestResult {
        let consumed = self.consumed_nodes();
        if consumed.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected no consumed events, got {consumed:?}"
            )))
        }
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

    fn step_labels(&self) -> Vec<&str> {
        self.steps.iter().map(|step| step.label.as_str()).collect()
    }
}

fn require_replay_node(kind: &str, node: UiNodeId, actual: Vec<UiNodeId>) -> TestResult {
    if actual.contains(&node) {
        Ok(())
    } else {
        Err(TestFailure::new(format!(
            "expected event replay {kind} node {node:?}, got {actual:?}"
        )))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayNodeOutcome {
    Clicked,
    Focused,
    ConsumedBy,
}

impl ReplayNodeOutcome {
    const fn label(self) -> &'static str {
        match self {
            Self::Clicked => "clicked",
            Self::Focused => "focused",
            Self::ConsumedBy => "consumed by",
        }
    }

    fn node(self, step: &EventReplayStepResult) -> Option<UiNodeId> {
        let result = step.result.as_ref()?;
        match self {
            Self::Clicked => result.clicked,
            Self::Focused => result.focused,
            Self::ConsumedBy => result.consumed.then_some(result.consumed_by).flatten(),
        }
    }
}

fn require_replay_step_node(
    step: &EventReplayStepResult,
    outcome: ReplayNodeOutcome,
    node: UiNodeId,
) -> TestResult<UiNodeId> {
    if outcome.node(step) == Some(node) {
        Ok(node)
    } else {
        Err(TestFailure::new(format!(
            "expected event replay step `{}` to be {} {node:?}, got {:?}",
            step.label,
            outcome.label(),
            step.result
        )))
    }
}

fn format_replay_named_node_failure(
    document: &UiDocument,
    report: &EventReplayReport,
    kind: &str,
    expected_name: &str,
    expected_id: UiNodeId,
) -> String {
    let actual_ids = match kind {
        "clicked" => report.clicked_nodes(),
        "focused" => report.focused_nodes(),
        _ => Vec::new(),
    };
    let actual_names = replay_node_names(document, &actual_ids);
    let mut parts = vec![format!(
        "expected event replay {kind} `{expected_name}`, got [{}]",
        actual_names.join(", ")
    )];
    if let Some((event, point)) = report.last_pointer_event() {
        push_replay_pointer_diagnostics(
            &mut parts,
            document,
            "last pointer",
            event,
            point,
            expected_name,
            expected_id,
        );
    } else {
        parts.push(
            "no pointer event was replayed; inspect event_route_panel for keyboard/focus routing"
                .to_owned(),
        );
    }
    parts.join("; ")
}

fn format_replay_step_named_node_failure(
    document: &UiDocument,
    step: &EventReplayStepResult,
    outcome: ReplayNodeOutcome,
    expected_name: &str,
    expected_id: UiNodeId,
) -> String {
    let actual = outcome
        .node(step)
        .map(|node| replay_node_name(document, node))
        .unwrap_or_else(|| "none".to_owned());
    let mut parts = vec![format!(
        "expected event replay step `{}` {} `{expected_name}`, got {actual}",
        step.label,
        outcome.label()
    )];
    if let Some((event, point)) = step.converted.as_ref().and_then(ui_input_pointer_event) {
        push_replay_pointer_diagnostics(
            &mut parts,
            document,
            "step pointer",
            event,
            point,
            expected_name,
            expected_id,
        );
    } else {
        parts.push(format!(
            "step `{}` is not a pointer event; converted: {:?}",
            step.label, step.converted
        ));
        parts.push("inspect: event_route_panel for keyboard/focus routing".to_owned());
    }
    parts.join("; ")
}

fn push_replay_pointer_diagnostics(
    parts: &mut Vec<String>,
    document: &UiDocument,
    label: &str,
    event: DebugPointerRouteKind,
    point: UiPoint,
    expected_name: &str,
    expected_id: UiNodeId,
) {
    let probe = match event {
        DebugPointerRouteKind::Down => DebugPointerProbe::pointer_down(document, point),
        DebugPointerRouteKind::Move => DebugPointerProbe::pointer_move(document, point),
        DebugPointerRouteKind::Up => DebugPointerProbe::pointer_up(document, point),
        DebugPointerRouteKind::Wheel => {
            DebugPointerProbe::from_pointer_event(document, DebugPointerRouteKind::Wheel, point)
        }
    };
    parts.push(format!(
        "{label} {} at {}",
        event.label(),
        format_test_point(point)
    ));
    parts.push(format!(
        "routed target: {}",
        probe.target_name.as_deref().unwrap_or("none")
    ));
    parts.push(format!(
        "top candidate: {}",
        probe
            .candidates
            .first()
            .map(|candidate| candidate.name.as_str())
            .unwrap_or("none")
    ));
    if let Some(candidate) = probe.candidate(expected_name) {
        parts.push(format!(
            "expected candidate: {}",
            format_hit_candidate_detail(candidate)
        ));
        parts.push(format!("cause: {}", candidate.cause));
        parts.push(format!("fix: {}", candidate.fix));
    } else {
        parts.push(format!(
            "expected node #{} was not a pointer probe candidate",
            expected_id.0
        ));
        parts.push(
            "fix: inspect visibility, hitbox, clip, pointer input, and z/layer order".to_owned(),
        );
    }
    parts.push("inspect: pointer_autopsy_panel or hitbox_debug_overlay".to_owned());
}

fn replay_node_names(document: &UiDocument, nodes: &[UiNodeId]) -> Vec<String> {
    if nodes.is_empty() {
        return vec!["none".to_owned()];
    }
    nodes
        .iter()
        .map(|node| replay_node_name(document, *node))
        .collect()
}

fn replay_node_name(document: &UiDocument, node: UiNodeId) -> String {
    document
        .nodes()
        .get(node.0)
        .map(|node| node.name.clone())
        .unwrap_or_else(|| format!("#{}", node.0))
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayListReuseAssertions<'a> {
    report: &'a DisplayListReuseReport,
}

impl<'a> DisplayListReuseAssertions<'a> {
    pub const fn new(report: &'a DisplayListReuseReport) -> Self {
        Self { report }
    }

    pub const fn report(&self) -> &'a DisplayListReuseReport {
        self.report
    }

    pub fn require_outcome(&self, expected: DisplayListReuseOutcome) -> TestResult {
        if self.report.outcome == expected {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "display-list `{}` expected reuse outcome {expected:?}, got {:?}",
                self.report.key.id.as_str(),
                self.report.outcome
            )))
        }
    }

    pub fn require_reused(&self) -> TestResult {
        self.require_outcome(DisplayListReuseOutcome::Reused)
    }

    pub fn require_miss_absent(&self) -> TestResult {
        self.require_outcome(DisplayListReuseOutcome::MissAbsent)
    }

    pub fn require_miss_dirty(&self) -> TestResult {
        self.require_outcome(DisplayListReuseOutcome::MissDirty)
    }

    pub fn require_miss_evicted(&self) -> TestResult {
        self.require_outcome(DisplayListReuseOutcome::MissEvicted)
    }

    pub fn require_item_count(&self, expected: usize) -> TestResult {
        if self.report.item_count == Some(expected) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "display-list `{}` expected {expected} retained paint item(s), got {:?}",
                self.report.key.id.as_str(),
                self.report.item_count
            )))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayListInvalidationAssertions<'a> {
    report: &'a DisplayListInvalidationReport,
}

impl<'a> DisplayListInvalidationAssertions<'a> {
    pub const fn new(report: &'a DisplayListInvalidationReport) -> Self {
        Self { report }
    }

    pub const fn report(&self) -> &'a DisplayListInvalidationReport {
        self.report
    }

    pub fn require_removed_count(&self, expected: usize) -> TestResult {
        let actual = self.report.removed_count();
        if actual == expected {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "display-list invalidation expected {expected} removed entry/entries, got {actual}"
            )))
        }
    }

    pub fn require_removed_key(&self, key: &DisplayListKey) -> TestResult {
        if self.report.removed_key(key) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "display-list invalidation did not remove key `{}`; removed keys: {:?}",
                key.id.as_str(),
                self.report.removed_keys
            )))
        }
    }

    pub fn require_after_len(&self, expected: usize) -> TestResult {
        if self.report.after_len == expected {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "display-list invalidation expected cache length {expected}, got {}",
                self.report.after_len
            )))
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DisplayListReuseSeries {
    name: String,
    reports: Vec<DisplayListReuseReport>,
}

impl DisplayListReuseSeries {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            reports: Vec::new(),
        }
    }

    pub fn report(mut self, report: DisplayListReuseReport) -> Self {
        self.push(report);
        self
    }

    pub fn push(&mut self, report: DisplayListReuseReport) {
        self.reports.push(report);
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn reports(&self) -> &[DisplayListReuseReport] {
        &self.reports
    }

    pub fn len(&self) -> usize {
        self.reports.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reports.is_empty()
    }

    pub fn outcome_count(&self, outcome: DisplayListReuseOutcome) -> usize {
        self.reports
            .iter()
            .filter(|report| report.outcome == outcome)
            .count()
    }

    pub fn reused_count(&self) -> usize {
        self.outcome_count(DisplayListReuseOutcome::Reused)
    }

    pub fn missed_count(&self) -> usize {
        self.reports.iter().filter(|report| report.missed()).count()
    }

    pub fn reuse_rate(&self) -> Option<f64> {
        (!self.reports.is_empty()).then(|| self.reused_count() as f64 / self.reports.len() as f64)
    }

    pub fn reports_for_key<'a>(
        &'a self,
        key: &'a DisplayListKey,
    ) -> impl Iterator<Item = &'a DisplayListReuseReport> + 'a {
        self.reports.iter().filter(move |report| &report.key == key)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayListReuseSeriesAssertions<'a> {
    series: &'a DisplayListReuseSeries,
}

impl<'a> DisplayListReuseSeriesAssertions<'a> {
    pub const fn new(series: &'a DisplayListReuseSeries) -> Self {
        Self { series }
    }

    pub const fn series(&self) -> &'a DisplayListReuseSeries {
        self.series
    }

    pub fn require_report_count(&self, expected_count: usize) -> TestResult {
        let actual = self.series.len();
        if actual == expected_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "{} expected {expected_count} display-list reuse report(s), got {actual}",
                self.series.name()
            )))
        }
    }

    pub fn require_outcome_count(
        &self,
        outcome: DisplayListReuseOutcome,
        expected_count: usize,
    ) -> TestResult {
        let actual = self.series.outcome_count(outcome);
        if actual == expected_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "{} expected {expected_count} {outcome:?} display-list reuse outcome(s), got {actual}",
                self.series.name()
            )))
        }
    }

    pub fn require_min_reused(&self, minimum_count: usize) -> TestResult {
        let actual = self.series.reused_count();
        if actual >= minimum_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "{} expected at least {minimum_count} reused display-list report(s), got {actual}",
                self.series.name()
            )))
        }
    }

    pub fn require_no_evictions(&self) -> TestResult {
        self.require_outcome_count(DisplayListReuseOutcome::MissEvicted, 0)
    }

    pub fn require_reuse_rate_at_least(&self, minimum_rate: f64) -> TestResult<f64> {
        let actual = self.series.reuse_rate().ok_or_else(|| {
            TestFailure::new(format!(
                "{} has no display-list reuse reports",
                self.series.name()
            ))
        })?;
        if actual >= minimum_rate {
            Ok(actual)
        } else {
            Err(TestFailure::new(format!(
                "{} expected display-list reuse rate at least {minimum_rate:.2}, got {actual:.2}",
                self.series.name()
            )))
        }
    }

    pub fn require_key_outcome_count(
        &self,
        key: &DisplayListKey,
        outcome: DisplayListReuseOutcome,
        expected_count: usize,
    ) -> TestResult {
        let actual = self
            .series
            .reports_for_key(key)
            .filter(|report| report.outcome == outcome)
            .count();
        if actual == expected_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "{} expected key `{}` to have {expected_count} {outcome:?} outcome(s), got {actual}",
                self.series.name(),
                key.id.as_str()
            )))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandReplayStepResult {
    pub label: String,
    pub input: ReplayInput,
    pub converted: Option<UiInputEvent>,
    pub result: Option<UiInputResult>,
    pub platform_response: Option<PlatformServiceResponse>,
    pub viewport_resize: Option<UiSize>,
    pub shortcut_route: Option<HostShortcutRoute>,
    pub dispatch: Option<HostCommandDispatch>,
    pub direct_command: Option<CommandId>,
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

    pub fn direct_commands(&self) -> impl Iterator<Item = &CommandId> {
        self.steps
            .iter()
            .filter_map(|step| step.direct_command.as_ref())
    }

    pub fn platform_responses(&self) -> impl Iterator<Item = &PlatformServiceResponse> {
        self.steps
            .iter()
            .filter_map(|step| step.platform_response.as_ref())
    }

    pub fn viewport_resizes(&self) -> impl Iterator<Item = UiSize> + '_ {
        self.steps.iter().filter_map(|step| step.viewport_resize)
    }

    pub fn dispatched_commands(&self) -> Vec<CommandId> {
        self.dispatches()
            .map(|dispatch| dispatch.command.clone())
            .chain(self.direct_commands().cloned())
            .collect()
    }

    pub fn require_command_dispatched(&self, command: impl Into<CommandId>) -> TestResult {
        let command = command.into();
        if self
            .dispatches()
            .any(|dispatch| dispatch.command == command)
            || self
                .direct_commands()
                .any(|direct_command| *direct_command == command)
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
        } else if let Some(command) = self.direct_commands().next() {
            Err(TestFailure::new(format!(
                "expected no command dispatches, got direct `{command}`"
            )))
        } else {
            Ok(())
        }
    }
}

pub struct UiStateMatrixCase<'a> {
    pub label: String,
    build: Box<dyn Fn(UiSize) -> TestResult<UiStateMatrixDocument> + 'a>,
}

impl<'a> UiStateMatrixCase<'a> {
    pub fn new(
        label: impl Into<String>,
        build: impl Fn(UiSize) -> TestResult<UiStateMatrixDocument> + 'a,
    ) -> Self {
        Self {
            label: label.into(),
            build: Box::new(build),
        }
    }
}

impl fmt::Debug for UiStateMatrixCase<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UiStateMatrixCase")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiStateMatrixViewport {
    pub label: String,
    pub size: UiSize,
}

impl UiStateMatrixViewport {
    pub fn new(label: impl Into<String>, size: UiSize) -> Self {
        Self {
            label: label.into(),
            size,
        }
    }
}

#[derive(Debug)]
pub struct UiStateMatrixDocument {
    pub document: UiDocument,
    pub targets: Vec<UiStateMatrixTarget>,
}

impl UiStateMatrixDocument {
    pub fn new(
        document: UiDocument,
        targets: impl IntoIterator<Item = UiStateMatrixTarget>,
    ) -> Self {
        Self {
            document,
            targets: targets.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiStateMatrixTarget {
    pub label: String,
    pub node: UiNodeId,
    pub interaction: UiStateMatrixInteraction,
}

impl UiStateMatrixTarget {
    pub fn layout(label: impl Into<String>, node: UiNodeId) -> Self {
        Self {
            label: label.into(),
            node,
            interaction: UiStateMatrixInteraction::None,
        }
    }

    pub fn pointer_click(label: impl Into<String>, node: UiNodeId) -> Self {
        Self {
            label: label.into(),
            node,
            interaction: UiStateMatrixInteraction::PointerClick,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UiStateMatrixInteraction {
    #[default]
    None,
    PointerClick,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiStateMatrixReport {
    pub viewports: usize,
    pub cases: usize,
    pub documents: usize,
    pub targets: usize,
    pub pointer_interactions: usize,
}

impl UiStateMatrixReport {
    fn record_document(&mut self, target_count: usize) {
        self.documents += 1;
        self.targets += target_count;
    }

    fn record_pointer_interaction(&mut self) {
        self.pointer_interactions += 1;
    }
}

pub fn run_ui_state_matrix<'a>(
    viewports: impl IntoIterator<Item = UiStateMatrixViewport>,
    cases: impl IntoIterator<Item = UiStateMatrixCase<'a>>,
) -> TestResult<UiStateMatrixReport> {
    let viewports = viewports.into_iter().collect::<Vec<_>>();
    let cases = cases.into_iter().collect::<Vec<_>>();
    let mut report = UiStateMatrixReport {
        viewports: viewports.len(),
        cases: cases.len(),
        ..UiStateMatrixReport::default()
    };
    for case in &cases {
        for viewport in &viewports {
            let mut matrix_document = (case.build)(viewport.size).map_err(|error| {
                TestFailure::new(format!(
                    "state matrix `{}` at viewport `{}` failed to build: {error}",
                    case.label, viewport.label
                ))
            })?;
            matrix_document
                .document
                .compute_layout(viewport.size, &mut ApproxTextMeasurer)
                .map_err(|error| {
                    TestFailure::new(format!(
                        "state matrix `{}` at viewport `{}` failed to layout: {error}",
                        case.label, viewport.label
                    ))
                })?;
            JustWorkAssertions::new(&matrix_document.document)
                .require_clean()
                .map_err(|error| {
                    TestFailure::new(format!(
                        "state matrix `{}` at viewport `{}` ({:?}) had blocking audit warnings: {}",
                        case.label, viewport.label, viewport.size, error
                    ))
                })?;
            for target in &matrix_document.targets {
                require_state_matrix_target_geometry(
                    &case.label,
                    viewport,
                    &matrix_document.document,
                    target,
                )?;
                match target.interaction {
                    UiStateMatrixInteraction::None => {}
                    UiStateMatrixInteraction::PointerClick => {
                        require_state_matrix_pointer_click(
                            &case.label,
                            viewport,
                            &mut matrix_document.document,
                            target,
                        )?;
                        report.record_pointer_interaction();
                    }
                }
            }
            report.record_document(matrix_document.targets.len());
        }
    }
    Ok(report)
}

pub fn is_blocking_audit_warning(warning: &AuditWarning) -> bool {
    matches!(
        warning,
        AuditWarning::NonFiniteRect { .. }
            | AuditWarning::InvisibleInteractiveNode { .. }
            | AuditWarning::EmptyInteractiveClip { .. }
            | AuditWarning::InteractiveTooSmall { .. }
            | AuditWarning::DuplicateNodeName { .. }
            | AuditWarning::TextClipped { .. }
            | AuditWarning::ScrollRangeHidden { .. }
            | AuditWarning::ScrollOffsetOutOfRange { .. }
            | AuditWarning::ScrollbarVisibleWithoutRange { .. }
            | AuditWarning::NodeOutsideRoot { .. }
            | AuditWarning::PaintItemEmptyClip { .. }
    )
}

fn require_state_matrix_target_geometry(
    case_label: &str,
    viewport: &UiStateMatrixViewport,
    document: &UiDocument,
    target: &UiStateMatrixTarget,
) -> TestResult {
    let node = document.nodes().get(target.node.0).ok_or_else(|| {
        TestFailure::new(format!(
            "state matrix `{case_label}` at viewport `{}` target `{}` references missing node {:?}",
            viewport.label, target.label, target.node
        ))
    })?;
    if !node.layout.visible {
        return Err(TestFailure::new(format!(
            "state matrix `{case_label}` at viewport `{}` target `{}` is not visible",
            viewport.label, target.label
        )));
    }
    if !state_matrix_rect_is_finite(node.layout.rect)
        || !state_matrix_rect_is_finite(node.layout.clip_rect)
    {
        return Err(TestFailure::new(format!(
            "state matrix `{case_label}` at viewport `{}` target `{}` has non-finite geometry rect={:?} clip={:?}",
            viewport.label, target.label, node.layout.rect, node.layout.clip_rect
        )));
    }
    let clipped_rect = state_matrix_clipped_rect(node.layout.rect, node.layout.clip_rect);
    if clipped_rect.width <= f32::EPSILON || clipped_rect.height <= f32::EPSILON {
        return Err(TestFailure::new(format!(
            "state matrix `{case_label}` at viewport `{}` target `{}` has an empty effective rect: rect={:?} clip={:?}",
            viewport.label, target.label, node.layout.rect, node.layout.clip_rect
        )));
    }
    Ok(())
}

fn require_state_matrix_pointer_click(
    case_label: &str,
    viewport: &UiStateMatrixViewport,
    document: &mut UiDocument,
    target: &UiStateMatrixTarget,
) -> TestResult {
    let node = document.nodes().get(target.node.0).ok_or_else(|| {
        TestFailure::new(format!(
            "state matrix `{case_label}` at viewport `{}` target `{}` references missing node {:?}",
            viewport.label, target.label, target.node
        ))
    })?;
    let clipped_rect = state_matrix_clipped_rect(node.layout.rect, node.layout.clip_rect);
    let point = state_matrix_rect_center(clipped_rect);
    let hit = document.hit_test(point).ok_or_else(|| {
        TestFailure::new(format!(
            "state matrix `{case_label}` at viewport `{}` target `{}` was not hit-testable at {:?}",
            viewport.label, target.label, point
        ))
    })?;
    if !document.node_is_descendant_or_self(target.node, hit) {
        return Err(TestFailure::new(format!(
            "state matrix `{case_label}` at viewport `{}` target `{}` expected hit under {:?}, got {:?}",
            viewport.label, target.label, target.node, hit
        )));
    }

    document.handle_input(UiInputEvent::PointerMove(point));
    document.handle_input(UiInputEvent::PointerDown(point));
    let result = document.handle_input(UiInputEvent::PointerUp(point));
    let Some(clicked) = result.clicked else {
        return Err(TestFailure::new(format!(
            "state matrix `{case_label}` at viewport `{}` target `{}` did not produce a click at {:?}",
            viewport.label, target.label, point
        )));
    };
    if document.node_is_descendant_or_self(target.node, clicked) {
        Ok(())
    } else {
        Err(TestFailure::new(format!(
            "state matrix `{case_label}` at viewport `{}` target `{}` expected click under {:?}, got {:?}",
            viewport.label, target.label, target.node, clicked
        )))
    }
}

fn state_matrix_rect_is_finite(rect: UiRect) -> bool {
    rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite()
}

fn state_matrix_clipped_rect(rect: UiRect, clip_rect: UiRect) -> UiRect {
    rect.intersection(clip_rect)
        .unwrap_or(UiRect::new(0.0, 0.0, 0.0, 0.0))
}

fn state_matrix_rect_center(rect: UiRect) -> UiPoint {
    UiPoint::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
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
        let mut measurer = ApproxTextMeasurer;
        self.require_visible_with_measurer(name, &mut measurer)
    }

    pub fn visibility_trace(&self, text_measurer: &mut impl TextMeasurer) -> DebugVisibilityTrace {
        DebugVisibilityTrace::from_document(self.document, text_measurer)
    }

    pub fn require_visible_with_measurer(
        &self,
        name: &str,
        text_measurer: &mut impl TextMeasurer,
    ) -> TestResult {
        let (_, node) = self.node(name)?;
        if node.layout.visible {
            Ok(())
        } else {
            let visibility = self.visibility_trace(text_measurer);
            Err(TestFailure::new(format_visibility_assertion_failure(
                name,
                node,
                visibility.record(name),
            )))
        }
    }

    pub fn hit_probe(&self, point: UiPoint) -> DebugPointerProbe {
        DebugPointerProbe::pointer_down(self.document, point)
    }

    pub fn require_hit_at(&self, name: &str, point: UiPoint) -> TestResult<UiNodeId> {
        let (expected, _) = self.node(name)?;
        let hit = self.document.hit_test(point);
        if let Some(hit) =
            hit.filter(|hit| self.document.node_is_descendant_or_self(expected, *hit))
        {
            Ok(hit)
        } else {
            let probe = self.hit_probe(point);
            Err(TestFailure::new(format_hit_assertion_failure(
                self.document,
                name,
                expected,
                point,
                hit,
                &probe,
            )))
        }
    }

    pub fn require_hit_center(&self, name: &str) -> TestResult<UiNodeId> {
        let rect = self.rect(name)?;
        self.require_hit_at(name, test_rect_center(rect))
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
pub struct AuditAssertions<'a> {
    document: &'a UiDocument,
    warnings: Vec<AuditWarning>,
}

impl<'a> AuditAssertions<'a> {
    pub fn new(document: &'a UiDocument) -> Self {
        Self {
            document,
            warnings: document.audit_layout(),
        }
    }

    pub fn warnings(&self) -> &[AuditWarning] {
        &self.warnings
    }

    pub fn require_no_warnings(&self) -> TestResult {
        if self.warnings.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected no audit warnings, got {:?}",
                self.warnings
            )))
        }
    }

    pub fn require_no_accessibility_warnings(&self) -> TestResult {
        let warnings = self
            .warnings
            .iter()
            .filter(|warning| is_accessibility_audit_warning(warning))
            .collect::<Vec<_>>();
        if warnings.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected no accessibility audit warnings, got {warnings:?}"
            )))
        }
    }

    pub fn require_accessibility_metadata_gap(&self, name: &str) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(
                warning,
                AuditWarning::InteractiveAccessibilityMissing { .. }
            )
        })
    }

    pub fn require_no_accessibility_metadata_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(
                warning,
                AuditWarning::InteractiveAccessibilityMissing { .. }
            )
        })
    }

    pub fn require_accessible_name_gap(&self, name: &str) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibleNameMissing { .. })
        })
    }

    pub fn require_no_accessible_name_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibleNameMissing { .. })
        })
    }

    pub fn require_accessibility_action_gap(&self, name: &str) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityActionMissing { .. })
        })
    }

    pub fn require_no_accessibility_action_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityActionMissing { .. })
        })
    }

    pub fn require_accessibility_action_id_gap(&self, name: &str) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityActionIdMissing { .. })
        })
    }

    pub fn require_no_accessibility_action_id_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityActionIdMissing { .. })
        })
    }

    pub fn require_accessibility_action_label_gap(&self, name: &str) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(
                warning,
                AuditWarning::AccessibilityActionLabelMissing { .. }
            )
        })
    }

    pub fn require_no_accessibility_action_label_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(
                warning,
                AuditWarning::AccessibilityActionLabelMissing { .. }
            )
        })
    }

    pub fn require_accessibility_action_duplicate_gap(
        &self,
        name: &str,
        action_id: &str,
    ) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(
                warning,
                AuditWarning::AccessibilityActionDuplicate {
                    action_id: actual,
                    ..
                } if actual == action_id
            )
        })
    }

    pub fn require_no_accessibility_action_duplicate_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityActionDuplicate { .. })
        })
    }

    pub fn require_accessibility_state_gap(
        &self,
        name: &str,
        state: AccessibilityStateKind,
    ) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(
                warning,
                AuditWarning::AccessibilityStateMissing { state: actual, .. } if *actual == state
            )
        })
    }

    pub fn require_no_accessibility_state_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityStateMissing { .. })
        })
    }

    pub fn require_accessibility_value_gap(&self, name: &str) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityValueMissing { .. })
        })
    }

    pub fn require_no_accessibility_value_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityValueMissing { .. })
        })
    }

    pub fn require_accessibility_value_range_gap(&self, name: &str) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityValueRangeMissing { .. })
        })
    }

    pub fn require_no_accessibility_value_range_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityValueRangeMissing { .. })
        })
    }

    pub fn require_accessibility_value_range_invalid_gap(
        &self,
        name: &str,
        issue: AccessibilityValueRangeIssue,
    ) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(
                warning,
                AuditWarning::AccessibilityValueRangeInvalid { issue: actual, .. } if *actual == issue
            )
        })
    }

    pub fn require_no_accessibility_value_range_invalid_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::AccessibilityValueRangeInvalid { .. })
        })
    }

    pub fn require_relation_target_gap(
        &self,
        name: &str,
        relation: AccessibilityRelationKind,
        target_name: &str,
    ) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        let target = self.node_id(target_name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(
                warning,
                AuditWarning::AccessibilityRelationTargetMissing {
                    relation: actual,
                    target: actual_target,
                    ..
                } if *actual == relation && *actual_target == target
            )
        })
    }

    pub fn require_no_relation_target_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(
                warning,
                AuditWarning::AccessibilityRelationTargetMissing { .. }
            )
        })
    }

    pub fn require_text_contrast_gap(&self, name: &str) -> TestResult<&AuditWarning> {
        let node = self.node_id(name)?;
        self.require_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::TextContrastTooLow { .. })
        })
    }

    pub fn require_no_text_contrast_gap(&self, name: &str) -> TestResult {
        let node = self.node_id(name)?;
        self.require_no_warning_for(name, node, |warning| {
            matches!(warning, AuditWarning::TextContrastTooLow { .. })
        })
    }

    fn node_id(&self, name: &str) -> TestResult<UiNodeId> {
        LayoutAssertions::new(self.document)
            .node(name)
            .map(|(id, _)| id)
    }

    fn require_warning_for(
        &self,
        name: &str,
        node: UiNodeId,
        mut predicate: impl FnMut(&AuditWarning) -> bool,
    ) -> TestResult<&AuditWarning> {
        self.warnings
            .iter()
            .find(|warning| warning_node(warning) == Some(node) && predicate(warning))
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing expected audit warning for node `{name}`; got {:?}",
                    self.warnings
                ))
            })
    }

    fn require_no_warning_for(
        &self,
        name: &str,
        node: UiNodeId,
        mut predicate: impl FnMut(&AuditWarning) -> bool,
    ) -> TestResult {
        if let Some(warning) = self
            .warnings
            .iter()
            .find(|warning| warning_node(warning) == Some(node) && predicate(warning))
        {
            Err(TestFailure::new(format!(
                "node `{name}` had unexpected audit warning {warning:?}"
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct JustWorkAssertions<'a> {
    document: &'a UiDocument,
    warnings: Vec<AuditWarning>,
}

impl<'a> JustWorkAssertions<'a> {
    pub fn new(document: &'a UiDocument) -> Self {
        Self {
            document,
            warnings: document.audit_layout(),
        }
    }

    pub const fn document(&self) -> &'a UiDocument {
        self.document
    }

    pub fn warnings(&self) -> &[AuditWarning] {
        &self.warnings
    }

    pub fn blocking_warnings(&self) -> Vec<&AuditWarning> {
        self.warnings
            .iter()
            .filter(|warning| is_blocking_audit_warning(warning))
            .collect()
    }

    pub fn require_clean(&self) -> TestResult {
        let warnings = self.blocking_warnings();
        if warnings.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected no blocking Just Work audit warnings:\n{}",
                format_audit_warning_summaries(warnings)
            )))
        }
    }

    pub fn require_no_text_clipping(&self) -> TestResult {
        self.require_no_blocking_kind("text clipping", |warning| {
            matches!(warning, AuditWarning::TextClipped { .. })
        })
    }

    pub fn require_no_scroll_failures(&self) -> TestResult {
        self.require_no_blocking_kind("scroll range", |warning| {
            matches!(
                warning,
                AuditWarning::ScrollRangeHidden { .. }
                    | AuditWarning::ScrollOffsetOutOfRange { .. }
                    | AuditWarning::ScrollbarVisibleWithoutRange { .. }
            )
        })
    }

    pub fn require_no_geometry_failures(&self) -> TestResult {
        self.require_no_blocking_kind("geometry", |warning| {
            matches!(
                warning,
                AuditWarning::NonFiniteRect { .. }
                    | AuditWarning::InvisibleInteractiveNode { .. }
                    | AuditWarning::EmptyInteractiveClip { .. }
                    | AuditWarning::InteractiveTooSmall { .. }
                    | AuditWarning::NodeOutsideRoot { .. }
                    | AuditWarning::PaintItemEmptyClip { .. }
            )
        })
    }

    pub fn overlap_report(&self) -> DebugOverlapReport {
        DebugOverlapReport::from_document(self.document)
    }

    pub fn require_no_interactive_overlaps(&self) -> TestResult {
        let report = self.overlap_report();
        let overlaps = report
            .overlaps
            .iter()
            .filter(|overlap| overlap.kind == DebugOverlapKind::InteractiveHitbox)
            .collect::<Vec<_>>();
        if overlaps.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected no interactive hitbox overlaps:\n{}",
                format_interactive_overlap_summaries(overlaps)
            )))
        }
    }

    pub fn paint_hit_mismatch_trace(
        &self,
        text_measurer: &mut impl TextMeasurer,
    ) -> DebugPaintHitMismatchTrace {
        DebugPaintHitMismatchTrace::from_document(self.document, text_measurer)
    }

    pub fn require_no_paint_hit_mismatches(
        &self,
        text_measurer: &mut impl TextMeasurer,
    ) -> TestResult {
        let trace = self.paint_hit_mismatch_trace(text_measurer);
        let mismatches = trace
            .records
            .iter()
            .filter(|record| {
                matches!(
                    record.status,
                    DebugFrameTraceStatus::Error | DebugFrameTraceStatus::Warning
                )
            })
            .collect::<Vec<_>>();
        if mismatches.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected no paint/hit mismatches:\n{}",
                format_paint_hit_mismatch_summaries(mismatches)
            )))
        }
    }

    fn require_no_blocking_kind(
        &self,
        label: &str,
        mut predicate: impl FnMut(&AuditWarning) -> bool,
    ) -> TestResult {
        let warnings = self
            .warnings
            .iter()
            .filter(|warning| is_blocking_audit_warning(warning) && predicate(warning))
            .collect::<Vec<_>>();
        if warnings.is_empty() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected no blocking Just Work {label} warnings:\n{}",
                format_audit_warning_summaries(warnings)
            )))
        }
    }
}

fn format_visibility_assertion_failure(
    name: &str,
    node: &UiNode,
    record: Option<&DebugVisibilityRecord>,
) -> String {
    if let Some(record) = record {
        return format!(
            "node `{name}` is not visible; issue: {}; cause: {}; fix: {}; rect: {}; clip: {}; inspect: visibility_panel",
            record.issue.title(),
            record.cause,
            record.fix,
            format_test_rect(record.rect),
            format_test_rect(record.clip_rect)
        );
    }

    format!(
        "node `{name}` is not visible; rect: {}; clip: {}; inspect: visibility_panel",
        format_test_rect(node.layout.rect),
        format_test_rect(node.layout.clip_rect)
    )
}

fn format_paint_assertion_failure(
    name: &str,
    node: &UiNode,
    record: Option<&DebugVisibilityRecord>,
) -> String {
    if let Some(record) = record {
        return format!(
            "node `{name}` has no paint items; issue: {}; cause: {}; fix: {}; rect: {}; clip: {}; inspect: visibility_panel",
            record.issue.title(),
            record.cause,
            record.fix,
            format_test_rect(record.rect),
            format_test_rect(record.clip_rect)
        );
    }

    format!(
        "node `{name}` has no paint items; visible: {}; rect: {}; clip: {}; inspect: visibility_panel or paint_order_panel",
        node.layout.visible,
        format_test_rect(node.layout.rect),
        format_test_rect(node.layout.clip_rect)
    )
}

fn format_hit_assertion_failure(
    document: &UiDocument,
    expected_name: &str,
    expected_id: UiNodeId,
    point: UiPoint,
    actual_hit: Option<UiNodeId>,
    probe: &DebugPointerProbe,
) -> String {
    let actual = actual_hit
        .map(|hit| document.node(hit).name.as_str())
        .unwrap_or("none");
    let candidate = probe.candidate(expected_name);
    let cause = candidate
        .map(|candidate| candidate.cause.as_str())
        .unwrap_or_else(|| {
            probe
                .candidates
                .first()
                .map(|candidate| candidate.cause.as_str())
                .unwrap_or("no retained layout, paint, or hitbox geometry was close to this point")
        });
    let fix = candidate
        .map(|candidate| candidate.fix.as_str())
        .unwrap_or_else(|| {
            probe
                .candidates
                .first()
                .map(|candidate| candidate.fix.as_str())
                .unwrap_or(
                    "confirm the point is in the expected viewport and the target has layout size",
                )
        });
    let target = probe.target_name.as_deref().unwrap_or("none");
    let top = probe
        .candidates
        .first()
        .map(|candidate| candidate.name.as_str())
        .unwrap_or("none");
    let expected_detail = candidate
        .map(format_hit_candidate_detail)
        .unwrap_or_else(|| {
            format!(
                "expected node #{} was not a pointer probe candidate",
                expected_id.0
            )
        });

    format!(
        "expected point {} to hit `{expected_name}`, got `{actual}`; routed target: {target}; top candidate: {top}; expected candidate: {expected_detail}; cause: {cause}; fix: {fix}; inspect: pointer_autopsy_panel or hitbox_debug_overlay",
        format_test_point(point),
    )
}

fn format_hit_candidate_detail(candidate: &DebugPointerProbeCandidate) -> String {
    let mut parts = vec![
        format!("hit={}", candidate.hit),
        format!("target={}", candidate.target),
        format!("layout={}", candidate.layout_contains_point),
        format!("clip={}", candidate.clip_contains_point),
        format!("effective={}", candidate.effective_hit),
        format!("pointer={}", candidate.pointer),
    ];
    if let Some(hit_rect) = candidate.hit_rect {
        parts.push(format!("hit rect {}", format_test_rect(hit_rect)));
    }
    if !candidate.rejections.is_empty() {
        parts.push(format!("{} route rejection(s)", candidate.rejections.len()));
    }
    parts.join(", ")
}

fn format_interactive_overlap_summaries(overlaps: Vec<&DebugOverlapRecord>) -> String {
    overlaps
        .into_iter()
        .enumerate()
        .map(|(index, overlap)| {
            let reasons = if overlap.reasons.is_empty() {
                "no recorded reason".to_owned()
            } else {
                overlap
                    .reasons
                    .iter()
                    .map(debug_overlap_reason_summary)
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!(
                "{}. `{}` overlaps `{}` at {} ({:.0}px2); reasons: {}; inspect: overlap_report_panel",
                index + 1,
                overlap.back.name,
                overlap.front.name,
                format_test_rect(overlap.overlap_rect),
                test_rect_area(overlap.overlap_rect),
                reasons
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_paint_hit_mismatch_summaries(mismatches: Vec<&DebugPaintHitMismatchRecord>) -> String {
    mismatches
        .into_iter()
        .enumerate()
        .map(|(index, mismatch)| {
            format!(
                "{}. `{}` {}: {}; cause: {}; fix: {}",
                index + 1,
                mismatch.name,
                mismatch.kind.title(),
                mismatch.summary,
                mismatch.cause,
                mismatch.fix
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn debug_overlap_reason_summary(reason: &DebugOverlapReason) -> String {
    match reason {
        DebugOverlapReason::SharedParent { .. } => "shared parent layout".to_owned(),
        DebugOverlapReason::DifferentParents { .. } => "different parent chains".to_owned(),
        DebugOverlapReason::AncestorDescendant { .. } => "ancestor/descendant geometry".to_owned(),
        DebugOverlapReason::AbsolutePositioning { .. } => "absolute positioning".to_owned(),
        DebugOverlapReason::LayerOrdering { .. } => "layer ordering".to_owned(),
        DebugOverlapReason::ZOrdering { .. } => "z ordering".to_owned(),
        DebugOverlapReason::BothPointerInteractive => {
            "both nodes are pointer-interactive".to_owned()
        }
        DebugOverlapReason::PaintExtendsOutsideLayout { .. } => {
            "paint extends outside layout".to_owned()
        }
        DebugOverlapReason::ClipReducesVisibleHitbox { .. } => {
            "clip reduces visible hitbox".to_owned()
        }
    }
}

fn format_test_rect(rect: UiRect) -> String {
    format!(
        "x={:.1} y={:.1} w={:.1} h={:.1}",
        rect.x, rect.y, rect.width, rect.height
    )
}

fn format_test_point(point: UiPoint) -> String {
    format!("x={:.1} y={:.1}", point.x, point.y)
}

fn test_rect_center(rect: UiRect) -> UiPoint {
    UiPoint::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

fn test_rect_area(rect: UiRect) -> f32 {
    rect.width.max(0.0) * rect.height.max(0.0)
}

fn format_audit_warning_summaries(warnings: Vec<&AuditWarning>) -> String {
    warnings
        .into_iter()
        .enumerate()
        .map(|(index, warning)| format!("{}. {}", index + 1, warning.diagnostic_summary()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_accessibility_audit_warning(warning: &AuditWarning) -> bool {
    matches!(
        warning,
        AuditWarning::AccessibleNameMissing { .. }
            | AuditWarning::InteractiveAccessibilityMissing { .. }
            | AuditWarning::AccessibilityActionMissing { .. }
            | AuditWarning::AccessibilityActionIdMissing { .. }
            | AuditWarning::AccessibilityActionLabelMissing { .. }
            | AuditWarning::AccessibilityActionDuplicate { .. }
            | AuditWarning::AccessibilityStateMissing { .. }
            | AuditWarning::AccessibilityValueMissing { .. }
            | AuditWarning::AccessibilityValueRangeMissing { .. }
            | AuditWarning::AccessibilityValueRangeInvalid { .. }
            | AuditWarning::AccessibilityRelationTargetMissing { .. }
            | AuditWarning::TextContrastTooLow { .. }
            | AuditWarning::FocusableMissingFromAccessibilityTree { .. }
    )
}

fn warning_node(warning: &AuditWarning) -> Option<UiNodeId> {
    warning.node()
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

    pub fn require_action(&self, name: &str, action_id: &str, label: &str) -> TestResult {
        let node = self.node(name)?;
        if node
            .actions
            .iter()
            .any(|action| action.id == action_id && action.label == label)
        {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected accessibility action `{action_id}` with label `{label}`, got {:?}",
                node.actions
            )))
        }
    }

    pub fn require_action_shortcut(
        &self,
        name: &str,
        action_id: &str,
        shortcut: &str,
    ) -> TestResult {
        let node = self.node(name)?;
        if node
            .actions
            .iter()
            .any(|action| action.id == action_id && action.shortcut.as_deref() == Some(shortcut))
        {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected accessibility action `{action_id}` shortcut `{shortcut}`, got {:?}",
                node.actions
            )))
        }
    }

    pub fn require_key_shortcut(&self, name: &str, shortcut: &str) -> TestResult {
        let node = self.node(name)?;
        if node.key_shortcuts.iter().any(|actual| actual == shortcut) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected accessibility key shortcut `{shortcut}`, got {:?}",
                node.key_shortcuts
            )))
        }
    }

    pub fn require_accessible_name(&self, name: &str, expected: &str) -> TestResult {
        let node = self.node(name)?;
        let actual = self.tree.accessible_name(node.id);
        if actual.as_deref() == Some(expected) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected resolved accessible name `{expected}`, got {actual:?}"
            )))
        }
    }

    pub fn require_accessible_description(&self, name: &str, expected: &str) -> TestResult {
        let node = self.node(name)?;
        let actual = self.tree.accessible_description(node.id);
        if actual.as_deref() == Some(expected) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected resolved accessible description `{expected}`, got {actual:?}"
            )))
        }
    }

    pub fn require_screen_reader_text_contains(&self, name: &str, text: &str) -> TestResult {
        let node = self.node(name)?;
        let actual = self.tree.screen_reader_text(node.id);
        if actual
            .as_deref()
            .is_some_and(|actual| actual.contains(text))
        {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected resolved screen-reader text containing `{text}`, got {actual:?}"
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

    pub fn require_effective_focus_order(&self, names: &[&str]) -> TestResult {
        let expected = names
            .iter()
            .map(|name| {
                LayoutAssertions::new(self.document)
                    .node(name)
                    .map(|(id, _)| id)
            })
            .collect::<TestResult<Vec<_>>>()?;
        let actual = self.tree.effective_focus_order();
        if actual == expected {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected effective accessibility focus order {expected:?}, got {actual:?}"
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

    pub fn require_move_focus(
        &self,
        target: UiNodeId,
        restore: FocusRestoreTarget,
    ) -> TestResult<&'a AccessibilityAdapterRequest> {
        self.requests
            .iter()
            .find(|request| {
                matches!(
                    request,
                    AccessibilityAdapterRequest::MoveFocus {
                        target: actual_target,
                        restore: actual_restore,
                    } if *actual_target == target && *actual_restore == restore
                )
            })
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing accessibility MoveFocus request for {target:?} with restore {restore:?}; available requests: {:?}",
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

#[derive(Debug, Clone, Copy)]
pub struct AccessibilityResponseAssertions<'a> {
    responses: &'a [AccessibilityAdapterResponse],
}

impl<'a> AccessibilityResponseAssertions<'a> {
    pub const fn new(responses: &'a [AccessibilityAdapterResponse]) -> Self {
        Self { responses }
    }

    pub const fn responses(&self) -> &'a [AccessibilityAdapterResponse] {
        self.responses
    }

    pub fn response_count(&self, kind: AccessibilityRequestKind) -> usize {
        self.responses
            .iter()
            .filter(|response| accessibility_response_kind(response) == Some(kind))
            .count()
    }

    pub fn require_unsupported(&self, kind: AccessibilityRequestKind) -> TestResult {
        if self.responses.iter().any(|response| {
            matches!(response, AccessibilityAdapterResponse::Unsupported(actual) if *actual == kind)
        }) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "missing accessibility Unsupported response for {kind:?}; available responses: {:?}",
                self.responses
            )))
        }
    }

    pub fn require_no_unsupported(&self) -> TestResult {
        if let Some(unsupported) = self
            .responses
            .iter()
            .find(|response| matches!(response, AccessibilityAdapterResponse::Unsupported(_)))
        {
            Err(TestFailure::new(format!(
                "expected no unsupported accessibility responses, got {unsupported:?}"
            )))
        } else {
            Ok(())
        }
    }
}

fn accessibility_response_kind(
    response: &AccessibilityAdapterResponse,
) -> Option<AccessibilityRequestKind> {
    match response {
        AccessibilityAdapterResponse::Unsupported(kind) => Some(*kind),
        AccessibilityAdapterResponse::Failed { request, .. } => Some(*request),
        AccessibilityAdapterResponse::Applied
        | AccessibilityAdapterResponse::FocusChanged(_)
        | AccessibilityAdapterResponse::PreferencesChanged(_) => None,
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

    pub fn require_kind_count(
        &self,
        selector: PaintKindSelector,
        expected_count: usize,
    ) -> TestResult {
        let actual = self.count_kind(selector);
        if actual == expected_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected {expected_count} paint items of kind {selector:?}, got {actual}"
            )))
        }
    }

    pub fn require_min_kind_count(
        &self,
        selector: PaintKindSelector,
        minimum_count: usize,
    ) -> TestResult {
        let actual = self.count_kind(selector);
        if actual >= minimum_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected at least {minimum_count} paint items of kind {selector:?}, got {actual}"
            )))
        }
    }

    pub fn node_items(&self, node_name: &str) -> TestResult<Vec<&PaintItem>> {
        let (id, _) = LayoutAssertions::new(self.document).node(node_name)?;
        let items = self
            .paint
            .items
            .iter()
            .filter(|item| item.node == id)
            .collect::<Vec<_>>();
        if items.is_empty() {
            let mut measurer = ApproxTextMeasurer;
            let visibility = DebugVisibilityTrace::from_document(self.document, &mut measurer);
            Err(TestFailure::new(format_paint_assertion_failure(
                node_name,
                self.document.node(id),
                visibility.record(node_name),
            )))
        } else {
            Ok(items)
        }
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

    pub fn require_node_shader(&self, node_name: &str, shader_key: &str) -> TestResult<&PaintItem> {
        self.node_items(node_name)?
            .into_iter()
            .find(|item| {
                item.shader
                    .as_ref()
                    .is_some_and(|shader| shader.key == shader_key)
            })
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "node `{node_name}` has no paint item using shader `{shader_key}`"
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

#[derive(Debug, Clone, Copy)]
pub struct CanvasHitAssertions<'a> {
    report: &'a CanvasRenderReport,
}

impl<'a> CanvasHitAssertions<'a> {
    pub const fn new(report: &'a CanvasRenderReport) -> Self {
        Self { report }
    }

    pub const fn report(&self) -> &'a CanvasRenderReport {
        self.report
    }

    pub fn collections(&self) -> Vec<CanvasHitCollection> {
        self.report.hit_collections()
    }

    pub fn targets(&self) -> Vec<CanvasHitTarget> {
        self.report.hit_targets()
    }

    pub fn require_collection_count(&self, expected_count: usize) -> TestResult {
        let actual = self.collections().len();
        if actual == expected_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected {expected_count} canvas hit collections, got {actual}"
            )))
        }
    }

    pub fn require_collection(&self, key: &str) -> TestResult<CanvasHitCollection> {
        let collections = self.collections();
        collections
            .into_iter()
            .find(|collection| collection.key == key)
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing canvas hit collection `{key}`; available collections: {:?}",
                    self.collection_keys()
                ))
            })
    }

    pub fn require_collection_for_node(
        &self,
        node: UiNodeId,
        key: &str,
    ) -> TestResult<CanvasHitCollection> {
        let collections = self.collections();
        collections
            .into_iter()
            .find(|collection| collection.node == node && collection.key == key)
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing canvas hit collection `{key}` for node {node:?}; available collections: {:?}",
                    self.collection_keys()
                ))
            })
    }

    pub fn require_target_ids(&self, key: &str, expected_ids: &[&str]) -> TestResult {
        let collection = self.require_collection(key)?;
        let actual = collection
            .targets
            .iter()
            .map(|target| target.id.as_str())
            .collect::<Vec<_>>();
        if actual == expected_ids {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "canvas `{key}` expected hit target ids {expected_ids:?}, got {actual:?}"
            )))
        }
    }

    pub fn require_target(&self, key: &str, target_id: &str) -> TestResult<CanvasHitTarget> {
        let collection = self.require_collection(key)?;
        collection
            .targets
            .into_iter()
            .find(|target| target.id == target_id)
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "canvas `{key}` missing hit target `{target_id}`; available targets: {:?}",
                    self.target_ids(key).unwrap_or_default()
                ))
            })
    }

    pub fn require_topmost_target_at(
        &self,
        key: &str,
        point: UiPoint,
        expected_target_id: &str,
    ) -> TestResult {
        let collection = self.require_collection(key)?;
        let actual = collection
            .topmost_at(point)
            .map(|target| target.id.as_str());
        if actual == Some(expected_target_id) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "canvas `{key}` expected topmost hit target `{expected_target_id}` at {point:?}, got {actual:?}"
            )))
        }
    }

    pub fn require_target_accessibility_label(
        &self,
        key: &str,
        target_id: &str,
        expected_label: &str,
    ) -> TestResult {
        let collection = self.require_collection(key)?;
        let Some((index, target)) = collection
            .targets
            .iter()
            .enumerate()
            .find(|(_, target)| target.id == target_id)
        else {
            return Err(TestFailure::new(format!(
                "canvas `{key}` missing hit target `{target_id}`; available targets: {:?}",
                self.target_ids(key).unwrap_or_default()
            )));
        };
        let meta = target.accessibility_meta(index, collection.targets.len(), false);
        if meta.label.as_deref() == Some(expected_label) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "canvas `{key}` target `{target_id}` expected accessibility label `{expected_label}`, got {:?}",
                meta.label
            )))
        }
    }

    pub fn require_target_disabled(
        &self,
        key: &str,
        target_id: &str,
        expected_disabled: bool,
    ) -> TestResult {
        let target = self.require_target(key, target_id)?;
        if target.disabled == expected_disabled {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "canvas `{key}` target `{target_id}` expected disabled={expected_disabled}, got {}",
                target.disabled
            )))
        }
    }

    pub fn require_target_metadata(
        &self,
        key: &str,
        target_id: &str,
        metadata_key: &str,
        expected_value: &str,
    ) -> TestResult {
        let target = self.require_target(key, target_id)?;
        if target
            .metadata
            .iter()
            .any(|(key, value)| key == metadata_key && value == expected_value)
        {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "canvas `{key}` target `{target_id}` expected metadata `{metadata_key}`=`{expected_value}`, got {:?}",
                target.metadata
            )))
        }
    }

    fn collection_keys(&self) -> Vec<String> {
        self.collections()
            .into_iter()
            .map(|collection| collection.key)
            .collect()
    }

    fn target_ids(&self, key: &str) -> Option<Vec<String>> {
        self.collections()
            .into_iter()
            .find(|collection| collection.key == key)
            .map(|collection| {
                collection
                    .targets
                    .into_iter()
                    .map(|target| target.id)
                    .collect()
            })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOutputAssertions<'a> {
    output: &'a RenderFrameOutput,
}

impl<'a> RenderOutputAssertions<'a> {
    pub const fn new(output: &'a RenderFrameOutput) -> Self {
        Self { output }
    }

    pub const fn output(&self) -> &'a RenderFrameOutput {
        self.output
    }

    pub fn timing_assertions(&self) -> FrameTimingAssertions<'a> {
        FrameTimingAssertions::new(&self.output.timings)
    }

    pub fn require_target_kind(&self, kind: RenderTargetKind) -> TestResult {
        let actual = self.output.target.kind();
        if actual == kind {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected render target kind {kind:?}, got {actual:?}"
            )))
        }
    }

    pub fn require_painted_items(&self, expected_count: usize) -> TestResult {
        if self.output.painted_items == expected_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected {expected_count} painted items, got {}",
                self.output.painted_items
            )))
        }
    }

    pub fn require_min_painted_items(&self, minimum_count: usize) -> TestResult {
        if self.output.painted_items >= minimum_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected at least {minimum_count} painted items, got {}",
                self.output.painted_items
            )))
        }
    }

    pub fn require_batch_count(&self, expected_count: usize) -> TestResult {
        let actual = self.output.batches.len();
        if actual == expected_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected {expected_count} render batches, got {actual}"
            )))
        }
    }

    pub fn require_min_batch_count(&self, minimum_count: usize) -> TestResult {
        let actual = self.output.batches.len();
        if actual >= minimum_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "expected at least {minimum_count} render batches, got {actual}"
            )))
        }
    }

    pub fn require_snapshot(&self) -> TestResult<&'a RenderedImage> {
        self.output.snapshot.as_ref().ok_or_else(|| {
            TestFailure::new(format!(
                "render target {:?} did not produce a snapshot",
                self.output.target.kind()
            ))
        })
    }

    pub fn require_no_snapshot(&self) -> TestResult {
        if self.output.snapshot.is_none() {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "render target {:?} unexpectedly produced a snapshot",
                self.output.target.kind()
            )))
        }
    }

    pub fn require_snapshot_format(&self, format: ResourceFormat) -> TestResult<&'a RenderedImage> {
        let image = self.require_snapshot()?;
        if image.format == format {
            Ok(image)
        } else {
            Err(TestFailure::new(format!(
                "expected snapshot format {format:?}, got {:?}",
                image.format
            )))
        }
    }

    pub fn require_snapshot_rgba8(
        &self,
        name: impl Into<String>,
    ) -> TestResult<SnapshotAssertions<'a>> {
        let image = self.require_snapshot_format(ResourceFormat::Rgba8)?;
        let view = RgbaImageView::new(
            image.size.width as usize,
            image.size.height as usize,
            &image.pixels,
        )?;
        Ok(SnapshotAssertions::new(name, view))
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
                    request.id.value()
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
                response.id.value(),
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
                    response.id.value(),
                    response.kind()
                )));
            }
        }
        Ok(())
    }

    pub fn require_all_requests_have_responses(&self) -> TestResult {
        for request in self.requests.iter() {
            if !self
                .responses
                .iter()
                .any(|response| response.is_for(request) && response.kind() == request.kind())
            {
                return Err(TestFailure::new(format!(
                    "platform request id {} kind {:?} has no matching response",
                    request.id.value(),
                    request.kind()
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
                "platform response id {} kind {:?} was unsupported{}",
                response.id.value(),
                response.kind(),
                platform_response_diagnostic_suffix(response)
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
                "platform response id {} kind {:?} returned {:?}: {}{}",
                response.id.value(),
                response.kind(),
                error.code,
                error.message,
                platform_response_diagnostic_suffix(response)
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

fn platform_response_diagnostic_suffix(response: &PlatformServiceResponse) -> String {
    crate::errors::classify_platform_service_response(response)
        .map(|report| format!("\n{report}"))
        .unwrap_or_default()
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

    pub fn hash(self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in self.pixels {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    pub fn changed_pixels_from(self, color: ColorRgba) -> usize {
        self.pixels
            .chunks_exact(4)
            .filter(|pixel| *pixel != [color.r, color.g, color.b, color.a])
            .count()
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

#[derive(Debug, Clone)]
pub struct SnapshotAssertions<'a> {
    name: String,
    image: RgbaImageView<'a>,
}

impl<'a> SnapshotAssertions<'a> {
    pub fn new(name: impl Into<String>, image: RgbaImageView<'a>) -> Self {
        Self {
            name: name.into(),
            image,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn image(&self) -> RgbaImageView<'a> {
        self.image
    }

    pub fn hash(&self) -> u64 {
        self.image.hash()
    }

    pub fn changed_pixels_from(&self, color: ColorRgba) -> usize {
        self.image.changed_pixels_from(color)
    }

    pub fn require_hash(&self, expected_hash: u64) -> TestResult<u64> {
        let actual = self.hash();
        if expected_hash == 0 {
            return Err(TestFailure::new(format!(
                "{} snapshot hash: {actual:#018x}",
                self.name
            )));
        }
        if actual == expected_hash {
            Ok(actual)
        } else {
            Err(TestFailure::new(format!(
                "{} snapshot hash changed: expected {expected_hash:#018x}, got {actual:#018x}",
                self.name
            )))
        }
    }

    pub fn require_min_changed_pixels_from(
        &self,
        color: ColorRgba,
        minimum_changed_pixels: usize,
    ) -> TestResult<usize> {
        let changed_pixels = self.changed_pixels_from(color);
        if changed_pixels >= minimum_changed_pixels {
            Ok(changed_pixels)
        } else {
            Err(TestFailure::new(format!(
                "{} rendered too little content: expected at least {minimum_changed_pixels} changed pixels, got {changed_pixels}",
                self.name
            )))
        }
    }

    pub fn require_matches(
        &self,
        expected: RgbaImageView<'_>,
        tolerance: PixelDiffTolerance,
    ) -> TestResult<PixelDiffReport> {
        let report = diff_rgba8(expected, self.image)?;
        if report.is_within(tolerance) {
            Ok(report)
        } else {
            Err(TestFailure::new(format!(
                "{} snapshot differed beyond tolerance: {} changed pixels, max channel delta {}, total channel delta {}",
                self.name,
                report.changed_pixels,
                report.max_channel_delta,
                report.total_channel_delta
            )))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameTimingAssertions<'a> {
    timing: &'a FrameTiming,
}

impl<'a> FrameTimingAssertions<'a> {
    pub const fn new(timing: &'a FrameTiming) -> Self {
        Self { timing }
    }

    pub const fn timing(&self) -> &'a FrameTiming {
        self.timing
    }

    pub fn explanation(&self, budget: Option<Duration>) -> FrameTimingExplanation {
        FrameTimingExplanation::from_frame_timing(self.timing, budget)
    }

    pub fn require_section(&self, name: &str) -> TestResult<Duration> {
        self.timing.duration(name).ok_or_else(|| {
            TestFailure::new(format!(
                "missing frame timing section `{name}`; available sections: {:?}",
                self.section_names()
            ))
        })
    }

    pub fn require_sections<'b>(
        &self,
        names: impl IntoIterator<Item = &'b str>,
    ) -> TestResult<Vec<Duration>> {
        names
            .into_iter()
            .map(|name| self.require_section(name))
            .collect()
    }

    pub fn require_total_within(&self, budget: Duration) -> TestResult<Duration> {
        let total = self.timing.total();
        if total <= budget {
            Ok(total)
        } else {
            Err(TestFailure::new(format_frame_timing_budget_failure(
                &self.explanation(Some(budget)),
            )))
        }
    }

    pub fn require_section_within(&self, name: &str, budget: Duration) -> TestResult<Duration> {
        let duration = self.require_section(name)?;
        if duration <= budget {
            Ok(duration)
        } else {
            Err(TestFailure::new(format!(
                "frame timing section `{name}` duration {duration:?} exceeded budget {budget:?}; {}; inspect: frame_timing_panel",
                format_frame_timing_section_detail(self.timing, name, duration),
            )))
        }
    }

    fn section_names(&self) -> Vec<&str> {
        self.timing
            .sections
            .iter()
            .map(|section| section.name.as_str())
            .collect()
    }
}

fn format_frame_timing_budget_failure(explanation: &FrameTimingExplanation) -> String {
    let budget = explanation
        .budget
        .map(|budget| format!("{budget:?}"))
        .unwrap_or_else(|| "none".to_owned());
    let over_by = explanation
        .budget
        .and_then(|budget| (explanation.total > budget).then(|| explanation.total - budget))
        .map(|duration| format!(" by {duration:?}"))
        .unwrap_or_default();
    let slowest = explanation
        .slowest_stage_explanation()
        .map(format_frame_timing_stage_explanation)
        .unwrap_or_else(|| "no timing stages recorded".to_owned());
    let grouped = if explanation.stages.is_empty() {
        "none".to_owned()
    } else {
        explanation
            .stages
            .iter()
            .take(3)
            .map(format_frame_timing_stage_explanation)
            .collect::<Vec<_>>()
            .join("; ")
    };
    let missing = if explanation.missing_required_stages.is_empty() {
        "none".to_owned()
    } else {
        explanation
            .missing_required_stages
            .iter()
            .map(|stage| stage.label().to_owned())
            .collect::<Vec<_>>()
            .join(", ")
    };

    format!(
        "frame timing total {:?} exceeded budget {budget}{over_by}; slowest stage: {slowest}; grouped stages: {grouped}; missing instrumentation: {missing}; inspect: frame_timing_panel, frame_budget_panel, or frame_bottleneck_panel",
        explanation.total
    )
}

fn format_frame_timing_stage_explanation(
    stage: &crate::diagnostics::FrameTimingStageExplanation,
) -> String {
    let sources = if stage.source_labels.is_empty() {
        "none".to_owned()
    } else {
        stage.source_labels.join("+")
    };
    format!(
        "{} {:?} ({:.1}% of frame, sections: {sources})",
        stage.label, stage.duration, stage.percent_of_total
    )
}

fn format_frame_timing_section_detail(
    timing: &FrameTiming,
    section_name: &str,
    duration: Duration,
) -> String {
    let explanation = FrameTimingExplanation::from_frame_timing(timing, None);
    let percent = timing
        .section_fraction(section_name)
        .map(|fraction| format!("{:.1}% of frame", fraction * 100.0))
        .unwrap_or_else(|| "unknown frame share".to_owned());
    let stage = explanation
        .stages
        .iter()
        .find(|stage| {
            stage
                .source_labels
                .iter()
                .any(|source| source == section_name)
        })
        .map(|stage| {
            format!(
                "pipeline stage {} totals {:?} across {} section{}",
                stage.label,
                stage.duration,
                stage.section_count,
                plural_s(stage.section_count)
            )
        })
        .unwrap_or_else(|| "no matching pipeline stage".to_owned());
    format!("section share {percent}; {stage}; sample {duration:?}")
}

fn plural_s(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameTimingSeries {
    name: String,
    frames: Vec<FrameTiming>,
}

impl FrameTimingSeries {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            frames: Vec::new(),
        }
    }

    pub fn frame(mut self, timing: FrameTiming) -> Self {
        self.push(timing);
        self
    }

    pub fn push(&mut self, timing: FrameTiming) {
        self.frames.push(timing);
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn frames(&self) -> &[FrameTiming] {
        &self.frames
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn section_names(&self) -> Vec<&str> {
        let mut names = Vec::new();
        for frame in &self.frames {
            for section in &frame.sections {
                if !names.contains(&section.name.as_str()) {
                    names.push(section.name.as_str());
                }
            }
        }
        names
    }

    pub fn total_samples(&self) -> PerformanceSamples {
        let mut samples = PerformanceSamples::new(format!("{}.total", self.name));
        for frame in &self.frames {
            samples.push(frame.total());
        }
        samples
    }

    pub fn section_samples(&self, section_name: &str) -> PerformanceSamples {
        let mut samples = PerformanceSamples::new(format!("{}.{}", self.name, section_name));
        for frame in &self.frames {
            for section in &frame.sections {
                if section.name == section_name {
                    samples.push(section.duration);
                }
            }
        }
        samples
    }

    pub fn section_summaries(&self) -> Vec<FrameTimingSectionSummary> {
        let all_sections_total = self
            .frames
            .iter()
            .flat_map(|frame| frame.sections.iter())
            .map(|section| section.duration)
            .sum();
        let mut aggregates = Vec::<(String, usize, Duration, Duration)>::new();
        for frame in &self.frames {
            for section in &frame.sections {
                if let Some((_, sample_count, total, max)) = aggregates
                    .iter_mut()
                    .find(|(name, _, _, _)| name == &section.name)
                {
                    *sample_count += 1;
                    *total += section.duration;
                    *max = (*max).max(section.duration);
                } else {
                    aggregates.push((section.name.clone(), 1, section.duration, section.duration));
                }
            }
        }
        aggregates
            .into_iter()
            .map(|(name, sample_count, total, max)| {
                FrameTimingSectionSummary::new(name, sample_count, total, max, all_sections_total)
            })
            .collect()
    }

    pub fn dominant_section(&self) -> Option<FrameTimingSectionSummary> {
        self.section_summaries()
            .into_iter()
            .max_by_key(|summary| summary.total)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameTimingSeriesAssertions<'a> {
    series: &'a FrameTimingSeries,
}

impl<'a> FrameTimingSeriesAssertions<'a> {
    pub const fn new(series: &'a FrameTimingSeries) -> Self {
        Self { series }
    }

    pub const fn series(&self) -> &'a FrameTimingSeries {
        self.series
    }

    pub fn require_frame_count(&self, expected_count: usize) -> TestResult {
        let actual = self.series.len();
        if actual == expected_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "{} expected {expected_count} frame timing samples, got {actual}",
                self.series.name()
            )))
        }
    }

    pub fn require_section_sample_count(
        &self,
        section_name: &str,
        expected_count: usize,
    ) -> TestResult {
        PerformanceAssertions::new(&self.series.section_samples(section_name))
            .require_sample_count(expected_count)
    }

    pub fn require_total_average_within(&self, budget: Duration) -> TestResult<Duration> {
        let samples = self.series.total_samples();
        let average = samples.average().ok_or_else(|| {
            TestFailure::new(format!(
                "{} has no frame timing samples",
                self.series.name()
            ))
        })?;
        if average <= budget {
            Ok(average)
        } else {
            Err(TestFailure::new(format!(
                "{} total average {average:?} exceeded budget {budget:?}; dominant section: {}",
                self.series.name(),
                dominant_section_summary(self.series)
            )))
        }
    }

    pub fn require_total_max_within(&self, budget: Duration) -> TestResult<Duration> {
        PerformanceAssertions::new(&self.series.total_samples()).require_max_sample_within(budget)
    }

    pub fn require_total_percentile_within(
        &self,
        percentile: f64,
        budget: Duration,
    ) -> TestResult<Duration> {
        PerformanceAssertions::new(&self.series.total_samples())
            .require_percentile_within(percentile, budget)
    }

    pub fn require_section_average_within(
        &self,
        section_name: &str,
        budget: Duration,
    ) -> TestResult<Duration> {
        PerformanceAssertions::new(&self.series.section_samples(section_name))
            .require_average_within(budget)
    }

    pub fn require_section_max_within(
        &self,
        section_name: &str,
        budget: Duration,
    ) -> TestResult<Duration> {
        PerformanceAssertions::new(&self.series.section_samples(section_name))
            .require_max_sample_within(budget)
    }

    pub fn require_section_percentile_within(
        &self,
        section_name: &str,
        percentile: f64,
        budget: Duration,
    ) -> TestResult<Duration> {
        PerformanceAssertions::new(&self.series.section_samples(section_name))
            .require_percentile_within(percentile, budget)
    }

    pub fn require_dominant_section(
        &self,
        expected_section_name: &str,
    ) -> TestResult<FrameTimingSectionSummary> {
        let summary = self.series.dominant_section().ok_or_else(|| {
            TestFailure::new(format!(
                "{} has no frame timing sections",
                self.series.name()
            ))
        })?;
        if summary.name == expected_section_name {
            Ok(summary)
        } else {
            Err(TestFailure::new(format!(
                "{} expected dominant frame timing section `{expected_section_name}`, got {}",
                self.series.name(),
                summary.diagnostic_summary()
            )))
        }
    }
}

fn dominant_section_summary(series: &FrameTimingSeries) -> String {
    series
        .dominant_section()
        .map(|summary| summary.diagnostic_summary())
        .unwrap_or_else(|| "no sections recorded".to_string())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PerformanceSamples {
    name: String,
    samples: Vec<Duration>,
}

impl PerformanceSamples {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            samples: Vec::new(),
        }
    }

    pub fn single(name: impl Into<String>, duration: Duration) -> Self {
        Self::new(name).sample(duration)
    }

    pub fn sample(mut self, duration: Duration) -> Self {
        self.push(duration);
        self
    }

    pub fn push(&mut self, duration: Duration) {
        self.samples.push(duration);
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn samples(&self) -> &[Duration] {
        &self.samples
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn total(&self) -> Duration {
        self.samples.iter().copied().sum()
    }

    pub fn max_sample(&self) -> Option<Duration> {
        self.samples.iter().copied().max()
    }

    pub fn average(&self) -> Option<Duration> {
        (!self.samples.is_empty()).then(|| {
            Duration::from_secs_f64(self.total().as_secs_f64() / self.samples.len() as f64)
        })
    }

    pub fn percentile(&self, percentile: f64) -> Option<Duration> {
        if self.samples.is_empty() || !percentile.is_finite() {
            return None;
        }
        let mut samples = self.samples.clone();
        samples.sort_unstable();
        let clamped = percentile.clamp(0.0, 100.0);
        let index = if clamped <= 0.0 {
            0
        } else {
            ((clamped / 100.0 * samples.len() as f64).ceil() as usize)
                .saturating_sub(1)
                .min(samples.len() - 1)
        };
        samples.get(index).copied()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PerformanceAssertions<'a> {
    samples: &'a PerformanceSamples,
}

impl<'a> PerformanceAssertions<'a> {
    pub const fn new(samples: &'a PerformanceSamples) -> Self {
        Self { samples }
    }

    pub const fn samples(&self) -> &'a PerformanceSamples {
        self.samples
    }

    pub fn require_sample_count(&self, expected_count: usize) -> TestResult {
        let actual = self.samples.len();
        if actual == expected_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "{} expected {expected_count} performance samples, got {actual}",
                self.samples.name()
            )))
        }
    }

    pub fn require_min_sample_count(&self, minimum_count: usize) -> TestResult {
        let actual = self.samples.len();
        if actual >= minimum_count {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "{} expected at least {minimum_count} performance samples, got {actual}",
                self.samples.name()
            )))
        }
    }

    pub fn require_total_within(&self, budget: Duration) -> TestResult<Duration> {
        let total = self.samples.total();
        if total <= budget {
            Ok(total)
        } else {
            Err(TestFailure::new(format!(
                "{} total duration {total:?} exceeded budget {budget:?} across {} sample(s)",
                self.samples.name(),
                self.samples.len()
            )))
        }
    }

    pub fn require_average_within(&self, budget: Duration) -> TestResult<Duration> {
        let average = self.samples.average().ok_or_else(|| {
            TestFailure::new(format!(
                "{} has no performance samples",
                self.samples.name()
            ))
        })?;
        if average <= budget {
            Ok(average)
        } else {
            Err(TestFailure::new(format!(
                "{} average duration {average:?} exceeded budget {budget:?} across {} sample(s)",
                self.samples.name(),
                self.samples.len()
            )))
        }
    }

    pub fn require_max_sample_within(&self, budget: Duration) -> TestResult<Duration> {
        let max_sample = self.samples.max_sample().ok_or_else(|| {
            TestFailure::new(format!(
                "{} has no performance samples",
                self.samples.name()
            ))
        })?;
        if max_sample <= budget {
            Ok(max_sample)
        } else {
            Err(TestFailure::new(format!(
                "{} max sample duration {max_sample:?} exceeded budget {budget:?}",
                self.samples.name()
            )))
        }
    }

    pub fn require_percentile_within(
        &self,
        percentile: f64,
        budget: Duration,
    ) -> TestResult<Duration> {
        let sample = self.samples.percentile(percentile).ok_or_else(|| {
            if percentile.is_finite() {
                TestFailure::new(format!(
                    "{} has no performance samples",
                    self.samples.name()
                ))
            } else {
                TestFailure::new(format!(
                    "{} percentile must be finite, got {percentile}",
                    self.samples.name()
                ))
            }
        })?;
        if sample <= budget {
            Ok(sample)
        } else {
            Err(TestFailure::new(format!(
                "{} p{percentile} duration {sample:?} exceeded budget {budget:?}",
                self.samples.name()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{
        Command, CommandId, CommandMeta, CommandRegistry, CommandScope, Shortcut,
    };
    use crate::debug::{
        DebugContractCheckKind, DebugFrameChangeKind, DebugInspectNodeSource, DebugWhyScope,
        DebugWhySource,
    };
    use crate::host::{
        process_document_frame, HostDocumentFrameRequest, HostFrameOutput, HostInteractionState,
    };
    use crate::input::{RawKeyboardEvent, RawWheelEvent, WheelDeltaUnit};
    use crate::platform::{
        ClipboardRequest, ClipboardResponse, CursorGrabMode, CursorRequest, PixelSize,
        PlatformErrorCode, PlatformRequest, PlatformRequestId, PlatformRequestIdAllocator,
        PlatformResponse, PlatformServiceError, PlatformServiceKind, RepaintRequest,
        RepaintResponse,
    };
    use crate::renderer::{
        CanvasHostCaptureDiagnosticKind, CanvasRenderContext, CanvasRenderOutput,
        CanvasRenderRegistry, DirtyRegionSet, ImageRenderContext, ImageRenderOutput,
        ImageRenderRegistry, PaintBatch, PaintBatchKey, PaintBatchKind, RenderFrameOutput,
        RenderFrameRequest, RenderTarget, RenderTargetKind, RenderedImage, ResourceFormat,
    };
    use crate::DirtyFlags;
    use crate::{
        length, root_style, AccessibilityAction, AccessibilityLiveRegion, AccessibilityMeta,
        AccessibilityRole, AccessibilitySummary, AccessibilityValueRange, ApproxTextMeasurer,
        CanvasContent, CanvasInteractionPolicy, ClipBehavior, ColorRgba, ImageContent,
        InputBehavior, LayoutStyle, PaintTransform, ScrollAxes, ShaderEffect, StrokeStyle,
        TextStyle, UiContent, UiDocument, UiNode, UiNodeStyle, UiPoint, UiVisual,
    };
    use taffy::prelude::{Dimension, Size as TaffySize, Style};

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
            .pointer_click("play", UiPoint::new(12.0, 12.0))
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
        assert_eq!(
            report.step("play.up").expect("up step").converted,
            Some(UiInputEvent::PointerUp(UiPoint::new(12.0, 12.0)))
        );
        report.require_clicked(button).expect("clicked button");
        report.require_focused(button).expect("focused button");
        report.require_no_scrolls().expect("no scrolls");
        assert!(report.require_all_converted().is_ok());
    }

    #[test]
    fn event_replay_builders_and_assertions_cover_scroll_and_miss_paths() {
        let mut document = UiDocument::new(root_style(160.0, 120.0));
        let scroll_area = document.add_child(
            document.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(120.0),
                            height: length(48.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        document.add_child(
            scroll_area,
            UiNode::container("content", fixed_style(120.0, 140.0)),
        );
        document
            .compute_layout(UiSize::new(160.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let mut recorder = InteractionRecorder::new();
        recorder.record_wheel(
            "recorded.scroll",
            UiPoint::new(24.0, 24.0),
            UiPoint::new(0.0, 32.0),
        );
        let report = recorder
            .into_replay()
            .long_wheel_scroll(
                "scroll.down",
                UiPoint::new(24.0, 24.0),
                UiPoint::new(0.0, 32.0),
                8,
            )
            .pointer_drag(
                "empty.drag",
                UiPoint::new(140.0, 80.0),
                UiPoint::new(150.0, 88.0),
                [UiPoint::new(145.0, 84.0)],
            )
            .run(&mut document);

        report
            .require_step_count(14)
            .expect("recorded scroll plus drag");
        report.require_scrolled(scroll_area).expect("scrolled area");
        report
            .require_scroll_at_end(&document, scroll_area)
            .expect("long synthetic scroll reaches end");
        report.require_no_clicks().expect("drag outside misses");
        assert!(report.require_clicked(scroll_area).is_err());
        assert!(report.step("empty.drag.move.0").is_ok());
        assert!(report.step("missing").is_err());
    }

    #[test]
    fn event_replay_named_assertions_explain_pointer_route_failures() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        document.add_child(
            document.root,
            UiNode::container(
                "back.button",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 90.0, 32.0)).style,
                    z_index: 1.0,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON),
        );
        document.add_child(
            document.root,
            UiNode::container(
                "front.button",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(50.0, 16.0, 90.0, 40.0)).style,
                    z_index: 10.0,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let report = EventReplay::new()
            .pointer_click("back.center", UiPoint::new(65.0, 36.0))
            .run(&mut document);

        report
            .require_clicked_name(&document, "front.button")
            .expect("front button clicked");
        report
            .require_focused_name(&document, "front.button")
            .expect("front button focused");
        report
            .require_step_focused_name(&document, "back.center.down", "front.button")
            .expect("front button focused on down");
        report
            .require_step_consumed_by_name(&document, "back.center.down", "front.button")
            .expect("front button consumed down");
        report
            .require_step_clicked_name(&document, "back.center.up", "front.button")
            .expect("front button clicked on up");
        let error = report
            .require_clicked_name(&document, "back.button")
            .expect_err("back button should be covered");
        let step_error = report
            .require_step_clicked_name(&document, "back.center.up", "back.button")
            .expect_err("back button should be covered on pointer up");

        assert!(error
            .message
            .contains("expected event replay clicked `back.button`"));
        assert!(error.message.contains("got [front.button]"));
        assert!(error.message.contains("last pointer pointer-up"));
        assert!(error.message.contains("routed target: front.button"));
        assert!(error.message.contains("expected candidate:"));
        assert!(error.message.contains("cause:"));
        assert!(error.message.contains("fix:"));
        assert!(error.message.contains("pointer_autopsy_panel"));
        assert!(step_error
            .message
            .contains("expected event replay step `back.center.up` clicked `back.button`"));
        assert!(step_error.message.contains("got front.button"));
        assert!(step_error.message.contains("step pointer pointer-up"));
        assert!(step_error.message.contains("routed target: front.button"));
        assert!(step_error.message.contains("expected candidate:"));
        assert!(step_error.message.contains("pointer_autopsy_panel"));
    }

    #[test]
    fn event_replay_reports_topmost_wheel_consumption_without_scroll() {
        let mut document = UiDocument::new(root_style(160.0, 140.0));
        let back_scroll = document.add_child(
            document.root,
            UiNode::container(
                "back.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 100.0, 60.0)).style,
                    clip: ClipBehavior::Clip,
                    z_index: 1.0,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        document.add_child(
            back_scroll,
            UiNode::container("back.content", fixed_style(100.0, 140.0)),
        );
        let front = document.add_child(
            document.root,
            UiNode::container(
                "front.panel",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(10.0, 10.0, 120.0, 90.0)).style,
                    clip: ClipBehavior::Clip,
                    z_index: 10.0,
                    ..Default::default()
                },
            )
            .with_visual(UiVisual::panel(ColorRgba::new(28, 34, 44, 255), None, 0.0)),
        );
        document
            .compute_layout(UiSize::new(160.0, 140.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let report = EventReplay::new()
            .wheel(
                "front.wheel",
                UiPoint::new(30.0, 30.0),
                UiPoint::new(0.0, 30.0),
            )
            .run(&mut document);

        report.require_no_scrolls().expect("no lower scroll");
        report
            .require_consumed_by(front)
            .expect("front panel consumed wheel");
        report
            .require_step_consumed_by("front.wheel", front)
            .expect("front wheel step consumption");
        assert_eq!(document.scroll_state(back_scroll).unwrap().offset.y, 0.0);
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
    fn event_replay_records_direct_command_steps() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let mut registry = CommandRegistry::new();
        registry
            .register(Command::new(CommandMeta::new("file.save", "Save")))
            .unwrap();
        registry
            .register(Command::new(CommandMeta::new("file.export", "Export")).disabled("clean"))
            .unwrap();

        let mut recorder = InteractionRecorder::new();
        recorder
            .record_command("palette.save", "file.save")
            .record_command("palette.export", "file.export");
        let report = recorder.into_replay().run_with_commands(
            &mut document,
            HostInteractionState::default(),
            &registry,
        );

        assert_eq!(
            report.dispatched_commands(),
            vec![CommandId::new("file.save")]
        );
        assert_eq!(
            report.steps[0].direct_command,
            Some(CommandId::new("file.save"))
        );
        assert_eq!(report.steps[0].converted, None);
        assert_eq!(report.steps[1].direct_command, None);
        report
            .require_command_dispatched("file.save")
            .expect("direct command dispatch");
        assert!(report.require_command_dispatched("file.export").is_err());
    }

    #[test]
    fn event_replay_records_platform_response_steps() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let response = PlatformServiceResponse::new(
            PlatformRequestId::new(77),
            PlatformResponse::Clipboard(ClipboardResponse::Text(Some("copied".to_owned()))),
        );

        let mut recorder = InteractionRecorder::new();
        recorder.record_platform_response("clipboard.response", response.clone());
        let report = recorder.into_replay().run(&mut document);

        let step = report
            .step("clipboard.response")
            .expect("platform response step");
        assert_eq!(step.converted, None);
        assert_eq!(step.result, None);
        assert_eq!(step.platform_response.as_ref(), Some(&response));
        assert_eq!(
            report.platform_responses().cloned().collect::<Vec<_>>(),
            vec![response.clone()]
        );
        assert_eq!(
            report
                .require_platform_response(PlatformRequestId::new(77))
                .expect("recorded platform response"),
            &response
        );
    }

    #[test]
    fn scenario_harness_replays_platform_responses_into_host_output() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        let mut harness = ScenarioHarness::new(UiSize::new(180.0, 100.0));
        let response = PlatformServiceResponse::new(
            PlatformRequestId::new(91),
            PlatformResponse::Repaint(RepaintResponse::Scheduled {
                delay: Duration::from_millis(16),
            }),
        );

        let frame = harness
            .run_frame(
                "host.response",
                &mut document,
                EventReplay::new().platform_response("repaint.response", response.clone()),
            )
            .expect("scenario frame");

        assert_eq!(
            frame
                .events
                .platform_responses()
                .cloned()
                .collect::<Vec<_>>(),
            vec![response.clone()]
        );
        assert_eq!(
            frame.document.host_output.platform_responses,
            vec![response]
        );
        frame
            .events
            .require_platform_response(PlatformRequestId::new(91))
            .expect("scenario replayed response");
    }

    #[test]
    fn scenario_harness_applies_replayed_window_resize_before_layout() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        let mut harness = ScenarioHarness::new(UiSize::new(180.0, 100.0));
        let resized = UiSize::new(260.0, 140.0);

        let mut recorder = InteractionRecorder::new();
        recorder.record_window_resize("window.resize", resized);
        let frame = harness
            .run_frame("resized", &mut document, recorder.into_replay())
            .expect("scenario frame");

        assert_eq!(harness.viewport, resized);
        assert_eq!(
            frame.render.target,
            RenderTarget::window("scenario", resized)
        );
        assert_eq!(
            frame
                .events
                .step("window.resize")
                .expect("resize step")
                .viewport_resize,
            Some(resized)
        );
        frame
            .events
            .require_viewport_resize(resized)
            .expect("resize recorded");
    }

    #[test]
    fn scenario_frame_reports_canvas_capture_diagnostics() {
        let viewport = UiSize::new(320.0, 200.0);
        let mut document = UiDocument::new(fixed_style(320.0, 200.0));
        let canvas = document.add_child(
            document.root,
            UiNode::canvas(
                "viewport",
                "app.viewport",
                LayoutStyle::from_taffy_style(fixed_style(160.0, 96.0).layout),
            ),
        );
        document.set_node_content(
            canvas,
            UiContent::Canvas(
                CanvasContent::new("app.viewport")
                    .native_viewport()
                    .interaction(CanvasInteractionPolicy::NATIVE_VIEWPORT),
            ),
        );
        let mut harness = ScenarioHarness::new(viewport);

        let frame = harness
            .run_frame("canvas.capture", &mut document, EventReplay::new())
            .expect("scenario frame");

        let diagnostics = frame.canvas_capture_diagnostics(PlatformServiceCapabilities::DESKTOP);
        assert!(diagnostics.has_kind(CanvasHostCaptureDiagnosticKind::Acquired));
        assert_eq!(
            diagnostics
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.node == Some(canvas))
                .count(),
            1
        );

        let unavailable = frame.canvas_capture_diagnostics(PlatformServiceCapabilities::NONE);
        assert!(unavailable.has_kind(CanvasHostCaptureDiagnosticKind::Unavailable));
        assert!(!unavailable.diagnostics[0].unsupported_requests.is_empty());
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
    fn scenario_harness_runs_replay_document_frame_and_records_paint() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        let root = document.root;
        let button = document.add_child(
            root,
            UiNode::container("menu.open", fixed_style(96.0, 32.0))
                .with_input(InputBehavior::BUTTON)
                .with_visual(UiVisual::panel(
                    ColorRgba::new(32, 44, 58, 255),
                    Some(StrokeStyle::new(ColorRgba::new(98, 128, 164, 255), 1.0)),
                    4.0,
                ))
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Open menu")
                        .focusable()
                        .action(AccessibilityAction::new("open", "Open")),
                ),
        );
        document.add_child(
            button,
            UiNode::text(
                "menu.open.label",
                "Open",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(fixed_style(52.0, 18.0).layout),
            ),
        );

        let mut harness = ScenarioHarness::new(UiSize::new(180.0, 100.0));
        let report = harness
            .run_frame(
                "open-menu",
                &mut document,
                EventReplay::new().pointer_click("open", UiPoint::new(16.0, 16.0)),
            )
            .expect("scenario frame");

        assert_eq!(report.label, "open-menu");
        assert_eq!(report.document.input_results.len(), 3);
        report
            .events
            .require_clicked(button)
            .expect("clicked button");
        report
            .events
            .require_focused(button)
            .expect("focused button");
        report
            .render_assertions()
            .require_target_kind(RenderTargetKind::Window)
            .expect("window target");
        report
            .render_assertions()
            .require_min_painted_items(2)
            .expect("painted items");
        report
            .render_assertions()
            .require_no_snapshot()
            .expect("no snapshot");
        report
            .timing_assertions()
            .require_sections([
                "pre-input-layout",
                "input",
                "document-frame",
                "render-frame",
                "platform-requests",
            ])
            .expect("scenario timing sections");
        report
            .platform_assertions()
            .require_no_error_responses()
            .expect("no platform errors");
        assert_eq!(harness.current_state().interaction.focused, Some(button));
    }

    #[test]
    fn scenario_frame_report_builds_debug_trace_and_capture_report() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        let root = document.root;
        let button = document.add_child(
            root,
            UiNode::container("menu.open", fixed_style(96.0, 32.0))
                .with_input(InputBehavior::BUTTON)
                .with_visual(UiVisual::panel(
                    ColorRgba::new(32, 44, 58, 255),
                    Some(StrokeStyle::new(ColorRgba::new(98, 128, 164, 255), 1.0)),
                    4.0,
                ))
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Open menu")
                        .focusable()
                        .action(AccessibilityAction::new("open", "Open")),
                ),
        );
        document.add_child(
            button,
            UiNode::text(
                "menu.open.label",
                "Open",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(fixed_style(52.0, 18.0).layout),
            ),
        );

        let mut harness = ScenarioHarness::new(UiSize::new(180.0, 100.0));
        let frame = harness
            .run_frame(
                "open-menu",
                &mut document,
                EventReplay::new().pointer_click("open", UiPoint::new(16.0, 16.0)),
            )
            .expect("scenario frame");
        let mut measurer = ApproxTextMeasurer;

        let trace = frame.debug_frame_trace_with_options(
            &document,
            &mut measurer,
            Some(Duration::from_nanos(0)),
            Some("menu.open"),
        );

        assert_eq!(trace.label, "open-menu");
        assert_eq!(trace.viewport, UiSize::new(180.0, 100.0));
        assert_eq!(trace.dirty.flags, frame.document.render_request.dirty_flags);
        assert_eq!(trace.routes.len(), 3);
        assert_eq!(trace.timing.budget, Some(Duration::from_nanos(0)));
        assert_eq!(trace.summary.route_count, 3);
        assert_eq!(trace.summary.selected_node.as_deref(), Some("menu.open"));
        assert_eq!(
            trace.selected_node.as_ref().map(|node| node.name.as_str()),
            Some("menu.open")
        );

        let report = frame.debug_capture_report_with_options(
            &document,
            &mut ApproxTextMeasurer,
            Some(Duration::from_nanos(0)),
            Some("menu.open"),
        );
        let markdown = report.to_markdown();

        assert_eq!(report.title, "Operad Debug Capture open-menu");
        assert_eq!(report.frame_label, "open-menu[0]");
        assert!(report
            .selected_node_summary
            .as_ref()
            .is_some_and(|summary| summary.contains("menu.open")));
        assert!(markdown.contains("# Operad Debug Capture open-menu"));
        assert!(markdown.contains("## Question guide"));
        assert!(markdown.contains("## Bottleneck triage"));
        assert!(markdown.contains("menu.open"));

        let contract = frame.debug_contract_report_with_options(
            &document,
            &mut ApproxTextMeasurer,
            Some(Duration::from_nanos(0)),
            Some("menu.open"),
        );
        let routes = contract
            .check(DebugContractCheckKind::EventRoutes)
            .expect("event-route contract check");
        let selected = contract
            .check(DebugContractCheckKind::SelectedNode)
            .expect("selected-node contract check");

        assert_eq!(contract.frame_label, "open-menu");
        assert_eq!(contract.check_count, 10);
        assert!(routes.checked);
        assert!(routes.summary.contains("3 event routes"));
        assert!(selected.checked);
        assert!(selected.summary.contains("menu.open"));

        let budget = frame.debug_frame_budget_trace_with_options(
            &document,
            &mut ApproxTextMeasurer,
            Some(Duration::from_nanos(0)),
            Some("menu.open"),
        );
        let issues = frame.debug_issue_report_with_options(
            &document,
            &mut ApproxTextMeasurer,
            Some(Duration::from_nanos(0)),
            Some("menu.open"),
        );
        let slow_frame = frame.debug_slow_frame_trace_with_options(
            &document,
            &mut ApproxTextMeasurer,
            Some(Duration::from_nanos(0)),
            Some("menu.open"),
        );
        let bottlenecks = frame.debug_frame_bottleneck_trace_with_options(
            &document,
            &mut ApproxTextMeasurer,
            Some(Duration::from_nanos(0)),
            Some("menu.open"),
        );
        let autopsy = frame.debug_frame_autopsy_trace_with_options(
            &document,
            &mut ApproxTextMeasurer,
            Some(Duration::from_nanos(0)),
            Some("menu.open"),
        );
        let why_frame = frame.debug_why_frame_with_options(
            &document,
            &mut ApproxTextMeasurer,
            Some(Duration::from_nanos(0)),
            Some("menu.open"),
        );

        assert_eq!(budget.status, DebugFrameTraceStatus::Warning);
        assert!(budget.over_budget_by.is_some());
        assert_eq!(issues.frame_label, "open-menu");
        assert!(issues.issue_count > 0);
        assert_eq!(slow_frame.frame_label, "open-menu");
        assert!(slow_frame.over_budget_by.is_some());
        assert!(slow_frame.record_count > 0);
        assert_eq!(bottlenecks.frame_label, "open-menu");
        assert!(bottlenecks.record_count > 0);
        assert!(bottlenecks.top_label.is_some());
        assert_eq!(autopsy.frame_label, "open-menu");
        assert_eq!(autopsy.record_count, 10);
        assert!(autopsy.summary.contains("frame autopsy"));
        assert_eq!(why_frame.scope, DebugWhyScope::Frame);
        assert!(why_frame.record(DebugWhySource::QuestionGuide).is_some());
        assert!(why_frame.record(DebugWhySource::Issue).is_some());
        assert!(why_frame
            .first_step()
            .is_some_and(|step| !step.action.is_empty()
                && !step.panel.is_empty()
                && !step.overlays.is_empty()));

        let inspect_node = frame
            .debug_inspect_node_trace(&document, &mut ApproxTextMeasurer, "menu.open")
            .expect("inspect selected node");
        let inspect_node_for_step = frame
            .debug_inspect_node_trace_for_step(
                &document,
                &mut ApproxTextMeasurer,
                "menu.open",
                "open.down",
            )
            .expect("inspect selected node for pointer step");
        let why_node = frame
            .debug_why_node_for_step(&document, &mut ApproxTextMeasurer, "menu.open", "open.down")
            .expect("why selected node for pointer step");

        assert_eq!(inspect_node.selected_name, "menu.open");
        assert!(inspect_node.summary.contains("inspect node"));
        assert!(inspect_node
            .record(DebugInspectNodeSource::NextStep)
            .is_some());
        assert!(inspect_node
            .record(DebugInspectNodeSource::Provenance)
            .is_some());
        assert_eq!(inspect_node_for_step.selected_name, "menu.open");
        assert!(inspect_node_for_step.explanation.route.is_some());
        assert_eq!(why_node.scope, DebugWhyScope::Node);
        assert_eq!(why_node.subject, "menu.open");
        assert!(why_node.record(DebugWhySource::InspectNode).is_some());
        assert!(why_node
            .first_step()
            .is_some_and(|step| step.subject == "menu.open" && !step.overlays.is_empty()));
        assert!(frame
            .debug_inspect_node_trace_for_step(
                &document,
                &mut ApproxTextMeasurer,
                "missing.node",
                "open.down",
            )
            .is_err());

        assert_eq!(
            frame.last_pointer_event(),
            Some((DebugPointerRouteKind::Up, UiPoint::new(16.0, 16.0)))
        );
        assert_eq!(
            frame.pointer_step("open.down").expect("named pointer step"),
            EventReplayPointerEvent {
                label: "open.down".to_owned(),
                event: DebugPointerRouteKind::Down,
                point: UiPoint::new(16.0, 16.0),
            }
        );
        assert!(frame.pointer_step("missing").is_err());
        assert!(frame.pointer_step("open").is_err());

        let down_autopsy = frame
            .debug_point_autopsy(&document, &mut ApproxTextMeasurer, "open.down")
            .expect("named pointer point autopsy");
        let all_autopsies = frame.debug_point_autopsies(&document, &mut ApproxTextMeasurer);
        let all_inspect_points = frame.debug_inspect_points(&document, &mut ApproxTextMeasurer);
        let point_autopsy = frame
            .debug_last_point_autopsy(&document, &mut ApproxTextMeasurer)
            .expect("last pointer point autopsy");
        let inspect_point = frame
            .debug_last_inspect_point(&document, &mut ApproxTextMeasurer)
            .expect("last pointer inspect point");
        let why_point = frame
            .debug_why_point(&document, &mut ApproxTextMeasurer, "open.down")
            .expect("named pointer why trace");
        let last_why_point = frame
            .debug_last_why_point(&document, &mut ApproxTextMeasurer)
            .expect("last pointer why trace");

        assert_eq!(down_autopsy.event, DebugPointerRouteKind::Down);
        assert_eq!(down_autopsy.target_name.as_deref(), Some("menu.open"));
        assert_eq!(
            all_autopsies
                .iter()
                .map(|diagnostic| diagnostic.label.as_str())
                .collect::<Vec<_>>(),
            vec!["open.move", "open.down", "open.up"]
        );
        assert!(all_autopsies
            .iter()
            .all(|diagnostic| diagnostic.trace.target_name.as_deref() == Some("menu.open")));
        assert_eq!(all_inspect_points.len(), 3);
        assert!(all_inspect_points
            .iter()
            .all(|diagnostic| diagnostic.trace.summary.contains("inspect point")));
        assert_eq!(point_autopsy.event, DebugPointerRouteKind::Up);
        assert_eq!(point_autopsy.target_name.as_deref(), Some("menu.open"));
        assert!(point_autopsy.summary.contains("point autopsy"));
        assert!(point_autopsy.pointer.summary.contains("pointer autopsy"));
        assert_eq!(inspect_point.target_name.as_deref(), Some("menu.open"));
        assert!(inspect_point.summary.contains("inspect point"));
        assert_eq!(why_point.scope, DebugWhyScope::Point);
        assert!(why_point.record(DebugWhySource::PointAutopsy).is_some());
        assert!(
            why_point.first_step().is_some_and(
                |step| step.action.contains("point-autopsy") && !step.overlays.is_empty()
            )
        );
        assert_eq!(last_why_point.scope, DebugWhyScope::Point);
    }

    #[test]
    fn scenario_frames_record_debug_timeline_and_node_history() {
        let mut document = UiDocument::new(root_style(220.0, 120.0));
        let root = document.root;
        let button = document.add_child(
            root,
            UiNode::container("menu.open", fixed_style(96.0, 32.0))
                .with_input(InputBehavior::BUTTON)
                .with_visual(UiVisual::panel(
                    ColorRgba::new(32, 44, 58, 255),
                    Some(StrokeStyle::new(ColorRgba::new(98, 128, 164, 255), 1.0)),
                    4.0,
                )),
        );
        let mut harness = ScenarioHarness::new(UiSize::new(220.0, 120.0));
        let mut recorder = DebugFrameRecorder::new(4);

        let first = harness
            .run_frame("menu", &mut document, EventReplay::new())
            .expect("first scenario frame");
        let recorded_first = first
            .record_debug_frame_with_options(
                &mut recorder,
                &document,
                &mut ApproxTextMeasurer,
                Some(1),
                Some(Duration::from_millis(16)),
                Some("menu.open"),
            )
            .expect("recorded first frame");
        assert_eq!(recorded_first.frame_index, Some(1));

        document.set_node_style(button, fixed_style(128.0, 40.0));
        let second = harness
            .run_frame("menu", &mut document, EventReplay::new())
            .expect("second scenario frame");
        second
            .record_debug_frame_with_options(
                &mut recorder,
                &document,
                &mut ApproxTextMeasurer,
                Some(2),
                Some(Duration::from_nanos(0)),
                Some("menu.open"),
            )
            .expect("recorded second frame");

        let summary = recorder.summary();
        let timeline = recorder.timeline();
        let history = recorder.node_history("menu.open");
        let changed = history.record("menu #2").expect("second history row");
        let latest_change = recorder
            .latest_node_change("menu.open")
            .expect("latest node change");
        let latest_bottlenecks = recorder
            .latest_frame_bottleneck_trace()
            .expect("latest frame bottlenecks");
        let latest_inspect = recorder
            .latest_inspect_node_trace("menu.open")
            .expect("latest inspect node");
        let report = recorder.capture_report().expect("latest capture report");

        assert_eq!(recorder.len(), 2);
        assert_eq!(summary.latest_frame_index, Some(2));
        assert_eq!(summary.latest_label.as_deref(), Some("menu #2"));
        assert_eq!(timeline.frame_count, 2);
        assert!(timeline.record("menu #1").is_some());
        assert!(timeline.record("menu #2").is_some());
        assert_eq!(history.frame_count, 2);
        assert_eq!(history.changed_frame_count, 1);
        assert!(changed.changed);
        assert!(changed.change.is_some());
        assert!(latest_change.changed);
        assert!(latest_change.record(DebugFrameChangeKind::Layout).is_some());
        assert_eq!(latest_bottlenecks.frame_index, Some(2));
        assert!(latest_bottlenecks.record_count > 0);
        assert_eq!(latest_inspect.selected_name, "menu.open");
        assert!(recorder.latest_inspect_node_trace("missing.node").is_none());
        assert_eq!(report.frame_index, Some(2));
        assert!(report
            .timeline_summary
            .as_ref()
            .is_some_and(|summary| summary.contains("frame timeline")));
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
            ))
            .with_shader(ShaderEffect::new("panel.surface")),
        );
        document.add_child(
            panel,
            UiNode::image(
                "panel.icon",
                ImageContent::new("icons.play"),
                LayoutStyle::from_taffy_style(fixed_style(24.0, 24.0).layout),
            ),
        );
        document.add_child(
            panel,
            UiNode::text(
                "panel.label",
                "Play",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
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
            .require_min_kind_count(PaintKindSelector::Rect, 1)
            .expect("rect paint count");
        assert!(paint
            .require_kind_count(PaintKindSelector::Text, 2)
            .is_err());
        assert!(!paint.node_items("panel").expect("panel paint").is_empty());
        paint
            .require_node_shader("panel", "panel.surface")
            .expect("panel shader");
        paint
            .require_node_kind("panel.icon", PaintKindSelector::Image)
            .expect("icon paint");
        paint
            .require_node_kind("panel.label", PaintKindSelector::Text)
            .expect("text paint");
    }

    #[test]
    fn layout_assertions_explain_invisible_nodes_with_visibility_trace() {
        let mut document = UiDocument::new(root_style(120.0, 80.0));
        document.add_child(
            document.root,
            UiNode::container(
                "offscreen.button",
                LayoutStyle::absolute_rect(UiRect::new(180.0, 16.0, 56.0, 28.0)),
            )
            .with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(120.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let error = LayoutAssertions::new(&document)
            .require_visible("offscreen.button")
            .expect_err("offscreen node should fail visibility assertion");

        assert!(error.message.contains("offscreen.button"));
        assert!(error.message.contains("issue: Hidden"));
        assert!(error.message.contains("cause:"));
        assert!(error.message.contains("fix:"));
        assert!(error.message.contains("visibility_panel"));
    }

    #[test]
    fn paint_assertions_explain_missing_node_paint_with_visibility_trace() {
        let mut document = UiDocument::new(root_style(140.0, 80.0));
        document.add_child(
            document.root,
            UiNode::container(
                "blank.button",
                LayoutStyle::absolute_rect(UiRect::new(16.0, 16.0, 80.0, 28.0)),
            )
            .with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(140.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let error = PaintAssertions::new(&document)
            .node_items("blank.button")
            .expect_err("blank button should fail paint assertion");

        assert!(error.message.contains("blank.button"));
        assert!(error.message.contains("issue: No paint"));
        assert!(error.message.contains("cause:"));
        assert!(error.message.contains("fix:"));
        assert!(error.message.contains("visibility_panel"));
    }

    #[test]
    fn layout_assertions_explain_hit_test_mismatches_with_pointer_probe() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        document.add_child(
            document.root,
            UiNode::container(
                "back.button",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 90.0, 32.0)).style,
                    z_index: 1.0,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON),
        );
        let front = document.add_child(
            document.root,
            UiNode::container(
                "front.button",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(50.0, 16.0, 90.0, 40.0)).style,
                    z_index: 10.0,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let layout = LayoutAssertions::new(&document);
        assert_eq!(
            layout
                .require_hit_center("front.button")
                .expect("front hit"),
            front
        );
        let error = layout
            .require_hit_center("back.button")
            .expect_err("front button should occlude back center");

        assert!(error.message.contains("expected point"));
        assert!(error.message.contains("back.button"));
        assert!(error.message.contains("got `front.button`"));
        assert!(error.message.contains("routed target: front.button"));
        assert!(error.message.contains("expected candidate:"));
        assert!(error.message.contains("cause:"));
        assert!(error.message.contains("fix:"));
        assert!(error.message.contains("pointer_autopsy_panel"));
    }

    #[test]
    fn audit_assertions_report_accessibility_gaps_by_stable_name() {
        let mut document = UiDocument::new(root_style(260.0, 120.0));
        let root = document.root;
        document.add_child(
            root,
            UiNode::container("missing_metadata", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON),
        );
        document.add_child(
            root,
            UiNode::container("unlabeled", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Button).focusable()),
        );
        let label = document.add_child(
            root,
            UiNode::text(
                "relation_label",
                "Relation label",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(fixed_style(80.0, 20.0).layout),
            ),
        );
        document.add_child(
            root,
            UiNode::container("relation_gap", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .labelled_by(label)
                        .action(AccessibilityAction::new("activate", "Activate"))
                        .focusable(),
                ),
        );
        document.add_child(
            root,
            UiNode::container("value_gap", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Slider)
                        .label("Missing value")
                        .action(AccessibilityAction::new("increase", "Increase"))
                        .action(AccessibilityAction::new("decrease", "Decrease"))
                        .focusable(),
                ),
        );
        document.add_child(
            root,
            UiNode::container("invalid_range", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Slider)
                        .label("Invalid range")
                        .value("50%")
                        .value_range(AccessibilityValueRange::new(10.0, 0.0))
                        .action(AccessibilityAction::new("increase", "Increase"))
                        .action(AccessibilityAction::new("decrease", "Decrease"))
                        .focusable(),
                ),
        );
        document.add_child(
            root,
            UiNode::container("action_id_gap", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Blank action id")
                        .action(AccessibilityAction::new(" ", "Activate"))
                        .focusable(),
                ),
        );
        document.add_child(
            root,
            UiNode::container("action_label_gap", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Blank action label")
                        .action(AccessibilityAction::new("activate", " "))
                        .focusable(),
                ),
        );
        document.add_child(
            root,
            UiNode::container("action_duplicate_gap", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Duplicate action")
                        .action(AccessibilityAction::new("activate", "Activate"))
                        .action(AccessibilityAction::new("activate", "Activate again"))
                        .focusable(),
                ),
        );
        document.add_child(
            root,
            UiNode::container("state_gap", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::ToggleButton)
                        .label("Missing pressed")
                        .action(AccessibilityAction::new("toggle", "Toggle"))
                        .focusable(),
                ),
        );
        document.add_child(
            root,
            UiNode::container("complete", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Complete")
                        .action(AccessibilityAction::new("activate", "Activate"))
                        .focusable(),
                ),
        );
        document.add_child(
            root,
            UiNode::container("complete_slider", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Slider)
                        .label("Complete slider")
                        .value("25%")
                        .value_range(AccessibilityValueRange::new(0.0, 100.0))
                        .action(AccessibilityAction::new("increase", "Increase"))
                        .action(AccessibilityAction::new("decrease", "Decrease"))
                        .focusable(),
                ),
        );
        document.add_child(
            root,
            UiNode::container("complete_toggle", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::ToggleButton)
                        .label("Complete toggle")
                        .pressed(false)
                        .action(AccessibilityAction::new("toggle", "Toggle"))
                        .focusable(),
                ),
        );
        document
            .compute_layout(UiSize::new(260.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let audit = AuditAssertions::new(&document);
        assert!(audit.require_no_warnings().is_err());
        assert!(audit.require_no_accessibility_warnings().is_err());
        audit
            .require_accessibility_metadata_gap("missing_metadata")
            .expect("missing metadata");
        audit
            .require_accessible_name_gap("unlabeled")
            .expect("missing name");
        audit
            .require_accessibility_action_gap("unlabeled")
            .expect("missing action");
        audit
            .require_relation_target_gap(
                "relation_gap",
                AccessibilityRelationKind::LabelledBy,
                "relation_label",
            )
            .expect("missing relation target");
        audit
            .require_accessibility_value_gap("value_gap")
            .expect("missing value");
        audit
            .require_accessibility_value_range_gap("value_gap")
            .expect("missing range");
        audit
            .require_accessibility_value_range_invalid_gap(
                "invalid_range",
                AccessibilityValueRangeIssue::Reversed,
            )
            .expect("invalid range");
        audit
            .require_accessibility_action_id_gap("action_id_gap")
            .expect("missing action id");
        audit
            .require_accessibility_action_label_gap("action_label_gap")
            .expect("missing action label");
        audit
            .require_accessibility_action_duplicate_gap("action_duplicate_gap", "activate")
            .expect("duplicate action id");
        audit
            .require_accessibility_state_gap("state_gap", AccessibilityStateKind::Pressed)
            .expect("missing pressed state");
        audit
            .require_no_accessible_name_gap("complete")
            .expect("complete label");
        audit
            .require_no_accessibility_metadata_gap("complete")
            .expect("complete metadata");
        audit
            .require_no_accessibility_action_gap("complete")
            .expect("complete action");
        audit
            .require_no_accessibility_action_id_gap("complete")
            .expect("complete action id");
        audit
            .require_no_accessibility_action_label_gap("complete")
            .expect("complete action label");
        audit
            .require_no_accessibility_action_duplicate_gap("complete")
            .expect("complete action ids");
        audit
            .require_no_relation_target_gap("complete")
            .expect("complete relations");
        audit
            .require_no_accessibility_value_gap("complete_slider")
            .expect("complete value");
        audit
            .require_no_accessibility_value_range_gap("complete_slider")
            .expect("complete range");
        audit
            .require_no_accessibility_value_range_invalid_gap("complete_slider")
            .expect("valid range");
        audit
            .require_no_accessibility_state_gap("complete_toggle")
            .expect("complete toggle state");
    }

    #[test]
    fn audit_assertions_report_text_contrast_by_stable_name() {
        let mut document = UiDocument::new(root_style(180.0, 80.0));
        document.node_mut(document.root).visual =
            UiVisual::panel(ColorRgba::new(24, 30, 38, 255), None, 0.0);
        document.add_child(
            document.root,
            UiNode::text(
                "low_contrast_text",
                "Low contrast",
                TextStyle {
                    font_size: 12.0,
                    line_height: 16.0,
                    color: ColorRgba::new(34, 40, 48, 255),
                    ..Default::default()
                },
                LayoutStyle::from_taffy_style(fixed_style(120.0, 20.0).layout),
            ),
        );
        document.add_child(
            document.root,
            UiNode::text(
                "readable_text",
                "Readable",
                TextStyle {
                    font_size: 12.0,
                    line_height: 16.0,
                    color: ColorRgba::new(232, 238, 246, 255),
                    ..Default::default()
                },
                LayoutStyle::from_taffy_style(fixed_style(120.0, 20.0).layout),
            ),
        );
        document
            .compute_layout(UiSize::new(180.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let audit = AuditAssertions::new(&document);
        audit
            .require_text_contrast_gap("low_contrast_text")
            .expect("low contrast");
        audit
            .require_no_text_contrast_gap("readable_text")
            .expect("readable contrast");
    }

    #[test]
    fn just_work_assertions_expose_blocking_edge_falloff_warnings() {
        let mut document = UiDocument::new(root_style(180.0, 90.0));
        let scroll = document.add_child(
            document.root,
            UiNode::container(
                "vertical.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(80.0),
                            height: length(40.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        document.add_child(
            scroll,
            UiNode::container(
                "wide.content",
                LayoutStyle::size(160.0, 40.0).with_flex_shrink(0.0),
            ),
        );
        document
            .compute_layout(UiSize::new(180.0, 90.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let audit = JustWorkAssertions::new(&document);
        let clean_error = audit
            .require_clean()
            .expect_err("scroll failure should produce Just Work summary");
        assert!(clean_error
            .message
            .contains("reason: scroll content extends beyond a disabled axis"));
        assert!(clean_error.message.contains("name: vertical.scroll"));
        assert!(clean_error.message.contains("measured: axis=Horizontal"));
        assert!(clean_error
            .message
            .contains("hint: enable scrolling on the overflowing axis"));
        assert!(audit.require_no_scroll_failures().is_err());
        audit
            .require_no_text_clipping()
            .expect("scroll issue is not text clipping");
        assert!(audit
            .blocking_warnings()
            .iter()
            .any(|warning| matches!(warning, AuditWarning::ScrollRangeHidden { name, .. } if name == "vertical.scroll")));
        assert!(audit.warnings().iter().any(is_blocking_audit_warning));
    }

    #[test]
    fn just_work_assertions_explain_interactive_overlap_failures() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        document.add_child(
            document.root,
            UiNode::container(
                "back.button",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 80.0, 32.0)).style,
                    z_index: 1.0,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON),
        );
        document.add_child(
            document.root,
            UiNode::container(
                "front.button",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(56.0, 24.0, 80.0, 32.0)).style,
                    z_index: 2.0,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let audit = JustWorkAssertions::new(&document);
        let error = audit
            .require_no_interactive_overlaps()
            .expect_err("overlapping buttons should fail");
        assert!(error.message.contains("interactive hitbox overlaps"));
        assert!(error.message.contains("back.button"));
        assert!(error.message.contains("front.button"));
        assert!(error.message.contains("both nodes are pointer-interactive"));
        assert!(error.message.contains("overlap_report_panel"));
    }

    #[test]
    fn just_work_assertions_explain_paint_hit_mismatch_failures() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        document.add_child(
            document.root,
            UiNode::container(
                "blank.button",
                LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 96.0, 32.0)),
            )
            .with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let audit = JustWorkAssertions::new(&document);
        let error = audit
            .require_no_paint_hit_mismatches(&mut ApproxTextMeasurer)
            .expect_err("blank button should fail paint/hit assertion");
        assert!(error.message.contains("paint/hit mismatches"));
        assert!(error.message.contains("blank.button"));
        assert!(error.message.contains("Hit without visible paint"));
        assert!(error.message.contains("cause:"));
        assert!(error.message.contains("fix:"));
    }

    #[test]
    fn render_assertions_check_canvas_image_handlers_and_interaction() {
        let mut document = UiDocument::new(root_style(240.0, 120.0));
        let root = document.root;
        let mut canvas = UiNode::canvas(
            "editor.viewport",
            "editor.viewport",
            LayoutStyle::from_taffy_style(fixed_style(120.0, 80.0).layout),
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
                LayoutStyle::from_taffy_style(fixed_style(48.0, 48.0).layout),
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
    fn canvas_hit_assertions_check_targets_topmost_and_accessibility() {
        let canvas = CanvasContent::new("editor.viewport").domain_hit_testing(true);
        let mut paint = PaintList::default();
        paint.items.push(PaintItem {
            node: UiNodeId(3),
            rect: UiRect::new(8.0, 10.0, 120.0, 64.0),
            clip_rect: UiRect::new(0.0, 0.0, 160.0, 120.0),
            z_index: 0.0,
            layer_order: crate::platform::LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            material: None,
            kind: PaintKind::Canvas(canvas),
        });
        let request = RenderFrameRequest::new(
            RenderTarget::snapshot(PixelSize::new(160, 120)),
            UiSize::new(160.0, 120.0),
            paint,
        );
        let mut registry: CanvasRenderRegistry<()> = CanvasRenderRegistry::new();
        registry.register(
            "editor.viewport",
            |_context: CanvasRenderContext<'_, ()>| {
                Ok(CanvasRenderOutput::new().hit_targets([
                    CanvasHitTarget::new("item.body", UiRect::new(10.0, 12.0, 60.0, 24.0))
                        .label("Item body")
                        .metadata("kind", "range")
                        .z_index(1.0),
                    CanvasHitTarget::new("disabled.overlay", UiRect::new(10.0, 12.0, 60.0, 24.0))
                        .label("Disabled overlay")
                        .disabled(true)
                        .z_index(10.0),
                    CanvasHitTarget::new("item.resize", UiRect::new(14.0, 12.0, 12.0, 24.0))
                        .label("Resize handle")
                        .value("start edge")
                        .z_index(4.0),
                ]))
            },
        );
        let report = registry.render_frame_canvases(&request, &mut ());
        let hits = CanvasHitAssertions::new(&report);

        hits.require_collection_count(1).expect("collection count");
        hits.require_collection_for_node(UiNodeId(3), "editor.viewport")
            .expect("node collection");
        hits.require_target_ids(
            "editor.viewport",
            &["item.body", "disabled.overlay", "item.resize"],
        )
        .expect("target ids");
        hits.require_target_metadata("editor.viewport", "item.body", "kind", "range")
            .expect("target metadata");
        hits.require_target_accessibility_label("editor.viewport", "item.resize", "Resize handle")
            .expect("accessibility label");
        hits.require_target_disabled("editor.viewport", "disabled.overlay", true)
            .expect("disabled target");
        hits.require_topmost_target_at("editor.viewport", UiPoint::new(16.0, 20.0), "item.resize")
            .expect("topmost target");

        assert!(hits.require_collection("missing.viewport").is_err());
        assert!(hits
            .require_target_ids("editor.viewport", &["item.body"])
            .is_err());
    }

    #[test]
    fn render_output_assertions_check_snapshots_timings_and_counts() {
        let mut output = RenderFrameOutput::new(RenderTarget::snapshot(PixelSize::new(2, 1)));
        output.painted_items = 3;
        output.batches = vec![PaintBatch {
            key: PaintBatchKey {
                kind: PaintBatchKind::Rect,
                z_index: 0.0,
                clip_rect: UiRect::new(0.0, 0.0, 2.0, 1.0),
                layer_order: crate::platform::LayerOrder::DEFAULT,
                shader: None,
            },
            item_indices: vec![0, 1, 2],
            bounds: UiRect::new(0.0, 0.0, 2.0, 1.0),
        }];
        output.timings = FrameTiming::new()
            .section("paint-build", Duration::from_millis(2))
            .section("render", Duration::from_millis(3));
        output.snapshot = Some(RenderedImage::new(
            PixelSize::new(2, 1),
            ResourceFormat::Rgba8,
            vec![0, 0, 0, 255, 12, 24, 36, 255],
        ));

        let assertions = RenderOutputAssertions::new(&output);
        assertions
            .require_target_kind(RenderTargetKind::Snapshot)
            .expect("snapshot target");
        assertions
            .require_painted_items(3)
            .expect("painted item count");
        assertions
            .require_min_painted_items(2)
            .expect("minimum painted items");
        assertions.require_batch_count(1).expect("batch count");
        assertions
            .require_min_batch_count(1)
            .expect("minimum batch count");
        assertions
            .timing_assertions()
            .require_section_within("render", Duration::from_millis(3))
            .expect("render timing");
        let snapshot = assertions
            .require_snapshot_rgba8("render-output")
            .expect("snapshot view");
        assert_eq!(snapshot.image().width, 2);
        snapshot
            .require_min_changed_pixels_from(ColorRgba::new(0, 0, 0, 255), 1)
            .expect("snapshot content");
        assert!(assertions.require_painted_items(4).is_err());
        assert!(assertions.require_no_snapshot().is_err());

        let window_output =
            RenderFrameOutput::new(RenderTarget::window("main", UiSize::new(24.0, 24.0)));
        RenderOutputAssertions::new(&window_output)
            .require_no_snapshot()
            .expect("window output has no snapshot");

        let mut bgra_output = RenderFrameOutput::new(RenderTarget::snapshot(PixelSize::new(1, 1)));
        bgra_output.snapshot = Some(RenderedImage::new(
            PixelSize::new(1, 1),
            ResourceFormat::Bgra8,
            vec![0, 0, 0, 255],
        ));
        assert!(RenderOutputAssertions::new(&bgra_output)
            .require_snapshot_rgba8("bgra")
            .is_err());
    }

    #[test]
    fn accessibility_assertions_use_stable_node_names() {
        let mut document = UiDocument::new(root_style(260.0, 120.0));
        let root = document.root;
        let title = document.add_child(
            root,
            UiNode::text(
                "choices.title",
                "Choices",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            )
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label("Choices")),
        );
        let hint = document.add_child(
            root,
            UiNode::text(
                "choices.hint",
                "Pick one option",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Tooltip).label("Pick one option"),
            ),
        );
        let list = document.add_child(
            root,
            UiNode::container("choices", fixed_style(160.0, 80.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::List)
                    .labelled_by(title)
                    .described_by(hint)
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
                    .shortcut("Enter")
                    .action(AccessibilityAction::new("select", "Select").shortcut("Enter"))
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
                .labelled_by(title)
                .described_by(hint)
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
            .require_action("choices.alpha", "select", "Select")
            .expect("option action");
        accessibility
            .require_action_shortcut("choices.alpha", "select", "Enter")
            .expect("option action shortcut");
        accessibility
            .require_key_shortcut("choices.alpha", "Enter")
            .expect("option key shortcut");
        accessibility
            .require_accessible_name("choices", "Choices")
            .expect("resolved list name");
        accessibility
            .require_accessible_description("choices", "Pick one option")
            .expect("resolved list description");
        accessibility
            .require_screen_reader_text_contains("choices", "Choices. 2 options. Pick one option")
            .expect("resolved list screen-reader text");
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
            .require_effective_focus_order(&["choices", "choices.alpha", "choices.beta"])
            .expect("effective focus order");
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
        let next = UiNodeId(8);
        let requests = vec![
            AccessibilityAdapterRequest::PublishTree {
                tree: tree.clone(),
                focused: Some(focused),
                preferences,
            },
            AccessibilityAdapterRequest::ApplyPreferences(preferences),
            AccessibilityAdapterRequest::MoveFocus {
                target: next,
                restore: FocusRestoreTarget::Previous,
            },
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
        assertions
            .require_move_focus(next, FocusRestoreTarget::Previous)
            .expect("move focus request");
        let announcement = assertions
            .require_announcement_contains("complete")
            .expect("announcement text");
        assert_eq!(announcement.source, Some(focused));
        assert!(assertions.require_announcement_contains("missing").is_err());

        let responses = vec![
            AccessibilityAdapterResponse::Applied,
            AccessibilityAdapterResponse::Unsupported(AccessibilityRequestKind::PublishTree),
            AccessibilityAdapterResponse::Failed {
                request: AccessibilityRequestKind::Announce,
                reason: "muted".to_string(),
            },
        ];
        let response_assertions = AccessibilityResponseAssertions::new(&responses);
        assert_eq!(
            response_assertions.response_count(AccessibilityRequestKind::PublishTree),
            1
        );
        assert_eq!(
            response_assertions.response_count(AccessibilityRequestKind::Announce),
            1
        );
        response_assertions
            .require_unsupported(AccessibilityRequestKind::PublishTree)
            .expect("unsupported publish response");
        assert!(response_assertions.require_no_unsupported().is_err());
        AccessibilityResponseAssertions::new(&[AccessibilityAdapterResponse::Applied])
            .require_no_unsupported()
            .expect("no unsupported accessibility responses");
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
        platform
            .require_all_requests_have_responses()
            .expect("answered requests");
        platform.require_no_error_responses().expect("no errors");

        let unmatched = HostFrameOutput::new(HostInteractionState::default()).response(
            PlatformRequestId::new(99),
            PlatformResponse::Repaint(RepaintResponse::Coalesced),
        );
        assert!(PlatformAssertions::from_host_frame(&unmatched)
            .require_all_responses_match_requests()
            .is_err());

        let missing_response = HostFrameOutput::new(HostInteractionState::default()).request(
            PlatformRequestId::new(100),
            PlatformRequest::Clipboard(ClipboardRequest::ReadText),
        );
        assert!(PlatformAssertions::from_host_frame(&missing_response)
            .require_all_requests_have_responses()
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
        let error_message = PlatformAssertions::from_host_frame(&error_output)
            .require_no_error_responses()
            .expect_err("denied clipboard should fail")
            .to_string();
        assert!(error_message.contains("next_step"), "{error_message}");
        assert!(
            error_message.contains("clipboard blocked"),
            "{error_message}"
        );
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
        let unsupported_message = unsupported_platform
            .require_no_unsupported_responses()
            .expect_err("unsupported clipboard should fail")
            .to_string();
        assert!(
            unsupported_message.contains("platform_request_id: 22"),
            "{unsupported_message}"
        );
        assert!(
            unsupported_message.contains("next_step"),
            "{unsupported_message}"
        );

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
            UiNode::canvas(
                "viewport",
                "app.viewport",
                LayoutStyle::from_taffy_style(fixed_style(120.0, 80.0).layout),
            ),
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
            PlatformRequest::Cursor(CursorRequest::SetGrab(CursorGrabMode::Locked))
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
    fn snapshot_assertions_hash_content_and_tolerance() {
        let expected = [0, 0, 0, 255, 10, 20, 30, 255];
        let actual = [0, 0, 0, 255, 11, 20, 30, 255];
        let view = RgbaImageView::new(2, 1, &actual).expect("actual view");
        let snapshot = SnapshotAssertions::new("snapshot", view);
        let hash = snapshot.hash();

        snapshot.require_hash(hash).expect("matching snapshot hash");
        assert!(snapshot.require_hash(0).is_err());
        assert_eq!(
            snapshot
                .require_min_changed_pixels_from(ColorRgba::new(0, 0, 0, 255), 1)
                .expect("changed pixels"),
            1
        );
        assert!(snapshot
            .require_min_changed_pixels_from(ColorRgba::new(0, 0, 0, 255), 2)
            .is_err());

        let report = snapshot
            .require_matches(
                RgbaImageView::new(2, 1, &expected).expect("expected view"),
                PixelDiffTolerance {
                    max_changed_pixels: 1,
                    max_channel_delta: 1,
                    max_total_channel_delta: 1,
                },
            )
            .expect("within tolerance");
        assert_eq!(report.changed_pixels, 1);
        assert!(snapshot
            .require_matches(
                RgbaImageView::new(2, 1, &expected).expect("expected view"),
                PixelDiffTolerance::EXACT,
            )
            .is_err());
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
        assert_eq!(timing.section_fraction("paint"), Some(4.0_f64 / 15.0_f64));
        assert_eq!(
            timing.dominant_section().expect("dominant section").name,
            "render"
        );
        assert!(timing.within_budget(Duration::from_millis(16)));
        assert!(!timing.within_budget(Duration::from_millis(10)));

        let assertions = FrameTimingAssertions::new(&timing);
        assert_eq!(
            assertions
                .require_sections(["layout", "paint", "render"])
                .expect("required sections"),
            vec![
                Duration::from_millis(3),
                Duration::from_millis(4),
                Duration::from_millis(8)
            ]
        );
        assert!(assertions
            .require_total_within(Duration::from_millis(16))
            .is_ok());
        let timing_error = assertions
            .require_total_within(Duration::from_millis(10))
            .expect_err("over-budget timing should explain slowest stage");
        assert!(timing_error.message.contains("slowest stage: backend-draw"));
        assert!(timing_error.message.contains("grouped stages:"));
        assert!(timing_error.message.contains("missing instrumentation:"));
        assert!(timing_error.message.contains("frame_bottleneck_panel"));
        assert!(assertions
            .require_section_within("paint", Duration::from_millis(4))
            .is_ok());
        let section_error = assertions
            .require_section_within("paint", Duration::from_millis(3))
            .expect_err("section budget should explain pipeline stage");
        assert!(section_error.message.contains("pipeline stage paint-list"));
        assert!(section_error.message.contains("frame_timing_panel"));
        assert!(assertions.require_section("input").is_err());

        let series = FrameTimingSeries::new("scenario")
            .frame(timing.clone())
            .frame(
                FrameTiming::new()
                    .section("layout", Duration::from_millis(2))
                    .section("paint", Duration::from_millis(6))
                    .section("render", Duration::from_millis(7)),
            );
        assert_eq!(series.len(), 2);
        assert_eq!(series.section_names(), vec!["layout", "paint", "render"]);
        assert_eq!(series.total_samples().samples().len(), 2);
        assert_eq!(series.section_samples("layout").samples().len(), 2);
        assert_eq!(series.section_samples("input").samples().len(), 0);
        let dominant = series.dominant_section().expect("series dominant section");
        assert_eq!(dominant.name, "render");
        assert_eq!(dominant.sample_count, 2);
        assert_eq!(dominant.total, Duration::from_millis(15));
        assert!(dominant.diagnostic_summary().contains("share=50.0%"));

        let series_assertions = FrameTimingSeriesAssertions::new(&series);
        series_assertions
            .require_frame_count(2)
            .expect("frame count");
        series_assertions
            .require_dominant_section("render")
            .expect("dominant section");
        series_assertions
            .require_section_sample_count("paint", 2)
            .expect("paint sample count");
        series_assertions
            .require_total_average_within(Duration::from_millis(15))
            .expect("total average budget");
        series_assertions
            .require_total_max_within(Duration::from_millis(15))
            .expect("total max budget");
        series_assertions
            .require_total_percentile_within(95.0, Duration::from_millis(15))
            .expect("total percentile budget");
        series_assertions
            .require_section_average_within("paint", Duration::from_millis(5))
            .expect("paint average budget");
        series_assertions
            .require_section_max_within("paint", Duration::from_millis(6))
            .expect("paint max budget");
        series_assertions
            .require_section_percentile_within("paint", 90.0, Duration::from_millis(6))
            .expect("paint percentile budget");
        assert!(series_assertions.require_frame_count(3).is_err());
        assert!(series_assertions
            .require_section_sample_count("input", 1)
            .is_err());
        assert!(series_assertions
            .require_section_average_within("paint", Duration::from_millis(4))
            .is_err());
        assert!(series_assertions
            .require_section_max_within("paint", Duration::from_millis(5))
            .is_err());
        assert!(series_assertions
            .require_section_percentile_within("paint", 90.0, Duration::from_millis(5))
            .is_err());
        let average_error = series_assertions
            .require_total_average_within(Duration::from_millis(14))
            .expect_err("average budget should include dominant section");
        assert!(average_error.message.contains("dominant section: render"));
        assert!(series_assertions
            .require_dominant_section("paint")
            .expect_err("wrong dominant section")
            .message
            .contains("got render"));

        let mut samples = PerformanceSamples::new("render smoke");
        samples.push(Duration::from_millis(4));
        samples.push(Duration::from_millis(6));
        samples.push(Duration::from_millis(5));
        assert_eq!(samples.len(), 3);
        assert_eq!(samples.total(), Duration::from_millis(15));
        assert_eq!(samples.max_sample(), Some(Duration::from_millis(6)));
        assert_eq!(samples.average(), Some(Duration::from_millis(5)));
        assert_eq!(samples.percentile(0.0), Some(Duration::from_millis(4)));
        assert_eq!(samples.percentile(50.0), Some(Duration::from_millis(5)));
        assert_eq!(samples.percentile(95.0), Some(Duration::from_millis(6)));
        assert_eq!(samples.percentile(f64::NAN), None);

        let performance = PerformanceAssertions::new(&samples);
        performance.require_sample_count(3).expect("sample count");
        performance
            .require_min_sample_count(2)
            .expect("minimum sample count");
        performance
            .require_total_within(Duration::from_millis(16))
            .expect("total budget");
        performance
            .require_average_within(Duration::from_millis(5))
            .expect("average budget");
        performance
            .require_max_sample_within(Duration::from_millis(6))
            .expect("max sample budget");
        performance
            .require_percentile_within(95.0, Duration::from_millis(6))
            .expect("percentile budget");
        assert!(performance.require_sample_count(4).is_err());
        assert!(performance
            .require_total_within(Duration::from_millis(14))
            .is_err());
        assert!(performance
            .require_average_within(Duration::from_millis(4))
            .is_err());
        assert!(performance
            .require_max_sample_within(Duration::from_millis(5))
            .is_err());
        assert!(performance
            .require_percentile_within(95.0, Duration::from_millis(5))
            .is_err());
        assert!(performance
            .require_percentile_within(f64::INFINITY, Duration::from_millis(5))
            .is_err());
        assert!(
            PerformanceAssertions::new(&PerformanceSamples::new("empty"))
                .require_average_within(Duration::from_millis(1))
                .is_err()
        );
    }

    #[test]
    fn display_list_reuse_assertions_cover_hits_misses_and_invalidation() {
        use crate::display::{
            DisplayListInvalidation, DisplayListInvalidationRequest, DisplayListKind,
            DisplayListScope, RetainedDisplayListCache,
        };

        fn retained_paint(items: usize) -> PaintList {
            PaintList {
                items: (0..items)
                    .map(|index| PaintItem {
                        node: UiNodeId(index),
                        rect: UiRect::new(index as f32, 0.0, 1.0, 1.0),
                        clip_rect: UiRect::new(0.0, 0.0, 16.0, 16.0),
                        z_index: 0.0,
                        layer_order: crate::platform::LayerOrder::DEFAULT,
                        opacity: 1.0,
                        transform: PaintTransform::default(),
                        shader: None,
                        material: None,
                        kind: PaintKind::Rect {
                            fill: ColorRgba::new(16, 24, 32, 255),
                            stroke: None,
                            corner_radius: 0.0,
                        },
                    })
                    .collect(),
            }
        }

        let input_dirty = DirtyFlags {
            input: true,
            ..DirtyFlags::NONE
        };
        let paint_dirty = DirtyFlags {
            paint: true,
            ..DirtyFlags::NONE
        };
        let mut cache = RetainedDisplayListCache::new();
        let key = DisplayListKey::editor_background("scenario-editor", 7);
        cache.insert(
            key.clone(),
            DisplayListKind::StaticBackground,
            DisplayListInvalidation::STATIC_EDITOR_BACKGROUND,
            retained_paint(3),
        );

        let reused = cache.reuse_report(&key, input_dirty);
        DisplayListReuseAssertions::new(&reused)
            .require_reused()
            .expect("reused display list");
        DisplayListReuseAssertions::new(&reused)
            .require_item_count(3)
            .expect("item count");

        let dirty_miss = cache.reuse_report(&key, paint_dirty);
        DisplayListReuseAssertions::new(&dirty_miss)
            .require_miss_dirty()
            .expect("dirty miss");

        let absent = cache.reuse_report(
            &DisplayListKey::new(DisplayListScope::Document, "missing", 0),
            DirtyFlags::NONE,
        );
        DisplayListReuseAssertions::new(&absent)
            .require_miss_absent()
            .expect("absent miss");

        let invalidation =
            cache.invalidate_with_report(DisplayListInvalidationRequest::Dirty(paint_dirty));
        let invalidation_assertions = DisplayListInvalidationAssertions::new(&invalidation);
        invalidation_assertions
            .require_removed_count(1)
            .expect("removed count");
        invalidation_assertions
            .require_removed_key(&key)
            .expect("removed key");
        invalidation_assertions
            .require_after_len(0)
            .expect("after length");

        let mut evicting_cache = RetainedDisplayListCache::with_capacity_limit(1);
        let first = DisplayListKey::new(DisplayListScope::custom("cache"), "first", 0);
        let second = DisplayListKey::new(DisplayListScope::custom("cache"), "second", 0);
        evicting_cache.insert(
            first.clone(),
            DisplayListKind::StaticPanel,
            DisplayListInvalidation::STATIC_PANEL,
            retained_paint(1),
        );
        evicting_cache.advance_frame();
        evicting_cache.insert(
            second,
            DisplayListKind::StaticPanel,
            DisplayListInvalidation::STATIC_PANEL,
            retained_paint(1),
        );
        let evicted = evicting_cache.reuse_report(&first, DirtyFlags::NONE);
        DisplayListReuseAssertions::new(&evicted)
            .require_miss_evicted()
            .expect("evicted miss");
    }

    #[test]
    fn display_list_reuse_series_assertions_track_multi_frame_outcomes() {
        let key = DisplayListKey::editor_background("grid", 1);
        let other = DisplayListKey::editor_background("overlay", 1);
        let reports = DisplayListReuseSeries::new("display-list reuse")
            .report(DisplayListReuseReport {
                key: key.clone(),
                outcome: DisplayListReuseOutcome::MissAbsent,
                dirty_flags: DirtyFlags::NONE,
                frame: 0,
                kind: None,
                invalidation: None,
                item_count: None,
                created_frame: None,
                last_used_frame: None,
            })
            .report(DisplayListReuseReport {
                key: key.clone(),
                outcome: DisplayListReuseOutcome::Reused,
                dirty_flags: DirtyFlags {
                    input: true,
                    ..DirtyFlags::NONE
                },
                frame: 1,
                kind: None,
                invalidation: None,
                item_count: Some(4),
                created_frame: Some(0),
                last_used_frame: Some(1),
            })
            .report(DisplayListReuseReport {
                key: key.clone(),
                outcome: DisplayListReuseOutcome::Reused,
                dirty_flags: DirtyFlags {
                    input: true,
                    ..DirtyFlags::NONE
                },
                frame: 2,
                kind: None,
                invalidation: None,
                item_count: Some(4),
                created_frame: Some(0),
                last_used_frame: Some(2),
            })
            .report(DisplayListReuseReport {
                key: other,
                outcome: DisplayListReuseOutcome::MissDirty,
                dirty_flags: DirtyFlags {
                    paint: true,
                    ..DirtyFlags::NONE
                },
                frame: 2,
                kind: None,
                invalidation: None,
                item_count: Some(1),
                created_frame: Some(0),
                last_used_frame: Some(0),
            });

        assert_eq!(reports.len(), 4);
        assert_eq!(reports.reused_count(), 2);
        assert_eq!(reports.missed_count(), 2);
        assert_eq!(reports.reuse_rate(), Some(0.5));

        let assertions = DisplayListReuseSeriesAssertions::new(&reports);
        assertions.require_report_count(4).expect("report count");
        assertions
            .require_outcome_count(DisplayListReuseOutcome::Reused, 2)
            .expect("reused count");
        assertions.require_min_reused(2).expect("min reused");
        assertions
            .require_reuse_rate_at_least(0.5)
            .expect("reuse rate");
        assertions
            .require_key_outcome_count(&key, DisplayListReuseOutcome::Reused, 2)
            .expect("key reused count");
        assertions.require_no_evictions().expect("no evictions");
        assert!(assertions.require_report_count(5).is_err());
        assert!(assertions.require_min_reused(3).is_err());
        assert!(assertions.require_reuse_rate_at_least(0.75).is_err());
        assert!(assertions
            .require_key_outcome_count(&key, DisplayListReuseOutcome::MissDirty, 1)
            .is_err());
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
            Some(UiInputEvent::Wheel(
                crate::UiWheelEvent::pixels(UiPoint::new(1.0, 1.0), UiPoint::new(0.0, 20.0))
                    .unit(WheelDeltaUnit::Line)
            ))
        );
        assert!(report.steps[1].converted.is_none());
        assert!(report.require_all_converted().is_err());
    }
}
