//! Date, color, numeric, and path picker widget models.
//!
//! These APIs intentionally stop at state, value conversion, and selection
//! helpers. Renderers and application command layers can project the models
//! into their own node trees, menus, dialogs, or native integrations.

use std::path::{Component, Path, PathBuf};

use crate::{ColorRgba, EditPhase};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl CalendarDate {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }
        if !(1..=Self::days_in_month(year, month)).contains(&day) {
            return None;
        }
        Some(Self { year, month, day })
    }

    pub fn clamp_day(year: i32, month: u8, day: u8) -> Self {
        let month = month.clamp(1, 12);
        let day = day.clamp(1, Self::days_in_month(year, month));
        Self { year, month, day }
    }

    pub const fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    pub const fn days_in_month(year: i32, month: u8) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap_year(year) => 29,
            2 => 28,
            _ => 0,
        }
    }

    pub fn weekday(self) -> Weekday {
        let days = days_from_civil(self.year, self.month, self.day);
        Weekday::from_number_from_sunday((days + 4).rem_euclid(7) as u8)
    }

    pub const fn month(self) -> CalendarMonth {
        CalendarMonth {
            year: self.year,
            month: self.month,
        }
    }

    pub fn add_days(self, days: i32) -> Self {
        civil_from_days(days_from_civil(self.year, self.month, self.day) + i64::from(days))
    }

    pub fn add_months(self, months: i32) -> Self {
        let month = self.month().shifted(months);
        Self::clamp_day(month.year, month.month, self.day)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CalendarMonth {
    pub year: i32,
    pub month: u8,
}

impl CalendarMonth {
    pub fn new(year: i32, month: u8) -> Option<Self> {
        (1..=12).contains(&month).then_some(Self { year, month })
    }

    pub fn first_day(self) -> CalendarDate {
        CalendarDate {
            year: self.year,
            month: self.month,
            day: 1,
        }
    }

    pub fn day_count(self) -> u8 {
        CalendarDate::days_in_month(self.year, self.month)
    }

    pub fn shifted(self, months: i32) -> Self {
        let zero_based = self.year * 12 + i32::from(self.month) - 1 + months;
        Self {
            year: zero_based.div_euclid(12),
            month: (zero_based.rem_euclid(12) + 1) as u8,
        }
    }

    pub fn previous(self) -> Self {
        self.shifted(-1)
    }

    pub fn next(self) -> Self {
        self.shifted(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Weekday {
    Sunday = 0,
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
}

impl Weekday {
    pub const ALL: [Self; 7] = [
        Self::Sunday,
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
    ];

    pub const fn number_from_sunday(self) -> u8 {
        self as u8
    }

    pub const fn from_number_from_sunday(number: u8) -> Self {
        match number % 7 {
            0 => Self::Sunday,
            1 => Self::Monday,
            2 => Self::Tuesday,
            3 => Self::Wednesday,
            4 => Self::Thursday,
            5 => Self::Friday,
            _ => Self::Saturday,
        }
    }

    pub fn days_since(self, first_weekday: Self) -> usize {
        (i16::from(self.number_from_sunday()) - i16::from(first_weekday.number_from_sunday()))
            .rem_euclid(7) as usize
    }
}

impl Default for Weekday {
    fn default() -> Self {
        Self::Sunday
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDayCell {
    pub date: CalendarDate,
    pub in_visible_month: bool,
    pub selected: bool,
    pub today: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatePickerModel {
    pub selected: Option<CalendarDate>,
    pub visible_month: CalendarMonth,
    pub min: Option<CalendarDate>,
    pub max: Option<CalendarDate>,
    pub first_weekday: Weekday,
    pub today: Option<CalendarDate>,
}

impl DatePickerModel {
    pub fn builder() -> DatePickerBuilder {
        DatePickerBuilder::default()
    }

    pub fn new(selected: Option<CalendarDate>) -> Self {
        Self::builder().selected(selected).build()
    }

    pub fn can_select(&self, date: CalendarDate) -> bool {
        self.min.is_none_or(|min| date >= min) && self.max.is_none_or(|max| date <= max)
    }

    pub fn select(&mut self, date: CalendarDate) -> DatePickerSelection {
        let previous = self.selected;
        if !self.can_select(date) {
            return DatePickerSelection {
                previous,
                selected: self.selected,
                phase: EditPhase::Preview,
                changed: false,
            };
        }

        self.selected = Some(date);
        self.visible_month = date.month();
        DatePickerSelection {
            previous,
            selected: self.selected,
            phase: EditPhase::CommitEdit,
            changed: previous != self.selected,
        }
    }

    pub fn show_month(&mut self, month: CalendarMonth) {
        self.visible_month = month;
    }

    pub fn show_previous_month(&mut self) {
        self.visible_month = self.visible_month.previous();
    }

    pub fn show_next_month(&mut self) {
        self.visible_month = self.visible_month.next();
    }

    pub fn grid(&self) -> Vec<CalendarDayCell> {
        let first = self.visible_month.first_day();
        let leading_days = first.weekday().days_since(self.first_weekday) as i32;
        let start = first.add_days(-leading_days);

        (0..42)
            .map(|offset| {
                let date = start.add_days(offset);
                CalendarDayCell {
                    date,
                    in_visible_month: date.month() == self.visible_month,
                    selected: self.selected == Some(date),
                    today: self.today == Some(date),
                    disabled: !self.can_select(date),
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatePickerBuilder {
    selected: Option<CalendarDate>,
    visible_month: Option<CalendarMonth>,
    min: Option<CalendarDate>,
    max: Option<CalendarDate>,
    first_weekday: Weekday,
    today: Option<CalendarDate>,
}

impl DatePickerBuilder {
    pub fn selected(mut self, selected: Option<CalendarDate>) -> Self {
        self.selected = selected;
        self
    }

    pub fn visible_month(mut self, visible_month: CalendarMonth) -> Self {
        self.visible_month = Some(visible_month);
        self
    }

    pub fn min(mut self, min: Option<CalendarDate>) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: Option<CalendarDate>) -> Self {
        self.max = max;
        self
    }

    pub fn bounds(mut self, min: Option<CalendarDate>, max: Option<CalendarDate>) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn first_weekday(mut self, first_weekday: Weekday) -> Self {
        self.first_weekday = first_weekday;
        self
    }

    pub fn today(mut self, today: Option<CalendarDate>) -> Self {
        self.today = today;
        self
    }

    pub fn build(self) -> DatePickerModel {
        let (min, max) = ordered_bounds(self.min, self.max);
        let selected = self.selected.filter(|date| {
            min.is_none_or(|min| *date >= min) && max.is_none_or(|max| *date <= max)
        });
        let anchor = selected.or(self.today).or(min).unwrap_or(CalendarDate {
            year: 1970,
            month: 1,
            day: 1,
        });

        DatePickerModel {
            selected,
            visible_month: self.visible_month.unwrap_or_else(|| anchor.month()),
            min,
            max,
            first_weekday: self.first_weekday,
            today: self.today,
        }
    }
}

impl Default for DatePickerBuilder {
    fn default() -> Self {
        Self {
            selected: None,
            visible_month: None,
            min: None,
            max: None,
            first_weekday: Weekday::Sunday,
            today: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatePickerSelection {
    pub previous: Option<CalendarDate>,
    pub selected: Option<CalendarDate>,
    pub phase: EditPhase,
    pub changed: bool,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorSwatch {
    pub id: String,
    pub label: String,
    pub color: ColorRgba,
}

impl ColorSwatch {
    pub fn new(id: impl Into<String>, label: impl Into<String>, color: ColorRgba) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPickerState {
    pub value: ColorRgba,
    pub palette: ColorPalette,
    pub recent: Vec<ColorRgba>,
    pub max_recent: usize,
}

impl ColorPickerState {
    pub fn new(value: ColorRgba) -> Self {
        Self {
            value,
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
        ColorHsv::from_rgba(self.value)
    }

    pub fn set_rgba(&mut self, value: ColorRgba, phase: EditPhase) -> ColorPickerUpdate {
        let previous = self.value;
        self.value = value;
        if phase == EditPhase::CommitEdit {
            self.remember_recent(value);
        }
        ColorPickerUpdate {
            previous,
            value: self.value,
            hsv: self.hsv(),
            phase,
            changed: previous != self.value,
        }
    }

    pub fn set_hsv(&mut self, value: ColorHsv, phase: EditPhase) -> ColorPickerUpdate {
        self.set_rgba(value.to_rgba(), phase)
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPickerUpdate {
    pub previous: ColorRgba,
    pub value: ColorRgba,
    pub hsv: ColorHsv,
    pub phase: EditPhase,
    pub changed: bool,
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

    pub fn cancel_edit(&mut self) -> NumericInputOutcome {
        self.phase = EditPhase::CancelEdit;
        self.text = self.precision.format(self.value);
        self.outcome(self.value, false)
    }

    pub fn set_value(&mut self, value: f64, phase: EditPhase) -> NumericInputOutcome {
        let previous = self.value;
        self.value = self.normalize_value(value);
        self.text = self.precision.format(self.value);
        self.phase = phase;
        self.outcome(previous, previous != self.value)
    }

    pub fn nudge(&mut self, steps: i32) -> NumericInputOutcome {
        self.set_value(
            self.value + self.precision.step * f64::from(steps),
            EditPhase::UpdateEdit,
        )
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPickerMode {
    OpenFile,
    SaveFile,
    Directory,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathBreadcrumb {
    pub label: String,
    pub path: PathBuf,
    pub is_root: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPickerState {
    pub mode: PathPickerMode,
    pub current_path: PathBuf,
    pub selected_path: Option<PathBuf>,
    pub recent_paths: Vec<PathBuf>,
    pub max_recent: usize,
}

impl PathPickerState {
    pub fn new(mode: PathPickerMode, current_path: impl Into<PathBuf>) -> Self {
        Self {
            mode,
            current_path: current_path.into(),
            selected_path: None,
            recent_paths: Vec::new(),
            max_recent: 8,
        }
    }

    pub fn with_selected_path(mut self, selected_path: impl Into<PathBuf>) -> Self {
        self.selected_path = Some(selected_path.into());
        self
    }

    pub fn with_recent_paths(mut self, recent_paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let paths: Vec<_> = recent_paths.into_iter().collect();
        for path in paths.into_iter().rev() {
            self.remember_recent(path);
        }
        self
    }

    pub fn breadcrumbs(&self) -> Vec<PathBreadcrumb> {
        path_breadcrumbs(&self.current_path)
    }

    pub fn navigate_to(&mut self, path: impl Into<PathBuf>) -> PathPickerUpdate {
        let previous = self.selected_path.clone();
        let path = path.into();
        let changed = self.current_path != path;
        self.current_path = path;
        PathPickerUpdate {
            previous,
            selected_path: self.selected_path.clone(),
            current_path: self.current_path.clone(),
            phase: EditPhase::UpdateEdit,
            changed,
        }
    }

    pub fn select_path(&mut self, path: impl Into<PathBuf>) -> PathPickerUpdate {
        let previous = self.selected_path.clone();
        let path = path.into();
        self.selected_path = Some(path.clone());
        self.remember_recent(path);
        let changed = previous != self.selected_path;
        PathPickerUpdate {
            previous,
            selected_path: self.selected_path.clone(),
            current_path: self.current_path.clone(),
            phase: EditPhase::CommitEdit,
            changed,
        }
    }

    pub fn clear_selection(&mut self) -> PathPickerUpdate {
        let previous = self.selected_path.take();
        PathPickerUpdate {
            previous: previous.clone(),
            selected_path: None,
            current_path: self.current_path.clone(),
            phase: EditPhase::CancelEdit,
            changed: previous.is_some(),
        }
    }

    pub fn remember_recent(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        self.recent_paths.retain(|recent| recent != &path);
        self.recent_paths.insert(0, path);
        self.recent_paths.truncate(self.max_recent);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPickerUpdate {
    pub previous: Option<PathBuf>,
    pub selected_path: Option<PathBuf>,
    pub current_path: PathBuf,
    pub phase: EditPhase,
    pub changed: bool,
}

pub fn path_breadcrumbs(path: impl AsRef<Path>) -> Vec<PathBreadcrumb> {
    let mut crumbs = Vec::new();
    let mut current = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            Component::Prefix(prefix) => {
                current.push(prefix.as_os_str());
                crumbs.push(PathBreadcrumb {
                    label: prefix.as_os_str().to_string_lossy().into_owned(),
                    path: current.clone(),
                    is_root: true,
                });
            }
            Component::RootDir => {
                current.push(component.as_os_str());
                crumbs.push(PathBreadcrumb {
                    label: std::path::MAIN_SEPARATOR.to_string(),
                    path: current.clone(),
                    is_root: true,
                });
            }
            Component::Normal(part) => {
                current.push(part);
                crumbs.push(PathBreadcrumb {
                    label: part.to_string_lossy().into_owned(),
                    path: current.clone(),
                    is_root: false,
                });
            }
            Component::ParentDir => {
                current.push("..");
                crumbs.push(PathBreadcrumb {
                    label: "..".to_string(),
                    path: current.clone(),
                    is_root: false,
                });
            }
            Component::CurDir => {}
        }
    }

    if crumbs.is_empty() {
        crumbs.push(PathBreadcrumb {
            label: ".".to_string(),
            path: PathBuf::from("."),
            is_root: false,
        });
    }

    crumbs
}

fn ordered_bounds(
    min: Option<CalendarDate>,
    max: Option<CalendarDate>,
) -> (Option<CalendarDate>, Option<CalendarDate>) {
    match (min, max) {
        (Some(min), Some(max)) if min > max => (Some(max), Some(min)),
        bounds => bounds,
    }
}

fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> CalendarDate {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);

    CalendarDate {
        year: year as i32,
        month: month as u8,
        day: day as u8,
    }
}

fn normalize_hue(hue: f32) -> f32 {
    if hue.is_finite() {
        hue.rem_euclid(360.0)
    } else {
        0.0
    }
}

fn unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn channel(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

fn hex_nibble(value: char) -> Option<u8> {
    value.to_digit(16).map(|value| value as u8)
}

fn hex_byte(value: &str) -> Option<u8> {
    u8::from_str_radix(value, 16).ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_dates_validate_leap_years_and_weekdays() {
        assert_eq!(CalendarDate::days_in_month(2024, 2), 29);
        assert_eq!(CalendarDate::days_in_month(2023, 2), 28);
        assert_eq!(CalendarDate::new(2023, 2, 29), None);

        let leap_day = CalendarDate::new(2024, 2, 29).unwrap();
        assert_eq!(leap_day.weekday(), Weekday::Thursday);
        assert_eq!(
            leap_day.add_days(1),
            CalendarDate {
                year: 2024,
                month: 3,
                day: 1
            }
        );
        assert_eq!(
            CalendarDate {
                year: 2024,
                month: 3,
                day: 31
            }
            .add_months(-1),
            CalendarDate {
                year: 2024,
                month: 2,
                day: 29
            }
        );
    }

    #[test]
    fn date_picker_builder_filters_bounds_and_builds_grid() {
        let min = CalendarDate::new(2024, 5, 10).unwrap();
        let max = CalendarDate::new(2024, 5, 20).unwrap();
        let selected = CalendarDate::new(2024, 5, 15).unwrap();
        let mut picker = DatePickerModel::builder()
            .selected(Some(selected))
            .bounds(Some(max), Some(min))
            .first_weekday(Weekday::Monday)
            .today(Some(CalendarDate::new(2024, 5, 12).unwrap()))
            .build();

        assert_eq!(picker.min, Some(min));
        assert_eq!(picker.max, Some(max));
        assert_eq!(picker.visible_month, selected.month());

        let cells = picker.grid();
        assert_eq!(cells.len(), 42);
        assert_eq!(cells[0].date, CalendarDate::new(2024, 4, 29).unwrap());
        assert!(cells
            .iter()
            .any(|cell| cell.selected && cell.date == selected));
        assert!(cells.iter().any(|cell| cell.today));
        assert!(
            cells
                .iter()
                .find(|cell| cell.date == CalendarDate::new(2024, 5, 9).unwrap())
                .unwrap()
                .disabled
        );

        let rejected = picker.select(CalendarDate::new(2024, 5, 21).unwrap());
        assert_eq!(rejected.phase, EditPhase::Preview);
        assert_eq!(picker.selected, Some(selected));

        let accepted = picker.select(CalendarDate::new(2024, 5, 20).unwrap());
        assert_eq!(accepted.phase, EditPhase::CommitEdit);
        assert!(accepted.changed);
    }

    #[test]
    fn hsv_and_hex_helpers_round_trip_rgba() {
        let color = ColorRgba::new(51, 102, 153, 128);
        let hsv = ColorHsv::from_rgba(color);
        assert!((hsv.hue - 210.0).abs() < 0.01);
        assert!((hsv.saturation - (2.0 / 3.0)).abs() < 0.01);
        assert_eq!(hsv.to_rgba(), color);

        assert_eq!(format_hex_color(color, true), "#33669980");
        assert_eq!(
            parse_hex_color("#3698"),
            Some(ColorRgba::new(51, 102, 153, 136))
        );
        assert_eq!(
            parse_hex_color("336699"),
            Some(ColorRgba::new(51, 102, 153, 255))
        );
        assert_eq!(parse_hex_color("not-a-color"), None);
    }

    #[test]
    fn color_picker_selects_swatches_and_tracks_recent_colors() {
        let palette = ColorPalette::new([
            ColorSwatch::new("red", "Red", ColorRgba::new(255, 0, 0, 255)),
            ColorSwatch::new("blue", "Blue", ColorRgba::new(0, 0, 255, 255)),
        ]);
        let mut picker = ColorPickerState::new(ColorRgba::new(0, 0, 0, 255))
            .with_palette(palette)
            .with_recent([ColorRgba::new(1, 2, 3, 255), ColorRgba::new(4, 5, 6, 255)]);

        let update = picker.select_swatch("blue").unwrap();
        assert_eq!(update.phase, EditPhase::CommitEdit);
        assert_eq!(picker.value, ColorRgba::new(0, 0, 255, 255));
        assert_eq!(picker.recent[0], picker.value);

        picker.remember_recent(ColorRgba::new(1, 2, 3, 255));
        assert_eq!(picker.recent[0], ColorRgba::new(1, 2, 3, 255));
        assert_eq!(
            picker
                .recent
                .iter()
                .filter(|color| **color == ColorRgba::new(1, 2, 3, 255))
                .count(),
            1
        );
    }

    #[test]
    fn numeric_input_clamps_quantizes_and_reports_phases() {
        let mut input = NumericInputState::new(0.0)
            .with_precision(NumericPrecision::decimals(2).with_step(0.25))
            .with_range(NumericRange::new(-1.0, 1.0));

        assert_eq!(input.begin_edit().phase, EditPhase::BeginEdit);
        let update = input.update_text("0.62");
        assert_eq!(update.phase, EditPhase::UpdateEdit);
        assert_eq!(update.value, 0.5);

        let commit = input.commit_text();
        assert_eq!(commit.phase, EditPhase::CommitEdit);
        assert_eq!(commit.text, "0.50");

        let nudged = input.nudge(10);
        assert_eq!(nudged.value, 1.0);

        let canceled = input.update_text("NaN");
        assert!(!canceled.changed);
        let canceled = input.commit_text();
        assert_eq!(canceled.phase, EditPhase::CancelEdit);
        assert_eq!(canceled.text, "1.00");
    }

    #[test]
    fn numeric_drag_uses_precision_speed_and_range() {
        let precision = NumericPrecision::decimals(1).with_step(0.5);
        let range = Some(NumericRange::new(0.0, 3.0));
        let drag = NumericDragSpec {
            pixels_per_step: 10.0,
            ..Default::default()
        };

        assert_eq!(
            drag_value(1.0, 20.0, precision, range, drag, NumericDragSpeed::Normal),
            2.0
        );
        assert_eq!(
            drag_value(1.0, 100.0, precision, range, drag, NumericDragSpeed::Coarse),
            3.0
        );
        assert_eq!(
            drag_value(1.0, 10.0, precision, range, drag, NumericDragSpeed::Fine),
            1.0
        );
    }

    #[test]
    fn path_picker_builds_breadcrumbs_and_dedupes_recent_paths() {
        let breadcrumbs = path_breadcrumbs(Path::new("/tmp/project/file.txt"));
        assert_eq!(breadcrumbs[0].label, std::path::MAIN_SEPARATOR.to_string());
        assert!(breadcrumbs[0].is_root);
        assert_eq!(breadcrumbs.last().unwrap().label, "file.txt");
        assert_eq!(
            breadcrumbs.last().unwrap().path,
            PathBuf::from("/tmp/project/file.txt")
        );

        let mut picker = PathPickerState::new(PathPickerMode::OpenFile, "/tmp")
            .with_recent_paths([PathBuf::from("/a"), PathBuf::from("/b")]);
        assert_eq!(
            picker.recent_paths,
            vec![PathBuf::from("/a"), PathBuf::from("/b")]
        );

        let update = picker.select_path("/b");
        assert_eq!(update.phase, EditPhase::CommitEdit);
        assert_eq!(picker.recent_paths[0], PathBuf::from("/b"));
        assert_eq!(picker.recent_paths.len(), 2);

        let nav = picker.navigate_to("/var");
        assert_eq!(nav.phase, EditPhase::UpdateEdit);
        assert_eq!(picker.breadcrumbs().last().unwrap().label, "var");
    }
}
