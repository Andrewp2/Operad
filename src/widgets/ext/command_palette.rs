//! Command palette widget implementations.

use std::cmp::Ordering;

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, Rect as TaffyRect,
    Size as TaffySize, Style,
};

use crate::tooltips::ShortcutFormatter;
use crate::{
    length, AccessibilityMeta, AccessibilityRole, AnimationMachine, ClipBehavior, ColorRgba,
    CommandId, CommandRegistry, CommandScope, ImageContent, InputBehavior, KeyCode, LayoutStyle,
    ScrollAxes, ShaderEffect, StrokeStyle, TextStyle, UiDocument, UiInputEvent, UiNode, UiNodeId,
    UiNodeStyle, UiSize, UiVisual,
};

use super::menu::{
    command_shortcut_label, label, leading_image, menu_accessibility_label, normalize, place_popup,
    pop_last_char, popup_panel, set_active_descendant, visible_match_range, visible_row_count,
    AnchoredPopup, NavigationDirection, PopupOptions, SearchFieldState, SearchStatusText,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub shortcut: Option<String>,
    pub keywords: Vec<String>,
    pub enabled: bool,
    pub image: Option<ImageContent>,
    pub accessibility_label: Option<String>,
}

impl CommandPaletteItem {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: None,
            shortcut: None,
            keywords: Vec::new(),
            enabled: true,
            image: None,
            accessibility_label: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    pub fn keywords(mut self, keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords.extend(keywords.into_iter().map(Into::into));
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn image(mut self, image: ImageContent) -> Self {
        self.image = Some(image);
        self
    }

    pub fn image_key(self, key: impl Into<String>) -> Self {
        self.image(ImageContent::new(key))
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteMatch {
    pub index: usize,
    pub id: String,
    pub score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteSelection {
    pub index: usize,
    pub id: String,
}

impl CommandPaletteSelection {
    pub fn command_id(&self) -> CommandId {
        CommandId::from(self.id.as_str())
    }

    pub fn command_selection(&self) -> CommandPaletteCommandSelection {
        CommandPaletteCommandSelection {
            index: self.index,
            command: self.command_id(),
        }
    }

    pub fn into_command_selection(self) -> CommandPaletteCommandSelection {
        CommandPaletteCommandSelection {
            index: self.index,
            command: CommandId::from(self.id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteCommandSelection {
    pub index: usize,
    pub command: CommandId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteState {
    query: String,
    active_match: Option<usize>,
    max_results: usize,
}

impl CommandPaletteState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            active_match: None,
            max_results: 12,
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self.active_match = None;
        self
    }

    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = max_results;
        self
    }

    pub fn with_first_active_match(mut self, items: &[CommandPaletteItem]) -> Self {
        self.refresh_active_match(items);
        self
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn active_match(&self) -> Option<usize> {
        self.active_match
    }

    pub fn max_results(&self) -> usize {
        self.max_results
    }

    pub fn set_max_results(&mut self, max_results: usize) {
        self.max_results = max_results;
    }

    pub fn search_field(&self) -> SearchFieldState {
        SearchFieldState::from_query(self.query.clone())
    }

    pub fn apply_search_field(
        &mut self,
        field: &SearchFieldState,
        items: &[CommandPaletteItem],
    ) -> CommandPaletteOutcome {
        let query_changed = self.query != field.query;
        self.query = field.query.clone();
        let matches = self.matches(items);
        self.active_match = first_enabled_palette_match(items, &matches);
        CommandPaletteOutcome {
            query_changed,
            active_match: self.active_match,
            ..Default::default()
        }
    }

    pub fn clear_query(&mut self, items: &[CommandPaletteItem]) -> CommandPaletteOutcome {
        if self.query.is_empty() {
            return CommandPaletteOutcome::default();
        }
        self.set_query("", items);
        CommandPaletteOutcome {
            query_changed: true,
            active_match: self.active_match,
            ..Default::default()
        }
    }

    pub fn matches(&self, items: &[CommandPaletteItem]) -> Vec<CommandPaletteMatch> {
        filter_command_palette(items, &self.query, self.max_results)
    }

    pub fn visible_count(&self, items: &[CommandPaletteItem]) -> usize {
        self.matches(items).len()
    }

    pub fn is_empty(&self, items: &[CommandPaletteItem]) -> bool {
        self.matches(items).is_empty()
    }

    pub fn search_status(&self, items: &[CommandPaletteItem]) -> SearchStatusText {
        self.search_field().status(
            self.visible_count(items),
            items.len(),
            "command",
            "commands",
        )
    }

    pub fn search_status_accessibility(&self, items: &[CommandPaletteItem]) -> AccessibilityMeta {
        self.search_status(items)
            .accessibility("Command search results")
    }

    pub fn set_query(&mut self, query: impl Into<String>, items: &[CommandPaletteItem]) {
        self.query = query.into();
        self.active_match = first_enabled_palette_match(items, &self.matches(items));
    }

    pub fn refresh_active_match(&mut self, items: &[CommandPaletteItem]) -> Option<usize> {
        self.active_match = first_enabled_palette_match(items, &self.matches(items));
        self.active_match
    }

    pub fn move_active(
        &mut self,
        items: &[CommandPaletteItem],
        direction: NavigationDirection,
    ) -> Option<usize> {
        let matches = self.matches(items);
        let active = next_enabled_palette_match(items, &matches, self.active_match, direction);
        self.active_match = active;
        active
    }

    pub fn select_active(&self, items: &[CommandPaletteItem]) -> Option<CommandPaletteSelection> {
        let matches = self.matches(items);
        let active = self.active_match?;
        let palette_match = matches.get(active)?;
        let item = items.get(palette_match.index)?;
        if !item.enabled {
            return None;
        }
        Some(CommandPaletteSelection {
            index: palette_match.index,
            id: palette_match.id.clone(),
        })
    }

    pub fn handle_event(
        &mut self,
        items: &[CommandPaletteItem],
        event: &UiInputEvent,
    ) -> CommandPaletteOutcome {
        let mut outcome = CommandPaletteOutcome::default();
        match event {
            UiInputEvent::TextInput(text) => {
                if push_command_palette_text(&mut self.query, text) {
                    self.active_match = first_enabled_palette_match(items, &self.matches(items));
                    outcome.query_changed = true;
                    outcome.active_match = self.active_match;
                }
            }
            UiInputEvent::Key { key, .. } => match key {
                KeyCode::Backspace | KeyCode::Delete => {
                    if pop_last_char(&mut self.query) {
                        self.active_match =
                            first_enabled_palette_match(items, &self.matches(items));
                        outcome.query_changed = true;
                        outcome.active_match = self.active_match;
                    }
                }
                KeyCode::ArrowDown => {
                    outcome.active_match = self.move_active(items, NavigationDirection::Next);
                }
                KeyCode::ArrowUp => {
                    outcome.active_match = self.move_active(items, NavigationDirection::Previous);
                }
                KeyCode::Home => {
                    let matches = self.matches(items);
                    self.active_match = first_enabled_palette_match(items, &matches);
                    outcome.active_match = self.active_match;
                }
                KeyCode::End => {
                    let matches = self.matches(items);
                    self.active_match = last_enabled_palette_match(items, &matches);
                    outcome.active_match = self.active_match;
                }
                KeyCode::Enter => {
                    outcome.selected = self.select_active(items);
                }
                KeyCode::Escape => outcome.closed = true,
                _ => {}
            },
            _ => {}
        }
        outcome
    }
}

fn push_command_palette_text(query: &mut String, text: &str) -> bool {
    let before = query.len();
    for character in text.chars() {
        if !character.is_control() {
            query.push(character);
        }
    }
    query.len() != before
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandPaletteOutcome {
    pub query_changed: bool,
    pub active_match: Option<usize>,
    pub selected: Option<CommandPaletteSelection>,
    pub closed: bool,
}

impl CommandPaletteOutcome {
    pub fn selected_command(&self) -> Option<CommandPaletteCommandSelection> {
        self.selected
            .as_ref()
            .map(CommandPaletteSelection::command_selection)
    }

    pub fn record_selection(&self, history: &mut CommandPaletteHistory) -> Option<CommandId> {
        let command = self.selected.as_ref()?.command_id();
        history.record(command.clone());
        Some(command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteHistory {
    recent: Vec<CommandId>,
    capacity: usize,
}

impl CommandPaletteHistory {
    pub const DEFAULT_CAPACITY: usize = 12;

    pub fn new() -> Self {
        Self {
            recent: Vec::new(),
            capacity: Self::DEFAULT_CAPACITY,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            recent: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn recent(&self) -> &[CommandId] {
        &self.recent
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn clear(&mut self) {
        self.recent.clear();
    }

    pub fn record(&mut self, command: impl Into<CommandId>) {
        let command = command.into();
        self.recent.retain(|recent| recent != &command);
        self.recent.insert(0, command);
        self.recent.truncate(self.capacity);
    }

    pub fn recency_rank(&self, command: &CommandId) -> Option<usize> {
        self.recent.iter().position(|recent| recent == command)
    }

    pub fn is_recent(&self, command: &CommandId) -> bool {
        self.recency_rank(command).is_some()
    }
}

impl Default for CommandPaletteHistory {
    fn default() -> Self {
        Self::new()
    }
}

pub fn command_palette_item_from_command(
    registry: &CommandRegistry,
    command: impl Into<CommandId>,
    active_scopes: &[CommandScope],
    formatter: &ShortcutFormatter,
) -> Option<CommandPaletteItem> {
    let command_id = command.into();
    let command = registry.command(&command_id)?;
    let mut item = CommandPaletteItem::new(command_id.as_str(), command.meta.label.clone());
    if let Some(description) = &command.meta.description {
        item = item.subtitle(description.clone());
    } else if let Some(category) = &command.meta.category {
        item = item.subtitle(category.clone());
    }
    if let Some(category) = &command.meta.category {
        item = item.keyword(category.clone());
    }
    if let Some(shortcut) = command_shortcut_label(registry, &command_id, active_scopes, formatter)
    {
        item = item.shortcut(shortcut);
    }
    if !command.enabled {
        item = item.disabled();
    }
    Some(item)
}

pub fn command_palette_items_from_registry(
    registry: &CommandRegistry,
    active_scopes: &[CommandScope],
    formatter: &ShortcutFormatter,
) -> Vec<CommandPaletteItem> {
    let mut command_ids = registry
        .commands()
        .map(|command| command.meta.id.clone())
        .collect::<Vec<_>>();
    command_ids.sort();
    command_ids
        .into_iter()
        .filter_map(|command| {
            command_palette_item_from_command(registry, command, active_scopes, formatter)
        })
        .collect()
}

pub fn command_palette_items_from_registry_with_history(
    registry: &CommandRegistry,
    active_scopes: &[CommandScope],
    formatter: &ShortcutFormatter,
    history: &CommandPaletteHistory,
) -> Vec<CommandPaletteItem> {
    let mut command_ids = registry
        .commands()
        .map(|command| command.meta.id.clone())
        .collect::<Vec<_>>();
    command_ids.sort_by(|left, right| {
        match (history.recency_rank(left), history.recency_rank(right)) {
            (Some(left_rank), Some(right_rank)) => left_rank.cmp(&right_rank),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => left.cmp(right),
        }
    });
    command_ids
        .into_iter()
        .filter_map(|command| {
            let recent = history.is_recent(&command);
            command_palette_item_from_command(registry, command, active_scopes, formatter).map(
                |item| {
                    if recent {
                        item.keyword("recent")
                    } else {
                        item
                    }
                },
            )
        })
        .collect()
}

pub fn filter_command_palette(
    items: &[CommandPaletteItem],
    query: &str,
    max_results: usize,
) -> Vec<CommandPaletteMatch> {
    let query = normalize(query);
    if query.trim().is_empty() {
        return items
            .iter()
            .enumerate()
            .take(max_results)
            .map(|(index, item)| CommandPaletteMatch {
                index,
                id: item.id.clone(),
                score: 0,
            })
            .collect();
    }

    let mut matches = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            score_command_palette_item(item, &query).map(|score| CommandPaletteMatch {
                index,
                id: item.id.clone(),
                score,
            })
        })
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.index.cmp(&right.index))
    });
    matches.truncate(max_results);
    matches
}

#[derive(Debug, Clone)]
pub struct CommandPaletteOptions {
    pub width: f32,
    pub row_height: f32,
    pub max_visible_rows: usize,
    pub panel_visual: UiVisual,
    pub input_visual: UiVisual,
    pub row_visual: UiVisual,
    pub active_row_visual: UiVisual,
    pub text_style: TextStyle,
    pub muted_text_style: TextStyle,
    pub disabled_text_style: TextStyle,
    pub image_size: UiSize,
    pub accessibility_label: Option<String>,
    pub panel_shader: Option<ShaderEffect>,
    pub active_row_shader: Option<ShaderEffect>,
    pub panel_animation: Option<AnimationMachine>,
    pub z_index: i16,
    pub action_prefix: Option<String>,
}

impl Default for CommandPaletteOptions {
    fn default() -> Self {
        Self {
            width: 520.0,
            row_height: 34.0,
            max_visible_rows: 10,
            panel_visual: UiVisual::panel(
                ColorRgba::new(24, 29, 37, 255),
                Some(StrokeStyle::new(ColorRgba::new(83, 97, 119, 255), 1.0)),
                6.0,
            ),
            input_visual: UiVisual::panel(ColorRgba::new(18, 22, 28, 255), None, 4.0),
            row_visual: UiVisual::TRANSPARENT,
            active_row_visual: UiVisual::panel(ColorRgba::new(58, 87, 126, 255), None, 3.0),
            text_style: TextStyle::default(),
            muted_text_style: TextStyle {
                color: ColorRgba::new(178, 188, 204, 255),
                ..Default::default()
            },
            disabled_text_style: TextStyle {
                color: ColorRgba::new(138, 148, 164, 255),
                ..Default::default()
            },
            image_size: UiSize::new(18.0, 18.0),
            accessibility_label: None,
            panel_shader: None,
            active_row_shader: None,
            panel_animation: None,
            z_index: 120,
            action_prefix: None,
        }
    }
}

impl CommandPaletteOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteNodes {
    pub root: UiNodeId,
    pub input: UiNodeId,
    pub rows: Vec<UiNodeId>,
}

pub fn command_palette(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    items: &[CommandPaletteItem],
    state: &CommandPaletteState,
    popup: Option<AnchoredPopup>,
    options: CommandPaletteOptions,
) -> CommandPaletteNodes {
    let name = name.into();
    let matches = state.matches(items);
    let search_field = state.search_field();
    let search_status = search_field.status(matches.len(), items.len(), "command", "commands");
    let visible_rows = visible_row_count(matches.len(), options.max_visible_rows);
    let visible_range = visible_match_range(
        matches.len(),
        state.active_match(),
        options.max_visible_rows,
    );
    let height = 42.0 + visible_rows as f32 * options.row_height;
    let root = if let Some(popup) = popup {
        let layout = place_popup(
            popup.anchor,
            UiSize::new(options.width.max(0.0), height.max(0.0)),
            popup.viewport,
            popup.placement,
        );
        popup_panel(
            document,
            parent,
            name.clone(),
            layout.rect,
            PopupOptions {
                visual: options.panel_visual,
                z_index: options.z_index,
                clip: ClipBehavior::Clip,
                scroll_axes: ScrollAxes::NONE,
                accessibility: Some(AccessibilityMeta::new(AccessibilityRole::Dialog).label(
                    menu_accessibility_label(&name, options.accessibility_label.as_ref()),
                )),
                shader: options.panel_shader.clone(),
                animation: options.panel_animation.clone(),
                ..Default::default()
            },
        )
    } else {
        let mut node = UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(height.max(0.0)),
                    },
                    max_size: TaffySize {
                        width: length(options.width.max(0.0)),
                        height: Dimension::auto(),
                    },
                    padding: TaffyRect::length(4.0),
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                z_index: options.z_index,
                ..Default::default()
            },
        )
        .with_visual(options.panel_visual)
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Dialog).label(
            menu_accessibility_label(&name, options.accessibility_label.as_ref()),
        ));
        if let Some(shader) = options.panel_shader.clone() {
            node = node.with_shader(shader);
        }
        if let Some(animation) = options.panel_animation.clone() {
            node = node.with_animation(animation);
        }
        document.add_child(parent, node)
    };

    let input = document.add_child(
        root,
        UiNode::container(
            format!("{name}.input"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(34.0),
                    },
                    padding: TaffyRect::length(8.0),
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(options.input_visual)
        .with_accessibility(
            search_field
                .input_accessibility("Command search")
                .shortcut("Ctrl+K"),
        )
        .with_action(
            options
                .action_prefix
                .as_deref()
                .map(|prefix| format!("{prefix}.search"))
                .unwrap_or_else(|| format!("{name}.search")),
        ),
    );
    label(
        document,
        input,
        format!("{name}.query"),
        if state.query().is_empty() {
            ""
        } else {
            state.query()
        },
        options.text_style.clone(),
        LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: Dimension::percent(1.0),
                height: Dimension::auto(),
            },
            ..Default::default()
        }),
    );

    let mut list_node = UiNode::container(
        format!("{name}.results"),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(visible_rows as f32 * options.row_height),
                },
                ..Default::default()
            })
            .style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    );
    if matches.len() > visible_rows {
        list_node = list_node.with_scroll(ScrollAxes::VERTICAL);
    }
    list_node = list_node.with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::List)
            .label(format!("{name} results"))
            .value(search_status.text),
    );
    let list = document.add_child(root, list_node);

    let mut rows = Vec::with_capacity(visible_range.len());
    for (match_index, palette_match) in matches[visible_range.clone()].iter().enumerate() {
        let actual_match_index = visible_range.start + match_index;
        let Some(item) = items.get(palette_match.index) else {
            continue;
        };
        let active = state.active_match() == Some(actual_match_index);
        let visual = if active {
            options.active_row_visual
        } else {
            options.row_visual
        };
        let text_style = if item.enabled {
            options.text_style.clone()
        } else {
            options.disabled_text_style.clone()
        };
        let row = document.add_child(
            list,
            command_palette_row_node(
                format!("{name}.result.{}", palette_match.index),
                item,
                active,
                visual,
                &options,
            ),
        );
        if item.enabled {
            if let Some(prefix) = options.action_prefix.as_deref() {
                document.node_mut(row).action = Some(format!("{prefix}.item.{}", item.id).into());
            }
        }
        if let Some(image) = &item.image {
            leading_image(
                document,
                row,
                format!("{name}.result.{}.image", palette_match.index),
                image.clone(),
                &command_item_accessibility_label(item),
                options.image_size,
            );
        }
        let text_column = document.add_child(
            row,
            UiNode::container(
                format!("{name}.result.{}.text", palette_match.index),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: Some(JustifyContent::Center),
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: Dimension::auto(),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        label(
            document,
            text_column,
            format!("{name}.result.{}.title", palette_match.index),
            &item.title,
            text_style,
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        );
        if let Some(subtitle) = &item.subtitle {
            label(
                document,
                text_column,
                format!("{name}.result.{}.subtitle", palette_match.index),
                subtitle,
                options.muted_text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            );
        }
        if let Some(shortcut) = &item.shortcut {
            label(
                document,
                row,
                format!("{name}.result.{}.shortcut", palette_match.index),
                shortcut,
                options.muted_text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            );
        }
        rows.push(row);
    }
    let active_row = state.active_match().and_then(|index| {
        visible_range
            .contains(&index)
            .then(|| rows.get(index - visible_range.start))
            .flatten()
            .copied()
    });
    set_active_descendant(document, input, active_row);
    set_active_descendant(document, list, active_row);

    CommandPaletteNodes { root, input, rows }
}

fn command_palette_row_node(
    name: impl Into<String>,
    item: &CommandPaletteItem,
    active: bool,
    visual: UiVisual,
    options: &CommandPaletteOptions,
) -> UiNode {
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::ListItem)
        .label(command_item_accessibility_label(item))
        .value(item.id.clone());
    if let Some(shortcut) = &item.shortcut {
        accessibility = accessibility.hint(format!("Shortcut {shortcut}"));
    }
    if item.enabled {
        accessibility = accessibility.focusable();
    } else {
        accessibility = accessibility.disabled();
    }

    let mut node = UiNode::container(
        name,
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(options.row_height),
                },
                padding: TaffyRect::length(6.0),
                flex_shrink: 0.0,
                ..Default::default()
            })
            .style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_input(if item.enabled {
        InputBehavior::BUTTON
    } else {
        InputBehavior::NONE
    })
    .with_visual(visual)
    .with_accessibility(accessibility);

    if active {
        if let Some(shader) = options.active_row_shader.clone() {
            node = node.with_shader(shader);
        }
    }
    node
}

