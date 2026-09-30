use super::*;
use crate::input::{PointerButton, PointerEventKind, PointerId, RawPointerEvent};
use crate::{
    ApproxTextMeasurer, InputBehavior, LayoutStyle, ScrollAxes, UiInputEvent, UiNode, UiRect,
    WidgetAction,
};

const VIEWPORT: UiSize = UiSize::new(400.0, 300.0);

#[test]
fn frame_hook_changes_rebuild_views_and_unchanged_hooks_preserve_other_invalidation() {
    use crate::runtime::{RuntimeHookResult, RuntimeHooks, RuntimeMetrics};
    #[derive(Default)]
    struct State {
        value: usize,
        before: bool,
        requests: bool,
        services: bool,
        responses: bool,
    }
    let mut state = State::default();
    let mut hooks = RuntimeHooks::new()
        .with_before_render(|state: &mut State, _| {
            if state.before {
                state.value += 1;
            }
            RuntimeHookResult {
                value: (),
                view_changed: state.before,
            }
        })
        .with_platform_requests(|state: &mut State, _| {
            if state.requests {
                state.value += 1;
            }
            RuntimeHookResult {
                value: Vec::<PlatformRequest>::new(),
                view_changed: state.requests,
            }
        })
        .with_platform_service_requests(|state: &mut State, _| {
            if state.services {
                state.value += 1;
            }
            RuntimeHookResult {
                value: Vec::<PlatformServiceRequest>::new(),
                view_changed: state.services,
            }
        })
        .with_platform_responses(|state: &mut State, _| {
            if state.responses {
                state.value += 1;
            }
            RuntimeHookResult {
                value: (),
                view_changed: state.responses,
            }
        });
    let mut session = RuntimeSession::new();
    let mut ids = PlatformRequestIdAllocator::default();
    let metrics = RuntimeMetrics {
        physical_size: crate::platform::PixelSize::new(400, 300),
        viewport: VIEWPORT,
        scale_factor: 1.0,
        dpi_scale: 1.0,
        elapsed: Duration::ZERO,
    };
    let response = crate::platform::PlatformServiceResponse::new(
        crate::platform::PlatformRequestId(7),
        crate::platform::PlatformResponse::Cursor(crate::platform::CursorResponse::Applied),
    );
    let mut builds = 0;
    for step in 0..8 {
        state.before = step == 2;
        state.requests = step == 3;
        state.services = step == 4;
        state.responses = step == 5;
        if step == 6 {
            session.invalidate_view();
        }
        session.apply_before_render(&mut hooks, &mut state, metrics);
        assert!(session
            .take_platform_requests(&mut hooks, &mut state, metrics, &mut ids)
            .is_empty());
        session.apply_platform_responses(&mut hooks, &mut state, std::slice::from_ref(&response));
        let document = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |_, _| {
                    builds += 1;
                    let mut doc = UiDocument::new(LayoutStyle::column());
                    doc.add_child(
                        doc.root(),
                        UiNode::text(
                            "value",
                            state.value.to_string(),
                            crate::TextStyle::default(),
                            LayoutStyle::size(100.0, 30.0),
                        ),
                    );
                    doc
                },
            )
            .unwrap();
        assert!(
            matches!(document.node(UiNodeId(1)).content(), crate::UiContent::Text(text) if text.text == state.value.to_string())
        );
        assert_eq!(
            builds,
            match step {
                0 | 1 => 1,
                2..=6 => step,
                7 => 6,
                _ => unreachable!(),
            }
        );
        session.retain_document(document);
    }
}

#[test]
fn platform_work_can_run_without_rebuilding_and_plain_outputs_stay_conservative() {
    use crate::platform::{CursorRequest, CursorShape, PlatformRequestId};
    use crate::runtime::{RuntimeHookResult, RuntimeHooks, RuntimeMetrics};
    let mut session = RuntimeSession::new();
    let metrics = RuntimeMetrics {
        physical_size: crate::platform::PixelSize::new(400, 300),
        viewport: VIEWPORT,
        scale_factor: 1.0,
        dpi_scale: 1.0,
        elapsed: Duration::ZERO,
    };
    let mut builds = 0;
    let mut render = |session: &mut RuntimeSession| {
        let doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |_, _| {
                    builds += 1;
                    document(&["a"])
                },
            )
            .unwrap();
        session.retain_document(doc);
    };
    render(&mut session);
    let cursor = PlatformRequest::Cursor(CursorRequest::SetShape(CursorShape::Default));
    let owned_id = PlatformRequestId(91);
    let mut responses_seen = 0;
    let mut hooks = RuntimeHooks::new()
        .with_before_render(|_: &mut usize, _| RuntimeHookResult::unchanged(()))
        .with_platform_requests(move |_: &mut usize, _| {
            RuntimeHookResult::unchanged(vec![cursor.clone()])
        })
        .with_platform_service_requests(move |_: &mut usize, _| {
            RuntimeHookResult::unchanged(vec![PlatformServiceRequest::new(
                owned_id,
                PlatformRequest::Repaint(RepaintRequest::NextFrame),
            )])
        })
        .with_platform_responses(|seen: &mut usize, responses| {
            *seen += responses.len();
            RuntimeHookResult::unchanged(())
        });
    let mut ids = PlatformRequestIdAllocator::default();
    session.apply_before_render(&mut hooks, &mut responses_seen, metrics);
    let requests =
        session.take_platform_requests(&mut hooks, &mut responses_seen, metrics, &mut ids);
    assert_eq!(requests.len(), 2);
    assert_ne!(requests[0].id, owned_id);
    assert_eq!(requests[1].id, owned_id);
    let responses = [PlatformServiceResponse::new(
        requests[0].id,
        crate::platform::PlatformResponse::Cursor(crate::platform::CursorResponse::Applied),
    )];
    session.apply_platform_responses(&mut hooks, &mut responses_seen, &responses);
    assert_eq!(responses_seen, 1);
    render(&mut session);
    let mut conservative = RuntimeHooks::new()
        .with_before_render(|_: &mut usize, _| ())
        .with_platform_requests(|_: &mut usize, _| Vec::<PlatformRequest>::new())
        .with_platform_service_requests(|_: &mut usize, _| Vec::<PlatformServiceRequest>::new())
        .with_platform_responses(|_: &mut usize, _| ());
    session.apply_before_render(&mut conservative, &mut responses_seen, metrics);
    render(&mut session);
    session.take_platform_requests(&mut conservative, &mut responses_seen, metrics, &mut ids);
    render(&mut session);
    session.apply_platform_responses(&mut conservative, &mut responses_seen, &[]);
    render(&mut session);
    session.apply_platform_responses(&mut conservative, &mut responses_seen, &responses);
    render(&mut session);
    assert_eq!(builds, 4, "one initial build, then each conservative callback phase; no builds for unchanged work or an empty response batch");
}

#[test]
fn scheduling_distinguishes_view_invalidation_from_presentation() {
    let mut session = RuntimeSession::new();
    let mut builds = 0;
    for now in [0, 10, 20, 50] {
        if now == 20 {
            session.invalidate_view();
        }
        session.begin_frame(Duration::from_millis(now));
        let mut doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |_, _views| {
                    builds += 1;
                    document(&["a"])
                },
            )
            .unwrap();
        frame(&mut session, &mut doc, Vec::new());
        session.retain_document(doc);
        if now == 0 {
            session.request_repaint(
                Duration::ZERO,
                RepaintRequest::After(Duration::from_millis(50)),
            );
        }
        session.frame_presented();
        assert_eq!(
            session.next_frame_delay(Duration::from_millis(now)),
            (now < 50).then(|| Duration::from_millis(50 - now))
        );
    }
    assert_eq!(builds, 2);
}

#[test]
fn failed_presentation_retries_without_losing_an_in_frame_request() {
    let mut session = RuntimeSession::new();
    session.begin_frame(Duration::ZERO);
    session.request_repaint(
        Duration::ZERO,
        RepaintRequest::After(Duration::from_millis(50)),
    );
    session.frame_failed(Duration::ZERO);
    session.animations_active = true;
    assert_eq!(
        session.next_frame_delay(Duration::ZERO),
        Some(Duration::from_millis(16)),
        "animation must not bypass a surface retry deadline"
    );
    session.animations_active = false;
    session.begin_frame(Duration::from_millis(16));
    session.request_repaint(Duration::from_millis(16), RepaintRequest::NextFrame);
    session.frame_presented();
    assert_eq!(
        session.next_frame_delay(Duration::from_millis(16)),
        Some(Duration::ZERO)
    );
    session.begin_frame(Duration::from_millis(17));
    session.frame_presented();
    assert_eq!(
        session.next_frame_delay(Duration::from_millis(17)),
        Some(Duration::from_millis(33))
    );
}

fn document(names: &[&str]) -> UiDocument {
    let mut document = UiDocument::new(LayoutStyle::column().with_size(400.0, 300.0));
    for name in names {
        document.add_child(
            document.root(),
            UiNode::container(*name, LayoutStyle::size(100.0, 30.0))
                .with_input(InputBehavior::BUTTON),
        );
    }
    document
}

fn add_modal_barrier(document: &mut UiDocument) -> UiNodeId {
    document.add_child(
        document.root(),
        UiNode::container(
            "modal.barrier",
            LayoutStyle::absolute_rect(UiRect::new(2.0, 2.0, 70.0, 50.0)),
        )
        .with_accessibility(
            crate::AccessibilityMeta::new(crate::AccessibilityRole::Dialog)
                .modal()
                .focusable(),
        ),
    )
}

fn node(document: &UiDocument, name: &str) -> UiNodeId {
    UiNodeId(
        document
            .nodes()
            .iter()
            .position(|node| node.name() == name)
            .unwrap(),
    )
}

fn prepare(session: &mut RuntimeSession, document: &mut UiDocument) {
    session
        .prepare_document(
            document,
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
}

fn frame(
    session: &mut RuntimeSession,
    document: &mut UiDocument,
    events: Vec<RawInputEvent>,
) -> HostDocumentFrameOutput {
    let input = session
        .process_input(
            document,
            VIEWPORT,
            events,
            Vec::new(),
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    session
        .finish_frame(
            document,
            VIEWPORT,
            RenderTarget::window("test", VIEWPORT),
            input,
            &mut ApproxTextMeasurer,
            &mut PlatformRequestIdAllocator::default(),
        )
        .unwrap()
}

fn press(session: &mut RuntimeSession, document: &mut UiDocument, name: &str) {
    let id = node(document, name);
    let rect = document.node(id).layout().rect;
    frame(
        session,
        document,
        vec![RawInputEvent::Pointer(RawPointerEvent::new(
            PointerEventKind::Down(PointerButton::Primary),
            UiPoint::new(rect.x + 5.0, rect.y + 5.0),
            1,
        ))],
    );
    assert_eq!(session.interaction().pressed, Some(id));
    assert_eq!(session.interaction().focused, Some(id));
}

fn click(session: &mut RuntimeSession, document: &mut UiDocument, name: &str) {
    press(session, document, name);
    let rect = document.node(node(document, name)).layout().rect;
    frame(
        session,
        document,
        vec![RawInputEvent::Pointer(RawPointerEvent::new(
            PointerEventKind::Up(PointerButton::Primary),
            UiPoint::new(rect.x + 5.0, rect.y + 5.0),
            2,
        ))],
    );
}

#[test]
fn ime_cursor_geometry_matches_transformed_clipped_document_geometry() {
    use crate::{AnimatedValues, AnimationMachine, AnimationState, TextInputSnapshot};
    for ui_scale in [1.0, 1.5] {
        for paint_scale in [0.5, 1.0, 2.0, -1.0] {
            let mut session = RuntimeSession::new();
            let mut doc = UiDocument::new(LayoutStyle::size(140.0, 100.0));
            let root = doc.root();
            doc.node_mut(root).style.clip = crate::ClipBehavior::Clip;
            let editor = doc.add_child(
                root,
                UiNode::container(
                    "editor",
                    LayoutStyle::absolute_rect(UiRect::new(40.0, 30.0, 140.0, 80.0)),
                )
                .with_input(InputBehavior::BUTTON),
            );
            let cursor = UiRect::new(200.0, -20.0, 1.0, 18.0);
            doc.node_mut(editor)
                .set_text_input(Some(TextInputSnapshot::new("text", 4..4, cursor)));
            let translation = if paint_scale < 0.0 {
                UiPoint::new(150.0, 120.0)
            } else {
                UiPoint::new(10.0, 5.0)
            };
            doc.node_mut(editor).animation = Some(
                AnimationMachine::new(
                    vec![AnimationState::new(
                        "transform",
                        AnimatedValues::new(1.0, translation, paint_scale),
                    )],
                    Vec::new(),
                    "transform",
                )
                .unwrap(),
            );
            doc.set_focus_state(UiFocusState {
                focused: Some(editor),
                ..Default::default()
            });
            session
                .prepare_document(
                    &mut doc,
                    VIEWPORT,
                    UiDocumentScale::new(ui_scale, 1.0),
                    None,
                    &mut ApproxTextMeasurer,
                )
                .unwrap();
            frame(&mut session, &mut doc, Vec::new());
            // Compare the runtime's direct lookup against the geometry contract
            // used by hit testing and inspectors, including its visible bounds.
            let geometry = doc
                .effective_geometries()
                .into_iter()
                .find(|geometry| geometry.node == editor)
                .unwrap();
            let rect = geometry.original_rect;
            let visible = geometry.visible_rect().expect("partially visible editor");
            let mut expected = geometry.transform.transform_rect_bounds(UiRect::new(
                rect.x + cursor.x * ui_scale,
                rect.y + cursor.y * ui_scale,
                cursor.width * ui_scale,
                cursor.height * ui_scale,
            ));
            expected.width = expected.width.min(visible.width).max(1.0);
            expected.height = expected.height.min(visible.height).max(1.0);
            expected.x = expected
                .x
                .clamp(visible.x, (visible.right() - expected.width).max(visible.x));
            expected.y = expected.y.clamp(
                visible.y,
                (visible.bottom() - expected.height).max(visible.y),
            );
            assert_eq!(
                session.interaction().text_ime.as_ref().unwrap().cursor_rect,
                crate::platform::LogicalRect::new(
                    expected.x,
                    expected.y,
                    expected.width,
                    expected.height
                ),
                "ui_scale={ui_scale}, paint_scale={paint_scale}"
            );
        }
    }
}

#[test]
fn composition_sessions_keep_identity_geometry_and_reject_obsolete_events() {
    use crate::input::RawTextCompositionEvent;
    use crate::{TextCompositionEvent, TextInputSnapshot};
    let describe = |names: &[&str]| {
        let mut doc = document(names);
        for name in names {
            let id = node(&doc, name);
            doc.node_mut(id).set_text_input(Some(TextInputSnapshot::new(
                "a😀z",
                1..5,
                UiRect::new(12.0, 4.0, 1.0, 18.0),
            )));
        }
        doc
    };
    let mut session = RuntimeSession::new();
    let mut doc = describe(&["a", "b"]);
    prepare(&mut session, &mut doc);
    click(&mut session, &mut doc, "a");
    let original = session.interaction().text_ime.clone().unwrap();
    assert_eq!(original.selection, crate::platform::TextRange::new(1, 5));
    assert_eq!(original.cursor_rect.origin.y, 4.0);
    let compose = |input| {
        RawInputEvent::Composition(RawTextCompositionEvent {
            input,
            event: TextCompositionEvent::Preedit {
                text: "候補".into(),
                selection: Some(3..6),
                replacement: None,
            },
            timestamp_millis: 3,
        })
    };
    let mut reordered = describe(&["b", "a"]);
    prepare(&mut session, &mut reordered);
    let output = frame(
        &mut session,
        &mut reordered,
        vec![compose(original.input.clone())],
    );
    assert_eq!(
        session.interaction().text_ime.as_ref().unwrap().input,
        original.input
    );
    assert_eq!(
        session
            .interaction()
            .text_ime
            .as_ref()
            .unwrap()
            .cursor_rect
            .origin
            .y,
        34.0
    );
    assert!(
        matches!(&output.host_output.ui_events().cloned().collect::<Vec<_>>()[0], UiInputEvent::Composition { target: Some(target), .. } if *target == node(&reordered, "a"))
    );
    click(&mut session, &mut reordered, "b");
    assert_ne!(
        session.interaction().text_ime.as_ref().unwrap().input,
        original.input
    );
    let output = frame(&mut session, &mut reordered, vec![compose(original.input)]);
    assert!(
        output.host_output.ui_events().next().is_none(),
        "a previous field's event was retargeted"
    );
    let mut removed = describe(&["a"]);
    prepare(&mut session, &mut removed);
    frame(&mut session, &mut removed, Vec::new());
    assert!(session.interaction().text_ime.is_none());
}

#[test]
fn rebound_text_fields_cancel_the_original_owner_and_reject_queued_composition() {
    use crate::input::RawTextCompositionEvent;
    use crate::{TextCompositionEvent, TextInputSnapshot};

    let describe = |binding: Option<WidgetActionBinding>, inserted, text: &str| {
        let mut doc = document(if inserted {
            &["prefix", "editor"]
        } else {
            &["editor"]
        });
        let editor = node(&doc, "editor");
        doc.node_mut(editor).action = binding;
        doc.node_mut(editor)
            .set_text_input(Some(TextInputSnapshot::new(
                text,
                text.len()..text.len(),
                UiRect::new(12.0, 4.0, 1.0, 18.0),
            )));
        doc.set_focus_state(UiFocusState {
            focused: Some(editor),
            ..Default::default()
        });
        doc
    };
    for (original_binding, replacement_binding) in [
        (
            Some(WidgetActionBinding::action("first")),
            Some(WidgetActionBinding::action("second")),
        ),
        (
            Some(WidgetActionBinding::action("field")),
            Some(WidgetActionBinding::command("field")),
        ),
        (Some(WidgetActionBinding::action("first")), None),
        (None, Some(WidgetActionBinding::action("second"))),
    ] {
        let mut session = RuntimeSession::new();
        let mut doc = describe(original_binding.clone(), false, "before");
        prepare(&mut session, &mut doc);
        frame(&mut session, &mut doc, Vec::new());
        let original = session
            .interaction()
            .text_ime
            .as_ref()
            .unwrap()
            .input
            .clone();
        let compose = |input, event| {
            RawInputEvent::Composition(RawTextCompositionEvent {
                input,
                event,
                timestamp_millis: 1,
            })
        };

        // Text/selection changes and shifted local IDs retain the same owner.
        let mut edited = describe(original_binding.clone(), true, "edited text");
        prepare(&mut session, &mut edited);
        let output = frame(
            &mut session,
            &mut edited,
            vec![compose(
                original.clone(),
                TextCompositionEvent::Preedit {
                    text: "候補".into(),
                    selection: None,
                    replacement: None,
                },
            )],
        );
        assert_eq!(
            session.interaction().text_ime.as_ref().unwrap().input,
            original
        );
        assert_eq!(output.host_output.ui_events().count(), 1);
        assert!(session.take_interaction_cancellations().is_empty());

        let mut rebound = describe(replacement_binding.clone(), false, "replacement");
        prepare(&mut session, &mut rebound);
        assert!(
            session.interaction().text_ime.is_none(),
            "rebinding must revoke the old session before queued input"
        );
        let cancellations = session.take_interaction_cancellations();
        assert_eq!(cancellations.len(), usize::from(original_binding.is_some()));
        if let Some(binding) = &original_binding {
            assert_eq!(&cancellations[0].binding, binding);
            assert!(matches!(
                &cancellations[0].kind,
                WidgetActionKind::TextEdit(crate::WidgetTextEdit {
                    event: UiInputEvent::Composition {
                        target: None,
                        event: TextCompositionEvent::Cancel
                    },
                    ..
                })
            ));
        }
        prepare(&mut session, &mut rebound);
        assert!(
            session.take_interaction_cancellations().is_empty(),
            "cancellation is delivered once"
        );
        let commit = |input| {
            compose(
                input,
                TextCompositionEvent::Commit {
                    text: "committed".into(),
                    replacement: None,
                },
            )
        };
        let output = frame(&mut session, &mut rebound, vec![commit(original.clone())]);
        assert!(
            output.host_output.ui_events().next().is_none(),
            "old composition must not reach the new binding"
        );
        assert!(crate::host::collect_document_widget_actions(&output).is_empty());
        let replacement = session
            .interaction()
            .text_ime
            .as_ref()
            .unwrap()
            .input
            .clone();
        assert_ne!(replacement, original);
        assert_eq!(output.platform_requests().iter().filter(|request| matches!(request,
            PlatformRequest::TextIme(TextImeRequest::Deactivate { input }) if *input == original
        )).count(), 1);
        assert_eq!(output.platform_requests().iter().filter(|request| matches!(request,
            PlatformRequest::TextIme(TextImeRequest::Activate(ime)) if ime.input == replacement
        )).count(), 1);

        let output = frame(&mut session, &mut rebound, vec![commit(replacement)]);
        assert_eq!(output.host_output.ui_events().count(), 1);
        let actions = crate::host::collect_document_widget_actions(&output);
        assert_eq!(actions.len(), usize::from(replacement_binding.is_some()));
        if let Some(binding) = replacement_binding {
            assert_eq!(actions[0].binding, binding);
        }
    }
}

#[test]
fn explicit_ime_sessions_route_to_the_focused_custom_editor_and_ignore_stale_deactivation() {
    use crate::input::RawTextCompositionEvent;
    use crate::platform::{LogicalRect, TextImeSession, TextInputId};
    let mut session = RuntimeSession::new();
    let mut doc = document(&["custom"]);
    prepare(&mut session, &mut doc);
    press(&mut session, &mut doc, "custom");
    let ime = TextImeSession::new(
        TextInputId::new("custom-edit"),
        LogicalRect::new(10.0, 10.0, 1.0, 20.0),
    );
    session.apply_text_ime_request(&TextImeRequest::Activate(ime.clone()));
    // Explicit sessions retain their application-owned lifetime even if the
    // custom editor changes its dispatch binding or has no text snapshot.
    doc.set_node_action(node(&doc, "custom"), "custom.binding");
    prepare(&mut session, &mut doc);
    let result = frame(
        &mut session,
        &mut doc,
        vec![RawInputEvent::Composition(RawTextCompositionEvent {
            input: ime.input.clone(),
            event: crate::TextCompositionEvent::Commit {
                text: "你好".into(),
                replacement: None,
            },
            timestamp_millis: 2,
        })],
    );
    assert!(
        matches!(result.host_output.ui_events().cloned().collect::<Vec<_>>().as_slice(), [UiInputEvent::Composition { target: Some(target), .. }] if *target == node(&doc, "custom"))
    );
    assert_eq!(session.interaction().text_ime, Some(ime.clone()));
    session.apply_text_ime_request(&TextImeRequest::Deactivate {
        input: TextInputId::new("obsolete"),
    });
    assert_eq!(session.interaction().text_ime, Some(ime.clone()));
    session.apply_text_ime_request(&TextImeRequest::Deactivate { input: ime.input });
    assert!(session.interaction().text_ime.is_none());
}

#[cfg(feature = "widgets")]
#[test]
fn composition_commit_and_focus_changes_are_dispatched_in_event_order() {
    use crate::input::RawTextCompositionEvent;
    use crate::{TextCompositionEvent, WidgetActionKind};
    for commit_first in [true, false] {
        for return_to_original in [false, true] {
            let mut session = RuntimeSession::new();
            let mut doc = UiDocument::new(LayoutStyle::column().with_size(400.0, 300.0));
            let root = doc.root();
            for name in ["a", "b"] {
                crate::widgets::text_input(
                    &mut doc,
                    root,
                    name,
                    &crate::widgets::TextInputState::new(""),
                    crate::widgets::TextInputOptions::default().with_edit_action(name),
                );
            }
            prepare(&mut session, &mut doc);
            click(&mut session, &mut doc, "a");
            let input = session
                .interaction()
                .text_ime
                .as_ref()
                .unwrap()
                .input
                .clone();
            let commit = RawInputEvent::Composition(RawTextCompositionEvent {
                input,
                event: TextCompositionEvent::Commit {
                    text: "候補".into(),
                    replacement: None,
                },
                timestamp_millis: 3,
            });
            let b = doc.node(node(&doc, "b")).layout().rect;
            let click = RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Down(PointerButton::Primary),
                UiPoint::new(b.x + 5.0, b.y + 5.0),
                4,
            ));
            let mut events = vec![click];
            if return_to_original {
                let a = doc.node(node(&doc, "a")).layout().rect;
                events.extend([
                    RawInputEvent::Pointer(RawPointerEvent::new(
                        PointerEventKind::Up(PointerButton::Primary),
                        UiPoint::new(b.x + 5.0, b.y + 5.0),
                        5,
                    )),
                    RawInputEvent::Pointer(RawPointerEvent::new(
                        PointerEventKind::Down(PointerButton::Primary),
                        UiPoint::new(a.x + 5.0, a.y + 5.0),
                        6,
                    )),
                ]);
            }
            if commit_first {
                events.insert(0, commit);
            } else {
                events.push(commit);
            }
            let output = frame(&mut session, &mut doc, events);
            let actions = crate::host::collect_document_widget_actions(&output);
            let commits: Vec<_> = actions.iter().filter(|action| matches!(&action.kind,
            WidgetActionKind::TextEdit(edit) if matches!(edit.event, UiInputEvent::Composition { event: TextCompositionEvent::Commit { .. }, .. })
        )).collect();
            assert_eq!(commits.len(), usize::from(commit_first));
            if commit_first {
                assert_eq!(commits[0].binding.action_id().unwrap().as_str(), "a");
            }
        }
    }
}

