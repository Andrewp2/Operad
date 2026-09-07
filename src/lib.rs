//! Operad is a retained UI document library with a shared runtime.
//!
//! Documents, widgets, layout, text measurement, input, and animation are
//! renderer-neutral. Platform adapters, diagnostics, and inspector UI are optional.
//! Product-specific screens and game/application state should live in the
//! consuming application crate.

pub mod accessibility;
#[cfg(feature = "accesskit-winit")]
pub use adapters::accesskit_winit_adapter;
pub use interaction::actions;
pub mod adapters;
pub use domain::charts;
pub use interaction::commands;
pub use render::assets;
pub use render::compositor;
pub mod core;
#[cfg(feature = "text-cosmic")]
pub use core::document::CosmicTextMeasurer;
pub use core::document::{
    length, root_style, AccessibilityAction, AccessibilityChecked, AccessibilityLiveRegion,
    AccessibilityMeta, AccessibilityNode, AccessibilityRelationKind, AccessibilityRelations,
    AccessibilityRole, AccessibilitySortDirection, AccessibilityStateKind, AccessibilitySummary,
    AccessibilitySummaryItem, AccessibilityTree, AccessibilityValueRange,
    AccessibilityValueRangeIssue, AnimatedValues, AnimationActiveTransitionSnapshot,
    AnimationBlendBinding, AnimationCondition, AnimationInputValue, AnimationMachine,
    AnimationNumberComparison, AnimationState, AnimationTickOutcome, AnimationTickReport,
    AnimationTransition, AnimationTrigger, ApproxTextMeasurer, AvailableSize, CanvasContent,
    CanvasContextDescriptor, CanvasContextKind, CanvasInteractionPolicy, CanvasRenderMode,
    CanvasRenderProgram, CanvasShaderConstant, ClipBehavior, ClipScope, ColorRgba, ComputedLayout,
    EditPhase, ElementMaterial, ElementShape, FocusDirection, FontFamily, FontStretch, FontStyle,
    FontWeight, GeometryEffect, ImageContent, InputBehavior, InteractionVisuals, IntrinsicSize,
    KeyCode, KeyModifiers, KnownSize, LayoutSnapshot, LayoutStyle, PaintCompositorLayer, PaintItem,
    PaintKind, PaintList, PaintTransform, ScenePrimitive, ScrollAxes, ScrollState, ShaderEffect,
    ShaderUniform, StrokeStyle, TextContent, TextInteractionStyles, TextMeasurer, TextStyle,
    TextWrap, UiContent, UiDocument, UiDocumentScale, UiFocusState, UiInputEvent, UiInputResult,
    UiNode, UiNodeId, UiNodeLayoutConstraint, UiNodeStyle, UiPoint, UiPortalId, UiPortalTarget,
    UiRect, UiSize, UiVisual, UiWheelEvent, ANIMATION_INPUT_ACTIVATED, ANIMATION_INPUT_ACTIVE,
    ANIMATION_INPUT_FOCUSED, ANIMATION_INPUT_HOVER, ANIMATION_INPUT_POINTER_NORM_X,
    ANIMATION_INPUT_POINTER_NORM_Y, ANIMATION_INPUT_POINTER_X, ANIMATION_INPUT_POINTER_Y,
    ANIMATION_INPUT_PRESSED, APP_OVERLAY_PORTAL,
};
#[cfg(any(test, feature = "diagnostics"))]
pub use diagnostics::debug;
pub mod diagnostics;
pub use render::display;
pub mod domain;
pub use core::i18n;
pub use diagnostics::errors;
pub use domain::editor;
pub use interaction::drag_drop;
pub use interaction::forms;
pub use interaction::input;
pub use interaction::input_devices;
pub use render::effective_geometry;
pub use render::fonts;
pub use runtime::host;
pub mod interaction;
pub use core::layout;
pub use diagnostics::limits;
pub use interaction::navigation;
pub use interaction::overlays;
pub use render::layout_animation;
pub use render::paint;
pub use runtime::platform;
pub mod prelude;
pub mod render;
pub use render::renderer;
pub use render::resource_cache;
pub mod runtime;
pub use render::scrolling;
#[cfg(all(feature = "web-runtime", target_arch = "wasm32"))]
pub use runtime::web;
pub mod shell;
pub use core::state;
#[cfg(any(test, feature = "test-support"))]
pub use diagnostics::testing;
pub use interaction::tasks;
pub mod theme;