fn command_item_accessibility_label(item: &CommandPaletteItem) -> String {
    item.accessibility_label
        .clone()
        .unwrap_or_else(|| item.title.clone())
}

fn score_command_palette_item(item: &CommandPaletteItem, query: &str) -> Option<i32> {
    let title = normalize(&item.title);
    let subtitle = item.subtitle.as_deref().map(normalize);
    let shortcut = item.shortcut.as_deref().map(normalize);
    let keywords = item
        .keywords
        .iter()
        .map(|keyword| normalize(keyword))
        .collect::<Vec<_>>();
    let tokens = query.split_whitespace().collect::<Vec<_>>();

    if tokens.iter().any(|token| {
        !title.contains(token)
            && !subtitle
                .as_deref()
                .is_some_and(|subtitle| subtitle.contains(token))
            && !shortcut
                .as_deref()
                .is_some_and(|shortcut| shortcut.contains(token))
            && !keywords.iter().any(|keyword| keyword.contains(token))
    }) {
        return None;
    }

    let mut score = match title.as_str().cmp(query) {
        Ordering::Equal => 1200,
        Ordering::Less | Ordering::Greater if title.starts_with(query) => 900,
        Ordering::Less | Ordering::Greater if title.contains(query) => 650,
        Ordering::Less | Ordering::Greater => 100,
    };

    for token in tokens {
        if title.starts_with(token) {
            score += 90;
        } else if title.contains(token) {
            score += 50;
        }
        if keywords.iter().any(|keyword| keyword.contains(token)) {
            score += 35;
        }
        if subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.contains(token))
        {
            score += 20;
        }
        if shortcut
            .as_deref()
            .is_some_and(|shortcut| shortcut.contains(token))
        {
            score += 10;
        }
    }

    Some(score)
}