#[cfg(feature = "widgets")]
#[test]
fn composition_cancellation_rejects_queued_commits_across_frame_boundaries() {
    use crate::host::collect_document_widget_actions;
    use crate::input::RawTextCompositionEvent;
    use crate::widgets::{text_input, TextInputOptions, TextInputState};
    use crate::TextCompositionEvent;

    let draft = TextCompositionEvent::Preedit {
        text: "候補".into(),
        selection: Some(6..6),
        replacement: None,
    };
    let options = TextInputOptions {
        focused: true,
        layout: LayoutStyle::absolute_rect(UiRect::new(0.0, 0.0, 300.0, 40.0)),
        ..TextInputOptions::default().with_edit_action("editor")
    };
    let describe = |state: &TextInputState| {
        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
        let root = doc.root();
        let field = text_input(&mut doc, root, "editor", state, options.clone());
        doc.set_focus_state(UiFocusState {
            focused: Some(field),
            ..Default::default()
        });
        doc
    };
    let compose = |input, event| {
        RawInputEvent::Composition(RawTextCompositionEvent {
            input,
            event,
            timestamp_millis: 2,
        })
    };
    let mut failures = Vec::new();
    for cancellation in ["pointer", "escape", "ime"] {
        for displayed_draft in [false, true] {
            for commit_first in [false, true] {
                for clear_draft in [false, true] {
                    let event_count =
                        if displayed_draft { 2 } else { 3 } + usize::from(clear_draft);
                    for boundaries in 0..1 << (event_count - 1) {
                        let mut state = TextInputState::new("start tail");
                        state.set_caret(5);
                        if displayed_draft {
                            state.apply_composition(&draft);
                        }
                        let mut session = RuntimeSession::new();
                        let mut doc = describe(&state);
                        prepare(&mut session, &mut doc);
                        frame(&mut session, &mut doc, Vec::new());
                        let original = session
                            .interaction()
                            .text_ime
                            .as_ref()
                            .unwrap()
                            .input
                            .clone();
                        let commit = compose(
                            original.clone(),
                            TextCompositionEvent::Commit {
                                text: "候補".into(),
                                replacement: None,
                            },
                        );
                        let pointer = RawInputEvent::Pointer(RawPointerEvent::new(
                            PointerEventKind::Down(PointerButton::Primary),
                            UiPoint::new(8.0, 14.0),
                            3,
                        ));
                        let cancel = match cancellation {
                            "pointer" => pointer,
                            "escape" => {
                                RawInputEvent::Keyboard(crate::input::RawKeyboardEvent::press(
                                    crate::KeyCode::Escape,
                                    crate::KeyModifiers::default(),
                                    3,
                                ))
                            }
                            _ => compose(original.clone(), TextCompositionEvent::Cancel),
                        };
                        let mut events = Vec::new();
                        if !displayed_draft {
                            events.push(compose(original.clone(), draft.clone()));
                        }
                        if clear_draft {
                            events.push(compose(
                                original.clone(),
                                TextCompositionEvent::Preedit {
                                    text: String::new(),
                                    selection: None,
                                    replacement: None,
                                },
                            ));
                        }
                        if commit_first {
                            events.extend([commit, cancel]);
                        } else {
                            events.extend([cancel, commit]);
                        }
                        let mut pending = Vec::new();
                        let mut ime_requests = Vec::new();
                        for (index, event) in events.into_iter().enumerate() {
                            pending.push(event);
                            if boundaries & (1 << index) == 0 && index + 1 != event_count {
                                continue;
                            }
                            let output =
                                frame(&mut session, &mut doc, std::mem::take(&mut pending));
                            ime_requests.extend(
                                output
                                    .host_output
                                    .platform_requests
                                    .iter()
                                    .map(|request| request.request.clone()),
                            );
                            for action in collect_document_widget_actions(&output) {
                                if let WidgetActionKind::TextEdit(edit) = action.kind {
                                    state.apply_widget_text_edit(&edit, &options);
                                }
                            }
                            doc = describe(&state);
                            prepare(&mut session, &mut doc);
                            frame(&mut session, &mut doc, Vec::new());
                        }
                        let expected = if commit_first {
                            "start候補 tail"
                        } else {
                            "start tail"
                        };
                        if state.text() != expected {
                            failures.push(format!("{cancellation}: displayed={displayed_draft}, commit_first={commit_first}, clear={clear_draft}, boundaries={boundaries:b}: {:?} != {expected:?}", state.text()));
                            continue;
                        }
                        assert!(state.composition().is_none());
                        assert!(!session.interaction().text_composition.is_active());
                        assert_eq!(state.history().can_undo(), commit_first);
                        let current = session
                            .interaction()
                            .text_ime
                            .as_ref()
                            .unwrap()
                            .input
                            .clone();
                        assert_eq!(current == original, commit_first);
                        if !commit_first {
                            let deactivated = ime_requests.iter().position(|request| matches!(request,
                            PlatformRequest::TextIme(TextImeRequest::Deactivate { input }) if *input == original)).unwrap();
                            let activated = ime_requests.iter().position(|request| matches!(request,
                            PlatformRequest::TextIme(TextImeRequest::Activate(session)) if session.input == current)).unwrap();
                            assert!(
                                deactivated < activated,
                                "retire the canceled platform session before reopening input"
                            );
                        }
                        let output = frame(
                            &mut session,
                            &mut doc,
                            vec![compose(
                                current,
                                TextCompositionEvent::Commit {
                                    text: "fresh".into(),
                                    replacement: None,
                                },
                            )],
                        );
                        for action in collect_document_widget_actions(&output) {
                            if let WidgetActionKind::TextEdit(edit) = action.kind {
                                state.apply_widget_text_edit(&edit, &options);
                            }
                        }
                        assert!(
                            state.text().contains("fresh"),
                            "new input session must remain usable"
                        );
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches (pointer={}, Escape={}, IME cancel={}); first cases: {:#?}",
        failures.len(),
        failures
            .iter()
            .filter(|failure| failure.starts_with("pointer:"))
            .count(),
        failures
            .iter()
            .filter(|failure| failure.starts_with("escape:"))
            .count(),
        failures
            .iter()
            .filter(|failure| failure.starts_with("ime:"))
            .count(),
        &failures[..failures.len().min(12)]
    );
}

#[cfg(feature = "widgets")]
#[test]
fn replacing_or_clearing_a_composition_snapshot_revokes_queued_input() {
    use crate::host::collect_document_widget_actions;
    use crate::input::RawTextCompositionEvent;
    use crate::widgets::{text_input, TextInputOptions, TextInputState};
    use crate::TextCompositionEvent;

    let options = TextInputOptions::default().with_edit_action("editor");
    let describe = |state: &TextInputState| {
        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
        let root = doc.root();
        let editor = text_input(&mut doc, root, "editor", state, options.clone());
        doc.set_focus_state(UiFocusState {
            focused: Some(editor),
            ..Default::default()
        });
        doc
    };
    let mut failures = Vec::new();
    for reset in ["cancel", "caret", "replace", "same text"] {
        let mut state = TextInputState::new("original");
        state.set_selection(1, 5);
        state.apply_composition(&TextCompositionEvent::Preedit {
            text: "候補".into(),
            selection: Some(6..6),
            replacement: None,
        });
        let mut session = RuntimeSession::new();
        let mut doc = describe(&state);
        prepare(&mut session, &mut doc);
        frame(&mut session, &mut doc, Vec::new());
        let old_input = session
            .interaction()
            .text_ime
            .as_ref()
            .unwrap()
            .input
            .clone();
        match reset {
            "cancel" => {
                state.apply_composition(&TextCompositionEvent::Cancel);
            }
            "caret" => state.set_caret(0),
            "replace" => state.set_text("replacement"),
            _ => state.set_text("original"),
        }
        let expected = state.text().to_owned();
        let expected_caret = state.caret();
        let expected_selection = state.selection_anchor();
        doc = describe(&state);
        prepare(&mut session, &mut doc);
        let cleanup = session.take_interaction_cancellations();
        assert_eq!(cleanup.len(), 1, "the old model receives one cancellation");
        assert_eq!(cleanup[0].binding.action_id().unwrap().as_str(), "editor");
        for cancellation in cleanup {
            let WidgetActionKind::TextEdit(edit) = cancellation.kind else {
                panic!("expected text cancellation")
            };
            state.apply_widget_text_edit(&edit, &options);
        }
        prepare(&mut session, &mut doc);
        assert!(session.take_interaction_cancellations().is_empty());
        let output = frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Composition(RawTextCompositionEvent {
                input: old_input.clone(),
                event: TextCompositionEvent::Commit {
                    text: "late".into(),
                    replacement: None,
                },
                timestamp_millis: 5,
            })],
        );
        for action in collect_document_widget_actions(&output) {
            if let WidgetActionKind::TextEdit(edit) = action.kind {
                state.apply_widget_text_edit(&edit, &options);
            }
        }
        if state.text() != expected {
            failures.push(format!("{reset}: {:?} != {expected:?}", state.text()));
            continue;
        }
        assert_eq!(state.caret(), expected_caret);
        assert_eq!(state.selection_anchor(), expected_selection);
        assert!(!state.history().can_undo());
        assert_ne!(
            session.interaction().text_ime.as_ref().unwrap().input,
            old_input
        );
    }
    assert!(
        failures.is_empty(),
        "obsolete commits after model reset: {failures:#?}"
    );

    // Custom editors may publish an empty marked range while native input
    // waits to commit. Clearing that range is not an application cancellation.
    for pending_preedit in [false, true] {
        let mut state = TextInputState::new("original");
        state.set_caret(1);
        let mut doc = describe(&state);
        let editor = node(&doc, "editor");
        let mut snapshot = doc.node(editor).text_input().unwrap().clone();
        snapshot.composition = Some(1..1);
        doc.node_mut(editor).set_text_input(Some(snapshot));
        let mut session = RuntimeSession::new();
        prepare(&mut session, &mut doc);
        frame(&mut session, &mut doc, Vec::new());
        let input = session
            .interaction()
            .text_ime
            .as_ref()
            .unwrap()
            .input
            .clone();
        if pending_preedit {
            // Input can run ahead of the next published draft. A previously empty
            // range must not be mistaken for a discarded nonempty draft.
            frame(
                &mut session,
                &mut doc,
                vec![RawInputEvent::Composition(RawTextCompositionEvent {
                    input: input.clone(),
                    event: TextCompositionEvent::Preedit {
                        text: "pending".into(),
                        selection: None,
                        replacement: None,
                    },
                    timestamp_millis: 5,
                })],
            );
        }
        doc = describe(&state);
        prepare(&mut session, &mut doc);
        let output = frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Composition(RawTextCompositionEvent {
                input: input.clone(),
                event: TextCompositionEvent::Commit {
                    text: "valid".into(),
                    replacement: None,
                },
                timestamp_millis: 6,
            })],
        );
        for action in collect_document_widget_actions(&output) {
            if let WidgetActionKind::TextEdit(edit) = action.kind {
                state.apply_widget_text_edit(&edit, &options);
            }
        }
        assert_eq!(
            state.text(),
            "ovalidriginal",
            "empty marked range dropped a valid commit"
        );
        assert_eq!(
            session.interaction().text_ime.as_ref().unwrap().input,
            input
        );
        state.undo_text_edit().unwrap();
        assert_eq!(state.text(), "original");
    }
}

#[cfg(feature = "widgets")]
#[test]
fn text_selection_is_independent_of_pointer_event_frame_boundaries() {
    use crate::host::collect_document_widget_actions;
    use crate::widgets::{text_input, TextInputOptions, TextInputState};

    let events = [
        (
            PointerEventKind::Down(PointerButton::Primary),
            UiPoint::new(7.0, 12.0),
        ),
        (PointerEventKind::Move, UiPoint::new(29.0, 12.0)),
        (
            PointerEventKind::Up(PointerButton::Primary),
            UiPoint::new(220.0, 12.0),
        ),
        (
            PointerEventKind::Down(PointerButton::Primary),
            UiPoint::new(15.0, 52.0),
        ),
        (
            PointerEventKind::Up(PointerButton::Primary),
            UiPoint::new(15.0, 52.0),
        ),
        (PointerEventKind::Move, UiPoint::new(70.0, 12.0)),
    ];
    for boundaries in 0..1 << (events.len() - 1) {
        let mut session = RuntimeSession::new();
        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
        let mut states = [TextInputState::new("abcdef"), TextInputState::new("ghijkl")];
        let options = TextInputOptions::default();
        let root = doc.root();
        let fields: Vec<_> = states
            .iter()
            .enumerate()
            .map(|(index, state)| {
                text_input(
                    &mut doc,
                    root,
                    format!("field.{index}"),
                    state,
                    TextInputOptions {
                        layout: LayoutStyle::absolute_rect(UiRect::new(
                            0.0,
                            index as f32 * 40.0,
                            180.0,
                            30.0,
                        )),
                        ..options.clone().with_edit_action(format!("field.{index}"))
                    },
                )
            })
            .collect();
        prepare(&mut session, &mut doc);
        let mut phases = Vec::new();
        let mut pending = Vec::new();
        for (index, (kind, point)) in events.iter().copied().enumerate() {
            pending.push(RawInputEvent::Pointer(RawPointerEvent::new(
                kind,
                point,
                index as u64,
            )));
            if boundaries & (1 << index) == 0 && index + 1 != events.len() {
                continue;
            }
            let output = frame(&mut session, &mut doc, std::mem::take(&mut pending));
            for action in collect_document_widget_actions(&output) {
                if let WidgetActionKind::TextEdit(edit) = action.kind {
                    if edit.local_position.is_some() {
                        let field = fields.iter().position(|id| *id == action.target).unwrap();
                        phases.push((field, edit.phase));
                        states[field].apply_widget_text_edit(&edit, &options);
                    }
                }
            }
        }
        assert_eq!(
            phases,
            [
                (0, WidgetValueEditPhase::Begin),
                (0, WidgetValueEditPhase::Update),
                (0, WidgetValueEditPhase::Commit),
                (1, WidgetValueEditPhase::Begin),
            ],
            "frame boundaries={boundaries:05b}"
        );
        // Stationary releases end capture without reinterpreting the caret.
        assert!(session.interaction().pressed.is_none());
        assert_eq!(
            states[0].selected_text(),
            Some("abcdef"),
            "frame boundaries={boundaries:05b}"
        );
        assert!(
            states[1].selected_text().is_none(),
            "frame boundaries={boundaries:05b}"
        );
    }
}

#[test]
fn focus_press_and_gesture_follow_identity_across_insertions_and_reorders() {
    let mut session = RuntimeSession::new();
    let mut first = document(&["a", "b"]);
    prepare(&mut session, &mut first);
    press(&mut session, &mut first, "b");
    for names in [vec!["new", "a", "b"], vec!["b", "a", "new"], vec!["a", "b"]] {
        let mut next = document(&names);
        prepare(&mut session, &mut next);
        let expected = node(&next, "b");
        assert_eq!(session.interaction().focused, Some(expected));
        assert_eq!(session.interaction().pressed, Some(expected));
        assert_eq!(session.interaction().drag_capture.unwrap().target, expected);
        assert_eq!(
            session
                .interaction()
                .gesture_tracker
                .active_capture(PointerId::MOUSE)
                .unwrap()
                .target,
            expected
        );
        frame(&mut session, &mut next, Vec::new());
    }
}

fn editor_document(names: &[&str], playhead_x: f32, enabled: bool) -> UiDocument {
    use crate::{AccessibilityMeta, AccessibilityRole, WidgetActionBinding};
    let mut doc = UiDocument::new(LayoutStyle::size(VIEWPORT.width, VIEWPORT.height));
    for (index, name) in names.iter().enumerate() {
        let control = if *name == "playhead" {
            let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Group);
            accessibility.enabled = enabled;
            UiNode::container(
                *name,
                crate::layout::absolute(playhead_x, 60.0, 30.0, 120.0),
            )
            // Orbifold's editing handles participate in pointer input without focus.
            .with_input(InputBehavior {
                pointer: true,
                focusable: false,
                keyboard: false,
            })
            .with_pointer_edit_action(WidgetActionBinding::action("timeline.seek"))
            .with_accessibility(accessibility)
        } else {
            UiNode::container(
                *name,
                crate::layout::absolute(index as f32 * 70.0, 0.0, 60.0, 30.0),
            )
            .with_input(InputBehavior::BUTTON)
            .with_action(WidgetActionBinding::action(format!("control.{name}")))
        };
        doc.add_child(doc.root(), control);
    }
    doc
}

