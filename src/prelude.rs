//! Common Operad imports for application code.
//!
//! The prelude intentionally stays small. It contains the document, geometry,
//! styling, input, action, paint, and common widget entry points most app code
//! needs without pulling backend adapters into backend-neutral builds. Backend
//! services, renderer diagnostics, layout escape hatches, raw scroll, manual
//! scrollbar, and absolute-position helpers stay available from their modules
//! instead of becoming the default path.

pub use crate::{
    length, root_style, AccessibilityAction, AccessibilityChecked, AccessibilityLiveRegion,
    AccessibilityMeta, AccessibilityRole, AccessibilityValueRange, AnimatedValues,
    AnimationBlendBinding, AnimationCondition, AnimationInputValue, AnimationMachine,
    AnimationNumberComparison, AnimationState, AnimationTickOutcome, AnimationTickReport,
    AnimationTransition, AnimationTrigger, CanvasContent, CanvasContextDescriptor,
    CanvasContextKind, CanvasInteractionPolicy, CanvasRenderMode, ClipBehavior, ClipScope,
    ColorRgba, ComputedLayout, DragDropSurfaceKind, DropPayloadFilter, EditPhase, ElementMaterial,
    ElementShape, FieldId, FieldState, FocusDirection, FocusNavigationDirection,
    FocusRestoreTarget, FontFamily, FontStretch, FontStyle, FontWeight, FormId, FormState,
    GeometryEffect, ImageContent, InputBehavior, InteractionVisuals, KeyCode, KeyModifiers, Layout,
    LayoutAlignment, LayoutDimension, LayoutDisplay, LayoutFlexDirection, LayoutFlexWrap,
    LayoutGap, LayoutGridTrack, LayoutInset, LayoutInsets, LayoutJustifyContent, LayoutLength,
    LayoutPosition, LayoutSize, LayoutSpacing, LayoutStyle, PaintItem, PaintKind, PaintList,
    PaintTransform, ScenePrimitive, ScrollAxes, ScrollState, StrokeStyle, TextContent, TextStyle,
    TextWrap, UiContent, UiDocument, UiDocumentScale, UiFocusState, UiInputEvent, UiInputResult,
    UiNode, UiNodeId, UiNodeStyle, UiPoint, UiPortalId, UiPortalTarget, UiRect, UiSize, UiVisual,
    UiWheelEvent, ValidationMessage, ValidationSeverity, WidgetAction, WidgetActionBinding,
    WidgetActionId, WidgetActionKind, WidgetActionMode, WidgetActionQueue, WidgetActionTrigger,
    ANIMATION_INPUT_ACTIVATED, ANIMATION_INPUT_ACTIVE, ANIMATION_INPUT_FOCUSED,
    ANIMATION_INPUT_HOVER, ANIMATION_INPUT_POINTER_NORM_X, ANIMATION_INPUT_POINTER_NORM_Y,
    ANIMATION_INPUT_POINTER_X, ANIMATION_INPUT_POINTER_Y, ANIMATION_INPUT_PRESSED,
    APP_OVERLAY_PORTAL,
};

pub use crate::layout;
pub use crate::scrolling::ScrollbarVisibility;

#[cfg(feature = "widgets")]
pub use crate::widgets::ext::{
    color_edit_button, color_picker, color_swatch_button, compact_color_button, image_menu_button,
    image_text_menu_button, menu_button, show_color, submenu, submenu_button, ColorButtonNodes,
    ColorButtonOptions, ColorHsv, ColorOklch, ColorPalette, ColorPickerMode, ColorPickerNodes,
    ColorPickerOptions, ColorPickerState, ColorPickerStyle, ColorSwatch, ColorValueFormat,
    MenuButtonAnchors, MenuButtonNodes, MenuButtonOptions, MenuButtonOutcome, MenuButtonState,
    MenuItem, MenuItemKind, MenuNavigationState, MenuSelection, MenuSubmenuAnchor, PopupAlign,
    PopupPlacement, PopupSide,
};
#[cfg(feature = "widgets")]
pub use crate::widgets::{
    add_enabled, add_enabled_ui, add_visible, add_visible_ui, bottom_panel, button, canvas,
    central_panel, checkbox, checkbox_with_state, code_editor, code_label, code_text_style,
    collapsing_header, colored_label, colored_text_style, columns, combo_box, dnd_drag_source,
    dnd_drop_zone, drag_angle, drag_angle_tau, drag_value_input, field_help_text, field_label,
    field_validation_message, form_action_buttons, form_error_summary, form_field_order,
    form_has_errors, form_row, form_section, frame, group, group_panel, heading_label,
    heading_text_style, hyperlink, icon_button, image, image_button, indented_section, label,
    left_panel, link, localized_label, modal_dialog, monospace_label, monospace_text_style,
    multiline_text_input, next_form_field, panel, password_input, previous_form_field,
    radio_button, radio_group, reset_button, resize_container, right_panel, scene, scroll_area,
    scroll_container, scroll_to_cursor, scroll_to_rect, scroll_to_rect_with_options, search_input,
    selectable_label, selectable_value, separator, set_subtree_enabled, set_subtree_visible,
    side_panel, sides, singleline_text_input, slider, small_button, small_label, small_text_style,
    spacer, spinner, strong_label, strong_text_style, table_header, text_area, text_input,
    theme_preference_buttons, theme_preference_switch, toggle_button, toggle_switch, tooltip_box,
    top_panel, validation_text_style, virtual_list, weak_label, weak_text_style, wrapped_label,
    CheckboxState, CollapsingHeaderNodes, CollapsingHeaderOptions, ColumnsNodes, ColumnsOptions,
    DialogDismissReason, DialogDismissal, DragImagePolicy, DragSourceNodes, DragSourceOptions,
    DragValueOptions, DropZoneNodes, DropZoneOptions, FieldHelpOptions, FieldLabelOptions,
    FormActionAvailability, FormActionButtonNodes, FormActionButtonsOptions, FormActionKind,
    FormActionLabels, FormErrorSummaryNodes, FormErrorSummaryOptions, FormRowOptions,
    FormSectionNodes, FormSectionOptions, FrameOptions, ImageOptions, IndentOptions, LinkOptions,
    ModalDialogNodes, ModalDialogOptions, NumericPrecision, NumericRange, NumericUnitFormat,
    PanelKind, PanelOptions, RadioButtonOptions, RadioGroupOptions, RadioOption,
    ResizeContainerNodes, ResizeContainerOptions, SceneOptions, ScrollContainerNodes,
    ScrollContainerOptions, SelectableLabelOptions, SeparatorOptions, SeparatorOrientation,
    SidePanelSide, SidesNodes, SidesOptions, SpinnerOptions, ThemePreference,
    ThemePreferenceButtonNodes, ThemePreferenceButtonsOptions, ThemePreferenceLabels,
    ThemePreferenceSwitchOptions, ToggleSwitchOptions, ToggleValue, TooltipBoxOptions,
    TooltipTriggerMode, TooltipTriggerOptions, ValidationMessageOptions, VirtualListSpec,
};
