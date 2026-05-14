//! Color picker widget model and document builder.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, Rect as TaffyRect, Size as TaffySize, Style,
};

use crate::{
    length, AccessibilityMeta, AccessibilityRole, AccessibilityValueRange, ClipBehavior, ColorRgba,
    EditPhase, GradientStop, ImageContent, InputBehavior, LayoutStyle, LinearGradient, PaintBrush,
    PaintRect, ScenePrimitive, ShaderEffect, StrokeStyle, TextStyle, UiDocument, UiNode, UiNodeId,
    UiNodeStyle, UiPoint, UiRect, UiSize, UiVisual, WidgetActionBinding, WidgetActionKind,
    WidgetPointerEdit,
};

use super::pickers::{PickerAnimationMeta, PickerElementStyle};

const COLOR_FIELD_WIDTH: f32 = 204.0;
const COLOR_FIELD_HEIGHT: f32 = 112.0;
const CHANNEL_TRACK_WIDTH: f32 = 124.0;
const CHANNEL_TRACK_HEIGHT: f32 = 12.0;
const OKLCH_CHROMA_MAX: f32 = 0.4;
const OKLCH_FIELD_LIGHTNESS_STRIPS: usize = 112;
const OKLCH_FIELD_CHROMA_STOPS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorHsv {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub alpha: f32,
}

impl ColorHsv {
    pub fn new(hue: f32, saturation: f32, value: f32, alpha: f32) -> Self {
        Self {
            hue: normalize_hue(hue),
            saturation: unit(saturation),
            value: unit(value),
            alpha: unit(alpha),
        }
    }

    pub fn from_rgba(color: ColorRgba) -> Self {
        let r = color.r as f32 / 255.0;
        let g = color.g as f32 / 255.0;
        let b = color.b as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let hue = if delta <= f32::EPSILON {
            0.0
        } else if (max - r).abs() <= f32::EPSILON {
            60.0 * ((g - b) / delta).rem_euclid(6.0)
        } else if (max - g).abs() <= f32::EPSILON {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };

        let saturation = if max <= f32::EPSILON {
            0.0
        } else {
            delta / max
        };

        Self::new(hue, saturation, max, color.a as f32 / 255.0)
    }

