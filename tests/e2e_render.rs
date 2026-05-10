#![cfg(feature = "widgets")]

mod common;

use common::{assert_snapshot, render_document};
use operad::widgets::*;
use operad::*;

const VIEWPORT: UiSize = UiSize::new(640.0, 360.0);

#[test]
fn core_controls_snapshot() {
    let mut document = screen();
    let root = document.root;
    let panel = document.add_child(
        root,
        UiNode::container(
            "controls.panel",
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_flex_start_children(layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::percent(1.0),
                    )),
                    16.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(panel_visual())
        .with_shader(ShaderEffect::new("snapshot.panel_glow").uniform("radius", 18.0)),
    );

    let left = document.add_child(
        panel,
        UiNode::container("controls.left", column_style(280.0, 328.0)),
    );
    let right = document.add_child(
        panel,
        UiNode::container(
            "controls.preview",
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_margin_left(
                        layout::with_size(layout::column(), layout::px(312.0), layout::px(328.0)),
                        16.0,
                    ),
                    12.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(16, 23, 31, 255),
            Some(StrokeStyle::new(ColorRgba::new(73, 91, 109, 255), 1.0)),
            4.0,
        )),
    );

    button(
        &mut document,
        left,
        "transport.play",
        "Play",
        ButtonOptions {
            layout: fixed_style(148.0, 36.0),
            visual: UiVisual::panel(
                ColorRgba::new(34, 83, 112, 255),
                Some(StrokeStyle::new(ColorRgba::new(101, 178, 214, 255), 1.0)),
                4.0,
            ),
            text_style: text_style(15.0, ColorRgba::WHITE),
            leading_image: Some(
                ImageContent::new("icons.play").tinted(ColorRgba::new(152, 222, 255, 255)),
            ),
            ..Default::default()
        },
    );
    checkbox(
        &mut document,
        left,
        "transport.loop",
        "Loop region",
        true,
        CheckboxOptions::default(),
    );
    slider(
        &mut document,
        left,
        "filter.cutoff",
        0.72,
        0.0..1.0,
        SliderOptions {
            layout: fixed_style(220.0, 30.0),
            ..Default::default()
        },
    );
    text_input(
        &mut document,
        left,
        "track.name",
        &TextInputState::new("Warm pad"),
        TextInputOptions {
            layout: fixed_style(220.0, 34.0),
            placeholder: "Track name".to_string(),
            ..Default::default()
        },
    );
    document.add_child(
        left,
        UiNode::canvas(
            "controls.meter",
            "snapshot.level_meter",
            fixed_style(220.0, 74.0),
        ),
    );

    document.add_child(
        right,
        UiNode::image(
            "controls.artwork",
            ImageContent::new("images.patch_art").tinted(ColorRgba::new(88, 124, 164, 255)),
            fixed_style(288.0, 112.0),
        ),
    );
    document.add_child(
        right,
        UiNode::scene(
            "controls.envelope",
            vec![
                ScenePrimitive::Line {
                    from: UiPoint::new(8.0, 84.0),
                    to: UiPoint::new(70.0, 22.0),
                    stroke: StrokeStyle::new(ColorRgba::new(123, 202, 255, 255), 2.0),
                },
                ScenePrimitive::Line {
                    from: UiPoint::new(70.0, 22.0),
                    to: UiPoint::new(142.0, 44.0),
                    stroke: StrokeStyle::new(ColorRgba::new(123, 202, 255, 255), 2.0),
                },
                ScenePrimitive::Line {
                    from: UiPoint::new(142.0, 44.0),
                    to: UiPoint::new(276.0, 68.0),
                    stroke: StrokeStyle::new(ColorRgba::new(123, 202, 255, 255), 2.0),
                },
                ScenePrimitive::Circle {
                    center: UiPoint::new(70.0, 22.0),
                    radius: 5.0,
                    fill: ColorRgba::new(250, 232, 147, 255),
                    stroke: None,
                },
            ],
            fixed_style(288.0, 112.0),
        ),
    );

    let image = render_document(&mut document, VIEWPORT);
    assert_snapshot("core_controls", &image, 0x3f1125bd63d7419b);
}

