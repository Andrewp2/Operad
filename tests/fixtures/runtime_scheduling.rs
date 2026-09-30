//! Interactive browser regression probe for repaint scheduling. Ticks are off
//! unless the clock regression probe explicitly enables them before startup.
//! Build with --target wasm32-unknown-unknown --no-default-features
//! --features web-runtime,widgets --example runtime_scheduling, run wasm-bindgen
//! with --target web --out-name runtime_scheduling, and serve the adjacent HTML.

#[cfg(target_arch = "wasm32")]
mod probe {
    use operad::platform::{
        ClipboardRequest, CursorGrabMode, CursorRequest, PlatformRequest, PlatformResponse,
        RepaintRequest,
    };
    use operad::runtime::{task_channel, TaskSender};
    use operad::{
        widgets, LayoutStyle, TextStyle, UiDocument, UiNode, UiSize, WidgetAction, WidgetActionKind,
    };
    use std::time::Duration;

    struct State {
        frames: u64,
        ticks: u64,
        count: u64,
        response_seen: bool,
        continuous: bool,
        delayed_at: Option<f64>,
        delayed_complete: bool,
        requests: Vec<PlatformRequest>,
        task_pending: bool,
        task_result: Option<Result<u32, String>>,
        text: widgets::TextInputState,
        text_focused: bool,
    }

    impl Default for State {
        fn default() -> Self {
            Self {
                frames: 0,
                ticks: 0,
                count: 0,
                response_seen: false,
                continuous: false,
                delayed_at: None,
                delayed_complete: false,
                requests: Vec::new(),
                task_pending: false,
                task_result: None,
                text: widgets::TextInputState::new(""),
                text_focused: false,
            }
        }
    }

