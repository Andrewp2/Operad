//! One editor application shared by native and browser hosts.
//! Run natively with `cargo run --example runtime_editor`. The adjacent test
//! fixture HTML loads a wasm build made with `--features web-runtime`.

use operad::input::{PointerEventKind, RawInputEvent};
use operad::platform::{CursorGrabMode, CursorRequest, PlatformRequest, PlatformResponse};
#[cfg(any(
    feature = "native-window",
    all(feature = "web-runtime", target_arch = "wasm32")
))]
use operad::runtime::Application;
use operad::runtime::{RuntimeHookResult, RuntimeHooks, RuntimeObservation};
use operad::{
    layout, AccessibilityMeta, AccessibilityRole, ColorRgba, InputBehavior, LayoutStyle, TextStyle,
    UiContent, UiDocument, UiNode, UiPoint, UiSize, UiVisual, WidgetAction, WidgetActionBinding,
};
use operad::{CanvasContent, CanvasInteractionPolicy, KeyCode, PointerButton};
use std::cell::Cell;
use std::rc::Rc;

pub const LONG_LABEL: &str =
    "A long arrangement name with 🎹 tracks, automation, and many more details";

#[derive(Default)]
pub struct Editor {
    pub revision: usize,
    pub downs: usize,
    pub moves: usize,
    pub releases: usize,
    pub cancellations: usize,
    pub blocked_activations: usize,
    pub dragging: bool,
    pub disabled: bool,
    pub removed: bool,
    pub last_local: Option<UiPoint>,
    pub builds: Rc<Cell<usize>>,
    pub requests: Vec<PlatformRequest>,
    pub lock_responses: usize,
    pub raw_motion: usize,
    pub idle: bool,
}

pub fn status(state: &Editor) -> String {
    format!(
        "Revision {} | Drag {} | Down {} | Move {} | Up {} | Cancel {}",
        state.revision,
        state.dragging,
        state.downs,
        state.moves,
        state.releases,
        state.cancellations
    )
}

pub fn update(state: &mut Editor, action: WidgetAction) {
    match action.binding.action_id().map(|id| id.as_str()) {
        Some("reorder") => state.revision += 1,
        Some("disabled") => state.blocked_activations += 1,
        _ => {}
    }
}

