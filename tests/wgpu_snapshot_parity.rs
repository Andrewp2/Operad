#![cfg(feature = "wgpu")]

use operad::compositor::{
    CompositorClip, CompositorFilter, CompositorFilterKind, CompositorMask, MaskMode,
};
use operad::layout;
use operad::platform::{
    ImageHandle, LayerOrder, PixelColorSpace, PixelSize, ResourceHandle, ResourceId,
};
use operad::renderer::EmptyResourceResolver;
use operad::renderer::{
    RenderFrameRequest, RenderOptions, RenderTarget, RenderedImage, RendererAdapter,
    ResourceDescriptor, ResourceFormat, ResourceResolver, ResourceUpdate,
};
use operad::testing::{EventReplay, ScenarioHarness};
use operad::wgpu_renderer::WgpuRenderer;
use operad::{root_style, ApproxTextMeasurer, TextStyle};
use operad::{
    AlignedStroke, ColorRgba, CornerRadii, LinearGradient, PaintBrush, PaintCompositorLayer,
    PaintEffect, PaintItem, PaintKind, PaintList, PaintPath, PaintRect, PaintTransform,
    PathFillRule, ShaderEffect, StrokeLineCap, StrokeLineJoin, StrokeStyle, TextContent,
    UiDocument, UiNode, UiNodeId, UiNodeStyle, UiPoint, UiRect, UiSize, UiVisual,
};

fn scene_document() -> UiDocument {
    let mut document = UiDocument::new(root_style(160.0, 120.0));
    let root = document.root();
    document
        .node_mut(root)
        .set_visual(UiVisual::panel(ColorRgba::new(16, 20, 28, 255), None, 0.0));

    let panel_style = UiNodeStyle::from(layout::fixed(80.0, 40.0));
    document.add_child(
        root,
        UiNode::container("panel", panel_style).with_visual(UiVisual::panel(
            ColorRgba::new(64, 128, 188, 255),
            Some(operad::StrokeStyle::new(
                ColorRgba::new(255, 255, 255, 255),
                1.5,
            )),
            0.0,
        )),
    );

    document
}

fn render_snapshot_with_renderer(
    mut document: UiDocument,
    renderer: &mut impl RendererAdapter,
) -> RenderedImage {
    let mut harness = ScenarioHarness::new(UiSize::new(160.0, 120.0))
        .target(RenderTarget::snapshot(PixelSize::new(160, 120)));

    let mut measurer = ApproxTextMeasurer;
    let report = harness
        .run_frame_with_measurer_and_renderer(
            "snapshot-parity",
            &mut document,
            EventReplay::new(),
            &mut measurer,
            renderer,
            &EmptyResourceResolver,
        )
        .unwrap();
    report
        .render
        .snapshot
        .expect("snapshot should be present for snapshot target")
}

#[test]
fn wgpu_snapshot_renders_scene_document() {
    let wgpu_render_output = {
        let document = scene_document();
        let mut renderer = WgpuRenderer::default();
        render_snapshot_with_renderer(document, &mut renderer)
    };

    assert_eq!(wgpu_render_output.size, PixelSize::new(160, 120));
    assert_eq!(wgpu_render_output.format, ResourceFormat::Rgba8);
    assert_eq!(wgpu_render_output.color_space, PixelColorSpace::Srgb);
    assert_eq!(
        pixel_rgba(&wgpu_render_output.pixels, 160, 20, 20),
        [64, 128, 188, 255]
    );
    assert_eq!(
        pixel_rgba(&wgpu_render_output.pixels, 160, 120, 100),
        [16, 20, 28, 255]
    );
}

#[derive(Debug, Clone)]
struct SingleResourceResolver {
    descriptor: ResourceDescriptor,
}

impl ResourceResolver for SingleResourceResolver {
    fn resolve_resource(&self, id: &ResourceId) -> Option<ResourceDescriptor> {
        (self.descriptor.handle.id() == id).then(|| self.descriptor.clone())
    }
}

#[test]
fn wgpu_image_snapshot_uses_uploaded_texture_resource() {
    let handle = ResourceHandle::Image(ImageHandle::app("cover.texture"));
    let descriptor =
        ResourceDescriptor::new(handle, PixelSize::new(2, 1), ResourceFormat::Rgba8).version(1);
    let update = ResourceUpdate::full(
        descriptor.clone(),
        vec![
            255, 0, 0, 255, //
            0, 255, 0, 255,
        ],
    );
    let paint = PaintList {
        items: vec![PaintItem {
            node: UiNodeId::root(),
            rect: UiRect::new(0.0, 0.0, 2.0, 1.0),
            clip_rect: UiRect::new(0.0, 0.0, 2.0, 1.0),
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            material: None,
            kind: PaintKind::Image {
                key: "cover.texture".to_string(),
                tint: None,
            },
        }],
    };
    let request = RenderFrameRequest::new(
        RenderTarget::snapshot(PixelSize::new(2, 1)),
        UiSize::new(2.0, 1.0),
        paint,
    )
    .resource_update(update);

    let mut renderer = WgpuRenderer::default();
    let output = renderer
        .render_frame(
            request,
            &SingleResourceResolver {
                descriptor: descriptor.clone(),
            },
        )
        .expect("wgpu image render");
    let image = output.snapshot.expect("snapshot");

    assert_eq!(image.size, PixelSize::new(2, 1));
    assert_eq!(
        image.pixels,
        vec![
            255, 0, 0, 255, //
            0, 255, 0, 255,
        ]
    );
}

