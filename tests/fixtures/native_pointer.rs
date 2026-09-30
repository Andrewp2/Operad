//! Native showcase input probe. Run with scripts/check-native-pointer.py.
#![allow(dead_code, unused_imports)]

#[cfg(not(target_arch = "wasm32"))]
mod showcase {
    include!("../../examples/showcase.rs");

    pub fn probe() -> NativeWindowResult {
        use std::cell::Cell;
        use std::io::Write;
        use std::rc::Rc;

        let mut state = ShowcaseState::default();
        state.windows.clear_all();
        state.windows.checkbox = true;
        state
            .desktop
            .ensure_window("checkbox", window_defaults("checkbox"));
        let actions = Rc::new(Cell::new(0usize));
        let updated_actions = actions.clone();
        let mut inputs = 0;
        let hooks = operad::runtime::RuntimeHooks::new()
            .with_before_render(|state: &mut ShowcaseState, metrics| state.prepare_frame(metrics.viewport))
            .with_platform_service_requests(|state: &mut ShowcaseState, _| state.platform.drain_requests())
            .with_platform_responses(|state: &mut ShowcaseState, responses| state.apply_platform_responses(responses))
            .with_frame_observer(move |state: &ShowcaseState, observation| {
                inputs += observation.frame.host_output.events.len();
                // Actions can cause a second document pass whose observation no
                // longer contains the original input. Count both kinds of work.
                let processed = inputs + actions.get();
                let mut points = Vec::new();
                for name in [
                    "controls.checkbox.box", "controls.checkbox.label",
                    "checkbox.enabled.box", "checkbox.enabled.label",
                ] {
                    if let Some(node) = observation.document.nodes().iter().find(|node| node.name() == name) {
                        let layout = node.layout();
                        let x = layout.rect.x + layout.rect.width / 2.0;
                        let y = layout.rect.y + layout.rect.height / 2.0;
                        if layout.visible && layout.clip_rect.contains_point(UiPoint::new(x, y)) {
                            points.push(format!("{name:?}:[{x},{y}]"));
                        }
                    }
                }
                println!(
                    "POINTER_STATE {{\"processed\":{processed},\"checked\":{},\"open\":{},\"scroll\":{},\"points\":{{{}}}}}",
                    state.checked, state.windows.checkbox, state.controls_scroll.offset().y,
                    points.join(","),
                );
                std::io::stdout().flush().unwrap();
            });
        operad::runtime::Application::new(
            state,
            move |state: &mut ShowcaseState, action| {
                updated_actions.set(updated_actions.get() + 1);
                state.update(action);
            },
            |state: &ShowcaseState, viewport, _| state.view(viewport),
        )
        .with_hooks(hooks)
        .run_native(NativeWindowOptions::new("Native pointer probe").with_size(900.0, 760.0))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> operad::native::NativeWindowResult {
    showcase::probe()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
