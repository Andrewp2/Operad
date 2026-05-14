//! Menu bar widget implementations.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, Rect as TaffyRect, Size as TaffySize, Style,
};

use crate::{
    length, AccessibilityMeta, AccessibilityRole, ClipBehavior, ColorRgba, InputBehavior,
    LayoutStyle, TextStyle, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiRect, UiVisual,
    WidgetActionBinding,
};

use super::menu::{
    button_like_with_input, first_navigable_index, length_percentage, menu_selection_at_path,
    next_navigable_index, set_active_descendant, AnchoredPopup, MenuItem, MenuSelection,
    NavigationDirection, PopupAlign, PopupPlacement, PopupSide,
};
use super::menu_list::{menu_list_popup, MenuListNodes, MenuListOptions};

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

    pub fn set_active_item_by_id(&mut self, menus: &[MenuBarMenu], id: &str) -> Option<usize> {
        let menu_index = self.open_menu?;
        let menu = menus.get(menu_index)?;
        let item_index = menu
            .items
            .iter()
            .position(|item| item.id.as_deref() == Some(id) && item.enabled)?;
        self.active_item = Some(item_index);
        Some(item_index)
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
    pub action_prefix: Option<String>,
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
            action_prefix: None,
        }
    }
}

impl MenuBarOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
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
                layout: options.layout.style,
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
        if menu.enabled {
            if let Some(prefix) = &options.action_prefix {
                document.node_mut(button).action =
                    Some(WidgetActionBinding::action(format!("{prefix}.{}", menu.id)));
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_bar_state_sets_active_item_by_command_id() {
        let menus = vec![MenuBarMenu::new(
            "file",
            "File",
            vec![
                MenuItem::command("new", "New"),
                MenuItem::command("disabled", "Disabled").disabled(),
                MenuItem::command("open", "Open"),
            ],
        )];
        let mut state = MenuBarState {
            open_menu: Some(0),
            active_item: Some(0),
        };

        assert_eq!(state.set_active_item_by_id(&menus, "open"), Some(2));
        assert_eq!(state.active_item, Some(2));
        assert_eq!(state.set_active_item_by_id(&menus, "disabled"), None);
        assert_eq!(state.active_item, Some(2));
    }
}