#[test]
fn wgpu_rounded_rect_uses_sdf_edges() {
    let paint = PaintList {
        items: vec![PaintItem {
            node: UiNodeId::root(),
            rect: UiRect::new(1.0, 1.0, 10.0, 10.0),
            clip_rect: UiRect::new(0.0, 0.0, 12.0, 12.0),
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            material: None,
            kind: PaintKind::Rect {
                fill: ColorRgba::new(32, 180, 220, 255),
                stroke: None,
                corner_radius: 4.0,
            },
        }],
    };
    let request = RenderFrameRequest::new(
        RenderTarget::snapshot(PixelSize::new(12, 12)),
        UiSize::new(12.0, 12.0),
        paint,
    )
    .options(RenderOptions {
        clear_color: ColorRgba::new(0, 0, 0, 255),
        ..RenderOptions::default()
    });

    let mut renderer = WgpuRenderer::default();
    let output = renderer
        .render_frame(request, &EmptyResourceResolver)
        .expect("wgpu rounded rect render");
    let image = output.snapshot.expect("snapshot");

    let corner = pixel_rgba(&image.pixels, 12, 1, 1);
    let center = pixel_rgba(&image.pixels, 12, 6, 6);

    assert_eq!(corner, [0, 0, 0, 255]);
    assert_eq!(center, [32, 180, 220, 255]);
}

#[test]
fn wgpu_text_snapshot_preserves_fractional_glyph_positioning() {
    let integer = text_snapshot_at(UiPoint::new(4.0, 4.0));
    let fractional = text_snapshot_at(UiPoint::new(4.5, 4.25));

    assert!(lit_pixel_count(&integer) > 24);
    assert!(lit_pixel_count(&fractional) > 24);
    assert_ne!(
        integer.pixels, fractional.pixels,
        "fractional text placement should affect grayscale glyph coverage"
    );
}

#[test]
fn wgpu_replacing_fonts_renders_the_same_glyphs_as_a_fresh_renderer() {
    let font = |bytes: &[u8]| {
        operad::fonts::FontLibrary::new()
            .with_memory_font("replacement", bytes)
            .with_sans_serif_family("ReplacementFixture")
    };
    let fonts = [
        font(include_bytes!("fixtures/fonts/replacement-lower.ttf")),
        font(include_bytes!("fixtures/fonts/replacement-upper.ttf")),
    ];

    for count in [1, 16] {
        let snapshot = |renderer: &mut WgpuRenderer| {
            let size = UiSize::new(160.0, 160.0);
            let request = RenderFrameRequest::new(
                RenderTarget::snapshot(PixelSize::new(160, 160)),
                size,
                PaintList {
                    items: (0..count)
                        .map(|index| PaintItem {
                            node: UiNodeId::from_index(index),
                            rect: UiRect::new(
                                (index % 4) as f32 * 40.0,
                                (index / 4) as f32 * 40.0,
                                40.0,
                                40.0,
                            ),
                            clip_rect: UiRect::new(0.0, 0.0, size.width, size.height),
                            z_index: 0.0,
                            layer_order: LayerOrder::DEFAULT,
                            opacity: 1.0,
                            transform: PaintTransform::default(),
                            shader: None,
                            material: None,
                            kind: PaintKind::Text(TextContent::new(
                                "A",
                                TextStyle {
                                    font_size: 32.0,
                                    line_height: 40.0,
                                    color: ColorRgba::WHITE,
                                    ..Default::default()
                                },
                            )),
                        })
                        .collect(),
                },
            )
            .options(RenderOptions {
                clear_color: ColorRgba::BLACK,
                ..RenderOptions::default()
            });
            renderer
                .render_frame(request, &EmptyResourceResolver)
                .expect("font replacement frame")
                .snapshot
                .expect("font replacement snapshot")
        };
        let expected = fonts.each_ref().map(|font| {
            let mut fresh = WgpuRenderer::default();
            fresh.set_fonts(font.clone());
            let image = snapshot(&mut fresh);
            assert!(lit_pixel_count(&image) > 0, "fixture glyph must be visible");
            image
        });
        assert!(
            expected[0].pixels != expected[1].pixels,
            "the replacement must change the rendered glyph"
        );

        let mut retained = WgpuRenderer::default();
        for font_index in [0, 1, 0] {
            retained.set_fonts(fonts[font_index].clone());
            for repeated_frame in 0..2 {
                let actual = snapshot(&mut retained);
                let differing_pixels = actual
                    .pixels
                    .chunks_exact(4)
                    .zip(expected[font_index].pixels.chunks_exact(4))
                    .filter(|(actual, expected)| actual != expected)
                    .count();
                assert!(
                    actual.pixels == expected[font_index].pixels,
                    "{count} text items, font {font_index}, repeated frame {repeated_frame}: \
                     replacing fonts must not retain glyphs from the previous library \
                     ({differing_pixels} pixels differ)"
                );
            }
        }
    }
}