#[test]
fn editor_drag_dispatches_to_its_semantic_owner_through_view_rebuilds() {
    use crate::host::collect_document_widget_actions;
    use crate::{WidgetActionBinding, WidgetActionKind, WidgetValueEditPhase};

    let mut session = RuntimeSession::new();
    let mut revision = 0;
    let mut builds = 0;
    let mut phases = Vec::new();
    let down = UiPoint::new(85.0, 75.0);
    let moves = [UiPoint::new(130.0, 90.0), UiPoint::new(170.0, 100.0)];
    let outside = [
        UiPoint::new(420.0, 320.0),
        UiPoint::new(460.0, 350.0),
        UiPoint::new(490.0, 360.0),
    ];
    let batches = [
        vec![(PointerEventKind::Down(PointerButton::Primary), down)],
        vec![(PointerEventKind::Move, moves[0])],
        vec![(PointerEventKind::Move, moves[1])],
        vec![
            (PointerEventKind::Move, outside[0]),
            (PointerEventKind::Move, outside[1]),
            (PointerEventKind::Up(PointerButton::Primary), outside[2]),
        ],
        vec![(PointerEventKind::Move, down)],
    ];
    for (frame_index, batch) in batches.into_iter().enumerate() {
        session.begin_frame(Duration::from_millis(frame_index as u64 * 20));
        let playhead_x = 80.0 + revision as f32 * 20.0;
        let mut doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                batch.last().map(|(_, position)| *position),
                &mut ApproxTextMeasurer,
                |_, _views| {
                    builds += 1;
                    let names: &[&str] = match revision {
                        0 => &["transport", "playhead", "inspector"],
                        1 => &["selection", "inspector", "playhead", "transport"],
                        2 => &["playhead", "transport"],
                        _ => &["transport", "inspector", "playhead"],
                    };
                    editor_document(names, playhead_x, true)
                },
            )
            .unwrap();
        assert!(session.take_interaction_cancellations().is_empty());
        let input = batch
            .iter()
            .enumerate()
            .map(|(index, (kind, position))| {
                RawInputEvent::Pointer(RawPointerEvent::new(
                    *kind,
                    *position,
                    (frame_index * 20 + index) as u64,
                ))
            })
            .collect();
        let output = frame(&mut session, &mut doc, input);
        let actions = collect_document_widget_actions(&output);
        assert_eq!(actions.len(), [0, 1, 1, 3, 0][frame_index]);
        for (action_index, action) in actions.iter().enumerate() {
            assert_eq!(action.binding, WidgetActionBinding::action("timeline.seek"));
            assert_eq!(action.target, node(&doc, "playhead"));
            let WidgetActionKind::PointerEdit(edit) = action.kind else {
                panic!("editor drag dispatched an unrelated action: {action:?}");
            };
            let position = match frame_index {
                1 => down,
                2 => moves[1],
                3 => outside[action_index],
                _ => unreachable!(),
            };
            assert_eq!(edit.position, position);
            assert_eq!(edit.target_rect.x, playhead_x);
            assert_eq!(
                edit.local_position,
                UiPoint::new(position.x - playhead_x, position.y - 60.0)
            );
            phases.push(edit.phase);
        }
        if !actions.is_empty() {
            // Application updates rebuild the view and change the set/order of controls.
            revision += 1;
            session.invalidate_view();
        }
        session.retain_document(doc);
        session.frame_presented();
    }
    assert_eq!(builds, 4);
    assert_eq!(
        phases,
        vec![
            WidgetValueEditPhase::Begin,
            WidgetValueEditPhase::Update,
            WidgetValueEditPhase::Update,
            WidgetValueEditPhase::Update,
            WidgetValueEditPhase::Commit,
        ]
    );
    assert!(session.interaction().drag_capture.is_none());
}

#[test]
fn removed_or_disabled_editor_drag_cannot_activate_a_replacement() {
    use crate::host::collect_document_widget_actions;
    use crate::{
        AccessibilityMeta, AccessibilityRole, HitTestBehavior, WidgetActionBinding,
        WidgetActionKind, WidgetActionMode, WidgetValueEditPhase,
    };

    for (cancellation, mode) in [
        ("removed", WidgetActionMode::PointerEdit),
        ("disabled", WidgetActionMode::PointerEdit),
        ("ancestor", WidgetActionMode::PointerEdit),
        ("block", WidgetActionMode::PointerEdit),
        ("pass_through", WidgetActionMode::PointerEdit),
        ("binding", WidgetActionMode::PointerEdit),
        ("mode", WidgetActionMode::PointerEdit),
        ("removed", WidgetActionMode::Drag),
        ("modal", WidgetActionMode::PointerEdit),
        ("modal", WidgetActionMode::Drag),
    ] {
        let mut session = RuntimeSession::new();
        let mut first = editor_document(&["transport", "playhead"], 80.0, true);
        let target = node(&first, "playhead");
        first.node_mut(target).action_mode = mode;
        prepare(&mut session, &mut first);
        let output = frame(
            &mut session,
            &mut first,
            vec![
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Down(PointerButton::Primary),
                    UiPoint::new(85.0, 75.0),
                    0,
                )),
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Move,
                    UiPoint::new(130.0, 90.0),
                    1,
                )),
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Move,
                    UiPoint::new(160.0, 110.0),
                    2,
                )),
            ],
        );
        assert_eq!(collect_document_widget_actions(&output).len(), 2);
        let mut next = editor_document(
            if cancellation == "removed" {
                &["transport", "replacement"]
            } else {
                &["transport", "replacement", "playhead"]
            },
            80.0,
            cancellation != "disabled",
        );
        match cancellation {
            "modal" => {
                let target = node(&next, "playhead");
                next.node_mut(target).action_mode = mode;
                add_modal_barrier(&mut next);
            }
            "ancestor" => {
                let root = next.root();
                next.node_mut(root).accessibility =
                    Some(AccessibilityMeta::new(AccessibilityRole::Group).disabled());
            }
            "block" | "pass_through" => {
                let target = node(&next, "playhead");
                next.node_mut(target)
                    .set_hit_test_behavior(if cancellation == "block" {
                        HitTestBehavior::Block
                    } else {
                        HitTestBehavior::PassThrough
                    });
            }
            "binding" => {
                let target = node(&next, "playhead");
                next.node_mut(target).action = Some(WidgetActionBinding::action("unrelated.edit"));
            }
            "mode" => {
                let target = node(&next, "playhead");
                next.node_mut(target).action_mode = WidgetActionMode::Activate;
            }
            _ => {}
        }
        prepare(&mut session, &mut next);
        let cancellations = session.take_interaction_cancellations();
        assert_eq!(
            cancellations.len(),
            1,
            "{cancellation} lost application cleanup"
        );
        assert_eq!(
            cancellations[0].binding,
            WidgetActionBinding::action("timeline.seek")
        );
        match cancellations[0].kind {
            WidgetActionKind::PointerEdit(edit) => {
                assert_eq!(edit.phase, WidgetValueEditPhase::Cancel);
                assert_eq!(edit.position, UiPoint::new(160.0, 110.0));
                assert_eq!(edit.local_position, UiPoint::new(80.0, 50.0));
            }
            WidgetActionKind::Drag(edit) => {
                assert_eq!(edit.phase, crate::WidgetDragPhase::Cancel);
                assert_eq!(edit.origin, UiPoint::new(85.0, 75.0));
                assert_eq!(edit.current, UiPoint::new(160.0, 110.0));
                assert_eq!(edit.previous, edit.current);
                assert_eq!(edit.delta, UiPoint::new(0.0, 0.0));
            }
            _ => panic!("expected cancellation of the original edit"),
        }
        assert!(session.take_interaction_cancellations().is_empty());
        prepare(&mut session, &mut next);
        assert!(session.take_interaction_cancellations().is_empty());
        let replacement_position = UiPoint::new(75.0, 5.0);
        let output = frame(
            &mut session,
            &mut next,
            vec![
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Move,
                    replacement_position,
                    3,
                )),
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Up(PointerButton::Primary),
                    replacement_position,
                    4,
                )),
            ],
        );
        assert!(
            collect_document_widget_actions(&output).is_empty(),
            "{cancellation} allowed an orphaned drag to dispatch an action"
        );
        assert!(session.interaction().drag_capture.is_none());
        if cancellation == "ancestor" {
            let root = next.root();
            next.node_mut(root).accessibility = None;
            prepare(&mut session, &mut next);
        }
        if cancellation == "modal" {
            let modal = node(&next, "modal.barrier");
            next.node_mut(modal).style.layout.display = taffy::Display::None;
            prepare(&mut session, &mut next);
        }
        // The replacement is interactive; suppression applies only to the orphaned drag.
        let output = frame(
            &mut session,
            &mut next,
            vec![
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Down(PointerButton::Primary),
                    replacement_position,
                    5,
                )),
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Up(PointerButton::Primary),
                    replacement_position,
                    6,
                )),
            ],
        );
        let actions = collect_document_widget_actions(&output);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].binding,
            crate::WidgetActionBinding::action("control.replacement")
        );
    }
}

#[test]
fn removal_cancels_interaction_without_resurrecting_on_reinsertion() {
    let mut session = RuntimeSession::new();
    let mut first = document(&["original"]);
    prepare(&mut session, &mut first);
    press(&mut session, &mut first, "original");
    for names in [vec!["replacement"], vec!["original"]] {
        let mut next = document(&names);
        prepare(&mut session, &mut next);
        assert_eq!(session.interaction().focused, None);
        assert_eq!(session.interaction().pressed, None);
        assert_eq!(session.interaction().drag_capture, None);
        assert!(session
            .interaction()
            .gesture_tracker
            .active_capture(PointerId::MOUSE)
            .is_none());
        let output = frame(
            &mut session,
            &mut next,
            vec![RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Up(PointerButton::Primary),
                UiPoint::new(5.0, 5.0),
                2,
            ))],
        );
        assert!(output
            .host_output
            .gestures()
            .all(|gesture| !matches!(gesture, crate::GestureEvent::Click(_))));
    }
}

#[test]
fn queued_release_retains_press_owner_until_input_is_processed() {
    let mut session = RuntimeSession::new();
    let mut first = document(&["original"]);
    prepare(&mut session, &mut first);
    press(&mut session, &mut first, "original");
    let mut next = document(&["original"]);
    prepare(&mut session, &mut next);
    let output = frame(
        &mut session,
        &mut next,
        vec![RawInputEvent::Pointer(RawPointerEvent::new(
            PointerEventKind::Up(PointerButton::Primary),
            UiPoint::new(5.0, 5.0),
            2,
        ))],
    );
    assert_eq!(
        output.input_results().next().unwrap().clicked,
        Some(node(&next, "original"))
    );
    assert_eq!(session.interaction().pressed, None);
}

#[test]
fn pointer_click_policy_survives_rebuild_and_uses_action_owner() {
    use crate::host::collect_document_widget_actions;

    for mode in [
        WidgetActionMode::Activate,
        WidgetActionMode::ActivateAnyButton,
    ] {
        for hit_child in [false, true] {
            for button in [
                PointerButton::Primary,
                PointerButton::Auxiliary,
                PointerButton::Secondary,
                PointerButton::Back,
                PointerButton::Forward,
                PointerButton::Other(4),
            ] {
                let describe = |inserted| {
                    let mut doc = document(if inserted {
                        &["inserted", "control"]
                    } else {
                        &["control"]
                    });
                    if inserted {
                        // Change the index while keeping the control under the pointer.
                        doc.set_node_style(node(&doc, "inserted"), LayoutStyle::size(100.0, 0.0));
                    }
                    let control = node(&doc, "control");
                    doc.node_mut(control).action = Some("control.activate".into());
                    doc.node_mut(control).action_mode = mode;
                    if hit_child {
                        doc.add_child(
                            control,
                            UiNode::container("label", LayoutStyle::size(80.0, 25.0)).with_input(
                                InputBehavior {
                                    pointer: true,
                                    focusable: false,
                                    keyboard: false,
                                },
                            ),
                        );
                    }
                    doc
                };
                let mut session = RuntimeSession::new();
                let mut doc = describe(false);
                prepare(&mut session, &mut doc);
                frame(
                    &mut session,
                    &mut doc,
                    vec![RawInputEvent::Pointer(RawPointerEvent::new(
                        PointerEventKind::Down(button),
                        UiPoint::new(5.0, 5.0),
                        1,
                    ))],
                );
                let mut doc = describe(true);
                prepare(&mut session, &mut doc);
                let output = frame(
                    &mut session,
                    &mut doc,
                    vec![RawInputEvent::Pointer(RawPointerEvent::new(
                        PointerEventKind::Up(button),
                        UiPoint::new(5.0, 5.0),
                        2,
                    ))],
                );
                let allowed =
                    button == PointerButton::Primary || mode == WidgetActionMode::ActivateAnyButton;
                let expected =
                    allowed.then_some(node(&doc, if hit_child { "label" } else { "control" }));
                assert_eq!(
                    output.input_results().last().unwrap().clicked,
                    expected,
                    "{mode:?}, {button:?}, child={hit_child}"
                );
                let actions = collect_document_widget_actions(&output);
                assert_eq!(
                    actions.len(),
                    usize::from(allowed),
                    "{mode:?}, {button:?}, child={hit_child}: {actions:?}"
                );
                if allowed {
                    assert_eq!(actions[0].target, node(&doc, "control"));
                    assert!(
                        matches!(actions[0].kind, WidgetActionKind::Activate(ref activation)
                        if activation.pointer_button() == Some(button))
                    );
                }
                assert!(session.interaction().pressed.is_none());
                assert!(session.interaction().drag_capture.is_none());
            }
        }
    }
}

#[test]
fn click_release_uses_current_hit_instead_of_capture_target() {
    use crate::host::collect_document_widget_actions;

    for batched in [false, true] {
        for (release, activates) in [
            (UiPoint::new(5.0, 28.0), true),
            (UiPoint::new(5.0, 31.0), false),
            (UiPoint::new(101.0, 29.0), false),
        ] {
            let mut session = RuntimeSession::new();
            let mut doc = document(&["original", "neighbor"]);
            let original = node(&doc, "original");
            doc.node_mut(original).action = Some("original.activate".into());
            let neighbor = node(&doc, "neighbor");
            doc.node_mut(neighbor).action = Some("neighbor.activate".into());
            prepare(&mut session, &mut doc);
            let mut events = vec![RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Down(PointerButton::Primary),
                UiPoint::new(5.0, 29.0),
                1,
            ))];
            if !batched {
                frame(&mut session, &mut doc, std::mem::take(&mut events));
            }
            events.push(RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Up(PointerButton::Primary),
                release,
                2,
            )));
            let output = frame(&mut session, &mut doc, events);
            let actions = collect_document_widget_actions(&output);
            assert_eq!(
                actions.len(),
                usize::from(activates),
                "batched={batched}, release={release:?}: {actions:?}"
            );
            if activates {
                assert_eq!(actions[0].target, original);
            }
            assert!(session.interaction().pressed.is_none());
            assert!(session.interaction().drag_capture.is_none());
        }
    }
}

#[test]
fn keyboard_focus_survives_frames_independently_of_pointer_hit_policy() {
    use crate::host::collect_document_widget_actions;
    use crate::input::RawKeyboardEvent;
    use crate::{
        AccessibilityMeta, AccessibilityRole, HitTestBehavior, HitTestResult, KeyCode, KeyModifiers,
    };

    for policy in [
        HitTestBehavior::Auto,
        HitTestBehavior::PassThrough,
        HitTestBehavior::Block,
    ] {
        for accessibility_focus in [false, true] {
            for authored in [false, true] {
                for rebuild in [false, true] {
                    let describe = |inserted| {
                        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
                        if inserted {
                            doc.add_child(
                                doc.root(),
                                UiNode::container("padding", LayoutStyle::size(1.0, 1.0)),
                            );
                        }
                        let rect = UiRect::new(0.0, 0.0, 160.0, 30.0);
                        doc.add_child(
                            doc.root(),
                            UiNode::container("background", LayoutStyle::absolute_rect(rect))
                                .with_input(InputBehavior {
                                    pointer: true,
                                    focusable: false,
                                    keyboard: false,
                                }),
                        );
                        let mut control =
                            UiNode::container("control", LayoutStyle::absolute_rect(rect))
                                .with_input(InputBehavior {
                                    pointer: true,
                                    focusable: !accessibility_focus,
                                    keyboard: true,
                                })
                                .with_hit_test_behavior(policy)
                                .with_action("activate");
                        if accessibility_focus {
                            control = control.with_accessibility(
                                AccessibilityMeta::new(AccessibilityRole::Button).focusable(),
                            );
                        }
                        doc.add_child(doc.root(), control);
                        doc
                    };
                    let mut session = RuntimeSession::new();
                    let mut doc = describe(false);
                    if authored {
                        doc.set_focus_state(UiFocusState {
                            focused: Some(node(&doc, "control")),
                            ..Default::default()
                        });
                    }
                    prepare(&mut session, &mut doc);
                    if !authored {
                        frame(
                            &mut session,
                            &mut doc,
                            vec![RawInputEvent::Keyboard(RawKeyboardEvent::press(
                                KeyCode::Tab,
                                KeyModifiers::NONE,
                                1,
                            ))],
                        );
                    }
                    let original = node(&doc, "control");
                    assert_eq!(
                        session.interaction().focused,
                        Some(original),
                        "initial: policy={policy:?}, authored={authored}"
                    );
                    if rebuild {
                        doc = describe(true);
                        assert_ne!(original, node(&doc, "control"));
                    }
                    prepare(&mut session, &mut doc);
                    let target = node(&doc, "control");
                    assert_eq!(session.interaction().focused, Some(target), "policy={policy:?}, accessibility={accessibility_focus}, authored={authored}, rebuild={rebuild}");
                    let expected_hit = match policy {
                        HitTestBehavior::Auto => HitTestResult::Target(target),
                        HitTestBehavior::PassThrough => {
                            HitTestResult::Target(node(&doc, "background"))
                        }
                        HitTestBehavior::Block => HitTestResult::Blocked(target),
                    };
                    assert_eq!(
                        doc.hit_test_result(UiPoint::new(10.0, 10.0)),
                        Some(expected_hit)
                    );
                    let output = frame(
                        &mut session,
                        &mut doc,
                        vec![RawInputEvent::Keyboard(RawKeyboardEvent::press(
                            KeyCode::Enter,
                            KeyModifiers::NONE,
                            2,
                        ))],
                    );
                    let actions = collect_document_widget_actions(&output);
                    assert!(
                        matches!(actions.as_slice(), [action] if action.target == target && matches!(action.kind, WidgetActionKind::Activate(_))),
                        "keyboard activation: {actions:?}"
                    );

                    doc.set_node_enabled(target, false);
                    prepare(&mut session, &mut doc);
                    assert_eq!(session.interaction().focused, None);
                    let output = frame(
                        &mut session,
                        &mut doc,
                        vec![RawInputEvent::Keyboard(RawKeyboardEvent::press(
                            KeyCode::Enter,
                            KeyModifiers::NONE,
                            3,
                        ))],
                    );
                    assert!(collect_document_widget_actions(&output).is_empty());
                }
            }
        }
    }
}

#[cfg(feature = "widgets")]
#[test]
fn pointer_hit_policy_changes_preserve_text_composition_across_rebuilds() {
    use crate::host::collect_document_widget_actions;
    use crate::input::RawTextCompositionEvent;
    use crate::widgets::{text_input, TextInputOptions, TextInputState};
    use crate::{HitTestBehavior, TextCompositionEvent};

    for policy in [HitTestBehavior::PassThrough, HitTestBehavior::Block] {
        let options = TextInputOptions::default().with_edit_action("edit");
        let mut text = TextInputState::new("");
        let describe = |text: &TextInputState, policy, inserted| {
            let mut doc = UiDocument::new(LayoutStyle::column().with_size(400.0, 300.0));
            let root = doc.root();
            if inserted {
                doc.add_child(
                    root,
                    UiNode::container("padding", LayoutStyle::size(10.0, 10.0)),
                );
            }
            let editor = text_input(&mut doc, root, "editor", text, options.clone());
            doc.node_mut(editor).set_hit_test_behavior(policy);
            doc
        };
        let mut session = RuntimeSession::new();
        let mut doc = describe(&text, HitTestBehavior::Auto, false);
        doc.set_focus_state(UiFocusState {
            focused: Some(node(&doc, "editor")),
            ..Default::default()
        });
        prepare(&mut session, &mut doc);
        frame(&mut session, &mut doc, Vec::new());
        let input = session
            .interaction()
            .text_ime
            .as_ref()
            .unwrap()
            .input
            .clone();
        let output = frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Composition(RawTextCompositionEvent {
                input: input.clone(),
                event: TextCompositionEvent::Preedit {
                    text: "候補".into(),
                    selection: None,
                    replacement: None,
                },
                timestamp_millis: 1,
            })],
        );
        for action in collect_document_widget_actions(&output) {
            if let WidgetActionKind::TextEdit(edit) = action.kind {
                text.apply_widget_text_edit(&edit, &options);
            }
        }
        let original = node(&doc, "editor");
        doc = describe(&text, policy, true);
        prepare(&mut session, &mut doc);
        let editor = node(&doc, "editor");
        assert_ne!(editor, original);
        assert_eq!(
            session.interaction().focused,
            Some(editor),
            "policy={policy:?}"
        );
        assert!(session.take_interaction_cancellations().is_empty());
        let output = frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Composition(RawTextCompositionEvent {
                input: input.clone(),
                event: TextCompositionEvent::Commit {
                    text: "候補".into(),
                    replacement: None,
                },
                timestamp_millis: 2,
            })],
        );
        assert_eq!(
            session.interaction().text_ime.as_ref().unwrap().input,
            input
        );
        let actions = collect_document_widget_actions(&output);
        let [WidgetAction {
            target,
            kind: WidgetActionKind::TextEdit(edit),
            ..
        }] = actions.as_slice()
        else {
            panic!("expected one commit without focus loss or cancellation: {actions:?}");
        };
        assert_eq!(*target, editor);
        assert!(matches!(
            edit.event,
            UiInputEvent::Composition {
                event: TextCompositionEvent::Commit { .. },
                ..
            }
        ));
        text.apply_widget_text_edit(edit, &options);
        assert_eq!(text.text(), "候補");
    }
}