#[test]
fn built_in_icon_fallbacks_snapshot() {
    let mut document = screen();
    let root = document.root;
    let icons = [
        BuiltInIcon::Play,
        BuiltInIcon::Record,
        BuiltInIcon::Search,
        BuiltInIcon::Folder,
        BuiltInIcon::Grid,
    ];

    for (index, icon) in icons.into_iter().enumerate() {
        let tint = match icon {
            BuiltInIcon::Record => ColorRgba::new(238, 76, 92, 255),
            BuiltInIcon::Folder => ColorRgba::new(246, 190, 92, 255),
            BuiltInIcon::Grid => ColorRgba::new(132, 220, 172, 255),
            _ => ColorRgba::new(152, 222, 255, 255),
        };
        document.add_child(
            root,
            UiNode::scene(
                format!("builtin.{}", icon.key()),
                icon.fallback_scene(UiRect::new(5.0, 5.0, 26.0, 26.0), tint),
                absolute_style(40.0 + index as f32 * 54.0, 44.0, 36.0, 36.0),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(16, 23, 31, 255),
                Some(StrokeStyle::new(ColorRgba::new(73, 91, 109, 255), 1.0)),
                4.0,
            )),
        );
    }

    let image = render_document(&mut document, VIEWPORT);
    assert_snapshot("built_in_icon_fallbacks", &image, 0xd56578109ab09ee1);
}

#[test]
fn menus_and_command_palette_snapshot() {
    let mut document = screen();
    let root = document.root;
    let options = vec![
        SelectOption::new("saw", "Saw"),
        SelectOption::new("pad", "Pad"),
        SelectOption::new("keys", "Keys"),
        SelectOption::new("vox", "Vocal chop").disabled(),
    ];
    let mut select_state = SelectMenuState::with_selected(1);
    select_state.open(&options);
    dropdown_select(
        &mut document,
        root,
        "instrument",
        &options,
        &select_state,
        Some(AnchoredPopup::new(
            UiRect::new(24.0, 22.0, 180.0, 30.0),
            UiRect::new(0.0, 0.0, VIEWPORT.width, VIEWPORT.height),
            PopupPlacement::new(PopupSide::Bottom, PopupAlign::Start),
        )),
        DropdownSelectOptions {
            trigger_layout: absolute_style(24.0, 22.0, 180.0, 30.0),
            ..Default::default()
        },
    );

    let commands = vec![
        CommandPaletteItem::new("transport.play", "Toggle transport")
            .subtitle("Playback")
            .shortcut("Space")
            .keyword("play"),
        CommandPaletteItem::new("track.freeze", "Freeze track")
            .subtitle("Track")
            .shortcut("F"),
        CommandPaletteItem::new("track.transform", "Transform selection")
            .subtitle("Edit")
            .shortcut("T"),
        CommandPaletteItem::new("render.bounce", "Bounce region")
            .subtitle("Export")
            .disabled(),
    ];
    let mut palette_state = CommandPaletteState::new().with_query("tr");
    palette_state.move_active(&commands, NavigationDirection::Next);
    command_palette(
        &mut document,
        root,
        "palette",
        &commands,
        &palette_state,
        Some(AnchoredPopup::new(
            UiRect::new(72.0, 84.0, 480.0, 34.0),
            UiRect::new(0.0, 0.0, VIEWPORT.width, VIEWPORT.height),
            PopupPlacement::new(PopupSide::Bottom, PopupAlign::Center),
        )),
        CommandPaletteOptions::default(),
    );

    let image = render_document(&mut document, VIEWPORT);
    assert_snapshot("menus_palette", &image, 0x2ec158ddf2098f9f);
}

