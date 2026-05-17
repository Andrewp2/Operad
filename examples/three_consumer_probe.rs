use operad::{
    layout, root_style, AccessibilityAction, AccessibilityMeta, AccessibilityRole,
    ApproxTextMeasurer, ClipBehavior, ColorRgba, InputBehavior, ScrollAxes, StrokeStyle, TextStyle,
    UiDocument, UiInputEvent, UiNode, UiNodeStyle, UiPoint, UiSize, UiVisual,
};

fn main() -> Result<(), String> {
    let probes = [
        ("game_overlay", build_game_overlay()),
        ("tool_panel", build_tool_panel()),
        ("timeline_editor", build_timeline_editor()),
    ];

    for (name, mut document) in probes {
        document
            .compute_layout(UiSize::new(800.0, 600.0), &mut ApproxTextMeasurer)
            .map_err(|error| format!("{name} layout failed: {error}"))?;
        let warnings = document.audit_layout();
        if !warnings.is_empty() {
            return Err(format!(
                "{name} should have no basic layout audit warnings: {warnings:#?}"
            ));
        }
        if document.paint_list().is_empty() {
            return Err(format!("{name} should produce renderer-neutral paint"));
        }
        println!("{name}: {} paint items", document.paint_list().items.len());
    }
    Ok(())
}

fn build_game_overlay() -> UiDocument {
    let mut document = UiDocument::new(root_style(800.0, 600.0));
    let hotbar = document.add_child(
        document.root,
        UiNode::container(
            "game.hotbar",
            UiNodeStyle {
                clip: ClipBehavior::Clip,
                z_index: 10,
                ..layout::node_style(layout::with_margin_bottom(
                    layout::with_auto_horizontal_margin(layout::with_size(
                        layout::centered_row(),
                        layout::px(360.0),
                        layout::px(64.0),
                    )),
                    18.0,
                ))
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(20, 24, 31, 230),
            Some(StrokeStyle::new(ColorRgba::new(96, 113, 139, 255), 1.0)),
            6.0,
        )),
    );

    for slot in 0..8 {
        document.add_child(
            hotbar,
            UiNode::container(
                format!("game.hotbar.slot.{slot}"),
                layout::node_style(layout::with_margin_all(layout::fixed(36.0, 36.0), 4.0)),
            )
            .with_input(InputBehavior::BUTTON)
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label(format!("Hotbar slot {}", slot + 1))
                    .focusable()
                    .action(AccessibilityAction::new("activate", "Activate")),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(40, 49, 62, 255),
                Some(StrokeStyle::new(ColorRgba::new(105, 124, 153, 255), 1.0)),
                4.0,
            )),
        );
    }

    document
}

fn build_tool_panel() -> UiDocument {
    let mut document = UiDocument::new(root_style(800.0, 600.0));
    let panel = document.add_child(
        document.root,
        UiNode::container(
            "tool.sidebar.modules",
            UiNodeStyle {
                clip: ClipBehavior::Clip,
                ..layout::node_style(layout::with_size(
                    layout::column(),
                    layout::px(260.0),
                    layout::px(220.0),
                ))
            },
        )
        .with_scroll(ScrollAxes::VERTICAL)
        .with_visual(UiVisual::panel(
            ColorRgba::new(28, 33, 39, 255),
            Some(StrokeStyle::new(ColorRgba::new(74, 85, 104, 255), 1.0)),
            4.0,
        )),
    );

    for row in 0..12 {
        let label = format!("Layer module {}", row + 1);
        document.add_child(
            panel,
            UiNode::text(
                format!("tool.module.{row}"),
                label.clone(),
                TextStyle::default(),
                layout::size(layout::percent(1.0), layout::px(32.0)),
            )
            .with_input(InputBehavior::BUTTON)
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label(label)
                    .focusable()
                    .action(AccessibilityAction::new("activate", "Activate")),
            ),
        );
    }

    document.handle_input(UiInputEvent::wheel(
        UiPoint::new(12.0, 12.0),
        UiPoint::new(0.0, 24.0),
    ));

    document
}

fn build_timeline_editor() -> UiDocument {
    let mut document = UiDocument::new(root_style(800.0, 600.0));
    let shell = document.add_child(
        document.root,
        UiNode::container(
            "timeline.shell",
            UiNodeStyle {
                clip: ClipBehavior::Clip,
                ..layout::node_style(layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::percent(1.0),
                ))
            },
        ),
    );

    document.add_child(
        shell,
        UiNode::text(
            "timeline.transport",
            "Transport",
            TextStyle::default(),
            layout::size(layout::percent(1.0), layout::px(32.0)),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(30, 36, 44, 255),
            Some(StrokeStyle::new(ColorRgba::new(84, 96, 115, 255), 1.0)),
            0.0,
        )),
    );
    document.add_child(
        shell,
        UiNode::canvas(
            "timeline.editor",
            "timeline.editor.display_list_surface",
            layout::size(layout::percent(1.0), layout::px(260.0)),
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::EditorSurface)
                .label("Timeline editor")
                .focusable(),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(16, 19, 23, 255),
            Some(StrokeStyle::new(ColorRgba::new(64, 75, 92, 255), 1.0)),
            0.0,
        )),
    );

    document
}
