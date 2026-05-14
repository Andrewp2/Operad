//! Backend-neutral unified diagnostics.
//!
//! The report types in this module are intended to collect the public
//! renderer-neutral contracts that already exist in Operad into one bounded
//! diagnostic surface. They preserve typed records for tests/tools and also
//! emit stable human-readable summaries for logs and snapshots.

use std::time::Duration;

use crate::{
    AccessibilityAdapterApplyReport, AccessibilityAdapterRequest, AccessibilityAdapterResponse,
    AccessibilityAdapterState, AccessibilityAnnouncement, AccessibilityPreferences,
    AccessibilityRequestKind, DirtyFlags, EffectiveGeometryRecord, EffectiveHitRejection,
    FocusRestoreTarget, FocusTrap, FrameTiming, OverlayEntry, OverlayHitTestDecision, OverlayId,
    OverlayStack, PerformanceSnapshot, UiInputResult, UiNodeId, WidgetAction, WidgetActionBinding,
    WidgetActionKind, WidgetActionTrigger, WidgetDragPhase, WidgetValueEditPhase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Trace,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCategory {
    InputRouting,
    WidgetAction,
    OverlayStack,
    Accessibility,
    GeometryHit,
    RenderTiming,
    Performance,
    DirtyState,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSummaryRecord {
    pub severity: DiagnosticSeverity,
    pub category: DiagnosticCategory,
    pub label: String,
    pub summary: String,
}

impl DiagnosticSummaryRecord {
    pub fn new(
        severity: DiagnosticSeverity,
        category: DiagnosticCategory,
        label: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            category,
            label: label.into(),
            summary: summary.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticMessage {
    pub severity: DiagnosticSeverity,
    pub category: DiagnosticCategory,
    pub label: String,
    pub message: String,
}

impl DiagnosticMessage {
    pub fn warning(label: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            category: DiagnosticCategory::Warning,
            label: label.into(),
            message: message.into(),
        }
    }

    pub fn error(label: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::Error,
            label: label.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputRoutingDiagnostic {
    pub hovered: Option<UiNodeId>,
    pub focused: Option<UiNodeId>,
    pub pressed: Option<UiNodeId>,
    pub clicked: Option<UiNodeId>,
    pub scrolled: Option<UiNodeId>,
}

impl From<&UiInputResult> for InputRoutingDiagnostic {
    fn from(result: &UiInputResult) -> Self {
        Self {
            hovered: result.hovered,
            focused: result.focused,
            pressed: result.pressed,
            clicked: result.clicked,
            scrolled: result.scrolled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetActionDiagnostic {
    pub target: UiNodeId,
    pub node_label: String,
    pub action_label: String,
    pub binding_label: String,
    pub kind_label: String,
}

impl From<&WidgetAction> for WidgetActionDiagnostic {
    fn from(action: &WidgetAction) -> Self {
        Self {
            target: action.target,
            node_label: node_label(action.target),
            action_label: widget_action_label(action),
            binding_label: widget_binding_label(&action.binding),
            kind_label: widget_action_kind_label(&action.kind).to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayEntryDiagnostic {
    pub id: OverlayId,
    pub label: String,
    pub kind_label: String,
    pub parent: Option<OverlayId>,
    pub modal: bool,
    pub layer: i32,
}

impl From<&OverlayEntry> for OverlayEntryDiagnostic {
    fn from(entry: &OverlayEntry) -> Self {
        Self {
            id: entry.id,
            label: overlay_label(entry.id),
            kind_label: format!("{:?}", entry.kind),
            parent: entry.parent,
            modal: entry.modal,
            layer: entry.layer,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayStackDiagnostic {
    pub entries: Vec<OverlayEntryDiagnostic>,
    pub topmost: Option<OverlayId>,
}

impl From<&OverlayStack> for OverlayStackDiagnostic {
    fn from(stack: &OverlayStack) -> Self {
        Self {
            entries: stack
                .entries()
                .iter()
                .map(OverlayEntryDiagnostic::from)
                .collect(),
            topmost: stack.topmost(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayRoutingDiagnostic {
    pub hit: Option<OverlayId>,
    pub blocked_by_modal: Option<OverlayId>,
    pub dismiss: Vec<OverlayId>,
}

impl From<&OverlayHitTestDecision> for OverlayRoutingDiagnostic {
    fn from(decision: &OverlayHitTestDecision) -> Self {
        Self {
            hit: decision.hit,
            blocked_by_modal: decision.blocked_by_modal,
            dismiss: decision.dismiss.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityRequestDiagnostic {
    pub kind: AccessibilityRequestKind,
    pub label: String,
    pub summary: String,
}

impl From<&AccessibilityAdapterRequest> for AccessibilityRequestDiagnostic {
    fn from(request: &AccessibilityAdapterRequest) -> Self {
        Self {
            kind: request.kind(),
            label: accessibility_request_label(request).to_string(),
            summary: accessibility_request_summary(request),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityResponseDiagnostic {
    pub severity: DiagnosticSeverity,
    pub label: String,
    pub summary: String,
}

impl From<&AccessibilityAdapterResponse> for AccessibilityResponseDiagnostic {
    fn from(response: &AccessibilityAdapterResponse) -> Self {
        Self {
            severity: accessibility_response_severity(response),
            label: accessibility_response_label(response).to_string(),
            summary: accessibility_response_summary(response),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityOutputDiagnostic {
    pub focused: Option<UiNodeId>,
    pub published_nodes: usize,
    pub announcements: usize,
    pub target_summaries: usize,
    pub focus_trap_root: Option<UiNodeId>,
    pub preferences: AccessibilityPreferences,
}

impl From<&AccessibilityAdapterState> for AccessibilityOutputDiagnostic {
    fn from(state: &AccessibilityAdapterState) -> Self {
        Self {
            focused: state.focused,
            published_nodes: state
                .published_tree
                .as_ref()
                .map_or(0, |tree| tree.nodes.len()),
            announcements: state.announcements.len(),
            target_summaries: state.target_summaries.len(),
            focus_trap_root: state.focus_trap.map(|trap| trap.trap.root),
            preferences: state.preferences,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeometryHitDiagnostic {
    pub node: UiNodeId,
    pub node_label: String,
    pub hit_testable: bool,
    pub visible: bool,
    pub point_hit: Option<bool>,
    pub rejected_by: Vec<EffectiveHitRejection>,
    pub resolved_z: i32,
    pub order: usize,
}

impl From<&EffectiveGeometryRecord> for GeometryHitDiagnostic {
    fn from(record: &EffectiveGeometryRecord) -> Self {
        Self {
            node: record.node,
            node_label: node_label(record.node),
            hit_testable: record.hit_testable,
            visible: record.visible,
            point_hit: record.point_hit,
            rejected_by: record.point_rejections.clone(),
            resolved_z: record.resolved_z,
            order: record.order,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTimingDiagnostic {
    pub section_count: usize,
    pub total: Duration,
    pub sections: Vec<RenderTimingSectionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTimingSectionDiagnostic {
    pub name: String,
    pub duration: Duration,
}

impl From<&FrameTiming> for RenderTimingDiagnostic {
    fn from(timing: &FrameTiming) -> Self {
        Self {
            section_count: timing.sections.len(),
            total: timing.total(),
            sections: timing
                .sections
                .iter()
                .map(|section| RenderTimingSectionDiagnostic {
                    name: section.name.clone(),
                    duration: section.duration,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceSnapshotDiagnostic {
    pub frame: u64,
    pub total: Duration,
    pub slowest_stage: Option<String>,
    pub missing_stage_labels: Vec<String>,
    pub missing_cache_labels: Vec<String>,
    pub caches: Vec<PerformanceCacheDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceCacheDiagnostic {
    pub kind_label: String,
    pub name: String,
    pub lookups: usize,
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub retained_bytes: Option<usize>,
    pub hit_rate: Option<f32>,
}

impl From<&PerformanceSnapshot> for PerformanceSnapshotDiagnostic {
    fn from(snapshot: &PerformanceSnapshot) -> Self {
        Self {
            frame: snapshot.frame,
            total: snapshot.pipeline.total(),
            slowest_stage: snapshot
                .pipeline
                .slowest_stage()
                .map(|section| section.stage.label().to_string()),
            missing_stage_labels: snapshot
                .missing_required_stages()
                .into_iter()
                .map(|stage| stage.label().to_string())
                .collect(),
            missing_cache_labels: snapshot
                .missing_required_cache_kinds()
                .into_iter()
                .map(|kind| kind.label().to_string())
                .collect(),
            caches: snapshot
                .caches
                .iter()
                .map(|cache| PerformanceCacheDiagnostic {
                    kind_label: cache.kind.label().to_string(),
                    name: cache.name.clone(),
                    lookups: cache.lookups,
                    hits: cache.hits,
                    misses: cache.misses,
                    evictions: cache.evictions,
                    retained_bytes: cache.retained_bytes,
                    hit_rate: cache.hit_rate(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyFlagsDiagnostic {
    pub flags: DirtyFlags,
    pub active: Vec<String>,
}

impl From<DirtyFlags> for DirtyFlagsDiagnostic {
    fn from(flags: DirtyFlags) -> Self {
        Self {
            flags,
            active: dirty_flag_labels(flags)
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticRecord {
    InputRouting(InputRoutingDiagnostic),
    WidgetAction(WidgetActionDiagnostic),
    OverlayStack(OverlayStackDiagnostic),
    OverlayRouting(OverlayRoutingDiagnostic),
    AccessibilityRequest(AccessibilityRequestDiagnostic),
    AccessibilityResponse(AccessibilityResponseDiagnostic),
    AccessibilityOutput(AccessibilityOutputDiagnostic),
    GeometryHit(GeometryHitDiagnostic),
    RenderTiming(RenderTimingDiagnostic),
    PerformanceSnapshot(PerformanceSnapshotDiagnostic),
    DirtyFlags(DirtyFlagsDiagnostic),
    Message(DiagnosticMessage),
}

impl DiagnosticRecord {
    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            Self::AccessibilityResponse(response) => response.severity,
            Self::GeometryHit(hit) if !hit.rejected_by.is_empty() => DiagnosticSeverity::Trace,
            Self::PerformanceSnapshot(performance)
                if !performance.missing_stage_labels.is_empty()
                    || !performance.missing_cache_labels.is_empty() =>
            {
                DiagnosticSeverity::Warning
            }
            Self::DirtyFlags(dirty) if dirty.flags.any() => DiagnosticSeverity::Info,
            Self::Message(message) => message.severity,
            _ => DiagnosticSeverity::Info,
        }
    }

    pub fn category(&self) -> DiagnosticCategory {
        match self {
            Self::InputRouting(_) => DiagnosticCategory::InputRouting,
            Self::WidgetAction(_) => DiagnosticCategory::WidgetAction,
            Self::OverlayStack(_) | Self::OverlayRouting(_) => DiagnosticCategory::OverlayStack,
            Self::AccessibilityRequest(_)
            | Self::AccessibilityResponse(_)
            | Self::AccessibilityOutput(_) => DiagnosticCategory::Accessibility,
            Self::GeometryHit(_) => DiagnosticCategory::GeometryHit,
            Self::RenderTiming(_) => DiagnosticCategory::RenderTiming,
            Self::PerformanceSnapshot(_) => DiagnosticCategory::Performance,
            Self::DirtyFlags(_) => DiagnosticCategory::DirtyState,
            Self::Message(message) => message.category,
        }
    }

    pub fn summary(&self) -> DiagnosticSummaryRecord {
        match self {
            Self::InputRouting(input) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                "input-routing",
                input_routing_summary(input),
            ),
            Self::WidgetAction(action) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                action.action_label.clone(),
                format!("{} on {}", action.kind_label, action.node_label),
            ),
            Self::OverlayStack(stack) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                "overlay-stack",
                format!(
                    "{} overlays, topmost {}",
                    stack.entries.len(),
                    optional_overlay_label(stack.topmost)
                ),
            ),
            Self::OverlayRouting(routing) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                "overlay-routing",
                format!(
                    "hit {}, blocked {}, dismiss {}",
                    optional_overlay_label(routing.hit),
                    optional_overlay_label(routing.blocked_by_modal),
                    routing.dismiss.len()
                ),
            ),
            Self::AccessibilityRequest(request) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                request.label.clone(),
                request.summary.clone(),
            ),
            Self::AccessibilityResponse(response) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                response.label.clone(),
                response.summary.clone(),
            ),
            Self::AccessibilityOutput(output) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                "accessibility-output",
                format!(
                    "{} nodes, focused {}, {} announcements, {} targets",
                    output.published_nodes,
                    optional_node_label(output.focused),
                    output.announcements,
                    output.target_summaries
                ),
            ),
            Self::GeometryHit(hit) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                hit.node_label.clone(),
                geometry_hit_summary(hit),
            ),
            Self::RenderTiming(timing) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                "render-timing",
                format!(
                    "{} sections, total {}",
                    timing.section_count,
                    duration_label(timing.total)
                ),
            ),
            Self::PerformanceSnapshot(performance) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                "performance",
                performance_snapshot_summary(performance),
            ),
            Self::DirtyFlags(dirty) => DiagnosticSummaryRecord::new(
                self.severity(),
                self.category(),
                "dirty-flags",
                dirty_flags_summary(dirty.flags),
            ),
            Self::Message(message) => DiagnosticSummaryRecord::new(
                message.severity,
                message.category,
                message.label.clone(),
                message.message.clone(),
            ),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiagnosticReport {
    pub records: Vec<DiagnosticRecord>,
    pub summaries: Vec<DiagnosticSummaryRecord>,
}

impl DiagnosticReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, record: DiagnosticRecord) -> &mut Self {
        self.summaries.push(record.summary());
        self.records.push(record);
        self
    }

    pub fn input_routing(&mut self, result: &UiInputResult) -> &mut Self {
        self.push(DiagnosticRecord::InputRouting(result.into()))
    }

    pub fn widget_action(&mut self, action: &WidgetAction) -> &mut Self {
        self.push(DiagnosticRecord::WidgetAction(action.into()))
    }

    pub fn widget_actions<'a>(
        &mut self,
        actions: impl IntoIterator<Item = &'a WidgetAction>,
    ) -> &mut Self {
        for action in actions {
            self.widget_action(action);
        }
        self
    }

    pub fn overlay_stack(&mut self, stack: &OverlayStack) -> &mut Self {
        self.push(DiagnosticRecord::OverlayStack(stack.into()))
    }

    pub fn overlay_routing(&mut self, decision: &OverlayHitTestDecision) -> &mut Self {
        self.push(DiagnosticRecord::OverlayRouting(decision.into()))
    }

    pub fn accessibility_request(&mut self, request: &AccessibilityAdapterRequest) -> &mut Self {
        self.push(DiagnosticRecord::AccessibilityRequest(request.into()))
    }

    pub fn accessibility_requests<'a>(
        &mut self,
        requests: impl IntoIterator<Item = &'a AccessibilityAdapterRequest>,
    ) -> &mut Self {
        for request in requests {
            self.accessibility_request(request);
        }
        self
    }

    pub fn accessibility_response(&mut self, response: &AccessibilityAdapterResponse) -> &mut Self {
        self.push(DiagnosticRecord::AccessibilityResponse(response.into()))
    }

    pub fn accessibility_apply_report(
        &mut self,
        report: &AccessibilityAdapterApplyReport,
    ) -> &mut Self {
        for response in &report.responses {
            self.accessibility_response(response);
        }
        self
    }

    pub fn accessibility_output(&mut self, state: &AccessibilityAdapterState) -> &mut Self {
        self.push(DiagnosticRecord::AccessibilityOutput(state.into()))
    }

    pub fn geometry_hit(&mut self, record: &EffectiveGeometryRecord) -> &mut Self {
        self.push(DiagnosticRecord::GeometryHit(record.into()))
    }

    pub fn geometry_hits<'a>(
        &mut self,
        records: impl IntoIterator<Item = &'a EffectiveGeometryRecord>,
    ) -> &mut Self {
        for record in records {
            self.geometry_hit(record);
        }
        self
    }

    pub fn render_timing(&mut self, timing: &FrameTiming) -> &mut Self {
        self.push(DiagnosticRecord::RenderTiming(timing.into()))
    }

    pub fn performance_snapshot(&mut self, snapshot: &PerformanceSnapshot) -> &mut Self {
        self.push(DiagnosticRecord::PerformanceSnapshot(snapshot.into()))
    }

    pub fn dirty_flags(&mut self, flags: DirtyFlags) -> &mut Self {
        self.push(DiagnosticRecord::DirtyFlags(flags.into()))
    }

    pub fn warning(&mut self, label: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.push(DiagnosticRecord::Message(DiagnosticMessage::warning(
            label, message,
        )))
    }

    pub fn error(&mut self, label: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.push(DiagnosticRecord::Message(DiagnosticMessage::error(
            label, message,
        )))
    }

    pub fn highest_severity(&self) -> Option<DiagnosticSeverity> {
        self.records.iter().map(DiagnosticRecord::severity).max()
    }

    pub fn summaries_by_category(
        &self,
        category: DiagnosticCategory,
    ) -> impl Iterator<Item = &DiagnosticSummaryRecord> {
        self.summaries
            .iter()
            .filter(move |summary| summary.category == category)
    }
}

pub fn node_label(node: UiNodeId) -> String {
    format!("node:{}", node.0)
}

pub fn overlay_label(overlay: OverlayId) -> String {
    format!("overlay:{}", overlay.0)
}

pub fn widget_action_label(action: &WidgetAction) -> String {
    format!(
        "{}:{}",
        widget_binding_label(&action.binding),
        widget_action_kind_label(&action.kind)
    )
}

fn optional_node_label(node: Option<UiNodeId>) -> String {
    node.map(node_label).unwrap_or_else(|| "none".to_string())
}

fn optional_overlay_label(overlay: Option<OverlayId>) -> String {
    overlay
        .map(overlay_label)
        .unwrap_or_else(|| "none".to_string())
}

fn widget_binding_label(binding: &WidgetActionBinding) -> String {
    match binding {
        WidgetActionBinding::Action(id) => format!("action:{}", id.as_str()),
        WidgetActionBinding::Command(id) => format!("command:{}", id.as_str()),
    }
}

fn widget_action_kind_label(kind: &WidgetActionKind) -> &'static str {
    match kind {
        WidgetActionKind::Activate(activation) => match activation.trigger {
            WidgetActionTrigger::Pointer => "activate.pointer",
            WidgetActionTrigger::Keyboard => "activate.keyboard",
            WidgetActionTrigger::Accessibility => "activate.accessibility",
            WidgetActionTrigger::Programmatic => "activate.programmatic",
        },
        WidgetActionKind::Selection(_) => "selection",
        WidgetActionKind::ValueEdit(phase) => match phase {
            WidgetValueEditPhase::Preview => "value.preview",
            WidgetValueEditPhase::Begin => "value.begin",
            WidgetValueEditPhase::Update => "value.update",
            WidgetValueEditPhase::Commit => "value.commit",
            WidgetValueEditPhase::Cancel => "value.cancel",
        },
        WidgetActionKind::Open => "open",
        WidgetActionKind::Close => "close",
        WidgetActionKind::Drag(drag) => match drag.phase {
            WidgetDragPhase::Begin => "drag.begin",
            WidgetDragPhase::Update => "drag.update",
            WidgetDragPhase::Commit => "drag.commit",
            WidgetDragPhase::Cancel => "drag.cancel",
        },
        WidgetActionKind::PointerEdit(edit) => match edit.phase {
            WidgetValueEditPhase::Preview => "pointer.value.preview",
            WidgetValueEditPhase::Begin => "pointer.value.begin",
            WidgetValueEditPhase::Update => "pointer.value.update",
            WidgetValueEditPhase::Commit => "pointer.value.commit",
            WidgetValueEditPhase::Cancel => "pointer.value.cancel",
        },
        WidgetActionKind::TextEdit(_) => "text.edit",
        WidgetActionKind::Scroll(_) => "scroll",
        WidgetActionKind::Focus(focus) if focus.focused => "focus.gained",
        WidgetActionKind::Focus(_) => "focus.lost",
    }
}

fn input_routing_summary(input: &InputRoutingDiagnostic) -> String {
    format!(
        "hovered {}, focused {}, pressed {}, clicked {}, scrolled {}",
        optional_node_label(input.hovered),
        optional_node_label(input.focused),
        optional_node_label(input.pressed),
        optional_node_label(input.clicked),
        optional_node_label(input.scrolled)
    )
}

fn accessibility_request_label(request: &AccessibilityAdapterRequest) -> &'static str {
    match request {
        AccessibilityAdapterRequest::PublishTree { .. } => "accessibility.publish-tree",
        AccessibilityAdapterRequest::MoveFocus { .. } => "accessibility.move-focus",
        AccessibilityAdapterRequest::SetFocusTrap(_) => "accessibility.set-focus-trap",
        AccessibilityAdapterRequest::ClearFocusTrap { .. } => "accessibility.clear-focus-trap",
        AccessibilityAdapterRequest::RestoreFocus(_) => "accessibility.restore-focus",
        AccessibilityAdapterRequest::Announce(_) => "accessibility.announce",
        AccessibilityAdapterRequest::ApplyPreferences(_) => "accessibility.apply-preferences",
    }
}

fn accessibility_request_summary(request: &AccessibilityAdapterRequest) -> String {
    match request {
        AccessibilityAdapterRequest::PublishTree { tree, focused, .. } => format!(
            "publish {} nodes, focused {}",
            tree.nodes.len(),
            optional_node_label(*focused)
        ),
        AccessibilityAdapterRequest::MoveFocus { target, restore } => {
            format!(
                "move focus to {}, restore {}",
                node_label(*target),
                focus_restore_label(*restore)
            )
        }
        AccessibilityAdapterRequest::SetFocusTrap(trap) => focus_trap_summary("set", *trap),
        AccessibilityAdapterRequest::ClearFocusTrap { restore } => {
            format!(
                "clear focus trap, restore {}",
                focus_restore_label(*restore)
            )
        }
        AccessibilityAdapterRequest::RestoreFocus(restore) => {
            format!("restore focus {}", focus_restore_label(*restore))
        }
        AccessibilityAdapterRequest::Announce(announcement) => announcement_summary(announcement),
        AccessibilityAdapterRequest::ApplyPreferences(preferences) => {
            accessibility_preferences_summary(*preferences)
        }
    }
}

fn accessibility_response_label(response: &AccessibilityAdapterResponse) -> &'static str {
    match response {
        AccessibilityAdapterResponse::Applied => "accessibility.applied",
        AccessibilityAdapterResponse::Unsupported(_) => "accessibility.unsupported",
        AccessibilityAdapterResponse::FocusChanged(_) => "accessibility.focus-changed",
        AccessibilityAdapterResponse::PreferencesChanged(_) => "accessibility.preferences-changed",
        AccessibilityAdapterResponse::Failed { .. } => "accessibility.failed",
    }
}

fn accessibility_response_summary(response: &AccessibilityAdapterResponse) -> String {
    match response {
        AccessibilityAdapterResponse::Applied => "applied".to_string(),
        AccessibilityAdapterResponse::Unsupported(kind) => format!("unsupported {:?}", kind),
        AccessibilityAdapterResponse::FocusChanged(focused) => {
            format!("focus changed to {}", optional_node_label(*focused))
        }
        AccessibilityAdapterResponse::PreferencesChanged(preferences) => {
            accessibility_preferences_summary(*preferences)
        }
        AccessibilityAdapterResponse::Failed { request, reason } => {
            format!("failed {:?}: {}", request, reason)
        }
    }
}

fn accessibility_response_severity(response: &AccessibilityAdapterResponse) -> DiagnosticSeverity {
    match response {
        AccessibilityAdapterResponse::Failed { .. } => DiagnosticSeverity::Error,
        AccessibilityAdapterResponse::Unsupported(_) => DiagnosticSeverity::Warning,
        AccessibilityAdapterResponse::Applied
        | AccessibilityAdapterResponse::FocusChanged(_)
        | AccessibilityAdapterResponse::PreferencesChanged(_) => DiagnosticSeverity::Info,
    }
}

fn focus_restore_label(restore: FocusRestoreTarget) -> String {
    match restore {
        FocusRestoreTarget::None => "none".to_string(),
        FocusRestoreTarget::Previous => "previous".to_string(),
        FocusRestoreTarget::Node(node) => node_label(node),
    }
}

fn focus_trap_summary(action: &str, trap: FocusTrap) -> String {
    format!(
        "{} focus trap root {}, restore {}, wrap {}",
        action,
        node_label(trap.root),
        focus_restore_label(trap.restore_focus),
        trap.wrap
    )
}

fn announcement_summary(announcement: &AccessibilityAnnouncement) -> String {
    format!(
        "announce {:?} from {}: {}",
        announcement.live_region,
        optional_node_label(announcement.source),
        announcement.message
    )
}

fn accessibility_preferences_summary(preferences: AccessibilityPreferences) -> String {
    format!(
        "screen_reader {}, reduced_motion {}, high_contrast {}, text_scale {:.2}",
        preferences.screen_reader_active,
        preferences.reduced_motion,
        preferences.should_use_high_contrast(),
        preferences.normalized_text_scale()
    )
}

fn geometry_hit_summary(hit: &GeometryHitDiagnostic) -> String {
    match hit.point_hit {
        Some(true) => format!("hit at z {}, order {}", hit.resolved_z, hit.order),
        Some(false) => format!(
            "miss at z {}, order {}, rejected by {:?}",
            hit.resolved_z, hit.order, hit.rejected_by
        ),
        None => format!(
            "eligible {}, visible {}, hit_testable {}",
            hit.rejected_by.is_empty(),
            hit.visible,
            hit.hit_testable
        ),
    }
}

fn performance_snapshot_summary(performance: &PerformanceSnapshotDiagnostic) -> String {
    let slowest = performance.slowest_stage.as_deref().unwrap_or("none");
    if performance.missing_stage_labels.is_empty() && performance.missing_cache_labels.is_empty() {
        format!(
            "frame {}, total {}, slowest {}, {} caches",
            performance.frame,
            duration_label(performance.total),
            slowest,
            performance.caches.len()
        )
    } else {
        format!(
            "frame {}, missing {} stages and {} caches, total {}, slowest {}",
            performance.frame,
            performance.missing_stage_labels.len(),
            performance.missing_cache_labels.len(),
            duration_label(performance.total),
            slowest
        )
    }
}

fn dirty_flags_summary(flags: DirtyFlags) -> String {
    let labels = dirty_flag_labels(flags);
    if labels.is_empty() {
        "clean".to_string()
    } else {
        format!("dirty {}", labels.join(", "))
    }
}

fn dirty_flag_labels(flags: DirtyFlags) -> Vec<&'static str> {
    let mut labels = Vec::new();
    if flags.layout {
        labels.push("layout");
    }
    if flags.paint {
        labels.push("paint");
    }
    if flags.input {
        labels.push("input");
    }
    if flags.theme {
        labels.push("theme");
    }
    if flags.text_measurement {
        labels.push("text_measurement");
    }
    labels
}

fn duration_label(duration: Duration) -> String {
    format!("{:.3}ms", duration.as_secs_f64() * 1000.0)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::diagnostics::{required_pipeline_stages, CacheDiagnostic, FramePipelineTiming};
    use crate::{
        AccessibilityAnnouncement, AccessibilityCapabilities, AccessibilityLiveRegion,
        AccessibilityTree, EffectiveClip, EffectiveGeometry, FrameTiming, OverlayEntry,
        OverlayKind, UiPoint, UiRect, WidgetActionBinding,
    };

    #[test]
    fn combines_hit_routing_overlay_state_and_actions() {
        let input = UiInputResult {
            hovered: Some(UiNodeId(1)),
            focused: Some(UiNodeId(2)),
            clicked: Some(UiNodeId(3)),
            ..UiInputResult::default()
        };
        let mut stack = OverlayStack::new();
        stack.push(OverlayEntry::new(
            OverlayId(10),
            OverlayKind::Menu,
            UiRect::new(0.0, 0.0, 100.0, 100.0),
        ));
        stack.push(
            OverlayEntry::new(
                OverlayId(11),
                OverlayKind::Dialog,
                UiRect::new(20.0, 20.0, 80.0, 80.0),
            )
            .modal(true)
            .layer(10),
        );
        let decision = stack.route_pointer_down(UiPoint::new(25.0, 25.0));
        let action = WidgetAction::pointer_activate(
            UiNodeId(3),
            WidgetActionBinding::action("toolbar.run"),
            1,
        );

        let mut report = DiagnosticReport::new();
        report
            .input_routing(&input)
            .overlay_stack(&stack)
            .overlay_routing(&decision)
            .widget_action(&action);

        assert_eq!(report.records.len(), 4);
        assert!(report
            .summaries_by_category(DiagnosticCategory::InputRouting)
            .next()
            .unwrap()
            .summary
            .contains("clicked node:3"));
        assert!(report.summaries.iter().any(|summary| {
            summary.category == DiagnosticCategory::OverlayStack
                && summary.summary.contains("topmost overlay:11")
        }));
        assert!(report.summaries.iter().any(|summary| {
            summary.category == DiagnosticCategory::WidgetAction
                && summary.label == "action:toolbar.run:activate.pointer"
        }));
    }

    #[test]
    fn summarizes_accessibility_requests_responses_and_output() {
        let request = AccessibilityAdapterRequest::Announce(
            AccessibilityAnnouncement::new("Saved", AccessibilityLiveRegion::Polite)
                .source(UiNodeId(7)),
        );
        let mut state = AccessibilityAdapterState::new();
        state.publish_tree(
            AccessibilityTree {
                nodes: Vec::new(),
                focus_order: Vec::new(),
                modal_scope: None,
            },
            Some(UiNodeId(7)),
            AccessibilityPreferences::DEFAULT.screen_reader_active(true),
        );
        state.announce(AccessibilityAnnouncement::assertive("Done"));

        let mut apply = AccessibilityAdapterApplyReport::new();
        apply.record(
            AccessibilityRequestKind::PublishTree,
            AccessibilityAdapterResponse::Unsupported(AccessibilityRequestKind::PublishTree),
        );
        apply.record(
            AccessibilityRequestKind::Announce,
            AccessibilityAdapterResponse::Failed {
                request: AccessibilityRequestKind::Announce,
                reason: "adapter offline".to_string(),
            },
        );

        let mut report = DiagnosticReport::new();
        report
            .accessibility_request(&request)
            .accessibility_output(&state)
            .accessibility_apply_report(&apply);

        assert_eq!(report.highest_severity(), Some(DiagnosticSeverity::Error));
        assert!(report.summaries.iter().any(|summary| {
            summary.label == "accessibility-output"
                && summary.summary.contains("focused node:7")
                && summary.summary.contains("1 announcements")
        }));
        assert!(report.summaries.iter().any(|summary| {
            summary.severity == DiagnosticSeverity::Warning
                && summary.summary.contains("unsupported PublishTree")
        }));
        assert!(report.summaries.iter().any(|summary| {
            summary.severity == DiagnosticSeverity::Error
                && summary.summary.contains("adapter offline")
        }));
    }

    #[test]
    fn summarizes_render_timing_and_dirty_flags() {
        let timing = FrameTiming::new()
            .section("layout", Duration::from_millis(2))
            .section("paint", Duration::from_micros(500));
        let dirty = DirtyFlags {
            layout: true,
            paint: true,
            ..DirtyFlags::NONE
        };

        let mut report = DiagnosticReport::new();
        report.render_timing(&timing).dirty_flags(dirty);

        assert!(report.summaries.iter().any(|summary| {
            summary.category == DiagnosticCategory::RenderTiming
                && summary.summary == "2 sections, total 2.500ms"
        }));
        assert!(report.summaries.iter().any(|summary| {
            summary.category == DiagnosticCategory::DirtyState
                && summary.summary == "dirty layout, paint"
        }));
    }

    #[test]
    fn summarizes_performance_snapshot_diagnostics() {
        let pipeline = required_pipeline_stages()
            .into_iter()
            .fold(FramePipelineTiming::new(), |pipeline, stage| {
                pipeline.stage(stage, Duration::from_micros(2))
            });
        let complete = PerformanceSnapshot::new(9)
            .pipeline(pipeline)
            .cache(CacheDiagnostic::new("layout").lookup(true))
            .cache(CacheDiagnostic::new("shaped-text").lookup(true))
            .cache(CacheDiagnostic::new("image").lookup(true))
            .cache(CacheDiagnostic::new("canvas-texture").lookup(false))
            .cache(CacheDiagnostic::new("display-list").lookup(true));

        let mut report = DiagnosticReport::new();
        report.performance_snapshot(&complete);

        assert_eq!(report.highest_severity(), Some(DiagnosticSeverity::Info));
        assert!(report.summaries.iter().any(|summary| {
            summary.category == DiagnosticCategory::Performance
                && summary.summary.contains("frame 9")
                && summary.summary.contains("5 caches")
        }));

        let incomplete = PerformanceSnapshot::new(10).cache(CacheDiagnostic::new("display-list"));
        let mut report = DiagnosticReport::new();
        report.performance_snapshot(&incomplete);

        assert_eq!(report.highest_severity(), Some(DiagnosticSeverity::Warning));
        assert!(matches!(
            &report.records[0],
            DiagnosticRecord::PerformanceSnapshot(performance)
                if performance.missing_stage_labels.contains(&"tree-build".to_string())
                    && performance.missing_cache_labels.contains(&"layout".to_string())
        ));
    }

    #[test]
    fn classifies_severity_for_geometry_and_messages() {
        let geometry = EffectiveGeometry::new(UiNodeId(4), UiRect::new(0.0, 0.0, 10.0, 10.0))
            .clip(EffectiveClip::new(UiRect::new(0.0, 0.0, 5.0, 5.0)));
        let record = geometry.diagnostic_record_for_point(UiPoint::new(7.0, 7.0));

        let mut report = DiagnosticReport::new();
        report
            .geometry_hit(&record)
            .warning("cache", "evicted display list");

        assert!(matches!(
            &report.records[0],
            DiagnosticRecord::GeometryHit(hit)
                if hit.rejected_by == vec![EffectiveHitRejection::OutsideClipChain]
        ));
        assert_eq!(report.records[0].severity(), DiagnosticSeverity::Trace);
        assert_eq!(report.highest_severity(), Some(DiagnosticSeverity::Warning));
        assert!(report.summaries.iter().any(|summary| {
            summary.category == DiagnosticCategory::Warning
                && summary.summary == "evicted display list"
        }));
    }

    #[test]
    fn accessibility_request_plan_can_feed_diagnostics() {
        let requests = vec![
            AccessibilityAdapterRequest::ApplyPreferences(
                AccessibilityPreferences::DEFAULT.high_contrast(true),
            ),
            AccessibilityAdapterRequest::RestoreFocus(FocusRestoreTarget::Previous),
        ];
        let plan = crate::AccessibilityAdapterRequestPlan::from_requests(
            &requests,
            AccessibilityCapabilities::NONE,
        );

        let mut report = DiagnosticReport::new();
        report
            .accessibility_requests(&requests)
            .accessibility_apply_report(&AccessibilityAdapterApplyReport {
                supported: Vec::new(),
                unsupported: plan
                    .unsupported_responses
                    .iter()
                    .filter_map(|response| match response {
                        AccessibilityAdapterResponse::Unsupported(kind) => Some(*kind),
                        _ => None,
                    })
                    .collect(),
                responses: plan.unsupported_responses,
            });

        assert_eq!(
            report
                .summaries
                .iter()
                .filter(|summary| summary.severity == DiagnosticSeverity::Warning)
                .count(),
            2
        );
    }
}
