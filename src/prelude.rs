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
    DropTargetDescriptor, DropTargetHit, DropTargetId, EditPhase, FieldId, FieldState,
    FocusDirection, FontFamily, FontStretch, FontStyle, FontWeight, FormId, FormState,
    ImageContent, InputBehavior, InteractionVisuals, KeyCode, KeyModifiers, LayoutStyle, PaintItem,
    PaintKind, PaintList, PaintTransform, ScenePrimitive, ScrollAxes, ScrollState, ShaderEffect,
    ShaderUniform, StrokeStyle, TextContent, TextStyle, TextWrap, UiContent, UiDocument,
    UiDocumentScale, UiFocusState, UiInputEvent, UiInputResult, UiNode, UiNodeId, UiNodeStyle,
    UiPoint, UiRect, UiSize, UiVisual, UiWheelEvent, ValidationMessage, ValidationSeverity,
    WidgetAction, WidgetActionBinding, WidgetActionId, WidgetActionKind, WidgetActionMode,
    WidgetActionQueue, WidgetActionTrigger,
};

pub use crate::layout;
pub use crate::platform::{DragBytes, DragDropRequest, DragOperation, DragPayload};

#[cfg(feature = "widgets")]
pub use crate::widgets::{
    bottom_panel, button, canvas, central_panel, checkbox, code_editor, code_label,
    code_text_style, collapsing_header, color_edit_button_hsva, color_edit_button_oklch,
    color_edit_button_rgb, color_edit_button_rgba, color_edit_button_srgb, color_edit_button_srgba,
    color_picker, color_swatch_button, colored_label, colored_text_style, columns, combo_box,
    compact_color_button, dnd_drag_source, dnd_drag_source_actions_from_gesture_event,
    dnd_drag_source_descriptor, dnd_drag_start_request, dnd_drop_target_descriptor, dnd_drop_zone,
    dnd_drop_zone_actions_from_gesture_event, drag_value_input, field_help_text, field_label,
    field_validation_message, form_error_summary, form_row, form_section, frame, grid, grid_row,
    grid_text_cell, group, group_panel, heading_label, heading_text_style, hyperlink, icon_button,
    image, image_button, indented_section, label, left_panel, link, localized_label, modal_dialog,
    monospace_label, monospace_text_style, multiline_text_input, panel, password_input,
    radio_button, radio_group, reset_button, resize_container, resize_handle, right_panel,
    scroll_area, scrollbar, search_input, selectable_label, separator, show_color, show_color_at,
    side_panel, sides, singleline_text_input, slider, small_button, small_label, small_text_style,
    spacer, spinner, strong_label, strong_text_style, table_header, text_area, text_input,
    toggle_button, toggle_switch, tooltip_box, top_panel, validation_text_style, virtual_list,
    weak_label, weak_text_style, wrapped_label, CollapsingHeaderNodes, CollapsingHeaderOptions,
    ColorButtonNodes, ColorButtonOptions, ColorHsv, ColorOklch, ColorPalette, ColorPickerMode,
    ColorPickerNodes, ColorPickerOptions, ColorPickerState, ColorPickerStyle, ColorSwatch,
    ColorValueFormat, ColumnsNodes, ColumnsOptions, DragSourceNodes, DragSourceOptions,
    DragValueOptions, DropZoneNodes, DropZoneOptions, FieldHelpOptions, FieldLabelOptions,
    FormErrorSummaryNodes, FormErrorSummaryOptions, FormRowOptions, FormSectionNodes,
    FormSectionOptions, FrameOptions, GridCellOptions, GridOptions, GridRowOptions, ImageOptions,
    IndentOptions, LinkOptions, ModalDialogNodes, ModalDialogOptions, NumericPrecision,
    NumericRange, NumericUnitFormat, PanelKind, PanelOptions, RadioButtonOptions,
    RadioGroupOptions, RadioOption, ResizeContainerNodes, ResizeContainerOptions,
    ResizeHandleOptions, ResizeHandlePlacement, SelectableLabelOptions, SeparatorOptions,
    SeparatorOrientation, SidePanelSide, SidesNodes, SidesOptions, SpinnerOptions,
    ToggleSwitchOptions, ToggleValue, TooltipBoxOptions, ValidationMessageOptions,
};

#[cfg(feature = "native-window")]
pub use crate::{
    run_app, run_app_with, run_app_with_canvas_renderers, run_ui_document, run_ui_document_with,
    run_ui_document_with_canvas_renderers, NativeWgpuCanvasRenderRegistry, NativeWindowOptions,
    NativeWindowResult,
};
