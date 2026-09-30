use super::*;
use crate::runtime::session::RuntimeSession;
use crate::{
    ApproxTextMeasurer, AvailableSize, KnownSize, LayoutStyle, TextContent, TextMeasurer,
    TextStyle, UiNode,
};
use std::cell::Cell;

const VIEWPORT: UiSize = UiSize::new(800.0, 400.0);

#[derive(Default)]
struct Measurements(HashMap<String, usize>);

impl TextMeasurer for Measurements {
    fn measure(
        &mut self,
        text: &TextContent,
        known: KnownSize,
        available: AvailableSize,
    ) -> UiSize {
        *self.0.entry(text.text.clone()).or_default() += 1;
        ApproxTextMeasurer.measure(text, known, available)
    }
}

fn panel(value: &usize, _: &mut ViewContext<'_>) -> UiDocument {
    let mut doc = UiDocument::new(LayoutStyle::column().with_size(250.0, 300.0));
    for row in 0..*value {
        doc.add_child(
            doc.root(),
            UiNode::text(
                format!("row-{row}"),
                format!("{value} rows, item {row}"),
                TextStyle::default(),
                LayoutStyle::default(),
            ),
        );
    }
    doc
}

fn describe(
    viewport: UiSize,
    views: &mut ViewContext<'_>,
    value: usize,
    second: bool,
    reverse: bool,
) -> UiDocument {
    let mut doc = UiDocument::new(LayoutStyle::row().with_size(viewport.width, viewport.height));
    let root = doc.root();
    let mut sections = vec![("first", value)];
    if second {
        sections.push(("second", 7));
    }
    if reverse {
        sections.reverse();
    }
    // Change display order while keeping each section's definition unchanged.
    for (name, rows) in sections {
        views.section(&mut doc, root, name, &rows, panel);
    }
    doc
}

#[test]
fn unrelated_panel_reuses_builder_and_measurements_through_changes_and_reordering() {
    let mut session = RuntimeSession::new();
    let mut measurements = Measurements::default();
    for (step, value, reverse, expected) in [
        (
            0,
            2,
            false,
            ViewBuildStats {
                rebuilt: 2,
                reused: 0,
            },
        ),
        (
            1,
            4,
            false,
            ViewBuildStats {
                rebuilt: 1,
                reused: 1,
            },
        ),
        (
            2,
            4,
            true,
            ViewBuildStats {
                rebuilt: 0,
                reused: 2,
            },
        ),
        (
            3,
            1,
            false,
            ViewBuildStats {
                rebuilt: 1,
                reused: 1,
            },
        ),
    ] {
        let before = measurements.0.clone();
        session.invalidate_view();
        let doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut measurements,
                |size, views| describe(size, views, value, true, reverse),
            )
            .unwrap();
        assert_eq!(session.view_build_stats(), expected);
        if step > 0 {
            for (text, count) in &measurements.0 {
                if text.starts_with("7 rows") {
                    assert_eq!(
                        Some(count),
                        before.get(text),
                        "remeasured unchanged sibling: {text}"
                    );
                }
            }
        }
        // A fresh session is the reference for layout, including changes to local
        // IDs, child counts, and sibling position. Compare all authored geometry.
        let mut fresh = RuntimeSession::new();
        let reference = fresh
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |size, views| describe(size, views, value, true, reverse),
            )
            .unwrap();
        for (actual, expected) in doc.nodes().iter().zip(reference.nodes()) {
            assert_eq!(actual.name(), expected.name());
            assert_eq!(
                actual.layout(),
                expected.layout(),
                "step {step}: {}",
                actual.name()
            );
        }
        session.retain_document(doc);
        session.frame_presented();
    }
}

