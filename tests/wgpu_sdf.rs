#![cfg(feature = "wgpu")]

use operad::platform::{LayerOrder, PixelSize};
use operad::renderer::{
    EmptyResourceResolver, RenderFrameRequest, RenderOptions, RenderTarget, RenderedImage,
    RendererAdapter,
};
use operad::wgpu_renderer::WgpuRenderer;
use operad::{
    AlignedStroke, ColorRgba, CornerRadii, GradientStop, LinearGradient, PaintBrush, PaintEffect,
    PaintItem, PaintKind, PaintList, PaintRect, PaintTransform, StrokeStyle, UiNodeId, UiPoint,
    UiRect, UiSize,
};

fn item(rect: UiRect, kind: PaintKind) -> PaintItem {
    PaintItem {
        node: UiNodeId::root(),
        rect,
        kind,
        clip_rect: UiRect::new(0.0, 0.0, 160.0, 120.0),
        z_index: 0.0,
        layer_order: LayerOrder::DEFAULT,
        opacity: 1.0,
        transform: PaintTransform::default(),
        shader: None,
        material: None,
    }
}
fn render(renderer: &mut WgpuRenderer, items: Vec<PaintItem>, scale: f32) -> RenderedImage {
    renderer
        .render_frame(
            RenderFrameRequest::new(
                RenderTarget::snapshot(PixelSize::new(
                    (160.0 * scale) as u32,
                    (120.0 * scale) as u32,
                )),
                UiSize::new(160.0, 120.0),
                PaintList { items },
            )
            .options(RenderOptions {
                scale_factor: scale,
                clear_color: ColorRgba::BLACK,
                ..Default::default()
            }),
            &EmptyResourceResolver,
        )
        .expect("SDF snapshot")
        .snapshot
        .unwrap()
}
fn pixel(image: &RenderedImage, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * image.size.width + x) * 4) as usize;
    image.pixels[i..i + 4].try_into().unwrap()
}
fn near(actual: [u8; 4], expected: [u8; 4], tolerance: u8, context: impl std::fmt::Debug) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(b) <= tolerance),
        "{context:?}: got {actual:?}, expected {expected:?}"
    );
}
fn rich(rect: PaintRect) -> PaintItem {
    item(rect.rect, PaintKind::RichRect(rect))
}

#[test]
fn rounded_multistop_gradients_follow_transforms_and_clipping() {
    let mut renderer = WgpuRenderer::default();
    let rect = UiRect::new(10.0, 10.0, 80.0, 60.0);
    let gradient = LinearGradient::new(
        UiPoint::new(10.0, 10.0),
        UiPoint::new(90.0, 70.0),
        ColorRgba::new(255, 0, 0, 255),
        ColorRgba::new(0, 0, 255, 255),
    )
    .stop(0.4, ColorRgba::new(0, 255, 0, 255))
    .fallback(ColorRgba::TRANSPARENT);
    for scale in [1.0, 2.0] {
        let mut shape = rich(
            PaintRect::new(rect, operad::PaintBrush::LinearGradient(gradient.clone()))
                .corner_radii(CornerRadii::new(35.0, 3.0, 20.0, 0.0)),
        );
        shape.transform = PaintTransform {
            translation: UiPoint::new(5.0, 7.0),
            scale: 1.25,
        };
        shape.clip_rect = UiRect::new(22.0, 0.0, 138.0, 120.0);
        let image = render(&mut renderer, vec![shape], scale);
        for (x, y) in [(36, 36), (65, 44), (95, 66), (110, 80)] {
            // Probe matching logical positions at either display scale.
            let (px, py) = if scale == 1.0 { (x, y) } else { (x * 2, y * 2) };
            let lx = (px as f32 + 0.5) / scale;
            let ly = (py as f32 + 0.5) / scale;
            let p = UiPoint::new((lx - 5.0) / 1.25, (ly - 7.0) / 1.25);
            let t = (((p.x - 10.0) * 80.0 + (p.y - 10.0) * 60.0) / 10000.0).clamp(0.0, 1.0);
            let expected = if t <= 0.4 {
                [
                    (255.0 * (1.0 - t / 0.4)).round() as u8,
                    (255.0 * t / 0.4).round() as u8,
                    0,
                    255,
                ]
            } else {
                [
                    0,
                    (255.0 * (1.0 - (t - 0.4) / 0.6)).round() as u8,
                    (255.0 * (t - 0.4) / 0.6).round() as u8,
                    255,
                ]
            };
            near(pixel(&image, px, py), expected, 2, (scale, px, py));
        }
        near(
            pixel(&image, (18.0 * scale) as u32, (45.0 * scale) as u32),
            [0, 0, 0, 255],
            0,
            "clip",
        );
        near(
            pixel(&image, (24.0 * scale) as u32, (21.0 * scale) as u32),
            [0, 0, 0, 255],
            0,
            "rounded corner",
        );
    }
}

