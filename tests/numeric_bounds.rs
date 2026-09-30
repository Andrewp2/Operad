#![cfg(feature = "widgets")]

use operad::widgets::ext::numeric_input::{
    drag_value, NumericDragSpec, NumericDragSpeed, NumericInputState, NumericKeyboardStep,
    NumericParameterSpec, NumericPrecision, NumericRange, NumericSliderState, SliderGeometry,
};
use operad::widgets::{
    drag_value_input, DragValueOptions, NumericUnitFormat, SliderClamping, SliderValueSpec,
};
use operad::{root_style, EditPhase, UiContent, UiDocument, UiPoint, UiRect};

fn cases() -> Vec<(NumericRange, NumericPrecision)> {
    vec![
        (NumericRange::new(0.1, 0.9), NumericPrecision::INTEGER),
        (NumericRange::new(-0.9, -0.1), NumericPrecision::INTEGER),
        (
            NumericRange::new(0.11, 0.89),
            NumericPrecision::decimals(2).with_step(0.25),
        ),
        (
            NumericRange::new(0.3, 0.8),
            NumericPrecision::decimals(2).with_step(0.25),
        ),
        (NumericRange::new(0.125, 0.125), NumericPrecision::INTEGER),
        (NumericRange::new(-0.001, 0.001), NumericPrecision::INTEGER),
        (
            NumericRange::new(0.00001, 0.00002),
            NumericPrecision::decimals(3),
        ),
    ]
}

fn assert_value_text(range: NumericRange, value: f64, text: &str) {
    assert!(range.contains(value), "value={value}, range={range:?}");
    assert_eq!(
        text.parse::<f64>().unwrap(),
        value,
        "stored value and text disagree"
    );
}

#[test]
fn numeric_parameter_bounds_survive_quantization_formatting_and_parsing() {
    for (range, precision) in cases() {
        let parameter = NumericParameterSpec::new("bounded", range, precision)
            .unit_prefix("$")
            .unit_suffix(" units");
        for value in [
            f64::MIN,
            range.min,
            (range.min + range.max) / 2.0,
            range.max,
            f64::MAX,
        ] {
            let normalized = parameter.normalize_value(value);
            assert!(
                range.contains(normalized),
                "{range:?}, {precision:?}, {value} -> {normalized}"
            );
            assert_eq!(parameter.normalize_value(normalized), normalized);
            if value <= range.min {
                assert_eq!(normalized, range.min);
            }
            if value >= range.max {
                assert_eq!(normalized, range.max);
            }
            let text = parameter.format_value(value);
            assert_eq!(parameter.parse_text(&text), Some(normalized));
            let validation = parameter.validate_text(&text);
            assert!(
                validation.is_valid(),
                "formatted value is invalid: {text}, {validation:?}"
            );
            assert_eq!(validation.parsed, Some(normalized));
            assert_eq!(validation.normalized, Some(normalized));
            let raw_validation = parameter.validate_text(&format!("${value} units"));
            assert_eq!(raw_validation.normalized, Some(normalized));
            assert_eq!(raw_validation.is_valid(), range.contains(value));
        }
        for logarithmic in [false, true] {
            let parameter = if logarithmic {
                parameter.clone().logarithmic(10.0)
            } else {
                parameter.clone()
            };
            assert_eq!(parameter.value_at_position(0.0), range.min);
            assert_eq!(parameter.value_at_position(1.0), range.max);
        }
    }
}

#[test]
fn numeric_editing_paths_keep_values_and_committed_text_within_bounds() {
    for (range, precision) in cases() {
        let parameter = NumericParameterSpec::new("bounded", range, precision);
        let mut input = NumericInputState::new(0.0)
            .with_precision(precision)
            .with_range(range);
        assert_value_text(range, input.value, &input.text);
        for text in ["-100", "0.45", "100"] {
            input.begin_edit();
            input.paste_text(text);
            assert!(range.contains(input.value));
            input.commit_text();
            assert_value_text(range, input.value, &input.text);
            assert_eq!(input.copy_value_text(), input.text);
            assert!(input.validation().is_valid());
            input.update_text("invalid");
            input.cancel_edit();
            assert_value_text(range, input.value, &input.text);
        }
        for step in [
            NumericKeyboardStep::Minimum,
            NumericKeyboardStep::Maximum,
            NumericKeyboardStep::LargeDecrement,
            NumericKeyboardStep::LargeIncrement,
        ] {
            let outcome = input.apply_keyboard_step(step);
            assert_value_text(range, outcome.value, &outcome.text);
        }
        for delta in [-1000.0, 0.0, 1000.0] {
            let value = drag_value(
                range.min,
                delta,
                precision,
                Some(range),
                NumericDragSpec::default(),
                NumericDragSpeed::Coarse,
            );
            assert!(range.contains(value));
            let outcome = input.apply_drag(
                range.max,
                delta,
                NumericDragSpec::default(),
                NumericDragSpeed::Normal,
            );
            assert_value_text(range, outcome.value, &outcome.text);
        }
        let geometry = SliderGeometry::horizontal(UiRect::new(10.0, 10.0, 100.0, 10.0));
        let mut slider = NumericSliderState::new(range.min, &parameter);
        let begin = slider.begin_drag(geometry, UiPoint::new(110.0, 15.0), &parameter);
        assert_value_text(range, begin.value, &begin.text);
        assert_eq!(begin.value, range.max);
        let cancel = slider.cancel_drag(&parameter);
        assert_value_text(range, cancel.value, &cancel.text);
        assert_eq!(cancel.value, range.min);
        for step in [NumericKeyboardStep::Minimum, NumericKeyboardStep::Maximum] {
            let outcome = slider.apply_keyboard_step(step, &parameter);
            assert_value_text(range, outcome.value, &outcome.text);
        }
        input.set_parameter_value(range.max, EditPhase::CommitEdit, &parameter);
        assert_value_text(range, input.value, &input.text);
        let meta = input.slider_accessibility_meta("bounded");
        assert_value_text(range, input.value, meta.value.as_deref().unwrap());
    }
}