#[test]
fn switching_section_call_sites_rebuilds_content_and_then_reuses_it() {
    let mut session = RuntimeSession::new();
    let mut builds = [0; 2];
    for alternate in [false, false, true, true, false, false] {
        session.invalidate_view();
        let document = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |_, views| {
                    let mut document = UiDocument::new(LayoutStyle::column());
                    let root = document.root();
                    if alternate {
                        views.section(&mut document, root, "panel", &(), |_, _| {
                            builds[1] += 1;
                            let mut child = UiDocument::new(LayoutStyle::column());
                            child.add_child(
                                child.root(),
                                UiNode::container("alternate", LayoutStyle::size(80.0, 40.0)),
                            );
                            child
                        });
                    } else {
                        views.section(&mut document, root, "panel", &(), |_, _| {
                            builds[0] += 1;
                            let mut child = UiDocument::new(LayoutStyle::column());
                            child.add_child(
                                child.root(),
                                UiNode::container("initial", LayoutStyle::size(160.0, 40.0)),
                            );
                            child
                        });
                    }
                    document
                },
            )
            .unwrap();
        let content = document.nodes().last().unwrap();
        assert_eq!(
            content.name(),
            if alternate { "alternate" } else { "initial" },
            "a section must not display the previous branch's content"
        );
        session.retain_document(document);
        session.frame_presented();
    }
    assert_eq!(
        builds,
        [2, 1],
        "unchanged call sites must still reuse their views"
    );
}

#[test]
fn nested_sections_evict_removed_inputs_and_refresh_external_dependencies() {
    let mut session = RuntimeSession::new();
    let parent_builds = Cell::new(0);
    let child_builds = Cell::new(0);
    let sibling_builds = Cell::new(0);
    let render = |session: &mut RuntimeSession, parent_value, child_value, shown, scale| {
        session.invalidate_view();
        let doc = session
            .build_document(
                VIEWPORT,
                scale,
                None,
                &mut ApproxTextMeasurer,
                |_, views| {
                    let mut doc = UiDocument::new(LayoutStyle::column());
                    let root = doc.root();
                    views.section(
                        &mut doc,
                        root,
                        "parent",
                        &(parent_value, child_value, shown),
                        |(_, child_value, shown), nested| {
                            parent_builds.set(parent_builds.get() + 1);
                            let mut doc = UiDocument::new(LayoutStyle::column());
                            let root = doc.root();
                            if *shown {
                                nested.section(
                                    &mut doc,
                                    root,
                                    "child",
                                    child_value,
                                    |value, context| {
                                        child_builds.set(child_builds.get() + 1);
                                        panel(value, context)
                                    },
                                );
                            }
                            doc
                        },
                    );
                    views.section(&mut doc, root, "sibling", &3usize, |value, context| {
                        sibling_builds.set(sibling_builds.get() + 1);
                        panel(value, context)
                    });
                    doc
                },
            )
            .unwrap();
        session.retain_document(doc);
    };
    render(&mut session, 0, 2, true, UiDocumentScale::DEFAULT);
    render(&mut session, 1, 2, true, UiDocumentScale::DEFAULT);
    assert_eq!(
        (
            parent_builds.get(),
            child_builds.get(),
            sibling_builds.get()
        ),
        (2, 1, 1)
    );
    render(&mut session, 1, 2, false, UiDocumentScale::DEFAULT);
    render(&mut session, 1, 2, true, UiDocumentScale::DEFAULT);
    assert_eq!(child_builds.get(), 2, "removed child cache must be evicted");
    render(&mut session, 1, 2, true, UiDocumentScale::new(1.5, 2.0));
    assert_eq!((child_builds.get(), sibling_builds.get()), (3, 2));
    session.refresh_view();
    render(&mut session, 1, 2, true, UiDocumentScale::new(1.5, 2.0));
    assert_eq!((child_builds.get(), sibling_builds.get()), (4, 3));
}