#[test]
fn distant_release_does_not_activate_or_increment_click_count() {
    use crate::host::collect_document_widget_actions;
    for rebuild in [false, true] {
        for partition in 0..4 {
            let mut session = RuntimeSession::new();
            let mut doc = document(&["control"]);
            let control = node(&doc, "control");
            doc.node_mut(control).set_action("activate");
            prepare(&mut session, &mut doc);
            let events = [
                (
                    PointerEventKind::Down(PointerButton::Primary),
                    UiPoint::new(5.0, 5.0),
                ),
                (PointerEventKind::Move, UiPoint::new(6.0, 5.0)),
                (
                    PointerEventKind::Up(PointerButton::Primary),
                    UiPoint::new(80.0, 20.0),
                ),
            ];
            let mut batch = Vec::new();
            let mut rebuilt_frames = 0;
            for (index, (kind, point)) in events.into_iter().enumerate() {
                batch.push(RawInputEvent::Pointer(RawPointerEvent::new(
                    kind,
                    point,
                    index as u64,
                )));
                if index != 2 && partition & (1 << index) == 0 {
                    continue;
                }
                if rebuild {
                    let padded = rebuilt_frames % 2 != 0;
                    rebuilt_frames += 1;
                    doc = document(if padded {
                        &["padding", "control"]
                    } else {
                        &["control"]
                    });
                    // Keep the same geometry while changing the retained node's index.
                    if padded {
                        let padding = node(&doc, "padding");
                        doc.set_node_style(padding, LayoutStyle::size(0.0, 0.0));
                    }
                    let control = node(&doc, "control");
                    doc.node_mut(control).set_action("activate");
                }
                prepare(&mut session, &mut doc);
                let output = frame(&mut session, &mut doc, std::mem::take(&mut batch));
                assert!(
                    collect_document_widget_actions(&output).is_empty(),
                    "distant release activated a control: rebuild={rebuild}, partition={partition}"
                );
            }
            assert!(session.interaction().pressed.is_none());
            assert!(session.interaction().drag_capture.is_none());
            assert!(session
                .interaction()
                .gesture_tracker
                .active_capture(PointerId::MOUSE)
                .is_none());

            let output = frame(
                &mut session,
                &mut doc,
                vec![
                    RawInputEvent::Pointer(RawPointerEvent::new(
                        PointerEventKind::Down(PointerButton::Primary),
                        UiPoint::new(80.0, 20.0),
                        4,
                    )),
                    RawInputEvent::Pointer(RawPointerEvent::new(
                        PointerEventKind::Up(PointerButton::Primary),
                        UiPoint::new(80.0, 20.0),
                        5,
                    )),
                ],
            );
            let actions = collect_document_widget_actions(&output);
            assert_eq!(actions.len(), 1);
            assert_eq!(actions[0].target, node(&doc, "control"));
            let WidgetActionKind::Activate(activation) = actions[0].kind else {
                panic!("normal click did not activate: {actions:?}");
            };
            assert_eq!(activation.count, 1);
        }
    }
}

#[cfg(feature = "widgets")]
#[test]
fn confirmed_clicks_agree_with_input_results_manual_helpers_and_animations() {
    use crate::host::collect_document_widget_actions;
    use crate::widgets::{button, ButtonOptions};
    use crate::{
        AnimatedValues, AnimationCondition, AnimationState, AnimationTransition,
        ANIMATION_INPUT_ACTIVATED,
    };

    let values = AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0);
    let options = ButtonOptions {
        layout: LayoutStyle::size(120.0, 40.0),
        animation: Some(
            AnimationMachine::new(
                vec![
                    AnimationState::new("idle", values),
                    AnimationState::new("activated", values),
                ],
                vec![AnimationTransition::when(
                    "idle",
                    "activated",
                    AnimationCondition::trigger(ANIMATION_INPUT_ACTIVATED),
                    0.0,
                )],
                "idle",
            )
            .unwrap(),
        ),
        ..ButtonOptions::default().with_action("activate")
    };
    let describe = |padded| {
        let mut doc = UiDocument::new(LayoutStyle::column().with_size(400.0, 300.0));
        let root = doc.root();
        if padded {
            doc.add_child(
                root,
                UiNode::container("padding", LayoutStyle::size(0.0, 0.0)),
            );
        }
        button(&mut doc, root, "control", "Control", options.clone());
        doc
    };
    for (motion, release, clicks) in [
        (UiPoint::new(6.0, 5.0), UiPoint::new(6.0, 5.0), 1),
        (UiPoint::new(6.0, 5.0), UiPoint::new(80.0, 20.0), 0),
        (UiPoint::new(40.0, 10.0), UiPoint::new(5.0, 5.0), 0),
    ] {
        for rebuild in [false, true] {
            for partition in 0..4 {
                let mut session = RuntimeSession::new();
                let mut doc = describe(false);
                prepare(&mut session, &mut doc);
                let events = [
                    (
                        PointerEventKind::Down(PointerButton::Primary),
                        UiPoint::new(5.0, 5.0),
                    ),
                    (PointerEventKind::Move, motion),
                    (PointerEventKind::Up(PointerButton::Primary), release),
                ];
                let mut batch = Vec::new();
                let mut frames = 0;
                let mut automatic = 0;
                let mut manual = 0;
                let mut reported = 0;
                for (index, (kind, position)) in events.into_iter().enumerate() {
                    batch.push(RawInputEvent::Pointer(RawPointerEvent::new(
                        kind,
                        position,
                        index as u64,
                    )));
                    if index != 2 && partition & (1 << index) == 0 {
                        continue;
                    }
                    if rebuild {
                        doc = describe(frames % 2 != 0);
                    }
                    frames += 1;
                    prepare(&mut session, &mut doc);
                    let control = node(&doc, "control");
                    let output = frame(&mut session, &mut doc, std::mem::take(&mut batch));
                    automatic += collect_document_widget_actions(&output)
                        .iter()
                        .filter(|action| matches!(action.kind, WidgetActionKind::Activate(_)))
                        .count();
                    for result in output.input_results() {
                        reported += usize::from(result.clicked == Some(control));
                        manual += crate::widgets::button::button_actions_from_input_result(
                            &doc, control, &options, result,
                        )
                        .len();
                    }
                }
                assert_eq!(automatic, clicks);
                assert_eq!((reported, manual), (clicks, clicks), "motion={motion:?}, release={release:?}, rebuild={rebuild}, partition={partition}");
                let animation = doc.node(node(&doc, "control")).animation.as_ref().unwrap();
                assert_eq!(
                    animation.current_state_name(),
                    if clicks == 1 { "activated" } else { "idle" }
                );
            }
        }
    }
}

#[test]
fn authored_focus_and_explicit_blur_override_retention() {
    let mut session = RuntimeSession::new();
    let mut first = document(&["a", "b"]);
    prepare(&mut session, &mut first);
    press(&mut session, &mut first, "a");
    let mut next = document(&["a", "b"]);
    next.set_focus_state(UiFocusState {
        focused: Some(node(&next, "b")),
        ..Default::default()
    });
    prepare(&mut session, &mut next);
    assert_eq!(session.interaction().focused, Some(node(&next, "b")));
    frame(&mut session, &mut next, Vec::new());
    let mut blurred = document(&["a", "b"]);
    blurred.set_focus_state(UiFocusState::default());
    prepare(&mut session, &mut blurred);
    assert_eq!(session.interaction().focused, None);
    frame(&mut session, &mut blurred, Vec::new());
    assert_eq!(session.interaction().focused, None);
}

#[test]
fn duplicate_names_never_choose_an_arbitrary_state_owner() {
    let mut session = RuntimeSession::new();
    let mut first = document(&["a"]);
    prepare(&mut session, &mut first);
    press(&mut session, &mut first, "a");
    let mut next = document(&["a", "a"]);
    prepare(&mut session, &mut next);
    assert_eq!(session.interaction().focused, None);
    assert_eq!(session.interaction().pressed, None);
    frame(&mut session, &mut next, Vec::new());
    let mut unique = document(&["a"]);
    prepare(&mut session, &mut unique);
    assert_eq!(session.interaction().focused, None);
}

#[test]
fn path_segments_and_ambiguous_ancestors_are_respected() {
    let mut doc = document(&["a/b", "a"]);
    let parent = node(&doc, "a");
    let child = doc.add_child(
        parent,
        UiNode::container("b", LayoutStyle::size(10.0, 10.0)),
    );
    let identities = NodeIdentityIndex::from_document(&doc);
    assert_ne!(
        identities.by_node[child.index()],
        identities.by_node[node(&doc, "a/b").index()]
    );
    doc.add_child(
        doc.root(),
        UiNode::container("a", LayoutStyle::size(10.0, 10.0)),
    );
    let identities = NodeIdentityIndex::from_document(&doc);
    assert!(identities.by_node[child.index()].is_none());
}

fn scroll_document() -> UiDocument {
    let mut doc = UiDocument::new(LayoutStyle::column().with_size(100.0, 80.0));
    let scroll = doc.add_child(
        doc.root(),
        UiNode::container("scroll", LayoutStyle::column().with_size(100.0, 80.0))
            .with_scroll(ScrollAxes::VERTICAL),
    );
    doc.add_child(
        scroll,
        UiNode::container(
            "content",
            LayoutStyle::size(100.0, 240.0).with_flex_shrink(0.0),
        ),
    );
    doc
}

#[cfg(feature = "widgets")]
#[test]
fn wheel_over_checkbox_scrolls_without_activating_it() {
    use crate::host::collect_document_widget_actions;
    use crate::input::RawWheelEvent;
    use crate::widgets::{checkbox, CheckboxOptions};

    for checked in [false, true] {
        for part in ["check.box", "check.label"] {
            for (scrollable, delta) in [(false, 8.0), (true, 8.0), (true, -8.0)] {
                for lines in [false, true] {
                    let mut session = RuntimeSession::new();
                    let mut doc = UiDocument::new(LayoutStyle::column().with_size(300.0, 120.0));
                    let mut container = UiNode::container(
                        "container",
                        LayoutStyle::column().with_size(300.0, 120.0),
                    );
                    if scrollable {
                        container = container.with_scroll(ScrollAxes::VERTICAL);
                    }
                    let container = doc.add_child(doc.root(), container);
                    let check = checkbox(
                        &mut doc,
                        container,
                        "check",
                        "Enable option",
                        checked,
                        CheckboxOptions {
                            layout: LayoutStyle::size(240.0, 40.0).with_flex_shrink(0.0),
                            ..CheckboxOptions::default().with_action("check.toggle")
                        },
                    );
                    doc.add_child(
                        container,
                        UiNode::container(
                            "content",
                            LayoutStyle::size(300.0, 400.0).with_flex_shrink(0.0),
                        ),
                    );
                    prepare(&mut session, &mut doc);
                    let rect = doc.node(node(&doc, part)).layout().rect;
                    let point = UiPoint::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
                    let wheel = if lines {
                        RawWheelEvent::lines(point, UiPoint::new(0.0, delta / 16.0), 1)
                    } else {
                        RawWheelEvent::pixels(point, UiPoint::new(0.0, delta), 1)
                    };
                    let output = frame(&mut session, &mut doc, vec![RawInputEvent::Wheel(wheel)]);
                    let actions = collect_document_widget_actions(&output);
                    assert!(actions.is_empty(), "wheel activated {part}: {actions:#?}");
                    assert!(matches!(
                        output.host_output.gestures().cloned().collect::<Vec<_>>().as_slice(),
                        [GestureEvent::WheelTargeted { target: Some(target), event }]
                            if *target == check && *event == wheel
                    ));
                    let offset = doc
                        .scroll_state(container)
                        .map_or(0.0, |scroll| scroll.offset.y);
                    assert_eq!(
                        offset > 0.0,
                        scrollable && delta > 0.0,
                        "{part}: scrollable={scrollable}, delta={delta}, lines={lines}"
                    );

                    // A wheel event must not interfere with a subsequent normal click.
                    let rect = doc.node(check).layout().rect;
                    let point = UiPoint::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
                    let output = frame(
                        &mut session,
                        &mut doc,
                        vec![
                            RawInputEvent::Pointer(RawPointerEvent::new(
                                PointerEventKind::Down(PointerButton::Primary),
                                point,
                                2,
                            )),
                            RawInputEvent::Pointer(RawPointerEvent::new(
                                PointerEventKind::Up(PointerButton::Primary),
                                point,
                                3,
                            )),
                        ],
                    );
                    let actions = collect_document_widget_actions(&output);
                    assert!(matches!(actions.as_slice(), [action]
                        if action.target == check && matches!(action.kind, WidgetActionKind::Activate(_))));
                }
            }
        }
    }
}

#[test]
fn scrolling_survives_rebuilds_but_authored_offsets_take_precedence() {
    let mut session = RuntimeSession::new();
    let mut first = scroll_document();
    prepare(&mut session, &mut first);
    first.set_scroll_offset(node(&first, "scroll"), UiPoint::new(0.0, 120.0));
    frame(&mut session, &mut first, Vec::new());
    let mut next = scroll_document();
    prepare(&mut session, &mut next);
    assert_eq!(
        next.scroll_state(node(&next, "scroll")).unwrap().offset.y,
        120.0
    );
    assert_eq!(next.node(node(&next, "content")).layout().rect.y, -120.0);
    let mut authored = scroll_document();
    let id = node(&authored, "scroll");
    authored
        .node_mut(id)
        .scroll
        .as_mut()
        .unwrap()
        .set_offset(UiPoint::new(0.0, 40.0));
    prepare(&mut session, &mut authored);
    assert_eq!(authored.scroll_state(id).unwrap().offset.y, 40.0);
    assert_eq!(
        authored.node(node(&authored, "content")).layout().rect.y,
        -40.0
    );
}

#[test]
fn automatic_scrollbar_drag_survives_document_preparation() {
    for rebuilt in [false, true] {
        let mut session = RuntimeSession::new();
        let mut doc = scroll_document();
        prepare(&mut session, &mut doc);
        let old_id = node(&doc, "scroll");
        frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Down(PointerButton::Primary),
                UiPoint::new(99.0, 12.0),
                1,
            ))],
        );
        let before = doc.scroll_state(old_id).unwrap().offset.y;
        if rebuilt {
            // Insert a sibling before the scroller to change every retained ID.
            let mut reordered = UiDocument::new(LayoutStyle::column().with_size(100.0, 80.0));
            reordered.add_child(
                reordered.root(),
                UiNode::container("new", LayoutStyle::size(0.0, 0.0)),
            );
            let scroll = reordered.add_child(
                reordered.root(),
                UiNode::container("scroll", LayoutStyle::column().with_size(100.0, 80.0))
                    .with_scroll(ScrollAxes::VERTICAL),
            );
            reordered.add_child(
                scroll,
                UiNode::container(
                    "content",
                    LayoutStyle::size(100.0, 240.0).with_flex_shrink(0.0),
                ),
            );
            doc = reordered;
            assert_ne!(node(&doc, "scroll"), old_id);
        }
        prepare(&mut session, &mut doc);
        frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Move,
                UiPoint::new(99.0, 65.0),
                2,
            ))],
        );
        assert!(
            doc.scroll_state(node(&doc, "scroll")).unwrap().offset.y > before,
            "scrollbar drag was lost during preparation: rebuilt={rebuilt}"
        );
        frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Up(PointerButton::Primary),
                UiPoint::new(99.0, 65.0),
                3,
            ))],
        );
        assert!(session.interaction().drag_capture.is_none());
        assert!(doc.auto_scrollbar_drag.is_none());
    }
}

#[test]
fn automatic_scrollbar_actions_are_independent_of_frame_boundaries() {
    let points = [
        (
            PointerEventKind::Down(PointerButton::Primary),
            UiPoint::new(40.0, 12.0),
        ),
        (
            PointerEventKind::Up(PointerButton::Primary),
            UiPoint::new(40.0, 12.0),
        ),
        (
            PointerEventKind::Down(PointerButton::Primary),
            UiPoint::new(99.0, 12.0),
        ),
        (PointerEventKind::Move, UiPoint::new(99.0, 65.0)),
        (
            PointerEventKind::Up(PointerButton::Primary),
            UiPoint::new(99.0, 65.0),
        ),
    ];
    // The content click must activate once; the scrollbar drag must only scroll,
    // regardless of which events share a frame.
    for partition in 0..1 << (points.len() - 1) {
        let mut session = RuntimeSession::new();
        let mut doc = scroll_document();
        let scroll = node(&doc, "scroll");
        doc.node_mut(scroll).input = InputBehavior::BUTTON;
        doc.node_mut(scroll).action = Some("scroll.changed".into());
        let mut actions = Vec::new();
        let mut events = Vec::new();
        for (index, (kind, point)) in points.iter().copied().enumerate() {
            events.push(RawInputEvent::Pointer(RawPointerEvent::new(
                kind,
                point,
                index as u64,
            )));
            if index == points.len() - 1 || partition & (1 << index) != 0 {
                prepare(&mut session, &mut doc);
                let output = frame(&mut session, &mut doc, std::mem::take(&mut events));
                actions.extend(crate::host::collect_document_widget_actions(&output));
            }
        }
        assert!(
            matches!(
                actions.first().map(|action| &action.kind),
                Some(WidgetActionKind::Activate(_))
            ),
            "partition={partition}: {actions:?}"
        );
        assert!(
            actions[1..]
                .iter()
                .all(|action| matches!(action.kind, WidgetActionKind::Scroll(_))),
            "partition={partition}: {actions:?}"
        );
        assert!(actions
            .iter()
            .any(|action| matches!(action.kind, WidgetActionKind::Scroll(_))));
        assert!(doc.scroll_state(scroll).unwrap().offset.y > 0.0);
        assert!(doc.auto_scrollbar_drag.is_none());
        assert!(session.widget_edits.is_empty());
    }
}

#[test]
fn automatic_scrollbar_click_does_not_activate_the_container_binding() {
    let mut session = RuntimeSession::new();
    let mut doc = scroll_document();
    let scroll = node(&doc, "scroll");
    doc.node_mut(scroll).input = InputBehavior::BUTTON;
    doc.node_mut(scroll).action = Some("scroll.changed".into());
    prepare(&mut session, &mut doc);
    let output = frame(
        &mut session,
        &mut doc,
        vec![
            RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Down(PointerButton::Primary),
                UiPoint::new(99.0, 12.0),
                1,
            )),
            RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Up(PointerButton::Primary),
                UiPoint::new(99.0, 12.0),
                2,
            )),
        ],
    );
    let actions = crate::host::collect_document_widget_actions(&output);
    assert!(
        actions
            .iter()
            .all(|action| matches!(action.kind, WidgetActionKind::Scroll(_))),
        "{actions:?}"
    );
}

#[test]
fn invalid_automatic_scrollbar_capture_cannot_turn_into_a_widget_click() {
    for change in [
        "overflow", "axis", "disabled", "hidden", "blocked", "removed", "replaced", "modal",
    ] {
        let mut session = RuntimeSession::new();
        let mut doc = scroll_document();
        prepare(&mut session, &mut doc);
        frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Down(PointerButton::Primary),
                UiPoint::new(99.0, 12.0),
                1,
            ))],
        );
        doc = scroll_document();
        let scroll = node(&doc, "scroll");
        doc.node_mut(scroll).input = InputBehavior::BUTTON;
        doc.node_mut(scroll).action = Some("scroll.changed".into());
        match change {
            "modal" => {
                add_modal_barrier(&mut doc);
            }
            "overflow" => {
                let content = node(&doc, "content");
                doc.node_mut(content).style.layout = LayoutStyle::size(50.0, 20.0).style;
            }
            "axis" => doc.node_mut(scroll).scroll.as_mut().unwrap().axes = ScrollAxes::HORIZONTAL,
            "disabled" => doc.set_node_enabled(scroll, false),
            "hidden" => doc.node_mut(scroll).style.layout.display = taffy::Display::None,
            "blocked" => doc.node_mut(scroll).hit_test_behavior = crate::HitTestBehavior::Block,
            "removed" => doc.node_mut(scroll).scroll = None,
            "replaced" => doc.node_mut(scroll).name = "replacement".into(),
            _ => unreachable!(),
        }
        prepare(&mut session, &mut doc);
        if change == "modal" {
            assert!(
                doc.auto_scrollbar_drag.is_none(),
                "modal must end the existing drag before release"
            );
            assert!(session.interaction().drag_capture.is_none());
        }
        let output = frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Up(PointerButton::Primary),
                UiPoint::new(99.0, 12.0),
                2,
            ))],
        );
        assert!(doc.auto_scrollbar_drag.is_none(), "{change}");
        assert!(session.interaction().drag_capture.is_none(), "{change}");
        assert!(
            crate::host::collect_document_widget_actions(&output).is_empty(),
            "{change}"
        );
    }
}

