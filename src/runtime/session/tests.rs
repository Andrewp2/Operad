use super::*;
use crate::input::{PointerButton, PointerId, RawPointerEvent};
use crate::{ApproxTextMeasurer, InputBehavior, LayoutStyle, ScrollAxes, UiNode};

const VIEWPORT: UiSize = UiSize::new(400.0, 300.0);

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
    let input = session.process_input(document, VIEWPORT, events, Vec::new());
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
            .gestures
            .iter()
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
        output.input_results[0].clicked,
        Some(node(&next, "original"))
    );
    assert_eq!(session.interaction().pressed, None);
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
fn independent_sessions_do_not_share_state() {
    let mut first_session = RuntimeSession::new();
    let mut first = document(&["a"]);
    prepare(&mut first_session, &mut first);
    press(&mut first_session, &mut first, "a");
    let mut other_session = RuntimeSession::new();
    let mut other = document(&["a"]);
    prepare(&mut other_session, &mut other);
    assert_eq!(other_session.interaction().focused, None);
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
            |viewport| {
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
                |_| panic!("unchanged view rebuilt"),
            )
            .unwrap();
        let input = session.process_input(&doc, VIEWPORT, Vec::new(), Vec::new());
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
            |viewport| {
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
            |_| panic!("scale does not change the authored view"),
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
            |viewport| {
                builds += 1;
                measured_document(viewport, "resized")
            },
        )
        .unwrap();
    assert_eq!(builds, 3, "viewport changes must rebuild the view");
}

#[test]
fn uploads_are_retained_for_failed_frames_and_consumed_after_presentation() {
    use crate::platform::{ImageHandle, PixelSize};
    use crate::renderer::ResourceUpdate;
    let mut session = RuntimeSession::new();
    let mut doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_| {
                let mut doc = document(&["a"]);
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
    let mut retry = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_| panic!("retry should reuse document"),
        )
        .unwrap();
    assert_eq!(
        frame(&mut session, &mut retry, Vec::new())
            .render_request
            .resource_updates
            .len(),
        1
    );
    session.retain_document(retry);
    session.frame_presented();
    let mut next = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_| panic!("redraw should reuse document"),
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
            |_| {
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
            |_| panic!("view should be retained"),
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
            |viewport| {
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
            |_| panic!("hover must not rebuild the view"),
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
            |_| {
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
    for _ in 0..3 {
        let mut doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                Some(UiPoint::new(5.0, 5.0)),
                &mut ApproxTextMeasurer,
                |_| panic!("tooltips should not rebuild the app"),
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
            |_| panic!("moving away should reuse the view"),
        )
        .unwrap();
    frame(&mut session, &mut doc, Vec::new());
    assert_eq!(
        doc.node_count(),
        authored,
        "inactive tooltips must disappear"
    );
}
