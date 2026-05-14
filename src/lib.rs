//! Operad is a small retained-mode UI document layer.
//!
//! The crate intentionally contains only reusable primitives: layout, text
//! measurement, hit testing, focus, animation, and optional renderer backends.
//! Product-specific screens and game/application state should live in the
//! consuming application crate.

pub mod accessibility;
#[cfg(feature = "accesskit-winit")]
#[path = "adapters/accesskit_winit_adapter.rs"]
pub mod accesskit_winit_adapter;
#[path = "interaction/actions.rs"]
pub mod actions;
pub mod adapters;
#[path = "render/assets.rs"]
pub mod assets;
#[path = "domain/charts.rs"]
pub mod charts;
#[path = "interaction/commands.rs"]
pub mod commands;
#[path = "render/compositor.rs"]
pub mod compositor;
pub mod core;
pub use core::document::*;
#[path = "diagnostics/debug.rs"]
pub mod debug;
pub mod diagnostics;
#[path = "render/display.rs"]
pub mod display;
pub mod domain;
#[path = "interaction/drag_drop.rs"]
pub mod drag_drop;
#[path = "domain/editor.rs"]
pub mod editor;
#[path = "render/effective_geometry.rs"]
pub mod effective_geometry;
#[cfg(feature = "egui")]
#[path = "adapters/egui_host.rs"]
pub mod egui_host;
#[path = "diagnostics/errors.rs"]
pub mod errors;
#[path = "render/fonts.rs"]
pub mod fonts;
#[path = "interaction/forms.rs"]
pub mod forms;
#[path = "runtime/host.rs"]
pub mod host;
#[path = "core/i18n.rs"]
pub mod i18n;
#[path = "interaction/input.rs"]
pub mod input;
#[path = "interaction/input_devices.rs"]
pub mod input_devices;
pub mod interaction;
#[path = "core/layout.rs"]
pub mod layout;
#[path = "diagnostics/limits.rs"]
pub mod limits;
#[path = "interaction/navigation.rs"]
pub mod navigation;
#[path = "interaction/overlays.rs"]
pub mod overlays;
#[path = "render/paint.rs"]
pub mod paint;
#[path = "runtime/platform.rs"]
pub mod platform;
pub mod prelude;
pub mod render;
#[path = "render/renderer.rs"]
pub mod renderer;
#[path = "render/resource_cache.rs"]
pub mod resource_cache;
pub mod runtime;
#[path = "render/scrolling.rs"]
pub mod scrolling;
pub mod shell;
#[path = "core/state.rs"]
pub mod state;
#[path = "interaction/tasks.rs"]
pub mod tasks;
#[path = "diagnostics/testing.rs"]
pub mod testing;
pub mod theme;
#[path = "theme/stability.rs"]
pub mod theme_stability;
#[path = "accessibility/tooltips.rs"]
pub mod tooltips;
#[path = "interaction/transactions.rs"]
pub mod transactions;
#[path = "core/versioning.rs"]
pub mod versioning;
#[path = "render/virtualization.rs"]
pub mod virtualization;
#[cfg(feature = "wgpu")]
#[path = "adapters/wgpu_renderer.rs"]
pub mod wgpu_renderer;
#[cfg(feature = "widgets")]
#[path = "widgets/ext/mod.rs"]
mod widget_ext;
#[cfg(feature = "widgets")]
pub mod widgets;
#[path = "runtime/windows.rs"]
pub mod windows;

