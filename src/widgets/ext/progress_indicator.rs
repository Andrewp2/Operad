//! Progress indicator widget.

use taffy::prelude::{Dimension, Size as TaffySize, Style};

use crate::{
    length, AccessibilityMeta, AccessibilityRole, AccessibilityValueRange, ClipBehavior, ColorRgba,
    LayoutStyle, ScrollAxes, ShaderEffect, StrokeStyle, TextStyle, UiDocument, UiNode, UiNodeId,
    UiNodeStyle, UiRect, UiVisual,
};

use super::surfaces::{DEFAULT_ACCENT, DEFAULT_SURFACE_BG, DEFAULT_SURFACE_STROKE};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgressLogLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl ProgressLogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    pub const fn color(self) -> ColorRgba {
        match self {
            Self::Info => ColorRgba::new(196, 210, 230, 255),
            Self::Success => ColorRgba::new(111, 203, 159, 255),
            Self::Warning => ColorRgba::new(232, 186, 88, 255),
            Self::Error => ColorRgba::new(255, 122, 122, 255),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressLogEntry {
    pub level: ProgressLogLevel,
    pub message: String,
}

impl ProgressLogEntry {
    pub fn new(level: ProgressLogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(ProgressLogLevel::Info, message)
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(ProgressLogLevel::Success, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(ProgressLogLevel::Warning, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(ProgressLogLevel::Error, message)
    }
}

impl From<&str> for ProgressLogEntry {
    fn from(value: &str) -> Self {
        Self::info(value)
    }
}

impl From<String> for ProgressLogEntry {
    fn from(value: String) -> Self {
        Self::info(value)
    }
}

#[derive(Debug, Clone)]
pub struct ProgressLogPanelOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub progress_options: ProgressIndicatorOptions,
    pub log_viewport_layout: LayoutStyle,
    pub log_visual: UiVisual,
    pub log_row_height: f32,
    pub log_text_style: TextStyle,
    pub empty_text_style: TextStyle,
    pub accessibility_label: Option<String>,
    pub empty_message: String,
}

impl Default for ProgressLogPanelOptions {
    fn default() -> Self {
        let mut progress_options = ProgressIndicatorOptions::default();
        progress_options.layout = LayoutStyle::new().with_width_percent(1.0).with_height(10.0);
        Self {
            layout: LayoutStyle::column()
                .with_width_percent(1.0)
                .with_padding(10.0)
                .with_gap(8.0),
            visual: UiVisual::panel(
                ColorRgba::new(17, 21, 27, 255),
                Some(StrokeStyle::new(DEFAULT_SURFACE_STROKE, 1.0)),
                4.0,
            ),
            progress_options,
            log_viewport_layout: LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(96.0),
            log_visual: UiVisual::panel(
                ColorRgba::new(11, 15, 21, 255),
                Some(StrokeStyle::new(ColorRgba::new(45, 57, 73, 255), 1.0)),
                3.0,
            ),
            log_row_height: 26.0,
            log_text_style: TextStyle {
                font_size: 12.0,
                line_height: 18.0,
                color: ColorRgba::new(196, 210, 230, 255),
                ..Default::default()
            },
            empty_text_style: TextStyle {
                font_size: 12.0,
                line_height: 18.0,
                color: ColorRgba::new(154, 166, 184, 255),
                ..Default::default()
            },
            accessibility_label: None,
            empty_message: "Waiting for log output...".to_string(),
        }
    }
}

impl ProgressLogPanelOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_log_viewport_height(mut self, height: f32) -> Self {
        let height = height.max(0.0);
        self.log_viewport_layout = self.log_viewport_layout.with_height(height);
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressLogPanelNodes {
    pub root: UiNodeId,
    pub progress: ProgressIndicatorNodes,
    pub logs: UiNodeId,
}

pub fn progress_log_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    value: ProgressIndicatorValue,
    logs: &[ProgressLogEntry],
    mut options: ProgressLogPanelOptions,
) -> ProgressLogPanelNodes {
    let name = name.into();
    let label = options
        .accessibility_label
        .clone()
        .unwrap_or_else(|| name.clone());
    let root = document.add_child(
        parent,
        UiNode::container(name.clone(), options.layout)
            .with_visual(options.visual)
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).label(label)),
    );

    if options.progress_options.accessibility_label.is_none() {
        options.progress_options.accessibility_label = Some(format!("{name} progress"));
    }
    let progress = progress_indicator(
        document,
        root,
        format!("{name}.progress"),
        value,
        options.progress_options,
    );

    let logs_node = crate::widgets::scroll_area(
        document,
        root,
        format!("{name}.logs"),
        ScrollAxes::VERTICAL,
        options.log_viewport_layout,
    );
    {
        let node = document.node_mut(logs_node);
        node.set_visual(options.log_visual);
        node.accessibility = Some(
            AccessibilityMeta::new(AccessibilityRole::List)
                .label(format!("{name} logs"))
                .value(format!("{} entries", logs.len())),
        );
    }

    if logs.is_empty() {
        let empty_row_height = options
            .log_row_height
            .max(options.empty_text_style.line_height + 8.0);
        document.add_child(
            logs_node,
            UiNode::text(
                format!("{name}.logs.empty"),
                options.empty_message,
                options.empty_text_style,
                LayoutStyle::new()
                    .with_width_percent(1.0)
                    .with_height(empty_row_height)
                    .with_padding(4.0)
                    .with_flex_shrink(0.0),
            )
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Status).label("No logs")),
        );
    } else {
        let log_row_height = options
            .log_row_height
            .max(options.log_text_style.line_height + 8.0);
        for (index, entry) in logs.iter().enumerate() {
            let mut text_style = options.log_text_style.clone();
            text_style.color = entry.level.color();
            document.add_child(
                logs_node,
                UiNode::text(
                    format!("{name}.logs.row.{index}"),
                    format!("[{}] {}", entry.level.as_str(), entry.message),
                    text_style,
                    LayoutStyle::new()
                        .with_width_percent(1.0)
                        .with_height(log_row_height)
                        .with_padding(4.0)
                        .with_flex_shrink(0.0),
                )
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::ListItem).label(format!(
                        "{}: {}",
                        entry.level.as_str(),
                        entry.message
                    )),
                ),
            );
        }
    }

    ProgressLogPanelNodes {
        root,
        progress,
        logs: logs_node,
    }
}
