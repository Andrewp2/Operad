use operad::native::{NativeWindowOptions, NativeWindowResult};
use operad::tooltips::ShortcutFormatter;
use operad::widgets;
use operad::widgets::ext::{self as ext_widgets, CommandPaletteItem, CommandPaletteState};
use operad::{
    root_style, ColorRgba, CommandMeta, CommandRegistry, CommandScope, LayoutStyle, Shortcut,
    TextStyle, UiDocument, UiNode, UiSize, UiVisual, WidgetAction, WidgetActionKind,
};

fn main() -> NativeWindowResult {
    operad::native::run_app_with(
        NativeWindowOptions::new("Command palette").with_min_size(620.0, 420.0),
        CommandApp::default(),
        CommandApp::update,
        CommandApp::view,
    )
}

struct CommandApp {
    palette: CommandPaletteState,
    last_command: String,
}

impl Default for CommandApp {
    fn default() -> Self {
        Self {
            palette: CommandPaletteState::new(),
            last_command: "None".to_string(),
        }
    }
}

impl CommandApp {
    fn update(&mut self, action: WidgetAction) {
        let Some(action_id) = action.binding.action_id().map(|id| id.as_str()) else {
            return;
        };
        let items = command_items();
        if action_id == "commands.search" {
            if let WidgetActionKind::TextEdit(edit) = &action.kind {
                if edit.local_position.is_none() {
                    if let Some(selection) = self.palette.handle_event(&items, &edit.event).selected
                    {
                        self.last_command = selection.id;
                    }
                }
            }
        } else if let Some(command) = action_id.strip_prefix("commands.item.") {
            self.last_command = command.to_string();
        }
    }

    fn view(&self, viewport: UiSize) -> UiDocument {
        let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
        let panel = ui.add_child(
            ui.root(),
            UiNode::container(
                "commands.panel",
                LayoutStyle::column()
                    .with_size(560.0, 360.0)
                    .with_padding(16.0)
                    .with_gap(10.0),
            )
            .with_visual(UiVisual::panel(ColorRgba::new(24, 29, 36, 255), None, 6.0)),
        );
        widgets::label(
            &mut ui,
            panel,
            "commands.title",
            "Command palette and shortcuts",
            heading(),
            LayoutStyle::new().with_width_percent(1.0).with_height(32.0),
        );
        let mut options =
            ext_widgets::CommandPaletteOptions::default().with_action_prefix("commands");
        options.width = 520.0;
        options.max_visible_rows = 5;
        ext_widgets::command_palette(
            &mut ui,
            panel,
            "commands.palette",
            &command_items(),
            &self.palette,
            None,
            options,
        );
        widgets::label(
            &mut ui,
            panel,
            "commands.last",
            format!("Last command: {}", self.last_command),
            muted(),
            LayoutStyle::new().with_width_percent(1.0).with_height(28.0),
        );
        ui
    }
}

fn command_items() -> Vec<CommandPaletteItem> {
    let mut registry = CommandRegistry::new();
    registry
        .register(
            CommandMeta::new("app.open", "Open project")
                .description("Open an existing project")
                .category("File"),
        )
        .expect("register command");
    registry
        .register(
            CommandMeta::new("app.save", "Save project")
                .description("Write current changes")
                .category("File"),
        )
        .expect("register command");
    registry
        .register(
            CommandMeta::new("app.toggle_sidebar", "Toggle sidebar")
                .description("Show or hide the left navigation")
                .category("View"),
        )
        .expect("register command");
    registry
        .bind_shortcut(CommandScope::Global, Shortcut::ctrl('o'), "app.open")
        .expect("bind shortcut");
    registry
        .bind_shortcut(CommandScope::Global, Shortcut::ctrl('s'), "app.save")
        .expect("bind shortcut");
    registry
        .bind_shortcut(
            CommandScope::Global,
            Shortcut::ctrl('b'),
            "app.toggle_sidebar",
        )
        .expect("bind shortcut");
    ext_widgets::command_palette::command_palette_items_from_registry(
        &registry,
        &[CommandScope::Global],
        &ShortcutFormatter::default(),
    )
}

fn heading() -> TextStyle {
    TextStyle {
        font_size: 22.0,
        line_height: 30.0,
        color: ColorRgba::WHITE,
        ..TextStyle::default()
    }
}

fn muted() -> TextStyle {
    TextStyle {
        color: ColorRgba::new(166, 178, 196, 255),
        ..TextStyle::default()
    }
}
