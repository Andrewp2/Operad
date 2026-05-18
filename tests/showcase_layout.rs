#![allow(dead_code, unused_imports)]

mod showcase_app {
    #![allow(dead_code, unused_imports)]

    include!("../examples/showcase.rs");

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::{Duration, Instant};

        use operad::diagnostics::AuditWarning;
        use operad::host::{process_document_frame, HostDocumentFrameRequest, HostFrameOutput};
        #[cfg(feature = "wgpu")]
        use operad::platform::PixelSize;
        #[cfg(feature = "wgpu")]
        use operad::renderer::{RenderOptions, RenderedImage, ResourceFormat};
        use operad::renderer::{RenderTarget, RendererAdapter};
        use operad::testing::EmptyResourceResolver;
        #[cfg(feature = "wgpu")]
        use operad::wgpu_renderer::WgpuRenderer;
        #[cfg(feature = "wgpu")]
        use operad::ColorRgba;
        use operad::{
            AnimationInputValue, ApproxTextMeasurer, CosmicTextMeasurer, KeyCode, KeyModifiers,
            PaintKind, TextMeasurer, TextWrap, UiContent, UiFocusState, UiInputEvent, UiWheelEvent,
            WidgetPointerEdit, WidgetValueEditPhase,
        };

        fn state_with_window(id: &str) -> ShowcaseState {
            let mut state = ShowcaseState::default();
            state.windows.clear_all();
            *state.windows.slot_mut(id).expect("known showcase window") = true;
            state.desktop.ensure_window(id, window_defaults(id));
            if id == "popup_panel" {
                state.popup_open = true;
            }
            if id == "overlays" {
                state.overlay_popup_open = true;
                state.overlay_modal_open = true;
            }
            state
        }