pub(in crate::widgets::ext) fn first_enabled_palette_match(
    items: &[CommandPaletteItem],
    matches: &[CommandPaletteMatch],
) -> Option<usize> {
    matches
        .iter()
        .position(|palette_match| items[palette_match.index].enabled)
}

fn last_enabled_palette_match(
    items: &[CommandPaletteItem],
    matches: &[CommandPaletteMatch],
) -> Option<usize> {
    matches
        .iter()
        .rposition(|palette_match| items[palette_match.index].enabled)
}

fn next_enabled_palette_match(
    items: &[CommandPaletteItem],
    matches: &[CommandPaletteMatch],
    current: Option<usize>,
    direction: NavigationDirection,
) -> Option<usize> {
    let len = matches.len();
    if len == 0 {
        return None;
    }
    let start = match (current.filter(|index| *index < len), direction) {
        (Some(index), NavigationDirection::Next) => (index + 1) % len,
        (Some(index), NavigationDirection::Previous) => (index + len - 1) % len,
        (None, NavigationDirection::Next) => 0,
        (None, NavigationDirection::Previous) => len - 1,
    };
    for offset in 0..len {
        let index = match direction {
            NavigationDirection::Next => (start + offset) % len,
            NavigationDirection::Previous => (start + len - offset) % len,
        };
        if items[matches[index].index].enabled {
            return Some(index);
        }
    }
    None
}
