use operad::{
    layout, root_style, ApproxTextMeasurer, ClipBehavior, ColorRgba, InputBehavior, ScrollAxes,
    StrokeStyle, TextStyle, UiDocument, UiInputEvent, UiNode, UiNodeStyle, UiPoint, UiSize,
    UiVisual,
};

fn main() {
    let probes = [
        ("game_hud", build_game_hud()),
        ("fabricad_panel", build_fabricad_panel()),
        ("orbifold_editor", build_orbifold_editor()),
    ];

    for (name, mut document) in probes {
        document
            .compute_layout(UiSize::new(800.0, 600.0), &mut ApproxTextMeasurer)
            .expect("probe layout should compute");
        assert!(
            document.audit_layout().is_empty(),
            "{name} should have no basic layout audit warnings"
        );
        assert!(
            !document.paint_list().is_empty(),
            "{name} should produce renderer-neutral paint"
        );
        println!("{name}: {} paint items", document.paint_list().items.len());
    }
}

fn build_game_hud() -> UiDocument {
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
            .with_visual(UiVisual::panel(
                ColorRgba::new(40, 49, 62, 255),
                Some(StrokeStyle::new(ColorRgba::new(105, 124, 153, 255), 1.0)),
                4.0,
            )),
        );
    }

    document
}

fn build_fabricad_panel() -> UiDocument {
    let mut document = UiDocument::new(root_style(800.0, 600.0));
    let panel = document.add_child(
        document.root,
        UiNode::container(
            "fabricad.sidebar.modules",
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
        document.add_child(
            panel,
            UiNode::text(
                format!("fabricad.module.{row}"),
                format!("Layer module {row}"),
                TextStyle::default(),
                layout::size(layout::percent(1.0), layout::px(32.0)),
            )
            .with_input(InputBehavior::BUTTON),
        );
    }

    document.handle_input(UiInputEvent::wheel(
        UiPoint::new(12.0, 12.0),
        UiPoint::new(0.0, 24.0),
    ));

    document
}

fn build_orbifold_editor() -> UiDocument {
    let mut document = UiDocument::new(root_style(800.0, 600.0));
    let shell = document.add_child(
        document.root,
        UiNode::container(
            "orbifold.shell",
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
            "orbifold.transport",
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
            "orbifold.piano_roll",
            "orbifold.piano_roll.display_list_surface",
            layout::size(layout::percent(1.0), layout::px(260.0)),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(16, 19, 23, 255),
            Some(StrokeStyle::new(ColorRgba::new(64, 75, 92, 255), 1.0)),
            0.0,
        )),
    );

    document
}