    pub fn to_rgba(self) -> ColorRgba {
        let color = Self::new(self.hue, self.saturation, self.value, self.alpha);
        let chroma = color.value * color.saturation;
        let hue_sector = color.hue / 60.0;
        let x = chroma * (1.0 - (hue_sector.rem_euclid(2.0) - 1.0).abs());
        let (r1, g1, b1) = match hue_sector as u8 {
            0 => (chroma, x, 0.0),
            1 => (x, chroma, 0.0),
            2 => (0.0, chroma, x),
            3 => (0.0, x, chroma),
            4 => (x, 0.0, chroma),
            _ => (chroma, 0.0, x),
        };
        let m = color.value - chroma;

        ColorRgba::new(
            channel((r1 + m) * 255.0),
            channel((g1 + m) * 255.0),
            channel((b1 + m) * 255.0),
            channel(color.alpha * 255.0),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorOklch {
    pub lightness: f32,
    pub chroma: f32,
    pub hue: f32,
    pub alpha: f32,
}

impl ColorOklch {
    pub fn new(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Self {
        Self {
            lightness: unit(lightness),
            chroma: chroma_unit(chroma),
            hue: normalize_hue(hue),
            alpha: unit(alpha),
        }
    }

    pub fn from_rgba(color: ColorRgba) -> Self {
        let r = srgb_to_linear(color.r as f32 / 255.0);
        let g = srgb_to_linear(color.g as f32 / 255.0);
        let b = srgb_to_linear(color.b as f32 / 255.0);

        let l = (0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
        let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
        let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();

        let lightness = 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s;
        let a = 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s;
        let b = 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s;
        let chroma = (a * a + b * b).sqrt();
        let hue = if chroma <= 0.000_001 {
            0.0
        } else {
            b.atan2(a).to_degrees()
        };

        Self::new(lightness, chroma, hue, color.a as f32 / 255.0)
    }

    pub fn to_rgba(self) -> ColorRgba {
        let color = Self::new(self.lightness, self.chroma, self.hue, self.alpha);
        let hue_radians = color.hue.to_radians();
        let a = color.chroma * hue_radians.cos();
        let b = color.chroma * hue_radians.sin();

        let l = color.lightness + 0.396_337_78 * a + 0.215_803_76 * b;
        let m = color.lightness - 0.105_561_346 * a - 0.063_854_17 * b;
        let s = color.lightness - 0.089_484_18 * a - 1.291_485_5 * b;

        let l3 = l * l * l;
        let m3 = m * m * m;
        let s3 = s * s * s;

        let r = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
        let g = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
        let b = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;

        ColorRgba::new(
            channel(linear_to_srgb(r) * 255.0),
            channel(linear_to_srgb(g) * 255.0),
            channel(linear_to_srgb(b) * 255.0),
            channel(color.alpha * 255.0),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPickerMode {
    Hsv,
    Oklch,
}

impl ColorPickerMode {
    pub const ALL: [Self; 2] = [Self::Hsv, Self::Oklch];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Hsv => "HSV",
            Self::Oklch => "OKLCH",
        }
    }

    fn from_action_suffix(suffix: &str) -> Option<Self> {
        Some(match suffix {
            "hsv" => Self::Hsv,
            "oklch" => Self::Oklch,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorSwatch {
    pub id: String,
    pub label: String,
    pub color: ColorRgba,
    pub image: Option<ImageContent>,
    pub shader: Option<ShaderEffect>,
    pub animation: Option<PickerAnimationMeta>,
}

impl ColorSwatch {
    pub fn new(id: impl Into<String>, label: impl Into<String>, color: ColorRgba) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color,
            image: None,
            shader: None,
            animation: None,
        }
    }

    pub fn with_image(mut self, image: ImageContent) -> Self {
        self.image = Some(image);
        self
    }

    pub fn with_shader(mut self, shader: ShaderEffect) -> Self {
        self.shader = Some(shader);
        self
    }

    pub fn with_animation(mut self, animation: PickerAnimationMeta) -> Self {
        self.animation = Some(animation);
        self
    }

    pub fn accessibility_meta(&self, selected: bool) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(AccessibilityRole::Button)
            .label(self.label.clone())
            .value(format_hex_color(self.color, self.color.a < 255))
            .selected(selected)
            .focusable();
        if selected {
            meta = meta.hint("selected");
        }
        meta
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColorPalette {
    pub swatches: Vec<ColorSwatch>,
}

impl ColorPalette {
    pub fn new(swatches: impl IntoIterator<Item = ColorSwatch>) -> Self {
        Self {
            swatches: swatches.into_iter().collect(),
        }
    }

    pub fn find(&self, id: &str) -> Option<&ColorSwatch> {
        self.swatches.iter().find(|swatch| swatch.id == id)
    }

    pub fn push(&mut self, swatch: ColorSwatch) {
        self.swatches.push(swatch);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorPickerState {
    pub value: ColorRgba,
    pub hsv: ColorHsv,
    pub oklch: ColorOklch,
    pub mode: ColorPickerMode,
    pub palette: ColorPalette,
    pub recent: Vec<ColorRgba>,
    pub max_recent: usize,
}

impl ColorPickerState {
    pub fn new(value: ColorRgba) -> Self {
        Self {
            value,
            hsv: ColorHsv::from_rgba(value),
            oklch: ColorOklch::from_rgba(value),
            mode: ColorPickerMode::Hsv,
            palette: ColorPalette::default(),
            recent: Vec::new(),
            max_recent: 8,
        }
    }

    pub fn with_palette(mut self, palette: ColorPalette) -> Self {
        self.palette = palette;
        self
    }

    pub fn with_recent(mut self, recent: impl IntoIterator<Item = ColorRgba>) -> Self {
        let colors: Vec<_> = recent.into_iter().collect();
        for color in colors.into_iter().rev() {
            self.remember_recent(color);
        }
        self
    }

    pub fn hsv(&self) -> ColorHsv {
        self.hsv
    }

    pub fn oklch(&self) -> ColorOklch {
        self.oklch
    }

    pub fn mode(&self) -> ColorPickerMode {
        self.mode
    }

    pub fn with_mode(mut self, mode: ColorPickerMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn set_mode(&mut self, mode: ColorPickerMode) -> bool {
        let changed = self.mode != mode;
        self.mode = mode;
        changed
    }

    pub fn value_accessibility_meta(&self, label: impl Into<String>) -> AccessibilityMeta {
        AccessibilityMeta::new(AccessibilityRole::TextBox)
            .label(label)
            .value(format_hex_color(self.value, self.value.a < 255))
            .hint("Enter a hex color")
            .focusable()
    }

    pub fn palette_accessibility_meta(&self) -> AccessibilityMeta {
        AccessibilityMeta::new(AccessibilityRole::Grid)
            .label("Color palette")
            .value(format!("{} swatches", self.palette.swatches.len()))
            .focusable()
    }

    pub fn channel_accessibility_meta(&self, channel: ColorChannel) -> AccessibilityMeta {
        AccessibilityMeta::new(AccessibilityRole::Slider)
            .label(channel.label())
            .value(channel.format_value(self.hsv()))
            .hint(channel.hint())
            .value_range(match channel {
                ColorChannel::Hue => AccessibilityValueRange::new(0.0, 360.0),
                ColorChannel::Saturation | ColorChannel::Value | ColorChannel::Alpha => {
                    AccessibilityValueRange::new(0.0, 1.0)
                }
            })
            .focusable()
    }

    pub fn set_rgba(&mut self, value: ColorRgba, phase: EditPhase) -> ColorPickerUpdate {
        let previous = self.value;
        self.value = value;
        self.hsv = ColorHsv::from_rgba(value);
        self.oklch = ColorOklch::from_rgba(value);
        if phase == EditPhase::CommitEdit {
            self.remember_recent(value);
        }
        ColorPickerUpdate {
            previous,
            value: self.value,
            hsv: self.hsv(),
            oklch: self.oklch(),
            mode: self.mode(),
            phase,
            changed: previous != self.value,
        }
    }

    pub fn set_hsv(&mut self, value: ColorHsv, phase: EditPhase) -> ColorPickerUpdate {
        let previous = self.value;
        let previous_hsv = self.hsv;
        self.hsv = value;
        self.value = value.to_rgba();
        self.oklch = ColorOklch::from_rgba(self.value);
        if phase == EditPhase::CommitEdit {
            self.remember_recent(self.value);
        }
        ColorPickerUpdate {
            previous,
            value: self.value,
            hsv: self.hsv,
            oklch: self.oklch,
            mode: self.mode(),
            phase,
            changed: previous != self.value || previous_hsv != self.hsv,
        }
    }

    pub fn set_oklch(&mut self, value: ColorOklch, phase: EditPhase) -> ColorPickerUpdate {
        let previous = self.value;
        let previous_oklch = self.oklch;
        self.oklch = value;
        self.value = value.to_rgba();
        self.hsv = ColorHsv::from_rgba(self.value);
        if phase == EditPhase::CommitEdit {
            self.remember_recent(self.value);
        }
        ColorPickerUpdate {
            previous,
            value: self.value,
            hsv: self.hsv,
            oklch: self.oklch,
            mode: self.mode(),
            phase,
            changed: previous != self.value || previous_oklch != self.oklch,
        }
    }

    pub fn set_channel(
        &mut self,
        channel: ColorChannel,
        value: f32,
        phase: EditPhase,
    ) -> ColorPickerUpdate {
        let mut hsv = self.hsv();
        match channel {
            ColorChannel::Hue => hsv.hue = normalize_hue(value),
            ColorChannel::Saturation => hsv.saturation = unit(value),
            ColorChannel::Value => hsv.value = unit(value),
            ColorChannel::Alpha => hsv.alpha = unit(value),
        }
        self.set_hsv(hsv, phase)
    }

    pub fn set_oklch_channel(
        &mut self,
        channel: ColorOklchChannel,
        value: f32,
        phase: EditPhase,
    ) -> ColorPickerUpdate {
        let mut oklch = self.oklch();
        match channel {
            ColorOklchChannel::Lightness => oklch.lightness = unit(value),
            ColorOklchChannel::Chroma => oklch.chroma = chroma_unit(value),
            ColorOklchChannel::Hue => oklch.hue = normalize_hue(value),
            ColorOklchChannel::Alpha => oklch.alpha = unit(value),
        }
        self.set_oklch(oklch, phase)
    }

    pub fn nudge_channel(
        &mut self,
        channel: ColorChannel,
        steps: i32,
        speed: ColorChannelStep,
    ) -> ColorPickerUpdate {
        let hsv = self.hsv();
        let value = channel.value(hsv) + channel.step(speed) * steps as f32;
        self.set_channel(channel, value, EditPhase::UpdateEdit)
    }

    pub fn set_field_position(
        &mut self,
        local_position: UiPoint,
        size: UiSize,
        phase: EditPhase,
    ) -> ColorPickerUpdate {
        let width = size.width.max(1.0);
        let height = size.height.max(1.0);
        match self.mode {
            ColorPickerMode::Hsv => {
                let mut hsv = self.hsv();
                hsv.saturation = (local_position.x / width).clamp(0.0, 1.0);
                hsv.value = (1.0 - local_position.y / height).clamp(0.0, 1.0);
                self.set_hsv(hsv, phase)
            }
            ColorPickerMode::Oklch => {
                let mut oklch = self.oklch();
                oklch.chroma = ((local_position.x / width).clamp(0.0, 1.0)) * OKLCH_CHROMA_MAX;
                oklch.lightness = (1.0 - local_position.y / height).clamp(0.0, 1.0);
                self.set_oklch(oklch, phase)
            }
        }
    }

    pub fn set_channel_position(
        &mut self,
        channel: ColorChannel,
        local_position: UiPoint,
        size: UiSize,
        phase: EditPhase,
    ) -> ColorPickerUpdate {
        let t = (local_position.x / size.width.max(1.0)).clamp(0.0, 1.0);
        let value = match channel {
            ColorChannel::Hue => hue_from_unit(t),
            ColorChannel::Saturation | ColorChannel::Value | ColorChannel::Alpha => t,
        };
        self.set_channel(channel, value, phase)
    }

    pub fn set_oklch_channel_position(
        &mut self,
        channel: ColorOklchChannel,
        local_position: UiPoint,
        size: UiSize,
        phase: EditPhase,
    ) -> ColorPickerUpdate {
        let t = (local_position.x / size.width.max(1.0)).clamp(0.0, 1.0);
        let value = match channel {
            ColorOklchChannel::Lightness | ColorOklchChannel::Alpha => t,
            ColorOklchChannel::Chroma => t * OKLCH_CHROMA_MAX,
            ColorOklchChannel::Hue => hue_from_unit(t),
        };
        self.set_oklch_channel(channel, value, phase)
    }

    pub fn select_swatch(&mut self, id: &str) -> Option<ColorPickerUpdate> {
        let color = self.palette.find(id)?.color;
        Some(self.set_rgba(color, EditPhase::CommitEdit))
    }

    pub fn remember_recent(&mut self, color: ColorRgba) {
        self.recent.retain(|recent| *recent != color);
        self.recent.insert(0, color);
        self.recent.truncate(self.max_recent);
    }

    pub fn apply_pointer_edit(
        &mut self,
        target: ColorPickerTarget,
        edit: WidgetPointerEdit,
    ) -> ColorPickerUpdate {
        let phase = edit.phase.edit_phase();
        match target {
            ColorPickerTarget::Field => self.set_field_position(
                edit.local_position,
                UiSize::new(edit.target_rect.width, edit.target_rect.height),
                phase,
            ),
            ColorPickerTarget::Channel(channel) => match channel {
                ColorPickerChannel::Hsv(channel) => self.set_channel_position(
                    channel,
                    edit.local_position,
                    UiSize::new(edit.target_rect.width, edit.target_rect.height),
                    phase,
                ),
                ColorPickerChannel::Oklch(channel) => self.set_oklch_channel_position(
                    channel,
                    edit.local_position,
                    UiSize::new(edit.target_rect.width, edit.target_rect.height),
                    phase,
                ),
            },
        }
    }

    pub fn apply_action(
        &mut self,
        action_id: &str,
        kind: WidgetActionKind,
        options: ColorPickerActionOptions<'_>,
    ) -> ColorPickerActionOutcome {
        if Some(action_id) == options.copy_hex_action {
            return ColorPickerActionOutcome {
                update: None,
                effect: Some(ColorPickerEffect::CopyHex(format_hex_color(
                    self.value, false,
                ))),
                mode_changed: false,
            };
        }

        if let Some(swatch_id) = options
            .swatch_action_prefix
            .and_then(|prefix| action_id.strip_prefix(prefix))
            .and_then(|suffix| suffix.strip_prefix('.'))
        {
            return ColorPickerActionOutcome {
                update: self.select_swatch(swatch_id),
                effect: None,
                mode_changed: false,
            };
        }

        if let Some(mode) = options
            .action_prefix
            .and_then(|prefix| action_id.strip_prefix(prefix))
            .and_then(|suffix| suffix.strip_prefix(".mode."))
            .and_then(ColorPickerMode::from_action_suffix)
        {
            return ColorPickerActionOutcome {
                update: None,
                effect: None,
                mode_changed: self.set_mode(mode),
            };
        }

        let Some(target) = options
            .action_prefix
            .and_then(|prefix| action_id.strip_prefix(prefix))
            .and_then(|suffix| suffix.strip_prefix('.'))
            .and_then(|suffix| ColorPickerTarget::from_action_suffix(suffix, self.mode))
        else {
            return ColorPickerActionOutcome::default();
        };
        let WidgetActionKind::PointerEdit(edit) = kind else {
            return ColorPickerActionOutcome::default();
        };
        ColorPickerActionOutcome {
            update: Some(self.apply_pointer_edit(target, edit)),
            effect: None,
            mode_changed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPickerUpdate {
    pub previous: ColorRgba,
    pub value: ColorRgba,
    pub hsv: ColorHsv,
    pub oklch: ColorOklch,
    pub mode: ColorPickerMode,
    pub phase: EditPhase,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPickerTarget {
    Field,
    Channel(ColorPickerChannel),
}

impl ColorPickerTarget {
    pub fn from_action_suffix(suffix: &str, mode: ColorPickerMode) -> Option<Self> {
        Some(match suffix {
            "field" => Self::Field,
            "hue" => Self::Channel(match mode {
                ColorPickerMode::Hsv => ColorPickerChannel::Hsv(ColorChannel::Hue),
                ColorPickerMode::Oklch => ColorPickerChannel::Oklch(ColorOklchChannel::Hue),
            }),
            "saturation" if mode == ColorPickerMode::Hsv => {
                Self::Channel(ColorPickerChannel::Hsv(ColorChannel::Saturation))
            }
            "value" if mode == ColorPickerMode::Hsv => {
                Self::Channel(ColorPickerChannel::Hsv(ColorChannel::Value))
            }
            "lightness" if mode == ColorPickerMode::Oklch => {
                Self::Channel(ColorPickerChannel::Oklch(ColorOklchChannel::Lightness))
            }
            "chroma" if mode == ColorPickerMode::Oklch => {
                Self::Channel(ColorPickerChannel::Oklch(ColorOklchChannel::Chroma))
            }
            "alpha" => Self::Channel(match mode {
                ColorPickerMode::Hsv => ColorPickerChannel::Hsv(ColorChannel::Alpha),
                ColorPickerMode::Oklch => ColorPickerChannel::Oklch(ColorOklchChannel::Alpha),
            }),
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPickerChannel {
    Hsv(ColorChannel),
    Oklch(ColorOklchChannel),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ColorPickerActionOptions<'a> {
    pub action_prefix: Option<&'a str>,
    pub swatch_action_prefix: Option<&'a str>,
    pub copy_hex_action: Option<&'a str>,
}

impl<'a> ColorPickerActionOptions<'a> {
    pub const fn new(action_prefix: &'a str) -> Self {
        Self {
            action_prefix: Some(action_prefix),
            swatch_action_prefix: None,
            copy_hex_action: None,
        }
    }

    pub const fn swatches(mut self, prefix: &'a str) -> Self {
        self.swatch_action_prefix = Some(prefix);
        self
    }

    pub const fn copy_hex(mut self, action: &'a str) -> Self {
        self.copy_hex_action = Some(action);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorPickerEffect {
    CopyHex(String),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColorPickerActionOutcome {
    pub update: Option<ColorPickerUpdate>,
    pub effect: Option<ColorPickerEffect>,
    pub mode_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChannel {
    Hue,
    Saturation,
    Value,
    Alpha,
}

impl ColorChannel {
    pub const ALL: [Self; 4] = [Self::Hue, Self::Saturation, Self::Value, Self::Alpha];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Value => "Value",
            Self::Alpha => "Alpha",
        }
    }

    pub const fn action_suffix(self) -> &'static str {
        match self {
            Self::Hue => "hue",
            Self::Saturation => "saturation",
            Self::Value => "value",
            Self::Alpha => "alpha",
        }
    }

    pub const fn hint(self) -> &'static str {
        match self {
            Self::Hue => "Use arrow keys to adjust degrees",
            Self::Saturation | Self::Value | Self::Alpha => "Use arrow keys to adjust percentage",
        }
    }

    pub fn value(self, hsv: ColorHsv) -> f32 {
        match self {
            Self::Hue => hsv.hue,
            Self::Saturation => hsv.saturation,
            Self::Value => hsv.value,
            Self::Alpha => hsv.alpha,
        }
    }

    pub fn format_value(self, hsv: ColorHsv) -> String {
        match self {
            Self::Hue => format!("{} degrees", self.value(hsv).round() as i32),
            Self::Saturation | Self::Value | Self::Alpha => {
                format!("{}%", (self.value(hsv) * 100.0).round() as i32)
            }
        }
    }

    pub fn step(self, speed: ColorChannelStep) -> f32 {
        match (self, speed) {
            (Self::Hue, ColorChannelStep::Fine) => 1.0,
            (Self::Hue, ColorChannelStep::Normal) => 5.0,
            (Self::Hue, ColorChannelStep::Coarse) => 15.0,
            (_, ColorChannelStep::Fine) => 0.01,
            (_, ColorChannelStep::Normal) => 0.05,
            (_, ColorChannelStep::Coarse) => 0.10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorOklchChannel {
    Lightness,
    Chroma,
    Hue,
    Alpha,
}

impl ColorOklchChannel {
    pub const ALL: [Self; 4] = [Self::Lightness, Self::Chroma, Self::Hue, Self::Alpha];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Lightness => "Lightness",
            Self::Chroma => "Chroma",
            Self::Hue => "Hue",
            Self::Alpha => "Alpha",
        }
    }

    pub const fn action_suffix(self) -> &'static str {
        match self {
            Self::Lightness => "lightness",
            Self::Chroma => "chroma",
            Self::Hue => "hue",
            Self::Alpha => "alpha",
        }
    }

    pub const fn hint(self) -> &'static str {
        match self {
            Self::Hue => "Use arrow keys to adjust degrees",
            Self::Chroma => "Use arrow keys to adjust colorfulness",
            Self::Lightness | Self::Alpha => "Use arrow keys to adjust percentage",
        }
    }

    pub fn value(self, oklch: ColorOklch) -> f32 {
        match self {
            Self::Lightness => oklch.lightness,
            Self::Chroma => oklch.chroma,
            Self::Hue => oklch.hue,
            Self::Alpha => oklch.alpha,
        }
    }

    pub fn unit_value(self, oklch: ColorOklch) -> f32 {
        match self {
            Self::Chroma => (oklch.chroma / OKLCH_CHROMA_MAX).clamp(0.0, 1.0),
            Self::Hue => (oklch.hue / 360.0).clamp(0.0, 1.0),
            Self::Lightness => oklch.lightness,
            Self::Alpha => oklch.alpha,
        }
    }

    pub fn format_value(self, oklch: ColorOklch) -> String {
        match self {
            Self::Hue => format!("{} degrees", self.value(oklch).round() as i32),
            Self::Chroma => format!("{:.3}", self.value(oklch)),
            Self::Lightness | Self::Alpha => {
                format!("{}%", (self.value(oklch) * 100.0).round() as i32)
            }
        }
    }

    pub fn step(self, speed: ColorChannelStep) -> f32 {
        match (self, speed) {
            (Self::Hue, ColorChannelStep::Fine) => 1.0,
            (Self::Hue, ColorChannelStep::Normal) => 5.0,
            (Self::Hue, ColorChannelStep::Coarse) => 15.0,
            (Self::Chroma, ColorChannelStep::Fine) => 0.005,
            (Self::Chroma, ColorChannelStep::Normal) => 0.02,
            (Self::Chroma, ColorChannelStep::Coarse) => 0.05,
            (_, ColorChannelStep::Fine) => 0.01,
            (_, ColorChannelStep::Normal) => 0.05,
            (_, ColorChannelStep::Coarse) => 0.10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChannelStep {
    Fine,
    Normal,
    Coarse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorPickerStyle {
    pub text_field: PickerElementStyle,
    pub invalid_text_field: PickerElementStyle,
    pub swatch: PickerElementStyle,
    pub selected_swatch: PickerElementStyle,
    pub recent_swatch: PickerElementStyle,
    pub channel_slider: PickerElementStyle,
}

impl ColorPickerStyle {
    pub fn style_for_swatch(&self, selected: bool, recent: bool) -> &PickerElementStyle {
        if selected {
            &self.selected_swatch
        } else if recent {
            &self.recent_swatch
        } else {
            &self.swatch
        }
    }
}

impl Default for ColorPickerStyle {
    fn default() -> Self {
        Self {
            text_field: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(235, 240, 247, 255))
                .with_background(ColorRgba::new(18, 22, 28, 255)),
            invalid_text_field: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(255, 238, 240, 255))
                .with_border(ColorRgba::new(201, 74, 91, 255)),
            swatch: PickerElementStyle::default()
                .with_background(ColorRgba::new(28, 34, 43, 255))
                .with_border(ColorRgba::new(70, 82, 102, 255)),
            selected_swatch: PickerElementStyle::default()
                .with_border(ColorRgba::new(255, 255, 255, 255))
                .with_animation(PickerAnimationMeta::new("color.selected", 0.10)),
            recent_swatch: PickerElementStyle::default()
                .with_border(ColorRgba::new(106, 188, 137, 255)),
            channel_slider: PickerElementStyle::default()
                .with_background(ColorRgba::new(40, 47, 58, 255)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorPickerOptions {
    pub layout: LayoutStyle,
    pub style: ColorPickerStyle,
    pub label: String,
    pub action_prefix: Option<String>,
    pub swatch_action_prefix: Option<String>,
    pub copy_hex_action: Option<WidgetActionBinding>,
    pub copy_hex_label: String,
    pub show_swatches: bool,
}

impl Default for ColorPickerOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: length(220.0),
                    height: Dimension::auto(),
                },
                padding: TaffyRect::length(8.0),
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(8.0),
                    height: taffy::prelude::LengthPercentage::length(8.0),
                },
                ..Default::default()
            }),
            style: ColorPickerStyle::default(),
            label: "Color".to_string(),
            action_prefix: None,
            swatch_action_prefix: None,
            copy_hex_action: None,
            copy_hex_label: "Copy".to_string(),
            show_swatches: false,
        }
    }
}

impl ColorPickerOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }

    pub fn with_swatch_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.swatch_action_prefix = Some(prefix.into());
        self.show_swatches = true;
        self
    }

    pub const fn with_swatches(mut self, show_swatches: bool) -> Self {
        self.show_swatches = show_swatches;
        self
    }

    pub fn with_copy_hex_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.copy_hex_action = Some(action.into());
        self
    }

    pub fn with_copy_hex_label(mut self, label: impl Into<String>) -> Self {
        self.copy_hex_label = label.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorValueFormat {
    HexRgb,
    HexRgba,
    Rgb,
    Rgba,
    RgbaPremultiplied,
    RgbaUnmultiplied,
    Srgb,
    Srgba,
    SrgbaPremultiplied,
    SrgbaUnmultiplied,
    Color32,
    Hsva,
    Oklch,
}

impl ColorValueFormat {
    pub const fn label(self) -> &'static str {
        match self {
            Self::HexRgb => "HEX",
            Self::HexRgba => "HEXA",
            Self::Rgb => "RGB",
            Self::Rgba => "RGBA",
            Self::RgbaPremultiplied => "RGBA premultiplied",
            Self::RgbaUnmultiplied => "RGBA unmultiplied",
            Self::Srgb => "SRGB",
            Self::Srgba => "SRGBA",
            Self::SrgbaPremultiplied => "SRGBA premultiplied",
            Self::SrgbaUnmultiplied => "SRGBA unmultiplied",
            Self::Color32 => "Color32",
            Self::Hsva => "HSVA",
            Self::Oklch => "OKLCH",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorButtonOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub swatch_size: UiSize,
    pub text_style: TextStyle,
    pub format: ColorValueFormat,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub show_label: bool,
}

impl ColorButtonOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }

    pub fn with_swatch_size(mut self, size: UiSize) -> Self {
        self.swatch_size = UiSize::new(size.width.max(1.0), size.height.max(1.0));
        self
    }

    pub fn with_format(mut self, format: ColorValueFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub const fn show_label(mut self, show_label: bool) -> Self {
        self.show_label = show_label;
        self
    }
}

impl Default for ColorButtonOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(taffy::prelude::JustifyContent::Center),
                size: TaffySize {
                    width: length(112.0),
                    height: length(30.0),
                },
                padding: TaffyRect::length(4.0),
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(6.0),
                    height: taffy::prelude::LengthPercentage::length(6.0),
                },
                ..Default::default()
            }),
            visual: UiVisual::panel(
                ColorRgba::new(36, 42, 52, 255),
                Some(StrokeStyle::new(ColorRgba::new(74, 85, 104, 255), 1.0)),
                4.0,
            ),
            swatch_size: UiSize::new(20.0, 20.0),
            text_style: TextStyle {
                font_size: 11.0,
                line_height: 14.0,
                color: ColorRgba::new(235, 240, 247, 255),
                ..Default::default()
            },
            format: ColorValueFormat::HexRgb,
            action: None,
            accessibility_label: None,
            show_label: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorButtonNodes {
    pub root: UiNodeId,
    pub swatch: UiNodeId,
    pub label: Option<UiNodeId>,
}

#[derive(Debug, Clone)]
pub struct ColorHsva2dOptions {
    pub layout: LayoutStyle,
    pub action_prefix: Option<String>,
    pub accessibility_label: String,
}

impl Default for ColorHsva2dOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                size: TaffySize {
                    width: length(COLOR_FIELD_WIDTH),
                    height: length(COLOR_FIELD_HEIGHT),
                },
                ..Default::default()
            }),
            action_prefix: None,
            accessibility_label: "HSV color field".to_string(),
        }
    }
}

impl ColorHsva2dOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = label.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorHsva2dNodes {
    pub root: UiNodeId,
    pub field: UiNodeId,
}

pub fn compact_color_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(document, parent, name, color, options)
}

pub fn color_swatch_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(document, parent, name, color, options.show_label(false))
}

pub fn color_edit_button_rgb(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(
        document,
        parent,
        name,
        ColorRgba::new(color.r, color.g, color.b, 255),
        options.with_format(ColorValueFormat::Rgb),
    )
}

pub fn color_edit_button_rgba(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(
        document,
        parent,
        name,
        color,
        options.with_format(ColorValueFormat::Rgba),
    )
}

pub fn color_edit_button_rgba_premultiplied(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button_with_value(
        document,
        parent,
        name,
        unpremultiply_alpha(color),
        format_color_value(color, ColorValueFormat::RgbaPremultiplied),
        options.with_format(ColorValueFormat::RgbaPremultiplied),
    )
}

pub fn color_edit_button_rgba_unmultiplied(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(
        document,
        parent,
        name,
        color,
        options.with_format(ColorValueFormat::RgbaUnmultiplied),
    )
}

pub fn color_edit_button_srgb(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(
        document,
        parent,
        name,
        ColorRgba::new(color.r, color.g, color.b, 255),
        options.with_format(ColorValueFormat::Srgb),
    )
}

pub fn color_edit_button_srgba(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(
        document,
        parent,
        name,
        color,
        options.with_format(ColorValueFormat::Srgba),
    )
}

pub fn color_edit_button_srgba_premultiplied(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button_with_value(
        document,
        parent,
        name,
        unpremultiply_alpha(color),
        format_color_value(color, ColorValueFormat::SrgbaPremultiplied),
        options.with_format(ColorValueFormat::SrgbaPremultiplied),
    )
}

pub fn color_edit_button_srgba_unmultiplied(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(
        document,
        parent,
        name,
        color,
        options.with_format(ColorValueFormat::SrgbaUnmultiplied),
    )
}

pub fn color_picker_color32(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(
        document,
        parent,
        name,
        color,
        options.with_format(ColorValueFormat::Color32),
    )
}

pub fn color_edit_button_hsva(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(
        document,
        parent,
        name,
        color,
        options.with_format(ColorValueFormat::Hsva),
    )
}

pub fn color_edit_button_oklch(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    color_button(
        document,
        parent,
        name,
        color,
        options.with_format(ColorValueFormat::Oklch),
    )
}

pub fn color_picker_hsva_2d(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    hsv: ColorHsv,
    options: ColorHsva2dOptions,
) -> ColorHsva2dNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Slider)
                .label(options.accessibility_label.clone())
                .value(format!(
                    "hue {:.0}, saturation {:.0}%, value {:.0}%",
                    hsv.hue,
                    hsv.saturation * 100.0,
                    hsv.value * 100.0
                ))
                .hint("Drag to edit saturation and value")
                .focusable(),
        ),
    );
    let state = ColorPickerState::new(hsv.to_rgba()).with_mode(ColorPickerMode::Hsv);
    let field = color_field(
        document,
        root,
        &name,
        &state,
        options.action_prefix.as_deref(),
    );
    ColorHsva2dNodes { root, field }
}

pub fn show_color(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    size: UiSize,
) -> UiNodeId {
    let name = name.into();
    document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            LayoutStyle::size(size.width.max(1.0), size.height.max(1.0)),
        )
        .with_visual(UiVisual::panel(
            color,
            Some(StrokeStyle::new(ColorRgba::new(220, 226, 236, 255), 1.0)),
            3.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Image)
                .label(format!("{name} color"))
                .value(format_hex_color(color, color.a < 255)),
        ),
    )
}

