//! Operad is a small retained-mode UI document layer.
//!
//! The crate intentionally contains only reusable primitives: layout, text
//! measurement, hit testing, focus, animation, and optional renderer backends.
//! Product-specific screens and game/application state should live in the
//! consuming application crate.

use std::collections::HashMap;

#[cfg(feature = "text-cosmic")]
use cosmic_text::{
    Attrs, Buffer, Family as CosmicFamily, FontSystem, Metrics, Shaping, Stretch as CosmicStretch,
    Style as CosmicFontStyle, Weight as CosmicWeight, Wrap as CosmicWrap,
};
use taffy::prelude::{
    AvailableSpace, Dimension, Display, FlexDirection, NodeId as TaffyNodeId, Size as TaffySize,
    Style, TaffyTree,
};

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

#[derive(Debug, Clone, PartialEq)]
pub enum UiContent {
    Empty,
    Text(TextContent),
    Canvas(CanvasContent),
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
    Wheel { position: UiPoint, delta: UiPoint },
    Focus(FocusDirection),
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
    pub nodes: Vec<UiNode>,
    pub focus: UiFocusState,
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

    pub fn node_mut(&mut self, id: UiNodeId) -> &mut UiNode {
        &mut self.nodes[id.0]
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
                UiContent::Empty | UiContent::Canvas(_) => {
                    taffy.new_leaf(node.style.layout.clone())?
                }
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
        let scroll_offset = if let Some(scroll) = &mut self.nodes[id.0].scroll {
            scroll.offset = scroll.clamp_offset(scroll.offset);
            scroll.offset
        } else {
            UiPoint::new(0.0, 0.0)
        };
        let clip_rect = if has_scroll || self.nodes[id.0].style.clip == ClipBehavior::Clip {
            parent_clip
                .intersection(rect)
                .unwrap_or(UiRect::new(rect.x, rect.y, 0.0, 0.0))
        } else {
            parent_clip
        };
        let animation_opacity = self.nodes[id.0]
            .animation
            .as_ref()
            .map(|animation| animation.values().opacity)
            .unwrap_or(1.0);
        self.nodes[id.0].layout = ComputedLayout {
            rect,
            clip_rect,
            visible: rect.intersects(parent_clip),
            opacity: self.nodes[id.0].style.opacity * animation_opacity,
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
        let children = self.nodes[id.0].children.clone();
        if has_scroll {
            let mut content_size = UiSize::new(rect.width, rect.height);
            for child in children {
                let child_rect = self.nodes[child.0].layout.rect;
                content_size.width = content_size.width.max(child_rect.right() - child_origin.x);
                content_size.height = content_size
                    .height
                    .max(child_rect.bottom() - child_origin.y);
            }
            let scroll = self.nodes[id.0]
                .scroll
                .as_mut()
                .expect("scroll state exists when has_scroll is true");
            scroll.viewport_size = UiSize::new(rect.width, rect.height);
            scroll.content_size = content_size;
            scroll.offset = scroll.clamp_offset(scroll.offset);
        }
        Ok(())
    }

    pub fn hit_test(&self, point: UiPoint) -> Option<UiNodeId> {
        let mut best = None;
        let mut best_z = i16::MIN;
        for (index, node) in self.nodes.iter().enumerate() {
            if !node.input.pointer
                || !node.layout.visible
                || !node.layout.clip_rect.contains_point(point)
            {
                continue;
            }
            if node.style.z_index >= best_z && node.layout.rect.contains_point(point) {
                best = Some(UiNodeId(index));
                best_z = node.style.z_index;
            }
        }
        best
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
                if let Some(target) = self
                    .hit_test(position)
                    .and_then(|hit| self.scrollable_ancestor(hit))
                {
                    if self.scroll_by(target, delta) {
                        scrolled = Some(target);
                    }
                }
                None
            }
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

    fn scrollable_ancestor(&self, mut id: UiNodeId) -> Option<UiNodeId> {
        loop {
            let node = &self.nodes[id.0];
            if node
                .scroll
                .is_some_and(|scroll| scroll.axes.horizontal || scroll.axes.vertical)
            {
                return Some(id);
            }
            id = node.parent?;
        }
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
        let mut order = (0..self.nodes.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| (z_indexes[*index], *index));
        for index in order {
            let id = UiNodeId(index);
            let node = &self.nodes[index];
            if !node.layout.visible
                || node.layout.clip_rect.width <= f32::EPSILON
                || node.layout.clip_rect.height <= f32::EPSILON
            {
                continue;
            }
            let z_index = z_indexes[index];
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
                    opacity: node.layout.opacity,
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
                    opacity: node.layout.opacity,
                    kind: PaintKind::Text(text.clone()),
                }),
                UiContent::Canvas(canvas) => list.items.push(PaintItem {
                    node: id,
                    rect: node.layout.rect,
                    clip_rect: node.layout.clip_rect,
                    z_index,
                    opacity: node.layout.opacity,
                    kind: PaintKind::Canvas(canvas.clone()),
                }),
            }
        }
        list
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
    pub kind: PaintKind,
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
pub enum AuditWarning {
    NonFiniteRect { node: UiNodeId, name: String },
    InvisibleInteractiveNode { node: UiNodeId, name: String },
    EmptyInteractiveClip { node: UiNodeId, name: String },
}

impl UiDocument {
    pub fn layout_snapshot(&self) -> LayoutSnapshot {
        self.layout_snapshot_subtree(self.root)
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
        for (index, node) in self.nodes.iter().enumerate() {
            let id = UiNodeId(index);
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

fn rect_is_finite(rect: UiRect) -> bool {
    rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite()
}

#[cfg(feature = "widgets")]
pub mod widgets {
    use taffy::prelude::{AlignItems, JustifyContent};

    use super::*;

    #[derive(Debug, Clone)]
    pub struct ButtonOptions {
        pub layout: Style,
        pub visual: UiVisual,
        pub text_style: TextStyle,
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
                text_style: TextStyle::default(),
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
        let button = document.add_child(
            parent,
            UiNode::container(
                name.clone(),
                UiNodeStyle {
                    layout: options.layout,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON)
            .with_visual(options.visual),
        );
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
        document.add_child(parent, UiNode::text(name, text, style, layout))
    }

    pub fn scroll_area(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        axes: ScrollAxes,
        layout: Style,
    ) -> UiNodeId {
        document.add_child(
            parent,
            UiNode::container(
                name,
                UiNodeStyle {
                    layout,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(axes),
        )
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
    paint_document_egui_impl(document, ctx, layer, None);
}

#[cfg(feature = "egui")]
pub fn paint_document_egui_clipped(
    document: &UiDocument,
    ctx: &egui::Context,
    layer: egui::LayerId,
    clip_rect: UiRect,
) {
    paint_document_egui_impl(document, ctx, layer, Some(clip_rect));
}

#[cfg(feature = "egui")]
fn paint_document_egui_impl(
    document: &UiDocument,
    ctx: &egui::Context,
    layer: egui::LayerId,
    outer_clip: Option<UiRect>,
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
        let rect = egui_rect(item.rect);
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
                    egui::FontId::proportional(text.style.font_size),
                    egui_color(text.style.color, item.opacity),
                );
            }
            PaintKind::Canvas(_) => {
                simple_rect_batch.flush(&painter, outer_clip);
            }
        }
    }
    simple_rect_batch.flush(&painter, outer_clip);
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
        let mut text_style = TextStyle::default();
        text_style.family = FontFamily::Monospace;
        text_style.weight = FontWeight::BOLD;
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
}