pub fn view(
    state: &Editor,
    viewport: UiSize,
    views: &mut operad::runtime::ViewContext<'_>,
) -> UiDocument {
    state.builds.set(state.builds.get() + 1);
    let mut doc = UiDocument::new(LayoutStyle::size(viewport.width, viewport.height));
    let root = doc.root();
    doc.set_node_visual(
        root,
        UiVisual::panel(ColorRgba::new(16, 25, 35, 255), None, 0.0),
    );
    let text = TextStyle {
        color: ColorRgba::WHITE,
        ..TextStyle::default()
    };
    views.section(
        &mut doc,
        root,
        "toolbar",
        &(viewport, text.clone()),
        |(viewport, text), _| {
            let mut doc = UiDocument::new(layout::absolute(0.0, 0.0, viewport.width, 60.0));
            let root = doc.root();
            doc.add_child(
                root,
                UiNode::text(
                    "arrangement.name",
                    LONG_LABEL,
                    text.clone().ellipsis(),
                    layout::absolute(24.0, 18.0, 220.0, 30.0),
                )
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Label).label(LONG_LABEL),
                ),
            );
            doc.add_child(
                root,
                UiNode::text(
                    "reorder",
                    "Move controls",
                    text.clone(),
                    layout::absolute(260.0, 18.0, 150.0, 32.0),
                )
                .with_visual(UiVisual::panel(ColorRgba::new(53, 75, 101, 255), None, 5.0))
                .with_input(InputBehavior::BUTTON)
                .with_action(WidgetActionBinding::action("reorder")),
            );
            doc
        },
    );
    // Changing siblings and geometry during capture models an editor rebuilding
    // selection tools and inspectors in response to every edit.
    if state.revision % 2 == 1 {
        doc.add_child(
            root,
            UiNode::text(
                "selection.tools",
                "Selection tools",
                text.clone(),
                layout::absolute(24.0, 60.0, 200.0, 28.0),
            ),
        );
    }
    if !state.removed {
        let canvas = doc.add_child(
            root,
            UiNode::canvas(
                "timeline",
                "timeline",
                layout::absolute(
                    24.0 + (state.revision % 2) as f32 * 12.0,
                    100.0,
                    320.0,
                    160.0,
                ),
            )
            .with_visual(UiVisual::panel(ColorRgba::new(34, 48, 66, 255), None, 6.0)),
        );
        doc.set_node_content(
            canvas,
            UiContent::Canvas(
                CanvasContent::new("timeline").interaction(CanvasInteractionPolicy::EDITOR),
            ),
        );
        doc.set_node_enabled(canvas, !state.disabled);
        for track in 0..3 {
            doc.add_child(
                canvas,
                UiNode::container(
                    format!("track.{track}"),
                    layout::absolute(16.0, 20.0 + track as f32 * 42.0, 180.0, 28.0),
                )
                .with_visual(UiVisual::panel(
                    ColorRgba::new(45, 116, 129, 255),
                    None,
                    4.0,
                )),
            );
        }
    }
    let disabled = doc.add_child(
        root,
        UiNode::text(
            "disabled.control",
            "Unavailable",
            text.clone(),
            layout::absolute(260.0, 110.0, 110.0, 40.0),
        )
        .with_visual(UiVisual::panel(ColorRgba::new(77, 77, 81, 255), None, 4.0))
        .with_input(InputBehavior::BUTTON)
        .with_action(WidgetActionBinding::action("disabled")),
    );
    doc.set_node_enabled(disabled, false);
    doc.add_child(
        root,
        UiNode::text(
            "status",
            status(state),
            text.clone(),
            layout::absolute(24.0, 282.0, 660.0, 30.0),
        ),
    );
    doc.add_child(
        root,
        UiNode::text(
            "help",
            "Drag the tracks. D toggles the editor. R removes it.",
            text,
            layout::absolute(24.0, 322.0, 640.0, 30.0),
        ),
    );
    doc
}

pub fn hooks(
    observer: impl for<'a> FnMut(&Editor, RuntimeObservation<'a>) + 'static,
) -> RuntimeHooks<Editor> {
    RuntimeHooks::new()
        .with_canvas_input(|state: &mut Editor, event| {
            let RawInputEvent::Pointer(pointer) = event.input else {
                return false;
            };
            state.last_local = event.local_position;
            match pointer.kind {
                PointerEventKind::Down(PointerButton::Primary) => {
                    state.dragging = true;
                    state.downs += 1;
                    state.revision += 1;
                }
                PointerEventKind::Move if state.dragging => {
                    state.moves += 1;
                    state.revision += 1;
                }
                PointerEventKind::Up(PointerButton::Primary) if state.dragging => {
                    state.dragging = false;
                    state.releases += 1;
                    state.revision += 1;
                }
                PointerEventKind::Cancel if state.dragging => {
                    state.dragging = false;
                    state.cancellations += 1;
                    state.revision += 1;
                }
                _ => {}
            }
            true
        })
        .with_keyboard_input(|state: &mut Editor, input| {
            let key = input.event;
            if !key.pressed {
                return false;
            }
            match key.key {
                KeyCode::Character('d' | 'D') => state.disabled = !state.disabled,
                KeyCode::Character('r' | 'R') => state.removed = !state.removed,
                KeyCode::Character('i' | 'I') => state.idle = !state.idle,
                KeyCode::Character('l' | 'L') => {
                    state
                        .requests
                        .push(PlatformRequest::Cursor(CursorRequest::SetGrab(
                            CursorGrabMode::Locked,
                        )))
                }
                KeyCode::Character('u' | 'U') => {
                    state
                        .requests
                        .push(PlatformRequest::Cursor(CursorRequest::SetGrab(
                            CursorGrabMode::None,
                        )))
                }
                _ => return false,
            }
            state.revision += 1;
            true
        })
        .with_platform_requests(|state, _| {
            RuntimeHookResult::unchanged(std::mem::take(&mut state.requests))
        })
        .with_platform_responses(|state, responses| {
            state.lock_responses += responses
                .iter()
                .filter(|response| matches!(response.response, PlatformResponse::Cursor(_)))
                .count();
            RuntimeHookResult::unchanged(())
        })
        .with_raw_mouse_motion(|state, _| {
            state.raw_motion += 1;
            true
        })
        .with_idle_redraw(|state| state.idle)
        .with_frame_observer(observer)
}

