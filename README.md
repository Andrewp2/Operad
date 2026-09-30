# Operad

[![Crates.io](https://img.shields.io/crates/v/operad.svg)](https://crates.io/crates/operad)
[![Documentation](https://docs.rs/operad/badge.svg)](https://docs.rs/operad)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A cross-platform GUI library for Rust.

## Features

- Simple, renderer-neutral API.
- Retained UI tree with flexible layout.
- Built-in widgets and editor controls.
- Type-safe actions and commands.
- Accessibility and input handling.
- WGPU renderer support.
- Custom drawing surfaces.
- Headless testing utilities.

## Overview

The shortest path to a native window is `run`:

```rust
use operad::{root_style, widgets, LayoutStyle, NativeWindowResult, UiDocument, UiSize};

fn main() -> NativeWindowResult {
    operad::run("app", view)
}

fn view(viewport: UiSize) -> UiDocument {
    let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
    let root = ui.root();

    widgets::button(
        &mut ui,
        root,
        "run",
        "Run",
        widgets::ButtonOptions::new(LayoutStyle::size(140.0, 36.0)),
    );

    ui
}
```

The runner opens the window, creates the renderer, lays out the document, and
routes input. Use `run_app` when widget actions should update application state;
the `showcase` example is a compact app built that way.

The runners retain the document between redraws. They rebuild the view after
application updates, mutable hooks, or viewport changes. Input and animation
frames reuse the document and its layout when possible. View functions describe
application state; use tick actions or hooks for time-dependent state changes.

Runtime state follows the path of node names, so give siblings unique, stable
names. Reordering siblings preserves focus, active gestures, scrolling, and
animation. Removing a node ends its runtime lifetime; moving it to a different
parent starts a new lifetime. Explicit focus and scroll settings in a rebuilt
view take precedence over retained state.

Custom hosts can use `runtime::session::RuntimeSession` for the same lifecycle.
Call `begin_frame` once with monotonic elapsed time before processing a host
frame. Call `invalidate_view` when application state changes, obtain the document
with `build_document`, process input and finish the frame, then return the document
with `retain_document`. Call `frame_presented` after successful rendering to
acknowledge resource uploads, or `frame_failed(now)` to retain them and retry.
Custom hosts honor `frame_retry_delay(now)` before retrying temporary failures.
Use `request_repaint` and `next_frame_delay` for immediate, delayed, or continuous
presentation. View invalidation marks the description stale; it does not wake
the platform event loop by itself. Keep a separate session for each independent UI.
Use `RuntimeSessionOptions` to configure host accessibility capabilities,
rendering preferences, and layout animation. See [Architecture](ARCHITECTURE.md)
for the ownership and invalidation rules.

Use `runtime::Application::new(state, update, view).with_hooks(hooks)` to define
an application once. Launch it with `run_native(NativeWindowOptions)` or
`run_web(WebRuntimeOptions)`. Both hosts accept `runtime::RuntimeHooks` and supply
`RuntimeMetrics`, normalized keyboard events, and `CanvasInput`. The showcase
uses one application definition for both platforms.

The view callback takes `(&State, UiSize, &mut runtime::ViewContext)`. Use
`views.section(&mut document, parent, name, &inputs, builder)` to rebuild an
expensive panel only when its inputs change. Sections preserve normal document
inspection and interaction, and layout retains unchanged text measurements
across revisions. See [selective rebuilding](docs/runtime-integration.md#selective-rebuilding)
for dependency rules and an example.

Canvas hooks run in event order. A canvas with pointer capture keeps receiving
moves and release outside its bounds, even when its hook consumes the press or
the view rebuilds. If its owner disappears or becomes disabled, its hook receives
a cancellation with no current node ID. Widget edits receive equivalent cleanup
through `with_interaction_cancelled`. Use stable action bindings for application
transactions; never save a `UiNodeId` across views.

`RuntimeHooks::with_frame_observer` borrows the final laid-out document and frame
that the host is about to submit. Use it for control geometry, accessibility,
text diagnostics, or paint capture without calling the view again. It is a
submission observation, not a successful-presentation notification. Custom hosts
call `hooks.observe(...)` at the same point; between frames, `session.document()`
provides the retained document. See [runtime integration](docs/runtime-integration.md)
for migration and lifecycle details.

Native and web runners sleep when idle. Input, resize, asynchronous service
responses, background completions, active animations, tick actions, and explicit
repaint requests wake them. Use `runtime::task_channel` with
`RuntimeHooks::with_task_completions` to receive typed job results on the UI
thread and rebuild the view. Applications choose their own threads or async
executor; see [background work](docs/runtime-integration.md#background-work)
for queue limits, cancellation, and custom hosts.
Frame hooks run when a frame is requested; use a tick action or continuous
repaint request for application state that changes with time.

Text inputs support native and browser IME drafts, candidate positioning,
cancellation, and one undoable commit. Draft text is separate from the committed
editing model and follows the editor across document rebuilds. See
[text composition](docs/runtime-integration.md#text-composition) for action
handling, custom editors, and cleanup when removing an active field.

Embedded web hosts should size their canvas through CSS and disable the default
document chrome with `with_document_chrome(false)`. The runner observes that
canvas's size and display scale independently of the browser window.

Web apps use the same retained document contract through the `web-runtime`
feature:

```rust
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    operad::web::run("app", view).await
}
```

For custom WGPU drawing, add a GPU canvas to the document and register a canvas
renderer:

```rust
let mut canvases = operad::NativeWgpuCanvasRenderRegistry::new();
canvases.register("viewport", |state: &mut AppState, context| {
    state.renderer.render(&context.surface)?;
    Ok(operad::CanvasRenderOutput::new())
});

operad::run_app_with_canvas_renderers(options, state, update, view, canvases)?;
```

The renderer callback gets the canvas texture context, so it can record normal
WGPU command buffers and render passes before Operad composites the UI.

Apps that already own a WGPU swapchain or render graph can render Operad into an
existing `TextureView` with `WgpuRenderer::render_frame_into_view_with_encoder`
and `WgpuRenderTargetView::load()`.

## Install

```bash
cargo add operad
```

## Feature Flags

- `widgets`: widget helpers.
- `native-window`: native winit/WGPU windows.
- `web-runtime`: WASM/WebGPU runtime entry points with cosmic-text layout
  measurement.
- `web-showcase`: web runtime plus showcase widgets.
- `wgpu`: WGPU rendering.
- `accesskit-winit`: AccessKit support for winit hosts.
- `text-cosmic`: cosmic-text measurement and shaping.
- `audit`: audit helpers.
- `diagnostics`: debug snapshots and reports.
- `inspector`: diagnostics plus inspector and theme editor widgets.
- `test-support`: headless scenarios, replay, and assertions for application tests.

## Examples

Open a native window:

```bash
cargo run --example showcase --features inspector
```

The starter native template is checked as an ordinary example:

```bash
cargo run --example minimal_native
```

For the web template, build `minimal_web` for `wasm32-unknown-unknown`, run
`wasm-bindgen`, and serve `web/minimal`.

## Development Checks

Use the fast gate while iterating:

```bash
scripts/test-fast.sh
```

That runs formatting, locked all-target/all-feature compilation, all-feature
library tests, and the locked no-default compile gate without running perf smoke
or WGPU snapshot integration tests.

Focused cargo aliases are available for common loops:

```bash
cargo test-native
cargo test-matrix
cargo test-wgpu-snap
cargo test-perf
```

Run the full local gate before release-level handoff:

```bash
scripts/test-full.sh
```

That adds the full all-feature test suite and the supported WASM showcase check
for `wasm32-unknown-unknown`.

Browser-runner connection and cleanup regressions use Node.js 22 without Chrome,
a display, or a GPU:

```bash
node --test scripts/*.test.mjs
```

## Learn More

- [API documentation](https://docs.rs/operad)
- [Examples](examples)