#[test]
fn resource_snapshots_release_superseded_pixels_through_retries_and_section_reuse() {
    use crate::platform::{ImageHandle, PixelSize};
    use crate::renderer::{PixelRect, ResourceDescriptor, ResourceFormat, ResourceUpdate};
    use std::sync::Arc;
    use std::time::Duration;

    let mut session = RuntimeSession::new();
    let mut allocations = Vec::new();
    let mut build = |session: &mut RuntimeSession, revision: u64| {
        session.invalidate_view();
        session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |_, views| {
                    let mut doc = UiDocument::new(LayoutStyle::size(32.0, 32.0));
                    let root = doc.root();
                    views.section(&mut doc, root, "preview", &revision, |revision, _| {
                        let mut child = UiDocument::new(LayoutStyle::size(32.0, 32.0));
                        let size = PixelSize::new(2 + (*revision % 3) as u32, 2);
                        let format = match revision % 3 {
                            0 => ResourceFormat::Rgba8,
                            1 => ResourceFormat::Bgra8,
                            _ => ResourceFormat::Alpha8,
                        };
                        let descriptor =
                            ResourceDescriptor::new(ImageHandle::app("preview"), size, format)
                                .version(*revision * 2 + 1);
                        let pixels: Arc<[u8]> = vec![
                            *revision as u8;
                            size.width as usize
                                * size.height as usize
                                * format.bytes_per_pixel()
                        ]
                        .into();
                        allocations.push(Arc::downgrade(&pixels));
                        child.set_resource(descriptor.clone(), pixels);
                        child.add_resource_update(ResourceUpdate::partial(
                            descriptor.version(*revision * 2 + 2),
                            PixelRect::new(0, 0, 1, 1),
                            vec![255; format.bytes_per_pixel()],
                        ));
                        child
                    });
                    doc
                },
            )
            .unwrap()
    };
    for revision in 0..64 {
        let now = Duration::from_millis(revision * 250);
        session.begin_frame(now);
        let doc = build(&mut session, revision);
        assert_eq!(
            doc.resource_updates().len(),
            2,
            "only the latest image snapshot and its following patch should remain"
        );
        assert_eq!(
            doc.resource_updates()[0].descriptor.version,
            revision * 2 + 1
        );
        assert!(doc.resource_updates()[0]
            .bytes
            .iter()
            .all(|byte| *byte == revision as u8));
        assert!(doc.resource_updates()[1].is_partial());
        session.retain_document(doc);
        session.frame_failed(now);
    }
    // Reusing the section must retain its unpresented snapshot without uploading it twice.
    let retry = build(&mut session, 63);
    assert_eq!(session.view_build_stats().reused, 1);
    assert_eq!(retry.resource_updates().len(), 2);
    session.retain_document(retry);
    drop(build);
    assert_eq!(
        allocations
            .iter()
            .filter(|pixels| pixels.strong_count() > 0)
            .count(),
        1,
        "superseded image allocations must be released before presentation recovers"
    );
    session.frame_presented();
    assert!(allocations.iter().all(|pixels| pixels.strong_count() == 0));
    let next = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_, _| panic!("presented document should be reused"),
        )
        .unwrap();
    assert!(next.resource_updates().is_empty());
}

#[test]
fn failed_upload_survives_rebuild_and_cache_hits_do_not_reupload_after_presentation() {
    use crate::platform::{ImageHandle, PixelSize};
    use crate::renderer::ResourceUpdate;
    let mut session = RuntimeSession::new();
    let build = |session: &mut RuntimeSession, revision| {
        session.invalidate_view();
        session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |size, views| {
                    let mut doc = describe(size, views, revision, false, false);
                    let root = doc.root();
                    views.section(&mut doc, root, "image", &(), |_, _| {
                        let mut doc = UiDocument::new(LayoutStyle::size(1.0, 1.0));
                        doc.add_resource_update(ResourceUpdate::rgba8_image(
                            ImageHandle::app("pixel"),
                            PixelSize::new(1, 1),
                            vec![255; 4],
                        ));
                        doc
                    });
                    doc
                },
            )
            .unwrap()
    };
    let first = build(&mut session, 1);
    assert_eq!(first.resource_updates().len(), 1);
    let pixels = std::sync::Arc::downgrade(&first.resource_updates()[0].bytes);
    session.retain_document(first);
    session.frame_failed(std::time::Duration::ZERO);
    let retry = build(&mut session, 2);
    assert_eq!(
        retry.resource_updates().len(),
        1,
        "rebuilding cannot acknowledge an upload"
    );
    assert!(std::sync::Arc::ptr_eq(
        &retry.resource_updates()[0].bytes,
        &pixels
            .upgrade()
            .expect("pending upload survives rebuilding")
    ));
    session.retain_document(retry);
    session.frame_presented();
    assert!(
        pixels.upgrade().is_none(),
        "presentation must release upload storage from both the session and view cache"
    );
    let next = build(&mut session, 3);
    assert!(next.resource_updates().is_empty());
}

