//! Diagnostics, error, limit, and test-support APIs.

pub mod performance;
pub mod report;

pub use crate::core::document::{AuditAxis, AuditWarning, ScrollbarAuditState};
pub use crate::{debug, errors, limits, testing};
pub use performance::{
    required_cache_diagnostic_kinds, required_pipeline_stages, CacheDiagnostic,
    CacheDiagnosticKind, FramePipelineSection, FramePipelineStage, FramePipelineTiming,
    FrameTimingExplanation, FrameTimingSectionExplanation, FrameTimingStageExplanation,
    PerformanceSnapshot,
};
pub use report::{
    node_label, overlay_label, widget_action_label, AccessibilityOutputDiagnostic,
    AccessibilityRequestDiagnostic, AccessibilityResponseDiagnostic, DiagnosticCategory,
    DiagnosticMessage, DiagnosticRecord, DiagnosticReport, DiagnosticSeverity,
    DiagnosticSummaryRecord, DirtyFlagsDiagnostic, DirtyInvalidationExplanation,
    DirtyStateExplanation, DirtyStateSubsystem, DirtySubsystemExplanation, GeometryHitDiagnostic,
    InputRoutingDiagnostic, JustWorkIssueDiagnostic, JustWorkIssueKind, OverlayEntryDiagnostic,
    OverlayRoutingDiagnostic, OverlayStackDiagnostic, PerformanceCacheDiagnostic,
    PerformanceSnapshotDiagnostic, RenderTimingDiagnostic, RenderTimingSectionDiagnostic,
    WidgetActionDiagnostic,
};
