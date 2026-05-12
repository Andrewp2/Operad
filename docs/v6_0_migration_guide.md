# Operad 6.0 Migration Guide

Operad `6.0.0` promotes direct GPU canvas contexts and starts the v6 module
organization while keeping common v5 public paths available for downstream
consumers.

## Upgrade from `5.0.0` to `6.0.0`

1. Update dependency metadata.

```toml
operad = { version = "6.0.0", default-features = false, features = ["widgets", "wgpu"] }
```

Use a local path dependency while validating unreleased downstream changes.

2. Move custom GPU canvas drawing to attached contexts.

The preferred canvas path is now a texture-backed GPU context. Applications can
obtain the context and run a shader pass directly against the canvas surface:

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

The WGPU renderer samples that same texture when it paints the canvas item.
Consumers no longer need to model shader-driven canvas drawing as a keyed
canvas callback or a resource-update upload.

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
- `CanvasRenderRegistry` remains available for app-owned callback-style canvas
  integrations.
- `CanvasContent::gpu_context()` and `UiNode::gpu_canvas(...)` are the preferred
  construction helpers for WGPU-backed shader canvas work.
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
