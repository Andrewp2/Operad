//! Timeline ruler widget.

use taffy::prelude::{Dimension, LengthPercentageAuto, Position, Rect, Size as TaffySize, Style};

use crate::{
    length, AccessibilityMeta, AccessibilityRole, AccessibilityValueRange, ClipBehavior, ColorRgba,
    LayoutStyle, ScenePrimitive, ShaderEffect, StrokeStyle, TextStyle, UiDocument, UiNode,
    UiNodeId, UiNodeStyle, UiPoint, UiVisual,
};

use super::surfaces::DEFAULT_SURFACE_STROKE;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRange {
    pub start: f64,
    pub end: f64,
}

impl TimelineRange {
    pub const fn new(start: f64, end: f64) -> Self {
        Self { start, end }
    }

    pub fn ordered(self) -> Self {
        if !self.start.is_finite() || !self.end.is_finite() {
            return Self {
                start: 0.0,
                end: 0.0,
            };
        }
        if self.start <= self.end {
            self
        } else {
            Self {
                start: self.end,
                end: self.start,
            }
        }
    }

    pub fn duration(self) -> f64 {
        let ordered = self.ordered();
        (ordered.end - ordered.start).max(0.0)
    }

    pub fn contains(self, value: f64) -> bool {
        let ordered = self.ordered();
        value.is_finite() && value >= ordered.start && value <= ordered.end
    }

    pub fn normalized(self, value: f64) -> f32 {
        let ordered = self.ordered();
        let duration = ordered.duration();
        if duration <= f64::EPSILON {
            return 0.0;
        }
        if !value.is_finite() {
            return 0.0;
        }
        ((value - ordered.start) / duration).clamp(0.0, 1.0) as f32
    }

    pub fn value_to_x(self, value: f64, width: f32) -> f32 {
        let width = if width.is_finite() {
            width.max(0.0)
        } else {
            0.0
        };
        self.normalized(value) * width
    }

    pub fn x_to_value(self, x: f32, width: f32) -> f64 {
        let ordered = self.ordered();
        let width = if width.is_finite() && width > f32::EPSILON {
            width
        } else {
            1.0
        };
        let x = if x.is_finite() {
            x.clamp(0.0, width)
        } else {
            0.0
        };
        ordered.start + ordered.duration() * (x as f64 / width as f64)
    }

