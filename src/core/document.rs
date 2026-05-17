use std::collections::{HashMap, HashSet};

#[cfg(feature = "text-cosmic")]
use cosmic_text::{
    fontdb, Attrs, Buffer, Family as CosmicFamily, FontSystem, Metrics, Shaping,
    Stretch as CosmicStretch, Style as CosmicFontStyle, Weight as CosmicWeight, Wrap as CosmicWrap,
};
#[cfg(feature = "text-cosmic")]
use std::sync::Arc;
use taffy::prelude::{
    AlignItems, AvailableSpace, CompactLength, Dimension, Display, FlexDirection, FlexWrap,
    JustifyContent, LengthPercentage, LengthPercentageAuto, NodeId as TaffyNodeId,
    Rect as TaffyRect, Size as TaffySize, Style, TaffyTree,
};

use crate::compositor::*;
use crate::effective_geometry::EffectiveGeometry;
use crate::i18n::{
    BidiPolicy, DynamicLabelMeta, LocaleId, LocalizationPolicy, ResolvedTextDirection,
};
use crate::paint::*;
use crate::{actions, input, platform, renderer};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiPoint {
    pub x: f32,
    pub y: f32,
}

impl UiPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiSize {
    pub width: f32,
    pub height: f32,
}

impl UiSize {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl UiRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    pub fn contains_point(self, point: UiPoint) -> bool {
        point.x >= self.x
            && point.x <= self.right()
            && point.y >= self.y
            && point.y <= self.bottom()
    }

    pub fn intersects(self, other: UiRect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub fn contains_rect(self, other: UiRect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    pub fn intersection(self, other: UiRect) -> Option<UiRect> {
        if !self.intersects(other) {
            return None;
        }
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        Some(UiRect::new(
            x,
            y,
            (right - x).max(0.0),
            (bottom - y).max(0.0),
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UiNodeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorRgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ColorRgba {
    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn composite_over(self, background: Self) -> Self {
        let foreground_alpha = self.a as f32 / 255.0;
        let background_alpha = background.a as f32 / 255.0;
        let alpha = foreground_alpha + background_alpha * (1.0 - foreground_alpha);
        if alpha <= f32::EPSILON {
            return Self::TRANSPARENT;
        }
        let channel = |foreground: u8, background: u8| {
            ((foreground as f32 * foreground_alpha
                + background as f32 * background_alpha * (1.0 - foreground_alpha))
                / alpha)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Self::new(
            channel(self.r, background.r),
            channel(self.g, background.g),
            channel(self.b, background.b),
            (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
        )
    }

    pub fn relative_luminance(self) -> f32 {
        fn channel(value: u8) -> f32 {
            let value = value as f32 / 255.0;
            if value <= 0.03928 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    pub fn contrast_ratio(self, other: Self) -> f32 {
        let first = self.relative_luminance();
        let second = other.relative_luminance();
        let lighter = first.max(second);
        let darker = first.min(second);
        (lighter + 0.05) / (darker + 0.05)
    }

    pub fn meets_contrast_ratio(self, background: Self, minimum_ratio: f32) -> bool {
        self.contrast_ratio(background) + f32::EPSILON >= minimum_ratio
    }

    pub fn highest_contrast_against(self, first: Self, second: Self) -> Self {
        if first.contrast_ratio(self) >= second.contrast_ratio(self) {
            first
        } else {
            second
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipBehavior {
    None,
    Clip,
}

/// Controls which ancestor clip a node inherits during layout and hit testing.
///
/// `Viewport` keeps the node's normal layout parent and coordinate space, but
/// treats the document viewport as the inherited clip. This is intended for
/// floating overlay roots such as popups, tooltips, and modal scrims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipScope {
    Parent,
    Viewport,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UiPortalId(String);

impl UiPortalId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for UiPortalId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for UiPortalId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for UiPortalId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

pub const APP_OVERLAY_PORTAL: &str = "app-overlay";

/// Controls where a node is mounted in the document tree.
///
/// Portal children still use the coordinates supplied by their layout style.
/// Use `AppOverlay` for viewport-positioned floating surfaces, `Named` for
/// application-provided hosts, and `Parent` for inline/preview surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiPortalTarget {
    Parent,
    AppOverlay,
    Named(UiPortalId),
}

impl UiPortalTarget {
    pub fn named(id: impl Into<UiPortalId>) -> Self {
        Self::Named(id.into())
    }
}

impl Default for UiPortalTarget {
    fn default() -> Self {
        Self::Parent
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeStyle {
    pub color: ColorRgba,
    pub width: f32,
}

impl StrokeStyle {
    pub const fn new(color: ColorRgba, width: f32) -> Self {
        Self { color, width }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiVisual {
    pub fill: ColorRgba,
    pub stroke: Option<StrokeStyle>,
    pub corner_radius: f32,
}

impl UiVisual {
    pub const TRANSPARENT: Self = Self {
        fill: ColorRgba::TRANSPARENT,
        stroke: None,
        corner_radius: 0.0,
    };

    pub const fn panel(fill: ColorRgba, stroke: Option<StrokeStyle>, corner_radius: f32) -> Self {
        Self {
            fill,
            stroke,
            corner_radius,
        }
    }
}

impl Default for UiVisual {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InteractionVisuals {
    pub normal: UiVisual,
    pub hovered: Option<UiVisual>,
    pub pressed: Option<UiVisual>,
    pub pressed_hovered: Option<UiVisual>,
    pub focused: Option<UiVisual>,
    pub disabled: Option<UiVisual>,
}

impl InteractionVisuals {
    pub const fn new(normal: UiVisual) -> Self {
        Self {
            normal,
            hovered: None,
            pressed: None,
            pressed_hovered: None,
            focused: None,
            disabled: None,
        }
    }

    pub const fn hovered(mut self, hovered: UiVisual) -> Self {
        self.hovered = Some(hovered);
        self
    }

    pub const fn pressed(mut self, pressed: UiVisual) -> Self {
        self.pressed = Some(pressed);
        self
    }

    pub const fn pressed_hovered(mut self, pressed_hovered: UiVisual) -> Self {
        self.pressed_hovered = Some(pressed_hovered);
        self
    }

    pub const fn focused(mut self, focused: UiVisual) -> Self {
        self.focused = Some(focused);
        self
    }

    pub const fn disabled(mut self, disabled: UiVisual) -> Self {
        self.disabled = Some(disabled);
        self
    }

    pub const fn resolve(
        self,
        enabled: bool,
        hovered: bool,
        pressed: bool,
        focused: bool,
    ) -> UiVisual {
        if !enabled {
            match self.disabled {
                Some(disabled) => disabled,
                None => self.normal,
            }
        } else if pressed && hovered {
            match self.pressed_hovered {
                Some(pressed_hovered) => pressed_hovered,
                None => match self.pressed {
                    Some(pressed) => pressed,
                    None => self.normal,
                },
            }
        } else if pressed {
            match self.pressed {
                Some(pressed) => pressed,
                None => self.normal,
            }
        } else if hovered {
            match self.hovered {
                Some(hovered) => hovered,
                None => self.normal,
            }
        } else if focused {
            match self.focused {
                Some(focused) => focused,
                None => self.normal,
            }
        } else {
            self.normal
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(String),
}

impl Default for FontFamily {
    fn default() -> Self {
        Self::SansSerif
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const NORMAL: Self = Self(400);
    pub const BOLD: Self = Self(700);
    pub const BLACK: Self = Self(900);
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontStretch {
    Condensed,
    Normal,
    Expanded,
}

impl Default for FontStretch {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextWrap {
    None,
    Glyph,
    Word,
    WordOrGlyph,
}

impl Default for TextWrap {
    fn default() -> Self {
        Self::Word
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub family: FontFamily,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub stretch: FontStretch,
    pub wrap: TextWrap,
    pub color: ColorRgba,
    pub underline: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            line_height: 20.0,
            family: FontFamily::SansSerif,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
            stretch: FontStretch::Normal,
            wrap: TextWrap::Word,
            color: ColorRgba::WHITE,
            underline: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextInteractionStyles {
    pub normal: TextStyle,
    pub hovered: Option<TextStyle>,
    pub pressed: Option<TextStyle>,
    pub pressed_hovered: Option<TextStyle>,
    pub focused: Option<TextStyle>,
    pub disabled: Option<TextStyle>,
}

impl TextInteractionStyles {
    pub fn new(normal: TextStyle) -> Self {
        Self {
            normal,
            hovered: None,
            pressed: None,
            pressed_hovered: None,
            focused: None,
            disabled: None,
        }
    }

    pub fn hovered(mut self, hovered: TextStyle) -> Self {
        self.hovered = Some(hovered);
        self
    }

    pub fn pressed(mut self, pressed: TextStyle) -> Self {
        self.pressed = Some(pressed);
        self
    }

    pub fn pressed_hovered(mut self, pressed_hovered: TextStyle) -> Self {
        self.pressed_hovered = Some(pressed_hovered);
        self
    }

    pub fn focused(mut self, focused: TextStyle) -> Self {
        self.focused = Some(focused);
        self
    }

    pub fn disabled(mut self, disabled: TextStyle) -> Self {
        self.disabled = Some(disabled);
        self
    }

    pub fn resolve(&self, enabled: bool, hovered: bool, pressed: bool, focused: bool) -> TextStyle {
        if !enabled {
            self.disabled.clone().unwrap_or_else(|| self.normal.clone())
        } else if pressed && hovered {
            self.pressed_hovered
                .clone()
                .or_else(|| self.pressed.clone())
                .unwrap_or_else(|| self.normal.clone())
        } else if pressed {
            self.pressed.clone().unwrap_or_else(|| self.normal.clone())
        } else if hovered {
            self.hovered.clone().unwrap_or_else(|| self.normal.clone())
        } else if focused {
            self.focused.clone().unwrap_or_else(|| self.normal.clone())
        } else {
            self.normal.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextContent {
    pub text: String,
    pub intrinsic_text: Option<String>,
    pub style: TextStyle,
    pub locale: Option<LocaleId>,
    pub direction: ResolvedTextDirection,
    pub bidi: BidiPolicy,
    pub dynamic_label: Option<DynamicLabelMeta>,
}

impl TextContent {
    pub fn new(text: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            intrinsic_text: None,
            style,
            locale: None,
            direction: ResolvedTextDirection::Ltr,
            bidi: BidiPolicy::default(),
            dynamic_label: None,
        }
    }

    pub fn with_intrinsic_text(mut self, text: impl Into<String>) -> Self {
        self.intrinsic_text = Some(text.into());
        self
    }

    pub fn with_localization_policy(mut self, policy: &LocalizationPolicy) -> Self {
        self.locale = Some(policy.locale.clone());
        self.direction = policy.resolved_direction();
        self.bidi = policy.bidi;
        self
    }

    pub fn with_dynamic_label(
        mut self,
        label: DynamicLabelMeta,
        policy: Option<&LocalizationPolicy>,
    ) -> Self {
        self.text = label.fallback.clone();
        self.locale = label
            .locale
            .clone()
            .or_else(|| policy.map(|policy| policy.locale.clone()));
        self.direction = label.resolved_direction(policy);
        self.bidi = label.bidi;
        self.dynamic_label = Some(label);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasContent {
    pub key: String,
    pub render_mode: CanvasRenderMode,
    pub context: CanvasContextDescriptor,
    pub interaction: CanvasInteractionPolicy,
    pub program: Option<CanvasRenderProgram>,
}

impl CanvasContent {
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        Self {
            context: CanvasContextDescriptor::gpu_texture(&key),
            key,
            render_mode: CanvasRenderMode::Callback,
            interaction: CanvasInteractionPolicy::default(),
            program: None,
        }
    }

    pub fn from_context(context: CanvasContextDescriptor) -> Self {
        Self {
            key: context.surface.id.key.clone(),
            context,
            render_mode: CanvasRenderMode::AttachedContext,
            interaction: CanvasInteractionPolicy::default(),
            program: None,
        }
    }

    pub fn render_mode(mut self, render_mode: CanvasRenderMode) -> Self {
        self.render_mode = render_mode;
        self
    }

    pub fn callback(self) -> Self {
        self.render_mode(CanvasRenderMode::Callback)
    }

    pub fn texture(self) -> Self {
        self.render_mode(CanvasRenderMode::Texture)
    }

    pub fn native_viewport(self) -> Self {
        self.render_mode(CanvasRenderMode::NativeViewport)
            .context_kind(CanvasContextKind::NativeViewport)
    }

    pub fn attached_context(self) -> Self {
        self.render_mode(CanvasRenderMode::AttachedContext)
    }

    pub fn context(mut self, context: CanvasContextDescriptor) -> Self {
        self.context = context;
        self
    }

    pub fn context_kind(mut self, kind: CanvasContextKind) -> Self {
        self.context.kind = kind;
        self
    }

    pub fn gpu_context(self) -> Self {
        self.attached_context().context_kind(CanvasContextKind::Gpu)
    }

    pub fn wgsl(mut self, shader: impl Into<String>) -> Self {
        self.program = Some(CanvasRenderProgram::wgsl(shader));
        self.gpu_context()
    }

    pub fn program(mut self, program: CanvasRenderProgram) -> Self {
        self.program = Some(program);
        self.gpu_context()
    }

    pub fn two_d_context(self) -> Self {
        self.attached_context()
            .context_kind(CanvasContextKind::TwoD)
    }

    pub fn surface_key(&self) -> &str {
        &self.context.surface.id.key
    }

    pub fn surface_handle(&self) -> platform::ResourceHandle {
        platform::ResourceHandle::Texture(self.context.surface.clone())
    }

    pub fn surface_descriptor(
        &self,
        size: platform::PixelSize,
        format: renderer::ResourceFormat,
    ) -> renderer::ResourceDescriptor {
        renderer::ResourceDescriptor::new(self.surface_handle(), size, format)
    }

    pub fn interaction(mut self, interaction: CanvasInteractionPolicy) -> Self {
        self.interaction = interaction;
        self
    }

    pub fn pointer_capture(mut self, pointer_capture: bool) -> Self {
        self.interaction.pointer_capture = pointer_capture;
        self
    }

    pub fn keyboard_capture(mut self, keyboard_capture: bool) -> Self {
        self.interaction.keyboard_capture = keyboard_capture;
        self
    }

    pub fn wheel_capture(mut self, wheel_capture: bool) -> Self {
        self.interaction.wheel_capture = wheel_capture;
        self
    }

    pub fn pointer_lock(mut self, pointer_lock: bool) -> Self {
        self.interaction.pointer_lock = pointer_lock;
        self
    }

    pub fn domain_hit_testing(mut self, domain_hit_testing: bool) -> Self {
        self.interaction.domain_hit_testing = domain_hit_testing;
        self
    }

    pub const fn requires_host_input_capture(&self) -> bool {
        self.interaction.requires_host_capture()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasRenderProgram {
    pub label: Option<String>,
    pub wgsl: String,
    pub vertex_entry_point: String,
    pub fragment_entry_point: String,
    pub clear_color: Option<ColorRgba>,
    pub constants: Vec<CanvasShaderConstant>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasShaderConstant {
    pub name: String,
    pub value: f64,
}

impl CanvasShaderConstant {
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

impl CanvasRenderProgram {
    pub fn wgsl(shader: impl Into<String>) -> Self {
        Self {
            label: Some("canvas-render-pass".to_string()),
            wgsl: shader.into(),
            vertex_entry_point: "vs_main".to_string(),
            fragment_entry_point: "fs_main".to_string(),
            clear_color: None,
            constants: Vec::new(),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn vertex_entry_point(mut self, entry_point: impl Into<String>) -> Self {
        self.vertex_entry_point = entry_point.into();
        self
    }

    pub fn fragment_entry_point(mut self, entry_point: impl Into<String>) -> Self {
        self.fragment_entry_point = entry_point.into();
        self
    }

    pub const fn clear_color(mut self, clear_color: Option<ColorRgba>) -> Self {
        self.clear_color = clear_color;
        self
    }

    pub fn constant(mut self, name: impl Into<String>, value: f64) -> Self {
        self.constants.push(CanvasShaderConstant::new(name, value));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasContextDescriptor {
    pub kind: CanvasContextKind,
    pub surface: platform::TextureHandle,
    pub alpha: bool,
    pub antialias: bool,
    pub preserve_drawing_buffer: bool,
}

impl CanvasContextDescriptor {
    pub fn gpu_texture(key: impl Into<String>) -> Self {
        Self {
            kind: CanvasContextKind::Gpu,
            surface: platform::TextureHandle::app(key),
            alpha: true,
            antialias: true,
            preserve_drawing_buffer: true,
        }
    }

    pub fn two_d_texture(key: impl Into<String>) -> Self {
        Self {
            kind: CanvasContextKind::TwoD,
            ..Self::gpu_texture(key)
        }
    }

    pub fn native_viewport(key: impl Into<String>) -> Self {
        Self {
            kind: CanvasContextKind::NativeViewport,
            surface: platform::TextureHandle::host(key),
            alpha: true,
            antialias: true,
            preserve_drawing_buffer: false,
        }
    }

    pub fn surface(mut self, surface: platform::TextureHandle) -> Self {
        self.surface = surface;
        self
    }

    pub const fn alpha(mut self, alpha: bool) -> Self {
        self.alpha = alpha;
        self
    }

    pub const fn antialias(mut self, antialias: bool) -> Self {
        self.antialias = antialias;
        self
    }

    pub const fn preserve_drawing_buffer(mut self, preserve_drawing_buffer: bool) -> Self {
        self.preserve_drawing_buffer = preserve_drawing_buffer;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanvasContextKind {
    TwoD,
    Gpu,
    NativeViewport,
}

impl CanvasContextKind {
    pub const fn is_texture_backed(self) -> bool {
        matches!(self, Self::TwoD | Self::Gpu)
    }

    pub const fn is_gpu_backed(self) -> bool {
        matches!(self, Self::Gpu)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasRenderMode {
    Callback,
    Texture,
    AttachedContext,
    NativeViewport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CanvasInteractionPolicy {
    pub pointer_capture: bool,
    pub keyboard_capture: bool,
    pub wheel_capture: bool,
    pub pointer_lock: bool,
    pub domain_hit_testing: bool,
}

impl CanvasInteractionPolicy {
    pub const NONE: Self = Self {
        pointer_capture: false,
        keyboard_capture: false,
        wheel_capture: false,
        pointer_lock: false,
        domain_hit_testing: false,
    };

    pub const EDITOR: Self = Self {
        pointer_capture: true,
        keyboard_capture: true,
        wheel_capture: true,
        pointer_lock: false,
        domain_hit_testing: true,
    };

    pub const NATIVE_VIEWPORT: Self = Self {
        pointer_capture: true,
        keyboard_capture: true,
        wheel_capture: true,
        pointer_lock: true,
        domain_hit_testing: true,
    };

    pub const fn requires_host_capture(self) -> bool {
        self.pointer_capture || self.keyboard_capture || self.wheel_capture || self.pointer_lock
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageContent {
    pub key: String,
    pub tint: Option<ColorRgba>,
}

impl ImageContent {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            tint: None,
        }
    }

    pub fn tinted(mut self, tint: ColorRgba) -> Self {
        self.tint = Some(tint);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShaderEffect {
    pub key: String,
    pub uniforms: Vec<ShaderUniform>,
}

impl ShaderEffect {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            uniforms: Vec::new(),
        }
    }

    pub fn uniform(mut self, name: impl Into<String>, value: f32) -> Self {
        self.uniforms.push(ShaderUniform {
            name: name.into(),
            value,
        });
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShaderUniform {
    pub name: String,
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityRole {
    Alert,
    Application,
    Button,
    Checkbox,
    ColumnHeader,
    ComboBox,
    Dialog,
    EditorSurface,
    Group,
    Grid,
    GridCell,
    Image,
    Label,
    Link,
    List,
    ListItem,
    Meter,
    Menu,
    MenuBar,
    MenuItem,
    ProgressBar,
    RadioButton,
    Row,
    RowHeader,
    Ruler,
    SearchBox,
    Separator,
    Slider,
    SpinButton,
    Splitter,
    Status,
    Switch,
    Tab,
    TabList,
    TabPanel,
    TextBox,
    ToggleButton,
    Toolbar,
    Tooltip,
    Tree,
    TreeItem,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityChecked {
    False,
    True,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityLiveRegion {
    Off,
    Polite,
    Assertive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilitySortDirection {
    None,
    Ascending,
    Descending,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccessibilityValueRange {
    pub min: f64,
    pub max: f64,
    pub step: Option<f64>,
}

impl AccessibilityValueRange {
    pub const fn new(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            step: None,
        }
    }

    pub const fn with_step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityAction {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
}

impl AccessibilityAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
        }
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilitySummaryItem {
    pub label: String,
    pub value: String,
}

impl AccessibilitySummaryItem {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilitySummary {
    pub title: String,
    pub description: Option<String>,
    pub items: Vec<AccessibilitySummaryItem>,
    pub instructions: Vec<String>,
}

impl AccessibilitySummary {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            items: Vec::new(),
            instructions: Vec::new(),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn item(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.items.push(AccessibilitySummaryItem::new(label, value));
        self
    }

    pub fn instruction(mut self, instruction: impl Into<String>) -> Self {
        self.instructions.push(instruction.into());
        self
    }

    pub fn screen_reader_text(&self) -> String {
        let mut parts = Vec::new();
        if !self.title.is_empty() {
            parts.push(self.title.clone());
        }
        if let Some(description) = &self.description {
            if !description.is_empty() {
                parts.push(description.clone());
            }
        }
        for item in &self.items {
            if item.value.is_empty() {
                parts.push(item.label.clone());
            } else {
                parts.push(format!("{}: {}", item.label, item.value));
            }
        }
        parts.extend(
            self.instructions
                .iter()
                .filter(|instruction| !instruction.is_empty())
                .cloned(),
        );
        parts.join(". ")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccessibilityRelations {
    pub labelled_by: Vec<UiNodeId>,
    pub described_by: Vec<UiNodeId>,
    pub controls: Vec<UiNodeId>,
    pub owns: Vec<UiNodeId>,
    pub active_descendant: Option<UiNodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityMeta {
    pub role: AccessibilityRole,
    pub label: Option<String>,
    pub value: Option<String>,
    pub hint: Option<String>,
    pub enabled: bool,
    pub focusable: bool,
    pub hidden: bool,
    pub modal: bool,
    pub selected: Option<bool>,
    pub checked: Option<AccessibilityChecked>,
    pub expanded: Option<bool>,
    pub pressed: Option<bool>,
    pub read_only: bool,
    pub required: bool,
    pub invalid: Option<String>,
    pub live_region: AccessibilityLiveRegion,
    pub sort: AccessibilitySortDirection,
    pub value_range: Option<AccessibilityValueRange>,
    pub focus_order: Option<i32>,
    pub key_shortcuts: Vec<String>,
    pub actions: Vec<AccessibilityAction>,
    pub relations: AccessibilityRelations,
    pub summary: Option<AccessibilitySummary>,
}

impl AccessibilityMeta {
    pub fn new(role: AccessibilityRole) -> Self {
        Self {
            role,
            label: None,
            value: None,
            hint: None,
            enabled: true,
            focusable: false,
            hidden: false,
            modal: false,
            selected: None,
            checked: None,
            expanded: None,
            pressed: None,
            read_only: false,
            required: false,
            invalid: None,
            live_region: AccessibilityLiveRegion::Off,
            sort: AccessibilitySortDirection::None,
            value_range: None,
            focus_order: None,
            key_shortcuts: Vec::new(),
            actions: Vec::new(),
            relations: AccessibilityRelations::default(),
            summary: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn focusable(mut self) -> Self {
        self.focusable = true;
        self
    }

    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    pub fn modal(mut self) -> Self {
        self.modal = true;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(if checked {
            AccessibilityChecked::True
        } else {
            AccessibilityChecked::False
        });
        self
    }

    pub fn mixed(mut self) -> Self {
        self.checked = Some(AccessibilityChecked::Mixed);
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn invalid(mut self, reason: impl Into<String>) -> Self {
        self.invalid = Some(reason.into());
        self
    }

    pub fn live_region(mut self, live_region: AccessibilityLiveRegion) -> Self {
        self.live_region = live_region;
        self
    }

    pub fn sort(mut self, sort: AccessibilitySortDirection) -> Self {
        self.sort = sort;
        self
    }

    pub fn value_range(mut self, range: AccessibilityValueRange) -> Self {
        self.value_range = Some(range);
        self
    }

    pub fn focus_order(mut self, order: i32) -> Self {
        self.focus_order = Some(order);
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.key_shortcuts.push(shortcut.into());
        self
    }

    pub fn action(mut self, action: AccessibilityAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn summary(mut self, summary: AccessibilitySummary) -> Self {
        self.summary = Some(summary);
        self
    }

    pub fn labelled_by(mut self, id: UiNodeId) -> Self {
        self.relations.labelled_by.push(id);
        self
    }

    pub fn described_by(mut self, id: UiNodeId) -> Self {
        self.relations.described_by.push(id);
        self
    }

    pub fn controls(mut self, id: UiNodeId) -> Self {
        self.relations.controls.push(id);
        self
    }

    pub fn owns(mut self, id: UiNodeId) -> Self {
        self.relations.owns.push(id);
        self
    }

    pub fn active_descendant(mut self, id: UiNodeId) -> Self {
        self.relations.active_descendant = Some(id);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScenePrimitive {
    Line {
        from: UiPoint,
        to: UiPoint,
        stroke: StrokeStyle,
    },
    Circle {
        center: UiPoint,
        radius: f32,
        fill: ColorRgba,
        stroke: Option<StrokeStyle>,
    },
    Polygon {
        points: Vec<UiPoint>,
        fill: ColorRgba,
        stroke: Option<StrokeStyle>,
    },
    MorphPolygon {
        from_points: Vec<UiPoint>,
        to_points: Vec<UiPoint>,
        amount: f32,
        fill: ColorRgba,
        stroke: Option<StrokeStyle>,
    },
    Image {
        key: String,
        rect: UiRect,
        tint: Option<ColorRgba>,
    },
    Rect(PaintRect),
    Text(PaintText),
    Path(PaintPath),
    ImagePlacement(PaintImage),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiContent {
    Empty,
    Text(TextContent),
    Canvas(CanvasContent),
    Image(ImageContent),
    PaintRect(PaintRect),
    Scene(Vec<ScenePrimitive>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputBehavior {
    pub pointer: bool,
    pub focusable: bool,
    pub keyboard: bool,
}

impl InputBehavior {
    pub const NONE: Self = Self {
        pointer: false,
        focusable: false,
        keyboard: false,
    };

    pub const BUTTON: Self = Self {
        pointer: true,
        focusable: true,
        keyboard: true,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollAxes {
    pub horizontal: bool,
    pub vertical: bool,
}

impl ScrollAxes {
    pub const NONE: Self = Self {
        horizontal: false,
        vertical: false,
    };
    pub const VERTICAL: Self = Self {
        horizontal: false,
        vertical: true,
    };
    pub const HORIZONTAL: Self = Self {
        horizontal: true,
        vertical: false,
    };
    pub const BOTH: Self = Self {
        horizontal: true,
        vertical: true,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollState {
    pub axes: ScrollAxes,
    pub offset: UiPoint,
    pub viewport_size: UiSize,
    pub content_size: UiSize,
}

impl ScrollState {
    pub const fn new(axes: ScrollAxes) -> Self {
        Self {
            axes,
            offset: UiPoint::new(0.0, 0.0),
            viewport_size: UiSize::ZERO,
            content_size: UiSize::ZERO,
        }
    }

    pub fn max_offset(self) -> UiPoint {
        UiPoint::new(
            if self.axes.horizontal {
                (self.content_size.width - self.viewport_size.width).max(0.0)
            } else {
                0.0
            },
            if self.axes.vertical {
                (self.content_size.height - self.viewport_size.height).max(0.0)
            } else {
                0.0
            },
        )
    }

    pub fn clamp_offset(self, offset: UiPoint) -> UiPoint {
        let max = self.max_offset();
        UiPoint::new(offset.x.clamp(0.0, max.x), offset.y.clamp(0.0, max.y))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarAuditState {
    pub axis: AuditAxis,
    pub scroll: ScrollState,
}

impl ScrollbarAuditState {
    pub const fn new(axis: AuditAxis, scroll: ScrollState) -> Self {
        Self { axis, scroll }
    }

    pub fn max_offset(self) -> f32 {
        match self.axis {
            AuditAxis::Horizontal => self.scroll.max_offset().x,
            AuditAxis::Vertical => self.scroll.max_offset().y,
        }
    }

    pub fn viewport(self) -> f32 {
        match self.axis {
            AuditAxis::Horizontal => self.scroll.viewport_size.width,
            AuditAxis::Vertical => self.scroll.viewport_size.height,
        }
    }

    pub fn content(self) -> f32 {
        match self.axis {
            AuditAxis::Horizontal => self.scroll.content_size.width,
            AuditAxis::Vertical => self.scroll.content_size.height,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutStyle {
    pub(crate) style: Style,
}

impl LayoutStyle {
    pub fn new() -> Self {
        Self {
            style: Style::default(),
        }
    }

    pub fn from_taffy_style(style: Style) -> Self {
        Self { style }
    }

    pub fn as_taffy_style(&self) -> &Style {
        &self.style
    }

    pub fn as_taffy_style_mut(&mut self) -> &mut Style {
        &mut self.style
    }

    pub fn is_absolute(&self) -> bool {
        self.style.position == taffy::prelude::Position::Absolute
    }

    pub fn row() -> Self {
        Self::from_taffy_style(Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            ..Default::default()
        })
    }

    pub fn column() -> Self {
        Self::from_taffy_style(Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
    }

    pub fn toolbar() -> Self {
        Self::row()
            .with_align_items(AlignItems::Center)
            .with_width_percent(1.0)
    }

    pub fn size(width: f32, height: f32) -> Self {
        Self::new().with_size(width, height)
    }

    pub fn absolute_rect(rect: UiRect) -> Self {
        Self::from_taffy_style(Style {
            position: taffy::prelude::Position::Absolute,
            inset: taffy::prelude::Rect {
                left: taffy::prelude::LengthPercentageAuto::length(rect.x),
                top: taffy::prelude::LengthPercentageAuto::length(rect.y),
                right: taffy::prelude::LengthPercentageAuto::auto(),
                bottom: taffy::prelude::LengthPercentageAuto::auto(),
            },
            size: TaffySize {
                width: length(rect.width),
                height: length(rect.height),
            },
            ..Default::default()
        })
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.style.size = TaffySize {
            width: length(width),
            height: length(height),
        };
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.style.size.width = length(width);
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.style.size.height = length(height);
        self
    }

    pub fn with_width_percent(mut self, width: f32) -> Self {
        self.style.size.width = Dimension::percent(width);
        self
    }

    pub fn with_height_percent(mut self, height: f32) -> Self {
        self.style.size.height = Dimension::percent(height);
        self
    }

    pub fn with_flex_grow(mut self, grow: f32) -> Self {
        self.style.flex_grow = grow;
        self
    }

    pub fn with_flex_shrink(mut self, shrink: f32) -> Self {
        self.style.flex_shrink = shrink;
        self
    }

    pub fn with_align_items(mut self, align_items: AlignItems) -> Self {
        self.style.align_items = Some(align_items);
        self
    }

    pub fn with_justify_content(mut self, justify_content: JustifyContent) -> Self {
        self.style.justify_content = Some(justify_content);
        self
    }

    pub fn with_padding(mut self, value: f32) -> Self {
        self.style.padding = taffy::prelude::Rect::length(value);
        self
    }

    pub fn padding(self, value: f32) -> Self {
        self.with_padding(value)
    }

    pub fn with_gap(mut self, value: f32) -> Self {
        self.style.gap = TaffySize {
            width: taffy::prelude::LengthPercentage::length(value),
            height: taffy::prelude::LengthPercentage::length(value),
        };
        self
    }

    pub fn gap(self, value: f32) -> Self {
        self.with_gap(value)
    }
}

impl From<Style> for LayoutStyle {
    fn from(style: Style) -> Self {
        Self::from_taffy_style(style)
    }
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct UiNodeStyle {
    pub layout: Style,
    pub clip: ClipBehavior,
    pub opacity: f32,
    pub z_index: i16,
}

impl From<LayoutStyle> for UiNodeStyle {
    fn from(layout: LayoutStyle) -> Self {
        Self {
            layout: layout.style,
            ..Default::default()
        }
    }
}

impl From<Style> for UiNodeStyle {
    fn from(style: Style) -> Self {
        Self {
            layout: style,
            ..Default::default()
        }
    }
}

impl Default for UiNodeStyle {
    fn default() -> Self {
        Self {
            layout: Style::default(),
            clip: ClipBehavior::None,
            opacity: 1.0,
            z_index: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiNode {
    pub name: String,
    pub parent: Option<UiNodeId>,
    pub children: Vec<UiNodeId>,
    pub style: UiNodeStyle,
    pub layer: Option<platform::UiLayer>,
    pub clip_scope: ClipScope,
    pub visual: UiVisual,
    pub interaction_visuals: Option<InteractionVisuals>,
    pub interaction_text_styles: Option<TextInteractionStyles>,
    pub action: Option<actions::WidgetActionBinding>,
    pub action_mode: actions::WidgetActionMode,
    pub content: UiContent,
    pub input: InputBehavior,
    pub scroll: Option<ScrollState>,
    pub scrollbar: Option<ScrollbarAuditState>,
    pub animation: Option<AnimationMachine>,
    pub accessibility: Option<AccessibilityMeta>,
    pub shader: Option<ShaderEffect>,
    pub layout_constraint: Option<UiNodeLayoutConstraint>,
    pub layout: ComputedLayout,
}

impl UiNode {
    pub fn container(name: impl Into<String>, style: impl Into<UiNodeStyle>) -> Self {
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: style.into(),
            layer: None,
            clip_scope: ClipScope::Parent,
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::Empty,
            input: InputBehavior::NONE,
            scroll: None,
            scrollbar: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout_constraint: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn text(
        name: impl Into<String>,
        text: impl Into<String>,
        text_style: TextStyle,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        let layout = layout.into();
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: UiNodeStyle {
                layout: layout.style,
                ..Default::default()
            },
            layer: None,
            clip_scope: ClipScope::Parent,
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::Text(TextContent::new(text, text_style)),
            input: InputBehavior::NONE,
            scroll: None,
            scrollbar: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout_constraint: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn localized_text(
        name: impl Into<String>,
        label: DynamicLabelMeta,
        policy: Option<&LocalizationPolicy>,
        text_style: TextStyle,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        let layout = layout.into();
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: UiNodeStyle {
                layout: layout.style,
                ..Default::default()
            },
            layer: None,
            clip_scope: ClipScope::Parent,
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::Text(
                TextContent::new(label.fallback.clone(), text_style)
                    .with_dynamic_label(label, policy),
            ),
            input: InputBehavior::NONE,
            scroll: None,
            scrollbar: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout_constraint: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn canvas(
        name: impl Into<String>,
        key: impl Into<String>,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        let layout = layout.into();
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: UiNodeStyle {
                layout: layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
            layer: None,
            clip_scope: ClipScope::Parent,
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::Canvas(CanvasContent::new(key)),
            input: InputBehavior {
                pointer: true,
                focusable: true,
                keyboard: true,
            },
            scroll: None,
            scrollbar: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout_constraint: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn canvas_with_context(
        name: impl Into<String>,
        context: CanvasContextDescriptor,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        let mut node = Self::canvas(name, context.surface.id.key.clone(), layout);
        node.content = UiContent::Canvas(CanvasContent::from_context(context));
        node
    }

    pub fn gpu_canvas(
        name: impl Into<String>,
        key: impl Into<String>,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        Self::canvas_with_context(name, CanvasContextDescriptor::gpu_texture(key), layout)
    }

    pub fn image(
        name: impl Into<String>,
        image: ImageContent,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        let layout = layout.into();
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: UiNodeStyle {
                layout: layout.style,
                ..Default::default()
            },
            layer: None,
            clip_scope: ClipScope::Parent,
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::Image(image),
            input: InputBehavior::NONE,
            scroll: None,
            scrollbar: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout_constraint: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn paint_rect(
        name: impl Into<String>,
        rect: PaintRect,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        let layout = layout.into();
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: UiNodeStyle {
                layout: layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
            layer: None,
            clip_scope: ClipScope::Parent,
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::PaintRect(rect),
            input: InputBehavior::NONE,
            scroll: None,
            scrollbar: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout_constraint: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn paint_fill(
        name: impl Into<String>,
        fill: impl Into<PaintBrush>,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        Self::paint_rect(
            name,
            PaintRect::new(UiRect::new(0.0, 0.0, 0.0, 0.0), fill),
            layout,
        )
    }

    pub fn scene(
        name: impl Into<String>,
        primitives: Vec<ScenePrimitive>,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        let layout = layout.into();
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: UiNodeStyle {
                layout: layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
            layer: None,
            clip_scope: ClipScope::Parent,
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::Scene(primitives),
            input: InputBehavior::NONE,
            scroll: None,
            scrollbar: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout_constraint: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn with_input(mut self, input: InputBehavior) -> Self {
        self.input = input;
        self
    }

    pub fn with_layer(mut self, layer: platform::UiLayer) -> Self {
        self.layer = Some(layer);
        self
    }

    pub fn with_clip_scope(mut self, clip_scope: ClipScope) -> Self {
        self.clip_scope = clip_scope;
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }

    pub fn with_interaction_visuals(mut self, visuals: InteractionVisuals) -> Self {
        self.interaction_visuals = Some(visuals);
        self.visual = visuals.normal;
        self
    }

    pub fn with_interaction_text_styles(mut self, styles: TextInteractionStyles) -> Self {
        if let UiContent::Text(text) = &mut self.content {
            text.style = styles.normal.clone();
        }
        self.interaction_text_styles = Some(styles);
        self
    }

    pub fn with_action(mut self, action: impl Into<actions::WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_action_mode(mut self, mode: actions::WidgetActionMode) -> Self {
        self.action_mode = mode;
        self
    }

    pub fn with_pointer_edit_action(
        mut self,
        action: impl Into<actions::WidgetActionBinding>,
    ) -> Self {
        self.action = Some(action.into());
        self.action_mode = actions::WidgetActionMode::PointerEdit;
        self
    }

    pub fn with_scroll(mut self, axes: ScrollAxes) -> Self {
        self.style.clip = ClipBehavior::Clip;
        self.scroll = Some(ScrollState::new(axes));
        self
    }

    pub fn with_scrollbar_audit(mut self, axis: AuditAxis, scroll: ScrollState) -> Self {
        self.scrollbar = Some(ScrollbarAuditState::new(axis, scroll));
        self
    }

    pub fn with_animation(mut self, animation: AnimationMachine) -> Self {
        self.animation = Some(animation);
        self
    }

    pub fn with_accessibility(mut self, accessibility: AccessibilityMeta) -> Self {
        self.accessibility = Some(accessibility);
        self
    }

    pub fn with_shader(mut self, shader: ShaderEffect) -> Self {
        self.shader = Some(shader);
        self
    }

    pub fn with_layout_constraint(mut self, constraint: UiNodeLayoutConstraint) -> Self {
        self.layout_constraint = Some(constraint);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComputedLayout {
    pub rect: UiRect,
    pub clip_rect: UiRect,
    pub visible: bool,
    pub opacity: f32,
    pub content_size: Option<UiSize>,
}

impl Default for ComputedLayout {
    fn default() -> Self {
        Self {
            rect: UiRect::new(0.0, 0.0, 0.0, 0.0),
            clip_rect: UiRect::new(0.0, 0.0, 0.0, 0.0),
            visible: false,
            opacity: 1.0,
            content_size: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntrinsicSize {
    pub min: UiSize,
    pub preferred: UiSize,
}

impl IntrinsicSize {
    pub const ZERO: Self = Self {
        min: UiSize::ZERO,
        preferred: UiSize::ZERO,
    };
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiNodeLayoutConstraint {
    InlineIntrinsicSize {
        sources: Vec<UiNodeId>,
        min_size: UiSize,
    },
    StackedIntrinsicSize {
        sources: Vec<UiNodeId>,
        min_size: UiSize,
        bounds: UiRect,
        fit_to_preferred: bool,
    },
}

#[derive(Debug, Clone)]
enum MeasureContext {
    Text { node: UiNodeId, text: TextContent },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntrinsicMeasureMode {
    Min,
    Preferred,
}

#[derive(Debug)]
struct LayoutSizingPass {
    root: TaffyNodeId,
    taffy: TaffyTree<MeasureContext>,
    mapping: Vec<Option<TaffyNodeId>>,
    measured_content: Vec<Option<UiSize>>,
    sizes: Vec<UiSize>,
}

#[derive(Debug, Clone, Copy)]
pub struct KnownSize {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct AvailableSize {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

pub trait TextMeasurer {
    fn measure(&mut self, text: &TextContent, known: KnownSize, available: AvailableSize)
        -> UiSize;
}

#[derive(Debug, Clone, Copy)]
pub struct ApproxTextMeasurer;

impl TextMeasurer for ApproxTextMeasurer {
    fn measure(
        &mut self,
        text: &TextContent,
        known: KnownSize,
        available: AvailableSize,
    ) -> UiSize {
        let char_width = text.style.font_size * 0.55;
        let explicit_width = known.width.or(available.width);
        let raw_width = (text.text.chars().count() as f32 * char_width).max(char_width);
        let width = explicit_width.map_or(raw_width, |available| {
            raw_width.min(available.max(char_width))
        });
        let lines = (raw_width / width.max(char_width)).ceil().max(1.0);
        UiSize::new(
            known.width.unwrap_or(width),
            known.height.unwrap_or(lines * text.style.line_height),
        )
    }
}

#[cfg(feature = "text-cosmic")]
const COSMIC_TEXT_MEASURE_CACHE_LIMIT: usize = 32_768;

#[cfg(feature = "text-cosmic")]
pub struct CosmicTextMeasurer {
    font_system: FontSystem,
    cache: HashMap<u64, Vec<(TextMeasureKey, UiSize)>>,
    cache_len: usize,
}

#[cfg(feature = "text-cosmic")]
impl CosmicTextMeasurer {
    pub fn new() -> Self {
        Self {
            font_system: default_cosmic_font_system(),
            cache: HashMap::new(),
            cache_len: 0,
        }
    }
}

#[cfg(feature = "text-cosmic")]
impl Default for CosmicTextMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "text-cosmic")]
impl TextMeasurer for CosmicTextMeasurer {
    fn measure(
        &mut self,
        text: &TextContent,
        known: KnownSize,
        available: AvailableSize,
    ) -> UiSize {
        let hash = TextMeasureKey::cache_hash(text, known, available);
        if let Some(measured) = self.cache.get(&hash).and_then(|bucket| {
            bucket.iter().find_map(|(key, measured)| {
                key.matches(text, known, available).then_some(*measured)
            })
        }) {
            return measured;
        }
        let font_size = text.style.font_size.max(1.0);
        let line_height = text.style.line_height.max(font_size);
        let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(font_size, line_height));
        buffer.set_wrap(&mut self.font_system, cosmic_wrap(text.style.wrap));
        buffer.set_size(
            &mut self.font_system,
            known.width.or(available.width),
            known.height.or(available.height),
        );
        let attrs = Attrs::new()
            .family(cosmic_family(&text.style.family))
            .weight(cosmic_weight(text.style.weight))
            .style(cosmic_font_style(text.style.style))
            .stretch(cosmic_stretch(text.style.stretch));
        buffer.set_text(
            &mut self.font_system,
            &text.text,
            &attrs,
            cosmic_shaping(&text.text),
        );

        let mut measured = UiSize::ZERO;
        for run in buffer.layout_runs() {
            measured.width = measured.width.max(run.line_w);
            measured.height = measured.height.max(run.line_top + run.line_height);
        }
        if measured.height <= f32::EPSILON {
            measured.height = line_height;
        }
        let measured = UiSize::new(
            known.width.unwrap_or(measured.width),
            known.height.unwrap_or(measured.height),
        );
        if self.cache_len > COSMIC_TEXT_MEASURE_CACHE_LIMIT {
            self.cache.clear();
            self.cache_len = 0;
        }
        self.cache
            .entry(hash)
            .or_default()
            .push((TextMeasureKey::new(text, known, available), measured));
        self.cache_len += 1;
        measured
    }
}

#[cfg(feature = "text-cosmic")]
fn default_cosmic_font_system() -> FontSystem {
    let mut font_system = FontSystem::new_with_fonts([
        embedded_cosmic_font(epaint_default_fonts::UBUNTU_LIGHT),
        embedded_cosmic_font(epaint_default_fonts::HACK_REGULAR),
        embedded_cosmic_font(epaint_default_fonts::NOTO_EMOJI_REGULAR),
    ]);
    {
        let db = font_system.db_mut();
        db.set_sans_serif_family("Ubuntu");
        db.set_serif_family("Ubuntu");
        db.set_monospace_family("Hack");
    }
    font_system
}

#[cfg(feature = "text-cosmic")]
fn embedded_cosmic_font(bytes: &'static [u8]) -> fontdb::Source {
    let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bytes);
    fontdb::Source::Binary(data)
}

fn measure_taffy_text(
    text_measurer: &mut impl TextMeasurer,
    text: &TextContent,
    known: TaffySize<Option<f32>>,
    available: TaffySize<AvailableSpace>,
) -> UiSize {
    let measurement_text;
    let text = if let Some(intrinsic_text) = &text.intrinsic_text {
        measurement_text = TextContent {
            text: intrinsic_text.clone(),
            intrinsic_text: None,
            ..text.clone()
        };
        &measurement_text
    } else {
        text
    };
    if known.width.is_none() && available.width == AvailableSpace::MinContent {
        return measure_text_min_content(text_measurer, text, known, available);
    }
    text_measurer.measure(
        text,
        KnownSize {
            width: known.width,
            height: known.height,
        },
        AvailableSize {
            width: available_space_to_option(available.width),
            height: available_space_to_option(available.height),
        },
    )
}

fn measure_text_min_content(
    text_measurer: &mut impl TextMeasurer,
    text: &TextContent,
    known: TaffySize<Option<f32>>,
    available: TaffySize<AvailableSpace>,
) -> UiSize {
    match text.style.wrap {
        TextWrap::None => text_measurer.measure(
            text,
            KnownSize {
                width: known.width,
                height: known.height,
            },
            AvailableSize {
                width: None,
                height: available_space_to_option(available.height),
            },
        ),
        TextWrap::Word => measure_text_fragments_min_content(
            text_measurer,
            text,
            known,
            available,
            text.text.split_whitespace(),
        ),
        TextWrap::Glyph | TextWrap::WordOrGlyph => measure_text_fragments_min_content(
            text_measurer,
            text,
            known,
            available,
            text.text
                .chars()
                .map(|ch| ch.len_utf8())
                .scan(0, |start, len| {
                    let end = *start + len;
                    let fragment = &text.text[*start..end];
                    *start = end;
                    Some(fragment)
                }),
        ),
    }
}

fn measure_text_fragments_min_content<'a>(
    text_measurer: &mut impl TextMeasurer,
    text: &TextContent,
    known: TaffySize<Option<f32>>,
    available: TaffySize<AvailableSpace>,
    fragments: impl IntoIterator<Item = &'a str>,
) -> UiSize {
    let mut measured = UiSize::ZERO;
    let mut found_fragment = false;
    for fragment in fragments {
        if fragment.is_empty() {
            continue;
        }
        found_fragment = true;
        let mut fragment_text = text.clone();
        fragment_text.text = fragment.to_string();
        fragment_text.style.wrap = TextWrap::None;
        let fragment_size = text_measurer.measure(
            &fragment_text,
            KnownSize {
                width: None,
                height: known.height,
            },
            AvailableSize {
                width: None,
                height: available_space_to_option(available.height),
            },
        );
        measured.width = measured.width.max(fragment_size.width);
        measured.height = measured.height.max(fragment_size.height);
    }

    if found_fragment {
        UiSize::new(
            known.width.unwrap_or(measured.width),
            known.height.unwrap_or(measured.height),
        )
    } else {
        text_measurer.measure(
            text,
            KnownSize {
                width: known.width,
                height: known.height,
            },
            AvailableSize {
                width: None,
                height: available_space_to_option(available.height),
            },
        )
    }
}

#[cfg(feature = "text-cosmic")]
fn cosmic_family(family: &FontFamily) -> CosmicFamily<'_> {
    match family {
        FontFamily::SansSerif => CosmicFamily::SansSerif,
        FontFamily::Serif => CosmicFamily::Serif,
        FontFamily::Monospace => CosmicFamily::Monospace,
        FontFamily::Named(name) => CosmicFamily::Name(name),
    }
}

#[cfg(feature = "text-cosmic")]
fn cosmic_weight(weight: FontWeight) -> CosmicWeight {
    CosmicWeight(weight.0)
}

#[cfg(feature = "text-cosmic")]
fn cosmic_font_style(style: FontStyle) -> CosmicFontStyle {
    match style {
        FontStyle::Normal => CosmicFontStyle::Normal,
        FontStyle::Italic => CosmicFontStyle::Italic,
        FontStyle::Oblique => CosmicFontStyle::Oblique,
    }
}

#[cfg(feature = "text-cosmic")]
fn cosmic_stretch(stretch: FontStretch) -> CosmicStretch {
    match stretch {
        FontStretch::Condensed => CosmicStretch::Condensed,
        FontStretch::Normal => CosmicStretch::Normal,
        FontStretch::Expanded => CosmicStretch::Expanded,
    }
}

#[cfg(feature = "text-cosmic")]
fn cosmic_wrap(wrap: TextWrap) -> CosmicWrap {
    match wrap {
        TextWrap::None => CosmicWrap::None,
        TextWrap::Glyph => CosmicWrap::Glyph,
        TextWrap::Word => CosmicWrap::Word,
        TextWrap::WordOrGlyph => CosmicWrap::WordOrGlyph,
    }
}

#[cfg(feature = "text-cosmic")]
fn cosmic_shaping(text: &str) -> Shaping {
    if text.is_ascii() {
        Shaping::Basic
    } else {
        Shaping::Advanced
    }
}

#[cfg(feature = "text-cosmic")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextMeasureKey {
    text: String,
    font_size_bits: u32,
    line_height_bits: u32,
    family: FontFamily,
    weight: u16,
    style: FontStyle,
    stretch: FontStretch,
    wrap: u8,
    known_width_bits: Option<u32>,
    known_height_bits: Option<u32>,
    available_width_bits: Option<u32>,
    available_height_bits: Option<u32>,
}

#[cfg(feature = "text-cosmic")]
impl TextMeasureKey {
    fn cache_hash(text: &TextContent, known: KnownSize, available: AvailableSize) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in text.text.as_bytes() {
            hash = fnv_mix(hash, *byte as u64);
        }
        hash = fnv_mix(hash, text.style.font_size.to_bits() as u64);
        hash = fnv_mix(hash, text.style.line_height.to_bits() as u64);
        hash = fnv_mix(hash, font_family_key(&text.style.family));
        hash = fnv_mix(hash, text.style.weight.0 as u64);
        hash = fnv_mix(hash, font_style_key(text.style.style));
        hash = fnv_mix(hash, font_stretch_key(text.style.stretch));
        hash = fnv_mix(hash, wrap_key(text.style.wrap) as u64);
        hash = fnv_mix(hash, option_f32_key(known.width));
        hash = fnv_mix(hash, option_f32_key(known.height));
        hash = fnv_mix(hash, option_f32_key(available.width));
        fnv_mix(hash, option_f32_key(available.height))
    }

    fn new(text: &TextContent, known: KnownSize, available: AvailableSize) -> Self {
        Self {
            text: text.text.clone(),
            font_size_bits: text.style.font_size.to_bits(),
            line_height_bits: text.style.line_height.to_bits(),
            family: text.style.family.clone(),
            weight: text.style.weight.0,
            style: text.style.style,
            stretch: text.style.stretch,
            wrap: wrap_key(text.style.wrap),
            known_width_bits: known.width.map(f32::to_bits),
            known_height_bits: known.height.map(f32::to_bits),
            available_width_bits: available.width.map(f32::to_bits),
            available_height_bits: available.height.map(f32::to_bits),
        }
    }

    fn matches(&self, text: &TextContent, known: KnownSize, available: AvailableSize) -> bool {
        self.text == text.text
            && self.font_size_bits == text.style.font_size.to_bits()
            && self.line_height_bits == text.style.line_height.to_bits()
            && self.family == text.style.family
            && self.weight == text.style.weight.0
            && self.style == text.style.style
            && self.stretch == text.style.stretch
            && self.wrap == wrap_key(text.style.wrap)
            && self.known_width_bits == known.width.map(f32::to_bits)
            && self.known_height_bits == known.height.map(f32::to_bits)
            && self.available_width_bits == available.width.map(f32::to_bits)
            && self.available_height_bits == available.height.map(f32::to_bits)
    }
}

#[cfg(feature = "text-cosmic")]
fn wrap_key(wrap: TextWrap) -> u8 {
    match wrap {
        TextWrap::None => 0,
        TextWrap::Glyph => 1,
        TextWrap::Word => 2,
        TextWrap::WordOrGlyph => 3,
    }
}

#[cfg(feature = "text-cosmic")]
fn fnv_mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x100000001b3)
}

#[cfg(feature = "text-cosmic")]
fn option_f32_key(value: Option<f32>) -> u64 {
    value.map_or(0xffff_ffff_ffff_ffff, |value| value.to_bits() as u64)
}

#[cfg(feature = "text-cosmic")]
fn font_family_key(family: &FontFamily) -> u64 {
    match family {
        FontFamily::SansSerif => 1,
        FontFamily::Serif => 2,
        FontFamily::Monospace => 3,
        FontFamily::Named(name) => name
            .as_bytes()
            .iter()
            .fold(4_u64, |hash, byte| fnv_mix(hash, *byte as u64)),
    }
}

#[cfg(feature = "text-cosmic")]
fn font_style_key(style: FontStyle) -> u64 {
    match style {
        FontStyle::Normal => 1,
        FontStyle::Italic => 2,
        FontStyle::Oblique => 3,
    }
}

#[cfg(feature = "text-cosmic")]
fn font_stretch_key(stretch: FontStretch) -> u64 {
    match stretch {
        FontStretch::Condensed => 1,
        FontStretch::Normal => 2,
        FontStretch::Expanded => 3,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    Next,
    Previous,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiWheelEvent {
    pub position: UiPoint,
    pub delta: UiPoint,
    pub unit: input::WheelDeltaUnit,
    pub phase: input::WheelPhase,
}

impl UiWheelEvent {
    pub const fn pixels(position: UiPoint, delta: UiPoint) -> Self {
        Self {
            position,
            delta,
            unit: input::WheelDeltaUnit::Pixel,
            phase: input::WheelPhase::Moved,
        }
    }

    pub const fn unit(mut self, unit: input::WheelDeltaUnit) -> Self {
        self.unit = unit;
        self
    }

    pub const fn phase(mut self, phase: input::WheelPhase) -> Self {
        self.phase = phase;
        self
    }

    pub const fn scrolls_document(self) -> bool {
        matches!(
            self.phase,
            input::WheelPhase::Moved | input::WheelPhase::Momentum
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiInputEvent {
    PointerMove(UiPoint),
    PointerDown(UiPoint),
    PointerUp(UiPoint),
    Wheel(UiWheelEvent),
    TextInput(String),
    Key {
        key: KeyCode,
        modifiers: KeyModifiers,
    },
    Focus(FocusDirection),
}

impl UiInputEvent {
    pub const fn wheel(position: UiPoint, delta: UiPoint) -> Self {
        Self::Wheel(UiWheelEvent::pixels(position, delta))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl KeyModifiers {
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
        meta: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Character(char),
    Backspace,
    Delete,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    Enter,
    Escape,
    Tab,
    F10,
    ContextMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditPhase {
    Preview,
    BeginEdit,
    UpdateEdit,
    CommitEdit,
    CancelEdit,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiInputResult {
    /// Topmost pointer-interactive node under the latest pointer position.
    pub hovered: Option<UiNodeId>,
    /// Node that owns keyboard focus after the input is handled.
    pub focused: Option<UiNodeId>,
    /// Node currently pressed by an active pointer interaction.
    pub pressed: Option<UiNodeId>,
    /// Node activated by a completed pointer click.
    pub clicked: Option<UiNodeId>,
    /// Scroll container whose offset changed because of a wheel event.
    pub scrolled: Option<UiNodeId>,
    /// The input was intentionally handled by the document.
    ///
    /// This can be true even when no widget state changed. For example, a
    /// visually topmost non-scrollable panel consumes wheel input so that a
    /// lower scroll container cannot move behind it.
    pub consumed: bool,
    /// Node that consumed the input, when the document can identify one.
    pub consumed_by: Option<UiNodeId>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct WheelInputResult {
    scrolled: Option<UiNodeId>,
    consumed_by: Option<UiNodeId>,
}

#[derive(Debug, Clone, Default)]
pub struct UiFocusState {
    pub hovered: Option<UiNodeId>,
    pub focused: Option<UiNodeId>,
    pub pressed: Option<UiNodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LayoutCacheKey {
    width_bits: u32,
    height_bits: u32,
    ui_scale_bits: u32,
    revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiDocumentScale {
    pub ui_scale: f32,
    pub dpi_scale: f32,
}

impl UiDocumentScale {
    pub const DEFAULT: Self = Self {
        ui_scale: 1.0,
        dpi_scale: 1.0,
    };

    pub fn new(ui_scale: f32, dpi_scale: f32) -> Self {
        Self {
            ui_scale: normalized_scale(ui_scale),
            dpi_scale: normalized_scale(dpi_scale),
        }
    }

    pub fn effective_scale(self) -> f32 {
        normalized_scale(self.ui_scale) * normalized_scale(self.dpi_scale)
    }
}

impl Default for UiDocumentScale {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug)]
pub struct UiDocument {
    pub root: UiNodeId,
    pub focus: UiFocusState,
    pub scale: UiDocumentScale,
    pub(crate) nodes: Vec<UiNode>,
    portal_hosts: HashMap<UiPortalId, UiNodeId>,
    layout_revision: u64,
    layout_cache_key: Option<LayoutCacheKey>,
}

impl UiDocument {
    pub fn new(root_style: impl Into<UiNodeStyle>) -> Self {
        Self::with_capacity(root_style, 1)
    }

    pub fn with_capacity(root_style: impl Into<UiNodeStyle>, capacity: usize) -> Self {
        let root_style = root_style.into();
        let root = UiNodeId(0);
        let mut nodes = Vec::with_capacity(capacity.max(1));
        nodes.push(UiNode::container("root", root_style));
        Self {
            root,
            nodes,
            focus: UiFocusState::default(),
            scale: UiDocumentScale::default(),
            portal_hosts: HashMap::new(),
            layout_revision: 0,
            layout_cache_key: None,
        }
    }

    pub fn with_scale(mut self, scale: UiDocumentScale) -> Self {
        self.set_scale(scale);
        self
    }

    pub fn set_scale(&mut self, scale: UiDocumentScale) {
        let scale = UiDocumentScale::new(scale.ui_scale, scale.dpi_scale);
        if self.scale != scale {
            self.scale = scale;
            self.invalidate_layout();
        }
    }

    pub fn set_ui_scale(&mut self, ui_scale: f32) {
        self.set_scale(UiDocumentScale::new(ui_scale, self.scale.dpi_scale));
    }

    pub fn set_dpi_scale(&mut self, dpi_scale: f32) {
        self.set_scale(UiDocumentScale::new(self.scale.ui_scale, dpi_scale));
    }

    pub fn ui_scale(&self) -> f32 {
        normalized_scale(self.scale.ui_scale)
    }

    pub fn dpi_scale(&self) -> f32 {
        normalized_scale(self.scale.dpi_scale)
    }

    pub fn effective_scale(&self) -> f32 {
        self.scale.effective_scale()
    }

    pub fn add_child(&mut self, parent: UiNodeId, mut node: UiNode) -> UiNodeId {
        assert_valid_node_id("UiDocument::add_child parent", parent, self.nodes.len());
        self.invalidate_layout();
        let id = UiNodeId(self.nodes.len());
        node.parent = Some(parent);
        self.nodes.push(node);
        self.nodes[parent.0].children.push(id);
        id
    }

    /// Registers an existing node as a portal host for future portal children.
    pub fn register_portal_host(
        &mut self,
        id: impl Into<UiPortalId>,
        host: UiNodeId,
    ) -> Option<UiNodeId> {
        self.portal_hosts.insert(id.into(), host)
    }

    /// Returns the node currently registered as a named portal host.
    pub fn portal_host(&self, id: impl Into<UiPortalId>) -> Option<UiNodeId> {
        self.portal_hosts.get(&id.into()).copied()
    }

    /// Adds a child to a portal target, falling back to `parent` when needed.
    pub fn add_portal_child(
        &mut self,
        parent: UiNodeId,
        target: UiPortalTarget,
        node: UiNode,
    ) -> UiNodeId {
        let portal_parent = match target {
            UiPortalTarget::Parent => parent,
            UiPortalTarget::AppOverlay => self.ensure_app_overlay_portal(),
            UiPortalTarget::Named(id) => self.portal_hosts.get(&id).copied().unwrap_or(parent),
        };
        self.add_child(portal_parent, node)
    }

    /// Ensures the built-in viewport-sized app overlay portal exists.
    pub fn ensure_app_overlay_portal(&mut self) -> UiNodeId {
        let portal_id = UiPortalId::from(APP_OVERLAY_PORTAL);
        if let Some(host) = self.portal_hosts.get(&portal_id).copied() {
            if self.nodes.get(host.0).is_some() {
                return host;
            }
        }

        let mut style = Style {
            display: Display::Flex,
            position: taffy::prelude::Position::Absolute,
            inset: taffy::prelude::Rect::length(0.0),
            size: TaffySize {
                width: Dimension::percent(1.0),
                height: Dimension::percent(1.0),
            },
            ..Default::default()
        };
        style.flex_direction = FlexDirection::Column;

        let host = self.add_child(
            self.root,
            UiNode::container(
                "portal.app_overlay",
                UiNodeStyle {
                    layout: style,
                    clip: ClipBehavior::None,
                    ..Default::default()
                },
            )
            .with_layer(platform::UiLayer::AppOverlay)
            .with_clip_scope(ClipScope::Viewport)
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).hidden()),
        );
        self.portal_hosts.insert(portal_id, host);
        host
    }

    pub fn node(&self, id: UiNodeId) -> &UiNode {
        self.nodes
            .get(id.0)
            .unwrap_or_else(|| invalid_node_id_panic("UiDocument::node", id, self.nodes.len()))
    }

    pub fn nodes(&self) -> &[UiNode] {
        &self.nodes
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn node_mut(&mut self, id: UiNodeId) -> &mut UiNode {
        assert_valid_node_id("UiDocument::node_mut", id, self.nodes.len());
        self.invalidate_layout();
        &mut self.nodes[id.0]
    }

    fn node_mut_without_invalidation(
        &mut self,
        operation: &'static str,
        id: UiNodeId,
    ) -> &mut UiNode {
        let len = self.nodes.len();
        self.nodes
            .get_mut(id.0)
            .unwrap_or_else(|| invalid_node_id_panic(operation, id, len))
    }

    pub fn edit_node(&mut self, id: UiNodeId, edit: impl FnOnce(&mut UiNode)) {
        edit(self.node_mut_without_invalidation("UiDocument::edit_node", id));
        self.invalidate_layout();
    }

    pub fn set_node_style(&mut self, id: UiNodeId, style: impl Into<UiNodeStyle>) {
        self.node_mut_without_invalidation("UiDocument::set_node_style", id)
            .style = style.into();
        self.invalidate_layout();
    }

    pub fn set_node_content(&mut self, id: UiNodeId, content: UiContent) {
        self.node_mut_without_invalidation("UiDocument::set_node_content", id)
            .content = content;
        self.invalidate_layout();
    }

    pub fn apply_localization_policy(&mut self, policy: &LocalizationPolicy) {
        for node in &mut self.nodes {
            if let UiContent::Text(text) = &mut node.content {
                if let Some(label) = text.dynamic_label.clone() {
                    let style = text.style.clone();
                    *text = TextContent::new(label.fallback.clone(), style)
                        .with_dynamic_label(label, Some(policy));
                } else {
                    text.locale = Some(policy.locale.clone());
                    text.direction = policy.resolved_direction();
                    text.bidi = policy.bidi;
                }
            }
        }
        self.invalidate_layout();
    }

    pub fn set_node_input(&mut self, id: UiNodeId, input: InputBehavior) {
        self.node_mut_without_invalidation("UiDocument::set_node_input", id)
            .input = input;
    }

    pub fn set_node_visual(&mut self, id: UiNodeId, visual: UiVisual) {
        self.node_mut_without_invalidation("UiDocument::set_node_visual", id)
            .visual = visual;
    }

    pub fn set_node_action(
        &mut self,
        id: UiNodeId,
        action: impl Into<actions::WidgetActionBinding>,
    ) {
        self.node_mut_without_invalidation("UiDocument::set_node_action", id)
            .action = Some(action.into());
    }

    pub fn set_node_interaction_visuals(&mut self, id: UiNodeId, visuals: InteractionVisuals) {
        self.node_mut_without_invalidation("UiDocument::set_node_interaction_visuals", id)
            .interaction_visuals = Some(visuals);
        self.refresh_interaction_visual(id);
    }

    pub fn clear_node_interaction_visuals(&mut self, id: UiNodeId) {
        self.node_mut_without_invalidation("UiDocument::clear_node_interaction_visuals", id)
            .interaction_visuals = None;
    }

    pub fn set_focus_state(&mut self, focus: UiFocusState) {
        self.sanitize_focus_state();
        let previous_hovered = self.focus.hovered;
        let previous_pressed = self.focus.pressed;
        let previous_focused = self.focus.focused;
        self.focus = self.sanitized_focus_state(focus);
        self.sync_interaction_animation_inputs_for(
            previous_hovered,
            previous_pressed,
            previous_focused,
            None,
            None,
        );
        self.refresh_interaction_visuals_for(previous_hovered, previous_pressed, previous_focused);
    }

    fn sanitize_focus_state(&mut self) {
        self.focus = self.sanitized_focus_state(self.focus.clone());
    }

    fn sanitized_focus_state(&self, focus: UiFocusState) -> UiFocusState {
        UiFocusState {
            hovered: self.valid_node_id(focus.hovered),
            focused: self.valid_node_id(focus.focused),
            pressed: self.valid_node_id(focus.pressed),
        }
    }

    fn valid_node_id(&self, id: Option<UiNodeId>) -> Option<UiNodeId> {
        id.filter(|id| id.0 < self.nodes.len())
    }

    fn refresh_interaction_visuals_for(
        &mut self,
        previous_hovered: Option<UiNodeId>,
        previous_pressed: Option<UiNodeId>,
        previous_focused: Option<UiNodeId>,
    ) {
        let ids = [
            previous_hovered,
            previous_pressed,
            previous_focused,
            self.focus.hovered,
            self.focus.pressed,
            self.focus.focused,
        ];
        for index in 0..ids.len() {
            let Some(id) = ids[index] else {
                continue;
            };
            if ids[..index].contains(&Some(id)) {
                continue;
            }
            self.refresh_interaction_visual(id);
        }
    }

    fn refresh_interaction_visual(&mut self, id: UiNodeId) {
        let Some(node) = self.nodes.get_mut(id.0) else {
            return;
        };
        let enabled = node.input.pointer || node.input.focusable || node.input.keyboard;
        let hovered = self.focus.hovered == Some(id);
        let pressed = self.focus.pressed == Some(id);
        let focused = self.focus.focused == Some(id);
        if let Some(visuals) = node.interaction_visuals {
            node.visual = visuals.resolve(enabled, hovered, pressed, focused);
        }
        if let (Some(styles), UiContent::Text(text)) =
            (node.interaction_text_styles.as_ref(), &mut node.content)
        {
            text.style = styles.resolve(enabled, hovered, pressed, focused);
        }
    }

    pub fn scroll_state(&self, id: UiNodeId) -> Option<ScrollState> {
        self.nodes.get(id.0).and_then(|node| node.scroll)
    }

    pub fn set_scroll_offset(&mut self, id: UiNodeId, offset: UiPoint) -> bool {
        let Some(node) = self.nodes.get_mut(id.0) else {
            return false;
        };
        let Some(scroll) = &mut node.scroll else {
            return false;
        };
        let offset = scroll.clamp_offset(offset);
        if scroll.offset == offset {
            return false;
        }
        scroll.offset = offset;
        self.invalidate_layout();
        true
    }

    pub fn clamp_scroll_offsets(&mut self) -> bool {
        let changed = self.clamp_scroll_offsets_in_place();
        if changed {
            self.invalidate_layout();
        }
        changed
    }

    fn clamp_scroll_offsets_in_place(&mut self) -> bool {
        let mut changed = false;
        for node in &mut self.nodes {
            let Some(scroll) = &mut node.scroll else {
                continue;
            };
            let offset = scroll.clamp_offset(scroll.offset);
            if scroll.offset != offset {
                scroll.offset = offset;
                changed = true;
            }
        }
        changed
    }

    pub fn scroll_by(&mut self, id: UiNodeId, delta: UiPoint) -> bool {
        let Some(scroll) = self.scroll_state(id) else {
            return false;
        };
        self.set_scroll_offset(
            id,
            UiPoint::new(scroll.offset.x + delta.x, scroll.offset.y + delta.y),
        )
    }

    pub fn scroll_to_node(&mut self, scroll_node: UiNodeId, target: UiNodeId) -> bool {
        let Some(target_node) = self.nodes.get(target.0) else {
            return false;
        };
        self.scroll_rect_into_view(scroll_node, target_node.layout.rect)
    }

    pub fn scroll_rect_into_view(&mut self, scroll_node: UiNodeId, target_rect: UiRect) -> bool {
        let Some(scroll) = self.scroll_state(scroll_node) else {
            return false;
        };
        let Some(scroll_node_ref) = self.nodes.get(scroll_node.0) else {
            return false;
        };
        let viewport = scroll_node_ref.layout.rect;
        let mut offset = scroll.offset;
        if scroll.axes.horizontal {
            if target_rect.x < viewport.x {
                offset.x -= viewport.x - target_rect.x;
            } else if target_rect.right() > viewport.right() {
                offset.x += target_rect.right() - viewport.right();
            }
        }
        if scroll.axes.vertical {
            if target_rect.y < viewport.y {
                offset.y -= viewport.y - target_rect.y;
            } else if target_rect.bottom() > viewport.bottom() {
                offset.y += target_rect.bottom() - viewport.bottom();
            }
        }
        self.set_scroll_offset(scroll_node, offset)
    }

    pub fn invalidate_layout(&mut self) {
        self.layout_revision = self.layout_revision.wrapping_add(1);
        self.layout_cache_key = None;
    }

    pub fn compute_layout(
        &mut self,
        viewport: UiSize,
        text_measurer: &mut impl TextMeasurer,
    ) -> Result<(), taffy::TaffyError> {
        let cache_key = LayoutCacheKey {
            width_bits: viewport.width.to_bits(),
            height_bits: viewport.height.to_bits(),
            ui_scale_bits: self.ui_scale().to_bits(),
            revision: self.layout_revision,
        };
        if self.layout_cache_key == Some(cache_key) {
            return Ok(());
        }
        let sizing = self.compute_layout_sizing_pass(viewport, text_measurer)?;
        self.apply_layout_position_pass(&sizing, viewport)?;
        if self.clamp_scroll_offsets_in_place() {
            self.apply_layout_position_pass(&sizing, viewport)?;
        }
        self.layout_cache_key = Some(cache_key);
        Ok(())
    }

    fn compute_layout_sizing_pass(
        &mut self,
        viewport: UiSize,
        text_measurer: &mut impl TextMeasurer,
    ) -> Result<LayoutSizingPass, taffy::TaffyError> {
        self.resolve_layout_constraints(text_measurer)?;
        let mut taffy = TaffyTree::<MeasureContext>::new();
        let mut mapping = vec![None; self.nodes.len()];
        let root = self.build_taffy_subtree(self.root, &mut taffy, &mut mapping)?;
        let mut measured_content = vec![None; self.nodes.len()];
        taffy.compute_layout_with_measure(
            root,
            TaffySize {
                width: AvailableSpace::Definite(viewport.width),
                height: AvailableSpace::Definite(viewport.height),
            },
            |known, available, _node_id, context, _style| {
                let Some(MeasureContext::Text { node, text }) = context else {
                    return TaffySize::ZERO;
                };
                let measured = measure_taffy_text(text_measurer, text, known, available);
                if let Some(slot) = measured_content.get_mut(node.0) {
                    *slot = Some(measured);
                }
                TaffySize {
                    width: measured.width,
                    height: measured.height,
                }
            },
        )?;
        let mut sizes = vec![UiSize::ZERO; self.nodes.len()];
        for (index, taffy_node) in mapping.iter().enumerate() {
            let Some(taffy_node) = *taffy_node else {
                continue;
            };
            let layout = taffy.layout(taffy_node)?;
            sizes[index] = UiSize::new(layout.size.width, layout.size.height);
        }

        Ok(LayoutSizingPass {
            root,
            taffy,
            mapping,
            measured_content,
            sizes,
        })
    }

    fn apply_layout_position_pass(
        &mut self,
        sizing: &LayoutSizingPass,
        viewport: UiSize,
    ) -> Result<(), taffy::TaffyError> {
        let viewport_rect = UiRect::new(0.0, 0.0, viewport.width, viewport.height);
        self.apply_layout_position_subtree(
            self.root,
            sizing.root,
            &sizing.taffy,
            UiPoint::new(0.0, 0.0),
            viewport_rect,
            viewport_rect,
            &sizing.mapping,
            &sizing.measured_content,
            &sizing.sizes,
        )?;
        Ok(())
    }

    pub fn intrinsic_size(
        &self,
        id: UiNodeId,
        text_measurer: &mut impl TextMeasurer,
    ) -> Result<IntrinsicSize, taffy::TaffyError> {
        Ok(IntrinsicSize {
            min: self.intrinsic_size_for_available_space(
                id,
                AvailableSpace::MinContent,
                text_measurer,
            )?,
            preferred: self.intrinsic_preferred_size(id, text_measurer),
        })
    }

    fn intrinsic_size_for_available_space(
        &self,
        id: UiNodeId,
        width: AvailableSpace,
        text_measurer: &mut impl TextMeasurer,
    ) -> Result<UiSize, taffy::TaffyError> {
        if width == AvailableSpace::MinContent {
            return Ok(self.intrinsic_min_size(id, text_measurer));
        }
        if width == AvailableSpace::MaxContent {
            return Ok(self.intrinsic_preferred_size(id, text_measurer));
        }
        if let Some(size) =
            self.fast_leaf_intrinsic_size_for_available_space(id, width, text_measurer)
        {
            return Ok(size);
        }
        let mut taffy = TaffyTree::<MeasureContext>::new();
        let mut mapping = vec![None; self.nodes.len()];
        let mode = match width {
            AvailableSpace::MinContent => IntrinsicMeasureMode::Min,
            AvailableSpace::Definite(_) | AvailableSpace::MaxContent => {
                IntrinsicMeasureMode::Preferred
            }
        };
        let root = self.build_taffy_subtree_for_intrinsic(
            id,
            &mut taffy,
            &mut mapping,
            Some(id),
            Some(mode),
        )?;
        taffy.compute_layout_with_measure(
            root,
            TaffySize {
                width,
                height: AvailableSpace::MaxContent,
            },
            |known, available, _node_id, context, _style| {
                let Some(MeasureContext::Text { text, .. }) = context else {
                    return TaffySize::ZERO;
                };
                let measured = measure_taffy_text(text_measurer, text, known, available);
                TaffySize {
                    width: measured.width,
                    height: measured.height,
                }
            },
        )?;
        let layout = taffy.layout(root)?;
        let scale = normalized_scale(self.ui_scale());
        Ok(UiSize::new(
            layout.size.width / scale,
            layout.size.height / scale,
        ))
    }

    fn intrinsic_min_size(&self, id: UiNodeId, text_measurer: &mut impl TextMeasurer) -> UiSize {
        let scale = normalized_scale(self.ui_scale());
        let size = self.intrinsic_min_size_scaled(id, text_measurer, scale, None);
        UiSize::new(size.width / scale, size.height / scale)
    }

    fn intrinsic_min_size_for_width(
        &self,
        id: UiNodeId,
        width: f32,
        text_measurer: &mut impl TextMeasurer,
    ) -> UiSize {
        let scale = normalized_scale(self.ui_scale());
        let allocated_width = (width * scale).is_finite().then_some(width * scale);
        let size = self.intrinsic_min_size_scaled(id, text_measurer, scale, allocated_width);
        UiSize::new(size.width / scale, size.height / scale)
    }

    fn intrinsic_preferred_size(
        &self,
        id: UiNodeId,
        text_measurer: &mut impl TextMeasurer,
    ) -> UiSize {
        let scale = normalized_scale(self.ui_scale());
        let size = self.intrinsic_preferred_size_scaled(id, text_measurer, scale, None);
        UiSize::new(size.width / scale, size.height / scale)
    }

    fn intrinsic_min_size_scaled(
        &self,
        id: UiNodeId,
        text_measurer: &mut impl TextMeasurer,
        scale: f32,
        allocated_width: Option<f32>,
    ) -> UiSize {
        let Some(node) = self.nodes.get(id.0) else {
            return UiSize::ZERO;
        };
        let style = scaled_taffy_style(&node.style.layout, scale);
        if style.display == Display::None {
            return UiSize::ZERO;
        }

        let mut content_size = if node.children.is_empty() {
            self.intrinsic_leaf_min_size_scaled(node, &style, text_measurer, scale, allocated_width)
        } else {
            self.intrinsic_children_min_size_scaled(
                node,
                &style,
                text_measurer,
                scale,
                allocated_width,
            )
        };

        if let Some(width) = dimension_points(style.size.width).or(allocated_width) {
            content_size.width = content_size.width.max(width);
        }
        if let Some(height) = dimension_points(style.size.height) {
            content_size.height = content_size.height.max(height);
        }
        content_size.width = constrain_intrinsic_extent(
            content_size.width,
            style.min_size.width,
            style.max_size.width,
        );
        content_size.height = constrain_intrinsic_extent(
            content_size.height,
            style.min_size.height,
            style.max_size.height,
        );
        content_size
    }

    fn intrinsic_preferred_size_scaled(
        &self,
        id: UiNodeId,
        text_measurer: &mut impl TextMeasurer,
        scale: f32,
        allocated_width: Option<f32>,
    ) -> UiSize {
        let Some(node) = self.nodes.get(id.0) else {
            return UiSize::ZERO;
        };
        let style = scaled_taffy_style(&node.style.layout, scale);
        if style.display == Display::None {
            return UiSize::ZERO;
        }

        let mut content_size = if node.children.is_empty() {
            self.intrinsic_leaf_preferred_size_scaled(
                node,
                &style,
                text_measurer,
                scale,
                allocated_width,
            )
        } else {
            self.intrinsic_children_preferred_size_scaled(
                node,
                &style,
                text_measurer,
                scale,
                allocated_width,
            )
        };

        if let Some(width) = dimension_points(style.size.width).or(allocated_width) {
            content_size.width = content_size.width.max(width);
        }
        if let Some(height) = dimension_points(style.size.height) {
            content_size.height = content_size.height.max(height);
        }
        content_size.width = constrain_intrinsic_extent(
            content_size.width,
            style.min_size.width,
            style.max_size.width,
        );
        content_size.height = constrain_intrinsic_extent(
            content_size.height,
            style.min_size.height,
            style.max_size.height,
        );
        content_size
    }

    fn intrinsic_leaf_min_size_scaled(
        &self,
        node: &UiNode,
        style: &Style,
        text_measurer: &mut impl TextMeasurer,
        scale: f32,
        allocated_width: Option<f32>,
    ) -> UiSize {
        let known = TaffySize {
            width: dimension_points(style.size.width).or(allocated_width),
            height: dimension_points(style.size.height),
        };
        match &node.content {
            UiContent::Text(text) => {
                let text = scaled_text_content(text, scale);
                measure_taffy_text(
                    text_measurer,
                    &text,
                    known,
                    TaffySize {
                        width: known
                            .width
                            .map_or(AvailableSpace::MinContent, AvailableSpace::Definite),
                        height: AvailableSpace::MaxContent,
                    },
                )
            }
            UiContent::PaintRect(rect) => paint_rect_intrinsic_size(rect, scale),
            UiContent::Scene(primitives) => scene_primitives_intrinsic_size(primitives, scale),
            UiContent::Empty | UiContent::Canvas(_) | UiContent::Image(_) => {
                UiSize::new(known.width.unwrap_or(0.0), known.height.unwrap_or(0.0))
            }
        }
    }

    fn intrinsic_leaf_preferred_size_scaled(
        &self,
        node: &UiNode,
        style: &Style,
        text_measurer: &mut impl TextMeasurer,
        scale: f32,
        allocated_width: Option<f32>,
    ) -> UiSize {
        let known = TaffySize {
            width: dimension_points(style.size.width).or(allocated_width),
            height: dimension_points(style.size.height),
        };
        match &node.content {
            UiContent::Text(text) => {
                let text = scaled_text_content(text, scale);
                measure_taffy_text(
                    text_measurer,
                    &text,
                    known,
                    TaffySize {
                        width: known
                            .width
                            .map_or(AvailableSpace::MaxContent, AvailableSpace::Definite),
                        height: AvailableSpace::MaxContent,
                    },
                )
            }
            UiContent::PaintRect(rect) => paint_rect_intrinsic_size(rect, scale),
            UiContent::Scene(primitives) => scene_primitives_intrinsic_size(primitives, scale),
            UiContent::Empty | UiContent::Canvas(_) | UiContent::Image(_) => {
                UiSize::new(known.width.unwrap_or(0.0), known.height.unwrap_or(0.0))
            }
        }
    }

    fn intrinsic_children_min_size_scaled(
        &self,
        node: &UiNode,
        style: &Style,
        text_measurer: &mut impl TextMeasurer,
        scale: f32,
        allocated_width: Option<f32>,
    ) -> UiSize {
        let spacing_width = box_horizontal_spacing(style, scale);
        let spacing_height = box_vertical_spacing(style, scale);
        let content_width = dimension_points(style.size.width)
            .or(allocated_width)
            .map(|width| (width - spacing_width).max(0.0));
        let children = node
            .children
            .iter()
            .filter_map(|child| {
                let child_node = self.nodes.get(child.0)?;
                if child_node.style.layout.display == Display::None
                    || child_node.style.layout.position == taffy::prelude::Position::Absolute
                {
                    return None;
                }
                let child_style = scaled_taffy_style(&child_node.style.layout, scale);
                let child_width = resolve_intrinsic_child_width(&child_style, content_width);
                let child_size =
                    self.intrinsic_min_size_scaled(*child, text_measurer, scale, child_width);
                Some(outer_intrinsic_size(child_size, &child_style))
            })
            .collect::<Vec<_>>();

        if children.is_empty() {
            return UiSize::new(spacing_width, spacing_height);
        }

        let gap_count = children.len().saturating_sub(1) as f32;
        match style.flex_direction {
            FlexDirection::Row | FlexDirection::RowReverse => UiSize::new(
                children.iter().map(|child| child.width).sum::<f32>()
                    + length_percentage_points(style.gap.width, 0.0, 1.0) * gap_count
                    + spacing_width,
                children
                    .iter()
                    .map(|child| child.height)
                    .fold(0.0, f32::max)
                    + spacing_height,
            ),
            FlexDirection::Column | FlexDirection::ColumnReverse => UiSize::new(
                children.iter().map(|child| child.width).fold(0.0, f32::max) + spacing_width,
                children.iter().map(|child| child.height).sum::<f32>()
                    + length_percentage_points(style.gap.height, 0.0, 1.0) * gap_count
                    + spacing_height,
            ),
        }
    }

    fn intrinsic_children_preferred_size_scaled(
        &self,
        node: &UiNode,
        style: &Style,
        text_measurer: &mut impl TextMeasurer,
        scale: f32,
        allocated_width: Option<f32>,
    ) -> UiSize {
        let spacing_width = box_horizontal_spacing(style, scale);
        let spacing_height = box_vertical_spacing(style, scale);
        let content_width = dimension_points(style.size.width)
            .or(allocated_width)
            .map(|width| (width - spacing_width).max(0.0));
        let children = node
            .children
            .iter()
            .filter_map(|child| {
                let child_node = self.nodes.get(child.0)?;
                if child_node.style.layout.display == Display::None
                    || child_node.style.layout.position == taffy::prelude::Position::Absolute
                {
                    return None;
                }
                let child_style = scaled_taffy_style(&child_node.style.layout, scale);
                let child_width = resolve_intrinsic_child_width(&child_style, content_width);
                let child_size =
                    self.intrinsic_preferred_size_scaled(*child, text_measurer, scale, child_width);
                Some(outer_intrinsic_size(child_size, &child_style))
            })
            .collect::<Vec<_>>();

        if children.is_empty() {
            return UiSize::new(spacing_width, spacing_height);
        }

        let gap_count = children.len().saturating_sub(1) as f32;
        match style.flex_direction {
            FlexDirection::Row | FlexDirection::RowReverse => UiSize::new(
                children.iter().map(|child| child.width).sum::<f32>()
                    + length_percentage_points(style.gap.width, 0.0, 1.0) * gap_count
                    + spacing_width,
                children
                    .iter()
                    .map(|child| child.height)
                    .fold(0.0, f32::max)
                    + spacing_height,
            ),
            FlexDirection::Column | FlexDirection::ColumnReverse => UiSize::new(
                children.iter().map(|child| child.width).fold(0.0, f32::max) + spacing_width,
                children.iter().map(|child| child.height).sum::<f32>()
                    + length_percentage_points(style.gap.height, 0.0, 1.0) * gap_count
                    + spacing_height,
            ),
        }
    }

    fn fast_leaf_intrinsic_size_for_available_space(
        &self,
        id: UiNodeId,
        width: AvailableSpace,
        text_measurer: &mut impl TextMeasurer,
    ) -> Option<UiSize> {
        let node = self.nodes.get(id.0)?;
        if !node.children.is_empty() {
            return None;
        }
        let scale = normalized_scale(self.ui_scale());
        let mut style = scaled_taffy_style(&node.style.layout, scale);
        normalize_intrinsic_root_style(&mut style);
        let known = TaffySize {
            width: dimension_points(style.size.width),
            height: dimension_points(style.size.height),
        };
        let available = TaffySize {
            width,
            height: AvailableSpace::MaxContent,
        };
        let mut size = match &node.content {
            UiContent::Text(text) => {
                let text = scaled_text_content(text, scale);
                measure_taffy_text(text_measurer, &text, known, available)
            }
            UiContent::PaintRect(rect) => paint_rect_intrinsic_size(rect, scale),
            UiContent::Scene(primitives) => scene_primitives_intrinsic_size(primitives, scale),
            UiContent::Empty | UiContent::Canvas(_) | UiContent::Image(_) => {
                UiSize::new(known.width.unwrap_or(0.0), known.height.unwrap_or(0.0))
            }
        };
        if let Some(width) = known.width {
            size.width = size.width.max(width);
        }
        if let Some(height) = known.height {
            size.height = size.height.max(height);
        }
        size.width =
            constrain_intrinsic_extent(size.width, style.min_size.width, style.max_size.width);
        size.height =
            constrain_intrinsic_extent(size.height, style.min_size.height, style.max_size.height);
        Some(UiSize::new(size.width / scale, size.height / scale))
    }

    fn resolve_layout_constraints(
        &mut self,
        text_measurer: &mut impl TextMeasurer,
    ) -> Result<(), taffy::TaffyError> {
        let constraints = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                node.layout_constraint
                    .clone()
                    .map(|constraint| (UiNodeId(index), constraint))
            })
            .collect::<Vec<_>>();

        for (target, constraint) in &constraints {
            if let UiNodeLayoutConstraint::InlineIntrinsicSize { sources, min_size } = constraint {
                let base_size = UiSize::new(
                    finite_or(min_size.width, 0.0),
                    finite_or(min_size.height, 0.0),
                );
                let mut min_row = UiSize::ZERO;
                let mut preferred_row = UiSize::ZERO;
                for source in sources {
                    if source.0 >= self.nodes.len() {
                        continue;
                    }
                    let source_size = self.intrinsic_size(*source, text_measurer)?;
                    min_row.width += source_size.min.width;
                    min_row.height = min_row.height.max(source_size.min.height);
                    preferred_row.width += source_size.preferred.width;
                    preferred_row.height = preferred_row.height.max(source_size.preferred.height);
                }
                let min_size = UiSize::new(
                    base_size.width + min_row.width,
                    base_size.height.max(min_row.height),
                );
                let preferred_size = UiSize::new(
                    base_size.width + preferred_row.width,
                    base_size.height.max(preferred_row.height),
                );
                self.apply_inline_intrinsic_size_constraint(*target, min_size, preferred_size);
            }
        }

        for (target, constraint) in constraints {
            if let UiNodeLayoutConstraint::StackedIntrinsicSize {
                sources,
                min_size,
                bounds,
                fit_to_preferred,
            } = constraint
            {
                let mut min_stack = UiSize::ZERO;
                let mut preferred_stack = UiSize::ZERO;
                let mut valid_sources = Vec::new();
                for source in &sources {
                    if source.0 >= self.nodes.len() {
                        continue;
                    }
                    valid_sources.push(source);
                    if fit_to_preferred {
                        let source_size = self.intrinsic_size(*source, text_measurer)?;
                        min_stack.width = min_stack.width.max(source_size.min.width);
                        preferred_stack.width =
                            preferred_stack.width.max(source_size.preferred.width);
                        preferred_stack.height += source_size.preferred.height;
                    } else {
                        let source_size = self.intrinsic_size_for_available_space(
                            *source,
                            AvailableSpace::MinContent,
                            text_measurer,
                        )?;
                        min_stack.width = min_stack.width.max(source_size.width);
                    }
                }
                let constrained_min_width =
                    finite_or(min_size.width, 0.0).max(min_stack.width).max(1.0);
                for source in valid_sources {
                    let source_size = self.intrinsic_min_size_for_width(
                        *source,
                        constrained_min_width,
                        text_measurer,
                    );
                    min_stack.height += source_size.height;
                }
                let min_size = UiSize::new(
                    constrained_min_width,
                    finite_or(min_size.height, 0.0)
                        .max(min_stack.height)
                        .max(1.0),
                );
                let preferred_size = if fit_to_preferred {
                    UiSize::new(
                        preferred_stack.width.max(min_size.width),
                        preferred_stack.height.max(min_size.height),
                    )
                } else {
                    min_size
                };
                self.apply_intrinsic_size_constraint(
                    target,
                    min_size,
                    preferred_size,
                    bounds,
                    fit_to_preferred,
                );
            }
        }

        Ok(())
    }

    fn apply_inline_intrinsic_size_constraint(
        &mut self,
        target: UiNodeId,
        min_size: UiSize,
        preferred_size: UiSize,
    ) {
        let Some(node) = self.nodes.get_mut(target.0) else {
            return;
        };
        let min_size = ceil_layout_size(min_size);
        let preferred_size = ceil_layout_size(preferred_size);
        node.style.layout.min_size.width = max_length_dimension(
            node.style.layout.min_size.width,
            min_size.width.max(preferred_size.width).max(1.0),
        );
        node.style.layout.min_size.height = max_length_dimension(
            node.style.layout.min_size.height,
            min_size.height.max(preferred_size.height).max(1.0),
        );
        if node.style.layout.size.width == Dimension::auto() {
            node.style.layout.size.width = Dimension::length(preferred_size.width.max(1.0));
        }
        if node.style.layout.size.height == Dimension::auto() {
            node.style.layout.size.height = Dimension::length(preferred_size.height.max(1.0));
        }
    }

    fn apply_intrinsic_size_constraint(
        &mut self,
        target: UiNodeId,
        min_size: UiSize,
        preferred_size: UiSize,
        bounds: UiRect,
        fit_to_preferred: bool,
    ) {
        let Some(node) = self.nodes.get_mut(target.0) else {
            return;
        };
        let min_size = ceil_layout_size(min_size);
        let preferred_size = ceil_layout_size(preferred_size);
        let rect = absolute_rect_from_style(&node.style.layout).unwrap_or_else(|| {
            UiRect::new(0.0, 0.0, min_size.width.max(1.0), min_size.height.max(1.0))
        });
        let bounds = finite_rect(bounds);
        let size = if fit_to_preferred {
            preferred_size
        } else {
            UiSize::new(rect.width, rect.height)
        };
        let contained = crate::layout::contain_rect(
            UiRect::new(rect.x, rect.y, size.width, size.height),
            bounds,
            min_size,
        );
        node.style.layout.size = TaffySize {
            width: Dimension::length(contained.width),
            height: Dimension::length(contained.height),
        };
        node.style.layout.min_size = TaffySize {
            width: Dimension::length(min_size.width.min(bounds.width.max(1.0))),
            height: Dimension::length(min_size.height.min(bounds.height.max(1.0))),
        };
        node.style.layout.inset.left = LengthPercentageAuto::length(contained.x);
        node.style.layout.inset.top = LengthPercentageAuto::length(contained.y);
    }

    fn build_taffy_subtree(
        &self,
        id: UiNodeId,
        taffy: &mut TaffyTree<MeasureContext>,
        mapping: &mut [Option<TaffyNodeId>],
    ) -> Result<TaffyNodeId, taffy::TaffyError> {
        self.build_taffy_subtree_for_intrinsic(id, taffy, mapping, None, None)
    }

    fn build_taffy_subtree_for_intrinsic(
        &self,
        id: UiNodeId,
        taffy: &mut TaffyTree<MeasureContext>,
        mapping: &mut [Option<TaffyNodeId>],
        intrinsic_root: Option<UiNodeId>,
        intrinsic_mode: Option<IntrinsicMeasureMode>,
    ) -> Result<TaffyNodeId, taffy::TaffyError> {
        let node = &self.nodes[id.0];
        let layout_scale = self.ui_scale();
        let mut style = scaled_taffy_style(&node.style.layout, layout_scale);
        if intrinsic_root == Some(id) {
            normalize_intrinsic_root_style(&mut style);
        } else if intrinsic_root.is_some() {
            normalize_intrinsic_descendant_style(&mut style);
        }
        if intrinsic_mode == Some(IntrinsicMeasureMode::Min)
            && style.flex_direction == FlexDirection::Row
        {
            style.flex_wrap = FlexWrap::NoWrap;
        }
        let taffy_node = if node.children.is_empty() {
            match &node.content {
                UiContent::Text(text) => taffy.new_leaf_with_context(
                    style,
                    MeasureContext::Text {
                        node: id,
                        text: scaled_text_content(text, layout_scale),
                    },
                )?,
                UiContent::Empty
                | UiContent::Canvas(_)
                | UiContent::Image(_)
                | UiContent::PaintRect(_)
                | UiContent::Scene(_) => taffy.new_leaf(style)?,
            }
        } else {
            let children = node
                .children
                .iter()
                .map(|child| {
                    self.build_taffy_subtree_for_intrinsic(
                        *child,
                        taffy,
                        mapping,
                        intrinsic_root,
                        intrinsic_mode,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            taffy.new_with_children(style, &children)?
        };
        if let Some(slot) = mapping.get_mut(id.0) {
            *slot = Some(taffy_node);
        }
        Ok(taffy_node)
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_layout_position_subtree(
        &mut self,
        id: UiNodeId,
        taffy_node: TaffyNodeId,
        taffy: &TaffyTree<MeasureContext>,
        parent_origin: UiPoint,
        parent_clip: UiRect,
        viewport_clip: UiRect,
        mapping: &[Option<TaffyNodeId>],
        measured_content: &[Option<UiSize>],
        sizes: &[UiSize],
    ) -> Result<(), taffy::TaffyError> {
        let layout = taffy.layout(taffy_node)?;
        let resolved_size = sizes
            .get(id.0)
            .copied()
            .unwrap_or_else(|| UiSize::new(layout.size.width, layout.size.height));
        let mut rect = UiRect::new(
            parent_origin.x + layout.location.x,
            parent_origin.y + layout.location.y,
            resolved_size.width,
            resolved_size.height,
        );
        if matches!(self.nodes[id.0].content, UiContent::Canvas(_)) {
            if let Some(aspect_ratio) = self.nodes[id.0].style.layout.aspect_ratio {
                rect = aspect_fit_rect(rect, aspect_ratio);
            }
        }
        let has_scroll = self.nodes[id.0].scroll.is_some();
        let scroll_offset = self.nodes[id.0]
            .scroll
            .map(|scroll| scroll.offset)
            .unwrap_or(UiPoint::new(0.0, 0.0));
        let inherited_clip = match self.nodes[id.0].clip_scope {
            ClipScope::Parent => parent_clip,
            ClipScope::Viewport => viewport_clip,
        };
        let clip_rect = if has_scroll || self.nodes[id.0].style.clip == ClipBehavior::Clip {
            inherited_clip
                .intersection(rect)
                .unwrap_or(UiRect::new(rect.x, rect.y, 0.0, 0.0))
        } else {
            inherited_clip
        };
        self.nodes[id.0].layout = ComputedLayout {
            rect,
            clip_rect,
            visible: rect.intersects(inherited_clip),
            opacity: self.nodes[id.0].style.opacity,
            content_size: measured_content.get(id.0).copied().flatten(),
        };
        let children = self.nodes[id.0].children.clone();
        let child_origin = if has_scroll {
            UiPoint::new(rect.x - scroll_offset.x, rect.y - scroll_offset.y)
        } else {
            UiPoint::new(rect.x, rect.y)
        };
        for child in children {
            let Some(child_taffy) = mapping.get(child.0).copied().flatten() else {
                continue;
            };
            self.apply_layout_position_subtree(
                child,
                child_taffy,
                taffy,
                child_origin,
                clip_rect,
                viewport_clip,
                mapping,
                measured_content,
                sizes,
            )?;
        }
        if has_scroll {
            let mut content_size = UiSize::new(rect.width, rect.height);
            self.include_descendant_content_bounds(id, child_origin, &mut content_size);
            if let Some(scroll) = self.nodes[id.0].scroll.as_mut() {
                scroll.viewport_size = UiSize::new(rect.width, rect.height);
                scroll.content_size = content_size;
            }
        }
        Ok(())
    }

    fn include_descendant_content_bounds(
        &self,
        id: UiNodeId,
        content_origin: UiPoint,
        content_size: &mut UiSize,
    ) {
        for child in &self.nodes[id.0].children {
            let child_node = &self.nodes[child.0];
            if child_node.clip_scope == ClipScope::Viewport {
                continue;
            }
            let child_rect = child_node.layout.rect;
            if rect_is_finite(child_rect) {
                content_size.width = content_size
                    .width
                    .max(child_rect.right() - content_origin.x);
                content_size.height = content_size
                    .height
                    .max(child_rect.bottom() - content_origin.y);
            }
            if child_node.scroll.is_some() || child_node.style.clip == ClipBehavior::Clip {
                continue;
            }
            self.include_descendant_content_bounds(*child, content_origin, content_size);
        }
    }

    pub fn hit_test(&self, point: UiPoint) -> Option<UiNodeId> {
        let layer_orders = self.effective_layer_orders();
        let visual_order = self.visual_order_with_layer(&layer_orders);
        for (order, index) in visual_order.into_iter().enumerate().rev() {
            let geometry = self.effective_geometry_for_index(index, order, layer_orders[index]);
            if geometry.contains_point(point) {
                return Some(geometry.node);
            }
        }
        None
    }

    pub fn effective_geometries(&self) -> Vec<EffectiveGeometry> {
        let layer_orders = self.effective_layer_orders();
        self.visual_order_with_layer(&layer_orders)
            .into_iter()
            .enumerate()
            .map(|(order, index)| {
                self.effective_geometry_for_index(index, order, layer_orders[index])
            })
            .collect()
    }

    fn effective_geometry_for_index(
        &self,
        index: usize,
        order: usize,
        layer_order: platform::LayerOrder,
    ) -> EffectiveGeometry {
        let id = UiNodeId(index);
        let node = &self.nodes[index];
        EffectiveGeometry::new(id, node.layout.rect)
            .paint_transform(Self::node_paint_transform(node))
            .clip_rect(node.layout.clip_rect)
            .layer_order(layer_order)
            .order(order)
            .visible(node.layout.visible)
            .hit_testable(node.input.pointer)
            .accessibility_rect(node.layout.rect)
    }

    pub fn handle_input(&mut self, event: UiInputEvent) -> UiInputResult {
        self.sanitize_focus_state();
        let previous_hovered = self.focus.hovered;
        let previous_pressed = self.focus.pressed;
        let previous_focused = self.focus.focused;
        let mut scrolled = None;
        let mut pointer = None;
        let mut consumed_by = None;
        let clicked = match event {
            UiInputEvent::PointerMove(point) => {
                pointer = Some(point);
                self.focus.hovered = self.hit_test(point);
                consumed_by = self.focus.hovered;
                None
            }
            UiInputEvent::PointerDown(point) => {
                pointer = Some(point);
                let hit = self.hit_test(point);
                self.focus.pressed = hit;
                if hit.is_some_and(|id| self.nodes[id.0].input.focusable) {
                    self.focus.focused = hit;
                }
                consumed_by = hit;
                None
            }
            UiInputEvent::PointerUp(point) => {
                pointer = Some(point);
                let hit = self.hit_test(point);
                let clicked = self.focus.pressed.filter(|pressed| Some(*pressed) == hit);
                self.focus.pressed = None;
                consumed_by = hit.or(clicked);
                clicked
            }
            UiInputEvent::Wheel(wheel) => {
                let wheel = self.apply_wheel_scroll(wheel);
                scrolled = wheel.scrolled;
                consumed_by = wheel.consumed_by;
                None
            }
            UiInputEvent::TextInput(_) | UiInputEvent::Key { .. } => None,
            UiInputEvent::Focus(direction) => {
                self.focus.focused = self.next_focus(self.focus.focused, direction);
                consumed_by = self.focus.focused;
                None
            }
        };
        self.sync_interaction_animation_inputs_for(
            previous_hovered,
            previous_pressed,
            previous_focused,
            pointer,
            clicked,
        );
        self.trigger_interaction_animations_for(
            previous_hovered,
            previous_pressed,
            previous_focused,
        );
        self.refresh_interaction_visuals_for(previous_hovered, previous_pressed, previous_focused);
        UiInputResult {
            hovered: self.focus.hovered,
            focused: self.focus.focused,
            pressed: self.focus.pressed,
            clicked,
            scrolled,
            consumed: consumed_by.is_some(),
            consumed_by,
        }
    }

    fn sync_interaction_animation_inputs_for(
        &mut self,
        previous_hovered: Option<UiNodeId>,
        previous_pressed: Option<UiNodeId>,
        previous_focused: Option<UiNodeId>,
        pointer: Option<UiPoint>,
        clicked: Option<UiNodeId>,
    ) {
        let ids = [
            previous_hovered,
            previous_pressed,
            previous_focused,
            self.focus.hovered,
            self.focus.pressed,
            self.focus.focused,
            clicked,
        ];
        for index in 0..ids.len() {
            let Some(id) = ids[index] else {
                continue;
            };
            if ids[..index].contains(&Some(id)) {
                continue;
            }
            let hovered = self.focus.hovered == Some(id);
            let pressed = self.focus.pressed == Some(id);
            let focused = self.focus.focused == Some(id);
            let active = hovered || pressed || focused;
            let Some(rect) = self.nodes.get(id.0).map(|node| node.layout.rect) else {
                continue;
            };
            if let Some(animation) = self
                .nodes
                .get_mut(id.0)
                .and_then(|node| node.animation.as_mut())
            {
                animation.set_bool_input(ANIMATION_INPUT_HOVER, hovered);
                animation.set_bool_input(ANIMATION_INPUT_PRESSED, pressed);
                animation.set_bool_input(ANIMATION_INPUT_FOCUSED, focused);
                animation.set_bool_input(ANIMATION_INPUT_ACTIVE, active);
                if clicked == Some(id) {
                    animation.fire_trigger_input(ANIMATION_INPUT_ACTIVATED);
                }
                let local_pointer = active
                    .then(|| {
                        pointer.map(|pointer| {
                            UiPoint::new(
                                (pointer.x - rect.x).clamp(0.0, rect.width.max(0.0)),
                                (pointer.y - rect.y).clamp(0.0, rect.height.max(0.0)),
                            )
                        })
                    })
                    .flatten();
                let local_x = local_pointer.map_or(0.0, |pointer| pointer.x);
                let local_y = local_pointer.map_or(0.0, |pointer| pointer.y);
                animation.set_number_input(ANIMATION_INPUT_POINTER_X, local_x);
                animation.set_number_input(ANIMATION_INPUT_POINTER_Y, local_y);
                animation.set_number_input(
                    ANIMATION_INPUT_POINTER_NORM_X,
                    local_x / rect.width.max(f32::EPSILON),
                );
                animation.set_number_input(
                    ANIMATION_INPUT_POINTER_NORM_Y,
                    local_y / rect.height.max(f32::EPSILON),
                );
            }
        }
    }

    fn trigger_interaction_animations_for(
        &mut self,
        previous_hovered: Option<UiNodeId>,
        previous_pressed: Option<UiNodeId>,
        previous_focused: Option<UiNodeId>,
    ) {
        let hovered = self.focus.hovered;
        if previous_hovered != hovered {
            if let Some(id) = previous_hovered {
                self.trigger_node_animation(id, AnimationTrigger::PointerLeave);
            }
            if let Some(id) = hovered {
                self.trigger_node_animation(id, AnimationTrigger::PointerEnter);
            }
        }

        let pressed = self.focus.pressed;
        if previous_pressed != pressed {
            if let Some(id) = previous_pressed {
                self.trigger_node_animation(id, AnimationTrigger::Released);
            }
            if let Some(id) = pressed {
                self.trigger_node_animation(id, AnimationTrigger::Pressed);
            }
        }

        let focused = self.focus.focused;
        if previous_focused != focused {
            if let Some(id) = previous_focused {
                self.trigger_node_animation(id, AnimationTrigger::FocusLost);
            }
            if let Some(id) = focused {
                self.trigger_node_animation(id, AnimationTrigger::FocusGained);
            }
        }
    }

    fn trigger_node_animation(&mut self, id: UiNodeId, trigger: AnimationTrigger) -> bool {
        self.nodes
            .get_mut(id.0)
            .and_then(|node| node.animation.as_mut())
            .is_some_and(|animation| animation.trigger(trigger))
    }

    fn apply_wheel_scroll(&mut self, wheel: UiWheelEvent) -> WheelInputResult {
        if !wheel.scrolls_document() {
            return WheelInputResult::default();
        }
        if let Some(scope) = self.wheel_event_scope(wheel.position) {
            let scrolled = self.scroll_wheel_from_scope(scope, wheel.position, wheel.delta);
            return WheelInputResult {
                scrolled,
                consumed_by: scrolled.or(Some(scope)),
            };
        }
        let scrolled = self.scroll_topmost_wheel_candidate(wheel.position, wheel.delta, None);
        WheelInputResult {
            scrolled,
            consumed_by: scrolled,
        }
    }

    fn scroll_wheel_from_scope(
        &mut self,
        scope: UiNodeId,
        position: UiPoint,
        delta: UiPoint,
    ) -> Option<UiNodeId> {
        if let Some(target) = self.scroll_topmost_wheel_candidate(position, delta, Some(scope)) {
            return Some(target);
        }

        let mut current = self.nodes.get(scope.0).and_then(|node| node.parent);
        while let Some(id) = current {
            if self.node_can_receive_wheel_scroll(id, position) && self.scroll_by(id, delta) {
                return Some(id);
            }
            current = self.nodes.get(id.0).and_then(|node| node.parent);
        }

        None
    }

    fn wheel_event_scope(&self, position: UiPoint) -> Option<UiNodeId> {
        self.visual_order()
            .into_iter()
            .rev()
            .map(UiNodeId)
            .find(|id| self.node_occludes_wheel_at(*id, position))
    }

    fn scroll_topmost_wheel_candidate(
        &mut self,
        position: UiPoint,
        delta: UiPoint,
        subtree_root: Option<UiNodeId>,
    ) -> Option<UiNodeId> {
        for index in self.visual_order().into_iter().rev() {
            let target = UiNodeId(index);
            if subtree_root.is_none_or(|root| self.node_is_descendant_or_self(root, target))
                && self.node_can_receive_wheel_scroll(target, position)
                && self.scroll_by(target, delta)
            {
                return Some(target);
            }
        }
        None
    }

    fn node_can_receive_wheel_scroll(&self, id: UiNodeId, position: UiPoint) -> bool {
        let Some(node) = self.nodes.get(id.0) else {
            return false;
        };
        node.layout.visible
            && node.layout.clip_rect.contains_point(position)
            && self.node_paint_rect(id.0).contains_point(position)
            && node
                .scroll
                .is_some_and(|scroll| scroll.axes.horizontal || scroll.axes.vertical)
    }

    fn node_occludes_wheel_at(&self, id: UiNodeId, position: UiPoint) -> bool {
        let Some(node) = self.nodes.get(id.0) else {
            return false;
        };
        node.layout.visible
            && node.layout.clip_rect.contains_point(position)
            && self.node_paint_rect(id.0).contains_point(position)
            && node_blocks_wheel_passthrough(node)
    }

    fn next_focus(&self, current: Option<UiNodeId>, direction: FocusDirection) -> Option<UiNodeId> {
        let focusable = self.focus_navigation_order();
        if focusable.is_empty() {
            return None;
        }
        let current_index =
            current.and_then(|id| focusable.iter().position(|candidate| *candidate == id));
        let next_index = match (direction, current_index) {
            (FocusDirection::Next, Some(index)) => (index + 1) % focusable.len(),
            (FocusDirection::Previous, Some(0)) => focusable.len() - 1,
            (FocusDirection::Previous, Some(index)) => index - 1,
            (_, None) => 0,
        };
        Some(focusable[next_index])
    }

    fn focus_navigation_order(&self) -> Vec<UiNodeId> {
        let accessibility = self.accessibility_snapshot();
        let mut focusable = Vec::new();
        for id in accessibility.effective_focus_order() {
            if self.is_focus_navigation_candidate(id, accessibility.modal_scope)
                && !focusable.contains(&id)
            {
                focusable.push(id);
            }
        }
        for index in 0..self.nodes.len() {
            let id = UiNodeId(index);
            if self.is_focus_navigation_candidate(id, accessibility.modal_scope)
                && !focusable.contains(&id)
            {
                focusable.push(id);
            }
        }
        focusable
    }

    fn is_focus_navigation_candidate(&self, id: UiNodeId, modal_scope: Option<UiNodeId>) -> bool {
        let Some(node) = self.nodes.get(id.0) else {
            return false;
        };
        if !node.layout.visible || !node.layout.rect.intersects(node.layout.clip_rect) {
            return false;
        }
        if let Some(accessibility) = &node.accessibility {
            if accessibility.hidden || !accessibility.enabled {
                return false;
            }
        }
        if let Some(scope) = modal_scope {
            if !self.node_is_descendant_or_self(scope, id) {
                return false;
            }
        }
        node.input.focusable
            || node
                .accessibility
                .as_ref()
                .is_some_and(|accessibility| accessibility.focusable)
    }

    pub(crate) fn node_is_descendant_or_self(&self, ancestor: UiNodeId, node: UiNodeId) -> bool {
        if ancestor == node {
            return self.nodes.get(node.0).is_some();
        }
        let mut current = self.nodes.get(node.0).and_then(|node| node.parent);
        while let Some(parent) = current {
            if parent == ancestor {
                return true;
            }
            current = self.nodes.get(parent.0).and_then(|node| node.parent);
        }
        false
    }

    pub fn trigger_animation(&mut self, id: UiNodeId, trigger: AnimationTrigger) -> bool {
        self.trigger_node_animation(id, trigger)
    }

    pub fn set_animation_input(
        &mut self,
        id: UiNodeId,
        name: impl Into<String>,
        value: AnimationInputValue,
    ) -> bool {
        self.nodes
            .get_mut(id.0)
            .and_then(|node| node.animation.as_mut())
            .is_some_and(|animation| animation.set_input(name, value))
    }

    pub fn set_animation_bool_input(
        &mut self,
        id: UiNodeId,
        name: impl Into<String>,
        value: bool,
    ) -> bool {
        self.set_animation_input(id, name, AnimationInputValue::bool(value))
    }

    pub fn set_animation_number_input(
        &mut self,
        id: UiNodeId,
        name: impl Into<String>,
        value: f32,
    ) -> bool {
        self.set_animation_input(id, name, AnimationInputValue::number(value))
    }

    pub fn fire_animation_trigger_input(&mut self, id: UiNodeId, name: impl Into<String>) -> bool {
        self.set_animation_input(id, name, AnimationInputValue::fired_trigger())
    }

    pub fn tick_animations(&mut self, dt_seconds: f32) -> AnimationTickReport {
        let mut report = AnimationTickReport::default();
        for node in &mut self.nodes {
            if let Some(animation) = &mut node.animation {
                report.ticked += 1;
                let outcome = animation.tick(dt_seconds);
                if outcome.advanced {
                    report.advanced += 1;
                }
                if outcome.active {
                    report.active += 1;
                }
                if outcome.completed {
                    report.completed += 1;
                }
            }
        }
        report
    }

    pub fn animations_active(&self) -> bool {
        self.nodes.iter().any(|node| {
            node.animation
                .as_ref()
                .is_some_and(AnimationMachine::is_animating)
        })
    }

    pub fn paint_list(&self) -> PaintList {
        let mut list = PaintList {
            items: Vec::with_capacity(self.nodes.len()),
        };
        let layer_orders = self.effective_layer_orders();
        for index in self.visual_order_with_layer(&layer_orders) {
            let id = UiNodeId(index);
            let node = &self.nodes[index];
            if !node.layout.visible
                || node.layout.clip_rect.width <= f32::EPSILON
                || node.layout.clip_rect.height <= f32::EPSILON
            {
                continue;
            }
            let layer_order = layer_orders[index];
            let z_index = layer_order.local_z;
            let animation_values = Self::node_animation_values(node);
            let opacity = node.layout.opacity * animation_values.opacity;
            let transform = Self::paint_transform_from_values(animation_values);
            if node.visual.fill.a > 0
                || node
                    .visual
                    .stroke
                    .is_some_and(|stroke| stroke.width > 0.0 && stroke.color.a > 0)
            {
                list.items.push(PaintItem {
                    node: id,
                    rect: node.layout.rect,
                    clip_rect: node.layout.clip_rect,
                    z_index,
                    layer_order,
                    opacity,
                    transform,
                    shader: node.shader.clone(),
                    kind: PaintKind::Rect {
                        fill: node.visual.fill,
                        stroke: node.visual.stroke,
                        corner_radius: node.visual.corner_radius,
                    },
                });
            }
            match &node.content {
                UiContent::Empty => {}
                UiContent::Text(text) => {
                    let text_rect =
                        text_content_rect(node.layout.rect, &node.style.layout, self.ui_scale());
                    list.items.push(PaintItem {
                        node: id,
                        rect: text_rect,
                        clip_rect: node.layout.clip_rect,
                        z_index,
                        layer_order,
                        opacity,
                        transform,
                        shader: node.shader.clone(),
                        kind: PaintKind::Text(scaled_text_content(text, self.ui_scale())),
                    });
                    if text.style.underline {
                        let underline = text_underline_segment(
                            text_rect,
                            node.layout.content_size,
                            text,
                            self.ui_scale(),
                        );
                        list.items.push(PaintItem {
                            node: id,
                            rect: node.layout.rect,
                            clip_rect: node.layout.clip_rect,
                            z_index,
                            layer_order,
                            opacity,
                            transform,
                            shader: node.shader.clone(),
                            kind: PaintKind::Line {
                                from: underline.0,
                                to: underline.1,
                                stroke: StrokeStyle::new(
                                    text.style.color,
                                    self.ui_scale().max(1.0),
                                ),
                            },
                        });
                    }
                }
                UiContent::Canvas(canvas) => list.items.push(PaintItem {
                    node: id,
                    rect: node.layout.rect,
                    clip_rect: node.layout.clip_rect,
                    z_index,
                    layer_order,
                    opacity,
                    transform,
                    shader: node.shader.clone(),
                    kind: PaintKind::Canvas(canvas.clone()),
                }),
                UiContent::Image(image) => list.items.push(PaintItem {
                    node: id,
                    rect: node.layout.rect,
                    clip_rect: node.layout.clip_rect,
                    z_index,
                    layer_order,
                    opacity,
                    transform,
                    shader: node.shader.clone(),
                    kind: PaintKind::Image {
                        key: image.key.clone(),
                        tint: image.tint,
                    },
                }),
                UiContent::PaintRect(rect) => list.items.push(PaintItem {
                    node: id,
                    rect: node.layout.rect,
                    clip_rect: node.layout.clip_rect,
                    z_index,
                    layer_order,
                    opacity,
                    transform,
                    shader: node.shader.clone(),
                    kind: PaintKind::RichRect(paint_rect_for_node(rect, node.layout.rect)),
                }),
                UiContent::Scene(primitives) => {
                    let context = ScenePaintContext {
                        node: id,
                        node_rect: node.layout.rect,
                        clip_rect: node.layout.clip_rect,
                        z_index,
                        layer_order,
                        opacity,
                        morph: animation_values.morph,
                        transform,
                        shader: node.shader.clone(),
                    };
                    for primitive in primitives {
                        list.items
                            .push(scene_primitive_to_paint_item(&context, primitive));
                    }
                }
            }
        }
        list
    }

    fn node_animation_values(node: &UiNode) -> AnimatedValues {
        node.animation
            .as_ref()
            .map(AnimationMachine::values)
            .unwrap_or_default()
    }

    fn node_paint_transform(node: &UiNode) -> PaintTransform {
        Self::paint_transform_from_values(Self::node_animation_values(node))
    }

    fn paint_transform_from_values(values: AnimatedValues) -> PaintTransform {
        PaintTransform {
            translation: values.translate,
            scale: values.scale,
        }
    }

    fn node_paint_rect(&self, index: usize) -> UiRect {
        let node = &self.nodes[index];
        Self::node_paint_transform(node).transform_rect(node.layout.rect)
    }

    fn visual_order(&self) -> Vec<usize> {
        let layer_orders = self.effective_layer_orders();
        self.visual_order_with_layer(&layer_orders)
    }

    fn visual_order_with_layer(&self, layer_orders: &[platform::LayerOrder]) -> Vec<usize> {
        let mut order = (0..self.nodes.len()).collect::<Vec<_>>();
        if !layer_orders.windows(2).all(|pair| pair[0] <= pair[1]) {
            order.sort_by_key(|index| (layer_orders[*index], *index));
        }
        order
    }

    fn effective_layer_orders(&self) -> Vec<platform::LayerOrder> {
        let mut orders = vec![platform::LayerOrder::DEFAULT; self.nodes.len()];
        for index in 0..self.nodes.len() {
            let node = &self.nodes[index];
            let local_z = if index == self.root.0 {
                node.style.z_index
            } else if node.style.z_index == 0 {
                node.parent
                    .map(|parent| orders[parent.0].local_z)
                    .unwrap_or(node.style.z_index)
            } else {
                node.style.z_index
            };
            let layer = node
                .layer
                .or_else(|| node.parent.map(|parent| orders[parent.0].layer))
                .unwrap_or(platform::UiLayer::AppContent);
            orders[index] = platform::LayerOrder::new(layer, local_z);
        }
        orders
    }
}

#[derive(Debug, Clone)]
struct ScenePaintContext {
    node: UiNodeId,
    node_rect: UiRect,
    clip_rect: UiRect,
    z_index: i16,
    layer_order: platform::LayerOrder,
    opacity: f32,
    morph: f32,
    transform: PaintTransform,
    shader: Option<ShaderEffect>,
}

fn scene_primitive_to_paint_item(
    context: &ScenePaintContext,
    primitive: &ScenePrimitive,
) -> PaintItem {
    match primitive {
        ScenePrimitive::Line { from, to, stroke } => {
            let from = point_in_rect(context.node_rect, *from);
            let to = point_in_rect(context.node_rect, *to);
            PaintItem {
                node: context.node,
                rect: rect_from_points(&[from, to]),
                clip_rect: context.clip_rect,
                z_index: context.z_index,
                layer_order: context.layer_order,
                opacity: context.opacity,
                transform: context.transform,
                shader: context.shader.clone(),
                kind: PaintKind::Line {
                    from,
                    to,
                    stroke: *stroke,
                },
            }
        }
        ScenePrimitive::Circle {
            center,
            radius,
            fill,
            stroke,
        } => {
            let center = point_in_rect(context.node_rect, *center);
            PaintItem {
                node: context.node,
                rect: UiRect::new(
                    center.x - radius,
                    center.y - radius,
                    radius * 2.0,
                    radius * 2.0,
                ),
                clip_rect: context.clip_rect,
                z_index: context.z_index,
                layer_order: context.layer_order,
                opacity: context.opacity,
                transform: context.transform,
                shader: context.shader.clone(),
                kind: PaintKind::Circle {
                    center,
                    radius: *radius,
                    fill: *fill,
                    stroke: *stroke,
                },
            }
        }
        ScenePrimitive::Polygon {
            points,
            fill,
            stroke,
        } => {
            let points = points
                .iter()
                .map(|point| point_in_rect(context.node_rect, *point))
                .collect::<Vec<_>>();
            polygon_paint_item(context, points, *fill, *stroke)
        }
        ScenePrimitive::MorphPolygon {
            from_points,
            to_points,
            amount,
            fill,
            stroke,
        } => {
            let amount = (*amount + context.morph).clamp(0.0, 1.0);
            let points = morph_polygon_points(from_points, to_points, amount)
                .into_iter()
                .map(|point| point_in_rect(context.node_rect, point))
                .collect::<Vec<_>>();
            polygon_paint_item(context, points, *fill, *stroke)
        }
        ScenePrimitive::Image { key, rect, tint } => PaintItem {
            node: context.node,
            rect: UiRect::new(
                context.node_rect.x + rect.x,
                context.node_rect.y + rect.y,
                rect.width,
                rect.height,
            ),
            clip_rect: context.clip_rect,
            z_index: context.z_index,
            layer_order: context.layer_order,
            opacity: context.opacity,
            transform: context.transform,
            shader: context.shader.clone(),
            kind: PaintKind::Image {
                key: key.clone(),
                tint: *tint,
            },
        },
        ScenePrimitive::Rect(rect) => {
            let rect = rect
                .clone()
                .translated(UiPoint::new(context.node_rect.x, context.node_rect.y));
            PaintItem {
                node: context.node,
                rect: rect.rect,
                clip_rect: context.clip_rect,
                z_index: context.z_index,
                layer_order: context.layer_order,
                opacity: context.opacity,
                transform: context.transform,
                shader: context.shader.clone(),
                kind: PaintKind::RichRect(rect),
            }
        }
        ScenePrimitive::Text(text) => {
            let text = text
                .clone()
                .translated(UiPoint::new(context.node_rect.x, context.node_rect.y));
            PaintItem {
                node: context.node,
                rect: text.rect,
                clip_rect: context.clip_rect,
                z_index: context.z_index,
                layer_order: context.layer_order,
                opacity: context.opacity,
                transform: context.transform,
                shader: context.shader.clone(),
                kind: PaintKind::SceneText(text),
            }
        }
        ScenePrimitive::Path(path) => {
            let path = path
                .clone()
                .translated(UiPoint::new(context.node_rect.x, context.node_rect.y));
            PaintItem {
                node: context.node,
                rect: path.bounds(),
                clip_rect: context.clip_rect,
                z_index: context.z_index,
                layer_order: context.layer_order,
                opacity: context.opacity,
                transform: context.transform,
                shader: context.shader.clone(),
                kind: PaintKind::Path(path),
            }
        }
        ScenePrimitive::ImagePlacement(image) => {
            let image = image
                .clone()
                .translated(UiPoint::new(context.node_rect.x, context.node_rect.y));
            PaintItem {
                node: context.node,
                rect: image.rect,
                clip_rect: context.clip_rect,
                z_index: context.z_index,
                layer_order: context.layer_order,
                opacity: context.opacity,
                transform: context.transform,
                shader: context.shader.clone(),
                kind: PaintKind::ImagePlacement(image),
            }
        }
    }
}

fn polygon_paint_item(
    context: &ScenePaintContext,
    points: Vec<UiPoint>,
    fill: ColorRgba,
    stroke: Option<StrokeStyle>,
) -> PaintItem {
    PaintItem {
        node: context.node,
        rect: rect_from_points(&points),
        clip_rect: context.clip_rect,
        z_index: context.z_index,
        layer_order: context.layer_order,
        opacity: context.opacity,
        transform: context.transform,
        shader: context.shader.clone(),
        kind: PaintKind::Polygon {
            points,
            fill,
            stroke,
        },
    }
}

fn morph_polygon_points(
    from_points: &[UiPoint],
    to_points: &[UiPoint],
    amount: f32,
) -> Vec<UiPoint> {
    if from_points.is_empty() || to_points.is_empty() {
        return Vec::new();
    }
    let count = from_points.len().max(to_points.len()).max(3);
    let from = resample_closed_polygon(from_points, count);
    let to = resample_closed_polygon(to_points, count);
    let amount = finite_or(amount, 0.0).clamp(0.0, 1.0);
    from.into_iter()
        .zip(to)
        .map(|(from, to)| {
            UiPoint::new(
                from.x + (to.x - from.x) * amount,
                from.y + (to.y - from.y) * amount,
            )
        })
        .collect()
}

fn resample_closed_polygon(points: &[UiPoint], count: usize) -> Vec<UiPoint> {
    if points.is_empty() || count == 0 {
        return Vec::new();
    }
    if points.len() == 1 {
        return vec![points[0]; count];
    }
    if points.len() == count {
        return points.to_vec();
    }
    let mut lengths = Vec::with_capacity(points.len());
    let mut perimeter = 0.0;
    for index in 0..points.len() {
        let next = points[(index + 1) % points.len()];
        let length = distance(points[index], next);
        lengths.push(length);
        perimeter += length;
    }
    if perimeter <= f32::EPSILON {
        return vec![points[0]; count];
    }
    let mut samples = Vec::with_capacity(count);
    let mut segment = 0;
    let mut segment_start_distance = 0.0;
    for sample in 0..count {
        let target = perimeter * sample as f32 / count as f32;
        while segment + 1 < points.len() && segment_start_distance + lengths[segment] < target {
            segment_start_distance += lengths[segment];
            segment += 1;
        }
        let from = points[segment];
        let to = points[(segment + 1) % points.len()];
        let length = lengths[segment].max(f32::EPSILON);
        let t = ((target - segment_start_distance) / length).clamp(0.0, 1.0);
        samples.push(UiPoint::new(
            from.x + (to.x - from.x) * t,
            from.y + (to.y - from.y) * t,
        ));
    }
    samples
}

fn distance(from: UiPoint, to: UiPoint) -> f32 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    (dx * dx + dy * dy).sqrt()
}

fn point_in_rect(rect: UiRect, point: UiPoint) -> UiPoint {
    UiPoint::new(rect.x + point.x, rect.y + point.y)
}

fn rect_from_points(points: &[UiPoint]) -> UiRect {
    if points.is_empty() {
        return UiRect::new(0.0, 0.0, 0.0, 0.0);
    }
    let mut left = points[0].x;
    let mut top = points[0].y;
    let mut right = points[0].x;
    let mut bottom = points[0].y;
    for point in points.iter().copied().skip(1) {
        left = left.min(point.x);
        top = top.min(point.y);
        right = right.max(point.x);
        bottom = bottom.max(point.y);
    }
    UiRect::new(left, top, right - left, bottom - top)
}

fn scene_primitives_intrinsic_size(primitives: &[ScenePrimitive], scale: f32) -> UiSize {
    let Some(bounds) = primitives
        .iter()
        .filter_map(scene_primitive_bounds)
        .reduce(union_rects)
    else {
        return UiSize::ZERO;
    };
    rect_intrinsic_size(bounds, scale)
}

fn scene_primitive_bounds(primitive: &ScenePrimitive) -> Option<UiRect> {
    match primitive {
        ScenePrimitive::Line { from, to, stroke } => Some(expand_rect(
            rect_from_points(&[*from, *to]),
            stroke.width * 0.5,
        )),
        ScenePrimitive::Circle {
            center,
            radius,
            stroke,
            ..
        } => {
            let radius = finite_or(*radius, 0.0).max(0.0)
                + stroke.map_or(0.0, |stroke| stroke.width.max(0.0) * 0.5);
            Some(UiRect::new(
                center.x - radius,
                center.y - radius,
                radius * 2.0,
                radius * 2.0,
            ))
        }
        ScenePrimitive::Polygon { points, stroke, .. } => (!points.is_empty()).then(|| {
            expand_rect(
                rect_from_points(points),
                stroke.map_or(0.0, |stroke| stroke.width.max(0.0) * 0.5),
            )
        }),
        ScenePrimitive::MorphPolygon {
            from_points,
            to_points,
            stroke,
            ..
        } => {
            let from = (!from_points.is_empty()).then(|| rect_from_points(from_points));
            let to = (!to_points.is_empty()).then(|| rect_from_points(to_points));
            from.into_iter()
                .chain(to)
                .reduce(union_rects)
                .map(|bounds| {
                    expand_rect(
                        bounds,
                        stroke.map_or(0.0, |stroke| stroke.width.max(0.0) * 0.5),
                    )
                })
        }
        ScenePrimitive::Image { rect, .. } => Some(finite_rect(*rect)),
        ScenePrimitive::Rect(rect) => Some(paint_rect_bounds(rect)),
        ScenePrimitive::Text(text) => Some(finite_rect(text.rect)),
        ScenePrimitive::Path(path) => Some(expand_rect(
            path.bounds(),
            path.stroke
                .map_or(0.0, |stroke| stroke.style.width.max(0.0) * 0.5),
        )),
        ScenePrimitive::ImagePlacement(image) => Some(finite_rect(image.rect)),
    }
}

fn paint_rect_intrinsic_size(rect: &PaintRect, scale: f32) -> UiSize {
    if rect.rect.width <= f32::EPSILON || rect.rect.height <= f32::EPSILON {
        return UiSize::ZERO;
    }
    rect_intrinsic_size(paint_rect_bounds(rect), scale)
}

fn paint_rect_bounds(rect: &PaintRect) -> UiRect {
    let mut bounds = finite_rect(rect.rect);
    if let Some(stroke) = rect.stroke {
        bounds = expand_rect(bounds, aligned_stroke_outer_extent(stroke));
    }
    for effect in &rect.effects {
        if effect.color.a == 0 {
            continue;
        }
        match effect.kind {
            PaintEffectKind::Shadow | PaintEffectKind::Glow => {
                let outset = effect.spread.max(0.0) + effect.blur_radius.max(0.0);
                let effect_bounds = expand_rect(bounds, outset);
                let effect_bounds = UiRect::new(
                    effect_bounds.x + effect.offset.x,
                    effect_bounds.y + effect.offset.y,
                    effect_bounds.width,
                    effect_bounds.height,
                );
                bounds = union_rects(bounds, effect_bounds);
            }
            PaintEffectKind::InsetShadow => {}
        }
    }
    bounds
}

fn aligned_stroke_outer_extent(stroke: AlignedStroke) -> f32 {
    let width = stroke.style.width.max(0.0);
    match stroke.alignment {
        StrokeAlignment::Inside => 0.0,
        StrokeAlignment::Center => width * 0.5,
        StrokeAlignment::Outside => width,
    }
}

fn rect_intrinsic_size(rect: UiRect, scale: f32) -> UiSize {
    let rect = finite_rect(rect);
    let width = rect.right().max(rect.width).max(0.0) * scale;
    let height = rect.bottom().max(rect.height).max(0.0) * scale;
    UiSize::new(width, height)
}

fn expand_rect(rect: UiRect, amount: f32) -> UiRect {
    let rect = finite_rect(rect);
    let amount = finite_or(amount, 0.0).max(0.0);
    UiRect::new(
        rect.x - amount,
        rect.y - amount,
        rect.width + amount * 2.0,
        rect.height + amount * 2.0,
    )
}

fn union_rects(a: UiRect, b: UiRect) -> UiRect {
    let a = finite_rect(a);
    let b = finite_rect(b);
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = a.right().max(b.right());
    let bottom = a.bottom().max(b.bottom());
    UiRect::new(left, top, (right - left).max(0.0), (bottom - top).max(0.0))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PaintList {
    pub items: Vec<PaintItem>,
}

impl PaintList {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaintItem {
    pub node: UiNodeId,
    pub rect: UiRect,
    pub clip_rect: UiRect,
    pub z_index: i16,
    pub layer_order: platform::LayerOrder,
    pub opacity: f32,
    pub transform: PaintTransform,
    pub shader: Option<ShaderEffect>,
    pub kind: PaintKind,
}

/// A nested paint list that should be rendered as an isolated compositor layer.
///
/// `bounds`, `clip`, and `mask` use the same paint-space coordinates as the
/// child paint list. Backends can render the child list into an offscreen target
/// and composite it back with opacity, clipping, masks, and filters.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintCompositorLayer {
    pub bounds: UiRect,
    pub paint: PaintList,
    pub clip: Option<CompositorClip>,
    pub mask: Option<CompositorMask>,
    pub filters: Vec<CompositorFilter>,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub isolation: OffscreenIsolation,
    pub subpixel_text: SubpixelTextPolicy,
}

impl PaintCompositorLayer {
    pub fn new(bounds: UiRect, paint: PaintList) -> Self {
        Self {
            bounds,
            paint,
            clip: None,
            mask: None,
            filters: Vec::new(),
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            isolation: OffscreenIsolation::Auto,
            subpixel_text: SubpixelTextPolicy::Grayscale,
        }
    }

    pub fn clip(mut self, clip: CompositorClip) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn mask(mut self, mask: CompositorMask) -> Self {
        self.mask = Some(mask);
        self
    }

    pub fn filter(mut self, filter: CompositorFilter) -> Self {
        self.filters.push(filter);
        self
    }

    pub const fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub const fn blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    pub const fn isolation(mut self, isolation: OffscreenIsolation) -> Self {
        self.isolation = isolation;
        self
    }

    pub const fn subpixel_text(mut self, policy: SubpixelTextPolicy) -> Self {
        self.subpixel_text = policy;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaintTransform {
    pub translation: UiPoint,
    pub scale: f32,
}

impl Default for PaintTransform {
    fn default() -> Self {
        Self {
            translation: UiPoint::new(0.0, 0.0),
            scale: 1.0,
        }
    }
}

impl PaintTransform {
    pub fn transform_point(self, point: UiPoint) -> UiPoint {
        UiPoint::new(
            point.x * self.scale + self.translation.x,
            point.y * self.scale + self.translation.y,
        )
    }

    pub fn transform_rect(self, rect: UiRect) -> UiRect {
        let top_left = self.transform_point(UiPoint::new(rect.x, rect.y));
        UiRect::new(
            top_left.x,
            top_left.y,
            rect.width * self.scale,
            rect.height * self.scale,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaintKind {
    Rect {
        fill: ColorRgba,
        stroke: Option<StrokeStyle>,
        corner_radius: f32,
    },
    Text(TextContent),
    Canvas(CanvasContent),
    Line {
        from: UiPoint,
        to: UiPoint,
        stroke: StrokeStyle,
    },
    Circle {
        center: UiPoint,
        radius: f32,
        fill: ColorRgba,
        stroke: Option<StrokeStyle>,
    },
    Polygon {
        points: Vec<UiPoint>,
        fill: ColorRgba,
        stroke: Option<StrokeStyle>,
    },
    Image {
        key: String,
        tint: Option<ColorRgba>,
    },
    CompositedLayer(PaintCompositorLayer),
    RichRect(PaintRect),
    SceneText(PaintText),
    Path(PaintPath),
    ImagePlacement(PaintImage),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSnapshot {
    pub id: UiNodeId,
    pub name: String,
    pub rect: UiRect,
    pub clip_rect: UiRect,
    pub visible: bool,
    pub pointer: bool,
    pub focusable: bool,
    pub scroll: Option<ScrollState>,
    pub scrollbar: Option<ScrollbarAuditState>,
    pub children: Vec<LayoutSnapshot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityNode {
    pub id: UiNodeId,
    pub parent: Option<UiNodeId>,
    pub role: AccessibilityRole,
    pub label: Option<String>,
    pub value: Option<String>,
    pub hint: Option<String>,
    pub rect: UiRect,
    pub enabled: bool,
    pub focusable: bool,
    pub modal: bool,
    pub selected: Option<bool>,
    pub checked: Option<AccessibilityChecked>,
    pub expanded: Option<bool>,
    pub pressed: Option<bool>,
    pub read_only: bool,
    pub required: bool,
    pub invalid: Option<String>,
    pub live_region: AccessibilityLiveRegion,
    pub sort: AccessibilitySortDirection,
    pub value_range: Option<AccessibilityValueRange>,
    pub focus_order: Option<i32>,
    pub key_shortcuts: Vec<String>,
    pub actions: Vec<AccessibilityAction>,
    pub relations: AccessibilityRelations,
    pub summary: Option<AccessibilitySummary>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccessibilityTree {
    pub nodes: Vec<AccessibilityNode>,
    pub focus_order: Vec<UiNodeId>,
    pub modal_scope: Option<UiNodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuditAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditWarning {
    NonFiniteRect {
        node: UiNodeId,
        name: String,
    },
    InvisibleInteractiveNode {
        node: UiNodeId,
        name: String,
    },
    EmptyInteractiveClip {
        node: UiNodeId,
        name: String,
    },
    InteractiveTooSmall {
        node: UiNodeId,
        name: String,
        rect: UiRect,
    },
    DuplicateNodeName {
        name: String,
    },
    FocusableMissingFromAccessibilityTree {
        node: UiNodeId,
        name: String,
    },
    InteractiveAccessibilityMissing {
        node: UiNodeId,
        name: String,
    },
    AccessibleNameMissing {
        node: UiNodeId,
        name: String,
        role: AccessibilityRole,
    },
    AccessibilityActionMissing {
        node: UiNodeId,
        name: String,
        role: AccessibilityRole,
    },
    AccessibilityActionIdMissing {
        node: UiNodeId,
        name: String,
    },
    AccessibilityActionLabelMissing {
        node: UiNodeId,
        name: String,
        action_id: String,
    },
    AccessibilityActionDuplicate {
        node: UiNodeId,
        name: String,
        action_id: String,
    },
    AccessibilityStateMissing {
        node: UiNodeId,
        name: String,
        role: AccessibilityRole,
        state: AccessibilityStateKind,
    },
    AccessibilityValueMissing {
        node: UiNodeId,
        name: String,
        role: AccessibilityRole,
    },
    AccessibilityValueRangeMissing {
        node: UiNodeId,
        name: String,
        role: AccessibilityRole,
    },
    AccessibilityValueRangeInvalid {
        node: UiNodeId,
        name: String,
        role: AccessibilityRole,
        issue: AccessibilityValueRangeIssue,
        range: AccessibilityValueRange,
    },
    AccessibilityRelationTargetMissing {
        node: UiNodeId,
        name: String,
        relation: AccessibilityRelationKind,
        target: UiNodeId,
    },
    TextClipped {
        node: UiNodeId,
        name: String,
        rect: UiRect,
        clip_rect: UiRect,
    },
    ScrollRangeHidden {
        node: UiNodeId,
        name: String,
        axis: AuditAxis,
        viewport: f32,
        content: f32,
    },
    ScrollOffsetOutOfRange {
        node: UiNodeId,
        name: String,
        axis: AuditAxis,
        offset: f32,
        max_offset: f32,
    },
    ScrollbarVisibleWithoutRange {
        node: UiNodeId,
        name: String,
        axis: AuditAxis,
        viewport: f32,
        content: f32,
    },
    TextContrastTooLow {
        node: UiNodeId,
        name: String,
        text_color: ColorRgba,
        background_color: ColorRgba,
        contrast_ratio: f32,
        required_ratio: f32,
    },
    NodeOutsideRoot {
        node: UiNodeId,
        name: String,
        rect: UiRect,
    },
    PaintItemEmptyClip {
        node: UiNodeId,
    },
}

impl AuditWarning {
    pub fn node(&self) -> Option<UiNodeId> {
        match self {
            Self::NonFiniteRect { node, .. }
            | Self::InvisibleInteractiveNode { node, .. }
            | Self::EmptyInteractiveClip { node, .. }
            | Self::InteractiveTooSmall { node, .. }
            | Self::FocusableMissingFromAccessibilityTree { node, .. }
            | Self::InteractiveAccessibilityMissing { node, .. }
            | Self::AccessibleNameMissing { node, .. }
            | Self::AccessibilityActionMissing { node, .. }
            | Self::AccessibilityActionIdMissing { node, .. }
            | Self::AccessibilityActionLabelMissing { node, .. }
            | Self::AccessibilityActionDuplicate { node, .. }
            | Self::AccessibilityStateMissing { node, .. }
            | Self::AccessibilityValueMissing { node, .. }
            | Self::AccessibilityValueRangeMissing { node, .. }
            | Self::AccessibilityValueRangeInvalid { node, .. }
            | Self::AccessibilityRelationTargetMissing { node, .. }
            | Self::TextClipped { node, .. }
            | Self::ScrollRangeHidden { node, .. }
            | Self::ScrollOffsetOutOfRange { node, .. }
            | Self::ScrollbarVisibleWithoutRange { node, .. }
            | Self::TextContrastTooLow { node, .. }
            | Self::NodeOutsideRoot { node, .. }
            | Self::PaintItemEmptyClip { node } => Some(*node),
            Self::DuplicateNodeName { .. } => None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::NonFiniteRect { name, .. }
            | Self::InvisibleInteractiveNode { name, .. }
            | Self::EmptyInteractiveClip { name, .. }
            | Self::InteractiveTooSmall { name, .. }
            | Self::FocusableMissingFromAccessibilityTree { name, .. }
            | Self::InteractiveAccessibilityMissing { name, .. }
            | Self::AccessibleNameMissing { name, .. }
            | Self::AccessibilityActionMissing { name, .. }
            | Self::AccessibilityActionIdMissing { name, .. }
            | Self::AccessibilityActionLabelMissing { name, .. }
            | Self::AccessibilityActionDuplicate { name, .. }
            | Self::AccessibilityStateMissing { name, .. }
            | Self::AccessibilityValueMissing { name, .. }
            | Self::AccessibilityValueRangeMissing { name, .. }
            | Self::AccessibilityValueRangeInvalid { name, .. }
            | Self::AccessibilityRelationTargetMissing { name, .. }
            | Self::TextClipped { name, .. }
            | Self::ScrollRangeHidden { name, .. }
            | Self::ScrollOffsetOutOfRange { name, .. }
            | Self::ScrollbarVisibleWithoutRange { name, .. }
            | Self::TextContrastTooLow { name, .. }
            | Self::NodeOutsideRoot { name, .. }
            | Self::DuplicateNodeName { name } => Some(name),
            Self::PaintItemEmptyClip { .. } => None,
        }
    }

    pub fn reason(&self) -> &'static str {
        match self {
            Self::NonFiniteRect { .. } => "layout rect contains non-finite geometry",
            Self::InvisibleInteractiveNode { .. } => "interactive node is not visible",
            Self::EmptyInteractiveClip { .. } => "interactive node has an empty clip",
            Self::InteractiveTooSmall { .. } => "interactive target is smaller than the minimum",
            Self::DuplicateNodeName { .. } => "node name is duplicated",
            Self::FocusableMissingFromAccessibilityTree { .. } => {
                "focusable node is missing from the accessibility tree"
            }
            Self::InteractiveAccessibilityMissing { .. } => {
                "interactive node is missing accessibility metadata"
            }
            Self::AccessibleNameMissing { .. } => "accessible node is missing a name",
            Self::AccessibilityActionMissing { .. } => {
                "interactive accessible node is missing an action"
            }
            Self::AccessibilityActionIdMissing { .. } => "accessibility action is missing an id",
            Self::AccessibilityActionLabelMissing { .. } => {
                "accessibility action is missing a label"
            }
            Self::AccessibilityActionDuplicate { .. } => "accessibility action id is duplicated",
            Self::AccessibilityStateMissing { .. } => {
                "accessibility role is missing required state"
            }
            Self::AccessibilityValueMissing { .. } => {
                "accessibility role is missing a current value"
            }
            Self::AccessibilityValueRangeMissing { .. } => {
                "accessibility role is missing a value range"
            }
            Self::AccessibilityValueRangeInvalid { .. } => "accessibility value range is invalid",
            Self::AccessibilityRelationTargetMissing { .. } => {
                "accessibility relation points at a missing target"
            }
            Self::TextClipped { .. } => "text is clipped without an explicit overflow policy",
            Self::ScrollRangeHidden { .. } => "scroll content extends beyond a disabled axis",
            Self::ScrollOffsetOutOfRange { .. } => "scroll offset is outside the reachable range",
            Self::ScrollbarVisibleWithoutRange { .. } => {
                "scrollbar is visible even though its axis has no scroll range"
            }
            Self::TextContrastTooLow { .. } => "text contrast is below the required ratio",
            Self::NodeOutsideRoot { .. } => {
                "node extends outside the root without scroll or overlay intent"
            }
            Self::PaintItemEmptyClip { .. } => "paint item has an empty clip",
        }
    }

    pub fn measured_values(&self) -> String {
        match self {
            Self::InteractiveTooSmall { rect, .. } => format!("rect={rect:?}"),
            Self::DuplicateNodeName { name } => format!("name={name:?}"),
            Self::AccessibleNameMissing { role, .. }
            | Self::AccessibilityActionMissing { role, .. }
            | Self::AccessibilityValueMissing { role, .. }
            | Self::AccessibilityValueRangeMissing { role, .. } => format!("role={role:?}"),
            Self::AccessibilityActionLabelMissing { action_id, .. }
            | Self::AccessibilityActionDuplicate { action_id, .. } => {
                format!("action_id={action_id:?}")
            }
            Self::AccessibilityStateMissing { role, state, .. } => {
                format!("role={role:?}, state={state:?}")
            }
            Self::AccessibilityValueRangeInvalid {
                role, issue, range, ..
            } => {
                format!("role={role:?}, issue={issue:?}, range={range:?}")
            }
            Self::AccessibilityRelationTargetMissing {
                relation, target, ..
            } => {
                format!("relation={relation:?}, target={target:?}")
            }
            Self::TextClipped {
                rect, clip_rect, ..
            } => {
                format!("rect={rect:?}, clip_rect={clip_rect:?}")
            }
            Self::ScrollRangeHidden {
                axis,
                viewport,
                content,
                ..
            } => {
                format!("axis={axis:?}, viewport={viewport:.2}, content={content:.2}")
            }
            Self::ScrollOffsetOutOfRange {
                axis,
                offset,
                max_offset,
                ..
            } => {
                format!("axis={axis:?}, offset={offset:.2}, max_offset={max_offset:.2}")
            }
            Self::ScrollbarVisibleWithoutRange {
                axis,
                viewport,
                content,
                ..
            } => {
                format!("axis={axis:?}, viewport={viewport:.2}, content={content:.2}")
            }
            Self::TextContrastTooLow {
                text_color,
                background_color,
                contrast_ratio,
                required_ratio,
                ..
            } => {
                format!(
                    "text_color={text_color:?}, background_color={background_color:?}, contrast={contrast_ratio:.2}, required={required_ratio:.2}"
                )
            }
            Self::NodeOutsideRoot { rect, .. } => format!("rect={rect:?}"),
            Self::PaintItemEmptyClip { node } => format!("node={node:?}"),
            Self::NonFiniteRect { .. }
            | Self::InvisibleInteractiveNode { .. }
            | Self::EmptyInteractiveClip { .. }
            | Self::FocusableMissingFromAccessibilityTree { .. }
            | Self::InteractiveAccessibilityMissing { .. }
            | Self::AccessibilityActionIdMissing { .. } => String::new(),
        }
    }

    pub fn remediation_hint(&self) -> &'static str {
        match self {
            Self::NonFiniteRect { .. } => "sanitize layout inputs before building the document",
            Self::InvisibleInteractiveNode { .. } => {
                "hide non-interactive decoration, or remove pointer/focus input from invisible nodes"
            }
            Self::EmptyInteractiveClip { .. } => {
                "increase the computed size or route the control through a scroll/overflow container"
            }
            Self::InteractiveTooSmall { .. } => {
                "publish a larger intrinsic minimum or mark the control as intentionally compact"
            }
            Self::DuplicateNodeName { .. } => "give each retained node a stable unique name",
            Self::FocusableMissingFromAccessibilityTree { .. }
            | Self::InteractiveAccessibilityMissing { .. } => {
                "attach role, label, value, and action metadata through AccessibilityMeta"
            }
            Self::AccessibleNameMissing { .. } => {
                "set an accessibility label or connect a labelled-by relation"
            }
            Self::AccessibilityActionMissing { .. } => {
                "publish an accessibility action for the interactive behavior"
            }
            Self::AccessibilityActionIdMissing { .. } => {
                "assign a stable action id so replay and assistive tech can identify it"
            }
            Self::AccessibilityActionLabelMissing { .. } => {
                "give the action a user-facing label"
            }
            Self::AccessibilityActionDuplicate { .. } => {
                "deduplicate action ids on the node"
            }
            Self::AccessibilityStateMissing { .. } => {
                "publish the role-specific checked, expanded, pressed, or selected state"
            }
            Self::AccessibilityValueMissing { .. } => {
                "publish the current accessible value"
            }
            Self::AccessibilityValueRangeMissing { .. } => {
                "publish min, max, current, and step values for range-like controls"
            }
            Self::AccessibilityValueRangeInvalid { .. } => {
                "ensure the range is finite, ordered, and has a positive step"
            }
            Self::AccessibilityRelationTargetMissing { .. } => {
                "create the relation target in the same document or remove the stale relation"
            }
            Self::TextClipped { .. } => {
                "compute text intrinsic size from the active font, or opt into clipping/scrolling explicitly"
            }
            Self::ScrollRangeHidden { .. } => {
                "enable scrolling on the overflowing axis or constrain the content extent"
            }
            Self::ScrollOffsetOutOfRange { .. } => {
                "clamp persisted scroll offsets after content or viewport size changes"
            }
            Self::ScrollbarVisibleWithoutRange { .. } => {
                "hide and disable the scrollbar when ScrollState::max_offset is zero on that axis"
            }
            Self::TextContrastTooLow { .. } => {
                "adjust foreground or background tokens until the required contrast ratio is met"
            }
            Self::NodeOutsideRoot { .. } => {
                "place the node in an overlay, scroll container, or resize the parent constraints"
            }
            Self::PaintItemEmptyClip { .. } => {
                "check the node clip chain and avoid painting items outside their reachable clip"
            }
        }
    }

    pub fn diagnostic_summary(&self) -> String {
        let mut parts = vec![format!("reason: {}", self.reason())];
        if let Some(node) = self.node() {
            parts.push(format!("node: {node:?}"));
        }
        if let Some(name) = self.name() {
            parts.push(format!("name: {name}"));
        }
        let measured = self.measured_values();
        if !measured.is_empty() {
            parts.push(format!("measured: {measured}"));
        }
        parts.push(format!("hint: {}", self.remediation_hint()));
        parts.join("; ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityRelationKind {
    LabelledBy,
    DescribedBy,
    Controls,
    Owns,
    ActiveDescendant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityStateKind {
    Checked,
    Expanded,
    Pressed,
    Selected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityValueRangeIssue {
    NonFinite,
    Reversed,
    NonPositiveStep,
}

impl UiDocument {
    pub fn layout_snapshot(&self) -> LayoutSnapshot {
        self.layout_snapshot_subtree(self.root)
    }

    pub fn accessibility_tree(&self) -> Vec<AccessibilityNode> {
        self.accessibility_snapshot().nodes
    }

    pub fn accessibility_snapshot(&self) -> AccessibilityTree {
        let accessible_nodes = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let accessibility = node.accessibility.as_ref()?;
                (!accessibility.hidden).then_some(index)
            })
            .collect::<HashSet<_>>();
        let nodes = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let accessibility = node.accessibility.as_ref()?;
                if accessibility.hidden {
                    return None;
                }
                Some(AccessibilityNode {
                    id: UiNodeId(index),
                    parent: nearest_accessible_parent(&self.nodes, node.parent, &accessible_nodes),
                    role: accessibility.role,
                    label: accessibility.label.clone(),
                    value: accessibility.value.clone(),
                    hint: accessibility.hint.clone(),
                    rect: node.layout.rect,
                    enabled: accessibility.enabled,
                    focusable: accessibility.focusable || node.input.focusable,
                    modal: accessibility.modal,
                    selected: accessibility.selected,
                    checked: accessibility.checked,
                    expanded: accessibility.expanded,
                    pressed: accessibility.pressed,
                    read_only: accessibility.read_only,
                    required: accessibility.required,
                    invalid: accessibility.invalid.clone(),
                    live_region: accessibility.live_region,
                    sort: accessibility.sort,
                    value_range: accessibility.value_range,
                    focus_order: accessibility.focus_order,
                    key_shortcuts: accessibility.key_shortcuts.clone(),
                    actions: accessibility.actions.clone(),
                    relations: accessibility.relations.clone(),
                    summary: accessibility.summary.clone(),
                })
            })
            .collect::<Vec<_>>();
        let focus_order = accessibility_focus_order(&nodes);
        let modal_scope = nodes
            .iter()
            .find(|node| node.modal && node.enabled)
            .map(|node| node.id);
        AccessibilityTree {
            nodes,
            focus_order,
            modal_scope,
        }
    }

    pub fn accessibility_focus_order(&self) -> Vec<UiNodeId> {
        self.accessibility_snapshot().focus_order
    }

    fn layout_snapshot_subtree(&self, id: UiNodeId) -> LayoutSnapshot {
        let node = &self.nodes[id.0];
        LayoutSnapshot {
            id,
            name: node.name.clone(),
            rect: node.layout.rect,
            clip_rect: node.layout.clip_rect,
            visible: node.layout.visible,
            pointer: node.input.pointer,
            focusable: node.input.focusable,
            scroll: node.scroll,
            scrollbar: node.scrollbar,
            children: node
                .children
                .iter()
                .map(|child| self.layout_snapshot_subtree(*child))
                .collect(),
        }
    }

    pub fn audit_layout(&self) -> Vec<AuditWarning> {
        let mut warnings = Vec::new();
        let mut names = HashSet::new();
        let root_rect = self.nodes[self.root.0].layout.rect;
        let accessibility_snapshot = self.accessibility_snapshot();
        let focus_order = accessibility_snapshot
            .focus_order
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let accessible_nodes = accessibility_snapshot
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<HashSet<_>>();
        for (index, node) in self.nodes.iter().enumerate() {
            let id = UiNodeId(index);
            if !node.name.is_empty() && !names.insert(node.name.clone()) {
                warnings.push(AuditWarning::DuplicateNodeName {
                    name: node.name.clone(),
                });
            }
            if !rect_is_finite(node.layout.rect) || !rect_is_finite(node.layout.clip_rect) {
                warnings.push(AuditWarning::NonFiniteRect {
                    node: id,
                    name: node.name.clone(),
                });
            }
            if (node.input.pointer || node.input.focusable)
                && !node.layout.visible
                && !self.has_scroll_ancestor(id)
            {
                warnings.push(AuditWarning::InvisibleInteractiveNode {
                    node: id,
                    name: node.name.clone(),
                });
            }
            if (node.input.pointer || node.input.focusable)
                && (node.layout.clip_rect.width <= f32::EPSILON
                    || node.layout.clip_rect.height <= f32::EPSILON)
                && !self.has_scroll_ancestor(id)
            {
                warnings.push(AuditWarning::EmptyInteractiveClip {
                    node: id,
                    name: node.name.clone(),
                });
            }
            if (node.input.pointer || node.input.focusable)
                && node.layout.visible
                && !self.has_scroll_ancestor(id)
            {
                let hit_rect = node
                    .layout
                    .rect
                    .intersection(node.layout.clip_rect)
                    .unwrap_or(UiRect::new(0.0, 0.0, 0.0, 0.0));
                if hit_rect.width < 8.0 || hit_rect.height < 8.0 {
                    warnings.push(AuditWarning::InteractiveTooSmall {
                        node: id,
                        name: node.name.clone(),
                        rect: hit_rect,
                    });
                }
            }
            if node.input.focusable && !focus_order.contains(&id) {
                warnings.push(AuditWarning::FocusableMissingFromAccessibilityTree {
                    node: id,
                    name: node.name.clone(),
                });
            }
            if (node.input.pointer || node.input.focusable)
                && node.layout.visible
                && node
                    .accessibility
                    .as_ref()
                    .is_none_or(|accessibility| accessibility.hidden)
            {
                warnings.push(AuditWarning::InteractiveAccessibilityMissing {
                    node: id,
                    name: node.name.clone(),
                });
            }
            if let Some(scroll) = node.scroll {
                push_scroll_audit_warnings(&mut warnings, id, &node.name, scroll);
            }
            if let Some(scrollbar) = node.scrollbar {
                push_scrollbar_audit_warnings(&mut warnings, id, &node.name, node, scrollbar);
            }
            if let Some(accessibility) = node
                .accessibility
                .as_ref()
                .filter(|accessibility| !accessibility.hidden)
            {
                if accessibility_needs_name(accessibility.role)
                    && accessibility_snapshot.accessible_name(id).is_none()
                {
                    warnings.push(AuditWarning::AccessibleNameMissing {
                        node: id,
                        name: node.name.clone(),
                        role: accessibility.role,
                    });
                }
                if accessibility_needs_action(accessibility.role)
                    && (node.input.pointer || node.input.focusable || accessibility.focusable)
                    && accessibility.actions.is_empty()
                    && accessibility.enabled
                {
                    warnings.push(AuditWarning::AccessibilityActionMissing {
                        node: id,
                        name: node.name.clone(),
                        role: accessibility.role,
                    });
                }
                push_action_quality_warnings(&mut warnings, id, &node.name, &accessibility.actions);
                if let Some(state) = missing_required_accessibility_state(accessibility) {
                    warnings.push(AuditWarning::AccessibilityStateMissing {
                        node: id,
                        name: node.name.clone(),
                        role: accessibility.role,
                        state,
                    });
                }
                if accessibility_needs_value(accessibility.role)
                    && accessibility
                        .value
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                {
                    warnings.push(AuditWarning::AccessibilityValueMissing {
                        node: id,
                        name: node.name.clone(),
                        role: accessibility.role,
                    });
                }
                if accessibility_needs_value_range(accessibility.role)
                    && accessibility.value_range.is_none()
                {
                    warnings.push(AuditWarning::AccessibilityValueRangeMissing {
                        node: id,
                        name: node.name.clone(),
                        role: accessibility.role,
                    });
                }
                if let Some((range, issue)) = accessibility
                    .value_range
                    .and_then(invalid_accessibility_value_range)
                {
                    warnings.push(AuditWarning::AccessibilityValueRangeInvalid {
                        node: id,
                        name: node.name.clone(),
                        role: accessibility.role,
                        issue,
                        range,
                    });
                }
                push_missing_relation_target_warnings(
                    &mut warnings,
                    id,
                    &node.name,
                    &accessible_nodes,
                    &accessibility.relations,
                );
            }
            if let UiContent::Text(text) = &node.content {
                let (scroll_axes, clip_bounds) =
                    self.scroll_ancestor_axes_and_cross_bounds(id, node.layout.clip_rect);
                if rect_exceeds_unscrollable_axes(node.layout.rect, clip_bounds, scroll_axes) {
                    warnings.push(AuditWarning::TextClipped {
                        node: id,
                        name: node.name.clone(),
                        rect: node.layout.rect,
                        clip_rect: node.layout.clip_rect,
                    });
                }
                if let Some(background_color) = self.effective_background_color(id) {
                    push_text_contrast_warning(
                        &mut warnings,
                        id,
                        &node.name,
                        &text.style,
                        background_color,
                    );
                }
            }
            if let UiContent::Scene(primitives) = &node.content {
                if let Some(background_color) = self.effective_background_color(id) {
                    for primitive in primitives {
                        if let ScenePrimitive::Text(text) = primitive {
                            push_text_contrast_warning(
                                &mut warnings,
                                id,
                                &node.name,
                                &text.style,
                                background_color,
                            );
                        }
                    }
                }
            }
            let (root_scroll_axes, root_bounds) =
                self.scroll_ancestor_axes_and_cross_bounds(id, root_rect);
            if id != self.root
                && rect_exceeds_unscrollable_axes(node.layout.rect, root_bounds, root_scroll_axes)
                && !matches!(node.content, UiContent::Canvas(_))
            {
                warnings.push(AuditWarning::NodeOutsideRoot {
                    node: id,
                    name: node.name.clone(),
                    rect: node.layout.rect,
                });
            }
        }
        for item in self.paint_list().items {
            if item.clip_rect.width <= f32::EPSILON || item.clip_rect.height <= f32::EPSILON {
                warnings.push(AuditWarning::PaintItemEmptyClip { node: item.node });
            }
        }
        warnings
    }

    fn has_scroll_ancestor(&self, mut id: UiNodeId) -> bool {
        if self.nodes[id.0].clip_scope == ClipScope::Viewport {
            return false;
        }
        while let Some(parent) = self.nodes[id.0].parent {
            let parent_node = &self.nodes[parent.0];
            if parent_node.scroll.is_some() {
                return true;
            }
            if parent_node.clip_scope == ClipScope::Viewport {
                return false;
            }
            id = parent;
        }
        false
    }

    fn scroll_ancestor_axes_and_cross_bounds(
        &self,
        mut id: UiNodeId,
        fallback_bounds: UiRect,
    ) -> (ScrollAxes, UiRect) {
        if self.nodes[id.0].clip_scope == ClipScope::Viewport {
            return (ScrollAxes::NONE, fallback_bounds);
        }
        let mut axes = ScrollAxes::NONE;
        let mut horizontal_bounds = None;
        let mut vertical_bounds = None;
        while let Some(parent) = self.nodes[id.0].parent {
            let parent_node = &self.nodes[parent.0];
            if let Some(scroll) = parent_node.scroll {
                axes.horizontal |= scroll.axes.horizontal;
                axes.vertical |= scroll.axes.vertical;
                if scroll.axes.vertical && !scroll.axes.horizontal {
                    horizontal_bounds = intersect_axis_bounds(
                        horizontal_bounds,
                        parent_node.layout.rect.x,
                        parent_node.layout.rect.right(),
                    );
                }
                if scroll.axes.horizontal && !scroll.axes.vertical {
                    vertical_bounds = intersect_axis_bounds(
                        vertical_bounds,
                        parent_node.layout.rect.y,
                        parent_node.layout.rect.bottom(),
                    );
                }
            }
            if parent_node.clip_scope == ClipScope::Viewport {
                break;
            }
            id = parent;
        }

        let mut bounds = fallback_bounds;
        if let Some((x, right)) = horizontal_bounds {
            bounds.x = x;
            bounds.width = (right - x).max(0.0);
        }
        if let Some((y, bottom)) = vertical_bounds {
            bounds.y = y;
            bounds.height = (bottom - y).max(0.0);
        }
        (axes, bounds)
    }

    fn effective_background_color(&self, mut id: UiNodeId) -> Option<ColorRgba> {
        let mut lineage = Vec::new();
        loop {
            lineage.push(id);
            let Some(parent) = self.nodes[id.0].parent else {
                break;
            };
            id = parent;
        }
        let mut color = None;
        for id in lineage.into_iter().rev() {
            let fill = self.nodes[id.0].visual.fill;
            if fill.a > 0 {
                color = Some(match color {
                    Some(background) => fill.composite_over(background),
                    None => fill,
                });
            }
        }
        color
    }
}

fn effective_foreground_color(foreground: ColorRgba, background: ColorRgba) -> ColorRgba {
    if foreground.a == u8::MAX {
        foreground
    } else {
        foreground.composite_over(background)
    }
}

fn required_text_contrast_ratio(style: &TextStyle) -> f32 {
    if style.font_size >= 24.0 || (style.font_size >= 18.66 && style.weight.0 >= FontWeight::BOLD.0)
    {
        3.0
    } else {
        4.5
    }
}

fn push_text_contrast_warning(
    warnings: &mut Vec<AuditWarning>,
    node: UiNodeId,
    name: &str,
    style: &TextStyle,
    background_color: ColorRgba,
) {
    let text_color = effective_foreground_color(style.color, background_color);
    let contrast_ratio = text_color.contrast_ratio(background_color);
    let required_ratio = required_text_contrast_ratio(style);
    if contrast_ratio + f32::EPSILON < required_ratio {
        warnings.push(AuditWarning::TextContrastTooLow {
            node,
            name: name.to_string(),
            text_color: style.color,
            background_color,
            contrast_ratio,
            required_ratio,
        });
    }
}

fn push_scrollbar_audit_warnings(
    warnings: &mut Vec<AuditWarning>,
    node_id: UiNodeId,
    name: &str,
    node: &UiNode,
    scrollbar: ScrollbarAuditState,
) {
    if node.layout.visible && scrollbar.max_offset() <= f32::EPSILON {
        warnings.push(AuditWarning::ScrollbarVisibleWithoutRange {
            node: node_id,
            name: name.to_string(),
            axis: scrollbar.axis,
            viewport: scrollbar.viewport(),
            content: scrollbar.content(),
        });
    }
}

fn push_scroll_audit_warnings(
    warnings: &mut Vec<AuditWarning>,
    node: UiNodeId,
    name: &str,
    scroll: ScrollState,
) {
    if scroll.content_size.width > scroll.viewport_size.width + f32::EPSILON
        && !scroll.axes.horizontal
    {
        warnings.push(AuditWarning::ScrollRangeHidden {
            node,
            name: name.to_string(),
            axis: AuditAxis::Horizontal,
            viewport: scroll.viewport_size.width,
            content: scroll.content_size.width,
        });
    }
    if scroll.content_size.height > scroll.viewport_size.height + f32::EPSILON
        && !scroll.axes.vertical
    {
        warnings.push(AuditWarning::ScrollRangeHidden {
            node,
            name: name.to_string(),
            axis: AuditAxis::Vertical,
            viewport: scroll.viewport_size.height,
            content: scroll.content_size.height,
        });
    }

    let max_offset = scroll.max_offset();
    if scroll_offset_out_of_range(scroll.offset.x, max_offset.x) {
        warnings.push(AuditWarning::ScrollOffsetOutOfRange {
            node,
            name: name.to_string(),
            axis: AuditAxis::Horizontal,
            offset: scroll.offset.x,
            max_offset: max_offset.x,
        });
    }
    if scroll_offset_out_of_range(scroll.offset.y, max_offset.y) {
        warnings.push(AuditWarning::ScrollOffsetOutOfRange {
            node,
            name: name.to_string(),
            axis: AuditAxis::Vertical,
            offset: scroll.offset.y,
            max_offset: max_offset.y,
        });
    }
}

fn scroll_offset_out_of_range(offset: f32, max_offset: f32) -> bool {
    !offset.is_finite()
        || !max_offset.is_finite()
        || offset < -f32::EPSILON
        || offset > max_offset + f32::EPSILON
}

fn nearest_accessible_parent(
    nodes: &[UiNode],
    mut parent: Option<UiNodeId>,
    accessible_nodes: &HashSet<usize>,
) -> Option<UiNodeId> {
    while let Some(id) = parent {
        if accessible_nodes.contains(&id.0) {
            return Some(id);
        }
        parent = nodes.get(id.0).and_then(|node| node.parent);
    }

    None
}

fn accessibility_focus_order(nodes: &[AccessibilityNode]) -> Vec<UiNodeId> {
    let mut focusable = nodes
        .iter()
        .enumerate()
        .filter_map(|(document_order, node)| {
            (node.enabled && node.focusable).then_some((
                node.focus_order.unwrap_or(i32::MAX),
                document_order,
                node.id,
            ))
        })
        .collect::<Vec<_>>();
    focusable.sort_by_key(|(focus_order, document_order, _)| (*focus_order, *document_order));
    focusable.into_iter().map(|(_, _, id)| id).collect()
}

fn accessibility_needs_name(role: AccessibilityRole) -> bool {
    matches!(
        role,
        AccessibilityRole::Alert
            | AccessibilityRole::Button
            | AccessibilityRole::Checkbox
            | AccessibilityRole::ComboBox
            | AccessibilityRole::Dialog
            | AccessibilityRole::EditorSurface
            | AccessibilityRole::Grid
            | AccessibilityRole::Image
            | AccessibilityRole::Link
            | AccessibilityRole::List
            | AccessibilityRole::Menu
            | AccessibilityRole::MenuBar
            | AccessibilityRole::MenuItem
            | AccessibilityRole::Meter
            | AccessibilityRole::ProgressBar
            | AccessibilityRole::RadioButton
            | AccessibilityRole::SearchBox
            | AccessibilityRole::Slider
            | AccessibilityRole::SpinButton
            | AccessibilityRole::Splitter
            | AccessibilityRole::Status
            | AccessibilityRole::Switch
            | AccessibilityRole::Tab
            | AccessibilityRole::TabList
            | AccessibilityRole::TabPanel
            | AccessibilityRole::TextBox
            | AccessibilityRole::ToggleButton
            | AccessibilityRole::Toolbar
            | AccessibilityRole::Tooltip
            | AccessibilityRole::Tree
            | AccessibilityRole::TreeItem
            | AccessibilityRole::Window
    )
}

fn accessibility_needs_action(role: AccessibilityRole) -> bool {
    matches!(
        role,
        AccessibilityRole::Button
            | AccessibilityRole::Checkbox
            | AccessibilityRole::ComboBox
            | AccessibilityRole::Link
            | AccessibilityRole::MenuItem
            | AccessibilityRole::RadioButton
            | AccessibilityRole::SearchBox
            | AccessibilityRole::Slider
            | AccessibilityRole::SpinButton
            | AccessibilityRole::Splitter
            | AccessibilityRole::Switch
            | AccessibilityRole::Tab
            | AccessibilityRole::TextBox
            | AccessibilityRole::ToggleButton
            | AccessibilityRole::TreeItem
    )
}

fn accessibility_needs_value(role: AccessibilityRole) -> bool {
    matches!(
        role,
        AccessibilityRole::Meter
            | AccessibilityRole::ProgressBar
            | AccessibilityRole::Slider
            | AccessibilityRole::SpinButton
    )
}

fn accessibility_needs_value_range(role: AccessibilityRole) -> bool {
    matches!(
        role,
        AccessibilityRole::Meter | AccessibilityRole::Slider | AccessibilityRole::SpinButton
    )
}

fn missing_required_accessibility_state(
    accessibility: &AccessibilityMeta,
) -> Option<AccessibilityStateKind> {
    match accessibility.role {
        AccessibilityRole::Checkbox
        | AccessibilityRole::RadioButton
        | AccessibilityRole::Switch
            if accessibility.checked.is_none() =>
        {
            Some(AccessibilityStateKind::Checked)
        }
        AccessibilityRole::ComboBox if accessibility.expanded.is_none() => {
            Some(AccessibilityStateKind::Expanded)
        }
        AccessibilityRole::ToggleButton if accessibility.pressed.is_none() => {
            Some(AccessibilityStateKind::Pressed)
        }
        AccessibilityRole::Tab if accessibility.selected.is_none() => {
            Some(AccessibilityStateKind::Selected)
        }
        _ => None,
    }
}

fn invalid_accessibility_value_range(
    range: AccessibilityValueRange,
) -> Option<(AccessibilityValueRange, AccessibilityValueRangeIssue)> {
    if !range.min.is_finite()
        || !range.max.is_finite()
        || range.step.is_some_and(|step| !step.is_finite())
    {
        Some((range, AccessibilityValueRangeIssue::NonFinite))
    } else if range.max < range.min {
        Some((range, AccessibilityValueRangeIssue::Reversed))
    } else if range.step.is_some_and(|step| step <= 0.0) {
        Some((range, AccessibilityValueRangeIssue::NonPositiveStep))
    } else {
        None
    }
}

fn push_action_quality_warnings(
    warnings: &mut Vec<AuditWarning>,
    node: UiNodeId,
    name: &str,
    actions: &[AccessibilityAction],
) {
    let mut seen_ids = HashSet::new();
    for action in actions {
        let action_id = action.id.trim();
        if action_id.is_empty() {
            warnings.push(AuditWarning::AccessibilityActionIdMissing {
                node,
                name: name.to_string(),
            });
        } else if !seen_ids.insert(action_id.to_string()) {
            warnings.push(AuditWarning::AccessibilityActionDuplicate {
                node,
                name: name.to_string(),
                action_id: action_id.to_string(),
            });
        }
        if action.label.trim().is_empty() {
            warnings.push(AuditWarning::AccessibilityActionLabelMissing {
                node,
                name: name.to_string(),
                action_id: action.id.clone(),
            });
        }
    }
}

fn push_missing_relation_target_warnings(
    warnings: &mut Vec<AuditWarning>,
    node: UiNodeId,
    name: &str,
    accessible_nodes: &HashSet<UiNodeId>,
    relations: &AccessibilityRelations,
) {
    for target in &relations.labelled_by {
        push_missing_relation_target_warning(
            warnings,
            node,
            name,
            AccessibilityRelationKind::LabelledBy,
            *target,
            accessible_nodes,
        );
    }
    for target in &relations.described_by {
        push_missing_relation_target_warning(
            warnings,
            node,
            name,
            AccessibilityRelationKind::DescribedBy,
            *target,
            accessible_nodes,
        );
    }
    for target in &relations.controls {
        push_missing_relation_target_warning(
            warnings,
            node,
            name,
            AccessibilityRelationKind::Controls,
            *target,
            accessible_nodes,
        );
    }
    for target in &relations.owns {
        push_missing_relation_target_warning(
            warnings,
            node,
            name,
            AccessibilityRelationKind::Owns,
            *target,
            accessible_nodes,
        );
    }
    if let Some(target) = relations.active_descendant {
        push_missing_relation_target_warning(
            warnings,
            node,
            name,
            AccessibilityRelationKind::ActiveDescendant,
            target,
            accessible_nodes,
        );
    }
}

fn push_missing_relation_target_warning(
    warnings: &mut Vec<AuditWarning>,
    node: UiNodeId,
    name: &str,
    relation: AccessibilityRelationKind,
    target: UiNodeId,
    accessible_nodes: &HashSet<UiNodeId>,
) {
    if !accessible_nodes.contains(&target) {
        warnings.push(AuditWarning::AccessibilityRelationTargetMissing {
            node,
            name: name.to_owned(),
            relation,
            target,
        });
    }
}

pub(crate) fn rect_is_finite(rect: UiRect) -> bool {
    rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite()
}

fn node_blocks_wheel_passthrough(node: &UiNode) -> bool {
    node.input.pointer
        || node.input.focusable
        || node.scroll.is_some()
        || node.visual.fill.a > 0
        || node
            .visual
            .stroke
            .is_some_and(|stroke| stroke.width > 0.0 && stroke.color.a > 0)
        || matches!(
            &node.content,
            UiContent::Canvas(_)
                | UiContent::Image(_)
                | UiContent::PaintRect(_)
                | UiContent::Scene(_)
        )
}

fn rect_exceeds_unscrollable_axes(rect: UiRect, bounds: UiRect, scroll_axes: ScrollAxes) -> bool {
    let horizontal_exceeds = rect.x < bounds.x || rect.right() > bounds.right();
    let vertical_exceeds = rect.y < bounds.y || rect.bottom() > bounds.bottom();
    (horizontal_exceeds && !scroll_axes.horizontal) || (vertical_exceeds && !scroll_axes.vertical)
}

fn intersect_axis_bounds(current: Option<(f32, f32)>, start: f32, end: f32) -> Option<(f32, f32)> {
    let (start, end) = match current {
        Some((current_start, current_end)) => (current_start.max(start), current_end.min(end)),
        None => (start, end),
    };
    Some((start, start.max(end)))
}

fn aspect_fit_rect(rect: UiRect, aspect_ratio: f32) -> UiRect {
    if !aspect_ratio.is_finite() || aspect_ratio <= f32::EPSILON {
        return rect;
    }
    let width = rect.width.max(0.0);
    let height = rect.height.max(0.0);
    if width <= f32::EPSILON || height <= f32::EPSILON {
        return UiRect::new(rect.x, rect.y, width, height);
    }
    let height_from_width = width / aspect_ratio;
    let (fit_width, fit_height) = if height_from_width <= height {
        (width, height_from_width)
    } else {
        (height * aspect_ratio, height)
    };
    UiRect::new(
        rect.x + (width - fit_width) * 0.5,
        rect.y + (height - fit_height) * 0.5,
        fit_width,
        fit_height,
    )
}

fn assert_valid_node_id(operation: &'static str, id: UiNodeId, len: usize) {
    if id.0 >= len {
        invalid_node_id_panic(operation, id, len);
    }
}

fn invalid_node_id_panic(operation: &'static str, id: UiNodeId, len: usize) -> ! {
    panic!(
        "{operation} received stale or invalid node id UiNodeId({}); document has {len} node(s). \
         Node ids are frame-local; rebuild the id from the current UiDocument instead of reusing \
         one from an earlier frame.",
        id.0
    )
}

fn normalized_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

fn scaled_text_content(text: &TextContent, scale: f32) -> TextContent {
    let scale = normalized_scale(scale);
    if (scale - 1.0).abs() <= f32::EPSILON {
        return text.clone();
    }
    let mut text = text.clone();
    text.style.font_size = (text.style.font_size * scale).max(1.0);
    text.style.line_height = (text.style.line_height * scale).max(text.style.font_size);
    text
}

fn text_underline_segment(
    rect: UiRect,
    content_size: Option<UiSize>,
    text: &TextContent,
    scale: f32,
) -> (UiPoint, UiPoint) {
    let scale = normalized_scale(scale);
    let content_width = content_size
        .map(|size| size.width)
        .filter(|width| width.is_finite() && *width > f32::EPSILON)
        .unwrap_or_else(|| approximate_unwrapped_text_width(text, scale));
    let width = content_width.max(0.0);
    let line_height = (text.style.line_height * scale)
        .max(text.style.font_size * scale)
        .max(1.0);
    let y = rect.y + (line_height * 0.84).min(rect.height.max(0.0));
    (UiPoint::new(rect.x, y), UiPoint::new(rect.x + width, y))
}

fn approximate_unwrapped_text_width(text: &TextContent, scale: f32) -> f32 {
    let char_width = text.style.font_size * scale * 0.55;
    (text.text.chars().count() as f32 * char_width).max(char_width)
}

fn scaled_taffy_style(style: &Style, scale: f32) -> Style {
    let scale = normalized_scale(scale);
    if (scale - 1.0).abs() <= f32::EPSILON {
        return style.clone();
    }
    let mut style = style.clone();
    style.scrollbar_width *= scale;
    style.inset = scale_taffy_rect_auto(style.inset, scale);
    style.size = TaffySize {
        width: scale_dimension(style.size.width, scale),
        height: scale_dimension(style.size.height, scale),
    };
    style.min_size = TaffySize {
        width: scale_dimension(style.min_size.width, scale),
        height: scale_dimension(style.min_size.height, scale),
    };
    style.max_size = TaffySize {
        width: scale_dimension(style.max_size.width, scale),
        height: scale_dimension(style.max_size.height, scale),
    };
    style.margin = scale_taffy_rect_auto(style.margin, scale);
    style.padding = scale_taffy_rect(style.padding, scale);
    style.border = scale_taffy_rect(style.border, scale);
    style.gap = TaffySize {
        width: scale_length_percentage(style.gap.width, scale),
        height: scale_length_percentage(style.gap.height, scale),
    };
    style.flex_basis = scale_dimension(style.flex_basis, scale);
    style
}

fn normalize_intrinsic_root_style(style: &mut Style) {
    style.position = taffy::prelude::Position::Relative;
    style.inset = TaffyRect {
        left: LengthPercentageAuto::auto(),
        right: LengthPercentageAuto::auto(),
        top: LengthPercentageAuto::auto(),
        bottom: LengthPercentageAuto::auto(),
    };
    style.flex_grow = 0.0;
    style.flex_shrink = 0.0;
    style.flex_basis = Dimension::auto();
    if is_percent_dimension(style.size.width) {
        style.size.width = Dimension::auto();
    }
    if is_percent_dimension(style.size.height) {
        style.size.height = Dimension::auto();
    }
}

fn normalize_intrinsic_descendant_style(style: &mut Style) {
    style.flex_grow = 0.0;
    style.flex_shrink = 0.0;
    style.flex_basis = Dimension::auto();
    if is_percent_dimension(style.size.width) {
        style.size.width = Dimension::auto();
    }
    if is_percent_dimension(style.size.height) {
        style.size.height = Dimension::auto();
    }
}

fn is_percent_dimension(value: Dimension) -> bool {
    value.tag() == CompactLength::PERCENT_TAG
}

fn scale_dimension(value: Dimension, scale: f32) -> Dimension {
    let raw = value.into_raw();
    if raw.tag() == CompactLength::LENGTH_TAG {
        Dimension::length(raw.value() * scale)
    } else {
        value
    }
}

fn scale_length_percentage(value: LengthPercentage, scale: f32) -> LengthPercentage {
    let raw = value.into_raw();
    if raw.tag() == CompactLength::LENGTH_TAG {
        LengthPercentage::length(raw.value() * scale)
    } else {
        value
    }
}

fn scale_length_percentage_auto(value: LengthPercentageAuto, scale: f32) -> LengthPercentageAuto {
    let raw = value.into_raw();
    if raw.tag() == CompactLength::LENGTH_TAG {
        LengthPercentageAuto::length(raw.value() * scale)
    } else {
        value
    }
}

fn scale_taffy_rect(rect: TaffyRect<LengthPercentage>, scale: f32) -> TaffyRect<LengthPercentage> {
    TaffyRect {
        left: scale_length_percentage(rect.left, scale),
        right: scale_length_percentage(rect.right, scale),
        top: scale_length_percentage(rect.top, scale),
        bottom: scale_length_percentage(rect.bottom, scale),
    }
}

fn scale_taffy_rect_auto(
    rect: TaffyRect<LengthPercentageAuto>,
    scale: f32,
) -> TaffyRect<LengthPercentageAuto> {
    TaffyRect {
        left: scale_length_percentage_auto(rect.left, scale),
        right: scale_length_percentage_auto(rect.right, scale),
        top: scale_length_percentage_auto(rect.top, scale),
        bottom: scale_length_percentage_auto(rect.bottom, scale),
    }
}

fn paint_rect_for_node(rect: &PaintRect, node_rect: UiRect) -> PaintRect {
    let mut rect = rect.clone();
    if rect.rect.width <= f32::EPSILON || rect.rect.height <= f32::EPSILON {
        rect.rect = node_rect;
        rect.fill = rect.fill.translated(UiPoint::new(node_rect.x, node_rect.y));
        return rect;
    }
    rect.translated(UiPoint::new(node_rect.x, node_rect.y))
}

fn available_space_to_option(value: AvailableSpace) -> Option<f32> {
    match value {
        AvailableSpace::Definite(value) => Some(value),
        AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
    }
}

fn absolute_rect_from_style(style: &Style) -> Option<UiRect> {
    Some(UiRect::new(
        length_percentage_auto_points(style.inset.left)?,
        length_percentage_auto_points(style.inset.top)?,
        dimension_points(style.size.width)?,
        dimension_points(style.size.height)?,
    ))
}

fn dimension_points(value: Dimension) -> Option<f32> {
    if value.tag() == CompactLength::LENGTH_TAG {
        Some(value.value())
    } else {
        None
    }
}

fn max_length_dimension(current: Dimension, value: f32) -> Dimension {
    let value = value.max(0.0);
    match dimension_points(current) {
        Some(current) => Dimension::length(current.max(value)),
        None => Dimension::length(value),
    }
}

fn text_content_rect(rect: UiRect, style: &Style, scale: f32) -> UiRect {
    let scale = normalized_scale(scale);
    let padding = style.padding;
    let left = length_percentage_points(padding.left, rect.width, scale);
    let right = length_percentage_points(padding.right, rect.width, scale);
    let top = length_percentage_points(padding.top, rect.height, scale);
    let bottom = length_percentage_points(padding.bottom, rect.height, scale);
    let inset_width = (left + right).min(rect.width.max(0.0));
    let inset_height = (top + bottom).min(rect.height.max(0.0));
    UiRect::new(
        rect.x + left.min(rect.width.max(0.0)),
        rect.y + top.min(rect.height.max(0.0)),
        (rect.width - inset_width).max(0.0),
        (rect.height - inset_height).max(0.0),
    )
}

fn ceil_layout_size(size: UiSize) -> UiSize {
    UiSize::new(
        ceil_layout_extent(size.width),
        ceil_layout_extent(size.height),
    )
}

fn ceil_layout_extent(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0).ceil()
    } else {
        0.0
    }
}

fn constrain_intrinsic_extent(value: f32, min: Dimension, max: Dimension) -> f32 {
    let value = dimension_points(min)
        .map(|min| value.max(min))
        .unwrap_or(value);
    dimension_points(max)
        .map(|max| value.min(max))
        .unwrap_or(value)
}

fn resolve_intrinsic_child_width(style: &Style, parent_content_width: Option<f32>) -> Option<f32> {
    dimension_points(style.size.width).or_else(|| {
        parent_content_width.and_then(|basis| percent_dimension_points(style.size.width, basis))
    })
}

fn percent_dimension_points(value: Dimension, basis: f32) -> Option<f32> {
    let raw = value.into_raw();
    (raw.tag() == CompactLength::PERCENT_TAG).then(|| (basis * raw.value()).max(0.0))
}

fn box_horizontal_spacing(style: &Style, scale: f32) -> f32 {
    length_percentage_points(style.padding.left, 0.0, scale)
        + length_percentage_points(style.padding.right, 0.0, scale)
        + length_percentage_points(style.border.left, 0.0, scale)
        + length_percentage_points(style.border.right, 0.0, scale)
}

fn box_vertical_spacing(style: &Style, scale: f32) -> f32 {
    length_percentage_points(style.padding.top, 0.0, scale)
        + length_percentage_points(style.padding.bottom, 0.0, scale)
        + length_percentage_points(style.border.top, 0.0, scale)
        + length_percentage_points(style.border.bottom, 0.0, scale)
}

fn outer_intrinsic_size(size: UiSize, style: &Style) -> UiSize {
    UiSize::new(
        size.width
            + length_percentage_auto_points(style.margin.left)
                .unwrap_or(0.0)
                .max(0.0)
            + length_percentage_auto_points(style.margin.right)
                .unwrap_or(0.0)
                .max(0.0),
        size.height
            + length_percentage_auto_points(style.margin.top)
                .unwrap_or(0.0)
                .max(0.0)
            + length_percentage_auto_points(style.margin.bottom)
                .unwrap_or(0.0)
                .max(0.0),
    )
}

fn length_percentage_auto_points(value: LengthPercentageAuto) -> Option<f32> {
    let raw = value.into_raw();
    if raw.tag() == CompactLength::LENGTH_TAG {
        Some(raw.value())
    } else {
        None
    }
}

fn length_percentage_points(value: LengthPercentage, basis: f32, scale: f32) -> f32 {
    let raw = value.into_raw();
    if raw.tag() == CompactLength::LENGTH_TAG {
        (raw.value() * scale).max(0.0)
    } else if raw.tag() == CompactLength::PERCENT_TAG {
        (basis * raw.value()).max(0.0)
    } else {
        0.0
    }
}

fn finite_rect(rect: UiRect) -> UiRect {
    UiRect::new(
        finite_or(rect.x, 0.0),
        finite_or(rect.y, 0.0),
        finite_or(rect.width, 0.0).max(0.0),
        finite_or(rect.height, 0.0).max(0.0),
    )
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimatedValues {
    pub opacity: f32,
    pub translate: UiPoint,
    pub scale: f32,
    pub morph: f32,
}

impl AnimatedValues {
    pub const fn new(opacity: f32, translate: UiPoint, scale: f32) -> Self {
        Self {
            opacity,
            translate,
            scale,
            morph: 0.0,
        }
    }

    pub const fn with_morph(mut self, morph: f32) -> Self {
        self.morph = morph;
        self
    }

    fn lerp(self, to: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            opacity: self.opacity + (to.opacity - self.opacity) * t,
            translate: UiPoint::new(
                self.translate.x + (to.translate.x - self.translate.x) * t,
                self.translate.y + (to.translate.y - self.translate.y) * t,
            ),
            scale: self.scale + (to.scale - self.scale) * t,
            morph: self.morph + (to.morph - self.morph) * t,
        }
    }
}

impl Default for AnimatedValues {
    fn default() -> Self {
        Self::new(1.0, UiPoint::new(0.0, 0.0), 1.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationState {
    pub name: String,
    pub values: AnimatedValues,
}

impl AnimationState {
    pub fn new(name: impl Into<String>, values: AnimatedValues) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnimationTrigger {
    PointerEnter,
    PointerLeave,
    FocusGained,
    FocusLost,
    Pressed,
    Released,
    Custom(String),
}

pub const ANIMATION_INPUT_HOVER: &str = "hover";
pub const ANIMATION_INPUT_PRESSED: &str = "pressed";
pub const ANIMATION_INPUT_FOCUSED: &str = "focused";
pub const ANIMATION_INPUT_ACTIVE: &str = "active";
pub const ANIMATION_INPUT_ACTIVATED: &str = "activated";
pub const ANIMATION_INPUT_POINTER_X: &str = "pointer_x";
pub const ANIMATION_INPUT_POINTER_Y: &str = "pointer_y";
pub const ANIMATION_INPUT_POINTER_NORM_X: &str = "pointer_norm_x";
pub const ANIMATION_INPUT_POINTER_NORM_Y: &str = "pointer_norm_y";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationInputValue {
    Bool(bool),
    Number(f32),
    Trigger(bool),
}

impl AnimationInputValue {
    pub const fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    pub fn number(value: f32) -> Self {
        Self::Number(finite_or(value, 0.0))
    }

    pub const fn trigger() -> Self {
        Self::Trigger(false)
    }

    pub const fn fired_trigger() -> Self {
        Self::Trigger(true)
    }

    pub const fn as_bool(self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(value),
            Self::Number(_) | Self::Trigger(_) => None,
        }
    }

    pub const fn as_number(self) -> Option<f32> {
        match self {
            Self::Number(value) => Some(value),
            Self::Bool(_) | Self::Trigger(_) => None,
        }
    }

    pub const fn trigger_fired(self) -> bool {
        matches!(self, Self::Trigger(true))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationNumberComparison {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnimationCondition {
    Bool {
        input: String,
        value: bool,
    },
    Number {
        input: String,
        comparison: AnimationNumberComparison,
        value: f32,
    },
    Trigger {
        input: String,
    },
}

impl AnimationCondition {
    pub fn bool(input: impl Into<String>, value: bool) -> Self {
        Self::Bool {
            input: input.into(),
            value,
        }
    }

    pub fn number(
        input: impl Into<String>,
        comparison: AnimationNumberComparison,
        value: f32,
    ) -> Self {
        Self::Number {
            input: input.into(),
            comparison,
            value: finite_or(value, 0.0),
        }
    }

    pub fn trigger(input: impl Into<String>) -> Self {
        Self::Trigger {
            input: input.into(),
        }
    }

    fn matches(&self, inputs: &HashMap<String, AnimationInputValue>) -> bool {
        match self {
            Self::Bool { input, value } => inputs
                .get(input)
                .and_then(|input| input.as_bool())
                .is_some_and(|input| input == *value),
            Self::Number {
                input,
                comparison,
                value,
            } => inputs
                .get(input)
                .and_then(|input| input.as_number())
                .is_some_and(|input| compare_animation_number(input, *comparison, *value)),
            Self::Trigger { input } => inputs.get(input).is_some_and(|input| input.trigger_fired()),
        }
    }
}

fn compare_animation_number(input: f32, comparison: AnimationNumberComparison, value: f32) -> bool {
    match comparison {
        AnimationNumberComparison::LessThan => input < value,
        AnimationNumberComparison::LessThanOrEqual => input <= value,
        AnimationNumberComparison::GreaterThan => input > value,
        AnimationNumberComparison::GreaterThanOrEqual => input >= value,
        AnimationNumberComparison::Equal => (input - value).abs() <= f32::EPSILON,
        AnimationNumberComparison::NotEqual => (input - value).abs() > f32::EPSILON,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationTransition {
    pub from: String,
    pub to: String,
    pub trigger: Option<AnimationTrigger>,
    pub conditions: Vec<AnimationCondition>,
    pub duration_seconds: f32,
}

impl AnimationTransition {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        trigger: AnimationTrigger,
        duration_seconds: f32,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            trigger: Some(trigger),
            conditions: Vec::new(),
            duration_seconds,
        }
    }

    pub fn when(
        from: impl Into<String>,
        to: impl Into<String>,
        condition: AnimationCondition,
        duration_seconds: f32,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            trigger: None,
            conditions: vec![condition],
            duration_seconds,
        }
    }

    pub fn with_condition(mut self, condition: AnimationCondition) -> Self {
        self.conditions.push(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationBlendBinding {
    pub input: String,
    pub from: String,
    pub to: String,
    pub min: f32,
    pub max: f32,
}

impl AnimationBlendBinding {
    pub fn new(input: impl Into<String>, from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            from: from.into(),
            to: to.into(),
            min: 0.0,
            max: 1.0,
        }
    }

    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.min = finite_or(min, 0.0);
        self.max = finite_or(max, self.min + 1.0);
        if self.max < self.min {
            std::mem::swap(&mut self.min, &mut self.max);
        }
        self
    }

    fn amount(&self, inputs: &HashMap<String, AnimationInputValue>) -> Option<f32> {
        let value = inputs.get(&self.input)?.as_number()?;
        Some(((value - self.min) / (self.max - self.min).max(f32::EPSILON)).clamp(0.0, 1.0))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnimationTickOutcome {
    pub advanced: bool,
    pub active: bool,
    pub completed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnimationTickReport {
    pub ticked: usize,
    pub advanced: usize,
    pub active: usize,
    pub completed: usize,
}

#[derive(Debug, Clone)]
struct ActiveTransition {
    from_state: usize,
    from_values: AnimatedValues,
    to_state: usize,
    duration_seconds: f32,
    elapsed_seconds: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationActiveTransitionSnapshot {
    pub from_state: usize,
    pub from_state_name: String,
    pub to_state: usize,
    pub to_state_name: String,
    pub from_values: AnimatedValues,
    pub to_values: AnimatedValues,
    pub duration_seconds: f32,
    pub elapsed_seconds: f32,
    pub progress: f32,
}

#[derive(Debug, Clone)]
pub struct AnimationMachine {
    states: Vec<AnimationState>,
    transitions: Vec<AnimationTransition>,
    inputs: HashMap<String, AnimationInputValue>,
    blend_bindings: Vec<AnimationBlendBinding>,
    current_state: usize,
    active: Option<ActiveTransition>,
    values: AnimatedValues,
}

impl AnimationMachine {
    pub fn single_state(name: impl Into<String>, values: AnimatedValues) -> Self {
        let state = AnimationState::new(name, values);
        Self {
            states: vec![state],
            transitions: Vec::new(),
            inputs: HashMap::new(),
            blend_bindings: Vec::new(),
            current_state: 0,
            active: None,
            values,
        }
    }

    pub fn new(
        states: Vec<AnimationState>,
        transitions: Vec<AnimationTransition>,
        initial: &str,
    ) -> Result<Self, String> {
        let current_state = states
            .iter()
            .position(|state| state.name == initial)
            .ok_or_else(|| format!("initial animation state {initial:?} does not exist"))?;
        let values = states[current_state].values;
        Ok(Self {
            states,
            transitions,
            inputs: HashMap::new(),
            blend_bindings: Vec::new(),
            current_state,
            active: None,
            values,
        })
    }

    pub fn current_state_name(&self) -> &str {
        &self.states[self.current_state].name
    }

    pub fn states(&self) -> &[AnimationState] {
        &self.states
    }

    pub fn transitions(&self) -> &[AnimationTransition] {
        &self.transitions
    }

    pub fn inputs(&self) -> &HashMap<String, AnimationInputValue> {
        &self.inputs
    }

    pub fn blend_bindings(&self) -> &[AnimationBlendBinding] {
        &self.blend_bindings
    }

    pub fn values(&self) -> AnimatedValues {
        self.values
    }

    pub fn is_animating(&self) -> bool {
        self.active.is_some()
    }

    pub fn active_transition(&self) -> Option<AnimationActiveTransitionSnapshot> {
        let active = self.active.as_ref()?;
        let from_state = self.states.get(active.from_state)?;
        let to_state = self.states.get(active.to_state)?;
        let progress = if active.duration_seconds <= f32::EPSILON {
            1.0
        } else {
            (active.elapsed_seconds / active.duration_seconds).clamp(0.0, 1.0)
        };
        Some(AnimationActiveTransitionSnapshot {
            from_state: active.from_state,
            from_state_name: from_state.name.clone(),
            to_state: active.to_state,
            to_state_name: to_state.name.clone(),
            from_values: active.from_values,
            to_values: to_state.values,
            duration_seconds: active.duration_seconds,
            elapsed_seconds: active.elapsed_seconds,
            progress,
        })
    }

    pub fn has_same_definition(&self, other: &Self) -> bool {
        self.states == other.states
            && self.transitions == other.transitions
            && self.blend_bindings == other.blend_bindings
    }

    pub fn input(&self, name: &str) -> Option<AnimationInputValue> {
        self.inputs.get(name).copied()
    }

    pub fn with_input(mut self, name: impl Into<String>, value: AnimationInputValue) -> Self {
        self.set_input(name, value);
        self
    }

    pub fn with_bool_input(self, name: impl Into<String>, value: bool) -> Self {
        self.with_input(name, AnimationInputValue::bool(value))
    }

    pub fn with_number_input(self, name: impl Into<String>, value: f32) -> Self {
        self.with_input(name, AnimationInputValue::number(value))
    }

    pub fn with_trigger_input(self, name: impl Into<String>) -> Self {
        self.with_input(name, AnimationInputValue::trigger())
    }

    pub fn with_blend_binding(mut self, binding: AnimationBlendBinding) -> Self {
        self.blend_bindings.push(binding);
        self.apply_blend_bindings();
        self
    }

    pub fn set_input(&mut self, name: impl Into<String>, value: AnimationInputValue) -> bool {
        let name = name.into();
        let previous = self.inputs.insert(name, value);
        if previous == Some(value) {
            return false;
        }
        let transitioned = self.evaluate_input_transitions();
        if !transitioned {
            self.apply_blend_bindings();
        }
        self.consume_trigger_inputs();
        transitioned || previous != Some(value)
    }

    pub fn set_bool_input(&mut self, name: impl Into<String>, value: bool) -> bool {
        self.set_input(name, AnimationInputValue::bool(value))
    }

    pub fn set_number_input(&mut self, name: impl Into<String>, value: f32) -> bool {
        self.set_input(name, AnimationInputValue::number(value))
    }

    pub fn fire_trigger_input(&mut self, name: impl Into<String>) -> bool {
        self.set_input(name, AnimationInputValue::fired_trigger())
    }

    pub fn retain_runtime_from(&mut self, previous: &Self) -> bool {
        if !self.has_same_definition(previous) {
            return false;
        }
        let desired_inputs = self.inputs.clone();
        self.current_state = previous.current_state;
        self.active = previous.active.clone();
        self.values = previous.values;
        self.inputs = previous.inputs.clone();
        for (name, value) in desired_inputs {
            self.set_input(name, value);
        }
        if self.active.is_none() {
            self.apply_blend_bindings();
        }
        true
    }

    pub fn trigger(&mut self, trigger: AnimationTrigger) -> bool {
        let started = self.start_matching_transition(Some(&trigger));
        self.consume_trigger_inputs();
        started
    }

    fn evaluate_input_transitions(&mut self) -> bool {
        self.start_matching_transition(None)
    }

    fn start_matching_transition(&mut self, trigger: Option<&AnimationTrigger>) -> bool {
        let logical_state = self
            .active
            .as_ref()
            .map(|active| active.to_state)
            .unwrap_or(self.current_state);
        let current_name = self.states[logical_state].name.as_str();
        let Some(transition) = self
            .transitions
            .iter()
            .find(|transition| {
                transition.from == current_name
                    && transition_matches_trigger(transition.trigger.as_ref(), trigger)
                    && transition
                        .conditions
                        .iter()
                        .all(|condition| condition.matches(&self.inputs))
            })
            .cloned()
        else {
            return false;
        };
        let Some(to_state) = self
            .states
            .iter()
            .position(|state| state.name == transition.to)
        else {
            return false;
        };
        self.current_state = to_state;
        self.active = Some(ActiveTransition {
            from_state: logical_state,
            from_values: self.values,
            to_state,
            duration_seconds: transition.duration_seconds.max(0.0),
            elapsed_seconds: 0.0,
        });
        true
    }

    fn apply_blend_bindings(&mut self) -> bool {
        if self.active.is_some() {
            return false;
        }
        let mut changed = false;
        for binding in &self.blend_bindings {
            let Some(amount) = binding.amount(&self.inputs) else {
                continue;
            };
            let Some(from_state) = self.state_index(&binding.from) else {
                continue;
            };
            let Some(to_state) = self.state_index(&binding.to) else {
                continue;
            };
            let values = self.states[from_state]
                .values
                .lerp(self.states[to_state].values, amount);
            if self.values != values {
                self.values = values;
                changed = true;
            }
        }
        changed
    }

    fn state_index(&self, name: &str) -> Option<usize> {
        self.states.iter().position(|state| state.name == name)
    }

    fn consume_trigger_inputs(&mut self) {
        for input in self.inputs.values_mut() {
            if matches!(input, AnimationInputValue::Trigger(true)) {
                *input = AnimationInputValue::Trigger(false);
            }
        }
    }

    pub fn tick(&mut self, dt_seconds: f32) -> AnimationTickOutcome {
        let Some(active) = &mut self.active else {
            return AnimationTickOutcome::default();
        };
        let previous_values = self.values;
        let dt_seconds = if dt_seconds.is_finite() {
            dt_seconds.max(0.0)
        } else {
            0.0
        };
        active.elapsed_seconds = (active.elapsed_seconds + dt_seconds).max(0.0);
        let t = if active.duration_seconds <= f32::EPSILON {
            1.0
        } else {
            active.elapsed_seconds / active.duration_seconds
        };
        let target_values = self.states[active.to_state].values;
        self.values = active.from_values.lerp(target_values, t);
        if t >= 1.0 {
            self.current_state = active.to_state;
            self.values = target_values;
            self.active = None;
            self.evaluate_input_transitions();
            self.consume_trigger_inputs();
            if self.active.is_none() {
                self.apply_blend_bindings();
            }
        }
        AnimationTickOutcome {
            advanced: self.values != previous_values || t >= 1.0,
            active: self.active.is_some(),
            completed: t >= 1.0,
        }
    }
}

fn transition_matches_trigger(
    transition_trigger: Option<&AnimationTrigger>,
    requested: Option<&AnimationTrigger>,
) -> bool {
    match (transition_trigger, requested) {
        (Some(transition_trigger), Some(requested)) => transition_trigger == requested,
        (None, None) => true,
        _ => false,
    }
}

pub fn root_style(width: f32, height: f32) -> UiNodeStyle {
    UiNodeStyle {
        layout: LayoutStyle::from_taffy_style(Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: TaffySize {
                width: Dimension::length(width),
                height: Dimension::length(height),
            },
            ..Default::default()
        })
        .style,
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

pub fn length(value: f32) -> Dimension {
    Dimension::length(value)
}

#[cfg(feature = "egui-renderer-compat")]
pub fn egui_rect(rect: UiRect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::Pos2::new(rect.x, rect.y),
        egui::Vec2::new(rect.width, rect.height),
    )
}

#[cfg(feature = "egui-renderer-compat")]
pub fn egui_color(color: ColorRgba, opacity: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        color.r,
        color.g,
        color.b,
        ((color.a as f32) * opacity.clamp(0.0, 1.0)).round() as u8,
    )
}

#[cfg(feature = "egui-renderer-compat")]
pub fn paint_document_egui(document: &UiDocument, ctx: &egui::Context, layer: egui::LayerId) {
    paint_document_egui_impl(document, ctx, layer, None, None, None);
}

#[cfg(feature = "egui-renderer-compat")]
pub fn paint_document_egui_clipped(
    document: &UiDocument,
    ctx: &egui::Context,
    layer: egui::LayerId,
    clip_rect: UiRect,
) {
    paint_document_egui_impl(document, ctx, layer, Some(clip_rect), None, None);
}

#[cfg(feature = "egui-renderer-compat")]
pub fn paint_document_egui_with_canvas(
    document: &UiDocument,
    ctx: &egui::Context,
    layer: egui::LayerId,
    mut paint_canvas: impl FnMut(&CanvasContent, &PaintItem, &egui::Painter),
) {
    paint_document_egui_impl(document, ctx, layer, None, None, Some(&mut paint_canvas));
}

#[cfg(feature = "egui-renderer-compat")]
pub fn paint_document_egui_with_images(
    document: &UiDocument,
    ctx: &egui::Context,
    layer: egui::LayerId,
    mut paint_image: impl FnMut(&PaintImage, &PaintItem, &egui::Painter),
) {
    paint_document_egui_impl(document, ctx, layer, None, Some(&mut paint_image), None);
}

#[cfg(feature = "egui-renderer-compat")]
pub fn paint_document_egui_with_callbacks(
    document: &UiDocument,
    ctx: &egui::Context,
    layer: egui::LayerId,
    mut paint_image: impl FnMut(&PaintImage, &PaintItem, &egui::Painter),
    mut paint_canvas: impl FnMut(&CanvasContent, &PaintItem, &egui::Painter),
) {
    paint_document_egui_impl(
        document,
        ctx,
        layer,
        None,
        Some(&mut paint_image),
        Some(&mut paint_canvas),
    );
}

#[cfg(feature = "egui-renderer-compat")]
type EguiImageCallback<'a> = dyn FnMut(&PaintImage, &PaintItem, &egui::Painter) + 'a;

#[cfg(feature = "egui-renderer-compat")]
type EguiCanvasCallback<'a> = dyn FnMut(&CanvasContent, &PaintItem, &egui::Painter) + 'a;

#[cfg(feature = "egui-renderer-compat")]
fn paint_document_egui_impl(
    document: &UiDocument,
    ctx: &egui::Context,
    layer: egui::LayerId,
    outer_clip: Option<UiRect>,
    mut paint_image: Option<&mut EguiImageCallback<'_>>,
    mut paint_canvas: Option<&mut EguiCanvasCallback<'_>>,
) {
    let painter = ctx.layer_painter(layer);
    let mut simple_rect_batch = SimpleRectBatch::default();
    for item in document.paint_list().items {
        let Some(clip_rect) = (match outer_clip {
            Some(outer) => item.clip_rect.intersection(outer),
            None => Some(item.clip_rect),
        }) else {
            continue;
        };
        if clip_rect.width <= f32::EPSILON || clip_rect.height <= f32::EPSILON {
            continue;
        }
        let clip_rect = egui_rect(clip_rect);
        let rect = egui_rect(transform_rect(item.rect, item.transform));
        match &item.kind {
            PaintKind::Rect { .. } if simple_rect_batch.try_push(&item, rect, clip_rect) => {}
            PaintKind::Rect {
                fill,
                stroke,
                corner_radius,
            } => {
                simple_rect_batch.flush(&painter, outer_clip);
                let node_painter = painter.with_clip_rect(clip_rect);
                if fill.a > 0 {
                    node_painter.rect_filled(rect, *corner_radius, egui_color(*fill, item.opacity));
                }
                if let Some(stroke) = *stroke {
                    node_painter.rect_stroke(
                        rect,
                        *corner_radius,
                        egui::Stroke::new(stroke.width, egui_color(stroke.color, item.opacity)),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            PaintKind::RichRect(rect_primitive) => {
                simple_rect_batch.flush(&painter, outer_clip);
                paint_rich_rect_egui(
                    &painter.with_clip_rect(clip_rect),
                    rect,
                    rect_primitive,
                    item.opacity,
                );
            }
            PaintKind::Text(text) => {
                simple_rect_batch.flush(&painter, outer_clip);
                let text_painter = painter.with_clip_rect(clip_rect);
                text_painter.text(
                    egui::Pos2::new(rect.min.x, rect.min.y),
                    egui::Align2::LEFT_TOP,
                    &text.text,
                    egui_font_id(&text.style, item.transform.scale),
                    egui_color(text.style.color, item.opacity),
                );
            }
            PaintKind::SceneText(text) => {
                simple_rect_batch.flush(&painter, outer_clip);
                let node_painter = painter.with_clip_rect(clip_rect);
                let text_rect = egui_rect(transform_rect(text.rect, item.transform));
                node_painter.text(
                    scene_text_pos(text_rect, text),
                    scene_text_align(text),
                    scene_text_content(text),
                    egui_font_id(&text.style, item.transform.scale),
                    egui_color(text.style.color, item.opacity),
                );
            }
            PaintKind::Canvas(canvas) => {
                simple_rect_batch.flush(&painter, outer_clip);
                if let Some(callback) = paint_canvas.as_deref_mut() {
                    callback(canvas, &item, &painter.with_clip_rect(clip_rect));
                }
            }
            PaintKind::Line { from, to, stroke } => {
                simple_rect_batch.flush(&painter, outer_clip);
                painter.with_clip_rect(clip_rect).line_segment(
                    [
                        egui_pos(transform_point(*from, item.transform)),
                        egui_pos(transform_point(*to, item.transform)),
                    ],
                    egui::Stroke::new(stroke.width, egui_color(stroke.color, item.opacity)),
                );
            }
            PaintKind::Circle {
                center,
                radius,
                fill,
                stroke,
            } => {
                simple_rect_batch.flush(&painter, outer_clip);
                let node_painter = painter.with_clip_rect(clip_rect);
                let center = egui_pos(transform_point(*center, item.transform));
                let radius = radius * item.transform.scale.max(0.0);
                if fill.a > 0 {
                    node_painter.circle_filled(center, radius, egui_color(*fill, item.opacity));
                }
                if let Some(stroke) = *stroke {
                    node_painter.circle_stroke(
                        center,
                        radius,
                        egui::Stroke::new(stroke.width, egui_color(stroke.color, item.opacity)),
                    );
                }
            }
            PaintKind::Polygon {
                points,
                fill,
                stroke,
            } => {
                simple_rect_batch.flush(&painter, outer_clip);
                let points = points
                    .iter()
                    .copied()
                    .map(|point| egui_pos(transform_point(point, item.transform)))
                    .collect::<Vec<_>>();
                if fill.a > 0 && points.len() >= 3 {
                    painter
                        .with_clip_rect(clip_rect)
                        .add(egui::Shape::convex_polygon(
                            points.clone(),
                            egui_color(*fill, item.opacity),
                            egui::Stroke::NONE,
                        ));
                }
                if let Some(stroke) = *stroke {
                    painter.with_clip_rect(clip_rect).add(egui::Shape::line(
                        points,
                        egui::Stroke::new(stroke.width, egui_color(stroke.color, item.opacity)),
                    ));
                }
            }
            PaintKind::Image { key, tint } => {
                simple_rect_batch.flush(&painter, outer_clip);
                if let Some(callback) = paint_image.as_deref_mut() {
                    let mut image = PaintImage::new(key.clone(), item.rect);
                    image.tint = *tint;
                    callback(&image, &item, &painter.with_clip_rect(clip_rect));
                }
            }
            PaintKind::CompositedLayer(_) => {
                simple_rect_batch.flush(&painter, outer_clip);
            }
            PaintKind::Path(path) => {
                simple_rect_batch.flush(&painter, outer_clip);
                let points = paint_path_points(path)
                    .into_iter()
                    .map(|point| egui_pos(transform_point(point, item.transform)))
                    .collect::<Vec<_>>();
                if let Some(fill) = &path.fill {
                    if points.len() >= 3 {
                        painter
                            .with_clip_rect(clip_rect)
                            .add(egui::Shape::convex_polygon(
                                points.clone(),
                                egui_color(fill.fallback_color(), item.opacity),
                                egui::Stroke::NONE,
                            ));
                    }
                }
                if let Some(stroke) = path.stroke {
                    if points.len() >= 2 {
                        painter.with_clip_rect(clip_rect).add(egui::Shape::line(
                            points,
                            egui::Stroke::new(
                                stroke.style.width,
                                egui_color(stroke.style.color, item.opacity),
                            ),
                        ));
                    }
                }
            }
            PaintKind::ImagePlacement(image) => {
                simple_rect_batch.flush(&painter, outer_clip);
                if let Some(callback) = paint_image.as_deref_mut() {
                    callback(image, &item, &painter.with_clip_rect(clip_rect));
                }
            }
        }
    }
    simple_rect_batch.flush(&painter, outer_clip);
}

#[cfg(feature = "egui-renderer-compat")]
fn paint_rich_rect_egui(
    painter: &egui::Painter,
    rect: egui::Rect,
    rect_primitive: &PaintRect,
    opacity: f32,
) {
    let radius = rect_primitive.corner_radii.max_radius();
    for effect in &rect_primitive.effects {
        let color = egui_color(effect.color, opacity);
        match effect.kind {
            PaintEffectKind::Shadow | PaintEffectKind::Glow => {
                let spread = effect.spread.max(0.0) + effect.blur_radius.max(0.0) * 0.25;
                let effect_rect = rect
                    .expand(spread)
                    .translate(egui::vec2(effect.offset.x, effect.offset.y));
                painter.rect_filled(effect_rect, radius + spread, color);
            }
            PaintEffectKind::InsetShadow => {
                painter.rect_stroke(
                    rect.shrink(effect.spread.max(0.0)),
                    radius,
                    egui::Stroke::new(effect.blur_radius.max(1.0), color),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }

    let fill = rect_primitive.fill.fallback_color();
    if fill.a > 0 {
        painter.rect_filled(rect, radius, egui_color(fill, opacity));
    }
    if let Some(stroke) = rect_primitive.stroke {
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(stroke.style.width, egui_color(stroke.style.color, opacity)),
            egui_stroke_kind(stroke.alignment),
        );
    }
}

#[cfg(feature = "egui-renderer-compat")]
fn egui_stroke_kind(alignment: StrokeAlignment) -> egui::StrokeKind {
    match alignment {
        StrokeAlignment::Inside => egui::StrokeKind::Inside,
        StrokeAlignment::Center => egui::StrokeKind::Middle,
        StrokeAlignment::Outside => egui::StrokeKind::Outside,
    }
}

#[cfg(feature = "egui-renderer-compat")]
fn scene_text_pos(rect: egui::Rect, text: &PaintText) -> egui::Pos2 {
    let x = match text.horizontal_align {
        TextHorizontalAlign::Start => rect.min.x,
        TextHorizontalAlign::Center => rect.center().x,
        TextHorizontalAlign::End => rect.max.x,
    };
    let y = match text.vertical_align {
        TextVerticalAlign::Top | TextVerticalAlign::Baseline => rect.min.y,
        TextVerticalAlign::Center => rect.center().y,
        TextVerticalAlign::Bottom => rect.max.y,
    };
    egui::Pos2::new(x, y)
}

#[cfg(feature = "egui-renderer-compat")]
fn scene_text_align(text: &PaintText) -> egui::Align2 {
    match (text.horizontal_align, text.vertical_align) {
        (TextHorizontalAlign::Start, TextVerticalAlign::Top | TextVerticalAlign::Baseline) => {
            egui::Align2::LEFT_TOP
        }
        (TextHorizontalAlign::Center, TextVerticalAlign::Top | TextVerticalAlign::Baseline) => {
            egui::Align2::CENTER_TOP
        }
        (TextHorizontalAlign::End, TextVerticalAlign::Top | TextVerticalAlign::Baseline) => {
            egui::Align2::RIGHT_TOP
        }
        (TextHorizontalAlign::Start, TextVerticalAlign::Center) => egui::Align2::LEFT_CENTER,
        (TextHorizontalAlign::Center, TextVerticalAlign::Center) => egui::Align2::CENTER_CENTER,
        (TextHorizontalAlign::End, TextVerticalAlign::Center) => egui::Align2::RIGHT_CENTER,
        (TextHorizontalAlign::Start, TextVerticalAlign::Bottom) => egui::Align2::LEFT_BOTTOM,
        (TextHorizontalAlign::Center, TextVerticalAlign::Bottom) => egui::Align2::CENTER_BOTTOM,
        (TextHorizontalAlign::End, TextVerticalAlign::Bottom) => egui::Align2::RIGHT_BOTTOM,
    }
}

#[cfg(feature = "egui-renderer-compat")]
fn scene_text_content(text: &PaintText) -> &str {
    if text.multiline {
        &text.text
    } else {
        text.text.lines().next().unwrap_or("")
    }
}

#[cfg(feature = "egui-renderer-compat")]
fn paint_path_points(path: &PaintPath) -> Vec<UiPoint> {
    path.verbs
        .iter()
        .filter_map(|verb| match *verb {
            PathVerb::MoveTo(point) | PathVerb::LineTo(point) => Some(point),
            PathVerb::QuadraticTo { to, .. } | PathVerb::CubicTo { to, .. } => Some(to),
            PathVerb::Close => None,
        })
        .collect()
}

#[cfg(feature = "egui-renderer-compat")]
fn egui_pos(point: UiPoint) -> egui::Pos2 {
    egui::Pos2::new(point.x, point.y)
}

#[cfg(feature = "egui-renderer-compat")]
fn transform_point(point: UiPoint, transform: PaintTransform) -> UiPoint {
    transform.transform_point(point)
}

#[cfg(feature = "egui-renderer-compat")]
fn transform_rect(rect: UiRect, transform: PaintTransform) -> UiRect {
    transform.transform_rect(rect)
}

#[cfg(feature = "egui-renderer-compat")]
fn egui_font_id(style: &TextStyle, scale: f32) -> egui::FontId {
    let size = style.font_size * scale.max(0.0);
    match style.family {
        FontFamily::Monospace => egui::FontId::monospace(size),
        FontFamily::SansSerif | FontFamily::Serif | FontFamily::Named(_) => {
            egui::FontId::proportional(size)
        }
    }
}

#[cfg(feature = "egui-renderer-compat")]
#[derive(Default)]
struct SimpleRectBatch {
    mesh: egui::epaint::Mesh,
}

#[cfg(feature = "egui-renderer-compat")]
impl SimpleRectBatch {
    fn try_push(&mut self, item: &PaintItem, rect: egui::Rect, clip_rect: egui::Rect) -> bool {
        let PaintKind::Rect {
            fill,
            stroke,
            corner_radius,
        } = &item.kind
        else {
            return false;
        };
        let fill = *fill;
        let stroke = *stroke;
        let corner_radius = *corner_radius;
        if !rect_is_inside_clip(rect, clip_rect) || corner_radius > 2.0 {
            return false;
        }
        let has_fill = fill.a > 0;
        let has_stroke = stroke.is_some_and(|stroke| stroke.width > 0.0 && stroke.color.a > 0);
        if !has_fill && !has_stroke {
            return false;
        }
        if has_fill {
            self.mesh
                .add_colored_rect(rect, egui_color(fill, item.opacity));
        }
        if let Some(stroke) = stroke.filter(|stroke| stroke.width > 0.0 && stroke.color.a > 0) {
            add_inner_rect_stroke(
                &mut self.mesh,
                rect,
                stroke.width,
                egui_color(stroke.color, item.opacity),
            );
        }
        true
    }

    fn flush(&mut self, painter: &egui::Painter, outer_clip: Option<UiRect>) {
        if self.mesh.indices.is_empty() {
            return;
        }
        let mesh = std::mem::take(&mut self.mesh);
        if let Some(clip) = outer_clip {
            painter
                .with_clip_rect(egui_rect(clip))
                .add(egui::Shape::Mesh(mesh.into()));
        } else {
            painter.add(egui::Shape::Mesh(mesh.into()));
        }
    }
}

#[cfg(feature = "egui-renderer-compat")]
fn rect_is_inside_clip(rect: egui::Rect, clip_rect: egui::Rect) -> bool {
    rect.min.x >= clip_rect.min.x
        && rect.min.y >= clip_rect.min.y
        && rect.max.x <= clip_rect.max.x
        && rect.max.y <= clip_rect.max.y
}

#[cfg(feature = "egui-renderer-compat")]
fn add_inner_rect_stroke(
    mesh: &mut egui::epaint::Mesh,
    rect: egui::Rect,
    width: f32,
    color: egui::Color32,
) {
    let width = width
        .max(0.0)
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5);
    if width <= f32::EPSILON {
        return;
    }
    mesh.add_colored_rect(
        egui::Rect::from_min_max(
            rect.left_top(),
            egui::pos2(rect.right(), rect.top() + width),
        ),
        color,
    );
    mesh.add_colored_rect(
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.bottom() - width),
            rect.right_bottom(),
        ),
        color,
    );
    mesh.add_colored_rect(
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.top() + width),
            egui::pos2(rect.left() + width, rect.bottom() - width),
        ),
        color,
    );
    mesh.add_colored_rect(
        egui::Rect::from_min_max(
            egui::pos2(rect.right() - width, rect.top() + width),
            egui::pos2(rect.right(), rect.bottom() - width),
        ),
        color,
    );
}

#[cfg(test)]
mod tests {
    use taffy::prelude::{AlignItems, JustifyContent, LengthPercentageAuto, Position, Rect};

    use crate::effective_geometry::topmost_effective_hit;
    use crate::input::WheelPhase;
    use crate::*;

    use super::*;

    fn button_style(width: f32, height: f32) -> UiNodeStyle {
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(width),
                    height: length(height),
                },
                ..Default::default()
            })
            .style,
            ..Default::default()
        }
    }

    #[test]
    fn layout_helpers_cover_common_taffy_shapes() {
        let absolute = layout::absolute(12.0, 18.0, 80.0, 40.0);
        let absolute_taffy = absolute.as_taffy_style();
        assert_eq!(absolute_taffy.position, Position::Absolute);
        assert_eq!(
            absolute_taffy.inset.left,
            LengthPercentageAuto::length(12.0)
        );
        assert_eq!(absolute_taffy.inset.top, LengthPercentageAuto::length(18.0));
        assert_eq!(absolute_taffy.inset.right, LengthPercentageAuto::auto());
        assert_eq!(absolute_taffy.size.width, layout::px(80.0));
        assert_eq!(absolute_taffy.size.height, layout::px(40.0));

        let centered = layout::with_gap_all(layout::centered_row(), 6.0);
        let centered_taffy = centered.as_taffy_style();
        assert_eq!(centered_taffy.display, Display::Flex);
        assert_eq!(centered_taffy.flex_direction, FlexDirection::Row);
        assert_eq!(centered_taffy.align_items, Some(AlignItems::Center));
        assert_eq!(centered_taffy.justify_content, Some(JustifyContent::Center));
        assert_eq!(centered_taffy.gap.width, layout::spacing(6.0));
        assert_eq!(centered_taffy.gap.height, layout::spacing(6.0));

        let flex = layout::flex_item(2.0, 0.5, layout::px(64.0));
        let flex_taffy = flex.as_taffy_style();
        assert_eq!(flex_taffy.flex_grow, 2.0);
        assert_eq!(flex_taffy.flex_shrink, 0.5);
        assert_eq!(flex_taffy.flex_basis, layout::px(64.0));

        let constrained = layout::with_max_size(
            layout::with_min_size(layout::fill(), layout::px(120.0), layout::px(40.0)),
            layout::px(240.0),
            layout::auto(),
        );
        let constrained_taffy = constrained.as_taffy_style();
        assert_eq!(constrained_taffy.min_size.width, layout::px(120.0));
        assert_eq!(constrained_taffy.min_size.height, layout::px(40.0));
        assert_eq!(constrained_taffy.max_size.width, layout::px(240.0));
        assert_eq!(constrained_taffy.max_size.height, layout::auto());

        let node_style = layout::clipped_node_style(absolute);
        assert_eq!(node_style.clip, ClipBehavior::Clip);
        assert_eq!(node_style.layout.position, Position::Absolute);
    }

    #[test]
    fn color_contrast_helpers_support_accessible_text_selection() {
        let dark = ColorRgba::new(18, 22, 28, 255);
        let translucent = ColorRgba::new(255, 255, 255, 128);
        let composited = translucent.composite_over(dark);
        assert!(composited.r > dark.r);
        assert_eq!(
            ColorRgba::WHITE.contrast_ratio(ColorRgba::BLACK).round(),
            21.0
        );
        assert!(ColorRgba::WHITE.meets_contrast_ratio(dark, 4.5));
        assert!(!ColorRgba::new(44, 50, 58, 255).meets_contrast_ratio(dark, 4.5));
        assert_eq!(
            dark.highest_contrast_against(ColorRgba::WHITE, ColorRgba::BLACK),
            ColorRgba::WHITE
        );
    }

    #[test]
    fn ui_node_factories_accept_legacy_taffy_styles() {
        let legacy = Style {
            size: TaffySize {
                width: length(200.0),
                height: length(40.0),
            },
            ..Default::default()
        };
        let container = UiNode::container("legacy-container", legacy.clone());
        let text = UiNode::text("legacy-text", "label", TextStyle::default(), legacy.clone());
        let image = UiNode::image(
            "legacy-image",
            ImageContent::new("icons.render"),
            legacy.clone(),
        );
        let scene = UiNode::scene("legacy-scene", Vec::new(), legacy.clone());
        let canvas = UiNode::canvas("legacy-canvas", "canvas_key", legacy.clone());

        assert_eq!(container.style.layout.size, legacy.size);
        assert_eq!(text.style.layout.size, legacy.size);
        assert_eq!(image.style.layout.size, legacy.size);
        assert_eq!(scene.style.layout.size, legacy.size);
        assert_eq!(canvas.style.layout.size, legacy.size);
    }

    #[test]
    fn gpu_canvas_factory_attaches_texture_backed_context() {
        let canvas = CanvasContent::new("viewport").gpu_context();
        assert_eq!(canvas.context.kind, CanvasContextKind::Gpu);
        assert_eq!(canvas.render_mode, CanvasRenderMode::AttachedContext);
        assert!(canvas.context.kind.is_texture_backed());
        assert!(canvas.context.kind.is_gpu_backed());

        let node = UiNode::gpu_canvas("viewport", "app.viewport", layout::fixed(320.0, 180.0));
        let UiContent::Canvas(content) = node.content else {
            panic!("expected canvas content");
        };
        assert_eq!(content.key, "app.viewport");
        assert_eq!(content.surface_key(), "app.viewport");
        assert_eq!(content.context.kind, CanvasContextKind::Gpu);
        assert_eq!(content.render_mode, CanvasRenderMode::AttachedContext);
    }

    #[test]
    fn document_accepts_legacy_taffy_root_and_style_updates() {
        let legacy = Style {
            size: TaffySize {
                width: length(800.0),
                height: length(600.0),
            },
            ..Default::default()
        };
        let mut doc = UiDocument::new(legacy.clone());
        let child = doc.add_child(
            doc.root,
            UiNode::container(
                "child",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(120.0),
                            height: length(24.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    ..Default::default()
                },
            ),
        );
        let updated = Style {
            size: TaffySize {
                width: length(180.0),
                height: length(36.0),
            },
            ..Default::default()
        };
        doc.set_node_style(child, updated.clone());
        let child_style = &doc.node(child).style.layout;
        assert_eq!(child_style.size, updated.size);
        assert_eq!(doc.node(doc.root).style.layout.size, legacy.size);
    }

    #[test]
    fn taffy_layout_places_bottom_centered_hotbar() {
        let mut doc = UiDocument::new(root_style(800.0, 600.0));
        let hotbar = doc.add_child(
            doc.root,
            UiNode::container(
                "hotbar",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(360.0),
                            height: length(64.0),
                        },
                        margin: Rect {
                            left: LengthPercentageAuto::auto(),
                            right: LengthPercentageAuto::auto(),
                            top: LengthPercentageAuto::auto(),
                            bottom: LengthPercentageAuto::length(18.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        doc.compute_layout(UiSize::new(800.0, 600.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let rect = doc.node(hotbar).layout.rect;
        assert_eq!(rect.width, 360.0);
        assert_eq!(rect.height, 64.0);
        assert!((rect.x - 220.0).abs() < 0.01, "{rect:?}");
        assert!((rect.y - 518.0).abs() < 0.01, "{rect:?}");
    }

    #[test]
    fn text_nodes_are_measured_through_cosmic_text_facing_model() {
        let mut doc = UiDocument::new(root_style(300.0, 200.0));
        let text_style = TextStyle {
            family: FontFamily::Monospace,
            weight: FontWeight::BOLD,
            ..Default::default()
        };
        let text = doc.add_child(
            doc.root,
            UiNode::text(
                "label",
                "Inventory",
                text_style,
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                })
                .style,
            ),
        );
        doc.compute_layout(UiSize::new(300.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let rect = doc.node(text).layout.rect;
        assert!(rect.width > 0.0);
        assert!(rect.height > 0.0);
    }

    #[test]
    fn intrinsic_text_size_distinguishes_min_and_preferred_widths() {
        let mut doc = UiDocument::new(root_style(300.0, 200.0));
        let text = doc.add_child(
            doc.root,
            UiNode::text(
                "wrapping.label",
                "short LongIdentifierWithoutSpaces",
                TextStyle {
                    wrap: TextWrap::Word,
                    ..TextStyle::default()
                },
                LayoutStyle::new(),
            ),
        );
        let intrinsic = doc
            .intrinsic_size(text, &mut ApproxTextMeasurer)
            .expect("intrinsic size");

        assert!(intrinsic.min.width > 0.0, "{intrinsic:?}");
        assert!(
            intrinsic.preferred.width > intrinsic.min.width,
            "{intrinsic:?}"
        );
    }

    #[test]
    fn scene_intrinsic_size_includes_local_primitive_bounds() {
        let mut doc = UiDocument::new(root_style(320.0, 160.0));
        let scene = doc.add_child(
            doc.root,
            UiNode::scene(
                "scene",
                vec![
                    ScenePrimitive::Rect(
                        PaintRect::solid(
                            UiRect::new(24.0, 18.0, 80.0, 32.0),
                            ColorRgba::new(90, 120, 200, 255),
                        )
                        .stroke(AlignedStroke::outside(StrokeStyle::new(
                            ColorRgba::WHITE,
                            2.0,
                        )))
                        .effect(PaintEffect::shadow(
                            ColorRgba::new(0, 0, 0, 180),
                            UiPoint::new(6.0, 4.0),
                            8.0,
                            3.0,
                        )),
                    ),
                    ScenePrimitive::Text(PaintText::new(
                        "Label",
                        UiRect::new(32.0, 24.0, 96.0, 20.0),
                        TextStyle::default(),
                    )),
                ],
                LayoutStyle::new().with_height(80.0),
            ),
        );

        let intrinsic = doc
            .intrinsic_size(scene, &mut ApproxTextMeasurer)
            .expect("scene intrinsic size");

        assert!(intrinsic.min.width >= 128.0, "{intrinsic:?}");
        assert!(intrinsic.min.height >= 65.0, "{intrinsic:?}");
        assert_eq!(intrinsic.min, intrinsic.preferred);
    }

    #[test]
    fn inline_intrinsic_constraint_publishes_chrome_plus_text_width() {
        let mut doc = UiDocument::new(root_style(300.0, 200.0));
        let control = doc.add_child(
            doc.root,
            UiNode::container(
                "inline.control",
                UiNodeStyle {
                    layout: LayoutStyle::row().with_height(28.0).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        doc.add_child(
            control,
            UiNode::container(
                "inline.control.chrome",
                LayoutStyle::size(16.0, 16.0).with_flex_shrink(0.0),
            ),
        );
        let label = doc.add_child(
            control,
            UiNode::text(
                "inline.control.label",
                "Inline label",
                TextStyle::default(),
                LayoutStyle::new(),
            ),
        );
        doc.node_mut(control).layout_constraint =
            Some(UiNodeLayoutConstraint::InlineIntrinsicSize {
                sources: vec![label],
                min_size: UiSize::new(24.0, 28.0),
            });

        doc.compute_layout(UiSize::new(300.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let control_rect = doc.node(control).layout.rect;
        let label_rect = doc.node(label).layout.rect;
        assert!(
            control_rect.width >= label_rect.width + 24.0,
            "{control_rect:?} {label_rect:?}"
        );
        assert!(!doc.audit_layout().iter().any(|warning| matches!(
            warning,
            AuditWarning::TextClipped { name, .. } if name == "inline.control.label"
        )));
    }

    #[test]
    fn layout_pipeline_computes_sizes_before_positions_and_paint() {
        let viewport = UiSize::new(300.0, 200.0);
        let mut doc = UiDocument::new(root_style(viewport.width, viewport.height));
        let control = doc.add_child(
            doc.root,
            UiNode::container(
                "pipeline.control",
                UiNodeStyle {
                    layout: LayoutStyle::row().with_height(28.0).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        let label = doc.add_child(
            control,
            UiNode::text(
                "pipeline.control.label",
                "Measured before positioned",
                TextStyle {
                    wrap: TextWrap::None,
                    ..Default::default()
                },
                LayoutStyle::new(),
            ),
        );
        doc.node_mut(control).layout_constraint =
            Some(UiNodeLayoutConstraint::InlineIntrinsicSize {
                sources: vec![label],
                min_size: UiSize::new(24.0, 28.0),
            });

        assert_eq!(doc.node(control).layout, ComputedLayout::default());
        let sizing = doc
            .compute_layout_sizing_pass(viewport, &mut ApproxTextMeasurer)
            .expect("sizing pass");

        assert_eq!(doc.node(control).layout, ComputedLayout::default());
        assert!(
            doc.paint_list().items.is_empty(),
            "paint must wait until positions are applied"
        );

        let control_size = sizing.sizes.get(control.0).copied().expect("control size");
        let label_size = sizing.sizes.get(label.0).copied().expect("label size");
        assert!(
            control_size.width >= label_size.width + 24.0,
            "{control_size:?} {label_size:?}"
        );

        doc.apply_layout_position_pass(&sizing, viewport)
            .expect("position pass");
        let control_rect = doc.node(control).layout.rect;
        assert!(
            control_rect.width >= label_size.width + 24.0,
            "{control_rect:?} {label_size:?}"
        );
        assert!(doc.paint_list().items.iter().any(|item| item.node == label));
    }

    #[test]
    fn document_ui_scale_applies_to_layout_and_text_paint() {
        let mut doc = UiDocument::new(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0),
        );
        doc.set_ui_scale(1.5);
        let text = doc.add_child(
            doc.root,
            UiNode::text(
                "scaled",
                "Zoom",
                TextStyle {
                    font_size: 12.0,
                    line_height: 14.0,
                    ..Default::default()
                },
                LayoutStyle::size(100.0, 20.0),
            ),
        );

        doc.compute_layout(UiSize::new(300.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let rect = doc.node(text).layout.rect;
        assert!((rect.width - 150.0).abs() < 0.01, "{rect:?}");
        assert!((rect.height - 30.0).abs() < 0.01, "{rect:?}");

        let paint = doc.paint_list();
        let Some(PaintKind::Text(text)) = paint
            .items
            .iter()
            .find(|item| item.node == text)
            .map(|item| &item.kind)
        else {
            panic!("missing scaled text paint");
        };
        assert!((text.style.font_size - 18.0).abs() < 0.01);
        assert!((text.style.line_height - 21.0).abs() < 0.01);
    }

    #[test]
    fn text_paint_uses_layout_padding_as_content_inset() {
        let mut doc = UiDocument::new(root_style(160.0, 80.0));
        let text = doc.add_child(
            doc.root,
            UiNode::text(
                "padded",
                "FPS",
                TextStyle::default(),
                LayoutStyle::size(80.0, 32.0).padding(5.0),
            ),
        );

        doc.compute_layout(UiSize::new(160.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let paint = doc.paint_list();
        let text_item = paint
            .items
            .iter()
            .find(|item| item.node == text && matches!(item.kind, PaintKind::Text(_)))
            .expect("text paint");
        assert_eq!(text_item.rect, UiRect::new(5.0, 5.0, 70.0, 22.0));
    }

    #[test]
    fn document_localization_policy_updates_text_paint_metadata() {
        let mut doc = UiDocument::new(root_style(300.0, 100.0));
        let text = doc.add_child(
            doc.root,
            UiNode::text(
                "plain",
                "Plain",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            ),
        );
        let policy = LocalizationPolicy::new(LocaleId::new("ar-EG").expect("locale"))
            .with_bidi(BidiPolicy::Embed);

        doc.apply_localization_policy(&policy);
        doc.compute_layout(UiSize::new(300.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let paint = doc.paint_list();
        let text_item = paint
            .items
            .iter()
            .find(|item| item.node == text)
            .expect("text paint");
        let PaintKind::Text(content) = &text_item.kind else {
            panic!("expected text paint");
        };
        assert_eq!(content.locale.as_ref().map(LocaleId::as_str), Some("ar-EG"));
        assert_eq!(content.direction, ResolvedTextDirection::Rtl);
        assert_eq!(content.bidi, BidiPolicy::Embed);
    }

    #[test]
    fn mutating_nodes_invalidates_cached_layout() {
        let mut doc = UiDocument::new(root_style(300.0, 200.0));
        let child = doc.add_child(
            doc.root,
            UiNode::container("panel", button_style(80.0, 40.0)),
        );
        doc.compute_layout(UiSize::new(300.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");
        assert_eq!(doc.node(child).layout.rect.width, 80.0);

        doc.node_mut(child).style.layout.size.width = length(120.0);
        doc.compute_layout(UiSize::new(300.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(doc.node(child).layout.rect.width, 120.0);
    }

    #[cfg(feature = "text-cosmic")]
    #[test]
    fn cosmic_text_measurer_wraps_text_under_constraints() {
        let style = TextStyle {
            font_size: 16.0,
            line_height: 20.0,
            wrap: TextWrap::WordOrGlyph,
            ..Default::default()
        };
        let mut measurer = CosmicTextMeasurer::new();
        let measured = measurer.measure(
            &TextContent::new(
                "Glyphon delegates layout to cosmic text for player UI labels",
                style,
            ),
            KnownSize {
                width: None,
                height: None,
            },
            AvailableSize {
                width: Some(96.0),
                height: None,
            },
        );

        assert!(measured.width <= 96.0, "{measured:?}");
        assert!(measured.height > 20.0, "{measured:?}");
    }

    #[cfg(feature = "text-cosmic")]
    #[test]
    fn cosmic_text_measurer_includes_embedded_default_fonts() {
        let measurer = CosmicTextMeasurer::new();
        let families = measurer
            .font_system
            .db()
            .faces()
            .flat_map(|face| face.families.iter().map(|(name, _)| name.as_str()))
            .collect::<Vec<_>>();

        assert!(
            families.iter().any(|name| *name == "Ubuntu"),
            "embedded sans-serif font was not loaded: {families:?}"
        );
        assert!(
            families.iter().any(|name| *name == "Hack"),
            "embedded monospace font was not loaded: {families:?}"
        );
    }

    #[test]
    fn clipping_limits_hit_testing_to_visible_rect() {
        let mut doc = UiDocument::new(root_style(200.0, 200.0));
        let clip_parent = doc.add_child(
            doc.root,
            UiNode::container(
                "clip",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(100.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        let child = doc.add_child(
            clip_parent,
            UiNode::container(
                "oversized_button",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(160.0),
                            height: length(80.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON),
        );
        doc.compute_layout(UiSize::new(200.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(doc.hit_test(UiPoint::new(90.0, 40.0)), Some(child));
        assert_eq!(doc.hit_test(UiPoint::new(140.0, 40.0)), None);
    }

    #[test]
    fn hit_testing_uses_animation_transform_rect() {
        let animation = AnimationMachine::new(
            vec![AnimationState::new(
                "shown",
                AnimatedValues::new(1.0, UiPoint::new(30.0, 0.0), 2.0),
            )],
            Vec::new(),
            "shown",
        )
        .expect("animation");
        let mut doc = UiDocument::new(root_style(160.0, 100.0));
        let button = doc.add_child(
            doc.root,
            UiNode::container("toast_action", button_style(40.0, 20.0))
                .with_input(InputBehavior::BUTTON)
                .with_animation(animation),
        );
        doc.compute_layout(UiSize::new(160.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(
            doc.node(button).layout.rect,
            UiRect::new(0.0, 0.0, 40.0, 20.0)
        );
        assert_eq!(doc.hit_test(UiPoint::new(20.0, 10.0)), None);
        assert_eq!(doc.hit_test(UiPoint::new(95.0, 30.0)), Some(button));
    }

    #[test]
    fn document_effective_geometries_feed_hit_testing_and_accessibility_bounds() {
        let animation = AnimationMachine::new(
            vec![AnimationState::new(
                "shown",
                AnimatedValues::new(1.0, UiPoint::new(30.0, 0.0), 2.0),
            )],
            Vec::new(),
            "shown",
        )
        .expect("animation");
        let mut doc = UiDocument::new(root_style(160.0, 100.0));
        let clip_parent = doc.add_child(
            doc.root,
            UiNode::container(
                "clip",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(80.0),
                            height: length(80.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        let button = doc.add_child(
            clip_parent,
            UiNode::container("transformed", button_style(40.0, 20.0))
                .with_input(InputBehavior::BUTTON)
                .with_animation(animation),
        );
        doc.compute_layout(UiSize::new(160.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let geometries = doc.effective_geometries();
        let geometry = geometries
            .iter()
            .find(|geometry| geometry.node == button)
            .expect("button geometry");

        assert_eq!(
            geometry.transformed_bounds(),
            UiRect::new(30.0, 0.0, 80.0, 40.0)
        );
        assert_eq!(
            geometry.visible_rect(),
            Some(UiRect::new(30.0, 0.0, 50.0, 40.0))
        );
        assert_eq!(
            geometry.accessibility_bounds().map(|bounds| bounds.rect),
            Some(UiRect::new(30.0, 0.0, 50.0, 40.0))
        );
        assert_eq!(
            topmost_effective_hit(&geometries, UiPoint::new(75.0, 20.0)).map(|hit| hit.node),
            Some(button)
        );
        assert_eq!(doc.hit_test(UiPoint::new(75.0, 20.0)), Some(button));
        assert_eq!(doc.hit_test(UiPoint::new(95.0, 20.0)), None);
    }

    #[test]
    fn hit_testing_uses_effective_paint_z_order() {
        let mut doc = UiDocument::new(root_style(240.0, 160.0));
        let under = doc.add_child(
            doc.root,
            UiNode::container(
                "under",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(100.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    z_index: 5,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON),
        );
        let overlay = doc.add_child(
            doc.root,
            UiNode::container(
                "overlay",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(100.0),
                        },
                        margin: Rect {
                            top: LengthPercentageAuto::length(-100.0),
                            ..Rect::length(0.0)
                        },
                        ..Default::default()
                    })
                    .style,
                    z_index: 10,
                    ..Default::default()
                },
            ),
        );
        let over_child = doc.add_child(
            overlay,
            UiNode::container("overlay_child", button_style(100.0, 100.0))
                .with_input(InputBehavior::BUTTON),
        );
        doc.compute_layout(UiSize::new(240.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(doc.hit_test(UiPoint::new(10.0, 10.0)), Some(over_child));
        assert_ne!(doc.hit_test(UiPoint::new(10.0, 10.0)), Some(under));
    }

    #[test]
    fn hit_testing_and_paint_order_use_platform_layers_before_local_z() {
        let mut doc = UiDocument::new(root_style(240.0, 160.0));
        let app_overlay = doc.add_child(
            doc.root,
            UiNode::container(
                "app_overlay",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        position: Position::Absolute,
                        inset: Rect {
                            left: LengthPercentageAuto::length(0.0),
                            top: LengthPercentageAuto::length(0.0),
                            ..Rect::length(0.0)
                        },
                        size: TaffySize {
                            width: length(100.0),
                            height: length(100.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    z_index: platform::LAYER_LOCAL_Z_MAX,
                    ..Default::default()
                },
            )
            .with_layer(platform::UiLayer::AppOverlay)
            .with_input(InputBehavior::BUTTON)
            .with_visual(UiVisual::panel(ColorRgba::new(20, 80, 140, 255), None, 0.0)),
        );
        let debug_overlay = doc.add_child(
            doc.root,
            UiNode::container(
                "debug_overlay",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        position: Position::Absolute,
                        inset: Rect {
                            left: LengthPercentageAuto::length(0.0),
                            top: LengthPercentageAuto::length(0.0),
                            ..Rect::length(0.0)
                        },
                        size: TaffySize {
                            width: length(100.0),
                            height: length(100.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    z_index: platform::LAYER_LOCAL_Z_MIN,
                    ..Default::default()
                },
            )
            .with_layer(platform::UiLayer::DebugOverlay)
            .with_input(InputBehavior::BUTTON)
            .with_visual(UiVisual::panel(ColorRgba::new(180, 40, 40, 255), None, 0.0)),
        );
        doc.compute_layout(UiSize::new(240.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(doc.hit_test(UiPoint::new(10.0, 10.0)), Some(debug_overlay));

        let paint = doc.paint_list();
        assert_eq!(
            paint.items.iter().map(|item| item.node).collect::<Vec<_>>(),
            vec![app_overlay, debug_overlay]
        );
        assert_eq!(
            paint.items[0].layer_order,
            platform::LayerOrder::new(platform::UiLayer::AppOverlay, platform::LAYER_LOCAL_Z_MAX)
        );
        assert_eq!(
            paint.items[1].layer_order,
            platform::LayerOrder::new(platform::UiLayer::DebugOverlay, platform::LAYER_LOCAL_Z_MIN)
        );
        assert!(paint.items[0].layer_order < paint.items[1].layer_order);
    }

    #[test]
    fn viewport_clip_scope_escapes_ancestor_clipping() {
        let mut doc = UiDocument::new(root_style(240.0, 160.0));
        let scroll = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 120.0, 60.0)).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        let clipped = doc.add_child(
            scroll,
            UiNode::container(
                "clipped",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(12.0, 84.0, 80.0, 32.0)).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON),
        );
        let overlay = doc.add_child(
            scroll,
            UiNode::container(
                "overlay",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(12.0, 84.0, 80.0, 32.0)).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_layer(platform::UiLayer::AppOverlay)
            .with_clip_scope(ClipScope::Viewport)
            .with_input(InputBehavior::BUTTON),
        );

        doc.compute_layout(UiSize::new(240.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let clipped_layout = doc.node(clipped).layout;
        assert!(!clipped_layout.visible);
        assert!(clipped_layout.clip_rect.width <= f32::EPSILON);
        assert!(clipped_layout.clip_rect.height <= f32::EPSILON);

        let overlay_layout = doc.node(overlay).layout;
        assert!(overlay_layout.visible);
        assert_eq!(overlay_layout.clip_rect, overlay_layout.rect);
        assert_eq!(doc.hit_test(UiPoint::new(40.0, 112.0)), Some(overlay));
    }

    #[test]
    fn viewport_clip_scope_does_not_expand_scroll_content() {
        let mut doc = UiDocument::new(root_style(240.0, 160.0));
        let scroll = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 120.0, 60.0)).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            scroll,
            UiNode::container(
                "overlay",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(12.0, 140.0, 80.0, 32.0)).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_layer(platform::UiLayer::AppOverlay)
            .with_clip_scope(ClipScope::Viewport),
        );

        doc.compute_layout(UiSize::new(240.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let scroll = doc.scroll_state(scroll).expect("scroll state");
        assert_eq!(scroll.viewport_size, UiSize::new(120.0, 60.0));
        assert_eq!(scroll.content_size, UiSize::new(120.0, 60.0));
    }

    #[test]
    fn nested_scroll_content_does_not_expand_parent_scroll_range() {
        let mut doc = UiDocument::new(root_style(240.0, 160.0));
        let outer = doc.add_child(
            doc.root,
            UiNode::container(
                "outer.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 120.0, 60.0)).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        let inner = doc.add_child(
            outer,
            UiNode::container(
                "inner.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::size(80.0, 40.0).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::HORIZONTAL),
        );
        doc.add_child(
            inner,
            UiNode::container(
                "inner.content",
                LayoutStyle::size(260.0, 40.0).with_flex_shrink(0.0),
            ),
        );

        doc.compute_layout(UiSize::new(240.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let outer_scroll = doc.scroll_state(outer).expect("outer scroll state");
        let inner_scroll = doc.scroll_state(inner).expect("inner scroll state");
        assert_eq!(outer_scroll.viewport_size, UiSize::new(120.0, 60.0));
        assert_eq!(outer_scroll.content_size, UiSize::new(120.0, 60.0));
        assert_eq!(inner_scroll.viewport_size, UiSize::new(80.0, 40.0));
        assert_eq!(inner_scroll.content_size, UiSize::new(260.0, 40.0));
    }

    #[test]
    fn app_overlay_portal_host_is_created_once_and_receives_children() {
        let mut doc = UiDocument::new(root_style(240.0, 160.0));
        let first = doc.add_portal_child(
            doc.root,
            UiPortalTarget::AppOverlay,
            UiNode::container(
                "first.overlay",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(10.0, 10.0, 30.0, 20.0)).style,
                    ..Default::default()
                },
            ),
        );
        let second = doc.add_portal_child(
            doc.root,
            UiPortalTarget::AppOverlay,
            UiNode::container(
                "second.overlay",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(40.0, 10.0, 30.0, 20.0)).style,
                    ..Default::default()
                },
            ),
        );

        let host = doc
            .portal_host(APP_OVERLAY_PORTAL)
            .expect("app overlay portal host");
        assert_eq!(doc.node(host).parent, Some(doc.root));
        assert_eq!(doc.node(host).layer, Some(platform::UiLayer::AppOverlay));
        assert_eq!(doc.node(host).clip_scope, ClipScope::Viewport);
        assert_eq!(doc.node(first).parent, Some(host));
        assert_eq!(doc.node(second).parent, Some(host));
        assert_eq!(doc.node(host).children, vec![first, second]);
    }

    #[test]
    fn named_portal_targets_registered_hosts_and_falls_back_to_parent() {
        let mut doc = UiDocument::new(root_style(240.0, 160.0));
        let local_parent = doc.add_child(
            doc.root,
            UiNode::container(
                "local",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(0.0, 0.0, 120.0, 80.0)).style,
                    ..Default::default()
                },
            ),
        );
        let host = doc.add_child(
            doc.root,
            UiNode::container(
                "menu.portal",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(0.0, 0.0, 240.0, 160.0)).style,
                    ..Default::default()
                },
            )
            .with_clip_scope(ClipScope::Viewport),
        );
        doc.register_portal_host("menus", host);

        let routed = doc.add_portal_child(
            local_parent,
            UiPortalTarget::named("menus"),
            UiNode::container("routed", LayoutStyle::size(20.0, 20.0)),
        );
        let fallback = doc.add_portal_child(
            local_parent,
            UiPortalTarget::named("missing"),
            UiNode::container("fallback", LayoutStyle::size(20.0, 20.0)),
        );

        assert_eq!(doc.node(routed).parent, Some(host));
        assert_eq!(doc.node(fallback).parent, Some(local_parent));
    }

    #[test]
    fn scroll_area_tracks_content_size_and_offsets_children() {
        let mut doc = UiDocument::new(root_style(120.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "events",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(60.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        let row = doc.add_child(
            scroll_area,
            UiNode::container("row", button_style(100.0, 120.0)).with_input(InputBehavior::BUTTON),
        );
        doc.compute_layout(UiSize::new(120.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let scroll = doc.scroll_state(scroll_area).expect("scroll state");
        assert_eq!(scroll.viewport_size, UiSize::new(100.0, 60.0));
        assert_eq!(scroll.content_size, UiSize::new(100.0, 120.0));

        let input = doc.handle_input(UiInputEvent::wheel(
            UiPoint::new(10.0, 10.0),
            UiPoint::new(0.0, 30.0),
        ));
        assert_eq!(input.scrolled, Some(scroll_area));

        doc.compute_layout(UiSize::new(120.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");
        assert_eq!(doc.node(row).layout.rect.y, -30.0);
        assert_eq!(doc.hit_test(UiPoint::new(10.0, 90.0)), None);
    }

    #[test]
    fn audit_layout_allows_scroll_overflow_only_on_enabled_axis() {
        let mut doc = UiDocument::new(root_style(120.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "vertical.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::size(100.0, 60.0).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            scroll_area,
            UiNode::text(
                "vertical.scroll.wide_text",
                "Wide text",
                TextStyle::default(),
                LayoutStyle::size(180.0, 20.0).with_flex_shrink(0.0),
            ),
        );
        doc.compute_layout(UiSize::new(120.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert!(doc.audit_layout().iter().any(|warning| matches!(
            warning,
            AuditWarning::TextClipped { name, .. } if name == "vertical.scroll.wide_text"
        )));
    }

    #[test]
    fn audit_layout_suppresses_text_clipping_on_enabled_scroll_axis() {
        let mut doc = UiDocument::new(root_style(120.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "vertical.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::size(100.0, 60.0).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            scroll_area,
            UiNode::text(
                "vertical.scroll.tall_text",
                "Tall text",
                TextStyle::default(),
                LayoutStyle::size(100.0, 120.0).with_flex_shrink(0.0),
            ),
        );
        doc.compute_layout(UiSize::new(120.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert!(!doc.audit_layout().iter().any(|warning| matches!(
            warning,
            AuditWarning::TextClipped { name, .. } if name == "vertical.scroll.tall_text"
        )));
    }

    #[test]
    fn audit_layout_uses_scroll_viewport_for_unscrollable_axis_bounds() {
        let mut doc = UiDocument::new(root_style(120.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "vertical.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::size(100.0, 60.0).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            scroll_area,
            UiNode::text(
                "vertical.scroll.offscreen_text",
                "Offscreen",
                TextStyle::default(),
                LayoutStyle::absolute_rect(UiRect::new(0.0, 80.0, 100.0, 20.0)),
            ),
        );
        doc.add_child(
            scroll_area,
            UiNode::text(
                "vertical.scroll.offscreen_wide_text",
                "Wide offscreen",
                TextStyle::default(),
                LayoutStyle::absolute_rect(UiRect::new(0.0, 110.0, 140.0, 20.0)),
            ),
        );
        doc.compute_layout(UiSize::new(120.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let warnings = doc.audit_layout();

        assert!(!warnings.iter().any(|warning| matches!(
            warning,
            AuditWarning::TextClipped { name, .. } if name == "vertical.scroll.offscreen_text"
        )));
        assert!(warnings.iter().any(|warning| matches!(
            warning,
            AuditWarning::TextClipped { name, .. } if name == "vertical.scroll.offscreen_wide_text"
        )));
    }

    #[test]
    fn wheel_scrolls_blank_space_inside_scroll_region() {
        let mut doc = UiDocument::new(root_style(120.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(60.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            scroll_area,
            UiNode::container("content", button_style(100.0, 140.0)),
        );
        doc.compute_layout(UiSize::new(120.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let input = doc.handle_input(UiInputEvent::wheel(
            UiPoint::new(90.0, 50.0),
            UiPoint::new(0.0, 30.0),
        ));

        assert_eq!(input.scrolled, Some(scroll_area));
        assert_eq!(doc.scroll_state(scroll_area).unwrap().offset.y, 30.0);
    }

    #[test]
    fn wheel_scroll_is_eaten_by_non_scrollable_top_hit() {
        let mut doc = UiDocument::new(root_style(160.0, 140.0));
        let back_scroll = doc.add_child(
            doc.root,
            UiNode::container(
                "back.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 100.0, 60.0)).style,
                    clip: ClipBehavior::Clip,
                    z_index: 1,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            back_scroll,
            UiNode::container("back.content", button_style(100.0, 140.0)),
        );
        let front = doc.add_child(
            doc.root,
            UiNode::container(
                "front.panel",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(10.0, 10.0, 120.0, 90.0)).style,
                    clip: ClipBehavior::Clip,
                    z_index: 10,
                    ..Default::default()
                },
            )
            .with_visual(UiVisual::panel(ColorRgba::new(28, 34, 44, 255), None, 0.0)),
        );
        doc.compute_layout(UiSize::new(160.0, 140.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(doc.wheel_event_scope(UiPoint::new(30.0, 30.0)), Some(front));
        let input = doc.handle_input(UiInputEvent::wheel(
            UiPoint::new(30.0, 30.0),
            UiPoint::new(0.0, 30.0),
        ));

        assert_eq!(input.scrolled, None);
        assert_eq!(doc.scroll_state(back_scroll).unwrap().offset.y, 0.0);
    }

    #[test]
    fn wheel_scroll_searches_descendants_of_top_hit_before_lower_layers() {
        let mut doc = UiDocument::new(root_style(180.0, 150.0));
        let back_scroll = doc.add_child(
            doc.root,
            UiNode::container(
                "back.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 100.0, 60.0)).style,
                    clip: ClipBehavior::Clip,
                    z_index: 1,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            back_scroll,
            UiNode::container("back.content", button_style(100.0, 140.0)),
        );
        let front = doc.add_child(
            doc.root,
            UiNode::container(
                "front.panel",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(10.0, 10.0, 140.0, 110.0)).style,
                    clip: ClipBehavior::Clip,
                    z_index: 10,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior {
                pointer: true,
                focusable: false,
                keyboard: false,
            }),
        );
        let front_scroll = doc.add_child(
            front,
            UiNode::container(
                "front.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(10.0, 10.0, 90.0, 50.0)).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            front_scroll,
            UiNode::container("front.content", button_style(90.0, 130.0)),
        );
        doc.compute_layout(UiSize::new(180.0, 150.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let input = doc.handle_input(UiInputEvent::wheel(
            UiPoint::new(35.0, 35.0),
            UiPoint::new(0.0, 30.0),
        ));

        assert_eq!(input.scrolled, Some(front_scroll));
        assert_eq!(doc.scroll_state(front_scroll).unwrap().offset.y, 30.0);
        assert_eq!(doc.scroll_state(back_scroll).unwrap().offset.y, 0.0);
    }

    #[test]
    fn wheel_scroll_only_mutates_offsets_for_motion_phases() {
        let mut doc = UiDocument::new(root_style(120.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(60.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            scroll_area,
            UiNode::container("content", button_style(100.0, 140.0)),
        );
        doc.compute_layout(UiSize::new(120.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let started = doc.handle_input(UiInputEvent::Wheel(
            UiWheelEvent::pixels(UiPoint::new(20.0, 20.0), UiPoint::new(0.0, 30.0))
                .phase(WheelPhase::Started),
        ));
        assert_eq!(started.scrolled, None);
        assert_eq!(doc.scroll_state(scroll_area).unwrap().offset.y, 0.0);

        let ended = doc.handle_input(UiInputEvent::Wheel(
            UiWheelEvent::pixels(UiPoint::new(20.0, 20.0), UiPoint::new(0.0, 30.0))
                .phase(WheelPhase::Ended),
        ));
        assert_eq!(ended.scrolled, None);
        assert_eq!(doc.scroll_state(scroll_area).unwrap().offset.y, 0.0);

        let momentum = doc.handle_input(UiInputEvent::Wheel(
            UiWheelEvent::pixels(UiPoint::new(20.0, 20.0), UiPoint::new(0.0, 30.0))
                .phase(WheelPhase::Momentum),
        ));
        assert_eq!(momentum.scrolled, Some(scroll_area));
        assert_eq!(doc.scroll_state(scroll_area).unwrap().offset.y, 30.0);
    }

    #[test]
    fn wheel_scroll_targets_animation_transform_rect() {
        let animation = AnimationMachine::new(
            vec![AnimationState::new(
                "shown",
                AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 0.5),
            )],
            Vec::new(),
            "shown",
        )
        .expect("animation");
        let mut doc = UiDocument::new(root_style(160.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(60.0),
                            height: length(50.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL)
            .with_animation(animation),
        );
        doc.add_child(
            scroll_area,
            UiNode::container("content", button_style(60.0, 120.0)),
        );
        doc.compute_layout(UiSize::new(160.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let stale_layout_input = doc.handle_input(UiInputEvent::wheel(
            UiPoint::new(45.0, 20.0),
            UiPoint::new(0.0, 30.0),
        ));
        assert_eq!(stale_layout_input.scrolled, None);
        assert_eq!(doc.scroll_state(scroll_area).unwrap().offset.y, 0.0);

        let input = doc.handle_input(UiInputEvent::wheel(
            UiPoint::new(20.0, 20.0),
            UiPoint::new(0.0, 30.0),
        ));
        assert_eq!(input.scrolled, Some(scroll_area));
        assert_eq!(doc.scroll_state(scroll_area).unwrap().offset.y, 30.0);
    }

    #[test]
    fn scroll_rect_into_view_scrolls_explicit_rects_by_enabled_axes() {
        let mut doc = UiDocument::new(root_style(200.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(80.0),
                            height: length(50.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::BOTH),
        );
        let content = doc.add_child(
            scroll_area,
            UiNode::container("content", button_style(240.0, 180.0)),
        );
        doc.add_child(
            content,
            UiNode::container(
                "content_extent",
                UiNodeStyle {
                    layout: layout::absolute(230.0, 170.0, 10.0, 10.0).style,
                    ..Default::default()
                },
            ),
        );
        doc.compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert!(doc.scroll_rect_into_view(scroll_area, UiRect::new(140.0, 90.0, 12.0, 14.0)));
        assert_eq!(
            doc.scroll_state(scroll_area).unwrap().offset,
            UiPoint::new(72.0, 54.0)
        );

        assert!(doc.set_scroll_offset(scroll_area, UiPoint::new(500.0, 500.0)));
        assert_eq!(
            doc.scroll_state(scroll_area).unwrap().offset,
            UiPoint::new(160.0, 130.0)
        );
    }

    #[test]
    fn scroll_to_node_scrolls_nested_targets_into_view() {
        let mut doc = UiDocument::new(root_style(200.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(80.0),
                            height: length(50.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::BOTH),
        );
        let content = doc.add_child(
            scroll_area,
            UiNode::container("content", button_style(240.0, 180.0)),
        );
        let target = doc.add_child(
            content,
            UiNode::container(
                "target",
                UiNodeStyle {
                    layout: layout::absolute(160.0, 130.0, 20.0, 20.0).style,
                    ..Default::default()
                },
            ),
        );
        doc.compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert!(doc.scroll_to_node(scroll_area, target));
        assert_eq!(
            doc.scroll_state(scroll_area).unwrap().offset,
            UiPoint::new(100.0, 100.0)
        );

        doc.compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let target_rect = doc.node(target).layout.rect;
        let viewport = doc.node(scroll_area).layout.rect;
        assert!(viewport.contains_point(UiPoint::new(target_rect.x, target_rect.y)));
        assert!(viewport.contains_point(UiPoint::new(target_rect.right(), target_rect.bottom())));
        assert!(!doc.scroll_to_node(scroll_area, target));
    }

    #[test]
    fn audit_layout_reports_hidden_scroll_range_on_disabled_axis() {
        let mut doc = UiDocument::new(root_style(200.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "vertical.scroll",
                UiNodeStyle {
                    layout: LayoutStyle::size(100.0, 60.0).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            scroll_area,
            UiNode::container(
                "wide.content",
                LayoutStyle::size(180.0, 120.0).with_flex_shrink(0.0),
            ),
        );
        doc.compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let warnings = doc.audit_layout();
        assert!(warnings.iter().any(|warning| matches!(
            warning,
            AuditWarning::ScrollRangeHidden {
                node,
                name,
                axis: AuditAxis::Horizontal,
                viewport,
                content,
            } if *node == scroll_area
                && name == "vertical.scroll"
                && (*viewport - 100.0).abs() < 0.01
                && (*content - 180.0).abs() < 0.01
        )));
        assert!(!warnings.iter().any(|warning| matches!(
            warning,
            AuditWarning::ScrollRangeHidden {
                axis: AuditAxis::Vertical,
                ..
            }
        )));
    }

    #[test]
    fn audit_layout_reports_scroll_offsets_outside_reachable_range() {
        let mut doc = UiDocument::new(root_style(200.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::size(100.0, 60.0).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.add_child(
            scroll_area,
            UiNode::container(
                "tall.content",
                LayoutStyle::size(100.0, 140.0).with_flex_shrink(0.0),
            ),
        );
        doc.compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");
        doc.node_mut(scroll_area).scroll.as_mut().unwrap().offset = UiPoint::new(12.0, 120.0);

        let warnings = doc.audit_layout();
        assert!(warnings.iter().any(|warning| matches!(
            warning,
            AuditWarning::ScrollOffsetOutOfRange {
                node,
                name,
                axis: AuditAxis::Horizontal,
                offset,
                max_offset,
            } if *node == scroll_area
                && name == "scroll"
                && (*offset - 12.0).abs() < 0.01
                && max_offset.abs() < 0.01
        )));
        assert!(warnings.iter().any(|warning| matches!(
            warning,
            AuditWarning::ScrollOffsetOutOfRange {
                node,
                name,
                axis: AuditAxis::Vertical,
                offset,
                max_offset,
            } if *node == scroll_area
                && name == "scroll"
                && (*offset - 120.0).abs() < 0.01
                && (*max_offset - 80.0).abs() < 0.01
        )));
    }

    #[test]
    fn audit_layout_reports_visible_scrollbar_without_scroll_range() {
        let scroll = ScrollState {
            axes: ScrollAxes::VERTICAL,
            offset: UiPoint::new(0.0, 0.0),
            viewport_size: UiSize::new(12.0, 120.0),
            content_size: UiSize::new(12.0, 120.0),
        };
        let mut doc = UiDocument::new(root_style(200.0, 160.0));
        let root = doc.root;
        let scrollbar = doc.add_child(
            root,
            UiNode::container("manual.scrollbar", LayoutStyle::size(12.0, 120.0))
                .with_scrollbar_audit(AuditAxis::Vertical, scroll)
                .with_input(InputBehavior::BUTTON),
        );
        doc.compute_layout(UiSize::new(200.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let warnings = doc.audit_layout();
        assert!(warnings.iter().any(|warning| matches!(
            warning,
            AuditWarning::ScrollbarVisibleWithoutRange {
                node,
                name,
                axis: AuditAxis::Vertical,
                viewport,
                content,
            } if *node == scrollbar
                && name == "manual.scrollbar"
                && (*viewport - 120.0).abs() < 0.01
                && (*content - 120.0).abs() < 0.01
        )));
        let summary = warnings
            .iter()
            .find_map(|warning| {
                matches!(warning, AuditWarning::ScrollbarVisibleWithoutRange { .. })
                    .then(|| warning.diagnostic_summary())
            })
            .expect("scrollbar warning summary");
        assert!(summary.contains("reason: scrollbar is visible"));
        assert!(summary.contains("hint: hide and disable"));
    }

    #[test]
    fn compute_layout_clamps_stale_scroll_offsets_to_current_range() {
        let mut doc = UiDocument::new(root_style(200.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::size(100.0, 60.0).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        doc.node_mut(scroll_area).scroll.as_mut().unwrap().offset = UiPoint::new(0.0, 40.0);
        let content = doc.add_child(
            scroll_area,
            UiNode::container(
                "content",
                LayoutStyle::size(100.0, 40.0).with_flex_shrink(0.0),
            ),
        );

        doc.compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(
            doc.scroll_state(scroll_area).unwrap().offset,
            UiPoint::new(0.0, 0.0)
        );
        assert_eq!(
            doc.node(content).layout.rect.y,
            doc.node(scroll_area).layout.rect.y
        );
        assert!(!doc.audit_layout().iter().any(|warning| matches!(
            warning,
            AuditWarning::ScrollOffsetOutOfRange { node, .. } if *node == scroll_area
        )));
    }

    #[test]
    fn audit_layout_reports_focusable_nodes_missing_accessibility_traversal() {
        let mut doc = UiDocument::new(root_style(200.0, 80.0));
        let missing = doc.add_child(
            doc.root,
            UiNode::container("missing_semantics", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON),
        );
        let hidden = doc.add_child(
            doc.root,
            UiNode::container("hidden_semantics", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Hidden")
                        .hidden(),
                ),
        );
        let accessible = doc.add_child(
            doc.root,
            UiNode::container("accessible", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Accessible")
                        .focusable(),
                ),
        );
        doc.compute_layout(UiSize::new(200.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let warnings = doc.audit_layout();
        assert!(
            warnings.contains(&AuditWarning::FocusableMissingFromAccessibilityTree {
                node: missing,
                name: "missing_semantics".to_string(),
            })
        );
        assert!(
            warnings.contains(&AuditWarning::InteractiveAccessibilityMissing {
                node: missing,
                name: "missing_semantics".to_string(),
            })
        );
        assert!(
            warnings.contains(&AuditWarning::FocusableMissingFromAccessibilityTree {
                node: hidden,
                name: "hidden_semantics".to_string(),
            })
        );
        assert!(
            warnings.contains(&AuditWarning::InteractiveAccessibilityMissing {
                node: hidden,
                name: "hidden_semantics".to_string(),
            })
        );
        assert!(
            !warnings.contains(&AuditWarning::FocusableMissingFromAccessibilityTree {
                node: accessible,
                name: "accessible".to_string(),
            })
        );
        assert!(
            !warnings.contains(&AuditWarning::InteractiveAccessibilityMissing {
                node: accessible,
                name: "accessible".to_string(),
            })
        );
    }

    #[test]
    fn audit_layout_reports_low_text_contrast_against_effective_background() {
        let mut doc = UiDocument::new(root_style(240.0, 80.0));
        doc.node_mut(doc.root).visual = UiVisual::panel(ColorRgba::new(18, 22, 28, 255), None, 0.0);
        let panel = doc.add_child(
            doc.root,
            UiNode::container("panel", button_style(220.0, 64.0)).with_visual(UiVisual::panel(
                ColorRgba::new(30, 36, 44, 255),
                None,
                0.0,
            )),
        );
        let low = doc.add_child(
            panel,
            UiNode::text(
                "low_contrast",
                "Low contrast",
                TextStyle {
                    font_size: 12.0,
                    line_height: 16.0,
                    color: ColorRgba::new(42, 48, 56, 255),
                    ..Default::default()
                },
                button_style(120.0, 20.0).layout,
            ),
        );
        let good = doc.add_child(
            panel,
            UiNode::text(
                "good_contrast",
                "Readable",
                TextStyle {
                    font_size: 12.0,
                    line_height: 16.0,
                    color: ColorRgba::new(230, 238, 248, 255),
                    ..Default::default()
                },
                button_style(120.0, 20.0).layout,
            ),
        );
        doc.compute_layout(UiSize::new(240.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let warnings = doc.audit_layout();
        assert!(warnings.iter().any(|warning| {
            matches!(
                warning,
                AuditWarning::TextContrastTooLow {
                    node,
                    name,
                    background_color,
                    contrast_ratio,
                    required_ratio,
                    ..
                } if *node == low
                    && name == "low_contrast"
                    && *background_color == ColorRgba::new(30, 36, 44, 255)
                    && *contrast_ratio < *required_ratio
            )
        }));
        assert!(!warnings.iter().any(|warning| {
            matches!(
                warning,
                AuditWarning::TextContrastTooLow { node, .. } if *node == good
            )
        }));
    }

    #[test]
    fn audit_layout_reports_low_scene_text_contrast() {
        let mut doc = UiDocument::new(root_style(240.0, 80.0));
        doc.node_mut(doc.root).visual = UiVisual::panel(ColorRgba::new(18, 22, 28, 255), None, 0.0);
        let scene = doc.add_child(
            doc.root,
            UiNode::scene(
                "scene_labels",
                vec![ScenePrimitive::Text(PaintText::new(
                    "Scene label",
                    UiRect::new(8.0, 8.0, 120.0, 20.0),
                    TextStyle {
                        font_size: 12.0,
                        line_height: 16.0,
                        color: ColorRgba::new(34, 40, 48, 255),
                        ..Default::default()
                    },
                ))],
                layout::fixed(160.0, 40.0),
            ),
        );
        doc.compute_layout(UiSize::new(240.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let warnings = doc.audit_layout();
        assert!(warnings.iter().any(|warning| {
            matches!(
                warning,
                AuditWarning::TextContrastTooLow {
                    node,
                    name,
                    contrast_ratio,
                    required_ratio,
                    ..
                } if *node == scene
                    && name == "scene_labels"
                    && *contrast_ratio < *required_ratio
            )
        }));
    }

    #[test]
    fn audit_layout_reports_accessibility_name_action_and_relation_gaps() {
        let mut doc = UiDocument::new(root_style(260.0, 120.0));
        let unlabeled = doc.add_child(
            doc.root,
            UiNode::container("unlabeled", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Button).focusable()),
        );
        let hidden_label = doc.add_child(
            doc.root,
            UiNode::text(
                "hidden_label",
                "Hidden label",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(80.0),
                        height: length(20.0),
                    },
                    ..Default::default()
                }),
            ),
        );
        let related_label = doc.add_child(
            doc.root,
            UiNode::text(
                "related_label",
                "Related label",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(100.0),
                        height: length(20.0),
                    },
                    ..Default::default()
                }),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Label).label("Related label"),
            ),
        );
        let named_by_relation = doc.add_child(
            doc.root,
            UiNode::container("named_by_relation", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .labelled_by(related_label)
                        .action(AccessibilityAction::new("activate", "Activate"))
                        .focusable(),
                ),
        );
        let relation_gap = doc.add_child(
            doc.root,
            UiNode::container("relation_gap", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .labelled_by(hidden_label)
                        .action(AccessibilityAction::new("activate", "Activate"))
                        .focusable(),
                ),
        );
        let action_id_gap = doc.add_child(
            doc.root,
            UiNode::container("action_id_gap", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Blank action id")
                        .action(AccessibilityAction::new(" ", "Activate"))
                        .focusable(),
                ),
        );
        let action_label_gap = doc.add_child(
            doc.root,
            UiNode::container("action_label_gap", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Blank action label")
                        .action(AccessibilityAction::new("activate", " "))
                        .focusable(),
                ),
        );
        let action_duplicate_gap = doc.add_child(
            doc.root,
            UiNode::container("action_duplicate_gap", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Duplicate action")
                        .action(AccessibilityAction::new("activate", "Activate"))
                        .action(AccessibilityAction::new("activate", "Activate again"))
                        .focusable(),
                ),
        );
        let checked_state_gap = doc.add_child(
            doc.root,
            UiNode::container("checked_state_gap", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Checkbox)
                        .label("Missing checked")
                        .action(AccessibilityAction::new("toggle", "Toggle"))
                        .focusable(),
                ),
        );
        let expanded_state_gap = doc.add_child(
            doc.root,
            UiNode::container("expanded_state_gap", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::ComboBox)
                        .label("Missing expanded")
                        .action(AccessibilityAction::new("open", "Open"))
                        .focusable(),
                ),
        );
        let pressed_state_gap = doc.add_child(
            doc.root,
            UiNode::container("pressed_state_gap", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::ToggleButton)
                        .label("Missing pressed")
                        .action(AccessibilityAction::new("toggle", "Toggle"))
                        .focusable(),
                ),
        );
        let selected_state_gap = doc.add_child(
            doc.root,
            UiNode::container("selected_state_gap", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Tab)
                        .label("Missing selected")
                        .action(AccessibilityAction::new("select", "Select"))
                        .focusable(),
                ),
        );
        let value_gap = doc.add_child(
            doc.root,
            UiNode::container("value_gap", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Slider)
                        .label("Missing value")
                        .action(AccessibilityAction::new("increase", "Increase"))
                        .action(AccessibilityAction::new("decrease", "Decrease"))
                        .focusable(),
                ),
        );
        let range_gap = doc.add_child(
            doc.root,
            UiNode::container("range_gap", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Slider)
                        .label("Missing range")
                        .value("50%")
                        .action(AccessibilityAction::new("increase", "Increase"))
                        .action(AccessibilityAction::new("decrease", "Decrease"))
                        .focusable(),
                ),
        );
        let invalid_range = doc.add_child(
            doc.root,
            UiNode::container("invalid_range", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Slider)
                        .label("Invalid range")
                        .value("50%")
                        .value_range(AccessibilityValueRange::new(100.0, 0.0))
                        .action(AccessibilityAction::new("increase", "Increase"))
                        .action(AccessibilityAction::new("decrease", "Decrease"))
                        .focusable(),
                ),
        );
        let meter_range_gap = doc.add_child(
            doc.root,
            UiNode::container("meter_range_gap", button_style(80.0, 24.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Meter)
                    .label("Missing meter range")
                    .value("72%"),
            ),
        );
        let complete = doc.add_child(
            doc.root,
            UiNode::container("complete", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Complete")
                        .action(AccessibilityAction::new("activate", "Activate"))
                        .focusable(),
                ),
        );
        let complete_checkbox = doc.add_child(
            doc.root,
            UiNode::container("complete_checkbox", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Checkbox)
                        .label("Complete checkbox")
                        .checked(false)
                        .action(AccessibilityAction::new("toggle", "Toggle"))
                        .focusable(),
                ),
        );
        let complete_slider = doc.add_child(
            doc.root,
            UiNode::container("complete_slider", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Slider)
                        .label("Complete slider")
                        .value("75%")
                        .value_range(AccessibilityValueRange::new(0.0, 100.0))
                        .action(AccessibilityAction::new("increase", "Increase"))
                        .action(AccessibilityAction::new("decrease", "Decrease"))
                        .focusable(),
                ),
        );
        let complete_meter = doc.add_child(
            doc.root,
            UiNode::container("complete_meter", button_style(80.0, 24.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Meter)
                    .label("Complete meter")
                    .value("20%")
                    .value_range(AccessibilityValueRange::new(0.0, 100.0)),
            ),
        );
        doc.compute_layout(UiSize::new(260.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let warnings = doc.audit_layout();
        assert!(warnings.contains(&AuditWarning::AccessibleNameMissing {
            node: unlabeled,
            name: "unlabeled".to_string(),
            role: AccessibilityRole::Button,
        }));
        assert!(
            warnings.contains(&AuditWarning::AccessibilityActionMissing {
                node: unlabeled,
                name: "unlabeled".to_string(),
                role: AccessibilityRole::Button,
            })
        );
        assert!(warnings.contains(&AuditWarning::AccessibleNameMissing {
            node: relation_gap,
            name: "relation_gap".to_string(),
            role: AccessibilityRole::Button,
        }));
        assert!(
            warnings.contains(&AuditWarning::AccessibilityRelationTargetMissing {
                node: relation_gap,
                name: "relation_gap".to_string(),
                relation: AccessibilityRelationKind::LabelledBy,
                target: hidden_label,
            })
        );
        assert!(
            warnings.contains(&AuditWarning::AccessibilityActionIdMissing {
                node: action_id_gap,
                name: "action_id_gap".to_string(),
            })
        );
        assert!(
            warnings.contains(&AuditWarning::AccessibilityActionLabelMissing {
                node: action_label_gap,
                name: "action_label_gap".to_string(),
                action_id: "activate".to_string(),
            })
        );
        assert!(
            warnings.contains(&AuditWarning::AccessibilityActionDuplicate {
                node: action_duplicate_gap,
                name: "action_duplicate_gap".to_string(),
                action_id: "activate".to_string(),
            })
        );
        assert!(warnings.contains(&AuditWarning::AccessibilityStateMissing {
            node: checked_state_gap,
            name: "checked_state_gap".to_string(),
            role: AccessibilityRole::Checkbox,
            state: AccessibilityStateKind::Checked,
        }));
        assert!(warnings.contains(&AuditWarning::AccessibilityStateMissing {
            node: expanded_state_gap,
            name: "expanded_state_gap".to_string(),
            role: AccessibilityRole::ComboBox,
            state: AccessibilityStateKind::Expanded,
        }));
        assert!(warnings.contains(&AuditWarning::AccessibilityStateMissing {
            node: pressed_state_gap,
            name: "pressed_state_gap".to_string(),
            role: AccessibilityRole::ToggleButton,
            state: AccessibilityStateKind::Pressed,
        }));
        assert!(warnings.contains(&AuditWarning::AccessibilityStateMissing {
            node: selected_state_gap,
            name: "selected_state_gap".to_string(),
            role: AccessibilityRole::Tab,
            state: AccessibilityStateKind::Selected,
        }));
        assert!(warnings.contains(&AuditWarning::AccessibilityValueMissing {
            node: value_gap,
            name: "value_gap".to_string(),
            role: AccessibilityRole::Slider,
        }));
        assert!(
            warnings.contains(&AuditWarning::AccessibilityValueRangeMissing {
                node: value_gap,
                name: "value_gap".to_string(),
                role: AccessibilityRole::Slider,
            })
        );
        assert!(
            warnings.contains(&AuditWarning::AccessibilityValueRangeMissing {
                node: range_gap,
                name: "range_gap".to_string(),
                role: AccessibilityRole::Slider,
            })
        );
        assert!(
            warnings.contains(&AuditWarning::AccessibilityValueRangeInvalid {
                node: invalid_range,
                name: "invalid_range".to_string(),
                role: AccessibilityRole::Slider,
                issue: AccessibilityValueRangeIssue::Reversed,
                range: AccessibilityValueRange::new(100.0, 0.0),
            })
        );
        assert!(
            warnings.contains(&AuditWarning::AccessibilityValueRangeMissing {
                node: meter_range_gap,
                name: "meter_range_gap".to_string(),
                role: AccessibilityRole::Meter,
            })
        );
        assert!(!warnings.contains(&AuditWarning::AccessibleNameMissing {
            node: named_by_relation,
            name: "named_by_relation".to_string(),
            role: AccessibilityRole::Button,
        }));
        assert!(!warnings.contains(&AuditWarning::AccessibleNameMissing {
            node: complete,
            name: "complete".to_string(),
            role: AccessibilityRole::Button,
        }));
        assert!(
            !warnings.contains(&AuditWarning::AccessibilityActionMissing {
                node: complete,
                name: "complete".to_string(),
                role: AccessibilityRole::Button,
            })
        );
        assert!(
            !warnings.contains(&AuditWarning::AccessibilityActionIdMissing {
                node: complete,
                name: "complete".to_string(),
            })
        );
        assert!(
            !warnings.contains(&AuditWarning::AccessibilityActionLabelMissing {
                node: complete,
                name: "complete".to_string(),
                action_id: "activate".to_string(),
            })
        );
        assert!(
            !warnings.contains(&AuditWarning::AccessibilityActionDuplicate {
                node: complete,
                name: "complete".to_string(),
                action_id: "activate".to_string(),
            })
        );
        assert!(
            !warnings.contains(&AuditWarning::AccessibilityStateMissing {
                node: complete_checkbox,
                name: "complete_checkbox".to_string(),
                role: AccessibilityRole::Checkbox,
                state: AccessibilityStateKind::Checked,
            })
        );
        assert!(
            !warnings.contains(&AuditWarning::AccessibilityValueMissing {
                node: complete_slider,
                name: "complete_slider".to_string(),
                role: AccessibilityRole::Slider,
            })
        );
        assert!(
            !warnings.contains(&AuditWarning::AccessibilityValueRangeMissing {
                node: complete_slider,
                name: "complete_slider".to_string(),
                role: AccessibilityRole::Slider,
            })
        );
        assert!(
            !warnings.contains(&AuditWarning::AccessibilityValueRangeMissing {
                node: complete_meter,
                name: "complete_meter".to_string(),
                role: AccessibilityRole::Meter,
            })
        );
        assert!(
            !warnings.contains(&AuditWarning::AccessibilityValueRangeInvalid {
                node: complete_slider,
                name: "complete_slider".to_string(),
                role: AccessibilityRole::Slider,
                issue: AccessibilityValueRangeIssue::Reversed,
                range: AccessibilityValueRange::new(100.0, 0.0),
            })
        );
    }

    #[test]
    fn scroll_content_bounds_include_nested_descendants() {
        let mut doc = UiDocument::new(root_style(160.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(60.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        let wrapper = doc.add_child(
            scroll_area,
            UiNode::container("wrapper", button_style(100.0, 30.0)),
        );
        doc.add_child(
            wrapper,
            UiNode::container("nested_tall", button_style(100.0, 130.0)),
        );
        doc.compute_layout(UiSize::new(160.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(
            doc.scroll_state(scroll_area).unwrap().content_size.height,
            130.0
        );
    }

    #[test]
    fn paint_list_exposes_rect_text_and_canvas_items_without_a_backend() {
        let mut doc = UiDocument::new(root_style(240.0, 120.0));
        let panel = doc.add_child(
            doc.root,
            UiNode::container("panel", button_style(100.0, 50.0)).with_visual(UiVisual::panel(
                ColorRgba::new(10, 20, 30, 255),
                Some(StrokeStyle::new(ColorRgba::WHITE, 1.0)),
                4.0,
            )),
        );
        let _label = doc.add_child(
            panel,
            UiNode::text(
                "label",
                "Gain",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            ),
        );
        let _canvas = doc.add_child(
            doc.root,
            UiNode::canvas(
                "editor_surface",
                "app.editor_surface",
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(100.0),
                        height: length(50.0),
                    },
                    ..Default::default()
                }),
            ),
        );
        doc.compute_layout(UiSize::new(240.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let paint = doc.paint_list();
        assert!(paint
            .items
            .iter()
            .any(|item| matches!(item.kind, PaintKind::Rect { .. })));
        assert!(paint
            .items
            .iter()
            .any(|item| matches!(item.kind, PaintKind::Text(_))));
        assert!(paint
            .items
            .iter()
            .any(|item| matches!(item.kind, PaintKind::Canvas(_))));
    }

    #[test]
    fn paint_list_exposes_scene_primitives() {
        let mut doc = UiDocument::new(root_style(120.0, 80.0));
        doc.add_child(
            doc.root,
            UiNode::scene(
                "scene",
                vec![
                    ScenePrimitive::Line {
                        from: UiPoint::new(0.0, 0.0),
                        to: UiPoint::new(20.0, 20.0),
                        stroke: StrokeStyle::new(ColorRgba::WHITE, 1.0),
                    },
                    ScenePrimitive::Circle {
                        center: UiPoint::new(30.0, 20.0),
                        radius: 8.0,
                        fill: ColorRgba::new(20, 120, 220, 255),
                        stroke: None,
                    },
                ],
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(80.0),
                        height: length(60.0),
                    },
                    ..Default::default()
                }),
            ),
        );
        doc.compute_layout(UiSize::new(120.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let paint = doc.paint_list();
        assert!(paint
            .items
            .iter()
            .any(|item| matches!(item.kind, PaintKind::Line { .. })));
        assert!(paint
            .items
            .iter()
            .any(|item| matches!(item.kind, PaintKind::Circle { .. })));
    }

    #[test]
    fn paint_list_exposes_rich_scene_display_list_primitives() {
        let mut doc = UiDocument::new(root_style(140.0, 100.0));
        doc.add_child(
            doc.root,
            UiNode::scene(
                "scene.rich",
                vec![
                    ScenePrimitive::Rect(
                        PaintRect::new(
                            UiRect::new(4.0, 6.0, 72.0, 26.0),
                            PaintBrush::LinearGradient(
                                LinearGradient::new(
                                    UiPoint::new(4.0, 6.0),
                                    UiPoint::new(76.0, 32.0),
                                    ColorRgba::new(18, 30, 44, 255),
                                    ColorRgba::new(42, 74, 105, 255),
                                )
                                .stop(0.45, ColorRgba::new(25, 48, 72, 255))
                                .fallback(ColorRgba::new(20, 36, 54, 255)),
                            ),
                        )
                        .stroke(AlignedStroke::inside(StrokeStyle::new(
                            ColorRgba::new(120, 190, 255, 255),
                            1.5,
                        )))
                        .corner_radii(CornerRadii::new(4.0, 8.0, 8.0, 4.0))
                        .effect(PaintEffect::shadow(
                            ColorRgba::new(0, 0, 0, 90),
                            UiPoint::new(0.0, 3.0),
                            10.0,
                            1.0,
                        )),
                    ),
                    ScenePrimitive::Text(
                        PaintText::new(
                            "Peak Level",
                            UiRect::new(8.0, 10.0, 64.0, 16.0),
                            TextStyle {
                                color: ColorRgba::new(230, 240, 250, 255),
                                ..Default::default()
                            },
                        )
                        .horizontal_align(TextHorizontalAlign::Center)
                        .vertical_align(TextVerticalAlign::Baseline)
                        .overflow(TextOverflow::Ellipsis)
                        .multiline(false),
                    ),
                    ScenePrimitive::Path(
                        PaintPath::new()
                            .move_to(UiPoint::new(8.0, 58.0))
                            .cubic_to(
                                UiPoint::new(30.0, 20.0),
                                UiPoint::new(62.0, 84.0),
                                UiPoint::new(96.0, 46.0),
                            )
                            .stroke(AlignedStroke::center(StrokeStyle::new(
                                ColorRgba::new(242, 205, 96, 255),
                                2.0,
                            ))),
                    ),
                    ScenePrimitive::ImagePlacement(
                        PaintImage::new("meters.peak", UiRect::new(82.0, 8.0, 24.0, 24.0))
                            .tinted(ColorRgba::new(120, 210, 255, 255))
                            .fit(ImageFit::Contain)
                            .align(ImageAlignment::End, ImageAlignment::Start),
                    ),
                ],
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(120.0),
                        height: length(90.0),
                    },
                    ..Default::default()
                }),
            ),
        );
        doc.compute_layout(UiSize::new(140.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let paint = doc.paint_list();
        let rich_rect = paint
            .items
            .iter()
            .find_map(|item| match &item.kind {
                PaintKind::RichRect(rect) => Some(rect),
                _ => None,
            })
            .expect("rich rect paint item");
        assert_eq!(rich_rect.rect, UiRect::new(4.0, 6.0, 72.0, 26.0));
        assert_eq!(
            rich_rect.stroke.expect("stroke").alignment,
            StrokeAlignment::Inside
        );
        assert_eq!(rich_rect.corner_radii.top_right, 8.0);
        assert_eq!(rich_rect.effects[0].kind, PaintEffectKind::Shadow);
        assert!(matches!(
            rich_rect.fill,
            PaintBrush::LinearGradient(ref gradient)
                if gradient.stops.len() == 3
                    && gradient.fallback == ColorRgba::new(20, 36, 54, 255)
        ));

        let text = paint
            .items
            .iter()
            .find_map(|item| match &item.kind {
                PaintKind::SceneText(text) => Some(text),
                _ => None,
            })
            .expect("scene text paint item");
        assert_eq!(text.horizontal_align, TextHorizontalAlign::Center);
        assert_eq!(text.vertical_align, TextVerticalAlign::Baseline);
        assert_eq!(text.overflow, TextOverflow::Ellipsis);
        assert!(!text.multiline);

        let path = paint
            .items
            .iter()
            .find_map(|item| match &item.kind {
                PaintKind::Path(path) => Some(path),
                _ => None,
            })
            .expect("path paint item");
        assert_eq!(path.verbs.len(), 2);
        assert_eq!(path.bounds(), UiRect::new(8.0, 20.0, 88.0, 64.0));

        let image = paint
            .items
            .iter()
            .find_map(|item| match &item.kind {
                PaintKind::ImagePlacement(image) => Some(image),
                _ => None,
            })
            .expect("image placement paint item");
        assert_eq!(image.key, "meters.peak");
        assert_eq!(image.fit, ImageFit::Contain);
        assert_eq!(image.horizontal_align, ImageAlignment::End);
    }

    #[test]
    fn paint_list_exposes_image_and_shader_metadata() {
        let mut doc = UiDocument::new(root_style(120.0, 80.0));
        let image = doc.add_child(
            doc.root,
            UiNode::image(
                "icon",
                ImageContent::new("icons.play").tinted(ColorRgba::new(120, 180, 255, 255)),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(24.0),
                        height: length(24.0),
                    },
                    ..Default::default()
                }),
            )
            .with_shader(ShaderEffect::new("ui.glow").uniform("intensity", 0.5)),
        );
        doc.compute_layout(UiSize::new(120.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let item = doc
            .paint_list()
            .items
            .into_iter()
            .find(|item| item.node == image)
            .expect("image paint item");
        assert!(matches!(
            item.kind,
            PaintKind::Image {
                ref key,
                tint: Some(_)
            } if key == "icons.play"
        ));
        assert_eq!(item.shader.unwrap().key, "ui.glow");
    }

    #[cfg(feature = "egui-renderer-compat")]
    #[test]
    fn egui_paint_callbacks_receive_image_and_canvas_items() {
        let mut doc = UiDocument::new(root_style(160.0, 120.0));
        doc.add_child(
            doc.root,
            UiNode::image(
                "icon",
                ImageContent::new("icons.play").tinted(ColorRgba::new(120, 180, 255, 255)),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(24.0),
                        height: length(24.0),
                    },
                    ..Default::default()
                }),
            ),
        );
        doc.add_child(
            doc.root,
            UiNode::scene(
                "preview",
                vec![ScenePrimitive::ImagePlacement(
                    PaintImage::new("thumbs.lot", UiRect::new(4.0, 6.0, 32.0, 20.0))
                        .fit(ImageFit::Contain),
                )],
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(48.0),
                        height: length(32.0),
                    },
                    ..Default::default()
                }),
            ),
        );
        doc.add_child(
            doc.root,
            UiNode::canvas(
                "mask",
                "editor.mask.viewport",
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(80.0),
                        height: length(48.0),
                    },
                    ..Default::default()
                }),
            ),
        );
        doc.compute_layout(UiSize::new(160.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let ctx = egui::Context::default();
        let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("operad-test"));
        let mut image_keys = Vec::new();
        let mut canvas_keys = Vec::new();

        paint_document_egui_with_callbacks(
            &doc,
            &ctx,
            layer,
            |image, _item, _painter| image_keys.push(image.key.clone()),
            |canvas, _item, _painter| canvas_keys.push(canvas.key.clone()),
        );

        assert_eq!(image_keys, vec!["icons.play", "thumbs.lot"]);
        assert_eq!(canvas_keys, vec!["editor.mask.viewport"]);
    }

    #[test]
    fn accessibility_tree_exports_explicit_node_metadata() {
        let mut doc = UiDocument::new(root_style(180.0, 80.0));
        let button = doc.add_child(
            doc.root,
            UiNode::container("play", button_style(80.0, 32.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Play")
                        .hint("Starts transport")
                        .focusable(),
                ),
        );
        doc.compute_layout(UiSize::new(180.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let tree = doc.accessibility_tree();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, button);
        assert_eq!(tree[0].role, AccessibilityRole::Button);
        assert_eq!(tree[0].label.as_deref(), Some("Play"));
        assert!(tree[0].focusable);
        assert_eq!(tree[0].rect.width, 80.0);
    }

    #[test]
    fn accessibility_snapshot_tracks_focus_order_state_relations_and_actions() {
        let mut doc = UiDocument::new(root_style(240.0, 140.0));
        let name = doc.add_child(
            doc.root,
            UiNode::text(
                "play.name",
                "Play",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            )
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label("Play")),
        );
        let hint = doc.add_child(
            doc.root,
            UiNode::text(
                "play.hint",
                "Starts transport",
                TextStyle::default(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Tooltip).label("Starts transport"),
            ),
        );
        let dialog = doc.add_child(
            doc.root,
            UiNode::container("modal", button_style(60.0, 32.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Dialog)
                        .label("Command palette")
                        .modal()
                        .focusable()
                        .focus_order(0),
                ),
        );
        let slider = doc.add_child(
            doc.root,
            UiNode::container("volume", button_style(120.0, 20.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Slider)
                        .label("Volume")
                        .value("-6 dB")
                        .value_range(AccessibilityValueRange::new(-60.0, 6.0).with_step(0.5))
                        .focusable()
                        .focus_order(1),
                ),
        );
        let button = doc.add_child(
            doc.root,
            UiNode::container("play", button_style(80.0, 32.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::ToggleButton)
                        .label("Transport play")
                        .labelled_by(name)
                        .described_by(hint)
                        .controls(slider)
                        .pressed(true)
                        .selected(true)
                        .shortcut("Space")
                        .action(AccessibilityAction::new("activate", "Activate").shortcut("Space"))
                        .focusable()
                        .focus_order(2),
                ),
        );
        doc.add_child(
            doc.root,
            UiNode::container("hidden", button_style(40.0, 20.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label("Hidden")
                    .hidden()
                    .focusable(),
            ),
        );
        doc.compute_layout(UiSize::new(240.0, 140.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let snapshot = doc.accessibility_snapshot();
        assert_eq!(snapshot.modal_scope, Some(dialog));
        assert_eq!(snapshot.focus_order, vec![dialog, slider, button]);
        assert!(!snapshot
            .nodes
            .iter()
            .any(|node| node.label.as_deref() == Some("Hidden")));

        let button_node = snapshot
            .nodes
            .iter()
            .find(|node| node.id == button)
            .expect("button accessibility");
        assert_eq!(button_node.role, AccessibilityRole::ToggleButton);
        assert_eq!(button_node.pressed, Some(true));
        assert_eq!(button_node.selected, Some(true));
        assert_eq!(button_node.key_shortcuts, vec!["Space"]);
        assert_eq!(button_node.actions[0].id, "activate");
        assert_eq!(button_node.relations.labelled_by, vec![name]);
        assert_eq!(button_node.relations.described_by, vec![hint]);
        assert_eq!(button_node.relations.controls, vec![slider]);
        assert_eq!(snapshot.accessible_name(button).as_deref(), Some("Play"));
        assert_eq!(
            snapshot.accessible_description(button).as_deref(),
            Some("Starts transport")
        );

        let slider_node = snapshot
            .nodes
            .iter()
            .find(|node| node.id == slider)
            .expect("slider accessibility");
        assert_eq!(
            slider_node.value_range,
            Some(AccessibilityValueRange::new(-60.0, 6.0).with_step(0.5))
        );
    }

    #[test]
    fn pointer_and_keyboard_focus_are_tracked() {
        let mut doc = UiDocument::new(root_style(400.0, 200.0));
        let first = doc.add_child(
            doc.root,
            UiNode::container("first", button_style(80.0, 40.0)).with_input(InputBehavior::BUTTON),
        );
        let second = doc.add_child(
            doc.root,
            UiNode::container("second", button_style(80.0, 40.0)).with_input(InputBehavior::BUTTON),
        );
        doc.compute_layout(UiSize::new(400.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let down = doc.handle_input(UiInputEvent::PointerDown(UiPoint::new(20.0, 20.0)));
        assert_eq!(down.focused, Some(first));
        let up = doc.handle_input(UiInputEvent::PointerUp(UiPoint::new(20.0, 20.0)));
        assert_eq!(up.clicked, Some(first));

        let tab = doc.handle_input(UiInputEvent::Focus(FocusDirection::Next));
        assert_eq!(tab.focused, Some(second));
    }

    #[test]
    fn keyboard_focus_uses_accessibility_order_and_modal_scope() {
        let mut doc = UiDocument::new(root_style(360.0, 180.0));
        let outside = doc.add_child(
            doc.root,
            UiNode::container("outside", button_style(80.0, 36.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Outside")
                        .focusable()
                        .focus_order(-10),
                ),
        );
        let modal = doc.add_child(
            doc.root,
            UiNode::container("modal", button_style(220.0, 120.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Dialog)
                        .label("Command palette")
                        .modal()
                        .focusable()
                        .focus_order(10),
                ),
        );
        doc.add_child(
            modal,
            UiNode::container("modal.disabled", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Disabled")
                        .disabled()
                        .focusable()
                        .focus_order(-5),
                ),
        );
        let a11y_only = doc.add_child(
            modal,
            UiNode::container("modal.a11y", button_style(80.0, 24.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label("A11y only")
                    .focusable()
                    .focus_order(0),
            ),
        );
        let input = doc.add_child(
            modal,
            UiNode::container("modal.input", button_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Input")
                        .focusable()
                        .focus_order(1),
                ),
        );
        doc.compute_layout(UiSize::new(360.0, 180.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let snapshot = doc.accessibility_snapshot();
        assert_eq!(snapshot.focus_order, vec![outside, a11y_only, input, modal]);
        assert_eq!(
            snapshot.effective_focus_order(),
            vec![a11y_only, input, modal]
        );

        let first = doc.handle_input(UiInputEvent::Focus(FocusDirection::Next));
        assert_eq!(first.focused, Some(a11y_only));
        let second = doc.handle_input(UiInputEvent::Focus(FocusDirection::Next));
        assert_eq!(second.focused, Some(input));
        let third = doc.handle_input(UiInputEvent::Focus(FocusDirection::Next));
        assert_eq!(third.focused, Some(modal));
        let wrapped = doc.handle_input(UiInputEvent::Focus(FocusDirection::Next));
        assert_eq!(wrapped.focused, Some(a11y_only));
    }

    #[test]
    fn animation_machine_transitions_between_named_states() {
        let idle = AnimationState::new(
            "idle",
            AnimatedValues::new(0.5, UiPoint::new(0.0, 0.0), 1.0),
        );
        let focused = AnimationState::new(
            "focused",
            AnimatedValues::new(1.0, UiPoint::new(0.0, -4.0), 1.05),
        );
        let mut machine = AnimationMachine::new(
            vec![idle, focused],
            vec![AnimationTransition::new(
                "idle",
                "focused",
                AnimationTrigger::FocusGained,
                0.20,
            )],
            "idle",
        )
        .expect("animation machine");

        assert_eq!(machine.current_state_name(), "idle");
        assert!(machine.trigger(AnimationTrigger::FocusGained));
        machine.tick(0.10);
        assert!(machine.values().opacity > 0.5 && machine.values().opacity < 1.0);
        machine.tick(0.10);
        assert_eq!(machine.current_state_name(), "focused");
        assert_eq!(machine.values().scale, 1.05);
    }

    #[test]
    fn animation_machine_inputs_drive_state_transitions() {
        let mut machine = AnimationMachine::new(
            vec![
                AnimationState::new(
                    "closed",
                    AnimatedValues::new(0.0, UiPoint::new(0.0, 12.0), 0.96),
                ),
                AnimationState::new(
                    "open",
                    AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0),
                ),
            ],
            vec![
                AnimationTransition::when(
                    "closed",
                    "open",
                    AnimationCondition::bool("open", true),
                    0.10,
                ),
                AnimationTransition::when(
                    "open",
                    "closed",
                    AnimationCondition::bool("open", false),
                    0.10,
                ),
            ],
            "closed",
        )
        .expect("animation")
        .with_bool_input("open", false);

        assert!(machine.set_bool_input("open", true));
        assert_eq!(machine.current_state_name(), "open");
        assert!(machine.is_animating());
        machine.tick(0.10);
        assert_eq!(machine.values().opacity, 1.0);

        assert!(machine.set_bool_input("open", false));
        machine.tick(0.10);
        assert_eq!(machine.current_state_name(), "closed");
        assert_eq!(machine.values().opacity, 0.0);
    }

    #[test]
    fn animation_machine_number_inputs_blend_between_states() {
        let mut machine = AnimationMachine::new(
            vec![
                AnimationState::new(
                    "start",
                    AnimatedValues::new(0.35, UiPoint::new(0.0, 0.0), 0.85),
                ),
                AnimationState::new(
                    "end",
                    AnimatedValues::new(1.0, UiPoint::new(80.0, -16.0), 1.2).with_morph(1.0),
                ),
            ],
            Vec::new(),
            "start",
        )
        .expect("animation")
        .with_number_input("scrub", 0.25)
        .with_blend_binding(AnimationBlendBinding::new("scrub", "start", "end"));

        let values = machine.values();
        assert!((values.translate.x - 20.0).abs() <= f32::EPSILON);
        assert!((values.morph - 0.25).abs() <= f32::EPSILON);
        assert!(values.opacity > 0.35 && values.opacity < 1.0);

        assert!(machine.set_number_input("scrub", 0.75));
        let values = machine.values();
        assert!((values.translate.x - 60.0).abs() <= f32::EPSILON);
        assert!((values.morph - 0.75).abs() <= f32::EPSILON);
        assert!(!machine.is_animating());
    }

    #[test]
    fn morph_polygon_scene_primitive_blends_different_point_counts() {
        let mut doc = UiDocument::new(root_style(140.0, 100.0));
        doc.add_child(
            doc.root,
            UiNode::scene(
                "morph",
                vec![ScenePrimitive::MorphPolygon {
                    from_points: vec![
                        UiPoint::new(20.0, 20.0),
                        UiPoint::new(60.0, 20.0),
                        UiPoint::new(60.0, 60.0),
                        UiPoint::new(20.0, 60.0),
                    ],
                    to_points: vec![
                        UiPoint::new(40.0, 12.0),
                        UiPoint::new(72.0, 34.0),
                        UiPoint::new(60.0, 72.0),
                        UiPoint::new(20.0, 72.0),
                        UiPoint::new(8.0, 34.0),
                    ],
                    amount: 0.5,
                    fill: ColorRgba::new(120, 210, 180, 255),
                    stroke: Some(StrokeStyle::new(ColorRgba::WHITE, 1.0)),
                }],
                LayoutStyle::size(100.0, 80.0),
            ),
        );
        doc.compute_layout(UiSize::new(140.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let paint = doc.paint_list();
        let points = paint
            .items
            .iter()
            .find_map(|item| match &item.kind {
                PaintKind::Polygon { points, .. } => Some(points),
                _ => None,
            })
            .expect("morphed polygon paint");
        assert_eq!(points.len(), 5);
        assert_eq!(points[0], UiPoint::new(30.0, 16.0));
        assert!(points
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite()));
    }

    #[test]
    fn animated_morph_value_drives_morph_polygon_paint() {
        let animation = AnimationMachine::new(
            vec![
                AnimationState::new(
                    "square",
                    AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0),
                ),
                AnimationState::new(
                    "diamond",
                    AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0).with_morph(1.0),
                ),
            ],
            Vec::new(),
            "square",
        )
        .expect("animation")
        .with_number_input("amount", 0.5)
        .with_blend_binding(AnimationBlendBinding::new("amount", "square", "diamond"));

        let mut doc = UiDocument::new(root_style(120.0, 120.0));
        doc.add_child(
            doc.root,
            UiNode::scene(
                "animated_morph",
                vec![ScenePrimitive::MorphPolygon {
                    from_points: vec![
                        UiPoint::new(20.0, 20.0),
                        UiPoint::new(60.0, 20.0),
                        UiPoint::new(60.0, 60.0),
                        UiPoint::new(20.0, 60.0),
                    ],
                    to_points: vec![
                        UiPoint::new(40.0, 8.0),
                        UiPoint::new(72.0, 40.0),
                        UiPoint::new(40.0, 72.0),
                        UiPoint::new(8.0, 40.0),
                    ],
                    amount: 0.0,
                    fill: ColorRgba::new(160, 120, 220, 255),
                    stroke: None,
                }],
                LayoutStyle::size(100.0, 100.0),
            )
            .with_animation(animation),
        );
        doc.compute_layout(UiSize::new(120.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let paint = doc.paint_list();
        let points = paint
            .items
            .iter()
            .find_map(|item| match &item.kind {
                PaintKind::Polygon { points, .. } => Some(points),
                _ => None,
            })
            .expect("animated morph paint");
        assert_eq!(points[0], UiPoint::new(30.0, 14.0));
    }

    #[test]
    fn retained_animation_runtime_merges_fresh_input_values() {
        let base_animation = || {
            AnimationMachine::new(
                vec![
                    AnimationState::new(
                        "start",
                        AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0),
                    ),
                    AnimationState::new(
                        "end",
                        AnimatedValues::new(1.0, UiPoint::new(100.0, 0.0), 1.0),
                    ),
                ],
                Vec::new(),
                "start",
            )
            .expect("animation")
            .with_blend_binding(AnimationBlendBinding::new("scrub", "start", "end"))
        };
        let previous = base_animation().with_number_input("scrub", 0.80);
        let mut fresh = base_animation().with_number_input("scrub", 0.20);

        assert!(fresh.retain_runtime_from(&previous));
        assert_eq!(
            fresh.input("scrub").and_then(|value| value.as_number()),
            Some(0.20)
        );
        assert!((fresh.values().translate.x - 20.0).abs() <= f32::EPSILON);
    }

    #[test]
    fn document_ticks_node_animation_state_machines() {
        let animation = AnimationMachine::new(
            vec![
                AnimationState::new(
                    "closed",
                    AnimatedValues::new(0.0, UiPoint::new(0.0, 16.0), 1.0),
                ),
                AnimationState::new(
                    "open",
                    AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0),
                ),
            ],
            vec![AnimationTransition::new(
                "closed",
                "open",
                AnimationTrigger::Custom("inventory_open".to_string()),
                0.15,
            )],
            "closed",
        )
        .expect("animation");
        let mut doc = UiDocument::new(root_style(300.0, 200.0));
        let panel = doc.add_child(
            doc.root,
            UiNode::container(
                "inventory_panel",
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        align_items: Some(AlignItems::Center),
                        justify_content: Some(JustifyContent::Center),
                        position: Position::Relative,
                        size: TaffySize {
                            width: length(120.0),
                            height: length(80.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    ..Default::default()
                },
            )
            .with_animation(animation),
        );
        assert!(doc.trigger_animation(
            panel,
            AnimationTrigger::Custom("inventory_open".to_string())
        ));
        let report = doc.tick_animations(0.20);
        assert_eq!(
            report,
            AnimationTickReport {
                ticked: 1,
                advanced: 1,
                active: 0,
                completed: 1,
            }
        );
        assert_eq!(
            doc.node(panel)
                .animation
                .as_ref()
                .unwrap()
                .current_state_name(),
            "open"
        );
    }

    #[test]
    fn interaction_input_triggers_interruptible_node_animations() {
        let animation = AnimationMachine::new(
            vec![
                AnimationState::new(
                    "idle",
                    AnimatedValues::new(0.5, UiPoint::new(0.0, 0.0), 1.0),
                ),
                AnimationState::new(
                    "hover",
                    AnimatedValues::new(1.0, UiPoint::new(0.0, -2.0), 1.0),
                ),
            ],
            vec![
                AnimationTransition::new("idle", "hover", AnimationTrigger::PointerEnter, 0.20),
                AnimationTransition::new("hover", "idle", AnimationTrigger::PointerLeave, 0.20),
            ],
            "idle",
        )
        .expect("animation");
        let mut doc = UiDocument::new(root_style(160.0, 100.0));
        let button = doc.add_child(
            doc.root,
            UiNode::container("button", button_style(80.0, 40.0))
                .with_input(InputBehavior::BUTTON)
                .with_animation(animation),
        );
        doc.compute_layout(UiSize::new(160.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        doc.handle_input(UiInputEvent::PointerMove(UiPoint::new(20.0, 20.0)));
        assert!(doc.node(button).animation.as_ref().unwrap().is_animating());
        doc.tick_animations(0.10);
        let entered_opacity = doc
            .node(button)
            .animation
            .as_ref()
            .unwrap()
            .values()
            .opacity;
        assert!(entered_opacity > 0.5 && entered_opacity < 1.0);

        doc.handle_input(UiInputEvent::PointerMove(UiPoint::new(140.0, 80.0)));
        doc.tick_animations(0.20);
        let animation = doc.node(button).animation.as_ref().unwrap();
        assert_eq!(animation.current_state_name(), "idle");
        assert_eq!(animation.values().opacity, 0.5);
    }

    #[test]
    fn stale_focus_state_from_previous_document_is_discarded() {
        let mut doc = UiDocument::new(root_style(120.0, 80.0));
        doc.set_focus_state(UiFocusState {
            hovered: Some(UiNodeId(1509)),
            focused: Some(UiNodeId(1510)),
            pressed: Some(UiNodeId(1511)),
        });

        assert_eq!(doc.focus.hovered, None);
        assert_eq!(doc.focus.focused, None);
        assert_eq!(doc.focus.pressed, None);

        let result = doc.handle_input(UiInputEvent::PointerMove(UiPoint::new(4.0, 4.0)));
        assert_eq!(result.hovered, None);
        assert_eq!(result.focused, None);
        assert_eq!(result.pressed, None);
    }

    #[test]
    #[should_panic(expected = "UiDocument::add_child parent received stale or invalid node id")]
    fn invalid_add_child_parent_reports_operad_context() {
        let mut doc = UiDocument::new(root_style(120.0, 80.0));
        doc.add_child(
            UiNodeId(1509),
            UiNode::container("bad", LayoutStyle::size(20.0, 20.0)),
        );
    }

    #[test]
    #[should_panic(expected = "UiDocument::node received stale or invalid node id")]
    fn invalid_node_lookup_reports_operad_context() {
        let doc = UiDocument::new(root_style(120.0, 80.0));
        let _ = doc.node(UiNodeId(1509));
    }

    #[test]
    fn interaction_state_is_published_to_animation_inputs() {
        let animation = AnimationMachine::new(
            vec![
                AnimationState::new(
                    "idle",
                    AnimatedValues::new(0.6, UiPoint::new(0.0, 0.0), 1.0),
                ),
                AnimationState::new(
                    "hover",
                    AnimatedValues::new(1.0, UiPoint::new(8.0, 0.0), 1.0),
                ),
                AnimationState::new(
                    "activated",
                    AnimatedValues::new(1.0, UiPoint::new(12.0, 0.0), 0.96),
                ),
            ],
            vec![
                AnimationTransition::when(
                    "idle",
                    "hover",
                    AnimationCondition::bool(ANIMATION_INPUT_HOVER, true),
                    0.10,
                ),
                AnimationTransition::when(
                    "hover",
                    "idle",
                    AnimationCondition::bool(ANIMATION_INPUT_HOVER, false),
                    0.10,
                ),
                AnimationTransition::when(
                    "hover",
                    "activated",
                    AnimationCondition::trigger(ANIMATION_INPUT_ACTIVATED),
                    0.0,
                ),
            ],
            "idle",
        )
        .expect("animation");
        let mut doc = UiDocument::new(root_style(160.0, 100.0));
        let button = doc.add_child(
            doc.root,
            UiNode::container("button", button_style(80.0, 40.0))
                .with_input(InputBehavior::BUTTON)
                .with_animation(animation),
        );
        doc.compute_layout(UiSize::new(160.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        doc.handle_input(UiInputEvent::PointerMove(UiPoint::new(20.0, 20.0)));
        let animation = doc.node(button).animation.as_ref().unwrap();
        assert_eq!(
            animation
                .input(ANIMATION_INPUT_HOVER)
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            animation
                .input(ANIMATION_INPUT_ACTIVE)
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            animation
                .input(ANIMATION_INPUT_POINTER_X)
                .and_then(|value| value.as_number()),
            Some(20.0)
        );
        assert_eq!(
            animation
                .input(ANIMATION_INPUT_POINTER_NORM_X)
                .and_then(|value| value.as_number()),
            Some(0.25)
        );
        assert_eq!(animation.current_state_name(), "hover");

        doc.handle_input(UiInputEvent::PointerDown(UiPoint::new(20.0, 20.0)));
        let animation = doc.node(button).animation.as_ref().unwrap();
        assert_eq!(
            animation
                .input(ANIMATION_INPUT_PRESSED)
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            animation
                .input(ANIMATION_INPUT_FOCUSED)
                .and_then(|value| value.as_bool()),
            Some(true)
        );

        doc.handle_input(UiInputEvent::PointerUp(UiPoint::new(20.0, 20.0)));
        let animation = doc.node(button).animation.as_ref().unwrap();
        assert_eq!(animation.current_state_name(), "activated");
        assert_eq!(
            animation
                .input(ANIMATION_INPUT_PRESSED)
                .and_then(|value| value.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn animation_values_are_reflected_in_paint_without_relayout() {
        let animation = AnimationMachine::new(
            vec![
                AnimationState::new(
                    "hidden",
                    AnimatedValues::new(0.0, UiPoint::new(0.0, 20.0), 0.5),
                ),
                AnimationState::new(
                    "shown",
                    AnimatedValues::new(1.0, UiPoint::new(5.0, 0.0), 1.0),
                ),
            ],
            vec![AnimationTransition::new(
                "hidden",
                "shown",
                AnimationTrigger::Custom("show".to_string()),
                0.1,
            )],
            "hidden",
        )
        .expect("animation");
        let mut doc = UiDocument::new(root_style(160.0, 100.0));
        let panel = doc.add_child(
            doc.root,
            UiNode::container("toast", button_style(80.0, 30.0))
                .with_visual(UiVisual::panel(ColorRgba::WHITE, None, 0.0))
                .with_animation(animation),
        );
        doc.compute_layout(UiSize::new(160.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");
        assert!(doc.trigger_animation(panel, AnimationTrigger::Custom("show".to_string())));
        doc.tick_animations(0.1);

        let item = doc
            .paint_list()
            .items
            .into_iter()
            .find(|item| item.node == panel)
            .expect("paint item");
        assert_eq!(item.opacity, 1.0);
        assert_eq!(item.transform.translation, UiPoint::new(5.0, 0.0));
        assert_eq!(item.transform.scale, 1.0);
    }
}