pub fn show_color_at(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    rect: UiRect,
) -> UiNodeId {
    let name = name.into();
    document.add_child(
        parent,
        UiNode::container(name.clone(), LayoutStyle::absolute_rect(rect))
            .with_visual(UiVisual::panel(
                color,
                Some(StrokeStyle::new(ColorRgba::new(220, 226, 236, 255), 1.0)),
                3.0,
            ))
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Image)
                    .label(format!("{name} color"))
                    .value(format_hex_color(color, color.a < 255)),
            ),
    )
}

fn color_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    let name = name.into();
    let value = format_color_value(color, options.format);
    color_button_with_value(document, parent, name, color, value, options)
}

fn color_button_with_value(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    color: ColorRgba,
    value: String,
    options: ColorButtonOptions,
) -> ColorButtonNodes {
    let name = name.into();
    let mut root = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_visual(options.visual)
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Button)
            .label(
                options
                    .accessibility_label
                    .clone()
                    .unwrap_or_else(|| format!("Edit {name} color")),
            )
            .value(value.clone())
            .focusable(),
    );
    if let Some(action) = options.action {
        root = root.with_input(InputBehavior::BUTTON).with_action(action);
    }
    let root = document.add_child(parent, root);
    let swatch = document.add_child(
        root,
        UiNode::container(
            format!("{name}.swatch"),
            LayoutStyle::size(options.swatch_size.width, options.swatch_size.height),
        )
        .with_visual(UiVisual::panel(
            color,
            Some(StrokeStyle::new(ColorRgba::new(220, 226, 236, 255), 1.0)),
            3.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Image)
                .label(format!("{name} swatch"))
                .value(format_hex_color(color, color.a < 255)),
        ),
    );
    let label = options.show_label.then(|| {
        document.add_child(
            root,
            UiNode::text(
                format!("{name}.label"),
                value,
                options.text_style,
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    flex_shrink: 1.0,
                    ..Default::default()
                }),
            ),
        )
    });
    ColorButtonNodes {
        root,
        swatch,
        label,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPickerNodes {
    pub root: UiNodeId,
    pub value: UiNodeId,
    pub copy_hex: Option<UiNodeId>,
    pub swatches: Vec<UiNodeId>,
    pub channels: Vec<UiNodeId>,
}

pub fn color_picker(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    state: &ColorPickerState,
    options: ColorPickerOptions,
) -> ColorPickerNodes {
    let name = name.into();
    let text_style = TextStyle {
        font_size: 12.0,
        line_height: 16.0,
        color: options
            .style
            .text_field
            .foreground
            .unwrap_or(ColorRgba::new(235, 240, 247, 255)),
        ..Default::default()
    };
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            options
                .style
                .text_field
                .background
                .unwrap_or(ColorRgba::new(24, 29, 36, 255)),
            options
                .style
                .text_field
                .border
                .map(|color| StrokeStyle::new(color, 1.0)),
            4.0,
        ))
        .with_accessibility(state.palette_accessibility_meta()),
    );
    let value = document.add_child(
        root,
        UiNode::container(
            format!("{name}.value"),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(30.0),
                },
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(8.0),
                    height: taffy::prelude::LengthPercentage::length(8.0),
                },
                ..Default::default()
            }),
        )
        .with_accessibility(state.value_accessibility_meta(options.label)),
    );
    document.add_child(
        value,
        UiNode::container(
            format!("{name}.value.swatch"),
            LayoutStyle::size(24.0, 24.0),
        )
        .with_visual(UiVisual::panel(
            state.value,
            Some(StrokeStyle::new(ColorRgba::new(220, 226, 236, 255), 1.0)),
            3.0,
        )),
    );
    document.add_child(
        value,
        UiNode::text(
            format!("{name}.value.text"),
            format_hex_color(state.value, state.value.a < 255),
            text_style.clone(),
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        ),
    );
    let copy_hex = options.copy_hex_action.clone().map(|action| {
        let copy = document.add_child(
            value,
            UiNode::container(
                format!("{name}.value.copy_hex"),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        justify_content: Some(taffy::prelude::JustifyContent::Center),
                        size: TaffySize {
                            width: length(44.0),
                            height: length(22.0),
                        },
                        flex_shrink: 0.0,
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON)
            .with_action(action)
            .with_visual(UiVisual::panel(
                ColorRgba::new(48, 112, 184, 255),
                Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.0)),
                3.0,
            ))
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label("Copy current color as hex")
                    .value(format_hex_color(state.value, state.value.a < 255))
                    .focusable(),
            ),
        );
        document.add_child(
            copy,
            UiNode::text(
                format!("{name}.value.copy_hex.label"),
                options.copy_hex_label.clone(),
                TextStyle {
                    font_size: 10.0,
                    line_height: 12.0,
                    color: ColorRgba::new(246, 249, 252, 255),
                    ..Default::default()
                },
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            ),
        );
        copy
    });

    if options.action_prefix.is_some() {
        color_mode_row(
            document,
            root,
            &name,
            state,
            options.action_prefix.as_deref(),
        );
    }

    color_field(
        document,
        root,
        &name,
        state,
        options.action_prefix.as_deref(),
    );

    let mut swatches = Vec::new();
    if options.show_swatches {
        let swatch_row = document.add_child(
            root,
            UiNode::container(
                format!("{name}.swatches"),
                LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    flex_wrap: taffy::prelude::FlexWrap::Wrap,
                    gap: TaffySize {
                        width: taffy::prelude::LengthPercentage::length(6.0),
                        height: taffy::prelude::LengthPercentage::length(6.0),
                    },
                    ..Default::default()
                }),
            ),
        );
        for swatch in &state.palette.swatches {
            let selected = swatch.color == state.value;
            let style = options.style.style_for_swatch(selected, false);
            let mut node = UiNode::container(
                format!("{name}.swatch.{}", swatch.id),
                LayoutStyle::size(24.0, 24.0),
            )
            .with_input(InputBehavior::BUTTON)
            .with_visual(UiVisual::panel(
                swatch.color,
                Some(StrokeStyle::new(
                    style
                        .border
                        .unwrap_or_else(|| ColorRgba::new(70, 82, 102, 255)),
                    if selected { 2.0 } else { 1.0 },
                )),
                3.0,
            ))
            .with_accessibility(swatch.accessibility_meta(selected));
            if let Some(shader) = style.shader.clone().or_else(|| swatch.shader.clone()) {
                node = node.with_shader(shader);
            }
            if let Some(prefix) = options.swatch_action_prefix.as_deref() {
                node = node.with_action(format!("{prefix}.{}", swatch.id));
            }
            let node = document.add_child(swatch_row, node);
            swatches.push(node);
        }
    }

    let channels = match state.mode() {
        ColorPickerMode::Hsv => ColorChannel::ALL
            .into_iter()
            .map(|channel| {
                hsv_channel_row(
                    document,
                    root,
                    &name,
                    state,
                    channel,
                    &text_style,
                    options.action_prefix.as_deref(),
                )
            })
            .collect(),
        ColorPickerMode::Oklch => ColorOklchChannel::ALL
            .into_iter()
            .map(|channel| {
                oklch_channel_row(
                    document,
                    root,
                    &name,
                    state,
                    channel,
                    &text_style,
                    options.action_prefix.as_deref(),
                )
            })
            .collect(),
    };

    ColorPickerNodes {
        root,
        value,
        copy_hex,
        swatches,
        channels,
    }
}

