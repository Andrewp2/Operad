//! Diagnostics, error, limit, and test-support APIs.

#[cfg(any(test, feature = "diagnostics"))]
pub mod debug;
pub mod errors;
pub mod limits;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;

#[cfg(any(test, feature = "diagnostics"))]
pub mod performance;
#[cfg(any(test, feature = "diagnostics"))]
pub mod report;

pub use crate::core::document::{AuditAxis, AuditWarning, ScrollbarAuditState};

#[cfg(any(test, feature = "diagnostics"))]
pub use performance::{
    required_cache_diagnostic_kinds, required_pipeline_stages, CacheDiagnostic,
    CacheDiagnosticKind, FramePipelineSection, FramePipelineStage, FramePipelineTiming,
    FrameTimingExplanation, FrameTimingSectionExplanation, FrameTimingStageExplanation,
    PerformanceSnapshot,
};
#[cfg(any(test, feature = "diagnostics"))]
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
