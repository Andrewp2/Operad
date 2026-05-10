//! Popup, menu, dropdown, and command-palette widgets.

use std::cmp::Ordering;

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentageAuto, Position,
    Rect as TaffyRect, Size as TaffySize, Style,
};

use crate::{
    length, AccessibilityAction, AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole,
    AnimationMachine, ClipBehavior, ColorRgba, CommandId, CommandRegistry, CommandScope,
    CommandTooltipResolver, ImageContent, InputBehavior, KeyCode, KeyModifiers, LayoutStyle,
    ScrollAxes, ShaderEffect, ShortcutFormatter, StrokeStyle, TextStyle, UiDocument, UiInputEvent,
    UiNode, UiNodeId, UiNodeStyle, UiPoint, UiRect, UiSize, UiVisual,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupSide {
    Top,
    Bottom,
    Left,
    Right,
}

impl PopupSide {
    pub const fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupAlign {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupPlacement {
    pub side: PopupSide,
    pub align: PopupAlign,
    pub offset: f32,
    pub viewport_margin: f32,
    pub flip: bool,
    pub constrain_to_viewport: bool,
}

impl PopupPlacement {
    pub const fn new(side: PopupSide, align: PopupAlign) -> Self {
        Self {
            side,
            align,
            offset: 4.0,
            viewport_margin: 4.0,
            flip: true,
            constrain_to_viewport: true,
        }
    }

    pub const fn with_offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    pub const fn with_viewport_margin(mut self, margin: f32) -> Self {
        self.viewport_margin = margin;
        self
    }

    pub const fn with_flip(mut self, flip: bool) -> Self {
        self.flip = flip;
        self
    }

    pub const fn with_viewport_constraint(mut self, constrain: bool) -> Self {
        self.constrain_to_viewport = constrain;
        self
    }
}

impl Default for PopupPlacement {
    fn default() -> Self {
        Self::new(PopupSide::Bottom, PopupAlign::Start)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupLayout {
    pub rect: UiRect,
    pub side: PopupSide,
    pub flipped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnchoredPopup {
    pub anchor: UiRect,
    pub viewport: UiRect,
    pub placement: PopupPlacement,
}

impl AnchoredPopup {
    pub const fn new(anchor: UiRect, viewport: UiRect, placement: PopupPlacement) -> Self {
        Self {
            anchor,
            viewport,
            placement,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PopupOptions {
    pub visual: UiVisual,
    pub z_index: i16,
    pub clip: ClipBehavior,
    pub scroll_axes: ScrollAxes,
    pub accessibility: Option<AccessibilityMeta>,
    pub shader: Option<ShaderEffect>,
    pub animation: Option<AnimationMachine>,
}

impl Default for PopupOptions {
    fn default() -> Self {
        Self {
            visual: UiVisual::panel(
                ColorRgba::new(26, 31, 39, 255),
                Some(StrokeStyle::new(ColorRgba::new(77, 90, 111, 255), 1.0)),
                4.0,
            ),
            z_index: 100,
            clip: ClipBehavior::Clip,
            scroll_axes: ScrollAxes::NONE,
            accessibility: Some(AccessibilityMeta::new(AccessibilityRole::Dialog)),
            shader: None,
            animation: None,
        }
    }
}

pub fn place_popup(
    anchor: UiRect,
    popup_size: UiSize,
    viewport: UiRect,
    placement: PopupPlacement,
) -> PopupLayout {
    let inner_viewport = inset_viewport(viewport, placement.viewport_margin.max(0.0));
    let primary = popup_rect_for_anchor(anchor, popup_size, placement.side, placement);
    let mut rect = primary;
    let mut side = placement.side;
    let mut flipped = false;

    if placement.flip {
        let opposite_side = placement.side.opposite();
        let opposite = popup_rect_for_anchor(anchor, popup_size, opposite_side, placement);
        if overflow_amount(opposite, inner_viewport) < overflow_amount(primary, inner_viewport) {
            rect = opposite;
            side = opposite_side;
            flipped = true;
        }
    }

    if placement.constrain_to_viewport {
        rect = constrain_rect_to_viewport(rect, inner_viewport);
    }

    PopupLayout {
        rect,
        side,
        flipped,
    }
}

pub fn centered_popup_rect(viewport: UiRect, popup_size: UiSize, viewport_margin: f32) -> UiRect {
    let inner = inset_viewport(viewport, viewport_margin.max(0.0));
    constrain_rect_to_viewport(
        UiRect::new(
            inner.x + (inner.width - popup_size.width) * 0.5,
            inner.y + (inner.height - popup_size.height) * 0.5,
            popup_size.width,
            popup_size.height,
        ),
        inner,
    )
}

pub fn popup_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    rect: UiRect,
    options: PopupOptions,
) -> UiNodeId {
    let PopupOptions {
        visual,
        z_index,
        clip,
        scroll_axes,
        accessibility,
        shader,
        animation,
    } = options;
    let mut node = UiNode::container(
        name,
        UiNodeStyle {
            layout: absolute_rect_style(rect),
            clip,
            z_index,
            ..Default::default()
        },
    )
    .with_visual(visual);

    if scroll_axes != ScrollAxes::NONE {
        node = node.with_scroll(scroll_axes);
    }
    if let Some(accessibility) = accessibility {
        node = node.with_accessibility(accessibility);
    }
    if let Some(shader) = shader {
        node = node.with_shader(shader);
    }
    if let Some(animation) = animation {
        node = node.with_animation(animation);
    }

    document.add_child(parent, node)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItemKind {
    Command,
    Check { checked: bool },
    Separator,
    Submenu { items: Vec<MenuItem> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub id: Option<String>,
    pub label: String,
    pub enabled: bool,
    pub shortcut: Option<String>,
    pub image: Option<ImageContent>,
    pub destructive: bool,
    pub accessibility_label: Option<String>,
    pub kind: MenuItemKind,
}

impl MenuItem {
    pub fn command(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            label: label.into(),
            enabled: true,
            shortcut: None,
            image: None,
            destructive: false,
            accessibility_label: None,
            kind: MenuItemKind::Command,
        }
    }

    pub fn check(id: impl Into<String>, label: impl Into<String>, checked: bool) -> Self {
        Self {
            id: Some(id.into()),
            label: label.into(),
            enabled: true,
            shortcut: None,
            image: None,
            destructive: false,
            accessibility_label: None,
            kind: MenuItemKind::Check { checked },
        }
    }

    pub fn submenu(id: impl Into<String>, label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self {
            id: Some(id.into()),
            label: label.into(),
            enabled: true,
            shortcut: None,
            image: None,
            destructive: false,
            accessibility_label: None,
            kind: MenuItemKind::Submenu { items },
        }
    }

    pub fn separator() -> Self {
        Self {
            id: None,
            label: String::new(),
            enabled: false,
            shortcut: None,
            image: None,
            destructive: false,
            accessibility_label: None,
            kind: MenuItemKind::Separator,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn image(mut self, image: ImageContent) -> Self {
        self.image = Some(image);
        self
    }

    pub fn image_key(self, key: impl Into<String>) -> Self {
        self.image(ImageContent::new(key))
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn is_separator(&self) -> bool {
        matches!(self.kind, MenuItemKind::Separator)
    }

    pub fn is_navigable(&self) -> bool {
        self.enabled && !self.is_separator()
    }

    pub fn is_action(&self) -> bool {
        self.enabled
            && matches!(
                self.kind,
                MenuItemKind::Command | MenuItemKind::Check { .. }
            )
    }

    pub fn children(&self) -> Option<&[MenuItem]> {
        match &self.kind {
            MenuItemKind::Submenu { items } => Some(items),
            MenuItemKind::Command | MenuItemKind::Check { .. } | MenuItemKind::Separator => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSelection {
    pub id: Option<String>,
    pub index_path: Vec<usize>,
}

impl MenuSelection {
    pub fn command_id(&self) -> Option<CommandId> {
        self.id.as_deref().map(CommandId::from)
    }

    pub fn command_selection(&self) -> Option<MenuCommandSelection> {
        Some(MenuCommandSelection {
            command: self.command_id()?,
            index_path: self.index_path.clone(),
        })
    }

    pub fn into_command_selection(self) -> Option<MenuCommandSelection> {
        Some(MenuCommandSelection {
            command: self.id.map(CommandId::from)?,
            index_path: self.index_path,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuCommandSelection {
    pub command: CommandId,
    pub index_path: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    Next,
    Previous,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuNavigationState {
    pub active_path: Vec<usize>,
}

impl MenuNavigationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_active_path(active_path: impl Into<Vec<usize>>) -> Self {
        Self {
            active_path: active_path.into(),
        }
    }

    pub fn clear(&mut self) {
        self.active_path.clear();
    }

    pub fn open_root(&mut self, items: &[MenuItem]) -> Option<Vec<usize>> {
        let index = first_navigable_index(items)?;
        self.active_path = vec![index];
        Some(self.active_path.clone())
    }

    pub fn active_item<'a>(&self, items: &'a [MenuItem]) -> Option<&'a MenuItem> {
        menu_item_at_path(items, &self.active_path)
    }

    pub fn active_parent_path(&self) -> &[usize] {
        self.active_path
            .split_last()
            .map(|(_, parent)| parent)
            .unwrap_or(&[])
    }

    pub fn move_active(
        &mut self,
        items: &[MenuItem],
        direction: NavigationDirection,
    ) -> Option<Vec<usize>> {
        let (level_items, parent_path, current) = self.current_level(items)?;
        let index = next_navigable_index(level_items, current, direction)?;
        self.active_path = parent_path;
        self.active_path.push(index);
        Some(self.active_path.clone())
    }

    pub fn open_submenu(&mut self, items: &[MenuItem]) -> Option<Vec<usize>> {
        let item = self.active_item(items)?;
        let child_index = first_navigable_index(item.children()?)?;
        self.active_path.push(child_index);
        Some(self.active_path.clone())
    }

    pub fn close_submenu(&mut self) -> Option<Vec<usize>> {
        if self.active_path.len() <= 1 {
            return None;
        }
        self.active_path.pop();
        Some(self.active_path.clone())
    }

    pub fn select_active(&self, items: &[MenuItem]) -> Option<MenuSelection> {
        menu_selection_at_path(items, &self.active_path)
    }

    pub fn activate_active(&mut self, items: &[MenuItem]) -> MenuNavigationOutcome {
        let mut outcome = MenuNavigationOutcome::default();
        if let Some(selection) = self.select_active(items) {
            outcome.selected = Some(selection);
            outcome.closed = true;
            self.clear();
        } else if let Some(path) = self.open_submenu(items) {
            outcome.active_path = Some(path);
            outcome.opened_submenu = true;
        }
        outcome
    }

    pub fn handle_event(
        &mut self,
        items: &[MenuItem],
        event: &UiInputEvent,
    ) -> MenuNavigationOutcome {
        let mut outcome = MenuNavigationOutcome::default();
        if let UiInputEvent::Key {
            key: KeyCode::Escape,
            ..
        } = event
        {
            self.clear();
            outcome.closed = true;
            return outcome;
        }

        if self.active_path.is_empty() {
            outcome.active_path = self.open_root(items);
            if self.active_path.is_empty() {
                return outcome;
            }
        }

        if let UiInputEvent::TextInput(text) = event {
            if let Some(character) = first_typeahead_character(text) {
                outcome.active_path = self.move_active_to_match(items, character);
            }
            return outcome;
        }

        let UiInputEvent::Key { key, .. } = event else {
            return outcome;
        };

        match *key {
            KeyCode::ArrowDown => {
                outcome.active_path = self.move_active(items, NavigationDirection::Next);
            }
            KeyCode::ArrowUp => {
                outcome.active_path = self.move_active(items, NavigationDirection::Previous);
            }
            KeyCode::Home => {
                outcome.active_path = self.move_to_edge(items, NavigationDirection::Next);
            }
            KeyCode::End => {
                outcome.active_path = self.move_to_edge(items, NavigationDirection::Previous);
            }
            KeyCode::ArrowRight => {
                outcome.active_path = self.open_submenu(items);
                outcome.opened_submenu = outcome.active_path.is_some();
            }
            KeyCode::ArrowLeft => {
                outcome.active_path = self.close_submenu();
                outcome.closed_submenu = outcome.active_path.is_some();
            }
            KeyCode::Enter | KeyCode::Character(' ') => {
                outcome = self.activate_active(items);
            }
            KeyCode::Character(character) if is_typeahead_character(character) => {
                outcome.active_path = self.move_active_to_match(items, character);
            }
            _ => {}
        }

        outcome
    }

    fn current_level<'a>(
        &self,
        items: &'a [MenuItem],
    ) -> Option<(&'a [MenuItem], Vec<usize>, Option<usize>)> {
        let (current, parent_path) = self
            .active_path
            .split_last()
            .map(|(current, parent)| (Some(*current), parent.to_vec()))
            .unwrap_or((None, Vec::new()));
        let level_items = menu_items_at_path(items, &parent_path)?;
        Some((level_items, parent_path, current))
    }

    fn move_to_edge(
        &mut self,
        items: &[MenuItem],
        direction: NavigationDirection,
    ) -> Option<Vec<usize>> {
        let (level_items, mut parent_path, _) = self.current_level(items)?;
        let index = match direction {
            NavigationDirection::Next => first_navigable_index(level_items),
            NavigationDirection::Previous => last_navigable_index(level_items),
        }?;
        parent_path.push(index);
        self.active_path = parent_path;
        Some(self.active_path.clone())
    }

    fn move_active_to_match(&mut self, items: &[MenuItem], character: char) -> Option<Vec<usize>> {
        let (level_items, mut parent_path, current) = self.current_level(items)?;
        let index = next_menu_typeahead_index(level_items, current, character)?;
        parent_path.push(index);
        self.active_path = parent_path;
        Some(self.active_path.clone())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuNavigationOutcome {
    pub active_path: Option<Vec<usize>>,
    pub opened_submenu: bool,
    pub closed_submenu: bool,
    pub closed: bool,
    pub selected: Option<MenuSelection>,
}

impl MenuNavigationOutcome {
    pub fn selected_command(&self) -> Option<MenuCommandSelection> {
        self.selected
            .as_ref()
            .and_then(MenuSelection::command_selection)
    }
}

pub fn menu_item_at_path<'a>(items: &'a [MenuItem], path: &[usize]) -> Option<&'a MenuItem> {
    let (first, rest) = path.split_first()?;
    let item = items.get(*first)?;
    if rest.is_empty() {
        return Some(item);
    }
    menu_item_at_path(item.children()?, rest)
}

pub fn menu_items_at_path<'a>(items: &'a [MenuItem], path: &[usize]) -> Option<&'a [MenuItem]> {
    if path.is_empty() {
        return Some(items);
    }
    menu_item_at_path(items, path)?.children()
}

pub fn menu_selection_at_path(items: &[MenuItem], path: &[usize]) -> Option<MenuSelection> {
    let item = menu_item_at_path(items, path)?;
    if !item.is_action() {
        return None;
    }
    Some(MenuSelection {
        id: item.id.clone(),
        index_path: path.to_vec(),
    })
}

pub fn menu_command_selection_at_path(
    items: &[MenuItem],
    path: &[usize],
) -> Option<MenuCommandSelection> {
    menu_selection_at_path(items, path)?.into_command_selection()
}

pub fn menu_item_from_command(
    registry: &CommandRegistry,
    command: impl Into<CommandId>,
    active_scopes: &[CommandScope],
    formatter: &ShortcutFormatter,
) -> Option<MenuItem> {
    let command_id = command.into();
    let command = registry.command(&command_id)?;
    let mut item = MenuItem::command(command_id.as_str(), command.meta.label.clone());
    if let Some(shortcut) = command_shortcut_label(registry, &command_id, active_scopes, formatter)
    {
        item = item.shortcut(shortcut);
    }
    if !command.enabled {
        item = item.disabled();
    }
    Some(item)
}

pub fn first_navigable_index(items: &[MenuItem]) -> Option<usize> {
    items.iter().position(MenuItem::is_navigable)
}

pub fn last_navigable_index(items: &[MenuItem]) -> Option<usize> {
    items.iter().rposition(MenuItem::is_navigable)
}

pub fn next_navigable_index(
    items: &[MenuItem],
    current: Option<usize>,
    direction: NavigationDirection,
) -> Option<usize> {
    let len = items.len();
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
        if items[index].is_navigable() {
            return Some(index);
        }
    }

    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub image: Option<ImageContent>,
    pub accessibility_label: Option<String>,
}

impl SelectOption {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            enabled: true,
            image: None,
            accessibility_label: None,
        }
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
pub struct SelectSelection {
    pub index: usize,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectMenuState {
    pub open: bool,
    pub selected: Option<usize>,
    pub active: Option<usize>,
}

impl SelectMenuState {
    pub const fn new() -> Self {
        Self {
            open: false,
            selected: None,
            active: None,
        }
    }

    pub const fn with_selected(selected: usize) -> Self {
        Self {
            open: false,
            selected: Some(selected),
            active: Some(selected),
        }
    }

    pub fn selected_id<'a>(&self, options: &'a [SelectOption]) -> Option<&'a str> {
        self.selected
            .and_then(|index| options.get(index))
            .map(|option| option.id.as_str())
    }

    pub fn selected_label<'a>(&self, options: &'a [SelectOption]) -> Option<&'a str> {
        self.selected
            .and_then(|index| options.get(index))
            .map(|option| option.label.as_str())
    }

    pub fn open(&mut self, options: &[SelectOption]) {
        self.open = true;
        self.active = self
            .selected
            .filter(|index| options.get(*index).is_some_and(|option| option.enabled))
            .or_else(|| first_enabled_select_index(options));
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn toggle(&mut self, options: &[SelectOption]) {
        if self.open {
            self.close();
        } else {
            self.open(options);
        }
    }

    pub fn move_active(
        &mut self,
        options: &[SelectOption],
        direction: NavigationDirection,
    ) -> Option<usize> {
        let active = next_enabled_select_index(options, self.active, direction);
        self.active = active;
        active
    }

    pub fn select_active(&mut self, options: &[SelectOption]) -> Option<SelectSelection> {
        let index = self.active?;
        let option = options.get(index)?;
        if !option.enabled {
            return None;
        }
        self.selected = Some(index);
        self.open = false;
        Some(SelectSelection {
            index,
            id: option.id.clone(),
        })
    }

    pub fn handle_event(
        &mut self,
        options: &[SelectOption],
        event: &UiInputEvent,
    ) -> SelectMenuOutcome {
        let mut outcome = SelectMenuOutcome::default();
        if let UiInputEvent::TextInput(text) = event {
            return first_typeahead_character(text)
                .map(|character| self.handle_typeahead(options, character))
                .unwrap_or(outcome);
        }

        let UiInputEvent::Key { key, .. } = event else {
            return outcome;
        };

        match *key {
            KeyCode::ArrowDown => {
                if !self.open {
                    self.open(options);
                    outcome.opened = true;
                    outcome.active = self.active;
                } else {
                    outcome.active = self.move_active(options, NavigationDirection::Next);
                }
            }
            KeyCode::ArrowUp => {
                if !self.open {
                    self.open = true;
                    self.active = self
                        .selected
                        .filter(|index| options.get(*index).is_some_and(|option| option.enabled))
                        .or_else(|| last_enabled_select_index(options));
                    outcome.opened = true;
                    outcome.active = self.active;
                } else {
                    outcome.active = self.move_active(options, NavigationDirection::Previous);
                }
            }
            KeyCode::Home if self.open => {
                self.active = first_enabled_select_index(options);
                outcome.active = self.active;
            }
            KeyCode::End if self.open => {
                self.active = last_enabled_select_index(options);
                outcome.active = self.active;
            }
            KeyCode::Enter | KeyCode::Character(' ') if self.open => {
                outcome.selected = self.select_active(options);
                outcome.closed = outcome.selected.is_some();
            }
            KeyCode::Enter | KeyCode::Character(' ') => {
                self.open(options);
                outcome.opened = true;
                outcome.active = self.active;
            }
            KeyCode::Escape if self.open => {
                self.close();
                outcome.closed = true;
            }
            KeyCode::Character(character) if is_typeahead_character(character) => {
                return self.handle_typeahead(options, character);
            }
            _ => {}
        }

        outcome
    }

    fn handle_typeahead(&mut self, options: &[SelectOption], character: char) -> SelectMenuOutcome {
        let mut outcome = SelectMenuOutcome::default();
        let Some(index) =
            next_select_typeahead_index(options, self.active.or(self.selected), character)
        else {
            return outcome;
        };

        self.active = Some(index);
        if self.open {
            outcome.active = Some(index);
        } else if let Some(option) = options.get(index) {
            self.selected = Some(index);
            outcome.selected = Some(SelectSelection {
                index,
                id: option.id.clone(),
            });
        }
        outcome
    }
}

impl Default for SelectMenuState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectMenuOutcome {
    pub opened: bool,
    pub closed: bool,
    pub active: Option<usize>,
    pub selected: Option<SelectSelection>,
}

impl SelectMenuOutcome {
    pub fn is_empty(&self) -> bool {
        !self.opened && !self.closed && self.active.is_none() && self.selected.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFilterTiming {
    Immediate,
    Debounced { delay_ms: u64 },
}

impl SearchFilterTiming {
    pub const IMMEDIATE: Self = Self::Immediate;

    pub const fn debounced(delay_ms: u64) -> Self {
        Self::Debounced { delay_ms }
    }
}

impl Default for SearchFilterTiming {
    fn default() -> Self {
        Self::Immediate
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchFilterRequest {
    pub query: String,
    pub revision: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchFieldOutcome {
    pub query_changed: bool,
    pub cleared: bool,
    pub filter_pending: bool,
    pub close_requested: bool,
}

impl SearchFieldOutcome {
    pub fn is_empty(&self) -> bool {
        !self.query_changed && !self.cleared && !self.filter_pending && !self.close_requested
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchClearButtonMeta {
    pub id: String,
    pub label: String,
    pub accessibility_label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
}

impl SearchClearButtonMeta {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        accessibility_label: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            accessibility_label: accessibility_label.into(),
            shortcut: Some("Escape".to_string()),
            enabled: true,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn without_shortcut(mut self) -> Self {
        self.shortcut = None;
        self
    }

    pub fn accessibility(&self) -> AccessibilityMeta {
        let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Button)
            .label(self.accessibility_label.clone());
        if let Some(shortcut) = &self.shortcut {
            accessibility = accessibility.shortcut(shortcut.clone());
        }
        if self.enabled {
            accessibility.focusable()
        } else {
            accessibility.disabled()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchStatusText {
    pub query: String,
    pub visible_count: usize,
    pub total_count: usize,
    pub text: String,
}

impl SearchStatusText {
    pub fn new(
        query: impl AsRef<str>,
        visible_count: usize,
        total_count: usize,
        singular: impl AsRef<str>,
        plural: impl AsRef<str>,
    ) -> Self {
        let query = query.as_ref().trim().to_string();
        let singular = singular.as_ref();
        let plural = plural.as_ref();
        let text = if query.is_empty() {
            format!(
                "{} available",
                search_count_label(total_count, singular, plural)
            )
        } else if visible_count == 0 {
            format!("No {plural} match \"{query}\"")
        } else {
            let verb = if visible_count == 1 {
                "matches"
            } else {
                "match"
            };
            format!(
                "{} {verb} \"{query}\"",
                search_count_label(visible_count, singular, plural)
            )
        };

        Self {
            query,
            visible_count,
            total_count,
            text,
        }
    }

    pub fn accessibility(&self, label: impl Into<String>) -> AccessibilityMeta {
        AccessibilityMeta::new(AccessibilityRole::Status)
            .label(label)
            .value(self.text.clone())
            .live_region(AccessibilityLiveRegion::Polite)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchFieldState {
    pub query: String,
    pub revision: u64,
    pub filtered_revision: u64,
    pub last_changed_at_ms: Option<u64>,
}

impl SearchFieldState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            revision: 0,
            filtered_revision: 0,
            last_changed_at_ms: None,
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    pub fn from_query(query: impl Into<String>) -> Self {
        Self::new().with_query(query)
    }

    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    pub fn is_filter_pending(&self) -> bool {
        self.filtered_revision != self.revision
    }

    pub fn clear_button(&self) -> Option<SearchClearButtonMeta> {
        (!self.query.is_empty())
            .then(|| SearchClearButtonMeta::new("clear-search", "Clear", "Clear search"))
    }

    pub fn input_accessibility(&self, label: impl Into<String>) -> AccessibilityMeta {
        let mut accessibility = AccessibilityMeta::new(AccessibilityRole::SearchBox)
            .label(label)
            .value(self.query.clone())
            .focusable();
        if let Some(clear_button) = self.clear_button() {
            let mut action =
                AccessibilityAction::new(clear_button.id, clear_button.accessibility_label);
            if let Some(shortcut) = clear_button.shortcut {
                action = action.shortcut(shortcut);
            }
            accessibility = accessibility.action(action);
        }
        accessibility
    }

    pub fn status(
        &self,
        visible_count: usize,
        total_count: usize,
        singular: impl AsRef<str>,
        plural: impl AsRef<str>,
    ) -> SearchStatusText {
        SearchStatusText::new(&self.query, visible_count, total_count, singular, plural)
    }

    pub fn status_accessibility(
        &self,
        label: impl Into<String>,
        visible_count: usize,
        total_count: usize,
        singular: impl AsRef<str>,
        plural: impl AsRef<str>,
    ) -> AccessibilityMeta {
        self.status(visible_count, total_count, singular, plural)
            .accessibility(label)
    }

    pub fn set_query(&mut self, query: impl Into<String>, now_ms: u64) -> SearchFieldOutcome {
        let query = query.into();
        if self.query == query {
            return SearchFieldOutcome::default();
        }

        let cleared = !self.query.is_empty() && query.is_empty();
        self.query = query;
        self.revision = self.revision.saturating_add(1);
        self.last_changed_at_ms = Some(now_ms);
        SearchFieldOutcome {
            query_changed: true,
            cleared,
            filter_pending: true,
            close_requested: false,
        }
    }

    pub fn push_text(&mut self, text: impl AsRef<str>, now_ms: u64) -> SearchFieldOutcome {
        let text = text.as_ref();
        if text.is_empty() {
            return SearchFieldOutcome::default();
        }
        let mut query = self.query.clone();
        query.push_str(text);
        self.set_query(query, now_ms)
    }

    pub fn backspace(&mut self, now_ms: u64) -> SearchFieldOutcome {
        let mut query = self.query.clone();
        if pop_last_char(&mut query) {
            self.set_query(query, now_ms)
        } else {
            SearchFieldOutcome::default()
        }
    }

    pub fn clear(&mut self, now_ms: u64) -> SearchFieldOutcome {
        self.set_query(String::new(), now_ms)
    }

    pub fn handle_event(&mut self, event: &UiInputEvent, now_ms: u64) -> SearchFieldOutcome {
        match event {
            UiInputEvent::TextInput(text) => self.push_text(text, now_ms),
            UiInputEvent::Key {
                key: KeyCode::Backspace,
                modifiers,
            } if search_field_clear_modifier(*modifiers) => self.clear(now_ms),
            UiInputEvent::Key {
                key: KeyCode::Backspace,
                ..
            } => self.backspace(now_ms),
            UiInputEvent::Key {
                key: KeyCode::Delete,
                modifiers,
            } if search_field_clear_modifier(*modifiers) => self.clear(now_ms),
            UiInputEvent::Key {
                key: KeyCode::Escape,
                ..
            } if self.query.is_empty() => SearchFieldOutcome {
                close_requested: true,
                ..Default::default()
            },
            UiInputEvent::Key {
                key: KeyCode::Escape,
                ..
            } => self.clear(now_ms),
            _ => SearchFieldOutcome::default(),
        }
    }

    pub fn filter_request(
        &self,
        now_ms: u64,
        timing: SearchFilterTiming,
    ) -> Option<SearchFilterRequest> {
        if !self.is_filter_pending() {
            return None;
        }

        let elapsed_ms = self
            .last_changed_at_ms
            .map(|changed_at| now_ms.saturating_sub(changed_at))
            .unwrap_or(0);
        match timing {
            SearchFilterTiming::Immediate => {}
            SearchFilterTiming::Debounced { delay_ms } if elapsed_ms >= delay_ms => {}
            SearchFilterTiming::Debounced { .. } => return None,
        }

        Some(SearchFilterRequest {
            query: self.query.clone(),
            revision: self.revision,
            elapsed_ms,
        })
    }

    pub fn take_filter_request(
        &mut self,
        now_ms: u64,
        timing: SearchFilterTiming,
    ) -> Option<SearchFilterRequest> {
        let request = self.filter_request(now_ms, timing)?;
        self.mark_filter_applied(request.revision);
        Some(request)
    }

    pub fn mark_filter_applied(&mut self, revision: u64) {
        if revision > self.filtered_revision && revision <= self.revision {
            self.filtered_revision = revision;
        }
    }
}

impl Default for SearchFieldState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOptionFilterMatch {
    pub visible_index: usize,
    pub option_index: usize,
    pub id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOptionFilterEmptyState {
    pub query: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOptionFilterState {
    pub query: String,
    pub active_match: Option<usize>,
    pub empty_label: String,
}

impl SelectOptionFilterState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            active_match: None,
            empty_label: "No options".to_string(),
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self.active_match = None;
        self
    }

    pub fn with_empty_label(mut self, label: impl Into<String>) -> Self {
        self.empty_label = label.into();
        self
    }

    pub fn search_field(&self) -> SearchFieldState {
        SearchFieldState::from_query(self.query.clone())
    }

    pub fn apply_search_field(
        &mut self,
        field: &SearchFieldState,
        options: &[SelectOption],
    ) -> SelectOptionFilterOutcome {
        let query_changed = self.query != field.query;
        self.query = field.query.clone();
        let matches = self.matches(options);
        self.active_match = first_enabled_select_option_match(options, &matches);
        SelectOptionFilterOutcome {
            query_changed,
            active_match: self.active_match,
            ..Default::default()
        }
    }

    pub fn clear_query(&mut self, options: &[SelectOption]) -> SelectOptionFilterOutcome {
        if self.query.is_empty() {
            return SelectOptionFilterOutcome::default();
        }
        self.set_query("", options);
        SelectOptionFilterOutcome {
            query_changed: true,
            active_match: self.active_match,
            ..Default::default()
        }
    }

    pub fn filtered_indices(&self, options: &[SelectOption]) -> Vec<usize> {
        filter_select_option_indices(options, &self.query)
    }

    pub fn matches(&self, options: &[SelectOption]) -> Vec<SelectOptionFilterMatch> {
        filter_select_options(options, &self.query)
    }

    pub fn visible_count(&self, options: &[SelectOption]) -> usize {
        self.filtered_indices(options).len()
    }

    pub fn is_empty(&self, options: &[SelectOption]) -> bool {
        self.filtered_indices(options).is_empty()
    }

    pub fn empty_state(&self, options: &[SelectOption]) -> Option<SelectOptionFilterEmptyState> {
        self.is_empty(options)
            .then(|| SelectOptionFilterEmptyState {
                query: self.query.clone(),
                label: self.empty_label.clone(),
            })
    }

    pub fn search_status(&self, options: &[SelectOption]) -> SearchStatusText {
        self.search_field().status(
            self.visible_count(options),
            options.len(),
            "option",
            "options",
        )
    }

    pub fn search_status_accessibility(&self, options: &[SelectOption]) -> AccessibilityMeta {
        self.search_status(options)
            .accessibility("Option search results")
    }

    pub fn set_query(&mut self, query: impl Into<String>, options: &[SelectOption]) {
        self.query = query.into();
        let matches = self.matches(options);
        self.active_match = first_enabled_select_option_match(options, &matches);
    }

    pub fn active_option_index(&self, options: &[SelectOption]) -> Option<usize> {
        let matches = self.matches(options);
        self.active_match
            .and_then(|index| matches.get(index))
            .map(|option_match| option_match.option_index)
    }

    pub fn active_option<'a>(&self, options: &'a [SelectOption]) -> Option<&'a SelectOption> {
        self.active_option_index(options)
            .and_then(|index| options.get(index))
    }

    pub fn active_id<'a>(&self, options: &'a [SelectOption]) -> Option<&'a str> {
        self.active_option(options).map(|option| option.id.as_str())
    }

    pub fn active_label<'a>(&self, options: &'a [SelectOption]) -> Option<&'a str> {
        self.active_option(options)
            .map(|option| option.label.as_str())
    }

    pub fn active_accessibility_label(&self, options: &[SelectOption]) -> Option<String> {
        self.active_option(options).map(option_accessibility_label)
    }

    pub fn accessibility_value(&self, options: &[SelectOption]) -> String {
        let count = self.visible_count(options);
        if count == 0 {
            self.empty_label.clone()
        } else if self.query.is_empty() {
            format!("{count} options")
        } else {
            format!("{count} filtered options")
        }
    }

    pub fn move_active(
        &mut self,
        options: &[SelectOption],
        direction: NavigationDirection,
    ) -> Option<usize> {
        let matches = self.matches(options);
        let active =
            next_enabled_select_option_match(options, &matches, self.active_match, direction);
        self.active_match = active;
        active
    }

    pub fn select_active(&self, options: &[SelectOption]) -> Option<SelectSelection> {
        let matches = self.matches(options);
        let active = self.active_match?;
        let option_match = matches.get(active)?;
        let option = options.get(option_match.option_index)?;
        if !option.enabled {
            return None;
        }
        Some(SelectSelection {
            index: option_match.option_index,
            id: option.id.clone(),
        })
    }

    pub fn handle_event(
        &mut self,
        options: &[SelectOption],
        event: &UiInputEvent,
    ) -> SelectOptionFilterOutcome {
        let mut outcome = SelectOptionFilterOutcome::default();
        match event {
            UiInputEvent::TextInput(text) => {
                self.query.push_str(text);
                let matches = self.matches(options);
                self.active_match = first_enabled_select_option_match(options, &matches);
                outcome.query_changed = true;
                outcome.active_match = self.active_match;
            }
            UiInputEvent::Key { key, .. } => match key {
                KeyCode::Backspace => {
                    if pop_last_char(&mut self.query) {
                        let matches = self.matches(options);
                        self.active_match = first_enabled_select_option_match(options, &matches);
                        outcome.query_changed = true;
                        outcome.active_match = self.active_match;
                    }
                }
                KeyCode::ArrowDown => {
                    outcome.active_match = self.move_active(options, NavigationDirection::Next);
                }
                KeyCode::ArrowUp => {
                    outcome.active_match = self.move_active(options, NavigationDirection::Previous);
                }
                KeyCode::Home => {
                    let matches = self.matches(options);
                    self.active_match = first_enabled_select_option_match(options, &matches);
                    outcome.active_match = self.active_match;
                }
                KeyCode::End => {
                    let matches = self.matches(options);
                    self.active_match = last_enabled_select_option_match(options, &matches);
                    outcome.active_match = self.active_match;
                }
                KeyCode::Enter => {
                    outcome.selected = self.select_active(options);
                }
                KeyCode::Escape => outcome.closed = true,
                _ => {}
            },
            _ => {}
        }
        outcome
    }
}

impl Default for SelectOptionFilterState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectOptionFilterOutcome {
    pub query_changed: bool,
    pub active_match: Option<usize>,
    pub selected: Option<SelectSelection>,
    pub closed: bool,
}

impl SelectOptionFilterOutcome {
    pub fn is_empty(&self) -> bool {
        !self.query_changed
            && self.active_match.is_none()
            && self.selected.is_none()
            && !self.closed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchableSelectCloseReason {
    Escape,
    Outside,
    Selection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchableSelectState {
    pub open: bool,
    pub selected: Option<usize>,
    pub filter: SelectOptionFilterState,
}

impl SearchableSelectState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected: None,
            filter: SelectOptionFilterState::new(),
        }
    }

    pub fn with_selected(selected: usize) -> Self {
        Self {
            open: false,
            selected: Some(selected),
            filter: SelectOptionFilterState::new(),
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.filter = self.filter.with_query(query);
        self
    }

    pub fn search_field(&self) -> SearchFieldState {
        self.filter.search_field()
    }

    pub fn selected_id<'a>(&self, options: &'a [SelectOption]) -> Option<&'a str> {
        self.selected
            .and_then(|index| options.get(index))
            .map(|option| option.id.as_str())
    }

    pub fn selected_label<'a>(&self, options: &'a [SelectOption]) -> Option<&'a str> {
        self.selected
            .and_then(|index| options.get(index))
            .map(|option| option.label.as_str())
    }

    pub fn open(&mut self, options: &[SelectOption]) {
        self.open = true;
        if self.filter.active_match.is_none() {
            let matches = self.filter.matches(options);
            self.filter.active_match = self
                .selected
                .and_then(|selected| {
                    matches
                        .iter()
                        .position(|option_match| option_match.option_index == selected)
                })
                .filter(|match_index| options[matches[*match_index].option_index].enabled)
                .or_else(|| first_enabled_select_option_match(options, &matches));
        }
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn dismiss(&mut self, reason: SearchableSelectCloseReason) -> SearchableSelectOutcome {
        if !self.open {
            return SearchableSelectOutcome::default();
        }
        self.close();
        SearchableSelectOutcome {
            closed: true,
            close_reason: Some(reason),
            ..Default::default()
        }
    }

    pub fn apply_search_field(
        &mut self,
        field: &SearchFieldState,
        options: &[SelectOption],
    ) -> SearchableSelectOutcome {
        let filter_outcome = self.filter.apply_search_field(field, options);
        SearchableSelectOutcome {
            query_changed: filter_outcome.query_changed,
            active_match: filter_outcome.active_match,
            ..Default::default()
        }
    }

    pub fn move_active(
        &mut self,
        options: &[SelectOption],
        direction: NavigationDirection,
    ) -> Option<usize> {
        self.filter.move_active(options, direction)
    }

    pub fn select_active(&mut self, options: &[SelectOption]) -> Option<SelectSelection> {
        let selection = self.filter.select_active(options)?;
        self.selected = Some(selection.index);
        self.open = false;
        Some(selection)
    }

    pub fn handle_outside_dismiss(&mut self) -> SearchableSelectOutcome {
        self.dismiss(SearchableSelectCloseReason::Outside)
    }

    pub fn handle_event(
        &mut self,
        options: &[SelectOption],
        event: &UiInputEvent,
    ) -> SearchableSelectOutcome {
        let mut outcome = SearchableSelectOutcome::default();
        match event {
            UiInputEvent::TextInput(text) => {
                if !self.open {
                    self.open(options);
                    outcome.opened = true;
                }
                let filter_outcome = self
                    .filter
                    .handle_event(options, &UiInputEvent::TextInput(text.clone()));
                outcome.query_changed = filter_outcome.query_changed;
                outcome.active_match = filter_outcome.active_match;
            }
            UiInputEvent::Key { key, .. } => match key {
                KeyCode::ArrowDown => {
                    if !self.open {
                        self.open(options);
                        outcome.opened = true;
                        outcome.active_match = self.filter.active_match;
                    } else {
                        outcome.active_match = self.move_active(options, NavigationDirection::Next);
                    }
                }
                KeyCode::ArrowUp => {
                    if !self.open {
                        self.open(options);
                        outcome.opened = true;
                        let matches = self.filter.matches(options);
                        self.filter.active_match =
                            last_enabled_select_option_match(options, &matches);
                        outcome.active_match = self.filter.active_match;
                    } else {
                        outcome.active_match =
                            self.move_active(options, NavigationDirection::Previous);
                    }
                }
                KeyCode::Home if self.open => {
                    let matches = self.filter.matches(options);
                    self.filter.active_match = first_enabled_select_option_match(options, &matches);
                    outcome.active_match = self.filter.active_match;
                }
                KeyCode::End if self.open => {
                    let matches = self.filter.matches(options);
                    self.filter.active_match = last_enabled_select_option_match(options, &matches);
                    outcome.active_match = self.filter.active_match;
                }
                KeyCode::Backspace => {
                    let filter_outcome = self.filter.handle_event(options, event);
                    outcome.query_changed = filter_outcome.query_changed;
                    outcome.active_match = filter_outcome.active_match;
                }
                KeyCode::Enter if self.open => {
                    outcome.selected = self.select_active(options);
                    if outcome.selected.is_some() {
                        outcome.closed = true;
                        outcome.close_reason = Some(SearchableSelectCloseReason::Selection);
                    }
                }
                KeyCode::Enter | KeyCode::Character(' ') => {
                    self.open(options);
                    outcome.opened = true;
                    outcome.active_match = self.filter.active_match;
                }
                KeyCode::Escape if self.open && self.filter.query.is_empty() => {
                    self.close();
                    outcome.closed = true;
                    outcome.close_reason = Some(SearchableSelectCloseReason::Escape);
                }
                KeyCode::Escape if self.open => {
                    let filter_outcome = self.filter.clear_query(options);
                    outcome.query_changed = filter_outcome.query_changed;
                    outcome.active_match = filter_outcome.active_match;
                }
                _ => {}
            },
            _ => {}
        }
        outcome
    }
}

impl Default for SearchableSelectState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchableSelectOutcome {
    pub opened: bool,
    pub closed: bool,
    pub close_reason: Option<SearchableSelectCloseReason>,
    pub query_changed: bool,
    pub active_match: Option<usize>,
    pub selected: Option<SelectSelection>,
}

impl SearchableSelectOutcome {
    pub fn is_empty(&self) -> bool {
        !self.opened
            && !self.closed
            && self.close_reason.is_none()
            && !self.query_changed
            && self.active_match.is_none()
            && self.selected.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchableSelectSpec {
    pub max_visible_rows: usize,
    pub placeholder: String,
    pub accessibility_label: Option<String>,
    pub search_label: String,
    pub list_label: String,
}

impl SearchableSelectSpec {
    pub fn new() -> Self {
        Self {
            max_visible_rows: 8,
            placeholder: String::new(),
            accessibility_label: None,
            search_label: "Search options".to_string(),
            list_label: "Options".to_string(),
        }
    }

    pub fn max_visible_rows(mut self, rows: usize) -> Self {
        self.max_visible_rows = rows;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn search_label(mut self, label: impl Into<String>) -> Self {
        self.search_label = label.into();
        self
    }

    pub fn list_label(mut self, label: impl Into<String>) -> Self {
        self.list_label = label.into();
        self
    }
}

impl Default for SearchableSelectSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchableSelectRow {
    pub id: String,
    pub match_index: usize,
    pub option_index: usize,
    pub option_id: String,
    pub label: String,
    pub enabled: bool,
    pub selected: bool,
    pub active: bool,
    pub accessibility: AccessibilityMeta,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchableSelectContract {
    pub id: String,
    pub open: bool,
    pub selected_id: Option<String>,
    pub query: String,
    pub total_count: usize,
    pub match_count: usize,
    pub visible_range: std::ops::Range<usize>,
    pub active_descendant_id: Option<String>,
    pub trigger_accessibility: AccessibilityMeta,
    pub search_accessibility: AccessibilityMeta,
    pub listbox_accessibility: AccessibilityMeta,
    pub status: SearchStatusText,
    pub empty_state: Option<SelectOptionFilterEmptyState>,
    pub rows: Vec<SearchableSelectRow>,
}

pub fn searchable_select_contract(
    id: impl Into<String>,
    options: &[SelectOption],
    state: &SearchableSelectState,
    spec: SearchableSelectSpec,
) -> SearchableSelectContract {
    let id = id.into();
    let matches = state.filter.matches(options);
    let visible_range = visible_match_range(
        matches.len(),
        state.filter.active_match,
        spec.max_visible_rows,
    );
    let status = state.filter.search_status(options);
    let active_descendant_id = state.filter.active_match.and_then(|active| {
        visible_range
            .contains(&active)
            .then(|| matches.get(active))
            .flatten()
            .map(|option_match| format!("{id}.option.{}", option_match.option_index))
    });
    let selected_label = state
        .selected_label(options)
        .unwrap_or(spec.placeholder.as_str());
    let selected_id = state.selected_id(options).map(ToString::to_string);
    let mut trigger_accessibility = AccessibilityMeta::new(AccessibilityRole::ComboBox)
        .label(
            spec.accessibility_label
                .clone()
                .unwrap_or_else(|| id.clone()),
        )
        .value(selected_label.to_string())
        .expanded(state.open)
        .focusable();
    if state.open {
        trigger_accessibility = trigger_accessibility.hint("Searchable listbox open");
    }

    let search_accessibility = state
        .search_field()
        .input_accessibility(spec.search_label.clone());
    let listbox_accessibility = AccessibilityMeta::new(AccessibilityRole::List)
        .label(spec.list_label.clone())
        .value(status.text.clone());
    let rows = matches[visible_range.clone()]
        .iter()
        .map(|option_match| {
            let option = &options[option_match.option_index];
            let active = state.filter.active_match == Some(option_match.visible_index);
            let selected = state.selected == Some(option_match.option_index);
            let mut accessibility = AccessibilityMeta::new(AccessibilityRole::ListItem)
                .label(option_accessibility_label(option))
                .value(if selected { "selected" } else { "not selected" })
                .selected(selected);
            if active {
                accessibility = accessibility.hint("Active option");
            }
            if option.enabled {
                accessibility = accessibility.focusable();
            } else {
                accessibility = accessibility.disabled();
            }
            SearchableSelectRow {
                id: format!("{id}.option.{}", option_match.option_index),
                match_index: option_match.visible_index,
                option_index: option_match.option_index,
                option_id: option.id.clone(),
                label: option.label.clone(),
                enabled: option.enabled,
                selected,
                active,
                accessibility,
            }
        })
        .collect();

    SearchableSelectContract {
        id,
        open: state.open,
        selected_id,
        query: state.filter.query.clone(),
        total_count: options.len(),
        match_count: matches.len(),
        visible_range,
        active_descendant_id,
        trigger_accessibility,
        search_accessibility,
        listbox_accessibility,
        status,
        empty_state: state.filter.empty_state(options),
        rows,
    }
}

#[derive(Debug, Clone)]
pub struct SelectMenuOptions {
    pub width: f32,
    pub row_height: f32,
    pub max_visible_rows: usize,
    pub menu_visual: UiVisual,
    pub item_visual: UiVisual,
    pub active_visual: UiVisual,
    pub selected_visual: UiVisual,
    pub disabled_visual: UiVisual,
    pub text_style: TextStyle,
    pub disabled_text_style: TextStyle,
    pub image_size: UiSize,
    pub accessibility_label: Option<String>,
    pub menu_shader: Option<ShaderEffect>,
    pub active_shader: Option<ShaderEffect>,
    pub menu_animation: Option<AnimationMachine>,
    pub z_index: i16,
}

impl Default for SelectMenuOptions {
    fn default() -> Self {
        Self {
            width: 220.0,
            row_height: 28.0,
            max_visible_rows: 8,
            menu_visual: UiVisual::panel(
                ColorRgba::new(26, 31, 39, 255),
                Some(StrokeStyle::new(ColorRgba::new(77, 90, 111, 255), 1.0)),
                4.0,
            ),
            item_visual: UiVisual::TRANSPARENT,
            active_visual: UiVisual::panel(ColorRgba::new(58, 87, 126, 255), None, 2.0),
            selected_visual: UiVisual::panel(ColorRgba::new(42, 62, 87, 255), None, 2.0),
            disabled_visual: UiVisual::TRANSPARENT,
            text_style: TextStyle::default(),
            disabled_text_style: TextStyle {
                color: ColorRgba::new(138, 148, 164, 255),
                ..Default::default()
            },
            image_size: UiSize::new(18.0, 18.0),
            accessibility_label: None,
            menu_shader: None,
            active_shader: None,
            menu_animation: None,
            z_index: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectMenuNodes {
    pub root: UiNodeId,
    pub rows: Vec<UiNodeId>,
}

pub fn select_menu(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: &[SelectOption],
    state: &SelectMenuState,
    menu_options: SelectMenuOptions,
) -> SelectMenuNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        menu_container_node(name.clone(), options.len(), &menu_options),
    );
    let rows = populate_select_menu(document, root, &name, options, state, &menu_options);
    set_active_descendant(
        document,
        root,
        active_select_row(options, &rows, state.active),
    );
    SelectMenuNodes { root, rows }
}

pub fn select_menu_popup(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    popup: AnchoredPopup,
    options: &[SelectOption],
    state: &SelectMenuState,
    menu_options: SelectMenuOptions,
) -> SelectMenuNodes {
    let name = name.into();
    let height = visible_row_count(options.len(), menu_options.max_visible_rows) as f32
        * menu_options.row_height;
    let layout = place_popup(
        popup.anchor,
        UiSize::new(menu_options.width.max(0.0), height.max(0.0)),
        popup.viewport,
        popup.placement,
    );
    let root = popup_panel(
        document,
        parent,
        name.clone(),
        layout.rect,
        PopupOptions {
            visual: menu_options.menu_visual,
            z_index: menu_options.z_index,
            scroll_axes: if options.len() > menu_options.max_visible_rows {
                ScrollAxes::VERTICAL
            } else {
                ScrollAxes::NONE
            },
            accessibility: Some(AccessibilityMeta::new(AccessibilityRole::List).label(
                menu_accessibility_label(&name, menu_options.accessibility_label.as_ref()),
            )),
            shader: menu_options.menu_shader.clone(),
            animation: menu_options.menu_animation.clone(),
            ..Default::default()
        },
    );
    let rows = populate_select_menu(document, root, &name, options, state, &menu_options);
    set_active_descendant(
        document,
        root,
        active_select_row(options, &rows, state.active),
    );
    SelectMenuNodes { root, rows }
}

#[derive(Debug, Clone)]
pub struct DropdownSelectOptions {
    pub trigger_layout: LayoutStyle,
    pub trigger_visual: UiVisual,
    pub text_style: TextStyle,
    pub placeholder: String,
    pub accessibility_label: Option<String>,
    pub menu: SelectMenuOptions,
}

impl Default for DropdownSelectOptions {
    fn default() -> Self {
        Self {
            trigger_layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                size: TaffySize {
                    width: length(180.0),
                    height: length(30.0),
                },
                padding: TaffyRect::length(6.0),
                ..Default::default()
            }),
            trigger_visual: UiVisual::panel(
                ColorRgba::new(31, 37, 46, 255),
                Some(StrokeStyle::new(ColorRgba::new(84, 98, 121, 255), 1.0)),
                4.0,
            ),
            text_style: TextStyle::default(),
            placeholder: String::new(),
            accessibility_label: None,
            menu: SelectMenuOptions::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropdownSelectNodes {
    pub trigger: UiNodeId,
    pub popup: Option<SelectMenuNodes>,
}

pub fn dropdown_select(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: &[SelectOption],
    state: &SelectMenuState,
    popup: Option<AnchoredPopup>,
    dropdown_options: DropdownSelectOptions,
) -> DropdownSelectNodes {
    let name = name.into();
    let label = state
        .selected_label(options)
        .unwrap_or(dropdown_options.placeholder.as_str());
    let trigger = button_like(
        document,
        parent,
        name.clone(),
        label,
        dropdown_options.trigger_layout,
        dropdown_options.trigger_visual,
        dropdown_options.text_style,
    );
    document.node_mut(trigger).accessibility = Some(
        AccessibilityMeta::new(AccessibilityRole::ComboBox)
            .label(
                dropdown_options
                    .accessibility_label
                    .clone()
                    .unwrap_or_else(|| name.clone()),
            )
            .value(label.to_string())
            .expanded(state.open)
            .focusable(),
    );
    let popup = state.open.then(|| {
        popup.map(|popup| {
            select_menu_popup(
                document,
                parent,
                format!("{name}.popup"),
                popup,
                options,
                state,
                dropdown_options.menu,
            )
        })
    });
    let popup = popup.flatten();
    if let Some(popup) = &popup {
        set_active_descendant(
            document,
            trigger,
            active_select_row(options, &popup.rows, state.active),
        );
    }

    DropdownSelectNodes { trigger, popup }
}

#[derive(Debug, Clone)]
pub struct MenuListOptions {
    pub width: f32,
    pub row_height: f32,
    pub separator_height: f32,
    pub max_visible_rows: usize,
    pub menu_visual: UiVisual,
    pub item_visual: UiVisual,
    pub active_visual: UiVisual,
    pub disabled_visual: UiVisual,
    pub text_style: TextStyle,
    pub disabled_text_style: TextStyle,
    pub shortcut_text_style: TextStyle,
    pub destructive_text_style: TextStyle,
    pub image_size: UiSize,
    pub accessibility_label: Option<String>,
    pub menu_shader: Option<ShaderEffect>,
    pub active_shader: Option<ShaderEffect>,
    pub menu_animation: Option<AnimationMachine>,
    pub z_index: i16,
}

impl Default for MenuListOptions {
    fn default() -> Self {
        Self {
            width: 240.0,
            row_height: 28.0,
            separator_height: 8.0,
            max_visible_rows: 12,
            menu_visual: UiVisual::panel(
                ColorRgba::new(26, 31, 39, 255),
                Some(StrokeStyle::new(ColorRgba::new(77, 90, 111, 255), 1.0)),
                4.0,
            ),
            item_visual: UiVisual::TRANSPARENT,
            active_visual: UiVisual::panel(ColorRgba::new(58, 87, 126, 255), None, 2.0),
            disabled_visual: UiVisual::TRANSPARENT,
            text_style: TextStyle::default(),
            disabled_text_style: TextStyle {
                color: ColorRgba::new(138, 148, 164, 255),
                ..Default::default()
            },
            shortcut_text_style: TextStyle {
                color: ColorRgba::new(178, 188, 204, 255),
                ..Default::default()
            },
            destructive_text_style: TextStyle {
                color: ColorRgba::new(238, 116, 106, 255),
                ..Default::default()
            },
            image_size: UiSize::new(18.0, 18.0),
            accessibility_label: None,
            menu_shader: None,
            active_shader: None,
            menu_animation: None,
            z_index: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuListNodes {
    pub root: UiNodeId,
    pub rows: Vec<UiNodeId>,
}

pub fn menu_list(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    items: &[MenuItem],
    active: Option<usize>,
    options: MenuListOptions,
) -> MenuListNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        menu_list_container_node(name.clone(), items, &options),
    );
    let rows = populate_menu_list(document, root, &name, items, active, &options);
    set_active_descendant(document, root, active_menu_row(items, &rows, active));
    MenuListNodes { root, rows }
}

pub fn menu_list_popup(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    popup: AnchoredPopup,
    items: &[MenuItem],
    active: Option<usize>,
    options: MenuListOptions,
) -> MenuListNodes {
    let name = name.into();
    let height = visible_menu_height(items, &options);
    let layout = place_popup(
        popup.anchor,
        UiSize::new(options.width.max(0.0), height.max(0.0)),
        popup.viewport,
        popup.placement,
    );
    let root = popup_panel(
        document,
        parent,
        name.clone(),
        layout.rect,
        PopupOptions {
            visual: options.menu_visual,
            z_index: options.z_index,
            scroll_axes: if menu_row_count_for_scroll(items) > options.max_visible_rows {
                ScrollAxes::VERTICAL
            } else {
                ScrollAxes::NONE
            },
            accessibility: Some(AccessibilityMeta::new(AccessibilityRole::Menu).label(
                menu_accessibility_label(&name, options.accessibility_label.as_ref()),
            )),
            shader: options.menu_shader.clone(),
            animation: options.menu_animation.clone(),
            ..Default::default()
        },
    );
    let rows = populate_menu_list(document, root, &name, items, active, &options);
    set_active_descendant(document, root, active_menu_row(items, &rows, active));
    MenuListNodes { root, rows }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenuState {
    pub open: bool,
    pub anchor: UiPoint,
    pub active: Option<usize>,
}

impl ContextMenuState {
    pub const fn closed() -> Self {
        Self {
            open: false,
            anchor: UiPoint::new(0.0, 0.0),
            active: None,
        }
    }

    pub const fn open_at(anchor: UiPoint) -> Self {
        Self {
            open: true,
            anchor,
            active: None,
        }
    }

    pub fn open_with_items(&mut self, anchor: UiPoint, items: &[MenuItem]) {
        self.open = true;
        self.anchor = anchor;
        self.active = first_navigable_index(items);
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn move_active(
        &mut self,
        items: &[MenuItem],
        direction: NavigationDirection,
    ) -> Option<usize> {
        let active = next_navigable_index(items, self.active, direction);
        self.active = active;
        active
    }

    pub fn handle_event(&mut self, items: &[MenuItem], event: &UiInputEvent) -> MenuOutcome {
        let mut outcome = MenuOutcome::default();
        if !self.open {
            return outcome;
        }

        if let UiInputEvent::TextInput(text) = event {
            if let Some(character) = first_typeahead_character(text) {
                outcome.active = self.move_active_to_match(items, character);
            }
            return outcome;
        }

        let UiInputEvent::Key { key, .. } = event else {
            return outcome;
        };

        match *key {
            KeyCode::ArrowDown => {
                outcome.active = self.move_active(items, NavigationDirection::Next)
            }
            KeyCode::ArrowUp => {
                outcome.active = self.move_active(items, NavigationDirection::Previous)
            }
            KeyCode::Home => {
                self.active = first_navigable_index(items);
                outcome.active = self.active;
            }
            KeyCode::End => {
                self.active = last_navigable_index(items);
                outcome.active = self.active;
            }
            KeyCode::Enter | KeyCode::Character(' ') => {
                if let Some(index) = self.active {
                    outcome.selected = menu_selection_at_path(items, &[index]);
                    if outcome.selected.is_some() {
                        self.close();
                        outcome.closed = true;
                    }
                }
            }
            KeyCode::Escape => {
                self.close();
                outcome.closed = true;
            }
            KeyCode::Character(character) if is_typeahead_character(character) => {
                outcome.active = self.move_active_to_match(items, character);
            }
            _ => {}
        }

        outcome
    }

    fn move_active_to_match(&mut self, items: &[MenuItem], character: char) -> Option<usize> {
        let active = next_menu_typeahead_index(items, self.active, character);
        self.active = active;
        active
    }
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self::closed()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuOutcome {
    pub opened: bool,
    pub closed: bool,
    pub active: Option<usize>,
    pub selected: Option<MenuSelection>,
}

impl MenuOutcome {
    pub fn selected_command(&self) -> Option<MenuCommandSelection> {
        self.selected
            .as_ref()
            .and_then(MenuSelection::command_selection)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn context_menu(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    items: &[MenuItem],
    state: &ContextMenuState,
    viewport: UiRect,
    placement: PopupPlacement,
    options: MenuListOptions,
) -> Option<MenuListNodes> {
    if !state.open {
        return None;
    }
    Some(menu_list_popup(
        document,
        parent,
        name,
        AnchoredPopup::new(
            UiRect::new(state.anchor.x, state.anchor.y, 1.0, 1.0),
            viewport,
            placement,
        ),
        items,
        state.active,
        options,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuBarMenu {
    pub id: String,
    pub label: String,
    pub items: Vec<MenuItem>,
    pub enabled: bool,
}

impl MenuBarMenu {
    pub fn new(id: impl Into<String>, label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            items,
            enabled: true,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuBarState {
    pub open_menu: Option<usize>,
    pub active_item: Option<usize>,
}

impl MenuBarState {
    pub fn open(&mut self, menus: &[MenuBarMenu], index: usize) -> bool {
        let Some(menu) = menus.get(index) else {
            return false;
        };
        if !menu.enabled {
            return false;
        }
        self.open_menu = Some(index);
        self.active_item = first_navigable_index(&menu.items);
        true
    }

    pub fn close(&mut self) {
        self.open_menu = None;
        self.active_item = None;
    }

    pub fn move_menu(
        &mut self,
        menus: &[MenuBarMenu],
        direction: NavigationDirection,
    ) -> Option<usize> {
        let index = next_enabled_menu_bar_index(menus, self.open_menu, direction)?;
        self.open(menus, index);
        Some(index)
    }

    pub fn move_item(
        &mut self,
        menus: &[MenuBarMenu],
        direction: NavigationDirection,
    ) -> Option<usize> {
        let menu = self.open_menu.and_then(|index| menus.get(index))?;
        let active = next_navigable_index(&menu.items, self.active_item, direction);
        self.active_item = active;
        active
    }

    pub fn select_active(&self, menus: &[MenuBarMenu]) -> Option<MenuSelection> {
        let menu_index = self.open_menu?;
        let item_index = self.active_item?;
        let menu = menus.get(menu_index)?;
        let mut selection = menu_selection_at_path(&menu.items, &[item_index])?;
        selection.index_path.insert(0, menu_index);
        Some(selection)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuBarAnchors {
    pub anchors: Vec<UiRect>,
    pub viewport: UiRect,
}

#[derive(Debug, Clone)]
pub struct MenuBarOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub button_visual: UiVisual,
    pub active_button_visual: UiVisual,
    pub text_style: TextStyle,
    pub disabled_text_style: TextStyle,
    pub popup_placement: PopupPlacement,
    pub popup_menu: MenuListOptions,
}

impl Default for MenuBarOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(30.0),
                },
                ..Default::default()
            }),
            visual: UiVisual::panel(ColorRgba::new(22, 27, 34, 255), None, 0.0),
            button_visual: UiVisual::TRANSPARENT,
            active_button_visual: UiVisual::panel(ColorRgba::new(45, 55, 68, 255), None, 2.0),
            text_style: TextStyle::default(),
            disabled_text_style: TextStyle {
                color: ColorRgba::new(138, 148, 164, 255),
                ..Default::default()
            },
            popup_placement: PopupPlacement::new(PopupSide::Bottom, PopupAlign::Start),
            popup_menu: MenuListOptions::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuBarNodes {
    pub root: UiNodeId,
    pub buttons: Vec<UiNodeId>,
    pub popup: Option<MenuListNodes>,
}

pub fn menu_bar(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    menus: &[MenuBarMenu],
    state: &MenuBarState,
    anchors: Option<&MenuBarAnchors>,
    options: MenuBarOptions,
) -> MenuBarNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::MenuBar).label(name.clone())),
    );
    let mut buttons = Vec::with_capacity(menus.len());
    for (index, menu) in menus.iter().enumerate() {
        let active = state.open_menu == Some(index);
        let visual = if active {
            options.active_button_visual
        } else {
            options.button_visual
        };
        let text_style = if menu.enabled {
            options.text_style.clone()
        } else {
            options.disabled_text_style.clone()
        };
        let button = button_like_with_input(
            document,
            root,
            format!("{name}.{}", menu.id),
            &menu.label,
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::percent(1.0),
                },
                padding: TaffyRect {
                    left: length_percentage(10.0),
                    right: length_percentage(10.0),
                    top: length_percentage(0.0),
                    bottom: length_percentage(0.0),
                },
                ..Default::default()
            }),
            visual,
            text_style,
            if menu.enabled {
                InputBehavior::BUTTON
            } else {
                InputBehavior::NONE
            },
        );
        document.node_mut(button).accessibility = Some(menu_button_accessibility(menu, active));
        buttons.push(button);
    }
    set_active_descendant(
        document,
        root,
        state
            .open_menu
            .filter(|index| menus.get(*index).is_some_and(|menu| menu.enabled))
            .and_then(|index| buttons.get(index).copied()),
    );

    let popup = state
        .open_menu
        .and_then(|index| Some((index, menus.get(index)?)))
        .and_then(|(index, menu)| {
            let anchors = anchors?;
            let anchor = *anchors.anchors.get(index)?;
            Some(menu_list_popup(
                document,
                parent,
                format!("{name}.{}.popup", menu.id),
                AnchoredPopup::new(anchor, anchors.viewport, options.popup_placement),
                &menu.items,
                state.active_item,
                options.popup_menu,
            ))
        });

    MenuBarNodes {
        root,
        buttons,
        popup,
    }
}

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
    pub query: String,
    pub active_match: Option<usize>,
    pub max_results: usize,
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
                self.query.push_str(text);
                self.active_match = first_enabled_palette_match(items, &self.matches(items));
                outcome.query_changed = true;
                outcome.active_match = self.active_match;
            }
            UiInputEvent::Key { key, .. } => match key {
                KeyCode::Backspace => {
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
        }
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
                        width: length(options.width.max(0.0)),
                        height: length(height.max(0.0)),
                    },
                    padding: TaffyRect::length(4.0),
                    ..Default::default()
                }),
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
                }),
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
        ),
    );
    label(
        document,
        input,
        format!("{name}.query"),
        if state.query.is_empty() {
            ""
        } else {
            &state.query
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
            }),
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

    let mut rows = Vec::with_capacity(matches.len());
    for (match_index, palette_match) in matches.iter().enumerate() {
        let Some(item) = items.get(palette_match.index) else {
            continue;
        };
        let active = state.active_match == Some(match_index);
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
                    }),
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
    let active_row = state
        .active_match
        .and_then(|index| rows.get(index).copied());
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
            }),
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

fn popup_rect_for_anchor(
    anchor: UiRect,
    popup_size: UiSize,
    side: PopupSide,
    placement: PopupPlacement,
) -> UiRect {
    let offset = placement.offset.max(0.0);
    match side {
        PopupSide::Top => UiRect::new(
            aligned_x(anchor, popup_size.width, placement.align),
            anchor.y - popup_size.height - offset,
            popup_size.width,
            popup_size.height,
        ),
        PopupSide::Bottom => UiRect::new(
            aligned_x(anchor, popup_size.width, placement.align),
            anchor.bottom() + offset,
            popup_size.width,
            popup_size.height,
        ),
        PopupSide::Left => UiRect::new(
            anchor.x - popup_size.width - offset,
            aligned_y(anchor, popup_size.height, placement.align),
            popup_size.width,
            popup_size.height,
        ),
        PopupSide::Right => UiRect::new(
            anchor.right() + offset,
            aligned_y(anchor, popup_size.height, placement.align),
            popup_size.width,
            popup_size.height,
        ),
    }
}

fn aligned_x(anchor: UiRect, width: f32, align: PopupAlign) -> f32 {
    match align {
        PopupAlign::Start => anchor.x,
        PopupAlign::Center => anchor.x + (anchor.width - width) * 0.5,
        PopupAlign::End => anchor.right() - width,
    }
}

fn aligned_y(anchor: UiRect, height: f32, align: PopupAlign) -> f32 {
    match align {
        PopupAlign::Start => anchor.y,
        PopupAlign::Center => anchor.y + (anchor.height - height) * 0.5,
        PopupAlign::End => anchor.bottom() - height,
    }
}

fn inset_viewport(viewport: UiRect, margin: f32) -> UiRect {
    let x_margin = margin.min(viewport.width * 0.5);
    let y_margin = margin.min(viewport.height * 0.5);
    UiRect::new(
        viewport.x + x_margin,
        viewport.y + y_margin,
        (viewport.width - x_margin * 2.0).max(0.0),
        (viewport.height - y_margin * 2.0).max(0.0),
    )
}

fn overflow_amount(rect: UiRect, viewport: UiRect) -> f32 {
    (viewport.x - rect.x).max(0.0)
        + (rect.right() - viewport.right()).max(0.0)
        + (viewport.y - rect.y).max(0.0)
        + (rect.bottom() - viewport.bottom()).max(0.0)
}

fn constrain_rect_to_viewport(rect: UiRect, viewport: UiRect) -> UiRect {
    let (x, width) = constrain_axis(rect.x, rect.width, viewport.x, viewport.right());
    let (y, height) = constrain_axis(rect.y, rect.height, viewport.y, viewport.bottom());
    UiRect::new(x, y, width, height)
}

fn constrain_axis(start: f32, size: f32, min: f32, max: f32) -> (f32, f32) {
    let available = (max - min).max(0.0);
    let size = size.max(0.0).min(available);
    let max_start = max - size;
    let start = if max_start <= min {
        min
    } else {
        start.clamp(min, max_start)
    };
    (start, size)
}

fn absolute_rect_style(rect: UiRect) -> LayoutStyle {
    LayoutStyle::from_taffy_style(Style {
        position: Position::Absolute,
        inset: TaffyRect {
            left: LengthPercentageAuto::length(rect.x),
            right: LengthPercentageAuto::auto(),
            top: LengthPercentageAuto::length(rect.y),
            bottom: LengthPercentageAuto::auto(),
        },
        size: TaffySize {
            width: length(rect.width),
            height: length(rect.height),
        },
        ..Default::default()
    })
}

fn menu_container_node(
    name: impl Into<String>,
    item_count: usize,
    options: &SelectMenuOptions,
) -> UiNode {
    let name = name.into();
    let scroll = item_count > options.max_visible_rows;
    let height =
        visible_row_count(item_count, options.max_visible_rows) as f32 * options.row_height;
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: length(options.width.max(0.0)),
                    height: length(height.max(0.0)),
                },
                ..Default::default()
            }),
            clip: ClipBehavior::Clip,
            z_index: options.z_index,
            ..Default::default()
        },
    )
    .with_visual(options.menu_visual)
    .with_accessibility(AccessibilityMeta::new(AccessibilityRole::List).label(
        menu_accessibility_label(&name, options.accessibility_label.as_ref()),
    ));
    if scroll {
        node = node.with_scroll(ScrollAxes::VERTICAL);
    }
    if let Some(shader) = options.menu_shader.clone() {
        node = node.with_shader(shader);
    }
    if let Some(animation) = options.menu_animation.clone() {
        node = node.with_animation(animation);
    }
    node
}

fn populate_select_menu(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    options: &[SelectOption],
    state: &SelectMenuState,
    menu_options: &SelectMenuOptions,
) -> Vec<UiNodeId> {
    let mut rows = Vec::with_capacity(options.len());
    for (index, option) in options.iter().enumerate() {
        let text_style = if option.enabled {
            menu_options.text_style.clone()
        } else {
            menu_options.disabled_text_style.clone()
        };
        let row = document.add_child(
            parent,
            select_row_node(
                format!("{name}.option.{index}"),
                index,
                option,
                state,
                menu_options,
            ),
        );
        if let Some(image) = &option.image {
            leading_image(
                document,
                row,
                format!("{name}.option.{index}.image"),
                image.clone(),
                &option_accessibility_label(option),
                menu_options.image_size,
            );
        }
        label(
            document,
            row,
            format!("{name}.option.{index}.label"),
            &option.label,
            text_style,
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        );
        rows.push(row);
    }
    rows
}

fn select_row_visual(
    index: usize,
    option: &SelectOption,
    state: &SelectMenuState,
    options: &SelectMenuOptions,
) -> UiVisual {
    if !option.enabled {
        options.disabled_visual
    } else if state.active == Some(index) {
        options.active_visual
    } else if state.selected == Some(index) {
        options.selected_visual
    } else {
        options.item_visual
    }
}

fn select_row_node(
    name: impl Into<String>,
    index: usize,
    option: &SelectOption,
    state: &SelectMenuState,
    options: &SelectMenuOptions,
) -> UiNode {
    let selected = state.selected == Some(index);
    let active = state.active == Some(index);
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::ListItem)
        .label(option_accessibility_label(option))
        .value(if selected { "selected" } else { "not selected" })
        .selected(selected);
    if active {
        accessibility = accessibility.hint("Active option");
    }
    if option.enabled {
        accessibility = accessibility.focusable();
    } else {
        accessibility = accessibility.disabled();
    }

    let mut node = UiNode::container(name, row_style(options.row_height))
        .with_input(if option.enabled {
            InputBehavior::BUTTON
        } else {
            InputBehavior::NONE
        })
        .with_visual(select_row_visual(index, option, state, options))
        .with_accessibility(accessibility);
    if active {
        if let Some(shader) = options.active_shader.clone() {
            node = node.with_shader(shader);
        }
    }
    node
}

fn first_enabled_select_index(options: &[SelectOption]) -> Option<usize> {
    options.iter().position(|option| option.enabled)
}

fn last_enabled_select_index(options: &[SelectOption]) -> Option<usize> {
    options.iter().rposition(|option| option.enabled)
}

fn next_enabled_select_index(
    options: &[SelectOption],
    current: Option<usize>,
    direction: NavigationDirection,
) -> Option<usize> {
    let len = options.len();
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
        if options[index].enabled {
            return Some(index);
        }
    }
    None
}

fn next_select_typeahead_index(
    options: &[SelectOption],
    current: Option<usize>,
    character: char,
) -> Option<usize> {
    let query = normalized_character(character)?;
    next_matching_index(options.len(), current, NavigationDirection::Next, |index| {
        let option = &options[index];
        option.enabled && normalize(&option.label).starts_with(&query)
    })
}

pub fn filter_select_option_indices(options: &[SelectOption], query: &str) -> Vec<usize> {
    let query = normalize(query);
    if query.trim().is_empty() {
        return (0..options.len()).collect();
    }

    options
        .iter()
        .enumerate()
        .filter_map(|(index, option)| select_option_matches_query(option, &query).then_some(index))
        .collect()
}

pub fn filter_select_options(
    options: &[SelectOption],
    query: &str,
) -> Vec<SelectOptionFilterMatch> {
    filter_select_option_indices(options, query)
        .into_iter()
        .enumerate()
        .map(|(visible_index, option_index)| {
            let option = &options[option_index];
            SelectOptionFilterMatch {
                visible_index,
                option_index,
                id: option.id.clone(),
                enabled: option.enabled,
            }
        })
        .collect()
}

fn select_option_matches_query(option: &SelectOption, query: &str) -> bool {
    let tokens = query.split_whitespace().collect::<Vec<_>>();
    let id = normalize(&option.id);
    let label = normalize(&option.label);
    let accessibility_label = option.accessibility_label.as_deref().map(normalize);

    tokens.iter().all(|token| {
        id.contains(token)
            || label.contains(token)
            || accessibility_label
                .as_deref()
                .is_some_and(|label| label.contains(token))
    })
}

fn first_enabled_select_option_match(
    options: &[SelectOption],
    matches: &[SelectOptionFilterMatch],
) -> Option<usize> {
    matches
        .iter()
        .position(|option_match| options[option_match.option_index].enabled)
}

fn last_enabled_select_option_match(
    options: &[SelectOption],
    matches: &[SelectOptionFilterMatch],
) -> Option<usize> {
    matches
        .iter()
        .rposition(|option_match| options[option_match.option_index].enabled)
}

fn next_enabled_select_option_match(
    options: &[SelectOption],
    matches: &[SelectOptionFilterMatch],
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
        if options[matches[index].option_index].enabled {
            return Some(index);
        }
    }
    None
}

fn active_select_row(
    options: &[SelectOption],
    rows: &[UiNodeId],
    active: Option<usize>,
) -> Option<UiNodeId> {
    active
        .filter(|index| options.get(*index).is_some_and(|option| option.enabled))
        .and_then(|index| rows.get(index).copied())
}

fn menu_list_container_node(
    name: impl Into<String>,
    items: &[MenuItem],
    options: &MenuListOptions,
) -> UiNode {
    let name = name.into();
    let scroll = menu_row_count_for_scroll(items) > options.max_visible_rows;
    let height = visible_menu_height(items, options);
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: length(options.width.max(0.0)),
                    height: length(height.max(0.0)),
                },
                ..Default::default()
            }),
            clip: ClipBehavior::Clip,
            z_index: options.z_index,
            ..Default::default()
        },
    )
    .with_visual(options.menu_visual)
    .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Menu).label(
        menu_accessibility_label(&name, options.accessibility_label.as_ref()),
    ));
    if scroll {
        node = node.with_scroll(ScrollAxes::VERTICAL);
    }
    if let Some(shader) = options.menu_shader.clone() {
        node = node.with_shader(shader);
    }
    if let Some(animation) = options.menu_animation.clone() {
        node = node.with_animation(animation);
    }
    node
}

