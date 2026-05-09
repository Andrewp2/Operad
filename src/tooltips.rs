//! Tooltip and shortcut-display contracts for commands and compact controls.
//!
//! This module keeps tooltip content and shortcut formatting independent from
//! any renderer. Widgets, menus, icon buttons, and command palettes can use the
//! same command metadata and platform-aware shortcut labels.

use crate::commands::{CommandId, CommandRegistry, CommandScope, Shortcut};
use crate::{KeyCode, UiNodeId, UiRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutDisplayPlatform {
    Generic,
    Apple,
    Windows,
    Linux,
}

impl Default for ShortcutDisplayPlatform {
    fn default() -> Self {
        Self::Generic
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutFormatter {
    pub platform: ShortcutDisplayPlatform,
    pub separator: String,
}

impl ShortcutFormatter {
    pub fn new(platform: ShortcutDisplayPlatform) -> Self {
        Self {
            platform,
            separator: "+".to_string(),
        }
    }

    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    pub fn format(&self, shortcut: Shortcut) -> String {
        let mut parts = Vec::<String>::new();
        if shortcut.modifiers.ctrl {
            parts.push(
                match self.platform {
                    ShortcutDisplayPlatform::Apple => "Control",
                    _ => "Ctrl",
                }
                .to_string(),
            );
        }
        if shortcut.modifiers.alt {
            parts.push(
                match self.platform {
                    ShortcutDisplayPlatform::Apple => "Option",
                    _ => "Alt",
                }
                .to_string(),
            );
        }
        if shortcut.modifiers.shift {
            parts.push("Shift".to_string());
        }
        if shortcut.modifiers.meta {
            parts.push(
                match self.platform {
                    ShortcutDisplayPlatform::Apple => "Cmd",
                    ShortcutDisplayPlatform::Windows => "Win",
                    ShortcutDisplayPlatform::Linux => "Super",
                    ShortcutDisplayPlatform::Generic => "Meta",
                }
                .to_string(),
            );
        }
        parts.push(format_key(shortcut.key));
        parts.join(&self.separator)
    }
}

impl Default for ShortcutFormatter {
    fn default() -> Self {
        Self::new(ShortcutDisplayPlatform::Generic)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandTooltip {
    pub command: CommandId,
    pub title: String,
    pub description: Option<String>,
    pub shortcut: Option<Shortcut>,
    pub shortcut_label: Option<String>,
    pub disabled_reason: Option<String>,
}

impl CommandTooltip {
    pub fn text(&self) -> String {
        let mut text = self.title.clone();
        if let Some(shortcut) = &self.shortcut_label {
            text.push_str(" (");
            text.push_str(shortcut);
            text.push(')');
        }
        if let Some(description) = &self.description {
            text.push('\n');
            text.push_str(description);
        }
        if let Some(reason) = &self.disabled_reason {
            text.push('\n');
            text.push_str(reason);
        }
        text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TooltipPlacement {
    Above,
    Below,
    Left,
    Right,
    Cursor,
}

impl Default for TooltipPlacement {
    fn default() -> Self {
        Self::Above
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipAnchor {
    pub node: UiNodeId,
    pub rect: UiRect,
}

impl TooltipAnchor {
    pub const fn new(node: UiNodeId, rect: UiRect) -> Self {
        Self { node, rect }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TooltipContent {
    pub title: String,
    pub body: Option<String>,
    pub shortcut_label: Option<String>,
    pub disabled_reason: Option<String>,
}

impl TooltipContent {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: None,
            shortcut_label: None,
            disabled_reason: None,
        }
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn shortcut_label(mut self, shortcut_label: impl Into<String>) -> Self {
        self.shortcut_label = Some(shortcut_label.into());
        self
    }

    pub fn disabled_reason(mut self, disabled_reason: impl Into<String>) -> Self {
        self.disabled_reason = Some(disabled_reason.into());
        self
    }

    pub fn text(&self) -> String {
        let tooltip = CommandTooltip {
            command: CommandId::new("tooltip.content"),
            title: self.title.clone(),
            description: self.body.clone(),
            shortcut: None,
            shortcut_label: self.shortcut_label.clone(),
            disabled_reason: self.disabled_reason.clone(),
        };
        tooltip.text()
    }
}

impl From<CommandTooltip> for TooltipContent {
    fn from(value: CommandTooltip) -> Self {
        Self {
            title: value.title,
            body: value.description,
            shortcut_label: value.shortcut_label,
            disabled_reason: value.disabled_reason,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipRequest {
    pub anchor: TooltipAnchor,
    pub placement: TooltipPlacement,
    pub delay_ms: u16,
    pub content: TooltipContent,
}

impl TooltipRequest {
    pub fn new(anchor: TooltipAnchor, content: TooltipContent) -> Self {
        Self {
            anchor,
            placement: TooltipPlacement::default(),
            delay_ms: 450,
            content,
        }
    }

    pub const fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub const fn delay_ms(mut self, delay_ms: u16) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    pub fn from_command(anchor: TooltipAnchor, tooltip: CommandTooltip) -> Self {
        Self::new(anchor, tooltip.into())
    }

    pub fn text(&self) -> String {
        self.content.text()
    }
}

#[derive(Debug, Clone)]
pub struct CommandTooltipResolver<'a> {
    registry: &'a CommandRegistry,
    formatter: ShortcutFormatter,
}

impl<'a> CommandTooltipResolver<'a> {
    pub fn new(registry: &'a CommandRegistry) -> Self {
        Self {
            registry,
            formatter: ShortcutFormatter::default(),
        }
    }

    pub fn formatter(mut self, formatter: ShortcutFormatter) -> Self {
        self.formatter = formatter;
        self
    }

    pub fn tooltip_for(
        &self,
        command: impl Into<CommandId>,
        active_scopes: &[CommandScope],
    ) -> Option<CommandTooltip> {
        let command_id = command.into();
        let command = self.registry.command(&command_id)?;
        let shortcut = self.shortcut_for(&command_id, active_scopes);
        let shortcut_label = shortcut.map(|shortcut| self.formatter.format(shortcut));
        Some(CommandTooltip {
            command: command_id,
            title: command.meta.label.clone(),
            description: command.meta.description.clone(),
            shortcut,
            shortcut_label,
            disabled_reason: command.disabled_reason.clone(),
        })
    }

    pub fn request_for(
        &self,
        anchor: TooltipAnchor,
        command: impl Into<CommandId>,
        active_scopes: &[CommandScope],
    ) -> Option<TooltipRequest> {
        self.tooltip_for(command, active_scopes)
            .map(|tooltip| TooltipRequest::from_command(anchor, tooltip))
    }

    pub fn shortcut_for(
        &self,
        command: &CommandId,
        active_scopes: &[CommandScope],
    ) -> Option<Shortcut> {
        let scopes = ordered_tooltip_scopes(active_scopes);
        scopes.into_iter().rev().find_map(|scope| {
            self.registry
                .bindings()
                .iter()
                .find(|binding| binding.scope == scope && binding.command == *command)
                .map(|binding| binding.shortcut)
        })
    }
}

fn ordered_tooltip_scopes(active_scopes: &[CommandScope]) -> Vec<CommandScope> {
    let mut scopes = Vec::<(CommandScope, usize)>::new();
    upsert_scope(&mut scopes, CommandScope::Global, 0);
    for (index, scope) in active_scopes.iter().enumerate() {
        upsert_scope(&mut scopes, scope.clone(), index + 1);
    }
    scopes.sort_by(|(left_scope, left_index), (right_scope, right_index)| {
        left_scope
            .hierarchy_rank()
            .cmp(&right_scope.hierarchy_rank())
            .then_with(|| left_index.cmp(right_index))
    });
    scopes.into_iter().map(|(scope, _)| scope).collect()
}

fn upsert_scope(scopes: &mut Vec<(CommandScope, usize)>, scope: CommandScope, index: usize) {
    if let Some((_, existing_index)) = scopes
        .iter_mut()
        .find(|(existing_scope, _)| *existing_scope == scope)
    {
        *existing_index = index;
    } else {
        scopes.push((scope, index));
    }
}

fn format_key(key: KeyCode) -> String {
    match key {
        KeyCode::Character(character) => character.to_ascii_uppercase().to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::ArrowLeft => "Left".to_string(),
        KeyCode::ArrowRight => "Right".to_string(),
        KeyCode::ArrowUp => "Up".to_string(),
        KeyCode::ArrowDown => "Down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Escape => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, CommandMeta};
    use crate::{KeyModifiers, UiRect};

    fn registry() -> CommandRegistry {
        let mut registry = CommandRegistry::new();
        registry
            .register(Command::new(
                CommandMeta::new("save", "Save Project")
                    .description("Writes the project file to disk")
                    .category("File"),
            ))
            .unwrap();
        registry
            .register(Command::new(CommandMeta::new(
                "duplicate",
                "Duplicate Note",
            )))
            .unwrap();
        registry
            .register(
                Command::new(CommandMeta::new("quantize", "Quantize")).disabled("No clip selected"),
            )
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('s'), "save")
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('d'), "duplicate")
            .unwrap();
        registry
            .bind_shortcut(
                CommandScope::Editor,
                Shortcut::new(
                    KeyCode::Character('d'),
                    KeyModifiers {
                        ctrl: true,
                        shift: true,
                        ..KeyModifiers::NONE
                    },
                ),
                "duplicate",
            )
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Editor, Shortcut::ctrl('q'), "quantize")
            .unwrap();
        registry
    }

    #[test]
    fn shortcut_formatter_uses_platform_specific_modifier_names() {
        let shortcut = Shortcut::new(
            KeyCode::Character('s'),
            KeyModifiers {
                shift: true,
                meta: true,
                ..KeyModifiers::NONE
            },
        );

        assert_eq!(
            ShortcutFormatter::new(ShortcutDisplayPlatform::Generic).format(shortcut),
            "Shift+Meta+S"
        );
        assert_eq!(
            ShortcutFormatter::new(ShortcutDisplayPlatform::Apple).format(shortcut),
            "Shift+Cmd+S"
        );
        assert_eq!(
            ShortcutFormatter::new(ShortcutDisplayPlatform::Windows).format(shortcut),
            "Shift+Win+S"
        );
        assert_eq!(
            ShortcutFormatter::new(ShortcutDisplayPlatform::Linux)
                .separator(" ")
                .format(shortcut),
            "Shift Super S"
        );
    }

    #[test]
    fn command_tooltip_prefers_active_scope_shortcut_and_formats_text() {
        let registry = registry();
        let resolver = CommandTooltipResolver::new(&registry);
        let tooltip = resolver
            .tooltip_for(
                "duplicate",
                &[CommandScope::Workspace, CommandScope::Editor],
            )
            .expect("tooltip");

        assert_eq!(tooltip.title, "Duplicate Note");
        assert_eq!(tooltip.shortcut_label.as_deref(), Some("Ctrl+Shift+D"));
        assert_eq!(tooltip.text(), "Duplicate Note (Ctrl+Shift+D)");
    }

    #[test]
    fn command_tooltip_uses_global_shortcut_when_no_scope_specific_binding_exists() {
        let registry = registry();
        let tooltip = CommandTooltipResolver::new(&registry)
            .tooltip_for("save", &[CommandScope::Editor])
            .expect("tooltip");

        assert_eq!(tooltip.title, "Save Project");
        assert_eq!(tooltip.shortcut_label.as_deref(), Some("Ctrl+S"));
        assert_eq!(
            tooltip.text(),
            "Save Project (Ctrl+S)\nWrites the project file to disk"
        );
    }

    #[test]
    fn disabled_command_tooltip_preserves_shortcut_and_disabled_reason() {
        let registry = registry();
        let tooltip = CommandTooltipResolver::new(&registry)
            .tooltip_for("quantize", &[CommandScope::Editor])
            .expect("tooltip");

        assert_eq!(tooltip.shortcut_label.as_deref(), Some("Ctrl+Q"));
        assert_eq!(tooltip.disabled_reason.as_deref(), Some("No clip selected"));
        assert_eq!(tooltip.text(), "Quantize (Ctrl+Q)\nNo clip selected");
    }

    #[test]
    fn tooltip_request_wraps_command_tooltip_for_renderer_neutral_delivery() {
        let registry = registry();
        let anchor = TooltipAnchor::new(UiNodeId(4), UiRect::new(10.0, 20.0, 32.0, 18.0));
        let request = CommandTooltipResolver::new(&registry)
            .formatter(ShortcutFormatter::new(ShortcutDisplayPlatform::Apple))
            .request_for(anchor, "save", &[CommandScope::Global])
            .expect("tooltip request")
            .placement(TooltipPlacement::Below)
            .delay_ms(250);

        assert_eq!(request.anchor, anchor);
        assert_eq!(request.placement, TooltipPlacement::Below);
        assert_eq!(request.delay_ms, 250);
        assert_eq!(
            request.text(),
            "Save Project (Control+S)\nWrites the project file to disk"
        );
    }

    #[test]
    fn tooltip_content_can_be_built_without_command_registry() {
        let content = TooltipContent::new("Snap to grid")
            .body("Constrains edits to the visible grid")
            .shortcut_label("G");

        assert_eq!(
            content.text(),
            "Snap to grid (G)\nConstrains edits to the visible grid"
        );
    }
}