#[test]
fn inactive_canvas_capture_releases_pointer_lock_without_reacquiring_it() {
    use crate::platform::{CursorGrabMode, CursorRequest};
    use crate::{CanvasContent, CanvasInteractionPolicy, UiContent};
    for cause in ["modal", "disabled"] {
        let mut session = RuntimeSession::new();
        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
        let canvas = doc.add_child(
            doc.root(),
            UiNode::canvas("canvas", "canvas", LayoutStyle::size(100.0, 80.0)),
        );
        doc.set_node_content(
            canvas,
            UiContent::Canvas(
                CanvasContent::new("canvas").interaction(CanvasInteractionPolicy::NATIVE_VIEWPORT),
            ),
        );
        let modal = add_modal_barrier(&mut doc);
        let mut was_active = false;
        for active in [true, false, false, true, true, false] {
            doc.node_mut(modal).style.layout.display = if cause == "modal" && !active {
                taffy::Display::Flex
            } else {
                taffy::Display::None
            };
            doc.set_node_enabled(canvas, cause != "disabled" || active);
            prepare(&mut session, &mut doc);
            let output = frame(&mut session, &mut doc, Vec::new());
            assert_eq!(
                session
                    .interaction()
                    .canvas_host_capture
                    .active_plans()
                    .len(),
                usize::from(active),
                "{cause}"
            );
            assert_eq!(
                output.render_request.canvas_host_capture_plans().len(),
                usize::from(active)
            );
            assert_eq!(
                output.render_request.canvas_requests().len(),
                1,
                "the background canvas still renders"
            );
            let requests = output
                .platform_requests()
                .into_iter()
                .filter(|request| matches!(request, PlatformRequest::Cursor(_)))
                .collect::<Vec<_>>();
            let expected = if active == was_active {
                Vec::new()
            } else {
                vec![
                    PlatformRequest::Cursor(CursorRequest::SetGrab(if active {
                        CursorGrabMode::Locked
                    } else {
                        CursorGrabMode::None
                    })),
                    PlatformRequest::Cursor(CursorRequest::SetVisible(!active)),
                ]
            };
            assert_eq!(
                requests, expected,
                "{cause}: active={active}, previous={was_active}"
            );
            let UiContent::Canvas(authored) = doc.node(canvas).content() else {
                panic!("canvas")
            };
            assert_eq!(
                authored.interaction,
                CanvasInteractionPolicy::NATIVE_VIEWPORT
            );
            was_active = active;
        }
    }
}

#[test]
fn canvas_pointer_lock_survives_removal_modal_and_owner_handoffs() {
    use crate::platform::{CursorGrabMode, CursorRequest};
    use crate::{CanvasContent, CanvasInteractionPolicy, UiContent};
    let mut session = RuntimeSession::new();
    let mut was_locked = false;
    // 0 = absent, 1 = editor capture, 2 = pointer-locked viewport.
    for (a, b, modal) in [
        (2, 2, false),
        (0, 2, false),
        (2, 2, false),
        (2, 2, true),
        (2, 2, false),
        (2, 1, false),
        (0, 1, false),
        (2, 1, false),
        (0, 2, false),
        (0, 0, false),
        (0, 0, false),
    ] {
        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
        for (name, mode) in [("a", a), ("b", b)] {
            if mode == 0 {
                continue;
            }
            let parent = if name == "b" {
                let parent = add_modal_barrier(&mut doc);
                doc.node_mut(parent).accessibility_mut().unwrap().modal = modal;
                parent
            } else {
                doc.root()
            };
            let canvas = doc.add_child(
                parent,
                UiNode::canvas(name, name, LayoutStyle::size(40.0, 30.0)),
            );
            doc.set_node_content(
                canvas,
                UiContent::Canvas(CanvasContent::new(name).interaction(if mode == 2 {
                    CanvasInteractionPolicy::NATIVE_VIEWPORT
                } else {
                    CanvasInteractionPolicy::EDITOR
                })),
            );
        }
        prepare(&mut session, &mut doc);
        let output = frame(&mut session, &mut doc, Vec::new());
        let locked = (a == 2 && !modal) || b == 2;
        let requests = output
            .platform_requests()
            .into_iter()
            .filter(|request| matches!(request, PlatformRequest::Cursor(_)))
            .collect::<Vec<_>>();
        let expected = if was_locked == locked {
            Vec::new()
        } else {
            vec![
                PlatformRequest::Cursor(CursorRequest::SetGrab(if locked {
                    CursorGrabMode::Locked
                } else {
                    CursorGrabMode::None
                })),
                PlatformRequest::Cursor(CursorRequest::SetVisible(!locked)),
            ]
        };
        assert_eq!(requests, expected, "a={a}, b={b}, modal={modal}");
        assert_eq!(
            session
                .interaction()
                .canvas_host_capture
                .active_plans()
                .len(),
            usize::from(a != 0 && !modal) + usize::from(b != 0)
        );
        was_locked = locked;
    }
}

#[test]
fn independent_sessions_do_not_share_state() {
    let mut first_session = RuntimeSession::new();
    let mut first = document(&["a", "b"]);
    prepare(&mut first_session, &mut first);
    press(&mut first_session, &mut first, "a");
    let mut other_session = RuntimeSession::new();
    let mut other = document(&["a", "b"]);
    prepare(&mut other_session, &mut other);
    assert_eq!(other_session.interaction().focused, None);
    assert_eq!(first_session.interaction().focused, Some(node(&first, "a")));
    press(&mut other_session, &mut other, "b");
    for names in [["b", "a"], ["a", "b"]] {
        first = document(&names);
        prepare(&mut first_session, &mut first);
        assert_eq!(first_session.interaction().focused, Some(node(&first, "a")));
        assert_eq!(first_session.interaction().pressed, Some(node(&first, "a")));
        assert_eq!(other_session.interaction().focused, Some(node(&other, "b")));
        assert_eq!(other_session.interaction().pressed, Some(node(&other, "b")));
        other = document(&names);
        prepare(&mut other_session, &mut other);
        assert_eq!(other_session.interaction().focused, Some(node(&other, "b")));
        assert_eq!(first_session.interaction().focused, Some(node(&first, "a")));
    }
    other = document(&["a"]);
    prepare(&mut other_session, &mut other);
    assert_eq!(other_session.interaction().focused, None);
    assert_eq!(first_session.interaction().focused, Some(node(&first, "a")));
    assert_eq!(first_session.interaction().pressed, Some(node(&first, "a")));
}

#[test]
fn disabled_controls_cancel_focus_and_drag_ownership() {
    let mut session = RuntimeSession::new();
    let mut first = document(&["a"]);
    prepare(&mut session, &mut first);
    press(&mut session, &mut first, "a");
    let mut next = document(&["a"]);
    let id = node(&next, "a");
    next.set_node_input(id, InputBehavior::NONE);
    prepare(&mut session, &mut next);
    assert_eq!(session.interaction().focused, None);
    assert_eq!(session.interaction().pressed, None);
    assert_eq!(session.interaction().drag_capture, None);
    assert!(session
        .interaction()
        .gesture_tracker
        .active_capture(PointerId::MOUSE)
        .is_none());
}

#[test]
fn text_ime_rebinds_local_ids_and_releases_removed_targets() {
    use crate::platform::{LogicalRect, TextImeSession};
    for custom_id in [false, true] {
        let mut session = RuntimeSession::new();
        let mut first = document(&["text"]);
        prepare(&mut session, &mut first);
        press(&mut session, &mut first, "text");
        let old = node(&first, "text");
        let input = if custom_id {
            crate::platform::TextInputId::new("editor")
        } else {
            text_input_id_for_node(old)
        };
        session.frame.interaction.activate_text_ime_for(
            old,
            TextImeSession::new(input.clone(), LogicalRect::new(0.0, 0.0, 1.0, 20.0)),
        );
        let mut next = document(&["inserted", "text"]);
        prepare(&mut session, &mut next);
        let new = node(&next, "text");
        let expected = if custom_id {
            input.clone()
        } else {
            text_input_id_for_node(new)
        };
        assert_eq!(session.interaction().text_target, Some(new));
        assert_eq!(
            session.interaction().text_ime.as_ref().unwrap().input,
            expected
        );
        let result = frame(&mut session, &mut next, Vec::new());
        if !custom_id {
            assert!(result
                .platform_requests()
                .contains(&PlatformRequest::TextIme(TextImeRequest::Deactivate {
                    input
                })));
        }
        let mut removed = document(&["replacement"]);
        prepare(&mut session, &mut removed);
        assert!(session.interaction().text_ime.is_none());
        assert!(session.interaction().text_target.is_none());
        let result = frame(&mut session, &mut removed, Vec::new());
        assert!(result
            .platform_requests()
            .contains(&PlatformRequest::TextIme(TextImeRequest::Deactivate {
                input: expected
            })));
    }
}

#[test]
fn canvas_capture_survives_reordering_and_releases_on_removal() {
    use crate::platform::{CursorGrabMode, CursorRequest};
    use crate::{CanvasContent, CanvasInteractionPolicy, UiContent};
    let canvas_doc = |insert: bool| {
        let mut doc = document(if insert { &["extra"] } else { &[] });
        let id = doc.add_child(
            doc.root(),
            UiNode::canvas("canvas", "viewport", LayoutStyle::size(100.0, 100.0)),
        );
        doc.set_node_content(
            id,
            UiContent::Canvas(
                CanvasContent::new("viewport")
                    .native_viewport()
                    .interaction(CanvasInteractionPolicy::NATIVE_VIEWPORT),
            ),
        );
        doc
    };
    let mut session = RuntimeSession::new();
    let mut first = canvas_doc(false);
    prepare(&mut session, &mut first);
    frame(&mut session, &mut first, Vec::new());
    assert_eq!(
        session
            .interaction()
            .canvas_host_capture
            .active_plans()
            .len(),
        1
    );
    let mut next = canvas_doc(true);
    prepare(&mut session, &mut next);
    assert_eq!(
        session.interaction().canvas_host_capture.active_plans()[0].node,
        node(&next, "canvas")
    );
    frame(&mut session, &mut next, Vec::new());
    let mut removed = document(&["replacement"]);
    prepare(&mut session, &mut removed);
    assert!(session.interaction().canvas_host_capture.is_empty());
    let result = frame(&mut session, &mut removed, Vec::new());
    assert!(result
        .platform_requests()
        .contains(&PlatformRequest::Cursor(CursorRequest::SetGrab(
            CursorGrabMode::None
        ))));
    assert!(result
        .platform_requests()
        .contains(&PlatformRequest::Cursor(CursorRequest::SetVisible(true))));
}

#[test]
fn animation_progress_survives_rebuilds_but_not_a_changed_definition() {
    use crate::{AnimatedValues, AnimationState, AnimationTransition, AnimationTrigger};
    let animated = |endpoint: f32| {
        let mut doc = document(&["a"]);
        let machine = AnimationMachine::new(
            vec![
                AnimationState::new(
                    "start",
                    AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0),
                ),
                AnimationState::new(
                    "end",
                    AnimatedValues::new(1.0, UiPoint::new(endpoint, 0.0), 1.0),
                ),
            ],
            vec![AnimationTransition::new(
                "start",
                "end",
                AnimationTrigger::Custom("go".into()),
                1.0,
            )],
            "start",
        )
        .unwrap();
        let id = node(&doc, "a");
        doc.node_mut(id).animation = Some(machine);
        doc
    };
    let mut session = RuntimeSession::new();
    let mut first = animated(100.0);
    prepare(&mut session, &mut first);
    first.trigger_animation(node(&first, "a"), AnimationTrigger::Custom("go".into()));
    first.tick_animations(0.25);
    let expected = first
        .node(node(&first, "a"))
        .animation
        .as_ref()
        .unwrap()
        .values();
    frame(&mut session, &mut first, Vec::new());
    assert_eq!(
        session.next_frame_delay(Duration::ZERO),
        Some(Duration::ZERO)
    );
    let mut next = animated(100.0);
    prepare(&mut session, &mut next);
    assert_eq!(
        next.node(node(&next, "a"))
            .animation
            .as_ref()
            .unwrap()
            .values(),
        expected
    );
    next.tick_animations(1.0);
    frame(&mut session, &mut next, Vec::new());
    assert_eq!(session.next_frame_delay(Duration::ZERO), None);
    let mut changed = animated(200.0);
    prepare(&mut session, &mut changed);
    assert_eq!(
        changed
            .node(node(&changed, "a"))
            .animation
            .as_ref()
            .unwrap()
            .current_state_name(),
        "start"
    );
}

#[derive(Default)]
struct CountingMeasurer(usize);

impl TextMeasurer for CountingMeasurer {
    fn measure(
        &mut self,
        text: &crate::TextContent,
        known: crate::KnownSize,
        available: crate::AvailableSize,
    ) -> UiSize {
        self.0 += 1;
        ApproxTextMeasurer.measure(text, known, available)
    }
}

fn measured_document(viewport: UiSize, text: &str) -> UiDocument {
    let mut doc = UiDocument::new(LayoutStyle::column().with_size(viewport.width, viewport.height));
    doc.add_child(
        doc.root(),
        UiNode::text(
            "label",
            text,
            crate::TextStyle::default(),
            LayoutStyle::default(),
        ),
    );
    doc
}

#[test]
fn unchanged_frames_reuse_view_and_layout_while_invalidations_rebuild() {
    let mut session = RuntimeSession::new();
    let mut measurer = CountingMeasurer::default();
    let mut builds = 0;
    let mut doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut measurer,
            |viewport, _views| {
                builds += 1;
                measured_document(viewport, "short")
            },
        )
        .unwrap();
    let measured = measurer.0;
    assert!(measured > 0);
    let first_width = doc
        .node(node(&doc, "label"))
        .layout()
        .content_size
        .unwrap()
        .width;
    frame(&mut session, &mut doc, Vec::new());
    session.retain_document(doc);
    for _ in 0..4 {
        let mut doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                Some(UiPoint::new(200.0, 200.0)),
                &mut measurer,
                |_, _views| panic!("unchanged view rebuilt"),
            )
            .unwrap();
        let input = session
            .process_input(&mut doc, VIEWPORT, Vec::new(), Vec::new(), &mut measurer)
            .unwrap();
        session
            .finish_frame(
                &mut doc,
                VIEWPORT,
                RenderTarget::window("test", VIEWPORT),
                input,
                &mut measurer,
                &mut PlatformRequestIdAllocator::default(),
            )
            .unwrap();
        session.retain_document(doc);
    }
    assert_eq!(
        measurer.0, measured,
        "unchanged frames must reuse measured layout"
    );
    session.invalidate_view();
    let doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut measurer,
            |viewport, _views| {
                builds += 1;
                measured_document(viewport, "a substantially longer label")
            },
        )
        .unwrap();
    assert_eq!(builds, 2);
    assert!(measurer.0 > measured);
    assert!(
        doc.node(node(&doc, "label"))
            .layout()
            .content_size
            .unwrap()
            .width
            > first_width
    );
    session.retain_document(doc);
    let measured = measurer.0;
    let doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::new(2.0, 1.0),
            None,
            &mut measurer,
            |viewport, _views| {
                builds += 1;
                measured_document(viewport, "a substantially longer label")
            },
        )
        .unwrap();
    assert!(
        measurer.0 > measured,
        "scale changes must invalidate layout"
    );
    session.retain_document(doc);
    session
        .build_document(
            UiSize::new(800.0, 600.0),
            UiDocumentScale::DEFAULT,
            None,
            &mut measurer,
            |viewport, _views| {
                builds += 1;
                measured_document(viewport, "resized")
            },
        )
        .unwrap();
    assert_eq!(
        builds, 4,
        "viewport and scale changes must rebuild the view"
    );
}

#[test]
fn deferred_input_preserves_retry_deadline_and_uploads_without_replaying_actions() {
    use crate::platform::{ImageHandle, PixelSize};
    use crate::renderer::ResourceUpdate;
    let mut session = RuntimeSession::new();
    session.begin_frame(Duration::ZERO);
    let mut doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_, _views| {
                let mut doc = document(&["a"]);
                let target = node(&doc, "a");
                doc.node_mut(target).set_action("activate");
                doc.add_resource_update(ResourceUpdate::rgba8_image(
                    ImageHandle::app("pixel"),
                    PixelSize::new(1, 1),
                    vec![255; 4],
                ));
                doc
            },
        )
        .unwrap();
    assert_eq!(
        frame(&mut session, &mut doc, Vec::new())
            .render_request
            .resource_updates
            .len(),
        1
    );
    session.retain_document(doc);
    session.frame_failed(Duration::ZERO);
    session.begin_frame(Duration::from_millis(5));
    let mut deferred = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_, _| panic!("deferred input should reuse document"),
        )
        .unwrap();
    let rect = deferred.node(node(&deferred, "a")).layout().rect;
    let output = frame(
        &mut session,
        &mut deferred,
        vec![
            RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Down(PointerButton::Primary),
                UiPoint::new(rect.x + 5.0, rect.y + 5.0),
                5,
            )),
            RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Up(PointerButton::Primary),
                UiPoint::new(rect.x + 5.0, rect.y + 5.0),
                6,
            )),
        ],
    );
    assert_eq!(
        crate::host::collect_document_widget_actions(&output).len(),
        1
    );
    assert_eq!(output.render_request.resource_updates.len(), 1);
    session.retain_document(deferred);
    session.frame_deferred();
    assert_eq!(
        session.next_frame_delay(Duration::from_millis(5)),
        Some(Duration::from_millis(11)),
        "input cannot prolong the retry interval"
    );
    session.begin_frame(Duration::from_millis(16));
    let mut retry = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_, _views| panic!("retry should reuse document"),
        )
        .unwrap();
    let output = frame(&mut session, &mut retry, Vec::new());
    assert_eq!(output.render_request.resource_updates.len(), 1);
    assert!(crate::host::collect_document_widget_actions(&output).is_empty());
    session.retain_document(retry);
    session.frame_presented();
    let mut next = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_, _views| panic!("redraw should reuse document"),
        )
        .unwrap();
    assert!(frame(&mut session, &mut next, Vec::new())
        .render_request
        .resource_updates
        .is_empty());
}

#[test]
fn focus_requests_are_consumed_once_even_when_the_document_is_cached() {
    let mut session = RuntimeSession::new();
    let mut doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_, _views| {
                let mut doc = document(&["a", "b"]);
                doc.set_focus_state(UiFocusState {
                    focused: Some(node(&doc, "a")),
                    ..Default::default()
                });
                doc
            },
        )
        .unwrap();
    press(&mut session, &mut doc, "b");
    session.retain_document(doc);
    let cached = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_, _views| panic!("view should be retained"),
        )
        .unwrap();
    assert_eq!(cached.focus_state().focused, Some(node(&cached, "b")));
}

#[test]
fn retained_live_regions_are_not_reannounced_when_indices_change() {
    use crate::{AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole};
    let live_document = |inserted: bool, label: &str| {
        let mut doc = document(if inserted { &["extra"] } else { &[] });
        let mut meta = AccessibilityMeta::new(AccessibilityRole::Status).label(label);
        meta.live_region = AccessibilityLiveRegion::Polite;
        doc.add_child(
            doc.root(),
            UiNode::container("status", LayoutStyle::size(100.0, 30.0)).with_accessibility(meta),
        );
        doc
    };
    let mut session = RuntimeSession::new();
    let mut first = live_document(false, "Ready");
    prepare(&mut session, &mut first);
    assert_eq!(
        frame(&mut session, &mut first, Vec::new())
            .announcements
            .pending
            .len(),
        1
    );
    let mut next = live_document(true, "Ready");
    prepare(&mut session, &mut next);
    assert!(frame(&mut session, &mut next, Vec::new())
        .announcements
        .pending
        .is_empty());
    let mut changed = live_document(true, "Finished");
    prepare(&mut session, &mut changed);
    assert_eq!(
        frame(&mut session, &mut changed, Vec::new())
            .announcements
            .pending
            .len(),
        1
    );
}

#[test]
fn custom_host_options_reach_rendering_and_accessibility() {
    use crate::accessibility::{AccessibilityAdapterRequest, AccessibilityPreferences};
    let preferences = AccessibilityPreferences::DEFAULT
        .high_contrast(true)
        .reduced_motion(true);
    let mut session = RuntimeSession::with_options(RuntimeSessionOptions {
        accessibility_capabilities: AccessibilityCapabilities::SCREEN_READER,
        layout_animation: Some(LayoutAnimationOptions {
            progress: 0.5,
            ..Default::default()
        }),
        render: RenderOptions {
            scale_factor: 1.5,
            accessibility_preferences: preferences,
            ..Default::default()
        },
    });
    let mut doc = document(&["a"]);
    prepare(&mut session, &mut doc);
    let result = frame(&mut session, &mut doc, Vec::new());
    assert_eq!(result.render_request.options.scale_factor, 1.5);
    assert_eq!(
        result.render_request.options.accessibility_preferences,
        preferences
    );
    assert!(result
        .accessibility_requests
        .iter()
        .any(|request| matches!(request, AccessibilityAdapterRequest::PublishTree { .. })));
    let mut changed = document(&["extra", "a"]);
    prepare(&mut session, &mut changed);
    let result = frame(&mut session, &mut changed, Vec::new());
    assert!(
        result.layout_animation_transitions.is_empty(),
        "reduced motion must govern layout transitions too"
    );
}

#[test]
fn hover_text_metrics_invalidate_cached_layout() {
    let mut session = RuntimeSession::new();
    let mut measurer = CountingMeasurer::default();
    let mut first = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut measurer,
            |viewport, _views| {
                let mut doc = UiDocument::new(
                    LayoutStyle::column().with_size(viewport.width, viewport.height),
                );
                let normal = crate::TextStyle::default();
                let hovered = crate::TextStyle {
                    font_size: 40.0,
                    line_height: 50.0,
                    ..normal.clone()
                };
                doc.add_child(
                    doc.root(),
                    UiNode::text("label", "Hover", normal.clone(), LayoutStyle::default())
                        .with_input(InputBehavior::BUTTON)
                        .with_interaction_text_styles(
                            crate::TextInteractionStyles::new(normal).hovered(hovered),
                        ),
                );
                doc
            },
        )
        .unwrap();
    let height = first.node(node(&first, "label")).layout().rect.height;
    frame(&mut session, &mut first, Vec::new());
    session.retain_document(first);
    let measured = measurer.0;
    let next = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            Some(UiPoint::new(5.0, 5.0)),
            &mut measurer,
            |_, _views| panic!("hover must not rebuild the view"),
        )
        .unwrap();
    assert!(measurer.0 > measured);
    assert!(next.node(node(&next, "label")).layout().rect.height > height);
}