#[test]
fn wgpu_changing_text_reclaims_obsolete_glyphs_without_corrupting_cached_text() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("GPU adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits {
            max_texture_dimension_2d: 256,
            ..Default::default()
        },
        ..Default::default()
    }))
    .expect("device with a small glyph atlas budget");
    eprintln!(
        "glyph atlas stress: {:?}, texture limit {}",
        adapter.get_info(),
        device.limits().max_texture_dimension_2d
    );
    let fonts = operad::fonts::FontLibrary::new()
        .with_memory_font(
            "atlas-fixture",
            include_bytes!("fixtures/fonts/replacement-lower.ttf").as_slice(),
        )
        .with_sans_serif_family("ReplacementFixture");
    let fresh = || {
        WgpuRenderer::with_device_queue_and_fonts(device.clone(), queue.clone(), fonts.clone())
            .expect("renderer with a small glyph atlas budget")
    };
    let mut retained = fresh();

    // Repeatedly change the working set, then revisit earlier sizes. The first
    // text batch stays cached while later batches create new rasterized glyphs.
    for frame in 0..256 {
        let font_size = 32.0 + (frame % 128) as f32 * 0.5;
        let request = glyph_atlas_request(&[font_size], frame % 3 == 0);
        let actual = retained
            .render_frame(request.clone(), &EmptyResourceResolver)
            .unwrap_or_else(|error| panic!("small text frame {frame} failed: {error}"))
            .snapshot
            .expect("text snapshot");
        if frame % 31 == 0 || frame == 255 {
            let expected = fresh()
                .render_frame(request, &EmptyResourceResolver)
                .expect("individual text working set fits the atlas")
                .snapshot
                .expect("fresh text snapshot");
            assert!(
                expected
                    .pixels
                    .chunks_exact(4)
                    .any(|pixel| pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200),
                "reference text must be visible independently of the separator"
            );
            assert!(
                actual.pixels == expected.pixels,
                "frame {frame}: atlas reclamation must preserve both cached and changing text"
            );
        }
    }

    // A working set that exceeds capacity may fail, but must leave the renderer
    // usable. Compare its result with a fresh renderer instead of requiring a
    // particular packing strategy or hard-coding which requests must fail.
    let crowded = glyph_atlas_request(&(64..128).map(|size| size as f32).collect::<Vec<_>>(), true);
    let expected = fresh().render_frame(crowded.clone(), &EmptyResourceResolver);
    let actual = retained.render_frame(crowded, &EmptyResourceResolver);
    eprintln!(
        "crowded text working set: fresh succeeds={}, retained succeeds={}",
        expected.is_ok(),
        actual.is_ok()
    );
    assert_eq!(actual.is_ok(), expected.is_ok(), "text capacity result");
    if let (Ok(actual), Ok(expected)) = (actual, expected) {
        assert!(actual.snapshot.unwrap().pixels == expected.snapshot.unwrap().pixels);
    }
    retained
        .warm_up()
        .expect("warm-up must recover after a rejected text working set");
    let request = glyph_atlas_request(&[48.0], false);
    let actual = retained
        .render_frame(request.clone(), &EmptyResourceResolver)
        .expect("renderer remains usable")
        .snapshot
        .unwrap();
    let expected = fresh()
        .render_frame(request, &EmptyResourceResolver)
        .expect("fresh renderer")
        .snapshot
        .unwrap();
    assert!(actual.pixels == expected.pixels);
}

fn glyph_atlas_request(font_sizes: &[f32], composited_anchor: bool) -> RenderFrameRequest {
    let size = PixelSize::new(256, 192);
    let bounds = UiRect::new(0.0, 0.0, 256.0, 192.0);
    let mut items = std::iter::once(18.0)
        .chain(font_sizes.iter().copied())
        .enumerate()
        .map(|(index, font_size)| PaintItem {
            node: UiNodeId::from_index(index),
            rect: UiRect::new(4.0, if index == 0 { 0.0 } else { 40.0 }, 248.0, 144.0),
            clip_rect: bounds,
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            material: None,
            kind: PaintKind::Text(TextContent::new(
                "A",
                TextStyle {
                    font_size,
                    line_height: font_size + 4.0,
                    color: ColorRgba::WHITE,
                    ..Default::default()
                },
            )),
        })
        .collect::<Vec<_>>();
    items.insert(
        1,
        PaintItem {
            node: UiNodeId::from_index(font_sizes.len() + 1),
            rect: UiRect::new(0.0, 28.0, 256.0, 1.0),
            clip_rect: bounds,
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            material: None,
            kind: PaintKind::Rect {
                fill: ColorRgba::new(32, 48, 64, 255),
                stroke: None,
                corner_radius: 0.0,
            },
        },
    );
    if composited_anchor {
        let anchor = items[0].clone();
        items[0].rect = bounds;
        items[0].kind = PaintKind::CompositedLayer(PaintCompositorLayer::new(
            bounds,
            PaintList {
                items: vec![anchor],
            },
        ));
    }
    RenderFrameRequest::new(
        RenderTarget::snapshot(size),
        UiSize::new(256.0, 192.0),
        PaintList { items },
    )
    .options(RenderOptions {
        clear_color: ColorRgba::BLACK,
        ..RenderOptions::default()
    })
}

