//! Popup, menu, dropdown, and command-palette widgets.

use std::cmp::Ordering;

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentageAuto, Position,
    Rect as TaffyRect, Size as TaffySize, Style,
};

use crate::{
    length, ClipBehavior, ColorRgba, InputBehavior, KeyCode, ScrollAxes, StrokeStyle, TextStyle,
    UiDocument, UiInputEvent, UiNode, UiNodeId, UiNodeStyle, UiPoint, UiRect, UiSize, UiVisual,
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

#[derive(Debug, Clone, Copy)]
pub struct PopupOptions {
    pub visual: UiVisual,
    pub z_index: i16,
    pub clip: ClipBehavior,
    pub scroll_axes: ScrollAxes,
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
    let mut node = UiNode::container(
        name,
        UiNodeStyle {
            layout: absolute_rect_style(rect),
            clip: options.clip,
            z_index: options.z_index,
            ..Default::default()
        },
    )
    .with_visual(options.visual);

    if options.scroll_axes != ScrollAxes::NONE {
        node = node.with_scroll(options.scroll_axes);
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
    pub kind: MenuItemKind,
}

impl MenuItem {
    pub fn command(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            label: label.into(),
            enabled: true,
            shortcut: None,
            kind: MenuItemKind::Command,
        }
    }

    pub fn check(id: impl Into<String>, label: impl Into<String>, checked: bool) -> Self {
        Self {
            id: Some(id.into()),
            label: label.into(),
            enabled: true,
            shortcut: None,
            kind: MenuItemKind::Check { checked },
        }
    }

    pub fn submenu(id: impl Into<String>, label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self {
            id: Some(id.into()),
            label: label.into(),
            enabled: true,
            shortcut: None,
            kind: MenuItemKind::Submenu { items },
        }
    }

    pub fn separator() -> Self {
        Self {
            id: None,
            label: String::new(),
            enabled: false,
            shortcut: None,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    Next,
    Previous,
}

pub fn menu_item_at_path<'a>(items: &'a [MenuItem], path: &[usize]) -> Option<&'a MenuItem> {
    let (first, rest) = path.split_first()?;
    let item = items.get(*first)?;
    if rest.is_empty() {
        return Some(item);
    }
    menu_item_at_path(item.children()?, rest)
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
}

impl SelectOption {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            enabled: true,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
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
        let UiInputEvent::Key { key, .. } = event else {
            return outcome;
        };

        match key {
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
            _ => {}
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
            ..Default::default()
        },
    );
    let rows = populate_select_menu(document, root, &name, options, state, &menu_options);
    SelectMenuNodes { root, rows }
}

#[derive(Debug, Clone)]
pub struct DropdownSelectOptions {
    pub trigger_layout: Style,
    pub trigger_visual: UiVisual,
    pub text_style: TextStyle,
    pub placeholder: String,
    pub menu: SelectMenuOptions,
}

impl Default for DropdownSelectOptions {
    fn default() -> Self {
        Self {
            trigger_layout: Style {
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
            },
            trigger_visual: UiVisual::panel(
                ColorRgba::new(31, 37, 46, 255),
                Some(StrokeStyle::new(ColorRgba::new(84, 98, 121, 255), 1.0)),
                4.0,
            ),
            text_style: TextStyle::default(),
            placeholder: String::new(),
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

    DropdownSelectNodes {
        trigger,
        popup: popup.flatten(),
    }
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
            ..Default::default()
        },
    );
    let rows = populate_menu_list(document, root, &name, items, active, &options);
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

        let UiInputEvent::Key { key, .. } = event else {
            return outcome;
        };

