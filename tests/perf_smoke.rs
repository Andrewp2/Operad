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
    let started = Instant::now();
    let mut combined_hash = 0_u64;

    for frame in 0..12 {
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
    }

    let elapsed = started.elapsed();
    assert_ne!(combined_hash, 0);
    assert!(
        elapsed < Duration::from_secs(5),
        "virtualized table render smoke exceeded budget: {elapsed:?}"
    );
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

    let started = Instant::now();
    for _ in 0..20 {
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
    }

    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(3),
        "command palette smoke exceeded budget: {elapsed:?}"
    );
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
