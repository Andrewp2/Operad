//! Renderer-neutral widget action events.
//!
//! Widget actions are intentionally semantic and product-neutral. Widgets can
//! bind an interaction either to an application-owned opaque action id or to an
//! existing command id, then enqueue ordered action events for the application
//! layer to interpret.

use std::fmt;

use crate::commands::CommandId;
use crate::input::{DragGesture, GestureEvent, GesturePhase, PointerButton};
use crate::{
    EditPhase, KeyCode, KeyModifiers, ScrollState, UiDocument, UiInputEvent, UiInputResult,
    UiNodeId, UiPoint, UiRect,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetActionId(String);

impl WidgetActionId {
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

impl AsRef<str> for WidgetActionId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for WidgetActionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WidgetActionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&WidgetActionId> for WidgetActionId {
    fn from(value: &WidgetActionId) -> Self {
        value.clone()
    }
}

impl fmt::Display for WidgetActionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidgetActionBinding {
    Action(WidgetActionId),
    Command(CommandId),
}

impl WidgetActionBinding {
    pub fn action(id: impl Into<WidgetActionId>) -> Self {
        Self::Action(id.into())
    }

    pub fn command(id: impl Into<CommandId>) -> Self {
        Self::Command(id.into())
    }

    pub const fn action_id(&self) -> Option<&WidgetActionId> {
        match self {
            Self::Action(id) => Some(id),
            Self::Command(_) => None,
        }
    }

    pub const fn command_id(&self) -> Option<&CommandId> {
        match self {
            Self::Action(_) => None,
            Self::Command(id) => Some(id),
        }
    }
}

impl From<WidgetActionId> for WidgetActionBinding {
    fn from(value: WidgetActionId) -> Self {
        Self::Action(value)
    }
}

impl From<CommandId> for WidgetActionBinding {
    fn from(value: CommandId) -> Self {
        Self::Command(value)
    }
}

impl From<&str> for WidgetActionBinding {
    fn from(value: &str) -> Self {
        Self::action(value)
    }
}

impl From<String> for WidgetActionBinding {
    fn from(value: String) -> Self {
        Self::action(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetActionTrigger {
    Pointer,
    Keyboard,
    Accessibility,
    Programmatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetActivation {
    pub trigger: WidgetActionTrigger,
    pub count: u8,
}

impl WidgetActivation {
    pub const fn new(trigger: WidgetActionTrigger) -> Self {
        Self { trigger, count: 1 }
    }

    pub const fn pointer(count: u8) -> Self {
        Self {
            trigger: WidgetActionTrigger::Pointer,
            count,
        }
    }

    pub const fn keyboard() -> Self {
        Self::new(WidgetActionTrigger::Keyboard)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetSelection {
    pub selected: Option<bool>,
}

impl WidgetSelection {
    pub const ANY: Self = Self { selected: None };

    pub const fn new(selected: bool) -> Self {
        Self {
            selected: Some(selected),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetValueEditPhase {
    Preview,
    Begin,
    Update,
    Commit,
    Cancel,
}

impl WidgetValueEditPhase {
    pub const fn edit_phase(self) -> EditPhase {
        match self {
            Self::Preview => EditPhase::Preview,
            Self::Begin => EditPhase::BeginEdit,
            Self::Update => EditPhase::UpdateEdit,
            Self::Commit => EditPhase::CommitEdit,
            Self::Cancel => EditPhase::CancelEdit,
        }
    }
}

impl From<GesturePhase> for WidgetValueEditPhase {
    fn from(value: GesturePhase) -> Self {
        match value {
            GesturePhase::Preview => Self::Preview,
            GesturePhase::Begin => Self::Begin,
            GesturePhase::Update => Self::Update,
            GesturePhase::Commit => Self::Commit,
            GesturePhase::Cancel => Self::Cancel,
        }
    }
}

impl From<EditPhase> for WidgetValueEditPhase {
    fn from(value: EditPhase) -> Self {
        match value {
            EditPhase::Preview => Self::Preview,
            EditPhase::BeginEdit => Self::Begin,
            EditPhase::UpdateEdit => Self::Update,
            EditPhase::CommitEdit => Self::Commit,
            EditPhase::CancelEdit => Self::Cancel,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetDragPhase {
    Begin,
    Update,
    Commit,
    Cancel,
}

impl TryFrom<GesturePhase> for WidgetDragPhase {
    type Error = ();

    fn try_from(value: GesturePhase) -> Result<Self, Self::Error> {
        match value {
            GesturePhase::Begin => Ok(Self::Begin),
            GesturePhase::Update => Ok(Self::Update),
            GesturePhase::Commit => Ok(Self::Commit),
            GesturePhase::Cancel => Ok(Self::Cancel),
            GesturePhase::Preview => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidgetDrag {
    pub phase: WidgetDragPhase,
    pub origin: UiPoint,
    pub current: UiPoint,
    pub previous: UiPoint,
    pub delta: UiPoint,
    pub total_delta: UiPoint,
}

impl TryFrom<&DragGesture> for WidgetDrag {
    type Error = ();

    fn try_from(value: &DragGesture) -> Result<Self, Self::Error> {
        Ok(Self {
            phase: WidgetDragPhase::try_from(value.phase)?,
            origin: value.origin,
            current: value.current,
            previous: value.previous,
            delta: value.delta,
            total_delta: value.total_delta,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetFocusChange {
    pub focused: bool,
}

impl WidgetFocusChange {
    pub const GAINED: Self = Self { focused: true };
    pub const LOST: Self = Self { focused: false };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidgetPointerEdit {
    pub phase: WidgetValueEditPhase,
    pub position: UiPoint,
    pub local_position: UiPoint,
    pub target_rect: UiRect,
}

impl WidgetPointerEdit {
    pub const fn new(
        phase: WidgetValueEditPhase,
        position: UiPoint,
        local_position: UiPoint,
        target_rect: UiRect,
    ) -> Self {
        Self {
            phase,
            position,
            local_position,
            target_rect,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetTextEdit {
    pub event: UiInputEvent,
    pub phase: WidgetValueEditPhase,
    pub position: Option<UiPoint>,
    pub local_position: Option<UiPoint>,
    pub target_rect: Option<UiRect>,
    pub selecting: bool,
}

impl WidgetTextEdit {
    pub fn new(event: UiInputEvent) -> Self {
        Self {
            event,
            phase: WidgetValueEditPhase::Preview,
            position: None,
            local_position: None,
            target_rect: None,
            selecting: false,
        }
    }

    pub fn pointer(
        event: UiInputEvent,
        phase: WidgetValueEditPhase,
        position: UiPoint,
        local_position: UiPoint,
        target_rect: UiRect,
        selecting: bool,
    ) -> Self {
        Self {
            event,
            phase,
            position: Some(position),
            local_position: Some(local_position),
            target_rect: Some(target_rect),
            selecting,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetActionMode {
    Activate,
    Drag,
    PointerEdit,
    PointerEditParentRect,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetActionKind {
    Activate(WidgetActivation),
    Selection(WidgetSelection),
    ValueEdit(WidgetValueEditPhase),
    Open,
    Close,
    Drag(WidgetDrag),
    PointerEdit(WidgetPointerEdit),
    TextEdit(WidgetTextEdit),
    Scroll(ScrollState),
    Focus(WidgetFocusChange),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetAction {
    pub target: UiNodeId,
    pub binding: WidgetActionBinding,
    pub kind: WidgetActionKind,
}

impl WidgetAction {
    pub fn new(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        kind: WidgetActionKind,
    ) -> Self {
        Self {
            target,
            binding: binding.into(),
            kind,
        }
    }

    pub fn activate(target: UiNodeId, binding: impl Into<WidgetActionBinding>) -> Self {
        Self::new(
            target,
            binding,
            WidgetActionKind::Activate(WidgetActivation::new(WidgetActionTrigger::Programmatic)),
        )
    }

    pub fn pointer_activate(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        count: u8,
    ) -> Self {
        Self::new(
            target,
            binding,
            WidgetActionKind::Activate(WidgetActivation::pointer(count)),
        )
    }

    pub fn keyboard_activate(target: UiNodeId, binding: impl Into<WidgetActionBinding>) -> Self {
        Self::new(
            target,
            binding,
            WidgetActionKind::Activate(WidgetActivation::keyboard()),
        )
    }

    pub fn accessibility_activate(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
    ) -> Self {
        Self::new(
            target,
            binding,
            WidgetActionKind::Activate(WidgetActivation::new(WidgetActionTrigger::Accessibility)),
        )
    }

    pub fn selection(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        selected: Option<bool>,
    ) -> Self {
        Self::new(
            target,
            binding,
            WidgetActionKind::Selection(WidgetSelection { selected }),
        )
    }

    pub fn value_edit(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        phase: impl Into<WidgetValueEditPhase>,
    ) -> Self {
        Self::new(target, binding, WidgetActionKind::ValueEdit(phase.into()))
    }

    pub fn open(target: UiNodeId, binding: impl Into<WidgetActionBinding>) -> Self {
        Self::new(target, binding, WidgetActionKind::Open)
    }

    pub fn close(target: UiNodeId, binding: impl Into<WidgetActionBinding>) -> Self {
        Self::new(target, binding, WidgetActionKind::Close)
    }

    pub fn focus(target: UiNodeId, binding: impl Into<WidgetActionBinding>, focused: bool) -> Self {
        Self::new(
            target,
            binding,
            WidgetActionKind::Focus(WidgetFocusChange { focused }),
        )
    }

    pub fn drag(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        drag: WidgetDrag,
    ) -> Self {
        Self::new(target, binding, WidgetActionKind::Drag(drag))
    }

    pub fn pointer_edit(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        edit: WidgetPointerEdit,
    ) -> Self {
        Self::new(target, binding, WidgetActionKind::PointerEdit(edit))
    }

    pub fn text_edit(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        event: UiInputEvent,
    ) -> Self {
        Self::new(
            target,
            binding,
            WidgetActionKind::TextEdit(WidgetTextEdit::new(event)),
        )
    }

    pub fn text_pointer_edit(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        event: UiInputEvent,
        phase: impl Into<WidgetValueEditPhase>,
        position: UiPoint,
        target_rect: UiRect,
        selecting: bool,
    ) -> Self {
        let local_position = UiPoint::new(position.x - target_rect.x, position.y - target_rect.y);
        Self::new(
            target,
            binding,
            WidgetActionKind::TextEdit(WidgetTextEdit::pointer(
                event,
                phase.into(),
                position,
                local_position,
                target_rect,
                selecting,
            )),
        )
    }

    pub fn scroll(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        scroll: ScrollState,
    ) -> Self {
        Self::new(target, binding, WidgetActionKind::Scroll(scroll))
    }

    pub fn drag_from_gesture(
        gesture: &DragGesture,
        binding: impl Into<WidgetActionBinding>,
    ) -> Option<Self> {
        WidgetDrag::try_from(gesture)
            .ok()
            .map(|drag| Self::drag(gesture.target, binding, drag))
    }

    pub fn value_edit_from_drag(
        gesture: &DragGesture,
        binding: impl Into<WidgetActionBinding>,
    ) -> Self {
        Self::value_edit(
            gesture.target,
            binding,
            WidgetValueEditPhase::from(gesture.phase),
        )
    }

    pub fn activation_from_input_result(
        result: &UiInputResult,
        mut binding_for: impl FnMut(UiNodeId) -> Option<WidgetActionBinding>,
    ) -> Option<Self> {
        let target = result.clicked?;
        let binding = binding_for(target)?;
        Some(Self::pointer_activate(target, binding, 1))
    }

    pub fn activation_from_input_result_for_document(
        document: &UiDocument,
        result: &UiInputResult,
        binding_for: impl FnMut(UiNodeId) -> Option<WidgetActionBinding>,
    ) -> Option<Self> {
        let target = result.clicked?;
        let (target, binding, mode) = resolve_action_target(document, target, binding_for)?;
        if mode != WidgetActionMode::Activate {
            return None;
        }
        Some(Self::pointer_activate(target, binding, 1))
    }

    pub fn activation_from_key(
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<Self> {
        keyboard_activation_key(key, modifiers).then(|| Self::keyboard_activate(target, binding))
    }

    pub fn from_gesture_event(
        event: &GestureEvent,
        mut binding_for: impl FnMut(UiNodeId) -> Option<WidgetActionBinding>,
    ) -> Option<Self> {
        match event {
            GestureEvent::Click(click) if click.button == PointerButton::Primary => {
                let binding = binding_for(click.target)?;
                Some(Self::pointer_activate(click.target, binding, click.count))
            }
            GestureEvent::Drag(gesture) => {
                let binding = binding_for(gesture.target)?;
                Self::drag_from_gesture(gesture, binding)
            }
            _ => None,
        }
    }

    pub fn from_gesture_event_for_document(
        document: &UiDocument,
        event: &GestureEvent,
        binding_for: impl FnMut(UiNodeId) -> Option<WidgetActionBinding>,
    ) -> Option<Self> {
        match event {
            GestureEvent::Click(click) if click.button == PointerButton::Primary => {
                let (target, binding, mode) =
                    resolve_action_target(document, click.target, binding_for)?;
                match mode {
                    WidgetActionMode::Activate => {
                        Some(Self::pointer_activate(target, binding, click.count))
                    }
                    WidgetActionMode::Drag => None,
                    WidgetActionMode::PointerEdit => Some(pointer_edit_action(
                        document,
                        target,
                        target,
                        binding,
                        WidgetValueEditPhase::Commit,
                        click.position,
                    )),
                    WidgetActionMode::PointerEditParentRect => Some(pointer_edit_action(
                        document,
                        target,
                        action_rect_parent(document, target),
                        binding,
                        WidgetValueEditPhase::Commit,
                        click.position,
                    )),
                }
            }
            GestureEvent::Drag(gesture) => {
                let (target, binding, mode) =
                    resolve_action_target(document, gesture.target, binding_for)?;
                match mode {
                    WidgetActionMode::Activate => None,
                    WidgetActionMode::Drag => {
                        let mut gesture = *gesture;
                        gesture.target = target;
                        Self::drag_from_gesture(&gesture, binding)
                    }
                    WidgetActionMode::PointerEdit => Some(pointer_edit_drag_action(
                        document, target, target, binding, gesture,
                    )),
                    WidgetActionMode::PointerEditParentRect => Some(pointer_edit_drag_action(
                        document,
                        target,
                        action_rect_parent(document, target),
                        binding,
                        gesture,
                    )),
                }
            }
            _ => None,
        }
    }
}

fn resolve_action_target(
    document: &UiDocument,
    target: UiNodeId,
    mut binding_for: impl FnMut(UiNodeId) -> Option<WidgetActionBinding>,
) -> Option<(UiNodeId, WidgetActionBinding, WidgetActionMode)> {
    let mut current = Some(target);
    while let Some(candidate) = current {
        let node = document.nodes().get(candidate.0)?;
        if let Some(binding) = binding_for(candidate) {
            return action_target_enabled(document, candidate).then_some((
                candidate,
                binding,
                node.action_mode,
            ));
        }
        current = node.parent;
    }
    None
}

fn pointer_edit_action(
    document: &UiDocument,
    target: UiNodeId,
    rect_source: UiNodeId,
    binding: WidgetActionBinding,
    phase: WidgetValueEditPhase,
    position: UiPoint,
) -> WidgetAction {
    let target_rect = document
        .nodes()
        .get(rect_source.0)
        .map(|node| node.layout.rect)
        .unwrap_or_else(|| UiRect::new(0.0, 0.0, 0.0, 0.0));
    let local_position = UiPoint::new(position.x - target_rect.x, position.y - target_rect.y);
    WidgetAction::pointer_edit(
        target,
        binding,
        WidgetPointerEdit::new(phase, position, local_position, target_rect),
    )
}

fn pointer_edit_drag_action(
    document: &UiDocument,
    target: UiNodeId,
    rect_source: UiNodeId,
    binding: WidgetActionBinding,
    gesture: &DragGesture,
) -> WidgetAction {
    let phase = WidgetValueEditPhase::from(gesture.phase);
    let position = match gesture.phase {
        GesturePhase::Begin => gesture.origin,
        GesturePhase::Preview
        | GesturePhase::Update
        | GesturePhase::Commit
        | GesturePhase::Cancel => gesture.current,
    };
    pointer_edit_action(document, target, rect_source, binding, phase, position)
}

fn action_rect_parent(document: &UiDocument, target: UiNodeId) -> UiNodeId {
    document
        .nodes()
        .get(target.0)
        .and_then(|node| node.parent)
        .unwrap_or(target)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WidgetActionQueue {
    actions: Vec<WidgetAction>,
}

impl WidgetActionQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_actions(actions: impl IntoIterator<Item = WidgetAction>) -> Self {
        Self {
            actions: actions.into_iter().collect(),
        }
    }

    pub fn push(&mut self, action: WidgetAction) -> &mut Self {
        self.actions.push(action);
        self
    }

    pub fn extend(&mut self, actions: impl IntoIterator<Item = WidgetAction>) -> &mut Self {
        self.actions.extend(actions);
        self
    }

    pub fn activate(
        &mut self,
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
    ) -> &mut Self {
        self.push(WidgetAction::activate(target, binding))
    }

    pub fn command_activate(
        &mut self,
        target: UiNodeId,
        command: impl Into<CommandId>,
    ) -> &mut Self {
        self.push(WidgetAction::pointer_activate(
            target,
            WidgetActionBinding::command(command),
            1,
        ))
    }

    pub fn keyboard_activate(
        &mut self,
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
    ) -> &mut Self {
        self.push(WidgetAction::keyboard_activate(target, binding))
    }

    pub fn select(
        &mut self,
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        selected: bool,
    ) -> &mut Self {
        self.push(WidgetAction::selection(target, binding, Some(selected)))
    }

    pub fn value_edit(
        &mut self,
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        phase: impl Into<WidgetValueEditPhase>,
    ) -> &mut Self {
        self.push(WidgetAction::value_edit(target, binding, phase))
    }

    pub fn open(&mut self, target: UiNodeId, binding: impl Into<WidgetActionBinding>) -> &mut Self {
        self.push(WidgetAction::open(target, binding))
    }

    pub fn close(
        &mut self,
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
    ) -> &mut Self {
        self.push(WidgetAction::close(target, binding))
    }

    pub fn focus(
        &mut self,
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        focused: bool,
    ) -> &mut Self {
        self.push(WidgetAction::focus(target, binding, focused))
    }

    pub fn drag(
        &mut self,
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        drag: WidgetDrag,
    ) -> &mut Self {
        self.push(WidgetAction::drag(target, binding, drag))
    }

    pub fn push_input_result(
        &mut self,
        result: &UiInputResult,
        binding_for: impl FnMut(UiNodeId) -> Option<WidgetActionBinding>,
    ) -> &mut Self {
        if let Some(action) = WidgetAction::activation_from_input_result(result, binding_for) {
            self.actions.push(action);
        }
        self
    }

    pub fn push_input_result_for_document(
        &mut self,
        document: &UiDocument,
        result: &UiInputResult,
        binding_for: impl FnMut(UiNodeId) -> Option<WidgetActionBinding>,
    ) -> &mut Self {
        if let Some(action) =
            WidgetAction::activation_from_input_result_for_document(document, result, binding_for)
        {
            self.actions.push(action);
        }
        self
    }

    pub fn push_key_activation(
        &mut self,
        target: UiNodeId,
        binding: impl Into<WidgetActionBinding>,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> &mut Self {
        if let Some(action) = WidgetAction::activation_from_key(target, binding, key, modifiers) {
            self.actions.push(action);
        }
        self
    }

    pub fn push_gesture_event(
        &mut self,
        event: &GestureEvent,
        binding_for: impl FnMut(UiNodeId) -> Option<WidgetActionBinding>,
    ) -> &mut Self {
        if let Some(action) = WidgetAction::from_gesture_event(event, binding_for) {
            self.actions.push(action);
        }
        self
    }

    pub fn push_gesture_event_for_document(
        &mut self,
        document: &UiDocument,
        event: &GestureEvent,
        binding_for: impl FnMut(UiNodeId) -> Option<WidgetActionBinding>,
    ) -> &mut Self {
        if let Some(action) =
            WidgetAction::from_gesture_event_for_document(document, event, binding_for)
        {
            self.actions.push(action);
        }
        self
    }

    pub fn push_drag_value_edit(
        &mut self,
        gesture: &DragGesture,
        binding: impl Into<WidgetActionBinding>,
    ) -> &mut Self {
        self.actions
            .push(WidgetAction::value_edit_from_drag(gesture, binding));
        self
    }

    pub fn as_slice(&self) -> &[WidgetAction] {
        &self.actions
    }

    pub fn iter(&self) -> impl Iterator<Item = &WidgetAction> {
        self.actions.iter()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = WidgetAction> + '_ {
        self.actions.drain(..)
    }

    pub fn into_vec(self) -> Vec<WidgetAction> {
        self.actions
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

pub fn action_target_enabled(document: &UiDocument, target: UiNodeId) -> bool {
    document.nodes().get(target.0).is_some_and(|node| {
        node.accessibility
            .as_ref()
            .is_none_or(|accessibility| accessibility.enabled && !accessibility.hidden)
    })
}

pub const fn keyboard_activation_key(key: KeyCode, modifiers: KeyModifiers) -> bool {
    let plain_or_shift = !modifiers.ctrl && !modifiers.alt && !modifiers.meta;
    plain_or_shift && matches!(key, KeyCode::Enter | KeyCode::Character(' '))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::PointerId;
    use crate::{
        AccessibilityMeta, AccessibilityRole, ApproxTextMeasurer, InputBehavior, LayoutStyle,
        UiInputResult, UiNode, UiSize,
    };

    fn fixed_doc() -> UiDocument {
        UiDocument::new(LayoutStyle::new())
    }

    fn binding(id: &'static str) -> impl FnMut(UiNodeId) -> Option<WidgetActionBinding> {
        move |_| Some(WidgetActionBinding::action(id))
    }

    fn drag(phase: GesturePhase) -> DragGesture {
        DragGesture {
            pointer_id: PointerId::MOUSE,
            target: UiNodeId(7),
            phase,
            origin: UiPoint::new(1.0, 2.0),
            current: UiPoint::new(5.0, 8.0),
            previous: UiPoint::new(3.0, 4.0),
            delta: UiPoint::new(2.0, 4.0),
            total_delta: UiPoint::new(4.0, 6.0),
            button: PointerButton::Primary,
            modifiers: KeyModifiers::NONE,
            captured: true,
            timestamp_millis: 42,
        }
    }

    #[test]
    fn click_input_result_maps_to_pointer_activation() {
        let result = UiInputResult {
            clicked: Some(UiNodeId(2)),
            ..Default::default()
        };

        let action =
            WidgetAction::activation_from_input_result(&result, binding("transport.play")).unwrap();

        assert_eq!(action.target, UiNodeId(2));
        assert_eq!(
            action.binding,
            WidgetActionBinding::action("transport.play")
        );
        assert_eq!(
            action.kind,
            WidgetActionKind::Activate(WidgetActivation::pointer(1))
        );
    }

    #[test]
    fn primary_gesture_click_maps_to_pointer_activation() {
        let click = GestureEvent::Click(crate::PointerClick {
            pointer_id: PointerId::MOUSE,
            target: UiNodeId(3),
            position: UiPoint::new(10.0, 12.0),
            button: PointerButton::Primary,
            count: 2,
            modifiers: KeyModifiers::NONE,
            timestamp_millis: 5,
        });

        let action = WidgetAction::from_gesture_event(&click, binding("button.open")).unwrap();

        assert_eq!(
            action.kind,
            WidgetActionKind::Activate(WidgetActivation::pointer(2))
        );
    }

    #[test]
    fn disabled_accessibility_metadata_suppresses_input_activation() {
        let mut document = fixed_doc();
        let disabled = document.add_child(
            document.root,
            UiNode::container("disabled", LayoutStyle::new())
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Button).disabled()),
        );
        let result = UiInputResult {
            clicked: Some(disabled),
            ..Default::default()
        };
        let mut queue = WidgetActionQueue::new();

        queue.push_input_result_for_document(&document, &result, binding("disabled.action"));

        assert!(queue.is_empty());
    }

    #[test]
    fn activate_mode_ignores_drag_gestures_for_document_actions() {
        let mut document = UiDocument::new(LayoutStyle::size(200.0, 100.0));
        let target = document.add_child(
            document.root,
            UiNode::container("close", LayoutStyle::size(40.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_action("window.close"),
        );
        let drag = GestureEvent::Drag(DragGesture {
            pointer_id: PointerId::MOUSE,
            target,
            phase: GesturePhase::Update,
            origin: UiPoint::new(8.0, 8.0),
            current: UiPoint::new(40.0, 12.0),
            previous: UiPoint::new(20.0, 10.0),
            delta: UiPoint::new(20.0, 2.0),
            total_delta: UiPoint::new(32.0, 4.0),
            button: PointerButton::Primary,
            modifiers: KeyModifiers::NONE,
            captured: true,
            timestamp_millis: 16,
        });

        let action = WidgetAction::from_gesture_event_for_document(&document, &drag, |id| {
            document.nodes()[id.0].action.clone()
        });

        assert!(
            action.is_none(),
            "ordinary activate buttons must not fire when a drag starts on them"
        );
    }

    #[test]
    fn drag_mode_emits_drag_gestures_for_document_actions() {
        let mut document = UiDocument::new(LayoutStyle::size(200.0, 100.0));
        let target = document.add_child(
            document.root,
            UiNode::container("drag_source", LayoutStyle::size(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_action("asset.drag")
                .with_action_mode(WidgetActionMode::Drag),
        );
        let drag = GestureEvent::Drag(DragGesture {
            pointer_id: PointerId::MOUSE,
            target,
            phase: GesturePhase::Update,
            origin: UiPoint::new(8.0, 8.0),
            current: UiPoint::new(40.0, 12.0),
            previous: UiPoint::new(20.0, 10.0),
            delta: UiPoint::new(20.0, 2.0),
            total_delta: UiPoint::new(32.0, 4.0),
            button: PointerButton::Primary,
            modifiers: KeyModifiers::NONE,
            captured: true,
            timestamp_millis: 16,
        });

        let action = WidgetAction::from_gesture_event_for_document(&document, &drag, |id| {
            document.nodes()[id.0].action.clone()
        })
        .expect("drag action");

        assert_eq!(action.target, target);
        assert_eq!(action.binding, WidgetActionBinding::action("asset.drag"));
        assert!(matches!(action.kind, WidgetActionKind::Drag(_)));
    }

    #[test]
    fn pointer_edit_action_carries_local_position_and_target_rect() {
        let mut document = UiDocument::new(LayoutStyle::size(200.0, 100.0));
        let target = document.add_child(
            document.root,
            UiNode::container("field", LayoutStyle::size(100.0, 20.0))
                .with_input(InputBehavior::BUTTON)
                .with_pointer_edit_action("color.field"),
        );
        document
            .compute_layout(UiSize::new(200.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let click = GestureEvent::Click(crate::PointerClick {
            pointer_id: PointerId::MOUSE,
            target,
            position: UiPoint::new(25.0, 8.0),
            button: PointerButton::Primary,
            count: 1,
            modifiers: KeyModifiers::NONE,
            timestamp_millis: 5,
        });

        let action = WidgetAction::from_gesture_event_for_document(&document, &click, |id| {
            document.nodes()[id.0].action.clone()
        })
        .expect("pointer edit action");

        assert_eq!(action.target, target);
        assert_eq!(action.binding, WidgetActionBinding::action("color.field"));
        let WidgetActionKind::PointerEdit(edit) = action.kind else {
            panic!("expected pointer edit");
        };
        assert_eq!(edit.phase, WidgetValueEditPhase::Commit);
        assert_eq!(edit.local_position, UiPoint::new(25.0, 8.0));
        assert_eq!(edit.target_rect.width, 100.0);
        assert_eq!(edit.target_rect.height, 20.0);
    }

    #[test]
    fn pointer_edit_drag_begin_uses_pointer_origin() {
        let mut document = UiDocument::new(LayoutStyle::size(200.0, 100.0));
        let target = document.add_child(
            document.root,
            UiNode::container("scrollbar", LayoutStyle::size(120.0, 8.0))
                .with_input(InputBehavior::BUTTON)
                .with_pointer_edit_action("scrollbar.drag"),
        );
        document
            .compute_layout(UiSize::new(200.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let drag = GestureEvent::Drag(DragGesture {
            pointer_id: PointerId::MOUSE,
            target,
            phase: GesturePhase::Begin,
            origin: UiPoint::new(4.0, 4.0),
            current: UiPoint::new(90.0, 4.0),
            previous: UiPoint::new(4.0, 4.0),
            delta: UiPoint::new(86.0, 0.0),
            total_delta: UiPoint::new(86.0, 0.0),
            button: PointerButton::Primary,
            modifiers: KeyModifiers::NONE,
            captured: true,
            timestamp_millis: 16,
        });

        let action = WidgetAction::from_gesture_event_for_document(&document, &drag, |id| {
            document.nodes()[id.0].action.clone()
        })
        .expect("pointer edit action");

        let WidgetActionKind::PointerEdit(edit) = action.kind else {
            panic!("expected pointer edit");
        };
        assert_eq!(edit.phase, WidgetValueEditPhase::Begin);
        assert_eq!(edit.position, UiPoint::new(4.0, 4.0));
        assert_eq!(edit.local_position, UiPoint::new(4.0, 4.0));
        assert_eq!(edit.target_rect.width, 120.0);
    }

    #[test]
    fn pointer_edit_parent_rect_uses_parent_geometry() {
        let mut document = UiDocument::new(LayoutStyle::size(240.0, 160.0));
        let parent = document.add_child(
            document.root,
            UiNode::container("window", LayoutStyle::size(180.0, 90.0)),
        );
        let target = document.add_child(
            parent,
            UiNode::container("window.resize", LayoutStyle::size(16.0, 16.0))
                .with_input(InputBehavior::BUTTON)
                .with_action("window.resize")
                .with_action_mode(WidgetActionMode::PointerEditParentRect),
        );
        document
            .compute_layout(UiSize::new(240.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let drag = GestureEvent::Drag(DragGesture {
            pointer_id: PointerId::MOUSE,
            target,
            phase: GesturePhase::Update,
            origin: UiPoint::new(170.0, 80.0),
            current: UiPoint::new(190.0, 100.0),
            previous: UiPoint::new(170.0, 80.0),
            delta: UiPoint::new(20.0, 20.0),
            total_delta: UiPoint::new(20.0, 20.0),
            button: PointerButton::Primary,
            modifiers: KeyModifiers::NONE,
            captured: true,
            timestamp_millis: 5,
        });

        let action = WidgetAction::from_gesture_event_for_document(&document, &drag, |id| {
            document.nodes()[id.0].action.clone()
        })
        .expect("pointer edit action");

        assert_eq!(action.target, target);
        assert_eq!(action.binding, WidgetActionBinding::action("window.resize"));
        let WidgetActionKind::PointerEdit(edit) = action.kind else {
            panic!("expected pointer edit");
        };
        assert_eq!(edit.phase, WidgetValueEditPhase::Update);
        assert_eq!(edit.target_rect.width, 180.0);
        assert_eq!(edit.target_rect.height, 90.0);
        assert_eq!(edit.local_position, UiPoint::new(190.0, 100.0));
    }

    #[test]
    fn keyboard_activation_helper_accepts_enter_and_space() {
        let mut queue = WidgetActionQueue::new();
        queue
            .push_key_activation(
                UiNodeId(1),
                WidgetActionBinding::action("submit"),
                KeyCode::Enter,
                KeyModifiers::NONE,
            )
            .push_key_activation(
                UiNodeId(2),
                WidgetActionBinding::action("toggle"),
                KeyCode::Character(' '),
                KeyModifiers {
                    shift: true,
                    ..KeyModifiers::NONE
                },
            )
            .push_key_activation(
                UiNodeId(3),
                WidgetActionBinding::action("shortcut-owned"),
                KeyCode::Enter,
                KeyModifiers {
                    ctrl: true,
                    ..KeyModifiers::NONE
                },
            );

        assert_eq!(queue.len(), 2);
        assert_eq!(
            queue.as_slice()[0].kind,
            WidgetActionKind::Activate(WidgetActivation::keyboard())
        );
        assert_eq!(queue.as_slice()[1].target, UiNodeId(2));
    }

    #[test]
    fn command_binding_preserves_command_id() {
        let action = WidgetAction::pointer_activate(
            UiNodeId(4),
            WidgetActionBinding::command("file.save"),
            1,
        );

        assert_eq!(
            action.binding.command_id(),
            Some(&CommandId::from("file.save"))
        );
        assert_eq!(action.binding.action_id(), None);
    }

    #[test]
    fn queue_preserves_action_order() {
        let mut queue = WidgetActionQueue::new();
        queue
            .activate(UiNodeId(1), WidgetActionBinding::action("one"))
            .select(UiNodeId(2), WidgetActionBinding::action("two"), true)
            .command_activate(UiNodeId(3), "three");

        let actions = queue.as_slice();
        assert_eq!(actions[0].target, UiNodeId(1));
        assert_eq!(actions[1].target, UiNodeId(2));
        assert_eq!(actions[2].target, UiNodeId(3));
        assert_eq!(
            actions[2].binding.command_id(),
            Some(&CommandId::from("three"))
        );
    }

    #[test]
    fn drag_gesture_phases_map_to_drag_actions() {
        for (gesture_phase, expected_phase) in [
            (GesturePhase::Begin, WidgetDragPhase::Begin),
            (GesturePhase::Update, WidgetDragPhase::Update),
            (GesturePhase::Commit, WidgetDragPhase::Commit),
            (GesturePhase::Cancel, WidgetDragPhase::Cancel),
        ] {
            let event = GestureEvent::Drag(drag(gesture_phase));
            let action = WidgetAction::from_gesture_event(&event, binding("slider.drag")).unwrap();

            assert!(matches!(
                action.kind,
                WidgetActionKind::Drag(WidgetDrag {
                    phase,
                    origin: UiPoint { x: 1.0, y: 2.0 },
                    current: UiPoint { x: 5.0, y: 8.0 },
                    previous: UiPoint { x: 3.0, y: 4.0 },
                    delta: UiPoint { x: 2.0, y: 4.0 },
                    total_delta: UiPoint { x: 4.0, y: 6.0 },
                }) if phase == expected_phase
            ));
        }
    }

    #[test]
    fn drag_gesture_can_be_collected_as_value_edit_phase() {
        let mut queue = WidgetActionQueue::new();

        queue
            .push_drag_value_edit(
                &drag(GesturePhase::Preview),
                WidgetActionBinding::action("trim"),
            )
            .push_drag_value_edit(
                &drag(GesturePhase::Commit),
                WidgetActionBinding::action("trim"),
            )
            .push_drag_value_edit(
                &drag(GesturePhase::Cancel),
                WidgetActionBinding::action("trim"),
            );

        assert_eq!(
            queue
                .iter()
                .map(|action| action.kind.clone())
                .collect::<Vec<_>>(),
            vec![
                WidgetActionKind::ValueEdit(WidgetValueEditPhase::Preview),
                WidgetActionKind::ValueEdit(WidgetValueEditPhase::Commit),
                WidgetActionKind::ValueEdit(WidgetValueEditPhase::Cancel),
            ]
        );
    }

    #[test]
    fn secondary_gesture_click_is_not_semantic_activation() {
        let click = GestureEvent::Click(crate::PointerClick {
            pointer_id: PointerId::MOUSE,
            target: UiNodeId(3),
            position: UiPoint::new(10.0, 12.0),
            button: PointerButton::Secondary,
            count: 1,
            modifiers: KeyModifiers::NONE,
            timestamp_millis: 5,
        });

        assert!(WidgetAction::from_gesture_event(&click, binding("context")).is_none());
    }

    #[test]
    fn preview_gesture_is_not_a_drag_action_but_maps_to_value_edit_preview() {
        let preview = drag(GesturePhase::Preview);

        assert!(
            WidgetAction::drag_from_gesture(&preview, WidgetActionBinding::action("preview"))
                .is_none()
        );
        assert_eq!(
            WidgetAction::value_edit_from_drag(&preview, WidgetActionBinding::action("preview"))
                .kind,
            WidgetActionKind::ValueEdit(WidgetValueEditPhase::Preview)
        );
    }

    #[test]
    fn enabled_document_target_allows_activation() {
        let mut document = fixed_doc();
        let enabled = document.add_child(
            document.root,
            UiNode::container("enabled", LayoutStyle::new())
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Button)),
        );
        let result = UiInputResult {
            clicked: Some(enabled),
            ..Default::default()
        };
        let mut queue = WidgetActionQueue::new();

        queue.push_input_result_for_document(&document, &result, binding("enabled.action"));

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.as_slice()[0].target, enabled);
    }

    #[test]
    fn descendant_document_target_resolves_to_actionable_ancestor() {
        let mut document = fixed_doc();
        let button = document.add_child(
            document.root,
            UiNode::container("button", LayoutStyle::new())
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Button)),
        );
        let label = document.add_child(
            button,
            UiNode::container("button.label", LayoutStyle::new()),
        );
        let result = UiInputResult {
            clicked: Some(label),
            ..Default::default()
        };
        let mut queue = WidgetActionQueue::new();

        queue.push_input_result_for_document(&document, &result, |target| {
            (target == button).then(|| WidgetActionBinding::action("button.activate"))
        });

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.as_slice()[0].target, button);
    }

    #[test]
    fn missing_document_target_is_suppressed() {
        let document = fixed_doc();
        let result = UiInputResult {
            clicked: Some(UiNodeId(99)),
            ..Default::default()
        };
        let mut queue = WidgetActionQueue::new();

        queue.push_input_result_for_document(&document, &result, binding("missing.action"));

        assert!(queue.is_empty());
    }
}
