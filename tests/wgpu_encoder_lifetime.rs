#![cfg(feature = "wgpu")]

use operad::platform::{ImageHandle, LayerOrder, PixelSize};
use operad::renderer::{
    EmptyResourceResolver, PixelRect, RenderFrameRequest, RenderOptions, RenderTarget,
    RendererAdapter, ResourceDescriptor, ResourceFormat, ResourceUpdate,
};
use operad::wgpu_renderer::{WgpuRenderTargetView, WgpuRenderer};
use operad::{
    CanvasContent, CanvasRenderProgram, ColorRgba, CornerRadii, PaintCompositorLayer, PaintEffect,
    PaintItem, PaintKind, PaintList, PaintRect, PaintTransform, StrokeStyle, TextContent,
    TextStyle, UiNodeId, UiPoint, UiRect, UiSize,
};

#[derive(Clone, Copy, Debug)]
enum Content {
    Geometry,
    SdfGradient,
    CompositedSdfGradient,
    Text,
    CompositedText,
    Image,
    CompositedImage,
    NestedImage,
    ShaderImage,
    Canvas,
    CompositedCanvas,
    InvalidCanvas,
}

#[test]
fn deferred_passes_match_individually_submitted_frames() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("GPU adapter");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("GPU device");
    eprintln!("deferred-render adapter: {:?}", adapter.get_info());
    let mut mismatches = Vec::new();
    for content in [
        Content::Geometry,
        Content::SdfGradient,
        Content::CompositedSdfGradient,
        Content::Text,
        Content::CompositedText,
        Content::Image,
        Content::CompositedImage,
        Content::NestedImage,
        Content::ShaderImage,
        Content::Canvas,
        Content::CompositedCanvas,
        Content::InvalidCanvas,
    ] {
        let sizes = [
            PixelSize::new(64, 64),
            PixelSize::new(96, 48),
            PixelSize::new(80, 72),
        ];
        let requests = sizes
            .iter()
            .enumerate()
            .map(|(frame, size)| request(content, frame, *size))
            .collect::<Vec<_>>();
        let mut reference = WgpuRenderer::with_device_queue(device.clone(), queue.clone()).unwrap();
        let expected = requests
            .iter()
            .zip(sizes)
            .map(|(request, size)| {
                let mut request = request.clone();
                request.target = RenderTarget::snapshot(size);
                let image = reference
                    .render_frame(request, &EmptyResourceResolver)
                    .expect("individual frame")
                    .snapshot
                    .unwrap();
                assert!(
                    image
                        .pixels
                        .chunks_exact(4)
                        .any(|pixel| pixel[..3].iter().any(|c| *c > 0)),
                    "reference content must be visible"
                );
                image.pixels
            })
            .collect::<Vec<_>>();

        for separate_encoders in [false, true] {
            let mut renderer =
                WgpuRenderer::with_device_queue(device.clone(), queue.clone()).unwrap();
            let targets = sizes.map(|size| ReadbackTarget::new(&device, size));
            let mut encoder = device.create_command_encoder(&Default::default());
            let mut commands = Vec::new();
            for (request, target) in requests.iter().zip(&targets) {
                renderer
                    .render_frame_into_view_with_encoder(
                        request.clone(),
                        &EmptyResourceResolver,
                        &mut encoder,
                        WgpuRenderTargetView::new(&target.view, wgpu::TextureFormat::Rgba8Unorm),
                    )
                    .expect("record deferred frame");
                target.copy_to_buffer(&mut encoder);
                if separate_encoders {
                    commands.push(encoder.finish());
                    encoder = device.create_command_encoder(&Default::default());
                }
            }
            commands.push(encoder.finish());
            queue.submit(commands);
            for (frame, (target, expected)) in targets.iter().zip(&expected).enumerate() {
                let actual = target.read(&device);
                if actual != *expected {
                    let pixels = actual
                        .chunks_exact(4)
                        .zip(expected.chunks_exact(4))
                        .filter(|(a, b)| a != b)
                        .count();
                    mismatches.push(format!("{content:?}, separate encoders={separate_encoders}, frame {frame}: {pixels} pixels"));
                }
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "deferred passes changed after recording:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn deferred_image_updates_respect_small_buffer_limits() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits {
            max_buffer_size: 16_384,
            max_uniform_buffer_binding_size: 16_384,
            max_storage_buffer_binding_size: 16_384,
            ..Default::default()
        },
        ..Default::default()
    }))
    .unwrap();
    eprintln!(
        "image upload adapter: {:?}, buffer limit: {}",
        adapter.get_info(),
        device.limits().max_buffer_size
    );
    let size = PixelSize::new(64, 32);
    // Padded rows exceed the budget vertically; the wide image also needs
    // horizontal splits. Partial updates start inside the original image.
    for image_size in [PixelSize::new(129, 133), PixelSize::new(4097, 5)] {
        let mut reference = WgpuRenderer::with_device_queue(device.clone(), queue.clone()).unwrap();
        let mut deferred = WgpuRenderer::with_device_queue(device.clone(), queue.clone()).unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        let mut outputs = Vec::new();
        for partial in [false, true] {
            let rect = if partial {
                PixelRect::new(1, 1, image_size.width - 2, image_size.height - 2)
            } else {
                PixelRect::new(0, 0, image_size.width, image_size.height)
            };
            let mut pixels = Vec::new();
            for y in 0..rect.height {
                for x in 0..rect.width {
                    pixels.extend_from_slice(&[
                        x as u8,
                        y as u8,
                        if partial { 255 } else { 80 },
                        255,
                    ]);
                }
            }
            let descriptor = ResourceDescriptor::new(
                ImageHandle::app("deferred-image"),
                image_size,
                ResourceFormat::Rgba8,
            );
            let update = if partial {
                ResourceUpdate::partial(descriptor, rect, pixels)
            } else {
                ResourceUpdate::rgba8_image(ImageHandle::app("deferred-image"), image_size, pixels)
            };
            let mut request = request(Content::Image, 0, size);
            request.resource_updates = vec![update];
            let mut snapshot_request = request.clone();
            snapshot_request.target = RenderTarget::snapshot(size);
            let expected = reference
                .render_frame(snapshot_request, &EmptyResourceResolver)
                .unwrap()
                .snapshot
                .unwrap()
                .pixels;
            assert!(expected.chunks_exact(4).any(|pixel| pixel[2] > 0));
            let target = ReadbackTarget::new(&device, size);
            deferred
                .render_frame_into_view_with_encoder(
                    request,
                    &EmptyResourceResolver,
                    &mut encoder,
                    WgpuRenderTargetView::new(&target.view, wgpu::TextureFormat::Rgba8Unorm),
                )
                .unwrap();
            target.copy_to_buffer(&mut encoder);
            outputs.push((partial, target, expected));
        }
        queue.submit([encoder.finish()]);
        for (partial, target, expected) in outputs {
            assert!(
                target.read(&device) == expected,
                "image {image_size:?}, partial={partial}"
            );
        }
    }
}