        match key {
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
            KeyCode::Enter => {
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
            _ => {}
        }

        outcome
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
    pub layout: Style,
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
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(30.0),
                },
                ..Default::default()
            },
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
        .with_visual(options.visual),
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
            Style {
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
            },
            visual,
            text_style,
            if menu.enabled {
                InputBehavior::BUTTON
            } else {
                InputBehavior::NONE
            },
        );
        buttons.push(button);
    }

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

    pub fn matches(&self, items: &[CommandPaletteItem]) -> Vec<CommandPaletteMatch> {
        filter_command_palette(items, &self.query, self.max_results)
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
            },
        )
    } else {
        document.add_child(
            parent,
            UiNode::container(
                name.clone(),
                UiNodeStyle {
                    layout: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        size: TaffySize {
                            width: length(options.width.max(0.0)),
                            height: length(height.max(0.0)),
                        },
                        padding: TaffyRect::length(4.0),
                        ..Default::default()
                    },
                    clip: ClipBehavior::Clip,
                    z_index: options.z_index,
                    ..Default::default()
                },
            )
            .with_visual(options.panel_visual),
        )
    };

    let input = document.add_child(
        root,
        UiNode::container(
            format!("{name}.input"),
            UiNodeStyle {
                layout: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(34.0),
                    },
                    padding: TaffyRect::length(8.0),
                    ..Default::default()
                },
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(options.input_visual),
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
        Style {
            size: TaffySize {
                width: Dimension::percent(1.0),
                height: Dimension::auto(),
            },
            ..Default::default()
        },
    );

    let mut list_node = UiNode::container(
        format!("{name}.results"),
        UiNodeStyle {
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(visible_rows as f32 * options.row_height),
                },
                ..Default::default()
            },
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    );
    if matches.len() > visible_rows {
        list_node = list_node.with_scroll(ScrollAxes::VERTICAL);
    }
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
            UiNode::container(
                format!("{name}.result.{}", palette_match.index),
                UiNodeStyle {
                    layout: Style {
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
                    },
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(if item.enabled {
                InputBehavior::BUTTON
            } else {
                InputBehavior::NONE
            })
            .with_visual(visual),
        );
        label(
            document,
            row,
            format!("{name}.result.{}.title", palette_match.index),
            &item.title,
            text_style,
            Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
        );
        if let Some(shortcut) = &item.shortcut {
            label(
                document,
                row,
                format!("{name}.result.{}.shortcut", palette_match.index),
                shortcut,
                options.muted_text_style.clone(),
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            );
        }
        rows.push(row);
    }

    CommandPaletteNodes { root, input, rows }
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

fn absolute_rect_style(rect: UiRect) -> Style {
    Style {
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
    }
}

fn menu_container_node(
    name: impl Into<String>,
    item_count: usize,
    options: &SelectMenuOptions,
) -> UiNode {
    let scroll = item_count > options.max_visible_rows;
    let height =
        visible_row_count(item_count, options.max_visible_rows) as f32 * options.row_height;
    let mut node = UiNode::container(
        name,
        UiNodeStyle {
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: length(options.width.max(0.0)),
                    height: length(height.max(0.0)),
                },
                ..Default::default()
            },
            clip: ClipBehavior::Clip,
            z_index: options.z_index,
            ..Default::default()
        },
    )
    .with_visual(options.menu_visual);
    if scroll {
        node = node.with_scroll(ScrollAxes::VERTICAL);
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
        let visual = select_row_visual(index, option, state, menu_options);
        let text_style = if option.enabled {
            menu_options.text_style.clone()
        } else {
            menu_options.disabled_text_style.clone()
        };
        let row = document.add_child(
            parent,
            UiNode::container(
                format!("{name}.option.{index}"),
                row_style(menu_options.row_height),
            )
            .with_input(if option.enabled {
                InputBehavior::BUTTON
            } else {
                InputBehavior::NONE
            })
            .with_visual(visual),
        );
        label(
            document,
            row,
            format!("{name}.option.{index}.label"),
            &option.label,
            text_style,
            Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
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

fn menu_list_container_node(
    name: impl Into<String>,
    items: &[MenuItem],
    options: &MenuListOptions,
) -> UiNode {
    let scroll = menu_row_count_for_scroll(items) > options.max_visible_rows;
    let height = visible_menu_height(items, options);
    let mut node = UiNode::container(
        name,
        UiNodeStyle {
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: length(options.width.max(0.0)),
                    height: length(height.max(0.0)),
                },
                ..Default::default()
            },
            clip: ClipBehavior::Clip,
            z_index: options.z_index,
            ..Default::default()
        },
    )
    .with_visual(options.menu_visual);
    if scroll {
        node = node.with_scroll(ScrollAxes::VERTICAL);
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

        let visual = if item.enabled {
            if active == Some(index) {
                options.active_visual
            } else {
                options.item_visual
            }
        } else {
            options.disabled_visual
        };
        let text_style = if item.enabled {
            options.text_style.clone()
        } else {
            options.disabled_text_style.clone()
        };
        let row = document.add_child(
            parent,
            UiNode::container(
                format!("{name}.item.{index}"),
                row_style(options.row_height),
            )
            .with_input(if item.enabled {
                InputBehavior::BUTTON
            } else {
                InputBehavior::NONE
            })
            .with_visual(visual),
        );
        label(
            document,
            row,
            format!("{name}.item.{index}.label"),
            menu_item_label(item),
            text_style,
            Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
        );
        if let Some(shortcut) = &item.shortcut {
            label(
                document,
                row,
                format!("{name}.item.{index}.shortcut"),
                shortcut,
                options.shortcut_text_style.clone(),
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            );
        } else if item.children().is_some() {
            label(
                document,
                row,
                format!("{name}.item.{index}.submenu"),
                ">",
                options.shortcut_text_style.clone(),
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            );
        }
        rows.push(row);
    }
    rows
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
                layout: Style {
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
                },
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
                layout: Style {
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(1.0),
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(ColorRgba::new(77, 90, 111, 255), None, 0.0)),
    );
    row
}

