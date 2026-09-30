// Analytic shapes are described once per instance; the GPU evaluates their
// coverage. Gaussian shadows follow Evan Wallace's CC0 Gaussian-integral method:
// https://madebyevan.com/shaders/fast-rounded-rectangle-shadows/
// Independent corner radii use horizontal cross sections, with eight vertical
// samples rather than the reference's four. blur_extent is three sigma.

struct Scene { viewport: vec2<f32>, padding: vec2<f32> }
@group(0) @binding(0) var<uniform> scene: Scene;

struct GradientStop { position: vec4<f32>, color: vec4<f32> }
@group(1) @binding(0) var<storage, read> stops: array<GradientStop>;

struct Instance {
    @location(0) draw_rect: vec4<f32>,
    @location(1) shape_rect: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_color: vec4<f32>,
    @location(4) radii: vec4<f32>,
    @location(5) params: vec4<f32>,
    @location(6) gradient_line: vec4<f32>,
    @location(7) effect: vec4<f32>,
    @location(8) gradient_range: vec2<u32>,
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) shape_rect: vec4<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
    @location(2) @interpolate(flat) border_color: vec4<f32>,
    @location(3) @interpolate(flat) radii: vec4<f32>,
    @location(4) @interpolate(flat) params: vec4<f32>,
    @location(5) @interpolate(flat) gradient_line: vec4<f32>,
    @location(6) @interpolate(flat) effect: vec4<f32>,
    @location(7) @interpolate(flat) gradient_range: vec2<u32>,
    @location(8) @interpolate(flat) outer_rect: vec4<f32>,
    @location(9) @interpolate(flat) inner_rect: vec4<f32>,
    @location(10) @interpolate(flat) outer_radii: vec4<f32>,
    @location(11) @interpolate(flat) inner_radii: vec4<f32>,
    @location(12) @interpolate(flat) gradient_end_color: vec4<f32>,
}

@vertex
fn vs_sdf(@builtin(vertex_index) index: u32, instance: Instance) -> Fragment {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0));
    let point = instance.draw_rect.xy + corners[index] * instance.draw_rect.zw;
    var out: Fragment;
    out.position = vec4<f32>(point / scene.viewport * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.shape_rect = instance.shape_rect;
    out.color = instance.color;
    out.border_color = instance.border_color;
    out.radii = instance.radii;
    out.params = instance.params;
    out.gradient_line = instance.gradient_line;
    out.effect = instance.effect;
    out.gradient_range = instance.gradient_range;
    // Common two-stop gradients fetch once per vertex, avoiding a storage
    // search per fragment. Other gradients retain the binary-search program.
    out.gradient_end_color = vec4<f32>(0.0);
    if instance.gradient_range.y == 2u {
        let first = stops[instance.gradient_range.x];
        let last = stops[instance.gradient_range.x + 1u];
        out.color = first.color;
        out.gradient_end_color = last.color;
        out.effect = vec4<f32>(first.position.x, last.position.x, 0.0, 0.0);
    }
    // Border bounds and normalized radii are constant across the instance.
    let outset = instance.params.y * instance.params.z * 0.5;
    let inset = instance.params.y - outset;
    out.outer_rect = vec4<f32>(instance.shape_rect.xy - vec2<f32>(outset), instance.shape_rect.zw + 2.0 * outset);
    out.inner_rect = vec4<f32>(instance.shape_rect.xy + vec2<f32>(inset), instance.shape_rect.zw - 2.0 * inset);
    out.outer_radii = normalize_radii(select(instance.radii, instance.radii + vec4<f32>(outset), instance.radii > vec4<f32>(0.0)), out.outer_rect.zw);
    out.inner_radii = normalize_radii(max(instance.radii - vec4<f32>(inset), vec4<f32>(0.0)), max(out.inner_rect.zw, vec2<f32>(0.0)));
    if instance.params.x == 4.0 {
        let spread = instance.effect.z;
        out.inner_rect = vec4<f32>(instance.shape_rect.xy + instance.effect.xy + vec2<f32>(spread), instance.shape_rect.zw - 2.0 * spread);
        out.inner_radii = normalize_radii(max(instance.radii - vec4<f32>(spread), vec4<f32>(0.0)), max(out.inner_rect.zw, vec2<f32>(0.0)));
    }
    return out;
}

fn shape_distance(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>, kind: f32) -> f32 {
    return select(rect_distance(p, rect, radii), length(p - (rect.xy + rect.zw * 0.5)) - rect.z * 0.5, kind == 1.0);
}

fn two_stop_gradient_color(input: Fragment) -> vec4<f32> {
    let delta = input.gradient_line.zw - input.gradient_line.xy;
    let length_squared = dot(delta, delta);
    let t = clamp(dot(input.position.xy - input.gradient_line.xy, delta) / max(length_squared, 0.00000001), 0.0, 1.0);
    let fraction = clamp((t - input.effect.x) / max(input.effect.y - input.effect.x, 0.00000001), 0.0, 1.0);
    let color = select(mix(input.color, input.gradient_end_color, fraction), input.gradient_end_color, t > input.effect.x && input.effect.y == input.effect.x);
    return select(color, input.gradient_end_color, length_squared <= 0.00000001);
}

