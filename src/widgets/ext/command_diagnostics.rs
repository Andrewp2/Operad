//! Command registry diagnostics widgets.

use taffy::prelude::{
    Dimension, Display, FlexDirection, LengthPercentage, Size as TaffySize, Style,
};

use crate::commands::{ShortcutBinding, ShortcutConflict};
use crate::tooltips::ShortcutFormatter;
use crate::{
    ColorRgba, Command, CommandId, CommandRegistry, CommandScope, LayoutStyle, TextStyle,
    UiDocument, UiNode, UiNodeId, UiNodeStyle,
};

use super::data::PropertyValueKind;
use super::property_inspector::{
    property_inspector_grid, PropertyGridRow, PropertyInspectorOptions,
};

#[derive(Debug, Clone)]
pub struct CommandDiagnosticsPanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_command_rows: usize,
    pub max_conflict_rows: usize,
    pub action_prefix: Option<String>,
    pub title_style: TextStyle,
}

impl Default for CommandDiagnosticsPanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                gap: taffy::geometry::Size {
                    width: LengthPercentage::length(8.0),
                    height: LengthPercentage::length(8.0),
                },
                ..Default::default()
            }),
            label_width: 190.0,
            row_height: 26.0,
            max_command_rows: 32,
            max_conflict_rows: 8,
            action_prefix: None,
            title_style: TextStyle {
                font_size: 14.0,
                line_height: 20.0,
                color: ColorRgba::new(238, 243, 248, 255),
                ..Default::default()
            },
        }
    }
}

impl CommandDiagnosticsPanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandDiagnosticsPanelNodes {
    pub root: UiNodeId,
    pub command_grid: UiNodeId,
    pub conflict_grid: UiNodeId,
}

pub fn command_diagnostics_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    registry: &CommandRegistry,
    active_scopes: &[CommandScope],
    formatter: &ShortcutFormatter,
    options: CommandDiagnosticsPanelOptions,
) -> CommandDiagnosticsPanelNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                ..Default::default()
            },
        ),
    );

    document.add_child(
        root,
        UiNode::text(
            format!("{name}.commands.title"),
            "Commands",
            options.title_style.clone(),
            LayoutStyle::new().with_width_percent(1.0).with_height(24.0),
        ),
    );
    let command_grid = property_inspector_grid(
        document,
        root,
        format!("{name}.commands"),
        &command_rows(registry, active_scopes, formatter, options.max_command_rows),
        grid_options(
            options.label_width,
            options.row_height,
            options
                .action_prefix
                .as_deref()
                .map(|prefix| format!("{prefix}.command")),
            "Command registry",
        ),
    );

    document.add_child(
        root,
        UiNode::text(
            format!("{name}.conflicts.title"),
            "Shortcut conflicts",
            TextStyle {
                color: ColorRgba::new(171, 183, 201, 255),
                ..options.title_style
            },
            LayoutStyle::new().with_width_percent(1.0).with_height(24.0),
        ),
    );
    let conflict_grid = property_inspector_grid(
        document,
        root,
        format!("{name}.conflicts"),
        &conflict_rows(registry, formatter, options.max_conflict_rows),
        grid_options(
            options.label_width,
            options.row_height,
            options
                .action_prefix
                .as_deref()
                .map(|prefix| format!("{prefix}.conflict")),
            "Shortcut conflicts",
        ),
    );

    CommandDiagnosticsPanelNodes {
        root,
        command_grid,
        conflict_grid,
    }
}

fn grid_options(
    label_width: f32,
    row_height: f32,
    action_prefix: Option<String>,
    accessibility_label: impl Into<String>,
) -> PropertyInspectorOptions {
    PropertyInspectorOptions {
        label_width,
        row_height,
        action_prefix,
        accessibility_label: Some(accessibility_label.into()),
        ..Default::default()
    }
}

fn command_rows(
    registry: &CommandRegistry,
    active_scopes: &[CommandScope],
    formatter: &ShortcutFormatter,
    limit: usize,
) -> Vec<PropertyGridRow> {
    let mut commands = registry.commands().collect::<Vec<_>>();
    commands.sort_by(|left, right| left.meta.id.cmp(&right.meta.id));
    let rows = commands
        .into_iter()
        .take(limit)
        .map(|command| command_row(registry, command, active_scopes, formatter))
        .collect::<Vec<_>>();

    if rows.is_empty() {
        vec![PropertyGridRow::new("empty", "Commands", "No commands").read_only()]
    } else {
        rows
    }
}