        fn state_with_all_windows() -> ShowcaseState {
            let mut state = ShowcaseState::default();
            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                *state.windows.slot_mut(id).expect("known showcase window") = true;
                state.desktop.ensure_window(id, window_defaults(id));
            }
            state.popup_open = true;
            state.overlay_popup_open = true;
            state.overlay_modal_open = true;
            state.combo_open = true;
            state.dropdown.open(&select_options());
            state.slider_trailing_picker_open = true;
            state.menu_button.open(&menu_items(state.menu_autosave));
            state
                .context_menu
                .open_with_items(UiPoint::new(160.0, 160.0), &menu_items(state.menu_autosave));
            state
        }

        fn node_id(document: &UiDocument, name: &str) -> UiNodeId {
            document
                .nodes()
                .iter()
                .enumerate()
                .find_map(|(index, node)| {
                    (node.name() == name).then_some(UiNodeId::from_index(index))
                })
                .unwrap_or_else(|| panic!("missing node {name:?}"))
        }

        fn maybe_node_id(document: &UiDocument, name: &str) -> Option<UiNodeId> {
            document
                .nodes()
                .iter()
                .enumerate()
                .find_map(|(index, node)| {
                    (node.name() == name).then_some(UiNodeId::from_index(index))
                })
        }

        fn severe_layout_warning(warning: &AuditWarning) -> bool {
            matches!(
                warning,
                AuditWarning::NonFiniteRect { .. }
                    | AuditWarning::EmptyInteractiveClip { .. }
                    | AuditWarning::TextClipped { .. }
                    | AuditWarning::ScrollRangeHidden { .. }
                    | AuditWarning::ScrollOffsetOutOfRange { .. }
                    | AuditWarning::ScrollbarVisibleWithoutRange { .. }
                    | AuditWarning::NodeOutsideRoot { .. }
                    | AuditWarning::PaintItemEmptyClip { .. }
            )
        }

        fn rects_overlap(left: UiRect, right: UiRect) -> bool {
            left.x < right.x + right.width
                && left.x + left.width > right.x
                && left.y < right.y + right.height
                && left.y + left.height > right.y
        }

        fn first_scene_rect(document: &UiDocument, name: &str) -> UiRect {
            let node = document.node(node_id(document, name));
            let UiContent::Scene(primitives) = node.content() else {
                panic!("{name:?} is not a scene node");
            };
            let Some(ScenePrimitive::Rect(rect)) = primitives.first() else {
                panic!("{name:?} does not start with a rect primitive");
            };
            rect.rect
        }

        #[test]
        fn showcase_all_widgets_open_frame_stays_within_interactive_budget() {
            let state = state_with_all_windows();
            let viewport = UiSize::new(1200.0, 900.0);
            let mut measurer = CosmicTextMeasurer::new();

            let mut startup_document = ShowcaseState::default().view(viewport);
            startup_document
                .compute_layout(viewport, &mut measurer)
                .expect("startup layout");

            let view_started = Instant::now();
            let mut document = state.view(viewport);
            let view_elapsed = view_started.elapsed();

            let layout_started = Instant::now();
            document
                .compute_layout(viewport, &mut measurer)
                .expect("showcase layout");
            let layout_elapsed = layout_started.elapsed();

            let frame_started = Instant::now();
            let frame = process_document_frame(
                &mut document,
                &mut measurer,
                HostDocumentFrameRequest::new(
                    viewport,
                    RenderTarget::window("showcase", viewport),
                    HostFrameOutput::new(Default::default()),
                ),
            )
            .expect("showcase frame");
            let frame_elapsed = frame_started.elapsed();
            let total_elapsed = view_elapsed + layout_elapsed + frame_elapsed;

            eprintln!(
                "all widgets open: nodes={} paint={} view={:?} layout={:?} frame={:?} total={:?}",
                document.node_count(),
                frame.render_request.paint.items.len(),
                view_elapsed,
                layout_elapsed,
                frame_elapsed,
                total_elapsed
            );

            let warm_view_started = Instant::now();
            let mut warm_document = state.view(viewport);
            let warm_view_elapsed = warm_view_started.elapsed();
            let warm_layout_started = Instant::now();
            warm_document
                .compute_layout(viewport, &mut measurer)
                .expect("warm showcase layout");
            let warm_layout_elapsed = warm_layout_started.elapsed();
            let warm_frame_started = Instant::now();
            let warm_frame = process_document_frame(
                &mut warm_document,
                &mut measurer,
                HostDocumentFrameRequest::new(
                    viewport,
                    RenderTarget::window("showcase", viewport),
                    HostFrameOutput::new(Default::default()),
                ),
            )
            .expect("warm showcase frame");
            let warm_frame_elapsed = warm_frame_started.elapsed();
            let warm_elapsed = warm_view_elapsed + warm_layout_elapsed + warm_frame_elapsed;
            eprintln!(
                "all widgets open warm: nodes={} paint={} view={:?} layout={:?} frame={:?} total={:?}",
                warm_document.node_count(),
                warm_frame.render_request.paint.items.len(),
                warm_view_elapsed,
                warm_layout_elapsed,
                warm_frame_elapsed,
                warm_elapsed,
            );

            assert!(
                warm_elapsed.as_millis() < 250,
                "warm all widgets open frame exceeded budget after text cache warmup: {warm_elapsed:?}"
            );
            #[cfg(not(debug_assertions))]
            {
                let frame_120hz_budget = Duration::from_micros(8_334);
                let mut best_120hz_sample = warm_elapsed;
                for _ in 0..8 {
                    let sample_view_started = Instant::now();
                    let mut sample_document = state.view(viewport);
                    let sample_view_elapsed = sample_view_started.elapsed();
                    let sample_layout_started = Instant::now();
                    sample_document
                        .compute_layout(viewport, &mut measurer)
                        .expect("120hz sample showcase layout");
                    let sample_layout_elapsed = sample_layout_started.elapsed();
                    let sample_frame_started = Instant::now();
                    let _sample_frame = process_document_frame(
                        &mut sample_document,
                        &mut measurer,
                        HostDocumentFrameRequest::new(
                            viewport,
                            RenderTarget::window("showcase", viewport),
                            HostFrameOutput::new(Default::default()),
                        ),
                    )
                    .expect("120hz sample showcase frame");
                    let sample_elapsed = sample_view_elapsed
                        + sample_layout_elapsed
                        + sample_frame_started.elapsed();
                    best_120hz_sample = best_120hz_sample.min(sample_elapsed);
                }
                assert!(
                    best_120hz_sample <= frame_120hz_budget,
                    "best warm all-widgets frame did not reach the 120 FPS CPU budget: best={best_120hz_sample:?}"
                );

                #[cfg(feature = "wgpu")]
                {
                    let mut renderer = WgpuRenderer::default();
                    renderer.warm_up().expect("wgpu warm-up");
                    for _ in 0..3 {
                        renderer
                            .render_frame(warm_frame.render_request.clone(), &EmptyResourceResolver)
                            .expect("all widgets render warm-up");
                    }
                    let mut best_render_sample = Duration::MAX;
                    let mut best_recorded_render = Duration::MAX;
                    let mut best_recorded_batch = Duration::MAX;
                    for _ in 0..8 {
                        let render_started = Instant::now();
                        let output = renderer
                            .render_frame(warm_frame.render_request.clone(), &EmptyResourceResolver)
                            .expect("all widgets render");
                        best_render_sample = best_render_sample.min(render_started.elapsed());
                        if let Some(render) = output.timings.duration("render") {
                            best_recorded_render = best_recorded_render.min(render);
                        }
                        if let Some(batch) = output.timings.duration("batch") {
                            best_recorded_batch = best_recorded_batch.min(batch);
                        }
                    }
                    eprintln!(
                        "all widgets 120hz best samples: cpu={:?} render_wall={:?} batch_section={:?} render_section={:?} combined={:?}",
                        best_120hz_sample,
                        best_render_sample,
                        best_recorded_batch,
                        best_recorded_render,
                        best_120hz_sample + best_render_sample
                    );
                    assert!(
                        best_120hz_sample + best_render_sample <= frame_120hz_budget,
                        "best warm all-widgets CPU+WGPU frame did not reach the 120 FPS budget: cpu={best_120hz_sample:?} render={best_render_sample:?}"
                    );
                    assert!(
                        best_render_sample <= Duration::from_millis(2),
                        "best warm all-widgets WGPU render exceeded budget: best={best_render_sample:?}"
                    );
                }
            }
            #[cfg(debug_assertions)]
            let cold_budget = std::cmp::max(
                warm_elapsed.mul_f32(12.0) + Duration::from_millis(200),
                Duration::from_millis(900),
            );
            #[cfg(not(debug_assertions))]
            let cold_budget = warm_elapsed.mul_f32(8.0) + Duration::from_millis(200);
            assert!(
                total_elapsed <= cold_budget,
                "cold all widgets open frame was disproportionate to warm frame: cold={total_elapsed:?} warm={warm_elapsed:?} budget={cold_budget:?}"
            );
        }

        #[test]
        fn showcase_windows_avoid_hard_clipping_at_common_viewport_sizes() {
            let viewports = [
                UiSize::new(900.0, 760.0),
                UiSize::new(720.0, 560.0),
                UiSize::new(1180.0, 820.0),
            ];

            for viewport in viewports {
                for id in SHOWCASE_WIDGET_WINDOW_IDS {
                    let state = state_with_window(id);
                    let mut document = state.view(viewport);
                    document
                        .compute_layout(viewport, &mut ApproxTextMeasurer)
                        .expect("showcase layout");
                    let warnings = document
                        .audit_layout()
                        .into_iter()
                        .filter(severe_layout_warning)
                        .collect::<Vec<_>>();
                    assert!(
                    warnings.is_empty(),
                    "window {id:?} at {viewport:?} emitted severe layout warnings: {warnings:#?}"
                );
                }
            }
        }

        #[test]
        fn showcase_state_matrix_audits_every_widget_window_common_states() {
            let viewport = UiSize::new(940.0, 780.0);
            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                audit_showcase_window_variant(id, "normal", viewport, |_, _| {});
                audit_showcase_window_variant(id, "collapsed", viewport, |state, id| {
                    state.desktop.collapsed.insert(id.to_string());
                });
                audit_showcase_window_variant(id, "narrow", viewport, |state, id| {
                    state
                        .desktop
                        .sizes
                        .insert(id.to_string(), UiSize::new(220.0, 140.0));
                    state.desktop.user_sized.insert(id.to_string());
                });
                audit_showcase_window_variant(id, "scrolled", viewport, |_, _| {});
                audit_showcase_window_focus_variant(id, "focused", viewport, true);
                audit_showcase_window_focus_variant(id, "hovered", viewport, false);
            }
        }

        fn audit_showcase_window_variant(
            id: &str,
            variant: &str,
            viewport: UiSize,
            configure: impl FnOnce(&mut ShowcaseState, &str),
        ) {
            let mut state = state_with_window(id);
            configure(&mut state, id);
            let mut document = state.view(viewport);
            let mut measurer = CosmicTextMeasurer::new();
            document
                .compute_layout(viewport, &mut measurer)
                .unwrap_or_else(|error| panic!("{id} {variant} failed layout: {error}"));
            if variant == "scrolled" {
                scroll_all_nodes_to_end(&mut document, viewport, &mut measurer);
            }
            assert_no_severe_showcase_warnings(id, variant, &document);
        }

        fn audit_showcase_window_focus_variant(
            id: &str,
            variant: &str,
            viewport: UiSize,
            focused: bool,
        ) {
            let state = state_with_window(id);
            let mut document = state.view(viewport);
            let mut measurer = CosmicTextMeasurer::new();
            document
                .compute_layout(viewport, &mut measurer)
                .unwrap_or_else(|error| panic!("{id} {variant} failed layout: {error}"));
            let window_name = format!("showcase.windows.window.{id}");
            let window = node_id(&document, &window_name);
            if let Some(target) = first_interactive_descendant(&document, window) {
                document.set_focus_state(UiFocusState {
                    hovered: Some(target),
                    focused: focused.then_some(target),
                    pressed: None,
                });
            }
            assert_no_severe_showcase_warnings(id, variant, &document);
        }

        fn scroll_all_nodes_to_end(
            document: &mut UiDocument,
            viewport: UiSize,
            measurer: &mut impl TextMeasurer,
        ) {
            let scrolls = (0..document.node_count())
                .filter_map(|index| {
                    let id = UiNodeId::from_index(index);
                    let mut scroll = document.scroll_state(id)?;
                    scroll.set_offset(scroll.max_offset());
                    Some((id, scroll))
                })
                .collect::<Vec<_>>();
            for (id, scroll) in scrolls {
                document.node_mut(id).set_scroll(scroll);
            }
            document.invalidate_layout();
            document
                .compute_layout(viewport, measurer)
                .expect("scrolled showcase layout");
        }

        fn first_interactive_descendant(document: &UiDocument, root: UiNodeId) -> Option<UiNodeId> {
            document
                .nodes()
                .iter()
                .enumerate()
                .find_map(|(index, node)| {
                    let id = UiNodeId::from_index(index);
                    (node.action().is_some()
                        && node.layout().visible
                        && is_descendant_or_self(document, root, id))
                    .then_some(id)
                })
        }

        fn is_descendant_or_self(
            document: &UiDocument,
            ancestor: UiNodeId,
            node: UiNodeId,
        ) -> bool {
            let mut current = Some(node);
            while let Some(id) = current {
                if id == ancestor {
                    return true;
                }
                current = document.node(id).parent();
            }
            false
        }

        fn assert_no_severe_showcase_warnings(id: &str, variant: &str, document: &UiDocument) {
            let warnings = document
                .audit_layout()
                .into_iter()
                .filter(severe_layout_warning)
                .collect::<Vec<_>>();
            assert!(
                warnings.is_empty(),
                "window {id:?} variant {variant:?} emitted severe layout warnings: {warnings:#?}"
            );
        }

        #[test]
        fn showcase_bulk_actions_open_all_and_publish_fps_counter() {
            let mut state = ShowcaseState::default();
            state.windows.clear_all();
            state.update(WidgetAction::activate(UiNodeId::root(), "window.add_all"));
            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                assert!(state.windows.is_visible(id), "{id}");
            }

            state.fps = 58.6;
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            let fps_panel = document.node(node_id(&document, "showcase.fps"));
            let fps_label = document.node(node_id(&document, "showcase.fps.label"));
            let organize_button = document.node(node_id(&document, "showcase.organize_windows"));
            assert_eq!(
                organize_button
                    .action()
                    .and_then(|action| action.action_id())
                    .map(|id| id.as_str()),
                Some("window.organize_open")
            );
            let UiContent::Text(text) = fps_label.content() else {
                panic!("fps label should be text");
            };
            assert_eq!(text.text, "59 FPS");
            let fps_text_paint = document
                .paint_list()
                .items
                .into_iter()
                .find(|item| {
                    item.node == node_id(&document, "showcase.fps.label")
                        && matches!(item.kind, PaintKind::Text(_))
                })
                .expect("fps text paint");
            assert!(
                fps_text_paint.rect.x >= fps_panel.layout().rect.x + 4.0,
                "{:?} {:?}",
                fps_text_paint.rect,
                fps_panel.layout().rect
            );
            assert!(
                fps_text_paint.rect.y >= fps_panel.layout().rect.y + 4.0,
                "{:?} {:?}",
                fps_text_paint.rect,
                fps_panel.layout().rect
            );
        }

        #[test]
        fn showcase_default_window_placement_reserves_organize_button_space() {
            let viewport = UiSize::new(900.0, 760.0);
            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                let state = state_with_window(id);
                let mut document = state.view(viewport);
                document
                    .compute_layout(viewport, &mut ApproxTextMeasurer)
                    .unwrap_or_else(|error| panic!("{id} layout failed: {error}"));
                let organize = document
                    .node(node_id(&document, "showcase.organize_windows"))
                    .layout()
                    .rect;
                let window = document
                    .node(node_id(&document, &format!("showcase.windows.window.{id}")))
                    .layout()
                    .rect;
                assert!(
                    !rects_overlap(organize, window),
                    "{id} default window overlaps organize button: organize={organize:?} window={window:?}"
                );
            }
        }

        #[test]
        fn showcase_organize_button_reflows_currently_open_windows() {
            let mut state = ShowcaseState::default();
            state.windows.clear_all();
            for id in ["checkbox", "progress"] {
                *state.windows.slot_mut(id).expect("known showcase window") = true;
                state.desktop.ensure_window(id, window_defaults(id));
                state
                    .desktop
                    .positions
                    .insert(id.to_string(), UiPoint::new(500.0, 500.0));
            }
            state
                .desktop
                .ensure_window("labels", window_defaults("labels"));
            state
                .desktop
                .positions
                .insert("labels".to_string(), UiPoint::new(500.0, 500.0));
            state.last_desktop_size = UiSize::new(900.0, 700.0);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.organize_open",
            ));

            let rects = ["checkbox", "progress"]
                .into_iter()
                .map(|id| {
                    let position = state.desktop.position(id, UiPoint::new(0.0, 0.0));
                    let size = state.desktop.size(id, UiSize::ZERO);
                    (
                        id,
                        UiRect::new(position.x, position.y, size.width, size.height),
                    )
                })
                .collect::<Vec<_>>();
            for (left_index, (left_id, left_rect)) in rects.iter().enumerate() {
                assert_ne!(
                    *left_rect,
                    UiRect::new(
                        500.0,
                        500.0,
                        window_defaults(left_id).size.width,
                        window_defaults(left_id).size.height
                    )
                );
                assert!(
                    left_rect.x >= 0.0
                        && left_rect.y >= SHOWCASE_ORGANIZE_BUTTON_RESERVED_HEIGHT
                        && left_rect.right() <= state.last_desktop_size.width + f32::EPSILON
                        && left_rect.bottom() <= state.last_desktop_size.height + f32::EPSILON,
                    "{left_id} was arranged outside the visible desktop: {left_rect:?}"
                );
                for (right_id, right_rect) in rects.iter().skip(left_index + 1) {
                    assert!(
                        !rects_overlap(*left_rect, *right_rect),
                        "{left_id} {left_rect:?} overlapped {right_id} {right_rect:?}"
                    );
                }
            }
            assert_eq!(
                state.desktop.position("labels", UiPoint::new(0.0, 0.0)),
                UiPoint::new(500.0, 500.0)
            );
            assert!(!state.windows.is_visible("labels"));
        }

        #[test]
        fn showcase_organize_uses_measured_window_minimums() {
            let mut state = ShowcaseState::default();
            let ids = ["labels", "buttons", "color_picker", "canvas"];
            state.windows.clear_all();
            for id in ids {
                *state.windows.slot_mut(id).expect("known showcase window") = true;
                state.desktop.ensure_window(id, window_defaults(id));
            }
            state.last_desktop_size = UiSize::new(1900.0, 1180.0);
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.organize_open",
            ));
            for id in ids {
                assert!(
                    !state.desktop.is_collapsed(id),
                    "organize should not collapse {id}"
                );
            }

            let viewport = UiSize::new(2200.0, 1180.0);
            let mut document = state.view(viewport);
            let mut measurer = CosmicTextMeasurer::new();
            document
                .compute_layout(viewport, &mut measurer)
                .expect("showcase layout");
            let rects = ids
                .into_iter()
                .map(|id| {
                    let name = format!("showcase.windows.window.{id}");
                    (id, document.node(node_id(&document, &name)).layout().rect)
                })
                .collect::<Vec<_>>();

            for (left_index, (left_id, left_rect)) in rects.iter().enumerate() {
                assert!(
                    left_rect.x >= 0.0
                        && left_rect.y >= 0.0
                        && left_rect.right() <= state.last_desktop_size.width + f32::EPSILON
                        && left_rect.bottom() <= state.last_desktop_size.height + f32::EPSILON,
                    "{left_id} was arranged outside the visible desktop: {left_rect:?}"
                );
                for (right_id, right_rect) in rects.iter().skip(left_index + 1) {
                    assert!(
                        !rects_overlap(*left_rect, *right_rect),
                        "{left_id} {left_rect:?} overlapped {right_id} {right_rect:?}"
                    );
                }
            }
        }

        #[test]
        fn showcase_date_picker_spawns_at_content_width() {
            let state = state_with_window("date_picker");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            let mut measurer = CosmicTextMeasurer::new();
            document
                .compute_layout(viewport, &mut measurer)
                .expect("showcase layout");

            let window = document
                .node(node_id(&document, "showcase.windows.window.date_picker"))
                .layout()
                .rect;
            let picker_id = node_id(&document, "date.picker");
            let picker = document.node(picker_id).layout().rect;
            let controls = document
                .node(node_id(&document, "date.options"))
                .layout()
                .rect;

            assert!(
                window.width <= 304.0,
                "window={window:?} picker={picker:?} controls={controls:?}"
            );
            assert!(
                picker.right() <= window.right(),
                "picker={picker:?} window={window:?}"
            );
            assert!(
                controls.right() <= window.right(),
                "controls={controls:?} window={window:?}"
            );
        }

        #[test]
        fn showcase_organize_all_open_windows_without_overlap() {
            let mut state = ShowcaseState::default();
            state.windows.open_all();
            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                state.desktop.ensure_window(id, window_defaults(id));
            }
            state.last_desktop_size = UiSize::new(1900.0, 1180.0);
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.organize_open",
            ));

            let viewport = UiSize::new(2200.0, 1180.0);
            let mut document = state.view(viewport);
            let mut measurer = CosmicTextMeasurer::new();
            document
                .compute_layout(viewport, &mut measurer)
                .expect("showcase layout");
            let rects = SHOWCASE_WIDGET_WINDOW_IDS
                .into_iter()
                .map(|id| {
                    let name = format!("showcase.windows.window.{id}");
                    (id, document.node(node_id(&document, &name)).layout().rect)
                })
                .collect::<Vec<_>>();

            for (left_index, (left_id, left_rect)) in rects.iter().enumerate() {
                assert!(
                    left_rect.x >= 0.0
                        && left_rect.y >= 0.0
                        && left_rect.right() <= state.last_desktop_size.width + f32::EPSILON
                        && left_rect.bottom() <= state.last_desktop_size.height + f32::EPSILON,
                    "{left_id} was arranged outside the visible desktop: {left_rect:?}"
                );
                for (right_id, right_rect) in rects.iter().skip(left_index + 1) {
                    assert!(
                        !rects_overlap(*left_rect, *right_rect),
                        "{left_id} {left_rect:?} overlapped {right_id} {right_rect:?}"
                    );
                }
            }
        }

        #[test]
        fn showcase_organize_short_desktop_collapses_open_windows() {
            let mut state = ShowcaseState::default();
            let ids = [
                "labels",
                "checkbox",
                "toggles",
                "slider",
                "color_picker",
                "canvas",
            ];
            state.windows.clear_all();
            for id in ids {
                *state.windows.slot_mut(id).expect("known showcase window") = true;
                state.desktop.ensure_window(id, window_defaults(id));
            }
            state.last_desktop_size = UiSize::new(1900.0, 397.0);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.organize_open",
            ));

            for id in ids {
                assert!(
                    state.desktop.is_collapsed(id),
                    "organize should collapse {id} when a short desktop cannot fit the expanded layout"
                );
            }
        }

        #[test]
        fn showcase_virtualized_table_exercises_public_sort_filter_and_resize_actions() {
            let mut state = state_with_window("lists_tables");
            let viewport = UiSize::new(920.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("lists layout");

            let name_header = document.node(node_id(
                &document,
                "lists_tables.virtualized_table.header.name",
            ));
            assert_eq!(
                name_header
                    .action()
                    .as_ref()
                    .and_then(|action| action.action_id())
                    .map(|id| id.as_str()),
                Some("lists_tables.virtualized_table.sort.name")
            );
            assert!(maybe_node_id(
                &document,
                "lists_tables.virtualized_table.header.value.resize"
            )
            .is_some());

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "lists_tables.virtualized_table.sort.name",
            ));
            assert!(state.virtual_table_descending);
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "lists_tables.virtualized_table.filter.status",
            ));
            assert!(state.virtual_table_ready_only);
            let visible_rows = virtual_table_visible_rows(&state);
            assert!(!visible_rows.is_empty());
            assert!(visible_rows.iter().all(|row| row % 2 == 0));
            assert!(visible_rows.windows(2).all(|pair| pair[0] > pair[1]));

            let value_header = document
                .node(node_id(
                    &document,
                    "lists_tables.virtualized_table.header.value",
                ))
                .layout()
                .rect;
            let drag_origin = UiPoint::new(value_header.right(), value_header.y + 12.0);
            state.update(WidgetAction::pointer_edit(
                UiNodeId::root(),
                "lists_tables.virtualized_table.resize.value",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Begin,
                    drag_origin,
                    UiPoint::new(value_header.width, value_header.height),
                    value_header,
                ),
            ));
            state.update(WidgetAction::pointer_edit(
                UiNodeId::root(),
                "lists_tables.virtualized_table.resize.value",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    UiPoint::new(drag_origin.x + 42.0, drag_origin.y),
                    UiPoint::new(value_header.width + 42.0, value_header.height),
                    value_header,
                ),
            ));
            assert!(state.virtual_table_value_width > 100.0);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "lists_tables.virtualized_table.resize.reset",
            ));
            assert_eq!(state.virtual_table_value_width, 70.0);
        }

        #[test]
        fn showcase_diagnostics_animation_controls_mutate_owned_state() {
            let mut state = state_with_window("diagnostics");
            let viewport = UiSize::new(1200.0, 900.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("diagnostics layout");

            assert!(maybe_node_id(&document, "diagnostics.animation.controls").is_some());
            assert!(maybe_node_id(
                &document,
                "diagnostics.animation.controls.input.active.toggle"
            )
            .is_some());
            assert!(maybe_node_id(
                &document,
                "diagnostics.animation.controls.input.hover.set.slider"
            )
            .is_some());
            assert!(
                maybe_node_id(&document, "diagnostics.animation.controls.input.pulse.fire")
                    .is_some()
            );

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "diagnostics.animation.controls.transport.pause_toggle",
            ));
            assert!(state.diagnostics_animation_paused);

            let before_step = state.diagnostics_animation_scrub;
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "diagnostics.animation.controls.transport.step",
            ));
            assert!(state.diagnostics_animation_paused);
            assert!(state.diagnostics_animation_scrub > before_step);

            let scrub_rect = document
                .node(node_id(
                    &document,
                    "diagnostics.animation.controls.transport.scrub",
                ))
                .layout()
                .rect;
            let scrub_point =
                UiPoint::new(scrub_rect.x + scrub_rect.width * 0.75, scrub_rect.y + 8.0);
            let expected_scrub = scaled_slider(scrub_rect, scrub_point, 0.0, 1.0);
            state.update(WidgetAction::pointer_edit(
                UiNodeId::root(),
                "diagnostics.animation.controls.transport.scrub",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    scrub_point,
                    UiPoint::new(scrub_rect.width * 0.75, 8.0),
                    scrub_rect,
                ),
            ));
            assert!((state.diagnostics_animation_scrub - expected_scrub).abs() < 0.01);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "diagnostics.animation.controls.input.active.toggle",
            ));
            assert!(!state.diagnostics_animation_active);

            let hover_rect = document
                .node(node_id(
                    &document,
                    "diagnostics.animation.controls.input.hover.set.slider",
                ))
                .layout()
                .rect;
            let hover_point =
                UiPoint::new(hover_rect.x + hover_rect.width * 0.2, hover_rect.y + 8.0);
            let expected_hover = scaled_slider(hover_rect, hover_point, 0.0, 1.0);
            state.update(WidgetAction::pointer_edit(
                UiNodeId::root(),
                "diagnostics.animation.controls.input.hover.set",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    hover_point,
                    UiPoint::new(hover_rect.width * 0.2, 8.0),
                    hover_rect,
                ),
            ));
            assert!((state.diagnostics_animation_hover - expected_hover).abs() < 0.01);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "diagnostics.animation.controls.input.pulse.fire",
            ));
            assert_eq!(state.diagnostics_animation_pulse_count, 1);

            let snapshot = diagnostics_sample_snapshot(&state);
            let animation = snapshot
                .animation("diagnostics.sample.preview")
                .expect("animation snapshot");
            assert!(animation.inputs.iter().any(|(name, value)| {
                name == "active" && matches!(value, AnimationInputValue::Bool(false))
            }));
            assert!(animation.inputs.iter().any(|(name, value)| {
                name == "hover"
                    && matches!(value, AnimationInputValue::Number(number) if (*number - expected_hover).abs() < 0.01)
            }));
        }

        #[test]
        fn showcase_diagnostics_accessibility_overlay_preview_clips_children() {
            let state = state_with_window("diagnostics");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("diagnostics layout");

            let preview = node_id(&document, "diagnostics.a11y.preview");
            let preview_rect = document.node(preview).layout().rect;
            assert!(
                preview_rect.height >= 130.0,
                "diagnostics overlay preview collapsed: {preview_rect:?}"
            );
            let panel = document.node(node_id(&document, "diagnostics.a11y"));
            assert!(
                panel.layout().rect.y >= preview_rect.bottom() - 0.5,
                "diagnostics overlay panel overlapped preview: panel={:?} preview={preview_rect:?}",
                panel.layout().rect
            );
            let paint = document.paint_list();
            for item in paint
                .items
                .iter()
                .filter(|item| is_descendant_or_self(&document, preview, item.node))
            {
                assert!(
                    item.clip_rect.x >= preview_rect.x - 0.5
                        && item.clip_rect.y >= preview_rect.y - 0.5
                        && item.clip_rect.right() <= preview_rect.right() + 0.5
                        && item.clip_rect.bottom() <= preview_rect.bottom() + 0.5,
                    "diagnostics overlay preview leaked paint outside the preview: item={item:#?} preview={preview_rect:?}"
                );
            }
        }

        #[test]
        fn showcase_trees_include_tree_table_and_focus_preservation_example() {
            let state = state_with_window("trees");
            let viewport = UiSize::new(1100.0, 820.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("trees layout");

            assert!(maybe_node_id(&document, "trees.table").is_some());
            assert!(maybe_node_id(&document, "trees.table.header.name").is_some());
            assert!(maybe_node_id(&document, "trees.table.row.1.cell.name").is_some());
            assert!(maybe_node_id(&document, "trees.table.cell.1.0.label").is_some());

            let previous_state = ext_widgets::TreeViewState::expanded(["root", "branch-a"]);
            let next_state = ext_widgets::TreeViewState::expanded(["root"]);
            let previous = previous_state.visible_items(&tree_table_items());
            let next = next_state.visible_items(&tree_table_items());
            let focus =
                ext_widgets::tree_view::tree_focus_preservation_by_id(&previous, &next, "docs");
            assert_eq!(focus.index_before, Some(5));
            assert_eq!(focus.index_after, Some(3));
            let plan = ext_widgets::VirtualTreeViewSpec::new(24.0, 96.0)
                .focus(focus.clone())
                .plan(next.len());
            assert_eq!(plan.focus, Some(focus));
        }

        #[test]
        fn showcase_clear_all_accepts_stale_focus_from_previous_all_widgets_frame() {
            let mut state = ShowcaseState::default();
            state.windows.clear_all();
            state.update(WidgetAction::activate(UiNodeId::root(), "window.add_all"));
            let viewport = UiSize::new(900.0, 760.0);
            let mut all_document = state.view(viewport);
            all_document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("all widgets layout");
            let stale = UiNodeId::from_index(all_document.node_count().saturating_sub(1));

            state.update(WidgetAction::activate(UiNodeId::root(), "window.clear_all"));
            let mut cleared_document = state.view(viewport);
            cleared_document.set_focus_state(UiFocusState {
                hovered: Some(stale),
                focused: Some(stale),
                pressed: Some(stale),
            });
            cleared_document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("cleared showcase layout");
            assert!(cleared_document.node_count() < stale.index());
            assert_eq!(cleared_document.focus_state().hovered, None);
            assert_eq!(cleared_document.focus_state().focused, None);
            assert_eq!(cleared_document.focus_state().pressed, None);
        }

        #[test]
        fn showcase_styling_uses_color_buttons_for_stroke_and_shadow_colors() {
            let mut state = state_with_window("styling");
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "styling.stroke_color_button",
            ));
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "styling.fill_color_button",
            ));
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "styling.shadow_color_button",
            ));

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            assert!(maybe_node_id(&document, "styling.stroke_color_button").is_some());
            assert!(maybe_node_id(&document, "styling.fill_color_button").is_some());
            assert!(maybe_node_id(&document, "styling.shadow_color_button").is_some());
            assert!(maybe_node_id(&document, "styling.stroke_picker").is_some());
            assert!(maybe_node_id(&document, "styling.fill_picker").is_some());
            assert!(maybe_node_id(&document, "styling.shadow_picker").is_some());
            assert!(maybe_node_id(&document, "styling.stroke_color.slider").is_none());
            assert!(maybe_node_id(&document, "styling.fill.slider").is_none());
            assert!(maybe_node_id(&document, "styling.shadow_alpha.slider").is_none());
        }

        #[test]
        fn showcase_styling_composes_controls_and_preview_with_grid_layout() {
            let state = state_with_window("styling");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let grid = document.node(node_id(&document, "styling.grid"));
            let grid_layout = grid.style().layout().expect("styling grid layout");
            assert_eq!(grid_layout.display, operad::LayoutDisplay::Grid);
            assert_eq!(grid.style().layout_style().grid_template_column_count(), 3);
        }

        #[test]
        fn showcase_styling_preview_text_is_center_aligned() {
            let state = state_with_window("styling");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let scene = node_id(&document, "styling.preview.scene");
            let paint = document.paint_list();
            let text = paint
                .items
                .iter()
                .find_map(|item| match &item.kind {
                    PaintKind::SceneText(text) if item.node == scene && text.text == "Content" => {
                        Some(text)
                    }
                    _ => None,
                })
                .expect("styling preview scene text");

            assert_eq!(text.horizontal_align, TextHorizontalAlign::Center);
            assert_eq!(text.vertical_align, TextVerticalAlign::Center);
        }

        #[test]
        fn showcase_styling_window_minimum_keeps_preview_content_visible() {
            let mut state = state_with_window("styling");
            state
                .desktop
                .sizes
                .insert("styling".to_string(), UiSize::new(420.0, 260.0));
            state.desktop.user_sized.insert("styling".to_string());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("styling layout");

            let scene = node_id(&document, "styling.preview.scene");
            let scene_rect = document.node(scene).layout().rect;
            let content = document
                .node(node_id(
                    &document,
                    "showcase.windows.window.styling.content",
                ))
                .layout()
                .rect;
            assert!(
                scene_rect.right() <= content.right(),
                "scene={scene_rect:?} content={content:?}"
            );

            for item in document
                .paint_list()
                .items
                .into_iter()
                .filter(|item| item.node == scene)
            {
                if matches!(item.kind, PaintKind::RichRect(_) | PaintKind::SceneText(_)) {
                    assert!(
                        item.rect.right() <= item.clip_rect.right() + 0.5,
                        "preview item should fit horizontally: rect={:?} clip={:?}",
                        item.rect,
                        item.clip_rect
                    );
                }
            }
        }

        #[test]
        fn showcase_forms_overlays_drag_drop_and_media_minimums_keep_demo_content_reachable() {
            let viewport = UiSize::new(900.0, 760.0);
            let specs: [(&str, &[&str], &[&str]); 4] = [
                (
                    "forms",
                    &[
                        "forms.profile",
                        "forms.profile.status.dirty",
                        "forms.profile.status.pending",
                        "forms.profile.status.submitted",
                        "forms.profile.name.label",
                        "forms.profile.name.input",
                        "forms.profile.name.help",
                        "forms.profile.email.label",
                        "forms.profile.email.input",
                        "forms.profile.email.validation",
                        "forms.profile.role.label",
                        "forms.profile.role.input",
                        "forms.profile.role.help",
                        "forms.profile.newsletter.input",
                        "forms.profile.newsletter.help",
                        "forms.profile.errors",
                        "forms.profile.actions.submit",
                        "forms.profile.actions.apply",
                        "forms.profile.actions.cancel",
                        "forms.profile.actions.reset",
                        "forms.profile.status",
                    ],
                    &["forms.profile.status"],
                ),
                (
                    "overlays",
                    &[
                        "overlays.collapsing",
                        "overlays.collapsing.body",
                        "overlays.popup.toggle",
                        "overlays.modal.open",
                        "overlays.tooltip",
                        "overlays.tooltip.body",
                        "overlays.tooltip.shortcut",
                        "overlays.tooltip.disabled_reason",
                        "overlays.tooltip_rect.preview",
                        "overlays.tooltip_rect.scene",
                        "overlays.popup_panel",
                        "overlays.popup_panel.close",
                        "overlays.modal",
                        "overlays.modal.close",
                        "overlays.modal.body.close",
                    ],
                    &["overlays.tooltip_rect.scene"],
                ),
                (
                    "drag_drop",
                    &[
                        "drag_drop.text_source",
                        "drag_drop.file_source",
                        "drag_drop.bytes_source",
                        "drag_drop.accept_text",
                        "drag_drop.files_only",
                        "drag_drop.image_bytes",
                        "drag_drop.disabled",
                        "drag_drop.operation.copy",
                        "drag_drop.operation.move",
                        "drag_drop.operation.link",
                        "drag_drop.status",
                    ],
                    &["drag_drop.status"],
                ),
                (
                    "media",
                    &[
                        "media.icons.label",
                        "media.variants.label",
                        "media.image.untinted",
                        "media.image.warning",
                        "media.image.shader",
                        "media.image.note",
                    ],
                    &["media.image.note"],
                ),
            ];

            for (id, required_nodes, bottom_nodes) in specs {
                let mut document = minimized_showcase_window_document(id, viewport);
                assert_window_resolved_to_minimum(&document, id);
                assert_no_severe_showcase_warnings(id, "minimum", &document);
                assert_required_demo_nodes_fit_minimized_window(&document, id, required_nodes);

                if id == "media" {
                    assert_media_icon_grid_present_and_inside_window(&document);
                }

                let mut measurer = CosmicTextMeasurer::new();
                scroll_all_nodes_to_end(&mut document, viewport, &mut measurer);
                assert_no_severe_showcase_warnings(id, "minimum scrolled", &document);
                for name in bottom_nodes {
                    assert_node_has_visible_paint_inside_clip(&document, name);
                }
            }
        }

        fn minimized_showcase_window_document(id: &str, viewport: UiSize) -> UiDocument {
            let mut state = state_with_window(id);
            state
                .desktop
                .sizes
                .insert(id.to_string(), UiSize::new(1.0, 1.0));
            state.desktop.user_sized.insert(id.to_string());
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                .unwrap_or_else(|error| panic!("{id} minimum failed layout: {error}"));
            document
        }

        fn assert_window_resolved_to_minimum(document: &UiDocument, id: &str) {
            let window = document.node(node_id(document, &format!("showcase.windows.window.{id}")));
            let min_size = window.style().layout_style().min_size();
            let min_width = min_size
                .and_then(|size| size.width.points_value())
                .unwrap_or(window.layout().rect.width);
            let min_height = min_size
                .and_then(|size| size.height.points_value())
                .unwrap_or(window.layout().rect.height);
            assert!(
                (window.layout().rect.width - min_width).abs() < 0.5,
                "{id} window was not minimized to its computed minimum width: rect={:?} min_width={min_width}",
                window.layout().rect
            );
            assert!(
                (window.layout().rect.height - min_height).abs() < 0.5,
                "{id} window was not minimized to its computed minimum height: rect={:?} min_height={min_height}",
                window.layout().rect
            );
        }

        fn assert_required_demo_nodes_fit_minimized_window(
            document: &UiDocument,
            id: &str,
            required_nodes: &[&str],
        ) {
            let window_content =
                node_id(document, &format!("showcase.windows.window.{id}.content"));
            let content_rect = document.node(window_content).layout().rect;
            for name in required_nodes {
                let node = node_id(document, name);
                let rect = document.node(node).layout().rect;
                assert!(
                    rect.width.is_finite()
                        && rect.height.is_finite()
                        && rect.width >= 0.0
                        && rect.height >= 0.0,
                    "{id} required node {name:?} had invalid geometry: {rect:?}"
                );
                if is_descendant_or_self(document, window_content, node) {
                    assert!(
                        rect.x >= content_rect.x - 0.5
                            && rect.right() <= content_rect.right() + 0.5,
                        "{id} required node {name:?} escaped minimized content horizontally: node={rect:?} content={content_rect:?}"
                    );
                }
            }
        }

        fn assert_media_icon_grid_present_and_inside_window(document: &UiDocument) {
            let mut required = Vec::new();
            for icon in BuiltInIcon::COMMON {
                let key = media_icon_test_name(icon);
                required.push(format!("media.icon_tile.{key}"));
                required.push(format!("media.icon.{key}"));
                required.push(format!("media.icon_label.{key}"));
            }
            let names = required.iter().map(String::as_str).collect::<Vec<_>>();
            assert_required_demo_nodes_fit_minimized_window(document, "media", &names);
        }

        fn media_icon_test_name(icon: BuiltInIcon) -> String {
            icon.key().replace('.', "_").replace('-', "_")
        }

        fn assert_node_has_visible_paint_inside_clip(document: &UiDocument, name: &str) {
            let node = node_id(document, name);
            let paint = document.paint_list();
            let matching = paint
                .items
                .iter()
                .filter(|item| item.node == node)
                .collect::<Vec<_>>();
            assert!(
                !matching.is_empty(),
                "expected {name:?} to produce paint after scrolling to the minimized demo end"
            );
            assert!(
                matching.iter().any(|item| {
                    item.rect.width > 0.0
                        && item.rect.height > 0.0
                        && rects_overlap(item.rect, item.clip_rect)
                        && item.rect.x >= item.clip_rect.x - 0.5
                        && item.rect.y >= item.clip_rect.y - 0.5
                        && item.rect.right() <= item.clip_rect.right() + 0.5
                        && item.rect.bottom() <= item.clip_rect.bottom() + 0.5
                }),
                "expected at least one visible paint item for {name:?}, got {matching:#?}"
            );
        }

        #[test]
        fn showcase_media_minimized_scroll_area_has_visible_scrollbar_affordance() {
            let document = minimized_showcase_window_document("media", UiSize::new(900.0, 760.0));
            assert_scroll_area_has_visible_auto_scrollbar(&document, "media.section_scroll");
        }

        fn assert_scroll_area_has_visible_auto_scrollbar(document: &UiDocument, name: &str) {
            let scroll_node = node_id(document, name);
            let scroll = document
                .scroll_state(scroll_node)
                .unwrap_or_else(|| panic!("{name:?} should be scrollable"));
            assert!(
                scroll.max_offset().y > 0.0,
                "{name:?} should have a vertical scroll range: {scroll:?}"
            );
            let rect = document.node(scroll_node).layout().rect;
            let scrollbar_items = document
                .paint_list()
                .items
                .into_iter()
                .filter(|item| {
                    item.node == scroll_node
                        && matches!(item.kind, PaintKind::Rect { .. })
                        && item.rect.width <= 8.0
                        && item.rect.height >= 18.0
                        && item.rect.x >= rect.right() - 12.0
                        && item.rect.right() <= rect.right() + 0.5
                })
                .collect::<Vec<_>>();
            assert!(
                scrollbar_items.len() >= 2,
                "{name:?} should paint an automatic track and thumb, got {scrollbar_items:#?}"
            );
        }

        #[cfg(feature = "wgpu")]
        #[test]
        fn showcase_forms_overlays_drag_drop_and_media_minimized_snapshots_have_visible_regions() {
            let viewport = UiSize::new(900.0, 760.0);
            let specs: [(&str, &[&str], &[&str]); 4] = [
                (
                    "forms",
                    &[
                        "forms.profile.name.input",
                        "forms.profile.email.input",
                        "forms.profile.actions.submit",
                    ],
                    &["forms.profile.status"],
                ),
                (
                    "overlays",
                    &[
                        "overlays.tooltip.body",
                        "overlays.tooltip_rect.scene",
                        "overlays.popup_panel",
                        "overlays.modal",
                    ],
                    &["overlays.tooltip_rect.scene"],
                ),
                (
                    "drag_drop",
                    &[
                        "drag_drop.text_source",
                        "drag_drop.accept_text",
                        "drag_drop.operation.copy",
                    ],
                    &["drag_drop.status"],
                ),
                (
                    "media",
                    &["media.icons.label"],
                    &[
                        "media.image.untinted",
                        "media.image.warning",
                        "media.image.shader",
                        "media.image.note",
                    ],
                ),
            ];

            let mut renderer = WgpuRenderer::default();
            renderer.warm_up().expect("wgpu warm-up");
            for (id, visible_nodes, bottom_nodes) in specs {
                let mut document = minimized_showcase_window_document(id, viewport);
                let image =
                    render_showcase_snapshot(&mut document, viewport, &mut renderer, id, "top");
                for name in visible_nodes {
                    assert_rendered_node_has_visible_detail(&image, &document, name);
                }

                let mut measurer = CosmicTextMeasurer::new();
                scroll_all_nodes_to_end(&mut document, viewport, &mut measurer);
                let image =
                    render_showcase_snapshot(&mut document, viewport, &mut renderer, id, "bottom");
                for name in bottom_nodes {
                    assert_rendered_node_has_visible_detail(&image, &document, name);
                }
            }
        }

        #[cfg(feature = "wgpu")]
        #[test]
        #[ignore = "manual visual audit generator; set OPERAD_SHOWCASE_VISUAL_DUMP_DIR to collect every demo snapshot"]
        fn showcase_all_demo_windows_visual_audit_snapshots() {
            let viewport = UiSize::new(900.0, 760.0);
            let mut renderer = WgpuRenderer::default();
            renderer.warm_up().expect("wgpu warm-up");

            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                let state = state_with_window(id);
                let mut document = state.view(viewport);
                let image =
                    render_showcase_snapshot(&mut document, viewport, &mut renderer, id, "default");
                let window_name = format!("showcase.windows.window.{id}");
                assert_rendered_node_has_visible_detail(&image, &document, &window_name);
                assert_no_severe_showcase_warnings(id, "visual default", &document);

                let mut scrolled = state.view(viewport);
                scrolled
                    .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                    .unwrap_or_else(|error| panic!("{id} visual bottom layout failed: {error}"));
                let mut measurer = CosmicTextMeasurer::new();
                scroll_all_nodes_to_end(&mut scrolled, viewport, &mut measurer);
                let image =
                    render_showcase_snapshot(&mut scrolled, viewport, &mut renderer, id, "bottom");
                assert_rendered_node_has_visible_detail(&image, &scrolled, &window_name);
                assert_no_severe_showcase_warnings(id, "visual bottom", &scrolled);
            }
        }

        #[cfg(feature = "wgpu")]
        fn render_showcase_snapshot(
            document: &mut UiDocument,
            viewport: UiSize,
            renderer: &mut WgpuRenderer,
            id: &str,
            variant: &str,
        ) -> RenderedImage {
            let pixel_size =
                PixelSize::new(viewport.width.ceil() as u32, viewport.height.ceil() as u32);
            let request = HostDocumentFrameRequest::new(
                viewport,
                RenderTarget::snapshot(pixel_size),
                HostFrameOutput::new(Default::default()),
            )
            .render_options(RenderOptions {
                clear_color: ColorRgba::new(13, 17, 23, 255),
                ..RenderOptions::default()
            });
            let frame = process_document_frame(document, &mut CosmicTextMeasurer::new(), request)
                .unwrap_or_else(|error| panic!("{id} {variant} snapshot layout failed: {error}"));
            let output = renderer
                .render_frame(frame.render_request, &EmptyResourceResolver)
                .unwrap_or_else(|error| panic!("{id} {variant} snapshot render failed: {error}"));
            let image = output
                .snapshot
                .expect("snapshot target should render pixels");
            assert_eq!(image.size, pixel_size);
            assert_eq!(image.format, ResourceFormat::Rgba8);
            maybe_dump_showcase_snapshot(&image, id, variant);
            image
        }

        #[cfg(feature = "wgpu")]
        fn assert_rendered_node_has_visible_detail(
            image: &RenderedImage,
            document: &UiDocument,
            name: &str,
        ) {
            let node = document.node(node_id(document, name));
            let visible_rect = node
                .layout()
                .rect
                .intersection(node.layout().clip_rect)
                .unwrap_or_else(|| panic!("{name:?} has no visible clipped region"));
            let stats = rendered_region_stats(image, visible_rect);
            let minimum_changed = 4.max(stats.total_pixels / 500);
            assert!(
                stats.total_pixels >= 16,
                "{name:?} visible region is too small for visual inspection: {visible_rect:?}"
            );
            assert!(
                stats.changed_pixels >= minimum_changed && stats.max_channel_span >= 8,
                "{name:?} did not render enough visible pixel detail: rect={visible_rect:?} stats={stats:?}"
            );
        }

        #[cfg(feature = "wgpu")]
        #[derive(Debug)]
        struct RenderedRegionStats {
            total_pixels: usize,
            changed_pixels: usize,
            max_channel_span: u8,
        }

        #[cfg(feature = "wgpu")]
        fn rendered_region_stats(image: &RenderedImage, rect: UiRect) -> RenderedRegionStats {
            let width = image.size.width as usize;
            let height = image.size.height as usize;
            let x0 = rect.x.max(0.0).floor() as usize;
            let y0 = rect.y.max(0.0).floor() as usize;
            let x1 = rect.right().min(image.size.width as f32).ceil() as usize;
            let y1 = rect.bottom().min(image.size.height as f32).ceil() as usize;
            let x1 = x1.min(width);
            let y1 = y1.min(height);

            let mut first = None;
            let mut changed_pixels = 0;
            let mut min_channel = [u8::MAX; 3];
            let mut max_channel = [0_u8; 3];
            let mut total_pixels = 0;

            for y in y0..y1 {
                for x in x0..x1 {
                    let offset = (y * width + x) * 4;
                    let pixel = [
                        image.pixels[offset],
                        image.pixels[offset + 1],
                        image.pixels[offset + 2],
                        image.pixels[offset + 3],
                    ];
                    if let Some(first) = first {
                        if pixel_channel_delta(pixel, first) > 4 {
                            changed_pixels += 1;
                        }
                    } else {
                        first = Some(pixel);
                    }
                    for channel in 0..3 {
                        min_channel[channel] = min_channel[channel].min(pixel[channel]);
                        max_channel[channel] = max_channel[channel].max(pixel[channel]);
                    }
                    total_pixels += 1;
                }
            }

            let max_channel_span = (0..3)
                .map(|channel| max_channel[channel].saturating_sub(min_channel[channel]))
                .max()
                .unwrap_or(0);
            RenderedRegionStats {
                total_pixels,
                changed_pixels,
                max_channel_span,
            }
        }

        #[cfg(feature = "wgpu")]
        fn pixel_channel_delta(left: [u8; 4], right: [u8; 4]) -> u8 {
            (0..4)
                .map(|channel| left[channel].abs_diff(right[channel]))
                .max()
                .unwrap_or(0)
        }

        #[cfg(feature = "wgpu")]
        fn maybe_dump_showcase_snapshot(image: &RenderedImage, id: &str, variant: &str) {
            let Ok(dir) = std::env::var("OPERAD_SHOWCASE_VISUAL_DUMP_DIR") else {
                return;
            };
            let path = std::path::Path::new(&dir).join(format!("{id}-{variant}.ppm"));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create showcase visual dump dir");
            }
            let mut bytes =
                format!("P6\n{} {}\n255\n", image.size.width, image.size.height).into_bytes();
            for pixel in image.pixels.chunks_exact(4) {
                bytes.extend_from_slice(&pixel[0..3]);
            }
            std::fs::write(&path, bytes).expect("write showcase visual dump");
        }

        #[test]
        fn showcase_windows_survive_small_user_resizes() {
            let viewport = UiSize::new(900.0, 760.0);
            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                let mut state = state_with_window(id);
                state
                    .desktop
                    .sizes
                    .insert(id.to_string(), UiSize::new(220.0, 140.0));
                state.desktop.user_sized.insert(id.to_string());
                if id == "selection" {
                    state.combo_open = true;
                    state.dropdown.open(&select_options());
                }
                if id == "slider" {
                    state.slider_trailing_picker_open = true;
                }
                if id == "menus" {
                    state.menu_button.open(&menu_items(state.menu_autosave));
                    state.context_menu.open_with_items(
                        UiPoint::new(160.0, 160.0),
                        &menu_items(state.menu_autosave),
                    );
                }
                if id == "overlays" {
                    state.overlay_popup_open = true;
                    state.overlay_modal_open = true;
                }
                if id == "popup_panel" {
                    state.popup_open = true;
                }

                let mut document = state.view(viewport);
                document
                    .compute_layout(viewport, &mut ApproxTextMeasurer)
                    .expect("showcase layout");
                let warnings = document
                    .audit_layout()
                    .into_iter()
                    .filter(severe_layout_warning)
                    .collect::<Vec<_>>();
                assert!(
                    warnings.is_empty(),
                    "resized window {id:?} emitted severe layout warnings: {warnings:#?}"
                );
            }
        }

        #[test]
        fn showcase_widget_list_scrollbar_reaches_bottom_after_long_wheel_scroll() {
            let viewport = UiSize::new(900.0, 760.0);
            let mut state = ShowcaseState::default();
            state.windows.clear_all();

            for _ in 0..40 {
                let mut document = state.view(viewport);
                document
                    .compute_layout(viewport, &mut ApproxTextMeasurer)
                    .expect("showcase layout");
                let list = node_id(&document, "controls.widget_list.viewport");
                let rect = document.node(list).layout().rect;
                let wheel_point =
                    UiPoint::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
                let result = document.handle_input(UiInputEvent::Wheel(UiWheelEvent::pixels(
                    wheel_point,
                    UiPoint::new(0.0, 80.0),
                )));
                if let Some(scrolled) = result.scrolled {
                    let scroll = document.scroll_state(scrolled).expect("scroll state");
                    state.update(WidgetAction::scroll(
                        scrolled,
                        "controls.widget_list.scroll",
                        scroll,
                    ));
                }
            }

            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            let list = node_id(&document, "controls.widget_list.viewport");
            let scroll = document.scroll_state(list).expect("scroll state");
            let max_offset = scroll.max_offset().y;
            assert!(max_offset > 0.0, "{scroll:?}");
            assert!(
                (scroll.offset().y - max_offset).abs() < 0.01,
                "scroll did not reach max offset: {scroll:?}"
            );

            let track = document
                .node(node_id(
                    &document,
                    "controls.widget_list.vertical-scrollbar",
                ))
                .layout()
                .rect;
            let thumb = document
                .node(node_id(
                    &document,
                    "controls.widget_list.vertical-scrollbar.thumb",
                ))
                .layout()
                .rect;
            assert!(
                (thumb.bottom() - track.bottom()).abs() < 0.01,
                "thumb={thumb:?} track={track:?} scroll={scroll:?}"
            );
        }

        #[test]
        fn showcase_widget_list_scrollbar_thumb_drag_updates_scroll() {
            let viewport = UiSize::new(900.0, 760.0);
            let mut state = ShowcaseState::default();
            state.windows.clear_all();
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let scrollbar = node_id(&document, "controls.widget_list.vertical-scrollbar");
            let thumb = node_id(&document, "controls.widget_list.vertical-scrollbar.thumb");
            let track_rect = document.node(scrollbar).layout().rect;
            let thumb_rect = document.node(thumb).layout().rect;
            let start = UiPoint::new(thumb_rect.x + thumb_rect.width * 0.5, thumb_rect.y + 2.0);
            let end = UiPoint::new(start.x, track_rect.bottom() - 2.0);

            assert_eq!(
                document
                    .handle_input(UiInputEvent::PointerDown(start))
                    .pressed,
                Some(scrollbar)
            );
            state.update(WidgetAction::pointer_edit(
                scrollbar,
                "controls.widget_list.scrollbar",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Begin,
                    start,
                    UiPoint::new(start.x - track_rect.x, start.y - track_rect.y),
                    track_rect,
                ),
            ));
            state.update(WidgetAction::pointer_edit(
                scrollbar,
                "controls.widget_list.scrollbar",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    end,
                    UiPoint::new(end.x - track_rect.x, end.y - track_rect.y),
                    track_rect,
                ),
            ));

            let mut scrolled = state.view(viewport);
            scrolled
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("scrolled showcase layout");
            let list = node_id(&scrolled, "controls.widget_list.viewport");
            let scroll = scrolled.scroll_state(list).expect("scroll state");
            assert!(
                scroll.offset().y > scroll.max_offset().y * 0.9,
                "dragging scrollbar thumb should move the widget list near the bottom: {scroll:?}"
            );
        }

        #[test]
        fn showcase_canvas_aspect_fits_when_window_is_short() {
            let mut state = state_with_window("canvas");
            state
                .desktop
                .sizes
                .insert("canvas".to_string(), UiSize::new(420.0, 160.0));
            state.desktop.user_sized.insert("canvas".to_string());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let canvas = document
                .nodes()
                .iter()
                .find(|node| node.name() == "canvas.shader")
                .expect("canvas shader node");
            let rect = canvas.layout().rect;
            assert!(rect.width > 0.0 && rect.height > 0.0, "{rect:?}");
            assert!(
                (rect.width / rect.height - 16.0 / 9.0).abs() < 0.01,
                "{rect:?}"
            );
            assert!(rect.width < 300.0, "{rect:?}");
        }

        #[test]
        fn showcase_canvas_default_window_spawns_larger() {
            let state = state_with_window("canvas");
            let viewport = UiSize::new(1180.0, 820.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let window = document
                .node(node_id(&document, "showcase.windows.window.canvas"))
                .layout()
                .rect;
            assert!(window.width >= 560.0, "{window:?}");
            assert!(window.height >= 390.0, "{window:?}");
        }

        #[test]
        fn showcase_slider_primary_track_width_is_stable_when_window_resizes() {
            let mut widths = Vec::new();
            for window_width in [430.0, 340.0] {
                let mut state = state_with_window("slider");
                state
                    .desktop
                    .sizes
                    .insert("slider".to_string(), UiSize::new(window_width, 360.0));
                state.desktop.user_sized.insert("slider".to_string());

                let viewport = UiSize::new(900.0, 760.0);
                let mut document = state.view(viewport);
                document
                    .compute_layout(viewport, &mut ApproxTextMeasurer)
                    .expect("showcase layout");
                let slider = document
                    .nodes()
                    .iter()
                    .find(|node| node.name() == "slider.value")
                    .expect("primary slider node");
                widths.push(slider.layout().rect.width);
            }

            assert!((widths[0] - 180.0).abs() < 0.01, "{widths:?}");
            assert!((widths[1] - 180.0).abs() < 0.01, "{widths:?}");
        }

        #[test]
        fn showcase_buttons_window_can_shrink_below_default_height() {
            let mut state = state_with_window("buttons");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("initial buttons layout");
            let window = document
                .node(node_id(&document, "showcase.windows.window.buttons"))
                .layout()
                .rect;
            let resize_origin = UiPoint::new(window.right(), window.bottom());
            state.update(WidgetAction::pointer_edit(
                UiNodeId::root(),
                "window.resize.buttons",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Begin,
                    resize_origin,
                    UiPoint::new(window.width, window.height),
                    window,
                ),
            ));
            state.update(WidgetAction::pointer_edit(
                UiNodeId::root(),
                "window.resize.buttons",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    UiPoint::new(window.right(), window.y + 150.0),
                    UiPoint::new(window.width, 150.0),
                    window,
                ),
            ));

            let mut resized = state.view(viewport);
            resized
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("resized buttons layout");
            let resized_window = resized
                .node(node_id(&resized, "showcase.windows.window.buttons"))
                .layout()
                .rect;
            assert!(
                resized_window.width >= 494.0,
                "resized_window={resized_window:?}"
            );
            assert!(
                resized_window.height >= 200.0 && resized_window.height <= 240.0,
                "resized_window={resized_window:?}"
            );
            let content = resized
                .node(node_id(&resized, "showcase.windows.window.buttons.content"))
                .layout()
                .rect;
            let last = resized
                .node(node_id(&resized, "buttons.last"))
                .layout()
                .rect;
            assert!(
                last.bottom() <= content.bottom(),
                "last={last:?} content={content:?}"
            );
        }

        #[test]
        fn showcase_layout_widgets_use_dock_workspace_and_floating_state() {
            let mut state = state_with_window("layout_widgets");
            let viewport = UiSize::new(920.0, 760.0);

            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            assert!(maybe_node_id(&document, "layout_widgets.dock").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.dock.panel.inspector").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.dock.panel.assets").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.document.tabs").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.dock.drop.left.chip").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.dock.drop.right.chip").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.dock.drop.center.chip").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.dock.drop.floating.chip").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.dock.drawers").is_some());
            assert!(
                maybe_node_id(&document, "layout_widgets.dock.drawers.drawer.assets").is_some()
            );
            assert!(maybe_node_id(
                &document,
                "layout_widgets.dock.reorder.left.inspector.before.chip"
            )
            .is_some());

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "layout_widgets.drawer.assets",
            ));
            assert!(state.layout_dock.is_hidden("assets"));
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("drawer layout");
            assert!(maybe_node_id(&document, "layout_widgets.dock.panel.assets").is_none());

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "layout_widgets.reorder.assets.before.inspector",
            ));
            assert!(!state.layout_dock.is_hidden("assets"));
            let order: Vec<_> = state
                .layout_dock
                .panel_order()
                .iter()
                .map(String::as_str)
                .collect();
            assert_eq!(order, vec!["assets", "inspector", "document"]);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "layout_widgets.float_inspector",
            ));
            assert!(state.layout_dock.is_floating("inspector"));
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("floating layout");
            assert!(maybe_node_id(&document, "layout_widgets.floating.inspector").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.dock.panel.inspector").is_none());

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "layout_widgets.dock_inspector",
            ));
            assert!(!state.layout_dock.is_floating("inspector"));
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("docked layout");
            assert!(maybe_node_id(&document, "layout_widgets.dock.panel.inspector").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.floating.inspector").is_none());
        }

        #[test]
        fn showcase_labels_window_minimum_keeps_links_inside_content() {
            let mut state = state_with_window("labels");
            state
                .desktop
                .sizes
                .insert("labels".to_string(), UiSize::new(220.0, 140.0));
            state.desktop.user_sized.insert("labels".to_string());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("labels layout");

            let content = document
                .node(node_id(&document, "showcase.windows.window.labels.content"))
                .layout()
                .rect;
            let link = document
                .node(node_id(&document, "labels.link"))
                .layout()
                .rect;
            let hyperlink = document
                .node(node_id(&document, "labels.hyperlink"))
                .layout()
                .rect;
            assert!(
                link.bottom() <= content.bottom() && hyperlink.bottom() <= content.bottom(),
                "links should fit inside minimized labels content: link={link:?} hyperlink={hyperlink:?} content={content:?}"
            );
        }

        #[test]
        fn showcase_color_picker_spawns_to_picker_minimum() {
            let state = state_with_window("color_picker");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("color picker layout");

            let window = document
                .node(node_id(&document, "showcase.windows.window.color_picker"))
                .layout()
                .rect;
            let picker = document
                .node(node_id(&document, "color.picker"))
                .layout()
                .rect;
            let content = document
                .node(node_id(
                    &document,
                    "showcase.windows.window.color_picker.content",
                ))
                .layout()
                .rect;
            let left_padding = picker.x - content.x;
            let right_padding = content.right() - picker.right();
            assert!(
                window.width <= picker.width + left_padding + right_padding + 1.0,
                "window should spawn at the color picker content minimum: window={window:?} content={content:?} picker={picker:?}"
            );
            assert!(
                picker.bottom() <= content.bottom(),
                "picker should fit inside color picker content: picker={picker:?} content={content:?}"
            );
        }

        #[test]
        fn showcase_numeric_window_minimum_tracks_wrapped_text_and_range_width() {
            let mut state = state_with_window("numeric");
            state.numeric_angle = 6.0;
            state
                .desktop
                .sizes
                .insert("numeric".to_string(), UiSize::new(220.0, 96.0));
            state.desktop.user_sized.insert("numeric".to_string());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("numeric layout");

            let content = document
                .node(node_id(
                    &document,
                    "showcase.windows.window.numeric.content",
                ))
                .layout()
                .rect;
            let note = document
                .node(node_id(&document, "numeric.note"))
                .layout()
                .rect;
            assert!(
                note.bottom() <= content.bottom(),
                "wrapped numeric note should fit inside minimized content: note={note:?} content={content:?}"
            );

            let degrees = document
                .node(node_id(&document, "numeric.drag_angle"))
                .layout()
                .rect;
            let degrees_value = document
                .node(node_id(&document, "numeric.drag_angle.value"))
                .layout()
                .rect;
            let UiContent::Text(degrees_text) = document
                .node(node_id(&document, "numeric.drag_angle.value"))
                .content()
            else {
                panic!("numeric angle value should be text");
            };
            assert_eq!(degrees_text.style.wrap, TextWrap::None);
            assert_eq!(degrees_text.intrinsic_text.as_deref(), Some("360.0 deg"));
            assert!(
                degrees.width >= degrees_value.width + 16.0,
                "degrees input should reserve width for its range maximum: degrees={degrees:?} value={degrees_value:?}"
            );
        }

        #[test]
        fn showcase_multiline_text_inputs_accept_enter_as_newline() {
            let mut state = state_with_window("text_input");
            for action in [
                "text.multiline.edit",
                "text.area.edit",
                "text.code_editor.edit",
            ] {
                let before = match action {
                    "text.multiline.edit" => state.multiline_text.text().to_string(),
                    "text.area.edit" => state.text_area_text.text().to_string(),
                    "text.code_editor.edit" => state.code_editor_text.text().to_string(),
                    _ => unreachable!(),
                };
                state.update(WidgetAction::text_edit(
                    UiNodeId::root(),
                    action,
                    UiInputEvent::Key {
                        key: KeyCode::Enter,
                        modifiers: KeyModifiers::NONE,
                    },
                ));
                let after = match action {
                    "text.multiline.edit" => state.multiline_text.text(),
                    "text.area.edit" => state.text_area_text.text(),
                    "text.code_editor.edit" => state.code_editor_text.text(),
                    _ => unreachable!(),
                };
                assert_eq!(after, &format!("{before}\n"), "{action}");
            }
        }

        #[test]
        fn showcase_text_input_has_single_selectable_demo() {
            let state = state_with_window("text_input");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("text input layout");

            assert!(maybe_node_id(&document, "text.selectable").is_some());
            assert!(maybe_node_id(&document, "text.selectable_helper").is_none());
        }

        #[test]
        fn showcase_select_dropdown_popups_are_anchored_to_controls() {
            let mut state = state_with_window("selection");
            state.combo_open = true;
            state.dropdown.open(&select_options());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("selection layout");

            for (trigger_name, popup_name) in [
                ("combo.toggle", "selection.combo_menu"),
                ("selection.dropdown", "selection.dropdown.popup"),
            ] {
                let trigger = document
                    .node(node_id(&document, trigger_name))
                    .layout()
                    .rect;
                let popup = document.node(node_id(&document, popup_name)).layout().rect;
                assert!(
                    (popup.x - trigger.x).abs() <= 1.0,
                    "{popup_name} should align to {trigger_name}: popup={popup:?} trigger={trigger:?}"
                );
                assert!(
                    popup.y >= trigger.bottom() - 1.0,
                    "{popup_name} should open below {trigger_name}: popup={popup:?} trigger={trigger:?}"
                );
            }
        }

        #[test]
        fn showcase_popup_panel_close_button_label_is_visible() {
            let state = state_with_window("popup_panel");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("popup panel layout");

            let close_label = node_id(&document, "popup_panel.inline_preview.close.label");
            let text_item = document
                .paint_list()
                .items
                .into_iter()
                .find(|item| item.node == close_label && matches!(item.kind, PaintKind::Text(_)))
                .expect("popup close label paint");
            assert!(
                text_item.rect.width >= 6.0 && text_item.clip_rect.width >= 6.0,
                "popup close label should be visible: rect={:?} clip={:?}",
                text_item.rect,
                text_item.clip_rect
            );
        }

        #[test]
        fn showcase_animation_stages_have_full_size_scene_viewports() {
            let mut state = state_with_window("animation");
            state
                .desktop
                .sizes
                .insert("animation".to_string(), UiSize::new(220.0, 140.0));
            state.desktop.user_sized.insert("animation".to_string());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            for (stage_name, scene_name) in [
                ("animation.live.stage", "animation.live.orb"),
                ("animation.scrub.stage", "animation.scrub.shape"),
                ("animation.state.stage", "animation.state.panel"),
                (
                    "animation.interaction.stage",
                    "animation.interaction.target",
                ),
            ] {
                let stage = document.node(node_id(&document, stage_name)).layout().rect;
                let scene = document.node(node_id(&document, scene_name)).layout().rect;
                assert!(stage.width >= ANIMATION_STAGE_MIN_WIDTH, "{stage_name}");
                assert!(stage.height >= ANIMATION_STAGE_HEIGHT, "{stage_name}");
                assert!(
                    (scene.x - stage.x).abs() < 0.01
                        && (scene.y - stage.y).abs() < 0.01
                        && (scene.width - stage.width).abs() < 0.01
                        && (scene.height - stage.height).abs() < 0.01,
                    "{scene_name} should fill {stage_name}: scene={scene:?} stage={stage:?}"
                );
            }
        }

        #[test]
        fn showcase_animation_boolean_panel_stays_inside_stage_at_minimum_width() {
            for open in [false, true] {
                let mut state = state_with_window("animation");
                state.animation_open = open;
                state
                    .desktop
                    .sizes
                    .insert("animation".to_string(), UiSize::new(220.0, 140.0));
                state.desktop.user_sized.insert("animation".to_string());

                let viewport = UiSize::new(900.0, 760.0);
                let mut document = state.view(viewport);
                document
                    .compute_layout(viewport, &mut ApproxTextMeasurer)
                    .expect("showcase layout");

                let stage = document
                    .node(node_id(&document, "animation.state.stage"))
                    .layout()
                    .rect;
                let scene = document
                    .node(node_id(&document, "animation.state.panel"))
                    .layout()
                    .rect;
                let primitive = first_scene_rect(&document, "animation.state.panel");
                let panel = UiRect::new(
                    scene.x + primitive.x,
                    scene.y + primitive.y,
                    primitive.width,
                    primitive.height,
                );
                assert!(
                    stage.contains_rect(panel),
                    "open={open}: panel primitive should fit inside state stage: panel={panel:?} stage={stage:?}"
                );
            }
        }

        #[test]
        fn showcase_scrubbed_animation_uses_morphing_polygon_topology() {
            let mut state = state_with_window("animation");
            state.animation_scrub = 0.5;

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let node = document.node(node_id(&document, "animation.scrub.shape"));
            let UiContent::Scene(primitives) = node.content() else {
                panic!("scrubbed animation shape should be a scene");
            };
            let Some(ScenePrimitive::MorphPolygon {
                from_points,
                to_points,
                amount,
                ..
            }) = primitives.first()
            else {
                panic!("scrubbed animation should use topology morph primitive");
            };
            assert_eq!(from_points.len(), 5);
            assert_eq!(to_points.len(), 5);
            let top = from_points
                .iter()
                .map(|point| point.y)
                .fold(f32::INFINITY, f32::min);
            let bottom = from_points
                .iter()
                .map(|point| point.y)
                .fold(f32::NEG_INFINITY, f32::max);
            let left = from_points
                .iter()
                .map(|point| point.x)
                .fold(f32::INFINITY, f32::min);
            let right = from_points
                .iter()
                .map(|point| point.x)
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                from_points.iter().filter(|point| point.y == top).count() >= 2,
                "{from_points:?}"
            );
            assert!(
                from_points.iter().filter(|point| point.y == bottom).count() >= 2,
                "{from_points:?}"
            );
            assert!(
                from_points.iter().filter(|point| point.x == left).count() >= 2,
                "{from_points:?}"
            );
            assert!(
                from_points.iter().filter(|point| point.x == right).count() >= 2,
                "{from_points:?}"
            );
            assert!((*amount - 0.5).abs() < 0.01, "{amount}");
        }

        #[test]
        fn showcase_interaction_animation_tracks_pointer_input() {
            let mut state = state_with_window("animation");
            state.animation_timed_expanded = false;
            state.animation_scrub_expanded = false;
            state.animation_state_expanded = false;
            state.animation_interaction_expanded = true;
            let viewport = UiSize::new(900.0, 1120.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let target = node_id(&document, "animation.interaction.target");
            let target_rect = document.node(target).layout().rect;
            let input = document.handle_input(UiInputEvent::PointerMove(UiPoint::new(
                target_rect.x + target_rect.width * 0.75,
                target_rect.y + target_rect.height * 0.5,
            )));
            let hovered_name = input
                .hovered
                .map(|hovered| document.node(hovered).name().to_owned());
            assert_eq!(
                input.hovered,
                Some(target),
                "hovered={hovered_name:?} target_rect={target_rect:?}"
            );

            let values = document
                .node(target)
                .animation()
                .as_ref()
                .expect("interaction target animation")
                .values();
            assert!(values.morph > 0.70, "{values:?}");
            assert_eq!(values.translate, UiPoint::new(0.0, 0.0));

            let input = document.handle_input(UiInputEvent::PointerMove(UiPoint::new(
                target_rect.x + target_rect.width * 0.75,
                target_rect.y + target_rect.height * 0.5,
            )));
            assert_eq!(
                input.hovered,
                Some(target),
                "interaction animation should not move its own hit target"
            );

            document.handle_input(UiInputEvent::PointerMove(UiPoint::new(
                target_rect.x - 8.0,
                target_rect.y - 8.0,
            )));
            let values = document
                .node(target)
                .animation()
                .as_ref()
                .expect("interaction target animation")
                .values();
            assert!(values.morph <= 0.01, "{values:?}");
        }

        #[test]
        fn showcase_animation_window_minimum_sums_animation_sections() {
            let mut state = state_with_window("animation");
            state
                .desktop
                .sizes
                .insert("animation".to_string(), UiSize::new(160.0, 96.0));
            state.desktop.user_sized.insert("animation".to_string());

            let viewport = UiSize::new(900.0, 1120.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let window = document
                .node(node_id(&document, "showcase.windows.window.animation"))
                .layout()
                .rect;
            let section = document
                .node(node_id(&document, "animation.section_scroll"))
                .layout()
                .rect;
            assert!(window.height >= ANIMATION_STAGE_HEIGHT * 4.0, "{window:?}");
            assert!(
                section.height >= ANIMATION_STAGE_HEIGHT * 4.0,
                "section={section:?}"
            );
        }

        #[test]
        fn showcase_animation_sections_use_collapsing_headers() {
            let mut state = state_with_window("animation");
            let viewport = UiSize::new(900.0, 1120.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            for section in [
                "animation.timed",
                "animation.scrub",
                "animation.state",
                "animation.interaction",
            ] {
                assert!(maybe_node_id(&document, &format!("{section}.header")).is_some());
                assert!(maybe_node_id(&document, &format!("{section}.body")).is_some());
            }

            state.animation_scrub_expanded = false;
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            assert!(maybe_node_id(&document, "animation.scrub.header").is_some());
            assert!(maybe_node_id(&document, "animation.scrub.body").is_none());
            assert!(maybe_node_id(&document, "animation.scrub.stage").is_none());
        }

        #[test]
        fn showcase_animation_collapsed_header_labels_keep_full_width() {
            let mut state = state_with_window("animation");
            state.animation_timed_expanded = false;
            state.animation_scrub_expanded = false;
            state.animation_state_expanded = false;
            state.animation_interaction_expanded = false;
            state
                .desktop
                .sizes
                .insert("animation".to_string(), UiSize::new(160.0, 96.0));
            state.desktop.user_sized.insert("animation".to_string());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            for label in [
                "animation.timed.label",
                "animation.scrub.label",
                "animation.state.label",
                "animation.interaction.label",
            ] {
                let label = node_id(&document, label);
                let intrinsic = document
                    .intrinsic_size(label, &mut ApproxTextMeasurer)
                    .expect("label intrinsic size");
                let rect = document.node(label).layout().rect;
                assert!(
                    rect.width + 0.5 >= intrinsic.preferred.width,
                    "{label:?}: rect={rect:?} intrinsic={intrinsic:?}"
                );
            }
        }

        #[test]
        fn showcase_slider_color_button_opens_inline_picker() {
            let mut state = state_with_window("slider");
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "slider.trailing_color_button",
            ));
            assert!(state.slider_trailing_picker_open);

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            assert!(document
                .nodes()
                .iter()
                .any(|node| node.name() == "slider.trailing_picker"));
            assert!(!document
                .nodes()
                .iter()
                .any(|node| node.name() == "slider.trailing_color_button.label"));
        }

        #[test]
        #[allow(clippy::field_reassign_with_default)]
        fn showcase_progress_phase_does_not_wrap_on_tick() {
            let mut state = ShowcaseState::default();
            state.progress_phase = std::f32::consts::TAU - 0.001;
            state.update(WidgetAction::activate(UiNodeId::root(), "runtime.tick"));
            assert!(state.progress_phase > std::f32::consts::TAU);
        }
    }
}
