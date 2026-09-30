use super::*;
use crate::{AlignedStroke, PaintEffect, PaintRect};

#[test]
fn common_shapes_have_constant_geometry_and_batch_in_paint_order() {
    let clip = UiRect::new(-100.0, -100.0, 20000.0, 20000.0);
    for scale in [0.25, 1.0, 2.0, 8.0] {
        for radius in [0.0, 0.25, 8.0, 120.0, 500.0] {
            let mut geometry = RenderGeometry::default();
            let rect = PaintRect::new(
                UiRect::new(10.0, 10.0, 1000.0, 800.0),
                crate::PaintBrush::LinearGradient(crate::LinearGradient::new(
                    UiPoint::new(0.0, 0.0),
                    UiPoint::new(1000.0, 800.0),
                    ColorRgba::WHITE,
                    ColorRgba::BLACK,
                )),
            )
            .corner_radii(CornerRadii::uniform(radius))
            .stroke(AlignedStroke::outside(StrokeStyle::new(
                ColorRgba::WHITE,
                0.25,
            )));
            push_rich_rect(
                &mut geometry,
                &rect,
                clip,
                1.0,
                PaintTransform {
                    scale,
                    ..Default::default()
                },
            )
            .unwrap();
            geometry.push_shape(
                clip,
                SdfInstance::circle(
                    UiPoint::new(80.0, 80.0),
                    radius.max(0.25) * scale,
                    ColorRgba::WHITE,
                    Some(StrokeStyle::new(ColorRgba::BLACK, 0.25)),
                    1.0,
                )
                .unwrap(),
            );
            push_line(
                &mut geometry,
                UiPoint::new(1.0, 1.0),
                UiPoint::new(20.0, 20.0),
                clip,
                StrokeStyle::new(ColorRgba::WHITE, 0.25),
                1.0,
            );
            assert!(
                geometry.vertices.is_empty(),
                "common shapes generated CPU triangles"
            );
            assert_eq!(geometry.shapes.len(), 3);
            assert_eq!(geometry.batches.len(), 3);
            assert!(geometry.batches.iter().all(|b| b.count == 1));
            // Compatible consecutive segments still share a single draw.
            let mut compatible = RenderGeometry::default();
            for to in [UiPoint::new(20.0, 20.0), UiPoint::new(30.0, 20.0)] {
                push_line(
                    &mut compatible,
                    UiPoint::new(1.0, 1.0),
                    to,
                    clip,
                    StrokeStyle::new(ColorRgba::WHITE, 0.25),
                    1.0,
                );
            }
            assert_eq!(compatible.batches.len(), 1);
            assert_eq!(compatible.batches[0].count, 2);
            // Different clips and intervening triangles must remain ordering barriers.
            push_line(
                &mut geometry,
                UiPoint::new(1.0, 1.0),
                UiPoint::new(20.0, 20.0),
                UiRect::new(0.0, 0.0, 10.0, 10.0),
                StrokeStyle::new(ColorRgba::WHITE, 1.0),
                1.0,
            );
            push_polygon(
                &mut geometry,
                &[
                    UiPoint::new(0.0, 0.0),
                    UiPoint::new(5.0, 0.0),
                    UiPoint::new(0.0, 5.0),
                ],
                clip,
                ColorRgba::WHITE,
                1.0,
            );
            push_line(
                &mut geometry,
                UiPoint::new(0.0, 0.0),
                UiPoint::new(10.0, 10.0),
                clip,
                StrokeStyle::new(ColorRgba::WHITE, 1.0),
                1.0,
            );
            assert_eq!(
                geometry.batches.iter().map(|b| b.kind).collect::<Vec<_>>(),
                [
                    GeometryBatchKind::Sdf(SdfPipelineKind::GradientBorder),
                    GeometryBatchKind::Sdf(SdfPipelineKind::Border),
                    GeometryBatchKind::Sdf(SdfPipelineKind::Segment),
                    GeometryBatchKind::Sdf(SdfPipelineKind::Segment),
                    GeometryBatchKind::Triangle,
                    GeometryBatchKind::Sdf(SdfPipelineKind::Segment)
                ]
            );
        }
    }
}

#[test]
fn effects_and_gradient_buffers_preserve_order_and_device_limits() {
    let clip = UiRect::new(0.0, 0.0, 128.0, 128.0);
    let rect = PaintRect::new(UiRect::new(20.0, 20.0, 60.0, 60.0), ColorRgba::WHITE)
        .stroke(AlignedStroke::inside(StrokeStyle::new(
            ColorRgba::BLACK,
            2.0,
        )))
        .effect(PaintEffect::shadow(
            ColorRgba::BLACK,
            UiPoint::new(0.0, 0.0),
            12.0,
            1.0,
        ))
        .effect(PaintEffect::inset_shadow(
            ColorRgba::BLACK,
            UiPoint::new(0.0, 0.0),
            12.0,
            1.0,
        ));
    let mut geometry = RenderGeometry::default();
    push_rich_rect(&mut geometry, &rect, clip, 1.0, PaintTransform::default()).unwrap();
    assert_eq!(
        geometry
            .shapes
            .iter()
            .map(|s| s.params[0] as u8)
            .collect::<Vec<_>>(),
        [3, 0, 4, 0]
    );
    assert_eq!(geometry.shapes[1].border_color[3], 0.0);
    assert_eq!(geometry.shapes[3].color[3], 0.0);
    assert!(geometry.vertices.is_empty());
    assert_eq!(sdf::buffer_capacity(544, 640).unwrap(), 640);
    assert!(sdf::buffer_capacity(641, 640).is_err());
    assert!(sdf::buffer_capacity(u64::MAX, u64::MAX).is_err());
}

#[test]
fn srgb_fill_and_translucent_border_blend_in_linear_light() {
    let mut renderer = WgpuRenderer::default();
    let context = renderer.ensure_context().unwrap();
    let size = PixelSize::new(4, 4);
    let format = TextureFormat::Rgba8UnormSrgb;
    let texture = context
        .create_texture_2d(
            "sdf-srgb-test",
            size,
            format,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        )
        .unwrap();
    let view = texture.create_view(&Default::default());
    let mut geometry = RenderGeometry::default();
    geometry.push_shape(
        UiRect::new(0.0, 0.0, 4.0, 4.0),
        SdfInstance::rectangle(
            UiRect::new(1.0, 1.0, 2.0, 2.0),
            color_as_vertex(ColorRgba::new(0, 0, 255, 128), 1.0),
            CornerRadii::ZERO,
            Some((
                StrokeStyle::new(ColorRgba::new(255, 0, 0, 128), 0.25),
                crate::StrokeAlignment::Inside,
            )),
            1.0,
        )
        .unwrap(),
    );
    let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 1024,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = context.device.create_command_encoder(&Default::default());
    record_render_pass(
        context,
        &mut encoder,
        &view,
        format,
        size,
        &geometry,
        WgpuRenderLoadOp::Clear(ColorRgba::BLACK),
        false,
        false,
        true,
    )
    .unwrap();
    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
    );
    context.queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    context
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    rx.recv().unwrap().unwrap();
    let bytes = slice.get_mapped_range();
    let actual = &bytes[256 + 4..256 + 8];
    let encode = |v: f32| ((1.055 * v.powf(1.0 / 2.4) - 0.055) * 255.0).round() as u8;
    let alpha = 128.0 / 255.0;
    let expected = [
        encode(alpha * 0.25),
        0,
        encode(alpha * (1.0 - alpha * 0.25)),
        255,
    ];
    assert!(
        actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
        "sRGB blended {actual:?}, expected {expected:?}"
    );
}