fn color_field(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    state: &ColorPickerState,
    action_prefix: Option<&str>,
) -> UiNodeId {
    let field = document.add_child(
        parent,
        UiNode::container(
            format!("{name}.field"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    position: taffy::prelude::Position::Relative,
                    size: TaffySize {
                        width: length(COLOR_FIELD_WIDTH),
                        height: length(COLOR_FIELD_HEIGHT),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::TRANSPARENT,
            Some(StrokeStyle::new(ColorRgba::new(70, 82, 102, 255), 1.0)),
            3.0,
        ))
        .with_input(InputBehavior::BUTTON),
    );
    if let Some(prefix) = action_prefix {
        document.node_mut(field).action = Some(format!("{prefix}.field").into());
        document.node_mut(field).action_mode = crate::WidgetActionMode::PointerEdit;
    }
    match state.mode() {
        ColorPickerMode::Hsv => {
            let hsv = state.hsv();
            let hue = ColorHsv::new(hsv.hue, 1.0, 1.0, 1.0).to_rgba();
            document.add_child(
                field,
                UiNode::paint_fill(
                    format!("{name}.field.saturation"),
                    PaintBrush::LinearGradient(LinearGradient::new(
                        UiPoint::new(0.0, 0.0),
                        UiPoint::new(COLOR_FIELD_WIDTH, 0.0),
                        ColorRgba::WHITE,
                        hue,
                    )),
                    fill_layout(),
                ),
            );
            document.add_child(
                field,
                UiNode::paint_fill(
                    format!("{name}.field.value"),
                    PaintBrush::LinearGradient(LinearGradient::new(
                        UiPoint::new(0.0, 0.0),
                        UiPoint::new(0.0, COLOR_FIELD_HEIGHT),
                        ColorRgba::new(0, 0, 0, 0),
                        ColorRgba::new(0, 0, 0, 255),
                    )),
                    fill_layout(),
                ),
            );
        }
        ColorPickerMode::Oklch => {
            document.add_child(
                field,
                UiNode::scene(
                    format!("{name}.field.oklch"),
                    oklch_field_rects(state.oklch()),
                    fill_layout(),
                ),
            );
        }
    }
    let (thumb_x, thumb_y) = match state.mode() {
        ColorPickerMode::Hsv => {
            let hsv = state.hsv();
            (
                hsv.saturation * COLOR_FIELD_WIDTH,
                (1.0 - hsv.value) * COLOR_FIELD_HEIGHT,
            )
        }
        ColorPickerMode::Oklch => {
            let oklch = state.oklch();
            (
                (oklch.chroma / OKLCH_CHROMA_MAX).clamp(0.0, 1.0) * COLOR_FIELD_WIDTH,
                (1.0 - oklch.lightness) * COLOR_FIELD_HEIGHT,
            )
        }
    };
    document.add_child(
        field,
        UiNode::container(
            format!("{name}.field.thumb"),
            LayoutStyle::absolute_rect(UiRect::new(thumb_x - 5.0, thumb_y - 5.0, 10.0, 10.0)),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::TRANSPARENT,
            Some(StrokeStyle::new(ColorRgba::WHITE, 2.0)),
            5.0,
        )),
    );
    field
}