#[cfg(feature = "widgets")]
#[test]
fn tooltips_are_frame_owned_and_do_not_accumulate_in_the_retained_document() {
    let mut session = RuntimeSession::new();
    let mut doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            Some(UiPoint::new(5.0, 5.0)),
            &mut ApproxTextMeasurer,
            |_, _views| {
                let mut doc = UiDocument::new(LayoutStyle::column().with_size(400.0, 300.0));
                doc.add_child(
                    doc.root(),
                    UiNode::container("button", LayoutStyle::size(100.0, 30.0))
                        .with_input(InputBehavior::BUTTON)
                        .with_tooltip(crate::tooltips::TooltipContent::new("Help")),
                );
                doc
            },
        )
        .unwrap();
    let authored = doc.node_count();
    frame(&mut session, &mut doc, Vec::new());
    assert!(doc.node_count() > authored);
    session.retain_document(doc);
    assert!(
        session.document().unwrap().node_count() > authored,
        "read-only inspection must retain the tooltip submitted in this frame"
    );
    for _ in 0..3 {
        let mut doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                Some(UiPoint::new(5.0, 5.0)),
                &mut ApproxTextMeasurer,
                |_, _views| panic!("tooltips should not rebuild the app"),
            )
            .unwrap();
        assert_eq!(doc.node_count(), authored);
        frame(&mut session, &mut doc, Vec::new());
        assert!(doc.node_count() > authored);
        session.retain_document(doc);
    }
    let mut doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            Some(UiPoint::new(200.0, 200.0)),
            &mut ApproxTextMeasurer,
            |_, _views| panic!("moving away should reuse the view"),
        )
        .unwrap();
    frame(&mut session, &mut doc, Vec::new());
    assert_eq!(
        doc.node_count(),
        authored,
        "inactive tooltips must disappear"
    );
}

#[cfg(feature = "widgets")]
#[test]
fn tooltip_portals_keep_their_modal_owner_across_cached_frames() {
    let mut session = RuntimeSession::new();
    let mut doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            Some(UiPoint::new(200.0, 50.0)),
            &mut ApproxTextMeasurer,
            |_, _| {
                let mut doc = UiDocument::new(LayoutStyle::size(VIEWPORT.width, VIEWPORT.height));
                let modal = doc.add_child(
                    doc.root(),
                    UiNode::container(
                        "dialog",
                        LayoutStyle::absolute_rect(UiRect::new(10.0, 10.0, 120.0, 80.0)),
                    )
                    .with_accessibility(
                        crate::AccessibilityMeta::new(crate::AccessibilityRole::Dialog).modal(),
                    )
                    .with_tooltip(crate::tooltips::TooltipContent::new("Dialog help")),
                );
                doc.add_portal_child(
                    modal,
                    crate::UiPortalTarget::AppOverlay,
                    UiNode::container(
                        "popup",
                        LayoutStyle::absolute_rect(UiRect::new(180.0, 40.0, 80.0, 30.0)),
                    )
                    .with_input(InputBehavior::BUTTON)
                    .with_accessibility(crate::AccessibilityMeta::new(
                        crate::AccessibilityRole::Button,
                    )),
                );
                doc
            },
        )
        .unwrap();
    let authored = doc.node_count();
    for _ in 0..3 {
        let modal = node(&doc, "dialog");
        frame(&mut session, &mut doc, Vec::new());
        let tree = doc.accessibility_snapshot();
        let tooltips: Vec<_> = tree
            .nodes
            .iter()
            .filter(|node| node.role == crate::AccessibilityRole::Tooltip)
            .collect();
        assert_eq!(tooltips.len(), 1, "owned popup should inherit dialog help");
        assert_eq!(
            tooltips[0].parent,
            Some(modal),
            "help must remain inside the modal's accessibility subtree"
        );
        assert!(doc.node_in_modal_scope(tooltips[0].id, Some(modal)));
        session.retain_document(doc);
        doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                Some(UiPoint::new(200.0, 50.0)),
                &mut ApproxTextMeasurer,
                |_, _| panic!("tooltip must not rebuild the app"),
            )
            .unwrap();
        assert_eq!(
            doc.node_count(),
            authored,
            "runtime tooltip should be removed before the next frame"
        );
    }
}

#[cfg(feature = "widgets")]
#[test]
fn cursor_tooltips_follow_pointer_moves_in_cached_frames_at_ui_scale() {
    for ui_scale in [1.0, 2.0] {
        let mut session = RuntimeSession::new();
        let scale = UiDocumentScale::new(ui_scale, 1.0);
        let mut doc = session
            .build_document(VIEWPORT, scale, None, &mut ApproxTextMeasurer, |_, _| {
                let mut doc = UiDocument::new(
                    LayoutStyle::new()
                        .with_width_percent(1.0)
                        .with_height_percent(1.0),
                );
                doc.add_child(
                    doc.root(),
                    UiNode::container(
                        "control",
                        LayoutStyle::absolute_rect(UiRect::new(20.0, 20.0, 160.0, 100.0)),
                    )
                    .with_input(InputBehavior::BUTTON)
                    .with_tooltip(crate::tooltips::TooltipContent::new("Help"))
                    .with_tooltip_size(UiSize::new(80.0, 40.0))
                    .with_tooltip_placement(crate::tooltips::TooltipPlacement::Cursor),
                );
                doc
            })
            .unwrap();
        for (index, point) in [
            UiPoint::new(60.0, 60.0),
            UiPoint::new(100.0, 80.0),
            UiPoint::new(170.0, 110.0),
        ]
        .into_iter()
        .enumerate()
        {
            frame(
                &mut session,
                &mut doc,
                vec![RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Move,
                    point,
                    index as u64 + 1,
                ))],
            );
            let tooltip = doc
                .accessibility_snapshot()
                .nodes
                .into_iter()
                .find(|node| node.role == crate::AccessibilityRole::Tooltip)
                .expect("cursor help");
            assert_eq!(
                tooltip.rect,
                UiRect::new(
                    point.x + 8.0 * ui_scale,
                    point.y + 8.0 * ui_scale,
                    80.0 * ui_scale,
                    40.0 * ui_scale
                ),
                "cursor placement ignores current pointer at UI scale {ui_scale}"
            );
            session.retain_document(doc);
            doc = session
                .build_document(
                    VIEWPORT,
                    scale,
                    Some(point),
                    &mut ApproxTextMeasurer,
                    |_, _| panic!("pointer-only movement should reuse the view"),
                )
                .unwrap();
        }
    }
}

#[cfg(feature = "widgets")]
#[test]
fn authored_tooltips_follow_host_scale_through_cached_views() {
    use crate::tooltips::{TooltipAnchor, TooltipContent, TooltipPlacement, TooltipRequest};
    use crate::widgets::tooltip::{tooltip_box_from_request, TooltipBoxOptions};

    let anchor = UiRect::new(180.0, 140.0, 20.0, 20.0);
    let size = UiSize::new(64.0, 24.0);
    for ui_scale in [0.5, 1.0, 1.5, 2.0] {
        for placement in [
            TooltipPlacement::Above,
            TooltipPlacement::Below,
            TooltipPlacement::Left,
            TooltipPlacement::Right,
            TooltipPlacement::Cursor,
        ] {
            for cursor in [None, Some(UiPoint::new(210.0, 180.0))] {
                let mut session = RuntimeSession::new();
                let mut builds = 0;
                for step in 0..3 {
                    if step < 2 {
                        session.invalidate_view();
                    }
                    let mut doc = session
                        .build_document(
                            VIEWPORT,
                            UiDocumentScale::new(ui_scale, 2.0),
                            cursor,
                            &mut ApproxTextMeasurer,
                            |viewport, views| {
                                assert!(step < 2, "unchanged view was rebuilt");
                                let mut doc = UiDocument::new(
                                    LayoutStyle::new()
                                        .with_width_percent(1.0)
                                        .with_height_percent(1.0),
                                );
                                let root = doc.root();
                                views.section(&mut doc, root, "help.section", &(), |_, _| {
                                    builds += 1;
                                    let mut fragment = UiDocument::new(
                                        LayoutStyle::new()
                                            .with_width_percent(1.0)
                                            .with_height_percent(1.0),
                                    );
                                    let root = fragment.root();
                                    tooltip_box_from_request(
                                        &mut fragment,
                                        root,
                                        "help",
                                        &TooltipRequest::new(
                                            TooltipAnchor::new(root, anchor),
                                            TooltipContent::new("Help"),
                                        )
                                        .placement(placement),
                                        UiRect::new(0.0, 0.0, viewport.width, viewport.height),
                                        size,
                                        cursor,
                                        TooltipBoxOptions::default().with_animation(None),
                                    );
                                    fragment
                                });
                                doc
                            },
                        )
                        .unwrap();
                    let gap = 8.0 * ui_scale;
                    let width = size.width * ui_scale;
                    let height = size.height * ui_scale;
                    let (x, y) = match placement {
                        TooltipPlacement::Above => (anchor.x, anchor.y - height - gap),
                        TooltipPlacement::Below => (anchor.x, anchor.bottom() + gap),
                        TooltipPlacement::Left => (anchor.x - width - gap, anchor.y),
                        TooltipPlacement::Right => (anchor.right() + gap, anchor.y),
                        TooltipPlacement::Cursor => cursor
                            .map(|point| (point.x + gap, point.y + gap))
                            .unwrap_or((anchor.right() + gap, anchor.bottom() + gap)),
                    };
                    let expected = UiRect::new(x, y, width, height);
                    assert_eq!(
                        doc.node(node(&doc, "help")).layout.rect,
                        expected,
                        "ui={ui_scale}, placement={placement:?}, cursor={cursor:?}, step={step}"
                    );
                    let output = frame(&mut session, &mut doc, Vec::new());
                    let help = output
                        .accessibility_tree
                        .nodes
                        .iter()
                        .find(|node| node.role == crate::AccessibilityRole::Tooltip)
                        .unwrap();
                    assert_eq!(help.rect, expected);
                    session.retain_document(doc);
                }
                assert_eq!(builds, 1, "unchanged help section was rebuilt");
            }
        }
    }
}

#[cfg(feature = "widgets")]
#[test]
fn constrained_popups_keep_geometry_and_scroll_through_cached_section_rebuilds() {
    use crate::widgets::ext::{
        select_menu_popup, AnchoredPopup, PopupPlacement, SelectMenuOptions, SelectMenuState,
        SelectOption,
    };
    for ui_scale in [0.5, 1.0, 2.0] {
        let mut session = RuntimeSession::new();
        let mut builds = 0;
        let mut bottom_offset = None;
        for step in 0..3 {
            session.invalidate_view();
            let mut doc = session
                .build_document(
                    VIEWPORT,
                    UiDocumentScale::new(ui_scale, 2.0),
                    None,
                    &mut ApproxTextMeasurer,
                    |viewport, views| {
                        let mut doc = UiDocument::new(
                            LayoutStyle::new()
                                .with_width_percent(1.0)
                                .with_height_percent(1.0),
                        );
                        let root = doc.root();
                        views.section(&mut doc, root, "menu.section", &(), |_, _| {
                            builds += 1;
                            let mut fragment = UiDocument::new(
                                LayoutStyle::new()
                                    .with_width_percent(1.0)
                                    .with_height_percent(1.0),
                            );
                            let root = fragment.root();
                            let items: Vec<_> = (0..32)
                                .map(|i| SelectOption::new(i.to_string(), format!("Item {i}")))
                                .collect();
                            select_menu_popup(
                                &mut fragment,
                                root,
                                "choice",
                                AnchoredPopup::new(
                                    UiRect::new(40.0, 20.0, 80.0, 24.0),
                                    UiRect::new(0.0, 0.0, viewport.width, viewport.height),
                                    PopupPlacement::default().with_flip(false),
                                ),
                                &items,
                                &SelectMenuState::new(),
                                SelectMenuOptions {
                                    width: 120.0,
                                    row_height: 28.0,
                                    max_visible_rows: 32,
                                    ..Default::default()
                                },
                            );
                            fragment
                        });
                        doc
                    },
                )
                .unwrap();
            let popup = node(&doc, "choice");
            let rect = doc.node(popup).layout.rect;
            assert_eq!(
                rect,
                UiRect::new(
                    40.0,
                    4.0 * ui_scale,
                    120.0 * ui_scale,
                    VIEWPORT.height - 8.0 * ui_scale
                )
            );
            let events = if step == 0 {
                let first = doc.node(node(&doc, "choice.option.0")).layout.rect;
                vec![RawInputEvent::Wheel(crate::input::RawWheelEvent::pixels(
                    UiPoint::new(first.x + first.width / 2.0, first.y + first.height / 2.0),
                    UiPoint::new(0.0, 10000.0),
                    1,
                ))]
            } else {
                Vec::new()
            };
            frame(&mut session, &mut doc, events);
            let offset = doc
                .scroll_state(popup)
                .expect("clipped popup scroll state")
                .offset();
            assert!(
                offset.y > 0.0,
                "ui={ui_scale}, frame={step}: scroll was lost"
            );
            if let Some(previous) = bottom_offset {
                assert_eq!(offset, previous);
            }
            bottom_offset = Some(offset);
            let last = node(&doc, "choice.option.31");
            let last_rect = doc.node(last).layout.rect;
            assert!(last_rect.y >= rect.y && last_rect.bottom() <= rect.bottom() + 0.01);
            assert_eq!(
                doc.hit_test(UiPoint::new(
                    last_rect.x + last_rect.width / 2.0,
                    last_rect.y + last_rect.height / 2.0
                )),
                Some(last)
            );
            session.retain_document(doc);
        }
        assert_eq!(builds, 1, "unchanged section was rebuilt");
    }
}

#[cfg(feature = "widgets")]
#[test]
fn active_menu_rows_follow_keyboard_changes_without_replaying_after_manual_scroll() {
    use crate::widgets::ext::{
        select_menu_popup, AnchoredPopup, PopupPlacement, SelectMenuOptions, SelectMenuState,
        SelectOption,
    };
    for cached in [false, true] {
        for base_scale in [0.5, 1.0, 2.0] {
            let mut session = RuntimeSession::new();
            let items: Vec<_> = (0..32)
                .map(|i| SelectOption::new(i.to_string(), format!("Item {i}")))
                .collect();
            let mut state = SelectMenuState::new().with_open(&items);
            let mut builds = 0;
            let mut previous_active = None;
            for step in 0..8 {
                let key = match step {
                    0 | 3 => Some(crate::KeyCode::End),
                    2 => Some(crate::KeyCode::Home),
                    _ => None,
                };
                if let Some(key) = key {
                    state.handle_event(
                        &items,
                        &UiInputEvent::Key {
                            key,
                            modifiers: Default::default(),
                        },
                    );
                }
                let scale = base_scale * if step >= 4 { 1.25 } else { 1.0 };
                session.invalidate_view();
                let mut doc = session
                    .build_document(
                        VIEWPORT,
                        UiDocumentScale::new(scale, 2.0),
                        None,
                        &mut ApproxTextMeasurer,
                        |viewport, views| {
                            let mut doc = UiDocument::new(
                                LayoutStyle::column()
                                    .with_width_percent(1.0)
                                    .with_height_percent(1.0),
                            );
                            let root = doc.root();
                            // Unrelated siblings change every document-local ID in
                            // the menu, including when its section is reused.
                            for i in 0..step {
                                doc.add_child(
                                    root,
                                    UiNode::container(
                                        format!("unrelated.{i}"),
                                        LayoutStyle::size(0.0, 0.0),
                                    ),
                                );
                            }
                            if step != 6 {
                                views.section(
                                    &mut doc,
                                    root,
                                    "menu.section",
                                    &(state.active_index(), if cached { 0 } else { step }),
                                    |_, _| {
                                        builds += 1;
                                        let mut fragment = UiDocument::new(
                                            LayoutStyle::column()
                                                .with_width_percent(1.0)
                                                .with_height_percent(1.0),
                                        );
                                        let root = fragment.root();
                                        select_menu_popup(
                                            &mut fragment,
                                            root,
                                            "choice",
                                            AnchoredPopup::new(
                                                UiRect::new(40.0, 20.0, 80.0, 24.0),
                                                UiRect::new(
                                                    0.0,
                                                    0.0,
                                                    viewport.width,
                                                    viewport.height,
                                                ),
                                                PopupPlacement::default().with_flip(false),
                                            ),
                                            &items,
                                            &state,
                                            SelectMenuOptions {
                                                width: 120.0,
                                                row_height: 28.0,
                                                max_visible_rows: 32,
                                                ..Default::default()
                                            },
                                        );
                                        fragment
                                    },
                                );
                            }
                            doc
                        },
                    )
                    .unwrap();
                if step == 6 {
                    frame(&mut session, &mut doc, Vec::new());
                    session.retain_document(doc);
                    continue;
                }
                let popup = node(&doc, "choice");
                let active = node(
                    &doc,
                    &format!("choice.option.{}", state.active_index().unwrap()),
                );
                if step == 1 {
                    assert_ne!(Some(active), previous_active, "fixture must remap node IDs");
                    assert_eq!(builds, if cached { 1 } else { 2 });
                }
                previous_active = Some(active);
                let context = format!("cached={cached}, scale={scale}, step={step}");
                if matches!(step, 1 | 5) {
                    assert_eq!(
                        doc.scroll_state(popup).unwrap().offset().y,
                        0.0,
                        "{context}: rebuild overrode manual scrolling"
                    );
                } else {
                    let layout = doc.node(active).layout();
                    assert!(
                        layout.rect.y >= layout.clip_rect.y - 0.01
                            && layout.rect.bottom() <= layout.clip_rect.bottom() + 0.01,
                        "{context}: active row clipped: {layout:?}"
                    );
                    assert_eq!(
                        doc.hit_test(UiPoint::new(
                            layout.rect.x + layout.rect.width / 2.0,
                            layout.rect.y + layout.rect.height / 2.0
                        )),
                        Some(active),
                        "{context}"
                    );
                }
                let events = if matches!(step, 0 | 1 | 3 | 4) {
                    let rect = doc.node(popup).layout.rect;
                    vec![RawInputEvent::Wheel(crate::input::RawWheelEvent::pixels(
                        UiPoint::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0),
                        UiPoint::new(0.0, if step == 1 { 7.125 } else { -10000.0 }),
                        step as u64 + 1,
                    ))]
                } else {
                    Vec::new()
                };
                frame(&mut session, &mut doc, events);
                if matches!(step, 0 | 1 | 3 | 4) {
                    assert_eq!(
                        doc.scroll_state(popup).unwrap().offset().y,
                        if step == 1 { 7.125 } else { 0.0 },
                        "{context}: reveal fought wheel input"
                    );
                }
                session.retain_document(doc);
            }
        }
    }
}

#[cfg(feature = "widgets")]
#[test]
fn authored_menu_offsets_override_reveal_and_do_not_replay_in_cached_sections() {
    use crate::widgets::ext::{select_menu, SelectMenuOptions, SelectMenuState, SelectOption};
    for cached in [false, true] {
        for authored_offset in [0.0, 10000.0] {
            let mut session = RuntimeSession::new();
            let items: Vec<_> = (0..16)
                .map(|i| SelectOption::new(i.to_string(), format!("Item {i}")))
                .collect();
            // Deliberately request the opposite end from the active row.
            let active = if authored_offset == 0.0 { 15 } else { 0 };
            let state = SelectMenuState::new()
                .with_open(&items)
                .with_active(&items, active);
            let mut manual_offset = 0.0;
            for step in 0..2 {
                session.invalidate_view();
                let mut doc = session
                    .build_document(
                        VIEWPORT,
                        UiDocumentScale::DEFAULT,
                        None,
                        &mut ApproxTextMeasurer,
                        |_, views| {
                            let mut doc = UiDocument::new(
                                LayoutStyle::column().with_size(VIEWPORT.width, VIEWPORT.height),
                            );
                            let root = doc.root();
                            views.section(
                                &mut doc,
                                root,
                                "section",
                                &if cached { 0 } else { step },
                                |_, _| {
                                    let mut fragment = UiDocument::new(LayoutStyle::column());
                                    let root = fragment.root();
                                    let menu = select_menu(
                                        &mut fragment,
                                        root,
                                        "choice",
                                        &items,
                                        &state,
                                        SelectMenuOptions {
                                            max_visible_rows: 4,
                                            ..Default::default()
                                        },
                                    );
                                    fragment
                                        .node_mut(menu.root)
                                        .scroll
                                        .as_mut()
                                        .unwrap()
                                        .set_offset(UiPoint::new(0.0, authored_offset));
                                    fragment
                                },
                            );
                            doc
                        },
                    )
                    .unwrap();
                let popup = node(&doc, "choice");
                let scroll = doc.scroll_state(popup).unwrap();
                let expected = if cached && step == 1 {
                    manual_offset
                } else {
                    authored_offset.min(scroll.max_offset().y)
                };
                assert_eq!(
                    scroll.offset().y,
                    expected,
                    "cached={cached}, authored={authored_offset}, step={step}"
                );
                if step == 0 {
                    manual_offset = if authored_offset == 0.0 {
                        scroll.max_offset().y
                    } else {
                        0.0
                    };
                    assert!(doc.set_scroll_offset(popup, UiPoint::new(0.0, manual_offset)));
                }
                frame(&mut session, &mut doc, Vec::new());
                assert_eq!(
                    doc.scroll_state(popup).unwrap().offset().y,
                    if step == 0 { manual_offset } else { expected }
                );
                session.retain_document(doc);
            }
        }
    }
}