fn populate_menu_list(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    items: &[MenuItem],
    active: Option<usize>,
    options: &MenuListOptions,
) -> Vec<UiNodeId> {
    let mut rows = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        if item.is_separator() {
            rows.push(separator_row(document, parent, name, index, options));
            continue;
        }

        let text_style = menu_item_text_style(item, options);
        let row = document.add_child(
            parent,
            menu_item_row_node(
                format!("{name}.item.{index}"),
                item,
                active == Some(index),
                options,
            ),
        );
        if let MenuItemKind::Check { checked } = &item.kind {
            label(
                document,
                row,
                format!("{name}.item.{index}.check"),
                if *checked { "[x]" } else { "[ ]" },
                options.shortcut_text_style.clone(),
                leading_label_layout(24.0),
            );
        }
        if let Some(image) = &item.image {
            leading_image(
                document,
                row,
                format!("{name}.item.{index}.image"),
                image.clone(),
                &menu_item_accessibility_label(item),
                options.image_size,
            );
        }
        label(
            document,
            row,
            format!("{name}.item.{index}.label"),
            menu_item_label(item),
            text_style,
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        );
        if let Some(shortcut) = &item.shortcut {
            label(
                document,
                row,
                format!("{name}.item.{index}.shortcut"),
                shortcut,
                options.shortcut_text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            );
        } else if item.children().is_some() {
            label(
                document,
                row,
                format!("{name}.item.{index}.submenu"),
                ">",
                options.shortcut_text_style.clone(),
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
    rows
}

fn active_menu_row(
    items: &[MenuItem],
    rows: &[UiNodeId],
    active: Option<usize>,
) -> Option<UiNodeId> {
    active
        .filter(|index| items.get(*index).is_some_and(MenuItem::is_navigable))
        .and_then(|index| rows.get(index).copied())
}

fn menu_item_row_node(
    name: impl Into<String>,
    item: &MenuItem,
    active: bool,
    options: &MenuListOptions,
) -> UiNode {
    let visual = if item.enabled {
        if active {
            options.active_visual
        } else {
            options.item_visual
        }
    } else {
        options.disabled_visual
    };
    let mut node = UiNode::container(name, row_style(options.row_height))
        .with_input(if item.enabled {
            InputBehavior::BUTTON
        } else {
            InputBehavior::NONE
        })
        .with_visual(visual)
        .with_accessibility(menu_item_accessibility(item, active));
    if active {
        if let Some(shader) = options.active_shader.clone() {
            node = node.with_shader(shader);
        }
    }
    node
}

fn menu_item_text_style(item: &MenuItem, options: &MenuListOptions) -> TextStyle {
    if !item.enabled {
        options.disabled_text_style.clone()
    } else if item.destructive {
        options.destructive_text_style.clone()
    } else {
        options.text_style.clone()
    }
}

fn separator_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    index: usize,
    options: &MenuListOptions,
) -> UiNodeId {
    let row = document.add_child(
        parent,
        UiNode::container(
            format!("{name}.separator.{index}"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    justify_content: Some(JustifyContent::Center),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(options.separator_height.max(0.0)),
                    },
                    flex_shrink: 0.0,
                    padding: TaffyRect {
                        left: length_percentage(8.0),
                        right: length_percentage(8.0),
                        top: length_percentage(0.0),
                        bottom: length_percentage(0.0),
                    },
                    ..Default::default()
                }),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    document.add_child(
        row,
        UiNode::container(
            format!("{name}.separator.{index}.line"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(1.0),
                    },
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(ColorRgba::new(77, 90, 111, 255), None, 0.0)),
    );
    row
}

fn menu_item_label(item: &MenuItem) -> String {
    match &item.kind {
        MenuItemKind::Command | MenuItemKind::Submenu { .. } => item.label.clone(),
        MenuItemKind::Check { .. } | MenuItemKind::Separator => item.label.clone(),
    }
}

fn menu_row_count_for_scroll(items: &[MenuItem]) -> usize {
    items.len()
}

fn visible_menu_height(items: &[MenuItem], options: &MenuListOptions) -> f32 {
    let mut height = 0.0;
    for item in items.iter().take(options.max_visible_rows) {
        height += if item.is_separator() {
            options.separator_height
        } else {
            options.row_height
        };
    }
    height
}

fn next_menu_typeahead_index(
    items: &[MenuItem],
    current: Option<usize>,
    character: char,
) -> Option<usize> {
    let query = normalized_character(character)?;
    next_matching_index(items.len(), current, NavigationDirection::Next, |index| {
        let item = &items[index];
        item.is_navigable() && normalize(&item.label).starts_with(&query)
    })
}

fn next_enabled_menu_bar_index(
    menus: &[MenuBarMenu],
    current: Option<usize>,
    direction: NavigationDirection,
) -> Option<usize> {
    let len = menus.len();
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
        if menus[index].enabled {
            return Some(index);
        }
    }
    None
}