fn gradient_color(input: Fragment) -> vec4<f32> {
    let base = input.gradient_range.x;
    let count = input.gradient_range.y;
    if count == 0u { return input.color; }
    let delta = input.gradient_line.zw - input.gradient_line.xy;
    let length_squared = dot(delta, delta);
    if length_squared <= 0.00000001 { return stops[base + count - 1u].color; }
    let t = clamp(dot(input.position.xy - input.gradient_line.xy, delta) / length_squared, 0.0, 1.0);
    // Lower bound gives the first authored stop at an exact duplicate offset.
    var lo = 0u;
    var hi = count;
    loop {
        if lo >= hi { break; }
        let mid = lo + (hi - lo) / 2u;
        if stops[base + mid].position.x < t { lo = mid + 1u; } else { hi = mid; }
    }
    if lo == 0u { return stops[base].color; }
    if lo == count { return stops[base + count - 1u].color; }
    let left = stops[base + lo - 1u];
    let right = stops[base + lo];
    let fraction = clamp((t - left.position.x) / max(right.position.x - left.position.x, 0.00000001), 0.0, 1.0);
    return mix(left.color, right.color, fraction);
}

fn erf2(value: vec2<f32>) -> vec2<f32> {
    let a = abs(value);
    let polynomial = 1.0 + (0.278393 + (0.230389 + 0.078108 * a * a) * a) * a;
    let square = polynomial * polynomial;
    return sign(value) * (1.0 - 1.0 / (square * square));
}

fn gaussian_interval(point: f32, lower: f32, upper: f32, sigma: f32) -> f32 {
    let integral = 0.5 + 0.5 * erf2((vec2<f32>(upper, lower) - point) * (0.70710678118 / sigma));
    return max(integral.x - integral.y, 0.0);
}

fn corner_inset(radius: f32, distance_to_edge: f32) -> f32 {
    let delta = max(radius - distance_to_edge, 0.0);
    return radius - sqrt(max(radius * radius - delta * delta, 0.0));
}

fn blurred_rect(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>, blur_extent: f32) -> f32 {
    if any(rect.zw <= vec2<f32>(0.0)) { return 0.0; }
    if blur_extent <= 0.0001 { return coverage(rect_distance(p, rect, radii)); }
    let sigma = blur_extent / 3.0;
    let end = rect.xy + rect.zw;
    if all(radii == vec4<f32>(0.0)) {
        return gaussian_interval(p.x, rect.x, end.x, sigma) * gaussian_interval(p.y, rect.y, end.y, sigma);
    }
    let lower = max(p.y - end.y, -blur_extent);
    let upper = min(p.y - rect.y, blur_extent);
    if lower >= upper { return 0.0; }
    // A corner farther than four sigma contributes less than 0.000064 of
    // Gaussian mass. Straight edges and interiors can use a separable box
    // integral; keep the same finite vertical support as the corner fallback.
    let margin = 4.0 * sigma;
    let straight_x = p.x >= rect.x + max(radii.x, radii.w) + margin
        && p.x <= end.x - max(radii.y, radii.z) - margin;
    let straight_y = p.y >= rect.y + max(radii.x, radii.y) + margin
        && p.y <= end.y - max(radii.w, radii.z) - margin;
    if straight_x || straight_y {
        return gaussian_interval(p.x, rect.x, end.x, sigma) * gaussian_interval(0.0, lower, upper, sigma);
    }
    let step = (upper - lower) / 8.0;
    var alpha = 0.0;
    for (var i = 0u; i < 8u; i += 1u) {
        let offset = lower + (f32(i) + 0.5) * step;
        let y = p.y - offset;
        let left_inset = max(corner_inset(radii.x, y - rect.y), corner_inset(radii.w, end.y - y));
        let right_inset = max(corner_inset(radii.y, y - rect.y), corner_inset(radii.z, end.y - y));
        let horizontal = gaussian_interval(p.x, rect.x + left_inset, end.x - right_inset, sigma);
        alpha += horizontal * exp(-0.5 * offset * offset / (sigma * sigma)) * step / (2.50662827463 * sigma);
    }
    return clamp(alpha, 0.0, 1.0);
}

fn linear_channel(value: f32) -> f32 {
    if value <= 0.04045 { return value / 12.92; }
    return pow((value + 0.055) / 1.055, 2.4);
}

fn target_color(color: vec4<f32>, srgb: bool) -> vec4<f32> {
    if !srgb { return color; }
    return vec4<f32>(linear_channel(color.r), linear_channel(color.g), linear_channel(color.b), color.a);
}