fn command_row(
    registry: &CommandRegistry,
    command: &Command,
    active_scopes: &[CommandScope],
    formatter: &ShortcutFormatter,
) -> PropertyGridRow {
    let shortcut = command_shortcut(registry, &command.meta.id, active_scopes, formatter)
        .unwrap_or_else(|| "unbound".to_owned());
    let mut value = vec![shortcut];
    if let Some(category) = &command.meta.category {
        value.push(format!("category={category}"));
    }
    if let Some(description) = &command.meta.description {
        value.push(description.clone());
    }
    if !command.enabled {
        value.push(
            command
                .disabled_reason
                .clone()
                .unwrap_or_else(|| "disabled".to_owned()),
        );
    }

    let row = PropertyGridRow::new(
        sanitize_row_id(command.meta.id.as_str()),
        command.meta.label.clone(),
        compact_value(value.join("; "), 40),
    )
    .with_kind(PropertyValueKind::Custom);

    if command.enabled {
        row
    } else {
        row.disabled()
    }
}

fn conflict_rows(
    registry: &CommandRegistry,
    formatter: &ShortcutFormatter,
    limit: usize,
) -> Vec<PropertyGridRow> {
    let rows = registry
        .conflicts()
        .iter()
        .take(limit)
        .map(|conflict| conflict_row(conflict, formatter))
        .collect::<Vec<_>>();

    if rows.is_empty() {
        vec![PropertyGridRow::new("none", "Conflicts", "No shortcut conflicts").read_only()]
    } else {
        rows
    }
}

fn conflict_row(conflict: &ShortcutConflict, formatter: &ShortcutFormatter) -> PropertyGridRow {
    PropertyGridRow::new(
        sanitize_row_id(format!(
            "{:?}.{}",
            conflict.scope,
            formatter.format(conflict.shortcut)
        )),
        format!(
            "{:?} {}",
            conflict.scope,
            formatter.format(conflict.shortcut)
        ),
        conflict
            .commands
            .iter()
            .map(CommandId::as_str)
            .collect::<Vec<_>>()
            .join(", "),
    )
    .warning("Multiple commands use this shortcut in the same scope")
    .read_only()
}

fn command_shortcut(
    registry: &CommandRegistry,
    command: &CommandId,
    active_scopes: &[CommandScope],
    formatter: &ShortcutFormatter,
) -> Option<String> {
    registry
        .bindings()
        .iter()
        .filter(|binding| &binding.command == command)
        .min_by_key(|binding| shortcut_rank(binding, active_scopes))
        .map(|binding| formatter.format(binding.shortcut))
}

fn shortcut_rank(binding: &ShortcutBinding, active_scopes: &[CommandScope]) -> (u8, usize) {
    let active_index = active_scopes
        .iter()
        .position(|scope| scope == &binding.scope)
        .unwrap_or(usize::MAX);
    (binding.scope.hierarchy_rank(), active_index)
}

fn sanitize_row_id(value: impl AsRef<str>) -> String {
    let mut out = String::new();
    for character in value.as_ref().chars() {
        if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
            out.push(character);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "row".to_owned()
    } else {
        out
    }
}

fn compact_value(value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }
    let mut out = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{root_style, ApproxTextMeasurer, CommandMeta, KeyModifiers, Shortcut, UiSize};

    #[test]
    fn command_diagnostics_panel_lists_commands_shortcuts_and_empty_conflicts() {
        let mut registry = CommandRegistry::new();
        registry
            .register(
                CommandMeta::new("file.open", "Open")
                    .description("Open a file")
                    .category("File"),
            )
            .unwrap();
        registry
            .register(CommandMeta::new("file.close", "Close"))
            .unwrap();
        registry
            .bind_shortcut(
                CommandScope::Global,
                Shortcut::character(
                    'o',
                    KeyModifiers {
                        ctrl: true,
                        ..KeyModifiers::NONE
                    },
                ),
                "file.open",
            )
            .unwrap();

        let mut doc = UiDocument::new(root_style(500.0, 280.0));
        let root = doc.root;
        let nodes = command_diagnostics_panel(
            &mut doc,
            root,
            "commands.debug",
            &registry,
            &[CommandScope::Global],
            &ShortcutFormatter::default(),
            CommandDiagnosticsPanelOptions {
                action_prefix: Some("commands.inspect".to_owned()),
                ..Default::default()
            },
        );
        doc.compute_layout(UiSize::new(500.0, 280.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(doc.node(nodes.command_grid).children.len(), 2);
        assert_eq!(doc.node(nodes.conflict_grid).children.len(), 1);
        let open_row = doc.node(doc.node(nodes.command_grid).children[1]);
        assert_eq!(
            open_row
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("commands.inspect.command.row.file_open")
        );
    }
}