pub use accessibility::{
    AccessibilityAdapter, AccessibilityAdapterApplyReport, AccessibilityAdapterRequest,
    AccessibilityAdapterRequestPlan, AccessibilityAdapterResponse, AccessibilityAdapterState,
    AccessibilityAdapterTargetKind, AccessibilityAdapterTargetSummary, AccessibilityAnnouncement,
    AccessibilityCapabilities, AccessibilityFocusTrapState, AccessibilityLiveRegionEntry,
    AccessibilityLiveRegionSnapshot, AccessibilityNavigableItem, AccessibilityNavigableItemSource,
    AccessibilityPreferences, AccessibilityRequestKind, FocusNavigationDirection,
    FocusRestoreTarget, FocusTrap, HeadlessAccessibilityAdapter,
};
#[cfg(feature = "accesskit-winit")]
pub use accesskit_winit_adapter::{
    accesskit_node_id, accesskit_tree_update, operad_node_id, AccessKitTreeOptions,
    AccessKitWinitAdapter, ACCESSKIT_ROOT_NODE_ID, ACCESSKIT_WINIT_CAPABILITIES,
};
pub use actions::{
    action_target_enabled, keyboard_activation_key, WidgetAction, WidgetActionBinding,
    WidgetActionId, WidgetActionKind, WidgetActionMode, WidgetActionQueue, WidgetActionTrigger,
    WidgetActivation, WidgetDrag, WidgetDragPhase, WidgetFocusChange, WidgetPointerEdit,
    WidgetSelection, WidgetTextEdit, WidgetValueEditPhase,
};
pub use assets::{
    AssetRegistry, BuiltInIcon, IconAsset, IconButtonAsset, IconDescriptor, ImageDescriptor,
};
pub use charts::{
    ChartAxisMeta, ChartAxisOrientation, ChartAxisTick, ChartDataSummary, ChartHitCollection,
    ChartHitKind, ChartHitMeta, ChartOverlayKind, ChartOverlayLayer, ChartOverlayStack, ChartRange,
    ChartSample, ChartSelectionSummary, ChartSeriesId, ChartViewport, GridCell, GridCellRange,
    GridMapCellMeta, GridMapGeometry, GridMapSummary, SparklineGeometry,
};
pub use commands::{
    Command, CommandEffect, CommandEffectInvocation, CommandId, CommandMeta, CommandRegistry,
    CommandRegistryError, CommandScope, Shortcut, ShortcutBinding, ShortcutConflict,
};
pub use compositor::{
    plan_render_feature_fallbacks, sort_layers_for_paint, topmost_layer_at, AffineTransform,
    BlendMode, ColorManagementLevel, CompositorClip, CompositorFilter, CompositorFilterKind,
    CompositorLayer, CompositorLayerId, CompositorMask, CompositorScene, FeatureFallbackAction,
    FeatureSupportLevel, MaskMode, OffscreenIsolation, OffscreenReason, RenderFeature,
    RenderFeaturePlan, RenderFeaturePlanItem, RenderFeatureRequirement, RenderFeatureSupport,
    StackingContext, StackingContextId, SubpixelTextPolicy,
};
pub use debug::{
    layout_snapshot_dump, DebugGestureKind, DebugGestureState, DebugHitCandidate, DebugHitTrace,
    DebugOverlayContext, DebugOverlayNode, DebugOverlayOptions, DebugOverlaySnapshot,
    DebugPaintDump, DebugPaintItem, DebugPaintKindCount, DebugPaintStats, DebugThemeComponentState,
    DebugThemeScopeInfo, DebugThemeSnapshot, DebugThemeToken, DebugThemeTokenKind,
};
pub use diagnostics::{
    node_label, overlay_label, required_cache_diagnostic_kinds, required_pipeline_stages,
    widget_action_label, AccessibilityOutputDiagnostic, AccessibilityRequestDiagnostic,
    AccessibilityResponseDiagnostic, CacheDiagnostic, CacheDiagnosticKind, DiagnosticCategory,
    DiagnosticMessage, DiagnosticRecord, DiagnosticReport, DiagnosticSeverity,
    DiagnosticSummaryRecord, DirtyFlagsDiagnostic, FramePipelineSection, FramePipelineStage,
    FramePipelineTiming, GeometryHitDiagnostic, InputRoutingDiagnostic, OverlayEntryDiagnostic,
    OverlayRoutingDiagnostic, OverlayStackDiagnostic, PerformanceCacheDiagnostic,
    PerformanceSnapshot, PerformanceSnapshotDiagnostic, RenderTimingDiagnostic,
    RenderTimingSectionDiagnostic, WidgetActionDiagnostic,
};
pub use display::{
    DisplayListId, DisplayListInvalidation, DisplayListInvalidationReport,
    DisplayListInvalidationRequest, DisplayListKey, DisplayListKind, DisplayListReuseOutcome,
    DisplayListReuseReport, DisplayListScope, RetainedDisplayList, RetainedDisplayListCache,
};
pub use drag_drop::{
    payload_has_content, DragDropSurfaceKind, DragSourceDescriptor, DragSourceId,
    DropPayloadFilter, DropTargetDescriptor, DropTargetHit, DropTargetId,
};
pub use editor::{
    generate_ruler_ticks, CurveEditorGeometry, CurveInterpolation, CurvePoint, CurveSegment,
    EditorAccessibleTarget, EditorAxisRange, EditorCursor, EditorHitId, EditorHitKind,
    EditorHitTarget, EditorHitTest, EditorHitTester, EditorOverlay, EditorOverlayStack,
    EditorSurfaceAccessibility, EditorSurfaceId, EditorSurfaceState, EditorToolId, EditorToolMode,
    EditorTransform, LaneGeometry, LaneTimelineGeometry, LaneValueGeometry, LaneValueRange,
    LaneValueRangeItem, MarqueeSelection, RulerTickConfig, SnapGrid, TimelineGeometry,
    TimelineRangeItem, TimelineRangeItemEdge, TimelineRangeItemGeometry, VisibleLaneRange,
};
pub use effective_geometry::{
    accessibility_bounds, clipped_visible_rect, effective_geometry_records,
    effective_hit_test_records, topmost_effective_hit, transformed_bounds,
    EffectiveAccessibilityBounds, EffectiveAccessibilityBoundsSource, EffectiveClip,
    EffectiveGeometry, EffectiveGeometryRecord, EffectiveHit, EffectiveHitEligibility,
    EffectiveHitRejection, EffectiveTransform,
};
#[cfg(feature = "egui")]
pub use egui_host::{
    egui_cursor_icon, egui_host_capabilities, egui_key, egui_modifiers, egui_pointer_button,
    egui_texture_id_for_resource, EguiAccessibilityOutputPlan, EguiHostAdapter, EguiInputAdapter,
    EguiPlatformOutputPlan, EguiTextureDeltaPlan,
};
pub use errors::{
    classify_platform_error, classify_render_error, ErrorContext, ErrorDomain, ErrorKind,
    ErrorReport, ErrorSeverity, FallbackAction, FallbackDecision, FallbackScope, PlatformErrorKind,
    RendererErrorKind, ResourceErrorKind, RuntimeErrorKind, UiErrorKind,
};
pub use fonts::{
    CachedFontFace, FontCachePolicy, FontEvictionCandidate, FontEvictionPlan, FontFaceDescriptor,
    FontFaceId, FontFallbackAttempt, FontFallbackResolution, FontFallbackStack, FontFamilyId,
    FontGeneration, FontLifecycleIssue, FontLifecycleOutcome, FontLifecycleReport, FontLoadStatus,
    FontRegistry, FontSourceDescriptor,
};
pub use forms::{
    AccessibleErrorSummaryRecord, FieldId, FieldState, FieldValidationRequest,
    FieldValidationResult, FormId, FormPhase, FormState, FormValidationRequest,
    FormValidationResult, ValidationApplyDisposition, ValidationGeneration, ValidationMessage,
    ValidationSeverity,
};
pub use host::{
    process_document_frame, process_shell_frame, text_input_id_for_node, HostAccessibilityState,
    HostAdapter, HostAdapterError, HostCommandDispatch, HostDocumentFrameOutput,
    HostDocumentFrameRequest, HostDocumentFrameState, HostFrameOutput, HostFrameRequest,
    HostInteractionState, HostNodeInteraction, HostShellEvent, HostShellFrameOutput,
    HostShellFrameRequest, HostShortcutRoute,
};
pub use i18n::{
    BidiPolicy, DynamicLabelMeta, LabelUpdatePolicy, LayoutMirrorMode, LocaleId,
    LocaleIdentifierError, LocalizationPolicy, ResolvedTextDirection, TextDirection,
};
pub use input::{
    DragGesture, GestureEvent, GesturePhase, GestureSettings, PointerButton, PointerButtons,
    PointerCapture, PointerClick, PointerEventKind, PointerGestureTracker, PointerId, PointerKind,
    RawInputEvent, RawKeyboardEvent, RawPointerEvent, RawTextInputEvent, RawWheelEvent,
    WheelDeltaUnit, WheelPhase,
};
pub use input_devices::{
    classify_touch_gesture, map_gamepad_navigation, route_cancel_for_capture,
    route_stylus_metadata, CancelCaptureRouteReport, GamepadAxis, GamepadButton, GamepadDeviceId,
    GamepadInput, GamepadNavigationAction, GamepadNavigationPolicy, GamepadNavigationReport,
    PointerCaptureInteraction, StylusButton, StylusContactPhase, StylusMetadata,
    StylusMetadataField, StylusMetadataSupport, StylusRouteReport, TouchGestureClassification,
    TouchGestureKind, TouchGesturePolicy, TouchGestureSample,
};
pub use layout::{
    Layout, LayoutAlignment, LayoutDimension, LayoutDisplay, LayoutFlexDirection, LayoutGap,
    LayoutInset, LayoutInsets, LayoutJustifyContent, LayoutLength, LayoutPosition, LayoutSize,
    LayoutSpacing,
};
pub use limits::{
    drag_payload_byte_len, truncate_str_to_byte_limit, validate_cache_budget,
    validate_drag_drop_payload, validate_font_bytes, validate_image_dimensions,
    validate_paste_payload, validate_resource_descriptor, validate_resource_id,
    validate_resource_update, validate_text_input, validate_texture_size,
    validate_virtualized_row_count, CacheBudgetAction, CacheBudgetDecision, DimensionLimit,
    InputLimitPolicy, LimitKind, LimitPolicy, LimitReport, LimitStatus, LimitValue,
    ResourceLimitPolicy,
};
pub use navigation::{
    next_enabled_item, NavigationAction, NavigationBoundaryBehavior, NavigationCollectionKind,
    NavigationContract, NavigationDirection as CompositeNavigationDirection, NavigationFocusModel,
    NavigationItem, NavigationItemId, NavigationKeyResult, NavigationOrientation, NavigationState,
};
pub use overlays::{
    OverlayDismissOutcome, OverlayDismissPolicy, OverlayDismissReason, OverlayEntry,
    OverlayFocusRestoreRecord, OverlayFocusRestoreTarget, OverlayHitTestDecision, OverlayId,
    OverlayKind, OverlayStack,
};
#[cfg(feature = "wgpu")]
pub use wgpu_renderer::{
    WgpuCanvasContext, WgpuCanvasRenderPass, WgpuRenderer, WgpuSurfaceRenderer,
};

