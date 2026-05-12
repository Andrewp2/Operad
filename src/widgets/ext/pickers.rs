//! Date, color, numeric, and path picker widget models.
//!
//! These APIs intentionally stop at state, value conversion, and selection
//! helpers. Renderers and application command layers can project the models
//! into their own node trees, menus, dialogs, or native integrations.

use std::path::{Component, Path, PathBuf};

use crate::{
    AccessibilityMeta, AccessibilityRole, AccessibilityValueRange, ColorRgba, EditPhase,
    ImageContent, KeyCode, KeyModifiers, ShaderEffect, UiPoint, UiRect,
};

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

    pub fn iso_string(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn accessibility_label(self) -> String {
        format!("{} {}, {}", month_name(self.month), self.day, self.year)
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

impl CalendarDayCell {
    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        let mut states = Vec::new();
        if self.selected {
            states.push("selected");
        }
        if self.today {
            states.push("today");
        }
        if !self.in_visible_month {
            states.push("outside visible month");
        }
        if self.disabled {
            states.push("unavailable");
        }

        let mut meta = AccessibilityMeta::new(AccessibilityRole::GridCell)
            .label(self.date.accessibility_label())
            .value(self.date.iso_string())
            .selected(self.selected)
            .focusable();
        if !states.is_empty() {
            meta = meta.hint(states.join(", "));
        }
        if self.disabled {
            meta = meta.disabled();
        }
        meta
    }
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

    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(AccessibilityRole::Grid)
            .label(format_month_label(self.visible_month))
            .value(
                self.selected
                    .map_or_else(|| "No date selected".to_string(), CalendarDate::iso_string),
            )
            .focusable();
        if self.min.is_some() || self.max.is_some() {
            meta = meta.hint(format_date_range_hint(self.min, self.max));
        }
        meta
    }

    pub fn control_accessibility_meta(&self, control: DatePickerControl) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(AccessibilityRole::Button)
            .label(control.label(self))
            .action(crate::AccessibilityAction::new("activate", "Activate"))
            .focusable();
        if let Some(value) = control.value(self) {
            meta = meta.value(value);
        }
        if !control.enabled(self) {
            meta = meta.disabled();
        }
        meta
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

    pub fn nearest_selectable_date(&self, date: CalendarDate) -> Option<CalendarDate> {
        let mut date = date;
        if let Some(min) = self.min {
            date = date.max(min);
        }
        if let Some(max) = self.max {
            date = date.min(max);
        }
        self.can_select(date).then_some(date)
    }

    pub fn step_selection(&mut self, step: DatePickerKeyboardStep) -> DatePickerSelection {
        let previous = self.selected;
        let Some(anchor) = self.keyboard_anchor() else {
            return DatePickerSelection {
                previous,
                selected: self.selected,
                phase: EditPhase::Preview,
                changed: false,
            };
        };

        let target = self.keyboard_step_target(anchor, step);
        let target = self.nearest_selectable_date(target).unwrap_or(anchor);
        self.select(target)
    }

    pub fn handle_keyboard_step(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<DatePickerSelection> {
        DatePickerKeyboardStep::from_key(key, modifiers).map(|step| self.step_selection(step))
    }

    fn keyboard_anchor(&self) -> Option<CalendarDate> {
        let preferred = self
            .selected
            .or(self.today)
            .unwrap_or_else(|| self.visible_month.first_day());
        self.nearest_selectable_date(preferred)
    }

    fn keyboard_step_target(
        &self,
        anchor: CalendarDate,
        step: DatePickerKeyboardStep,
    ) -> CalendarDate {
        match step {
            DatePickerKeyboardStep::PreviousDay => anchor.add_days(-1),
            DatePickerKeyboardStep::NextDay => anchor.add_days(1),
            DatePickerKeyboardStep::PreviousWeek => anchor.add_days(-7),
            DatePickerKeyboardStep::NextWeek => anchor.add_days(7),
            DatePickerKeyboardStep::PreviousMonth => anchor.add_months(-1),
            DatePickerKeyboardStep::NextMonth => anchor.add_months(1),
            DatePickerKeyboardStep::PreviousYear => anchor.add_months(-12),
            DatePickerKeyboardStep::NextYear => anchor.add_months(12),
            DatePickerKeyboardStep::StartOfMonth => anchor.month().first_day(),
            DatePickerKeyboardStep::EndOfMonth => {
                let month = anchor.month();
                CalendarDate {
                    year: month.year,
                    month: month.month,
                    day: month.day_count(),
                }
            }
            DatePickerKeyboardStep::FirstSelectable => {
                self.min.unwrap_or_else(|| self.visible_month.first_day())
            }
            DatePickerKeyboardStep::LastSelectable => self.max.unwrap_or_else(|| {
                let month = self.visible_month;
                CalendarDate {
                    year: month.year,
                    month: month.month,
                    day: month.day_count(),
                }
            }),
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePickerKeyboardStep {
    PreviousDay,
    NextDay,
    PreviousWeek,
    NextWeek,
    PreviousMonth,
    NextMonth,
    PreviousYear,
    NextYear,
    StartOfMonth,
    EndOfMonth,
    FirstSelectable,
    LastSelectable,
}

impl DatePickerKeyboardStep {
    pub fn from_key(key: KeyCode, modifiers: KeyModifiers) -> Option<Self> {
        let range_shortcut = modifiers.ctrl || modifiers.meta;
        match key {
            KeyCode::ArrowLeft => Some(if range_shortcut {
                Self::PreviousMonth
            } else {
                Self::PreviousDay
            }),
            KeyCode::ArrowRight => Some(if range_shortcut {
                Self::NextMonth
            } else {
                Self::NextDay
            }),
            KeyCode::ArrowUp => Some(if range_shortcut {
                Self::PreviousYear
            } else {
                Self::PreviousWeek
            }),
            KeyCode::ArrowDown => Some(if range_shortcut {
                Self::NextYear
            } else {
                Self::NextWeek
            }),
            KeyCode::Home => Some(if range_shortcut {
                Self::FirstSelectable
            } else {
                Self::StartOfMonth
            }),
            KeyCode::End => Some(if range_shortcut {
                Self::LastSelectable
            } else {
                Self::EndOfMonth
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePickerControl {
    PreviousMonth,
    NextMonth,
    Today,
    Clear,
}

impl DatePickerControl {
    fn label(self, picker: &DatePickerModel) -> String {
        match self {
            Self::PreviousMonth => format!(
                "Previous month, {}",
                format_month_label(picker.visible_month.previous())
            ),
            Self::NextMonth => format!(
                "Next month, {}",
                format_month_label(picker.visible_month.next())
            ),
            Self::Today => picker.today.map_or_else(
                || "Today unavailable".to_string(),
                |today| format!("Today, {}", today.accessibility_label()),
            ),
            Self::Clear => "Clear selected date".to_string(),
        }
    }

    fn value(self, picker: &DatePickerModel) -> Option<String> {
        match self {
            Self::PreviousMonth | Self::NextMonth => Some(format_month_label(picker.visible_month)),
            Self::Today => picker.today.map(CalendarDate::iso_string),
            Self::Clear => picker.selected.map(CalendarDate::iso_string),
        }
    }

    fn enabled(self, picker: &DatePickerModel) -> bool {
        match self {
            Self::PreviousMonth | Self::NextMonth => true,
            Self::Today => picker.today.is_some_and(|today| picker.can_select(today)),
            Self::Clear => picker.selected.is_some(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickerAnimationMeta {
    pub name: String,
    pub duration_seconds: f32,
}

impl PickerAnimationMeta {
    pub fn new(name: impl Into<String>, duration_seconds: f32) -> Self {
        Self {
            name: name.into(),
            duration_seconds: finite_or_f32(duration_seconds, 0.0).max(0.0),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PickerElementStyle {
    pub foreground: Option<ColorRgba>,
    pub background: Option<ColorRgba>,
    pub border: Option<ColorRgba>,
    pub image: Option<ImageContent>,
    pub shader: Option<ShaderEffect>,
    pub animation: Option<PickerAnimationMeta>,
}

impl PickerElementStyle {
    pub fn with_foreground(mut self, color: ColorRgba) -> Self {
        self.foreground = Some(color);
        self
    }

    pub fn with_background(mut self, color: ColorRgba) -> Self {
        self.background = Some(color);
        self
    }

    pub fn with_border(mut self, color: ColorRgba) -> Self {
        self.border = Some(color);
        self
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatePickerStyle {
    pub day: PickerElementStyle,
    pub outside_month_day: PickerElementStyle,
    pub selected_day: PickerElementStyle,
    pub today_day: PickerElementStyle,
    pub disabled_day: PickerElementStyle,
    pub error_day: PickerElementStyle,
    pub navigation_button: PickerElementStyle,
}

impl DatePickerStyle {
    pub fn style_for_cell(&self, cell: &CalendarDayCell) -> &PickerElementStyle {
        if cell.disabled {
            &self.disabled_day
        } else if cell.selected {
            &self.selected_day
        } else if cell.today {
            &self.today_day
        } else if !cell.in_visible_month {
            &self.outside_month_day
        } else {
            &self.day
        }
    }
}

impl Default for DatePickerStyle {
    fn default() -> Self {
        Self {
            day: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(232, 236, 244, 255))
                .with_background(ColorRgba::new(22, 27, 34, 255)),
            outside_month_day: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(128, 138, 153, 255))
                .with_background(ColorRgba::new(16, 20, 26, 255)),
            selected_day: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(255, 255, 255, 255))
                .with_background(ColorRgba::new(60, 125, 216, 255))
                .with_animation(PickerAnimationMeta::new("date.selected", 0.12)),
            today_day: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(255, 255, 255, 255))
                .with_border(ColorRgba::new(106, 188, 137, 255)),
            disabled_day: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(98, 105, 118, 255))
                .with_background(ColorRgba::new(18, 21, 27, 255)),
            error_day: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(255, 255, 255, 255))
                .with_background(ColorRgba::new(157, 55, 67, 255)),
            navigation_button: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(238, 242, 248, 255))
                .with_background(ColorRgba::new(35, 42, 53, 255)),
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPickerMode {
    OpenFile,
    SaveFile,
    Directory,
    Any,
}

impl PathPickerMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenFile => "Open file",
            Self::SaveFile => "Save file",
            Self::Directory => "Choose directory",
            Self::Any => "Choose path",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathBreadcrumb {
    pub label: String,
    pub path: PathBuf,
    pub is_root: bool,
}

impl PathBreadcrumb {
    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        AccessibilityMeta::new(AccessibilityRole::Button)
            .label(format!("Go to {}", self.label))
            .value(path_to_text(&self.path))
            .focusable()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPickerState {
    pub mode: PathPickerMode,
    pub current_path: PathBuf,
    pub selected_path: Option<PathBuf>,
    pub text: String,
    pub recent_paths: Vec<PathBuf>,
    pub max_recent: usize,
}

impl PathPickerState {
    pub fn new(mode: PathPickerMode, current_path: impl Into<PathBuf>) -> Self {
        Self {
            mode,
            current_path: current_path.into(),
            selected_path: None,
            text: String::new(),
            recent_paths: Vec::new(),
            max_recent: 8,
        }
    }

    pub fn with_selected_path(mut self, selected_path: impl Into<PathBuf>) -> Self {
        let selected_path = selected_path.into();
        self.text = path_to_text(&selected_path);
        self.selected_path = Some(selected_path);
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

    pub fn validation(&self) -> PathTextValidation {
        validate_path_text(&self.text)
    }

    pub fn field_accessibility_meta(&self, label: impl Into<String>) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(AccessibilityRole::TextBox)
            .label(label)
            .value(self.text.clone())
            .hint(format!(
                "{}. Current folder {}",
                self.mode.label(),
                path_to_text(&self.current_path)
            ))
            .focusable();
        if !self.validation().is_valid() && !self.text.is_empty() {
            meta = meta.hint("Enter a path");
        }
        meta
    }

    pub fn control_accessibility_meta(&self, control: PathPickerControl) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(AccessibilityRole::Button)
            .label(control.label(self))
            .focusable();
        if let Some(value) = control.value(self) {
            meta = meta.value(value);
        }
        if !control.enabled(self) {
            meta = meta.disabled();
        }
        meta
    }

    pub fn copy_current_path(&self) -> String {
        path_to_text(&self.current_path)
    }

    pub fn copy_selected_path(&self) -> Option<String> {
        self.selected_path.as_deref().map(path_to_text)
    }

    pub fn copy_text(&self) -> String {
        if self.text.is_empty() {
            self.copy_selected_path()
                .unwrap_or_else(|| self.copy_current_path())
        } else {
            self.text.clone()
        }
    }

    pub fn update_text(&mut self, text: impl Into<String>) -> PathPickerUpdate {
        let previous = self.selected_path.clone();
        let text = text.into();
        let changed = self.text != text;
        self.text = text;
        PathPickerUpdate {
            previous,
            selected_path: self.selected_path.clone(),
            current_path: self.current_path.clone(),
            text: self.text.clone(),
            phase: EditPhase::UpdateEdit,
            changed,
        }
    }

    pub fn paste_path_text(&mut self, text: &str) -> Option<PathPickerUpdate> {
        parse_path_text(text).map(|path| self.select_path(path))
    }

    pub fn commit_text(&mut self) -> PathPickerUpdate {
        let previous = self.selected_path.clone();
        let Some(path) = parse_path_text(&self.text) else {
            return PathPickerUpdate {
                previous,
                selected_path: self.selected_path.clone(),
                current_path: self.current_path.clone(),
                text: self.text.clone(),
                phase: EditPhase::CancelEdit,
                changed: false,
            };
        };
        self.select_path(path)
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
            text: self.text.clone(),
            phase: EditPhase::UpdateEdit,
            changed,
        }
    }

    pub fn select_path(&mut self, path: impl Into<PathBuf>) -> PathPickerUpdate {
        let previous = self.selected_path.clone();
        let path = path.into();
        self.text = path_to_text(&path);
        self.selected_path = Some(path.clone());
        self.remember_recent(path);
        let changed = previous != self.selected_path;
        PathPickerUpdate {
            previous,
            selected_path: self.selected_path.clone(),
            current_path: self.current_path.clone(),
            text: self.text.clone(),
            phase: EditPhase::CommitEdit,
            changed,
        }
    }

    pub fn clear_selection(&mut self) -> PathPickerUpdate {
        let previous = self.selected_path.take();
        self.text.clear();
        PathPickerUpdate {
            previous: previous.clone(),
            selected_path: None,
            current_path: self.current_path.clone(),
            text: self.text.clone(),
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
    pub text: String,
    pub phase: EditPhase,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathTextValidationStatus {
    Valid,
    Empty,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathTextValidation {
    pub status: PathTextValidationStatus,
    pub path: Option<PathBuf>,
    pub message: Option<String>,
}

impl PathTextValidation {
    pub fn is_valid(&self) -> bool {
        self.status == PathTextValidationStatus::Valid
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPickerControl {
    Browse,
    Clear,
    Confirm,
}

impl PathPickerControl {
    fn label(self, picker: &PathPickerState) -> String {
        match self {
            Self::Browse => picker.mode.label().to_string(),
            Self::Clear => "Clear selected path".to_string(),
            Self::Confirm => match picker.mode {
                PathPickerMode::OpenFile => "Open selected path".to_string(),
                PathPickerMode::SaveFile => "Save to selected path".to_string(),
                PathPickerMode::Directory => "Choose selected directory".to_string(),
                PathPickerMode::Any => "Choose selected path".to_string(),
            },
        }
    }

    fn value(self, picker: &PathPickerState) -> Option<String> {
        match self {
            Self::Browse => Some(path_to_text(&picker.current_path)),
            Self::Clear | Self::Confirm => picker.selected_path.as_deref().map(path_to_text),
        }
    }

    fn enabled(self, picker: &PathPickerState) -> bool {
        match self {
            Self::Browse => true,
            Self::Clear | Self::Confirm => picker.selected_path.is_some(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathPickerStyle {
    pub text_field: PickerElementStyle,
    pub invalid_text_field: PickerElementStyle,
    pub browse_button: PickerElementStyle,
    pub breadcrumb_button: PickerElementStyle,
    pub selected_path: PickerElementStyle,
}

impl PathPickerStyle {
    pub fn style_for_validation(&self, validation: &PathTextValidation) -> &PickerElementStyle {
        if validation.is_valid() || validation.status == PathTextValidationStatus::Empty {
            &self.text_field
        } else {
            &self.invalid_text_field
        }
    }
}

impl Default for PathPickerStyle {
    fn default() -> Self {
        Self {
            text_field: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(235, 240, 247, 255))
                .with_background(ColorRgba::new(18, 22, 28, 255)),
            invalid_text_field: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(255, 238, 240, 255))
                .with_border(ColorRgba::new(201, 74, 91, 255)),
            browse_button: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(238, 242, 248, 255))
                .with_background(ColorRgba::new(35, 42, 53, 255))
                .with_image(ImageContent::new("icons.folder")),
            breadcrumb_button: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(199, 209, 223, 255)),
            selected_path: PickerElementStyle::default()
                .with_background(ColorRgba::new(47, 72, 103, 255))
                .with_animation(PickerAnimationMeta::new("path.selected", 0.10)),
        }
    }
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

fn month_name(month: u8) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

fn format_month_label(month: CalendarMonth) -> String {
    format!("{} {} calendar", month_name(month.month), month.year)
}

fn format_date_range_hint(min: Option<CalendarDate>, max: Option<CalendarDate>) -> String {
    match (min, max) {
        (Some(min), Some(max)) => {
            format!(
                "Selectable dates {} through {}",
                min.iso_string(),
                max.iso_string()
            )
        }
        (Some(min), None) => format!("Selectable dates from {}", min.iso_string()),
        (None, Some(max)) => format!("Selectable dates through {}", max.iso_string()),
        (None, None) => String::new(),
    }
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

fn path_to_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn parse_path_text(text: &str) -> Option<PathBuf> {
    let text = normalize_path_clipboard_text(text);
    (!text.is_empty() && !text.contains('\0')).then(|| PathBuf::from(text))
}

fn validate_path_text(text: &str) -> PathTextValidation {
    let text = normalize_path_clipboard_text(text);
    if text.is_empty() {
        return PathTextValidation {
            status: PathTextValidationStatus::Empty,
            path: None,
            message: Some("Enter a path".to_string()),
        };
    }
    if text.contains('\0') {
        return PathTextValidation {
            status: PathTextValidationStatus::Invalid,
            path: None,
            message: Some("Path contains an invalid null byte".to_string()),
        };
    }
    PathTextValidation {
        status: PathTextValidationStatus::Valid,
        path: Some(PathBuf::from(text)),
        message: None,
    }
}

fn normalize_path_clipboard_text(text: &str) -> String {
    let text = text.trim();
    let text = text
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .or_else(|| {
            text.strip_prefix('\'')
                .and_then(|text| text.strip_suffix('\''))
        })
        .unwrap_or(text);
    text.trim().to_string()
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
    use crate::{AccessibilityRole, ImageContent, KeyCode, KeyModifiers, ShaderEffect};

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
    fn date_picker_exposes_accessibility_style_and_keyboard_steps() {
        let min = CalendarDate::new(2024, 5, 10).unwrap();
        let max = CalendarDate::new(2024, 5, 20).unwrap();
        let selected = CalendarDate::new(2024, 5, 15).unwrap();
        let mut picker = DatePickerModel::builder()
            .selected(Some(selected))
            .bounds(Some(min), Some(max))
            .today(Some(selected))
            .build();

        let calendar_meta = picker.accessibility_meta();
        assert_eq!(calendar_meta.role, AccessibilityRole::Grid);
        assert_eq!(calendar_meta.value.as_deref(), Some("2024-05-15"));
        assert!(calendar_meta
            .hint
            .as_deref()
            .unwrap()
            .contains("2024-05-10"));

        let selected_cell = picker
            .grid()
            .into_iter()
            .find(|cell| cell.date == selected)
            .unwrap();
        let cell_meta = selected_cell.accessibility_meta();
        assert_eq!(cell_meta.role, AccessibilityRole::GridCell);
        assert_eq!(cell_meta.label.as_deref(), Some("May 15, 2024"));
        assert_eq!(cell_meta.value.as_deref(), Some("2024-05-15"));
        assert_eq!(cell_meta.hint.as_deref(), Some("selected, today"));

        let style = DatePickerStyle::default();
        assert_eq!(style.style_for_cell(&selected_cell), &style.selected_day);
        assert!(style.selected_day.animation.is_some());

        let moved = picker
            .handle_keyboard_step(KeyCode::ArrowRight, KeyModifiers::NONE)
            .unwrap();
        assert_eq!(
            moved.selected,
            Some(CalendarDate::new(2024, 5, 16).unwrap())
        );

        let jumped = picker
            .handle_keyboard_step(
                KeyCode::End,
                KeyModifiers {
                    ctrl: true,
                    ..KeyModifiers::NONE
                },
            )
            .unwrap();
        assert_eq!(jumped.selected, Some(max));

        let clamped = picker
            .handle_keyboard_step(KeyCode::ArrowRight, KeyModifiers::NONE)
            .unwrap();
        assert_eq!(clamped.selected, Some(max));
        assert!(!clamped.changed);

        let today = picker.control_accessibility_meta(DatePickerControl::Today);
        assert_eq!(today.role, AccessibilityRole::Button);
        assert_eq!(today.value.as_deref(), Some("2024-05-15"));
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
    fn color_picker_exposes_swatch_media_and_channel_accessibility() {
        let swatch = ColorSwatch::new("brand", "Brand", ColorRgba::new(10, 20, 30, 255))
            .with_image(ImageContent::new("swatches.brand"))
            .with_shader(ShaderEffect::new("swatch.checker").uniform("scale", 8.0))
            .with_animation(PickerAnimationMeta::new("swatch.selected", 0.2));
        assert_eq!(swatch.image.as_ref().unwrap().key, "swatches.brand");
        assert_eq!(swatch.shader.as_ref().unwrap().key, "swatch.checker");

        let meta = swatch.accessibility_meta(true);
        assert_eq!(meta.role, AccessibilityRole::Button);
        assert_eq!(meta.label.as_deref(), Some("Brand"));
        assert_eq!(meta.value.as_deref(), Some("#0A141E"));
        assert_eq!(meta.hint.as_deref(), Some("selected"));

        let palette = ColorPalette::new([swatch.clone()]);
        let mut picker =
            ColorPickerState::new(ColorRgba::new(255, 0, 0, 255)).with_palette(palette);
        let palette_meta = picker.palette_accessibility_meta();
        assert_eq!(palette_meta.role, AccessibilityRole::Grid);
        assert_eq!(palette_meta.value.as_deref(), Some("1 swatches"));

        let alpha_meta = picker.channel_accessibility_meta(ColorChannel::Alpha);
        assert_eq!(alpha_meta.role, AccessibilityRole::Slider);
        assert_eq!(alpha_meta.value.as_deref(), Some("100%"));

        let update = picker.nudge_channel(ColorChannel::Alpha, -1, ColorChannelStep::Coarse);
        assert_eq!(update.phase, EditPhase::UpdateEdit);
        assert_eq!(update.value.a, 230);

        let style = ColorPickerStyle::default();
        assert!(style.selected_swatch.animation.is_some());
        assert_eq!(style.style_for_swatch(true, false), &style.selected_swatch);
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
    fn numeric_input_validates_keyboard_steps_and_clipboard_text() {
        let mut input = NumericInputState::new(10.0)
            .with_precision(NumericPrecision::decimals(1).with_step(0.5))
            .with_range(NumericRange::new(0.0, 2000.0));

        let out_of_range = input.validate_text_value("2001");
        assert_eq!(out_of_range.status, NumericValidationStatus::OutOfRange);
        assert_eq!(out_of_range.normalized, Some(2000.0));
        assert!(out_of_range.message.as_deref().unwrap().contains("2000.0"));

        let pasted = input.paste_text(" \"1,234.5\"\n");
        assert_eq!(pasted.value, 1234.5);
        assert_eq!(input.copy_text(), "1234.5");
        assert_eq!(input.copy_value_text(), "1234.5");

        let stepped = input
            .handle_keyboard_step(
                KeyCode::ArrowDown,
                KeyModifiers {
                    shift: true,
                    ..KeyModifiers::NONE
                },
            )
            .unwrap();
        assert_eq!(stepped.value, 1229.5);

        let min = input
            .handle_keyboard_step(KeyCode::Home, KeyModifiers::NONE)
            .unwrap();
        assert_eq!(min.value, 0.0);

        input.update_text("not numeric");
        let meta = input.text_accessibility_meta("Amount");
        assert_eq!(meta.role, AccessibilityRole::TextBox);
        assert_eq!(meta.hint.as_deref(), Some("Enter a finite number"));

        let style = NumericInputStyle::default();
        assert_eq!(
            style.style_for_validation(&input.validation()),
            &style.error_text_field
        );
        assert!(style.drag_handle.image.is_some());
        assert!(style.slider.shader.is_some());
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
    fn numeric_parameter_spec_formats_units_and_log_positions() {
        let frequency = NumericParameterSpec::new(
            "Filter frequency",
            NumericRange::new(20.0, 20_000.0),
            NumericPrecision::decimals(1).with_step(0.1),
        )
        .logarithmic(10.0)
        .unit_suffix(" Hz");

        assert_eq!(frequency.format_value(1234.56), "1234.6 Hz");
        assert_eq!(frequency.parse_text("1,000.25 Hz"), Some(1000.3));

        let midpoint = frequency.value_at_position(0.5);
        assert!((midpoint - 632.5).abs() < 0.2, "{midpoint}");
        assert!((frequency.position_for_value(midpoint) - 0.5).abs() < 0.001);

        let text_meta = frequency.text_accessibility_meta("1000.0 Hz");
        assert_eq!(text_meta.role, AccessibilityRole::TextBox);
        assert_eq!(text_meta.value.as_deref(), Some("1000.0 Hz"));
        assert!(text_meta.actions.iter().any(|action| action.id == "commit"));

        let slider_meta = frequency.slider_accessibility_meta(1000.0);
        assert_eq!(slider_meta.role, AccessibilityRole::Slider);
        assert_eq!(slider_meta.value.as_deref(), Some("1000.0 Hz"));
        assert_eq!(slider_meta.value_range.as_ref().unwrap().min, 20.0);
        assert!(slider_meta
            .actions
            .iter()
            .any(|action| action.id == "increase"));
    }

    #[test]
    fn numeric_slider_geometry_maps_axis_fill_and_thumb_rects() {
        let horizontal =
            SliderGeometry::horizontal(UiRect::new(10.0, 20.0, 100.0, 10.0)).thumb_size(12.0);
        assert_eq!(
            horizontal.position_from_point(UiPoint::new(60.0, 25.0)),
            0.5
        );
        assert_eq!(
            horizontal.fill_rect(0.5),
            UiRect::new(10.0, 20.0, 50.0, 10.0)
        );
        assert_eq!(
            horizontal.thumb_rect(0.5),
            UiRect::new(54.0, 19.0, 12.0, 12.0)
        );

        let vertical =
            SliderGeometry::vertical(UiRect::new(0.0, 0.0, 10.0, 100.0)).thumb_size(10.0);
        assert_eq!(vertical.position_from_point(UiPoint::new(5.0, 25.0)), 0.75);
        assert_eq!(vertical.fill_rect(0.75), UiRect::new(0.0, 25.0, 10.0, 75.0));
        assert_eq!(
            vertical.thumb_rect(0.75),
            UiRect::new(0.0, 20.0, 10.0, 10.0)
        );
    }

    #[test]
    fn numeric_slider_state_tracks_drag_keyboard_and_accessibility() {
        let utilization = NumericParameterSpec::new(
            "Utilization",
            NumericRange::new(0.0, 100.0),
            NumericPrecision::decimals(1).with_step(0.5),
        )
        .unit_suffix("%");
        let geometry = SliderGeometry::horizontal(UiRect::new(0.0, 0.0, 100.0, 8.0));
        let mut slider = NumericSliderState::new(25.2, &utilization);

        assert_eq!(slider.value, 25.0);
        assert_eq!(slider.position(&utilization), 0.25);
        assert_eq!(
            slider.fill_rect(geometry, &utilization),
            UiRect::new(0.0, 0.0, 25.0, 8.0)
        );

        let begin = slider.begin_drag(geometry, UiPoint::new(60.0, 4.0), &utilization);
        assert_eq!(begin.phase, EditPhase::BeginEdit);
        assert_eq!(begin.value, 60.0);
        assert!(begin.changed);
        assert!(slider.dragging.is_some());

        let update = slider.update_drag(geometry, UiPoint::new(80.0, 4.0), &utilization);
        assert_eq!(update.phase, EditPhase::UpdateEdit);
        assert_eq!(update.value, 80.0);

        let commit = slider.end_drag(&utilization);
        assert_eq!(commit.phase, EditPhase::CommitEdit);
        assert_eq!(commit.text, "80.0%");
        assert!(slider.dragging.is_none());

        let decrement = slider
            .handle_keyboard_step(KeyCode::ArrowLeft, KeyModifiers::NONE, &utilization)
            .expect("keyboard step");
        assert_eq!(decrement.value, 79.5);
        assert_eq!(decrement.position, 0.795);

        slider.begin_drag(geometry, UiPoint::new(20.0, 4.0), &utilization);
        let cancel = slider.cancel_drag(&utilization);
        assert_eq!(cancel.phase, EditPhase::CancelEdit);
        assert_eq!(cancel.value, 79.5);

        let meta = slider.accessibility_meta(&utilization);
        assert_eq!(meta.role, AccessibilityRole::Slider);
        assert_eq!(meta.value.as_deref(), Some("79.5%"));
        assert_eq!(meta.value_range.as_ref().unwrap().max, 100.0);
    }

    #[test]
    fn numeric_input_state_applies_parameter_affixes_and_commit_phases() {
        let gain = NumericParameterSpec::new(
            "Gain",
            NumericRange::new(-96.0, 12.0),
            NumericPrecision::decimals(1).with_step(0.1),
        )
        .unit_suffix(" dB");
        let budget = NumericParameterSpec::new(
            "Budget",
            NumericRange::new(0.0, 100.0),
            NumericPrecision::decimals(2).with_step(0.01),
        )
        .unit_prefix("$");
        let mut input = NumericInputState::new(0.0).with_parameter(&gain);

        assert_eq!(input.text, "0.0 dB");
        assert_eq!(budget.format_value(12.0), "$12.00");

        let update = input.update_parameter_text("6.24 dB", &gain);
        assert_eq!(update.phase, EditPhase::UpdateEdit);
        assert_eq!(update.value, 6.2);

        let commit = input.commit_parameter_text(&gain);
        assert_eq!(commit.phase, EditPhase::CommitEdit);
        assert_eq!(commit.text, "6.2 dB");

        let position = gain.position_for_value(-42.0);
        let positioned = input.set_parameter_position(position, EditPhase::UpdateEdit, &gain);
        assert_eq!(positioned.value, -42.0);
        assert_eq!(positioned.text, "-42.0 dB");

        input.update_parameter_text("not gain", &gain);
        let cancel = input.commit_parameter_text(&gain);
        assert_eq!(cancel.phase, EditPhase::CancelEdit);
        assert_eq!(cancel.text, "-42.0 dB");
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
        assert_eq!(picker.text, "/b");

        let nav = picker.navigate_to("/var");
        assert_eq!(nav.phase, EditPhase::UpdateEdit);
        assert_eq!(picker.breadcrumbs().last().unwrap().label, "var");
    }

    #[test]
    fn path_picker_tracks_text_clipboard_and_accessibility() {
        let mut picker = PathPickerState::new(PathPickerMode::SaveFile, "/tmp")
            .with_selected_path("/tmp/report.txt");
        assert_eq!(picker.text, "/tmp/report.txt");
        assert_eq!(
            picker.copy_selected_path().as_deref(),
            Some("/tmp/report.txt")
        );
        assert_eq!(picker.copy_current_path(), "/tmp");

        let field = picker.field_accessibility_meta("Path");
        assert_eq!(field.role, AccessibilityRole::TextBox);
        assert_eq!(field.value.as_deref(), Some("/tmp/report.txt"));
        assert!(field.hint.as_deref().unwrap().contains("Save file"));

        let pasted = picker
            .paste_path_text(" \"/var/tmp/output.txt\" ")
            .expect("valid path");
        assert_eq!(pasted.phase, EditPhase::CommitEdit);
        assert_eq!(
            picker.selected_path,
            Some(PathBuf::from("/var/tmp/output.txt"))
        );
        assert_eq!(picker.text, "/var/tmp/output.txt");

        let breadcrumb = picker.breadcrumbs().last().unwrap().accessibility_meta();
        assert_eq!(breadcrumb.role, AccessibilityRole::Button);
        assert_eq!(breadcrumb.label.as_deref(), Some("Go to tmp"));

        picker.update_text("bad\0path");
        let validation = picker.validation();
        assert_eq!(validation.status, PathTextValidationStatus::Invalid);
        let style = PathPickerStyle::default();
        assert_eq!(
            style.style_for_validation(&validation),
            &style.invalid_text_field
        );
        assert!(style.browse_button.image.is_some());

        let clear = picker.clear_selection();
        assert_eq!(clear.phase, EditPhase::CancelEdit);
        assert_eq!(picker.text, "");
        let clear_meta = picker.control_accessibility_meta(PathPickerControl::Clear);
        assert!(!clear_meta.enabled);
    }
}
