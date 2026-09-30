// Shared analytic shape coverage for primitives and compositor masks.
fn coverage(distance: f32) -> f32 { return clamp(0.5 - distance, 0.0, 1.0); }

fn normalize_radii(radii: vec4<f32>, size: vec2<f32>) -> vec4<f32> {
    let limits = vec4<f32>(size.x, size.x, size.y, size.y);
    let sums = vec4<f32>(radii.x + radii.y, radii.w + radii.z, radii.x + radii.w, radii.y + radii.z);
    let ratios = limits / max(sums, vec4<f32>(0.000001));
    return radii * min(1.0, min(min(ratios.x, ratios.y), min(ratios.z, ratios.w)));
}

fn rect_distance(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>) -> f32 {
    let end = rect.xy + rect.zw;
    let tl = p.x < rect.x + radii.x && p.y < rect.y + radii.x;
    let tr = p.x > end.x - radii.y && p.y < rect.y + radii.y;
    let br = p.x > end.x - radii.z && p.y > end.y - radii.z;
    let bl = p.x < rect.x + radii.w && p.y > end.y - radii.w;
    let radius = select(select(select(select(0.0, radii.w, bl), radii.z, br), radii.y, tr), radii.x, tl);
    let center = select(select(select(select(rect.xy + rect.zw * 0.5,
        vec2<f32>(rect.x + radii.w, end.y - radii.w), bl), end - vec2<f32>(radii.z), br),
        vec2<f32>(end.x - radii.y, rect.y + radii.y), tr), rect.xy + vec2<f32>(radii.x), tl);
    let q = abs(p - (rect.xy + rect.zw * 0.5)) - rect.zw * 0.5;
    let box = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
    // Normalized adjacent corner regions cannot overlap. Diagonally opposite
    // regions can, so both excluded corner disks must constrain coverage.
    let opposite_center = select(end - vec2<f32>(radii.z), vec2<f32>(rect.x + radii.w, end.y - radii.w), tr && bl);
    let opposite_radius = select(radii.z, radii.w, tr && bl);
    let opposite = select(-1e20, length(p - opposite_center) - opposite_radius, (tl && br) || (tr && bl));
    let corner = select(-1e20, length(p - center) - radius, tl || tr || br || bl);
    let distance = max(box, max(corner, opposite));
    return select(distance, 1e20, any(rect.zw <= vec2<f32>(0.0)));
}

