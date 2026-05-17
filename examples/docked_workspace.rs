use operad::widgets::{self, DockPanelDescriptor, DockSide, DockWorkspaceOptions};
use operad::{
    root_style, ColorRgba, LayoutStyle, StrokeStyle, TextStyle, UiDocument, UiNode, UiSize,
    UiVisual,
};

fn main() -> operad::NativeWindowResult {
    operad::run("Docked workspace", workspace_document)
}

fn workspace_document(viewport: UiSize) -> UiDocument {
    let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
    let root = ui.add_child(
        ui.root,
        UiNode::container(
            "workspace.root",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0)
                .with_padding(12.0),
        )
        .with_visual(UiVisual::panel(ColorRgba::new(13, 17, 23, 255), None, 0.0)),
    );

    let panels = [
        DockPanelDescriptor::new("assets", "Assets", DockSide::Left, 190.0)
            .with_min_size(140.0)
            .resizable(true),
        DockPanelDescriptor::center("editor", "Editor").with_min_size(220.0),
        DockPanelDescriptor::new("inspector", "Inspector", DockSide::Right, 220.0)
            .with_min_size(170.0)
            .resizable(true),
        DockPanelDescriptor::new("timeline", "Timeline", DockSide::Bottom, 110.0)
            .with_min_size(80.0)
            .resizable(true),
    ];

    let mut options = DockWorkspaceOptions::default();
    options.layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_height_percent(1.0);
    options.panel_visual = UiVisual::panel(
        ColorRgba::new(24, 29, 36, 255),
        Some(StrokeStyle::new(ColorRgba::new(58, 68, 84, 255), 1.0)),
        0.0,
    );
    options.center_visual = UiVisual::panel(
        ColorRgba::new(16, 21, 28, 255),
        Some(StrokeStyle::new(ColorRgba::new(58, 68, 84, 255), 1.0)),
        0.0,
    );

    widgets::dock_workspace(&mut ui, root, "workspace", &panels, options, panel_body);
    ui
}

fn panel_body(ui: &mut UiDocument, parent: operad::UiNodeId, panel: &DockPanelDescriptor) {
    match panel.id.as_str() {
        "assets" => {
            panel_label(ui, parent, "workspace.assets.one", "models/scene.glb");
            panel_label(ui, parent, "workspace.assets.two", "textures/albedo.png");
            panel_label(ui, parent, "workspace.assets.three", "materials/metal.toml");
        }
        "editor" => {
            panel_heading(ui, parent, "workspace.editor.title", "Scene editor");
            panel_label(
                ui,
                parent,
                "workspace.editor.body",
                "The center panel fills remaining space.",
            );
        }
        "inspector" => {
            panel_heading(ui, parent, "workspace.inspector.title", "Selected object");
            panel_label(
                ui,
                parent,
                "workspace.inspector.position",
                "Position: 12, 4, -8",
            );
            panel_label(
                ui,
                parent,
                "workspace.inspector.material",
                "Material: brushed steel",
            );
        }
        "timeline" => {
            panel_label(ui, parent, "workspace.timeline.one", "Frame 0000");
            panel_label(ui, parent, "workspace.timeline.two", "Frame 0120");
        }
        _ => {}
    }
}

fn panel_heading(ui: &mut UiDocument, parent: operad::UiNodeId, name: &str, text: &str) {
    widgets::label(
        ui,
        parent,
        name,
        text,
        TextStyle {
            font_size: 16.0,
            line_height: 22.0,
            color: ColorRgba::WHITE,
            ..TextStyle::default()
        },
        LayoutStyle::new().with_width_percent(1.0).with_height(28.0),
    );
}

fn panel_label(ui: &mut UiDocument, parent: operad::UiNodeId, name: &str, text: &str) {
    widgets::label(
        ui,
        parent,
        name,
        text,
        TextStyle::default(),
        LayoutStyle::new().with_width_percent(1.0).with_height(24.0),
    );
}
