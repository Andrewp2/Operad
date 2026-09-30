use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::{prelude::*, JsCast};

use crate::core::text_input::{byte_to_utf16, utf16_to_byte};
use crate::input::{RawInputEvent, RawKeyboardEvent, RawTextCompositionEvent};
use crate::platform::{TextImeSession, TextInputId};
use crate::{KeyCode, KeyModifiers, TextCompositionEvent};

use super::frame_driver::WebFrameDriver;

#[wasm_bindgen(module = "/src/runtime/web/ime.js")]
extern "C" {
    type ImeBridge;
    #[wasm_bindgen(js_name = createImeBridge)]
    fn create_bridge(canvas: &web_sys::HtmlCanvasElement, emit: &js_sys::Function) -> ImeBridge;
    #[wasm_bindgen(method)]
    fn sync(this: &ImeBridge, snapshot: &JsValue, left: f64, top: f64, width: f64, height: f64);
    #[wasm_bindgen(method)]
    fn deactivate(this: &ImeBridge, input: &str);
    #[wasm_bindgen(method)]
    fn focus(this: &ImeBridge);
    #[wasm_bindgen(method, js_name = isFocused)]
    fn is_focused(this: &ImeBridge) -> bool;
    #[wasm_bindgen(method, js_name = isComposing)]
    fn is_composing(this: &ImeBridge) -> bool;
    #[wasm_bindgen(method, js_name = ownsKey)]
    fn owns_key(this: &ImeBridge, code: &str, pressed: bool, native_composing: bool) -> bool;
    #[wasm_bindgen(method)]
    fn destroy(this: &ImeBridge);
}

pub(super) struct WebTextInput {
    bridge: ImeBridge,
    _callback: Closure<dyn FnMut(JsValue)>,
}

impl WebTextInput {
    pub fn new(
        canvas: &web_sys::HtmlCanvasElement,
        pending: Rc<RefCell<Vec<RawInputEvent>>>,
        driver: &Rc<WebFrameDriver>,
    ) -> Self {
        let driver = Rc::downgrade(driver);
        let callback = Closure::wrap(Box::new(move |value: JsValue| {
            let Some(driver) = driver.upgrade() else {
                return;
            };
            if driver.is_stopped() {
                return;
            }
            let get =
                |key: &str| js_sys::Reflect::get(&value, &key.into()).unwrap_or(JsValue::UNDEFINED);
            let string = |key: &str| get(key).as_string().unwrap_or_default();
            let timestamp_millis = driver.now().as_millis().try_into().unwrap_or(u64::MAX);
            let kind = string("kind");
            let raw = match kind.as_str() {
                "undo" | "redo" | "enter" => RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    if kind == "enter" {
                        KeyCode::Enter
                    } else {
                        KeyCode::Character('z')
                    },
                    KeyModifiers {
                        ctrl: kind != "enter",
                        shift: kind == "redo",
                        ..KeyModifiers::NONE
                    },
                    timestamp_millis,
                )),
                _ => {
                    let text = string("text");
                    let source = string("source");
                    let range = js_sys::Array::from(&get("range"));
                    let offset = |range: &js_sys::Array, index| {
                        range.get(index).as_f64().unwrap_or(0.0).max(0.0) as usize
                    };
                    let replacement = Some(
                        utf16_to_byte(&source, offset(&range, 0))
                            ..utf16_to_byte(&source, offset(&range, 1)),
                    );
                    let event = match kind.as_str() {
                        "preedit" => {
                            let selection = get("selection");
                            let selection = (!selection.is_null()).then(|| {
                                let range = js_sys::Array::from(&selection);
                                utf16_to_byte(&text, offset(&range, 0))
                                    ..utf16_to_byte(&text, offset(&range, 1))
                            });
                            TextCompositionEvent::Preedit {
                                text,
                                selection,
                                replacement,
                            }
                        }
                        "commit" => TextCompositionEvent::Commit { text, replacement },
                        "cancel" => TextCompositionEvent::Cancel,
                        _ => return,
                    };
                    RawInputEvent::Composition(RawTextCompositionEvent {
                        input: TextInputId::new(string("input")),
                        event,
                        timestamp_millis,
                    })
                }
            };
            pending.borrow_mut().push(raw);
            driver.wake();
        }) as Box<dyn FnMut(JsValue)>);
        let bridge = create_bridge(canvas, callback.as_ref().unchecked_ref());
        Self {
            bridge,
            _callback: callback,
        }
    }

    pub fn sync(
        &self,
        session: &TextImeSession,
        canvas: &web_sys::HtmlCanvasElement,
        css_scale: f32,
    ) {
        let snapshot = js_sys::Object::new();
        for (key, value) in [
            ("input", JsValue::from_str(session.input.as_str())),
            ("text", JsValue::from_str(&session.surrounding_text)),
            (
                "selectionStart",
                JsValue::from_f64(
                    byte_to_utf16(&session.surrounding_text, session.selection.start) as f64,
                ),
            ),
            (
                "selectionEnd",
                JsValue::from_f64(
                    byte_to_utf16(&session.surrounding_text, session.selection.end) as f64,
                ),
            ),
            (
                "composing",
                JsValue::from_bool(session.composition.is_some()),
            ),
            ("sensitive", JsValue::from_bool(session.sensitive)),
            ("multiline", JsValue::from_bool(session.multiline)),
        ] {
            js_sys::Reflect::set(&snapshot, &key.into(), &value).expect("IME snapshot property");
        }
        let canvas_rect = canvas.get_bounding_client_rect();
        let rect = session.cursor_rect;
        self.bridge.sync(
            &snapshot,
            canvas_rect.left() + (rect.origin.x * css_scale) as f64,
            canvas_rect.top() + (rect.origin.y * css_scale) as f64,
            (rect.size.width * css_scale) as f64,
            (rect.size.height * css_scale) as f64,
        );
    }

    pub fn deactivate(&self, input: &TextInputId) {
        self.bridge.deactivate(input.as_str());
    }
    pub fn shutdown(&self) {
        self.bridge.destroy();
    }
    pub fn focus(&self) {
        self.bridge.focus();
    }
    pub fn is_focused(&self) -> bool {
        self.bridge.is_focused()
    }
    pub fn is_composing(&self) -> bool {
        self.bridge.is_composing()
    }
    pub fn owns_key(&self, event: &web_sys::KeyboardEvent, pressed: bool) -> bool {
        self.bridge.owns_key(
            &event.code(),
            pressed,
            event.is_composing()
                || event.key_code() == 229
                || matches!(event.key().as_str(), "Process" | "Dead"),
        )
    }
}

impl Drop for WebTextInput {
    fn drop(&mut self) {
        self.shutdown();
    }
}
