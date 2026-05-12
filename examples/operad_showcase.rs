#[cfg(not(feature = "widgets"))]
fn main() {
    println!("operad_showcase: rebuild with `--features widgets` to render the full showcase UI");
}

#[cfg(feature = "widgets")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    const VIEWPORT: operad::UiSize = operad::UiSize::new(1280.0, 800.0);

    let mut document = showcase::build_document(VIEWPORT);
    let image = operad::CpuSnapshotRenderer::default().render_document(&mut document, VIEWPORT)?;
    let path = std::env::var_os("OPERAD_SHOWCASE_PPM")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/operad_showcase.ppm"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    image.write_ppm(&path)?;

    println!(
        "operad_showcase: wrote {:?}, {} paint items, {} accessibility nodes",
        path,
        document.paint_list().items.len(),
        document.accessibility_tree().len()
    );

    Ok(())
}

#[cfg(feature = "widgets")]
mod showcase {
    use operad::widgets::*;
    use operad::*;

    pub fn build_document(viewport: UiSize) -> UiDocument {
        let mut document = UiDocument::new(root_style(viewport.width, viewport.height));
        let root = document.root;
        document.node_mut(root).visual = UiVisual::panel(
            ColorRgba::new(10, 13, 16, 255),
            Some(StrokeStyle::new(ColorRgba::new(31, 39, 49, 255), 1.0)),
            0.0,
        );

        add_background_scene(&mut document, root, viewport);
        add_header(&mut document, root, viewport);
        add_left_panel(&mut document, root);
        add_workspace(&mut document, root, viewport);
        add_right_panel(&mut document, root, viewport);
        add_global_overlays(&mut document, root, viewport);

        document
    }

    fn add_background_scene(document: &mut UiDocument, root: UiNodeId, viewport: UiSize) {
        let gradient = PaintRect::new(
            UiRect::new(0.0, 0.0, viewport.width, viewport.height),
            PaintBrush::LinearGradient(
                LinearGradient::new(
                    UiPoint::new(0.0, 0.0),
                    UiPoint::new(viewport.width, viewport.height),
                    ColorRgba::new(12, 17, 20, 255),
                    ColorRgba::new(21, 20, 30, 255),
                )
                .stop(0.58, ColorRgba::new(15, 24, 24, 255))
                .fallback(ColorRgba::new(12, 17, 20, 255)),
            ),
        );
        document.add_child(
            root,
            UiNode::scene(
                "showcase.background",
                vec![ScenePrimitive::Rect(gradient)],
                layout::absolute(0.0, 0.0, viewport.width, viewport.height),
            ),
        );
    }

    fn add_header(document: &mut UiDocument, root: UiNodeId, viewport: UiSize) {
        let header = document.add_child(
            root,
            UiNode::container(
                "showcase.header",
                UiNodeStyle {
                    clip: ClipBehavior::Clip,
                    ..layout::node_style(layout::absolute(0.0, 0.0, viewport.width, 48.0))
                },
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(17, 22, 27, 244),
                Some(StrokeStyle::new(ColorRgba::new(46, 57, 70, 255), 1.0)),
                0.0,
            ))
            .with_shader(ShaderEffect::new("showcase.header.surface")),
        );