#[test]
fn wgpu_paint_order_allows_geometry_to_cover_prior_text() {
    let paint = PaintList {
        items: vec![
            PaintItem {
                node: UiNodeId::root(),
                rect: UiRect::new(4.0, 4.0, 88.0, 28.0),
                clip_rect: UiRect::new(0.0, 0.0, 96.0, 36.0),
                z_index: 0.0,
                layer_order: LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: PaintTransform::default(),
                shader: None,
                material: None,
                kind: PaintKind::Text(TextContent::new(
                    "Covered",
                    TextStyle {
                        font_size: 20.0,
                        line_height: 24.0,
                        color: ColorRgba::new(255, 255, 255, 255),
                        ..Default::default()
                    },
                )),
            },
            PaintItem {
                node: UiNodeId::from_index(1),
                rect: UiRect::new(0.0, 0.0, 96.0, 36.0),
                clip_rect: UiRect::new(0.0, 0.0, 96.0, 36.0),
                z_index: 0.0,
                layer_order: LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: PaintTransform::default(),
                shader: None,
                material: None,
                kind: PaintKind::Rect {
                    fill: ColorRgba::new(0, 96, 64, 255),
                    stroke: None,
                    corner_radius: 0.0,
                },
            },
        ],
    };
    let request = RenderFrameRequest::new(
        RenderTarget::snapshot(PixelSize::new(96, 36)),
        UiSize::new(96.0, 36.0),
        paint,
    )
    .options(RenderOptions {
        clear_color: ColorRgba::new(0, 0, 0, 255),
        ..RenderOptions::default()
    });

    let mut renderer = WgpuRenderer::default();
    let output = renderer
        .render_frame(request, &EmptyResourceResolver)
        .expect("wgpu covered text render");
    let image = output.snapshot.expect("snapshot");

    assert!(image
        .pixels
        .chunks_exact(4)
        .all(|pixel| pixel == [0, 96, 64, 255]));
}

#[test]
fn wgpu_rich_rect_gradient_and_effects_render_on_gpu() {
    let rich_rect = PaintRect::new(
        UiRect::new(8.0, 8.0, 48.0, 24.0),
        PaintBrush::LinearGradient(
            LinearGradient::new(
                UiPoint::new(8.0, 8.0),
                UiPoint::new(56.0, 8.0),
                ColorRgba::new(240, 32, 48, 255),
                ColorRgba::new(24, 72, 230, 255),
            )
            .stop(0.5, ColorRgba::new(24, 210, 96, 255))
            .fallback(ColorRgba::new(240, 32, 48, 255)),
        ),
    )
    .stroke(AlignedStroke::outside(StrokeStyle::new(
        ColorRgba::new(255, 255, 255, 255),
        2.0,
    )))
    .effect(PaintEffect::shadow(
        ColorRgba::new(0, 0, 0, 96),
        UiPoint::new(4.0, 4.0),
        8.0,
        2.0,
    ));
    let request = RenderFrameRequest::new(
        RenderTarget::snapshot(PixelSize::new(72, 48)),
        UiSize::new(72.0, 48.0),
        PaintList {
            items: vec![PaintItem {
                node: UiNodeId::root(),
                rect: UiRect::new(8.0, 8.0, 48.0, 24.0),
                clip_rect: UiRect::new(0.0, 0.0, 72.0, 48.0),
                z_index: 0.0,
                layer_order: LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: PaintTransform::default(),
                shader: None,
                material: None,
                kind: PaintKind::RichRect(rich_rect),
            }],
        },
    )
    .options(RenderOptions {
        clear_color: ColorRgba::new(18, 20, 24, 255),
        ..RenderOptions::default()
    });

    let wgpu = WgpuRenderer::default()
        .render_frame(request, &EmptyResourceResolver)
        .expect("wgpu rich rect render")
        .snapshot
        .expect("wgpu snapshot");

    assert_eq!(wgpu.size, PixelSize::new(72, 48));
    let left = pixel_rgba(&wgpu.pixels, 72, 12, 18);
    let middle = pixel_rgba(&wgpu.pixels, 72, 32, 18);
    let right = pixel_rgba(&wgpu.pixels, 72, 52, 18);
    assert!(left[0] > left[1] && left[0] > left[2], "{left:?}");
    assert!(middle[1] > middle[0] && middle[1] > middle[2], "{middle:?}");
    assert!(right[2] > right[0] && right[2] > right[1], "{right:?}");
    assert_ne!(
        pixel_rgba(&wgpu.pixels, 72, 60, 36),
        [18, 20, 24, 255],
        "shadow/effect fallback should affect pixels outside the filled rect"
    );
}

#[test]
fn wgpu_rich_rect_zero_width_stroke_does_not_render_hairline() {
    let rect = UiRect::new(8.0, 8.0, 20.0, 20.0);
    let image = wgpu_snapshot_for_item(
        PixelSize::new(36, 36),
        PaintItem {
            node: UiNodeId::root(),
            rect,
            clip_rect: UiRect::new(0.0, 0.0, 36.0, 36.0),
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            material: None,
            kind: PaintKind::RichRect(PaintRect {
                rect,
                fill: PaintBrush::Solid(ColorRgba::TRANSPARENT),
                stroke: Some(AlignedStroke::inside(StrokeStyle::new(
                    ColorRgba::WHITE,
                    0.0,
                ))),
                corner_radii: CornerRadii::ZERO,
                effects: Vec::new(),
            }),
        },
    );

    assert_eq!(
        lit_pixel_count(&image),
        0,
        "zero-width rich-rect strokes should not be clamped into visible hairlines"
    );
}