#[test]
fn overlay_and_internal_references_remain_correct_after_cached_section_moves() {
    use crate::{
        AccessibilityMeta, AccessibilityRole, InputBehavior, UiNodeLayoutConstraint, UiPortalTarget,
    };
    let mut session = RuntimeSession::new();
    for inserted in [false, true] {
        session.invalidate_view();
        let doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |size, views| {
                    let mut doc =
                        UiDocument::new(LayoutStyle::column().with_size(size.width, size.height));
                    let root = doc.root();
                    if inserted {
                        doc.add_child(
                            root,
                            UiNode::container("new", LayoutStyle::size(20.0, 20.0)),
                        );
                    }
                    views.section(&mut doc, root, "panel", &(), |_, _| {
                        let mut doc =
                            UiDocument::new(LayoutStyle::column().with_size(100.0, 100.0));
                        let label = doc.add_child(
                            doc.root(),
                            UiNode::text(
                                "label",
                                "Menu",
                                TextStyle::default(),
                                LayoutStyle::default(),
                            ),
                        );
                        let menu = doc.add_portal_child(
                            label,
                            UiPortalTarget::AppOverlay,
                            UiNode::container(
                                "menu",
                                crate::layout::absolute(500.0, 100.0, 80.0, 40.0),
                            )
                            .with_input(InputBehavior::BUTTON)
                            .with_accessibility(
                                AccessibilityMeta::new(AccessibilityRole::Button)
                                    .labelled_by(label),
                            ),
                        );
                        doc.add_child(
                            doc.root(),
                            UiNode::container("sized", LayoutStyle::default())
                                .with_layout_constraint(
                                    UiNodeLayoutConstraint::InlineIntrinsicSize {
                                        sources: vec![label],
                                        min_size: UiSize::ZERO,
                                    },
                                ),
                        );
                        assert!(doc.node(menu).stack_parent.is_some());
                        doc
                    });
                    doc
                },
            )
            .unwrap();
        let find = |name| {
            UiNodeId(
                doc.nodes()
                    .iter()
                    .position(|node| node.name() == name)
                    .unwrap(),
            )
        };
        let label = find("label");
        let menu = find("menu");
        assert_eq!(doc.node(menu).stack_parent, Some(label));
        assert_eq!(
            doc.node(menu)
                .accessibility
                .as_ref()
                .unwrap()
                .relations
                .labelled_by,
            vec![label]
        );
        assert_eq!(doc.node(menu).layout().rect.x, 500.0);
        assert!(
            doc.node(menu).layout().visible,
            "viewport overlay was clipped to its panel"
        );
        assert!(doc.node(find("sized")).layout().rect.width > 0.0);
        assert_eq!(session.view_build_stats().reused, usize::from(inserted));
        session.retain_document(doc);
    }
}

#[test]
fn cached_sections_preserve_focus_scroll_and_press_but_removal_ends_the_lifetime() {
    use crate::host::HostFrameOutput;
    use crate::input::{PointerButton, PointerEventKind, RawInputEvent, RawPointerEvent};
    use crate::platform::PlatformRequestIdAllocator;
    use crate::renderer::RenderTarget;
    use crate::{InputBehavior, ScrollAxes, ScrollState, UiFocusState, UiPoint};
    let mut session = RuntimeSession::new();
    let describe = |views: &mut ViewContext<'_>, prefix, present| {
        let mut doc = UiDocument::new(LayoutStyle::column().with_size(800.0, 400.0));
        let root = doc.root();
        if prefix {
            doc.add_child(
                root,
                UiNode::container("prefix", LayoutStyle::size(20.0, 20.0)),
            );
        }
        if present {
            views.section(&mut doc, root, "panel", &(), |_, _| {
                let mut doc = UiDocument::new(LayoutStyle::column().with_size(200.0, 150.0));
                doc.node_mut(doc.root()).scroll = Some(
                    ScrollState::new(ScrollAxes::VERTICAL).with_offset(UiPoint::new(0.0, 10.0)),
                );
                let mut ids = Vec::new();
                for index in 0..10 {
                    ids.push(
                        doc.add_child(
                            doc.root(),
                            UiNode::container(
                                format!("button-{index}"),
                                LayoutStyle::size(180.0, 40.0).with_flex_shrink(0.0),
                            )
                            .with_input(InputBehavior::BUTTON),
                        ),
                    );
                }
                doc.set_focus_state(UiFocusState {
                    focused: Some(ids[0]),
                    ..Default::default()
                });
                doc
            });
        }
        doc
    };
    let mut doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_, views| describe(views, false, true),
        )
        .unwrap();
    let find = |doc: &UiDocument, name| {
        UiNodeId(
            doc.nodes()
                .iter()
                .position(|node| node.name() == name)
                .unwrap(),
        )
    };
    let button = find(&doc, "button-1");
    let rect = doc.node(button).layout().rect;
    let input = session
        .process_input(
            &mut doc,
            VIEWPORT,
            vec![RawInputEvent::Pointer(RawPointerEvent::new(
                PointerEventKind::Down(PointerButton::Primary),
                UiPoint::new(rect.x + 4.0, rect.y + 4.0),
                1,
            ))],
            vec![],
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    session
        .finish_frame(
            &mut doc,
            VIEWPORT,
            RenderTarget::window("views", VIEWPORT),
            input,
            &mut ApproxTextMeasurer,
            &mut PlatformRequestIdAllocator::default(),
        )
        .unwrap();
    let scroll = find(&doc, "panel");
    doc.node_mut(scroll)
        .scroll
        .as_mut()
        .unwrap()
        .set_host_offset(UiPoint::new(0.0, 50.0));
    session
        .finish_frame(
            &mut doc,
            VIEWPORT,
            RenderTarget::window("views", VIEWPORT),
            HostFrameOutput::new(session.interaction().clone()),
            &mut ApproxTextMeasurer,
            &mut PlatformRequestIdAllocator::default(),
        )
        .unwrap();
    assert_eq!(doc.focus_state().focused, Some(button));
    session.retain_document(doc);
    for present in [true, false, true] {
        session.invalidate_view();
        let doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |_, views| describe(views, true, present),
            )
            .unwrap();
        if present && session.view_build_stats().reused == 1 {
            let button = find(&doc, "button-1");
            assert_eq!(
                doc.focus_state().focused,
                Some(button),
                "cached authored focus stole focus back"
            );
            assert_eq!(session.interaction().pressed, Some(button));
            assert_eq!(
                doc.node(find(&doc, "panel")).scroll.unwrap().offset().y,
                50.0,
                "cached authored offset reset scrolling"
            );
        } else {
            assert!(
                session.interaction().pressed.is_none(),
                "removed press must not resurrect"
            );
            if present {
                assert_eq!(doc.focus_state().focused, Some(find(&doc, "button-0")));
            }
        }
        session.retain_document(doc);
    }
}