#[test]
fn gradient_hard_stops_degenerate_lines_and_empty_fallbacks_render() {
    let mut renderer = WgpuRenderer::default();
    let rect = UiRect::new(10.0, 10.0, 60.0, 40.0);
    let mut gradient = LinearGradient::new(
        UiPoint::new(10.5, 0.0),
        UiPoint::new(70.5, 0.0),
        ColorRgba::new(255, 0, 0, 255),
        ColorRgba::new(0, 0, 255, 255),
    );
    // Direct paint data is unsorted; equal stops retain author order.
    gradient.stops = vec![
        GradientStop::new(1.0, ColorRgba::new(0, 0, 255, 255)),
        GradientStop::new(0.5, ColorRgba::new(255, 0, 0, 255)),
        GradientStop::new(0.5, ColorRgba::new(0, 0, 255, 255)),
        GradientStop::new(0.0, ColorRgba::new(255, 0, 0, 255)),
    ];
    let image = render(
        &mut renderer,
        vec![rich(PaintRect::new(
            rect,
            operad::PaintBrush::LinearGradient(gradient.clone()),
        ))],
        1.0,
    );
    near(
        pixel(&image, 39, 30),
        [255, 0, 0, 255],
        0,
        "before discontinuity",
    );
    near(
        pixel(&image, 40, 30),
        [255, 0, 0, 255],
        0,
        "first stop at discontinuity",
    );
    near(
        pixel(&image, 41, 30),
        [0, 0, 255, 255],
        0,
        "after discontinuity",
    );
    // Two-stop programs fetch colors per vertex; preserve hard stops and the
    // zero-length contract without relying on the storage-search program.
    let mut two = gradient.clone();
    two.stops = vec![
        GradientStop::new(0.5, ColorRgba::new(255, 0, 0, 255)),
        GradientStop::new(0.5, ColorRgba::new(0, 0, 255, 255)),
    ];
    let image = render(
        &mut renderer,
        vec![rich(PaintRect::new(
            rect,
            operad::PaintBrush::LinearGradient(two.clone()),
        ))],
        1.0,
    );
    for (x, expected) in [
        (39, [255, 0, 0, 255]),
        (40, [255, 0, 0, 255]),
        (41, [0, 0, 255, 255]),
    ] {
        near(pixel(&image, x, 30), expected, 0, "two-stop discontinuity");
    }
    two.end = two.start;
    let image = render(
        &mut renderer,
        vec![rich(PaintRect::new(
            rect,
            operad::PaintBrush::LinearGradient(two),
        ))],
        1.0,
    );
    near(
        pixel(&image, 30, 30),
        [0, 0, 255, 255],
        0,
        "two-stop zero-length gradient",
    );
    gradient.end = gradient.start;
    let image = render(
        &mut renderer,
        vec![rich(PaintRect::new(
            rect,
            operad::PaintBrush::LinearGradient(gradient.clone()),
        ))],
        1.0,
    );
    near(
        pixel(&image, 30, 30),
        [0, 0, 255, 255],
        0,
        "zero-length gradient uses last stop",
    );
    gradient.stops.clear();
    gradient.fallback = ColorRgba::new(20, 100, 200, 128);
    let image = render(
        &mut renderer,
        vec![rich(PaintRect::new(
            rect,
            operad::PaintBrush::LinearGradient(gradient),
        ))],
        1.0,
    );
    near(
        pixel(&image, 30, 30),
        [10, 50, 100, 255],
        1,
        "empty gradient fallback alpha",
    );
}