#[test]
fn pickers_snapshot() {
    let mut document = screen();
    let root = document.root;
    let picker_root = document.add_child(
        root,
        UiNode::container(
            "pickers.root",
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(layout::row(), layout::percent(1.0), layout::percent(1.0)),
                    16.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(panel_visual()),
    );
    let calendar = document.add_child(
        picker_root,
        UiNode::container("pickers.calendar", column_style(300.0, 328.0)),
    );
    label(
        &mut document,
        calendar,
        "pickers.calendar.title",
        "May 2026",
        text_style(17.0, ColorRgba::new(232, 238, 247, 255)),
        fixed_style(280.0, 28.0),
    );
    let model = DatePickerModel::builder()
        .selected(CalendarDate::new(2026, 5, 9))
        .today(CalendarDate::new(2026, 5, 9))
        .first_weekday(Weekday::Monday)
        .build();
    let cells = model.grid();
    for (week_index, week) in cells.chunks(7).enumerate() {
        let row = document.add_child(
            calendar,
            UiNode::container(
                format!("pickers.calendar.week.{week_index}"),
                UiNodeStyle {
                    layout: layout::with_size(layout::row(), layout::px(280.0), layout::px(32.0)),
                    ..Default::default()
                },
            ),
        );
        for cell in week {
            let mut options = ButtonOptions {
                layout: fixed_style(38.0, 30.0),
                text_style: text_style(13.0, ColorRgba::new(230, 236, 246, 255)),
                ..Default::default()
            };
            if !cell.in_visible_month {
                options.text_style.color = ColorRgba::new(107, 119, 136, 255);
                options.visual.fill = ColorRgba::new(15, 19, 25, 255);
            } else if cell.selected {
                options.visual = UiVisual::panel(ColorRgba::new(56, 97, 138, 255), None, 4.0);
            } else if cell.today {
                options.visual = UiVisual::panel(
                    ColorRgba::new(34, 47, 61, 255),
                    Some(StrokeStyle::new(ColorRgba::new(242, 206, 104, 255), 1.0)),
                    4.0,
                );
            }
            button(
                &mut document,
                row,
                format!("pickers.calendar.day.{}", cell.date.day),
                cell.date.day.to_string(),
                options,
            );
        }
    }

    let side = document.add_child(
        picker_root,
        UiNode::container(
            "pickers.side",
            UiNodeStyle {
                layout: layout::with_margin_left(
                    layout::with_size(layout::column(), layout::px(292.0), layout::px(328.0)),
                    16.0,
                ),
                ..Default::default()
            },
        ),
    );
    let swatches = [
        ColorRgba::new(244, 96, 112, 255),
        ColorRgba::new(242, 184, 84, 255),
        ColorRgba::new(74, 186, 132, 255),
        ColorRgba::new(91, 166, 240, 255),
        ColorRgba::new(180, 126, 235, 255),
    ];
    let swatch_row = document.add_child(
        side,
        UiNode::container(
            "pickers.swatches",
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::px(260.0), layout::px(42.0)),
                ..Default::default()
            },
        ),
    );
    for (index, color) in swatches.iter().copied().enumerate() {
        document.add_child(
            swatch_row,
            UiNode::container(
                format!("pickers.swatch.{index}"),
                UiNodeStyle {
                    layout: layout::with_margin_right(layout::fixed(40.0, 34.0), 8.0),
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_visual(UiVisual::panel(
                color,
                Some(StrokeStyle::new(ColorRgba::new(235, 241, 250, 255), 1.0)),
                4.0,
            ))
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label(format!("Color swatch {index}"))
                    .focusable(),
            ),
        );
    }
    text_input(
        &mut document,
        side,
        "pickers.hex",
        &TextInputState::new("#5ba6f0"),
        TextInputOptions {
            layout: fixed_style(180.0, 34.0),
            ..Default::default()
        },
    );
    let path_state = PathPickerState::new(PathPickerMode::OpenFile, "/sessions/album/take-09.wav");
    for crumb in path_state.breadcrumbs().into_iter().take(4) {
        button(
            &mut document,
            side,
            format!("pickers.path.{}", crumb.label),
            crumb.label,
            ButtonOptions {
                layout: fixed_style(220.0, 28.0),
                text_style: text_style(13.0, ColorRgba::new(213, 222, 235, 255)),
                ..Default::default()
            },
        );
    }

    let image = render_document(&mut document, VIEWPORT);
    assert_snapshot("pickers", &image, 0xf668731a14f39c35);
}

