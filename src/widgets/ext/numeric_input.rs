//! Numeric input and slider widget models.

use crate::{
    AccessibilityMeta, AccessibilityRole, AccessibilityValueRange, ColorRgba, EditPhase,
    ImageContent, KeyCode, KeyModifiers, ShaderEffect, UiPoint, UiRect,
};

use super::pickers::{PickerAnimationMeta, PickerElementStyle};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericRange {
    pub min: f64,
    pub max: f64,
}

impl NumericRange {
    pub fn new(min: f64, max: f64) -> Self {
        let min = finite_or(min, 0.0);
        let max = finite_or(max, min);
        if min <= max {
            Self { min, max }
        } else {
            Self { min: max, max: min }
        }
    }

    pub fn clamp(self, value: f64) -> f64 {
        finite_or(value, self.min).clamp(self.min, self.max)
    }

    pub fn contains(self, value: f64) -> bool {
        value.is_finite() && value >= self.min && value <= self.max
    }

    pub fn span(self) -> f64 {
        self.max - self.min
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericPrecision {
    pub decimals: u8,
    pub step: f64,
}

impl NumericPrecision {
    pub const INTEGER: Self = Self {
        decimals: 0,
        step: 1.0,
    };

    pub fn decimals(decimals: u8) -> Self {
        let decimals = decimals.min(12);
        Self {
            decimals,
            step: 10_f64.powi(-i32::from(decimals)),
        }
    }

    pub fn with_step(mut self, step: f64) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        self
    }

    pub fn quantize(self, value: f64) -> f64 {
        let value = finite_or(value, 0.0);
        let stepped = (value / self.step).round() * self.step;
        let scale = 10_f64.powi(i32::from(self.decimals));
        let rounded = (stepped * scale).round() / scale;
        if rounded == 0.0 {
            0.0
        } else {
            rounded
        }
    }

    pub fn format(self, value: f64) -> String {
        format!("{:.*}", usize::from(self.decimals), self.quantize(value))
    }
}

impl Default for NumericPrecision {
    fn default() -> Self {
        Self::INTEGER
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumericScale {
    Linear,
    Logarithmic { base: f64 },
}

impl NumericScale {
    pub const LINEAR: Self = Self::Linear;

    pub fn logarithmic(base: f64) -> Self {
        Self::Logarithmic {
            base: if base.is_finite() && base > 1.0 {
                base
            } else {
                10.0
            },
        }
    }

    pub fn supports_range(self, range: NumericRange) -> bool {
        match self {
            Self::Linear => range.span() > 0.0,
            Self::Logarithmic { .. } => range.min > 0.0 && range.span() > 0.0,
        }
    }

    pub fn position_for_value(self, range: NumericRange, value: f64) -> Option<f64> {
        if !self.supports_range(range) {
            return None;
        }
        let value = range.clamp(value);
        let position = match self {
            Self::Linear => (value - range.min) / range.span(),
            Self::Logarithmic { base } => {
                let min = range.min.log(base);
                let max = range.max.log(base);
                let value = value.log(base);
                (value - min) / (max - min)
            }
        };
        Some(position.clamp(0.0, 1.0))
    }

    pub fn value_at_position(self, range: NumericRange, position: f64) -> Option<f64> {
        if !self.supports_range(range) {
            return None;
        }
        let position = finite_or(position, 0.0).clamp(0.0, 1.0);
        let value = match self {
            Self::Linear => range.min + range.span() * position,
            Self::Logarithmic { base } => {
                let min = range.min.log(base);
                let max = range.max.log(base);
                base.powf(min + (max - min) * position)
            }
        };
        Some(range.clamp(value))
    }
}

impl Default for NumericScale {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NumericUnitFormat {
    pub prefix: String,
    pub suffix: String,
}

impl NumericUnitFormat {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub fn format(&self, value: impl AsRef<str>) -> String {
        format!("{}{}{}", self.prefix, value.as_ref(), self.suffix)
    }

    pub fn strip_affixes(&self, text: &str) -> String {
        let mut text = text.trim();
        if !self.prefix.is_empty() {
            text = text.strip_prefix(&self.prefix).unwrap_or(text).trim_start();
        }
        if !self.suffix.is_empty() {
            text = text.strip_suffix(&self.suffix).unwrap_or(text).trim_end();
        }
        text.trim().replace(['_', ','], "")
    }

    pub fn is_empty(&self) -> bool {
        self.prefix.is_empty() && self.suffix.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericParameterSpec {
    pub label: String,
    pub range: NumericRange,
    pub precision: NumericPrecision,
    pub scale: NumericScale,
    pub unit: NumericUnitFormat,
}

impl NumericParameterSpec {
    pub fn new(label: impl Into<String>, range: NumericRange, precision: NumericPrecision) -> Self {
        Self {
            label: label.into(),
            range,
            precision,
            scale: NumericScale::Linear,
            unit: NumericUnitFormat::default(),
        }
    }

    pub fn logarithmic(mut self, base: f64) -> Self {
        self.scale = NumericScale::logarithmic(base);
        self
    }

    pub fn unit_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.unit.prefix = prefix.into();
        self
    }

    pub fn unit_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.unit.suffix = suffix.into();
        self
    }

    pub fn normalize_value(&self, value: f64) -> f64 {
        self.precision.quantize(self.range.clamp(value))
    }

    pub fn format_value(&self, value: f64) -> String {
        self.unit
            .format(self.precision.format(self.normalize_value(value)))
    }

    pub fn validate_text(&self, text: &str) -> NumericTextValidation {
        validate_numeric_text(
            &self.unit.strip_affixes(text),
            Some(self.range),
            self.precision,
        )
    }

    pub fn parse_text(&self, text: &str) -> Option<f64> {
        parse_numeric_text(&self.unit.strip_affixes(text)).map(|value| self.normalize_value(value))
    }

    pub fn position_for_value(&self, value: f64) -> f64 {
        self.scale
            .position_for_value(self.range, value)
            .or_else(|| NumericScale::Linear.position_for_value(self.range, value))
            .unwrap_or(0.0)
    }

    pub fn value_at_position(&self, position: f64) -> f64 {
        let value = self
            .scale
            .value_at_position(self.range, position)
            .or_else(|| NumericScale::Linear.value_at_position(self.range, position))
            .unwrap_or(self.range.min);
        self.normalize_value(value)
    }

    pub fn text_accessibility_meta(&self, value_text: impl Into<String>) -> AccessibilityMeta {
        let value_text = value_text.into();
        let mut meta = AccessibilityMeta::new(AccessibilityRole::TextBox)
            .label(self.label.clone())
            .value(value_text.clone())
            .hint(numeric_range_hint(self.range, self.precision))
            .focusable()
            .action(crate::AccessibilityAction::new("commit", "Commit value"))
            .action(crate::AccessibilityAction::new("cancel", "Cancel edit"));
        if !self.validate_text(&value_text).is_valid() {
            meta = meta.invalid("Enter a valid parameter value");
        }
        meta
    }

    pub fn slider_accessibility_meta(&self, value: f64) -> AccessibilityMeta {
        AccessibilityMeta::new(AccessibilityRole::Slider)
            .label(self.label.clone())
            .value(self.format_value(value))
            .hint(numeric_range_hint(self.range, self.precision))
            .value_range(AccessibilityValueRange::new(self.range.min, self.range.max))
            .focusable()
            .action(crate::AccessibilityAction::new("decrease", "Decrease"))
            .action(crate::AccessibilityAction::new("increase", "Increase"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericValidationStatus {
    Valid,
    Empty,
    InvalidNumber,
    OutOfRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericTextValidation {
    pub status: NumericValidationStatus,
    pub parsed: Option<f64>,
    pub normalized: Option<f64>,
    pub message: Option<String>,
}

impl NumericTextValidation {
    pub fn is_valid(&self) -> bool {
        self.status == NumericValidationStatus::Valid
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericKeyboardStep {
    Decrement,
    Increment,
    LargeDecrement,
    LargeIncrement,
    Minimum,
    Maximum,
}

impl NumericKeyboardStep {
    pub fn from_key(key: KeyCode, modifiers: KeyModifiers) -> Option<Self> {
        let large = modifiers.shift;
        match key {
            KeyCode::ArrowUp | KeyCode::ArrowRight => Some(if large {
                Self::LargeIncrement
            } else {
                Self::Increment
            }),
            KeyCode::ArrowDown | KeyCode::ArrowLeft => Some(if large {
                Self::LargeDecrement
            } else {
                Self::Decrement
            }),
            KeyCode::Home => Some(Self::Minimum),
            KeyCode::End => Some(Self::Maximum),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericInputState {
    pub value: f64,
    pub text: String,
    pub range: Option<NumericRange>,
    pub precision: NumericPrecision,
    pub phase: EditPhase,
}

impl NumericInputState {
    pub fn new(value: f64) -> Self {
        let precision = NumericPrecision::default();
        let value = precision.quantize(value);
        Self {
            value,
            text: precision.format(value),
            range: None,
            precision,
            phase: EditPhase::Preview,
        }
    }

    pub fn with_range(mut self, range: NumericRange) -> Self {
        self.range = Some(range);
        self.value = self.normalize_value(self.value);
        self.text = self.precision.format(self.value);
        self
    }

    pub fn with_precision(mut self, precision: NumericPrecision) -> Self {
        self.precision = precision;
        self.value = self.normalize_value(self.value);
        self.text = self.precision.format(self.value);
        self
    }

    pub fn with_parameter(mut self, parameter: &NumericParameterSpec) -> Self {
        self.range = Some(parameter.range);
        self.precision = parameter.precision;
        self.value = parameter.normalize_value(self.value);
        self.text = parameter.format_value(self.value);
        self
    }

    pub fn validation(&self) -> NumericTextValidation {
        self.validate_text_value(&self.text)
    }

    pub fn validate_text_value(&self, text: &str) -> NumericTextValidation {
        validate_numeric_text(text, self.range, self.precision)
    }

    pub fn text_accessibility_meta(&self, label: impl Into<String>) -> AccessibilityMeta {
        let validation = self.validation();
        let mut meta = AccessibilityMeta::new(AccessibilityRole::TextBox)
            .label(label)
            .value(self.text.clone())
            .focusable();
        if let Some(message) = validation.message {
            meta = meta.hint(message.clone()).invalid(message);
        } else if let Some(range) = self.range {
            meta = meta.hint(numeric_range_hint(range, self.precision));
        }
        if self.range.is_none() {
            meta = meta.action(crate::AccessibilityAction::new("commit", "Commit value"));
        }
        meta
    }

    pub fn slider_accessibility_meta(&self, label: impl Into<String>) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(AccessibilityRole::Slider)
            .label(label)
            .value(self.precision.format(self.value))
            .focusable();
        if let Some(range) = self.range {
            meta = meta
                .hint(numeric_range_hint(range, self.precision))
                .value_range(AccessibilityValueRange::new(range.min, range.max));
        }
        meta
    }

    pub fn copy_text(&self) -> String {
        self.text.clone()
    }

    pub fn copy_value_text(&self) -> String {
        self.precision.format(self.value)
    }

    pub fn paste_text(&mut self, text: &str) -> NumericInputOutcome {
        self.update_text(normalize_numeric_clipboard_text(text))
    }

    pub fn begin_edit(&mut self) -> NumericInputOutcome {
        self.phase = EditPhase::BeginEdit;
        self.text = self.precision.format(self.value);
        self.outcome(self.value, false)
    }

    pub fn update_text(&mut self, text: impl Into<String>) -> NumericInputOutcome {
        let previous = self.value;
        self.phase = EditPhase::UpdateEdit;
        self.text = text.into();
        if let Some(parsed) = parse_numeric_text(&self.text) {
            self.value = self.normalize_value(parsed);
        }
        self.outcome(previous, previous != self.value)
    }

    pub fn update_parameter_text(
        &mut self,
        text: impl Into<String>,
        parameter: &NumericParameterSpec,
    ) -> NumericInputOutcome {
        let previous = self.value;
        self.phase = EditPhase::UpdateEdit;
        self.text = text.into();
        if let Some(parsed) = parameter.parse_text(&self.text) {
            self.value = parsed;
        }
        self.outcome(previous, previous != self.value)
    }

    pub fn commit_text(&mut self) -> NumericInputOutcome {
        let previous = self.value;
        if let Some(parsed) = parse_numeric_text(&self.text) {
            self.value = self.normalize_value(parsed);
            self.text = self.precision.format(self.value);
            self.phase = EditPhase::CommitEdit;
            self.outcome(previous, previous != self.value)
        } else {
            self.text = self.precision.format(self.value);
            self.phase = EditPhase::CancelEdit;
            self.outcome(previous, false)
        }
    }

    pub fn commit_parameter_text(
        &mut self,
        parameter: &NumericParameterSpec,
    ) -> NumericInputOutcome {
        let previous = self.value;
        if let Some(parsed) = parameter.parse_text(&self.text) {
            self.value = parsed;
            self.text = parameter.format_value(self.value);
            self.phase = EditPhase::CommitEdit;
            self.outcome(previous, previous != self.value)
        } else {
            self.text = parameter.format_value(self.value);
            self.phase = EditPhase::CancelEdit;
            self.outcome(previous, false)
        }
    }

    pub fn cancel_edit(&mut self) -> NumericInputOutcome {
        self.phase = EditPhase::CancelEdit;
        self.text = self.precision.format(self.value);
        self.outcome(self.value, false)
    }

    pub fn cancel_parameter_edit(
        &mut self,
        parameter: &NumericParameterSpec,
    ) -> NumericInputOutcome {
        self.phase = EditPhase::CancelEdit;
        self.text = parameter.format_value(self.value);
        self.outcome(self.value, false)
    }

    pub fn set_value(&mut self, value: f64, phase: EditPhase) -> NumericInputOutcome {
        let previous = self.value;
        self.value = self.normalize_value(value);
        self.text = self.precision.format(self.value);
        self.phase = phase;
        self.outcome(previous, previous != self.value)
    }

    pub fn set_parameter_value(
        &mut self,
        value: f64,
        phase: EditPhase,
        parameter: &NumericParameterSpec,
    ) -> NumericInputOutcome {
        let previous = self.value;
        self.range = Some(parameter.range);
        self.precision = parameter.precision;
        self.value = parameter.normalize_value(value);
        self.text = parameter.format_value(self.value);
        self.phase = phase;
        self.outcome(previous, previous != self.value)
    }

    pub fn set_parameter_position(
        &mut self,
        position: f64,
        phase: EditPhase,
        parameter: &NumericParameterSpec,
    ) -> NumericInputOutcome {
        self.set_parameter_value(parameter.value_at_position(position), phase, parameter)
    }

    pub fn nudge(&mut self, steps: i32) -> NumericInputOutcome {
        self.set_value(
            self.value + self.precision.step * f64::from(steps),
            EditPhase::UpdateEdit,
        )
    }

    pub fn apply_keyboard_step(&mut self, step: NumericKeyboardStep) -> NumericInputOutcome {
        match step {
            NumericKeyboardStep::Decrement => self.nudge(-1),
            NumericKeyboardStep::Increment => self.nudge(1),
            NumericKeyboardStep::LargeDecrement => self.nudge(-10),
            NumericKeyboardStep::LargeIncrement => self.nudge(10),
            NumericKeyboardStep::Minimum => {
                if let Some(range) = self.range {
                    self.set_value(range.min, EditPhase::UpdateEdit)
                } else {
                    self.phase = EditPhase::Preview;
                    self.outcome(self.value, false)
                }
            }
            NumericKeyboardStep::Maximum => {
                if let Some(range) = self.range {
                    self.set_value(range.max, EditPhase::UpdateEdit)
                } else {
                    self.phase = EditPhase::Preview;
                    self.outcome(self.value, false)
                }
            }
        }
    }

    pub fn handle_keyboard_step(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<NumericInputOutcome> {
        NumericKeyboardStep::from_key(key, modifiers).map(|step| self.apply_keyboard_step(step))
    }

    pub fn apply_drag(
        &mut self,
        start_value: f64,
        delta_pixels: f32,
        drag: NumericDragSpec,
        speed: NumericDragSpeed,
    ) -> NumericInputOutcome {
        self.set_value(
            drag_value(
                start_value,
                delta_pixels,
                self.precision,
                self.range,
                drag,
                speed,
            ),
            EditPhase::UpdateEdit,
        )
    }

    fn normalize_value(&self, value: f64) -> f64 {
        let value = self
            .range
            .map_or(finite_or(value, 0.0), |range| range.clamp(value));
        self.precision.quantize(value)
    }

    fn outcome(&self, previous: f64, changed: bool) -> NumericInputOutcome {
        NumericInputOutcome {
            previous,
            value: self.value,
            text: self.text.clone(),
            phase: self.phase,
            changed,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericInputOutcome {
    pub previous: f64,
    pub value: f64,
    pub text: String,
    pub phase: EditPhase,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderAxis {
    Horizontal,
    Vertical,
}

impl SliderAxis {
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }

    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Vertical)
    }
}

impl Default for SliderAxis {
    fn default() -> Self {
        Self::Horizontal
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderGeometry {
    pub track: UiRect,
    pub axis: SliderAxis,
    pub thumb_size: f32,
}

impl SliderGeometry {
    pub fn new(track: UiRect, axis: SliderAxis) -> Self {
        Self {
            track: sanitized_rect(track),
            axis,
            thumb_size: 12.0,
        }
    }

    pub fn horizontal(track: UiRect) -> Self {
        Self::new(track, SliderAxis::Horizontal)
    }

    pub fn vertical(track: UiRect) -> Self {
        Self::new(track, SliderAxis::Vertical)
    }

    pub fn thumb_size(mut self, thumb_size: f32) -> Self {
        self.thumb_size = finite_or_f32(thumb_size, 12.0).max(0.0);
        self
    }

    pub fn position_from_point(self, point: UiPoint) -> f64 {
        let point = UiPoint::new(finite_or_f32(point.x, 0.0), finite_or_f32(point.y, 0.0));
        match self.axis {
            SliderAxis::Horizontal => {
                if self.track.width <= f32::EPSILON {
                    0.0
                } else {
                    ((point.x - self.track.x) / self.track.width).clamp(0.0, 1.0) as f64
                }
            }
            SliderAxis::Vertical => {
                if self.track.height <= f32::EPSILON {
                    0.0
                } else {
                    (1.0 - (point.y - self.track.y) / self.track.height).clamp(0.0, 1.0) as f64
                }
            }
        }
    }

    pub fn fill_rect(self, position: f64) -> UiRect {
        let position = unit_f64(position) as f32;
        match self.axis {
            SliderAxis::Horizontal => UiRect::new(
                self.track.x,
                self.track.y,
                self.track.width * position,
                self.track.height,
            ),
            SliderAxis::Vertical => {
                let height = self.track.height * position;
                UiRect::new(
                    self.track.x,
                    self.track.bottom() - height,
                    self.track.width,
                    height,
                )
            }
        }
    }

    pub fn thumb_rect(self, position: f64) -> UiRect {
        let position = unit_f64(position) as f32;
        let size = self.thumb_size.max(0.0);
        match self.axis {
            SliderAxis::Horizontal => UiRect::new(
                self.track.x + self.track.width * position - size * 0.5,
                self.track.y + self.track.height * 0.5 - size * 0.5,
                size,
                size,
            ),
            SliderAxis::Vertical => UiRect::new(
                self.track.x + self.track.width * 0.5 - size * 0.5,
                self.track.bottom() - self.track.height * position - size * 0.5,
                size,
                size,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericSliderDrag {
    pub start_value: f64,
    pub start_position: f64,
    pub pointer_start: UiPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericSliderState {
    pub value: f64,
    pub phase: EditPhase,
    pub dragging: Option<NumericSliderDrag>,
}

impl NumericSliderState {
    pub fn new(value: f64, parameter: &NumericParameterSpec) -> Self {
        Self {
            value: parameter.normalize_value(value),
            phase: EditPhase::Preview,
            dragging: None,
        }
    }

    pub fn position(&self, parameter: &NumericParameterSpec) -> f64 {
        parameter.position_for_value(self.value)
    }

    pub fn set_value(
        &mut self,
        value: f64,
        phase: EditPhase,
        parameter: &NumericParameterSpec,
    ) -> NumericSliderOutcome {
        let previous = self.value;
        self.value = parameter.normalize_value(value);
        self.phase = phase;
        self.outcome(previous, parameter)
    }

    pub fn set_position(
        &mut self,
        position: f64,
        phase: EditPhase,
        parameter: &NumericParameterSpec,
    ) -> NumericSliderOutcome {
        self.set_value(parameter.value_at_position(position), phase, parameter)
    }

    pub fn value_from_point(
        &self,
        geometry: SliderGeometry,
        point: UiPoint,
        parameter: &NumericParameterSpec,
    ) -> f64 {
        parameter.value_at_position(geometry.position_from_point(point))
    }

    pub fn begin_drag(
        &mut self,
        geometry: SliderGeometry,
        point: UiPoint,
        parameter: &NumericParameterSpec,
    ) -> NumericSliderOutcome {
        self.dragging = Some(NumericSliderDrag {
            start_value: self.value,
            start_position: self.position(parameter),
            pointer_start: point,
        });
        self.set_value(
            self.value_from_point(geometry, point, parameter),
            EditPhase::BeginEdit,
            parameter,
        )
    }

    pub fn update_drag(
        &mut self,
        geometry: SliderGeometry,
        point: UiPoint,
        parameter: &NumericParameterSpec,
    ) -> NumericSliderOutcome {
        if self.dragging.is_none() {
            return self.begin_drag(geometry, point, parameter);
        }
        self.set_value(
            self.value_from_point(geometry, point, parameter),
            EditPhase::UpdateEdit,
            parameter,
        )
    }

    pub fn end_drag(&mut self, parameter: &NumericParameterSpec) -> NumericSliderOutcome {
        self.dragging = None;
        self.set_value(self.value, EditPhase::CommitEdit, parameter)
    }

    pub fn cancel_drag(&mut self, parameter: &NumericParameterSpec) -> NumericSliderOutcome {
        let previous = self.value;
        let restored = self.dragging.map_or(self.value, |drag| drag.start_value);
        self.dragging = None;
        self.value = parameter.normalize_value(restored);
        self.phase = EditPhase::CancelEdit;
        self.outcome(previous, parameter)
    }

    pub fn apply_keyboard_step(
        &mut self,
        step: NumericKeyboardStep,
        parameter: &NumericParameterSpec,
    ) -> NumericSliderOutcome {
        match step {
            NumericKeyboardStep::Decrement => self.nudge(-1, EditPhase::UpdateEdit, parameter),
            NumericKeyboardStep::Increment => self.nudge(1, EditPhase::UpdateEdit, parameter),
            NumericKeyboardStep::LargeDecrement => {
                self.nudge(-10, EditPhase::UpdateEdit, parameter)
            }
            NumericKeyboardStep::LargeIncrement => self.nudge(10, EditPhase::UpdateEdit, parameter),
            NumericKeyboardStep::Minimum => {
                self.set_value(parameter.range.min, EditPhase::UpdateEdit, parameter)
            }
            NumericKeyboardStep::Maximum => {
                self.set_value(parameter.range.max, EditPhase::UpdateEdit, parameter)
            }
        }
    }

    pub fn handle_keyboard_step(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
        parameter: &NumericParameterSpec,
    ) -> Option<NumericSliderOutcome> {
        NumericKeyboardStep::from_key(key, modifiers)
            .map(|step| self.apply_keyboard_step(step, parameter))
    }

    pub fn fill_rect(&self, geometry: SliderGeometry, parameter: &NumericParameterSpec) -> UiRect {
        geometry.fill_rect(self.position(parameter))
    }

    pub fn thumb_rect(&self, geometry: SliderGeometry, parameter: &NumericParameterSpec) -> UiRect {
        geometry.thumb_rect(self.position(parameter))
    }

    pub fn accessibility_meta(&self, parameter: &NumericParameterSpec) -> AccessibilityMeta {
        parameter.slider_accessibility_meta(self.value)
    }

    fn nudge(
        &mut self,
        steps: i32,
        phase: EditPhase,
        parameter: &NumericParameterSpec,
    ) -> NumericSliderOutcome {
        self.set_value(
            self.value + parameter.precision.step * f64::from(steps),
            phase,
            parameter,
        )
    }

    fn outcome(&self, previous: f64, parameter: &NumericParameterSpec) -> NumericSliderOutcome {
        NumericSliderOutcome {
            previous,
            value: self.value,
            position: self.position(parameter),
            text: parameter.format_value(self.value),
            phase: self.phase,
            changed: previous != self.value,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericSliderOutcome {
    pub previous: f64,
    pub value: f64,
    pub position: f64,
    pub text: String,
    pub phase: EditPhase,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericInputStyle {
    pub text_field: PickerElementStyle,
    pub error_text_field: PickerElementStyle,
    pub drag_handle: PickerElementStyle,
    pub slider: PickerElementStyle,
}

impl NumericInputStyle {
    pub fn style_for_validation(&self, validation: &NumericTextValidation) -> &PickerElementStyle {
        if validation.is_valid() {
            &self.text_field
        } else {
            &self.error_text_field
        }
    }
}

impl Default for NumericInputStyle {
    fn default() -> Self {
        Self {
            text_field: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(235, 240, 247, 255))
                .with_background(ColorRgba::new(18, 22, 28, 255)),
            error_text_field: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(255, 238, 240, 255))
                .with_border(ColorRgba::new(201, 74, 91, 255))
                .with_animation(PickerAnimationMeta::new("numeric.error", 0.14)),
            drag_handle: PickerElementStyle::default()
                .with_background(ColorRgba::new(46, 55, 68, 255))
                .with_image(ImageContent::new("icons.drag-horizontal")),
            slider: PickerElementStyle::default()
                .with_background(ColorRgba::new(42, 49, 58, 255))
                .with_shader(ShaderEffect::new("numeric.slider-fill")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericDragSpec {
    pub pixels_per_step: f32,
    pub fine_multiplier: f64,
    pub coarse_multiplier: f64,
}

impl NumericDragSpec {
    pub const DEFAULT: Self = Self {
        pixels_per_step: 8.0,
        fine_multiplier: 0.1,
        coarse_multiplier: 10.0,
    };

    pub fn value_delta(
        self,
        delta_pixels: f32,
        precision: NumericPrecision,
        speed: NumericDragSpeed,
    ) -> f64 {
        let pixels_per_step = self.pixels_per_step.max(1.0);
        let multiplier = match speed {
            NumericDragSpeed::Fine => self.fine_multiplier,
            NumericDragSpeed::Normal => 1.0,
            NumericDragSpeed::Coarse => self.coarse_multiplier,
        };
        f64::from(delta_pixels) / f64::from(pixels_per_step) * precision.step * multiplier
    }
}

impl Default for NumericDragSpec {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericDragSpeed {
    Fine,
    Normal,
    Coarse,
}

pub fn drag_value(
    start_value: f64,
    delta_pixels: f32,
    precision: NumericPrecision,
    range: Option<NumericRange>,
    drag: NumericDragSpec,
    speed: NumericDragSpeed,
) -> f64 {
    let value = start_value + drag.value_delta(delta_pixels, precision, speed);
    let value = range.map_or(finite_or(value, 0.0), |range| range.clamp(value));
    precision.quantize(value)
}

fn finite_or_f32(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn sanitized_rect(rect: UiRect) -> UiRect {
    UiRect::new(
        finite_or_f32(rect.x, 0.0),
        finite_or_f32(rect.y, 0.0),
        finite_or_f32(rect.width, 0.0).max(0.0),
        finite_or_f32(rect.height, 0.0).max(0.0),
    )
}

fn unit_f64(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn validate_numeric_text(
    text: &str,
    range: Option<NumericRange>,
    precision: NumericPrecision,
) -> NumericTextValidation {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return NumericTextValidation {
            status: NumericValidationStatus::Empty,
            parsed: None,
            normalized: None,
            message: Some("Enter a number".to_string()),
        };
    }

    let Some(parsed) = parse_numeric_text(trimmed) else {
        return NumericTextValidation {
            status: NumericValidationStatus::InvalidNumber,
            parsed: None,
            normalized: None,
            message: Some("Enter a finite number".to_string()),
        };
    };

    let in_range = range.is_none_or(|range| range.contains(parsed));
    let bounded = range.map_or(parsed, |range| range.clamp(parsed));
    let normalized = precision.quantize(bounded);
    NumericTextValidation {
        status: if in_range {
            NumericValidationStatus::Valid
        } else {
            NumericValidationStatus::OutOfRange
        },
        parsed: Some(parsed),
        normalized: Some(normalized),
        message: if in_range {
            None
        } else {
            range.map(|range| numeric_range_hint(range, precision))
        },
    }
}

fn numeric_range_hint(range: NumericRange, precision: NumericPrecision) -> String {
    format!(
        "Range {} to {}; step {}",
        precision.format(range.min),
        precision.format(range.max),
        precision.format(precision.step)
    )
}

fn normalize_numeric_clipboard_text(text: &str) -> String {
    let candidate = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .trim_matches(|ch| ch == '"' || ch == '\'')
        .replace(['_', ','], "");
    candidate.trim().to_string()
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn parse_numeric_text(text: &str) -> Option<f64> {
    let value = text.trim().parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}