fn color_mode_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    state: &ColorPickerState,
    action_prefix: Option<&str>,
) -> UiNodeId {
    let row = document.add_child(
        parent,
        UiNode::container(
            format!("{name}.mode"),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(26.0),
                },
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(6.0),
                    height: taffy::prelude::LengthPercentage::length(6.0),
                },
                ..Default::default()
            }),
        ),
    );

    for mode in ColorPickerMode::ALL {
        let selected = mode == state.mode();
        let mut node = UiNode::container(
            format!("{name}.mode.{}", mode.label().to_ascii_lowercase()),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    align_items: Some(AlignItems::Center),
                    justify_content: Some(taffy::prelude::JustifyContent::Center),
                    size: TaffySize {
                        width: length(68.0),
                        height: length(24.0),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            if selected {
                ColorRgba::new(48, 112, 184, 255)
            } else {
                ColorRgba::new(32, 38, 47, 255)
            },
            Some(StrokeStyle::new(
                if selected {
                    ColorRgba::new(120, 170, 230, 255)
                } else {
                    ColorRgba::new(70, 82, 102, 255)
                },
                1.0,
            )),
            3.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Button)
                .label(format!("{} color mode", mode.label()))
                .selected(selected)
                .focusable(),
        );
        if let Some(prefix) = action_prefix {
            node = node.with_action(format!(
                "{prefix}.mode.{}",
                mode.label().to_ascii_lowercase()
            ));
        }
        let button = document.add_child(row, node);
        document.add_child(
            button,
            UiNode::text(
                format!("{name}.mode.{}.label", mode.label().to_ascii_lowercase()),
                mode.label(),
                TextStyle {
                    font_size: 11.0,
                    line_height: 13.0,
                    color: ColorRgba::new(238, 244, 252, 255),
                    ..Default::default()
                },
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            ),
        );
    }

    row
}