#[test]
fn asymmetric_corners_and_aligned_subpixel_borders_match_coverage() {
    let mut renderer = WgpuRenderer::default();
    let rect = UiRect::new(10.0, 10.0, 60.0, 60.0);
    let image = render(
        &mut renderer,
        vec![rich(
            PaintRect::new(rect, ColorRgba::WHITE)
                .corner_radii(CornerRadii::new(50.0, 0.0, 0.0, 0.0)),
        )],
        1.0,
    );
    for (x, y) in [
        (45, 11),
        (40, 14),
        (38, 16),
        (10, 45),
        (55, 15),
        (11, 11),
        (60, 60),
    ] {
        let p = UiPoint::new(x as f32 + 0.5, y as f32 + 0.5);
        let alpha = if p.x < 60.0 && p.y < 60.0 {
            (0.5 - ((p.x - 60.0).hypot(p.y - 60.0) - 50.0)).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let value = (alpha * 255.0).round() as u8;
        near(
            pixel(&image, x, y),
            [value, value, value, 255],
            2,
            (x, y, "asymmetric corner"),
        );
    }
    let image = render(
        &mut renderer,
        vec![rich(
            PaintRect::new(rect, ColorRgba::WHITE)
                .corner_radii(CornerRadii::new(50.0, 0.0, 50.0, 0.0)),
        )],
        1.0,
    );
    // Diagonally opposite corner regions overlap even when adjacent sums fit.
    // Their excluded disks must both constrain the visible shape.
    for y in 10..70 {
        for x in 10..70 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let mut d = (10.0 - px).max(px - 70.0).max(10.0 - py).max(py - 70.0);
            if px < 60.0 && py < 60.0 {
                d = d.max((px - 60.0).hypot(py - 60.0) - 50.0);
            }
            if px > 20.0 && py > 20.0 {
                d = d.max((px - 20.0).hypot(py - 20.0) - 50.0);
            }
            let v = ((0.5 - d).clamp(0.0, 1.0) * 255.0).round() as u8;
            near(
                pixel(&image, x, y),
                [v, v, v, 255],
                2,
                (x, y, "diagonal corners"),
            );
        }
    }
    let rect = UiRect::new(20.0, 20.0, 40.0, 40.0);
    for (stroke, outside_x, inside_x) in [
        (
            AlignedStroke::inside(StrokeStyle::new(ColorRgba::new(255, 0, 0, 255), 4.0)),
            19,
            21,
        ),
        (
            AlignedStroke::center(StrokeStyle::new(ColorRgba::new(255, 0, 0, 255), 4.0)),
            18,
            21,
        ),
        (
            AlignedStroke::outside(StrokeStyle::new(ColorRgba::new(255, 0, 0, 255), 4.0)),
            17,
            22,
        ),
    ] {
        let image = render(
            &mut renderer,
            vec![rich(
                PaintRect::new(rect, ColorRgba::new(0, 0, 255, 255)).stroke(stroke),
            )],
            1.0,
        );
        near(
            pixel(&image, outside_x, 40),
            if stroke.alignment == operad::StrokeAlignment::Inside {
                [0, 0, 0, 255]
            } else {
                [255, 0, 0, 255]
            },
            0,
            stroke,
        );
        near(
            pixel(&image, inside_x, 40),
            if stroke.alignment == operad::StrokeAlignment::Outside {
                [0, 0, 255, 255]
            } else {
                [255, 0, 0, 255]
            },
            0,
            stroke,
        );
    }
    let rect = UiRect::new(20.25, 20.0, 40.0, 40.0);
    let image = render(
        &mut renderer,
        vec![rich(
            PaintRect::new(rect, ColorRgba::new(0, 0, 255, 255)).stroke(AlignedStroke::inside(
                StrokeStyle::new(ColorRgba::new(255, 0, 0, 255), 0.25),
            )),
        )],
        1.0,
    );
    near(
        pixel(&image, 20, 40),
        [64, 0, 128, 255],
        1,
        "fractional correlated fill and border",
    );
    let image = render(
        &mut renderer,
        vec![rich(
            PaintRect::new(
                UiRect::new(20.0, 20.0, 40.0, 40.0),
                ColorRgba::new(0, 0, 255, 128),
            )
            .stroke(AlignedStroke::inside(StrokeStyle::new(
                ColorRgba::new(255, 0, 0, 128),
                2.0,
            ))),
        )],
        1.0,
    );
    near(
        pixel(&image, 20, 40),
        [128, 0, 64, 255],
        1,
        "translucent border over translucent fill",
    );
}

