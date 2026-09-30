use super::*;
use crate::input::{RawKeyboardEvent, RawPointerEvent, RawTextInputEvent};
use crate::platform::PlatformRequestIdAllocator;
use crate::renderer::{CanvasHostCapturePlan, RenderTarget};
use crate::runtime::session::RuntimeSession;
use crate::{
    AccessibilityMeta, AccessibilityRole, AnimatedValues, AnimationMachine, AnimationState,
    ApproxTextMeasurer, CanvasInteractionPolicy, KeyCode, KeyModifiers, LayoutStyle,
    UiDocumentScale, UiInputEvent, UiNode, UiSize,
};

const VIEWPORT: UiSize = UiSize::new(400.0, 300.0);

fn document(insert_before: bool) -> (UiDocument, UiNodeId) {
    let mut document = UiDocument::new(LayoutStyle::size(400.0, 300.0));
    if insert_before {
        document.add_child(
            document.root(),
            UiNode::container("new", LayoutStyle::size(10.0, 10.0)),
        );
    }
    let mut canvas = UiNode::canvas(
        "editor",
        "scene",
        LayoutStyle::absolute_rect(UiRect::new(20.0, 30.0, 100.0, 80.0)),
    );
    canvas.content =
        UiContent::Canvas(CanvasContent::new("scene").interaction(CanvasInteractionPolicy::EDITOR));
    let canvas = document.add_child(document.root(), canvas);
    (document, canvas)
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

fn finish(session: &mut RuntimeSession, document: &mut UiDocument, input: HostFrameOutput) {
    session
        .finish_frame(
            document,
            VIEWPORT,
            RenderTarget::window("test", VIEWPORT),
            input,
            &mut ApproxTextMeasurer,
            &mut PlatformRequestIdAllocator::default(),
        )
        .unwrap();
}

fn pointer(kind: PointerEventKind, x: f32, y: f32) -> RawInputEvent {
    RawInputEvent::Pointer(RawPointerEvent::new(kind, UiPoint::new(x, y), 12))
}

#[cfg(feature = "widgets")]
#[test]
fn pointer_observation_tracks_text_ownership_across_rebuild_and_outside_release() {
    #[derive(Default)]
    struct Trace {
        owners: Vec<(Option<String>, Option<String>)>,
        canvas: Vec<RawInputEvent>,
    }
    let make_document = |insert| {
        let (mut doc, _) = document(insert);
        let root = doc.root();
        let field = crate::widgets::singleline_text_input(
            &mut doc,
            root,
            "name",
            &crate::widgets::TextInputState::new("Track"),
            crate::widgets::TextInputOptions::default()
                .with_layout(LayoutStyle::absolute_rect(UiRect::new(
                    20.0, 30.0, 100.0, 30.0,
                )))
                .with_edit_action("edit.name"),
        );
        (doc, field)
    };
    let mut hooks = RuntimeHooks::new()
        .with_pointer_observer(|trace: &mut Trace, input| {
            trace.owners.push((
                input.hit.map(|node| node.name().to_owned()),
                input.captured.map(|node| node.name().to_owned()),
            ));
        })
        .with_canvas_input(|trace: &mut Trace, input| {
            trace.canvas.push(input.input);
            false
        });
    let mut trace = Trace::default();
    let mut session = RuntimeSession::new();
    let (mut doc, old_field) = make_document(false);
    prepare(&mut session, &mut doc);
    let input = session
        .process_input_with_hooks(
            &mut doc,
            VIEWPORT,
            &[
                pointer(PointerEventKind::Move, 30.0, 40.0),
                pointer(PointerEventKind::Down(PointerButton::Primary), 30.0, 40.0),
                pointer(PointerEventKind::Move, 300.0, 200.0),
            ],
            &[],
            &mut hooks,
            &mut trace,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    assert!(input
        .events
        .iter()
        .filter_map(|event| event.document_result.as_ref())
        .flat_map(|result| &result.actions)
        .any(|action| action.target == old_field
            && matches!(action.kind, crate::WidgetActionKind::TextEdit(_))));
    assert!(
        trace.canvas.is_empty(),
        "the text field owns the press, not the canvas below it"
    );
    finish(&mut session, &mut doc, input);
    session.retain_document(doc);
    let (mut doc, new_field) = make_document(true);
    prepare(&mut session, &mut doc);
    assert_ne!(old_field, new_field);
    let input = session
        .process_input_with_hooks(
            &mut doc,
            VIEWPORT,
            &[
                pointer(PointerEventKind::Up(PointerButton::Primary), 300.0, 200.0),
                pointer(PointerEventKind::Move, 30.0, 90.0),
            ],
            &[],
            &mut hooks,
            &mut trace,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    finish(&mut session, &mut doc, input);
    assert_eq!(
        trace.owners,
        [
            (Some("name".into()), None),
            (Some("name".into()), None),
            (None, Some("name".into())),
            (None, Some("name".into())),
            (Some("editor".into()), None),
        ]
    );
    assert!(session.interaction().pressed.is_none());
    assert!(
        matches!(trace.canvas.as_slice(), [RawInputEvent::Pointer(event)]
        if event.kind == PointerEventKind::Move)
    );
}

#[test]
fn pointer_observation_resolves_action_owners_and_respects_blocking_and_modals() {
    for obstruction in ["none", "disabled", "blocked", "modal"] {
        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
        let button = doc.add_child(
            doc.root(),
            UiNode::container("button", LayoutStyle::size(100.0, 60.0))
                .with_input(crate::InputBehavior::BUTTON)
                .with_action("activate"),
        );
        doc.add_child(
            button,
            UiNode::container("label", LayoutStyle::size(100.0, 60.0)).with_input(
                crate::InputBehavior {
                    pointer: true,
                    focusable: false,
                    keyboard: false,
                },
            ),
        );
        match obstruction {
            "disabled" => doc
                .node_mut(button)
                .set_accessibility(AccessibilityMeta::new(AccessibilityRole::Button).disabled()),
            "blocked" => {
                doc.add_child(
                    doc.root(),
                    UiNode::container(
                        "overlay",
                        LayoutStyle::absolute_rect(UiRect::new(0.0, 0.0, 100.0, 60.0)),
                    )
                    .with_hit_test_behavior(HitTestBehavior::Block),
                );
            }
            "modal" => {
                doc.add_child(
                    doc.root(),
                    UiNode::container(
                        "dialog",
                        LayoutStyle::absolute_rect(UiRect::new(200.0, 200.0, 100.0, 80.0)),
                    )
                    .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Dialog).modal()),
                );
            }
            _ => {}
        }
        let mut session = RuntimeSession::new();
        prepare(&mut session, &mut doc);
        let mut seen = Vec::new();
        let mut hooks =
            RuntimeHooks::new().with_pointer_observer(|seen: &mut Vec<Option<String>>, input| {
                seen.push(
                    input
                        .hit
                        .and_then(|node| node.action())
                        .and_then(|binding| binding.action_id())
                        .map(|id| id.as_str().to_owned()),
                );
            });
        let input = session
            .process_input_with_hooks(
                &mut doc,
                VIEWPORT,
                &[
                    pointer(PointerEventKind::Move, 20.0, 20.0),
                    pointer(PointerEventKind::Down(PointerButton::Primary), 20.0, 20.0),
                    pointer(PointerEventKind::Up(PointerButton::Primary), 20.0, 20.0),
                ],
                &[],
                &mut hooks,
                &mut seen,
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        let activations = input
            .events
            .iter()
            .filter_map(|event| event.document_result.as_ref())
            .flat_map(|result| &result.actions)
            .filter(|action| matches!(action.kind, crate::WidgetActionKind::Activate(_)))
            .count();
        let expected = (obstruction == "none").then(|| "activate".to_owned());
        assert_eq!(seen, vec![expected; 3], "{obstruction}");
        assert_eq!(
            activations,
            usize::from(obstruction == "none"),
            "{obstruction}"
        );
    }
}

#[test]
fn consumed_canvas_drag_survives_queued_events_rebuild_and_outside_release() {
    let mut session = RuntimeSession::new();
    let mut seen = Vec::<CanvasInput>::new();
    let mut hooks = RuntimeHooks::new().with_canvas_input(|seen: &mut Vec<CanvasInput>, input| {
        seen.push(input);
        true
    });
    let (mut first, old_node) = document(false);
    prepare(&mut session, &mut first);
    let input = session
        .process_input_with_hooks(
            &mut first,
            VIEWPORT,
            &[
                pointer(PointerEventKind::Down(PointerButton::Primary), 30.0, 40.0),
                pointer(PointerEventKind::Move, 300.0, 200.0),
            ],
            &[],
            &mut hooks,
            &mut seen,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    assert!(input.ui_events().next().is_none());
    assert!(input.gestures().next().is_none());
    assert_eq!(
        seen.len(),
        2,
        "outside move in the same queue belongs to the consumed press"
    );
    assert_eq!(seen[1].node, Some(old_node));
    finish(&mut session, &mut first, input);
    session.retain_document(first);
    session.frame_presented();

    let (mut next, new_node) = document(true);
    next.node_mut(new_node).style.layout =
        LayoutStyle::absolute_rect(UiRect::new(50.0, 60.0, 120.0, 90.0)).style;
    prepare(&mut session, &mut next);
    assert_ne!(new_node, old_node);
    let input = session
        .process_input_with_hooks(
            &mut next,
            VIEWPORT,
            &[
                pointer(PointerEventKind::Move, 320.0, 230.0),
                pointer(PointerEventKind::Up(PointerButton::Primary), 320.0, 230.0),
                pointer(PointerEventKind::Move, 350.0, 250.0),
            ],
            &[],
            &mut hooks,
            &mut seen,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    assert_eq!(
        seen.len(),
        4,
        "release ends ownership; later outside moves are unclaimed"
    );
    assert_eq!(seen[2].node, Some(new_node));
    assert_eq!(seen[2].local_position, Some(UiPoint::new(270.0, 170.0)));
    assert_eq!(seen[3].key, "scene");
    assert!(matches!(
        input.ui_events().cloned().collect::<Vec<_>>().as_slice(),
        [UiInputEvent::PointerCancel, UiInputEvent::PointerMove(_)]
    ));
    assert!(input
        .gestures()
        .all(|gesture| !matches!(gesture, crate::GestureEvent::Click(_))));
}

#[test]
fn removed_disabled_or_rebound_canvas_cancels_once_without_transferring_ownership() {
    for loss in [
        "removed",
        "disabled",
        "blocked",
        "pass-through",
        "rebound",
        "ancestor",
        "modal",
        "policy",
    ] {
        let mut session = RuntimeSession::new();
        let mut seen = Vec::<CanvasInput>::new();
        let mut hooks =
            RuntimeHooks::new().with_canvas_input(|seen: &mut Vec<CanvasInput>, input| {
                seen.push(input);
                true
            });
        let (mut first, _) = document(false);
        prepare(&mut session, &mut first);
        let input = session
            .process_input_with_hooks(
                &mut first,
                VIEWPORT,
                &[pointer(
                    PointerEventKind::Down(PointerButton::Primary),
                    30.0,
                    40.0,
                )],
                &[],
                &mut hooks,
                &mut seen,
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        finish(&mut session, &mut first, input);
        let (mut next, node) = document(true);
        match loss {
            "policy" => {
                let UiContent::Canvas(canvas) = &mut next.node_mut(node).content else {
                    unreachable!()
                };
                canvas.interaction = CanvasInteractionPolicy::NONE;
            }
            "modal" => {
                next.add_child(
                    next.root(),
                    UiNode::container("dialog", LayoutStyle::size(80.0, 60.0)).with_accessibility(
                        AccessibilityMeta::new(AccessibilityRole::Dialog)
                            .modal()
                            .focusable(),
                    ),
                );
            }
            "removed" => {
                next = UiDocument::new(LayoutStyle::size(400.0, 300.0));
            }
            "disabled" => {
                next.node_mut(node)
                    .set_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).disabled());
            }
            "blocked" => next
                .node_mut(node)
                .set_hit_test_behavior(HitTestBehavior::Block),
            "pass-through" => next
                .node_mut(node)
                .set_hit_test_behavior(HitTestBehavior::PassThrough),
            "rebound" => {
                let UiContent::Canvas(canvas) = &mut next.node_mut(node).content else {
                    unreachable!()
                };
                canvas.key = "another operation".to_owned();
            }
            "ancestor" => {
                let root = next.root();
                next.node_mut(root)
                    .set_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).disabled());
            }
            _ => unreachable!(),
        }
        prepare(&mut session, &mut next);
        session.reconcile_input_hooks(&next, &mut hooks, &mut seen);
        session.reconcile_input_hooks(&next, &mut hooks, &mut seen);
        assert_eq!(seen.len(), 2, "{loss}");
        assert_eq!(seen[1].node, None, "{loss}: no stale node ID");
        assert_eq!(seen[1].key, "scene");
        assert!(matches!(
            seen[1].input,
            RawInputEvent::Pointer(RawPointerEvent {
                kind: PointerEventKind::Cancel,
                ..
            })
        ));
        session
            .process_input_with_hooks(
                &mut next,
                VIEWPORT,
                &[pointer(
                    PointerEventKind::Up(PointerButton::Primary),
                    350.0,
                    250.0,
                )],
                &[],
                &mut hooks,
                &mut seen,
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        assert_eq!(
            seen.len(),
            2,
            "{loss}: outside release must not reach a replacement"
        );
    }
}

#[test]
fn cached_canvas_capture_rechecks_in_place_identity_changes() {
    use crate::core::document::view_fragment::ViewFragment;
    for mutation in [
        "rename",
        "edit",
        "ancestor",
        "duplicate",
        "fragment",
        "truncate",
    ] {
        let mut session = RuntimeSession::new();
        let mut seen = Vec::<CanvasInput>::new();
        let mut hooks =
            RuntimeHooks::new().with_canvas_input(|seen: &mut Vec<CanvasInput>, input| {
                seen.push(input);
                true
            });
        let (mut doc, canvas) = document(false);
        prepare(&mut session, &mut doc);
        let input = session
            .process_input_with_hooks(
                &mut doc,
                VIEWPORT,
                &[pointer(
                    PointerEventKind::Down(PointerButton::Primary),
                    30.0,
                    40.0,
                )],
                &[],
                &mut hooks,
                &mut seen,
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        finish(&mut session, &mut doc, input);
        match mutation {
            "rename" => doc.node_mut(canvas).name = "replacement".into(),
            "edit" => doc.edit_node(canvas, |node| node.name = "replacement".into()),
            "ancestor" => {
                let root = doc.root();
                doc.node_mut(root).name = "another-root".into();
            }
            "duplicate" => {
                doc.add_child(
                    doc.root(),
                    UiNode::container("editor", LayoutStyle::size(10.0, 10.0)),
                );
            }
            "fragment" => {
                let fragment =
                    ViewFragment::from_document(UiDocument::new(LayoutStyle::size(10.0, 10.0)));
                doc.append_view_fragment(doc.root(), "editor".into(), &fragment, true);
            }
            "truncate" => doc.truncate_runtime_nodes(1),
            _ => unreachable!(),
        }
        // Custom hosts can edit a live document before the next input batch.
        session.reconcile_input_hooks(&doc, &mut hooks, &mut seen);
        session.reconcile_input_hooks(&doc, &mut hooks, &mut seen);
        assert_eq!(
            seen.len(),
            2,
            "{mutation}: cancel the original operation once"
        );
        assert_eq!(seen[1].node, None);
        assert_eq!(seen[1].key, "scene");
        assert!(matches!(
            seen[1].input,
            RawInputEvent::Pointer(RawPointerEvent {
                kind: PointerEventKind::Cancel,
                ..
            })
        ));
        session
            .process_input_with_hooks(
                &mut doc,
                VIEWPORT,
                &[pointer(PointerEventKind::Move, 350.0, 250.0)],
                &[],
                &mut hooks,
                &mut seen,
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        assert_eq!(
            seen.len(),
            2,
            "{mutation}: no stale capture after cancellation"
        );
    }
}

#[test]
fn keyboard_hooks_observe_focus_in_event_order_across_batches_and_rebuilds() {
    let describe = |inserted| {
        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
        if inserted {
            doc.add_child(
                doc.root(),
                UiNode::container("decoration", LayoutStyle::size(0.0, 0.0)),
            );
        }
        for (name, x) in [("first", 0.0), ("second", 100.0)] {
            doc.add_child(
                doc.root(),
                UiNode::container(
                    name,
                    LayoutStyle::absolute_rect(UiRect::new(x, 0.0, 80.0, 30.0)),
                )
                .with_input(crate::InputBehavior::BUTTON)
                .with_action(name),
            );
        }
        doc
    };
    let key = |key| RawInputEvent::Keyboard(RawKeyboardEvent::press(key, KeyModifiers::NONE, 12));
    let events = [
        key(KeyCode::Enter),
        pointer(PointerEventKind::Down(PointerButton::Primary), 10.0, 10.0),
        pointer(PointerEventKind::Up(PointerButton::Primary), 10.0, 10.0),
        key(KeyCode::Tab),
        key(KeyCode::Enter),
        pointer(PointerEventKind::Down(PointerButton::Primary), 300.0, 200.0),
        pointer(PointerEventKind::Up(PointerButton::Primary), 300.0, 200.0),
        key(KeyCode::Enter),
    ];
    for partition in [0, 0b1111111, 0b0000100, 0b0001000] {
        let mut session = RuntimeSession::new();
        let mut doc = describe(false);
        prepare(&mut session, &mut doc);
        let mut seen = Vec::<(KeyCode, Option<String>)>::new();
        let mut hooks = RuntimeHooks::new().with_keyboard_input(
            |seen: &mut Vec<(KeyCode, Option<String>)>, input| {
                let focused = input
                    .focused
                    .and_then(|node| node.action())
                    .and_then(|binding| binding.action_id())
                    .map(|id| id.as_str().to_owned());
                seen.push((input.event.key, focused.clone()));
                // An application Enter shortcut yields to focused controls.
                input.event.key == KeyCode::Enter && focused.is_none()
            },
        );
        let mut pending = Vec::new();
        let mut activations = Vec::new();
        for (index, event) in events.iter().enumerate() {
            pending.push(event.clone());
            if index == events.len() - 1 || partition & (1 << index) != 0 {
                let output = session
                    .process_input_with_hooks(
                        &mut doc,
                        VIEWPORT,
                        &pending,
                        &[],
                        &mut hooks,
                        &mut seen,
                        &mut ApproxTextMeasurer,
                    )
                    .unwrap();
                activations.extend(
                    output
                        .events
                        .iter()
                        .filter_map(|event| event.document_result.as_ref())
                        .flat_map(|result| result.actions.iter())
                        .filter(|action| {
                            matches!(action.kind, crate::WidgetActionKind::Activate(_))
                        })
                        .map(|action| action.binding.action_id().unwrap().as_str().to_owned()),
                );
                finish(&mut session, &mut doc, output);
                pending.clear();
                doc = describe(index % 2 == 0);
                prepare(&mut session, &mut doc);
            }
        }
        assert_eq!(
            seen,
            [
                (KeyCode::Enter, None),
                (KeyCode::Tab, Some("first".into())),
                (KeyCode::Enter, Some("second".into())),
                (KeyCode::Enter, None),
            ],
            "partition={partition}"
        );
        assert_eq!(activations, ["first", "second"], "partition={partition}");
    }
}

#[test]
fn consumed_keyboard_suppresses_only_its_paired_text_and_preserves_enter_dispatch() {
    let mut session = RuntimeSession::new();
    let (mut doc, _) = document(false);
    prepare(&mut session, &mut doc);
    let mut keys = Vec::new();
    let mut hooks = RuntimeHooks::new().with_keyboard_input(|keys: &mut Vec<KeyCode>, key| {
        keys.push(key.event.key);
        key.event.key == KeyCode::Character('x')
    });
    let input = session
        .process_input_with_hooks(
            &mut doc,
            VIEWPORT,
            &[
                RawInputEvent::Keyboard(
                    RawKeyboardEvent::press(KeyCode::Character('x'), KeyModifiers::NONE, 10)
                        .with_text("x"),
                ),
                RawInputEvent::Text(RawTextInputEvent::new("independent IME commit", 11)),
                RawInputEvent::Keyboard(
                    RawKeyboardEvent::press(KeyCode::Enter, KeyModifiers::NONE, 12).with_text("\n"),
                ),
            ],
            &[],
            &mut hooks,
            &mut keys,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    assert_eq!(keys, [KeyCode::Character('x'), KeyCode::Enter]);
    assert_eq!(
        input.ui_events().cloned().collect::<Vec<_>>(),
        [
            UiInputEvent::TextInput("independent IME commit".to_owned()),
            UiInputEvent::Key {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE
            },
        ]
    );
}

#[test]
fn canvas_coordinates_undo_paint_transform_and_cancel_releases_capture() {
    let mut session = RuntimeSession::new();
    let (mut doc, node) = document(false);
    prepare(&mut session, &mut doc);
    let mut seen = Vec::<CanvasInput>::new();
    let mut hooks = RuntimeHooks::new().with_canvas_input(|seen: &mut Vec<CanvasInput>, input| {
        seen.push(input);
        true
    });
    let input = session
        .process_input_with_hooks(
            &mut doc,
            VIEWPORT,
            &[pointer(
                PointerEventKind::Down(PointerButton::Primary),
                30.0,
                40.0,
            )],
            &[],
            &mut hooks,
            &mut seen,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    assert_eq!(seen[0].local_position, Some(UiPoint::new(10.0, 10.0)));
    finish(&mut session, &mut doc, input);
    for (scale, translation) in [
        (2.0, UiPoint::new(10.0, 20.0)),
        (0.5, UiPoint::new(-10.0, -20.0)),
        (-2.0, UiPoint::new(200.0, 200.0)),
        (0.0, UiPoint::new(10.0, 20.0)),
    ] {
        doc.node_mut(node).style.layout =
            LayoutStyle::absolute_rect(UiRect::new(50.0, 60.0, 120.0, 90.0)).style;
        doc.node_mut(node).animation = Some(
            AnimationMachine::new(
                vec![AnimationState::new(
                    "zoom",
                    AnimatedValues::new(1.0, translation, scale),
                )],
                Vec::new(),
                "zoom",
            )
            .unwrap(),
        );
        prepare(&mut session, &mut doc);
        let paint = doc.paint_list();
        let painted = paint.items.iter().find(|item| item.node == node).unwrap();
        let local = UiPoint::new(-15.0, 250.0);
        let point = painted.transform.transform_point(UiPoint::new(
            painted.rect.x + local.x,
            painted.rect.y + local.y,
        ));
        let previous = seen.len();
        let input = session
            .process_input_with_hooks(
                &mut doc,
                VIEWPORT,
                &[pointer(PointerEventKind::Move, point.x, point.y)],
                &[],
                &mut hooks,
                &mut seen,
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        assert_eq!(seen.len(), previous + 1);
        assert_eq!(seen.last().unwrap().rect, painted.rect);
        assert_eq!(
            seen.last().unwrap().local_position,
            (scale != 0.0).then_some(local),
            "scale={scale}, translation={translation:?}"
        );
        finish(&mut session, &mut doc, input);
    }
    let previous = seen.len();
    session
        .process_input_with_hooks(
            &mut doc,
            VIEWPORT,
            &[
                pointer(PointerEventKind::Cancel, 350.0, 250.0),
                pointer(PointerEventKind::Move, 350.0, 250.0),
            ],
            &[],
            &mut hooks,
            &mut seen,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    assert_eq!(seen.len(), previous + 1);
    assert!(matches!(
        seen.last().unwrap().input,
        RawInputEvent::Pointer(RawPointerEvent {
            kind: PointerEventKind::Cancel,
            ..
        })
    ));
}

#[test]
fn intercepted_release_cancels_widget_gestures_and_clears_pressed_state() {
    for terminal in [
        PointerEventKind::Up(PointerButton::Primary),
        PointerEventKind::Cancel,
    ] {
        let mut session = RuntimeSession::new();
        let (mut doc, _) = document(false);
        prepare(&mut session, &mut doc);
        let mut hooks = RuntimeHooks::new().with_canvas_input(|_: &mut (), input| {
            matches!(
                input.input,
                RawInputEvent::Pointer(RawPointerEvent {
                    kind: PointerEventKind::Up(_) | PointerEventKind::Cancel,
                    ..
                })
            )
        });
        let input = session
            .process_input_with_hooks(
                &mut doc,
                VIEWPORT,
                &[
                    pointer(PointerEventKind::Down(PointerButton::Primary), 30.0, 40.0),
                    pointer(PointerEventKind::Move, 60.0, 40.0),
                    pointer(terminal, 70.0, 40.0),
                ],
                &[],
                &mut hooks,
                &mut (),
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        assert!(input.state.drag_capture.is_none());
        assert!(input
            .state
            .gesture_tracker
            .active_capture(PointerId::MOUSE)
            .is_none());
        assert!(
            matches!(input.gestures().next_back(), Some(crate::GestureEvent::Drag(drag)) if drag.phase == crate::GesturePhase::Cancel)
        );
        finish(&mut session, &mut doc, input);
        assert_eq!(doc.focus_state().pressed, None, "{terminal:?}");
        assert_eq!(session.interaction().pressed, None, "{terminal:?}");
        let next = session
            .process_input_with_hooks(
                &mut doc,
                VIEWPORT,
                &[pointer(PointerEventKind::Move, 350.0, 250.0)],
                &[],
                &mut hooks,
                &mut (),
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        assert!(next
            .gestures()
            .all(|gesture| !matches!(gesture, crate::GestureEvent::Drag(_))));
    }
}

#[test]
fn automatic_scrollbar_drag_is_not_intercepted_by_canvas_hooks() {
    let events = [
        pointer(PointerEventKind::Down(PointerButton::Primary), 119.0, 42.0),
        pointer(PointerEventKind::Move, 60.0, 100.0),
        pointer(PointerEventKind::Up(PointerButton::Primary), 60.0, 100.0),
    ];
    for partition in 0..4 {
        let mut session = RuntimeSession::new();
        let mut seen = Vec::<CanvasInput>::new();
        let mut hooks =
            RuntimeHooks::new().with_canvas_input(|seen: &mut Vec<CanvasInput>, input| {
                seen.push(input);
                true
            });
        let (mut doc, canvas) = document(false);
        let scroll = doc
            .node(canvas)
            .clone()
            .with_scroll(crate::ScrollAxes::VERTICAL);
        *doc.node_mut(canvas) = scroll;
        doc.add_child(
            canvas,
            UiNode::container(
                "content",
                LayoutStyle::size(60.0, 300.0).with_flex_shrink(0.0),
            ),
        );
        let mut queued = Vec::new();
        for (index, event) in events.iter().enumerate() {
            queued.push(event.clone());
            if index == events.len() - 1 || partition & (1 << index) != 0 {
                prepare(&mut session, &mut doc);
                let input = session
                    .process_input_with_hooks(
                        &mut doc,
                        VIEWPORT,
                        &queued,
                        &[],
                        &mut hooks,
                        &mut seen,
                        &mut ApproxTextMeasurer,
                    )
                    .unwrap();
                finish(&mut session, &mut doc, input);
                queued.clear();
            }
        }
        assert!(
            seen.is_empty(),
            "scrollbar drag reached canvas hook: partition={partition}"
        );
        assert!(doc.scroll_state(canvas).unwrap().offset.y > 0.0);
        assert!(session.interaction().drag_capture.is_none());
        let input = session
            .process_input_with_hooks(
                &mut doc,
                VIEWPORT,
                &[pointer(
                    PointerEventKind::Down(PointerButton::Primary),
                    60.0,
                    60.0,
                )],
                &[],
                &mut hooks,
                &mut seen,
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        finish(&mut session, &mut doc, input);
        assert_eq!(
            seen.len(),
            1,
            "ordinary canvas input still reaches its hook"
        );
    }
}

#[test]
fn canvas_hooks_hit_test_after_preceding_scroll_events() {
    let events = [
        RawInputEvent::Wheel(crate::input::RawWheelEvent::pixels(
            UiPoint::new(20.0, 10.0),
            UiPoint::new(0.0, 40.0),
            1,
        )),
        pointer(PointerEventKind::Down(PointerButton::Primary), 20.0, 10.0),
        pointer(PointerEventKind::Up(PointerButton::Primary), 20.0, 10.0),
    ];
    for partition in 0..4 {
        let mut session = RuntimeSession::new();
        let mut doc = UiDocument::new(LayoutStyle::size(100.0, 80.0));
        let scroll = doc.add_child(
            doc.root(),
            UiNode::container("scroll", LayoutStyle::column().with_size(100.0, 80.0))
                .with_scroll(crate::ScrollAxes::VERTICAL),
        );
        let content = doc.add_child(
            scroll,
            UiNode::container(
                "content",
                LayoutStyle::column()
                    .with_size(80.0, 240.0)
                    .with_flex_shrink(0.0),
            ),
        );
        for key in ["first", "second", "third"] {
            let mut canvas = UiNode::canvas(
                key,
                key,
                LayoutStyle::size(80.0, 40.0).with_flex_shrink(0.0),
            );
            canvas.content = UiContent::Canvas(
                CanvasContent::new(key).interaction(CanvasInteractionPolicy::EDITOR),
            );
            doc.add_child(content, canvas);
        }
        let mut seen = Vec::<CanvasInput>::new();
        let mut hooks =
            RuntimeHooks::new().with_canvas_input(|seen: &mut Vec<CanvasInput>, input| {
                let consumed = matches!(input.input, RawInputEvent::Pointer(_));
                seen.push(input);
                consumed
            });
        let mut queued = Vec::new();
        for (index, event) in events.iter().enumerate() {
            queued.push(event.clone());
            if index == events.len() - 1 || partition & (1 << index) != 0 {
                prepare(&mut session, &mut doc);
                let input = session
                    .process_input_with_hooks(
                        &mut doc,
                        VIEWPORT,
                        &queued,
                        &[],
                        &mut hooks,
                        &mut seen,
                        &mut ApproxTextMeasurer,
                    )
                    .unwrap();
                finish(&mut session, &mut doc, input);
                queued.clear();
            }
        }
        assert_eq!(
            seen.iter()
                .map(|input| input.key.as_str())
                .collect::<Vec<_>>(),
            ["first", "second", "second"],
            "partition={partition}"
        );
        assert_eq!(seen[1].local_position, Some(UiPoint::new(20.0, 10.0)));
    }
}

#[test]
fn keyboard_hook_does_not_consume_unrelated_text_with_the_same_timestamp() {
    let mut session = RuntimeSession::new();
    let (mut doc, _) = document(false);
    prepare(&mut session, &mut doc);
    let mut hooks =
        RuntimeHooks::new().with_keyboard_input(|_: &mut (), key| key.event.key == KeyCode::Escape);
    let input = session
        .process_input_with_hooks(
            &mut doc,
            VIEWPORT,
            &[
                RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    KeyCode::Escape,
                    KeyModifiers::NONE,
                    10,
                )),
                RawInputEvent::Text(RawTextInputEvent::new("independent text", 10)),
            ],
            &[],
            &mut hooks,
            &mut (),
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    assert_eq!(
        input.ui_events().cloned().collect::<Vec<_>>(),
        [UiInputEvent::TextInput("independent text".into())]
    );
}

#[test]
fn keyboard_and_canvas_hooks_can_consume_tab_before_navigation() {
    for canvas_hook in [false, true] {
        let mut session = RuntimeSession::new();
        let (mut doc, canvas) = document(false);
        let next = doc.add_child(
            doc.root(),
            UiNode::container("next", LayoutStyle::size(80.0, 30.0))
                .with_input(crate::InputBehavior::BUTTON),
        );
        doc.set_focus_state(crate::UiFocusState {
            focused: Some(canvas),
            ..Default::default()
        });
        prepare(&mut session, &mut doc);
        let mut hooks = if canvas_hook {
            RuntimeHooks::new().with_canvas_input(|consume: &mut bool, event| {
                *consume && matches!(event.input, RawInputEvent::Keyboard(key) if key.key == KeyCode::Tab)
            })
        } else {
            RuntimeHooks::new().with_keyboard_input(|consume: &mut bool, key| {
                *consume && key.event.key == KeyCode::Tab
            })
        };
        for (mut consume, expected) in [(true, canvas), (false, next)] {
            let input = session
                .process_input_with_hooks(
                    &mut doc,
                    VIEWPORT,
                    &[RawInputEvent::Keyboard(
                        RawKeyboardEvent::press(KeyCode::Tab, KeyModifiers::NONE, 1)
                            .with_text("\t"),
                    )],
                    &[],
                    &mut hooks,
                    &mut consume,
                    &mut ApproxTextMeasurer,
                )
                .unwrap();
            assert_eq!(input.events.is_empty(), consume);
            finish(&mut session, &mut doc, input);
            assert_eq!(
                session.interaction().focused,
                Some(expected),
                "canvas_hook={canvas_hook}, consume={consume}"
            );
        }
    }
}

#[test]
fn keyboard_and_canvas_hooks_consume_key_text_as_one_event() {
    let events = [
        RawInputEvent::Keyboard(
            RawKeyboardEvent::press(KeyCode::Character('x'), KeyModifiers::NONE, 10).with_text("x"),
        ),
        RawInputEvent::Text(RawTextInputEvent::new("independent", 10)),
        RawInputEvent::Keyboard(
            RawKeyboardEvent::press(KeyCode::Character('y'), KeyModifiers::NONE, 10).with_text("y"),
        ),
    ];
    for canvas_hook in [false, true] {
        for partition in 0..4 {
            let mut session = RuntimeSession::new();
            let (mut doc, canvas) = document(false);
            doc.set_focus_state(crate::UiFocusState {
                focused: Some(canvas),
                ..Default::default()
            });
            prepare(&mut session, &mut doc);
            let mut hooks = if canvas_hook {
                RuntimeHooks::new().with_canvas_input(|_: &mut (), input| {
                    matches!(input.input, RawInputEvent::Keyboard(key) if key.key == KeyCode::Character('x'))
                })
            } else {
                RuntimeHooks::new()
                    .with_keyboard_input(|_: &mut (), key| key.event.key == KeyCode::Character('x'))
            };
            let mut delivered = Vec::new();
            let mut queued = Vec::new();
            for (index, event) in events.iter().enumerate() {
                queued.push(event.clone());
                if index == events.len() - 1 || partition & (1 << index) != 0 {
                    let input = session
                        .process_input_with_hooks(
                            &mut doc,
                            VIEWPORT,
                            &queued,
                            &[],
                            &mut hooks,
                            &mut (),
                            &mut ApproxTextMeasurer,
                        )
                        .unwrap();
                    delivered.extend(input.ui_events().cloned());
                    finish(&mut session, &mut doc, input);
                    queued.clear();
                }
            }
            assert_eq!(
                delivered,
                [
                    UiInputEvent::TextInput("independent".into()),
                    UiInputEvent::Key {
                        key: KeyCode::Character('y'),
                        modifiers: KeyModifiers::NONE
                    },
                    UiInputEvent::TextInput("y".into()),
                ],
                "canvas_hook={canvas_hook}, partition={partition}"
            );
        }
    }
}

#[test]
fn retained_canvas_input_uses_current_policy_before_finishing_the_frame() {
    use crate::input::RawWheelEvent;
    for previous in [
        CanvasInteractionPolicy::EDITOR,
        CanvasInteractionPolicy::NATIVE_VIEWPORT,
        CanvasInteractionPolicy {
            pointer_capture: true,
            ..CanvasInteractionPolicy::NONE
        },
    ] {
        for mask in 0..16 {
            let policy = CanvasInteractionPolicy {
                pointer_capture: mask & 1 != 0,
                keyboard_capture: mask & 2 != 0,
                wheel_capture: mask & 4 != 0,
                pointer_lock: mask & 8 != 0,
                domain_hit_testing: true,
            };
            for direct in [false, true] {
                let mut session = RuntimeSession::new();
                let mut seen = Vec::<CanvasInput>::new();
                let mut hooks =
                    RuntimeHooks::new().with_canvas_input(|seen: &mut Vec<CanvasInput>, input| {
                        seen.push(input);
                        true
                    });
                let (mut first, canvas) = document(false);
                first.set_node_content(
                    canvas,
                    UiContent::Canvas(CanvasContent::new("scene").interaction(previous)),
                );
                prepare(&mut session, &mut first);
                let input = session
                    .process_input_with_hooks(
                        &mut first,
                        VIEWPORT,
                        &[],
                        &[],
                        &mut hooks,
                        &mut seen,
                        &mut ApproxTextMeasurer,
                    )
                    .unwrap();
                finish(&mut session, &mut first, input);

                let (mut next, canvas) = document(true);
                next.set_node_content(
                    canvas,
                    UiContent::Canvas(CanvasContent::new("scene").interaction(policy)),
                );
                next.set_focus_state(crate::UiFocusState {
                    focused: direct.then_some(canvas),
                    ..Default::default()
                });
                prepare(&mut session, &mut next);
                assert_eq!(session.interaction().focused, direct.then_some(canvas));
                let point = if direct {
                    UiPoint::new(30.0, 40.0)
                } else {
                    UiPoint::new(350.0, 250.0)
                };
                let events = [
                    pointer(PointerEventKind::Move, point.x, point.y),
                    RawInputEvent::Wheel(RawWheelEvent::pixels(point, UiPoint::new(0.0, 12.0), 13)),
                    RawInputEvent::Keyboard(
                        RawKeyboardEvent::press(KeyCode::Character('x'), KeyModifiers::NONE, 14)
                            .with_text("x"),
                    ),
                    RawInputEvent::Text(RawTextInputEvent::new("paste", 15)),
                ];
                session
                    .process_input_with_hooks(
                        &mut next,
                        VIEWPORT,
                        &events,
                        &[],
                        &mut hooks,
                        &mut seen,
                        &mut ApproxTextMeasurer,
                    )
                    .unwrap();
                let expected = events
                    .iter()
                    .filter(|event| match event {
                        RawInputEvent::Pointer(_) => {
                            direct && (policy.pointer_capture || policy.pointer_lock)
                        }
                        RawInputEvent::Wheel(_) => policy.wheel_capture,
                        RawInputEvent::Keyboard(_) | RawInputEvent::Text(_) => {
                            policy.keyboard_capture
                        }
                        _ => unreachable!(),
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                assert_eq!(
                    seen.iter()
                        .map(|input| input.input.clone())
                        .collect::<Vec<_>>(),
                    expected,
                    "previous={previous:?}, policy={policy:?}, direct={direct}"
                );
                assert!(seen
                    .iter()
                    .all(|input| input.node == Some(canvas) && input.key == "scene"));
            }
        }
    }
}

#[test]
fn retained_canvas_fallback_does_not_transfer_to_an_ancestor_with_the_same_key() {
    use crate::input::RawWheelEvent;
    let build = |replaced: bool| {
        let mut doc = UiDocument::new(LayoutStyle::size(400.0, 300.0));
        let parent = doc.add_child(
            doc.root(),
            UiNode::canvas("parent", "scene", LayoutStyle::size(180.0, 140.0)),
        );
        doc.set_node_content(
            parent,
            UiContent::Canvas(CanvasContent::new("scene").interaction(if replaced {
                CanvasInteractionPolicy::EDITOR
            } else {
                CanvasInteractionPolicy::NONE
            })),
        );
        let child = if replaced {
            UiNode::container("child", LayoutStyle::size(80.0, 60.0))
        } else {
            let mut child = UiNode::canvas("child", "scene", LayoutStyle::size(80.0, 60.0));
            child.content = UiContent::Canvas(
                CanvasContent::new("scene").interaction(CanvasInteractionPolicy::EDITOR),
            );
            child
        };
        doc.add_child(parent, child);
        (doc, parent)
    };
    let mut session = RuntimeSession::new();
    let mut seen = Vec::<CanvasInput>::new();
    let mut hooks = RuntimeHooks::new().with_canvas_input(|seen: &mut Vec<CanvasInput>, input| {
        seen.push(input);
        true
    });
    let (mut first, _) = build(false);
    prepare(&mut session, &mut first);
    let input = session
        .process_input_with_hooks(
            &mut first,
            VIEWPORT,
            &[],
            &[],
            &mut hooks,
            &mut seen,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    finish(&mut session, &mut first, input);
    let events = [
        RawInputEvent::Wheel(RawWheelEvent::pixels(
            UiPoint::new(350.0, 250.0),
            UiPoint::new(0.0, 12.0),
            13,
        )),
        RawInputEvent::Keyboard(RawKeyboardEvent::press(
            KeyCode::Character('x'),
            KeyModifiers::NONE,
            14,
        )),
    ];
    let (mut next, parent) = build(true);
    prepare(&mut session, &mut next);
    let input = session
        .process_input_with_hooks(
            &mut next,
            VIEWPORT,
            &events,
            &[],
            &mut hooks,
            &mut seen,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    assert!(
        seen.is_empty(),
        "a different canvas cannot inherit the child's capture: {seen:?}"
    );
    finish(&mut session, &mut next, input);
    prepare(&mut session, &mut next);
    session
        .process_input_with_hooks(
            &mut next,
            VIEWPORT,
            &events,
            &[],
            &mut hooks,
            &mut seen,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    assert_eq!(
        seen.len(),
        events.len(),
        "the parent can subsequently acquire its own capture"
    );
    assert!(seen.iter().all(|input| input.node == Some(parent)));
}

#[test]
fn modal_keyboard_input_cannot_fall_back_to_background_canvas_capture() {
    use crate::input::RawTextCompositionEvent;
    use crate::platform::TextInputId;
    use crate::TextCompositionEvent;
    for local_canvas in [false, true] {
        let (mut doc, background) = document(false);
        // A dialog may be inside a canvas subtree. Resolving its nearest canvas
        // must not escape back out through that ancestor.
        let modal = doc.add_child(
            background,
            UiNode::container("dialog", LayoutStyle::size(80.0, 60.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Dialog)
                    .modal()
                    .focusable(),
            ),
        );
        let inside = local_canvas.then(|| {
            let mut node = UiNode::canvas("inside", "inside", LayoutStyle::size(60.0, 30.0));
            node.content = UiContent::Canvas(
                CanvasContent::new("inside").interaction(CanvasInteractionPolicy::EDITOR),
            );
            doc.add_child(modal, node)
        });
        let mut state = HostInteractionState::default();
        state
            .canvas_host_capture
            .sync(
                std::iter::once(background)
                    .chain(inside)
                    .map(|id| CanvasHostCapturePlan {
                        node: id,
                        key: if id == background { "scene" } else { "inside" }.into(),
                        rect: UiRect::new(0.0, 0.0, 80.0, 60.0),
                        pointer_capture: false,
                        keyboard_capture: true,
                        wheel_capture: false,
                        pointer_lock: false,
                        domain_hit_testing: false,
                    }),
            );
        let events = [
            RawInputEvent::Keyboard(RawKeyboardEvent::press(
                KeyCode::Enter,
                KeyModifiers::NONE,
                1,
            )),
            RawInputEvent::Text(RawTextInputEvent::new("x", 2)),
            RawInputEvent::Composition(RawTextCompositionEvent {
                input: TextInputId::new("editor"),
                event: TextCompositionEvent::Commit {
                    text: "x".into(),
                    replacement: None,
                },
                timestamp_millis: 3,
            }),
        ];
        for displayed in [true, false, true] {
            doc.node_mut(modal).style.layout.display = if displayed {
                taffy::prelude::Display::Flex
            } else {
                taffy::prelude::Display::None
            };
            doc.compute_layout(VIEWPORT, &mut ApproxTextMeasurer)
                .unwrap();
            let target = if displayed { inside } else { Some(background) };
            let rect = doc.node(target.unwrap_or(modal)).layout().rect;
            let point = UiPoint::new(rect.x + 10.0, rect.y + 10.0);
            for event in [
                RawInputEvent::Pointer(RawPointerEvent::new(
                    PointerEventKind::Down(PointerButton::Primary),
                    point,
                    4,
                )),
                RawInputEvent::Wheel(crate::input::RawWheelEvent::pixels(
                    point,
                    UiPoint::new(0.0, 10.0),
                    5,
                )),
            ] {
                assert_eq!(
                    canvas_input_for_raw_event(&doc, &state, &event).and_then(|input| input.node),
                    target
                );
            }
            if displayed {
                let outside = RawInputEvent::Wheel(crate::input::RawWheelEvent::pixels(
                    UiPoint::new(350.0, 250.0),
                    UiPoint::new(0.0, 10.0),
                    6,
                ));
                assert!(canvas_input_for_raw_event(&doc, &state, &outside).is_none());
            }
            for focused in [None, Some(background), Some(modal), inside] {
                state.focused = focused;
                for event in &events {
                    let input = canvas_input_for_raw_event(&doc, &state, event);
                    assert_eq!(input.and_then(|input| input.node), if displayed { inside } else { Some(background) },
                        "local_canvas={local_canvas}, displayed={displayed}, focused={focused:?}, event={event:?}");
                }
            }
        }
    }
}