pub fn observed_status(observation: RuntimeObservation<'_>) -> &str {
    observation
        .document
        .nodes()
        .iter()
        .find_map(|node| match (node.name(), node.content()) {
            ("status", UiContent::Text(content)) => Some(content.text.as_str()),
            _ => None,
        })
        .expect("the computed editor document contains its status")
}

#[cfg(any(
    feature = "native-window",
    all(feature = "web-runtime", target_arch = "wasm32")
))]
fn application() -> Application<Editor> {
    Application::new(Editor::default(), update, view).with_hooks(hooks(|state, observation| {
        assert_eq!(observed_status(observation), status(state));
        #[cfg(all(target_arch = "wasm32", feature = "web-runtime"))]
        publish(state, observation);
    }))
}

#[cfg(all(target_arch = "wasm32", feature = "web-runtime"))]
fn publish(state: &Editor, observation: RuntimeObservation<'_>) {
    use wasm_bindgen::JsValue;
    let window = web_sys::window().unwrap();
    let value = js_sys::Object::new();
    for (key, number) in [
        ("revision", state.revision),
        ("downs", state.downs),
        ("moves", state.moves),
        ("releases", state.releases),
        ("cancellations", state.cancellations),
        ("blockedActivations", state.blocked_activations),
        ("builds", state.builds.get()),
        ("sectionsRebuilt", observation.view_build_stats.rebuilt),
        ("sectionsReused", observation.view_build_stats.reused),
        ("lockResponses", state.lock_responses),
        ("rawMotion", state.raw_motion),
    ] {
        js_sys::Reflect::set(
            &value,
            &JsValue::from_str(key),
            &JsValue::from_f64(number as f64),
        )
        .unwrap();
    }
    js_sys::Reflect::set(&value, &"dragging".into(), &state.dragging.into()).unwrap();
    if let Some(point) = state.last_local {
        js_sys::Reflect::set(&value, &"localX".into(), &point.x.into()).unwrap();
        js_sys::Reflect::set(&value, &"localY".into(), &point.y.into()).unwrap();
    }
    js_sys::Reflect::set(&value, &"disabled".into(), &state.disabled.into()).unwrap();
    js_sys::Reflect::set(&value, &"removed".into(), &state.removed.into()).unwrap();
    js_sys::Reflect::set(&value, &"idle".into(), &state.idle.into()).unwrap();
    js_sys::Reflect::set(
        &value,
        &"status".into(),
        &observed_status(observation).into(),
    )
    .unwrap();
    js_sys::Reflect::set(&window, &"__OPERAD_EDITOR__".into(), &value).unwrap();
    if !js_sys::Reflect::has(&window, &"__OPERAD_EDITOR_BUILD_COUNT__".into()).unwrap() {
        use wasm_bindgen::JsCast;
        let builds = state.builds.clone();
        let getter = wasm_bindgen::closure::Closure::<dyn Fn() -> f64>::wrap(Box::new(move || {
            builds.get() as f64
        }));
        js_sys::Reflect::set(
            &window,
            &"__OPERAD_EDITOR_BUILD_COUNT__".into(),
            getter.as_ref().unchecked_ref(),
        )
        .unwrap();
        getter.forget();
    }
    if let Some(status) = window.document().unwrap().get_element_by_id("probe-status") {
        status.set_text_content(Some(observed_status(observation)));
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "native-window"))]
fn main() -> operad::native::NativeWindowResult {
    application().run_native(
        operad::native::NativeWindowOptions::new("Arrangement editor").with_size(700.0, 420.0),
    )
}

#[cfg(any(target_arch = "wasm32", not(feature = "native-window")))]
fn main() {}

#[cfg(all(target_arch = "wasm32", feature = "web-runtime"))]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    application()
        .run_web(
            operad::web::WebRuntimeOptions::new("Arrangement editor")
                .without_status()
                .with_document_chrome(false),
        )
        .await
}
