//! Popover widget and overlay frame state.

use crate::accessibility::{
    AccessibilityAdapterRequest, AccessibilityCapabilities, FocusRestoreTarget, FocusTrap,
};
use crate::{AccessibilityMeta, AccessibilityRole, UiDocument, UiNodeId, UiPoint, UiRect, UiSize};

use super::dialog::{DialogDescriptor, DialogDismissReason, DialogStack};
pub use super::surfaces::surface_open_close_animation;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopoverAnchor {
    Node(UiNodeId),
    Rect(UiRect),
    Point(UiPoint),
}

impl PopoverAnchor {
    pub fn resolve(self, document: &UiDocument) -> Option<UiRect> {
        match self {
            Self::Node(id) => document.nodes().get(id.0).map(|node| node.layout.rect),
            Self::Rect(rect) => Some(rect),
            Self::Point(point) => Some(UiRect::new(point.x, point.y, 0.0, 0.0)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverPlacement {
    Top,
    Bottom,
    Left,
    Right,
}

impl PopoverPlacement {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverDismissReason {
    EscapeKey,
    OutsidePointer,
    Toggle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopoverDescriptor {
    pub id: String,
    pub anchor: PopoverAnchor,
    pub placement: PopoverPlacement,
    pub modal: bool,
    pub close_on_outside: bool,
    pub close_on_escape: bool,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl PopoverDescriptor {
    pub fn new(id: impl Into<String>, anchor: PopoverAnchor, placement: PopoverPlacement) -> Self {
        Self {
            id: id.into(),
            anchor,
            placement,
            modal: false,
            close_on_outside: true,
            close_on_escape: true,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    pub fn close_on_outside(mut self, close_on_outside: bool) -> Self {
        self.close_on_outside = close_on_outside;
        self
    }

    pub fn close_on_escape(mut self, close_on_escape: bool) -> Self {
        self.close_on_escape = close_on_escape;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    pub fn accessibility(&self) -> AccessibilityMeta {
        let label = self
            .accessibility_label
            .clone()
            .unwrap_or_else(|| self.id.clone());
        let hint = self.accessibility_hint.clone().unwrap_or_else(|| {
            let dismiss_hint = if self.close_on_escape {
                "; press Escape to dismiss"
            } else {
                "; dismissal is controlled by the application"
            };
            format!(
                "Popover anchored to the {}{}",
                self.placement.as_str(),
                dismiss_hint
            )
        });
        let meta = AccessibilityMeta::new(if self.modal {
            AccessibilityRole::Dialog
        } else {
            AccessibilityRole::Menu
        })
        .label(label)
        .hint(hint)
        .expanded(true)
        .focusable();
        if self.modal {
            meta.modal()
        } else {
            meta
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PopoverState {
    pub current: Option<PopoverDescriptor>,
}

impl PopoverState {
    pub fn open(&mut self, popover: PopoverDescriptor) {
        self.current = Some(popover);
    }

    pub fn close(&mut self) -> Option<PopoverDescriptor> {
        self.current.take()
    }

    pub fn toggle(&mut self, popover: PopoverDescriptor) {
        if self.is_open(&popover.id) {
            self.close();
        } else {
            self.open(popover);
        }
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.current
            .as_ref()
            .is_some_and(|popover| popover.id == id)
    }

    pub fn dismiss_for_outside_pointer(&mut self) -> Option<PopoverDescriptor> {
        self.dismiss(PopoverDismissReason::OutsidePointer)
    }

    pub fn dismiss(&mut self, reason: PopoverDismissReason) -> Option<PopoverDescriptor> {
        let should_close = self.current.as_ref().is_some_and(|popover| match reason {
            PopoverDismissReason::EscapeKey => popover.close_on_escape,
            PopoverDismissReason::OutsidePointer => popover.close_on_outside,
            PopoverDismissReason::Toggle => true,
        });
        should_close.then(|| self.close()).flatten()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlayFrameState {
    pub dialogs: DialogStack,
    pub popover: PopoverState,
    pub focus_trap: Option<FocusTrap>,
}

impl OverlayFrameState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_overlay(&self) -> bool {
        !self.dialogs.dialogs.is_empty() || self.popover.current.is_some()
    }

    pub fn traps_focus(&self) -> bool {
        self.focus_trap.is_some() || self.dialogs.traps_focus() || self.popover_modal()
    }

    pub fn popover_modal(&self) -> bool {
        self.popover
            .current
            .as_ref()
            .is_some_and(|popover| popover.modal)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OverlayFrameEvent {
    OpenDialog {
        dialog: DialogDescriptor,
        focus_trap: Option<FocusTrap>,
    },
    CloseDialog {
        id: String,
    },
    DismissDialog {
        reason: DialogDismissReason,
    },
    OpenPopover {
        popover: PopoverDescriptor,
        focus_trap: Option<FocusTrap>,
    },
    TogglePopover {
        popover: PopoverDescriptor,
    },
    DismissPopover {
        reason: PopoverDismissReason,
    },
    SetFocusTrap(FocusTrap),
    ClearFocusTrap {
        restore: FocusRestoreTarget,
    },
}

impl OverlayFrameEvent {
    pub fn open_dialog(dialog: DialogDescriptor) -> Self {
        Self::OpenDialog {
            dialog,
            focus_trap: None,
        }
    }

    pub fn open_dialog_with_focus_trap(dialog: DialogDescriptor, focus_trap: FocusTrap) -> Self {
        Self::OpenDialog {
            dialog,
            focus_trap: Some(focus_trap),
        }
    }

    pub fn close_dialog(id: impl Into<String>) -> Self {
        Self::CloseDialog { id: id.into() }
    }

    pub const fn dismiss_dialog(reason: DialogDismissReason) -> Self {
        Self::DismissDialog { reason }
    }

    pub fn open_popover(popover: PopoverDescriptor) -> Self {
        Self::OpenPopover {
            popover,
            focus_trap: None,
        }
    }

    pub fn open_popover_with_focus_trap(popover: PopoverDescriptor, focus_trap: FocusTrap) -> Self {
        Self::OpenPopover {
            popover,
            focus_trap: Some(focus_trap),
        }
    }

    pub fn toggle_popover(popover: PopoverDescriptor) -> Self {
        Self::TogglePopover { popover }
    }

    pub const fn dismiss_popover(reason: PopoverDismissReason) -> Self {
        Self::DismissPopover { reason }
    }

    pub const fn set_focus_trap(focus_trap: FocusTrap) -> Self {
        Self::SetFocusTrap(focus_trap)
    }

    pub const fn clear_focus_trap(restore: FocusRestoreTarget) -> Self {
        Self::ClearFocusTrap { restore }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayFrameRequest {
    pub state: OverlayFrameState,
    pub events: Vec<OverlayFrameEvent>,
    pub accessibility_capabilities: AccessibilityCapabilities,
}

impl OverlayFrameRequest {
    pub fn new(state: OverlayFrameState) -> Self {
        Self {
            state,
            events: Vec::new(),
            accessibility_capabilities: AccessibilityCapabilities::NONE,
        }
    }

    pub fn event(mut self, event: OverlayFrameEvent) -> Self {
        self.events.push(event);
        self
    }

    pub fn events(mut self, events: impl IntoIterator<Item = OverlayFrameEvent>) -> Self {
        self.events.extend(events);
        self
    }

    pub const fn accessibility_capabilities(
        mut self,
        capabilities: AccessibilityCapabilities,
    ) -> Self {
        self.accessibility_capabilities = capabilities;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlayFrameOutput {
    pub state: OverlayFrameState,
    pub dismissed_dialogs: Vec<DialogDescriptor>,
    pub dismissed_popover: Option<PopoverDescriptor>,
    pub accessibility_requests: Vec<AccessibilityAdapterRequest>,
    pub changed: bool,
}

pub fn process_overlay_frame(request: OverlayFrameRequest) -> OverlayFrameOutput {
    let OverlayFrameRequest {
        mut state,
        events,
        accessibility_capabilities,
    } = request;
    let mut output = OverlayFrameOutput {
        state: OverlayFrameState::new(),
        dismissed_dialogs: Vec::new(),
        dismissed_popover: None,
        accessibility_requests: Vec::new(),
        changed: false,
    };

    for event in events {
        apply_overlay_event(&mut state, &mut output, event, accessibility_capabilities);
    }

    output.state = state;
    output
}

fn apply_overlay_event(
    state: &mut OverlayFrameState,
    output: &mut OverlayFrameOutput,
    event: OverlayFrameEvent,
    capabilities: AccessibilityCapabilities,
) {
    match event {
        OverlayFrameEvent::OpenDialog { dialog, focus_trap } => {
            state.dialogs.open(dialog);
            output.changed = true;
            if let Some(focus_trap) = focus_trap {
                set_overlay_focus_trap(state, output, focus_trap, capabilities);
            }
        }
        OverlayFrameEvent::CloseDialog { id } => {
            if let Some(dialog) = state.dialogs.close(&id) {
                output.dismissed_dialogs.push(dialog);
                output.changed = true;
            }
        }
        OverlayFrameEvent::DismissDialog { reason } => {
            if let Some(dialog) = state.dialogs.dismiss_top(reason) {
                output.dismissed_dialogs.push(dialog);
                output.changed = true;
            }
        }
        OverlayFrameEvent::OpenPopover {
            popover,
            focus_trap,
        } => {
            state.popover.open(popover);
            output.changed = true;
            if let Some(focus_trap) = focus_trap {
                set_overlay_focus_trap(state, output, focus_trap, capabilities);
            }
        }
        OverlayFrameEvent::TogglePopover { popover } => {
            let was_open = state.popover.is_open(&popover.id);
            let dismissed = was_open.then(|| state.popover.current.clone()).flatten();
            state.popover.toggle(popover);
            output.dismissed_popover = dismissed;
            output.changed = true;
        }
        OverlayFrameEvent::DismissPopover { reason } => {
            if let Some(popover) = state.popover.dismiss(reason) {
                output.dismissed_popover = Some(popover);
                output.changed = true;
            }
        }
        OverlayFrameEvent::SetFocusTrap(focus_trap) => {
            set_overlay_focus_trap(state, output, focus_trap, capabilities);
        }
        OverlayFrameEvent::ClearFocusTrap { restore } => {
            clear_overlay_focus_trap(state, output, restore, capabilities);
        }
    }
}

fn set_overlay_focus_trap(
    state: &mut OverlayFrameState,
    output: &mut OverlayFrameOutput,
    focus_trap: FocusTrap,
    capabilities: AccessibilityCapabilities,
) {
    if state.focus_trap == Some(focus_trap) {
        return;
    }
    state.focus_trap = Some(focus_trap);
    output.changed = true;
    push_accessibility_request(
        &mut output.accessibility_requests,
        capabilities,
        AccessibilityAdapterRequest::SetFocusTrap(focus_trap),
    );
}

fn clear_overlay_focus_trap(
    state: &mut OverlayFrameState,
    output: &mut OverlayFrameOutput,
    restore: FocusRestoreTarget,
    capabilities: AccessibilityCapabilities,
) {
    if state.focus_trap.take().is_none() {
        return;
    }
    output.changed = true;
    push_accessibility_request(
        &mut output.accessibility_requests,
        capabilities,
        AccessibilityAdapterRequest::ClearFocusTrap { restore },
    );
}

fn push_accessibility_request(
    requests: &mut Vec<AccessibilityAdapterRequest>,
    capabilities: AccessibilityCapabilities,
    request: AccessibilityAdapterRequest,
) {
    if capabilities.supports(request.kind()) {
        requests.push(request);
    }
}

pub fn resolve_popover_rect(
    anchor: UiRect,
    popover_size: UiSize,
    viewport: UiRect,
    placement: PopoverPlacement,
    offset: f32,
) -> UiRect {
    let offset = if offset.is_finite() {
        offset.max(0.0)
    } else {
        0.0
    };
    let popover_size = UiSize::new(
        if popover_size.width.is_finite() {
            popover_size.width.max(0.0)
        } else {
            0.0
        },
        if popover_size.height.is_finite() {
            popover_size.height.max(0.0)
        } else {
            0.0
        },
    );
    let mut rect = match placement {
        PopoverPlacement::Top => UiRect::new(
            anchor.x,
            anchor.y - popover_size.height - offset,
            popover_size.width,
            popover_size.height,
        ),
        PopoverPlacement::Bottom => UiRect::new(
            anchor.x,
            anchor.bottom() + offset,
            popover_size.width,
            popover_size.height,
        ),
        PopoverPlacement::Left => UiRect::new(
            anchor.x - popover_size.width - offset,
            anchor.y,
            popover_size.width,
            popover_size.height,
        ),
        PopoverPlacement::Right => UiRect::new(
            anchor.right() + offset,
            anchor.y,
            popover_size.width,
            popover_size.height,
        ),
    };
    rect.x = clamp_to_viewport(rect.x, rect.width, viewport.x, viewport.right());
    rect.y = clamp_to_viewport(rect.y, rect.height, viewport.y, viewport.bottom());
    rect
}

fn clamp_to_viewport(value: f32, extent: f32, min: f32, max: f32) -> f32 {
    let min = if min.is_finite() { min } else { 0.0 };
    let max = if max.is_finite() { max.max(min) } else { min };
    let extent = if extent.is_finite() {
        extent.max(0.0)
    } else {
        0.0
    };
    let value = if value.is_finite() { value } else { min };
    let upper = (max - extent).max(min);
    value.clamp(min, upper)
}
