#![allow(dead_code)]

use std::fs;
use std::path::Path;

use operad::{
    ApproxTextMeasurer, CanvasContent, ColorRgba, PaintItem, PaintKind, PaintTransform,
    StrokeStyle, TextContent, UiDocument, UiPoint, UiRect, UiSize,
};

pub const SNAPSHOT_BACKGROUND: ColorRgba = ColorRgba::new(9, 12, 16, 255);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl RasterImage {
    pub fn new(width: usize, height: usize, background: ColorRgba) -> Self {
        let mut pixels = vec![0; width * height * 4];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[0] = background.r;
            pixel[1] = background.g;
            pixel[2] = background.b;
            pixel[3] = background.a;
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn hash(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in &self.pixels {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    pub fn changed_pixels_from(&self, color: ColorRgba) -> usize {
        self.pixels
            .chunks_exact(4)
            .filter(|pixel| *pixel != [color.r, color.g, color.b, color.a])
            .count()
    }

    pub fn write_ppm(&self, path: impl AsRef<Path>) {
        let mut data = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        for pixel in self.pixels.chunks_exact(4) {
            data.extend_from_slice(&pixel[..3]);
        }
        fs::write(path, data).expect("write snapshot ppm");
    }
}

pub fn render_document(document: &mut UiDocument, viewport: UiSize) -> RasterImage {
    document
        .compute_layout(viewport, &mut ApproxTextMeasurer)
        .expect("layout");
    let mut image = RasterImage::new(
        viewport.width.round() as usize,
        viewport.height.round() as usize,
        SNAPSHOT_BACKGROUND,
    );
    for item in document.paint_list().items {
        draw_item(&mut image, &item);
    }
    image
}

pub fn assert_snapshot(name: &str, image: &RasterImage, expected_hash: u64) {
    if let Ok(dir) = std::env::var("OPERAD_WRITE_SNAPSHOTS") {
        fs::create_dir_all(&dir).expect("create snapshot directory");
        image.write_ppm(Path::new(&dir).join(format!("{name}.ppm")));
    }
    let changed_pixels = image.changed_pixels_from(SNAPSHOT_BACKGROUND);
    assert!(
        changed_pixels > image.width * image.height / 40,
        "{name} rendered too little content: {changed_pixels} changed pixels"
    );
    let actual = image.hash();
    if expected_hash == 0 {
        panic!("{name} snapshot hash: {actual:#018x}");
    }
    assert_eq!(actual, expected_hash, "{name} snapshot hash changed");
}

fn draw_item(image: &mut RasterImage, item: &PaintItem) {
    let clip = item.clip_rect;
    match &item.kind {
        PaintKind::Rect {
            fill,
            stroke,
            corner_radius: _,
        } => {
            let rect = transform_rect(item.rect, item.transform);
            fill_rect(image, rect, clip, *fill, item.opacity);
            if let Some(stroke) = stroke {
                stroke_rect(image, rect, clip, *stroke, item.opacity);
            }
        }
        PaintKind::RichRect(rect_primitive) => {
            let rect = transform_rect(rect_primitive.rect, item.transform);
            for effect in &rect_primitive.effects {
                let spread = effect.spread.max(0.0) + effect.blur_radius.max(0.0) * 0.25;
                let effect_rect = UiRect::new(
                    rect.x + effect.offset.x - spread,
                    rect.y + effect.offset.y - spread,
                    rect.width + spread * 2.0,
                    rect.height + spread * 2.0,
                );
                fill_rect(image, effect_rect, clip, effect.color, item.opacity);
            }
            fill_rect(
                image,
                rect,
                clip,
                rect_primitive.fill.fallback_color(),
                item.opacity,
            );
            if let Some(stroke) = rect_primitive.stroke {
                stroke_rect(image, rect, clip, stroke.style, item.opacity);
            }
        }
        PaintKind::Text(text) => draw_text(image, item, text),
        PaintKind::SceneText(text) => {
            let text_content = TextContent::new(text.text.clone(), text.style.clone());
            let item = PaintItem {
                rect: text.rect,
                kind: PaintKind::Text(text_content.clone()),
                ..(*item).clone()
            };
            draw_text(image, &item, &text_content);
        }
        PaintKind::Canvas(canvas) => draw_canvas(image, item, canvas),
        PaintKind::Line { from, to, stroke } => {
            draw_line(
                image,
                transform_point(*from, item.transform),
                transform_point(*to, item.transform),
                clip,
                *stroke,
                item.opacity,
            );
        }
        PaintKind::Circle {
            center,
            radius,
            fill,
            stroke,
        } => {
            let center = transform_point(*center, item.transform);
            let radius = radius * item.transform.scale.max(0.0);
            fill_circle(image, center, radius, clip, *fill, item.opacity);
            if let Some(stroke) = stroke {
                stroke_circle(image, center, radius, clip, *stroke, item.opacity);
            }
        }
        PaintKind::Polygon {
            points,
            fill,
            stroke,
        } => {
            let points = points
                .iter()
                .copied()
                .map(|point| transform_point(point, item.transform))
                .collect::<Vec<_>>();
            fill_polygon(image, &points, clip, *fill, item.opacity);
            if let Some(stroke) = stroke {
                for segment in points.windows(2) {
                    draw_line(image, segment[0], segment[1], clip, *stroke, item.opacity);
                }
                if points.len() > 2 {
                    draw_line(
                        image,
                        *points.last().unwrap(),
                        points[0],
                        clip,
                        *stroke,
                        item.opacity,
                    );
                }
            }
        }
        PaintKind::Image { key, tint } => {
            draw_image_placeholder(
                image,
                transform_rect(item.rect, item.transform),
                clip,
                key,
                *tint,
            );
        }
        PaintKind::Path(path) => {
            let points = path
                .verbs
                .iter()
                .filter_map(|verb| match *verb {
                    operad::PathVerb::MoveTo(point) | operad::PathVerb::LineTo(point) => {
                        Some(transform_point(point, item.transform))
                    }
                    operad::PathVerb::QuadraticTo { to, .. }
                    | operad::PathVerb::CubicTo { to, .. } => {
                        Some(transform_point(to, item.transform))
                    }
                    operad::PathVerb::Close => None,
                })
                .collect::<Vec<_>>();
            if let Some(fill) = &path.fill {
                fill_polygon(image, &points, clip, fill.fallback_color(), item.opacity);
            }
            if let Some(stroke) = path.stroke {
                for segment in points.windows(2) {
                    draw_line(
                        image,
                        segment[0],
                        segment[1],
                        clip,
                        stroke.style,
                        item.opacity,
                    );
                }
            }
        }
        PaintKind::ImagePlacement(image_placement) => {
            draw_image_placeholder(
                image,
                transform_rect(image_placement.rect, item.transform),
                clip,
                &image_placement.key,
                image_placement.tint,
            );
        }
    }
}

fn draw_text(image: &mut RasterImage, item: &PaintItem, text: &TextContent) {
    let rect = transform_rect(item.rect, item.transform);
    let color = text.style.color;
    let glyph_width = (text.style.font_size * item.transform.scale * 0.52).max(4.0);
    let glyph_height = (text.style.line_height * item.transform.scale * 0.70).max(5.0);
    let baseline_y = rect.y + (text.style.line_height * item.transform.scale * 0.18).max(1.0);
    let mut x = rect.x;
    let mut y = baseline_y;
    for ch in text.text.chars() {
        if ch == '\n' {
            x = rect.x;
            y += text.style.line_height * item.transform.scale;
            continue;
        }
        if !ch.is_whitespace() {
            let hash = hash_str(&ch.to_string());
            let inset = (hash % 3) as f32;
            fill_rect(
                image,
                UiRect::new(
                    x + inset,
                    y + inset,
                    (glyph_width - inset).max(1.0),
                    (glyph_height - inset * 2.0).max(1.0),
                ),
                item.clip_rect,
                color,
                item.opacity,
            );
        }
        x += glyph_width;
        if x > rect.right() {
            break;
        }
    }
}

fn draw_canvas(image: &mut RasterImage, item: &PaintItem, canvas: &CanvasContent) {
    let rect = transform_rect(item.rect, item.transform);
    let base = color_from_key(&canvas.key, 210);
    fill_rect(image, rect, item.clip_rect, base, item.opacity);
    let accent = ColorRgba::new(
        base.r.saturating_add(34),
        base.g.saturating_add(24),
        base.b.saturating_add(18),
        255,
    );
    let step = 12.0;
    let mut x = rect.x;
    while x < rect.right() {
        draw_line(
            image,
            UiPoint::new(x, rect.y),
            UiPoint::new(x + rect.height, rect.bottom()),
            item.clip_rect,
            StrokeStyle::new(accent, 1.0),
            item.opacity,
        );
        x += step;
    }
}

fn draw_image_placeholder(
    image: &mut RasterImage,
    rect: UiRect,
    clip: UiRect,
    key: &str,
    tint: Option<ColorRgba>,
) {
    let base = tint.unwrap_or_else(|| color_from_key(key, 235));
    fill_rect(image, rect, clip, base, 1.0);
    let hash = hash_str(key);
    let stripe = ColorRgba::new(
        base.r.saturating_sub(((hash >> 8) & 31) as u8),
        base.g.saturating_sub(((hash >> 16) & 31) as u8),
        base.b.saturating_sub(((hash >> 24) & 31) as u8),
        base.a,
    );
    let mut x = rect.x;
    while x < rect.right() {
        fill_rect(
            image,
            UiRect::new(x, rect.y, 2.0, rect.height),
            clip,
            stripe,
            0.8,
        );
        x += 6.0;
    }
}

fn fill_rect(image: &mut RasterImage, rect: UiRect, clip: UiRect, color: ColorRgba, opacity: f32) {
    if color.a == 0 || opacity <= 0.0 {
        return;
    }
    let Some(rect) = rect.intersection(clip) else {
        return;
    };
    let left = rect.x.floor().max(0.0) as usize;
    let top = rect.y.floor().max(0.0) as usize;
    let right = rect.right().ceil().min(image.width as f32) as usize;
    let bottom = rect.bottom().ceil().min(image.height as f32) as usize;
    for y in top..bottom {
        for x in left..right {
            blend_pixel(image, x, y, color, opacity);
        }
    }
}

fn stroke_rect(
    image: &mut RasterImage,
    rect: UiRect,
    clip: UiRect,
    stroke: StrokeStyle,
    opacity: f32,
) {
    let width = stroke.width.max(1.0);
    fill_rect(
        image,
        UiRect::new(rect.x, rect.y, rect.width, width),
        clip,
        stroke.color,
        opacity,
    );
    fill_rect(
        image,
        UiRect::new(rect.x, rect.bottom() - width, rect.width, width),
        clip,
        stroke.color,
        opacity,
    );
    fill_rect(
        image,
        UiRect::new(rect.x, rect.y, width, rect.height),
        clip,
        stroke.color,
        opacity,
    );
    fill_rect(
        image,
        UiRect::new(rect.right() - width, rect.y, width, rect.height),
        clip,
        stroke.color,
        opacity,
    );
}

fn draw_line(
    image: &mut RasterImage,
    from: UiPoint,
    to: UiPoint,
    clip: UiRect,
    stroke: StrokeStyle,
    opacity: f32,
) {
    let min_x = from.x.min(to.x).floor().max(0.0) as usize;
    let min_y = from.y.min(to.y).floor().max(0.0) as usize;
    let max_x = from.x.max(to.x).ceil().min(image.width as f32 - 1.0) as usize;
    let max_y = from.y.max(to.y).ceil().min(image.height as f32 - 1.0) as usize;
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f32::EPSILON {
        fill_rect(
            image,
            UiRect::new(from.x, from.y, stroke.width.max(1.0), stroke.width.max(1.0)),
            clip,
            stroke.color,
            opacity,
        );
        return;
    }
    let radius = stroke.width.max(1.0) * 0.5 + 0.75;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = UiPoint::new(x as f32 + 0.5, y as f32 + 0.5);
            if !clip.contains_point(point) {
                continue;
            }
            let t = (((point.x - from.x) * dx + (point.y - from.y) * dy) / length_squared)
                .clamp(0.0, 1.0);
            let closest = UiPoint::new(from.x + dx * t, from.y + dy * t);
            let distance = ((point.x - closest.x).powi(2) + (point.y - closest.y).powi(2)).sqrt();
            if distance <= radius {
                blend_pixel(image, x, y, stroke.color, opacity);
            }
        }
    }
}

fn fill_circle(
    image: &mut RasterImage,
    center: UiPoint,
    radius: f32,
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
) {
    let bounds = UiRect::new(
        center.x - radius,
        center.y - radius,
        radius * 2.0,
        radius * 2.0,
    );
    let Some(bounds) = bounds.intersection(clip) else {
        return;
    };
    let left = bounds.x.floor().max(0.0) as usize;
    let top = bounds.y.floor().max(0.0) as usize;
    let right = bounds.right().ceil().min(image.width as f32) as usize;
    let bottom = bounds.bottom().ceil().min(image.height as f32) as usize;
    let radius_squared = radius * radius;
    for y in top..bottom {
        for x in left..right {
            let dx = x as f32 + 0.5 - center.x;
            let dy = y as f32 + 0.5 - center.y;
            if dx * dx + dy * dy <= radius_squared {
                blend_pixel(image, x, y, color, opacity);
            }
        }
    }
}

fn stroke_circle(
    image: &mut RasterImage,
    center: UiPoint,
    radius: f32,
    clip: UiRect,
    stroke: StrokeStyle,
    opacity: f32,
) {
    let bounds = UiRect::new(
        center.x - radius,
        center.y - radius,
        radius * 2.0,
        radius * 2.0,
    );
    let Some(bounds) = bounds.intersection(clip) else {
        return;
    };
    let left = bounds.x.floor().max(0.0) as usize;
    let top = bounds.y.floor().max(0.0) as usize;
    let right = bounds.right().ceil().min(image.width as f32) as usize;
    let bottom = bounds.bottom().ceil().min(image.height as f32) as usize;
    let half = stroke.width.max(1.0) * 0.5;
    for y in top..bottom {
        for x in left..right {
            let dx = x as f32 + 0.5 - center.x;
            let dy = y as f32 + 0.5 - center.y;
            let distance = (dx * dx + dy * dy).sqrt();
            if (radius - half..=radius + half).contains(&distance) {
                blend_pixel(image, x, y, stroke.color, opacity);
            }
        }
    }
}

fn fill_polygon(
    image: &mut RasterImage,
    points: &[UiPoint],
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
) {
    if points.len() < 3 {
        return;
    }
    let mut left = points[0].x;
    let mut top = points[0].y;
    let mut right = points[0].x;
    let mut bottom = points[0].y;
    for point in points {
        left = left.min(point.x);
        top = top.min(point.y);
        right = right.max(point.x);
        bottom = bottom.max(point.y);
    }
    let Some(bounds) = UiRect::new(left, top, right - left, bottom - top).intersection(clip) else {
        return;
    };
    let left = bounds.x.floor().max(0.0) as usize;
    let top = bounds.y.floor().max(0.0) as usize;
    let right = bounds.right().ceil().min(image.width as f32) as usize;
    let bottom = bounds.bottom().ceil().min(image.height as f32) as usize;
    for y in top..bottom {
        for x in left..right {
            if point_in_polygon(UiPoint::new(x as f32 + 0.5, y as f32 + 0.5), points) {
                blend_pixel(image, x, y, color, opacity);
            }
        }
    }
}

fn point_in_polygon(point: UiPoint, points: &[UiPoint]) -> bool {
    let mut inside = false;
    let mut previous = points.len() - 1;
    for current in 0..points.len() {
        let pi = points[current];
        let pj = points[previous];
        if ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn blend_pixel(image: &mut RasterImage, x: usize, y: usize, color: ColorRgba, opacity: f32) {
    if x >= image.width || y >= image.height {
        return;
    }
    let index = (y * image.width + x) * 4;
    let alpha = (f32::from(color.a) / 255.0 * opacity.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let inv = 1.0 - alpha;
    image.pixels[index] = (f32::from(image.pixels[index]) * inv + f32::from(color.r) * alpha)
        .round()
        .clamp(0.0, 255.0) as u8;
    image.pixels[index + 1] = (f32::from(image.pixels[index + 1]) * inv
        + f32::from(color.g) * alpha)
        .round()
        .clamp(0.0, 255.0) as u8;
    image.pixels[index + 2] = (f32::from(image.pixels[index + 2]) * inv
        + f32::from(color.b) * alpha)
        .round()
        .clamp(0.0, 255.0) as u8;
    image.pixels[index + 3] = 255;
}

fn transform_point(point: UiPoint, transform: PaintTransform) -> UiPoint {
    UiPoint::new(
        point.x * transform.scale + transform.translation.x,
        point.y * transform.scale + transform.translation.y,
    )
}

fn transform_rect(rect: UiRect, transform: PaintTransform) -> UiRect {
    let top_left = transform_point(UiPoint::new(rect.x, rect.y), transform);
    UiRect::new(
        top_left.x,
        top_left.y,
        rect.width * transform.scale,
        rect.height * transform.scale,
    )
}

fn color_from_key(key: &str, alpha: u8) -> ColorRgba {
    let hash = hash_str(key);
    ColorRgba::new(
        48 + (hash & 127) as u8,
        58 + ((hash >> 8) & 127) as u8,
        68 + ((hash >> 16) & 127) as u8,
        alpha,
    )
}

fn hash_str(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