fn hsv_channel_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    state: &ColorPickerState,
    channel: ColorChannel,
    text_style: &TextStyle,
    action_prefix: Option<&str>,
) -> UiNodeId {
    let row = document.add_child(
        parent,
        UiNode::container(
            format!("{name}.channel.{}", channel.label().to_ascii_lowercase()),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(22.0),
                },
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(8.0),
                    height: taffy::prelude::LengthPercentage::length(8.0),
                },
                ..Default::default()
            }),
        )
        .with_accessibility(state.channel_accessibility_meta(channel)),
    );
    document.add_child(
        row,
        UiNode::text(
            format!(
                "{name}.channel.{}.label",
                channel.label().to_ascii_lowercase()
            ),
            channel.label(),
            text_style.clone(),
            LayoutStyle::size(72.0, 16.0),
        ),
    );
    let track = if channel == ColorChannel::Alpha {
        alpha_channel_track(document, row, name, state, action_prefix)
    } else {
        let mut track_node = UiNode::paint_rect(
            format!(
                "{name}.channel.{}.track",
                channel.label().to_ascii_lowercase()
            ),
            PaintRect::new(
                UiRect::new(0.0, 0.0, 0.0, 0.0),
                channel_gradient(state.hsv(), channel),
            )
            .stroke(StrokeStyle::new(ColorRgba::new(70, 82, 102, 255), 1.0)),
            LayoutStyle::size(CHANNEL_TRACK_WIDTH, CHANNEL_TRACK_HEIGHT),
        )
        .with_input(InputBehavior::BUTTON);
        if let Some(prefix) = action_prefix {
            track_node = track_node
                .with_pointer_edit_action(format!("{prefix}.{}", channel.action_suffix()));
        }
        document.add_child(row, track_node)
    };
    document.add_child(
        track,
        UiNode::container(
            format!(
                "{name}.channel.{}.thumb",
                channel.label().to_ascii_lowercase()
            ),
            LayoutStyle::absolute_rect(UiRect::new(
                channel_unit_value(state.hsv(), channel) * (CHANNEL_TRACK_WIDTH - 8.0),
                -2.0,
                8.0,
                16.0,
            )),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(242, 246, 250, 255),
            Some(StrokeStyle::new(ColorRgba::new(35, 42, 52, 255), 1.0)),
            2.0,
        )),
    );
    row
}

fn oklch_channel_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    state: &ColorPickerState,
    channel: ColorOklchChannel,
    text_style: &TextStyle,
    action_prefix: Option<&str>,
) -> UiNodeId {
    let row = document.add_child(
        parent,
        UiNode::container(
            format!("{name}.channel.{}", channel.action_suffix()),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(22.0),
                },
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(8.0),
                    height: taffy::prelude::LengthPercentage::length(8.0),
                },
                ..Default::default()
            }),
        )
        .with_accessibility(oklch_channel_accessibility_meta(state, channel)),
    );
    document.add_child(
        row,
        UiNode::text(
            format!("{name}.channel.{}.label", channel.action_suffix()),
            channel.label(),
            text_style.clone(),
            LayoutStyle::size(72.0, 16.0),
        ),
    );
    let track = if channel == ColorOklchChannel::Alpha {
        oklch_alpha_channel_track(document, row, name, state, action_prefix)
    } else {
        let mut track_node = UiNode::paint_rect(
            format!("{name}.channel.{}.track", channel.action_suffix()),
            PaintRect::new(
                UiRect::new(0.0, 0.0, 0.0, 0.0),
                oklch_channel_gradient(state.oklch(), channel),
            )
            .stroke(StrokeStyle::new(ColorRgba::new(70, 82, 102, 255), 1.0)),
            LayoutStyle::size(CHANNEL_TRACK_WIDTH, CHANNEL_TRACK_HEIGHT),
        )
        .with_input(InputBehavior::BUTTON);
        if let Some(prefix) = action_prefix {
            track_node = track_node
                .with_pointer_edit_action(format!("{prefix}.{}", channel.action_suffix()));
        }
        document.add_child(row, track_node)
    };
    document.add_child(
        track,
        UiNode::container(
            format!("{name}.channel.{}.thumb", channel.action_suffix()),
            LayoutStyle::absolute_rect(UiRect::new(
                channel.unit_value(state.oklch()) * (CHANNEL_TRACK_WIDTH - 8.0),
                -2.0,
                8.0,
                16.0,
            )),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(242, 246, 250, 255),
            Some(StrokeStyle::new(ColorRgba::new(35, 42, 52, 255), 1.0)),
            2.0,
        )),
    );
    row
}

fn alpha_channel_track(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    state: &ColorPickerState,
    action_prefix: Option<&str>,
) -> UiNodeId {
    let mut track_node = UiNode::container(
        format!("{name}.channel.alpha.track"),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                position: taffy::prelude::Position::Relative,
                size: TaffySize {
                    width: length(CHANNEL_TRACK_WIDTH),
                    height: length(CHANNEL_TRACK_HEIGHT),
                },
                ..Default::default()
            })
            .style,
            clip: ClipBehavior::None,
            ..Default::default()
        },
    )
    .with_input(InputBehavior::BUTTON);
    if let Some(prefix) = action_prefix {
        track_node = track_node.with_pointer_edit_action(format!("{prefix}.alpha"));
    }
    let track = document.add_child(parent, track_node);
    document.add_child(
        track,
        UiNode::scene(
            format!("{name}.channel.alpha.checker"),
            checkerboard_rects(CHANNEL_TRACK_WIDTH, CHANNEL_TRACK_HEIGHT, 6.0),
            fill_layout(),
        ),
    );
    document.add_child(
        track,
        UiNode::paint_rect(
            format!("{name}.channel.alpha.gradient"),
            PaintRect::new(
                UiRect::new(0.0, 0.0, 0.0, 0.0),
                channel_gradient(state.hsv(), ColorChannel::Alpha),
            )
            .stroke(StrokeStyle::new(ColorRgba::new(70, 82, 102, 255), 1.0)),
            fill_layout(),
        ),
    );
    track
}

fn oklch_alpha_channel_track(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    state: &ColorPickerState,
    action_prefix: Option<&str>,
) -> UiNodeId {
    let mut track_node = UiNode::container(
        format!("{name}.channel.alpha.track"),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                position: taffy::prelude::Position::Relative,
                size: TaffySize {
                    width: length(CHANNEL_TRACK_WIDTH),
                    height: length(CHANNEL_TRACK_HEIGHT),
                },
                ..Default::default()
            })
            .style,
            clip: ClipBehavior::None,
            ..Default::default()
        },
    )
    .with_input(InputBehavior::BUTTON);
    if let Some(prefix) = action_prefix {
        track_node = track_node.with_pointer_edit_action(format!("{prefix}.alpha"));
    }
    let track = document.add_child(parent, track_node);
    document.add_child(
        track,
        UiNode::scene(
            format!("{name}.channel.alpha.checker"),
            checkerboard_rects(CHANNEL_TRACK_WIDTH, CHANNEL_TRACK_HEIGHT, 6.0),
            fill_layout(),
        ),
    );
    document.add_child(
        track,
        UiNode::paint_rect(
            format!("{name}.channel.alpha.gradient"),
            PaintRect::new(
                UiRect::new(0.0, 0.0, 0.0, 0.0),
                oklch_channel_gradient(state.oklch(), ColorOklchChannel::Alpha),
            )
            .stroke(StrokeStyle::new(ColorRgba::new(70, 82, 102, 255), 1.0)),
            fill_layout(),
        ),
    );
    track
}

fn oklch_field_rects(oklch: ColorOklch) -> Vec<ScenePrimitive> {
    let strip_height = COLOR_FIELD_HEIGHT / OKLCH_FIELD_LIGHTNESS_STRIPS as f32;
    let mut primitives = Vec::with_capacity(OKLCH_FIELD_LIGHTNESS_STRIPS);
    for row in 0..OKLCH_FIELD_LIGHTNESS_STRIPS {
        let lightness = 1.0 - ((row as f32 + 0.5) / OKLCH_FIELD_LIGHTNESS_STRIPS as f32);
        let stops = (0..=OKLCH_FIELD_CHROMA_STOPS)
            .map(|index| {
                let offset = index as f32 / OKLCH_FIELD_CHROMA_STOPS as f32;
                GradientStop::new(
                    offset,
                    ColorOklch::new(lightness, offset * OKLCH_CHROMA_MAX, oklch.hue, oklch.alpha)
                        .to_rgba(),
                )
            })
            .collect();
        primitives.push(ScenePrimitive::Rect(PaintRect::new(
            UiRect::new(
                0.0,
                row as f32 * strip_height,
                COLOR_FIELD_WIDTH,
                strip_height + 0.5,
            ),
            PaintBrush::LinearGradient(LinearGradient {
                start: UiPoint::new(0.0, 0.0),
                end: UiPoint::new(COLOR_FIELD_WIDTH, 0.0),
                stops,
                fallback: ColorOklch::new(lightness, oklch.chroma, oklch.hue, oklch.alpha)
                    .to_rgba(),
            }),
        )));
    }
    primitives
}

fn checkerboard_rects(width: f32, height: f32, cell: f32) -> Vec<ScenePrimitive> {
    let cell = cell.max(1.0);
    let columns = (width / cell).ceil() as usize;
    let rows = (height / cell).ceil() as usize;
    let mut primitives = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        for column in 0..columns {
            let color = if (row + column) % 2 == 0 {
                ColorRgba::new(238, 241, 245, 255)
            } else {
                ColorRgba::new(166, 174, 184, 255)
            };
            primitives.push(ScenePrimitive::Rect(PaintRect::solid(
                UiRect::new(column as f32 * cell, row as f32 * cell, cell, cell),
                color,
            )));
        }
    }
    primitives
}