fn visible_row_count(count: usize, max_visible_rows: usize) -> usize {
    if max_visible_rows == 0 {
        0
    } else {
        count.min(max_visible_rows)
    }
}

fn visible_match_range(
    count: usize,
    active: Option<usize>,
    max_visible_rows: usize,
) -> std::ops::Range<usize> {
    let visible_rows = visible_row_count(count, max_visible_rows);
    if visible_rows == 0 {
        return 0..0;
    }

    let active = active.filter(|index| *index < count).unwrap_or(0);
    let half_window = visible_rows / 2;
    let max_start = count.saturating_sub(visible_rows);
    let start = active.saturating_sub(half_window).min(max_start);
    start..(start + visible_rows)
}

fn row_style(height: f32) -> UiNodeStyle {
    UiNodeStyle {
        layout: LayoutStyle::from_taffy_style(Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            size: TaffySize {
                width: Dimension::percent(1.0),
                height: length(height.max(0.0)),
            },
            padding: TaffyRect::length(6.0),
            flex_shrink: 0.0,
            ..Default::default()
        }),
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

fn leading_label_layout(width: f32) -> LayoutStyle {
    LayoutStyle::from_taffy_style(Style {
        size: TaffySize {
            width: length(width.max(0.0)),
            height: Dimension::auto(),
        },
        flex_shrink: 0.0,
        ..Default::default()
    })
}

fn leading_image(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    image: ImageContent,
    accessibility_label: &str,
    image_size: UiSize,
) -> UiNodeId {
    document.add_child(
        parent,
        UiNode::image(
            name,
            image,
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(image_size.width.max(0.0)),
                    height: length(image_size.height.max(0.0)),
                },
                margin: TaffyRect {
                    left: LengthPercentageAuto::length(0.0),
                    right: LengthPercentageAuto::length(6.0),
                    top: LengthPercentageAuto::length(0.0),
                    bottom: LengthPercentageAuto::length(0.0),
                },
                flex_shrink: 0.0,
                ..Default::default()
            }),
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Image).label(accessibility_label),
        ),
    )
}