        let menus = vec![
            MenuBarMenu::new(
                "file",
                "File",
                vec![
                    MenuItem::command("session.new", "New Session").shortcut("Ctrl+N"),
                    MenuItem::command("session.open", "Open...").shortcut("Ctrl+O"),
                    MenuItem::separator(),
                    MenuItem::command("session.export", "Export Snapshot").shortcut("Ctrl+E"),
                ],
            ),
            MenuBarMenu::new(
                "edit",
                "Edit",
                vec![
                    MenuItem::command("edit.undo", "Undo").shortcut("Ctrl+Z"),
                    MenuItem::command("edit.redo", "Redo").shortcut("Ctrl+Shift+Z"),
                    MenuItem::separator(),
                    MenuItem::command("edit.copy", "Copy").shortcut("Ctrl+C"),
                    MenuItem::command("edit.paste", "Paste").shortcut("Ctrl+V"),
                ],
            ),
            MenuBarMenu::new(
                "view",
                "View",
                vec![
                    MenuItem::check("view.grid", "Show Grid", true),
                    MenuItem::check("view.inspector", "Inspector", true),
                    MenuItem::submenu(
                        "view.zoom",
                        "Zoom",
                        vec![
                            MenuItem::command("view.zoom_in", "Zoom In").shortcut("Ctrl++"),
                            MenuItem::command("view.zoom_out", "Zoom Out").shortcut("Ctrl+-"),
                        ],
                    ),
                ],
            ),
            MenuBarMenu::new(
                "run",
                "Run",
                vec![
                    MenuItem::command("run.preview", "Preview").shortcut("Space"),
                    MenuItem::command("run.profile", "Profile Frame").shortcut("F6"),
                    MenuItem::command("run.reset", "Reset Scene").destructive(),
                ],
            ),
        ];
        let state = MenuBarState {
            open_menu: Some(2),
            active_item: Some(0),
        };
        let anchors = MenuBarAnchors {
            anchors: vec![
                UiRect::new(8.0, 9.0, 44.0, 30.0),
                UiRect::new(52.0, 9.0, 44.0, 30.0),
                UiRect::new(96.0, 9.0, 52.0, 30.0),
                UiRect::new(148.0, 9.0, 44.0, 30.0),
            ],
            viewport: UiRect::new(0.0, 0.0, viewport.width, viewport.height),
        };
        menu_bar(
            document,
            root,
            "showcase.menu_bar",
            &menus,
            &state,
            Some(&anchors),
            MenuBarOptions {
                layout: absolute_row(8.0, 9.0, 250.0, 30.0),
                text_style: text(13.0, ColorRgba::new(226, 233, 241, 255)),
                popup_menu: MenuListOptions {
                    active_shader: Some(ShaderEffect::new("showcase.menu.active")),
                    menu_shader: Some(ShaderEffect::new("showcase.menu.shadow")),
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        document.add_child(
            header,
            UiNode::scene(
                "showcase.search_icon",
                BuiltInIcon::Search.fallback_scene(
                    UiRect::new(3.0, 3.0, 18.0, 18.0),
                    ColorRgba::new(153, 213, 221, 255),
                ),
                layout::absolute(370.0, 13.0, 24.0, 24.0),
            ),
        );
        label(
            document,
            header,
            "showcase.command_hint",
            "Command Palette",
            text(14.0, ColorRgba::new(236, 241, 246, 255)),
            layout::absolute(400.0, 9.0, 146.0, 30.0),
        );
        label(
            document,
            header,
            "showcase.command_shortcut",
            "Ctrl+Shift+P",
            text(12.0, ColorRgba::new(161, 174, 188, 255)),
            layout::absolute(548.0, 10.0, 104.0, 28.0),
        );
        label(
            document,
            header,
            "showcase.title",
            "Operad v5 Showcase",
            text(15.0, ColorRgba::new(247, 240, 204, 255)),
            layout::absolute(viewport.width - 270.0, 9.0, 180.0, 30.0),
        );
        label(
            document,
            header,
            "showcase.frame_rate",
            "120Hz UI",
            text(12.0, ColorRgba::new(137, 211, 164, 255)),
            layout::absolute(viewport.width - 86.0, 10.0, 70.0, 28.0),
        );
    }

    fn add_left_panel(document: &mut UiDocument, root: UiNodeId) {
        let panel = panel(
            document,
            root,
            "showcase.left",
            UiRect::new(16.0, 64.0, 260.0, 704.0),
            "Navigation + State",
        );

        let tree_state = TreeViewState {
            expanded_ids: vec!["workspace".to_string(), "screens".to_string()],
            selected_index: Some(2),
        };
        tree_view(
            document,
            panel,
            "showcase.outliner",
            &[TreeItem::new("workspace", "Workspace").with_children(vec![
                TreeItem::new("screens", "Screens").with_children(vec![
                    TreeItem::new("dashboard", "Dashboard"),
                    TreeItem::new("editor", "Editor"),
                    TreeItem::new("settings", "Settings").disabled(),
                ]),
                TreeItem::new("assets", "Assets"),
                TreeItem::new("commands", "Commands"),
            ])],
            &tree_state,
            TreeViewOptions {
                layout: layout::absolute(14.0, 44.0, 232.0, 150.0),
                selected_row_shader: Some(ShaderEffect::new("showcase.tree.selected")),
                ..Default::default()
            },
        );

        property_inspector_grid(
            document,
            panel,
            "showcase.properties",
            &[
                PropertyGridRow::new("surface", "Surface", "Main editor"),
                PropertyGridRow::new("scale", "Scale", "1.25x")
                    .with_kind(PropertyValueKind::Number),
                PropertyGridRow::new("locked", "Locked", "false")
                    .with_kind(PropertyValueKind::Boolean),
                PropertyGridRow::new("accent", "Accent", "#5ba6f0")
                    .with_kind(PropertyValueKind::Color),
                PropertyGridRow::new("id", "Stable ID", "surface.editor").read_only(),
            ],
            PropertyInspectorOptions {
                layout: layout::absolute(14.0, 210.0, 232.0, 154.0),
                selected_index: Some(3),
                ..Default::default()
            },
        );

        label(
            document,
            panel,
            "showcase.pickers.title",
            "Picker primitives",
            text(13.0, ColorRgba::new(234, 239, 247, 255)),
            layout::absolute(14.0, 386.0, 200.0, 20.0),
        );
        let model = DatePickerModel::builder()
            .selected(CalendarDate::new(2026, 5, 12))
            .today(CalendarDate::new(2026, 5, 12))
            .first_weekday(Weekday::Monday)
            .build();
        for (index, cell) in model.grid().into_iter().skip(7).take(14).enumerate() {
            let x = 14.0 + (index % 7) as f32 * 32.0;
            let y = 414.0 + (index / 7) as f32 * 30.0;
            let mut options = ButtonOptions {
                layout: layout::absolute(x, y, 28.0, 26.0),
                text_style: text(12.0, ColorRgba::new(224, 232, 242, 255)),
                ..Default::default()
            };
            if cell.selected {
                options.visual = UiVisual::panel(ColorRgba::new(55, 105, 143, 255), None, 4.0);
            } else if cell.today {
                options.visual = UiVisual::panel(
                    ColorRgba::new(42, 49, 55, 255),
                    Some(StrokeStyle::new(ColorRgba::new(232, 198, 93, 255), 1.0)),
                    4.0,
                );
            } else if !cell.in_visible_month {
                options.text_style.color = ColorRgba::new(113, 124, 139, 255);
            }
            button(
                document,
                panel,
                format!("showcase.calendar.day.{}", cell.date.day),
                cell.date.day.to_string(),
                options,
            );
        }

        let swatches = [
            ColorRgba::new(238, 82, 98, 255),
            ColorRgba::new(235, 179, 78, 255),
            ColorRgba::new(81, 183, 129, 255),
            ColorRgba::new(89, 163, 232, 255),
            ColorRgba::new(173, 119, 226, 255),
        ];
        for (index, color) in swatches.into_iter().enumerate() {
            document.add_child(
                panel,
                UiNode::container(
                    format!("showcase.swatch.{index}"),
                    UiNodeStyle {
                        clip: ClipBehavior::Clip,
                        ..layout::node_style(layout::absolute(
                            14.0 + index as f32 * 42.0,
                            486.0,
                            32.0,
                            28.0,
                        ))
                    },
                )
                .with_input(InputBehavior::BUTTON)
                .with_visual(UiVisual::panel(
                    color,
                    Some(StrokeStyle::new(ColorRgba::new(230, 236, 246, 255), 1.0)),
                    4.0,
                ))
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label(format!("Accent swatch {}", index + 1))
                        .focusable(),
                ),
            );
        }

        let path_state = PathPickerState::new(PathPickerMode::OpenFile, "/workspaces/demo/ui.rs");
        for (index, crumb) in path_state.breadcrumbs().into_iter().take(4).enumerate() {
            button(
                document,
                panel,
                format!("showcase.path.{}", crumb.label),
                crumb.label,
                ButtonOptions {
                    layout: layout::absolute(14.0, 532.0 + index as f32 * 31.0, 214.0, 27.0),
                    text_style: text(12.0, ColorRgba::new(214, 224, 236, 255)),
                    ..Default::default()
                },
            );
        }

        let mut readme =
            TextInputState::new("Selectable status text: copy allowed, editing denied.");
        readme.selection_anchor = Some(0);
        readme.caret = 22;
        selectable_text(
            document,
            panel,
            "showcase.selectable_status",
            &readme,
            TextInputOptions {
                layout: layout::absolute(14.0, 666.0, 232.0, 26.0),
                text_style: text(12.0, ColorRgba::new(226, 233, 241, 255)),
                accessibility_label: Some("Selectable status text".to_string()),
                ..Default::default()
            },
        );
    }

    fn add_workspace(document: &mut UiDocument, root: UiNodeId, viewport: UiSize) {
        let workspace = panel(
            document,
            root,
            "showcase.workspace",
            UiRect::new(292.0, 64.0, 640.0, 704.0),
            "Workspace",
        );

        let select_options = vec![
            SelectOption::new("canvas", "Canvas"),
            SelectOption::new("data", "Data"),
            SelectOption::new("debug", "Debug"),
            SelectOption::new("disabled", "Disabled").disabled(),
        ];
        let mut select_state = SelectMenuState::with_selected(0);
        select_state.open(&select_options);
        dropdown_select(
            document,
            workspace,
            "showcase.workspace_mode",
            &select_options,
            &select_state,
            Some(AnchoredPopup::new(
                UiRect::new(18.0, 44.0, 168.0, 30.0),
                UiRect::new(0.0, 0.0, viewport.width, viewport.height),
                PopupPlacement::new(PopupSide::Bottom, PopupAlign::Start),
            )),
            DropdownSelectOptions {
                trigger_layout: layout::absolute(18.0, 44.0, 168.0, 30.0),
                ..Default::default()
            },
        );

        button(
            document,
            workspace,
            "showcase.primary_action",
            "Run preview",
            ButtonOptions {
                layout: layout::absolute(206.0, 44.0, 126.0, 30.0),
                visual: UiVisual::panel(
                    ColorRgba::new(35, 92, 96, 255),
                    Some(StrokeStyle::new(ColorRgba::new(107, 203, 204, 255), 1.0)),
                    4.0,
                ),
                leading_image: Some(
                    ImageContent::new("icons.play").tinted(ColorRgba::new(148, 238, 218, 255)),
                ),
                text_style: text(13.0, ColorRgba::WHITE),
                action: Some(WidgetActionBinding::command("run.preview")),
                ..Default::default()
            },
        );
        button(
            document,
            workspace,
            "showcase.secondary_action",
            "Profile",
            ButtonOptions {
                layout: layout::absolute(342.0, 44.0, 92.0, 30.0),
                text_style: text(13.0, ColorRgba::new(232, 238, 247, 255)),
                action: Some(WidgetActionBinding::command("run.profile")),
                ..Default::default()
            },
        );
        checkbox(
            document,
            workspace,
            "showcase.snap_grid",
            "Snap",
            true,
            CheckboxOptions {
                layout: layout::absolute(452.0, 45.0, 90.0, 28.0),
                text_style: text(13.0, ColorRgba::new(223, 231, 241, 255)),
                ..Default::default()
            },
        );

        let editor = document.add_child(
            workspace,
            UiNode::container(
                "showcase.editor_card",
                UiNodeStyle {
                    clip: ClipBehavior::Clip,
                    ..layout::node_style(layout::absolute(18.0, 98.0, 604.0, 312.0))
                },
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(12, 17, 22, 255),
                Some(StrokeStyle::new(ColorRgba::new(50, 64, 78, 255), 1.0)),
                5.0,
            ))
            .with_shader(ShaderEffect::new("showcase.editor.layer")),
        );
        add_editor_scene(document, editor);

        timeline_ruler(
            document,
            workspace,
            "showcase.timeline_ruler",
            RulerSpec {
                range: TimelineRange::new(0.0, 32.0),
                width: 604.0,
                major_step: 4.0,
                minor_step: 1.0,
                label_every: 1,
            },
            TimelineRulerOptions {
                layout: layout::absolute(18.0, 422.0, 604.0, 36.0),
                height: 36.0,
                shader: Some(ShaderEffect::new("showcase.timeline.scanline")),
                ..Default::default()
            },
        );

        split_pane(
            document,
            workspace,
            "showcase.split",
            SplitAxis::Horizontal,
            SplitPaneState::new(0.56).with_min_sizes(210.0, 180.0),
            SplitPaneOptions {
                layout: layout::absolute(18.0, 474.0, 604.0, 112.0),
                pane_visual: UiVisual::panel(
                    ColorRgba::new(18, 24, 30, 255),
                    Some(StrokeStyle::new(ColorRgba::new(52, 66, 80, 255), 1.0)),
                    3.0,
                ),
                handle_visual: UiVisual::panel(
                    ColorRgba::new(92, 115, 136, 255),
                    Some(StrokeStyle::new(ColorRgba::new(136, 165, 188, 255), 1.0)),
                    2.0,
                ),
                ..Default::default()
            },
            |document, parent| {
                label(
                    document,
                    parent,
                    "showcase.split.left.label",
                    "Canvas host",
                    text(13.0, ColorRgba::new(230, 237, 247, 255)),
                    layout::absolute(10.0, 8.0, 190.0, 20.0),
                );
                document.add_child(
                    parent,
                    UiNode::canvas(
                        "showcase.canvas_probe",
                        "showcase.routing_graph",
                        layout::absolute(10.0, 34.0, 292.0, 62.0),
                    ),
                );
            },
            |document, parent| {
                label(
                    document,
                    parent,
                    "showcase.split.right.label",
                    "Image/resource slot",
                    text(13.0, ColorRgba::new(230, 237, 247, 255)),
                    layout::absolute(10.0, 8.0, 170.0, 20.0),
                );
                document.add_child(
                    parent,
                    UiNode::image(
                        "showcase.resource_preview",
                        ImageContent::new("images.resource_preview")
                            .tinted(ColorRgba::new(91, 142, 191, 255)),
                        layout::absolute(10.0, 34.0, 220.0, 62.0),
                    ),
                );
            },
        );

        tab_group(
            document,
            workspace,
            "showcase.tabs",
            &[
                TabItem::new("preview", "Preview"),
                TabItem::new("actions", "Actions").dirty(),
                TabItem::new("a11y", "A11y").closable(),
            ],
            TabGroupState::selected(1),
            TabGroupOptions {
                layout: layout::absolute(18.0, 604.0, 604.0, 80.0),
                selected_tab_shader: Some(ShaderEffect::new("showcase.tab.selected")),
                panel_shader: Some(ShaderEffect::new("showcase.tab.panel")),
                ..Default::default()
            },
            |document, parent, index| {
                label(
                    document,
                    parent,
                    "showcase.tab.content",
                    format!(
                        "Selected tab {index}: action routing, focus order, overlay restore, diagnostics"
                    ),
                    text(12.0, ColorRgba::new(216, 226, 238, 255)),
                    layout::absolute(10.0, 12.0, 520.0, 22.0),
                );
            },
        );
    }