#[test]
fn click_after_scrolling_uses_the_scrolled_geometry_in_every_frame_partition() {
    let point = UiPoint::new(20.0, 10.0);
    let events = [
        RawInputEvent::Wheel(crate::input::RawWheelEvent::pixels(
            point,
            UiPoint::new(0.0, 40.0),
            1,
        )),
        RawInputEvent::Pointer(RawPointerEvent::new(
            PointerEventKind::Down(PointerButton::Primary),
            point,
            2,
        )),
        RawInputEvent::Pointer(RawPointerEvent::new(
            PointerEventKind::Up(PointerButton::Primary),
            point,
            3,
        )),
    ];
    for partition in (0..4).rev() {
        let mut session = RuntimeSession::new();
        let mut doc = scroll_document();
        let content = node(&doc, "content");
        doc.node_mut(content).style.layout = LayoutStyle::column()
            .with_size(100.0, 240.0)
            .with_flex_shrink(0.0)
            .style;
        for name in ["first", "second", "third"] {
            doc.add_child(
                content,
                UiNode::container(name, LayoutStyle::size(80.0, 40.0).with_flex_shrink(0.0))
                    .with_input(InputBehavior::BUTTON)
                    .with_action(name),
            );
        }
        let mut actions = Vec::new();
        let mut queued = Vec::new();
        for (index, event) in events.iter().enumerate() {
            queued.push(event.clone());
            if index == events.len() - 1 || partition & (1 << index) != 0 {
                prepare(&mut session, &mut doc);
                let output = frame(&mut session, &mut doc, std::mem::take(&mut queued));
                actions.extend(crate::host::collect_document_widget_actions(&output));
            }
        }
        assert_eq!(
            actions
                .iter()
                .map(|action| doc.node(action.target).name())
                .collect::<Vec<_>>(),
            ["second"],
            "partition={partition}"
        );
        assert_eq!(
            session.interaction().focused,
            Some(node(&doc, "second")),
            "partition={partition}"
        );
    }
}

#[test]
fn queued_scroll_actions_preserve_each_event_offset() {
    let mut session = RuntimeSession::new();
    let mut doc = scroll_document();
    let scroll = node(&doc, "scroll");
    doc.node_mut(scroll).action = Some("scroll.changed".into());
    prepare(&mut session, &mut doc);
    let output = frame(
        &mut session,
        &mut doc,
        vec![10.0, 15.0]
            .into_iter()
            .enumerate()
            .map(|(index, delta)| {
                RawInputEvent::Wheel(crate::input::RawWheelEvent::pixels(
                    UiPoint::new(20.0, 10.0),
                    UiPoint::new(0.0, delta),
                    index as u64,
                ))
            })
            .collect(),
    );
    let actions = crate::host::collect_document_widget_actions(&output);
    let offsets = actions
        .iter()
        .filter_map(|action| match action.kind {
            WidgetActionKind::Scroll(scroll) => Some(scroll.offset.y),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(offsets, [10.0, 25.0]);
}

#[test]
fn pointer_edit_actions_keep_geometry_from_before_later_scroll_events() {
    let mut session = RuntimeSession::new();
    let mut doc = scroll_document();
    let content = node(&doc, "content");
    let control = doc.add_child(
        content,
        UiNode::container("control", LayoutStyle::size(80.0, 40.0))
            .with_input(InputBehavior::BUTTON)
            .with_action("control.changed")
            .with_action_mode(WidgetActionMode::PointerEdit),
    );
    prepare(&mut session, &mut doc);
    let point = UiPoint::new(20.0, 10.0);
    let output = frame(
        &mut session,
        &mut doc,
        vec![
            RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Down(PointerButton::Primary),
                point,
                1,
            )),
            RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Up(PointerButton::Primary),
                point,
                2,
            )),
            RawInputEvent::Wheel(crate::input::RawWheelEvent::pixels(
                point,
                UiPoint::new(0.0, 40.0),
                3,
            )),
        ],
    );
    assert_eq!(doc.node(control).layout().rect.y, -40.0);
    let actions = crate::host::collect_document_widget_actions(&output);
    let [WidgetAction {
        kind: WidgetActionKind::PointerEdit(edit),
        ..
    }] = actions.as_slice()
    else {
        panic!("expected one pointer edit: {actions:?}");
    };
    assert_eq!(edit.target_rect.y, 0.0);
    assert_eq!(edit.local_position, point);
}

#[test]
fn queued_pointer_moves_reuse_unchanged_text_layout() {
    let session = RuntimeSession::new();
    let mut doc = measured_document(VIEWPORT, "unchanged label");
    let mut measurer = CountingMeasurer::default();
    doc.compute_layout(VIEWPORT, &mut measurer).unwrap();
    let measured = measurer.0;
    assert!(measured > 0);
    let events = (0..100)
        .map(|index| {
            RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Move,
                UiPoint::new(20.0 + index as f32, 10.0),
                index,
            ))
        })
        .collect();
    let output = session
        .process_input(&mut doc, VIEWPORT, events, Vec::new(), &mut measurer)
        .unwrap();
    assert_eq!(output.events.len(), 100);
    assert_eq!(
        measurer.0, measured,
        "routing pointer motion must not remeasure unchanged text"
    );
}

#[test]
fn cancelled_pointer_edit_preserves_the_last_delivered_geometry() {
    let mut session = RuntimeSession::new();
    let mut doc = scroll_document();
    let content = node(&doc, "content");
    let control = doc.add_child(
        content,
        UiNode::container("control", LayoutStyle::size(80.0, 40.0))
            .with_input(InputBehavior::BUTTON)
            .with_action("control.changed")
            .with_action_mode(WidgetActionMode::PointerEdit),
    );
    prepare(&mut session, &mut doc);
    let output = frame(
        &mut session,
        &mut doc,
        vec![
            RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Down(PointerButton::Primary),
                UiPoint::new(20.0, 10.0),
                1,
            )),
            RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Move,
                UiPoint::new(40.0, 15.0),
                2,
            )),
            RawInputEvent::Wheel(crate::input::RawWheelEvent::pixels(
                UiPoint::new(20.0, 10.0),
                UiPoint::new(0.0, 40.0),
                3,
            )),
        ],
    );
    let actions = crate::host::collect_document_widget_actions(&output);
    let [WidgetAction {
        kind: WidgetActionKind::PointerEdit(edit),
        ..
    }] = actions.as_slice()
    else {
        panic!("expected one pointer edit: {actions:?}");
    };
    assert_eq!(doc.node(control).layout().rect.y, -40.0);
    let mut replacement = scroll_document();
    prepare(&mut session, &mut replacement);
    let cancellations = session.take_interaction_cancellations();
    let [RuntimeInteractionCancellation {
        kind: WidgetActionKind::PointerEdit(cancel),
        ..
    }] = cancellations.as_slice()
    else {
        panic!("expected one cancellation: {cancellations:?}");
    };
    assert_eq!(cancel.phase, WidgetValueEditPhase::Cancel);
    assert_eq!(cancel.target_rect, edit.target_rect);
    assert_eq!(cancel.local_position, edit.local_position);
}

#[cfg(feature = "widgets")]
fn modal_focus_document(dialogs: &[&str], padding: bool) -> UiDocument {
    use crate::widgets::{
        button, modal_dialog, text_input, ButtonOptions, ModalDialogOptions, TextInputOptions,
        TextInputState,
    };
    let mut doc = UiDocument::new(LayoutStyle::column().with_size(400.0, 300.0));
    let root = doc.root();
    if padding {
        doc.add_child(
            root,
            UiNode::container("padding", LayoutStyle::size(10.0, 10.0)),
        );
    }
    for name in ["opener", "alternate"] {
        button(
            &mut doc,
            root,
            name,
            name,
            ButtonOptions::default().with_action(name),
        );
    }
    text_input(
        &mut doc,
        root,
        "background.editor",
        &mut TextInputState::new(""),
        TextInputOptions::default().with_edit_action("background.editor"),
    );
    for name in dialogs {
        let dialog = modal_dialog(
            &mut doc,
            root,
            *name,
            *name,
            ModalDialogOptions::default()
                .with_size(240.0, 200.0)
                .without_close_button(),
        );
        text_input(
            &mut doc,
            dialog.body,
            format!("{name}.editor"),
            &mut TextInputState::new(""),
            TextInputOptions::default().with_edit_action(format!("{name}.editor")),
        );
    }
    doc
}

#[cfg(feature = "widgets")]
#[test]
fn opening_and_closing_modals_owns_keyboard_focus_across_rebuilds() {
    use crate::input::{RawKeyboardEvent, RawTextInputEvent};
    use crate::{KeyCode, KeyModifiers};

    for opener in ["opener", "background.editor"] {
        let mut session = RuntimeSession::new();
        let stages: &[(&[&str], Option<&str>, &str)] = &[
            (&[], Some(opener), opener),
            (&["back"], None, "back.dialog"),
            (&["back"], Some("back.editor"), "back.editor"),
            (&["back", "front"], None, "front.dialog"),
            (&["back"], None, "back.editor"),
            (&[], None, opener),
            (&[], None, opener),
        ];
        let mut previous = None;
        for (step, &(dialogs, authored, expected)) in stages.iter().enumerate() {
            let mut doc = modal_focus_document(dialogs, step % 2 != 0);
            if let Some(authored) = authored {
                doc.set_focus_state(UiFocusState {
                    focused: Some(node(&doc, authored)),
                    ..Default::default()
                });
            }
            prepare(&mut session, &mut doc);
            let target = node(&doc, expected);
            assert_eq!(
                session.interaction().focused,
                Some(target),
                "opener={opener}, step={step}"
            );
            let output = frame(
                &mut session,
                &mut doc,
                vec![
                    RawInputEvent::Keyboard(RawKeyboardEvent::press(
                        KeyCode::Enter,
                        KeyModifiers::NONE,
                        1,
                    )),
                    RawInputEvent::Text(RawTextInputEvent::new("x", 2)),
                ],
            );
            let actions = crate::host::collect_document_widget_actions(&output);
            let edits = actions
                .iter()
                .filter(|action| {
                    matches!(
                        &action.kind,
                        WidgetActionKind::Activate(_)
                            | WidgetActionKind::TextEdit(crate::WidgetTextEdit {
                                event: UiInputEvent::TextInput(_),
                                ..
                            })
                    )
                })
                .collect::<Vec<_>>();
            if expected.ends_with(".dialog") {
                assert!(
                    edits.is_empty(),
                    "background input escaped modal: {edits:?}"
                );
            } else {
                assert!(!edits.is_empty());
                assert!(edits.iter().all(|action| action.target == target));
            }
            let focus_changes = actions
                .iter()
                .filter_map(|action| match &action.kind {
                    WidgetActionKind::Focus(change) => {
                        Some((doc.node(action.target).name().to_owned(), change.focused))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            let mut wanted = Vec::new();
            if previous != Some(expected) {
                if let Some(name) = previous.filter(|name: &&str| name.ends_with(".editor")) {
                    if doc.nodes().iter().any(|node| node.name() == name) {
                        wanted.push((name.to_owned(), false));
                    }
                }
                if expected.ends_with(".editor") {
                    wanted.push((expected.to_owned(), true));
                }
            }
            assert_eq!(focus_changes, wanted, "step={step}");
            previous = Some(expected);
        }
    }
}

#[cfg(feature = "widgets")]
#[test]
fn modal_restore_policy_respects_authored_focus_and_target_lifetimes() {
    use crate::accessibility::FocusRestoreTarget;
    for policy in ["previous", "none", "alternate"] {
        for closing in [
            "removed",
            "hidden",
            "disabled",
            "authored",
            "deleted-opener",
            "disabled-opener",
            "recreated-opener",
            "ambiguous-opener",
        ] {
            let mut session = RuntimeSession::new();
            let mut doc = modal_focus_document(&[], false);
            let opener = node(&doc, "opener");
            doc.set_focus_state(UiFocusState {
                focused: Some(opener),
                ..Default::default()
            });
            prepare(&mut session, &mut doc);
            frame(&mut session, &mut doc, Vec::new());
            let mut doc = modal_focus_document(&["dialog"], true);
            let dialog = node(&doc, "dialog.dialog");
            let alternate = node(&doc, "alternate");
            doc.node_mut(dialog)
                .accessibility
                .as_mut()
                .unwrap()
                .modal_focus_restore = match policy {
                "none" => FocusRestoreTarget::None,
                "alternate" => FocusRestoreTarget::Node(alternate),
                _ => FocusRestoreTarget::Previous,
            };
            prepare(&mut session, &mut doc);
            assert_eq!(session.interaction().focused, Some(dialog));
            frame(&mut session, &mut doc, Vec::new());
            if closing == "recreated-opener" {
                let opener = node(&doc, "opener");
                doc.node_mut(opener).name = "replacement".into();
                prepare(&mut session, &mut doc);
                frame(&mut session, &mut doc, Vec::new());
            }
            let mut next = modal_focus_document(
                if matches!(closing, "hidden" | "disabled") {
                    &["dialog"]
                } else {
                    &[]
                },
                false,
            );
            if closing == "hidden" {
                let id = node(&next, "dialog");
                next.node_mut(id).style.layout.display = taffy::prelude::Display::None;
            }
            if closing == "disabled" {
                let id = node(&next, "dialog.dialog");
                next.set_node_enabled(id, false);
            }
            if matches!(closing, "deleted-opener" | "ambiguous-opener") {
                let id = node(&next, "opener");
                next.node_mut(id).name = if closing == "deleted-opener" {
                    "replacement"
                } else {
                    "alternate"
                }
                .into();
            }
            if closing == "disabled-opener" {
                let id = node(&next, "opener");
                next.set_node_enabled(id, false);
            }
            if closing == "authored" {
                next.set_focus_state(UiFocusState {
                    focused: Some(node(&next, "background.editor")),
                    ..Default::default()
                });
            }
            // Hidden dialogs still carry the author-selected restore policy.
            if matches!(closing, "hidden" | "disabled") {
                let dialog = node(&next, "dialog.dialog");
                let alternate = node(&next, "alternate");
                next.node_mut(dialog)
                    .accessibility
                    .as_mut()
                    .unwrap()
                    .modal_focus_restore = match policy {
                    "none" => FocusRestoreTarget::None,
                    "alternate" => FocusRestoreTarget::Node(alternate),
                    _ => FocusRestoreTarget::Previous,
                };
            }
            prepare(&mut session, &mut next);
            let expected = if closing == "authored" {
                Some(node(&next, "background.editor"))
            } else if policy == "alternate" && closing != "ambiguous-opener" {
                Some(node(&next, "alternate"))
            } else if policy == "previous" && matches!(closing, "removed" | "hidden" | "disabled") {
                Some(node(&next, "opener"))
            } else {
                None
            };
            assert_eq!(
                session.interaction().focused,
                expected,
                "{policy}/{closing}"
            );
            let output = frame(&mut session, &mut next, Vec::new());
            let focus_changes = crate::host::collect_document_widget_actions(&output)
                .into_iter()
                .filter(|action| matches!(action.kind, WidgetActionKind::Focus(_)))
                .collect::<Vec<_>>();
            assert_eq!(
                focus_changes.len(),
                usize::from(closing == "authored"),
                "{policy}/{closing}"
            );
        }
    }
}

#[cfg(feature = "widgets")]
#[test]
fn modal_keyboard_guard_rejects_background_focus_without_session_preparation() {
    use crate::input::{RawKeyboardEvent, RawTextInputEvent};
    use crate::{KeyCode, KeyModifiers};
    for background in ["opener", "background.editor"] {
        let mut session = RuntimeSession::new();
        let mut doc = modal_focus_document(&["dialog"], false);
        prepare(&mut session, &mut doc);
        doc.set_focus_state(UiFocusState {
            focused: Some(node(&doc, background)),
            ..Default::default()
        });
        let output = frame(
            &mut session,
            &mut doc,
            vec![
                RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    KeyCode::Enter,
                    KeyModifiers::NONE,
                    1,
                )),
                RawInputEvent::Text(RawTextInputEvent::new("x", 2)),
            ],
        );
        assert_eq!(
            session.interaction().focused,
            Some(node(&doc, "dialog.dialog"))
        );
        let actions = crate::host::collect_document_widget_actions(&output);
        assert!(!actions.iter().any(|action| matches!(
            &action.kind,
            WidgetActionKind::Activate(_)
                | WidgetActionKind::TextEdit(crate::WidgetTextEdit {
                    event: UiInputEvent::TextInput(_),
                    ..
                })
        )));
    }
}

#[cfg(feature = "widgets")]
#[test]
fn modal_background_click_preserves_composition_across_frame_boundaries() {
    use crate::host::collect_document_widget_actions;
    use crate::input::RawTextCompositionEvent;
    use crate::TextCompositionEvent;

    for rebuild in [false, true] {
        // Exercise every partition of down, up, and commit into input frames.
        for partition in 0..4 {
            let mut session = RuntimeSession::new();
            let mut doc = modal_focus_document(&["back", "front"], false);
            let editor = node(&doc, "front.editor");
            doc.set_focus_state(UiFocusState {
                focused: Some(editor),
                ..Default::default()
            });
            prepare(&mut session, &mut doc);
            frame(&mut session, &mut doc, Vec::new());
            let input = session
                .interaction()
                .text_ime
                .as_ref()
                .unwrap()
                .input
                .clone();
            frame(
                &mut session,
                &mut doc,
                vec![RawInputEvent::Composition(RawTextCompositionEvent {
                    input: input.clone(),
                    event: TextCompositionEvent::Preedit {
                        text: "候補".into(),
                        selection: None,
                        replacement: None,
                    },
                    timestamp_millis: 1,
                })],
            );
            let point = UiPoint::new(2.0, 2.0);
            assert_eq!(
                doc.hit_test_result(point),
                Some(crate::HitTestResult::Blocked(node(&doc, "front.dialog")))
            );
            let events = [
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Down(PointerButton::Primary),
                    point,
                    2,
                )),
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Up(PointerButton::Primary),
                    point,
                    3,
                )),
                RawInputEvent::Composition(RawTextCompositionEvent {
                    input: input.clone(),
                    event: TextCompositionEvent::Commit {
                        text: "候補".into(),
                        replacement: None,
                    },
                    timestamp_millis: 4,
                }),
            ];
            let mut batch = Vec::new();
            let mut commits = 0;
            for (index, event) in events.into_iter().enumerate() {
                batch.push(event);
                if index != 2 && partition & (1 << index) == 0 {
                    continue;
                }
                if rebuild {
                    doc = modal_focus_document(&["back", "front"], index % 2 == 0);
                    prepare(&mut session, &mut doc);
                    assert!(session.take_interaction_cancellations().is_empty());
                }
                let output = frame(&mut session, &mut doc, std::mem::take(&mut batch));
                let editor = node(&doc, "front.editor");
                assert_eq!(
                    session.interaction().focused,
                    Some(editor),
                    "rebuild={rebuild}, partition={partition}, event={index}"
                );
                assert_eq!(
                    session.interaction().text_ime.as_ref().unwrap().input,
                    input
                );
                for action in collect_document_widget_actions(&output) {
                    assert_eq!(action.target, editor);
                    assert!(
                        matches!(action.kind, WidgetActionKind::TextEdit(crate::WidgetTextEdit {
                        event: UiInputEvent::Composition { event: TextCompositionEvent::Commit { ref text, .. }, .. }, ..
                    }) if text == "候補"),
                        "unexpected action: {action:?}"
                    );
                    commits += 1;
                }
            }
            assert_eq!(commits, 1, "rebuild={rebuild}, partition={partition}");
        }
    }
}

#[cfg(feature = "widgets")]
#[test]
fn modal_open_cancels_ime_once_and_rejects_the_previous_session_commit() {
    use crate::input::RawTextCompositionEvent;
    use crate::TextCompositionEvent;
    for rebound in [false, true] {
        let mut session = RuntimeSession::new();
        let mut doc = modal_focus_document(&[], false);
        doc.set_focus_state(UiFocusState {
            focused: Some(node(&doc, "background.editor")),
            ..Default::default()
        });
        prepare(&mut session, &mut doc);
        frame(&mut session, &mut doc, Vec::new());
        let input = session
            .interaction()
            .text_ime
            .as_ref()
            .unwrap()
            .input
            .clone();
        frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Composition(RawTextCompositionEvent {
                input: input.clone(),
                event: TextCompositionEvent::Preedit {
                    text: "候補".into(),
                    selection: None,
                    replacement: None,
                },
                timestamp_millis: 1,
            })],
        );
        let mut modal = modal_focus_document(&["dialog"], true);
        if rebound {
            let editor = node(&modal, "background.editor");
            modal.set_node_action(editor, "rebound.editor");
        }
        prepare(&mut session, &mut modal);
        let cancellations = session.take_interaction_cancellations();
        assert_eq!(cancellations.len(), usize::from(rebound));
        let commit = RawInputEvent::Composition(RawTextCompositionEvent {
            input: input.clone(),
            event: TextCompositionEvent::Commit {
                text: "stale".into(),
                replacement: None,
            },
            timestamp_millis: 2,
        });
        let output = frame(&mut session, &mut modal, vec![commit.clone()]);
        let actions = crate::host::collect_document_widget_actions(&output);
        let cancelled = |kind: &WidgetActionKind| {
            matches!(
                kind,
                WidgetActionKind::TextEdit(crate::WidgetTextEdit {
                    event: UiInputEvent::Composition {
                        event: TextCompositionEvent::Cancel,
                        ..
                    },
                    ..
                })
            )
        };
        let original_binding = WidgetActionBinding::action("background.editor");
        assert_eq!(
            cancellations
                .iter()
                .filter(|cancel| cancel.binding == original_binding && cancelled(&cancel.kind))
                .count()
                + actions
                    .iter()
                    .filter(|action| action.binding == original_binding && cancelled(&action.kind))
                    .count(),
            1
        );
        assert!(output.host_output.ui_events().next().is_none());
        assert!(session.interaction().text_ime.is_none());
        let focus_changes = actions
            .iter()
            .filter_map(|action| match &action.kind {
                WidgetActionKind::Focus(change) => Some(change.focused),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(focus_changes, [false]);
        let mut closed = modal_focus_document(&[], false);
        prepare(&mut session, &mut closed);
        let output = frame(&mut session, &mut closed, vec![commit]);
        assert!(output.host_output.ui_events().next().is_none());
        assert_eq!(
            session.interaction().focused,
            Some(node(&closed, "background.editor"))
        );
        assert_ne!(
            session.interaction().text_ime.as_ref().unwrap().input,
            input
        );
    }
}