fn menu_accessibility_label(name: &str, explicit: Option<&String>) -> String {
    explicit.cloned().unwrap_or_else(|| name.to_string())
}

fn option_accessibility_label(option: &SelectOption) -> String {
    option
        .accessibility_label
        .clone()
        .unwrap_or_else(|| option.label.clone())
}

fn menu_item_accessibility_label(item: &MenuItem) -> String {
    item.accessibility_label
        .clone()
        .unwrap_or_else(|| item.label.clone())
}

fn command_item_accessibility_label(item: &CommandPaletteItem) -> String {
    item.accessibility_label
        .clone()
        .unwrap_or_else(|| item.title.clone())
}

fn set_active_descendant(
    document: &mut UiDocument,
    owner: UiNodeId,
    active_descendant: Option<UiNodeId>,
) {
    if let Some(active_descendant) = active_descendant {
        if let Some(accessibility) = document.node_mut(owner).accessibility.as_mut() {
            accessibility.relations.active_descendant = Some(active_descendant);
        }
    }
}

fn command_shortcut_label(
    registry: &CommandRegistry,
    command: &CommandId,
    active_scopes: &[CommandScope],
    formatter: &ShortcutFormatter,
) -> Option<String> {
    CommandTooltipResolver::new(registry)
        .formatter(formatter.clone())
        .shortcut_for(command, active_scopes)
        .map(|shortcut| formatter.format(shortcut))
}

