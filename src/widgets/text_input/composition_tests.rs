use super::*;

fn draft(text: &str, selection: Option<Range<usize>>) -> TextCompositionEvent {
    TextCompositionEvent::Preedit {
        text: text.into(),
        selection,
        replacement: None,
    }
}

#[test]
fn composition_replaces_a_selection_once_and_commits_one_undoable_transaction() {
    for prefix in ["", "é", "😀", "a\nb"] {
        let original = format!("{prefix}old!");
        let mut state = TextInputState::new(&original).multiline(true);
        state.set_selection(prefix.len() + 3, prefix.len());
        for (text, selection) in [("ni", 1..2), ("你", 3..3), ("你好", 3..6)] {
            assert!(
                !state
                    .apply_composition(&draft(text, Some(selection.clone())))
                    .changed
            );
            assert_eq!(state.text(), original);
            assert_eq!(state.display_text(), format!("{prefix}{text}!"));
            assert_eq!(
                state.display_selection(),
                Some(prefix.len() + selection.start..prefix.len() + selection.end)
            );
            assert!(!state.history().can_undo());
        }
        // winit clears preedit immediately before committing. The original
        // selection must still be replaced, including a reversed selection.
        state.apply_composition(&draft("", None));
        let commit = state.apply_composition(&TextCompositionEvent::Commit {
            text: "你好".into(),
            replacement: None,
        });
        assert!(commit.changed);
        assert!(commit.transaction.is_some());
        assert_eq!(state.text(), format!("{prefix}你好!"));
        assert_eq!(state.caret(), prefix.len() + "你好".len());
        state.undo_text_edit().unwrap();
        assert_eq!(state.text(), original);
        assert!(state.undo_text_edit().is_none());
        state.redo_text_edit().unwrap();
        assert_eq!(state.text(), format!("{prefix}你好!"));
    }
}

#[test]
fn cancel_and_candidate_keys_preserve_committed_text_selection_and_history() {
    let mut state = TextInputState::new("a😀z");
    state.set_selection(1, 5);
    state.apply_composition(&draft("候補", Some(3..6)));
    for key in [
        KeyCode::Enter,
        KeyCode::Backspace,
        KeyCode::ArrowLeft,
        KeyCode::Tab,
    ] {
        let outcome = state.handle_event(&UiInputEvent::Key {
            key,
            modifiers: KeyModifiers::NONE,
        });
        assert!(!outcome.changed && !outcome.committed && !outcome.canceled);
        assert_eq!(state.text(), "a😀z");
        assert_eq!(state.selected_range(), Some(1..5));
        assert_eq!(state.composing(), Some("候補"));
    }
    let outcome = state.handle_event(&UiInputEvent::Key {
        key: KeyCode::Escape,
        modifiers: KeyModifiers::NONE,
    });
    assert!(
        !outcome.canceled,
        "Escape cancels the draft, not the entire field edit"
    );
    assert_eq!(state.display_text(), "a😀z");
    assert_eq!(state.selected_range(), Some(1..5));
    assert!(state.undo_text_edit().is_none());
    state.apply_composition(&draft("new", Some(3..3)));
    state.handle_event_with_policy(
        &UiInputEvent::Composition {
            target: None,
            event: TextCompositionEvent::Cancel,
        },
        TextInputInteractionPolicy::disabled(),
    );
    assert!(
        state.composition().is_none(),
        "disabling must not block cleanup"
    );
}