#[test]
fn circle_and_capsule_coverage_scale_without_polygon_facets() {
    let mut renderer = WgpuRenderer::default();
    for scale in [1.0, 2.0] {
        let center = UiPoint::new(42.25, 42.75);
        let radius = 20.5;
        let image = render(
            &mut renderer,
            vec![item(
                UiRect::new(10.0, 10.0, 70.0, 70.0),
                PaintKind::Circle {
                    center,
                    radius,
                    fill: ColorRgba::WHITE,
                    stroke: None,
                },
            )],
            scale,
        );
        let mut partial = 0;
        for y in 18..69 {
            for x in 18..69 {
                let px = (x as f32 + 0.5) / scale;
                let py = (y as f32 + 0.5) / scale;
                let alpha =
                    (0.5 - ((px - center.x).hypot(py - center.y) - radius) * scale).clamp(0.0, 1.0);
                let value = (alpha * 255.0).round() as u8;
                near(
                    pixel(&image, x, y),
                    [value, value, value, 255],
                    2,
                    (scale, x, y),
                );
                if value > 0 && value < 255 {
                    partial += 1;
                }
            }
        }
        assert!(partial > 5, "curved edge should contain antialiasing");
    }
    for (from, to, width) in [
        (UiPoint::new(20.25, 20.75), UiPoint::new(75.5, 56.25), 0.25),
        (UiPoint::new(20.25, 20.75), UiPoint::new(75.5, 56.25), 6.5),
        (UiPoint::new(50.0, 50.0), UiPoint::new(50.0, 50.0), 6.5),
    ] {
        let image = render(
            &mut renderer,
            vec![item(
                UiRect::new(5.0, 5.0, 100.0, 80.0),
                PaintKind::Line {
                    from,
                    to,
                    stroke: StrokeStyle::new(ColorRgba::WHITE, width),
                },
            )],
            1.0,
        );
        for y in 10..70 {
            for x in 10..90 {
                let p = UiPoint::new(x as f32 + 0.5, y as f32 + 0.5);
                let delta = UiPoint::new(to.x - from.x, to.y - from.y);
                let len = delta.x * delta.x + delta.y * delta.y;
                let t = if len == 0.0 {
                    0.0
                } else {
                    ((p.x - from.x) * delta.x + (p.y - from.y) * delta.y) / len
                }
                .clamp(0.0, 1.0);
                let d =
                    (p.x - from.x - t * delta.x).hypot(p.y - from.y - t * delta.y) - width / 2.0;
                let value = ((0.5 - d).clamp(0.0, 1.0) * 255.0).round() as u8;
                near(
                    pixel(&image, x, y),
                    [value, value, value, 255],
                    2,
                    (from, to, width, x, y),
                );
            }
        }
    }
}

// Dense numerical convolution of a binary rounded rectangle with a Gaussian.
// This reference uses neither the shader's erf approximation nor its quadrature.
fn shadow_reference(x: f64, y: f64, rect: UiRect, radii: CornerRadii, sigma: f64) -> f64 {
    let step = 0.25_f64;
    let mut sum = 0.0;
    let nx = (rect.width as f64 / step).ceil() as usize;
    let ny = (rect.height as f64 / step).ceil() as usize;
    let sx = rect.width as f64 / nx as f64;
    let sy = rect.height as f64 / ny as f64;
    for iy in 0..ny {
        for ix in 0..nx {
            let qx = rect.x as f64 + (ix as f64 + 0.5) * sx;
            let qy = rect.y as f64 + (iy as f64 + 0.5) * sy;
            let inside = [
                (
                    rect.x as f64,
                    rect.y as f64,
                    radii.top_left as f64,
                    1.0,
                    1.0,
                ),
                (
                    rect.right() as f64,
                    rect.y as f64,
                    radii.top_right as f64,
                    -1.0,
                    1.0,
                ),
                (
                    rect.right() as f64,
                    rect.bottom() as f64,
                    radii.bottom_right as f64,
                    -1.0,
                    -1.0,
                ),
                (
                    rect.x as f64,
                    rect.bottom() as f64,
                    radii.bottom_left as f64,
                    1.0,
                    -1.0,
                ),
            ]
            .iter()
            .all(|&(cx, cy, r, dx, dy)| {
                let lx = (qx - cx) * dx;
                let ly = (qy - cy) * dy;
                lx >= r || ly >= r || (lx - r).hypot(ly - r) <= r
            });
            if inside {
                sum += (-((x - qx).powi(2) + (y - qy).powi(2)) / (2.0 * sigma * sigma)).exp()
                    * sx
                    * sy
                    / (2.0 * std::f64::consts::PI * sigma * sigma);
            }
        }
    }
    sum
}

