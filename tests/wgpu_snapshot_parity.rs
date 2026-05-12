#![cfg(feature = "wgpu")]

use operad::layout;
use operad::platform::{ImageHandle, LayerOrder, PixelSize, ResourceHandle, ResourceId};
use operad::{
    root_style, ApproxTextMeasurer, EmptyResourceResolver, EventReplay, RenderTarget,
    ScenarioHarness, TextStyle, WgpuRenderer,
};
use operad::{
    ColorRgba, CpuSnapshotRenderer, PaintItem, PaintKind, PaintList, PaintTransform,
    RenderFrameRequest, RenderOptions, RenderedImage, RendererAdapter, ResourceDescriptor,
    ResourceFormat, ResourceResolver, ResourceUpdate, TextContent, UiDocument, UiNode, UiNodeId,
    UiNodeStyle, UiRect, UiSize, UiVisual,
};

fn scene_document() -> UiDocument {
    let mut document = UiDocument::new(root_style(160.0, 120.0));
    let root = document.root;
    document.node_mut(root).visual = UiVisual::panel(ColorRgba::new(16, 20, 28, 255), None, 0.0);

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
    renderer: &mut impl operad::RendererAdapter,
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
fn wgpu_snapshot_matches_cpu_snapshot() {
    let cpu_render_output = {
        let document = scene_document();
        let mut renderer = CpuSnapshotRenderer::default();
        render_snapshot_with_renderer(document, &mut renderer)
    };

    let wgpu_render_output = {
        let document = scene_document();
        let mut renderer = WgpuRenderer::default();
        render_snapshot_with_renderer(document, &mut renderer)
    };

    assert_eq!(cpu_render_output.size, wgpu_render_output.size);
    assert_eq!(cpu_render_output.format, wgpu_render_output.format);
    assert_eq!(cpu_render_output.pixels, wgpu_render_output.pixels);
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
            node: UiNodeId(0),
            rect: UiRect::new(0.0, 0.0, 2.0, 1.0),
            clip_rect: UiRect::new(0.0, 0.0, 2.0, 1.0),
            z_index: 0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
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
            node: UiNodeId(0),
            rect: UiRect::new(1.0, 1.0, 10.0, 10.0),
            clip_rect: UiRect::new(0.0, 0.0, 12.0, 12.0),
            z_index: 0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
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
fn wgpu_text_snapshot_uses_glyphon_rendering() {
    let paint = PaintList {
        items: vec![PaintItem {
            node: UiNodeId(0),
            rect: UiRect::new(4.0, 4.0, 88.0, 28.0),
            clip_rect: UiRect::new(0.0, 0.0, 96.0, 36.0),
            z_index: 0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: PaintTransform::default(),
            shader: None,
            kind: PaintKind::Text(TextContent::new(
                "Glyphon",
                TextStyle {
                    font_size: 20.0,
                    line_height: 24.0,
                    color: ColorRgba::new(255, 255, 255, 255),
                    ..Default::default()
                },
            )),
        }],
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
        .expect("wgpu text render");
    let image = output.snapshot.expect("snapshot");

    let lit_pixels = image
        .pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 16 || pixel[1] > 16 || pixel[2] > 16)
        .count();
    assert!(
        lit_pixels > 24,
        "expected visible glyph pixels, got {lit_pixels}"
    );
}

#[test]
fn wgpu_paint_order_allows_geometry_to_cover_prior_text() {
    let paint = PaintList {
        items: vec![
            PaintItem {
                node: UiNodeId(0),
                rect: UiRect::new(4.0, 4.0, 88.0, 28.0),
                clip_rect: UiRect::new(0.0, 0.0, 96.0, 36.0),
                z_index: 0,
                layer_order: LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: PaintTransform::default(),
                shader: None,
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
                node: UiNodeId(1),
                rect: UiRect::new(0.0, 0.0, 96.0, 36.0),
                clip_rect: UiRect::new(0.0, 0.0, 96.0, 36.0),
                z_index: 0,
                layer_order: LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: PaintTransform::default(),
                shader: None,
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

fn pixel_rgba(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let index = (y * width + x) * 4;
    [
        pixels[index],
        pixels[index + 1],
        pixels[index + 2],
        pixels[index + 3],
    ]
}
