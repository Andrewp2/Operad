//! Feature-gated egui host adapters.
//!
//! This module keeps egui integration at the boundary of the crate by
//! translating egui input into Operad's renderer-neutral raw input contracts.

use crate::input::{
    PointerButton, PointerButtons, PointerEventKind, RawInputEvent, RawKeyboardEvent,
    RawPointerEvent, RawTextInputEvent, RawWheelEvent,
};
use crate::{FocusDirection, KeyCode, KeyModifiers, UiPoint};

#[derive(Debug, Clone, PartialEq)]
pub struct EguiInputAdapter {
    buttons: PointerButtons,
    last_pointer_pos: Option<UiPoint>,
    modifiers: KeyModifiers,
}

impl EguiInputAdapter {
    pub const fn new() -> Self {
        Self {
            buttons: PointerButtons::NONE,
            last_pointer_pos: None,
            modifiers: KeyModifiers::NONE,
        }
    }

    pub const fn buttons(&self) -> PointerButtons {
        self.buttons
    }

    pub const fn last_pointer_pos(&self) -> Option<UiPoint> {
        self.last_pointer_pos
    }

    pub const fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    pub fn translate_events<'a>(
        &mut self,
        events: impl IntoIterator<Item = &'a egui::Event>,
        timestamp_millis: u64,
    ) -> Vec<RawInputEvent> {
        events
            .into_iter()
            .flat_map(|event| self.translate_event(event, timestamp_millis))
            .collect()
    }

    pub fn translate_event(
        &mut self,
        event: &egui::Event,
        timestamp_millis: u64,
    ) -> Vec<RawInputEvent> {
        match event {
            egui::Event::PointerMoved(pos) => {
                let position = ui_point(*pos);
                self.last_pointer_pos = Some(position);
                vec![RawInputEvent::Pointer(
                    RawPointerEvent::new(PointerEventKind::Move, position, timestamp_millis)
                        .buttons(self.buttons)
                        .modifiers(self.modifiers),
                )]
            }
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } => {
                let position = ui_point(*pos);
                let button = egui_pointer_button(*button);
                self.last_pointer_pos = Some(position);
                self.modifiers = egui_modifiers(*modifiers);
                self.buttons = if *pressed {
                    self.buttons.with(button)
                } else {
                    self.buttons.without(button)
                };
                let kind = if *pressed {
                    PointerEventKind::Down(button)
                } else {
                    PointerEventKind::Up(button)
                };
                vec![RawInputEvent::Pointer(
                    RawPointerEvent::new(kind, position, timestamp_millis)
                        .buttons(self.buttons)
                        .modifiers(self.modifiers),
                )]
            }
            egui::Event::PointerGone => {
                let Some(position) = self.last_pointer_pos.take() else {
                    return Vec::new();
                };
                let event = RawInputEvent::Pointer(
                    RawPointerEvent::new(PointerEventKind::Cancel, position, timestamp_millis)
                        .buttons(self.buttons)
                        .modifiers(self.modifiers),
                );
                self.buttons = PointerButtons::NONE;
                vec![event]
            }
            egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
            } => {
                self.modifiers = egui_modifiers(*modifiers);
                let position = self.last_pointer_pos.unwrap_or(UiPoint::new(0.0, 0.0));
                vec![RawInputEvent::Wheel(
                    egui_wheel_event(*unit, position, *delta, timestamp_millis)
                        .modifiers(self.modifiers),
                )]
            }
            egui::Event::Key {
                key,
                pressed,
                repeat,
                modifiers,
                ..
            } => {
                self.modifiers = egui_modifiers(*modifiers);
                if *pressed && *key == egui::Key::Tab {
                    let direction = if modifiers.shift {
                        FocusDirection::Previous
                    } else {
                        FocusDirection::Next
                    };
                    return vec![RawInputEvent::Focus(direction)];
                }
                let Some(key) = egui_key(*key) else {
                    return Vec::new();
                };
                let event = if *pressed {
                    RawKeyboardEvent::press(key, self.modifiers, timestamp_millis)
                } else {
                    RawKeyboardEvent::release(key, self.modifiers, timestamp_millis)
                }
                .repeat(*repeat);
                vec![RawInputEvent::Keyboard(event)]
            }
            egui::Event::Text(text) | egui::Event::Paste(text) => {
                vec![RawInputEvent::Text(RawTextInputEvent::new(
                    text.clone(),
                    timestamp_millis,
                ))]
            }
            egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                vec![RawInputEvent::Text(RawTextInputEvent::new(
                    text.clone(),
                    timestamp_millis,
                ))]
            }
            _ => Vec::new(),
        }
    }
}

