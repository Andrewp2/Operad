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
#[path = "render/layout_animation.rs"]
pub mod layout_animation;
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
#[cfg(all(feature = "web-runtime", target_arch = "wasm32"))]
pub use runtime::web;
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
pub mod widgets;
#[path = "runtime/windows.rs"]
pub mod windows;

pub use accessibility::{FocusNavigationDirection, FocusRestoreTarget, FocusTrap};
pub use actions::{
    WidgetAction, WidgetActionBinding, WidgetActionId, WidgetActionKind, WidgetActionMode,
    WidgetActionQueue, WidgetActionTrigger, WidgetActivation, WidgetDrag, WidgetDragPhase,
    WidgetFocusChange, WidgetPointerEdit, WidgetSelection, WidgetTextEdit, WidgetValueEditPhase,
};
pub use assets::BuiltInIcon;
pub use commands::{
    Command, CommandEffect, CommandId, CommandMeta, CommandRegistry, CommandScope, Shortcut,
};
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
pub use testing::{DirtyFlags, FrameTiming};
pub use theme::{
    color_with_alpha, text_style_with_color, text_style_with_scale, ColorTokens,
    ComponentIconStates, ComponentLayoutTokens, ComponentRole, ComponentState, ComponentStateSlot,
    ComponentStyle, ComponentTextStates, ComponentTokens, ComponentVisualStates, EffectTokens,
    IconStyle, LayerEffect, LayerEffectKind, MotionCurve, MotionTokens, OpacityTokens,
    RadiusTokens, ScopedThemeRegistry, SpacingTokens, StrokeTokens, Theme, ThemePatch, ThemeScope,
    ThemeScopeError, ThemeScopeId, ThemeScopeKind, TypographyTokens, OPERAD_BUBBLEGUM_THEME_NAME,
    OPERAD_DARK_THEME_NAME, OPERAD_LIGHT_THEME_NAME,
};