#[test]
fn cached_text_reflows_when_its_parent_changes_available_width() {
    let mut session = RuntimeSession::new();
    let build = |width, views: &mut ViewContext<'_>| {
        let mut doc = UiDocument::new(LayoutStyle::column().with_size(width, 400.0));
        let root = doc.root();
        views.section(&mut doc, root, "paragraph", &(), |_, _| {
            let mut doc = UiDocument::new(LayoutStyle::column());
            doc.add_child(doc.root(), UiNode::text("text", "A paragraph whose line wrapping depends on the available width of its surrounding panel.", TextStyle::default(), LayoutStyle::default()));
            doc
        });
        doc
    };
    let mut heights = Vec::new();
    for width in [300.0, 90.0, 220.0, 300.0] {
        session.invalidate_view();
        let doc = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |_, views| build(width, views),
            )
            .unwrap();
        let reference = RuntimeSession::new()
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                &mut ApproxTextMeasurer,
                |_, views| build(width, views),
            )
            .unwrap();
        for (actual, expected) in doc.nodes().iter().zip(reference.nodes()) {
            assert_eq!(
                actual.layout(),
                expected.layout(),
                "width {width}, {}",
                actual.name()
            );
        }
        heights.push(doc.nodes().last().unwrap().layout().rect.height);
        if heights.len() > 1 {
            assert_eq!(session.view_build_stats().reused, 1);
        }
        session.retain_document(doc);
    }
    assert!(
        heights[1] > heights[0],
        "narrower paragraph must wrap to more lines"
    );
    assert_eq!(heights[0], heights[3]);
}

#[test]
fn external_measurement_refresh_is_not_lost_while_host_holds_document() {
    let mut session = RuntimeSession::new();
    let mut measurements = Measurements::default();
    let doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut measurements,
            |size, views| describe(size, views, 2, true, false),
        )
        .unwrap();
    let before = measurements.0.clone();
    // A host can receive newly loaded fonts between preparing and retaining its
    // document. The next build must discard that document's old measurements.
    session.refresh_view();
    session.retain_document(doc);
    let _doc = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            &mut measurements,
            |size, views| describe(size, views, 2, true, false),
        )
        .unwrap();
    for (text, count) in &before {
        assert!(
            measurements.0[text] > *count,
            "refresh reused measurement for {text}"
        );
    }
    assert_eq!(session.view_build_stats().rebuilt, 2);
}