fn request(content: Content, frame: usize, size: PixelSize) -> RenderFrameRequest {
    let bounds = UiRect::new(0.0, 0.0, size.width as f32, size.height as f32);
    let rect = UiRect::new(4.0 + frame as f32 * 3.0, 6.0 + frame as f32, 24.0, 20.0);
    let color = [
        ColorRgba::new(255, 0, 0, 255),
        ColorRgba::new(0, 255, 0, 255),
        ColorRgba::new(0, 0, 255, 255),
    ][frame];
    let item = |index, rect, kind| PaintItem {
        node: UiNodeId::from_index(index),
        rect,
        clip_rect: bounds,
        z_index: 0.0,
        layer_order: LayerOrder::DEFAULT,
        opacity: 1.0,
        transform: PaintTransform::default(),
        shader: None,
        material: None,
        kind,
    };
    let mut updates = Vec::new();
    let mut items = match content {
        Content::Geometry => vec![
            item(
                0,
                rect,
                PaintKind::Rect {
                    fill: color,
                    stroke: None,
                    corner_radius: 0.0,
                },
            ),
            item(
                1,
                UiRect::new(32.0, 24.0, 16.0, 16.0),
                PaintKind::RichRect(
                    PaintRect::solid(UiRect::new(32.0, 24.0, 16.0, 16.0), color)
                        .corner_radii(CornerRadii::uniform(4.0 + frame as f32))
                        .effect(PaintEffect::shadow(
                            ColorRgba::new(180, 180, 180, 128),
                            UiPoint::new(frame as f32 + 1.0, 2.0),
                            2.0,
                            1.0,
                        )),
                ),
            ),
            item(
                2,
                bounds,
                PaintKind::Line {
                    from: UiPoint::new(2.0, bounds.height - 6.0),
                    to: UiPoint::new(bounds.width - 4.0, bounds.height - 2.0),
                    stroke: StrokeStyle::new(color, 2.0),
                },
            ),
        ],
        Content::SdfGradient | Content::CompositedSdfGradient => {
            let mut gradient = operad::LinearGradient::new(
                UiPoint::new(rect.x, rect.y),
                UiPoint::new(rect.right(), rect.bottom()),
                color,
                ColorRgba::new(255 - color.r, 255 - color.g, 255 - color.b, 180),
            );
            for stop in 0..(frame * 5 + 1) {
                gradient = gradient.stop(
                    (stop + 1) as f32 / (frame * 5 + 2) as f32,
                    ColorRgba::new((frame * 80) as u8, (stop * 23) as u8, 200, 255),
                );
            }
            vec![
                item(
                    0,
                    rect,
                    PaintKind::RichRect(
                        PaintRect::new(rect, operad::PaintBrush::LinearGradient(gradient))
                            .corner_radii(CornerRadii::new(12.0, 1.0, 6.0, 2.0))
                            .stroke(operad::AlignedStroke::outside(StrokeStyle::new(
                                ColorRgba::WHITE,
                                0.25 + frame as f32,
                            )))
                            .effect(PaintEffect::inset_shadow(
                                ColorRgba::BLACK,
                                UiPoint::new(frame as f32, 1.0),
                                6.0,
                                0.0,
                            )),
                    ),
                ),
                item(
                    1,
                    bounds,
                    PaintKind::Circle {
                        center: UiPoint::new(bounds.width - 12.0, bounds.height - 12.0),
                        radius: 7.25,
                        fill: color,
                        stroke: Some(StrokeStyle::new(ColorRgba::WHITE, 0.5)),
                    },
                ),
            ]
        }
        Content::Text | Content::CompositedText => vec![item(
            0,
            UiRect::new(rect.x, rect.y, bounds.width - 16.0, 36.0),
            PaintKind::Text(TextContent::new(
                format!("Pass {frame}"),
                TextStyle {
                    font_size: 12.0 + frame as f32 * 3.0,
                    line_height: 24.0,
                    color,
                    ..Default::default()
                },
            )),
        )],
        Content::Image | Content::CompositedImage | Content::NestedImage | Content::ShaderImage => {
            let handle = ImageHandle::app("deferred-image");
            updates.push(if frame < 2 {
                ResourceUpdate::rgba8_image(
                    handle,
                    PixelSize::new(4, 4),
                    [color.r, color.g, color.b, color.a].repeat(16),
                )
            } else {
                ResourceUpdate::partial(
                    ResourceDescriptor::new(handle, PixelSize::new(4, 4), ResourceFormat::Rgba8),
                    PixelRect::new(0, 0, 2, 1),
                    [color.r, color.g, color.b, color.a].repeat(2),
                )
            });
            vec![item(
                0,
                rect,
                PaintKind::Image {
                    key: "deferred-image".to_owned(),
                    tint: None,
                },
            )]
        }
        Content::Canvas | Content::CompositedCanvas | Content::InvalidCanvas => {
            let uniforms = [color.r, color.g, color.b, color.a]
                .into_iter()
                .flat_map(|channel| (f32::from(channel) / 255.0).to_ne_bytes())
                .collect::<Vec<_>>();
            vec![item(
                0,
                rect,
                PaintKind::Canvas(
                    CanvasContent::new("deferred-canvas").program(
                        CanvasRenderProgram::wgsl(
                            if matches!(content, Content::InvalidCanvas) && frame == 1 {
                                "invalid shader"
                            } else {
                                CANVAS_SHADER
                            },
                        )
                        .uniform_bytes(uniforms),
                    ),
                ),
            )]
        }
    };
    let depth = match content {
        Content::CompositedImage
        | Content::CompositedText
        | Content::CompositedCanvas
        | Content::CompositedSdfGradient => 1,
        Content::NestedImage => 2,
        _ => 0,
    };
    if matches!(content, Content::ShaderImage) {
        items[0].shader = Some(operad::ShaderEffect::tint(ColorRgba::WHITE, 0.25));
    }
    for _ in 0..depth {
        items = vec![item(
            0,
            bounds,
            PaintKind::CompositedLayer(PaintCompositorLayer::new(bounds, PaintList { items })),
        )];
    }
    let viewport = UiSize::new(bounds.width, bounds.height);
    RenderFrameRequest::new(
        RenderTarget::app_owned("deferred-test", viewport),
        viewport,
        PaintList { items },
    )
    .resource_updates(updates)
    .options(RenderOptions {
        clear_color: ColorRgba::BLACK,
        ..Default::default()
    })
}

const CANVAS_SHADER: &str = r#"
@group(0) @binding(0) var<uniform> color: vec4<f32>;
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4(positions[i], 0.0, 1.0);
}
@fragment fn fs_main() -> @location(0) vec4<f32> { return color; }
"#;

struct ReadbackTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    buffer: wgpu::Buffer,
    stride: u32,
    size: PixelSize,
}

impl ReadbackTarget {
    fn new(device: &wgpu::Device, size: PixelSize) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("deferred test target"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let stride = (size.width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("deferred test readback"),
            size: u64::from(stride) * u64::from(size.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Self {
            texture,
            view,
            buffer,
            stride,
            size,
        }
    }

    fn copy_to_buffer(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.stride),
                    rows_per_image: Some(self.size.height),
                },
            },
            wgpu::Extent3d {
                width: self.size.width,
                height: self.size.height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn read(&self, device: &wgpu::Device) -> Vec<u8> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = self.buffer.slice(..).get_mapped_range();
        let mut pixels = Vec::new();
        for row in mapped.chunks_exact(self.stride as usize) {
            pixels.extend_from_slice(&row[..self.size.width as usize * 4]);
        }
        drop(mapped);
        self.buffer.unmap();
        pixels
    }
}
