use super::{
    handle_text_input_event_with_options, singleline_text_input, TextInputOptions,
    TextInputPlatformContext, TextInputState,
};
use crate::host::collect_document_widget_actions;
use crate::input::{PointerButton, PointerEventKind, RawInputEvent, RawPointerEvent};
use crate::platform::{
    LogicalRect, PlatformRequest, PlatformRequestIdAllocator, TextImeRequest, TextInputId,
};
use crate::renderer::RenderTarget;
use crate::runtime::session::RuntimeSession;
use crate::{
    ApproxTextMeasurer, LayoutStyle, PaintKind, UiDocument, UiDocumentScale, UiInputEvent, UiNode,
    UiNodeId, UiPoint, UiSize, WidgetActionKind,
};
#[test]
fn text_input_geometry_roundtrips_caret_through_runtime() {
    let mut regressions = 0;
    for (masked, build) in [singleline_text_input, super::password_input]
        .into_iter()
        .enumerate()
    {
        for scale in [0.75, 1.0, 1.5, 2.0] {
            for (case, (padding, height)) in [
                (0.0, 30.0),
                (6.0, 30.0),
                (0.0, 100.0),
                (6.0, 100.0),
                (18.0, 100.0),
                (6.0, 100.0),
                (6.0, 100.0),
            ]
            .into_iter()
            .enumerate()
            {
                let viewport = UiSize::new(800.0, 480.0);
                let mut state = TextInputState::new("iiiiiiiiiiii");
                state.set_caret(3);
                let mut layout = LayoutStyle::size(240.0, height).with_padding(padding);
                if case == 5 {
                    use taffy::prelude::{length, percent, Rect};
                    layout.style.padding = Rect {
                        left: percent(0.07_f32),
                        right: length(4.0_f32),
                        top: length(3.0_f32),
                        bottom: percent(0.02_f32),
                    };
                    layout.style.border = Rect::length(2.0_f32);
                }
                let options = TextInputOptions {
                    layout,
                    focused: true,
                    text_style: crate::TextStyle {
                        font_size: 9.0 + case as f32 * 3.0,
                        line_height: 12.0 + case as f32 * 3.0,
                        family: match case % 3 {
                            0 => crate::FontFamily::SansSerif,
                            1 => crate::FontFamily::Serif,
                            _ => crate::FontFamily::Monospace,
                        },
                        weight: crate::FontWeight::BOLD,
                        ..Default::default()
                    },
                    ..TextInputOptions::default().with_edit_action("field")
                };
                // Focus handlers can change application styling before the queued
                // pointer edit is applied. Hit testing still belongs to this frame.
                let event_options = TextInputOptions::default();
                let mut doc = UiDocument::new(
                    LayoutStyle::column()
                        .with_size(viewport.width, viewport.height)
                        .with_padding(10.0),
                );
                let root = doc.root();
                let field = build(&mut doc, root, "field", &state, options.clone());
                if case == 6 {
                    use crate::{AnimatedValues, AnimationMachine, AnimationState};
                    let content = doc.node(field).text_input_content().unwrap().node;
                    doc.node_mut(content).animation = Some(
                        AnimationMachine::new(
                            vec![AnimationState::new(
                                "visible",
                                AnimatedValues::new(1.0, UiPoint::new(2.0, 3.0), 1.25),
                            )],
                            Vec::new(),
                            "visible",
                        )
                        .unwrap(),
                    );
                }
                let mut session = RuntimeSession::new();
                session
                    .prepare_document(
                        &mut doc,
                        viewport,
                        UiDocumentScale::new(scale, 2.0),
                        None,
                        &mut ApproxTextMeasurer,
                    )
                    .unwrap();
                let input = session
                    .process_input(
                        &mut doc,
                        viewport,
                        Vec::new(),
                        Vec::new(),
                        &mut ApproxTextMeasurer,
                    )
                    .unwrap();
                session
                    .finish_frame(
                        &mut doc,
                        viewport,
                        RenderTarget::window("probe", viewport),
                        input,
                        &mut ApproxTextMeasurer,
                        &mut PlatformRequestIdAllocator::default(),
                    )
                    .unwrap();
                let snapshot = doc.node(field).text_input().unwrap();
                let paint = doc.paint_list();
                let caret = paint
                    .items
                    .iter()
                    .find_map(|item| match &item.kind {
                        PaintKind::RichRect(rect)
                            if rect.rect.width == snapshot.cursor_rect.width
                                && rect.rect.height == snapshot.cursor_rect.height =>
                        {
                            crate::effective_geometry::EffectiveTransform::from(item.transform)
                                .transform_rect_bounds(rect.rect)
                                .intersection(item.clip_rect)
                        }
                        _ => None,
                    })
                    .unwrap();
                let ime = session.interaction().text_ime.as_ref().unwrap().cursor_rect;
                let ime_matches = (caret.x - ime.origin.x).abs() < 0.01
                    && (caret.y - ime.origin.y).abs() < 0.01
                    && (caret.width - ime.size.width).abs() < 0.01
                    && (caret.height - ime.size.height).abs() < 0.01;
                let point =
                    UiPoint::new(caret.x + caret.width * 0.25, caret.y + caret.height * 0.5);
                let input = session
                    .process_input(
                        &mut doc,
                        viewport,
                        vec![RawInputEvent::Pointer(RawPointerEvent::new(
                            PointerEventKind::Down(PointerButton::Primary),
                            point,
                            1,
                        ))],
                        Vec::new(),
                        &mut ApproxTextMeasurer,
                    )
                    .unwrap();
                let frame = session
                    .finish_frame(
                        &mut doc,
                        viewport,
                        RenderTarget::window("probe", viewport),
                        input,
                        &mut ApproxTextMeasurer,
                        &mut PlatformRequestIdAllocator::default(),
                    )
                    .unwrap();
                let mut delivered = 0;
                for action in collect_document_widget_actions(&frame) {
                    if let WidgetActionKind::TextEdit(edit) = action.kind {
                        state.apply_widget_text_edit(&edit, &event_options);
                        delivered += 1;
                    }
                }
                let routed_caret = state.caret();
                let pointer_matches = delivered == 1 && routed_caret == 3;
                state.set_caret(0);
                let direct = handle_text_input_event_with_options(
                    &mut doc,
                    field,
                    &mut state,
                    &event_options,
                    UiInputEvent::PointerDown(point),
                    Some(TextInputPlatformContext::new(
                        TextInputId::new("direct"),
                        LogicalRect::new(0.0, 0.0, 1.0, 1.0),
                    )),
                );
                let direct_cursor =
                    direct
                        .platform_requests
                        .iter()
                        .find_map(|request| match request {
                            PlatformRequest::TextIme(TextImeRequest::Update(session))
                                if session.sensitive == (masked == 1) =>
                            {
                                Some(session.cursor_rect)
                            }
                            _ => None,
                        });
                let direct_matches = state.caret() == 3
                    && direct_cursor.is_some_and(|cursor| {
                        (cursor.origin.x - caret.x).abs() < 0.01
                            && (cursor.origin.y - caret.y).abs() < 0.01
                            && (cursor.size.width - caret.width).abs() < 0.01
                            && (cursor.size.height - caret.height).abs() < 0.01
                    });
                let painted_font = paint
                    .items
                    .iter()
                    .find_map(|item| match &item.kind {
                        PaintKind::SceneText(text) => {
                            Some(text.style.font_size * item.transform.scale)
                        }
                        _ => None,
                    })
                    .unwrap();
                let font_matches = (painted_font
                    - options.text_style.font_size * scale * if case == 6 { 1.25 } else { 1.0 })
                .abs()
                    < 0.01;
                println!("masked={masked}, case={case}, scale={scale}, padding={padding}, height={height}: paint={caret:?}, IME={ime:?}, ime_matches={ime_matches}, routed_caret={routed_caret}, direct_caret={}, font_matches={font_matches}, direct_matches={direct_matches}, direct_cursor={direct_cursor:?}", state.caret());
                regressions += usize::from(
                    !ime_matches || !pointer_matches || !direct_matches || !font_matches,
                );
            }
        }
    }
    assert_eq!(
        regressions, 0,
        "text-input geometry disagrees across boundaries"
    );
}