#[test]
fn gaussian_shadows_and_visible_insets_match_numerical_convolution() {
    let mut renderer = WgpuRenderer::default();
    let rect = UiRect::new(32.0, 28.0, 28.0, 24.0);
    let radii = CornerRadii::new(15.0, 2.0, 7.0, 0.0);
    for spread in [-2.0, 0.0, 3.0] {
        let offset = UiPoint::new(4.0, 5.0);
        let blur = 12.0;
        let image = render(
            &mut renderer,
            vec![rich(
                PaintRect::new(rect, ColorRgba::TRANSPARENT)
                    .corner_radii(radii)
                    .effect(PaintEffect::shadow(ColorRgba::WHITE, offset, blur, spread)),
            )],
            1.0,
        );
        let shadow = UiRect::new(
            rect.x + offset.x - spread,
            rect.y + offset.y - spread,
            rect.width + 2.0 * spread,
            rect.height + 2.0 * spread,
        );
        let corners = CornerRadii::new(
            (radii.top_left + spread).max(0.0),
            (radii.top_right + spread).max(0.0),
            (radii.bottom_right + spread).max(0.0),
            (radii.bottom_left + spread).max(0.0),
        );
        for (x, y) in [
            (30, 30),
            (40, 27),
            (60, 28),
            (62, 40),
            (49, 54),
            (38, 39),
            (20, 20),
        ] {
            let expected = (shadow_reference(x as f64 + 0.5, y as f64 + 0.5, shadow, corners, 4.0)
                * 255.0)
                .round() as u8;
            near(
                pixel(&image, x, y),
                [expected, expected, expected, 255],
                4,
                (spread, x, y, "Gaussian"),
            );
        }
    }
    let image = render(
        &mut renderer,
        vec![rich(PaintRect::new(rect, ColorRgba::WHITE).effect(
            PaintEffect::inset_shadow(ColorRgba::BLACK, UiPoint::new(3.0, 0.0), 12.0, 1.0),
        ))],
        1.0,
    );
    let hole = UiRect::new(36.0, 29.0, 26.0, 22.0);
    for (x, y) in [(32, 40), (34, 40), (39, 40), (48, 40), (58, 40), (42, 28)] {
        let value = (shadow_reference(x as f64 + 0.5, y as f64 + 0.5, hole, CornerRadii::ZERO, 4.0)
            * 255.0)
            .round() as u8;
        near(
            pixel(&image, x, y),
            [value, value, value, 255],
            2,
            (x, y, "inset over opaque fill"),
        );
    }
    assert!(pixel(&image, 32, 40)[0] < pixel(&image, 48, 40)[0]);
    near(
        pixel(&image, 31, 40),
        [0, 0, 0, 255],
        0,
        "inset clipped to element",
    );
}

