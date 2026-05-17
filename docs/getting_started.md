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

## Platform Services

Apps should request OS/browser behavior through the runtime instead of calling
platform APIs directly. Use `PlatformServiceClient` in app state, drain its
requests through the runtime hook, and handle responses from the matching
response hook.

```rust
let hooks = operad::NativeWindowHooks::new()
    .with_platform_service_requests(|state: &mut AppState, _metrics| {
        state.platform.drain_requests()
    })
    .with_platform_responses(|state: &mut AppState, responses| {
        state.platform.record_responses(responses.iter().cloned());
    });

state.platform.write_clipboard_text("copied text");
state.platform.open_url("https://docs.rs/operad");
```

The same hook names are available on `operad::web::WebRuntimeHooks`. Native
responses are usually immediate. Browser clipboard responses may arrive in a
later frame because the browser APIs are asynchronous.

## Web App

Enable `web-runtime` and export a wasm entry point that calls `operad::web`.
The runtime owns canvas lookup or creation, WebGPU setup, real text measurement
through cosmic-text, resize handling, input routing, scroll persistence,
animation ticking, and status/failure reporting.
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
  workspaces, OS/platform drag-drop, and accessibility support. Profiles expand
  to the same `BackendCapabilityDiagnostic` records as lower-level capability
  checks.
- Use the built-in capability profiles from `BackendCapabilities::native_window()`,
  `BackendCapabilities::web_runtime()`, and `test_host_capabilities()` when
  checking the default native runner, web runner, and deterministic test harness
  behavior.

Prefer the default runtime unless the application has a concrete host or
renderer requirement that the runtime cannot own. The native runtime applies
standard platform services such as text clipboard, open URL, cursor, text IME,
and repaint requests itself; custom hosts can still consume the same
`PlatformRequest` records directly.

## Regression Checks

Applications can use Operad's renderer-neutral diagnostics in their own tests:

- `JustWorkAssertions` checks the blocking layout, clipping, scroll, scrollbar
  range, geometry, hit-target, naming, and paint warnings that cause
  edge-falloff bugs. Failures include the responsible node, reason, measured
  values, and a remediation hint.
- `run_ui_state_matrix` runs the same audit across multiple viewports and
  interaction states.
- `EventReplay::long_wheel_scroll` can stress scroll containers and then assert
  that the target reaches the exact scroll end.
- `EventReplayReport::require_consumed_by` can assert that a visually topmost
  node intentionally consumed input, even when no scroll offset changed.
- `ScenarioHarness` runs document, input, render, platform-request, and timing
  checks without a native or web host.
- `FrameTimingSeries::dominant_section()` summarizes the slowest frame
  subsystem across repeated frames, and timing budget failures include that
  dominant section in their diagnostic message.
- `runtime_error_overlay` turns an `ErrorReport` into an accessible debug
  overlay, so recoverable runtime, renderer, resource, and platform failures
  can be shown with the operation, consequence, next step, and fallback.
- `classify_platform_service_response` turns unsupported, denied, or failed
  platform service responses into actionable `ErrorReport` values with the
  request id, subsystem, consequence, next step, and fallback decision.
- `PlatformAssertions` includes those platform diagnostics in unsupported or
  failed response assertions so host-capability regressions fail with a next
  step instead of only a response enum.
- `scripts/test-fast.sh` and `scripts/test-full.sh` mirror the locked local
  gates used for release handoff; the full gate includes the supported WASM
  showcase check for `wasm32-unknown-unknown`.
- `scripts/check-web-showcase.mjs` runs the built Pages artifact in headless
  Chrome/WebGPU and fails on browser-only startup panics, missing assets,
  console errors, and shader warnings.

These helpers live outside showcase code so downstream apps can keep the same
failure modes under test.
