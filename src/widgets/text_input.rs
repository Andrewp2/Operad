use super::*;

use crate::host::text_input_id_for_node;
use crate::transactions::{
    EditTransaction, TextEditChange, TextEditHistory, TextEditHistoryApply, TextEditTransaction,
    TransactionId, TransactionTarget,
};
#[cfg(feature = "text-cosmic")]
use cosmic_text::{
    fontdb, Attrs, Buffer, Family as CosmicFamily, FontSystem, Metrics, Shaping,
    Stretch as CosmicStretch, Style as CosmicFontStyle, Weight as CosmicWeight, Wrap as CosmicWrap,
};
#[cfg(feature = "text-cosmic")]
use std::{cell::RefCell, sync::Arc};

const TEXT_INPUT_CONTENT_INSET_X: f32 = 6.0;
const TEXT_INPUT_CONTENT_INSET_Y: f32 = 6.0;
const TEXT_INPUT_APPROX_CHAR_WIDTH_FACTOR: f32 = 0.50;

#[cfg(feature = "text-cosmic")]
thread_local! {
    static TEXT_INPUT_FONT_SYSTEM: RefCell<FontSystem> =
        RefCell::new(text_input_cosmic_font_system());
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInputState {
    pub(crate) text: String,
    pub(crate) caret: usize,
    pub(crate) selection_anchor: Option<usize>,
    pub(crate) multiline: bool,
    pub(crate) composing: Option<String>,
    pub(crate) history: TextEditHistory,
    pub(crate) history_sequence: u64,
}

impl TextInputState {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            caret: text.len(),
            text,
            selection_anchor: None,
            multiline: false,
            composing: None,
            history: TextEditHistory::new(),
            history_sequence: 0,
        }
    }

    pub fn multiline(mut self, multiline: bool) -> Self {
        self.set_multiline(multiline);
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = filter_text_input(&text.into(), self.multiline);
        self.caret = self.text.len();
        self.selection_anchor = None;
        self.composing = None;
    }

    pub const fn is_multiline(&self) -> bool {
        self.multiline
    }

    pub fn set_multiline(&mut self, multiline: bool) {
        self.multiline = multiline;
        if !self.multiline {
            self.text = filter_text_input(&self.text, false);
        }
        self.normalize_selection();
        self.composing = self
            .composing
            .take()
            .map(|text| filter_text_input(&text, self.multiline))
            .filter(|text| !text.is_empty());
    }

    pub fn caret(&self) -> usize {
        self.normalized_caret()
    }

    pub fn set_caret(&mut self, caret: usize) {
        self.caret = clamp_to_char_boundary(&self.text, caret);
        self.selection_anchor = None;
    }

    pub fn selection_anchor(&self) -> Option<usize> {
        self.selection_anchor
            .map(|anchor| clamp_to_char_boundary(&self.text, anchor))
    }

    pub fn set_selection(&mut self, anchor: usize, caret: usize) {
        self.selection_anchor = Some(clamp_to_char_boundary(&self.text, anchor));
        self.caret = clamp_to_char_boundary(&self.text, caret);
    }

    pub fn composing(&self) -> Option<&str> {
        self.composing.as_deref()
    }

    pub fn set_composing(&mut self, composing: Option<String>) {
        self.composing = composing
            .map(|text| filter_text_input(&text, self.multiline))
            .filter(|text| !text.is_empty());
    }

    pub fn history(&self) -> &TextEditHistory {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history = TextEditHistory::new();
        self.history_sequence = 0;
    }

    pub fn selected_range(&self) -> Option<Range<usize>> {
        let anchor = clamp_to_char_boundary(&self.text, self.selection_anchor?);
        let caret = clamp_to_char_boundary(&self.text, self.caret);
        if anchor == caret {
            return None;
        }
        Some(anchor.min(caret)..anchor.max(caret))
    }

    pub fn selected_text(&self) -> Option<&str> {
        self.selected_range().map(|range| &self.text[range])
    }

    pub fn caret_position(&self) -> TextInputPosition {
        text_position_at(&self.text, self.caret)
    }

    pub fn caret_line_range(&self) -> Range<usize> {
        line_range_at(&self.text, self.caret)
    }

    pub fn caret_info(&self) -> TextInputCaretInfo {
        TextInputCaretInfo {
            position: self.caret_position(),
            line_range: self.caret_line_range(),
            selected_range: self.selected_range(),
        }
    }

    pub fn caret_rect(&self, metrics: TextInputLayoutMetrics) -> TextInputCaretRect {
        text_input_caret_rect(&self.text, self.caret, metrics)
    }

    pub fn normalized_caret(&self) -> usize {
        clamp_to_char_boundary(&self.text, self.caret)
    }

    pub fn position_at_point(
        &self,
        metrics: TextInputLayoutMetrics,
        point: UiPoint,
    ) -> TextInputPosition {
        text_position_at(
            &self.text,
            text_input_byte_index_at_point(&self.text, self.multiline, metrics, point),
        )
    }

    pub fn byte_index_at_point(&self, metrics: TextInputLayoutMetrics, point: UiPoint) -> usize {
        text_input_byte_index_at_point(&self.text, self.multiline, metrics, point)
    }

    pub fn move_caret_to_point(
        &mut self,
        metrics: TextInputLayoutMetrics,
        point: UiPoint,
        selecting: bool,
    ) {
        self.normalize_selection();
        let anchor = self.selection_anchor.unwrap_or(self.caret);
        self.caret = self.byte_index_at_point(metrics, point);
        self.selection_anchor = selecting.then_some(anchor);
    }

    pub fn selection_rects(&self, metrics: TextInputLayoutMetrics) -> Vec<TextInputSelectionRect> {
        text_input_selection_rects(&self.text, self.selected_range(), metrics)
    }

    pub fn render_plan(
        &self,
        metrics: TextInputLayoutMetrics,
        text_style: TextStyle,
        paint: TextInputPaintOptions,
    ) -> TextInputRenderPlan {
        TextInputRenderPlan::new(self, metrics, text_style, paint)
    }

    pub fn ime_session(&self, context: TextInputPlatformContext) -> TextImeSession {
        TextImeSession::new(context.input, context.cursor_rect)
            .surrounding_text(self.text.clone(), self.platform_selection_range())
            .multiline(self.multiline)
    }

    pub fn activate_ime_request(&self, context: TextInputPlatformContext) -> TextImeRequest {
        TextImeRequest::Activate(self.ime_session(context))
    }

    pub fn update_ime_request(&self, context: TextInputPlatformContext) -> TextImeRequest {
        TextImeRequest::Update(self.ime_session(context))
    }

    pub fn deactivate_ime_request(input: TextInputId) -> TextImeRequest {
        TextImeRequest::Deactivate { input }
    }

    pub fn show_keyboard_request(input: TextInputId) -> TextImeRequest {
        TextImeRequest::ShowKeyboard { input }
    }

    pub fn hide_keyboard_request(input: TextInputId) -> TextImeRequest {
        TextImeRequest::HideKeyboard { input }
    }

    pub fn apply_ime_response(&mut self, response: &TextImeResponse) -> TextInputOutcome {
        self.apply_ime_response_for_target(response, TransactionTarget::none())
    }

    pub fn apply_ime_response_with_policy(
        &mut self,
        response: &TextImeResponse,
        policy: TextInputInteractionPolicy,
    ) -> TextInputOutcome {
        self.apply_ime_response_for_target_with_policy(response, TransactionTarget::none(), policy)
    }

    pub fn apply_ime_response_for_target(
        &mut self,
        response: &TextImeResponse,
        target: TransactionTarget,
    ) -> TextInputOutcome {
        self.apply_ime_response_for_target_with_policy(
            response,
            target,
            TextInputInteractionPolicy::default(),
        )
    }

    pub fn apply_ime_response_for_target_with_policy(
        &mut self,
        response: &TextImeResponse,
        target: TransactionTarget,
        policy: TextInputInteractionPolicy,
    ) -> TextInputOutcome {
        if !policy.can_edit() {
            if matches!(response, TextImeResponse::Deactivated { .. }) {
                self.composing = None;
            }
            return TextInputOutcome::new(EditPhase::Preview, false, None);
        }
        let before = self.text.clone();
        let mut phase = EditPhase::Preview;
        match response {
            TextImeResponse::Commit { text, .. } => {
                self.composing = None;
                self.insert_text(text);
                phase = EditPhase::UpdateEdit;
            }
            TextImeResponse::Preedit { text, .. } => {
                self.composing = (!text.is_empty()).then_some(text.clone());
            }
            TextImeResponse::DeleteSurrounding {
                before_chars,
                after_chars,
                ..
            } => {
                if self.delete_surrounding_chars(*before_chars, *after_chars) {
                    phase = EditPhase::UpdateEdit;
                }
            }
            TextImeResponse::Deactivated { .. } => {
                self.composing = None;
            }
            TextImeResponse::Activated { .. }
            | TextImeResponse::Unsupported
            | TextImeResponse::Error(_) => {}
        }
        let changed = before != self.text;
        let transaction = (phase == EditPhase::UpdateEdit)
            .then(|| self.record_text_history(before, target))
            .flatten();
        TextInputOutcome::new(phase, changed, None).with_transaction(transaction)
    }

    pub fn apply_ime_response_for_input(
        &mut self,
        input: &TextInputId,
        response: &TextImeResponse,
    ) -> Option<TextInputOutcome> {
        response
            .is_for_input(input)
            .then(|| self.apply_ime_response(response))
    }

    pub fn apply_ime_response_for_input_with_policy(
        &mut self,
        input: &TextInputId,
        response: &TextImeResponse,
        policy: TextInputInteractionPolicy,
    ) -> Option<TextInputOutcome> {
        response
            .is_for_input(input)
            .then(|| self.apply_ime_response_with_policy(response, policy))
    }

    pub fn apply_ime_response_for_input_and_target(
        &mut self,
        input: &TextInputId,
        response: &TextImeResponse,
        target: TransactionTarget,
    ) -> Option<TextInputOutcome> {
        response
            .is_for_input(input)
            .then(|| self.apply_ime_response_for_target(response, target))
    }

    pub fn apply_ime_response_for_input_and_target_with_policy(
        &mut self,
        input: &TextInputId,
        response: &TextImeResponse,
        target: TransactionTarget,
        policy: TextInputInteractionPolicy,
    ) -> Option<TextInputOutcome> {
        response
            .is_for_input(input)
            .then(|| self.apply_ime_response_for_target_with_policy(response, target, policy))
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.caret = self.text.len();
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    pub fn insert_text(&mut self, text: &str) {
        let filtered = filter_text_input(text, self.multiline);
        self.replace_selection(&filtered);
    }

    pub fn copy_selection(&self) -> Option<String> {
        self.selected_range()
            .map(|range| self.text[range].to_string())
    }

    pub fn cut_selection(&mut self) -> Option<String> {
        let copied = self.copy_selection()?;
        self.replace_selection("");
        Some(copied)
    }

    pub fn paste_text(&mut self, text: &str) {
        let filtered = filter_text_input(text, self.multiline);
        self.replace_selection(&filtered);
    }

    pub fn paste_text_with_outcome(&mut self, text: &str) -> TextInputOutcome {
        self.paste_text_with_outcome_for_target(text, TransactionTarget::none())
    }

    pub fn paste_text_with_outcome_for_target(
        &mut self,
        text: &str,
        target: TransactionTarget,
    ) -> TextInputOutcome {
        let before = self.text.clone();
        self.paste_text(text);
        let transaction = self.record_text_history(before.clone(), target);
        TextInputOutcome::new(EditPhase::UpdateEdit, before != self.text, None)
            .with_transaction(transaction)
    }

    pub fn replace_selection(&mut self, text: &str) {
        self.normalize_selection();
        if let Some(range) = self.selected_range() {
            self.text.replace_range(range.clone(), text);
            self.caret = range.start + text.len();
        } else {
            self.text.insert_str(self.caret, text);
            self.caret += text.len();
        }
        self.caret = clamp_to_char_boundary(&self.text, self.caret);
        self.selection_anchor = None;
    }

    pub fn backspace(&mut self) -> bool {
        self.normalize_selection();
        if self.selected_range().is_some() {
            self.replace_selection("");
            return true;
        }
        if self.caret == 0 {
            return false;
        }
        let previous = previous_char_boundary(&self.text, self.caret);
        self.text.replace_range(previous..self.caret, "");
        self.caret = previous;
        true
    }

    pub fn delete(&mut self) -> bool {
        self.normalize_selection();
        if self.selected_range().is_some() {
            self.replace_selection("");
            return true;
        }
        if self.caret >= self.text.len() {
            return false;
        }
        let next = next_char_boundary(&self.text, self.caret);
        self.text.replace_range(self.caret..next, "");
        true
    }

    pub fn move_caret(&mut self, movement: CaretMovement, selecting: bool) {
        self.normalize_selection();
        let anchor = self.selection_anchor.unwrap_or(self.caret);
        self.caret = match movement {
            CaretMovement::Start => 0,
            CaretMovement::End => self.text.len(),
            CaretMovement::LineStart => line_range_at(&self.text, self.caret).start,
            CaretMovement::LineEnd => line_range_at(&self.text, self.caret).end,
            CaretMovement::Left => previous_char_boundary(&self.text, self.caret),
            CaretMovement::Right => next_char_boundary(&self.text, self.caret),
            CaretMovement::Up => move_caret_vertically(&self.text, self.caret, -1),
            CaretMovement::Down => move_caret_vertically(&self.text, self.caret, 1),
        };
        self.caret = clamp_to_char_boundary(&self.text, self.caret);
        self.selection_anchor = selecting.then_some(anchor);
    }

    pub fn handle_event(&mut self, event: &UiInputEvent) -> TextInputOutcome {
        self.handle_event_for_target(event, TransactionTarget::none())
    }

    pub fn apply_widget_text_edit(
        &mut self,
        edit: &WidgetTextEdit,
        options: &TextInputOptions,
    ) -> TextInputOutcome {
        let policy = options.interaction_policy();
        if !policy.enabled {
            return TextInputOutcome::new(EditPhase::Preview, false, None);
        }

        if let Some(point) = edit.local_position {
            if !policy.can_move_caret() {
                if !policy.selectable {
                    self.clear_selection();
                }
                return TextInputOutcome::new(EditPhase::Preview, false, None);
            }
            let target_rect = edit
                .target_rect
                .unwrap_or_else(|| UiRect::new(0.0, 0.0, 180.0, 30.0));
            let local_rect = UiRect::new(
                0.0,
                0.0,
                target_rect.width.max(1.0),
                target_rect.height.max(1.0),
            );
            let metrics = TextInputLayoutMetrics::from_style(
                text_input_content_rect(local_rect, &options.text_style),
                &options.text_style,
            );
            self.normalize_selection();
            let anchor = self.selection_anchor.unwrap_or(self.caret);
            let measured = TextInputMeasuredLayout::measure(&self.text, &options.text_style);
            self.caret = text_input_byte_index_at_point_with_layout(
                &self.text,
                self.multiline,
                metrics,
                point,
                measured.as_ref(),
            );
            self.selection_anchor = (edit.selecting && policy.can_select()).then_some(anchor);
            return TextInputOutcome::new(edit.phase.edit_phase(), false, None);
        }

        self.handle_event_with_policy(&edit.event, policy)
    }

    pub fn handle_event_with_policy(
        &mut self,
        event: &UiInputEvent,
        policy: TextInputInteractionPolicy,
    ) -> TextInputOutcome {
        self.handle_event_for_target_with_policy(event, TransactionTarget::none(), policy)
    }

    pub fn handle_event_for_target(
        &mut self,
        event: &UiInputEvent,
        target: TransactionTarget,
    ) -> TextInputOutcome {
        self.handle_event_for_target_with_policy(
            event,
            target,
            TextInputInteractionPolicy::default(),
        )
    }

    pub fn handle_event_for_target_with_policy(
        &mut self,
        event: &UiInputEvent,
        target: TransactionTarget,
        policy: TextInputInteractionPolicy,
    ) -> TextInputOutcome {
        if !policy.enabled {
            return TextInputOutcome::new(EditPhase::Preview, false, None);
        }
        if !policy.selectable {
            self.clear_selection();
        }
        let before = self.text.clone();
        let mut phase = EditPhase::Preview;
        let mut clipboard = None;
        let mut history_apply = None;
        match event {
            UiInputEvent::TextInput(text) if policy.can_edit() => {
                self.insert_text(text);
                phase = EditPhase::UpdateEdit;
            }
            UiInputEvent::Key { key, modifiers } => match key {
                KeyCode::Character(character) if modifiers.ctrl || modifiers.meta => {
                    match character.to_ascii_lowercase() {
                        'a' if policy.can_select() => self.select_all(),
                        'c' => {
                            if policy.can_copy() {
                                clipboard =
                                    self.copy_selection().map(TextInputClipboardAction::Copy);
                            }
                        }
                        'x' if policy.can_edit() => {
                            if policy.can_copy() {
                                clipboard = self.cut_selection().map(TextInputClipboardAction::Cut);
                            }
                            if clipboard.is_some() {
                                phase = EditPhase::UpdateEdit;
                            }
                        }
                        'v' if policy.can_edit() => {
                            clipboard = Some(TextInputClipboardAction::Paste);
                        }
                        'y' if policy.can_edit() => {
                            history_apply = self.redo_text_edit();
                            if history_apply.is_some() {
                                phase = EditPhase::UpdateEdit;
                            }
                        }
                        'z' if modifiers.shift && policy.can_edit() => {
                            history_apply = self.redo_text_edit();
                            if history_apply.is_some() {
                                phase = EditPhase::UpdateEdit;
                            }
                        }
                        'z' if policy.can_edit() => {
                            history_apply = self.undo_text_edit();
                            if history_apply.is_some() {
                                phase = EditPhase::UpdateEdit;
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Backspace if policy.can_edit() => {
                    if self.backspace() {
                        phase = EditPhase::UpdateEdit;
                    }
                }
                KeyCode::Delete if policy.can_edit() => {
                    if self.delete() {
                        phase = EditPhase::UpdateEdit;
                    }
                }
                KeyCode::ArrowLeft if policy.can_move_caret() => {
                    self.move_caret(CaretMovement::Left, modifiers.shift && policy.can_select());
                }
                KeyCode::ArrowRight if policy.can_move_caret() => {
                    self.move_caret(CaretMovement::Right, modifiers.shift && policy.can_select());
                }
                KeyCode::ArrowUp if self.multiline && policy.can_move_caret() => {
                    self.move_caret(CaretMovement::Up, modifiers.shift && policy.can_select());
                }
                KeyCode::ArrowDown if self.multiline && policy.can_move_caret() => {
                    self.move_caret(CaretMovement::Down, modifiers.shift && policy.can_select());
                }
                KeyCode::Home if policy.can_move_caret() => {
                    let movement = if self.multiline {
                        CaretMovement::LineStart
                    } else {
                        CaretMovement::Start
                    };
                    self.move_caret(movement, modifiers.shift && policy.can_select());
                }
                KeyCode::End if policy.can_move_caret() => {
                    let movement = if self.multiline {
                        CaretMovement::LineEnd
                    } else {
                        CaretMovement::End
                    };
                    self.move_caret(movement, modifiers.shift && policy.can_select());
                }
                KeyCode::Enter if self.multiline && policy.can_edit() => {
                    self.insert_text("\n");
                    phase = EditPhase::UpdateEdit;
                }
                KeyCode::Enter => phase = EditPhase::CommitEdit,
                KeyCode::Escape => phase = EditPhase::CancelEdit,
                _ => {}
            },
            _ => {}
        }
        let transaction = (phase == EditPhase::UpdateEdit && history_apply.is_none())
            .then(|| self.record_text_history(before.clone(), target))
            .flatten();
        TextInputOutcome::new(phase, before != self.text, clipboard)
            .with_transaction(transaction)
            .with_history_apply(history_apply)
    }

    pub fn undo_text_edit(&mut self) -> Option<TextEditHistoryApply> {
        let apply = self.history.undo()?;
        self.apply_history_text(apply.text.clone());
        Some(apply)
    }

    pub fn redo_text_edit(&mut self) -> Option<TextEditHistoryApply> {
        let apply = self.history.redo()?;
        self.apply_history_text(apply.text.clone());
        Some(apply)
    }

    fn platform_selection_range(&self) -> TextRange {
        self.selected_range()
            .map(|range| TextRange::new(range.start, range.end))
            .unwrap_or_else(|| TextRange::caret(clamp_to_char_boundary(&self.text, self.caret)))
    }

    fn delete_surrounding_chars(&mut self, before_chars: usize, after_chars: usize) -> bool {
        self.normalize_selection();
        let mut start = self.caret;
        for _ in 0..before_chars {
            if start == 0 {
                break;
            }
            start = previous_char_boundary(&self.text, start);
        }
        let mut end = self.caret;
        for _ in 0..after_chars {
            if end >= self.text.len() {
                break;
            }
            end = next_char_boundary(&self.text, end);
        }
        if start == end {
            return false;
        }
        self.text.replace_range(start..end, "");
        self.caret = start;
        self.selection_anchor = None;
        true
    }

    fn normalize_selection(&mut self) {
        self.caret = clamp_to_char_boundary(&self.text, self.caret);
        self.selection_anchor = self
            .selection_anchor
            .map(|anchor| clamp_to_char_boundary(&self.text, anchor));
    }

    fn apply_history_text(&mut self, text: String) {
        self.text = text;
        self.caret = self.text.len();
        self.selection_anchor = None;
        self.composing = None;
    }

    fn record_text_history(
        &mut self,
        before: String,
        target: TransactionTarget,
    ) -> Option<EditTransaction<TextEditChange>> {
        if before == self.text {
            return None;
        }
        let id = TransactionId::new(format!("text-input:{}", self.history_sequence));
        self.history_sequence = self.history_sequence.wrapping_add(1);
        self.history
            .record_committed(
                TextEditTransaction::new(id, before, self.text.clone()).target(target),
            )
            .ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretMovement {
    Start,
    End,
    LineStart,
    LineEnd,
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextInputPosition {
    pub byte_index: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInputCaretInfo {
    pub position: TextInputPosition,
    pub line_range: Range<usize>,
    pub selected_range: Option<Range<usize>>,
}

impl TextInputCaretInfo {
    pub fn accessibility_summary(&self, title: impl Into<String>) -> AccessibilitySummary {
        let mut summary = AccessibilitySummary::new(title)
            .item("Line", (self.position.line + 1).to_string())
            .item("Column", (self.position.column + 1).to_string())
            .item("Byte", self.position.byte_index.to_string());
        if let Some(range) = &self.selected_range {
            summary = summary.item(
                "Selection",
                format!("bytes {} to {}", range.start, range.end),
            );
        }
        summary
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextInputPlatformContext {
    pub input: TextInputId,
    pub cursor_rect: LogicalRect,
    pub target: Option<UiNodeId>,
}

impl TextInputPlatformContext {
    pub fn new(input: TextInputId, cursor_rect: LogicalRect) -> Self {
        Self {
            input,
            cursor_rect,
            target: None,
        }
    }

    pub fn from_caret_rect(input: TextInputId, caret: TextInputCaretRect) -> Self {
        Self::new(input, logical_rect_from_ui_rect(caret.rect))
    }

    pub fn for_node(node: UiNodeId, caret: TextInputCaretRect) -> Self {
        Self::from_caret_rect(text_input_id_for_node(node), caret).target(node)
    }

    pub const fn target(mut self, target: UiNodeId) -> Self {
        self.target = Some(target);
        self
    }

    pub const fn cursor_rect(mut self, cursor_rect: LogicalRect) -> Self {
        self.cursor_rect = cursor_rect;
        self
    }

    pub fn with_caret_rect(self, caret: TextInputCaretRect) -> Self {
        self.cursor_rect(logical_rect_from_ui_rect(caret.rect))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextInputLayoutMetrics {
    pub text_rect: UiRect,
    pub char_width: f32,
    pub line_height: f32,
    pub caret_width: f32,
    pub scroll_offset: UiPoint,
}

impl TextInputLayoutMetrics {
    pub fn new(text_rect: UiRect, char_width: f32, line_height: f32) -> Self {
        Self {
            text_rect,
            char_width: sanitize_positive_dimension(char_width, 1.0),
            line_height: sanitize_positive_dimension(line_height, 1.0),
            caret_width: 1.0,
            scroll_offset: UiPoint::new(0.0, 0.0),
        }
    }

    pub fn from_style(text_rect: UiRect, style: &TextStyle) -> Self {
        Self::new(
            text_rect,
            style.font_size * TEXT_INPUT_APPROX_CHAR_WIDTH_FACTOR,
            style.line_height.max(1.0),
        )
    }

    pub fn caret_width(mut self, caret_width: f32) -> Self {
        self.caret_width = sanitize_positive_dimension(caret_width, self.caret_width);
        self
    }

    pub const fn scroll_offset(mut self, scroll_offset: UiPoint) -> Self {
        self.scroll_offset = scroll_offset;
        self
    }

    pub fn point_for_position(self, position: TextInputPosition) -> UiPoint {
        UiPoint::new(
            self.text_rect.x - self.scroll_offset.x + position.column as f32 * self.char_width,
            self.text_rect.y - self.scroll_offset.y + position.line as f32 * self.line_height,
        )
    }

    fn font_size(self) -> f32 {
        sanitize_positive_dimension(
            self.char_width / TEXT_INPUT_APPROX_CHAR_WIDTH_FACTOR,
            self.char_width,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextInputCaretRect {
    pub position: TextInputPosition,
    pub rect: UiRect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextInputSelectionRect {
    pub byte_range: Range<usize>,
    pub line: usize,
    pub rect: UiRect,
}

#[derive(Debug, Clone)]
struct TextInputMeasuredLayout {
    lines: Vec<TextInputMeasuredLine>,
}

#[derive(Debug, Clone)]
struct TextInputMeasuredLine {
    byte_range: Range<usize>,
    stops: Vec<TextInputMeasuredStop>,
    width: f32,
}

#[derive(Debug, Clone, Copy)]
struct TextInputMeasuredStop {
    byte: usize,
    x: f32,
}

impl TextInputMeasuredLayout {
    fn measure(text: &str, style: &TextStyle) -> Option<Self> {
        text_input_measured_layout(text, style)
    }

    fn max_width(&self) -> f32 {
        self.lines.iter().map(|line| line.width).fold(0.0, f32::max)
    }

    fn x_for_byte(&self, line: usize, byte: usize) -> Option<f32> {
        self.lines.get(line)?.x_for_byte(byte)
    }

    fn span_between(&self, line: usize, start: usize, end: usize) -> Option<(f32, f32)> {
        let start_x = self.x_for_byte(line, start)?;
        let end_x = self.x_for_byte(line, end)?;
        Some((start_x.min(end_x), (end_x - start_x).abs()))
    }

    fn byte_for_x(&self, line: usize, x: f32) -> Option<usize> {
        self.lines.get(line)?.byte_for_x(x)
    }
}

#[cfg_attr(not(feature = "text-cosmic"), allow(dead_code))]
impl TextInputMeasuredLine {
    fn new(byte_range: Range<usize>) -> Self {
        Self {
            byte_range,
            stops: Vec::new(),
            width: 0.0,
        }
    }

    fn push_stop(&mut self, byte: usize, x: f32) {
        if byte < self.byte_range.start || byte > self.byte_range.end || !x.is_finite() {
            return;
        }
        self.stops.push(TextInputMeasuredStop { byte, x });
    }

    fn normalize(&mut self) {
        self.push_stop(self.byte_range.start, 0.0);
        self.push_stop(self.byte_range.end, self.width.max(0.0));
        self.stops.sort_by(|left, right| {
            left.byte
                .cmp(&right.byte)
                .then_with(|| left.x.total_cmp(&right.x))
        });
        self.stops.dedup_by(|left, right| left.byte == right.byte);
    }

    fn x_for_byte(&self, byte: usize) -> Option<f32> {
        let byte = byte.clamp(self.byte_range.start, self.byte_range.end);
        if let Some(stop) = self.stops.iter().find(|stop| stop.byte == byte) {
            return Some(stop.x);
        }
        let before = self.stops.iter().rev().find(|stop| stop.byte < byte)?;
        let after = self.stops.iter().find(|stop| stop.byte > byte)?;
        let span = after.byte.saturating_sub(before.byte).max(1) as f32;
        let t = byte.saturating_sub(before.byte) as f32 / span;
        Some(before.x + (after.x - before.x) * t)
    }

    fn byte_for_x(&self, x: f32) -> Option<usize> {
        if self.stops.is_empty() || !x.is_finite() {
            return Some(self.byte_range.start);
        }
        let mut stops = self.stops.clone();
        stops.sort_by(|left, right| {
            left.x
                .total_cmp(&right.x)
                .then_with(|| left.byte.cmp(&right.byte))
        });
        let mut previous = stops[0];
        for stop in stops.into_iter().skip(1) {
            let midpoint = previous.x + (stop.x - previous.x) * 0.5;
            if x < midpoint {
                return Some(previous.byte);
            }
            previous = stop;
        }
        Some(previous.byte)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextInputPaintOptions {
    pub selection_fill: ColorRgba,
    pub caret_fill: ColorRgba,
    pub selection_corner_radius: u8,
    pub show_caret: bool,
}

impl Default for TextInputPaintOptions {
    fn default() -> Self {
        Self {
            selection_fill: ColorRgba::new(64, 128, 255, 96),
            caret_fill: ColorRgba::WHITE,
            selection_corner_radius: 2,
            show_caret: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextInputRenderPlan {
    pub text: PaintText,
    pub caret: Option<TextInputCaretRect>,
    pub selection_rects: Vec<TextInputSelectionRect>,
    pub caret_paint: Option<PaintRect>,
    pub selection_paint: Vec<PaintRect>,
}

impl TextInputRenderPlan {
    pub fn new(
        state: &TextInputState,
        metrics: TextInputLayoutMetrics,
        text_style: TextStyle,
        paint: TextInputPaintOptions,
    ) -> Self {
        let text_style = text_input_render_text_style(text_style);
        let measured = TextInputMeasuredLayout::measure(&state.text, &text_style);
        let text = PaintText::new(state.text.clone(), metrics.text_rect, text_style)
            .multiline(state.multiline)
            .overflow(TextOverflow::Clip);
        let caret = paint.show_caret.then(|| {
            text_input_caret_rect_with_layout(&state.text, state.caret, metrics, measured.as_ref())
        });
        let selection_rects = text_input_selection_rects_with_layout(
            &state.text,
            state.selected_range(),
            metrics,
            measured.as_ref(),
        );
        let selection_paint = selection_rects
            .iter()
            .map(|selection| {
                PaintRect::solid(selection.rect, paint.selection_fill)
                    .corner_radii(CornerRadii::uniform(paint.selection_corner_radius as f32))
            })
            .collect::<Vec<_>>();
        let caret_paint = caret.map(|caret| PaintRect::solid(caret.rect, paint.caret_fill));
        Self {
            text,
            caret,
            selection_rects,
            caret_paint,
            selection_paint,
        }
    }

    pub fn overlay_primitives(&self) -> Vec<ScenePrimitive> {
        self.selection_paint
            .iter()
            .cloned()
            .map(ScenePrimitive::Rect)
            .chain(self.caret_paint.iter().cloned().map(ScenePrimitive::Rect))
            .collect()
    }

    pub fn scene_primitives(&self) -> Vec<ScenePrimitive> {
        self.selection_paint
            .iter()
            .cloned()
            .map(ScenePrimitive::Rect)
            .chain(std::iter::once(ScenePrimitive::Text(self.text.clone())))
            .chain(self.caret_paint.iter().cloned().map(ScenePrimitive::Rect))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextInputInteractionPolicy {
    pub enabled: bool,
    pub read_only: bool,
    pub selectable: bool,
    pub allow_copy: bool,
}

impl TextInputInteractionPolicy {
    pub const EDITABLE: Self = Self {
        enabled: true,
        read_only: false,
        selectable: true,
        allow_copy: true,
    };

    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            read_only: false,
            selectable: false,
            allow_copy: false,
        }
    }

    pub const fn read_only() -> Self {
        Self {
            enabled: true,
            read_only: true,
            selectable: true,
            allow_copy: true,
        }
    }

    pub const fn can_edit(self) -> bool {
        self.enabled && !self.read_only
    }

    pub const fn can_select(self) -> bool {
        self.enabled && self.selectable
    }

    pub const fn can_copy(self) -> bool {
        self.can_select() && self.allow_copy
    }

    pub const fn can_move_caret(self) -> bool {
        self.can_edit() || self.can_select()
    }

    pub const fn can_receive_focus(self) -> bool {
        self.can_edit() || self.can_select()
    }
}

impl Default for TextInputInteractionPolicy {
    fn default() -> Self {
        Self::EDITABLE
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputClipboardAction {
    Copy(String),
    Cut(String),
    Paste,
}

impl TextInputClipboardAction {
    pub fn clipboard_request(&self) -> ClipboardRequest {
        match self {
            Self::Copy(text) | Self::Cut(text) => ClipboardRequest::WriteText(text.clone()),
            Self::Paste => ClipboardRequest::ReadText,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInputOutcome {
    pub phase: EditPhase,
    pub changed: bool,
    pub committed: bool,
    pub canceled: bool,
    pub clipboard: Option<TextInputClipboardAction>,
    pub transaction: Option<EditTransaction<TextEditChange>>,
    pub history_apply: Option<TextEditHistoryApply>,
}

impl TextInputOutcome {
    fn new(phase: EditPhase, changed: bool, clipboard: Option<TextInputClipboardAction>) -> Self {
        Self {
            phase,
            changed,
            committed: phase == EditPhase::CommitEdit,
            canceled: phase == EditPhase::CancelEdit,
            clipboard,
            transaction: None,
            history_apply: None,
        }
    }

    pub fn clipboard_request(&self) -> Option<ClipboardRequest> {
        self.clipboard
            .as_ref()
            .map(TextInputClipboardAction::clipboard_request)
    }

    fn with_transaction(mut self, transaction: Option<EditTransaction<TextEditChange>>) -> Self {
        self.transaction = transaction;
        self
    }

    fn with_history_apply(mut self, history_apply: Option<TextEditHistoryApply>) -> Self {
        self.history_apply = history_apply;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextInputEventOutcome {
    pub input: UiInputResult,
    pub edit: Option<TextInputOutcome>,
    pub focused: bool,
    pub platform_requests: Vec<PlatformRequest>,
}

impl TextInputEventOutcome {
    pub fn did_edit(&self) -> bool {
        self.edit.as_ref().is_some_and(|edit| edit.changed)
    }

    pub fn committed(&self) -> bool {
        self.edit.as_ref().is_some_and(|edit| edit.committed)
    }

    pub fn canceled(&self) -> bool {
        self.edit.as_ref().is_some_and(|edit| edit.canceled)
    }
}

#[derive(Debug, Clone)]
pub struct TextInputOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub focused_visual: Option<UiVisual>,
    pub disabled_visual: Option<UiVisual>,
    pub text_style: TextStyle,
    pub placeholder_style: TextStyle,
    pub placeholder: String,
    pub shader: Option<ShaderEffect>,
    pub animation: Option<AnimationMachine>,
    pub enabled: bool,
    pub read_only: bool,
    pub selectable: bool,
    pub allow_copy: bool,
    pub focused: bool,
    pub caret_visible: bool,
    pub edit_action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for TextInputOptions {
    fn default() -> Self {
        let placeholder_style = TextStyle {
            color: ColorRgba::new(144, 156, 174, 255),
            ..Default::default()
        };
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(180.0),
                    height: length(30.0),
                },
                padding: taffy::prelude::Rect::length(6.0_f32),
                ..Default::default()
            }),
            visual: UiVisual::panel(
                ColorRgba::new(18, 22, 28, 255),
                Some(StrokeStyle::new(ColorRgba::new(72, 84, 104, 255), 1.0)),
                4.0,
            ),
            focused_visual: Some(UiVisual::panel(
                ColorRgba::new(20, 27, 36, 255),
                Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.5)),
                4.0,
            )),
            disabled_visual: Some(UiVisual::panel(
                ColorRgba::new(25, 28, 34, 170),
                Some(StrokeStyle::new(ColorRgba::new(58, 66, 78, 170), 1.0)),
                4.0,
            )),
            text_style: TextStyle::default(),
            placeholder_style,
            placeholder: String::new(),
            shader: None,
            animation: None,
            enabled: true,
            read_only: false,
            selectable: true,
            allow_copy: true,
            focused: false,
            caret_visible: true,
            edit_action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl TextInputOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_text_style(mut self, style: TextStyle) -> Self {
        self.text_style = style;
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn with_edit_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.edit_action = Some(action.into());
        self
    }

    pub const fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    pub const fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    pub const fn allow_copy(mut self, allow_copy: bool) -> Self {
        self.allow_copy = allow_copy;
        self
    }

    pub const fn interaction_policy(&self) -> TextInputInteractionPolicy {
        TextInputInteractionPolicy {
            enabled: self.enabled,
            read_only: self.read_only,
            selectable: self.selectable,
            allow_copy: self.allow_copy,
        }
    }

    pub const fn can_edit(&self) -> bool {
        self.interaction_policy().can_edit()
    }

    pub const fn can_select(&self) -> bool {
        self.interaction_policy().can_select()
    }

    pub const fn can_copy(&self) -> bool {
        self.interaction_policy().can_copy()
    }

    pub const fn can_receive_focus(&self) -> bool {
        self.interaction_policy().can_receive_focus()
    }
}

pub fn singleline_text_input(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    state: &TextInputState,
    options: TextInputOptions,
) -> UiNodeId {
    let mut state = state.clone();
    state.multiline = false;
    text_input(document, parent, name, &state, options)
}

pub fn multiline_text_input(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    state: &TextInputState,
    options: TextInputOptions,
) -> UiNodeId {
    let mut state = state.clone();
    state.multiline = true;
    let mut options = options;
    if text_input_dimension_is_default(options.layout.as_taffy_style().size.height, 30.0) {
        options.layout = options.layout.with_height(120.0);
    }
    text_input(document, parent, name, &state, options)
}

pub fn text_area(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    state: &TextInputState,
    options: TextInputOptions,
) -> UiNodeId {
    multiline_text_input(document, parent, name, state, options)
}

pub fn code_editor(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    state: &TextInputState,
    mut options: TextInputOptions,
) -> UiNodeId {
    options.text_style = code_text_style();
    options.placeholder_style = TextStyle {
        color: ColorRgba::new(126, 139, 158, 255),
        ..code_text_style()
    };
    if options.placeholder.is_empty() {
        options.placeholder = "Type code".to_string();
    }
    if options.accessibility_label.is_none() {
        options.accessibility_label = Some("Code editor".to_string());
    }
    if text_input_dimension_is_default(options.layout.as_taffy_style().size.width, 180.0) {
        options.layout = options.layout.with_width(360.0);
    }
    if text_input_dimension_is_default(options.layout.as_taffy_style().size.height, 30.0) {
        options.layout = options.layout.with_height(180.0);
    }
    multiline_text_input(document, parent, name, state, options)
}

pub fn search_input(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    state: &TextInputState,
    mut options: TextInputOptions,
) -> UiNodeId {
    if options.placeholder.is_empty() {
        options.placeholder = "Search".to_string();
    }
    if options.accessibility_label.is_none() {
        options.accessibility_label = Some("Search".to_string());
    }
    let node = singleline_text_input(document, parent, name, state, options);
    if let Some(accessibility) = document.node_mut(node).accessibility.as_mut() {
        accessibility.role = AccessibilityRole::SearchBox;
    }
    node
}

pub fn password_input(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    state: &TextInputState,
    mut options: TextInputOptions,
) -> UiNodeId {
    if options.placeholder.is_empty() {
        options.placeholder = "Password".to_string();
    }
    if options.accessibility_label.is_none() {
        options.accessibility_label = Some("Password".to_string());
    }
    let masked = password_display_state(state);
    singleline_text_input(document, parent, name, &masked, options)
}

pub fn text_input(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    state: &TextInputState,
    options: TextInputOptions,
) -> UiNodeId {
    let name = name.into();
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::TextBox)
        .label(
            options
                .accessibility_label
                .clone()
                .unwrap_or_else(|| name.clone()),
        )
        .value(state.text.clone())
        .summary(
            state
                .caret_info()
                .accessibility_summary(format!("{name} caret")),
        );
    if options.can_select() {
        accessibility = accessibility
            .shortcut("Ctrl+A")
            .action(AccessibilityAction::new("select_all", "Select all").shortcut("Ctrl+A"));
    }
    if options.can_copy() {
        accessibility = accessibility
            .shortcut("Ctrl+C")
            .action(AccessibilityAction::new("copy", "Copy").shortcut("Ctrl+C"));
    }
    if options.can_edit() {
        accessibility = accessibility
            .shortcut("Ctrl+X")
            .shortcut("Ctrl+V")
            .action(AccessibilityAction::new("cut", "Cut").shortcut("Ctrl+X"))
            .action(AccessibilityAction::new("paste", "Paste").shortcut("Ctrl+V"));
    }
    let hint = options
        .accessibility_hint
        .clone()
        .or_else(|| (!options.placeholder.is_empty()).then(|| options.placeholder.clone()));
    if let Some(hint) = hint {
        accessibility = accessibility.hint(hint);
    }
    if options.read_only {
        accessibility = accessibility.read_only();
    }
    if options.enabled {
        if options.can_receive_focus() {
            accessibility = accessibility.focusable();
        }
    } else {
        accessibility = accessibility.disabled();
    }
    let input_behavior = if options.can_receive_focus() {
        InputBehavior::BUTTON
    } else {
        InputBehavior::NONE
    };
    let interaction_policy = options.interaction_policy();
    let initial_visual = if options.enabled {
        options.visual
    } else {
        options.disabled_visual.unwrap_or(options.visual)
    };
    let mut root_node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style.clone(),
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_input(input_behavior)
    .with_visual(initial_visual)
    .with_accessibility(accessibility);
    if let Some(shader) = options.shader {
        root_node = root_node.with_shader(shader);
    }
    if let Some(animation) = options.animation {
        root_node = root_node.with_animation(animation);
    }
    if let Some(action) = options.edit_action.clone() {
        root_node = root_node.with_action(action);
    }
    let root = document.add_child(parent, root_node);
    if options.focused {
        let mut focus = document.focus_state().clone();
        focus.focused = Some(root);
        document.set_focus_state(focus);
    }
    let focused = options.focused || document.focus.focused == Some(root);
    let visual = if !options.enabled {
        options.disabled_visual.unwrap_or(options.visual)
    } else if focused {
        options.focused_visual.unwrap_or(options.visual)
    } else {
        options.visual
    };
    document.set_node_visual(root, visual);
    let show_caret = focused && options.caret_visible && interaction_policy.can_move_caret();
    let display_text = if state.text.is_empty() {
        options.placeholder
    } else {
        state.text.clone()
    };
    let style = text_input_render_text_style(if state.text.is_empty() {
        placeholder_style_for_text_style(options.placeholder_style, options.text_style)
    } else {
        options.text_style
    });
    let text_metrics = TextInputLayoutMetrics::from_style(
        text_input_scene_text_rect(state, &display_text, &style),
        &style,
    );
    let paint = TextInputPaintOptions {
        show_caret,
        ..TextInputPaintOptions::default()
    };
    let primitives = text_input_scene_primitives(
        state,
        display_text,
        style,
        text_metrics,
        paint,
        focused && interaction_policy.can_select(),
    );
    let text_scene = document.add_child(
        root,
        UiNode::scene(
            format!("{name}.text"),
            primitives,
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            }),
        ),
    );
    publish_inline_intrinsic_size(
        document,
        root,
        vec![text_scene],
        inline_intrinsic_base_size(&options.layout.style, &[], 1),
    );
    root
}

fn password_display_state(state: &TextInputState) -> TextInputState {
    let mut masked = state.clone();
    masked.text = "*".repeat(state.text.chars().count());
    masked.caret = char_count_before_byte(&state.text, state.caret);
    masked.selection_anchor = state
        .selection_anchor
        .map(|anchor| char_count_before_byte(&state.text, anchor));
    masked.multiline = false;
    masked.composing = state
        .composing
        .as_ref()
        .map(|text| "*".repeat(text.chars().count()));
    masked
}

fn text_input_render_text_style(mut style: TextStyle) -> TextStyle {
    style.wrap = TextWrap::None;
    style
}

fn char_count_before_byte(text: &str, index: usize) -> usize {
    let index = clamp_to_char_boundary(text, index);
    text[..index].chars().count()
}

fn text_input_dimension_is_default(dimension: Dimension, default: f32) -> bool {
    dimension == Dimension::auto() || dimension == Dimension::length(default)
}

fn text_input_scene_text_rect(
    state: &TextInputState,
    display_text: &str,
    style: &TextStyle,
) -> UiRect {
    let line_count = if state.multiline {
        display_text
            .chars()
            .filter(|character| *character == '\n')
            .count()
            + 1
    } else {
        1
    };
    let column_count = display_text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .max(state.caret_position().column);
    let char_width =
        sanitize_positive_dimension(style.font_size * TEXT_INPUT_APPROX_CHAR_WIDTH_FACTOR, 1.0);
    let estimated_text_width = TextInputMeasuredLayout::measure(display_text, style)
        .map(|layout| layout.max_width())
        .unwrap_or_else(|| {
            display_text
                .lines()
                .map(|line| {
                    line.chars()
                        .map(|character| {
                            text_input_char_advance_for_font_size(character, style.font_size)
                        })
                        .sum::<f32>()
                })
                .fold(0.0, f32::max)
        });
    let line_height = sanitize_positive_dimension(style.line_height, style.font_size.max(1.0));
    UiRect::new(
        TEXT_INPUT_CONTENT_INSET_X,
        TEXT_INPUT_CONTENT_INSET_Y,
        estimated_text_width.max(column_count as f32 * char_width) + char_width,
        (line_count as f32 * line_height).max(line_height),
    )
}

fn placeholder_style_for_text_style(mut placeholder: TextStyle, text: TextStyle) -> TextStyle {
    let default = TextStyle::default();
    if (placeholder.font_size - default.font_size).abs() <= f32::EPSILON {
        placeholder.font_size = text.font_size;
    }
    if (placeholder.line_height - default.line_height).abs() <= f32::EPSILON {
        placeholder.line_height = text.line_height;
    }
    if placeholder.family == default.family {
        placeholder.family = text.family;
    }
    placeholder
}

fn text_input_content_rect(rect: UiRect, style: &TextStyle) -> UiRect {
    let x_inset = TEXT_INPUT_CONTENT_INSET_X.min((rect.width * 0.5).max(0.0));
    let y_inset = TEXT_INPUT_CONTENT_INSET_Y.min((rect.height * 0.5).max(0.0));
    UiRect::new(
        rect.x + x_inset,
        rect.y + y_inset,
        (rect.width - x_inset * 2.0).max(1.0),
        (rect.height - y_inset * 2.0).max(sanitize_positive_dimension(style.line_height, 1.0)),
    )
}

fn text_input_scene_primitives(
    state: &TextInputState,
    display_text: String,
    style: TextStyle,
    metrics: TextInputLayoutMetrics,
    paint: TextInputPaintOptions,
    show_selection: bool,
) -> Vec<ScenePrimitive> {
    let measured = TextInputMeasuredLayout::measure(state.text(), &style);
    let text = PaintText::new(display_text, metrics.text_rect, style)
        .multiline(state.multiline)
        .overflow(TextOverflow::Clip);
    let selection_rects = if show_selection {
        text_input_selection_rects_with_layout(
            state.text(),
            state.selected_range(),
            metrics,
            measured.as_ref(),
        )
    } else {
        Vec::new()
    };
    let mut primitives = selection_rects
        .into_iter()
        .map(|selection| {
            ScenePrimitive::Rect(
                PaintRect::solid(selection.rect, paint.selection_fill)
                    .corner_radii(CornerRadii::uniform(paint.selection_corner_radius as f32)),
            )
        })
        .collect::<Vec<_>>();
    primitives.push(ScenePrimitive::Text(text));
    if paint.show_caret {
        primitives.push(ScenePrimitive::Rect(PaintRect::solid(
            text_input_caret_rect_with_layout(
                state.text(),
                state.caret,
                metrics,
                measured.as_ref(),
            )
            .rect,
            paint.caret_fill,
        )));
    }
    primitives
}

fn text_input_layout_metrics_from_document(
    document: &UiDocument,
    node: UiNodeId,
    options: &TextInputOptions,
) -> Option<TextInputLayoutMetrics> {
    let node = document.nodes.get(node.0)?;
    let rect = node
        .children
        .first()
        .and_then(|child| document.nodes.get(child.0))
        .map(|child| child.layout.rect)
        .unwrap_or(node.layout.rect);
    if !rect_is_finite(rect) || rect.width <= 0.0 || rect.height <= 0.0 {
        return None;
    }
    Some(TextInputLayoutMetrics::from_style(
        text_input_content_rect(rect, &options.text_style),
        &options.text_style,
    ))
}

pub fn selectable_text(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    state: &TextInputState,
    mut options: TextInputOptions,
) -> UiNodeId {
    options.read_only = true;
    options.selectable = true;
    options.allow_copy = true;
    text_input(document, parent, name, state, options)
}

pub fn handle_text_input_event(
    document: &mut UiDocument,
    node: UiNodeId,
    state: &mut TextInputState,
    event: UiInputEvent,
    platform_context: Option<TextInputPlatformContext>,
) -> TextInputEventOutcome {
    handle_text_input_event_with_metrics(document, node, state, event, platform_context, None)
}

pub fn handle_text_input_event_with_options(
    document: &mut UiDocument,
    node: UiNodeId,
    state: &mut TextInputState,
    options: &TextInputOptions,
    event: UiInputEvent,
    platform_context: Option<TextInputPlatformContext>,
) -> TextInputEventOutcome {
    handle_text_input_event_with_metrics_and_options(
        document,
        node,
        state,
        options,
        event,
        platform_context,
        None,
    )
}

pub fn handle_text_input_event_with_metrics(
    document: &mut UiDocument,
    node: UiNodeId,
    state: &mut TextInputState,
    event: UiInputEvent,
    platform_context: Option<TextInputPlatformContext>,
    layout_metrics: Option<TextInputLayoutMetrics>,
) -> TextInputEventOutcome {
    let options = TextInputOptions::default();
    handle_text_input_event_with_metrics_and_options(
        document,
        node,
        state,
        &options,
        event,
        platform_context,
        layout_metrics,
    )
}

pub fn handle_text_input_event_with_metrics_and_options(
    document: &mut UiDocument,
    node: UiNodeId,
    state: &mut TextInputState,
    options: &TextInputOptions,
    event: UiInputEvent,
    platform_context: Option<TextInputPlatformContext>,
    layout_metrics: Option<TextInputLayoutMetrics>,
) -> TextInputEventOutcome {
    let policy = options.interaction_policy();
    let was_focused = document.focus.focused == Some(node);
    let text_event = matches!(event, UiInputEvent::TextInput(_) | UiInputEvent::Key { .. });
    let input = if text_event {
        UiInputResult {
            hovered: document.focus.hovered,
            focused: document.focus.focused,
            pressed: document.focus.pressed,
            clicked: None,
            scrolled: None,
            consumed: document.focus.focused.is_some(),
            consumed_by: document.focus.focused,
        }
    } else {
        document.handle_input(event.clone())
    };
    let focused = document.focus.focused == Some(node);
    let mut platform_requests = Vec::new();
    let mut state_changed = false;
    let mut edit = None;
    let layout_metrics =
        layout_metrics.or_else(|| text_input_layout_metrics_from_document(document, node, options));

    if focused && text_event {
        let before_text = state.text.clone();
        let before_caret = state.caret;
        let before_selection = state.selection_anchor;
        let before_composing = state.composing.clone();
        let outcome = state.handle_event_for_target_with_policy(
            &event,
            TransactionTarget::node(node),
            policy,
        );
        if let Some(request) = outcome.clipboard_request() {
            platform_requests.push(PlatformRequest::Clipboard(request));
        }
        state_changed = before_text != state.text
            || before_caret != state.caret
            || before_selection != state.selection_anchor
            || before_composing != state.composing;
        edit = Some(outcome);
    } else if focused && policy.can_move_caret() {
        if let Some((point, selecting)) =
            text_input_pointer_edit(&event, input.pressed == Some(node) && policy.can_select())
        {
            if let Some(metrics) = layout_metrics {
                let before_caret = state.caret;
                let before_selection = state.selection_anchor;
                state.normalize_selection();
                let anchor = state.selection_anchor.unwrap_or(state.caret);
                let measured = TextInputMeasuredLayout::measure(&state.text, &options.text_style);
                state.caret = text_input_byte_index_at_point_with_layout(
                    &state.text,
                    state.multiline,
                    metrics,
                    point,
                    measured.as_ref(),
                );
                state.selection_anchor = selecting.then_some(anchor);
                state_changed =
                    before_caret != state.caret || before_selection != state.selection_anchor;
                edit = Some(TextInputOutcome::new(EditPhase::Preview, false, None));
            }
        }
    }

    let platform_context = platform_context.map(|context| {
        if let Some(metrics) = layout_metrics {
            let measured = TextInputMeasuredLayout::measure(&state.text, &options.text_style);
            context.with_caret_rect(text_input_caret_rect_with_layout(
                &state.text,
                state.caret,
                metrics,
                measured.as_ref(),
            ))
        } else {
            context
        }
    });

    if !was_focused && focused && policy.can_edit() {
        if let Some(context) = platform_context.clone() {
            platform_requests.push(PlatformRequest::TextIme(
                state.activate_ime_request(context.clone()),
            ));
            platform_requests.push(PlatformRequest::TextIme(
                TextInputState::show_keyboard_request(context.input),
            ));
        }
    } else if was_focused && !focused && policy.can_edit() {
        if let Some(context) = platform_context.clone() {
            platform_requests.push(PlatformRequest::TextIme(
                TextInputState::hide_keyboard_request(context.input.clone()),
            ));
            platform_requests.push(PlatformRequest::TextIme(
                TextInputState::deactivate_ime_request(context.input),
            ));
        }
    }

    if focused && policy.can_edit() {
        if let (Some(context), Some(outcome)) = (platform_context, edit.as_ref()) {
            if outcome.committed || outcome.canceled {
                platform_requests.push(PlatformRequest::TextIme(
                    TextInputState::hide_keyboard_request(context.input.clone()),
                ));
                platform_requests.push(PlatformRequest::TextIme(
                    TextInputState::deactivate_ime_request(context.input),
                ));
            } else if state_changed {
                platform_requests.push(PlatformRequest::TextIme(state.update_ime_request(context)));
            }
        }
    }

    TextInputEventOutcome {
        input,
        edit,
        focused,
        platform_requests,
    }
}

pub fn handle_selectable_text_event(
    document: &mut UiDocument,
    node: UiNodeId,
    state: &mut TextInputState,
    options: &TextInputOptions,
    event: UiInputEvent,
    platform_context: Option<TextInputPlatformContext>,
) -> TextInputEventOutcome {
    handle_selectable_text_event_with_metrics(
        document,
        node,
        state,
        options,
        event,
        platform_context,
        None,
    )
}

pub fn handle_selectable_text_event_with_metrics(
    document: &mut UiDocument,
    node: UiNodeId,
    state: &mut TextInputState,
    options: &TextInputOptions,
    event: UiInputEvent,
    platform_context: Option<TextInputPlatformContext>,
    layout_metrics: Option<TextInputLayoutMetrics>,
) -> TextInputEventOutcome {
    let mut options = options.clone();
    options.read_only = true;
    options.selectable = true;
    options.allow_copy = true;
    options.edit_action = None;
    handle_text_input_event_with_metrics_and_options(
        document,
        node,
        state,
        &options,
        event,
        platform_context,
        layout_metrics,
    )
}

pub fn text_input_actions_from_outcome(
    document: &UiDocument,
    input: UiNodeId,
    options: &TextInputOptions,
    outcome: &TextInputOutcome,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    push_text_input_outcome_actions(&mut queue, document, input, options, outcome);
    queue
}

pub fn push_text_input_outcome_actions<'a>(
    queue: &'a mut WidgetActionQueue,
    document: &UiDocument,
    input: UiNodeId,
    options: &TextInputOptions,
    outcome: &TextInputOutcome,
) -> &'a mut WidgetActionQueue {
    if !options.can_edit() || !action_target_enabled(document, input) {
        return queue;
    }
    if let Some(binding) = options.edit_action.clone() {
        queue.value_edit(input, binding, outcome.phase);
    }
    queue
}

fn text_input_pointer_edit(event: &UiInputEvent, pressed: bool) -> Option<(UiPoint, bool)> {
    match event {
        UiInputEvent::PointerDown(point) => Some((*point, false)),
        UiInputEvent::PointerMove(point) if pressed => Some((*point, true)),
        _ => None,
    }
}

#[cfg(feature = "text-cosmic")]
fn text_input_measured_layout(text: &str, style: &TextStyle) -> Option<TextInputMeasuredLayout> {
    let line_ranges = text_line_ranges(text);
    let mut measured = TextInputMeasuredLayout {
        lines: line_ranges
            .iter()
            .map(|(_, range)| TextInputMeasuredLine::new(range.clone()))
            .collect(),
    };
    let font_size = style.font_size.max(1.0);
    let line_height = style.line_height.max(font_size);
    TEXT_INPUT_FONT_SYSTEM.with(|font_system| {
        let mut font_system = font_system.borrow_mut();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(font_size, line_height));
        buffer.set_wrap(&mut font_system, CosmicWrap::None);
        buffer.set_size(&mut font_system, None, None);
        let attrs = Attrs::new()
            .family(text_input_cosmic_family(&style.family))
            .weight(text_input_cosmic_weight(style.weight))
            .style(text_input_cosmic_font_style(style.style))
            .stretch(text_input_cosmic_stretch(style.stretch));
        buffer.set_text(
            &mut font_system,
            text,
            &attrs,
            text_input_cosmic_shaping(text),
            None,
        );

        for run in buffer.layout_runs() {
            let Some((_, line_range)) = line_ranges.get(run.line_i) else {
                continue;
            };
            let Some(line) = measured.lines.get_mut(run.line_i) else {
                continue;
            };
            line.width = line.width.max(run.line_w);
            for glyph in run.glyphs {
                let start = line_range.start + glyph.start;
                let end = line_range.start + glyph.end;
                if glyph.level.is_rtl() {
                    line.push_stop(start, glyph.x + glyph.w);
                    line.push_stop(end, glyph.x);
                } else {
                    line.push_stop(start, glyph.x);
                    line.push_stop(end, glyph.x + glyph.w);
                }
            }
        }

        for line in &mut measured.lines {
            line.normalize();
        }
        Some(measured)
    })
}

#[cfg(not(feature = "text-cosmic"))]
fn text_input_measured_layout(_text: &str, _style: &TextStyle) -> Option<TextInputMeasuredLayout> {
    None
}

#[cfg(feature = "text-cosmic")]
fn text_input_cosmic_font_system() -> FontSystem {
    let mut font_system = FontSystem::new_with_fonts([
        text_input_embedded_cosmic_font(epaint_default_fonts::UBUNTU_LIGHT),
        text_input_embedded_cosmic_font(epaint_default_fonts::HACK_REGULAR),
        text_input_embedded_cosmic_font(epaint_default_fonts::NOTO_EMOJI_REGULAR),
    ]);
    {
        let db = font_system.db_mut();
        db.set_sans_serif_family("Ubuntu");
        db.set_serif_family("Ubuntu");
        db.set_monospace_family("Hack");
    }
    font_system
}

#[cfg(feature = "text-cosmic")]
fn text_input_embedded_cosmic_font(bytes: &'static [u8]) -> fontdb::Source {
    let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bytes);
    fontdb::Source::Binary(data)
}

#[cfg(feature = "text-cosmic")]
fn text_input_cosmic_family(family: &FontFamily) -> CosmicFamily<'_> {
    match family {
        FontFamily::SansSerif => CosmicFamily::SansSerif,
        FontFamily::Serif => CosmicFamily::Serif,
        FontFamily::Monospace => CosmicFamily::Monospace,
        FontFamily::Named(name) => CosmicFamily::Name(name),
    }
}

#[cfg(feature = "text-cosmic")]
fn text_input_cosmic_weight(weight: FontWeight) -> CosmicWeight {
    CosmicWeight(weight.value())
}

#[cfg(feature = "text-cosmic")]
fn text_input_cosmic_font_style(style: FontStyle) -> CosmicFontStyle {
    match style {
        FontStyle::Normal => CosmicFontStyle::Normal,
        FontStyle::Italic => CosmicFontStyle::Italic,
        FontStyle::Oblique => CosmicFontStyle::Oblique,
    }
}

#[cfg(feature = "text-cosmic")]
fn text_input_cosmic_stretch(stretch: FontStretch) -> CosmicStretch {
    match stretch {
        FontStretch::Condensed => CosmicStretch::Condensed,
        FontStretch::Normal => CosmicStretch::Normal,
        FontStretch::Expanded => CosmicStretch::Expanded,
    }
}

#[cfg(feature = "text-cosmic")]
fn text_input_cosmic_shaping(text: &str) -> Shaping {
    if text.is_ascii() {
        Shaping::Basic
    } else {
        Shaping::Advanced
    }
}

fn filter_text_input(text: &str, multiline: bool) -> String {
    if multiline {
        let mut filtered = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(character) = chars.next() {
            if character == '\r' {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                filtered.push('\n');
            } else if character == '\n' || !character.is_control() {
                filtered.push(character);
            } else {
                continue;
            }
        }
        return filtered;
    }

    let mut filtered = String::with_capacity(text.len());
    let mut in_line_break = false;
    for character in text.chars() {
        if character == '\r' || character == '\n' {
            if !in_line_break {
                filtered.push(' ');
                in_line_break = true;
            }
        } else if !character.is_control() {
            filtered.push(character);
            in_line_break = false;
        }
    }
    filtered
}

fn previous_char_boundary(text: &str, index: usize) -> usize {
    text[..index]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    text[index..]
        .char_indices()
        .nth(1)
        .map(|(offset, _)| index + offset)
        .unwrap_or(text.len())
}

fn text_position_at(text: &str, index: usize) -> TextInputPosition {
    let index = clamp_to_char_boundary(text, index);
    let mut line = 0;
    let mut line_start = 0;
    for (byte_index, character) in text.char_indices() {
        if byte_index >= index {
            break;
        }
        if character == '\n' {
            line += 1;
            line_start = byte_index + character.len_utf8();
        }
    }
    let column = text[line_start..index].chars().count();
    TextInputPosition {
        byte_index: index,
        line,
        column,
    }
}

fn line_range_at(text: &str, index: usize) -> Range<usize> {
    let index = clamp_to_char_boundary(text, index);
    let start = text[..index]
        .rfind('\n')
        .map(|offset| offset + '\n'.len_utf8())
        .unwrap_or(0);
    let end = text[index..]
        .find('\n')
        .map(|offset| index + offset)
        .unwrap_or(text.len());
    start..end
}

fn byte_index_for_line_column(text: &str, line: usize, column: usize) -> usize {
    let mut current_line = 0;
    let mut line_start = 0;
    for (byte_index, character) in text.char_indices() {
        if current_line == line {
            break;
        }
        if character == '\n' {
            current_line += 1;
            line_start = byte_index + character.len_utf8();
        }
    }
    if current_line != line {
        return text.len();
    }

    text[line_start..]
        .char_indices()
        .take_while(|(_, character)| *character != '\n')
        .nth(column)
        .map(|(offset, _)| line_start + offset)
        .unwrap_or_else(|| line_range_at(text, line_start).end)
}

fn move_caret_vertically(text: &str, index: usize, line_delta: isize) -> usize {
    let position = text_position_at(text, index);
    let last_line = text.chars().filter(|character| *character == '\n').count();
    let target_line = match line_delta {
        delta if delta < 0 => position.line.checked_sub(delta.unsigned_abs()),
        delta => Some(position.line + delta as usize),
    };
    target_line
        .filter(|line| *line <= last_line)
        .map(|line| byte_index_for_line_column(text, line, position.column))
        .unwrap_or(index)
}

fn text_input_caret_rect(
    text: &str,
    caret: usize,
    metrics: TextInputLayoutMetrics,
) -> TextInputCaretRect {
    text_input_caret_rect_with_layout(text, caret, metrics, None)
}

fn text_input_caret_rect_with_layout(
    text: &str,
    caret: usize,
    metrics: TextInputLayoutMetrics,
    measured: Option<&TextInputMeasuredLayout>,
) -> TextInputCaretRect {
    let position = text_position_at(text, caret);
    let line_range = line_range_at(text, caret);
    let line_prefix_width = measured
        .and_then(|layout| layout.x_for_byte(position.line, caret.min(line_range.end)))
        .unwrap_or_else(|| {
            text_input_prefix_width(&text[line_range.start..caret.min(line_range.end)], metrics)
        });
    let origin = UiPoint::new(
        metrics.text_rect.x - metrics.scroll_offset.x + line_prefix_width,
        metrics.text_rect.y - metrics.scroll_offset.y + position.line as f32 * metrics.line_height,
    );
    TextInputCaretRect {
        position,
        rect: UiRect::new(origin.x, origin.y, metrics.caret_width, metrics.line_height),
    }
}

fn text_input_byte_index_at_point(
    text: &str,
    multiline: bool,
    metrics: TextInputLayoutMetrics,
    point: UiPoint,
) -> usize {
    text_input_byte_index_at_point_with_layout(text, multiline, metrics, point, None)
}

fn text_input_byte_index_at_point_with_layout(
    text: &str,
    multiline: bool,
    metrics: TextInputLayoutMetrics,
    point: UiPoint,
    measured: Option<&TextInputMeasuredLayout>,
) -> usize {
    let line_ranges = text_line_ranges(text);
    if line_ranges.is_empty() {
        return 0;
    }
    let relative_y = point.y - metrics.text_rect.y + metrics.scroll_offset.y;
    let requested_line = if multiline && relative_y.is_finite() {
        (relative_y / metrics.line_height).floor().max(0.0) as usize
    } else {
        0
    };
    let line = requested_line.min(line_ranges.len().saturating_sub(1));
    let line_start = line_ranges[line].1.start;
    let line_end = line_ranges[line].1.end;
    let relative_x = point.x - metrics.text_rect.x + metrics.scroll_offset.x;
    if relative_x.is_finite() {
        measured
            .and_then(|layout| layout.byte_for_x(line, relative_x.max(0.0)))
            .unwrap_or_else(|| {
                byte_index_for_line_x(text, line_start..line_end, relative_x.max(0.0), metrics)
            })
    } else {
        line_start
    }
}

fn text_input_selection_rects(
    text: &str,
    selected_range: Option<Range<usize>>,
    metrics: TextInputLayoutMetrics,
) -> Vec<TextInputSelectionRect> {
    text_input_selection_rects_with_layout(text, selected_range, metrics, None)
}

fn text_input_selection_rects_with_layout(
    text: &str,
    selected_range: Option<Range<usize>>,
    metrics: TextInputLayoutMetrics,
    measured: Option<&TextInputMeasuredLayout>,
) -> Vec<TextInputSelectionRect> {
    let Some(selected_range) = selected_range else {
        return Vec::new();
    };
    text_line_ranges(text)
        .into_iter()
        .filter_map(|(line, line_range)| {
            let start = selected_range.start.max(line_range.start);
            let end = selected_range.end.min(line_range.end);
            let newline_selected = selected_range.start <= line_range.end
                && selected_range.end > line_range.end
                && line_range.end <= text.len();
            if start >= end && !newline_selected {
                return None;
            }
            let start = start.min(line_range.end);
            let end = end.max(start).min(line_range.end);
            let position = TextInputPosition {
                byte_index: start,
                line,
                column: text[line_range.start..start].chars().count(),
            };
            let measured_span = measured.and_then(|layout| layout.span_between(line, start, end));
            let prefix_width = measured_span.map(|(x, _)| x).unwrap_or_else(|| {
                text_input_prefix_width(&text[line_range.start..start], metrics)
            });
            let origin = UiPoint::new(
                metrics.text_rect.x - metrics.scroll_offset.x + prefix_width,
                metrics.text_rect.y - metrics.scroll_offset.y
                    + position.line as f32 * metrics.line_height,
            );
            let selected_width = measured_span
                .map(|(_, width)| width)
                .unwrap_or_else(|| text_input_prefix_width(&text[start..end], metrics));
            let width = if selected_width <= f32::EPSILON {
                metrics.caret_width
            } else {
                selected_width
            };
            Some(TextInputSelectionRect {
                byte_range: start..end,
                line,
                rect: UiRect::new(origin.x, origin.y, width, metrics.line_height),
            })
        })
        .collect()
}

fn byte_index_for_line_x(
    text: &str,
    line_range: Range<usize>,
    target_x: f32,
    metrics: TextInputLayoutMetrics,
) -> usize {
    let mut x = 0.0;
    let mut previous = line_range.start;
    for (byte_offset, character) in text[line_range.clone()].char_indices() {
        let byte_index = line_range.start + byte_offset;
        let advance = text_input_char_advance(character, metrics);
        if target_x < x + advance * 0.5 {
            return previous;
        }
        x += advance;
        previous = byte_index + character.len_utf8();
    }
    line_range.end
}

fn text_input_prefix_width(text: &str, metrics: TextInputLayoutMetrics) -> f32 {
    text.chars()
        .map(|character| text_input_char_advance(character, metrics))
        .sum()
}

fn text_input_char_advance(character: char, metrics: TextInputLayoutMetrics) -> f32 {
    sanitize_positive_dimension(
        text_input_char_advance_for_font_size(character, metrics.font_size()),
        metrics.char_width,
    )
}

fn text_input_char_advance_for_font_size(character: char, font_size: f32) -> f32 {
    let factor = match character {
        '\t' => 2.0,
        ' ' => 0.32,
        'i' | 'l' | 'I' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' | '`' => 0.28,
        'j' | 'r' | 't' | 'f' => 0.36,
        'm' | 'w' | 'M' | 'W' => 0.86,
        'A'..='Z' => 0.68,
        '0'..='9' => 0.56,
        _ => 0.50,
    };
    sanitize_positive_dimension(
        font_size * factor,
        font_size * TEXT_INPUT_APPROX_CHAR_WIDTH_FACTOR,
    )
}

fn text_line_ranges(text: &str) -> Vec<(usize, Range<usize>)> {
    let mut ranges = Vec::new();
    let mut line = 0;
    let mut start = 0;
    for (byte_index, character) in text.char_indices() {
        if character == '\n' {
            ranges.push((line, start..byte_index));
            line += 1;
            start = byte_index + character.len_utf8();
        }
    }
    ranges.push((line, start..text.len()));
    ranges
}

fn sanitize_positive_dimension(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback.max(1.0)
    }
}

fn logical_rect_from_ui_rect(rect: UiRect) -> LogicalRect {
    LogicalRect::new(rect.x, rect.y, rect.width, rect.height)
}

fn clamp_to_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selectable_text_preserves_edit_action_for_selection_events() {
        let mut document = UiDocument::new(root_style(240.0, 120.0));
        let root = document.root;
        let options = TextInputOptions {
            edit_action: Some("selectable.edit".into()),
            ..Default::default()
        };
        let node = selectable_text(
            &mut document,
            root,
            "selectable",
            &TextInputState::new("Selectable"),
            options,
        );

        assert_eq!(
            document.node(node).action.as_ref(),
            Some(&WidgetActionBinding::action("selectable.edit"))
        );
        let accessibility = document.node(node).accessibility.as_ref().unwrap();
        assert!(accessibility.read_only);
        assert!(accessibility.focusable);
    }

    #[test]
    fn text_input_paints_selection_only_when_focused() {
        let mut state = TextInputState::new("Editable text");
        state.set_selection(0, "Editable".len());

        let mut document = UiDocument::new(root_style(320.0, 120.0));
        let root = document.root;
        let input = text_input(
            &mut document,
            root,
            "unfocused",
            &state,
            TextInputOptions::default(),
        );
        let text_layer = document.node(input).children[0];
        let UiContent::Scene(primitives) = &document.node(text_layer).content else {
            panic!("text input should render its content through a scene");
        };
        assert!(
            primitives
                .iter()
                .all(|primitive| !matches!(primitive, ScenePrimitive::Rect(_))),
            "unfocused text inputs must not display a retained selection"
        );

        let mut document = UiDocument::new(root_style(320.0, 120.0));
        let root = document.root;
        let input = text_input(
            &mut document,
            root,
            "focused",
            &state,
            TextInputOptions {
                focused: true,
                ..Default::default()
            },
        );
        let text_layer = document.node(input).children[0];
        let UiContent::Scene(primitives) = &document.node(text_layer).content else {
            panic!("text input should render its content through a scene");
        };
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, ScenePrimitive::Rect(_))),
            "focused text inputs should paint their active selection/caret"
        );
    }

    #[test]
    fn text_input_selection_rects_follow_text_width_not_input_width() {
        let mut state = TextInputState::new("Editable text");
        state.set_selection(0, state.text().len());
        let metrics = TextInputLayoutMetrics::new(UiRect::new(0.0, 0.0, 300.0, 24.0), 8.0, 20.0);

        let selection = state.selection_rects(metrics);

        assert_eq!(selection.len(), 1);
        assert!(
            selection[0].rect.width < metrics.text_rect.width * 0.5,
            "selection should end at the selected glyphs, not the input edge"
        );
    }

    #[test]
    fn text_input_render_plan_uses_non_wrapping_text_geometry() {
        let mut state = TextInputState::new("iiiwww");
        state.set_selection(0, state.text().len());
        let text_style = TextStyle {
            wrap: TextWrap::Word,
            ..Default::default()
        };
        let metrics =
            TextInputLayoutMetrics::from_style(UiRect::new(0.0, 0.0, 360.0, 24.0), &text_style);

        let plan = state.render_plan(
            metrics,
            text_style,
            TextInputPaintOptions {
                show_caret: false,
                ..Default::default()
            },
        );

        assert_eq!(plan.text.style.wrap, TextWrap::None);
        assert_eq!(plan.selection_rects.len(), 1);
        assert!(
            plan.selection_rects[0].rect.width < metrics.text_rect.width * 0.5,
            "selection should use the rendered glyph span, not trailing empty input space"
        );
    }

    #[test]
    fn text_input_widget_selection_uses_same_no_wrap_style_as_painted_text() {
        let mut state = TextInputState::new("selected text");
        state.set_selection(0, state.text().len());
        let text_style = TextStyle {
            wrap: TextWrap::Word,
            ..Default::default()
        };
        let mut document = UiDocument::new(root_style(420.0, 100.0));
        let root = document.root;

        let input = text_input(
            &mut document,
            root,
            "selected",
            &state,
            TextInputOptions {
                focused: true,
                caret_visible: false,
                text_style,
                ..Default::default()
            },
        );
        let text_layer = document.node(input).children[0];
        let UiContent::Scene(primitives) = &document.node(text_layer).content else {
            panic!("text input should render its text and selection through a scene");
        };

        let text = primitives
            .iter()
            .find_map(|primitive| match primitive {
                ScenePrimitive::Text(text) => Some(text),
                _ => None,
            })
            .expect("painted text");
        let selection = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                ScenePrimitive::Rect(rect) => Some(rect),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(text.style.wrap, TextWrap::None);
        assert_eq!(selection.len(), 1);
        assert!(
            selection[0].rect.width < text.rect.width,
            "selection highlight should not fill invisible trailing text rect space"
        );
    }

    #[test]
    fn text_input_publishes_intrinsic_width_for_its_text() {
        let state = TextInputState::new("999999999999999999");
        let mut document = UiDocument::new(root_style(360.0, 120.0));
        let root = document.root;

        let input = text_input(
            &mut document,
            root,
            "wide",
            &state,
            TextInputOptions {
                layout: LayoutStyle::new().with_width(40.0).with_height(30.0),
                ..Default::default()
            },
        );
        document
            .compute_layout(UiSize::new(360.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let rect = document.node(input).layout().rect;
        assert!(
            rect.width > 40.0,
            "text input should grow beyond an undersized explicit width when its text requires it: {rect:?}"
        );
    }

    #[test]
    fn text_input_multiline_selection_uses_each_line_content_width() {
        let text = "tiny\nmuch wider line";
        let mut state = TextInputState::new(text).multiline(true);
        state.set_selection(0, state.text().len());
        let metrics = TextInputLayoutMetrics::from_style(
            UiRect::new(0.0, 0.0, 420.0, 80.0),
            &TextStyle::default(),
        );

        let plan = state.render_plan(
            metrics,
            TextStyle::default(),
            TextInputPaintOptions {
                show_caret: false,
                ..Default::default()
            },
        );

        assert_eq!(plan.selection_rects.len(), 2);
        assert_eq!(plan.selection_rects[0].byte_range, 0.."tiny".len());
        assert_eq!(
            plan.selection_rects[1].byte_range,
            "tiny\n".len()..text.len()
        );
        assert!(
            plan.selection_rects[0].rect.width < plan.selection_rects[1].rect.width,
            "each selected line should keep its own shaped width"
        );
        assert!(
            plan.selection_rects[1].rect.width < metrics.text_rect.width * 0.5,
            "selected lines should not extend to the input edge"
        );
    }

    #[cfg(feature = "text-cosmic")]
    #[test]
    fn text_input_measured_layout_uses_shaped_advances() {
        let style = TextStyle::default();
        let text = "iiiwww";
        let measured = TextInputMeasuredLayout::measure(text, &style).expect("measured layout");
        let narrow_width = measured.x_for_byte(0, "iii".len()).unwrap();
        let full_width = measured.x_for_byte(0, text.len()).unwrap();

        assert!(
            full_width - narrow_width > narrow_width,
            "shaped text metrics should account for glyph advance differences"
        );
    }
}