#[test]
fn rounded_shadow_straight_edges_and_shortcut_boundaries_match_convolution() {
    let mut renderer = WgpuRenderer::default();
    let rect = UiRect::new(24.0, 24.0, 80.0, 64.0);
    // The asymmetric case makes each side's largest corner different. Large
    // opposite corners also exercise overlapping corner regions in the fallback.
    for radii in [
        CornerRadii::uniform(6.0),
        CornerRadii::new(18.0, 2.0, 12.0, 0.0),
        CornerRadii::new(48.0, 0.0, 48.0, 0.0),
    ] {
        for sigma in [2.0, 4.0] {
            let image = render(
                &mut renderer,
                vec![rich(
                    PaintRect::new(rect, ColorRgba::TRANSPARENT)
                        .corner_radii(radii)
                        .effect(PaintEffect::shadow(
                            ColorRgba::WHITE,
                            UiPoint::new(0.0, 0.0),
                            sigma * 3.0,
                            0.0,
                        )),
                )],
                1.0,
            );
            let boundary_x = (rect.x + radii.top_left.max(radii.bottom_left) + 4.0 * sigma) as u32;
            let boundary_y = (rect.y + radii.top_left.max(radii.top_right) + 4.0 * sigma) as u32;
            for (x, y) in [
                (64, 22),
                (64, 24),
                (64, 87),
                (64, 89),
                (22, 56),
                (24, 56),
                (103, 56),
                (105, 56),
                (boundary_x - 1, 24),
                (boundary_x, 24),
                (24, boundary_y - 1),
                (24, boundary_y),
            ] {
                let expected =
                    (shadow_reference(x as f64 + 0.5, y as f64 + 0.5, rect, radii, sigma as f64)
                        * 255.0)
                        .round() as u8;
                near(
                    pixel(&image, x, y),
                    [expected, expected, expected, 255],
                    4,
                    (radii, sigma, x, y, "rounded shadow convolution"),
                );
            }
            if radii.top_left <= 18.0 {
                let inset = render(
                    &mut renderer,
                    vec![rich(
                        PaintRect::new(rect, ColorRgba::WHITE)
                            .corner_radii(radii)
                            .effect(PaintEffect::inset_shadow(
                                ColorRgba::BLACK,
                                UiPoint::new(0.0, 0.0),
                                sigma * 3.0,
                                0.0,
                            )),
                    )],
                    1.0,
                );
                // At these fully covered straight-edge pixels, a black inset
                // over white has the same value as the blurred hole's mask.
                for (x, y) in [(64, 24), (64, 87), (24, 56), (103, 56), (64, 56)] {
                    let expected = (shadow_reference(
                        x as f64 + 0.5,
                        y as f64 + 0.5,
                        rect,
                        radii,
                        sigma as f64,
                    ) * 255.0)
                        .round() as u8;
                    near(
                        pixel(&inset, x, y),
                        [expected, expected, expected, 255],
                        4,
                        (radii, sigma, x, y, "rounded inset convolution"),
                    );
                }
            }
        }
    }
}

#[test]
fn sdf_gallery_and_steady_frame_timing() {
    let mut renderer = WgpuRenderer::default();
    let mut items = vec![rich(PaintRect::new(
        UiRect::new(0.0, 0.0, 160.0, 120.0),
        ColorRgba::new(20, 25, 35, 255),
    ))];
    items.push(rich(
        PaintRect::new(
            UiRect::new(10.0, 10.0, 65.0, 40.0),
            operad::PaintBrush::LinearGradient(
                LinearGradient::new(
                    UiPoint::new(10.0, 10.0),
                    UiPoint::new(75.0, 50.0),
                    ColorRgba::new(0, 205, 215, 255),
                    ColorRgba::new(155, 60, 230, 255),
                )
                .stop(0.5, ColorRgba::new(45, 100, 240, 255)),
            ),
        )
        .corner_radii(CornerRadii::new(20.0, 4.0, 14.0, 0.0))
        .stroke(AlignedStroke::inside(StrokeStyle::new(
            ColorRgba::new(210, 230, 255, 255),
            1.25,
        ))),
    ));
    items.push(rich(
        PaintRect::new(
            UiRect::new(90.0, 15.0, 52.0, 35.0),
            ColorRgba::new(50, 150, 230, 255),
        )
        .corner_radii(CornerRadii::uniform(8.0))
        .effect(PaintEffect::shadow(
            ColorRgba::new(0, 0, 0, 200),
            UiPoint::new(3.0, 4.0),
            12.0,
            0.0,
        ))
        .effect(PaintEffect::inset_shadow(
            ColorRgba::new(0, 0, 0, 150),
            UiPoint::new(2.0, 3.0),
            9.0,
            0.0,
        )),
    ));
    items.push(item(
        UiRect::new(10.0, 65.0, 45.0, 45.0),
        PaintKind::Circle {
            center: UiPoint::new(33.0, 88.0),
            radius: 17.5,
            fill: ColorRgba::new(235, 85, 110, 255),
            stroke: Some(StrokeStyle::new(ColorRgba::new(255, 220, 225, 255), 1.25)),
        },
    ));
    for (i, w) in [0.25, 1.0, 4.0, 8.0].into_iter().enumerate() {
        items.push(item(
            UiRect::new(65.0, 65.0, 80.0, 50.0),
            PaintKind::Line {
                from: UiPoint::new(65.0, 68.0 + i as f32 * 11.0),
                to: UiPoint::new(140.0, 75.0 + i as f32 * 11.0),
                stroke: StrokeStyle::new(ColorRgba::new(245, 185, 90, 255), w),
            },
        ));
    }
    let image = render(&mut renderer, items.clone(), 1.0);
    near(
        pixel(&image, 155, 115),
        [20, 25, 35, 255],
        0,
        "gallery background",
    );
    near(
        pixel(&image, 33, 88),
        [235, 85, 110, 255],
        0,
        "gallery circle",
    );
    assert!(
        pixel(&image, 91, 31)[2] < pixel(&image, 115, 31)[2],
        "inset shadow remains visible in a mixed batch"
    );
    if let Ok(dir) = std::env::var("OPERAD_SDF_ARTIFACT_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("gallery.rgba"), &image.pixels).unwrap();
    }
    // The timer includes full synchronous snapshot/readback, not just shader work.
    for _ in 0..5 {
        render(&mut renderer, items.clone(), 1.0);
    }
    let mut times = Vec::new();
    for _ in 0..30 {
        let start = std::time::Instant::now();
        render(&mut renderer, items.clone(), 1.0);
        times.push(start.elapsed().as_micros());
    }
    times.sort();
    eprintln!(
        "SDF 160x120 gallery synchronous snapshot: median={}us p95={}us (30 frames)",
        times[15], times[28]
    );
}

