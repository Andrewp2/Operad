use operad::debug::DebugThemeSnapshot;
use operad::native::NativeWindowResult;
use operad::widgets;
use operad::widgets::ext::{self as ext_widgets, ThemeEditorPanelOptions};
use operad::{
    root_style, ColorRgba, LayoutStyle, StrokeStyle, TextStyle, Theme, UiDocument, UiNode, UiSize,
    UiVisual,
};

fn main() -> NativeWindowResult {
    operad::native::run("Theme customization", theme_document)
}

fn theme_document(viewport: UiSize) -> UiDocument {
    let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
    let panel = ui.add_child(
        ui.root(),
        UiNode::container(
            "theme.panel",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0)
                .with_padding(16.0)
                .with_gap(12.0),
        )
        .with_visual(UiVisual::panel(ColorRgba::new(13, 17, 23, 255), None, 0.0)),
    );

    widgets::label(
        &mut ui,
        panel,
        "theme.title",
        "Theme customization",
        TextStyle {
            font_size: 22.0,
            line_height: 30.0,
            color: ColorRgba::WHITE,
            ..TextStyle::default()
        },
        LayoutStyle::new().with_width_percent(1.0).with_height(34.0),
    );

    let theme = Theme::dark();
    let snapshot = DebugThemeSnapshot::from_theme(&theme);
    let mut options = ThemeEditorPanelOptions::default().with_action_prefix("theme");
    options.layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_height(0.0)
        .with_flex_grow(1.0)
        .with_gap(8.0);
    options.max_token_rows = 18;
    options.max_component_rows = 10;
    options.label_width = 220.0;
    options.row_height = 28.0;

    let editor = ui.add_child(
        panel,
        UiNode::container(
            "theme.editor.surface",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(0.0)
                .with_flex_grow(1.0)
                .with_padding(12.0),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(24, 29, 36, 255),
            Some(StrokeStyle::new(ColorRgba::new(58, 68, 84, 255), 1.0)),
            6.0,
        )),
    );

    ext_widgets::theme_editor_panel(&mut ui, editor, "theme.editor", &snapshot, options);
    ui
}
