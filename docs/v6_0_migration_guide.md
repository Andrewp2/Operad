# Operad 6.0 Migration Guide

Operad `6.0.0` adds the native runner, built-in widgets, app-owned GPU canvas
rendering, and the v6 module organization while keeping common v5 public paths
available for downstream consumers.

## Upgrade from `5.0.0` to `6.0.0`

1. Update dependency metadata.

```toml
operad = { version = "6.0.0", default-features = false, features = ["widgets", "wgpu"] }
```

Use a local path dependency while validating unreleased downstream changes.

2. Move custom GPU canvas drawing to app-owned canvas renderers.

Applications declare a canvas in the UI tree and register a renderer for that
canvas key:

```rust
let mut canvases = operad::NativeWgpuCanvasRenderRegistry::new();
canvases.register(
    "app.viewport",
    |state: &mut AppState, context: operad::NativeWgpuCanvasRenderContext<'_>| {
        state.renderer.render(&context.surface)?;
        Ok(operad::CanvasRenderOutput::new())
    },
);

operad::run_app_with_canvas_renderers(options, state, update, view, canvases)?;
```

In the view, declare a GPU canvas:

```rust
let canvas = operad::CanvasContent::new("app.viewport").gpu_context();
```

The registered renderer receives a `WgpuCanvasContext`. It can create command
encoders, begin render passes, use its own pipelines, and submit any number of
WGPU passes into the canvas texture. Operad samples that texture when it
composites the UI. `WgpuCanvasContext::render_pass` remains a convenience helper
for a single WGSL pass; it is not the primary canvas model.

3. Use the generic GPU naming.

The v6 API intentionally uses `gpu_context`, `get_gpu_context`, and
`WgpuCanvasRenderPass`. Do not use WebGL/WebGL2 terminology for native Operad
canvases; those names describe one browser API family, not Operad's backend
contract.

4. Adopt the v6 module shape incrementally.

The source tree now groups implementation by ownership area:

- `operad::core`
- `operad::interaction`
- `operad::render`
- `operad::runtime`
- `operad::adapters`
- `operad::diagnostics`
- `operad::domain`

Common v5 public paths remain available where compatibility is useful. New code
should prefer the grouped v6 paths when they make ownership clearer.

## Compatibility Notes

- `CanvasContent::new(key)` still exists for retained canvas paint items.
- `NativeWgpuCanvasRenderRegistry` is the native-window path for app-owned WGPU
  canvas renderers.
- `CanvasRenderRegistry` remains available for renderer-neutral callback-style
  canvas integrations.
- `CanvasContent::gpu_context()` and `UiNode::gpu_canvas(...)` are the preferred
  construction helpers for WGPU-backed canvas work.
- `WgpuCanvasContext::begin_render_pass(...)` still exposes the lower-level
  render-pass handle when a consumer needs a custom pipeline.
- `WgpuCanvasContext::render_pass(...)` is the convenience path for simple
  shader passes.

## Validation

Use the release gates in `docs/v6_0_release_checklist.md`. At minimum, customer
adoption branches should pass:

```bash
cargo fmt --all -- --check
cargo check --locked --no-default-features --all-targets
cargo check --locked --all-features --all-targets
cargo test --locked --no-default-features --lib
cargo test --locked --no-default-features --features wgpu canvas_context_render_pass_draws_shader_into_sampled_texture
```