    fn add_editor_scene(document: &mut UiDocument, parent: UiNodeId) {
        label(
            document,
            parent,
            "showcase.editor.title",
            "Editor primitives: layers, rich rects, handles, curves",
            text(13.0, ColorRgba::new(232, 239, 247, 255)),
            layout::absolute(14.0, 10.0, 420.0, 20.0),
        );
        label(
            document,
            parent,
            "showcase.editor.badge",
            "GPU compositor metadata attached",
            text(12.0, ColorRgba::new(135, 214, 170, 255)),
            layout::absolute(394.0, 10.0, 190.0, 20.0),
        );

        let transform = EditorTransform::new(UiRect::new(0.0, 0.0, 560.0, 220.0))
            .with_scale(UiPoint::new(12.0, 1.0));
        let arrangement = LaneTimelineGeometry::new(
            transform,
            LaneGeometry::new(34.0, 4)
                .with_origin_y(30.0)
                .with_lane_gap(8.0),
        );
        let timeline = arrangement.timeline;
        let ranges = TimelineRangeItemGeometry::new(arrangement).with_resize_handle_width_px(5.0);
        let curve = CurveEditorGeometry::new(
            timeline,
            EditorAxisRange::new(0.0, 1.0),
            UiRect::new(0.0, 182.0, 560.0, 34.0),
        )
        .with_point_radius_px(4.0);

        let mut scene = vec![ScenePrimitive::Rect(
            PaintRect::new(
                UiRect::new(0.0, 0.0, 560.0, 220.0),
                PaintBrush::LinearGradient(
                    LinearGradient::new(
                        UiPoint::new(0.0, 0.0),
                        UiPoint::new(560.0, 220.0),
                        ColorRgba::new(10, 15, 20, 255),
                        ColorRgba::new(20, 24, 31, 255),
                    )
                    .stop(0.5, ColorRgba::new(12, 24, 26, 255))
                    .fallback(ColorRgba::new(10, 15, 20, 255)),
                ),
            )
            .effect(PaintEffect::shadow(
                ColorRgba::new(0, 0, 0, 86),
                UiPoint::new(0.0, 5.0),
                10.0,
                1.0,
            )),
        )];

        for lane in 0..4 {
            let lane_rect = arrangement
                .view_range_rect(lane, EditorAxisRange::new(0.0, 46.0))
                .expect("lane rect");
            let fill = if lane % 2 == 0 {
                ColorRgba::new(15, 24, 30, 255)
            } else {
                ColorRgba::new(18, 28, 35, 255)
            };
            scene.push(ScenePrimitive::Rect(
                PaintRect::solid(lane_rect, fill)
                    .stroke(StrokeStyle::new(ColorRgba::new(40, 53, 66, 255), 1.0)),
            ));
        }
        for unit in (0..=48).step_by(4) {
            let x = timeline.unit_to_view_x(unit as f32);
            scene.push(ScenePrimitive::Line {
                from: UiPoint::new(x, 24.0),
                to: UiPoint::new(x, 216.0),
                stroke: StrokeStyle::new(
                    if unit % 16 == 0 {
                        ColorRgba::new(73, 92, 111, 255)
                    } else {
                        ColorRgba::new(36, 49, 61, 255)
                    },
                    1.0,
                ),
            });
        }

        let items = [
            (
                TimelineRangeItem::new("range.nav", 0, 2.0, 9.0).selected(true),
                ColorRgba::new(47, 126, 164, 255),
            ),
            (
                TimelineRangeItem::new("range.drag", 1, 13.0, 13.5).dragging(true),
                ColorRgba::new(111, 94, 184, 255),
            ),
            (
                TimelineRangeItem::new("range.disabled", 2, 7.0, 19.0).disabled(true),
                ColorRgba::new(75, 84, 94, 255),
            ),
            (
                TimelineRangeItem::new("range.commit", 3, 29.0, 12.0),
                ColorRgba::new(68, 143, 104, 255),
            ),
        ];
        for (item, fill) in &items {
            let Some(rect) = ranges.item_view_rect(item) else {
                continue;
            };
            scene.push(ScenePrimitive::Rect(
                PaintRect::solid(rect, *fill)
                    .stroke(StrokeStyle::new(ColorRgba::new(187, 210, 222, 255), 1.0))
                    .corner_radii(CornerRadii::uniform(4.0))
                    .effect(PaintEffect::glow(
                        ColorRgba::new(68, 154, 194, 40),
                        6.0,
                        1.0,
                    )),
            ));
            if !item.disabled {
                for edge in [TimelineRangeItemEdge::Start, TimelineRangeItemEdge::End] {
                    let handle = ranges
                        .edge_world_rect(item, edge)
                        .map(|rect| transform.world_to_view_rect(rect))
                        .expect("range handle");
                    scene.push(ScenePrimitive::Rect(PaintRect::solid(
                        handle,
                        ColorRgba::new(232, 239, 247, 220),
                    )));
                }
            }
        }

        scene.push(ScenePrimitive::Rect(PaintRect::solid(
            timeline.playhead_rect(21.25, 24.0, 192.0, 2.0),
            ColorRgba::new(238, 82, 98, 255),
        )));
        scene.push(ScenePrimitive::Rect(
            PaintRect::solid(curve.view_rect, ColorRgba::new(11, 16, 22, 255))
                .stroke(StrokeStyle::new(ColorRgba::new(45, 58, 71, 255), 1.0)),
        ));
        let curve_points = vec![
            CurvePoint::new("curve.a", 2.0, 0.25),
            CurvePoint::new("curve.b", 10.0, 0.72).selected(true),
            CurvePoint::new("curve.c", 22.0, 0.42),
            CurvePoint::new("curve.d", 35.0, 0.9).dragging(true),
            CurvePoint::new("curve.e", 46.0, 0.35),
        ];
        let path_points = curve.segment_view_path(&curve_points, CurveInterpolation::Step);
        let mut path = PaintPath::new();
        if let Some(first) = path_points.first().copied() {
            path = path.move_to(first);
            for point in path_points.into_iter().skip(1) {
                path = path.line_to(point);
            }
        }
        scene.push(ScenePrimitive::Path(
            path.stroke(StrokeStyle::new(ColorRgba::new(238, 190, 86, 255), 2.0)),
        ));
        for point in &curve_points {
            scene.push(ScenePrimitive::Circle {
                center: curve.point_view_position(point),
                radius: if point.selected || point.dragging {
                    5.0
                } else {
                    4.0
                },
                fill: if point.dragging {
                    ColorRgba::new(255, 232, 135, 255)
                } else {
                    ColorRgba::new(238, 190, 86, 255)
                },
                stroke: Some(StrokeStyle::new(ColorRgba::new(52, 40, 21, 255), 1.0)),
            });
        }

        document.add_child(
            parent,
            UiNode::scene(
                "showcase.editor.scene",
                scene,
                layout::absolute(22.0, 42.0, 560.0, 220.0),
            )
            .with_accessibility(
                EditorSurfaceAccessibility::new("Showcase editor surface")
                    .description("Reusable range, lane, curve, and canvas style geometry")
                    .visible_units(EditorAxisRange::new(0.0, 48.0))
                    .visible_lanes(VisibleLaneRange::new(0, 4))
                    .target_count(12)
                    .selected_count(2)
                    .active("range.drag")
                    .instruction("Drag ranges and curve points, or use arrow keys")
                    .accessibility_meta(),
            ),
        );
    }

