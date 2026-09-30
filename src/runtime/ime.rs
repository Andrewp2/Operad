//! Input-method session ownership and geometry shared by both built-in hosts.

use crate::host::{HostInteractionState, HostTextCompositionState};
use crate::platform::{
    LogicalRect, PlatformRequest, TextImeRequest, TextImeSession, TextInputId, TextRange,
};
use crate::{
    TextCompositionEvent, UiDocument, UiInputEvent, UiNode, WidgetActionBinding, WidgetActionKind,
    WidgetTextEdit,
};

#[derive(Debug, Default)]
pub(crate) struct RuntimeTextInput {
    next_id: u64,
    binding: Option<WidgetActionBinding>,
    managed: bool,
}

impl RuntimeTextInput {
    pub(crate) fn application_owned(&mut self) {
        self.managed = false;
        self.binding = None;
    }
    pub(crate) fn can_retain_for(&self, node: &UiNode, state: &HostInteractionState) -> bool {
        !self.managed
            || (node.text_input().is_some()
                && node.action() == self.binding.as_ref()
                && !Self::composition_was_discarded(node, state))
    }

    fn composition_was_discarded(node: &UiNode, state: &HostInteractionState) -> bool {
        // A rendered draft disappearing without a routed commit or empty
        // preedit is an application cancellation, not an ordinary snapshot
        // update. Revoke it before already queued input reaches the model.
        state.text_composition == HostTextCompositionState::Preedit
            && state.text_ime.as_ref().is_some_and(|ime| {
                ime.composition
                    .as_ref()
                    .is_some_and(|range| range.start != range.end)
            })
            && node
                .text_input()
                .is_some_and(|snapshot| snapshot.composition.is_none())
    }

    pub(crate) fn cancellation(
        &mut self,
    ) -> Option<super::session::RuntimeInteractionCancellation> {
        self.managed = false;
        self.binding
            .take()
            .map(|binding| super::session::RuntimeInteractionCancellation {
                binding,
                kind: WidgetActionKind::TextEdit(WidgetTextEdit::new(UiInputEvent::Composition {
                    target: None,
                    event: TextCompositionEvent::Cancel,
                })),
            })
    }

    pub(crate) fn sync(
        &mut self,
        state: &mut HostInteractionState,
        document: &UiDocument,
    ) -> Vec<PlatformRequest> {
        // Explicit sessions from application-owned hosts retain their own
        // surrounding-text and cursor contract until they deactivate.
        if !self.managed && state.text_ime.is_some() {
            return Vec::new();
        }
        if state
            .text_target
            .is_some_and(|target| Self::composition_was_discarded(document.node(target), state))
        {
            state.text_target = None;
        }
        let target = state.focused.filter(|target| {
            let node = document.node(*target);
            node.layout().visible
                && document.node_is_enabled(*target)
                && node.text_input().is_some()
        });
        let mut requests = Vec::new();
        let interrupted = target != state.text_target && state.text_ime.is_some();
        if target != state.text_target {
            if let Some(previous) = state.text_ime.take() {
                requests.push(PlatformRequest::TextIme(TextImeRequest::Deactivate {
                    input: previous.input,
                }));
            }
            state.text_target = None;
            state.text_composition = HostTextCompositionState::Inactive;
            self.binding = None;
            self.managed = false;
        }
        let Some(target) = target else {
            return requests;
        };
        let node = document.node(target);
        let snapshot = node.text_input().unwrap();
        let cursor = document.text_input_cursor_rect(target, snapshot.cursor_rect);
        let input = state
            .text_ime
            .as_ref()
            .map(|session| session.input.clone())
            .unwrap_or_else(|| {
                self.next_id = self
                    .next_id
                    .checked_add(1)
                    .expect("input-method session IDs exhausted");
                TextInputId::new(format!("ime:{}", self.next_id))
            });
        let mut session = TextImeSession::new(
            input,
            LogicalRect::new(cursor.x, cursor.y, cursor.width, cursor.height),
        )
        .surrounding_text(
            snapshot.text.clone(),
            TextRange::new(snapshot.selection.start, snapshot.selection.end),
        )
        .multiline(snapshot.multiline);
        session.composition = snapshot
            .composition
            .as_ref()
            .map(|range| TextRange::new(range.start, range.end));
        session.sensitive = snapshot.sensitive;
        // A newly published draft may come from an application-owned model.
        // Otherwise preserve ordered input: the old snapshot can still contain
        // a draft already committed by an event in the current frame.
        if !interrupted
            && session.composition.is_some()
            && state
                .text_ime
                .as_ref()
                .is_none_or(|previous| previous.composition != session.composition)
        {
            state.text_composition = HostTextCompositionState::from_session(&session);
        }
        if state.text_ime.as_ref() != Some(&session) {
            requests.push(PlatformRequest::TextIme(if state.text_ime.is_some() {
                TextImeRequest::Update(session.clone())
            } else {
                TextImeRequest::Activate(session.clone())
            }));
        }
        state.text_ime = Some(session);
        state.text_target = Some(target);
        self.binding = node.action().cloned();
        self.managed = true;
        requests
    }
}
