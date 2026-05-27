#![allow(dead_code, unused_imports)]

mod showcase_app {
    #![allow(dead_code, unused_imports)]

    include!("../examples/showcase.rs");

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::{Duration, Instant};

        use operad::diagnostics::AuditWarning;
        use operad::host::{
            collect_document_widget_actions, process_document_frame,
            process_host_frame_input_with_target_resolver, HostDocumentFrameRequest,
            HostDocumentFrameState, HostFrameOutput, HostFrameRequest,
        };
        use operad::input::{RawInputEvent, RawPointerEvent};
        use operad::platform::{CursorRequest, CursorShape, PlatformRequest};
        use operad::renderer::{RenderTarget, RendererAdapter};
        use operad::testing::EmptyResourceResolver;
        #[cfg(feature = "wgpu")]
        use operad::wgpu_renderer::WgpuRenderer;
        use operad::{
            AccessibilityChecked, AnimationInputValue, ApproxTextMeasurer, CosmicTextMeasurer,
            KeyCode, KeyModifiers, PaintKind, PointerButton, PointerEventKind, TextMeasurer,
            TextWrap, UiContent, UiFocusState, UiInputEvent, UiNodeLayoutConstraint, UiWheelEvent,
            WidgetDrag, WidgetDragPhase, WidgetPointerEdit, WidgetTextEdit, WidgetValueEditPhase,
        };

        fn state_with_window(id: &str) -> ShowcaseState {
            let mut state = ShowcaseState::default();
            state.windows.clear_all();
            *state.windows.slot_mut(id).expect("known showcase window") = true;
            state.desktop.ensure_window(id, window_defaults(id));
            if id == "diagnostics" {
                state.diagnostics_page = DiagnosticsPage::LegacyAll;
            }
            if id == "overlays" {
                state.overlay_popup_open = true;
                state.overlay_modal_open = true;
                state.toast_visible = true;
            }
            state
        }

        fn state_with_all_windows(viewport: UiSize) -> ShowcaseState {
            let mut state = ShowcaseState::default();
            state.windows.clear_all();
            state.last_desktop_size = desktop_size_for_viewport(viewport);
            state.update(WidgetAction::activate(UiNodeId::root(), "window.add_all"));
            state.overlay_popup_open = true;
            state.overlay_modal_open = true;
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

        fn raw_pointer(kind: PointerEventKind, position: UiPoint, timestamp: u64) -> RawInputEvent {
            RawInputEvent::Pointer(RawPointerEvent::new(kind, position, timestamp))
        }

        fn apply_showcase_pointer_frame(
            state: &mut ShowcaseState,
            frame_state: &mut HostDocumentFrameState,
            viewport: UiSize,
            event: RawInputEvent,
        ) -> UiDocument {
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("pre-input showcase layout");
            let host_output = process_host_frame_input_with_target_resolver(
                frame_state.host_frame_request(viewport).raw_event(event),
                |event, interaction| match event {
                    RawInputEvent::Pointer(pointer) => interaction
                        .drag_capture
                        .filter(|capture| {
                            capture.pointer_id == pointer.pointer_id
                                && matches!(
                                    pointer.kind,
                                    PointerEventKind::Move | PointerEventKind::Cancel
                                )
                        })
                        .map(|capture| capture.target)
                        .or_else(|| document.hit_test(pointer.position)),
                    RawInputEvent::Wheel(wheel) => interaction
                        .wheel_target
                        .or(interaction.hovered)
                        .or_else(|| document.hit_test(wheel.position)),
                    RawInputEvent::Keyboard(_)
                    | RawInputEvent::Text(_)
                    | RawInputEvent::Focus(_) => None,
                },
            );
            frame_state.apply_host_frame_output(&host_output);
            let frame = process_document_frame(
                &mut document,
                &mut ApproxTextMeasurer,
                frame_state.document_frame_request(
                    viewport,
                    RenderTarget::window("showcase", viewport),
                    host_output,
                ),
            )
            .expect("showcase input frame");
            for action in collect_document_widget_actions(&document, &frame) {
                state.update(action);
            }
            frame_state.apply_document_frame_output(&frame);
            document
        }

        fn text_paint_rect(document: &UiDocument, name: &str) -> UiRect {
            let node = node_id(document, name);
            document
                .paint_list()
                .items
                .into_iter()
                .find(|item| item.node == node && matches!(item.kind, PaintKind::Text(_)))
                .map(|item| item.rect)
                .unwrap_or_else(|| panic!("missing text paint for {name:?}"))
        }

        fn text_content(document: &UiDocument, name: &str) -> String {
            let UiContent::Text(text) = document.node(node_id(document, name)).content() else {
                panic!("{name:?} should be a text node");
            };
            text.text.clone()
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
                    | AuditWarning::DuplicateNodeName { .. }
                    | AuditWarning::PaintItemEmptyClip { .. }
            )
        }

        fn rects_overlap(left: UiRect, right: UiRect) -> bool {
            left.x < right.x + right.width
                && left.x + left.width > right.x
                && left.y < right.y + right.height
                && left.y + left.height > right.y
        }

        fn rect_center(rect: UiRect) -> UiPoint {
            UiPoint::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
        }

        fn assert_rect_centers_match(left: UiRect, right: UiRect) {
            let left = rect_center(left);
            let right = rect_center(right);
            assert!(
                (left.x - right.x).abs() < 0.01,
                "x centers differ: {left:?} vs {right:?}"
            );
            assert!(
                (left.y - right.y).abs() < 0.01,
                "y centers differ: {left:?} vs {right:?}"
            );
        }

        fn assert_circle_paint_centered_on_rect(
            document: &UiDocument,
            rect: UiRect,
            node: UiNodeId,
        ) {
            let expected = rect_center(rect);
            let Some(item) =
                document.paint_list().items.into_iter().find(|item| {
                    item.node == node && matches!(item.kind, PaintKind::Circle { .. })
                })
            else {
                panic!(
                    "missing circle paint for node {:?}",
                    document.node(node).name()
                );
            };
            let PaintKind::Circle { center, .. } = item.kind else {
                unreachable!();
            };
            assert!(
                (expected.x - center.x).abs() < 0.01,
                "paint x centers differ: {expected:?} vs {center:?}"
            );
            assert!(
                (expected.y - center.y).abs() < 0.01,
                "paint y centers differ: {expected:?} vs {center:?}"
            );
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
            let viewport = UiSize::new(1200.0, 900.0);
            let state = state_with_all_windows(viewport);
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
            let viewport = UiSize::new(1180.0, 820.0);
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
        fn showcase_widget_menu_checked_mark_uses_high_contrast_color() {
            let viewport = UiSize::new(900.0, 760.0);
            let state = state_with_window("labels");
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let expected = Theme::dark().colors.accent_text;
            let check_node = document.node(node_id(&document, "controls.labels.check"));
            let UiContent::Scene(primitives) = check_node.content() else {
                panic!("controls.labels.check should be a vector check mark");
            };
            let line_colors = primitives
                .iter()
                .filter_map(|primitive| match primitive {
                    ScenePrimitive::Line { stroke, .. } => Some(stroke.color),
                    _ => None,
                })
                .collect::<Vec<_>>();

            assert_eq!(line_colors, vec![expected, expected]);
        }

        #[test]
        fn showcase_widget_menu_gaps_do_not_toggle_adjacent_checkboxes() {
            let viewport = UiSize::new(900.0, 760.0);
            let mut state = ShowcaseState::default();
            let list_height = controls_list_viewport_height(viewport.height);
            let content_height = controls_list_content_height();
            state.controls_scroll = scroll_state(content_height, list_height, content_height);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let timeline = node_id(&document, "controls.timeline");
            let canvas = node_id(&document, "controls.canvas");
            let theme = node_id(&document, "controls.theme");
            let timeline_rect = document.node(timeline).layout().rect;
            let canvas_rect = document.node(canvas).layout().rect;
            let theme_rect = document.node(theme).layout().rect;

            assert!(timeline_rect.height <= CONTROLS_WIDGET_ROW_HEIGHT + 0.01);
            assert!(
                canvas_rect.y - timeline_rect.bottom() >= CONTROLS_WIDGET_ROW_GAP - 0.01,
                "timeline={timeline_rect:?} canvas={canvas_rect:?}"
            );
            assert!(
                theme_rect.y - canvas_rect.bottom() >= CONTROLS_WIDGET_ROW_GAP - 0.01,
                "canvas={canvas_rect:?} theme={theme_rect:?}"
            );

            for (above, below) in [(timeline, canvas), (canvas, theme)] {
                let above_rect = document.node(above).layout().rect;
                let below_rect = document.node(below).layout().rect;
                let point = UiPoint::new(
                    above_rect.x + 8.0,
                    above_rect.bottom() + (below_rect.y - above_rect.bottom()) * 0.5,
                );
                let hit = document.hit_test(point);
                assert_ne!(hit, Some(above), "gap point hit row above: {point:?}");
                assert_ne!(hit, Some(below), "gap point hit row below: {point:?}");
                if let Some(hit) = hit {
                    let action = document
                        .node(hit)
                        .action()
                        .and_then(|action| action.action_id())
                        .map(|id| id.as_str());
                    assert!(
                        action.is_none_or(|action| !action.starts_with("window.toggle.")),
                        "gap point should not hit a widget toggle action: point={point:?} action={action:?}"
                    );
                }
            }
        }

        #[test]
        fn showcase_theme_demo_switches_the_app_theme() {
            let mut state = state_with_window("theme");
            let viewport = UiSize::new(1180.0, 820.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("theme layout");

            assert_eq!(
                text_content(&document, "theme.current"),
                "Current theme: operad.dark.v3"
            );
            assert!(maybe_node_id(&document, "theme.choice.light").is_some());
            assert!(maybe_node_id(&document, "theme.choice.dark").is_some());
            assert!(maybe_node_id(&document, "theme.choice.bubblegum").is_some());
            assert_eq!(
                document.node(document.root()).visual().fill,
                Theme::dark().colors.canvas
            );

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "theme.demo.bubblegum",
            ));
            let mut bubblegum = state.view(viewport);
            bubblegum
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("bubblegum theme layout");

            assert_eq!(
                text_content(&bubblegum, "theme.current"),
                "Current theme: operad.bubblegum.v3"
            );
            assert_eq!(
                bubblegum.node(bubblegum.root()).visual().fill,
                Theme::bubblegum().colors.canvas
            );
            assert_eq!(
                bubblegum
                    .node(node_id(&bubblegum, "showcase.controls"))
                    .visual()
                    .fill,
                Theme::bubblegum().colors.surface
            );
        }

        #[test]
        fn showcase_light_theme_keeps_existing_demo_text_readable() {
            let mut state = state_with_window("labels");
            state.update(WidgetAction::activate(UiNodeId::root(), "theme.demo.light"));
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("light labels layout");

            let UiContent::Text(label) =
                document.node(node_id(&document, "labels.plain")).content()
            else {
                panic!("labels.plain should be text");
            };
            assert_eq!(label.style.color, Theme::light().colors.text);
            assert!(
                label
                    .style
                    .color
                    .contrast_ratio(Theme::light().colors.surface)
                    >= 7.0
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
        fn showcase_initial_frame_organizes_default_visible_windows() {
            let viewport = UiSize::new(2200.0, 1180.0);
            let ids = ["labels", "buttons", "color_picker", "canvas"];
            let mut state = ShowcaseState::default();
            state.prepare_frame(viewport);

            for id in ids {
                assert!(
                    state.windows.is_visible(id),
                    "default-visible window {id} should remain visible"
                );
                assert!(
                    !state.desktop.is_collapsed(id),
                    "wide startup viewport should keep {id} expanded"
                );
            }

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
                    left_rect.y >= SHOWCASE_ORGANIZE_BUTTON_RESERVED_HEIGHT,
                    "{left_id} overlaps the organize control band: {left_rect:?}"
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
        fn showcase_floating_window_drag_does_not_close_window() {
            let viewport = UiSize::new(1180.0, 820.0);
            let mut state = state_with_window("labels");
            let mut frame_state = HostDocumentFrameState::default();
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            let title_bar = document
                .node(node_id(
                    &document,
                    "showcase.windows.window.labels.title_bar",
                ))
                .layout()
                .rect;
            let start = UiPoint::new(title_bar.x + 120.0, title_bar.y + title_bar.height * 0.5);

            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(PointerEventKind::Down(PointerButton::Primary), start, 1),
            );
            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(
                    PointerEventKind::Move,
                    UiPoint::new(start.x + 80.0, start.y + 30.0),
                    2,
                ),
            );
            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(
                    PointerEventKind::Up(PointerButton::Primary),
                    UiPoint::new(start.x + 80.0, start.y + 30.0),
                    3,
                ),
            );

            assert!(
                state.windows.is_visible("labels"),
                "dragging the title bar must move the window, not close it"
            );
            let moved = state
                .desktop
                .position("labels", default_window_position("labels"));
            assert!(
                moved.x > default_window_position("labels").x + 1.0
                    || moved.y > default_window_position("labels").y + 1.0,
                "drag should update the stored position: {moved:?}"
            );
        }

