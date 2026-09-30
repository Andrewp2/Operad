//! Native/browser composition fixture. Browser tests drive Chrome's IME API.
use operad::widgets::{text_input, TextInputOptions, TextInputState};
use operad::{LayoutStyle, UiDocument, UiSize, WidgetAction, WidgetActionKind};

struct State {
    fields: [TextInputState; 2],
    reversed: bool,
    disabled: bool,
    removed: bool,
    password: bool,
    focused: [bool; 2],
    commits: usize,
    canceled: usize,
}

impl Default for State {
    fn default() -> Self {
        let mut first = TextInputState::new("a😀旧z").multiline(true);
        first.set_selection(1, 8);
        Self {
            fields: [first, TextInputState::new("second")],
            reversed: false,
            disabled: false,
            removed: false,
            password: false,
            focused: [false; 2],
            commits: 0,
            canceled: 0,
        }
    }
}

fn update(state: &mut State, action: WidgetAction) {
    let index = match action.binding.action_id().map(|id| id.as_str()) {
        Some("first") => 0,
        Some("second") => 1,
        _ => return,
    };
    if let WidgetActionKind::Focus(change) = &action.kind {
        state.focused[index] = change.focused;
    }
    if let WidgetActionKind::TextEdit(edit) = action.kind {
        // Surviving owners receive cancellation through normal actions; removed
        // owners use the interaction-cancelled hook. Count both delivery paths.
        state.canceled += usize::from(matches!(
            edit.event,
            operad::UiInputEvent::Composition {
                event: operad::TextCompositionEvent::Cancel,
                ..
            }
        ));
        let outcome =
            state.fields[index].apply_widget_text_edit(&edit, &TextInputOptions::default());
        state.commits += usize::from(outcome.changed);
    }
}

fn view(
    state: &State,
    viewport: UiSize,
    _views: &mut operad::runtime::ViewContext<'_>,
) -> UiDocument {
    let mut doc = UiDocument::new(
        LayoutStyle::column()
            .with_size(viewport.width, viewport.height)
            .with_padding(24.0)
            .with_gap(16.0),
    );
    let root = doc.root();
    for index in if state.reversed { [1, 0] } else { [0, 1] } {
        if index == 0 && state.removed {
            continue;
        }
        let name = if index == 0 { "first" } else { "second" };
        let options = TextInputOptions {
            enabled: index != 0 || !state.disabled,
            focused: state.focused[index],
            ..TextInputOptions::default()
                .with_layout(LayoutStyle::size(360.0, 100.0))
                .with_edit_action(name)
        };
        if index == 0 && state.password {
            operad::widgets::password_input(&mut doc, root, name, &state.fields[index], options);
        } else {
            text_input(&mut doc, root, name, &state.fields[index], options);
        }
    }
    doc
}

fn hooks() -> operad::runtime::RuntimeHooks<State> {
    operad::runtime::RuntimeHooks::new().with_interaction_cancelled(|state: &mut State, cancel| {
        let index = match cancel.binding.action_id().map(|id| id.as_str()) {
            Some("first") => 0,
            Some("second") => 1,
            _ => return,
        };
        if let WidgetActionKind::TextEdit(edit) = &cancel.kind {
            state.fields[index].apply_widget_text_edit(edit, &TextInputOptions::default());
            state.canceled += 1;
        }
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::{JsCast, JsValue};
    fn set(object: &js_sys::Object, key: &str, value: impl Into<JsValue>) {
        js_sys::Reflect::set(object, &key.into(), &value.into()).unwrap();
    }
    let (sender, receiver) = operad::runtime::task_channel::<String>(16);
    let command = wasm_bindgen::closure::Closure::<dyn FnMut(String)>::new(move |command| {
        sender.try_send(command).unwrap();
    });
    js_sys::Reflect::set(
        &js_sys::global(),
        &"__IME_COMMAND__".into(),
        command.as_ref().unchecked_ref(),
    )
    .unwrap();
    command.forget();
    let frames = std::cell::Cell::new(0u32);
    let commands = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let completed_commands = commands.clone();
    let hooks = hooks()
        .with_task_completions(receiver, move |state, command| {
            match command.as_str() {
                "reset" => *state = State::default(),
                "reorder" => state.reversed = !state.reversed,
                "disable" => state.disabled = !state.disabled,
                "remove" => state.removed = !state.removed,
                "password" => state.password = !state.password,
                "select" => state.fields[0].set_selection(1, 8),
                _ => panic!("unknown probe command"),
            }
            completed_commands.set(completed_commands.get() + 1);
        })
        .with_frame_observer(move |state: &State, observation| {
            frames.set(frames.get() + 1);
            let status = js_sys::Object::new();
            set(&status, "frames", frames.get());
            set(&status, "commands", commands.get());
            set(&status, "commits", state.commits as u32);
            set(&status, "canceled", state.canceled as u32);
            set(&status, "reversed", state.reversed);
            set(&status, "disabled", state.disabled);
            set(&status, "removed", state.removed);
            let fields = js_sys::Array::new();
            for field in &state.fields {
                let value = js_sys::Object::new();
                set(&value, "text", field.text());
                set(&value, "display", field.display_text());
                set(
                    &value,
                    "draft",
                    field
                        .composing()
                        .map(JsValue::from_str)
                        .unwrap_or(JsValue::NULL),
                );
                set(&value, "caret", field.caret() as u32);
                set(
                    &value,
                    "anchor",
                    field
                        .selection_anchor()
                        .map(|anchor| JsValue::from_f64(anchor as f64))
                        .unwrap_or(JsValue::NULL),
                );
                set(&value, "undo", field.history().can_undo());
                fields.push(&value);
            }
            set(&status, "fields", fields);
            if let Some(ime) = &observation.frame.host_output.state.text_ime {
                set(&status, "input", ime.input.as_str());
                let rect = js_sys::Array::new();
                for value in [
                    ime.cursor_rect.origin.x,
                    ime.cursor_rect.origin.y,
                    ime.cursor_rect.size.width,
                    ime.cursor_rect.size.height,
                ] {
                    rect.push(&JsValue::from_f64(value as f64));
                }
                set(&status, "cursor", rect);
            }
            js_sys::Reflect::set(&js_sys::global(), &"__IME_STATE__".into(), &status).unwrap();
        });
    operad::runtime::Application::new(State::default(), update, view)
        .with_hooks(hooks)
        .run_web(operad::runtime::web::WebRuntimeOptions::new("Text composition").without_status())
        .await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "native-window"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    operad::runtime::Application::new(State::default(), update, view)
        .with_hooks(hooks().with_frame_observer(|state: &State, observation| {
            println!(
                "IME_STATE committed={:?} display={:?} composing={} caret={} commits={} focused={} pressed={} anchor={:?}",
                state.fields[0].text(),
                state.fields[0].display_text(),
                state.fields[0].composition().is_some(),
                state.fields[0].caret(),
                state.commits,
                state.focused[0],
                observation.frame.host_output.state.pressed.is_some(),
                state.fields[0].selection_anchor(),
            );
        }))
        .run_native(operad::runtime::native::NativeWindowOptions::new(
            "Text composition",
        ))?;
    Ok(())
}

#[cfg(any(target_arch = "wasm32", not(feature = "native-window")))]
fn main() {}
