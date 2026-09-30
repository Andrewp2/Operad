//! Modal focus lifetime and preparation-time text-field notifications.

use crate::accessibility::FocusRestoreTarget;
use crate::core::identity::{NodeIdentity, NodeIdentityIndex};
use crate::host::{document_focus_transition_event, HostInputEvent};
use crate::{FocusDirection, UiDocument, UiFocusState, UiNodeId};

#[derive(Debug)]
struct ModalFocus {
    scope: NodeIdentity,
    previous: Option<NodeIdentity>,
    restore: Option<NodeIdentity>,
}

impl ModalFocus {
    fn refresh(&mut self, document: &UiDocument, identities: &NodeIdentityIndex) {
        // Disappearing or ambiguous targets end their lifetime even if the same
        // path is authored again before the dialog closes.
        self.previous = self
            .previous
            .take()
            .filter(|key| identities.by_identity.contains_key(key));
        self.restore = self
            .restore
            .take()
            .filter(|key| identities.by_identity.contains_key(key));
        if let Some(meta) = identities
            .by_identity
            .get(&self.scope)
            .and_then(|id| document.node(*id).accessibility.as_ref())
        {
            self.restore = match meta.modal_focus_restore {
                FocusRestoreTarget::Previous => self.previous.clone(),
                FocusRestoreTarget::None => None,
                FocusRestoreTarget::Node(id) => identity(identities, Some(id)),
            };
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct FocusLifecycle {
    modals: Vec<ModalFocus>,
    pending: Vec<FocusTransition>,
}

#[derive(Debug)]
struct FocusTransition {
    hovered: Option<NodeIdentity>,
    pressed: Option<NodeIdentity>,
    previous: Option<NodeIdentity>,
    current: Option<NodeIdentity>,
}

fn identity(identities: &NodeIdentityIndex, id: Option<UiNodeId>) -> Option<NodeIdentity> {
    identities.by_node.get(id?.index())?.clone()
}

impl FocusLifecycle {
    pub(super) fn reconcile(
        &mut self,
        document: &UiDocument,
        identities: &NodeIdentityIndex,
        previous: &UiFocusState,
        requested: Option<UiNodeId>,
        authored: bool,
    ) -> Option<UiNodeId> {
        let scope = document.accessibility_modal_scope();
        let scope_identity = identity(identities, scope);
        for modal in &mut self.modals {
            modal.refresh(document, identities);
        }
        let mut focus = requested;
        let mut restored = None;
        let mut closed = false;
        if let Some(index) = scope_identity
            .as_ref()
            .and_then(|scope| self.modals.iter().position(|modal| &modal.scope == scope))
        {
            while self.modals.len() > index + 1 {
                restored = self.modals.pop().unwrap().restore;
                closed = true;
            }
        } else {
            // Replacing a removed dialog should preserve its return destination
            // without accumulating a history entry for every replacement.
            while self.modals.last().is_some_and(|modal| {
                scope.is_none()
                    || !identities
                        .by_identity
                        .get(&modal.scope)
                        .is_some_and(|id| document.node_is_available_modal(*id))
            }) {
                restored = self.modals.pop().unwrap().restore;
                closed = true;
            }
            if let Some(scope) = scope_identity {
                let previous = identity(identities, previous.focused).or_else(|| restored.clone());
                let mut modal = ModalFocus {
                    scope,
                    previous,
                    restore: None,
                };
                modal.refresh(document, identities);
                self.modals.push(modal);
            } else if scope.is_some() {
                // Ambiguous modal names cannot own retained focus history.
                self.modals.clear();
            }
        }
        if closed && !authored {
            focus = restored
                .and_then(|key| identities.by_identity.get(&key).copied())
                .filter(|id| document.is_focus_navigation_candidate(*id, scope));
        }
        if let Some(scope) = scope {
            if !focus.is_some_and(|id| document.is_focus_navigation_candidate(id, Some(scope))) {
                focus = document.next_focus(None, FocusDirection::Next);
            }
        }
        if focus != previous.focused {
            self.pending.push(FocusTransition {
                hovered: identity(identities, previous.hovered),
                pressed: identity(identities, previous.pressed),
                previous: identity(identities, previous.focused),
                current: identity(identities, focus),
            });
        }
        focus
    }

    pub(super) fn will_cancel_composition(
        &self,
        document: &UiDocument,
        identities: &NodeIdentityIndex,
        target: UiNodeId,
        binding: &crate::WidgetActionBinding,
    ) -> bool {
        let Some(key) = identities
            .by_node
            .get(target.index())
            .and_then(Option::as_ref)
        else {
            return false;
        };
        document.node_is_text_control(target)
            && document.node(target).action.as_ref() == Some(binding)
            && self.pending.iter().any(|transition| {
                transition.previous.as_ref() == Some(key)
                    && transition.previous != transition.current
            })
    }

    pub(super) fn take_events(
        &mut self,
        document: &UiDocument,
        identities: &NodeIdentityIndex,
    ) -> Vec<HostInputEvent> {
        self.pending
            .drain(..)
            .filter_map(|transition| {
                let resolve = |key: Option<NodeIdentity>| {
                    key.and_then(|key| identities.by_identity.get(&key).copied())
                };
                document_focus_transition_event(
                    document,
                    UiFocusState {
                        hovered: resolve(transition.hovered),
                        pressed: resolve(transition.pressed),
                        focused: resolve(transition.previous),
                    },
                    resolve(transition.current),
                )
            })
            .collect()
    }
}