        #[test]
        fn showcase_floating_window_header_control_drags_do_not_activate() {
            let viewport = UiSize::new(1180.0, 820.0);

            let mut close_state = state_with_window("labels");
            let mut close_frame_state = HostDocumentFrameState::default();
            let mut document = close_state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            let close_rect = document
                .node(node_id(&document, "showcase.windows.window.labels.close"))
                .layout()
                .rect;
            let close_start = rect_center(close_rect);
            apply_showcase_pointer_frame(
                &mut close_state,
                &mut close_frame_state,
                viewport,
                raw_pointer(
                    PointerEventKind::Down(PointerButton::Primary),
                    close_start,
                    1,
                ),
            );
            apply_showcase_pointer_frame(
                &mut close_state,
                &mut close_frame_state,
                viewport,
                raw_pointer(
                    PointerEventKind::Move,
                    UiPoint::new(close_start.x - 40.0, close_start.y + 24.0),
                    2,
                ),
            );
            apply_showcase_pointer_frame(
                &mut close_state,
                &mut close_frame_state,
                viewport,
                raw_pointer(
                    PointerEventKind::Up(PointerButton::Primary),
                    UiPoint::new(close_start.x - 40.0, close_start.y + 24.0),
                    3,
                ),
            );
            assert!(
                close_state.windows.is_visible("labels"),
                "dragging from a close button must not close the window"
            );

            let mut collapse_state = state_with_window("labels");
            let mut collapse_frame_state = HostDocumentFrameState::default();
            let mut document = collapse_state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            let collapse_rect = document
                .node(node_id(
                    &document,
                    "showcase.windows.window.labels.collapse",
                ))
                .layout()
                .rect;
            let collapse_start = rect_center(collapse_rect);
            apply_showcase_pointer_frame(
                &mut collapse_state,
                &mut collapse_frame_state,
                viewport,
                raw_pointer(
                    PointerEventKind::Down(PointerButton::Primary),
                    collapse_start,
                    1,
                ),
            );
            apply_showcase_pointer_frame(
                &mut collapse_state,
                &mut collapse_frame_state,
                viewport,
                raw_pointer(
                    PointerEventKind::Move,
                    UiPoint::new(collapse_start.x + 40.0, collapse_start.y + 24.0),
                    2,
                ),
            );
            apply_showcase_pointer_frame(
                &mut collapse_state,
                &mut collapse_frame_state,
                viewport,
                raw_pointer(
                    PointerEventKind::Up(PointerButton::Primary),
                    UiPoint::new(collapse_start.x + 40.0, collapse_start.y + 24.0),
                    3,
                ),
            );
            assert!(
                !collapse_state.desktop.is_collapsed("labels"),
                "dragging from a collapse button must not collapse the window"
            );
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
                window.width <= 340.0,
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
        fn showcase_date_picker_demonstrates_custom_and_week_ranges() {
            let mut state = state_with_window("date_picker");
            state.update(WidgetAction::activate(UiNodeId::root(), "date.mode.range"));
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            let mut measurer = CosmicTextMeasurer::new();
            document
                .compute_layout(viewport, &mut measurer)
                .expect("showcase layout");

            for name in [
                "date.mode.single",
                "date.mode.range",
                "date.mode.week",
                "date.bounds.toggle",
                "date.clear",
            ] {
                assert!(maybe_node_id(&document, name).is_some(), "{name}");
            }

            let middle = document.node(node_id(&document, "date.picker.day.2026-05-14"));
            assert_eq!(
                middle.accessibility().and_then(|meta| meta.selected),
                Some(true)
            );
            assert!(middle
                .accessibility()
                .and_then(|meta| meta.hint.as_deref())
                .is_some_and(|hint| hint.contains("inside selected range")));

            state.update(WidgetAction::activate(UiNodeId::root(), "date.mode.week"));
            assert_eq!(
                state.date_range.range,
                Some(ext_widgets::CalendarDateRange::new(
                    CalendarDate::new(2026, 5, 10).unwrap(),
                    CalendarDate::new(2026, 5, 16).unwrap(),
                ))
            );
            state.update(WidgetAction::activate(UiNodeId::root(), "date.week.monday"));
            assert_eq!(
                state.date_range.range,
                Some(ext_widgets::CalendarDateRange::new(
                    CalendarDate::new(2026, 5, 4).unwrap(),
                    CalendarDate::new(2026, 5, 10).unwrap(),
                ))
            );
        }

        #[test]
        fn showcase_property_inspector_minimum_keeps_text_visible() {
            let mut state = state_with_window("property_inspector");
            state
                .desktop
                .sizes
                .insert("property_inspector".to_string(), UiSize::new(140.0, 120.0));
            state
                .desktop
                .user_sized
                .insert("property_inspector".to_string());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            let mut measurer = CosmicTextMeasurer::new();
            document
                .compute_layout(viewport, &mut measurer)
                .expect("showcase layout");

            let window = document
                .node(node_id(
                    &document,
                    "showcase.windows.window.property_inspector",
                ))
                .layout()
                .rect;
            assert!(
                window.width > 140.0,
                "property inspector should grow from tiny user sizes to its text-driven minimum: {window:?}"
            );

            for name in [
                "property_inspector.grid.row.target.label",
                "property_inspector.grid.row.target.value",
                "property_inspector.grid.row.radius.label",
                "property_inspector.grid.row.radius.value",
                "property_inspector.grid.row.state.value",
            ] {
                let node = node_id(&document, name);
                let text_item = document
                    .paint_list()
                    .items
                    .into_iter()
                    .find(|item| item.node == node && matches!(item.kind, PaintKind::Text(_)))
                    .unwrap_or_else(|| panic!("missing text paint for {name}"));
                assert!(
                    text_item.rect.x >= text_item.clip_rect.x - 0.5
                        && text_item.rect.y >= text_item.clip_rect.y - 0.5
                        && text_item.rect.right() <= text_item.clip_rect.right() + 0.5
                        && text_item.rect.bottom() <= text_item.clip_rect.bottom() + 0.5,
                    "{name} should not be clipped at the property inspector minimum: text={:?} clip={:?}",
                    text_item.rect,
                    text_item.clip_rect
                );
            }
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
        fn showcase_add_all_then_repeated_organize_stays_non_overlapping() {
            let viewport = UiSize::new(2200.0, 1180.0);
            let mut state = ShowcaseState::default();
            let mut initial = state.view(viewport);
            initial
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                .expect("initial showcase layout");

            state.update(WidgetAction::activate(UiNodeId::root(), "window.add_all"));
            let after_add_all = organized_window_rects(&mut state, viewport);
            assert_window_rects_do_not_overlap("after Add all", &after_add_all);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.organize_open",
            ));
            let after_first_organize = organized_window_rects(&mut state, viewport);
            assert_window_rects_do_not_overlap("after first Organize", &after_first_organize);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.organize_open",
            ));
            let after_second_organize = organized_window_rects(&mut state, viewport);
            assert_window_rects_do_not_overlap("after second Organize", &after_second_organize);

            assert_eq!(
                after_first_organize, after_second_organize,
                "Organize should be idempotent once all windows are open"
            );
        }

        fn organized_window_rects(
            state: &mut ShowcaseState,
            viewport: UiSize,
        ) -> Vec<(&'static str, UiRect)> {
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                .expect("showcase layout");
            SHOWCASE_WIDGET_WINDOW_IDS
                .into_iter()
                .filter(|id| state.windows.is_visible(id))
                .map(|id| {
                    let name = format!("showcase.windows.window.{id}");
                    (id, document.node(node_id(&document, &name)).layout().rect)
                })
                .collect()
        }

        fn assert_window_rects_do_not_overlap(label: &str, rects: &[(&str, UiRect)]) {
            for (left_index, (left_id, left_rect)) in rects.iter().enumerate() {
                for (right_id, right_rect) in rects.iter().skip(left_index + 1) {
                    assert!(
                        !rects_overlap(*left_rect, *right_rect),
                        "{label}: {left_id} {left_rect:?} overlapped {right_id} {right_rect:?}"
                    );
                }
            }
        }

        #[test]
        fn showcase_organize_short_desktop_collapses_overflow_suffix() {
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
            state.last_desktop_size = UiSize::new(1900.0, 620.0);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.organize_open",
            ));

            let first_collapsed = ids
                .iter()
                .position(|id| state.desktop.is_collapsed(id))
                .expect("short desktop should collapse at least one overflow window");
            assert!(
                first_collapsed > 0,
                "organize should keep the largest fitting prefix expanded"
            );
            for id in ids.iter().take(first_collapsed) {
                assert!(
                    !state.desktop.is_collapsed(id),
                    "organize should keep prefix window {id} expanded"
                );
            }
            for id in ids.iter().skip(first_collapsed) {
                assert!(
                    state.desktop.is_collapsed(id),
                    "organize should collapse overflow suffix window {id}"
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
            assert_eq!(state.virtual_table_value_width, 120.0);
        }

        #[test]
        fn showcase_lists_tables_use_scroll_viewports_instead_of_manual_scrollbar_pairs() {
            let state = state_with_window("lists_tables");
            let viewport = UiSize::new(920.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("lists layout");

            for old_scrollbar in [
                "lists_tables.scroll_area.scrollbar",
                "lists_tables.virtual_list.scrollbar",
                "lists_tables.data_table.scrollbar",
                "lists_tables.virtualized_table.scrollbar",
            ] {
                assert!(
                    maybe_node_id(&document, old_scrollbar).is_none(),
                    "{old_scrollbar} should not be hand-composed next to a scroll viewport"
                );
            }

            for viewport_name in [
                "lists_tables.scroll_area",
                "lists_tables.virtual_list",
                "lists_tables.virtualized_table.body",
            ] {
                assert!(
                    document
                        .node(node_id(&document, viewport_name))
                        .has_auto_scrollbar(),
                    "{viewport_name} should publish automatic scrollbar affordances"
                );
            }
            assert!(maybe_node_id(&document, "lists_tables.table_header").is_none());
            assert!(maybe_node_id(&document, "lists_tables.data_table").is_none());
        }

        #[test]
        fn showcase_lists_tables_has_compact_clear_table_layout() {
            let state = state_with_window("lists_tables");
            let viewport = UiSize::new(920.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("lists layout");

            let window = document
                .node(node_id(&document, "showcase.windows.window.lists_tables"))
                .layout()
                .rect;
            assert!(
                window.height <= 540.0,
                "Lists and tables should spawn as a compact demo, not a long mostly empty window: {window:?}"
            );

            let scroll_list = document
                .node(node_id(&document, "lists_tables.scroll_area.column"))
                .layout()
                .rect;
            let virtual_list = document
                .node(node_id(&document, "lists_tables.virtual_list.column"))
                .layout()
                .rect;
            assert!(
                virtual_list.x > scroll_list.x,
                "scroll and virtual list demos should share the top row instead of stretching the window vertically: scroll={scroll_list:?} virtual={virtual_list:?}"
            );

            let header = document
                .node(node_id(&document, "lists_tables.virtualized_table.header"))
                .layout()
                .rect;
            let value_header = document
                .node(node_id(
                    &document,
                    "lists_tables.virtualized_table.header.value",
                ))
                .layout()
                .rect;
            assert!(
                value_header.right() >= header.right() - 90.0,
                "virtualized table columns should occupy the header rather than leaving a large unexplained blank zone: value={value_header:?} header={header:?}"
            );

            let resize = document
                .node(node_id(
                    &document,
                    "lists_tables.virtualized_table.header.value.resize",
                ))
                .layout()
                .rect;
            let resize_line = document
                .node(node_id(
                    &document,
                    "lists_tables.virtualized_table.header.value.resize.line",
                ))
                .layout()
                .rect;
            assert_eq!(resize.width, 8.0);
            assert_eq!(resize_line.width, 2.0);
            assert!(
                resize_line.height < resize.height,
                "column resize affordance should read as a small grip, not a full-height unexplained bar: resize={resize:?} line={resize_line:?}"
            );
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
            let preview = snapshot
                .node("diagnostics.sample.preview")
                .expect("preview node");
            assert!(snapshot.hitbox("diagnostics.sample.preview").is_some());
            assert!(snapshot.overlaps_for(preview.id).any(|overlap| {
                overlap.back.name == "diagnostics.sample.hotspot"
                    || overlap.front.name == "diagnostics.sample.hotspot"
            }));
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
        fn showcase_diagnostics_default_page_stays_lightweight() {
            let mut state = ShowcaseState::default();
            state.windows.clear_all();
            *state
                .windows
                .slot_mut("diagnostics")
                .expect("known showcase window") = true;
            state
                .desktop
                .ensure_window("diagnostics", window_defaults("diagnostics"));

            assert_eq!(state.diagnostics_page, DiagnosticsPage::Overview);

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("diagnostics overview layout");

            assert!(
                document.nodes().len() < 2_000,
                "diagnostics overview should not build the full debug catalog, got {} nodes",
                document.nodes().len()
            );
            assert!(maybe_node_id(&document, "diagnostics.pages").is_some());
            assert!(maybe_node_id(&document, "diagnostics.health.panel").is_some());
            assert!(maybe_node_id(&document, "diagnostics.layout_cost_timeline.panel").is_none());

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "diagnostics.page.layout",
            ));
            assert_eq!(state.diagnostics_page, DiagnosticsPage::Layout);

            let mut layout_document = state.view(viewport);
            layout_document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("diagnostics layout page");
            assert!(
                maybe_node_id(&layout_document, "diagnostics.layout_cost_timeline.panel").is_some()
            );
            assert!(maybe_node_id(&layout_document, "diagnostics.dispatch.panel").is_none());
            assert!(
                layout_document.nodes().len() < 3_000,
                "diagnostics layout page should build one category, got {} nodes",
                layout_document.nodes().len()
            );
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
        fn showcase_diagnostics_hitbox_overlay_preview_clips_children() {
            let state = state_with_window("diagnostics");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("diagnostics layout");

            let preview = node_id(&document, "diagnostics.hitbox.preview");
            let preview_rect = document.node(preview).layout().rect;
            assert!(
                preview_rect.height >= 170.0,
                "diagnostics hitbox overlay preview collapsed: {preview_rect:?}"
            );
            let visual = node_id(&document, "diagnostics.hitbox.visual");
            assert!(
                document.node(visual).children().len() >= 3,
                "hitbox overlay should draw hitbox, paint, and overlap nodes"
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
                    "diagnostics hitbox overlay leaked paint outside the preview: item={item:#?} preview={preview_rect:?}"
                );
            }
        }

        #[test]
        fn showcase_diagnostics_bounds_overlay_preview_clips_children() {
            let state = state_with_window("diagnostics");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("diagnostics layout");

            let preview = node_id(&document, "diagnostics.bounds.preview");
            let preview_rect = document.node(preview).layout().rect;
            assert!(
                preview_rect.height >= 140.0,
                "diagnostics bounds overlay preview collapsed: {preview_rect:?}"
            );
            let visual = node_id(&document, "diagnostics.bounds.visual");
            assert!(
                document.node(visual).children().len() >= 5,
                "bounds overlay should draw layout, clip, visible, paint, hit, and effect rectangles"
            );
            assert!(maybe_node_id(&document, "diagnostics.bounds.visual.paint").is_some());
            assert!(maybe_node_id(&document, "diagnostics.bounds.visual.hit").is_some());
            assert!(maybe_node_id(&document, "diagnostics.bounds.visual.effect-outset").is_some());
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
                    "diagnostics bounds overlay leaked paint outside the preview: item={item:#?} preview={preview_rect:?}"
                );
            }
        }

        #[test]
        fn showcase_diagnostics_point_overlay_preview_clips_children() {
            let state = state_with_window("diagnostics");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("diagnostics layout");

            let preview = node_id(&document, "diagnostics.point.preview");
            let preview_rect = document.node(preview).layout().rect;
            assert!(
                preview_rect.height >= 140.0,
                "diagnostics point overlay preview collapsed: {preview_rect:?}"
            );
            let visual = node_id(&document, "diagnostics.point.visual");
            assert!(
                document.node(visual).children().len() >= 4,
                "point overlay should draw the point marker, target bounds, and overlap region"
            );
            assert!(maybe_node_id(&document, "diagnostics.point.visual.point").is_some());
            assert!(maybe_node_id(&document, "diagnostics.point.visual.target.hit").is_some());
            assert!(maybe_node_id(&document, "diagnostics.point.visual.overlap").is_some());
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
                    "diagnostics point overlay leaked paint outside the preview: item={item:#?} preview={preview_rect:?}"
                );
            }
        }

        #[test]
        fn showcase_diagnostics_uses_clear_labeled_rows_instead_of_raw_debug_blobs() {
            let state = state_with_window("diagnostics");
            let viewport = UiSize::new(1000.0, 820.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                .expect("diagnostics layout");

            assert_eq!(
                text_content(&document, "diagnostics.inspector.rows.row.name.value"),
                "Preview action"
            );
            assert!(
                text_content(&document, "diagnostics.trace.rows.row.summary.value")
                    .contains("selected"),
                "frame trace should identify the selected diagnostics node"
            );
            assert!(
                text_content(&document, "diagnostics.trace.rows.row.timing.value")
                    .contains("backend-draw"),
                "frame trace should summarize slowest timing stage"
            );
            assert_eq!(
                text_content(&document, "diagnostics.issues.rows.row.status.value"),
                "warning"
            );
            assert!(
                text_content(&document, "diagnostics.issues.rows.row.issue.0.value")
                    .contains("frame-budget"),
                "issue triage should put slow-frame guidance in the ranked list"
            );
            assert!(
                text_content(&document, "diagnostics.findings.rows.row.summary.value")
                    .contains("finding inbox"),
                "finding inbox should summarize the unified actionable findings feed"
            );
            assert!(
                text_content(&document, "diagnostics.findings.rows.row.top_action.value")
                    .contains("."),
                "finding inbox should expose a stable first action"
            );
            assert!(
                text_content(&document, "diagnostics.findings.rows.row.finding.0.value")
                    .contains("panel "),
                "finding inbox rows should include panel targets"
            );
            assert!(
                text_content(&document, "diagnostics.health.rows.row.summary.value")
                    .contains("health score"),
                "health score should summarize the start-here debug scorecard"
            );
            assert!(
                text_content(&document, "diagnostics.health.rows.row.score.value")
                    .parse::<usize>()
                    .is_ok(),
                "health score panel should expose a numeric score"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.health.rows.row.area.performance.value"
                )
                .contains("frame_budget"),
                "health score should point performance issues at frame budget tooling"
            );
            assert!(
                text_content(&document, "diagnostics.root_causes.rows.row.summary.value")
                    .contains("root-cause clusters"),
                "root-cause clusters should summarize repeated node and subsystem evidence"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.root_causes.rows.row.top_action.value"
                )
                .contains("."),
                "root-cause clusters should expose a stable first action"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.root_causes.rows.row.cluster.0.value"
                )
                .contains("clues"),
                "root-cause cluster rows should show how many clues were merged"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.root_cause_timeline.rows.row.summary.value"
                )
                .contains("root-cause timeline"),
                "root-cause timeline should summarize retained-frame cluster persistence"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.root_cause_timeline.rows.row.persistent.value"
                )
                .parse::<usize>()
                .is_ok(),
                "root-cause timeline should expose persistent cluster counts"
            );
            assert!(
                text_content(&document, "diagnostics.questions.rows.row.summary.value")
                    .contains("question guide"),
                "question guide should summarize developer-facing why questions"
            );
            let performance_question = text_content(
                &document,
                "diagnostics.questions.rows.row.question.performance.value",
            );
            assert!(
                performance_question.contains("frame_bottleneck"),
                "question guide should point slow-frame questions to ranked bottlenecks first: {performance_question}"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.questions.rows.row.question.paint-hit.value"
                )
                .contains("paint_hit_mismatch_panel"),
                "question guide should expose paint-vs-hit mismatch investigation"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.questions.rows.row.question.text-fit.value"
                )
                .contains("text_fit_panel"),
                "question guide should expose text fit investigation"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.panel_recommendations.rows.row.summary.value"
                )
                .contains("panel recommendations"),
                "panel recommendations should summarize the ranked next panel"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.panel_recommendations.rows.row.top_panel.value"
                )
                .contains("_panel"),
                "panel recommendations should expose the first panel to open"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.overlay_recommendations.rows.row.summary.value"
                )
                .contains("overlay recommendations"),
                "overlay recommendations should summarize visual debug layers"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.overlay_recommendations.rows.row.primary.value"
                ) != "none",
                "overlay recommendations should expose the first visual layer to show"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.overlay_recommendations.rows.row.layer.hitboxes.value"
                )
                .contains("hitbox_debug_overlay"),
                "overlay recommendations should point hitbox findings at the hitbox overlay"
            );
            assert!(
                text_content(&document, "diagnostics.coverage.rows.row.summary.value")
                    .contains("diagnostics coverage"),
                "diagnostics coverage should summarize frame instrumentation readiness"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.coverage.rows.row.area.timing-sections.label"
                )
                .contains("Timing sections"),
                "diagnostics coverage should expose timing instrumentation evidence"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.coverage.rows.row.area.timing-sections.value"
                )
                .contains("evidence"),
                "diagnostics coverage should count timing instrumentation evidence"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.coverage.rows.row.area.event-routes.value"
                )
                .contains("captured"),
                "diagnostics coverage should show whether route samples were captured"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.investigation.rows.row.summary.value"
                )
                .contains("investigation plan"),
                "investigation plan should summarize the ordered debugging checklist"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.investigation.rows.row.step.0.capture-evidence.value"
                )
                .contains("action coverage."),
                "investigation plan should start with capture gaps when evidence is missing"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.investigation.rows.row.step.0.capture-evidence.value"
                )
                .contains("panel "),
                "investigation plan should route missing evidence to a deeper diagnostics panel"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.session_narrative.rows.row.summary.value"
                )
                .contains("session narrative"),
                "session narrative should summarize captured frame changes"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.session_narrative.rows.row.top_panel.value"
                )
                .contains("_panel"),
                "session narrative should expose the latest recommended panel"
            );
            assert!(
                text_content(&document, "diagnostics.contract.rows.row.summary.value")
                    .contains("debug contract"),
                "debug contract should summarize cross-check UI health"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.contract.rows.row.check.frame-budget.value"
                )
                .contains("fail"),
                "debug contract should expose failing frame-budget checks"
            );
            assert!(
                text_content(&document, "diagnostics.invariants.rows.row.summary.value")
                    .contains("debug invariants"),
                "debug invariants should summarize reusable UI invariant checks"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.invariants.rows.row.invariant.no-interactive-overlap.value"
                )
                .contains("fail"),
                "debug invariants should expose failing overlap checks"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.invariant_timeline.rows.row.summary.value"
                )
                .contains("invariant timeline"),
                "invariant timeline should summarize health-check trends"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.constraint_timeline.rows.row.summary.value"
                )
                .contains("constraint timeline"),
                "constraint timeline should summarize layout constraint trends"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.constraint_timeline.rows.row.issues.value"
                )
                .parse::<usize>()
                .is_ok(),
                "constraint timeline should expose a numeric issue total"
            );
            assert!(
                text_content(&document, "diagnostics.capture.rows.row.summary.value")
                    .contains("capture"),
                "capture report should summarize the shareable debug capture"
            );
            assert!(
                text_content(&document, "diagnostics.capture.rows.row.markdown.value")
                    .parse::<usize>()
                    .is_ok_and(|lines| lines > 8),
                "capture report should expose a markdown export size"
            );
            assert!(
                text_content(&document, "diagnostics.capture.rows.row.section.1.value")
                    .contains("frame timeline"),
                "capture report should include frame timeline context"
            );
            assert!(
                text_content(&document, "diagnostics.capture.rows.row.section.3.value")
                    .contains("question guide"),
                "capture report should include question-guide context"
            );
            assert!(
                text_content(&document, "diagnostics.capture.rows.row.section.4.value")
                    .contains("bottleneck"),
                "capture report should include bottleneck triage context"
            );
            assert!(
                text_content(&document, "diagnostics.node_history.rows.row.summary.value")
                    .contains("node frame history"),
                "node frame history should summarize selected-node changes over time"
            );
            assert!(
                text_content(&document, "diagnostics.node_history.rows.row.frame.1.value")
                    .contains("changed"),
                "node frame history should expose per-frame selected-node changes"
            );
            assert!(
                text_content(&document, "diagnostics.node_history.rows.row.frame.1.value")
                    .contains("rect"),
                "node frame history should include selected-node bounds in frame rows"
            );
            assert!(
                text_content(&document, "diagnostics.node_history.rows.row.issues.value")
                    .parse::<usize>()
                    .is_ok(),
                "node frame history should expose issue-frame totals"
            );
            assert!(
                text_content(&document, "diagnostics.node_change.rows.row.summary.value")
                    .contains("node change"),
                "node change panel should summarize why the selected node changed"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.node_change.rows.row.change.animation.value"
                )
                .contains("animation"),
                "node change panel should expose selected-node animation change causes"
            );
            assert!(
                text_content(&document, "diagnostics.hotspots.rows.row.summary.value")
                    .contains("node hotspots"),
                "node hotspot panel should summarize ranked debug pressure"
            );
            assert!(
                text_content(&document, "diagnostics.hotspots.rows.row.hotspot.0.value")
                    .contains("score"),
                "node hotspot panel should expose ranked node scores"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.node_recommendations.rows.row.summary.value"
                )
                .contains("node recommendations"),
                "node recommendations should summarize ranked inspect targets"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.node_recommendations.rows.row.top_node.value"
                )
                .contains("diagnostics.sample"),
                "node recommendations should expose the first node to inspect"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.node_recommendations.rows.row.node.0.value"
                )
                .contains("panel "),
                "node recommendations should point ranked nodes at an inspection panel"
            );
            assert!(
                text_content(&document, "diagnostics.slow_nodes.rows.row.summary.value")
                    .contains("slow nodes"),
                "slow-node panel should summarize combined performance triage"
            );
            assert!(
                text_content(&document, "diagnostics.slow_nodes.rows.row.slow.0.value")
                    .contains("sources"),
                "slow-node panel should identify contributing diagnostics"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.slow_node_timeline.rows.row.summary.value"
                )
                .contains("slow node timeline"),
                "slow-node timeline should summarize repeated node-level pressure"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.slow_node_timeline.rows.row.records.value"
                )
                .parse::<usize>()
                .is_ok_and(|count| count >= 1),
                "slow-node timeline should count retained slow-node records"
            );
            assert!(
                text_content(&document, "diagnostics.slow_frame.rows.row.summary.value")
                    .contains("slow frame"),
                "slow-frame panel should summarize frame-level performance triage"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.slow_frame.rows.row.source.slow-node.0.value"
                )
                .contains("sources"),
                "slow-frame panel should connect the frame budget to ranked slow nodes"
            );
            assert!(
                text_content(&document, "diagnostics.tree.rows.row.node.2.label")
                    .contains("diagnostics.sample.preview"),
                "layout tree should expose the retained diagnostics sample node order"
            );
            assert!(
                text_content(&document, "diagnostics.tree.rows.row.node.2.value").contains("input"),
                "layout tree should summarize interaction state for each node"
            );
            assert_eq!(
                text_content(&document, "diagnostics.node_search.rows.row.query.value"),
                "preview"
            );
            assert_eq!(
                text_content(&document, "diagnostics.node_search.rows.row.matches.value"),
                "1"
            );
            assert!(
                text_content(&document, "diagnostics.node_search.rows.row.match.0.value")
                    .contains("fields"),
                "node search should show why a node matched the query"
            );
            assert_eq!(
                text_content(&document, "diagnostics.focus.rows.row.focused.value"),
                "diagnostics.sample.preview"
            );
            assert_eq!(
                text_content(&document, "diagnostics.focus.rows.row.next.value"),
                "diagnostics.sample.hotspot"
            );
            assert!(
                text_content(&document, "diagnostics.focus.rows.row.candidate.0.label")
                    .contains("current"),
                "focus panel should mark the currently focused candidate"
            );
            assert_eq!(
                text_content(
                    &document,
                    "diagnostics.focus_timeline.rows.row.changed_frames.value"
                ),
                "2"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.focus_timeline.rows.row.frame.1.value"
                )
                .contains("focus changed"),
                "focus timeline should show when focus moved between targets"
            );
            assert_eq!(
                text_content(&document, "diagnostics.a11y.tree.rows.row.focusable.value"),
                "2"
            );
            assert!(
                text_content(&document, "diagnostics.a11y.tree.rows.row.warnings.value")
                    .parse::<usize>()
                    .is_ok(),
                "accessibility tree should expose the warning count"
            );
            assert!(
                text_content(&document, "diagnostics.a11y.tree.rows.row.node.1.value")
                    .contains("Button"),
                "accessibility tree should show the button role and label details"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.a11y.timeline.rows.row.summary.value"
                )
                .contains("accessibility timeline"),
                "accessibility timeline should summarize retained audit warnings"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.a11y.timeline.rows.row.frame.2.value"
                )
                .contains("resolved"),
                "accessibility timeline should show resolved warning frames"
            );
            assert_eq!(
                text_content(&document, "diagnostics.budget.rows.row.status.value"),
                "warning"
            );
            assert_eq!(
                text_content(&document, "diagnostics.budget.rows.row.over_by.value"),
                "2.50ms"
            );
            assert!(
                text_content(&document, "diagnostics.budget.rows.row.stage.0.value")
                    .contains("74% budget"),
                "frame budget should rank the slowest stage against the frame budget"
            );
            assert!(
                text_content(&document, "diagnostics.waterfall.rows.row.summary.value")
                    .contains("frame timing waterfall"),
                "timing waterfall should summarize ordered timing sections"
            );
            assert_eq!(
                text_content(&document, "diagnostics.waterfall.rows.row.crossed_at.value"),
                "gpu-render"
            );
            assert!(
                text_content(&document, "diagnostics.waterfall.rows.row.section.5.value")
                    .contains("crosses budget"),
                "timing waterfall should identify the section that crossed the frame budget"
            );
            assert!(
                text_content(&document, "diagnostics.bottlenecks.rows.row.summary.value")
                    .contains("frame bottlenecks"),
                "frame bottleneck panel should summarize ranked slow-frame causes"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.bottlenecks.rows.row.bottleneck.0.value"
                )
                .contains("inspect"),
                "frame bottleneck panel should point to the next deeper diagnostics panel"
            );
            assert!(
                text_content(&document, "diagnostics.autopsy.rows.row.summary.value")
                    .contains("frame autopsy"),
                "frame autopsy should summarize cross-system frame causes"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.autopsy.rows.row.source.timing.value"
                )
                .contains("over by"),
                "frame autopsy should include timing budget cause"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.autopsy.rows.row.source.cache-reuse.value"
                )
                .contains("cache reuse"),
                "frame autopsy should include cache reuse cause when supplied"
            );
            assert!(
                text_content(&document, "diagnostics.recorder.rows.row.summary.value")
                    .contains("frame recorder"),
                "frame recorder panel should summarize retained frame history"
            );
            assert_eq!(
                text_content(&document, "diagnostics.recorder.rows.row.retained.value"),
                "3/8"
            );
            assert!(
                text_content(&document, "diagnostics.recorder.rows.row.frame.0.value")
                    .contains("over"),
                "frame recorder should list the ranked retained frame context"
            );
            assert!(
                text_content(&document, "diagnostics.timeline.rows.row.summary.value")
                    .contains("frame timeline"),
                "frame timeline panel should summarize captured frame history"
            );
            assert_eq!(
                text_content(&document, "diagnostics.timeline.rows.row.slow.value"),
                "2"
            );
            assert!(
                text_content(&document, "diagnostics.timeline.rows.row.frame.0.value")
                    .contains("over"),
                "frame timeline should rank slow captured frames first"
            );
            assert!(
                text_content(&document, "diagnostics.timeline.rows.row.frame.0.value")
                    .contains("changes"),
                "frame timeline should correlate slow frames with previous-frame changes"
            );
            assert!(
                text_content(&document, "diagnostics.regression.rows.row.summary.value")
                    .contains("frame regression"),
                "frame regression panel should summarize before/after frame changes"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.regression.rows.row.regressions.value"
                )
                .parse::<usize>()
                .map(|count| count >= 1)
                .unwrap_or(false),
                "frame regression panel should count ranked regressions"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.regression.rows.row.source.timing.value"
                )
                .contains("frame_budget_panel"),
                "frame regression panel should point timing regressions at the budget panel"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.fix_verification.rows.row.summary.value"
                )
                .contains("fix verification"),
                "fix verification panel should summarize before/after fix verdicts"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.fix_verification.rows.row.top_action.value"
                )
                .contains("verify."),
                "fix verification panel should expose a stable first verification action"
            );
            let fix_timing = text_content(
                &document,
                "diagnostics.fix_verification.rows.row.source.timing.value",
            );
            assert!(
                fix_timing.contains("frame_budget") || fix_timing.contains("verify.timing"),
                "fix verification rows should include panel or action targets: {fix_timing}"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.issue_timeline.rows.row.summary.value"
                )
                .contains("issue timeline"),
                "issue timeline panel should summarize issue source trends across retained frames"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.issue_timeline.rows.row.source.performance.value"
                )
                .contains("frame_budget_panel"),
                "issue timeline should route performance issues to the frame budget panel"
            );
            assert!(
                text_content(&document, "diagnostics.issue_timeline.rows.row.new.value")
                    .parse::<usize>()
                    .map(|count| count >= 1)
                    .unwrap_or(false),
                "issue timeline should count newly appeared issue fingerprints"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.performance_timeline.rows.row.summary.value"
                )
                .contains("performance timeline"),
                "performance timeline panel should summarize retained stage trends"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.performance_timeline.rows.row.stage.0.value"
                )
                .contains("avg"),
                "performance timeline panel should expose per-stage averages and dominance"
            );
            assert!(
                text_content(&document, "diagnostics.why.rows.row.summary.value")
                    .contains("why trace"),
                "why trace panel should summarize the ranked frame diagnosis"
            );
            assert!(
                text_content(&document, "diagnostics.why.rows.row.overlay-plan.value")
                    .contains("overlay plan"),
                "why trace panel should expose the visual overlay plan"
            );
            assert!(
                text_content(&document, "diagnostics.why_timeline.rows.row.summary.value")
                    .contains("why timeline"),
                "why timeline panel should summarize frame-by-frame diagnosis"
            );
            assert!(
                text_content(&document, "diagnostics.why_timeline.rows.row.frame.0.value")
                    .contains("clue"),
                "why timeline panel should list the top diagnostic clue for each retained frame"
            );
            assert!(
                text_content(&document, "diagnostics.cache.rows.row.summary.value")
                    .contains("cache reuse"),
                "cache reuse panel should summarize cache hit/miss health"
            );
            assert!(
                text_content(&document, "diagnostics.cache.rows.row.display.0.value")
                    .contains("Miss"),
                "cache reuse panel should show display-list miss reasons"
            );
            assert!(
                text_content(&document, "diagnostics.movement.rows.row.summary.value")
                    .contains("layout movement"),
                "layout movement panel should summarize moved and resized nodes"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.movement.rows.row.node.diagnostics.sample.preview.value"
                )
                .contains("resized"),
                "layout movement panel should explain preview resizing"
            );
            assert!(
                text_content(&document, "diagnostics.layout_jank.rows.row.summary.value")
                    .contains("layout jank timeline"),
                "layout jank timeline should summarize recurring retained-frame movement"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layout_jank.rows.row.movements.value"
                )
                .parse::<usize>()
                .is_ok(),
                "layout jank timeline should expose a numeric movement count"
            );
            assert!(
                text_content(&document, "diagnostics.layout_cost.rows.row.summary.value")
                    .contains("layout cost"),
                "layout cost panel should summarize estimated subtree pressure"
            );
            assert!(
                text_content(&document, "diagnostics.layout_cost.rows.row.cost.0.value")
                    .contains("score"),
                "layout cost panel should expose ranked subtree cost scores"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layout_cost_timeline.rows.row.summary.value"
                )
                .contains("layout cost timeline"),
                "layout cost timeline should summarize retained subtree cost"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layout_cost_timeline.rows.row.records.value"
                )
                .parse::<usize>()
                .is_ok(),
                "layout cost timeline should expose a numeric cost record count"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layout_pressure.rows.row.summary.value"
                )
                .contains("layout pressure"),
                "layout pressure panel should summarize squeezed, clipped, and overlapping nodes"
            );
            let pressure_value = text_content(
                &document,
                "diagnostics.layout_pressure.rows.row.pressure.0.value",
            );
            assert!(
                pressure_value.contains("overflow")
                    || pressure_value.contains("Constraint")
                    || pressure_value.contains("Overlap"),
                "layout pressure panel should rank actionable node pressure: {pressure_value}"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layout_pressure_timeline.rows.row.summary.value"
                )
                .contains("layout pressure timeline"),
                "layout pressure timeline should summarize retained fitting pressure"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layout_pressure_timeline.rows.row.records.value"
                )
                .parse::<usize>()
                .is_ok(),
                "layout pressure timeline should expose a numeric pressure record count"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layout_cost_autopsy.rows.row.summary.value"
                )
                .contains("layout cost autopsy"),
                "layout cost autopsy should summarize a selected expensive subtree"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layout_cost_autopsy.rows.row.source.subtree.value"
                )
                .contains("nodes"),
                "layout cost autopsy should break down subtree-size pressure"
            );
            assert!(
                text_content(&document, "diagnostics.layout.rows.row.summary.value")
                    .contains("diagnostics.sample.preview"),
                "layout cause panel should identify the selected diagnostics node"
            );
            assert!(
                text_content(&document, "diagnostics.layout.rows.row.chain.2.value")
                    .contains("diagnostics.sample.preview"),
                "layout cause panel should explain the selected node's chain"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layout_autopsy.rows.row.summary.value"
                )
                .contains("layout autopsy"),
                "layout autopsy should summarize selected-node size and placement"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layout_autopsy.rows.row.source.size.value"
                )
                .contains("size"),
                "layout autopsy should explain selected-node resolved size"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.style_compare.rows.row.summary.value"
                )
                .contains("style compare"),
                "node style compare panel should summarize node-to-node differences"
            );
            assert!(
                text_content(&document, "diagnostics.style_compare.rows.row.diff.0.value")
                    .contains("->"),
                "node style compare panel should expose field-level deltas"
            );
            assert_eq!(
                text_content(&document, "diagnostics.clip.rows.row.clipped_by.value"),
                "diagnostics.clip.viewport"
            );
            assert!(
                text_content(&document, "diagnostics.clip.rows.row.chain.1.value")
                    .contains("scroll"),
                "clip scroll panel should identify the scroll clipping ancestor"
            );
            assert!(
                text_content(&document, "diagnostics.clip_chain.rows.row.summary.value")
                    .contains("clip chain"),
                "clip chain panel should summarize clipped nodes across the frame"
            );
            assert!(
                text_content(&document, "diagnostics.clip_chain.rows.row.clip.0.value")
                    .contains("clipped"),
                "clip chain panel should rank nodes reduced by ancestor clips"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.clip_chain_timeline.rows.row.summary.value"
                )
                .contains("clip chain timeline"),
                "clip chain timeline panel should summarize retained clipping"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.clip_chain_timeline.rows.row.clip.0.value"
                )
                .contains("frames"),
                "clip chain timeline panel should list retained clipped nodes"
            );
            assert!(
                text_content(&document, "diagnostics.scroll.rows.row.summary.value")
                    .contains("scroll range"),
                "scroll range panel should summarize scrollable and overflowing nodes"
            );
            assert!(
                text_content(&document, "diagnostics.scroll.rows.row.range.0.value")
                    .contains("range"),
                "scroll range panel should expose range and offset details"
            );
            assert!(
                text_content(&document, "diagnostics.wheel.rows.row.summary.value")
                    .contains("wheel route"),
                "wheel route panel should summarize wheel targeting"
            );
            assert!(
                text_content(&document, "diagnostics.wheel.rows.row.scope.value")
                    .contains("front_panel"),
                "wheel route panel should name the top wheel scope"
            );
            assert!(
                text_content(&document, "diagnostics.wheel.rows.row.candidate.0.value")
                    .contains("scope"),
                "wheel route panel should list blocking wheel candidates"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.scroll_timeline.rows.row.summary.value"
                )
                .contains("scroll timeline"),
                "scroll timeline panel should summarize retained scroll range history"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.scroll_timeline.rows.row.frame.1.value"
                )
                .contains("offset/range"),
                "scroll timeline panel should expose offset or range changes"
            );
            assert!(
                text_content(&document, "diagnostics.responsive.rows.row.summary.value")
                    .contains("responsive layout"),
                "responsive panel should summarize viewport-by-viewport layout health"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.responsive.rows.row.viewport.0.label"
                )
                .contains("x"),
                "responsive panel should label each sampled viewport"
            );
            let responsive_value = text_content(
                &document,
                "diagnostics.responsive.rows.row.viewport.0.value",
            );
            assert!(
                responsive_value.contains("constraints")
                    || responsive_value.contains("hitbox")
                    || responsive_value.contains("hit targets"),
                "responsive panel should rank the first breakpoint issue"
            );
            assert!(
                text_content(&document, "diagnostics.overlaps.rows.row.summary.value")
                    .contains("overlap report"),
                "overlap report panel should summarize global overlap health"
            );
            assert!(
                text_content(&document, "diagnostics.hit_targets.rows.row.summary.value")
                    .contains("hit targets"),
                "hit target panel should summarize interactive hit health"
            );
            assert!(
                text_content(&document, "diagnostics.hit_targets.rows.row.target.0.value")
                    .contains("small hit target"),
                "hit target panel should rank tiny interactive regions"
            );
            assert!(
                text_content(&document, "diagnostics.hitbox_map.rows.row.summary.value")
                    .contains("hitbox map"),
                "hitbox map panel should summarize the whole-frame hitbox inventory"
            );
            assert!(
                text_content(&document, "diagnostics.hitbox_map.rows.row.hitbox.0.value")
                    .contains("clipped"),
                "hitbox map panel should rank clipped interactive regions first"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.hitbox_timeline.rows.row.summary.value"
                )
                .contains("hitbox timeline"),
                "hitbox timeline panel should summarize retained hitbox history"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.hitbox_timeline.rows.row.frame.0.value"
                )
                .contains("interactive"),
                "hitbox timeline panel should list frame-level hitbox changes"
            );
            assert!(
                text_content(&document, "diagnostics.visibility.rows.row.summary.value")
                    .contains("visibility"),
                "visibility panel should summarize why nodes cannot be seen"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.visibility.rows.row.visibility.0.value"
                )
                .contains("Transparent"),
                "visibility panel should rank transparent or hidden interactive nodes first"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.visibility_timeline.rows.row.summary.value"
                )
                .contains("visibility timeline"),
                "visibility timeline panel should summarize retained visibility history"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.visibility_timeline.rows.row.issue.0.value"
                )
                .contains("frames"),
                "visibility timeline panel should show recurring visibility issues"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.hitbox_occlusion.rows.row.summary.value"
                )
                .contains("hitbox occlusion"),
                "hitbox occlusion panel should summarize covered interactive regions"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.hitbox_occlusion.rows.row.occlusion.0.value"
                )
                .contains("covered"),
                "hitbox occlusion panel should rank covered hit regions"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.hitbox_occlusion_timeline.rows.row.summary.value"
                )
                .contains("hitbox occlusion timeline"),
                "hitbox occlusion timeline panel should summarize retained coverage"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.hitbox_occlusion_timeline.rows.row.occlusion.0.value"
                )
                .contains("frames"),
                "hitbox occlusion timeline panel should show recurring covered targets"
            );
            assert!(
                text_content(&document, "diagnostics.paint_hit.rows.row.summary.value")
                    .contains("paint/hit mismatch"),
                "paint/hit mismatch panel should summarize visible versus clickable geometry"
            );
            assert!(
                text_content(&document, "diagnostics.paint_hit.rows.row.mismatch.0.value")
                    .contains("Hit without visible paint")
                    || text_content(&document, "diagnostics.paint_hit.rows.row.mismatch.0.value")
                        .contains("Paint outside hit"),
                "paint/hit mismatch panel should rank clickable blanks or visual edges"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.paint_hit_timeline.rows.row.summary.value"
                )
                .contains("paint/hit"),
                "paint/hit mismatch timeline should summarize retained geometry gaps"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.paint_hit_timeline.rows.row.new.value"
                )
                .parse::<usize>()
                .is_ok_and(|count| count >= 1),
                "paint/hit mismatch timeline should count newly introduced mismatches"
            );
            assert!(
                text_content(&document, "diagnostics.overlaps.rows.row.overlap.0.value")
                    .contains("hitbox"),
                "overlap report panel should rank hitbox conflicts first"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.overlap_timeline.rows.row.summary.value"
                )
                .contains("overlap timeline"),
                "overlap timeline panel should summarize overlap changes across retained frames"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.overlap_timeline.rows.row.frame.0.value"
                )
                .contains("overlap"),
                "overlap timeline panel should list frame-level overlap history"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.overlap_autopsy.rows.row.summary.value"
                )
                .contains("overlap autopsy"),
                "overlap autopsy should summarize the selected collision"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.overlap_autopsy.rows.row.source.interactivity.value"
                )
                .contains("pointer"),
                "overlap autopsy should expose input-specific overlap causes"
            );
            assert_eq!(
                text_content(&document, "diagnostics.probe.rows.row.target.value"),
                "diagnostics.sample.hotspot"
            );
            assert!(
                text_content(&document, "diagnostics.probe.rows.row.candidate.1.value")
                    .contains("behind"),
                "pointer probe should explain why the preview lost the click"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.pointer_session.rows.row.summary.value"
                )
                .contains("pointer session"),
                "pointer session should summarize retained route history"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.pointer_session.rows.row.latest_target.value"
                )
                .contains("diagnostics.sample"),
                "pointer session should expose the latest routed target"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.pointer_autopsy.rows.row.summary.value"
                )
                .contains("pointer autopsy"),
                "pointer autopsy should summarize the click route diagnosis"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.pointer_autopsy.rows.row.source.occlusion.value"
                )
                .contains("behind"),
                "pointer autopsy should rank occluded hit regions"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.point_autopsy.rows.row.summary.value"
                )
                .contains("point autopsy"),
                "point autopsy should summarize the click-to-explain diagnosis"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.point_autopsy.rows.row.source.next-step.value"
                )
                .contains("warning")
                    || text_content(
                        &document,
                        "diagnostics.point_autopsy.rows.row.source.next-step.value"
                    )
                    .contains("behind"),
                "point autopsy should surface the first actionable click-debugging step"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.point_autopsy.rows.row.source.resolution.value"
                )
                .contains("point resolution"),
                "point autopsy should summarize the raw hit, target, paint, and action decision"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.point_autopsy.rows.row.source.visibility.value"
                )
                .contains("diagnostics.sample.hotspot"),
                "point autopsy should include target visibility in the click diagnosis"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.point_autopsy.rows.row.source.paint.value"
                )
                .contains("paint stack"),
                "point autopsy should include paint provenance in the click diagnosis"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.inspect_point.rows.row.summary.value"
                )
                .contains("inspect point"),
                "inspect point should use inspect-mode naming for the combined click diagnosis"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.inspect_point.rows.row.source.resolution.value"
                )
                .contains("point resolution"),
                "inspect point should include the point-resolution decision row"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.inspect_point.rows.row.source.paint.value"
                )
                .contains("paint stack"),
                "inspect point should include paint provenance in the click diagnosis"
            );
            assert!(
                text_content(&document, "diagnostics.interaction.rows.row.summary.value")
                    .contains("interaction state"),
                "interaction state panel should summarize active host roles"
            );
            assert_eq!(
                text_content(&document, "diagnostics.interaction.rows.row.hovered.value"),
                "diagnostics.sample.hotspot"
            );
            assert!(
                text_content(&document, "diagnostics.interaction.rows.row.shortcut.value")
                    .contains("Ctrl"),
                "interaction state panel should expose the routed shortcut"
            );
            assert_eq!(
                text_content(
                    &document,
                    "diagnostics.interaction.rows.row.shortcut_target.value"
                ),
                "diagnostics.sample.preview"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.shortcut_route.rows.row.summary.value"
                )
                .contains("shortcut route"),
                "shortcut route panel should summarize scoped keyboard routing"
            );
            assert_eq!(
                text_content(
                    &document,
                    "diagnostics.shortcut_route.rows.row.command.value"
                ),
                "diagnostics.palette"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.interaction.rows.row.interaction.0.value"
                )
                .contains("hovered"),
                "interaction state panel should list active node roles"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.interaction_timeline.rows.row.summary.value"
                )
                .contains("interaction state timeline"),
                "interaction timeline should summarize retained host roles"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.interaction_timeline.rows.row.drag_capture.value"
                )
                .parse::<usize>()
                .is_ok_and(|count| count >= 1),
                "interaction timeline should count retained drag capture"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.interaction_timeline.rows.row.frame.1.value"
                )
                .contains("new"),
                "interaction timeline should show new active interaction targets"
            );
            assert!(
                text_content(&document, "diagnostics.actions.rows.row.summary.value")
                    .contains("action map"),
                "action map panel should summarize action and command bindings"
            );
            assert!(
                text_content(&document, "diagnostics.actions.rows.row.bindings.value")
                    .parse::<usize>()
                    .is_ok_and(|count| count >= 2),
                "action map panel should show node action bindings"
            );
            assert!(
                text_content(&document, "diagnostics.dispatch.rows.row.summary.value")
                    .contains("action dispatch"),
                "action dispatch panel should summarize queued widget actions"
            );
            assert_eq!(
                text_content(&document, "diagnostics.dispatch.rows.row.queued.value"),
                "2"
            );
            assert_eq!(
                text_content(&document, "diagnostics.dispatch.rows.row.warnings.value"),
                "0"
            );
            let dispatch_records = [
                text_content(&document, "diagnostics.dispatch.rows.row.record.0.value"),
                text_content(&document, "diagnostics.dispatch.rows.row.record.1.value"),
            ];
            assert!(
                dispatch_records
                    .iter()
                    .any(|record| record.contains("diagnostics.preview")),
                "action dispatch panel should list queued action ids: {dispatch_records:?}"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.action_timeline.rows.row.summary.value"
                )
                .contains("action map timeline"),
                "action map timeline should summarize retained action binding diagnostics"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.action_timeline.rows.row.bindings.value"
                )
                .parse::<usize>()
                .is_ok_and(|count| count >= 1),
                "action map timeline should rebuild bindings from retained frame snapshots"
            );
            assert!(
                text_content(&document, "diagnostics.drag.rows.row.summary.value")
                    .contains("drag affordances"),
                "drag affordance panel should summarize resize and drag handles"
            );
            assert!(
                text_content(&document, "diagnostics.drag.rows.row.edge_hugging.value")
                    .parse::<usize>()
                    .is_ok_and(|count| count >= 1),
                "drag affordance panel should count edge-hugging drag bars"
            );
            assert!(
                text_content(&document, "diagnostics.drag.rows.row.drag.0.value")
                    .contains("margin"),
                "drag affordance panel should expose non-principal margin evidence"
            );
            assert!(
                text_content(&document, "diagnostics.affordances.rows.row.summary.value")
                    .contains("interaction affordances"),
                "interaction affordance panel should summarize clickability, action, visual, and a11y contracts"
            );
            assert!(
                text_content(&document, "diagnostics.affordances.rows.row.visual.value")
                    .parse::<usize>()
                    .is_ok_and(|count| count >= 1),
                "interaction affordance panel should count visible/clickable mismatches"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.affordance_timeline.rows.row.summary.value"
                )
                .contains("interaction affordance timeline"),
                "interaction affordance timeline should summarize retained clickability diagnostics"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.affordance_timeline.rows.row.checked.value"
                )
                .parse::<usize>()
                .is_ok_and(|count| count >= 1),
                "interaction affordance timeline should count retained checked controls"
            );
            assert_eq!(
                text_content(&document, "diagnostics.paint.rows.row.top.value"),
                "diagnostics.sample.hotspot"
            );
            assert!(
                text_content(&document, "diagnostics.paint.rows.row.stack.0.value")
                    .contains("visible"),
                "paint order panel should show the top-down visible stack"
            );
            assert!(
                text_content(&document, "diagnostics.stacking.rows.row.summary.value")
                    .contains("stacking order"),
                "stacking order panel should summarize whole-frame stack ordering"
            );
            assert!(
                text_content(&document, "diagnostics.stacking.rows.row.stack.0.value")
                    .contains("front")
                    || text_content(&document, "diagnostics.stacking.rows.row.stack.0.value")
                        .contains("z"),
                "stacking order panel should list front-to-back stack rows"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.stacking_timeline.rows.row.summary.value"
                )
                .contains("stacking order timeline"),
                "stacking order timeline should explain retained stack drift"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.stacking_timeline.rows.row.covered.value"
                )
                .parse::<usize>()
                .is_ok_and(|count| count >= 1),
                "stacking order timeline should count covered interactive targets"
            );
            assert!(
                text_content(&document, "diagnostics.layers.rows.row.summary.value")
                    .contains("render layers"),
                "render layers panel should summarize the full paint stack"
            );
            assert!(
                text_content(&document, "diagnostics.layers.rows.row.layer.0.value")
                    .contains("items"),
                "render layers panel should list layer paint counts"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layer_timeline.rows.row.summary.value"
                )
                .contains("render layer timeline"),
                "render layer timeline panel should summarize retained layer cost"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.layer_timeline.rows.row.layer.0.value"
                )
                .contains("frames"),
                "render layer timeline panel should list retained layer rows"
            );
            assert!(
                text_content(&document, "diagnostics.overdraw.rows.row.summary.value")
                    .contains("paint overdraw"),
                "paint overdraw panel should summarize covered paint work"
            );
            assert!(
                text_content(&document, "diagnostics.overdraw.rows.row.item.0.value")
                    .contains("covered"),
                "paint overdraw panel should list covered paint items"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.overdraw_timeline.rows.row.summary.value"
                )
                .contains("paint overdraw timeline"),
                "paint overdraw timeline panel should summarize retained covered paint"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.overdraw_timeline.rows.row.item.0.value"
                )
                .contains("frames"),
                "paint overdraw timeline panel should list retained overdraw items"
            );
            assert!(
                text_content(&document, "diagnostics.batches.rows.row.summary.value")
                    .contains("paint batches"),
                "paint batches panel should summarize renderer batch grouping"
            );
            assert!(
                text_content(&document, "diagnostics.batches.rows.row.break.0.value")
                    .contains("kind")
                    || text_content(&document, "diagnostics.batches.rows.row.break.0.value")
                        .contains("shader"),
                "paint batches panel should explain why adjacent paint splits"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.batch_timeline.rows.row.summary.value"
                )
                .contains("paint batch timeline"),
                "paint batch timeline panel should summarize retained batch fragmentation"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.batch_timeline.rows.row.reason.shader.value"
                )
                .contains("splits"),
                "paint batch timeline panel should list split reasons"
            );
            assert!(
                text_content(&document, "diagnostics.effects.rows.row.summary.value")
                    .contains("visual effect"),
                "visual effects panel should summarize effect-bound diagnostics"
            );
            assert!(
                text_content(&document, "diagnostics.effects.rows.row.effect.0.value")
                    .contains("paint"),
                "visual effects panel should explain paint/hit/layout bounds"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.effect_timeline.rows.row.summary.value"
                )
                .contains("visual-effect timeline"),
                "visual effect timeline should summarize retained effect-bound diagnostics"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.effect_timeline.rows.row.effect.0.value"
                )
                .contains("shader"),
                "visual effect timeline should list retained shader/material effect rows"
            );
            assert!(
                text_content(&document, "diagnostics.bounds.rows.row.summary.value")
                    .contains("bounds autopsy"),
                "bounds autopsy should summarize selected-node geometry systems"
            );
            assert!(
                text_content(&document, "diagnostics.bounds.rows.row.source.paint.value")
                    .contains("paint"),
                "bounds autopsy should expose selected-node paint bounds"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.bounds.rows.row.source.effect-outset.value"
                )
                .contains("declared"),
                "bounds autopsy should explain visual effect outsets"
            );
            assert!(
                text_content(&document, "diagnostics.text_fit.rows.row.summary.value")
                    .contains("text fit"),
                "text fit panel should summarize document-wide text fit health"
            );
            assert!(
                text_content(&document, "diagnostics.text_fit.rows.row.text.0.value")
                    .contains("overflow"),
                "text fit panel should rank overflowing labels"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_fit_timeline.rows.row.summary.value"
                )
                .contains("text fit timeline"),
                "text fit timeline should summarize retained text pressure"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_fit_timeline.rows.row.frame.2.value"
                )
                .contains("resolved"),
                "text fit timeline should show resolved overflow frames"
            );
            assert_eq!(
                text_content(&document, "diagnostics.text.rows.row.node.value"),
                "diagnostics.sample.label"
            );
            assert!(
                text_content(&document, "diagnostics.text.rows.row.measured.value").contains("x"),
                "text layout panel should show measured text size"
            );
            assert!(
                text_content(&document, "diagnostics.text_style.rows.row.summary.value")
                    .contains("text style"),
                "text style panel should summarize selected text style"
            );
            assert!(
                text_content(&document, "diagnostics.text_style.rows.row.family.value")
                    .contains("SansSerif"),
                "text style panel should expose the selected font family"
            );
            assert!(
                text_content(&document, "diagnostics.text_style.rows.row.color.value")
                    .starts_with('#'),
                "text style panel should expose the selected text color"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_input_state.rows.row.summary.value"
                )
                .contains("text input state"),
                "text input state panel should summarize caret and edit state"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_input_state.rows.row.selection.value"
                )
                .contains("0..5"),
                "text input state panel should expose active selection"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_input_state.rows.row.history.value"
                )
                .contains("undo 1"),
                "text input state panel should expose edit history depth"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_input_event.rows.row.summary.value"
                )
                .contains("text input event"),
                "text input event panel should summarize event routing and edit outcome"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_input_event.rows.row.edit.value"
                )
                .contains("changed"),
                "text input event panel should expose changed edit outcomes"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_input_event.rows.row.requests.value"
                )
                .parse::<usize>()
                .unwrap_or_default()
                    > 0,
                "text input event panel should expose emitted platform requests"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_localization.rows.row.summary.value"
                )
                .contains("text localization"),
                "text localization panel should summarize localization readiness"
            );
            assert_eq!(
                text_content(
                    &document,
                    "diagnostics.text_localization.rows.row.missing_intrinsic.value"
                ),
                "1"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_localization.rows.row.text.0.value"
                )
                .contains("toolbar.save"),
                "text localization panel should list dynamic-label metadata"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.text_contrast.rows.row.summary.value"
                )
                .contains("text contrast"),
                "text contrast panel should summarize readability"
            );
            assert_eq!(
                text_content(
                    &document,
                    "diagnostics.text_contrast.rows.row.low_contrast.value"
                ),
                "1"
            );
            assert!(
                text_content(&document, "diagnostics.text_contrast.rows.row.text.0.value")
                    .contains("/4.5"),
                "text contrast panel should rank low-contrast text"
            );
            assert!(
                text_content(&document, "diagnostics.explain.rows.row.summary.value")
                    .contains("diagnostics.sample.preview"),
                "node explanation should identify the selected diagnostics node"
            );
            assert!(
                text_content(&document, "diagnostics.explain.rows.row.resource.0.value")
                    .contains("missing"),
                "node explanation should include selected-node resource state"
            );
            assert!(
                text_content(&document, "diagnostics.inspect_node.rows.row.summary.value")
                    .contains("inspect node"),
                "inspect node should use inspect-mode naming for selected-node diagnosis"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.inspect_node.rows.row.source.next-step.value"
                )
                .contains("warning"),
                "inspect node should rank the first selected-node action"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.inspect_node.rows.row.source.hitbox.value"
                )
                .contains("hit"),
                "inspect node should include selected-node hitbox evidence"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.inspect_node.rows.row.source.overlap.value"
                )
                .contains("overlap"),
                "inspect node should include selected-node overlap evidence"
            );
            assert!(
                text_content(&document, "diagnostics.provenance.rows.row.summary.value")
                    .contains("provenance"),
                "node provenance should summarize the selected node's contributing systems"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.provenance.rows.row.source.resources.value"
                )
                .contains("missing"),
                "node provenance should include resource responsibility"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.provenance.rows.row.source.overlaps.value"
                )
                .contains("interactive"),
                "node provenance should include overlap responsibility"
            );
            assert_eq!(
                text_content(
                    &document,
                    "diagnostics.commands.row.diagnostics_palette.label"
                ),
                "Open command palette"
            );
            assert_eq!(
                text_content(
                    &document,
                    "diagnostics.commands.row.diagnostics_palette.shortcut"
                ),
                "Ctrl+K"
            );
            assert_eq!(
                text_content(&document, "diagnostics.theme.token.colors_accent.label"),
                "colors.accent"
            );
            assert_eq!(
                text_content(&document, "diagnostics.commands.conflicts.value"),
                "None"
            );
            assert!(
                text_content(&document, "diagnostics.timing.rows.row.slowest.value")
                    .contains("backend-draw"),
                "timing panel should identify the slowest pipeline stage"
            );
            assert_eq!(
                text_content(&document, "diagnostics.route.rows.row.target.value"),
                "diagnostics.sample.hotspot"
            );
            assert!(
                text_content(&document, "diagnostics.route.rows.row.candidate.2.value")
                    .contains("behind"),
                "route panel should explain that the preview candidate was behind the hotspot"
            );
            assert!(
                text_content(&document, "diagnostics.diff.rows.row.changed.value")
                    .parse::<usize>()
                    .unwrap()
                    >= 1,
                "frame diff should report at least the animated preview node"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.diff.rows.row.node.diagnostics.sample.preview.value"
                )
                .contains("animation"),
                "frame diff should explain animation changes on the preview node"
            );
            assert!(
                text_content(&document, "diagnostics.constraints.rows.row.total.value")
                    .parse::<usize>()
                    .unwrap()
                    >= 1,
                "constraint panel should report the intentional diagnostics overlap"
            );
            assert!(
                text_content(&document, "diagnostics.constraints.rows.row.issue.0.value")
                    .contains("accessibility")
                    || text_content(&document, "diagnostics.constraints.rows.row.issue.0.value")
                        .contains("overlap"),
                "constraint panel should surface actionable conflict guidance"
            );
            assert_eq!(
                text_content(&document, "diagnostics.resources.rows.row.missing.value"),
                "1"
            );
            assert!(
                text_content(&document, "diagnostics.resources.rows.row.resource.0.value")
                    .contains("missing"),
                "resource panel should surface missing image or texture resources"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.resource_timeline.rows.row.summary.value"
                )
                .contains("resource timeline"),
                "resource timeline panel should summarize retained resource state"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.resource_timeline.rows.row.resource.0.value"
                )
                .contains("frames"),
                "resource timeline panel should list retained resource rows"
            );
            assert_eq!(
                text_content(&document, "diagnostics.dirty.rows.row.order.value"),
                "Text measure, Layout, Input, Paint"
            );
            assert_eq!(
                text_content(&document, "diagnostics.dirty.rows.row.invalidation.0.label"),
                "Window resize"
            );
            assert!(
                text_content(&document, "diagnostics.widget_state.rows.row.summary.value")
                    .contains("widget state retention"),
                "widget state retention panel should summarize retained widget state"
            );
            assert_eq!(
                text_content(
                    &document,
                    "diagnostics.widget_state.rows.row.orphaned.value"
                ),
                "1"
            );
            assert!(
                text_content(&document, "diagnostics.widget_state.rows.row.state.0.value")
                    .contains("orphaned"),
                "widget state retention panel should rank orphaned state"
            );
            assert!(
                text_content(&document, "diagnostics.invalidation.rows.row.summary.value")
                    .contains("invalidations"),
                "invalidation blame panel should summarize dirty triggers"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.invalidation.rows.row.trigger.0.value"
                )
                .contains("Text measure"),
                "invalidation blame panel should rank broad resize invalidation first"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.invalidation_blast.rows.row.summary.value"
                )
                .contains("invalidation blast"),
                "invalidation blast panel should summarize dirty subsystem fan-out"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.invalidation_blast.rows.row.subsystem.0.value"
                )
                .contains("triggers"),
                "invalidation blast panel should expose subsystem trigger counts"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.invalidation_timeline.rows.row.summary.value"
                )
                .contains("invalidation timeline"),
                "invalidation timeline panel should summarize retained dirty frames"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.invalidation_timeline.rows.row.reason.0.value"
                )
                .contains("frames"),
                "invalidation timeline panel should expose recurring dirty reasons"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.animation.activity.rows.row.summary.value"
                )
                .contains("animation activity"),
                "animation activity panel should summarize global animation state"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.animation.activity.rows.row.animation.0.value"
                )
                .contains("score"),
                "animation activity panel should expose ranked animation records"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.animation.timeline.rows.row.summary.value"
                )
                .contains("animation activity timeline"),
                "animation activity timeline should summarize retained animation work"
            );
            assert!(
                text_content(
                    &document,
                    "diagnostics.animation.timeline.rows.row.animation.0.value"
                )
                .contains("active"),
                "animation activity timeline should list retained active animations"
            );
            assert_no_severe_showcase_warnings("diagnostics", "clear rows", &document);
        }

        #[test]
        fn showcase_diagnostics_panels_do_not_overlap_section_scrollbar() {
            let state = state_with_window("diagnostics");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("diagnostics layout");

            let scroll = node_id(&document, "diagnostics.section_scroll");
            let scroll_rect = document.node(scroll).layout().rect;
            document.handle_input(UiInputEvent::PointerMove(UiPoint::new(
                scroll_rect.x + 8.0,
                scroll_rect.y + 8.0,
            )));
            let paint = document.paint_list();
            let track = paint
                .items
                .iter()
                .filter(|item| {
                    item.node == scroll
                        && matches!(item.kind, PaintKind::Rect { .. })
                        && item.rect.width <= 8.0
                })
                .max_by(|left, right| left.rect.height.total_cmp(&right.rect.height))
                .expect("diagnostics vertical scrollbar track");

            for panel_name in [
                "diagnostics.inspector",
                "diagnostics.animation.activity.panel",
                "diagnostics.animation.timeline.panel",
                "diagnostics.animation.graph",
                "diagnostics.animation.controls",
                "diagnostics.a11y.preview",
                "diagnostics.a11y",
                "diagnostics.trace.panel",
                "diagnostics.issues.panel",
                "diagnostics.findings.panel",
                "diagnostics.health.panel",
                "diagnostics.root_causes.panel",
                "diagnostics.root_cause_timeline.panel",
                "diagnostics.issue_timeline.panel",
                "diagnostics.questions.panel",
                "diagnostics.panel_recommendations.panel",
                "diagnostics.overlay_recommendations.panel",
                "diagnostics.coverage.panel",
                "diagnostics.investigation.panel",
                "diagnostics.session_narrative.panel",
                "diagnostics.contract.panel",
                "diagnostics.invariants.panel",
                "diagnostics.invariant_timeline.panel",
                "diagnostics.constraint_timeline.panel",
                "diagnostics.capture.panel",
                "diagnostics.hotspots.panel",
                "diagnostics.node_recommendations.panel",
                "diagnostics.slow_nodes.panel",
                "diagnostics.slow_node_timeline.panel",
                "diagnostics.slow_frame.panel",
                "diagnostics.budget.panel",
                "diagnostics.waterfall.panel",
                "diagnostics.bottlenecks.panel",
                "diagnostics.autopsy.panel",
                "diagnostics.recorder.panel",
                "diagnostics.timeline.panel",
                "diagnostics.regression.panel",
                "diagnostics.fix_verification.panel",
                "diagnostics.performance_timeline.panel",
                "diagnostics.why.panel",
                "diagnostics.why_timeline.panel",
                "diagnostics.node_history.panel",
                "diagnostics.node_change.panel",
                "diagnostics.cache.panel",
                "diagnostics.movement.panel",
                "diagnostics.layout_jank.panel",
                "diagnostics.layout_cost.panel",
                "diagnostics.layout_cost_timeline.panel",
                "diagnostics.layout_pressure.panel",
                "diagnostics.layout_pressure_timeline.panel",
                "diagnostics.layout_cost_autopsy.panel",
                "diagnostics.tree.panel",
                "diagnostics.node_search.panel",
                "diagnostics.focus.panel",
                "diagnostics.focus_timeline.panel",
                "diagnostics.layout.panel",
                "diagnostics.layout_autopsy.panel",
                "diagnostics.style_compare.panel",
                "diagnostics.clip.panel",
                "diagnostics.clip_chain.panel",
                "diagnostics.clip_chain_timeline.panel",
                "diagnostics.scroll.panel",
                "diagnostics.wheel.panel",
                "diagnostics.scroll_timeline.panel",
                "diagnostics.responsive.panel",
                "diagnostics.visibility.panel",
                "diagnostics.visibility_timeline.panel",
                "diagnostics.hitbox_map.panel",
                "diagnostics.hitbox_timeline.panel",
                "diagnostics.hit_targets.panel",
                "diagnostics.hitbox_occlusion.panel",
                "diagnostics.hitbox_occlusion_timeline.panel",
                "diagnostics.paint_hit.panel",
                "diagnostics.paint_hit_timeline.panel",
                "diagnostics.overlaps.panel",
                "diagnostics.overlap_timeline.panel",
                "diagnostics.overlap_autopsy.panel",
                "diagnostics.probe.panel",
                "diagnostics.pointer_session.panel",
                "diagnostics.pointer_autopsy.panel",
                "diagnostics.point_autopsy.panel",
                "diagnostics.inspect_point.panel",
                "diagnostics.interaction.panel",
                "diagnostics.interaction_timeline.panel",
                "diagnostics.shortcut_route.panel",
                "diagnostics.actions.panel",
                "diagnostics.dispatch.panel",
                "diagnostics.action_timeline.panel",
                "diagnostics.drag.panel",
                "diagnostics.affordances.panel",
                "diagnostics.affordance_timeline.panel",
                "diagnostics.paint.panel",
                "diagnostics.stacking.panel",
                "diagnostics.stacking_timeline.panel",
                "diagnostics.layers.panel",
                "diagnostics.layer_timeline.panel",
                "diagnostics.overdraw.panel",
                "diagnostics.overdraw_timeline.panel",
                "diagnostics.batches.panel",
                "diagnostics.batch_timeline.panel",
                "diagnostics.effects.panel",
                "diagnostics.effect_timeline.panel",
                "diagnostics.bounds.panel",
                "diagnostics.text_fit.panel",
                "diagnostics.text_fit_timeline.panel",
                "diagnostics.text.panel",
                "diagnostics.text_style.panel",
                "diagnostics.text_localization.panel",
                "diagnostics.text_contrast.panel",
                "diagnostics.explain.panel",
                "diagnostics.inspect_node.panel",
                "diagnostics.provenance.panel",
                "diagnostics.resources.panel",
                "diagnostics.resource_timeline.panel",
                "diagnostics.invalidation.panel",
                "diagnostics.invalidation_blast.panel",
                "diagnostics.invalidation_timeline.panel",
                "diagnostics.commands",
                "diagnostics.theme",
                "diagnostics.a11y.tree.panel",
                "diagnostics.a11y.timeline.panel",
            ] {
                let rect = document.node(node_id(&document, panel_name)).layout().rect;
                assert!(
                    rect.right() <= track.rect.x + 0.5,
                    "{panel_name} should end before the diagnostics scrollbar gutter: panel={rect:?} track={:?}",
                    track.rect
                );
            }
        }

        #[test]
        fn showcase_trees_include_editable_tree_table_and_focus_preservation_example() {
            let mut state = state_with_window("trees");
            let viewport = UiSize::new(1100.0, 820.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("trees layout");

            assert_eq!(
                text_content(&document, "trees.tree_view.title"),
                "Editable tree"
            );
            assert!(
                maybe_node_id(&document, "trees.tree_view.row.child-0.action.delete").is_some()
            );
            assert!(maybe_node_id(&document, "trees.tree_view.row.child-0.action.add").is_some());
            assert!(
                maybe_node_id(&document, "trees.tree_view.row.child-0-3.indent.1.guide").is_some()
            );
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "trees.tree.action.child-0.add",
            ));
            let mut edited_document = state.view(viewport);
            edited_document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("edited trees layout");
            assert!(
                maybe_node_id(&edited_document, "trees.tree_view.row.editable-100").is_some(),
                "add action should insert a visible child under the expanded item"
            );
            assert_eq!(
                text_content(&edited_document, "trees.editable.status"),
                "Added child #4 under child #0"
            );
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "trees.tree.action.child-1-2.delete",
            ));
            let mut deleted_document = state.view(viewport);
            deleted_document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("deleted trees layout");
            assert!(
                maybe_node_id(&deleted_document, "trees.tree_view.row.child-1-2").is_none(),
                "delete action should remove the selected branch from the editable tree"
            );

            assert!(maybe_node_id(&document, "trees.virtual.row.root").is_some());
            assert!(maybe_node_id(&document, "trees.virtual.row.src").is_some());
            assert!(maybe_node_id(&document, "trees.virtual.row.src-file-00").is_some());
            assert_eq!(
                text_content(&document, "trees.virtual.row.root.disclosure"),
                "v"
            );
            assert!(maybe_node_id(&document, "trees.table").is_some());
            assert!(maybe_node_id(&document, "trees.table.header.name").is_some());
            assert!(maybe_node_id(&document, "trees.table.row.1.cell.name").is_some());
            assert!(maybe_node_id(&document, "trees.table.cell.1.0.label").is_some());
            assert_eq!(
                text_content(&document, "trees.table.row.root.disclosure"),
                "v"
            );
            assert_eq!(
                text_content(&document, "trees.table.row.branch-a.disclosure"),
                "v"
            );
            assert_eq!(
                text_content(&document, "trees.table.cell.0.0.label"),
                "Workspace"
            );

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

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "trees.virtual.row.root",
            ));
            assert!(!state.tree_virtual.is_expanded("root"));
            let mut collapsed_virtual = state.view(viewport);
            collapsed_virtual
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("collapsed virtual tree layout");
            assert!(maybe_node_id(&collapsed_virtual, "trees.virtual.row.src").is_none());
            assert_eq!(
                text_content(&collapsed_virtual, "trees.virtual.row.root.disclosure"),
                ">"
            );

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "trees.table.row.1",
            ));
            assert!(!state.tree_table.is_expanded("branch-a"));
            let mut collapsed_table = state.view(viewport);
            collapsed_table
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("collapsed tree table layout");
            assert!(
                maybe_node_id(&collapsed_table, "trees.table.row.widgets.disclosure").is_none()
            );
            assert_eq!(
                text_content(&collapsed_table, "trees.table.row.branch-a.disclosure"),
                ">"
            );

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "trees.table.cell.0.0",
            ));
            assert!(!state.tree_table.is_expanded("root"));
        }

        #[test]
        fn showcase_trees_minimum_uses_scroll_viewport_heights_not_internal_rows() {
            let viewport = UiSize::new(900.0, 1900.0);
            let mut reopened_state = ShowcaseState::default();
            reopened_state.last_desktop_size = viewport;
            reopened_state.update(WidgetAction::activate(UiNodeId::root(), "window.add_all"));
            reopened_state.update(WidgetAction::activate(UiNodeId::root(), "window.clear_all"));
            reopened_state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.toggle.trees",
            ));
            let mut reopened_document = reopened_state.view(viewport);
            reopened_document
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                .expect("reopened trees layout");
            let reopened_window = reopened_document
                .node(node_id(&reopened_document, "showcase.windows.window.trees"))
                .layout()
                .rect;
            assert!(
                reopened_window.height <= 720.0,
                "reopening Trees after Add all/Clear all should spawn at content-fit size, not stale organized height: {reopened_window:?}"
            );

            let mut stale_state = state_with_window("trees");
            stale_state
                .desktop
                .sizes
                .insert("trees".to_string(), UiSize::new(416.0, 1800.0));
            stale_state.desktop.user_sized.insert("trees".to_string());
            stale_state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.toggle.trees",
            ));
            stale_state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.toggle.trees",
            ));
            let mut stale_document = stale_state.view(viewport);
            stale_document
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                .expect("stale reopened trees layout");
            let stale_window = stale_document
                .node(node_id(&stale_document, "showcase.windows.window.trees"))
                .layout()
                .rect;
            assert!(
                stale_window.height <= 720.0,
                "hiding and reopening Trees should discard stale user-sized height: {stale_window:?}"
            );

            let mut organized_state = state_with_window("trees");
            organized_state.last_desktop_size = viewport;
            organized_state.update(WidgetAction::activate(
                UiNodeId::root(),
                "window.organize_open",
            ));
            let organized_size = organized_state
                .desktop
                .size("trees", window_defaults("trees").size);
            assert!(
                organized_size.height <= 720.0,
                "organizing should not expand Trees to the 64k measuring viewport through scroll content: {organized_size:?}"
            );

            let mut state = state_with_window("trees");
            state
                .desktop
                .sizes
                .insert("trees".to_string(), UiSize::new(1.0, 1.0));
            state.desktop.user_sized.insert("trees".to_string());

            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                .expect("trees layout");

            let window = document
                .node(node_id(&document, "showcase.windows.window.trees"))
                .layout()
                .rect;
            let content = document
                .node(node_id(&document, "showcase.windows.window.trees.content"))
                .layout()
                .rect;
            let table = document
                .node(node_id(&document, "trees.table"))
                .layout()
                .rect;
            let slack = content.bottom() - table.bottom();
            assert!(
                slack <= 48.0,
                "trees minimum left excess empty content below the fixed tree/table viewports: window={window:?} content={content:?} table={table:?} slack={slack}"
            );
            assert!(
                window.height <= 720.0,
                "trees minimum should stay close to the visible sections, not virtualized scroll content: {window:?}"
            );
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

            let group_names = [
                "styling.inner.group",
                "styling.outer.group",
                "styling.radius.group",
                "styling.fill.group",
                "styling.stroke.group",
                "styling.shadow.group",
            ];
            let first_rect = document
                .node(node_id(&document, group_names[0]))
                .layout()
                .rect;
            for name in group_names {
                let rect = document.node(node_id(&document, name)).layout().rect;
                assert!(
                    (rect.x - first_rect.x).abs() <= 0.5
                        && (rect.width - first_rect.width).abs() <= 0.5,
                    "{name} should align with the other styling control groups: first={first_rect:?} actual={rect:?}"
                );
            }
        }

        #[test]
        fn showcase_styling_stroke_slider_reaches_large_widths() {
            let mut state = state_with_window("styling");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let slider = node_id(&document, "styling.stroke.slider");
            let slider_rect = document.node(slider).layout().rect;
            let end = UiPoint::new(
                slider_rect.right(),
                slider_rect.y + slider_rect.height * 0.5,
            );
            state.update(WidgetAction::pointer_edit(
                slider,
                "styling.stroke",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    end,
                    UiPoint::new(slider_rect.width, slider_rect.height * 0.5),
                    slider_rect,
                ),
            ));

            assert!(
                (state.styling.stroke_width - STYLING_STROKE_MAX).abs() <= 0.01,
                "stroke slider should reach {STYLING_STROKE_MAX}px, got {}",
                state.styling.stroke_width
            );
        }

        #[test]
        fn showcase_styling_zero_stroke_removes_preview_outline() {
            let mut state = state_with_window("styling");
            state.styling.stroke_width = 0.0;

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let scene = document.node(node_id(&document, "styling.preview.scene"));
            let UiContent::Scene(primitives) = scene.content() else {
                panic!("styling preview should be a scene");
            };
            let Some(ScenePrimitive::Rect(rect)) = primitives.first() else {
                panic!("styling preview should start with the styled rect");
            };
            assert_eq!(
                rect.stroke, None,
                "zero stroke width should remove the preview outline"
            );
        }

        #[test]
        fn showcase_styling_individual_corner_radii_feed_preview_rect() {
            let mut state = state_with_window("styling");
            state.styling.radius_same = false;
            state.styling.corner_radius = 2.0;
            state.styling.corner_ne = 8.0;
            state.styling.corner_sw = 14.0;
            state.styling.corner_se = 20.0;

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let scene = document.node(node_id(&document, "styling.preview.scene"));
            let UiContent::Scene(primitives) = scene.content() else {
                panic!("styling preview should be a scene");
            };
            let Some(ScenePrimitive::Rect(rect)) = primitives.first() else {
                panic!("styling preview should start with the styled rect");
            };
            assert_eq!(
                rect.corner_radii,
                CornerRadii::new(2.0, 8.0, 20.0, 14.0),
                "styling preview should preserve independent NW/NE/SE/SW radii"
            );
        }

        #[test]
        fn showcase_color_picker_includes_button_that_opens_picker() {
            let mut state = state_with_window("color_picker");
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "color.button.open",
            ));

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("color picker layout");

            let edit = node_id(&document, "color.button.open");
            let swatch = node_id(&document, "color.button.open.swatch");
            assert_eq!(
                text_content(&document, "color.button.label"),
                "Button opens color picker"
            );
            assert!(maybe_node_id(&document, "color.button.open.label").is_none());
            assert!(maybe_node_id(&document, "color.button_picker").is_some());
            assert!(maybe_node_id(&document, "color_buttons.edit").is_none());

            let edit_rect = document.node(edit).layout().rect;
            let swatch_rect = document.node(swatch).layout().rect;
            assert_eq!(edit_rect.width, swatch_rect.width);
            assert_eq!(edit_rect.height, swatch_rect.height);
            assert!(
                swatch_rect.x >= edit_rect.x
                    && swatch_rect.y >= edit_rect.y
                    && swatch_rect.right() <= edit_rect.right()
                    && swatch_rect.bottom() <= edit_rect.bottom(),
                "swatch-only color edit button should be a solid button, not cramped text next to a swatch: edit={edit_rect:?} swatch={swatch_rect:?}"
            );
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
        fn showcase_styling_compact_controls_stay_inside_control_panel() {
            let state = state_with_window("styling");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let controls = document
                .node(node_id(&document, "styling.controls"))
                .layout()
                .rect;
            for name in [
                "styling.stroke.slider",
                "styling.shadow_y",
                "styling.shadow_spread",
                "styling.shadow.offsets",
                "styling.shadow.blur_spread",
            ] {
                let rect = document.node(node_id(&document, name)).layout().rect;
                assert!(
                    rect.right() <= controls.right() + 0.5,
                    "{name} should not cross out of the styling control panel: rect={rect:?} controls={controls:?}"
                );
            }

            for (input_name, text_name) in [
                ("styling.stroke", "styling.stroke.value"),
                ("styling.shadow_x", "styling.shadow_x.value"),
                ("styling.shadow_spread", "styling.shadow_spread.value"),
            ] {
                let input = document.node(node_id(&document, input_name)).layout().rect;
                let text = document.node(node_id(&document, text_name)).layout().rect;
                let input_center = input.x + input.width * 0.5;
                let text_center = text.x + text.width * 0.5;
                assert!(
                    text.x >= input.x + 3.0 && text.right() <= input.right() - 3.0,
                    "{text_name} should have readable inset inside {input_name}: text={text:?} input={input:?}"
                );
                assert!(
                    (text_center - input_center).abs() <= 3.0,
                    "{text_name} should be centered in {input_name}: text={text:?} input={input:?}"
                );
            }
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
            state.styling.inner_margin = 32.0;
            state.styling.outer_margin = 40.0;
            state.styling.shadow_x = 24.0;
            state.styling.shadow_y = 24.0;
            state.styling.shadow_blur = 32.0;
            state.styling.shadow_spread = 16.0;
            state
                .desktop
                .sizes
                .insert("styling".to_string(), UiSize::new(420.0, 260.0));
            state.desktop.user_sized.insert("styling".to_string());

            let viewport = UiSize::new(1200.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("styling layout");

            let scene = node_id(&document, "styling.preview.scene");
            let scene_rect = document.node(scene).layout().rect;
            let preview_rect = document
                .node(node_id(&document, "styling.preview"))
                .layout()
                .rect;
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

            let paint = document.paint_list();
            for item in paint.items.iter().filter(|item| item.node == scene) {
                if matches!(item.kind, PaintKind::RichRect(_) | PaintKind::SceneText(_)) {
                    assert!(
                        item.rect.right() <= item.clip_rect.right() + 0.5,
                        "preview item should fit horizontally: rect={:?} clip={:?}",
                        item.rect,
                        item.clip_rect
                    );
                    assert!(
                        item.rect.right() <= preview_rect.right() + 0.5,
                        "preview item should fit inside preview host: rect={:?} preview={preview_rect:?}",
                        item.rect
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
                        "forms.profile.summary",
                        "forms.profile.summary.title",
                        "forms.profile.summary.detail",
                        "forms.profile.summary.hint",
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
                        "forms.profile.action_help",
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
                        "overlays.tooltip_target",
                        "overlays.popup.toggle",
                        "overlays.modal.open",
                        "overlays.tooltip_rect.preview",
                        "overlays.tooltip_rect.scene",
                        "overlays.popup_panel",
                        "overlays.popup_panel.close",
                        "overlays.toasts.controls",
                        "overlays.toasts.status",
                        "overlays.toasts.action_status",
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
                    ],
                    &[],
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

        #[test]
        fn showcase_forms_valid_dirty_email_enables_apply_and_updates_summary() {
            let mut state = state_with_window("forms");
            let viewport = UiSize::new(900.0, 760.0);

            let mut initial = state.view(viewport);
            initial
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("initial forms layout");
            assert_eq!(
                text_content(&initial, "forms.profile.summary.title"),
                "Profile needs fixes"
            );
            let initial_apply = initial.node(node_id(&initial, "forms.profile.actions.apply"));
            assert_eq!(
                initial_apply.accessibility().map(|meta| meta.enabled),
                Some(false),
                "invalid email should keep Apply disabled"
            );

            state.form_email_text.set_text("new@example.com");
            state.update_profile_form_field("email", "new@example.com".to_string());
            let mut valid = state.view(viewport);
            valid
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("valid forms layout");
            assert!(maybe_node_id(&valid, "forms.profile.email.validation").is_none());
            assert_eq!(
                text_content(&valid, "forms.profile.summary.title"),
                "Profile draft"
            );
            assert!(
                text_content(&valid, "forms.profile.summary.detail").contains("new@example.com")
            );
            assert_eq!(
                text_content(&valid, "forms.profile.summary.hint"),
                "Apply saves the draft; Submit saves and marks it submitted."
            );
            assert_eq!(
                text_content(&valid, "forms.profile.action_help"),
                "Apply changes saves this draft and keeps editing. Submit profile saves and marks it submitted."
            );
            assert_eq!(
                text_content(&valid, "forms.profile.actions.submit.label"),
                "Submit profile"
            );
            assert_eq!(
                text_content(&valid, "forms.profile.actions.apply.label"),
                "Apply changes"
            );
            let apply = valid.node(node_id(&valid, "forms.profile.actions.apply"));
            assert_eq!(
                apply.accessibility().map(|meta| meta.enabled),
                Some(true),
                "valid dirty email should enable Apply"
            );

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "forms.profile.apply",
            ));
            let mut applied = state.view(viewport);
            applied
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("applied forms layout");
            assert_eq!(
                text_content(&applied, "forms.profile.summary.title"),
                "Profile saved"
            );
            let apply = applied.node(node_id(&applied, "forms.profile.actions.apply"));
            assert_eq!(
                apply.accessibility().map(|meta| meta.enabled),
                Some(false),
                "saved form should not offer Apply with no changes"
            );
        }

        #[test]
        fn showcase_drag_drop_source_click_cancels_and_drag_updates_cursor() {
            fn drag(phase: WidgetDragPhase) -> WidgetDrag {
                WidgetDrag {
                    phase,
                    origin: UiPoint::new(10.0, 10.0),
                    current: UiPoint::new(42.0, 18.0),
                    previous: UiPoint::new(24.0, 14.0),
                    delta: UiPoint::new(18.0, 4.0),
                    total_delta: UiPoint::new(32.0, 8.0),
                }
            }

            fn pending_cursor_shape(state: &ShowcaseState) -> Option<CursorShape> {
                state
                    .platform
                    .pending_requests()
                    .iter()
                    .rev()
                    .find_map(|request| match &request.request {
                        PlatformRequest::Cursor(CursorRequest::SetShape(shape)) => Some(*shape),
                        _ => None,
                    })
            }

            let mut state = state_with_window("drag_drop");
            state.update(WidgetAction::pointer_activate(
                UiNodeId::root(),
                "drag_drop.text_source",
                1,
            ));
            assert_eq!(state.drag_drop_status, "Text drag canceled");
            assert_eq!(pending_cursor_shape(&state), None);

            state.update(WidgetAction::drag(
                UiNodeId::root(),
                "drag_drop.text_source",
                drag(WidgetDragPhase::Begin),
            ));
            assert_eq!(state.drag_drop_status, "Text drag started");
            assert_eq!(pending_cursor_shape(&state), Some(CursorShape::Grabbing));

            state.platform.drain_requests();
            state.update(WidgetAction::drag(
                UiNodeId::root(),
                "drag_drop.text_source",
                drag(WidgetDragPhase::Update),
            ));
            assert_eq!(state.drag_drop_status, "Text dragging");
            assert_eq!(
                pending_cursor_shape(&state),
                None,
                "repeated drag updates should not enqueue duplicate cursor requests"
            );

            state.update(WidgetAction::drag(
                UiNodeId::root(),
                "drag_drop.text_source",
                drag(WidgetDragPhase::Commit),
            ));
            assert_eq!(state.drag_drop_status, "Text drag finished");
            assert_eq!(pending_cursor_shape(&state), Some(CursorShape::Default));

            state.update(WidgetAction::drag(
                UiNodeId::root(),
                "drag_drop.text_source",
                drag(WidgetDragPhase::Begin),
            ));
            state.update(WidgetAction::drag(
                UiNodeId::root(),
                "drag_drop.text_source",
                drag(WidgetDragPhase::Commit),
            ));
            state.update(WidgetAction::drag(
                UiNodeId::root(),
                "drag_drop.files_only",
                drag(WidgetDragPhase::Commit),
            ));
            assert_eq!(state.drag_drop_status, "Text drag failed");

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("drag drop layout");
            assert_eq!(
                text_content(&document, "drag_drop.status"),
                "Status: Text drag failed"
            );
        }

        #[test]
        fn showcase_buggy_sections_spawn_content_fit_and_minimize_without_clipping() {
            let viewport = UiSize::new(1180.0, 820.0);
            let specs: [(&str, &[&str]); 11] = [
                (
                    "lists_tables",
                    &[
                        "lists_tables.scroll_area",
                        "lists_tables.virtual_list",
                        "lists_tables.data_table.title",
                        "lists_tables.virtualized_table",
                    ],
                ),
                (
                    "diagnostics",
                    &[
                        "diagnostics.inspector",
                        "diagnostics.animation.activity.panel",
                        "diagnostics.animation.timeline.panel",
                        "diagnostics.animation.graph",
                        "diagnostics.animation.controls",
                        "diagnostics.a11y.preview",
                        "diagnostics.a11y.timeline.panel",
                        "diagnostics.findings.panel",
                        "diagnostics.health.panel",
                        "diagnostics.root_causes.panel",
                        "diagnostics.root_cause_timeline.panel",
                        "diagnostics.issue_timeline.panel",
                        "diagnostics.questions.panel",
                        "diagnostics.panel_recommendations.panel",
                        "diagnostics.overlay_recommendations.panel",
                        "diagnostics.coverage.panel",
                        "diagnostics.investigation.panel",
                        "diagnostics.session_narrative.panel",
                        "diagnostics.contract.panel",
                        "diagnostics.invariants.panel",
                        "diagnostics.invariant_timeline.panel",
                        "diagnostics.constraint_timeline.panel",
                        "diagnostics.capture.panel",
                        "diagnostics.node_recommendations.panel",
                        "diagnostics.slow_nodes.panel",
                        "diagnostics.slow_node_timeline.panel",
                        "diagnostics.slow_frame.panel",
                        "diagnostics.waterfall.panel",
                        "diagnostics.bottlenecks.panel",
                        "diagnostics.autopsy.panel",
                        "diagnostics.recorder.panel",
                        "diagnostics.layout_cost_timeline.panel",
                        "diagnostics.layout_pressure.panel",
                        "diagnostics.layout_pressure_timeline.panel",
                        "diagnostics.layout_cost_autopsy.panel",
                        "diagnostics.node_search.panel",
                        "diagnostics.layout_autopsy.panel",
                        "diagnostics.style_compare.panel",
                        "diagnostics.clip_chain.panel",
                        "diagnostics.clip_chain_timeline.panel",
                        "diagnostics.wheel.panel",
                        "diagnostics.scroll_timeline.panel",
                        "diagnostics.responsive.panel",
                        "diagnostics.visibility.panel",
                        "diagnostics.visibility_timeline.panel",
                        "diagnostics.hitbox_map.panel",
                        "diagnostics.hitbox_timeline.panel",
                        "diagnostics.hit_targets.panel",
                        "diagnostics.hitbox_occlusion.panel",
                        "diagnostics.hitbox_occlusion_timeline.panel",
                        "diagnostics.paint_hit.panel",
                        "diagnostics.paint_hit_timeline.panel",
                        "diagnostics.bounds.panel",
                        "diagnostics.text_fit.panel",
                        "diagnostics.text_fit_timeline.panel",
                        "diagnostics.text_style.panel",
                        "diagnostics.text_localization.panel",
                        "diagnostics.text_contrast.panel",
                        "diagnostics.inspect_node.panel",
                        "diagnostics.provenance.panel",
                        "diagnostics.timeline.panel",
                        "diagnostics.regression.panel",
                        "diagnostics.fix_verification.panel",
                        "diagnostics.performance_timeline.panel",
                        "diagnostics.why.panel",
                        "diagnostics.why_timeline.panel",
                        "diagnostics.node_history.panel",
                        "diagnostics.node_change.panel",
                        "diagnostics.overlaps.panel",
                        "diagnostics.overlap_timeline.panel",
                        "diagnostics.overlap_autopsy.panel",
                        "diagnostics.pointer_session.panel",
                        "diagnostics.pointer_autopsy.panel",
                        "diagnostics.point_autopsy.panel",
                        "diagnostics.inspect_point.panel",
                        "diagnostics.resource_timeline.panel",
                        "diagnostics.interaction.panel",
                        "diagnostics.interaction_timeline.panel",
                        "diagnostics.drag.panel",
                        "diagnostics.affordance_timeline.panel",
                        "diagnostics.stacking.panel",
                        "diagnostics.stacking_timeline.panel",
                        "diagnostics.layer_timeline.panel",
                        "diagnostics.batch_timeline.panel",
                        "diagnostics.invalidation_blast.panel",
                        "diagnostics.invalidation_timeline.panel",
                        "diagnostics.commands",
                        "diagnostics.theme",
                    ],
                ),
                (
                    "trees",
                    &[
                        "trees.tree_view",
                        "trees.editable.status",
                        "trees.virtual",
                        "trees.table",
                    ],
                ),
                (
                    "layout_widgets",
                    &[
                        "layout_widgets.dock_shell",
                        "layout_widgets.dock.panel.panel_a",
                        "layout_widgets.dock.panel.workspace",
                        "layout_widgets.dock.panel.panel_b",
                        "layout.panel_a.scroll_area",
                        "layout.workspace.scroll_area",
                        "layout.panel_b.scroll_area",
                    ],
                ),
                (
                    "containers",
                    &[
                        "containers.frame",
                        "containers.grid",
                        "containers.scene",
                        "containers.scroll_area_with_bars",
                        "containers.area.host",
                    ],
                ),
                (
                    "panels",
                    &[
                        "panels.shell",
                        "panels.top",
                        "panels.top_split.handle",
                        "panels.left",
                        "panels.left_split.handle",
                        "panels.center",
                        "panels.right_split.handle",
                        "panels.right",
                        "panels.bottom_split.handle",
                        "panels.bottom",
                    ],
                ),
                (
                    "forms",
                    &[
                        "forms.profile",
                        "forms.profile.summary",
                        "forms.profile.name.input",
                        "forms.profile.email.input",
                        "forms.profile.role.input",
                        "forms.profile.actions.submit",
                    ],
                ),
                (
                    "overlays",
                    &[
                        "overlays.collapsing",
                        "overlays.tooltip_target",
                        "overlays.popup.toggle",
                        "overlays.tooltip_rect.preview",
                        "overlays.popup_panel",
                        "overlays.toasts.controls",
                        "overlays.modal",
                    ],
                ),
                (
                    "timeline",
                    &[
                        "timeline.viewport",
                        "timeline.lane_labels",
                        "timeline.lane_labels.video",
                    ],
                ),
                (
                    "canvas",
                    &[
                        "canvas.section_scroll",
                        "canvas.options",
                        "canvas.grow_horizontal",
                        "canvas.grow_vertical",
                        "canvas.keep_aspect_ratio",
                        "canvas.shader",
                    ],
                ),
                (
                    "styling",
                    &["styling.grid", "styling.controls", "styling.preview.scene"],
                ),
            ];

            for (id, required_nodes) in specs {
                let state = state_with_window(id);
                let mut document = state.view(viewport);
                document
                    .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                    .unwrap_or_else(|error| panic!("{id} startup failed layout: {error}"));
                assert_default_window_uses_content_fit(&document, id);
                assert_no_severe_showcase_warnings(id, "startup", &document);
                assert_required_demo_nodes_fit_minimized_window(&document, id, required_nodes);

                let minimized = minimized_showcase_window_document(id, viewport);
                assert_window_resolved_to_minimum(&minimized, id);
                assert_no_severe_showcase_warnings(id, "minimized", &minimized);
                assert_required_demo_nodes_fit_minimized_window(&minimized, id, required_nodes);
            }
        }

        #[test]
        fn showcase_containers_stays_narrow_and_panels_are_separate() {
            let viewport = UiSize::new(1180.0, 820.0);
            let mut containers = state_with_window("containers").view(viewport);
            containers
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                .expect("containers layout");
            let container_window = containers
                .node(node_id(&containers, "showcase.windows.window.containers"))
                .layout()
                .rect;
            assert!(
                container_window.width <= 460.0,
                "containers should stay a narrow utility demo after panels were split out: {container_window:?}"
            );
            assert!(maybe_node_id(&containers, "containers.panels").is_none());
            assert!(maybe_node_id(&containers, "panels.shell").is_none());

            let mut panels = state_with_window("panels").view(viewport);
            panels
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                .expect("panels layout");
            for node in [
                "panels.shell",
                "panels.top",
                "panels.top_split.handle",
                "panels.left",
                "panels.left_split.handle",
                "panels.center",
                "panels.right_split.handle",
                "panels.right",
                "panels.bottom_split.handle",
                "panels.bottom",
            ] {
                assert!(maybe_node_id(&panels, node).is_some(), "{node}");
            }
            assert!(maybe_node_id(&panels, "panels.group").is_none());
        }

        #[test]
        fn showcase_panels_fill_available_shell_and_resize_handles_highlight() {
            let viewport = UiSize::new(1180.0, 820.0);
            let mut document = state_with_window("panels").view(viewport);
            document
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                .expect("panels layout");

            let shell = document
                .node(node_id(&document, "panels.shell"))
                .layout()
                .rect;
            let top = document
                .node(node_id(&document, "panels.top"))
                .layout()
                .rect;
            let bottom = document
                .node(node_id(&document, "panels.bottom"))
                .layout()
                .rect;
            assert!(
                top.y <= shell.y + 1.0,
                "top panel should start at the shell top: top={top:?} shell={shell:?}"
            );
            assert!(
                bottom.bottom() >= shell.bottom() - 1.0,
                "bottom panel should reach the shell bottom instead of leaving unused space: bottom={bottom:?} shell={shell:?}"
            );

            for (handle_name, action) in [
                ("panels.top_split.handle", "panels.resize.top"),
                ("panels.bottom_split.handle", "panels.resize.bottom"),
                ("panels.left_split.handle", "panels.resize.left"),
                ("panels.right_split.handle", "panels.resize.right"),
            ] {
                let handle = node_id(&document, handle_name);
                assert_eq!(
                    document.node(handle).action(),
                    Some(&WidgetActionBinding::action(action)),
                    "{handle_name} should publish a resize action"
                );
                assert_eq!(
                    document.node(handle).action_mode(),
                    operad::WidgetActionMode::PointerEditParentRect,
                    "{handle_name} should resize using its split root rect"
                );
                let rect = document.node(handle).layout().rect;
                if rect.width > rect.height {
                    assert!(
                        rect.height <= 3.0,
                        "{handle_name} should render as a thin horizontal splitter: {rect:?}"
                    );
                } else {
                    assert!(
                        rect.width <= 3.0,
                        "{handle_name} should render as a thin vertical splitter: {rect:?}"
                    );
                }
            }

            let handle = node_id(&document, "panels.left_split.handle");
            let normal = *document.node(handle).visual();
            document.set_focus_state(UiFocusState {
                hovered: Some(handle),
                pressed: None,
                focused: None,
            });
            assert_ne!(
                *document.node(handle).visual(),
                normal,
                "split handles should visibly highlight on hover"
            );
        }

        #[test]
        fn showcase_all_windows_spawn_with_content_fit_constraints() {
            let viewport = UiSize::new(1180.0, 820.0);
            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                let state = state_with_window(id);
                let mut document = state.view(viewport);
                document
                    .compute_layout(viewport, &mut CosmicTextMeasurer::new())
                    .unwrap_or_else(|error| panic!("{id} startup failed layout: {error}"));
                assert_default_window_uses_content_fit(&document, id);
                assert_no_severe_showcase_warnings(id, "startup", &document);
            }
        }

        fn assert_default_window_uses_content_fit(document: &UiDocument, id: &str) {
            let window = document.node(node_id(document, &format!("showcase.windows.window.{id}")));
            assert!(
                matches!(
                    window.layout_constraint(),
                    Some(UiNodeLayoutConstraint::StackedIntrinsicSize {
                        fit_to_preferred: true,
                        ..
                    })
                ),
                "{id} should start as a content-fit floating window"
            );
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
            let mut document =
                minimized_showcase_window_document("media", UiSize::new(900.0, 760.0));
            assert_scroll_area_has_visible_auto_scrollbar(&mut document, "media.section_scroll");
        }

        #[test]
        fn showcase_media_icons_wrap_to_available_width_without_horizontal_scroll() {
            let mut state = state_with_window("media");
            state
                .desktop
                .sizes
                .insert("media".to_string(), UiSize::new(260.0, 430.0));
            state.desktop.user_sized.insert("media".to_string());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("media layout");

            let section = node_id(&document, "media.section_scroll");
            let scroll = document
                .scroll_state(section)
                .expect("media section scroll");
            assert_eq!(
                scroll.max_offset().x,
                0.0,
                "media should wrap icons instead of creating horizontal scroll: {scroll:?}"
            );

            let section_rect = document.node(section).layout().rect;
            let icon_rects = BuiltInIcon::COMMON
                .iter()
                .take(12)
                .map(|icon| {
                    let key = media_icon_test_name(*icon);
                    document
                        .node(node_id(&document, &format!("media.icon_tile.{key}")))
                        .layout()
                        .rect
                })
                .collect::<Vec<_>>();
            let first_row_y = icon_rects[0].y;
            let first_row_count = icon_rects
                .iter()
                .take_while(|rect| (rect.y - first_row_y).abs() <= 0.5)
                .count();
            assert!(
                first_row_count < MEDIA_ICON_COLUMNS,
                "narrow media windows should wrap before the five-column cap instead of forcing a wide minimum: {icon_rects:?}"
            );
            assert!(
                first_row_count >= 2,
                "narrow media windows should still use available row space before wrapping: {icon_rects:?}"
            );
            assert!(
                icon_rects.iter().any(|rect| rect.y > first_row_y + 20.0),
                "media icon grid should place tiles onto later rows: {icon_rects:?}"
            );
            assert!(
                icon_rects
                    .iter()
                    .all(|rect| rect.x >= section_rect.x - 0.5
                        && rect.right() <= section_rect.right() + 0.5),
                "wrapped icon tiles should stay inside the visible section width: section={section_rect:?} tiles={icon_rects:?}"
            );
        }

        #[test]
        fn showcase_media_icons_cap_at_five_columns_when_space_allows() {
            let state = state_with_window("media");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("media layout");

            let icon_rects = BuiltInIcon::COMMON
                .iter()
                .take(MEDIA_ICON_COLUMNS + 1)
                .map(|icon| {
                    let key = media_icon_test_name(*icon);
                    document
                        .node(node_id(&document, &format!("media.icon_tile.{key}")))
                        .layout()
                        .rect
                })
                .collect::<Vec<_>>();
            let first_row_y = icon_rects[0].y;
            for rect in &icon_rects[..MEDIA_ICON_COLUMNS] {
                assert!(
                    (rect.y - first_row_y).abs() <= 0.5,
                    "first five media icon tiles should share the capped grid row: {icon_rects:?}"
                );
            }
            assert!(
                icon_rects[MEDIA_ICON_COLUMNS].y > first_row_y + 20.0,
                "sixth media icon tile should wrap after the five-column cap: {icon_rects:?}"
            );
            assert!(
                icon_rects[MEDIA_ICON_COLUMNS - 1].right()
                    <= icon_rects[0].x + media_icon_grid_width(MEDIA_ICON_COLUMNS) + 0.5,
                "first media grid row should not exceed the five-column cap: {icon_rects:?}"
            );
        }

        #[test]
        fn showcase_media_window_starts_at_capped_grid_width() {
            let state = state_with_window("media");
            let viewport = UiSize::new(1200.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("media layout");

            let window = document
                .node(node_id(&document, "showcase.windows.window.media"))
                .layout()
                .rect;
            let content = document
                .node(node_id(&document, "showcase.windows.window.media.content"))
                .layout()
                .rect;
            let grid = document
                .node(node_id(&document, "media.icons"))
                .layout()
                .rect;
            let title_intrinsic = document
                .intrinsic_size(
                    node_id(&document, "showcase.windows.window.media.title_bar"),
                    &mut ApproxTextMeasurer,
                )
                .expect("title intrinsic");
            let content_intrinsic = document
                .intrinsic_size(
                    node_id(&document, "showcase.windows.window.media.content"),
                    &mut ApproxTextMeasurer,
                )
                .expect("content intrinsic");
            let section_intrinsic = document
                .intrinsic_size(
                    node_id(&document, "media.section_scroll"),
                    &mut ApproxTextMeasurer,
                )
                .expect("section intrinsic");

            assert!(
                window.width <= media_icon_grid_width(MEDIA_ICON_COLUMNS) + 64.0,
                "media window should spawn near its capped grid width instead of leaving a wide empty surface: window={window:?} content={content:?} grid={grid:?} title_intrinsic={title_intrinsic:?} content_intrinsic={content_intrinsic:?} section_intrinsic={section_intrinsic:?}"
            );
            assert!(
                (grid.width - media_icon_grid_width(MEDIA_ICON_COLUMNS)).abs() <= 0.5,
                "media icon grid should still use exactly the five-column cap: grid={grid:?}"
            );
        }

        #[test]
        fn showcase_media_wrapped_icon_grid_reserves_its_full_height() {
            let state = state_with_window("media");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("media layout");

            let icon_bottom = BuiltInIcon::COMMON
                .iter()
                .map(|icon| {
                    let key = media_icon_test_name(*icon);
                    document
                        .node(node_id(&document, &format!("media.icon_tile.{key}")))
                        .layout()
                        .rect
                        .bottom()
                })
                .fold(f32::NEG_INFINITY, f32::max);
            let variants_label = document
                .node(node_id(&document, "media.variants.label"))
                .layout()
                .rect;
            assert!(
                variants_label.y >= icon_bottom + 9.0,
                "content after the icon grid should start below every wrapped icon row: icon_bottom={icon_bottom} variants_label={variants_label:?}"
            );
        }

        #[test]
        fn showcase_shader_effects_exposes_element_and_widget_shader_hooks() {
            let state = state_with_window("shaders");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader showcase layout");

            for (name, key) in [
                ("shaders.effect.tint.swatch", ShaderEffect::TINT),
                ("shaders.effect.shine.swatch", ShaderEffect::SHINE),
                ("shaders.effect.glow.swatch", ShaderEffect::GLOW),
                ("shaders.widgets.button", ShaderEffect::SHINE),
                ("shaders.widgets.button.image", ShaderEffect::TINT),
                ("shaders.widgets.checkbox.check", ShaderEffect::GLOW),
                ("shaders.widgets.progress.fill", ShaderEffect::SHINE),
                ("shaders.widgets.slider.fill", ShaderEffect::TINT),
                ("shaders.widgets.slider.thumb", ShaderEffect::GLOW),
            ] {
                let node = document.node(node_id(&document, name));
                assert_eq!(
                    node.shader().map(|shader| shader.key.as_str()),
                    Some(key),
                    "{name} should expose {key}"
                );
            }
            assert_no_severe_showcase_warnings("shaders", "shader demo", &document);
        }

        #[test]
        fn showcase_shader_lab_edits_canvas_source_and_targets_widgets() {
            let mut state = state_with_window("shader_lab");
            state.shader_lab_material_outset = SHADER_LAB_MATERIAL_OUTSET_MAX;
            let viewport = UiSize::new(1000.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader lab layout");

            assert!(maybe_node_id(&document, "shader_lab.preview.canvas").is_some());
            assert!(maybe_node_id(&document, "shader_lab.editor").is_some());
            assert!(
                maybe_node_id(&document, "shader_lab.preview.status").is_none(),
                "the valid canvas preview should not repeat that it uses the editor source"
            );
            for (name, expected) in [
                ("shader_lab.frame_text.toggle.label", "Frame"),
                ("shader_lab.button_text.toggle.label", "Button"),
                ("shader_lab.material.title", "Material"),
                ("shader_lab.material.current.text", "Selected"),
                ("shader_lab.material.outset.text", "Glow"),
                ("shader_lab.material.circle_hit.text", "Circle"),
                ("shader_lab.material.geometry_chip.text", "Warp"),
                ("shader_lab.validation", "WGSL valid"),
            ] {
                assert_eq!(
                    text_content(&document, name),
                    expected,
                    "{name} should keep the shader lab copy terse"
                );
            }
            assert_eq!(
                document
                    .node(node_id(&document, "shader_lab.target"))
                    .action()
                    .and_then(|action| action.action_id())
                    .map(|id| id.as_str()),
                Some("shader_lab.target.toggle")
            );
            assert_eq!(
                document
                    .node(node_id(&document, "shader_lab.preset"))
                    .action()
                    .and_then(|action| action.action_id())
                    .map(|id| id.as_str()),
                Some("shader_lab.preset.toggle")
            );
            let outset_material = document
                .node(node_id(&document, "shader_lab.material.outset"))
                .material()
                .expect("shader lab should show a declared paint outset material");
            assert_eq!(
                outset_material
                    .shader
                    .as_ref()
                    .map(|shader| shader.key.as_str()),
                Some(ShaderEffect::GLOW)
            );
            assert!(
                outset_material.visual_outset_for_rect(UiRect::new(0.0, 0.0, 40.0, 40.0))
                    >= SHADER_LAB_MATERIAL_OUTSET
            );
            let material_controls = document
                .node(node_id(&document, "shader_lab.material.controls"))
                .layout()
                .rect;
            for name in ["shader_lab.material.current", "shader_lab.material.outset"] {
                let node = document.node(node_id(&document, name));
                let rect = node.layout().rect;
                let material = node
                    .material()
                    .unwrap_or_else(|| panic!("{name:?} should have a material"));
                let paint_top = rect.y - material.visual_outset_for_rect(rect);
                assert!(
                    paint_top >= material_controls.bottom() + 5.5,
                    "{name} paint should clear the material controls: controls={material_controls:?} rect={rect:?} paint_top={paint_top}"
                );
            }
            let circle_material = document
                .node(node_id(&document, "shader_lab.material.circle_hit"))
                .material()
                .expect("shader lab should show an explicit hit shape");
            assert_eq!(circle_material.hit_shape, ElementShape::circle());
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.material.shader.option.grid",
            ));
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.material.shape.option.circle",
            ));
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.material.geometry.option.skew",
            ));
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader lab material layout");
            let current_material = document
                .node(node_id(&document, "shader_lab.material.current"))
                .material()
                .expect("shader lab should expose the selected material");
            assert_eq!(
                current_material
                    .shader
                    .as_ref()
                    .map(|shader| shader.key.as_str()),
                Some(ShaderEffect::GRID)
            );
            assert_eq!(current_material.hit_shape, ElementShape::circle());
            assert_eq!(
                current_material.geometry_effect,
                GeometryEffect::skew(0.12, 0.0)
            );
            let UiContent::Canvas(canvas) = document
                .node(node_id(&document, "shader_lab.preview.canvas"))
                .content()
            else {
                panic!("shader lab canvas preview should be a canvas");
            };
            let program = canvas.program.as_ref().expect("canvas shader program");
            assert!(program.wgsl.contains("fn fs_main"));
            assert!(
                program
                    .constants
                    .iter()
                    .any(|constant| constant.name == "TIME"),
                "shader lab canvas should expose time as a shader constant"
            );

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.preset.option.grid",
            ));
            assert_eq!(state.shader_lab_preset, ShaderLabPreset::Grid);
            assert!(state.shader_lab_source.text().contains("grid_line"));
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.preset.option.vertex_warp",
            ));
            assert_eq!(state.shader_lab_preset, ShaderLabPreset::VertexWarp);
            assert!(
                state.shader_lab_source.text().contains("p + bend"),
                "vertex warp preset should visibly edit vertex positions"
            );
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.preset.option.grid",
            ));

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.target.option.frame",
            ));
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader lab frame layout");
            let UiContent::Canvas(frame_shader) = document
                .node(node_id(&document, "shader_lab.preview.frame.shader"))
                .content()
            else {
                panic!("shader lab frame preview should use a canvas shader");
            };
            assert!(
                frame_shader
                    .program
                    .as_ref()
                    .expect("frame shader program")
                    .wgsl
                    .contains("grid_line"),
                "frame target should use the edited WGSL source"
            );

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.target.option.button",
            ));
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader lab button layout");
            let UiContent::Canvas(button_shader) = document
                .node(node_id(&document, "shader_lab.preview.button.shader"))
                .content()
            else {
                panic!("shader lab button preview should use a canvas shader");
            };
            assert!(
                button_shader
                    .program
                    .as_ref()
                    .expect("button shader program")
                    .wgsl
                    .contains("grid_line"),
                "button target should use the edited WGSL source"
            );

            state.shader_lab_source.set_text(
                "@fragment fn nope() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
            );
            state.refresh_shader_lab_validation();
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader lab invalid source layout");
            let UiContent::Text(validation) = document
                .node(node_id(&document, "shader_lab.validation"))
                .content()
            else {
                panic!("shader lab should show validation text");
            };
            let validation_text = validation.text.as_str();
            assert!(
                validation_text.contains("WGSL error"),
                "invalid WGSL should render an error label: {validation_text}"
            );
            let UiContent::Canvas(error_shader) = document
                .node(node_id(&document, "shader_lab.preview.button.shader"))
                .content()
            else {
                panic!("invalid button preview should still be a canvas");
            };
            assert!(
                error_shader
                    .program
                    .as_ref()
                    .expect("error shader program")
                    .wgsl
                    .contains("SHADER_LAB_ERROR")
                    || error_shader
                        .program
                        .as_ref()
                        .expect("error shader program")
                        .wgsl
                        .contains("stripe"),
                "invalid WGSL should use the fallback error shader"
            );
            assert_no_severe_showcase_warnings("shader_lab", "shader lab", &document);
        }

        #[test]
        fn showcase_shader_lab_uses_resizable_scrolled_editor_and_larger_preview() {
            let mut state = state_with_window("shader_lab");
            let viewport = UiSize::new(1400.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader lab layout");

            let workspace = node_id(&document, "shader_lab.workspace");
            let handle = node_id(&document, "shader_lab.workspace.handle");
            assert_eq!(
                document
                    .node(handle)
                    .action()
                    .and_then(|action| action.action_id())
                    .map(|id| id.as_str()),
                Some("shader_lab.workspace.resize")
            );

            let preview = document
                .node(node_id(&document, "shader_lab.preview.surface"))
                .layout()
                .rect;
            assert!(
                preview.width >= 340.0 && preview.height >= 280.0,
                "shader preview should start large enough to inspect the live result: {preview:?}"
            );

            let editor_scroll_node = node_id(&document, "shader_lab.editor.scroll");
            let editor_scroll = document
                .scroll_state(editor_scroll_node)
                .expect("shader source editor should be scrollable");
            assert_eq!(editor_scroll.axes(), ScrollAxes::BOTH);
            assert!(
                editor_scroll.max_offset().x > 0.0 && editor_scroll.max_offset().y > 0.0,
                "shader editor should scroll both long WGSL lines and tall source: {editor_scroll:?}"
            );

            state.shader_lab_editor_scroll = UiPoint::new(48.0, 96.0);
            let mut scrolled = state.view(viewport);
            scrolled
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("scrolled shader lab layout");
            let applied_scroll = scrolled
                .scroll_state(node_id(&scrolled, "shader_lab.editor.scroll"))
                .expect("shader source editor scroll");
            assert_eq!(
                applied_scroll.offset(),
                applied_scroll.clamp_offset(UiPoint::new(48.0, 96.0)),
                "shader editor should preserve the user's scroll offset"
            );

            let workspace_rect = document.node(workspace).layout().rect;
            let handle_rect = document.node(handle).layout().rect;
            let start = UiPoint::new(handle_rect.x + handle_rect.width * 0.5, handle_rect.y + 8.0);
            state.update(WidgetAction::pointer_edit(
                handle,
                "shader_lab.workspace.resize",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Begin,
                    start,
                    UiPoint::new(start.x - workspace_rect.x, start.y - workspace_rect.y),
                    workspace_rect,
                ),
            ));
            state.update(WidgetAction::pointer_edit(
                handle,
                "shader_lab.workspace.resize",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    UiPoint::new(start.x + 90.0, start.y),
                    UiPoint::new(
                        start.x + 90.0 - workspace_rect.x,
                        start.y - workspace_rect.y,
                    ),
                    workspace_rect,
                ),
            ));
            assert!(
                state.shader_lab_split.fraction > 0.52,
                "dragging the shader lab splitter should resize the preview/editor panes: {:?}",
                state.shader_lab_split
            );
        }

        #[test]
        fn showcase_shader_lab_frame_button_controls_affect_preview_chrome() {
            let mut state = state_with_window("shader_lab");
            let viewport = UiSize::new(1400.0, 760.0);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.target.frame",
            ));
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader lab frame layout");
            let frame = document
                .node(node_id(&document, "shader_lab.preview.frame"))
                .layout()
                .rect;
            let preview = document
                .node(node_id(&document, "shader_lab.preview.surface"))
                .layout()
                .rect;
            let frame_label = document
                .node(node_id(&document, "shader_lab.preview.frame.label"))
                .layout()
                .rect;
            assert!(
                (rect_center(frame).x - rect_center(frame_label).x).abs() < 1.0,
                "frame label should be horizontally centered: frame={frame:?} label={frame_label:?}"
            );
            assert!(
                (rect_center(frame).y - rect_center(frame_label).y).abs() < 1.0,
                "frame label should be vertically centered: frame={frame:?} label={frame_label:?}"
            );
            assert!(
                frame.width >= preview.width * 0.55 && frame.height >= preview.height * 0.35,
                "frame target should occupy a meaningful part of the preview: frame={frame:?} preview={preview:?}"
            );

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.frame_text.toggle",
            ));
            let mut hidden_frame_text = state.view(viewport);
            hidden_frame_text
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader lab hidden frame text layout");
            assert!(
                maybe_node_id(&hidden_frame_text, "shader_lab.preview.frame.label").is_none(),
                "frame text toggle should remove the frame label from the preview"
            );

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.target.button",
            ));
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "shader_lab.button_text.toggle",
            ));
            let target = UiRect::new(0.0, 0.0, 100.0, 20.0);
            state.update(WidgetAction::pointer_edit(
                UiNodeId::root(),
                "shader_lab.surface.stroke",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    UiPoint::new(target.x, target.y + 10.0),
                    UiPoint::new(0.0, 10.0),
                    target,
                ),
            ));
            state.update(WidgetAction::pointer_edit(
                UiNodeId::root(),
                "shader_lab.surface.radius",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    UiPoint::new(target.right(), target.y + 10.0),
                    UiPoint::new(target.width, 10.0),
                    target,
                ),
            ));
            let mut button_document = state.view(viewport);
            button_document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader lab button chrome layout");
            assert_eq!(
                text_content(&button_document, "shader_lab.preview.button.label"),
                "",
                "button text toggle should remove the visible button label"
            );
            let button =
                button_document.node(node_id(&button_document, "shader_lab.preview.button"));
            assert_eq!(
                button.visual().stroke,
                None,
                "zero border width should remove the button stroke"
            );
            assert!(
                button.visual().corner_radius >= SHADER_LAB_SURFACE_RADIUS_MAX - 0.1,
                "radius slider should update the button chrome: {:?}",
                button.visual()
            );
        }

        fn assert_scroll_area_has_visible_auto_scrollbar(document: &mut UiDocument, name: &str) {
            let scroll_node = node_id(document, name);
            let scroll = document
                .scroll_state(scroll_node)
                .unwrap_or_else(|| panic!("{name:?} should be scrollable"));
            assert!(
                scroll.max_offset().y > 0.0,
                "{name:?} should have a vertical scroll range: {scroll:?}"
            );
            let rect = document.node(scroll_node).layout().rect;
            document.handle_input(UiInputEvent::PointerMove(UiPoint::new(
                rect.x + 8.0,
                rect.y + 8.0,
            )));
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

        fn assert_scroll_area_has_no_horizontal_auto_scrollbar(document: &UiDocument, name: &str) {
            let scroll_node = node_id(document, name);
            let scroll = document
                .scroll_state(scroll_node)
                .unwrap_or_else(|| panic!("{name:?} should be scrollable"));
            assert_eq!(
                scroll.max_offset().x,
                0.0,
                "{name:?} should not have a horizontal scroll range: {scroll:?}"
            );
            let rect = document.node(scroll_node).layout().rect;
            let scrollbar_items = document
                .paint_list()
                .items
                .into_iter()
                .filter(|item| {
                    item.node == scroll_node
                        && matches!(item.kind, PaintKind::Rect { .. })
                        && item.rect.height <= 8.0
                        && item.rect.width >= 18.0
                        && item.rect.y >= rect.bottom() - 12.0
                        && item.rect.bottom() <= rect.bottom() + 0.5
                })
                .collect::<Vec<_>>();
            assert!(
                scrollbar_items.is_empty(),
                "{name:?} should not paint a horizontal automatic scrollbar, got {scrollbar_items:#?}"
            );
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
        fn showcase_canvas_minimum_prevents_tiny_user_size() {
            let mut state = state_with_window("canvas");
            state
                .desktop
                .sizes
                .insert("canvas".to_string(), UiSize::new(420.0, 160.0));
            state.desktop.user_sized.insert("canvas".to_string());

            let viewport = UiSize::new(1180.0, 820.0);
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
            assert!(rect.width >= 719.0, "{rect:?}");
            assert!(rect.height >= 404.0, "{rect:?}");

            let window = document
                .node(node_id(&document, "showcase.windows.window.canvas"))
                .layout()
                .rect;
            assert!(
                window.width >= 740.0 && window.height >= 500.0,
                "tiny user size should be clamped to the canvas demo's content minimum: window={window:?} canvas={rect:?}"
            );
        }

        #[test]
        fn showcase_canvas_default_window_spawns_to_canvas_intrinsic_size() {
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
            let canvas = document
                .node(node_id(&document, "canvas.shader"))
                .layout()
                .rect;
            let content = document
                .node(node_id(&document, "showcase.windows.window.canvas.content"))
                .layout()
                .rect;
            let body = document
                .node(node_id(&document, "canvas.section_scroll"))
                .layout()
                .rect;
            let controls = document
                .node(node_id(&document, "canvas.options"))
                .layout()
                .rect;
            assert!(maybe_node_id(&document, "canvas.grow_horizontal").is_some());
            assert!(maybe_node_id(&document, "canvas.grow_vertical").is_some());
            assert!(maybe_node_id(&document, "canvas.keep_aspect_ratio").is_some());
            assert!(window.width >= 739.0, "{window:?}");
            assert!(
                (canvas.width / canvas.height - 16.0 / 9.0).abs() < 0.01,
                "canvas={canvas:?}"
            );
            assert!(
                canvas.width >= 719.0 && canvas.height >= 404.0,
                "window={window:?} canvas={canvas:?}"
            );
            assert!(
                window.height <= canvas.height + 112.0,
                "window should be fit to canvas content instead of old excess height: window={window:?} content={content:?} body={body:?} controls={controls:?} canvas={canvas:?}"
            );
        }

        #[test]
        fn showcase_canvas_drag_rotates_cube() {
            let mut state = state_with_window("canvas");
            let viewport = UiSize::new(1180.0, 820.0);
            let mut frame_state = HostDocumentFrameState::default();
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let canvas = node_id(&document, "canvas.shader");
            assert_eq!(
                document.node(canvas).action_mode(),
                operad::WidgetActionMode::Drag,
                "the canvas demo should emit drag actions for cube rotation"
            );
            assert_eq!(
                document
                    .node(canvas)
                    .action()
                    .and_then(|action| action.action_id())
                    .map(|id| id.as_str()),
                Some("canvas.rotate")
            );

            let rect = document.node(canvas).layout().rect;
            let start = rect_center(rect);
            let end = UiPoint::new(start.x + 72.0, start.y + 24.0);
            let initial_yaw = state.cube.yaw;
            let initial_pitch = state.cube.pitch;

            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(PointerEventKind::Down(PointerButton::Primary), start, 1),
            );
            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(PointerEventKind::Move, end, 16),
            );
            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(PointerEventKind::Up(PointerButton::Primary), end, 32),
            );

            assert!(
                state.cube.yaw > initial_yaw && state.cube.pitch > initial_pitch,
                "dragging inside the canvas should rotate the cube: initial=({initial_yaw}, {initial_pitch}) current=({}, {})",
                state.cube.yaw,
                state.cube.pitch
            );
        }

        #[test]
        fn showcase_canvas_sizing_options_toggle_canvas_constraints() {
            let mut state = state_with_window("canvas");
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "canvas.grow_horizontal",
            ));
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "canvas.grow_vertical",
            ));
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "canvas.keep_aspect_ratio",
            ));
            assert!(!state.canvas_grow_horizontal);
            assert!(!state.canvas_grow_vertical);
            assert!(!state.canvas_keep_aspect_ratio);

            let viewport = UiSize::new(1180.0, 820.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            for name in [
                "canvas.grow_horizontal",
                "canvas.grow_vertical",
                "canvas.keep_aspect_ratio",
            ] {
                assert_eq!(
                    document
                        .node(node_id(&document, name))
                        .accessibility()
                        .and_then(|meta| meta.checked),
                    Some(operad::AccessibilityChecked::False),
                    "{name} should reflect the toggled-off option"
                );
            }

            let canvas = document
                .node(node_id(&document, "canvas.shader"))
                .layout()
                .rect;
            assert!(
                (canvas.width - 720.0).abs() < 0.01 && (canvas.height - 405.0).abs() < 0.01,
                "fixed canvas options should use the intrinsic demo size: {canvas:?}"
            );
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
        fn showcase_slider_drag_position_does_not_create_horizontal_scrollbar() {
            let mut state = state_with_window("slider");
            state
                .desktop
                .positions
                .insert("slider".to_string(), UiPoint::new(133.333, 97.667));
            state
                .desktop
                .sizes
                .insert("slider".to_string(), UiSize::new(430.0, 560.0));
            state.desktop.user_sized.insert("slider".to_string());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            assert_scroll_area_has_no_horizontal_auto_scrollbar(&document, "slider.section_scroll");
        }

        #[test]
        fn showcase_buttons_window_can_shrink_below_default_height() {
            fn resize_buttons_window(state: &mut ShowcaseState, window: UiRect, target: UiSize) {
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
                        UiPoint::new(window.x + target.width, window.y + target.height),
                        UiPoint::new(target.width, target.height),
                        window,
                    ),
                ));
            }

            fn assert_buttons_fit(document: &UiDocument) {
                let content = document
                    .node(node_id(document, "showcase.windows.window.buttons.content"))
                    .layout()
                    .rect;
                let last = document
                    .node(node_id(document, "buttons.last"))
                    .layout()
                    .rect;
                assert!(
                    last.bottom() <= content.bottom(),
                    "last={last:?} content={content:?}"
                );
            }

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
            resize_buttons_window(&mut state, window, UiSize::new(window.width, 150.0));

            let mut resized = state.view(viewport);
            resized
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("wide resized buttons layout");
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
            assert_buttons_fit(&resized);

            resize_buttons_window(&mut state, resized_window, UiSize::new(185.0, 120.0));
            let mut narrow = state.view(viewport);
            narrow
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("narrow resized buttons layout");
            let narrow_window = narrow
                .node(node_id(&narrow, "showcase.windows.window.buttons"))
                .layout()
                .rect;
            assert!(
                narrow_window.width <= 220.0,
                "narrow_window={narrow_window:?}"
            );
            assert!(
                narrow_window.height > resized_window.height + 180.0,
                "wide={resized_window:?} narrow={narrow_window:?}"
            );
            assert_buttons_fit(&narrow);
        }

        #[test]
        fn showcase_buttons_toggle_uses_state_visual_without_pressed_flash() {
            let mut state = state_with_window("buttons");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("buttons layout");

            let toggle = node_id(&document, "button.toggle");
            let toggle_rect = document.node(toggle).layout().rect;
            let off_visual = *document.node(toggle).visual();
            let off_pressed_expected = adjusted_button_visual(off_visual, -34);
            let input = document.handle_input(UiInputEvent::PointerDown(UiPoint::new(
                toggle_rect.x + toggle_rect.width * 0.5,
                toggle_rect.y + toggle_rect.height * 0.5,
            )));
            assert_eq!(input.pressed, Some(toggle));
            assert_eq!(*document.node(toggle).visual(), off_pressed_expected);
            assert_eq!(text_content(&document, "button.toggle.label"), "Toggle off");
            let accessibility = document.node(toggle).accessibility().unwrap();
            assert_eq!(accessibility.role, AccessibilityRole::ToggleButton);
            assert_eq!(accessibility.pressed, Some(false));

            state.update(WidgetAction::activate(toggle, "button.toggle"));
            let mut toggled_document = state.view(viewport);
            toggled_document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("toggled buttons layout");

            let toggled = node_id(&toggled_document, "button.toggle");
            assert_eq!(
                text_content(&toggled_document, "button.toggle.label"),
                "Toggle on"
            );
            assert_eq!(
                *toggled_document.node(toggled).visual(),
                button_visual(48, 112, 184)
            );
            let accessibility = toggled_document.node(toggled).accessibility().unwrap();
            assert_eq!(accessibility.role, AccessibilityRole::ToggleButton);
            assert_eq!(accessibility.pressed, Some(true));
        }

        #[test]
        fn showcase_wrapped_rows_keep_their_measured_height() {
            for (window, rows) in [
                (
                    "buttons",
                    &[
                        ("buttons.row", "buttons.row.options"),
                        ("buttons.row.options", "buttons.row.helpers"),
                        ("buttons.row.helpers", "buttons.last"),
                    ][..],
                ),
                ("labels", &[("labels.wrap.row", "labels.links")][..]),
                (
                    "toggles",
                    &[
                        ("toggles.radio_examples", "toggles.switch.separator"),
                        ("toggles.switch_examples", "toggles.theme.separator"),
                    ][..],
                ),
            ] {
                let mut state = state_with_window(window);
                state
                    .desktop
                    .sizes
                    .insert(window.to_string(), UiSize::new(260.0, 260.0));
                state.desktop.user_sized.insert(window.to_string());

                let viewport = UiSize::new(900.0, 760.0);
                let mut document = state.view(viewport);
                document
                    .compute_layout(viewport, &mut ApproxTextMeasurer)
                    .unwrap_or_else(|err| panic!("{window} layout: {err}"));

                for (upper, lower) in rows {
                    let upper_rect = document.node(node_id(&document, upper)).layout().rect;
                    let lower_rect = document.node(node_id(&document, lower)).layout().rect;
                    assert!(
                        upper_rect.bottom() <= lower_rect.y + 0.5,
                        "{window}: {upper} should reserve height before {lower}: upper={upper_rect:?} lower={lower_rect:?}"
                    );
                    assert!(
                        !rects_overlap(upper_rect, lower_rect),
                        "{window}: {upper} overlaps {lower}: upper={upper_rect:?} lower={lower_rect:?}"
                    );
                }
            }
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
            assert!(maybe_node_id(&document, "layout_widgets.dock.panel.panel_a").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.dock.panel.workspace").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.dock.panel.panel_b").is_some());
            assert!(maybe_node_id(&document, "layout.panel_a.scroll_area").is_some());
            assert!(maybe_node_id(&document, "layout.workspace.scroll_area").is_some());
            assert!(maybe_node_id(&document, "layout.panel_b.scroll_area").is_some());
            assert!(maybe_node_id(&document, "layout.workspace.card.one").is_some());
            assert!(maybe_node_id(&document, "layout_widgets.document.tabs").is_none());
            assert!(maybe_node_id(&document, "layout_widgets.dock.drop.left.chip").is_none());
            assert!(maybe_node_id(&document, "layout_widgets.dock.drawers").is_some());
            assert!(
                maybe_node_id(&document, "layout_widgets.dock.drawers.drawer.panel_b").is_some()
            );
            for (name, expected) in [
                ("layout_widgets.dock.panel.panel_a.title.label", "Panel A"),
                (
                    "layout_widgets.dock.panel.workspace.title.label",
                    "Workspace",
                ),
                ("layout_widgets.dock.panel.panel_b.title.label", "Panel B"),
            ] {
                let UiContent::Text(text) = document.node(node_id(&document, name)).content()
                else {
                    panic!("{name} should be a text label");
                };
                assert_eq!(text.text, expected);
            }

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "layout_widgets.drawer.panel_b",
            ));
            assert!(state.layout_dock.is_hidden("panel_b"));
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("drawer layout");
            assert!(maybe_node_id(&document, "layout_widgets.dock.panel.panel_b").is_none());
            assert!(maybe_node_id(&document, "layout_widgets.dock.panel.workspace").is_some());
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
        fn showcase_label_locale_dropdown_has_combobox_affordance() {
            let state = state_with_window("labels");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("labels layout");

            let trigger = document.node(node_id(&document, "labels.locale"));
            assert!(
                trigger.interaction_visuals().is_some(),
                "locale dropdown should publish hover/pressed visuals"
            );

            let label = document
                .node(node_id(&document, "labels.locale.label"))
                .layout()
                .rect;
            let indicator_node = document.node(node_id(&document, "labels.locale.indicator"));
            let indicator = indicator_node.layout().rect;
            assert!(
                label.x < indicator.x,
                "locale dropdown text should be left of the right-side indicator: label={label:?} indicator={indicator:?}"
            );
            assert!(matches!(
                indicator_node.content(),
                UiContent::Text(text) if text.text == "▼"
            ));
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
        fn showcase_numeric_window_fits_editable_value_unit_and_range_controls() {
            let mut state = state_with_window("numeric");
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
            for name in [
                "numeric.value_label",
                "numeric.value",
                "numeric.value.value",
                "numeric.unit",
                "numeric.range_min",
                "numeric.range_min.value",
                "numeric.range_max",
                "numeric.range_max.value",
                "numeric.sensitivity",
                "numeric.note",
            ] {
                let rect = document.node(node_id(&document, name)).layout().rect;
                assert!(
                    rect.right() <= content.right() + 1.0,
                    "{name} should fit inside minimized numeric content horizontally: rect={rect:?} content={content:?}"
                );
                assert!(
                    rect.bottom() <= content.bottom() + 1.0,
                    "{name} should fit inside minimized numeric content vertically: rect={rect:?} content={content:?}"
                );
            }
            assert!(maybe_node_id(&document, "numeric.drag_value").is_none());
            assert!(maybe_node_id(&document, "numeric.drag_angle").is_none());
            assert!(maybe_node_id(&document, "numeric.drag_angle_tau").is_none());
            assert_eq!(
                document
                    .node(node_id(&document, "numeric.value"))
                    .accessibility()
                    .map(|meta| meta.role),
                Some(AccessibilityRole::SpinButton)
            );

            state.focused_text = Some(FocusedTextInput::NumericValue);
            let mut focused = state.view(viewport);
            focused
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("focused numeric layout");
            assert_eq!(
                focused
                    .node(node_id(&focused, "numeric.value"))
                    .accessibility()
                    .map(|meta| meta.role),
                Some(AccessibilityRole::TextBox)
            );
            assert_eq!(
                document
                    .node(node_id(&document, "numeric.range_min.value"))
                    .accessibility()
                    .map(|meta| meta.role),
                Some(AccessibilityRole::TextBox)
            );
        }

        #[test]
        fn showcase_numeric_value_can_be_typed_dragged_and_reformatted_by_unit() {
            let mut state = state_with_window("numeric");

            state.numeric_text.set_text("");
            state.update(WidgetAction::text_edit(
                UiNodeId::root(),
                "numeric.value.edit",
                UiInputEvent::TextInput("88.5".to_string()),
            ));
            assert_eq!(state.numeric_value, 88.5);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "numeric.unit.option.deg",
            ));
            assert_eq!(numeric_unit_id(&state.numeric_unit), "deg");
            assert_eq!(state.numeric_text.text(), "88.5");
            assert_eq!(state.numeric_range_min, 0.0);
            assert_eq!(state.numeric_range_max, 360.0);
            assert_eq!(state.numeric_range_min_text.text(), "0.0");
            assert_eq!(state.numeric_range_max_text.text(), "360.0");

            let target = UiRect::new(0.0, 0.0, 150.0, 30.0);
            state.update(WidgetAction::pointer_edit(
                UiNodeId::root(),
                "numeric.value.drag",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Begin,
                    UiPoint::new(0.0, 12.0),
                    UiPoint::new(150.0, 30.0),
                    target,
                ),
            ));
            state.update(WidgetAction::pointer_edit(
                UiNodeId::root(),
                "numeric.value.drag",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    UiPoint::new(16.0, 12.0),
                    UiPoint::new(150.0, 30.0),
                    target,
                ),
            ));
            assert!(state.numeric_value > 88.5);

            state.numeric_range_max_text.set_text("360");
            state.apply_text_edit(
                FocusedTextInput::NumericRangeMax,
                WidgetTextEdit::new(UiInputEvent::Key {
                    key: KeyCode::Enter,
                    modifiers: KeyModifiers::default(),
                }),
            );
            assert_eq!(state.numeric_range_max, 360.0);

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "numeric.unit.option.turn",
            ));
            assert_eq!(numeric_unit_id(&state.numeric_unit), "turn");
            assert_eq!(state.numeric_range_min, 0.0);
            assert_eq!(state.numeric_range_max, 1.0);
            assert_eq!(state.numeric_range_min_text.text(), "0.000");
            assert_eq!(state.numeric_range_max_text.text(), "1.000");
            assert_eq!(state.numeric_value, 1.0);
            assert_eq!(state.numeric_text.text(), "1.000");
        }

        #[test]
        fn showcase_numeric_text_blurs_when_clicking_elsewhere() {
            let viewport = UiSize::new(860.0, 620.0);
            let mut state = state_with_window("numeric");
            let mut frame_state = HostDocumentFrameState::new();

            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("numeric layout");
            let value_rect = document
                .node(node_id(&document, "numeric.value"))
                .layout()
                .rect;
            let value_point = UiPoint::new(
                value_rect.x + value_rect.width * 0.5,
                value_rect.y + value_rect.height * 0.5,
            );
            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(
                    PointerEventKind::Down(PointerButton::Primary),
                    value_point,
                    1,
                ),
            );
            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(PointerEventKind::Up(PointerButton::Primary), value_point, 2),
            );
            assert_eq!(state.focused_text, Some(FocusedTextInput::NumericValue));
            state
                .numeric_text
                .set_selection(0, state.numeric_text.text().len());

            let mut focused = state.view(viewport);
            focused
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("focused numeric layout");
            let label_rect = focused
                .node(node_id(&focused, "numeric.value_label"))
                .layout()
                .rect;
            let label_point = UiPoint::new(
                label_rect.x + label_rect.width * 0.5,
                label_rect.y + label_rect.height * 0.5,
            );
            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(
                    PointerEventKind::Down(PointerButton::Primary),
                    label_point,
                    3,
                ),
            );

            assert_eq!(state.focused_text, None);
            assert_eq!(state.numeric_text.selection_anchor(), None);
            let mut blurred = state.view(viewport);
            blurred
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("blurred numeric layout");
            assert_eq!(
                blurred
                    .node(node_id(&blurred, "numeric.value"))
                    .accessibility()
                    .map(|meta| meta.role),
                Some(AccessibilityRole::SpinButton)
            );
        }

        #[test]
        fn showcase_menu_controls_align_adornments_and_keep_triggers_single_line() {
            let state = state_with_window("menus");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("menu controls layout");

            let check = document
                .node(node_id(&document, "menus.menu_list.item.3.check"))
                .layout()
                .rect;
            let submenu = document
                .node(node_id(&document, "menus.menu_list.item.4.submenu"))
                .layout()
                .rect;
            let check_center_x = check.x + check.width * 0.5;
            let submenu_center_x = submenu.x + submenu.width * 0.5;
            assert!(
                (check_center_x - submenu_center_x).abs() <= 0.5,
                "menu trailing check and submenu arrow should share an alignment column: check={check:?} submenu={submenu:?}"
            );

            for (button_name, label_name) in [
                ("menus.menu_button", "menus.menu_button.label"),
                (
                    "menus.image_text_menu_button",
                    "menus.image_text_menu_button.label",
                ),
            ] {
                let button = document.node(node_id(&document, button_name)).layout().rect;
                let label = document.node(node_id(&document, label_name));
                let UiContent::Text(text) = label.content() else {
                    panic!("{label_name} should be text");
                };
                assert_eq!(text.style.wrap, TextWrap::None, "{label_name}");
                let label_paint = text_paint_rect(&document, label_name);
                assert!(
                    label_paint.right() <= button.right() - 9.0
                        && label_paint.bottom() <= button.bottom() + 0.5,
                    "menu trigger label should fit on one line inside its button: button={button:?} label={label_paint:?}"
                );
            }
        }

        #[test]
        fn showcase_menu_submenu_stays_attached_to_parent_menu_row() {
            let mut state = state_with_window("menus");
            state.menu_bar.active_item = Some(4);

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("menu controls layout");

            let parent_row = document
                .node(node_id(&document, "menus.menu_list.item.4"))
                .layout()
                .rect;
            let submenu = document
                .node(node_id(&document, "menus.submenu"))
                .layout()
                .rect;
            assert!(
                submenu.x >= parent_row.right() - 0.5
                    && submenu.x <= parent_row.right() + 8.0
                    && submenu.y >= parent_row.y - 0.5
                    && submenu.y <= parent_row.y + 0.5,
                "submenu should be anchored beside the active parent row: row={parent_row:?} submenu={submenu:?}"
            );
        }

        #[test]
        fn showcase_context_menu_opens_near_demo_controls_not_top_left() {
            let mut state = state_with_window("menus");
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "menus.context.open",
            ));

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("menu controls layout");

            assert!(maybe_node_id(&document, "menus.menu_list").is_none());
            let context = document
                .node(node_id(&document, "menus.context_menu"))
                .layout()
                .rect;
            assert!(
                context.x > 20.0 && context.y > 120.0 && context.width >= 220.0,
                "context menu should open near the demo controls with usable width, not as a thin top-left overlay: {context:?}"
            );
        }

        #[test]
        fn showcase_command_palette_opens_near_top_center_with_search_affordance() {
            let viewport = UiSize::new(900.0, 760.0);
            let mut state = state_with_window("command_palette");
            state.last_desktop_size = desktop_size_for_viewport(viewport);
            state.command_palette_open = true;

            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("command palette layout");

            let desktop = desktop_size_for_viewport(viewport);
            let palette = document
                .node(node_id(&document, "command_palette.panel"))
                .layout()
                .rect;
            let palette_center = palette.x + palette.width * 0.5;
            assert!(
                (palette_center - desktop.width * 0.5).abs() <= 1.0,
                "command palette should be centered over the desktop: palette={palette:?} desktop={desktop:?}"
            );
            assert!(
                palette.y >= 48.0 && palette.y <= 96.0,
                "command palette should open near the top of the desktop: {palette:?}"
            );

            let input = document
                .node(node_id(&document, "command_palette.panel.input"))
                .layout()
                .rect;
            let results = document
                .node(node_id(&document, "command_palette.panel.results"))
                .layout()
                .rect;
            let results_id = node_id(&document, "command_palette.panel.results");
            let first_row_id = document.node(results_id).children()[0];
            let first_row = document.node(first_row_id).layout().rect;
            assert!(
                (input.x - results.x).abs() <= 0.5,
                "command palette search and results should stack in one column: input={input:?} results={results:?}"
            );
            assert!(
                results.y >= input.bottom() - 0.5,
                "command palette results should appear below the search field: input={input:?} results={results:?}"
            );
            assert!(
                first_row.x >= results.x - 0.5 && first_row.right() <= results.right() + 0.5,
                "command palette result rows should stay inside the results column: row={first_row:?} results={results:?}"
            );

            assert!(matches!(
                document
                    .node(node_id(&document, "command_palette.panel.search_icon"))
                    .content(),
                UiContent::Image(image) if image.key == "icons.search"
            ));
            let UiContent::Text(placeholder) = document
                .node(node_id(&document, "command_palette.panel.query"))
                .content()
            else {
                panic!("command palette query should be text");
            };
            assert_eq!(placeholder.text, "Search commands");
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
        fn showcase_shader_text_editor_scrollbar_can_be_dragged() {
            let state = state_with_window("shader_lab");
            let viewport = UiSize::new(1400.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("shader lab layout");

            let scroll = node_id(&document, "shader_lab.editor.scroll");
            let scroll_state = document
                .scroll_state(scroll)
                .expect("shader text editor scroll state");
            assert!(
                scroll_state.max_offset().y > 0.0,
                "shader text editor should have a vertical scroll range: {scroll_state:?}"
            );
            let viewport = document
                .node(scroll)
                .layout()
                .rect
                .intersection(document.node(scroll).layout().clip_rect)
                .expect("shader text editor scroll viewport");
            let start = UiPoint::new(viewport.right() - 1.0, viewport.y + 8.0);
            let end = UiPoint::new(viewport.right() - 1.0, viewport.bottom() - 8.0);

            let down = document.handle_input(UiInputEvent::PointerDown(start));
            assert_eq!(down.pressed, Some(scroll));
            assert_eq!(down.consumed_by, Some(scroll));

            let moved = document.handle_input(UiInputEvent::PointerMove(end));
            let scroll_state = document
                .scroll_state(scroll)
                .expect("shader text editor scroll state");
            assert_eq!(moved.scrolled, Some(scroll));
            assert!(
                scroll_state.offset().y > scroll_state.max_offset().y * 0.5,
                "dragging the shader text editor scrollbar should update the editor offset: {scroll_state:?}"
            );
        }

        #[test]
        fn showcase_select_dropdown_popups_are_anchored_to_controls() {
            let mut state = state_with_window("selection");
            state.dropdown.open(&select_options());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("selection layout");

            for (trigger_name, popup_name) in [("selection.dropdown", "selection.dropdown.popup")] {
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
                    (popup.y - trigger.bottom()).abs() <= 0.5,
                    "{popup_name} should touch {trigger_name} without a vertical gap: popup={popup:?} trigger={trigger:?}"
                );
            }
            assert!(maybe_node_id(&document, "combo.toggle").is_none());
            assert!(maybe_node_id(&document, "selection.combo_menu").is_none());
        }

        #[test]
        fn showcase_select_controls_use_one_dropdown_model_and_abstract_options() {
            let state = state_with_window("selection");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("selection layout");

            assert!(maybe_node_id(&document, "selection.combo.anchor").is_none());
            assert!(maybe_node_id(&document, "combo.toggle").is_none());

            for (name, expected) in [
                ("selection.dropdown.label", "Label 2"),
                ("selection.select_menu.option.0.label", "Label 1"),
                ("selection.select_menu.option.1.label", "Label 2"),
                ("selection.select_menu.option.2.label", "Label 3"),
                ("selection.select_menu.option.3.label", "Disabled"),
                ("selection.image_menu.option.0.label", "Label 1"),
                ("selection.image_menu.option.1.label", "Label 2"),
                ("selection.image_menu.option.2.label", "Label 3"),
                ("selection.image_menu.option.3.label", "Disabled"),
            ] {
                let UiContent::Text(text) = document.node(node_id(&document, name)).content()
                else {
                    panic!("{name} should be a text label");
                };
                assert_eq!(text.text, expected, "{name}");
            }

            for name in [
                "selection.image_menu.option.0.image",
                "selection.image_menu.option.1.image",
                "selection.image_menu.option.2.image",
            ] {
                assert!(
                    matches!(
                        document.node(node_id(&document, name)).content(),
                        UiContent::Image(_)
                    ),
                    "{name} should render an image option"
                );
            }

            assert!(
                document
                    .node(node_id(&document, "selection.select_menu.option.3"))
                    .action()
                    .is_none(),
                "disabled select menu option should not emit actions"
            );
        }

        #[test]
        fn showcase_overlay_popup_panel_close_button_label_is_visible() {
            let state = state_with_window("overlays");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("overlay layout");

            let close_label = node_id(&document, "overlays.popup_panel.close.label");
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
        fn showcase_overlay_popup_panel_preview_is_anchored_inside_demo_host() {
            let state = state_with_window("overlays");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("overlay layout");

            let host = node_id(&document, "overlays.popup.host");
            let popup = node_id(&document, "overlays.popup_panel");
            assert_eq!(document.node(popup).parent(), Some(host));

            let host_rect = document.node(host).layout().rect;
            let popup_rect = document.node(popup).layout().rect;
            assert!(
                popup_rect.x >= host_rect.x
                    && popup_rect.y >= host_rect.y
                    && popup_rect.right() <= host_rect.right()
                    && popup_rect.bottom() <= host_rect.bottom(),
                "popup preview should fit inside its anchor host: popup={popup_rect:?} host={host_rect:?}"
            );
        }

        #[test]
        fn showcase_overlay_popup_panel_empty_preview_keeps_text_inside_host_padding() {
            let mut state = state_with_window("overlays");
            state.overlay_popup_open = false;
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("overlay layout");

            let host = node_id(&document, "overlays.popup.host");
            let host_rect = document.node(host).layout().rect;
            let placeholder = node_id(&document, "overlays.popup.empty");
            let text_item = document
                .paint_list()
                .items
                .into_iter()
                .find(|item| item.node == placeholder && matches!(item.kind, PaintKind::Text(_)))
                .expect("placeholder text paint");

            assert!(
                text_item.rect.x >= host_rect.x + 9.0
                    && text_item.rect.y >= host_rect.y + 9.0
                    && text_item.rect.right() <= host_rect.right() - 9.0
                    && text_item.rect.bottom() <= host_rect.bottom() - 9.0,
                "placeholder text should remain inside host padding: text={:?} host={host_rect:?}",
                text_item.rect
            );
        }

        #[test]
        fn showcase_overlay_tooltip_appears_on_hover_and_anchors_to_target() {
            let state = state_with_window("overlays");
            let viewport = UiSize::new(900.0, 760.0);
            let mut hidden = state.view(viewport);
            hidden
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("overlay layout");
            assert!(maybe_node_id(&hidden, "overlays.tooltip_target.tooltip").is_none());

            let target = node_id(&hidden, "overlays.tooltip_target");
            let target_rect = hidden.node(target).layout().rect;
            hidden.set_focus_state(UiFocusState {
                hovered: Some(target),
                pressed: None,
                focused: None,
            });
            let frame = process_document_frame(
                &mut hidden,
                &mut ApproxTextMeasurer,
                HostDocumentFrameRequest::new(
                    viewport,
                    RenderTarget::window("showcase", viewport),
                    HostFrameOutput::new(Default::default()),
                ),
            )
            .expect("overlay hover frame");
            let tooltip = node_id(&hidden, "overlays.tooltip_target.tooltip");
            assert!(matches!(
                hidden.node(tooltip).accessibility().map(|meta| meta.role),
                Some(AccessibilityRole::Tooltip)
            ));
            let tooltip_rect = hidden.node(tooltip).layout().rect;
            assert!(
                tooltip_rect.y >= target_rect.bottom() + 7.0,
                "tooltip should be placed below the hovered target: target={target_rect:?} tooltip={tooltip_rect:?}"
            );
            assert!(
                tooltip_rect.x >= target_rect.x - 1.0
                    && tooltip_rect.x <= target_rect.right() + 1.0,
                "tooltip should stay horizontally anchored to the target: target={target_rect:?} tooltip={tooltip_rect:?}"
            );
            assert!(
                frame
                    .render_request
                    .paint
                    .items
                    .iter()
                    .any(|item| item.node == tooltip),
                "hover tooltip should be present in the rendered frame"
            );
        }

        #[test]
        fn showcase_overlay_placement_preview_keeps_target_visible() {
            let state = state_with_window("overlays");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("overlay layout");

            let scene = document.node(node_id(&document, "overlays.tooltip_rect.scene"));
            let UiContent::Scene(primitives) = scene.content() else {
                panic!("placement preview should be a scene");
            };
            let rects = primitives
                .iter()
                .filter_map(|primitive| match primitive {
                    ScenePrimitive::Rect(rect) => Some(rect.rect),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let tooltip = rects
                .iter()
                .copied()
                .find(|rect| (rect.width - 176.0).abs() < 0.01 && rect.height > 50.0)
                .expect("tooltip preview rect");
            let target = rects
                .iter()
                .copied()
                .find(|rect| (rect.width - 64.0).abs() < 0.01 && rect.height > 20.0)
                .expect("target preview rect");
            assert!(
                tooltip.right() <= target.x,
                "tooltip should fall back beside the target instead of covering it: tooltip={tooltip:?} target={target:?}"
            );
            assert!(
                tooltip.y >= 30.0 && target.y >= 30.0,
                "preview content should not be pinned to the top edge: tooltip={tooltip:?} target={target:?}"
            );

            let labels = primitives
                .iter()
                .filter_map(|primitive| match primitive {
                    ScenePrimitive::Text(text) => Some(text.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(labels.contains(&"Tooltip"), "{labels:?}");
            assert!(labels.contains(&"Target"), "{labels:?}");
        }

        #[test]
        fn showcase_timeline_has_tracks_and_continues_when_wide() {
            let mut state = state_with_window("timeline");
            state
                .desktop
                .sizes
                .insert("timeline".to_string(), UiSize::new(1900.0, 280.0));
            state.desktop.user_sized.insert("timeline".to_string());

            let viewport = UiSize::new(2200.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("timeline layout");

            let viewport_rect = document
                .node(node_id(&document, "timeline.viewport"))
                .layout()
                .rect;
            let ruler_rect = document
                .node(node_id(&document, "timeline.ruler"))
                .layout()
                .rect;
            assert!(
                ruler_rect.width > viewport_rect.width + 300.0,
                "timeline content should continue beyond a wide viewport: ruler={ruler_rect:?} viewport={viewport_rect:?}"
            );

            let tracks = document.node(node_id(&document, "timeline.tracks"));
            let UiContent::Scene(primitives) = tracks.content() else {
                panic!("timeline tracks should be a scene");
            };
            let track_labels = primitives
                .iter()
                .filter_map(|primitive| match primitive {
                    ScenePrimitive::Text(text) => Some(text.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for label in [
                "Video",
                "Audio",
                "Notes",
                "Intro",
                "Music bed",
                "Playhead 18.5s",
            ] {
                assert!(
                    track_labels.contains(&label),
                    "timeline should explain what the ruler corresponds to; missing {label:?} in {track_labels:?}"
                );
            }

            let furthest_tick = primitives
                .iter()
                .filter_map(|primitive| match primitive {
                    ScenePrimitive::Line { from, to, .. } => Some(from.x.max(to.x)),
                    _ => None,
                })
                .fold(0.0_f32, f32::max);
            assert!(
                furthest_tick > viewport_rect.width,
                "timeline grid should extend past the visible viewport when stretched: furthest_tick={furthest_tick} viewport={viewport_rect:?}"
            );
        }

        #[test]
        fn showcase_timeline_horizontal_scrollbar_thumb_drag_updates_scroll() {
            let viewport = UiSize::new(1100.0, 760.0);
            let mut state = state_with_window("timeline");
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("timeline layout");

            let scrollbar = node_id(&document, "timeline.horizontal-scrollbar");
            let thumb = node_id(&document, "timeline.horizontal-scrollbar.thumb");
            assert_eq!(
                document
                    .node(scrollbar)
                    .action()
                    .and_then(|action| action.action_id())
                    .map(|id| id.as_str()),
                Some("timeline.horizontal-scrollbar")
            );

            let track_rect = document.node(scrollbar).layout().rect;
            let thumb_rect = document.node(thumb).layout().rect;
            let start = UiPoint::new(thumb_rect.x + 2.0, thumb_rect.y + thumb_rect.height * 0.5);
            let end = UiPoint::new(track_rect.right() - 2.0, start.y);

            assert_eq!(
                document
                    .handle_input(UiInputEvent::PointerDown(start))
                    .pressed,
                Some(scrollbar)
            );
            state.update(WidgetAction::pointer_edit(
                scrollbar,
                "timeline.horizontal-scrollbar",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Begin,
                    start,
                    UiPoint::new(start.x - track_rect.x, start.y - track_rect.y),
                    track_rect,
                ),
            ));
            state.update(WidgetAction::pointer_edit(
                scrollbar,
                "timeline.horizontal-scrollbar",
                WidgetPointerEdit::new(
                    WidgetValueEditPhase::Commit,
                    end,
                    UiPoint::new(end.x - track_rect.x, end.y - track_rect.y),
                    track_rect,
                ),
            ));

            assert!(
                state.timeline_scroll.offset().x > state.timeline_scroll.max_offset().x * 0.75,
                "dragging the timeline thumb should move near the end: {:?}",
                state.timeline_scroll
            );

            let mut scrolled = state.view(viewport);
            scrolled
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("scrolled timeline layout");
            let viewport_scroll = scrolled
                .scroll_state(node_id(&scrolled, "timeline.viewport"))
                .expect("timeline viewport scroll state");
            assert!(
                viewport_scroll.offset().x > 0.0,
                "timeline viewport should render with dragged scroll offset: {viewport_scroll:?}"
            );
        }

        #[test]
        fn showcase_timeline_horizontal_scrollbar_host_drag_moves_viewport() {
            let viewport = UiSize::new(1100.0, 760.0);
            let mut state = state_with_window("timeline");
            let mut frame_state = HostDocumentFrameState::new();
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("timeline layout");

            let scrollbar = node_id(&document, "timeline.horizontal-scrollbar");
            let thumb = node_id(&document, "timeline.horizontal-scrollbar.thumb");
            let track_rect = document.node(scrollbar).layout().rect;
            let thumb_rect = document.node(thumb).layout().rect;
            let start = UiPoint::new(thumb_rect.x + 2.0, thumb_rect.y + thumb_rect.height * 0.5);
            let end = UiPoint::new(track_rect.right() - 2.0, start.y);

            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(PointerEventKind::Down(PointerButton::Primary), start, 1),
            );
            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(PointerEventKind::Move, end, 16),
            );
            apply_showcase_pointer_frame(
                &mut state,
                &mut frame_state,
                viewport,
                raw_pointer(PointerEventKind::Up(PointerButton::Primary), end, 32),
            );

            assert!(
                state.timeline_scroll.offset().x > state.timeline_scroll.max_offset().x * 0.75,
                "host pointer drag should move the timeline scrollbar near the end: {:?}",
                state.timeline_scroll
            );

            let mut scrolled = state.view(viewport);
            scrolled
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("scrolled timeline layout");
            let viewport_scroll = scrolled
                .scroll_state(node_id(&scrolled, "timeline.viewport"))
                .expect("timeline viewport scroll state");
            let content_rect = scrolled
                .node(node_id(&scrolled, "timeline.content"))
                .layout()
                .rect;
            let viewport_rect = scrolled
                .node(node_id(&scrolled, "timeline.viewport"))
                .layout()
                .rect;
            assert!(
                viewport_scroll.offset().x > 0.0,
                "timeline viewport should receive the scrollbar drag offset: {viewport_scroll:?}"
            );
            assert!(
                content_rect.x < viewport_rect.x - 100.0,
                "timeline content should be translated left after dragging: content={content_rect:?} viewport={viewport_rect:?}"
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
        fn showcase_easing_demo_wires_ease_in_and_ease_out_dropdowns_to_curve_scenes() {
            let mut state = state_with_window("easing");
            state
                .easing_in
                .select_id(&easing_options(EaseDirection::In), "back");
            state
                .easing_out
                .select_id(&easing_options(EaseDirection::Out), "bounce");
            state.progress_phase = 2.0;

            let viewport = UiSize::new(900.0, 900.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            let dropdown_label = text_content(&document, "easing.in.dropdown.label");
            assert_eq!(dropdown_label, "Ease in back");
            let dropdown_label = text_content(&document, "easing.out.dropdown.label");
            assert_eq!(dropdown_label, "Ease out bounce");

            for graph_name in ["easing.in.graph", "easing.out.graph"] {
                let graph = document.node(node_id(&document, graph_name));
                let UiContent::Scene(primitives) = graph.content() else {
                    panic!("{graph_name} should be a scene");
                };
                let line_count = primitives
                    .iter()
                    .filter(|primitive| matches!(primitive, ScenePrimitive::Line { .. }))
                    .count();
                assert!(
                    line_count >= 40,
                    "{graph_name} should plot the selected curve, got {line_count} lines"
                );
                assert!(
                    primitives
                        .iter()
                        .any(|primitive| matches!(primitive, ScenePrimitive::Circle { .. })),
                    "{graph_name} should include a live marker"
                );
            }

            let ease_in_back = selected_easing(&state.easing_in, EaseDirection::In);
            let ease_out_back = EasingFunction::new(EaseDirection::Out, EaseCurveKind::Back);
            let t = 0.25;
            assert!(
                (ease_in_back.sample(t) - (1.0 - ease_out_back.sample(1.0 - t))).abs() < 0.0001
            );

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "easing.in.dropdown.option.expo",
            ));
            assert_eq!(
                selected_easing(&state.easing_in, EaseDirection::In).kind,
                EaseCurveKind::Expo
            );
        }

        #[test]
        fn showcase_interaction_animation_tracks_pointer_input() {
            let mut state = state_with_window("animation");
            state.animation_timed_expanded = false;
            state.animation_scrub_expanded = false;
            state.animation_state_expanded = false;
            state.animation_interaction_expanded = true;
            let viewport = UiSize::new(900.0, 1400.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut CosmicTextMeasurer::new())
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

            let viewport = UiSize::new(900.0, 1400.0);
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
            let section_scroll = document
                .scroll_state(node_id(&document, "animation.section_scroll"))
                .expect("animation section scroll state");
            assert!(window.height >= ANIMATION_CONTENT_MIN_HEIGHT, "{window:?}");
            assert!(
                section.height >= ANIMATION_CONTENT_MIN_HEIGHT,
                "section={section:?}"
            );
            assert_eq!(
                section_scroll.max_offset(),
                UiPoint::new(0.0, 0.0),
                "minimum animation window should fit expanded sections without an internal scrollbar: {section_scroll:?}"
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
        fn showcase_animation_collapsing_sections_are_single_framed_cards() {
            let state = state_with_window("animation");
            let viewport = UiSize::new(900.0, 1120.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");

            for (section, first_child) in [
                ("animation.timed", "animation.live.stage"),
                ("animation.scrub", "animation.scrub.row"),
                ("animation.state", "animation.state.row"),
                ("animation.interaction", "animation.interaction.stage"),
            ] {
                let root = document.node(node_id(&document, section));
                assert!(
                    root.visual().stroke.is_some(),
                    "{section} should render one outer section frame"
                );
                let header = document
                    .node(node_id(&document, &format!("{section}.header")))
                    .layout()
                    .rect;
                let body = document
                    .node(node_id(&document, &format!("{section}.body")))
                    .layout()
                    .rect;
                let child = document.node(node_id(&document, first_child)).layout().rect;
                assert!(
                    (body.y - header.bottom()).abs() <= 0.5,
                    "{section} body should attach directly below header: header={header:?} body={body:?}"
                );
                assert!(
                    child.x >= body.x + 9.0 && child.y >= body.y + 9.0,
                    "{section} content should be padded inside the section body: child={child:?} body={body:?}"
                );
            }
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
            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "slider.thumb_color_button",
            ));
            assert!(state.slider_thumb_picker_open);

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            assert!(document
                .nodes()
                .iter()
                .any(|node| node.name() == "slider.trailing_picker"));
            assert!(document
                .nodes()
                .iter()
                .any(|node| node.name() == "slider.trailing_color_button.label"));
            assert!(document
                .nodes()
                .iter()
                .any(|node| node.name() == "slider.thumb_picker"));
        }

        #[test]
        fn showcase_slider_always_clamping_reformats_out_of_range_text() {
            let mut state = state_with_window("slider");
            state.slider_clamping = widgets::SliderClamping::Always;
            state.slider_value_text.set_text("55555");
            state.apply_slider_value_from_text(55555.0);

            assert_eq!(state.slider, 10000.0);
            assert_eq!(state.slider_value_text.text(), "10000");
        }

        #[test]
        fn showcase_checkbox_demonstrates_state_and_option_variants() {
            let state = state_with_window("checkbox");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("checkbox layout");

            for name in [
                "checkbox.enabled",
                "checkbox.unchecked_sample",
                "checkbox.checked_sample",
                "checkbox.indeterminate_sample",
                "checkbox.disabled",
                "checkbox.large",
                "checkbox.custom_color",
                "checkbox.image_check",
                "checkbox.compact_gap",
                "checkbox.indeterminate",
            ] {
                assert!(maybe_node_id(&document, name).is_some(), "{name}");
            }

            let large_box = document
                .node(node_id(&document, "checkbox.large.box"))
                .layout()
                .rect;
            assert_eq!(large_box.width, 22.0);
            assert_eq!(large_box.height, 22.0);

            let image_check = node_id(&document, "checkbox.image_check.check");
            assert!(matches!(
                document.node(image_check).content(),
                UiContent::Image(image) if image.key == "showcase.operad-logo.png"
            ));
            let image_check_box = document
                .node(node_id(&document, "checkbox.image_check.box"))
                .layout()
                .rect;
            assert_eq!(image_check_box.width, 72.0);
            assert_eq!(image_check_box.height, 72.0);
            let image_check_rect = document.node(image_check).layout().rect;
            assert_eq!(image_check_rect.width, 64.0);
            assert_eq!(image_check_rect.height, 64.0);
            assert_eq!(image_check_rect.x, image_check_box.x + 4.0);
            assert_eq!(image_check_rect.y, image_check_box.y + 4.0);

            let indeterminate = document.node(node_id(&document, "checkbox.indeterminate"));
            assert_eq!(
                indeterminate.accessibility().and_then(|meta| meta.checked),
                Some(AccessibilityChecked::Mixed)
            );
            assert!(maybe_node_id(&document, "checkbox.indeterminate.indeterminate").is_some());

            let disabled = document.node(node_id(&document, "checkbox.disabled"));
            assert_eq!(
                disabled.accessibility().map(|meta| meta.enabled),
                Some(false)
            );
            assert_no_severe_showcase_warnings("checkbox", "options", &document);
        }

        #[test]
        fn showcase_radio_and_toggle_demonstrates_option_variants() {
            let state = state_with_window("toggles");
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("toggles layout");

            for (name, expected) in [
                ("toggles.radio_group.foo.label", "Foo"),
                ("toggles.radio_group.bar.label", "Bar"),
                ("toggles.radio_group.baz.label", "Baz"),
            ] {
                let UiContent::Text(text) = document.node(node_id(&document, name)).content()
                else {
                    panic!("{name} should be a text label");
                };
                assert_eq!(text.text, expected);
            }

            for name in [
                "toggles.radio_custom",
                "toggles.radio_custom.dot",
                "toggles.radio_no_label",
                "toggles.switch_custom",
                "toggles.switch_custom.track",
                "toggles.switch_custom.thumb",
                "toggles.switch_no_label",
                "toggles.switch_disabled",
            ] {
                assert!(maybe_node_id(&document, name).is_some(), "{name}");
            }
            assert!(maybe_node_id(&document, "toggles.radio_no_label.label").is_none());
            assert!(maybe_node_id(&document, "toggles.switch_no_label.label").is_none());

            let selected_outer = document
                .node(node_id(&document, "toggles.radio_group.foo.outer"))
                .layout()
                .rect;
            let selected_dot = document
                .node(node_id(&document, "toggles.radio_group.foo.dot"))
                .layout()
                .rect;
            assert_rect_centers_match(selected_outer, selected_dot);
            assert_circle_paint_centered_on_rect(
                &document,
                selected_outer,
                node_id(&document, "toggles.radio_group.foo.dot"),
            );

            let custom_outer = document
                .node(node_id(&document, "toggles.radio_custom.outer"))
                .layout()
                .rect;
            let custom_dot = document
                .node(node_id(&document, "toggles.radio_custom.dot"))
                .layout()
                .rect;
            assert_rect_centers_match(custom_outer, custom_dot);
            assert_circle_paint_centered_on_rect(
                &document,
                custom_outer,
                node_id(&document, "toggles.radio_custom.dot"),
            );

            let radio_outer = document
                .node(node_id(&document, "toggles.radio_no_label.outer"))
                .layout()
                .rect;
            assert_eq!(radio_outer.width, 24.0);
            assert_eq!(radio_outer.height, 24.0);

            let switch_track = document
                .node(node_id(&document, "toggles.switch_custom.track"))
                .layout()
                .rect;
            assert_eq!(switch_track.width, 74.0);
            assert_eq!(switch_track.height, 24.0);
            let switch_thumb = document
                .node(node_id(&document, "toggles.switch_custom.thumb"))
                .layout()
                .rect;
            assert_eq!(switch_thumb.width, 28.0);
            assert_eq!(switch_thumb.height, 18.0);

            assert_eq!(
                document
                    .node(node_id(&document, "toggles.switch_disabled"))
                    .accessibility()
                    .map(|meta| meta.enabled),
                Some(false)
            );
            assert_no_severe_showcase_warnings("toggles", "options", &document);
        }

        #[test]
        #[allow(clippy::field_reassign_with_default)]
        fn showcase_progress_phase_does_not_wrap_on_tick() {
            let mut state = ShowcaseState::default();
            state.progress_phase = std::f32::consts::TAU - 0.001;
            state.update(WidgetAction::activate(UiNodeId::root(), "runtime.tick"));
            assert!(state.progress_phase > std::f32::consts::TAU);
        }

        #[test]
        fn showcase_progress_includes_logged_loading_panel() {
            let mut state = state_with_window("progress");
            state.progress_phase = 10.0;
            state.progress_loading_elapsed = PROGRESS_LOGGED_DURATION_SECONDS;
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("progress layout");

            let progress = node_id(&document, "progress.logged.progress");
            let reset = node_id(&document, "progress.logged.trailing_action");
            let logs = node_id(&document, "progress.logged.logs");
            assert!(
                document.node(progress).layout().rect.y < document.node(logs).layout().rect.y,
                "logged progress bar should be above logs"
            );
            assert!(
                document.node(reset).layout().rect.x > document.node(progress).layout().rect.x,
                "reset button should sit beside the logged progress bar"
            );
            assert!(document.node(logs).has_auto_scrollbar());
            let scroll = document.scroll_state(logs).expect("log scroll state");
            assert!(
                (scroll.offset().y - scroll.max_offset().y).abs() < 0.01,
                "log panel should follow the newest entry while it is at the tail"
            );
            assert_eq!(
                document.node(logs).accessibility().map(|meta| meta.role),
                Some(AccessibilityRole::List)
            );
            assert!(maybe_node_id(&document, "progress.logged.logs.row.0").is_some());
            assert!(maybe_node_id(&document, "progress.logged.logs.row.5").is_some());
            assert_no_severe_showcase_warnings("progress", "logged loading panel", &document);
        }

        #[test]
        fn showcase_progress_log_scroll_preserves_manual_position() {
            let mut state = state_with_window("progress");
            state.progress_loading_elapsed = PROGRESS_LOGGED_DURATION_SECONDS;
            state.progress_logs_follow_tail = false;
            state.progress_logs_scroll = progress_log_scroll_state(13.0, 6, false);
            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("progress layout");

            let logs = node_id(&document, "progress.logged.logs");
            let scroll = document.scroll_state(logs).expect("log scroll state");
            assert!(
                (scroll.offset().y - 13.0).abs() < 0.01,
                "log panel should preserve a manual upward scroll offset"
            );
            assert!(
                scroll.max_offset().y > scroll.offset().y,
                "full log panel should still have newer entries below the preserved position"
            );
        }

        #[test]
        fn showcase_progress_reset_restarts_logged_loading_timeline() {
            let mut state = state_with_window("progress");
            state.progress_loading_elapsed = PROGRESS_LOGGED_DURATION_SECONDS;
            state.progress_logs_follow_tail = false;

            state.update(WidgetAction::activate(
                UiNodeId::root(),
                "progress.logged.reset",
            ));
            assert_eq!(state.progress_loading_elapsed, 0.0);
            assert!(state.progress_logs_follow_tail);

            state.update(WidgetAction::activate(UiNodeId::root(), "runtime.tick"));
            assert!(state.progress_loading_elapsed > 0.0);
            assert!(state.progress_loading_elapsed < 0.1);
        }
    }
}
