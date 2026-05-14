//! Progress indicator widget.

use taffy::prelude::{Dimension, Size as TaffySize, Style};

use crate::{
    length, AccessibilityMeta, AccessibilityRole, AccessibilityValueRange, ClipBehavior,
    LayoutStyle, ShaderEffect, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiRect, UiVisual,
};

use super::surfaces::{DEFAULT_ACCENT, DEFAULT_SURFACE_BG};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgressIndicatorKind {
    Progress,
    Meter,
}

impl ProgressIndicatorKind {
    pub const fn accessibility_role(self) -> AccessibilityRole {
        match self {
            Self::Progress => AccessibilityRole::ProgressBar,
            Self::Meter => AccessibilityRole::Meter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressIndicatorValue {
    pub value: Option<f32>,
    pub min: f32,
    pub max: f32,
}

impl ProgressIndicatorValue {
    pub fn new(value: f32, min: f32, max: f32) -> Self {
        let (min, max) = ordered_progress_range(min, max);
        Self {
            value: value.is_finite().then_some(value.clamp(min, max)),
            min,
            max,
        }
    }

    pub fn percent(percent: f32) -> Self {
        Self::new(percent, 0.0, 100.0)
    }

    pub fn indeterminate(min: f32, max: f32) -> Self {
        let (min, max) = ordered_progress_range(min, max);
        Self {
            value: None,
            min,
            max,
        }
    }

    pub fn normalized(self) -> Option<f32> {
        let value = self.value?;
        let span = (self.max - self.min).max(f32::EPSILON);
        Some(((value - self.min) / span).clamp(0.0, 1.0))
    }

    pub fn value_text(self, unit: Option<&str>) -> String {
        let Some(value) = self.value else {
            return "Indeterminate".to_string();
        };
        if let Some(unit) = unit.filter(|unit| !unit.is_empty()) {
            format!("{} {}", format_progress_number(value), unit)
        } else if self.min == 0.0 && self.max == 100.0 {
            format!("{}%", format_progress_number(value))
        } else {
            format_progress_number(value)
        }
    }

    pub fn fill_rect(self, track: UiRect) -> UiRect {
        let normalized = self.normalized().unwrap_or(0.0);
        UiRect::new(track.x, track.y, track.width * normalized, track.height)
    }

    pub fn accessibility_meta(
        self,
        label: impl Into<String>,
        kind: ProgressIndicatorKind,
        unit: Option<&str>,
    ) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(kind.accessibility_role())
            .label(label)
            .value(self.value_text(unit));
        if self.value.is_some() {
            meta = meta.value_range(AccessibilityValueRange::new(
                self.min as f64,
                self.max as f64,
            ));
        } else {
            meta = meta.hint("Value is not currently available");
        }
        meta
    }
}

fn ordered_progress_range(min: f32, max: f32) -> (f32, f32) {
    let min = if min.is_finite() { min } else { 0.0 };
    let max = if max.is_finite() { max } else { 1.0 };
    if (max - min).abs() <= f32::EPSILON {
        (min, min + 1.0)
    } else if min <= max {
        (min, max)
    } else {
        (max, min)
    }
}

fn format_progress_number(value: f32) -> String {
    if value.fract().abs() <= 0.0001 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

#[derive(Debug, Clone)]
pub struct ProgressIndicatorOptions {
    pub layout: LayoutStyle,
    pub kind: ProgressIndicatorKind,
    pub track_visual: UiVisual,
    pub fill_visual: UiVisual,
    pub shader: Option<ShaderEffect>,
    pub fill_shader: Option<ShaderEffect>,
    pub accessibility_label: Option<String>,
    pub accessibility_unit: Option<String>,
}

impl Default for ProgressIndicatorOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(8.0),
                },
                ..Default::default()
            }),
            kind: ProgressIndicatorKind::Progress,
            track_visual: UiVisual::panel(DEFAULT_SURFACE_BG, None, 3.0),
            fill_visual: UiVisual::panel(DEFAULT_ACCENT, None, 3.0),
            shader: None,
            fill_shader: None,
            accessibility_label: None,
            accessibility_unit: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressIndicatorNodes {
    pub root: UiNodeId,
    pub fill: UiNodeId,
}

pub fn progress_indicator(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    value: ProgressIndicatorValue,
    options: ProgressIndicatorOptions,
) -> ProgressIndicatorNodes {
    let name = name.into();
    let label = options
        .accessibility_label
        .clone()
        .unwrap_or_else(|| name.clone());
    let mut root = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_visual(options.track_visual)
    .with_accessibility(value.accessibility_meta(
        label,
        options.kind,
        options.accessibility_unit.as_deref(),
    ));
    if let Some(shader) = options.shader {
        root = root.with_shader(shader);
    }
    let root = document.add_child(parent, root);

    let mut fill = UiNode::container(
        format!("{name}.fill"),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(value.normalized().unwrap_or(0.0)),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            })
            .style,
            ..Default::default()
        },
    )
    .with_visual(options.fill_visual);
    if let Some(shader) = options.fill_shader {
        fill = fill.with_shader(shader);
    }
    let fill = document.add_child(root, fill);

    ProgressIndicatorNodes { root, fill }
}