fn menu_item_accessibility(item: &MenuItem, active: bool) -> AccessibilityMeta {
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::MenuItem)
        .label(menu_item_accessibility_label(item))
        .selected(active);
    if let MenuItemKind::Check { checked } = &item.kind {
        accessibility = accessibility
            .value(if *checked { "checked" } else { "unchecked" })
            .checked(*checked);
    }
    if item.children().is_some() {
        accessibility = accessibility.hint("Opens submenu").expanded(active);
    } else if item.destructive {
        accessibility = accessibility.hint("Destructive action");
    } else if let Some(shortcut) = &item.shortcut {
        accessibility = accessibility
            .hint(format!("Shortcut {shortcut}"))
            .shortcut(shortcut.clone());
    } else if active {
        accessibility = accessibility.hint("Active menu item");
    }
    if item.enabled {
        accessibility.focusable()
    } else {
        accessibility.disabled()
    }
}

fn menu_button_accessibility(menu: &MenuBarMenu, active: bool) -> AccessibilityMeta {
    let accessibility = AccessibilityMeta::new(AccessibilityRole::MenuItem)
        .label(menu.label.clone())
        .value(if active { "open" } else { "closed" })
        .expanded(active);
    if menu.enabled {
        accessibility.focusable()
    } else {
        accessibility.disabled()
    }
}

