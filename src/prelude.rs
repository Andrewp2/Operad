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
pub use crate::platform::{DragBytes, DragDropRequest, DragImage, DragOperation, DragPayload};
pub use crate::scrolling::{ProgrammaticScrollBehavior, RevealOptions, RevealScrollPlan};

#[cfg(feature = "widgets")]
pub use crate::widgets::{
    add_enabled, add_enabled_ui, add_sized, add_visible, add_visible_ui, aligned_scrollbar_options,
    allocate_at_least, allocate_exact_size, allocate_painter, area, bottom_panel, button, canvas,
    central_panel, checkbox, code_editor, code_label, code_text_style, collapsing_header,
    color_edit_button_hsva, color_edit_button_oklch, color_edit_button_rgb, color_edit_button_rgba,
    color_edit_button_rgba_premultiplied, color_edit_button_rgba_unmultiplied,
    color_edit_button_srgb, color_edit_button_srgba, color_edit_button_srgba_premultiplied,
    color_edit_button_srgba_unmultiplied, color_picker, color_picker_color32, color_picker_hsva_2d,
    color_swatch_button, colored_label, colored_text_style, columns, combo_box,
    compact_color_button, dnd_apply_drop_zone_preview, dnd_drag_source,
    dnd_drag_source_actions_from_gesture_event, dnd_drag_source_descriptor, dnd_drag_start_request,
    dnd_drop_target_descriptor, dnd_drop_zone, dnd_drop_zone_actions_from_gesture_event,
    dnd_drop_zone_preview_state, drag_angle, drag_angle_tau, drag_value_input, field_help_text,
    field_label, field_validation_message, form_action_buttons, form_error_summary,
    form_field_order, form_has_errors, form_row, form_section, frame, grid, grid_row,
    grid_text_cell, group, group_panel, heading_label, heading_text_style, hyperlink, icon_button,
    image, image_button, image_menu_button, image_text_menu_button, indented_section, label,
    left_panel, link, localized_label, menu_button, modal_dialog,
    modal_dialog_close_actions_from_input_result, modal_dialog_descriptor,
    modal_dialog_dismiss_event_from_input_result, modal_dialog_dismiss_event_from_key_event,
    modal_dialog_dismiss_event_from_pointer_event, modal_dialog_focus_trap,
    modal_dialog_open_event, monospace_label, monospace_text_style, multiline_text_input,
    next_form_field, panel, password_input, previous_form_field, process_overlay_frame,
    radio_button, radio_group, reset_button, resize_container, resize_handle, right_panel, scene,
    scroll_area, scroll_area_with_bars, scroll_to_cursor, scroll_to_rect,
    scroll_to_rect_with_options, scrollbar, search_input, selectable_label, selectable_value,
    separator, set_subtree_enabled, set_subtree_visible, show_color, show_color_at, side_panel,
    sides, singleline_text_input, slider, small_button, small_label, small_text_style, spacer,
    spinner, strong_label, strong_text_style, submenu, submenu_button, table_header, text_area,
    text_input, theme_preference_buttons, theme_preference_switch, toggle_button, toggle_switch,
    tooltip_box, tooltip_box_from_request, tooltip_fade_slide_animation, tooltip_rect,
    tooltip_trigger_resolution, top_panel, validation_text_style, virtual_list, weak_label,
    weak_text_style, wrapped_label, AllocationOptions, AreaNodes, AreaOptions,
    CollapsingHeaderNodes, CollapsingHeaderOptions, ColorButtonNodes, ColorButtonOptions, ColorHsv,
    ColorHsva2dNodes, ColorHsva2dOptions, ColorOklch, ColorPalette, ColorPickerMode,
    ColorPickerNodes, ColorPickerOptions, ColorPickerState, ColorPickerStyle, ColorSwatch,
    ColorValueFormat, ColumnsNodes, ColumnsOptions, DialogDescriptor, DialogDismissReason,
    DialogDismissal, DragImagePolicy, DragSourceNodes, DragSourceOptions, DragValueOptions,
    DropZoneNodes, DropZoneOptions, DropZonePreviewState, FieldHelpOptions, FieldLabelOptions,
    FormActionAvailability, FormActionButtonNodes, FormActionButtonsOptions, FormActionKind,
    FormActionLabels, FormErrorSummaryNodes, FormErrorSummaryOptions, FormRowOptions,
    FormSectionNodes, FormSectionOptions, FrameOptions, GridCellOptions, GridOptions,
    GridRowOptions, ImageOptions, IndentOptions, LinkOptions, MenuButtonAnchors, MenuButtonNodes,
    MenuButtonOptions, MenuButtonOutcome, MenuButtonState, MenuItem, MenuItemKind, MenuListNodes,
    MenuListOptions, MenuNavigationState, MenuSelection, MenuSubmenuAnchor, ModalDialogNodes,
    ModalDialogOptions, NumericPrecision, NumericRange, NumericUnitFormat, OverlayFrameEvent,
    OverlayFrameOutput, OverlayFrameRequest, OverlayFrameState, PanelKind, PanelOptions,
    PopupAlign, PopupPlacement, PopupSide, RadioButtonOptions, RadioGroupOptions, RadioOption,
    ResizeContainerNodes, ResizeContainerOptions, ResizeHandleOptions, ResizeHandlePlacement,
    SceneOptions, ScrollAreaWithBarsNodes, ScrollAreaWithBarsOptions, SelectableLabelOptions,
    SeparatorOptions, SeparatorOrientation, SidePanelSide, SidesNodes, SidesOptions,
    SpinnerOptions, ThemePreference, ThemePreferenceButtonNodes, ThemePreferenceButtonsOptions,
    ThemePreferenceLabels, ThemePreferenceSwitchOptions, ToggleSwitchOptions, ToggleValue,
    TooltipBoxOptions, TooltipTriggerMode, TooltipTriggerOptions, ValidationMessageOptions,
    TOOLTIP_HIDE_TRIGGER, TOOLTIP_SHOW_TRIGGER,
};

#[cfg(feature = "native-window")]
pub use crate::{
    run_app, run_app_with, run_app_with_canvas_renderers, run_app_with_canvas_renderers_and_hooks,
    run_ui_document, run_ui_document_with, run_ui_document_with_canvas_renderers,
    NativeKeyboardInput, NativeWgpuCanvasRenderRegistry, NativeWindowHooks, NativeWindowMetrics,
    NativeWindowOptions, NativeWindowResult,
};