    pub fn pan(self, delta: f64) -> Self {
        if !delta.is_finite() {
            return self;
        }
        Self {
            start: self.start + delta,
            end: self.end + delta,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulerTickKind {
    Major,
    Minor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RulerTick {
    pub value: f64,
    pub x: f32,
    pub kind: RulerTickKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RulerSpec {
    pub range: TimelineRange,
    pub width: f32,
    pub major_step: f64,
    pub minor_step: f64,
    pub label_every: usize,
}

impl RulerSpec {
    pub fn ticks(self) -> Vec<RulerTick> {
        let range = self.range.ordered();
        if range.duration() <= f64::EPSILON
            || !self.width.is_finite()
            || self.width <= f32::EPSILON
            || !self.major_step.is_finite()
            || !self.minor_step.is_finite()
            || self.major_step <= f64::EPSILON
            || self.minor_step <= f64::EPSILON
        {
            return Vec::new();
        }
        let start_index = (range.start / self.minor_step).ceil() as i64;
        let end_index = (range.end / self.minor_step).floor() as i64;
        let label_every = self.label_every.max(1);
        let mut major_count = 0_usize;
        let mut ticks = Vec::new();
        for index in start_index..=end_index {
            if ticks.len() >= 10_000 {
                break;
            }
            let value = index as f64 * self.minor_step;
            let major_ratio = value / self.major_step;
            let is_major = (major_ratio - major_ratio.round()).abs() < 0.000_001;
            let label = if is_major {
                let should_label = major_count % label_every == 0;
                major_count += 1;
                should_label.then(|| format_ruler_label(value))
            } else {
                None
            };
            ticks.push(RulerTick {
                value,
                x: self.range.value_to_x(value, self.width),
                kind: if is_major {
                    RulerTickKind::Major
                } else {
                    RulerTickKind::Minor
                },
                label,
            });
        }
        ticks
    }
}

fn format_ruler_label(value: f64) -> String {
    if value.fract().abs() < 0.000_001 {
        return format!("{}", value.round() as i64);
    }
    let mut label = format!("{value:.3}");
    while label.contains('.') && label.ends_with('0') {
        label.pop();
    }
    if label.ends_with('.') {
        label.pop();
    }
    label
}

#[derive(Debug, Clone)]
pub struct TimelineRulerOptions {
    pub layout: LayoutStyle,
    pub height: f32,
    pub background_visual: UiVisual,
    pub major_stroke: StrokeStyle,
    pub minor_stroke: StrokeStyle,
    pub label_style: TextStyle,
    pub shader: Option<ShaderEffect>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for TimelineRulerOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(32.0),
                },
                ..Default::default()
            }),
            height: 32.0,
            background_visual: UiVisual::panel(
                ColorRgba::new(20, 24, 30, 255),
                Some(StrokeStyle::new(DEFAULT_SURFACE_STROKE, 1.0)),
                0.0,
            ),
            major_stroke: StrokeStyle::new(ColorRgba::new(180, 190, 205, 255), 1.0),
            minor_stroke: StrokeStyle::new(ColorRgba::new(86, 98, 116, 255), 1.0),
            label_style: TextStyle {
                font_size: 11.0,
                line_height: 14.0,
                color: ColorRgba::new(218, 226, 238, 255),
                ..Default::default()
            },
            shader: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

pub fn timeline_ruler(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    spec: RulerSpec,
    options: TimelineRulerOptions,
) -> UiNodeId {
    let name = name.into();
    let mut layout = options.layout;
    let height = if options.height.is_finite() {
        options.height.max(0.0)
    } else {
        0.0
    };
    let scene_width = if spec.width.is_finite() {
        spec.width.max(0.0)
    } else {
        0.0
    };
    layout.as_taffy_style_mut().size.height = length(height);
    let range = spec.range.ordered();
    let mut root_node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: layout.style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_visual(options.background_visual)
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Slider)
            .label(
                options
                    .accessibility_label
                    .clone()
                    .unwrap_or_else(|| format!("{name} timeline ruler")),
            )
            .value(format!(
                "{} to {}",
                format_ruler_label(range.start),
                format_ruler_label(range.end)
            ))
            .value_range(AccessibilityValueRange::new(range.start, range.end))
            .hint(
                options
                    .accessibility_hint
                    .clone()
                    .unwrap_or_else(|| "Shows timeline tick marks and labels".to_string()),
            ),
    );
    if let Some(shader) = &options.shader {
        root_node = root_node.with_shader(shader.clone());
    }
    let root = document.add_child(parent, root_node);

    let ticks = spec.ticks();
    let primitives = ticks
        .iter()
        .map(|tick| {
            let tick_height = match tick.kind {
                RulerTickKind::Major => height,
                RulerTickKind::Minor => height * 0.5,
            };
            ScenePrimitive::Line {
                from: UiPoint::new(tick.x, height),
                to: UiPoint::new(tick.x, height - tick_height),
                stroke: match tick.kind {
                    RulerTickKind::Major => options.major_stroke,
                    RulerTickKind::Minor => options.minor_stroke,
                },
            }
        })
        .collect::<Vec<_>>();
    document.add_child(
        root,
        UiNode::scene(
            format!("{name}.ticks"),
            primitives,
            LayoutStyle::from_taffy_style(Style {
                position: Position::Absolute,
                size: TaffySize {
                    width: length(scene_width),
                    height: length(height),
                },
                ..Default::default()
            }),
        ),
    );

    for tick in ticks.iter().filter(|tick| tick.label.is_some()) {
        let mut inset = Rect::length(0.0);
        inset.left = LengthPercentageAuto::length(tick.x + 3.0);
        inset.top = LengthPercentageAuto::length(2.0);
        document.add_child(
            root,
            UiNode::text(
                format!("{name}.label.{}", tick.value),
                tick.label.clone().unwrap_or_default(),
                options.label_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    position: Position::Absolute,
                    inset,
                    size: TaffySize {
                        width: length(64.0),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            ),
        );
    }

    root
}