#[test]
fn wgpu_rich_rect_preserves_individual_corner_radii() {
    let rect = UiRect::new(8.0, 8.0, 24.0, 24.0);
    let image = wgpu_snapshot_for_item(
        PixelSize::new(40, 40),
        PaintItem {
            node: UiNodeId::root(),
            rect,
            clip_rect: UiRect::new(0.0, 0.0, 40.0, 40.0),
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            material: None,
            kind: PaintKind::RichRect(
                PaintRect::solid(rect, ColorRgba::WHITE)
                    .corner_radii(CornerRadii::new(0.0, 10.0, 0.0, 10.0)),
            ),
        },
    );

    let square_top_left = pixel_rgba(&image.pixels, 40, 8, 8);
    let rounded_top_right = pixel_rgba(&image.pixels, 40, 31, 8);
    let square_bottom_right = pixel_rgba(&image.pixels, 40, 31, 31);
    let rounded_bottom_left = pixel_rgba(&image.pixels, 40, 8, 31);
    assert!(
        square_top_left[0] > 180,
        "top-left radius is zero, so the corner should be filled: {square_top_left:?}"
    );
    assert!(
        rounded_top_right[0] < 40,
        "top-right radius is rounded, so the corner should remain clear: {rounded_top_right:?}"
    );
    assert!(
        square_bottom_right[0] > 180,
        "bottom-right radius is zero, so the corner should be filled: {square_bottom_right:?}"
    );
    assert!(
        rounded_bottom_left[0] < 40,
        "bottom-left radius is rounded, so the corner should remain clear: {rounded_bottom_left:?}"
    );
}

#[test]
fn wgpu_shadered_paint_item_tint_changes_rendered_pixels() {
    let rect = UiRect::new(8.0, 8.0, 20.0, 20.0);
    let image = wgpu_snapshot_for_item(
        PixelSize::new(40, 40),
        PaintItem {
            node: UiNodeId::root(),
            rect,
            clip_rect: UiRect::new(0.0, 0.0, 40.0, 40.0),
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: Some(ShaderEffect::tint(ColorRgba::new(255, 0, 0, 255), 1.0)),
            material: None,
            kind: PaintKind::Rect {
                fill: ColorRgba::WHITE,
                stroke: None,
                corner_radius: 0.0,
            },
        },
    );

    let center = pixel_rgba(&image.pixels, 40, 18, 18);
    assert!(
        center[0] > 180 && center[1] < 50 && center[2] < 50,
        "tint shader should turn the white source rect red: {center:?}"
    );
}

#[test]
fn wgpu_shadered_paint_item_glow_can_render_outside_item_bounds() {
    let rect = UiRect::new(16.0, 16.0, 8.0, 8.0);
    let image = wgpu_snapshot_for_item(
        PixelSize::new(40, 40),
        PaintItem {
            node: UiNodeId::root(),
            rect,
            clip_rect: UiRect::new(0.0, 0.0, 40.0, 40.0),
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: Some(ShaderEffect::glow(
                ColorRgba::new(40, 120, 255, 255),
                1.0,
                4.0,
            )),
            material: None,
            kind: PaintKind::Rect {
                fill: ColorRgba::WHITE,
                stroke: None,
                corner_radius: 0.0,
            },
        },
    );

    let outside_left_edge = pixel_rgba(&image.pixels, 40, 13, 20);
    assert!(
        outside_left_edge[2] > outside_left_edge[0] + 40,
        "glow shader should add blue pixels outside the original rect: {outside_left_edge:?}"
    );
}

#[test]
fn wgpu_rich_rect_shadow_has_soft_falloff() {
    let rich_rect = PaintRect::new(
        UiRect::new(10.0, 10.0, 20.0, 12.0),
        PaintBrush::Solid(ColorRgba::new(40, 120, 220, 255)),
    )
    .effect(PaintEffect::shadow(
        ColorRgba::new(0, 0, 0, 160),
        UiPoint::new(0.0, 0.0),
        10.0,
        0.0,
    ));
    let request = RenderFrameRequest::new(
        RenderTarget::snapshot(PixelSize::new(52, 36)),
        UiSize::new(52.0, 36.0),
        PaintList {
            items: vec![PaintItem {
                node: UiNodeId::root(),
                rect: UiRect::new(10.0, 10.0, 20.0, 12.0),
                clip_rect: UiRect::new(0.0, 0.0, 52.0, 36.0),
                z_index: 0.0,
                layer_order: LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: PaintTransform::default(),
                shader: None,
                material: None,
                kind: PaintKind::RichRect(rich_rect),
            }],
        },
    )
    .options(RenderOptions {
        clear_color: ColorRgba::new(240, 240, 240, 255),
        ..RenderOptions::default()
    });

    let wgpu = WgpuRenderer::default()
        .render_frame(request, &EmptyResourceResolver)
        .expect("wgpu soft shadow render")
        .snapshot
        .expect("wgpu snapshot");

    let near = pixel_rgba(&wgpu.pixels, 52, 32, 16);
    // Sample inside the visible Gaussian falloff. At three sigma the tail can
    // round to the background in an 8-bit target.
    let far = pixel_rgba(&wgpu.pixels, 52, 36, 16);
    let outside = pixel_rgba(&wgpu.pixels, 52, 47, 16);
    assert!(
        near[0] < far[0] && far[0] < outside[0],
        "expected soft shadow falloff from near to far pixels, got near={near:?} far={far:?} outside={outside:?}"
    );
}

