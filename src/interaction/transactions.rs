//! Reusable edit transaction, selection, and text edit history primitives.
//!
//! These helpers intentionally avoid product semantics. Applications provide
//! the transaction IDs, widget/node identity, command labels, item IDs, and
//! text snapshots that make sense for their domain.

use std::fmt;

use crate::commands::CommandMeta;
use crate::{KeyModifiers, UiNodeId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TransactionId(String);

impl TransactionId {
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

impl AsRef<str> for TransactionId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for TransactionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for TransactionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&TransactionId> for TransactionId {
    fn from(value: &TransactionId) -> Self {
        value.clone()
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TransactionWidgetId(String);

impl TransactionWidgetId {
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

impl AsRef<str> for TransactionWidgetId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for TransactionWidgetId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for TransactionWidgetId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&TransactionWidgetId> for TransactionWidgetId {
    fn from(value: &TransactionWidgetId) -> Self {
        value.clone()
    }
}

impl fmt::Display for TransactionWidgetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TransactionTarget {
    pub widget: Option<TransactionWidgetId>,
    pub node: Option<UiNodeId>,
}

impl TransactionTarget {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn widget(widget: impl Into<TransactionWidgetId>) -> Self {
        Self {
            widget: Some(widget.into()),
            node: None,
        }
    }

    pub const fn node(node: UiNodeId) -> Self {
        Self {
            widget: None,
            node: Some(node),
        }
    }

    pub fn with_widget(mut self, widget: impl Into<TransactionWidgetId>) -> Self {
        self.widget = Some(widget.into());
        self
    }

    pub const fn with_node(mut self, node: UiNodeId) -> Self {
        self.node = Some(node);
        self
    }

    pub const fn has_identity(&self) -> bool {
        self.widget.is_some() || self.node.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoRedoCommandDescriptors {
    pub undo: CommandMeta,
    pub redo: CommandMeta,
}

impl UndoRedoCommandDescriptors {
    pub fn new(undo: CommandMeta, redo: CommandMeta) -> Self {
        Self { undo, redo }
    }

    pub fn labels(
        undo_id: impl Into<crate::commands::CommandId>,
        undo_label: impl Into<String>,
        redo_id: impl Into<crate::commands::CommandId>,
        redo_label: impl Into<String>,
    ) -> Self {
        Self::new(
            CommandMeta::new(undo_id, undo_label),
            CommandMeta::new(redo_id, redo_label),
        )
    }
}

impl Default for UndoRedoCommandDescriptors {
    fn default() -> Self {
        Self::labels("edit.undo", "Undo", "edit.redo", "Redo")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditTransactionPhase {
    Preview,
    Update,
    Commit,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditTransaction<T> {
    pub id: TransactionId,
    pub phase: EditTransactionPhase,
    pub target: TransactionTarget,
    pub commands: UndoRedoCommandDescriptors,
    pub payload: T,
}

impl<T> EditTransaction<T> {
    pub fn new(phase: EditTransactionPhase, id: impl Into<TransactionId>, payload: T) -> Self {
        Self {
            id: id.into(),
            phase,
            target: TransactionTarget::default(),
            commands: UndoRedoCommandDescriptors::default(),
            payload,
        }
    }

    pub fn preview(id: impl Into<TransactionId>, payload: T) -> Self {
        Self::new(EditTransactionPhase::Preview, id, payload)
    }

    pub fn update(id: impl Into<TransactionId>, payload: T) -> Self {
        Self::new(EditTransactionPhase::Update, id, payload)
    }

    pub fn commit(id: impl Into<TransactionId>, payload: T) -> Self {
        Self::new(EditTransactionPhase::Commit, id, payload)
    }

    pub fn cancel(id: impl Into<TransactionId>, payload: T) -> Self {
        Self::new(EditTransactionPhase::Cancel, id, payload)
    }

    pub fn target(mut self, target: TransactionTarget) -> Self {
        self.target = target;
        self
    }

    pub fn commands(mut self, commands: UndoRedoCommandDescriptors) -> Self {
        self.commands = commands;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    ActiveTransactionExists {
        active: TransactionId,
        requested: TransactionId,
    },
    ActiveTransactionMismatch {
        active: TransactionId,
        requested: TransactionId,
    },
    NoActiveTransaction {
        requested: TransactionId,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SelectionMode {
    #[default]
    Single,
    Multi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectionMovement {
    Previous,
    Next,
    First,
    Last,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionModel<T> {
    mode: SelectionMode,
    selected: Vec<T>,
    active: Option<T>,
    anchor: Option<T>,
}

impl<T> SelectionModel<T> {
    pub fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            selected: Vec::new(),
            active: None,
            anchor: None,
        }
    }

    pub fn single() -> Self {
        Self::new(SelectionMode::Single)
    }

    pub fn multi() -> Self {
        Self::new(SelectionMode::Multi)
    }

    pub const fn mode(&self) -> SelectionMode {
        self.mode
    }

    pub fn selected_items(&self) -> &[T] {
        &self.selected
    }

    pub fn active(&self) -> Option<&T> {
        self.active.as_ref()
    }

    pub fn anchor(&self) -> Option<&T> {
        self.anchor.as_ref()
    }

    pub const fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }
}

impl<T: Clone + PartialEq> SelectionModel<T> {
    pub fn is_selected(&self, item: &T) -> bool {
        self.selected.iter().any(|selected| selected == item)
    }

    pub fn clear(&mut self) {
        self.selected.clear();
        self.active = None;
        self.anchor = None;
    }

    pub fn select_only(&mut self, item: T) {
        self.selected.clear();
        self.selected.push(item.clone());
        self.active = Some(item.clone());
        self.anchor = Some(item);
    }

    pub fn set_active(&mut self, item: Option<T>) {
        self.active = item;
    }

    pub fn toggle(&mut self, item: T) {
        self.active = Some(item.clone());
        self.anchor = Some(item.clone());

        if let Some(index) = self.selected.iter().position(|selected| selected == &item) {
            self.selected.remove(index);
            return;
        }

        if self.mode == SelectionMode::Single {
            self.selected.clear();
        }
        self.selected.push(item);
    }

    pub fn select_range(&mut self, active: T, ordered_items: &[T]) {
        let anchor = self
            .anchor
            .clone()
            .or_else(|| self.active.clone())
            .unwrap_or_else(|| active.clone());
        self.replace_with_range(anchor, active, ordered_items);
    }

    pub fn add_range(&mut self, active: T, ordered_items: &[T]) {
        if self.mode == SelectionMode::Single {
            self.select_only(active);
            return;
        }

        let anchor = self
            .anchor
            .clone()
            .or_else(|| self.active.clone())
            .unwrap_or_else(|| active.clone());
        for item in inclusive_range_items(&anchor, &active, ordered_items) {
            push_unique(&mut self.selected, item);
        }
        self.active = Some(active);
        self.anchor = Some(anchor);
    }

    pub fn apply_pointer_selection(
        &mut self,
        item: T,
        modifiers: KeyModifiers,
        ordered_items: &[T],
    ) {
        if modifiers.shift {
            if toggle_modifier(modifiers) {
                self.add_range(item, ordered_items);
            } else {
                self.select_range(item, ordered_items);
            }
        } else if toggle_modifier(modifiers) {
            self.toggle(item);
        } else {
            self.select_only(item);
        }
    }

    pub fn move_active(
        &mut self,
        movement: SelectionMovement,
        modifiers: KeyModifiers,
        ordered_items: &[T],
    ) -> Option<T> {
        let next = next_item(self.active.as_ref(), movement, ordered_items)?;

        if modifiers.shift {
            if toggle_modifier(modifiers) {
                self.add_range(next.clone(), ordered_items);
            } else {
                self.select_range(next.clone(), ordered_items);
            }
        } else if toggle_modifier(modifiers) {
            self.active = Some(next.clone());
            if self.anchor.is_none() {
                self.anchor = Some(next.clone());
            }
        } else {
            self.select_only(next.clone());
        }

        Some(next)
    }

    fn replace_with_range(&mut self, anchor: T, active: T, ordered_items: &[T]) {
        if self.mode == SelectionMode::Single {
            self.select_only(active);
            return;
        }

        self.selected = inclusive_range_items(&anchor, &active, ordered_items);
        self.active = Some(active);
        self.anchor = Some(anchor);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditChange {
    pub before: String,
    pub after: String,
}

impl TextEditChange {
    pub fn new(before: impl Into<String>, after: impl Into<String>) -> Self {
        Self {
            before: before.into(),
            after: after.into(),
        }
    }

    pub fn is_noop(&self) -> bool {
        self.before == self.after
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditTransaction {
    pub id: TransactionId,
    pub target: TransactionTarget,
    pub commands: UndoRedoCommandDescriptors,
    pub change: TextEditChange,
}

impl TextEditTransaction {
    pub fn new(
        id: impl Into<TransactionId>,
        before: impl Into<String>,
        after: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            target: TransactionTarget::default(),
            commands: UndoRedoCommandDescriptors::default(),
            change: TextEditChange::new(before, after),
        }
    }

    pub fn target(mut self, target: TransactionTarget) -> Self {
        self.target = target;
        self
    }

    pub fn commands(mut self, commands: UndoRedoCommandDescriptors) -> Self {
        self.commands = commands;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditRecord {
    pub id: TransactionId,
    pub target: TransactionTarget,
    pub commands: UndoRedoCommandDescriptors,
    pub change: TextEditChange,
}

impl From<TextEditTransaction> for TextEditRecord {
    fn from(value: TextEditTransaction) -> Self {
        Self {
            id: value.id,
            target: value.target,
            commands: value.commands,
            change: value.change,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextEditHistoryDirection {
    Undo,
    Redo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditHistoryApply {
    pub id: TransactionId,
    pub direction: TextEditHistoryDirection,
    pub target: TransactionTarget,
    pub commands: UndoRedoCommandDescriptors,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextEditHistory {
    undo: Vec<TextEditRecord>,
    redo: Vec<TextEditRecord>,
    active_preview: Option<TextEditTransaction>,
}

impl TextEditHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn active_preview(&self) -> Option<&TextEditTransaction> {
        self.active_preview.as_ref()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.active_preview = None;
    }

    pub fn record_committed(
        &mut self,
        edit: TextEditTransaction,
    ) -> Result<EditTransaction<TextEditChange>, TransactionError> {
        if let Some(active) = &self.active_preview {
            return Err(TransactionError::ActiveTransactionExists {
                active: active.id.clone(),
                requested: edit.id,
            });
        }

        Ok(self.push_committed(edit))
    }

    pub fn preview(
        &mut self,
        edit: TextEditTransaction,
    ) -> Result<EditTransaction<TextEditChange>, TransactionError> {
        if let Some(active) = &mut self.active_preview {
            if active.id != edit.id {
                return Err(TransactionError::ActiveTransactionExists {
                    active: active.id.clone(),
                    requested: edit.id,
                });
            }

            active.target = edit.target;
            active.commands = edit.commands;
            active.change.after = edit.change.after;
            return Ok(
                EditTransaction::update(active.id.clone(), active.change.clone())
                    .target(active.target.clone())
                    .commands(active.commands.clone()),
            );
        }

        let event = EditTransaction::preview(edit.id.clone(), edit.change.clone())
            .target(edit.target.clone())
            .commands(edit.commands.clone());
        self.active_preview = Some(edit);
        Ok(event)
    }

    pub fn commit_preview(
        &mut self,
        id: impl Into<TransactionId>,
        after: impl Into<String>,
    ) -> Result<EditTransaction<TextEditChange>, TransactionError> {
        let requested = id.into();
        match &self.active_preview {
            Some(active) if active.id != requested => {
                return Err(TransactionError::ActiveTransactionMismatch {
                    active: active.id.clone(),
                    requested,
                });
            }
            None => {
                return Err(TransactionError::NoActiveTransaction { requested });
            }
            Some(_) => {}
        }

        let mut active = self.active_preview.take().expect("active preview checked");
        active.change.after = after.into();
        Ok(self.push_committed(active))
    }

    pub fn cancel_preview(
        &mut self,
        id: impl Into<TransactionId>,
    ) -> Result<EditTransaction<TextEditChange>, TransactionError> {
        let requested = id.into();
        match &self.active_preview {
            Some(active) if active.id != requested => {
                return Err(TransactionError::ActiveTransactionMismatch {
                    active: active.id.clone(),
                    requested,
                });
            }
            None => {
                return Err(TransactionError::NoActiveTransaction { requested });
            }
            Some(_) => {}
        }

        let active = self.active_preview.take().expect("active preview checked");
        Ok(EditTransaction::cancel(active.id, active.change)
            .target(active.target)
            .commands(active.commands))
    }

    pub fn undo(&mut self) -> Option<TextEditHistoryApply> {
        let record = self.undo.pop()?;
        let apply = TextEditHistoryApply {
            id: record.id.clone(),
            direction: TextEditHistoryDirection::Undo,
            target: record.target.clone(),
            commands: record.commands.clone(),
            text: record.change.before.clone(),
        };
        self.redo.push(record);
        Some(apply)
    }

    pub fn redo(&mut self) -> Option<TextEditHistoryApply> {
        let record = self.redo.pop()?;
        let apply = TextEditHistoryApply {
            id: record.id.clone(),
            direction: TextEditHistoryDirection::Redo,
            target: record.target.clone(),
            commands: record.commands.clone(),
            text: record.change.after.clone(),
        };
        self.undo.push(record);
        Some(apply)
    }

    fn push_committed(&mut self, edit: TextEditTransaction) -> EditTransaction<TextEditChange> {
        let event = EditTransaction::commit(edit.id.clone(), edit.change.clone())
            .target(edit.target.clone())
            .commands(edit.commands.clone());

        if !edit.change.is_noop() {
            self.undo.push(TextEditRecord::from(edit));
            self.redo.clear();
        }

        event
    }
}

fn toggle_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.ctrl || modifiers.meta
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    if !items.iter().any(|existing| existing == &item) {
        items.push(item);
    }
}

fn inclusive_range_items<T: Clone + PartialEq>(
    anchor: &T,
    active: &T,
    ordered_items: &[T],
) -> Vec<T> {
    let Some(anchor_index) = ordered_items.iter().position(|item| item == anchor) else {
        return vec![active.clone()];
    };
    let Some(active_index) = ordered_items.iter().position(|item| item == active) else {
        return vec![active.clone()];
    };

    let start = anchor_index.min(active_index);
    let end = anchor_index.max(active_index);
    ordered_items[start..=end].to_vec()
}

fn next_item<T: Clone + PartialEq>(
    active: Option<&T>,
    movement: SelectionMovement,
    ordered_items: &[T],
) -> Option<T> {
    if ordered_items.is_empty() {
        return None;
    }

    let current_index = active
        .and_then(|item| ordered_items.iter().position(|candidate| candidate == item))
        .unwrap_or(0);
    let next_index = match movement {
        SelectionMovement::Previous => current_index.saturating_sub(1),
        SelectionMovement::Next => (current_index + 1).min(ordered_items.len() - 1),
        SelectionMovement::First => 0,
        SelectionMovement::Last => ordered_items.len() - 1,
    };

    ordered_items.get(next_index).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl() -> KeyModifiers {
        KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        }
    }

    fn meta() -> KeyModifiers {
        KeyModifiers {
            meta: true,
            ..KeyModifiers::NONE
        }
    }

    fn shift() -> KeyModifiers {
        KeyModifiers {
            shift: true,
            ..KeyModifiers::NONE
        }
    }

    #[test]
    fn shift_range_selection_extends_from_anchor() {
        let items = [1, 2, 3, 4, 5];
        let mut selection = SelectionModel::multi();

        selection.apply_pointer_selection(2, KeyModifiers::NONE, &items);
        selection.apply_pointer_selection(5, shift(), &items);

        assert_eq!(selection.selected_items(), &[2, 3, 4, 5]);
        assert_eq!(selection.active(), Some(&5));
        assert_eq!(selection.anchor(), Some(&2));
    }

    #[test]
    fn ctrl_and_cmd_toggle_selection_preserves_multi_selection() {
        let items = [1, 2, 3, 4];
        let mut selection = SelectionModel::multi();

        selection.apply_pointer_selection(1, KeyModifiers::NONE, &items);
        selection.apply_pointer_selection(3, ctrl(), &items);
        selection.apply_pointer_selection(1, meta(), &items);

        assert_eq!(selection.selected_items(), &[3]);
        assert_eq!(selection.active(), Some(&1));
        assert_eq!(selection.anchor(), Some(&1));
    }

    #[test]
    fn active_movement_can_move_focus_or_selection() {
        let items = [1, 2, 3, 4];
        let mut selection = SelectionModel::multi();

        selection.apply_pointer_selection(2, KeyModifiers::NONE, &items);
        selection.move_active(SelectionMovement::Next, ctrl(), &items);

        assert_eq!(selection.selected_items(), &[2]);
        assert_eq!(selection.active(), Some(&3));
        assert_eq!(selection.anchor(), Some(&2));

        selection.move_active(SelectionMovement::Next, shift(), &items);

        assert_eq!(selection.selected_items(), &[2, 3, 4]);
        assert_eq!(selection.active(), Some(&4));
        assert_eq!(selection.anchor(), Some(&2));
    }

    #[test]
    fn text_preview_commit_and_cancel_boundaries_are_explicit() {
        let mut history = TextEditHistory::new();

        let preview = history
            .preview(TextEditTransaction::new("drag-1", "a", "ab"))
            .unwrap();
        let update = history
            .preview(TextEditTransaction::new("drag-1", "ab", "abcd"))
            .unwrap();
        let commit = history.commit_preview("drag-1", "abcde").unwrap();

        assert_eq!(preview.phase, EditTransactionPhase::Preview);
        assert_eq!(update.phase, EditTransactionPhase::Update);
        assert_eq!(update.payload.before, "a");
        assert_eq!(update.payload.after, "abcd");
        assert_eq!(commit.phase, EditTransactionPhase::Commit);
        assert_eq!(commit.payload.before, "a");
        assert_eq!(commit.payload.after, "abcde");
        assert_eq!(history.undo_len(), 1);

        let cancel = history
            .preview(TextEditTransaction::new("drag-2", "abcde", "abcdef"))
            .and_then(|_| history.cancel_preview("drag-2"))
            .unwrap();

        assert_eq!(cancel.phase, EditTransactionPhase::Cancel);
        assert_eq!(cancel.payload.before, "abcde");
        assert_eq!(cancel.payload.after, "abcdef");
        assert_eq!(history.undo_len(), 1);
        assert!(history.active_preview().is_none());
    }

    #[test]
    fn undo_and_redo_apply_committed_text_changes() {
        let mut history = TextEditHistory::new();

        history
            .record_committed(TextEditTransaction::new("type-1", "hello", "hello!"))
            .unwrap();
        history
            .record_committed(TextEditTransaction::new("type-2", "hello!", "hello!!"))
            .unwrap();

        let undo = history.undo().unwrap();
        assert_eq!(undo.direction, TextEditHistoryDirection::Undo);
        assert_eq!(undo.text, "hello!");
        assert_eq!(history.undo_len(), 1);
        assert_eq!(history.redo_len(), 1);

        let redo = history.redo().unwrap();
        assert_eq!(redo.direction, TextEditHistoryDirection::Redo);
        assert_eq!(redo.text, "hello!!");
        assert_eq!(history.undo_len(), 2);
        assert_eq!(history.redo_len(), 0);
    }

    #[test]
    fn recording_new_text_change_clears_stale_redo() {
        let mut history = TextEditHistory::new();

        history
            .record_committed(TextEditTransaction::new("type-1", "a", "ab"))
            .unwrap();
        history
            .record_committed(TextEditTransaction::new("type-2", "ab", "abc"))
            .unwrap();
        assert_eq!(history.undo().unwrap().text, "ab");
        assert_eq!(history.redo_len(), 1);

        history
            .record_committed(TextEditTransaction::new("type-3", "ab", "abx"))
            .unwrap();

        assert_eq!(history.undo_len(), 2);
        assert_eq!(history.redo_len(), 0);
        assert!(history.redo().is_none());
    }
}
