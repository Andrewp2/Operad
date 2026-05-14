struct CubeUniforms {
    yaw: f32,
    pitch: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0)
var<uniform> cube: CubeUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5, 0.5);
    return output;
}

fn saturate(value: f32) -> f32 {
    return clamp(value, 0.0, 1.0);
}

fn blend(base: vec3<f32>, color: vec3<f32>, alpha: f32) -> vec3<f32> {
    return base * (1.0 - alpha) + color * alpha;
}

fn ramp(low: vec3<f32>, high: vec3<f32>, value: f32) -> vec3<f32> {
    return low + (high - low) * saturate(value);
}

fn rotate_x(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, c, s),
        vec3<f32>(0.0, -s, c),
    );
}

fn rotate_y(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        vec3<f32>(c, 0.0, -s),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(s, 0.0, c),
    );
}

fn rotate_z(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        vec3<f32>(c, s, 0.0),
        vec3<f32>(-s, c, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    );
}

fn cube_space(point: vec3<f32>) -> vec3<f32> {
    var p = point;
    p = rotate_y(-cube.yaw) * p;
    p = rotate_x(cube.pitch) * p;
    p = rotate_z(-0.18) * p;
    return p;
}

fn cube_sdf(point: vec3<f32>) -> f32 {
    let half_size = vec3<f32>(0.68, 0.68, 0.68);
    let q = abs(cube_space(point)) - half_size;
    return length(max(q, vec3<f32>(0.0, 0.0, 0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn cube_normal(point: vec3<f32>) -> vec3<f32> {
    let e = 0.0025;
    let x = cube_sdf(point + vec3<f32>(e, 0.0, 0.0)) - cube_sdf(point - vec3<f32>(e, 0.0, 0.0));
    let y = cube_sdf(point + vec3<f32>(0.0, e, 0.0)) - cube_sdf(point - vec3<f32>(0.0, e, 0.0));
    let z = cube_sdf(point + vec3<f32>(0.0, 0.0, e)) - cube_sdf(point - vec3<f32>(0.0, 0.0, e));
    return normalize(vec3<f32>(x, y, z));
}

fn trace_cube(ray_origin: vec3<f32>, ray_direction: vec3<f32>) -> f32 {
    var distance = 0.0;
    for (var step_index = 0; step_index < 80; step_index = step_index + 1) {
        let point = ray_origin + ray_direction * distance;
        let scene_distance = cube_sdf(point);
        if (scene_distance < 0.0015) {
            return distance;
        }
        distance = distance + scene_distance * 0.82;
        if (distance > 7.0) {
            break;
        }
    }
    return -1.0;
}

fn grid_mask(value: f32) -> f32 {
    let line = abs(fract(value) - 0.5);
    return 1.0 - smoothstep(0.465, 0.500, line);
}

fn floor_grid(ray_origin: vec3<f32>, ray_direction: vec3<f32>, color: vec3<f32>) -> vec3<f32> {
    if (abs(ray_direction.y) < 0.001) {
        return color;
    }

    let plane_y = -0.92;
    let t = (plane_y - ray_origin.y) / ray_direction.y;
    if (t <= 0.0 || t > 7.0) {
        return color;
    }

    let hit = ray_origin + ray_direction * t;
    let major = max(grid_mask(hit.x * 0.62), grid_mask(hit.z * 0.62));
    let minor = max(grid_mask(hit.x * 1.86), grid_mask(hit.z * 1.86)) * 0.34;
    let fade = saturate(1.0 - length(hit.xz) * 0.135);
    let grid = saturate(max(major, minor) * fade);
    return blend(color, vec3<f32>(0.15, 0.40, 0.72), grid * 0.46);
}

fn cube_edge(local_point: vec3<f32>) -> f32 {
    let distance_to_faces = abs(abs(local_point) - vec3<f32>(0.68, 0.68, 0.68));
    let smallest = min(distance_to_faces.x, min(distance_to_faces.y, distance_to_faces.z));
    let largest = max(distance_to_faces.x, max(distance_to_faces.y, distance_to_faces.z));
    let second = distance_to_faces.x + distance_to_faces.y + distance_to_faces.z - smallest - largest;
    return 1.0 - smoothstep(0.025, 0.070, second);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = clamp(input.uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    var point = uv * 2.0 - vec2<f32>(1.0, 1.0);
    point.x = point.x * 1.34;

    let ray_origin = vec3<f32>(0.0, 0.0, 3.2);
    let ray_direction = normalize(vec3<f32>(point.x, point.y * 0.82, -2.18));

    let vignette = saturate(1.08 - dot(point, point) * 0.26);
    let glow = 1.0 / (1.0 + dot(point, point) * 6.0);
    var color = ramp(vec3<f32>(0.030, 0.037, 0.052), vec3<f32>(0.055, 0.090, 0.140), uv.y);
    color = color * (0.72 + vignette * 0.28) + vec3<f32>(0.030, 0.070, 0.130) * glow;
    color = floor_grid(ray_origin, ray_direction, color);

    let hit_distance = trace_cube(ray_origin, ray_direction);
    if (hit_distance > 0.0) {
        let hit = ray_origin + ray_direction * hit_distance;
        let normal = cube_normal(hit);
        let local_hit = cube_space(hit);
        let light = normalize(vec3<f32>(-0.45, 0.82, 0.62));
        let fill = ramp(vec3<f32>(0.10, 0.26, 0.78), vec3<f32>(0.72, 0.46, 1.00), normal.y * 0.42 + 0.58);
        let diffuse = saturate(dot(normal, light));
        let facing = saturate(dot(normal, -ray_direction));
        let rim = pow(1.0 - facing, 2.4);
        let edge = cube_edge(local_hit);
        color = fill * (0.25 + diffuse * 0.82);
        color = color + vec3<f32>(0.18, 0.58, 1.00) * rim * 0.52;
        color = blend(color, vec3<f32>(0.88, 0.96, 1.00), edge * 0.72);
    }

    return vec4<f32>(clamp(color, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0)), 1.0);
}