#[test]
fn tab_navigates_text_fields_but_preserves_composition_candidate_keys() {
    let mut doc = UiDocument::new(LayoutStyle::column().with_size(300.0, 120.0));
    let mut state = TextInputState::new("");
    let root = doc.root();
    let field = text_input(&mut doc, root, "field", &state, TextInputOptions::default());
    let next = doc.add_child(
        root,
        UiNode::container("next", LayoutStyle::size(80.0, 30.0)).with_input(InputBehavior::BUTTON),
    );
    doc.compute_layout(UiSize::new(300.0, 120.0), &mut ApproxTextMeasurer)
        .unwrap();
    doc.set_focus_state(UiFocusState {
        focused: Some(field),
        ..Default::default()
    });
    let context = TextInputPlatformContext::for_node(
        field,
        state.caret_rect(TextInputLayoutMetrics::from_style(
            doc.node(field).layout().rect,
            &TextStyle::default(),
        )),
    );
    let tab = UiInputEvent::Key {
        key: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
    };

    state.apply_composition(&draft("候補", None));
    let candidate = handle_text_input_event(
        &mut doc,
        field,
        &mut state,
        tab.clone(),
        Some(context.clone()),
    );
    assert_eq!(candidate.input.focused, Some(field));
    assert_eq!(state.composing(), Some("候補"));
    assert!(candidate.platform_requests.is_empty());

    state.apply_composition(&TextCompositionEvent::Commit {
        text: "候補".into(),
        replacement: None,
    });
    let leave = handle_text_input_event(
        &mut doc,
        field,
        &mut state,
        tab.clone(),
        Some(context.clone()),
    );
    assert_eq!(leave.input.focused, Some(next));
    assert!(leave.edit.is_none());
    assert_eq!(state.text(), "候補");
    assert!(matches!(
        leave.platform_requests.as_slice(),
        [
            PlatformRequest::TextIme(TextImeRequest::HideKeyboard { .. }),
            PlatformRequest::TextIme(TextImeRequest::Deactivate { .. }),
        ]
    ));

    let enter = handle_text_input_event(&mut doc, field, &mut state, tab, Some(context));
    assert_eq!(enter.input.focused, Some(field));
    assert!(enter.edit.is_none());
    assert!(matches!(
        enter.platform_requests.as_slice(),
        [
            PlatformRequest::TextIme(TextImeRequest::Activate(_)),
            PlatformRequest::TextIme(TextImeRequest::ShowKeyboard { .. }),
        ]
    ));
    state.apply_composition(&draft("new draft", None));
    handle_text_input_event(
        &mut doc,
        field,
        &mut state,
        UiInputEvent::Focus(crate::FocusDirection::Next),
        None,
    );
    assert_eq!(doc.focus_state().focused, Some(next));
    assert!(state.composition().is_none());
}

#[test]
fn explicit_replacement_and_filtered_unicode_offsets_stay_within_text_boundaries() {
    let mut state = TextInputState::new("a😀z");
    state.apply_composition(&TextCompositionEvent::Preedit {
        text: "é\r\n😀".into(),
        selection: Some(4..8),
        replacement: Some(1..5),
    });
    assert_eq!(state.display_text(), "aé 😀z");
    assert_eq!(state.display_selection(), Some(4..8));
    state.apply_composition(&TextCompositionEvent::Commit {
        text: "新".into(),
        replacement: None,
    });
    assert_eq!(state.text(), "a新z");
    state.undo_text_edit().unwrap();
    assert_eq!(state.text(), "a😀z");
    for offset in 0..10 {
        state.apply_composition(&TextCompositionEvent::Preedit {
            text: "😀é".into(),
            selection: Some(offset..usize::MAX),
            replacement: Some(offset..usize::MAX),
        });
        let display = state.display_text();
        let selection = state.display_selection().unwrap();
        assert!(display.is_char_boundary(selection.start));
        assert!(display.is_char_boundary(selection.end));
    }
}

#[test]
fn rendering_uses_draft_glyphs_selection_underlines_and_the_composition_caret() {
    let mut state = TextInputState::new("before old after").multiline(true);
    state.set_selection(7, 10);
    state.apply_composition(&draft("你\n好", Some(4..7)));
    let style = TextStyle::default();
    let metrics = TextInputLayoutMetrics::from_style(UiRect::new(6.0, 6.0, 300.0, 100.0), &style);
    let plan = state.render_plan(metrics, style.clone(), TextInputPaintOptions::default());
    assert_eq!(plan.text.text, "before 你\n好 after");
    assert_eq!(plan.composition_paint.len(), 2);
    assert_eq!(plan.selection_rects.len(), 1);
    let caret = plan.caret.unwrap();
    assert!(
        caret.rect.y > metrics.text_rect.y,
        "candidate cursor follows the second line"
    );
    assert_eq!(
        state.text_input_snapshot(caret.rect).composition,
        Some(7..14)
    );
    state.apply_composition(&draft("候", None));
    let plan = state.render_plan(metrics, style, TextInputPaintOptions::default());
    assert!(
        plan.caret.is_none(),
        "the input method can hide the preedit caret"
    );
    assert_eq!(plan.composition_paint.len(), 1);
}

