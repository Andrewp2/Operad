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
    ComputedLayout, EditPhase, FocusDirection, FontFamily, FontStretch, FontStyle, FontWeight,
    ImageContent, InputBehavior, InteractionVisuals, KeyCode, KeyModifiers, LayoutStyle, PaintItem,
    PaintKind, PaintList, PaintTransform, ScenePrimitive, ScrollAxes, ScrollState, ShaderEffect,
    ShaderUniform, StrokeStyle, TextContent, TextStyle, TextWrap, UiContent, UiDocument,
    UiDocumentScale, UiFocusState, UiInputEvent, UiInputResult, UiNode, UiNodeId, UiNodeStyle,
    UiPoint, UiRect, UiSize, UiVisual, UiWheelEvent, WidgetAction, WidgetActionBinding,
    WidgetActionId, WidgetActionKind, WidgetActionMode, WidgetActionQueue, WidgetActionTrigger,
};

pub use crate::layout;

#[cfg(feature = "widgets")]
pub use crate::widgets::{
    button, canvas, checkbox, combo_box, label, localized_label, scroll_area, scrollbar, slider,
    table_header, text_input, virtual_list,
};

#[cfg(feature = "native-window")]
pub use crate::{
    run_app, run_app_with, run_app_with_canvas_renderers, run_ui_document, run_ui_document_with,
    run_ui_document_with_canvas_renderers, NativeWgpuCanvasRenderRegistry, NativeWindowOptions,
    NativeWindowResult,
};