fn button_like(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    layout: LayoutStyle,
    visual: UiVisual,
    text_style: TextStyle,
) -> UiNodeId {
    button_like_with_input(
        document,
        parent,
        name,
        label_text,
        layout,
        visual,
        text_style,
        InputBehavior::BUTTON,
    )
}

#[allow(clippy::too_many_arguments)]
fn button_like_with_input(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    layout: LayoutStyle,
    visual: UiVisual,
    text_style: TextStyle,
    input: InputBehavior,
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(input)
        .with_visual(visual),
    );
    label(
        document,
        root,
        format!("{name}.label"),
        label_text,
        text_style,
        LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: Dimension::auto(),
                height: Dimension::auto(),
            },
            ..Default::default()
        }),
    );
    root
}

fn label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    text_style: TextStyle,
    layout: LayoutStyle,
) -> UiNodeId {
    document.add_child(parent, UiNode::text(name, text, text_style, layout))
}

fn length_percentage(value: f32) -> taffy::prelude::LengthPercentage {
    taffy::prelude::LengthPercentage::length(value)
}

fn normalize(value: &str) -> String {
    value.to_lowercase()
}

fn first_typeahead_character(text: &str) -> Option<char> {
    text.chars()
        .find(|character| is_typeahead_character(*character))
}

fn is_typeahead_character(character: char) -> bool {
    !character.is_control() && !character.is_whitespace()
}

fn normalized_character(character: char) -> Option<String> {
    is_typeahead_character(character).then(|| character.to_lowercase().collect())
}

fn next_matching_index(
    len: usize,
    current: Option<usize>,
    direction: NavigationDirection,
    mut matches: impl FnMut(usize) -> bool,
) -> Option<usize> {
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
        if matches(index) {
            return Some(index);
        }
    }
    None
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

