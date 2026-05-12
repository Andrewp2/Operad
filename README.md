# Operad

Operad is a renderer-neutral UI toolkit contract layer for Rust applications.
It lets applications describe layout, input, widget state, actions, rendering
intent, accessibility, diagnostics, resources, and host/runtime behavior without
binding product code to one renderer or native window stack.

Version `6.0.0` is the v6 public API for direct GPU canvas contexts plus the
module-organization foundation. The
crate provides backend-neutral records and optional adapters; applications still
own product state, command handlers, persistence, native event loops, and final
business behavior.

## What Is Included

- Retained `UiDocument` trees with layout, hit testing, focus, scrolling,
  effective geometry, accessibility metadata, and renderer-neutral paint output.
- Widget helpers for controls, menus,
  command palettes, text input, selection/copy policy, data widgets, pickers,
  surfaces, toasts, split panes, and editor-oriented primitives.
- Action, command, transaction, selection, form, task, overlay, navigation,
  virtualization, diagnostic, theme, resource, and font lifecycle contracts.
- Paint-list testing helpers and optional WGPU rendering paths.
- Optional `glyphon`/`cosmic-text` backed text paths and an optional
  `accesskit-winit` bridge for native accessibility publication.

## Install

The default feature set is demo-friendly: it includes `widgets` plus the native
WGPU window stack so the showcase example runs with Cargo's default example
command.

```toml
[dependencies]
operad = "6.0.0"
```

Library consumers that only need backend-neutral contracts should opt out of
defaults and enable only the integrations they need:

```toml
operad = { version = "6.0.0", default-features = false, features = ["widgets"] }
```

Enable renderer or host integrations explicitly:

```toml
operad = { version = "6.0.0", default-features = false, features = ["widgets", "wgpu"] }
```

Common feature flags:

- `widgets`: domain-neutral widget helpers, enabled by default.
- `wgpu`: WGPU renderer support, including glyphon text rendering.
- `native-window`: native winit/WGPU surface support, enabled by default so the
  showcase opens with `cargo run --example operad_showcase`.
- `accesskit-winit`: AccessKit publication bridge for winit hosts.
- `text-cosmic`: cosmic-text measurement/shaping support.
- `egui`: egui host/input compatibility.
- `audit`: audit-oriented helpers.

## Examples

Open the full v6 showcase UI in a native WGPU window:

```bash
cargo run --locked --example operad_showcase
```

For headless CI, validate the same showcase document without visual rendering:

```bash
OPERAD_SHOWCASE_HEADLESS=1 \
cargo run --locked --example operad_showcase
```

Capture the showcase through the real WGPU snapshot path:

```bash
OPERAD_SHOWCASE_WGPU_SCREENSHOT=target/operad-showcase/showcase.png \
OPERAD_SHOWCASE_VIEWPORT=1280x800 \
cargo run --locked --example operad_showcase
```

Run the native WGPU host example as an offscreen smoke:

```bash
cargo run --locked --features wgpu --example native_wgpu_host
```

Native OS-surface execution is opt-in because it requires a display and WGPU
adapter:

```bash
OPERAD_RUN_WGPU_EXAMPLE_WINDOW=1 \
OPERAD_WGPU_EXAMPLE_WINDOW_FRAMES=3 \
cargo run --locked --features native-window --example native_wgpu_host
```

## Canvas Contexts

Canvas nodes can be attached to a GPU texture context, matching the HTML canvas
model where an app obtains a drawing context and renders into the canvas surface
directly:

```rust
let canvas = operad::CanvasContent::new("app.viewport").gpu_context();
let context = renderer.get_gpu_context(&canvas, operad::PixelSize::new(640, 360))?;
context.render_pass(operad::WgpuCanvasRenderPass::fragment(r#"
@fragment
fn fs_main(input: OperadCanvasVertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.uv, 0.4, 1.0);
}
"#))?;
```

The WGPU renderer samples that same canvas texture when it encounters the canvas
paint item, so consumers do not need to route shader drawing through a separate
canvas render callback or resource-update upload.

## Documentation

Start with the v6 migration and organization notes:

- `docs/v6_0_migration_guide.md`
- `docs/v6_0_module_organization.md`
- `docs/v6_0_release_checklist.md`

The v5 concept docs remain available as historical background for the contract
model:

- `docs/v5_0_core_concepts.md`
- `docs/v5_0_theme_and_stability.md`
- `docs/v5_0_completion_audit.md`

The core rule for consumers is to keep backend-specific APIs behind local host
or renderer adapters. Product-facing code should prefer Operad's stable
backend-neutral records for layout, actions, transactions, accessibility,
diagnostics, resources, and rendering intent.

## Release Validation

The baseline v6 release gates are:

```bash
cargo fmt --all -- --check
cargo check --locked --no-default-features --all-targets
cargo test --locked --no-default-features
cargo check --locked --all-features --all-targets
cargo test --locked --all-features -- --list
cargo check --locked --all-features --examples
cargo doc --locked --all-features --no-deps
cargo package --locked
```

Perf and WGPU validation commands are listed in
`docs/v6_0_release_checklist.md`.

## License

MIT. See `LICENSE`.
