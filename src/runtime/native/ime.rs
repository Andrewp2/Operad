use std::collections::HashSet;

use crate::input::{RawInputEvent, RawTextCompositionEvent};
use crate::platform::{TextImeSession, TextInputId};
use crate::TextCompositionEvent;

#[derive(Default)]
pub(super) struct NativeTextInput {
    pub session: Option<TextImeSession>,
    enabled: bool,
    composing: bool,
    suppressed_keys: HashSet<winit::keyboard::PhysicalKey>,
}

impl NativeTextInput {
    pub fn window_unfocused(&mut self, timestamp_millis: u64) -> Option<RawInputEvent> {
        // Key releases may go to another window. Do not suppress its next press
        // when this window receives focus again.
        self.suppressed_keys.clear();
        self.event(&winit::event::Ime::Disabled, timestamp_millis)
    }
    /// True means the native context must restart. Ordinary layout/snapshot
    /// updates preserve the OS composition and its candidate selection.
    pub fn configure(&mut self, session: TextImeSession) -> bool {
        let restart = self
            .session
            .as_ref()
            .is_none_or(|old| old.input != session.input)
            || (self.composing && session.composition.is_none());
        if restart {
            self.enabled = false;
            self.composing = false;
        }
        self.session = Some(session);
        restart
    }

    pub fn deactivate(&mut self, input: &TextInputId) -> bool {
        if !self
            .session
            .as_ref()
            .is_some_and(|session| session.input == *input)
        {
            return false;
        }
        self.session = None;
        self.enabled = false;
        self.composing = false;
        true
    }

    pub fn event(
        &mut self,
        event: &winit::event::Ime,
        timestamp_millis: u64,
    ) -> Option<RawInputEvent> {
        use winit::event::Ime;
        let input = self.session.as_ref()?.input.clone();
        let event = match event {
            Ime::Enabled => {
                self.enabled = true;
                return None;
            }
            Ime::Disabled => {
                let was_enabled = self.enabled;
                self.enabled = false;
                self.composing = false;
                if !was_enabled {
                    return None;
                }
                TextCompositionEvent::Cancel
            }
            Ime::Preedit(text, selection) if self.enabled => {
                self.composing = !text.is_empty();
                TextCompositionEvent::Preedit {
                    text: text.clone(),
                    selection: selection.map(|(start, end)| start..end),
                    replacement: None,
                }
            }
            Ime::Commit(text) if self.enabled => {
                self.composing = false;
                TextCompositionEvent::Commit {
                    text: text.clone(),
                    replacement: None,
                }
            }
            _ => return None,
        };
        Some(RawInputEvent::Composition(RawTextCompositionEvent {
            input,
            event,
            timestamp_millis,
        }))
    }

    pub fn owns_key(&mut self, key: winit::keyboard::PhysicalKey, pressed: bool) -> bool {
        if !pressed {
            return self.suppressed_keys.remove(&key) || self.composing;
        }
        if self.composing {
            self.suppressed_keys.insert(key);
            true
        } else {
            self.suppressed_keys.contains(&key)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::LogicalRect;
    use winit::event::Ime;

    fn session(id: &str) -> TextImeSession {
        TextImeSession::new(TextInputId::new(id), LogicalRect::new(1.0, 2.0, 1.0, 20.0))
    }

    #[test]
    fn native_sequence_preserves_preedit_selection_commit_and_candidate_key_ownership() {
        let mut input = NativeTextInput::default();
        input.configure(session("field"));
        input.event(&Ime::Enabled, 0);
        let Some(RawInputEvent::Composition(preedit)) =
            input.event(&Ime::Preedit("候補".into(), Some((3, 6))), 1)
        else {
            panic!()
        };
        assert_eq!(
            preedit.event,
            TextCompositionEvent::Preedit {
                text: "候補".into(),
                selection: Some(3..6),
                replacement: None,
            }
        );
        let enter = winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Enter);
        assert!(input.owns_key(enter, true));
        let mut update = session("field");
        update.composition = Some(crate::platform::TextRange::new(0, 6));
        assert!(
            !input.configure(update),
            "caret updates must not reset the OS candidate list"
        );
        input.event(&Ime::Preedit(String::new(), None), 2).unwrap();
        let Some(RawInputEvent::Composition(commit)) = input.event(&Ime::Commit("候補".into()), 3)
        else {
            panic!()
        };
        assert_eq!(commit.input, preedit.input);
        assert_eq!(
            commit.event,
            TextCompositionEvent::Commit {
                text: "候補".into(),
                replacement: None
            }
        );
        assert!(
            input.owns_key(enter, false),
            "a confirming key must not leak its release to shortcuts"
        );
        assert!(!input.owns_key(enter, true));
        input.event(&Ime::Preedit("draft".into(), Some((5, 5))), 4);
        assert!(input.owns_key(enter, true));
        input.window_unfocused(5);
        input.event(&Ime::Enabled, 6);
        assert!(
            !input.owns_key(enter, true),
            "a lost key release must not swallow the next press after refocusing"
        );
        let Some(RawInputEvent::Composition(cancel)) = input.event(&Ime::Disabled, 4) else {
            panic!()
        };
        assert_eq!(cancel.event, TextCompositionEvent::Cancel);
        assert!(input.event(&Ime::Commit("late".into()), 5).is_none());
    }

    #[test]
    fn changing_fields_rejects_events_until_the_new_native_context_is_enabled() {
        let mut input = NativeTextInput::default();
        input.configure(session("old"));
        input.event(&Ime::Enabled, 0);
        input.event(&Ime::Preedit("old".into(), Some((3, 3))), 1);
        assert!(input.configure(session("new")));
        assert!(input.event(&Ime::Commit("late".into()), 2).is_none());
        assert!(input.event(&Ime::Disabled, 3).is_none());
        input.event(&Ime::Enabled, 4);
        let Some(RawInputEvent::Composition(commit)) = input.event(&Ime::Commit("new".into()), 5)
        else {
            panic!()
        };
        assert_eq!(commit.input, TextInputId::new("new"));
        assert!(!input.deactivate(&TextInputId::new("old")));
        assert!(input.deactivate(&TextInputId::new("new")));
        assert!(input
            .event(&Ime::Preedit("ignored".into(), None), 6)
            .is_none());
    }
}