    fn add_right_panel(document: &mut UiDocument, root: UiNodeId, viewport: UiSize) {
        let panel = panel(
            document,
            root,
            "showcase.right",
            UiRect::new(948.0, 64.0, 316.0, 704.0),
            "Interaction + Data",
        );

        label(
            document,
            panel,
            "showcase.controls.title",
            "Actions and text policies",
            text(13.0, ColorRgba::new(234, 239, 247, 255)),
            layout::absolute(14.0, 44.0, 200.0, 20.0),
        );
        button(
            document,
            panel,
            "showcase.clickable_text_button",
            "Text is inside a clickable button",
            ButtonOptions {
                layout: layout::absolute(14.0, 70.0, 236.0, 32.0),
                text_style: text(13.0, ColorRgba::WHITE),
                visual: UiVisual::panel(
                    ColorRgba::new(75, 67, 135, 255),
                    Some(StrokeStyle::new(ColorRgba::new(161, 147, 232, 255), 1.0)),
                    4.0,
                ),
                accessibility_hint: Some(
                    "Clicking the text child still activates the button".to_string(),
                ),
                ..Default::default()
            },
        );
        slider(
            document,
            panel,
            "showcase.opacity",
            0.68,
            0.0..1.0,
            SliderOptions {
                layout: layout::absolute(14.0, 116.0, 224.0, 30.0),
                fill_shader: Some(ShaderEffect::new("showcase.slider.fill")),
                thumb_shader: Some(ShaderEffect::new("showcase.slider.thumb")),
                ..Default::default()
            },
        );
        let mut editable = TextInputState::new("Editable value");
        editable.selection_anchor = Some(0);
        editable.caret = 8;
        text_input(
            document,
            panel,
            "showcase.editable_text",
            &editable,
            TextInputOptions {
                layout: layout::absolute(14.0, 158.0, 224.0, 32.0),
                focused: true,
                placeholder: "Edit text".to_string(),
                text_style: text(13.0, ColorRgba::new(236, 241, 246, 255)),
                ..Default::default()
            },
        );
        selectable_text(
            document,
            panel,
            "showcase.read_only_text",
            &TextInputState::new("Read-only, selectable, copyable"),
            TextInputOptions {
                layout: layout::absolute(14.0, 200.0, 250.0, 30.0),
                text_style: text(12.0, ColorRgba::new(224, 232, 242, 255)),
                accessibility_hint: Some("Use Ctrl+C after selecting text".to_string()),
                ..Default::default()
            },
        );

        let text_plan_state = {
            let mut state = TextInputState::new("Selection paint + caret plan");
            state.selection_anchor = Some(0);
            state.caret = 15;
            state
        };
        let text_plan = text_plan_state.render_plan(
            TextInputLayoutMetrics::new(UiRect::new(8.0, 7.0, 232.0, 18.0), 7.0, 18.0),
            text(12.0, ColorRgba::new(236, 241, 246, 255)),
            TextInputPaintOptions::default(),
        );
        document.add_child(
            panel,
            UiNode::scene(
                "showcase.text_render_plan",
                text_plan.scene_primitives(),
                layout::absolute(14.0, 240.0, 250.0, 34.0),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(18, 23, 29, 255),
                Some(StrokeStyle::new(ColorRgba::new(64, 78, 94, 255), 1.0)),
                4.0,
            ))
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::TextBox)
                    .label("Text render plan")
                    .value("Selection paint plus caret plan")
                    .read_only()
                    .focusable(),
            ),
        );

        label(
            document,
            panel,
            "showcase.table.title",
            "Virtualized table",
            text(13.0, ColorRgba::new(234, 239, 247, 255)),
            layout::absolute(14.0, 296.0, 180.0, 20.0),
        );
        virtualized_data_table(
            document,
            panel,
            "showcase.table",
            &[
                DataTableColumn::new("node", "Node", 116.0),
                DataTableColumn::new("z", "Z", 42.0).with_alignment(DataCellAlignment::End),
                DataTableColumn::new("state", "State", 86.0),
            ],
            VirtualDataTableSpec {
                row_count: 600,
                row_height: 24.0,
                viewport_width: 274.0,
                viewport_height: 142.0,
                scroll_offset: UiPoint::new(0.0, 384.0),
                overscan_rows: 2,
            },
            DataTableOptions {
                layout: layout::absolute(14.0, 322.0, 274.0, 142.0),
                selection: DataTableSelection::single_row(16)
                    .with_active_cell(DataTableCellIndex::new(16, 2)),
                selected_row_shader: Some(ShaderEffect::new("showcase.table.row")),
                active_cell_shader: Some(ShaderEffect::new("showcase.table.cell")),
                ..Default::default()
            },
            |document, parent, cell| {
                let value = match cell.column {
                    0 => format!("node.{}", cell.row),
                    1 => format!("{}", cell.row % 8),
                    _ => match cell.row % 3 {
                        0 => "dirty".to_string(),
                        1 => "cached".to_string(),
                        _ => "idle".to_string(),
                    },
                };
                document.add_child(
                    parent,
                    UiNode::text(
                        format!("showcase.table.cell.{}.{}", cell.row, cell.column),
                        value,
                        text(11.0, ColorRgba::new(224, 232, 242, 255)),
                        layout::fixed(100.0, 19.0),
                    ),
                );
            },
        );

        label(
            document,
            panel,
            "showcase.scroll.title",
            "Scroll surface",
            text(13.0, ColorRgba::new(234, 239, 247, 255)),
            layout::absolute(14.0, 486.0, 180.0, 20.0),
        );
        let scroll = scroll_area(
            document,
            panel,
            "showcase.scroll_area",
            ScrollAxes::VERTICAL,
            layout::absolute(14.0, 512.0, 274.0, 82.0),
        );
        for index in 0..6 {
            label(
                document,
                scroll,
                format!("showcase.scroll.row.{index}"),
                format!("Focusable row with stable id #{index}"),
                text(12.0, ColorRgba::new(211, 222, 234, 255)),
                layout::absolute(8.0, 7.0 + index as f32 * 24.0, 236.0, 20.0),
            );
        }

        let context_items = vec![
            MenuItem::command("selection.rename", "Rename").shortcut("F2"),
            MenuItem::command("selection.duplicate", "Duplicate").shortcut("Ctrl+D"),
            MenuItem::check("selection.locked", "Locked", false),
            MenuItem::separator(),
            MenuItem::command("selection.delete", "Delete").destructive(),
        ];
        let mut context_state = ContextMenuState::open_at(UiPoint::new(1038.0, 684.0));
        context_state.active = Some(1);
        context_menu(
            document,
            root,
            "showcase.context_menu",
            &context_items,
            &context_state,
            UiRect::new(0.0, 0.0, viewport.width, viewport.height),
            PopupPlacement::new(PopupSide::Right, PopupAlign::Start),
            MenuListOptions {
                width: 210.0,
                active_shader: Some(ShaderEffect::new("showcase.context.active")),
                ..Default::default()
            },
        );

        let mut toasts = ToastStack::new(3);
        toasts.push(
            ToastSeverity::Success,
            "Frame ready",
            Some("Paint and accessibility records generated".to_string()),
            None,
        );
        toasts.push(
            ToastSeverity::Info,
            "Right-click",
            Some("Context menu can open from pointer or keyboard".to_string()),
            None,
        );
        toast_stack(
            document,
            panel,
            "showcase.toasts",
            &toasts,
            ToastStackOptions {
                layout: layout::absolute(14.0, 616.0, 274.0, 70.0),
                ..Default::default()
            },
        );
    }

    fn add_global_overlays(document: &mut UiDocument, root: UiNodeId, viewport: UiSize) {
        let commands = vec![
            CommandPaletteItem::new("run.preview", "Run preview")
                .subtitle("Workspace")
                .shortcut("Space")
                .keyword("play"),
            CommandPaletteItem::new("selection.rename", "Rename selection")
                .subtitle("Edit")
                .shortcut("F2"),
            CommandPaletteItem::new("view.toggle_grid", "Toggle grid")
                .subtitle("View")
                .shortcut("G"),
            CommandPaletteItem::new("diagnostics.open", "Open diagnostics")
                .subtitle("Tools")
                .shortcut("Ctrl+Shift+D"),
            CommandPaletteItem::new("render.snapshot", "Render snapshot")
                .subtitle("Export")
                .shortcut("Ctrl+E")
                .disabled(),
        ];
        let mut palette_state = CommandPaletteState::new().with_query("re");
        palette_state.move_active(&commands, NavigationDirection::Next);
        command_palette(
            document,
            root,
            "showcase.command_palette",
            &commands,
            &palette_state,
            Some(AnchoredPopup::new(
                UiRect::new(366.0, 50.0, 548.0, 34.0),
                UiRect::new(0.0, 0.0, viewport.width, viewport.height),
                PopupPlacement::new(PopupSide::Bottom, PopupAlign::Center)
                    .with_viewport_margin(16.0),
            )),
            CommandPaletteOptions {
                panel_shader: Some(ShaderEffect::new("showcase.palette.panel")),
                active_row_shader: Some(ShaderEffect::new("showcase.palette.active")),
                ..Default::default()
            },
        );
    }

    fn panel(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: &'static str,
        rect: UiRect,
        title: &'static str,
    ) -> UiNodeId {
        let node = document.add_child(
            parent,
            UiNode::container(
                name,
                UiNodeStyle {
                    clip: ClipBehavior::Clip,
                    ..layout::node_style(layout::absolute(rect.x, rect.y, rect.width, rect.height))
                },
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(16, 21, 27, 244),
                Some(StrokeStyle::new(ColorRgba::new(51, 63, 77, 255), 1.0)),
                6.0,
            ))
            .with_shader(ShaderEffect::new(format!("{name}.surface")).uniform("elevation", 2.0))
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).label(title)),
        );
        label(
            document,
            node,
            format!("{name}.title"),
            title,
            text(14.0, ColorRgba::new(240, 245, 250, 255)),
            layout::absolute(14.0, 12.0, rect.width - 28.0, 22.0),
        );
        node
    }

    fn absolute_row(x: f32, y: f32, width: f32, height: f32) -> LayoutStyle {
        layout::with_absolute_position(
            layout::with_size(layout::row(), layout::px(width), layout::px(height)),
            x,
            y,
        )
    }

    fn text(font_size: f32, color: ColorRgba) -> TextStyle {
        TextStyle {
            font_size,
            line_height: font_size + 4.0,
            color,
            ..Default::default()
        }
    }
}
