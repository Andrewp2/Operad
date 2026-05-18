use operad::{root_style, ColorRgba, LayoutStyle, TextStyle, UiDocument, UiNode, UiSize, UiVisual};

fn main() -> operad::native::NativeWindowResult {
    operad::native::run("Minimal Operad", minimal_document)
}

fn minimal_document(viewport: UiSize) -> UiDocument {
    let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
    let panel = ui.add_child(
        ui.root(),
        UiNode::container(
            "app.panel",
            LayoutStyle::column()
                .with_size(360.0, 120.0)
                .with_padding(20.0)
                .with_gap(8.0),
        )
        .with_visual(UiVisual::panel(ColorRgba::new(24, 29, 36, 255), None, 6.0)),
    );
    ui.add_child(
        panel,
        UiNode::text(
            "app.title",
            "Hello from Operad",
            TextStyle {
                font_size: 22.0,
                line_height: 30.0,
                color: ColorRgba::WHITE,
                ..TextStyle::default()
            },
            LayoutStyle::size(320.0, 34.0),
        ),
    );
    ui.add_child(
        panel,
        UiNode::text(
            "app.subtitle",
            "This app uses the default native runtime.",
            TextStyle::default(),
            LayoutStyle::size(320.0, 28.0),
        ),
    );
    ui
}