pub use accessibility::tooltips;
#[cfg(feature = "wgpu")]
pub use adapters::wgpu_renderer;
pub use core::versioning;
pub use interaction::transactions;
pub use render::virtualization;
#[cfg(feature = "widgets")]
pub mod widgets;
pub use runtime::windows;

pub use accessibility::{FocusNavigationDirection, FocusRestoreTarget, FocusTrap};
pub use actions::{
    WidgetAction, WidgetActionBinding, WidgetActionId, WidgetActionKind, WidgetActionMode,
    WidgetActionQueue, WidgetActionTrigger, WidgetActivation, WidgetActivationSource, WidgetDrag,
    WidgetDragPhase, WidgetFocusChange, WidgetKeyboardActivation, WidgetPointerActivation,
    WidgetPointerEdit, WidgetSelection, WidgetTextEdit, WidgetValueEditPhase,
};
pub use assets::BuiltInIcon;
pub use commands::{
    Command, CommandEffect, CommandId, CommandMeta, CommandRegistry, CommandScope, Shortcut,
};
pub use core::invalidation::DirtyFlags;
pub use core::timing::{FrameTiming, FrameTimingSection, FrameTimingSectionSummary};
pub use drag_drop::{DragDropSurfaceKind, DropPayloadFilter};
pub use forms::{FieldId, FieldState, FormId, FormState, ValidationMessage, ValidationSeverity};
pub use i18n::{
    BidiPolicy, DynamicLabelMeta, LabelUpdatePolicy, LayoutMirrorMode, LocaleId,
    LocaleIdentifierError, LocalizationPolicy, ResolvedTextDirection, TextDirection,
};
pub use input::{
    DragGesture, GestureEvent, GesturePhase, PointerButton, PointerButtons, PointerClick,
    PointerEventKind, PointerId, PointerKind,
};
pub use layout::{
    ContainedFlowLayout, Layout, LayoutAlignment, LayoutDimension, LayoutDisplay,
    LayoutFlexDirection, LayoutFlexWrap, LayoutGap, LayoutGridTrack, LayoutInset, LayoutInsets,
    LayoutJustifyContent, LayoutLength, LayoutPosition, LayoutSize, LayoutSpacing,
};
pub use overlays::{
    OverlayDismissPolicy, OverlayDismissReason, OverlayEntry, OverlayFocusRestoreTarget, OverlayId,
    OverlayKind, OverlayStack,
};
pub use paint::{
    AlignedStroke, CornerRadii, GradientStop, ImageAlignment, ImageFit, LinearGradient, PaintBrush,
    PaintEffect, PaintEffectKind, PaintImage, PaintPath, PaintRect, PaintText, PathFillRule,
    PathStrokeOptions, PathVerb, PixelSnapPolicy, StrokeAlignment, StrokeLineCap, StrokeLineJoin,
    TextHorizontalAlign, TextOverflow, TextVerticalAlign,
};
#[cfg(feature = "native-window")]
pub use runtime::native;
pub use theme::{
    color_with_alpha, text_style_with_color, text_style_with_scale, ColorTokens,
    ComponentIconStates, ComponentLayoutTokens, ComponentRole, ComponentState, ComponentStateSlot,
    ComponentStyle, ComponentTextStates, ComponentTokens, ComponentVisualStates, EffectTokens,
    IconStyle, LayerEffect, LayerEffectKind, MotionCurve, MotionTokens, OpacityTokens,
    RadiusTokens, ScopedThemeRegistry, SpacingTokens, StrokeTokens, Theme, ThemePatch, ThemeScope,
    ThemeScopeError, ThemeScopeId, ThemeScopeKind, TypographyTokens, OPERAD_BUBBLEGUM_THEME_NAME,
    OPERAD_DARK_THEME_NAME, OPERAD_LIGHT_THEME_NAME,
};
