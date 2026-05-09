//! Command metadata and keyboard shortcut routing primitives.
//!
//! The command identifiers are intentionally opaque to Operad. Applications own
//! the ID namespace and can use strings, UUIDs, or any other stable identifier
//! serialized into a [`CommandId`].

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::platform::{
    AppLifecycleRequest, ClipboardRequest, FileDialogRequest, NotificationRequest, OpenUrlRequest,
    PlatformRequest, PlatformRequestId, PlatformServiceCapabilities, PlatformServiceKind,
    PlatformServiceRequest, RepaintRequest, ScreenshotRequest,
};
use crate::{KeyCode, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(String);

impl CommandId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for CommandId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for CommandId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CommandId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&CommandId> for CommandId {
    fn from(value: &CommandId) -> Self {
        value.clone()
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandMeta {
    pub id: CommandId,
    pub label: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

impl CommandMeta {
    pub fn new(id: impl Into<CommandId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            category: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub meta: CommandMeta,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

impl Command {
    pub fn new(meta: CommandMeta) -> Self {
        Self {
            meta,
            enabled: true,
            disabled_reason: None,
        }
    }

    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        self.enabled = false;
        self.disabled_reason = Some(reason.into());
        self
    }

    pub fn enabled(mut self) -> Self {
        self.enabled = true;
        self.disabled_reason = None;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl From<CommandMeta> for Command {
    fn from(meta: CommandMeta) -> Self {
        Self::new(meta)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandEffect {
    Platform(PlatformRequest),
    Custom(String),
}

impl CommandEffect {
    pub fn platform(request: PlatformRequest) -> Self {
        Self::Platform(request)
    }

    pub fn custom(effect: impl Into<String>) -> Self {
        Self::Custom(effect.into())
    }

    pub fn clipboard(request: ClipboardRequest) -> Self {
        Self::Platform(PlatformRequest::Clipboard(request))
    }

    pub fn file_dialog(request: FileDialogRequest) -> Self {
        Self::Platform(PlatformRequest::FileDialog(request))
    }

    pub fn open_url(request: OpenUrlRequest) -> Self {
        Self::Platform(PlatformRequest::OpenUrl(request))
    }

    pub fn notification(request: NotificationRequest) -> Self {
        Self::Platform(PlatformRequest::Notification(request))
    }

    pub fn screenshot(request: ScreenshotRequest) -> Self {
        Self::Platform(PlatformRequest::Screenshot(request))
    }

    pub fn quit() -> Self {
        Self::Platform(PlatformRequest::AppLifecycle(AppLifecycleRequest::Quit))
    }

    pub fn close_window(window_id: impl Into<String>) -> Self {
        Self::Platform(PlatformRequest::AppLifecycle(
            AppLifecycleRequest::close_window(window_id),
        ))
    }

    pub fn close_active_window() -> Self {
        Self::Platform(PlatformRequest::AppLifecycle(
            AppLifecycleRequest::close_active_window(),
        ))
    }

    pub fn repaint(request: RepaintRequest) -> Self {
        Self::Platform(PlatformRequest::Repaint(request))
    }

    pub const fn platform_kind(&self) -> Option<PlatformServiceKind> {
        match self {
            Self::Platform(request) => Some(request.kind()),
            Self::Custom(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandEffectInvocation {
    Platform(PlatformServiceRequest),
    Custom { command: CommandId, effect: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommandScope {
    Global,
    Workspace,
    Panel,
    Editor,
    Text,
    Modal,
    Custom(String),
}

impl CommandScope {
    pub const BUILT_INS: [Self; 6] = [
        Self::Global,
        Self::Workspace,
        Self::Panel,
        Self::Editor,
        Self::Text,
        Self::Modal,
    ];

    pub fn custom(id: impl Into<String>) -> Self {
        Self::Custom(id.into())
    }

    pub const fn hierarchy_rank(&self) -> u8 {
        match self {
            Self::Global => 0,
            Self::Workspace => 10,
            Self::Panel => 20,
            Self::Editor => 30,
            Self::Text => 40,
            Self::Modal => 50,
            Self::Custom(_) => 60,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Shortcut {
    pub fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self {
            key: normalize_key(key),
            modifiers,
        }
    }

    pub fn character(character: char, modifiers: KeyModifiers) -> Self {
        Self::new(KeyCode::Character(character), modifiers)
    }

    pub fn ctrl(character: char) -> Self {
        Self::character(
            character,
            KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
        )
    }

    pub fn meta(character: char) -> Self {
        Self::character(
            character,
            KeyModifiers {
                meta: true,
                ..KeyModifiers::NONE
            },
        )
    }
}

impl Hash for Shortcut {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_key_code(self.key, state);
        self.modifiers.shift.hash(state);
        self.modifiers.ctrl.hash(state);
        self.modifiers.alt.hash(state);
        self.modifiers.meta.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutBinding {
    pub scope: CommandScope,
    pub shortcut: Shortcut,
    pub command: CommandId,
}

impl ShortcutBinding {
    pub fn new(scope: CommandScope, shortcut: Shortcut, command: impl Into<CommandId>) -> Self {
        Self {
            scope,
            shortcut,
            command: command.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutConflict {
    pub scope: CommandScope,
    pub shortcut: Shortcut,
    pub commands: Vec<CommandId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandRegistryError {
    DuplicateCommand(CommandId),
    UnknownCommand(CommandId),
    DisabledCommand(CommandId),
    MissingCommandEffect(CommandId),
    UnsupportedCommandEffect {
        command: CommandId,
        kind: PlatformServiceKind,
    },
    ShortcutConflict(ShortcutConflict),
}

impl fmt::Display for CommandRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCommand(command) => write!(formatter, "duplicate command `{command}`"),
            Self::UnknownCommand(command) => write!(formatter, "unknown command `{command}`"),
            Self::DisabledCommand(command) => write!(formatter, "disabled command `{command}`"),
            Self::MissingCommandEffect(command) => {
                write!(formatter, "missing command effect for `{command}`")
            }
            Self::UnsupportedCommandEffect { command, kind } => {
                write!(
                    formatter,
                    "unsupported platform effect {kind:?} for `{command}`"
                )
            }
            Self::ShortcutConflict(conflict) => {
                write!(
                    formatter,
                    "shortcut conflict in {:?} for {:?}",
                    conflict.scope, conflict.shortcut
                )
            }
        }
    }
}

impl std::error::Error for CommandRegistryError {}

#[derive(Debug, Clone, Default)]
pub struct CommandRegistry {
    commands: HashMap<CommandId, Command>,
    bindings: Vec<ShortcutBinding>,
    effects: HashMap<CommandId, CommandEffect>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, command: impl Into<Command>) -> Result<(), CommandRegistryError> {
        let command = command.into();
        let id = command.meta.id.clone();

        if self.commands.contains_key(&id) {
            return Err(CommandRegistryError::DuplicateCommand(id));
        }

        self.commands.insert(id, command);
        Ok(())
    }

    pub fn command(&self, id: impl Into<CommandId>) -> Option<&Command> {
        let id = id.into();
        self.commands.get(&id)
    }

    pub fn commands(&self) -> impl Iterator<Item = &Command> {
        self.commands.values()
    }

    pub fn bindings(&self) -> &[ShortcutBinding] {
        &self.bindings
    }

    pub fn effect(&self, command: impl Into<CommandId>) -> Option<&CommandEffect> {
        let command = command.into();
        self.effects.get(&command)
    }

    pub fn effects(&self) -> impl Iterator<Item = (&CommandId, &CommandEffect)> {
        self.effects.iter()
    }

    pub fn bind_effect(
        &mut self,
        command: impl Into<CommandId>,
        effect: CommandEffect,
    ) -> Result<(), CommandRegistryError> {
        let command = command.into();

        if !self.commands.contains_key(&command) {
            return Err(CommandRegistryError::UnknownCommand(command));
        }

        self.effects.insert(command, effect);
        Ok(())
    }

    pub fn bind_platform_effect(
        &mut self,
        command: impl Into<CommandId>,
        request: PlatformRequest,
    ) -> Result<(), CommandRegistryError> {
        self.bind_effect(command, CommandEffect::platform(request))
    }

    pub fn bind_custom_effect(
        &mut self,
        command: impl Into<CommandId>,
        effect: impl Into<String>,
    ) -> Result<(), CommandRegistryError> {
        self.bind_effect(command, CommandEffect::custom(effect))
    }

    pub fn invoke_effect(
        &self,
        command: impl Into<CommandId>,
        request_id: PlatformRequestId,
        capabilities: PlatformServiceCapabilities,
    ) -> Result<CommandEffectInvocation, CommandRegistryError> {
        let command = command.into();
        let registered = self
            .commands
            .get(&command)
            .ok_or_else(|| CommandRegistryError::UnknownCommand(command.clone()))?;

        if !registered.is_enabled() {
            return Err(CommandRegistryError::DisabledCommand(command));
        }

        let effect = self
            .effects
            .get(&command)
            .ok_or_else(|| CommandRegistryError::MissingCommandEffect(command.clone()))?;

        match effect {
            CommandEffect::Platform(request) => {
                if !capabilities.supports(request) {
                    return Err(CommandRegistryError::UnsupportedCommandEffect {
                        command,
                        kind: request.kind(),
                    });
                }

                Ok(CommandEffectInvocation::Platform(
                    PlatformServiceRequest::new(request_id, request.clone()),
                ))
            }
            CommandEffect::Custom(effect) => Ok(CommandEffectInvocation::Custom {
                command,
                effect: effect.clone(),
            }),
        }
    }

    pub fn bind_shortcut(
        &mut self,
        scope: CommandScope,
        shortcut: Shortcut,
        command: impl Into<CommandId>,
    ) -> Result<(), CommandRegistryError> {
        let binding = ShortcutBinding::new(scope, shortcut, command);

        if !self.commands.contains_key(&binding.command) {
            return Err(CommandRegistryError::UnknownCommand(binding.command));
        }

        if let Some(conflict) = self.conflict_for_binding(&binding) {
            return Err(CommandRegistryError::ShortcutConflict(conflict));
        }

        if !self.bindings.contains(&binding) {
            self.bindings.push(binding);
        }

        Ok(())
    }

    pub fn set_enabled(
        &mut self,
        command: impl Into<CommandId>,
        enabled: bool,
    ) -> Result<(), CommandRegistryError> {
        let command = command.into();
        let registered = self
            .commands
            .get_mut(&command)
            .ok_or_else(|| CommandRegistryError::UnknownCommand(command.clone()))?;

        registered.enabled = enabled;
        if enabled {
            registered.disabled_reason = None;
        }

        Ok(())
    }

    pub fn disable(
        &mut self,
        command: impl Into<CommandId>,
        reason: impl Into<String>,
    ) -> Result<(), CommandRegistryError> {
        let command = command.into();
        let registered = self
            .commands
            .get_mut(&command)
            .ok_or_else(|| CommandRegistryError::UnknownCommand(command.clone()))?;

        registered.enabled = false;
        registered.disabled_reason = Some(reason.into());
        Ok(())
    }

    pub fn enable(&mut self, command: impl Into<CommandId>) -> Result<(), CommandRegistryError> {
        self.set_enabled(command, true)
    }

    pub fn conflicts(&self) -> Vec<ShortcutConflict> {
        let mut by_shortcut: HashMap<(CommandScope, Shortcut), HashSet<CommandId>> = HashMap::new();

        for binding in &self.bindings {
            by_shortcut
                .entry((binding.scope.clone(), binding.shortcut))
                .or_default()
                .insert(binding.command.clone());
        }

        let mut conflicts = by_shortcut
            .into_iter()
            .filter_map(|((scope, shortcut), commands)| {
                if commands.len() < 2 {
                    return None;
                }

                let mut commands = commands.into_iter().collect::<Vec<_>>();
                commands.sort();

                Some(ShortcutConflict {
                    scope,
                    shortcut,
                    commands,
                })
            })
            .collect::<Vec<_>>();

        conflicts.sort_by(|left, right| {
            left.scope
                .hierarchy_rank()
                .cmp(&right.scope.hierarchy_rank())
                .then_with(|| format!("{:?}", left.scope).cmp(&format!("{:?}", right.scope)))
                .then_with(|| format!("{:?}", left.shortcut).cmp(&format!("{:?}", right.shortcut)))
        });

        conflicts
    }

    pub fn resolve_key(
        &self,
        key: KeyCode,
        modifiers: KeyModifiers,
        active_scopes: &[CommandScope],
    ) -> Option<CommandId> {
        self.resolve(Shortcut::new(key, modifiers), active_scopes)
    }

    pub fn resolve(&self, shortcut: Shortcut, active_scopes: &[CommandScope]) -> Option<CommandId> {
        let scopes = ordered_active_scopes(active_scopes);

        for scope in scopes.iter().rev() {
            let Some(binding) = self
                .bindings
                .iter()
                .find(|binding| binding.scope == *scope && binding.shortcut == shortcut)
            else {
                continue;
            };

            if self
                .commands
                .get(&binding.command)
                .is_some_and(Command::is_enabled)
            {
                return Some(binding.command.clone());
            }
        }

        None
    }

    fn conflict_for_binding(&self, binding: &ShortcutBinding) -> Option<ShortcutConflict> {
        let mut commands = self
            .bindings
            .iter()
            .filter(|existing| {
                existing.scope == binding.scope
                    && existing.shortcut == binding.shortcut
                    && existing.command != binding.command
            })
            .map(|existing| existing.command.clone())
            .collect::<Vec<_>>();

        if commands.is_empty() {
            return None;
        }

        commands.push(binding.command.clone());
        commands.sort();
        commands.dedup();

        Some(ShortcutConflict {
            scope: binding.scope.clone(),
            shortcut: binding.shortcut,
            commands,
        })
    }
}

fn ordered_active_scopes(active_scopes: &[CommandScope]) -> Vec<CommandScope> {
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

fn normalize_key(key: KeyCode) -> KeyCode {
    match key {
        KeyCode::Character(character) => KeyCode::Character(character.to_ascii_lowercase()),
        other => other,
    }
}

fn hash_key_code<H: Hasher>(key: KeyCode, state: &mut H) {
    match key {
        KeyCode::Character(character) => {
            0_u8.hash(state);
            character.hash(state);
        }
        KeyCode::Backspace => 1_u8.hash(state),
        KeyCode::Delete => 2_u8.hash(state),
        KeyCode::ArrowLeft => 3_u8.hash(state),
        KeyCode::ArrowRight => 4_u8.hash(state),
        KeyCode::ArrowUp => 5_u8.hash(state),
        KeyCode::ArrowDown => 6_u8.hash(state),
        KeyCode::Home => 7_u8.hash(state),
        KeyCode::End => 8_u8.hash(state),
        KeyCode::Enter => 9_u8.hash(state),
        KeyCode::Escape => 10_u8.hash(state),
        KeyCode::Tab => 11_u8.hash(state),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(id: &'static str) -> Command {
        Command::new(CommandMeta::new(id, id))
    }

    fn registry_with(ids: &[&'static str]) -> CommandRegistry {
        let mut registry = CommandRegistry::new();
        for id in ids {
            registry.register(command(id)).unwrap();
        }
        registry
    }

    #[test]
    fn command_ids_are_opaque_and_metadata_is_preserved() {
        let mut registry = CommandRegistry::new();

        registry
            .register(
                CommandMeta::new("workspace.save", "Save Workspace")
                    .description("Persist the active workspace")
                    .category("File"),
            )
            .unwrap();

        let command = registry.command("workspace.save").unwrap();
        assert_eq!(command.meta.id.as_str(), "workspace.save");
        assert_eq!(command.meta.label, "Save Workspace");
        assert_eq!(
            command.meta.description.as_deref(),
            Some("Persist the active workspace")
        );
        assert_eq!(command.meta.category.as_deref(), Some("File"));
    }

    #[test]
    fn duplicate_commands_are_rejected() {
        let mut registry = registry_with(&["save"]);

        let error = registry.register(command("save")).unwrap_err();

        assert_eq!(
            error,
            CommandRegistryError::DuplicateCommand(CommandId::from("save"))
        );
    }

    #[test]
    fn same_scope_shortcut_conflicts_are_rejected() {
        let mut registry = registry_with(&["save", "search"]);

        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('s'), "save")
            .unwrap();
        let error = registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('s'), "search")
            .unwrap_err();

        assert_eq!(
            error,
            CommandRegistryError::ShortcutConflict(ShortcutConflict {
                scope: CommandScope::Global,
                shortcut: Shortcut::ctrl('s'),
                commands: vec![CommandId::from("save"), CommandId::from("search")],
            })
        );
    }

    #[test]
    fn same_shortcut_can_be_bound_in_more_specific_scopes() {
        let mut registry = registry_with(&["global.search", "editor.search"]);

        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('f'), "global.search")
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Editor, Shortcut::ctrl('f'), "editor.search")
            .unwrap();

        assert_eq!(
            registry.resolve(Shortcut::ctrl('f'), &[]),
            Some(CommandId::from("global.search"))
        );
        assert_eq!(
            registry.resolve(Shortcut::ctrl('f'), &[CommandScope::Editor]),
            Some(CommandId::from("editor.search"))
        );
    }

    #[test]
    fn scope_hierarchy_wins_over_input_scope_order() {
        let mut registry = registry_with(&["workspace.rename", "text.rename"]);

        registry
            .bind_shortcut(
                CommandScope::Workspace,
                Shortcut::new(KeyCode::Enter, KeyModifiers::NONE),
                "workspace.rename",
            )
            .unwrap();
        registry
            .bind_shortcut(
                CommandScope::Text,
                Shortcut::new(KeyCode::Enter, KeyModifiers::NONE),
                "text.rename",
            )
            .unwrap();

        assert_eq!(
            registry.resolve(
                Shortcut::new(KeyCode::Enter, KeyModifiers::NONE),
                &[CommandScope::Text, CommandScope::Workspace]
            ),
            Some(CommandId::from("text.rename"))
        );
        assert_eq!(
            registry.resolve(
                Shortcut::new(KeyCode::Enter, KeyModifiers::NONE),
                &[CommandScope::Workspace, CommandScope::Text]
            ),
            Some(CommandId::from("text.rename"))
        );
    }

    #[test]
    fn disabled_commands_do_not_resolve() {
        let mut registry = registry_with(&["global.cancel", "modal.close"]);

        registry
            .bind_shortcut(
                CommandScope::Global,
                Shortcut::new(KeyCode::Escape, KeyModifiers::NONE),
                "global.cancel",
            )
            .unwrap();
        registry
            .bind_shortcut(
                CommandScope::Modal,
                Shortcut::new(KeyCode::Escape, KeyModifiers::NONE),
                "modal.close",
            )
            .unwrap();
        registry.disable("modal.close", "Nothing to close").unwrap();

        assert_eq!(
            registry.resolve(
                Shortcut::new(KeyCode::Escape, KeyModifiers::NONE),
                &[CommandScope::Modal]
            ),
            Some(CommandId::from("global.cancel"))
        );
    }

    #[test]
    fn character_shortcuts_are_case_normalized() {
        let mut registry = registry_with(&["save"]);

        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('S'), "save")
            .unwrap();

        assert_eq!(
            registry.resolve_key(
                KeyCode::Character('s'),
                KeyModifiers {
                    ctrl: true,
                    ..KeyModifiers::NONE
                },
                &[]
            ),
            Some(CommandId::from("save"))
        );
    }

    #[test]
    fn conflicts_reports_existing_same_scope_collisions() {
        let registry = CommandRegistry {
            commands: HashMap::new(),
            bindings: vec![
                ShortcutBinding::new(CommandScope::Panel, Shortcut::ctrl('k'), "open.palette"),
                ShortcutBinding::new(CommandScope::Panel, Shortcut::ctrl('k'), "focus.search"),
                ShortcutBinding::new(CommandScope::Text, Shortcut::ctrl('k'), "insert.link"),
            ],
            effects: HashMap::new(),
        };

        assert_eq!(
            registry.conflicts(),
            vec![ShortcutConflict {
                scope: CommandScope::Panel,
                shortcut: Shortcut::ctrl('k'),
                commands: vec![
                    CommandId::from("focus.search"),
                    CommandId::from("open.palette")
                ],
            }]
        );
    }

    #[test]
    fn command_effects_create_platform_requests_when_supported() {
        let mut registry = registry_with(&["file.open", "app.quit", "capture.viewport"]);
        let open_request =
            FileDialogRequest::new(crate::platform::FileDialogMode::OpenFile).title("Open Project");
        let screenshot_request =
            ScreenshotRequest::new(crate::platform::ScreenshotTarget::Viewport);

        registry
            .bind_effect(
                "file.open",
                CommandEffect::file_dialog(open_request.clone()),
            )
            .unwrap();
        registry
            .bind_effect("app.quit", CommandEffect::quit())
            .unwrap();
        registry
            .bind_effect(
                "capture.viewport",
                CommandEffect::screenshot(screenshot_request.clone()),
            )
            .unwrap();

        let open = registry
            .invoke_effect(
                "file.open",
                PlatformRequestId::new(7),
                PlatformServiceCapabilities::DESKTOP,
            )
            .unwrap();
        let quit = registry
            .invoke_effect(
                "app.quit",
                PlatformRequestId::new(8),
                PlatformServiceCapabilities::DESKTOP,
            )
            .unwrap();

        assert_eq!(
            open,
            CommandEffectInvocation::Platform(PlatformServiceRequest::new(
                PlatformRequestId::new(7),
                PlatformRequest::FileDialog(open_request)
            ))
        );
        assert_eq!(
            quit,
            CommandEffectInvocation::Platform(PlatformServiceRequest::new(
                PlatformRequestId::new(8),
                PlatformRequest::AppLifecycle(AppLifecycleRequest::Quit)
            ))
        );
        assert_eq!(
            registry.effect("capture.viewport").unwrap().platform_kind(),
            Some(PlatformServiceKind::Screenshot)
        );
    }

    #[test]
    fn command_effect_invocation_checks_command_state_and_capabilities() {
        let mut registry = registry_with(&["capture.viewport", "disabled", "no.effect"]);
        registry
            .bind_effect(
                "capture.viewport",
                CommandEffect::screenshot(ScreenshotRequest::new(
                    crate::platform::ScreenshotTarget::Viewport,
                )),
            )
            .unwrap();
        registry
            .bind_effect(
                "disabled",
                CommandEffect::clipboard(ClipboardRequest::ReadText),
            )
            .unwrap();
        registry.disable("disabled", "Unavailable").unwrap();

        assert_eq!(
            registry
                .invoke_effect(
                    "capture.viewport",
                    PlatformRequestId::new(1),
                    PlatformServiceCapabilities::NONE
                )
                .unwrap_err(),
            CommandRegistryError::UnsupportedCommandEffect {
                command: CommandId::from("capture.viewport"),
                kind: PlatformServiceKind::Screenshot,
            }
        );
        assert_eq!(
            registry
                .invoke_effect(
                    "disabled",
                    PlatformRequestId::new(2),
                    PlatformServiceCapabilities::DESKTOP
                )
                .unwrap_err(),
            CommandRegistryError::DisabledCommand(CommandId::from("disabled"))
        );
        assert_eq!(
            registry
                .invoke_effect(
                    "no.effect",
                    PlatformRequestId::new(3),
                    PlatformServiceCapabilities::DESKTOP
                )
                .unwrap_err(),
            CommandRegistryError::MissingCommandEffect(CommandId::from("no.effect"))
        );
        assert_eq!(
            registry
                .bind_effect("missing", CommandEffect::quit())
                .unwrap_err(),
            CommandRegistryError::UnknownCommand(CommandId::from("missing"))
        );
    }

    #[test]
    fn custom_command_effects_round_trip_without_platform_capabilities() {
        let mut registry = registry_with(&["orbifold.quantize"]);

        registry
            .bind_custom_effect("orbifold.quantize", "orbifold.quantize:selected")
            .unwrap();

        assert_eq!(
            registry
                .invoke_effect(
                    "orbifold.quantize",
                    PlatformRequestId::new(42),
                    PlatformServiceCapabilities::NONE
                )
                .unwrap(),
            CommandEffectInvocation::Custom {
                command: CommandId::from("orbifold.quantize"),
                effect: "orbifold.quantize:selected".to_string(),
            }
        );
    }
}