pub use paint::{
    AlignedStroke, CornerRadii, GradientStop, ImageAlignment, ImageFit, LinearGradient, PaintBrush,
    PaintEffect, PaintEffectKind, PaintImage, PaintPath, PaintRect, PaintText, PathFillRule,
    PathStrokeOptions, PathVerb, PixelSnapPolicy, StrokeAlignment, StrokeLineCap, StrokeLineJoin,
    TextHorizontalAlign, TextOverflow, TextVerticalAlign,
};
pub use renderer::{
    CanvasHitCollection, CanvasHitTarget, CanvasHostCaptureChange, CanvasHostCaptureChangeKind,
    CanvasHostCaptureId, CanvasHostCapturePlan, CanvasHostCaptureState,
    CanvasHostCaptureTransition, CanvasRenderContext, CanvasRenderHandler, CanvasRenderOutcome,
    CanvasRenderOutput, CanvasRenderRegistry, CanvasRenderReport, CanvasRenderRequest,
    DirtyRegionSet, ImageRenderContext, ImageRenderHandler, ImageRenderKind, ImageRenderOutcome,
    ImageRenderOutput, ImageRenderRegistry, ImageRenderReport, ImageRenderRequest, PaintBatch,
    PaintBatchKey, PaintBatchKind, PaintBatcher, PixelRect, RenderError, RenderFrameOutput,
    RenderFrameRequest, RenderOptions, RenderTarget, RenderTargetKind, RenderedImage,
    RendererAdapter, ResourceDescriptor, ResourceFormat, ResourceResolver, ResourceUpdate,
};
pub use resource_cache::{
    resource_descriptor_byte_len, validate_resource_update as validate_resource_cache_update,
    validate_resource_update_with_descriptor as validate_resource_cache_update_with_descriptor,
    CachedResource, ResourceCache, ResourceCachePolicy, ResourceEvictionCandidate,
    ResourceEvictionPlan, ResourceEvictionReason, ResourceEvictionReport, ResourceLifecycleOutcome,
    ResourceUpdateIssue, ResourceUpdateKind, ResourceUpdateReport, ResourceUpdateValidation,
};
#[cfg(feature = "native-window")]
pub use runtime::native::{
    run_app, run_app_with, run_app_with_canvas_renderers, run_ui_document, run_ui_document_with,
    run_ui_document_with_canvas_renderers, NativeWgpuCanvasRenderContext,
    NativeWgpuCanvasRenderHandler, NativeWgpuCanvasRenderRegistry, NativeWindowOptions,
    NativeWindowResult,
};
pub use runtime::{
    coalesce_repaint_requests, collect_repaint_requests, completed_platform_response,
    RuntimeFrameClock, RuntimeFramePhase, RuntimeFramePlan, RuntimeInvalidation,
    RuntimeInvalidationReason, RuntimeLoopGuard, RuntimeLoopState, RuntimePhaseTrace,
    RuntimeRepaintScheduler, RuntimeSurfaceId, RuntimeWindowEvent, RuntimeWindowId,
};
pub use scrolling::{
    apply_scroll_anchor, arbitrate_nested_scroll, content_viewport_rect, resolve_scroll_attachment,
    resolve_scroll_delta, resolve_sticky_rect, reveal_rect_into_view, scrollbar_thumb_rect,
    step_kinetic_scroll, synchronize_scroll_surfaces, FixedConstraints, KineticScrollConfig,
    KineticScrollState, KineticScrollStep, NestedScrollCandidate, NestedScrollPlan,
    NestedScrollStep, OverscrollBehavior, OverscrollPolicy, ProgrammaticScrollBehavior,
    RevealAlignment, RevealOptions, RevealScrollPlan, ScrollAnchorAdjustment,
    ScrollAnchorCandidate, ScrollAttachment, ScrollAttachmentMode, ScrollAttachmentResolution,
    ScrollAxis, ScrollInsets, ScrollSyncMode, ScrollSyncPlan, ScrollSyncSurface, ScrollSyncUpdate,
    ScrollbarPolicy, ScrollbarVisibility, StickyConstraints, StickyEdges,
};
pub use shell::{
    build_shell_document, DockPlacement, PersistentSplitState, ScrollSyncAxes, ScrollSyncGroup,
    ScrollSyncMember, ShellBarCluster, ShellBarItem, ShellBarItemLayout, ShellBarItemRole,
    ShellBarLayoutPlan, ShellBarLayoutSpacing, ShellBarOverflowItem, ShellBarOverflowPolicy,
    ShellDocumentNodes, ShellDocumentOptions, ShellExtent, ShellLayoutPlan, ShellNumericReadout,
    ShellPanelDocumentNode, ShellPanelLayout, ShellPanelState, ShellRegion,
    ShellRegionDocumentNode, ShellRegionLayout, ShellWorkspaceState, SplitPaneSide,
};
pub use state::{
    RetainedWidgetStateEntry, RetainedWidgetStateStore, WidgetEditState, WidgetFocusState,
    WidgetHoverState, WidgetId, WidgetKey, WidgetOverlayState, WidgetPressedState,
    WidgetStateError, WidgetStateInvalidationSummary, WidgetStateKey, WidgetStateLifecycleItem,
    WidgetStateLifecycleOutcome, WidgetStateLifecycleReport, WidgetStateRetention,
    WidgetStateScope, WidgetStateSlotDescriptor, WidgetStateSlotId, WidgetStateSlotKind,
    WidgetStateUpdateReport, WidgetStateValue,
};
pub use tasks::{
    TaskError, TaskGeneration, TaskHandle, TaskId, TaskInvalidationSummary, TaskLifecycleReport,
    TaskProgress, TaskRegistry, TaskResultDisposition, TaskState, TaskStatus,
};
pub use testing::{
    diff_rgba8, AccessibilityAssertions, AccessibilityRequestAssertions, AuditAssertions,
    CommandReplayReport, CommandReplayStepResult, DirtyFlags, DisplayListInvalidationAssertions,
    DisplayListReuseAssertions, DisplayListReuseSeries, DisplayListReuseSeriesAssertions,
    EmptyResourceResolver, EventReplay, EventReplayReport, EventReplayStep, EventReplayStepResult,
    FrameTiming, FrameTimingAssertions, FrameTimingSection, FrameTimingSeries,
    FrameTimingSeriesAssertions, LayoutAssertions, PaintAssertions, PaintKindSelector,
    PaintRecorderRenderer, PerformanceAssertions, PerformanceSamples, PixelDiffReport,
    PixelDiffTolerance, PlatformAssertions, RenderAssertions, RenderOutputAssertions, ReplayInput,
    RgbaImageView, ScenarioFrameReport, ScenarioHarness, SnapshotAssertions, TestFailure,
    TestResult,
};
pub use theme::{
    color_with_alpha, text_style_with_color, text_style_with_scale, ColorTokens,
    ComponentIconStates, ComponentLayoutTokens, ComponentRole, ComponentState, ComponentStateSlot,
    ComponentStyle, ComponentTextStates, ComponentTokens, ComponentVisualStates, EffectTokens,
    IconStyle, LayerEffect, LayerEffectKind, MotionCurve, MotionTokens, OpacityTokens,
    RadiusTokens, ScopedThemeRegistry, SpacingTokens, StrokeTokens, Theme, ThemePatch, ThemeScope,
    ThemeScopeError, ThemeScopeId, ThemeScopeKind, TypographyTokens, OPERAD_DARK_THEME_NAME,
};
pub use theme_stability::{
    stable_theme_token_categories, theme_feature_stability, theme_scope_stability,
    theme_token_stability, ThemeScopeStability, ThemeStabilityScope, ThemeTokenCategory,
    ThemeTokenStability, THEME_FEATURE_STABILITY, THEME_SCOPE_STABILITY, THEME_TOKEN_STABILITY,
};
pub use tooltips::{
    clamp_context_menu_position, keyboard_context_menu_key, keyboard_context_menu_position,
    resolve_context_menu_request, resolve_tooltip_dismissal, resolve_tooltip_request,
    AccessibleHelpText, CommandTooltip, CommandTooltipResolver, ContextMenuRequest,
    ContextMenuResolution, ContextMenuSuppressedReason, ContextMenuTrigger, HelpDismissReason,
    HelpItemState, HelpOverlayRecord, HelpTimingPolicy, ShortcutDisplayPlatform, ShortcutFormatter,
    TooltipAnchor, TooltipContent, TooltipInvocationKind, TooltipPlacement, TooltipRequest,
    TooltipResolution, TooltipVisibility, ValidationHelp, ValidationHelpSeverity,
};
pub use transactions::{
    EditTransaction, EditTransactionPhase, SelectionMode, SelectionModel, SelectionMovement,
    TextEditChange, TextEditHistory, TextEditHistoryApply, TextEditHistoryDirection,
    TextEditRecord, TextEditTransaction, TransactionError, TransactionId, TransactionTarget,
    TransactionWidgetId, UndoRedoCommandDescriptors,
};
pub use versioning::{
    ApiStability, ApiStabilityMarker, ApiStatus, BackendSpecific, Experimental, FeatureStability,
    MigrationOnly, StabilityNote, Stable,
};
pub use virtualization::{
    plan_virtualized_range, virtual_offset_for_index, virtual_scroll_anchor_adjustment,
    VirtualAccessibilityRecord, VirtualAxis, VirtualCollectionKind, VirtualExtent,
    VirtualFocusPreservation, VirtualItemKey, VirtualItemPlan, VirtualMeasuredExtent,
    VirtualOverscan, VirtualPlan, VirtualPlanRequest, VirtualScrollAnchor,
    VirtualScrollAnchorAdjustment, VirtualSelectionPreservation, VirtualStickyEdge,
    VirtualStickyRegion,
};
pub use windows::{
    DocumentId, OverlayId as WindowOverlayId, OverlayOwner, RenderSurfaceOwner, RoutedRenderTarget,
    RoutedWindowEvent, SurfaceId, WindowDocumentTarget, WindowId, WindowRouteEventKind,
    WindowRouteRejection, WindowRouter, WindowRoutingState, WindowRoutingSummary,
};