#[test]
fn instance_and_gradient_device_limit_errors_allow_the_next_frame() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits {
            max_buffer_size: 16384,
            max_uniform_buffer_binding_size: 16384,
            max_storage_buffer_binding_size: 16384,
            ..Default::default()
        },
        ..Default::default()
    }))
    .unwrap();
    let mut renderer = WgpuRenderer::with_device_queue(device, queue).unwrap();
    let rect = UiRect::new(2.0, 2.0, 10.0, 10.0);
    let request = |items| {
        RenderFrameRequest::new(
            RenderTarget::snapshot(PixelSize::new(16, 16)),
            UiSize::new(16.0, 16.0),
            PaintList { items },
        )
    };
    let shape = rich(PaintRect::new(rect, ColorRgba::WHITE));
    for count in [120, 121, 120] {
        let output =
            renderer.render_frame(request(vec![shape.clone(); count]), &EmptyResourceResolver);
        if count == 121 {
            assert!(output
                .unwrap_err()
                .to_string()
                .contains("SDF buffer requires"));
        } else {
            assert_eq!(
                pixel(&output.unwrap().snapshot.unwrap(), 5, 5),
                [255, 255, 255, 255]
            );
        }
    }
    for count in [512, 513, 512] {
        let mut gradient = LinearGradient::new(
            UiPoint::new(0.0, 0.0),
            UiPoint::new(16.0, 0.0),
            ColorRgba::WHITE,
            ColorRgba::WHITE,
        );
        gradient.stops = (0..count)
            .map(|i| GradientStop::new(i as f32 / (count - 1) as f32, ColorRgba::WHITE))
            .collect();
        let output = renderer.render_frame(
            request(vec![rich(PaintRect::new(
                rect,
                operad::PaintBrush::LinearGradient(gradient),
            ))]),
            &EmptyResourceResolver,
        );
        if count == 513 {
            assert!(output
                .unwrap_err()
                .to_string()
                .contains("SDF buffer requires"));
        } else {
            assert_eq!(
                pixel(&output.unwrap().snapshot.unwrap(), 5, 5),
                [255, 255, 255, 255]
            );
        }
    }
    let mut gradient = LinearGradient::new(
        UiPoint::new(0.0, 0.0),
        UiPoint::new(16.0, 0.0),
        ColorRgba::WHITE,
        ColorRgba::WHITE,
    );
    gradient.stops = (0..513)
        .map(|i| GradientStop::new(i as f32 / 512.0, ColorRgba::WHITE))
        .collect();
    for opacity in [0.0, 1.0] {
        let mut invisible = rich(PaintRect::new(
            rect,
            PaintBrush::LinearGradient(gradient.clone()),
        ));
        invisible.opacity = opacity;
        if opacity == 1.0 {
            invisible.clip_rect = UiRect::new(20.0, 20.0, 4.0, 4.0);
        }
        let output = renderer
            .render_frame(
                request(vec![shape.clone(), invisible]),
                &EmptyResourceResolver,
            )
            .expect("invisible gradient stops must not exhaust GPU storage");
        assert_eq!(pixel(&output.snapshot.unwrap(), 5, 5), [255, 255, 255, 255]);
    }
}