#[test]
fn wgpu_path_stroke_flattens_quadratic_curve() {
    let path = PaintPath::new()
        .move_to(UiPoint::new(8.0, 28.0))
        .quadratic_to(UiPoint::new(26.0, 4.0), UiPoint::new(44.0, 28.0))
        .stroke(StrokeStyle::new(ColorRgba::WHITE, 4.0));
    let request = RenderFrameRequest::new(
        RenderTarget::snapshot(PixelSize::new(56, 36)),
        UiSize::new(56.0, 36.0),
        PaintList {
            items: vec![PaintItem {
                node: UiNodeId::root(),
                rect: path.bounds(),
                clip_rect: UiRect::new(0.0, 0.0, 56.0, 36.0),
                z_index: 0.0,
                layer_order: LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: PaintTransform::default(),
                shader: None,
                material: None,
                kind: PaintKind::Path(path),
            }],
        },
    )
    .options(RenderOptions {
        clear_color: ColorRgba::new(0, 0, 0, 255),
        ..RenderOptions::default()
    });

    let wgpu = WgpuRenderer::default()
        .render_frame(request, &EmptyResourceResolver)
        .expect("wgpu flattened path render")
        .snapshot
        .expect("wgpu snapshot");

    let curve_midpoint = pixel_rgba(&wgpu.pixels, 56, 26, 16);
    assert!(
        curve_midpoint[0] > 16,
        "expected quadratic control point to lift the rendered stroke, got {curve_midpoint:?}"
    );
}

#[test]
fn wgpu_path_stroke_line_caps_are_tessellated() {
    let butt = stroked_path_snapshot(
        PaintPath::new()
            .move_to(UiPoint::new(16.0, 16.0))
            .line_to(UiPoint::new(32.0, 16.0))
            .stroke(StrokeStyle::new(ColorRgba::WHITE, 8.0))
            .line_cap(StrokeLineCap::Butt),
        PixelSize::new(48, 32),
    );
    let square = stroked_path_snapshot(
        PaintPath::new()
            .move_to(UiPoint::new(16.0, 16.0))
            .line_to(UiPoint::new(32.0, 16.0))
            .stroke(StrokeStyle::new(ColorRgba::WHITE, 8.0))
            .line_cap(StrokeLineCap::Square),
        PixelSize::new(48, 32),
    );

    assert_eq!(pixel_rgba(&butt.pixels, 48, 13, 16), [0, 0, 0, 255]);
    assert_eq!(pixel_rgba(&square.pixels, 48, 13, 16), [255, 255, 255, 255]);
}

#[test]
fn wgpu_path_stroke_line_joins_are_tessellated() {
    let base_path = || {
        PaintPath::new()
            .move_to(UiPoint::new(10.0, 30.0))
            .line_to(UiPoint::new(24.0, 8.0))
            .line_to(UiPoint::new(38.0, 30.0))
            .stroke(StrokeStyle::new(ColorRgba::WHITE, 8.0))
            .line_cap(StrokeLineCap::Butt)
    };
    let bevel = stroked_path_snapshot(
        base_path().line_join(StrokeLineJoin::Bevel),
        PixelSize::new(48, 36),
    );
    let miter = stroked_path_snapshot(
        base_path()
            .line_join(StrokeLineJoin::Miter)
            .miter_limit(8.0),
        PixelSize::new(48, 36),
    );

    assert_ne!(
        bevel.pixels, miter.pixels,
        "miter and bevel joins should not collapse to the same stroke mesh"
    );
    assert!(
        lit_pixel_count(&miter) > lit_pixel_count(&bevel),
        "miter join should add coverage beyond bevel"
    );
}

#[test]
fn wgpu_path_fill_even_odd_contours_cut_holes() {
    let path = PaintPath::new()
        .move_to(UiPoint::new(4.0, 4.0))
        .line_to(UiPoint::new(44.0, 4.0))
        .line_to(UiPoint::new(44.0, 44.0))
        .line_to(UiPoint::new(4.0, 44.0))
        .close()
        .move_to(UiPoint::new(14.0, 14.0))
        .line_to(UiPoint::new(34.0, 14.0))
        .line_to(UiPoint::new(34.0, 34.0))
        .line_to(UiPoint::new(14.0, 34.0))
        .close()
        .fill_rule(PathFillRule::EvenOdd)
        .fill(ColorRgba::new(220, 40, 60, 255));
    let image = filled_path_snapshot(path, PixelSize::new(48, 48));

    assert_eq!(pixel_rgba(&image.pixels, 48, 8, 8), [220, 40, 60, 255]);
    assert_eq!(pixel_rgba(&image.pixels, 48, 24, 24), [0, 0, 0, 255]);
}

