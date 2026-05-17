# Getting Started

Operad's default path is the runtime path. It should be the first thing a new
application tries before building a custom host.

## Native App

Use `operad::run` for a stateless document, or `operad::run_app` when widget
actions should update application state.

```rust
use operad::{root_style, widgets, LayoutStyle, NativeWindowResult, UiDocument, UiSize};

fn main() -> NativeWindowResult {
    operad::run("app", view)
}

fn view(viewport: UiSize) -> UiDocument {
    let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
    widgets::button(
        &mut ui,
        ui.root,
        "run",
        "Run",
        widgets::ButtonOptions::new(LayoutStyle::size(140.0, 36.0)),
    );
    ui
}
```

Run the smallest checked template with:

```bash
cargo run --example minimal_native
```

## Stateful App

Stateful apps use the same runtime. The update function receives widget actions;
the view function rebuilds the retained document from the current state.

```rust
operad::run_app("app", state, update, view)?;
```

Use `examples/simple_form.rs` as the smallest stateful text-input example.

## Web App

Enable `web-runtime` and export a wasm entry point that calls `operad::web`.
The runtime owns canvas lookup or creation, WebGPU setup, resize handling, input
routing, scroll persistence, animation ticking, and status/failure reporting.
If WebGPU startup fails, the runtime writes the failing operation, consequence,
and next step into the configured status element before returning the error.
Browser clipboard read/write requests are serviced asynchronously and delivered
back through the normal platform response queue.

```rust
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    operad::web::run("app", view).await
}
```

Build the minimal web template with:

```bash
cargo build --release --target wasm32-unknown-unknown \
  --no-default-features --features web-runtime --example minimal_web

wasm-bindgen --target web --out-dir web/minimal/pkg \
  --out-name minimal_web \
  target/wasm32-unknown-unknown/release/examples/minimal_web.wasm
```

Then serve `web/minimal` from a local static file server.

## Checked Templates

The maintained starter examples are ordinary examples, so `cargo check
--all-features --examples` and CI keep them from drifting:

- `minimal_native`: smallest native runtime app.
- `minimal_web`: smallest WASM/WebGPU runtime app.
- `simple_form`: state, text input, and actions.
- `canvas_app`: WGPU canvas with runtime ticking.
- `command_palette_hotkeys`: command metadata, shortcuts, and palette UI.
- `docked_workspace`: docked panels with public workspace widgets.
- `theme_customization`: theme snapshot/editor surface.
- `animation_state_machine`: state-machine input and scene morphing.

## Escape Hatches

The default runtime is not the only way to use Operad. Advanced applications can
still own lower-level pieces:

- Use `run_app_with` to configure window size, minimum size, UI scale, and tick
  actions.
- Use `run_app_with_canvas_renderers` when a canvas needs custom WGPU rendering.
- Use the host, platform, renderer, and diagnostics modules directly for custom
  hosts or test harnesses.
- Query `BackendCapabilities` before depending on platform behavior such as
  pointer lock, raw mouse motion, cursor grab, clipboard, IME, WebGPU, or native
  child windows.
- Query `BackendCapabilityProfile` for product-level needs such as command
  hotkeys, text editing, canvas pointer editing, 3D flycam controls, docked
  workspaces, and accessibility support. Profiles expand to the same
  `BackendCapabilityDiagnostic` records as lower-level capability checks.

Prefer the default runtime unless the application has a concrete host or
renderer requirement that the runtime cannot own. The native runtime applies
standard platform services such as text clipboard, open URL, cursor, and repaint
requests itself; custom hosts can still consume the same `PlatformRequest`
records directly.

## Regression Checks

Applications can use Operad's renderer-neutral diagnostics in their own tests:

- `JustWorkAssertions` checks the blocking layout, clipping, scroll, geometry,
  hit-target, naming, and paint warnings that cause edge-falloff bugs.
- `run_ui_state_matrix` runs the same audit across multiple viewports and
  interaction states.
- `EventReplay::long_wheel_scroll` can stress scroll containers and then assert
  that the target reaches the exact scroll end.
- `ScenarioHarness` runs document, input, render, platform-request, and timing
  checks without a native or web host.
- `runtime_error_overlay` turns an `ErrorReport` into an accessible debug
  overlay, so recoverable runtime, renderer, resource, and platform failures
  can be shown with the operation, consequence, next step, and fallback.

These helpers live outside showcase code so downstream apps can keep the same
failure modes under test.