    fn update(state: &mut State, action: WidgetAction, sender: &TaskSender<Result<u32, String>>) {
        match action.binding.action_id().map(|id| id.as_str()) {
            Some("tick") => state.ticks += 1,
            Some("text") => match action.kind {
                WidgetActionKind::Focus(change) => state.text_focused = change.focused,
                WidgetActionKind::TextEdit(edit) => {
                    state
                        .text
                        .apply_widget_text_edit(&edit, &widgets::TextInputOptions::default());
                }
                _ => {}
            },
            Some("increment") => state.count += 1,
            Some("lock") => state
                .requests
                .push(PlatformRequest::Cursor(CursorRequest::SetGrab(
                    CursorGrabMode::Locked,
                ))),
            Some("task") if !state.task_pending => {
                state.task_pending = true;
                state.task_result = None;
                // The browser probe controls completion independently of input
                // so it can verify that a pending job leaves the renderer idle.
                let promise = js_sys::Promise::new(&mut |resolve, reject| {
                    js_sys::Reflect::set(&js_sys::global(), &"__completeTask".into(), &resolve)
                        .unwrap();
                    js_sys::Reflect::set(&js_sys::global(), &"__failTask".into(), &reject).unwrap();
                });
                let sender = sender.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = wasm_bindgen_futures::JsFuture::from(promise)
                        .await
                        .map(|value| value.as_f64().unwrap() as u32)
                        .map_err(|error| error.as_string().unwrap_or_else(|| "failed".into()));
                    match sender.try_send(result) {
                        Ok(()) => {}
                        Err(operad::runtime::TaskSendError::Closed(_)) => {
                            js_sys::Reflect::set(
                                &js_sys::global(),
                                &"__taskReceiverClosed".into(),
                                &true.into(),
                            )
                            .unwrap();
                        }
                        Err(error) => panic!("unexpected task completion failure: {error}"),
                    }
                });
            }
            Some("async") => {
                // The isolated test browser may allow or deny this write. Both
                // paths complete asynchronously and must wake an idle app.
                state
                    .requests
                    .push(PlatformRequest::Clipboard(ClipboardRequest::WriteText(
                        "operad scheduling probe".into(),
                    )));
            }
            Some("delayed") => {
                // Deadline starts when requested, even if rendering has been idle.
                let now = web_sys::window().unwrap().performance().unwrap().now();
                state.delayed_at = Some(now + 500.0);
                state.delayed_complete = false;
                state
                    .requests
                    .push(PlatformRequest::Repaint(RepaintRequest::After(
                        Duration::from_millis(500),
                    )));
            }
            Some("continuous") => {
                state.continuous = !state.continuous;
                state
                    .requests
                    .push(PlatformRequest::Repaint(RepaintRequest::Continuous {
                        active: state.continuous,
                    }));
            }
            _ => {}
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
        if js_sys::Reflect::get(&js_sys::global(), &"__OPERAD_RENDER_FAILURE__".into())
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            // An embedded GPU program on a native viewport is an invalid
            // renderer configuration. It returns Backend before any GPU work,
            // exercising terminal frame errors independently of device loss.
            widgets::canvas(
                &mut doc,
                root,
                "invalid-canvas",
                operad::CanvasContent::new("invalid-canvas")
                    .wgsl("")
                    .native_viewport(),
                widgets::CanvasOptions {
                    layout: LayoutStyle::size(32.0, 32.0),
                    ..Default::default()
                },
            );
        }
        for (id, label) in [
            ("increment", "Increment"),
            ("delayed", "Repaint after 500 ms"),
            ("continuous", "Toggle continuous rendering"),
            ("async", "Request async service"),
            ("task", "Start background job"),
            ("lock", "Lock pointer"),
        ] {
            widgets::button(
                &mut doc,
                root,
                id,
                label,
                widgets::ButtonOptions::new(LayoutStyle::size(300.0, 44.0)).with_action(id),
            );
        }
        widgets::text_input(
            &mut doc,
            root,
            "text",
            &state.text,
            widgets::TextInputOptions {
                focused: state.text_focused,
                ..widgets::TextInputOptions::default()
                    .with_layout(LayoutStyle::size(300.0, 44.0))
                    .with_edit_action("text")
            },
        );
        let status = format!(
            "Frames: {} | Count: {} | Continuous: {} | Delayed: {} | Width: {} | Async: {} | Task: {}",
            state.frames,
            state.count,
            state.continuous,
            if state.delayed_complete {
                "complete"
            } else if state.delayed_at.is_some() {
                "pending"
            } else {
                "none"
            },
            viewport.width,
            state.response_seen,
            if state.task_pending { "pending".to_owned() } else {
                match &state.task_result {
                    Some(Ok(value)) => value.to_string(),
                    Some(Err(_)) => "error".to_owned(),
                    None => "none".to_owned(),
                }
            }
        );
        doc.add_child(
            root,
            UiNode::text(
                "status",
                status.clone(),
                TextStyle::default(),
                LayoutStyle::size(700.0, 30.0),
            ),
        );
        doc
    }

    #[wasm_bindgen::prelude::wasm_bindgen]
    pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
        let tick_probe = js_sys::Reflect::get(&js_sys::global(), &"__OPERAD_CLOCK_PROBE__".into())
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let (sender, receiver) = task_channel(1);
        let hooks = operad::runtime::RuntimeHooks::new()
            .with_close_requested(|_: &mut State| false)
            .with_task_completions(receiver, |state: &mut State, result| {
                state.task_pending = false;
                state.task_result = Some(result);
            })
            .with_before_render(|state: &mut State, _| {
                state.frames += 1;
                // Runtime metrics use a runtime-relative clock. Compare against
                // the same browser clock used when the repaint was requested.
                let now = web_sys::window().unwrap().performance().unwrap().now();
                if state.delayed_at.is_some_and(|deadline| now >= deadline) {
                    state.delayed_at = None;
                    state.delayed_complete = true;
                }
            })
            .with_frame_observer(move |state: &State, observation| {
                if tick_probe {
                    let report = js_sys::Object::new();
                    for (key, value) in [
                        ("frames", state.frames as f64),
                        ("ticks", state.ticks as f64),
                        (
                            "elapsed",
                            observation.metrics.elapsed.as_secs_f64() * 1000.0,
                        ),
                    ] {
                        js_sys::Reflect::set(&report, &key.into(), &value.into()).unwrap();
                    }
                    js_sys::Reflect::set(&report, &"text".into(), &state.text.text().into())
                        .unwrap();
                    js_sys::Reflect::set(&js_sys::global(), &"__CLOCK_STATE__".into(), &report)
                        .unwrap();
                }
                // Mirror the text that actually reached the final frame. Reading
                // observations and UAT snapshots must never build another view.
                let status = observation
                    .document
                    .nodes()
                    .iter()
                    .find(|node| node.name() == "status")
                    .and_then(|node| match node.content() {
                        operad::UiContent::Text(text) => Some(text.text.as_str()),
                        _ => None,
                    })
                    .unwrap();
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("probe-status")
                    .unwrap()
                    .set_text_content(Some(status));
            })
            .with_platform_requests(|state: &mut State, _| std::mem::take(&mut state.requests))
            .with_platform_responses(|state: &mut State, responses| {
                state.response_seen |= responses
                    .iter()
                    .any(|response| matches!(response.response, PlatformResponse::Clipboard(_)));
            });
        let mut options =
            operad::web::WebRuntimeOptions::new("Runtime scheduling").with_document_chrome(false);
        if tick_probe {
            options = options.with_tick_action("tick").with_tick_rate_hz(20.0);
        }
        let with_status =
            js_sys::Reflect::get(&js_sys::global(), &"__OPERAD_DEVICE_LOSS_STATUS__".into())
                .ok()
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
        operad::runtime::Application::new(
            State::default(),
            move |state, action| update(state, action, &sender),
            view,
        )
        .with_hooks(hooks)
        .run_web(if with_status {
            options.with_status_id("device-loss-status")
        } else {
            options.without_status()
        })
        .await
    }
}

fn main() {}