fn first_enabled_palette_match(
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

fn search_count_label(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

fn search_field_clear_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.ctrl || modifiers.meta
}

fn pop_last_char(text: &mut String) -> bool {
    let Some((index, _)) = text.char_indices().next_back() else {
        return false;
    };
    text.truncate(index);
    true
}

#[cfg(test)]
mod tests {
    use taffy::prelude::{Size as TaffySize, Style};

    use super::*;
    use crate::{
        root_style, AccessibilityMeta, AccessibilityRole, AnimatedValues, AnimationMachine,
        AnimationState, AnimationTransition, AnimationTrigger, ApproxTextMeasurer, Command,
        CommandId, CommandMeta, CommandRegistry, CommandScope, KeyModifiers, ShaderEffect,
        Shortcut, ShortcutFormatter, UiContent,
    };

    fn test_animation() -> AnimationMachine {
        AnimationMachine::new(
            vec![
                AnimationState::new(
                    "hidden",
                    AnimatedValues::new(0.0, UiPoint::new(0.0, 0.0), 0.98),
                ),
                AnimationState::new("shown", AnimatedValues::default()),
            ],
            vec![AnimationTransition::new(
                "hidden",
                "shown",
                AnimationTrigger::Custom("show".to_string()),
                0.12,
            )],
            "hidden",
        )
        .expect("animation")
    }

    #[test]
    fn popup_placement_flips_and_clamps_to_viewport() {
        let layout = place_popup(
            UiRect::new(260.0, 170.0, 32.0, 24.0),
            UiSize::new(140.0, 80.0),
            UiRect::new(0.0, 0.0, 300.0, 220.0),
            PopupPlacement::new(PopupSide::Bottom, PopupAlign::End)
                .with_offset(6.0)
                .with_viewport_margin(8.0),
        );

        assert_eq!(layout.side, PopupSide::Top);
        assert!(layout.flipped);
        assert!(layout.rect.x >= 8.0, "{layout:?}");
        assert!(layout.rect.right() <= 292.0, "{layout:?}");
        assert!(layout.rect.y >= 8.0, "{layout:?}");
        assert!(layout.rect.bottom() <= 212.0, "{layout:?}");
    }

    #[test]
    fn popup_panel_uses_absolute_layout_and_optional_scroll() {
        let mut document = UiDocument::new(root_style(300.0, 200.0));
        let root = document.root;
        let popup = popup_panel(
            &mut document,
            root,
            "popup",
            UiRect::new(16.0, 20.0, 120.0, 80.0),
            PopupOptions {
                scroll_axes: ScrollAxes::VERTICAL,
                ..Default::default()
            },
        );

        let node = document.node(popup);
        assert!(node.style.layout.is_absolute());
        assert_eq!(node.style.z_index, 100);
        assert!(node.scroll.is_some());
    }

    #[test]
    fn popup_panel_exposes_accessibility_shader_and_animation_metadata() {
        let mut document = UiDocument::new(root_style(300.0, 200.0));
        let root = document.root;
        let popup = popup_panel(
            &mut document,
            root,
            "popup",
            UiRect::new(16.0, 20.0, 120.0, 80.0),
            PopupOptions {
                accessibility: Some(
                    AccessibilityMeta::new(AccessibilityRole::Dialog)
                        .label("Inspector")
                        .focusable(),
                ),
                shader: Some(ShaderEffect::new("ui.popup.shadow").uniform("elevation", 8.0)),
                animation: Some(test_animation()),
                ..Default::default()
            },
        );

        let node = document.node(popup);
        let accessibility = node.accessibility.as_ref().expect("accessibility");
        assert_eq!(accessibility.role, AccessibilityRole::Dialog);
        assert_eq!(accessibility.label.as_deref(), Some("Inspector"));
        assert!(accessibility.focusable);
        assert_eq!(node.shader.as_ref().unwrap().key, "ui.popup.shadow");
        assert_eq!(
            node.animation.as_ref().unwrap().current_state_name(),
            "hidden"
        );
    }

    #[test]
    fn select_menu_keyboard_navigation_skips_disabled_options() {
        let options = vec![
            SelectOption::new("a", "Alpha").disabled(),
            SelectOption::new("b", "Beta"),
            SelectOption::new("c", "Gamma"),
        ];
        let mut state = SelectMenuState::new();

        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::ArrowDown,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(outcome.opened);
        assert_eq!(state.active, Some(1));

        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(
            outcome.selected,
            Some(SelectSelection {
                index: 1,
                id: "b".to_string(),
            })
        );
        assert_eq!(state.selected_id(&options), Some("b"));
        assert!(!state.open);
    }

    #[test]
    fn select_menu_typeahead_selects_closed_and_moves_open_active_option() {
        let options = vec![
            SelectOption::new("alpha", "Alpha"),
            SelectOption::new("beta", "Beta").disabled(),
            SelectOption::new("gamma", "Gamma"),
        ];
        let mut state = SelectMenuState::with_selected(0);

        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::Character('g'),
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(
            outcome.selected,
            Some(SelectSelection {
                index: 2,
                id: "gamma".to_string(),
            })
        );
        assert_eq!(state.selected, Some(2));

        state.open(&options);
        let outcome = state.handle_event(&options, &UiInputEvent::TextInput("a".to_string()));
        assert_eq!(outcome.active, Some(0));
        assert_eq!(state.active, Some(0));
    }

    #[test]
    fn search_field_tracks_clear_button_debounce_status_and_keyboard_clear() {
        let mut field = SearchFieldState::new();
        assert!(field.clear_button().is_none());

        let outcome = field.handle_event(&UiInputEvent::TextInput("alp".to_string()), 100);
        assert!(outcome.query_changed);
        assert!(outcome.filter_pending);
        assert_eq!(field.query, "alp");
        assert!(field.is_filter_pending());

        let clear_button = field.clear_button().expect("clear button");
        assert_eq!(clear_button.id, "clear-search");
        assert_eq!(clear_button.label, "Clear");
        assert_eq!(clear_button.shortcut.as_deref(), Some("Escape"));
        let clear_accessibility = clear_button.accessibility();
        assert_eq!(clear_accessibility.role, AccessibilityRole::Button);
        assert!(clear_accessibility.focusable);

        let input_accessibility = field.input_accessibility("Search commands");
        assert_eq!(input_accessibility.role, AccessibilityRole::SearchBox);
        assert_eq!(input_accessibility.value.as_deref(), Some("alp"));
        assert_eq!(input_accessibility.actions.len(), 1);
        assert_eq!(input_accessibility.actions[0].id, "clear-search");

        assert!(field
            .filter_request(149, SearchFilterTiming::debounced(50))
            .is_none());
        let request = field
            .filter_request(150, SearchFilterTiming::debounced(50))
            .expect("debounced filter");
        assert_eq!(request.query, "alp");
        assert_eq!(request.revision, field.revision);
        assert_eq!(request.elapsed_ms, 50);

        let request = field
            .take_filter_request(150, SearchFilterTiming::debounced(50))
            .expect("taken filter");
        assert_eq!(request.query, "alp");
        assert!(!field.is_filter_pending());

        let status = field.status(2, 4, "option", "options");
        assert_eq!(status.text, "2 options match \"alp\"");
        let status_accessibility = status.accessibility("Search results");
        assert_eq!(status_accessibility.role, AccessibilityRole::Status);
        assert_eq!(
            status_accessibility.live_region,
            AccessibilityLiveRegion::Polite
        );

        let outcome = field.handle_event(
            &UiInputEvent::Key {
                key: KeyCode::Escape,
                modifiers: KeyModifiers::NONE,
            },
            200,
        );
        assert!(outcome.cleared);
        assert_eq!(field.query, "");
        assert!(field.is_filter_pending());

        let outcome = field.handle_event(
            &UiInputEvent::Key {
                key: KeyCode::Escape,
                modifiers: KeyModifiers::NONE,
            },
            250,
        );
        assert!(outcome.close_requested);
    }

    #[test]
    fn select_option_filter_accepts_search_field_and_reports_accessible_status() {
        let options = vec![
            SelectOption::new("alpha", "Alpha"),
            SelectOption::new("alpine", "Alpine"),
            SelectOption::new("beta", "Beta"),
        ];
        let mut field = SearchFieldState::new();
        field.set_query("alp", 10);
        let mut state = SelectOptionFilterState::new();

        let outcome = state.apply_search_field(&field, &options);
        assert!(outcome.query_changed);
        assert_eq!(state.query, "alp");
        assert_eq!(state.filtered_indices(&options), vec![0, 1]);
        assert_eq!(state.active_match, Some(0));
        assert_eq!(
            state.search_status(&options).text,
            "2 options match \"alp\""
        );
        assert_eq!(
            state.search_status_accessibility(&options).live_region,
            AccessibilityLiveRegion::Polite
        );
        assert_eq!(state.search_field().query, "alp");

        let outcome = state.clear_query(&options);
        assert!(outcome.query_changed);
        assert_eq!(state.query, "");
        assert_eq!(state.active_match, Some(0));
        assert_eq!(state.search_status(&options).text, "3 options available");
    }

    #[test]
    fn select_option_filter_preserves_option_indices_and_searches_option_metadata() {
        let options = vec![
            SelectOption::new("lofi", "Low fidelity").accessibility_label("Preview quality"),
            SelectOption::new("studio", "Studio").disabled(),
            SelectOption::new("high", "High fidelity"),
        ];
        let mut state = SelectOptionFilterState::new();

        state.set_query("fidelity", &options);
        assert_eq!(state.filtered_indices(&options), vec![0, 2]);
        assert_eq!(state.active_match, Some(0));
        assert_eq!(state.active_option_index(&options), Some(0));
        assert_eq!(state.active_id(&options), Some("lofi"));
        assert_eq!(
            state.active_accessibility_label(&options).as_deref(),
            Some("Preview quality")
        );
        assert_eq!(state.accessibility_value(&options), "2 filtered options");

        state.set_query("studio", &options);
        assert_eq!(state.filtered_indices(&options), vec![1]);
        assert_eq!(state.active_match, None);
        assert_eq!(state.select_active(&options), None);
    }

    #[test]
    fn select_option_filter_moves_through_filtered_enabled_options_and_selects_existing_shape() {
        let options = vec![
            SelectOption::new("alpha", "Alpha"),
            SelectOption::new("alpine", "Alpine").disabled(),
            SelectOption::new("atlas", "Atlas"),
            SelectOption::new("beta", "Beta"),
        ];
        let mut state = SelectOptionFilterState::new().with_query("a");
        state.set_query("a", &options);

        assert_eq!(state.filtered_indices(&options), vec![0, 1, 2, 3]);
        assert_eq!(state.active_match, Some(0));

        let active = state.move_active(&options, NavigationDirection::Next);
        assert_eq!(active, Some(2));
        assert_eq!(
            state.select_active(&options),
            Some(SelectSelection {
                index: 2,
                id: "atlas".to_string(),
            })
        );

        let active = state.move_active(&options, NavigationDirection::Previous);
        assert_eq!(active, Some(0));
    }

    #[test]
    fn select_option_filter_handles_text_input_backspace_empty_and_escape() {
        let options = vec![
            SelectOption::new("alpha", "Alpha"),
            SelectOption::new("beta", "Beta"),
        ];
        let mut state = SelectOptionFilterState::new().with_empty_label("No matching options");

        let outcome = state.handle_event(&options, &UiInputEvent::TextInput("zz".to_string()));
        assert!(outcome.query_changed);
        assert_eq!(state.query, "zz");
        assert_eq!(
            state.empty_state(&options),
            Some(SelectOptionFilterEmptyState {
                query: "zz".to_string(),
                label: "No matching options".to_string(),
            })
        );
        assert_eq!(state.accessibility_value(&options), "No matching options");

        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(outcome.query_changed);
        assert_eq!(state.query, "z");

        state.set_query("beta", &options);
        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(
            outcome.selected,
            Some(SelectSelection {
                index: 1,
                id: "beta".to_string(),
            })
        );

        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::Escape,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(outcome.closed);
    }

    #[test]
    fn searchable_select_state_composes_open_filter_select_and_dismiss_outcomes() {
        let options = vec![
            SelectOption::new("alpha", "Alpha"),
            SelectOption::new("alpine", "Alpine").disabled(),
            SelectOption::new("atlas", "Atlas"),
            SelectOption::new("beta", "Beta"),
        ];
        let mut state = SearchableSelectState::new();

        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::ArrowDown,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(outcome.opened);
        assert_eq!(state.filter.active_match, Some(0));

        let outcome = state.handle_event(&options, &UiInputEvent::TextInput("a".to_string()));
        assert!(outcome.query_changed);
        assert_eq!(state.filter.query, "a");
        assert_eq!(state.filter.active_match, Some(0));

        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::ArrowDown,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(outcome.active_match, Some(2));

        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(
            outcome.selected,
            Some(SelectSelection {
                index: 2,
                id: "atlas".to_string(),
            })
        );
        assert_eq!(
            outcome.close_reason,
            Some(SearchableSelectCloseReason::Selection)
        );
        assert_eq!(state.selected, Some(2));
        assert!(!state.open);

        state.open(&options);
        state.filter.set_query("alp", &options);
        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::Escape,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(outcome.query_changed);
        assert!(state.open);
        assert_eq!(state.filter.query, "");

        let outcome = state.handle_event(
            &options,
            &UiInputEvent::Key {
                key: KeyCode::Escape,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(outcome.closed);
        assert_eq!(
            outcome.close_reason,
            Some(SearchableSelectCloseReason::Escape)
        );
        assert!(!state.open);

        state.open(&options);
        let outcome = state.handle_outside_dismiss();
        assert!(outcome.closed);
        assert_eq!(
            outcome.close_reason,
            Some(SearchableSelectCloseReason::Outside)
        );
        assert!(!state.open);
    }

    #[test]
    fn searchable_select_contract_exports_bounded_rows_and_accessibility_metadata() {
        let options = (0..6)
            .map(|index| SelectOption::new(format!("id-{index}"), format!("Item {index}")))
            .collect::<Vec<_>>();
        let mut state = SearchableSelectState::with_selected(4);
        state.open(&options);

        let contract = searchable_select_contract(
            "fabricad.filter",
            &options,
            &state,
            SearchableSelectSpec::new()
                .max_visible_rows(3)
                .placeholder("Choose")
                .accessibility_label("Fabricad filter")
                .search_label("Filter options")
                .list_label("Filter results"),
        );

        assert!(contract.open);
        assert_eq!(contract.selected_id.as_deref(), Some("id-4"));
        assert_eq!(contract.match_count, 6);
        assert_eq!(contract.visible_range, 3..6);
        assert_eq!(contract.rows.len(), 3);
        assert_eq!(
            contract.active_descendant_id.as_deref(),
            Some("fabricad.filter.option.4")
        );
        assert_eq!(
            contract.trigger_accessibility.role,
            AccessibilityRole::ComboBox
        );
        assert_eq!(
            contract.trigger_accessibility.label.as_deref(),
            Some("Fabricad filter")
        );
        assert_eq!(
            contract.trigger_accessibility.value.as_deref(),
            Some("Item 4")
        );
        assert_eq!(contract.trigger_accessibility.expanded, Some(true));
        assert_eq!(
            contract.search_accessibility.role,
            AccessibilityRole::SearchBox
        );
        assert_eq!(
            contract.search_accessibility.label.as_deref(),
            Some("Filter options")
        );
        assert_eq!(contract.listbox_accessibility.role, AccessibilityRole::List);
        assert_eq!(
            contract.listbox_accessibility.value.as_deref(),
            Some("6 options available")
        );
        assert_eq!(contract.rows[1].option_index, 4);
        assert!(contract.rows[1].active);
        assert!(contract.rows[1].selected);
        assert_eq!(
            contract.rows[1].accessibility.hint.as_deref(),
            Some("Active option")
        );
    }

    #[test]
    fn select_menu_builds_scrollable_renderer_neutral_rows() {
        let mut document = UiDocument::new(root_style(320.0, 240.0));
        let root = document.root;
        let options = (0..6)
            .map(|index| SelectOption::new(format!("id-{index}"), format!("Item {index}")))
            .collect::<Vec<_>>();
        let state = SelectMenuState {
            open: true,
            selected: Some(1),
            active: Some(2),
        };
        let nodes = select_menu(
            &mut document,
            root,
            "select",
            &options,
            &state,
            SelectMenuOptions {
                max_visible_rows: 3,
                row_height: 20.0,
                ..Default::default()
            },
        );

        assert_eq!(nodes.rows.len(), 6);
        assert_eq!(
            document.node(nodes.root).scroll.unwrap().axes,
            ScrollAxes::VERTICAL
        );
        assert!(document.node(nodes.rows[2]).input.focusable);
    }

    #[test]
    fn select_menu_exports_accessible_rows_images_and_active_shader() {
        let mut document = UiDocument::new(root_style(320.0, 240.0));
        let root = document.root;
        let options = vec![
            SelectOption::new("compact", "Compact").image_key("icons.compact"),
            SelectOption::new("comfortable", "Comfortable")
                .accessibility_label("Comfortable density"),
        ];
        let state = SelectMenuState {
            open: true,
            selected: Some(0),
            active: Some(1),
        };
        let nodes = select_menu(
            &mut document,
            root,
            "density",
            &options,
            &state,
            SelectMenuOptions {
                accessibility_label: Some("Density choices".to_string()),
                active_shader: Some(ShaderEffect::new("ui.option.active")),
                ..Default::default()
            },
        );

        let root_accessibility = document.node(nodes.root).accessibility.as_ref().unwrap();
        assert_eq!(root_accessibility.role, AccessibilityRole::List);
        assert_eq!(root_accessibility.label.as_deref(), Some("Density choices"));
        assert_eq!(
            root_accessibility.relations.active_descendant,
            Some(nodes.rows[1])
        );

        let first_row = document.node(nodes.rows[0]);
        let first_accessibility = first_row.accessibility.as_ref().unwrap();
        assert_eq!(first_accessibility.value.as_deref(), Some("selected"));
        assert_eq!(first_accessibility.selected, Some(true));
        assert!(first_row.children.iter().any(|child| matches!(
            &document.node(*child).content,
            UiContent::Image(image) if image.key == "icons.compact"
        )));

        let active_accessibility = document.node(nodes.rows[1]).accessibility.as_ref().unwrap();
        assert_eq!(active_accessibility.selected, Some(false));
        assert_eq!(
            active_accessibility.label.as_deref(),
            Some("Comfortable density")
        );
        assert_eq!(
            document.node(nodes.rows[1]).shader.as_ref().unwrap().key,
            "ui.option.active"
        );
    }

    #[test]
    fn nested_menu_selection_returns_index_path_and_id() {
        let items = vec![MenuItem::submenu(
            "file",
            "File",
            vec![
                MenuItem::command("new", "New"),
                MenuItem::separator(),
                MenuItem::command("open", "Open").shortcut("Ctrl+O"),
            ],
        )];

        assert_eq!(
            menu_selection_at_path(&items, &[0, 2]),
            Some(MenuSelection {
                id: Some("open".to_string()),
                index_path: vec![0, 2],
            })
        );
        assert!(menu_selection_at_path(&items, &[0, 1]).is_none());
        assert_eq!(
            next_navigable_index(&items, None, NavigationDirection::Next),
            Some(0)
        );
    }

    #[test]
    fn nested_menu_navigation_opens_submenus_and_selects_index_path() {
        let items = vec![MenuItem::submenu(
            "file",
            "File",
            vec![
                MenuItem::command("new", "New"),
                MenuItem::separator(),
                MenuItem::submenu(
                    "recent",
                    "Recent",
                    vec![
                        MenuItem::command("recent-a", "Recent A").disabled(),
                        MenuItem::command("recent-b", "Recent B"),
                    ],
                ),
            ],
        )];
        let mut state = MenuNavigationState::new();

        assert_eq!(state.open_root(&items), Some(vec![0]));

        let outcome = state.handle_event(
            &items,
            &UiInputEvent::Key {
                key: KeyCode::ArrowRight,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(outcome.opened_submenu);
        assert_eq!(outcome.active_path, Some(vec![0, 0]));

        let outcome = state.handle_event(
            &items,
            &UiInputEvent::Key {
                key: KeyCode::ArrowDown,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(outcome.active_path, Some(vec![0, 2]));

        let outcome = state.handle_event(
            &items,
            &UiInputEvent::Key {
                key: KeyCode::ArrowRight,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(outcome.opened_submenu);
        assert_eq!(outcome.active_path, Some(vec![0, 2, 1]));

        let outcome = state.handle_event(
            &items,
            &UiInputEvent::Key {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(
            outcome.selected,
            Some(MenuSelection {
                id: Some("recent-b".to_string()),
                index_path: vec![0, 2, 1],
            })
        );
        assert!(outcome.closed);
        assert!(state.active_path.is_empty());
    }

    #[test]
    fn nested_menu_navigation_typeahead_scopes_to_current_menu_level() {
        let items = vec![
            MenuItem::submenu(
                "file",
                "File",
                vec![
                    MenuItem::command("new", "New"),
                    MenuItem::command("open", "Open"),
                ],
            ),
            MenuItem::command("view", "View"),
        ];
        let mut state = MenuNavigationState::new();
        assert_eq!(state.open_root(&items), Some(vec![0]));

        state.handle_event(
            &items,
            &UiInputEvent::Key {
                key: KeyCode::ArrowRight,
                modifiers: KeyModifiers::NONE,
            },
        );
        let outcome = state.handle_event(&items, &UiInputEvent::TextInput("o".to_string()));
        assert_eq!(outcome.active_path, Some(vec![0, 1]));

        let outcome = state.handle_event(
            &items,
            &UiInputEvent::Key {
                key: KeyCode::ArrowLeft,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(outcome.closed_submenu);
        assert_eq!(outcome.active_path, Some(vec![0]));

        let outcome = state.handle_event(&items, &UiInputEvent::TextInput("v".to_string()));
        assert_eq!(outcome.active_path, Some(vec![1]));
    }

    #[test]
    fn menu_list_exports_accessible_rich_rows_and_active_shader() {
        let mut document = UiDocument::new(root_style(360.0, 240.0));
        let root = document.root;
        let items = vec![
            MenuItem::check("snap", "Snap to Grid", true).image_key("icons.grid"),
            MenuItem::command("delete", "Delete")
                .shortcut("Del")
                .destructive()
                .accessibility_label("Delete selection"),
            MenuItem::submenu(
                "arrange",
                "Arrange",
                vec![MenuItem::command("front", "Bring to Front")],
            ),
        ];

        let nodes = menu_list(
            &mut document,
            root,
            "context",
            &items,
            Some(1),
            MenuListOptions {
                accessibility_label: Some("Context actions".to_string()),
                active_shader: Some(ShaderEffect::new("ui.menu.active")),
                ..Default::default()
            },
        );

        let root_accessibility = document.node(nodes.root).accessibility.as_ref().unwrap();
        assert_eq!(root_accessibility.role, AccessibilityRole::Menu);
        assert_eq!(root_accessibility.label.as_deref(), Some("Context actions"));
        assert_eq!(
            root_accessibility.relations.active_descendant,
            Some(nodes.rows[1])
        );

        let check_accessibility = document.node(nodes.rows[0]).accessibility.as_ref().unwrap();
        assert_eq!(check_accessibility.role, AccessibilityRole::MenuItem);
        assert_eq!(check_accessibility.value.as_deref(), Some("checked"));
        assert!(document
            .node(nodes.rows[0])
            .children
            .iter()
            .any(|child| matches!(
                &document.node(*child).content,
                UiContent::Image(image) if image.key == "icons.grid"
            )));

        let delete_accessibility = document.node(nodes.rows[1]).accessibility.as_ref().unwrap();
        assert_eq!(
            delete_accessibility.label.as_deref(),
            Some("Delete selection")
        );
        assert_eq!(
            delete_accessibility.hint.as_deref(),
            Some("Destructive action")
        );
        assert_eq!(
            document.node(nodes.rows[1]).shader.as_ref().unwrap().key,
            "ui.menu.active"
        );
        assert!(document
            .node(nodes.rows[1])
            .children
            .iter()
            .any(|child| matches!(
                &document.node(*child).content,
                UiContent::Text(text) if text.text == "Del"
            )));

        let submenu_accessibility = document.node(nodes.rows[2]).accessibility.as_ref().unwrap();
        assert_eq!(submenu_accessibility.hint.as_deref(), Some("Opens submenu"));
    }

    #[test]
    fn context_menu_keyboard_outcome_selects_ids_not_commands() {
        let items = vec![
            MenuItem::separator(),
            MenuItem::command("copy", "Copy"),
            MenuItem::command("paste", "Paste").disabled(),
        ];
        let mut state = ContextMenuState::open_at(UiPoint::new(20.0, 30.0));

        let outcome = state.handle_event(
            &items,
            &UiInputEvent::Key {
                key: KeyCode::ArrowDown,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(outcome.active, Some(1));

        let outcome = state.handle_event(
            &items,
            &UiInputEvent::Key {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(
            outcome.selected,
            Some(MenuSelection {
                id: Some("copy".to_string()),
                index_path: vec![1],
            })
        );
        assert!(outcome.closed);
    }

    #[test]
    fn menu_command_helpers_return_typed_command_ids_and_shortcuts() {
        let mut registry = CommandRegistry::new();
        registry
            .register(Command::new(
                CommandMeta::new("file.save", "Save Project")
                    .description("Save the current project")
                    .category("File"),
            ))
            .unwrap();
        registry
            .register(
                Command::new(CommandMeta::new("file.export", "Export Audio").category("File"))
                    .disabled("No mixdown target"),
            )
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('s'), "file.save")
            .unwrap();

        let formatter = ShortcutFormatter::default();
        let save = menu_item_from_command(
            &registry,
            "file.save",
            &[CommandScope::Workspace],
            &formatter,
        )
        .unwrap();
        assert_eq!(save.id.as_deref(), Some("file.save"));
        assert_eq!(save.shortcut.as_deref(), Some("Ctrl+S"));
        assert!(save.enabled);

        let export = menu_item_from_command(
            &registry,
            "file.export",
            &[CommandScope::Workspace],
            &formatter,
        )
        .unwrap();
        assert_eq!(export.id.as_deref(), Some("file.export"));
        assert!(!export.enabled);

        let items = vec![save, export];
        let selection = menu_command_selection_at_path(&items, &[0]).unwrap();
        assert_eq!(selection.command, CommandId::from("file.save"));
        assert_eq!(selection.index_path, vec![0]);
        assert!(menu_command_selection_at_path(&items, &[1]).is_none());

        let outcome = MenuOutcome {
            selected: Some(MenuSelection {
                id: Some("file.save".to_string()),
                index_path: vec![0],
            }),
            ..Default::default()
        };
        assert_eq!(
            outcome.selected_command().unwrap().command,
            CommandId::from("file.save")
        );
    }

    #[test]
    fn context_menu_typeahead_wraps_and_skips_disabled_items() {
        let items = vec![
            MenuItem::command("copy", "Copy"),
            MenuItem::command("paste", "Paste").disabled(),
            MenuItem::command("prefs", "Preferences"),
            MenuItem::command("save", "Save"),
        ];
        let mut state = ContextMenuState::open_at(UiPoint::new(20.0, 30.0));

        let outcome = state.handle_event(
            &items,
            &UiInputEvent::Key {
                key: KeyCode::Character('p'),
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(outcome.active, Some(2));

        let outcome = state.handle_event(&items, &UiInputEvent::TextInput("c".to_string()));
        assert_eq!(outcome.active, Some(0));
        assert_eq!(state.active, Some(0));
    }

    #[test]
    fn menu_bar_state_skips_disabled_menus_and_selects_active_item() {
        let menus = vec![
            MenuBarMenu::new("file", "File", vec![MenuItem::command("new", "New")]),
            MenuBarMenu::new("edit", "Edit", vec![MenuItem::command("undo", "Undo")]).disabled(),
            MenuBarMenu::new("view", "View", vec![MenuItem::check("grid", "Grid", true)]),
        ];
        let mut state = MenuBarState::default();

        assert!(state.open(&menus, 0));
        assert_eq!(state.move_menu(&menus, NavigationDirection::Next), Some(2));
        assert_eq!(
            state.select_active(&menus),
            Some(MenuSelection {
                id: Some("grid".to_string()),
                index_path: vec![2, 0],
            })
        );
    }

    #[test]
    fn menu_bar_exports_menubar_and_menuitem_accessibility() {
        let mut document = UiDocument::new(root_style(480.0, 80.0));
        let root = document.root;
        let menus = vec![
            MenuBarMenu::new("file", "File", vec![MenuItem::command("new", "New")]),
            MenuBarMenu::new("edit", "Edit", vec![MenuItem::command("undo", "Undo")]).disabled(),
        ];
        let state = MenuBarState {
            open_menu: Some(0),
            active_item: Some(0),
        };

        let nodes = menu_bar(
            &mut document,
            root,
            "main-menu",
            &menus,
            &state,
            None,
            MenuBarOptions::default(),
        );

        let root_accessibility = document.node(nodes.root).accessibility.as_ref().unwrap();
        assert_eq!(root_accessibility.role, AccessibilityRole::MenuBar);
        assert_eq!(root_accessibility.label.as_deref(), Some("main-menu"));
        assert_eq!(
            root_accessibility.relations.active_descendant,
            Some(nodes.buttons[0])
        );

        let file_accessibility = document
            .node(nodes.buttons[0])
            .accessibility
            .as_ref()
            .unwrap();
        assert_eq!(file_accessibility.role, AccessibilityRole::MenuItem);
        assert_eq!(file_accessibility.value.as_deref(), Some("open"));
        assert!(file_accessibility.enabled);
        assert!(file_accessibility.focusable);

        let edit_accessibility = document
            .node(nodes.buttons[1])
            .accessibility
            .as_ref()
            .unwrap();
        assert_eq!(edit_accessibility.value.as_deref(), Some("closed"));
        assert!(!edit_accessibility.enabled);
    }

    #[test]
    fn command_palette_filter_ranks_title_matches_and_selects_ids() {
        let items = vec![
            CommandPaletteItem::new("open", "Open File").keyword("recent"),
            CommandPaletteItem::new("save", "Save Project").shortcut("Ctrl+S"),
            CommandPaletteItem::new("export", "Export Audio").disabled(),
        ];
        let matches = filter_command_palette(&items, "save", 10);
        assert_eq!(matches[0].id, "save");

        let mut state = CommandPaletteState::new().with_query("project");
        state.active_match = first_enabled_palette_match(&items, &state.matches(&items));
        assert_eq!(
            state.select_active(&items),
            Some(CommandPaletteSelection {
                index: 1,
                id: "save".to_string(),
            })
        );
    }

    #[test]
    fn command_palette_helpers_build_from_registry_and_select_command_ids() {
        let mut registry = CommandRegistry::new();
        registry
            .register(Command::new(
                CommandMeta::new("file.save", "Save Project")
                    .description("Save the current project")
                    .category("File"),
            ))
            .unwrap();
        registry
            .register(
                Command::new(CommandMeta::new("edit.quantize", "Quantize Clip").category("Edit"))
                    .disabled("No clip selected"),
            )
            .unwrap();
        registry
            .bind_shortcut(CommandScope::Global, Shortcut::ctrl('s'), "file.save")
            .unwrap();

        let formatter = ShortcutFormatter::default();
        let items = command_palette_items_from_registry(&registry, &[], &formatter);

        assert_eq!(
            items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["edit.quantize", "file.save"]
        );
        let save = items.iter().find(|item| item.id == "file.save").unwrap();
        assert_eq!(save.shortcut.as_deref(), Some("Ctrl+S"));
        assert_eq!(save.subtitle.as_deref(), Some("Save the current project"));

        let quantize = items
            .iter()
            .find(|item| item.id == "edit.quantize")
            .unwrap();
        assert!(!quantize.enabled);
        assert!(filter_command_palette(&items, "edit", 10)
            .iter()
            .any(|palette_match| palette_match.id == "edit.quantize"));

        let selection = CommandPaletteSelection {
            index: 1,
            id: "file.save".to_string(),
        };
        assert_eq!(selection.command_id(), CommandId::from("file.save"));

        let outcome = CommandPaletteOutcome {
            selected: Some(selection),
            ..Default::default()
        };
        assert_eq!(
            outcome.selected_command().unwrap().command,
            CommandId::from("file.save")
        );
    }

    #[test]
    fn command_palette_state_accepts_search_field_clear_and_status() {
        let items = vec![
            CommandPaletteItem::new("open", "Open File"),
            CommandPaletteItem::new("save", "Save Project"),
            CommandPaletteItem::new("close", "Close Project"),
        ];
        let mut field = SearchFieldState::new();
        field.set_query("save", 25);
        let mut state = CommandPaletteState::new();

        let outcome = state.apply_search_field(&field, &items);
        assert!(outcome.query_changed);
        assert_eq!(state.query, "save");
        assert_eq!(state.matches(&items).len(), 1);
        assert_eq!(state.active_match, Some(0));
        assert_eq!(
            state.search_status(&items).text,
            "1 command matches \"save\""
        );
        assert_eq!(
            state.search_status_accessibility(&items).live_region,
            AccessibilityLiveRegion::Polite
        );

        let outcome = state.clear_query(&items);
        assert!(outcome.query_changed);
        assert_eq!(state.query, "");
        assert_eq!(state.active_match, Some(0));
        assert_eq!(state.visible_count(&items), 3);
        assert_eq!(state.search_status(&items).text, "3 commands available");
    }

    #[test]
    fn command_palette_builder_creates_input_and_result_rows() {
        let mut document = UiDocument::new(root_style(600.0, 400.0));
        let root = document.root;
        let items = vec![
            CommandPaletteItem::new("open", "Open File")
                .subtitle("Recent documents")
                .image_key("icons.open")
                .accessibility_label("Open a file"),
            CommandPaletteItem::new("save", "Save Project"),
        ];
        let mut state = CommandPaletteState::new().with_query("o");
        state.active_match = Some(0);

        let nodes = command_palette(
            &mut document,
            root,
            "palette",
            &items,
            &state,
            None,
            CommandPaletteOptions {
                accessibility_label: Some("Command palette".to_string()),
                panel_shader: Some(ShaderEffect::new("ui.palette.panel")),
                active_row_shader: Some(ShaderEffect::new("ui.palette.active")),
                ..Default::default()
            },
        );
        document
            .compute_layout(UiSize::new(600.0, 400.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(nodes.rows.len(), 2);
        assert!(document.node(nodes.input).input.focusable);
        assert!(document.node(nodes.root).layout.rect.width > 0.0);
        assert_eq!(
            document
                .node(nodes.root)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Command palette")
        );
        assert_eq!(
            document.node(nodes.root).shader.as_ref().unwrap().key,
            "ui.palette.panel"
        );
        assert_eq!(
            document
                .node(nodes.input)
                .accessibility
                .as_ref()
                .unwrap()
                .role,
            AccessibilityRole::SearchBox
        );
        assert_eq!(
            document
                .node(nodes.input)
                .accessibility
                .as_ref()
                .unwrap()
                .actions[0]
                .id,
            "clear-search"
        );
        assert_eq!(
            document
                .node(nodes.input)
                .accessibility
                .as_ref()
                .unwrap()
                .relations
                .active_descendant,
            Some(nodes.rows[0])
        );
        let result_list = document.node(nodes.root).children[1];
        assert_eq!(
            document
                .node(result_list)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("2 commands match \"o\"")
        );
        assert_eq!(
            document
                .node(result_list)
                .accessibility
                .as_ref()
                .unwrap()
                .relations
                .active_descendant,
            Some(nodes.rows[0])
        );

        let active_row = document.node(nodes.rows[0]);
        let active_accessibility = active_row.accessibility.as_ref().unwrap();
        assert_eq!(active_accessibility.role, AccessibilityRole::ListItem);
        assert_eq!(active_accessibility.label.as_deref(), Some("Open a file"));
        assert_eq!(active_accessibility.value.as_deref(), Some("open"));
        assert_eq!(active_row.shader.as_ref().unwrap().key, "ui.palette.active");
        assert!(active_row.children.iter().any(|child| matches!(
            &document.node(*child).content,
            UiContent::Image(image) if image.key == "icons.open"
        )));
    }

    #[test]
    fn dropdown_select_can_build_trigger_without_popup_anchor() {
        let mut document = UiDocument::new(root_style(320.0, 160.0));
        let root = document.root;
        let options = vec![
            SelectOption::new("low", "Low"),
            SelectOption::new("high", "High"),
        ];
        let state = SelectMenuState::with_selected(1);

        let nodes = dropdown_select(
            &mut document,
            root,
            "quality",
            &options,
            &state,
            None,
            DropdownSelectOptions {
                trigger_layout: LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(120.0),
                        height: length(30.0),
                    },
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        assert!(document.node(nodes.trigger).input.focusable);
        let accessibility = document.node(nodes.trigger).accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::ComboBox);
        assert_eq!(accessibility.value.as_deref(), Some("High"));
        assert!(nodes.popup.is_none());

        let mut open_document = UiDocument::new(root_style(320.0, 160.0));
        let open_root = open_document.root;
        let open_state = SelectMenuState {
            open: true,
            selected: Some(1),
            active: Some(0),
        };
        let open_nodes = dropdown_select(
            &mut open_document,
            open_root,
            "quality-open",
            &options,
            &open_state,
            Some(AnchoredPopup::new(
                UiRect::new(10.0, 10.0, 120.0, 30.0),
                UiRect::new(0.0, 0.0, 320.0, 160.0),
                PopupPlacement::default(),
            )),
            DropdownSelectOptions::default(),
        );
        let popup = open_nodes.popup.as_ref().unwrap();
        let trigger_accessibility = open_document
            .node(open_nodes.trigger)
            .accessibility
            .as_ref()
            .unwrap();
        assert_eq!(
            trigger_accessibility.relations.active_descendant,
            Some(popup.rows[0])
        );
    }
}
