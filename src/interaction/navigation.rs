//! Backend-neutral keyboard navigation contracts for composite widgets.
//!
//! The types in this module describe navigation decisions before any renderer,
//! platform adapter, or accessibility backend is involved. Widgets can keep
//! DOM-style roving focus or aria-activedescendant-style state with the same
//! item registry and key handling rules.

use crate::{KeyCode, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NavigationItemId(pub u64);

impl NavigationItemId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationCollectionKind {
    Menu,
    Listbox,
    Toolbar,
    Table,
    Tree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationFocusModel {
    RovingFocus,
    ActiveDescendant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationOrientation {
    Horizontal,
    Vertical,
    Grid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationBoundaryBehavior {
    Clamp,
    Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    Previous,
    Next,
    First,
    Last,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationAction {
    Move {
        direction: NavigationDirection,
        from: Option<NavigationItemId>,
        to: NavigationItemId,
    },
    Activate(NavigationItemId),
    Dismiss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationKeyResult {
    pub consumed: bool,
    pub action: Option<NavigationAction>,
}

impl NavigationKeyResult {
    pub const fn ignored() -> Self {
        Self {
            consumed: false,
            action: None,
        }
    }

    pub const fn consumed(action: NavigationAction) -> Self {
        Self {
            consumed: true,
            action: Some(action),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationItem {
    pub id: NavigationItemId,
    pub disabled: bool,
}

impl NavigationItem {
    pub const fn enabled(id: NavigationItemId) -> Self {
        Self {
            id,
            disabled: false,
        }
    }

    pub const fn disabled(id: NavigationItemId) -> Self {
        Self { id, disabled: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationState {
    pub focused: Option<NavigationItemId>,
    pub active_descendant: Option<NavigationItemId>,
}

impl NavigationState {
    pub const fn roving(focused: Option<NavigationItemId>) -> Self {
        Self {
            focused,
            active_descendant: None,
        }
    }

    pub const fn active_descendant(active_descendant: Option<NavigationItemId>) -> Self {
        Self {
            focused: None,
            active_descendant,
        }
    }

    pub fn current(&self, focus_model: NavigationFocusModel) -> Option<NavigationItemId> {
        match focus_model {
            NavigationFocusModel::RovingFocus => self.focused,
            NavigationFocusModel::ActiveDescendant => self.active_descendant,
        }
    }

    pub fn set_current(
        &mut self,
        focus_model: NavigationFocusModel,
        item: Option<NavigationItemId>,
    ) {
        match focus_model {
            NavigationFocusModel::RovingFocus => self.focused = item,
            NavigationFocusModel::ActiveDescendant => self.active_descendant = item,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationContract {
    pub kind: NavigationCollectionKind,
    pub focus_model: NavigationFocusModel,
    pub orientation: NavigationOrientation,
    pub boundary: NavigationBoundaryBehavior,
    pub activate_on_enter: bool,
    pub activate_on_space: bool,
    pub dismiss_on_escape: bool,
}

impl NavigationContract {
    pub const fn new(
        kind: NavigationCollectionKind,
        focus_model: NavigationFocusModel,
        orientation: NavigationOrientation,
    ) -> Self {
        Self {
            kind,
            focus_model,
            orientation,
            boundary: NavigationBoundaryBehavior::Clamp,
            activate_on_enter: true,
            activate_on_space: true,
            dismiss_on_escape: true,
        }
    }

    pub const fn menu() -> Self {
        Self::new(
            NavigationCollectionKind::Menu,
            NavigationFocusModel::RovingFocus,
            NavigationOrientation::Vertical,
        )
    }

    pub const fn listbox_active_descendant() -> Self {
        Self::new(
            NavigationCollectionKind::Listbox,
            NavigationFocusModel::ActiveDescendant,
            NavigationOrientation::Vertical,
        )
    }

    pub const fn toolbar() -> Self {
        Self::new(
            NavigationCollectionKind::Toolbar,
            NavigationFocusModel::RovingFocus,
            NavigationOrientation::Horizontal,
        )
    }

    pub const fn table() -> Self {
        Self::new(
            NavigationCollectionKind::Table,
            NavigationFocusModel::ActiveDescendant,
            NavigationOrientation::Grid,
        )
    }

    pub const fn tree() -> Self {
        Self::new(
            NavigationCollectionKind::Tree,
            NavigationFocusModel::ActiveDescendant,
            NavigationOrientation::Vertical,
        )
    }

    pub const fn with_boundary(mut self, boundary: NavigationBoundaryBehavior) -> Self {
        self.boundary = boundary;
        self
    }

    pub const fn activate_on_enter(mut self, activate: bool) -> Self {
        self.activate_on_enter = activate;
        self
    }

    pub const fn activate_on_space(mut self, activate: bool) -> Self {
        self.activate_on_space = activate;
        self
    }

    pub const fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.dismiss_on_escape = dismiss;
        self
    }

    pub fn handle_key(
        &self,
        state: &mut NavigationState,
        items: &[NavigationItem],
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> NavigationKeyResult {
        if modifiers.ctrl || modifiers.alt || modifiers.meta {
            return NavigationKeyResult::ignored();
        }

        let Some(direction) = self.direction_for_key(key) else {
            if self.is_activation_key(key) {
                if let Some(current) = state.current(self.focus_model) {
                    if item_is_enabled(items, current) {
                        return NavigationKeyResult::consumed(NavigationAction::Activate(current));
                    }
                }
            }
            if key == KeyCode::Escape && self.dismiss_on_escape {
                return NavigationKeyResult::consumed(NavigationAction::Dismiss);
            }
            return NavigationKeyResult::ignored();
        };

        let from = state.current(self.focus_model);
        let Some(to) = next_enabled_item(items, from, direction, self.boundary) else {
            return NavigationKeyResult::ignored();
        };
        state.set_current(self.focus_model, Some(to));
        NavigationKeyResult::consumed(NavigationAction::Move {
            direction,
            from,
            to,
        })
    }

    fn direction_for_key(&self, key: KeyCode) -> Option<NavigationDirection> {
        match key {
            KeyCode::Home => Some(NavigationDirection::First),
            KeyCode::End => Some(NavigationDirection::Last),
            KeyCode::ArrowUp if self.accepts_vertical_arrows() => {
                Some(NavigationDirection::Previous)
            }
            KeyCode::ArrowDown if self.accepts_vertical_arrows() => Some(NavigationDirection::Next),
            KeyCode::ArrowLeft if self.accepts_horizontal_arrows() => {
                Some(NavigationDirection::Previous)
            }
            KeyCode::ArrowRight if self.accepts_horizontal_arrows() => {
                Some(NavigationDirection::Next)
            }
            _ => None,
        }
    }

    fn accepts_vertical_arrows(&self) -> bool {
        matches!(
            self.orientation,
            NavigationOrientation::Vertical | NavigationOrientation::Grid
        )
    }

    fn accepts_horizontal_arrows(&self) -> bool {
        matches!(
            self.orientation,
            NavigationOrientation::Horizontal | NavigationOrientation::Grid
        )
    }

    fn is_activation_key(&self, key: KeyCode) -> bool {
        (self.activate_on_enter && key == KeyCode::Enter)
            || (self.activate_on_space && key == KeyCode::Character(' '))
    }
}

pub fn next_enabled_item(
    items: &[NavigationItem],
    current: Option<NavigationItemId>,
    direction: NavigationDirection,
    boundary: NavigationBoundaryBehavior,
) -> Option<NavigationItemId> {
    if items.is_empty() {
        return None;
    }

    let enabled_count = items.iter().filter(|item| !item.disabled).count();
    if enabled_count == 0 {
        return None;
    }

    let current_index = current.and_then(|id| items.iter().position(|item| item.id == id));
    match direction {
        NavigationDirection::First => items.iter().find(|item| !item.disabled).map(|item| item.id),
        NavigationDirection::Last => items
            .iter()
            .rev()
            .find(|item| !item.disabled)
            .map(|item| item.id),
        NavigationDirection::Previous => step_enabled(items, current_index, -1, boundary),
        NavigationDirection::Next => step_enabled(items, current_index, 1, boundary),
    }
}

fn step_enabled(
    items: &[NavigationItem],
    current_index: Option<usize>,
    step: isize,
    boundary: NavigationBoundaryBehavior,
) -> Option<NavigationItemId> {
    let len = items.len() as isize;
    let mut index = match (current_index, step) {
        (Some(index), _) => index as isize + step,
        (None, 1) => 0,
        (None, -1) => len - 1,
        _ => return None,
    };

    for _ in 0..items.len() {
        if index < 0 || index >= len {
            match boundary {
                NavigationBoundaryBehavior::Clamp => return None,
                NavigationBoundaryBehavior::Wrap => index = if index < 0 { len - 1 } else { 0 },
            }
        }
        let item = items[index as usize];
        if !item.disabled {
            return Some(item.id);
        }
        index += step;
    }

    None
}

fn item_is_enabled(items: &[NavigationItem], id: NavigationItemId) -> bool {
    items.iter().any(|item| item.id == id && !item.disabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: u64) -> NavigationItemId {
        NavigationItemId::new(value)
    }

    fn items() -> Vec<NavigationItem> {
        vec![
            NavigationItem::enabled(id(1)),
            NavigationItem::disabled(id(2)),
            NavigationItem::enabled(id(3)),
            NavigationItem::enabled(id(4)),
        ]
    }

    #[test]
    fn navigation_roving_focus_moves_with_wrap() {
        let contract = NavigationContract::menu().with_boundary(NavigationBoundaryBehavior::Wrap);
        let mut state = NavigationState::roving(Some(id(4)));

        let result =
            contract.handle_key(&mut state, &items(), KeyCode::ArrowDown, KeyModifiers::NONE);

        assert_eq!(state.focused, Some(id(1)));
        assert_eq!(
            result.action,
            Some(NavigationAction::Move {
                direction: NavigationDirection::Next,
                from: Some(id(4)),
                to: id(1),
            })
        );
    }

    #[test]
    fn navigation_roving_focus_clamps_at_boundary() {
        let contract = NavigationContract::menu().with_boundary(NavigationBoundaryBehavior::Clamp);
        let mut state = NavigationState::roving(Some(id(4)));

        let result =
            contract.handle_key(&mut state, &items(), KeyCode::ArrowDown, KeyModifiers::NONE);

        assert_eq!(result, NavigationKeyResult::ignored());
        assert_eq!(state.focused, Some(id(4)));
    }

    #[test]
    fn navigation_active_descendant_updates_without_roving_focus() {
        let contract = NavigationContract::listbox_active_descendant();
        let mut state = NavigationState::active_descendant(Some(id(1)));

        let result =
            contract.handle_key(&mut state, &items(), KeyCode::ArrowDown, KeyModifiers::NONE);

        assert!(result.consumed);
        assert_eq!(state.focused, None);
        assert_eq!(state.active_descendant, Some(id(3)));
    }

    #[test]
    fn navigation_disabled_items_are_skipped() {
        let contract = NavigationContract::menu();
        let mut state = NavigationState::roving(Some(id(1)));

        contract.handle_key(&mut state, &items(), KeyCode::ArrowDown, KeyModifiers::NONE);

        assert_eq!(state.focused, Some(id(3)));
    }

    #[test]
    fn navigation_enter_and_space_activate_enabled_current_item() {
        let contract = NavigationContract::toolbar();
        let mut state = NavigationState::roving(Some(id(1)));

        let enter = contract.handle_key(&mut state, &items(), KeyCode::Enter, KeyModifiers::NONE);
        let space = contract.handle_key(
            &mut state,
            &items(),
            KeyCode::Character(' '),
            KeyModifiers::NONE,
        );

        assert_eq!(enter.action, Some(NavigationAction::Activate(id(1))));
        assert_eq!(space.action, Some(NavigationAction::Activate(id(1))));
    }

    #[test]
    fn navigation_activation_ignores_disabled_current_item() {
        let contract = NavigationContract::menu();
        let mut state = NavigationState::roving(Some(id(2)));

        let result = contract.handle_key(&mut state, &items(), KeyCode::Enter, KeyModifiers::NONE);

        assert_eq!(result, NavigationKeyResult::ignored());
    }

    #[test]
    fn navigation_escape_dismisses_contract() {
        let contract = NavigationContract::tree();
        let mut state = NavigationState::active_descendant(Some(id(1)));

        let result = contract.handle_key(&mut state, &items(), KeyCode::Escape, KeyModifiers::NONE);

        assert_eq!(result.action, Some(NavigationAction::Dismiss));
    }
}
