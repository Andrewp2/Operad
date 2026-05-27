//! Additional domain-neutral widgets built on the core widget primitives.
#![allow(unused_imports)]

pub mod data;
pub mod menu;
pub mod pickers;
pub mod surfaces;

pub mod color_picker;
pub mod command_diagnostics;
pub mod command_palette;
pub mod context_menu;
pub mod data_table;
pub mod date_picker;
pub mod debug_inspector;
pub mod dialog;
pub mod dock_workspace;
pub mod dropdown;
pub mod editable_form;
pub mod floating_window;
pub mod menu_bar;
pub mod menu_list;
pub mod numeric_input;
pub mod path_picker;
pub mod popover;
pub mod progress_indicator;
pub mod property_inspector;
pub mod split_pane;
pub mod tab_group;
pub mod theme_editor;
pub mod timeline_ruler;
pub mod toast;
pub mod toggle_control;
pub mod tree_view;

pub use color_picker::{
    color_edit_button, color_picker, color_swatch_button, compact_color_button, show_color,
    ColorButtonNodes, ColorButtonOptions, ColorChannel, ColorChannelStep, ColorHsv,
    ColorHsva2dNodes, ColorHsva2dOptions, ColorOklch, ColorOklchChannel, ColorPalette,
    ColorPickerActionOptions, ColorPickerActionOutcome, ColorPickerChannel, ColorPickerEffect,
    ColorPickerMode, ColorPickerNodes, ColorPickerOptions, ColorPickerState, ColorPickerStyle,
    ColorPickerTarget, ColorPickerUpdate, ColorSwatch, ColorValueFormat,
};
pub use command_diagnostics::{
    command_diagnostics_panel, CommandDiagnosticsPanelNodes, CommandDiagnosticsPanelOptions,
};
pub use command_palette::{
    command_palette, CommandPaletteCommandSelection, CommandPaletteHistory, CommandPaletteItem,
    CommandPaletteMatch, CommandPaletteNodes, CommandPaletteOptions, CommandPaletteOutcome,
    CommandPaletteSelection, CommandPaletteState,
};
pub use context_menu::{context_menu, ContextMenuOpenOutcome, ContextMenuState, MenuOutcome};
pub use data::{PropertyRowStatus, PropertyValueKind};
pub use data_table::{
    virtualized_data_table, DataCellAlignment, DataTableAction, DataTableCellIndex,
    DataTableCellMeta, DataTableColumn, DataTableColumnRegion, DataTableExportFormat,
    DataTableExportOptions, DataTableExportScope, DataTableFilterState, DataTableOptions,
    DataTableRowDropPlacement, DataTableRowDropPolicy, DataTableRowIdentity, DataTableRowMeta,
    DataTableSelection, DataTableSortDirection, DataTableSortState, DataTableStickyColumns,
    DataTableStickySpec, DataViewEmptyReason, DataViewEmptyState, DataViewEntry,
    DataViewProjection, DataViewRow, DataViewRowIdentity, DataViewSectionHeader,
    VirtualDataTableSpec,
};
pub use date_picker::{
    date_picker, date_range_picker, CalendarDate, CalendarDateRange, CalendarDayCell,
    CalendarMonth, DatePickerBuilder, DatePickerControl, DatePickerKeyboardStep, DatePickerModel,
    DatePickerNodes, DatePickerOptions, DatePickerSelection, DatePickerStyle,
    DateRangeCellPosition, DateRangePickerBuilder, DateRangePickerModel, DateRangePickerOptions,
    DateRangePickerSelection, DateRangePickerStyle, DateRangeSelectionMode, Weekday,
};
pub use debug_inspector::{
    accessibility_debug_overlay, accessibility_overlay_panel, accessibility_timeline_panel,
    accessibility_tree_panel, action_dispatch_panel, action_map_panel, action_map_timeline_panel,
    animation_activity_panel, animation_activity_timeline_panel,
    animation_inspector_controls_panel, animation_state_graph_panel, bounds_autopsy_overlay,
    bounds_autopsy_panel, cache_reuse_panel, clip_chain_panel, clip_chain_timeline_panel,
    clip_scroll_panel, constraint_issues_panel, constraint_timeline_panel,
    debug_capture_report_panel, debug_contract_panel, debug_finding_inbox_panel,
    debug_health_score_panel, debug_inspector_panel, debug_invariant_panel, debug_issue_panel,
    debug_layout_tree_panel, debug_panel_recommendation_panel, debug_session_narrative_panel,
    diagnostics_coverage_panel, dirty_state_panel, drag_affordance_panel, event_route_panel,
    fix_verification_panel, focus_navigation_panel, focus_navigation_timeline_panel,
    frame_autopsy_panel, frame_bottleneck_panel, frame_budget_panel, frame_diff_panel,
    frame_recorder_panel, frame_regression_panel, frame_timeline_panel, frame_timing_panel,
    frame_timing_waterfall_panel, frame_trace_panel, hit_target_panel, hitbox_debug_overlay,
    hitbox_map_panel, hitbox_occlusion_panel, hitbox_occlusion_timeline_panel,
    hitbox_timeline_panel, inspect_node_panel, inspect_point_overlay, inspect_point_panel,
    interaction_affordance_panel, interaction_affordance_timeline_panel, interaction_state_panel,
    interaction_state_timeline_panel, invalidation_blame_panel, invalidation_blast_panel,
    invalidation_timeline_panel, invariant_timeline_panel, investigation_plan_panel,
    issue_timeline_panel, layout_autopsy_panel, layout_cause_panel, layout_cost_autopsy_panel,
    layout_cost_panel, layout_cost_timeline_panel, layout_jank_timeline_panel,
    layout_movement_panel, layout_pressure_panel, layout_pressure_timeline_panel,
    node_change_panel, node_explanation_panel, node_frame_history_panel, node_hotspots_panel,
    node_provenance_panel, node_recommendation_panel, node_search_panel, node_style_compare_panel,
    overlap_autopsy_panel, overlap_report_panel, overlap_timeline_panel,
    overlay_recommendation_panel, paint_batch_timeline_panel, paint_batches_panel,
    paint_hit_mismatch_panel, paint_hit_mismatch_timeline_panel, paint_order_panel,
    paint_overdraw_panel, paint_overdraw_timeline_panel, performance_timeline_panel,
    point_autopsy_overlay, point_autopsy_panel, pointer_autopsy_panel, pointer_probe_panel,
    pointer_session_panel, question_guide_panel, render_layer_timeline_panel, render_layers_panel,
    resource_diagnostics_panel, resource_timeline_panel, responsive_layout_panel,
    root_cause_cluster_panel, root_cause_timeline_panel, scroll_range_panel, scroll_timeline_panel,
    shortcut_route_panel, slow_frame_panel, slow_node_timeline_panel, slow_nodes_panel,
    stacking_order_panel, stacking_order_timeline_panel, text_contrast_panel, text_fit_panel,
    text_fit_timeline_panel, text_input_event_panel, text_input_state_panel, text_layout_panel,
    text_localization_panel, text_style_panel, visibility_panel, visibility_timeline_panel,
    visual_effect_timeline_panel, visual_effects_panel, wheel_route_panel, why_timeline_panel,
    why_trace_panel, widget_state_retention_panel, AccessibilityDebugOverlayNodes,
    AccessibilityDebugOverlayOptions, AccessibilityOverlayPanelOptions,
    AccessibilityTimelinePanelNodes, AccessibilityTimelinePanelOptions,
    AccessibilityTreePanelNodes, AccessibilityTreePanelOptions, ActionDispatchPanelNodes,
    ActionDispatchPanelOptions, ActionMapPanelNodes, ActionMapPanelOptions,
    ActionMapTimelinePanelNodes, ActionMapTimelinePanelOptions, AnimationActivityPanelNodes,
    AnimationActivityPanelOptions, AnimationActivityTimelinePanelNodes,
    AnimationActivityTimelinePanelOptions, AnimationInspectorControlsNodes,
    AnimationInspectorControlsOptions, AnimationStateGraphPanelNodes,
    AnimationStateGraphPanelOptions, BoundsAutopsyOverlayNodes, BoundsAutopsyOverlayOptions,
    BoundsAutopsyPanelNodes, BoundsAutopsyPanelOptions, CacheReusePanelNodes,
    CacheReusePanelOptions, ClipChainPanelNodes, ClipChainPanelOptions,
    ClipChainTimelinePanelNodes, ClipChainTimelinePanelOptions, ClipScrollPanelNodes,
    ClipScrollPanelOptions, ConstraintIssuesPanelNodes, ConstraintIssuesPanelOptions,
    ConstraintTimelinePanelNodes, ConstraintTimelinePanelOptions, DebugCaptureReportPanelNodes,
    DebugCaptureReportPanelOptions, DebugContractPanelNodes, DebugContractPanelOptions,
    DebugFindingInboxPanelNodes, DebugFindingInboxPanelOptions, DebugHealthScorePanelNodes,
    DebugHealthScorePanelOptions, DebugInspectorPanelNodes, DebugInspectorPanelOptions,
    DebugInvariantPanelNodes, DebugInvariantPanelOptions, DebugIssuePanelNodes,
    DebugIssuePanelOptions, DebugLayoutTreePanelNodes, DebugLayoutTreePanelOptions,
    DebugPanelRecommendationPanelNodes, DebugPanelRecommendationPanelOptions,
    DebugSessionNarrativePanelNodes, DebugSessionNarrativePanelOptions,
    DiagnosticsCoveragePanelNodes, DiagnosticsCoveragePanelOptions, DirtyStatePanelNodes,
    DirtyStatePanelOptions, DragAffordancePanelNodes, DragAffordancePanelOptions,
    EventRoutePanelNodes, EventRoutePanelOptions, FixVerificationPanelNodes,
    FixVerificationPanelOptions, FocusNavigationPanelNodes, FocusNavigationPanelOptions,
    FocusNavigationTimelinePanelNodes, FocusNavigationTimelinePanelOptions, FrameAutopsyPanelNodes,
    FrameAutopsyPanelOptions, FrameBottleneckPanelNodes, FrameBottleneckPanelOptions,
    FrameBudgetPanelNodes, FrameBudgetPanelOptions, FrameDiffPanelNodes, FrameDiffPanelOptions,
    FrameRecorderPanelNodes, FrameRecorderPanelOptions, FrameRegressionPanelNodes,
    FrameRegressionPanelOptions, FrameTimelinePanelNodes, FrameTimelinePanelOptions,
    FrameTimingPanelNodes, FrameTimingPanelOptions, FrameTimingWaterfallPanelNodes,
    FrameTimingWaterfallPanelOptions, FrameTracePanelNodes, FrameTracePanelOptions,
    HitTargetPanelNodes, HitTargetPanelOptions, HitboxDebugOverlayNodes, HitboxDebugOverlayOptions,
    HitboxMapPanelNodes, HitboxMapPanelOptions, HitboxOcclusionPanelNodes,
    HitboxOcclusionPanelOptions, HitboxOcclusionTimelinePanelNodes,
    HitboxOcclusionTimelinePanelOptions, HitboxTimelinePanelNodes, HitboxTimelinePanelOptions,
    InspectNodePanelNodes, InspectNodePanelOptions, InspectPointOverlayNodes,
    InspectPointOverlayOptions, InspectPointPanelNodes, InspectPointPanelOptions,
    InteractionAffordancePanelNodes, InteractionAffordancePanelOptions,
    InteractionAffordanceTimelinePanelNodes, InteractionAffordanceTimelinePanelOptions,
    InteractionStatePanelNodes, InteractionStatePanelOptions, InteractionStateTimelinePanelNodes,
    InteractionStateTimelinePanelOptions, InvalidationBlamePanelNodes,
    InvalidationBlamePanelOptions, InvalidationBlastPanelNodes, InvalidationBlastPanelOptions,
    InvalidationTimelinePanelNodes, InvalidationTimelinePanelOptions, InvariantTimelinePanelNodes,
    InvariantTimelinePanelOptions, InvestigationPlanPanelNodes, InvestigationPlanPanelOptions,
    IssueTimelinePanelNodes, IssueTimelinePanelOptions, LayoutAutopsyPanelNodes,
    LayoutAutopsyPanelOptions, LayoutCausePanelNodes, LayoutCausePanelOptions,
    LayoutCostAutopsyPanelNodes, LayoutCostAutopsyPanelOptions, LayoutCostPanelNodes,
    LayoutCostPanelOptions, LayoutCostTimelinePanelNodes, LayoutCostTimelinePanelOptions,
    LayoutJankTimelinePanelNodes, LayoutJankTimelinePanelOptions, LayoutMovementPanelNodes,
    LayoutMovementPanelOptions, LayoutPressurePanelNodes, LayoutPressurePanelOptions,
    LayoutPressureTimelinePanelNodes, LayoutPressureTimelinePanelOptions, NodeChangePanelNodes,
    NodeChangePanelOptions, NodeExplanationPanelNodes, NodeExplanationPanelOptions,
    NodeFrameHistoryPanelNodes, NodeFrameHistoryPanelOptions, NodeHotspotsPanelNodes,
    NodeHotspotsPanelOptions, NodeProvenancePanelNodes, NodeProvenancePanelOptions,
    NodeRecommendationPanelNodes, NodeRecommendationPanelOptions, NodeSearchPanelNodes,
    NodeSearchPanelOptions, NodeStyleComparePanelNodes, NodeStyleComparePanelOptions,
    OverlapAutopsyPanelNodes, OverlapAutopsyPanelOptions, OverlapReportPanelNodes,
    OverlapReportPanelOptions, OverlapTimelinePanelNodes, OverlapTimelinePanelOptions,
    OverlayRecommendationPanelNodes, OverlayRecommendationPanelOptions,
    PaintBatchTimelinePanelNodes, PaintBatchTimelinePanelOptions, PaintBatchesPanelNodes,
    PaintBatchesPanelOptions, PaintHitMismatchPanelNodes, PaintHitMismatchPanelOptions,
    PaintHitMismatchTimelinePanelNodes, PaintHitMismatchTimelinePanelOptions, PaintOrderPanelNodes,
    PaintOrderPanelOptions, PaintOverdrawPanelNodes, PaintOverdrawPanelOptions,
    PaintOverdrawTimelinePanelNodes, PaintOverdrawTimelinePanelOptions,
    PerformanceTimelinePanelNodes, PerformanceTimelinePanelOptions, PointAutopsyOverlayNodes,
    PointAutopsyOverlayOptions, PointAutopsyPanelNodes, PointAutopsyPanelOptions,
    PointerAutopsyPanelNodes, PointerAutopsyPanelOptions, PointerProbePanelNodes,
    PointerProbePanelOptions, PointerSessionPanelNodes, PointerSessionPanelOptions,
    QuestionGuidePanelNodes, QuestionGuidePanelOptions, RenderLayerTimelinePanelNodes,
    RenderLayerTimelinePanelOptions, RenderLayersPanelNodes, RenderLayersPanelOptions,
    ResourceDiagnosticsPanelNodes, ResourceDiagnosticsPanelOptions, ResourceTimelinePanelNodes,
    ResourceTimelinePanelOptions, ResponsiveLayoutPanelNodes, ResponsiveLayoutPanelOptions,
    RootCauseClusterPanelNodes, RootCauseClusterPanelOptions, RootCauseTimelinePanelNodes,
    RootCauseTimelinePanelOptions, ScrollRangePanelNodes, ScrollRangePanelOptions,
    ScrollTimelinePanelNodes, ScrollTimelinePanelOptions, ShortcutRoutePanelNodes,
    ShortcutRoutePanelOptions, SlowFramePanelNodes, SlowFramePanelOptions,
    SlowNodeTimelinePanelNodes, SlowNodeTimelinePanelOptions, SlowNodesPanelNodes,
    SlowNodesPanelOptions, StackingOrderPanelNodes, StackingOrderPanelOptions,
    StackingOrderTimelinePanelNodes, StackingOrderTimelinePanelOptions, TextContrastPanelNodes,
    TextContrastPanelOptions, TextFitPanelNodes, TextFitPanelOptions, TextFitTimelinePanelNodes,
    TextFitTimelinePanelOptions, TextInputEventPanelNodes, TextInputEventPanelOptions,
    TextInputStatePanelNodes, TextInputStatePanelOptions, TextLayoutPanelNodes,
    TextLayoutPanelOptions, TextLocalizationPanelNodes, TextLocalizationPanelOptions,
    TextStylePanelNodes, TextStylePanelOptions, VisibilityPanelNodes, VisibilityPanelOptions,
    VisibilityTimelinePanelNodes, VisibilityTimelinePanelOptions, VisualEffectTimelinePanelNodes,
    VisualEffectTimelinePanelOptions, VisualEffectsPanelNodes, VisualEffectsPanelOptions,
    WheelRoutePanelNodes, WheelRoutePanelOptions, WhyTimelinePanelNodes, WhyTimelinePanelOptions,
    WhyTracePanelNodes, WhyTracePanelOptions, WidgetStateRetentionPanelNodes,
    WidgetStateRetentionPanelOptions,
};
pub use dialog::{DialogDescriptor, DialogDismissReason, DialogDismissal, DialogStack};
pub use dock_workspace::{
    dock_drawer_rail, dock_workspace, DockDrawerDescriptor, DockDrawerItemNode,
    DockDrawerRailNodes, DockDrawerRailOptions, DockDropPlacement, DockFloatingPanel,
    DockPanelDescriptor, DockPanelLayoutSnapshot, DockPanelNode, DockPanelPlacement,
    DockPanelReorderPlacement, DockPanelReorderTarget, DockPanelReorderTargetId, DockSide,
    DockWorkspaceDragOptions, DockWorkspaceDropZone, DockWorkspaceLayoutApplyReport,
    DockWorkspaceLayoutSnapshot, DockWorkspaceNodes, DockWorkspaceOptions,
    DockWorkspaceReorderChange, DockWorkspaceReorderOptions, DockWorkspaceState,
    DockWorkspaceStateChange, DockWorkspaceVisibilityChange,
};
pub use dropdown::{
    dropdown_select, searchable_select_contract, select_menu, select_menu_popup,
    DropdownSelectNodes, DropdownSelectOptions, SearchableSelectCloseReason,
    SearchableSelectContract, SearchableSelectOutcome, SearchableSelectRow, SearchableSelectSpec,
    SearchableSelectState, SelectMenuNodes, SelectMenuOptions, SelectMenuOutcome, SelectMenuState,
    SelectOption, SelectOptionFilterEmptyState, SelectOptionFilterMatch, SelectOptionFilterOutcome,
    SelectOptionFilterState, SelectSelection,
};
pub use editable_form::{
    editable_form_contract, EditableFormCommand, EditableFormCommandKind, EditableFormCommitMode,
    EditableFormContract, EditableFormField, EditableFormFieldContract, EditableFormFieldKind,
    EditableFormOutcome, EditableFormState,
};
pub use floating_window::{
    floating_desktop, floating_window_layout, FloatingDesktopNodes, FloatingDesktopOptions,
    FloatingDesktopState, FloatingDesktopZPolicy, FloatingWindowDefaults, FloatingWindowDescriptor,
    FloatingWindowDragState, FloatingWindowNode, FloatingWindowOrganizeMode,
    FloatingWindowOrganizeOutcome, FloatingWindowOrganizeSpec, FloatingWindowPlacement,
    FloatingWindowResizeState,
};
pub use menu::{
    image_menu_button, image_text_menu_button, menu_button, popup_panel, submenu, submenu_button,
    AnchoredPopup, MenuButtonAnchors, MenuButtonNodes, MenuButtonOptions, MenuButtonOutcome,
    MenuButtonState, MenuCommandSelection, MenuItem, MenuItemKind, MenuNavigationOutcome,
    MenuNavigationState, MenuSelection, MenuSubmenuAnchor, NavigationDirection, PopupAlign,
    PopupLayout, PopupOptions, PopupPlacement, PopupSide, SearchClearButtonMeta,
    SearchFieldOutcome, SearchFieldState, SearchFilterRequest, SearchFilterTiming,
    SearchStatusText,
};
pub use menu_bar::{
    menu_bar, MenuBarAnchors, MenuBarMenu, MenuBarNodes, MenuBarOptions, MenuBarState,
};
pub use menu_list::{menu_list, menu_list_popup, MenuListNodes, MenuListOptions};
pub use numeric_input::{
    drag_value, NumericDragSpec, NumericDragSpeed, NumericInputOutcome, NumericInputState,
    NumericInputStyle, NumericKeyboardStep, NumericParameterSpec, NumericPrecision, NumericRange,
    NumericScale, NumericSliderDrag, NumericSliderOutcome, NumericSliderState,
    NumericTextValidation, NumericUnitFormat, NumericValidationStatus, SliderAxis, SliderGeometry,
};
pub use path_picker::{
    path_breadcrumbs, PathBreadcrumb, PathPickerControl, PathPickerMode, PathPickerState,
    PathPickerStyle, PathPickerUpdate, PathTextValidation, PathTextValidationStatus,
};
pub use pickers::{PickerAnimationMeta, PickerElementStyle};
pub use popover::{
    OverlayFrameEvent, OverlayFrameOutput, OverlayFrameRequest, OverlayFrameState, PopoverAnchor,
    PopoverDescriptor, PopoverDismissReason, PopoverPlacement, PopoverState,
};
pub use progress_indicator::{
    progress_indicator, progress_log_panel, ProgressIndicatorKind, ProgressIndicatorNodes,
    ProgressIndicatorOptions, ProgressIndicatorValue, ProgressLogEntry, ProgressLogLevel,
    ProgressLogPanelAction, ProgressLogPanelNodes, ProgressLogPanelOptions,
};
pub use property_inspector::{property_inspector_grid, PropertyGridRow, PropertyInspectorOptions};
pub use split_pane::{
    split_pane, SplitAxis, SplitPaneNodes, SplitPaneOptions, SplitPaneSizes, SplitPaneState,
};
pub use surfaces::{
    surface_open_close_animation, toast_enter_exit_animation, SURFACE_CLOSE_TRIGGER,
    SURFACE_OPEN_TRIGGER, TOAST_ENTER_TRIGGER, TOAST_EXIT_TRIGGER,
};
pub use tab_group::{tab_group, TabGroupOptions, TabGroupState, TabItem};
pub use theme_editor::{
    theme_editor_panel, ThemeAccessibilityAudit, ThemeAccessibilityAuditOptions,
    ThemeAccessibilityIssue, ThemeAccessibilityIssueKind, ThemeEditorPanelNodes,
    ThemeEditorPanelOptions, ThemePatchExport, ThemePatchGroup, ThemePatchSnippetOptions,
    ThemePatchTokenChange,
};
pub use timeline_ruler::{
    timeline_ruler, RulerSpec, RulerTick, RulerTickKind, TimelineRange, TimelineRulerOptions,
};
pub use toast::{
    toast_stack, Toast, ToastAction, ToastId, ToastSeverity, ToastStack, ToastStackOptions,
};
pub use toggle_control::{
    segmented_control, SegmentedControlItem, SegmentedControlNodes, SegmentedControlOptions,
    ToggleControlOutcome, ToggleControlRole, ToggleControlState, ToggleValue,
};
pub use tree_view::{
    outliner, tree_view, virtualized_tree_view, TreeDropPlacement, TreeItem, TreeItemDropPolicy,
    TreeRowAction, TreeViewOptions, TreeViewState, TreeVisibleItem, VirtualTreeViewNodes,
    VirtualTreeViewSpec,
};