fn fill_color(input: Fragment, color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(color.rgb, color.a * coverage(shape_distance(input.position.xy, input.shape_rect, input.radii, input.params.x)));
}
fn segment_color(input: Fragment, color: vec4<f32>) -> vec4<f32> {
    let delta = input.shape_rect.zw - input.shape_rect.xy;
    let t = clamp(dot(input.position.xy - input.shape_rect.xy, delta) / max(dot(delta, delta), 0.00000001), 0.0, 1.0);
    return vec4<f32>(color.rgb, color.a * coverage(length(input.position.xy - (input.shape_rect.xy + t * delta)) - input.params.y * 0.5));
}
fn border_color(input: Fragment, color: vec4<f32>, border: vec4<f32>) -> vec4<f32> {
    let p = input.position.xy;
    let kind = input.params.x;
    let fill = coverage(shape_distance(p, input.shape_rect, input.radii, kind));
    let outer = coverage(shape_distance(p, input.outer_rect, input.outer_radii, kind));
    let inner = coverage(shape_distance(p, input.inner_rect, input.inner_radii, kind));
    let border_coverage = max(outer - inner, 0.0);
    let overlap = max(min(fill, outer) - inner, 0.0);
    let background_alpha = color.a * (fill - overlap * border.a);
    let border_alpha = border.a * border_coverage;
    let alpha = background_alpha + border_alpha;
    return vec4<f32>((color.rgb * background_alpha + border.rgb * border_alpha) / max(alpha, 0.00000001), alpha);
}
fn shadow_color(input: Fragment, color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(color.rgb, color.a * blurred_rect(input.position.xy, input.shape_rect, input.radii, input.params.w));
}
fn inset_color(input: Fragment, color: vec4<f32>) -> vec4<f32> {
    let alpha = (1.0 - blurred_rect(input.position.xy, input.inner_rect, input.inner_radii, input.params.w)) * coverage(rect_distance(input.position.xy, input.shape_rect, input.radii));
    return vec4<f32>(color.rgb, color.a * alpha);
}
@fragment fn fs_sdf(input: Fragment) -> @location(0) vec4<f32> { return fill_color(input, input.color); }
@fragment fn fs_sdf_srgb(input: Fragment) -> @location(0) vec4<f32> { return fill_color(input, target_color(input.color, true)); }
@fragment fn fs_sdf_border(input: Fragment) -> @location(0) vec4<f32> { return border_color(input, input.color, input.border_color); }
@fragment fn fs_sdf_border_srgb(input: Fragment) -> @location(0) vec4<f32> { return border_color(input, target_color(input.color, true), target_color(input.border_color, true)); }
@fragment fn fs_sdf_multistop(input: Fragment) -> @location(0) vec4<f32> { return fill_color(input, gradient_color(input)); }
@fragment fn fs_sdf_multistop_srgb(input: Fragment) -> @location(0) vec4<f32> { return fill_color(input, target_color(gradient_color(input), true)); }
@fragment fn fs_sdf_multistop_border(input: Fragment) -> @location(0) vec4<f32> { return border_color(input, gradient_color(input), input.border_color); }
@fragment fn fs_sdf_multistop_border_srgb(input: Fragment) -> @location(0) vec4<f32> { return border_color(input, target_color(gradient_color(input), true), target_color(input.border_color, true)); }
@fragment fn fs_sdf_segment(input: Fragment) -> @location(0) vec4<f32> { return segment_color(input, input.color); }
@fragment fn fs_sdf_segment_srgb(input: Fragment) -> @location(0) vec4<f32> { return segment_color(input, target_color(input.color, true)); }
@fragment fn fs_sdf_shadow(input: Fragment) -> @location(0) vec4<f32> { return shadow_color(input, input.color); }
@fragment fn fs_sdf_shadow_srgb(input: Fragment) -> @location(0) vec4<f32> { return shadow_color(input, target_color(input.color, true)); }
@fragment fn fs_sdf_inset(input: Fragment) -> @location(0) vec4<f32> { return inset_color(input, input.color); }
@fragment fn fs_sdf_inset_srgb(input: Fragment) -> @location(0) vec4<f32> { return inset_color(input, target_color(input.color, true)); }
@fragment fn fs_sdf_gradient(input: Fragment) -> @location(0) vec4<f32> { return fill_color(input, two_stop_gradient_color(input)); }
@fragment fn fs_sdf_gradient_srgb(input: Fragment) -> @location(0) vec4<f32> { return fill_color(input, target_color(two_stop_gradient_color(input), true)); }
@fragment fn fs_sdf_gradient_border(input: Fragment) -> @location(0) vec4<f32> { return border_color(input, two_stop_gradient_color(input), input.border_color); }
@fragment fn fs_sdf_gradient_border_srgb(input: Fragment) -> @location(0) vec4<f32> { return border_color(input, target_color(two_stop_gradient_color(input), true), target_color(input.border_color, true)); }
