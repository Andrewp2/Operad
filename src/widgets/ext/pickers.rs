//! Shared picker primitives and legacy picker exports.
//!
//! Concrete picker widget implementations live in the dedicated widget modules:
//! `color_picker`, `date_picker`, `numeric_input`, and `path_picker`.

use crate::{ColorRgba, ImageContent, ShaderEffect};

pub use super::color_picker::{
    color_picker, format_hex_color, parse_hex_color, ColorChannel, ColorChannelStep, ColorHsv,
    ColorOklch, ColorOklchChannel, ColorPalette, ColorPickerMode, ColorPickerNodes,
    ColorPickerOptions, ColorPickerState, ColorPickerStyle, ColorPickerUpdate, ColorSwatch,
};
pub use super::date_picker::{
    CalendarDate, CalendarDayCell, CalendarMonth, DatePickerBuilder, DatePickerControl,
    DatePickerKeyboardStep, DatePickerModel, DatePickerNodes, DatePickerOptions,
    DatePickerSelection, DatePickerStyle, Weekday,
};
pub use super::numeric_input::{
    drag_value, NumericDragSpec, NumericDragSpeed, NumericInputOutcome, NumericInputState,
    NumericInputStyle, NumericKeyboardStep, NumericParameterSpec, NumericPrecision, NumericRange,
    NumericScale, NumericSliderDrag, NumericSliderOutcome, NumericSliderState,
    NumericTextValidation, NumericUnitFormat, NumericValidationStatus, SliderAxis, SliderGeometry,
};
pub use super::path_picker::{
    path_breadcrumbs, PathBreadcrumb, PathPickerControl, PathPickerMode, PathPickerState,
    PathPickerStyle, PathPickerUpdate, PathTextValidation, PathTextValidationStatus,
};

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

