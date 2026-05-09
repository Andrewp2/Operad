//! Renderer-neutral paint primitives for dense application and editor surfaces.

use crate::{ColorRgba, StrokeStyle, TextStyle, UiPoint, UiRect};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum StrokeAlignment {
    Inside,
    #[default]
    Center,
    Outside,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignedStroke {
    pub style: StrokeStyle,
    pub alignment: StrokeAlignment,
}

impl AlignedStroke {
    pub const fn new(style: StrokeStyle, alignment: StrokeAlignment) -> Self {
        Self { style, alignment }
    }

    pub const fn inside(style: StrokeStyle) -> Self {
        Self::new(style, StrokeAlignment::Inside)
    }

    pub const fn center(style: StrokeStyle) -> Self {
        Self::new(style, StrokeAlignment::Center)
    }

    pub const fn outside(style: StrokeStyle) -> Self {
        Self::new(style, StrokeAlignment::Outside)
    }
}

impl From<StrokeStyle> for AlignedStroke {
    fn from(style: StrokeStyle) -> Self {
        Self::center(style)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    pub offset: f32,
    pub color: ColorRgba,
}

impl GradientStop {
    pub fn new(offset: f32, color: ColorRgba) -> Self {
        Self {
            offset: offset.clamp(0.0, 1.0),
            color,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinearGradient {
    pub start: UiPoint,
    pub end: UiPoint,
    pub stops: Vec<GradientStop>,
    pub fallback: ColorRgba,
}

impl LinearGradient {
    pub fn new(start: UiPoint, end: UiPoint, from: ColorRgba, to: ColorRgba) -> Self {
        Self {
            start,
            end,
            stops: vec![GradientStop::new(0.0, from), GradientStop::new(1.0, to)],
            fallback: from,
        }
    }

    pub fn stop(mut self, offset: f32, color: ColorRgba) -> Self {
        self.stops.push(GradientStop::new(offset, color));
        self.stops.sort_by(|a, b| a.offset.total_cmp(&b.offset));
        self
    }

    pub const fn fallback(mut self, fallback: ColorRgba) -> Self {
        self.fallback = fallback;
        self
    }

    pub fn translated(mut self, offset: UiPoint) -> Self {
        self.start.x += offset.x;
        self.start.y += offset.y;
        self.end.x += offset.x;
        self.end.y += offset.y;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaintBrush {
    Solid(ColorRgba),
    LinearGradient(LinearGradient),
}

impl PaintBrush {
    pub const fn solid(color: ColorRgba) -> Self {
        Self::Solid(color)
    }

    pub fn linear_gradient(start: UiPoint, end: UiPoint, from: ColorRgba, to: ColorRgba) -> Self {
        Self::LinearGradient(LinearGradient::new(start, end, from, to))
    }

    pub const fn fallback_color(&self) -> ColorRgba {
        match self {
            Self::Solid(color) => *color,
            Self::LinearGradient(gradient) => gradient.fallback,
        }
    }

    pub const fn is_visible(&self) -> bool {
        self.fallback_color().a > 0
    }

    pub fn translated(&self, offset: UiPoint) -> Self {
        match self {
            Self::Solid(color) => Self::Solid(*color),
            Self::LinearGradient(gradient) => {
                Self::LinearGradient(gradient.clone().translated(offset))
            }
        }
    }
}

impl From<ColorRgba> for PaintBrush {
    fn from(color: ColorRgba) -> Self {
        Self::Solid(color)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    pub const ZERO: Self = Self::uniform(0.0);

    pub const fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    pub const fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    pub fn max_radius(self) -> f32 {
        self.top_left
            .max(self.top_right)
            .max(self.bottom_right)
            .max(self.bottom_left)
    }
}

impl Default for CornerRadii {
    fn default() -> Self {
        Self::ZERO
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaintEffectKind {
    Shadow,
    Glow,
    InsetShadow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaintEffect {
    pub kind: PaintEffectKind,
    pub color: ColorRgba,
    pub offset: UiPoint,
    pub blur_radius: f32,
    pub spread: f32,
}

impl PaintEffect {
    pub const fn shadow(color: ColorRgba, offset: UiPoint, blur_radius: f32, spread: f32) -> Self {
        Self {
            kind: PaintEffectKind::Shadow,
            color,
            offset,
            blur_radius,
            spread,
        }
    }

    pub const fn glow(color: ColorRgba, blur_radius: f32, spread: f32) -> Self {
        Self {
            kind: PaintEffectKind::Glow,
            color,
            offset: UiPoint::new(0.0, 0.0),
            blur_radius,
            spread,
        }
    }

    pub const fn inset_shadow(
        color: ColorRgba,
        offset: UiPoint,
        blur_radius: f32,
        spread: f32,
    ) -> Self {
        Self {
            kind: PaintEffectKind::InsetShadow,
            color,
            offset,
            blur_radius,
            spread,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaintRect {
    pub rect: UiRect,
    pub fill: PaintBrush,
    pub stroke: Option<AlignedStroke>,
    pub corner_radii: CornerRadii,
    pub effects: Vec<PaintEffect>,
}

impl PaintRect {
    pub fn new(rect: UiRect, fill: impl Into<PaintBrush>) -> Self {
        Self {
            rect,
            fill: fill.into(),
            stroke: None,
            corner_radii: CornerRadii::ZERO,
            effects: Vec::new(),
        }
    }

    pub fn solid(rect: UiRect, fill: ColorRgba) -> Self {
        Self::new(rect, fill)
    }

    pub fn stroke(mut self, stroke: impl Into<AlignedStroke>) -> Self {
        self.stroke = Some(stroke.into());
        self
    }

    pub const fn corner_radii(mut self, corner_radii: CornerRadii) -> Self {
        self.corner_radii = corner_radii;
        self
    }

    pub fn effect(mut self, effect: PaintEffect) -> Self {
        self.effects.push(effect);
        self
    }

    pub fn translated(mut self, offset: UiPoint) -> Self {
        self.rect.x += offset.x;
        self.rect.y += offset.y;
        self.fill = self.fill.translated(offset);
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextHorizontalAlign {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextVerticalAlign {
    #[default]
    Top,
    Center,
    Baseline,
    Bottom,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextOverflow {
    #[default]
    Clip,
    Ellipsis,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaintText {
    pub text: String,
    pub rect: UiRect,
    pub style: TextStyle,
    pub horizontal_align: TextHorizontalAlign,
    pub vertical_align: TextVerticalAlign,
    pub overflow: TextOverflow,
    pub multiline: bool,
}

impl PaintText {
    pub fn new(text: impl Into<String>, rect: UiRect, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            rect,
            style,
            horizontal_align: TextHorizontalAlign::Start,
            vertical_align: TextVerticalAlign::Top,
            overflow: TextOverflow::Clip,
            multiline: true,
        }
    }

    pub const fn horizontal_align(mut self, align: TextHorizontalAlign) -> Self {
        self.horizontal_align = align;
        self
    }

    pub const fn vertical_align(mut self, align: TextVerticalAlign) -> Self {
        self.vertical_align = align;
        self
    }

    pub const fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }

    pub const fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    pub fn translated(mut self, offset: UiPoint) -> Self {
        self.rect.x += offset.x;
        self.rect.y += offset.y;
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ImageFit {
    #[default]
    Fill,
    Contain,
    Cover,
    Original,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ImageAlignment {
    #[default]
    Center,
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaintImage {
    pub key: String,
    pub rect: UiRect,
    pub tint: Option<ColorRgba>,
    pub fit: ImageFit,
    pub horizontal_align: ImageAlignment,
    pub vertical_align: ImageAlignment,
}

impl PaintImage {
    pub fn new(key: impl Into<String>, rect: UiRect) -> Self {
        Self {
            key: key.into(),
            rect,
            tint: None,
            fit: ImageFit::Fill,
            horizontal_align: ImageAlignment::Center,
            vertical_align: ImageAlignment::Center,
        }
    }

    pub const fn tinted(mut self, tint: ColorRgba) -> Self {
        self.tint = Some(tint);
        self
    }

    pub const fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    pub const fn align(mut self, horizontal: ImageAlignment, vertical: ImageAlignment) -> Self {
        self.horizontal_align = horizontal;
        self.vertical_align = vertical;
        self
    }

    pub fn translated(mut self, offset: UiPoint) -> Self {
        self.rect.x += offset.x;
        self.rect.y += offset.y;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathVerb {
    MoveTo(UiPoint),
    LineTo(UiPoint),
    QuadraticTo {
        control: UiPoint,
        to: UiPoint,
    },
    CubicTo {
        control_a: UiPoint,
        control_b: UiPoint,
        to: UiPoint,
    },
    Close,
}

impl PathVerb {
    pub fn translated(self, offset: UiPoint) -> Self {
        match self {
            Self::MoveTo(point) => Self::MoveTo(translated_point(point, offset)),
            Self::LineTo(point) => Self::LineTo(translated_point(point, offset)),
            Self::QuadraticTo { control, to } => Self::QuadraticTo {
                control: translated_point(control, offset),
                to: translated_point(to, offset),
            },
            Self::CubicTo {
                control_a,
                control_b,
                to,
            } => Self::CubicTo {
                control_a: translated_point(control_a, offset),
                control_b: translated_point(control_b, offset),
                to: translated_point(to, offset),
            },
            Self::Close => Self::Close,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaintPath {
    pub verbs: Vec<PathVerb>,
    pub fill: Option<PaintBrush>,
    pub stroke: Option<AlignedStroke>,
}

impl PaintPath {
    pub fn new() -> Self {
        Self {
            verbs: Vec::new(),
            fill: None,
            stroke: None,
        }
    }

    pub fn move_to(mut self, point: UiPoint) -> Self {
        self.verbs.push(PathVerb::MoveTo(point));
        self
    }

    pub fn line_to(mut self, point: UiPoint) -> Self {
        self.verbs.push(PathVerb::LineTo(point));
        self
    }

    pub fn quadratic_to(mut self, control: UiPoint, to: UiPoint) -> Self {
        self.verbs.push(PathVerb::QuadraticTo { control, to });
        self
    }

    pub fn cubic_to(mut self, control_a: UiPoint, control_b: UiPoint, to: UiPoint) -> Self {
        self.verbs.push(PathVerb::CubicTo {
            control_a,
            control_b,
            to,
        });
        self
    }

    pub fn close(mut self) -> Self {
        self.verbs.push(PathVerb::Close);
        self
    }

    pub fn fill(mut self, fill: impl Into<PaintBrush>) -> Self {
        self.fill = Some(fill.into());
        self
    }

    pub fn stroke(mut self, stroke: impl Into<AlignedStroke>) -> Self {
        self.stroke = Some(stroke.into());
        self
    }

    pub fn translated(mut self, offset: UiPoint) -> Self {
        self.verbs = self
            .verbs
            .into_iter()
            .map(|verb| verb.translated(offset))
            .collect();
        if let Some(fill) = &self.fill {
            self.fill = Some(fill.translated(offset));
        }
        self
    }

    pub fn bounds(&self) -> UiRect {
        let mut points = Vec::new();
        for verb in &self.verbs {
            match *verb {
                PathVerb::MoveTo(point) | PathVerb::LineTo(point) => points.push(point),
                PathVerb::QuadraticTo { control, to } => {
                    points.push(control);
                    points.push(to);
                }
                PathVerb::CubicTo {
                    control_a,
                    control_b,
                    to,
                } => {
                    points.push(control_a);
                    points.push(control_b);
                    points.push(to);
                }
                PathVerb::Close => {}
            }
        }

        rect_from_points(&points)
    }
}

impl Default for PaintPath {
    fn default() -> Self {
        Self::new()
    }
}

fn translated_point(point: UiPoint, offset: UiPoint) -> UiPoint {
    UiPoint::new(point.x + offset.x, point.y + offset.y)
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
