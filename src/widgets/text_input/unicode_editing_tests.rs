use super::*;

const CLUSTERS: &[&str] = &[
    "\u{301}\u{302}",
    "A",
    "e\u{301}",
    "👩‍🚀",
    "🇺🇸",
    "🇨🇦",
    "🇯",
    "👍🏽",
    "क्षि",
    "각",
    "Z",
];

fn sample() -> (String, Vec<usize>) {
    let mut text = String::new();
    let mut boundaries = vec![0];
    for cluster in CLUSTERS {
        text.push_str(cluster);
        boundaries.push(text.len());
    }
    (text, boundaries)
}

#[test]
fn text_input_navigation_steps_over_graphemes_from_any_scalar_boundary() {
    let (text, boundaries) = sample();
    for index in text.char_indices().map(|(i, _)| i).chain([text.len()]) {
        for (movement, expected) in [
            (
                CaretMovement::Left,
                boundaries
                    .iter()
                    .copied()
                    .filter(|&b| b < index)
                    .last()
                    .unwrap_or(0),
            ),
            (
                CaretMovement::Right,
                boundaries
                    .iter()
                    .copied()
                    .find(|&b| b > index)
                    .unwrap_or(text.len()),
            ),
        ] {
            for selecting in [false, true] {
                let mut state = TextInputState::new(&text);
                state.set_caret(index);
                state.move_caret(movement, selecting);
                assert_eq!(state.caret(), expected, "{movement:?}, index={index}");
                assert_eq!(state.selection_anchor(), selecting.then_some(index));
                assert_eq!(state.text(), text);
                assert!(!state.history().can_undo());
            }
        }
    }
    // Precise application/IME selections may start or end within a cluster.
    // Collapsing them must still put the visible caret on an outer boundary.
    let offsets: Vec<_> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect();
    for &anchor in &offsets {
        for &caret in &offsets {
            if anchor == caret {
                continue;
            }
            for (movement, expected) in [
                (
                    CaretMovement::Left,
                    boundaries
                        .iter()
                        .copied()
                        .filter(|&b| b <= anchor.min(caret))
                        .last()
                        .unwrap(),
                ),
                (
                    CaretMovement::Right,
                    boundaries
                        .iter()
                        .copied()
                        .find(|&b| b >= anchor.max(caret))
                        .unwrap(),
                ),
            ] {
                let mut state = TextInputState::new(&text);
                state.set_selection(anchor, caret);
                state.move_caret(movement, false);
                assert_eq!(state.caret(), expected, "{movement:?}: {anchor}..{caret}");
                assert_eq!(state.selection_anchor(), None);
                assert_eq!(state.text(), text);
            }
        }
    }
}

#[test]
fn text_input_forward_delete_removes_the_containing_grapheme_and_is_undoable() {
    let (text, boundaries) = sample();
    for index in text.char_indices().map(|(i, _)| i) {
        let start = boundaries
            .iter()
            .copied()
            .filter(|&b| b <= index)
            .last()
            .unwrap();
        let end = boundaries.iter().copied().find(|&b| b > index).unwrap();
        let mut expected = text.clone();
        expected.replace_range(start..end, "");
        let mut state = TextInputState::new(&text);
        state.set_caret(index);
        let outcome = state.handle_event(&UiInputEvent::Key {
            key: KeyCode::Delete,
            modifiers: KeyModifiers::NONE,
        });
        assert!(outcome.changed && outcome.transaction.is_some());
        assert_eq!(state.text(), expected, "delete at {index}");
        assert_eq!(state.caret(), start);
        state.undo_text_edit().expect("undo grapheme deletion");
        assert_eq!(state.text(), text);
        state.redo_text_edit().expect("redo grapheme deletion");
        assert_eq!(state.text(), expected);
    }
}

#[test]
fn text_input_vertical_navigation_counts_graphemes() {
    for separator in ["\n", "\r\n"] {
        let prefix = format!("e\u{301}👩‍🚀x{separator}");
        let mut state = TextInputState::new(format!("{prefix}🇺🇸👍🏽Z")).multiline(true);
        state.set_caret("e\u{301}👩‍🚀".len());
        assert_eq!(state.caret_position().column, 2);
        state.move_caret(CaretMovement::Down, false);
        assert_eq!(state.caret(), prefix.len() + "🇺🇸👍🏽".len());
        assert_eq!(state.caret_position().column, 2);
        state.move_caret(CaretMovement::Up, true);
        assert_eq!(state.caret(), "e\u{301}👩‍🚀".len());
        assert_eq!(
            state.selected_text(),
            Some(format!("x{separator}🇺🇸👍🏽").as_str())
        );
        state.move_caret(CaretMovement::LineEnd, false);
        assert_eq!(state.caret(), "e\u{301}👩‍🚀x".len());
        state.move_caret(CaretMovement::Right, false);
        assert_eq!(state.caret(), prefix.len());
    }
}

#[test]
fn text_input_pointer_placement_cannot_split_a_grapheme() {
    let (text, boundaries) = sample();
    let style = TextStyle::default();
    let metrics = TextInputLayoutMetrics::from_style(UiRect::new(0.0, 0.0, 1000.0, 40.0), &style);
    let measured = TextInputMeasuredLayout::measure(&text, &style);
    for layout in [None, measured.as_ref()] {
        let mut reached = std::collections::BTreeSet::new();
        for step in -4..4000 {
            let index = text_input_byte_index_at_point_with_layout(
                &text,
                false,
                metrics,
                UiPoint::new(step as f32 * 0.25, 0.0),
                layout,
            );
            assert!(
                boundaries.contains(&index),
                "point {} split grapheme at {index}; measured={}",
                step as f32 * 0.25,
                layout.is_some()
            );
            reached.insert(index);
        }
        assert!(
            reached.contains(&0) && reached.contains(&text.len()),
            "line edges must remain reachable; measured={}, reached={reached:?}",
            layout.is_some()
        );
        assert!(
            reached.len() > 2,
            "interior characters must remain reachable"
        );
    }
}

#[test]
fn text_input_explicit_ranges_and_ime_deletion_remain_scalar_precise() {
    let mut state = TextInputState::new("e\u{301}Z");
    state.set_selection(1, 3);
    state.delete();
    assert_eq!(state.text(), "eZ");

    let mut state = TextInputState::new("e\u{301}Z");
    state.set_caret(3);
    assert!(state.delete_surrounding_chars(1, 0));
    assert_eq!(state.text(), "eZ");

    let mut state = TextInputState::new("e\u{301}Z");
    state.set_caret(3);
    state.backspace();
    assert_eq!(
        state.text(),
        "eZ",
        "backspace can remove a combining component"
    );
}
