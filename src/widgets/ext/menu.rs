//! Popup, menu, dropdown, and command-palette widgets.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentageAuto, Position,
    Rect as TaffyRect, Size as TaffySize, Style,
};

use super::menu_list::{menu_list_popup, MenuListNodes, MenuListOptions};

use crate::tooltips::{CommandTooltipResolver, ShortcutFormatter};
use crate::widgets::{
    inline_intrinsic_base_size, publish_inline_intrinsic_size, single_line_text_style,
};
use crate::{
    layout, length, AccessibilityAction, AccessibilityLiveRegion, AccessibilityMeta,
    AccessibilityRole, AnimationMachine, ClipBehavior, ClipScope, ColorRgba, CommandId,
    CommandRegistry, CommandScope, ImageContent, InputBehavior, KeyCode, KeyModifiers, LayoutStyle,
    ScrollAxes, ShaderEffect, StrokeStyle, TextStyle, UiDocument, UiInputEvent, UiNode, UiNodeId,
    UiNodeStyle, UiPortalTarget, UiRect, UiSize, UiVisual, WidgetActionBinding,
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
    pub primary_rect: UiRect,
    pub unconstrained_rect: UiRect,
    pub overflow_before_constrain: f32,
    pub overflow_after_constrain: f32,
    pub constrained: bool,
}

impl PopupLayout {
    pub fn diagnostic_summary(&self) -> String {
        format!(
            "popup placement side={:?} flipped={} constrained={} primary={:?} unconstrained={:?} final={:?} overflow_before={:.2} overflow_after={:.2}",
            self.side,
            self.flipped,
            self.constrained,
            self.primary_rect,
            self.unconstrained_rect,
            self.rect,
            self.overflow_before_constrain,
            self.overflow_after_constrain
        )
    }
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
    pub clip_scope: ClipScope,
    pub layer: Option<crate::platform::UiLayer>,
    pub portal: UiPortalTarget,
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
            clip_scope: ClipScope::Viewport,
            layer: Some(crate::platform::UiLayer::AppOverlay),
            portal: UiPortalTarget::AppOverlay,
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
    let inner_viewport = layout::inset_rect(viewport, placement.viewport_margin.max(0.0));
    let primary = popup_rect_for_anchor(anchor, popup_size, placement.side, placement);
    let mut rect = primary;
    let mut side = placement.side;
    let mut flipped = false;

    if placement.flip {
        let opposite_side = placement.side.opposite();
        let opposite = popup_rect_for_anchor(anchor, popup_size, opposite_side, placement);
        if layout::rect_overflow_amount(opposite, inner_viewport)
            < layout::rect_overflow_amount(primary, inner_viewport)
        {
            rect = opposite;
            side = opposite_side;
            flipped = true;
        }
    }

    let unconstrained_rect = rect;
    let overflow_before_constrain =
        layout::rect_overflow_amount(unconstrained_rect, inner_viewport);
    if placement.constrain_to_viewport {
        rect = layout::contain_rect(rect, inner_viewport, UiSize::ZERO);
    }
    let overflow_after_constrain = layout::rect_overflow_amount(rect, inner_viewport);

    PopupLayout {
        rect,
        side,
        flipped,
        primary_rect: primary,
        unconstrained_rect,
        overflow_before_constrain,
        overflow_after_constrain,
        constrained: rect != unconstrained_rect,
    }
}