fn menu_item_label(item: &MenuItem) -> String {
    match &item.kind {
        MenuItemKind::Check { checked: true } => format!("[x] {}", item.label),
        MenuItemKind::Check { checked: false } => format!("[ ] {}", item.label),
        MenuItemKind::Command | MenuItemKind::Submenu { .. } => item.label.clone(),
        MenuItemKind::Separator => String::new(),
    }
}

fn menu_row_count_for_scroll(items: &[MenuItem]) -> usize {
    items.len()
}

fn visible_menu_height(items: &[MenuItem], options: &MenuListOptions) -> f32 {
    let mut visible_rows = 0usize;
    let mut height = 0.0;
    for item in items {
        if visible_rows >= options.max_visible_rows {
            break;
        }
        height += if item.is_separator() {
            options.separator_height
        } else {
            options.row_height
        };
        visible_rows += 1;
    }
    height
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

fn row_style(height: f32) -> UiNodeStyle {
    UiNodeStyle {
        layout: Style {
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
        },
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

fn button_like(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    layout: Style,
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

fn button_like_with_input(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    layout: Style,
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
        Style {
            size: TaffySize {
                width: Dimension::auto(),
                height: Dimension::auto(),
            },
            ..Default::default()
        },
    );
    root
}

fn label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    text_style: TextStyle,
    layout: Style,
) -> UiNodeId {
    document.add_child(parent, UiNode::text(name, text, text_style, layout))
}

fn length_percentage(value: f32) -> taffy::prelude::LengthPercentage {
    taffy::prelude::LengthPercentage::length(value)
}

fn normalize(value: &str) -> String {
    value.to_lowercase()
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
    use crate::{root_style, ApproxTextMeasurer, KeyModifiers};

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
        assert_eq!(node.style.layout.position, Position::Absolute);
        assert_eq!(node.style.z_index, 100);
        assert!(node.scroll.is_some());
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
    fn command_palette_builder_creates_input_and_result_rows() {
        let mut document = UiDocument::new(root_style(600.0, 400.0));
        let root = document.root;
        let items = vec![
            CommandPaletteItem::new("open", "Open File"),
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
            CommandPaletteOptions::default(),
        );
        document
            .compute_layout(UiSize::new(600.0, 400.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(nodes.rows.len(), 2);
        assert!(document.node(nodes.input).input.focusable);
        assert!(document.node(nodes.root).layout.rect.width > 0.0);
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
                trigger_layout: Style {
                    size: TaffySize {
                        width: length(120.0),
                        height: length(30.0),
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        assert!(document.node(nodes.trigger).input.focusable);
        assert!(nodes.popup.is_none());
    }
}