fn channel_unit_value(hsv: ColorHsv, channel: ColorChannel) -> f32 {
    match channel {
        ColorChannel::Hue => (hsv.hue / 360.0).clamp(0.0, 1.0),
        ColorChannel::Saturation => hsv.saturation,
        ColorChannel::Value => hsv.value,
        ColorChannel::Alpha => hsv.alpha,
    }
}

fn channel_gradient(hsv: ColorHsv, channel: ColorChannel) -> PaintBrush {
    let gradient = match channel {
        ColorChannel::Hue => LinearGradient {
            start: UiPoint::new(0.0, 0.0),
            end: UiPoint::new(CHANNEL_TRACK_WIDTH, 0.0),
            stops: vec![
                GradientStop::new(0.0, ColorHsv::new(0.0, 1.0, 1.0, 1.0).to_rgba()),
                GradientStop::new(1.0 / 6.0, ColorHsv::new(60.0, 1.0, 1.0, 1.0).to_rgba()),
                GradientStop::new(2.0 / 6.0, ColorHsv::new(120.0, 1.0, 1.0, 1.0).to_rgba()),
                GradientStop::new(3.0 / 6.0, ColorHsv::new(180.0, 1.0, 1.0, 1.0).to_rgba()),
                GradientStop::new(4.0 / 6.0, ColorHsv::new(240.0, 1.0, 1.0, 1.0).to_rgba()),
                GradientStop::new(5.0 / 6.0, ColorHsv::new(300.0, 1.0, 1.0, 1.0).to_rgba()),
                GradientStop::new(1.0, ColorHsv::new(360.0, 1.0, 1.0, 1.0).to_rgba()),
            ],
            fallback: ColorHsv::new(hsv.hue, 1.0, 1.0, 1.0).to_rgba(),
        },
        ColorChannel::Saturation => LinearGradient::new(
            UiPoint::new(0.0, 0.0),
            UiPoint::new(CHANNEL_TRACK_WIDTH, 0.0),
            ColorHsv::new(hsv.hue, 0.0, hsv.value, hsv.alpha).to_rgba(),
            ColorHsv::new(hsv.hue, 1.0, hsv.value, hsv.alpha).to_rgba(),
        ),
        ColorChannel::Value => LinearGradient::new(
            UiPoint::new(0.0, 0.0),
            UiPoint::new(CHANNEL_TRACK_WIDTH, 0.0),
            ColorHsv::new(hsv.hue, hsv.saturation, 0.0, hsv.alpha).to_rgba(),
            ColorHsv::new(hsv.hue, hsv.saturation, 1.0, hsv.alpha).to_rgba(),
        ),
        ColorChannel::Alpha => {
            let opaque = ColorHsv::new(hsv.hue, hsv.saturation, hsv.value, 1.0).to_rgba();
            LinearGradient::new(
                UiPoint::new(0.0, 0.0),
                UiPoint::new(CHANNEL_TRACK_WIDTH, 0.0),
                ColorRgba::new(opaque.r, opaque.g, opaque.b, 0),
                opaque,
            )
        }
    };
    PaintBrush::LinearGradient(gradient)
}

fn oklch_channel_gradient(oklch: ColorOklch, channel: ColorOklchChannel) -> PaintBrush {
    let gradient = match channel {
        ColorOklchChannel::Lightness => LinearGradient::new(
            UiPoint::new(0.0, 0.0),
            UiPoint::new(CHANNEL_TRACK_WIDTH, 0.0),
            ColorOklch::new(0.0, oklch.chroma, oklch.hue, oklch.alpha).to_rgba(),
            ColorOklch::new(1.0, oklch.chroma, oklch.hue, oklch.alpha).to_rgba(),
        ),
        ColorOklchChannel::Chroma => LinearGradient::new(
            UiPoint::new(0.0, 0.0),
            UiPoint::new(CHANNEL_TRACK_WIDTH, 0.0),
            ColorOklch::new(oklch.lightness, 0.0, oklch.hue, oklch.alpha).to_rgba(),
            ColorOklch::new(oklch.lightness, OKLCH_CHROMA_MAX, oklch.hue, oklch.alpha).to_rgba(),
        ),
        ColorOklchChannel::Hue => LinearGradient {
            start: UiPoint::new(0.0, 0.0),
            end: UiPoint::new(CHANNEL_TRACK_WIDTH, 0.0),
            stops: vec![
                GradientStop::new(
                    0.0,
                    ColorOklch::new(oklch.lightness, oklch.chroma, 0.0, 1.0).to_rgba(),
                ),
                GradientStop::new(
                    1.0 / 6.0,
                    ColorOklch::new(oklch.lightness, oklch.chroma, 60.0, 1.0).to_rgba(),
                ),
                GradientStop::new(
                    2.0 / 6.0,
                    ColorOklch::new(oklch.lightness, oklch.chroma, 120.0, 1.0).to_rgba(),
                ),
                GradientStop::new(
                    3.0 / 6.0,
                    ColorOklch::new(oklch.lightness, oklch.chroma, 180.0, 1.0).to_rgba(),
                ),
                GradientStop::new(
                    4.0 / 6.0,
                    ColorOklch::new(oklch.lightness, oklch.chroma, 240.0, 1.0).to_rgba(),
                ),
                GradientStop::new(
                    5.0 / 6.0,
                    ColorOklch::new(oklch.lightness, oklch.chroma, 300.0, 1.0).to_rgba(),
                ),
                GradientStop::new(
                    1.0,
                    ColorOklch::new(oklch.lightness, oklch.chroma, 360.0, 1.0).to_rgba(),
                ),
            ],
            fallback: ColorOklch::new(oklch.lightness, oklch.chroma, oklch.hue, 1.0).to_rgba(),
        },
        ColorOklchChannel::Alpha => {
            let opaque = ColorOklch::new(oklch.lightness, oklch.chroma, oklch.hue, 1.0).to_rgba();
            LinearGradient::new(
                UiPoint::new(0.0, 0.0),
                UiPoint::new(CHANNEL_TRACK_WIDTH, 0.0),
                ColorRgba::new(opaque.r, opaque.g, opaque.b, 0),
                opaque,
            )
        }
    };
    PaintBrush::LinearGradient(gradient)
}

fn oklch_channel_accessibility_meta(
    state: &ColorPickerState,
    channel: ColorOklchChannel,
) -> AccessibilityMeta {
    AccessibilityMeta::new(AccessibilityRole::Slider)
        .label(channel.label())
        .value(channel.format_value(state.oklch()))
        .hint(channel.hint())
        .value_range(match channel {
            ColorOklchChannel::Hue => AccessibilityValueRange::new(0.0, 360.0),
            ColorOklchChannel::Chroma => AccessibilityValueRange::new(0.0, OKLCH_CHROMA_MAX as f64),
            ColorOklchChannel::Lightness | ColorOklchChannel::Alpha => {
                AccessibilityValueRange::new(0.0, 1.0)
            }
        })
        .focusable()
}

fn fill_layout() -> LayoutStyle {
    LayoutStyle::from_taffy_style(Style {
        position: taffy::prelude::Position::Absolute,
        inset: TaffyRect {
            left: taffy::prelude::LengthPercentageAuto::length(0.0),
            right: taffy::prelude::LengthPercentageAuto::length(0.0),
            top: taffy::prelude::LengthPercentageAuto::length(0.0),
            bottom: taffy::prelude::LengthPercentageAuto::length(0.0),
        },
        size: TaffySize {
            width: Dimension::percent(1.0),
            height: Dimension::percent(1.0),
        },
        ..Default::default()
    })
}

pub fn format_hex_color(color: ColorRgba, include_alpha: bool) -> String {
    if include_alpha {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            color.r, color.g, color.b, color.a
        )
    } else {
        format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
    }
}

pub fn format_color_value(color: ColorRgba, format: ColorValueFormat) -> String {
    match format {
        ColorValueFormat::HexRgb => format_hex_color(color, false),
        ColorValueFormat::HexRgba => format_hex_color(color, true),
        ColorValueFormat::Rgb => format!("rgb({}, {}, {})", color.r, color.g, color.b),
        ColorValueFormat::Rgba => format!(
            "rgba({}, {}, {}, {:.3})",
            color.r,
            color.g,
            color.b,
            color.a as f32 / 255.0
        ),
        ColorValueFormat::RgbaPremultiplied => format!(
            "rgba_premultiplied({}, {}, {}, {:.3})",
            color.r,
            color.g,
            color.b,
            color.a as f32 / 255.0
        ),
        ColorValueFormat::RgbaUnmultiplied => format!(
            "rgba_unmultiplied({}, {}, {}, {:.3})",
            color.r,
            color.g,
            color.b,
            color.a as f32 / 255.0
        ),
        ColorValueFormat::Srgb => format!("srgb({}, {}, {})", color.r, color.g, color.b),
        ColorValueFormat::Srgba => format!(
            "srgba({}, {}, {}, {:.3})",
            color.r,
            color.g,
            color.b,
            color.a as f32 / 255.0
        ),
        ColorValueFormat::SrgbaPremultiplied => format!(
            "srgba_premultiplied({}, {}, {}, {:.3})",
            color.r,
            color.g,
            color.b,
            color.a as f32 / 255.0
        ),
        ColorValueFormat::SrgbaUnmultiplied => format!(
            "srgba_unmultiplied({}, {}, {}, {:.3})",
            color.r,
            color.g,
            color.b,
            color.a as f32 / 255.0
        ),
        ColorValueFormat::Color32 => {
            format!(
                "Color32({}, {}, {}, {})",
                color.r, color.g, color.b, color.a
            )
        }
        ColorValueFormat::Hsva => {
            let hsv = ColorHsv::from_rgba(color);
            format!(
                "hsva({:.0}, {:.3}, {:.3}, {:.3})",
                hsv.hue, hsv.saturation, hsv.value, hsv.alpha
            )
        }
        ColorValueFormat::Oklch => {
            let oklch = ColorOklch::from_rgba(color);
            format!(
                "oklch({:.3}, {:.3}, {:.0}, {:.3})",
                oklch.lightness, oklch.chroma, oklch.hue, oklch.alpha
            )
        }
    }
}

