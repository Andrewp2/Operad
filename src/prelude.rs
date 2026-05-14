//! Common Operad imports for application code.
//!
//! The prelude intentionally stays small. It contains the document, geometry,
//! styling, input, action, paint, and common widget entry points most examples
//! need without pulling backend adapters into backend-neutral builds.

pub use crate::{
    length, root_style, AccessibilityAction, AccessibilityChecked, AccessibilityLiveRegion,
    AccessibilityMeta, AccessibilityRole, AccessibilityValueRange, AnimationMachine,
    AnimationState, AnimationTransition, AnimationTrigger, CanvasContent, CanvasContextDescriptor,
    CanvasContextKind, CanvasInteractionPolicy, CanvasRenderMode, ClipBehavior, ColorRgba,
    ComputedLayout, DragDropSurfaceKind, DragSourceDescriptor, DragSourceId, DropPayloadFilter,
    DropTargetDescriptor, DropTargetHit, DropTargetId, EditPhase, FocusDirection, FontFamily,
    FontStretch, FontStyle, FontWeight, ImageContent, InputBehavior, InteractionVisuals, KeyCode,
    KeyModifiers, LayoutStyle, PaintItem, PaintKind, PaintList, PaintTransform, ScenePrimitive,
    ScrollAxes, ScrollState, ShaderEffect, ShaderUniform, StrokeStyle, TextContent, TextStyle,
    TextWrap, UiContent, UiDocument, UiDocumentScale, UiFocusState, UiInputEvent, UiInputResult,
    UiNode, UiNodeId, UiNodeStyle, UiPoint, UiRect, UiSize, UiVisual, UiWheelEvent, WidgetAction,
    WidgetActionBinding, WidgetActionId, WidgetActionKind, WidgetActionMode, WidgetActionQueue,
    WidgetActionTrigger,
};

pub use crate::layout;
pub use crate::platform::{DragBytes, DragDropRequest, DragOperation, DragPayload};

#[cfg(feature = "widgets")]
pub use crate::widgets::{
    button, canvas, checkbox, code_editor, code_label, code_text_style, collapsing_header,
    colored_label, colored_text_style, combo_box, dnd_drag_source,
    dnd_drag_source_actions_from_gesture_event, dnd_drag_source_descriptor, dnd_drag_start_request,
    dnd_drop_target_descriptor, dnd_drop_zone, dnd_drop_zone_actions_from_gesture_event,
    drag_value_input, grid, grid_row, grid_text_cell, heading_label, heading_text_style, hyperlink,
    image, label, link, localized_label, modal_dialog, monospace_label, monospace_text_style,
    multiline_text_input, panel, password_input, radio_button, radio_group, scroll_area, scrollbar,
    search_input, selectable_label, separator, singleline_text_input, slider, small_label,
    small_text_style, spacer, spinner, strong_label, strong_text_style, table_header, text_area,
    text_input, toggle_switch, tooltip_box, virtual_list, weak_label, weak_text_style,
    wrapped_label, CollapsingHeaderNodes, CollapsingHeaderOptions, DragSourceNodes,
    DragSourceOptions, DragValueOptions, DropZoneNodes, DropZoneOptions, GridCellOptions,
    GridOptions, GridRowOptions, ImageOptions, LinkOptions, ModalDialogNodes, ModalDialogOptions,
    NumericPrecision, NumericRange, NumericUnitFormat, PanelKind, PanelOptions, RadioButtonOptions,
    RadioGroupOptions, RadioOption, SelectableLabelOptions, SeparatorOptions, SeparatorOrientation,
    SpinnerOptions, ToggleSwitchOptions, ToggleValue, TooltipBoxOptions,
};

#[cfg(feature = "native-window")]
pub use crate::{
    run_app, run_app_with, run_app_with_canvas_renderers, run_ui_document, run_ui_document_with,
    run_ui_document_with_canvas_renderers, NativeWgpuCanvasRenderRegistry, NativeWindowOptions,
    NativeWindowResult,
};