#[test]
fn password_composition_is_masked_but_platform_offsets_refer_to_real_text() {
    let mut state = TextInputState::new("a😀z");
    state.set_selection(1, 5);
    state.apply_composition(&draft("秘密", Some(3..6)));
    let mut doc = UiDocument::new(LayoutStyle::size(300.0, 100.0));
    let root = doc.root();
    let node = password_input(
        &mut doc,
        root,
        "password",
        &state,
        TextInputOptions {
            focused: true,
            ..Default::default()
        },
    );
    let snapshot = doc.node(node).text_input().unwrap();
    assert!(snapshot.sensitive);
    assert_eq!(snapshot.text, "a秘密z");
    assert_eq!(snapshot.selection, 4..7);
    let scene = doc.node(doc.node(node).children()[0]);
    let UiContent::Scene(scene) = scene.content() else {
        panic!("missing editor scene")
    };
    assert!(scene
        .iter()
        .any(|primitive| matches!(primitive, ScenePrimitive::Text(text) if text.text == "****")));
}

#[test]
fn pointer_selection_cancels_drafts_using_displayed_geometry() {
    // Reconversion may replace only the combining mark of a grapheme. A
    // discarded draft must not turn a pointer hit into a partial-grapheme stop.
    let mut partial = TextInputState::new("Ae\u{301}Z");
    partial.set_selection(2, 4);
    partial.apply_composition(&draft("ab", Some(2..2)));
    let display = partial.display_text();
    let metrics = TextInputLayoutMetrics::from_style(
        UiRect::new(0.0, 0.0, 300.0, 40.0),
        &TextStyle::default(),
    );
    for (displayed, committed) in [(2, 1), (3, 1), (4, 4), (5, 5)] {
        let caret = text_input_caret_rect(&display, displayed, metrics).rect;
        let mut state = partial.clone();
        state.move_caret_to_point(metrics, UiPoint::new(caret.x + 0.1, caret.y + 1.0), false);
        assert_eq!(state.caret(), committed);
        assert_eq!(state.text(), "Ae\u{301}Z");
        assert!(state.composition().is_none() && !state.history().can_undo());
    }

    let original = "pré OLD tail😀Z";
    let start = original.find("OLD").unwrap();
    let viewport = UiSize::new(900.0, 400.0);
    let mut failures = Vec::new();
    for (kind, build) in [singleline_text_input, multiline_text_input, password_input]
        .into_iter()
        .enumerate()
    {
        for scale in [0.75, 1.5] {
            for preedit in ["é", "WWWWiiii候補", "é\n二"] {
                for replacing in [false, true] {
                    let end = start + if replacing { 3 } else { 0 };
                    let mut state = TextInputState::new(original).multiline(kind == 1);
                    if replacing {
                        state.set_selection(start, end);
                    } else {
                        state.set_caret(start);
                    }
                    state.apply_composition(&draft(preedit, Some(preedit.len()..preedit.len())));
                    let composition = state.composition().unwrap();
                    let display = state.display_text();
                    let draft_end = start + composition.text.len();
                    let mut targets = vec![
                        (1, 1),
                        (start, start),
                        (draft_end, end),
                        (display.len() - 1, original.len() - 1),
                    ];
                    let inside = start + composition.text.chars().next().unwrap().len_utf8();
                    if inside < draft_end {
                        targets.push((inside, start));
                    }
                    let options = TextInputOptions {
                        focused: true,
                        layout: crate::layout::absolute(8.0, 8.0, 420.0, 180.0),
                        ..Default::default()
                    };
                    let mut doc =
                        UiDocument::new(LayoutStyle::size(viewport.width, viewport.height));
                    doc.set_ui_scale(scale);
                    let root = doc.root();
                    let field = build(&mut doc, root, "field", &state, options.clone());
                    doc.compute_layout(viewport, &mut ApproxTextMeasurer)
                        .unwrap();
                    let content = doc.node(field).text_input_content().unwrap().clone();
                    let bounds = doc.text_input_content_bounds(field);
                    let metrics = TextInputLayoutMetrics::from_style(
                        text_input_content_rect(bounds, &content.text_style),
                        &content.text_style,
                    );
                    let painted_text = content.mask.map_or_else(
                        || display.clone(),
                        |mask| mask.to_string().repeat(display.chars().count()),
                    );
                    let measured =
                        TextInputMeasuredLayout::measure(&painted_text, &content.text_style);
                    for (displayed_index, expected) in targets {
                        for selecting in [false, true] {
                            let painted_index = content.mask.map_or(displayed_index, |mask| {
                                char_count_before_byte(&display, displayed_index) * mask.len_utf8()
                            });
                            let caret = text_input_caret_rect_with_layout(
                                &painted_text,
                                painted_index,
                                metrics,
                                measured.as_ref(),
                            )
                            .rect;
                            let origin = doc.node(content.node).layout().rect;
                            let point = doc.node_content_transform(content.node).transform_point(
                                UiPoint::new(
                                    origin.x + caret.x + 0.1,
                                    origin.y + caret.y + caret.height * 0.5,
                                ),
                            );
                            assert_eq!(
                                doc.hit_test(point),
                                Some(field),
                                "fixture point is inside the editor"
                            );
                            let event = if selecting {
                                UiInputEvent::PointerMove(point)
                            } else {
                                UiInputEvent::PointerDown(point)
                            };
                            let mut action = WidgetTextEdit::new(event.clone());
                            action.selecting = selecting;
                            action.geometry = doc.text_input_pointer_geometry(field, point);
                            let mut routed = state.clone();
                            let outcome = routed.apply_widget_text_edit(&action, &options);
                            assert!(!outcome.changed && outcome.transaction.is_none());

                            let mut direct = state.clone();
                            doc.set_focus_state(UiFocusState {
                                focused: Some(field),
                                pressed: selecting.then_some(field),
                                ..Default::default()
                            });
                            doc.pointer_position = None;
                            let result = handle_text_input_event_with_options(
                                &mut doc,
                                field,
                                &mut direct,
                                &options,
                                event,
                                Some(TextInputPlatformContext::new(
                                    TextInputId::new("field"),
                                    LogicalRect::new(0.0, 0.0, 1.0, 1.0),
                                )),
                            );
                            let platform_cleared = result.platform_requests.iter().any(|request| matches!(request, PlatformRequest::TextIme(TextImeRequest::Update(session)) if session.composition.is_none() && session.surrounding_text == original && session.selection == TextRange::new(if selecting { start } else { expected }, expected)));

                            let mut plain = state.clone();
                            let approximate_caret =
                                text_input_caret_rect(&display, displayed_index, metrics).rect;
                            let approximate_point = UiPoint::new(
                                approximate_caret.x + 0.1,
                                approximate_caret.y + approximate_caret.height * 0.5,
                            );
                            assert_eq!(
                                plain.byte_index_at_point(metrics, approximate_point),
                                expected
                            );
                            assert_eq!(
                                plain.position_at_point(metrics, approximate_point),
                                text_position_at(original, expected)
                            );
                            plain.move_caret_to_point(metrics, approximate_point, selecting);
                            for (route, result) in
                                [("action", &routed), ("direct", &direct), ("model", &plain)]
                            {
                                if result.caret() != expected
                                    || result.composition().is_some()
                                    || result.selection_anchor() != selecting.then_some(start)
                                    || (route == "direct" && !platform_cleared)
                                {
                                    failures.push(format!("kind={kind}, scale={scale}, draft={preedit:?}, replacing={replacing}, index={displayed_index}, selecting={selecting}, route={route}: caret={} expected={expected}, composing={}, platform_cleared={platform_cleared}", result.caret(), result.composition().is_some()));
                                }
                                assert_eq!(result.text(), original);
                                assert!(!result.history().can_undo());
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches; first cases: {:#?}",
        failures.len(),
        &failures[..failures.len().min(12)]
    );
}