fn finite_or_f32(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::{
        root_style, AccessibilityRole, ApproxTextMeasurer, ColorRgba, EditPhase, ImageContent,
        KeyCode, KeyModifiers, PaintBrush, ShaderEffect, UiContent, UiDocument, UiNodeId, UiPoint,
        UiRect, UiSize, WidgetActionBinding,
    };

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
    fn color_picker_preserves_hue_when_value_or_saturation_reaches_zero() {
        let mut picker = ColorPickerState::new(ColorHsv::new(214.0, 0.68, 0.92, 1.0).to_rgba());
        let preserved_hue = picker.hsv().hue;
        picker.set_field_position(
            UiPoint::new(0.0, 112.0),
            UiSize::new(204.0, 112.0),
            EditPhase::UpdateEdit,
        );

        assert!((picker.hsv().hue - preserved_hue).abs() < 0.01);
        assert_eq!(picker.hsv().value, 0.0);

        picker.set_field_position(
            UiPoint::new(102.0, 56.0),
            UiSize::new(204.0, 112.0),
            EditPhase::UpdateEdit,
        );

        assert!((picker.hsv().hue - preserved_hue).abs() < 0.01);
        assert!((picker.hsv().saturation - 0.5).abs() < 0.01);
        assert!((picker.hsv().value - 0.5).abs() < 0.01);
        assert_ne!(picker.value.r, 255, "hue must not snap to red");
    }

    #[test]
    fn color_picker_keeps_hue_slider_endpoint_at_right_edge() {
        let mut picker = ColorPickerState::new(ColorRgba::new(0, 128, 255, 255));

        picker.set_channel_position(
            ColorChannel::Hue,
            UiPoint::new(124.0, 6.0),
            UiSize::new(124.0, 12.0),
            EditPhase::UpdateEdit,
        );

        assert_eq!(picker.hsv().hue, 360.0);
        assert_eq!(picker.value, ColorRgba::new(255, 0, 0, 255));
    }

    #[test]
    fn color_picker_oklch_mode_edits_lightness_chroma_hue_and_alpha() {
        let mut picker = ColorPickerState::new(ColorRgba::new(118, 183, 255, 255))
            .with_mode(ColorPickerMode::Oklch);

        picker.set_field_position(
            UiPoint::new(102.0, 28.0),
            UiSize::new(204.0, 112.0),
            EditPhase::UpdateEdit,
        );
        assert!((picker.oklch().chroma - 0.2).abs() < 0.001);
        assert!((picker.oklch().lightness - 0.75).abs() < 0.001);

        picker.set_oklch_channel_position(
            ColorOklchChannel::Hue,
            UiPoint::new(124.0, 6.0),
            UiSize::new(124.0, 12.0),
            EditPhase::UpdateEdit,
        );
        assert_eq!(picker.oklch().hue, 360.0);

        picker.set_oklch_channel(ColorOklchChannel::Alpha, 0.5, EditPhase::UpdateEdit);
        assert_eq!(picker.value.a, 128);
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
    fn color_picker_builder_creates_swatch_actions_and_channels() {
        let state = ColorPickerState::new(ColorRgba::new(255, 0, 0, 255)).with_palette(
            ColorPalette::new([
                ColorSwatch::new("red", "Red", ColorRgba::new(255, 0, 0, 255)),
                ColorSwatch::new("blue", "Blue", ColorRgba::new(0, 0, 255, 255)),
            ]),
        );
        let mut document = UiDocument::new(root_style(240.0, 180.0));
        let root = document.root;

        let nodes = color_picker(
            &mut document,
            root,
            "color",
            &state,
            ColorPickerOptions::default().with_swatch_action_prefix("color.swatch"),
        );

        assert_eq!(nodes.swatches.len(), 2);
        assert_eq!(nodes.channels.len(), ColorChannel::ALL.len());
        assert!(document
            .nodes()
            .iter()
            .any(|node| node.name == "color.field.saturation"
                && matches!(
                    &node.content,
                    UiContent::PaintRect(rect)
                        if matches!(rect.fill, PaintBrush::LinearGradient(_))
                )));
        assert!(document
            .nodes()
            .iter()
            .any(|node| node.name == "color.channel.hue.track"
                && matches!(
                    &node.content,
                    UiContent::PaintRect(rect)
                        if matches!(rect.fill, PaintBrush::LinearGradient(_))
                )));
        assert_eq!(
            document
                .node(nodes.root)
                .accessibility
                .as_ref()
                .unwrap()
                .role,
            AccessibilityRole::Grid
        );
        assert_eq!(
            document
                .node(nodes.swatches[0])
                .action
                .as_ref()
                .and_then(WidgetActionBinding::action_id)
                .map(|id| id.as_str()),
            Some("color.swatch.red")
        );
    }

    #[test]
    fn color_picker_builder_builds_oklch_mode_controls() {
        let state = ColorPickerState::new(ColorRgba::new(118, 183, 255, 255))
            .with_mode(ColorPickerMode::Oklch);
        let mut document = UiDocument::new(root_style(240.0, 220.0));
        let root = document.root;

        let nodes = color_picker(
            &mut document,
            root,
            "color",
            &state,
            ColorPickerOptions::default().with_action_prefix("color"),
        );

        assert_eq!(nodes.channels.len(), ColorOklchChannel::ALL.len());
        assert!(document
            .nodes()
            .iter()
            .any(|node| node.name == "color.field.oklch"
                && matches!(&node.content, UiContent::Scene(_))));
        let field = document
            .nodes()
            .iter()
            .find(|node| node.name == "color.field.oklch")
            .unwrap();
        let UiContent::Scene(primitives) = &field.content else {
            unreachable!("checked above");
        };
        assert_eq!(primitives.len(), 112);
        assert!(primitives.iter().all(|primitive| matches!(
            primitive,
            crate::ScenePrimitive::Rect(rect)
                if matches!(rect.fill, PaintBrush::LinearGradient(_))
        )));
        assert!(document
            .nodes()
            .iter()
            .any(|node| node.name == "color.channel.lightness.track"
                && matches!(
                    &node.content,
                    UiContent::PaintRect(rect)
                        if matches!(rect.fill, PaintBrush::LinearGradient(_))
                )));
        assert_eq!(
            document
                .nodes()
                .iter()
                .find(|node| node.name == "color.mode.oklch")
                .and_then(|node| node.action.as_ref())
                .and_then(WidgetActionBinding::action_id)
                .map(|id| id.as_str()),
            Some("color.mode.oklch")
        );
    }

    #[test]
    fn color_picker_default_layout_matches_fixed_controls() {
        let state = ColorPickerState::new(ColorRgba::new(118, 183, 255, 255));
        let mut document = UiDocument::new(root_style(320.0, 360.0));
        let root = document.root;

        color_picker(
            &mut document,
            root,
            "color",
            &state,
            ColorPickerOptions::default(),
        );
        document
            .compute_layout(UiSize::new(320.0, 360.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let picker = node_by_name(&document, "color");
        let field = node_by_name(&document, "color.field");
        let track = node_by_name(&document, "color.channel.hue.track");
        let picker_rect = document.node(picker).layout.rect;
        let field_rect = document.node(field).layout.rect;
        let track_rect = document.node(track).layout.rect;

        assert_eq!(picker_rect.width, 220.0);
        assert_eq!(field_rect.width, 204.0);
        assert_eq!(track_rect.width, 124.0);
        assert!((field_rect.x - (picker_rect.x + 8.0)).abs() < 0.01);
        assert!(picker_rect.width <= field_rect.width + 20.0);
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

    fn node_by_name(document: &UiDocument, name: &str) -> UiNodeId {
        document
            .nodes()
            .iter()
            .enumerate()
            .find_map(|(index, node)| (node.name == name).then_some(UiNodeId(index)))
            .unwrap_or_else(|| panic!("missing node `{name}`"))
    }
}
