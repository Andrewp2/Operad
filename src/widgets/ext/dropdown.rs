//! Dropdown and select widget implementations.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, Rect as TaffyRect,
    Size as TaffySize, Style,
};

use crate::{
    length, AccessibilityMeta, AccessibilityRole, AnimationMachine, ClipBehavior, ClipScope,
    ColorRgba, ImageContent, InputBehavior, KeyCode, LayoutStyle, ScrollAxes, ShaderEffect,
    StrokeStyle, TextStyle, UiDocument, UiInputEvent, UiNode, UiNodeId, UiNodeStyle,
    UiPortalTarget, UiSize, UiVisual,
};

use super::menu::{
    button_like, first_typeahead_character, is_typeahead_character, label, leading_image,
    menu_accessibility_label, next_matching_index, normalize, normalized_character, place_popup,
    pop_last_char, popup_panel, row_style, set_active_descendant, visible_match_range,
    visible_row_count, AnchoredPopup, NavigationDirection, PopupOptions, SearchFieldState,
    SearchStatusText,
};

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
    open: bool,
    selected: Option<usize>,
    active: Option<usize>,
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

    pub fn with_open(mut self, options: &[SelectOption]) -> Self {
        self.open(options);
        self
    }

    pub fn with_active(mut self, options: &[SelectOption], active: usize) -> Self {
        self.activate_index(options, active);
        self
    }

    pub const fn is_open(&self) -> bool {
        self.open
    }

    pub const fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub const fn active_index(&self) -> Option<usize> {
        self.active
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

    pub fn activate_index(&mut self, options: &[SelectOption], index: usize) -> Option<usize> {
        let option = options.get(index)?;
        if !option.enabled {
            return None;
        }
        self.active = Some(index);
        Some(index)
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

    pub fn select_id(&mut self, options: &[SelectOption], id: &str) -> Option<SelectSelection> {
        let index = options
            .iter()
            .position(|option| option.enabled && option.id == id)?;
        self.selected = Some(index);
        self.active = Some(index);
        Some(SelectSelection {
            index,
            id: id.to_string(),
        })
    }

    pub fn select_id_and_close(
        &mut self,
        options: &[SelectOption],
        id: &str,
    ) -> Option<SelectSelection> {
        let selection = self.select_id(options, id)?;
        self.close();
        Some(selection)
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
    pub portal: UiPortalTarget,
    pub z_index: i16,
    pub action_prefix: Option<String>,
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
            portal: UiPortalTarget::AppOverlay,
            z_index: 100,
            action_prefix: None,
        }
    }
}

impl SelectMenuOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
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
            clip_scope: match menu_options.portal {
                UiPortalTarget::Parent => ClipScope::Parent,
                _ => ClipScope::Viewport,
            },
            portal: menu_options.portal.clone(),
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
    {
        let layout = &mut document.node_mut(root).style.layout;
        layout.display = Display::Flex;
        layout.flex_direction = FlexDirection::Column;
    }
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
            })
            .style,
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
    if option.enabled {
        if let Some(prefix) = options.action_prefix.as_deref() {
            node = node.with_action(format!("{prefix}.option.{}", option.id));
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

fn option_accessibility_label(option: &SelectOption) -> String {
    option
        .accessibility_label
        .clone()
        .unwrap_or_else(|| option.label.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::ext::PopupPlacement;
    use crate::{root_style, ApproxTextMeasurer, UiRect};

    #[test]
    fn select_menu_state_selects_enabled_option_ids() {
        let options = vec![
            SelectOption::new("compact", "Compact"),
            SelectOption::new("disabled", "Disabled").disabled(),
            SelectOption::new("spacious", "Spacious"),
        ];
        let mut state = SelectMenuState::default();

        assert_eq!(
            state.select_id(&options, "spacious"),
            Some(SelectSelection {
                index: 2,
                id: "spacious".to_string(),
            })
        );
        assert_eq!(state.selected, Some(2));
        assert_eq!(state.active, Some(2));

        state.open = true;
        assert_eq!(state.select_id_and_close(&options, "disabled"), None);
        assert!(state.open);
        assert_eq!(
            state.select_id_and_close(&options, "compact"),
            Some(SelectSelection {
                index: 0,
                id: "compact".to_string(),
            })
        );
        assert!(!state.open);
    }

    #[test]
    fn select_menu_popup_stacks_rows_to_option_count() {
        let mut document = UiDocument::new(root_style(360.0, 320.0));
        let options = vec![
            SelectOption::new("en-US", "English"),
            SelectOption::new("es-MX", "Español"),
            SelectOption::new("fr-FR", "Français"),
            SelectOption::new("de-DE", "Deutsch"),
        ];
        let state = SelectMenuState {
            open: true,
            selected: Some(1),
            active: Some(1),
        };
        let menu_options = SelectMenuOptions {
            width: 148.0,
            row_height: 30.0,
            max_visible_rows: options.len(),
            ..Default::default()
        };

        let root = document.root;
        let nodes = select_menu_popup(
            &mut document,
            root,
            "locale.popup",
            AnchoredPopup::new(
                UiRect::new(12.0, 12.0, 148.0, 30.0),
                UiRect::new(0.0, 0.0, 360.0, 320.0),
                PopupPlacement::default(),
            ),
            &options,
            &state,
            menu_options,
        );
        document
            .compute_layout(UiSize::new(360.0, 320.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let popup = document.node(nodes.root);
        assert_eq!(popup.style.layout.display, Display::Flex);
        assert_eq!(popup.style.layout.flex_direction, FlexDirection::Column);
        assert!((popup.layout.rect.height - 120.0).abs() < 0.01);

        assert_eq!(nodes.rows.len(), options.len());
        let first_y = document.node(nodes.rows[0]).layout.rect.y;
        let first_x = document.node(nodes.rows[0]).layout.rect.x;
        for (index, row) in nodes.rows.iter().enumerate() {
            let rect = document.node(*row).layout.rect;
            assert!((rect.x - first_x).abs() < 0.01);
            assert!((rect.y - (first_y + index as f32 * 30.0)).abs() < 0.01);
            assert!((rect.height - 30.0).abs() < 0.01);
            assert!((rect.width - 148.0).abs() < 0.01);
        }
    }
}