#[test]
fn wgpu_path_fill_tessellates_curved_concave_shapes() {
    let path = PaintPath::new()
        .move_to(UiPoint::new(8.0, 8.0))
        .line_to(UiPoint::new(48.0, 8.0))
        .cubic_to(
            UiPoint::new(56.0, 8.0),
            UiPoint::new(56.0, 20.0),
            UiPoint::new(48.0, 20.0),
        )
        .line_to(UiPoint::new(24.0, 20.0))
        .line_to(UiPoint::new(24.0, 32.0))
        .line_to(UiPoint::new(48.0, 32.0))
        .cubic_to(
            UiPoint::new(56.0, 32.0),
            UiPoint::new(56.0, 44.0),
            UiPoint::new(48.0, 44.0),
        )
        .line_to(UiPoint::new(8.0, 44.0))
        .close()
        .fill_rule(PathFillRule::NonZero)
        .fill(ColorRgba::new(40, 210, 110, 255));
    let image = filled_path_snapshot(path, PixelSize::new(64, 52));

    assert_eq!(pixel_rgba(&image.pixels, 64, 12, 26), [40, 210, 110, 255]);
    assert_eq!(pixel_rgba(&image.pixels, 64, 36, 26), [0, 0, 0, 255]);
}

#[test]
fn wgpu_composited_layer_rounded_clip_masks_child_content() {
    let layer_bounds = UiRect::new(4.0, 4.0, 24.0, 24.0);
    let child = PaintItem {
        node: UiNodeId::from_index(1),
        rect: layer_bounds,
        clip_rect: UiRect::new(0.0, 0.0, 32.0, 32.0),
        z_index: 0.0,
        layer_order: LayerOrder::DEFAULT,
        opacity: 1.0,
        transform: PaintTransform::default(),
        shader: None,
        material: None,
        kind: PaintKind::Rect {
            fill: ColorRgba::new(220, 32, 48, 255),
            stroke: None,
            corner_radius: 0.0,
        },
    };
    let layer = PaintCompositorLayer::new(layer_bounds, PaintList { items: vec![child] }).clip(
        CompositorClip::rounded_rect(layer_bounds, CornerRadii::uniform(10.0)),
    );
    let request =
        composited_layer_request(layer, PixelSize::new(32, 32), ColorRgba::new(2, 4, 8, 255));

    let wgpu = WgpuRenderer::default()
        .render_frame(request, &EmptyResourceResolver)
        .expect("wgpu composited rounded clip render")
        .snapshot
        .expect("wgpu snapshot");

    assert_eq!(pixel_rgba(&wgpu.pixels, 32, 4, 4), [2, 4, 8, 255]);
    assert_eq!(pixel_rgba(&wgpu.pixels, 32, 16, 16), [220, 32, 48, 255]);
}

#[test]
fn wgpu_composited_layer_mask_and_filter_apply_on_gpu() {
    let layer_bounds = UiRect::new(4.0, 4.0, 24.0, 16.0);
    let child_color = ColorRgba::new(100, 120, 200, 255);
    let child = PaintItem {
        node: UiNodeId::from_index(1),
        rect: layer_bounds,
        clip_rect: UiRect::new(0.0, 0.0, 32.0, 24.0),
        z_index: 0.0,
        layer_order: LayerOrder::DEFAULT,
        opacity: 1.0,
        transform: PaintTransform::default(),
        shader: None,
        material: None,
        kind: PaintKind::Rect {
            fill: child_color,
            stroke: None,
            corner_radius: 0.0,
        },
    };
    let layer = PaintCompositorLayer::new(layer_bounds, PaintList { items: vec![child] })
        .mask(CompositorMask::new(
            UiRect::new(10.0, 4.0, 12.0, 16.0),
            MaskMode::Alpha,
        ))
        .filter(CompositorFilter::new(CompositorFilterKind::Brightness, 0.5));
    let request =
        composited_layer_request(layer, PixelSize::new(32, 24), ColorRgba::new(3, 4, 5, 255));

    let wgpu = WgpuRenderer::default()
        .render_frame(request, &EmptyResourceResolver)
        .expect("wgpu composited mask/filter render")
        .snapshot
        .expect("wgpu snapshot");

    assert_eq!(pixel_rgba(&wgpu.pixels, 32, 6, 10), [3, 4, 5, 255]);
    let masked_filtered = pixel_rgba(&wgpu.pixels, 32, 14, 10);
    assert!(
        masked_filtered[0] > 3
            && masked_filtered[1] > 4
            && masked_filtered[2] > 5
            && masked_filtered[0] < child_color.r
            && masked_filtered[1] < child_color.g
            && masked_filtered[2] < child_color.b,
        "expected brightness filter to darken masked child, got {masked_filtered:?}"
    );
}

#[test]
fn wgpu_composited_layer_blur_runs_on_gpu_texture() {
    let layer_bounds = UiRect::new(0.0, 0.0, 32.0, 16.0);
    let child = PaintItem {
        node: UiNodeId::from_index(1),
        rect: UiRect::new(8.0, 4.0, 4.0, 8.0),
        clip_rect: layer_bounds,
        z_index: 0.0,
        layer_order: LayerOrder::DEFAULT,
        opacity: 1.0,
        transform: PaintTransform::default(),
        shader: None,
        material: None,
        kind: PaintKind::Rect {
            fill: ColorRgba::WHITE,
            stroke: None,
            corner_radius: 0.0,
        },
    };
    let layer = PaintCompositorLayer::new(layer_bounds, PaintList { items: vec![child] })
        .filter(CompositorFilter::new(CompositorFilterKind::Blur, 4.0));
    let request =
        composited_layer_request(layer, PixelSize::new(32, 16), ColorRgba::new(0, 0, 0, 255));

    let wgpu = WgpuRenderer::default()
        .render_frame(request, &EmptyResourceResolver)
        .expect("wgpu composited blur render")
        .snapshot
        .expect("wgpu snapshot");

    let blurred_edge = pixel_rgba(&wgpu.pixels, 32, 5, 8);
    assert!(
        blurred_edge[0] > 0 && blurred_edge[0] < 255,
        "expected blurred offscreen content to affect a neighboring pixel, got {blurred_edge:?}"
    );
}

