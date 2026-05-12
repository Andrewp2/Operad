//! Shared overlay stack model for popovers, menus, dialogs, and tooltips.
//!
//! This module decides stack ordering, hit routing, dismissal, focus restore,
//! and modal blocking without depending on any renderer backend.

use crate::{UiNodeId, UiPoint, UiRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverlayId(pub u64);

impl OverlayId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Popover,
    Menu,
    Dialog,
    Tooltip,
    ContextMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayDismissReason {
    Escape,
    OutsidePointer,
    ParentClosed,
    Programmatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayDismissPolicy {
    pub escape: bool,
    pub outside_pointer: bool,
    pub parent_close: bool,
}

impl OverlayDismissPolicy {
    pub const NONE: Self = Self {
        escape: false,
        outside_pointer: false,
        parent_close: false,
    };

    pub const fn dismissible() -> Self {
        Self {
            escape: true,
            outside_pointer: true,
            parent_close: true,
        }
    }

    pub const fn tooltip() -> Self {
        Self {
            escape: false,
            outside_pointer: false,
            parent_close: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayFocusRestoreTarget {
    Node(UiNodeId),
    OverlayTrigger(OverlayId),
    Logical(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayFocusRestoreRecord {
    pub overlay: OverlayId,
    pub target: OverlayFocusRestoreTarget,
    pub reason: OverlayDismissReason,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayEntry {
    pub id: OverlayId,
    pub kind: OverlayKind,
    pub parent: Option<OverlayId>,
    pub modal: bool,
    pub dismiss_policy: OverlayDismissPolicy,
    pub layer: i32,
    pub bounds: UiRect,
    pub focus_restore: Option<OverlayFocusRestoreTarget>,
    order: u64,
}

impl OverlayEntry {
    pub const fn new(id: OverlayId, kind: OverlayKind, bounds: UiRect) -> Self {
        Self {
            id,
            kind,
            parent: None,
            modal: false,
            dismiss_policy: OverlayDismissPolicy::dismissible(),
            layer: 0,
            bounds,
            focus_restore: None,
            order: 0,
        }
    }

    pub const fn parent(mut self, parent: OverlayId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub const fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    pub const fn dismiss_policy(mut self, dismiss_policy: OverlayDismissPolicy) -> Self {
        self.dismiss_policy = dismiss_policy;
        self
    }

    pub const fn layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    pub fn focus_restore(mut self, target: OverlayFocusRestoreTarget) -> Self {
        self.focus_restore = Some(target);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayHitTestDecision {
    pub hit: Option<OverlayId>,
    pub blocked_by_modal: Option<OverlayId>,
    pub dismiss: Vec<OverlayId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayDismissOutcome {
    pub dismissed: Vec<OverlayId>,
    pub focus_restore: Vec<OverlayFocusRestoreRecord>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlayStack {
    entries: Vec<OverlayEntry>,
    next_order: u64,
}

impl OverlayStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, mut entry: OverlayEntry) {
        entry.order = self.next_order;
        self.next_order += 1;
        self.entries.retain(|existing| existing.id != entry.id);
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[OverlayEntry] {
        &self.entries
    }

    pub fn get(&self, id: OverlayId) -> Option<&OverlayEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn topmost(&self) -> Option<OverlayId> {
        self.entries
            .iter()
            .max_by_key(|entry| self.order_key(entry))
            .map(|entry| entry.id)
    }

    pub fn topmost_at(&self, point: UiPoint) -> Option<OverlayId> {
        self.entries
            .iter()
            .filter(|entry| entry.bounds.contains_point(point))
            .max_by_key(|entry| self.order_key(entry))
            .map(|entry| entry.id)
    }

    pub fn route_pointer_down(&self, point: UiPoint) -> OverlayHitTestDecision {
        let hit = self.topmost_at(point);
        let blocked_by_modal = self
            .topmost_modal()
            .filter(|modal| hit.is_none_or(|hit| !self.is_descendant_or_self(hit, modal.id)))
            .map(|modal| modal.id);

        let dismiss = if blocked_by_modal.is_some() {
            Vec::new()
        } else {
            self.outside_pointer_dismissals(hit)
        };

        OverlayHitTestDecision {
            hit,
            blocked_by_modal,
            dismiss,
        }
    }

    pub fn dismiss_escape(&mut self) -> OverlayDismissOutcome {
        let Some(topmost) = self
            .entries
            .iter()
            .filter(|entry| entry.dismiss_policy.escape)
            .max_by_key(|entry| self.order_key(entry))
            .map(|entry| entry.id)
        else {
            return OverlayDismissOutcome {
                dismissed: Vec::new(),
                focus_restore: Vec::new(),
            };
        };
        self.dismiss(topmost, OverlayDismissReason::Escape)
    }

    pub fn dismiss(
        &mut self,
        id: OverlayId,
        reason: OverlayDismissReason,
    ) -> OverlayDismissOutcome {
        let dismissed = self.dismissal_closure(id, reason);
        let focus_restore = dismissed
            .iter()
            .filter_map(|overlay| {
                self.get(*overlay).and_then(|entry| {
                    entry
                        .focus_restore
                        .clone()
                        .map(|target| OverlayFocusRestoreRecord {
                            overlay: *overlay,
                            target,
                            reason,
                        })
                })
            })
            .collect::<Vec<_>>();

        self.entries
            .retain(|entry| !dismissed.iter().any(|id| *id == entry.id));

        OverlayDismissOutcome {
            dismissed,
            focus_restore,
        }
    }

    fn topmost_modal(&self) -> Option<&OverlayEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.modal)
            .max_by_key(|entry| self.order_key(entry))
    }

    fn outside_pointer_dismissals(&self, hit: Option<OverlayId>) -> Vec<OverlayId> {
        let mut dismiss = Vec::new();
        for entry in self.entries_by_topmost() {
            if Some(entry.id) == hit || hit.is_some_and(|hit| self.is_ancestor(entry.id, hit)) {
                break;
            }
            if entry.dismiss_policy.outside_pointer {
                dismiss.push(entry.id);
            }
        }
        dismiss
    }

    fn dismissal_closure(&self, id: OverlayId, reason: OverlayDismissReason) -> Vec<OverlayId> {
        let mut dismissed = Vec::new();
        self.collect_dismissal(id, reason, &mut dismissed);
        dismissed.sort_by_key(|id| {
            self.get(*id)
                .map(|entry| std::cmp::Reverse(self.order_key(entry)))
        });
        dismissed
    }

    fn collect_dismissal(
        &self,
        id: OverlayId,
        reason: OverlayDismissReason,
        dismissed: &mut Vec<OverlayId>,
    ) {
        if dismissed.contains(&id) {
            return;
        }
        dismissed.push(id);
        for child in self.entries.iter().filter(|entry| entry.parent == Some(id)) {
            if reason != OverlayDismissReason::ParentClosed || child.dismiss_policy.parent_close {
                self.collect_dismissal(child.id, OverlayDismissReason::ParentClosed, dismissed);
            }
        }
    }

    fn entries_by_topmost(&self) -> Vec<&OverlayEntry> {
        let mut entries = self.entries.iter().collect::<Vec<_>>();
        entries.sort_by_key(|entry| std::cmp::Reverse(self.order_key(entry)));
        entries
    }

    fn is_descendant_or_self(&self, id: OverlayId, ancestor: OverlayId) -> bool {
        id == ancestor || self.is_ancestor(ancestor, id)
    }

    fn is_ancestor(&self, ancestor: OverlayId, mut child: OverlayId) -> bool {
        while let Some(entry) = self.get(child) {
            let Some(parent) = entry.parent else {
                return false;
            };
            if parent == ancestor {
                return true;
            }
            child = parent;
        }
        false
    }

    fn order_key(&self, entry: &OverlayEntry) -> (i32, u64) {
        (entry.layer, entry.order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UiPoint;

    fn id(value: u64) -> OverlayId {
        OverlayId::new(value)
    }

    fn rect(x: f32, y: f32, width: f32, height: f32) -> UiRect {
        UiRect::new(x, y, width, height)
    }

    #[test]
    fn overlay_escape_dismisses_topmost_dismissible_overlay() {
        let mut stack = OverlayStack::new();
        stack.push(OverlayEntry::new(
            id(1),
            OverlayKind::Popover,
            rect(0.0, 0.0, 50.0, 50.0),
        ));
        stack.push(OverlayEntry::new(
            id(2),
            OverlayKind::Menu,
            rect(5.0, 5.0, 20.0, 20.0),
        ));

        let outcome = stack.dismiss_escape();

        assert_eq!(outcome.dismissed, vec![id(2)]);
        assert_eq!(stack.topmost(), Some(id(1)));
    }

    #[test]
    fn overlay_nested_child_is_topmost_at_same_layer() {
        let mut stack = OverlayStack::new();
        stack.push(OverlayEntry::new(
            id(1),
            OverlayKind::Menu,
            rect(0.0, 0.0, 100.0, 100.0),
        ));
        stack.push(
            OverlayEntry::new(id(2), OverlayKind::Menu, rect(10.0, 10.0, 20.0, 20.0)).parent(id(1)),
        );

        assert_eq!(stack.topmost(), Some(id(2)));
        assert_eq!(stack.topmost_at(UiPoint::new(12.0, 12.0)), Some(id(2)));
    }

    #[test]
    fn overlay_outside_click_dismisses_topmost_outside_overlay() {
        let mut stack = OverlayStack::new();
        stack.push(OverlayEntry::new(
            id(1),
            OverlayKind::Popover,
            rect(0.0, 0.0, 100.0, 100.0),
        ));
        stack.push(
            OverlayEntry::new(id(2), OverlayKind::Menu, rect(110.0, 0.0, 50.0, 50.0)).parent(id(1)),
        );

        let decision = stack.route_pointer_down(UiPoint::new(10.0, 10.0));

        assert_eq!(decision.hit, Some(id(1)));
        assert_eq!(decision.dismiss, vec![id(2)]);
    }

    #[test]
    fn overlay_modal_blocks_hits_behind_it() {
        let mut stack = OverlayStack::new();
        stack.push(OverlayEntry::new(
            id(1),
            OverlayKind::Popover,
            rect(0.0, 0.0, 100.0, 100.0),
        ));
        stack.push(
            OverlayEntry::new(id(2), OverlayKind::Dialog, rect(200.0, 0.0, 100.0, 100.0))
                .modal(true)
                .layer(10),
        );

        let decision = stack.route_pointer_down(UiPoint::new(10.0, 10.0));

        assert_eq!(decision.hit, Some(id(1)));
        assert_eq!(decision.blocked_by_modal, Some(id(2)));
        assert!(decision.dismiss.is_empty());
    }

    #[test]
    fn overlay_dismiss_records_focus_restore_targets() {
        let mut stack = OverlayStack::new();
        stack.push(
            OverlayEntry::new(id(7), OverlayKind::Dialog, rect(0.0, 0.0, 100.0, 100.0))
                .focus_restore(OverlayFocusRestoreTarget::Node(UiNodeId(42))),
        );

        let outcome = stack.dismiss(id(7), OverlayDismissReason::Programmatic);

        assert_eq!(outcome.dismissed, vec![id(7)]);
        assert_eq!(
            outcome.focus_restore,
            vec![OverlayFocusRestoreRecord {
                overlay: id(7),
                target: OverlayFocusRestoreTarget::Node(UiNodeId(42)),
                reason: OverlayDismissReason::Programmatic,
            }]
        );
    }
}
