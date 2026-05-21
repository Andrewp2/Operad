//! Renderer-neutral raw input and gesture routing primitives.
//!
//! `UiInputEvent` remains the small document-facing event enum. This module is
//! the richer backend-facing layer that tracks pointer identity, capture,
//! double-click timing, high-resolution wheel deltas, and drag edit phases.

use std::collections::HashMap;

use crate::{
    EditPhase, FocusDirection, KeyCode, KeyModifiers, UiInputEvent, UiNodeId, UiPoint, UiSize,
    UiWheelEvent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PointerId(pub(crate) u64);

impl PointerId {
    pub const MOUSE: Self = Self(1);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerKind {
    Mouse,
    Touch,
    Pen,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerButton {
    Primary,
    Secondary,
    Auxiliary,
    Back,
    Forward,
    Other(u16),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PointerButtons(u16);

impl PointerButtons {
    pub const NONE: Self = Self(0);
    pub const PRIMARY: Self = Self(1 << 0);
    pub const SECONDARY: Self = Self(1 << 1);
    pub const AUXILIARY: Self = Self(1 << 2);
    pub const BACK: Self = Self(1 << 3);
    pub const FORWARD: Self = Self(1 << 4);

    pub const fn empty() -> Self {
        Self::NONE
    }

    pub const fn from_button(button: PointerButton) -> Self {
        Self(button_bit(button))
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn contains(self, button: PointerButton) -> bool {
        (self.0 & button_bit(button)) != 0
    }

    pub const fn with(self, button: PointerButton) -> Self {
        Self(self.0 | button_bit(button))
    }

    pub const fn without(self, button: PointerButton) -> Self {
        Self(self.0 & !button_bit(button))
    }
}

const fn button_bit(button: PointerButton) -> u16 {
    match button {
        PointerButton::Primary => 1 << 0,
        PointerButton::Secondary => 1 << 1,
        PointerButton::Auxiliary => 1 << 2,
        PointerButton::Back => 1 << 3,
        PointerButton::Forward => 1 << 4,
        PointerButton::Other(value) => {
            if value < 11 {
                1 << (value + 5)
            } else {
                0
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerEventKind {
    Move,
    Down(PointerButton),
    Up(PointerButton),
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawPointerEvent {
    pub pointer_id: PointerId,
    pub pointer_kind: PointerKind,
    pub kind: PointerEventKind,
    pub position: UiPoint,
    pub buttons: PointerButtons,
    pub modifiers: KeyModifiers,
    pub timestamp_millis: u64,
}

impl RawPointerEvent {
    pub const fn new(kind: PointerEventKind, position: UiPoint, timestamp_millis: u64) -> Self {
        Self {
            pointer_id: PointerId::MOUSE,
            pointer_kind: PointerKind::Mouse,
            kind,
            position,
            buttons: PointerButtons::NONE,
            modifiers: KeyModifiers::NONE,
            timestamp_millis,
        }
    }

    pub const fn pointer_id(mut self, pointer_id: PointerId) -> Self {
        self.pointer_id = pointer_id;
        self
    }

    pub const fn pointer_kind(mut self, pointer_kind: PointerKind) -> Self {
        self.pointer_kind = pointer_kind;
        self
    }

    pub const fn buttons(mut self, buttons: PointerButtons) -> Self {
        self.buttons = buttons;
        self
    }

    pub const fn modifiers(mut self, modifiers: KeyModifiers) -> Self {
        self.modifiers = modifiers;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WheelDeltaUnit {
    Pixel,
    Line,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WheelPhase {
    Started,
    Moved,
    Ended,
    Momentum,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawWheelEvent {
    pub position: UiPoint,
    pub delta: UiPoint,
    pub unit: WheelDeltaUnit,
    pub phase: WheelPhase,
    pub modifiers: KeyModifiers,
    pub timestamp_millis: u64,
}

impl RawWheelEvent {
    pub const fn pixels(position: UiPoint, delta: UiPoint, timestamp_millis: u64) -> Self {
        Self {
            position,
            delta,
            unit: WheelDeltaUnit::Pixel,
            phase: WheelPhase::Moved,
            modifiers: KeyModifiers::NONE,
            timestamp_millis,
        }
    }

    pub const fn lines(position: UiPoint, delta: UiPoint, timestamp_millis: u64) -> Self {
        Self {
            position,
            delta,
            unit: WheelDeltaUnit::Line,
            phase: WheelPhase::Moved,
            modifiers: KeyModifiers::NONE,
            timestamp_millis,
        }
    }

    pub const fn pages(position: UiPoint, delta: UiPoint, timestamp_millis: u64) -> Self {
        Self {
            position,
            delta,
            unit: WheelDeltaUnit::Page,
            phase: WheelPhase::Moved,
            modifiers: KeyModifiers::NONE,
            timestamp_millis,
        }
    }

    pub const fn phase(mut self, phase: WheelPhase) -> Self {
        self.phase = phase;
        self
    }

    pub const fn modifiers(mut self, modifiers: KeyModifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    pub fn pixel_delta(self, line_size: f32, page_size: UiSize) -> UiPoint {
        match self.unit {
            WheelDeltaUnit::Pixel => self.delta,
            WheelDeltaUnit::Line => {
                UiPoint::new(self.delta.x * line_size, self.delta.y * line_size)
            }
            WheelDeltaUnit::Page => UiPoint::new(
                self.delta.x * page_size.width,
                self.delta.y * page_size.height,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawKeyboardEvent {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
    pub pressed: bool,
    pub repeat: bool,
    pub timestamp_millis: u64,
}

impl RawKeyboardEvent {
    pub const fn press(key: KeyCode, modifiers: KeyModifiers, timestamp_millis: u64) -> Self {
        Self {
            key,
            modifiers,
            pressed: true,
            repeat: false,
            timestamp_millis,
        }
    }

    pub const fn release(key: KeyCode, modifiers: KeyModifiers, timestamp_millis: u64) -> Self {
        Self {
            key,
            modifiers,
            pressed: false,
            repeat: false,
            timestamp_millis,
        }
    }

    pub const fn repeat(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTextInputEvent {
    pub text: String,
    pub timestamp_millis: u64,
}

impl RawTextInputEvent {
    pub fn new(text: impl Into<String>, timestamp_millis: u64) -> Self {
        Self {
            text: text.into(),
            timestamp_millis,
        }
    }
}

#[allow(dead_code)]
pub(crate) fn text_input_for_key_event_text(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    let mut filtered = String::with_capacity(text.len());
    for character in text.chars() {
        if !character.is_control() {
            filtered.push(character);
        }
    }
    (!filtered.is_empty()).then_some(filtered)
}

pub(crate) fn text_input_is_keyboard_line_break(text: &str) -> bool {
    matches!(text, "\n" | "\r" | "\r\n")
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawInputEvent {
    Pointer(RawPointerEvent),
    Wheel(RawWheelEvent),
    Keyboard(RawKeyboardEvent),
    Text(RawTextInputEvent),
    Focus(FocusDirection),
}

impl RawInputEvent {
    pub fn to_ui_input_event(&self) -> Option<UiInputEvent> {
        self.to_ui_input_event_with_wheel_scale(16.0, UiSize::new(800.0, 600.0))
    }

    pub fn to_ui_input_event_with_wheel_scale(
        &self,
        line_size: f32,
        page_size: UiSize,
    ) -> Option<UiInputEvent> {
        match self {
            Self::Pointer(event) => match event.kind {
                PointerEventKind::Move => Some(UiInputEvent::PointerMove(event.position)),
                PointerEventKind::Down(_) => Some(UiInputEvent::PointerDown(event.position)),
                PointerEventKind::Up(_) => Some(UiInputEvent::PointerUp(event.position)),
                PointerEventKind::Cancel => None,
            },
            Self::Wheel(event) => Some(UiInputEvent::Wheel(
                UiWheelEvent::pixels(event.position, event.pixel_delta(line_size, page_size))
                    .unit(event.unit)
                    .phase(event.phase)
                    .modifiers(event.modifiers),
            )),
            Self::Keyboard(event) if event.pressed => Some(UiInputEvent::Key {
                key: event.key,
                modifiers: event.modifiers,
            }),
            Self::Keyboard(_) => None,
            Self::Text(event) => Some(UiInputEvent::TextInput(event.text.clone())),
            Self::Focus(direction) => Some(UiInputEvent::Focus(*direction)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GesturePhase {
    Preview,
    Begin,
    Update,
    Commit,
    Cancel,
}

impl GesturePhase {
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerCapture {
    pub pointer_id: PointerId,
    pub target: UiNodeId,
    pub origin: UiPoint,
    pub threshold: f32,
    pub modifiers: KeyModifiers,
}

impl PointerCapture {
    pub const fn new(
        pointer_id: PointerId,
        target: UiNodeId,
        origin: UiPoint,
        threshold: f32,
        modifiers: KeyModifiers,
    ) -> Self {
        Self {
            pointer_id,
            target,
            origin,
            threshold,
            modifiers,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragGesture {
    pub pointer_id: PointerId,
    pub target: UiNodeId,
    pub phase: GesturePhase,
    pub origin: UiPoint,
    pub current: UiPoint,
    pub previous: UiPoint,
    pub delta: UiPoint,
    pub total_delta: UiPoint,
    pub button: PointerButton,
    pub modifiers: KeyModifiers,
    pub captured: bool,
    pub timestamp_millis: u64,
}

impl DragGesture {
    pub const fn edit_phase(self) -> EditPhase {
        self.phase.edit_phase()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerClick {
    pub pointer_id: PointerId,
    pub target: UiNodeId,
    pub position: UiPoint,
    pub button: PointerButton,
    pub count: u8,
    pub modifiers: KeyModifiers,
    pub timestamp_millis: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GestureEvent {
    Hover {
        target: Option<UiNodeId>,
        position: UiPoint,
        modifiers: KeyModifiers,
    },
    Press {
        target: Option<UiNodeId>,
        pointer_id: PointerId,
        position: UiPoint,
        button: PointerButton,
        modifiers: KeyModifiers,
    },
    Drag(DragGesture),
    Click(PointerClick),
    WheelTargeted {
        target: Option<UiNodeId>,
        event: RawWheelEvent,
    },
    Cancel {
        target: UiNodeId,
        pointer_id: PointerId,
        position: UiPoint,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestureSettings {
    pub drag_threshold: f32,
    pub double_click_millis: u64,
    pub double_click_distance: f32,
}

impl Default for GestureSettings {
    fn default() -> Self {
        Self {
            drag_threshold: 4.0,
            double_click_millis: 500,
            double_click_distance: 6.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointerGestureTracker {
    settings: GestureSettings,
    active: HashMap<PointerId, ActivePointer>,
    last_click: Option<ClickMemory>,
}

impl PointerGestureTracker {
    pub fn new(settings: GestureSettings) -> Self {
        Self {
            settings,
            active: HashMap::new(),
            last_click: None,
        }
    }

    pub fn settings(&self) -> GestureSettings {
        self.settings
    }

    pub fn active_capture(&self, pointer_id: PointerId) -> Option<PointerCapture> {
        self.active
            .get(&pointer_id)
            .map(|active| active.capture(self.settings.drag_threshold))
    }

    pub fn pointer_down(
        &mut self,
        target: Option<UiNodeId>,
        event: RawPointerEvent,
    ) -> GestureEvent {
        let button = match event.kind {
            PointerEventKind::Down(button) => button,
            _ => PointerButton::Primary,
        };
        if let Some(target) = target {
            self.active.insert(
                event.pointer_id,
                ActivePointer {
                    pointer_id: event.pointer_id,
                    target,
                    button,
                    origin: event.position,
                    previous: event.position,
                    current: event.position,
                    modifiers: event.modifiers,
                    timestamp_millis: event.timestamp_millis,
                    dragging: false,
                },
            );
        }

        GestureEvent::Press {
            target,
            pointer_id: event.pointer_id,
            position: event.position,
            button,
            modifiers: event.modifiers,
        }
    }

    pub fn pointer_move(
        &mut self,
        target: Option<UiNodeId>,
        event: RawPointerEvent,
    ) -> Option<GestureEvent> {
        let Some(active) = self.active.get_mut(&event.pointer_id) else {
            return Some(GestureEvent::Hover {
                target,
                position: event.position,
                modifiers: event.modifiers,
            });
        };

        let previous = active.current;
        active.previous = previous;
        active.current = event.position;
        active.modifiers = event.modifiers;
        active.timestamp_millis = event.timestamp_millis;

        let total_delta = delta(active.origin, active.current);
        if !active.dragging && distance(total_delta) < self.settings.drag_threshold {
            return None;
        }

        let phase = if active.dragging {
            GesturePhase::Update
        } else {
            active.dragging = true;
            GesturePhase::Begin
        };
        Some(GestureEvent::Drag(active.drag_event(phase, previous)))
    }

    pub fn pointer_up(
        &mut self,
        hit_target: Option<UiNodeId>,
        event: RawPointerEvent,
    ) -> Option<GestureEvent> {
        let button = match event.kind {
            PointerEventKind::Up(button) => button,
            _ => PointerButton::Primary,
        };
        let active = self.active.remove(&event.pointer_id)?;
        if active.dragging {
            let mut active = active;
            let previous = active.current;
            active.previous = previous;
            active.current = event.position;
            active.modifiers = event.modifiers;
            active.timestamp_millis = event.timestamp_millis;
            return Some(GestureEvent::Drag(
                active.drag_event(GesturePhase::Commit, previous),
            ));
        }

        if hit_target != Some(active.target) {
            return None;
        }

        let count = self.click_count(
            active.target,
            button,
            event.position,
            event.timestamp_millis,
        );
        let click = PointerClick {
            pointer_id: event.pointer_id,
            target: active.target,
            position: event.position,
            button,
            count,
            modifiers: event.modifiers,
            timestamp_millis: event.timestamp_millis,
        };
        self.last_click = Some(ClickMemory {
            target: active.target,
            button,
            position: event.position,
            count,
            timestamp_millis: event.timestamp_millis,
        });
        Some(GestureEvent::Click(click))
    }

    pub fn pointer_cancel(
        &mut self,
        pointer_id: PointerId,
        position: UiPoint,
    ) -> Option<GestureEvent> {
        let active = self.active.remove(&pointer_id)?;
        Some(if active.dragging {
            GestureEvent::Drag(active.drag_event(GesturePhase::Cancel, active.current))
        } else {
            GestureEvent::Cancel {
                target: active.target,
                pointer_id,
                position,
            }
        })
    }

    pub const fn wheel(target: Option<UiNodeId>, event: RawWheelEvent) -> GestureEvent {
        GestureEvent::WheelTargeted { target, event }
    }

    fn click_count(
        &self,
        target: UiNodeId,
        button: PointerButton,
        position: UiPoint,
        timestamp_millis: u64,
    ) -> u8 {
        let Some(last) = self.last_click else {
            return 1;
        };
        let within_time = timestamp_millis.saturating_sub(last.timestamp_millis)
            <= self.settings.double_click_millis;
        let within_distance =
            distance(delta(last.position, position)) <= self.settings.double_click_distance;
        if last.target == target && last.button == button && within_time && within_distance {
            last.count.saturating_add(1).max(2)
        } else {
            1
        }
    }
}

impl Default for PointerGestureTracker {
    fn default() -> Self {
        Self::new(GestureSettings::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ActivePointer {
    pointer_id: PointerId,
    target: UiNodeId,
    button: PointerButton,
    origin: UiPoint,
    previous: UiPoint,
    current: UiPoint,
    modifiers: KeyModifiers,
    timestamp_millis: u64,
    dragging: bool,
}

impl ActivePointer {
    const fn capture(self, threshold: f32) -> PointerCapture {
        PointerCapture::new(
            self.pointer_id,
            self.target,
            self.origin,
            threshold,
            self.modifiers,
        )
    }

    fn drag_event(self, phase: GesturePhase, previous: UiPoint) -> DragGesture {
        DragGesture {
            pointer_id: self.pointer_id,
            target: self.target,
            phase,
            origin: self.origin,
            current: self.current,
            previous,
            delta: delta(previous, self.current),
            total_delta: delta(self.origin, self.current),
            button: self.button,
            modifiers: self.modifiers,
            captured: true,
            timestamp_millis: self.timestamp_millis,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ClickMemory {
    target: UiNodeId,
    button: PointerButton,
    position: UiPoint,
    count: u8,
    timestamp_millis: u64,
}

const fn delta(from: UiPoint, to: UiPoint) -> UiPoint {
    UiPoint::new(to.x - from.x, to.y - from.y)
}

fn distance(delta: UiPoint) -> f32 {
    (delta.x * delta.x + delta.y * delta.y).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pointer_event(kind: PointerEventKind, x: f32, y: f32, timestamp: u64) -> RawPointerEvent {
        RawPointerEvent::new(kind, UiPoint::new(x, y), timestamp)
    }

    #[test]
    fn raw_input_events_convert_to_document_events_with_wheel_metadata() {
        let wheel = RawInputEvent::Wheel(
            RawWheelEvent::lines(UiPoint::new(10.0, 20.0), UiPoint::new(0.5, -2.0), 5)
                .phase(WheelPhase::Started)
                .modifiers(KeyModifiers {
                    shift: true,
                    ..KeyModifiers::NONE
                }),
        );
        assert_eq!(
            wheel.to_ui_input_event_with_wheel_scale(18.0, UiSize::new(400.0, 300.0)),
            Some(UiInputEvent::Wheel(
                UiWheelEvent::pixels(UiPoint::new(10.0, 20.0), UiPoint::new(9.0, -36.0))
                    .unit(WheelDeltaUnit::Line)
                    .phase(WheelPhase::Started)
                    .modifiers(KeyModifiers {
                        shift: true,
                        ..KeyModifiers::NONE
                    })
            ))
        );

        let key = RawInputEvent::Keyboard(RawKeyboardEvent::press(
            KeyCode::Enter,
            KeyModifiers::NONE,
            10,
        ));
        assert_eq!(
            key.to_ui_input_event(),
            Some(UiInputEvent::Key {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            })
        );
        let key_release = RawInputEvent::Keyboard(RawKeyboardEvent::release(
            KeyCode::Enter,
            KeyModifiers::NONE,
            11,
        ));
        assert_eq!(key_release.to_ui_input_event(), None);
    }

    #[test]
    fn key_event_text_filters_control_characters() {
        assert_eq!(text_input_for_key_event_text("a"), Some("a".to_string()));
        assert_eq!(text_input_for_key_event_text("é"), Some("é".to_string()));
        assert_eq!(text_input_for_key_event_text("\r"), None);
        assert_eq!(text_input_for_key_event_text("\n"), None);
        assert_eq!(text_input_for_key_event_text("\t"), None);
        assert_eq!(text_input_for_key_event_text("a\n"), Some("a".to_string()));
        assert!(text_input_is_keyboard_line_break("\n"));
        assert!(text_input_is_keyboard_line_break("\r"));
        assert!(text_input_is_keyboard_line_break("\r\n"));
        assert!(!text_input_is_keyboard_line_break("\n\n"));
    }

    #[test]
    fn pointer_buttons_track_independent_button_bits() {
        let buttons = PointerButtons::empty()
            .with(PointerButton::Primary)
            .with(PointerButton::Other(2));

        assert!(buttons.contains(PointerButton::Primary));
        assert!(buttons.contains(PointerButton::Other(2)));
        assert!(!buttons.contains(PointerButton::Secondary));
        assert!(!buttons
            .without(PointerButton::Primary)
            .contains(PointerButton::Primary));
    }

    #[test]
    fn drag_starts_after_threshold_and_captures_original_target() {
        let mut tracker = PointerGestureTracker::new(GestureSettings {
            drag_threshold: 5.0,
            ..Default::default()
        });
        let target = UiNodeId(7);
        let outside = UiNodeId(8);
        tracker.pointer_down(
            Some(target),
            pointer_event(
                PointerEventKind::Down(PointerButton::Primary),
                10.0,
                10.0,
                1,
            ),
        );

        assert!(tracker.active_capture(PointerId::MOUSE).is_some());
        assert_eq!(
            tracker.pointer_move(
                Some(outside),
                pointer_event(PointerEventKind::Move, 13.0, 13.0, 2)
            ),
            None
        );

        let begin = tracker
            .pointer_move(
                Some(outside),
                pointer_event(PointerEventKind::Move, 20.0, 14.0, 3),
            )
            .expect("drag begin");
        let GestureEvent::Drag(begin) = begin else {
            panic!("expected drag begin");
        };
        assert_eq!(begin.target, target);
        assert_eq!(begin.phase, GesturePhase::Begin);
        assert!(begin.captured);
        assert_eq!(begin.total_delta, UiPoint::new(10.0, 4.0));

        let commit = tracker
            .pointer_up(
                Some(outside),
                pointer_event(PointerEventKind::Up(PointerButton::Primary), 24.0, 18.0, 4),
            )
            .expect("drag commit");
        let GestureEvent::Drag(commit) = commit else {
            panic!("expected drag commit");
        };
        assert_eq!(commit.target, target);
        assert_eq!(commit.phase, GesturePhase::Commit);
        assert_eq!(commit.total_delta, UiPoint::new(14.0, 8.0));
        assert_eq!(tracker.active_capture(PointerId::MOUSE), None);
    }

    #[test]
    fn double_click_count_respects_target_time_and_distance() {
        let mut tracker = PointerGestureTracker::new(GestureSettings {
            double_click_millis: 250,
            double_click_distance: 4.0,
            ..Default::default()
        });
        let target = UiNodeId(3);
        tracker.pointer_down(
            Some(target),
            pointer_event(
                PointerEventKind::Down(PointerButton::Primary),
                10.0,
                10.0,
                10,
            ),
        );
        let first = tracker
            .pointer_up(
                Some(target),
                pointer_event(PointerEventKind::Up(PointerButton::Primary), 10.0, 10.0, 20),
            )
            .expect("first click");
        assert!(matches!(
            first,
            GestureEvent::Click(PointerClick { count: 1, .. })
        ));

        tracker.pointer_down(
            Some(target),
            pointer_event(
                PointerEventKind::Down(PointerButton::Primary),
                12.0,
                11.0,
                100,
            ),
        );
        let second = tracker
            .pointer_up(
                Some(target),
                pointer_event(
                    PointerEventKind::Up(PointerButton::Primary),
                    12.0,
                    11.0,
                    120,
                ),
            )
            .expect("second click");
        assert!(matches!(
            second,
            GestureEvent::Click(PointerClick { count: 2, .. })
        ));

        tracker.pointer_down(
            Some(target),
            pointer_event(
                PointerEventKind::Down(PointerButton::Primary),
                30.0,
                30.0,
                150,
            ),
        );
        let far_click = tracker
            .pointer_up(
                Some(target),
                pointer_event(
                    PointerEventKind::Up(PointerButton::Primary),
                    30.0,
                    30.0,
                    160,
                ),
            )
            .expect("far click");
        assert!(matches!(
            far_click,
            GestureEvent::Click(PointerClick { count: 1, .. })
        ));
    }

    #[test]
    fn cancel_reports_drag_cancel_phase() {
        let mut tracker = PointerGestureTracker::default();
        let target = UiNodeId(2);
        tracker.pointer_down(
            Some(target),
            pointer_event(PointerEventKind::Down(PointerButton::Primary), 0.0, 0.0, 1),
        );
        tracker.pointer_move(
            Some(target),
            pointer_event(PointerEventKind::Move, 20.0, 0.0, 2),
        );

        let cancel = tracker
            .pointer_cancel(PointerId::MOUSE, UiPoint::new(20.0, 0.0))
            .expect("cancel");
        let GestureEvent::Drag(cancel) = cancel else {
            panic!("expected drag cancel");
        };
        assert_eq!(cancel.target, target);
        assert_eq!(cancel.phase, GesturePhase::Cancel);
    }
}
