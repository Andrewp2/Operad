//! Date picker widget model and document builder.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, FlexWrap, JustifyContent, Rect as TaffyRect,
    Size as TaffySize, Style,
};

use crate::{
    length, AccessibilityMeta, AccessibilityRole, ClipBehavior, ColorRgba, EditPhase,
    InputBehavior, KeyCode, KeyModifiers, LayoutStyle, StrokeStyle, TextStyle, UiDocument, UiNode,
    UiNodeId, UiNodeStyle, UiVisual,
};

use super::pickers::{PickerAnimationMeta, PickerElementStyle};

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

    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Sunday => "Sun",
            Self::Monday => "Mon",
            Self::Tuesday => "Tue",
            Self::Wednesday => "Wed",
            Self::Thursday => "Thu",
            Self::Friday => "Fri",
            Self::Saturday => "Sat",
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

#[derive(Debug, Clone)]
pub struct DatePickerOptions {
    pub layout: LayoutStyle,
    pub style: DatePickerStyle,
    pub text_style: TextStyle,
    pub action_prefix: Option<String>,
}

impl Default for DatePickerOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: length(264.0),
                    height: Dimension::auto(),
                },
                padding: TaffyRect::length(8.0),
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(8.0),
                    height: taffy::prelude::LengthPercentage::length(8.0),
                },
                ..Default::default()
            }),
            style: DatePickerStyle::default(),
            text_style: TextStyle {
                font_size: 12.0,
                line_height: 16.0,
                color: ColorRgba::new(232, 236, 244, 255),
                ..Default::default()
            },
            action_prefix: None,
        }
    }
}

impl DatePickerOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatePickerNodes {
    pub root: UiNodeId,
    pub previous_month: UiNodeId,
    pub next_month: UiNodeId,
    pub grid: UiNodeId,
}

pub fn date_picker(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    model: &DatePickerModel,
    options: DatePickerOptions,
) -> DatePickerNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(18, 22, 28, 255),
            Some(StrokeStyle::new(ColorRgba::new(48, 58, 72, 255), 1.0)),
            4.0,
        ))
        .with_accessibility(model.accessibility_meta()),
    );

    let header = document.add_child(
        root,
        UiNode::container(
            format!("{name}.header"),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(30.0),
                },
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(6.0),
                    height: taffy::prelude::LengthPercentage::length(6.0),
                },
                ..Default::default()
            }),
        ),
    );
    let previous_month = date_control_button(
        document,
        header,
        format!("{name}.previous"),
        "<",
        model,
        DatePickerControl::PreviousMonth,
        options
            .action_prefix
            .as_deref()
            .map(|prefix| format!("{prefix}.previous")),
        &options,
    );
    document.add_child(
        header,
        UiNode::text(
            format!("{name}.month"),
            format_month_heading(model.visible_month),
            options.text_style.clone(),
            LayoutStyle::from_taffy_style(Style {
                flex_grow: 1.0,
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        ),
    );
    let next_month = date_control_button(
        document,
        header,
        format!("{name}.next"),
        ">",
        model,
        DatePickerControl::NextMonth,
        options
            .action_prefix
            .as_deref()
            .map(|prefix| format!("{prefix}.next")),
        &options,
    );

    let weekdays = document.add_child(
        root,
        UiNode::container(
            format!("{name}.weekdays"),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(4.0),
                    height: taffy::prelude::LengthPercentage::length(4.0),
                },
                ..Default::default()
            }),
        ),
    );
    for weekday in ordered_weekdays(model.first_weekday) {
        document.add_child(
            weekdays,
            UiNode::text(
                format!("{name}.weekday.{}", weekday.number_from_sunday()),
                weekday.short_label(),
                TextStyle {
                    color: ColorRgba::new(145, 156, 173, 255),
                    ..options.text_style.clone()
                },
                LayoutStyle::size(32.0, 16.0),
            ),
        );
    }

    let grid = document.add_child(
        root,
        UiNode::container(
            format!("{name}.grid"),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                size: TaffySize {
                    width: length(248.0),
                    height: Dimension::auto(),
                },
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(4.0),
                    height: taffy::prelude::LengthPercentage::length(4.0),
                },
                ..Default::default()
            }),
        )
        .with_accessibility(model.accessibility_meta()),
    );
    for cell in model.grid() {
        let style = options.style.style_for_cell(&cell);
        let mut node = UiNode::container(
            format!("{name}.day.{}", cell.date.iso_string()),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                size: TaffySize {
                    width: length(32.0),
                    height: length(28.0),
                },
                ..Default::default()
            }),
        )
        .with_visual(UiVisual::panel(
            style
                .background
                .unwrap_or_else(|| ColorRgba::new(22, 27, 34, 255)),
            style.border.map(|color| StrokeStyle::new(color, 1.0)),
            3.0,
        ))
        .with_accessibility(cell.accessibility_meta());
        if !cell.disabled {
            node = node.with_input(InputBehavior::BUTTON);
            if let Some(prefix) = options.action_prefix.as_deref() {
                node = node.with_action(format!("{prefix}.day.{}", cell.date.iso_string()));
            }
        }
        let day = document.add_child(grid, node);
        document.add_child(
            day,
            UiNode::text(
                format!("{name}.day.{}.label", cell.date.iso_string()),
                cell.date.day.to_string(),
                TextStyle {
                    color: style
                        .foreground
                        .unwrap_or_else(|| ColorRgba::new(232, 236, 244, 255)),
                    ..options.text_style.clone()
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

    DatePickerNodes {
        root,
        previous_month,
        next_month,
        grid,
    }
}

fn date_control_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    model: &DatePickerModel,
    control: DatePickerControl,
    action: Option<String>,
    options: &DatePickerOptions,
) -> UiNodeId {
    let name = name.into();
    let enabled = control.enabled(model);
    let style = &options.style.navigation_button;
    let mut node = UiNode::container(
        name.clone(),
        LayoutStyle::from_taffy_style(Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            justify_content: Some(JustifyContent::Center),
            size: TaffySize {
                width: length(30.0),
                height: length(28.0),
            },
            ..Default::default()
        }),
    )
    .with_visual(UiVisual::panel(
        style
            .background
            .unwrap_or_else(|| ColorRgba::new(35, 42, 53, 255)),
        style.border.map(|color| StrokeStyle::new(color, 1.0)),
        3.0,
    ))
    .with_accessibility(model.control_accessibility_meta(control));
    if enabled {
        node = node.with_input(InputBehavior::BUTTON);
        if let Some(action) = action {
            node = node.with_action(action);
        }
    }
    let id = document.add_child(parent, node);
    document.add_child(
        id,
        UiNode::text(
            format!("{name}.label"),
            label,
            options.text_style.clone(),
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        ),
    );
    id
}

fn ordered_weekdays(first: Weekday) -> [Weekday; 7] {
    let all = Weekday::ALL;
    let start = first.number_from_sunday() as usize;
    [
        all[start % 7],
        all[(start + 1) % 7],
        all[(start + 2) % 7],
        all[(start + 3) % 7],
        all[(start + 4) % 7],
        all[(start + 5) % 7],
        all[(start + 6) % 7],
    ]
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

fn format_month_heading(month: CalendarMonth) -> String {
    format!("{} {}", month_name(month.month), month.year)
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
