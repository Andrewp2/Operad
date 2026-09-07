use web_time::{Duration, Instant};

use operad::debug::{
    DebugAccessibilityTimelineTrace, DebugAccessibilityTreeTrace, DebugActionDispatchTrace,
    DebugActionMapTimelineTrace, DebugActionMapTrace, DebugAnimationActivityTimelineTrace,
    DebugAnimationActivityTrace, DebugAnimationGraphEdgeKind, DebugBoundsAutopsyTrace,
    DebugCacheReuseTrace, DebugCaptureReport, DebugClipChainTimelineTrace, DebugClipChainTrace,
    DebugClipScrollTrace, DebugConstraintReport, DebugConstraintTimelineTrace, DebugContractReport,
    DebugDiagnosticsCoverageTrace, DebugDragAffordanceTrace, DebugEventRouteTrace,
    DebugFindingInboxTrace, DebugFixVerificationTrace, DebugFocusNavigationTimelineTrace,
    DebugFocusNavigationTrace, DebugFrameAutopsyTrace, DebugFrameBottleneckTrace,
    DebugFrameBudgetTrace, DebugFrameDiff, DebugFrameRecorder, DebugFrameRegressionTrace,
    DebugFrameTimelineTrace, DebugFrameTimingWaterfallTrace, DebugFrameTrace,
    DebugFrameTraceContext, DebugHealthScoreTrace, DebugHitTargetTrace, DebugHitboxMapTrace,
    DebugHitboxOcclusionTimelineTrace, DebugHitboxOcclusionTrace, DebugHitboxTimelineTrace,
    DebugInspectNodeTrace, DebugInspectPointTrace, DebugInspectorSnapshot,
    DebugInteractionAffordanceTimelineTrace, DebugInteractionAffordanceTrace,
    DebugInteractionStateTimelineTrace, DebugInteractionStateTrace, DebugInvalidationBlameTrace,
    DebugInvalidationBlastTrace, DebugInvalidationTimelineTrace, DebugInvariantTimelineTrace,
    DebugInvariantTrace, DebugInvestigationPlanTrace, DebugIssueReport, DebugIssueTimelineTrace,
    DebugLayoutAutopsyTrace, DebugLayoutCauseTrace, DebugLayoutCostAutopsyTrace,
    DebugLayoutCostTimelineTrace, DebugLayoutCostTrace, DebugLayoutJankTimelineTrace,
    DebugLayoutMovementTrace, DebugLayoutPressureTimelineTrace, DebugLayoutPressureTrace,
    DebugLayoutTree, DebugNodeChangeTrace, DebugNodeExplanation, DebugNodeFrameHistoryTrace,
    DebugNodeHotspotTrace, DebugNodeProvenanceTrace, DebugNodeRecommendationTrace,
    DebugNodeSearchTrace, DebugNodeStyleCompareTrace, DebugOverlapAutopsyTrace, DebugOverlapReport,
    DebugOverlapTimelineTrace, DebugOverlayContext, DebugOverlayOptions,
    DebugOverlayRecommendationTrace, DebugPaintBatchTimelineTrace, DebugPaintBatchTrace,
    DebugPaintHitMismatchTimelineTrace, DebugPaintHitMismatchTrace, DebugPaintOrderTrace,
    DebugPaintOverdrawTimelineTrace, DebugPaintOverdrawTrace, DebugPanelRecommendationTrace,
    DebugPerformanceTimelineTrace, DebugPointAutopsyTrace, DebugPointerAutopsyTrace,
    DebugPointerProbe, DebugPointerRouteKind, DebugPointerSessionTrace, DebugQuestionGuideTrace,
    DebugRenderLayerTimelineTrace, DebugRenderLayerTrace, DebugResourceReport,
    DebugResourceTimelineTrace, DebugResponsiveLayoutTrace, DebugRootCauseClusterTrace,
    DebugRootCauseTimelineTrace, DebugScrollRangeTrace, DebugScrollTimelineTrace,
    DebugSessionNarrativeTrace, DebugShortcutRouteTrace, DebugSlowFrameTrace,
    DebugSlowNodeTimelineTrace, DebugSlowNodeTrace, DebugStackingOrderTimelineTrace,
    DebugStackingOrderTrace, DebugTextContrastTrace, DebugTextFitTimelineTrace, DebugTextFitTrace,
    DebugTextInputEventTrace, DebugTextInputStateTrace, DebugTextLayoutTrace,
    DebugTextLocalizationTrace, DebugTextStyleTrace, DebugThemeSnapshot,
    DebugVisibilityTimelineTrace, DebugVisibilityTrace, DebugVisualEffectTimelineTrace,
    DebugVisualEffectTrace, DebugWheelRouteTrace, DebugWhyTimelineTrace, DebugWhyTrace,
    DebugWidgetStateRetentionTrace,
};
use operad::diagnostics::{CacheDiagnostic, DirtyStateExplanation, PerformanceSnapshot};
use operad::display::{
    DisplayListInvalidation, DisplayListKey, DisplayListKind, DisplayListReuseOutcome,
    DisplayListReuseReport, DisplayListScope,
};
use operad::forms::FormValidationResult;
use operad::host::{HostInteractionState, HostShortcutRoute};
#[cfg(all(not(target_arch = "wasm32"), feature = "native-window"))]
use operad::native::{
    NativeWgpuCanvasRenderRegistry, NativeWindowHooks, NativeWindowOptions, NativeWindowResult,
};
use operad::platform::{
    ClipboardResponse, CursorRequest, CursorShape, DragBytes, DragOperation, DragPayload,
    ImageHandle, PixelSize, PlatformRequest, PlatformResponse, PlatformServiceResponse,
    ResourceHandle,
};
use operad::renderer::ResourceUpdate;
use operad::runtime::{PlatformServiceClient, RuntimeInvalidation, RuntimeInvalidationReason};
use operad::tooltips::{ShortcutFormatter, TooltipContent, TooltipPlacement};
use operad::widgets::ext::{self as ext_widgets, CalendarDate};
use operad::widgets::{scroll_area as scroll_area_widgets, scrollbar as scrollbar_widgets};
use operad::widgets::{TextInputOptions, TextInputState};
#[cfg(feature = "text-cosmic")]
use operad::CosmicTextMeasurer;
use operad::{
    root_style, widgets, AccessibilityAction, AccessibilityMeta, AccessibilityRole, AlignedStroke,
    AnimatedValues, AnimationBlendBinding, AnimationCondition, AnimationMachine, AnimationState,
    AnimationTransition, ApproxTextMeasurer, BidiPolicy, BuiltInIcon, CanvasContent,
    CanvasRenderProgram, ClipBehavior, ColorRgba, ColorTokens, CommandId, CommandMeta,
    CommandRegistry, CommandScope, ComponentRole, ComponentState, CornerRadii, DirtyFlags,
    DragDropSurfaceKind, DropPayloadFilter, DynamicLabelMeta, EditPhase, ElementMaterial,
    ElementShape, FocusRestoreTarget, FontFamily, FontWeight, FormState, FrameTiming,
    GeometryEffect, GestureEvent, ImageContent, InputBehavior, Layout, LayoutAlignment,
    LayoutDimension, LayoutFlexWrap, LayoutGap, LayoutGridTrack, LayoutInsets, LayoutLength,
    LayoutSize, LayoutSpacing, LayoutStyle, LocaleId, LocalizationPolicy, PaintEffect, PaintItem,
    PaintKind, PaintList, PaintRect, PaintText, PaintTransform, PointerButton, PointerId,
    ScenePrimitive, ScrollAxes, ShaderEffect, Shortcut, StrokeStyle, TextHorizontalAlign,
    TextStyle, TextVerticalAlign, TextWrap, Theme, UiDocument, UiFocusState, UiInputEvent, UiNode,
    UiNodeId, UiNodeStyle, UiPoint, UiPortalTarget, UiRect, UiSize, UiVisual, UiWheelEvent,
    ValidationMessage, WidgetAction, WidgetActionBinding, WidgetActionKind, WidgetActionMode,
    WidgetActionQueue, WidgetDrag, WidgetDragPhase, WidgetPointerEdit, WidgetTextEdit,
    ANIMATION_INPUT_POINTER_NORM_X,
};
const RIGHT_PANEL_WIDTH: f32 = 300.0;
const SHOWCASE_WINDOW_Z_BASE: f32 = 64.0;
const SHOWCASE_WINDOW_Z_STRIDE: f32 = 32.0;
const SHOWCASE_WINDOW_Z_MAX: f32 = 960.0;
const SHOWCASE_TICK_RATE_HZ: f32 = 120.0;
const SHOWCASE_FPS_SAMPLE_INTERVAL: Duration = Duration::from_millis(500);
const SHOWCASE_ORGANIZE_BUTTON_RESERVED_HEIGHT: f32 = 44.0;
const SHOWCASE_ORGANIZE_MEASURE_HEIGHT: f32 = 64_000.0;
const SHOWCASE_PROGRESS_RADIANS_PER_SECOND: f32 = 1.08;
const TEXT_CARET_BLINK_HZ: f32 = 1.1;
const CONTROLS_WIDGET_ROW_HEIGHT: f32 = 17.0;
const CONTROLS_WIDGET_ROW_GAP: f32 = 12.0;
const SHOWCASE_DOCUMENT_NODE_CAPACITY: usize = 2_048;
const PROGRESS_LOGGED_DURATION_SECONDS: f32 = 9.25;
const PROGRESS_LOG_VIEWPORT_HEIGHT: f32 = 96.0;
const PROGRESS_LOG_ROW_HEIGHT: f32 = 26.0;
const ANIMATION_INPUT_OPEN: &str = "open";
const ANIMATION_INPUT_PROGRESS: &str = "progress";
const ANIMATION_INPUT_SCRUB: &str = "scrub";
const ANIMATION_STAGE_MIN_WIDTH: f32 = 360.0;
const ANIMATION_STAGE_HEIGHT: f32 = 170.0;
const ANIMATION_CONTENT_MIN_HEIGHT: f32 = ANIMATION_STAGE_HEIGHT * 4.0 + 260.0;
const EASING_STAGE_MIN_WIDTH: f32 = 360.0;
const EASING_STAGE_HEIGHT: f32 = 170.0;
const EASING_CONTENT_MIN_HEIGHT: f32 = EASING_STAGE_HEIGHT * 2.0 + 150.0;
const TIMELINE_CONTENT_WIDTH: f32 = 2400.0;
const TIMELINE_VIEWPORT_HEIGHT: f32 = 172.0;
const TIMELINE_SCROLLBAR_HEIGHT: f32 = 8.0;
const TIMELINE_SCROLL_CONTAINER_HEIGHT: f32 = 188.0;
const PANELS_SPLIT_HANDLE_THICKNESS: f32 = 2.0;
const MEDIA_ICON_COLUMNS: usize = 5;
const MEDIA_ICON_TILE_WIDTH: f32 = 70.0;
const MEDIA_ICON_TILE_HEIGHT: f32 = 78.0;
const MEDIA_ICON_GRID_GAP: f32 = 8.0;
const SHADER_LAB_PREVIEW_WIDTH: f32 = 380.0;
const SHADER_LAB_PREVIEW_HEIGHT: f32 = 320.0;
const SHADER_LAB_PREVIEW_MIN_WIDTH: f32 = 320.0;
const SHADER_LAB_FRAME_MIN_WIDTH: f32 = 280.0;
const SHADER_LAB_FRAME_MIN_HEIGHT: f32 = 160.0;
const SHADER_LAB_BUTTON_WIDTH: f32 = 280.0;
const SHADER_LAB_BUTTON_HEIGHT: f32 = 68.0;
const SHADER_LAB_EDITOR_WIDTH: f32 = 440.0;
const SHADER_LAB_EDITOR_HEIGHT: f32 = 360.0;
const SHADER_LAB_EDITOR_MIN_WIDTH: f32 = 360.0;
const SHADER_LAB_SPLIT_HANDLE_THICKNESS: f32 = 6.0;
const SHADER_LAB_WORKSPACE_HEIGHT: f32 = 620.0;
const SHADER_LAB_SURFACE_STROKE_MAX: f32 = 4.0;
const SHADER_LAB_SURFACE_RADIUS_MAX: f32 = 24.0;
const SHADER_LAB_MATERIAL_OUTSET: f32 = 16.0;
const SHADER_LAB_MATERIAL_OUTSET_MAX: f32 = 32.0;
const SHADER_LAB_CONTENT_MIN_WIDTH: f32 =
    SHADER_LAB_PREVIEW_MIN_WIDTH + SHADER_LAB_EDITOR_MIN_WIDTH + SHADER_LAB_SPLIT_HANDLE_THICKNESS;
const SHADER_LAB_CONTENT_MIN_HEIGHT: f32 = SHADER_LAB_WORKSPACE_HEIGHT;
const STYLING_CONTROLS_WIDTH: f32 = 300.0;
const STYLING_WIDE_FIELDS_WIDTH: f32 = 192.0;
const STYLING_VALUE_INPUT_WIDTH: f32 = 46.0;
const ANIMATION_ORB_SIZE: f32 = 96.0;
const ANIMATION_SHAPE_SIZE: f32 = 96.0;
const ANIMATION_PANEL_INSET_X: f32 = 24.0;
const ANIMATION_PANEL_Y: f32 = 62.0;
const ANIMATION_PANEL_WIDTH: f32 = 136.0;
const ANIMATION_PANEL_HEIGHT: f32 = 46.0;
const STYLING_STROKE_MAX: f32 = 30.0;
const SHOWCASE_USER_IMAGE_KEY: &str = "showcase.operad-logo.png";
const SHOWCASE_USER_IMAGE_PNG: &[u8] = include_bytes!("../operad_icon.png");

const SHOWCASE_WIDGET_WINDOW_IDS: [&str; 32] = [
    "labels",
    "buttons",
    "checkbox",
    "toggles",
    "slider",
    "numeric",
    "text_input",
    "selection",
    "menus",
    "command_palette",
    "date_picker",
    "color_picker",
    "progress",
    "animation",
    "easing",
    "lists_tables",
    "property_inspector",
    "diagnostics",
    "trees",
    "layout_widgets",
    "containers",
    "panels",
    "forms",
    "overlays",
    "drag_drop",
    "media",
    "shaders",
    "shader_lab",
    "timeline",
    "canvas",
    "theme",
    "styling",
];

fn showcase_user_image_update() -> Option<ResourceUpdate> {
    ResourceUpdate::from_encoded_image(
        ImageHandle::app(SHOWCASE_USER_IMAGE_KEY),
        SHOWCASE_USER_IMAGE_PNG,
    )
    .ok()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "native-window"))]
fn main() -> NativeWindowResult {
    let canvas_renderers = NativeWgpuCanvasRenderRegistry::new();
    let hooks = NativeWindowHooks::new()
        .with_before_render(|state: &mut ShowcaseState, metrics| {
            state.prepare_frame(metrics.viewport);
        })
        .with_platform_service_requests(|state: &mut ShowcaseState, _metrics| {
            state.platform.drain_requests()
        })
        .with_platform_responses(|state: &mut ShowcaseState, responses| {
            state.apply_platform_responses(responses);
        });
    operad::native::run_app_with_canvas_renderers_and_hooks(
        NativeWindowOptions::new("showcase")
            .with_size(900.0, 760.0)
            .with_min_size(720.0, 560.0)
            .with_tick_action("runtime.tick")
            .with_tick_rate_hz(SHOWCASE_TICK_RATE_HZ),
        ShowcaseState::default(),
        ShowcaseState::update,
        ShowcaseState::view,
        canvas_renderers,
        hooks,
    )
}

#[cfg(target_arch = "wasm32")]
pub async fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    let hooks = operad::web::WebRuntimeHooks::new()
        .with_before_render(|state: &mut ShowcaseState, metrics| {
            state.prepare_frame(metrics.viewport);
        })
        .with_platform_service_requests(|state: &mut ShowcaseState, _metrics| {
            state.platform.drain_requests()
        })
        .with_platform_responses(|state: &mut ShowcaseState, responses| {
            state.apply_platform_responses(responses);
        });

    operad::web::run_app_with_hooks(
        operad::web::WebRuntimeOptions::new("Operad showcase")
            .with_canvas_id("operad-showcase-canvas")
            .with_status_id("operad-showcase-status")
            .with_target_name("showcase")
            .with_tick_action("runtime.tick")
            .with_tick_rate_hz(SHOWCASE_TICK_RATE_HZ),
        ShowcaseState::default(),
        ShowcaseState::update,
        ShowcaseState::view,
        hooks,
    )
    .await
}

struct ShowcaseState {
    checked: bool,
    checkbox_indeterminate: widgets::CheckboxState,
    slider: f32,
    slider_left: f32,
    slider_right: f32,
    slider_value_text: TextInputState,
    slider_left_text: TextInputState,
    slider_right_text: TextInputState,
    slider_step_value: f32,
    slider_step_text: TextInputState,
    slider_trailing_color: bool,
    slider_trailing_picker: ext_widgets::ColorPickerState,
    slider_trailing_picker_open: bool,
    slider_thumb_picker: ext_widgets::ColorPickerState,
    slider_thumb_picker_open: bool,
    slider_thumb_shape: SliderThumbChoice,
    slider_use_steps: bool,
    slider_logarithmic: bool,
    slider_clamping: widgets::SliderClamping,
    slider_smart_aim: bool,
    label_locale: ext_widgets::SelectMenuState,
    label_link_visited: bool,
    label_hyperlink_visited: bool,
    label_link_status: &'static str,
    color: ext_widgets::ColorPickerState,
    date: ext_widgets::DatePickerModel,
    date_range: ext_widgets::DateRangePickerModel,
    date_mode: DateDemoMode,
    radio_choice: &'static str,
    switch_enabled: bool,
    mixed_switch: ext_widgets::ToggleValue,
    theme_preference: widgets::ThemePreference,
    showcase_theme: ShowcaseThemeChoice,
    numeric_value: f32,
    numeric_text: TextInputState,
    numeric_range_min: f32,
    numeric_range_max: f32,
    numeric_range_min_text: TextInputState,
    numeric_range_max_text: TextInputState,
    numeric_sensitivity: f32,
    numeric_unit: ext_widgets::SelectMenuState,
    numeric_drag_start: Option<(f32, f32)>,
    dropdown: ext_widgets::SelectMenuState,
    select_menu: ext_widgets::SelectMenuState,
    image_select_menu: ext_widgets::SelectMenuState,
    text: TextInputState,
    selectable_text: TextInputState,
    singleline_text: TextInputState,
    multiline_text: TextInputState,
    text_area_text: TextInputState,
    code_editor_text: TextInputState,
    search_text: TextInputState,
    password_text: TextInputState,
    focused_text: Option<FocusedTextInput>,
    platform: PlatformServiceClient,
    clipboard_text: String,
    pending_clipboard_paste: Option<FocusedTextInput>,
    last_button: &'static str,
    toggle_button: bool,
    table_selection: ext_widgets::DataTableSelection,
    tree: ext_widgets::TreeViewState,
    editable_tree: Vec<EditableTreeNode>,
    editable_tree_next_id: u32,
    editable_tree_status: String,
    outliner: ext_widgets::TreeViewState,
    tree_virtual: ext_widgets::TreeViewState,
    tree_virtual_scroll: f32,
    tree_table: ext_widgets::TreeViewState,
    tree_table_scroll: f32,
    toast_visible: bool,
    toast_action_status: &'static str,
    progress_phase: f32,
    progress_loading_elapsed: f32,
    progress_logs_scroll: operad::ScrollState,
    progress_logs_follow_tail: bool,
    animation_scrub: f32,
    animation_open: bool,
    animation_timed_expanded: bool,
    animation_scrub_expanded: bool,
    animation_state_expanded: bool,
    animation_interaction_expanded: bool,
    easing_in: ext_widgets::SelectMenuState,
    easing_out: ext_widgets::SelectMenuState,
    caret_phase: f32,
    command_palette: ext_widgets::CommandPaletteState,
    command_palette_open: bool,
    command_history: ext_widgets::CommandPaletteHistory,
    last_command: String,
    list_scroll: f32,
    virtual_scroll: f32,
    table_scroll: f32,
    virtual_table_scroll: f32,
    virtual_table_descending: bool,
    virtual_table_ready_only: bool,
    virtual_table_value_width: f32,
    virtual_table_resize: Option<(f32, f32)>,
    layout_panel_a_scroll: f32,
    layout_panel_b_scroll: f32,
    layout_workspace_scroll: f32,
    scrollbars: scrollbar_widgets::ScrollbarControllerState,
    layout_tab: usize,
    styling: StylingState,
    styling_stroke_picker: ext_widgets::ColorPickerState,
    styling_stroke_picker_open: bool,
    styling_fill_picker: ext_widgets::ColorPickerState,
    styling_fill_picker_open: bool,
    styling_shadow_picker: ext_widgets::ColorPickerState,
    styling_shadow_picker_open: bool,
    cube: CanvasCubeState,
    canvas_grow_horizontal: bool,
    canvas_grow_vertical: bool,
    canvas_keep_aspect_ratio: bool,
    menu_bar: ext_widgets::MenuBarState,
    menu_button: ext_widgets::MenuButtonState,
    image_text_menu_button: ext_widgets::MenuButtonState,
    image_menu_button: ext_widgets::MenuButtonState,
    context_menu: ext_widgets::ContextMenuState,
    menu_autosave: bool,
    menu_grid: bool,
    form: FormState,
    form_name_text: TextInputState,
    form_email_text: TextInputState,
    form_role_text: TextInputState,
    form_newsletter: bool,
    form_status: String,
    overlay_expanded: bool,
    overlay_popup_open: bool,
    overlay_modal_open: bool,
    color_picker_button_open: bool,
    drag_drop_active_payload: Option<DragDropDemoPayload>,
    drag_drop_status: &'static str,
    drag_drop_cursor_shape: CursorShape,
    shader_lab_split: ext_widgets::SplitPaneState,
    shader_lab_editor_scroll: UiPoint,
    shader_lab_show_frame_text: bool,
    shader_lab_show_button_text: bool,
    shader_lab_surface_stroke_width: f32,
    shader_lab_surface_radius: f32,
    shader_lab_target: ShaderLabTarget,
    shader_lab_target_menu: ext_widgets::SelectMenuState,
    shader_lab_preset: ShaderLabPreset,
    shader_lab_preset_menu: ext_widgets::SelectMenuState,
    shader_lab_material_shader: ShaderLabMaterialShader,
    shader_lab_material_shader_menu: ext_widgets::SelectMenuState,
    shader_lab_material_shape: ShaderLabMaterialShape,
    shader_lab_material_shape_menu: ext_widgets::SelectMenuState,
    shader_lab_material_geometry: ShaderLabMaterialGeometry,
    shader_lab_material_geometry_menu: ext_widgets::SelectMenuState,
    shader_lab_material_outset: f32,
    shader_lab_source: TextInputState,
    shader_lab_source_error: Option<String>,
    timeline_scroll: operad::ScrollState,
    panels_top_split: ext_widgets::SplitPaneState,
    panels_bottom_split: ext_widgets::SplitPaneState,
    panels_left_split: ext_widgets::SplitPaneState,
    panels_right_split: ext_widgets::SplitPaneState,
    layout_split: ext_widgets::SplitPaneState,
    layout_dock: ext_widgets::DockWorkspaceState,
    diagnostics_animation_paused: bool,
    diagnostics_animation_scrub: f32,
    diagnostics_animation_active: bool,
    diagnostics_animation_hover: f32,
    diagnostics_animation_pulse_count: u32,
    diagnostics_page: DiagnosticsPage,
    diagnostics_snapshot: DebugInspectorSnapshot,
    containers_scroll: operad::ScrollState,
    controls_scroll: operad::ScrollState,
    color_copied_hex: Option<String>,
    fps_last_sample: Instant,
    fps_frames: u32,
    fps: f32,
    last_desktop_size: UiSize,
    initial_organize_pending: bool,
    windows: ShowcaseWindows,
    desktop: ext_widgets::FloatingDesktopState,
    user_image_update: Option<ResourceUpdate>,
}

#[derive(Debug, Clone)]
struct ShowcaseWindowMeasurement {
    id: String,
    size: UiSize,
    min_size: UiSize,
    collapsed_size: UiSize,
}

#[derive(Clone, Copy)]
struct StylingState {
    inner_same: bool,
    inner_margin: f32,
    inner_right: f32,
    inner_top: f32,
    inner_bottom: f32,
    outer_same: bool,
    outer_margin: f32,
    outer_right: f32,
    outer_top: f32,
    outer_bottom: f32,
    radius_same: bool,
    corner_radius: f32,
    corner_ne: f32,
    corner_sw: f32,
    corner_se: f32,
    shadow_x: f32,
    shadow_y: f32,
    shadow_blur: f32,
    shadow_spread: f32,
    shadow: ColorRgba,
    stroke_width: f32,
    stroke: ColorRgba,
    fill: ColorRgba,
}

impl Default for StylingState {
    fn default() -> Self {
        Self {
            inner_same: true,
            inner_margin: 12.0,
            inner_right: 12.0,
            inner_top: 12.0,
            inner_bottom: 12.0,
            outer_same: true,
            outer_margin: 24.0,
            outer_right: 24.0,
            outer_top: 24.0,
            outer_bottom: 24.0,
            radius_same: true,
            corner_radius: 12.0,
            corner_ne: 12.0,
            corner_sw: 12.0,
            corner_se: 12.0,
            shadow_x: 8.0,
            shadow_y: 12.0,
            shadow_blur: 16.0,
            shadow_spread: 0.0,
            shadow: ColorRgba::new(0, 0, 0, 140),
            stroke_width: 1.0,
            stroke: ColorRgba::new(198, 198, 205, 255),
            fill: ColorRgba::new(100, 55, 205, 255),
        }
    }
}

impl StylingState {
    fn inner_edges(self) -> [f32; 4] {
        if self.inner_same {
            [self.inner_margin; 4]
        } else {
            [
                self.inner_margin,
                self.inner_right,
                self.inner_top,
                self.inner_bottom,
            ]
        }
    }

    fn outer_edges(self) -> [f32; 4] {
        if self.outer_same {
            [self.outer_margin; 4]
        } else {
            [
                self.outer_margin,
                self.outer_right,
                self.outer_top,
                self.outer_bottom,
            ]
        }
    }

    fn radii(self) -> CornerRadii {
        if self.radius_same {
            CornerRadii::uniform(self.corner_radius)
        } else {
            CornerRadii::new(
                self.corner_radius,
                self.corner_ne,
                self.corner_se,
                self.corner_sw,
            )
        }
    }

    fn stroke_color(self) -> ColorRgba {
        self.stroke
    }

    fn fill_color(self) -> ColorRgba {
        self.fill
    }

    fn shadow_color(self) -> ColorRgba {
        self.shadow
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusedTextInput {
    Editable,
    Selectable,
    Singleline,
    Multiline,
    TextArea,
    CodeEditor,
    Search,
    Password,
    FormName,
    FormEmail,
    FormRole,
    NumericValue,
    NumericRangeMin,
    NumericRangeMax,
    SliderValue,
    SliderRangeLeft,
    SliderRangeRight,
    SliderStep,
    ShaderLabSource,
}

impl FocusedTextInput {
    const fn is_read_only(self) -> bool {
        matches!(self, Self::Selectable)
    }

    const fn is_multiline(self) -> bool {
        matches!(
            self,
            Self::Multiline | Self::TextArea | Self::CodeEditor | Self::ShaderLabSource
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SliderThumbChoice {
    Circle,
    Square,
    Rectangle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragDropDemoPayload {
    Text,
    File,
    ImageBytes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragDropDemoTarget {
    Text,
    FilesOnly,
    ImageBytes,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShaderLabTarget {
    Canvas,
    Frame,
    Button,
}

impl ShaderLabTarget {
    const ALL: [Self; 3] = [Self::Canvas, Self::Frame, Self::Button];

    const fn id(self) -> &'static str {
        match self {
            Self::Canvas => "canvas",
            Self::Frame => "frame",
            Self::Button => "button",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Canvas => "Canvas",
            Self::Frame => "Frame",
            Self::Button => "Button",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "canvas" => Some(Self::Canvas),
            "frame" => Some(Self::Frame),
            "button" => Some(Self::Button),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShaderLabPreset {
    Plasma,
    Rings,
    Grid,
    VertexWarp,
}

impl ShaderLabPreset {
    const ALL: [Self; 4] = [Self::Plasma, Self::Rings, Self::Grid, Self::VertexWarp];

    const fn id(self) -> &'static str {
        match self {
            Self::Plasma => "plasma",
            Self::Rings => "rings",
            Self::Grid => "grid",
            Self::VertexWarp => "vertex_warp",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Plasma => "Plasma",
            Self::Rings => "Rings",
            Self::Grid => "Grid",
            Self::VertexWarp => "Vertex warp",
        }
    }

    const fn source(self) -> &'static str {
        match self {
            Self::Plasma => SHADER_LAB_PLASMA_WGSL,
            Self::Rings => SHADER_LAB_RINGS_WGSL,
            Self::Grid => SHADER_LAB_GRID_WGSL,
            Self::VertexWarp => SHADER_LAB_VERTEX_WARP_WGSL,
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "plasma" => Some(Self::Plasma),
            "rings" => Some(Self::Rings),
            "grid" => Some(Self::Grid),
            "vertex_warp" => Some(Self::VertexWarp),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShaderLabMaterialShader {
    None,
    Tint,
    Shine,
    Glow,
    Plasma,
    Rings,
    Grid,
}

impl ShaderLabMaterialShader {
    const ALL: [Self; 7] = [
        Self::None,
        Self::Tint,
        Self::Shine,
        Self::Glow,
        Self::Plasma,
        Self::Rings,
        Self::Grid,
    ];

    const fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Tint => "tint",
            Self::Shine => "shine",
            Self::Glow => "glow",
            Self::Plasma => "plasma",
            Self::Rings => "rings",
            Self::Grid => "grid",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Tint => "Tint",
            Self::Shine => "Shine",
            Self::Glow => "Glow",
            Self::Plasma => "Plasma",
            Self::Rings => "Rings",
            Self::Grid => "Grid",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "none" => Some(Self::None),
            "tint" => Some(Self::Tint),
            "shine" => Some(Self::Shine),
            "glow" => Some(Self::Glow),
            "plasma" => Some(Self::Plasma),
            "rings" => Some(Self::Rings),
            "grid" => Some(Self::Grid),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShaderLabMaterialShape {
    Rect,
    Rounded,
    Circle,
    Hexagon,
}

impl ShaderLabMaterialShape {
    const ALL: [Self; 4] = [Self::Rect, Self::Rounded, Self::Circle, Self::Hexagon];

    const fn id(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Rounded => "rounded",
            Self::Circle => "circle",
            Self::Hexagon => "hexagon",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Rect => "Rectangle",
            Self::Rounded => "Rounded",
            Self::Circle => "Circle",
            Self::Hexagon => "Hexagon",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "rect" => Some(Self::Rect),
            "rounded" => Some(Self::Rounded),
            "circle" => Some(Self::Circle),
            "hexagon" => Some(Self::Hexagon),
            _ => None,
        }
    }

    fn shape(self) -> ElementShape {
        match self {
            Self::Rect => ElementShape::rect(),
            Self::Rounded => ElementShape::rounded_rect(16.0),
            Self::Circle => ElementShape::circle(),
            Self::Hexagon => ElementShape::normalized_polygon(vec![
                UiPoint::new(0.50, 0.00),
                UiPoint::new(0.95, 0.25),
                UiPoint::new(0.95, 0.75),
                UiPoint::new(0.50, 1.00),
                UiPoint::new(0.05, 0.75),
                UiPoint::new(0.05, 0.25),
            ]),
        }
    }

    const fn visual_radius(self) -> f32 {
        match self {
            Self::Rect | Self::Hexagon => 4.0,
            Self::Rounded => 16.0,
            Self::Circle => 999.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShaderLabMaterialGeometry {
    None,
    PulseScale,
    Skew,
    Wave,
}

impl ShaderLabMaterialGeometry {
    const ALL: [Self; 4] = [Self::None, Self::PulseScale, Self::Skew, Self::Wave];

    const fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::PulseScale => "pulse_scale",
            Self::Skew => "skew",
            Self::Wave => "wave",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::PulseScale => "Pulse scale",
            Self::Skew => "Skew",
            Self::Wave => "Wave",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "none" => Some(Self::None),
            "pulse_scale" => Some(Self::PulseScale),
            "skew" => Some(Self::Skew),
            "wave" => Some(Self::Wave),
            _ => None,
        }
    }

    const fn effect(self) -> GeometryEffect {
        match self {
            Self::None => GeometryEffect::None,
            Self::PulseScale => GeometryEffect::PulseScale { max_scale: 1.18 },
            Self::Skew => GeometryEffect::Skew { x: 0.12, y: 0.0 },
            Self::Wave => GeometryEffect::Wave { amplitude: 10.0 },
        }
    }
}

fn shader_lab_target_options() -> Vec<ext_widgets::SelectOption> {
    ShaderLabTarget::ALL
        .into_iter()
        .map(|target| ext_widgets::SelectOption::new(target.id(), target.label()))
        .collect()
}

fn shader_lab_preset_options() -> Vec<ext_widgets::SelectOption> {
    ShaderLabPreset::ALL
        .into_iter()
        .map(|preset| ext_widgets::SelectOption::new(preset.id(), preset.label()))
        .collect()
}

fn shader_lab_material_shader_options() -> Vec<ext_widgets::SelectOption> {
    ShaderLabMaterialShader::ALL
        .into_iter()
        .map(|shader| ext_widgets::SelectOption::new(shader.id(), shader.label()))
        .collect()
}

fn shader_lab_material_shape_options() -> Vec<ext_widgets::SelectOption> {
    ShaderLabMaterialShape::ALL
        .into_iter()
        .map(|shape| ext_widgets::SelectOption::new(shape.id(), shape.label()))
        .collect()
}

fn shader_lab_material_geometry_options() -> Vec<ext_widgets::SelectOption> {
    ShaderLabMaterialGeometry::ALL
        .into_iter()
        .map(|geometry| ext_widgets::SelectOption::new(geometry.id(), geometry.label()))
        .collect()
}

const SHADER_LAB_PLASMA_WGSL: &str = r#"override TIME: f32 = 0.0;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5, 0.5);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let p = input.uv * 2.0 - vec2<f32>(1.0, 1.0);
    let a = sin((p.x * 7.0 + TIME * 2.4));
    let b = sin((p.y * 8.0 - TIME * 1.8));
    let c = sin((length(p) * 12.0 - TIME * 3.2));
    let value = (a + b + c) / 3.0;
    let cold = vec3<f32>(0.05, 0.16, 0.42);
    let hot = vec3<f32>(0.10, 0.78, 0.92);
    let flare = vec3<f32>(0.95, 0.55, 0.20) * pow(max(value, 0.0), 2.0);
    return vec4<f32>(mix(cold, hot, value * 0.5 + 0.5) + flare, 1.0);
}
"#;

const SHADER_LAB_RINGS_WGSL: &str = r#"override TIME: f32 = 0.0;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5, 0.5);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let center = vec2<f32>(0.5 + sin(TIME * 1.1) * 0.08, 0.5 + cos(TIME * 0.9) * 0.08);
    let p = input.uv - center;
    let d = length(p);
    let ring = 0.5 + 0.5 * cos((d * 18.0 - TIME * 2.0) * 6.28318);
    let fade = 1.0 - smoothstep(0.15, 0.74, d);
    let base = vec3<f32>(0.08, 0.06, 0.15);
    let color = base + vec3<f32>(1.0, 0.55, 0.18) * ring * fade;
    return vec4<f32>(color, 1.0);
}
"#;

const SHADER_LAB_GRID_WGSL: &str = r#"override TIME: f32 = 0.0;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5, 0.5);
    return output;
}

fn grid_line(value: f32) -> f32 {
    let cell = abs(fract(value) - 0.5);
    return 1.0 - smoothstep(0.46, 0.50, cell);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = input.uv + vec2<f32>(TIME * 0.04, TIME * -0.03);
    let major = max(grid_line(uv.x * 8.0), grid_line(uv.y * 8.0));
    let minor = max(grid_line(uv.x * 24.0), grid_line(uv.y * 24.0)) * 0.28;
    let glow = max(major, minor);
    let base = vec3<f32>(0.03, 0.04, 0.07);
    let color = base + vec3<f32>(0.54, 0.38, 1.0) * glow;
    return vec4<f32>(color, 1.0);
}
"#;

const SHADER_LAB_VERTEX_WARP_WGSL: &str = r#"override TIME: f32 = 0.0;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let uv_points = array<vec2<f32>, 3>(
        vec2<f32>(0.06, 0.12),
        vec2<f32>(0.96, 0.18),
        vec2<f32>(0.16, 0.94),
    );
    let uv = uv_points[vertex_index];
    let p = uv * 2.0 - vec2<f32>(1.0, 1.0);
    let wave = sin(TIME * 2.2 + f32(vertex_index) * 2.1);
    let bend = vec2<f32>(
        0.12 * wave,
        0.10 * cos(TIME * 1.7 + f32(vertex_index) * 1.6),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(p + bend, 0.0, 1.0);
    output.uv = uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let stripes = 0.5 + 0.5 * sin((input.uv.x + input.uv.y) * 24.0 - TIME * 4.0);
    let edge = smoothstep(0.02, 0.12, min(min(input.uv.x, input.uv.y), 1.0 - max(input.uv.x, input.uv.y)));
    let base = vec3<f32>(0.18, 0.10, 0.42);
    let hot = vec3<f32>(0.98, 0.65, 0.20);
    let color = mix(base, hot, stripes) + vec3<f32>(0.10, 0.32, 0.70) * input.uv.x;
    return vec4<f32>(color * (0.72 + edge * 0.28), 1.0);
}
"#;

const SHADER_LAB_ERROR_WGSL: &str = r#"override TIME: f32 = 0.0;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5, 0.5);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let stripe = step(0.5, fract((input.uv.x + input.uv.y + TIME * 0.04) * 14.0));
    let dark = vec3<f32>(0.10, 0.02, 0.04);
    let hot = vec3<f32>(0.58, 0.05, 0.12);
    return vec4<f32>(mix(dark, hot, stripe), 1.0);
}
"#;

fn drag_drop_preview_status(
    payload: DragDropDemoPayload,
    target: DragDropDemoTarget,
) -> &'static str {
    match (payload, target) {
        (_, DragDropDemoTarget::Disabled) => "Drop target disabled",
        (DragDropDemoPayload::Text, DragDropDemoTarget::Text) => "Text payload can be dropped",
        (DragDropDemoPayload::Text, DragDropDemoTarget::FilesOnly) => {
            "Text payload rejected: files only"
        }
        (DragDropDemoPayload::Text, DragDropDemoTarget::ImageBytes) => {
            "Text payload rejected: image bytes only"
        }
        (DragDropDemoPayload::File, DragDropDemoTarget::FilesOnly) => "File payload can be dropped",
        (DragDropDemoPayload::File, DragDropDemoTarget::Text) => "File payload rejected: text only",
        (DragDropDemoPayload::File, DragDropDemoTarget::ImageBytes) => {
            "File payload rejected: image bytes only"
        }
        (DragDropDemoPayload::ImageBytes, DragDropDemoTarget::ImageBytes) => {
            "Image bytes can be dropped"
        }
        (DragDropDemoPayload::ImageBytes, DragDropDemoTarget::Text) => {
            "Image bytes rejected: text only"
        }
        (DragDropDemoPayload::ImageBytes, DragDropDemoTarget::FilesOnly) => {
            "Image bytes rejected: files only"
        }
    }
}

fn drag_drop_drop_status(payload: DragDropDemoPayload, target: DragDropDemoTarget) -> &'static str {
    match (payload, target) {
        (_, DragDropDemoTarget::Disabled) => "Drop failed: target disabled",
        (DragDropDemoPayload::Text, DragDropDemoTarget::Text) => "Text payload accepted",
        (DragDropDemoPayload::Text, _) => "Text drag failed",
        (DragDropDemoPayload::File, DragDropDemoTarget::FilesOnly) => "File payload accepted",
        (DragDropDemoPayload::File, _) => "File drag failed",
        (DragDropDemoPayload::ImageBytes, DragDropDemoTarget::ImageBytes) => "Image bytes accepted",
        (DragDropDemoPayload::ImageBytes, _) => "Image byte drag failed",
    }
}

#[derive(Clone, Debug)]
struct EditableTreeNode {
    id: String,
    label: String,
    children: Vec<EditableTreeNode>,
}

impl EditableTreeNode {
    fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
        }
    }

    fn with_children(mut self, children: Vec<EditableTreeNode>) -> Self {
        self.children = children;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DateDemoMode {
    Single,
    Range,
    Week,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiagnosticsPage {
    Overview,
    Frame,
    Layout,
    Hit,
    Input,
    Paint,
    Text,
    State,
    Accessibility,
    LegacyAll,
}

impl DiagnosticsPage {
    const VISIBLE: [Self; 9] = [
        Self::Overview,
        Self::Frame,
        Self::Layout,
        Self::Hit,
        Self::Input,
        Self::Paint,
        Self::Text,
        Self::State,
        Self::Accessibility,
    ];

    fn visible() -> &'static [Self] {
        &Self::VISIBLE
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Frame => "Frame",
            Self::Layout => "Layout",
            Self::Hit => "Hit",
            Self::Input => "Input",
            Self::Paint => "Paint",
            Self::Text => "Text",
            Self::State => "State",
            Self::Accessibility => "A11y",
            Self::LegacyAll => "All",
        }
    }

    const fn action_suffix(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Frame => "frame",
            Self::Layout => "layout",
            Self::Hit => "hit",
            Self::Input => "input",
            Self::Paint => "paint",
            Self::Text => "text",
            Self::State => "state",
            Self::Accessibility => "a11y",
            Self::LegacyAll => "all",
        }
    }

    fn from_action_suffix(suffix: &str) -> Option<Self> {
        match suffix {
            "overview" => Some(Self::Overview),
            "frame" => Some(Self::Frame),
            "layout" => Some(Self::Layout),
            "hit" => Some(Self::Hit),
            "input" => Some(Self::Input),
            "paint" => Some(Self::Paint),
            "text" => Some(Self::Text),
            "state" => Some(Self::State),
            "a11y" => Some(Self::Accessibility),
            "all" => Some(Self::LegacyAll),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShowcaseThemeChoice {
    Light,
    Dark,
    Bubblegum,
}

impl ShowcaseThemeChoice {
    const fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Bubblegum => "Bubblegum",
        }
    }

    const fn action(self) -> &'static str {
        match self {
            Self::Light => "theme.demo.light",
            Self::Dark => "theme.demo.dark",
            Self::Bubblegum => "theme.demo.bubblegum",
        }
    }

    fn theme(self) -> Theme {
        match self {
            Self::Light => Theme::light(),
            Self::Dark => Theme::dark(),
            Self::Bubblegum => Theme::bubblegum(),
        }
    }
}

thread_local! {
    static SHOWCASE_ACTIVE_THEME: std::cell::Cell<ShowcaseThemeChoice> =
        std::cell::Cell::new(ShowcaseThemeChoice::Dark);
}

fn set_showcase_active_theme(choice: ShowcaseThemeChoice) {
    SHOWCASE_ACTIVE_THEME.with(|active| active.set(choice));
}

fn active_showcase_theme_choice() -> ShowcaseThemeChoice {
    SHOWCASE_ACTIVE_THEME.with(std::cell::Cell::get)
}

fn active_showcase_colors() -> ColorTokens {
    match active_showcase_theme_choice() {
        ShowcaseThemeChoice::Light => ColorTokens::light(),
        ShowcaseThemeChoice::Dark => ColorTokens::dark(),
        ShowcaseThemeChoice::Bubblegum => ColorTokens::bubblegum(),
    }
}

#[derive(Clone, Copy)]
struct CanvasCubeState {
    yaw: f32,
    pitch: f32,
    drag_origin_yaw: f32,
    drag_origin_pitch: f32,
}

impl Default for CanvasCubeState {
    fn default() -> Self {
        Self {
            yaw: 0.82,
            pitch: 0.52,
            drag_origin_yaw: 0.82,
            drag_origin_pitch: 0.52,
        }
    }
}

impl CanvasCubeState {
    fn apply_drag(&mut self, drag: WidgetDrag) {
        match drag.phase {
            WidgetDragPhase::Begin => {
                self.drag_origin_yaw = self.yaw;
                self.drag_origin_pitch = self.pitch;
                self.apply_drag_delta(drag.total_delta);
            }
            WidgetDragPhase::Update | WidgetDragPhase::Commit => {
                self.apply_drag_delta(drag.total_delta);
            }
            WidgetDragPhase::Cancel => {
                self.yaw = self.drag_origin_yaw;
                self.pitch = self.drag_origin_pitch;
            }
        }
    }

    fn apply_drag_delta(&mut self, total_delta: UiPoint) {
        self.yaw = self.drag_origin_yaw + total_delta.x * 0.012;
        self.pitch = (self.drag_origin_pitch + total_delta.y * 0.012).clamp(-1.25, 1.25);
    }
}

impl Default for ShowcaseState {
    fn default() -> Self {
        let text = TextInputState::new("Editable text");
        let mut selectable_text = TextInputState::new("Selectable read-only text");
        selectable_text.set_selection(0, "Selectable".len());
        let form = profile_form_state();
        let form_name_text = TextInputState::new(profile_form_value(&form, "name"));
        let form_email_text = TextInputState::new(profile_form_value(&form, "email"));
        let form_role_text = TextInputState::new(profile_form_value(&form, "role"));
        let initial_select_options = select_options();
        let initial_image_select_options = select_options_with_images();
        let windows = ShowcaseWindows::default();
        let mut desktop = ext_widgets::FloatingDesktopState::with_visible_order(
            SHOWCASE_WIDGET_WINDOW_IDS
                .into_iter()
                .filter(|id| windows.is_visible(id))
                .map(str::to_string),
            showcase_window_z_policy(),
        );
        for id in SHOWCASE_WIDGET_WINDOW_IDS
            .into_iter()
            .filter(|id| windows.is_visible(id))
        {
            desktop.ensure_window(id, window_defaults(id));
        }

        Self {
            checked: true,
            checkbox_indeterminate: widgets::CheckboxState::Indeterminate,
            slider: 10.0,
            slider_left: 1.0,
            slider_right: 10000.0,
            slider_value_text: TextInputState::new("10"),
            slider_left_text: TextInputState::new("1"),
            slider_right_text: TextInputState::new("10000"),
            slider_step_value: 10.0,
            slider_step_text: TextInputState::new("10"),
            slider_trailing_color: true,
            slider_trailing_picker: ext_widgets::ColorPickerState::new(color(120, 170, 230)),
            slider_trailing_picker_open: false,
            slider_thumb_picker: ext_widgets::ColorPickerState::new(color(235, 240, 247)),
            slider_thumb_picker_open: false,
            slider_thumb_shape: SliderThumbChoice::Circle,
            slider_use_steps: false,
            slider_logarithmic: true,
            slider_clamping: widgets::SliderClamping::Always,
            slider_smart_aim: true,
            label_locale: ext_widgets::SelectMenuState::with_selected(1),
            label_link_visited: false,
            label_hyperlink_visited: false,
            label_link_status: "No link action yet",
            color: ext_widgets::ColorPickerState::new(color(118, 183, 255)),
            date: ext_widgets::DatePickerModel::builder()
                .selected(CalendarDate::new(2026, 5, 12))
                .today(CalendarDate::new(2026, 5, 12))
                .build(),
            date_range: ext_widgets::DateRangePickerModel::builder()
                .range(Some(ext_widgets::CalendarDateRange::new(
                    CalendarDate::new(2026, 5, 12).expect("demo range start"),
                    CalendarDate::new(2026, 5, 18).expect("demo range end"),
                )))
                .today(CalendarDate::new(2026, 5, 12))
                .build(),
            date_mode: DateDemoMode::Single,
            radio_choice: "foo",
            switch_enabled: true,
            mixed_switch: ext_widgets::ToggleValue::Mixed,
            theme_preference: widgets::ThemePreference::Dark,
            showcase_theme: ShowcaseThemeChoice::Dark,
            numeric_value: 42.0,
            numeric_text: TextInputState::new("42.0"),
            numeric_range_min: 0.0,
            numeric_range_max: 100.0,
            numeric_range_min_text: TextInputState::new("0.0"),
            numeric_range_max_text: TextInputState::new("100.0"),
            numeric_sensitivity: 1.0,
            numeric_unit: ext_widgets::SelectMenuState::with_selected(0),
            numeric_drag_start: None,
            dropdown: ext_widgets::SelectMenuState::with_selected(1),
            select_menu: ext_widgets::SelectMenuState::with_selected(0)
                .with_open(&initial_select_options)
                .with_active(&initial_select_options, 2),
            image_select_menu: ext_widgets::SelectMenuState::with_selected(0)
                .with_open(&initial_image_select_options)
                .with_active(&initial_image_select_options, 1),
            text,
            selectable_text,
            singleline_text: TextInputState::new("Single line"),
            multiline_text: TextInputState::new("First line\nSecond line").multiline(true),
            text_area_text: TextInputState::new("Text area content").multiline(true),
            code_editor_text: TextInputState::new("fn main() {\n    println!(\"showcase\");\n}")
                .multiline(true),
            search_text: TextInputState::new("widgets"),
            password_text: TextInputState::new("correct horse"),
            focused_text: None,
            platform: PlatformServiceClient::new(),
            clipboard_text: String::new(),
            pending_clipboard_paste: None,
            last_button: "None",
            toggle_button: false,
            table_selection: ext_widgets::DataTableSelection::single_row(2)
                .with_active_cell(ext_widgets::DataTableCellIndex::new(2, 1)),
            tree: ext_widgets::TreeViewState::expanded(["root", "child-0", "child-0-3", "child-1"]),
            editable_tree: editable_tree_default_nodes(),
            editable_tree_next_id: 100,
            editable_tree_status: "Use row buttons to add or delete children".to_owned(),
            outliner: ext_widgets::TreeViewState::expanded(["root", "assets"]),
            tree_virtual: ext_widgets::TreeViewState::expanded(["root", "src"]),
            tree_virtual_scroll: 0.0,
            tree_table: ext_widgets::TreeViewState::expanded(["root", "branch-a"]),
            tree_table_scroll: 0.0,
            toast_visible: false,
            toast_action_status: "No toast action",
            progress_phase: 0.0,
            progress_loading_elapsed: 0.0,
            progress_logs_scroll: operad::ScrollState::new(ScrollAxes::VERTICAL),
            progress_logs_follow_tail: true,
            animation_scrub: 0.0,
            animation_open: false,
            animation_timed_expanded: true,
            animation_scrub_expanded: true,
            animation_state_expanded: true,
            animation_interaction_expanded: true,
            easing_in: ext_widgets::SelectMenuState::with_selected(1),
            easing_out: ext_widgets::SelectMenuState::with_selected(1),
            caret_phase: 0.0,
            command_palette: ext_widgets::CommandPaletteState::new()
                .with_max_results(24)
                .with_first_active_match(&command_palette_items()),
            command_palette_open: false,
            command_history: ext_widgets::CommandPaletteHistory::with_capacity(4),
            last_command: "None".to_string(),
            list_scroll: 0.0,
            virtual_scroll: 0.0,
            table_scroll: 0.0,
            virtual_table_scroll: 0.0,
            virtual_table_descending: false,
            virtual_table_ready_only: false,
            virtual_table_value_width: 120.0,
            virtual_table_resize: None,
            layout_panel_a_scroll: 0.0,
            layout_panel_b_scroll: 0.0,
            layout_workspace_scroll: 0.0,
            scrollbars: scrollbar_widgets::ScrollbarControllerState::new(),
            layout_tab: 0,
            styling: StylingState::default(),
            styling_stroke_picker: ext_widgets::ColorPickerState::new(
                StylingState::default().stroke_color(),
            ),
            styling_stroke_picker_open: false,
            styling_fill_picker: ext_widgets::ColorPickerState::new(
                StylingState::default().fill_color(),
            ),
            styling_fill_picker_open: false,
            styling_shadow_picker: ext_widgets::ColorPickerState::new(
                StylingState::default().shadow_color(),
            ),
            styling_shadow_picker_open: false,
            cube: CanvasCubeState::default(),
            canvas_grow_horizontal: true,
            canvas_grow_vertical: true,
            canvas_keep_aspect_ratio: true,
            menu_bar: ext_widgets::MenuBarState {
                open_menu: Some(0),
                active_item: Some(0),
            },
            menu_button: ext_widgets::MenuButtonState::new(),
            image_text_menu_button: ext_widgets::MenuButtonState::new(),
            image_menu_button: ext_widgets::MenuButtonState::new(),
            context_menu: ext_widgets::ContextMenuState::closed(),
            menu_autosave: true,
            menu_grid: true,
            form,
            form_name_text,
            form_email_text,
            form_role_text,
            form_newsletter: true,
            form_status: "Unsaved profile changes".to_string(),
            overlay_expanded: true,
            overlay_popup_open: false,
            overlay_modal_open: false,
            color_picker_button_open: false,
            drag_drop_active_payload: None,
            drag_drop_status: "Idle",
            drag_drop_cursor_shape: CursorShape::Default,
            shader_lab_split: ext_widgets::SplitPaneState::new(0.52)
                .with_min_sizes(SHADER_LAB_PREVIEW_MIN_WIDTH, SHADER_LAB_EDITOR_MIN_WIDTH),
            shader_lab_editor_scroll: UiPoint::new(0.0, 0.0),
            shader_lab_show_frame_text: true,
            shader_lab_show_button_text: true,
            shader_lab_surface_stroke_width: 1.0,
            shader_lab_surface_radius: 8.0,
            shader_lab_target: ShaderLabTarget::Canvas,
            shader_lab_target_menu: ext_widgets::SelectMenuState::with_selected(0),
            shader_lab_preset: ShaderLabPreset::Plasma,
            shader_lab_preset_menu: ext_widgets::SelectMenuState::with_selected(0),
            shader_lab_material_shader: ShaderLabMaterialShader::Glow,
            shader_lab_material_shader_menu: ext_widgets::SelectMenuState::with_selected(3),
            shader_lab_material_shape: ShaderLabMaterialShape::Rounded,
            shader_lab_material_shape_menu: ext_widgets::SelectMenuState::with_selected(1),
            shader_lab_material_geometry: ShaderLabMaterialGeometry::Wave,
            shader_lab_material_geometry_menu: ext_widgets::SelectMenuState::with_selected(3),
            shader_lab_material_outset: SHADER_LAB_MATERIAL_OUTSET,
            shader_lab_source: TextInputState::new(ShaderLabPreset::Plasma.source())
                .multiline(true),
            shader_lab_source_error: None,
            timeline_scroll: operad::ScrollState::new(ScrollAxes::HORIZONTAL).with_sizes(
                UiSize::new(620.0, TIMELINE_VIEWPORT_HEIGHT),
                UiSize::new(TIMELINE_CONTENT_WIDTH, TIMELINE_VIEWPORT_HEIGHT),
            ),
            panels_top_split: ext_widgets::SplitPaneState::new(0.18).with_min_sizes(46.0, 150.0),
            panels_bottom_split: ext_widgets::SplitPaneState::new(0.74).with_min_sizes(120.0, 46.0),
            panels_left_split: ext_widgets::SplitPaneState::new(0.22).with_min_sizes(76.0, 180.0),
            panels_right_split: ext_widgets::SplitPaneState::new(0.74).with_min_sizes(120.0, 76.0),
            layout_split: ext_widgets::SplitPaneState::new(0.44).with_min_sizes(80.0, 80.0),
            layout_dock: ext_widgets::DockWorkspaceState::new(),
            diagnostics_animation_paused: false,
            diagnostics_animation_scrub: 0.35,
            diagnostics_animation_active: true,
            diagnostics_animation_hover: 0.35,
            diagnostics_animation_pulse_count: 0,
            diagnostics_page: DiagnosticsPage::Overview,
            diagnostics_snapshot: diagnostics_sample_snapshot_for(0.35, true),
            containers_scroll: operad::ScrollState::new(ScrollAxes::VERTICAL)
                .with_sizes(UiSize::new(260.0, 82.0), UiSize::new(260.0, 180.0))
                .with_offset(UiPoint::new(0.0, 18.0)),
            controls_scroll: operad::ScrollState::new(ScrollAxes::VERTICAL),
            color_copied_hex: None,
            fps_last_sample: Instant::now(),
            fps_frames: 0,
            fps: 0.0,
            last_desktop_size: desktop_size_for_viewport(UiSize::new(900.0, 760.0)),
            initial_organize_pending: true,
            windows,
            desktop,
            user_image_update: showcase_user_image_update(),
        }
    }
}

struct ShowcaseWindows {
    labels: bool,
    buttons: bool,
    checkbox: bool,
    toggles: bool,
    slider: bool,
    numeric: bool,
    text_input: bool,
    selection: bool,
    menus: bool,
    command_palette: bool,
    date_picker: bool,
    color_picker: bool,
    progress: bool,
    animation: bool,
    easing: bool,
    lists_tables: bool,
    property_inspector: bool,
    diagnostics: bool,
    trees: bool,
    layout_widgets: bool,
    containers: bool,
    panels: bool,
    forms: bool,
    overlays: bool,
    drag_drop: bool,
    media: bool,
    shaders: bool,
    shader_lab: bool,
    timeline: bool,
    canvas: bool,
    theme: bool,
    styling: bool,
}

impl Default for ShowcaseWindows {
    fn default() -> Self {
        Self {
            labels: true,
            buttons: true,
            checkbox: false,
            toggles: false,
            slider: false,
            numeric: false,
            text_input: false,
            selection: false,
            menus: false,
            command_palette: false,
            date_picker: false,
            color_picker: true,
            progress: false,
            animation: false,
            easing: false,
            lists_tables: false,
            property_inspector: false,
            diagnostics: false,
            trees: false,
            layout_widgets: false,
            containers: false,
            panels: false,
            forms: false,
            overlays: false,
            drag_drop: false,
            media: false,
            shaders: false,
            shader_lab: false,
            timeline: false,
            canvas: true,
            theme: false,
            styling: false,
        }
    }
}

impl ShowcaseWindows {
    fn is_visible(&self, id: &str) -> bool {
        match id {
            "labels" => self.labels,
            "buttons" => self.buttons,
            "checkbox" => self.checkbox,
            "toggles" => self.toggles,
            "slider" => self.slider,
            "numeric" => self.numeric,
            "text_input" => self.text_input,
            "selection" => self.selection,
            "menus" => self.menus,
            "command_palette" => self.command_palette,
            "date_picker" => self.date_picker,
            "color_picker" => self.color_picker,
            "progress" => self.progress,
            "animation" => self.animation,
            "easing" => self.easing,
            "lists_tables" => self.lists_tables,
            "property_inspector" => self.property_inspector,
            "diagnostics" => self.diagnostics,
            "trees" => self.trees,
            "layout_widgets" => self.layout_widgets,
            "containers" => self.containers,
            "panels" => self.panels,
            "forms" => self.forms,
            "overlays" => self.overlays,
            "drag_drop" => self.drag_drop,
            "media" => self.media,
            "shaders" => self.shaders,
            "shader_lab" => self.shader_lab,
            "timeline" => self.timeline,
            "canvas" => self.canvas,
            "theme" => self.theme,
            "styling" => self.styling,
            _ => false,
        }
    }

    fn slot_mut(&mut self, id: &str) -> Option<&mut bool> {
        match id {
            "labels" => Some(&mut self.labels),
            "buttons" => Some(&mut self.buttons),
            "checkbox" => Some(&mut self.checkbox),
            "toggles" => Some(&mut self.toggles),
            "slider" => Some(&mut self.slider),
            "numeric" => Some(&mut self.numeric),
            "text_input" => Some(&mut self.text_input),
            "selection" => Some(&mut self.selection),
            "menus" => Some(&mut self.menus),
            "command_palette" => Some(&mut self.command_palette),
            "date_picker" => Some(&mut self.date_picker),
            "color_picker" => Some(&mut self.color_picker),
            "progress" => Some(&mut self.progress),
            "animation" => Some(&mut self.animation),
            "easing" => Some(&mut self.easing),
            "lists_tables" => Some(&mut self.lists_tables),
            "property_inspector" => Some(&mut self.property_inspector),
            "diagnostics" => Some(&mut self.diagnostics),
            "trees" => Some(&mut self.trees),
            "layout_widgets" => Some(&mut self.layout_widgets),
            "containers" => Some(&mut self.containers),
            "panels" => Some(&mut self.panels),
            "forms" => Some(&mut self.forms),
            "overlays" => Some(&mut self.overlays),
            "drag_drop" => Some(&mut self.drag_drop),
            "media" => Some(&mut self.media),
            "shaders" => Some(&mut self.shaders),
            "shader_lab" => Some(&mut self.shader_lab),
            "timeline" => Some(&mut self.timeline),
            "canvas" => Some(&mut self.canvas),
            "theme" => Some(&mut self.theme),
            "styling" => Some(&mut self.styling),
            _ => None,
        }
    }

    fn toggle(&mut self, id: &str) -> Option<bool> {
        if let Some(visible) = self.slot_mut(id) {
            *visible = !*visible;
            return Some(*visible);
        }
        None
    }

    fn close(&mut self, id: &str) {
        if let Some(visible) = self.slot_mut(id) {
            *visible = false;
        }
    }

    fn clear_all(&mut self) {
        for id in SHOWCASE_WIDGET_WINDOW_IDS {
            if let Some(visible) = self.slot_mut(id) {
                *visible = false;
            }
        }
    }

    fn open_all(&mut self) {
        for id in SHOWCASE_WIDGET_WINDOW_IDS {
            if let Some(visible) = self.slot_mut(id) {
                *visible = true;
            }
        }
    }
}

fn showcase_window_z_policy() -> ext_widgets::FloatingDesktopZPolicy {
    ext_widgets::FloatingDesktopZPolicy::new(
        SHOWCASE_WINDOW_Z_BASE,
        SHOWCASE_WINDOW_Z_STRIDE,
        SHOWCASE_WINDOW_Z_MAX,
    )
}

fn window_defaults(id: &str) -> ext_widgets::FloatingWindowDefaults {
    ext_widgets::FloatingWindowDefaults::new(
        default_window_position(id),
        default_window_size(id),
        default_window_state_min_size(id),
    )
}

fn desktop_size_for_viewport(viewport: UiSize) -> UiSize {
    UiSize::new(
        (viewport.width - RIGHT_PANEL_WIDTH).max(360.0),
        viewport.height,
    )
}

fn showcase_desktop_options(
    desktop_size: UiSize,
    theme: &Theme,
) -> ext_widgets::FloatingDesktopOptions {
    let mut options = ext_widgets::FloatingDesktopOptions::new(desktop_size).with_layout(
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height_percent(1.0),
    );
    options = options.with_bounds_rect(UiRect::new(
        0.0,
        SHOWCASE_ORGANIZE_BUTTON_RESERVED_HEIGHT,
        desktop_size.width,
        (desktop_size.height - SHOWCASE_ORGANIZE_BUTTON_RESERVED_HEIGHT).max(0.0),
    ));
    options.base_z_index = SHOWCASE_WINDOW_Z_BASE;
    options.window_z_stride = SHOWCASE_WINDOW_Z_STRIDE;
    options.margin = 18.0;
    options.gap = 14.0;
    options.window_visual = UiVisual::panel(theme.colors.surface, Some(theme.stroke.surface), 0.0);
    options.title_bar_visual =
        UiVisual::panel(theme.colors.surface_muted, Some(theme.stroke.surface), 0.0);
    options.content_visual = UiVisual::panel(theme.colors.surface, None, 0.0);
    options.title_style = themed_text(theme, 13.0);
    options.close_button_text_style = themed_text(theme, 14.0);
    options.close_button_visual = UiVisual::panel(ColorRgba::TRANSPARENT, None, 3.0);
    options.close_button_hovered_visual =
        theme.resolve_visual(ComponentRole::Button, ComponentState::HOVERED);
    options.close_button_pressed_visual =
        theme.resolve_visual(ComponentRole::Button, ComponentState::PRESSED);
    options
}

impl ShowcaseState {
    fn prepare_frame(&mut self, viewport: UiSize) {
        self.last_desktop_size = desktop_size_for_viewport(viewport);
        if self.initial_organize_pending {
            self.organize_open_windows();
            self.initial_organize_pending = false;
        }
        self.record_frame();
    }

    fn record_frame(&mut self) {
        self.fps_frames = self.fps_frames.saturating_add(1);
        let now = Instant::now();
        let elapsed = now
            .checked_duration_since(self.fps_last_sample)
            .unwrap_or(Duration::ZERO);
        if elapsed < SHOWCASE_FPS_SAMPLE_INTERVAL {
            return;
        }
        let seconds = elapsed.as_secs_f32().max(f32::EPSILON);
        self.fps = self.fps_frames as f32 / seconds;
        self.fps_frames = 0;
        self.fps_last_sample = now;
    }

    fn request_drag_drop_cursor(&mut self, shape: CursorShape) {
        if self.drag_drop_cursor_shape == shape {
            return;
        }
        self.drag_drop_cursor_shape = shape;
        self.platform
            .request(PlatformRequest::Cursor(CursorRequest::SetShape(shape)));
    }

    fn apply_drag_drop_source_action(
        &mut self,
        payload: DragDropDemoPayload,
        started: &'static str,
        dragging: &'static str,
        finished: &'static str,
        canceled: &'static str,
        kind: &WidgetActionKind,
    ) {
        match kind {
            WidgetActionKind::Drag(drag) => match drag.phase {
                WidgetDragPhase::Begin => {
                    self.drag_drop_active_payload = Some(payload);
                    self.drag_drop_status = started;
                    self.request_drag_drop_cursor(CursorShape::Grabbing);
                }
                WidgetDragPhase::Update => {
                    self.drag_drop_active_payload = Some(payload);
                    self.drag_drop_status = dragging;
                    self.request_drag_drop_cursor(CursorShape::Grabbing);
                }
                WidgetDragPhase::Commit => {
                    self.drag_drop_status = finished;
                    self.request_drag_drop_cursor(CursorShape::Default);
                }
                WidgetDragPhase::Cancel => {
                    self.drag_drop_active_payload = None;
                    self.drag_drop_status = canceled;
                    self.request_drag_drop_cursor(CursorShape::Default);
                }
            },
            WidgetActionKind::Activate(_) => {
                self.drag_drop_active_payload = None;
                self.drag_drop_status = canceled;
                self.request_drag_drop_cursor(CursorShape::Default);
            }
            _ => {}
        }
    }

    fn apply_drag_drop_target_action(
        &mut self,
        target: DragDropDemoTarget,
        kind: &WidgetActionKind,
    ) {
        let WidgetActionKind::Drag(drag) = kind else {
            return;
        };
        let Some(payload) = self.drag_drop_active_payload else {
            return;
        };
        self.drag_drop_status = match drag.phase {
            WidgetDragPhase::Begin | WidgetDragPhase::Update => {
                drag_drop_preview_status(payload, target)
            }
            WidgetDragPhase::Commit => drag_drop_drop_status(payload, target),
            WidgetDragPhase::Cancel => "Drop canceled",
        };
        if matches!(
            drag.phase,
            WidgetDragPhase::Commit | WidgetDragPhase::Cancel
        ) {
            self.drag_drop_active_payload = None;
            self.request_drag_drop_cursor(CursorShape::Default);
        }
    }

    fn organize_open_windows(&mut self) {
        let desktop_size = self.last_desktop_size;
        let theme = self.app_theme();
        let options = showcase_desktop_options(desktop_size, &theme);
        let arrange_rect = UiRect::new(
            0.0,
            SHOWCASE_ORGANIZE_BUTTON_RESERVED_HEIGHT,
            desktop_size.width,
            (desktop_size.height - SHOWCASE_ORGANIZE_BUTTON_RESERVED_HEIGHT).max(0.0),
        );
        let measured_sizes = self.measured_open_window_sizes(desktop_size);
        let windows = SHOWCASE_WIDGET_WINDOW_IDS
            .into_iter()
            .filter(|id| self.windows.is_visible(id))
            .map(|id| {
                let mut defaults = window_defaults(id);
                let mut collapsed_size =
                    UiSize::new(defaults.min_size.width, options.title_bar_height);
                if let Some(measurement) = measured_sizes
                    .iter()
                    .find(|measurement| measurement.id == id)
                {
                    defaults.size = UiSize::new(
                        measurement.size.width.max(defaults.size.width),
                        measurement.size.height.max(defaults.size.height),
                    );
                    defaults.min_size = UiSize::new(
                        defaults.min_size.width.max(measurement.min_size.width),
                        defaults.min_size.height.max(measurement.min_size.height),
                    );
                    collapsed_size = UiSize::new(
                        collapsed_size.width.max(measurement.collapsed_size.width),
                        collapsed_size.height.max(measurement.collapsed_size.height),
                    );
                }
                ext_widgets::FloatingWindowOrganizeSpec::new(id, defaults)
                    .with_collapsed_size(collapsed_size)
            });
        let _outcome = self
            .desktop
            .organize_window_specs_in_rect(windows, arrange_rect, &options);
    }

    fn measured_open_window_sizes(&self, desktop_size: UiSize) -> Vec<ShowcaseWindowMeasurement> {
        let measure_height = desktop_size.height.max(SHOWCASE_ORGANIZE_MEASURE_HEIGHT);
        let viewport = UiSize::new(desktop_size.width + RIGHT_PANEL_WIDTH, measure_height);
        let mut document = self.window_measurement_view(viewport);
        #[cfg(feature = "text-cosmic")]
        let mut measurer = CosmicTextMeasurer::new();
        #[cfg(not(feature = "text-cosmic"))]
        let mut measurer = ApproxTextMeasurer;
        if document.compute_layout(viewport, &mut measurer).is_err() {
            return Vec::new();
        }
        let theme = self.app_theme();
        let options = showcase_desktop_options(desktop_size, &theme);
        let mut measurements = SHOWCASE_WIDGET_WINDOW_IDS
            .into_iter()
            .filter(|id| self.windows.is_visible(id))
            .filter_map(|id| {
                let name = format!("showcase.windows.window.{id}");
                let collapsed_size = showcase_collapsed_window_size(id, &options);
                document
                    .nodes()
                    .iter()
                    .find(|node| node.name() == name)
                    .map(|node| {
                        let min_size = node.style().layout_style().min_size();
                        ShowcaseWindowMeasurement {
                            id: id.to_string(),
                            size: UiSize::new(node.layout().rect.width, node.layout().rect.height),
                            min_size: UiSize::new(
                                min_size
                                    .and_then(|size| size.width.points_value())
                                    .unwrap_or(node.layout().rect.width),
                                min_size
                                    .and_then(|size| size.height.points_value())
                                    .unwrap_or(node.layout().rect.height),
                            ),
                            collapsed_size,
                        }
                    })
            })
            .collect::<Vec<_>>();

        let mut compact_desktop = self.desktop.clone();
        compact_desktop.collapsed.clear();
        for measurement in &measurements {
            compact_desktop.sizes.insert(
                measurement.id.clone(),
                UiSize::new(
                    measurement.min_size.width,
                    default_window_state_min_size(&measurement.id).height,
                ),
            );
            compact_desktop.user_sized.insert(measurement.id.clone());
        }
        let mut compact_document =
            self.window_measurement_view_with_desktop(viewport, compact_desktop);
        #[cfg(feature = "text-cosmic")]
        let mut compact_measurer = CosmicTextMeasurer::new();
        #[cfg(not(feature = "text-cosmic"))]
        let mut compact_measurer = ApproxTextMeasurer;
        if compact_document
            .compute_layout(viewport, &mut compact_measurer)
            .is_ok()
        {
            for measurement in &mut measurements {
                let name = format!("showcase.windows.window.{}", measurement.id);
                if let Some(node) = compact_document
                    .nodes()
                    .iter()
                    .find(|node| node.name() == name)
                {
                    measurement.min_size.width =
                        measurement.min_size.width.max(node.layout().rect.width);
                    measurement.min_size.height =
                        measurement.min_size.height.max(node.layout().rect.height);
                }
            }
        }

        measurements
    }

    fn window_measurement_view(&self, viewport: UiSize) -> UiDocument {
        self.window_measurement_view_with_desktop(viewport, self.desktop.clone())
    }

    fn window_measurement_view_with_desktop(
        &self,
        viewport: UiSize,
        mut measurement_desktop: ext_widgets::FloatingDesktopState,
    ) -> UiDocument {
        set_showcase_active_theme(self.showcase_theme);
        let theme = self.app_theme();
        let desktop_size = desktop_size_for_viewport(viewport);
        measurement_desktop.collapsed.clear();

        let mut ui = UiDocument::with_capacity(
            root_style(viewport.width, viewport.height),
            SHOWCASE_DOCUMENT_NODE_CAPACITY,
        );
        let root = ui.root();
        let desktop = ui.add_child(
            root,
            UiNode::container(
                "showcase.desktop.measurement",
                LayoutStyle::new()
                    .with_width(desktop_size.width)
                    .with_height(viewport.height),
            ),
        );
        showcase_windows_with_desktop_state(
            &mut ui,
            desktop,
            self,
            &measurement_desktop,
            desktop_size,
            &theme,
        );
        ui
    }

    fn update(&mut self, action: WidgetAction) {
        let WidgetAction { binding, kind, .. } = action;
        let WidgetActionBinding::Action(action_id) = binding else {
            return;
        };
        let action_id = action_id.as_str();

        let color_outcome = self.color.apply_action(
            action_id,
            kind.clone(),
            ext_widgets::ColorPickerActionOptions::new("color").copy_hex("color.copy_hex"),
        );
        if color_outcome.update.is_some()
            || color_outcome.effect.is_some()
            || color_outcome.mode_changed
        {
            if let Some(ext_widgets::ColorPickerEffect::CopyHex(hex)) = color_outcome.effect {
                self.copy_text_to_clipboard(&hex);
                self.color_copied_hex = Some(hex);
            }
            return;
        }
        let color_button_picker_outcome = self.color.apply_action(
            action_id,
            kind.clone(),
            ext_widgets::ColorPickerActionOptions::new("color.button_picker"),
        );
        if color_button_picker_outcome.update.is_some() || color_button_picker_outcome.mode_changed
        {
            return;
        }
        let slider_color_outcome = self.slider_trailing_picker.apply_action(
            action_id,
            kind.clone(),
            ext_widgets::ColorPickerActionOptions::new("slider.trailing_picker"),
        );
        if slider_color_outcome.update.is_some() || slider_color_outcome.mode_changed {
            return;
        }
        let slider_thumb_outcome = self.slider_thumb_picker.apply_action(
            action_id,
            kind.clone(),
            ext_widgets::ColorPickerActionOptions::new("slider.thumb_picker"),
        );
        if slider_thumb_outcome.update.is_some() || slider_thumb_outcome.mode_changed {
            return;
        }
        let styling_stroke_outcome = self.styling_stroke_picker.apply_action(
            action_id,
            kind.clone(),
            ext_widgets::ColorPickerActionOptions::new("styling.stroke_picker"),
        );
        if styling_stroke_outcome.update.is_some() || styling_stroke_outcome.mode_changed {
            self.styling.stroke = self.styling_stroke_picker.value();
            return;
        }
        let styling_fill_outcome = self.styling_fill_picker.apply_action(
            action_id,
            kind.clone(),
            ext_widgets::ColorPickerActionOptions::new("styling.fill_picker"),
        );
        if styling_fill_outcome.update.is_some() || styling_fill_outcome.mode_changed {
            self.styling.fill = self.styling_fill_picker.value();
            return;
        }
        let styling_shadow_outcome = self.styling_shadow_picker.apply_action(
            action_id,
            kind.clone(),
            ext_widgets::ColorPickerActionOptions::new("styling.shadow_picker"),
        );
        if styling_shadow_outcome.update.is_some() || styling_shadow_outcome.mode_changed {
            self.styling.shadow = self.styling_shadow_picker.value();
            return;
        }

        if action_id == "window.clear_all" {
            self.windows.clear_all();
            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                self.desktop.close(id);
            }
            self.command_palette_open = false;
            return;
        }
        if action_id == "window.add_all" {
            self.windows.open_all();
            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                self.desktop.ensure_window(id, window_defaults(id));
                self.desktop.bring_to_front(id);
            }
            self.reset_progress_loading();
            self.organize_open_windows();
            self.initial_organize_pending = false;
            return;
        }
        if action_id == "window.organize_open" {
            self.organize_open_windows();
            self.initial_organize_pending = false;
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.toggle.") {
            let visible = self.windows.toggle(id).unwrap_or(false);
            if visible {
                self.desktop.ensure_window(id, window_defaults(id));
                self.desktop.bring_to_front(id);
                if id == "progress" {
                    self.reset_progress_loading();
                }
            } else {
                self.desktop.close(id);
            }
            if id == "command_palette" {
                self.command_palette_open = visible;
            }
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.close.") {
            self.windows.close(id);
            self.desktop.close(id);
            if id == "command_palette" {
                self.command_palette_open = false;
            }
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.activate.") {
            self.desktop.bring_to_front(id);
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.drag.") {
            if let WidgetActionKind::PointerEdit(edit) = kind {
                self.desktop
                    .apply_drag(id, edit, default_window_position(id));
            }
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.resize.") {
            if let WidgetActionKind::PointerEdit(edit) = kind {
                self.desktop.apply_resize(id, edit, window_defaults(id));
            }
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.collapse.") {
            self.desktop.toggle_collapsed(id);
            return;
        }
        if let Some(id) = window_for_action(action_id) {
            self.desktop.bring_to_front(id);
        }
        if action_id == "runtime.tick" {
            self.progress_phase += SHOWCASE_PROGRESS_RADIANS_PER_SECOND / SHOWCASE_TICK_RATE_HZ;
            if self.windows.progress {
                self.progress_loading_elapsed = (self.progress_loading_elapsed
                    + 1.0 / SHOWCASE_TICK_RATE_HZ)
                    .min(PROGRESS_LOGGED_DURATION_SECONDS);
            }
            self.caret_phase = (self.caret_phase
                + std::f32::consts::TAU * TEXT_CARET_BLINK_HZ / SHOWCASE_TICK_RATE_HZ)
                % std::f32::consts::TAU;
            return;
        }
        if action_id == "progress.logged.reset" {
            self.reset_progress_loading();
            return;
        }
        if action_id == "command_palette.search" {
            if let WidgetActionKind::TextEdit(edit) = kind {
                self.apply_command_palette_event(edit.event);
            }
            return;
        }
        if action_id == "command_palette.open" {
            let items = command_palette_items_with_history(&self.command_history);
            self.command_palette.refresh_active_match(&items);
            self.command_palette_open = true;
            return;
        }
        if action_id == "command_palette.close" {
            self.command_palette_open = false;
            return;
        }
        if let Some(id) = action_id.strip_prefix("command_palette.item.") {
            self.select_command_palette_item(id);
            return;
        }
        if let Some(input) = focused_text_for_action(action_id) {
            match kind {
                WidgetActionKind::TextEdit(edit) => self.apply_text_edit(input, edit),
                WidgetActionKind::Focus(change) => self.apply_text_focus(input, change.focused),
                _ => {}
            }
            return;
        }
        if matches!(kind, WidgetActionKind::Focus(_)) {
            return;
        }
        if let Some(page) = action_id
            .strip_prefix("diagnostics.page.")
            .and_then(DiagnosticsPage::from_action_suffix)
        {
            self.diagnostics_page = page;
            return;
        }

        match action_id {
            "labels.link" => {
                self.label_link_visited = true;
                self.label_link_status = "Internal link activated";
                return;
            }
            "labels.hyperlink" => {
                self.label_hyperlink_visited = true;
                self.label_link_status = "Opened docs.rs/operad";
                self.platform.open_url("https://docs.rs/operad");
                return;
            }
            "button.default" => self.last_button = "Default",
            "button.primary" => self.last_button = "Primary",
            "button.secondary" => self.last_button = "Secondary",
            "button.destructive" => self.last_button = "Destructive",
            "button.small" => self.last_button = "Small",
            "button.icon" => self.last_button = "Settings",
            "button.image" => self.last_button = "Folder",
            "button.reset" => {
                self.toggle_button = false;
                self.last_button = "Reset";
            }
            "button.toggle" => {
                self.toggle_button = !self.toggle_button;
                self.last_button = "Toggle";
            }
            "checkbox.enabled"
            | "checkbox.large"
            | "checkbox.custom_color"
            | "checkbox.image_check"
            | "checkbox.compact_gap" => self.checked = !self.checked,
            "checkbox.indeterminate" => {
                self.checkbox_indeterminate = self.checkbox_indeterminate.next(true);
                return;
            }
            "labels.locale.toggle" => {
                self.label_locale.toggle(&label_locale_options());
                return;
            }
            "toggles.switch" => self.switch_enabled = !self.switch_enabled,
            "toggles.mixed" => self.mixed_switch = self.mixed_switch.toggled(),
            "toggles.radio.foo" => self.radio_choice = "foo",
            "toggles.radio.bar" => self.radio_choice = "bar",
            "toggles.radio.baz" => self.radio_choice = "baz",
            "toggles.theme.system" => {
                self.theme_preference = widgets::ThemePreference::System;
                return;
            }
            "toggles.theme.light" => {
                self.theme_preference = widgets::ThemePreference::Light;
                return;
            }
            "toggles.theme.dark" => {
                self.theme_preference = widgets::ThemePreference::Dark;
                return;
            }
            "theme.preference.dark" => {
                self.theme_preference = if self.theme_preference.is_dark() {
                    widgets::ThemePreference::Light
                } else {
                    widgets::ThemePreference::Dark
                };
                return;
            }
            "theme.demo.light" => {
                self.showcase_theme = ShowcaseThemeChoice::Light;
                return;
            }
            "theme.demo.dark" => {
                self.showcase_theme = ShowcaseThemeChoice::Dark;
                return;
            }
            "theme.demo.bubblegum" => {
                self.showcase_theme = ShowcaseThemeChoice::Bubblegum;
                return;
            }
            "selection.dropdown.toggle" => {
                self.dropdown.toggle(&select_options());
                return;
            }
            "numeric.unit.toggle" => {
                self.numeric_unit.toggle(&numeric_unit_options());
                return;
            }
            "menus.menu_button" => {
                let button_items = menu_items(self.menu_autosave);
                let outcome = self.menu_button.toggle(&button_items);
                if outcome.opened {
                    self.image_text_menu_button.close();
                    self.image_menu_button.close();
                    self.context_menu.close();
                }
                return;
            }
            "menus.image_text_menu_button" => {
                let button_items = menu_items(self.menu_autosave);
                let outcome = self.image_text_menu_button.toggle(&button_items);
                if outcome.opened {
                    self.menu_button.close();
                    self.image_menu_button.close();
                    self.context_menu.close();
                }
                return;
            }
            "menus.image_menu_button" => {
                let button_items = menu_items(self.menu_autosave);
                let outcome = self.image_menu_button.toggle(&button_items);
                if outcome.opened {
                    self.menu_button.close();
                    self.image_text_menu_button.close();
                    self.context_menu.close();
                }
                return;
            }
            "menus.context.open" => {
                self.context_menu
                    .open_with_items(menu_demo_context_anchor(), &menu_items(self.menu_autosave));
                self.menu_bar.close();
                self.menu_button.close();
                self.image_text_menu_button.close();
                self.image_menu_button.close();
                return;
            }
            "menus.context.close" => {
                self.context_menu.close();
                return;
            }
            "menus.bar.file" => {
                self.menu_bar
                    .open(&menu_bar_menus(self.menu_autosave, self.menu_grid), 0);
                return;
            }
            "menus.bar.edit" => {
                self.menu_bar
                    .open(&menu_bar_menus(self.menu_autosave, self.menu_grid), 1);
                return;
            }
            "menus.bar.view" => {
                self.menu_bar
                    .open(&menu_bar_menus(self.menu_autosave, self.menu_grid), 2);
                return;
            }
            "date.previous" => {
                self.date.show_previous_month();
                self.date_range.show_previous_month();
            }
            "date.next" => {
                self.date.show_next_month();
                self.date_range.show_next_month();
            }
            "date.week.sunday" => {
                self.date.first_weekday = ext_widgets::Weekday::Sunday;
                self.date_range.first_weekday = ext_widgets::Weekday::Sunday;
                self.refresh_date_week_range();
                return;
            }
            "date.week.monday" => {
                self.date.first_weekday = ext_widgets::Weekday::Monday;
                self.date_range.first_weekday = ext_widgets::Weekday::Monday;
                self.refresh_date_week_range();
                return;
            }
            "date.mode.single" => {
                self.date_mode = DateDemoMode::Single;
                return;
            }
            "date.mode.range" => {
                self.date_mode = DateDemoMode::Range;
                self.date_range.mode = ext_widgets::DateRangeSelectionMode::Custom;
                return;
            }
            "date.mode.week" => {
                self.date_mode = DateDemoMode::Week;
                self.date_range.mode = ext_widgets::DateRangeSelectionMode::Week;
                self.refresh_date_week_range();
                return;
            }
            "date.clear" => {
                self.date.selected = None;
                self.date_range.clear();
                return;
            }
            "date.bounds.toggle" | "date.range.toggle" => {
                if self.date.min.is_some() || self.date.max.is_some() {
                    self.date.min = None;
                    self.date.max = None;
                    self.date_range.min = None;
                    self.date_range.max = None;
                } else {
                    self.date.min = CalendarDate::new(2026, 5, 4);
                    self.date.max = CalendarDate::new(2026, 5, 29);
                    self.date_range.min = CalendarDate::new(2026, 5, 4);
                    self.date_range.max = CalendarDate::new(2026, 5, 29);
                }
                return;
            }
            "toast.show" => {
                self.toast_visible = true;
                return;
            }
            "toast.hide" => {
                self.toast_visible = false;
                return;
            }
            id if id.starts_with("toast.dismiss.") => {
                self.toast_visible = false;
                return;
            }
            "toast.action.1.undo" => {
                self.toast_action_status = "Undo requested";
                return;
            }
            "layout.tab.preview" => {
                self.layout_tab = 0;
                return;
            }
            "layout.tab.settings" => {
                self.layout_tab = 1;
                return;
            }
            "forms.profile.submit" => {
                self.form.submit();
                self.form.apply();
                self.form.submitted = true;
                self.form_status =
                    "Submitted profile; changes are saved and the submission flag is set."
                        .to_string();
                return;
            }
            "forms.profile.apply" => {
                self.form.apply();
                self.form.submitted = false;
                self.form_status =
                    "Applied changes; draft is saved but the profile is not submitted.".to_string();
                return;
            }
            "forms.profile.cancel" => {
                self.form.cancel();
                self.sync_profile_form_text_fields();
                self.form_status = "Cancelled".to_string();
                return;
            }
            "forms.profile.reset" => {
                self.form = profile_form_state();
                self.form_newsletter = true;
                self.sync_profile_form_text_fields();
                self.form_status = "Reset".to_string();
                return;
            }
            "forms.profile.newsletter.toggle" => {
                self.form_newsletter = !self.form_newsletter;
                let _ = self.form.update_field(
                    "newsletter",
                    if self.form_newsletter {
                        "true"
                    } else {
                        "false"
                    },
                );
                self.validate_profile_form();
                self.form_status = "Editing profile".to_string();
                return;
            }
            "overlays.collapsing.toggle" => {
                self.overlay_expanded = !self.overlay_expanded;
                return;
            }
            "overlays.popup.toggle" => {
                self.overlay_popup_open = !self.overlay_popup_open;
                return;
            }
            "overlays.popup.close" => {
                self.overlay_popup_open = false;
                return;
            }
            "overlays.modal.open" => {
                self.overlay_modal_open = true;
                return;
            }
            "overlays.modal.close" => {
                self.overlay_modal_open = false;
                return;
            }
            "drag_drop.text_source" => {
                self.apply_drag_drop_source_action(
                    DragDropDemoPayload::Text,
                    "Text drag started",
                    "Text dragging",
                    "Text drag finished",
                    "Text drag canceled",
                    &kind,
                );
                return;
            }
            "drag_drop.file_source" => {
                self.apply_drag_drop_source_action(
                    DragDropDemoPayload::File,
                    "File drag started",
                    "File dragging",
                    "File drag finished",
                    "File drag canceled",
                    &kind,
                );
                return;
            }
            "drag_drop.bytes_source" => {
                self.apply_drag_drop_source_action(
                    DragDropDemoPayload::ImageBytes,
                    "Image byte drag started",
                    "Image bytes dragging",
                    "Image byte drag finished",
                    "Image byte drag canceled",
                    &kind,
                );
                return;
            }
            "drag_drop.accept_text" => {
                self.apply_drag_drop_target_action(DragDropDemoTarget::Text, &kind);
                return;
            }
            "drag_drop.files_only" => {
                self.apply_drag_drop_target_action(DragDropDemoTarget::FilesOnly, &kind);
                return;
            }
            "drag_drop.image_bytes" => {
                self.apply_drag_drop_target_action(DragDropDemoTarget::ImageBytes, &kind);
                return;
            }
            "drag_drop.disabled" => {
                self.apply_drag_drop_target_action(DragDropDemoTarget::Disabled, &kind);
                return;
            }
            "slider.trailing" => {
                self.slider_trailing_color = !self.slider_trailing_color;
                return;
            }
            "slider.trailing_color_button" => {
                self.slider_trailing_picker_open = !self.slider_trailing_picker_open;
                return;
            }
            "slider.thumb_color_button" => {
                self.slider_thumb_picker_open = !self.slider_thumb_picker_open;
                return;
            }
            "slider.thumb.circle" => {
                self.slider_thumb_shape = SliderThumbChoice::Circle;
                return;
            }
            "slider.thumb.square" => {
                self.slider_thumb_shape = SliderThumbChoice::Square;
                return;
            }
            "slider.thumb.rectangle" => {
                self.slider_thumb_shape = SliderThumbChoice::Rectangle;
                return;
            }
            "slider.steps" => {
                self.slider_use_steps = !self.slider_use_steps;
                if self.slider_use_steps {
                    self.set_slider_value(widgets::slider::round_slider_to_step(
                        self.slider,
                        self.slider_step(),
                    ));
                }
                return;
            }
            "slider.logarithmic" => {
                self.slider_logarithmic = !self.slider_logarithmic;
                return;
            }
            "slider.clamping.never" => {
                self.slider_clamping = widgets::SliderClamping::Never;
                return;
            }
            "slider.clamping.edits" => {
                self.slider_clamping = widgets::SliderClamping::Edits;
                return;
            }
            "slider.clamping.always" => {
                self.slider_clamping = widgets::SliderClamping::Always;
                self.clamp_slider_to_range();
                return;
            }
            "slider.smart_aim" => {
                self.slider_smart_aim = !self.slider_smart_aim;
                return;
            }
            "animation.open" => {
                self.animation_open = !self.animation_open;
                return;
            }
            "animation.timed.toggle" => {
                self.animation_timed_expanded = !self.animation_timed_expanded;
                return;
            }
            "easing.in.dropdown.toggle" => {
                self.easing_in.toggle(&easing_options(EaseDirection::In));
                return;
            }
            "easing.out.dropdown.toggle" => {
                self.easing_out.toggle(&easing_options(EaseDirection::Out));
                return;
            }
            "animation.scrub.toggle" => {
                self.animation_scrub_expanded = !self.animation_scrub_expanded;
                return;
            }
            "animation.state.toggle" => {
                self.animation_state_expanded = !self.animation_state_expanded;
                return;
            }
            "animation.interaction.toggle" => {
                self.animation_interaction_expanded = !self.animation_interaction_expanded;
                return;
            }
            "shader_lab.target.toggle" => {
                self.shader_lab_target_menu
                    .toggle(&shader_lab_target_options());
                return;
            }
            "shader_lab.preset.toggle" => {
                self.shader_lab_preset_menu
                    .toggle(&shader_lab_preset_options());
                return;
            }
            "shader_lab.material.shader.toggle" => {
                self.shader_lab_material_shader_menu
                    .toggle(&shader_lab_material_shader_options());
                return;
            }
            "shader_lab.material.shape.toggle" => {
                self.shader_lab_material_shape_menu
                    .toggle(&shader_lab_material_shape_options());
                return;
            }
            "shader_lab.material.geometry.toggle" => {
                self.shader_lab_material_geometry_menu
                    .toggle(&shader_lab_material_geometry_options());
                return;
            }
            "shader_lab.target.canvas" => {
                self.set_shader_lab_target(ShaderLabTarget::Canvas);
                return;
            }
            "shader_lab.target.frame" => {
                self.set_shader_lab_target(ShaderLabTarget::Frame);
                return;
            }
            "shader_lab.target.button" => {
                self.set_shader_lab_target(ShaderLabTarget::Button);
                return;
            }
            "shader_lab.preset.plasma" => {
                self.set_shader_lab_preset(ShaderLabPreset::Plasma);
                return;
            }
            "shader_lab.preset.rings" => {
                self.set_shader_lab_preset(ShaderLabPreset::Rings);
                return;
            }
            "shader_lab.preset.grid" => {
                self.set_shader_lab_preset(ShaderLabPreset::Grid);
                return;
            }
            "shader_lab.preset.vertex_warp" => {
                self.set_shader_lab_preset(ShaderLabPreset::VertexWarp);
                return;
            }
            "shader_lab.frame_text.toggle" => {
                self.shader_lab_show_frame_text = !self.shader_lab_show_frame_text;
                return;
            }
            "shader_lab.button_text.toggle" => {
                self.shader_lab_show_button_text = !self.shader_lab_show_button_text;
                return;
            }
            "shader_lab.surface.stroke" => {
                if let WidgetActionKind::PointerEdit(edit) = kind {
                    self.shader_lab_surface_stroke_width = scaled_slider(
                        edit.target_rect,
                        edit.position,
                        0.0,
                        SHADER_LAB_SURFACE_STROKE_MAX,
                    );
                }
                return;
            }
            "shader_lab.material.outset" => {
                if let WidgetActionKind::PointerEdit(edit) = kind {
                    self.shader_lab_material_outset = scaled_slider(
                        edit.target_rect,
                        edit.position,
                        0.0,
                        SHADER_LAB_MATERIAL_OUTSET_MAX,
                    );
                }
                return;
            }
            "shader_lab.surface.radius" => {
                if let WidgetActionKind::PointerEdit(edit) = kind {
                    self.shader_lab_surface_radius = scaled_slider(
                        edit.target_rect,
                        edit.position,
                        0.0,
                        SHADER_LAB_SURFACE_RADIUS_MAX,
                    );
                }
                return;
            }
            "animation.scrub" => {
                if let WidgetActionKind::PointerEdit(edit) = kind {
                    self.animation_scrub = scaled_slider(edit.target_rect, edit.position, 0.0, 1.0);
                }
                return;
            }
            "diagnostics.animation.controls.transport.pause_toggle" => {
                self.diagnostics_animation_paused = !self.diagnostics_animation_paused;
                return;
            }
            "diagnostics.animation.controls.transport.step" => {
                self.diagnostics_animation_paused = true;
                self.diagnostics_animation_scrub =
                    (self.diagnostics_animation_scrub + 1.0 / 12.0).min(1.0);
                return;
            }
            "diagnostics.animation.controls.transport.scrub" => {
                if let WidgetActionKind::PointerEdit(edit) = kind {
                    self.diagnostics_animation_scrub =
                        scaled_slider(edit.target_rect, edit.position, 0.0, 1.0);
                }
                return;
            }
            "diagnostics.animation.controls.input.active.toggle" => {
                self.diagnostics_animation_active = !self.diagnostics_animation_active;
                self.refresh_diagnostics_snapshot();
                return;
            }
            "diagnostics.animation.controls.input.hover.set" => {
                if let WidgetActionKind::PointerEdit(edit) = kind {
                    self.diagnostics_animation_hover =
                        scaled_slider(edit.target_rect, edit.position, 0.0, 1.0);
                    self.refresh_diagnostics_snapshot();
                }
                return;
            }
            "diagnostics.animation.controls.input.pulse.fire" => {
                self.diagnostics_animation_pulse_count =
                    self.diagnostics_animation_pulse_count.saturating_add(1);
                return;
            }
            "layout_widgets.float_panel_a" => {
                let panel = ext_widgets::DockPanelDescriptor::new(
                    "panel_a",
                    "Panel A",
                    ext_widgets::DockSide::Left,
                    200.0,
                );
                self.layout_dock
                    .float_panel(&panel, UiRect::new(20.0, 58.0, 236.0, 210.0));
                return;
            }
            "layout_widgets.dock_panel_a" => {
                let panel = ext_widgets::DockPanelDescriptor::new(
                    "panel_a",
                    "Panel A",
                    ext_widgets::DockSide::Left,
                    200.0,
                );
                self.layout_dock
                    .dock_panel(&panel, ext_widgets::DockSide::Left);
                return;
            }
            "layout_widgets.drawer.panel_a" => {
                self.layout_dock.toggle_panel_hidden("panel_a");
                return;
            }
            "layout_widgets.drawer.panel_b" => {
                self.layout_dock.toggle_panel_hidden("panel_b");
                return;
            }
            "layout_widgets.reorder.panel_b.before.panel_a" => {
                let mut panels = base_layout_dock_panels();
                self.layout_dock.apply_order_to_panels(&mut panels);
                let payload = ext_widgets::dock_workspace::dock_panel_drag_payload("panel_b");
                self.layout_dock.apply_reorder_to_panels(
                    &mut panels,
                    &payload,
                    "panel_a",
                    ext_widgets::DockPanelReorderPlacement::Before,
                );
                return;
            }
            "layout_widgets.reorder.panel_b.after.panel_a" => {
                let mut panels = base_layout_dock_panels();
                self.layout_dock.apply_order_to_panels(&mut panels);
                let payload = ext_widgets::dock_workspace::dock_panel_drag_payload("panel_b");
                self.layout_dock.apply_reorder_to_panels(
                    &mut panels,
                    &payload,
                    "panel_a",
                    ext_widgets::DockPanelReorderPlacement::After,
                );
                return;
            }
            "styling.stroke_color_button" => {
                self.styling_stroke_picker_open = !self.styling_stroke_picker_open;
                return;
            }
            "styling.fill_color_button" => {
                self.styling_fill_picker_open = !self.styling_fill_picker_open;
                return;
            }
            "styling.shadow_color_button" => {
                self.styling_shadow_picker_open = !self.styling_shadow_picker_open;
                return;
            }
            "styling.inner_same" => {
                self.styling.inner_same = !self.styling.inner_same;
                return;
            }
            "styling.outer_same" => {
                self.styling.outer_same = !self.styling.outer_same;
                return;
            }
            "styling.radius_same" => {
                self.styling.radius_same = !self.styling.radius_same;
                return;
            }
            "canvas.grow_horizontal" => {
                self.canvas_grow_horizontal = !self.canvas_grow_horizontal;
                return;
            }
            "canvas.grow_vertical" => {
                self.canvas_grow_vertical = !self.canvas_grow_vertical;
                return;
            }
            "canvas.keep_aspect_ratio" => {
                self.canvas_keep_aspect_ratio = !self.canvas_keep_aspect_ratio;
                return;
            }
            _ => {}
        }

        if action_id == "canvas.rotate" {
            if let WidgetActionKind::Drag(drag) = kind {
                self.cube.apply_drag(drag);
            }
            return;
        }
        if let WidgetActionKind::Scroll(scroll) = &kind {
            match action_id {
                "lists_tables.scroll_area.scroll" => self.list_scroll = scroll.offset().y,
                "lists_tables.virtual_list.scroll" => self.virtual_scroll = scroll.offset().y,
                "lists_tables.data_table.scroll" => self.table_scroll = scroll.offset().y,
                "lists_tables.virtualized_table.scroll" => {
                    self.virtual_table_scroll = scroll.offset().y
                }
                "layout.panel_a.scroll" => self.layout_panel_a_scroll = scroll.offset().y,
                "layout.panel_b.scroll" => self.layout_panel_b_scroll = scroll.offset().y,
                "layout.workspace.scroll" => self.layout_workspace_scroll = scroll.offset().y,
                "trees.virtual.scroll" => self.tree_virtual_scroll = scroll.offset().y,
                "trees.table.scroll" => self.tree_table_scroll = scroll.offset().y,
                "containers.scroll_area_with_bars.scroll" => {
                    self.containers_scroll.set_offset(scroll.offset());
                }
                "progress.logged.logs.scroll" => {
                    self.progress_logs_scroll = *scroll;
                    self.progress_logs_scroll.set_offset(scroll.offset());
                    self.progress_logs_follow_tail =
                        scroll.offset().y >= scroll.max_offset().y - 0.5;
                }
                "timeline.scroll" => {
                    self.timeline_scroll = *scroll;
                    self.timeline_scroll.set_offset(scroll.offset());
                }
                "shader_lab.editor.scroll" => self.shader_lab_editor_scroll = scroll.offset(),
                "controls.widget_list.scroll" => {
                    self.controls_scroll = *scroll;
                    self.controls_scroll.set_offset(scroll.offset());
                }
                _ => {}
            }
            return;
        }

        if let Some(date) = action_id
            .strip_prefix("date.day.")
            .and_then(parse_calendar_date)
        {
            match self.date_mode {
                DateDemoMode::Single => {
                    self.date.select(date);
                    self.date_range.show_month(date.month());
                }
                DateDemoMode::Range | DateDemoMode::Week => {
                    self.date_range.select(date);
                    self.date.show_month(date.month());
                }
            }
            return;
        }

        if let Some(option_id) = action_id.strip_prefix("labels.locale.option.") {
            self.label_locale
                .select_id_and_close(&label_locale_options(), option_id);
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("selection.dropdown.option.") {
            self.dropdown
                .select_id_and_close(&select_options(), option_id);
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("selection.menu.option.") {
            self.select_menu.select_id(&select_options(), option_id);
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("selection.image_menu.option.") {
            self.image_select_menu
                .select_id(&select_options_with_images(), option_id);
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("numeric.unit.option.") {
            self.numeric_unit
                .select_id_and_close(&numeric_unit_options(), option_id);
            self.reset_numeric_range_for_unit();
            self.set_numeric_value(self.numeric_value, true);
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("easing.in.dropdown.option.") {
            self.easing_in
                .select_id_and_close(&easing_options(EaseDirection::In), option_id);
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("easing.out.dropdown.option.") {
            self.easing_out
                .select_id_and_close(&easing_options(EaseDirection::Out), option_id);
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("shader_lab.target.option.") {
            if let Some(target) = ShaderLabTarget::from_id(option_id) {
                self.shader_lab_target_menu
                    .select_id_and_close(&shader_lab_target_options(), option_id);
                self.shader_lab_target = target;
            }
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("shader_lab.preset.option.") {
            if let Some(preset) = ShaderLabPreset::from_id(option_id) {
                self.shader_lab_preset_menu
                    .select_id_and_close(&shader_lab_preset_options(), option_id);
                self.shader_lab_preset = preset;
                self.shader_lab_source.set_text(preset.source());
                self.refresh_shader_lab_validation();
            }
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("shader_lab.material.shader.option.") {
            if let Some(shader) = ShaderLabMaterialShader::from_id(option_id) {
                self.shader_lab_material_shader_menu
                    .select_id_and_close(&shader_lab_material_shader_options(), option_id);
                self.shader_lab_material_shader = shader;
            }
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("shader_lab.material.shape.option.") {
            if let Some(shape) = ShaderLabMaterialShape::from_id(option_id) {
                self.shader_lab_material_shape_menu
                    .select_id_and_close(&shader_lab_material_shape_options(), option_id);
                self.shader_lab_material_shape = shape;
            }
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("shader_lab.material.geometry.option.") {
            if let Some(geometry) = ShaderLabMaterialGeometry::from_id(option_id) {
                self.shader_lab_material_geometry_menu
                    .select_id_and_close(&shader_lab_material_geometry_options(), option_id);
                self.shader_lab_material_geometry = geometry;
            }
            return;
        }
        if let Some(menu_id) = action_id.strip_prefix("menus.item.") {
            self.apply_menu_item(menu_id);
            return;
        }
        if let Some(menu_id) = action_id.strip_prefix("menus.context.") {
            self.apply_menu_item(menu_id);
            self.context_menu.close();
            return;
        }
        if action_id == "color.button.open" {
            self.color_picker_button_open = !self.color_picker_button_open;
            return;
        }
        if let Some(row) = action_id
            .strip_prefix("lists_tables.data_table.row.")
            .and_then(|row| row.parse::<usize>().ok())
        {
            self.table_selection = ext_widgets::DataTableSelection::single_row(row)
                .with_active_cell(ext_widgets::DataTableCellIndex::new(row, 0));
            return;
        }
        if let Some(cell) = action_id
            .strip_prefix("lists_tables.data_table.cell.")
            .and_then(parse_table_cell)
        {
            self.table_selection =
                ext_widgets::DataTableSelection::single_row(cell.row).with_active_cell(cell);
            return;
        }
        match action_id {
            "lists_tables.virtualized_table.sort.name" => {
                self.virtual_table_descending = !self.virtual_table_descending;
                return;
            }
            "lists_tables.virtualized_table.filter.status" => {
                self.virtual_table_ready_only = !self.virtual_table_ready_only;
                self.virtual_table_scroll = 0.0;
                return;
            }
            "lists_tables.virtualized_table.resize.reset" => {
                self.virtual_table_value_width = 120.0;
                self.virtual_table_resize = None;
                return;
            }
            _ => {}
        }
        if let Some(row) = action_id
            .strip_prefix("lists_tables.virtualized_table.row.")
            .and_then(|row| row.parse::<usize>().ok())
        {
            self.table_selection = ext_widgets::DataTableSelection::single_row(row)
                .with_active_cell(ext_widgets::DataTableCellIndex::new(row, 0));
            return;
        }
        if let Some(cell) = action_id
            .strip_prefix("lists_tables.virtualized_table.cell.")
            .and_then(parse_table_cell)
        {
            self.table_selection =
                ext_widgets::DataTableSelection::single_row(cell.row).with_active_cell(cell);
            return;
        }
        if let Some(rest) = action_id.strip_prefix("trees.tree.action.") {
            if let Some((id, action)) = rest.rsplit_once('.') {
                self.apply_editable_tree_action(id, action);
            }
            return;
        }
        if let Some(id) = action_id.strip_prefix("trees.tree.row.") {
            self.apply_tree_row(id, false);
            return;
        }
        if let Some(id) = action_id.strip_prefix("trees.outliner.row.") {
            self.apply_tree_row(id, true);
            return;
        }
        if let Some(id) = action_id.strip_prefix("trees.virtual.row.") {
            self.apply_virtual_tree_row(id);
            return;
        }
        if let Some(row) = action_id
            .strip_prefix("trees.table.row.")
            .and_then(|row| row.parse::<usize>().ok())
        {
            self.apply_tree_table_row(row);
            return;
        }
        if let Some(cell) = action_id
            .strip_prefix("trees.table.cell.")
            .and_then(parse_table_cell)
        {
            self.apply_tree_table_row(cell.row);
            return;
        }

        let WidgetActionKind::PointerEdit(edit) = kind else {
            return;
        };
        match action_id {
            "numeric.value.drag" => {
                self.apply_numeric_drag(edit);
            }
            "numeric.range_min" => {
                let domain = self.numeric_unit_domain();
                let min = domain.min as f32;
                let max = domain.max as f32;
                let span = self.numeric_minimum_span();
                self.set_numeric_range_min(scaled_slider(
                    edit.target_rect,
                    edit.position,
                    min,
                    (max - span).max(min),
                ));
            }
            "numeric.range_max" => {
                let domain = self.numeric_unit_domain();
                let min = domain.min as f32;
                let max = domain.max as f32;
                let span = self.numeric_minimum_span();
                self.set_numeric_range_max(scaled_slider(
                    edit.target_rect,
                    edit.position,
                    (min + span).min(max),
                    max,
                ));
            }
            "numeric.sensitivity" => {
                self.numeric_sensitivity =
                    scaled_slider(edit.target_rect, edit.position, 0.25, 4.0);
            }
            "layout_widgets.split_pane.handle" => {
                let total_extent = self
                    .desktop
                    .size("layout_widgets", default_window_size("layout_widgets"))
                    .width
                    - 48.0;
                let total_extent = total_extent.max(1.0);
                let handle_center = edit.target_rect.x + edit.target_rect.width * 0.5;
                self.layout_split
                    .resize_by(edit.position.x - handle_center, total_extent, 6.0);
            }
            "shader_lab.workspace.resize" => {
                resize_split_from_pointer(
                    &mut self.shader_lab_split,
                    ext_widgets::SplitAxis::Horizontal,
                    edit,
                    SHADER_LAB_SPLIT_HANDLE_THICKNESS,
                );
            }
            "panels.resize.top" => {
                resize_split_from_pointer(
                    &mut self.panels_top_split,
                    ext_widgets::SplitAxis::Vertical,
                    edit,
                    PANELS_SPLIT_HANDLE_THICKNESS,
                );
            }
            "panels.resize.bottom" => {
                resize_split_from_pointer(
                    &mut self.panels_bottom_split,
                    ext_widgets::SplitAxis::Vertical,
                    edit,
                    PANELS_SPLIT_HANDLE_THICKNESS,
                );
            }
            "panels.resize.left" => {
                resize_split_from_pointer(
                    &mut self.panels_left_split,
                    ext_widgets::SplitAxis::Horizontal,
                    edit,
                    PANELS_SPLIT_HANDLE_THICKNESS,
                );
            }
            "panels.resize.right" => {
                resize_split_from_pointer(
                    &mut self.panels_right_split,
                    ext_widgets::SplitAxis::Horizontal,
                    edit,
                    PANELS_SPLIT_HANDLE_THICKNESS,
                );
            }
            "slider.value" => {
                self.set_slider_value(
                    self.slider_value_spec()
                        .value_from_control_point(edit.target_rect, edit.position),
                );
            }
            "slider.range_left" => {
                let value = widgets::slider::SliderValueSpec::new(0.0, self.slider_right.max(1.0))
                    .value_from_control_point(edit.target_rect, edit.position);
                self.set_slider_left(value.min(self.slider_right - 1.0));
            }
            "slider.range_right" => {
                let value = widgets::slider::SliderValueSpec::new(self.slider_left + 1.0, 10000.0)
                    .value_from_control_point(edit.target_rect, edit.position);
                self.set_slider_right(value.max(self.slider_left + 1.0));
            }
            "lists_tables.virtualized_table.resize.value" => match edit.phase.edit_phase() {
                EditPhase::Preview => {}
                EditPhase::BeginEdit => {
                    self.virtual_table_resize =
                        Some((self.virtual_table_value_width, edit.position.x));
                }
                EditPhase::UpdateEdit | EditPhase::CommitEdit => {
                    let (origin_width, origin_x) = self
                        .virtual_table_resize
                        .unwrap_or((self.virtual_table_value_width, edit.position.x));
                    self.virtual_table_value_width =
                        (origin_width + edit.position.x - origin_x).clamp(56.0, 180.0);
                    if edit.phase.edit_phase() == EditPhase::CommitEdit {
                        self.virtual_table_resize = None;
                    }
                }
                EditPhase::CancelEdit => {
                    if let Some((origin_width, _)) = self.virtual_table_resize.take() {
                        self.virtual_table_value_width = origin_width;
                    }
                }
            },
            "containers.scroll_area_with_bars.vertical-scrollbar" => {
                let offset = self.scrollbars.apply_drag_for_target_rect(
                    "containers.vertical",
                    self.containers_scroll,
                    scrollbar_widgets::ScrollAxis::Vertical,
                    edit,
                );
                self.containers_scroll.set_offset(offset);
            }
            "containers.scroll_area_with_bars.horizontal-scrollbar" => {
                let offset = self.scrollbars.apply_drag_for_target_rect(
                    "containers.horizontal",
                    self.containers_scroll,
                    scrollbar_widgets::ScrollAxis::Horizontal,
                    edit,
                );
                self.containers_scroll.set_offset(offset);
            }
            "timeline.horizontal-scrollbar" => {
                let mut scroll =
                    timeline_scroll_state_for_view(self.timeline_scroll, edit.target_rect.width);
                let offset = self.scrollbars.apply_drag_for_target_rect(
                    "timeline",
                    scroll,
                    scrollbar_widgets::ScrollAxis::Horizontal,
                    edit,
                );
                scroll.set_offset(offset);
                self.timeline_scroll = scroll;
            }
            "controls.widget_list.scrollbar" => {
                let mut scroll =
                    controls_scroll_state_for_view(self.controls_scroll, edit.target_rect.height);
                let offset = self.scrollbars.apply_drag_for_target_rect(
                    "controls.widget_list",
                    scroll,
                    scrollbar_widgets::ScrollAxis::Vertical,
                    edit,
                );
                scroll.set_offset(offset);
                self.controls_scroll = scroll;
            }
            "styling.inner" => {
                self.styling.inner_margin =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 32.0);
                if self.styling.inner_same {
                    self.styling.inner_right = self.styling.inner_margin;
                    self.styling.inner_top = self.styling.inner_margin;
                    self.styling.inner_bottom = self.styling.inner_margin;
                }
            }
            "styling.inner_right" => {
                self.styling.inner_right =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 32.0);
            }
            "styling.inner_top" => {
                self.styling.inner_top = scaled_slider(edit.target_rect, edit.position, 0.0, 32.0);
            }
            "styling.inner_bottom" => {
                self.styling.inner_bottom =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 32.0);
            }
            "styling.outer" => {
                self.styling.outer_margin =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 40.0);
                if self.styling.outer_same {
                    self.styling.outer_right = self.styling.outer_margin;
                    self.styling.outer_top = self.styling.outer_margin;
                    self.styling.outer_bottom = self.styling.outer_margin;
                }
            }
            "styling.outer_right" => {
                self.styling.outer_right =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 40.0);
            }
            "styling.outer_top" => {
                self.styling.outer_top = scaled_slider(edit.target_rect, edit.position, 0.0, 40.0);
            }
            "styling.outer_bottom" => {
                self.styling.outer_bottom =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 40.0);
            }
            "styling.radius" => {
                self.styling.corner_radius =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 28.0);
                if self.styling.radius_same {
                    self.styling.corner_ne = self.styling.corner_radius;
                    self.styling.corner_sw = self.styling.corner_radius;
                    self.styling.corner_se = self.styling.corner_radius;
                }
            }
            "styling.radius_ne" => {
                self.styling.corner_ne = scaled_slider(edit.target_rect, edit.position, 0.0, 28.0);
            }
            "styling.radius_sw" => {
                self.styling.corner_sw = scaled_slider(edit.target_rect, edit.position, 0.0, 28.0);
            }
            "styling.radius_se" => {
                self.styling.corner_se = scaled_slider(edit.target_rect, edit.position, 0.0, 28.0);
            }
            "styling.shadow_x" => {
                self.styling.shadow_x = scaled_slider(edit.target_rect, edit.position, -24.0, 24.0);
            }
            "styling.shadow_y" => {
                self.styling.shadow_y = scaled_slider(edit.target_rect, edit.position, -24.0, 24.0);
            }
            "styling.shadow" => {
                self.styling.shadow_blur =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 32.0);
            }
            "styling.shadow_spread" => {
                self.styling.shadow_spread =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 16.0);
            }
            "styling.stroke" => {
                self.styling.stroke_width =
                    scaled_slider(edit.target_rect, edit.position, 0.0, STYLING_STROKE_MAX);
            }
            _ => {}
        }
    }

    fn apply_command_palette_event(&mut self, event: operad::UiInputEvent) {
        let items = command_palette_items_with_history(&self.command_history);
        let outcome = self.command_palette.handle_event(&items, &event);
        if let Some(selection) = outcome.selected {
            self.select_command_palette_item(&selection.id);
        }
    }

    fn select_command_palette_item(&mut self, id: &str) {
        if let Some(item) = command_palette_items_with_history(&self.command_history)
            .into_iter()
            .find(|item| item.id == id && item.enabled)
        {
            self.command_history.record(item.id.as_str());
            self.last_command = item.title;
            let items = command_palette_items_with_history(&self.command_history);
            self.command_palette.set_query("", &items);
            self.command_palette_open = false;
        }
    }

    fn text_edit_options(&self, input: FocusedTextInput) -> TextInputOptions {
        let mut options = TextInputOptions::default();
        options.focused = self.focused_text == Some(input);
        options.caret_visible = caret_visible(self.caret_phase);
        match input {
            FocusedTextInput::Editable => {
                options.layout = LayoutStyle::new().with_width(300.0).with_height(36.0);
                options.text_style = text(13.0, color(230, 236, 246));
                options.placeholder_style = text(13.0, color(144, 156, 174));
                options.placeholder = "Type here".to_string();
            }
            FocusedTextInput::Selectable => {
                options.layout = LayoutStyle::new().with_width(360.0).with_height(36.0);
                options.text_style = text(13.0, color(196, 210, 230));
                options.read_only = true;
                options.selectable = true;
            }
            FocusedTextInput::Singleline => {
                options.layout = LayoutStyle::new().with_width(300.0).with_height(34.0);
                options.text_style = text(13.0, color(230, 236, 246));
                options.placeholder = "Single line".to_string();
            }
            FocusedTextInput::Multiline => {
                options.layout = LayoutStyle::new().with_width(360.0).with_height(72.0);
                options.text_style = text(13.0, color(230, 236, 246));
            }
            FocusedTextInput::TextArea => {
                options.layout = LayoutStyle::new().with_width(360.0).with_height(66.0);
                options.text_style = text(13.0, color(230, 236, 246));
            }
            FocusedTextInput::CodeEditor => {
                options.layout = LayoutStyle::new().with_width(360.0).with_height(88.0);
                options.text_style = widgets::code_text_style();
            }
            FocusedTextInput::Search => {
                options.layout = LayoutStyle::new().with_width(300.0).with_height(34.0);
                options.text_style = text(13.0, color(230, 236, 246));
                options.placeholder = "Search".to_string();
            }
            FocusedTextInput::Password => {
                options.layout = LayoutStyle::new().with_width(300.0).with_height(34.0);
                options.text_style = text(13.0, color(230, 236, 246));
                options.placeholder = "Password".to_string();
            }
            FocusedTextInput::FormName
            | FocusedTextInput::FormEmail
            | FocusedTextInput::FormRole => {
                options.layout = LayoutStyle::new().with_width_percent(1.0).with_height(30.0);
                options.text_style = text(12.0, color(230, 236, 246));
                options.placeholder_style = text(12.0, color(144, 156, 174));
                options.placeholder = "Required".to_string();
            }
            FocusedTextInput::NumericValue => {
                options.layout = LayoutStyle::new()
                    .with_width(112.0)
                    .with_height(30.0)
                    .with_flex_shrink(0.0);
                options.text_style = text(12.0, color(230, 236, 246));
                options.placeholder_style = text(12.0, color(144, 156, 174));
                options.accessibility_label = Some("Numeric value".to_string());
            }
            FocusedTextInput::NumericRangeMin | FocusedTextInput::NumericRangeMax => {
                options.layout = LayoutStyle::new()
                    .with_width(70.0)
                    .with_height(28.0)
                    .with_flex_shrink(0.0);
                options.text_style = text(12.0, color(230, 236, 246));
                options.placeholder_style = text(12.0, color(144, 156, 174));
                options.accessibility_label = Some(
                    match input {
                        FocusedTextInput::NumericRangeMin => "Numeric range minimum",
                        _ => "Numeric range maximum",
                    }
                    .to_string(),
                );
            }
            FocusedTextInput::SliderValue | FocusedTextInput::SliderStep => {
                options.layout = LayoutStyle::new().with_width(86.0).with_height(28.0);
                options.text_style = text(12.0, color(230, 236, 246));
                options.placeholder_style = text(12.0, color(144, 156, 174));
            }
            FocusedTextInput::SliderRangeLeft | FocusedTextInput::SliderRangeRight => {
                options.layout = LayoutStyle::new().with_width(96.0).with_height(28.0);
                options.text_style = text(12.0, color(230, 236, 246));
                options.placeholder_style = text(12.0, color(144, 156, 174));
            }
            FocusedTextInput::ShaderLabSource => {
                let editor_content_size =
                    shader_lab_editor_content_size(self.shader_lab_source.text());
                options.layout = LayoutStyle::new()
                    .with_width(editor_content_size.width)
                    .with_height(editor_content_size.height)
                    .with_flex_shrink(0.0);
                options.text_style = widgets::code_text_style();
                options.placeholder_style = text(12.0, color(144, 156, 174));
                options.placeholder = "WGSL shader source".to_string();
                options.accessibility_label = Some("Shader source editor".to_string());
            }
        }
        options
    }

    fn apply_text_edit(&mut self, input: FocusedTextInput, edit: WidgetTextEdit) {
        self.set_focused_text(Some(input));
        let options = self.text_edit_options(input);
        let outcome = self.text_state_mut(input).map(|state| {
            state.set_multiline(input.is_multiline());
            state.apply_widget_text_edit(&edit, &options)
        });
        if let Some(outcome) = outcome {
            self.sync_text_input_value(input, outcome.committed, outcome.canceled);
            self.apply_text_clipboard_outcome(input, outcome);
            if input == FocusedTextInput::ShaderLabSource {
                self.refresh_shader_lab_validation();
            }
        }
    }

    fn apply_text_focus(&mut self, input: FocusedTextInput, focused: bool) {
        if focused {
            self.set_focused_text(Some(input));
        } else if self.focused_text == Some(input) {
            self.sync_text_input_value(input, true, false);
            self.set_focused_text(None);
        }
    }

    fn set_focused_text(&mut self, next: Option<FocusedTextInput>) {
        if self.focused_text != next {
            if let Some(previous) = self.focused_text {
                if let Some(state) = self.text_state_mut(previous) {
                    state.clear_selection();
                }
            }
        }
        self.focused_text = next;
    }

    fn apply_text_clipboard_outcome(
        &mut self,
        input: FocusedTextInput,
        outcome: widgets::text_input::TextInputOutcome,
    ) {
        match outcome.clipboard {
            Some(widgets::text_input::TextInputClipboardAction::Copy(text))
            | Some(widgets::text_input::TextInputClipboardAction::Cut(text)) => {
                self.copy_text_to_clipboard(&text);
            }
            Some(widgets::text_input::TextInputClipboardAction::Paste) => {
                self.pending_clipboard_paste = Some(input);
                self.platform.read_clipboard_text();
            }
            None => {}
        }
    }

    fn text_state_mut(&mut self, input: FocusedTextInput) -> Option<&mut TextInputState> {
        match input {
            FocusedTextInput::Editable => Some(&mut self.text),
            FocusedTextInput::Selectable => Some(&mut self.selectable_text),
            FocusedTextInput::Singleline => Some(&mut self.singleline_text),
            FocusedTextInput::Multiline => Some(&mut self.multiline_text),
            FocusedTextInput::TextArea => Some(&mut self.text_area_text),
            FocusedTextInput::CodeEditor => Some(&mut self.code_editor_text),
            FocusedTextInput::Search => Some(&mut self.search_text),
            FocusedTextInput::Password => Some(&mut self.password_text),
            FocusedTextInput::FormName => Some(&mut self.form_name_text),
            FocusedTextInput::FormEmail => Some(&mut self.form_email_text),
            FocusedTextInput::FormRole => Some(&mut self.form_role_text),
            FocusedTextInput::NumericValue => Some(&mut self.numeric_text),
            FocusedTextInput::NumericRangeMin => Some(&mut self.numeric_range_min_text),
            FocusedTextInput::NumericRangeMax => Some(&mut self.numeric_range_max_text),
            FocusedTextInput::SliderValue => Some(&mut self.slider_value_text),
            FocusedTextInput::SliderRangeLeft => Some(&mut self.slider_left_text),
            FocusedTextInput::SliderRangeRight => Some(&mut self.slider_right_text),
            FocusedTextInput::SliderStep => Some(&mut self.slider_step_text),
            FocusedTextInput::ShaderLabSource => Some(&mut self.shader_lab_source),
        }
    }

    fn sync_text_input_value(&mut self, input: FocusedTextInput, committed: bool, canceled: bool) {
        match input {
            FocusedTextInput::SliderValue => {
                if let Ok(value) = self.slider_value_text.text().parse::<f32>() {
                    self.apply_slider_value_from_text(value);
                }
            }
            FocusedTextInput::SliderRangeLeft => {
                if let Ok(value) = self.slider_left_text.text().parse::<f32>() {
                    self.apply_slider_left_from_text(value);
                }
            }
            FocusedTextInput::SliderRangeRight => {
                if let Ok(value) = self.slider_right_text.text().parse::<f32>() {
                    self.apply_slider_right_from_text(value);
                }
            }
            FocusedTextInput::SliderStep => {
                if let Ok(value) = self.slider_step_text.text().parse::<f32>() {
                    self.slider_step_value = value.abs().max(0.0001);
                    if self.slider_use_steps {
                        self.set_slider_value(widgets::slider::round_slider_to_step(
                            self.slider,
                            self.slider_step(),
                        ));
                    }
                }
            }
            FocusedTextInput::NumericValue => {
                self.sync_numeric_value_from_text(committed, canceled);
            }
            FocusedTextInput::NumericRangeMin => {
                self.sync_numeric_range_min_from_text(committed, canceled);
            }
            FocusedTextInput::NumericRangeMax => {
                self.sync_numeric_range_max_from_text(committed, canceled);
            }
            FocusedTextInput::FormName => {
                self.update_profile_form_field("name", self.form_name_text.text().to_string());
            }
            FocusedTextInput::FormEmail => {
                self.update_profile_form_field("email", self.form_email_text.text().to_string());
            }
            FocusedTextInput::FormRole => {
                self.update_profile_form_field("role", self.form_role_text.text().to_string());
            }
            _ => {}
        }
    }

    fn numeric_precision(&self) -> ext_widgets::NumericPrecision {
        match numeric_unit_id(&self.numeric_unit) {
            "turn" => ext_widgets::NumericPrecision::decimals(3).with_step(0.001),
            _ => ext_widgets::NumericPrecision::decimals(1).with_step(0.1),
        }
    }

    fn numeric_range(&self) -> ext_widgets::NumericRange {
        let span = self.numeric_minimum_span();
        ext_widgets::NumericRange::new(
            f64::from(self.numeric_range_min),
            f64::from(self.numeric_range_max.max(self.numeric_range_min + span)),
        )
    }

    fn formatted_numeric_value(&self) -> String {
        self.numeric_precision()
            .format(f64::from(self.numeric_value))
    }

    fn numeric_unit_domain(&self) -> ext_widgets::NumericRange {
        numeric_unit_default_range(numeric_unit_id(&self.numeric_unit))
    }

    fn numeric_minimum_span(&self) -> f32 {
        self.numeric_precision().step as f32
    }

    fn format_numeric_range_bound(&self, value: f32) -> String {
        self.numeric_precision().format(f64::from(value))
    }

    fn reset_progress_loading(&mut self) {
        self.progress_loading_elapsed = 0.0;
        self.progress_logs_follow_tail = true;
        self.progress_logs_scroll = progress_log_scroll_state(0.0, 0, true);
    }

    fn set_shader_lab_target(&mut self, target: ShaderLabTarget) {
        self.shader_lab_target = target;
        self.shader_lab_target_menu
            .select_id(&shader_lab_target_options(), target.id());
    }

    fn set_shader_lab_preset(&mut self, preset: ShaderLabPreset) {
        self.shader_lab_preset = preset;
        self.shader_lab_preset_menu
            .select_id(&shader_lab_preset_options(), preset.id());
        self.shader_lab_source.set_text(preset.source());
        self.refresh_shader_lab_validation();
    }

    fn refresh_shader_lab_validation(&mut self) {
        self.shader_lab_source_error = shader_lab_source_error(self.shader_lab_source.text());
    }

    fn set_numeric_value(&mut self, value: f32, sync_text: bool) {
        let range = self.numeric_range();
        self.numeric_value = self
            .numeric_precision()
            .quantize(range.clamp(f64::from(value))) as f32;
        if sync_text {
            self.sync_numeric_text_to_value();
        }
    }

    fn set_numeric_range_min(&mut self, value: f32) {
        let domain = self.numeric_unit_domain();
        let min_domain = domain.min as f32;
        let max_domain = domain.max as f32;
        let span = self.numeric_minimum_span();
        let max_allowed =
            (self.numeric_range_max - span).clamp(min_domain, (max_domain - span).max(min_domain));
        self.numeric_range_min = value.clamp(min_domain, max_allowed);
        self.numeric_range_min_text
            .set_text(self.format_numeric_range_bound(self.numeric_range_min));
        self.set_numeric_value(self.numeric_value, true);
    }

    fn set_numeric_range_max(&mut self, value: f32) {
        let domain = self.numeric_unit_domain();
        let min_domain = domain.min as f32;
        let max_domain = domain.max as f32;
        let span = self.numeric_minimum_span();
        let min_allowed = (self.numeric_range_min + span).clamp(min_domain + span, max_domain);
        self.numeric_range_max = value.clamp(min_allowed, max_domain);
        self.numeric_range_max_text
            .set_text(self.format_numeric_range_bound(self.numeric_range_max));
        self.set_numeric_value(self.numeric_value, true);
    }

    fn reset_numeric_range_for_unit(&mut self) {
        let range = self.numeric_unit_domain();
        self.numeric_range_min = range.min as f32;
        self.numeric_range_max = range.max as f32;
        self.numeric_range_min_text
            .set_text(self.format_numeric_range_bound(self.numeric_range_min));
        self.numeric_range_max_text
            .set_text(self.format_numeric_range_bound(self.numeric_range_max));
    }

    fn sync_numeric_text_to_value(&mut self) {
        self.numeric_text.set_text(self.formatted_numeric_value());
    }

    fn sync_numeric_value_from_text(&mut self, committed: bool, canceled: bool) {
        if canceled {
            self.sync_numeric_text_to_value();
            return;
        }
        if let Some(value) = parse_numeric_edit_text(
            self.numeric_text.text(),
            &numeric_unit_format(&self.numeric_unit),
        ) {
            self.set_numeric_value(value, false);
        }
        if committed {
            self.sync_numeric_text_to_value();
        }
    }

    fn sync_numeric_range_min_from_text(&mut self, committed: bool, canceled: bool) {
        if canceled {
            self.numeric_range_min_text
                .set_text(self.format_numeric_range_bound(self.numeric_range_min));
            return;
        }
        if let Ok(value) = self.numeric_range_min_text.text().trim().parse::<f32>() {
            let raw = self.numeric_range_min_text.text().to_string();
            self.set_numeric_range_min(value);
            if !committed {
                self.numeric_range_min_text.set_text(raw);
            }
        }
        if committed {
            self.numeric_range_min_text
                .set_text(self.format_numeric_range_bound(self.numeric_range_min));
        }
    }

    fn sync_numeric_range_max_from_text(&mut self, committed: bool, canceled: bool) {
        if canceled {
            self.numeric_range_max_text
                .set_text(self.format_numeric_range_bound(self.numeric_range_max));
            return;
        }
        if let Ok(value) = self.numeric_range_max_text.text().trim().parse::<f32>() {
            let raw = self.numeric_range_max_text.text().to_string();
            self.set_numeric_range_max(value);
            if !committed {
                self.numeric_range_max_text.set_text(raw);
            }
        }
        if committed {
            self.numeric_range_max_text
                .set_text(self.format_numeric_range_bound(self.numeric_range_max));
        }
    }

    fn apply_numeric_drag(&mut self, edit: WidgetPointerEdit) {
        let phase = edit.phase.edit_phase();
        if phase == EditPhase::CommitEdit && self.numeric_drag_start.is_none() {
            self.set_focused_text(Some(FocusedTextInput::NumericValue));
            self.sync_numeric_text_to_value();
            return;
        }
        if phase == EditPhase::BeginEdit {
            self.numeric_drag_start = Some((self.numeric_value, edit.position.x));
            return;
        }
        let Some((start_value, start_x)) = self.numeric_drag_start else {
            return;
        };
        let precision = self.numeric_precision();
        let range = Some(self.numeric_range());
        let sensitivity = self.numeric_sensitivity.clamp(0.25, 4.0);
        let drag = ext_widgets::NumericDragSpec {
            pixels_per_step: (8.0 / sensitivity).clamp(1.0, 64.0),
            ..ext_widgets::NumericDragSpec::default()
        };
        let delta = edit.position.x - start_x;
        if phase == EditPhase::CommitEdit && delta.abs() < 1.0 {
            self.set_focused_text(Some(FocusedTextInput::NumericValue));
            self.sync_numeric_text_to_value();
            self.numeric_drag_start = None;
            return;
        }
        let value = ext_widgets::drag_value(
            f64::from(start_value),
            delta,
            precision,
            range,
            drag,
            ext_widgets::NumericDragSpeed::Normal,
        ) as f32;
        self.set_numeric_value(value, true);
        if matches!(phase, EditPhase::CommitEdit | EditPhase::CancelEdit) {
            if phase == EditPhase::CancelEdit {
                self.set_numeric_value(start_value, true);
            }
            self.numeric_drag_start = None;
        }
    }

    fn update_profile_form_field(&mut self, id: &'static str, value: String) {
        let _ = self.form.update_field(id, value);
        self.validate_profile_form();
        self.form_status = "Editing profile".to_string();
    }

    fn sync_profile_form_text_fields(&mut self) {
        self.form_name_text = TextInputState::new(profile_form_value(&self.form, "name"));
        self.form_email_text = TextInputState::new(profile_form_value(&self.form, "email"));
        self.form_role_text = TextInputState::new(profile_form_value(&self.form, "role"));
    }

    fn validate_profile_form(&mut self) {
        let request = self.form.begin_form_validation();
        let values = request.values.clone();
        let mut result = FormValidationResult::new(request.generation);
        let field_value = |id: &str| {
            values
                .iter()
                .find_map(|(field_id, value)| (field_id.as_str() == id).then_some(value.as_str()))
                .unwrap_or_default()
        };
        let name = field_value("name").trim();
        let email = field_value("email").trim();
        let role = field_value("role").trim();

        if name.is_empty() {
            result = result
                .with_field_messages("name", vec![ValidationMessage::error("Name is required")]);
        }
        if !profile_email_valid(email) {
            result = result.with_field_messages(
                "email",
                vec![ValidationMessage::error("Use a complete email address")],
            );
        }
        if role.is_empty() {
            result = result.with_field_messages(
                "role",
                vec![ValidationMessage::warning("Role can be added later")],
            );
        }
        if self.form.dirty {
            result =
                result.with_form_message(ValidationMessage::warning("Unsaved profile changes"));
        }
        let _ = self.form.apply_form_validation(result);
    }

    fn copy_text_to_clipboard(&mut self, text: &str) {
        self.clipboard_text = text.to_string();
        self.platform.write_clipboard_text(text);
    }

    fn apply_platform_responses(&mut self, responses: &[PlatformServiceResponse]) {
        self.platform.record_responses(responses.iter().cloned());
        for response in responses {
            match &response.response {
                PlatformResponse::Clipboard(ClipboardResponse::Text(text)) => {
                    let pasted = text
                        .as_deref()
                        .filter(|text| !text.is_empty())
                        .unwrap_or(&self.clipboard_text)
                        .to_string();
                    self.apply_pending_clipboard_paste(&pasted);
                }
                PlatformResponse::Clipboard(ClipboardResponse::Unsupported)
                | PlatformResponse::Clipboard(ClipboardResponse::Error(_)) => {
                    let pasted = self.clipboard_text.clone();
                    self.apply_pending_clipboard_paste(&pasted);
                }
                _ => {}
            }
        }
    }

    fn apply_pending_clipboard_paste(&mut self, pasted: &str) {
        let Some(input) = self.pending_clipboard_paste.take() else {
            return;
        };
        if input.is_read_only() {
            return;
        }
        if let Some(state) = self.text_state_mut(input) {
            state.paste_text(pasted);
        }
        self.sync_text_input_value(input, false, false);
    }

    fn apply_menu_item(&mut self, id: &str) {
        let menus = menu_bar_menus(self.menu_autosave, self.menu_grid);
        self.menu_bar.set_active_item_by_id(&menus, id);
        if id == "autosave" {
            self.menu_autosave = !self.menu_autosave;
        } else if id == "grid" {
            self.menu_grid = !self.menu_grid;
        }
        self.menu_button.close();
        self.image_text_menu_button.close();
        self.image_menu_button.close();
    }

    fn apply_tree_row(&mut self, id: &str, outliner: bool) {
        let roots = if outliner {
            tree_items()
        } else {
            editable_tree_items(&self.editable_tree)
        };
        let state = if outliner {
            &mut self.outliner
        } else {
            &mut self.tree
        };
        state.activate_visible_item_id(&roots, id);
    }

    fn apply_editable_tree_action(&mut self, id: &str, action: &str) {
        match action {
            "add" => {
                let new_id = format!("editable-{}", self.editable_tree_next_id);
                self.editable_tree_next_id += 1;
                if let Some(parent) = find_editable_tree_node_mut(&mut self.editable_tree, id) {
                    let label = format!("child #{}", parent.children.len());
                    parent
                        .children
                        .push(EditableTreeNode::new(new_id.clone(), label.clone()));
                    self.tree.set_expanded(id.to_owned(), true);
                    self.editable_tree_status = format!("Added {label} under {}", parent.label);
                }
            }
            "delete" => {
                if id == "root" {
                    return;
                }
                if let Some(label) = remove_editable_tree_node(&mut self.editable_tree, id) {
                    self.tree.select(None);
                    self.editable_tree_status = format!("Deleted {label}");
                }
            }
            _ => {}
        }
    }

    fn apply_virtual_tree_row(&mut self, id: &str) {
        let roots = virtual_tree_items();
        if self
            .tree_virtual
            .activate_visible_item_id(&roots, id)
            .is_some_and(|item| item.has_children())
        {
            self.tree_virtual_scroll = 0.0;
        }
    }

    fn apply_tree_table_row(&mut self, row: usize) {
        let roots = tree_table_items();
        let Some(item) = self.tree_table.visible_items(&roots).get(row).cloned() else {
            return;
        };
        if item.disabled {
            return;
        }
        self.tree_table.select(Some(item.index));
        if item.has_children() {
            self.tree_table.toggle_expanded(item.id);
            self.tree_table_scroll = 0.0;
        }
    }

    fn slider_value_spec(&self) -> widgets::slider::SliderValueSpec {
        let mut spec = widgets::slider::SliderValueSpec::new(self.slider_left, self.slider_right)
            .logarithmic(self.slider_logarithmic)
            .clamping(self.slider_clamping)
            .smart_aim(self.slider_smart_aim);
        if self.slider_use_steps {
            spec = spec.step(self.slider_step());
        }
        spec
    }

    fn set_slider_value(&mut self, value: f32) {
        let value = self.slider_value_spec().adjust_value(value);
        self.slider = value;
        self.slider_value_text
            .set_text(widgets::slider::format_slider_value(value));
    }

    fn apply_slider_value_from_text(&mut self, value: f32) {
        if self.slider_clamping == widgets::SliderClamping::Always {
            self.set_slider_value(value);
        } else {
            self.slider = value;
        }
    }

    fn set_slider_left(&mut self, value: f32) {
        self.slider_left = value.min(self.slider_right - 1.0).max(0.0);
        self.slider_left_text
            .set_text(widgets::slider::format_slider_value(self.slider_left));
        if self.slider_clamping == widgets::SliderClamping::Always {
            self.clamp_slider_to_range();
        }
    }

    fn apply_slider_left_from_text(&mut self, value: f32) {
        if value < self.slider_right {
            if self.slider_clamping == widgets::SliderClamping::Always {
                self.set_slider_left(value);
                return;
            }
            self.slider_left = value.max(0.0);
            if self.slider_clamping == widgets::SliderClamping::Always {
                self.slider = self.slider.clamp(self.slider_left, self.slider_right);
            }
        }
    }

    fn set_slider_right(&mut self, value: f32) {
        self.slider_right = value.max(self.slider_left + 1.0).min(10000.0);
        self.slider_right_text
            .set_text(widgets::slider::format_slider_value(self.slider_right));
        if self.slider_clamping == widgets::SliderClamping::Always {
            self.clamp_slider_to_range();
        }
    }

    fn apply_slider_right_from_text(&mut self, value: f32) {
        if value > self.slider_left {
            if self.slider_clamping == widgets::SliderClamping::Always {
                self.set_slider_right(value);
                return;
            }
            self.slider_right = value.min(10000.0);
            if self.slider_clamping == widgets::SliderClamping::Always {
                self.slider = self.slider.clamp(self.slider_left, self.slider_right);
            }
        }
    }

    fn clamp_slider_to_range(&mut self) {
        self.set_slider_value(self.slider.clamp(self.slider_left, self.slider_right));
    }

    fn slider_step(&self) -> f32 {
        self.slider_step_value.abs().max(0.0001)
    }

    fn refresh_diagnostics_snapshot(&mut self) {
        self.diagnostics_snapshot = diagnostics_sample_snapshot(self);
    }

    fn refresh_date_week_range(&mut self) {
        if self.date_mode != DateDemoMode::Week {
            return;
        }
        let anchor = self
            .date_range
            .range
            .map(|range| range.start)
            .or(self.date.selected)
            .or(self.date_range.today);
        if let Some(anchor) = anchor {
            self.date_range.mode = ext_widgets::DateRangeSelectionMode::Week;
            self.date_range.select(anchor);
        }
    }

    fn app_theme(&self) -> Theme {
        self.showcase_theme.theme()
    }

    fn view(&self, viewport: UiSize) -> UiDocument {
        set_showcase_active_theme(self.showcase_theme);
        let theme = self.app_theme();
        let mut ui = UiDocument::with_capacity(
            root_style(viewport.width, viewport.height),
            SHOWCASE_DOCUMENT_NODE_CAPACITY,
        );
        if let Some(update) = self.user_image_update.clone() {
            ui.add_resource_update(update);
        }
        ui.node_mut(ui.root())
            .set_visual(UiVisual::panel(theme.colors.canvas, None, 0.0));

        let root = ui.root();
        let shell = ui.add_child(
            root,
            UiNode::container(
                "showcase.shell",
                LayoutStyle::row().with_size(viewport.width, viewport.height),
            ),
        );
        let desktop_size = desktop_size_for_viewport(viewport);
        let desktop_width = desktop_size.width;
        let desktop = ui.add_child(
            shell,
            UiNode::container(
                "showcase.desktop",
                LayoutStyle::new()
                    .with_width(desktop_width)
                    .with_height(viewport.height)
                    .with_flex_shrink(1.0),
            )
            .with_visual(UiVisual::panel(theme.colors.canvas_subtle, None, 0.0)),
        );
        let controls = ui.add_child(
            shell,
            UiNode::container(
                "showcase.controls",
                LayoutStyle::column()
                    .with_width(RIGHT_PANEL_WIDTH)
                    .with_height(viewport.height)
                    .with_flex_shrink(0.0)
                    .padding(12.0)
                    .gap(4.0),
            )
            .with_visual(UiVisual::panel(
                theme.colors.surface,
                Some(theme.stroke.surface),
                0.0,
            )),
        );

        showcase_windows(&mut ui, desktop, self, desktop_size, &theme);
        organize_windows_button(&mut ui, desktop, &theme);
        fps_counter(&mut ui, desktop, self, viewport.height, &theme);
        control_panel(&mut ui, controls, self, viewport.height, &theme);

        ui
    }
}

fn organize_windows_button(ui: &mut UiDocument, desktop: UiNodeId, theme: &Theme) {
    let mut options = widgets::ButtonOptions::themed(
        theme,
        ComponentState::NORMAL,
        operad::layout::absolute(12.0, 12.0, 104.0, 28.0),
    )
    .with_action("window.organize_open")
    .with_accessibility_label("Organize open windows");
    options.text_style = themed_text(theme, 12.0);
    let button = widgets::button(
        ui,
        desktop,
        "showcase.organize_windows",
        "Organize",
        options,
    );
    ui.node_mut(button)
        .style_mut()
        .set_z_index(SHOWCASE_WINDOW_Z_MAX + 20.0);
}

fn fps_counter(
    ui: &mut UiDocument,
    desktop: UiNodeId,
    state: &ShowcaseState,
    viewport_height: f32,
    theme: &Theme,
) {
    let mut counter_style = UiNodeStyle::from(operad::layout::absolute(
        12.0,
        (viewport_height - 34.0).max(12.0),
        92.0,
        24.0,
    ));
    counter_style.set_z_index(SHOWCASE_WINDOW_Z_MAX + 16.0);
    let counter = ui.add_child(
        desktop,
        UiNode::container("showcase.fps", counter_style)
            .with_visual(UiVisual::panel(
                theme.colors.surface_overlay,
                Some(theme.stroke.surface),
                4.0,
            ))
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Label).label("FPS counter"),
            ),
    );
    let fps = if state.fps > 0.0 {
        format!("{:.0} FPS", state.fps)
    } else {
        "-- FPS".to_string()
    };
    widgets::label(
        ui,
        counter,
        "showcase.fps.label",
        fps,
        themed_text(theme, 11.0),
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height_percent(1.0)
            .padding(5.0),
    );
}

fn showcase_windows(
    ui: &mut UiDocument,
    desktop: UiNodeId,
    state: &ShowcaseState,
    desktop_size: UiSize,
    theme: &Theme,
) {
    showcase_windows_with_desktop_state(ui, desktop, state, &state.desktop, desktop_size, theme);
}

fn showcase_windows_with_desktop_state(
    ui: &mut UiDocument,
    desktop: UiNodeId,
    state: &ShowcaseState,
    desktop_state: &ext_widgets::FloatingDesktopState,
    desktop_size: UiSize,
    theme: &Theme,
) {
    let windows = showcase_window_descriptors(state, desktop_state, desktop_size);
    let options = showcase_desktop_options(desktop_size, theme);
    ext_widgets::floating_desktop(
        ui,
        desktop,
        "showcase.windows",
        &windows,
        options,
        |ui, window, descriptor| match descriptor.id.as_str() {
            "labels" => labels(ui, window, state),
            "buttons" => buttons(ui, window, state),
            "checkbox" => checkbox(ui, window, state),
            "toggles" => toggles(ui, window, state),
            "slider" => slider(ui, window, state),
            "numeric" => numeric_inputs(ui, window, state),
            "text_input" => text_input(ui, window, state),
            "selection" => selection_widgets(ui, window, state),
            "menus" => menu_widgets(ui, window, state),
            "command_palette" => command_palette(ui, window, state),
            "date_picker" => date_picker(ui, window, state),
            "color_picker" => color_picker(ui, window, state),
            "progress" => progress_indicator(ui, window, state),
            "animation" => animation_widgets(ui, window, state),
            "easing" => easing_widgets(ui, window, state),
            "lists_tables" => list_and_table_widgets(ui, window, state),
            "property_inspector" => property_inspector(ui, window, state),
            "diagnostics" => diagnostics_widgets(ui, window, state),
            "trees" => tree_widgets(ui, window, state),
            "layout_widgets" => tab_split_dock_widgets(ui, window, state),
            "containers" => container_widgets(ui, window, state),
            "panels" => panel_widgets(ui, window, state),
            "forms" => form_widgets(ui, window, state),
            "overlays" => overlay_widgets(ui, window, state),
            "drag_drop" => drag_drop_widgets(ui, window, state),
            "media" => media_widgets(ui, window, state),
            "shaders" => shader_effect_widgets(ui, window, state),
            "shader_lab" => shader_lab_widgets(ui, window, state),
            "timeline" => timeline_ruler(ui, window, state),
            "canvas" => canvas(ui, window, state),
            "theme" => theme_demo_widgets(ui, window, state, theme),
            "styling" => styling_widgets(ui, window, state),
            _ => {}
        },
    );
    showcase_overlays(ui, desktop, state, desktop_size);
}

#[allow(clippy::field_reassign_with_default)]
fn showcase_overlays(
    ui: &mut UiDocument,
    desktop: UiNodeId,
    state: &ShowcaseState,
    desktop_size: UiSize,
) {
    if state.toast_visible {
        let overlay_width = 320.0;
        let mut overlay_style = UiNodeStyle::from(operad::layout::absolute(
            (desktop_size.width - overlay_width - 18.0).max(18.0),
            18.0,
            overlay_width,
            180.0,
        ));
        overlay_style.set_clip(ClipBehavior::None);
        overlay_style.set_z_index(6000.0);
        let overlay = ui.add_child(
            desktop,
            UiNode::container("showcase.toast_overlay", overlay_style),
        );
        let mut stack = ext_widgets::ToastStack::new(3);
        stack.push_toast(
            ext_widgets::Toast::new(
                ext_widgets::ToastId::new(1),
                ext_widgets::ToastSeverity::Success,
                "Saved",
                Some("All changes are written".to_string()),
                None,
            )
            .with_action(ext_widgets::ToastAction::new("undo", "Undo")),
        );
        stack.push(
            ext_widgets::ToastSeverity::Warning,
            "Autosave paused",
            Some("Changes are kept locally".to_string()),
            None,
        );
        let mut options = ext_widgets::ToastStackOptions::default();
        options.z_index = 6100.0;
        ext_widgets::toast_stack(ui, overlay, "showcase.toast_overlay.stack", &stack, options);
    }
}

fn showcase_window_descriptors(
    state: &ShowcaseState,
    desktop_state: &ext_widgets::FloatingDesktopState,
    desktop_size: UiSize,
) -> Vec<ext_widgets::FloatingWindowDescriptor> {
    let wide = (desktop_size.width - 36.0).clamp(320.0, 720.0);
    let medium = (desktop_size.width - 36.0).clamp(300.0, 604.0);
    let buttons_width = medium.min(620.0);
    let mut windows = Vec::new();
    push_window(
        &mut windows,
        state.windows.labels,
        "labels",
        "Labels",
        UiSize::new(380.0, 460.0),
    );
    push_window(
        &mut windows,
        state.windows.buttons,
        "buttons",
        "Buttons",
        UiSize::new(buttons_width, 220.0),
    );
    push_window(
        &mut windows,
        state.windows.checkbox,
        "checkbox",
        "Checkbox",
        UiSize::new(250.0, 72.0),
    );
    push_window(
        &mut windows,
        state.windows.toggles,
        "toggles",
        "Radio and toggles",
        UiSize::new(360.0, 320.0),
    );
    push_window(
        &mut windows,
        state.windows.slider,
        "slider",
        "Slider",
        UiSize::new(430.0, 560.0),
    );
    push_window(
        &mut windows,
        state.windows.numeric,
        "numeric",
        "Numeric input",
        UiSize::new(360.0, 180.0),
    );
    push_window(
        &mut windows,
        state.windows.text_input,
        "text_input",
        "Text input",
        UiSize::new(520.0, 560.0),
    );
    push_window(
        &mut windows,
        state.windows.selection,
        "selection",
        "Select controls",
        UiSize::new(300.0, 430.0),
    );
    push_window(
        &mut windows,
        state.windows.menus,
        "menus",
        "Menu controls",
        UiSize::new(wide, 520.0),
    );
    push_window(
        &mut windows,
        state.windows.command_palette,
        "command_palette",
        "Command palette",
        UiSize::new(280.0, 130.0),
    );
    push_window(
        &mut windows,
        state.windows.date_picker,
        "date_picker",
        "Date picker",
        UiSize::new(430.0, 390.0),
    );
    push_window(
        &mut windows,
        state.windows.color_picker,
        "color_picker",
        "Color picker",
        UiSize::new(340.0, 390.0),
    );
    push_window(
        &mut windows,
        state.windows.progress,
        "progress",
        "Progress indicator",
        UiSize::new(500.0, 168.0),
    );
    push_window(
        &mut windows,
        state.windows.animation,
        "animation",
        "Animation",
        UiSize::new(520.0, 430.0),
    );
    push_window(
        &mut windows,
        state.windows.easing,
        "easing",
        "Easing",
        UiSize::new(520.0, 450.0),
    );
    push_window(
        &mut windows,
        state.windows.lists_tables,
        "lists_tables",
        "Lists and tables",
        UiSize::new(600.0, 500.0),
    );
    push_window(
        &mut windows,
        state.windows.property_inspector,
        "property_inspector",
        "Property inspector",
        UiSize::new(330.0, 250.0),
    );
    push_window(
        &mut windows,
        state.windows.diagnostics,
        "diagnostics",
        "Diagnostics",
        UiSize::new(640.0, 760.0),
    );
    push_window(
        &mut windows,
        state.windows.trees,
        "trees",
        "Trees",
        UiSize::new(430.0, 390.0),
    );
    push_window(
        &mut windows,
        state.windows.layout_widgets,
        "layout_widgets",
        "Layout widgets",
        UiSize::new(wide.min(700.0), 430.0),
    );
    push_window(
        &mut windows,
        state.windows.containers,
        "containers",
        "Containers",
        UiSize::new(380.0, 520.0),
    );
    push_window(
        &mut windows,
        state.windows.panels,
        "panels",
        "Panels",
        UiSize::new(460.0, 280.0),
    );
    push_window(
        &mut windows,
        state.windows.forms,
        "forms",
        "Forms",
        UiSize::new(520.0, 620.0),
    );
    push_window(
        &mut windows,
        state.windows.overlays,
        "overlays",
        "Overlays",
        UiSize::new(620.0, 680.0),
    );
    push_window(
        &mut windows,
        state.windows.drag_drop,
        "drag_drop",
        "Drag and drop",
        UiSize::new(500.0, 460.0),
    );
    push_window(
        &mut windows,
        state.windows.media,
        "media",
        "Media",
        UiSize::new(520.0, 430.0),
    );
    push_window(
        &mut windows,
        state.windows.shaders,
        "shaders",
        "Shader effects",
        UiSize::new(500.0, 410.0),
    );
    push_window(
        &mut windows,
        state.windows.shader_lab,
        "shader_lab",
        "Shader lab",
        UiSize::new(1000.0, 700.0),
    );
    push_window(
        &mut windows,
        state.windows.timeline,
        "timeline",
        "Timeline",
        UiSize::new(600.0, 120.0),
    );
    push_window(
        &mut windows,
        state.windows.canvas,
        "canvas",
        "Canvas",
        UiSize::new(760.0, 500.0),
    );
    push_window(
        &mut windows,
        state.windows.theme,
        "theme",
        "Theme",
        UiSize::new(430.0, 360.0),
    );
    push_window(
        &mut windows,
        state.windows.styling,
        "styling",
        "Styling",
        UiSize::new(540.0, 440.0),
    );
    for window in &mut windows {
        window.drag_action = Some(WidgetActionBinding::action(format!(
            "window.drag.{}",
            window.id
        )));
        window.collapse_action = Some(WidgetActionBinding::action(format!(
            "window.collapse.{}",
            window.id
        )));
        window.resize_action = Some(WidgetActionBinding::action(format!(
            "window.resize.{}",
            window.id
        )));
        desktop_state.apply_to_descriptor(window, window_defaults(window.id.as_str()));
    }
    windows
}

fn push_window(
    windows: &mut Vec<ext_widgets::FloatingWindowDescriptor>,
    visible: bool,
    id: &'static str,
    title: &'static str,
    preferred_size: UiSize,
) {
    if visible {
        let mut window = ext_widgets::FloatingWindowDescriptor::new(id, title, preferred_size)
            .with_min_size(default_window_state_min_size(id))
            .with_auto_size_to_content(true)
            .with_activate_action(format!("window.activate.{id}"))
            .with_close_action(format!("window.close.{id}"));
        if id == "animation" {
            window = window.with_content_min_size(UiSize::new(
                ANIMATION_STAGE_MIN_WIDTH,
                ANIMATION_CONTENT_MIN_HEIGHT,
            ));
        } else if id == "easing" {
            window = window.with_content_min_size(UiSize::new(
                EASING_STAGE_MIN_WIDTH,
                EASING_CONTENT_MIN_HEIGHT,
            ));
        } else if id == "layout_widgets" {
            window = window.with_content_min_size(UiSize::new(640.0, 360.0));
        } else if id == "canvas" {
            window = window.with_content_min_size(UiSize::new(720.0, 440.0));
        } else if id == "shader_lab" {
            window = window.with_content_min_size(UiSize::new(
                SHADER_LAB_CONTENT_MIN_WIDTH,
                SHADER_LAB_CONTENT_MIN_HEIGHT,
            ));
        }
        windows.push(window);
    }
}

fn default_window_size(id: &str) -> UiSize {
    match id {
        "labels" => UiSize::new(380.0, 460.0),
        "buttons" => UiSize::new(604.0, 220.0),
        "checkbox" => UiSize::new(380.0, 360.0),
        "toggles" => UiSize::new(400.0, 430.0),
        "slider" => UiSize::new(430.0, 560.0),
        "numeric" => UiSize::new(430.0, 260.0),
        "text_input" => UiSize::new(520.0, 640.0),
        "selection" => UiSize::new(300.0, 430.0),
        "menus" => UiSize::new(640.0, 640.0),
        "command_palette" => UiSize::new(280.0, 130.0),
        "date_picker" => UiSize::new(304.0, 470.0),
        "color_picker" => UiSize::new(340.0, 390.0),
        "progress" => UiSize::new(500.0, 300.0),
        "animation" => UiSize::new(520.0, 430.0),
        "easing" => UiSize::new(520.0, 450.0),
        "lists_tables" => UiSize::new(600.0, 500.0),
        "property_inspector" => UiSize::new(330.0, 250.0),
        "diagnostics" => UiSize::new(640.0, 760.0),
        "trees" => UiSize::new(430.0, 450.0),
        "layout_widgets" => UiSize::new(700.0, 430.0),
        "containers" => UiSize::new(380.0, 520.0),
        "panels" => UiSize::new(640.0, 440.0),
        "forms" => UiSize::new(520.0, 620.0),
        "overlays" => UiSize::new(620.0, 680.0),
        "drag_drop" => UiSize::new(500.0, 460.0),
        "media" => UiSize::new(430.0, 560.0),
        "shaders" => UiSize::new(500.0, 410.0),
        "shader_lab" => UiSize::new(1000.0, 700.0),
        "timeline" => UiSize::new(760.0, 280.0),
        "canvas" => UiSize::new(760.0, 500.0),
        "theme" => UiSize::new(430.0, 360.0),
        "styling" => UiSize::new(640.0, 560.0),
        _ => UiSize::new(300.0, 180.0),
    }
}

fn default_window_state_min_size(_id: &str) -> UiSize {
    UiSize::new(160.0, 96.0)
}

fn showcase_window_title(id: &str) -> &'static str {
    match id {
        "labels" => "Labels",
        "buttons" => "Buttons",
        "checkbox" => "Checkbox",
        "toggles" => "Radio and toggles",
        "slider" => "Slider",
        "numeric" => "Numeric input",
        "text_input" => "Text input",
        "selection" => "Select controls",
        "menus" => "Menu controls",
        "command_palette" => "Command palette",
        "date_picker" => "Date picker",
        "color_picker" => "Color picker",
        "progress" => "Progress indicator",
        "animation" => "Animation",
        "easing" => "Easing",
        "lists_tables" => "Lists and tables",
        "property_inspector" => "Property inspector",
        "diagnostics" => "Diagnostics",
        "trees" => "Trees",
        "layout_widgets" => "Layout widgets",
        "containers" => "Containers",
        "panels" => "Panels",
        "forms" => "Forms",
        "overlays" => "Overlays",
        "drag_drop" => "Drag and drop",
        "media" => "Media",
        "shaders" => "Shader effects",
        "shader_lab" => "Shader lab",
        "timeline" => "Timeline",
        "canvas" => "Canvas",
        "theme" => "Theme",
        "styling" => "Styling",
        _ => "Window",
    }
}

fn showcase_collapsed_window_size(
    id: &str,
    options: &ext_widgets::FloatingDesktopOptions,
) -> UiSize {
    let min_size = default_window_state_min_size(id);
    let padding = options.content_padding.max(0.0);
    let button = options.close_button_size.max(1.0);
    let control_width = (button + 8.0) * 2.0;
    let font_size = options.title_style.font_size.max(1.0);
    let title_width =
        (showcase_window_title(id).chars().count() as f32 * font_size * 0.55).max(font_size);
    UiSize::new(
        min_size
            .width
            .max(padding * 2.0 + control_width + title_width),
        options.title_bar_height.max(1.0),
    )
}

fn default_window_position(id: &str) -> UiPoint {
    match id {
        "labels" => UiPoint::new(18.0, 18.0),
        "buttons" => UiPoint::new(420.0, 18.0),
        "checkbox" => UiPoint::new(360.0, 18.0),
        "toggles" => UiPoint::new(360.0, 110.0),
        "slider" => UiPoint::new(360.0, 110.0),
        "numeric" => UiPoint::new(360.0, 260.0),
        "text_input" => UiPoint::new(360.0, 18.0),
        "selection" => UiPoint::new(360.0, 404.0),
        "menus" => UiPoint::new(18.0, 18.0),
        "command_palette" => UiPoint::new(68.0, 88.0),
        "date_picker" => UiPoint::new(300.0, 170.0),
        "color_picker" => UiPoint::new(18.0, 560.0),
        "progress" => UiPoint::new(72.0, 540.0),
        "animation" => UiPoint::new(180.0, 170.0),
        "easing" => UiPoint::new(220.0, 210.0),
        "lists_tables" => UiPoint::new(18.0, 90.0),
        "property_inspector" => UiPoint::new(300.0, 420.0),
        "diagnostics" => UiPoint::new(640.0, 70.0),
        "trees" => UiPoint::new(36.0, 220.0),
        "layout_widgets" => UiPoint::new(18.0, 18.0),
        "containers" => UiPoint::new(48.0, 120.0),
        "panels" => UiPoint::new(140.0, 120.0),
        "forms" => UiPoint::new(120.0, 160.0),
        "overlays" => UiPoint::new(80.0, 110.0),
        "drag_drop" => UiPoint::new(210.0, 250.0),
        "media" => UiPoint::new(120.0, 360.0),
        "shaders" => UiPoint::new(180.0, 260.0),
        "shader_lab" => UiPoint::new(120.0, 170.0),
        "timeline" => UiPoint::new(18.0, 620.0),
        "canvas" => UiPoint::new(280.0, 390.0),
        "theme" => UiPoint::new(120.0, 120.0),
        "styling" => UiPoint::new(86.0, 118.0),
        _ => UiPoint::new(18.0, 18.0),
    }
}

fn window_for_action(action_id: &str) -> Option<&'static str> {
    match action_id {
        id if id.starts_with("labels.") => Some("labels"),
        id if id.starts_with("button.") => Some("buttons"),
        id if id.starts_with("checkbox.") => Some("checkbox"),
        id if id.starts_with("toggles.") => Some("toggles"),
        id if id.starts_with("theme.preference.") => Some("toggles"),
        id if id.starts_with("slider.") => Some("slider"),
        id if id.starts_with("numeric.") => Some("numeric"),
        id if id.starts_with("text.") => Some("text_input"),
        id if id.starts_with("selection.dropdown.")
            || id.starts_with("selection.menu.")
            || id.starts_with("selection.image_menu.") =>
        {
            Some("selection")
        }
        id if id.starts_with("menus.") => Some("menus"),
        id if id.starts_with("command_palette.") => Some("command_palette"),
        id if id.starts_with("date.") => Some("date_picker"),
        id if id.starts_with("color.") => Some("color_picker"),
        id if id.starts_with("progress.") => Some("progress"),
        id if id.starts_with("animation.") => Some("animation"),
        id if id.starts_with("easing.") => Some("easing"),
        id if id.starts_with("lists_tables.") => Some("lists_tables"),
        id if id.starts_with("property_inspector.") => Some("property_inspector"),
        id if id.starts_with("diagnostics.") => Some("diagnostics"),
        id if id.starts_with("trees.") => Some("trees"),
        id if id.starts_with("layout.") || id.starts_with("layout_widgets.") => {
            Some("layout_widgets")
        }
        id if id.starts_with("containers.") => Some("containers"),
        id if id.starts_with("panels.") => Some("panels"),
        id if id.starts_with("forms.") => Some("forms"),
        id if id.starts_with("overlays.") => Some("overlays"),
        id if id.starts_with("drag_drop.") => Some("drag_drop"),
        id if id.starts_with("media.") => Some("media"),
        id if id.starts_with("shaders.") => Some("shaders"),
        id if id.starts_with("shader_lab.") => Some("shader_lab"),
        id if id.starts_with("toast.") => Some("overlays"),
        id if id.starts_with("canvas.") => Some("canvas"),
        id if id.starts_with("theme.") => Some("theme"),
        id if id.starts_with("styling.") => Some("styling"),
        _ => None,
    }
}

fn focused_text_for_action(action_id: &str) -> Option<FocusedTextInput> {
    Some(match action_id {
        "text.input.edit" => FocusedTextInput::Editable,
        "text.selectable.edit" => FocusedTextInput::Selectable,
        "text.singleline.edit" => FocusedTextInput::Singleline,
        "text.multiline.edit" => FocusedTextInput::Multiline,
        "text.area.edit" => FocusedTextInput::TextArea,
        "text.code_editor.edit" => FocusedTextInput::CodeEditor,
        "text.search.edit" => FocusedTextInput::Search,
        "text.password.edit" => FocusedTextInput::Password,
        "forms.profile.name.input.edit" => FocusedTextInput::FormName,
        "forms.profile.email.input.edit" => FocusedTextInput::FormEmail,
        "forms.profile.role.input.edit" => FocusedTextInput::FormRole,
        "numeric.value.edit" => FocusedTextInput::NumericValue,
        "numeric.range_min.value.edit" => FocusedTextInput::NumericRangeMin,
        "numeric.range_max.value.edit" => FocusedTextInput::NumericRangeMax,
        "slider.value_text.edit" => FocusedTextInput::SliderValue,
        "slider.left_text.edit" => FocusedTextInput::SliderRangeLeft,
        "slider.right_text.edit" => FocusedTextInput::SliderRangeRight,
        "slider.step_text.edit" => FocusedTextInput::SliderStep,
        "shader_lab.editor.edit" => FocusedTextInput::ShaderLabSource,
        _ => return None,
    })
}

fn control_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    viewport_height: f32,
    theme: &Theme,
) {
    widgets::label(
        ui,
        parent,
        "controls.title",
        "Widgets",
        themed_text(theme, 16.0),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let list_viewport_height = controls_list_viewport_height(viewport_height);
    let controls_scroll =
        controls_scroll_state_for_view(state.controls_scroll, list_viewport_height);
    let list_nodes = scroll_area_widgets::scroll_container_shell(
        ui,
        parent,
        "controls.widget_list",
        controls_scroll,
        widgets::ScrollContainerOptions::default()
            .with_layout(
                LayoutStyle::column()
                    .with_width_percent(1.0)
                    .with_height(list_viewport_height)
                    .with_flex_grow(1.0)
                    .with_flex_shrink(1.0),
            )
            .with_viewport_layout(
                LayoutStyle::column()
                    .with_width(0.0)
                    .with_height_percent(1.0)
                    .with_flex_grow(1.0)
                    .with_flex_shrink(1.0)
                    .gap(CONTROLS_WIDGET_ROW_GAP),
            )
            .with_axes(ScrollAxes::VERTICAL)
            .with_scrollbar_thickness(8.0)
            .with_gap(2.0)
            .with_action_prefix("controls.widget_list")
            .with_vertical_scrollbar(
                scrollbar_widgets::ScrollbarOptions::default()
                    .with_action("controls.widget_list.scrollbar"),
            ),
    );
    let list = list_nodes.viewport;

    window_toggle(ui, list, "labels", "Labels", state.windows.labels, theme);
    window_toggle(ui, list, "buttons", "Buttons", state.windows.buttons, theme);
    window_toggle(
        ui,
        list,
        "checkbox",
        "Checkbox",
        state.windows.checkbox,
        theme,
    );
    window_toggle(
        ui,
        list,
        "toggles",
        "Radio and toggles",
        state.windows.toggles,
        theme,
    );
    window_toggle(ui, list, "slider", "Slider", state.windows.slider, theme);
    window_toggle(
        ui,
        list,
        "numeric",
        "Numeric input",
        state.windows.numeric,
        theme,
    );
    window_toggle(
        ui,
        list,
        "text_input",
        "Text input",
        state.windows.text_input,
        theme,
    );
    window_toggle(
        ui,
        list,
        "selection",
        "Select controls",
        state.windows.selection,
        theme,
    );
    window_toggle(
        ui,
        list,
        "menus",
        "Menu controls",
        state.windows.menus,
        theme,
    );
    window_toggle(
        ui,
        list,
        "command_palette",
        "Command palette",
        state.windows.command_palette,
        theme,
    );
    window_toggle(
        ui,
        list,
        "date_picker",
        "Date picker",
        state.windows.date_picker,
        theme,
    );
    window_toggle(
        ui,
        list,
        "color_picker",
        "Color picker",
        state.windows.color_picker,
        theme,
    );
    window_toggle(
        ui,
        list,
        "progress",
        "Progress indicator",
        state.windows.progress,
        theme,
    );
    window_toggle(
        ui,
        list,
        "animation",
        "Animation",
        state.windows.animation,
        theme,
    );
    window_toggle(ui, list, "easing", "Easing", state.windows.easing, theme);
    window_toggle(
        ui,
        list,
        "lists_tables",
        "Lists and tables",
        state.windows.lists_tables,
        theme,
    );
    window_toggle(
        ui,
        list,
        "property_inspector",
        "Property inspector",
        state.windows.property_inspector,
        theme,
    );
    window_toggle(
        ui,
        list,
        "diagnostics",
        "Diagnostics",
        state.windows.diagnostics,
        theme,
    );
    window_toggle(ui, list, "trees", "Trees", state.windows.trees, theme);
    window_toggle(
        ui,
        list,
        "layout_widgets",
        "Layout widgets",
        state.windows.layout_widgets,
        theme,
    );
    window_toggle(
        ui,
        list,
        "containers",
        "Containers",
        state.windows.containers,
        theme,
    );
    window_toggle(ui, list, "panels", "Panels", state.windows.panels, theme);
    window_toggle(ui, list, "forms", "Forms", state.windows.forms, theme);
    window_toggle(
        ui,
        list,
        "overlays",
        "Overlays, popups, and toasts",
        state.windows.overlays,
        theme,
    );
    window_toggle(
        ui,
        list,
        "drag_drop",
        "Drag and drop",
        state.windows.drag_drop,
        theme,
    );
    window_toggle(ui, list, "media", "Media", state.windows.media, theme);
    window_toggle(
        ui,
        list,
        "shaders",
        "Shader effects",
        state.windows.shaders,
        theme,
    );
    window_toggle(
        ui,
        list,
        "shader_lab",
        "Shader lab",
        state.windows.shader_lab,
        theme,
    );
    window_toggle(
        ui,
        list,
        "timeline",
        "Timeline",
        state.windows.timeline,
        theme,
    );
    window_toggle(ui, list, "canvas", "Canvas", state.windows.canvas, theme);
    window_toggle(ui, list, "theme", "Theme", state.windows.theme, theme);
    window_toggle(ui, list, "styling", "Styling", state.windows.styling, theme);

    ui.add_child(
        parent,
        UiNode::container(
            "controls.clear_all.spacer",
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(1.0)
                .with_flex_grow(1.0)
                .with_flex_shrink(1.0),
        ),
    );
    let actions = ui.add_child(
        parent,
        UiNode::container(
            "controls.bulk_actions",
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height(30.0)
                .with_flex_shrink(0.0)
                .gap(8.0),
        ),
    );
    control_action_button(
        ui,
        actions,
        "controls.add_all",
        "Add all",
        "window.add_all",
        "Add all widgets",
        theme,
    );
    control_action_button(
        ui,
        actions,
        "controls.clear_all",
        "Clear all",
        "window.clear_all",
        "Clear all widgets",
        theme,
    );
}

fn control_action_button(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    action: &'static str,
    accessibility_label: &'static str,
    theme: &Theme,
) {
    let mut options = themed_button_options(
        theme,
        action,
        ComponentState::NORMAL,
        LayoutStyle::new()
            .with_width(0.0)
            .with_height_percent(1.0)
            .with_flex_grow(1.0)
            .with_flex_shrink(1.0),
    );
    options.text_style = themed_text(theme, 12.0);
    options.accessibility_label = Some(accessibility_label.to_string());
    widgets::button(ui, parent, name, label, options);
}

fn window_toggle(
    ui: &mut UiDocument,
    parent: UiNodeId,
    id: &'static str,
    label: &'static str,
    checked: bool,
    theme: &Theme,
) {
    let mut options =
        widgets::CheckboxOptions::default().with_action(format!("window.toggle.{id}"));
    options.layout = LayoutStyle::new()
        .with_width_percent(1.0)
        .with_height(CONTROLS_WIDGET_ROW_HEIGHT);
    options.text_style = themed_text(theme, 12.0);
    options.box_visual = UiVisual::panel(
        theme.colors.surface_sunken,
        Some(StrokeStyle::new(theme.colors.border_strong, 1.0)),
        3.0,
    );
    options.checked_box_visual = Some(UiVisual::panel(
        theme.colors.accent,
        Some(theme.stroke.focus),
        3.0,
    ));
    options.check_color = theme.colors.accent_text;
    widgets::checkbox(
        ui,
        parent,
        format!("controls.{id}"),
        label,
        checked,
        options,
    );
}

#[allow(clippy::field_reassign_with_default)]
fn labels(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "labels", "Labels");
    ui.set_node_style(
        body,
        LayoutStyle::column()
            .with_width_percent(1.0)
            .with_height_percent(1.0)
            .with_flex_grow(1.0)
            .gap(10.0),
    );
    widgets::label(
        ui,
        body,
        "labels.plain",
        "Plain label",
        text(13.0, color(226, 232, 242)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let locale_items = label_locale_options();
    let locale_id = state
        .label_locale
        .selected_id(&locale_items)
        .unwrap_or("es-MX");
    let localization =
        LocalizationPolicy::new(LocaleId::new(locale_id).unwrap_or_else(|_| LocaleId::default()));
    let locale_row = ui.add_child(
        body,
        UiNode::container(
            "labels.locale.row",
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_align_items(taffy::prelude::AlignItems::Center)
                .gap(10.0),
        ),
    );
    let locale_label_width = 270.0;
    let locale_dropdown_width = 148.0;
    let locale_gap = 10.0;
    widgets::localized_label(
        ui,
        locale_row,
        "labels.localized",
        DynamicLabelMeta::keyed("showcase.localized.greeting", localized_label(locale_id)),
        Some(&localization),
        text(13.0, color(170, 202, 255)),
        LayoutStyle::new().with_width(locale_label_width),
    );
    let mut locale_options = ext_widgets::DropdownSelectOptions::default();
    locale_options.trigger_layout = LayoutStyle::row()
        .with_width(locale_dropdown_width)
        .with_height(30.0)
        .with_align_items(taffy::prelude::AlignItems::Center)
        .with_justify_content(taffy::prelude::JustifyContent::FlexStart)
        .gap(6.0)
        .padding(6.0);
    locale_options.text_style = text(13.0, color(226, 232, 242));
    locale_options.accessibility_label = Some("Locale".to_string());
    locale_options.menu = ext_widgets::SelectMenuOptions::default();
    locale_options.menu.width = locale_dropdown_width;
    locale_options.menu.row_height = 30.0;
    locale_options.menu.max_visible_rows = locale_items.len();
    locale_options.menu.text_style = text(13.0, color(226, 232, 242));
    locale_options.menu.portal = UiPortalTarget::Parent;
    locale_options = locale_options.with_action_prefix("labels.locale");
    ext_widgets::dropdown_select(
        ui,
        locale_row,
        "labels.locale",
        &locale_items,
        &state.label_locale,
        Some(ext_widgets::AnchoredPopup::new(
            UiRect::new(
                locale_label_width + locale_gap,
                0.0,
                locale_dropdown_width,
                30.0,
            ),
            UiRect::new(0.0, 0.0, 460.0, 260.0),
            ext_widgets::PopupPlacement::default()
                .with_offset(0.0)
                .with_viewport_margin(0.0),
        )),
        locale_options,
    );
    widgets::label(
        ui,
        body,
        "labels.muted",
        "Muted helper label",
        text(12.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let sizes = ui.add_child(
        body,
        UiNode::container(
            "labels.sizes",
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_align_items(taffy::prelude::AlignItems::FlexEnd)
                .gap(12.0),
        ),
    );
    widgets::label(
        ui,
        sizes,
        "labels.size.small",
        "12px",
        text(12.0, color(226, 232, 242)),
        LayoutStyle::new(),
    );
    widgets::label(
        ui,
        sizes,
        "labels.size.default",
        "13px",
        text(13.0, color(226, 232, 242)),
        LayoutStyle::new(),
    );
    widgets::label(
        ui,
        sizes,
        "labels.size.large",
        "18px",
        text(18.0, color(246, 249, 252)),
        LayoutStyle::new(),
    );
    widgets::label(
        ui,
        sizes,
        "labels.size.display",
        "24px",
        text(24.0, color(246, 249, 252)),
        LayoutStyle::new(),
    );

    let style_row = row(ui, body, "labels.styles", 12.0);
    let mut bold = text(13.0, color(246, 249, 252));
    bold.weight = FontWeight::BOLD;
    widgets::label(
        ui,
        style_row,
        "labels.style.bold",
        "Bold",
        bold,
        LayoutStyle::new(),
    );
    widgets::label(
        ui,
        style_row,
        "labels.style.weak",
        "Muted",
        text(13.0, color(154, 166, 184)),
        LayoutStyle::new(),
    );

    let font_row = row(ui, body, "labels.fonts", 12.0);
    let mut serif = text(13.0, color(226, 232, 242));
    serif.family = FontFamily::Serif;
    widgets::label(
        ui,
        font_row,
        "labels.font.serif",
        "Serif",
        serif,
        LayoutStyle::new(),
    );
    let mut mono = text(13.0, color(226, 232, 242));
    mono.family = FontFamily::Monospace;
    widgets::label(
        ui,
        font_row,
        "labels.font.mono",
        "Monospace",
        mono,
        LayoutStyle::new(),
    );

    let code_panel = ui.add_child(
        body,
        UiNode::container(
            "labels.code.panel",
            LayoutStyle::new()
                .with_width_percent(1.0)
                .padding(8.0)
                .with_height(36.0),
        )
        .with_visual(UiVisual::panel(
            color(10, 14, 20),
            Some(StrokeStyle::new(color(47, 59, 74), 1.0)),
            4.0,
        )),
    );
    widgets::code_label(
        ui,
        code_panel,
        "labels.code",
        "let label = widgets::label(...);",
        LayoutStyle::new().with_width_percent(1.0),
    );

    let colors = row(ui, body, "labels.colors", 14.0);
    widgets::colored_label(
        ui,
        colors,
        "labels.color.green",
        "Green",
        color(111, 203, 159),
        LayoutStyle::new(),
    );
    widgets::colored_label(
        ui,
        colors,
        "labels.color.yellow",
        "Yellow",
        color(232, 196, 101),
        LayoutStyle::new(),
    );
    widgets::colored_label(
        ui,
        colors,
        "labels.color.red",
        "Red",
        color(244, 118, 118),
        LayoutStyle::new(),
    );

    let wrap_row = wrapping_row(ui, body, "labels.wrap.row", 10.0);
    let wrap_word = ui.add_child(
        wrap_row,
        UiNode::container(
            "labels.wrap.word.panel",
            LayoutStyle::column().with_width(172.0).padding(8.0),
        )
        .with_visual(UiVisual::panel(
            color(18, 23, 31),
            Some(StrokeStyle::new(color(47, 59, 74), 1.0)),
            4.0,
        )),
    );
    widgets::wrapped_label(
        ui,
        wrap_word,
        "labels.wrap.word",
        "Word wrapping keeps this sentence readable in a narrow box.",
        TextWrap::Word,
        LayoutStyle::new().with_width_percent(1.0),
    );
    let wrap_glyph = ui.add_child(
        wrap_row,
        UiNode::container(
            "labels.wrap.glyph.panel",
            LayoutStyle::column().with_width(172.0).padding(8.0),
        )
        .with_visual(UiVisual::panel(
            color(18, 23, 31),
            Some(StrokeStyle::new(color(47, 59, 74), 1.0)),
            4.0,
        )),
    );
    widgets::wrapped_label(
        ui,
        wrap_glyph,
        "labels.wrap.glyph",
        "LongIdentifierWithoutSpaces",
        TextWrap::Glyph,
        LayoutStyle::new().with_width_percent(1.0),
    );

    let links = wrapping_row(ui, body, "labels.links", 12.0);
    widgets::link(
        ui,
        links,
        "labels.link",
        "Internal action",
        widgets::LinkOptions::default()
            .visited(state.label_link_visited)
            .with_action("labels.link"),
    );
    widgets::hyperlink(
        ui,
        links,
        "labels.hyperlink",
        "Open docs.rs",
        "https://docs.rs/operad",
        widgets::LinkOptions::default()
            .visited(state.label_hyperlink_visited)
            .with_action("labels.hyperlink"),
    );
    if state.label_link_status != "No link action yet" {
        widgets::label(
            ui,
            body,
            "labels.status",
            format!("Last action: {}", state.label_link_status),
            text(12.0, color(154, 166, 184)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }
}

fn buttons(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "buttons", "Buttons");
    let primary_row = wrapping_row(ui, body, "buttons.row", 10.0);
    button(
        ui,
        primary_row,
        "button.default",
        "Default",
        "button.default",
        button_visual(38, 46, 58),
    );
    button(
        ui,
        primary_row,
        "button.primary",
        "Primary",
        "button.primary",
        button_visual(48, 112, 184),
    );
    button(
        ui,
        primary_row,
        "button.secondary",
        "Secondary",
        "button.secondary",
        button_visual(58, 78, 96),
    );
    button(
        ui,
        primary_row,
        "button.destructive",
        "Destructive",
        "button.destructive",
        button_visual(157, 65, 73),
    );
    let mut disabled = widgets::ButtonOptions::new(LayoutStyle::size(92.0, 32.0));
    disabled.enabled = false;
    disabled.visual = button_visual(40, 44, 52);
    disabled.text_style = text(13.0, color(138, 146, 158));
    widgets::button(ui, primary_row, "button.disabled", "Disabled", disabled);
    let second_row = wrapping_row(ui, body, "buttons.row.options", 10.0);
    button(
        ui,
        second_row,
        "button.momentary",
        "Press only",
        "button.default",
        button_visual(42, 50, 62),
    );
    let toggle_visual = if state.toggle_button {
        button_visual(48, 112, 184)
    } else {
        button_visual(42, 50, 62)
    };
    let mut toggle =
        widgets::ButtonOptions::new(LayoutStyle::size(112.0, 32.0)).with_action("button.toggle");
    toggle.visual = toggle_visual;
    toggle.hovered_visual = Some(readable_button_hover_visual(toggle_visual));
    toggle.pressed_visual = Some(adjusted_button_visual(toggle_visual, -34));
    toggle.pressed_hovered_visual = Some(adjusted_button_visual(toggle_visual, -18));
    toggle.accessibility_label = Some("Toggle button state".to_owned());
    toggle.text_style = text(13.0, color(246, 249, 252));
    let toggle_button = widgets::button(
        ui,
        second_row,
        "button.toggle",
        if state.toggle_button {
            "Toggle on"
        } else {
            "Toggle off"
        },
        toggle,
    );
    mark_as_toggle_button(ui, toggle_button, state.toggle_button);
    let mut forced_pressed = widgets::ButtonOptions::new(LayoutStyle::size(112.0, 32.0));
    forced_pressed.pressed = true;
    forced_pressed.visual = button_visual(42, 50, 62);
    forced_pressed.hovered_visual = Some(button_visual(62, 74, 92));
    forced_pressed.pressed_visual = Some(button_visual(38, 82, 136));
    forced_pressed.pressed_hovered_visual = Some(button_visual(62, 126, 196));
    forced_pressed.text_style = text(13.0, color(246, 249, 252));
    widgets::button(
        ui,
        second_row,
        "button.state.pressed",
        "Pressed",
        forced_pressed,
    );
    let helper_row = wrapping_row(ui, body, "buttons.row.helpers", 10.0);
    widgets::small_button(
        ui,
        helper_row,
        "button.small",
        "Small",
        widgets::ButtonOptions::default().with_action("button.small"),
    );
    widgets::icon_button(
        ui,
        helper_row,
        "button.icon",
        icon_image(BuiltInIcon::Settings),
        "Settings",
        widgets::ButtonOptions::default().with_action("button.icon"),
    );
    widgets::image_button(
        ui,
        helper_row,
        "button.image",
        icon_image(BuiltInIcon::Folder),
        "Folder",
        widgets::ButtonOptions::default().with_action("button.image"),
    );
    widgets::reset_button(
        ui,
        helper_row,
        "button.reset",
        state.toggle_button,
        widgets::ButtonOptions::default().with_action("button.reset"),
    );
    widgets::toggle_button(
        ui,
        helper_row,
        "button.toggle_helper",
        "Toggle helper",
        state.toggle_button,
        widgets::ButtonOptions::default().with_action("button.toggle"),
    );
    widgets::label(
        ui,
        body,
        "buttons.last",
        format!("Last pressed: {}", state.last_button),
        text(12.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn mark_as_toggle_button(ui: &mut UiDocument, button: UiNodeId, pressed: bool) {
    if let Some(accessibility) = ui.node_mut(button).accessibility_mut() {
        accessibility.role = AccessibilityRole::ToggleButton;
        accessibility.pressed = Some(pressed);
    }
}

fn checkbox(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body =
        section_with_min_viewport(ui, parent, "checkbox", "Checkbox", UiSize::new(300.0, 0.0));
    widgets::label(
        ui,
        body,
        "checkbox.states.title",
        "States",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let options = widgets::CheckboxOptions::default()
        .with_action("checkbox.enabled")
        .with_text_style(text(13.0, color(222, 228, 238)))
        .with_accessibility_hint("Toggle the shared checkbox demo value");
    widgets::checkbox(
        ui,
        body,
        "checkbox.enabled",
        "Toggle me",
        state.checked,
        options,
    );
    widgets::checkbox(
        ui,
        body,
        "checkbox.unchecked_sample",
        "Unchecked",
        false,
        widgets::CheckboxOptions::default().with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::checkbox(
        ui,
        body,
        "checkbox.checked_sample",
        "Checked",
        true,
        widgets::CheckboxOptions::default().with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::checkbox_with_state(
        ui,
        body,
        "checkbox.indeterminate_sample",
        "Indeterminate",
        widgets::CheckboxState::Indeterminate,
        widgets::CheckboxOptions::default()
            .with_indeterminate_support(true)
            .with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::checkbox(
        ui,
        body,
        "checkbox.disabled",
        "Disabled",
        true,
        widgets::CheckboxOptions::default()
            .disabled()
            .with_text_style(text(13.0, color(128, 138, 154))),
    );

    widgets::separator(
        ui,
        body,
        "checkbox.options.separator",
        widgets::SeparatorOptions::default(),
    );
    widgets::label(
        ui,
        body,
        "checkbox.options.title",
        "Options",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::checkbox(
        ui,
        body,
        "checkbox.large",
        "Larger box and hit target",
        state.checked,
        widgets::CheckboxOptions::default()
            .with_action("checkbox.large")
            .with_layout(LayoutStyle::row().with_width_percent(1.0).with_height(36.0))
            .with_box_size(UiSize::new(22.0, 22.0))
            .with_gap(10.0)
            .with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::checkbox(
        ui,
        body,
        "checkbox.custom_color",
        "Custom check color",
        state.checked,
        widgets::CheckboxOptions::default()
            .with_action("checkbox.custom_color")
            .with_check_color(color(111, 203, 159))
            .with_checked_box_visual(UiVisual::panel(
                color(29, 68, 50),
                Some(StrokeStyle::new(color(111, 203, 159), 1.0)),
                3.0,
            ))
            .with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::checkbox(
        ui,
        body,
        "checkbox.image_check",
        "Operad logo PNG check image",
        state.checked,
        widgets::CheckboxOptions::default()
            .with_action("checkbox.image_check")
            .with_box_size(UiSize::new(72.0, 72.0))
            .with_gap(14.0)
            .with_check_image(ImageContent::from(ImageHandle::app(
                SHOWCASE_USER_IMAGE_KEY,
            )))
            .with_checked_box_visual(UiVisual::panel(
                color(47, 39, 90),
                Some(StrokeStyle::new(color(156, 124, 255), 1.0)),
                3.0,
            ))
            .with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::checkbox(
        ui,
        body,
        "checkbox.compact_gap",
        "Compact gap",
        state.checked,
        widgets::CheckboxOptions::default()
            .with_action("checkbox.compact_gap")
            .with_gap(4.0)
            .with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::checkbox_with_state(
        ui,
        body,
        "checkbox.indeterminate",
        "Tri-state cycle",
        state.checkbox_indeterminate,
        widgets::CheckboxOptions::default()
            .with_action("checkbox.indeterminate")
            .with_indeterminate_support(true)
            .with_box_size(UiSize::new(22.0, 22.0))
            .with_checked_box_visual(UiVisual::panel(
                color(42, 53, 70),
                Some(StrokeStyle::new(color(108, 180, 255), 1.0)),
                3.0,
            ))
            .with_text_style(text(13.0, color(222, 228, 238))),
    );
}

fn toggles(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "toggles",
        "Radio and toggles",
        UiSize::new(390.0, 0.0),
    );
    widgets::label(
        ui,
        body,
        "toggles.radio.title",
        "Radio",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let radio_button_options = widgets::RadioButtonOptions::default()
        .with_text_style(text(13.0, color(222, 228, 238)))
        .with_outer_size(UiSize::new(18.0, 18.0))
        .with_dot_radius(4.5);
    let radio_options = [
        widgets::RadioOption::new("foo", "Foo").with_action("toggles.radio.foo"),
        widgets::RadioOption::new("bar", "Bar").with_action("toggles.radio.bar"),
        widgets::RadioOption::new("baz", "Baz").with_action("toggles.radio.baz"),
        widgets::RadioOption::new("disabled", "Disabled").enabled(false),
    ];
    let mut radio_group_options = widgets::RadioGroupOptions::default();
    radio_group_options.button_options = radio_button_options;
    widgets::radio_group(
        ui,
        body,
        "toggles.radio_group",
        &radio_options,
        Some(state.radio_choice),
        radio_group_options,
    );
    widgets::label(
        ui,
        body,
        "toggles.radio.options_note",
        "Custom indicator and no-label option",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let radio_examples = wrapping_row(ui, body, "toggles.radio_examples", 10.0);
    widgets::radio_button(
        ui,
        radio_examples,
        "toggles.radio_custom",
        "Foo",
        true,
        widgets::RadioButtonOptions::default()
            .with_action("toggles.radio.foo")
            .with_outer_visual(UiVisual::panel(
                color(33, 38, 48),
                Some(StrokeStyle::new(color(162, 128, 255), 1.0)),
                6.0,
            ))
            .with_selected_outer_visual(UiVisual::panel(
                color(55, 38, 112),
                Some(StrokeStyle::new(color(183, 148, 255), 1.0)),
                6.0,
            ))
            .with_dot_color(color(255, 206, 99))
            .with_outer_size(UiSize::new(18.0, 18.0))
            .with_dot_radius(4.0)
            .with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::radio_button(
        ui,
        radio_examples,
        "toggles.radio_no_label",
        "",
        state.radio_choice == "bar",
        widgets::RadioButtonOptions::default()
            .with_action("toggles.radio.bar")
            .accessibility_label("No-label radio option")
            .with_outer_size(UiSize::new(24.0, 24.0))
            .with_dot_radius(6.0),
    );

    widgets::separator(
        ui,
        body,
        "toggles.switch.separator",
        widgets::SeparatorOptions::default(),
    );
    widgets::label(
        ui,
        body,
        "toggles.switch.title",
        "Switches",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::toggle_switch(
        ui,
        body,
        "toggles.switch",
        "Label 1",
        ext_widgets::ToggleValue::from(state.switch_enabled),
        widgets::ToggleSwitchOptions::default()
            .with_action("toggles.switch")
            .with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::toggle_switch(
        ui,
        body,
        "toggles.mixed",
        "Label 2",
        state.mixed_switch,
        widgets::ToggleSwitchOptions::default()
            .with_action("toggles.mixed")
            .with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::label(
        ui,
        body,
        "toggles.switch.options_note",
        "Track color, thumb shape, length, disabled, and no-label variants",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let switch_examples = wrapping_row(ui, body, "toggles.switch_examples", 10.0);
    widgets::toggle_switch(
        ui,
        switch_examples,
        "toggles.switch_custom",
        "Label 3",
        ext_widgets::ToggleValue::from(state.switch_enabled),
        widgets::ToggleSwitchOptions::default()
            .with_action("toggles.switch")
            .with_track_size(UiSize::new(74.0, 24.0))
            .with_thumb_size(UiSize::new(28.0, 18.0))
            .with_track_visual(UiVisual::panel(color(47, 53, 66), None, 4.0))
            .with_on_track_visual(UiVisual::panel(color(91, 65, 158), None, 4.0))
            .with_thumb_visual(UiVisual::panel(
                color(255, 205, 90),
                Some(StrokeStyle::new(color(255, 236, 171), 1.0)),
                3.0,
            ))
            .with_text_style(text(13.0, color(222, 228, 238))),
    );
    widgets::toggle_switch(
        ui,
        switch_examples,
        "toggles.switch_no_label",
        "",
        ext_widgets::ToggleValue::from(state.switch_enabled),
        widgets::ToggleSwitchOptions::default()
            .with_action("toggles.switch")
            .accessibility_label("No-label switch")
            .with_track_size(UiSize::new(54.0, 26.0))
            .with_thumb_size(UiSize::new(22.0, 22.0))
            .with_on_track_visual(UiVisual::panel(color(30, 106, 84), None, 13.0)),
    );
    widgets::toggle_switch(
        ui,
        switch_examples,
        "toggles.switch_disabled",
        "Disabled",
        ext_widgets::ToggleValue::Off,
        widgets::ToggleSwitchOptions::default()
            .enabled(false)
            .with_text_style(text(13.0, color(128, 138, 154))),
    );

    widgets::separator(
        ui,
        body,
        "toggles.theme.separator",
        widgets::SeparatorOptions::default(),
    );
    widgets::label(
        ui,
        body,
        "toggles.theme.title",
        "Theme",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::theme_preference_buttons(
        ui,
        body,
        "toggles.theme_buttons",
        state.theme_preference,
        widgets::ThemePreferenceButtonsOptions::default().with_action_prefix("toggles.theme"),
    );
    widgets::theme_preference_switch(
        ui,
        body,
        "toggles.theme_switch",
        state.theme_preference,
        widgets::ThemePreferenceSwitchOptions::default().with_action("theme.preference.dark"),
    );
}

fn slider(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "slider", "Slider");
    widgets::label(
        ui,
        body,
        "slider.note",
        "Click a slider value to edit it with the keyboard.",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let value_row = row(ui, body, "slider.value.row", 10.0);
    let options = slider_options(state, 180.0).with_value_edit_action("slider.value");
    let slider_unit = state.slider_value_spec().normalize(state.slider);
    widgets::slider(
        ui,
        value_row,
        "slider.value",
        slider_unit,
        0.0..1.0,
        options.clone(),
    );
    slider_number_input(
        ui,
        value_row,
        "slider.value_text",
        &state.slider_value_text,
        FocusedTextInput::SliderValue,
        state,
        86.0,
    );
    widgets::label(
        ui,
        value_row,
        "slider.value.label",
        "f64 demo slider",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    widgets::label(
        ui,
        body,
        "slider.precision",
        format!(
            "Displayed value: {}    Full precision: {:.6}",
            widgets::slider::format_slider_value(state.slider),
            state.slider
        ),
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    divider(ui, body, "slider.divider.range");
    widgets::label(
        ui,
        body,
        "slider.range.label",
        "Slider range",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let left_row = row(ui, body, "slider.range.left.row", 10.0);
    let left_options = widgets::SliderOptions::default()
        .with_layout(
            LayoutStyle::new()
                .with_width(180.0)
                .with_height(24.0)
                .with_flex_shrink(0.0),
        )
        .with_value_edit_action("slider.range_left");
    widgets::slider(
        ui,
        left_row,
        "slider.range_left",
        state.slider_left,
        0.0..state.slider_right.max(1.0),
        left_options,
    );
    slider_number_input(
        ui,
        left_row,
        "slider.left_text",
        &state.slider_left_text,
        FocusedTextInput::SliderRangeLeft,
        state,
        96.0,
    );
    widgets::label(
        ui,
        left_row,
        "slider.range.left.label",
        "left",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width(46.0),
    );
    let right_row = row(ui, body, "slider.range.right.row", 10.0);
    let right_options = widgets::SliderOptions::default()
        .with_layout(
            LayoutStyle::new()
                .with_width(180.0)
                .with_height(24.0)
                .with_flex_shrink(0.0),
        )
        .with_value_edit_action("slider.range_right");
    widgets::slider(
        ui,
        right_row,
        "slider.range_right",
        state.slider_right,
        (state.slider_left + 1.0)..10000.0,
        right_options,
    );
    slider_number_input(
        ui,
        right_row,
        "slider.right_text",
        &state.slider_right_text,
        FocusedTextInput::SliderRangeRight,
        state,
        96.0,
    );
    widgets::label(
        ui,
        right_row,
        "slider.range.right.label",
        "right",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width(46.0),
    );

    divider(ui, body, "slider.divider.trailing");
    let trailing_row = row(ui, body, "slider.trailing.row", 8.0);
    slider_checkbox_with_layout(
        ui,
        trailing_row,
        "slider.trailing",
        "Trailing color",
        state.slider_trailing_color,
        LayoutStyle::new()
            .with_width(142.0)
            .with_height(30.0)
            .with_flex_shrink(0.0),
    );
    ext_widgets::color_edit_button(
        ui,
        trailing_row,
        "slider.trailing_color_button",
        state.slider_trailing_picker.value(),
        color_square_button_options("slider.trailing_color_button")
            .with_format(ext_widgets::ColorValueFormat::Rgb)
            .accessibility_label("Pick trailing slider color"),
    );
    widgets::label(
        ui,
        trailing_row,
        "slider.trailing_color_button.label",
        "Track color",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width(78.0),
    );
    if state.slider_trailing_picker_open {
        ext_widgets::color_picker(
            ui,
            body,
            "slider.trailing_picker",
            &state.slider_trailing_picker,
            ext_widgets::ColorPickerOptions::default()
                .with_label("Trailing slider color")
                .with_action_prefix("slider.trailing_picker"),
        );
    }
    let thumb_color_row = row(ui, body, "slider.thumb_color.row", 8.0);
    widgets::label(
        ui,
        thumb_color_row,
        "slider.thumb_color.label",
        "Thumb color",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width(142.0),
    );
    ext_widgets::color_edit_button(
        ui,
        thumb_color_row,
        "slider.thumb_color_button",
        state.slider_thumb_picker.value(),
        color_square_button_options("slider.thumb_color_button")
            .with_format(ext_widgets::ColorValueFormat::Rgb)
            .accessibility_label("Pick slider thumb color"),
    );
    if state.slider_thumb_picker_open {
        ext_widgets::color_picker(
            ui,
            body,
            "slider.thumb_picker",
            &state.slider_thumb_picker,
            ext_widgets::ColorPickerOptions::default()
                .with_label("Slider thumb color")
                .with_action_prefix("slider.thumb_picker"),
        );
    }
    let thumb_row = row(ui, body, "slider.thumb.row", 8.0);
    widgets::label(
        ui,
        thumb_row,
        "slider.thumb.label",
        "Thumb",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width(64.0),
    );
    ext_widgets::segmented_control(
        ui,
        thumb_row,
        "slider.thumb",
        &[
            ext_widgets::SegmentedControlItem::new("circle", "Circle"),
            ext_widgets::SegmentedControlItem::new("square", "Square"),
            ext_widgets::SegmentedControlItem::new("rectangle", "Rectangle"),
        ],
        Some(match state.slider_thumb_shape {
            SliderThumbChoice::Circle => "circle",
            SliderThumbChoice::Square => "square",
            SliderThumbChoice::Rectangle => "rectangle",
        }),
        segmented_options("slider.thumb"),
    );
    slider_checkbox(
        ui,
        body,
        "slider.steps",
        "Use steps",
        state.slider_use_steps,
    );
    let step_row = row(ui, body, "slider.step.row", 10.0);
    widgets::label(
        ui,
        step_row,
        "slider.step.label",
        "Step value",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width(74.0),
    );
    slider_number_input(
        ui,
        step_row,
        "slider.step_text",
        &state.slider_step_text,
        FocusedTextInput::SliderStep,
        state,
        86.0,
    );
    slider_checkbox(
        ui,
        body,
        "slider.logarithmic",
        "Logarithmic",
        state.slider_logarithmic,
    );
    let clamp_row = row(ui, body, "slider.clamping.row", 8.0);
    widgets::label(
        ui,
        clamp_row,
        "slider.clamping.label",
        "Clamping",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width(74.0),
    );
    ext_widgets::segmented_control(
        ui,
        clamp_row,
        "slider.clamping",
        &[
            ext_widgets::SegmentedControlItem::new("never", "Never"),
            ext_widgets::SegmentedControlItem::new("edits", "Edits"),
            ext_widgets::SegmentedControlItem::new("always", "Always"),
        ],
        Some(match state.slider_clamping {
            widgets::SliderClamping::Never => "never",
            widgets::SliderClamping::Edits => "edits",
            widgets::SliderClamping::Always => "always",
        }),
        segmented_options("slider.clamping"),
    );
    slider_checkbox(
        ui,
        body,
        "slider.smart_aim",
        "Smart aim",
        state.slider_smart_aim,
    );
}

fn numeric_inputs(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "numeric", "Numeric input");
    let unit_domain = state.numeric_unit_domain();
    let unit_min = unit_domain.min as f32;
    let unit_max = unit_domain.max as f32;
    let unit_span = state.numeric_minimum_span();

    let value_row = row(ui, body, "numeric.value_row", 10.0);
    widgets::label(
        ui,
        value_row,
        "numeric.value_label",
        "Value",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new()
            .with_width(72.0)
            .with_height(30.0)
            .with_flex_shrink(0.0),
    );
    numeric_value_editor(ui, value_row, state);

    let unit_width = 102.0;
    let unit_anchor = ui.add_child(
        value_row,
        UiNode::container(
            "numeric.unit.anchor",
            LayoutStyle::new()
                .with_width(unit_width)
                .with_height(30.0)
                .with_flex_shrink(0.0),
        ),
    );
    let unit_options = numeric_unit_options();
    ext_widgets::dropdown_select(
        ui,
        unit_anchor,
        "numeric.unit",
        &unit_options,
        &state.numeric_unit,
        Some(select_popup(
            UiRect::new(0.0, 0.0, unit_width, 30.0),
            UiRect::new(0.0, 0.0, 320.0, 260.0),
        )),
        dropdown_select_options(unit_width, "numeric.unit", "Unit", "Numeric unit"),
    );

    divider(ui, body, "numeric.range.divider");
    numeric_slider_row(
        ui,
        body,
        "numeric.range_min",
        "Min",
        state.numeric_range_min,
        unit_min..(unit_max - unit_span).max(unit_min),
        Some(&state.numeric_range_min_text),
        Some(FocusedTextInput::NumericRangeMin),
        state,
    );
    numeric_slider_row(
        ui,
        body,
        "numeric.range_max",
        "Max",
        state.numeric_range_max,
        (unit_min + unit_span).min(unit_max)..unit_max,
        Some(&state.numeric_range_max_text),
        Some(FocusedTextInput::NumericRangeMax),
        state,
    );
    numeric_slider_row(
        ui,
        body,
        "numeric.sensitivity",
        "Drag speed",
        state.numeric_sensitivity,
        0.25..4.0,
        None,
        None,
        state,
    );

    widgets::label(
        ui,
        body,
        "numeric.note",
        format!(
            "Range: {} to {} {}",
            state.format_numeric_range_bound(state.numeric_range_min),
            state.format_numeric_range_bound(state.numeric_range_max),
            numeric_unit_label(&state.numeric_unit)
        ),
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn numeric_value_editor(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) -> UiNodeId {
    if state.focused_text == Some(FocusedTextInput::NumericValue) {
        let mut options = state.text_edit_options(FocusedTextInput::NumericValue);
        options.layout = LayoutStyle::new()
            .with_width(150.0)
            .with_height(30.0)
            .with_flex_shrink(0.0);
        options.edit_action = Some("numeric.value.edit".into());
        return widgets::text_input(ui, parent, "numeric.value", &state.numeric_text, options);
    }

    let mut options = widgets::DragValueOptions::default()
        .with_layout(LayoutStyle::new().with_width(150.0).with_height(30.0))
        .with_precision(state.numeric_precision())
        .with_range(state.numeric_range())
        .with_unit(numeric_unit_format(&state.numeric_unit))
        .with_action("numeric.value.drag");
    options.text_style = text(12.0, color(230, 236, 246));
    options.accessibility_label = Some("Numeric value".to_string());
    options.accessibility_hint = Some("Click to edit, or drag horizontally to adjust.".to_string());
    widgets::drag_value_input(
        ui,
        parent,
        "numeric.value",
        f64::from(state.numeric_value),
        options,
    )
}

fn numeric_slider_row(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    value: f32,
    range: std::ops::Range<f32>,
    input: Option<&TextInputState>,
    focused: Option<FocusedTextInput>,
    state: &ShowcaseState,
) {
    let row = row(ui, parent, format!("{name}.row"), 10.0);
    widgets::label(
        ui,
        row,
        format!("{name}.label"),
        label,
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new()
            .with_width(72.0)
            .with_height(28.0)
            .with_flex_shrink(0.0),
    );
    let mut options = widgets::SliderOptions::default()
        .with_layout(
            LayoutStyle::new()
                .with_width(190.0)
                .with_height(24.0)
                .with_flex_shrink(0.0),
        )
        .with_value_edit_action(name);
    options.accessibility_label = Some(label.to_string());
    widgets::slider(ui, row, name, value, range, options);
    if let (Some(input), Some(focused)) = (input, focused) {
        let mut options = state.text_edit_options(focused);
        options.layout = LayoutStyle::new()
            .with_width(70.0)
            .with_height(28.0)
            .with_flex_shrink(0.0);
        options.edit_action = Some(format!("{name}.value.edit").into());
        widgets::text_input(ui, row, format!("{name}.value"), input, options);
    } else {
        widgets::label(
            ui,
            row,
            format!("{name}.value"),
            format!("{:.2}x", value),
            text(12.0, color(230, 236, 246)),
            LayoutStyle::new()
                .with_width(64.0)
                .with_height(28.0)
                .with_flex_shrink(0.0),
        );
    }
}

fn numeric_unit_options() -> Vec<ext_widgets::SelectOption> {
    vec![
        ext_widgets::SelectOption::new("px", "Pixels"),
        ext_widgets::SelectOption::new("deg", "Degrees"),
        ext_widgets::SelectOption::new("turn", "Turns"),
        ext_widgets::SelectOption::new("percent", "Percent"),
    ]
}

fn numeric_unit_id(state: &ext_widgets::SelectMenuState) -> &'static str {
    match state.selected_index().unwrap_or(0) {
        1 => "deg",
        2 => "turn",
        3 => "percent",
        _ => "px",
    }
}

fn numeric_unit_default_range(unit_id: &str) -> ext_widgets::NumericRange {
    match unit_id {
        "deg" => ext_widgets::NumericRange::new(0.0, 360.0),
        "turn" => ext_widgets::NumericRange::new(0.0, 1.0),
        "percent" => ext_widgets::NumericRange::new(0.0, 100.0),
        _ => ext_widgets::NumericRange::new(0.0, 100.0),
    }
}

fn numeric_unit_label(state: &ext_widgets::SelectMenuState) -> &'static str {
    match numeric_unit_id(state) {
        "deg" => "deg",
        "turn" => "turn",
        "percent" => "%",
        _ => "px",
    }
}

fn numeric_unit_format(state: &ext_widgets::SelectMenuState) -> ext_widgets::NumericUnitFormat {
    match numeric_unit_id(state) {
        "deg" => ext_widgets::NumericUnitFormat::default().suffix(" deg"),
        "turn" => ext_widgets::NumericUnitFormat::default().suffix(" turn"),
        "percent" => ext_widgets::NumericUnitFormat::default().suffix("%"),
        _ => ext_widgets::NumericUnitFormat::default().suffix(" px"),
    }
}

fn parse_numeric_edit_text(text: &str, unit: &ext_widgets::NumericUnitFormat) -> Option<f32> {
    let value = unit.strip_affixes(text).parse::<f32>().ok()?;
    value.is_finite().then_some(value)
}

fn selection_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "selection",
        "Select controls",
        UiSize::new(250.0, 0.0),
    );
    let select_width = 220.0;
    let select_options = select_options();

    widgets::label(
        ui,
        body,
        "selection.dropdown.title",
        "Dropdown select",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let dropdown_anchor = ui.add_child(
        body,
        UiNode::container(
            "selection.dropdown.anchor",
            LayoutStyle::new()
                .with_width(select_width)
                .with_height(30.0),
        ),
    );
    ext_widgets::dropdown_select(
        ui,
        dropdown_anchor,
        "selection.dropdown",
        &select_options,
        &state.dropdown,
        Some(select_popup(
            UiRect::new(0.0, 0.0, select_width, 30.0),
            UiRect::new(0.0, 0.0, 320.0, 260.0),
        )),
        dropdown_select_options(
            select_width,
            "selection.dropdown",
            "Select option",
            "Dropdown select",
        ),
    );

    widgets::label(
        ui,
        body,
        "selection.menu.label",
        "Open menu",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    ext_widgets::select_menu(
        ui,
        body,
        "selection.select_menu",
        &select_options,
        &state.select_menu,
        select_menu_options(select_width)
            .with_action_prefix("selection.menu")
            .with_row_height(30.0)
            .with_max_visible_rows(4)
            .with_selected_visual(UiVisual::panel(color(42, 62, 87), None, 2.0))
            .with_active_visual(UiVisual::panel(color(58, 87, 126), None, 2.0)),
    );

    widgets::label(
        ui,
        body,
        "selection.images.label",
        "Image options",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let image_options = select_options_with_images();
    ext_widgets::select_menu(
        ui,
        body,
        "selection.image_menu",
        &image_options,
        &state.image_select_menu,
        select_menu_options(select_width)
            .with_action_prefix("selection.image_menu")
            .with_row_height(32.0)
            .with_max_visible_rows(4)
            .with_image_size(UiSize::new(16.0, 16.0))
            .with_menu_visual(UiVisual::panel(
                color(20, 25, 32),
                Some(StrokeStyle::new(color(77, 90, 111), 1.0)),
                4.0,
            ))
            .with_active_visual(UiVisual::panel(color(59, 70, 94), None, 2.0))
            .with_selected_visual(UiVisual::panel(color(36, 74, 91), None, 2.0)),
    );
}

fn select_menu_options(width: f32) -> ext_widgets::SelectMenuOptions {
    ext_widgets::SelectMenuOptions::default()
        .with_width(width)
        .with_portal(UiPortalTarget::Parent)
        .with_text_style(text(13.0, color(226, 232, 242)))
        .with_disabled_text_style(text(13.0, color(138, 148, 164)))
}

fn dropdown_select_options(
    width: f32,
    action_prefix: &str,
    placeholder: &str,
    accessibility_label: &str,
) -> ext_widgets::DropdownSelectOptions {
    ext_widgets::DropdownSelectOptions::default()
        .with_trigger_layout(
            LayoutStyle::row()
                .with_width(width)
                .with_height(30.0)
                .with_align_items(taffy::prelude::AlignItems::Center)
                .with_justify_content(taffy::prelude::JustifyContent::FlexStart)
                .gap(6.0)
                .padding(6.0),
        )
        .with_text_style(text(13.0, color(226, 232, 242)))
        .with_placeholder(placeholder)
        .with_accessibility_label(accessibility_label)
        .with_menu(select_menu_options(width))
        .with_action_prefix(action_prefix)
}

fn select_popup(anchor: UiRect, viewport: UiRect) -> ext_widgets::AnchoredPopup {
    ext_widgets::AnchoredPopup::new(
        anchor,
        viewport,
        ext_widgets::PopupPlacement::default()
            .with_offset(0.0)
            .with_viewport_margin(0.0),
    )
}

#[allow(clippy::field_reassign_with_default)]
fn text_input(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "text_input", "Text input");
    let mut options = TextInputOptions::default();
    options.placeholder = "Type here".to_string();
    options.layout = LayoutStyle::new().with_width(300.0).with_height(36.0);
    options.text_style = text(13.0, color(230, 236, 246));
    options.placeholder_style = text(13.0, color(144, 156, 174));
    options.edit_action = Some("text.input.edit".into());
    options.focused = state.focused_text == Some(FocusedTextInput::Editable);
    options.caret_visible = caret_visible(state.caret_phase);
    widgets::text_input(ui, body, "text.input", &state.text, options);

    let mut selectable_options = TextInputOptions::default();
    selectable_options.layout = LayoutStyle::new().with_width(360.0).with_height(36.0);
    selectable_options.text_style = text(13.0, color(196, 210, 230));
    selectable_options.read_only = true;
    selectable_options.selectable = true;
    selectable_options.focused = state.focused_text == Some(FocusedTextInput::Selectable);
    selectable_options.edit_action = Some("text.selectable.edit".into());
    selectable_options.caret_visible = caret_visible(state.caret_phase);
    widgets::text_input(
        ui,
        body,
        "text.selectable",
        &state.selectable_text,
        selectable_options,
    );

    let mut singleline = TextInputOptions::default();
    singleline.layout = LayoutStyle::new().with_width(300.0).with_height(34.0);
    singleline.text_style = text(13.0, color(230, 236, 246));
    singleline.placeholder = "Single line".to_string();
    singleline.edit_action = Some("text.singleline.edit".into());
    singleline.focused = state.focused_text == Some(FocusedTextInput::Singleline);
    singleline.caret_visible = caret_visible(state.caret_phase);
    widgets::singleline_text_input(
        ui,
        body,
        "text.singleline",
        &state.singleline_text,
        singleline,
    );

    let mut multiline = TextInputOptions::default();
    multiline.layout = LayoutStyle::new().with_width(360.0).with_height(72.0);
    multiline.text_style = text(13.0, color(230, 236, 246));
    multiline.edit_action = Some("text.multiline.edit".into());
    multiline.focused = state.focused_text == Some(FocusedTextInput::Multiline);
    multiline.caret_visible = caret_visible(state.caret_phase);
    widgets::multiline_text_input(ui, body, "text.multiline", &state.multiline_text, multiline);

    let mut area = TextInputOptions::default();
    area.layout = LayoutStyle::new().with_width(360.0).with_height(66.0);
    area.text_style = text(13.0, color(230, 236, 246));
    area.edit_action = Some("text.area.edit".into());
    area.focused = state.focused_text == Some(FocusedTextInput::TextArea);
    area.caret_visible = caret_visible(state.caret_phase);
    widgets::text_area(ui, body, "text.area", &state.text_area_text, area);

    let mut code = TextInputOptions::default();
    code.layout = LayoutStyle::new().with_width(360.0).with_height(88.0);
    code.edit_action = Some("text.code_editor.edit".into());
    code.focused = state.focused_text == Some(FocusedTextInput::CodeEditor);
    code.caret_visible = caret_visible(state.caret_phase);
    widgets::code_editor(ui, body, "text.code_editor", &state.code_editor_text, code);

    let mut search = TextInputOptions::default();
    search.layout = LayoutStyle::new().with_width(300.0).with_height(34.0);
    search.text_style = text(13.0, color(230, 236, 246));
    search.edit_action = Some("text.search.edit".into());
    search.focused = state.focused_text == Some(FocusedTextInput::Search);
    search.caret_visible = caret_visible(state.caret_phase);
    widgets::search_input(ui, body, "text.search", &state.search_text, search);

    let mut password = TextInputOptions::default();
    password.layout = LayoutStyle::new().with_width(300.0).with_height(34.0);
    password.text_style = text(13.0, color(230, 236, 246));
    password.edit_action = Some("text.password.edit".into());
    password.focused = state.focused_text == Some(FocusedTextInput::Password);
    password.caret_visible = caret_visible(state.caret_phase);
    widgets::password_input(ui, body, "text.password", &state.password_text, password);
}

fn date_picker(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body =
        section_with_min_viewport(ui, parent, "date", "Date picker", UiSize::new(284.0, 0.0));
    let mode = wrapping_row(ui, body, "date.mode_row", 8.0);
    ext_widgets::segmented_control(
        ui,
        mode,
        "date.mode",
        &[
            ext_widgets::SegmentedControlItem::new("single", "Single"),
            ext_widgets::SegmentedControlItem::new("range", "Range"),
            ext_widgets::SegmentedControlItem::new("week", "Week"),
        ],
        Some(match state.date_mode {
            DateDemoMode::Single => "single",
            DateDemoMode::Range => "range",
            DateDemoMode::Week => "week",
        }),
        segmented_options("date.mode"),
    );

    let controls = wrapping_row(ui, body, "date.options", 8.0);
    ext_widgets::segmented_control(
        ui,
        controls,
        "date.week",
        &[
            ext_widgets::SegmentedControlItem::new("sunday", "Sun first"),
            ext_widgets::SegmentedControlItem::new("monday", "Mon first"),
        ],
        Some(match state.date.first_weekday {
            ext_widgets::Weekday::Sunday => "sunday",
            ext_widgets::Weekday::Monday => "monday",
            _ => "sunday",
        }),
        segmented_options("date.week").with_item_layout(
            LayoutStyle::new()
                .with_width(92.0)
                .with_height(28.0)
                .with_flex_shrink(0.0),
        ),
    );
    let bounds = row(ui, body, "date.bounds_options", 8.0);
    let mut bounds_button =
        widgets::ButtonOptions::new(LayoutStyle::new().with_width(92.0).with_height(28.0))
            .with_action("date.bounds.toggle");
    bounds_button.visual = if state.date.min.is_some() || state.date.max.is_some() {
        button_visual(48, 112, 184)
    } else {
        button_visual(38, 46, 58)
    };
    bounds_button.hovered_visual = Some(button_visual(65, 86, 106));
    bounds_button.text_style = text(12.0, color(238, 244, 252));
    widgets::button(
        ui,
        bounds,
        "date.bounds.toggle",
        "May bounds",
        bounds_button,
    );
    let mut clear_options =
        widgets::ButtonOptions::new(LayoutStyle::new().with_width(64.0).with_height(28.0))
            .with_action("date.clear");
    clear_options.visual = button_visual(38, 46, 58);
    clear_options.hovered_visual = Some(button_visual(65, 86, 106));
    clear_options.text_style = text(12.0, color(238, 244, 252));
    widgets::button(ui, bounds, "date.clear", "Clear", clear_options);

    match state.date_mode {
        DateDemoMode::Single => {
            ext_widgets::date_picker(
                ui,
                body,
                "date.picker",
                &state.date,
                ext_widgets::DatePickerOptions::default().with_action_prefix("date"),
            );
        }
        DateDemoMode::Range | DateDemoMode::Week => {
            ext_widgets::date_range_picker(
                ui,
                body,
                "date.picker",
                &state.date_range,
                ext_widgets::DateRangePickerOptions::default().with_action_prefix("date"),
            );
        }
    }
    widgets::label(
        ui,
        body,
        "date.mode_status",
        date_mode_status(state),
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::label(
        ui,
        body,
        "date.selected",
        format!("Selected: {}", date_selection_summary(state)),
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn date_selection_summary(state: &ShowcaseState) -> String {
    match state.date_mode {
        DateDemoMode::Single => state
            .date
            .selected
            .map_or_else(|| "None".to_string(), CalendarDate::iso_string),
        DateDemoMode::Range | DateDemoMode::Week => state.date_range.range.map_or_else(
            || "None".to_string(),
            ext_widgets::CalendarDateRange::iso_string,
        ),
    }
}

fn date_mode_status(state: &ShowcaseState) -> String {
    match state.date_mode {
        DateDemoMode::Single => "Single date".to_string(),
        DateDemoMode::Range => match state.date_range.pending_start {
            Some(start) => format!("Range start: {}", start.iso_string()),
            None => "Custom date range".to_string(),
        },
        DateDemoMode::Week => "Whole-week range".to_string(),
    }
}

fn color_picker(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "color", "Color picker");
    let button_row = row(ui, body, "color.button.row", 8.0);
    widgets::label(
        ui,
        button_row,
        "color.button.label",
        "Button opens color picker",
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new()
            .with_width(0.0)
            .with_flex_grow(1.0)
            .with_flex_shrink(1.0),
    );
    ext_widgets::color_swatch_button(
        ui,
        button_row,
        "color.button.open",
        state.color.value(),
        color_square_button_options("color.button.open").accessibility_label("Open color picker"),
    );
    if state.color_picker_button_open {
        ext_widgets::color_picker(
            ui,
            body,
            "color.button_picker",
            &state.color,
            ext_widgets::ColorPickerOptions::default()
                .with_label("Button color")
                .with_action_prefix("color.button_picker"),
        );
        divider(ui, body, "color.button.divider");
    }
    ext_widgets::color_picker(
        ui,
        body,
        "color.picker",
        &state.color,
        ext_widgets::ColorPickerOptions::default()
            .with_action_prefix("color")
            .with_copy_hex_action("color.copy_hex")
            .with_copy_hex_label("Copy"),
    );
    if let Some(hex) = &state.color_copied_hex {
        widgets::label(
            ui,
            body,
            "color.copied",
            format!("Copied {hex}"),
            text(11.0, color(154, 166, 184)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }
}

fn menu_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "menus",
        "Menu controls",
        UiSize::new(320.0, 0.0),
    );
    let menus = menu_bar_menus(state.menu_autosave, state.menu_grid);
    let active_items = state
        .menu_bar
        .open_menu
        .and_then(|index| menus.get(index))
        .map(|menu| menu.items.clone())
        .unwrap_or_default();
    widgets::label(
        ui,
        body,
        "menus.menu_bar.title",
        "Menu bar",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    ext_widgets::menu_bar(
        ui,
        body,
        "menus.menu_bar",
        &menus,
        &state.menu_bar,
        None,
        ext_widgets::MenuBarOptions::default().with_action_prefix("menus.bar"),
    );

    if !active_items.is_empty() {
        let menu_columns = ui.add_child(
            body,
            UiNode::container(
                "menus.menu_columns",
                Layout::row()
                    .size(LayoutSize::new(
                        LayoutDimension::Auto,
                        LayoutDimension::Auto,
                    ))
                    .align_items(LayoutAlignment::Start)
                    .gap(LayoutGap::points(4.0, 4.0))
                    .flex(0.0, 0.0, LayoutDimension::Auto)
                    .to_layout_style(),
            ),
        );
        ext_widgets::menu_list(
            ui,
            menu_columns,
            "menus.menu_list",
            &active_items,
            state.menu_bar.active_item,
            ext_widgets::MenuListOptions::default().with_action_prefix("menus.item"),
        );
        if let Some(active_item) = state.menu_bar.active_item {
            if let Some(children) = active_items
                .get(active_item)
                .and_then(|item| item.children())
            {
                let submenu_column = ui.add_child(
                    menu_columns,
                    UiNode::container(
                        "menus.submenu_column",
                        Layout::column()
                            .size(LayoutSize::new(
                                LayoutDimension::Auto,
                                LayoutDimension::Auto,
                            ))
                            .gap(LayoutGap::points(0.0, 0.0))
                            .flex(0.0, 0.0, LayoutDimension::Auto)
                            .to_layout_style(),
                    ),
                );
                let offset = menu_item_top_offset(&active_items, active_item);
                if offset > 0.0 {
                    widgets::spacer(
                        ui,
                        submenu_column,
                        "menus.submenu_spacer",
                        LayoutStyle::new().with_width(1.0).with_height(offset),
                    );
                }
                ext_widgets::menu_list(
                    ui,
                    submenu_column,
                    "menus.submenu",
                    children,
                    Some(0),
                    ext_widgets::MenuListOptions::default().with_action_prefix("menus.item"),
                );
            }
        }
    }
    divider(ui, body, "menus.divider.buttons");
    widgets::label(
        ui,
        body,
        "menus.buttons.title",
        "Menu buttons",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let button_row = row(ui, body, "menus.buttons", 10.0);
    let button_items = menu_items(state.menu_autosave);
    ext_widgets::menu_button(
        ui,
        button_row,
        "menus.menu_button",
        "Menu button",
        &button_items,
        &state.menu_button,
        None,
        ext_widgets::MenuButtonOptions::default().with_action("menus.menu_button"),
    );
    ext_widgets::image_text_menu_button(
        ui,
        button_row,
        "menus.image_text_menu_button",
        "Image text",
        icon_image(BuiltInIcon::Folder),
        &button_items,
        &state.image_text_menu_button,
        None,
        ext_widgets::MenuButtonOptions::default().with_action("menus.image_text_menu_button"),
    );
    ext_widgets::image_menu_button(
        ui,
        button_row,
        "menus.image_menu_button",
        icon_image(BuiltInIcon::Settings),
        &button_items,
        &state.image_menu_button,
        None,
        ext_widgets::MenuButtonOptions::default().with_action("menus.image_menu_button"),
    );
    if state.menu_button.open || state.image_text_menu_button.open || state.image_menu_button.open {
        let active = state
            .menu_button
            .navigation
            .active_path
            .first()
            .copied()
            .or_else(|| {
                state
                    .image_text_menu_button
                    .navigation
                    .active_path
                    .first()
                    .copied()
            })
            .or_else(|| {
                state
                    .image_menu_button
                    .navigation
                    .active_path
                    .first()
                    .copied()
            });
        ext_widgets::menu_list(
            ui,
            body,
            "menus.button_menu",
            &button_items,
            active,
            ext_widgets::MenuListOptions::default().with_action_prefix("menus.item"),
        );
    }

    divider(ui, body, "menus.divider.context");
    widgets::label(
        ui,
        body,
        "menus.context.title",
        "Context menu",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let context_row = row(ui, body, "menus.context.controls", 8.0);
    button(
        ui,
        context_row,
        "menus.context.open",
        "Open context",
        "menus.context.open",
        button_visual(48, 112, 184),
    );
    button(
        ui,
        context_row,
        "menus.context.close",
        "Close",
        "menus.context.close",
        button_visual(58, 78, 96),
    );
    let mut context_options =
        ext_widgets::MenuListOptions::default().with_action_prefix("menus.context");
    context_options.width = 240.0;
    context_options.max_visible_rows = 6;
    let _ = ext_widgets::context_menu(
        ui,
        parent,
        "menus.context_menu",
        &button_items,
        &state.context_menu,
        UiRect::new(0.0, 0.0, 560.0, 460.0),
        ext_widgets::PopupPlacement::default(),
        context_options,
    );
}

fn menu_demo_context_anchor() -> UiPoint {
    UiPoint::new(30.0, 390.0)
}

fn command_palette(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "command_palette",
        "Command palette",
        UiSize::new(240.0, 72.0),
    );
    let items = command_palette_items_with_history(&state.command_history);
    let mut trigger_options =
        widgets::ButtonOptions::new(LayoutStyle::new().with_width(150.0).with_height(32.0))
            .with_action(if state.command_palette_open {
                "command_palette.close"
            } else {
                "command_palette.open"
            })
            .with_accessibility_label(if state.command_palette_open {
                "Close command palette"
            } else {
                "Open command palette"
            });
    trigger_options.visual = button_visual(48, 112, 184);
    trigger_options.hovered_visual = Some(button_visual(62, 126, 196));
    trigger_options.pressed_visual = Some(button_visual(38, 82, 136));
    trigger_options.text_style = text(13.0, color(246, 249, 252));
    widgets::button(
        ui,
        body,
        "command_palette.open",
        if state.command_palette_open {
            "Close palette"
        } else {
            "Open palette"
        },
        trigger_options,
    );
    widgets::label(
        ui,
        body,
        "command_palette.last",
        format!("Last command: {}", state.last_command),
        text(12.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    if state.command_palette_open {
        let palette_width = command_palette_popup_width(state.last_desktop_size);
        let mut options =
            ext_widgets::CommandPaletteOptions::default().with_action_prefix("command_palette");
        options.width = palette_width;
        options.row_height = 44.0;
        options.max_visible_rows = 5;
        options.text_style = text(13.0, color(238, 244, 252));
        options.muted_text_style = text(12.0, color(166, 178, 196));
        options.z_index = SHOWCASE_WINDOW_Z_MAX + 40.0;
        ext_widgets::command_palette(
            ui,
            body,
            "command_palette.panel",
            &items,
            &state.command_palette,
            Some(command_palette_popup(
                state.last_desktop_size,
                palette_width,
            )),
            options,
        );
    }
}

fn command_palette_popup_width(desktop_size: UiSize) -> f32 {
    (desktop_size.width - 48.0).clamp(320.0, 560.0)
}

fn command_palette_popup(desktop_size: UiSize, width: f32) -> ext_widgets::AnchoredPopup {
    let viewport = UiRect::new(0.0, 0.0, desktop_size.width, desktop_size.height);
    let x = ((desktop_size.width - width) * 0.5).max(12.0);
    let y = (desktop_size.height * 0.12).clamp(48.0, 96.0);
    ext_widgets::AnchoredPopup::new(
        UiRect::new(x, y, width, 0.0),
        viewport,
        ext_widgets::PopupPlacement::new(
            ext_widgets::PopupSide::Bottom,
            ext_widgets::PopupAlign::Center,
        )
        .with_offset(0.0)
        .with_flip(false)
        .with_viewport_margin(12.0),
    )
}

#[allow(clippy::field_reassign_with_default)]
fn progress_indicator(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "progress", "Progress indicator");
    let animated = smooth_loop(state.progress_phase * 0.85, 0.0) * 100.0;
    let mut progress = ext_widgets::ProgressIndicatorOptions::default();
    progress.layout = LayoutStyle::new().with_width_percent(1.0).with_height(10.0);
    progress.accessibility_label = Some("Progress".to_string());
    ext_widgets::progress_indicator(
        ui,
        body,
        "progress.primary",
        ext_widgets::ProgressIndicatorValue::percent(animated),
        progress,
    );
    let compact_value = smooth_loop(state.progress_phase * 1.15, 0.7) * 100.0;
    let mut compact = ext_widgets::ProgressIndicatorOptions::default();
    compact.layout = LayoutStyle::new().with_width_percent(1.0).with_height(6.0);
    compact.fill_visual = UiVisual::panel(color(111, 203, 159), None, 3.0);
    ext_widgets::progress_indicator(
        ui,
        body,
        "progress.compact",
        ext_widgets::ProgressIndicatorValue::percent(compact_value),
        compact,
    );
    let warning_value = smooth_loop(state.progress_phase * 0.65, 1.4) * 100.0;
    let mut warning = ext_widgets::ProgressIndicatorOptions::default();
    warning.layout = LayoutStyle::new().with_width_percent(1.0).with_height(14.0);
    warning.fill_visual = UiVisual::panel(color(232, 186, 88), None, 4.0);
    ext_widgets::progress_indicator(
        ui,
        body,
        "progress.warning",
        ext_widgets::ProgressIndicatorValue::percent(warning_value),
        warning,
    );
    let logged_value =
        (state.progress_loading_elapsed / PROGRESS_LOGGED_DURATION_SECONDS * 100.0).min(100.0);
    let logged_entries = progress_demo_logs(logged_value);
    progress_loading_panel(
        ui,
        body,
        "progress.logged",
        logged_value,
        &logged_entries,
        state,
    );
    let spinner_row = row(ui, body, "progress.spinner.row", 8.0);
    widgets::spinner(
        ui,
        spinner_row,
        "progress.spinner",
        widgets::SpinnerOptions::default()
            .with_phase(state.progress_phase)
            .with_accessibility_label("Loading spinner"),
    );
    widgets::label(
        ui,
        spinner_row,
        "progress.spinner.label",
        "Spinner",
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn progress_loading_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    progress_value: f32,
    logs: &[ext_widgets::ProgressLogEntry],
    state: &ShowcaseState,
) {
    let mut progress = ext_widgets::ProgressIndicatorOptions::default();
    progress.layout = LayoutStyle::new()
        .with_width_percent(1.0)
        .with_height(10.0)
        .with_flex_grow(1.0)
        .with_flex_shrink(1.0);
    progress.fill_visual = UiVisual::panel(color(111, 203, 159), None, 3.0);
    progress.accessibility_label = Some("Logged loading progress".to_string());
    let mut reset = widgets::ButtonOptions::new(
        LayoutStyle::new()
            .with_width(76.0)
            .with_height(30.0)
            .with_flex_shrink(0.0),
    )
    .with_action("progress.logged.reset");
    reset.visual = button_visual(38, 46, 58);
    reset.hovered_visual = Some(button_visual(65, 86, 106));
    reset.pressed_visual = Some(button_visual(34, 54, 84));
    reset.text_style = text(12.0, color(238, 244, 252));

    let log_scroll = progress_log_scroll_state(
        state.progress_logs_scroll.offset().y,
        logs.len(),
        state.progress_logs_follow_tail,
    );
    let mut panel_options = ext_widgets::ProgressLogPanelOptions::default()
        .with_accessibility_label("Loading progress with logs")
        .with_log_scroll(log_scroll)
        .with_log_scroll_action("progress.logged.logs.scroll")
        .with_trailing_action(
            ext_widgets::ProgressLogPanelAction::new("Reset", "progress.logged.reset")
                .with_options(reset),
        );
    panel_options.layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_padding(10.0)
        .with_gap(8.0)
        .with_flex_shrink(0.0);
    panel_options.visual = UiVisual::panel(
        color(17, 21, 27),
        Some(StrokeStyle::new(color(70, 82, 101), 1.0)),
        4.0,
    );
    panel_options.progress_options = progress;
    panel_options.log_viewport_layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_height(PROGRESS_LOG_VIEWPORT_HEIGHT)
        .with_flex_shrink(0.0);
    panel_options.log_visual = UiVisual::panel(
        color(11, 15, 21),
        Some(StrokeStyle::new(color(45, 57, 73), 1.0)),
        3.0,
    );
    panel_options.log_row_height = PROGRESS_LOG_ROW_HEIGHT;
    panel_options.empty_text_style = text(12.0, color(154, 166, 184));
    panel_options.log_text_style = text(12.0, color(196, 210, 230));
    panel_options.log_text_style.line_height = 18.0;

    ext_widgets::progress_log_panel(
        ui,
        parent,
        name,
        ext_widgets::ProgressIndicatorValue::percent(progress_value),
        logs,
        panel_options,
    );
}

fn progress_log_scroll_state(
    saved_offset_y: f32,
    log_count: usize,
    follow_tail: bool,
) -> operad::ScrollState {
    let content_height = log_count.max(1) as f32 * PROGRESS_LOG_ROW_HEIGHT;
    let max_offset = (content_height - PROGRESS_LOG_VIEWPORT_HEIGHT).max(0.0);
    let offset_y = if follow_tail {
        max_offset
    } else {
        saved_offset_y.min(max_offset)
    };
    operad::ScrollState::new(ScrollAxes::VERTICAL)
        .with_sizes(
            UiSize::new(8.0, PROGRESS_LOG_VIEWPORT_HEIGHT),
            UiSize::new(8.0, content_height),
        )
        .with_offset(UiPoint::new(0.0, offset_y))
}

fn progress_demo_logs(progress: f32) -> Vec<ext_widgets::ProgressLogEntry> {
    let mut logs = vec![
        ext_widgets::ProgressLogEntry::info("Initializing renderer"),
        ext_widgets::ProgressLogEntry::info("Mounting content archive"),
    ];
    if progress >= 24.0 {
        logs.push(ext_widgets::ProgressLogEntry::success(
            "Compiled material shaders",
        ));
    }
    if progress >= 48.0 {
        logs.push(ext_widgets::ProgressLogEntry::info("Decoded texture atlas"));
    }
    if progress >= 72.0 {
        logs.push(ext_widgets::ProgressLogEntry::warning(
            "Optional cloud profile is still pending",
        ));
    }
    if progress >= 96.0 {
        logs.push(ext_widgets::ProgressLogEntry::success("Ready"));
    }
    logs
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EaseCurveKind {
    Quad,
    Cubic,
    Quart,
    Expo,
    Back,
    Elastic,
    Bounce,
}

impl EaseCurveKind {
    fn id(self) -> &'static str {
        match self {
            Self::Quad => "quad",
            Self::Cubic => "cubic",
            Self::Quart => "quart",
            Self::Expo => "expo",
            Self::Back => "back",
            Self::Elastic => "elastic",
            Self::Bounce => "bounce",
        }
    }

    fn base_label(self) -> &'static str {
        match self {
            Self::Quad => "quad",
            Self::Cubic => "cubic",
            Self::Quart => "quart",
            Self::Expo => "expo",
            Self::Back => "back",
            Self::Elastic => "elastic",
            Self::Bounce => "bounce",
        }
    }

    fn sample_out(self, progress: f32) -> f32 {
        let t = unit(progress);
        match self {
            Self::Quad => 1.0 - (1.0 - t).powi(2),
            Self::Cubic => 1.0 - (1.0 - t).powi(3),
            Self::Quart => 1.0 - (1.0 - t).powi(4),
            Self::Expo => {
                if t >= 1.0 {
                    1.0
                } else {
                    1.0 - 2.0_f32.powf(-10.0 * t)
                }
            }
            Self::Back => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            Self::Elastic => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    let period = (2.0 * std::f32::consts::PI) / 3.0;
                    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * period).sin() + 1.0
                }
            }
            Self::Bounce => ease_out_bounce(t),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EaseDirection {
    In,
    Out,
}

impl EaseDirection {
    fn label_prefix(self) -> &'static str {
        match self {
            Self::In => "Ease in",
            Self::Out => "Ease out",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EasingFunction {
    direction: EaseDirection,
    kind: EaseCurveKind,
}

impl EasingFunction {
    const fn new(direction: EaseDirection, kind: EaseCurveKind) -> Self {
        Self { direction, kind }
    }

    fn label(self) -> String {
        format!(
            "{} {}",
            self.direction.label_prefix(),
            self.kind.base_label()
        )
    }

    fn sample(self, progress: f32) -> f32 {
        let t = unit(progress);
        match self.direction {
            EaseDirection::In => 1.0 - self.kind.sample_out(1.0 - t),
            EaseDirection::Out => self.kind.sample_out(t),
        }
    }
}

fn ease_out_bounce(t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

fn easing_options(direction: EaseDirection) -> Vec<ext_widgets::SelectOption> {
    [
        EaseCurveKind::Quad,
        EaseCurveKind::Cubic,
        EaseCurveKind::Quart,
        EaseCurveKind::Expo,
        EaseCurveKind::Back,
        EaseCurveKind::Elastic,
        EaseCurveKind::Bounce,
    ]
    .into_iter()
    .map(|kind| {
        ext_widgets::SelectOption::new(kind.id(), EasingFunction::new(direction, kind).label())
    })
    .collect()
}

fn selected_easing(
    state: &ext_widgets::SelectMenuState,
    direction: EaseDirection,
) -> EasingFunction {
    let options = easing_options(direction);
    let kind = match state.selected_id(&options) {
        Some("quad") => EaseCurveKind::Quad,
        Some("quart") => EaseCurveKind::Quart,
        Some("expo") => EaseCurveKind::Expo,
        Some("back") => EaseCurveKind::Back,
        Some("elastic") => EaseCurveKind::Elastic,
        Some("bounce") => EaseCurveKind::Bounce,
        _ => EaseCurveKind::Cubic,
    };
    EasingFunction::new(direction, kind)
}

fn animation_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "animation", "Animation");

    if let Some(section) = animation_section(
        ui,
        body,
        "animation.timed",
        "Timed playback",
        state.animation_timed_expanded,
    ) {
        let live_stage = animation_stage(ui, section, "animation.live.stage");
        let live_amount = smooth_loop(state.progress_phase * 1.65, 0.0);
        let live_values = animation_blend_machine(
            ANIMATION_INPUT_PROGRESS,
            live_amount,
            UiPoint::new(220.0, 0.0),
            0.88,
            1.10,
            1.0,
        )
        .with_bool_input("looping", true)
        .values();
        ui.add_child(
            live_stage,
            UiNode::scene(
                "animation.live.orb",
                animation_orb_primitives(
                    color(108, 180, 255),
                    ANIMATION_ORB_SIZE * live_values.scale,
                    UiPoint::new(
                        28.0 + live_values.translate.x,
                        37.0 + live_values.translate.y,
                    ),
                ),
                animation_scene_layout(),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Image).label("Looping orb"),
            ),
        );
    }

    if let Some(section) = animation_section(
        ui,
        body,
        "animation.scrub",
        "Scrubbed input",
        state.animation_scrub_expanded,
    ) {
        let scrub_row = row(ui, section, "animation.scrub.row", 10.0);
        widgets::slider(
            ui,
            scrub_row,
            "animation.scrub.slider",
            state.animation_scrub,
            0.0..1.0,
            widgets::SliderOptions::default()
                .with_layout(
                    LayoutStyle::new()
                        .with_width(200.0)
                        .with_height(28.0)
                        .with_flex_shrink(0.0),
                )
                .with_value_edit_action("animation.scrub"),
        );
        widgets::label(
            ui,
            scrub_row,
            "animation.scrub.value",
            format!("{:.0}%", state.animation_scrub * 100.0),
            text(12.0, color(186, 198, 216)),
            LayoutStyle::new().with_width_percent(1.0),
        );
        let scrub_stage = animation_stage(ui, section, "animation.scrub.stage");
        let scrub_values = animation_blend_machine(
            ANIMATION_INPUT_SCRUB,
            state.animation_scrub,
            UiPoint::new(220.0, 0.0),
            0.82,
            1.14,
            1.0,
        )
        .values();
        ui.add_child(
            scrub_stage,
            UiNode::scene(
                "animation.scrub.shape",
                animation_morph_shape_primitives(
                    color(111, 203, 159),
                    ANIMATION_SHAPE_SIZE * scrub_values.scale,
                    UiPoint::new(
                        28.0 + scrub_values.translate.x,
                        37.0 + scrub_values.translate.y,
                    ),
                    scrub_values.morph,
                ),
                animation_scene_layout(),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Image).label("Scrubbed morphing shape"),
            ),
        );
    }

    if let Some(section) = animation_section(
        ui,
        body,
        "animation.state",
        "Boolean input transition",
        state.animation_state_expanded,
    ) {
        let state_row = row(ui, section, "animation.state.row", 10.0);
        let mut open = widgets::ButtonOptions::new(
            LayoutStyle::new()
                .with_width(92.0)
                .with_height(30.0)
                .with_flex_shrink(0.0),
        )
        .with_action("animation.open");
        open.visual = if state.animation_open {
            button_visual(48, 112, 184)
        } else {
            button_visual(38, 46, 58)
        };
        open.hovered_visual = Some(button_visual(65, 86, 106));
        open.pressed_visual = Some(button_visual(34, 54, 84));
        open.text_style = text(12.0, color(238, 244, 252));
        widgets::button(
            ui,
            state_row,
            "animation.open",
            if state.animation_open {
                "Close"
            } else {
                "Open"
            },
            open,
        );
        let open_stage = animation_stage(ui, section, "animation.state.stage");
        let panel_offset = if state.animation_open {
            UiPoint::new(
                ANIMATION_STAGE_MIN_WIDTH - ANIMATION_PANEL_WIDTH - ANIMATION_PANEL_INSET_X,
                ANIMATION_PANEL_Y,
            )
        } else {
            UiPoint::new(ANIMATION_PANEL_INSET_X, ANIMATION_PANEL_Y)
        };
        ui.add_child(
            open_stage,
            UiNode::scene(
                "animation.state.panel",
                animation_panel_primitives(panel_offset),
                animation_scene_layout(),
            )
            .with_animation(animation_open_machine(state.animation_open))
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Image).label("Open state panel"),
            ),
        );
    }

    if let Some(section) = animation_section(
        ui,
        body,
        "animation.interaction",
        "Interaction inputs",
        state.animation_interaction_expanded,
    ) {
        let interaction_stage = animation_stage(ui, section, "animation.interaction.stage");
        ui.add_child(
            interaction_stage,
            UiNode::scene(
                "animation.interaction.target",
                animation_interaction_primitives(
                    color(176, 126, 230),
                    ANIMATION_ORB_SIZE,
                    UiPoint::new(40.0, 37.0),
                ),
                animation_scene_layout(),
            )
            .with_input(InputBehavior::BUTTON)
            .with_animation(animation_interaction_machine())
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label("Interaction animation target")
                    .focusable(),
            ),
        );
    }
}

fn easing_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "easing", "Easing");
    let linear_progress = (state.progress_phase * 0.25).rem_euclid(1.0);
    easing_curve_demo(
        ui,
        body,
        "easing.in",
        "Ease-in functions",
        EaseDirection::In,
        &state.easing_in,
        linear_progress,
    );
    divider(ui, body, "easing.divider");
    easing_curve_demo(
        ui,
        body,
        "easing.out",
        "Ease-out functions",
        EaseDirection::Out,
        &state.easing_out,
        linear_progress,
    );
}

fn easing_curve_demo(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    title: &'static str,
    direction: EaseDirection,
    state: &ext_widgets::SelectMenuState,
    linear_progress: f32,
) {
    widgets::label(
        ui,
        parent,
        format!("{name}.title"),
        title,
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let options = easing_options(direction);
    let selected = selected_easing(state, direction);
    let eased_progress = selected.sample(linear_progress);
    let controls = row(ui, parent, format!("{name}.controls"), 10.0);
    let dropdown_width = 184.0;
    let dropdown_name = format!("{name}.dropdown");
    let dropdown_anchor = ui.add_child(
        controls,
        UiNode::container(
            format!("{name}.dropdown.anchor"),
            LayoutStyle::new()
                .with_width(dropdown_width)
                .with_height(30.0)
                .with_flex_shrink(0.0),
        ),
    );
    ext_widgets::dropdown_select(
        ui,
        dropdown_anchor,
        dropdown_name.clone(),
        &options,
        state,
        Some(select_popup(
            UiRect::new(0.0, 0.0, dropdown_width, 30.0),
            UiRect::new(0.0, 0.0, EASING_STAGE_MIN_WIDTH, 260.0),
        )),
        dropdown_select_options(
            dropdown_width,
            dropdown_name.as_str(),
            "Ease function",
            title,
        ),
    );
    widgets::label(
        ui,
        controls,
        format!("{name}.value"),
        format!(
            "{:.0}% -> {:.0}%",
            linear_progress * 100.0,
            eased_progress * 100.0
        ),
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let stage = easing_stage(ui, parent, format!("{name}.stage"));
    ui.add_child(
        stage,
        UiNode::scene(
            format!("{name}.graph"),
            easing_curve_primitives(selected, linear_progress),
            animation_scene_layout(),
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Image)
                .label(format!("{} curve and looping marker", selected.label())),
        ),
    );
}

fn animation_section(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    title: &'static str,
    expanded: bool,
) -> Option<UiNodeId> {
    let mut options = widgets::CollapsingHeaderOptions::default()
        .expanded(expanded)
        .with_toggle_action(format!("{name}.toggle"));
    options.text_style = text(12.0, color(220, 228, 238));
    options.indicator_text_style = text(12.0, color(186, 198, 216));
    options.root_visual = UiVisual::panel(
        color(17, 22, 29),
        Some(StrokeStyle::new(color(48, 58, 72), 1.0)),
        6.0,
    );
    options.header_visual = UiVisual::panel(color(21, 26, 33), None, 0.0);
    options.hovered_visual = UiVisual::panel(color(38, 48, 61), None, 0.0);
    options.pressed_visual = UiVisual::panel(color(27, 36, 48), None, 0.0);
    options.body_layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_padding(10.0)
        .with_gap(10.0);
    widgets::collapsing_header(ui, parent, name, title, options).body
}

fn animation_stage(ui: &mut UiDocument, parent: UiNodeId, name: impl Into<String>) -> UiNodeId {
    let layout = LayoutStyle::row()
        .with_width_percent(1.0)
        .with_height(ANIMATION_STAGE_HEIGHT)
        .with_align_items(taffy::prelude::AlignItems::Center)
        .with_flex_shrink(0.0);
    let layout = operad::layout::with_min_size(
        layout,
        operad::length(ANIMATION_STAGE_MIN_WIDTH),
        operad::length(ANIMATION_STAGE_HEIGHT),
    );
    ui.add_child(
        parent,
        UiNode::container(name, layout).with_visual(UiVisual::panel(
            color(16, 21, 28),
            Some(StrokeStyle::new(color(48, 58, 72), 1.0)),
            6.0,
        )),
    )
}

fn easing_stage(ui: &mut UiDocument, parent: UiNodeId, name: impl Into<String>) -> UiNodeId {
    let layout = LayoutStyle::row()
        .with_width_percent(1.0)
        .with_height(EASING_STAGE_HEIGHT)
        .with_align_items(taffy::prelude::AlignItems::Center)
        .with_flex_shrink(0.0);
    let layout = operad::layout::with_min_size(
        layout,
        operad::length(EASING_STAGE_MIN_WIDTH),
        operad::length(EASING_STAGE_HEIGHT),
    );
    ui.add_child(
        parent,
        UiNode::container(name, layout).with_visual(UiVisual::panel(
            color(16, 21, 28),
            Some(StrokeStyle::new(color(48, 58, 72), 1.0)),
            6.0,
        )),
    )
}

fn animation_scene_layout() -> LayoutStyle {
    let layout = LayoutStyle::new()
        .with_width_percent(1.0)
        .with_height_percent(1.0)
        .with_flex_grow(1.0)
        .with_flex_shrink(1.0);
    operad::layout::with_min_size(layout, operad::length(0.0), operad::length(0.0))
}

fn easing_curve_primitives(function: EasingFunction, linear_progress: f32) -> Vec<ScenePrimitive> {
    let mut primitives = Vec::new();
    let graph = UiRect::new(24.0, 24.0, 172.0, 112.0);
    primitives.push(ScenePrimitive::Rect(
        PaintRect::solid(graph, color(12, 17, 24))
            .stroke(AlignedStroke::inside(StrokeStyle::new(
                color(58, 70, 88),
                1.0,
            )))
            .corner_radii(CornerRadii::uniform(4.0)),
    ));
    for index in 1..4 {
        let fraction = index as f32 / 4.0;
        let x = graph.x + graph.width * fraction;
        let y = graph.y + graph.height * fraction;
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(x, graph.y),
            to: UiPoint::new(x, graph.y + graph.height),
            stroke: StrokeStyle::new(color(29, 38, 50), 1.0),
        });
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(graph.x, y),
            to: UiPoint::new(graph.x + graph.width, y),
            stroke: StrokeStyle::new(color(29, 38, 50), 1.0),
        });
    }
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(graph.x, graph.y + graph.height),
        to: UiPoint::new(graph.x + graph.width, graph.y + graph.height),
        stroke: StrokeStyle::new(color(118, 136, 162), 1.0),
    });
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(graph.x, graph.y),
        to: UiPoint::new(graph.x, graph.y + graph.height),
        stroke: StrokeStyle::new(color(118, 136, 162), 1.0),
    });

    let samples = 40;
    let mut previous = None;
    for index in 0..=samples {
        let t = index as f32 / samples as f32;
        let eased = function.sample(t);
        let point = UiPoint::new(
            graph.x + graph.width * t,
            graph.y + graph.height - graph.height * eased.clamp(-0.16, 1.16),
        );
        if let Some(from) = previous {
            primitives.push(ScenePrimitive::Line {
                from,
                to: point,
                stroke: StrokeStyle::new(color(112, 181, 255), 2.0),
            });
        }
        previous = Some(point);
    }

    let eased_progress = function.sample(linear_progress);
    let graph_marker = UiPoint::new(
        graph.x + graph.width * linear_progress,
        graph.y + graph.height - graph.height * eased_progress.clamp(-0.16, 1.16),
    );
    primitives.push(ScenePrimitive::Circle {
        center: graph_marker,
        radius: 5.5,
        fill: color(248, 252, 255),
        stroke: Some(StrokeStyle::new(color(112, 181, 255), 2.0)),
    });

    let track = UiRect::new(232.0, 64.0, 96.0, 12.0);
    let marker_progress = eased_progress.clamp(-0.10, 1.10);
    primitives.push(ScenePrimitive::Rect(
        PaintRect::solid(track, color(37, 46, 58)).corner_radii(CornerRadii::uniform(6.0)),
    ));
    primitives.push(ScenePrimitive::Rect(
        PaintRect::solid(
            UiRect::new(
                track.x,
                track.y,
                track.width * eased_progress.clamp(0.0, 1.0),
                track.height,
            ),
            color(108, 180, 255),
        )
        .corner_radii(CornerRadii::uniform(6.0)),
    ));
    primitives.push(ScenePrimitive::Circle {
        center: UiPoint::new(
            track.x + track.width * marker_progress,
            track.y + track.height * 0.5,
        ),
        radius: 14.0,
        fill: color(112, 181, 255),
        stroke: Some(StrokeStyle::new(color(232, 242, 255), 2.0)),
    });
    primitives.push(ScenePrimitive::Text(
        PaintText::new(
            function.label(),
            UiRect::new(222.0, 98.0, 120.0, 20.0),
            text(10.0, color(186, 198, 216)),
        )
        .horizontal_align(TextHorizontalAlign::Center)
        .multiline(false),
    ));
    primitives
}

fn animation_blend_machine(
    input: &'static str,
    value: f32,
    translate: UiPoint,
    start_scale: f32,
    end_scale: f32,
    end_opacity: f32,
) -> AnimationMachine {
    let start_values = AnimatedValues::new(0.45, UiPoint::new(0.0, 0.0), start_scale);
    let end_values = AnimatedValues::new(end_opacity, translate, end_scale).with_morph(1.0);
    AnimationMachine::new(
        vec![
            AnimationState::new("start", start_values),
            AnimationState::new("end", end_values),
        ],
        Vec::new(),
        "start",
    )
    .unwrap_or_else(|_| AnimationMachine::single_state("start", start_values))
    .with_number_input(input, value)
    .with_blend_binding(AnimationBlendBinding::new(input, "start", "end"))
}

fn animation_open_machine(open: bool) -> AnimationMachine {
    let closed_values = AnimatedValues::new(0.35, UiPoint::new(0.0, 0.0), 1.0);
    let open_values = AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0);
    let fallback_values = if open { open_values } else { closed_values };
    AnimationMachine::new(
        vec![
            AnimationState::new("closed", closed_values),
            AnimationState::new("open", open_values),
        ],
        vec![
            AnimationTransition::when(
                "closed",
                "open",
                AnimationCondition::bool(ANIMATION_INPUT_OPEN, true),
                0.18,
            ),
            AnimationTransition::when(
                "open",
                "closed",
                AnimationCondition::bool(ANIMATION_INPUT_OPEN, false),
                0.14,
            ),
        ],
        "closed",
    )
    .unwrap_or_else(|_| AnimationMachine::single_state("closed", fallback_values))
    .with_bool_input(ANIMATION_INPUT_OPEN, open)
}

fn animation_interaction_machine() -> AnimationMachine {
    let rest_values = AnimatedValues::new(0.72, UiPoint::new(0.0, 0.0), 1.0);
    let right_values = AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0).with_morph(1.0);
    AnimationMachine::new(
        vec![
            AnimationState::new("rest", rest_values),
            AnimationState::new("right", right_values),
        ],
        Vec::new(),
        "rest",
    )
    .unwrap_or_else(|_| AnimationMachine::single_state("rest", rest_values))
    .with_number_input(ANIMATION_INPUT_POINTER_NORM_X, 0.0)
    .with_blend_binding(AnimationBlendBinding::new(
        ANIMATION_INPUT_POINTER_NORM_X,
        "rest",
        "right",
    ))
}

fn animation_interaction_primitives(
    fill: ColorRgba,
    size: f32,
    offset: UiPoint,
) -> Vec<ScenePrimitive> {
    vec![
        ScenePrimitive::MorphPolygon {
            from_points: animation_square_points(size, offset),
            to_points: animation_pentagon_points(size, offset),
            amount: 0.0,
            fill,
            stroke: Some(StrokeStyle::new(color(236, 244, 255), 1.0)),
        },
        ScenePrimitive::Circle {
            center: UiPoint::new(offset.x + size * 0.34, offset.y + size * 0.30),
            radius: size * 0.10,
            fill: color(244, 248, 255),
            stroke: None,
        },
    ]
}

fn animation_orb_primitives(fill: ColorRgba, size: f32, offset: UiPoint) -> Vec<ScenePrimitive> {
    let center = size * 0.5;
    let radius = size * 0.44;
    vec![
        ScenePrimitive::Circle {
            center: UiPoint::new(offset.x + center, offset.y + center),
            radius,
            fill,
            stroke: Some(StrokeStyle::new(color(236, 244, 255), 1.0)),
        },
        ScenePrimitive::Circle {
            center: UiPoint::new(offset.x + size * 0.34, offset.y + size * 0.30),
            radius: size * 0.12,
            fill: color(244, 248, 255),
            stroke: None,
        },
    ]
}

fn animation_morph_shape_primitives(
    fill: ColorRgba,
    size: f32,
    offset: UiPoint,
    amount: f32,
) -> Vec<ScenePrimitive> {
    vec![ScenePrimitive::MorphPolygon {
        from_points: animation_square_points(size, offset),
        to_points: animation_pentagon_points(size, offset),
        amount,
        fill,
        stroke: Some(StrokeStyle::new(color(226, 246, 236), 1.0)),
    }]
}

fn animation_square_points(size: f32, offset: UiPoint) -> Vec<UiPoint> {
    let inset = size * 0.08;
    let left = offset.x + inset;
    let top = offset.y + inset;
    let right = offset.x + size - inset;
    let bottom = offset.y + size - inset;
    let center_x = offset.x + size * 0.5;
    vec![
        UiPoint::new(center_x, top),
        UiPoint::new(right, top),
        UiPoint::new(right, bottom),
        UiPoint::new(left, bottom),
        UiPoint::new(left, top),
    ]
}

fn animation_pentagon_points(size: f32, offset: UiPoint) -> Vec<UiPoint> {
    let center = size * 0.5;
    let radius = size * 0.46;
    (0..5)
        .map(|index| {
            let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::TAU / 5.0;
            UiPoint::new(
                offset.x + center + angle.cos() * radius,
                offset.y + center + angle.sin() * radius,
            )
        })
        .collect()
}

fn animation_panel_primitives(offset: UiPoint) -> Vec<ScenePrimitive> {
    vec![ScenePrimitive::Rect(
        PaintRect::solid(
            UiRect::new(
                offset.x,
                offset.y,
                ANIMATION_PANEL_WIDTH,
                ANIMATION_PANEL_HEIGHT,
            ),
            color(232, 186, 88),
        )
        .stroke(AlignedStroke::inside(StrokeStyle::new(
            color(255, 226, 154),
            1.0,
        )))
        .corner_radii(CornerRadii::uniform(6.0)),
    )]
}

fn list_and_table_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "lists_tables",
        "Lists and tables",
        UiSize::new(520.0, 0.0),
    );

    let list_row = ui.add_child(
        body,
        UiNode::container(
            "lists_tables.list_row",
            Layout::row()
                .size(LayoutSize::new(
                    LayoutDimension::percent(1.0),
                    LayoutDimension::Auto,
                ))
                .gap(LayoutGap::points(10.0, 10.0))
                .flex_wrap(LayoutFlexWrap::Wrap)
                .to_layout_style(),
        ),
    );
    let scroll_column = ui.add_child(
        list_row,
        UiNode::container(
            "lists_tables.scroll_area.column",
            Layout::column()
                .min_size(LayoutSize::points(220.0, 0.0))
                .gap(LayoutGap::points(6.0, 6.0))
                .flex(1.0, 1.0, LayoutDimension::points(245.0))
                .to_layout_style(),
        ),
    );
    widgets::label(
        ui,
        scroll_column,
        "lists_tables.scroll_area.title",
        "Scrollable list",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let nested_scroll = widgets::scroll_area_with_options(
        ui,
        scroll_column,
        "lists_tables.scroll_area",
        widgets::ScrollAreaOptions::new(
            ScrollAxes::VERTICAL,
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(104.0),
        )
        .with_scroll(
            operad::ScrollState::new(ScrollAxes::VERTICAL)
                .with_offset(UiPoint::new(0.0, state.list_scroll)),
        )
        .with_action("lists_tables.scroll_area.scroll"),
    );
    for index in 0..6 {
        widgets::label(
            ui,
            nested_scroll,
            format!("lists_tables.scroll_area.row.{index}"),
            format!("Scroll row {}", index + 1),
            text(12.0, color(200, 212, 228)),
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(26.0)
                .with_flex_shrink(0.0),
        );
    }

    let virtual_list_column = ui.add_child(
        list_row,
        UiNode::container(
            "lists_tables.virtual_list.column",
            Layout::column()
                .min_size(LayoutSize::points(220.0, 0.0))
                .gap(LayoutGap::points(6.0, 6.0))
                .flex(1.0, 1.0, LayoutDimension::points(245.0))
                .to_layout_style(),
        ),
    );

    widgets::label(
        ui,
        virtual_list_column,
        "lists_tables.virtual_list.title",
        "Virtualized list",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let virtual_list = widgets::virtual_list(
        ui,
        virtual_list_column,
        "lists_tables.virtual_list",
        widgets::VirtualListSpec {
            row_count: 24,
            row_height: 28.0,
            viewport_height: 104.0,
            scroll_offset: state.virtual_scroll,
            overscan: 1,
        },
        |ui, row_parent, row| {
            widgets::label(
                ui,
                row_parent,
                format!("lists_tables.virtual_list.row.{row}"),
                format!("Virtual row {}", row + 1),
                text(12.0, color(214, 224, 238)),
                LayoutStyle::new()
                    .with_width_percent(1.0)
                    .with_height(28.0)
                    .with_flex_shrink(0.0),
            );
        },
    );
    ui.node_mut(virtual_list)
        .set_action("lists_tables.virtual_list.scroll");

    widgets::separator(
        ui,
        body,
        "lists_tables.virtualized_table.separator",
        widgets::SeparatorOptions::default(),
    );
    widgets::label(
        ui,
        body,
        "lists_tables.data_table.title",
        "Virtualized selectable table",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let virtual_controls = wrapping_row(ui, body, "lists_tables.virtualized_table.controls", 8.0);
    button(
        ui,
        virtual_controls,
        "lists_tables.virtualized_table.sort.name",
        if state.virtual_table_descending {
            "Name desc"
        } else {
            "Name asc"
        },
        "lists_tables.virtualized_table.sort.name",
        button_visual(38, 52, 70),
    );
    button(
        ui,
        virtual_controls,
        "lists_tables.virtualized_table.filter.status",
        if state.virtual_table_ready_only {
            "Ready only"
        } else {
            "All status"
        },
        "lists_tables.virtualized_table.filter.status",
        button_visual(38, 52, 70),
    );
    button(
        ui,
        virtual_controls,
        "lists_tables.virtualized_table.resize.reset",
        "Reset width",
        "lists_tables.virtualized_table.resize.reset",
        button_visual(38, 52, 70),
    );

    let columns = virtual_table_columns(state);
    let visible_rows = virtual_table_visible_rows(state);
    let mut table_options = ext_widgets::DataTableOptions::default()
        .with_row_action_prefix("lists_tables.virtualized_table")
        .with_cell_action_prefix("lists_tables.virtualized_table")
        .with_scroll_action("lists_tables.virtualized_table.scroll");
    table_options.layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_flex_shrink(0.0);
    table_options.header_visual = UiVisual::panel(
        color(34, 41, 50),
        Some(StrokeStyle::new(color(67, 78, 95), 1.0)),
        0.0,
    );
    table_options.header_text_style = text(12.0, color(222, 230, 240));
    table_options.selection = state.table_selection.clone();
    ext_widgets::virtualized_data_table(
        ui,
        body,
        "lists_tables.virtualized_table",
        &columns,
        ext_widgets::VirtualDataTableSpec {
            row_count: visible_rows.len(),
            row_height: 28.0,
            viewport_width: 520.0,
            viewport_height: 156.0,
            scroll_offset: UiPoint::new(0.0, state.virtual_table_scroll),
            overscan_rows: 1,
        },
        table_options,
        |ui, cell_parent, cell| {
            let source_row = visible_rows.get(cell.row).copied().unwrap_or(cell.row);
            let value = virtual_table_cell_value(source_row, cell.column);
            widgets::label(
                ui,
                cell_parent,
                format!(
                    "lists_tables.virtualized_table.cell.{}.{}.label",
                    cell.row, cell.column
                ),
                value,
                text(12.0, color(220, 228, 238)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
    );
}

#[allow(clippy::field_reassign_with_default)]
fn property_inspector(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "property_inspector", "Property inspector");
    widgets::label(
        ui,
        body,
        "property_inspector.target",
        "Inspecting: Styling preview",
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let mut options = ext_widgets::PropertyInspectorOptions::default();
    options.selected_index = Some(0);
    options.label_width = 120.0;
    options.row_height = 30.0;
    ext_widgets::property_inspector_grid(
        ui,
        body,
        "property_inspector.grid",
        &[
            ext_widgets::PropertyGridRow::new("target", "Widget", "Button preview").read_only(),
            ext_widgets::PropertyGridRow::new(
                "inner",
                "Inner margin",
                format!("{:.0}px", state.styling.inner_margin),
            )
            .with_kind(ext_widgets::PropertyValueKind::Number),
            ext_widgets::PropertyGridRow::new(
                "outer",
                "Outer margin",
                format!("{:.0}px", state.styling.outer_margin),
            )
            .with_kind(ext_widgets::PropertyValueKind::Number),
            ext_widgets::PropertyGridRow::new(
                "radius",
                "Corner radius",
                format!("{:.0}px", state.styling.corner_radius),
            )
            .with_kind(ext_widgets::PropertyValueKind::Number),
            ext_widgets::PropertyGridRow::new(
                "stroke",
                "Stroke",
                format!("{:.1}px", state.styling.stroke_width),
            )
            .with_kind(ext_widgets::PropertyValueKind::Number)
            .changed(),
            ext_widgets::PropertyGridRow::new("state", "Source", "Styling widget").read_only(),
        ],
        options,
    );
}

fn diagnostics_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "diagnostics", "Diagnostics");
    diagnostics_page_picker(ui, body, state.diagnostics_page);

    match state.diagnostics_page {
        DiagnosticsPage::Overview => diagnostics_overview_widgets(ui, body, state),
        DiagnosticsPage::Frame => diagnostics_frame_page(ui, body, state),
        DiagnosticsPage::Layout => diagnostics_layout_page(ui, body, state),
        DiagnosticsPage::Hit => diagnostics_hit_page(ui, body, state),
        DiagnosticsPage::Input => diagnostics_input_page(ui, body, state),
        DiagnosticsPage::Paint => diagnostics_paint_page(ui, body, state),
        DiagnosticsPage::Text => diagnostics_text_page(ui, body, state),
        DiagnosticsPage::State => diagnostics_state_page(ui, body, state),
        DiagnosticsPage::Accessibility => diagnostics_accessibility_page(ui, body, state),
        DiagnosticsPage::LegacyAll => diagnostics_all_widgets(ui, body, state),
    }
}

fn diagnostics_page_picker(ui: &mut UiDocument, parent: UiNodeId, selected_page: DiagnosticsPage) {
    let page_row = wrapping_row(ui, parent, "diagnostics.pages", 8.0);
    for page in DiagnosticsPage::visible() {
        let action = format!("diagnostics.page.{}", page.action_suffix());
        diagnostic_button(ui, page_row, action, page.label(), *page == selected_page);
    }
}

struct DiagnosticsFrameBundle {
    frame_trace: DebugFrameTrace,
    frame_samples: Vec<DebugFrameTrace>,
    frame_recorder: DebugFrameRecorder,
}

fn diagnostics_frame_bundle(
    state: &ShowcaseState,
    debug_snapshot: &DebugInspectorSnapshot,
) -> DiagnosticsFrameBundle {
    let resource_report = diagnostics_resource_report_sample(debug_snapshot);
    let frame_trace = diagnostics_frame_trace_sample(state, &resource_report);
    let frame_samples = diagnostics_frame_trace_samples(state, &resource_report);
    let mut frame_recorder = DebugFrameRecorder::new(8);
    for trace in frame_samples.iter().cloned() {
        frame_recorder.record_frame(trace);
    }
    DiagnosticsFrameBundle {
        frame_trace,
        frame_samples,
        frame_recorder,
    }
}

fn diagnostics_overview_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let debug_snapshot = &state.diagnostics_snapshot;
    let resource_report = diagnostics_resource_report_sample(debug_snapshot);
    let frame_trace = diagnostics_frame_trace_sample(state, &resource_report);
    let health_score = DebugHealthScoreTrace::from_frame_trace(&frame_trace);

    diagnostics_health_score_panel(ui, parent, &health_score);
    diagnostics_issue_panel(ui, parent, &frame_trace);
    diagnostics_finding_inbox_panel(ui, parent, &frame_trace);
    diagnostics_frame_trace_panel(ui, parent, &frame_trace);
    diagnostics_selected_node_panel(ui, parent, debug_snapshot);
    diagnostics_panel_recommendation_panel(ui, parent, &frame_trace);
}

fn diagnostics_frame_page(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let debug_snapshot = &state.diagnostics_snapshot;
    let bundle = diagnostics_frame_bundle(state, debug_snapshot);
    let frame_timeline = bundle.frame_recorder.timeline();
    let frame_regression = bundle.frame_recorder.latest_frame_regression_trace();
    let fix_verification = bundle.frame_recorder.latest_fix_verification_trace();
    let performance_timeline = bundle.frame_recorder.performance_timeline();
    let issue_timeline = bundle.frame_recorder.issue_timeline();
    let why_trace = DebugWhyTrace::from_frame_trace(&bundle.frame_trace);
    let why_timeline = bundle.frame_recorder.why_timeline();
    let root_causes = DebugRootCauseClusterTrace::from_frame_trace(&bundle.frame_trace);
    let root_cause_timeline = bundle.frame_recorder.root_cause_timeline();

    diagnostics_frame_trace_panel(ui, parent, &bundle.frame_trace);
    diagnostics_issue_panel(ui, parent, &bundle.frame_trace);
    diagnostics_root_cause_cluster_panel(ui, parent, &root_causes);
    diagnostics_root_cause_timeline_panel(ui, parent, &root_cause_timeline);
    diagnostics_issue_timeline_panel(ui, parent, &issue_timeline);
    diagnostics_question_guide_panel(ui, parent, &bundle.frame_trace);
    diagnostics_coverage_panel(ui, parent, &bundle.frame_trace);
    diagnostics_investigation_plan_panel(ui, parent, &bundle.frame_trace);
    diagnostics_capture_report_panel(ui, parent, &bundle.frame_trace, &frame_timeline);
    diagnostics_slow_frame_panel(ui, parent, &bundle.frame_trace, state);
    diagnostics_frame_budget_panel(ui, parent);
    diagnostics_frame_timing_waterfall_panel(ui, parent, &bundle.frame_trace);
    diagnostics_frame_bottleneck_panel(ui, parent, &bundle.frame_trace, state);
    diagnostics_frame_autopsy_panel(ui, parent, &bundle.frame_trace);
    diagnostics_frame_recorder_panel(ui, parent, &bundle.frame_recorder);
    diagnostics_frame_timeline_panel(ui, parent, &frame_timeline);
    if let Some(frame_regression) = &frame_regression {
        diagnostics_frame_regression_panel(ui, parent, frame_regression);
    }
    if let Some(fix_verification) = &fix_verification {
        diagnostics_fix_verification_panel(ui, parent, fix_verification);
    }
    diagnostics_performance_timeline_panel(ui, parent, &performance_timeline);
    diagnostics_why_trace_panel(ui, parent, &why_trace);
    diagnostics_why_timeline_panel(ui, parent, &why_timeline);
    diagnostics_node_frame_history_panel(ui, parent, &bundle.frame_samples);
    diagnostics_node_change_panel(ui, parent, &bundle.frame_samples);
}

fn diagnostics_layout_page(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let debug_snapshot = &state.diagnostics_snapshot;
    let bundle = diagnostics_frame_bundle(state, debug_snapshot);
    let slow_nodes = diagnostics_slow_node_trace_sample(state);
    let layout_jank_timeline = bundle.frame_recorder.layout_jank_timeline();
    let layout_cost_timeline = bundle.frame_recorder.layout_cost_timeline();
    let layout_pressure_timeline = bundle.frame_recorder.layout_pressure_timeline();

    diagnostics_node_recommendation_panel(ui, parent, &bundle.frame_trace, Some(&slow_nodes));
    diagnostics_slow_nodes_panel(ui, parent, &slow_nodes);
    diagnostics_slow_node_timeline_panel(ui, parent, &bundle.frame_recorder.slow_node_timeline());
    diagnostics_layout_movement_panel(ui, parent, state, debug_snapshot);
    diagnostics_layout_jank_timeline_panel(ui, parent, &layout_jank_timeline);
    diagnostics_layout_cost_panel(ui, parent, debug_snapshot);
    diagnostics_layout_cost_timeline_panel(ui, parent, &layout_cost_timeline);
    diagnostics_layout_pressure_panel(ui, parent, debug_snapshot);
    diagnostics_layout_pressure_timeline_panel(ui, parent, &layout_pressure_timeline);
    diagnostics_layout_cost_autopsy_panel(ui, parent, debug_snapshot);
    diagnostics_layout_tree_panel(ui, parent, debug_snapshot);
    diagnostics_node_search_panel(ui, parent, debug_snapshot);
    diagnostics_layout_cause_panel(ui, parent, debug_snapshot);
    diagnostics_layout_autopsy_panel(ui, parent, debug_snapshot);
    diagnostics_style_compare_panel(ui, parent, debug_snapshot);
    diagnostics_responsive_layout_panel(ui, parent, state);
    diagnostics_frame_diff_panel(ui, parent, state, debug_snapshot);
    diagnostics_constraint_panel(ui, parent, debug_snapshot);
}

fn diagnostics_hit_page(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let debug_snapshot = &state.diagnostics_snapshot;
    let bundle = diagnostics_frame_bundle(state, debug_snapshot);
    let overlap_timeline = bundle.frame_recorder.overlap_timeline();
    let hitbox_timeline = bundle.frame_recorder.hitbox_timeline();
    let hitbox_occlusion_timeline = diagnostics_hitbox_occlusion_timeline_sample();
    let pointer_session = bundle.frame_recorder.pointer_session();

    diagnostics_geometry_preview(ui, parent, state, debug_snapshot);
    diagnostics_visibility_panel(ui, parent);
    diagnostics_visibility_timeline_panel(ui, parent);
    diagnostics_hitbox_map_panel(ui, parent);
    diagnostics_hitbox_timeline_panel(ui, parent, &hitbox_timeline);
    diagnostics_hit_targets_panel(ui, parent);
    diagnostics_hitbox_occlusion_panel(ui, parent);
    diagnostics_hitbox_occlusion_timeline_panel(ui, parent, &hitbox_occlusion_timeline);
    diagnostics_paint_hit_mismatch_panel(ui, parent);
    diagnostics_paint_hit_mismatch_timeline_panel(ui, parent);
    diagnostics_overlap_report_panel(ui, parent, state);
    diagnostics_overlap_timeline_panel(ui, parent, &overlap_timeline);
    diagnostics_overlap_autopsy_panel(ui, parent, state);
    diagnostics_pointer_probe_panel(ui, parent, state);
    diagnostics_pointer_session_panel(ui, parent, &pointer_session);
    diagnostics_pointer_autopsy_panel(ui, parent, state);
    diagnostics_point_autopsy_panel(ui, parent, state);
    diagnostics_inspect_point_panel(ui, parent, state);
}

fn diagnostics_input_page(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let debug_snapshot = &state.diagnostics_snapshot;
    let bundle = diagnostics_frame_bundle(state, debug_snapshot);
    let focus_timeline = diagnostics_focus_navigation_timeline_sample(state);
    let action_map_timeline = bundle.frame_recorder.action_map_timeline();
    let interaction_affordance_timeline = bundle.frame_recorder.interaction_affordance_timeline();
    let interaction_state_timeline = diagnostics_interaction_state_timeline_sample(state);

    diagnostics_focus_panel(ui, parent, state);
    diagnostics_focus_timeline_panel(ui, parent, &focus_timeline);
    diagnostics_clip_scroll_panel(ui, parent);
    diagnostics_clip_chain_panel(ui, parent);
    diagnostics_clip_chain_timeline_panel(ui, parent);
    diagnostics_scroll_range_panel(ui, parent);
    diagnostics_wheel_route_panel(ui, parent);
    diagnostics_scroll_timeline_panel(ui, parent);
    diagnostics_interaction_state_panel(ui, parent, state);
    diagnostics_interaction_state_timeline_panel(ui, parent, &interaction_state_timeline);
    diagnostics_shortcut_route_panel(ui, parent);
    diagnostics_action_map_panel(ui, parent, state);
    diagnostics_action_dispatch_panel(ui, parent);
    diagnostics_action_map_timeline_panel(ui, parent, &action_map_timeline);
    diagnostics_drag_affordance_panel(ui, parent);
    diagnostics_interaction_affordance_panel(ui, parent);
    diagnostics_interaction_affordance_timeline_panel(ui, parent, &interaction_affordance_timeline);
}

fn diagnostics_paint_page(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let debug_snapshot = &state.diagnostics_snapshot;
    let resource_report = diagnostics_resource_report_sample(debug_snapshot);

    diagnostics_paint_order_panel(ui, parent, state);
    diagnostics_stacking_order_panel(ui, parent, state);
    diagnostics_stacking_order_timeline_panel(ui, parent);
    diagnostics_render_layers_panel(ui, parent, state);
    diagnostics_render_layer_timeline_panel(ui, parent);
    diagnostics_paint_overdraw_panel(ui, parent, state);
    diagnostics_paint_overdraw_timeline_panel(ui, parent, state);
    diagnostics_paint_batches_panel(ui, parent, state);
    diagnostics_paint_batch_timeline_panel(ui, parent);
    diagnostics_visual_effects_panel(ui, parent);
    diagnostics_visual_effect_timeline_panel(ui, parent);
    diagnostics_bounds_autopsy_panel(ui, parent);
    diagnostics_resource_panel(ui, parent, &resource_report);
    diagnostics_resource_timeline_panel(ui, parent);
}

fn diagnostics_text_page(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    diagnostics_text_fit_panel(ui, parent);
    diagnostics_text_fit_timeline_panel(ui, parent);
    diagnostics_text_layout_panel(ui, parent, state);
    diagnostics_text_style_panel(ui, parent, state);
    diagnostics_text_input_state_panel(ui, parent);
    diagnostics_text_input_event_panel(ui, parent);
    diagnostics_text_localization_panel(ui, parent);
    diagnostics_text_contrast_panel(ui, parent);
}

fn diagnostics_state_page(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let debug_snapshot = &state.diagnostics_snapshot;
    let resource_report = diagnostics_resource_report_sample(debug_snapshot);
    let frame_trace = diagnostics_frame_trace_sample(state, &resource_report);
    let bundle = diagnostics_frame_bundle(state, debug_snapshot);
    let session_narrative = bundle.frame_recorder.session_narrative();
    let invariant_timeline = bundle.frame_recorder.invariant_timeline();
    let constraint_timeline = bundle.frame_recorder.constraint_timeline();
    let registry = diagnostics_command_registry();
    let theme_snapshot = DebugThemeSnapshot::from_theme(&Theme::dark());

    diagnostics_session_narrative_panel(ui, parent, &session_narrative);
    diagnostics_contract_panel(ui, parent, &frame_trace);
    diagnostics_invariant_panel(ui, parent);
    diagnostics_invariant_timeline_panel(ui, parent, &invariant_timeline);
    diagnostics_constraint_timeline_panel(ui, parent, &constraint_timeline);
    diagnostics_commands_panel(ui, parent, &registry);
    diagnostics_theme_panel(ui, parent, &theme_snapshot);
    diagnostics_cache_reuse_panel(ui, parent);
    diagnostics_timing_panel(ui, parent);
    diagnostics_dirty_state_panel(ui, parent);
    diagnostics_widget_state_retention_panel(ui, parent);
    diagnostics_invalidation_blame_panel(ui, parent);
    diagnostics_invalidation_blast_panel(ui, parent);
    diagnostics_invalidation_timeline_panel(ui, parent);
    diagnostics_node_explanation_panel(ui, parent, state, debug_snapshot, &resource_report);
    diagnostics_inspect_node_panel(ui, parent, state, debug_snapshot, &resource_report);
    diagnostics_node_provenance_panel(ui, parent, debug_snapshot, &resource_report);
}

fn diagnostics_accessibility_page(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let debug_snapshot = &state.diagnostics_snapshot;

    widgets::label(
        ui,
        parent,
        "diagnostics.a11y.title",
        "Accessibility",
        text(14.0, color(222, 230, 240)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let mut overlay_preview_style = UiNodeStyle::from(
        LayoutStyle::new()
            .with_width(320.0)
            .with_height(140.0)
            .with_flex_shrink(0.0),
    );
    overlay_preview_style.set_clip(ClipBehavior::Clip);
    let overlay_preview = ui.add_child(
        parent,
        UiNode::container("diagnostics.a11y.preview", overlay_preview_style).with_visual(
            UiVisual::panel(
                color(12, 17, 24),
                Some(StrokeStyle::new(color(47, 62, 82), 1.0)),
                4.0,
            ),
        ),
    );
    let mut overlay_options = ext_widgets::AccessibilityDebugOverlayOptions {
        action_prefix: Some("diagnostics.a11y.visual".to_owned()),
        ..Default::default()
    };
    overlay_options.show_labels = false;
    ext_widgets::accessibility_debug_overlay(
        ui,
        overlay_preview,
        "diagnostics.a11y.visual",
        debug_snapshot,
        overlay_options,
    );
    diagnostics_accessibility_tree_panel(ui, parent, debug_snapshot);
    diagnostics_accessibility_timeline_panel(ui, parent);
    diagnostics_accessibility_details(ui, parent, debug_snapshot);
}

fn diagnostics_all_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = parent;
    let debug_snapshot = &state.diagnostics_snapshot;
    let resource_report = diagnostics_resource_report_sample(debug_snapshot);
    let frame_trace = diagnostics_frame_trace_sample(state, &resource_report);
    let frame_samples = diagnostics_frame_trace_samples(state, &resource_report);
    let mut frame_recorder = DebugFrameRecorder::new(8);
    for trace in frame_samples.iter().cloned() {
        frame_recorder.record_frame(trace);
    }
    let frame_timeline = frame_recorder.timeline();
    let frame_regression = frame_recorder.latest_frame_regression_trace();
    let fix_verification = frame_recorder.latest_fix_verification_trace();
    let performance_timeline = frame_recorder.performance_timeline();
    let issue_timeline = frame_recorder.issue_timeline();
    let why_trace = DebugWhyTrace::from_frame_trace(&frame_trace);
    let why_timeline = frame_recorder.why_timeline();
    let health_score = DebugHealthScoreTrace::from_frame_trace(&frame_trace);
    let root_causes = DebugRootCauseClusterTrace::from_frame_trace(&frame_trace);
    let root_cause_timeline = frame_recorder.root_cause_timeline();
    let overlap_timeline = frame_recorder.overlap_timeline();
    let hitbox_timeline = frame_recorder.hitbox_timeline();
    let hitbox_occlusion_timeline = diagnostics_hitbox_occlusion_timeline_sample();
    let invariant_timeline = frame_recorder.invariant_timeline();
    let constraint_timeline = frame_recorder.constraint_timeline();
    let layout_jank_timeline = frame_recorder.layout_jank_timeline();
    let layout_cost_timeline = frame_recorder.layout_cost_timeline();
    let layout_pressure_timeline = frame_recorder.layout_pressure_timeline();
    let focus_timeline = diagnostics_focus_navigation_timeline_sample(state);
    let action_map_timeline = frame_recorder.action_map_timeline();
    let interaction_affordance_timeline = frame_recorder.interaction_affordance_timeline();
    let interaction_state_timeline = diagnostics_interaction_state_timeline_sample(state);
    let animation_activity_timeline = diagnostics_animation_activity_timeline_sample();
    let session_narrative = frame_recorder.session_narrative();
    let pointer_session = frame_recorder.pointer_session();
    let slow_nodes = diagnostics_slow_node_trace_sample(state);

    diagnostics_frame_trace_panel(ui, body, &frame_trace);
    diagnostics_issue_panel(ui, body, &frame_trace);
    diagnostics_finding_inbox_panel(ui, body, &frame_trace);
    diagnostics_health_score_panel(ui, body, &health_score);
    diagnostics_root_cause_cluster_panel(ui, body, &root_causes);
    diagnostics_root_cause_timeline_panel(ui, body, &root_cause_timeline);
    diagnostics_issue_timeline_panel(ui, body, &issue_timeline);
    diagnostics_question_guide_panel(ui, body, &frame_trace);
    diagnostics_panel_recommendation_panel(ui, body, &frame_trace);
    diagnostics_overlay_recommendation_panel(ui, body, &frame_trace);
    diagnostics_coverage_panel(ui, body, &frame_trace);
    diagnostics_investigation_plan_panel(ui, body, &frame_trace);
    diagnostics_session_narrative_panel(ui, body, &session_narrative);
    diagnostics_contract_panel(ui, body, &frame_trace);
    diagnostics_invariant_panel(ui, body);
    diagnostics_invariant_timeline_panel(ui, body, &invariant_timeline);
    diagnostics_constraint_timeline_panel(ui, body, &constraint_timeline);
    diagnostics_capture_report_panel(ui, body, &frame_trace, &frame_timeline);
    diagnostics_hotspots_panel(ui, body, debug_snapshot);
    diagnostics_node_recommendation_panel(ui, body, &frame_trace, Some(&slow_nodes));
    diagnostics_slow_nodes_panel(ui, body, &slow_nodes);
    diagnostics_slow_node_timeline_panel(ui, body, &frame_recorder.slow_node_timeline());
    diagnostics_slow_frame_panel(ui, body, &frame_trace, state);
    diagnostics_frame_budget_panel(ui, body);
    diagnostics_frame_timing_waterfall_panel(ui, body, &frame_trace);
    diagnostics_frame_bottleneck_panel(ui, body, &frame_trace, state);
    diagnostics_frame_autopsy_panel(ui, body, &frame_trace);
    diagnostics_frame_recorder_panel(ui, body, &frame_recorder);
    diagnostics_frame_timeline_panel(ui, body, &frame_timeline);
    if let Some(frame_regression) = &frame_regression {
        diagnostics_frame_regression_panel(ui, body, frame_regression);
    }
    if let Some(fix_verification) = &fix_verification {
        diagnostics_fix_verification_panel(ui, body, fix_verification);
    }
    diagnostics_performance_timeline_panel(ui, body, &performance_timeline);
    diagnostics_why_trace_panel(ui, body, &why_trace);
    diagnostics_why_timeline_panel(ui, body, &why_timeline);
    diagnostics_node_frame_history_panel(ui, body, &frame_samples);
    diagnostics_node_change_panel(ui, body, &frame_samples);
    diagnostics_cache_reuse_panel(ui, body);
    diagnostics_layout_movement_panel(ui, body, state, debug_snapshot);
    diagnostics_layout_jank_timeline_panel(ui, body, &layout_jank_timeline);
    diagnostics_layout_cost_panel(ui, body, debug_snapshot);
    diagnostics_layout_cost_timeline_panel(ui, body, &layout_cost_timeline);
    diagnostics_layout_pressure_panel(ui, body, debug_snapshot);
    diagnostics_layout_pressure_timeline_panel(ui, body, &layout_pressure_timeline);
    diagnostics_layout_cost_autopsy_panel(ui, body, debug_snapshot);
    diagnostics_layout_tree_panel(ui, body, debug_snapshot);
    diagnostics_node_search_panel(ui, body, debug_snapshot);
    diagnostics_focus_panel(ui, body, state);
    diagnostics_focus_timeline_panel(ui, body, &focus_timeline);
    diagnostics_selected_node_panel(ui, body, debug_snapshot);
    diagnostics_layout_cause_panel(ui, body, debug_snapshot);
    diagnostics_layout_autopsy_panel(ui, body, debug_snapshot);
    diagnostics_style_compare_panel(ui, body, debug_snapshot);
    diagnostics_clip_scroll_panel(ui, body);
    diagnostics_clip_chain_panel(ui, body);
    diagnostics_clip_chain_timeline_panel(ui, body);
    diagnostics_scroll_range_panel(ui, body);
    diagnostics_wheel_route_panel(ui, body);
    diagnostics_scroll_timeline_panel(ui, body);
    diagnostics_responsive_layout_panel(ui, body, state);
    diagnostics_node_explanation_panel(ui, body, state, debug_snapshot, &resource_report);
    diagnostics_inspect_node_panel(ui, body, state, debug_snapshot, &resource_report);
    diagnostics_node_provenance_panel(ui, body, debug_snapshot, &resource_report);
    diagnostics_geometry_preview(ui, body, state, debug_snapshot);
    diagnostics_visibility_panel(ui, body);
    diagnostics_visibility_timeline_panel(ui, body);
    diagnostics_hitbox_map_panel(ui, body);
    diagnostics_hitbox_timeline_panel(ui, body, &hitbox_timeline);
    diagnostics_hit_targets_panel(ui, body);
    diagnostics_hitbox_occlusion_panel(ui, body);
    diagnostics_hitbox_occlusion_timeline_panel(ui, body, &hitbox_occlusion_timeline);
    diagnostics_paint_hit_mismatch_panel(ui, body);
    diagnostics_paint_hit_mismatch_timeline_panel(ui, body);
    diagnostics_overlap_report_panel(ui, body, state);
    diagnostics_overlap_timeline_panel(ui, body, &overlap_timeline);
    diagnostics_overlap_autopsy_panel(ui, body, state);
    diagnostics_pointer_probe_panel(ui, body, state);
    diagnostics_pointer_session_panel(ui, body, &pointer_session);
    diagnostics_pointer_autopsy_panel(ui, body, state);
    diagnostics_point_autopsy_panel(ui, body, state);
    diagnostics_inspect_point_panel(ui, body, state);
    diagnostics_interaction_state_panel(ui, body, state);
    diagnostics_interaction_state_timeline_panel(ui, body, &interaction_state_timeline);
    diagnostics_shortcut_route_panel(ui, body);
    diagnostics_action_map_panel(ui, body, state);
    diagnostics_action_dispatch_panel(ui, body);
    diagnostics_action_map_timeline_panel(ui, body, &action_map_timeline);
    diagnostics_drag_affordance_panel(ui, body);
    diagnostics_interaction_affordance_panel(ui, body);
    diagnostics_interaction_affordance_timeline_panel(ui, body, &interaction_affordance_timeline);
    diagnostics_paint_order_panel(ui, body, state);
    diagnostics_stacking_order_panel(ui, body, state);
    diagnostics_stacking_order_timeline_panel(ui, body);
    diagnostics_render_layers_panel(ui, body, state);
    diagnostics_render_layer_timeline_panel(ui, body);
    diagnostics_paint_overdraw_panel(ui, body, state);
    diagnostics_paint_overdraw_timeline_panel(ui, body, state);
    diagnostics_paint_batches_panel(ui, body, state);
    diagnostics_paint_batch_timeline_panel(ui, body);
    diagnostics_visual_effects_panel(ui, body);
    diagnostics_visual_effect_timeline_panel(ui, body);
    diagnostics_bounds_autopsy_panel(ui, body);
    diagnostics_text_fit_panel(ui, body);
    diagnostics_text_fit_timeline_panel(ui, body);
    diagnostics_text_layout_panel(ui, body, state);
    diagnostics_text_style_panel(ui, body, state);
    diagnostics_text_input_state_panel(ui, body);
    diagnostics_text_input_event_panel(ui, body);
    diagnostics_text_localization_panel(ui, body);
    diagnostics_text_contrast_panel(ui, body);
    diagnostics_event_route_panel(ui, body, state);
    diagnostics_frame_diff_panel(ui, body, state, debug_snapshot);
    diagnostics_constraint_panel(ui, body, debug_snapshot);
    diagnostics_resource_panel(ui, body, &resource_report);
    diagnostics_resource_timeline_panel(ui, body);
    diagnostics_timing_panel(ui, body);
    diagnostics_dirty_state_panel(ui, body);
    diagnostics_widget_state_retention_panel(ui, body);
    diagnostics_invalidation_blame_panel(ui, body);
    diagnostics_invalidation_blast_panel(ui, body);
    diagnostics_invalidation_timeline_panel(ui, body);
    diagnostics_animation_activity_panel(ui, body, debug_snapshot);
    diagnostics_animation_activity_timeline_panel(ui, body, &animation_activity_timeline);
    diagnostics_animation_panel(ui, body, state, debug_snapshot);

    widgets::label(
        ui,
        body,
        "diagnostics.a11y.title",
        "Accessibility",
        text(14.0, color(222, 230, 240)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let mut overlay_preview_style = UiNodeStyle::from(
        LayoutStyle::new()
            .with_width(320.0)
            .with_height(140.0)
            .with_flex_shrink(0.0),
    );
    overlay_preview_style.set_clip(ClipBehavior::Clip);
    let overlay_preview = ui.add_child(
        body,
        UiNode::container("diagnostics.a11y.preview", overlay_preview_style).with_visual(
            UiVisual::panel(
                color(12, 17, 24),
                Some(StrokeStyle::new(color(47, 62, 82), 1.0)),
                4.0,
            ),
        ),
    );
    let mut overlay_options = ext_widgets::AccessibilityDebugOverlayOptions {
        action_prefix: Some("diagnostics.a11y.visual".to_owned()),
        ..Default::default()
    };
    overlay_options.show_labels = false;
    ext_widgets::accessibility_debug_overlay(
        ui,
        overlay_preview,
        "diagnostics.a11y.visual",
        &debug_snapshot,
        overlay_options,
    );
    diagnostics_accessibility_tree_panel(ui, body, debug_snapshot);
    diagnostics_accessibility_timeline_panel(ui, body);
    diagnostics_accessibility_details(ui, body, debug_snapshot);

    let diagnostic_columns = ui.add_child(
        body,
        UiNode::container(
            "diagnostics.columns",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_flex_shrink(0.0)
                .gap(10.0),
        ),
    );
    let command_column = ui.add_child(
        diagnostic_columns,
        UiNode::container(
            "diagnostics.commands.column",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_flex_shrink(0.0)
                .gap(8.0),
        ),
    );
    let theme_column = ui.add_child(
        diagnostic_columns,
        UiNode::container(
            "diagnostics.theme.column",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_flex_shrink(0.0)
                .gap(8.0),
        ),
    );

    let registry = diagnostics_command_registry();
    diagnostics_commands_panel(ui, command_column, &registry);

    let theme_snapshot = DebugThemeSnapshot::from_theme(&Theme::dark());
    diagnostics_theme_panel(ui, theme_column, &theme_snapshot);
}

fn diagnostics_frame_trace_panel(ui: &mut UiDocument, parent: UiNodeId, trace: &DebugFrameTrace) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.trace.panel", "Frame trace");
    ext_widgets::frame_trace_panel(
        ui,
        panel,
        "diagnostics.trace",
        trace,
        ext_widgets::FrameTracePanelOptions {
            action_prefix: Some("diagnostics.trace".to_owned()),
            max_route_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_issue_panel(ui: &mut UiDocument, parent: UiNodeId, trace: &DebugFrameTrace) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.issues.panel", "Issue triage");
    let report = DebugIssueReport::from_frame_trace(trace);
    ext_widgets::debug_issue_panel(
        ui,
        panel,
        "diagnostics.issues",
        &report,
        ext_widgets::DebugIssuePanelOptions {
            action_prefix: Some("diagnostics.issues".to_owned()),
            max_issue_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_finding_inbox_panel(ui: &mut UiDocument, parent: UiNodeId, trace: &DebugFrameTrace) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.findings.panel", "Finding inbox");
    let inbox = DebugFindingInboxTrace::from_frame_trace(trace);
    ext_widgets::debug_finding_inbox_panel(
        ui,
        panel,
        "diagnostics.findings",
        &inbox,
        ext_widgets::DebugFindingInboxPanelOptions {
            action_prefix: Some("diagnostics.findings".to_owned()),
            max_finding_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_health_score_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugHealthScoreTrace,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.health.panel", "Health score");
    ext_widgets::debug_health_score_panel(
        ui,
        panel,
        "diagnostics.health",
        trace,
        ext_widgets::DebugHealthScorePanelOptions {
            action_prefix: Some("diagnostics.health".to_owned()),
            max_area_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_root_cause_cluster_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugRootCauseClusterTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.root_causes.panel",
        "Root-cause clusters",
    );
    ext_widgets::root_cause_cluster_panel(
        ui,
        panel,
        "diagnostics.root_causes",
        trace,
        ext_widgets::RootCauseClusterPanelOptions {
            action_prefix: Some("diagnostics.root_causes".to_owned()),
            max_cluster_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_root_cause_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugRootCauseTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.root_cause_timeline.panel",
        "Root-cause timeline",
    );
    ext_widgets::root_cause_timeline_panel(
        ui,
        panel,
        "diagnostics.root_cause_timeline",
        trace,
        ext_widgets::RootCauseTimelinePanelOptions {
            action_prefix: Some("diagnostics.root_cause_timeline".to_owned()),
            max_cluster_rows: 8,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_issue_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugIssueTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.issue_timeline.panel",
        "Issue timeline",
    );
    ext_widgets::issue_timeline_panel(
        ui,
        panel,
        "diagnostics.issue_timeline",
        trace,
        ext_widgets::IssueTimelinePanelOptions {
            action_prefix: Some("diagnostics.issue_timeline".to_owned()),
            max_source_rows: 8,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_question_guide_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugFrameTrace,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.questions.panel", "Question guide");
    let guide = DebugQuestionGuideTrace::from_frame_trace(trace);
    ext_widgets::question_guide_panel(
        ui,
        panel,
        "diagnostics.questions",
        &guide,
        ext_widgets::QuestionGuidePanelOptions {
            action_prefix: Some("diagnostics.questions".to_owned()),
            max_question_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_panel_recommendation_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugFrameTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.panel_recommendations.panel",
        "Panel recommendations",
    );
    let recommendations = DebugPanelRecommendationTrace::from_frame_trace(trace);
    ext_widgets::debug_panel_recommendation_panel(
        ui,
        panel,
        "diagnostics.panel_recommendations",
        &recommendations,
        ext_widgets::DebugPanelRecommendationPanelOptions {
            action_prefix: Some("diagnostics.panel_recommendations".to_owned()),
            max_recommendation_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_overlay_recommendation_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugFrameTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.overlay_recommendations.panel",
        "Overlay recommendations",
    );
    let recommendations = DebugOverlayRecommendationTrace::from_frame_trace(trace);
    ext_widgets::overlay_recommendation_panel(
        ui,
        panel,
        "diagnostics.overlay_recommendations",
        &recommendations,
        ext_widgets::OverlayRecommendationPanelOptions {
            action_prefix: Some("diagnostics.overlay_recommendations".to_owned()),
            max_overlay_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_coverage_panel(ui: &mut UiDocument, parent: UiNodeId, trace: &DebugFrameTrace) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.coverage.panel",
        "Diagnostics coverage",
    );
    let coverage = DebugDiagnosticsCoverageTrace::from_frame_trace(trace);
    ext_widgets::diagnostics_coverage_panel(
        ui,
        panel,
        "diagnostics.coverage",
        &coverage,
        ext_widgets::DiagnosticsCoveragePanelOptions {
            action_prefix: Some("diagnostics.coverage".to_owned()),
            max_area_rows: 10,
            ..Default::default()
        },
    );
}

fn diagnostics_investigation_plan_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugFrameTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.investigation.panel",
        "Investigation plan",
    );
    let plan = DebugInvestigationPlanTrace::from_frame_trace(trace);
    ext_widgets::investigation_plan_panel(
        ui,
        panel,
        "diagnostics.investigation",
        &plan,
        ext_widgets::InvestigationPlanPanelOptions {
            action_prefix: Some("diagnostics.investigation".to_owned()),
            max_step_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_session_narrative_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugSessionNarrativeTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.session_narrative.panel",
        "Session narrative",
    );
    ext_widgets::debug_session_narrative_panel(
        ui,
        panel,
        "diagnostics.session_narrative",
        trace,
        ext_widgets::DebugSessionNarrativePanelOptions {
            action_prefix: Some("diagnostics.session_narrative".to_owned()),
            max_event_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_contract_panel(ui: &mut UiDocument, parent: UiNodeId, trace: &DebugFrameTrace) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.contract.panel", "Debug contract");
    let mut text_sample = diagnostics_text_fit_sample_document();
    text_sample
        .compute_layout(UiSize::new(260.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics contract text fit layout");
    let text_fit = DebugTextFitTrace::from_document(&text_sample, &mut ApproxTextMeasurer);
    let cache = diagnostics_cache_reuse_sample();
    let report =
        DebugContractReport::from_frame_trace_with_context(trace, Some(&text_fit), Some(&cache));
    ext_widgets::debug_contract_panel(
        ui,
        panel,
        "diagnostics.contract",
        &report,
        ext_widgets::DebugContractPanelOptions {
            action_prefix: Some("diagnostics.contract".to_owned()),
            max_check_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_invariant_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.invariants.panel", "UI invariants");
    let mut sample = diagnostics_invariant_sample_document();
    sample
        .compute_layout(UiSize::new(260.0, 150.0), &mut ApproxTextMeasurer)
        .expect("diagnostics invariant sample layout");
    let trace = DebugInvariantTrace::from_document(&sample, &mut ApproxTextMeasurer);
    ext_widgets::debug_invariant_panel(
        ui,
        panel,
        "diagnostics.invariants",
        &trace,
        ext_widgets::DebugInvariantPanelOptions {
            action_prefix: Some("diagnostics.invariants".to_owned()),
            max_invariant_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_invariant_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugInvariantTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.invariant_timeline.panel",
        "Invariant timeline",
    );
    ext_widgets::invariant_timeline_panel(
        ui,
        panel,
        "diagnostics.invariant_timeline",
        trace,
        ext_widgets::InvariantTimelinePanelOptions {
            action_prefix: Some("diagnostics.invariant_timeline".to_owned()),
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_constraint_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugConstraintTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.constraint_timeline.panel",
        "Constraint timeline",
    );
    ext_widgets::constraint_timeline_panel(
        ui,
        panel,
        "diagnostics.constraint_timeline",
        trace,
        ext_widgets::ConstraintTimelinePanelOptions {
            action_prefix: Some("diagnostics.constraint_timeline".to_owned()),
            max_frame_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_invariant_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(260.0, 150.0));
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.invariants.back",
            UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 92.0, 30.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.invariants.back.activate")
        .with_visual(UiVisual::panel(
            color(52, 112, 180),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            5.0,
        )),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.invariants.front",
            UiNodeStyle::new(operad::layout::absolute(44.0, 8.0, 92.0, 30.0)).with_z_index(8.0),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.invariants.front.activate")
        .with_visual(UiVisual::panel(
            color(168, 94, 52),
            Some(StrokeStyle::new(color(255, 203, 96), 1.0)),
            5.0,
        )),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.invariants.blank",
            UiNodeStyle::new(operad::layout::absolute(0.0, 48.0, 72.0, 28.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.invariants.blank.activate"),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.invariants.zero",
            UiNodeStyle::new(operad::layout::absolute(84.0, 48.0, 0.0, 24.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.invariants.zero.activate"),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.invariants.passive_action",
            UiNodeStyle::new(operad::layout::absolute(112.0, 48.0, 72.0, 28.0)),
        )
        .with_action("diagnostics.invariants.passive.activate")
        .with_visual(UiVisual::panel(
            color(46, 112, 84),
            Some(StrokeStyle::new(color(111, 202, 142), 1.0)),
            5.0,
        )),
    );
    let mut style = text(13.0, color(222, 230, 240));
    style.wrap = TextWrap::None;
    sample.add_child(
        sample.root(),
        UiNode::text(
            "diagnostics.invariants.label",
            "Very long localized toolbar label",
            style,
            operad::layout::absolute(0.0, 92.0, 64.0, 18.0),
        ),
    );
    sample
}

fn diagnostics_capture_report_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugFrameTrace,
    timeline: &DebugFrameTimelineTrace,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.capture.panel", "Capture report");
    let report = DebugCaptureReport::from_frame_trace_and_timeline(trace, Some(timeline));
    ext_widgets::debug_capture_report_panel(
        ui,
        panel,
        "diagnostics.capture",
        &report,
        ext_widgets::DebugCaptureReportPanelOptions {
            action_prefix: Some("diagnostics.capture".to_owned()),
            max_section_rows: 5,
            max_issue_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_hotspots_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.hotspots.panel", "Node hotspots");
    let trace = DebugNodeHotspotTrace::from_snapshot(snapshot);
    ext_widgets::node_hotspots_panel(
        ui,
        panel,
        "diagnostics.hotspots",
        &trace,
        ext_widgets::NodeHotspotsPanelOptions {
            action_prefix: Some("diagnostics.hotspots".to_owned()),
            max_hotspot_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_node_recommendation_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugFrameTrace,
    slow_nodes: Option<&DebugSlowNodeTrace>,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.node_recommendations.panel",
        "Node recommendations",
    );
    let recommendations =
        DebugNodeRecommendationTrace::from_frame_trace_with_slow_nodes(trace, slow_nodes);
    ext_widgets::node_recommendation_panel(
        ui,
        panel,
        "diagnostics.node_recommendations",
        &recommendations,
        ext_widgets::NodeRecommendationPanelOptions {
            action_prefix: Some("diagnostics.node_recommendations".to_owned()),
            max_node_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_slow_nodes_panel(ui: &mut UiDocument, parent: UiNodeId, trace: &DebugSlowNodeTrace) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.slow_nodes.panel", "Slow nodes");
    ext_widgets::slow_nodes_panel(
        ui,
        panel,
        "diagnostics.slow_nodes",
        trace,
        ext_widgets::SlowNodesPanelOptions {
            action_prefix: Some("diagnostics.slow_nodes".to_owned()),
            max_node_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_slow_node_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugSlowNodeTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.slow_node_timeline.panel",
        "Slow node timeline",
    );
    ext_widgets::slow_node_timeline_panel(
        ui,
        panel,
        "diagnostics.slow_node_timeline",
        trace,
        ext_widgets::SlowNodeTimelinePanelOptions {
            action_prefix: Some("diagnostics.slow_node_timeline".to_owned()),
            max_node_rows: 5,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_slow_node_trace_sample(state: &ShowcaseState) -> DebugSlowNodeTrace {
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    DebugSlowNodeTrace::from_document(&sample, &mut ApproxTextMeasurer)
}

fn diagnostics_layout_tree_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.tree.panel", "Layout tree");
    let tree = DebugLayoutTree::from_snapshot(snapshot);
    ext_widgets::debug_layout_tree_panel(
        ui,
        panel,
        "diagnostics.tree",
        &tree,
        ext_widgets::DebugLayoutTreePanelOptions {
            action_prefix: Some("diagnostics.tree".to_owned()),
            max_node_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_node_search_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.node_search.panel", "Node search");
    let trace = DebugNodeSearchTrace::from_snapshot(snapshot, "preview");
    ext_widgets::node_search_panel(
        ui,
        panel,
        "diagnostics.node_search",
        &trace,
        ext_widgets::NodeSearchPanelOptions {
            action_prefix: Some("diagnostics.node_search".to_owned()),
            max_match_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_focus_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.focus.panel", "Focus navigation");
    let mut sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let focused = sample.nodes().iter().enumerate().find_map(|(index, node)| {
        (node.name() == "diagnostics.sample.preview").then_some(UiNodeId::from_index(index))
    });
    sample.set_focus_state(UiFocusState {
        focused,
        ..Default::default()
    });
    let trace = DebugFocusNavigationTrace::from_document(&sample, &mut ApproxTextMeasurer);
    ext_widgets::focus_navigation_panel(
        ui,
        panel,
        "diagnostics.focus",
        &trace,
        ext_widgets::FocusNavigationPanelOptions {
            action_prefix: Some("diagnostics.focus".to_owned()),
            max_candidate_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_focus_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugFocusNavigationTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.focus_timeline.panel",
        "Focus timeline",
    );
    ext_widgets::focus_navigation_timeline_panel(
        ui,
        panel,
        "diagnostics.focus_timeline",
        trace,
        ext_widgets::FocusNavigationTimelinePanelOptions {
            action_prefix: Some("diagnostics.focus_timeline".to_owned()),
            max_candidate_rows: 6,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_focus_navigation_timeline_sample(
    state: &ShowcaseState,
) -> DebugFocusNavigationTimelineTrace {
    fn frame(state: &ShowcaseState, focused_name: &str, frame_index: u64) -> DebugFrameTrace {
        let mut sample = diagnostics_sample_document_for(
            state.diagnostics_animation_hover,
            state.diagnostics_animation_active,
        );
        let focused = sample.nodes().iter().enumerate().find_map(|(index, node)| {
            (node.name() == focused_name).then_some(UiNodeId::from_index(index))
        });
        sample.set_focus_state(UiFocusState {
            focused,
            ..Default::default()
        });
        DebugFrameTrace::from_document(
            &sample,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new("focus", UiSize::new(320.0, 180.0))
                .frame_index(frame_index),
        )
    }

    DebugFocusNavigationTimelineTrace::from_traces(
        [
            frame(state, "diagnostics.sample.preview", 1),
            frame(state, "diagnostics.sample.hotspot", 2),
            frame(state, "diagnostics.sample.preview", 3),
        ]
        .iter(),
    )
}

fn diagnostics_layout_cost_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.layout_cost.panel", "Layout cost");
    let trace = DebugLayoutCostTrace::from_snapshot(snapshot);
    ext_widgets::layout_cost_panel(
        ui,
        panel,
        "diagnostics.layout_cost",
        &trace,
        ext_widgets::LayoutCostPanelOptions {
            action_prefix: Some("diagnostics.layout_cost".to_owned()),
            max_cost_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_layout_cost_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugLayoutCostTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.layout_cost_timeline.panel",
        "Layout cost timeline",
    );
    ext_widgets::layout_cost_timeline_panel(
        ui,
        panel,
        "diagnostics.layout_cost_timeline",
        trace,
        ext_widgets::LayoutCostTimelinePanelOptions {
            action_prefix: Some("diagnostics.layout_cost_timeline".to_owned()),
            max_cost_rows: 5,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_layout_pressure_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.layout_pressure.panel",
        "Layout pressure",
    );
    let trace = DebugLayoutPressureTrace::from_snapshot(snapshot);
    ext_widgets::layout_pressure_panel(
        ui,
        panel,
        "diagnostics.layout_pressure",
        &trace,
        ext_widgets::LayoutPressurePanelOptions {
            action_prefix: Some("diagnostics.layout_pressure".to_owned()),
            max_pressure_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_layout_pressure_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugLayoutPressureTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.layout_pressure_timeline.panel",
        "Layout pressure timeline",
    );
    ext_widgets::layout_pressure_timeline_panel(
        ui,
        panel,
        "diagnostics.layout_pressure_timeline",
        trace,
        ext_widgets::LayoutPressureTimelinePanelOptions {
            action_prefix: Some("diagnostics.layout_pressure_timeline".to_owned()),
            max_pressure_rows: 5,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_layout_cost_autopsy_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let trace = DebugLayoutCostTrace::from_snapshot(snapshot);
    let Some(autopsy) = DebugLayoutCostAutopsyTrace::from_layout_cost_trace(&trace, None) else {
        return;
    };
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.layout_cost_autopsy.panel",
        "Layout cost autopsy",
    );
    ext_widgets::layout_cost_autopsy_panel(
        ui,
        panel,
        "diagnostics.layout_cost_autopsy",
        &autopsy,
        ext_widgets::LayoutCostAutopsyPanelOptions {
            action_prefix: Some("diagnostics.layout_cost_autopsy".to_owned()),
            max_source_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_frame_trace_sample(
    state: &ShowcaseState,
    resource_report: &DebugResourceReport,
) -> DebugFrameTrace {
    let mut sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    sample
        .compute_layout(UiSize::new(320.0, 180.0), &mut ApproxTextMeasurer)
        .expect("diagnostics trace sample layout");
    let route = DebugEventRouteTrace::from_pointer_event(
        &sample,
        DebugPointerRouteKind::Down,
        UiPoint::new(58.0, 44.0),
    );
    DebugFrameTrace::from_document(
        &sample,
        &mut ApproxTextMeasurer,
        DebugFrameTraceContext::new("diagnostics", UiSize::new(320.0, 180.0))
            .frame_index(42)
            .timing(diagnostics_frame_timing_sample())
            .timing_budget(std::time::Duration::from_millis(16))
            .dirty_flags(DirtyFlags {
                layout: true,
                input: true,
                paint: true,
                text_measurement: true,
                ..DirtyFlags::NONE
            })
            .route(route)
            .resources(resource_report.clone())
            .selected_node("diagnostics.sample.preview"),
    )
}

fn diagnostics_layout_cause_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.layout.panel", "Layout cause");
    if let Some(trace) =
        DebugLayoutCauseTrace::from_snapshot(snapshot, Some("diagnostics.sample.preview"))
    {
        ext_widgets::layout_cause_panel(
            ui,
            panel,
            "diagnostics.layout",
            &trace,
            ext_widgets::LayoutCausePanelOptions {
                action_prefix: Some("diagnostics.layout".to_owned()),
                max_chain_rows: 4,
                max_sibling_rows: 3,
                ..Default::default()
            },
        );
    }
}

fn diagnostics_layout_autopsy_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.layout_autopsy.panel",
        "Layout autopsy",
    );
    if let Some(trace) =
        DebugLayoutAutopsyTrace::from_snapshot(snapshot, Some("diagnostics.sample.preview"))
    {
        ext_widgets::layout_autopsy_panel(
            ui,
            panel,
            "diagnostics.layout_autopsy",
            &trace,
            ext_widgets::LayoutAutopsyPanelOptions {
                action_prefix: Some("diagnostics.layout_autopsy".to_owned()),
                max_source_rows: 8,
                ..Default::default()
            },
        );
    }
}

fn diagnostics_style_compare_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let Some(trace) = DebugNodeStyleCompareTrace::from_snapshot(
        snapshot,
        "diagnostics.sample.preview",
        "diagnostics.sample.hotspot",
    ) else {
        return;
    };
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.style_compare.panel",
        "Node style compare",
    );
    ext_widgets::node_style_compare_panel(
        ui,
        panel,
        "diagnostics.style_compare",
        &trace,
        ext_widgets::NodeStyleComparePanelOptions {
            action_prefix: Some("diagnostics.style_compare".to_owned()),
            max_difference_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_clip_scroll_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.clip.panel", "Clip and scroll");
    let mut sample = diagnostics_clip_scroll_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics clip sample layout");
    let snapshot = DebugInspectorSnapshot::from_document(&sample, &mut ApproxTextMeasurer);
    if let Some(trace) =
        DebugClipScrollTrace::from_snapshot(&snapshot, Some("diagnostics.clip.item"))
    {
        ext_widgets::clip_scroll_panel(
            ui,
            panel,
            "diagnostics.clip",
            &trace,
            ext_widgets::ClipScrollPanelOptions {
                action_prefix: Some("diagnostics.clip".to_owned()),
                max_chain_rows: 5,
                ..Default::default()
            },
        );
    }
}

fn diagnostics_clip_chain_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.clip_chain.panel", "Clip chain");
    let mut sample = diagnostics_clip_scroll_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics clip chain sample layout");
    let trace = DebugClipChainTrace::from_document(&sample);
    ext_widgets::clip_chain_panel(
        ui,
        panel,
        "diagnostics.clip_chain",
        &trace,
        ext_widgets::ClipChainPanelOptions {
            action_prefix: Some("diagnostics.clip_chain".to_owned()),
            max_clip_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_clip_chain_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.clip_chain_timeline.panel",
        "Clip chain timeline",
    );
    let trace = diagnostics_clip_chain_timeline_sample();
    ext_widgets::clip_chain_timeline_panel(
        ui,
        panel,
        "diagnostics.clip_chain_timeline",
        &trace,
        ext_widgets::ClipChainTimelinePanelOptions {
            action_prefix: Some("diagnostics.clip_chain_timeline".to_owned()),
            max_clip_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_clip_chain_timeline_sample() -> DebugClipChainTimelineTrace {
    fn frame(item_y: f32, frame_index: u64) -> DebugFrameTrace {
        let mut sample = diagnostics_clip_scroll_timeline_sample_document(20.0, item_y);
        sample
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("diagnostics clip chain timeline layout");
        DebugFrameTrace::from_document(
            &sample,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new("clip", UiSize::new(220.0, 120.0)).frame_index(frame_index),
        )
    }

    let first = frame(8.0, 30);
    let second = frame(30.0, 31);
    let third = frame(8.0, 32);
    DebugClipChainTimelineTrace::from_traces([&first, &second, &third])
}

fn diagnostics_scroll_range_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.scroll.panel", "Scroll ranges");
    let mut sample = diagnostics_clip_scroll_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics scroll sample layout");
    let snapshot = DebugInspectorSnapshot::from_document(&sample, &mut ApproxTextMeasurer);
    let trace = DebugScrollRangeTrace::from_snapshot(&snapshot);
    ext_widgets::scroll_range_panel(
        ui,
        panel,
        "diagnostics.scroll",
        &trace,
        ext_widgets::ScrollRangePanelOptions {
            action_prefix: Some("diagnostics.scroll".to_owned()),
            max_range_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_wheel_route_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.wheel.panel", "Wheel route");
    let mut sample = diagnostics_wheel_route_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 130.0), &mut ApproxTextMeasurer)
        .expect("diagnostics wheel route layout");
    let trace = DebugWheelRouteTrace::from_document(
        &sample,
        UiWheelEvent::pixels(UiPoint::new(32.0, 30.0), UiPoint::new(0.0, 28.0)),
    );
    ext_widgets::wheel_route_panel(
        ui,
        panel,
        "diagnostics.wheel",
        &trace,
        ext_widgets::WheelRoutePanelOptions {
            action_prefix: Some("diagnostics.wheel".to_owned()),
            max_candidate_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_wheel_route_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 130.0));
    let mut back_style =
        UiNodeStyle::new(operad::layout::absolute(24.0, 20.0, 116.0, 64.0)).with_z_index(1.0);
    back_style.set_clip(ClipBehavior::Clip);
    let back_scroll = sample.add_child(
        sample.root(),
        UiNode::container("diagnostics.wheel.back_scroll", back_style)
            .with_scroll(ScrollAxes::VERTICAL)
            .with_visual(UiVisual::panel(
                color(32, 68, 104),
                Some(StrokeStyle::new(color(82, 128, 172), 1.0)),
                4.0,
            )),
    );
    sample.add_child(
        back_scroll,
        UiNode::container(
            "diagnostics.wheel.content",
            LayoutStyle::size(116.0, 144.0).with_flex_shrink(0.0),
        ),
    );
    let front = UiNode::container(
        "diagnostics.wheel.front_panel",
        UiNodeStyle::new(operad::layout::absolute(12.0, 12.0, 148.0, 88.0)).with_z_index(10.0),
    )
    .with_visual(UiVisual::panel(
        color(44, 50, 58),
        Some(StrokeStyle::new(color(104, 116, 132), 1.0)),
        5.0,
    ));
    sample.add_child(sample.root(), front);
    sample
}

fn diagnostics_scroll_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.scroll_timeline.panel",
        "Scroll timeline",
    );
    let trace = diagnostics_scroll_timeline_sample();
    ext_widgets::scroll_timeline_panel(
        ui,
        panel,
        "diagnostics.scroll_timeline",
        &trace,
        ext_widgets::ScrollTimelinePanelOptions {
            action_prefix: Some("diagnostics.scroll_timeline".to_owned()),
            max_node_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_scroll_timeline_sample() -> DebugScrollTimelineTrace {
    fn frame(item_width: f32, offset: UiPoint, frame_index: u64) -> DebugFrameTrace {
        let mut document = diagnostics_clip_scroll_sample_document_with_item_width(item_width);
        document
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("diagnostics scroll timeline layout");
        if let Some(scroll_node) = document
            .nodes()
            .iter()
            .position(|node| node.name() == "diagnostics.clip.viewport")
            .map(UiNodeId::from_index)
        {
            document.set_scroll_offset(scroll_node, offset);
        }
        DebugFrameTrace::from_document(
            &document,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new("scroll", UiSize::new(220.0, 120.0))
                .frame_index(frame_index),
        )
    }

    let first = frame(78.0, UiPoint::new(0.0, 0.0), 40);
    let second = frame(180.0, UiPoint::new(28.0, 8.0), 41);
    let third = frame(72.0, UiPoint::new(0.0, 0.0), 42);
    DebugScrollTimelineTrace::from_traces([&first, &second, &third])
}

fn diagnostics_responsive_layout_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.responsive.panel",
        "Responsive triage",
    );
    let cramped = diagnostics_sample_snapshot_with_preview_size(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
        52.0,
        18.0,
    );
    let crowded = diagnostics_sample_snapshot_with_preview_size(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
        196.0,
        34.0,
    );
    let comfortable = diagnostics_sample_snapshot_with_preview_size(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
        96.0,
        32.0,
    );
    let trace = DebugResponsiveLayoutTrace::from_snapshots([
        (UiSize::new(280.0, 180.0), &cramped),
        (UiSize::new(360.0, 200.0), &crowded),
        (UiSize::new(520.0, 240.0), &comfortable),
    ]);
    ext_widgets::responsive_layout_panel(
        ui,
        panel,
        "diagnostics.responsive",
        &trace,
        ext_widgets::ResponsiveLayoutPanelOptions {
            action_prefix: Some("diagnostics.responsive".to_owned()),
            max_viewport_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_clip_scroll_sample_document() -> UiDocument {
    diagnostics_clip_scroll_sample_document_with_item_width(78.0)
}

fn diagnostics_clip_scroll_sample_document_with_item_width(item_width: f32) -> UiDocument {
    diagnostics_clip_scroll_timeline_sample_document(item_width, 26.0)
}

fn diagnostics_clip_scroll_timeline_sample_document(item_width: f32, item_y: f32) -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    let mut viewport_style = UiNodeStyle::from(
        LayoutStyle::new()
            .with_width(96.0)
            .with_height(48.0)
            .with_flex_shrink(0.0),
    );
    viewport_style.set_clip(ClipBehavior::Clip);
    let viewport = sample.add_child(
        sample.root(),
        UiNode::container("diagnostics.clip.viewport", viewport_style)
            .with_scroll(ScrollAxes::BOTH)
            .with_visual(UiVisual::panel(
                color(18, 24, 32),
                Some(StrokeStyle::new(color(62, 77, 98), 1.0)),
                4.0,
            )),
    );
    sample.add_child(
        viewport,
        UiNode::container(
            "diagnostics.clip.item",
            UiNodeStyle::new(operad::layout::absolute(70.0, item_y, item_width, 30.0)),
        )
        .with_visual(UiVisual::panel(
            color(52, 112, 180),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            4.0,
        )),
    );
    sample
}

fn diagnostics_timing_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.timing.panel", "Frame timing");
    ext_widgets::frame_timing_panel(
        ui,
        panel,
        "diagnostics.timing",
        &diagnostics_frame_timing_sample(),
        ext_widgets::FrameTimingPanelOptions {
            budget: Some(std::time::Duration::from_millis(16)),
            action_prefix: Some("diagnostics.timing".to_owned()),
            max_stage_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_frame_budget_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.budget.panel", "Frame budget");
    let trace = DebugFrameBudgetTrace::from_frame_timing(
        &diagnostics_frame_timing_sample(),
        Some(std::time::Duration::from_millis(16)),
        diagnostics_dirty_state_sample(),
    );
    ext_widgets::frame_budget_panel(
        ui,
        panel,
        "diagnostics.budget",
        &trace,
        ext_widgets::FrameBudgetPanelOptions {
            action_prefix: Some("diagnostics.budget".to_owned()),
            max_stage_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_frame_timing_waterfall_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    frame_trace: &DebugFrameTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.waterfall.panel",
        "Timing waterfall",
    );
    let trace = DebugFrameTimingWaterfallTrace::from_trace(frame_trace);
    ext_widgets::frame_timing_waterfall_panel(
        ui,
        panel,
        "diagnostics.waterfall",
        &trace,
        ext_widgets::FrameTimingWaterfallPanelOptions {
            action_prefix: Some("diagnostics.waterfall".to_owned()),
            max_section_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_slow_frame_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    frame_trace: &DebugFrameTrace,
    state: &ShowcaseState,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.slow_frame.panel", "Slow frame");
    let slow_nodes = diagnostics_slow_node_trace_sample(state);
    let cache = diagnostics_cache_reuse_sample();
    let trace = DebugSlowFrameTrace::from_trace_with_slow_nodes_and_cache(
        frame_trace,
        Some(&slow_nodes),
        Some(&cache),
    );
    ext_widgets::slow_frame_panel(
        ui,
        panel,
        "diagnostics.slow_frame",
        &trace,
        ext_widgets::SlowFramePanelOptions {
            action_prefix: Some("diagnostics.slow_frame".to_owned()),
            max_record_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_frame_bottleneck_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    frame_trace: &DebugFrameTrace,
    state: &ShowcaseState,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.bottlenecks.panel",
        "Frame bottlenecks",
    );
    let slow_nodes = diagnostics_slow_node_trace_sample(state);
    let cache = diagnostics_cache_reuse_sample();
    let trace = DebugFrameBottleneckTrace::from_trace_with_slow_nodes_and_cache(
        frame_trace,
        Some(&slow_nodes),
        Some(&cache),
    );
    ext_widgets::frame_bottleneck_panel(
        ui,
        panel,
        "diagnostics.bottlenecks",
        &trace,
        ext_widgets::FrameBottleneckPanelOptions {
            action_prefix: Some("diagnostics.bottlenecks".to_owned()),
            max_record_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_frame_autopsy_panel(ui: &mut UiDocument, parent: UiNodeId, trace: &DebugFrameTrace) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.autopsy.panel", "Frame autopsy");
    let cache = diagnostics_cache_reuse_sample();
    let autopsy = DebugFrameAutopsyTrace::from_trace_with_cache(trace, Some(&cache));
    ext_widgets::frame_autopsy_panel(
        ui,
        panel,
        "diagnostics.autopsy",
        &autopsy,
        ext_widgets::FrameAutopsyPanelOptions {
            action_prefix: Some("diagnostics.autopsy".to_owned()),
            max_source_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_frame_recorder_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    recorder: &DebugFrameRecorder,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.recorder.panel", "Frame recorder");
    ext_widgets::frame_recorder_panel(
        ui,
        panel,
        "diagnostics.recorder",
        recorder,
        ext_widgets::FrameRecorderPanelOptions {
            action_prefix: Some("diagnostics.recorder".to_owned()),
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_frame_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugFrameTimelineTrace,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.timeline.panel", "Frame timeline");
    ext_widgets::frame_timeline_panel(
        ui,
        panel,
        "diagnostics.timeline",
        trace,
        ext_widgets::FrameTimelinePanelOptions {
            action_prefix: Some("diagnostics.timeline".to_owned()),
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_frame_regression_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugFrameRegressionTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.regression.panel",
        "Frame regression",
    );
    ext_widgets::frame_regression_panel(
        ui,
        panel,
        "diagnostics.regression",
        trace,
        ext_widgets::FrameRegressionPanelOptions {
            action_prefix: Some("diagnostics.regression".to_owned()),
            max_record_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_fix_verification_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugFixVerificationTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.fix_verification.panel",
        "Fix verification",
    );
    ext_widgets::fix_verification_panel(
        ui,
        panel,
        "diagnostics.fix_verification",
        trace,
        ext_widgets::FixVerificationPanelOptions {
            action_prefix: Some("diagnostics.fix_verification".to_owned()),
            max_record_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_performance_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugPerformanceTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.performance_timeline.panel",
        "Performance timeline",
    );
    ext_widgets::performance_timeline_panel(
        ui,
        panel,
        "diagnostics.performance_timeline",
        trace,
        ext_widgets::PerformanceTimelinePanelOptions {
            action_prefix: Some("diagnostics.performance_timeline".to_owned()),
            max_stage_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_why_trace_panel(ui: &mut UiDocument, parent: UiNodeId, trace: &DebugWhyTrace) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.why.panel", "Why trace");
    ext_widgets::why_trace_panel(
        ui,
        panel,
        "diagnostics.why",
        trace,
        ext_widgets::WhyTracePanelOptions {
            action_prefix: Some("diagnostics.why".to_owned()),
            max_reason_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_why_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugWhyTimelineTrace,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.why_timeline.panel", "Why timeline");
    ext_widgets::why_timeline_panel(
        ui,
        panel,
        "diagnostics.why_timeline",
        trace,
        ext_widgets::WhyTimelinePanelOptions {
            action_prefix: Some("diagnostics.why_timeline".to_owned()),
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_node_frame_history_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    traces: &[DebugFrameTrace],
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.node_history.panel",
        "Node frame history",
    );
    let trace =
        DebugNodeFrameHistoryTrace::from_traces("diagnostics.sample.preview", traces.iter());
    ext_widgets::node_frame_history_panel(
        ui,
        panel,
        "diagnostics.node_history",
        &trace,
        ext_widgets::NodeFrameHistoryPanelOptions {
            action_prefix: Some("diagnostics.node_history".to_owned()),
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_node_change_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    traces: &[DebugFrameTrace],
) {
    let Some([before, after]) = traces.windows(2).last() else {
        return;
    };
    let panel = diagnostics_panel(ui, parent, "diagnostics.node_change.panel", "Node change");
    let trace =
        DebugNodeChangeTrace::from_frame_traces(before, after, "diagnostics.sample.preview");
    ext_widgets::node_change_panel(
        ui,
        panel,
        "diagnostics.node_change",
        &trace,
        ext_widgets::NodeChangePanelOptions {
            action_prefix: Some("diagnostics.node_change".to_owned()),
            max_change_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_frame_trace_samples(
    state: &ShowcaseState,
    resource_report: &DebugResourceReport,
) -> Vec<DebugFrameTrace> {
    let specs = [
        (
            40,
            0.0,
            false,
            FrameTiming::new()
                .section("tree-build", std::time::Duration::from_micros(700))
                .section("layout", std::time::Duration::from_micros(1_900))
                .section("render", std::time::Duration::from_micros(4_100)),
            DirtyFlags {
                paint: true,
                ..DirtyFlags::NONE
            },
            DebugResourceReport::default(),
        ),
        (
            41,
            0.6,
            false,
            FrameTiming::new()
                .section("layout", std::time::Duration::from_micros(4_400))
                .section("text", std::time::Duration::from_micros(3_200))
                .section("render", std::time::Duration::from_micros(11_400)),
            DirtyFlags {
                layout: true,
                text_measurement: true,
                paint: true,
                ..DirtyFlags::NONE
            },
            DebugResourceReport::default(),
        ),
        (
            42,
            state.diagnostics_animation_hover,
            state.diagnostics_animation_active,
            diagnostics_frame_timing_sample(),
            DirtyFlags {
                layout: true,
                input: true,
                paint: true,
                text_measurement: true,
                ..DirtyFlags::NONE
            },
            resource_report.clone(),
        ),
    ];
    specs
        .into_iter()
        .map(|(frame, hover, active, timing, dirty_flags, resources)| {
            let mut sample = diagnostics_sample_document_for(hover, active);
            sample
                .compute_layout(UiSize::new(320.0, 180.0), &mut ApproxTextMeasurer)
                .expect("diagnostics frame timeline sample layout");
            let route = DebugEventRouteTrace::from_pointer_event(
                &sample,
                DebugPointerRouteKind::Down,
                UiPoint::new(140.0, 22.0),
            );
            DebugFrameTrace::from_document(
                &sample,
                &mut ApproxTextMeasurer,
                DebugFrameTraceContext::new("diagnostics", UiSize::new(320.0, 180.0))
                    .frame_index(frame)
                    .timing(timing)
                    .timing_budget(std::time::Duration::from_millis(16))
                    .dirty_flags(dirty_flags)
                    .route(route)
                    .resources(resources)
                    .selected_node("diagnostics.sample.preview"),
            )
        })
        .collect::<Vec<_>>()
}

fn diagnostics_cache_reuse_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.cache.panel", "Cache reuse");
    let trace = diagnostics_cache_reuse_sample();
    ext_widgets::cache_reuse_panel(
        ui,
        panel,
        "diagnostics.cache",
        &trace,
        ext_widgets::CacheReusePanelOptions {
            action_prefix: Some("diagnostics.cache".to_owned()),
            max_cache_rows: 6,
            max_display_list_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_cache_reuse_sample() -> DebugCacheReuseTrace {
    let performance = PerformanceSnapshot::new(42)
        .cache(
            CacheDiagnostic::new("layout")
                .lookup(true)
                .lookup(false)
                .lookup(false),
        )
        .cache(
            CacheDiagnostic::new("shaped-text")
                .lookup(true)
                .lookup(true),
        )
        .cache(CacheDiagnostic::new("image").lookup(true))
        .cache(CacheDiagnostic::new("canvas-texture").lookup(true))
        .cache(
            CacheDiagnostic::new("display-list")
                .lookup(true)
                .lookup(false)
                .eviction(),
        );
    let display_reports = [
        DisplayListReuseReport {
            key: DisplayListKey::editor_background("diagnostics", 7),
            outcome: DisplayListReuseOutcome::Reused,
            dirty_flags: DirtyFlags::NONE,
            frame: 42,
            kind: Some(DisplayListKind::StaticBackground),
            invalidation: Some(DisplayListInvalidation::STATIC_EDITOR_BACKGROUND),
            item_count: Some(12),
            created_frame: Some(35),
            last_used_frame: Some(41),
        },
        DisplayListReuseReport {
            key: DisplayListKey::new(
                DisplayListScope::Node(UiNodeId::from_index(4)),
                "preview",
                3,
            ),
            outcome: DisplayListReuseOutcome::MissDirty,
            dirty_flags: DirtyFlags {
                paint: true,
                ..DirtyFlags::NONE
            },
            frame: 42,
            kind: Some(DisplayListKind::StaticPanel),
            invalidation: Some(DisplayListInvalidation::STATIC_PANEL),
            item_count: Some(8),
            created_frame: Some(40),
            last_used_frame: Some(41),
        },
        DisplayListReuseReport {
            key: DisplayListKey::new(DisplayListScope::custom("diagnostics"), "old-panel", 1),
            outcome: DisplayListReuseOutcome::MissEvicted,
            dirty_flags: DirtyFlags::NONE,
            frame: 42,
            kind: None,
            invalidation: None,
            item_count: None,
            created_frame: None,
            last_used_frame: None,
        },
    ];
    DebugCacheReuseTrace::from_performance_and_display_lists(&performance, &display_reports)
}

fn diagnostics_frame_timing_sample() -> FrameTiming {
    FrameTiming::new()
        .section("tree-build", std::time::Duration::from_micros(900))
        .section("layout", std::time::Duration::from_micros(2_800))
        .section("paint", std::time::Duration::from_micros(1_900))
        .section("batch", std::time::Duration::from_micros(1_100))
        .section("render", std::time::Duration::from_micros(7_600))
        .section("gpu-render", std::time::Duration::from_micros(4_200))
}

fn diagnostics_dirty_state_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.dirty.panel", "Invalidation");
    let explanation = diagnostics_dirty_state_sample();
    ext_widgets::dirty_state_panel(
        ui,
        panel,
        "diagnostics.dirty",
        &explanation,
        ext_widgets::DirtyStatePanelOptions {
            action_prefix: Some("diagnostics.dirty".to_owned()),
            max_invalidation_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_widget_state_retention_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.widget_state.panel", "Widget state");
    let trace = diagnostics_widget_state_retention_sample();
    ext_widgets::widget_state_retention_panel(
        ui,
        panel,
        "diagnostics.widget_state",
        &trace,
        ext_widgets::WidgetStateRetentionPanelOptions {
            action_prefix: Some("diagnostics.widget_state".to_owned()),
            max_state_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_widget_state_retention_sample() -> DebugWidgetStateRetentionTrace {
    let mut store =
        operad::state::RetainedWidgetStateStore::new(operad::state::WidgetStateRetention::new(2));
    let search = operad::state::WidgetKey::document("showcase", "search");
    let popover = operad::state::WidgetKey::document("showcase", "filter_popover");
    let popover_key = operad::state::WidgetStateKey::new(popover.clone(), "overlay");

    store.begin_frame([search.clone(), popover.clone()]);
    store
        .ensure(
            search.clone(),
            operad::state::WidgetStateSlotDescriptor::edit(),
            operad::state::WidgetStateValue::edit("widgets", 7),
        )
        .expect("diagnostics search edit state");
    store
        .set(
            popover.clone(),
            operad::state::WidgetStateSlotDescriptor::overlay(),
            operad::state::WidgetStateValue::overlay(true),
        )
        .expect("diagnostics popover state");
    let _ = store.set_keepalive(&popover_key, true);
    store.begin_frame([search]);
    DebugWidgetStateRetentionTrace::from_store(&store)
}

fn diagnostics_invalidation_blame_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.invalidation.panel",
        "Invalidation blame",
    );
    let trace = DebugInvalidationBlameTrace::from_dirty_state(&diagnostics_dirty_state_sample());
    ext_widgets::invalidation_blame_panel(
        ui,
        panel,
        "diagnostics.invalidation",
        &trace,
        ext_widgets::InvalidationBlamePanelOptions {
            action_prefix: Some("diagnostics.invalidation".to_owned()),
            max_trigger_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_invalidation_blast_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.invalidation_blast.panel",
        "Invalidation blast",
    );
    let trace = DebugInvalidationBlastTrace::from_dirty_state(&diagnostics_dirty_state_sample());
    ext_widgets::invalidation_blast_panel(
        ui,
        panel,
        "diagnostics.invalidation_blast",
        &trace,
        ext_widgets::InvalidationBlastPanelOptions {
            action_prefix: Some("diagnostics.invalidation_blast".to_owned()),
            max_subsystem_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_invalidation_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.invalidation_timeline.panel",
        "Invalidation timeline",
    );
    let trace = diagnostics_invalidation_timeline_sample();
    ext_widgets::invalidation_timeline_panel(
        ui,
        panel,
        "diagnostics.invalidation_timeline",
        &trace,
        ext_widgets::InvalidationTimelinePanelOptions {
            action_prefix: Some("diagnostics.invalidation_timeline".to_owned()),
            max_reason_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_dirty_state_sample() -> DirtyStateExplanation {
    let invalidations = [
        RuntimeInvalidation::new(RuntimeInvalidationReason::Resize).detail("viewport"),
        RuntimeInvalidation::new(RuntimeInvalidationReason::Input).detail("pointer moved"),
        RuntimeInvalidation::new(RuntimeInvalidationReason::Resource).detail("atlas"),
    ];
    DirtyStateExplanation::from_invalidations(&invalidations)
}

fn diagnostics_invalidation_timeline_sample() -> DebugInvalidationTimelineTrace {
    let resize_start = [
        RuntimeInvalidation::new(RuntimeInvalidationReason::Resize).detail("viewport"),
        RuntimeInvalidation::new(RuntimeInvalidationReason::Input).detail("pointer moved"),
    ];
    let resize_resource = [
        RuntimeInvalidation::new(RuntimeInvalidationReason::Resize).detail("viewport"),
        RuntimeInvalidation::new(RuntimeInvalidationReason::Resource).detail("atlas"),
    ];
    let paint_only = [
        RuntimeInvalidation::new(RuntimeInvalidationReason::Animation).detail("hover glow"),
        RuntimeInvalidation::new(RuntimeInvalidationReason::Resource).detail("atlas"),
    ];
    let first = DirtyStateExplanation::from_invalidations(&resize_start);
    let second = DirtyStateExplanation::from_invalidations(&resize_resource);
    let third = DirtyStateExplanation::from_invalidations(&paint_only);
    DebugInvalidationTimelineTrace::from_dirty_states([
        ("diagnostics #40", Some(40), &first),
        ("diagnostics #41", Some(41), &second),
        ("diagnostics #42", Some(42), &third),
    ])
}

fn diagnostics_overlap_report_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.overlaps.panel", "Overlap report");
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let report = DebugOverlapReport::from_document(&sample);
    ext_widgets::overlap_report_panel(
        ui,
        panel,
        "diagnostics.overlaps",
        &report,
        ext_widgets::OverlapReportPanelOptions {
            action_prefix: Some("diagnostics.overlaps".to_owned()),
            max_overlap_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_overlap_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugOverlapTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.overlap_timeline.panel",
        "Overlap timeline",
    );
    ext_widgets::overlap_timeline_panel(
        ui,
        panel,
        "diagnostics.overlap_timeline",
        trace,
        ext_widgets::OverlapTimelinePanelOptions {
            action_prefix: Some("diagnostics.overlap_timeline".to_owned()),
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_overlap_autopsy_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let report = DebugOverlapReport::from_document(&sample);
    let Some(trace) = DebugOverlapAutopsyTrace::from_report(&report) else {
        return;
    };
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.overlap_autopsy.panel",
        "Overlap autopsy",
    );
    ext_widgets::overlap_autopsy_panel(
        ui,
        panel,
        "diagnostics.overlap_autopsy",
        &trace,
        ext_widgets::OverlapAutopsyPanelOptions {
            action_prefix: Some("diagnostics.overlap_autopsy".to_owned()),
            max_source_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_pointer_probe_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.probe.panel", "Pointer probe");
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let probe = DebugPointerProbe::pointer_down(&sample, UiPoint::new(140.0, 22.0));
    ext_widgets::pointer_probe_panel(
        ui,
        panel,
        "diagnostics.probe",
        &probe,
        ext_widgets::PointerProbePanelOptions {
            action_prefix: Some("diagnostics.probe".to_owned()),
            max_candidate_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_pointer_session_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugPointerSessionTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.pointer_session.panel",
        "Pointer session",
    );
    ext_widgets::pointer_session_panel(
        ui,
        panel,
        "diagnostics.pointer_session",
        trace,
        ext_widgets::PointerSessionPanelOptions {
            action_prefix: Some("diagnostics.pointer_session".to_owned()),
            max_route_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_pointer_autopsy_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.pointer_autopsy.panel",
        "Pointer autopsy",
    );
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let trace = DebugPointerAutopsyTrace::pointer_down(&sample, UiPoint::new(140.0, 22.0));
    ext_widgets::pointer_autopsy_panel(
        ui,
        panel,
        "diagnostics.pointer_autopsy",
        &trace,
        ext_widgets::PointerAutopsyPanelOptions {
            action_prefix: Some("diagnostics.pointer_autopsy".to_owned()),
            max_source_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_point_autopsy_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.point_autopsy.panel",
        "Point autopsy",
    );
    let mut sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    sample
        .compute_layout(UiSize::new(260.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics point autopsy sample layout");
    let trace = DebugPointAutopsyTrace::pointer_down(
        &sample,
        &mut ApproxTextMeasurer,
        UiPoint::new(140.0, 22.0),
    );
    ext_widgets::point_autopsy_panel(
        ui,
        panel,
        "diagnostics.point_autopsy",
        &trace,
        ext_widgets::PointAutopsyPanelOptions {
            action_prefix: Some("diagnostics.point_autopsy".to_owned()),
            max_source_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_inspect_point_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.inspect_point.panel",
        "Inspect point",
    );
    let mut sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    sample
        .compute_layout(UiSize::new(260.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics inspect point sample layout");
    let trace = DebugInspectPointTrace::pointer_down(
        &sample,
        &mut ApproxTextMeasurer,
        UiPoint::new(140.0, 22.0),
    );
    ext_widgets::inspect_point_panel(
        ui,
        panel,
        "diagnostics.inspect_point",
        &trace,
        ext_widgets::InspectPointPanelOptions {
            action_prefix: Some("diagnostics.inspect_point".to_owned()),
            max_source_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_interaction_state_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.interaction.panel",
        "Interaction state",
    );
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let card = diagnostics_sample_node_id(&sample, "diagnostics.sample.card")
        .expect("diagnostics sample card");
    let preview = diagnostics_sample_node_id(&sample, "diagnostics.sample.preview")
        .expect("diagnostics sample preview");
    let hotspot = diagnostics_sample_node_id(&sample, "diagnostics.sample.hotspot")
        .expect("diagnostics sample hotspot");
    let mut host = HostInteractionState {
        hovered: Some(hotspot),
        pressed: Some(hotspot),
        focused: Some(preview),
        wheel_target: Some(card),
        input_consumed: true,
        input_consumed_by: Some(hotspot),
        active_shortcut_scopes: vec![CommandScope::Global, CommandScope::Panel],
        ..Default::default()
    };
    host.shortcut_route = Some(HostShortcutRoute {
        shortcut: Shortcut::ctrl('k'),
        active_scopes: vec![CommandScope::Global, CommandScope::Panel],
        target: Some(preview),
        command: Some(CommandId::new("diagnostics.palette")),
    });
    let trace = DebugInteractionStateTrace::from_document(
        &sample,
        DebugOverlayContext::new(host),
        DebugOverlayOptions::default(),
    );
    ext_widgets::interaction_state_panel(
        ui,
        panel,
        "diagnostics.interaction",
        &trace,
        ext_widgets::InteractionStatePanelOptions {
            action_prefix: Some("diagnostics.interaction".to_owned()),
            max_interaction_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_interaction_state_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugInteractionStateTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.interaction_timeline.panel",
        "Interaction timeline",
    );
    ext_widgets::interaction_state_timeline_panel(
        ui,
        panel,
        "diagnostics.interaction_timeline",
        trace,
        ext_widgets::InteractionStateTimelinePanelOptions {
            action_prefix: Some("diagnostics.interaction_timeline".to_owned()),
            max_interaction_rows: 5,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_interaction_state_timeline_sample(
    state: &ShowcaseState,
) -> DebugInteractionStateTimelineTrace {
    fn frame(state: &ShowcaseState, mode: &str, label: &str, frame_index: u64) -> DebugFrameTrace {
        let mut sample = diagnostics_sample_document_for(
            state.diagnostics_animation_hover,
            state.diagnostics_animation_active,
        );
        let card = diagnostics_sample_node_id(&sample, "diagnostics.sample.card")
            .expect("diagnostics sample card");
        let preview = diagnostics_sample_node_id(&sample, "diagnostics.sample.preview")
            .expect("diagnostics sample preview");
        let hotspot = diagnostics_sample_node_id(&sample, "diagnostics.sample.hotspot")
            .expect("diagnostics sample hotspot");
        let mut host = HostInteractionState::default();
        let mut gesture = None;
        match mode {
            "press" => {
                let press = GestureEvent::Press {
                    target: Some(hotspot),
                    pointer_id: PointerId::MOUSE,
                    position: UiPoint::new(140.0, 22.0),
                    button: PointerButton::Primary,
                    modifiers: operad::KeyModifiers::NONE,
                };
                host.apply_gesture(&press);
                host.focused = Some(preview);
                host.wheel_target = Some(card);
                host.input_consumed = true;
                host.input_consumed_by = Some(hotspot);
                gesture = Some(press);
            }
            "focus" => {
                host.focused = Some(preview);
            }
            _ => {}
        }
        sample
            .compute_layout(UiSize::new(260.0, 180.0), &mut ApproxTextMeasurer)
            .expect("diagnostics interaction timeline layout");
        let overlay_context = match gesture.as_ref() {
            Some(gesture) => DebugOverlayContext::new(host).active_gesture(gesture),
            None => DebugOverlayContext::new(host),
        };
        let interaction_state = DebugInteractionStateTrace::from_document(
            &sample,
            overlay_context,
            DebugOverlayOptions::default(),
        );
        DebugFrameTrace::from_document(
            &sample,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new(label, UiSize::new(260.0, 180.0))
                .frame_index(frame_index)
                .interaction_state(interaction_state),
        )
    }

    let idle = frame(state, "idle", "interaction idle", 120);
    let pressed = frame(state, "press", "interaction pressed", 121);
    let focused = frame(state, "focus", "interaction focus", 122);
    DebugInteractionStateTimelineTrace::from_traces([&idle, &pressed, &focused])
}

fn diagnostics_shortcut_route_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.shortcut_route.panel",
        "Shortcut route",
    );
    let registry = diagnostics_command_registry();
    let trace = DebugShortcutRouteTrace::from_registry(
        &registry,
        Shortcut::ctrl('k'),
        &[CommandScope::Global, CommandScope::Panel],
    );
    ext_widgets::shortcut_route_panel(
        ui,
        panel,
        "diagnostics.shortcut_route",
        &trace,
        ext_widgets::ShortcutRoutePanelOptions {
            action_prefix: Some("diagnostics.shortcut_route".to_owned()),
            max_scope_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_action_map_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.actions.panel", "Action map");
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let trace = DebugActionMapTrace::from_document(&sample);
    ext_widgets::action_map_panel(
        ui,
        panel,
        "diagnostics.actions",
        &trace,
        ext_widgets::ActionMapPanelOptions {
            action_prefix: Some("diagnostics.actions".to_owned()),
            max_action_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_action_dispatch_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.dispatch.panel", "Action dispatch");
    let trace = diagnostics_action_dispatch_sample();
    ext_widgets::action_dispatch_panel(
        ui,
        panel,
        "diagnostics.dispatch",
        &trace,
        ext_widgets::ActionDispatchPanelOptions {
            action_prefix: Some("diagnostics.dispatch".to_owned()),
            max_action_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_action_dispatch_sample() -> DebugActionDispatchTrace {
    let sample = diagnostics_sample_document_for(0.35, true);
    let preview = diagnostics_sample_node_id(&sample, "diagnostics.sample.preview")
        .expect("diagnostics sample preview");
    let hotspot = diagnostics_sample_node_id(&sample, "diagnostics.sample.hotspot")
        .expect("diagnostics sample hotspot");
    let mut queue = WidgetActionQueue::new();
    queue.push(WidgetAction::pointer_activate(
        preview,
        "diagnostics.preview.activate",
        1,
    ));
    queue.push(WidgetAction::keyboard_activate(
        hotspot,
        WidgetActionBinding::command(CommandId::new("diagnostics.inspect")),
    ));
    let registry = diagnostics_command_registry();
    DebugActionDispatchTrace::from_queue_with_registry(&queue, &registry)
}

fn diagnostics_action_map_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugActionMapTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.action_timeline.panel",
        "Action timeline",
    );
    ext_widgets::action_map_timeline_panel(
        ui,
        panel,
        "diagnostics.action_timeline",
        trace,
        ext_widgets::ActionMapTimelinePanelOptions {
            action_prefix: Some("diagnostics.action_timeline".to_owned()),
            max_action_rows: 6,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_drag_affordance_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.drag.panel", "Drag affordances");
    let mut sample = diagnostics_drag_affordance_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics drag affordance layout");
    let trace = DebugDragAffordanceTrace::from_document(&sample, &mut ApproxTextMeasurer);
    ext_widgets::drag_affordance_panel(
        ui,
        panel,
        "diagnostics.drag",
        &trace,
        ext_widgets::DragAffordancePanelOptions {
            action_prefix: Some("diagnostics.drag".to_owned()),
            max_drag_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_drag_affordance_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    let host = sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.drag.host",
            UiNodeStyle::new(operad::layout::absolute(8.0, 8.0, 172.0, 84.0)),
        )
        .with_visual(UiVisual::panel(
            color(18, 26, 36),
            Some(StrokeStyle::new(color(52, 65, 84), 1.0)),
            4.0,
        )),
    );
    sample.add_child(
        host,
        UiNode::container(
            "diagnostics.drag.edge",
            UiNodeStyle::new(operad::layout::absolute(82.0, 0.0, 2.0, 84.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.drag.edge.resize")
        .with_action_mode(WidgetActionMode::PointerEditParentRect)
        .with_visual(UiVisual::panel(
            color(92, 150, 218),
            Some(StrokeStyle::new(color(166, 210, 255), 1.0)),
            1.0,
        )),
    );
    sample.add_child(
        host,
        UiNode::container(
            "diagnostics.drag.gutter",
            UiNodeStyle::new(operad::layout::absolute(124.0, 8.0, 6.0, 68.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.drag.gutter.resize")
        .with_action_mode(WidgetActionMode::PointerEditParentRect)
        .with_visual(UiVisual::panel(
            color(72, 138, 92),
            Some(StrokeStyle::new(color(132, 214, 162), 1.0)),
            2.0,
        )),
    );
    sample
}

fn diagnostics_interaction_affordance_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.affordances.panel",
        "Interaction affordances",
    );
    let mut sample = diagnostics_interaction_affordance_sample_document();
    sample
        .compute_layout(UiSize::new(260.0, 160.0), &mut ApproxTextMeasurer)
        .expect("diagnostics affordance layout");
    let trace = DebugInteractionAffordanceTrace::from_document(&sample, &mut ApproxTextMeasurer);
    ext_widgets::interaction_affordance_panel(
        ui,
        panel,
        "diagnostics.affordances",
        &trace,
        ext_widgets::InteractionAffordancePanelOptions {
            action_prefix: Some("diagnostics.affordances".to_owned()),
            max_affordance_rows: 7,
            ..Default::default()
        },
    );
}

fn diagnostics_interaction_affordance_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugInteractionAffordanceTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.affordance_timeline.panel",
        "Affordance timeline",
    );
    ext_widgets::interaction_affordance_timeline_panel(
        ui,
        panel,
        "diagnostics.affordance_timeline",
        trace,
        ext_widgets::InteractionAffordanceTimelinePanelOptions {
            action_prefix: Some("diagnostics.affordance_timeline".to_owned()),
            max_affordance_rows: 5,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_interaction_affordance_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(260.0, 160.0));
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.affordances.good",
            UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 78.0, 30.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.affordances.good.activate")
        .with_visual(UiVisual::panel(
            color(52, 112, 180),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            5.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Button)
                .label("Good affordance")
                .focusable(),
        ),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.affordances.blank",
            UiNodeStyle::new(operad::layout::absolute(0.0, 42.0, 78.0, 30.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.affordances.blank.activate"),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.affordances.passive_action",
            UiNodeStyle::new(operad::layout::absolute(92.0, 0.0, 78.0, 30.0)),
        )
        .with_action("diagnostics.affordances.passive.activate")
        .with_visual(UiVisual::panel(
            color(70, 120, 86),
            Some(StrokeStyle::new(color(126, 196, 140), 1.0)),
            5.0,
        )),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.affordances.tiny",
            UiNodeStyle::new(operad::layout::absolute(92.0, 42.0, 12.0, 12.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(180, 72, 84),
            Some(StrokeStyle::new(color(255, 145, 155), 1.0)),
            3.0,
        )),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.affordances.glow",
            UiNodeStyle::new(operad::layout::absolute(0.0, 88.0, 72.0, 28.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.affordances.glow.activate")
        .with_material(ElementMaterial::new().with_paint_outset(LayoutInsets::points(12.0)))
        .with_visual(UiVisual::panel(
            color(34, 83, 130),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            6.0,
        )),
    );
    sample
}

fn diagnostics_paint_order_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.paint.panel", "Paint order");
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let trace = DebugPaintOrderTrace::from_document(&sample, UiPoint::new(140.0, 22.0));
    ext_widgets::paint_order_panel(
        ui,
        panel,
        "diagnostics.paint",
        &trace,
        ext_widgets::PaintOrderPanelOptions {
            action_prefix: Some("diagnostics.paint".to_owned()),
            max_stack_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_stacking_order_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.stacking.panel", "Stacking order");
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let trace = DebugStackingOrderTrace::from_document(&sample);
    ext_widgets::stacking_order_panel(
        ui,
        panel,
        "diagnostics.stacking",
        &trace,
        ext_widgets::StackingOrderPanelOptions {
            action_prefix: Some("diagnostics.stacking".to_owned()),
            max_stack_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_stacking_order_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.stacking_timeline.panel",
        "Stacking timeline",
    );
    let trace = diagnostics_stacking_order_timeline_sample();
    ext_widgets::stacking_order_timeline_panel(
        ui,
        panel,
        "diagnostics.stacking_timeline",
        &trace,
        ext_widgets::StackingOrderTimelinePanelOptions {
            action_prefix: Some("diagnostics.stacking_timeline".to_owned()),
            max_stack_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_stacking_order_timeline_sample() -> DebugStackingOrderTimelineTrace {
    fn frame(cover_z: f32, label: &str, frame_index: u64) -> DebugFrameTrace {
        let mut sample = UiDocument::new(root_style(220.0, 120.0));
        sample.add_child(
            sample.root(),
            UiNode::container(
                "diagnostics.stack.target",
                UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 92.0, 36.0)),
            )
            .with_input(InputBehavior::BUTTON)
            .with_visual(UiVisual::panel(ColorRgba::new(30, 92, 150, 255), None, 0.0)),
        );
        sample.add_child(
            sample.root(),
            UiNode::container(
                "diagnostics.stack.cover",
                UiNodeStyle::new(operad::layout::absolute(20.0, 0.0, 92.0, 36.0))
                    .with_z_index(cover_z),
            )
            .with_visual(UiVisual::panel(ColorRgba::new(130, 76, 34, 255), None, 0.0)),
        );
        sample
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("diagnostics stacking timeline layout");
        DebugFrameTrace::from_document(
            &sample,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new(label, UiSize::new(220.0, 120.0)).frame_index(frame_index),
        )
    }

    let behind = frame(-2.0, "stack target front", 40);
    let covered = frame(8.0, "stack cover front", 41);
    let restored = frame(-2.0, "stack restored", 42);
    DebugStackingOrderTimelineTrace::from_traces([&behind, &covered, &restored])
}

fn diagnostics_render_layers_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.layers.panel", "Render layers");
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let trace = DebugRenderLayerTrace::from_document(&sample);
    ext_widgets::render_layers_panel(
        ui,
        panel,
        "diagnostics.layers",
        &trace,
        ext_widgets::RenderLayersPanelOptions {
            action_prefix: Some("diagnostics.layers".to_owned()),
            max_layer_rows: 4,
            max_item_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_render_layer_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.layer_timeline.panel",
        "Render layer timeline",
    );
    let trace = diagnostics_render_layer_timeline_sample();
    ext_widgets::render_layer_timeline_panel(
        ui,
        panel,
        "diagnostics.layer_timeline",
        &trace,
        ext_widgets::RenderLayerTimelinePanelOptions {
            action_prefix: Some("diagnostics.layer_timeline".to_owned()),
            max_layer_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_render_layer_timeline_sample() -> DebugRenderLayerTimelineTrace {
    fn frame(effect: bool, frame_index: u64) -> DebugFrameTrace {
        let mut document = UiDocument::new(root_style(220.0, 120.0));
        document.add_child(
            document.root(),
            UiNode::container(
                "layer.base",
                UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 72.0, 32.0)),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(42, 120, 180, 255),
                None,
                0.0,
            )),
        );
        let mut accent = UiNode::container(
            "layer.accent",
            UiNodeStyle::new(operad::layout::absolute(86.0, 0.0, 72.0, 32.0)),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(112, 90, 190, 220),
            None,
            0.0,
        ));
        if effect {
            accent = accent.with_shader(ShaderEffect::new("debug.glow"));
        }
        document.add_child(document.root(), accent);
        document
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("diagnostics render layer timeline layout");
        DebugFrameTrace::from_document(
            &document,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new("layers", UiSize::new(220.0, 120.0))
                .frame_index(frame_index),
        )
    }

    let first = frame(false, 46);
    let second = frame(true, 47);
    let third = frame(false, 48);
    DebugRenderLayerTimelineTrace::from_traces([&first, &second, &third])
}

fn diagnostics_paint_overdraw_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.overdraw.panel", "Paint overdraw");
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let trace = DebugPaintOverdrawTrace::from_document(&sample);
    ext_widgets::paint_overdraw_panel(
        ui,
        panel,
        "diagnostics.overdraw",
        &trace,
        ext_widgets::PaintOverdrawPanelOptions {
            action_prefix: Some("diagnostics.overdraw".to_owned()),
            max_overdraw_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_paint_overdraw_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.overdraw_timeline.panel",
        "Paint overdraw timeline",
    );
    let trace = diagnostics_paint_overdraw_timeline_sample(state);
    ext_widgets::paint_overdraw_timeline_panel(
        ui,
        panel,
        "diagnostics.overdraw_timeline",
        &trace,
        ext_widgets::PaintOverdrawTimelinePanelOptions {
            action_prefix: Some("diagnostics.overdraw_timeline".to_owned()),
            max_item_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_paint_overdraw_timeline_sample(
    state: &ShowcaseState,
) -> DebugPaintOverdrawTimelineTrace {
    fn frame(mut document: UiDocument, frame_index: u64) -> DebugFrameTrace {
        document
            .compute_layout(UiSize::new(320.0, 180.0), &mut ApproxTextMeasurer)
            .expect("diagnostics overdraw timeline layout");
        DebugFrameTrace::from_document(
            &document,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new("overdraw", UiSize::new(320.0, 180.0))
                .frame_index(frame_index),
        )
    }

    let first = frame(
        diagnostics_sample_document_for(0.0, state.diagnostics_animation_active),
        40,
    );
    let second = frame(
        diagnostics_sample_document_for(
            state.diagnostics_animation_hover,
            state.diagnostics_animation_active,
        ),
        41,
    );
    let third = frame(
        diagnostics_sample_document_for(0.0, state.diagnostics_animation_active),
        42,
    );
    DebugPaintOverdrawTimelineTrace::from_traces([&first, &second, &third])
}

fn diagnostics_paint_batches_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.batches.panel", "Paint batches");
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    let layout = sample.layout_snapshot();
    let paint = sample.paint_list();
    let trace = DebugPaintBatchTrace::from_parts(&layout, &paint);
    ext_widgets::paint_batches_panel(
        ui,
        panel,
        "diagnostics.batches",
        &trace,
        ext_widgets::PaintBatchesPanelOptions {
            action_prefix: Some("diagnostics.batches".to_owned()),
            max_batch_rows: 5,
            max_break_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_paint_batch_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.batch_timeline.panel",
        "Paint batch timeline",
    );
    let trace = diagnostics_paint_batch_timeline_sample();
    ext_widgets::paint_batch_timeline_panel(
        ui,
        panel,
        "diagnostics.batch_timeline",
        &trace,
        ext_widgets::PaintBatchTimelinePanelOptions {
            action_prefix: Some("diagnostics.batch_timeline".to_owned()),
            max_reason_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_paint_batch_timeline_sample() -> DebugPaintBatchTimelineTrace {
    fn frame(shader: bool, frame_index: u64) -> DebugFrameTrace {
        let mut document = UiDocument::new(root_style(220.0, 120.0));
        document.add_child(
            document.root(),
            UiNode::container(
                "batch.left",
                UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 44.0, 24.0)),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(42, 120, 180, 255),
                None,
                0.0,
            )),
        );
        document.add_child(
            document.root(),
            UiNode::container(
                "batch.middle",
                UiNodeStyle::new(operad::layout::absolute(48.0, 0.0, 44.0, 24.0)),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(42, 120, 180, 255),
                None,
                0.0,
            )),
        );
        let mut glow = UiNode::container(
            "batch.glow",
            UiNodeStyle::new(operad::layout::absolute(96.0, 0.0, 44.0, 24.0)),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(42, 120, 180, 255),
            None,
            0.0,
        ));
        if shader {
            glow = glow.with_shader(ShaderEffect::new("debug.glow"));
        }
        document.add_child(document.root(), glow);
        document
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("diagnostics paint batch timeline layout");
        DebugFrameTrace::from_document(
            &document,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new("batches", UiSize::new(220.0, 120.0))
                .frame_index(frame_index),
        )
    }

    let first = frame(false, 50);
    let second = frame(true, 51);
    let third = frame(false, 52);
    DebugPaintBatchTimelineTrace::from_traces([&first, &second, &third])
}

fn diagnostics_visual_effects_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.effects.panel", "Visual effects");
    let mut sample = diagnostics_visual_effect_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics visual effect sample layout");
    let snapshot = DebugInspectorSnapshot::from_document(&sample, &mut ApproxTextMeasurer);
    let trace = DebugVisualEffectTrace::from_snapshot(&snapshot, Some("diagnostics.effects.glow"));
    ext_widgets::visual_effects_panel(
        ui,
        panel,
        "diagnostics.effects",
        &trace,
        ext_widgets::VisualEffectsPanelOptions {
            action_prefix: Some("diagnostics.effects".to_owned()),
            max_effect_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_visual_effect_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.effect_timeline.panel",
        "Effect timeline",
    );
    let trace = diagnostics_visual_effect_timeline_sample();
    ext_widgets::visual_effect_timeline_panel(
        ui,
        panel,
        "diagnostics.effect_timeline",
        &trace,
        ext_widgets::VisualEffectTimelinePanelOptions {
            action_prefix: Some("diagnostics.effect_timeline".to_owned()),
            max_effect_rows: 4,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_visual_effect_timeline_sample() -> DebugVisualEffectTimelineTrace {
    fn frame(label: &str, with_effect: bool, frame_index: u64) -> DebugFrameTrace {
        let mut sample = diagnostics_visual_effect_sample_document_for(with_effect);
        sample
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("diagnostics visual effect timeline layout");
        DebugFrameTrace::from_document(
            &sample,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new(label, UiSize::new(220.0, 120.0)).frame_index(frame_index),
        )
    }

    let clean = frame("effects clean", false, 60);
    let clipped = frame("effects glow", true, 61);
    let restored = frame("effects restored", false, 62);
    DebugVisualEffectTimelineTrace::from_traces([&clean, &clipped, &restored])
}

fn diagnostics_bounds_autopsy_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.bounds.panel", "Bounds autopsy");
    let mut sample = diagnostics_visual_effect_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics bounds autopsy sample layout");
    let snapshot = DebugInspectorSnapshot::from_document(&sample, &mut ApproxTextMeasurer);
    let Some(trace) =
        DebugBoundsAutopsyTrace::from_snapshot(&snapshot, Some("diagnostics.effects.glow"))
    else {
        return;
    };
    ext_widgets::bounds_autopsy_panel(
        ui,
        panel,
        "diagnostics.bounds",
        &trace,
        ext_widgets::BoundsAutopsyPanelOptions {
            action_prefix: Some("diagnostics.bounds".to_owned()),
            max_source_rows: 7,
            ..Default::default()
        },
    );
}

fn diagnostics_visual_effect_sample_document() -> UiDocument {
    diagnostics_visual_effect_sample_document_for(true)
}

fn diagnostics_visual_effect_sample_document_for(with_effect: bool) -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    let mut glow = UiNode::container(
        "diagnostics.effects.glow",
        UiNodeStyle::new(operad::layout::absolute(18.0, 20.0, 72.0, 32.0)),
    )
    .with_input(InputBehavior::BUTTON)
    .with_visual(UiVisual::panel(
        color(34, 83, 130),
        Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
        6.0,
    ))
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Button)
            .label("Glow effect target")
            .focusable(),
    );
    if with_effect {
        glow = glow.with_material(
            ElementMaterial::shader(ShaderEffect::glow(color(100, 180, 255), 0.85, 8.0))
                .with_paint_outset(LayoutInsets::points(10.0))
                .with_geometry_effect(GeometryEffect::wave(4.0)),
        );
    }
    sample.add_child(sample.root(), glow);
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.effects.marker",
            UiNodeStyle::new(operad::layout::absolute(104.0, 20.0, 52.0, 32.0)),
        )
        .with_visual(UiVisual::panel(
            color(48, 58, 72),
            Some(StrokeStyle::new(color(90, 105, 126), 1.0)),
            4.0,
        )),
    );
    sample
}

fn diagnostics_text_layout_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.text.panel", "Text layout");
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    if let Some(trace) = DebugTextLayoutTrace::from_document(
        &sample,
        &mut ApproxTextMeasurer,
        Some("diagnostics.sample.label"),
    ) {
        ext_widgets::text_layout_panel(
            ui,
            panel,
            "diagnostics.text",
            &trace,
            ext_widgets::TextLayoutPanelOptions {
                action_prefix: Some("diagnostics.text".to_owned()),
                ..Default::default()
            },
        );
    }
}

fn diagnostics_text_style_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.text_style.panel", "Text style");
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    if let Some(trace) =
        DebugTextStyleTrace::from_document(&sample, Some("diagnostics.sample.label"))
    {
        ext_widgets::text_style_panel(
            ui,
            panel,
            "diagnostics.text_style",
            &trace,
            ext_widgets::TextStylePanelOptions {
                action_prefix: Some("diagnostics.text_style".to_owned()),
                ..Default::default()
            },
        );
    }
}

fn diagnostics_text_input_state_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.text_input_state.panel",
        "Text input state",
    );
    let mut sample = TextInputState::new("alpha\nbeta").multiline(true);
    sample.paste_text_with_outcome("!");
    sample.set_selection(0, 5);
    sample.set_composing(Some("preedit".to_owned()));
    let trace = DebugTextInputStateTrace::from_state(&sample);
    ext_widgets::text_input_state_panel(
        ui,
        panel,
        "diagnostics.text_input_state",
        &trace,
        ext_widgets::TextInputStatePanelOptions {
            action_prefix: Some("diagnostics.text_input_state".to_owned()),
            ..Default::default()
        },
    );
}

fn diagnostics_text_input_event_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.text_input_event.panel",
        "Text input event",
    );
    let mut sample_doc = UiDocument::new(root_style(260.0, 90.0));
    let root = sample_doc.root();
    let mut input_state = TextInputState::new("");
    let options = TextInputOptions::default();
    let policy = options.interaction_policy();
    let input = widgets::text_input(
        &mut sample_doc,
        root,
        "diagnostics.text_input_event.sample",
        &input_state,
        options.clone(),
    );
    sample_doc
        .compute_layout(UiSize::new(260.0, 90.0), &mut ApproxTextMeasurer)
        .expect("diagnostics text input event layout");
    let metrics = operad::widgets::text_input::TextInputLayoutMetrics::new(
        UiRect::new(0.0, 0.0, 180.0, 30.0),
        8.0,
        18.0,
    );
    let context = operad::widgets::text_input::TextInputPlatformContext::for_node(
        input,
        input_state.caret_rect(metrics),
    );
    let _focus = operad::widgets::text_input::handle_text_input_event_with_options(
        &mut sample_doc,
        input,
        &mut input_state,
        &options,
        UiInputEvent::PointerDown(UiPoint::new(8.0, 8.0)),
        Some(context.clone()),
    );
    let outcome = operad::widgets::text_input::handle_text_input_event_with_options(
        &mut sample_doc,
        input,
        &mut input_state,
        &options,
        UiInputEvent::TextInput("AB".to_owned()),
        Some(context),
    );
    let trace = DebugTextInputEventTrace::from_event_outcome_with_policy(&outcome, policy);
    ext_widgets::text_input_event_panel(
        ui,
        panel,
        "diagnostics.text_input_event",
        &trace,
        ext_widgets::TextInputEventPanelOptions {
            action_prefix: Some("diagnostics.text_input_event".to_owned()),
            ..Default::default()
        },
    );
}

fn diagnostics_text_localization_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.text_localization.panel",
        "Text localization",
    );
    let sample = diagnostics_text_localization_sample_document();
    let trace = DebugTextLocalizationTrace::from_document(&sample);
    ext_widgets::text_localization_panel(
        ui,
        panel,
        "diagnostics.text_localization",
        &trace,
        ext_widgets::TextLocalizationPanelOptions {
            action_prefix: Some("diagnostics.text_localization".to_owned()),
            max_text_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_text_localization_sample_document() -> UiDocument {
    let mut style = text(13.0, color(222, 230, 240));
    style.wrap = TextWrap::None;
    let policy = LocalizationPolicy::new(LocaleId::new("ar-EG").expect("showcase locale"))
        .with_bidi(BidiPolicy::Embed);
    let mut sample = UiDocument::new(root_style(260.0, 100.0));
    sample.add_child(
        sample.root(),
        UiNode::localized_text(
            "diagnostics.text_localization.dynamic",
            DynamicLabelMeta::dynamic("toolbar.save", "Save", 7),
            Some(&policy),
            style.clone(),
            LayoutStyle::size(92.0, 18.0),
        ),
    );
    sample.add_child(
        sample.root(),
        UiNode::text(
            "diagnostics.text_localization.static",
            "Settings",
            style,
            LayoutStyle::size(128.0, 18.0),
        ),
    );
    sample
}

fn diagnostics_text_contrast_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.text_contrast.panel",
        "Text contrast",
    );
    let mut sample = diagnostics_text_contrast_sample_document();
    sample
        .compute_layout(UiSize::new(240.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics text contrast layout");
    let trace = DebugTextContrastTrace::from_document(&sample);
    ext_widgets::text_contrast_panel(
        ui,
        panel,
        "diagnostics.text_contrast",
        &trace,
        ext_widgets::TextContrastPanelOptions {
            action_prefix: Some("diagnostics.text_contrast".to_owned()),
            max_text_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_text_contrast_sample_document() -> UiDocument {
    let mut low_style = TextStyle::default();
    low_style.font_size = 13.0;
    low_style.line_height = 18.0;
    low_style.color = color(78, 84, 92);
    let mut readable_style = low_style.clone();
    readable_style.color = color(235, 242, 250);
    let mut sample = UiDocument::new(root_style(240.0, 120.0));
    let panel = sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.text_contrast.panel_bg",
            UiNodeStyle::new(operad::layout::absolute(12.0, 12.0, 188.0, 72.0)),
        )
        .with_visual(UiVisual::panel(
            color(44, 50, 58),
            Some(StrokeStyle::new(color(88, 100, 118), 1.0)),
            5.0,
        )),
    );
    sample.add_child(
        panel,
        UiNode::text(
            "diagnostics.text_contrast.low",
            "Muted label",
            low_style,
            LayoutStyle::size(124.0, 18.0),
        ),
    );
    sample.add_child(
        panel,
        UiNode::text(
            "diagnostics.text_contrast.good",
            "Readable label",
            readable_style,
            LayoutStyle::size(132.0, 18.0),
        ),
    );
    sample
}

fn diagnostics_text_fit_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.text_fit.panel", "Text fit");
    let mut sample = diagnostics_text_fit_sample_document();
    sample
        .compute_layout(UiSize::new(260.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics text fit layout");
    let trace = DebugTextFitTrace::from_document(&sample, &mut ApproxTextMeasurer);
    ext_widgets::text_fit_panel(
        ui,
        panel,
        "diagnostics.text_fit",
        &trace,
        ext_widgets::TextFitPanelOptions {
            action_prefix: Some("diagnostics.text_fit".to_owned()),
            max_text_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_text_fit_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.text_fit_timeline.panel",
        "Text fit timeline",
    );
    let trace = diagnostics_text_fit_timeline_sample();
    ext_widgets::text_fit_timeline_panel(
        ui,
        panel,
        "diagnostics.text_fit_timeline",
        &trace,
        ext_widgets::TextFitTimelinePanelOptions {
            action_prefix: Some("diagnostics.text_fit_timeline".to_owned()),
            max_text_rows: 4,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_text_fit_timeline_sample() -> DebugTextFitTimelineTrace {
    fn frame(width: f32, frame_index: u64) -> DebugFrameTrace {
        let mut style = text(13.0, color(222, 230, 240));
        style.wrap = TextWrap::None;
        let mut document = UiDocument::new(root_style(300.0, 80.0));
        document.add_child(
            document.root(),
            UiNode::text(
                "diagnostics.text_fit.timeline.long",
                "Localized toolbar label",
                style,
                LayoutStyle::size(width, 18.0),
            ),
        );
        document
            .compute_layout(UiSize::new(300.0, 80.0), &mut ApproxTextMeasurer)
            .expect("diagnostics text fit timeline layout");
        DebugFrameTrace::from_document(
            &document,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new("text", UiSize::new(300.0, 80.0)).frame_index(frame_index),
        )
    }

    let first = frame(240.0, 40);
    let second = frame(56.0, 41);
    let third = frame(240.0, 42);
    DebugTextFitTimelineTrace::from_traces([&first, &second, &third])
}

fn diagnostics_text_fit_sample_document() -> UiDocument {
    let mut style = text(13.0, color(222, 230, 240));
    style.wrap = TextWrap::None;
    let mut sample = UiDocument::new(root_style(260.0, 120.0));
    let stack = sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.text_fit.stack",
            LayoutStyle::column()
                .with_width(220.0)
                .with_height(90.0)
                .gap(6.0),
        ),
    );
    sample.add_child(
        stack,
        UiNode::text(
            "diagnostics.text_fit.short",
            "Fits",
            style.clone(),
            LayoutStyle::size(120.0, 18.0),
        ),
    );
    sample.add_child(
        stack,
        UiNode::text(
            "diagnostics.text_fit.long",
            "A very long localized toolbar label",
            style,
            LayoutStyle::size(80.0, 18.0),
        ),
    );
    sample
}

fn diagnostics_event_route_panel(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.route.panel", "Event route");
    let trace = diagnostics_sample_event_route(state);
    ext_widgets::event_route_panel(
        ui,
        panel,
        "diagnostics.route",
        &trace,
        ext_widgets::EventRoutePanelOptions {
            action_prefix: Some("diagnostics.route".to_owned()),
            max_candidate_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_frame_diff_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.diff.panel", "Frame diff");
    let previous = diagnostics_sample_snapshot_for(0.0, !state.diagnostics_animation_active);
    let diff = DebugFrameDiff::from_snapshots(&previous, snapshot);
    ext_widgets::frame_diff_panel(
        ui,
        panel,
        "diagnostics.diff",
        &diff,
        ext_widgets::FrameDiffPanelOptions {
            action_prefix: Some("diagnostics.diff".to_owned()),
            max_change_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_layout_movement_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.movement.panel", "Layout movement");
    let previous = diagnostics_sample_snapshot_with_preview_size(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
        132.0,
        24.0,
    );
    let trace = DebugLayoutMovementTrace::from_snapshots(&previous, snapshot);
    ext_widgets::layout_movement_panel(
        ui,
        panel,
        "diagnostics.movement",
        &trace,
        ext_widgets::LayoutMovementPanelOptions {
            action_prefix: Some("diagnostics.movement".to_owned()),
            max_movement_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_layout_jank_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugLayoutJankTimelineTrace,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.layout_jank.panel", "Layout jank");
    ext_widgets::layout_jank_timeline_panel(
        ui,
        panel,
        "diagnostics.layout_jank",
        trace,
        ext_widgets::LayoutJankTimelinePanelOptions {
            action_prefix: Some("diagnostics.layout_jank".to_owned()),
            max_node_rows: 6,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_constraint_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.constraints.panel", "Constraints");
    let report = DebugConstraintReport::from_snapshot(snapshot);
    ext_widgets::constraint_issues_panel(
        ui,
        panel,
        "diagnostics.constraints",
        &report,
        ext_widgets::ConstraintIssuesPanelOptions {
            action_prefix: Some("diagnostics.constraints".to_owned()),
            max_issue_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_resource_panel(ui: &mut UiDocument, parent: UiNodeId, report: &DebugResourceReport) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.resources.panel", "Resources");
    ext_widgets::resource_diagnostics_panel(
        ui,
        panel,
        "diagnostics.resources",
        report,
        ext_widgets::ResourceDiagnosticsPanelOptions {
            action_prefix: Some("diagnostics.resources".to_owned()),
            max_resource_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_resource_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.resource_timeline.panel",
        "Resource timeline",
    );
    let trace = diagnostics_resource_timeline_sample();
    ext_widgets::resource_timeline_panel(
        ui,
        panel,
        "diagnostics.resource_timeline",
        &trace,
        ext_widgets::ResourceTimelinePanelOptions {
            action_prefix: Some("diagnostics.resource_timeline".to_owned()),
            max_resource_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_resource_timeline_sample() -> DebugResourceTimelineTrace {
    fn report(upload: bool) -> DebugResourceReport {
        let handle = ResourceHandle::Image(ImageHandle::app("diagnostics.timeline"));
        let paint = PaintList {
            items: vec![diagnostics_resource_paint_item(
                UiNodeId::root(),
                handle.id().key.clone(),
                0.0,
            )],
        };
        let updates = if upload {
            vec![ResourceUpdate::rgba8_image(
                handle,
                PixelSize::new(2, 2),
                vec![96; 16],
            )]
        } else {
            Vec::new()
        };
        DebugResourceReport::from_paint_and_updates(&paint, &updates)
    }

    fn frame(upload: bool, frame_index: u64) -> DebugFrameTrace {
        let mut document = UiDocument::new(root_style(160.0, 80.0));
        document
            .compute_layout(UiSize::new(160.0, 80.0), &mut ApproxTextMeasurer)
            .expect("diagnostics resource timeline layout");
        DebugFrameTrace::from_document(
            &document,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new("resources", UiSize::new(160.0, 80.0))
                .frame_index(frame_index)
                .resources(report(upload)),
        )
    }

    let first = frame(true, 60);
    let second = frame(false, 61);
    let third = frame(true, 62);
    DebugResourceTimelineTrace::from_traces([&first, &second, &third])
}

fn diagnostics_resource_report_sample(snapshot: &DebugInspectorSnapshot) -> DebugResourceReport {
    let cover = ResourceHandle::Image(ImageHandle::app("diagnostics.cover"));
    let missing = ResourceHandle::Image(ImageHandle::app("diagnostics.missing"));
    let unused = ResourceHandle::Image(ImageHandle::app("diagnostics.unused"));
    let node = snapshot
        .node("diagnostics.sample.preview")
        .map(|node| node.id)
        .unwrap_or_else(UiNodeId::root);
    let paint = PaintList {
        items: vec![
            diagnostics_resource_paint_item(node, cover.id().key.clone(), 0.0),
            diagnostics_resource_paint_item(node, missing.id().key.clone(), 34.0),
        ],
    };
    let updates = vec![
        ResourceUpdate::rgba8_image(cover, PixelSize::new(2, 2), vec![72; 16]),
        ResourceUpdate::rgba8_image(unused, PixelSize::new(1, 1), vec![128; 4]),
    ];
    DebugResourceReport::from_paint_and_updates(&paint, &updates)
}

fn diagnostics_resource_paint_item(node: UiNodeId, key: String, x: f32) -> PaintItem {
    PaintItem {
        node,
        rect: UiRect::new(x, 0.0, 28.0, 28.0),
        clip_rect: UiRect::new(0.0, 0.0, 120.0, 48.0),
        z_index: 0.0,
        layer_order: operad::platform::LayerOrder::DEFAULT,
        opacity: 1.0,
        transform: PaintTransform::default(),
        shader: None,
        material: None,
        kind: PaintKind::Image { key, tint: None },
    }
}

fn diagnostics_node_explanation_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    snapshot: &DebugInspectorSnapshot,
    resources: &DebugResourceReport,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.explain.panel", "Explain node");
    let route = diagnostics_sample_event_route(state);
    if let Some(explanation) = DebugNodeExplanation::from_context(
        snapshot,
        Some("diagnostics.sample.preview"),
        Some(&route),
        Some(resources),
    ) {
        ext_widgets::node_explanation_panel(
            ui,
            panel,
            "diagnostics.explain",
            &explanation,
            ext_widgets::NodeExplanationPanelOptions {
                action_prefix: Some("diagnostics.explain".to_owned()),
                max_overlap_rows: 3,
                max_constraint_rows: 3,
                max_resource_rows: 3,
                ..Default::default()
            },
        );
    }
}

fn diagnostics_inspect_node_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    snapshot: &DebugInspectorSnapshot,
    resources: &DebugResourceReport,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.inspect_node.panel", "Inspect node");
    let route = diagnostics_sample_event_route(state);
    if let Some(trace) = DebugInspectNodeTrace::from_context(
        snapshot,
        Some("diagnostics.sample.preview"),
        Some(&route),
        Some(resources),
        None,
    ) {
        ext_widgets::inspect_node_panel(
            ui,
            panel,
            "diagnostics.inspect_node",
            &trace,
            ext_widgets::InspectNodePanelOptions {
                action_prefix: Some("diagnostics.inspect_node".to_owned()),
                max_source_rows: 8,
                ..Default::default()
            },
        );
    }
}

fn diagnostics_node_provenance_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
    resources: &DebugResourceReport,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.provenance.panel",
        "Node provenance",
    );
    if let Some(trace) = DebugNodeProvenanceTrace::from_context(
        snapshot,
        Some("diagnostics.sample.preview"),
        Some(resources),
    ) {
        ext_widgets::node_provenance_panel(
            ui,
            panel,
            "diagnostics.provenance",
            &trace,
            ext_widgets::NodeProvenancePanelOptions {
                action_prefix: Some("diagnostics.provenance".to_owned()),
                max_source_rows: 8,
                ..Default::default()
            },
        );
    }
}

fn diagnostics_geometry_preview(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    snapshot: &DebugInspectorSnapshot,
) {
    widgets::label(
        ui,
        parent,
        "diagnostics.hitbox.title",
        "Hitboxes",
        text(14.0, color(222, 230, 240)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let mut preview_style = UiNodeStyle::from(
        LayoutStyle::new()
            .with_width(320.0)
            .with_height(180.0)
            .with_flex_shrink(0.0),
    );
    preview_style.set_clip(ClipBehavior::Clip);
    let preview = ui.add_child(
        parent,
        UiNode::container("diagnostics.hitbox.preview", preview_style).with_visual(
            UiVisual::panel(
                color(12, 17, 24),
                Some(StrokeStyle::new(color(47, 62, 82), 1.0)),
                4.0,
            ),
        ),
    );
    let mut overlay_options = ext_widgets::HitboxDebugOverlayOptions {
        action_prefix: Some("diagnostics.hitbox.visual".to_owned()),
        selected_node: Some("diagnostics.sample.preview".to_owned()),
        show_labels: false,
        max_nodes: 16,
        max_overlaps: 8,
        ..Default::default()
    };
    overlay_options.show_paint_bounds = true;
    ext_widgets::hitbox_debug_overlay(
        ui,
        preview,
        "diagnostics.hitbox.visual",
        snapshot,
        overlay_options,
    );

    widgets::label(
        ui,
        parent,
        "diagnostics.point.preview.title",
        "Inspect point",
        text(14.0, color(222, 230, 240)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let mut point_sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    point_sample
        .compute_layout(UiSize::new(260.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics point overlay sample layout");
    let point_trace = DebugInspectPointTrace::pointer_down(
        &point_sample,
        &mut ApproxTextMeasurer,
        UiPoint::new(140.0, 22.0),
    );
    let mut point_preview_style = UiNodeStyle::from(
        LayoutStyle::new()
            .with_width(320.0)
            .with_height(150.0)
            .with_flex_shrink(0.0),
    );
    point_preview_style.set_clip(ClipBehavior::Clip);
    let point_preview = ui.add_child(
        parent,
        UiNode::container("diagnostics.point.preview", point_preview_style).with_visual(
            UiVisual::panel(
                color(12, 17, 24),
                Some(StrokeStyle::new(color(47, 62, 82), 1.0)),
                4.0,
            ),
        ),
    );
    let point_options = ext_widgets::InspectPointOverlayOptions {
        action_prefix: Some("diagnostics.point.visual".to_owned()),
        show_labels: false,
        ..Default::default()
    };
    ext_widgets::inspect_point_overlay(
        ui,
        point_preview,
        "diagnostics.point.visual",
        &point_trace,
        point_options,
    );

    widgets::label(
        ui,
        parent,
        "diagnostics.bounds.preview.title",
        "Bounds",
        text(14.0, color(222, 230, 240)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let mut bounds_sample = diagnostics_visual_effect_sample_document();
    bounds_sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics bounds overlay sample layout");
    let bounds_snapshot =
        DebugInspectorSnapshot::from_document(&bounds_sample, &mut ApproxTextMeasurer);
    let Some(bounds_trace) =
        DebugBoundsAutopsyTrace::from_snapshot(&bounds_snapshot, Some("diagnostics.effects.glow"))
    else {
        return;
    };
    let mut bounds_preview_style = UiNodeStyle::from(
        LayoutStyle::new()
            .with_width(320.0)
            .with_height(150.0)
            .with_flex_shrink(0.0),
    );
    bounds_preview_style.set_clip(ClipBehavior::Clip);
    let bounds_preview = ui.add_child(
        parent,
        UiNode::container("diagnostics.bounds.preview", bounds_preview_style).with_visual(
            UiVisual::panel(
                color(12, 17, 24),
                Some(StrokeStyle::new(color(47, 62, 82), 1.0)),
                4.0,
            ),
        ),
    );
    let bounds_options = ext_widgets::BoundsAutopsyOverlayOptions {
        action_prefix: Some("diagnostics.bounds.visual".to_owned()),
        show_labels: false,
        ..Default::default()
    };
    ext_widgets::bounds_autopsy_overlay(
        ui,
        bounds_preview,
        "diagnostics.bounds.visual",
        &bounds_trace,
        bounds_options,
    );
}

fn diagnostics_hit_targets_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.hit_targets.panel", "Hit targets");
    let mut sample = diagnostics_hit_targets_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics hit target layout");
    let trace = DebugHitTargetTrace::from_document(&sample, &mut ApproxTextMeasurer);
    ext_widgets::hit_target_panel(
        ui,
        panel,
        "diagnostics.hit_targets",
        &trace,
        ext_widgets::HitTargetPanelOptions {
            action_prefix: Some("diagnostics.hit_targets".to_owned()),
            max_target_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_hitbox_map_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.hitbox_map.panel", "Hitbox map");
    let mut sample = diagnostics_hitbox_map_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics hitbox map layout");
    let trace = DebugHitboxMapTrace::from_document(&sample);
    ext_widgets::hitbox_map_panel(
        ui,
        panel,
        "diagnostics.hitbox_map",
        &trace,
        ext_widgets::HitboxMapPanelOptions {
            action_prefix: Some("diagnostics.hitbox_map".to_owned()),
            max_hitbox_rows: 6,
            ..Default::default()
        },
    );
}

fn diagnostics_hitbox_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugHitboxTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.hitbox_timeline.panel",
        "Hitbox timeline",
    );
    ext_widgets::hitbox_timeline_panel(
        ui,
        panel,
        "diagnostics.hitbox_timeline",
        trace,
        ext_widgets::HitboxTimelinePanelOptions {
            action_prefix: Some("diagnostics.hitbox_timeline".to_owned()),
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_paint_hit_mismatch_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.paint_hit.panel",
        "Paint/hit mismatch",
    );
    let mut sample = diagnostics_paint_hit_mismatch_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics paint hit mismatch layout");
    let trace = DebugPaintHitMismatchTrace::from_document(&sample, &mut ApproxTextMeasurer);
    ext_widgets::paint_hit_mismatch_panel(
        ui,
        panel,
        "diagnostics.paint_hit",
        &trace,
        ext_widgets::PaintHitMismatchPanelOptions {
            action_prefix: Some("diagnostics.paint_hit".to_owned()),
            max_mismatch_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_paint_hit_mismatch_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.paint_hit_timeline.panel",
        "Paint/hit timeline",
    );
    let trace = diagnostics_paint_hit_mismatch_timeline_sample();
    ext_widgets::paint_hit_mismatch_timeline_panel(
        ui,
        panel,
        "diagnostics.paint_hit_timeline",
        &trace,
        ext_widgets::PaintHitMismatchTimelinePanelOptions {
            action_prefix: Some("diagnostics.paint_hit_timeline".to_owned()),
            max_mismatch_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_paint_hit_mismatch_timeline_sample() -> DebugPaintHitMismatchTimelineTrace {
    fn frame(mut document: UiDocument, label: &str, frame_index: u64) -> DebugFrameTrace {
        document
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("diagnostics paint hit timeline layout");
        DebugFrameTrace::from_document(
            &document,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new(label, UiSize::new(220.0, 120.0)).frame_index(frame_index),
        )
    }

    let clean = frame(
        diagnostics_paint_hit_mismatch_clean_sample_document(),
        "paint hit clean",
        80,
    );
    let mismatch = frame(
        diagnostics_paint_hit_mismatch_sample_document(),
        "paint hit mismatch",
        81,
    );
    let restored = frame(
        diagnostics_paint_hit_mismatch_clean_sample_document(),
        "paint hit restored",
        82,
    );
    DebugPaintHitMismatchTimelineTrace::from_traces([&clean, &mismatch, &restored])
}

fn diagnostics_visibility_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.visibility.panel", "Visibility");
    let mut sample = diagnostics_visibility_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics visibility layout");
    let trace = DebugVisibilityTrace::from_document(&sample, &mut ApproxTextMeasurer);
    ext_widgets::visibility_panel(
        ui,
        panel,
        "diagnostics.visibility",
        &trace,
        ext_widgets::VisibilityPanelOptions {
            action_prefix: Some("diagnostics.visibility".to_owned()),
            max_visibility_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_visibility_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.visibility_timeline.panel",
        "Visibility timeline",
    );
    let trace = diagnostics_visibility_timeline_sample();
    ext_widgets::visibility_timeline_panel(
        ui,
        panel,
        "diagnostics.visibility_timeline",
        &trace,
        ext_widgets::VisibilityTimelinePanelOptions {
            action_prefix: Some("diagnostics.visibility_timeline".to_owned()),
            max_issue_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_visibility_timeline_sample() -> DebugVisibilityTimelineTrace {
    fn frame(mut document: UiDocument, frame_index: u64) -> DebugFrameTrace {
        document
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("diagnostics visibility timeline layout");
        DebugFrameTrace::from_document(
            &document,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new("visibility", UiSize::new(220.0, 120.0))
                .frame_index(frame_index),
        )
    }

    let first = frame(diagnostics_visibility_sample_document(), 40);
    let second = frame(diagnostics_visibility_sample_document(), 41);
    let third = frame(diagnostics_visibility_recovered_sample_document(), 42);
    DebugVisibilityTimelineTrace::from_traces([&first, &second, &third])
}

fn diagnostics_visibility_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.visibility.transparent",
            UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 80.0, 28.0)).with_opacity(0.0),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(52, 112, 180),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            5.0,
        )),
    );
    let mut clip_style = UiNodeStyle::from(
        LayoutStyle::new()
            .with_width(44.0)
            .with_height(24.0)
            .with_flex_shrink(0.0),
    );
    clip_style.set_clip(ClipBehavior::Clip);
    let clipper = sample.add_child(
        sample.root(),
        UiNode::container("diagnostics.visibility.clipper", clip_style),
    );
    sample.add_child(
        clipper,
        UiNode::container(
            "diagnostics.visibility.clipped",
            UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 90.0, 24.0)),
        )
        .with_visual(UiVisual::panel(
            color(180, 72, 84),
            Some(StrokeStyle::new(color(255, 145, 155), 1.0)),
            4.0,
        )),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.visibility.no_paint",
            UiNodeStyle::new(operad::layout::absolute(0.0, 64.0, 80.0, 24.0)),
        )
        .with_input(InputBehavior::BUTTON),
    );
    sample
}

fn diagnostics_visibility_recovered_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.visibility.transparent",
            UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 80.0, 28.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(52, 112, 180),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            5.0,
        )),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.visibility.no_paint",
            UiNodeStyle::new(operad::layout::absolute(0.0, 64.0, 80.0, 24.0)),
        )
        .with_input(InputBehavior::BUTTON),
    );
    sample
}

fn diagnostics_paint_hit_mismatch_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.paint_hit.blank",
            UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 78.0, 30.0)),
        )
        .with_input(InputBehavior::BUTTON),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.paint_hit.glow",
            UiNodeStyle::new(operad::layout::absolute(96.0, 8.0, 72.0, 28.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_material(
            ElementMaterial::new()
                .with_paint_outset(LayoutInsets::points(12.0))
                .with_geometry_effect(GeometryEffect::wave(3.0)),
        )
        .with_visual(UiVisual::panel(
            color(34, 83, 130),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            6.0,
        )),
    );
    sample
}

fn diagnostics_paint_hit_mismatch_clean_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.paint_hit.blank",
            UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 78.0, 30.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(46, 116, 84),
            Some(StrokeStyle::new(color(121, 214, 167), 1.0)),
            6.0,
        )),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.paint_hit.glow",
            UiNodeStyle::new(operad::layout::absolute(96.0, 8.0, 72.0, 28.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(34, 83, 130),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            6.0,
        )),
    );
    sample
}

fn diagnostics_hitbox_map_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.hitbox_map.action",
            LayoutStyle::new().with_width(72.0).with_height(32.0),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.hitbox_map.action.activate")
        .with_visual(UiVisual::panel(
            color(52, 112, 180),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            5.0,
        )),
    );
    let mut clip_style = UiNodeStyle::from(
        LayoutStyle::new()
            .with_width(36.0)
            .with_height(24.0)
            .with_flex_shrink(0.0),
    );
    clip_style.set_clip(ClipBehavior::Clip);
    let clipper = sample.add_child(
        sample.root(),
        UiNode::container("diagnostics.hitbox_map.clipper", clip_style),
    );
    sample.add_child(
        clipper,
        UiNode::container(
            "diagnostics.hitbox_map.clipped",
            UiNodeStyle::new(operad::layout::absolute(0.0, 0.0, 80.0, 24.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(180, 72, 84),
            Some(StrokeStyle::new(color(255, 145, 155), 1.0)),
            4.0,
        )),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.hitbox_map.paint_only",
            UiNodeStyle::new(operad::layout::absolute(96.0, 0.0, 48.0, 24.0)),
        )
        .with_visual(UiVisual::panel(
            color(72, 105, 132),
            Some(StrokeStyle::new(color(140, 160, 190), 1.0)),
            4.0,
        )),
    );
    sample
}

fn diagnostics_hit_targets_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.hit_targets.good",
            LayoutStyle::new().with_width(72.0).with_height(32.0),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(52, 112, 180),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            5.0,
        )),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.hit_targets.tiny",
            UiNodeStyle::new(operad::layout::absolute(92.0, 0.0, 12.0, 12.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(180, 72, 84),
            Some(StrokeStyle::new(color(255, 145, 155), 1.0)),
            3.0,
        )),
    );
    sample
}

fn diagnostics_hitbox_occlusion_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.hitbox_occlusion.panel",
        "Hitbox occlusion",
    );
    let mut sample = diagnostics_hitbox_occlusion_sample_document();
    sample
        .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
        .expect("diagnostics hitbox occlusion layout");
    let trace = DebugHitboxOcclusionTrace::from_document(&sample);
    ext_widgets::hitbox_occlusion_panel(
        ui,
        panel,
        "diagnostics.hitbox_occlusion",
        &trace,
        ext_widgets::HitboxOcclusionPanelOptions {
            action_prefix: Some("diagnostics.hitbox_occlusion".to_owned()),
            max_occlusion_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_hitbox_occlusion_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugHitboxOcclusionTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.hitbox_occlusion_timeline.panel",
        "Hitbox occlusion timeline",
    );
    ext_widgets::hitbox_occlusion_timeline_panel(
        ui,
        panel,
        "diagnostics.hitbox_occlusion_timeline",
        trace,
        ext_widgets::HitboxOcclusionTimelinePanelOptions {
            action_prefix: Some("diagnostics.hitbox_occlusion_timeline".to_owned()),
            max_occlusion_rows: 4,
            max_frame_rows: 3,
            ..Default::default()
        },
    );
}

fn diagnostics_hitbox_occlusion_timeline_sample() -> DebugHitboxOcclusionTimelineTrace {
    fn frame(mut document: UiDocument, label: &str, frame_index: u64) -> DebugFrameTrace {
        document
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("diagnostics hitbox occlusion timeline layout");
        DebugFrameTrace::from_document(
            &document,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new(label, UiSize::new(220.0, 120.0)).frame_index(frame_index),
        )
    }

    let clean = frame(
        diagnostics_hitbox_occlusion_clean_sample_document(),
        "hitbox clear",
        90,
    );
    let covered = frame(
        diagnostics_hitbox_occlusion_sample_document(),
        "hitbox covered",
        91,
    );
    let restored = frame(
        diagnostics_hitbox_occlusion_clean_sample_document(),
        "hitbox restored",
        92,
    );
    DebugHitboxOcclusionTimelineTrace::from_traces([&clean, &covered, &restored])
}

fn diagnostics_hitbox_occlusion_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.hitbox_occlusion.target",
            LayoutStyle::new().with_width(96.0).with_height(36.0),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(52, 112, 180),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            5.0,
        )),
    );
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.hitbox_occlusion.cover",
            UiNodeStyle::new(operad::layout::absolute(24.0, 0.0, 96.0, 36.0)).with_z_index(8.0),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(180, 72, 84),
            Some(StrokeStyle::new(color(255, 145, 155), 1.0)),
            5.0,
        )),
    );
    sample
}

fn diagnostics_hitbox_occlusion_clean_sample_document() -> UiDocument {
    let mut sample = UiDocument::new(root_style(220.0, 120.0));
    sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.hitbox_occlusion.target",
            LayoutStyle::new().with_width(96.0).with_height(36.0),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            color(52, 112, 180),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            5.0,
        )),
    );
    sample
}

fn diagnostics_selected_node_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.inspector", "Selected node");
    let rows = snapshot
        .node("diagnostics.sample.preview")
        .map(|node| {
            vec![
                ext_widgets::PropertyGridRow::new("name", "Node", "Preview action").read_only(),
                ext_widgets::PropertyGridRow::new("role", "Role", "Button").read_only(),
                ext_widgets::PropertyGridRow::new(
                    "bounds",
                    "Bounds",
                    format!(
                        "{:.0}, {:.0}; {:.0} x {:.0}",
                        node.rect.x, node.rect.y, node.rect.width, node.rect.height
                    ),
                )
                .with_kind(ext_widgets::PropertyValueKind::Number)
                .read_only(),
                ext_widgets::PropertyGridRow::new(
                    "clip",
                    "Clip",
                    format!("{:.0} x {:.0}", node.clip_rect.width, node.clip_rect.height),
                )
                .with_kind(ext_widgets::PropertyValueKind::Number)
                .read_only(),
                ext_widgets::PropertyGridRow::new(
                    "input",
                    "Input",
                    if node.input.pointer {
                        "Receives pointer input"
                    } else {
                        "Passive"
                    },
                )
                .read_only(),
                ext_widgets::PropertyGridRow::new(
                    "hitbox",
                    "Hitbox",
                    snapshot
                        .hitbox("diagnostics.sample.preview")
                        .and_then(|hitbox| hitbox.hit_rect)
                        .map(|rect| {
                            format!(
                                "{:.0}, {:.0}; {:.0} x {:.0}",
                                rect.x, rect.y, rect.width, rect.height
                            )
                        })
                        .unwrap_or_else(|| "Not hit-testable".to_owned()),
                )
                .with_kind(ext_widgets::PropertyValueKind::Number)
                .read_only(),
                ext_widgets::PropertyGridRow::new(
                    "overlaps",
                    "Overlaps",
                    snapshot.overlaps_for(node.id).count().to_string(),
                )
                .with_kind(ext_widgets::PropertyValueKind::Number)
                .read_only(),
            ]
        })
        .unwrap_or_else(|| {
            vec![
                ext_widgets::PropertyGridRow::new("missing", "Selected node", "No node selected")
                    .read_only(),
            ]
        });
    ext_widgets::property_inspector_grid(
        ui,
        panel,
        "diagnostics.inspector.rows",
        &rows,
        diagnostics_grid_options("Selected node details"),
    );
}

fn diagnostics_animation_activity_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.animation.activity.panel",
        "Animation activity",
    );
    let trace = DebugAnimationActivityTrace::from_snapshot(snapshot);
    ext_widgets::animation_activity_panel(
        ui,
        panel,
        "diagnostics.animation.activity",
        &trace,
        ext_widgets::AnimationActivityPanelOptions {
            action_prefix: Some("diagnostics.animation.activity".to_owned()),
            max_activity_rows: 5,
            ..Default::default()
        },
    );
}

fn diagnostics_animation_activity_timeline_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    trace: &DebugAnimationActivityTimelineTrace,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.animation.timeline.panel",
        "Animation timeline",
    );
    ext_widgets::animation_activity_timeline_panel(
        ui,
        panel,
        "diagnostics.animation.timeline",
        trace,
        ext_widgets::AnimationActivityTimelinePanelOptions {
            action_prefix: Some("diagnostics.animation.timeline".to_owned()),
            max_activity_rows: 4,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_animation_activity_timeline_sample() -> DebugAnimationActivityTimelineTrace {
    fn frame(active: bool, label: &str, frame_index: u64) -> DebugFrameTrace {
        let mut sample = UiDocument::new(root_style(240.0, 140.0));
        if active {
            sample.add_child(
                sample.root(),
                UiNode::container("diagnostics.animation.pulse", LayoutStyle::size(80.0, 32.0))
                    .with_animation(
                        AnimationMachine::new(
                            vec![
                                AnimationState::new(
                                    "idle",
                                    AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0),
                                ),
                                AnimationState::new(
                                    "hot",
                                    AnimatedValues::new(1.0, UiPoint::new(20.0, 0.0), 1.1),
                                ),
                            ],
                            vec![AnimationTransition::when(
                                "idle",
                                "hot",
                                AnimationCondition::bool("hot", true),
                                0.25,
                            )],
                            "idle",
                        )
                        .expect("diagnostics animation")
                        .with_bool_input("hot", true)
                        .with_number_input("hover", 0.7)
                        .with_blend_binding(AnimationBlendBinding::new("hover", "idle", "hot")),
                    ),
            );
        }
        sample
            .compute_layout(UiSize::new(240.0, 140.0), &mut ApproxTextMeasurer)
            .expect("diagnostics animation timeline layout");
        DebugFrameTrace::from_document(
            &sample,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new(label, UiSize::new(240.0, 140.0)).frame_index(frame_index),
        )
    }

    let idle = frame(false, "animation idle", 70);
    let active = frame(true, "animation active", 71);
    let resolved = frame(false, "animation resolved", 72);
    DebugAnimationActivityTimelineTrace::from_traces([&idle, &active, &resolved])
}

fn diagnostics_animation_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    snapshot: &DebugInspectorSnapshot,
) {
    let graph_panel =
        diagnostics_panel(ui, parent, "diagnostics.animation.graph", "Animation state");
    if let Some(animation) = snapshot.animation("diagnostics.sample.preview") {
        let state_row = row(ui, graph_panel, "diagnostics.animation.graph.states", 8.0);
        for state_name in ["idle", "hot"] {
            diagnostic_chip(
                ui,
                state_row,
                format!("diagnostics.animation.graph.state.{state_name}"),
                state_name,
                animation.current_state == state_name,
            );
        }

        let graph = animation.state_graph();
        for (index, edge) in graph.edges.iter().take(2).enumerate() {
            let value = if edge.kind == DebugAnimationGraphEdgeKind::Blend {
                "Input blend"
            } else {
                "State change"
            };
            let detail = if edge.label.is_empty() {
                if edge.active { "Active" } else { "Inactive" }.to_owned()
            } else if edge.active {
                format!("{}; active", edge.label)
            } else {
                edge.label.clone()
            };
            diagnostic_value_row(
                ui,
                graph_panel,
                format!("diagnostics.animation.graph.edge.{index}"),
                value,
                format!("{} -> {}", edge.from, edge.to),
            );
            diagnostic_muted_label(
                ui,
                graph_panel,
                format!("diagnostics.animation.graph.edge.{index}.detail"),
                detail,
            );
        }
    } else {
        diagnostic_muted_label(
            ui,
            graph_panel,
            "diagnostics.animation.graph.empty",
            "No animation state machine",
        );
    }

    let controls_panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.animation.controls",
        "Animation controls",
    );
    let transport = row(
        ui,
        controls_panel,
        "diagnostics.animation.controls.transport",
        8.0,
    );
    diagnostic_button(
        ui,
        transport,
        "diagnostics.animation.controls.transport.pause_toggle",
        if state.diagnostics_animation_paused {
            "Resume"
        } else {
            "Pause"
        },
        state.diagnostics_animation_paused,
    );
    diagnostic_button(
        ui,
        transport,
        "diagnostics.animation.controls.transport.step",
        "Step",
        false,
    );
    diagnostic_slider_row(
        ui,
        controls_panel,
        "diagnostics.animation.controls.transport.scrub",
        "Scrub progress",
        state.diagnostics_animation_scrub,
        "diagnostics.animation.controls.transport.scrub",
    );
    diagnostic_button(
        ui,
        controls_panel,
        "diagnostics.animation.controls.input.active.toggle",
        if state.diagnostics_animation_active {
            "Active input: true"
        } else {
            "Active input: false"
        },
        state.diagnostics_animation_active,
    );
    diagnostic_slider_row(
        ui,
        controls_panel,
        "diagnostics.animation.controls.input.hover.set",
        "Hover blend",
        state.diagnostics_animation_hover,
        "diagnostics.animation.controls.input.hover.set",
    );
    diagnostic_button(
        ui,
        controls_panel,
        "diagnostics.animation.controls.input.pulse.fire",
        "Fire pulse",
        false,
    );
    widgets::label(
        ui,
        controls_panel,
        "diagnostics.animation.controls.status",
        format!(
            "Scrub {:.0}%   Hover {:.0}%   Pulses {}",
            state.diagnostics_animation_scrub * 100.0,
            state.diagnostics_animation_hover * 100.0,
            state.diagnostics_animation_pulse_count
        ),
        text(12.0, color(166, 180, 198)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn diagnostics_accessibility_tree_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.a11y.tree.panel",
        "Accessibility tree",
    );
    let trace = DebugAccessibilityTreeTrace::from_snapshot(snapshot);
    ext_widgets::accessibility_tree_panel(
        ui,
        panel,
        "diagnostics.a11y.tree",
        &trace,
        ext_widgets::AccessibilityTreePanelOptions {
            action_prefix: Some("diagnostics.a11y.tree".to_owned()),
            max_node_rows: 8,
            ..Default::default()
        },
    );
}

fn diagnostics_accessibility_timeline_panel(ui: &mut UiDocument, parent: UiNodeId) {
    let panel = diagnostics_panel(
        ui,
        parent,
        "diagnostics.a11y.timeline.panel",
        "Accessibility timeline",
    );
    let trace = diagnostics_accessibility_timeline_sample();
    ext_widgets::accessibility_timeline_panel(
        ui,
        panel,
        "diagnostics.a11y.timeline",
        &trace,
        ext_widgets::AccessibilityTimelinePanelOptions {
            action_prefix: Some("diagnostics.a11y.timeline".to_owned()),
            max_node_rows: 6,
            max_frame_rows: 4,
            ..Default::default()
        },
    );
}

fn diagnostics_accessibility_timeline_sample() -> DebugAccessibilityTimelineTrace {
    fn frame(accessible: bool, frame_index: u64) -> DebugFrameTrace {
        let mut document = UiDocument::new(root_style(180.0, 80.0));
        let mut node = UiNode::container(
            "diagnostics.a11y.timeline.target",
            LayoutStyle::size(80.0, 28.0),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.a11y.timeline.activate");
        if accessible {
            node = node.with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label("Timeline target")
                    .focusable()
                    .action(AccessibilityAction::new("activate", "Activate")),
            );
        }
        document.add_child(document.root(), node);
        document
            .compute_layout(UiSize::new(180.0, 80.0), &mut ApproxTextMeasurer)
            .expect("diagnostics accessibility timeline layout");
        DebugFrameTrace::from_document(
            &document,
            &mut ApproxTextMeasurer,
            DebugFrameTraceContext::new("a11y", UiSize::new(180.0, 80.0)).frame_index(frame_index),
        )
    }

    let first = frame(true, 30);
    let second = frame(false, 31);
    let third = frame(true, 32);
    DebugAccessibilityTimelineTrace::from_traces([&first, &second, &third])
}

fn diagnostics_accessibility_details(
    ui: &mut UiDocument,
    parent: UiNodeId,
    snapshot: &DebugInspectorSnapshot,
) {
    let rows = snapshot
        .accessibility_overlay
        .iter()
        .find(|node| node.name == "diagnostics.sample.preview")
        .map(|node| {
            let accessibility = node.accessibility.as_ref();
            vec![
                ext_widgets::PropertyGridRow::new("role", "Role", "Button").read_only(),
                ext_widgets::PropertyGridRow::new(
                    "label",
                    "Label",
                    accessibility
                        .and_then(|meta| meta.label.clone())
                        .unwrap_or_else(|| "Preview action".to_owned()),
                )
                .read_only(),
                ext_widgets::PropertyGridRow::new(
                    "focus",
                    "Focus order",
                    node.focus_index
                        .map(|index| format!("#{}", index + 1))
                        .unwrap_or_else(|| "Not focusable".to_owned()),
                )
                .read_only(),
                ext_widgets::PropertyGridRow::new(
                    "warnings",
                    "Warnings",
                    if node.warnings.is_empty() {
                        "None"
                    } else {
                        "Needs review"
                    },
                )
                .read_only(),
            ]
        })
        .unwrap_or_else(|| {
            vec![
                ext_widgets::PropertyGridRow::new("missing", "Accessibility", "No metadata")
                    .read_only(),
            ]
        });
    ext_widgets::property_inspector_grid(
        ui,
        parent,
        "diagnostics.a11y",
        &rows,
        diagnostics_grid_options("Accessibility metadata"),
    );
}

fn diagnostics_commands_panel(ui: &mut UiDocument, parent: UiNodeId, registry: &CommandRegistry) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.commands", "Commands");
    let formatter = ShortcutFormatter::default();
    for command_id in [
        "diagnostics.palette",
        "diagnostics.inspect",
        "diagnostics.record",
        "diagnostics.export_theme",
    ] {
        if let Some(command) = registry.command(command_id) {
            let shortcut = registry
                .command_bindings(command.meta.id.clone())
                .first()
                .map(|binding| formatter.format(binding.shortcut))
                .unwrap_or_else(|| "Unbound".to_owned());
            let status = if command.enabled {
                command
                    .meta
                    .category
                    .clone()
                    .unwrap_or_else(|| "General".to_owned())
            } else {
                command
                    .disabled_reason
                    .clone()
                    .unwrap_or_else(|| "Disabled".to_owned())
            };
            diagnostic_command_row(
                ui,
                panel,
                format!(
                    "diagnostics.commands.row.{}",
                    command.meta.id.as_str().replace('.', "_")
                ),
                &command.meta.label,
                &shortcut,
                &status,
            );
        }
    }
    diagnostic_value_row(
        ui,
        panel,
        "diagnostics.commands.conflicts",
        "Shortcut conflicts",
        if registry.conflicts().is_empty() {
            "None"
        } else {
            "Needs review"
        },
    );
}

fn diagnostics_theme_panel(ui: &mut UiDocument, parent: UiNodeId, snapshot: &DebugThemeSnapshot) {
    let panel = diagnostics_panel(ui, parent, "diagnostics.theme", "Theme tokens");
    diagnostic_value_row(
        ui,
        panel,
        "diagnostics.theme.name",
        "Theme",
        snapshot.name.as_str(),
    );
    for token_path in ["colors.accent", "colors.surface", "typography.body"] {
        if let Some(token) = snapshot.token(token_path) {
            diagnostic_value_row(
                ui,
                panel,
                format!("diagnostics.theme.token.{}", token_path.replace('.', "_")),
                token_path,
                token.value.as_str(),
            );
        }
    }
    if let Some(component) = snapshot.component_states.first() {
        diagnostic_value_row(
            ui,
            panel,
            "diagnostics.theme.component.button",
            "Button normal",
            format!(
                "{:.0} x {:.0}, padding {:.0}",
                component.min_width, component.min_height, component.padding_x
            ),
        );
    }
}

fn diagnostics_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    title: impl Into<String>,
) -> UiNodeId {
    let name = name.into();
    let title = title.into();
    let panel = ui.add_child(
        parent,
        UiNode::container(
            name.clone(),
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_padding(10.0)
                .with_gap(8.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(15, 20, 28),
            Some(StrokeStyle::new(color(52, 65, 84), 1.0)),
            4.0,
        ))
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).label(title.clone())),
    );
    widgets::label(
        ui,
        panel,
        format!("{name}.title"),
        title,
        text(13.0, color(222, 230, 240)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    panel
}

fn diagnostics_grid_options(label: impl Into<String>) -> ext_widgets::PropertyInspectorOptions {
    ext_widgets::PropertyInspectorOptions {
        label_width: 112.0,
        row_height: 28.0,
        accessibility_label: Some(label.into()),
        ..Default::default()
    }
}

fn diagnostic_value_row(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    value: impl Into<String>,
) -> UiNodeId {
    let name = name.into();
    let row = row(ui, parent, name.clone(), 8.0);
    widgets::label(
        ui,
        row,
        format!("{name}.label"),
        label.into(),
        text(12.0, color(166, 180, 198)),
        LayoutStyle::new().with_width(136.0).with_flex_shrink(0.0),
    );
    widgets::label(
        ui,
        row,
        format!("{name}.value"),
        value.into(),
        text(12.0, color(226, 234, 244)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    row
}

fn diagnostic_muted_label(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
) -> UiNodeId {
    let mut style = text(12.0, color(166, 180, 198));
    style.wrap = TextWrap::WordOrGlyph;
    widgets::label(
        ui,
        parent,
        name,
        label.into(),
        style,
        LayoutStyle::new().with_width_percent(1.0),
    )
}

fn diagnostic_command_row(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: &str,
    shortcut: &str,
    status: &str,
) -> UiNodeId {
    let name = name.into();
    let row = row(ui, parent, name.clone(), 8.0);
    widgets::label(
        ui,
        row,
        format!("{name}.label"),
        label,
        text(12.0, color(226, 234, 244)),
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_flex_grow(1.0),
    );
    widgets::label(
        ui,
        row,
        format!("{name}.shortcut"),
        shortcut,
        text(12.0, color(166, 180, 198)),
        LayoutStyle::new().with_width(78.0).with_flex_shrink(0.0),
    );
    widgets::label(
        ui,
        row,
        format!("{name}.status"),
        status,
        text(12.0, color(166, 180, 198)),
        LayoutStyle::new().with_width(140.0).with_flex_shrink(0.0),
    );
    row
}

fn diagnostic_slider_row(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    value: f32,
    action: impl Into<String>,
) -> UiNodeId {
    let name = name.into();
    let label = label.into();
    let row = row(ui, parent, format!("{name}.row"), 8.0);
    widgets::label(
        ui,
        row,
        format!("{name}.label"),
        label.clone(),
        text(12.0, color(166, 180, 198)),
        LayoutStyle::new().with_width(136.0).with_flex_shrink(0.0),
    );
    let slider_name = if name.ends_with(".set") {
        format!("{name}.slider")
    } else {
        name.clone()
    };
    let mut options = widgets::SliderOptions::default()
        .with_layout(LayoutStyle::new().with_width(160.0).with_height(24.0))
        .with_value_edit_action(action.into());
    options.accessibility_label = Some(label);
    widgets::slider(ui, row, slider_name, value, 0.0..1.0, options);
    widgets::label(
        ui,
        row,
        format!("{name}.percent"),
        format!("{:.0}%", value * 100.0),
        text(12.0, color(226, 234, 244)),
        LayoutStyle::new().with_width(46.0).with_flex_shrink(0.0),
    );
    row
}

fn diagnostic_button(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    active: bool,
) -> UiNodeId {
    let name = name.into();
    let mut options = widgets::ButtonOptions::default()
        .with_layout(LayoutStyle::new().with_height(32.0))
        .with_action(name.clone())
        .pressed(active);
    if active {
        options.visual = UiVisual::panel(
            color(47, 94, 150),
            Some(StrokeStyle::new(color(103, 164, 224), 1.0)),
            4.0,
        );
    }
    widgets::button(ui, parent, name, label, options)
}

fn diagnostic_chip(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    active: bool,
) -> UiNodeId {
    let name = name.into();
    let chip = ui.add_child(
        parent,
        UiNode::container(
            name.clone(),
            LayoutStyle::new()
                .with_width(82.0)
                .with_height(28.0)
                .with_padding(4.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(if active {
            UiVisual::panel(
                color(47, 94, 150),
                Some(StrokeStyle::new(color(103, 164, 224), 1.0)),
                4.0,
            )
        } else {
            UiVisual::panel(
                color(31, 39, 50),
                Some(StrokeStyle::new(color(62, 76, 96), 1.0)),
                4.0,
            )
        }),
    );
    widgets::label(
        ui,
        chip,
        format!("{name}.label"),
        label.into(),
        text(12.0, color(226, 234, 244)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    chip
}

fn diagnostics_sample_snapshot(state: &ShowcaseState) -> DebugInspectorSnapshot {
    diagnostics_sample_snapshot_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    )
}

fn diagnostics_sample_snapshot_for(hover: f32, active: bool) -> DebugInspectorSnapshot {
    let sample = diagnostics_sample_document_for(hover, active);
    DebugInspectorSnapshot::from_document(&sample, &mut ApproxTextMeasurer)
}

fn diagnostics_sample_snapshot_with_preview_size(
    hover: f32,
    active: bool,
    preview_width: f32,
    preview_height: f32,
) -> DebugInspectorSnapshot {
    let sample =
        diagnostics_sample_document_with_preview_size(hover, active, preview_width, preview_height);
    DebugInspectorSnapshot::from_document(&sample, &mut ApproxTextMeasurer)
}

fn diagnostics_sample_event_route(state: &ShowcaseState) -> DebugEventRouteTrace {
    let sample = diagnostics_sample_document_for(
        state.diagnostics_animation_hover,
        state.diagnostics_animation_active,
    );
    DebugEventRouteTrace::from_pointer_event(
        &sample,
        DebugPointerRouteKind::Down,
        UiPoint::new(140.0, 22.0),
    )
}

fn diagnostics_sample_node_id(document: &UiDocument, name: &str) -> Option<UiNodeId> {
    document
        .nodes()
        .iter()
        .enumerate()
        .find_map(|(index, node)| (node.name() == name).then_some(UiNodeId::from_index(index)))
}

fn diagnostics_sample_document_for(hover: f32, active: bool) -> UiDocument {
    diagnostics_sample_document_with_preview_size(hover, active, 160.0, 38.0)
}

fn diagnostics_sample_document_with_preview_size(
    hover: f32,
    active: bool,
    preview_width: f32,
    preview_height: f32,
) -> UiDocument {
    let mut sample = UiDocument::new(root_style(320.0, 180.0));
    let card = sample.add_child(
        sample.root(),
        UiNode::container(
            "diagnostics.sample.card",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(120.0)
                .padding(12.0)
                .gap(8.0),
        )
        .with_visual(UiVisual::panel(
            color(16, 22, 30),
            Some(StrokeStyle::new(color(62, 77, 98), 1.0)),
            6.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label("Diagnostics sample"),
        ),
    );
    sample.add_child(
        card,
        UiNode::container(
            "diagnostics.sample.preview",
            LayoutStyle::new()
                .with_width(preview_width)
                .with_height(preview_height),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.preview.activate")
        .with_visual(UiVisual::panel(
            color(52, 112, 180),
            Some(StrokeStyle::new(color(116, 183, 255), 1.0)),
            5.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Button)
                .label("Preview action")
                .focusable(),
        )
        .with_animation(
            AnimationMachine::new(
                vec![
                    AnimationState::new(
                        "idle",
                        AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0),
                    ),
                    AnimationState::new(
                        "hot",
                        AnimatedValues::new(0.92, UiPoint::new(18.0, 0.0), 1.08),
                    ),
                ],
                vec![AnimationTransition::when(
                    "idle",
                    "hot",
                    AnimationCondition::bool("active", true),
                    0.18,
                )],
                "idle",
            )
            .expect("sample animation")
            .with_number_input("hover", hover)
            .with_blend_binding(AnimationBlendBinding::new("hover", "idle", "hot"))
            .with_bool_input("active", active)
            .with_trigger_input("pulse"),
        ),
    );
    sample.add_child(
        card,
        UiNode::container(
            "diagnostics.sample.hotspot",
            UiNodeStyle::new(operad::layout::absolute(126.0, 10.0, 82.0, 34.0)).with_z_index(8.0),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action("diagnostics.hotspot.activate")
        .with_visual(UiVisual::panel(
            color(160, 98, 54),
            Some(StrokeStyle::new(color(255, 203, 96), 1.0)),
            5.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Button)
                .label("Overlapping debug target")
                .focusable(),
        ),
    );
    widgets::label(
        &mut sample,
        card,
        "diagnostics.sample.label",
        "Sample node",
        text(12.0, color(198, 210, 226)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    sample
        .compute_layout(UiSize::new(320.0, 180.0), &mut ApproxTextMeasurer)
        .expect("sample layout");
    sample
}

fn diagnostics_command_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    registry
        .register(
            CommandMeta::new("diagnostics.palette", "Open command palette")
                .description("Show command search")
                .category("Debug"),
        )
        .expect("command");
    registry
        .register(
            CommandMeta::new("diagnostics.inspect", "Inspect selected node")
                .description("Focus the layout inspector")
                .category("Debug"),
        )
        .expect("command");
    registry
        .register(
            CommandMeta::new("diagnostics.record", "Start interaction recording")
                .description("Capture replay steps")
                .category("Testing"),
        )
        .expect("command");
    registry
        .register(CommandMeta::new(
            "diagnostics.export_theme",
            "Export theme patch",
        ))
        .expect("command");
    registry
        .bind_shortcut(
            CommandScope::Global,
            Shortcut::ctrl('k'),
            "diagnostics.palette",
        )
        .expect("shortcut");
    registry
        .bind_shortcut(
            CommandScope::Panel,
            Shortcut::ctrl('i'),
            "diagnostics.inspect",
        )
        .expect("shortcut");
    registry
        .bind_shortcut(
            CommandScope::Panel,
            Shortcut::ctrl('r'),
            "diagnostics.record",
        )
        .expect("shortcut");
    registry
        .disable("diagnostics.export_theme", "No changes to export")
        .expect("disable");
    registry
}

fn tree_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "trees", "Tree view");
    widgets::label(
        ui,
        body,
        "trees.tree_view.title",
        "Editable tree",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    ext_widgets::tree_view(
        ui,
        body,
        "trees.tree_view",
        &editable_tree_items(&state.editable_tree),
        &state.tree,
        ext_widgets::TreeViewOptions::default().with_row_action_prefix("trees.tree"),
    );
    widgets::label(
        ui,
        body,
        "trees.editable.status",
        &state.editable_tree_status,
        text(12.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::label(
        ui,
        body,
        "trees.virtual.title",
        "Virtualized tree",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    ext_widgets::virtualized_tree_view(
        ui,
        body,
        "trees.virtual",
        &virtual_tree_items(),
        &state.tree_virtual,
        ext_widgets::VirtualTreeViewSpec::new(24.0, 112.0)
            .scroll_offset(state.tree_virtual_scroll)
            .overscan_rows(1),
        ext_widgets::TreeViewOptions::default()
            .with_row_action_prefix("trees.virtual")
            .with_scroll_action("trees.virtual.scroll"),
    );
    widgets::label(
        ui,
        body,
        "trees.table.title",
        "Tree table",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    tree_table_widgets(ui, body, state);
}

fn tree_table_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let rows = state.tree_table.visible_items(&tree_table_items());
    let columns = [
        ext_widgets::DataTableColumn::new("name", "Name", 220.0),
        ext_widgets::DataTableColumn::new("kind", "Kind", 84.0),
        ext_widgets::DataTableColumn::new("status", "Status", 92.0),
    ];
    let mut options = ext_widgets::DataTableOptions::default()
        .with_row_action_prefix("trees.table")
        .with_cell_action_prefix("trees.table")
        .with_scroll_action("trees.table.scroll");
    options.selection = state
        .tree_table
        .selected_index()
        .map(ext_widgets::DataTableSelection::single_row)
        .unwrap_or_default();
    options.layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_height(132.0)
        .with_flex_shrink(0.0);
    ext_widgets::virtualized_data_table(
        ui,
        parent,
        "trees.table",
        &columns,
        ext_widgets::VirtualDataTableSpec {
            row_count: rows.len(),
            row_height: 24.0,
            viewport_width: 396.0,
            viewport_height: 96.0,
            scroll_offset: UiPoint::new(0.0, state.tree_table_scroll),
            overscan_rows: 1,
        },
        options,
        |ui, cell_parent, cell| {
            let Some(item) = rows.get(cell.row) else {
                return;
            };
            if cell.column == 0 {
                tree_table_name_cell(ui, cell_parent, cell.row, item);
            } else {
                widgets::label(
                    ui,
                    cell_parent,
                    format!("trees.table.cell.{}.{}.label", cell.row, cell.column),
                    tree_table_cell_value(item, cell.column),
                    text(12.0, color(220, 228, 238)),
                    LayoutStyle::new().with_width_percent(1.0),
                );
            }
        },
    );
}

fn tree_table_name_cell(
    ui: &mut UiDocument,
    parent: UiNodeId,
    row: usize,
    item: &ext_widgets::TreeVisibleItem,
) {
    if item.depth > 0 {
        ui.add_child(
            parent,
            UiNode::container(
                format!("trees.table.row.{}.indent", item.id),
                LayoutStyle::new()
                    .with_width(item.depth as f32 * 16.0)
                    .with_height_percent(1.0)
                    .with_flex_shrink(0.0),
            ),
        );
    }
    widgets::label(
        ui,
        parent,
        format!("trees.table.row.{}.disclosure", item.id),
        if item.has_children() {
            if item.expanded {
                "v"
            } else {
                ">"
            }
        } else {
            ""
        },
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new()
            .with_width(18.0)
            .with_height_percent(1.0)
            .with_flex_shrink(0.0),
    );
    widgets::label(
        ui,
        parent,
        format!("trees.table.cell.{row}.0.label"),
        item.label.clone(),
        if item.disabled {
            text(12.0, color(154, 166, 184))
        } else {
            text(12.0, color(220, 228, 238))
        },
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn tree_table_cell_value(item: &ext_widgets::TreeVisibleItem, column: usize) -> String {
    match column {
        0 => item.label.clone(),
        1 => {
            if item.has_children() {
                "Folder".to_owned()
            } else {
                "File".to_owned()
            }
        }
        _ => {
            if item.disabled {
                "Locked".to_owned()
            } else if item.has_children() && item.expanded {
                "Expanded".to_owned()
            } else if item.has_children() {
                "Collapsed".to_owned()
            } else if item.expanded {
                "Expanded".to_owned()
            } else {
                "Ready".to_owned()
            }
        }
    }
}

fn tab_split_dock_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "layout_widgets",
        "Layout widgets",
        UiSize::new(640.0, 360.0),
    );
    let shell = ui.add_child(
        body,
        UiNode::container(
            "layout_widgets.dock_shell",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(360.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(13, 17, 23),
            Some(StrokeStyle::new(color(54, 65, 80), 1.0)),
            0.0,
        )),
    );

    let mut panels = base_layout_dock_panels();
    state.layout_dock.apply_order_to_panels(&mut panels);
    state.layout_dock.apply_visibility_to_panels(&mut panels);

    let mut drawer_options = ext_widgets::DockDrawerRailOptions::default();
    drawer_options.layout = LayoutStyle::row()
        .with_width_percent(1.0)
        .with_height(34.0)
        .with_padding(4.0)
        .with_gap(4.0);
    ext_widgets::dock_drawer_rail(
        ui,
        shell,
        "layout_widgets.dock.drawers",
        &[
            ext_widgets::DockDrawerDescriptor::new(
                "panel_a",
                "Panel A",
                "panel_a",
                ext_widgets::DockSide::Left,
            )
            .open(!state.layout_dock.is_hidden("panel_a"))
            .with_action("layout_widgets.drawer.panel_a"),
            ext_widgets::DockDrawerDescriptor::new(
                "panel_b",
                "Panel B",
                "panel_b",
                ext_widgets::DockSide::Right,
            )
            .open(!state.layout_dock.is_hidden("panel_b"))
            .with_action("layout_widgets.drawer.panel_b"),
        ],
        drawer_options,
    );

    let mut options = ext_widgets::DockWorkspaceOptions::default();
    options.layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_height(0.0)
        .with_flex_grow(1.0);
    options.show_titles = true;
    options.handle_thickness = 2.0;
    options.panel_visual = UiVisual::panel(
        color(18, 22, 29),
        Some(StrokeStyle::new(color(54, 65, 80), 1.0)),
        0.0,
    );
    options.center_visual = UiVisual::panel(
        color(15, 19, 25),
        Some(StrokeStyle::new(color(54, 65, 80), 1.0)),
        0.0,
    );
    options.resize_handle_visual = UiVisual::panel(color(65, 78, 96), None, 0.0);

    ext_widgets::dock_workspace(
        ui,
        shell,
        "layout_widgets.dock",
        &panels,
        options,
        |ui, parent, panel| match panel.id.as_str() {
            "panel_a" => layout_panel_contents(
                ui,
                parent,
                "layout.panel_a",
                state.layout_panel_a_scroll,
                &["Item 1", "Item 2", "Item 3", "Item 4", "Item 5", "Item 6"],
            ),
            "workspace" => layout_workspace_contents(
                ui,
                parent,
                "layout.workspace",
                state.layout_workspace_scroll,
            ),
            "panel_b" => layout_panel_contents(
                ui,
                parent,
                "layout.panel_b",
                state.layout_panel_b_scroll,
                &[
                    "Value A", "Value B", "Value C", "Value D", "Value E", "Value F",
                ],
            ),
            _ => {}
        },
    );

    if let Some(floating) = state.layout_dock.floating_panel("panel_a") {
        let floating_panel = ui.add_child(
            shell,
            UiNode::container(
                "layout_widgets.floating.panel_a",
                operad::layout::absolute(
                    floating.rect.x,
                    floating.rect.y,
                    floating.rect.width,
                    floating.rect.height,
                ),
            )
            .with_visual(UiVisual::panel(
                color(18, 22, 29),
                Some(StrokeStyle::new(color(86, 102, 124), 1.0)),
                4.0,
            )),
        );
        layout_panel_contents(
            ui,
            floating_panel,
            "layout.panel_a_floating",
            state.layout_panel_a_scroll,
            &["Item 1", "Item 2", "Item 3", "Item 4"],
        );
    }
}

fn base_layout_dock_panels() -> Vec<ext_widgets::DockPanelDescriptor> {
    vec![
        ext_widgets::DockPanelDescriptor::new(
            "panel_a",
            "Panel A",
            ext_widgets::DockSide::Left,
            200.0,
        )
        .with_min_size(150.0)
        .resizable(true),
        ext_widgets::DockPanelDescriptor::center("workspace", "Workspace").with_min_size(220.0),
        ext_widgets::DockPanelDescriptor::new(
            "panel_b",
            "Panel B",
            ext_widgets::DockSide::Right,
            200.0,
        )
        .with_min_size(150.0)
        .resizable(true),
    ]
}

fn container_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "containers",
        "Containers",
        UiSize::new(420.0, 0.0),
    );

    let frame = widgets::frame(
        ui,
        body,
        "containers.frame",
        widgets::FrameOptions::default().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(64.0)
                .with_padding(8.0)
                .with_gap(6.0),
        ),
    );
    widgets::strong_label(
        ui,
        frame,
        "containers.frame.title",
        "Frame",
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::weak_label(
        ui,
        frame,
        "containers.frame.body",
        "Framed surface with padding.",
        LayoutStyle::new().with_width_percent(1.0),
    );

    let group = widgets::group(ui, body, "containers.group");
    widgets::label(
        ui,
        group,
        "containers.group.label",
        "Group helper",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let generic_panel = widgets::panel(
        ui,
        body,
        "containers.panel",
        widgets::PanelOptions::group().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(44.0)
                .with_padding(8.0),
        ),
    );
    widgets::label(
        ui,
        generic_panel,
        "containers.panel.label",
        "Generic panel",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let group_panel = widgets::group_panel(ui, body, "containers.group_panel");
    widgets::label(
        ui,
        group_panel,
        "containers.group_panel.label",
        "Group panel",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    widgets::separator(
        ui,
        body,
        "containers.separator",
        widgets::SeparatorOptions::default(),
    );
    widgets::spacer(
        ui,
        body,
        "containers.spacer",
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height(8.0)
            .with_flex_shrink(0.0),
    );

    let grid = widgets::grid::grid(
        ui,
        body,
        "containers.grid",
        widgets::grid::GridOptions::default().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(78.0)
                .with_gap(4.0),
        ),
    );
    for row_index in 0..2 {
        let row = widgets::grid::grid_row(
            ui,
            grid,
            format!("containers.grid.row.{row_index}"),
            widgets::grid::GridRowOptions::default(),
        );
        for column_index in 0..3 {
            widgets::grid::grid_text_cell(
                ui,
                row,
                format!("containers.grid.row.{row_index}.cell.{column_index}"),
                format!("R{} C{}", row_index + 1, column_index + 1),
                widgets::grid::GridCellOptions {
                    text_style: text(12.0, color(214, 224, 238)),
                    ..Default::default()
                },
            );
        }
    }

    widgets::sides(
        ui,
        body,
        "containers.sides",
        widgets::SidesOptions::default()
            .with_layout(LayoutStyle::row().with_width_percent(1.0).with_height(48.0))
            .with_gap(8.0)
            .with_visual(UiVisual::panel(
                color(20, 25, 32),
                Some(StrokeStyle::new(color(58, 68, 84), 1.0)),
                4.0,
            )),
        |ui, left| {
            widgets::label(
                ui,
                left,
                "containers.sides.left.label",
                "Left side",
                text(12.0, color(220, 228, 238)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
        |ui, right| {
            widgets::label(
                ui,
                right,
                "containers.sides.right.label",
                "Right side",
                text(12.0, color(220, 228, 238)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
    );

    widgets::columns(
        ui,
        body,
        "containers.columns",
        3,
        widgets::ColumnsOptions::default()
            .with_layout(LayoutStyle::row().with_width_percent(1.0).with_height(48.0))
            .with_gap(8.0),
        |ui, column, index| {
            widgets::label(
                ui,
                column,
                format!("containers.columns.{index}.label"),
                format!("Column {}", index + 1),
                text(12.0, color(220, 228, 238)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
    );

    let indented = widgets::indented_section(
        ui,
        body,
        "containers.indented",
        widgets::IndentOptions::default().with_amount(24.0),
    );
    widgets::label(
        ui,
        indented,
        "containers.indented.label",
        "Indented section",
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    widgets::resize_container(
        ui,
        body,
        "containers.resize_container",
        widgets::ResizeContainerOptions::default().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(92.0)
                .with_flex_shrink(0.0),
        ),
        |ui, content| {
            widgets::label(
                ui,
                content,
                "containers.resize_container.label",
                "Resize container",
                text(12.0, color(220, 228, 238)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
    );

    widgets::scene(
        ui,
        body,
        "containers.scene",
        vec![
            ScenePrimitive::Rect(
                PaintRect::solid(UiRect::new(8.0, 12.0, 108.0, 46.0), color(48, 112, 184))
                    .stroke(AlignedStroke::inside(StrokeStyle::new(
                        color(132, 174, 222),
                        1.0,
                    )))
                    .corner_radii(CornerRadii::uniform(6.0)),
            ),
            ScenePrimitive::Circle {
                center: UiPoint::new(150.0, 35.0),
                radius: 22.0,
                fill: color(111, 203, 159),
                stroke: Some(StrokeStyle::new(color(176, 236, 206), 1.0)),
            },
            ScenePrimitive::Line {
                from: UiPoint::new(188.0, 18.0),
                to: UiPoint::new(238.0, 52.0),
                stroke: StrokeStyle::new(color(232, 186, 88), 3.0),
            },
        ],
        widgets::SceneOptions::default()
            .with_layout(LayoutStyle::new().with_width(260.0).with_height(70.0))
            .accessibility_label("Scene primitives"),
    );

    widgets::scroll_container(
        ui,
        body,
        "containers.scroll_area_with_bars",
        state.containers_scroll,
        widgets::ScrollContainerOptions::default()
            .with_axes(ScrollAxes::VERTICAL)
            .with_layout(LayoutStyle::column().with_width(260.0).with_height(116.0))
            .with_viewport_layout(
                LayoutStyle::column()
                    .with_width(0.0)
                    .with_height_percent(1.0)
                    .with_flex_grow(1.0)
                    .with_flex_shrink(1.0),
            ),
        |ui, viewport| {
            for index in 0..5 {
                widgets::label(
                    ui,
                    viewport,
                    format!("containers.scroll_area_with_bars.row.{index}"),
                    format!("Scrollable row {}", index + 1),
                    text(12.0, color(200, 212, 228)),
                    LayoutStyle::new()
                        .with_width(232.0)
                        .with_height(28.0)
                        .with_flex_shrink(0.0),
                );
            }
        },
    );

    widgets::label(
        ui,
        body,
        "containers.area.title",
        "Absolute area",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let area_host = ui.add_child(
        body,
        UiNode::container(
            "containers.area.host",
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(82.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(17, 20, 25),
            Some(StrokeStyle::new(color(58, 68, 84), 1.0)),
            4.0,
        )),
    );
    widgets::container::area(
        ui,
        area_host,
        "containers.area",
        widgets::container::AreaOptions::new(UiRect::new(14.0, 14.0, 180.0, 44.0))
            .with_visual(UiVisual::panel(color(39, 72, 109), None, 4.0))
            .accessibility_label("Absolute positioned area"),
        |ui, area| {
            widgets::label(
                ui,
                area,
                "containers.area.label",
                "Area",
                text(12.0, color(238, 244, 252)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
    );
}

fn panel_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(ui, parent, "panels", "Panels", UiSize::new(520.0, 320.0));
    widgets::label(
        ui,
        body,
        "panels.title",
        "Drag the split bars to resize the docked panels.",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let shell = widgets::frame(
        ui,
        body,
        "panels.shell",
        widgets::FrameOptions::default().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(260.0)
                .with_flex_grow(1.0)
                .with_padding(0.0)
                .with_gap(0.0),
        ),
    );
    ext_widgets::split_pane(
        ui,
        shell,
        "panels.top_split",
        ext_widgets::SplitAxis::Vertical,
        state.panels_top_split,
        panel_split_options("panels.resize.top"),
        |ui, top| {
            panel_region(
                ui,
                top,
                "panels.top",
                widgets::PanelKind::Top,
                "Top",
                "Header controls",
            );
        },
        |ui, lower| {
            ext_widgets::split_pane(
                ui,
                lower,
                "panels.bottom_split",
                ext_widgets::SplitAxis::Vertical,
                state.panels_bottom_split,
                panel_split_options("panels.resize.bottom"),
                |ui, middle| {
                    ext_widgets::split_pane(
                        ui,
                        middle,
                        "panels.left_split",
                        ext_widgets::SplitAxis::Horizontal,
                        state.panels_left_split,
                        panel_split_options("panels.resize.left"),
                        |ui, left| {
                            panel_region(
                                ui,
                                left,
                                "panels.left",
                                widgets::PanelKind::Left,
                                "Left",
                                "Navigation",
                            );
                        },
                        |ui, center_and_right| {
                            ext_widgets::split_pane(
                                ui,
                                center_and_right,
                                "panels.right_split",
                                ext_widgets::SplitAxis::Horizontal,
                                state.panels_right_split,
                                panel_split_options("panels.resize.right"),
                                |ui, center| {
                                    panel_region(
                                        ui,
                                        center,
                                        "panels.center",
                                        widgets::PanelKind::Central,
                                        "Central",
                                        "Primary workspace",
                                    );
                                },
                                |ui, right| {
                                    panel_region(
                                        ui,
                                        right,
                                        "panels.right",
                                        widgets::PanelKind::Right,
                                        "Right",
                                        "Inspector",
                                    );
                                },
                            );
                        },
                    );
                },
                |ui, bottom| {
                    panel_region(
                        ui,
                        bottom,
                        "panels.bottom",
                        widgets::PanelKind::Bottom,
                        "Bottom",
                        "Status and output",
                    );
                },
            );
        },
    );
}

fn panel_split_options(action: &'static str) -> ext_widgets::SplitPaneOptions {
    let mut options = ext_widgets::SplitPaneOptions::default().with_handle_action(action);
    options.handle_thickness = PANELS_SPLIT_HANDLE_THICKNESS;
    options.handle_visual = UiVisual::panel(color(58, 70, 88), None, 0.0);
    options.handle_hover_visual = Some(UiVisual::panel(color(100, 172, 244), None, 0.0));
    options
}

fn panel_region(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    kind: widgets::PanelKind,
    title: &'static str,
    detail: &'static str,
) -> UiNodeId {
    let panel = widgets::panel(
        ui,
        parent,
        name,
        widgets::PanelOptions {
            kind,
            layout: LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0)
                .with_padding(10.0)
                .with_gap(6.0),
            visual: UiVisual::panel(
                color(18, 23, 31),
                Some(StrokeStyle::new(color(66, 80, 98), 1.0)),
                0.0,
            ),
            accessibility_label: Some(title.to_string()),
            ..Default::default()
        },
    );
    widgets::label(
        ui,
        panel,
        format!("{name}.label"),
        title,
        text(13.0, color(232, 240, 250)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::label(
        ui,
        panel,
        format!("{name}.detail"),
        detail,
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    panel
}

fn form_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(ui, parent, "forms", "Forms", UiSize::new(390.0, 0.0));
    let section = widgets::form_section(
        ui,
        body,
        "forms.profile",
        None::<String>,
        widgets::FormSectionOptions::default().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_padding(12.0)
                .with_gap(10.0),
        ),
    );
    profile_form_summary(ui, section.root, state);

    let status_row = wrapping_row(ui, section.root, "forms.profile.status_flags", 6.0);
    form_status_chip(
        ui,
        status_row,
        "forms.profile.status.dirty",
        "dirty",
        state.form.dirty,
    );
    form_status_chip(
        ui,
        status_row,
        "forms.profile.status.pending",
        "pending",
        state.form.pending,
    );
    form_status_chip(
        ui,
        status_row,
        "forms.profile.status.submitted",
        "submitted",
        state.form.submitted,
    );

    let mut name_options = widgets::FormRowOptions::default().required();
    if state.form_name_text.text().trim().is_empty() {
        name_options = name_options.invalid("Name is required");
    }
    let name = widgets::form_row(ui, section.root, "forms.profile.name", name_options);
    widgets::field_label(
        ui,
        name,
        "forms.profile.name.label",
        "Name",
        widgets::FieldLabelOptions::default().required(),
    );
    form_text_field(
        ui,
        name,
        "forms.profile.name.input",
        &state.form_name_text,
        FocusedTextInput::FormName,
        state,
    );
    if state.form_name_text.text().trim().is_empty() {
        widgets::field_validation_message(
            ui,
            name,
            "forms.profile.name.validation",
            ValidationMessage::error("Name is required"),
            widgets::ValidationMessageOptions::default(),
        );
    } else {
        widgets::field_help_text(
            ui,
            name,
            "forms.profile.name.help",
            "Shown in window titles and project lists.",
            widgets::FieldHelpOptions::default(),
        );
    }

    let mut email_options = widgets::FormRowOptions::default().required();
    if !profile_email_valid(state.form_email_text.text()) {
        email_options = email_options.invalid("Use a complete email address");
    }
    let email = widgets::form_row(ui, section.root, "forms.profile.email", email_options);
    widgets::field_label(
        ui,
        email,
        "forms.profile.email.label",
        "Email",
        widgets::FieldLabelOptions::default().required(),
    );
    form_text_field(
        ui,
        email,
        "forms.profile.email.input",
        &state.form_email_text,
        FocusedTextInput::FormEmail,
        state,
    );
    if profile_email_valid(state.form_email_text.text()) {
        widgets::field_help_text(
            ui,
            email,
            "forms.profile.email.help",
            "Used for workspace invites and notifications.",
            widgets::FieldHelpOptions::default(),
        );
    } else {
        widgets::field_validation_message(
            ui,
            email,
            "forms.profile.email.validation",
            ValidationMessage::error("Use a complete email address"),
            widgets::ValidationMessageOptions::default(),
        );
    }

    let role = widgets::form_row(
        ui,
        section.root,
        "forms.profile.role",
        widgets::FormRowOptions::default(),
    );
    widgets::field_label(
        ui,
        role,
        "forms.profile.role.label",
        "Role",
        widgets::FieldLabelOptions::default(),
    );
    form_text_field(
        ui,
        role,
        "forms.profile.role.input",
        &state.form_role_text,
        FocusedTextInput::FormRole,
        state,
    );
    widgets::field_validation_message(
        ui,
        role,
        "forms.profile.role.help",
        if state.form_role_text.text().trim().is_empty() {
            ValidationMessage::warning("Role can be added later")
        } else {
            ValidationMessage::info(
                "Form rows compose labels, controls, help, and validation text.",
            )
        },
        widgets::ValidationMessageOptions::default(),
    );

    let newsletter = widgets::form_row(
        ui,
        section.root,
        "forms.profile.newsletter",
        widgets::FormRowOptions::default().with_accessibility_label("Newsletter preference"),
    );
    let mut newsletter_options =
        widgets::CheckboxOptions::default().with_action("forms.profile.newsletter.toggle");
    newsletter_options.layout = LayoutStyle::new().with_width_percent(1.0).with_height(30.0);
    newsletter_options.text_style = text(12.0, color(220, 228, 238));
    widgets::checkbox(
        ui,
        newsletter,
        "forms.profile.newsletter.input",
        "Send release notes",
        state.form_newsletter,
        newsletter_options,
    );
    widgets::field_help_text(
        ui,
        newsletter,
        "forms.profile.newsletter.help",
        "Checkboxes participate in the same form state as text fields.",
        widgets::FieldHelpOptions::default(),
    );

    widgets::form_error_summary(
        ui,
        section.root,
        "forms.profile.errors",
        &state.form,
        widgets::FormErrorSummaryOptions::default(),
    );
    widgets::label(
        ui,
        section.root,
        "forms.profile.action_help",
        "Apply changes saves this draft and keeps editing. Submit profile saves and marks it submitted.",
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let action_layout = Layout::row()
        .size(LayoutSize::new(
            LayoutDimension::percent(1.0),
            LayoutDimension::Auto,
        ))
        .gap(LayoutGap::points(8.0, 8.0))
        .flex_wrap(LayoutFlexWrap::Wrap)
        .to_layout_style();
    widgets::form_action_buttons(
        ui,
        section.root,
        "forms.profile.actions",
        &state.form,
        widgets::FormActionButtonsOptions::default()
            .with_layout(action_layout)
            .with_labels(widgets::FormActionLabels {
                submit: "Submit profile".to_string(),
                apply: "Apply changes".to_string(),
                cancel: "Cancel".to_string(),
                reset: "Reset".to_string(),
            })
            .include_reset(true)
            .with_action_prefix("forms.profile"),
    );
    widgets::label(
        ui,
        section.root,
        "forms.profile.status",
        format!("Status: {}", state.form_status),
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn overlay_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body =
        section_with_min_viewport(ui, parent, "overlays", "Overlays", UiSize::new(420.0, 0.0));
    let header = widgets::collapsing_header(
        ui,
        body,
        "overlays.collapsing",
        "Collapsing header",
        widgets::CollapsingHeaderOptions::default()
            .expanded(state.overlay_expanded)
            .with_toggle_action("overlays.collapsing.toggle"),
    );
    if let Some(panel) = header.body {
        widgets::label(
            ui,
            panel,
            "overlays.collapsing.body_text",
            "Expanded content lives under the header and remains part of normal layout.",
            text(12.0, color(196, 210, 230)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }

    let controls = wrapping_row(ui, body, "overlays.controls", 8.0);
    let tooltip_visual = button_visual(58, 78, 96);
    let mut tooltip_options = widgets::ButtonOptions::new(LayoutStyle::new().with_height(32.0));
    tooltip_options.visual = tooltip_visual;
    tooltip_options.hovered_visual = Some(readable_button_hover_visual(tooltip_visual));
    tooltip_options.pressed_visual = Some(adjusted_button_visual(tooltip_visual, -62));
    tooltip_options.pressed_hovered_visual = Some(adjusted_button_visual(tooltip_visual, -24));
    tooltip_options.text_style = text(13.0, color(246, 249, 252));
    let tooltip_target = widgets::button(
        ui,
        controls,
        "overlays.tooltip_target",
        "Tooltip target",
        tooltip_options,
    );
    ui.node_mut(tooltip_target).set_tooltip(
        TooltipContent::new("Tooltip")
            .body("Tooltips render as overlay surfaces anchored to a target.")
            .shortcut_label("Ctrl+K")
            .disabled_reason("Disabled reasons can be announced without changing the trigger."),
    );
    ui.node_mut(tooltip_target)
        .set_tooltip_placement(TooltipPlacement::Below);
    ui.node_mut(tooltip_target)
        .set_tooltip_size(UiSize::new(240.0, 148.0));
    button(
        ui,
        controls,
        "overlays.popup.toggle",
        if state.overlay_popup_open {
            "Close popup"
        } else {
            "Open popup"
        },
        "overlays.popup.toggle",
        button_visual(48, 112, 184),
    );
    button(
        ui,
        controls,
        "overlays.modal.open",
        "Open modal",
        "overlays.modal.open",
        button_visual(58, 78, 96),
    );

    widgets::label(
        ui,
        body,
        "overlays.tooltip_rect.label",
        "A right-edge target keeps its tooltip inside the preview.",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let preview_viewport = UiRect::new(0.0, 0.0, 420.0, 112.0);
    let tooltip_target = UiRect::new(328.0, 42.0, 64.0, 28.0);
    let tooltip_size = UiSize::new(176.0, 58.0);
    let placed_tooltip = widgets::tooltip::tooltip_rect(
        tooltip_target,
        tooltip_size,
        preview_viewport,
        TooltipPlacement::Right,
        8.0,
        None,
    );
    let clamped_preview = ui.add_child(
        body,
        UiNode::container(
            "overlays.tooltip_rect.preview",
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(112.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(12, 16, 22),
            Some(StrokeStyle::new(color(52, 64, 80), 1.0)),
            4.0,
        )),
    );
    ui.add_child(
        clamped_preview,
        UiNode::scene(
            "overlays.tooltip_rect.scene",
            vec![
                ScenePrimitive::Line {
                    from: UiPoint::new(placed_tooltip.right() + 2.0, placed_tooltip.y + 29.0),
                    to: UiPoint::new(tooltip_target.x - 2.0, tooltip_target.y + 14.0),
                    stroke: StrokeStyle::new(color(92, 106, 128), 1.0),
                },
                ScenePrimitive::Rect(
                    PaintRect::solid(placed_tooltip, color(24, 29, 38))
                        .stroke(AlignedStroke::inside(StrokeStyle::new(
                            color(92, 106, 128),
                            1.0,
                        )))
                        .corner_radii(CornerRadii::uniform(4.0)),
                ),
                ScenePrimitive::Text(
                    PaintText::new(
                        "Tooltip",
                        UiRect::new(
                            placed_tooltip.x + 12.0,
                            placed_tooltip.y + 9.0,
                            placed_tooltip.width - 24.0,
                            18.0,
                        ),
                        text(12.0, color(225, 233, 244)),
                    )
                    .multiline(false),
                ),
                ScenePrimitive::Text(
                    PaintText::new(
                        "Placed inside",
                        UiRect::new(
                            placed_tooltip.x + 12.0,
                            placed_tooltip.y + 31.0,
                            placed_tooltip.width - 24.0,
                            18.0,
                        ),
                        text(10.0, color(156, 170, 190)),
                    )
                    .multiline(false),
                ),
                ScenePrimitive::Rect(
                    PaintRect::solid(tooltip_target, color(48, 112, 184))
                        .stroke(AlignedStroke::inside(StrokeStyle::new(
                            color(132, 190, 255),
                            1.0,
                        )))
                        .corner_radii(CornerRadii::uniform(3.0)),
                ),
                ScenePrimitive::Text(
                    PaintText::new("Target", tooltip_target, text(10.0, color(240, 247, 255)))
                        .horizontal_align(TextHorizontalAlign::Center)
                        .vertical_align(TextVerticalAlign::Center)
                        .multiline(false),
                ),
            ],
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height_percent(1.0),
        ),
    );

    widgets::label(
        ui,
        body,
        "overlays.popup.label",
        "Popup panel",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::label(
        ui,
        body,
        "overlays.popup.status",
        if state.overlay_popup_open {
            "Popup overlay is open."
        } else {
            "Popup overlay is closed."
        },
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let popup_host_layout = if state.overlay_popup_open {
        LayoutStyle::column()
            .with_width_percent(1.0)
            .with_height(128.0)
            .with_flex_shrink(0.0)
    } else {
        LayoutStyle::column()
            .with_width_percent(1.0)
            .with_padding(10.0)
            .with_flex_shrink(0.0)
    };
    let popup_host = ui.add_child(
        body,
        UiNode::container("overlays.popup.host", popup_host_layout).with_visual(UiVisual::panel(
            color(12, 16, 22),
            Some(StrokeStyle::new(color(52, 64, 80), 1.0)),
            4.0,
        )),
    );
    if state.overlay_popup_open {
        let popup = ext_widgets::popup_panel(
            ui,
            popup_host,
            "overlays.popup_panel",
            UiRect::new(10.0, 10.0, 220.0, 104.0),
            ext_widgets::PopupOptions {
                z_index: 4.0,
                portal: UiPortalTarget::Parent,
                accessibility: Some(
                    AccessibilityMeta::new(AccessibilityRole::Dialog).label("Popup preview"),
                ),
                ..Default::default()
            },
        );
        let popup_body = ui.add_child(
            popup,
            UiNode::container(
                "overlays.popup_panel.body",
                LayoutStyle::column()
                    .with_width_percent(1.0)
                    .with_height_percent(1.0)
                    .with_padding(10.0)
                    .with_gap(6.0),
            ),
        );
        let popup_header = row(ui, popup_body, "overlays.popup_panel.header", 8.0);
        widgets::label(
            ui,
            popup_header,
            "overlays.popup_panel.label",
            "Popup panel",
            text(12.0, color(220, 228, 238)),
            LayoutStyle::new().with_width_percent(1.0),
        );
        let mut close = widgets::ButtonOptions::new(LayoutStyle::size(26.0, 22.0))
            .with_action("overlays.popup.close");
        close.visual = UiVisual::panel(color(28, 34, 43), None, 3.0);
        close.hovered_visual = Some(button_visual(54, 70, 92));
        close.text_style = text(12.0, color(220, 228, 238));
        widgets::button(ui, popup_header, "overlays.popup_panel.close", "x", close);
        widgets::label(
            ui,
            popup_body,
            "overlays.popup_panel.body_text",
            "Popup content is conditionally rendered.",
            text(11.0, color(196, 210, 230)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    } else {
        widgets::label(
            ui,
            popup_host,
            "overlays.popup.empty",
            "Open the popup to render an overlay inside this host.",
            text(12.0, color(154, 166, 184)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }

    widgets::label(
        ui,
        body,
        "overlays.toasts.label",
        "Toasts",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let toast_controls = row(ui, body, "overlays.toasts.controls", 10.0);
    button(
        ui,
        toast_controls,
        "overlays.toasts.show",
        "Show toast",
        "toast.show",
        button_visual(48, 112, 184),
    );
    button(
        ui,
        toast_controls,
        "overlays.toasts.hide",
        "Hide",
        "toast.hide",
        button_visual(58, 78, 96),
    );
    widgets::label(
        ui,
        body,
        "overlays.toasts.status",
        if state.toast_visible {
            "Toast overlay is visible."
        } else {
            "Toast overlay is hidden."
        },
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::label(
        ui,
        body,
        "overlays.toasts.action_status",
        format!("Action: {}", state.toast_action_status),
        text(12.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    if state.overlay_modal_open {
        let modal = widgets::modal_dialog(
            ui,
            parent,
            "overlays.modal",
            "Modal dialog",
            widgets::ModalDialogOptions::default()
                .with_size(320.0, 180.0)
                .with_close_action("overlays.modal.close")
                .with_dismissal(ext_widgets::DialogDismissal::MODAL)
                .with_focus_restore(FocusRestoreTarget::Previous),
        );
        widgets::label(
            ui,
            modal.body,
            "overlays.modal.body.text",
            "Modal dialogs are portaled to the application overlay, include a scrim, and trap focus.",
            text(12.0, color(220, 228, 238)),
            LayoutStyle::new().with_width_percent(1.0),
        );
        button(
            ui,
            modal.body,
            "overlays.modal.body.close",
            "Close modal",
            "overlays.modal.close",
            button_visual(48, 112, 184),
        );
    }
}

fn drag_drop_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "drag_drop",
        "Drag and drop",
        UiSize::new(420.0, 0.0),
    );
    widgets::label(
        ui,
        body,
        "drag_drop.sources.label",
        "Drag sources",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let sources = wrapping_row(ui, body, "drag_drop.sources", 8.0);
    widgets::dnd_drag_source(
        ui,
        sources,
        "drag_drop.text_source",
        "Text payload",
        DragPayload::text("Operad payload"),
        widgets::DragSourceOptions::default()
            .with_layout(drag_source_layout())
            .with_kind(DragDropSurfaceKind::ListRow)
            .with_allowed_operations([DragOperation::Copy, DragOperation::Move])
            .with_action("drag_drop.text_source")
            .with_accessibility_hint("Start a text drag operation"),
    );
    widgets::dnd_drag_source(
        ui,
        sources,
        "drag_drop.file_source",
        "File payload",
        DragPayload::files(["/tmp/showcase.scene"]),
        widgets::DragSourceOptions::default()
            .with_layout(drag_source_layout())
            .with_kind(DragDropSurfaceKind::Asset)
            .with_drag_image_policy(widgets::DragImagePolicy::image_key(
                BuiltInIcon::Folder.key(),
                UiSize::new(120.0, 36.0),
                UiPoint::new(10.0, 10.0),
            ))
            .with_allowed_operations([DragOperation::Copy])
            .with_action("drag_drop.file_source"),
    );
    widgets::dnd_drag_source(
        ui,
        sources,
        "drag_drop.bytes_source",
        "Image bytes",
        DragPayload::bytes(DragBytes::new("image/png", vec![137, 80, 78, 71]).name("sprite.png")),
        widgets::DragSourceOptions::default()
            .with_layout(drag_source_layout())
            .with_kind(DragDropSurfaceKind::Asset)
            .with_action("drag_drop.bytes_source")
            .without_drag_image(),
    );

    widgets::label(
        ui,
        body,
        "drag_drop.zones.label",
        "Drop zones",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let zones = wrapping_row(ui, body, "drag_drop.zones", 8.0);
    let accepted_options = widgets::DropZoneOptions::default()
        .with_layout(drop_zone_layout())
        .with_kind(DragDropSurfaceKind::EditorSurface)
        .with_accepted_payload(DropPayloadFilter::empty().text())
        .with_accepted_operations([DragOperation::Copy, DragOperation::Move])
        .with_action("drag_drop.accept_text")
        .with_accessibility_hint("Accepts text payloads");
    let accepted = widgets::dnd_drop_zone(
        ui,
        zones,
        "drag_drop.accept_text",
        "Text accepted",
        accepted_options.clone(),
    );
    widgets::drag_drop::dnd_apply_drop_zone_preview(
        ui,
        accepted.root,
        &accepted_options,
        widgets::drag_drop::DropZonePreviewState::Accepted,
    );

    let rejected_options = widgets::DropZoneOptions::default()
        .with_layout(drop_zone_layout())
        .with_kind(DragDropSurfaceKind::Asset)
        .with_accepted_payload(DropPayloadFilter::empty().files())
        .with_action("drag_drop.files_only");
    let rejected = widgets::dnd_drop_zone(
        ui,
        zones,
        "drag_drop.files_only",
        "Files only",
        rejected_options.clone(),
    );
    widgets::drag_drop::dnd_apply_drop_zone_preview(
        ui,
        rejected.root,
        &rejected_options,
        widgets::drag_drop::DropZonePreviewState::Rejected,
    );
    let image_options = widgets::DropZoneOptions::default()
        .with_layout(drop_zone_layout())
        .with_kind(DragDropSurfaceKind::Asset)
        .with_accepted_payload(DropPayloadFilter::empty().mime_type("image/*"))
        .with_accepted_operations([DragOperation::Copy])
        .with_action("drag_drop.image_bytes");
    let image_zone = widgets::dnd_drop_zone(
        ui,
        zones,
        "drag_drop.image_bytes",
        "Image bytes",
        image_options.clone(),
    );
    widgets::drag_drop::dnd_apply_drop_zone_preview(
        ui,
        image_zone.root,
        &image_options,
        widgets::drag_drop::DropZonePreviewState::Hovered,
    );

    let disabled_options = widgets::DropZoneOptions::default()
        .with_layout(drop_zone_layout())
        .with_kind(DragDropSurfaceKind::EditorSurface)
        .with_accepted_payload(DropPayloadFilter::any())
        .with_action("drag_drop.disabled")
        .disabled();
    let disabled_zone = widgets::dnd_drop_zone(
        ui,
        zones,
        "drag_drop.disabled",
        "Disabled",
        disabled_options.clone(),
    );
    widgets::drag_drop::dnd_apply_drop_zone_preview(
        ui,
        disabled_zone.root,
        &disabled_options,
        widgets::drag_drop::DropZonePreviewState::Disabled,
    );

    let operation_row = wrapping_row(ui, body, "drag_drop.operations", 6.0);
    dnd_operation_chip(ui, operation_row, "drag_drop.operation.copy", "copy");
    dnd_operation_chip(ui, operation_row, "drag_drop.operation.move", "move");
    dnd_operation_chip(ui, operation_row, "drag_drop.operation.link", "link");
    widgets::label(
        ui,
        body,
        "drag_drop.status",
        format!("Status: {}", state.drag_drop_status),
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn media_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "media",
        "Media",
        UiSize::new(MEDIA_ICON_TILE_WIDTH, 0.0),
    );
    widgets::label(
        ui,
        body,
        "media.icons.label",
        "Built-in icons",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let icon_columns = media_icon_columns(state);
    let icons = media_icon_grid(
        ui,
        body,
        "media.icons",
        icon_columns,
        BuiltInIcon::COMMON.len(),
    );
    for icon in BuiltInIcon::COMMON {
        media_icon_tile(ui, icons, icon);
    }

    widgets::label(
        ui,
        body,
        "media.variants.label",
        "Image variants",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let variants = wrapping_row(ui, body, "media.variants", 10.0);
    widgets::image(
        ui,
        variants,
        "media.image.user_png",
        ImageContent::from(ImageHandle::app(SHOWCASE_USER_IMAGE_KEY)),
        widgets::ImageOptions::default()
            .with_layout(media_preview_image_layout())
            .with_accessibility_label("User supplied PNG image"),
    );
    widgets::image(
        ui,
        variants,
        "media.image.untinted",
        icon_image(BuiltInIcon::Play),
        widgets::ImageOptions::default()
            .with_layout(media_preview_image_layout())
            .with_accessibility_label("Untinted play icon"),
    );
    widgets::image(
        ui,
        variants,
        "media.image.warning",
        ImageContent::new(BuiltInIcon::Warning.key()).tinted(color(232, 186, 88)),
        widgets::ImageOptions::default()
            .with_layout(media_preview_image_layout())
            .with_accessibility_label("Tinted warning icon"),
    );
    widgets::image(
        ui,
        variants,
        "media.image.shader",
        ImageContent::new(BuiltInIcon::Grid.key()).tinted(color(118, 183, 255)),
        widgets::ImageOptions::default()
            .with_layout(media_preview_image_layout())
            .with_shader(ShaderEffect::tint(color(169, 119, 255), 0.65))
            .with_accessibility_label("Shader-decorated grid icon"),
    );
}

fn shader_effect_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "shaders",
        "Shader effects",
        UiSize::new(420.0, 280.0),
    );
    widgets::label(
        ui,
        body,
        "shaders.effects.label",
        "Built-in effects",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let phase = (state.progress_phase / std::f32::consts::TAU).fract();
    let previews = wrapping_row(ui, body, "shaders.effects", 10.0);
    shader_effect_preview_card(ui, previews, "base", "Base", None);
    shader_effect_preview_card(
        ui,
        previews,
        "tint",
        "Tint",
        Some(ShaderEffect::tint(color(252, 186, 90), 0.72)),
    );
    shader_effect_preview_card(
        ui,
        previews,
        "shine",
        "Shine",
        Some(ShaderEffect::shine(phase, 0.55).uniform("width", 0.14)),
    );
    shader_effect_preview_card(
        ui,
        previews,
        "glow",
        "Glow",
        Some(ShaderEffect::glow(color(118, 183, 255), 0.9, 7.0)),
    );

    widgets::label(
        ui,
        body,
        "shaders.widgets.label",
        "Applied to widgets",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let panel = ui.add_child(
        body,
        UiNode::container(
            "shaders.widgets",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_padding(10.0)
                .with_gap(10.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(13, 18, 25),
            Some(StrokeStyle::new(color(48, 61, 78), 1.0)),
            4.0,
        )),
    );
    let control_row = wrapping_row(ui, panel, "shaders.widgets.controls", 10.0);
    let mut shine_button = widgets::ButtonOptions::new(
        LayoutStyle::new()
            .with_width(150.0)
            .with_height(34.0)
            .with_flex_shrink(0.0),
    );
    shine_button.leading_image = Some(icon_image(BuiltInIcon::Settings));
    shine_button.image_shader = Some(ShaderEffect::tint(color(111, 203, 159), 0.85));
    shine_button.shader = Some(ShaderEffect::shine(phase, 0.45).uniform("width", 0.12));
    widgets::button(
        ui,
        control_row,
        "shaders.widgets.button",
        "Shine button",
        shine_button,
    );

    widgets::checkbox_with_state(
        ui,
        control_row,
        "shaders.widgets.checkbox",
        "Glow check",
        widgets::CheckboxState::Checked,
        widgets::CheckboxOptions::default()
            .with_check_color(color(118, 183, 255))
            .with_check_shader(ShaderEffect::glow(color(118, 183, 255), 1.0, 4.0)),
    );

    let progress_value = smooth_loop(state.progress_phase * 0.8, 0.2) * 100.0;
    let mut progress = ext_widgets::ProgressIndicatorOptions::default();
    progress.layout = LayoutStyle::new().with_width_percent(1.0).with_height(12.0);
    progress.fill_visual = UiVisual::panel(color(111, 203, 159), None, 4.0);
    progress.fill_shader = Some(ShaderEffect::shine(phase, 0.5).uniform("width", 0.16));
    progress.accessibility_label = Some("Shadered progress fill".to_string());
    ext_widgets::progress_indicator(
        ui,
        panel,
        "shaders.widgets.progress",
        ext_widgets::ProgressIndicatorValue::percent(progress_value),
        progress,
    );

    let mut slider_options = widgets::SliderOptions::default();
    slider_options.layout = LayoutStyle::new()
        .with_width_percent(1.0)
        .with_height(28.0)
        .with_flex_shrink(0.0);
    slider_options.fill_shader = Some(ShaderEffect::tint(color(169, 119, 255), 0.55));
    slider_options.thumb_shader = Some(ShaderEffect::glow(color(252, 186, 90), 0.9, 4.0));
    slider_options.accessibility_label = Some("Shadered slider".to_string());
    widgets::slider(
        ui,
        panel,
        "shaders.widgets.slider",
        smooth_loop(state.progress_phase * 0.6, 0.4),
        0.0..1.0,
        slider_options,
    );
}

fn shader_effect_preview_card(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    shader: Option<ShaderEffect>,
) {
    let tile = ui.add_child(
        parent,
        UiNode::container(
            format!("shaders.effect_tile.{name}"),
            LayoutStyle::column()
                .with_width(96.0)
                .with_height(104.0)
                .with_padding(8.0)
                .with_gap(8.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(17, 22, 30),
            Some(StrokeStyle::new(color(50, 62, 78), 1.0)),
            4.0,
        ))
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).label(label)),
    );
    let mut swatch = UiNode::container(
        format!("shaders.effect.{name}.swatch"),
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height(50.0)
            .with_flex_shrink(0.0),
    )
    .with_visual(UiVisual::panel(
        color(64, 109, 194),
        Some(StrokeStyle::new(color(138, 164, 194), 1.0)),
        8.0,
    ));
    if let Some(shader) = shader {
        swatch = swatch.with_shader(shader);
    }
    ui.add_child(tile, swatch);
    widgets::label(
        ui,
        tile,
        format!("shaders.effect.{name}.label"),
        label,
        text(11.0, color(204, 216, 232)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn shader_lab_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section_with_min_viewport(
        ui,
        parent,
        "shader_lab",
        "Shader lab",
        UiSize::new(SHADER_LAB_CONTENT_MIN_WIDTH, SHADER_LAB_CONTENT_MIN_HEIGHT),
    );
    let source_error = state.shader_lab_source_error.as_deref();
    let source_valid = source_error.is_none();

    let mut split_options = ext_widgets::SplitPaneOptions::default()
        .with_handle_action("shader_lab.workspace.resize")
        .with_handle_hover_visual(UiVisual::panel(color(96, 166, 238), None, 2.0));
    split_options.layout = Layout::row()
        .size(LayoutSize::new(
            LayoutDimension::percent(1.0),
            LayoutDimension::points(SHADER_LAB_WORKSPACE_HEIGHT),
        ))
        .min_size(LayoutSize::points(
            SHADER_LAB_CONTENT_MIN_WIDTH,
            SHADER_LAB_WORKSPACE_HEIGHT,
        ))
        .flex(0.0, 0.0, LayoutDimension::Auto)
        .to_layout_style();
    split_options.handle_thickness = SHADER_LAB_SPLIT_HANDLE_THICKNESS;
    split_options.handle_visual = UiVisual::panel(color(48, 61, 78), None, 2.0);

    ext_widgets::split_pane(
        ui,
        body,
        "shader_lab.workspace",
        ext_widgets::SplitAxis::Horizontal,
        state.shader_lab_split,
        split_options,
        |ui, preview_pane| {
            shader_lab_preview_column(ui, preview_pane, state, source_error, source_valid);
        },
        |ui, editor_pane| {
            shader_lab_editor_column(ui, editor_pane, state, source_error);
        },
    );
}

fn shader_lab_preview_column(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    source_error: Option<&str>,
    source_valid: bool,
) {
    let preview_column = ui.add_child(
        parent,
        UiNode::container(
            "shader_lab.preview.column",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0)
                .with_gap(8.0)
                .with_flex_shrink(1.0),
        ),
    );
    let target_row = row(ui, preview_column, "shader_lab.target.row", 8.0);
    widgets::label(
        ui,
        target_row,
        "shader_lab.target.caption",
        "Preview",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new()
            .with_width(58.0)
            .with_height(30.0)
            .with_flex_shrink(0.0),
    );
    shader_lab_dropdown_select(
        ui,
        target_row,
        "shader_lab.target",
        &shader_lab_target_options(),
        &state.shader_lab_target_menu,
        160.0,
        "Preview target",
        "Shader lab preview target",
    );
    shader_lab_preview_controls(ui, preview_column, state);

    let preview = ui.add_child(
        preview_column,
        UiNode::container(
            "shader_lab.preview.surface",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(0.0)
                .with_flex_grow(1.0)
                .with_padding(18.0)
                .with_gap(8.0)
                .with_align_items(taffy::prelude::AlignItems::Center)
                .with_justify_content(taffy::prelude::JustifyContent::Center)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(8, 12, 18),
            Some(StrokeStyle::new(color(48, 61, 78), 1.0)),
            4.0,
        )),
    );
    shader_lab_preview(ui, preview, state, source_valid);

    if let Some(status) = shader_lab_status_label(state, source_error) {
        widgets::label(
            ui,
            preview_column,
            "shader_lab.preview.status",
            status,
            text(11.0, color(166, 176, 190)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }
    shader_lab_material_contract_demo(ui, preview_column, state);
}

fn shader_lab_preview_controls(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let controls = ui.add_child(
        parent,
        UiNode::container(
            "shader_lab.preview.controls",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_gap(6.0)
                .with_flex_shrink(0.0),
        ),
    );
    let text_row = wrapping_row(ui, controls, "shader_lab.preview.text_controls", 8.0);
    shader_lab_option_checkbox(
        ui,
        text_row,
        "shader_lab.frame_text.toggle",
        "Frame",
        state.shader_lab_show_frame_text,
    );
    shader_lab_option_checkbox(
        ui,
        text_row,
        "shader_lab.button_text.toggle",
        "Button",
        state.shader_lab_show_button_text,
    );

    let style_row = wrapping_row(ui, controls, "shader_lab.preview.style_controls", 8.0);
    shader_lab_slider_control(
        ui,
        style_row,
        "shader_lab.surface.stroke",
        "Border",
        state.shader_lab_surface_stroke_width,
        SHADER_LAB_SURFACE_STROKE_MAX,
        1,
    );
    shader_lab_slider_control(
        ui,
        style_row,
        "shader_lab.surface.radius",
        "Radius",
        state.shader_lab_surface_radius,
        SHADER_LAB_SURFACE_RADIUS_MAX,
        0,
    );
}

fn shader_lab_material_contract_demo(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let panel = ui.add_child(
        parent,
        UiNode::container(
            "shader_lab.material",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_gap(6.0)
                .with_flex_shrink(0.0),
        ),
    );
    widgets::label(
        ui,
        panel,
        "shader_lab.material.title",
        "Material",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let controls = wrapping_row(ui, panel, "shader_lab.material.controls", 8.0);
    shader_lab_labeled_dropdown(
        ui,
        controls,
        "shader_lab.material.shader",
        "Shader",
        &shader_lab_material_shader_options(),
        &state.shader_lab_material_shader_menu,
        132.0,
    );
    shader_lab_labeled_dropdown(
        ui,
        controls,
        "shader_lab.material.shape",
        "Shape",
        &shader_lab_material_shape_options(),
        &state.shader_lab_material_shape_menu,
        132.0,
    );
    shader_lab_labeled_dropdown(
        ui,
        controls,
        "shader_lab.material.geometry",
        "Geometry",
        &shader_lab_material_geometry_options(),
        &state.shader_lab_material_geometry_menu,
        140.0,
    );
    shader_lab_slider_control(
        ui,
        controls,
        "shader_lab.material.outset",
        "Outset",
        state.shader_lab_material_outset,
        SHADER_LAB_MATERIAL_OUTSET_MAX,
        0,
    );

    let contracts_layout = Layout::column()
        .size(LayoutSize::new(
            LayoutDimension::percent(1.0),
            LayoutDimension::Auto,
        ))
        .padding(LayoutSpacing::new(
            LayoutLength::points(4.0),
            LayoutLength::points(4.0),
            LayoutLength::points(SHADER_LAB_MATERIAL_OUTSET_MAX),
            LayoutLength::points(8.0),
        ))
        .flex(0.0, 0.0, LayoutDimension::Auto)
        .to_layout_style();
    let contracts_shell = ui.add_child(
        panel,
        UiNode::container("shader_lab.material.contracts.shell", contracts_layout),
    );
    let row = wrapping_row(ui, contracts_shell, "shader_lab.material.contracts", 8.0);
    shader_lab_material_chip(
        ui,
        row,
        "shader_lab.material.current",
        "Selected",
        shader_lab_selected_material(state),
        shader_lab_material_visual(state.shader_lab_material_shape),
    );
    shader_lab_material_chip(
        ui,
        row,
        "shader_lab.material.outset",
        "Glow",
        ElementMaterial::shader(ShaderEffect::glow(
            color(100, 180, 255),
            0.95,
            SHADER_LAB_MATERIAL_OUTSET,
        ))
        .with_paint_outset(LayoutInsets::points(
            state
                .shader_lab_material_outset
                .clamp(0.0, SHADER_LAB_MATERIAL_OUTSET_MAX),
        )),
        UiVisual::panel(color(32, 64, 96), None, 8.0),
    );
    shader_lab_material_chip(
        ui,
        row,
        "shader_lab.material.circle_hit",
        "Circle",
        ElementMaterial::new()
            .with_clip_shape(ElementShape::circle())
            .with_hit_shape(ElementShape::circle()),
        UiVisual::panel(
            color(74, 133, 198),
            Some(StrokeStyle::new(color(212, 232, 255), 1.0)),
            999.0,
        ),
    );
    shader_lab_material_chip(
        ui,
        row,
        "shader_lab.material.geometry_chip",
        "Warp",
        ElementMaterial::new()
            .with_paint_outset(LayoutInsets::points(8.0))
            .with_geometry_effect(GeometryEffect::wave(8.0)),
        UiVisual::panel(
            color(101, 70, 170),
            Some(StrokeStyle::new(color(214, 196, 255), 1.0)),
            6.0,
        ),
    );
}

fn shader_lab_labeled_dropdown(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    options: &[ext_widgets::SelectOption],
    state: &ext_widgets::SelectMenuState,
    width: f32,
) {
    let control = ui.add_child(
        parent,
        UiNode::container(
            format!("{name}.control"),
            LayoutStyle::row()
                .with_width(width + 74.0)
                .with_height(30.0)
                .with_gap(6.0)
                .with_align_items(taffy::prelude::AlignItems::Center)
                .with_flex_shrink(0.0),
        ),
    );
    widgets::label(
        ui,
        control,
        format!("{name}.caption"),
        label,
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new()
            .with_width(66.0)
            .with_height(30.0)
            .with_flex_shrink(0.0),
    );
    shader_lab_dropdown_select(ui, control, name, options, state, width, label, label);
}

fn shader_lab_dropdown_select(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    options: &[ext_widgets::SelectOption],
    state: &ext_widgets::SelectMenuState,
    width: f32,
    placeholder: &'static str,
    accessibility_label: &'static str,
) {
    let anchor = ui.add_child(
        parent,
        UiNode::container(
            format!("{name}.anchor"),
            LayoutStyle::new()
                .with_width(width)
                .with_height(30.0)
                .with_flex_shrink(0.0),
        ),
    );
    ext_widgets::dropdown_select(
        ui,
        anchor,
        name,
        options,
        state,
        Some(select_popup(
            UiRect::new(0.0, 0.0, width, 30.0),
            UiRect::new(0.0, 0.0, width + 48.0, 240.0),
        )),
        dropdown_select_options(width, name, placeholder, accessibility_label),
    );
}

fn shader_lab_selected_material(state: &ShowcaseState) -> ElementMaterial {
    let shape = state.shader_lab_material_shape.shape();
    let mut material = ElementMaterial::new()
        .with_paint_outset(LayoutInsets::points(
            state
                .shader_lab_material_outset
                .clamp(0.0, SHADER_LAB_MATERIAL_OUTSET_MAX),
        ))
        .with_clip_shape(shape.clone())
        .with_hit_shape(shape)
        .with_geometry_effect(state.shader_lab_material_geometry.effect());
    if let Some(shader) = shader_lab_material_shader_effect(state) {
        material = material.with_shader(shader);
    }
    material
}

fn shader_lab_material_shader_effect(state: &ShowcaseState) -> Option<ShaderEffect> {
    let phase = state.progress_phase.rem_euclid(1.0);
    match state.shader_lab_material_shader {
        ShaderLabMaterialShader::None => None,
        ShaderLabMaterialShader::Tint => Some(ShaderEffect::tint(color(255, 196, 92), 0.62)),
        ShaderLabMaterialShader::Shine => Some(ShaderEffect::shine(phase, 0.92)),
        ShaderLabMaterialShader::Glow => Some(ShaderEffect::glow(
            color(100, 180, 255),
            0.95,
            state
                .shader_lab_material_outset
                .clamp(0.0, SHADER_LAB_MATERIAL_OUTSET_MAX),
        )),
        ShaderLabMaterialShader::Plasma => {
            Some(ShaderEffect::plasma(phase, color(82, 190, 255), 0.75, 12.0))
        }
        ShaderLabMaterialShader::Rings => {
            Some(ShaderEffect::rings(phase, color(232, 170, 88), 0.78, 11.0))
        }
        ShaderLabMaterialShader::Grid => {
            Some(ShaderEffect::grid(phase, color(156, 132, 255), 0.85, 9.0))
        }
    }
}

fn shader_lab_material_visual(shape: ShaderLabMaterialShape) -> UiVisual {
    UiVisual::panel(
        color(39, 71, 114),
        Some(StrokeStyle::new(color(168, 205, 255), 1.0)),
        shape.visual_radius(),
    )
}

fn shader_lab_material_chip(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    material: ElementMaterial,
    visual: UiVisual,
) {
    let chip = ui.add_child(
        parent,
        UiNode::container(
            name,
            LayoutStyle::row()
                .with_width(156.0)
                .with_height(44.0)
                .with_padding(8.0)
                .with_align_items(taffy::prelude::AlignItems::Center)
                .with_justify_content(taffy::prelude::JustifyContent::Center)
                .with_flex_shrink(0.0),
        )
        .with_visual(visual)
        .with_material(material)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Image).label(format!("{label} material")),
        ),
    );
    widgets::label(
        ui,
        chip,
        format!("{name}.text"),
        label,
        text(11.0, color(246, 249, 252)),
        LayoutStyle::new().with_flex_shrink(0.0),
    );
}

fn shader_lab_option_checkbox(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    checked: bool,
) {
    let mut options = widgets::CheckboxOptions::default()
        .with_action(name)
        .with_text_style(text(12.0, color(220, 228, 238)));
    options.layout = LayoutStyle::new()
        .with_width(112.0)
        .with_height(24.0)
        .with_flex_shrink(0.0);
    widgets::checkbox(ui, parent, name, label, checked, options);
}

fn shader_lab_slider_control(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    value: f32,
    max: f32,
    decimals: usize,
) {
    let control = ui.add_child(
        parent,
        UiNode::container(
            format!("{name}.control"),
            Layout::row()
                .size(LayoutSize::new(
                    LayoutDimension::points(214.0),
                    LayoutDimension::Auto,
                ))
                .align_items(LayoutAlignment::Center)
                .gap(LayoutGap::points(6.0, 6.0))
                .flex(0.0, 0.0, LayoutDimension::Auto)
                .to_layout_style(),
        ),
    );
    widgets::label(
        ui,
        control,
        format!("{name}.label"),
        label,
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new()
            .with_width(46.0)
            .with_height(22.0)
            .with_flex_shrink(0.0),
    );
    let mut options = widgets::SliderOptions::default()
        .with_layout(
            LayoutStyle::new()
                .with_width(96.0)
                .with_height(22.0)
                .with_flex_shrink(0.0),
        )
        .with_value_edit_action(name);
    options.accessibility_label = Some(format!("Shader lab {label}"));
    widgets::slider(
        ui,
        control,
        format!("{name}.slider"),
        (value / max.max(f32::EPSILON)).clamp(0.0, 1.0),
        0.0..1.0,
        options,
    );
    widgets::label(
        ui,
        control,
        format!("{name}.value"),
        format!("{value:.decimals$}px"),
        text(12.0, color(226, 232, 242)),
        LayoutStyle::new()
            .with_width(48.0)
            .with_height(22.0)
            .with_flex_shrink(0.0),
    );
}

fn shader_lab_surface_stroke(state: &ShowcaseState) -> Option<StrokeStyle> {
    (state.shader_lab_surface_stroke_width > f32::EPSILON).then(|| {
        StrokeStyle::new(
            color(150, 180, 235),
            state
                .shader_lab_surface_stroke_width
                .clamp(0.0, SHADER_LAB_SURFACE_STROKE_MAX),
        )
    })
}

fn shader_lab_surface_radius(state: &ShowcaseState) -> f32 {
    state
        .shader_lab_surface_radius
        .clamp(0.0, SHADER_LAB_SURFACE_RADIUS_MAX)
}

fn shader_lab_editor_column(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    source_error: Option<&str>,
) {
    let editor_column = ui.add_child(
        parent,
        UiNode::container(
            "shader_lab.editor.column",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0)
                .with_gap(8.0)
                .with_flex_shrink(1.0),
        ),
    );
    let preset_row = row(ui, editor_column, "shader_lab.preset.row", 8.0);
    widgets::label(
        ui,
        preset_row,
        "shader_lab.preset.caption",
        "Program",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new()
            .with_width(62.0)
            .with_height(30.0)
            .with_flex_shrink(0.0),
    );
    shader_lab_dropdown_select(
        ui,
        preset_row,
        "shader_lab.preset",
        &shader_lab_preset_options(),
        &state.shader_lab_preset_menu,
        180.0,
        "WGSL preset",
        "Shader lab WGSL preset",
    );

    let editor_frame = ui.add_child(
        editor_column,
        UiNode::container(
            "shader_lab.editor.frame",
            Layout::column()
                .size(LayoutSize::new(
                    LayoutDimension::percent(1.0),
                    LayoutDimension::points(0.0),
                ))
                .min_size(LayoutSize::points(0.0, SHADER_LAB_EDITOR_HEIGHT))
                .flex(1.0, 1.0, LayoutDimension::points(0.0))
                .to_layout_style(),
        )
        .with_visual(UiVisual::panel(
            color(18, 22, 28),
            Some(StrokeStyle::new(color(72, 84, 104), 1.0)),
            4.0,
        )),
    );
    let editor_scroll = widgets::scroll_area_with_options(
        ui,
        editor_frame,
        "shader_lab.editor.scroll",
        widgets::ScrollAreaOptions::new(
            ScrollAxes::BOTH,
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0),
        )
        .with_scroll(
            operad::ScrollState::new(ScrollAxes::BOTH).with_offset(state.shader_lab_editor_scroll),
        )
        .with_action("shader_lab.editor.scroll"),
    );

    let mut code_options = state.text_edit_options(FocusedTextInput::ShaderLabSource);
    code_options.edit_action = Some("shader_lab.editor.edit".into());
    code_options.visual = UiVisual::TRANSPARENT;
    code_options.focused_visual = Some(UiVisual::TRANSPARENT);
    code_options.disabled_visual = Some(UiVisual::TRANSPARENT);
    widgets::code_editor(
        ui,
        editor_scroll,
        "shader_lab.editor",
        &state.shader_lab_source,
        code_options,
    );
    let (validation_text, validation_color) = shader_lab_validation_label(source_error);
    widgets::label(
        ui,
        editor_column,
        "shader_lab.validation",
        validation_text,
        text(11.0, validation_color),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn shader_lab_editor_content_size(source: &str) -> UiSize {
    let style = widgets::code_text_style();
    let line_count = source.lines().count().max(1) as f32;
    let longest_line = source
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .max(48) as f32;
    UiSize::new(
        (longest_line * style.font_size * 0.56 + 24.0).max(SHADER_LAB_EDITOR_WIDTH),
        (line_count * style.line_height + 18.0).max(SHADER_LAB_EDITOR_HEIGHT),
    )
}

fn shader_lab_preview(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    source_valid: bool,
) {
    match state.shader_lab_target {
        ShaderLabTarget::Canvas => {
            let mut options = widgets::CanvasOptions::default()
                .with_layout(
                    LayoutStyle::new()
                        .with_width_percent(1.0)
                        .with_height_percent(1.0)
                        .with_flex_grow(1.0),
                )
                .with_intrinsic_size(UiSize::new(
                    SHADER_LAB_PREVIEW_WIDTH - 20.0,
                    SHADER_LAB_PREVIEW_HEIGHT,
                ))
                .with_accessibility_label("Shader lab canvas preview");
            options.visual = UiVisual::panel(color(8, 12, 18), None, 4.0);
            widgets::canvas(
                ui,
                parent,
                "shader_lab.preview.canvas",
                CanvasContent::new("shader_lab.preview.canvas")
                    .program(shader_lab_canvas_program(state, source_valid)),
                options,
            );
        }
        ShaderLabTarget::Frame => {
            let mut frame_node = UiNode::container(
                "shader_lab.preview.frame",
                operad::layout::with_min_size(
                    LayoutStyle::column()
                        .with_width_percent(0.82)
                        .with_height_percent(0.62)
                        .with_padding(14.0)
                        .with_align_items(taffy::prelude::AlignItems::Center)
                        .with_justify_content(taffy::prelude::JustifyContent::Center)
                        .with_flex_shrink(0.0),
                    operad::layout::px(SHADER_LAB_FRAME_MIN_WIDTH),
                    operad::layout::px(SHADER_LAB_FRAME_MIN_HEIGHT),
                ),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(0, 0, 0, 0),
                shader_lab_surface_stroke(state),
                shader_lab_surface_radius(state),
            ));
            frame_node.style_mut().set_clip(ClipBehavior::Clip);
            let frame = ui.add_child(parent, frame_node);
            shader_lab_canvas_layer_fill(
                ui,
                frame,
                "shader_lab.preview.frame.shader",
                state,
                source_valid,
            );
            if state.shader_lab_show_frame_text {
                let label = widgets::label(
                    ui,
                    frame,
                    "shader_lab.preview.frame.label",
                    "WGSL frame",
                    text(14.0, color(246, 249, 252)),
                    LayoutStyle::new().with_flex_shrink(0.0),
                );
                ui.node_mut(label).style_mut().set_z_index(1.0);
            }
        }
        ShaderLabTarget::Button => {
            let mut shell_node = UiNode::container(
                "shader_lab.preview.button.shell",
                LayoutStyle::column()
                    .with_width(SHADER_LAB_BUTTON_WIDTH)
                    .with_height(SHADER_LAB_BUTTON_HEIGHT)
                    .with_align_items(taffy::prelude::AlignItems::Center)
                    .with_justify_content(taffy::prelude::JustifyContent::Center),
            )
            .with_visual(UiVisual::TRANSPARENT);
            shell_node.style_mut().set_clip(ClipBehavior::Clip);
            let shell = ui.add_child(parent, shell_node);
            shader_lab_canvas_layer_fill(
                ui,
                shell,
                "shader_lab.preview.button.shader",
                state,
                source_valid,
            );
            let mut options = widgets::ButtonOptions::new(
                LayoutStyle::new()
                    .with_width(SHADER_LAB_BUTTON_WIDTH)
                    .with_height(SHADER_LAB_BUTTON_HEIGHT),
            )
            .with_action("shader_lab.preview.button")
            .with_accessibility_label("Shader button preview");
            options.visual = UiVisual::panel(
                ColorRgba::new(0, 0, 0, 0),
                shader_lab_surface_stroke(state),
                shader_lab_surface_radius(state),
            );
            options.hovered_visual = Some(UiVisual::panel(
                ColorRgba::new(255, 255, 255, 28),
                shader_lab_surface_stroke(state),
                shader_lab_surface_radius(state),
            ));
            options.pressed_visual = Some(UiVisual::panel(
                ColorRgba::new(0, 0, 0, 48),
                shader_lab_surface_stroke(state),
                shader_lab_surface_radius(state),
            ));
            options.text_style = text(14.0, color(246, 249, 252));
            let button = widgets::button(
                ui,
                shell,
                "shader_lab.preview.button",
                if state.shader_lab_show_button_text {
                    "Shader button"
                } else {
                    ""
                },
                options,
            );
            ui.node_mut(button).style_mut().set_z_index(1.0);
        }
    }
}

fn shader_lab_canvas_layer_fill(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    state: &ShowcaseState,
    source_valid: bool,
) -> UiNodeId {
    let layout = operad::layout::with_absolute_position(
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height_percent(1.0),
        0.0,
        0.0,
    );
    let mut options = widgets::CanvasOptions::default()
        .with_layout(layout)
        .with_intrinsic_size(UiSize::new(
            SHADER_LAB_PREVIEW_WIDTH,
            SHADER_LAB_PREVIEW_HEIGHT,
        ))
        .with_accessibility_label(format!("{name} shader preview"));
    options.input = InputBehavior::NONE;
    options.visual = UiVisual::TRANSPARENT;
    let canvas = widgets::canvas(
        ui,
        parent,
        name,
        CanvasContent::new(name).program(shader_lab_canvas_program(state, source_valid)),
        options,
    );
    ui.node_mut(canvas).style_mut().set_z_index(0.0);
    canvas
}

fn shader_lab_status_label(state: &ShowcaseState, source_error: Option<&str>) -> Option<String> {
    if source_error.is_some() {
        Some(format!(
            "{} fallback until WGSL validates",
            state.shader_lab_target.label()
        ))
    } else {
        None
    }
}

fn shader_lab_validation_label(source_error: Option<&str>) -> (String, ColorRgba) {
    if let Some(error) = source_error {
        (
            format!("WGSL error: {}", compact_shader_error(error, 160)),
            color(255, 139, 128),
        )
    } else {
        ("WGSL valid".to_string(), color(112, 221, 160))
    }
}

fn compact_shader_error(error: &str, max_chars: usize) -> String {
    let mut compact = error.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() > max_chars {
        compact = compact.chars().take(max_chars.saturating_sub(3)).collect();
        compact.push_str("...");
    }
    compact
}

fn shader_lab_canvas_program(state: &ShowcaseState, source_valid: bool) -> CanvasRenderProgram {
    let source = if source_valid {
        state.shader_lab_source.text().to_string()
    } else {
        SHADER_LAB_ERROR_WGSL.to_string()
    };
    CanvasRenderProgram::wgsl(source)
        .label("showcase.shader_lab.canvas")
        .constant("TIME", state.progress_phase as f64)
        .clear_color(Some(color(8, 12, 18)))
}

fn shader_lab_source_error(source: &str) -> Option<String> {
    if !shader_lab_source_has_entry_points(source) {
        return Some("source must define @vertex fn vs_main and @fragment fn fs_main".to_string());
    }

    shader_lab_compile_error(source)
}

#[cfg(feature = "wgpu")]
fn shader_lab_compile_error(source: &str) -> Option<String> {
    let module = match naga::front::wgsl::parse_str(source) {
        Ok(module) => module,
        Err(error) => return Some(error.emit_to_string(source)),
    };
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator
        .validate(&module)
        .err()
        .map(|error| error.to_string())
}

#[cfg(not(feature = "wgpu"))]
fn shader_lab_compile_error(_source: &str) -> Option<String> {
    None
}

fn shader_lab_source_has_entry_points(source: &str) -> bool {
    source.contains("@vertex")
        && source.contains("fn vs_main")
        && source.contains("@fragment")
        && source.contains("fn fs_main")
}

fn timeline_ruler(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body =
        section_with_min_viewport(ui, parent, "timeline", "Timeline", UiSize::new(560.0, 0.0));
    widgets::label(
        ui,
        body,
        "timeline.label",
        "Clip timeline",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::label(
        ui,
        body,
        "timeline.description",
        "The ruler maps time to tracks, clips, markers, and the current playhead.",
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let editor = row(ui, body, "timeline.editor", 0.0);
    let labels = ui.add_child(
        editor,
        UiNode::container(
            "timeline.lane_labels",
            LayoutStyle::column()
                .with_width(96.0)
                .with_height(172.0)
                .with_flex_shrink(0.0),
        ),
    );
    for (name, label, height) in [
        ("timeline.lane_labels.header", "Tracks", 40.0),
        ("timeline.lane_labels.video", "Video", 44.0),
        ("timeline.lane_labels.audio", "Audio", 44.0),
        ("timeline.lane_labels.notes", "Notes", 44.0),
    ] {
        widgets::label(
            ui,
            labels,
            name,
            label,
            text(11.0, color(166, 176, 190)),
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(height)
                .with_flex_shrink(0.0),
        );
    }

    let timeline_scroll = timeline_scroll_state_for_view(
        state.timeline_scroll,
        state.timeline_scroll.viewport_size().width,
    );
    let range = ext_widgets::TimelineRange::new(0.0, 48.0);
    let nodes = scroll_area_widgets::scroll_container_shell(
        ui,
        editor,
        "timeline",
        timeline_scroll,
        widgets::ScrollContainerOptions::default()
            .with_axes(ScrollAxes::HORIZONTAL)
            .with_action_prefix("timeline")
            .with_gap(4.0)
            .with_scrollbar_thickness(TIMELINE_SCROLLBAR_HEIGHT)
            .with_layout(
                LayoutStyle::column()
                    .with_width(0.0)
                    .with_flex_grow(1.0)
                    .with_height(TIMELINE_SCROLL_CONTAINER_HEIGHT)
                    .with_flex_shrink(0.0),
            )
            .with_viewport_layout(
                LayoutStyle::column()
                    .with_width(0.0)
                    .with_flex_grow(1.0)
                    .with_height(TIMELINE_VIEWPORT_HEIGHT)
                    .with_flex_shrink(1.0),
            )
            .with_horizontal_scrollbar(
                scrollbar_widgets::ScrollbarOptions::default()
                    .with_action("timeline.horizontal-scrollbar"),
            )
            .with_accessibility_label("Timeline horizontal scroller"),
    );
    let content = ui.add_child(
        nodes.viewport,
        UiNode::container(
            "timeline.content",
            LayoutStyle::column()
                .with_width(TIMELINE_CONTENT_WIDTH)
                .with_height(TIMELINE_VIEWPORT_HEIGHT)
                .with_flex_shrink(0.0),
        ),
    );
    let mut ruler_options = ext_widgets::TimelineRulerOptions::default();
    ruler_options.height = 40.0;
    ruler_options.layout = LayoutStyle::new()
        .with_width(TIMELINE_CONTENT_WIDTH)
        .with_height(40.0)
        .with_flex_shrink(0.0);
    ruler_options.accessibility_label = Some("Editing timeline ruler".to_string());
    ruler_options.accessibility_hint =
        Some("Shows seconds for the visible timeline clips".to_string());
    ext_widgets::timeline_ruler(
        ui,
        content,
        "timeline.ruler",
        ext_widgets::RulerSpec {
            range,
            width: TIMELINE_CONTENT_WIDTH,
            major_step: 4.0,
            minor_step: 1.0,
            label_every: 1,
        },
        ruler_options,
    );
    ui.add_child(
        content,
        UiNode::scene(
            "timeline.tracks",
            timeline_track_primitives(range, TIMELINE_CONTENT_WIDTH),
            LayoutStyle::new()
                .with_width(TIMELINE_CONTENT_WIDTH)
                .with_height(132.0)
                .with_flex_shrink(0.0),
        ),
    );
}

fn timeline_track_primitives(range: ext_widgets::TimelineRange, width: f32) -> Vec<ScenePrimitive> {
    let mut primitives = Vec::new();
    let lane_height = 36.0;
    let lane_gap = 8.0;
    let lanes = [
        ("Video", 0.0, color(16, 22, 30)),
        ("Audio", lane_height + lane_gap, color(13, 20, 27)),
        ("Notes", (lane_height + lane_gap) * 2.0, color(16, 22, 30)),
    ];

    for (label, y, fill) in lanes {
        primitives.push(ScenePrimitive::Rect(
            PaintRect::solid(UiRect::new(0.0, y, width, lane_height), fill)
                .stroke(AlignedStroke::inside(StrokeStyle::new(
                    color(38, 49, 64),
                    1.0,
                )))
                .corner_radii(CornerRadii::uniform(2.0)),
        ));
        primitives.push(ScenePrimitive::Text(
            PaintText::new(
                label,
                UiRect::new(8.0, y + 8.0, 72.0, 18.0),
                text(10.0, color(116, 128, 145)),
            )
            .multiline(false),
        ));
    }

    for second in (0..=48).step_by(4) {
        let x = range.value_to_x(second as f64, width);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(x, 0.0),
            to: UiPoint::new(x, 124.0),
            stroke: StrokeStyle::new(color(34, 44, 58), 1.0),
        });
    }

    push_timeline_clip(
        &mut primitives,
        range,
        width,
        "Intro",
        2.0,
        10.0,
        0.0,
        color(57, 126, 207),
    );
    push_timeline_clip(
        &mut primitives,
        range,
        width,
        "Cutaway",
        12.0,
        24.0,
        0.0,
        color(95, 107, 212),
    );
    push_timeline_clip(
        &mut primitives,
        range,
        width,
        "Final",
        28.0,
        44.0,
        0.0,
        color(68, 153, 122),
    );
    push_timeline_clip(
        &mut primitives,
        range,
        width,
        "Music bed",
        0.0,
        48.0,
        lane_height + lane_gap,
        color(205, 160, 71),
    );
    push_timeline_clip(
        &mut primitives,
        range,
        width,
        "Voiceover",
        8.0,
        18.0,
        lane_height + lane_gap,
        color(183, 107, 185),
    );

    for (second, label) in [(6.0, "Beat"), (21.0, "Cut"), (37.0, "Cue")] {
        let x = range.value_to_x(second, width);
        let y = (lane_height + lane_gap) * 2.0 + 8.0;
        primitives.push(ScenePrimitive::Polygon {
            points: vec![
                UiPoint::new(x, y),
                UiPoint::new(x + 8.0, y + 8.0),
                UiPoint::new(x, y + 16.0),
                UiPoint::new(x - 8.0, y + 8.0),
            ],
            fill: color(245, 198, 83),
            stroke: Some(StrokeStyle::new(color(255, 234, 178), 1.0)),
        });
        primitives.push(ScenePrimitive::Text(
            PaintText::new(
                label,
                UiRect::new(x + 12.0, y - 1.0, 72.0, 18.0),
                text(10.0, color(225, 233, 244)),
            )
            .multiline(false),
        ));
    }

    let playhead_x = range.value_to_x(18.5, width);
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(playhead_x, 0.0),
        to: UiPoint::new(playhead_x, 124.0),
        stroke: StrokeStyle::new(ColorRgba::new(255, 120, 96, 255), 2.0),
    });
    primitives.push(ScenePrimitive::Text(
        PaintText::new(
            "Playhead 18.5s",
            UiRect::new(playhead_x + 8.0, 106.0, 120.0, 18.0),
            text(10.0, ColorRgba::new(255, 172, 154, 255)),
        )
        .multiline(false),
    ));

    primitives
}

#[allow(clippy::too_many_arguments)]
fn push_timeline_clip(
    primitives: &mut Vec<ScenePrimitive>,
    range: ext_widgets::TimelineRange,
    width: f32,
    label: &'static str,
    start: f64,
    end: f64,
    lane_y: f32,
    fill: ColorRgba,
) {
    let x = range.value_to_x(start, width);
    let right = range.value_to_x(end, width);
    let rect = UiRect::new(x, lane_y + 6.0, (right - x).max(1.0), 24.0);
    primitives.push(ScenePrimitive::Rect(
        PaintRect::solid(rect, fill)
            .stroke(AlignedStroke::inside(StrokeStyle::new(
                ColorRgba::new(230, 240, 255, 96),
                1.0,
            )))
            .corner_radii(CornerRadii::uniform(4.0)),
    ));
    primitives.push(ScenePrimitive::Text(
        PaintText::new(label, rect, text(10.0, color(245, 248, 252)))
            .horizontal_align(TextHorizontalAlign::Center)
            .vertical_align(TextVerticalAlign::Center)
            .multiline(false),
    ));
}

fn theme_demo_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState, theme: &Theme) {
    let body = section(ui, parent, "theme", "Theme");
    widgets::label(
        ui,
        body,
        "theme.current",
        format!("Current theme: {}", theme.name),
        themed_text(theme, 14.0),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let choices = wrapping_row(ui, body, "theme.choices", 8.0);
    for choice in [
        ShowcaseThemeChoice::Light,
        ShowcaseThemeChoice::Dark,
        ShowcaseThemeChoice::Bubblegum,
    ] {
        theme_choice_button(
            ui,
            choices,
            choice,
            state.showcase_theme == choice,
            choice.theme(),
        );
    }

    let swatches = wrapping_row(ui, body, "theme.swatches", 8.0);
    theme_swatch(
        ui,
        swatches,
        "theme.swatch.canvas",
        "Canvas",
        theme.colors.canvas,
        theme,
    );
    theme_swatch(
        ui,
        swatches,
        "theme.swatch.surface",
        "Surface",
        theme.colors.surface,
        theme,
    );
    theme_swatch(
        ui,
        swatches,
        "theme.swatch.accent",
        "Accent",
        theme.colors.accent,
        theme,
    );
    theme_swatch(
        ui,
        swatches,
        "theme.swatch.selected",
        "Selected",
        theme.colors.selected,
        theme,
    );

    let preview = ui.add_child(
        body,
        UiNode::container(
            "theme.preview",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_padding(12.0)
                .with_gap(10.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            theme.colors.surface,
            Some(theme.stroke.surface),
            theme.radius.md,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label("Theme preview"),
        ),
    );
    widgets::label(
        ui,
        preview,
        "theme.preview.title",
        "Preview controls",
        themed_text(theme, 13.0),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let preview_row = row(ui, preview, "theme.preview.controls", 8.0);
    let mut primary = themed_button_options(
        theme,
        "theme.preview.primary",
        ComponentState::ACTIVE,
        LayoutStyle::new().with_height(34.0),
    );
    primary.accessibility_label = Some("Primary preview button".to_owned());
    widgets::button(ui, preview_row, "theme.preview.primary", "Primary", primary);
    let mut secondary = themed_button_options(
        theme,
        "theme.preview.secondary",
        ComponentState::NORMAL,
        LayoutStyle::new().with_height(34.0),
    );
    secondary.accessibility_label = Some("Secondary preview button".to_owned());
    widgets::button(
        ui,
        preview_row,
        "theme.preview.secondary",
        "Secondary",
        secondary,
    );
    let mut help = themed_muted_text(theme, 12.0);
    help.wrap = TextWrap::WordOrGlyph;
    widgets::label(
        ui,
        preview,
        "theme.preview.copy",
        "The selected theme drives the app background, right panel, floating windows, and this preview.",
        help,
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn theme_choice_button(
    ui: &mut UiDocument,
    parent: UiNodeId,
    choice: ShowcaseThemeChoice,
    selected: bool,
    preview_theme: Theme,
) {
    let mut options = themed_button_options(
        &preview_theme,
        choice.action(),
        if selected {
            ComponentState::SELECTED
        } else {
            ComponentState::NORMAL
        },
        LayoutStyle::new()
            .with_width(116.0)
            .with_height(34.0)
            .with_flex_shrink(0.0),
    )
    .with_action(choice.action());
    options.accessibility_label = Some(format!("Use {} theme", choice.label()));
    widgets::button(
        ui,
        parent,
        format!("theme.choice.{}", choice.label().to_ascii_lowercase()),
        choice.label(),
        options,
    );
}

fn theme_swatch(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    swatch_color: ColorRgba,
    theme: &Theme,
) {
    let tile = ui.add_child(
        parent,
        UiNode::container(
            name,
            LayoutStyle::column()
                .with_width(92.0)
                .with_height(76.0)
                .with_padding(8.0)
                .with_gap(6.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            theme.colors.surface_muted,
            Some(theme.stroke.surface),
            4.0,
        ))
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).label(label)),
    );
    ui.add_child(
        tile,
        UiNode::container(
            format!("{name}.color"),
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(26.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            swatch_color,
            Some(StrokeStyle::new(theme.colors.border_strong, 1.0)),
            4.0,
        )),
    );
    widgets::label(
        ui,
        tile,
        format!("{name}.label"),
        label,
        themed_muted_text(theme, 11.0),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn themed_button_options(
    theme: &Theme,
    action: impl Into<String>,
    state: ComponentState,
    layout: LayoutStyle,
) -> widgets::ButtonOptions {
    widgets::ButtonOptions::themed(theme, state, layout).with_action(action.into())
}

fn styling_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let preview_scene_size = style_preview_scene_size(state.styling);
    let preview_min_width = preview_scene_size.width + 16.0;
    let preview_min_height = preview_scene_size.height + 16.0;
    let body_min_width = STYLING_CONTROLS_WIDTH + 1.0 + preview_min_width + 20.0;
    let body = section_with_min_viewport(
        ui,
        parent,
        "styling",
        "Styling",
        UiSize::new(body_min_width, preview_min_height),
    );
    let grid_layout = operad::layout::with_grid_template_columns(
        Layout::grid()
            .size(LayoutSize::percent(1.0, 1.0))
            .gap(LayoutGap::points(10.0, 10.0))
            .to_layout_style(),
        [
            LayoutGridTrack::points(STYLING_CONTROLS_WIDTH),
            LayoutGridTrack::points(1.0),
            LayoutGridTrack::minmax_points_fraction(preview_min_width, 1.0),
        ],
    );
    let grid = ui.add_child(body, UiNode::container("styling.grid", grid_layout));
    let controls = ui.add_child(
        grid,
        UiNode::container(
            "styling.controls",
            LayoutStyle::column()
                .with_width(STYLING_CONTROLS_WIDTH)
                .with_height_percent(1.0)
                .with_flex_shrink(0.0)
                .gap(6.0),
        ),
    );
    style_edge_group(
        ui,
        controls,
        "styling.inner",
        "Inner margin",
        "styling.inner_same",
        state.styling.inner_same,
        [
            ("Left", "styling.inner", state.styling.inner_margin),
            ("Right", "styling.inner_right", state.styling.inner_right),
            ("Top", "styling.inner_top", state.styling.inner_top),
            ("Bottom", "styling.inner_bottom", state.styling.inner_bottom),
        ],
        0.0..32.0,
    );
    style_edge_group(
        ui,
        controls,
        "styling.outer",
        "Outer margin",
        "styling.outer_same",
        state.styling.outer_same,
        [
            ("Left", "styling.outer", state.styling.outer_margin),
            ("Right", "styling.outer_right", state.styling.outer_right),
            ("Top", "styling.outer_top", state.styling.outer_top),
            ("Bottom", "styling.outer_bottom", state.styling.outer_bottom),
        ],
        0.0..40.0,
    );
    style_edge_group(
        ui,
        controls,
        "styling.radius",
        "Corner radius",
        "styling.radius_same",
        state.styling.radius_same,
        [
            ("NW", "styling.radius", state.styling.corner_radius),
            ("NE", "styling.radius_ne", state.styling.corner_ne),
            ("SW", "styling.radius_sw", state.styling.corner_sw),
            ("SE", "styling.radius_se", state.styling.corner_se),
        ],
        0.0..28.0,
    );
    style_fill_group(ui, controls, state);
    style_stroke_group(ui, controls, state);
    style_shadow_group(ui, controls, state);
    widgets::separator(
        ui,
        grid,
        "styling.preview.separator",
        widgets::SeparatorOptions::vertical().with_layout(
            LayoutStyle::new()
                .with_width(1.0)
                .with_height_percent(1.0)
                .with_flex_shrink(0.0),
        ),
    );

    let preview = ui.add_child(
        grid,
        UiNode::container(
            "styling.preview",
            operad::layout::with_min_size(
                LayoutStyle::column()
                    .with_width_percent(1.0)
                    .with_height_percent(1.0)
                    .with_flex_shrink(0.0)
                    .padding(8.0),
                operad::layout::px(preview_min_width),
                operad::layout::px(preview_min_height),
            ),
        )
        .with_visual(UiVisual::panel(color(17, 20, 25), None, 0.0)),
    );
    style_preview(ui, preview, state.styling);
}

#[allow(clippy::too_many_arguments)]
fn style_edge_group(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    title: &'static str,
    same_action: &'static str,
    same: bool,
    values: [(&'static str, &'static str, f32); 4],
    range: std::ops::Range<f32>,
) {
    let group = style_control_group(ui, parent, format!("{name}.group"));
    style_group_title(ui, group, format!("{name}.title"), title);
    let fields = ui.add_child(
        group,
        UiNode::container(
            format!("{name}.fields"),
            LayoutStyle::column()
                .with_width(138.0)
                .with_flex_shrink(0.0)
                .gap(3.0),
        ),
    );
    style_compact_checkbox(ui, fields, same_action, "same", same);
    if same {
        style_number_row(ui, fields, values[0].1, "All", values[0].2, range, 0);
    } else {
        for (label, action, value) in values {
            style_number_row(ui, fields, action, label, value, range.clone(), 0);
        }
    }
}

fn style_fill_group(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let group = style_control_group(ui, parent, "styling.fill.group");
    style_group_title(ui, group, "styling.fill.title", "Fill");
    let fields = style_group_fields(
        ui,
        group,
        "styling.fill.fields",
        STYLING_WIDE_FIELDS_WIDTH,
        4.0,
    );
    style_color_button_row(
        ui,
        fields,
        "styling.fill_color_button",
        "",
        state.styling.fill_color(),
        "Pick fill color",
    );
    if state.styling_fill_picker_open {
        ext_widgets::color_picker(
            ui,
            fields,
            "styling.fill_picker",
            &state.styling_fill_picker,
            ext_widgets::ColorPickerOptions::default()
                .with_label("Fill")
                .with_action_prefix("styling.fill_picker"),
        );
    }
}

fn style_stroke_group(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let group = style_control_group(ui, parent, "styling.stroke.group");
    style_group_title(ui, group, "styling.stroke.title", "Stroke");
    let fields = style_group_fields(
        ui,
        group,
        "styling.stroke.fields",
        STYLING_WIDE_FIELDS_WIDTH,
        4.0,
    );
    let width_row = row(ui, fields, "styling.stroke.row", 6.0);
    style_inline_number(
        ui,
        width_row,
        "styling.stroke",
        "width",
        state.styling.stroke_width,
        0.0..STYLING_STROKE_MAX,
        1,
    );
    let mut options = widgets::SliderOptions::default()
        .with_layout(
            LayoutStyle::new()
                .with_width(60.0)
                .with_height(20.0)
                .with_flex_shrink(0.0),
        )
        .with_value_edit_action("styling.stroke");
    options.fill_color = color(120, 170, 230);
    widgets::slider(
        ui,
        width_row,
        "styling.stroke.slider",
        (state.styling.stroke_width / STYLING_STROKE_MAX).clamp(0.0, 1.0),
        0.0..1.0,
        options,
    );
    style_color_button_row(
        ui,
        fields,
        "styling.stroke_color_button",
        "",
        state.styling.stroke_color(),
        "Pick stroke color",
    );
    if state.styling_stroke_picker_open {
        ext_widgets::color_picker(
            ui,
            fields,
            "styling.stroke_picker",
            &state.styling_stroke_picker,
            ext_widgets::ColorPickerOptions::default()
                .with_label("Stroke color")
                .with_action_prefix("styling.stroke_picker"),
        );
    }
}

fn style_shadow_group(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let group = style_control_group(ui, parent, "styling.shadow.group");
    style_group_title(ui, group, "styling.shadow.title", "Shadow");
    let fields = style_group_fields(
        ui,
        group,
        "styling.shadow.fields",
        STYLING_WIDE_FIELDS_WIDTH,
        4.0,
    );
    let offsets = row(ui, fields, "styling.shadow.offsets", 6.0);
    style_inline_number(
        ui,
        offsets,
        "styling.shadow_x",
        "x",
        state.styling.shadow_x,
        -24.0..24.0,
        0,
    );
    style_inline_number(
        ui,
        offsets,
        "styling.shadow_y",
        "y",
        state.styling.shadow_y,
        -24.0..24.0,
        0,
    );
    let spread = row(ui, fields, "styling.shadow.blur_spread", 6.0);
    style_inline_number(
        ui,
        spread,
        "styling.shadow",
        "blur",
        state.styling.shadow_blur,
        0.0..32.0,
        0,
    );
    style_inline_number(
        ui,
        spread,
        "styling.shadow_spread",
        "spread",
        state.styling.shadow_spread,
        0.0..16.0,
        0,
    );
    style_color_button_row(
        ui,
        fields,
        "styling.shadow_color_button",
        "",
        state.styling.shadow_color(),
        "Pick shadow color",
    );
    if state.styling_shadow_picker_open {
        ext_widgets::color_picker(
            ui,
            fields,
            "styling.shadow_picker",
            &state.styling_shadow_picker,
            ext_widgets::ColorPickerOptions::default()
                .with_label("Shadow color")
                .with_action_prefix("styling.shadow_picker"),
        );
    }
}

fn style_control_group(ui: &mut UiDocument, parent: UiNodeId, name: impl Into<String>) -> UiNodeId {
    ui.add_child(
        parent,
        UiNode::container(
            name,
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_flex_shrink(0.0)
                .padding(4.0)
                .gap(8.0),
        )
        .with_visual(UiVisual::panel(color(23, 27, 33), None, 2.0)),
    )
}

fn style_group_fields(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    width: f32,
    gap: f32,
) -> UiNodeId {
    ui.add_child(
        parent,
        UiNode::container(
            name,
            LayoutStyle::column()
                .with_width(width)
                .with_flex_shrink(0.0)
                .gap(gap),
        ),
    )
}

fn style_group_title(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: &'static str,
) {
    widgets::label(
        ui,
        parent,
        name,
        label,
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new()
            .with_width(88.0)
            .with_flex_shrink(0.0)
            .with_height(22.0),
    );
}

fn style_color_button_row(
    ui: &mut UiDocument,
    parent: UiNodeId,
    action: &'static str,
    label: &'static str,
    value: ColorRgba,
    accessibility_label: &'static str,
) {
    let row = row(ui, parent, format!("{action}.row"), 8.0);
    if !label.is_empty() {
        widgets::label(
            ui,
            row,
            format!("{action}.label"),
            label,
            text(12.0, color(166, 176, 190)),
            LayoutStyle::new()
                .with_width(86.0)
                .with_flex_shrink(0.0)
                .with_height(24.0),
        );
    }
    ext_widgets::color_edit_button(
        ui,
        row,
        action,
        value,
        color_mini_button_options(action)
            .with_format(ext_widgets::ColorValueFormat::Rgba)
            .accessibility_label(accessibility_label),
    );
    widgets::label(
        ui,
        row,
        format!("{action}.value"),
        ext_widgets::color_picker::format_hex_color(value, value.a < 255),
        text(12.0, color(226, 232, 242)),
        LayoutStyle::new().with_width(96.0).with_height(24.0),
    );
}

fn style_number_row(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    value: f32,
    range: std::ops::Range<f32>,
    decimals: u8,
) {
    let row = row(ui, parent, format!("{name}.row"), 6.0);
    widgets::label(
        ui,
        row,
        format!("{name}.label"),
        label,
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width(48.0).with_height(22.0),
    );
    style_value_input(ui, row, name, value, range, decimals);
}

fn style_inline_number(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    value: f32,
    range: std::ops::Range<f32>,
    decimals: u8,
) {
    let row = compact_row(ui, parent, format!("{name}.inline"), 3.0);
    widgets::label(
        ui,
        row,
        format!("{name}.inline_label"),
        format!("{label}:"),
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new()
            .with_width(if label.len() > 1 { 42.0 } else { 16.0 })
            .with_height(22.0),
    );
    style_value_input(ui, row, name, value, range, decimals);
}

fn style_value_input(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    value: f32,
    range: std::ops::Range<f32>,
    decimals: u8,
) {
    let mut options = widgets::DragValueOptions::default()
        .with_layout(
            LayoutStyle::row()
                .with_width(STYLING_VALUE_INPUT_WIDTH)
                .with_height(22.0)
                .with_flex_shrink(0.0)
                .with_align_items(taffy::prelude::AlignItems::Center)
                .with_justify_content(taffy::prelude::JustifyContent::Center)
                .with_padding(4.0),
        )
        .with_range(ext_widgets::NumericRange::new(
            f64::from(range.start),
            f64::from(range.end),
        ))
        .with_precision(ext_widgets::NumericPrecision::decimals(decimals))
        .with_action(name);
    options.text_style = text(12.0, color(226, 232, 242));
    widgets::drag_value_input(ui, parent, name, f64::from(value), options);
}

fn style_compact_checkbox(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    checked: bool,
) {
    let mut options = widgets::CheckboxOptions::default().with_action(name);
    options.layout = LayoutStyle::new().with_width(92.0).with_height(22.0);
    options.text_style = text(12.0, color(220, 228, 238));
    widgets::checkbox(ui, parent, name, label, checked, options);
}

fn compact_row(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    gap: f32,
) -> UiNodeId {
    ui.add_child(
        parent,
        UiNode::container(
            name,
            LayoutStyle::row()
                .with_height(22.0)
                .with_flex_shrink(0.0)
                .with_align_items(taffy::prelude::AlignItems::Center)
                .gap(gap),
        ),
    )
}

fn color_mini_button_options(action: &'static str) -> ext_widgets::ColorButtonOptions {
    ext_widgets::ColorButtonOptions::default()
        .with_layout(LayoutStyle::size(28.0, 24.0).with_flex_shrink(0.0))
        .with_swatch_size(UiSize::new(22.0, 18.0))
        .with_action(action)
        .show_label(false)
}

fn style_preview(ui: &mut UiDocument, parent: UiNodeId, styling: StylingState) {
    let (frame, text_rect) = style_preview_rects(styling);
    let scene_size = style_preview_scene_size(styling);
    ui.add_child(
        parent,
        UiNode::scene(
            "styling.preview.scene",
            vec![
                ScenePrimitive::Rect(
                    PaintRect::solid(frame, styling.fill_color())
                        .stroke(AlignedStroke::inside(StrokeStyle::new(
                            styling.stroke_color(),
                            styling.stroke_width,
                        )))
                        .corner_radii(styling.radii())
                        .effect(PaintEffect::shadow(
                            styling.shadow_color(),
                            UiPoint::new(styling.shadow_x, styling.shadow_y),
                            styling.shadow_blur,
                            styling.shadow_spread,
                        )),
                ),
                ScenePrimitive::Text(
                    PaintText::new("Content", text_rect, text(13.0, color(255, 255, 255)))
                        .horizontal_align(TextHorizontalAlign::Center)
                        .vertical_align(TextVerticalAlign::Center)
                        .multiline(false),
                ),
            ],
            operad::layout::with_min_size(
                LayoutStyle::new()
                    .with_width_percent(1.0)
                    .with_height(180.0)
                    .with_flex_shrink(0.0),
                operad::layout::px(scene_size.width),
                operad::layout::px(scene_size.height),
            ),
        ),
    );
}

fn style_preview_rects(styling: StylingState) -> (UiRect, UiRect) {
    let outer = styling.outer_edges();
    let inner = styling.inner_edges();
    let frame = UiRect::new(
        22.0 + outer[0],
        28.0 + outer[2],
        108.0 + inner[0] + inner[1],
        40.0 + inner[2] + inner[3],
    );
    let text_rect = UiRect::new(
        frame.x + inner[0],
        frame.y + inner[2],
        (frame.width - inner[0] - inner[1]).max(1.0),
        (frame.height - inner[2] - inner[3]).max(1.0),
    );
    (frame, text_rect)
}

fn style_preview_scene_size(styling: StylingState) -> UiSize {
    let (frame, text_rect) = style_preview_rects(styling);
    let shadow_outset = styling.shadow_blur.max(0.0) + styling.shadow_spread.max(0.0);
    let shadow_bounds = UiRect::new(
        frame.x + styling.shadow_x - shadow_outset,
        frame.y + styling.shadow_y - shadow_outset,
        frame.width + shadow_outset * 2.0,
        frame.height + shadow_outset * 2.0,
    );
    let right = frame
        .right()
        .max(text_rect.right())
        .max(shadow_bounds.right());
    let bottom = frame
        .bottom()
        .max(text_rect.bottom())
        .max(shadow_bounds.bottom())
        .max(180.0);
    UiSize::new(right.ceil().max(1.0), bottom.ceil().max(1.0))
}

fn slider_options(state: &ShowcaseState, width: f32) -> widgets::SliderOptions {
    let mut options = widgets::SliderOptions::default().with_layout(
        LayoutStyle::new()
            .with_width(width)
            .with_height(24.0)
            .with_flex_shrink(0.0),
    );
    options.fill_color = if state.slider_trailing_color {
        state.slider_trailing_picker.value()
    } else {
        color(42, 49, 58)
    };
    options.thumb_shape = match state.slider_thumb_shape {
        SliderThumbChoice::Circle => widgets::slider::SliderThumbShape::Circle,
        SliderThumbChoice::Square => widgets::slider::SliderThumbShape::Square,
        SliderThumbChoice::Rectangle => widgets::slider::SliderThumbShape::Rectangle,
    };
    options.thumb_visual = UiVisual::panel(
        state.slider_thumb_picker.value(),
        Some(StrokeStyle::new(color(79, 93, 113), 1.0)),
        6.0,
    );
    options
}

#[allow(clippy::field_reassign_with_default)]
fn slider_number_input(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    input: &TextInputState,
    focused: FocusedTextInput,
    state: &ShowcaseState,
    width: f32,
) {
    let mut options = TextInputOptions::default();
    options.layout = LayoutStyle::new().with_width(width).with_height(28.0);
    options.text_style = text(12.0, color(230, 236, 246));
    options.placeholder_style = text(12.0, color(144, 156, 174));
    options.edit_action = Some(format!("{name}.edit").into());
    options.focused = state.focused_text == Some(focused);
    options.caret_visible = caret_visible(state.caret_phase);
    widgets::text_input(ui, parent, name, input, options);
}

fn form_status_chip(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    active: bool,
) {
    let chip = ui.add_child(
        parent,
        UiNode::container(
            name,
            LayoutStyle::new()
                .with_width(82.0)
                .with_height(24.0)
                .with_padding(4.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            if active {
                color(35, 74, 54)
            } else {
                color(28, 34, 43)
            },
            Some(StrokeStyle::new(
                if active {
                    color(90, 160, 112)
                } else {
                    color(60, 72, 88)
                },
                1.0,
            )),
            4.0,
        )),
    );
    widgets::label(
        ui,
        chip,
        format!("{name}.label"),
        label,
        text(11.0, color(218, 228, 240)),
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height_percent(1.0),
    );
}

fn profile_form_summary(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let has_errors = widgets::form_has_errors(&state.form);
    let title = profile_form_summary_title(state, has_errors);
    let detail = format!(
        "{} | {} | {}",
        profile_summary_value(state.form_name_text.text(), "No name"),
        profile_summary_value(state.form_email_text.text(), "No email"),
        profile_summary_value(state.form_role_text.text(), "No role"),
    );
    let hint = profile_form_summary_hint(state, has_errors);
    let stroke = if has_errors {
        color(196, 94, 104)
    } else if state.form.dirty {
        color(205, 160, 71)
    } else if state.form.submitted {
        color(91, 164, 119)
    } else {
        color(60, 72, 88)
    };
    let summary = ui.add_child(
        parent,
        UiNode::container(
            "forms.profile.summary",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_padding(10.0)
                .with_gap(4.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(20, 25, 32),
            Some(StrokeStyle::new(stroke, 1.0)),
            4.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group)
                .label("Live profile summary")
                .value(format!("{title}. {detail}. {hint}")),
        ),
    );
    widgets::label(
        ui,
        summary,
        "forms.profile.summary.title",
        title,
        text(13.0, color(232, 240, 250)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::label(
        ui,
        summary,
        "forms.profile.summary.detail",
        detail,
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::label(
        ui,
        summary,
        "forms.profile.summary.hint",
        hint,
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn profile_form_summary_title(state: &ShowcaseState, has_errors: bool) -> &'static str {
    if has_errors {
        "Profile needs fixes"
    } else if state.form.submitted {
        "Profile submitted"
    } else if state.form.dirty {
        "Profile draft"
    } else {
        "Profile saved"
    }
}

fn profile_form_summary_hint(state: &ShowcaseState, has_errors: bool) -> &'static str {
    if has_errors {
        "Fix validation errors before applying or submitting."
    } else if state.form.dirty {
        "Apply saves the draft; Submit saves and marks it submitted."
    } else if state.form.submitted {
        "Submission completed. Apply stays disabled until something changes."
    } else {
        "No pending changes. Submit marks the saved profile submitted."
    }
}

fn profile_summary_value<'a>(value: &'a str, empty: &'static str) -> &'a str {
    let value = value.trim();
    if value.is_empty() {
        empty
    } else {
        value
    }
}

#[allow(clippy::field_reassign_with_default)]
fn form_text_field(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    input: &TextInputState,
    focused: FocusedTextInput,
    state: &ShowcaseState,
) {
    let mut options = TextInputOptions::default();
    options.layout = LayoutStyle::new().with_width_percent(1.0).with_height(30.0);
    options.text_style = text(12.0, color(230, 236, 246));
    options.placeholder_style = text(12.0, color(144, 156, 174));
    options.placeholder = "Required".to_string();
    options.edit_action = Some(format!("{name}.edit").into());
    options.focused = state.focused_text == Some(focused);
    options.caret_visible = caret_visible(state.caret_phase);
    widgets::text_input(ui, parent, name, input, options);
}

fn profile_email_valid(email: &str) -> bool {
    let email = email.trim();
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
}

fn drag_source_layout() -> LayoutStyle {
    LayoutStyle::row()
        .with_width(128.0)
        .with_height(40.0)
        .with_padding(8.0)
        .with_gap(6.0)
        .with_flex_shrink(0.0)
}

fn drop_zone_layout() -> LayoutStyle {
    LayoutStyle::column()
        .with_width(128.0)
        .with_height(78.0)
        .with_padding(10.0)
        .with_gap(6.0)
        .with_flex_shrink(0.0)
}

fn dnd_operation_chip(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
) {
    let chip = ui.add_child(
        parent,
        UiNode::container(
            name,
            LayoutStyle::new()
                .with_width(58.0)
                .with_height(22.0)
                .with_padding(3.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(26, 32, 42),
            Some(StrokeStyle::new(color(62, 76, 94), 1.0)),
            3.0,
        )),
    );
    widgets::label(
        ui,
        chip,
        format!("{name}.label"),
        label,
        text(11.0, color(190, 204, 222)),
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height_percent(1.0),
    );
}

fn media_preview_image_layout() -> LayoutStyle {
    LayoutStyle::size(46.0, 46.0).with_flex_shrink(0.0)
}

fn media_icon_columns(state: &ShowcaseState) -> usize {
    let theme = state.app_theme();
    let options = showcase_desktop_options(state.last_desktop_size, &theme);
    let window_width = state
        .desktop
        .size("media", default_window_size("media"))
        .width;
    let content_width = (window_width - options.content_padding * 2.0).max(MEDIA_ICON_TILE_WIDTH);
    let pitch = MEDIA_ICON_TILE_WIDTH + MEDIA_ICON_GRID_GAP;
    (((content_width + MEDIA_ICON_GRID_GAP) / pitch).floor() as usize).clamp(1, MEDIA_ICON_COLUMNS)
}

fn media_icon_grid_width(columns: usize) -> f32 {
    let columns = columns.max(1);
    columns as f32 * MEDIA_ICON_TILE_WIDTH + columns.saturating_sub(1) as f32 * MEDIA_ICON_GRID_GAP
}

fn media_icon_grid_height(columns: usize, item_count: usize) -> f32 {
    let columns = columns.max(1);
    let rows = item_count.div_ceil(columns).max(1);
    rows as f32 * MEDIA_ICON_TILE_HEIGHT + rows.saturating_sub(1) as f32 * MEDIA_ICON_GRID_GAP
}

fn media_icon_grid(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    columns: usize,
    item_count: usize,
) -> UiNodeId {
    let columns = columns.clamp(1, MEDIA_ICON_COLUMNS);
    let rows = item_count.div_ceil(columns).max(1);
    let width = media_icon_grid_width(columns);
    let height = media_icon_grid_height(columns, item_count);
    let layout = operad::layout::with_grid_template_rows(
        operad::layout::with_grid_template_columns(
            Layout::grid()
                .size(LayoutSize::points(width, height))
                .gap(LayoutGap::points(MEDIA_ICON_GRID_GAP, MEDIA_ICON_GRID_GAP))
                .flex(0.0, 0.0, LayoutDimension::Auto)
                .to_layout_style(),
            (0..columns).map(|_| LayoutGridTrack::points(MEDIA_ICON_TILE_WIDTH)),
        ),
        (0..rows).map(|_| LayoutGridTrack::points(MEDIA_ICON_TILE_HEIGHT)),
    );
    ui.add_child(parent, UiNode::container(name, layout))
}

fn media_icon_tile(ui: &mut UiDocument, parent: UiNodeId, icon: BuiltInIcon) {
    let name = icon.key().replace('.', "_").replace('-', "_");
    let tile = ui.add_child(
        parent,
        UiNode::container(
            format!("media.icon_tile.{name}"),
            LayoutStyle::column()
                .with_width(MEDIA_ICON_TILE_WIDTH)
                .with_height(MEDIA_ICON_TILE_HEIGHT)
                .with_padding(6.0)
                .with_gap(4.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(17, 22, 30),
            Some(StrokeStyle::new(color(50, 62, 78), 1.0)),
            4.0,
        )),
    );
    widgets::image(
        ui,
        tile,
        format!("media.icon.{name}"),
        icon_image(icon),
        widgets::ImageOptions::default()
            .with_layout(LayoutStyle::size(28.0, 28.0))
            .with_accessibility_label(icon.label()),
    );
    widgets::label(
        ui,
        tile,
        format!("media.icon_label.{name}"),
        icon.label(),
        text(9.0, color(180, 194, 214)),
        LayoutStyle::new().with_width_percent(1.0).with_height(30.0),
    );
}

fn slider_checkbox(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    checked: bool,
) {
    slider_checkbox_with_layout(
        ui,
        parent,
        name,
        label,
        checked,
        LayoutStyle::new().with_width_percent(1.0).with_height(30.0),
    );
}

fn slider_checkbox_with_layout(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    checked: bool,
    layout: LayoutStyle,
) {
    let mut options = widgets::CheckboxOptions::default().with_action(name);
    options.layout = layout;
    options.text_style = text(12.0, color(220, 228, 238));
    widgets::checkbox(ui, parent, name, label, checked, options);
}

fn segmented_options(action_prefix: &'static str) -> ext_widgets::SegmentedControlOptions {
    let mut options = ext_widgets::SegmentedControlOptions::default()
        .with_action_prefix(action_prefix)
        .with_layout(LayoutStyle::row().with_height(30.0).with_gap(4.0));
    options.visual = button_visual(38, 46, 58);
    options.selected_visual = button_visual(48, 112, 184);
    options.hovered_visual = Some(button_visual(65, 86, 106));
    options.pressed_visual = Some(button_visual(34, 54, 84));
    options.text_style = text(12.0, color(238, 244, 252));
    options
}

fn divider(ui: &mut UiDocument, parent: UiNodeId, name: &'static str) {
    ui.add_child(
        parent,
        UiNode::container(
            name,
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(1.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(color(48, 58, 72), None, 0.0)),
    );
}

fn canvas(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let canvas_intrinsic = UiSize::new(720.0, 405.0);
    let body = section_with_min_viewport(ui, parent, "canvas", "Canvas", UiSize::new(720.0, 458.0));
    let controls = wrapping_row(ui, body, "canvas.options", 10.0);
    canvas_option_checkbox(
        ui,
        controls,
        "canvas.grow_horizontal",
        "Grow width",
        state.canvas_grow_horizontal,
    );
    canvas_option_checkbox(
        ui,
        controls,
        "canvas.grow_vertical",
        "Grow height",
        state.canvas_grow_vertical,
    );
    canvas_option_checkbox(
        ui,
        controls,
        "canvas.keep_aspect_ratio",
        "Keep aspect ratio",
        state.canvas_keep_aspect_ratio,
    );

    let mut options = widgets::CanvasOptions::default()
        .with_accessibility_label("Shader canvas")
        .with_action("canvas.rotate")
        .with_intrinsic_size(canvas_intrinsic);
    options.action_mode = WidgetActionMode::Drag;
    if state.canvas_keep_aspect_ratio {
        options = options.with_aspect_ratio(16.0 / 9.0);
    }
    let canvas_width = if state.canvas_grow_horizontal {
        LayoutDimension::percent(1.0)
    } else {
        LayoutDimension::points(canvas_intrinsic.width)
    };
    let canvas_height = if state.canvas_grow_vertical {
        LayoutDimension::percent(1.0)
    } else {
        LayoutDimension::points(canvas_intrinsic.height)
    };
    options.layout = Layout::new()
        .size(LayoutSize::new(canvas_width, canvas_height))
        .min_size(LayoutSize::points(
            canvas_intrinsic.width,
            canvas_intrinsic.height,
        ))
        .flex(
            if state.canvas_grow_vertical { 1.0 } else { 0.0 },
            1.0,
            LayoutDimension::Auto,
        )
        .to_layout_style();
    options.visual = UiVisual::panel(
        color(18, 22, 28),
        Some(StrokeStyle::new(color(58, 68, 84), 1.0)),
        4.0,
    );
    widgets::canvas(
        ui,
        body,
        "canvas.shader",
        CanvasContent::new("canvas.shader").program(showcase_canvas_program(state.cube)),
        options,
    );
}

fn canvas_option_checkbox(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    checked: bool,
) {
    let mut options = widgets::CheckboxOptions::default()
        .with_action(name)
        .with_text_style(text(12.0, color(220, 228, 238)));
    options.layout = LayoutStyle::new().with_height(28.0).with_flex_shrink(0.0);
    widgets::checkbox(ui, parent, name, label, checked, options);
}

fn showcase_canvas_program(cube: CanvasCubeState) -> CanvasRenderProgram {
    CanvasRenderProgram::wgsl(include_str!("shaders/showcase_canvas.wgsl"))
        .label("showcase.canvas")
        .constant("CUBE_YAW", cube.yaw as f64)
        .constant("CUBE_PITCH", cube.pitch as f64)
        .clear_color(Some(color(18, 22, 28)))
}

fn section(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    _title: impl Into<String>,
) -> UiNodeId {
    section_with_min_viewport(ui, parent, name, _title, UiSize::ZERO)
}

fn section_with_min_viewport(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    _title: impl Into<String>,
    min_viewport_size: UiSize,
) -> UiNodeId {
    let name = name.into();
    let layout = Layout::column()
        .size(LayoutSize::percent(1.0, 1.0))
        .min_size(LayoutSize::points(
            min_viewport_size.width.max(0.0),
            min_viewport_size.height.max(0.0),
        ))
        .gap(LayoutGap::points(10.0, 10.0))
        .flex(1.0, 1.0, LayoutDimension::Auto)
        .to_layout_style();
    widgets::scroll_area(
        ui,
        parent,
        format!("{name}.section_scroll"),
        ScrollAxes::BOTH,
        layout,
    )
}

fn row(ui: &mut UiDocument, parent: UiNodeId, name: impl Into<String>, gap: f32) -> UiNodeId {
    ui.add_child(
        parent,
        UiNode::container(
            name,
            Layout::row()
                .size(LayoutSize::new(
                    LayoutDimension::percent(1.0),
                    LayoutDimension::Auto,
                ))
                .align_items(LayoutAlignment::Center)
                .gap(LayoutGap::points(gap, gap))
                .flex(0.0, 0.0, LayoutDimension::Auto)
                .to_layout_style(),
        ),
    )
}

fn wrapping_row(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    gap: f32,
) -> UiNodeId {
    ui.add_child(
        parent,
        UiNode::container(
            name,
            Layout::row()
                .size(LayoutSize::new(
                    LayoutDimension::percent(1.0),
                    LayoutDimension::Auto,
                ))
                .min_size(LayoutSize::points(0.0, 0.0))
                .align_items(LayoutAlignment::Center)
                .gap(LayoutGap::points(gap, gap))
                .flex_wrap(LayoutFlexWrap::Wrap)
                .flex(0.0, 0.0, LayoutDimension::Auto)
                .to_layout_style(),
        ),
    )
}

fn layout_panel_contents(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    offset_y: f32,
    items: &[&'static str],
) {
    let scroll = widgets::scroll_area_with_options(
        ui,
        parent,
        format!("{name}.scroll_area"),
        widgets::ScrollAreaOptions::new(
            ScrollAxes::VERTICAL,
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(0.0)
                .with_flex_grow(1.0)
                .with_padding(8.0)
                .with_gap(6.0),
        )
        .with_scroll(
            operad::ScrollState::new(ScrollAxes::VERTICAL).with_offset(UiPoint::new(0.0, offset_y)),
        )
        .with_action(format!("{name}.scroll")),
    );
    for (index, item) in items.iter().enumerate() {
        let row = ui.add_child(
            scroll,
            UiNode::container(
                format!("{name}.row.{index}"),
                LayoutStyle::row()
                    .with_width_percent(1.0)
                    .with_height(30.0)
                    .with_align_items(taffy::prelude::AlignItems::Center)
                    .with_padding(8.0)
                    .with_flex_shrink(0.0),
            )
            .with_visual(UiVisual::panel(
                color(20, 26, 34),
                Some(StrokeStyle::new(color(45, 56, 72), 1.0)),
                4.0,
            )),
        );
        widgets::label(
            ui,
            row,
            format!("{name}.row.{index}.label"),
            *item,
            text(12.0, color(218, 228, 242)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }
}

fn layout_workspace_contents(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    offset_y: f32,
) {
    let scroll = widgets::scroll_area_with_options(
        ui,
        parent,
        format!("{name}.scroll_area"),
        widgets::ScrollAreaOptions::new(
            ScrollAxes::VERTICAL,
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(0.0)
                .with_flex_grow(1.0)
                .with_padding(10.0)
                .with_gap(10.0),
        )
        .with_scroll(
            operad::ScrollState::new(ScrollAxes::VERTICAL).with_offset(UiPoint::new(0.0, offset_y)),
        )
        .with_action(format!("{name}.scroll")),
    );
    let row_one = wrapping_row(ui, scroll, "layout.workspace.row.primary", 8.0);
    layout_card(
        ui,
        row_one,
        "layout.workspace.card.one",
        "Region 1",
        "Flexible",
    );
    layout_card(
        ui,
        row_one,
        "layout.workspace.card.two",
        "Region 2",
        "Wraps",
    );
    layout_card(
        ui,
        row_one,
        "layout.workspace.card.three",
        "Region 3",
        "Grows",
    );

    let row_two = row(ui, scroll, "layout.workspace.row.secondary", 8.0);
    layout_card(
        ui,
        row_two,
        "layout.workspace.card.four",
        "Region 4",
        "Left",
    );
    layout_card(
        ui,
        row_two,
        "layout.workspace.card.five",
        "Region 5",
        "Right",
    );
}

fn layout_card(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    title: &'static str,
    subtitle: &'static str,
) -> UiNodeId {
    let card = ui.add_child(
        parent,
        UiNode::container(
            name,
            LayoutStyle::column()
                .with_width(128.0)
                .with_height(76.0)
                .with_flex_grow(1.0)
                .with_flex_shrink(1.0)
                .with_padding(8.0)
                .with_gap(4.0),
        )
        .with_visual(UiVisual::panel(
            color(21, 28, 38),
            Some(StrokeStyle::new(color(55, 68, 86), 1.0)),
            5.0,
        )),
    );
    widgets::label(
        ui,
        card,
        format!("{name}.title"),
        title,
        text(13.0, color(236, 242, 250)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::label(
        ui,
        card,
        format!("{name}.subtitle"),
        subtitle,
        text(11.0, color(162, 176, 196)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    card
}

fn button(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    action: impl Into<String>,
    visual: UiVisual,
) -> UiNodeId {
    let mut options = widgets::ButtonOptions::new(LayoutStyle::new().with_height(32.0))
        .with_action(action.into());
    options.visual = visual;
    options.hovered_visual = Some(readable_button_hover_visual(visual));
    options.pressed_visual = Some(adjusted_button_visual(visual, -62));
    options.pressed_hovered_visual = Some(adjusted_button_visual(visual, -24));
    options.text_style = text(13.0, color(246, 249, 252));
    widgets::button(ui, parent, name, label, options)
}

fn button_visual(r: u8, g: u8, b: u8) -> UiVisual {
    UiVisual::panel(
        color(r, g, b),
        Some(StrokeStyle::new(color(86, 102, 124), 1.0)),
        4.0,
    )
}

fn color_square_button_options(action: &'static str) -> ext_widgets::ColorButtonOptions {
    ext_widgets::ColorButtonOptions::default()
        .with_layout(LayoutStyle::size(30.0, 30.0).with_flex_shrink(0.0))
        .with_swatch_size(UiSize::new(30.0, 30.0))
        .with_action(action)
        .show_label(false)
}

fn icon_image(icon: BuiltInIcon) -> ImageContent {
    ImageContent::new(icon.key()).tinted(color(220, 228, 238))
}

fn adjusted_button_visual(visual: UiVisual, delta: i16) -> UiVisual {
    UiVisual::panel(
        adjust_color(visual.fill, delta),
        visual.stroke.map(|stroke| StrokeStyle {
            color: adjust_color(stroke.color, delta / 2),
            width: stroke.width,
        }),
        visual.corner_radius,
    )
}

fn readable_button_hover_visual(visual: UiVisual) -> UiVisual {
    let hovered = adjusted_button_visual(visual, 18);
    if contrast_ratio(hovered.fill, color(246, 249, 252)) >= 4.5 {
        hovered
    } else {
        adjusted_button_visual(visual, -8)
    }
}

fn adjust_color(color: ColorRgba, delta: i16) -> ColorRgba {
    let channel = |value: u8| -> u8 { (i16::from(value) + delta).clamp(0, u8::MAX as i16) as u8 };
    ColorRgba::new(
        channel(color.r),
        channel(color.g),
        channel(color.b),
        color.a,
    )
}

fn contrast_ratio(left: ColorRgba, right: ColorRgba) -> f32 {
    let left = relative_luminance(left);
    let right = relative_luminance(right);
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

fn relative_luminance(color: ColorRgba) -> f32 {
    fn channel(value: u8) -> f32 {
        let value = f32::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

fn select_options() -> Vec<ext_widgets::SelectOption> {
    vec![
        ext_widgets::SelectOption::new("label-1", "Label 1"),
        ext_widgets::SelectOption::new("label-2", "Label 2"),
        ext_widgets::SelectOption::new("label-3", "Label 3"),
        ext_widgets::SelectOption::new("disabled", "Disabled").disabled(),
    ]
}

fn select_options_with_images() -> Vec<ext_widgets::SelectOption> {
    vec![
        ext_widgets::SelectOption::new("label-1", "Label 1").image_key(BuiltInIcon::Check.key()),
        ext_widgets::SelectOption::new("label-2", "Label 2").image_key(BuiltInIcon::Folder.key()),
        ext_widgets::SelectOption::new("label-3", "Label 3").image_key(BuiltInIcon::Grid.key()),
        ext_widgets::SelectOption::new("disabled", "Disabled")
            .image_key(BuiltInIcon::Close.key())
            .disabled(),
    ]
}

fn label_locale_options() -> Vec<ext_widgets::SelectOption> {
    vec![
        ext_widgets::SelectOption::new("en-US", "English"),
        ext_widgets::SelectOption::new("es-MX", "Español"),
        ext_widgets::SelectOption::new("fr-FR", "Français"),
        ext_widgets::SelectOption::new("de-DE", "Deutsch"),
        ext_widgets::SelectOption::new("it-IT", "Italiano"),
        ext_widgets::SelectOption::new("pt-BR", "Português"),
        ext_widgets::SelectOption::new("nl-NL", "Nederlands"),
    ]
}

fn localized_label(locale_id: &str) -> &'static str {
    match locale_id {
        "en-US" => "Interface language: English",
        "fr-FR" => "Langue de l'interface : français",
        "de-DE" => "Sprache der Oberfläche: Deutsch",
        "it-IT" => "Lingua dell'interfaccia: italiano",
        "pt-BR" => "Idioma da interface: português",
        "nl-NL" => "Interfacetaal: Nederlands",
        _ => "Idioma de interfaz: español de México",
    }
}

fn menu_bar_menus(autosave: bool, grid: bool) -> Vec<ext_widgets::MenuBarMenu> {
    vec![
        ext_widgets::MenuBarMenu::new("file", "File", menu_items(autosave)),
        ext_widgets::MenuBarMenu::new(
            "edit",
            "Edit",
            vec![
                ext_widgets::MenuItem::command("undo", "Undo").shortcut("Ctrl+Z"),
                ext_widgets::MenuItem::command("redo", "Redo").shortcut("Ctrl+Shift+Z"),
            ],
        ),
        ext_widgets::MenuBarMenu::new(
            "view",
            "View",
            vec![ext_widgets::MenuItem::check("grid", "Grid", grid)],
        ),
    ]
}

fn menu_items(autosave: bool) -> Vec<ext_widgets::MenuItem> {
    vec![
        ext_widgets::MenuItem::command("new", "New").shortcut("Ctrl+N"),
        ext_widgets::MenuItem::command("open", "Open").shortcut("Ctrl+O"),
        ext_widgets::MenuItem::separator(),
        ext_widgets::MenuItem::check("autosave", "Autosave", autosave),
        ext_widgets::MenuItem::submenu(
            "recent",
            "Recent",
            vec![
                ext_widgets::MenuItem::command("recent.one", "demo.rs"),
                ext_widgets::MenuItem::command("recent.two", "notes.md"),
            ],
        ),
        ext_widgets::MenuItem::command("delete", "Delete").destructive(),
        ext_widgets::MenuItem::command("disabled", "Disabled").disabled(),
    ]
}

fn menu_item_top_offset(items: &[ext_widgets::MenuItem], index: usize) -> f32 {
    items
        .iter()
        .take(index)
        .map(|item| menu_item_height(Some(item)))
        .sum()
}

fn menu_item_height(item: Option<&ext_widgets::MenuItem>) -> f32 {
    if item.is_some_and(ext_widgets::MenuItem::is_separator) {
        8.0
    } else {
        28.0
    }
}

fn command_palette_items() -> Vec<ext_widgets::CommandPaletteItem> {
    vec![
        ext_widgets::CommandPaletteItem::new("open", "Open")
            .subtitle("Open a document")
            .shortcut("Ctrl+O")
            .keyword("file"),
        ext_widgets::CommandPaletteItem::new("save", "Save")
            .subtitle("Write current changes")
            .shortcut("Ctrl+S"),
        ext_widgets::CommandPaletteItem::new("format", "Format document")
            .subtitle("Apply source formatting")
            .keyword("code"),
        ext_widgets::CommandPaletteItem::new("rename", "Rename symbol")
            .subtitle("Change every reference")
            .shortcut("F2"),
        ext_widgets::CommandPaletteItem::new("toggle_sidebar", "Toggle sidebar")
            .subtitle("Show or hide the widget panel")
            .shortcut("Ctrl+B"),
        ext_widgets::CommandPaletteItem::new("run", "Run current example")
            .subtitle("Launch showcase")
            .shortcut("Ctrl+R"),
        ext_widgets::CommandPaletteItem::new("focus_canvas", "Focus canvas")
            .subtitle("Move interaction to the canvas window"),
        ext_widgets::CommandPaletteItem::new("reset_layout", "Reset window layout")
            .subtitle("Restore the default showcase positions"),
        ext_widgets::CommandPaletteItem::new("disabled", "Disabled command").disabled(),
    ]
}

fn command_palette_items_with_history(
    history: &ext_widgets::CommandPaletteHistory,
) -> Vec<ext_widgets::CommandPaletteItem> {
    let mut items = command_palette_items()
        .into_iter()
        .map(|item| {
            let command = CommandId::from(item.id.as_str());
            if history.is_recent(&command) {
                item.keyword("recent")
            } else {
                item
            }
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        let left_id = CommandId::from(left.id.as_str());
        let right_id = CommandId::from(right.id.as_str());
        match (
            history.recency_rank(&left_id),
            history.recency_rank(&right_id),
        ) {
            (Some(left_rank), Some(right_rank)) => left_rank.cmp(&right_rank),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.title.cmp(&right.title),
        }
    });
    items
}

fn virtual_table_columns(state: &ShowcaseState) -> Vec<ext_widgets::DataTableColumn> {
    let sort = if state.virtual_table_descending {
        ext_widgets::DataTableSortState::descending()
    } else {
        ext_widgets::DataTableSortState::ascending()
    };
    let filter = if state.virtual_table_ready_only {
        ext_widgets::DataTableFilterState::active("status").with_value("Ready")
    } else {
        ext_widgets::DataTableFilterState::inactive()
    };
    vec![
        ext_widgets::DataTableColumn::new("name", "Name", 220.0)
            .with_sort(sort)
            .sortable("lists_tables.virtualized_table.sort.name"),
        ext_widgets::DataTableColumn::new("status", "Status", 160.0)
            .with_filter(filter)
            .filterable("lists_tables.virtualized_table.filter.status"),
        ext_widgets::DataTableColumn::new("value", "Value", state.virtual_table_value_width)
            .with_min_width(56.0)
            .with_alignment(ext_widgets::DataCellAlignment::End)
            .resize_command("lists_tables.virtualized_table.resize.value"),
    ]
}

fn virtual_table_visible_rows(state: &ShowcaseState) -> Vec<usize> {
    let mut rows = (0..32)
        .filter(|row| !state.virtual_table_ready_only || row % 2 == 0)
        .collect::<Vec<_>>();
    if state.virtual_table_descending {
        rows.reverse();
    }
    rows
}

fn virtual_table_cell_value(source_row: usize, column: usize) -> String {
    match column {
        0 => format!("Virtual row {}", source_row + 1),
        1 if source_row % 2 == 0 => "Ready".to_string(),
        1 => "Pending".to_string(),
        _ => format!("{}%", 30 + source_row * 2),
    }
}

fn editable_tree_default_nodes() -> Vec<EditableTreeNode> {
    vec![EditableTreeNode::new("root", "root").with_children(vec![
        EditableTreeNode::new("child-0", "child #0").with_children(vec![
            EditableTreeNode::new("child-0-0", "child #0"),
            EditableTreeNode::new("child-0-1", "child #1"),
            EditableTreeNode::new("child-0-2", "child #2"),
            EditableTreeNode::new("child-0-3", "child #3")
                .with_children(vec![EditableTreeNode::new("child-0-3-0", "child #0")]),
        ]),
        EditableTreeNode::new("child-1", "child #1").with_children(vec![
            EditableTreeNode::new("child-1-0", "child #0"),
            EditableTreeNode::new("child-1-1", "child #1"),
            EditableTreeNode::new("child-1-2", "child #2"),
        ]),
    ])]
}

fn editable_tree_items(nodes: &[EditableTreeNode]) -> Vec<ext_widgets::TreeItem> {
    nodes
        .iter()
        .map(|node| editable_tree_item(node, true))
        .collect()
}

fn editable_tree_item(node: &EditableTreeNode, root: bool) -> ext_widgets::TreeItem {
    let mut item = ext_widgets::TreeItem::new(node.id.clone(), node.label.clone()).with_children(
        node.children
            .iter()
            .map(|child| editable_tree_item(child, false))
            .collect(),
    );
    if !root {
        item =
            item.with_row_action(ext_widgets::TreeRowAction::new("delete", "delete").destructive());
    }
    item.with_row_action(ext_widgets::TreeRowAction::new("add", "+"))
}

fn find_editable_tree_node_mut<'a>(
    nodes: &'a mut [EditableTreeNode],
    id: &str,
) -> Option<&'a mut EditableTreeNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find_editable_tree_node_mut(&mut node.children, id) {
            return Some(found);
        }
    }
    None
}

fn remove_editable_tree_node(nodes: &mut Vec<EditableTreeNode>, id: &str) -> Option<String> {
    if let Some(index) = nodes.iter().position(|node| node.id == id) {
        return Some(nodes.remove(index).label);
    }
    for node in nodes {
        if let Some(label) = remove_editable_tree_node(&mut node.children, id) {
            return Some(label);
        }
    }
    None
}

fn tree_items() -> Vec<ext_widgets::TreeItem> {
    vec![
        ext_widgets::TreeItem::new("root", "Project").with_children(vec![
            ext_widgets::TreeItem::new("src", "src").with_children(vec![
                ext_widgets::TreeItem::new("lib", "lib.rs"),
                ext_widgets::TreeItem::new("widgets", "widgets.rs"),
            ]),
            ext_widgets::TreeItem::new("assets", "assets").with_children(vec![
                ext_widgets::TreeItem::new("shader", "shader.wgsl"),
                ext_widgets::TreeItem::new("logo", "logo.png"),
            ]),
            ext_widgets::TreeItem::new("target", "target").disabled(),
        ]),
    ]
}

fn virtual_tree_items() -> Vec<ext_widgets::TreeItem> {
    vec![
        ext_widgets::TreeItem::new("root", "Large project").with_children(vec![
            ext_widgets::TreeItem::new("src", "src").with_children(
                (0..32)
                    .map(|index| {
                        ext_widgets::TreeItem::new(
                            format!("src-file-{index:02}"),
                            format!("module_{index:02}.rs"),
                        )
                    })
                    .collect(),
            ),
            ext_widgets::TreeItem::new("examples", "examples").with_children(
                (0..12)
                    .map(|index| {
                        ext_widgets::TreeItem::new(
                            format!("example-file-{index:02}"),
                            format!("demo_{index:02}.rs"),
                        )
                    })
                    .collect(),
            ),
            ext_widgets::TreeItem::new("assets", "assets").with_children(vec![
                ext_widgets::TreeItem::new("icon", "icon.png"),
                ext_widgets::TreeItem::new("shader", "shader.wgsl"),
            ]),
            ext_widgets::TreeItem::new("target", "target").disabled(),
        ]),
    ]
}

fn tree_table_items() -> Vec<ext_widgets::TreeItem> {
    vec![
        ext_widgets::TreeItem::new("root", "Workspace").with_children(vec![
            ext_widgets::TreeItem::new("branch-a", "Interface").with_children(vec![
                ext_widgets::TreeItem::new("widgets", "widgets.rs"),
                ext_widgets::TreeItem::new("layout", "layout.rs"),
            ]),
            ext_widgets::TreeItem::new("branch-b", "Renderer").with_children(vec![
                ext_widgets::TreeItem::new("wgpu", "wgpu.rs"),
                ext_widgets::TreeItem::new("paint", "paint.rs").disabled(),
            ]),
            ext_widgets::TreeItem::new("docs", "docs"),
        ]),
    ]
}

fn parse_calendar_date(value: &str) -> Option<CalendarDate> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    CalendarDate::new(year, month, day)
}

fn parse_table_cell(value: &str) -> Option<ext_widgets::DataTableCellIndex> {
    let mut parts = value.split('.');
    let row = parts.next()?.parse().ok()?;
    let column = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(ext_widgets::DataTableCellIndex::new(row, column))
}

fn unit(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn smooth_loop(phase: f32, offset: f32) -> f32 {
    0.5 - ((phase + offset).cos() * 0.5)
}

fn profile_form_state() -> FormState {
    let mut form = FormState::new("profile")
        .with_field("name", "Operad")
        .with_field("email", "ada@example.com")
        .with_field("role", "Designer")
        .with_field("newsletter", "true");
    let _ = form.update_field("email", "invalid@example");
    let request = form.begin_form_validation();
    let _ = form.apply_form_validation(
        FormValidationResult::new(request.generation)
            .with_field_messages(
                "email",
                vec![ValidationMessage::error("Use a complete email address")],
            )
            .with_form_message(ValidationMessage::warning("Unsaved profile changes")),
    );
    form
}

fn profile_form_value(form: &FormState, id: &str) -> String {
    form.fields
        .iter()
        .find_map(|(field_id, field)| (field_id.as_str() == id).then(|| field.value.clone()))
        .unwrap_or_default()
}

fn scaled_slider(rect: UiRect, point: UiPoint, min: f32, max: f32) -> f32 {
    min + unit(widgets::slider::slider_value_from_control_point(
        rect,
        point,
        0.0..1.0,
    )) * (max - min)
}

fn resize_split_from_pointer(
    state: &mut ext_widgets::SplitPaneState,
    axis: ext_widgets::SplitAxis,
    edit: WidgetPointerEdit,
    handle_thickness: f32,
) -> bool {
    let total_extent = match axis {
        ext_widgets::SplitAxis::Horizontal => edit.target_rect.width,
        ext_widgets::SplitAxis::Vertical => edit.target_rect.height,
    }
    .max(1.0);
    let sizes = state.resolved_sizes(total_extent, handle_thickness);
    let handle_center = match axis {
        ext_widgets::SplitAxis::Horizontal => edit.target_rect.x + sizes.first + sizes.handle * 0.5,
        ext_widgets::SplitAxis::Vertical => edit.target_rect.y + sizes.first + sizes.handle * 0.5,
    };
    let pointer = match axis {
        ext_widgets::SplitAxis::Horizontal => edit.position.x,
        ext_widgets::SplitAxis::Vertical => edit.position.y,
    };
    state.resize_by(pointer - handle_center, total_extent, handle_thickness)
}

fn scroll_state(offset_y: f32, viewport_height: f32, content_height: f32) -> operad::ScrollState {
    operad::ScrollState::new(ScrollAxes::VERTICAL)
        .with_sizes(
            UiSize::new(8.0, viewport_height),
            UiSize::new(8.0, content_height),
        )
        .with_offset(UiPoint::new(0.0, offset_y))
}

fn timeline_scroll_state_for_view(
    saved: operad::ScrollState,
    viewport_width: f32,
) -> operad::ScrollState {
    let viewport_width = if viewport_width > f32::EPSILON {
        viewport_width
    } else if saved.viewport_size().width > f32::EPSILON {
        saved.viewport_size().width
    } else {
        620.0
    };
    operad::ScrollState::new(ScrollAxes::HORIZONTAL)
        .with_sizes(
            UiSize::new(viewport_width, TIMELINE_VIEWPORT_HEIGHT),
            UiSize::new(TIMELINE_CONTENT_WIDTH, TIMELINE_VIEWPORT_HEIGHT),
        )
        .with_offset(saved.offset())
}

fn controls_list_viewport_height(viewport_height: f32) -> f32 {
    (viewport_height - 110.0).max(120.0)
}

fn controls_scroll_state_for_view(
    saved: operad::ScrollState,
    viewport_height: f32,
) -> operad::ScrollState {
    let viewport_height = if saved.viewport_size().height > f32::EPSILON {
        saved.viewport_size().height
    } else {
        viewport_height
    };
    let content_height = if saved.content_size().height > f32::EPSILON {
        saved.content_size().height
    } else {
        controls_list_content_height()
    };
    scroll_state(saved.offset().y, viewport_height, content_height)
}

fn controls_list_content_height() -> f32 {
    SHOWCASE_WIDGET_WINDOW_IDS.len() as f32 * CONTROLS_WIDGET_ROW_HEIGHT
        + (SHOWCASE_WIDGET_WINDOW_IDS.len().saturating_sub(1)) as f32 * CONTROLS_WIDGET_ROW_GAP
}

fn caret_visible(phase: f32) -> bool {
    phase.sin() >= 0.0
}

fn showcase_text_color(color: ColorRgba) -> ColorRgba {
    if active_showcase_theme_choice() == ShowcaseThemeChoice::Dark || color.a == 0 {
        return color;
    }

    let max = color.r.max(color.g).max(color.b);
    let min = color.r.min(color.g).min(color.b);
    if max.saturating_sub(min) > 36 {
        return color;
    }

    let brightness = (u16::from(color.r) + u16::from(color.g) + u16::from(color.b)) / 3;
    let mut mapped = if brightness >= 215 {
        active_showcase_colors().text
    } else if brightness >= 170 {
        active_showcase_colors().text_muted
    } else if brightness >= 110 {
        active_showcase_colors().text_subtle
    } else {
        return color;
    };
    mapped.a = color.a;
    mapped
}

fn text(size: f32, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size: size,
        line_height: size + 5.0,
        color: showcase_text_color(color),
        ..Default::default()
    }
}

fn themed_text(theme: &Theme, size: f32) -> TextStyle {
    text(size, theme.colors.text)
}

fn themed_muted_text(theme: &Theme, size: f32) -> TextStyle {
    text(size, theme.colors.text_muted)
}

fn color(r: u8, g: u8, b: u8) -> ColorRgba {
    ColorRgba::new(r, g, b, 255)
}
