//! Renderer-neutral paint primitives for dense application and editor surfaces.

use lyon_tessellation::{
    geometry_builder::{simple_builder, VertexBuffers},
    math::point as lyon_point,
    path::Path as LyonPath,
    FillOptions, FillRule as LyonFillRule, FillTessellator, LineCap as LyonLineCap,
    LineJoin as LyonLineJoin, StrokeOptions, StrokeTessellator,
};

use crate::{ColorRgba, StrokeStyle, TextStyle, UiPoint, UiRect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelSnapPolicy {
    pub scale_factor: f32,
}

impl PixelSnapPolicy {
    pub const DISABLED: Self = Self { scale_factor: 0.0 };

    pub fn new(scale_factor: f32) -> Self {
        if scale_factor.is_finite() && scale_factor > 0.0 {
            Self { scale_factor }
        } else {
            Self::DISABLED
        }
    }

    pub const fn disabled() -> Self {
        Self::DISABLED
    }

    pub const fn enabled(self) -> bool {
        self.scale_factor > 0.0
    }

    pub fn pixel_size(self) -> f32 {
        if self.enabled() {
            1.0 / self.scale_factor
        } else {
            0.0
        }
    }

    pub fn snap_value(self, value: f32) -> f32 {
        if !self.enabled() || !value.is_finite() {
            return value;
        }
        (value * self.scale_factor).round() / self.scale_factor
    }

    pub fn snap_center_value(self, value: f32) -> f32 {
        if !self.enabled() || !value.is_finite() {
            return value;
        }
        ((value * self.scale_factor).floor() + 0.5) / self.scale_factor
    }

    pub fn snap_point(self, point: UiPoint) -> UiPoint {
        UiPoint::new(self.snap_value(point.x), self.snap_value(point.y))
    }

    pub fn snap_center_point(self, point: UiPoint) -> UiPoint {
        UiPoint::new(
            self.snap_center_value(point.x),
            self.snap_center_value(point.y),
        )
    }

    pub fn snap_rect(self, rect: UiRect) -> UiRect {
        if !self.enabled() {
            return rect;
        }
        let left = self.snap_value(rect.x);
        let top = self.snap_value(rect.y);
        let right = self.snap_value(rect.right());
        let bottom = self.snap_value(rect.bottom());
        UiRect::new(left, top, (right - left).max(0.0), (bottom - top).max(0.0))
    }

    pub fn snap_line_segment(self, from: UiPoint, to: UiPoint) -> (UiPoint, UiPoint) {
        if (from.x - to.x).abs() <= f32::EPSILON {
            let x = self.snap_center_value(from.x);
            return (
                UiPoint::new(x, self.snap_value(from.y)),
                UiPoint::new(x, self.snap_value(to.y)),
            );
        }
        if (from.y - to.y).abs() <= f32::EPSILON {
            let y = self.snap_center_value(from.y);
            return (
                UiPoint::new(self.snap_value(from.x), y),
                UiPoint::new(self.snap_value(to.x), y),
            );
        }
        (self.snap_point(from), self.snap_point(to))
    }

    pub fn snap_stroke_width(self, width: f32) -> f32 {
        if !self.enabled() || !width.is_finite() || width <= 0.0 {
            return width;
        }
        ((width * self.scale_factor).ceil().max(1.0)) / self.scale_factor
    }

    pub fn snap_stroke(self, stroke: StrokeStyle) -> StrokeStyle {
        StrokeStyle::new(stroke.color, self.snap_stroke_width(stroke.width))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum StrokeAlignment {
    Inside,
    #[default]
    Center,
    Outside,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum StrokeLineCap {
    Butt,
    Square,
    #[default]
    Round,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum StrokeLineJoin {
    Miter,
    Bevel,
    #[default]
    Round,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathStrokeOptions {
    pub line_cap: StrokeLineCap,
    pub line_join: StrokeLineJoin,
    pub miter_limit: f32,
}

impl PathStrokeOptions {
    pub const DEFAULT_MITER_LIMIT: f32 = 4.0;

    pub const fn new() -> Self {
        Self {
            line_cap: StrokeLineCap::Round,
            line_join: StrokeLineJoin::Round,
            miter_limit: Self::DEFAULT_MITER_LIMIT,
        }
    }

    pub const fn line_cap(mut self, line_cap: StrokeLineCap) -> Self {
        self.line_cap = line_cap;
        self
    }

    pub const fn line_join(mut self, line_join: StrokeLineJoin) -> Self {
        self.line_join = line_join;
        self
    }

    pub const fn miter_limit(mut self, miter_limit: f32) -> Self {
        self.miter_limit = miter_limit;
        self
    }
}

impl Default for PathStrokeOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PathFillRule {
    NonZero,
    #[default]
    EvenOdd,
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

    pub fn pixel_snapped(mut self, policy: PixelSnapPolicy) -> Self {
        self.rect = policy.snap_rect(self.rect);
        if let Some(stroke) = self.stroke {
            self.stroke = Some(AlignedStroke {
                style: policy.snap_stroke(stroke.style),
                alignment: stroke.alignment,
            });
        }
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

    pub fn pixel_snapped(self, policy: PixelSnapPolicy) -> Self {
        match self {
            Self::MoveTo(point) => Self::MoveTo(policy.snap_point(point)),
            Self::LineTo(point) => Self::LineTo(policy.snap_point(point)),
            Self::QuadraticTo { control, to } => Self::QuadraticTo {
                control: policy.snap_point(control),
                to: policy.snap_point(to),
            },
            Self::CubicTo {
                control_a,
                control_b,
                to,
            } => Self::CubicTo {
                control_a: policy.snap_point(control_a),
                control_b: policy.snap_point(control_b),
                to: policy.snap_point(to),
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
    pub stroke_options: PathStrokeOptions,
    pub fill_rule: PathFillRule,
}

impl PaintPath {
    pub fn new() -> Self {
        Self {
            verbs: Vec::new(),
            fill: None,
            stroke: None,
            stroke_options: PathStrokeOptions::default(),
            fill_rule: PathFillRule::default(),
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

    pub const fn fill_rule(mut self, fill_rule: PathFillRule) -> Self {
        self.fill_rule = fill_rule;
        self
    }

    pub fn stroke(mut self, stroke: impl Into<AlignedStroke>) -> Self {
        self.stroke = Some(stroke.into());
        self
    }

    pub const fn stroke_options(mut self, options: PathStrokeOptions) -> Self {
        self.stroke_options = options;
        self
    }

    pub const fn line_cap(mut self, line_cap: StrokeLineCap) -> Self {
        self.stroke_options.line_cap = line_cap;
        self
    }

    pub const fn line_join(mut self, line_join: StrokeLineJoin) -> Self {
        self.stroke_options.line_join = line_join;
        self
    }

    pub const fn miter_limit(mut self, miter_limit: f32) -> Self {
        self.stroke_options.miter_limit = miter_limit;
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

    pub fn pixel_snapped(mut self, policy: PixelSnapPolicy) -> Self {
        self.verbs = self
            .verbs
            .into_iter()
            .map(|verb| verb.pixel_snapped(policy))
            .collect();
        if let Some(stroke) = self.stroke {
            self.stroke = Some(AlignedStroke {
                style: policy.snap_stroke(stroke.style),
                alignment: stroke.alignment,
            });
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

    pub fn flattened_points(&self, tolerance: f32) -> Vec<UiPoint> {
        self.flattened_contours(tolerance)
            .into_iter()
            .flatten()
            .collect()
    }

    pub fn flattened_contours(&self, tolerance: f32) -> Vec<Vec<UiPoint>> {
        let tolerance = if tolerance.is_finite() && tolerance > 0.0 {
            tolerance
        } else {
            1.0
        };
        let mut contours = Vec::<Vec<UiPoint>>::new();
        let mut points = Vec::<UiPoint>::new();
        let mut current = None;
        let mut contour_start = None;
        for verb in &self.verbs {
            match *verb {
                PathVerb::MoveTo(point) => {
                    if !points.is_empty() {
                        contours.push(std::mem::take(&mut points));
                    }
                    points.push(point);
                    current = Some(point);
                    contour_start = Some(point);
                }
                PathVerb::LineTo(point) => {
                    points.push(point);
                    current = Some(point);
                }
                PathVerb::QuadraticTo { control, to } => {
                    let Some(from) = current else {
                        points.push(to);
                        current = Some(to);
                        contour_start.get_or_insert(to);
                        continue;
                    };
                    let segments = quadratic_segments(from, control, to, tolerance);
                    for index in 1..=segments {
                        let t = index as f32 / segments as f32;
                        points.push(quadratic_point(from, control, to, t));
                    }
                    current = Some(to);
                }
                PathVerb::CubicTo {
                    control_a,
                    control_b,
                    to,
                } => {
                    let Some(from) = current else {
                        points.push(to);
                        current = Some(to);
                        contour_start.get_or_insert(to);
                        continue;
                    };
                    let segments = cubic_segments(from, control_a, control_b, to, tolerance);
                    for index in 1..=segments {
                        let t = index as f32 / segments as f32;
                        points.push(cubic_point(from, control_a, control_b, to, t));
                    }
                    current = Some(to);
                }
                PathVerb::Close => {
                    if let (Some(start), Some(last)) = (contour_start, current) {
                        if start != last {
                            points.push(start);
                        }
                    }
                    if !points.is_empty() {
                        contours.push(std::mem::take(&mut points));
                    }
                    current = contour_start;
                    contour_start = None;
                }
            }
        }
        if !points.is_empty() {
            contours.push(points);
        }
        contours
    }

    pub fn is_closed(&self) -> bool {
        self.verbs
            .iter()
            .any(|verb| matches!(verb, PathVerb::Close))
    }

    pub fn tessellated_fill(&self, tolerance: f32) -> Vec<[UiPoint; 3]> {
        let path = self.to_lyon_path();
        let mut buffers: VertexBuffers<lyon_tessellation::math::Point, u16> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();
        let options = FillOptions::tolerance(finite_positive_or(tolerance, 1.0)).with_fill_rule(
            match self.fill_rule {
                PathFillRule::NonZero => LyonFillRule::NonZero,
                PathFillRule::EvenOdd => LyonFillRule::EvenOdd,
            },
        );
        if tessellator
            .tessellate_path(&path, &options, &mut simple_builder(&mut buffers))
            .is_err()
        {
            return tessellate_polygon(&self.flattened_points(tolerance));
        }
        vertex_buffers_to_triangles(buffers)
    }

    pub fn tessellated_stroke(&self, tolerance: f32) -> Vec<[UiPoint; 3]> {
        let Some(stroke) = self.stroke else {
            return Vec::new();
        };
        let path = self.to_lyon_path();
        let mut buffers: VertexBuffers<lyon_tessellation::math::Point, u16> = VertexBuffers::new();
        let mut tessellator = StrokeTessellator::new();
        let options = StrokeOptions::tolerance(finite_positive_or(tolerance, 1.0))
            .with_line_width(stroke.style.width.max(1.0))
            .with_line_cap(match self.stroke_options.line_cap {
                StrokeLineCap::Butt => LyonLineCap::Butt,
                StrokeLineCap::Square => LyonLineCap::Square,
                StrokeLineCap::Round => LyonLineCap::Round,
            })
            .with_line_join(match self.stroke_options.line_join {
                StrokeLineJoin::Miter => LyonLineJoin::Miter,
                StrokeLineJoin::Bevel => LyonLineJoin::Bevel,
                StrokeLineJoin::Round => LyonLineJoin::Round,
            })
            .with_miter_limit(
                finite_positive_or(
                    self.stroke_options.miter_limit,
                    PathStrokeOptions::DEFAULT_MITER_LIMIT,
                )
                .max(StrokeOptions::MINIMUM_MITER_LIMIT),
            );
        if tessellator
            .tessellate_path(&path, &options, &mut simple_builder(&mut buffers))
            .is_err()
        {
            return tessellate_polyline_stroke(
                &self.flattened_points(tolerance),
                stroke.style,
                self.stroke_options,
                self.is_closed(),
            );
        }
        vertex_buffers_to_triangles(buffers)
    }

    fn to_lyon_path(&self) -> LyonPath {
        let mut builder = LyonPath::builder().with_svg();
        for verb in &self.verbs {
            match *verb {
                PathVerb::MoveTo(point) => {
                    builder.move_to(to_lyon_point(point));
                }
                PathVerb::LineTo(point) => {
                    builder.line_to(to_lyon_point(point));
                }
                PathVerb::QuadraticTo { control, to } => {
                    builder.quadratic_bezier_to(to_lyon_point(control), to_lyon_point(to));
                }
                PathVerb::CubicTo {
                    control_a,
                    control_b,
                    to,
                } => {
                    builder.cubic_bezier_to(
                        to_lyon_point(control_a),
                        to_lyon_point(control_b),
                        to_lyon_point(to),
                    );
                }
                PathVerb::Close => {
                    builder.close();
                }
            }
        }
        builder.build()
    }
}

impl Default for PaintPath {
    fn default() -> Self {
        Self::new()
    }
}

fn to_lyon_point(point: UiPoint) -> lyon_tessellation::math::Point {
    lyon_point(point.x, point.y)
}

fn from_lyon_point(point: lyon_tessellation::math::Point) -> UiPoint {
    UiPoint::new(point.x, point.y)
}

fn vertex_buffers_to_triangles(
    buffers: VertexBuffers<lyon_tessellation::math::Point, u16>,
) -> Vec<[UiPoint; 3]> {
    buffers
        .indices
        .chunks_exact(3)
        .filter_map(|indices| {
            let a = buffers.vertices.get(usize::from(indices[0]))?;
            let b = buffers.vertices.get(usize::from(indices[1]))?;
            let c = buffers.vertices.get(usize::from(indices[2]))?;
            Some([
                from_lyon_point(*a),
                from_lyon_point(*b),
                from_lyon_point(*c),
            ])
        })
        .collect()
}

fn tessellate_polygon(points: &[UiPoint]) -> Vec<[UiPoint; 3]> {
    let mut polygon = sanitize_polygon(points);
    if polygon.len() < 3 {
        return Vec::new();
    }
    if signed_area(&polygon) < 0.0 {
        polygon.reverse();
    }

    let mut indices = (0..polygon.len()).collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity(polygon.len().saturating_sub(2));
    let mut guard = 0usize;
    while indices.len() > 3 && guard < polygon.len().saturating_mul(polygon.len()) {
        guard += 1;
        let mut clipped = false;
        for index in 0..indices.len() {
            let previous = indices[(index + indices.len() - 1) % indices.len()];
            let current = indices[index];
            let next = indices[(index + 1) % indices.len()];
            let a = polygon[previous];
            let b = polygon[current];
            let c = polygon[next];
            if cross(sub_points(b, a), sub_points(c, b)) <= 0.0 {
                continue;
            }
            if indices.iter().copied().any(|candidate| {
                candidate != previous
                    && candidate != current
                    && candidate != next
                    && point_in_triangle(polygon[candidate], a, b, c)
            }) {
                continue;
            }
            triangles.push([a, b, c]);
            indices.remove(index);
            clipped = true;
            break;
        }
        if !clipped {
            return polygon_fan_triangles(&polygon);
        }
    }

    if indices.len() == 3 {
        triangles.push([
            polygon[indices[0]],
            polygon[indices[1]],
            polygon[indices[2]],
        ]);
    }
    triangles
}

fn tessellate_polyline_stroke(
    points: &[UiPoint],
    stroke: StrokeStyle,
    options: PathStrokeOptions,
    closed: bool,
) -> Vec<[UiPoint; 3]> {
    if stroke.color.a == 0 {
        return Vec::new();
    }
    let points = sanitize_polyline(points);
    if points.is_empty() {
        return Vec::new();
    }
    let width = stroke.width.max(1.0);
    let half = width * 0.5 + 0.75;
    if points.len() == 1 {
        return circle_triangles(points[0], half);
    }

    let mut triangles = Vec::new();
    let segment_count = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    let mut directions = Vec::with_capacity(segment_count);
    let mut normals = Vec::with_capacity(segment_count);
    for index in 0..segment_count {
        let from = points[index];
        let to = points[(index + 1) % points.len()];
        let direction = normalize(sub_points(to, from));
        directions.push(direction);
        normals.push(UiPoint::new(-direction.y, direction.x));
    }

    for index in 0..segment_count {
        let mut from = points[index];
        let mut to = points[(index + 1) % points.len()];
        if !closed {
            if index == 0 && options.line_cap == StrokeLineCap::Square {
                from = add_points(from, scale_point(directions[index], -half));
            }
            if index == segment_count - 1 && options.line_cap == StrokeLineCap::Square {
                to = add_points(to, scale_point(directions[index], half));
            }
        }
        push_stroke_quad(&mut triangles, from, to, normals[index], half);
    }

    if closed {
        for index in 0..points.len() {
            let previous = (index + segment_count - 1) % segment_count;
            let next = index % segment_count;
            push_join_triangles(
                &mut triangles,
                points[index],
                normals[previous],
                normals[next],
                half,
                options,
            );
        }
    } else {
        if options.line_cap == StrokeLineCap::Round {
            triangles.extend(circle_triangles(points[0], half));
            triangles.extend(circle_triangles(*points.last().unwrap(), half));
        }
        for index in 1..points.len() - 1 {
            push_join_triangles(
                &mut triangles,
                points[index],
                normals[index - 1],
                normals[index],
                half,
                options,
            );
        }
    }

    triangles
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

fn point_distance(left: UiPoint, right: UiPoint) -> f32 {
    let dx = right.x - left.x;
    let dy = right.y - left.y;
    (dx * dx + dy * dy).sqrt()
}

fn quadratic_segments(from: UiPoint, control: UiPoint, to: UiPoint, tolerance: f32) -> usize {
    let length = point_distance(from, control) + point_distance(control, to);
    ((length / tolerance).ceil() as usize).clamp(4, 64)
}

fn cubic_segments(
    from: UiPoint,
    control_a: UiPoint,
    control_b: UiPoint,
    to: UiPoint,
    tolerance: f32,
) -> usize {
    let length = point_distance(from, control_a)
        + point_distance(control_a, control_b)
        + point_distance(control_b, to);
    ((length / tolerance).ceil() as usize).clamp(6, 96)
}

fn quadratic_point(from: UiPoint, control: UiPoint, to: UiPoint, t: f32) -> UiPoint {
    let inverse = 1.0 - t;
    UiPoint::new(
        inverse * inverse * from.x + 2.0 * inverse * t * control.x + t * t * to.x,
        inverse * inverse * from.y + 2.0 * inverse * t * control.y + t * t * to.y,
    )
}

fn cubic_point(
    from: UiPoint,
    control_a: UiPoint,
    control_b: UiPoint,
    to: UiPoint,
    t: f32,
) -> UiPoint {
    let inverse = 1.0 - t;
    UiPoint::new(
        inverse * inverse * inverse * from.x
            + 3.0 * inverse * inverse * t * control_a.x
            + 3.0 * inverse * t * t * control_b.x
            + t * t * t * to.x,
        inverse * inverse * inverse * from.y
            + 3.0 * inverse * inverse * t * control_a.y
            + 3.0 * inverse * t * t * control_b.y
            + t * t * t * to.y,
    )
}

fn sanitize_polygon(points: &[UiPoint]) -> Vec<UiPoint> {
    let mut clean = sanitize_polyline(points);
    if clean.len() > 1 && clean.first() == clean.last() {
        clean.pop();
    }
    clean
}

fn sanitize_polyline(points: &[UiPoint]) -> Vec<UiPoint> {
    let mut clean = Vec::with_capacity(points.len());
    for point in points.iter().copied() {
        if point.x.is_finite() && point.y.is_finite() && clean.last() != Some(&point) {
            clean.push(point);
        }
    }
    clean
}

fn polygon_fan_triangles(points: &[UiPoint]) -> Vec<[UiPoint; 3]> {
    if points.len() < 3 {
        return Vec::new();
    }
    let mut triangles = Vec::with_capacity(points.len().saturating_sub(2));
    for index in 1..points.len() - 1 {
        triangles.push([points[0], points[index], points[index + 1]]);
    }
    triangles
}

fn signed_area(points: &[UiPoint]) -> f32 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    area * 0.5
}

fn point_in_triangle(point: UiPoint, a: UiPoint, b: UiPoint, c: UiPoint) -> bool {
    let ab = cross(sub_points(b, a), sub_points(point, a));
    let bc = cross(sub_points(c, b), sub_points(point, b));
    let ca = cross(sub_points(a, c), sub_points(point, c));
    (ab >= -f32::EPSILON && bc >= -f32::EPSILON && ca >= -f32::EPSILON)
        || (ab <= f32::EPSILON && bc <= f32::EPSILON && ca <= f32::EPSILON)
}

fn push_stroke_quad(
    triangles: &mut Vec<[UiPoint; 3]>,
    from: UiPoint,
    to: UiPoint,
    normal: UiPoint,
    half_width: f32,
) {
    let offset = scale_point(normal, half_width);
    let a = add_points(from, offset);
    let b = add_points(to, offset);
    let c = sub_points(to, offset);
    let d = sub_points(from, offset);
    triangles.push([a, b, c]);
    triangles.push([a, c, d]);
}

fn push_join_triangles(
    triangles: &mut Vec<[UiPoint; 3]>,
    point: UiPoint,
    previous_normal: UiPoint,
    next_normal: UiPoint,
    half_width: f32,
    options: PathStrokeOptions,
) {
    match options.line_join {
        StrokeLineJoin::Round => triangles.extend(circle_triangles(point, half_width)),
        StrokeLineJoin::Bevel => {
            push_bevel_join(triangles, point, previous_normal, next_normal, half_width);
        }
        StrokeLineJoin::Miter => {
            if !push_miter_join(
                triangles,
                point,
                previous_normal,
                next_normal,
                half_width,
                options.miter_limit,
            ) {
                push_bevel_join(triangles, point, previous_normal, next_normal, half_width);
            }
        }
    }
}

fn push_bevel_join(
    triangles: &mut Vec<[UiPoint; 3]>,
    point: UiPoint,
    previous_normal: UiPoint,
    next_normal: UiPoint,
    half_width: f32,
) {
    triangles.push([
        point,
        add_points(point, scale_point(previous_normal, half_width)),
        add_points(point, scale_point(next_normal, half_width)),
    ]);
    triangles.push([
        point,
        sub_points(point, scale_point(previous_normal, half_width)),
        sub_points(point, scale_point(next_normal, half_width)),
    ]);
}

fn push_miter_join(
    triangles: &mut Vec<[UiPoint; 3]>,
    point: UiPoint,
    previous_normal: UiPoint,
    next_normal: UiPoint,
    half_width: f32,
    miter_limit: f32,
) -> bool {
    let Some(miter) = try_miter(previous_normal, next_normal, half_width, miter_limit) else {
        return false;
    };
    let previous = add_points(point, scale_point(previous_normal, half_width));
    let next = add_points(point, scale_point(next_normal, half_width));
    let tip = add_points(point, miter);
    triangles.push([previous, tip, next]);

    let previous = sub_points(point, scale_point(previous_normal, half_width));
    let next = sub_points(point, scale_point(next_normal, half_width));
    let tip = sub_points(point, miter);
    triangles.push([previous, next, tip]);
    true
}

fn try_miter(
    previous_normal: UiPoint,
    next_normal: UiPoint,
    half_width: f32,
    miter_limit: f32,
) -> Option<UiPoint> {
    let sum = add_points(previous_normal, next_normal);
    let miter = normalize(sum);
    if vector_length(miter) <= f32::EPSILON {
        return None;
    }
    let denominator = dot(miter, next_normal);
    if denominator.abs() <= 0.01 {
        return None;
    }
    let length = half_width / denominator;
    let max_length =
        half_width * finite_positive_or(miter_limit, PathStrokeOptions::DEFAULT_MITER_LIMIT);
    if length.abs() > max_length {
        return None;
    }
    Some(scale_point(miter, length))
}

fn circle_triangles(center: UiPoint, radius: f32) -> Vec<[UiPoint; 3]> {
    if radius <= 0.0 {
        return Vec::new();
    }
    let segments = ((radius * 4.0).ceil() as usize).clamp(12, 48);
    let mut triangles = Vec::with_capacity(segments);
    for index in 0..segments {
        let a0 = std::f32::consts::TAU * index as f32 / segments as f32;
        let a1 = std::f32::consts::TAU * (index + 1) as f32 / segments as f32;
        triangles.push([
            center,
            UiPoint::new(center.x + radius * a0.cos(), center.y + radius * a0.sin()),
            UiPoint::new(center.x + radius * a1.cos(), center.y + radius * a1.sin()),
        ]);
    }
    triangles
}

fn add_points(left: UiPoint, right: UiPoint) -> UiPoint {
    UiPoint::new(left.x + right.x, left.y + right.y)
}

fn sub_points(left: UiPoint, right: UiPoint) -> UiPoint {
    UiPoint::new(left.x - right.x, left.y - right.y)
}

fn scale_point(point: UiPoint, scale: f32) -> UiPoint {
    UiPoint::new(point.x * scale, point.y * scale)
}

fn dot(left: UiPoint, right: UiPoint) -> f32 {
    left.x * right.x + left.y * right.y
}

fn cross(left: UiPoint, right: UiPoint) -> f32 {
    left.x * right.y - left.y * right.x
}

fn vector_length(point: UiPoint) -> f32 {
    (point.x * point.x + point.y * point.y).sqrt()
}

fn normalize(point: UiPoint) -> UiPoint {
    let length = vector_length(point);
    if length <= f32::EPSILON {
        UiPoint::new(0.0, 0.0)
    } else {
        UiPoint::new(point.x / length, point.y / length)
    }
}

fn finite_positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_snap_policy_maps_values_rects_and_hairline_segments() {
        let policy = PixelSnapPolicy::new(2.0);

        assert!(policy.enabled());
        assert_eq!(policy.pixel_size(), 0.5);
        assert_eq!(policy.snap_value(10.26), 10.5);
        assert_eq!(policy.snap_center_value(10.26), 10.25);
        assert_eq!(
            policy.snap_point(UiPoint::new(0.24, 0.26)),
            UiPoint::new(0.0, 0.5)
        );
        assert_eq!(
            policy.snap_rect(UiRect::new(0.24, 0.26, 10.51, 4.49)),
            UiRect::new(0.0, 0.5, 11.0, 4.5)
        );

        let (from, to) = PixelSnapPolicy::new(1.0)
            .snap_line_segment(UiPoint::new(10.1, 0.2), UiPoint::new(10.1, 9.8));
        assert_eq!(from, UiPoint::new(10.5, 0.0));
        assert_eq!(to, UiPoint::new(10.5, 10.0));

        let (from, to) = PixelSnapPolicy::new(1.0)
            .snap_line_segment(UiPoint::new(0.2, 5.1), UiPoint::new(9.8, 5.1));
        assert_eq!(from, UiPoint::new(0.0, 5.5));
        assert_eq!(to, UiPoint::new(10.0, 5.5));
    }

    #[test]
    fn pixel_snap_policy_preserves_disabled_and_snaps_stroke_widths_up() {
        let disabled = PixelSnapPolicy::disabled();
        assert!(!disabled.enabled());
        assert_eq!(disabled.snap_value(10.26), 10.26);
        assert_eq!(PixelSnapPolicy::new(f32::NAN), PixelSnapPolicy::DISABLED);

        let policy = PixelSnapPolicy::new(2.0);
        assert_eq!(policy.snap_stroke_width(0.1), 0.5);
        assert_eq!(policy.snap_stroke_width(1.2), 1.5);
        assert_eq!(policy.snap_stroke_width(0.0), 0.0);
    }

    #[test]
    fn paint_rect_and_path_can_be_pixel_snapped() {
        let policy = PixelSnapPolicy::new(2.0);
        let rect = PaintRect::solid(UiRect::new(1.24, 2.26, 10.51, 4.49), ColorRgba::WHITE)
            .stroke(AlignedStroke::inside(StrokeStyle::new(
                ColorRgba::WHITE,
                0.3,
            )))
            .pixel_snapped(policy);

        assert_eq!(rect.rect, UiRect::new(1.0, 2.5, 11.0, 4.5));
        assert_eq!(rect.stroke.unwrap().style.width, 0.5);

        let path = PaintPath::new()
            .move_to(UiPoint::new(0.24, 0.26))
            .line_to(UiPoint::new(4.74, 3.24))
            .stroke(StrokeStyle::new(ColorRgba::WHITE, 0.2))
            .pixel_snapped(policy);

        assert_eq!(
            path.verbs,
            vec![
                PathVerb::MoveTo(UiPoint::new(0.0, 0.5)),
                PathVerb::LineTo(UiPoint::new(4.5, 3.0))
            ]
        );
        assert_eq!(path.stroke.unwrap().style.width, 0.5);
    }

    #[test]
    fn paint_path_flattens_quadratic_and_cubic_curves() {
        let path = PaintPath::new()
            .move_to(UiPoint::new(0.0, 10.0))
            .quadratic_to(UiPoint::new(10.0, 0.0), UiPoint::new(20.0, 10.0))
            .cubic_to(
                UiPoint::new(28.0, 18.0),
                UiPoint::new(34.0, 18.0),
                UiPoint::new(40.0, 10.0),
            );

        let points = path.flattened_points(4.0);

        assert!(points.len() > 6);
        assert_eq!(points.first(), Some(&UiPoint::new(0.0, 10.0)));
        assert_eq!(points.last(), Some(&UiPoint::new(40.0, 10.0)));
        assert!(
            points.iter().any(|point| point.y < 8.0),
            "quadratic control point should affect flattened curve"
        );
        assert!(
            points.iter().any(|point| point.y > 12.0),
            "cubic control points should affect flattened curve"
        );
    }

    #[test]
    fn paint_path_preserves_contours_and_stroke_options() {
        let path = PaintPath::new()
            .move_to(UiPoint::new(0.0, 0.0))
            .line_to(UiPoint::new(8.0, 0.0))
            .move_to(UiPoint::new(0.0, 8.0))
            .line_to(UiPoint::new(8.0, 8.0))
            .stroke(StrokeStyle::new(ColorRgba::WHITE, 2.0))
            .line_cap(StrokeLineCap::Butt)
            .line_join(StrokeLineJoin::Miter)
            .miter_limit(2.0);

        let contours = path.flattened_contours(1.0);
        assert_eq!(contours.len(), 2);
        assert_eq!(path.stroke_options.line_cap, StrokeLineCap::Butt);
        assert_eq!(path.stroke_options.line_join, StrokeLineJoin::Miter);
        assert_eq!(path.stroke_options.miter_limit, 2.0);
    }

    #[test]
    fn tessellators_cover_non_convex_fill_and_configurable_strokes() {
        let polygon = [
            UiPoint::new(0.0, 0.0),
            UiPoint::new(16.0, 0.0),
            UiPoint::new(16.0, 16.0),
            UiPoint::new(8.0, 8.0),
            UiPoint::new(0.0, 16.0),
        ];
        assert!(tessellate_polygon(&polygon).len() >= 3);

        let polyline = [
            UiPoint::new(0.0, 0.0),
            UiPoint::new(12.0, 0.0),
            UiPoint::new(12.0, 12.0),
        ];
        let butt = tessellate_polyline_stroke(
            &polyline,
            StrokeStyle::new(ColorRgba::WHITE, 3.0),
            PathStrokeOptions::new().line_cap(StrokeLineCap::Butt),
            false,
        );
        let round = tessellate_polyline_stroke(
            &polyline,
            StrokeStyle::new(ColorRgba::WHITE, 3.0),
            PathStrokeOptions::new()
                .line_cap(StrokeLineCap::Round)
                .line_join(StrokeLineJoin::Round),
            false,
        );
        assert!(round.len() > butt.len(), "round={round:?} butt={butt:?}");
    }
}