#[test]
fn occlusion_preserves_antialiased_edges_and_shadow_tails() {
    let mut renderer = WgpuRenderer::default();
    let transparent = item(
        UiRect::new(0.0, 0.0, 1.0, 1.0),
        PaintKind::Rect {
            fill: ColorRgba::TRANSPARENT,
            stroke: None,
            corner_radius: 0.0,
        },
    );
    let scene = vec![
        item(
            UiRect::new(20.25, 30.0, 2.0, 2.0),
            PaintKind::Rect {
                fill: ColorRgba::WHITE,
                stroke: None,
                corner_radius: 0.0,
            },
        ),
        item(
            UiRect::new(20.25, 15.25, 100.0, 80.0),
            PaintKind::Rect {
                fill: ColorRgba::BLACK,
                stroke: None,
                corner_radius: 0.0,
            },
        ),
    ];
    let reference = render(&mut renderer, scene.clone(), 1.0);
    near(
        pixel(&reference, 20, 30),
        [48, 48, 48, 255],
        1,
        "overlapping antialiased edges",
    );
    let mut many = scene;
    many.extend(vec![transparent.clone(); 128]);
    assert_eq!(
        reference.pixels,
        render(&mut renderer, many, 1.0).pixels,
        "occlusion changed fractional coverage"
    );
    let scene = vec![
        rich(
            PaintRect::new(UiRect::new(40.0, 40.0, 60.0, 60.0), ColorRgba::TRANSPARENT).effect(
                PaintEffect::shadow(ColorRgba::WHITE, UiPoint::new(30.0, 6.0), 12.0, 2.0),
            ),
        ),
        item(
            UiRect::new(34.0, 34.0, 74.0, 74.0),
            PaintKind::Rect {
                fill: ColorRgba::BLACK,
                stroke: None,
                corner_radius: 0.0,
            },
        ),
    ];
    let reference = render(&mut renderer, scene.clone(), 1.0);
    assert!(
        pixel(&reference, 130, 80)[0] > 50,
        "shadow tail must extend beyond the covering rectangle"
    );
    let mut many = scene;
    many.extend(vec![transparent; 128]);
    assert_eq!(
        reference.pixels,
        render(&mut renderer, many, 1.0).pixels,
        "occlusion discarded visible shadow tails"
    );
}

#[test]
fn compositor_clip_preserves_each_corner_of_a_gradient() {
    let mut renderer = WgpuRenderer::default();
    let rect = UiRect::new(10.0, 10.0, 60.0, 60.0);
    let gradient = LinearGradient::new(
        UiPoint::new(10.0, 10.0),
        UiPoint::new(70.0, 10.0),
        ColorRgba::new(255, 0, 0, 255),
        ColorRgba::new(0, 0, 255, 255),
    );
    let child = rich(PaintRect::new(rect, PaintBrush::LinearGradient(gradient)));
    let layer = operad::PaintCompositorLayer::new(rect, PaintList { items: vec![child] }).clip(
        operad::compositor::CompositorClip::rounded_rect(
            rect,
            CornerRadii::new(50.0, 0.0, 0.0, 0.0),
        ),
    );
    for scale in [1.0, 2.0] {
        let image = render(
            &mut renderer,
            vec![item(rect, PaintKind::CompositedLayer(layer.clone()))],
            scale,
        );
        for (x, y, visible) in [
            (11, 11, false),
            (68, 11, true),
            (11, 68, true),
            (68, 68, true),
            (44, 11, false),
            (55, 15, true),
        ] {
            let px = (x as f32 * scale) as u32;
            let py = (y as f32 * scale) as u32;
            if visible {
                let t = (((px as f32 + 0.5) / scale - 10.0) / 60.0).clamp(0.0, 1.0);
                near(
                    pixel(&image, px, py),
                    [
                        ((1.0 - t) * 255.0).round() as u8,
                        0,
                        (t * 255.0).round() as u8,
                        255,
                    ],
                    2,
                    (scale, x, y),
                );
            } else {
                near(pixel(&image, px, py), [0, 0, 0, 255], 0, (scale, x, y));
            }
        }
    }
}
