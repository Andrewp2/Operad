#![cfg(feature = "widgets")]

mod common;

use std::hint::black_box;
use std::time::{Duration, Instant};

use common::render_document;
use operad::widgets::*;
use operad::*;

const PERF_VIEWPORT: UiSize = UiSize::new(960.0, 540.0);

#[test]
fn virtualized_table_layout_and_raster_smoke_stays_under_budget() {
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

        let image = render_document(&mut document, PERF_VIEWPORT);
        combined_hash ^= image.hash();
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
fn retained_display_list_reuse_smoke_reports_expected_hit_rate() {
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
fn editor_geometry_scene_build_and_raster_smoke_stays_under_budget() {
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
        let image = render_document(&mut document, PERF_VIEWPORT);
        combined_hash ^= image.hash();
        black_box(document.paint_list().items.len());
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

#[test]
fn scenario_harness_multi_frame_render_smoke_stays_under_budget() {
    let mut harness = ScenarioHarness::new(PERF_VIEWPORT);
    let mut timings = FrameTimingSeries::new("scenario harness render smoke");
    let mut combined_hash = 0_u64;

    for frame in 0..8 {
        let mut document = scenario_perf_document(frame);
        let report = harness
            .run_frame(
                format!("scenario-frame-{frame}"),
                &mut document,
                EventReplay::new()
                    .pointer_click("activate", UiPoint::new(32.0, 20.0))
                    .wheel(
                        "scroll",
                        UiPoint::new(420.0, 132.0),
                        UiPoint::new(0.0, 18.0 + frame as f32),
                    ),
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
        combined_hash ^= {
            let snapshot = report
                .snapshot_assertions(format!("scenario-frame-{frame}"))
                .expect("snapshot");
            snapshot
                .require_min_changed_pixels_from(DEFAULT_CPU_SNAPSHOT_BACKGROUND, 1_000)
                .expect("visible scenario content");
            snapshot.hash()
        };
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
        .require_section_percentile_within("render-frame", 95.0, Duration::from_millis(500))
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

fn scenario_perf_document(frame: usize) -> UiDocument {
    let mut document = perf_screen();
    let root = document.root;
    let toolbar = document.add_child(
        root,
        UiNode::container(
            "perf.scenario.toolbar",
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::px(920.0), layout::px(42.0)),
                ..Default::default()
            },
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
                UiNodeStyle {
                    layout: layout::with_size(layout::row(), layout::px(900.0), layout::px(24.0)),
                    ..Default::default()
                },
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
                format!("Scenario row {row:02}    frame {frame}    reusable toolkit surface"),
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