#[test]
fn wgpu_composited_layer_renders_glyphon_text_child() {
    let layer_bounds = UiRect::new(0.0, 0.0, 96.0, 36.0);
    let child = PaintItem {
        node: UiNodeId::from_index(1),
        rect: UiRect::new(5.25, 5.5, 86.0, 26.0),
        clip_rect: layer_bounds,
        z_index: 0.0,
        layer_order: LayerOrder::DEFAULT,
        opacity: 1.0,
        transform: PaintTransform::default(),
        shader: None,
        material: None,
        kind: PaintKind::Text(TextContent::new(
            "Layer",
            TextStyle {
                font_size: 20.0,
                line_height: 24.0,
                color: ColorRgba::new(255, 255, 255, 255),
                ..Default::default()
            },
        )),
    };
    let layer = PaintCompositorLayer::new(layer_bounds, PaintList { items: vec![child] })
        .clip(CompositorClip::rect(layer_bounds));
    let request =
        composited_layer_request(layer, PixelSize::new(96, 36), ColorRgba::new(0, 0, 0, 255));

    let wgpu = WgpuRenderer::default()
        .render_frame(request, &EmptyResourceResolver)
        .expect("wgpu composited text render")
        .snapshot
        .expect("wgpu snapshot");

    let lit_pixels = wgpu
        .pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 16 || pixel[1] > 16 || pixel[2] > 16)
        .count();
    assert!(
        lit_pixels > 24,
        "expected visible glyph pixels in composited layer, got {lit_pixels}"
    );
}

fn composited_layer_request(
    layer: PaintCompositorLayer,
    size: PixelSize,
    clear_color: ColorRgba,
) -> RenderFrameRequest {
    RenderFrameRequest::new(
        RenderTarget::snapshot(size),
        UiSize::new(size.width as f32, size.height as f32),
        PaintList {
            items: vec![PaintItem {
                node: UiNodeId::root(),
                rect: layer.bounds,
                clip_rect: UiRect::new(0.0, 0.0, size.width as f32, size.height as f32),
                z_index: 0.0,
                layer_order: LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: PaintTransform::default(),
                shader: None,
                material: None,
                kind: PaintKind::CompositedLayer(layer),
            }],
        },
    )
    .options(RenderOptions {
        clear_color,
        ..RenderOptions::default()
    })
}

fn text_snapshot_at(position: UiPoint) -> RenderedImage {
    wgpu_snapshot_for_item(
        PixelSize::new(96, 36),
        PaintItem {
            node: UiNodeId::root(),
            rect: UiRect::new(position.x, position.y, 88.0, 28.0),
            clip_rect: UiRect::new(0.0, 0.0, 96.0, 36.0),
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            material: None,
            kind: PaintKind::Text(TextContent::new(
                "Glyphon",
                TextStyle {
                    font_size: 20.0,
                    line_height: 24.0,
                    color: ColorRgba::WHITE,
                    ..Default::default()
                },
            )),
        },
    )
}

fn stroked_path_snapshot(path: PaintPath, size: PixelSize) -> RenderedImage {
    path_snapshot(path, size)
}

fn filled_path_snapshot(path: PaintPath, size: PixelSize) -> RenderedImage {
    path_snapshot(path, size)
}

fn path_snapshot(path: PaintPath, size: PixelSize) -> RenderedImage {
    let bounds = path.bounds();
    wgpu_snapshot_for_item(
        size,
        PaintItem {
            node: UiNodeId::root(),
            rect: bounds,
            clip_rect: UiRect::new(0.0, 0.0, size.width as f32, size.height as f32),
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            material: None,
            kind: PaintKind::Path(path),
        },
    )
}

fn wgpu_snapshot_for_item(size: PixelSize, item: PaintItem) -> RenderedImage {
    let request = RenderFrameRequest::new(
        RenderTarget::snapshot(size),
        UiSize::new(size.width as f32, size.height as f32),
        PaintList { items: vec![item] },
    )
    .options(RenderOptions {
        clear_color: ColorRgba::new(0, 0, 0, 255),
        ..RenderOptions::default()
    });

    WgpuRenderer::default()
        .render_frame(request, &EmptyResourceResolver)
        .expect("wgpu paint item render")
        .snapshot
        .expect("wgpu snapshot")
}

fn lit_pixel_count(image: &RenderedImage) -> usize {
    image
        .pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 16 || pixel[1] > 16 || pixel[2] > 16)
        .count()
}

fn pixel_rgba(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let index = (y * width + x) * 4;
    [
        pixels[index],
        pixels[index + 1],
        pixels[index + 2],
        pixels[index + 3],
    ]
}