#[test]
fn data_widgets_snapshot() {
    let mut document = screen();
    let root = document.root;
    let shell = document.add_child(
        root,
        UiNode::container(
            "data.shell",
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(layout::row(), layout::percent(1.0), layout::percent(1.0)),
                    14.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(panel_visual()),
    );
    let left = document.add_child(
        shell,
        UiNode::container("data.left", column_style(244.0, 332.0)),
    );
    property_inspector_grid(
        &mut document,
        left,
        "inspector",
        &[
            PropertyGridRow::new("gain", "Gain", "-6.0 dB").with_kind(PropertyValueKind::Number),
            PropertyGridRow::new("mute", "Muted", "false").with_kind(PropertyValueKind::Boolean),
            PropertyGridRow::new("color", "Color", "#5ba6f0").with_kind(PropertyValueKind::Color),
            PropertyGridRow::new("id", "Track ID", "AUX-04").read_only(),
        ],
        PropertyInspectorOptions {
            selected_index: Some(2),
            ..Default::default()
        },
    );
    let tree_state = TreeViewState {
        expanded_ids: vec!["root".to_string()],
        selected_index: Some(2),
    };
    tree_view(
        &mut document,
        left,
        "outliner",
        &[TreeItem::new("root", "Session").with_children(vec![
            TreeItem::new("drums", "Drums"),
            TreeItem::new("pads", "Pads"),
            TreeItem::new("archive", "Archive").disabled(),
        ])],
        &tree_state,
        TreeViewOptions::default(),
    );

    let right = document.add_child(
        shell,
        UiNode::container(
            "data.right",
            UiNodeStyle {
                layout: layout::with_margin_left(
                    layout::with_size(layout::column(), layout::px(354.0), layout::px(332.0)),
                    12.0,
                ),
                ..Default::default()
            },
        ),
    );
    virtualized_data_table(
        &mut document,
        right,
        "clips",
        &[
            DataTableColumn::new("clip", "Clip", 150.0),
            DataTableColumn::new("bar", "Bar", 64.0).with_alignment(DataCellAlignment::End),
            DataTableColumn::new("state", "State", 110.0),
        ],
        VirtualDataTableSpec {
            row_count: 80,
            row_height: 24.0,
            viewport_width: 324.0,
            viewport_height: 132.0,
            scroll_offset: UiPoint::new(0.0, 216.0),
            overscan_rows: 1,
        },
        DataTableOptions {
            selection: DataTableSelection::single_row(10)
                .with_active_cell(DataTableCellIndex::new(10, 2)),
            ..Default::default()
        },
        |document, parent, cell| {
            let text = match cell.column {
                0 => format!("Clip {}", cell.row),
                1 => format!("{}", cell.row + 1),
                _ => {
                    if cell.row % 2 == 0 {
                        "armed".to_string()
                    } else {
                        "idle".to_string()
                    }
                }
            };
            document.add_child(
                parent,
                UiNode::text(
                    format!("clips.cell.{}.{}", cell.row, cell.column),
                    text,
                    text_style(12.0, ColorRgba::new(224, 232, 242, 255)),
                    fixed_style(100.0, 20.0),
                ),
            );
        },
    );
    tab_group(
        &mut document,
        right,
        "editors",
        &[
            TabItem::new("mixer", "Mixer"),
            TabItem::new("automation", "Automation").dirty(),
            TabItem::new("notes", "Notes").closable(),
        ],
        TabGroupState::selected(1),
        TabGroupOptions {
            layout: fixed_style(324.0, 156.0),
            ..Default::default()
        },
        |document, parent, index| {
            label(
                document,
                parent,
                "editors.panel.label",
                format!("Selected editor {index}"),
                text_style(14.0, ColorRgba::new(224, 232, 242, 255)),
                fixed_style(240.0, 30.0),
            );
        },
    );

    let image = render_document(&mut document, VIEWPORT);
    assert_snapshot("data_widgets", &image, 0x627c7f39523955c4);
}

#[test]
fn surfaces_snapshot() {
    let mut document = screen();
    let root = document.root;
    split_pane(
        &mut document,
        root,
        "surface.split",
        SplitAxis::Horizontal,
        SplitPaneState::new(0.42).with_min_sizes(160.0, 180.0),
        SplitPaneOptions {
            layout: layout::with_margin_all(
                layout::with_size(layout::row(), layout::px(612.0), layout::px(188.0)),
                14.0,
            ),
            pane_visual: UiVisual::panel(
                ColorRgba::new(17, 23, 30, 255),
                Some(StrokeStyle::new(ColorRgba::new(57, 72, 88, 255), 1.0)),
                3.0,
            ),
            ..Default::default()
        },
        |document, parent| {
            label(
                document,
                parent,
                "surface.left.label",
                "Dock: inspector",
                text_style(14.0, ColorRgba::new(230, 237, 247, 255)),
                fixed_style(180.0, 26.0),
            );
        },
        |document, parent| {
            document.add_child(
                parent,
                UiNode::canvas(
                    "surface.graph",
                    "snapshot.routing_graph",
                    fixed_style(300.0, 120.0),
                ),
            );
        },
    );
    let mut stack = ToastStack::new(3);
    stack.push(
        ToastSeverity::Success,
        "Rendered stem",
        Some("Lead Vocal bounced to disk".to_string()),
        None,
    );
    stack.push(
        ToastSeverity::Warning,
        "Peak warning",
        Some("Master reached -0.1 dBFS".to_string()),
        None,
    );
    toast_stack(
        &mut document,
        root,
        "surface.toasts",
        &stack,
        ToastStackOptions {
            layout: layout::with_absolute_position(
                layout::with_size(layout::column(), layout::px(310.0), layout::auto()),
                300.0,
                210.0,
            ),
            ..Default::default()
        },
    );
    timeline_ruler(
        &mut document,
        root,
        "surface.timeline",
        RulerSpec {
            range: TimelineRange::new(0.0, 16.0),
            width: 612.0,
            major_step: 4.0,
            minor_step: 1.0,
            label_every: 1,
        },
        TimelineRulerOptions {
            layout: absolute_style(14.0, 304.0, 612.0, 34.0),
            height: 34.0,
            ..Default::default()
        },
    );

    let image = render_document(&mut document, VIEWPORT);
    assert_snapshot("surfaces", &image, 0xe8a6f5c59d18077c);
}

#[test]
fn editor_primitives_snapshot() {
    let mut document = screen();
    let root = document.root;
    let panel = document.add_child(
        root,
        UiNode::container(
            "editor.panel",
            UiNodeStyle {
                layout: absolute_style(16.0, 16.0, 608.0, 328.0),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(panel_visual())
        .with_accessibility(
            EditorSurfaceAccessibility::new("Editor primitives")
                .description("Reusable range, lane, curve, and canvas-style editor geometry")
                .visible_units(EditorAxisRange::new(0.0, 56.0))
                .visible_lanes(VisibleLaneRange::new(0, 4))
                .target_count(14)
                .selected_count(2)
                .active("range.review")
                .instruction("Use arrow keys to move selected editor items")
                .accessibility_meta(),
        ),
    );
    label(
        &mut document,
        panel,
        "editor.title",
        "Reusable editor surface",
        text_style(15.0, ColorRgba::new(230, 237, 247, 255)),
        absolute_style(14.0, 10.0, 220.0, 24.0),
    );
    label(
        &mut document,
        panel,
        "editor.caption",
        "ranges, lanes, curve points, hit handles",
        text_style(12.0, ColorRgba::new(148, 161, 176, 255)),
        absolute_style(374.0, 12.0, 220.0, 20.0),
    );

    let transform = EditorTransform::new(UiRect::new(0.0, 0.0, 560.0, 248.0))
        .with_scale(UiPoint::new(10.0, 1.0));
    let arrangement = LaneTimelineGeometry::new(
        transform,
        LaneGeometry::new(30.0, 4)
            .with_origin_y(36.0)
            .with_lane_gap(9.0),
    );
    let timeline = arrangement.timeline;
    let range_geometry =
        TimelineRangeItemGeometry::new(arrangement).with_resize_handle_width_px(5.0);
    let curve_geometry = CurveEditorGeometry::new(
        timeline,
        EditorAxisRange::new(0.0, 1.0),
        UiRect::new(0.0, 194.0, 560.0, 44.0),
    )
    .with_point_radius_px(4.0);

    let mut scene = Vec::new();
    scene.push(ScenePrimitive::Rect(PaintRect::solid(
        UiRect::new(0.0, 0.0, 560.0, 248.0),
        ColorRgba::new(10, 14, 20, 255),
    )));
    for lane in 0..4 {
        let lane_rect = arrangement
            .view_range_rect(lane, EditorAxisRange::new(0.0, 56.0))
            .expect("lane rect");
        let fill = if lane % 2 == 0 {
            ColorRgba::new(14, 21, 29, 255)
        } else {
            ColorRgba::new(17, 25, 34, 255)
        };
        scene.push(ScenePrimitive::Rect(
            PaintRect::solid(lane_rect, fill)
                .stroke(StrokeStyle::new(ColorRgba::new(35, 48, 62, 255), 1.0)),
        ));
    }
    for unit in (0..=56).step_by(4) {
        let x = timeline.unit_to_view_x(unit as f32);
        scene.push(ScenePrimitive::Line {
            from: UiPoint::new(x, 28.0),
            to: UiPoint::new(x, 238.0),
            stroke: StrokeStyle::new(
                if unit % 16 == 0 {
                    ColorRgba::new(71, 90, 109, 255)
                } else {
                    ColorRgba::new(35, 47, 60, 255)
                },
                1.0,
            ),
        });
    }

    let items = [
        (
            TimelineRangeItem::new("range.lead", 0, 4.0, 10.0).selected(true),
            ColorRgba::new(54, 132, 166, 255),
        ),
        (
            TimelineRangeItem::new("range.review", 1, 15.0, 12.0).dragging(true),
            ColorRgba::new(115, 101, 190, 255),
        ),
        (
            TimelineRangeItem::new("range.disabled", 2, 8.0, 20.0).disabled(true),
            ColorRgba::new(74, 83, 93, 255),
        ),
        (
            TimelineRangeItem::new("range.tool", 3, 30.0, 18.0),
            ColorRgba::new(79, 145, 103, 255),
        ),
    ];
    for (item, fill) in &items {
        let Some(rect) = range_geometry.item_view_rect(item) else {
            continue;
        };
        scene.push(ScenePrimitive::Rect(
            PaintRect::solid(rect, *fill)
                .stroke(StrokeStyle::new(ColorRgba::new(187, 210, 222, 255), 1.0))
                .corner_radii(CornerRadii::uniform(4.0)),
        ));
        if !item.disabled {
            for edge in [TimelineRangeItemEdge::Start, TimelineRangeItemEdge::End] {
                let handle = range_geometry
                    .edge_world_rect(item, edge)
                    .map(|rect| transform.world_to_view_rect(rect))
                    .expect("range handle");
                scene.push(ScenePrimitive::Rect(PaintRect::solid(
                    handle,
                    ColorRgba::new(230, 237, 247, 210),
                )));
            }
        }
    }

    let playhead = timeline.playhead_rect(22.5, 28.0, 210.0, 2.0);
    scene.push(ScenePrimitive::Rect(PaintRect::solid(
        playhead,
        ColorRgba::new(244, 94, 104, 255),
    )));
    scene.push(ScenePrimitive::Rect(PaintRect::solid(
        curve_geometry.view_rect,
        ColorRgba::new(12, 18, 24, 255),
    )));
    let curve_points = vec![
        CurvePoint::new("curve.a", 2.0, 0.25),
        CurvePoint::new("curve.b", 12.0, 0.72).selected(true),
        CurvePoint::new("curve.c", 25.0, 0.42),
        CurvePoint::new("curve.d", 38.0, 0.9).dragging(true),
        CurvePoint::new("curve.e", 52.0, 0.35),
    ];
    let path_points = curve_geometry.segment_view_path(&curve_points, CurveInterpolation::Step);
    let mut path = PaintPath::new();
    if let Some(first) = path_points.first().copied() {
        path = path.move_to(first);
        for point in path_points.into_iter().skip(1) {
            path = path.line_to(point);
        }
    }
    scene.push(ScenePrimitive::Path(
        path.stroke(StrokeStyle::new(ColorRgba::new(246, 190, 92, 255), 2.0)),
    ));
    for point in &curve_points {
        let center = curve_geometry.point_view_position(point);
        scene.push(ScenePrimitive::Circle {
            center,
            radius: if point.selected || point.dragging {
                5.0
            } else {
                4.0
            },
            fill: if point.dragging {
                ColorRgba::new(255, 235, 146, 255)
            } else {
                ColorRgba::new(246, 190, 92, 255)
            },
            stroke: Some(StrokeStyle::new(ColorRgba::new(52, 40, 21, 255), 1.0)),
        });
    }

    document.add_child(
        panel,
        UiNode::scene(
            "editor.surface",
            scene,
            absolute_style(14.0, 46.0, 580.0, 260.0),
        ),
    );

    let image = render_document(&mut document, VIEWPORT);
    assert_snapshot("editor_primitives", &image, 0x1be24b70e9df5d9c);
}

fn screen() -> UiDocument {
    let mut document = UiDocument::new(root_style(VIEWPORT.width, VIEWPORT.height));
    let root = document.root;
    document.node_mut(root).visual = UiVisual::panel(
        ColorRgba::new(9, 12, 16, 255),
        Some(StrokeStyle::new(ColorRgba::new(30, 38, 48, 255), 1.0)),
        0.0,
    );
    document
}

fn panel_visual() -> UiVisual {
    UiVisual::panel(
        ColorRgba::new(13, 18, 24, 255),
        Some(StrokeStyle::new(ColorRgba::new(48, 60, 74, 255), 1.0)),
        4.0,
    )
}

fn fixed_style(width: f32, height: f32) -> LayoutStyle {
    layout::fixed(width, height)
}

fn absolute_style(x: f32, y: f32, width: f32, height: f32) -> LayoutStyle {
    layout::absolute(x, y, width, height)
}

fn column_style(width: f32, height: f32) -> UiNodeStyle {
    UiNodeStyle {
        layout: layout::with_size(layout::column(), layout::px(width), layout::px(height)),
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

fn text_style(font_size: f32, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        color,
        ..Default::default()
    }
}