impl Default for EguiInputAdapter {
    fn default() -> Self {
        Self::new()
    }
}

pub fn egui_modifiers(modifiers: egui::Modifiers) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.shift,
        ctrl: modifiers.ctrl || (modifiers.command && !modifiers.mac_cmd),
        alt: modifiers.alt,
        meta: modifiers.mac_cmd,
    }
}

pub fn egui_pointer_button(button: egui::PointerButton) -> PointerButton {
    match button {
        egui::PointerButton::Primary => PointerButton::Primary,
        egui::PointerButton::Secondary => PointerButton::Secondary,
        egui::PointerButton::Middle => PointerButton::Auxiliary,
        egui::PointerButton::Extra1 => PointerButton::Back,
        egui::PointerButton::Extra2 => PointerButton::Forward,
    }
}

pub fn egui_key(key: egui::Key) -> Option<KeyCode> {
    Some(match key {
        egui::Key::ArrowDown => KeyCode::ArrowDown,
        egui::Key::ArrowLeft => KeyCode::ArrowLeft,
        egui::Key::ArrowRight => KeyCode::ArrowRight,
        egui::Key::ArrowUp => KeyCode::ArrowUp,
        egui::Key::Escape => KeyCode::Escape,
        egui::Key::Tab => KeyCode::Tab,
        egui::Key::Backspace => KeyCode::Backspace,
        egui::Key::Enter => KeyCode::Enter,
        egui::Key::Delete => KeyCode::Delete,
        egui::Key::Home => KeyCode::Home,
        egui::Key::End => KeyCode::End,
        egui::Key::Space => KeyCode::Character(' '),
        egui::Key::Colon => KeyCode::Character(':'),
        egui::Key::Comma => KeyCode::Character(','),
        egui::Key::Backslash => KeyCode::Character('\\'),
        egui::Key::Slash => KeyCode::Character('/'),
        egui::Key::Pipe => KeyCode::Character('|'),
        egui::Key::Questionmark => KeyCode::Character('?'),
        egui::Key::Exclamationmark => KeyCode::Character('!'),
        egui::Key::OpenBracket => KeyCode::Character('['),
        egui::Key::CloseBracket => KeyCode::Character(']'),
        egui::Key::OpenCurlyBracket => KeyCode::Character('{'),
        egui::Key::CloseCurlyBracket => KeyCode::Character('}'),
        egui::Key::Backtick => KeyCode::Character('`'),
        egui::Key::Minus => KeyCode::Character('-'),
        egui::Key::Period => KeyCode::Character('.'),
        egui::Key::Plus => KeyCode::Character('+'),
        egui::Key::Equals => KeyCode::Character('='),
        egui::Key::Semicolon => KeyCode::Character(';'),
        egui::Key::Quote => KeyCode::Character('\''),
        egui::Key::Num0 => KeyCode::Character('0'),
        egui::Key::Num1 => KeyCode::Character('1'),
        egui::Key::Num2 => KeyCode::Character('2'),
        egui::Key::Num3 => KeyCode::Character('3'),
        egui::Key::Num4 => KeyCode::Character('4'),
        egui::Key::Num5 => KeyCode::Character('5'),
        egui::Key::Num6 => KeyCode::Character('6'),
        egui::Key::Num7 => KeyCode::Character('7'),
        egui::Key::Num8 => KeyCode::Character('8'),
        egui::Key::Num9 => KeyCode::Character('9'),
        egui::Key::A => KeyCode::Character('a'),
        egui::Key::B => KeyCode::Character('b'),
        egui::Key::C => KeyCode::Character('c'),
        egui::Key::D => KeyCode::Character('d'),
        egui::Key::E => KeyCode::Character('e'),
        egui::Key::F => KeyCode::Character('f'),
        egui::Key::G => KeyCode::Character('g'),
        egui::Key::H => KeyCode::Character('h'),
        egui::Key::I => KeyCode::Character('i'),
        egui::Key::J => KeyCode::Character('j'),
        egui::Key::K => KeyCode::Character('k'),
        egui::Key::L => KeyCode::Character('l'),
        egui::Key::M => KeyCode::Character('m'),
        egui::Key::N => KeyCode::Character('n'),
        egui::Key::O => KeyCode::Character('o'),
        egui::Key::P => KeyCode::Character('p'),
        egui::Key::Q => KeyCode::Character('q'),
        egui::Key::R => KeyCode::Character('r'),
        egui::Key::S => KeyCode::Character('s'),
        egui::Key::T => KeyCode::Character('t'),
        egui::Key::U => KeyCode::Character('u'),
        egui::Key::V => KeyCode::Character('v'),
        egui::Key::W => KeyCode::Character('w'),
        egui::Key::X => KeyCode::Character('x'),
        egui::Key::Y => KeyCode::Character('y'),
        egui::Key::Z => KeyCode::Character('z'),
        _ => return None,
    })
}

