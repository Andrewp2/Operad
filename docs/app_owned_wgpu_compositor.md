# App-Owned WGPU Compositor Integration

Operad's `wgpu` feature exposes two window/app-owned paths:

- `WgpuSurfaceRenderer` owns surface acquire, render, submit, and present.
- `WgpuRenderer::render_frame_into_view_with_encoder` records Operad UI into a caller-owned `wgpu::CommandEncoder` and `wgpu::TextureView`.

Use the second path when the app already renders a scene and wants Operad UI in the same command buffer.

```rust
use operad::fonts::FontLibrary;
use operad::renderer::{EmptyResourceResolver, RenderFrameRequest, RenderOptions, RenderTarget};
use operad::wgpu_renderer::{WgpuRenderTargetView, WgpuRenderer};
use operad::{CosmicTextMeasurer, UiSize};

let fonts = FontLibrary::new()
    .with_memory_font("ui", include_bytes!("../assets/Inter-Regular.ttf").as_slice())
    .with_sans_serif_family("Inter");

let mut text = CosmicTextMeasurer::with_fonts(fonts.clone());
let mut ui_renderer = WgpuRenderer::with_device_queue_and_fonts(device.clone(), queue.clone(), fonts)?;

let viewport = UiSize::new(width as f32, height as f32);
let mut document = build_document(viewport);
document.compute_layout(viewport, &mut text)?;
let paint = document.paint_list();

let request = RenderFrameRequest::new(
    RenderTarget::app_owned("main", viewport),
    viewport,
    paint,
)
.options(RenderOptions {
    scale_factor,
    ..RenderOptions::default()
});

record_scene(&mut encoder, &scene_view);

ui_renderer.render_frame_into_view_with_encoder(
    request,
    &EmptyResourceResolver,
    &mut encoder,
    WgpuRenderTargetView::new(&scene_view, surface_format).load(),
)?;

queue.submit(Some(encoder.finish()));
```

`WgpuRenderTargetView::load()` preserves the attachment contents and draws Operad UI over them. Use `.clear(color)` when Operad should clear the target before drawing. For multi-layer composition, use the load operation that matches each render pass: the first pass usually clears, later overlay passes usually load.

## Timing

Caller-owned timing is opt-in because Operad does not submit the encoder. Use `render_frame_into_view_with_encoder_timed` with `RenderOptions::collect_gpu_timing = true`, submit the encoder, then resolve the returned token:

```rust
let timed = ui_renderer.render_frame_into_view_with_encoder_timed(
    request,
    &EmptyResourceResolver,
    &mut encoder,
    WgpuRenderTargetView::new(&scene_view, surface_format).load(),
)?;

queue.submit(Some(encoder.finish()));

if let Some(token) = timed.gpu_timing_token {
    let gpu_render = ui_renderer.resolve_gpu_timing(token)?;
}
```

Only one timing token may be unresolved per renderer.

## Resources

The app-owned render target is not a `ResourceResolver` resource. The resolver is only for paint-list resources such as images, textures, thumbnails, and canvas surfaces.

Use `EmptyResourceResolver` when a frame only contains rects, text, paths, and shader-backed materials with no external resource handles.

When a frame includes external resources:

- `ResourceResolver::resolve_resource(id)` must return the current `ResourceDescriptor` for that resource ID.
- Send a full `ResourceUpdate` before the first draw of a resource or when size/format changes.
- Send a partial `ResourceUpdate` only after a compatible full upload exists.
- Partial dirty rects must be non-empty, inside the descriptor size, and carry exactly `width * height * bytes_per_pixel` bytes.
- Increment descriptor versions for new content. Stale versions are rejected or retained according to the cache path.

`ResourceFormat::Rgba8`, `Bgra8`, and `Alpha8` are accepted by the WGPU renderer. BGRA and alpha uploads are converted for the renderer's internal sampled textures.

## Capabilities

Check `renderer.capabilities().rendering.app_owned_view_rendering` or `RenderingCapabilityKind::AppOwnedViewRendering` before selecting this path. `WgpuRenderer` reports support for app-owned view composition.

## Legacy Encoder Path

`WgpuRenderer::render_frame_with_encoder` is kept for compatibility. For `Window` and `AppOwned` targets it records into an internal discard texture, so it is not a compositor integration point. Use `render_frame_into_view_with_encoder` for caller-owned swapchain or texture-view composition.
