//! Renderer-neutral testing helpers for Operad documents.
//!
//! These utilities are intended for consumers as well as Operad's own tests:
//! replay input without an egui harness, assert layout by stable node names,
//! inspect paint lists, diff rgba snapshots with tolerances, and track simple
//! frame timing sections.

use std::borrow::Cow;
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::accessibility::{
    AccessibilityAdapterRequest, AccessibilityAdapterResponse, AccessibilityAnnouncement,
    AccessibilityPreferences, AccessibilityRequestKind, FocusRestoreTarget,
};
use crate::commands::{CommandId, CommandRegistry};
use crate::host::{
    HostCommandDispatch, HostDocumentFrameOutput, HostFrameOutput, HostInteractionState,
    HostNodeInteraction, HostShortcutRoute,
};
use crate::platform::{
    AppLifecycleResponse, BackendAdapterKind, BackendCapabilities, ClipboardResponse,
    CursorResponse, DragDropResponse, FileDialogResponse, LayerCapabilities, NotificationResponse,
    OpenUrlResponse, PixelSize, PlatformRequestIdAllocator, PlatformResponse, PlatformServiceError,
    PlatformServiceKind, PlatformServiceRequest, PlatformServiceResponse, RenderingCapabilities,
    RepaintResponse, ResourceCapabilities, ScreenshotResponse, TextImeResponse,
};
use crate::renderer::{
    CanvasHitCollection, CanvasHitTarget, CanvasRenderRegistry, CanvasRenderReport,
    CanvasRenderRequest, ImageRenderRegistry, ImageRenderRequest, RenderError, RenderFrameOutput,
    RenderFrameRequest, RenderTarget, RenderTargetKind, RenderedImage, RendererAdapter,
    ResourceFormat, ResourceResolver,
};
use crate::{
    AccessibilityLiveRegion, AccessibilityNode, AccessibilityRelationKind, AccessibilityRole,
    AccessibilityStateKind, AccessibilityTree, ApproxTextMeasurer, AuditWarning, CanvasContent,
    ColorRgba, FocusDirection, KeyCode, KeyModifiers, PaintItem, PaintKind, PaintList,
    PaintTransform, PathVerb, RawInputEvent, StrokeStyle, TextContent, UiDocument, UiInputEvent,
    UiInputResult, UiNode, UiNodeId, UiPoint, UiRect, UiSize,
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

    pub fn require_clicked(&self, node: UiNodeId) -> TestResult {
        require_replay_node("clicked", node, self.clicked_nodes())
    }

    pub fn require_focused(&self, node: UiNodeId) -> TestResult {
        require_replay_node("focused", node, self.focused_nodes())
    }

    pub fn require_scrolled(&self, node: UiNodeId) -> TestResult {
        require_replay_node("scrolled", node, self.scrolled_nodes())
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

fn is_accessibility_audit_warning(warning: &AuditWarning) -> bool {
    matches!(
        warning,
        AuditWarning::AccessibleNameMissing { .. }
            | AuditWarning::AccessibilityActionMissing { .. }
            | AuditWarning::AccessibilityActionIdMissing { .. }
            | AuditWarning::AccessibilityActionLabelMissing { .. }
            | AuditWarning::AccessibilityActionDuplicate { .. }
            | AuditWarning::AccessibilityStateMissing { .. }
            | AuditWarning::AccessibilityValueMissing { .. }
            | AuditWarning::AccessibilityValueRangeMissing { .. }
            | AuditWarning::AccessibilityRelationTargetMissing { .. }
            | AuditWarning::FocusableMissingFromAccessibilityTree { .. }
    )
}

fn warning_node(warning: &AuditWarning) -> Option<UiNodeId> {
    match warning {
        AuditWarning::NonFiniteRect { node, .. }
        | AuditWarning::InvisibleInteractiveNode { node, .. }
        | AuditWarning::EmptyInteractiveClip { node, .. }
        | AuditWarning::InteractiveTooSmall { node, .. }
        | AuditWarning::FocusableMissingFromAccessibilityTree { node, .. }
        | AuditWarning::AccessibleNameMissing { node, .. }
        | AuditWarning::AccessibilityActionMissing { node, .. }
        | AuditWarning::AccessibilityActionIdMissing { node, .. }
        | AuditWarning::AccessibilityActionLabelMissing { node, .. }
        | AuditWarning::AccessibilityActionDuplicate { node, .. }
        | AuditWarning::AccessibilityStateMissing { node, .. }
        | AuditWarning::AccessibilityValueMissing { node, .. }
        | AuditWarning::AccessibilityValueRangeMissing { node, .. }
        | AuditWarning::AccessibilityRelationTargetMissing { node, .. }
        | AuditWarning::TextClipped { node, .. }
        | AuditWarning::NodeOutsideRoot { node, .. }
        | AuditWarning::PaintItemEmptyClip { node } => Some(*node),
        AuditWarning::DuplicateNodeName { .. } => None,
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
            Err(TestFailure::new(format!(
                "node `{node_name}` has no paint items"
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

    pub fn require_all_requests_have_responses(&self) -> TestResult {
        for request in self.requests.iter() {
            if !self
                .responses
                .iter()
                .any(|response| response.is_for(request) && response.kind() == request.kind())
            {
                return Err(TestFailure::new(format!(
                    "platform request id {} kind {:?} has no matching response",
                    request.id.0,
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

pub const DEFAULT_CPU_SNAPSHOT_BACKGROUND: ColorRgba = ColorRgba::new(9, 12, 16, 255);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuSnapshotImage {
    pub size: PixelSize,
    pub pixels: Vec<u8>,
}

impl CpuSnapshotImage {
    pub fn new(size: PixelSize, background: ColorRgba) -> Self {
        let len = usize::try_from(size.width)
            .ok()
            .and_then(|width| {
                usize::try_from(size.height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
            .unwrap_or(0);
        let mut pixels = vec![0; len];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[0] = background.r;
            pixel[1] = background.g;
            pixel[2] = background.b;
            pixel[3] = background.a;
        }
        Self { size, pixels }
    }

    pub fn width(&self) -> usize {
        self.size.width as usize
    }

    pub fn height(&self) -> usize {
        self.size.height as usize
    }

    pub fn view(&self) -> TestResult<RgbaImageView<'_>> {
        RgbaImageView::new(self.width(), self.height(), &self.pixels)
    }

    pub fn hash(&self) -> u64 {
        self.view()
            .expect("CpuSnapshotImage should always contain valid RGBA pixels")
            .hash()
    }

    pub fn changed_pixels_from(&self, color: ColorRgba) -> usize {
        self.view()
            .expect("CpuSnapshotImage should always contain valid RGBA pixels")
            .changed_pixels_from(color)
    }

    pub fn write_ppm(&self, path: impl AsRef<Path>) -> TestResult {
        let mut data = format!("P6\n{} {}\n255\n", self.size.width, self.size.height).into_bytes();
        for pixel in self.pixels.chunks_exact(4) {
            data.extend_from_slice(&pixel[..3]);
        }
        fs::write(path, data)
            .map_err(|error| TestFailure::new(format!("write snapshot ppm: {error}")))
    }

    pub fn into_rendered_image(self) -> RenderedImage {
        RenderedImage::new(self.size, ResourceFormat::Rgba8, self.pixels)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuSnapshotRenderer {
    pub background: ColorRgba,
}

impl CpuSnapshotRenderer {
    pub const fn new(background: ColorRgba) -> Self {
        Self { background }
    }

    pub fn render_document(
        &self,
        document: &mut UiDocument,
        viewport: UiSize,
    ) -> TestResult<CpuSnapshotImage> {
        document
            .compute_layout(viewport, &mut ApproxTextMeasurer)
            .map_err(|error| TestFailure::new(format!("layout failed: {error:?}")))?;
        let size = pixel_size_from_viewport(viewport)?;
        self.render_paint_list(&document.paint_list(), size)
    }

    pub fn render_paint_list(
        &self,
        paint: &PaintList,
        size: PixelSize,
    ) -> TestResult<CpuSnapshotImage> {
        let mut image = CpuSnapshotImage::new(size, self.background);
        for item in &paint.items {
            draw_cpu_snapshot_item(&mut image, item);
        }
        image.view()?;
        Ok(image)
    }

    pub fn render_request(&self, request: &RenderFrameRequest) -> TestResult<CpuSnapshotImage> {
        let size = render_target_pixel_size(&request.target, request.viewport)?;
        self.render_paint_list(&request.paint, size)
    }

    pub fn snapshot_assertions<'a>(
        &self,
        name: impl Into<String>,
        image: &'a CpuSnapshotImage,
    ) -> TestResult<SnapshotAssertions<'a>> {
        Ok(SnapshotAssertions::new(name, image.view()?))
    }
}

impl Default for CpuSnapshotRenderer {
    fn default() -> Self {
        Self::new(DEFAULT_CPU_SNAPSHOT_BACKGROUND)
    }
}

impl RendererAdapter for CpuSnapshotRenderer {
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::new("cpu-snapshot")
            .adapter(BackendAdapterKind::CpuSnapshot)
            .resources(ResourceCapabilities {
                images: true,
                icons: true,
                thumbnails: true,
                tinted_icons: true,
                ..ResourceCapabilities::NONE
            })
            .layers(LayerCapabilities::STANDARD)
            .rendering(RenderingCapabilities {
                high_dpi: false,
                offscreen: true,
                deterministic_snapshots: true,
                partial_updates: false,
            })
    }

    fn render_frame(
        &mut self,
        request: RenderFrameRequest,
        _resolver: &dyn ResourceResolver,
    ) -> Result<RenderFrameOutput, RenderError> {
        let batches = request.batches();
        let painted_items = request.paint.items.len();
        let dirty_regions = request.dirty_regions.clone();
        let snapshot = self
            .render_request(&request)
            .map_err(|failure| RenderError::Backend(failure.message))?
            .into_rendered_image();
        let mut output = RenderFrameOutput::new(request.target);
        output.painted_items = painted_items;
        output.batches = batches;
        output.dirty_regions = dirty_regions;
        output.snapshot = Some(snapshot);
        Ok(output)
    }
}

fn pixel_size_from_viewport(viewport: UiSize) -> TestResult<PixelSize> {
    if !viewport.width.is_finite() || !viewport.height.is_finite() {
        return Err(TestFailure::new("snapshot viewport must be finite"));
    }
    if viewport.width < 0.0 || viewport.height < 0.0 {
        return Err(TestFailure::new("snapshot viewport must be non-negative"));
    }
    if viewport.width.round() > u32::MAX as f32 || viewport.height.round() > u32::MAX as f32 {
        return Err(TestFailure::new(
            "snapshot viewport exceeds u32 pixel dimensions",
        ));
    }
    Ok(PixelSize::new(
        viewport.width.round() as u32,
        viewport.height.round() as u32,
    ))
}

fn render_target_pixel_size(target: &RenderTarget, viewport: UiSize) -> TestResult<PixelSize> {
    match target {
        RenderTarget::Offscreen { size, .. } | RenderTarget::Snapshot { size, .. } => Ok(*size),
        RenderTarget::Window { size, .. } | RenderTarget::AppOwned { size, .. } => {
            pixel_size_from_viewport(*size)
        }
    }
    .or_else(|_| pixel_size_from_viewport(viewport))
}

fn draw_cpu_snapshot_item(image: &mut CpuSnapshotImage, item: &PaintItem) {
    let clip = item.clip_rect;
    match &item.kind {
        PaintKind::Rect {
            fill,
            stroke,
            corner_radius: _,
        } => {
            let rect = cpu_snapshot_transform_rect(item.rect, item.transform);
            cpu_snapshot_fill_rect(image, rect, clip, *fill, item.opacity);
            if let Some(stroke) = stroke {
                cpu_snapshot_stroke_rect(image, rect, clip, *stroke, item.opacity);
            }
        }
        PaintKind::RichRect(rect_primitive) => {
            let rect = cpu_snapshot_transform_rect(rect_primitive.rect, item.transform);
            for effect in &rect_primitive.effects {
                let spread = effect.spread.max(0.0) + effect.blur_radius.max(0.0) * 0.25;
                let effect_rect = UiRect::new(
                    rect.x + effect.offset.x - spread,
                    rect.y + effect.offset.y - spread,
                    rect.width + spread * 2.0,
                    rect.height + spread * 2.0,
                );
                cpu_snapshot_fill_rect(image, effect_rect, clip, effect.color, item.opacity);
            }
            cpu_snapshot_fill_rect(
                image,
                rect,
                clip,
                rect_primitive.fill.fallback_color(),
                item.opacity,
            );
            if let Some(stroke) = rect_primitive.stroke {
                cpu_snapshot_stroke_rect(image, rect, clip, stroke.style, item.opacity);
            }
        }
        PaintKind::Text(text) => cpu_snapshot_draw_text(image, item, text),
        PaintKind::SceneText(text) => {
            let text_content = TextContent::new(text.text.clone(), text.style.clone());
            let item = PaintItem {
                rect: text.rect,
                kind: PaintKind::Text(text_content.clone()),
                ..(*item).clone()
            };
            cpu_snapshot_draw_text(image, &item, &text_content);
        }
        PaintKind::Canvas(canvas) => cpu_snapshot_draw_canvas(image, item, canvas),
        PaintKind::Line { from, to, stroke } => {
            cpu_snapshot_draw_line(
                image,
                cpu_snapshot_transform_point(*from, item.transform),
                cpu_snapshot_transform_point(*to, item.transform),
                clip,
                *stroke,
                item.opacity,
            );
        }
        PaintKind::Circle {
            center,
            radius,
            fill,
            stroke,
        } => {
            let center = cpu_snapshot_transform_point(*center, item.transform);
            let radius = radius * item.transform.scale.max(0.0);
            cpu_snapshot_fill_circle(image, center, radius, clip, *fill, item.opacity);
            if let Some(stroke) = stroke {
                cpu_snapshot_stroke_circle(image, center, radius, clip, *stroke, item.opacity);
            }
        }
        PaintKind::Polygon {
            points,
            fill,
            stroke,
        } => {
            let points = points
                .iter()
                .copied()
                .map(|point| cpu_snapshot_transform_point(point, item.transform))
                .collect::<Vec<_>>();
            cpu_snapshot_fill_polygon(image, &points, clip, *fill, item.opacity);
            if let Some(stroke) = stroke {
                for segment in points.windows(2) {
                    cpu_snapshot_draw_line(
                        image,
                        segment[0],
                        segment[1],
                        clip,
                        *stroke,
                        item.opacity,
                    );
                }
                if points.len() > 2 {
                    cpu_snapshot_draw_line(
                        image,
                        *points.last().unwrap(),
                        points[0],
                        clip,
                        *stroke,
                        item.opacity,
                    );
                }
            }
        }
        PaintKind::Image { key, tint } => {
            cpu_snapshot_draw_image_placeholder(
                image,
                cpu_snapshot_transform_rect(item.rect, item.transform),
                clip,
                key,
                *tint,
            );
        }
        PaintKind::Path(path) => {
            let points = path
                .verbs
                .iter()
                .filter_map(|verb| match *verb {
                    PathVerb::MoveTo(point) | PathVerb::LineTo(point) => {
                        Some(cpu_snapshot_transform_point(point, item.transform))
                    }
                    PathVerb::QuadraticTo { to, .. } | PathVerb::CubicTo { to, .. } => {
                        Some(cpu_snapshot_transform_point(to, item.transform))
                    }
                    PathVerb::Close => None,
                })
                .collect::<Vec<_>>();
            if let Some(fill) = &path.fill {
                cpu_snapshot_fill_polygon(
                    image,
                    &points,
                    clip,
                    fill.fallback_color(),
                    item.opacity,
                );
            }
            if let Some(stroke) = path.stroke {
                for segment in points.windows(2) {
                    cpu_snapshot_draw_line(
                        image,
                        segment[0],
                        segment[1],
                        clip,
                        stroke.style,
                        item.opacity,
                    );
                }
            }
        }
        PaintKind::ImagePlacement(image_placement) => {
            cpu_snapshot_draw_image_placeholder(
                image,
                cpu_snapshot_transform_rect(image_placement.rect, item.transform),
                clip,
                &image_placement.key,
                image_placement.tint,
            );
        }
    }
}

fn cpu_snapshot_draw_text(image: &mut CpuSnapshotImage, item: &PaintItem, text: &TextContent) {
    let rect = cpu_snapshot_transform_rect(item.rect, item.transform);
    let color = text.style.color;
    let glyph_width = (text.style.font_size * item.transform.scale * 0.52).max(4.0);
    let glyph_height = (text.style.line_height * item.transform.scale * 0.70).max(5.0);
    let baseline_y = rect.y + (text.style.line_height * item.transform.scale * 0.18).max(1.0);
    let mut x = rect.x;
    let mut y = baseline_y;
    for ch in text.text.chars() {
        if ch == '\n' {
            x = rect.x;
            y += text.style.line_height * item.transform.scale;
            continue;
        }
        if !ch.is_whitespace() {
            let hash = cpu_snapshot_hash_str(&ch.to_string());
            let inset = (hash % 3) as f32;
            cpu_snapshot_fill_rect(
                image,
                UiRect::new(
                    x + inset,
                    y + inset,
                    (glyph_width - inset).max(1.0),
                    (glyph_height - inset * 2.0).max(1.0),
                ),
                item.clip_rect,
                color,
                item.opacity,
            );
        }
        x += glyph_width;
        if x > rect.right() {
            break;
        }
    }
}

fn cpu_snapshot_draw_canvas(
    image: &mut CpuSnapshotImage,
    item: &PaintItem,
    canvas: &CanvasContent,
) {
    let rect = cpu_snapshot_transform_rect(item.rect, item.transform);
    let base = cpu_snapshot_color_from_key(&canvas.key, 210);
    cpu_snapshot_fill_rect(image, rect, item.clip_rect, base, item.opacity);
    let accent = ColorRgba::new(
        base.r.saturating_add(34),
        base.g.saturating_add(24),
        base.b.saturating_add(18),
        255,
    );
    let step = 12.0;
    let mut x = rect.x;
    while x < rect.right() {
        cpu_snapshot_draw_line(
            image,
            UiPoint::new(x, rect.y),
            UiPoint::new(x + rect.height, rect.bottom()),
            item.clip_rect,
            StrokeStyle::new(accent, 1.0),
            item.opacity,
        );
        x += step;
    }
}

fn cpu_snapshot_draw_image_placeholder(
    image: &mut CpuSnapshotImage,
    rect: UiRect,
    clip: UiRect,
    key: &str,
    tint: Option<ColorRgba>,
) {
    let base = tint.unwrap_or_else(|| cpu_snapshot_color_from_key(key, 235));
    cpu_snapshot_fill_rect(image, rect, clip, base, 1.0);
    let hash = cpu_snapshot_hash_str(key);
    let stripe = ColorRgba::new(
        base.r.saturating_sub(((hash >> 8) & 31) as u8),
        base.g.saturating_sub(((hash >> 16) & 31) as u8),
        base.b.saturating_sub(((hash >> 24) & 31) as u8),
        base.a,
    );
    let mut x = rect.x;
    while x < rect.right() {
        cpu_snapshot_fill_rect(
            image,
            UiRect::new(x, rect.y, 2.0, rect.height),
            clip,
            stripe,
            0.8,
        );
        x += 6.0;
    }
}

fn cpu_snapshot_fill_rect(
    image: &mut CpuSnapshotImage,
    rect: UiRect,
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
) {
    if color.a == 0 || opacity <= 0.0 {
        return;
    }
    let Some(rect) = rect.intersection(clip) else {
        return;
    };
    let left = rect.x.floor().max(0.0) as usize;
    let top = rect.y.floor().max(0.0) as usize;
    let right = rect.right().ceil().min(image.width() as f32) as usize;
    let bottom = rect.bottom().ceil().min(image.height() as f32) as usize;
    for y in top..bottom {
        for x in left..right {
            cpu_snapshot_blend_pixel(image, x, y, color, opacity);
        }
    }
}

fn cpu_snapshot_stroke_rect(
    image: &mut CpuSnapshotImage,
    rect: UiRect,
    clip: UiRect,
    stroke: StrokeStyle,
    opacity: f32,
) {
    let width = stroke.width.max(1.0);
    cpu_snapshot_fill_rect(
        image,
        UiRect::new(rect.x, rect.y, rect.width, width),
        clip,
        stroke.color,
        opacity,
    );
    cpu_snapshot_fill_rect(
        image,
        UiRect::new(rect.x, rect.bottom() - width, rect.width, width),
        clip,
        stroke.color,
        opacity,
    );
    cpu_snapshot_fill_rect(
        image,
        UiRect::new(rect.x, rect.y, width, rect.height),
        clip,
        stroke.color,
        opacity,
    );
    cpu_snapshot_fill_rect(
        image,
        UiRect::new(rect.right() - width, rect.y, width, rect.height),
        clip,
        stroke.color,
        opacity,
    );
}

fn cpu_snapshot_draw_line(
    image: &mut CpuSnapshotImage,
    from: UiPoint,
    to: UiPoint,
    clip: UiRect,
    stroke: StrokeStyle,
    opacity: f32,
) {
    let min_x = from.x.min(to.x).floor().max(0.0) as usize;
    let min_y = from.y.min(to.y).floor().max(0.0) as usize;
    let max_x = from.x.max(to.x).ceil().min(image.width() as f32 - 1.0) as usize;
    let max_y = from.y.max(to.y).ceil().min(image.height() as f32 - 1.0) as usize;
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f32::EPSILON {
        cpu_snapshot_fill_rect(
            image,
            UiRect::new(from.x, from.y, stroke.width.max(1.0), stroke.width.max(1.0)),
            clip,
            stroke.color,
            opacity,
        );
        return;
    }
    let radius = stroke.width.max(1.0) * 0.5 + 0.75;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = UiPoint::new(x as f32 + 0.5, y as f32 + 0.5);
            if !clip.contains_point(point) {
                continue;
            }
            let t = (((point.x - from.x) * dx + (point.y - from.y) * dy) / length_squared)
                .clamp(0.0, 1.0);
            let closest = UiPoint::new(from.x + dx * t, from.y + dy * t);
            let distance = ((point.x - closest.x).powi(2) + (point.y - closest.y).powi(2)).sqrt();
            if distance <= radius {
                cpu_snapshot_blend_pixel(image, x, y, stroke.color, opacity);
            }
        }
    }
}

fn cpu_snapshot_fill_circle(
    image: &mut CpuSnapshotImage,
    center: UiPoint,
    radius: f32,
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
) {
    let bounds = UiRect::new(
        center.x - radius,
        center.y - radius,
        radius * 2.0,
        radius * 2.0,
    );
    let Some(bounds) = bounds.intersection(clip) else {
        return;
    };
    let left = bounds.x.floor().max(0.0) as usize;
    let top = bounds.y.floor().max(0.0) as usize;
    let right = bounds.right().ceil().min(image.width() as f32) as usize;
    let bottom = bounds.bottom().ceil().min(image.height() as f32) as usize;
    let radius_squared = radius * radius;
    for y in top..bottom {
        for x in left..right {
            let dx = x as f32 + 0.5 - center.x;
            let dy = y as f32 + 0.5 - center.y;
            if dx * dx + dy * dy <= radius_squared {
                cpu_snapshot_blend_pixel(image, x, y, color, opacity);
            }
        }
    }
}

fn cpu_snapshot_stroke_circle(
    image: &mut CpuSnapshotImage,
    center: UiPoint,
    radius: f32,
    clip: UiRect,
    stroke: StrokeStyle,
    opacity: f32,
) {
    let bounds = UiRect::new(
        center.x - radius,
        center.y - radius,
        radius * 2.0,
        radius * 2.0,
    );
    let Some(bounds) = bounds.intersection(clip) else {
        return;
    };
    let left = bounds.x.floor().max(0.0) as usize;
    let top = bounds.y.floor().max(0.0) as usize;
    let right = bounds.right().ceil().min(image.width() as f32) as usize;
    let bottom = bounds.bottom().ceil().min(image.height() as f32) as usize;
    let half = stroke.width.max(1.0) * 0.5;
    for y in top..bottom {
        for x in left..right {
            let dx = x as f32 + 0.5 - center.x;
            let dy = y as f32 + 0.5 - center.y;
            let distance = (dx * dx + dy * dy).sqrt();
            if (radius - half..=radius + half).contains(&distance) {
                cpu_snapshot_blend_pixel(image, x, y, stroke.color, opacity);
            }
        }
    }
}

fn cpu_snapshot_fill_polygon(
    image: &mut CpuSnapshotImage,
    points: &[UiPoint],
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
) {
    if points.len() < 3 {
        return;
    }
    let mut left = points[0].x;
    let mut top = points[0].y;
    let mut right = points[0].x;
    let mut bottom = points[0].y;
    for point in points {
        left = left.min(point.x);
        top = top.min(point.y);
        right = right.max(point.x);
        bottom = bottom.max(point.y);
    }
    let Some(bounds) = UiRect::new(left, top, right - left, bottom - top).intersection(clip) else {
        return;
    };
    let left = bounds.x.floor().max(0.0) as usize;
    let top = bounds.y.floor().max(0.0) as usize;
    let right = bounds.right().ceil().min(image.width() as f32) as usize;
    let bottom = bounds.bottom().ceil().min(image.height() as f32) as usize;
    for y in top..bottom {
        for x in left..right {
            if cpu_snapshot_point_in_polygon(UiPoint::new(x as f32 + 0.5, y as f32 + 0.5), points) {
                cpu_snapshot_blend_pixel(image, x, y, color, opacity);
            }
        }
    }
}

fn cpu_snapshot_point_in_polygon(point: UiPoint, points: &[UiPoint]) -> bool {
    let mut inside = false;
    let mut previous = points.len() - 1;
    for current in 0..points.len() {
        let pi = points[current];
        let pj = points[previous];
        if ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn cpu_snapshot_blend_pixel(
    image: &mut CpuSnapshotImage,
    x: usize,
    y: usize,
    color: ColorRgba,
    opacity: f32,
) {
    if x >= image.width() || y >= image.height() {
        return;
    }
    let index = (y * image.width() + x) * 4;
    let alpha = (f32::from(color.a) / 255.0 * opacity.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let inv = 1.0 - alpha;
    image.pixels[index] = (f32::from(image.pixels[index]) * inv + f32::from(color.r) * alpha)
        .round()
        .clamp(0.0, 255.0) as u8;
    image.pixels[index + 1] = (f32::from(image.pixels[index + 1]) * inv
        + f32::from(color.g) * alpha)
        .round()
        .clamp(0.0, 255.0) as u8;
    image.pixels[index + 2] = (f32::from(image.pixels[index + 2]) * inv
        + f32::from(color.b) * alpha)
        .round()
        .clamp(0.0, 255.0) as u8;
    image.pixels[index + 3] = 255;
}

fn cpu_snapshot_transform_point(point: UiPoint, transform: PaintTransform) -> UiPoint {
    transform.transform_point(point)
}

fn cpu_snapshot_transform_rect(rect: UiRect, transform: PaintTransform) -> UiRect {
    transform.transform_rect(rect)
}

fn cpu_snapshot_color_from_key(key: &str, alpha: u8) -> ColorRgba {
    let hash = cpu_snapshot_hash_str(key);
    ColorRgba::new(
        48 + (hash & 127) as u8,
        58 + ((hash >> 8) & 127) as u8,
        68 + ((hash >> 16) & 127) as u8,
        alpha,
    )
}

fn cpu_snapshot_hash_str(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
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
            Err(TestFailure::new(format!(
                "frame timing total {total:?} exceeded budget {budget:?}"
            )))
        }
    }

    pub fn require_section_within(&self, name: &str, budget: Duration) -> TestResult<Duration> {
        let duration = self.require_section(name)?;
        if duration <= budget {
            Ok(duration)
        } else {
            Err(TestFailure::new(format!(
                "frame timing section `{name}` duration {duration:?} exceeded budget {budget:?}"
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{
        Command, CommandId, CommandMeta, CommandRegistry, CommandScope, Shortcut,
    };
    use crate::platform::{
        ClipboardRequest, ClipboardResponse, CursorRequest, LogicalRect, PixelSize,
        PlatformErrorCode, PlatformRequest, PlatformRequestId, PlatformRequestIdAllocator,
        PlatformResponse, PlatformServiceError, PlatformServiceKind, RepaintRequest,
        RepaintResponse,
    };
    use crate::{
        length, process_document_frame, root_style, AccessibilityAction, AccessibilityLiveRegion,
        AccessibilityMeta, AccessibilityRole, AccessibilitySummary, AccessibilityValueRange,
        ApproxTextMeasurer, CanvasContent, CanvasInteractionPolicy, CanvasRenderContext,
        CanvasRenderOutput, CanvasRenderRegistry, ClipBehavior, ColorRgba, DirtyRegionSet,
        HostDocumentFrameRequest, HostFrameOutput, HostInteractionState, ImageContent,
        ImageRenderContext, ImageRenderOutput, ImageRenderRegistry, InputBehavior, PaintBatch,
        PaintBatchKey, RawKeyboardEvent, RawWheelEvent, RenderFrameOutput, RenderFrameRequest,
        RenderTarget, RenderTargetKind, RenderedImage, ResourceFormat, ScrollAxes, ShaderEffect,
        StrokeStyle, TextStyle, UiContent, UiDocument, UiNode, UiNodeStyle, UiPoint, UiVisual,
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
                    layout: Style {
                        size: TaffySize {
                            width: length(120.0),
                            height: length(48.0),
                        },
                        ..Default::default()
                    },
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

        let report = EventReplay::new()
            .wheel(
                "scroll.down",
                UiPoint::new(24.0, 24.0),
                UiPoint::new(0.0, 32.0),
            )
            .pointer_drag(
                "empty.drag",
                UiPoint::new(140.0, 80.0),
                UiPoint::new(150.0, 88.0),
                [UiPoint::new(145.0, 84.0)],
            )
            .run(&mut document);

        report.require_scrolled(scroll_area).expect("scrolled area");
        report.require_no_clicks().expect("drag outside misses");
        assert!(report.require_clicked(scroll_area).is_err());
        assert!(report.step("empty.drag.move.0").is_ok());
        assert!(report.step("missing").is_err());
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
            ))
            .with_shader(ShaderEffect::new("panel.surface")),
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
    fn audit_assertions_report_accessibility_gaps_by_stable_name() {
        let mut document = UiDocument::new(root_style(260.0, 120.0));
        let root = document.root;
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
                fixed_style(80.0, 20.0).layout,
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
            .require_no_accessibility_state_gap("complete_toggle")
            .expect("complete toggle state");
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
    fn canvas_hit_assertions_check_targets_topmost_and_accessibility() {
        let canvas = CanvasContent::new("editor.viewport").domain_hit_testing(true);
        let mut paint = PaintList::default();
        paint.items.push(PaintItem {
            node: UiNodeId(3),
            rect: UiRect::new(8.0, 10.0, 120.0, 64.0),
            clip_rect: UiRect::new(0.0, 0.0, 160.0, 120.0),
            z_index: 0,
            layer_order: crate::platform::LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
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
                        .z_index(1),
                    CanvasHitTarget::new("disabled.overlay", UiRect::new(10.0, 12.0, 60.0, 24.0))
                        .label("Disabled overlay")
                        .disabled(true)
                        .z_index(10),
                    CanvasHitTarget::new("item.resize", UiRect::new(14.0, 12.0, 12.0, 24.0))
                        .label("Resize handle")
                        .value("start edge")
                        .z_index(4),
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
                kind: crate::PaintBatchKind::Rect,
                z_index: 0,
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
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            )
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label("Choices")),
        );
        let hint = document.add_child(
            root,
            UiNode::text(
                "choices.hint",
                "Pick one option",
                TextStyle::default(),
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
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
    fn cpu_snapshot_renderer_renders_documents_and_adapter_snapshots() {
        struct EmptyResolver;

        impl ResourceResolver for EmptyResolver {
            fn resolve_resource(
                &self,
                _id: &crate::platform::ResourceId,
            ) -> Option<crate::renderer::ResourceDescriptor> {
                None
            }
        }

        let viewport = UiSize::new(96.0, 64.0);
        let mut document = UiDocument::new(root_style(viewport.width, viewport.height));
        let root = document.root;
        let panel = document.add_child(
            root,
            UiNode::container("panel", fixed_style(72.0, 44.0)).with_visual(UiVisual::panel(
                ColorRgba::new(30, 42, 58, 255),
                Some(StrokeStyle::new(ColorRgba::new(120, 150, 180, 255), 1.0)),
                3.0,
            )),
        );
        document.add_child(
            panel,
            UiNode::text(
                "panel.label",
                "CPU",
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
        document.add_child(
            panel,
            UiNode::canvas(
                "panel.canvas",
                "snapshot.scope",
                fixed_style(24.0, 18.0).layout,
            ),
        );

        let renderer = CpuSnapshotRenderer::default();
        let image = renderer
            .render_document(&mut document, viewport)
            .expect("document snapshot");
        assert_eq!(image.size, PixelSize::new(96, 64));
        let assertions = renderer
            .snapshot_assertions("cpu-render", &image)
            .expect("snapshot assertions");
        assertions
            .require_min_changed_pixels_from(DEFAULT_CPU_SNAPSHOT_BACKGROUND, 100)
            .expect("rendered content");
        let first_hash = assertions.hash();

        let repeated = renderer
            .render_paint_list(&document.paint_list(), image.size)
            .expect("repeat snapshot");
        assert_eq!(repeated.hash(), first_hash);

        let request = RenderFrameRequest::new(
            RenderTarget::snapshot(image.size),
            viewport,
            document.paint_list(),
        );
        let mut adapter = CpuSnapshotRenderer::default();
        let output = adapter
            .render_frame(request, &EmptyResolver)
            .expect("adapter render");
        let snapshot = RenderOutputAssertions::new(&output)
            .require_snapshot_rgba8("adapter")
            .expect("adapter snapshot");
        assert_eq!(snapshot.hash(), first_hash);
        assert_eq!(
            adapter.capabilities().adapter,
            crate::platform::BackendAdapterKind::CpuSnapshot
        );
        assert!(adapter.capabilities().rendering.deterministic_snapshots);
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
        assert!(assertions
            .require_total_within(Duration::from_millis(10))
            .is_err());
        assert!(assertions
            .require_section_within("paint", Duration::from_millis(4))
            .is_ok());
        assert!(assertions
            .require_section_within("paint", Duration::from_millis(3))
            .is_err());
        assert!(assertions.require_section("input").is_err());

        let mut samples = PerformanceSamples::new("render smoke");
        samples.push(Duration::from_millis(4));
        samples.push(Duration::from_millis(6));
        samples.push(Duration::from_millis(5));
        assert_eq!(samples.len(), 3);
        assert_eq!(samples.total(), Duration::from_millis(15));
        assert_eq!(samples.max_sample(), Some(Duration::from_millis(6)));
        assert_eq!(samples.average(), Some(Duration::from_millis(5)));

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
        assert!(
            PerformanceAssertions::new(&PerformanceSamples::new("empty"))
                .require_average_within(Duration::from_millis(1))
                .is_err()
        );
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
                    .unit(crate::WheelDeltaUnit::Line)
            ))
        );
        assert!(report.steps[1].converted.is_none());
        assert!(report.require_all_converted().is_err());
    }
}