pub fn centered_popup_rect(viewport: UiRect, popup_size: UiSize, viewport_margin: f32) -> UiRect {
    let inner = layout::inset_rect(viewport, viewport_margin.max(0.0));
    layout::contain_rect(
        UiRect::new(
            inner.x + (inner.width - popup_size.width) * 0.5,
            inner.y + (inner.height - popup_size.height) * 0.5,
            popup_size.width,
            popup_size.height,
        ),
        inner,
        UiSize::ZERO,
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
        clip_scope,
        layer,
        portal,
        scroll_axes,
        accessibility,
        shader,
        animation,
    } = options;
    let mut node = UiNode::container(
        name,
        UiNodeStyle {
            layout: absolute_rect_style(rect).style,
            clip,
            z_index,
            ..Default::default()
        },
    )
    .with_clip_scope(clip_scope)
    .with_visual(visual);
    if let Some(layer) = layer {
        node = node.with_layer(layer);
    }

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

    document.add_portal_child(parent, portal, node)
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

pub fn submenu(id: impl Into<String>, label: impl Into<String>, items: Vec<MenuItem>) -> MenuItem {
    MenuItem::submenu(id, label, items)
}

pub fn submenu_button(
    id: impl Into<String>,
    label: impl Into<String>,
    items: Vec<MenuItem>,
) -> MenuItem {
    MenuItem::submenu(id, label, items)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuButtonState {
    pub open: bool,
    pub navigation: MenuNavigationState,
}

impl MenuButtonState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, items: &[MenuItem]) -> Option<Vec<usize>> {
        self.open = true;
        if self.navigation.active_path.is_empty() {
            self.navigation.open_root(items)
        } else {
            Some(self.navigation.active_path.clone())
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.navigation.clear();
    }

    pub fn toggle(&mut self, items: &[MenuItem]) -> MenuButtonOutcome {
        if self.open {
            self.close();
            MenuButtonOutcome {
                closed: true,
                ..Default::default()
            }
        } else {
            MenuButtonOutcome {
                opened: true,
                active_path: self.open(items),
                ..Default::default()
            }
        }
    }

    pub fn selected(&self, items: &[MenuItem]) -> Option<MenuSelection> {
        self.navigation.select_active(items)
    }

    pub fn handle_event(&mut self, items: &[MenuItem], event: &UiInputEvent) -> MenuButtonOutcome {
        if !self.open {
            if menu_button_open_event(event) {
                return MenuButtonOutcome {
                    opened: true,
                    active_path: self.open(items),
                    ..Default::default()
                };
            }
            return MenuButtonOutcome::default();
        }

        let navigation = self.navigation.handle_event(items, event);
        let mut outcome = MenuButtonOutcome {
            active_path: navigation.active_path.clone(),
            opened_submenu: navigation.opened_submenu,
            closed_submenu: navigation.closed_submenu,
            selected: navigation.selected,
            ..Default::default()
        };
        if navigation.closed || outcome.selected.is_some() {
            self.open = false;
            outcome.closed = true;
        }
        outcome
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuButtonOutcome {
    pub opened: bool,
    pub closed: bool,
    pub opened_submenu: bool,
    pub closed_submenu: bool,
    pub active_path: Option<Vec<usize>>,
    pub selected: Option<MenuSelection>,
}

impl MenuButtonOutcome {
    pub fn is_empty(&self) -> bool {
        !self.opened
            && !self.closed
            && !self.opened_submenu
            && !self.closed_submenu
            && self.active_path.is_none()
            && self.selected.is_none()
    }

    pub fn selected_command(&self) -> Option<MenuCommandSelection> {
        self.selected
            .as_ref()
            .and_then(MenuSelection::command_selection)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuSubmenuAnchor {
    pub path: Vec<usize>,
    pub anchor: UiRect,
}

impl MenuSubmenuAnchor {
    pub fn new(path: impl Into<Vec<usize>>, anchor: UiRect) -> Self {
        Self {
            path: path.into(),
            anchor,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuButtonAnchors {
    pub trigger: UiRect,
    pub viewport: UiRect,
    pub submenus: Vec<MenuSubmenuAnchor>,
}

impl MenuButtonAnchors {
    pub fn new(trigger: UiRect, viewport: UiRect) -> Self {
        Self {
            trigger,
            viewport,
            submenus: Vec::new(),
        }
    }

    pub fn with_submenu_anchor(mut self, path: impl Into<Vec<usize>>, anchor: UiRect) -> Self {
        self.submenus.push(MenuSubmenuAnchor::new(path, anchor));
        self
    }

    pub fn submenu_anchor(&self, path: &[usize]) -> Option<UiRect> {
        self.submenus
            .iter()
            .find(|submenu| submenu.path == path)
            .map(|submenu| submenu.anchor)
    }
}

#[derive(Debug, Clone)]
pub struct MenuButtonOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub open_visual: Option<UiVisual>,
    pub disabled_visual: Option<UiVisual>,
    pub text_style: TextStyle,
    pub disabled_text_style: TextStyle,
    pub leading_image: Option<ImageContent>,
    pub image_size: UiSize,
    pub shader: Option<ShaderEffect>,
    pub animation: Option<AnimationMachine>,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
    pub popup_placement: PopupPlacement,
    pub submenu_placement: PopupPlacement,
    pub popup_menu: MenuListOptions,
}

impl Default for MenuButtonOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                size: TaffySize {
                    width: Dimension::auto(),
                    height: length(30.0),
                },
                padding: TaffyRect {
                    left: length_percentage(10.0),
                    right: length_percentage(10.0),
                    top: length_percentage(0.0),
                    bottom: length_percentage(0.0),
                },
                ..Default::default()
            }),
            visual: UiVisual::panel(ColorRgba::new(36, 42, 52, 255), None, 3.0),
            open_visual: Some(UiVisual::panel(ColorRgba::new(45, 55, 68, 255), None, 3.0)),
            disabled_visual: Some(UiVisual::panel(ColorRgba::new(30, 34, 40, 170), None, 3.0)),
            text_style: TextStyle::default(),
            disabled_text_style: TextStyle {
                color: ColorRgba::new(138, 148, 164, 255),
                ..Default::default()
            },
            leading_image: None,
            image_size: UiSize::new(18.0, 18.0),
            shader: None,
            animation: None,
            enabled: true,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
            popup_placement: PopupPlacement::new(PopupSide::Bottom, PopupAlign::Start),
            submenu_placement: PopupPlacement::new(PopupSide::Right, PopupAlign::Start),
            popup_menu: MenuListOptions::default(),
        }
    }
}

impl MenuButtonOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_leading_image(mut self, image: impl Into<ImageContent>) -> Self {
        self.leading_image = Some(image.into());
        self
    }

    pub fn with_image_size(mut self, size: UiSize) -> Self {
        self.image_size = size;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn with_accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    pub fn with_popup_placement(mut self, placement: PopupPlacement) -> Self {
        self.popup_placement = placement;
        self
    }

    pub fn with_submenu_placement(mut self, placement: PopupPlacement) -> Self {
        self.submenu_placement = placement;
        self
    }

    pub fn with_popup_menu(mut self, options: MenuListOptions) -> Self {
        self.popup_menu = options;
        self
    }

    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.popup_menu.action_prefix = Some(prefix.into());
        self
    }

    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuButtonNodes {
    pub button: UiNodeId,
    pub popup: Option<MenuListNodes>,
    pub submenus: Vec<MenuListNodes>,
}

#[allow(clippy::too_many_arguments)]
pub fn menu_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    items: &[MenuItem],
    state: &MenuButtonState,
    anchors: Option<&MenuButtonAnchors>,
    options: MenuButtonOptions,
) -> MenuButtonNodes {
    menu_button_with_image(
        document,
        parent,
        name,
        label_text.into(),
        items,
        state,
        anchors,
        options,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn image_text_menu_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    image: impl Into<ImageContent>,
    items: &[MenuItem],
    state: &MenuButtonState,
    anchors: Option<&MenuButtonAnchors>,
    mut options: MenuButtonOptions,
) -> MenuButtonNodes {
    options.leading_image = Some(image.into());
    menu_button_with_image(
        document,
        parent,
        name,
        label_text.into(),
        items,
        state,
        anchors,
        options,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn image_menu_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    image: impl Into<ImageContent>,
    items: &[MenuItem],
    state: &MenuButtonState,
    anchors: Option<&MenuButtonAnchors>,
    mut options: MenuButtonOptions,
) -> MenuButtonNodes {
    options.leading_image = Some(image.into());
    menu_button_with_image(
        document,
        parent,
        name,
        String::new(),
        items,
        state,
        anchors,
        options,
    )
}

#[allow(clippy::too_many_arguments)]
fn menu_button_with_image(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: String,
    items: &[MenuItem],
    state: &MenuButtonState,
    anchors: Option<&MenuButtonAnchors>,
    options: MenuButtonOptions,
) -> MenuButtonNodes {
    let name = name.into();
    let button = menu_button_trigger(
        document,
        parent,
        name.clone(),
        label_text,
        state.open,
        &options,
    );
    let popup = state
        .open
        .then(|| {
            anchors.map(|anchors| {
                let mut popup_menu = options.popup_menu.clone();
                popup_menu.z_index = popup_menu.z_index.max(101);
                menu_list_popup(
                    document,
                    parent,
                    format!("{name}.popup"),
                    AnchoredPopup::new(anchors.trigger, anchors.viewport, options.popup_placement),
                    items,
                    state.navigation.active_path.first().copied(),
                    popup_menu,
                )
            })
        })
        .flatten();
    if let Some(popup) = &popup {
        if let Some(accessibility) = document.node_mut(button).accessibility.as_mut() {
            accessibility.relations.controls.push(popup.root);
        }
    }
    let submenus = if state.open {
        anchors
            .map(|anchors| {
                menu_button_submenus(
                    document,
                    parent,
                    &name,
                    items,
                    &state.navigation.active_path,
                    anchors,
                    &options,
                )
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    MenuButtonNodes {
        button,
        popup,
        submenus,
    }
}

fn menu_button_trigger(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: String,
    label_text: String,
    open: bool,
    options: &MenuButtonOptions,
) -> UiNodeId {
    let mut layout = options.layout.style.clone();
    layout.display = Display::Flex;
    layout.flex_direction = FlexDirection::Row;
    layout.align_items = Some(AlignItems::Center);
    layout.justify_content = layout.justify_content.or(Some(JustifyContent::Center));

    let visual = if !options.enabled {
        options.disabled_visual.unwrap_or(options.visual)
    } else if open {
        options.open_visual.unwrap_or(options.visual)
    } else {
        options.visual
    };
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout,
            clip: ClipBehavior::Clip,
            z_index: if open { 20 } else { 0 },
            ..Default::default()
        },
    )
    .with_input(if options.enabled {
        InputBehavior::BUTTON
    } else {
        InputBehavior::NONE
    })
    .with_visual(visual)
    .with_accessibility(menu_button_accessibility(&name, &label_text, open, options));
    if let Some(shader) = options.shader.clone() {
        node = node.with_shader(shader);
    }
    if let Some(animation) = options.animation.clone() {
        node = node.with_animation(animation);
    }

    let root = document.add_child(parent, node);
    if let Some(action) = options.action.clone() {
        document.node_mut(root).action = Some(action);
    }
    if let Some(image) = &options.leading_image {
        leading_image(
            document,
            root,
            format!("{name}.image"),
            image.clone(),
            &menu_button_accessibility_label(&name, &label_text, options),
            options.image_size,
        );
    }
    if !label_text.is_empty() {
        label(
            document,
            root,
            format!("{name}.label"),
            label_text,
            if options.enabled {
                options.text_style.clone()
            } else {
                options.disabled_text_style.clone()
            },
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        );
    }
    root
}

fn menu_button_submenus(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    items: &[MenuItem],
    active_path: &[usize],
    anchors: &MenuButtonAnchors,
    options: &MenuButtonOptions,
) -> Vec<MenuListNodes> {
    let mut submenus = Vec::new();
    let mut level_items = items;
    for depth in 0..active_path.len() {
        let index = active_path[depth];
        let Some(item) = level_items.get(index) else {
            break;
        };
        let Some(children) = item.children() else {
            break;
        };
        let path = active_path[..=depth].to_vec();
        let Some(anchor) = anchors.submenu_anchor(&path) else {
            break;
        };
        let mut popup_menu = options.popup_menu.clone();
        popup_menu.z_index = popup_menu.z_index.max(101).saturating_add(depth as i16 + 1);
        let submenu = menu_list_popup(
            document,
            parent,
            format!("{name}.submenu.{}", path_label(&path)),
            AnchoredPopup::new(anchor, anchors.viewport, options.submenu_placement),
            children,
            active_path.get(depth + 1).copied(),
            popup_menu,
        );
        submenus.push(submenu);
        level_items = children;
    }
    submenus
}

fn menu_button_accessibility(
    name: &str,
    label_text: &str,
    open: bool,
    options: &MenuButtonOptions,
) -> AccessibilityMeta {
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Button)
        .label(menu_button_accessibility_label(name, label_text, options))
        .value(if open { "open" } else { "closed" })
        .expanded(open)
        .action(if open {
            AccessibilityAction::new("close", "Close menu")
        } else {
            AccessibilityAction::new("open", "Open menu")
        });
    if let Some(hint) = &options.accessibility_hint {
        accessibility = accessibility.hint(hint.clone());
    }
    if options.enabled {
        accessibility.focusable()
    } else {
        accessibility.disabled()
    }
}

fn menu_button_accessibility_label(
    name: &str,
    label_text: &str,
    options: &MenuButtonOptions,
) -> String {
    options
        .accessibility_label
        .clone()
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| {
            if label_text.is_empty() {
                name.to_string()
            } else {
                label_text.to_string()
            }
        })
}

fn menu_button_open_event(event: &UiInputEvent) -> bool {
    matches!(
        event,
        UiInputEvent::Key {
            key: KeyCode::Enter | KeyCode::Character(' ') | KeyCode::ArrowDown,
            ..
        }
    )
}

fn path_label(path: &[usize]) -> String {
    path.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(".")
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

pub(in crate::widgets::ext) fn next_menu_typeahead_index(
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

pub(in crate::widgets::ext) fn visible_row_count(count: usize, max_visible_rows: usize) -> usize {
    if max_visible_rows == 0 {
        0
    } else {
        count.min(max_visible_rows)
    }
}

pub(in crate::widgets::ext) fn visible_match_range(
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

pub(in crate::widgets::ext) fn row_style(height: f32) -> UiNodeStyle {
    UiNodeStyle {
        layout: LayoutStyle::from_taffy_style(Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            size: TaffySize {
                width: Dimension::percent(1.0),
                height: length(height.max(0.0)),
            },
            padding: TaffyRect {
                left: taffy::prelude::LengthPercentage::length(10.0),
                right: taffy::prelude::LengthPercentage::length(10.0),
                top: taffy::prelude::LengthPercentage::length(6.0),
                bottom: taffy::prelude::LengthPercentage::length(6.0),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })
        .style,
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

pub(in crate::widgets::ext) fn leading_image(
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

pub(in crate::widgets::ext) fn menu_accessibility_label(
    name: &str,
    explicit: Option<&String>,
) -> String {
    explicit.cloned().unwrap_or_else(|| name.to_string())
}

pub(in crate::widgets::ext) fn set_active_descendant(
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

pub(in crate::widgets::ext) fn command_shortcut_label(
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

pub(in crate::widgets::ext) fn button_like(
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
pub(in crate::widgets::ext) fn button_like_with_input(
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
    let text_style = single_line_text_style(text_style);
    let mut layout_style = layout.style;
    layout_style.display = Display::Flex;
    layout_style.flex_direction = FlexDirection::Row;
    layout_style.align_items = Some(AlignItems::Center);
    layout_style.justify_content = layout_style
        .justify_content
        .or(Some(taffy::prelude::JustifyContent::Center));
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: layout_style.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(input)
        .with_visual(visual),
    );
    let label = label(
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
    publish_inline_intrinsic_size(
        document,
        root,
        vec![label],
        inline_intrinsic_base_size(&layout_style, &[], 1),
    );
    root
}

pub(in crate::widgets::ext) fn label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    text_style: TextStyle,
    layout: LayoutStyle,
) -> UiNodeId {
    document.add_child(parent, UiNode::text(name, text, text_style, layout))
}

pub(in crate::widgets::ext) fn length_percentage(value: f32) -> taffy::prelude::LengthPercentage {
    taffy::prelude::LengthPercentage::length(value)
}

pub(in crate::widgets::ext) fn normalize(value: &str) -> String {
    value.to_lowercase()
}

pub(in crate::widgets::ext) fn first_typeahead_character(text: &str) -> Option<char> {
    text.chars()
        .find(|character| is_typeahead_character(*character))
}

pub(in crate::widgets::ext) fn is_typeahead_character(character: char) -> bool {
    !character.is_control() && !character.is_whitespace()
}

pub(in crate::widgets::ext) fn normalized_character(character: char) -> Option<String> {
    is_typeahead_character(character).then(|| character.to_lowercase().collect())
}

pub(in crate::widgets::ext) fn next_matching_index(
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

pub(in crate::widgets::ext) fn pop_last_char(text: &mut String) -> bool {
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
    use crate::input::RawPointerEvent;
    use crate::tooltips::{ContextMenuSuppressedReason, HelpItemState, ShortcutFormatter};
    use crate::widgets::ext::{
        command_palette::*, context_menu::*, dropdown::*, menu_bar::*, menu_list::*,
    };
    use crate::{
        root_style, AccessibilityMeta, AccessibilityRole, AnimatedValues, AnimationMachine,
        AnimationState, AnimationTransition, AnimationTrigger, ApproxTextMeasurer, Command,
        CommandId, CommandMeta, CommandRegistry, CommandScope, KeyModifiers, PointerButton,
        PointerEventKind, ShaderEffect, Shortcut, UiContent, UiPoint, WidgetActionBinding,
        APP_OVERLAY_PORTAL,
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
        assert!(!layout.constrained);
        assert_eq!(layout.overflow_after_constrain, 0.0);
        assert!(layout.rect.x >= 8.0, "{layout:?}");
        assert!(layout.rect.right() <= 292.0, "{layout:?}");
        assert!(layout.rect.y >= 8.0, "{layout:?}");
        assert!(layout.rect.bottom() <= 212.0, "{layout:?}");

        let clamped = place_popup(
            UiRect::new(260.0, 190.0, 32.0, 24.0),
            UiSize::new(260.0, 180.0),
            UiRect::new(0.0, 0.0, 300.0, 220.0),
            PopupPlacement::new(PopupSide::Bottom, PopupAlign::End)
                .with_offset(6.0)
                .with_viewport_margin(8.0)
                .with_flip(false),
        );

        assert_eq!(clamped.side, PopupSide::Bottom);
        assert!(!clamped.flipped);
        assert!(clamped.constrained);
        assert!(clamped.overflow_before_constrain > clamped.overflow_after_constrain);
        assert_eq!(clamped.overflow_after_constrain, 0.0);
        assert!(
            clamped.diagnostic_summary().contains("constrained=true"),
            "{}",
            clamped.diagnostic_summary()
        );
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
        assert_eq!(node.style.layout.position, Position::Absolute);
        assert_eq!(node.style.z_index, 100);
        assert_eq!(node.clip_scope, ClipScope::Viewport);
        assert_eq!(node.layer, Some(crate::platform::UiLayer::AppOverlay));
        assert!(node.scroll.is_some());
        let portal = document
            .portal_host(APP_OVERLAY_PORTAL)
            .expect("app overlay portal");
        assert_eq!(node.parent, Some(portal));
    }

    #[test]
    fn popup_panel_can_stay_in_parent_tree_for_inline_previews() {
        let mut document = UiDocument::new(root_style(300.0, 200.0));
        let root = document.root;
        let parent = document.add_child(
            root,
            UiNode::container("parent", LayoutStyle::column().with_width(200.0)),
        );
        let popup = popup_panel(
            &mut document,
            parent,
            "popup",
            UiRect::new(16.0, 20.0, 120.0, 80.0),
            PopupOptions {
                portal: UiPortalTarget::Parent,
                ..Default::default()
            },
        );

        assert_eq!(document.node(popup).parent, Some(parent));
        assert!(document.portal_host(APP_OVERLAY_PORTAL).is_none());
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
        assert_eq!(state.active_index(), Some(1));

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
        assert!(!state.is_open());
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
        assert_eq!(state.selected_index(), Some(2));

        state.open(&options);
        let outcome = state.handle_event(&options, &UiInputEvent::TextInput("a".to_string()));
        assert_eq!(outcome.active, Some(0));
        assert_eq!(state.active_index(), Some(0));
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
            "example.filter",
            &options,
            &state,
            SearchableSelectSpec::new()
                .max_visible_rows(3)
                .placeholder("Choose")
                .accessibility_label("Example filter")
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
            Some("example.filter.option.4")
        );
        assert_eq!(
            contract.trigger_accessibility.role,
            AccessibilityRole::ComboBox
        );
        assert_eq!(
            contract.trigger_accessibility.label.as_deref(),
            Some("Example filter")
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
        let state = SelectMenuState::with_selected(1)
            .with_open(&options)
            .with_active(&options, 2);
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
        let state = SelectMenuState::with_selected(0)
            .with_open(&options)
            .with_active(&options, 1);
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
    fn menu_button_state_opens_navigates_submenus_and_selects_commands() {
        let items = vec![submenu_button(
            "file",
            "File",
            vec![
                MenuItem::command("new", "New"),
                MenuItem::command("open", "Open"),
            ],
        )];
        let mut state = MenuButtonState::new();

        let outcome = state.handle_event(
            &items,
            &UiInputEvent::Key {
                key: KeyCode::ArrowDown,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(outcome.opened);
        assert_eq!(outcome.active_path, Some(vec![0]));
        assert!(state.open);

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
        assert_eq!(outcome.active_path, Some(vec![0, 1]));

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
                id: Some("open".to_string()),
                index_path: vec![0, 1],
            })
        );
        assert!(outcome.closed);
        assert!(!state.open);
    }

    #[test]
    fn menu_button_builds_trigger_popup_and_submenu_popups() {
        let mut document = UiDocument::new(root_style(640.0, 480.0));
        let root = document.root;
        let items = vec![
            MenuItem::command("save", "Save").shortcut("Ctrl+S"),
            submenu(
                "recent",
                "Recent",
                vec![
                    MenuItem::command("demo", "demo.rs"),
                    MenuItem::command("notes", "notes.md"),
                ],
            ),
        ];
        let state = MenuButtonState {
            open: true,
            navigation: MenuNavigationState::with_active_path(vec![1, 0]),
        };
        let anchors = MenuButtonAnchors::new(
            UiRect::new(20.0, 20.0, 80.0, 30.0),
            UiRect::new(0.0, 0.0, 640.0, 480.0),
        )
        .with_submenu_anchor(vec![1], UiRect::new(220.0, 58.0, 240.0, 28.0));

        let nodes = menu_button(
            &mut document,
            root,
            "file-menu",
            "File",
            &items,
            &state,
            Some(&anchors),
            MenuButtonOptions::default()
                .with_action(WidgetActionBinding::action("file.toggle"))
                .with_action_prefix("menu"),
        );

        let popup = nodes.popup.expect("root popup");
        assert_eq!(popup.rows.len(), 2);
        assert_eq!(nodes.submenus.len(), 1);
        assert_eq!(nodes.submenus[0].rows.len(), 2);
        assert_eq!(
            document.node(popup.root).style.layout.position,
            Position::Absolute
        );
        document
            .compute_layout(UiSize::new(640.0, 480.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let first_row = document.node(popup.rows[0]).layout.rect;
        let second_row = document.node(popup.rows[1]).layout.rect;
        assert_eq!(first_row.x, second_row.x);
        assert!(second_row.y > first_row.y, "{first_row:?} {second_row:?}");
        assert!(
            document.node(nodes.submenus[0].root).style.z_index
                > document.node(popup.root).style.z_index
        );

        let button = document.node(nodes.button);
        assert_eq!(
            button
                .action
                .as_ref()
                .and_then(WidgetActionBinding::action_id)
                .map(AsRef::as_ref),
            Some("file.toggle")
        );
        let accessibility = button.accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::Button);
        assert_eq!(accessibility.label.as_deref(), Some("File"));
        assert_eq!(accessibility.expanded, Some(true));
        assert_eq!(accessibility.relations.controls, vec![popup.root]);

        let submenu_row = document.node(nodes.submenus[0].rows[0]);
        assert_eq!(
            submenu_row
                .action
                .as_ref()
                .and_then(WidgetActionBinding::action_id)
                .map(AsRef::as_ref),
            Some("menu.demo")
        );
    }

    #[test]
    fn image_menu_button_variants_add_image_content_and_accessible_labels() {
        let mut document = UiDocument::new(root_style(320.0, 180.0));
        let root = document.root;
        let items = vec![MenuItem::command("copy", "Copy")];
        let state = MenuButtonState::new();

        let image_only = image_menu_button(
            &mut document,
            root,
            "tools",
            ImageContent::new("icons.tools"),
            &items,
            &state,
            None,
            MenuButtonOptions::default().with_accessibility_label("Tools"),
        );
        let image_text = image_text_menu_button(
            &mut document,
            root,
            "insert",
            "Insert",
            ImageContent::new("icons.plus"),
            &items,
            &state,
            None,
            MenuButtonOptions::default(),
        );

        assert!(document
            .node(image_only.button)
            .children
            .iter()
            .any(|child| {
                matches!(
                    &document.node(*child).content,
                    UiContent::Image(image) if image.key == "icons.tools"
                )
            }));
        assert_eq!(
            document
                .node(image_only.button)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Tools")
        );
        assert!(document
            .node(image_text.button)
            .children
            .iter()
            .any(|child| {
                matches!(
                    &document.node(*child).content,
                    UiContent::Text(text) if text.text == "Insert"
                )
            }));
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
    fn context_menu_state_opens_from_pointer_and_keyboard_requests() {
        let items = vec![
            MenuItem::separator(),
            MenuItem::command("copy", "Copy"),
            MenuItem::command("paste", "Paste"),
        ];
        let anchor = UiRect::new(12.0, 18.0, 40.0, 20.0);
        let pointer = RawPointerEvent::new(
            PointerEventKind::Down(PointerButton::Secondary),
            UiPoint::new(32.0, 44.0),
            7,
        );
        let mut state = ContextMenuState::closed();

        let opened = state.open_from_pointer_event(
            UiNodeId(4),
            anchor,
            pointer,
            HelpItemState::ENABLED,
            &items,
        );

        assert!(opened.menu.opened);
        assert_eq!(opened.menu.active, Some(1));
        assert_eq!(opened.resolution.suppressed_reason, None);
        assert_eq!(state.anchor, UiPoint::new(32.0, 44.0));
        assert_eq!(state.active, Some(1));

        let primary = RawPointerEvent::new(
            PointerEventKind::Down(PointerButton::Primary),
            UiPoint::new(18.0, 22.0),
            8,
        );
        let ignored = state.open_from_pointer_event(
            UiNodeId(4),
            anchor,
            primary,
            HelpItemState::ENABLED,
            &items,
        );
        assert!(!ignored.menu.opened);
        assert_eq!(ignored.resolution.request, None);

        let mut keyboard_state = ContextMenuState::closed();
        let keyboard =
            keyboard_state.open_from_keyboard(UiNodeId(4), anchor, HelpItemState::ENABLED, &items);
        assert!(keyboard.menu.opened);
        assert_eq!(keyboard.menu.active, Some(1));
        assert!(keyboard_state.open);

        let mut key_state = ContextMenuState::closed();
        let key = key_state.open_from_key_event(
            UiNodeId(4),
            anchor,
            KeyCode::F10,
            KeyModifiers {
                shift: true,
                ..KeyModifiers::NONE
            },
            HelpItemState::ENABLED,
            &items,
        );
        assert!(key.menu.opened);
        assert_eq!(key.menu.active, Some(1));

        let ignored_key = key_state.open_from_key_event(
            UiNodeId(4),
            anchor,
            KeyCode::F10,
            KeyModifiers::NONE,
            HelpItemState::ENABLED,
            &items,
        );
        assert!(!ignored_key.menu.opened);
        assert_eq!(ignored_key.resolution.request, None);
    }

    #[test]
    fn context_menu_state_reports_suppressed_open_requests() {
        let items = vec![MenuItem::command("copy", "Copy")];
        let mut state = ContextMenuState::closed();

        let disabled = state.open_from_keyboard(
            UiNodeId(4),
            UiRect::new(10.0, 10.0, 20.0, 20.0),
            HelpItemState::disabled(),
            &items,
        );

        assert!(!disabled.menu.opened);
        assert!(!state.open);
        assert_eq!(
            disabled.resolution.suppressed_reason,
            Some(ContextMenuSuppressedReason::Disabled)
        );
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

        let state = CommandPaletteState::new()
            .with_query("project")
            .with_first_active_match(&items);
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

        let mut history = CommandPaletteHistory::with_capacity(2);
        assert_eq!(
            outcome.record_selection(&mut history),
            Some(CommandId::from("file.save"))
        );
        history.record("edit.quantize");
        history.record("file.save");
        let items =
            command_palette_items_from_registry_with_history(&registry, &[], &formatter, &history);
        assert_eq!(
            items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["file.save", "edit.quantize"]
        );
        let save = items.iter().find(|item| item.id == "file.save").unwrap();
        assert!(save.keywords.iter().any(|keyword| keyword == "recent"));
        assert_eq!(
            filter_command_palette(&items, "recent", 10)
                .iter()
                .map(|palette_match| palette_match.id.as_str())
                .collect::<Vec<_>>(),
            vec!["file.save", "edit.quantize"]
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
        assert_eq!(state.query(), "save");
        assert_eq!(state.matches(&items).len(), 1);
        assert_eq!(state.active_match(), Some(0));
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
        assert_eq!(state.query(), "");
        assert_eq!(state.active_match(), Some(0));
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
        let state = CommandPaletteState::new()
            .with_query("o")
            .with_first_active_match(&items);

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
        let open_state = SelectMenuState::with_selected(1)
            .with_open(&options)
            .with_active(&options, 0);
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
