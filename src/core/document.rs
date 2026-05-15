use std::collections::{HashMap, HashSet};

#[cfg(feature = "text-cosmic")]
use cosmic_text::{
    Attrs, Buffer, Family as CosmicFamily, FontSystem, Metrics, Shaping, Stretch as CosmicStretch,
    Style as CosmicFontStyle, Weight as CosmicWeight, Wrap as CosmicWrap,
};
use taffy::prelude::{
    AlignItems, AvailableSpace, CompactLength, Dimension, Display, FlexDirection, JustifyContent,
    LengthPercentage, LengthPercentageAuto, NodeId as TaffyNodeId, Rect as TaffyRect,
    Size as TaffySize, Style, TaffyTree,
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
            style,
            locale: None,
            direction: ResolvedTextDirection::Ltr,
            bidi: BidiPolicy::default(),
            dynamic_label: None,
        }
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

#[derive(Debug, Clone, Copy)]
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
    pub visual: UiVisual,
    pub interaction_visuals: Option<InteractionVisuals>,
    pub interaction_text_styles: Option<TextInteractionStyles>,
    pub action: Option<actions::WidgetActionBinding>,
    pub action_mode: actions::WidgetActionMode,
    pub content: UiContent,
    pub input: InputBehavior,
    pub scroll: Option<ScrollState>,
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
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::Empty,
            input: InputBehavior::NONE,
            scroll: None,
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
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::Text(TextContent::new(text, text_style)),
            input: InputBehavior::NONE,
            scroll: None,
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
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::Image(image),
            input: InputBehavior::NONE,
            scroll: None,
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
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::PaintRect(rect),
            input: InputBehavior::NONE,
            scroll: None,
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
            visual: UiVisual::default(),
            interaction_visuals: None,
            interaction_text_styles: None,
            action: None,
            action_mode: actions::WidgetActionMode::Activate,
            content: UiContent::Scene(primitives),
            input: InputBehavior::NONE,
            scroll: None,
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
    Text(TextContent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntrinsicMeasureMode {
    Min,
    Preferred,
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
pub struct CosmicTextMeasurer {
    font_system: FontSystem,
    cache: HashMap<TextMeasureKey, UiSize>,
}

#[cfg(feature = "text-cosmic")]
impl CosmicTextMeasurer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            cache: HashMap::new(),
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
        let key = TextMeasureKey::new(text, known, available);
        if let Some(measured) = self.cache.get(&key).copied() {
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
        buffer.set_text(&mut self.font_system, &text.text, &attrs, Shaping::Advanced);

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
        if self.cache.len() > 4096 {
            self.cache.clear();
        }
        self.cache.insert(key, measured);
        measured
    }
}

fn measure_taffy_text(
    text_measurer: &mut impl TextMeasurer,
    text: &TextContent,
    known: TaffySize<Option<f32>>,
    available: TaffySize<AvailableSpace>,
) -> UiSize {
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
    pub hovered: Option<UiNodeId>,
    pub focused: Option<UiNodeId>,
    pub pressed: Option<UiNodeId>,
    pub clicked: Option<UiNodeId>,
    pub scrolled: Option<UiNodeId>,
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
    layout_revision: u64,
    layout_cache_key: Option<LayoutCacheKey>,
}

impl UiDocument {
    pub fn new(root_style: impl Into<UiNodeStyle>) -> Self {
        let root_style = root_style.into();
        let root = UiNodeId(0);
        Self {
            root,
            nodes: vec![UiNode::container("root", root_style)],
            focus: UiFocusState::default(),
            scale: UiDocumentScale::default(),
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
        self.invalidate_layout();
        let id = UiNodeId(self.nodes.len());
        node.parent = Some(parent);
        self.nodes.push(node);
        self.nodes[parent.0].children.push(id);
        id
    }

    pub fn node(&self, id: UiNodeId) -> &UiNode {
        &self.nodes[id.0]
    }

    pub fn nodes(&self) -> &[UiNode] {
        &self.nodes
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn node_mut(&mut self, id: UiNodeId) -> &mut UiNode {
        self.invalidate_layout();
        &mut self.nodes[id.0]
    }

    pub fn edit_node(&mut self, id: UiNodeId, edit: impl FnOnce(&mut UiNode)) {
        edit(&mut self.nodes[id.0]);
        self.invalidate_layout();
    }

    pub fn set_node_style(&mut self, id: UiNodeId, style: impl Into<UiNodeStyle>) {
        self.nodes[id.0].style = style.into();
        self.invalidate_layout();
    }

    pub fn set_node_content(&mut self, id: UiNodeId, content: UiContent) {
        self.nodes[id.0].content = content;
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
        self.nodes[id.0].input = input;
    }

    pub fn set_node_visual(&mut self, id: UiNodeId, visual: UiVisual) {
        self.nodes[id.0].visual = visual;
    }

    pub fn set_node_action(
        &mut self,
        id: UiNodeId,
        action: impl Into<actions::WidgetActionBinding>,
    ) {
        self.nodes[id.0].action = Some(action.into());
    }

    pub fn set_node_interaction_visuals(&mut self, id: UiNodeId, visuals: InteractionVisuals) {
        self.nodes[id.0].interaction_visuals = Some(visuals);
        self.refresh_interaction_visual(id);
    }

    pub fn clear_node_interaction_visuals(&mut self, id: UiNodeId) {
        self.nodes[id.0].interaction_visuals = None;
    }

    pub fn set_focus_state(&mut self, focus: UiFocusState) {
        let previous_hovered = self.focus.hovered;
        let previous_pressed = self.focus.pressed;
        let previous_focused = self.focus.focused;
        self.focus = focus;
        self.refresh_interaction_visuals_for(previous_hovered, previous_pressed, previous_focused);
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
        if changed {
            self.invalidate_layout();
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
        self.resolve_layout_constraints(text_measurer)?;
        let mut taffy = TaffyTree::<MeasureContext>::new();
        let mut mapping = HashMap::<UiNodeId, TaffyNodeId>::new();
        let root = self.build_taffy_subtree(self.root, &mut taffy, &mut mapping)?;
        let mut measured_content = HashMap::<TaffyNodeId, UiSize>::new();
        taffy.compute_layout_with_measure(
            root,
            TaffySize {
                width: AvailableSpace::Definite(viewport.width),
                height: AvailableSpace::Definite(viewport.height),
            },
            |known, available, node_id, context, _style| {
                let Some(MeasureContext::Text(text)) = context else {
                    return TaffySize::ZERO;
                };
                let measured = measure_taffy_text(text_measurer, text, known, available);
                measured_content.insert(node_id, measured);
                TaffySize {
                    width: measured.width,
                    height: measured.height,
                }
            },
        )?;
        let viewport_rect = UiRect::new(0.0, 0.0, viewport.width, viewport.height);
        self.apply_layout_subtree(
            self.root,
            root,
            &taffy,
            UiPoint::new(0.0, 0.0),
            viewport_rect,
            &mapping,
            &measured_content,
        )?;
        self.layout_cache_key = Some(cache_key);
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
            preferred: self.intrinsic_size_for_available_space(
                id,
                AvailableSpace::MaxContent,
                text_measurer,
            )?,
        })
    }

    fn intrinsic_size_for_available_space(
        &self,
        id: UiNodeId,
        width: AvailableSpace,
        text_measurer: &mut impl TextMeasurer,
    ) -> Result<UiSize, taffy::TaffyError> {
        let mut taffy = TaffyTree::<MeasureContext>::new();
        let mut mapping = HashMap::<UiNodeId, TaffyNodeId>::new();
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
                let Some(MeasureContext::Text(text)) = context else {
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
                for source in sources {
                    if source.0 >= self.nodes.len() {
                        continue;
                    }
                    let source_size = self.intrinsic_size(source, text_measurer)?;
                    min_stack.width = min_stack.width.max(source_size.min.width);
                    min_stack.height += source_size.min.height;
                    preferred_stack.width = preferred_stack.width.max(source_size.preferred.width);
                    preferred_stack.height += source_size.preferred.height;
                }
                let min_size = UiSize::new(
                    finite_or(min_size.width, 0.0).max(min_stack.width).max(1.0),
                    finite_or(min_size.height, 0.0)
                        .max(min_stack.height)
                        .max(1.0),
                );
                let preferred_size = UiSize::new(
                    preferred_stack.width.max(min_size.width),
                    preferred_stack.height.max(min_size.height),
                );
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
        mapping: &mut HashMap<UiNodeId, TaffyNodeId>,
    ) -> Result<TaffyNodeId, taffy::TaffyError> {
        self.build_taffy_subtree_for_intrinsic(id, taffy, mapping, None, None)
    }

    fn build_taffy_subtree_for_intrinsic(
        &self,
        id: UiNodeId,
        taffy: &mut TaffyTree<MeasureContext>,
        mapping: &mut HashMap<UiNodeId, TaffyNodeId>,
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
        let scroll_min_axes = (intrinsic_mode == Some(IntrinsicMeasureMode::Min)
            && intrinsic_root != Some(id)
            && node.scroll.is_some())
        .then(|| node.scroll.expect("checked scroll").axes);
        if let Some(axes) = scroll_min_axes {
            normalize_intrinsic_scroll_min_style(&mut style, axes);
        }
        let collapse_scroll_for_min =
            scroll_min_axes.is_some_and(|axes| axes.horizontal && axes.vertical);
        let taffy_node = if node.children.is_empty() || collapse_scroll_for_min {
            match &node.content {
                UiContent::Text(text) => taffy.new_leaf_with_context(
                    style,
                    MeasureContext::Text(scaled_text_content(text, layout_scale)),
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
        mapping.insert(id, taffy_node);
        Ok(taffy_node)
    }

    fn apply_layout_subtree(
        &mut self,
        id: UiNodeId,
        taffy_node: TaffyNodeId,
        taffy: &TaffyTree<MeasureContext>,
        parent_origin: UiPoint,
        parent_clip: UiRect,
        mapping: &HashMap<UiNodeId, TaffyNodeId>,
        measured_content: &HashMap<TaffyNodeId, UiSize>,
    ) -> Result<(), taffy::TaffyError> {
        let layout = taffy.layout(taffy_node)?;
        let mut rect = UiRect::new(
            parent_origin.x + layout.location.x,
            parent_origin.y + layout.location.y,
            layout.size.width,
            layout.size.height,
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
        let clip_rect = if has_scroll || self.nodes[id.0].style.clip == ClipBehavior::Clip {
            parent_clip
                .intersection(rect)
                .unwrap_or(UiRect::new(rect.x, rect.y, 0.0, 0.0))
        } else {
            parent_clip
        };
        self.nodes[id.0].layout = ComputedLayout {
            rect,
            clip_rect,
            visible: rect.intersects(parent_clip),
            opacity: self.nodes[id.0].style.opacity,
            content_size: measured_content.get(&taffy_node).copied(),
        };
        let children = self.nodes[id.0].children.clone();
        let child_origin = if has_scroll {
            UiPoint::new(rect.x - scroll_offset.x, rect.y - scroll_offset.y)
        } else {
            UiPoint::new(rect.x, rect.y)
        };
        for child in children {
            let child_taffy = mapping[&child];
            self.apply_layout_subtree(
                child,
                child_taffy,
                taffy,
                child_origin,
                clip_rect,
                mapping,
                measured_content,
            )?;
        }
        if has_scroll {
            let mut content_size = UiSize::new(rect.width, rect.height);
            self.include_descendant_content_bounds(id, child_origin, &mut content_size);
            let scroll = self.nodes[id.0]
                .scroll
                .as_mut()
                .expect("scroll state exists when has_scroll is true");
            scroll.viewport_size = UiSize::new(rect.width, rect.height);
            scroll.content_size = content_size;
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
            let child_rect = self.nodes[child.0].layout.rect;
            if rect_is_finite(child_rect) {
                content_size.width = content_size
                    .width
                    .max(child_rect.right() - content_origin.x);
                content_size.height = content_size
                    .height
                    .max(child_rect.bottom() - content_origin.y);
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
        let previous_hovered = self.focus.hovered;
        let previous_pressed = self.focus.pressed;
        let previous_focused = self.focus.focused;
        let mut scrolled = None;
        let clicked = match event {
            UiInputEvent::PointerMove(point) => {
                self.focus.hovered = self.hit_test(point);
                None
            }
            UiInputEvent::PointerDown(point) => {
                let hit = self.hit_test(point);
                self.focus.pressed = hit;
                if hit.is_some_and(|id| self.nodes[id.0].input.focusable) {
                    self.focus.focused = hit;
                }
                None
            }
            UiInputEvent::PointerUp(point) => {
                let hit = self.hit_test(point);
                let clicked = self.focus.pressed.filter(|pressed| Some(*pressed) == hit);
                self.focus.pressed = None;
                clicked
            }
            UiInputEvent::Wheel(wheel) => {
                scrolled = self.apply_wheel_scroll(wheel);
                None
            }
            UiInputEvent::TextInput(_) | UiInputEvent::Key { .. } => None,
            UiInputEvent::Focus(direction) => {
                self.focus.focused = self.next_focus(self.focus.focused, direction);
                None
            }
        };
        self.refresh_interaction_visuals_for(previous_hovered, previous_pressed, previous_focused);
        UiInputResult {
            hovered: self.focus.hovered,
            focused: self.focus.focused,
            pressed: self.focus.pressed,
            clicked,
            scrolled,
        }
    }

    fn apply_wheel_scroll(&mut self, wheel: UiWheelEvent) -> Option<UiNodeId> {
        if !wheel.scrolls_document() {
            return None;
        }
        for index in self.visual_order().into_iter().rev() {
            let target = {
                let node = &self.nodes[index];
                if node.layout.visible
                    && node.layout.clip_rect.contains_point(wheel.position)
                    && self.node_paint_rect(index).contains_point(wheel.position)
                    && node
                        .scroll
                        .is_some_and(|scroll| scroll.axes.horizontal || scroll.axes.vertical)
                {
                    Some(UiNodeId(index))
                } else {
                    None
                }
            };
            if let Some(target) = target {
                if self.scroll_by(target, wheel.delta) {
                    return Some(target);
                }
            }
        }
        None
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
        self.nodes
            .get_mut(id.0)
            .and_then(|node| node.animation.as_mut())
            .is_some_and(|animation| animation.trigger(trigger))
    }

    pub fn tick_animations(&mut self, dt_seconds: f32) {
        for node in &mut self.nodes {
            if let Some(animation) = &mut node.animation {
                animation.tick(dt_seconds);
            }
        }
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
                    list.items.push(PaintItem {
                        node: id,
                        rect: node.layout.rect,
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
                            node.layout.rect,
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
                    fill: *fill,
                    stroke: *stroke,
                },
            }
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
        while let Some(parent) = self.nodes[id.0].parent {
            if self.nodes[parent.0].scroll.is_some() {
                return true;
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
    if is_percent_dimension(style.size.width) {
        style.size.width = Dimension::auto();
    }
    if is_percent_dimension(style.size.height) {
        style.size.height = Dimension::auto();
    }
}

fn normalize_intrinsic_scroll_min_style(style: &mut Style, axes: ScrollAxes) {
    style.flex_grow = 0.0;
    style.flex_shrink = 1.0;
    style.flex_basis = Dimension::auto();
    if is_percent_dimension(style.size.width) {
        style.size.width = Dimension::auto();
    }
    if is_percent_dimension(style.size.height) {
        style.size.height = Dimension::auto();
    }
    if axes.horizontal {
        style.size.width = Dimension::length(1.0);
        style.min_size.width = Dimension::length(0.0);
        style.max_size.width = Dimension::length(1.0);
    }
    if axes.vertical {
        style.size.height = Dimension::length(1.0);
        style.min_size.height = Dimension::length(0.0);
        style.max_size.height = Dimension::length(1.0);
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

fn length_percentage_auto_points(value: LengthPercentageAuto) -> Option<f32> {
    let raw = value.into_raw();
    if raw.tag() == CompactLength::LENGTH_TAG {
        Some(raw.value())
    } else {
        None
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
}

impl AnimatedValues {
    pub const fn new(opacity: f32, translate: UiPoint, scale: f32) -> Self {
        Self {
            opacity,
            translate,
            scale,
        }
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

#[derive(Debug, Clone)]
pub struct AnimationTransition {
    pub from: String,
    pub to: String,
    pub trigger: AnimationTrigger,
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
            trigger,
            duration_seconds,
        }
    }
}

#[derive(Debug, Clone)]
struct ActiveTransition {
    from_values: AnimatedValues,
    to_state: usize,
    duration_seconds: f32,
    elapsed_seconds: f32,
}

#[derive(Debug, Clone)]
pub struct AnimationMachine {
    states: Vec<AnimationState>,
    transitions: Vec<AnimationTransition>,
    current_state: usize,
    active: Option<ActiveTransition>,
    values: AnimatedValues,
}

impl AnimationMachine {
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
            current_state,
            active: None,
            values,
        })
    }

    pub fn current_state_name(&self) -> &str {
        &self.states[self.current_state].name
    }

    pub fn values(&self) -> AnimatedValues {
        self.values
    }

    pub fn trigger(&mut self, trigger: AnimationTrigger) -> bool {
        let current_name = self.current_state_name();
        let Some(transition) = self
            .transitions
            .iter()
            .find(|transition| transition.from == current_name && transition.trigger == trigger)
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
        self.active = Some(ActiveTransition {
            from_values: self.values,
            to_state,
            duration_seconds: transition.duration_seconds.max(0.0),
            elapsed_seconds: 0.0,
        });
        true
    }

    pub fn tick(&mut self, dt_seconds: f32) {
        let Some(active) = &mut self.active else {
            return;
        };
        active.elapsed_seconds = (active.elapsed_seconds + dt_seconds.max(0.0)).max(0.0);
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
        }
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
                "fabricad.mask.viewport",
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
        assert_eq!(canvas_keys, vec!["fabricad.mask.viewport"]);
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
        doc.tick_animations(0.20);
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
