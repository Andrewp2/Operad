//! Operad is a small retained-mode UI document layer.
//!
//! The crate intentionally contains only reusable primitives: layout, text
//! measurement, hit testing, focus, animation, and optional renderer backends.
//! Product-specific screens and game/application state should live in the
//! consuming application crate.

use std::collections::{HashMap, HashSet};

#[cfg(feature = "text-cosmic")]
use cosmic_text::{
    Attrs, Buffer, Family as CosmicFamily, FontSystem, Metrics, Shaping, Stretch as CosmicStretch,
    Style as CosmicFontStyle, Weight as CosmicWeight, Wrap as CosmicWrap,
};
use taffy::prelude::{
    AvailableSpace, Dimension, Display, FlexDirection, NodeId as TaffyNodeId, Size as TaffySize,
    Style, TaffyTree,
};

pub mod accessibility;
pub mod commands;
pub mod platform;
pub mod theme;

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
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextContent {
    pub text: String,
    pub style: TextStyle,
}

impl TextContent {
    pub fn new(text: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasContent {
    pub key: String,
}

impl CanvasContent {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiContent {
    Empty,
    Text(TextContent),
    Canvas(CanvasContent),
    Image(ImageContent),
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

#[derive(Debug, Clone)]
pub struct UiNodeStyle {
    pub layout: Style,
    pub clip: ClipBehavior,
    pub opacity: f32,
    pub z_index: i16,
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
    pub visual: UiVisual,
    pub content: UiContent,
    pub input: InputBehavior,
    pub scroll: Option<ScrollState>,
    pub animation: Option<AnimationMachine>,
    pub accessibility: Option<AccessibilityMeta>,
    pub shader: Option<ShaderEffect>,
    pub layout: ComputedLayout,
}

impl UiNode {
    pub fn container(name: impl Into<String>, style: UiNodeStyle) -> Self {
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style,
            visual: UiVisual::default(),
            content: UiContent::Empty,
            input: InputBehavior::NONE,
            scroll: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn text(
        name: impl Into<String>,
        text: impl Into<String>,
        text_style: TextStyle,
        layout: Style,
    ) -> Self {
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: UiNodeStyle {
                layout,
                ..Default::default()
            },
            visual: UiVisual::default(),
            content: UiContent::Text(TextContent::new(text, text_style)),
            input: InputBehavior::NONE,
            scroll: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn canvas(name: impl Into<String>, key: impl Into<String>, layout: Style) -> Self {
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: UiNodeStyle {
                layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
            visual: UiVisual::default(),
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
            layout: ComputedLayout::default(),
        }
    }

    pub fn image(name: impl Into<String>, image: ImageContent, layout: Style) -> Self {
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: UiNodeStyle {
                layout,
                ..Default::default()
            },
            visual: UiVisual::default(),
            content: UiContent::Image(image),
            input: InputBehavior::NONE,
            scroll: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn scene(name: impl Into<String>, primitives: Vec<ScenePrimitive>, layout: Style) -> Self {
        Self {
            name: name.into(),
            parent: None,
            children: Vec::new(),
            style: UiNodeStyle {
                layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
            visual: UiVisual::default(),
            content: UiContent::Scene(primitives),
            input: InputBehavior::NONE,
            scroll: None,
            animation: None,
            accessibility: None,
            shader: None,
            layout: ComputedLayout::default(),
        }
    }

    pub fn with_input(mut self, input: InputBehavior) -> Self {
        self.input = input;
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComputedLayout {
    pub rect: UiRect,
    pub clip_rect: UiRect,
    pub visible: bool,
    pub opacity: f32,
}

impl Default for ComputedLayout {
    fn default() -> Self {
        Self {
            rect: UiRect::new(0.0, 0.0, 0.0, 0.0),
            clip_rect: UiRect::new(0.0, 0.0, 0.0, 0.0),
            visible: false,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
enum MeasureContext {
    Text(TextContent),
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

#[derive(Debug, Clone, PartialEq)]
pub enum UiInputEvent {
    PointerMove(UiPoint),
    PointerDown(UiPoint),
    PointerUp(UiPoint),
    Wheel {
        position: UiPoint,
        delta: UiPoint,
    },
    TextInput(String),
    Key {
        key: KeyCode,
        modifiers: KeyModifiers,
    },
    Focus(FocusDirection),
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
    revision: u64,
}

#[derive(Debug)]
pub struct UiDocument {
    pub root: UiNodeId,
    pub focus: UiFocusState,
    nodes: Vec<UiNode>,
    layout_revision: u64,
    layout_cache_key: Option<LayoutCacheKey>,
}

impl UiDocument {
    pub fn new(root_style: UiNodeStyle) -> Self {
        let root = UiNodeId(0);
        Self {
            root,
            nodes: vec![UiNode::container("root", root_style)],
            focus: UiFocusState::default(),
            layout_revision: 0,
            layout_cache_key: None,
        }
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

    pub fn set_node_style(&mut self, id: UiNodeId, style: UiNodeStyle) {
        self.nodes[id.0].style = style;
        self.invalidate_layout();
    }

    pub fn set_node_content(&mut self, id: UiNodeId, content: UiContent) {
        self.nodes[id.0].content = content;
        self.invalidate_layout();
    }

    pub fn set_node_input(&mut self, id: UiNodeId, input: InputBehavior) {
        self.nodes[id.0].input = input;
    }

    pub fn set_node_visual(&mut self, id: UiNodeId, visual: UiVisual) {
        self.nodes[id.0].visual = visual;
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
        let Some(scroll) = self.scroll_state(scroll_node) else {
            return false;
        };
        let Some(target_node) = self.nodes.get(target.0) else {
            return false;
        };
        let viewport = self.nodes[scroll_node.0].layout.rect;
        let target_rect = target_node.layout.rect;
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
            revision: self.layout_revision,
        };
        if self.layout_cache_key == Some(cache_key) {
            return Ok(());
        }
        let mut taffy = TaffyTree::<MeasureContext>::new();
        let mut mapping = HashMap::<UiNodeId, TaffyNodeId>::new();
        let root = self.build_taffy_subtree(self.root, &mut taffy, &mut mapping)?;
        taffy.compute_layout_with_measure(
            root,
            TaffySize {
                width: AvailableSpace::Definite(viewport.width),
                height: AvailableSpace::Definite(viewport.height),
            },
            |known, available, _node_id, context, _style| {
                let Some(MeasureContext::Text(text)) = context else {
                    return TaffySize::ZERO;
                };
                let measured = text_measurer.measure(
                    text,
                    KnownSize {
                        width: known.width,
                        height: known.height,
                    },
                    AvailableSize {
                        width: available_space_to_option(available.width),
                        height: available_space_to_option(available.height),
                    },
                );
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
        )?;
        self.layout_cache_key = Some(cache_key);
        Ok(())
    }

    fn build_taffy_subtree(
        &self,
        id: UiNodeId,
        taffy: &mut TaffyTree<MeasureContext>,
        mapping: &mut HashMap<UiNodeId, TaffyNodeId>,
    ) -> Result<TaffyNodeId, taffy::TaffyError> {
        let node = &self.nodes[id.0];
        let taffy_node = if node.children.is_empty() {
            match &node.content {
                UiContent::Text(text) => taffy.new_leaf_with_context(
                    node.style.layout.clone(),
                    MeasureContext::Text(text.clone()),
                )?,
                UiContent::Empty
                | UiContent::Canvas(_)
                | UiContent::Image(_)
                | UiContent::Scene(_) => taffy.new_leaf(node.style.layout.clone())?,
            }
        } else {
            let children = node
                .children
                .iter()
                .map(|child| self.build_taffy_subtree(*child, taffy, mapping))
                .collect::<Result<Vec<_>, _>>()?;
            taffy.new_with_children(node.style.layout.clone(), &children)?
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
    ) -> Result<(), taffy::TaffyError> {
        let layout = taffy.layout(taffy_node)?;
        let rect = UiRect::new(
            parent_origin.x + layout.location.x,
            parent_origin.y + layout.location.y,
            layout.size.width,
            layout.size.height,
        );
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
        };
        let children = self.nodes[id.0].children.clone();
        let child_origin = if has_scroll {
            UiPoint::new(rect.x - scroll_offset.x, rect.y - scroll_offset.y)
        } else {
            UiPoint::new(rect.x, rect.y)
        };
        for child in children {
            let child_taffy = mapping[&child];
            self.apply_layout_subtree(child, child_taffy, taffy, child_origin, clip_rect, mapping)?;
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
        for index in self.visual_order().into_iter().rev() {
            let node = &self.nodes[index];
            if !node.input.pointer
                || !node.layout.visible
                || !node.layout.clip_rect.contains_point(point)
            {
                continue;
            }
            if node.layout.rect.contains_point(point) {
                return Some(UiNodeId(index));
            }
        }
        None
    }

    pub fn handle_input(&mut self, event: UiInputEvent) -> UiInputResult {
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
            UiInputEvent::Wheel { position, delta } => {
                scrolled = self.apply_wheel_scroll(position, delta);
                None
            }
            UiInputEvent::TextInput(_) | UiInputEvent::Key { .. } => None,
            UiInputEvent::Focus(direction) => {
                self.focus.focused = self.next_focus(self.focus.focused, direction);
                None
            }
        };
        UiInputResult {
            hovered: self.focus.hovered,
            focused: self.focus.focused,
            pressed: self.focus.pressed,
            clicked,
            scrolled,
        }
    }

    fn apply_wheel_scroll(&mut self, position: UiPoint, delta: UiPoint) -> Option<UiNodeId> {
        let targets = self
            .visual_order()
            .into_iter()
            .rev()
            .filter_map(|index| {
                let node = &self.nodes[index];
                (node.layout.visible
                    && node.layout.clip_rect.contains_point(position)
                    && node.layout.rect.contains_point(position)
                    && node
                        .scroll
                        .is_some_and(|scroll| scroll.axes.horizontal || scroll.axes.vertical))
                .then_some(UiNodeId(index))
            })
            .collect::<Vec<_>>();

        targets
            .into_iter()
            .find(|&target| self.scroll_by(target, delta))
    }

    fn next_focus(&self, current: Option<UiNodeId>, direction: FocusDirection) -> Option<UiNodeId> {
        let focusable = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                (node.input.focusable
                    && node.layout.visible
                    && node.layout.rect.intersects(node.layout.clip_rect))
                .then_some(UiNodeId(index))
            })
            .collect::<Vec<_>>();
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
        let mut list = PaintList::default();
        let z_indexes = self.effective_z_indexes();
        for index in self.visual_order_with_z(&z_indexes) {
            let id = UiNodeId(index);
            let node = &self.nodes[index];
            if !node.layout.visible
                || node.layout.clip_rect.width <= f32::EPSILON
                || node.layout.clip_rect.height <= f32::EPSILON
            {
                continue;
            }
            let z_index = z_indexes[index];
            let animation_values = node
                .animation
                .as_ref()
                .map(AnimationMachine::values)
                .unwrap_or_default();
            let opacity = node.layout.opacity * animation_values.opacity;
            let transform = PaintTransform {
                translation: animation_values.translate,
                scale: animation_values.scale,
            };
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
                UiContent::Text(text) => list.items.push(PaintItem {
                    node: id,
                    rect: node.layout.rect,
                    clip_rect: node.layout.clip_rect,
                    z_index,
                    opacity,
                    transform,
                    shader: node.shader.clone(),
                    kind: PaintKind::Text(text.clone()),
                }),
                UiContent::Canvas(canvas) => list.items.push(PaintItem {
                    node: id,
                    rect: node.layout.rect,
                    clip_rect: node.layout.clip_rect,
                    z_index,
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
                    opacity,
                    transform,
                    shader: node.shader.clone(),
                    kind: PaintKind::Image {
                        key: image.key.clone(),
                        tint: image.tint,
                    },
                }),
                UiContent::Scene(primitives) => {
                    let context = ScenePaintContext {
                        node: id,
                        node_rect: node.layout.rect,
                        clip_rect: node.layout.clip_rect,
                        z_index,
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

    fn visual_order(&self) -> Vec<usize> {
        let z_indexes = self.effective_z_indexes();
        self.visual_order_with_z(&z_indexes)
    }

    fn visual_order_with_z(&self, z_indexes: &[i16]) -> Vec<usize> {
        let mut order = (0..self.nodes.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| (z_indexes[*index], *index));
        order
    }

    fn effective_z_indexes(&self) -> Vec<i16> {
        let mut effective_z = vec![0_i16; self.nodes.len()];
        for index in 0..self.nodes.len() {
            let node = &self.nodes[index];
            effective_z[index] = if index == self.root.0 {
                node.style.z_index
            } else if node.style.z_index == 0 {
                node.parent
                    .map(|parent| effective_z[parent.0])
                    .unwrap_or(node.style.z_index)
            } else {
                node.style.z_index
            };
        }
        effective_z
    }
}

#[derive(Debug, Clone)]
struct ScenePaintContext {
    node: UiNodeId,
    node_rect: UiRect,
    clip_rect: UiRect,
    z_index: i16,
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
            opacity: context.opacity,
            transform: context.transform,
            shader: context.shader.clone(),
            kind: PaintKind::Image {
                key: key.clone(),
                tint: *tint,
            },
        },
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
    pub opacity: f32,
    pub transform: PaintTransform,
    pub shader: Option<ShaderEffect>,
    pub kind: PaintKind,
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
    TextClipped {
        node: UiNodeId,
        name: String,
        rect: UiRect,
        clip_rect: UiRect,
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
            if matches!(node.content, UiContent::Text(_))
                && !node.layout.clip_rect.contains_rect(node.layout.rect)
                && !self.has_scroll_ancestor(id)
            {
                warnings.push(AuditWarning::TextClipped {
                    node: id,
                    name: node.name.clone(),
                    rect: node.layout.rect,
                    clip_rect: node.layout.clip_rect,
                });
            }
            if id != self.root
                && !root_rect.contains_rect(node.layout.rect)
                && !self.has_scroll_ancestor(id)
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

fn rect_is_finite(rect: UiRect) -> bool {
    rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite()
}

#[cfg(feature = "widgets")]
mod widget_ext;

#[cfg(feature = "widgets")]
pub mod widgets {
    use std::ops::Range;

    use taffy::prelude::{AlignItems, JustifyContent};

    use super::*;

    pub use crate::widget_ext::*;

    #[derive(Debug, Clone)]
    pub struct ButtonOptions {
        pub layout: Style,
        pub visual: UiVisual,
        pub pressed_visual: Option<UiVisual>,
        pub focused_visual: Option<UiVisual>,
        pub disabled_visual: Option<UiVisual>,
        pub text_style: TextStyle,
        pub leading_image: Option<ImageContent>,
        pub image_size: UiSize,
        pub image_shader: Option<ShaderEffect>,
        pub shader: Option<ShaderEffect>,
        pub animation: Option<AnimationMachine>,
        pub enabled: bool,
        pub pressed: bool,
        pub focused: bool,
        pub accessibility_label: Option<String>,
        pub accessibility_hint: Option<String>,
    }

    impl ButtonOptions {
        pub fn new(layout: Style) -> Self {
            Self {
                layout,
                ..Default::default()
            }
        }
    }

    impl Default for ButtonOptions {
        fn default() -> Self {
            Self {
                layout: Style {
                    display: Display::Flex,
                    align_items: Some(AlignItems::Center),
                    justify_content: Some(JustifyContent::Center),
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
                visual: UiVisual::panel(
                    ColorRgba::new(36, 42, 52, 255),
                    Some(StrokeStyle::new(ColorRgba::new(74, 85, 104, 255), 1.0)),
                    4.0,
                ),
                pressed_visual: Some(UiVisual::panel(
                    ColorRgba::new(22, 27, 35, 255),
                    Some(StrokeStyle::new(ColorRgba::new(104, 128, 156, 255), 1.0)),
                    4.0,
                )),
                focused_visual: Some(UiVisual::panel(
                    ColorRgba::new(40, 49, 61, 255),
                    Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.5)),
                    4.0,
                )),
                disabled_visual: Some(UiVisual::panel(
                    ColorRgba::new(30, 34, 40, 180),
                    Some(StrokeStyle::new(ColorRgba::new(64, 72, 84, 180), 1.0)),
                    4.0,
                )),
                text_style: TextStyle::default(),
                leading_image: None,
                image_size: UiSize::new(18.0, 18.0),
                image_shader: None,
                shader: None,
                animation: None,
                enabled: true,
                pressed: false,
                focused: false,
                accessibility_label: None,
                accessibility_hint: None,
            }
        }
    }

    impl ButtonOptions {
        fn resolved_visual(&self) -> UiVisual {
            if !self.enabled {
                self.disabled_visual.unwrap_or(self.visual)
            } else if self.pressed {
                self.pressed_visual.unwrap_or(self.visual)
            } else if self.focused {
                self.focused_visual.unwrap_or(self.visual)
            } else {
                self.visual
            }
        }
    }

    pub fn button(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        label: impl Into<String>,
        options: ButtonOptions,
    ) -> UiNodeId {
        let name = name.into();
        let label = label.into();
        let accessibility_label = options
            .accessibility_label
            .clone()
            .unwrap_or_else(|| label.clone());
        let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Button)
            .label(accessibility_label)
            .pressed(options.pressed)
            .action(AccessibilityAction::new("activate", "Activate"));
        if let Some(hint) = options.accessibility_hint.clone() {
            accessibility = accessibility.hint(hint);
        }
        if options.enabled {
            accessibility = accessibility.focusable();
        } else {
            accessibility = accessibility.disabled();
        }
        let visual = options.resolved_visual();
        let mut node = UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(if options.enabled {
            InputBehavior::BUTTON
        } else {
            InputBehavior::NONE
        })
        .with_visual(visual)
        .with_accessibility(accessibility);
        if let Some(shader) = options.shader {
            node = node.with_shader(shader);
        }
        if let Some(animation) = options.animation {
            node = node.with_animation(animation);
        }
        let button = document.add_child(parent, node);
        if let Some(image) = options.leading_image {
            let mut image_node = UiNode::image(
                format!("{name}.image"),
                image,
                Style {
                    size: TaffySize {
                        width: length(options.image_size.width),
                        height: length(options.image_size.height),
                    },
                    margin: taffy::prelude::Rect {
                        right: taffy::prelude::LengthPercentageAuto::length(6.0),
                        ..taffy::prelude::Rect::length(0.0)
                    },
                    ..Default::default()
                },
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Image).label(label.clone()),
            );
            if let Some(shader) = options.image_shader {
                image_node = image_node.with_shader(shader);
            }
            document.add_child(button, image_node);
        }
        document.add_child(
            button,
            UiNode::text(
                format!("{name}.label"),
                label,
                options.text_style,
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            ),
        );
        button
    }

    pub fn label(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        text: impl Into<String>,
        style: TextStyle,
        layout: Style,
    ) -> UiNodeId {
        let text = text.into();
        document.add_child(
            parent,
            UiNode::text(name, text.clone(), style, layout)
                .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label(text)),
        )
    }

    pub fn scroll_area(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        axes: ScrollAxes,
        layout: Style,
    ) -> UiNodeId {
        let name = name.into();
        document.add_child(
            parent,
            UiNode::container(
                name.clone(),
                UiNodeStyle {
                    layout,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(axes)
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::List)
                    .label(name)
                    .value(scroll_axes_value(axes)),
            ),
        )
    }

    fn scroll_axes_value(axes: ScrollAxes) -> &'static str {
        match axes {
            ScrollAxes {
                horizontal: false,
                vertical: false,
            } => "not scrollable",
            ScrollAxes {
                horizontal: true,
                vertical: false,
            } => "horizontal",
            ScrollAxes {
                horizontal: false,
                vertical: true,
            } => "vertical",
            ScrollAxes {
                horizontal: true,
                vertical: true,
            } => "horizontal and vertical",
        }
    }

    #[derive(Debug, Clone)]
    pub struct CheckboxOptions {
        pub layout: Style,
        pub box_visual: UiVisual,
        pub checked_box_visual: Option<UiVisual>,
        pub disabled_box_visual: Option<UiVisual>,
        pub check_color: ColorRgba,
        pub check_image: Option<ImageContent>,
        pub check_shader: Option<ShaderEffect>,
        pub text_style: TextStyle,
        pub shader: Option<ShaderEffect>,
        pub animation: Option<AnimationMachine>,
        pub enabled: bool,
        pub accessibility_label: Option<String>,
        pub accessibility_hint: Option<String>,
    }

    impl Default for CheckboxOptions {
        fn default() -> Self {
            Self {
                layout: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: length(28.0),
                    },
                    ..Default::default()
                },
                box_visual: UiVisual::panel(
                    ColorRgba::new(29, 35, 43, 255),
                    Some(StrokeStyle::new(ColorRgba::new(98, 113, 135, 255), 1.0)),
                    3.0,
                ),
                checked_box_visual: Some(UiVisual::panel(
                    ColorRgba::new(21, 58, 92, 255),
                    Some(StrokeStyle::new(ColorRgba::new(108, 180, 255, 255), 1.0)),
                    3.0,
                )),
                disabled_box_visual: Some(UiVisual::panel(
                    ColorRgba::new(28, 32, 38, 160),
                    Some(StrokeStyle::new(ColorRgba::new(67, 75, 88, 160), 1.0)),
                    3.0,
                )),
                check_color: ColorRgba::new(108, 180, 255, 255),
                check_image: None,
                check_shader: None,
                text_style: TextStyle::default(),
                shader: None,
                animation: None,
                enabled: true,
                accessibility_label: None,
                accessibility_hint: None,
            }
        }
    }

    pub fn checkbox(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        label_text: impl Into<String>,
        checked: bool,
        options: CheckboxOptions,
    ) -> UiNodeId {
        let name = name.into();
        let label_text = label_text.into();
        let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Checkbox)
            .label(
                options
                    .accessibility_label
                    .clone()
                    .unwrap_or_else(|| label_text.clone()),
            )
            .value(if checked { "checked" } else { "unchecked" })
            .checked(checked)
            .action(AccessibilityAction::new("toggle", "Toggle"));
        if let Some(hint) = options.accessibility_hint.clone() {
            accessibility = accessibility.hint(hint);
        }
        if options.enabled {
            accessibility = accessibility.focusable();
        } else {
            accessibility = accessibility.disabled();
        }
        let mut root_node = UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(if options.enabled {
            InputBehavior::BUTTON
        } else {
            InputBehavior::NONE
        })
        .with_accessibility(accessibility);
        if let Some(shader) = options.shader {
            root_node = root_node.with_shader(shader);
        }
        if let Some(animation) = options.animation {
            root_node = root_node.with_animation(animation);
        }
        let root = document.add_child(parent, root_node);
        let box_visual = if !options.enabled {
            options.disabled_box_visual.unwrap_or(options.box_visual)
        } else if checked {
            options.checked_box_visual.unwrap_or(options.box_visual)
        } else {
            options.box_visual
        };
        let box_node = document.add_child(
            root,
            UiNode::container(
                format!("{name}.box"),
                UiNodeStyle {
                    layout: Style {
                        size: TaffySize {
                            width: length(16.0),
                            height: length(16.0),
                        },
                        margin: taffy::prelude::Rect {
                            left: taffy::prelude::LengthPercentageAuto::length(0.0),
                            right: taffy::prelude::LengthPercentageAuto::length(8.0),
                            top: taffy::prelude::LengthPercentageAuto::length(0.0),
                            bottom: taffy::prelude::LengthPercentageAuto::length(0.0),
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .with_visual(box_visual),
        );
        if checked {
            if let Some(image) = options.check_image {
                let mut check_node = UiNode::image(
                    format!("{name}.check"),
                    image,
                    Style {
                        size: TaffySize {
                            width: length(16.0),
                            height: length(16.0),
                        },
                        ..Default::default()
                    },
                );
                if let Some(shader) = options.check_shader {
                    check_node = check_node.with_shader(shader);
                }
                document.add_child(box_node, check_node);
            } else {
                let mut check_node = UiNode::scene(
                    format!("{name}.check"),
                    vec![
                        ScenePrimitive::Line {
                            from: UiPoint::new(3.0, 8.0),
                            to: UiPoint::new(6.5, 11.5),
                            stroke: StrokeStyle::new(options.check_color, 2.0),
                        },
                        ScenePrimitive::Line {
                            from: UiPoint::new(6.5, 11.5),
                            to: UiPoint::new(13.0, 4.0),
                            stroke: StrokeStyle::new(options.check_color, 2.0),
                        },
                    ],
                    Style {
                        size: TaffySize {
                            width: length(16.0),
                            height: length(16.0),
                        },
                        ..Default::default()
                    },
                );
                if let Some(shader) = options.check_shader {
                    check_node = check_node.with_shader(shader);
                }
                document.add_child(box_node, check_node);
            }
        }
        label(
            document,
            root,
            format!("{name}.label"),
            label_text,
            options.text_style,
            Style {
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
        );
        root
    }

    #[derive(Debug, Clone)]
    pub struct SliderOptions {
        pub layout: Style,
        pub track_visual: UiVisual,
        pub fill_color: ColorRgba,
        pub thumb_visual: UiVisual,
        pub disabled_track_visual: Option<UiVisual>,
        pub disabled_fill_color: Option<ColorRgba>,
        pub disabled_thumb_visual: Option<UiVisual>,
        pub track_shader: Option<ShaderEffect>,
        pub fill_shader: Option<ShaderEffect>,
        pub thumb_shader: Option<ShaderEffect>,
        pub shader: Option<ShaderEffect>,
        pub animation: Option<AnimationMachine>,
        pub enabled: bool,
        pub accessibility_label: Option<String>,
        pub accessibility_value: Option<String>,
        pub accessibility_hint: Option<String>,
    }

    impl Default for SliderOptions {
        fn default() -> Self {
            Self {
                layout: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: length(160.0),
                        height: length(28.0),
                    },
                    ..Default::default()
                },
                track_visual: UiVisual::panel(ColorRgba::new(42, 49, 58, 255), None, 3.0),
                fill_color: ColorRgba::new(108, 180, 255, 255),
                thumb_visual: UiVisual::panel(
                    ColorRgba::new(235, 240, 247, 255),
                    Some(StrokeStyle::new(ColorRgba::new(79, 93, 113, 255), 1.0)),
                    6.0,
                ),
                disabled_track_visual: Some(UiVisual::panel(
                    ColorRgba::new(35, 39, 45, 180),
                    None,
                    3.0,
                )),
                disabled_fill_color: Some(ColorRgba::new(92, 101, 114, 180)),
                disabled_thumb_visual: Some(UiVisual::panel(
                    ColorRgba::new(150, 158, 170, 180),
                    Some(StrokeStyle::new(ColorRgba::new(81, 90, 104, 180), 1.0)),
                    6.0,
                )),
                track_shader: None,
                fill_shader: None,
                thumb_shader: None,
                shader: None,
                animation: None,
                enabled: true,
                accessibility_label: None,
                accessibility_value: None,
                accessibility_hint: None,
            }
        }
    }

    pub fn slider(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        value: f32,
        range: Range<f32>,
        options: SliderOptions,
    ) -> UiNodeId {
        let name = name.into();
        let t = normalized_value(value, range.clone());
        let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Slider)
            .label(
                options
                    .accessibility_label
                    .clone()
                    .unwrap_or_else(|| name.clone()),
            )
            .value(
                options
                    .accessibility_value
                    .clone()
                    .unwrap_or_else(|| slider_accessibility_value(value, range.clone())),
            )
            .value_range(AccessibilityValueRange::new(
                range.start as f64,
                range.end as f64,
            ))
            .action(AccessibilityAction::new("increase", "Increase"))
            .action(AccessibilityAction::new("decrease", "Decrease"));
        if let Some(hint) = options.accessibility_hint.clone() {
            accessibility = accessibility.hint(hint);
        }
        if options.enabled {
            accessibility = accessibility.focusable();
        } else {
            accessibility = accessibility.disabled();
        }
        let mut root_node = UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(if options.enabled {
            InputBehavior::BUTTON
        } else {
            InputBehavior::NONE
        })
        .with_accessibility(accessibility);
        if let Some(shader) = options.shader {
            root_node = root_node.with_shader(shader);
        }
        if let Some(animation) = options.animation {
            root_node = root_node.with_animation(animation);
        }
        let root = document.add_child(parent, root_node);
        let track_visual = if options.enabled {
            options.track_visual
        } else {
            options
                .disabled_track_visual
                .unwrap_or(options.track_visual)
        };
        let fill_color = if options.enabled {
            options.fill_color
        } else {
            options.disabled_fill_color.unwrap_or(options.fill_color)
        };
        let thumb_visual = if options.enabled {
            options.thumb_visual
        } else {
            options
                .disabled_thumb_visual
                .unwrap_or(options.thumb_visual)
        };
        let mut track_node = UiNode::container(
            format!("{name}.track"),
            UiNodeStyle {
                layout: Style {
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(6.0),
                    },
                    ..Default::default()
                },
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(track_visual);
        if let Some(shader) = options.track_shader {
            track_node = track_node.with_shader(shader);
        }
        let track = document.add_child(root, track_node);
        let mut fill_node = UiNode::container(
            format!("{name}.fill"),
            UiNodeStyle {
                layout: Style {
                    size: TaffySize {
                        width: Dimension::percent(t),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(fill_color, None, 3.0));
        if let Some(shader) = options.fill_shader {
            fill_node = fill_node.with_shader(shader);
        }
        document.add_child(track, fill_node);
        let mut thumb_node = UiNode::container(
            format!("{name}.thumb"),
            UiNodeStyle {
                layout: Style {
                    size: TaffySize {
                        width: length(12.0),
                        height: length(12.0),
                    },
                    margin: taffy::prelude::Rect {
                        left: taffy::prelude::LengthPercentageAuto::length(-6.0),
                        right: taffy::prelude::LengthPercentageAuto::length(0.0),
                        top: taffy::prelude::LengthPercentageAuto::length(0.0),
                        bottom: taffy::prelude::LengthPercentageAuto::length(0.0),
                    },
                    ..Default::default()
                },
                z_index: 1,
                ..Default::default()
            },
        )
        .with_visual(thumb_visual);
        if let Some(shader) = options.thumb_shader {
            thumb_node = thumb_node.with_shader(shader);
        }
        document.add_child(root, thumb_node);
        root
    }

    fn slider_accessibility_value(value: f32, range: Range<f32>) -> String {
        let percent = normalized_value(value, range) * 100.0;
        format!("{value} ({percent:.0}%)")
    }

    pub fn normalized_value(value: f32, range: Range<f32>) -> f32 {
        let span = range.end - range.start;
        if span.abs() <= f32::EPSILON {
            return 0.0;
        }
        ((value - range.start) / span).clamp(0.0, 1.0)
    }

    pub fn slider_value_from_point(track: UiRect, point: UiPoint, range: Range<f32>) -> f32 {
        let t = ((point.x - track.x) / track.width.max(1.0)).clamp(0.0, 1.0);
        range.start + (range.end - range.start) * t
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TextInputState {
        pub text: String,
        pub caret: usize,
        pub selection_anchor: Option<usize>,
        pub multiline: bool,
        pub composing: Option<String>,
    }

    impl TextInputState {
        pub fn new(text: impl Into<String>) -> Self {
            let text = text.into();
            Self {
                caret: text.len(),
                text,
                selection_anchor: None,
                multiline: false,
                composing: None,
            }
        }

        pub fn multiline(mut self, multiline: bool) -> Self {
            self.multiline = multiline;
            self
        }

        pub fn selected_range(&self) -> Option<Range<usize>> {
            let anchor = clamp_to_char_boundary(&self.text, self.selection_anchor?);
            let caret = clamp_to_char_boundary(&self.text, self.caret);
            if anchor == caret {
                return None;
            }
            Some(anchor.min(caret)..anchor.max(caret))
        }

        pub fn select_all(&mut self) {
            self.selection_anchor = Some(0);
            self.caret = self.text.len();
        }

        pub fn clear_selection(&mut self) {
            self.selection_anchor = None;
        }

        pub fn insert_text(&mut self, text: &str) {
            let filtered = filter_text_input(text, self.multiline);
            self.replace_selection(&filtered);
        }

        pub fn copy_selection(&self) -> Option<String> {
            self.selected_range()
                .map(|range| self.text[range].to_string())
        }

        pub fn cut_selection(&mut self) -> Option<String> {
            let copied = self.copy_selection()?;
            self.replace_selection("");
            Some(copied)
        }

        pub fn paste_text(&mut self, text: &str) {
            let filtered = filter_text_input(text, self.multiline);
            self.replace_selection(&filtered);
        }

        pub fn paste_text_with_outcome(&mut self, text: &str) -> TextInputOutcome {
            let before = self.text.clone();
            self.paste_text(text);
            TextInputOutcome::new(EditPhase::UpdateEdit, before != self.text, None)
        }

        pub fn replace_selection(&mut self, text: &str) {
            self.normalize_selection();
            if let Some(range) = self.selected_range() {
                self.text.replace_range(range.clone(), text);
                self.caret = range.start + text.len();
            } else {
                self.text.insert_str(self.caret, text);
                self.caret += text.len();
            }
            self.caret = clamp_to_char_boundary(&self.text, self.caret);
            self.selection_anchor = None;
        }

        pub fn backspace(&mut self) -> bool {
            self.normalize_selection();
            if self.selected_range().is_some() {
                self.replace_selection("");
                return true;
            }
            if self.caret == 0 {
                return false;
            }
            let previous = previous_char_boundary(&self.text, self.caret);
            self.text.replace_range(previous..self.caret, "");
            self.caret = previous;
            true
        }

        pub fn delete(&mut self) -> bool {
            self.normalize_selection();
            if self.selected_range().is_some() {
                self.replace_selection("");
                return true;
            }
            if self.caret >= self.text.len() {
                return false;
            }
            let next = next_char_boundary(&self.text, self.caret);
            self.text.replace_range(self.caret..next, "");
            true
        }

        pub fn move_caret(&mut self, movement: CaretMovement, selecting: bool) {
            self.normalize_selection();
            let anchor = self.selection_anchor.unwrap_or(self.caret);
            self.caret = match movement {
                CaretMovement::Start => 0,
                CaretMovement::End => self.text.len(),
                CaretMovement::Left => previous_char_boundary(&self.text, self.caret),
                CaretMovement::Right => next_char_boundary(&self.text, self.caret),
            };
            self.caret = clamp_to_char_boundary(&self.text, self.caret);
            self.selection_anchor = selecting.then_some(anchor);
        }

        pub fn handle_event(&mut self, event: &UiInputEvent) -> TextInputOutcome {
            let before = self.text.clone();
            let mut phase = EditPhase::Preview;
            let mut clipboard = None;
            match event {
                UiInputEvent::TextInput(text) => {
                    self.insert_text(text);
                    phase = EditPhase::UpdateEdit;
                }
                UiInputEvent::Key { key, modifiers } => match key {
                    KeyCode::Character(character) if modifiers.ctrl || modifiers.meta => {
                        match character.to_ascii_lowercase() {
                            'a' => self.select_all(),
                            'c' => {
                                clipboard =
                                    self.copy_selection().map(TextInputClipboardAction::Copy);
                            }
                            'x' => {
                                clipboard = self.cut_selection().map(TextInputClipboardAction::Cut);
                                if clipboard.is_some() {
                                    phase = EditPhase::UpdateEdit;
                                }
                            }
                            'v' => {
                                clipboard = Some(TextInputClipboardAction::Paste);
                            }
                            _ => {}
                        }
                    }
                    KeyCode::Backspace => {
                        if self.backspace() {
                            phase = EditPhase::UpdateEdit;
                        }
                    }
                    KeyCode::Delete => {
                        if self.delete() {
                            phase = EditPhase::UpdateEdit;
                        }
                    }
                    KeyCode::ArrowLeft => {
                        self.move_caret(CaretMovement::Left, modifiers.shift);
                    }
                    KeyCode::ArrowRight => {
                        self.move_caret(CaretMovement::Right, modifiers.shift);
                    }
                    KeyCode::Home => {
                        self.move_caret(CaretMovement::Start, modifiers.shift);
                    }
                    KeyCode::End => {
                        self.move_caret(CaretMovement::End, modifiers.shift);
                    }
                    KeyCode::Enter if self.multiline => {
                        self.insert_text("\n");
                        phase = EditPhase::UpdateEdit;
                    }
                    KeyCode::Enter => phase = EditPhase::CommitEdit,
                    KeyCode::Escape => phase = EditPhase::CancelEdit,
                    _ => {}
                },
                _ => {}
            }
            TextInputOutcome::new(phase, before != self.text, clipboard)
        }

        fn normalize_selection(&mut self) {
            self.caret = clamp_to_char_boundary(&self.text, self.caret);
            self.selection_anchor = self
                .selection_anchor
                .map(|anchor| clamp_to_char_boundary(&self.text, anchor));
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CaretMovement {
        Start,
        End,
        Left,
        Right,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum TextInputClipboardAction {
        Copy(String),
        Cut(String),
        Paste,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TextInputOutcome {
        pub phase: EditPhase,
        pub changed: bool,
        pub committed: bool,
        pub canceled: bool,
        pub clipboard: Option<TextInputClipboardAction>,
    }

    impl TextInputOutcome {
        fn new(
            phase: EditPhase,
            changed: bool,
            clipboard: Option<TextInputClipboardAction>,
        ) -> Self {
            Self {
                phase,
                changed,
                committed: phase == EditPhase::CommitEdit,
                canceled: phase == EditPhase::CancelEdit,
                clipboard,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct TextInputOptions {
        pub layout: Style,
        pub visual: UiVisual,
        pub focused_visual: Option<UiVisual>,
        pub disabled_visual: Option<UiVisual>,
        pub text_style: TextStyle,
        pub placeholder_style: TextStyle,
        pub placeholder: String,
        pub shader: Option<ShaderEffect>,
        pub animation: Option<AnimationMachine>,
        pub enabled: bool,
        pub focused: bool,
        pub accessibility_label: Option<String>,
        pub accessibility_hint: Option<String>,
    }

    impl Default for TextInputOptions {
        fn default() -> Self {
            let placeholder_style = TextStyle {
                color: ColorRgba::new(144, 156, 174, 255),
                ..Default::default()
            };
            Self {
                layout: Style {
                    size: TaffySize {
                        width: length(180.0),
                        height: length(30.0),
                    },
                    padding: taffy::prelude::Rect::length(6.0),
                    ..Default::default()
                },
                visual: UiVisual::panel(
                    ColorRgba::new(18, 22, 28, 255),
                    Some(StrokeStyle::new(ColorRgba::new(72, 84, 104, 255), 1.0)),
                    4.0,
                ),
                focused_visual: Some(UiVisual::panel(
                    ColorRgba::new(20, 27, 36, 255),
                    Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.5)),
                    4.0,
                )),
                disabled_visual: Some(UiVisual::panel(
                    ColorRgba::new(25, 28, 34, 170),
                    Some(StrokeStyle::new(ColorRgba::new(58, 66, 78, 170), 1.0)),
                    4.0,
                )),
                text_style: TextStyle::default(),
                placeholder_style,
                placeholder: String::new(),
                shader: None,
                animation: None,
                enabled: true,
                focused: false,
                accessibility_label: None,
                accessibility_hint: None,
            }
        }
    }

    pub fn text_input(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        state: &TextInputState,
        options: TextInputOptions,
    ) -> UiNodeId {
        let name = name.into();
        let mut accessibility = AccessibilityMeta::new(AccessibilityRole::TextBox)
            .label(
                options
                    .accessibility_label
                    .clone()
                    .unwrap_or_else(|| name.clone()),
            )
            .value(state.text.clone())
            .shortcut("Ctrl+A")
            .shortcut("Ctrl+C")
            .shortcut("Ctrl+X")
            .shortcut("Ctrl+V")
            .action(AccessibilityAction::new("select_all", "Select all").shortcut("Ctrl+A"))
            .action(AccessibilityAction::new("copy", "Copy").shortcut("Ctrl+C"))
            .action(AccessibilityAction::new("cut", "Cut").shortcut("Ctrl+X"))
            .action(AccessibilityAction::new("paste", "Paste").shortcut("Ctrl+V"));
        let hint = options
            .accessibility_hint
            .clone()
            .or_else(|| (!options.placeholder.is_empty()).then(|| options.placeholder.clone()));
        if let Some(hint) = hint {
            accessibility = accessibility.hint(hint);
        }
        if options.enabled {
            accessibility = accessibility.focusable();
        } else {
            accessibility = accessibility.disabled();
        }
        let visual = if !options.enabled {
            options.disabled_visual.unwrap_or(options.visual)
        } else if options.focused {
            options.focused_visual.unwrap_or(options.visual)
        } else {
            options.visual
        };
        let mut root_node = UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(if options.enabled {
            InputBehavior::BUTTON
        } else {
            InputBehavior::NONE
        })
        .with_visual(visual)
        .with_accessibility(accessibility);
        if let Some(shader) = options.shader {
            root_node = root_node.with_shader(shader);
        }
        if let Some(animation) = options.animation {
            root_node = root_node.with_animation(animation);
        }
        let root = document.add_child(parent, root_node);
        let display_text = if state.text.is_empty() {
            options.placeholder
        } else {
            state.text.clone()
        };
        let style = if state.text.is_empty() {
            options.placeholder_style
        } else {
            options.text_style
        };
        label(
            document,
            root,
            format!("{name}.text"),
            display_text,
            style,
            Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
        );
        root
    }

    fn filter_text_input(text: &str, multiline: bool) -> String {
        if multiline {
            let mut filtered = String::with_capacity(text.len());
            let mut chars = text.chars().peekable();
            while let Some(character) = chars.next() {
                if character == '\r' {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    filtered.push('\n');
                } else {
                    filtered.push(character);
                }
            }
            return filtered;
        }

        let mut filtered = String::with_capacity(text.len());
        let mut in_line_break = false;
        for character in text.chars() {
            if character == '\r' || character == '\n' {
                if !in_line_break {
                    filtered.push(' ');
                    in_line_break = true;
                }
            } else {
                filtered.push(character);
                in_line_break = false;
            }
        }
        filtered
    }

    fn previous_char_boundary(text: &str, index: usize) -> usize {
        text[..index]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn next_char_boundary(text: &str, index: usize) -> usize {
        text[index..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| index + offset)
            .unwrap_or(text.len())
    }

    fn clamp_to_char_boundary(text: &str, mut index: usize) -> usize {
        index = index.min(text.len());
        while index > 0 && !text.is_char_boundary(index) {
            index -= 1;
        }
        index
    }

    #[derive(Debug, Clone)]
    pub struct ComboBoxOptions {
        pub layout: Style,
        pub visual: UiVisual,
        pub open_visual: Option<UiVisual>,
        pub disabled_visual: Option<UiVisual>,
        pub text_style: TextStyle,
        pub leading_image: Option<ImageContent>,
        pub image_size: UiSize,
        pub shader: Option<ShaderEffect>,
        pub animation: Option<AnimationMachine>,
        pub enabled: bool,
        pub accessibility_label: Option<String>,
        pub accessibility_hint: Option<String>,
    }

    impl Default for ComboBoxOptions {
        fn default() -> Self {
            Self {
                layout: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: length(180.0),
                        height: length(30.0),
                    },
                    padding: taffy::prelude::Rect::length(6.0),
                    ..Default::default()
                },
                visual: UiVisual::panel(
                    ColorRgba::new(31, 37, 46, 255),
                    Some(StrokeStyle::new(ColorRgba::new(84, 98, 121, 255), 1.0)),
                    4.0,
                ),
                open_visual: Some(UiVisual::panel(
                    ColorRgba::new(38, 48, 62, 255),
                    Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.5)),
                    4.0,
                )),
                disabled_visual: Some(UiVisual::panel(
                    ColorRgba::new(29, 33, 40, 170),
                    Some(StrokeStyle::new(ColorRgba::new(65, 73, 87, 170), 1.0)),
                    4.0,
                )),
                text_style: TextStyle::default(),
                leading_image: None,
                image_size: UiSize::new(18.0, 18.0),
                shader: None,
                animation: None,
                enabled: true,
                accessibility_label: None,
                accessibility_hint: None,
            }
        }
    }

    pub fn combo_box(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        selected_label: impl Into<String>,
        open: bool,
        options: ComboBoxOptions,
    ) -> UiNodeId {
        let name = name.into();
        let selected_label = selected_label.into();
        let accessibility_label = options
            .accessibility_label
            .clone()
            .unwrap_or_else(|| name.clone());
        let accessibility_hint = options.accessibility_hint.clone();
        let root = button(
            document,
            parent,
            name.clone(),
            selected_label.clone(),
            ButtonOptions {
                layout: options.layout,
                visual: options.visual,
                pressed_visual: options.open_visual,
                focused_visual: None,
                disabled_visual: options.disabled_visual,
                text_style: options.text_style,
                leading_image: options.leading_image,
                image_size: options.image_size,
                image_shader: None,
                shader: options.shader,
                animation: options.animation,
                enabled: options.enabled,
                pressed: open,
                focused: false,
                accessibility_label: Some(accessibility_label.clone()),
                accessibility_hint: accessibility_hint.clone(),
            },
        );
        let mut accessibility = AccessibilityMeta::new(AccessibilityRole::ComboBox)
            .label(accessibility_label)
            .value(if open {
                format!("{selected_label} (open)")
            } else {
                selected_label
            })
            .expanded(open)
            .action(if open {
                AccessibilityAction::new("close", "Close")
            } else {
                AccessibilityAction::new("open", "Open")
            });
        if let Some(hint) = accessibility_hint {
            accessibility = accessibility.hint(hint);
        }
        if options.enabled {
            accessibility = accessibility.focusable();
        } else {
            accessibility = accessibility.disabled();
        }
        document.node_mut(root).accessibility = Some(accessibility);
        if open {
            document.node_mut(root).style.z_index = 20;
        }
        root
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct VirtualListSpec {
        pub row_count: usize,
        pub row_height: f32,
        pub viewport_height: f32,
        pub scroll_offset: f32,
        pub overscan: usize,
    }

    impl VirtualListSpec {
        pub fn visible_range(self) -> Range<usize> {
            if self.row_count == 0 || self.row_height <= f32::EPSILON {
                return 0..0;
            }
            let first = (self.scroll_offset.max(0.0) / self.row_height).floor() as usize;
            let visible = (self.viewport_height / self.row_height).ceil() as usize + 1;
            let start = first.saturating_sub(self.overscan).min(self.row_count);
            let end = (first + visible + self.overscan).min(self.row_count);
            start..end
        }

        pub fn total_height(self) -> f32 {
            self.row_count as f32 * self.row_height
        }
    }

    pub fn virtual_list(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        spec: VirtualListSpec,
        mut build_row: impl FnMut(&mut UiDocument, UiNodeId, usize),
    ) -> UiNodeId {
        let name = name.into();
        let list = scroll_area(
            document,
            parent,
            name.clone(),
            ScrollAxes::VERTICAL,
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(spec.viewport_height),
                },
                ..Default::default()
            },
        );
        document.node_mut(list).accessibility = Some(
            AccessibilityMeta::new(AccessibilityRole::List)
                .label(name.clone())
                .value(format!("{} items", spec.row_count)),
        );
        if let Some(scroll) = &mut document.nodes[list.0].scroll {
            scroll.offset.y = spec.scroll_offset.max(0.0);
        }
        let range = spec.visible_range();
        let top = range.start as f32 * spec.row_height;
        if top > 0.0 {
            document.add_child(list, spacer(format!("{name}.top_spacer"), top));
        }
        for row in range.clone() {
            build_row(document, list, row);
        }
        let bottom = (spec.row_count.saturating_sub(range.end)) as f32 * spec.row_height;
        if bottom > 0.0 {
            document.add_child(list, spacer(format!("{name}.bottom_spacer"), bottom));
        }
        list
    }

    fn spacer(name: impl Into<String>, height: f32) -> UiNode {
        UiNode::container(
            name,
            UiNodeStyle {
                layout: Style {
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(height),
                    },
                    flex_shrink: 0.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TableColumn {
        pub id: String,
        pub label: String,
        pub width: f32,
    }

    pub fn table_header(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        columns: &[TableColumn],
    ) -> UiNodeId {
        let name = name.into();
        let row = document.add_child(
            parent,
            UiNode::container(
                name.clone(),
                UiNodeStyle {
                    layout: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: length(28.0),
                        },
                        ..Default::default()
                    },
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(34, 41, 50, 255),
                Some(StrokeStyle::new(ColorRgba::new(67, 78, 95, 255), 1.0)),
                0.0,
            ))
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Grid)
                    .label(name.clone())
                    .value(format!("{} columns", columns.len())),
            ),
        );
        for column in columns {
            let cell = label(
                document,
                row,
                format!("{name}.{}", column.id),
                &column.label,
                TextStyle::default(),
                Style {
                    size: TaffySize {
                        width: length(column.width),
                        height: Dimension::percent(1.0),
                    },
                    padding: taffy::prelude::Rect::length(4.0),
                    ..Default::default()
                },
            );
            document.node_mut(cell).accessibility = Some(
                AccessibilityMeta::new(AccessibilityRole::GridCell)
                    .label(column.label.clone())
                    .value(column.id.clone()),
            );
        }
        row
    }

    pub fn scrollbar_thumb(scroll: ScrollState, track: UiRect, axis: ScrollAxis) -> UiRect {
        match axis {
            ScrollAxis::Vertical => {
                if track.height <= f32::EPSILON || track.width <= f32::EPSILON {
                    return UiRect::new(track.x, track.y, 0.0, 0.0);
                }
                let ratio = scrollbar_viewport_ratio(
                    scroll.viewport_size.height,
                    scroll.content_size.height,
                );
                let height = track.height * ratio;
                let max_offset = scroll.max_offset().y;
                let offset_ratio = if max_offset <= f32::EPSILON {
                    0.0
                } else {
                    (scroll.offset.y / max_offset).clamp(0.0, 1.0)
                };
                let y = track.y + (track.height - height) * offset_ratio;
                UiRect::new(track.x, y, track.width, height)
            }
            ScrollAxis::Horizontal => {
                if track.width <= f32::EPSILON || track.height <= f32::EPSILON {
                    return UiRect::new(track.x, track.y, 0.0, 0.0);
                }
                let ratio =
                    scrollbar_viewport_ratio(scroll.viewport_size.width, scroll.content_size.width);
                let width = track.width * ratio;
                let max_offset = scroll.max_offset().x;
                let offset_ratio = if max_offset <= f32::EPSILON {
                    0.0
                } else {
                    (scroll.offset.x / max_offset).clamp(0.0, 1.0)
                };
                let x = track.x + (track.width - width) * offset_ratio;
                UiRect::new(x, track.y, width, track.height)
            }
        }
    }

    pub fn scrollbar_accessibility(
        label: impl Into<String>,
        scroll: ScrollState,
        axis: ScrollAxis,
    ) -> AccessibilityMeta {
        let (offset, max_offset) = match axis {
            ScrollAxis::Vertical => (scroll.offset.y, scroll.max_offset().y),
            ScrollAxis::Horizontal => (scroll.offset.x, scroll.max_offset().x),
        };
        let percent = if max_offset <= f32::EPSILON {
            100.0
        } else {
            (offset / max_offset * 100.0).clamp(0.0, 100.0)
        };
        let accessibility = AccessibilityMeta::new(AccessibilityRole::Slider)
            .label(label)
            .value(format!("{percent:.0}%"))
            .value_range(AccessibilityValueRange::new(
                0.0,
                max_offset.max(0.0) as f64,
            ))
            .action(AccessibilityAction::new(
                "scroll_backward",
                "Scroll backward",
            ))
            .action(AccessibilityAction::new("scroll_forward", "Scroll forward"));
        if max_offset <= f32::EPSILON {
            accessibility.disabled()
        } else {
            accessibility.focusable()
        }
    }

    fn scrollbar_viewport_ratio(viewport: f32, content: f32) -> f32 {
        if viewport <= f32::EPSILON || content <= viewport {
            1.0
        } else {
            (viewport / content).clamp(0.05, 1.0)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ScrollAxis {
        Vertical,
        Horizontal,
    }
}

fn available_space_to_option(value: AvailableSpace) -> Option<f32> {
    match value {
        AvailableSpace::Definite(value) => Some(value),
        AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
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
        layout: Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: TaffySize {
                width: Dimension::length(width),
                height: Dimension::length(height),
            },
            ..Default::default()
        },
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

pub fn length(value: f32) -> Dimension {
    Dimension::length(value)
}

pub mod layout {
    use taffy::prelude::{LengthPercentageAuto, Rect};

    use super::*;

    pub fn px(value: f32) -> Dimension {
        Dimension::length(value)
    }

    pub fn percent(value: f32) -> Dimension {
        Dimension::percent(value)
    }

    pub fn fixed(width: f32, height: f32) -> Style {
        Style {
            size: TaffySize {
                width: px(width),
                height: px(height),
            },
            ..Default::default()
        }
    }

    pub fn size(width: Dimension, height: Dimension) -> Style {
        Style {
            size: TaffySize { width, height },
            ..Default::default()
        }
    }

    pub fn row() -> Style {
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            ..Default::default()
        }
    }

    pub fn column() -> Style {
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    pub fn fill() -> Style {
        size(percent(1.0), percent(1.0))
    }

    pub fn with_size(mut style: Style, width: Dimension, height: Dimension) -> Style {
        style.size = TaffySize { width, height };
        style
    }

    pub fn with_margin_all(mut style: Style, value: f32) -> Style {
        style.margin = Rect::length(value);
        style
    }

    pub fn with_padding_all(mut style: Style, value: f32) -> Style {
        style.padding = Rect::length(value);
        style
    }

    pub fn with_auto_horizontal_margin(mut style: Style) -> Style {
        style.margin.left = LengthPercentageAuto::auto();
        style.margin.right = LengthPercentageAuto::auto();
        style
    }
}

#[cfg(feature = "egui")]
pub fn egui_rect(rect: UiRect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::Pos2::new(rect.x, rect.y),
        egui::Vec2::new(rect.width, rect.height),
    )
}

#[cfg(feature = "egui")]
pub fn egui_color(color: ColorRgba, opacity: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        color.r,
        color.g,
        color.b,
        ((color.a as f32) * opacity.clamp(0.0, 1.0)).round() as u8,
    )
}

#[cfg(feature = "egui")]
pub fn paint_document_egui(document: &UiDocument, ctx: &egui::Context, layer: egui::LayerId) {
    paint_document_egui_impl(document, ctx, layer, None, None);
}

#[cfg(feature = "egui")]
pub fn paint_document_egui_clipped(
    document: &UiDocument,
    ctx: &egui::Context,
    layer: egui::LayerId,
    clip_rect: UiRect,
) {
    paint_document_egui_impl(document, ctx, layer, Some(clip_rect), None);
}

#[cfg(feature = "egui")]
pub fn paint_document_egui_with_canvas(
    document: &UiDocument,
    ctx: &egui::Context,
    layer: egui::LayerId,
    mut paint_canvas: impl FnMut(&CanvasContent, &PaintItem, &egui::Painter),
) {
    paint_document_egui_impl(document, ctx, layer, None, Some(&mut paint_canvas));
}

#[cfg(feature = "egui")]
type EguiCanvasCallback<'a> = dyn FnMut(&CanvasContent, &PaintItem, &egui::Painter) + 'a;

#[cfg(feature = "egui")]
fn paint_document_egui_impl(
    document: &UiDocument,
    ctx: &egui::Context,
    layer: egui::LayerId,
    outer_clip: Option<UiRect>,
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
            PaintKind::Text(text) => {
                simple_rect_batch.flush(&painter, outer_clip);
                painter.with_clip_rect(clip_rect).text(
                    egui::Pos2::new(rect.min.x, rect.min.y),
                    egui::Align2::LEFT_TOP,
                    &text.text,
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
            PaintKind::Image { .. } => {
                simple_rect_batch.flush(&painter, outer_clip);
            }
        }
    }
    simple_rect_batch.flush(&painter, outer_clip);
}

#[cfg(feature = "egui")]
fn egui_pos(point: UiPoint) -> egui::Pos2 {
    egui::Pos2::new(point.x, point.y)
}

#[cfg(feature = "egui")]
fn transform_point(point: UiPoint, transform: PaintTransform) -> UiPoint {
    UiPoint::new(
        point.x * transform.scale + transform.translation.x,
        point.y * transform.scale + transform.translation.y,
    )
}

#[cfg(feature = "egui")]
fn transform_rect(rect: UiRect, transform: PaintTransform) -> UiRect {
    let top_left = transform_point(UiPoint::new(rect.x, rect.y), transform);
    UiRect::new(
        top_left.x,
        top_left.y,
        rect.width * transform.scale,
        rect.height * transform.scale,
    )
}

#[cfg(feature = "egui")]
fn egui_font_id(style: &TextStyle, scale: f32) -> egui::FontId {
    let size = style.font_size * scale.max(0.0);
    match style.family {
        FontFamily::Monospace => egui::FontId::monospace(size),
        FontFamily::SansSerif | FontFamily::Serif | FontFamily::Named(_) => {
            egui::FontId::proportional(size)
        }
    }
}

#[cfg(feature = "egui")]
#[derive(Default)]
struct SimpleRectBatch {
    mesh: egui::epaint::Mesh,
}

#[cfg(feature = "egui")]
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

#[cfg(feature = "egui")]
fn rect_is_inside_clip(rect: egui::Rect, clip_rect: egui::Rect) -> bool {
    rect.min.x >= clip_rect.min.x
        && rect.min.y >= clip_rect.min.y
        && rect.max.x <= clip_rect.max.x
        && rect.max.y <= clip_rect.max.y
}

#[cfg(feature = "egui")]
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

    use super::*;

    fn button_style(width: f32, height: f32) -> UiNodeStyle {
        UiNodeStyle {
            layout: Style {
                size: TaffySize {
                    width: length(width),
                    height: length(height),
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn taffy_layout_places_bottom_centered_hotbar() {
        let mut doc = UiDocument::new(root_style(800.0, 600.0));
        let hotbar = doc.add_child(
            doc.root,
            UiNode::container(
                "hotbar",
                UiNodeStyle {
                    layout: Style {
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
                    },
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
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            ),
        );
        doc.compute_layout(UiSize::new(300.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let rect = doc.node(text).layout.rect;
        assert!(rect.width > 0.0);
        assert!(rect.height > 0.0);
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
                    layout: Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(100.0),
                        },
                        ..Default::default()
                    },
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
                    layout: Style {
                        size: TaffySize {
                            width: length(160.0),
                            height: length(80.0),
                        },
                        ..Default::default()
                    },
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
    fn hit_testing_uses_effective_paint_z_order() {
        let mut doc = UiDocument::new(root_style(240.0, 160.0));
        let under = doc.add_child(
            doc.root,
            UiNode::container(
                "under",
                UiNodeStyle {
                    layout: Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(100.0),
                        },
                        ..Default::default()
                    },
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
                    layout: Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(100.0),
                        },
                        margin: Rect {
                            top: LengthPercentageAuto::length(-100.0),
                            ..Rect::length(0.0)
                        },
                        ..Default::default()
                    },
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
    fn scroll_area_tracks_content_size_and_offsets_children() {
        let mut doc = UiDocument::new(root_style(120.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "events",
                UiNodeStyle {
                    layout: Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(60.0),
                        },
                        ..Default::default()
                    },
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

        let input = doc.handle_input(UiInputEvent::Wheel {
            position: UiPoint::new(10.0, 10.0),
            delta: UiPoint::new(0.0, 30.0),
        });
        assert_eq!(input.scrolled, Some(scroll_area));

        doc.compute_layout(UiSize::new(120.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");
        assert_eq!(doc.node(row).layout.rect.y, -30.0);
        assert_eq!(doc.hit_test(UiPoint::new(10.0, 90.0)), None);
    }

    #[test]
    fn wheel_scrolls_blank_space_inside_scroll_region() {
        let mut doc = UiDocument::new(root_style(120.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(60.0),
                        },
                        ..Default::default()
                    },
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

        let input = doc.handle_input(UiInputEvent::Wheel {
            position: UiPoint::new(90.0, 50.0),
            delta: UiPoint::new(0.0, 30.0),
        });

        assert_eq!(input.scrolled, Some(scroll_area));
        assert_eq!(doc.scroll_state(scroll_area).unwrap().offset.y, 30.0);
    }

    #[test]
    fn scroll_content_bounds_include_nested_descendants() {
        let mut doc = UiDocument::new(root_style(160.0, 120.0));
        let scroll_area = doc.add_child(
            doc.root,
            UiNode::container(
                "scroll",
                UiNodeStyle {
                    layout: Style {
                        size: TaffySize {
                            width: length(100.0),
                            height: length(60.0),
                        },
                        ..Default::default()
                    },
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
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            ),
        );
        let _canvas = doc.add_child(
            doc.root,
            UiNode::canvas(
                "piano_roll",
                "orbifold.piano_roll",
                Style {
                    size: TaffySize {
                        width: length(100.0),
                        height: length(50.0),
                    },
                    ..Default::default()
                },
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
                Style {
                    size: TaffySize {
                        width: length(80.0),
                        height: length(60.0),
                    },
                    ..Default::default()
                },
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
    fn paint_list_exposes_image_and_shader_metadata() {
        let mut doc = UiDocument::new(root_style(120.0, 80.0));
        let image = doc.add_child(
            doc.root,
            UiNode::image(
                "icon",
                ImageContent::new("icons.play").tinted(ColorRgba::new(120, 180, 255, 255)),
                Style {
                    size: TaffySize {
                        width: length(24.0),
                        height: length(24.0),
                    },
                    ..Default::default()
                },
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
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            )
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label("Play")),
        );
        let hint = doc.add_child(
            doc.root,
            UiNode::text(
                "play.hint",
                "Starts transport",
                TextStyle::default(),
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
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
                    layout: Style {
                        align_items: Some(AlignItems::Center),
                        justify_content: Some(JustifyContent::Center),
                        position: Position::Relative,
                        size: TaffySize {
                            width: length(120.0),
                            height: length(80.0),
                        },
                        ..Default::default()
                    },
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

    #[cfg(feature = "widgets")]
    #[test]
    fn widget_button_builds_focusable_document_nodes() {
        let mut doc = UiDocument::new(root_style(200.0, 80.0));
        let root = doc.root;
        let button = widgets::button(
            &mut doc,
            root,
            "play",
            "Play",
            widgets::ButtonOptions::new(Style {
                size: TaffySize {
                    width: length(80.0),
                    height: length(32.0),
                },
                ..Default::default()
            }),
        );
        doc.compute_layout(UiSize::new(200.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert!(doc.node(button).input.focusable);
        assert_eq!(doc.node(button).children.len(), 1);
        assert!(doc
            .paint_list()
            .items
            .iter()
            .any(|item| item.node == button));
    }

    #[cfg(feature = "widgets")]
    #[test]
    fn widget_button_options_apply_disabled_accessibility_and_media_hooks() {
        let mut doc = UiDocument::new(root_style(200.0, 80.0));
        let root = doc.root;
        let disabled_visual = UiVisual::panel(ColorRgba::new(10, 11, 12, 180), None, 2.0);
        let button = widgets::button(
            &mut doc,
            root,
            "render",
            "Render",
            widgets::ButtonOptions {
                layout: Style {
                    size: TaffySize {
                        width: length(96.0),
                        height: length(32.0),
                    },
                    ..Default::default()
                },
                leading_image: Some(ImageContent::new("icons.render")),
                image_shader: Some(ShaderEffect::new("ui.icon_mask")),
                shader: Some(ShaderEffect::new("ui.disabled")),
                disabled_visual: Some(disabled_visual),
                enabled: false,
                accessibility_hint: Some("Unavailable while exporting".to_string()),
                ..Default::default()
            },
        );

        assert_eq!(doc.node(button).visual, disabled_visual);
        assert_eq!(doc.node(button).shader.as_ref().unwrap().key, "ui.disabled");
        assert!(!doc.node(button).input.pointer);
        assert!(!doc.node(button).input.focusable);

        let accessibility = doc.node(button).accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::Button);
        assert_eq!(accessibility.label.as_deref(), Some("Render"));
        assert_eq!(
            accessibility.hint.as_deref(),
            Some("Unavailable while exporting")
        );
        assert!(!accessibility.enabled);
        assert!(!accessibility.focusable);

        let image = doc.node(button).children[0];
        assert!(matches!(doc.node(image).content, UiContent::Image(_)));
        assert_eq!(doc.node(image).shader.as_ref().unwrap().key, "ui.icon_mask");
    }

    #[cfg(feature = "widgets")]
    #[test]
    fn widget_core_controls_export_accessibility_metadata() {
        let mut doc = UiDocument::new(root_style(360.0, 240.0));
        let root = doc.root;
        let title = widgets::label(
            &mut doc,
            root,
            "title",
            "Oscillator",
            TextStyle::default(),
            Style {
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
        );
        let scroll = widgets::scroll_area(
            &mut doc,
            root,
            "modulation_matrix",
            ScrollAxes::BOTH,
            Style {
                size: TaffySize {
                    width: length(160.0),
                    height: length(60.0),
                },
                ..Default::default()
            },
        );
        let checkbox = widgets::checkbox(
            &mut doc,
            root,
            "sync",
            "Hard sync",
            true,
            widgets::CheckboxOptions::default(),
        );
        let slider = widgets::slider(
            &mut doc,
            root,
            "volume",
            0.25,
            0.0..1.0,
            widgets::SliderOptions {
                accessibility_label: Some("Volume".to_string()),
                ..Default::default()
            },
        );
        let input_state = widgets::TextInputState::new("");
        let input = widgets::text_input(
            &mut doc,
            root,
            "preset_name",
            &input_state,
            widgets::TextInputOptions {
                placeholder: "Preset name".to_string(),
                ..Default::default()
            },
        );
        let combo = widgets::combo_box(
            &mut doc,
            root,
            "waveform",
            "Sine",
            true,
            widgets::ComboBoxOptions::default(),
        );

        let tree = doc.accessibility_tree();
        let node = |id| tree.iter().find(|node| node.id == id).unwrap();

        assert_eq!(node(title).role, AccessibilityRole::Label);
        assert_eq!(node(title).label.as_deref(), Some("Oscillator"));
        assert_eq!(node(scroll).role, AccessibilityRole::List);
        assert_eq!(
            node(scroll).value.as_deref(),
            Some("horizontal and vertical")
        );
        assert_eq!(node(checkbox).role, AccessibilityRole::Checkbox);
        assert_eq!(node(checkbox).value.as_deref(), Some("checked"));
        assert_eq!(node(slider).role, AccessibilityRole::Slider);
        assert_eq!(node(slider).label.as_deref(), Some("Volume"));
        assert_eq!(node(slider).value.as_deref(), Some("0.25 (25%)"));
        assert_eq!(node(input).role, AccessibilityRole::TextBox);
        assert_eq!(node(input).hint.as_deref(), Some("Preset name"));
        assert_eq!(node(combo).role, AccessibilityRole::ComboBox);
        assert_eq!(node(combo).value.as_deref(), Some("Sine (open)"));
    }

    #[cfg(feature = "widgets")]
    #[test]
    fn widget_text_input_edits_and_commits_state() {
        let mut state = widgets::TextInputState::new("gain");
        state.move_caret(widgets::CaretMovement::End, false);
        let outcome = state.handle_event(&UiInputEvent::TextInput("!".to_string()));
        assert!(outcome.changed);
        assert_eq!(state.text, "gain!");
        let outcome = state.handle_event(&UiInputEvent::Key {
            key: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
        });
        assert!(outcome.committed);
    }

    #[cfg(feature = "widgets")]
    #[test]
    fn widget_text_input_supports_clipboard_edit_primitives() {
        let mut state = widgets::TextInputState::new("wet dry");
        state.move_caret(widgets::CaretMovement::Start, false);
        state.move_caret(widgets::CaretMovement::Right, true);
        state.move_caret(widgets::CaretMovement::Right, true);
        state.move_caret(widgets::CaretMovement::Right, true);

        assert_eq!(state.copy_selection().as_deref(), Some("wet"));
        assert_eq!(state.cut_selection().as_deref(), Some("wet"));
        assert_eq!(state.text, " dry");
        state.paste_text("very\nwet");
        assert_eq!(state.text, "very wet dry");
    }

    #[cfg(feature = "widgets")]
    #[test]
    fn widget_text_input_reports_clipboard_key_commands_and_sanitizes_paste() {
        let mut state = widgets::TextInputState::new("café");
        state.caret = 4;
        state.selection_anchor = Some(0);
        assert_eq!(state.copy_selection().as_deref(), Some("caf"));

        state.select_all();
        let copy = state.handle_event(&UiInputEvent::Key {
            key: KeyCode::Character('c'),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
        });
        assert_eq!(
            copy.clipboard,
            Some(widgets::TextInputClipboardAction::Copy("café".to_string()))
        );
        assert!(!copy.changed);

        let cut = state.handle_event(&UiInputEvent::Key {
            key: KeyCode::Character('x'),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
        });
        assert_eq!(
            cut.clipboard,
            Some(widgets::TextInputClipboardAction::Cut("café".to_string()))
        );
        assert!(cut.changed);
        assert_eq!(state.text, "");

        let paste_request = state.handle_event(&UiInputEvent::Key {
            key: KeyCode::Character('v'),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
        });
        assert_eq!(
            paste_request.clipboard,
            Some(widgets::TextInputClipboardAction::Paste)
        );
        assert!(!paste_request.changed);

        let paste = state.paste_text_with_outcome("dry\r\nwet\n");
        assert!(paste.changed);
        assert_eq!(state.text, "dry wet ");

        let mut multiline = widgets::TextInputState::new("").multiline(true);
        multiline.paste_text("a\r\nb\rc");
        assert_eq!(multiline.text, "a\nb\nc");
    }

    #[cfg(feature = "widgets")]
    #[test]
    fn virtual_list_builds_only_visible_rows_with_spacers() {
        let mut doc = UiDocument::new(root_style(300.0, 200.0));
        let root = doc.root;
        let list = widgets::virtual_list(
            &mut doc,
            root,
            "events",
            widgets::VirtualListSpec {
                row_count: 100,
                row_height: 20.0,
                viewport_height: 60.0,
                scroll_offset: 200.0,
                overscan: 1,
            },
            |document, parent, row| {
                document.add_child(
                    parent,
                    UiNode::text(
                        format!("row.{row}"),
                        format!("Event {row}"),
                        TextStyle::default(),
                        Style {
                            size: TaffySize {
                                width: Dimension::percent(1.0),
                                height: length(20.0),
                            },
                            ..Default::default()
                        },
                    )
                    .with_input(InputBehavior::BUTTON),
                );
            },
        );
        doc.compute_layout(UiSize::new(300.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(doc.node(list).children.len(), 8);
        assert_eq!(doc.scroll_state(list).unwrap().content_size.height, 2000.0);
    }

    #[cfg(feature = "widgets")]
    #[test]
    fn widget_table_virtual_list_and_scrollbar_helpers_expose_metadata() {
        let mut doc = UiDocument::new(root_style(300.0, 200.0));
        let root = doc.root;
        let header = widgets::table_header(
            &mut doc,
            root,
            "events.header",
            &[
                widgets::TableColumn {
                    id: "time".to_string(),
                    label: "Time".to_string(),
                    width: 80.0,
                },
                widgets::TableColumn {
                    id: "name".to_string(),
                    label: "Name".to_string(),
                    width: 160.0,
                },
            ],
        );
        let list = widgets::virtual_list(
            &mut doc,
            root,
            "events",
            widgets::VirtualListSpec {
                row_count: 25,
                row_height: 20.0,
                viewport_height: 60.0,
                scroll_offset: 40.0,
                overscan: 0,
            },
            |document, parent, row| {
                document.add_child(
                    parent,
                    UiNode::text(
                        format!("row.{row}"),
                        format!("Event {row}"),
                        TextStyle::default(),
                        Style {
                            size: TaffySize {
                                width: Dimension::percent(1.0),
                                height: length(20.0),
                            },
                            ..Default::default()
                        },
                    ),
                );
            },
        );

        let tree = doc.accessibility_tree();
        let header_node = tree.iter().find(|node| node.id == header).unwrap();
        let list_node = tree.iter().find(|node| node.id == list).unwrap();
        assert_eq!(header_node.role, AccessibilityRole::Grid);
        assert_eq!(header_node.value.as_deref(), Some("2 columns"));
        assert_eq!(list_node.role, AccessibilityRole::List);
        assert_eq!(list_node.value.as_deref(), Some("25 items"));
        assert!(tree.iter().any(|node| {
            node.role == AccessibilityRole::GridCell && node.label.as_deref() == Some("Time")
        }));

        let scroll = ScrollState {
            axes: ScrollAxes::VERTICAL,
            offset: UiPoint::new(0.0, 999.0),
            viewport_size: UiSize::new(10.0, 100.0),
            content_size: UiSize::new(10.0, 300.0),
        };
        let thumb = widgets::scrollbar_thumb(
            scroll,
            UiRect::new(0.0, 0.0, 10.0, 100.0),
            widgets::ScrollAxis::Vertical,
        );
        assert!((thumb.y - 66.66667).abs() < 0.01, "{thumb:?}");
        assert!((thumb.height - 33.33333).abs() < 0.01, "{thumb:?}");

        let accessibility = widgets::scrollbar_accessibility(
            "Events scrollbar",
            scroll,
            widgets::ScrollAxis::Vertical,
        );
        assert_eq!(accessibility.role, AccessibilityRole::Slider);
        assert_eq!(accessibility.value.as_deref(), Some("100%"));
        assert!(accessibility.focusable);

        let disabled_accessibility = widgets::scrollbar_accessibility(
            "Empty scrollbar",
            ScrollState {
                axes: ScrollAxes::VERTICAL,
                offset: UiPoint::new(0.0, 0.0),
                viewport_size: UiSize::new(10.0, 100.0),
                content_size: UiSize::new(10.0, 100.0),
            },
            widgets::ScrollAxis::Vertical,
        );
        assert!(!disabled_accessibility.enabled);
        assert!(!disabled_accessibility.focusable);
    }
}
