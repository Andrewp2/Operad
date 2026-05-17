#![cfg(feature = "widgets")]

use std::hint::black_box;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[cfg(feature = "wgpu")]
use operad::platform::{ImageHandle, PixelSize, ResourceHandle};
use operad::widgets::*;
use operad::*;

const PERF_VIEWPORT: UiSize = UiSize::new(960.0, 540.0);
#[cfg(feature = "wgpu")]
const FRAME_PERCENTILE: f64 = 95.0;
#[cfg(all(feature = "wgpu", debug_assertions))]
const NO_READBACK_TEXT_RENDER_FRAME_P95_BUDGET: Duration = Duration::from_millis(4);
#[cfg(all(feature = "wgpu", not(debug_assertions)))]
const NO_READBACK_TEXT_RENDER_FRAME_P95_BUDGET: Duration = Duration::from_millis(1);

fn paint_list_fingerprint(paint: &PaintList) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for item in &paint.items {
        for value in [
            item.node.0 as u64,
            item.rect.x.to_bits() as u64,
            item.rect.y.to_bits() as u64,
            item.rect.width.to_bits() as u64,
            item.rect.height.to_bits() as u64,
            paint_kind_code(&item.kind),
        ] {
            hash ^= value;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

fn paint_kind_code(kind: &PaintKind) -> u64 {
    match kind {
        PaintKind::Rect { .. } => 1,
        PaintKind::Text(_) => 2,
        PaintKind::Canvas(_) => 3,
        PaintKind::Image { .. } => 4,
        PaintKind::Line { .. } => 5,
        PaintKind::Circle { .. } => 6,
        PaintKind::Polygon { .. } => 7,
        PaintKind::SceneText(_) => 8,
        PaintKind::Path(_) => 9,
        PaintKind::ImagePlacement(_) => 10,
        PaintKind::RichRect(_) => 11,
        PaintKind::CompositedLayer(_) => 12,
    }
}

#[test]
fn virtualized_table_layout_and_paint_smoke_stays_under_budget() {
    let _perf_guard = perf_test_lock();
    let mut perf = PerformanceSamples::new("virtualized table render smoke");
    let mut combined_hash = 0_u64;

    for frame in 0..12 {
        let started = Instant::now();
        let mut document = perf_screen();
        let root = document.root;
        virtualized_data_table(
            &mut document,
            root,
            "perf.table",
            &[
                DataTableColumn::new("name", "Name", 220.0),
                DataTableColumn::new("bar", "Bar", 72.0).with_alignment(DataCellAlignment::End),
                DataTableColumn::new("length", "Length", 96.0)
                    .with_alignment(DataCellAlignment::End),
                DataTableColumn::new("status", "Status", 140.0),
                DataTableColumn::new("notes", "Notes", 260.0),
            ],
            VirtualDataTableSpec {
                row_count: 100_000,
                row_height: 22.0,
                viewport_width: 840.0,
                viewport_height: 420.0,
                scroll_offset: UiPoint::new(0.0, frame as f32 * 137.0),
                overscan_rows: 3,
            },
            DataTableOptions {
                selection: DataTableSelection::single_row(40 + frame)
                    .with_active_cell(DataTableCellIndex::new(40 + frame, 3)),
                ..Default::default()
            },
            |document, parent, cell| {
                let text = match cell.column {
                    0 => format!("Clip {:05}", cell.row),
                    1 => format!("{}", cell.row % 256),
                    2 => format!("{} ms", 120 + cell.row % 900),
                    3 => {
                        if cell.row % 3 == 0 {
                            "rendered".to_string()
                        } else {
                            "pending".to_string()
                        }
                    }
                    _ => format!("automation lane {}", cell.row % 8),
                };
                document.add_child(
                    parent,
                    UiNode::text(
                        format!("perf.cell.{}.{}", cell.row, cell.column),
                        text,
                        TextStyle {
                            font_size: 11.0,
                            line_height: 15.0,
                            color: ColorRgba::new(226, 232, 241, 255),
                            ..Default::default()
                        },
                        layout::size(layout::auto(), layout::auto()),
                    ),
                );
            },
        );

        document
            .compute_layout(PERF_VIEWPORT, &mut ApproxTextMeasurer)
            .expect("layout");
        let paint = document.paint_list();
        combined_hash ^= paint_list_fingerprint(&paint);
        black_box(document.node_count());
        perf.push(started.elapsed());
    }

    assert_ne!(combined_hash, 0);
    let budget = PerformanceAssertions::new(&perf);
    budget.require_sample_count(12).expect("sample count");
    budget
        .require_total_within(Duration::from_secs(5))
        .expect("total budget");
    budget
        .require_average_within(Duration::from_millis(500))
        .expect("average budget");
}

#[test]
fn command_palette_filter_build_and_paint_stays_under_budget() {
    let _perf_guard = perf_test_lock();
    let items = (0..5_000)
        .map(|index| {
            CommandPaletteItem::new(format!("cmd.{index}"), format!("Transform clip {index}"))
                .subtitle("Batch command")
                .keyword("transform")
        })
        .collect::<Vec<_>>();
    let mut state = CommandPaletteState::new().with_query("trans 42");
    state.move_active(&items, NavigationDirection::Next);

    let mut perf = PerformanceSamples::new("command palette build and paint smoke");
    for _ in 0..20 {
        let started = Instant::now();
        let mut document = perf_screen();
        let root = document.root;
        command_palette(
            &mut document,
            root,
            "perf.palette",
            &items,
            &state,
            None,
            CommandPaletteOptions {
                width: 620.0,
                max_visible_rows: 12,
                ..Default::default()
            },
        );
        document
            .compute_layout(PERF_VIEWPORT, &mut ApproxTextMeasurer)
            .expect("layout");
        black_box(document.paint_list().items.len());
        perf.push(started.elapsed());
    }

    let budget = PerformanceAssertions::new(&perf);
    budget.require_sample_count(20).expect("sample count");
    budget
        .require_total_within(Duration::from_secs(3))
        .expect("total budget");
    budget
        .require_average_within(Duration::from_millis(150))
        .expect("average budget");
}

#[test]
fn interaction_heavy_frame_build_and_paint_stays_under_budget() {
    let _perf_guard = perf_test_lock();
    let mut perf = PerformanceSamples::new("interaction-heavy frame build and paint smoke");
    let mut interaction_count = 0_usize;
    let mut painted_count = 0_usize;

    for frame in 0..16 {
        let started = Instant::now();
        let mut document = perf_screen();
        let root = document.root;
        let toolbar = document.add_child(
            root,
            UiNode::container(
                "perf.interactions.toolbar",
                UiNodeStyle::from(layout::with_size(
                    layout::row(),
                    layout::px(920.0),
                    layout::px(44.0),
                )),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(17, 24, 32, 255),
                Some(StrokeStyle::new(ColorRgba::new(45, 57, 70, 255), 1.0)),
                0.0,
            )),
        );

        for index in 0..24 {
            button(
                &mut document,
                toolbar,
                format!("perf.interactions.tool.{index}"),
                format!("T{index:02}"),
                ButtonOptions {
                    layout: fixed_style(36.0, 30.0),
                    pressed: index == frame % 24,
                    focused: index == (frame + 2) % 24,
                    text_style: TextStyle {
                        font_size: 10.0,
                        line_height: 13.0,
                        color: ColorRgba::new(235, 240, 247, 255),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
        }

        let list = scroll_area(
            &mut document,
            root,
            "perf.interactions.list",
            ScrollAxes::VERTICAL,
            layout::with_size(layout::column(), layout::px(920.0), layout::px(470.0)),
        );
        for row in 0..180 {
            let active = row == frame * 7 % 180;
            let row_node = document.add_child(
                list,
                UiNode::container(
                    format!("perf.interactions.row.{row}"),
                    UiNodeStyle::from(layout::with_size(
                        layout::row(),
                        layout::px(900.0),
                        layout::px(20.0),
                    )),
                )
                .with_input(InputBehavior::BUTTON)
                .with_visual(UiVisual::panel(
                    if active {
                        ColorRgba::new(47, 83, 103, 255)
                    } else if row % 2 == 0 {
                        ColorRgba::new(14, 20, 28, 255)
                    } else {
                        ColorRgba::new(10, 16, 23, 255)
                    },
                    Some(StrokeStyle::new(ColorRgba::new(27, 37, 48, 255), 1.0)),
                    0.0,
                )),
            );
            if active || row % 11 == frame % 11 || row % 17 == frame % 17 {
                interaction_count += 1;
            }
            document.add_child(
                row_node,
                UiNode::text(
                    format!("perf.interactions.row.{row}.label"),
                    format!(
                        "route row {row:03} hover={} press={} focus={}",
                        row % 11 == frame % 11,
                        active,
                        row % 17 == frame % 17
                    ),
                    TextStyle {
                        font_size: 10.0,
                        line_height: 13.0,
                        color: ColorRgba::new(224, 231, 240, 255),
                        ..Default::default()
                    },
                    layout::size(layout::auto(), layout::auto()),
                ),
            );
        }

        document
            .compute_layout(PERF_VIEWPORT, &mut ApproxTextMeasurer)
            .expect("layout");
        let paint = document.paint_list();
        painted_count += paint.items.len();
        black_box(paint.items.len());
        perf.push(started.elapsed());
    }

    assert!(interaction_count > 400);
    assert!(painted_count > 900);
    let budget = PerformanceAssertions::new(&perf);
    budget.require_sample_count(16).expect("sample count");
    budget
        .require_total_within(Duration::from_secs(4))
        .expect("total budget");
    budget
        .require_average_within(Duration::from_millis(250))
        .expect("average budget");
}

#[test]
fn retained_display_list_reuse_smoke_reports_expected_hit_rate() {
    let _perf_guard = perf_test_lock();
    let mut cache = RetainedDisplayListCache::new();
    let key = DisplayListKey::editor_background("perf.static-grid", 1);
    let mut series = DisplayListReuseSeries::new("retained display-list reuse smoke");
    let mut invalidated = 0_usize;

    for frame in 0..12 {
        cache.advance_frame();
        let dirty = if frame == 6 {
            DirtyFlags {
                paint: true,
                ..DirtyFlags::NONE
            }
        } else if frame == 0 {
            DirtyFlags::NONE
        } else {
            DirtyFlags {
                input: true,
                ..DirtyFlags::NONE
            }
        };
        let report = cache.reuse_report(&key, dirty);
        let missed = report.missed();
        series.push(report);
        if dirty.paint {
            invalidated += cache
                .invalidate_with_report(DisplayListInvalidationRequest::Dirty(dirty))
                .removed_count();
        }
        if missed {
            cache.insert(
                key.clone(),
                DisplayListKind::StaticBackground,
                DisplayListInvalidation::STATIC_EDITOR_BACKGROUND,
                retained_panel_paint(32),
            );
        }
        black_box(cache.len());
    }

    assert_eq!(invalidated, 1);
    let assertions = DisplayListReuseSeriesAssertions::new(&series);
    assertions.require_report_count(12).expect("report count");
    assertions
        .require_key_outcome_count(&key, DisplayListReuseOutcome::MissAbsent, 1)
        .expect("initial miss");
    assertions
        .require_key_outcome_count(&key, DisplayListReuseOutcome::MissDirty, 1)
        .expect("dirty miss");
    assertions
        .require_key_outcome_count(&key, DisplayListReuseOutcome::Reused, 10)
        .expect("reuse count");
    assertions.require_no_evictions().expect("no evictions");
    assertions
        .require_reuse_rate_at_least(0.8)
        .expect("reuse rate");
}

#[test]
fn retained_display_list_large_scene_reuse_survives_interaction_churn() {
    let _perf_guard = perf_test_lock();
    let mut cache = RetainedDisplayListCache::new();
    let keys = (0..96)
        .map(|index| DisplayListKey::editor_background(format!("perf.large.layer.{index}"), 5))
        .collect::<Vec<_>>();
    let mut series = DisplayListReuseSeries::new("large retained display-list reuse smoke");
    let mut inserted_items = 0_usize;
    let mut invalidated = 0_usize;

    for frame in 0..18 {
        cache.advance_frame();
        for (index, key) in keys.iter().enumerate() {
            let dirty = if frame == 9 && index % 12 == 0 {
                DirtyFlags {
                    paint: true,
                    ..DirtyFlags::NONE
                }
            } else {
                DirtyFlags {
                    input: true,
                    ..DirtyFlags::NONE
                }
            };
            let report = cache.reuse_report(key, dirty);
            let missed = report.missed();
            series.push(report);
            if missed {
                let item_count = 96 + index % 32;
                inserted_items += item_count;
                cache.insert(
                    key.clone(),
                    DisplayListKind::StaticBackground,
                    DisplayListInvalidation::STATIC_EDITOR_BACKGROUND,
                    retained_panel_paint(item_count),
                );
            }
        }

        if frame == 9 {
            invalidated += cache
                .invalidate_with_report(DisplayListInvalidationRequest::Dirty(DirtyFlags {
                    paint: true,
                    ..DirtyFlags::NONE
                }))
                .removed_count();
        }
        black_box(cache.len());
    }

    assert_eq!(invalidated, 96);
    assert!(inserted_items > 18_000);
    let assertions = DisplayListReuseSeriesAssertions::new(&series);
    assertions
        .require_report_count(96 * 18)
        .expect("report count");
    assertions.require_no_evictions().expect("no evictions");
    assertions
        .require_reuse_rate_at_least(0.85)
        .expect("reuse rate");
}

#[test]
fn editor_geometry_scene_build_and_paint_smoke_stays_under_budget() {
    let _perf_guard = perf_test_lock();
    let mut perf = PerformanceSamples::new("editor geometry render smoke");
    let mut combined_hash = 0_u64;
    let mut combined_hits = 0_usize;

    for frame in 0..10 {
        let started = Instant::now();
        let mut document = perf_screen();
        let root = document.root;
        let transform = EditorTransform::new(UiRect::new(0.0, 0.0, 900.0, 500.0))
            .with_scale(UiPoint::new(12.0, 1.0));
        let arrangement = LaneTimelineGeometry::new(
            transform,
            LaneGeometry::new(18.0, 16)
                .with_origin_y(24.0)
                .with_lane_gap(4.0),
        );
        let range_geometry =
            TimelineRangeItemGeometry::new(arrangement).with_resize_handle_width_px(4.0);
        let curve_geometry = CurveEditorGeometry::new(
            arrangement.timeline,
            EditorAxisRange::new(0.0, 1.0),
            UiRect::new(0.0, 398.0, 900.0, 72.0),
        )
        .with_point_radius_px(3.0);
        let mut scene = Vec::with_capacity(640);

        scene.push(ScenePrimitive::Rect(PaintRect::solid(
            UiRect::new(0.0, 0.0, 900.0, 500.0),
            ColorRgba::new(9, 13, 18, 255),
        )));
        for lane in 0..16 {
            if let Some(rect) = arrangement.view_range_rect(lane, EditorAxisRange::new(0.0, 75.0)) {
                scene.push(ScenePrimitive::Rect(PaintRect::solid(
                    rect,
                    if lane % 2 == 0 {
                        ColorRgba::new(13, 19, 27, 255)
                    } else {
                        ColorRgba::new(16, 23, 31, 255)
                    },
                )));
            }
        }
        for unit in (0..=75).step_by(5) {
            let x = arrangement.timeline.unit_to_view_x(unit as f32);
            scene.push(ScenePrimitive::Line {
                from: UiPoint::new(x, 20.0),
                to: UiPoint::new(x, 382.0),
                stroke: StrokeStyle::new(ColorRgba::new(34, 46, 60, 255), 1.0),
            });
        }

        for index in 0..180 {
            let lane = index % 16;
            let start = ((index * 7 + frame * 3) % 68) as f32;
            let duration = 2.0 + (index % 7) as f32 * 0.35;
            let item = TimelineRangeItem::new(format!("range.{index}"), lane, start, duration)
                .selected(index % 19 == 0)
                .dragging(index % 37 == frame % 10);
            combined_hits += range_geometry.hit_targets(&item).len();
            let Some(rect) = range_geometry.item_view_rect(&item) else {
                continue;
            };
            scene.push(ScenePrimitive::Rect(
                PaintRect::solid(
                    rect,
                    if item.selected {
                        ColorRgba::new(86, 157, 190, 255)
                    } else if item.dragging {
                        ColorRgba::new(136, 113, 197, 255)
                    } else {
                        ColorRgba::new(58, 104, 136, 255)
                    },
                )
                .corner_radii(CornerRadii::uniform(3.0)),
            ));
        }

        let curve_points = (0..96)
            .map(|index| {
                let unit = index as f32 * 0.75;
                let value = ((index * 13 + frame * 5) % 100) as f32 / 100.0;
                CurvePoint::new(format!("curve.{index}"), unit, value)
            })
            .collect::<Vec<_>>();
        for segment in curve_geometry.segment_view_points(&curve_points) {
            scene.push(ScenePrimitive::Line {
                from: segment.from,
                to: segment.to,
                stroke: StrokeStyle::new(ColorRgba::new(230, 184, 88, 255), 1.0),
            });
        }
        for point in curve_points.iter().step_by(4) {
            scene.push(ScenePrimitive::Circle {
                center: curve_geometry.point_view_position(point),
                radius: 2.0,
                fill: ColorRgba::new(241, 207, 121, 255),
                stroke: None,
            });
        }

        document.add_child(
            root,
            UiNode::scene("perf.editor", scene, fixed_style(920.0, 500.0)),
        );
        document
            .compute_layout(PERF_VIEWPORT, &mut ApproxTextMeasurer)
            .expect("layout");
        let paint = document.paint_list();
        combined_hash ^= paint_list_fingerprint(&paint);
        black_box(paint.items.len());
        perf.push(started.elapsed());
    }

    assert_ne!(combined_hash, 0);
    assert!(combined_hits > 1_000);
    let budget = PerformanceAssertions::new(&perf);
    budget.require_sample_count(10).expect("sample count");
    budget
        .require_total_within(Duration::from_secs(5))
        .expect("total budget");
    budget
        .require_average_within(Duration::from_millis(500))
        .expect("average budget");
}

#[cfg(feature = "wgpu")]
#[test]
fn wgpu_large_resource_window_request_enumerates_no_readback_path_without_adapter() {
    let request = wgpu_large_resource_perf_request(3);

    assert_eq!(request.target.kind(), RenderTargetKind::Window);
    assert_eq!(request.resource_updates.len(), 3);
    assert_eq!(request.paint.items.len(), 1 + 3 * 48);
    assert!(request
        .resource_updates
        .iter()
        .all(|update| !update.is_partial()));
    assert!(request
        .resource_updates
        .iter()
        .all(ResourceUpdate::has_expected_byte_len));
    assert!(
        request
            .paint
            .items
            .iter()
            .any(|item| matches!(item.kind, PaintKind::Image { .. })),
        "large resource path should include texture-backed image paint items"
    );
}

#[cfg(feature = "wgpu")]
#[test]
fn scenario_harness_multi_frame_render_smoke_stays_under_budget() {
    let _perf_guard = perf_test_lock();
    let mut harness = ScenarioHarness::new(PERF_VIEWPORT)
        .target(RenderTarget::window("perf.scenario", PERF_VIEWPORT));
    let mut timings = FrameTimingSeries::new("scenario harness render smoke");
    let mut combined_hash = 0_u64;
    let mut measurer = ApproxTextMeasurer;
    let mut renderer = WgpuRenderer::default();
    renderer.warm_up().expect("wgpu renderer warm-up");
    for warmup_frame in 0..3 {
        let mut document = scenario_perf_document(warmup_frame);
        harness
            .run_frame_with_measurer_and_renderer(
                format!("scenario-warmup-{warmup_frame}"),
                &mut document,
                EventReplay::new(),
                &mut measurer,
                &mut renderer,
                &EmptyResourceResolver,
            )
            .expect("scenario warm-up frame");
    }

    for frame in 0..8 {
        let mut document = scenario_perf_document(frame);
        let report = harness
            .run_frame_with_measurer_and_renderer(
                format!("scenario-frame-{frame}"),
                &mut document,
                EventReplay::new().pointer_click("activate", UiPoint::new(32.0, 20.0)),
                &mut measurer,
                &mut renderer,
                &EmptyResourceResolver,
            )
            .expect("scenario frame");

        report
            .timing_assertions()
            .require_sections([
                "pre-input-layout",
                "input",
                "document-frame",
                "render-frame",
                "platform-requests",
            ])
            .expect("scenario timing sections");
        report
            .render_assertions()
            .require_min_painted_items(70)
            .expect("painted items");
        assert!(
            report.render.snapshot.is_none(),
            "window-target perf test must not use snapshot readback"
        );
        combined_hash ^= (report.render.painted_items as u64)
            .wrapping_mul(0x9E3779B97F4A7C15_u64)
            .rotate_left((frame % 64) as u32)
            ^ (frame as u64).wrapping_mul(0x9E3779B97F4A7C15_u64);
        timings.push(report.timings.clone());
    }

    assert_ne!(combined_hash, 0);
    let assertions = FrameTimingSeriesAssertions::new(&timings);
    assertions.require_frame_count(8).expect("frame count");
    for section in [
        "pre-input-layout",
        "input",
        "document-frame",
        "render-frame",
        "platform-requests",
    ] {
        assertions
            .require_section_sample_count(section, 8)
            .expect("section sample count");
    }
    assertions
        .require_total_average_within(Duration::from_millis(500))
        .expect("total average budget");
    assertions
        .require_total_max_within(Duration::from_secs(2))
        .expect("total max budget");
    assertions
        .require_total_percentile_within(95.0, Duration::from_secs(2))
        .expect("total percentile budget");
    assertions
        .require_section_average_within("render-frame", Duration::from_millis(250))
        .expect("render average budget");
    assertions
        .require_section_percentile_within(
            "render-frame",
            FRAME_PERCENTILE,
            NO_READBACK_TEXT_RENDER_FRAME_P95_BUDGET,
        )
        .expect("render percentile budget");
}

fn perf_screen() -> UiDocument {
    let mut document = UiDocument::new(root_style(PERF_VIEWPORT.width, PERF_VIEWPORT.height));
    let root = document.root;
    document.node_mut(root).visual = UiVisual::panel(
        ColorRgba::new(9, 12, 16, 255),
        Some(StrokeStyle::new(ColorRgba::new(34, 44, 56, 255), 1.0)),
        0.0,
    );
    document
}

#[cfg(feature = "wgpu")]
fn scenario_perf_document(frame: usize) -> UiDocument {
    let mut document = perf_screen();
    let root = document.root;
    let toolbar = document.add_child(
        root,
        UiNode::container(
            "perf.scenario.toolbar",
            UiNodeStyle::from(layout::with_size(
                layout::row(),
                layout::px(920.0),
                layout::px(42.0),
            )),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(15, 20, 28, 255),
            Some(StrokeStyle::new(ColorRgba::new(46, 58, 72, 255), 1.0)),
            0.0,
        )),
    );

    for index in 0..6 {
        button(
            &mut document,
            toolbar,
            format!("perf.scenario.button.{index}"),
            format!("Tool {index}"),
            ButtonOptions {
                layout: fixed_style(96.0, 32.0),
                text_style: TextStyle {
                    font_size: 11.0,
                    line_height: 15.0,
                    color: ColorRgba::new(232, 238, 246, 255),
                    ..Default::default()
                },
                pressed: index == frame % 6,
                ..Default::default()
            },
        );
    }

    let scroll = scroll_area(
        &mut document,
        root,
        "perf.scenario.scroll",
        ScrollAxes::VERTICAL,
        layout::with_size(layout::column(), layout::px(920.0), layout::px(460.0)),
    );

    for row in 0..64 {
        let selected = row == frame * 3 % 64;
        let row_node = document.add_child(
            scroll,
            UiNode::container(
                format!("perf.scenario.row.{row}"),
                UiNodeStyle::from(layout::with_size(
                    layout::row(),
                    layout::px(900.0),
                    layout::px(24.0),
                )),
            )
            .with_input(InputBehavior::BUTTON)
            .with_visual(UiVisual::panel(
                if selected {
                    ColorRgba::new(45, 75, 98, 255)
                } else if row % 2 == 0 {
                    ColorRgba::new(17, 23, 31, 255)
                } else {
                    ColorRgba::new(12, 18, 25, 255)
                },
                Some(StrokeStyle::new(ColorRgba::new(31, 41, 53, 255), 1.0)),
                0.0,
            )),
        );
        document.add_child(
            row_node,
            UiNode::text(
                format!("perf.scenario.row.{row}.label"),
                format!("Scenario row {row:02}    reusable toolkit surface"),
                TextStyle {
                    font_size: 11.0,
                    line_height: 15.0,
                    color: ColorRgba::new(222, 229, 238, 255),
                    ..Default::default()
                },
                layout::size(layout::auto(), layout::auto()),
            ),
        );
    }

    document
}

#[cfg(feature = "wgpu")]
#[test]
fn wgpu_text_cache_window_render_stays_under_budget_without_readback() {
    let _perf_guard = perf_test_lock();
    let mut renderer = WgpuRenderer::default();
    renderer.warm_up().expect("wgpu renderer warm-up");
    for frame in 0..4 {
        renderer
            .render_frame(wgpu_text_cache_perf_request(frame), &EmptyResourceResolver)
            .expect("wgpu text cache warm-up frame");
    }

    let mut samples = PerformanceSamples::new("wgpu text cache no-readback render");
    let mut combined_hash = 0_u64;
    for frame in 0..12 {
        let output = renderer
            .render_frame(wgpu_text_cache_perf_request(frame), &EmptyResourceResolver)
            .expect("wgpu text cache frame");
        assert!(
            output.snapshot.is_none(),
            "window-target perf test must not use snapshot readback"
        );
        let render_duration = output
            .timings
            .duration("render")
            .expect("renderer timing includes render section");
        samples.push(render_duration);
        combined_hash ^= (output.painted_items as u64)
            .wrapping_mul(0x517cc1b727220a95)
            .rotate_left((frame % 64) as u32);
    }

    assert_ne!(combined_hash, 0);
    let assertions = PerformanceAssertions::new(&samples);
    assertions.require_sample_count(12).expect("sample count");
    assertions
        .require_percentile_within(FRAME_PERCENTILE, NO_READBACK_TEXT_RENDER_FRAME_P95_BUDGET)
        .expect("render percentile budget");
}

#[cfg(feature = "wgpu")]
#[test]
fn wgpu_mixed_changing_ui_window_render_stays_under_budget_without_readback() {
    let _perf_guard = perf_test_lock();
    let mut renderer = WgpuRenderer::default();
    renderer.warm_up().expect("wgpu renderer warm-up");
    for frame in 0..4 {
        renderer
            .render_frame(wgpu_mixed_ui_perf_request(frame), &EmptyResourceResolver)
            .expect("wgpu mixed UI warm-up frame");
    }

    let mut samples = PerformanceSamples::new("wgpu mixed changing UI no-readback render");
    let mut combined_hash = 0_u64;
    for frame in 0..12 {
        let output = renderer
            .render_frame(wgpu_mixed_ui_perf_request(frame), &EmptyResourceResolver)
            .expect("wgpu mixed UI frame");
        assert!(
            output.snapshot.is_none(),
            "window-target perf test must not use snapshot readback"
        );
        let render_duration = output
            .timings
            .duration("render")
            .expect("renderer timing includes render section");
        samples.push(render_duration);
        combined_hash ^= (output.painted_items as u64)
            .wrapping_mul(0x94d049bb133111eb)
            .rotate_left((frame % 64) as u32)
            ^ (frame as u64);
    }

    assert_ne!(combined_hash, 0);
    let assertions = PerformanceAssertions::new(&samples);
    assertions.require_sample_count(12).expect("sample count");
    assertions
        .require_percentile_within(FRAME_PERCENTILE, NO_READBACK_TEXT_RENDER_FRAME_P95_BUDGET)
        .expect("render percentile budget");
}

#[cfg(all(feature = "wgpu", not(debug_assertions)))]
#[test]
fn wgpu_mixed_changing_ui_gpu_render_pass_stays_under_budget_when_timestamps_available() {
    let _perf_guard = perf_test_lock();
    let mut renderer = WgpuRenderer::default();
    renderer.warm_up().expect("wgpu renderer warm-up");
    for frame in 0..4 {
        renderer
            .render_frame(
                wgpu_mixed_ui_perf_request(frame).options(RenderOptions {
                    collect_gpu_timing: true,
                    ..RenderOptions::default()
                }),
                &EmptyResourceResolver,
            )
            .expect("wgpu mixed UI GPU timing warm-up frame");
    }

    let mut samples = PerformanceSamples::new("wgpu mixed changing UI GPU render pass");
    for frame in 0..12 {
        let output = renderer
            .render_frame(
                wgpu_mixed_ui_perf_request(frame).options(RenderOptions {
                    collect_gpu_timing: true,
                    ..RenderOptions::default()
                }),
                &EmptyResourceResolver,
            )
            .expect("wgpu mixed UI GPU timing frame");
        assert!(
            output.snapshot.is_none(),
            "window-target perf test must not use snapshot readback"
        );
        let Some(gpu_render_duration) = output.timings.duration("gpu-render") else {
            return;
        };
        samples.push(gpu_render_duration);
    }

    let assertions = PerformanceAssertions::new(&samples);
    assertions.require_sample_count(12).expect("sample count");
    assertions
        .require_percentile_within(FRAME_PERCENTILE, Duration::from_millis(1))
        .expect("GPU render pass percentile budget");
}

#[cfg(feature = "wgpu")]
fn wgpu_text_cache_perf_request(frame: usize) -> RenderFrameRequest {
    const ROWS: usize = 64;
    let dirty_row = frame % ROWS;
    let mut items = Vec::with_capacity(ROWS + 1);
    items.push(PaintItem {
        node: UiNodeId(50_000),
        rect: UiRect::new(0.0, 0.0, 640.0, 360.0),
        clip_rect: UiRect::new(0.0, 0.0, 640.0, 360.0),
        z_index: 0,
        layer_order: operad::platform::LayerOrder::DEFAULT,
        opacity: 1.0,
        transform: PaintTransform::default(),
        shader: None,
        kind: PaintKind::Rect {
            fill: ColorRgba::new(9, 12, 16, 255),
            stroke: None,
            corner_radius: 0.0,
        },
    });
    for row in 0..ROWS {
        let text = if row == dirty_row {
            format!("Row {row:02} changed on frame {frame}")
        } else {
            format!("Stable row {row:02} reusable toolkit surface")
        };
        items.push(PaintItem {
            node: UiNodeId(50_001 + row),
            rect: UiRect::new(12.0, 10.0 + row as f32 * 7.0, 420.0, 12.0),
            clip_rect: UiRect::new(0.0, 0.0, 640.0, 360.0),
            z_index: 0,
            layer_order: operad::platform::LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            kind: PaintKind::Text(TextContent::new(
                text,
                TextStyle {
                    font_size: 10.0,
                    line_height: 12.0,
                    color: ColorRgba::new(232, 238, 246, 255),
                    ..Default::default()
                },
            )),
        });
    }

    RenderFrameRequest::new(
        RenderTarget::window("perf.text-cache", UiSize::new(640.0, 360.0)),
        UiSize::new(640.0, 360.0),
        PaintList { items },
    )
}

#[cfg(feature = "wgpu")]
fn wgpu_mixed_ui_perf_request(frame: usize) -> RenderFrameRequest {
    const ROWS: usize = 48;
    let viewport = UiSize::new(960.0, 540.0);
    let clip = UiRect::new(0.0, 0.0, viewport.width, viewport.height);
    let dirty_row = frame % ROWS;
    let selected_row = frame.wrapping_mul(5) % ROWS;
    let mut items = Vec::with_capacity(1 + ROWS * 6);
    items.push(PaintItem {
        node: UiNodeId(60_000),
        rect: clip,
        clip_rect: clip,
        z_index: 0,
        layer_order: operad::platform::LayerOrder::DEFAULT,
        opacity: 1.0,
        transform: PaintTransform::default(),
        shader: None,
        kind: PaintKind::Rect {
            fill: ColorRgba::new(8, 11, 16, 255),
            stroke: None,
            corner_radius: 0.0,
        },
    });

    for row in 0..ROWS {
        let y = 24.0 + row as f32 * 10.0;
        let selected = row == selected_row;
        items.push(PaintItem {
            node: UiNodeId(60_100 + row),
            rect: UiRect::new(18.0, y, 900.0, 8.0),
            clip_rect: clip,
            z_index: 0,
            layer_order: operad::platform::LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            kind: PaintKind::Rect {
                fill: if selected {
                    ColorRgba::new(48, 84, 105, 255)
                } else if row % 2 == 0 {
                    ColorRgba::new(17, 24, 32, 255)
                } else {
                    ColorRgba::new(12, 19, 27, 255)
                },
                stroke: None,
                corner_radius: 3.0,
            },
        });
    }

    for row in 0..ROWS {
        let y = 25.0 + row as f32 * 10.0;
        let width = 36.0 + ((row * 17 + frame * 13) % 180) as f32;
        items.push(PaintItem {
            node: UiNodeId(60_200 + row),
            rect: UiRect::new(620.0, y, width, 4.0),
            clip_rect: clip,
            z_index: 0,
            layer_order: operad::platform::LayerOrder::DEFAULT,
            opacity: 0.9,
            transform: PaintTransform::default(),
            shader: None,
            kind: PaintKind::Rect {
                fill: ColorRgba::new(84, 153, 188, 255),
                stroke: None,
                corner_radius: 0.0,
            },
        });
    }

    for row in 0..ROWS {
        let y = 28.0 + row as f32 * 10.0;
        let phase = ((row * 7 + frame * 3) % 24) as f32;
        let x = 420.0 + (row % 3) as f32 * 2.0;
        items.push(PaintItem {
            node: UiNodeId(60_300 + row),
            rect: UiRect::new(x, y - 2.0, 4.0, 4.0),
            clip_rect: clip,
            z_index: 0,
            layer_order: operad::platform::LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            kind: PaintKind::Circle {
                center: UiPoint::new(x + 2.0, y),
                radius: 2.0,
                fill: ColorRgba::new(238, 190, 89, 255),
                stroke: None,
            },
        });
        items.push(PaintItem {
            node: UiNodeId(60_400 + row),
            rect: UiRect::new(450.0, y - 6.0, 72.0, 12.0),
            clip_rect: clip,
            z_index: 0,
            layer_order: operad::platform::LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            kind: PaintKind::Line {
                from: UiPoint::new(450.0, y + phase * 0.08 - 2.0),
                to: UiPoint::new(486.0, y - phase * 0.05 + 2.0),
                stroke: StrokeStyle::new(ColorRgba::new(88, 132, 178, 255), 1.0),
            },
        });
        items.push(PaintItem {
            node: UiNodeId(60_500 + row),
            rect: UiRect::new(486.0, y - 6.0, 72.0, 12.0),
            clip_rect: clip,
            z_index: 0,
            layer_order: operad::platform::LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            kind: PaintKind::Line {
                from: UiPoint::new(486.0, y - phase * 0.05 + 2.0),
                to: UiPoint::new(522.0, y + phase * 0.07 - 1.0),
                stroke: StrokeStyle::new(ColorRgba::new(134, 112, 196, 255), 1.0),
            },
        });
    }

    for row in 0..ROWS {
        let text = if row == dirty_row {
            format!("Mixed row {row:02} live value {frame:02}")
        } else {
            format!("Mixed row {row:02} cached surface")
        };
        items.push(PaintItem {
            node: UiNodeId(60_600 + row),
            rect: UiRect::new(32.0, 22.0 + row as f32 * 10.0, 320.0, 10.0),
            clip_rect: clip,
            z_index: 0,
            layer_order: operad::platform::LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            kind: PaintKind::Text(TextContent::new(
                text,
                TextStyle {
                    font_size: 9.0,
                    line_height: 10.0,
                    color: ColorRgba::new(225, 232, 242, 255),
                    ..Default::default()
                },
            )),
        });
    }

    RenderFrameRequest::new(
        RenderTarget::window("perf.mixed-ui", viewport),
        viewport,
        PaintList { items },
    )
}

#[cfg(feature = "wgpu")]
fn wgpu_large_resource_perf_request(frame: usize) -> RenderFrameRequest {
    const RESOURCE_COUNT: usize = 3;
    const TILE_COUNT: usize = 48;
    let viewport = UiSize::new(960.0, 540.0);
    let clip = UiRect::new(0.0, 0.0, viewport.width, viewport.height);
    let mut items = Vec::with_capacity(1 + RESOURCE_COUNT * TILE_COUNT);
    let mut request = RenderFrameRequest::new(
        RenderTarget::window("perf.large-resources", viewport),
        viewport,
        PaintList { items: Vec::new() },
    );

    items.push(PaintItem {
        node: UiNodeId(70_000),
        rect: clip,
        clip_rect: clip,
        z_index: 0,
        layer_order: operad::platform::LayerOrder::DEFAULT,
        opacity: 1.0,
        transform: PaintTransform::default(),
        shader: None,
        kind: PaintKind::Rect {
            fill: ColorRgba::new(7, 10, 14, 255),
            stroke: None,
            corner_radius: 0.0,
        },
    });

    for resource_index in 0..RESOURCE_COUNT {
        let key = format!("perf.large.texture.{resource_index}");
        let descriptor = ResourceDescriptor::new(
            ResourceHandle::Image(ImageHandle::app(key.clone())),
            PixelSize::new(512, 512),
            ResourceFormat::Rgba8,
        )
        .version(frame as u64 + 1);
        request = request.resource_update(ResourceUpdate::full(
            descriptor,
            large_resource_texture_bytes(resource_index, frame),
        ));

        for tile in 0..TILE_COUNT {
            let column = tile % 12;
            let row = tile / 12 + resource_index * 4;
            let x = 18.0 + column as f32 * 76.0;
            let y = 18.0 + row as f32 * 34.0;
            items.push(PaintItem {
                node: UiNodeId(70_100 + resource_index * TILE_COUNT + tile),
                rect: UiRect::new(x, y, 64.0, 28.0),
                clip_rect: clip,
                z_index: 0,
                layer_order: operad::platform::LayerOrder::DEFAULT,
                opacity: 0.95,
                transform: PaintTransform::default(),
                shader: None,
                kind: PaintKind::Image {
                    key: key.clone(),
                    tint: if tile % 7 == frame % 7 {
                        Some(ColorRgba::new(190, 220, 240, 255))
                    } else {
                        None
                    },
                },
            });
        }
    }

    request.paint = PaintList { items };
    request
}

#[cfg(feature = "wgpu")]
fn large_resource_texture_bytes(resource_index: usize, frame: usize) -> Vec<u8> {
    const WIDTH: usize = 512;
    const HEIGHT: usize = 512;
    let mut bytes = Vec::with_capacity(WIDTH * HEIGHT * 4);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let shade = ((x / 16 + y / 16 + resource_index * 17 + frame * 3) % 255) as u8;
            bytes.extend_from_slice(&[shade, shade.wrapping_add(40), shade.wrapping_add(90), 255]);
        }
    }
    bytes
}

fn perf_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn retained_panel_paint(item_count: usize) -> PaintList {
    PaintList {
        items: (0..item_count)
            .map(|index| PaintItem {
                node: UiNodeId(index),
                rect: UiRect::new(index as f32, 0.0, 1.0, 1.0),
                clip_rect: UiRect::new(0.0, 0.0, 960.0, 540.0),
                z_index: 0,
                layer_order: operad::platform::LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: PaintTransform::default(),
                shader: None,
                kind: PaintKind::Rect {
                    fill: ColorRgba::new(20, 28, 36, 255),
                    stroke: None,
                    corner_radius: 0.0,
                },
            })
            .collect(),
    }
}

fn fixed_style(width: f32, height: f32) -> LayoutStyle {
    layout::fixed(width, height)
}
