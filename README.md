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
    let root = ui.root;

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

## Install

```bash
cargo add operad
```

## Feature Flags

- `widgets`: widget helpers.
- `native-window`: native winit/WGPU windows.
- `web-runtime`: WASM/WebGPU runtime entry points.
- `web-showcase`: web runtime plus showcase widgets.
- `wgpu`: WGPU rendering.
- `accesskit-winit`: AccessKit support for winit hosts.
- `text-cosmic`: cosmic-text measurement and shaping.
- `egui`: egui host/input compatibility.
- `egui-renderer-compat`: egui renderer compatibility.
- `audit`: audit helpers.

## Examples

Open a native window:

```bash
cargo run --example showcase
```

Starter templates are checked as ordinary examples:

```bash
cargo run --example minimal_native
cargo run --example simple_form
cargo run --example docked_workspace
```

For the web template, build `minimal_web` for `wasm32-unknown-unknown`, run
`wasm-bindgen`, and serve `web/minimal`.

## Development Checks

Use the fast gate while iterating:

```bash
scripts/test-fast.sh
```

That runs all-target/all-feature compilation, all-feature library tests, and the
no-default compile gate without running perf smoke or WGPU snapshot integration
tests.

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

## Learn More

- [API documentation](https://docs.rs/operad)
- [Getting started](docs/getting_started.md)
- [Examples](examples)
- [Documentation](docs)
