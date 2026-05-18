//! Context menu widget implementations.

use crate::tooltips::{
    resolve_context_menu_request, ContextMenuRequest, ContextMenuResolution, HelpItemState,
};
use crate::{
    input::RawPointerEvent, KeyCode, KeyModifiers, UiDocument, UiInputEvent, UiNodeId, UiPoint,
    UiRect,
};

use super::menu::{
    first_navigable_index, first_typeahead_character, is_typeahead_character, last_navigable_index,
    menu_selection_at_path, next_menu_typeahead_index, next_navigable_index, AnchoredPopup,
    MenuCommandSelection, MenuItem, MenuSelection, NavigationDirection, PopupPlacement,
};
use super::menu_list::{menu_list_popup, MenuListNodes, MenuListOptions};

#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenuState {
    pub open: bool,
    pub anchor: UiPoint,
    pub active: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenuOpenOutcome {
    pub menu: MenuOutcome,
    pub resolution: ContextMenuResolution,
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

    pub fn open_from_request(
        &mut self,
        request: ContextMenuRequest,
        items: &[MenuItem],
    ) -> ContextMenuOpenOutcome {
        let resolution = resolve_context_menu_request(request);
        let mut menu = MenuOutcome::default();
        if let Some(request) = resolution.request {
            self.open_with_items(request.position, items);
            menu.opened = true;
            menu.active = self.active;
        }
        ContextMenuOpenOutcome { menu, resolution }
    }

    pub fn open_from_pointer_event(
        &mut self,
        target: UiNodeId,
        anchor_rect: UiRect,
        event: RawPointerEvent,
        item_state: HelpItemState,
        items: &[MenuItem],
    ) -> ContextMenuOpenOutcome {
        let Some(request) = ContextMenuRequest::from_pointer_event(target, anchor_rect, event)
            .map(|request| request.item_state(item_state))
        else {
            return ContextMenuOpenOutcome {
                menu: MenuOutcome::default(),
                resolution: ContextMenuResolution {
                    request: None,
                    suppressed_reason: None,
                },
            };
        };
        self.open_from_request(request, items)
    }

    pub fn open_from_keyboard(
        &mut self,
        target: UiNodeId,
        anchor_rect: UiRect,
        item_state: HelpItemState,
        items: &[MenuItem],
    ) -> ContextMenuOpenOutcome {
        self.open_from_request(
            ContextMenuRequest::keyboard(target, anchor_rect).item_state(item_state),
            items,
        )
    }

    pub fn open_from_key_event(
        &mut self,
        target: UiNodeId,
        anchor_rect: UiRect,
        key: KeyCode,
        modifiers: KeyModifiers,
        item_state: HelpItemState,
        items: &[MenuItem],
    ) -> ContextMenuOpenOutcome {
        let Some(request) = ContextMenuRequest::from_key_event(target, anchor_rect, key, modifiers)
            .map(|request| request.item_state(item_state))
        else {
            return ContextMenuOpenOutcome {
                menu: MenuOutcome::default(),
                resolution: ContextMenuResolution {
                    request: None,
                    suppressed_reason: None,
                },
            };
        };
        self.open_from_request(request, items)
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
