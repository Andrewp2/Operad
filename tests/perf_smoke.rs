#![cfg(feature = "widgets")]

mod common;

use std::hint::black_box;
use std::time::{Duration, Instant};

use common::render_document;
use operad::widgets::*;
use operad::*;
use taffy::prelude::{Dimension, Display, FlexDirection, Size as TaffySize, Style};

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
                        Style {
                            size: TaffySize {
                                width: Dimension::auto(),
                                height: Dimension::auto(),
                            },
                            ..Default::default()
                        },
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
        let arrangement = ArrangementGeometry::new(
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
            if let Some(rect) = arrangement.view_clip_rect(lane, EditorAxisRange::new(0.0, 75.0)) {
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

fn perf_screen() -> UiDocument {
    let mut document = UiDocument::new(root_style(PERF_VIEWPORT.width, PERF_VIEWPORT.height));
    let root = document.root;
    document.node_mut(root).style.layout.display = Display::Flex;
    document.node_mut(root).style.layout.flex_direction = FlexDirection::Column;
    document.node_mut(root).visual = UiVisual::panel(
        ColorRgba::new(9, 12, 16, 255),
        Some(StrokeStyle::new(ColorRgba::new(34, 44, 56, 255), 1.0)),
        0.0,
    );
    document
}

fn fixed_style(width: f32, height: f32) -> Style {
    Style {
        size: TaffySize {
            width: length(width),
            height: length(height),
        },
        ..Default::default()
    }
}
