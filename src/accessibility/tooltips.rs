//! Tooltip and shortcut-display contracts for commands and compact controls.
//!
//! This module keeps tooltip content and shortcut formatting independent from
//! any renderer. Widgets, menus, icon buttons, and command palettes can use the
//! same command metadata and platform-aware shortcut labels.

use crate::commands::{CommandId, CommandRegistry, CommandScope, Shortcut};
use crate::input::{PointerButton, PointerEventKind, RawPointerEvent};
use crate::{
    KeyCode, KeyModifiers, OverlayDismissPolicy, OverlayEntry, OverlayFocusRestoreTarget,
    OverlayId, OverlayKind, UiNodeId, UiPoint, UiRect,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleHelpText {
    pub label: String,
    pub description: Option<String>,
}

impl AccessibleHelpText {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn text(&self) -> String {
        match &self.description {
            Some(description) => format!("{}: {}", self.label, description),
            None => self.label.clone(),
        }
    }
}

impl From<&CommandTooltip> for AccessibleHelpText {
    fn from(value: &CommandTooltip) -> Self {
        let mut text = Self::new(value.title.clone());
        let description = value
            .description
            .iter()
            .chain(value.shortcut_label.iter())
            .chain(value.disabled_reason.iter())
            .cloned()
            .collect::<Vec<_>>();
        if !description.is_empty() {
            text.description = Some(description.join(". "));
        }
        text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TooltipInvocationKind {
    Hover,
    Focus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TooltipVisibility {
    Hidden,
    Pending,
    Visible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HelpDismissReason {
    HoverLost,
    FocusLost,
    Escape,
    PointerDown,
    ContentChanged,
    Programmatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HelpTimingPolicy {
    pub hover_show_delay_ms: u16,
    pub focus_show_delay_ms: u16,
    pub hide_delay_ms: u16,
    pub dismiss_grace_ms: u16,
}

impl HelpTimingPolicy {
    pub const fn immediate() -> Self {
        Self {
            hover_show_delay_ms: 0,
            focus_show_delay_ms: 0,
            hide_delay_ms: 0,
            dismiss_grace_ms: 0,
        }
    }
}

impl Default for HelpTimingPolicy {
    fn default() -> Self {
        Self {
            hover_show_delay_ms: 450,
            focus_show_delay_ms: 0,
            hide_delay_ms: 80,
            dismiss_grace_ms: 120,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HelpItemState {
    pub disabled: bool,
    pub read_only: bool,
    pub allow_disabled_help: bool,
    pub allow_read_only_context_menu: bool,
}

impl HelpItemState {
    pub const ENABLED: Self = Self {
        disabled: false,
        read_only: false,
        allow_disabled_help: true,
        allow_read_only_context_menu: true,
    };

    pub const fn disabled() -> Self {
        Self {
            disabled: true,
            read_only: false,
            allow_disabled_help: true,
            allow_read_only_context_menu: true,
        }
    }

    pub const fn read_only() -> Self {
        Self {
            disabled: false,
            read_only: true,
            allow_disabled_help: true,
            allow_read_only_context_menu: true,
        }
    }

    pub const fn allows_help(self) -> bool {
        !self.disabled || self.allow_disabled_help
    }

    pub const fn allows_context_menu(self) -> bool {
        !self.disabled && (!self.read_only || self.allow_read_only_context_menu)
    }
}

impl Default for HelpItemState {
    fn default() -> Self {
        Self::ENABLED
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationHelp {
    pub field: String,
    pub message: String,
    pub severity: ValidationHelpSeverity,
}

impl ValidationHelp {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationHelpSeverity::Error,
        }
    }

    pub const fn severity(mut self, severity: ValidationHelpSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn tooltip_content(&self) -> TooltipContent {
        TooltipContent::new(match self.severity {
            ValidationHelpSeverity::Info => "Help",
            ValidationHelpSeverity::Warning => "Warning",
            ValidationHelpSeverity::Error => "Error",
        })
        .body(self.message.clone())
    }

    pub fn accessible_text(&self) -> AccessibleHelpText {
        AccessibleHelpText::new(self.field.clone()).description(self.message.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationHelpSeverity {
    Info,
    Warning,
    Error,
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
    pub invocation: TooltipInvocationKind,
    pub content: TooltipContent,
}

impl TooltipRequest {
    pub fn new(anchor: TooltipAnchor, content: TooltipContent) -> Self {
        Self {
            anchor,
            placement: TooltipPlacement::default(),
            delay_ms: 450,
            invocation: TooltipInvocationKind::Hover,
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

    pub const fn invocation(mut self, invocation: TooltipInvocationKind) -> Self {
        self.invocation = invocation;
        self
    }

    pub fn from_command(anchor: TooltipAnchor, tooltip: CommandTooltip) -> Self {
        Self::new(anchor, tooltip.into())
    }

    pub fn text(&self) -> String {
        self.content.text()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipResolution {
    pub visibility: TooltipVisibility,
    pub request: Option<TooltipRequest>,
    pub show_at_ms: Option<u64>,
    pub hide_at_ms: Option<u64>,
}

impl TooltipResolution {
    pub const fn hidden() -> Self {
        Self {
            visibility: TooltipVisibility::Hidden,
            request: None,
            show_at_ms: None,
            hide_at_ms: None,
        }
    }
}

pub fn resolve_tooltip_request(
    hover: Option<TooltipRequest>,
    focus: Option<TooltipRequest>,
    item_state: HelpItemState,
    policy: HelpTimingPolicy,
    now_ms: u64,
) -> TooltipResolution {
    if !item_state.allows_help() {
        return TooltipResolution::hidden();
    }

    let request = focus
        .map(|request| {
            request
                .invocation(TooltipInvocationKind::Focus)
                .delay_ms(policy.focus_show_delay_ms)
        })
        .or_else(|| {
            hover.map(|request| {
                request
                    .invocation(TooltipInvocationKind::Hover)
                    .delay_ms(policy.hover_show_delay_ms)
            })
        });

    let Some(request) = request else {
        return TooltipResolution::hidden();
    };

    let delay_ms = request.delay_ms;
    TooltipResolution {
        visibility: if delay_ms == 0 {
            TooltipVisibility::Visible
        } else {
            TooltipVisibility::Pending
        },
        request: Some(request),
        show_at_ms: Some(now_ms + u64::from(delay_ms)),
        hide_at_ms: None,
    }
}

pub fn resolve_tooltip_dismissal(
    active: Option<TooltipRequest>,
    reason: HelpDismissReason,
    policy: HelpTimingPolicy,
    now_ms: u64,
) -> TooltipResolution {
    match (active, reason) {
        (Some(request), HelpDismissReason::HoverLost) => TooltipResolution {
            visibility: TooltipVisibility::Pending,
            request: Some(request),
            show_at_ms: None,
            hide_at_ms: Some(now_ms + u64::from(policy.hide_delay_ms)),
        },
        (Some(request), HelpDismissReason::FocusLost) if policy.dismiss_grace_ms > 0 => {
            TooltipResolution {
                visibility: TooltipVisibility::Pending,
                request: Some(request),
                show_at_ms: None,
                hide_at_ms: Some(now_ms + u64::from(policy.dismiss_grace_ms)),
            }
        }
        _ => TooltipResolution::hidden(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextMenuTrigger {
    Pointer,
    Keyboard,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextMenuRequest {
    pub trigger: ContextMenuTrigger,
    pub target: UiNodeId,
    pub anchor_rect: UiRect,
    pub position: UiPoint,
    pub item_state: HelpItemState,
}

impl ContextMenuRequest {
    pub const fn pointer(target: UiNodeId, anchor_rect: UiRect, position: UiPoint) -> Self {
        Self {
            trigger: ContextMenuTrigger::Pointer,
            target,
            anchor_rect,
            position,
            item_state: HelpItemState::ENABLED,
        }
    }

    pub fn from_pointer_event(
        target: UiNodeId,
        anchor_rect: UiRect,
        event: RawPointerEvent,
    ) -> Option<Self> {
        matches!(
            event.kind,
            PointerEventKind::Down(PointerButton::Secondary)
                | PointerEventKind::Up(PointerButton::Secondary)
        )
        .then(|| Self::pointer(target, anchor_rect, event.position))
    }

    pub fn keyboard(target: UiNodeId, anchor_rect: UiRect) -> Self {
        Self {
            trigger: ContextMenuTrigger::Keyboard,
            target,
            anchor_rect,
            position: keyboard_context_menu_position(anchor_rect),
            item_state: HelpItemState::ENABLED,
        }
    }

    pub fn from_key_event(
        target: UiNodeId,
        anchor_rect: UiRect,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<Self> {
        keyboard_context_menu_key(key, modifiers).then(|| Self::keyboard(target, anchor_rect))
    }

    pub const fn item_state(mut self, item_state: HelpItemState) -> Self {
        self.item_state = item_state;
        self
    }
}

pub const fn keyboard_context_menu_key(key: KeyCode, modifiers: KeyModifiers) -> bool {
    match key {
        KeyCode::ContextMenu => !modifiers.ctrl && !modifiers.alt && !modifiers.meta,
        KeyCode::F10 => modifiers.shift && !modifiers.ctrl && !modifiers.alt && !modifiers.meta,
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenuResolution {
    pub request: Option<ContextMenuRequest>,
    pub suppressed_reason: Option<ContextMenuSuppressedReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextMenuSuppressedReason {
    Disabled,
    ReadOnly,
}

pub fn resolve_context_menu_request(request: ContextMenuRequest) -> ContextMenuResolution {
    if request.item_state.disabled {
        return ContextMenuResolution {
            request: None,
            suppressed_reason: Some(ContextMenuSuppressedReason::Disabled),
        };
    }
    if request.item_state.read_only && !request.item_state.allow_read_only_context_menu {
        return ContextMenuResolution {
            request: None,
            suppressed_reason: Some(ContextMenuSuppressedReason::ReadOnly),
        };
    }
    ContextMenuResolution {
        request: Some(request),
        suppressed_reason: None,
    }
}

pub fn keyboard_context_menu_position(anchor_rect: UiRect) -> UiPoint {
    UiPoint::new(anchor_rect.x, anchor_rect.bottom())
}

pub fn clamp_context_menu_position(
    position: UiPoint,
    menu_size: crate::UiSize,
    viewport: UiRect,
) -> UiPoint {
    UiPoint::new(
        position
            .x
            .max(viewport.x)
            .min((viewport.right() - menu_size.width).max(viewport.x)),
        position
            .y
            .max(viewport.y)
            .min((viewport.bottom() - menu_size.height).max(viewport.y)),
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct HelpOverlayRecord {
    pub overlay: OverlayId,
    pub kind: OverlayKind,
    pub target: UiNodeId,
    pub bounds: UiRect,
    pub dismiss_policy: OverlayDismissPolicy,
    pub focus_restore: Option<OverlayFocusRestoreTarget>,
}

impl HelpOverlayRecord {
    pub fn tooltip(overlay: OverlayId, request: &TooltipRequest, bounds: UiRect) -> Self {
        Self {
            overlay,
            kind: OverlayKind::Tooltip,
            target: request.anchor.node,
            bounds,
            dismiss_policy: OverlayDismissPolicy::tooltip(),
            focus_restore: None,
        }
    }

    pub fn context_menu(overlay: OverlayId, request: &ContextMenuRequest, bounds: UiRect) -> Self {
        Self {
            overlay,
            kind: OverlayKind::ContextMenu,
            target: request.target,
            bounds,
            dismiss_policy: OverlayDismissPolicy::dismissible(),
            focus_restore: Some(OverlayFocusRestoreTarget::Node(request.target)),
        }
    }

    pub fn overlay_entry(&self) -> OverlayEntry {
        let mut entry = OverlayEntry::new(self.overlay, self.kind, self.bounds)
            .dismiss_policy(self.dismiss_policy);
        if let Some(target) = self.focus_restore.clone() {
            entry = entry.focus_restore(target);
        }
        entry
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
        KeyCode::F10 => "F10".to_string(),
        KeyCode::ContextMenu => "Context Menu".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, CommandMeta};
    use crate::input::{PointerButton, PointerEventKind, RawPointerEvent};
    use crate::overlays::OverlayFocusRestoreRecord;
    use crate::{KeyModifiers, OverlayDismissReason, OverlayStack, UiPoint, UiRect, UiSize};

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
        assert_eq!(
            ShortcutFormatter::default().format(Shortcut::new(
                KeyCode::F10,
                KeyModifiers {
                    shift: true,
                    ..KeyModifiers::NONE
                }
            )),
            "Shift+F10"
        );
        assert_eq!(
            ShortcutFormatter::default()
                .format(Shortcut::new(KeyCode::ContextMenu, KeyModifiers::NONE)),
            "Context Menu"
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

    #[test]
    fn delayed_hover_tooltip_resolves_pending_show_time() {
        let anchor = TooltipAnchor::new(UiNodeId(9), UiRect::new(10.0, 12.0, 44.0, 18.0));
        let hover = TooltipRequest::new(anchor, TooltipContent::new("Pan tool"));
        let resolution = resolve_tooltip_request(
            Some(hover),
            None,
            HelpItemState::ENABLED,
            HelpTimingPolicy::default(),
            1_000,
        );

        assert_eq!(resolution.visibility, TooltipVisibility::Pending);
        assert_eq!(resolution.show_at_ms, Some(1_450));
        let request = resolution.request.expect("hover request");
        assert_eq!(request.invocation, TooltipInvocationKind::Hover);
        assert_eq!(request.delay_ms, 450);
    }

    #[test]
    fn focus_tooltip_takes_precedence_over_hover_and_shows_immediately() {
        let hover_anchor = TooltipAnchor::new(UiNodeId(1), UiRect::new(0.0, 0.0, 20.0, 20.0));
        let focus_anchor = TooltipAnchor::new(UiNodeId(2), UiRect::new(30.0, 0.0, 20.0, 20.0));
        let resolution = resolve_tooltip_request(
            Some(TooltipRequest::new(
                hover_anchor,
                TooltipContent::new("Hover help"),
            )),
            Some(TooltipRequest::new(
                focus_anchor,
                TooltipContent::new("Focus help"),
            )),
            HelpItemState::ENABLED,
            HelpTimingPolicy::default(),
            5,
        );

        assert_eq!(resolution.visibility, TooltipVisibility::Visible);
        let request = resolution.request.expect("focus request");
        assert_eq!(request.anchor.node, UiNodeId(2));
        assert_eq!(request.invocation, TooltipInvocationKind::Focus);
        assert_eq!(request.delay_ms, 0);
    }

    #[test]
    fn command_tooltip_accessible_text_includes_description_shortcut_and_disabled_reason() {
        let registry = registry();
        let tooltip = CommandTooltipResolver::new(&registry)
            .tooltip_for("quantize", &[CommandScope::Editor])
            .expect("tooltip");
        let accessible = AccessibleHelpText::from(&tooltip);

        assert_eq!(accessible.label, "Quantize");
        assert_eq!(
            accessible.description.as_deref(),
            Some("Ctrl+Q. No clip selected")
        );
        assert_eq!(accessible.text(), "Quantize: Ctrl+Q. No clip selected");
    }

    #[test]
    fn validation_help_builds_tooltip_and_accessible_description() {
        let help = ValidationHelp::new("Gain", "Must be between -60 dB and +12 dB")
            .severity(ValidationHelpSeverity::Warning);

        assert_eq!(
            help.tooltip_content().text(),
            "Warning\nMust be between -60 dB and +12 dB"
        );
        assert_eq!(
            help.accessible_text().text(),
            "Gain: Must be between -60 dB and +12 dB"
        );
    }

    #[test]
    fn context_menu_pointer_and_keyboard_triggers_resolve_positions() {
        let anchor = UiRect::new(20.0, 30.0, 100.0, 24.0);
        let pointer_event = RawPointerEvent::new(
            PointerEventKind::Down(PointerButton::Secondary),
            UiPoint::new(40.0, 50.0),
            10,
        );
        let pointer_request =
            ContextMenuRequest::from_pointer_event(UiNodeId(3), anchor, pointer_event)
                .expect("pointer context menu");
        let keyboard_request = ContextMenuRequest::keyboard(UiNodeId(3), anchor);

        assert_eq!(pointer_request.trigger, ContextMenuTrigger::Pointer);
        assert_eq!(pointer_request.position, UiPoint::new(40.0, 50.0));
        assert_eq!(keyboard_request.trigger, ContextMenuTrigger::Keyboard);
        assert_eq!(keyboard_request.position, UiPoint::new(20.0, 54.0));
        let menu_key = ContextMenuRequest::from_key_event(
            UiNodeId(3),
            anchor,
            KeyCode::ContextMenu,
            KeyModifiers::NONE,
        )
        .expect("menu key context menu");
        let shift_f10 = ContextMenuRequest::from_key_event(
            UiNodeId(3),
            anchor,
            KeyCode::F10,
            KeyModifiers {
                shift: true,
                ..KeyModifiers::NONE
            },
        )
        .expect("shift f10 context menu");
        assert_eq!(menu_key.trigger, ContextMenuTrigger::Keyboard);
        assert_eq!(shift_f10.position, keyboard_context_menu_position(anchor));
        assert!(ContextMenuRequest::from_key_event(
            UiNodeId(3),
            anchor,
            KeyCode::F10,
            KeyModifiers::NONE,
        )
        .is_none());

        let clamped = clamp_context_menu_position(
            UiPoint::new(190.0, 190.0),
            UiSize::new(50.0, 40.0),
            UiRect::new(0.0, 0.0, 200.0, 200.0),
        );
        assert_eq!(clamped, UiPoint::new(150.0, 160.0));
    }

    #[test]
    fn context_menu_disabled_items_are_suppressed_but_read_only_can_allow_menu() {
        let anchor = UiRect::new(0.0, 0.0, 30.0, 20.0);
        let disabled = resolve_context_menu_request(
            ContextMenuRequest::keyboard(UiNodeId(4), anchor).item_state(HelpItemState::disabled()),
        );
        let read_only = resolve_context_menu_request(
            ContextMenuRequest::keyboard(UiNodeId(4), anchor)
                .item_state(HelpItemState::read_only()),
        );

        assert!(disabled.request.is_none());
        assert_eq!(
            disabled.suppressed_reason,
            Some(ContextMenuSuppressedReason::Disabled)
        );
        assert!(read_only.request.is_some());
    }

    #[test]
    fn dismissal_and_overlay_records_integrate_with_overlay_stack() {
        let anchor = TooltipAnchor::new(UiNodeId(8), UiRect::new(2.0, 4.0, 20.0, 12.0));
        let request = TooltipRequest::new(anchor, TooltipContent::new("Edit value"));
        let dismiss = resolve_tooltip_dismissal(
            Some(request.clone()),
            HelpDismissReason::HoverLost,
            HelpTimingPolicy::default(),
            100,
        );
        assert_eq!(dismiss.visibility, TooltipVisibility::Pending);
        assert_eq!(dismiss.hide_at_ms, Some(180));

        let context_request = ContextMenuRequest::keyboard(UiNodeId(8), anchor.rect);
        let record = HelpOverlayRecord::context_menu(
            OverlayId::new(30),
            &context_request,
            UiRect::new(2.0, 16.0, 120.0, 96.0),
        );
        let mut stack = OverlayStack::new();
        stack.push(record.overlay_entry());

        let outcome = stack.dismiss(OverlayId::new(30), OverlayDismissReason::Escape);
        assert_eq!(outcome.dismissed, vec![OverlayId::new(30)]);
        assert_eq!(
            outcome.focus_restore,
            vec![OverlayFocusRestoreRecord {
                overlay: OverlayId::new(30),
                target: OverlayFocusRestoreTarget::Node(UiNodeId(8)),
                reason: OverlayDismissReason::Escape,
            }]
        );
    }
}