#[cfg(feature = "widgets")]
#[test]
fn modal_portal_dropdown_keeps_input_and_accessibility_with_its_owner() {
    use crate::host::collect_document_widget_actions;
    use crate::input::{RawKeyboardEvent, RawWheelEvent};
    use crate::widgets::modal::modal_dialog_dismiss_event_from_pointer_event;
    use crate::widgets::{DialogDismissal, ModalDialogNodes, ModalDialogOptions};
    use crate::{KeyCode, KeyModifiers, UiPortalTarget};

    for (portal, owned) in [
        (UiPortalTarget::AppOverlay, true),
        (UiPortalTarget::named("popup.host"), true),
        (UiPortalTarget::GlobalAppOverlay, false),
        (UiPortalTarget::global_named("popup.host"), false),
    ] {
        let mut session = RuntimeSession::new();
        let mut doc = modal_portal_document("a", portal.clone());
        prepare(&mut session, &mut doc);
        let row = node(&doc, "choice.popup.option.0");
        let popup = node(&doc, "choice.popup");
        let dialog = node(&doc, "dialog.dialog");
        let rect = doc.node(row).layout().rect;
        let point = UiPoint::new(rect.x + 5.0, rect.y + 5.0);
        assert!(!doc.node(dialog).layout().rect.contains_point(point));
        assert_eq!(doc.hit_test(point) == Some(row), owned, "portal={portal:?}");
        let tree = doc.accessibility_snapshot();
        assert_eq!(tree.contains_node(dialog, row), owned);
        assert_eq!(tree.effective_focus_order().contains(&row), owned);
        let owner = node(&doc, "trigger.a");
        if !owned {
            doc.set_node_enabled(owner, false);
            assert!(doc.node_is_enabled(row));
            doc.node_mut(owner).style.layout.display = taffy::prelude::Display::None;
            prepare(&mut session, &mut doc);
            assert!(doc.node(row).layout().visible);
            continue;
        }
        assert_eq!(
            modal_dialog_dismiss_event_from_pointer_event(
                &doc,
                ModalDialogNodes {
                    overlay: node(&doc, "dialog"),
                    scrim: node(&doc, "dialog.scrim"),
                    dialog,
                    header: node(&doc, "dialog.header"),
                    title: node(&doc, "dialog.title"),
                    close_button: None,
                    body: node(&doc, "dialog.body"),
                },
                &ModalDialogOptions::default().with_dismissal(DialogDismissal::STANDARD),
                &UiInputEvent::PointerDown(point),
            ),
            None
        );
        let output = frame(
            &mut session,
            &mut doc,
            vec![
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Down(PointerButton::Primary),
                    point,
                    1,
                )),
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Up(PointerButton::Primary),
                    point,
                    2,
                )),
                RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    KeyCode::Enter,
                    KeyModifiers::NONE,
                    3,
                )),
            ],
        );
        let actions = collect_document_widget_actions(&output);
        assert_eq!(actions.len(), 2, "{actions:?}");
        assert!(actions.iter().all(|action| action.target == row
            && action.binding == WidgetActionBinding::action("choice.option.one")
            && matches!(action.kind, WidgetActionKind::Activate(_))));
        doc.node_mut(row).action = None;
        let output = frame(
            &mut session,
            &mut doc,
            vec![
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Down(PointerButton::Primary),
                    point,
                    4,
                )),
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Up(PointerButton::Primary),
                    point,
                    5,
                )),
            ],
        );
        assert!(
            matches!(collect_document_widget_actions(&output).as_slice(), [action] if action.target == owner && action.binding == WidgetActionBinding::action("trigger.a"))
        );
        let output = frame(
            &mut session,
            &mut doc,
            vec![RawInputEvent::Wheel(RawWheelEvent::pixels(
                point,
                UiPoint::new(0.0, 8.0),
                6,
            ))],
        );
        assert!(doc.scroll_state(popup).unwrap().offset.y > 0.0);
        assert!(!collect_document_widget_actions(&output)
            .iter()
            .any(|action| matches!(action.kind, WidgetActionKind::Activate(_))));

        let enabled_visual =
            crate::UiVisual::panel(crate::ColorRgba::new(40, 120, 200, 255), None, 0.0);
        let disabled_visual =
            crate::UiVisual::panel(crate::ColorRgba::new(80, 80, 80, 255), None, 0.0);
        let nested = doc.add_portal_child(
            popup,
            UiPortalTarget::AppOverlay,
            UiNode::container(
                "nested.popup",
                LayoutStyle::absolute_rect(UiRect::new(340.0, 260.0, 40.0, 25.0)),
            )
            .with_input(InputBehavior::BUTTON)
            .with_interaction_visuals(
                crate::InteractionVisuals::new(enabled_visual).disabled(disabled_visual),
            )
            .with_accessibility(
                crate::AccessibilityMeta::new(crate::AccessibilityRole::Button).focusable(),
            ),
        );
        prepare(&mut session, &mut doc);
        assert_eq!(doc.hit_test(UiPoint::new(345.0, 265.0)), Some(nested));
        assert!(doc.accessibility_snapshot().contains_node(dialog, nested));
        if matches!(portal, UiPortalTarget::Named(_)) {
            let host = node(&doc, "popup.host");
            doc.set_node_enabled(host, false);
            assert!(!doc.node_is_enabled(nested));
            assert_eq!(*doc.node(nested).visual(), disabled_visual);
            doc.set_node_enabled(host, true);
            assert!(doc.node_is_enabled(nested));
            assert_eq!(*doc.node(nested).visual(), enabled_visual);
        }
        doc.set_node_enabled(owner, false);
        assert!(!doc.node_is_enabled(row));
        assert!(!doc.node_is_enabled(nested));
        assert!(!doc.accessibility_snapshot().node(row).unwrap().enabled);
        doc.set_node_enabled(owner, true);
        doc.node_mut(owner).style.layout.display = taffy::prelude::Display::None;
        prepare(&mut session, &mut doc);
        assert!(!doc.node(row).layout().visible);
        assert!(!doc.node(nested).layout().visible);
        assert!(doc.accessibility_snapshot().node(row).is_none());
        assert!(!doc.paint_list().items.iter().any(|item| item.node == popup));
    }
}

#[cfg(feature = "widgets")]
fn modal_portal_document(owner: &str, portal: crate::UiPortalTarget) -> UiDocument {
    use crate::widgets::ext::{
        select_menu_popup, AnchoredPopup, PopupPlacement, SelectMenuOptions, SelectMenuState,
        SelectOption,
    };
    let mut doc = modal_focus_document(&["dialog"], false);
    let body = node(&doc, "dialog.body");
    for name in ["a", "b"] {
        doc.add_child(
            body,
            UiNode::container(format!("trigger.{name}"), LayoutStyle::size(60.0, 20.0))
                .with_input(InputBehavior::BUTTON)
                .with_action(format!("trigger.{name}")),
        );
    }
    let host = doc.add_child(
        doc.root(),
        UiNode::container(
            "popup.host",
            LayoutStyle::absolute_rect(UiRect::new(0.0, 0.0, VIEWPORT.width, VIEWPORT.height)),
        )
        .with_clip_scope(crate::ClipScope::Viewport)
        .with_layer(crate::platform::UiLayer::AppOverlay),
    );
    doc.register_portal_host("popup.host", host);
    let owner = node(&doc, &format!("trigger.{owner}"));
    let choices = [
        SelectOption::new("one", "One"),
        SelectOption::new("two", "Two"),
    ];
    select_menu_popup(
        &mut doc,
        owner,
        "choice.popup",
        AnchoredPopup::new(
            UiRect::new(2.0, 2.0, 60.0, 20.0),
            UiRect::new(0.0, 0.0, VIEWPORT.width, VIEWPORT.height),
            PopupPlacement::default(),
        ),
        &choices,
        &SelectMenuState::new().with_open(&choices),
        SelectMenuOptions::default()
            .with_width(100.0)
            .with_max_visible_rows(1)
            .with_action_prefix("choice")
            .with_portal(portal),
    );
    doc
}

#[cfg(feature = "widgets")]
#[test]
fn portal_owner_identity_survives_section_reuse_but_not_owner_replacement() {
    use crate::host::collect_document_widget_actions;
    let mut session = RuntimeSession::new();
    for (step, owner) in ["a", "a", "a", "b", "b"].into_iter().enumerate() {
        session.invalidate_view();
        let mut doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |_, views| {
                    let mut doc =
                        UiDocument::new(LayoutStyle::size(VIEWPORT.width, VIEWPORT.height));
                    let root = doc.root();
                    if step % 2 != 0 {
                        doc.add_child(
                            root,
                            UiNode::container("padding", LayoutStyle::size(10.0, 10.0)),
                        );
                    }
                    views.section(&mut doc, root, "panel", &owner, |owner, _| {
                        modal_portal_document(owner, crate::UiPortalTarget::AppOverlay)
                    });
                    doc
                },
            )
            .unwrap();
        let row = node(&doc, "choice.popup.option.0");
        let rect = doc.node(row).layout().rect;
        let release = RawInputEvent::Pointer(RawPointerEvent::new(
            PointerEventKind::Up(PointerButton::Primary),
            UiPoint::new(rect.x + 5.0, rect.y + 5.0),
            step as u64 + 10,
        ));
        if step == 3 {
            assert_ne!(
                session.interaction().focused,
                Some(row),
                "replacement owner must not inherit focus"
            );
            assert_eq!(session.interaction().pressed, None);
            let output = frame(&mut session, &mut doc, vec![release.clone()]);
            assert!(
                collect_document_widget_actions(&output).is_empty(),
                "old release must not activate replacement popup"
            );
        }
        if [0, 2, 3].contains(&step) {
            press(&mut session, &mut doc, "choice.popup.option.0");
        } else {
            assert_eq!(
                session.interaction().focused,
                Some(row),
                "cached section must preserve focus"
            );
            assert_eq!(
                session.interaction().pressed,
                Some(row),
                "cached section must preserve its press owner"
            );
            let output = frame(&mut session, &mut doc, vec![release]);
            assert!(
                matches!(collect_document_widget_actions(&output).as_slice(), [action] if action.target == row && matches!(action.kind, WidgetActionKind::Activate(_)))
            );
        }
        session.retain_document(doc);
    }
}

#[cfg(feature = "widgets")]
#[test]
fn cached_modal_sections_remap_explicit_restore_targets() {
    use crate::accessibility::FocusRestoreTarget;
    let mut session = RuntimeSession::new();
    for padding in [false, true, false] {
        session.invalidate_view();
        let mut doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |_, views| {
                    let mut doc = UiDocument::new(LayoutStyle::column().with_size(400.0, 300.0));
                    let root = doc.root();
                    if padding {
                        doc.add_child(
                            root,
                            UiNode::container("padding", LayoutStyle::size(10.0, 10.0)),
                        );
                    }
                    views.section(&mut doc, root, "panel", &(), |_, _| {
                        let mut panel = modal_focus_document(&["dialog"], false);
                        let dialog = node(&panel, "dialog.dialog");
                        let alternate = node(&panel, "alternate");
                        panel
                            .node_mut(dialog)
                            .accessibility
                            .as_mut()
                            .unwrap()
                            .modal_focus_restore = FocusRestoreTarget::Node(alternate);
                        panel
                    });
                    doc
                },
            )
            .unwrap();
        assert_eq!(
            session.interaction().focused,
            Some(node(&doc, "dialog.dialog"))
        );
        frame(&mut session, &mut doc, Vec::new());
        let overlay = node(&doc, "dialog");
        doc.node_mut(overlay).style.layout.display = taffy::prelude::Display::None;
        prepare(&mut session, &mut doc);
        frame(&mut session, &mut doc, Vec::new());
        assert_eq!(session.interaction().focused, Some(node(&doc, "alternate")));
        session.retain_document(doc);
    }
}

#[cfg(feature = "widgets")]
#[test]
fn stacked_modal_navigation_survives_frames_and_closing_dialogs() {
    use crate::input::RawKeyboardEvent;
    use crate::widgets::{button, modal_dialog, ButtonOptions, ModalDialogOptions};
    use crate::{KeyCode, KeyModifiers};

    for partition in 0..2 {
        let mut session = RuntimeSession::new();
        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
        let root = doc.root();
        let background = button(
            &mut doc,
            root,
            "background",
            "Background",
            ButtonOptions::default().with_action("background"),
        );
        let back = modal_dialog(
            &mut doc,
            root,
            "back",
            "Back",
            ModalDialogOptions::default()
                .with_size(200.0, 160.0)
                .without_close_button(),
        );
        let back_button = button(
            &mut doc,
            back.body,
            "back.button",
            "Back action",
            ButtonOptions::default().with_action("back"),
        );
        let front = modal_dialog(
            &mut doc,
            root,
            "front",
            "Front",
            ModalDialogOptions::default()
                .with_size(200.0, 160.0)
                .without_close_button(),
        );
        let front_button = button(
            &mut doc,
            front.body,
            "front.button",
            "Front action",
            ButtonOptions::default().with_action("front"),
        );
        for (hide, scope, target) in [
            (None, Some(front.dialog), front_button),
            (Some(front.overlay), Some(back.dialog), back_button),
            (Some(back.overlay), None, background),
        ] {
            if let Some(hide) = hide {
                doc.node_mut(hide).style.layout.display = taffy::prelude::Display::None;
            }
            prepare(&mut session, &mut doc);
            assert_eq!(doc.accessibility_snapshot().modal_scope, scope);
            #[cfg(feature = "diagnostics")]
            {
                use crate::debug::{
                    DebugAccessibilityTreeTrace, DebugFocusNavigationTrace, DebugInspectorSnapshot,
                };
                let snapshot = DebugInspectorSnapshot::from_document(&doc, &mut ApproxTextMeasurer);
                assert_eq!(
                    DebugAccessibilityTreeTrace::from_snapshot(&snapshot).modal_scope,
                    scope
                );
                assert_eq!(
                    DebugFocusNavigationTrace::from_snapshot(&snapshot, None).modal_scope,
                    scope
                );
            }
            let mut actions = Vec::new();
            let mut queued = Vec::new();
            // Modal entry already focuses its container before the first key.
            for (index, key) in [KeyCode::Tab, KeyCode::Enter].into_iter().enumerate() {
                queued.push(RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    key,
                    KeyModifiers::NONE,
                    index as u64,
                )));
                if index == 1 || partition & (1 << index) != 0 {
                    prepare(&mut session, &mut doc);
                    let output = frame(&mut session, &mut doc, std::mem::take(&mut queued));
                    actions.extend(crate::host::collect_document_widget_actions(&output));
                }
            }
            assert!(
                matches!(actions.as_slice(), [action] if action.target == target && matches!(action.kind, WidgetActionKind::Activate(_))),
                "partition={partition}, scope={scope:?}: {actions:?}"
            );
            assert_eq!(session.interaction().focused, Some(target));
        }
    }
}

#[cfg(feature = "widgets")]
#[test]
fn tab_navigation_and_activation_follow_event_order_across_frames() {
    use crate::input::RawKeyboardEvent;
    use crate::widgets::{
        button, checkbox, text_input, ButtonOptions, CheckboxOptions, TextInputOptions,
        TextInputState,
    };
    use crate::{KeyCode, KeyModifiers};

    let shift = KeyModifiers {
        shift: true,
        ..KeyModifiers::NONE
    };
    let events = [
        RawKeyboardEvent::press(KeyCode::Tab, shift, 1).with_text("\t"),
        RawKeyboardEvent::press(KeyCode::Character(' '), KeyModifiers::NONE, 2).with_text(" "),
        RawKeyboardEvent::release(KeyCode::Tab, shift, 3),
        RawKeyboardEvent::press(KeyCode::Tab, KeyModifiers::NONE, 4),
        RawKeyboardEvent::press(KeyCode::Enter, KeyModifiers::NONE, 5).with_text("\r"),
        RawKeyboardEvent::press(KeyCode::Tab, KeyModifiers::NONE, 6).repeat(true),
        RawKeyboardEvent::press(KeyCode::Character('é'), KeyModifiers::NONE, 7).with_text("é"),
        RawKeyboardEvent::press(KeyCode::Tab, shift, 8),
    ];
    for partition in 0..1 << (events.len() - 1) {
        let mut session = RuntimeSession::new();
        let mut doc = UiDocument::new(LayoutStyle::column().with_size(400.0, 300.0));
        let root = doc.root();
        let first = button(
            &mut doc,
            root,
            "first",
            "First",
            ButtonOptions::default().with_action("first"),
        );
        button(
            &mut doc,
            root,
            "disabled",
            "Disabled",
            ButtonOptions::default().with_action("disabled").disabled(),
        );
        let mut text = TextInputState::new("");
        let options = TextInputOptions::default().with_edit_action("edit");
        let field = text_input(&mut doc, root, "field", &mut text, options.clone());
        let last = checkbox(
            &mut doc,
            root,
            "last",
            "Last",
            false,
            CheckboxOptions::default().with_action("last"),
        );
        prepare(&mut session, &mut doc);
        let mut queued = Vec::new();
        let mut activated = Vec::new();
        let mut focused = Vec::new();
        for (index, event) in events.iter().enumerate() {
            queued.push(RawInputEvent::Keyboard(event.clone()));
            if index == events.len() - 1 || partition & (1 << index) != 0 {
                let output = frame(&mut session, &mut doc, std::mem::take(&mut queued));
                for action in crate::host::collect_document_widget_actions(&output) {
                    match action.kind {
                        WidgetActionKind::Activate(_) => activated.push(action.target),
                        WidgetActionKind::Focus(change) => {
                            focused.push((action.target, change.focused))
                        }
                        WidgetActionKind::TextEdit(edit) => {
                            assert!(
                                !matches!(
                                    edit.event,
                                    UiInputEvent::Key {
                                        key: KeyCode::Tab,
                                        ..
                                    }
                                ),
                                "navigation was dispatched as a text edit"
                            );
                            text.apply_widget_text_edit(&edit, &options);
                        }
                        _ => panic!("unexpected action: {action:?}"),
                    }
                }
            }
        }
        assert_eq!(activated, [last, first], "partition={partition}");
        assert_eq!(
            focused,
            [(field, true), (field, false)],
            "partition={partition}"
        );
        assert_eq!(text.text(), "é", "partition={partition}");
        assert_eq!(
            session.interaction().focused,
            Some(first),
            "partition={partition}"
        );
    }
}

#[cfg(feature = "widgets")]
#[test]
fn generated_key_text_and_independent_text_are_applied_once_across_frames() {
    use crate::input::{RawKeyboardEvent, RawTextInputEvent};
    use crate::widgets::{multiline_text_input, TextInputOptions, TextInputState};
    use crate::{KeyCode, KeyModifiers};

    let events = [
        RawInputEvent::Keyboard(
            RawKeyboardEvent::press(KeyCode::Character('a'), KeyModifiers::NONE, 10).with_text("a"),
        ),
        RawInputEvent::Keyboard(
            RawKeyboardEvent::press(KeyCode::Enter, KeyModifiers::NONE, 10).with_text("\r\n"),
        ),
        RawInputEvent::Text(RawTextInputEvent::new("\n", 10)),
        RawInputEvent::Keyboard(
            RawKeyboardEvent::press(KeyCode::Enter, KeyModifiers::NONE, 10)
                .repeat(true)
                .with_text("\r"),
        ),
        RawInputEvent::Keyboard(
            RawKeyboardEvent::release(KeyCode::Enter, KeyModifiers::NONE, 10).with_text("ignored"),
        ),
        RawInputEvent::Keyboard(
            RawKeyboardEvent::press(KeyCode::Character('é'), KeyModifiers::NONE, 10).with_text("é"),
        ),
    ];
    for partition in 0..1 << (events.len() - 1) {
        let mut session = RuntimeSession::new();
        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
        let mut state = TextInputState::new("").multiline(true);
        let options = TextInputOptions::default().with_edit_action("edit");
        let root = doc.root();
        let field = multiline_text_input(&mut doc, root, "field", &mut state, options.clone());
        doc.set_focus_state(UiFocusState {
            focused: Some(field),
            ..Default::default()
        });
        prepare(&mut session, &mut doc);
        let mut queued = Vec::new();
        for (index, event) in events.iter().enumerate() {
            queued.push(event.clone());
            if index == events.len() - 1 || partition & (1 << index) != 0 {
                let output = frame(&mut session, &mut doc, std::mem::take(&mut queued));
                for action in crate::host::collect_document_widget_actions(&output) {
                    if let WidgetActionKind::TextEdit(edit) = action.kind {
                        state.apply_widget_text_edit(&edit, &options);
                    }
                }
            }
        }
        assert_eq!(state.text(), "a\n\n\né", "partition={partition}");
    }
}