pub fn parse_hex_color(value: &str) -> Option<ColorRgba> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    match value.len() {
        3 | 4 => {
            let mut chars = value.chars();
            let r = hex_nibble(chars.next()?)? * 17;
            let g = hex_nibble(chars.next()?)? * 17;
            let b = hex_nibble(chars.next()?)? * 17;
            let a = chars
                .next()
                .map_or(Some(255), |alpha| hex_nibble(alpha).map(|alpha| alpha * 17))?;
            Some(ColorRgba::new(r, g, b, a))
        }
        6 | 8 => {
            let r = hex_byte(&value[0..2])?;
            let g = hex_byte(&value[2..4])?;
            let b = hex_byte(&value[4..6])?;
            let a = if value.len() == 8 {
                hex_byte(&value[6..8])?
            } else {
                255
            };
            Some(ColorRgba::new(r, g, b, a))
        }
        _ => None,
    }
}

fn normalize_hue(hue: f32) -> f32 {
    if hue.is_finite() {
        let normalized = hue.rem_euclid(360.0);
        if normalized <= f32::EPSILON && hue > 0.0 {
            360.0
        } else {
            normalized
        }
    } else {
        0.0
    }
}

fn hue_from_unit(value: f32) -> f32 {
    if value >= 1.0 {
        360.0
    } else {
        value.clamp(0.0, 1.0) * 360.0
    }
}

fn unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn chroma_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, OKLCH_CHROMA_MAX)
    } else {
        0.0
    }
}

fn channel(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> f32 {
    let value = if value.is_finite() { value } else { 0.0 };
    if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.max(0.0).powf(1.0 / 2.4) - 0.055
    }
    .clamp(0.0, 1.0)
}

fn hex_nibble(value: char) -> Option<u8> {
    value.to_digit(16).map(|value| value as u8)
}

fn hex_byte(value: &str) -> Option<u8> {
    u8::from_str_radix(value, 16).ok()
}

#[cfg(test)]
fn premultiply_alpha(color: ColorRgba) -> ColorRgba {
    let alpha = color.a as u16;
    ColorRgba::new(
        ((color.r as u16 * alpha + 127) / 255) as u8,
        ((color.g as u16 * alpha + 127) / 255) as u8,
        ((color.b as u16 * alpha + 127) / 255) as u8,
        color.a,
    )
}

fn unpremultiply_alpha(color: ColorRgba) -> ColorRgba {
    if color.a == 0 {
        return ColorRgba::TRANSPARENT;
    }
    let alpha = color.a as u16;
    ColorRgba::new(
        ((color.r as u16 * 255 + alpha / 2) / alpha).min(255) as u8,
        ((color.g as u16 * 255 + alpha / 2) / alpha).min(255) as u8,
        ((color.b as u16 * 255 + alpha / 2) / alpha).min(255) as u8,
        color.a,
    )
}

#[cfg(test)]
mod tests {
    use crate::root_style;

    use super::*;

    #[test]
    fn color_button_builders_expose_common_formats_and_actions() {
        let mut document = UiDocument::new(root_style(360.0, 180.0));
        let root = document.root;
        let color = ColorRgba::new(51, 102, 153, 128);

        let rgb = color_edit_button_rgb(
            &mut document,
            root,
            "brand.rgb",
            color,
            ColorButtonOptions::default().with_action("color.open"),
        );
        let rgba = color_edit_button_rgba(
            &mut document,
            root,
            "brand.rgba",
            color,
            ColorButtonOptions::default(),
        );
        let hsva = color_edit_button_hsva(
            &mut document,
            root,
            "brand.hsva",
            color,
            ColorButtonOptions::default(),
        );
        let swatch = color_swatch_button(
            &mut document,
            root,
            "brand.swatch",
            color,
            ColorButtonOptions::default(),
        );
        let absolute = show_color_at(
            &mut document,
            root,
            "brand.absolute",
            color,
            UiRect::new(20.0, 40.0, 24.0, 24.0),
        );

        assert_eq!(
            document.node(rgb.root).action.as_ref(),
            Some(&WidgetActionBinding::action("color.open"))
        );
        assert_eq!(
            document
                .node(rgb.root)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("rgb(51, 102, 153)")
        );
        assert_eq!(
            document
                .node(rgba.root)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("rgba(51, 102, 153, 0.502)")
        );
        assert!(document
            .node(hsva.root)
            .accessibility
            .as_ref()
            .unwrap()
            .value
            .as_deref()
            .unwrap()
            .starts_with("hsva(210"));
        assert!(swatch.label.is_none());
        assert!(
            document.node(absolute).style.layout.position == taffy::prelude::Position::Absolute
        );
    }

    #[test]
    fn color_button_builders_cover_egui_style_color_conveniences() {
        let mut document = UiDocument::new(root_style(420.0, 240.0));
        let root = document.root;
        let color = ColorRgba::new(51, 102, 153, 128);
        let premultiplied = premultiply_alpha(color);

        let rgba_premultiplied = color_edit_button_rgba_premultiplied(
            &mut document,
            root,
            "brand.rgba_premultiplied",
            premultiplied,
            ColorButtonOptions::default(),
        );
        let rgba_unmultiplied = color_edit_button_rgba_unmultiplied(
            &mut document,
            root,
            "brand.rgba_unmultiplied",
            color,
            ColorButtonOptions::default(),
        );
        let srgba_premultiplied = color_edit_button_srgba_premultiplied(
            &mut document,
            root,
            "brand.srgba_premultiplied",
            premultiplied,
            ColorButtonOptions::default(),
        );
        let srgba_unmultiplied = color_edit_button_srgba_unmultiplied(
            &mut document,
            root,
            "brand.srgba_unmultiplied",
            color,
            ColorButtonOptions::default(),
        );
        let color32 = color_picker_color32(
            &mut document,
            root,
            "brand.color32",
            color,
            ColorButtonOptions::default(),
        );
        let hsva_2d = color_picker_hsva_2d(
            &mut document,
            root,
            "brand.hsva_2d",
            ColorHsv::from_rgba(color),
            ColorHsva2dOptions::default().with_action_prefix("brand.hsva_2d"),
        );

        assert_eq!(
            document
                .node(rgba_premultiplied.root)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("rgba_premultiplied(26, 51, 77, 0.502)")
        );
        assert_eq!(
            document
                .node(rgba_unmultiplied.root)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("rgba_unmultiplied(51, 102, 153, 0.502)")
        );
        assert_eq!(
            document
                .node(srgba_premultiplied.root)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("srgba_premultiplied(26, 51, 77, 0.502)")
        );
        assert_eq!(
            document
                .node(srgba_unmultiplied.root)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("srgba_unmultiplied(51, 102, 153, 0.502)")
        );
        assert_eq!(
            document
                .node(color32.root)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("Color32(51, 102, 153, 128)")
        );
        assert_eq!(
            document.node(hsva_2d.field).action.as_ref(),
            Some(&WidgetActionBinding::action("brand.hsva_2d.field"))
        );
    }

    #[test]
    fn color_value_format_helpers_cover_hex_oklch_and_srgb() {
        let color = ColorRgba::new(51, 102, 153, 128);

        assert_eq!(
            format_color_value(color, ColorValueFormat::HexRgb),
            "#336699"
        );
        assert_eq!(
            format_color_value(color, ColorValueFormat::HexRgba),
            "#33669980"
        );
        assert_eq!(
            format_color_value(color, ColorValueFormat::Srgba),
            "srgba(51, 102, 153, 0.502)"
        );
        assert!(format_color_value(color, ColorValueFormat::Oklch).starts_with("oklch("));
    }

    #[test]
    fn color_picker_state_applies_prefixed_pointer_and_copy_actions() {
        let mut state = ColorPickerState::new(ColorRgba::new(255, 0, 0, 255));
        let pointer = WidgetPointerEdit::new(
            crate::WidgetValueEditPhase::Commit,
            UiPoint::new(50.0, 25.0),
            UiPoint::new(50.0, 25.0),
            UiRect::new(0.0, 0.0, 100.0, 100.0),
        );

        let outcome = state.apply_action(
            "color.field",
            WidgetActionKind::PointerEdit(pointer),
            ColorPickerActionOptions::new("color").copy_hex("color.copy_hex"),
        );

        assert!(outcome.update.is_some_and(|update| update.changed));
        assert!((state.hsv.saturation - 0.5).abs() < 0.001);
        assert!((state.hsv.value - 0.75).abs() < 0.001);

        let outcome = state.apply_action(
            "color.copy_hex",
            WidgetActionKind::Open,
            ColorPickerActionOptions::new("color").copy_hex("color.copy_hex"),
        );
        assert_eq!(
            outcome.effect,
            Some(ColorPickerEffect::CopyHex(format_hex_color(
                state.value,
                false
            )))
        );
    }
}