#[test]
fn text_input_geometry_survives_cached_section_relocation() {
    let viewport = UiSize::new(800.0, 480.0);
    let mut session = RuntimeSession::new();
    let mut previous_content = None;
    for prefix in [0, 3, 1] {
        session.invalidate_view();
        let mut doc = session
            .build_document(
                viewport,
                UiDocumentScale::new(1.5, 1.0),
                None,
                &mut ApproxTextMeasurer,
                |_, views| {
                    let mut doc = UiDocument::new(LayoutStyle::column().with_size(800.0, 480.0));
                    let root = doc.root();
                    for index in 0..prefix {
                        doc.add_child(
                            root,
                            UiNode::container(
                                format!("prefix-{index}"),
                                LayoutStyle::size(100.0, 8.0),
                            ),
                        );
                    }
                    views.section(&mut doc, root, "editor", &(), |_, _| {
                        let mut fragment = UiDocument::new(
                            LayoutStyle::column()
                                .with_size(300.0, 150.0)
                                .with_padding(8.0),
                        );
                        let root = fragment.root();
                        let mut state = TextInputState::new(
                            "A café melody across many instruments and many tracks",
                        );
                        state.set_caret(state.text().len());
                        singleline_text_input(
                            &mut fragment,
                            root,
                            "field",
                            &state,
                            TextInputOptions {
                                layout: LayoutStyle::size(220.0, 80.0).with_padding(13.0),
                                focused: true,
                                ..Default::default()
                            },
                        );
                        fragment
                    });
                    doc
                },
            )
            .unwrap();
        let field = UiNodeId(
            doc.nodes()
                .iter()
                .position(|node| node.name() == "field")
                .unwrap(),
        );
        let source = doc.node(field).text_input_content().unwrap().node;
        assert!(doc.scroll_state(field).unwrap().offset().x > 0.0);
        if let Some(previous) = previous_content {
            assert_ne!(source, previous);
            assert_eq!(session.view_build_stats().reused, 1);
        }
        previous_content = Some(source);
        let input = session
            .process_input(
                &mut doc,
                viewport,
                Vec::new(),
                Vec::new(),
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        session
            .finish_frame(
                &mut doc,
                viewport,
                RenderTarget::window("probe", viewport),
                input,
                &mut ApproxTextMeasurer,
                &mut PlatformRequestIdAllocator::default(),
            )
            .unwrap();
        let paint = doc.paint_list();
        let snapshot = doc.node(field).text_input().unwrap();
        let caret = paint
            .items
            .iter()
            .find_map(|item| match &item.kind {
                PaintKind::RichRect(rect)
                    if item.node == source
                        && rect.rect.width == snapshot.cursor_rect.width
                        && rect.rect.height == snapshot.cursor_rect.height =>
                {
                    Some(
                        crate::effective_geometry::EffectiveTransform::from(item.transform)
                            .transform_rect_bounds(rect.rect),
                    )
                }
                _ => None,
            })
            .unwrap();
        let ime = session.interaction().text_ime.as_ref().unwrap().cursor_rect;
        assert!(
            (caret.x - ime.origin.x).abs() < 0.01 && (caret.y - ime.origin.y).abs() < 0.01,
            "paint={caret:?}; IME={ime:?}"
        );
        session.retain_document(doc);
    }
}

#[test]
fn direct_pointer_release_keeps_the_caret_unless_the_pointer_moved() {
    for moved in [false, true] {
        let mut document = UiDocument::new(LayoutStyle::size(400.0, 80.0));
        let field = document.add_child(
            document.root(),
            UiNode::container("custom editor", LayoutStyle::size(400.0, 80.0))
                .with_input(crate::InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(400.0, 80.0), &mut ApproxTextMeasurer)
            .unwrap();
        let mut state = TextInputState::new("iiiiiiii");
        let mut options = TextInputOptions::default();
        options.text_style.font_size = 9.0;
        options.text_style.line_height = 12.0;
        let point = UiPoint::new(16.0, 12.0);
        handle_text_input_event_with_options(
            &mut document,
            field,
            &mut state,
            &options,
            UiInputEvent::PointerDown(point),
            None,
        );
        let pressed_caret = state.caret();
        assert!(pressed_caret > 0 && pressed_caret < state.text().len());
        options.text_style.font_size = 27.0;
        options.text_style.line_height = 32.0;
        let release = if moved {
            UiPoint::new(300.0, point.y)
        } else {
            point
        };
        handle_text_input_event_with_options(
            &mut document,
            field,
            &mut state,
            &options,
            UiInputEvent::PointerUp(release),
            None,
        );
        assert_eq!(
            state.caret(),
            if moved {
                state.text().len()
            } else {
                pressed_caret
            }
        );
        assert_eq!(state.selected_range().is_some(), moved);
        assert!(document.focus_state().pressed.is_none());
    }
}

#[test]
fn default_text_input_keeps_its_line_and_caret_visible() {
    let mut regressions = Vec::new();
    for scale in [0.75, 1.0, 1.5, 2.0] {
        for text in ["gypq", ""] {
            let mut document = UiDocument::new(LayoutStyle::column().with_padding(12.0));
            document.set_ui_scale(scale);
            let root = document.root();
            let field = singleline_text_input(
                &mut document,
                root,
                "field",
                &TextInputState::new(text),
                TextInputOptions {
                    focused: true,
                    placeholder: "Placeholder".into(),
                    ..Default::default()
                },
            );
            document
                .compute_layout(UiSize::new(600.0, 300.0), &mut ApproxTextMeasurer)
                .unwrap();
            let content = document.node(field).text_input_content().unwrap().node;
            let mut text_seen = false;
            let mut caret_seen = false;
            for item in document.paint_list().items {
                if item.node != content {
                    continue;
                }
                if !matches!(item.kind, PaintKind::SceneText(_) | PaintKind::RichRect(_)) {
                    continue;
                }
                let rect = crate::effective_geometry::EffectiveTransform::from(item.transform)
                    .transform_rect_bounds(item.rect);
                let visible = rect.intersection(item.clip_rect);
                if visible.is_none_or(|visible| (visible.height - rect.height).abs() > 0.01) {
                    regressions.push(format!(
                        "scale={scale}, text={text:?}, rect={rect:?}, visible={visible:?}"
                    ));
                }
                text_seen |= matches!(item.kind, PaintKind::SceneText(_));
                caret_seen |= matches!(item.kind, PaintKind::RichRect(_));
            }
            assert!(
                text_seen && caret_seen,
                "inspect both the text line and the caret"
            );
        }
    }
    assert!(
        regressions.is_empty(),
        "default field clips its content: {regressions:#?}"
    );
}

#[test]
fn constrained_text_fields_keep_bounds_and_reveal_the_displayed_caret() {
    use crate::widgets::{multiline_text_input, password_input, search_input, selectable_text};
    use crate::{InputBehavior, UiRect};
    let builders = [
        singleline_text_input,
        multiline_text_input,
        password_input,
        search_input,
        selectable_text,
    ];
    for (kind, build) in builders.into_iter().enumerate() {
        for scale in [0.75, 1.0, 1.5] {
            for width in [40.0, 100.0] {
                let text = if kind == 1 {
                    "A long café melody with many notes\nSecond line with notes\nThe final chord"
                } else {
                    "A long café melody with many notes and a final chord"
                };
                for caret in [0, 12, text.len()] {
                    let mut state = TextInputState::new(text).multiline(kind == 1);
                    state.set_caret(caret);
                    let viewport = UiSize::new(500.0, 250.0);
                    let mut doc =
                        UiDocument::new(LayoutStyle::size(viewport.width, viewport.height));
                    doc.set_ui_scale(scale);
                    let root = doc.root();
                    let options = TextInputOptions {
                        layout: crate::layout::absolute(10.0, 10.0, width, 40.0),
                        focused: true,
                        ..TextInputOptions::default().with_edit_action("edit")
                    };
                    let field = build(&mut doc, root, "field", &state, options.clone());
                    let next = doc.add_child(
                        root,
                        UiNode::container(
                            "next",
                            crate::layout::absolute(10.0 + width + 4.0, 10.0, 40.0, 40.0),
                        )
                        .with_input(InputBehavior::BUTTON),
                    );
                    doc.compute_layout(viewport, &mut ApproxTextMeasurer)
                        .unwrap();
                    let rect = doc.node(field).layout().rect;
                    assert!(
                        (rect.width - width * scale).abs() <= 1.0,
                        "kind={kind}, scale={scale}, width={width}, caret={caret}: {rect:?}"
                    );
                    let next_rect = doc.node(next).layout().rect;
                    assert!(rect.right() <= next_rect.x, "field covers its neighbour");
                    assert_eq!(
                        doc.hit_test(UiPoint::new(next_rect.x + 2.0, next_rect.y + 2.0)),
                        Some(next)
                    );
                    let content = doc.node(field).text_input_content().unwrap().node;
                    let caret_paint = doc
                        .paint_list()
                        .items
                        .into_iter()
                        .find_map(|item| {
                            if item.node != content {
                                return None;
                            }
                            let PaintKind::RichRect(caret) = item.kind else {
                                return None;
                            };
                            let rect =
                                crate::effective_geometry::EffectiveTransform::from(item.transform)
                                    .transform_rect_bounds(caret.rect);
                            Some((rect, item.clip_rect))
                        })
                        .expect("focused field paints its caret");
                    let (painted, clip) = caret_paint;
                    let visible = painted
                        .intersection(clip)
                        .unwrap_or(UiRect::new(0.0, 0.0, 0.0, 0.0));
                    assert!(
                        (visible.width - painted.width).abs() < 0.01
                            && (visible.height - painted.height).abs() < 0.01,
                        "kind={kind}, caret={caret}: caret={painted:?}, clip={clip:?}"
                    );
                    let point = UiPoint::new(
                        painted.x + painted.width * 0.25,
                        painted.y + painted.height * 0.5,
                    );
                    let geometry = doc.text_input_pointer_geometry(field, point).unwrap();
                    let mut clicked = state.clone();
                    clicked.apply_widget_text_edit(
                        &crate::actions::WidgetTextEdit {
                            event: UiInputEvent::PointerDown(point),
                            phase: crate::actions::WidgetValueEditPhase::Begin,
                            position: Some(point),
                            local_position: None,
                            target_rect: Some(rect),
                            geometry: Some(geometry),
                            selecting: false,
                        },
                        &options,
                    );
                    assert_eq!(
                        clicked.caret(),
                        caret,
                        "kind={kind}, scale={scale}: pointer agrees with caret"
                    );
                    assert_eq!(state.text(), text);
                }
            }
        }
    }
}

#[test]
fn masked_pointer_selection_maps_to_original_grapheme_boundaries() {
    use unicode_segmentation::UnicodeSegmentation;
    let text = "Wié e\u{301}👩‍🚀🇨🇦Z";
    let boundaries: Vec<_> = text
        .grapheme_indices(true)
        .map(|(byte, _)| byte)
        .chain([text.len()])
        .collect();
    let viewport = UiSize::new(500.0, 180.0);
    for scale in [0.75, 1.5] {
        for width in [64.0, 220.0] {
            for read_only in [false, true] {
                for caret in text
                    .char_indices()
                    .map(|(byte, _)| byte)
                    .chain([text.len()])
                {
                    let mut state = TextInputState::new(text);
                    state.set_caret(caret);
                    let options = TextInputOptions {
                        layout: crate::layout::absolute(10.0, 10.0, width, 40.0),
                        focused: true,
                        read_only,
                        ..Default::default()
                    };
                    let mut doc =
                        UiDocument::new(LayoutStyle::size(viewport.width, viewport.height));
                    doc.set_ui_scale(scale);
                    let root = doc.root();
                    let field =
                        super::password_input(&mut doc, root, "password", &state, options.clone());
                    doc.compute_layout(viewport, &mut ApproxTextMeasurer)
                        .unwrap();
                    let content = doc.node(field).text_input_content().unwrap().node;
                    let painted = doc
                        .paint_list()
                        .items
                        .into_iter()
                        .find_map(|item| {
                            if item.node != content {
                                return None;
                            }
                            let PaintKind::RichRect(caret) = item.kind else {
                                return None;
                            };
                            Some(
                                crate::effective_geometry::EffectiveTransform::from(item.transform)
                                    .transform_rect_bounds(caret.rect),
                            )
                        })
                        .unwrap();
                    let point = UiPoint::new(
                        painted.x + painted.width * 0.1,
                        painted.y + painted.height * 0.5,
                    );
                    let geometry = doc.text_input_pointer_geometry(field, point).unwrap();
                    for selecting in [false, true] {
                        let anchor = if caret == text.len() { 0 } else { text.len() };
                        let mut routed = state.clone();
                        routed.set_caret(anchor);
                        let event = if selecting {
                            UiInputEvent::PointerMove(point)
                        } else {
                            UiInputEvent::PointerDown(point)
                        };
                        let mut edit = crate::WidgetTextEdit::new(event.clone());
                        edit.geometry = Some(geometry.clone());
                        edit.selecting = selecting;
                        routed.apply_widget_text_edit(&edit, &options);

                        let mut direct = state.clone();
                        direct.set_caret(anchor);
                        doc.set_focus_state(crate::UiFocusState {
                            focused: Some(field),
                            pressed: selecting.then_some(field),
                            ..Default::default()
                        });
                        doc.pointer_position = None;
                        handle_text_input_event_with_options(
                            &mut doc,
                            field,
                            &mut direct,
                            &options,
                            event,
                            None,
                        );
                        assert_eq!(routed.caret(), direct.caret(), "routes disagree: scale={scale}, width={width}, read_only={read_only}, selecting={selecting}, caret={caret}");
                        for result in [&routed, &direct] {
                            assert!(
                                boundaries.contains(&result.caret()),
                                "pointer split a grapheme at {}",
                                result.caret()
                            );
                            if boundaries.contains(&caret) {
                                assert_eq!(
                                    result.caret(),
                                    caret,
                                    "pointer must preserve a painted caret at a grapheme boundary"
                                );
                            }
                            assert_eq!(result.selection_anchor(), selecting.then_some(anchor));
                            assert_eq!(result.text(), text);
                            assert!(!result.history().can_undo(), "selection does not edit text");
                        }
                    }
                }
            }
        }
    }
}