#[test]
fn drag_value_presentation_reports_the_bounded_value() {
    for (range, precision) in cases() {
        for value in [range.min, range.max] {
            let mut document = UiDocument::new(root_style(300.0, 60.0));
            let root = document.root();
            let control = drag_value_input(
                &mut document,
                root,
                "bounded",
                value,
                DragValueOptions::default()
                    .with_range(range)
                    .with_precision(precision)
                    .with_unit(NumericUnitFormat::new().suffix(" units")),
            );
            let meta = document.node(control).accessibility().unwrap();
            let displayed = meta
                .value
                .as_deref()
                .unwrap()
                .strip_suffix(" units")
                .unwrap();
            assert_value_text(range, value, displayed);
            let text = document
                .nodes()
                .iter()
                .find_map(|node| match node.content() {
                    UiContent::Text(text) if node.name() == "bounded.value" => {
                        Some(text.text.as_str())
                    }
                    _ => None,
                })
                .unwrap();
            assert_eq!(Some(text), meta.value.as_deref());
        }
    }
}

#[test]
fn slider_edit_clamping_applies_after_stepping() {
    for clamping in [
        SliderClamping::Always,
        SliderClamping::Edits,
        SliderClamping::Never,
    ] {
        let spec = SliderValueSpec::new(0.0, 10.0).step(6.0).clamping(clamping);
        let expected = if clamping == SliderClamping::Never {
            12.0
        } else {
            10.0
        };
        assert_eq!(spec.value_at_unit(1.0), expected, "{clamping:?}");
        assert_eq!(
            spec.adjust_value(spec.value_at_unit(1.0)),
            expected,
            "an already resolved edit must survive application storage: {clamping:?}"
        );
        assert_eq!(
            spec.value_from_control_point(
                UiRect::new(0.0, 0.0, 100.0, 20.0),
                UiPoint::new(100.0, 10.0)
            ),
            expected
        );
        assert_eq!(
            spec.adjust_value(12.0),
            if clamping == SliderClamping::Always {
                10.0
            } else {
                12.0
            }
        );
        if clamping != SliderClamping::Never {
            for i in 0..=100 {
                assert!((spec.min..=spec.max).contains(&spec.value_at_unit(i as f32 / 100.0)));
            }
            let fractional = SliderValueSpec::new(0.3, 0.8).step(0.25).clamping(clamping);
            assert_eq!(fractional.value_at_unit(0.0), 0.3);
            assert_eq!(fractional.value_at_unit(1.0), 0.8);
        }
    }
}

#[test]
fn slider_formatting_preserves_finite_stored_values() {
    for value in [0.0001, 0.0009, 0.123456, 123.456, 1000.25, f32::MAX / 2.0] {
        let spec = SliderValueSpec::new(0.0, f32::MAX);
        assert_eq!(spec.format_value(value).parse::<f32>().unwrap(), value);
    }
}

#[test]
fn finite_numeric_values_do_not_overflow_during_quantization() {
    for precision in [NumericPrecision::INTEGER, NumericPrecision::decimals(12)] {
        for value in [f64::MAX, -f64::MAX, f64::MAX / 2.0, 1e100, -1e100] {
            let quantized = precision.quantize(value);
            assert!(
                quantized.is_finite(),
                "{value} -> {quantized}, {precision:?}"
            );
            assert!((quantized / value - 1.0).abs() < 1e-12);
            assert_eq!(precision.format(value).parse::<f64>().unwrap(), quantized);
        }
    }
    let spec = SliderValueSpec::new(0.0, f32::MAX).step(0.000001);
    let value = f32::MAX / 2.0;
    let normalized = spec.adjust_value(value);
    assert!(normalized.is_finite());
    assert!((normalized / value - 1.0).abs() < 0.00001);
}