fn egui_wheel_event(
    unit: egui::MouseWheelUnit,
    position: UiPoint,
    delta: egui::Vec2,
    timestamp_millis: u64,
) -> RawWheelEvent {
    let delta = UiPoint::new(delta.x, delta.y);
    match unit {
        egui::MouseWheelUnit::Point => RawWheelEvent::pixels(position, delta, timestamp_millis),
        egui::MouseWheelUnit::Line => RawWheelEvent::lines(position, delta, timestamp_millis),
        egui::MouseWheelUnit::Page => RawWheelEvent::pages(position, delta, timestamp_millis),
    }
}

fn ui_point(pos: egui::Pos2) -> UiPoint {
    UiPoint::new(pos.x, pos.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{WheelDeltaUnit, WheelPhase};
    use crate::{
        ApproxTextMeasurer, InputBehavior, UiDocument, UiInputEvent, UiNode, UiNodeStyle, UiSize,
    };
    use taffy::prelude::{Dimension, Display, FlexDirection, Size as TaffySize, Style};

    fn key_event(
        key: egui::Key,
        pressed: bool,
        repeat: bool,
        modifiers: egui::Modifiers,
    ) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat,
            modifiers,
        }
    }

    fn fixed_style(width: f32, height: f32) -> UiNodeStyle {
        UiNodeStyle {
            layout: Style {
                size: TaffySize {
                    width: Dimension::length(width),
                    height: Dimension::length(height),
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn egui_pointer_events_preserve_position_buttons_and_modifiers() {
        let mut adapter = EguiInputAdapter::new();
        let modifiers = egui::Modifiers {
            shift: true,
            ctrl: true,
            command: true,
            ..egui::Modifiers::NONE
        };
        let events = [
            egui::Event::PointerButton {
                pos: egui::Pos2::new(10.0, 20.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers,
            },
            egui::Event::PointerMoved(egui::Pos2::new(16.0, 24.0)),
            egui::Event::PointerButton {
                pos: egui::Pos2::new(16.0, 24.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers,
            },
        ];

        let translated = adapter.translate_events(events.iter(), 42);

        assert_eq!(translated.len(), 3);
        let RawInputEvent::Pointer(down) = translated[0] else {
            panic!("expected pointer down");
        };
        assert_eq!(down.kind, PointerEventKind::Down(PointerButton::Primary));
        assert_eq!(down.position, UiPoint::new(10.0, 20.0));
        assert!(down.buttons.contains(PointerButton::Primary));
        assert_eq!(
            down.modifiers,
            KeyModifiers {
                shift: true,
                ctrl: true,
                alt: false,
                meta: false,
            }
        );

        let RawInputEvent::Pointer(moved) = translated[1] else {
            panic!("expected pointer move");
        };
        assert_eq!(moved.kind, PointerEventKind::Move);
        assert_eq!(moved.position, UiPoint::new(16.0, 24.0));
        assert!(moved.buttons.contains(PointerButton::Primary));
        assert_eq!(moved.modifiers, down.modifiers);

        let RawInputEvent::Pointer(up) = translated[2] else {
            panic!("expected pointer up");
        };
        assert_eq!(up.kind, PointerEventKind::Up(PointerButton::Primary));
        assert_eq!(up.buttons, PointerButtons::NONE);
        assert_eq!(adapter.buttons(), PointerButtons::NONE);
        assert_eq!(adapter.last_pointer_pos(), Some(UiPoint::new(16.0, 24.0)));
    }

    #[test]
    fn egui_wheel_events_keep_units_and_last_pointer_position() {
        let mut adapter = EguiInputAdapter::new();
        adapter.translate_event(&egui::Event::PointerMoved(egui::Pos2::new(40.0, 50.0)), 1);
        let modifiers = egui::Modifiers {
            alt: true,
            ..egui::Modifiers::NONE
        };
        let events = [
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::Vec2::new(2.0, -4.0),
                modifiers,
            },
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::Vec2::new(1.0, -2.0),
                modifiers,
            },
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Page,
                delta: egui::Vec2::new(0.0, 1.0),
                modifiers,
            },
        ];

        let translated = adapter.translate_events(events.iter(), 9);

        let RawInputEvent::Wheel(point) = translated[0] else {
            panic!("expected point wheel");
        };
        assert_eq!(point.position, UiPoint::new(40.0, 50.0));
        assert_eq!(point.delta, UiPoint::new(2.0, -4.0));
        assert_eq!(point.unit, WheelDeltaUnit::Pixel);
        assert_eq!(point.phase, WheelPhase::Moved);
        assert!(point.modifiers.alt);

        let RawInputEvent::Wheel(line) = translated[1] else {
            panic!("expected line wheel");
        };
        assert_eq!(line.unit, WheelDeltaUnit::Line);
        assert_eq!(
            translated[1].to_ui_input_event_with_wheel_scale(12.0, UiSize::new(100.0, 80.0)),
            Some(UiInputEvent::Wheel {
                position: UiPoint::new(40.0, 50.0),
                delta: UiPoint::new(12.0, -24.0),
            })
        );

        let RawInputEvent::Wheel(page) = translated[2] else {
            panic!("expected page wheel");
        };
        assert_eq!(page.unit, WheelDeltaUnit::Page);
        assert_eq!(
            translated[2].to_ui_input_event_with_wheel_scale(12.0, UiSize::new(100.0, 80.0)),
            Some(UiInputEvent::Wheel {
                position: UiPoint::new(40.0, 50.0),
                delta: UiPoint::new(0.0, 80.0),
            })
        );
    }

    #[test]
    fn egui_key_events_map_repeat_release_and_focus_navigation() {
        let mut adapter = EguiInputAdapter::new();
        let command = egui::Modifiers {
            command: true,
            ..egui::Modifiers::NONE
        };

        let translated = adapter.translate_events(
            [
                key_event(egui::Key::A, true, true, command),
                key_event(egui::Key::A, false, false, command),
                key_event(egui::Key::Tab, true, false, egui::Modifiers::NONE),
                key_event(egui::Key::Tab, true, false, egui::Modifiers::SHIFT),
            ]
            .iter(),
            12,
        );

        let RawInputEvent::Keyboard(press) = translated[0] else {
            panic!("expected key press");
        };
        assert_eq!(press.key, KeyCode::Character('a'));
        assert!(press.pressed);
        assert!(press.repeat);
        assert!(press.modifiers.ctrl);
        assert_eq!(
            translated[0].to_ui_input_event(),
            Some(UiInputEvent::Key {
                key: KeyCode::Character('a'),
                modifiers: press.modifiers,
            })
        );

        let RawInputEvent::Keyboard(release) = translated[1] else {
            panic!("expected key release");
        };
        assert!(!release.pressed);
        assert_eq!(translated[1].to_ui_input_event(), None);
        assert_eq!(translated[2], RawInputEvent::Focus(FocusDirection::Next));
        assert_eq!(
            translated[3],
            RawInputEvent::Focus(FocusDirection::Previous)
        );
    }

    #[test]
    fn egui_text_paste_and_ime_commit_translate_to_raw_text() {
        let mut adapter = EguiInputAdapter::new();
        let translated = adapter.translate_events(
            [
                egui::Event::Text("a".to_string()),
                egui::Event::Paste("paste".to_string()),
                egui::Event::Ime(egui::ImeEvent::Commit("ime".to_string())),
            ]
            .iter(),
            24,
        );

        assert_eq!(
            translated,
            vec![
                RawInputEvent::Text(RawTextInputEvent::new("a", 24)),
                RawInputEvent::Text(RawTextInputEvent::new("paste", 24)),
                RawInputEvent::Text(RawTextInputEvent::new("ime", 24)),
            ]
        );
    }

    #[test]
    fn egui_pointer_gone_emits_cancel_and_clears_capture_state() {
        let mut adapter = EguiInputAdapter::new();
        adapter.translate_event(
            &egui::Event::PointerButton {
                pos: egui::Pos2::new(8.0, 9.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            1,
        );

        let translated = adapter.translate_event(&egui::Event::PointerGone, 2);

        assert_eq!(translated.len(), 1);
        let RawInputEvent::Pointer(cancel) = translated[0] else {
            panic!("expected pointer cancel");
        };
        assert_eq!(cancel.kind, PointerEventKind::Cancel);
        assert_eq!(cancel.position, UiPoint::new(8.0, 9.0));
        assert!(cancel.buttons.contains(PointerButton::Primary));
        assert_eq!(adapter.buttons(), PointerButtons::NONE);
        assert_eq!(adapter.last_pointer_pos(), None);
    }

    #[test]
    fn translated_egui_events_feed_document_focus_routing() {
        let mut document = UiDocument::new(UiNodeStyle {
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                size: TaffySize {
                    width: Dimension::length(200.0),
                    height: Dimension::length(80.0),
                },
                ..Default::default()
            },
            ..Default::default()
        });
        let first = document.add_child(
            document.root,
            UiNode::container("first", fixed_style(80.0, 40.0)).with_input(InputBehavior::BUTTON),
        );
        let second = document.add_child(
            document.root,
            UiNode::container("second", fixed_style(80.0, 40.0)).with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(200.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let mut adapter = EguiInputAdapter::new();
        let translated = adapter.translate_events(
            [
                egui::Event::PointerButton {
                    pos: egui::Pos2::new(10.0, 10.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                key_event(egui::Key::Tab, true, false, egui::Modifiers::NONE),
            ]
            .iter(),
            36,
        );

        let mut focused = None;
        for raw in translated {
            if let Some(event) = raw.to_ui_input_event() {
                focused = document.handle_input(event).focused;
            }
        }

        assert_eq!(document.node(first).name, "first");
        assert_eq!(focused, Some(second));
    }
}
