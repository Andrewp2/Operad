//! Dialog widget.

use crate::{AccessibilityMeta, AccessibilityRole};

pub use super::surfaces::surface_open_close_animation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogDismissReason {
    EscapeKey,
    OutsidePointer,
    CloseButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogDismissal {
    pub escape_key: bool,
    pub outside_pointer: bool,
    pub close_button: bool,
}

impl DialogDismissal {
    pub const NONE: Self = Self {
        escape_key: false,
        outside_pointer: false,
        close_button: false,
    };

    pub const STANDARD: Self = Self {
        escape_key: true,
        outside_pointer: true,
        close_button: true,
    };

    pub const MODAL: Self = Self {
        escape_key: true,
        outside_pointer: false,
        close_button: true,
    };

    pub const fn allows(self, reason: DialogDismissReason) -> bool {
        match reason {
            DialogDismissReason::EscapeKey => self.escape_key,
            DialogDismissReason::OutsidePointer => self.outside_pointer,
            DialogDismissReason::CloseButton => self.close_button,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogDescriptor {
    pub id: String,
    pub title: String,
    pub modal: bool,
    pub trap_focus: bool,
    pub dismissal: DialogDismissal,
    pub accessibility_hint: Option<String>,
}

impl DialogDescriptor {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            modal: false,
            trap_focus: false,
            dismissal: DialogDismissal::STANDARD,
            accessibility_hint: None,
        }
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self.trap_focus = modal;
        if modal {
            self.dismissal = DialogDismissal::MODAL;
        }
        self
    }

    pub fn trap_focus(mut self, trap_focus: bool) -> Self {
        self.trap_focus = trap_focus;
        self
    }

    pub fn dismissal(mut self, dismissal: DialogDismissal) -> Self {
        self.dismissal = dismissal;
        self
    }

    pub fn accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    pub fn accessibility(&self) -> AccessibilityMeta {
        let label = if self.title.is_empty() {
            self.id.clone()
        } else {
            self.title.clone()
        };
        let hint = self.accessibility_hint.clone().unwrap_or_else(|| {
            let modality = if self.modal { "Modal dialog" } else { "Dialog" };
            if self.dismissal.escape_key {
                format!("{modality}; press Escape to dismiss")
            } else {
                format!("{modality}; dismissal is controlled by the application")
            }
        });
        let meta = AccessibilityMeta::new(AccessibilityRole::Dialog)
            .label(label)
            .hint(hint)
            .focusable();
        if self.modal {
            meta.modal()
        } else {
            meta
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DialogStack {
    pub dialogs: Vec<DialogDescriptor>,
}

impl DialogStack {
    pub fn open(&mut self, dialog: DialogDescriptor) {
        self.close(&dialog.id);
        self.dialogs.push(dialog);
    }

    pub fn close(&mut self, id: &str) -> Option<DialogDescriptor> {
        let index = self.dialogs.iter().position(|dialog| dialog.id == id)?;
        Some(self.dialogs.remove(index))
    }

    pub fn dismiss_top(&mut self, reason: DialogDismissReason) -> Option<DialogDescriptor> {
        let top = self.dialogs.last()?;
        if !top.dismissal.allows(reason) {
            return None;
        }
        self.dialogs.pop()
    }

    pub fn top(&self) -> Option<&DialogDescriptor> {
        self.dialogs.last()
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.dialogs.iter().any(|dialog| dialog.id == id)
    }

    pub fn traps_focus(&self) -> bool {
        self.dialogs
            .iter()
            .any(|dialog| dialog.modal || dialog.trap_focus)
    }
}
