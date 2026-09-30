use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use glyphon::{
    Attrs as GlyphAttrs, Buffer as GlyphBuffer, Cache as GlyphCache, Color as GlyphColor,
    ColorMode as GlyphColorMode, Family as GlyphFamily, FontSystem as GlyphFontSystem,
    Metrics as GlyphMetrics, PrepareError as GlyphPrepareError, RenderError as GlyphRenderError,
    Resolution as GlyphResolution, Shaping as GlyphShaping, Stretch as GlyphStretch,
    Style as GlyphFontStyle, SwashCache as GlyphSwashCache, TextArea as GlyphTextArea,
    TextAtlas as GlyphTextAtlas, TextBounds as GlyphTextBounds, TextRenderer as GlyphTextRenderer,
    Viewport as GlyphViewport, Weight as GlyphWeight, Wrap as GlyphWrap,
};
use pollster::block_on;
use web_time::Instant;
use wgpu::util::DeviceExt;
use wgpu::{
    BufferUsages, Extent3d, Origin3d, TexelCopyBufferInfo, TexelCopyBufferLayout,
    TexelCopyTextureInfo, TextureFormat, COPY_BYTES_PER_ROW_ALIGNMENT,
};

use crate::accessibility::AccessibilityCapabilities;
use crate::compositor::{CompositorClip, CompositorFilterKind, CompositorMask, MaskMode};
use crate::fonts::FontLibrary;
use crate::paint::tessellate_polygon_points;
use crate::platform::{
    BackendAdapterKind, BackendCapabilities, LayerCapabilities, PixelSize,
    PlatformServiceCapabilities, RenderingCapabilities, ResourceCapabilities,
};
use crate::renderer::{
    PixelRect, RenderError, RenderFrameOutput, RenderFrameRequest, RenderTarget, RenderTargetKind,
    RenderedImage, RendererAdapter, ResourceFormat, ResourceResolver, ResourceUpdate,
};
use crate::{
    BuiltInIcon, ColorRgba, CornerRadii, FontFamily, FontStretch, FontStyle, FrameTiming,
    ImageAlignment, ImageFit, PaintBrush, PaintCompositorLayer, PaintEffectKind, PaintKind,
    PaintTransform, ShaderEffect, StrokeStyle, TextHorizontalAlign, TextOverflow, TextStyle,
    TextVerticalAlign, TextWrap, UiPoint, UiRect, UiSize,
};

mod sdf;
#[cfg(test)]
mod sdf_tests;
use sdf::{SdfGradientStop, SdfInstance, SdfPipelineKind};

const OFFSCREEN_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
const GLYPH_TEXT_CHUNK_SIZE: usize = 8;
const MAX_CACHED_CANVAS_PIPELINES: usize = 128;
const GPU_TIMESTAMP_QUERY_BYTES: u64 = 16;
const MISSING_IMAGE_CHECKER_SIZE: f32 = 8.0;
const MISSING_IMAGE_DARK: ColorRgba = ColorRgba::new(16, 0, 28, 255);
const MISSING_IMAGE_PURPLE: ColorRgba = ColorRgba::new(210, 0, 255, 255);

const WGPU_UI_SHADER: &str = concat!(
    include_str!("wgpu_renderer/shape.wgsl"),
    r#"
struct Scene {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

struct TriangleInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct TexturedRectInput {
    @location(0) rect: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) tint: vec4<f32>,
};

struct CompositedRectInput {
    @location(0) rect: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) clip_rect: vec4<f32>,
    @location(4) mask_rect: vec4<f32>,
    @location(5) params: vec4<f32>,
    @location(6) filter_params: vec4<f32>,
    @location(7) texel_size: vec2<f32>,
    @location(8) shader_params: vec4<f32>,
    @location(9) shader_color: vec4<f32>,
    @location(10) clip_radii: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct TexturedVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

struct CompositedRectOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) world_position: vec2<f32>,
    @location(3) clip_rect: vec4<f32>,
    @location(4) mask_rect: vec4<f32>,
    @location(5) params: vec4<f32>,
    @location(6) filter_params: vec4<f32>,
    @location(7) texel_size: vec2<f32>,
    @location(8) shader_params: vec4<f32>,
    @location(9) shader_color: vec4<f32>,
    @location(10) clip_radii: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> scene: Scene;

@group(1) @binding(0)
var image_texture: texture_2d<f32>;

@group(1) @binding(1)
var image_sampler: sampler;

@vertex
fn vs_triangle(input: TriangleInput) -> VertexOutput {
    var output: VertexOutput;
    let x = input.position.x / scene.viewport.x * 2.0 - 1.0;
    let y = 1.0 - input.position.y / scene.viewport.y * 2.0;
    output.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@vertex
fn vs_textured_rect(@builtin(vertex_index) vertex_index: u32, input: TexturedRectInput) -> TexturedVertexOutput {
    let unit_positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0)
    );
    let unit = unit_positions[vertex_index];
    let position = input.rect.xy + unit * input.rect.zw;
    var output: TexturedVertexOutput;
    let x = position.x / scene.viewport.x * 2.0 - 1.0;
    let y = 1.0 - position.y / scene.viewport.y * 2.0;
    output.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = input.uv.xy + unit * input.uv.zw;
    output.tint = input.tint;
    return output;
}

@vertex
fn vs_composited_rect(@builtin(vertex_index) vertex_index: u32, input: CompositedRectInput) -> CompositedRectOutput {
    let unit_positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0)
    );
    let unit = unit_positions[vertex_index];
    let position = input.rect.xy + unit * input.rect.zw;
    var output: CompositedRectOutput;
    let x = position.x / scene.viewport.x * 2.0 - 1.0;
    let y = 1.0 - position.y / scene.viewport.y * 2.0;
    output.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = input.uv.xy + unit * input.uv.zw;
    output.tint = input.tint;
    output.world_position = position;
    output.clip_rect = input.clip_rect;
    output.mask_rect = input.mask_rect;
    output.params = input.params;
    output.filter_params = input.filter_params;
    output.texel_size = input.texel_size;
    output.shader_params = input.shader_params;
    output.shader_color = input.shader_color;
    output.clip_radii = input.clip_radii;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}

fn srgb_to_linear_channel(value: f32) -> f32 {
    if value <= 0.04045 {
        return value / 12.92;
    }
    return pow((value + 0.055) / 1.055, 2.4);
}

fn srgb_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        srgb_to_linear_channel(color.r),
        srgb_to_linear_channel(color.g),
        srgb_to_linear_channel(color.b)
    );
}

fn srgb_to_linear_rgba(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(srgb_to_linear_rgb(color.rgb), color.a);
}

@fragment
fn fs_main_srgb(input: VertexOutput) -> @location(0) vec4<f32> {
    return srgb_to_linear_rgba(input.color);
}

@fragment
fn fs_textured(input: TexturedVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(image_texture, image_sampler, input.uv) * input.tint;
}

@fragment
fn fs_textured_srgb(input: TexturedVertexOutput) -> @location(0) vec4<f32> {
    return srgb_to_linear_rgba(textureSample(image_texture, image_sampler, input.uv) * input.tint);
}


fn rounded_rect_alpha_with_radii(point: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>) -> f32 {
    if point.x < rect.x || point.y < rect.y || point.x > rect.x + rect.z || point.y > rect.y + rect.w {
        return 0.0;
    }
    let distance = rect_distance(point, rect, normalize_radii(radii, rect.zw));
    return 1.0 - smoothstep(-0.75, 0.75, distance);
}




fn sample_composited_layer(uv: vec2<f32>, texel_size: vec2<f32>, blur_radius: f32) -> vec4<f32> {
    if blur_radius <= 0.5 {
        return textureSampleLevel(image_texture, image_sampler, uv, 0.0);
    }
    let offset = texel_size * min(blur_radius, 8.0);
    var color = textureSampleLevel(image_texture, image_sampler, uv, 0.0) * 4.0;
    color = color + textureSampleLevel(image_texture, image_sampler, uv + vec2<f32>(offset.x, 0.0), 0.0);
    color = color + textureSampleLevel(image_texture, image_sampler, uv - vec2<f32>(offset.x, 0.0), 0.0);
    color = color + textureSampleLevel(image_texture, image_sampler, uv + vec2<f32>(0.0, offset.y), 0.0);
    color = color + textureSampleLevel(image_texture, image_sampler, uv - vec2<f32>(0.0, offset.y), 0.0);
    color = color + textureSampleLevel(image_texture, image_sampler, uv + offset, 0.0);
    color = color + textureSampleLevel(image_texture, image_sampler, uv - offset, 0.0);
    color = color + textureSampleLevel(image_texture, image_sampler, uv + vec2<f32>(offset.x, -offset.y), 0.0);
    color = color + textureSampleLevel(image_texture, image_sampler, uv + vec2<f32>(-offset.x, offset.y), 0.0);
    return color / 12.0;
}

fn shader_neighbor_alpha(uv: vec2<f32>, texel_size: vec2<f32>, radius: f32) -> f32 {
    let offset = texel_size * clamp(radius, 1.0, 24.0);
    var alpha = 0.0;
    alpha = max(alpha, textureSampleLevel(image_texture, image_sampler, uv + vec2<f32>(offset.x, 0.0), 0.0).a);
    alpha = max(alpha, textureSampleLevel(image_texture, image_sampler, uv - vec2<f32>(offset.x, 0.0), 0.0).a);
    alpha = max(alpha, textureSampleLevel(image_texture, image_sampler, uv + vec2<f32>(0.0, offset.y), 0.0).a);
    alpha = max(alpha, textureSampleLevel(image_texture, image_sampler, uv - vec2<f32>(0.0, offset.y), 0.0).a);
    alpha = max(alpha, textureSampleLevel(image_texture, image_sampler, uv + offset, 0.0).a);
    alpha = max(alpha, textureSampleLevel(image_texture, image_sampler, uv - offset, 0.0).a);
    alpha = max(alpha, textureSampleLevel(image_texture, image_sampler, uv + vec2<f32>(offset.x, -offset.y), 0.0).a);
    alpha = max(alpha, textureSampleLevel(image_texture, image_sampler, uv + vec2<f32>(-offset.x, offset.y), 0.0).a);
    return alpha;
}

fn shader_grid_line(value: f32) -> f32 {
    let cell = abs(fract(value) - 0.5);
    return 1.0 - smoothstep(0.46, 0.50, cell);
}

fn apply_element_shader(input: CompositedRectOutput, rgb: vec3<f32>, alpha: f32) -> vec4<f32> {
    let mode = input.shader_params.x;
    let amount = max(input.shader_params.y, 0.0);
    var out_rgb = rgb;
    var out_alpha = alpha;
    if mode > 0.5 && mode < 1.5 {
        out_rgb = mix(out_rgb, input.shader_color.rgb, clamp(amount, 0.0, 1.0));
    } else if mode > 1.5 && mode < 2.5 {
        let phase = fract(input.shader_params.z);
        let width = clamp(input.shader_params.w, 0.02, 0.45);
        let band_position = fract(input.uv.x * 0.78 + input.uv.y * 0.62);
        let distance = abs(band_position - phase);
        let wrapped_distance = min(distance, 1.0 - distance);
        let highlight = (1.0 - smoothstep(width * 0.35, width, wrapped_distance)) * amount * out_alpha;
        out_rgb = clamp(out_rgb + input.shader_color.rgb * highlight, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0));
    } else if mode > 2.5 && mode < 3.5 {
        let radius = max(input.shader_params.z, 1.0);
        let neighbor_alpha = shader_neighbor_alpha(input.uv, input.texel_size, radius);
        let glow_alpha = clamp((neighbor_alpha - out_alpha) * amount, 0.0, 1.0);
        if glow_alpha > out_alpha {
            out_rgb = input.shader_color.rgb;
            out_alpha = glow_alpha;
        } else if glow_alpha > 0.0001 {
            out_rgb = mix(input.shader_color.rgb, out_rgb, out_alpha);
            out_alpha = max(out_alpha, glow_alpha);
        }
    } else if mode > 3.5 && mode < 4.5 {
        let phase = input.shader_params.z;
        let scale = max(input.shader_params.w, 1.0);
        let p = input.uv * 2.0 - vec2<f32>(1.0, 1.0);
        let wave = (
            sin(p.x * scale + phase * 6.28318)
            + sin(p.y * (scale * 1.17) - phase * 5.49779)
            + sin(length(p) * (scale * 1.72) - phase * 8.16814)
        ) / 3.0;
        let mask = smoothstep(-0.45, 0.85, wave);
        out_rgb = mix(out_rgb, input.shader_color.rgb, mask * amount * out_alpha);
    } else if mode > 4.5 && mode < 5.5 {
        let phase = input.shader_params.z;
        let scale = max(input.shader_params.w, 1.0);
        let p = input.uv - vec2<f32>(0.5, 0.5);
        let ring = 0.5 + 0.5 * cos((length(p) * scale - phase * 2.0) * 6.28318);
        let fade = 1.0 - smoothstep(0.12, 0.72, length(p));
        out_rgb = mix(out_rgb, input.shader_color.rgb, ring * fade * amount * out_alpha);
    } else if mode > 5.5 && mode < 6.5 {
        let phase = input.shader_params.z;
        let scale = max(input.shader_params.w, 1.0);
        let uv = input.uv + vec2<f32>(phase * 0.12, phase * -0.08);
        let major = max(shader_grid_line(uv.x * scale), shader_grid_line(uv.y * scale));
        let minor = max(shader_grid_line(uv.x * scale * 3.0), shader_grid_line(uv.y * scale * 3.0)) * 0.32;
        out_rgb = mix(out_rgb, input.shader_color.rgb, max(major, minor) * amount * out_alpha);
    }
    return vec4<f32>(out_rgb, out_alpha);
}

fn composited_color(input: CompositedRectOutput) -> vec4<f32> {
    let opacity = input.params.x;
    let clip_enabled = input.params.y;
    let mask_enabled = input.params.z;
    let blur_radius = input.params.w;
    let brightness = input.filter_params.x;
    let contrast = input.filter_params.y;
    let saturate = input.filter_params.z;

    let sampled = sample_composited_layer(input.uv, input.texel_size, blur_radius);
    var rgb = sampled.rgb;
    var alpha = sampled.a;
    if alpha > 0.0001 {
        rgb = rgb / alpha;
    }
    if clip_enabled > 0.5 {
        alpha = alpha * rounded_rect_alpha_with_radii(input.world_position, input.clip_rect, input.clip_radii);
    }
    if mask_enabled > 0.5 {
        let inside_mask =
            input.world_position.x >= input.mask_rect.x &&
            input.world_position.y >= input.mask_rect.y &&
            input.world_position.x <= input.mask_rect.x + input.mask_rect.z &&
            input.world_position.y <= input.mask_rect.y + input.mask_rect.w;
        if !inside_mask {
            alpha = 0.0;
        }
    }
    rgb = clamp((rgb * brightness - vec3<f32>(0.5, 0.5, 0.5)) * contrast + vec3<f32>(0.5, 0.5, 0.5), vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0));
    let luma = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    rgb = mix(vec3<f32>(luma, luma, luma), rgb, saturate);
    let shadered = apply_element_shader(input, rgb, alpha);
    return vec4<f32>(shadered.rgb * input.tint.rgb, shadered.a * input.tint.a * opacity);
}

@fragment
fn fs_composited(input: CompositedRectOutput) -> @location(0) vec4<f32> {
    return composited_color(input);
}

@fragment
fn fs_composited_srgb(input: CompositedRectOutput) -> @location(0) vec4<f32> {
    return srgb_to_linear_rgba(composited_color(input));
}

"#
);

#[derive(Debug)]
pub struct WgpuRenderer {
    context: Option<WgpuContext>,
    geometry: RenderGeometry,
    font_library: FontLibrary,
}

#[derive(Debug)]
pub struct WgpuCanvasContext<'a> {
    size: PixelSize,
    format: TextureFormat,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    texture: &'a wgpu::Texture,
    view: &'a wgpu::TextureView,
    empty_pipeline_layout: &'a wgpu::PipelineLayout,
    uniform_bind_group_layout: &'a wgpu::BindGroupLayout,
    uniform_pipeline_layout: &'a wgpu::PipelineLayout,
    pipeline_cache: &'a RefCell<WgpuCanvasPipelineCache>,
}

#[derive(Debug, Clone)]
pub struct WgpuCanvasRenderPass<'a> {
    pub label: Option<&'a str>,
    pub shader: Cow<'a, str>,
    pub vertex_entry_point: &'a str,
    pub fragment_entry_point: &'a str,
    pub clear_color: Option<ColorRgba>,
    pub constants: Vec<(&'a str, f64)>,
    pub uniforms: Option<Cow<'a, [u8]>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgpuRenderLoadOp {
    /// Clear the caller-owned attachment before drawing Operad UI.
    Clear(ColorRgba),
    /// Preserve the caller-owned attachment contents and draw Operad UI over it.
    Load,
}

/// A caller-owned WGPU render target for app-composed Operad UI.
///
/// The texture view should match the pixel size implied by the
/// `RenderFrameRequest` target and scale factor.
#[derive(Debug, Clone, Copy)]
pub struct WgpuRenderTargetView<'a> {
    pub view: &'a wgpu::TextureView,
    pub format: TextureFormat,
    pub load_op: Option<WgpuRenderLoadOp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WgpuGpuTimingToken {
    id: u64,
}

impl WgpuGpuTimingToken {
    pub const fn id(self) -> u64 {
        self.id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WgpuRenderFrameIntoViewOutput {
    pub frame: RenderFrameOutput,
    pub gpu_timing_token: Option<WgpuGpuTimingToken>,
}

impl<'a> WgpuRenderTargetView<'a> {
    pub const fn new(view: &'a wgpu::TextureView, format: TextureFormat) -> Self {
        Self {
            view,
            format,
            load_op: None,
        }
    }

    /// Override the request clear color for this render pass.
    pub const fn clear(mut self, color: ColorRgba) -> Self {
        self.load_op = Some(WgpuRenderLoadOp::Clear(color));
        self
    }

    /// Draw Operad UI over the existing attachment contents.
    pub const fn load(mut self) -> Self {
        self.load_op = Some(WgpuRenderLoadOp::Load);
        self
    }

    fn resolved_load_op(self, default_clear: ColorRgba) -> WgpuRenderLoadOp {
        self.load_op
            .unwrap_or(WgpuRenderLoadOp::Clear(default_clear))
    }
}

impl<'a> WgpuCanvasRenderPass<'a> {
    pub fn wgsl(shader: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label: Some("operad-wgpu-canvas-render-pass"),
            shader: shader.into(),
            vertex_entry_point: "vs_main",
            fragment_entry_point: "fs_main",
            clear_color: None,
            constants: Vec::new(),
            uniforms: None,
        }
    }

    pub const fn label(mut self, label: Option<&'a str>) -> Self {
        self.label = label;
        self
    }

    pub const fn vertex_entry_point(mut self, entry_point: &'a str) -> Self {
        self.vertex_entry_point = entry_point;
        self
    }

    pub const fn fragment_entry_point(mut self, entry_point: &'a str) -> Self {
        self.fragment_entry_point = entry_point;
        self
    }

    pub const fn clear_color(mut self, clear_color: Option<ColorRgba>) -> Self {
        self.clear_color = clear_color;
        self
    }

    pub fn constant(mut self, name: &'a str, value: f64) -> Self {
        self.constants.push((name, value));
        self
    }

    pub fn constants(mut self, constants: impl IntoIterator<Item = (&'a str, f64)>) -> Self {
        self.constants.extend(constants);
        self
    }

    pub fn uniform_bytes(mut self, uniforms: impl Into<Cow<'a, [u8]>>) -> Self {
        self.uniforms = Some(uniforms.into());
        self
    }
}

// Cache lookups borrow the descriptor; only new pipelines retain owned source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WgpuCanvasPipelineKey<'a> {
    shader: Cow<'a, str>,
    vertex_entry_point: Cow<'a, str>,
    fragment_entry_point: Cow<'a, str>,
    format: TextureFormat,
    uses_uniforms: bool,
    constants: WgpuCanvasPipelineConstants<'a>,
}

#[derive(Debug, Clone)]
enum WgpuCanvasPipelineConstants<'a> {
    Borrowed(&'a [(&'a str, f64)]),
    Owned(Vec<WgpuCanvasPipelineConstant>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WgpuCanvasPipelineConstant {
    name: String,
    value_bits: u64,
}

impl<'a> WgpuCanvasPipelineConstants<'a> {
    fn len(&self) -> usize {
        match self {
            Self::Borrowed(constants) => constants.len(),
            Self::Owned(constants) => constants.len(),
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&str, u64)> + use<'_, 'a> {
        (0..self.len()).map(|index| match self {
            Self::Borrowed(constants) => {
                let (name, value) = constants[index];
                (name, value.to_bits())
            }
            Self::Owned(constants) => {
                let constant = &constants[index];
                (constant.name.as_str(), constant.value_bits)
            }
        })
    }

    fn into_owned(self) -> WgpuCanvasPipelineConstants<'static> {
        WgpuCanvasPipelineConstants::Owned(match self {
            Self::Borrowed(constants) => constants
                .iter()
                .map(|(name, value)| WgpuCanvasPipelineConstant {
                    name: (*name).to_string(),
                    value_bits: value.to_bits(),
                })
                .collect(),
            Self::Owned(constants) => constants,
        })
    }
}

impl PartialEq for WgpuCanvasPipelineConstants<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl Eq for WgpuCanvasPipelineConstants<'_> {}

impl Hash for WgpuCanvasPipelineConstants<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        for (name, value_bits) in self.iter() {
            name.hash(state);
            value_bits.hash(state);
        }
    }
}

impl<'a> WgpuCanvasPipelineKey<'a> {
    fn new(pass: &'a WgpuCanvasRenderPass<'_>, format: TextureFormat) -> Self {
        Self {
            shader: Cow::Borrowed(pass.shader.as_ref()),
            vertex_entry_point: Cow::Borrowed(pass.vertex_entry_point),
            fragment_entry_point: Cow::Borrowed(pass.fragment_entry_point),
            format,
            uses_uniforms: pass.uniforms.is_some(),
            constants: WgpuCanvasPipelineConstants::Borrowed(&pass.constants),
        }
    }

    fn into_owned(self) -> WgpuCanvasPipelineKey<'static> {
        WgpuCanvasPipelineKey {
            shader: Cow::Owned(self.shader.into_owned()),
            vertex_entry_point: Cow::Owned(self.vertex_entry_point.into_owned()),
            fragment_entry_point: Cow::Owned(self.fragment_entry_point.into_owned()),
            format: self.format,
            uses_uniforms: self.uses_uniforms,
            constants: self.constants.into_owned(),
        }
    }
}

#[derive(Debug)]
struct CachedCanvasPipeline {
    pipeline: wgpu::RenderPipeline,
    // Shared lookup lets a temporary descriptor borrow an owned cache key without
    // allocating shader source or constants on every draw.
    last_used: Cell<u64>,
}

/// Bound retained variants even for callers drawing outside the UI frame loop.
/// Eviction only drops the cache's handle; submitted commands keep their resources.
#[derive(Debug, Default)]
struct WgpuCanvasPipelineCache {
    entries: HashMap<WgpuCanvasPipelineKey<'static>, CachedCanvasPipeline>,
    access: u64,
}

impl WgpuCanvasPipelineCache {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn next_access(&mut self) -> u64 {
        if self.access == u64::MAX {
            // Avoid ambiguous ordering after counter wrap. Entries are recreatable.
            self.entries.clear();
            self.access = 0;
        }
        self.access += 1;
        self.access
    }

    fn get(&mut self, key: &WgpuCanvasPipelineKey<'_>) -> Option<wgpu::RenderPipeline> {
        let access = self.next_access();
        let cached = self.entries.get(key)?;
        cached.last_used.set(access);
        Some(cached.pipeline.clone())
    }

    fn insert(&mut self, key: WgpuCanvasPipelineKey<'static>, pipeline: wgpu::RenderPipeline) {
        let access = self.next_access();
        if self.entries.len() >= MAX_CACHED_CANVAS_PIPELINES {
            let oldest = self
                .entries
                .values()
                .map(|cached| cached.last_used.get())
                .min();
            // Access stamps are unique. Retaining by stamp avoids copying a shader
            // just to remove its key. Scanning this small map only happens on misses.
            self.entries
                .retain(|_, cached| Some(cached.last_used.get()) != oldest);
        }
        self.entries.insert(
            key,
            CachedCanvasPipeline {
                pipeline,
                last_used: Cell::new(access),
            },
        );
    }
}

impl<'a> WgpuCanvasContext<'a> {
    pub const fn size(&self) -> PixelSize {
        self.size
    }

    pub const fn format(&self) -> TextureFormat {
        self.format
    }

    pub const fn device(&self) -> &'a wgpu::Device {
        self.device
    }

    pub const fn queue(&self) -> &'a wgpu::Queue {
        self.queue
    }

    pub const fn texture(&self) -> &'a wgpu::Texture {
        self.texture
    }

    pub const fn view(&self) -> &'a wgpu::TextureView {
        self.view
    }

    pub fn create_command_encoder(&self, label: Option<&str>) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label })
    }

    pub fn begin_render_pass<'pass>(
        &'pass self,
        encoder: &'pass mut wgpu::CommandEncoder,
        clear_color: Option<ColorRgba>,
    ) -> wgpu::RenderPass<'pass> {
        let load = clear_color
            .map(|color| wgpu::LoadOp::Clear(wgpu_color_for_format(color, self.format)))
            .unwrap_or(wgpu::LoadOp::Load);
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("operad-wgpu-canvas-render-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        })
    }

    pub fn render_pass(&self, descriptor: WgpuCanvasRenderPass<'_>) -> Result<(), RenderError> {
        let mut encoder = self.create_command_encoder(descriptor.label);
        self.record_render_pass(&mut encoder, descriptor)?;
        self.submit(encoder);
        Ok(())
    }

    fn record_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        descriptor: WgpuCanvasRenderPass<'_>,
    ) -> Result<(), RenderError> {
        let label = descriptor.label;
        let pipeline_key = WgpuCanvasPipelineKey::new(&descriptor, self.format);
        let compilation_options = wgpu::PipelineCompilationOptions {
            constants: descriptor.constants.as_slice(),
            ..Default::default()
        };
        let cached_pipeline = self.pipeline_cache.borrow_mut().get(&pipeline_key);
        let pipeline = if let Some(pipeline) = cached_pipeline {
            pipeline
        } else {
            let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let shader = self
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label,
                    source: wgpu::ShaderSource::Wgsl(descriptor.shader.clone()),
                });
            let pipeline_layout = if descriptor.uniforms.is_some() {
                self.uniform_pipeline_layout
            } else {
                self.empty_pipeline_layout
            };
            let pipeline = self
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label,
                    layout: Some(pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some(descriptor.vertex_entry_point),
                        buffers: &[],
                        compilation_options: compilation_options.clone(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some(descriptor.fragment_entry_point),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: self.format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options,
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                });
            if let Some(error) = block_on(error_scope.pop()) {
                return Err(RenderError::Backend(format!(
                    "canvas shader validation failed: {error}"
                )));
            }
            self.pipeline_cache
                .borrow_mut()
                .insert(pipeline_key.into_owned(), pipeline.clone());
            pipeline
        };
        let uniform_bind_group = descriptor.uniforms.as_deref().map(|uniforms| {
            let uniform_bytes = padded_uniform_bytes(uniforms);
            let uniform_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("operad-wgpu-canvas-uniforms"),
                size: u64::try_from(uniform_bytes.len()).unwrap_or(16),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&uniform_buffer, 0, &uniform_bytes);
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("operad-wgpu-canvas-uniform-bind-group"),
                layout: self.uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            })
        });
        {
            let mut pass = self.begin_render_pass(encoder, descriptor.clear_color);
            pass.set_pipeline(&pipeline);
            if let Some(bind_group) = &uniform_bind_group {
                pass.set_bind_group(0, bind_group, &[]);
            }
            pass.draw(0..3, 0..1);
        }
        Ok(())
    }

    pub fn submit(&self, encoder: wgpu::CommandEncoder) {
        self.queue.submit(Some(encoder.finish()));
    }

    pub fn clear(&self, color: ColorRgba) {
        let mut encoder = self.create_command_encoder(Some("operad-wgpu-canvas-clear"));
        {
            let _pass = self.begin_render_pass(&mut encoder, Some(color));
        }
        self.submit(encoder);
    }
}

#[derive(Debug)]
enum GlyphPreparationError {
    Prepare(GlyphPrepareError),
    Render(RenderError),
}

impl From<RenderError> for GlyphPreparationError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

impl GlyphPreparationError {
    fn into_render_error(self) -> RenderError {
        match self {
            Self::Prepare(error) => glyph_prepare_error(error),
            Self::Render(error) => error,
        }
    }
}

struct WgpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    limits: wgpu::Limits,
    pipeline_layout: wgpu::PipelineLayout,
    texture_pipeline_layout: wgpu::PipelineLayout,
    canvas_empty_pipeline_layout: wgpu::PipelineLayout,
    canvas_uniform_bind_group_layout: wgpu::BindGroupLayout,
    canvas_uniform_pipeline_layout: wgpu::PipelineLayout,
    canvas_pipeline_cache: RefCell<WgpuCanvasPipelineCache>,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_sampler: wgpu::Sampler,
    shader: wgpu::ShaderModule,
    triangle_pipelines: HashMap<TextureFormat, wgpu::RenderPipeline>,
    textured_rect_pipelines: HashMap<TextureFormat, wgpu::RenderPipeline>,
    composited_rect_pipelines: HashMap<TextureFormat, wgpu::RenderPipeline>,
    sdf_shader: wgpu::ShaderModule,
    sdf_pipeline_layout: wgpu::PipelineLayout,
    sdf_gradient_bind_group_layout: wgpu::BindGroupLayout,
    sdf_pipelines: HashMap<(TextureFormat, SdfPipelineKind), wgpu::RenderPipeline>,
    scene_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_capacity: u64,
    textured_rect_buffer: Option<wgpu::Buffer>,
    textured_rect_capacity: u64,
    composited_rect_buffer: Option<wgpu::Buffer>,
    composited_rect_capacity: u64,
    sdf_instance_buffer: Option<wgpu::Buffer>,
    sdf_instance_capacity: u64,
    sdf_gradient_buffer: Option<wgpu::Buffer>,
    sdf_gradient_capacity: u64,
    sdf_gradient_bind_group: Option<wgpu::BindGroup>,
    textures: HashMap<String, WgpuTextureResource>,
    layer_textures: Vec<WgpuTextureResource>,
    font_system: GlyphFontSystem,
    swash_cache: GlyphSwashCache,
    glyph_cache: GlyphCache,
    glyph_viewport: GlyphViewport,
    glyph_atlas: Option<GlyphTextAtlas>,
    glyph_format: Option<TextureFormat>,
    glyph_buffer_cache: HashMap<TextBufferKey, CachedGlyphBuffer>,
    glyph_scratch_buffer_cache: HashMap<TextBufferKey, CachedGlyphBuffer>,
    glyph_scene_renderer: Option<GlyphTextRenderer>,
    glyph_scene_key: Vec<TextRenderKey>,
    glyph_scene_active: bool,
    glyph_chunk_cache: HashMap<TextChunkKey, CachedGlyphChunk>,
    glyph_chunk_order: Vec<TextChunkKey>,
    glyph_chunks_active: bool,
    glyph_generation: u64,
    gpu_timer: Option<GpuTimer>,
    discard_target: Option<CachedTarget>,
}

impl std::fmt::Debug for WgpuContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WgpuContext")
            .field("triangle_pipelines", &self.triangle_pipelines.len())
            .field(
                "textured_rect_pipelines",
                &self.textured_rect_pipelines.len(),
            )
            .field(
                "composited_rect_pipelines",
                &self.composited_rect_pipelines.len(),
            )
            .field(
                "canvas_pipeline_cache",
                &self.canvas_pipeline_cache.borrow().len(),
            )
            .field("sdf_pipelines", &self.sdf_pipelines.len())
            .field("textures", &self.textures.len())
            .field("layer_textures", &self.layer_textures.len())
            .field("glyph_format", &self.glyph_format)
            .field("glyph_buffer_cache", &self.glyph_buffer_cache.len())
            .field(
                "glyph_scratch_buffer_cache",
                &self.glyph_scratch_buffer_cache.len(),
            )
            .field("glyph_scene_key", &self.glyph_scene_key.len())
            .field("glyph_chunk_cache", &self.glyph_chunk_cache.len())
            .field("glyph_chunk_order", &self.glyph_chunk_order.len())
            .field("gpu_timer", &self.gpu_timer.is_some())
            .field("discard_target", &self.discard_target)
            .finish_non_exhaustive()
    }
}

impl WgpuContext {
    fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        font_library: &FontLibrary,
    ) -> Result<Self, RenderError> {
        let limits = device.limits();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("operad-wgpu-ui-shader"),
            source: wgpu::ShaderSource::Wgsl(WGPU_UI_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("operad-wgpu-ui-bind-group-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("operad-wgpu-ui-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let sdf_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("operad-sdf-shader"),
            source: wgpu::ShaderSource::Wgsl(sdf::SHADER.into()),
        });
        let sdf_gradient_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("operad-sdf-gradient-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            mem::size_of::<SdfGradientStop>() as u64
                        ),
                    },
                    count: None,
                }],
            });
        let sdf_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("operad-sdf-pipeline-layout"),
            bind_group_layouts: &[
                Some(&bind_group_layout),
                Some(&sdf_gradient_bind_group_layout),
            ],
            immediate_size: 0,
        });
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("operad-wgpu-texture-bind-group-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let texture_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("operad-wgpu-textured-ui-pipeline-layout"),
                bind_group_layouts: &[Some(&bind_group_layout), Some(&texture_bind_group_layout)],
                immediate_size: 0,
            });
        let canvas_empty_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("operad-wgpu-canvas-empty-pipeline-layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
        let canvas_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("operad-wgpu-canvas-uniform-bind-group-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let canvas_uniform_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("operad-wgpu-canvas-uniform-pipeline-layout"),
                bind_group_layouts: &[Some(&canvas_uniform_bind_group_layout)],
                immediate_size: 0,
            });
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("operad-wgpu-texture-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let scene_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("operad-wgpu-scene-uniform"),
            size: 16,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("operad-wgpu-ui-bind-group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scene_buffer.as_entire_binding(),
            }],
        });
        let textures = HashMap::new();
        let glyph_cache = GlyphCache::new(&device);
        let glyph_viewport = GlyphViewport::new(&device, &glyph_cache);
        let gpu_timer = GpuTimer::new_if_supported(&device);

        Ok(Self {
            device,
            queue,
            limits,
            pipeline_layout,
            texture_pipeline_layout,
            canvas_empty_pipeline_layout,
            canvas_uniform_bind_group_layout,
            canvas_uniform_pipeline_layout,
            canvas_pipeline_cache: RefCell::default(),
            texture_bind_group_layout,
            texture_sampler,
            shader,
            triangle_pipelines: HashMap::new(),
            textured_rect_pipelines: HashMap::new(),
            composited_rect_pipelines: HashMap::new(),
            sdf_shader,
            sdf_pipeline_layout,
            sdf_gradient_bind_group_layout,
            sdf_pipelines: HashMap::new(),
            scene_buffer,
            scene_bind_group,
            vertex_buffer: None,
            vertex_capacity: 0,
            textured_rect_buffer: None,
            textured_rect_capacity: 0,
            composited_rect_buffer: None,
            composited_rect_capacity: 0,
            sdf_instance_buffer: None,
            sdf_instance_capacity: 0,
            sdf_gradient_buffer: None,
            sdf_gradient_capacity: 0,
            sdf_gradient_bind_group: None,
            textures,
            layer_textures: Vec::new(),
            font_system: glyph_font_system(font_library),
            swash_cache: GlyphSwashCache::new(),
            glyph_cache,
            glyph_viewport,
            glyph_atlas: None,
            glyph_format: None,
            glyph_buffer_cache: HashMap::new(),
            glyph_scratch_buffer_cache: HashMap::new(),
            glyph_scene_renderer: None,
            glyph_scene_key: Vec::new(),
            glyph_scene_active: false,
            glyph_chunk_cache: HashMap::new(),
            glyph_chunk_order: Vec::new(),
            glyph_chunks_active: false,
            glyph_generation: 0,
            gpu_timer,
            discard_target: None,
        })
    }

    fn create_texture_2d(
        &self,
        label: &str,
        size: PixelSize,
        format: TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Result<wgpu::Texture, RenderError> {
        validate_texture_size(size, self.limits.max_texture_dimension_2d, label)?;
        Ok(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        }))
    }

    fn triangle_pipeline(&mut self, format: TextureFormat) -> &wgpu::RenderPipeline {
        let device = &self.device;
        let shader = &self.shader;
        let layout = &self.pipeline_layout;
        self.triangle_pipelines.entry(format).or_insert_with(|| {
            Self::create_pipeline_with(
                device,
                shader,
                format,
                "vs_triangle",
                main_fragment_entry_point(format),
                &[GpuVertex::layout()],
                layout,
            )
        })
    }

    fn textured_rect_pipeline(&mut self, format: TextureFormat) -> &wgpu::RenderPipeline {
        let device = &self.device;
        let shader = &self.shader;
        let layout = &self.texture_pipeline_layout;
        self.textured_rect_pipelines
            .entry(format)
            .or_insert_with(|| {
                Self::create_pipeline_with(
                    device,
                    shader,
                    format,
                    "vs_textured_rect",
                    textured_fragment_entry_point(format),
                    &[GpuTexturedRectInstance::layout()],
                    layout,
                )
            })
    }

    fn composited_rect_pipeline(&mut self, format: TextureFormat) -> &wgpu::RenderPipeline {
        let device = &self.device;
        let shader = &self.shader;
        let layout = &self.texture_pipeline_layout;
        self.composited_rect_pipelines
            .entry(format)
            .or_insert_with(|| {
                Self::create_pipeline_with(
                    device,
                    shader,
                    format,
                    "vs_composited_rect",
                    composited_fragment_entry_point(format),
                    &[GpuCompositedRectInstance::layout()],
                    layout,
                )
            })
    }

    fn sdf_pipeline(
        &mut self,
        format: TextureFormat,
        kind: SdfPipelineKind,
    ) -> &wgpu::RenderPipeline {
        let device = &self.device;
        let shader = &self.sdf_shader;
        let layout = &self.sdf_pipeline_layout;
        self.sdf_pipelines.entry((format, kind)).or_insert_with(|| {
            Self::create_pipeline_with(
                device,
                shader,
                format,
                "vs_sdf",
                kind.fragment_entry_point(format.is_srgb()),
                &[SdfInstance::layout()],
                layout,
            )
        })
    }

    fn sdf_buffers_for(
        &mut self,
        instances: &[SdfInstance],
        stops: &[SdfGradientStop],
        mut encoder: Option<&mut wgpu::CommandEncoder>,
    ) -> Result<Option<(wgpu::Buffer, wgpu::BindGroup)>, RenderError> {
        if instances.is_empty() {
            return Ok(None);
        }
        let dummy = [SdfGradientStop::default()];
        let stops = if stops.is_empty() { &dummy[..] } else { stops };
        let bytes = sdf::instance_bytes(instances);
        let gradient_bytes = sdf::gradient_bytes(stops);
        let instance_limit = self.limits.max_buffer_size;
        let gradient_limit =
            instance_limit.min(u64::from(self.limits.max_storage_buffer_binding_size));
        // Validate both requests before allocating either buffer.
        sdf::buffer_capacity(bytes.len() as u64, instance_limit)?;
        sdf::buffer_capacity(gradient_bytes.len() as u64, gradient_limit)?;
        if self.sdf_instance_capacity < bytes.len() as u64 {
            let capacity = sdf::buffer_capacity(bytes.len() as u64, instance_limit)?;
            self.sdf_instance_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("operad-sdf-instances"),
                size: capacity,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.sdf_instance_capacity = capacity;
        }
        if self.sdf_gradient_capacity < gradient_bytes.len() as u64 {
            let capacity = sdf::buffer_capacity(gradient_bytes.len() as u64, gradient_limit)?;
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("operad-sdf-gradient-stops"),
                size: capacity,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.sdf_gradient_bind_group =
                Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("operad-sdf-gradients"),
                    layout: &self.sdf_gradient_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                }));
            self.sdf_gradient_buffer = Some(buffer);
            self.sdf_gradient_capacity = capacity;
        }
        let buffer = self
            .sdf_instance_buffer
            .as_ref()
            .expect("allocated SDF instances");
        self.write_buffer(buffer, bytes, encoder.as_deref_mut());
        self.write_buffer(
            self.sdf_gradient_buffer
                .as_ref()
                .expect("allocated gradient stops"),
            gradient_bytes,
            encoder,
        );
        Ok(Some((
            buffer.clone(),
            self.sdf_gradient_bind_group
                .as_ref()
                .expect("allocated gradients binding")
                .clone(),
        )))
    }

    fn create_pipeline_with(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        format: TextureFormat,
        vertex_entry_point: &'static str,
        fragment_entry_point: &'static str,
        vertex_buffers: &[wgpu::VertexBufferLayout<'static>],
        layout: &wgpu::PipelineLayout,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("operad-wgpu-ui-pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some(vertex_entry_point),
                buffers: vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(fragment_entry_point),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn write_buffer(
        &self,
        buffer: &wgpu::Buffer,
        bytes: &[u8],
        encoder: Option<&mut wgpu::CommandEncoder>,
    ) {
        if let Some(encoder) = encoder {
            let upload = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("operad-wgpu-pass-upload"),
                    contents: bytes,
                    usage: BufferUsages::COPY_SRC,
                });
            encoder.copy_buffer_to_buffer(&upload, 0, buffer, 0, bytes.len() as u64);
        } else {
            self.queue.write_buffer(buffer, 0, bytes);
        }
    }

    fn write_scene_uniform(&self, size: PixelSize, encoder: Option<&mut wgpu::CommandEncoder>) {
        self.write_buffer(&self.scene_buffer, &pack_scene_uniform(size), encoder);
    }

    fn vertex_buffer_for(
        &mut self,
        vertices: &[GpuVertex],
        encoder: Option<&mut wgpu::CommandEncoder>,
    ) -> Option<wgpu::Buffer> {
        if vertices.is_empty() {
            return None;
        }

        let vertex_bytes = vertex_bytes(vertices);
        let required = u64::try_from(vertex_bytes.len()).ok()?;
        if self.vertex_capacity < required {
            let capacity = required.next_power_of_two();
            self.vertex_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("operad-wgpu-ui-vertices"),
                size: capacity,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.vertex_capacity = capacity;
        }

        let buffer = self.vertex_buffer.as_ref()?;
        self.write_buffer(buffer, vertex_bytes, encoder);
        Some(buffer.clone())
    }

    fn textured_rect_buffer_for(
        &mut self,
        rects: &[GpuTexturedRectInstance],
        encoder: Option<&mut wgpu::CommandEncoder>,
    ) -> Option<wgpu::Buffer> {
        if rects.is_empty() {
            return None;
        }

        let rect_bytes = textured_rect_instance_bytes(rects);
        let required = u64::try_from(rect_bytes.len()).ok()?;
        if self.textured_rect_capacity < required {
            let capacity = required.next_power_of_two();
            self.textured_rect_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("operad-wgpu-ui-textured-rect-instances"),
                size: capacity,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.textured_rect_capacity = capacity;
        }

        let buffer = self.textured_rect_buffer.as_ref()?;
        self.write_buffer(buffer, rect_bytes, encoder);
        Some(buffer.clone())
    }

    fn composited_rect_buffer_for(
        &mut self,
        rects: &[GpuCompositedRectInstance],
        encoder: Option<&mut wgpu::CommandEncoder>,
    ) -> Option<wgpu::Buffer> {
        if rects.is_empty() {
            return None;
        }

        let rect_bytes = composited_rect_instance_bytes(rects);
        let required = u64::try_from(rect_bytes.len()).ok()?;
        if self.composited_rect_capacity < required {
            let capacity = required.next_power_of_two();
            self.composited_rect_buffer =
                Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("operad-wgpu-ui-composited-rect-instances"),
                    size: capacity,
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            self.composited_rect_capacity = capacity;
        }

        let buffer = self.composited_rect_buffer.as_ref()?;
        self.write_buffer(buffer, rect_bytes, encoder);
        Some(buffer.clone())
    }

    fn begin_frame(&mut self) {
        // Only frame-owned compositor targets expire here. App images and canvas
        // buffers live independently of paint visibility and resource names.
        self.layer_textures.clear();
    }

    fn texture(&self, key: &WgpuTextureKey<'_>) -> Option<&WgpuTextureResource> {
        match key {
            WgpuTextureKey::Resource(key) => self.textures.get(key.as_ref()),
            WgpuTextureKey::Layer(index) => self.layer_textures.get(*index),
        }
    }

    fn insert_layer_texture(
        &mut self,
        size: PixelSize,
        texture: wgpu::Texture,
        view: wgpu::TextureView,
    ) -> WgpuTextureKey<'static> {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("operad-wgpu-layer-texture-bind-group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.texture_sampler),
                },
            ],
        });
        let index = self.layer_textures.len();
        self.layer_textures.push(WgpuTextureResource {
            size,
            source_format: ResourceFormat::Rgba8,
            texture,
            view,
            bind_group,
            render_attachment: true,
        });
        WgpuTextureKey::Layer(index)
    }

    fn canvas_context(
        &mut self,
        canvas: &crate::CanvasContent,
        size: PixelSize,
    ) -> Result<WgpuCanvasContext<'_>, RenderError> {
        if !canvas.context.kind.is_texture_backed() {
            return Err(RenderError::Backend(format!(
                "canvas {:?} does not have a texture-backed context",
                canvas.key
            )));
        }
        self.ensure_canvas_texture(canvas.surface_key(), size)?;
        let texture = self.textures.get(canvas.surface_key()).ok_or_else(|| {
            RenderError::Backend(format!(
                "wgpu canvas context target {:?} was not created",
                canvas.surface_key()
            ))
        })?;
        Ok(WgpuCanvasContext {
            size,
            format: OFFSCREEN_FORMAT,
            device: &self.device,
            queue: &self.queue,
            texture: &texture.texture,
            view: &texture.view,
            empty_pipeline_layout: &self.canvas_empty_pipeline_layout,
            uniform_bind_group_layout: &self.canvas_uniform_bind_group_layout,
            uniform_pipeline_layout: &self.canvas_uniform_pipeline_layout,
            pipeline_cache: &self.canvas_pipeline_cache,
        })
    }

    fn ensure_canvas_texture(&mut self, key: &str, size: PixelSize) -> Result<(), RenderError> {
        if size.width == 0 || size.height == 0 {
            return Err(RenderError::Backend(format!(
                "canvas context {key:?} requires a non-zero size"
            )));
        }
        let recreate = self
            .textures
            .get(key)
            .is_none_or(|texture| texture.size != size || !texture.render_attachment);
        if !recreate {
            return Ok(());
        }

        let texture = self.create_texture_2d(
            "operad-wgpu-canvas-texture",
            size,
            OFFSCREEN_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
        )?;
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("operad-wgpu-canvas-texture-bind-group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.texture_sampler),
                },
            ],
        });
        self.textures.insert(
            key.to_string(),
            WgpuTextureResource {
                size,
                source_format: ResourceFormat::Rgba8,
                texture,
                view,
                bind_group,
                render_attachment: true,
            },
        );
        Ok(())
    }

    fn upload_resource_updates(
        &mut self,
        updates: &[ResourceUpdate],
        mut encoder: Option<&mut wgpu::CommandEncoder>,
    ) -> Result<(), RenderError> {
        for update in updates {
            self.upload_resource_update(update, encoder.as_deref_mut())?;
        }
        Ok(())
    }

    fn upload_resource_update(
        &mut self,
        update: &ResourceUpdate,
        encoder: Option<&mut wgpu::CommandEncoder>,
    ) -> Result<(), RenderError> {
        if !update.has_expected_byte_len() || !update.dirty_rect_is_valid() {
            return Err(RenderError::InvalidResourceUpdate(
                update.descriptor.handle.id().key.clone(),
            ));
        }

        let key = update.descriptor.handle.id().key.clone();
        let size = update.descriptor.size;
        if size.width == 0 || size.height == 0 {
            self.textures.remove(&key);
            return Ok(());
        }

        let existing = self.textures.get(&key);
        if update.is_partial()
            && existing.is_none_or(|texture| {
                texture.size != size || texture.source_format != update.descriptor.format
            })
        {
            return Err(RenderError::InvalidResourceUpdate(key));
        }
        let recreate = existing.is_none_or(|texture| texture.size != size);
        if recreate {
            let texture = self
                .create_texture_2d(
                    "operad-wgpu-resource-texture",
                    size,
                    OFFSCREEN_FORMAT,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                )
                .map_err(|error| RenderError::InvalidResourceUpdate(format!("{key:?}: {error}")))?;
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("operad-wgpu-resource-texture-bind-group"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.texture_sampler),
                    },
                ],
            });
            self.textures.insert(
                key.clone(),
                WgpuTextureResource {
                    size,
                    source_format: update.descriptor.format,
                    texture,
                    view,
                    bind_group,
                    render_attachment: false,
                },
            );
        }

        let dirty_rect = update
            .dirty_rect
            .unwrap_or_else(|| PixelRect::new(0, 0, size.width, size.height));
        let rgba = rgba_bytes_for_update(update)?;
        let bytes_per_row = dirty_rect
            .width
            .checked_mul(4)
            .ok_or_else(|| RenderError::Backend("wgpu texture update row overflow".to_string()))?;
        let texture = self.textures.get_mut(&key).ok_or_else(|| {
            RenderError::Backend("wgpu texture upload target missing".to_string())
        })?;
        let destination = TexelCopyTextureInfo {
            texture: &texture.texture,
            mip_level: 0,
            origin: Origin3d {
                x: dirty_rect.x,
                y: dirty_rect.y,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        };
        if let Some(encoder) = encoder {
            encode_texture_upload(&self.device, encoder, destination, dirty_rect, &rgba)?;
        } else {
            self.queue.write_texture(
                destination,
                &rgba,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(dirty_rect.height),
                },
                Extent3d {
                    width: dirty_rect.width,
                    height: dirty_rect.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        texture.source_format = update.descriptor.format;
        Ok(())
    }

    fn prepare_with_glyph_atlas_retry<T>(
        &mut self,
        format: TextureFormat,
        mut prepare: impl FnMut(&mut Self) -> Result<T, GlyphPreparationError>,
    ) -> Result<T, RenderError> {
        self.ensure_glyphon_text(format);
        match prepare(self) {
            Err(GlyphPreparationError::Prepare(GlyphPrepareError::AtlasFull)) => {
                // Cached renderers retain atlas coordinates. Rebuild the atlas
                // and all prepared chunks together, then retry the whole pass.
                // A working set that still cannot fit returns its error.
                self.glyph_format = None;
                self.ensure_glyphon_text(format);
                prepare(self).map_err(GlyphPreparationError::into_render_error)
            }
            result => result.map_err(GlyphPreparationError::into_render_error),
        }
    }

    fn prepare_glyphon_text(
        &mut self,
        size: PixelSize,
        format: TextureFormat,
        texts: &[TextPaint],
    ) -> Result<bool, RenderError> {
        if texts.is_empty() || size.width == 0 || size.height == 0 {
            return Ok(false);
        }
        let target_rect = UiRect::new(0.0, 0.0, size.width as f32, size.height as f32);
        let visible_texts = texts
            .iter()
            .filter(|text| {
                text.rect
                    .intersection(text.clip)
                    .and_then(|rect| rect.intersection(target_rect))
                    .is_some()
            })
            .collect::<Vec<_>>();
        if visible_texts.is_empty() {
            return Ok(false);
        }

        let render_keys = visible_texts
            .iter()
            .map(|text| TextRenderKey::new(text, size))
            .collect::<Vec<_>>();
        self.glyph_generation = self.glyph_generation.wrapping_add(1);
        let generation = self.glyph_generation;
        self.update_glyph_viewport(size);

        self.prepare_with_glyph_atlas_retry(format, |context| {
            context.glyph_scene_active = false;
            context.glyph_chunks_active = false;
            for (text, render_key) in visible_texts.iter().zip(render_keys.iter()) {
                context.ensure_glyph_buffer(text, &render_key.buffer, generation);
            }

            if visible_texts.len() > GLYPH_TEXT_CHUNK_SIZE {
                context.glyph_chunk_order = context.prepare_glyphon_text_chunks(
                    size,
                    &visible_texts,
                    &render_keys,
                    generation,
                )?;
                context.glyph_chunks_active = true;
            } else if context.glyph_scene_renderer.is_some()
                && context.glyph_scene_key == render_keys
            {
                context.glyph_scene_active = true;
            } else {
                context.prepare_glyphon_text_scene(size, &visible_texts, &render_keys)?;
                context.glyph_scene_active = true;
            }
            Ok(())
        })?;
        self.prune_glyphon_text_cache();

        Ok(true)
    }

    fn prepare_glyphon_text_batches(
        &mut self,
        size: PixelSize,
        format: TextureFormat,
        geometry: &RenderGeometry,
    ) -> Result<Vec<Vec<TextChunkKey>>, RenderError> {
        if geometry.texts.is_empty() || size.width == 0 || size.height == 0 {
            return Ok(vec![Vec::new(); geometry.batches.len()]);
        }

        self.glyph_generation = self.glyph_generation.wrapping_add(1);
        let generation = self.glyph_generation;
        self.update_glyph_viewport(size);

        let target_rect = UiRect::new(0.0, 0.0, size.width as f32, size.height as f32);
        let batch_chunks = self.prepare_with_glyph_atlas_retry(format, |context| {
            context.glyph_scene_active = false;
            context.glyph_chunks_active = false;
            context.glyph_chunk_order.clear();
            let mut batch_chunks = vec![Vec::new(); geometry.batches.len()];
            for (batch_index, batch) in geometry.batches.iter().enumerate() {
                if batch.kind != GeometryBatchKind::Text {
                    continue;
                }
                let Some(texts) = text_batch_slice(&geometry.texts, batch) else {
                    continue;
                };
                let visible_texts = texts
                    .iter()
                    .filter(|text| {
                        text.rect
                            .intersection(text.clip)
                            .and_then(|rect| rect.intersection(target_rect))
                            .is_some()
                    })
                    .collect::<Vec<_>>();
                if visible_texts.is_empty() {
                    continue;
                }
                let render_keys = visible_texts
                    .iter()
                    .map(|text| TextRenderKey::new(text, size))
                    .collect::<Vec<_>>();
                for (text, render_key) in visible_texts.iter().zip(render_keys.iter()) {
                    context.ensure_glyph_buffer(text, &render_key.buffer, generation);
                }
                let chunks = context.prepare_glyphon_text_chunks(
                    size,
                    &visible_texts,
                    &render_keys,
                    generation,
                )?;
                context.glyph_chunk_order.extend(chunks.iter().cloned());
                batch_chunks[batch_index] = chunks;
                context.glyph_chunks_active = true;
            }
            Ok(batch_chunks)
        })?;
        self.prune_glyphon_text_cache();

        Ok(batch_chunks)
    }

    fn update_glyph_viewport(&mut self, size: PixelSize) {
        let resolution = GlyphResolution {
            width: size.width,
            height: size.height,
        };
        if self.glyph_viewport.resolution() != resolution {
            // Previously recorded passes can still reference the old uniform.
            // Equal-size passes reuse it; a resize gets an immutable replacement.
            self.glyph_viewport = GlyphViewport::new(&self.device, &self.glyph_cache);
            self.glyph_viewport.update(&self.queue, resolution);
        }
    }

    fn ensure_glyph_buffer(
        &mut self,
        text: &TextPaint,
        buffer_key: &TextBufferKey,
        generation: u64,
    ) {
        if let Some(cached) = self.glyph_buffer_cache.get_mut(buffer_key) {
            cached.last_used_generation = generation;
            return;
        }

        if let Some(cached) = self.glyph_scratch_buffer_cache.get_mut(buffer_key) {
            cached.stable_hits = cached.stable_hits.saturating_add(1);
            cached.last_used_generation = generation;
            if cached.stable_hits >= 1 {
                let Some(cached) = self.glyph_scratch_buffer_cache.remove(buffer_key) else {
                    return;
                };
                self.glyph_buffer_cache.insert(buffer_key.clone(), cached);
            }
            return;
        }

        let mut buffer = GlyphBuffer::new_empty(GlyphMetrics::new(1.0, 1.0));
        sync_glyph_buffer(&mut buffer, &mut self.font_system, text, None, buffer_key);
        self.glyph_scratch_buffer_cache.insert(
            buffer_key.clone(),
            CachedGlyphBuffer {
                key: buffer_key.clone(),
                buffer,
                stable_hits: 0,
                last_used_generation: generation,
            },
        );
    }

    fn prepare_glyphon_text_scene(
        &mut self,
        size: PixelSize,
        texts: &[&TextPaint],
        render_keys: &[TextRenderKey],
    ) -> Result<(), GlyphPreparationError> {
        let renderer = self.prepare_glyphon_text_renderer(size, texts, render_keys)?;
        self.glyph_scene_renderer = Some(renderer);
        self.glyph_scene_key = render_keys.to_vec();
        Ok(())
    }

    fn prepare_glyphon_text_chunks(
        &mut self,
        size: PixelSize,
        texts: &[&TextPaint],
        render_keys: &[TextRenderKey],
        generation: u64,
    ) -> Result<Vec<TextChunkKey>, GlyphPreparationError> {
        let mut chunk_order = Vec::with_capacity(
            render_keys.len().saturating_add(GLYPH_TEXT_CHUNK_SIZE - 1) / GLYPH_TEXT_CHUNK_SIZE,
        );
        for (text_chunk, key_chunk) in texts
            .chunks(GLYPH_TEXT_CHUNK_SIZE)
            .zip(render_keys.chunks(GLYPH_TEXT_CHUNK_SIZE))
        {
            let chunk_key = TextChunkKey {
                texts: key_chunk.to_vec(),
            };
            chunk_order.push(chunk_key.clone());
            if let Some(cached) = self.glyph_chunk_cache.get_mut(&chunk_key) {
                cached.last_used_generation = generation;
                continue;
            }

            let renderer = self.prepare_glyphon_text_renderer(size, text_chunk, key_chunk)?;
            self.glyph_chunk_cache.insert(
                chunk_key,
                CachedGlyphChunk {
                    renderer,
                    last_used_generation: generation,
                },
            );
        }
        Ok(chunk_order)
    }

    fn prepare_glyphon_text_renderer(
        &mut self,
        size: PixelSize,
        texts: &[&TextPaint],
        render_keys: &[TextRenderKey],
    ) -> Result<GlyphTextRenderer, GlyphPreparationError> {
        let mut text_areas = Vec::with_capacity(texts.len());
        for (text, render_key) in texts.iter().zip(render_keys.iter()) {
            let buffer = self
                .glyph_buffer_cache
                .get(&render_key.buffer)
                .or_else(|| self.glyph_scratch_buffer_cache.get(&render_key.buffer))
                .ok_or_else(|| {
                    RenderError::Backend("glyph text buffer missing from cache".to_string())
                })?;
            text_areas.push(GlyphTextArea {
                buffer: &buffer.buffer,
                left: text.rect.x,
                top: glyph_text_area_top(text, &buffer.buffer),
                scale: 1.0,
                bounds: glyph_text_bounds(text.clip, size),
                default_color: glyph_color(text.style.color, text.opacity),
                custom_glyphs: &[],
            });
        }
        let atlas = self
            .glyph_atlas
            .as_mut()
            .ok_or_else(|| RenderError::Backend("glyph atlas is not initialized".to_string()))?;
        let mut renderer =
            GlyphTextRenderer::new(atlas, &self.device, wgpu::MultisampleState::default(), None);
        renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                atlas,
                &self.glyph_viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .map_err(GlyphPreparationError::Prepare)?;
        Ok(renderer)
    }

    fn render_glyphon_text_chunks(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        chunk_keys: &[TextChunkKey],
    ) -> Result<(), RenderError> {
        let Some(atlas) = &self.glyph_atlas else {
            return Ok(());
        };
        for chunk_key in chunk_keys {
            let chunk = self.glyph_chunk_cache.get(chunk_key).ok_or_else(|| {
                RenderError::Backend("glyph text chunk missing from cache".to_string())
            })?;
            chunk
                .renderer
                .render(atlas, &self.glyph_viewport, pass)
                .map_err(glyph_render_error)?;
        }
        Ok(())
    }

    fn ensure_glyphon_text(&mut self, format: TextureFormat) {
        if self.glyph_format == Some(format) {
            return;
        }

        let atlas = GlyphTextAtlas::with_color_mode(
            &self.device,
            &self.queue,
            &self.glyph_cache,
            format,
            glyph_color_mode(format),
        );
        self.glyph_atlas = Some(atlas);
        self.glyph_format = Some(format);
        self.glyph_buffer_cache.clear();
        self.glyph_scratch_buffer_cache.clear();
        self.glyph_scene_renderer = None;
        self.glyph_scene_key.clear();
        self.glyph_scene_active = false;
        self.glyph_chunk_cache.clear();
        self.glyph_chunk_order.clear();
        self.glyph_chunks_active = false;
    }

    fn set_fonts(&mut self, fonts: &FontLibrary) {
        self.font_system = glyph_font_system(fonts);
        self.swash_cache = GlyphSwashCache::new();
        // Font IDs belong to a font database and can be reused by its replacement.
        // Rasterized atlas entries must not outlive the database that keyed them.
        self.glyph_atlas = None;
        self.glyph_format = None;
        self.glyph_buffer_cache.clear();
        self.glyph_scratch_buffer_cache.clear();
        self.glyph_scene_renderer = None;
        self.glyph_scene_key.clear();
        self.glyph_scene_active = false;
        self.glyph_chunk_cache.clear();
        self.glyph_chunk_order.clear();
        self.glyph_chunks_active = false;
        self.glyph_generation = self.glyph_generation.wrapping_add(1);
    }

    fn prune_glyphon_text_cache(&mut self) {
        const MAX_RETAINED_UNUSED_FRAMES: u64 = 120;
        let generation = self.glyph_generation;
        self.glyph_buffer_cache.retain(|_, cached| {
            generation.saturating_sub(cached.last_used_generation) <= MAX_RETAINED_UNUSED_FRAMES
        });
        self.glyph_scratch_buffer_cache.retain(|_, cached| {
            generation.saturating_sub(cached.last_used_generation) <= MAX_RETAINED_UNUSED_FRAMES
        });
        self.glyph_chunk_cache.retain(|_, cached| {
            generation.saturating_sub(cached.last_used_generation) <= MAX_RETAINED_UNUSED_FRAMES
        });
    }

    fn read_gpu_render_duration(&mut self) -> Result<Option<Duration>, RenderError> {
        let Some(timer) = &self.gpu_timer else {
            return Ok(None);
        };
        let readback_slice = timer.readback_buffer.slice(..GPU_TIMESTAMP_QUERY_BYTES);
        let (tx, rx) = mpsc::channel();
        readback_slice.map_async(wgpu::MapMode::Read, move |status| {
            let _ = tx.send(status);
        });
        let _ = self
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| RenderError::Backend(format!("wgpu poll failed: {error}")))?;
        rx.recv()
            .map_err(|_| RenderError::Backend("wgpu timestamp map wait failed".to_string()))?
            .map_err(|error| {
                RenderError::Backend(format!("wgpu timestamp map error: {error:?}"))
            })?;

        let mapped = readback_slice.get_mapped_range();
        let start = read_timestamp_query_value(&mapped[0..8])?;
        let end = read_timestamp_query_value(&mapped[8..16])?;
        drop(mapped);
        timer.readback_buffer.unmap();

        if end < start {
            return Ok(None);
        }
        let nanos = (end - start) as f64 * f64::from(self.queue.get_timestamp_period());
        Ok(Some(Duration::from_nanos(
            nanos.round().clamp(0.0, u64::MAX as f64) as u64,
        )))
    }

    fn has_pending_gpu_timing(&self) -> bool {
        self.gpu_timer
            .as_ref()
            .is_some_and(GpuTimer::has_pending_token)
    }

    fn mark_pending_gpu_timing(&mut self) -> Option<WgpuGpuTimingToken> {
        self.gpu_timer.as_mut().map(GpuTimer::mark_pending)
    }

    fn read_pending_gpu_render_duration(
        &mut self,
        token: WgpuGpuTimingToken,
    ) -> Result<Option<Duration>, RenderError> {
        let duration = self.read_gpu_render_duration()?;
        if let Some(timer) = &mut self.gpu_timer {
            timer.clear_pending(token)?;
        }
        Ok(duration)
    }

    fn discard_view(
        &mut self,
        size: PixelSize,
        format: TextureFormat,
    ) -> Result<wgpu::TextureView, RenderError> {
        let recreate = self
            .discard_target
            .as_ref()
            .is_none_or(|target| target.size != size || target.format != format);
        if recreate {
            let texture = self.create_texture_2d(
                "operad-wgpu-discard-texture",
                size,
                format,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            )?;
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.discard_target = Some(CachedTarget {
                size,
                format,
                _texture: texture,
                view,
            });
        }
        self.discard_target
            .as_ref()
            .map(|target| target.view.clone())
            .ok_or_else(|| RenderError::Backend("discard render target is unavailable".to_string()))
    }
}

#[derive(Debug)]
struct CachedTarget {
    size: PixelSize,
    format: TextureFormat,
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

#[derive(Debug)]
struct WgpuTextureResource {
    size: PixelSize,
    // Upload encoding can differ from the canonical RGBA8 GPU storage format.
    source_format: ResourceFormat,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    render_attachment: bool,
}

// Layer indices are private to one frame and cannot alias app resource names.
// Borrow app names while batching; only a new batch needs to own its key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum WgpuTextureKey<'a> {
    Resource(Cow<'a, str>),
    Layer(usize),
}

impl WgpuTextureKey<'_> {
    fn into_owned(self) -> WgpuTextureKey<'static> {
        match self {
            Self::Resource(key) => WgpuTextureKey::Resource(Cow::Owned(key.into_owned())),
            Self::Layer(index) => WgpuTextureKey::Layer(index),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct GpuVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl GpuVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

    fn new(point: UiPoint, color: [f32; 4]) -> Self {
        Self {
            position: [point.x, point.y],
            color,
        }
    }

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct GpuTexturedRectInstance {
    rect: [f32; 4],
    uv: [f32; 4],
    tint: [f32; 4],
}

impl GpuTexturedRectInstance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];

    fn new(rect: UiRect, uv: [f32; 4], tint: [f32; 4]) -> Self {
        Self {
            rect: [rect.x, rect.y, rect.width, rect.height],
            uv,
            tint,
        }
    }

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct GpuCompositedRectInstance {
    rect: [f32; 4],
    uv: [f32; 4],
    tint: [f32; 4],
    clip_rect: [f32; 4],
    mask_rect: [f32; 4],
    params: [f32; 4],
    filter_params: [f32; 4],
    texel_size: [f32; 2],
    shader_params: [f32; 4],
    shader_color: [f32; 4],
    clip_radii: [f32; 4],
    _pad: [f32; 2],
}

impl GpuCompositedRectInstance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 11] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x2,
        8 => Float32x4,
        9 => Float32x4,
        10 => Float32x4
    ];

    fn new(
        rect: UiRect,
        uv: [f32; 4],
        opacity: f32,
        clip: Option<(UiRect, CornerRadii)>,
        mask: Option<UiRect>,
        filter_params: LayerFilterParams,
        shader_params: LayerShaderParams,
        texture_size: PixelSize,
    ) -> Self {
        let (clip_rect, clip_radii, clip_enabled) = match clip {
            Some((rect, radii)) => (rect, radii, 1.0),
            None => (rect, CornerRadii::ZERO, 0.0),
        };
        let (mask_rect, mask_enabled) = match mask {
            Some(rect) => (rect, 1.0),
            None => (rect, 0.0),
        };
        Self {
            rect: [rect.x, rect.y, rect.width, rect.height],
            uv,
            tint: [1.0, 1.0, 1.0, 1.0],
            clip_rect: [clip_rect.x, clip_rect.y, clip_rect.width, clip_rect.height],
            mask_rect: [mask_rect.x, mask_rect.y, mask_rect.width, mask_rect.height],
            params: [
                opacity.clamp(0.0, 1.0),
                clip_enabled,
                mask_enabled,
                filter_params.blur_radius,
            ],
            filter_params: [
                filter_params.brightness,
                filter_params.contrast,
                filter_params.saturate,
                0.0,
            ],
            texel_size: [
                1.0 / texture_size.width.max(1) as f32,
                1.0 / texture_size.height.max(1) as f32,
            ],
            shader_params: shader_params.params,
            shader_color: shader_params.color,
            clip_radii: [
                clip_radii.top_left,
                clip_radii.top_right,
                clip_radii.bottom_right,
                clip_radii.bottom_left,
            ],
            _pad: [0.0; 2],
        }
    }

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeometryBatchKind {
    Sdf(SdfPipelineKind),
    Triangle,
    TexturedRect,
    CompositedRect,
    Text,
}

#[derive(Debug, Clone)]
struct GeometryBatch {
    kind: GeometryBatchKind,
    clip: UiRect,
    texture_key: Option<WgpuTextureKey<'static>>,
    start: u32,
    count: u32,
}

#[derive(Debug, Clone)]
struct TextPaint {
    rect: UiRect,
    clip: UiRect,
    text: String,
    style: TextStyle,
    horizontal_align: TextHorizontalAlign,
    vertical_align: TextVerticalAlign,
    opacity: f32,
}

struct CachedGlyphBuffer {
    key: TextBufferKey,
    buffer: GlyphBuffer,
    stable_hits: u8,
    last_used_generation: u64,
}

impl std::fmt::Debug for CachedGlyphBuffer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CachedGlyphBuffer")
            .field("key", &self.key)
            .field("stable_hits", &self.stable_hits)
            .field("last_used_generation", &self.last_used_generation)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextChunkKey {
    texts: Vec<TextRenderKey>,
}

struct CachedGlyphChunk {
    renderer: GlyphTextRenderer,
    last_used_generation: u64,
}

impl std::fmt::Debug for CachedGlyphChunk {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CachedGlyphChunk")
            .field("last_used_generation", &self.last_used_generation)
            .finish_non_exhaustive()
    }
}

struct GpuTimer {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    pending_token: Option<WgpuGpuTimingToken>,
    next_token_id: u64,
}

impl GpuTimer {
    fn new_if_supported(device: &wgpu::Device) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("operad-wgpu-render-timestamp-query"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("operad-wgpu-render-timestamp-resolve"),
            size: GPU_TIMESTAMP_QUERY_BYTES,
            usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("operad-wgpu-render-timestamp-readback"),
            size: GPU_TIMESTAMP_QUERY_BYTES,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Some(Self {
            query_set,
            resolve_buffer,
            readback_buffer,
            pending_token: None,
            next_token_id: 1,
        })
    }

    fn has_pending_token(&self) -> bool {
        self.pending_token.is_some()
    }

    fn mark_pending(&mut self) -> WgpuGpuTimingToken {
        let token = WgpuGpuTimingToken {
            id: self.next_token_id,
        };
        self.next_token_id = self.next_token_id.wrapping_add(1).max(1);
        self.pending_token = Some(token);
        token
    }

    fn clear_pending(&mut self, token: WgpuGpuTimingToken) -> Result<(), RenderError> {
        match self.pending_token {
            Some(pending) if pending == token => {
                self.pending_token = None;
                Ok(())
            }
            Some(pending) => Err(RenderError::Backend(format!(
                "wgpu GPU timing token mismatch: pending {}, got {}",
                pending.id, token.id
            ))),
            None => Err(RenderError::Backend(
                "wgpu GPU timing token has already been resolved".to_string(),
            )),
        }
    }
}

impl std::fmt::Debug for GpuTimer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("GpuTimer").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextBufferKey {
    text: String,
    width: u32,
    height: u32,
    font_size: u32,
    line_height: u32,
    family: FontFamily,
    weight: crate::FontWeight,
    style: FontStyle,
    stretch: FontStretch,
    wrap: TextWrap,
    overflow: TextOverflow,
    horizontal_align: TextHorizontalAlign,
}

impl TextBufferKey {
    fn new(text: &TextPaint) -> Self {
        Self {
            text: text.text.clone(),
            width: text.rect.width.max(0.0).to_bits(),
            height: text.rect.height.max(0.0).to_bits(),
            font_size: text.style.font_size.max(1.0).to_bits(),
            line_height: text.style.line_height.max(1.0).to_bits(),
            family: text.style.family.clone(),
            weight: text.style.weight,
            style: text.style.style,
            stretch: text.style.stretch,
            wrap: text.style.wrap,
            overflow: text.style.overflow,
            horizontal_align: text.horizontal_align,
        }
    }

    fn has_same_layout_as(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.font_size == other.font_size
            && self.line_height == other.line_height
            && self.family == other.family
            && self.weight == other.weight
            && self.style == other.style
            && self.stretch == other.stretch
            && self.wrap == other.wrap
            && self.overflow == other.overflow
            && self.horizontal_align == other.horizontal_align
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextRenderKey {
    buffer: TextBufferKey,
    target_width: u32,
    target_height: u32,
    rect_x: u32,
    rect_y: u32,
    clip_x: u32,
    clip_y: u32,
    clip_width: u32,
    clip_height: u32,
    color: (u8, u8, u8, u8),
    opacity: u32,
    vertical_align: TextVerticalAlign,
}

impl TextRenderKey {
    fn new(text: &TextPaint, target_size: PixelSize) -> Self {
        Self {
            buffer: TextBufferKey::new(text),
            target_width: target_size.width,
            target_height: target_size.height,
            rect_x: text.rect.x.to_bits(),
            rect_y: text.rect.y.to_bits(),
            clip_x: text.clip.x.to_bits(),
            clip_y: text.clip.y.to_bits(),
            clip_width: text.clip.width.to_bits(),
            clip_height: text.clip.height.to_bits(),
            color: (
                text.style.color.r,
                text.style.color.g,
                text.style.color.b,
                text.style.color.a,
            ),
            opacity: text.opacity.to_bits(),
            vertical_align: text.vertical_align,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct RenderGeometry {
    shapes: Vec<SdfInstance>,
    gradient_stops: Vec<SdfGradientStop>,
    textured_rects: Vec<GpuTexturedRectInstance>,
    composited_rects: Vec<GpuCompositedRectInstance>,
    vertices: Vec<GpuVertex>,
    texts: Vec<TextPaint>,
    batches: Vec<GeometryBatch>,
}

impl RenderGeometry {
    fn clear(&mut self) {
        self.shapes.clear();
        self.gradient_stops.clear();
        self.textured_rects.clear();
        self.composited_rects.clear();
        self.vertices.clear();
        self.texts.clear();
        self.batches.clear();
    }

    fn push_batch(
        &mut self,
        kind: GeometryBatchKind,
        clip: UiRect,
        texture_key: Option<WgpuTextureKey<'_>>,
        start: u32,
        count: u32,
    ) {
        if let Some(batch) = self.batches.last_mut() {
            if batch.kind == kind
                && batch.clip == clip
                && batch.texture_key == texture_key
                && batch
                    .start
                    .checked_add(batch.count)
                    .is_some_and(|end| end == start)
            {
                batch.count = batch.count.saturating_add(count);
                return;
            }
        }

        self.batches.push(GeometryBatch {
            kind,
            clip,
            texture_key: texture_key.map(WgpuTextureKey::into_owned),
            start,
            count,
        });
    }

    fn push_triangle_vertices(&mut self, clip: UiRect, vertices: &[GpuVertex]) {
        if vertices.is_empty() {
            return;
        }

        let Ok(vertex_start) = u32::try_from(self.vertices.len()) else {
            return;
        };
        let Ok(vertex_count) = u32::try_from(vertices.len()) else {
            return;
        };
        self.vertices.extend_from_slice(vertices);
        self.push_batch(
            GeometryBatchKind::Triangle,
            clip,
            None,
            vertex_start,
            vertex_count,
        );
    }

    fn push_shape(&mut self, clip: UiRect, shape: SdfInstance) {
        if !shape.intersects_clip(clip)
            || (shape.color[3] == 0.0
                && shape.border_color[3] == 0.0
                && shape.gradient_range[1] == 0)
        {
            return;
        }
        let Ok(start) = u32::try_from(self.shapes.len()) else {
            return;
        };
        let kind = shape.pipeline_kind();
        self.shapes.push(shape);
        self.push_batch(GeometryBatchKind::Sdf(kind), clip, None, start, 1);
    }

    fn push_textured_rect(
        &mut self,
        clip: UiRect,
        texture_key: &str,
        rect: UiRect,
        uv: [f32; 4],
        tint: [f32; 4],
    ) {
        let Ok(start) = u32::try_from(self.textured_rects.len()) else {
            return;
        };
        self.textured_rects
            .push(GpuTexturedRectInstance::new(rect, uv, tint));
        self.push_batch(
            GeometryBatchKind::TexturedRect,
            clip,
            Some(WgpuTextureKey::Resource(Cow::Borrowed(texture_key))),
            start,
            1,
        );
    }

    fn push_composited_rect(
        &mut self,
        clip: UiRect,
        texture_key: WgpuTextureKey<'static>,
        instance: GpuCompositedRectInstance,
    ) {
        let Ok(start) = u32::try_from(self.composited_rects.len()) else {
            return;
        };
        self.composited_rects.push(instance);
        self.push_batch(
            GeometryBatchKind::CompositedRect,
            clip,
            Some(texture_key),
            start,
            1,
        );
    }

    fn push_text(&mut self, text: TextPaint) {
        let Ok(start) = u32::try_from(self.texts.len()) else {
            return;
        };
        let clip = text.clip;
        self.texts.push(text);
        self.push_batch(GeometryBatchKind::Text, clip, None, start, 1);
    }
}

fn text_batch_slice<'a>(texts: &'a [TextPaint], batch: &GeometryBatch) -> Option<&'a [TextPaint]> {
    let start = usize::try_from(batch.start).ok()?;
    let count = usize::try_from(batch.count).ok()?;
    texts.get(start..start.checked_add(count)?)
}

impl WgpuRenderer {
    pub fn new() -> Self {
        Self {
            context: None,
            geometry: RenderGeometry::default(),
            font_library: FontLibrary::default(),
        }
    }

    pub fn with_device_queue(
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Result<Self, RenderError> {
        Self::with_device_queue_and_fonts(device, queue, FontLibrary::default())
    }

    pub fn with_device_queue_and_fonts(
        device: wgpu::Device,
        queue: wgpu::Queue,
        fonts: FontLibrary,
    ) -> Result<Self, RenderError> {
        Ok(Self {
            context: Some(WgpuContext::new(device, queue, &fonts)?),
            geometry: RenderGeometry::default(),
            font_library: fonts,
        })
    }

    pub fn font_library(&self) -> &FontLibrary {
        &self.font_library
    }

    pub fn set_fonts(&mut self, fonts: FontLibrary) -> &mut Self {
        self.font_library = fonts;
        if let Some(context) = &mut self.context {
            context.set_fonts(&self.font_library);
        }
        self
    }

    pub fn supports_app_owned_view_rendering(&self) -> bool {
        self.capabilities().rendering.app_owned_view_rendering
    }

    pub fn canvas_context(
        &mut self,
        canvas: &crate::CanvasContent,
        size: PixelSize,
    ) -> Result<WgpuCanvasContext<'_>, RenderError> {
        self.ensure_context()?.canvas_context(canvas, size)
    }

    pub fn get_canvas_context(
        &mut self,
        canvas: &crate::CanvasContent,
        size: PixelSize,
    ) -> Result<WgpuCanvasContext<'_>, RenderError> {
        self.canvas_context(canvas, size)
    }

    pub fn get_gpu_context(
        &mut self,
        canvas: &crate::CanvasContent,
        size: PixelSize,
    ) -> Result<WgpuCanvasContext<'_>, RenderError> {
        if !canvas.context.kind.is_gpu_backed() {
            return Err(RenderError::Backend(format!(
                "canvas {:?} does not have a GPU context",
                canvas.key
            )));
        }
        self.canvas_context(canvas, size)
    }

    pub fn warm_up(&mut self) -> Result<(), RenderError> {
        let context = self.ensure_context()?;
        let _ = context.triangle_pipeline(OFFSCREEN_FORMAT);
        let _ = context.textured_rect_pipeline(OFFSCREEN_FORMAT);
        let _ = context.composited_rect_pipeline(OFFSCREEN_FORMAT);
        for kind in SdfPipelineKind::ALL {
            let _ = context.sdf_pipeline(OFFSCREEN_FORMAT, kind);
        }
        context.prepare_glyphon_text(
            PixelSize::new(512, 128),
            OFFSCREEN_FORMAT,
            &[TextPaint {
                rect: UiRect::new(0.0, 0.0, 512.0, 128.0),
                clip: UiRect::new(0.0, 0.0, 512.0, 128.0),
                text: "Operad glyphon warmup 0123456789 reusable toolkit surface".to_string(),
                style: TextStyle {
                    font_size: 12.0,
                    line_height: 16.0,
                    color: ColorRgba::WHITE,
                    ..Default::default()
                },
                horizontal_align: TextHorizontalAlign::Start,
                vertical_align: TextVerticalAlign::Top,
                opacity: 1.0,
            }],
        )?;
        Ok(())
    }

    /// Record a frame through a caller-owned encoder without presenting it.
    ///
    /// This legacy path renders `Window` and `AppOwned` targets into an
    /// internal discard texture. Use
    /// [`WgpuRenderer::render_frame_into_view_with_encoder`] when composing
    /// Operad UI into a caller-owned swapchain or texture view.
    pub fn render_frame_with_encoder(
        &mut self,
        request: RenderFrameRequest,
        resolver: &dyn ResourceResolver,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<RenderFrameOutput, RenderError> {
        let batch_started = Instant::now();
        let batches = request.batches();
        let batch_duration = batch_started.elapsed();

        self.validate_resource_updates(&request, resolver)?;

        let size = render_target_pixel_size(
            &request.target,
            request.viewport,
            request.options.scale_factor,
        )?;
        let target_kind = request.target.kind();
        let clear_color = request.options.clear_color;
        let mut context = WgpuContext::new(device.clone(), queue.clone(), &self.font_library)?;
        context.upload_resource_updates(&request.resource_updates, None)?;
        context.begin_frame();
        self.geometry.clear();
        build_geometry_into(
            &mut self.geometry,
            &request.paint,
            &mut context,
            UiPoint::new(0.0, 0.0),
            request.options.scale_factor,
            None,
        )?;
        let mut output = RenderFrameOutput::new(request.target);
        output.painted_items = request.paint.items.len();
        output.batches = batches;
        output.dirty_regions = request.dirty_regions.clone();

        let render_started = Instant::now();
        match target_kind {
            RenderTargetKind::Snapshot | RenderTargetKind::Offscreen => {
                let snapshot =
                    render_snapshot_with_context(&mut context, size, &self.geometry, clear_color)?;
                output.snapshot = Some(RenderedImage::new(size, ResourceFormat::Rgba8, snapshot));
            }
            RenderTargetKind::Window | RenderTargetKind::AppOwned => {
                record_discard_frame(
                    &mut context,
                    encoder,
                    size,
                    &self.geometry,
                    clear_color,
                    false,
                )?;
            }
        }

        output.timings = FrameTiming::new()
            .section("batch", batch_duration)
            .section("render", render_started.elapsed());
        Ok(output)
    }

    /// Record an app-owned or window render request into a caller-owned texture
    /// view using a caller-owned command encoder.
    ///
    /// Construct this renderer with [`WgpuRenderer::with_device_queue`] using
    /// the same `wgpu::Device` and `wgpu::Queue` that created `encoder` and
    /// `target.view`. This method records commands only; the caller remains
    /// responsible for submitting the encoder and presenting the target. This
    /// includes image uploads, embedded canvas programs, and composited layers:
    /// multiple calls can be recorded before submission without overwriting an
    /// earlier pass's geometry or text. Submit dependent resource updates in
    /// recording order; later passes can reuse images updated by earlier ones.
    /// Dropping an encoder discards its recorded uploads as well as its draws.
    ///
    /// This method does not record GPU timing; use
    /// [`WgpuRenderer::render_frame_into_view_with_encoder_timed`] when the
    /// caller will resolve timing after queue submission.
    pub fn render_frame_into_view_with_encoder(
        &mut self,
        request: RenderFrameRequest,
        resolver: &dyn ResourceResolver,
        encoder: &mut wgpu::CommandEncoder,
        target: WgpuRenderTargetView<'_>,
    ) -> Result<RenderFrameOutput, RenderError> {
        Ok(self
            .render_frame_into_view_with_encoder_impl(request, resolver, encoder, target, false)?
            .frame)
    }

    /// Record into a caller-owned texture view and optionally return a GPU
    /// timing token.
    ///
    /// If `request.options.collect_gpu_timing` is true and the device supports
    /// timestamp queries, the returned token can be passed to
    /// [`WgpuRenderer::resolve_gpu_timing`] after the caller submits the
    /// encoder. A renderer can have only one unresolved timing token at a time.
    pub fn render_frame_into_view_with_encoder_timed(
        &mut self,
        request: RenderFrameRequest,
        resolver: &dyn ResourceResolver,
        encoder: &mut wgpu::CommandEncoder,
        target: WgpuRenderTargetView<'_>,
    ) -> Result<WgpuRenderFrameIntoViewOutput, RenderError> {
        let collect_gpu_timing = request.options.collect_gpu_timing;
        self.render_frame_into_view_with_encoder_impl(
            request,
            resolver,
            encoder,
            target,
            collect_gpu_timing,
        )
    }

    pub fn resolve_gpu_timing(
        &mut self,
        token: WgpuGpuTimingToken,
    ) -> Result<Option<Duration>, RenderError> {
        self.ensure_context()?
            .read_pending_gpu_render_duration(token)
    }

    fn render_frame_into_view_with_encoder_impl(
        &mut self,
        request: RenderFrameRequest,
        resolver: &dyn ResourceResolver,
        encoder: &mut wgpu::CommandEncoder,
        target: WgpuRenderTargetView<'_>,
        collect_gpu_timing: bool,
    ) -> Result<WgpuRenderFrameIntoViewOutput, RenderError> {
        let target_kind = request.target.kind();
        if !matches!(
            target_kind,
            RenderTargetKind::Window | RenderTargetKind::AppOwned
        ) {
            return Err(RenderError::UnsupportedTarget(target_kind));
        }

        let batch_started = Instant::now();
        let batches = request.batches();
        let batch_duration = batch_started.elapsed();

        self.validate_resource_updates(&request, resolver)?;

        let size = render_target_pixel_size(
            &request.target,
            request.viewport,
            request.options.scale_factor,
        )?;
        let clear_color = request.options.clear_color;
        let load_op = target.resolved_load_op(clear_color);
        {
            let context = self.ensure_context()?;
            context.upload_resource_updates(&request.resource_updates, Some(&mut *encoder))?;
            context.begin_frame();
        }
        let mut geometry = mem::take(&mut self.geometry);
        geometry.clear();
        let render_started = Instant::now();
        let gpu_timer_used;
        {
            let context = self.context.as_mut().ok_or_else(|| {
                RenderError::Backend("wgpu backend failed to initialize".to_string())
            })?;
            render_embedded_canvas_programs(context, &request, Some(&mut *encoder))?;
            build_geometry_into(
                &mut geometry,
                &request.paint,
                context,
                UiPoint::new(0.0, 0.0),
                request.options.scale_factor,
                Some(&mut *encoder),
            )?;
            gpu_timer_used = record_render_pass(
                context,
                encoder,
                target.view,
                target.format,
                size,
                &geometry,
                load_op,
                true,
                collect_gpu_timing,
                true,
            )?;
        }
        let gpu_timing_token = if gpu_timer_used {
            self.context
                .as_mut()
                .and_then(WgpuContext::mark_pending_gpu_timing)
        } else {
            None
        };
        self.geometry = geometry;

        let mut output = RenderFrameOutput::new(request.target);
        output.painted_items = request.paint.items.len();
        output.batches = batches;
        output.dirty_regions = request.dirty_regions;
        output.timings = FrameTiming::new()
            .section("batch", batch_duration)
            .section("render", render_started.elapsed());
        Ok(WgpuRenderFrameIntoViewOutput {
            frame: output,
            gpu_timing_token,
        })
    }

    fn prepare_frame<T>(
        &mut self,
        request: RenderFrameRequest,
        resolver: &dyn ResourceResolver,
        acquire: impl FnOnce(&mut WgpuContext, PixelSize) -> Result<T, RenderError>,
    ) -> Result<PreparedWgpuFrame<T>, RenderError> {
        let batch_started = Instant::now();
        let batches = request.batches();
        let batch_duration = batch_started.elapsed();
        self.validate_resource_updates(&request, resolver)?;
        let size = render_target_pixel_size(
            &request.target,
            request.viewport,
            request.options.scale_factor,
        )?;
        self.ensure_context()?;
        let context = self
            .context
            .as_mut()
            .ok_or_else(|| RenderError::Backend("wgpu backend failed to initialize".to_string()))?;
        // Hosts retain uploads after temporary presentation failures. Acquire
        // first: replaying an already-applied patch followed by a full resize
        // would try to apply the old patch against the new resource shape.
        let acquire_started = Instant::now();
        let acquired = acquire(context, size)?;
        let acquire_duration = acquire_started.elapsed();
        let mut geometry = mem::take(&mut self.geometry);
        geometry.clear();
        let prepared = (|| {
            context.upload_resource_updates(&request.resource_updates, None)?;
            context.begin_frame();
            render_embedded_canvas_programs(context, &request, None)?;
            build_geometry_into(
                &mut geometry,
                &request.paint,
                context,
                UiPoint::new(0.0, 0.0),
                request.options.scale_factor,
                None,
            )?;
            let mut output = RenderFrameOutput::new(request.target);
            output.painted_items = request.paint.items.len();
            output.batches = batches;
            output.dirty_regions = request.dirty_regions;
            output.timings = FrameTiming::new().section("batch", batch_duration);
            Ok(PreparedWgpuFrame {
                acquired,
                acquire_duration,
                size,
                output,
            })
        })();
        self.geometry = geometry;
        prepared
    }

    fn validate_resource_updates(
        &self,
        request: &RenderFrameRequest,
        _resolver: &dyn ResourceResolver,
    ) -> Result<(), RenderError> {
        let capabilities = self.capabilities();
        for update in &request.resource_updates {
            if !capabilities
                .resources
                .supports(update.descriptor.handle.kind())
            {
                return Err(RenderError::UnsupportedResource(
                    update.descriptor.handle.kind(),
                ));
            }
            if !update.has_expected_byte_len() || !update.dirty_rect_is_valid() {
                return Err(RenderError::InvalidResourceUpdate(
                    update.descriptor.handle.id().key.clone(),
                ));
            }
        }
        Ok(())
    }

    fn ensure_context(&mut self) -> Result<&mut WgpuContext, RenderError> {
        if self.context.is_none() {
            let instance =
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
            let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .map_err(|error| {
                RenderError::Backend(format!("wgpu request adapter failed: {error}"))
            })?;
            let adapter_features = adapter.features();
            let required_features = if adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
                wgpu::Features::TIMESTAMP_QUERY
            } else {
                wgpu::Features::empty()
            };

            let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("operad-wgpu-device"),
                required_features,
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            }))
            .map_err(|error| RenderError::Backend(error.to_string()))?;

            self.context = Some(WgpuContext::new(device, queue, &self.font_library)?);
        }

        self.context
            .as_mut()
            .ok_or_else(|| RenderError::Backend("wgpu backend failed to initialize".to_string()))
    }

    fn render_snapshot(
        &mut self,
        size: PixelSize,
        clear_color: ColorRgba,
    ) -> Result<Vec<u8>, RenderError> {
        self.ensure_context()?;
        let geometry = mem::take(&mut self.geometry);
        let result = {
            let context = self.context.as_mut().ok_or_else(|| {
                RenderError::Backend("wgpu backend failed to initialize".to_string())
            })?;
            render_snapshot_with_context(context, size, &geometry, clear_color)
        };
        self.geometry = geometry;
        result
    }

    fn render_discard_frame(
        &mut self,
        size: PixelSize,
        clear_color: ColorRgba,
        collect_gpu_timing: bool,
    ) -> Result<Option<Duration>, RenderError> {
        self.ensure_context()?;
        let geometry = mem::take(&mut self.geometry);
        let result = {
            let context = self.context.as_mut().ok_or_else(|| {
                RenderError::Backend("wgpu backend failed to initialize".to_string())
            })?;
            let mut encoder =
                context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("operad-wgpu-discard-encoder"),
                    });
            let gpu_timer_used = record_discard_frame(
                context,
                &mut encoder,
                size,
                &geometry,
                clear_color,
                collect_gpu_timing,
            )?;
            context.queue.submit(Some(encoder.finish()));
            if gpu_timer_used {
                context.read_gpu_render_duration()
            } else {
                Ok(None)
            }
        };
        self.geometry = geometry;
        result
    }
}

impl Default for WgpuRenderer {
    fn default() -> Self {
        Self::new()
    }
}

struct PreparedWgpuFrame<T> {
    acquired: T,
    acquire_duration: Duration,
    size: PixelSize,
    output: RenderFrameOutput,
}

#[derive(Debug)]
struct WgpuSurfaceTarget<'window> {
    surface: wgpu::Surface<'window>,
    surface_config: wgpu::SurfaceConfiguration,
    surface_needs_reconfigure: bool,
}

#[derive(Debug)]
pub struct WgpuSurfaceRenderer<'window> {
    renderer: WgpuRenderer,
    target: WgpuSurfaceTarget<'window>,
}

impl<'window> WgpuSurfaceRenderer<'window> {
    pub fn new(
        surface: wgpu::Surface<'window>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface_config: wgpu::SurfaceConfiguration,
    ) -> Result<Self, RenderError> {
        Self::new_with_fonts(
            surface,
            device,
            queue,
            surface_config,
            FontLibrary::default(),
        )
    }

    pub fn new_with_fonts(
        surface: wgpu::Surface<'window>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        mut surface_config: wgpu::SurfaceConfiguration,
        fonts: FontLibrary,
    ) -> Result<Self, RenderError> {
        surface_config
            .usage
            .insert(wgpu::TextureUsages::RENDER_ATTACHMENT);
        let renderer = WgpuRenderer::with_device_queue_and_fonts(device, queue, fonts)?;
        if surface_config.width > 0 && surface_config.height > 0 {
            let context = renderer.context.as_ref().ok_or_else(|| {
                RenderError::Backend("wgpu surface renderer failed to initialize".to_string())
            })?;
            validate_texture_size(
                PixelSize::new(surface_config.width, surface_config.height),
                context.limits.max_texture_dimension_2d,
                "surface",
            )?;
            surface.configure(&context.device, &surface_config);
        }
        Ok(Self {
            renderer,
            target: WgpuSurfaceTarget {
                surface,
                surface_config,
                surface_needs_reconfigure: false,
            },
        })
    }

    pub fn font_library(&self) -> &FontLibrary {
        self.renderer.font_library()
    }

    pub fn set_fonts(&mut self, fonts: FontLibrary) -> &mut Self {
        self.renderer.set_fonts(fonts);
        self
    }

    pub fn supports_app_owned_view_rendering(&self) -> bool {
        self.renderer.supports_app_owned_view_rendering()
    }

    pub fn canvas_context(
        &mut self,
        canvas: &crate::CanvasContent,
        size: PixelSize,
    ) -> Result<WgpuCanvasContext<'_>, RenderError> {
        self.renderer.canvas_context(canvas, size)
    }

    pub fn get_canvas_context(
        &mut self,
        canvas: &crate::CanvasContent,
        size: PixelSize,
    ) -> Result<WgpuCanvasContext<'_>, RenderError> {
        self.renderer.get_canvas_context(canvas, size)
    }

    pub fn get_gpu_context(
        &mut self,
        canvas: &crate::CanvasContent,
        size: PixelSize,
    ) -> Result<WgpuCanvasContext<'_>, RenderError> {
        self.renderer.get_gpu_context(canvas, size)
    }

    fn render_to_surface(
        &mut self,
        frame: Option<wgpu::SurfaceTexture>,
        size: PixelSize,
        clear_color: ColorRgba,
        collect_gpu_timing: bool,
    ) -> Result<Option<Duration>, RenderError> {
        let Some(frame) = frame else {
            return Ok(None);
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let context = self.renderer.context.as_mut().ok_or_else(|| {
            RenderError::Backend("wgpu surface renderer missing context".to_string())
        })?;
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("operad-wgpu-surface-encoder"),
            });
        let gpu_timer_used = record_render_pass(
            context,
            &mut encoder,
            &view,
            self.target.surface_config.format,
            size,
            &self.renderer.geometry,
            WgpuRenderLoadOp::Clear(clear_color),
            true,
            collect_gpu_timing,
            false,
        )?;
        context.queue.submit(Some(encoder.finish()));
        frame.present();
        if gpu_timer_used {
            context.read_gpu_render_duration()
        } else {
            Ok(None)
        }
    }
}

impl WgpuSurfaceTarget<'_> {
    fn acquire(
        &mut self,
        context: &WgpuContext,
        size: PixelSize,
    ) -> Result<Option<wgpu::SurfaceTexture>, RenderError> {
        if size.width == 0 || size.height == 0 {
            return Ok(None);
        }

        self.configure_surface(context, size)?;
        let mut acquired = self.surface.get_current_texture();
        if matches!(acquired, wgpu::CurrentSurfaceTexture::Outdated) {
            // A surface can become outdated without changing pixel dimensions.
            self.surface_needs_reconfigure = true;
            self.configure_surface(context, size)?;
            acquired = self.surface.get_current_texture();
        }
        let frame = match acquired {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                // Reconfigure only after this texture has been presented/dropped.
                self.surface_needs_reconfigure = true;
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                return Err(RenderError::SurfaceUnavailable(
                    "surface acquire timed out".to_string(),
                ));
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface_needs_reconfigure = true;
                return Err(RenderError::SurfaceUnavailable(
                    "surface remained outdated after reconfiguration".to_string(),
                ));
            }
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(None),
            wgpu::CurrentSurfaceTexture::Lost => {
                return Err(RenderError::Backend(
                    "surface was lost and must be recreated before rendering".to_string(),
                ));
            }
            other => {
                return Err(RenderError::Backend(format!(
                    "surface acquire failed: {other:?}"
                )));
            }
        };

        Ok(Some(frame))
    }

    fn configure_surface(
        &mut self,
        context: &WgpuContext,
        size: PixelSize,
    ) -> Result<(), RenderError> {
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        validate_texture_size(size, context.limits.max_texture_dimension_2d, "surface")?;

        let needs_resize =
            self.surface_config.width != size.width || self.surface_config.height != size.height;
        if !self
            .surface_config
            .usage
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            self.surface_config
                .usage
                .insert(wgpu::TextureUsages::RENDER_ATTACHMENT);
        }
        if needs_resize {
            self.surface_config.width = size.width;
            self.surface_config.height = size.height;
        }
        if needs_resize || self.surface_needs_reconfigure {
            self.surface
                .configure(&context.device, &self.surface_config);
            self.surface_needs_reconfigure = false;
        }
        Ok(())
    }
}

impl<'window> RendererAdapter for WgpuSurfaceRenderer<'window> {
    fn capabilities(&self) -> BackendCapabilities {
        self.renderer.capabilities()
    }

    fn render_frame(
        &mut self,
        request: RenderFrameRequest,
        resolver: &dyn ResourceResolver,
    ) -> Result<RenderFrameOutput, RenderError> {
        let target_kind = request.target.kind();
        if !matches!(
            target_kind,
            RenderTargetKind::Window | RenderTargetKind::AppOwned
        ) {
            return self.renderer.render_frame(request, resolver);
        }

        let clear_color = request.options.clear_color;
        let collect_gpu_timing = request.options.collect_gpu_timing;
        let prepared = self
            .renderer
            .prepare_frame(request, resolver, |context, size| {
                self.target.acquire(context, size)
            })?;
        let mut output = prepared.output;
        let render_started = Instant::now();
        let gpu_render_duration = self.render_to_surface(
            prepared.acquired,
            prepared.size,
            clear_color,
            collect_gpu_timing,
        )?;
        output.timings = output.timings.section(
            "render",
            prepared.acquire_duration + render_started.elapsed(),
        );
        if let Some(duration) = gpu_render_duration {
            output.timings = output.timings.section("gpu-render", duration);
        }
        Ok(output)
    }
}

impl RendererAdapter for WgpuRenderer {
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::new("wgpu")
            .adapter(BackendAdapterKind::Wgpu)
            .resources(ResourceCapabilities {
                images: true,
                icons: true,
                textures: true,
                thumbnails: true,
                tinted_icons: true,
                partial_texture_updates: true,
            })
            .layers(LayerCapabilities::STANDARD)
            .services(PlatformServiceCapabilities::NONE)
            .rendering(RenderingCapabilities {
                high_dpi: true,
                offscreen: true,
                deterministic_snapshots: true,
                partial_updates: true,
                webgpu_surface: true,
                app_owned_view_rendering: true,
                native_child_windows: false,
                platform_overlays: false,
            })
            .accessibility(AccessibilityCapabilities::NONE)
    }

    fn render_frame(
        &mut self,
        request: RenderFrameRequest,
        resolver: &dyn ResourceResolver,
    ) -> Result<RenderFrameOutput, RenderError> {
        let target_kind = request.target.kind();
        let clear_color = request.options.clear_color;
        let collect_gpu_timing = request.options.collect_gpu_timing;
        let prepared = self.prepare_frame(request, resolver, |_, _| Ok(()))?;
        let size = prepared.size;
        let mut output = prepared.output;
        let render_started = Instant::now();
        let mut gpu_render_duration = None;
        if matches!(
            target_kind,
            RenderTargetKind::Snapshot | RenderTargetKind::Offscreen
        ) {
            let snapshot = self.render_snapshot(size, clear_color)?;
            output.snapshot = Some(RenderedImage::new(size, ResourceFormat::Rgba8, snapshot));
        } else {
            gpu_render_duration =
                self.render_discard_frame(size, clear_color, collect_gpu_timing)?;
        }
        output.timings = output.timings.section("render", render_started.elapsed());
        if let Some(duration) = gpu_render_duration {
            output.timings = output.timings.section("gpu-render", duration);
        }
        Ok(output)
    }
}

fn render_embedded_canvas_programs(
    context: &mut WgpuContext,
    request: &RenderFrameRequest,
    mut encoder: Option<&mut wgpu::CommandEncoder>,
) -> Result<(), RenderError> {
    for (item, canvas) in request.canvas_items() {
        let Some(program) = canvas.program.as_ref() else {
            continue;
        };
        let size = canvas_surface_size(item.rect, request.options.scale_factor);
        let canvas_context = context.canvas_context(canvas, size)?;
        let descriptor = embedded_canvas_render_pass(program);
        let result = if let Some(encoder) = encoder.as_deref_mut() {
            canvas_context.record_render_pass(encoder, descriptor)
        } else {
            canvas_context.render_pass(descriptor)
        };
        if result.is_err() {
            let color = ColorRgba::new(88, 20, 34, 255);
            if let Some(encoder) = encoder.as_deref_mut() {
                drop(canvas_context.begin_render_pass(encoder, Some(color)));
            } else {
                canvas_context.clear(color);
            }
        }
    }
    Ok(())
}

fn embedded_canvas_render_pass(program: &crate::CanvasRenderProgram) -> WgpuCanvasRenderPass<'_> {
    let mut pass = WgpuCanvasRenderPass::wgsl(Cow::Borrowed(program.wgsl.as_str()))
        .label(program.label.as_deref())
        .vertex_entry_point(program.vertex_entry_point.as_str())
        .fragment_entry_point(program.fragment_entry_point.as_str())
        .clear_color(program.clear_color)
        .constants(
            program
                .constants
                .iter()
                .map(|constant| (constant.name.as_str(), constant.value)),
        );
    if let Some(uniforms) = program.uniforms.as_deref() {
        pass = pass.uniform_bytes(Cow::Borrowed(uniforms));
    }
    pass
}

fn canvas_surface_size(rect: UiRect, scale_factor: f32) -> PixelSize {
    let scale_factor = normalized_render_scale(scale_factor);
    let width = finite_canvas_extent(rect.width * scale_factor);
    let height = finite_canvas_extent(rect.height * scale_factor);
    PixelSize::new(width, height)
}

fn finite_canvas_extent(value: f32) -> u32 {
    if !value.is_finite() {
        return 1;
    }
    value.ceil().clamp(1.0, u32::MAX as f32) as u32
}

fn record_discard_frame(
    context: &mut WgpuContext,
    encoder: &mut wgpu::CommandEncoder,
    size: PixelSize,
    geometry: &RenderGeometry,
    clear_color: ColorRgba,
    collect_gpu_timing: bool,
) -> Result<bool, RenderError> {
    if size.width == 0 || size.height == 0 {
        return Ok(false);
    }
    let view = context.discard_view(size, OFFSCREEN_FORMAT)?;
    record_render_pass(
        context,
        encoder,
        &view,
        OFFSCREEN_FORMAT,
        size,
        geometry,
        WgpuRenderLoadOp::Clear(clear_color),
        true,
        collect_gpu_timing,
        false,
    )
}

fn render_snapshot_with_context(
    context: &mut WgpuContext,
    size: PixelSize,
    geometry: &RenderGeometry,
    clear_color: ColorRgba,
) -> Result<Vec<u8>, RenderError> {
    let pixel_bytes = render_byte_len(size)?;
    if pixel_bytes == 0 {
        return Ok(Vec::new());
    }

    let padded_row_stride = upload_row_stride(size.width)?;
    let padded_size = u64::from(padded_row_stride)
        .checked_mul(u64::from(size.height))
        .ok_or_else(|| RenderError::Backend("wgpu readback buffer too large".to_string()))?;
    if padded_size > context.limits.max_buffer_size {
        return Err(RenderError::Backend(format!(
            "wgpu snapshot readback requires {padded_size} bytes, exceeding device buffer limit {}",
            context.limits.max_buffer_size
        )));
    }
    let texture = context.create_texture_2d(
        "operad-wgpu-snapshot-texture",
        size,
        OFFSCREEN_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    )?;
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("operad-wgpu-snapshot-encoder"),
        });
    record_render_pass(
        context,
        &mut encoder,
        &view,
        OFFSCREEN_FORMAT,
        size,
        geometry,
        WgpuRenderLoadOp::Clear(clear_color),
        true,
        false,
        false,
    )?;

    let readback_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("operad-wgpu-readback-buffer"),
        size: padded_size,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &readback_buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row_stride),
                rows_per_image: Some(size.height),
            },
        },
        Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
    );

    context.queue.submit(Some(encoder.finish()));
    let _ = context
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| RenderError::Backend(format!("wgpu poll failed: {error}")))?;

    let readback_slice = readback_buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    readback_slice.map_async(wgpu::MapMode::Read, move |status| {
        let _ = tx.send(status);
    });
    let _ = context
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| RenderError::Backend(format!("wgpu poll failed: {error}")))?;
    rx.recv()
        .map_err(|_| RenderError::Backend("wgpu map wait failed".to_string()))?
        .map_err(|error| RenderError::Backend(format!("wgpu map error: {error:?}")))?;

    let mapped = readback_slice.get_mapped_range();
    let row_bytes = usize::try_from(size.width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or_else(|| RenderError::Backend("wgpu readback row overflow".to_string()))?;
    let padded_row_stride = usize::try_from(padded_row_stride)
        .map_err(|_| RenderError::Backend("wgpu readback stride overflow".to_string()))?;
    let height = usize::try_from(size.height)
        .map_err(|_| RenderError::Backend("wgpu readback height overflow".to_string()))?;
    let mut pixels = vec![0_u8; pixel_bytes];
    for row in 0..height {
        let source_start = row
            .checked_mul(padded_row_stride)
            .ok_or_else(|| RenderError::Backend("wgpu readback source overflow".to_string()))?;
        let destination_start = row.checked_mul(row_bytes).ok_or_else(|| {
            RenderError::Backend("wgpu readback destination overflow".to_string())
        })?;
        pixels[destination_start..destination_start + row_bytes]
            .copy_from_slice(&mapped[source_start..source_start + row_bytes]);
    }
    drop(mapped);
    readback_buffer.unmap();

    Ok(pixels)
}

#[allow(clippy::too_many_arguments)]
fn record_render_pass(
    context: &mut WgpuContext,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    format: TextureFormat,
    size: PixelSize,
    geometry: &RenderGeometry,
    load_op: WgpuRenderLoadOp,
    render_text: bool,
    collect_gpu_timing: bool,
    encode_uploads: bool,
) -> Result<bool, RenderError> {
    if size.width == 0 || size.height == 0 {
        return Ok(false);
    }
    validate_texture_size(
        size,
        context.limits.max_texture_dimension_2d,
        "render target",
    )?;

    // Queue writes precede all commands in a submission. Caller-owned passes
    // instead copy their data immediately before drawing so later recordings
    // cannot overwrite an earlier pass's shared buffers.
    let mut uploads = encode_uploads.then_some(&mut *encoder);
    context.write_scene_uniform(size, uploads.as_deref_mut());
    let text_batch_chunks = if render_text {
        context.prepare_glyphon_text_batches(size, format, geometry)?
    } else {
        vec![Vec::new(); geometry.batches.len()]
    };
    let bind_group = context.scene_bind_group.clone();
    let textured_rect_buffer =
        context.textured_rect_buffer_for(&geometry.textured_rects, uploads.as_deref_mut());
    let composited_rect_buffer =
        context.composited_rect_buffer_for(&geometry.composited_rects, uploads.as_deref_mut());
    let sdf_buffers = context.sdf_buffers_for(
        &geometry.shapes,
        &geometry.gradient_stops,
        uploads.as_deref_mut(),
    )?;
    let vertex_buffer = context.vertex_buffer_for(&geometry.vertices, uploads.as_deref_mut());
    let texture_bind_groups = geometry
        .batches
        .iter()
        .filter_map(|batch| batch.texture_key.as_ref())
        .filter_map(|key| {
            context
                .texture(key)
                .map(|texture| (key, texture.bind_group.clone()))
        })
        .collect::<HashMap<_, _>>();

    let load = match load_op {
        WgpuRenderLoadOp::Clear(color) => wgpu::LoadOp::Clear(wgpu_color_for_format(color, format)),
        WgpuRenderLoadOp::Load => wgpu::LoadOp::Load,
    };
    let triangle_pipeline = context.triangle_pipeline(format).clone();
    let textured_rect_pipeline = context.textured_rect_pipeline(format).clone();
    let composited_rect_pipeline = context.composited_rect_pipeline(format).clone();
    let mut sdf_pipelines = HashMap::new();
    for batch in &geometry.batches {
        if let GeometryBatchKind::Sdf(kind) = batch.kind {
            sdf_pipelines
                .entry(kind)
                .or_insert_with(|| context.sdf_pipeline(format, kind).clone());
        }
    }
    if collect_gpu_timing && context.has_pending_gpu_timing() {
        return Err(RenderError::Backend(
            "resolve the previous WGPU GPU timing token before recording another timed pass"
                .to_string(),
        ));
    }
    let gpu_timer = if collect_gpu_timing {
        context.gpu_timer.as_ref()
    } else {
        None
    };
    let timestamp_writes = gpu_timer.map(|timer| wgpu::RenderPassTimestampWrites {
        query_set: &timer.query_set,
        beginning_of_pass_write_index: Some(0),
        end_of_pass_write_index: Some(1),
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("operad-wgpu-ui-render-pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes,
        multiview_mask: None,
    });
    for (batch_index, batch) in geometry.batches.iter().enumerate() {
        let Some(scissor) = scissor_rect(batch.clip, size) else {
            continue;
        };
        pass.set_scissor_rect(scissor.x, scissor.y, scissor.width, scissor.height);
        pass.set_bind_group(0, &bind_group, &[]);
        match batch.kind {
            GeometryBatchKind::Sdf(kind) => {
                let Some((buffer, gradients)) = &sdf_buffers else {
                    continue;
                };
                pass.set_pipeline(&sdf_pipelines[&kind]);
                pass.set_bind_group(1, gradients, &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..6, batch.start..batch.start + batch.count);
            }
            GeometryBatchKind::Triangle => {
                let Some(vertex_buffer) = &vertex_buffer else {
                    continue;
                };
                pass.set_pipeline(&triangle_pipeline);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.draw(batch.start..batch.start + batch.count, 0..1);
            }
            GeometryBatchKind::TexturedRect => {
                let Some(textured_rect_buffer) = &textured_rect_buffer else {
                    continue;
                };
                let Some(texture_key) = batch.texture_key.as_ref() else {
                    continue;
                };
                let Some(texture_bind_group) = texture_bind_groups.get(texture_key) else {
                    continue;
                };
                pass.set_pipeline(&textured_rect_pipeline);
                pass.set_bind_group(1, texture_bind_group, &[]);
                pass.set_vertex_buffer(0, textured_rect_buffer.slice(..));
                pass.draw(0..6, batch.start..batch.start + batch.count);
            }
            GeometryBatchKind::CompositedRect => {
                let Some(composited_rect_buffer) = &composited_rect_buffer else {
                    continue;
                };
                let Some(texture_key) = batch.texture_key.as_ref() else {
                    continue;
                };
                let Some(texture_bind_group) = texture_bind_groups.get(texture_key) else {
                    continue;
                };
                pass.set_pipeline(&composited_rect_pipeline);
                pass.set_bind_group(1, texture_bind_group, &[]);
                pass.set_vertex_buffer(0, composited_rect_buffer.slice(..));
                pass.draw(0..6, batch.start..batch.start + batch.count);
            }
            GeometryBatchKind::Text => {
                context.render_glyphon_text_chunks(&mut pass, &text_batch_chunks[batch_index])?;
            }
        }
    }
    drop(pass);
    if let Some(timer) = gpu_timer {
        encoder.resolve_query_set(&timer.query_set, 0..2, &timer.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(
            &timer.resolve_buffer,
            0,
            &timer.readback_buffer,
            0,
            GPU_TIMESTAMP_QUERY_BYTES,
        );
    }
    Ok(gpu_timer.is_some())
}

fn build_geometry_into(
    geometry: &mut RenderGeometry,
    paint: &crate::PaintList,
    context: &mut WgpuContext,
    origin: UiPoint,
    target_scale: f32,
    mut encoder: Option<&mut wgpu::CommandEncoder>,
) -> Result<(), RenderError> {
    let target_scale = normalized_render_scale(target_scale);
    let occluded = paint_occlusion_mask(paint, origin, target_scale);
    for (index, item) in paint.items.iter().enumerate() {
        if occluded.get(index).copied().unwrap_or(false) {
            continue;
        }
        let clip = paint_rect_in_target(item.clip_rect, origin, target_scale);
        let transform = paint_transform_in_target(item.transform, origin, target_scale);
        if let Some(shader) =
            paint_item_shader(item).filter(|shader| shader_effect_is_supported(shader))
        {
            push_shadered_paint_item(
                geometry,
                item,
                shader,
                transform,
                clip,
                context,
                encoder.as_deref_mut(),
            )?;
            continue;
        }
        match &item.kind {
            PaintKind::Rect {
                fill,
                stroke,
                corner_radius,
            } => {
                let rect = transform.transform_rect(item.rect);
                if let Some(shape) = SdfInstance::rectangle(
                    rect,
                    color_as_vertex(*fill, item.opacity),
                    CornerRadii::uniform(corner_radius * transform.scale),
                    stroke.map(|s| {
                        (
                            scaled_stroke(s, transform.scale),
                            crate::StrokeAlignment::Inside,
                        )
                    }),
                    item.opacity,
                ) {
                    geometry.push_shape(clip, shape);
                }
            }
            PaintKind::Text(text) => push_text(
                geometry,
                item.rect,
                clip,
                &text.text,
                &text.style,
                TextHorizontalAlign::Start,
                TextVerticalAlign::Top,
                item.opacity,
                transform,
            ),
            PaintKind::SceneText(text) => {
                let mut style = text.style.clone();
                style.overflow = text.overflow;
                if !text.multiline {
                    style.wrap = TextWrap::None;
                }
                push_text(
                    geometry,
                    text.rect,
                    clip,
                    &text.text,
                    &style,
                    text.horizontal_align,
                    text.vertical_align,
                    item.opacity,
                    transform,
                );
            }
            PaintKind::Canvas(canvas) => {
                push_canvas(
                    geometry,
                    transform.transform_rect(item.rect),
                    clip,
                    canvas,
                    item.opacity,
                    context,
                );
            }
            PaintKind::Line { from, to, stroke } => {
                push_line(
                    geometry,
                    transform.transform_point(*from),
                    transform.transform_point(*to),
                    clip,
                    scaled_stroke(*stroke, transform.scale),
                    item.opacity,
                );
            }
            PaintKind::Circle {
                center,
                radius,
                fill,
                stroke,
            } => {
                if let Some(shape) = SdfInstance::circle(
                    transform.transform_point(*center),
                    radius * transform.scale,
                    *fill,
                    stroke.map(|s| scaled_stroke(s, transform.scale)),
                    item.opacity,
                ) {
                    geometry.push_shape(clip, shape);
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
                    .map(|point| transform.transform_point(point))
                    .collect::<Vec<_>>();
                push_polygon(geometry, &points, clip, *fill, item.opacity);
                if let Some(stroke) = stroke.filter(|stroke| stroke.is_visible()) {
                    push_polyline(
                        geometry,
                        &points,
                        clip,
                        scaled_stroke(stroke, transform.scale),
                        item.opacity,
                        true,
                    );
                }
            }
            PaintKind::Image { key, tint } => {
                push_image(
                    geometry,
                    transform.transform_rect(item.rect),
                    clip,
                    key,
                    *tint,
                    item.opacity,
                    ImageFit::Fill,
                    ImageAlignment::Center,
                    ImageAlignment::Center,
                    context,
                );
            }
            PaintKind::CompositedLayer(layer) => {
                push_composited_layer(
                    geometry,
                    item,
                    transform,
                    layer,
                    clip,
                    None,
                    context,
                    encoder.as_deref_mut(),
                )?;
            }
            PaintKind::RichRect(primitive) => {
                push_rich_rect(geometry, primitive, clip, item.opacity, transform)?;
            }
            PaintKind::Path(path) => {
                if let Some(fill) = &path.fill {
                    push_triangle_mesh(
                        geometry,
                        &path.tessellated_fill(1.0),
                        transform,
                        clip,
                        fill.fallback_color(),
                        item.opacity,
                    );
                }
                if let Some(stroke) = path.stroke.filter(|stroke| stroke.is_visible()) {
                    push_triangle_mesh(
                        geometry,
                        &path.tessellated_stroke(1.0),
                        transform,
                        clip,
                        stroke.style.color,
                        item.opacity,
                    );
                }
            }
            PaintKind::ImagePlacement(image) => {
                push_image(
                    geometry,
                    transform.transform_rect(image.rect),
                    clip,
                    &image.key,
                    image.tint,
                    item.opacity,
                    image.fit,
                    image.horizontal_align,
                    image.vertical_align,
                    context,
                );
            }
        }
    }
    Ok(())
}

fn push_shadered_paint_item(
    geometry: &mut RenderGeometry,
    item: &crate::PaintItem,
    shader: &ShaderEffect,
    transform: crate::PaintTransform,
    clip: UiRect,
    context: &mut WgpuContext,
    encoder: Option<&mut wgpu::CommandEncoder>,
) -> Result<(), RenderError> {
    let material_outset = item
        .material
        .as_ref()
        .map(|material| material.visual_outset_for_rect(item.rect))
        .unwrap_or(0.0);
    let bounds = expanded_rect(item.rect, shader_effect_outset(shader).max(material_outset));
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return Ok(());
    }
    let mut child = item.clone();
    child.opacity = 1.0;
    child.transform = PaintTransform::default();
    child.shader = None;
    child.material = None;
    let mut layer = PaintCompositorLayer::new(bounds, crate::PaintList { items: vec![child] });
    if let Some(clip) = material_clip_for_item(item) {
        layer = layer.clip(clip);
    }
    let wrapper = crate::PaintItem {
        node: item.node,
        rect: bounds,
        clip_rect: item.clip_rect,
        z_index: item.z_index,
        layer_order: item.layer_order,
        opacity: item.opacity,
        transform: item.transform,
        shader: None,
        material: None,
        kind: PaintKind::CompositedLayer(layer.clone()),
    };
    push_composited_layer(
        geometry,
        &wrapper,
        transform,
        &layer,
        clip,
        Some(shader),
        context,
        encoder,
    )
}

fn paint_item_shader(item: &crate::PaintItem) -> Option<&ShaderEffect> {
    item.material
        .as_ref()
        .and_then(|material| material.shader.as_ref())
        .or(item.shader.as_ref())
}

fn material_clip_for_item(item: &crate::PaintItem) -> Option<CompositorClip> {
    let material = item.material.as_ref()?;
    match &material.clip_shape {
        crate::ElementShape::Rect => None,
        crate::ElementShape::RoundedRect { radius } => Some(CompositorClip::rounded_rect(
            item.rect,
            CornerRadii::uniform((*radius).max(0.0)),
        )),
        crate::ElementShape::Circle | crate::ElementShape::NormalizedPolygon(_) => None,
    }
}

fn paint_occlusion_mask(paint: &crate::PaintList, origin: UiPoint, target_scale: f32) -> Vec<bool> {
    const OCCLUSION_MIN_ITEMS: usize = 128;
    if paint.items.len() < OCCLUSION_MIN_ITEMS {
        return Vec::new();
    }

    let mut covered = Vec::<UiRect>::new();
    let mut occluded = vec![false; paint.items.len()];
    for (index, item) in paint.items.iter().enumerate().rev() {
        if let Some(rect) = paint_item_visible_rect_in_target(item, origin, target_scale) {
            if covered.iter().any(|cover| rect_contains_rect(*cover, rect)) {
                occluded[index] = true;
                continue;
            }
        }
        if let Some(rect) = opaque_cover_rect_for_item(item, origin, target_scale) {
            covered.push(rect);
        }
    }
    occluded
}

fn opaque_cover_rect_for_item(
    item: &crate::PaintItem,
    origin: UiPoint,
    target_scale: f32,
) -> Option<UiRect> {
    const OCCLUSION_COVER_MIN_AREA: f32 = 4096.0;
    let PaintKind::Rect {
        fill,
        corner_radius,
        ..
    } = &item.kind
    else {
        return None;
    };
    if fill.a < u8::MAX
        || item.opacity < 1.0
        || item.shader.is_some()
        || corner_radius.abs() > f32::EPSILON
    {
        return None;
    }
    let clip = paint_rect_in_target(item.clip_rect, origin, target_scale);
    let transform = paint_transform_in_target(item.transform, origin, target_scale);
    let bounds = transform.transform_rect(item.rect);
    let rect = UiRect::new(
        bounds.x + 0.5,
        bounds.y + 0.5,
        bounds.width - 1.0,
        bounds.height - 1.0,
    )
    .intersection(clip)?;
    (rect.width * rect.height >= OCCLUSION_COVER_MIN_AREA).then_some(rect)
}

fn paint_item_visible_rect_in_target(
    item: &crate::PaintItem,
    origin: UiPoint,
    target_scale: f32,
) -> Option<UiRect> {
    let clip = paint_rect_in_target(item.clip_rect, origin, target_scale);
    let transform = paint_transform_in_target(item.transform, origin, target_scale);
    let bounds_for = |shape: SdfInstance| {
        let [x, y, w, h] = shape.draw_rect;
        UiRect::new(x, y, w, h)
    };
    let bounds = match &item.kind {
        PaintKind::Circle {
            center,
            radius,
            fill,
            stroke,
        } => bounds_for(SdfInstance::circle(
            transform.transform_point(*center),
            radius * transform.scale,
            *fill,
            stroke.map(|s| scaled_stroke(s, transform.scale)),
            item.opacity,
        )?),
        PaintKind::Line { from, to, stroke } => bounds_for(SdfInstance::segment(
            transform.transform_point(*from),
            transform.transform_point(*to),
            scaled_stroke(*stroke, transform.scale),
            item.opacity,
        )?),
        PaintKind::RichRect(rect) => {
            let bounds = transform.transform_rect(rect.rect);
            let radii = scaled_corner_radii(rect.corner_radii, transform.scale);
            let stroke = rect
                .stroke
                .map(|s| (scaled_stroke(s.style, transform.scale), s.alignment));
            let mut visible = bounds_for(SdfInstance::rectangle(
                bounds,
                [0.0; 4],
                radii,
                stroke,
                item.opacity,
            )?);
            for effect in rect
                .effects
                .iter()
                .filter(|e| e.kind != PaintEffectKind::InsetShadow)
            {
                let effect = crate::PaintEffect {
                    offset: UiPoint::new(
                        effect.offset.x * transform.scale,
                        effect.offset.y * transform.scale,
                    ),
                    spread: effect.spread * transform.scale,
                    blur_radius: effect.blur_radius * transform.scale,
                    ..*effect
                };
                if let Some(shadow) = SdfInstance::shadow(bounds, radii, effect, item.opacity) {
                    let shadow = bounds_for(shadow);
                    let left = visible.x.min(shadow.x);
                    let top = visible.y.min(shadow.y);
                    visible = UiRect::new(
                        left,
                        top,
                        visible.right().max(shadow.right()) - left,
                        visible.bottom().max(shadow.bottom()) - top,
                    );
                }
            }
            visible
        }
        PaintKind::Polygon {
            stroke: Some(stroke),
            ..
        } => expanded_rect(
            transform.transform_rect(item.rect),
            1.0 + stroke.width * transform.scale * 0.5,
        ),
        _ => expanded_rect(transform.transform_rect(item.rect), 1.0),
    };
    bounds.intersection(clip)
}

fn rect_contains_rect(outer: UiRect, inner: UiRect) -> bool {
    const EPSILON: f32 = 0.0;
    inner.x + EPSILON >= outer.x
        && inner.y + EPSILON >= outer.y
        && inner.right() <= outer.right() + EPSILON
        && inner.bottom() <= outer.bottom() + EPSILON
}

#[derive(Debug, Clone, Copy)]
struct LayerFilterParams {
    blur_radius: f32,
    brightness: f32,
    contrast: f32,
    saturate: f32,
}

impl Default for LayerFilterParams {
    fn default() -> Self {
        Self {
            blur_radius: 0.0,
            brightness: 1.0,
            contrast: 1.0,
            saturate: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LayerShaderParams {
    params: [f32; 4],
    color: [f32; 4],
}

impl Default for LayerShaderParams {
    fn default() -> Self {
        Self {
            params: [0.0, 0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

fn paint_transform_in_target(
    mut transform: crate::PaintTransform,
    origin: UiPoint,
    target_scale: f32,
) -> crate::PaintTransform {
    let target_scale = normalized_render_scale(target_scale);
    transform.translation.x = (transform.translation.x - origin.x) * target_scale;
    transform.translation.y = (transform.translation.y - origin.y) * target_scale;
    transform.scale *= target_scale;
    transform
}

fn paint_rect_in_target(rect: UiRect, origin: UiPoint, target_scale: f32) -> UiRect {
    let target_scale = normalized_render_scale(target_scale);
    UiRect::new(
        (rect.x - origin.x) * target_scale,
        (rect.y - origin.y) * target_scale,
        rect.width * target_scale,
        rect.height * target_scale,
    )
}

fn normalized_render_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

fn layer_texture_size(bounds: UiRect) -> Result<PixelSize, RenderError> {
    if !bounds.width.is_finite() || !bounds.height.is_finite() {
        return Err(RenderError::Backend(
            "composited layer bounds must be finite".to_string(),
        ));
    }
    let width = bounds.width.ceil().max(1.0);
    let height = bounds.height.ceil().max(1.0);
    if width > u32::MAX as f32 || height > u32::MAX as f32 {
        return Err(RenderError::Backend(
            "composited layer bounds exceed u32 pixel dimensions".to_string(),
        ));
    }
    Ok(PixelSize::new(width as u32, height as u32))
}

fn push_composited_layer(
    geometry: &mut RenderGeometry,
    item: &crate::PaintItem,
    transform: crate::PaintTransform,
    layer: &PaintCompositorLayer,
    clip: UiRect,
    shader: Option<&ShaderEffect>,
    context: &mut WgpuContext,
    encoder: Option<&mut wgpu::CommandEncoder>,
) -> Result<(), RenderError> {
    let opacity = item.opacity * layer.opacity;
    if opacity <= 0.0 || layer.bounds.width <= 0.0 || layer.bounds.height <= 0.0 {
        return Ok(());
    }

    let rect = transform.transform_rect(layer.bounds);
    let Some(batch_clip) = composited_layer_scissor(rect, clip, layer, transform) else {
        return Ok(());
    };

    let (texture_key, texture_size) = render_composited_layer_to_texture(context, layer, encoder)?;
    let instance = GpuCompositedRectInstance::new(
        rect,
        [0.0, 0.0, 1.0, 1.0],
        opacity,
        layer_clip_for_shader(layer.clip.as_ref(), transform),
        layer_mask_for_shader(layer.mask.as_ref(), transform),
        layer_filter_params(layer),
        layer_shader_params(shader),
        texture_size,
    );
    geometry.push_composited_rect(batch_clip, texture_key, instance);
    Ok(())
}

fn render_composited_layer_to_texture(
    context: &mut WgpuContext,
    layer: &PaintCompositorLayer,
    mut encoder: Option<&mut wgpu::CommandEncoder>,
) -> Result<(WgpuTextureKey<'static>, PixelSize), RenderError> {
    let size = layer_texture_size(layer.bounds)?;
    validate_texture_size(
        size,
        context.limits.max_texture_dimension_2d,
        "composited layer",
    )?;
    let origin = UiPoint::new(layer.bounds.x, layer.bounds.y);
    let mut geometry = RenderGeometry::default();
    build_geometry_into(
        &mut geometry,
        &layer.paint,
        context,
        origin,
        1.0,
        encoder.as_deref_mut(),
    )?;

    let texture = context.create_texture_2d(
        "operad-wgpu-composited-layer-texture",
        size,
        OFFSCREEN_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    )?;
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encode_uploads = encoder.is_some();
    let mut owned_encoder = encoder.is_none().then(|| {
        context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("operad-wgpu-composited-layer-encoder"),
            })
    });
    let encoder = encoder.unwrap_or_else(|| owned_encoder.as_mut().unwrap());
    record_render_pass(
        context,
        encoder,
        &view,
        OFFSCREEN_FORMAT,
        size,
        &geometry,
        WgpuRenderLoadOp::Clear(ColorRgba::TRANSPARENT),
        true,
        false,
        encode_uploads,
    )?;
    if let Some(encoder) = owned_encoder {
        context.queue.submit(Some(encoder.finish()));
    }
    Ok((context.insert_layer_texture(size, texture, view), size))
}

fn composited_layer_scissor(
    rect: UiRect,
    clip: UiRect,
    layer: &PaintCompositorLayer,
    transform: crate::PaintTransform,
) -> Option<UiRect> {
    let mut scissor = rect.intersection(clip)?;
    if let Some(layer_clip) = &layer.clip {
        scissor = scissor.intersection(transform.transform_rect(layer_clip.bounds()))?;
    }
    if let Some(mask) = &layer.mask {
        scissor = scissor.intersection(transform.transform_rect(mask.bounds))?;
    }
    Some(scissor)
}

fn layer_clip_for_shader(
    clip: Option<&CompositorClip>,
    transform: crate::PaintTransform,
) -> Option<(UiRect, CornerRadii)> {
    match clip {
        Some(CompositorClip::Rect(rect)) => {
            Some((transform.transform_rect(*rect), CornerRadii::ZERO))
        }
        Some(CompositorClip::RoundedRect { rect, radii }) => Some((
            transform.transform_rect(*rect),
            scaled_corner_radii(*radii, transform.scale),
        )),
        None => None,
    }
}

fn layer_mask_for_shader(
    mask: Option<&CompositorMask>,
    transform: crate::PaintTransform,
) -> Option<UiRect> {
    mask.map(|mask| match mask.mode {
        MaskMode::Alpha | MaskMode::Luminance => transform.transform_rect(mask.bounds),
    })
}

fn layer_filter_params(layer: &PaintCompositorLayer) -> LayerFilterParams {
    let mut params = LayerFilterParams::default();
    for filter in &layer.filters {
        let amount = finite_or(filter.amount, 1.0).max(0.0);
        match filter.kind {
            CompositorFilterKind::Blur => params.blur_radius = amount,
            CompositorFilterKind::Brightness => params.brightness = amount,
            CompositorFilterKind::Contrast => params.contrast = amount,
            CompositorFilterKind::Saturate => params.saturate = amount,
            CompositorFilterKind::Custom => {}
        }
    }
    params
}

fn layer_shader_params(shader: Option<&ShaderEffect>) -> LayerShaderParams {
    let Some(shader) = shader else {
        return LayerShaderParams::default();
    };
    let Some(mode) = shader_effect_mode(shader) else {
        return LayerShaderParams::default();
    };
    match mode {
        1.0 => LayerShaderParams {
            params: [
                mode,
                shader_uniform(shader, "amount", 1.0).clamp(0.0, 1.0),
                0.0,
                0.0,
            ],
            color: shader_color(shader, ColorRgba::WHITE),
        },
        2.0 => LayerShaderParams {
            params: [
                mode,
                shader_uniform(shader, "amount", 0.35).max(0.0),
                shader_uniform(shader, "phase", 0.0),
                shader_uniform(shader, "width", 0.18),
            ],
            color: shader_color(shader, ColorRgba::WHITE),
        },
        3.0 => LayerShaderParams {
            params: [
                mode,
                shader_uniform(shader, "amount", 0.8).max(0.0),
                shader_uniform(shader, "radius", 8.0).max(1.0),
                0.0,
            ],
            color: shader_color(shader, ColorRgba::new(118, 183, 255, 255)),
        },
        mode if (4.0..=6.0).contains(&mode) => LayerShaderParams {
            params: [
                mode,
                shader_uniform(shader, "amount", 0.65).clamp(0.0, 1.0),
                shader_uniform(shader, "phase", 0.0),
                shader_uniform(shader, "scale", 8.0).max(1.0),
            ],
            color: shader_color(shader, ColorRgba::WHITE),
        },
        _ => LayerShaderParams::default(),
    }
}

fn shader_effect_mode(shader: &ShaderEffect) -> Option<f32> {
    match shader.key.as_str() {
        ShaderEffect::TINT => Some(1.0),
        ShaderEffect::SHINE => Some(2.0),
        ShaderEffect::GLOW => Some(3.0),
        ShaderEffect::PLASMA => Some(4.0),
        ShaderEffect::RINGS => Some(5.0),
        ShaderEffect::GRID => Some(6.0),
        key if key.ends_with(".tint") => Some(1.0),
        key if key.ends_with(".shine") => Some(2.0),
        key if key.ends_with(".glow") => Some(3.0),
        key if key.ends_with(".plasma") => Some(4.0),
        key if key.ends_with(".rings") => Some(5.0),
        key if key.ends_with(".grid") => Some(6.0),
        _ => None,
    }
}

fn shader_effect_is_supported(shader: &ShaderEffect) -> bool {
    shader_effect_mode(shader).is_some()
}

fn shader_uniform(shader: &ShaderEffect, name: &str, fallback: f32) -> f32 {
    shader
        .uniforms
        .iter()
        .rev()
        .find(|uniform| uniform.name == name)
        .map(|uniform| finite_or(uniform.value, fallback))
        .unwrap_or(fallback)
}

fn shader_color(shader: &ShaderEffect, fallback: ColorRgba) -> [f32; 4] {
    [
        shader_uniform(shader, "red", f32::from(fallback.r) / 255.0).clamp(0.0, 1.0),
        shader_uniform(shader, "green", f32::from(fallback.g) / 255.0).clamp(0.0, 1.0),
        shader_uniform(shader, "blue", f32::from(fallback.b) / 255.0).clamp(0.0, 1.0),
        shader_uniform(shader, "alpha", f32::from(fallback.a) / 255.0).clamp(0.0, 1.0),
    ]
}

fn shader_effect_outset(shader: &ShaderEffect) -> f32 {
    match shader_effect_mode(shader) {
        Some(3.0) => {
            shader_uniform(shader, "outset", shader_uniform(shader, "radius", 8.0)).clamp(0.0, 64.0)
        }
        _ => 0.0,
    }
}

fn scaled_stroke(stroke: StrokeStyle, scale: f32) -> StrokeStyle {
    StrokeStyle::new(stroke.color, stroke.width * scale)
}

fn push_rich_rect(
    geometry: &mut RenderGeometry,
    primitive: &crate::PaintRect,
    clip: UiRect,
    opacity: f32,
    transform: PaintTransform,
) -> Result<(), RenderError> {
    if opacity <= 0.0 {
        return Ok(());
    }
    let rect = transform.transform_rect(primitive.rect);
    let radii = scaled_corner_radii(primitive.corner_radii, transform.scale);
    let stroke = primitive
        .stroke
        .map(|s| (scaled_stroke(s.style, transform.scale), s.alignment));
    let has_inset = primitive
        .effects
        .iter()
        .any(|e| e.kind == PaintEffectKind::InsetShadow && e.color.a > 0);
    let effect = |e: crate::PaintEffect| crate::PaintEffect {
        offset: UiPoint::new(e.offset.x * transform.scale, e.offset.y * transform.scale),
        spread: e.spread * transform.scale,
        blur_radius: e.blur_radius * transform.scale,
        ..e
    };
    for e in primitive
        .effects
        .iter()
        .filter(|e| e.kind != PaintEffectKind::InsetShadow)
    {
        if let Some(shape) = SdfInstance::shadow(rect, radii, effect(*e), opacity) {
            geometry.push_shape(clip, shape);
        }
    }
    if let Some(mut shape) = SdfInstance::rectangle(
        rect,
        color_as_vertex(primitive.fill.fallback_color(), opacity),
        radii,
        if has_inset { None } else { stroke },
        opacity,
    )
    .filter(|shape| shape.intersects_clip(clip))
    {
        if let PaintBrush::LinearGradient(gradient) = &primitive.fill {
            let start = transform.transform_point(gradient.start);
            let end = transform.transform_point(gradient.end);
            shape.gradient_line = [
                finite_or(start.x, rect.x),
                finite_or(start.y, rect.y),
                finite_or(end.x, rect.x),
                finite_or(end.y, rect.y),
            ];
            let base = geometry.gradient_stops.len();
            let stop_end = base
                .checked_add(gradient.stops.len())
                .filter(|end| *end <= u32::MAX as usize)
                .ok_or_else(|| {
                    RenderError::Backend(
                        "SDF gradient stop count exceeds GPU indexing limits".into(),
                    )
                })?;
            geometry
                .gradient_stops
                .extend(gradient.stops.iter().map(|stop| SdfGradientStop {
                    position: [finite_or(stop.offset, 0.0).clamp(0.0, 1.0), 0.0, 0.0, 0.0],
                    color: color_as_vertex(stop.color, opacity),
                }));
            // Public paint data can be constructed directly, bypassing the sorted builder.
            let stops = &mut geometry.gradient_stops[base..stop_end];
            if !stops
                .windows(2)
                .all(|w| w[0].position[0] <= w[1].position[0])
            {
                stops.sort_by(|a, b| a.position[0].total_cmp(&b.position[0]));
            }
            shape.gradient_range = [base as u32, gradient.stops.len() as u32];
        }
        geometry.push_shape(clip, shape);
    }
    if has_inset {
        for e in primitive
            .effects
            .iter()
            .filter(|e| e.kind == PaintEffectKind::InsetShadow)
        {
            if let Some(shape) = SdfInstance::shadow(rect, radii, effect(*e), opacity) {
                geometry.push_shape(clip, shape);
            }
        }
        if let Some(shape) = SdfInstance::rectangle(rect, [0.0; 4], radii, stroke, opacity) {
            geometry.push_shape(clip, shape);
        }
    }
    Ok(())
}

fn push_fill_rect(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
) {
    push_fill_rect_with_color(geometry, rect, clip, color_as_vertex(color, opacity));
}

fn push_fill_rect_with_color(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    color: [f32; 4],
) {
    if let Some(shape) = SdfInstance::rectangle(rect, color, CornerRadii::ZERO, None, 1.0) {
        geometry.push_shape(clip, shape);
    }
}

fn scaled_corner_radii(radii: CornerRadii, scale: f32) -> CornerRadii {
    let scale = scale.max(0.0);
    CornerRadii::new(
        radii.top_left * scale,
        radii.top_right * scale,
        radii.bottom_right * scale,
        radii.bottom_left * scale,
    )
}

fn normalized_corner_radii_for_rect(radii: CornerRadii, width: f32, height: f32) -> CornerRadii {
    let mut radii = CornerRadii::new(
        finite_or(radii.top_left, 0.0).max(0.0),
        finite_or(radii.top_right, 0.0).max(0.0),
        finite_or(radii.bottom_right, 0.0).max(0.0),
        finite_or(radii.bottom_left, 0.0).max(0.0),
    );
    let width = finite_or(width, 0.0).max(0.0);
    let height = finite_or(height, 0.0).max(0.0);
    let mut scale: f32 = 1.0;
    for (sum, limit) in [
        (radii.top_left + radii.top_right, width),
        (radii.bottom_left + radii.bottom_right, width),
        (radii.top_left + radii.bottom_left, height),
        (radii.top_right + radii.bottom_right, height),
    ] {
        if sum > limit && sum > f32::EPSILON {
            scale = scale.min(limit / sum);
        }
    }
    if scale < 1.0 {
        radii.top_left *= scale;
        radii.top_right *= scale;
        radii.bottom_right *= scale;
        radii.bottom_left *= scale;
    }
    radii
}

#[allow(clippy::too_many_arguments)]
fn push_text(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    text: &str,
    style: &TextStyle,
    horizontal_align: TextHorizontalAlign,
    vertical_align: TextVerticalAlign,
    opacity: f32,
    transform: crate::PaintTransform,
) {
    let rect = transform.transform_rect(rect);
    if text.is_empty() || rect.width <= 0.0 || rect.height <= 0.0 || opacity <= 0.0 {
        return;
    }
    if style.color.a == 0 {
        return;
    }

    let scale = transform.scale.max(0.0);
    if scale <= f32::EPSILON || rect.intersection(clip).is_none() {
        return;
    }
    let mut style = style.clone();
    style.font_size = (style.font_size * scale).max(1.0);
    style.line_height = (style.line_height * scale).max(style.font_size);
    let clip = if style.overflow == TextOverflow::Ellipsis {
        // Advances fit the box; italic glyph ink can still overhang them.
        clip.intersection(rect).expect("visible text rectangle")
    } else {
        clip
    };
    geometry.push_text(TextPaint {
        rect,
        clip,
        text: text.to_owned(),
        style: style.clone(),
        horizontal_align,
        vertical_align,
        opacity,
    });
}

fn push_canvas(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    canvas: &crate::CanvasContent,
    opacity: f32,
    context: &WgpuContext,
) {
    if opacity <= 0.0 {
        return;
    }
    let surface_key = canvas.surface_key();
    if let Some(texture_size) = context
        .textures
        .get(surface_key)
        .map(|texture| texture.size)
    {
        if let Some((rect, uv)) = image_placement(
            rect,
            texture_size,
            ImageFit::Fill,
            ImageAlignment::Center,
            ImageAlignment::Center,
        ) {
            if rect.width > 0.0 && rect.height > 0.0 && rect.intersection(clip).is_some() {
                geometry.push_textured_rect(clip, surface_key, rect, uv, image_tint(None, opacity));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_image(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    key: &str,
    tint: Option<ColorRgba>,
    opacity: f32,
    fit: ImageFit,
    horizontal_align: ImageAlignment,
    vertical_align: ImageAlignment,
    context: &WgpuContext,
) {
    if opacity <= 0.0 {
        return;
    }
    if let Some(texture_size) = context.textures.get(key).map(|texture| texture.size) {
        if let Some((rect, uv)) =
            image_placement(rect, texture_size, fit, horizontal_align, vertical_align)
        {
            if rect.width > 0.0 && rect.height > 0.0 && rect.intersection(clip).is_some() {
                geometry.push_textured_rect(clip, key, rect, uv, image_tint(tint, opacity));
                return;
            }
        }
    }

    push_image_placeholder(geometry, rect, clip, key, tint, opacity);
}

fn push_image_placeholder(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    key: &str,
    tint: Option<ColorRgba>,
    opacity: f32,
) {
    if push_built_in_icon_fallback(geometry, rect, clip, key, tint, opacity) {
        return;
    }
    push_missing_image_checkerboard(geometry, rect, clip, opacity);
}

fn push_built_in_icon_fallback(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    key: &str,
    tint: Option<ColorRgba>,
    opacity: f32,
) -> bool {
    let Some(icon) = BuiltInIcon::from_key(key) else {
        return false;
    };
    let color = tint.unwrap_or(ColorRgba::WHITE);
    for path in icon.fallback_paths(rect, color) {
        if let Some(fill) = &path.fill {
            push_triangle_mesh(
                geometry,
                &path.tessellated_fill(1.0),
                PaintTransform::default(),
                clip,
                fill.fallback_color(),
                opacity,
            );
        }
        if let Some(stroke) = path.stroke.filter(|stroke| stroke.is_visible()) {
            push_triangle_mesh(
                geometry,
                &path.tessellated_stroke(1.0),
                PaintTransform::default(),
                clip,
                stroke.style.color,
                opacity,
            );
        }
    }
    true
}

fn push_missing_image_checkerboard(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    opacity: f32,
) {
    if rect.width <= 0.0 || rect.height <= 0.0 || opacity <= 0.0 {
        return;
    }
    let mut y = rect.y;
    let mut row = 0;
    while y < rect.bottom() {
        let height = MISSING_IMAGE_CHECKER_SIZE.min(rect.bottom() - y);
        let mut x = rect.x;
        let mut column = 0;
        while x < rect.right() {
            let width = MISSING_IMAGE_CHECKER_SIZE.min(rect.right() - x);
            let color = if (row + column) % 2 == 0 {
                MISSING_IMAGE_DARK
            } else {
                MISSING_IMAGE_PURPLE
            };
            push_fill_rect(
                geometry,
                UiRect::new(x, y, width, height),
                clip,
                color,
                opacity,
            );
            x += MISSING_IMAGE_CHECKER_SIZE;
            column += 1;
        }
        y += MISSING_IMAGE_CHECKER_SIZE;
        row += 1;
    }
}

fn image_placement(
    rect: UiRect,
    texture_size: PixelSize,
    fit: ImageFit,
    horizontal_align: ImageAlignment,
    vertical_align: ImageAlignment,
) -> Option<(UiRect, [f32; 4])> {
    if rect.width <= 0.0
        || rect.height <= 0.0
        || texture_size.width == 0
        || texture_size.height == 0
    {
        return None;
    }

    let image_width = texture_size.width as f32;
    let image_height = texture_size.height as f32;
    let align_x = alignment_factor(horizontal_align);
    let align_y = alignment_factor(vertical_align);
    match fit {
        ImageFit::Fill => Some((rect, [0.0, 0.0, 1.0, 1.0])),
        ImageFit::Contain | ImageFit::Original => {
            let scale = if fit == ImageFit::Original {
                1.0
            } else {
                (rect.width / image_width).min(rect.height / image_height)
            };
            let width = image_width * scale;
            let height = image_height * scale;
            Some((
                UiRect::new(
                    rect.x + (rect.width - width) * align_x,
                    rect.y + (rect.height - height) * align_y,
                    width,
                    height,
                ),
                [0.0, 0.0, 1.0, 1.0],
            ))
        }
        ImageFit::Cover => {
            let source_aspect = image_width / image_height;
            let target_aspect = rect.width / rect.height;
            let mut uv = [0.0, 0.0, 1.0, 1.0];
            if source_aspect > target_aspect {
                let visible_width = (target_aspect / source_aspect).clamp(0.0, 1.0);
                uv[0] = (1.0 - visible_width) * align_x;
                uv[2] = visible_width;
            } else if source_aspect < target_aspect {
                let visible_height = (source_aspect / target_aspect).clamp(0.0, 1.0);
                uv[1] = (1.0 - visible_height) * align_y;
                uv[3] = visible_height;
            }
            Some((rect, uv))
        }
    }
}

fn alignment_factor(alignment: ImageAlignment) -> f32 {
    match alignment {
        ImageAlignment::Start => 0.0,
        ImageAlignment::Center => 0.5,
        ImageAlignment::End => 1.0,
    }
}

fn image_tint(tint: Option<ColorRgba>, opacity: f32) -> [f32; 4] {
    match tint {
        Some(tint) => color_as_vertex(tint, opacity),
        None => [1.0, 1.0, 1.0, opacity.clamp(0.0, 1.0)],
    }
}

fn push_line(
    geometry: &mut RenderGeometry,
    from: UiPoint,
    to: UiPoint,
    clip: UiRect,
    stroke: StrokeStyle,
    opacity: f32,
) {
    if let Some(shape) = SdfInstance::segment(from, to, stroke, opacity) {
        geometry.push_shape(clip, shape);
    }
}

fn push_polygon(
    geometry: &mut RenderGeometry,
    points: &[UiPoint],
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
) {
    if points.len() < 3 || color.a == 0 || opacity <= 0.0 {
        return;
    }
    push_triangle_mesh(
        geometry,
        &tessellate_polygon_points(points),
        PaintTransform::default(),
        clip,
        color,
        opacity,
    );
}

fn push_triangle_mesh(
    geometry: &mut RenderGeometry,
    triangles: &[[UiPoint; 3]],
    transform: PaintTransform,
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
) {
    if triangles.is_empty() || color.a == 0 || opacity <= 0.0 {
        return;
    }
    let color = color_as_vertex(color, opacity);
    let mut vertices = Vec::with_capacity(triangles.len().saturating_mul(3));
    for triangle in triangles {
        vertices.extend_from_slice(&[
            GpuVertex::new(transform.transform_point(triangle[0]), color),
            GpuVertex::new(transform.transform_point(triangle[1]), color),
            GpuVertex::new(transform.transform_point(triangle[2]), color),
        ]);
    }
    geometry.push_triangle_vertices(clip, &vertices);
}

fn push_polyline(
    geometry: &mut RenderGeometry,
    points: &[UiPoint],
    clip: UiRect,
    stroke: StrokeStyle,
    opacity: f32,
    closed: bool,
) {
    for segment in points.windows(2) {
        push_line(geometry, segment[0], segment[1], clip, stroke, opacity);
    }
    if closed && points.len() > 2 {
        push_line(
            geometry,
            points[points.len() - 1],
            points[0],
            clip,
            stroke,
            opacity,
        );
    }
}

fn scissor_rect(clip: UiRect, size: PixelSize) -> Option<PixelRect> {
    let left = finite_or(clip.x.floor(), 0.0).max(0.0);
    let top = finite_or(clip.y.floor(), 0.0).max(0.0);
    let right = finite_or(clip.right().ceil(), size.width as f32).min(size.width as f32);
    let bottom = finite_or(clip.bottom().ceil(), size.height as f32).min(size.height as f32);
    if left >= right || top >= bottom {
        return None;
    }
    Some(PixelRect::new(
        left as u32,
        top as u32,
        (right - left) as u32,
        (bottom - top) as u32,
    ))
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn expanded_rect(rect: UiRect, amount: f32) -> UiRect {
    let amount = finite_or(amount, 0.0).max(0.0);
    UiRect::new(
        rect.x - amount,
        rect.y - amount,
        rect.width + amount * 2.0,
        rect.height + amount * 2.0,
    )
}

fn color_as_vertex(color: ColorRgba, opacity: f32) -> [f32; 4] {
    [
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        (f32::from(color.a) / 255.0 * opacity.clamp(0.0, 1.0)).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
fn default_glyph_font_system() -> GlyphFontSystem {
    glyph_font_system(&FontLibrary::default())
}

fn glyph_font_system(fonts: &FontLibrary) -> GlyphFontSystem {
    let mut font_system = GlyphFontSystem::new_with_fonts([
        embedded_glyph_font(epaint_default_fonts::UBUNTU_LIGHT),
        embedded_glyph_font(epaint_default_fonts::HACK_REGULAR),
        embedded_glyph_font(epaint_default_fonts::NOTO_EMOJI_REGULAR),
    ]);
    for font in fonts.fonts() {
        font_system
            .db_mut()
            .load_font_source(glyph_font_bytes_source(font));
    }
    {
        let db = font_system.db_mut();
        db.set_sans_serif_family(fonts.sans_serif_family().unwrap_or("Ubuntu"));
        db.set_serif_family(fonts.serif_family().unwrap_or("Ubuntu"));
        db.set_monospace_family(fonts.monospace_family().unwrap_or("Hack"));
    }
    font_system
}

fn embedded_glyph_font(bytes: &'static [u8]) -> glyphon::cosmic_text::fontdb::Source {
    let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bytes);
    glyphon::cosmic_text::fontdb::Source::Binary(data)
}

fn glyph_font_bytes_source(font: &crate::fonts::FontBytes) -> glyphon::cosmic_text::fontdb::Source {
    let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(font.bytes().to_vec());
    glyphon::cosmic_text::fontdb::Source::Binary(data)
}

fn sync_glyph_buffer(
    buffer: &mut GlyphBuffer,
    font_system: &mut GlyphFontSystem,
    text: &TextPaint,
    previous_key: Option<&TextBufferKey>,
    next_key: &TextBufferKey,
) {
    if !previous_key.is_some_and(|previous| previous.has_same_layout_as(next_key)) {
        let metrics = GlyphMetrics::new(
            text.style.font_size.max(1.0),
            text.style.line_height.max(text.style.font_size).max(1.0),
        );
        buffer.set_metrics_and_size(
            font_system,
            metrics,
            Some(text.rect.width.max(0.0)),
            Some(text.rect.height.max(0.0)),
        );
        buffer.set_wrap(
            font_system,
            if text.style.overflow == TextOverflow::Ellipsis {
                GlyphWrap::None
            } else {
                glyph_wrap(text.style.wrap)
            },
        );
    }
    let attrs = glyph_attrs(&text.style);
    let fitted;
    let value = if text.style.overflow == TextOverflow::Ellipsis {
        fitted = fit_glyph_text(font_system, text);
        fitted.text.as_str()
    } else {
        &text.text
    };
    buffer.set_text(
        font_system,
        value,
        &attrs,
        glyph_shaping(value),
        glyph_horizontal_align(text.horizontal_align),
    );
}

fn fit_glyph_text(
    font_system: &mut GlyphFontSystem,
    text: &TextPaint,
) -> crate::core::text::FittedText {
    let line_height = text.style.line_height.max(text.style.font_size).max(1.0);
    let mut measurement = GlyphBuffer::new(
        font_system,
        GlyphMetrics::new(text.style.font_size.max(1.0), line_height),
    );
    measurement.set_wrap(font_system, GlyphWrap::None);
    let attrs = glyph_attrs(&text.style);
    crate::core::text::fit_single_line(&text.text, text.rect.width, |value| {
        measurement.set_text(font_system, value, &attrs, glyph_shaping(value), None);
        let mut size = UiSize::new(0.0, line_height);
        for run in measurement.layout_runs() {
            size.width = size.width.max(run.line_w);
            size.height = size.height.max(run.line_top + run.line_height);
        }
        size
    })
}

fn glyph_horizontal_align(align: TextHorizontalAlign) -> Option<glyphon::cosmic_text::Align> {
    match align {
        TextHorizontalAlign::Start => None,
        TextHorizontalAlign::Center => Some(glyphon::cosmic_text::Align::Center),
        TextHorizontalAlign::End => Some(glyphon::cosmic_text::Align::Right),
    }
}

fn glyph_attrs(style: &TextStyle) -> GlyphAttrs<'_> {
    GlyphAttrs::new()
        .family(glyph_family(&style.family))
        .weight(glyph_weight(style.weight))
        .style(glyph_font_style(style.style))
        .stretch(glyph_stretch(style.stretch))
}

fn glyph_family(family: &FontFamily) -> GlyphFamily<'_> {
    match family {
        FontFamily::SansSerif => GlyphFamily::SansSerif,
        FontFamily::Serif => GlyphFamily::Serif,
        FontFamily::Monospace => GlyphFamily::Monospace,
        FontFamily::Named(name) => GlyphFamily::Name(name),
    }
}

fn glyph_weight(weight: crate::FontWeight) -> GlyphWeight {
    GlyphWeight(weight.value())
}

fn glyph_font_style(style: FontStyle) -> GlyphFontStyle {
    match style {
        FontStyle::Normal => GlyphFontStyle::Normal,
        FontStyle::Italic => GlyphFontStyle::Italic,
        FontStyle::Oblique => GlyphFontStyle::Oblique,
    }
}

fn glyph_stretch(stretch: FontStretch) -> GlyphStretch {
    match stretch {
        FontStretch::Condensed => GlyphStretch::Condensed,
        FontStretch::Normal => GlyphStretch::Normal,
        FontStretch::Expanded => GlyphStretch::Expanded,
    }
}

fn glyph_wrap(wrap: TextWrap) -> GlyphWrap {
    match wrap {
        TextWrap::None => GlyphWrap::None,
        TextWrap::Glyph => GlyphWrap::Glyph,
        TextWrap::Word => GlyphWrap::Word,
        TextWrap::WordOrGlyph => GlyphWrap::WordOrGlyph,
    }
}

fn glyph_text_area_top(text: &TextPaint, buffer: &GlyphBuffer) -> f32 {
    let content_height = glyph_text_content_height(buffer);
    let slack = (text.rect.height - content_height).max(0.0);
    text.rect.y
        + match text.vertical_align {
            TextVerticalAlign::Top | TextVerticalAlign::Baseline => 0.0,
            TextVerticalAlign::Center => slack * 0.5,
            TextVerticalAlign::Bottom => slack,
        }
}

fn glyph_text_content_height(buffer: &GlyphBuffer) -> f32 {
    buffer
        .layout_runs()
        .map(|run| run.line_top + run.line_height)
        .fold(0.0, f32::max)
}

fn glyph_shaping(text: &str) -> GlyphShaping {
    if text.is_ascii() {
        GlyphShaping::Basic
    } else {
        GlyphShaping::Advanced
    }
}

fn glyph_color(color: ColorRgba, opacity: f32) -> GlyphColor {
    let alpha = (f32::from(color.a) * opacity.clamp(0.0, 1.0)).round();
    GlyphColor::rgba(color.r, color.g, color.b, alpha.clamp(0.0, 255.0) as u8)
}

fn glyph_text_bounds(clip: UiRect, size: PixelSize) -> GlyphTextBounds {
    let Some(scissor) = scissor_rect(clip, size) else {
        return GlyphTextBounds {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
    };
    GlyphTextBounds {
        left: scissor.x as i32,
        top: scissor.y as i32,
        right: scissor.x.saturating_add(scissor.width) as i32,
        bottom: scissor.y.saturating_add(scissor.height) as i32,
    }
}

fn encode_texture_upload(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    destination: TexelCopyTextureInfo<'_>,
    rect: PixelRect,
    rgba: &[u8],
) -> Result<(), RenderError> {
    if rect.width == 0 || rect.height == 0 {
        return Ok(());
    }
    // Encoded copies need aligned rows. Tile large uploads so padding never
    // makes a valid texture update exceed the device's staging-buffer limit.
    let budget = device.limits().max_buffer_size.min(usize::MAX as u64);
    let columns = (budget / 4).min(u64::from(rect.width)) as u32;
    if columns == 0 {
        return Err(RenderError::Backend(
            "device buffer limit cannot hold one image pixel".into(),
        ));
    }
    let source_stride = rect.width as usize * 4;
    for x in (0..rect.width).step_by(columns as usize) {
        let width = columns.min(rect.width - x);
        let row_bytes = u64::from(width) * 4;
        let stride = upload_row_stride(width)?;
        let rows =
            (1 + (budget - row_bytes) / u64::from(stride)).min(u64::from(rect.height)) as u32;
        for y in (0..rect.height).step_by(rows as usize) {
            let height = rows.min(rect.height - y);
            let byte_len = u64::from(stride) * u64::from(height - 1) + row_bytes;
            let source_start = y as usize * source_stride + x as usize * 4;
            let bytes = if height == 1 || (width == rect.width && u64::from(stride) == row_bytes) {
                Cow::Borrowed(&rgba[source_start..source_start + byte_len as usize])
            } else {
                let mut padded = vec![0; byte_len as usize];
                for row in 0..height as usize {
                    let start = source_start + row * source_stride;
                    let destination = row * stride as usize;
                    padded[destination..destination + row_bytes as usize]
                        .copy_from_slice(&rgba[start..start + row_bytes as usize]);
                }
                Cow::Owned(padded)
            };
            let upload = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("operad-wgpu-image-upload"),
                contents: &bytes,
                usage: BufferUsages::COPY_SRC,
            });
            encoder.copy_buffer_to_texture(
                wgpu::TexelCopyBufferInfo {
                    buffer: &upload,
                    layout: TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(height),
                    },
                },
                TexelCopyTextureInfo {
                    texture: destination.texture,
                    mip_level: destination.mip_level,
                    origin: Origin3d {
                        x: destination.origin.x + x,
                        y: destination.origin.y + y,
                        z: destination.origin.z,
                    },
                    aspect: destination.aspect,
                },
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
    Ok(())
}

fn glyph_prepare_error(error: GlyphPrepareError) -> RenderError {
    RenderError::Backend(format!("glyphon text prepare failed: {error}"))
}

fn glyph_render_error(error: GlyphRenderError) -> RenderError {
    RenderError::Backend(format!("glyphon text render failed: {error}"))
}

fn wgpu_color_for_format(color: ColorRgba, format: TextureFormat) -> wgpu::Color {
    if format.is_srgb() {
        wgpu::Color {
            r: srgb_channel_to_linear(color.r) as f64,
            g: srgb_channel_to_linear(color.g) as f64,
            b: srgb_channel_to_linear(color.b) as f64,
            a: f64::from(color.a) / 255.0,
        }
    } else {
        wgpu_color(color)
    }
}

fn wgpu_color(color: ColorRgba) -> wgpu::Color {
    wgpu::Color {
        r: f64::from(color.r) / 255.0,
        g: f64::from(color.g) / 255.0,
        b: f64::from(color.b) / 255.0,
        a: f64::from(color.a) / 255.0,
    }
}

fn srgb_channel_to_linear(value: u8) -> f32 {
    let value = f32::from(value) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn main_fragment_entry_point(format: TextureFormat) -> &'static str {
    if format.is_srgb() {
        "fs_main_srgb"
    } else {
        "fs_main"
    }
}

fn textured_fragment_entry_point(format: TextureFormat) -> &'static str {
    if format.is_srgb() {
        "fs_textured_srgb"
    } else {
        "fs_textured"
    }
}

fn composited_fragment_entry_point(format: TextureFormat) -> &'static str {
    if format.is_srgb() {
        "fs_composited_srgb"
    } else {
        "fs_composited"
    }
}

fn glyph_color_mode(format: TextureFormat) -> GlyphColorMode {
    if format.is_srgb() {
        GlyphColorMode::Accurate
    } else {
        GlyphColorMode::Web
    }
}

fn render_target_pixel_size(
    target: &RenderTarget,
    viewport: UiSize,
    scale_factor: f32,
) -> Result<PixelSize, RenderError> {
    match target {
        RenderTarget::Offscreen { size, .. } | RenderTarget::Snapshot { size, .. } => Ok(*size),
        RenderTarget::Window { .. } | RenderTarget::AppOwned { .. } => {
            pixel_size_from_viewport(viewport, scale_factor)
        }
    }
}

fn validate_texture_size(size: PixelSize, limit: u32, label: &str) -> Result<(), RenderError> {
    if size.width == 0 || size.height == 0 {
        return Err(RenderError::Backend(format!(
            "wgpu {label} requires non-zero dimensions"
        )));
    }
    if size.width > limit || size.height > limit {
        return Err(RenderError::Backend(format!(
            "wgpu {label} dimensions {}x{} exceed device texture limit {limit}x{limit}",
            size.width, size.height
        )));
    }
    Ok(())
}

fn pixel_size_from_viewport(viewport: UiSize, scale_factor: f32) -> Result<PixelSize, RenderError> {
    if !viewport.width.is_finite() || !viewport.height.is_finite() {
        return Err(RenderError::Backend(
            "snapshot viewport must be finite".to_string(),
        ));
    }
    if viewport.width < 0.0 || viewport.height < 0.0 {
        return Err(RenderError::Backend(
            "snapshot viewport must be non-negative".to_string(),
        ));
    }
    let scale_factor = normalized_render_scale(scale_factor);
    let width = viewport.width * scale_factor;
    let height = viewport.height * scale_factor;
    if width.round() > u32::MAX as f32 || height.round() > u32::MAX as f32 {
        return Err(RenderError::Backend(
            "snapshot viewport exceeds u32 pixel dimensions".to_string(),
        ));
    }
    Ok(PixelSize::new(width.round() as u32, height.round() as u32))
}

fn render_byte_len(size: PixelSize) -> Result<usize, RenderError> {
    let width = usize::try_from(size.width)
        .map_err(|_| RenderError::Backend("render width overflow".to_string()))?;
    let height = usize::try_from(size.height)
        .map_err(|_| RenderError::Backend("render height overflow".to_string()))?;
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| RenderError::Backend("render target byte length overflow".to_string()))
}

fn upload_row_stride(width: u32) -> Result<u32, RenderError> {
    let row_bytes = u64::from(width)
        .checked_mul(4)
        .ok_or_else(|| RenderError::Backend("surface row stride overflow".to_string()))?;
    let aligned_stride = row_bytes.div_ceil(u64::from(COPY_BYTES_PER_ROW_ALIGNMENT));
    let aligned_row_bytes = aligned_stride * u64::from(COPY_BYTES_PER_ROW_ALIGNMENT);
    u32::try_from(aligned_row_bytes)
        .map_err(|_| RenderError::Backend("surface row stride overflow".to_string()))
}

fn pack_scene_uniform(size: PixelSize) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    append_f32_to_buffer(&mut bytes[0..4], size.width.max(1) as f32);
    append_f32_to_buffer(&mut bytes[4..8], size.height.max(1) as f32);
    append_f32_to_buffer(&mut bytes[8..12], 0.0);
    append_f32_to_buffer(&mut bytes[12..16], 0.0);
    bytes
}

fn padded_uniform_bytes(bytes: &[u8]) -> Vec<u8> {
    let padded_len = bytes.len().max(16).div_ceil(16) * 16;
    let mut padded = vec![0_u8; padded_len];
    padded[..bytes.len()].copy_from_slice(bytes);
    padded
}

fn vertex_bytes(vertices: &[GpuVertex]) -> &[u8] {
    let byte_len = vertices.len().saturating_mul(mem::size_of::<GpuVertex>());
    // GpuVertex is #[repr(C)] and contains only f32 arrays, so its in-memory
    // layout is exactly the vertex buffer layout described to wgpu.
    unsafe { std::slice::from_raw_parts(vertices.as_ptr().cast::<u8>(), byte_len) }
}

fn textured_rect_instance_bytes(rects: &[GpuTexturedRectInstance]) -> &[u8] {
    let byte_len = rects
        .len()
        .saturating_mul(mem::size_of::<GpuTexturedRectInstance>());
    // GpuTexturedRectInstance is #[repr(C)] and contains only f32 arrays
    // matching the textured instance buffer layout described to wgpu.
    unsafe { std::slice::from_raw_parts(rects.as_ptr().cast::<u8>(), byte_len) }
}

fn composited_rect_instance_bytes(rects: &[GpuCompositedRectInstance]) -> &[u8] {
    let byte_len = rects
        .len()
        .saturating_mul(mem::size_of::<GpuCompositedRectInstance>());
    // GpuCompositedRectInstance is #[repr(C)] and contains only f32 arrays
    // matching the composited instance buffer layout described to wgpu.
    unsafe { std::slice::from_raw_parts(rects.as_ptr().cast::<u8>(), byte_len) }
}

fn rgba_bytes_for_update(update: &ResourceUpdate) -> Result<Cow<'_, [u8]>, RenderError> {
    if !update.has_expected_byte_len() {
        return Err(RenderError::InvalidResourceUpdate(
            update.descriptor.handle.id().key.clone(),
        ));
    }

    match update.descriptor.format {
        ResourceFormat::Rgba8 => Ok(Cow::Borrowed(&update.bytes)),
        ResourceFormat::Bgra8 => {
            let mut rgba = Vec::with_capacity(update.bytes.len());
            for bgra in update.bytes.chunks_exact(4) {
                rgba.extend_from_slice(&[bgra[2], bgra[1], bgra[0], bgra[3]]);
            }
            Ok(Cow::Owned(rgba))
        }
        ResourceFormat::Alpha8 => {
            let mut rgba = Vec::with_capacity(update.bytes.len().saturating_mul(4));
            for alpha in update.bytes.iter() {
                rgba.extend_from_slice(&[255, 255, 255, *alpha]);
            }
            Ok(Cow::Owned(rgba))
        }
    }
}

fn append_f32_to_buffer(target: &mut [u8], value: f32) {
    target.copy_from_slice(&value.to_le_bytes());
}

fn read_timestamp_query_value(bytes: &[u8]) -> Result<u64, RenderError> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| RenderError::Backend("wgpu timestamp query size mismatch".to_string()))?;
    Ok(u64::from_ne_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::LayerOrder;
    use crate::renderer::EmptyResourceResolver;
    use crate::renderer::RenderOptions;
    use crate::{PaintItem, PaintList, TextContent, UiNodeId};

    #[test]
    fn oversized_gpu_textures_return_errors_and_leave_renderer_usable() {
        #[derive(Debug, Clone, Copy)]
        enum Allocation {
            Image,
            Canvas,
            Snapshot,
            Offscreen,
            Discard,
            Compositor,
        }
        fn attempt(
            renderer: &mut WgpuRenderer,
            allocation: Allocation,
            size: PixelSize,
        ) -> Result<(), RenderError> {
            let viewport = UiSize::new(size.width as f32, size.height as f32);
            let mut request = RenderFrameRequest::new(
                RenderTarget::snapshot(PixelSize::new(1, 1)),
                UiSize::new(1.0, 1.0),
                PaintList::default(),
            );
            match allocation {
                Allocation::Image => {
                    request.resource_updates.push(ResourceUpdate::rgba8_image(
                        crate::platform::ImageHandle::app("size-limited-image"),
                        size,
                        vec![255; size.width as usize * size.height as usize * 4],
                    ));
                }
                Allocation::Canvas => {
                    return renderer
                        .get_gpu_context(
                            &crate::CanvasContent::new("size-limited-canvas").gpu_context(),
                            size,
                        )
                        .map(|_| ());
                }
                Allocation::Snapshot => request.target = RenderTarget::snapshot(size),
                Allocation::Offscreen => request.target = RenderTarget::offscreen(size),
                Allocation::Discard => {
                    request.target = RenderTarget::window("size-limited-window", viewport);
                    request.viewport = viewport;
                }
                Allocation::Compositor => {
                    let bounds = UiRect::new(0.0, 0.0, viewport.width, viewport.height);
                    let mut item = test_rect_item(UiNodeId(1), bounds, ColorRgba::WHITE, 0.0, 1.0);
                    item.kind = PaintKind::CompositedLayer(PaintCompositorLayer::new(
                        bounds,
                        PaintList::default(),
                    ));
                    request.paint.items.push(item);
                }
            }
            renderer
                .render_frame(request, &EmptyResourceResolver)
                .map(|_| ())
        }

        let mut renderer = WgpuRenderer::default();
        let limit = renderer
            .ensure_context()
            .expect("GPU context")
            .device
            .limits()
            .max_texture_dimension_2d;
        for allocation in [
            Allocation::Image,
            Allocation::Canvas,
            Allocation::Snapshot,
            Allocation::Offscreen,
            Allocation::Discard,
            Allocation::Compositor,
        ] {
            // One row or column exercises the real device boundary with little memory.
            for size in [PixelSize::new(limit + 1, 1), PixelSize::new(1, limit + 1)] {
                assert!(
                    attempt(&mut renderer, allocation, size).is_err(),
                    "{allocation:?} accepted {size:?}"
                );
                attempt(&mut renderer, allocation, PixelSize::new(1, 1))
                    .expect("a rejected size must not poison the renderer");
            }
            attempt(&mut renderer, allocation, PixelSize::new(limit, 1))
                .expect("exact device limit is supported");
        }
    }

    #[test]
    fn snapshot_readback_respects_device_buffer_limit_with_row_padding() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("GPU adapter");
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits {
                max_buffer_size: 65_536,
                max_storage_buffer_binding_size: 65_536,
                ..Default::default()
            },
            ..Default::default()
        }))
        .expect("device with a small readback budget");
        let limits = device.limits();
        let rows = (limits.max_buffer_size / u64::from(COPY_BYTES_PER_ROW_ALIGNMENT)) as u32;
        assert!(rows + 1 < limits.max_texture_dimension_2d);
        let mut renderer = WgpuRenderer::with_device_queue(device, queue).expect("renderer");
        for offscreen in [false, true] {
            let mut render = |height| {
                let size = PixelSize::new(1, height);
                renderer.render_frame(
                    RenderFrameRequest::new(
                        if offscreen {
                            RenderTarget::offscreen(size)
                        } else {
                            RenderTarget::snapshot(size)
                        },
                        UiSize::new(1.0, height as f32),
                        PaintList::default(),
                    ),
                    &EmptyResourceResolver,
                )
            };
            assert!(
                render(rows + 1).is_err(),
                "padded readback must fit the device buffer limit"
            );
            let image = render(rows)
                .expect("exact padded buffer limit")
                .snapshot
                .expect("snapshot");
            assert_eq!(image.pixels.len(), rows as usize * 4);
        }
    }

    #[test]
    fn upload_preparation_borrows_rgba_and_converts_other_pixel_formats() {
        let pixels = vec![0, 1, 127, 255, 255, 128, 2, 0];
        for format in [
            ResourceFormat::Rgba8,
            ResourceFormat::Bgra8,
            ResourceFormat::Alpha8,
        ] {
            let (bytes, expected) = match format {
                ResourceFormat::Rgba8 => (pixels.clone(), pixels.clone()),
                ResourceFormat::Bgra8 => (vec![127, 1, 0, 255, 2, 128, 255, 0], pixels.clone()),
                ResourceFormat::Alpha8 => {
                    (vec![0, 255], vec![255, 255, 255, 0, 255, 255, 255, 255])
                }
            };
            let update = ResourceUpdate::full(
                crate::renderer::ResourceDescriptor::new(
                    crate::platform::ImageHandle::app("upload"),
                    PixelSize::new(2, 1),
                    format,
                ),
                bytes,
            );
            let rgba = rgba_bytes_for_update(&update).expect("pixel conversion");
            assert_eq!(&rgba[..], expected.as_slice(), "{format:?}");
            if format == ResourceFormat::Rgba8 {
                assert_eq!(
                    rgba.as_ptr(),
                    update.bytes.as_ptr(),
                    "already-RGBA pixels must not be copied for the upload"
                );
            }
        }
    }

    #[test]
    fn canvas_pipeline_cache_preserves_variants_with_borrowed_lookups() {
        type Change = fn(&mut WgpuCanvasRenderPass<'static>, &mut TextureFormat);
        let changes: &[(&str, Change)] = &[
            ("unchanged", |_, _| {}),
            ("shader", |pass, _| {
                pass.shader = Cow::Owned("other source".into())
            }),
            ("vertex entry", |pass, _| pass.vertex_entry_point = "vertex"),
            ("fragment entry", |pass, _| {
                pass.fragment_entry_point = "fragment"
            }),
            ("format", |_, format| *format = TextureFormat::Bgra8Unorm),
            ("uniform binding", |pass, _| {
                pass.uniforms = Some(Cow::Owned(vec![0; 16]))
            }),
            ("constant name", |pass, _| pass.constants[0].0 = "OTHER"),
            ("constant value", |pass, _| pass.constants[0].1 = 2.0),
            ("extra constant", |pass, _| {
                pass.constants.push(("EXTRA", 1.0))
            }),
            ("missing constant", |pass, _| {
                pass.constants.pop();
            }),
            ("no constants", |pass, _| pass.constants.clear()),
            ("negative zero", |pass, _| pass.constants[1].1 = -0.0),
            ("nan payload one", |pass, _| {
                pass.constants[1].1 = f64::from_bits(0x7ff8000000000001)
            }),
            ("nan payload two", |pass, _| {
                pass.constants[1].1 = f64::from_bits(0x7ff8000000000002)
            }),
        ];
        fn check_cache<S: std::hash::BuildHasher + Default>(changes: &[(&str, Change)]) {
            let mut cache: HashMap<WgpuCanvasPipelineKey<'static>, usize, S> =
                HashMap::with_hasher(S::default());
            for (index, (name, change)) in changes.iter().enumerate() {
                let mut pass = WgpuCanvasRenderPass::wgsl(String::from("shader source"))
                    .constant("QUALITY", 1.0)
                    .constant("MODE", 0.0);
                let mut format = TextureFormat::Rgba8Unorm;
                change(&mut pass, &mut format);
                let key = WgpuCanvasPipelineKey::new(&pass, format);
                assert!(cache.get(&key).is_none(), "variant aliased: {name}");
                cache.insert(key.into_owned(), index);
                // The retained key must outlive this descriptor and its shader allocation.
            }
            for (index, (name, change)) in changes.iter().enumerate() {
                let mut pass = WgpuCanvasRenderPass::wgsl(String::from("shader source"))
                    .constant("QUALITY", 1.0)
                    .constant("MODE", 0.0);
                let mut format = TextureFormat::Rgba8Unorm;
                change(&mut pass, &mut format);
                pass.label = Some("different label");
                pass.clear_color = Some(ColorRgba::WHITE);
                if pass.uniforms.is_some() {
                    pass.uniforms = Some(Cow::Owned(vec![42; 32]));
                }
                assert_eq!(
                    cache.get(&WgpuCanvasPipelineKey::new(&pass, format)),
                    Some(&index),
                    "equivalent descriptor missed: {name}"
                );
            }
        }
        #[derive(Default)]
        struct CollidingHasher;
        impl Hasher for CollidingHasher {
            fn finish(&self) -> u64 {
                0
            }
            fn write(&mut self, _: &[u8]) {}
        }
        check_cache::<std::collections::hash_map::RandomState>(changes);
        check_cache::<std::hash::BuildHasherDefault<CollidingHasher>>(changes);
    }

    #[test]
    fn embedded_canvas_uniform_updates_reuse_pipelines() {
        let mut keys = std::collections::HashSet::new();
        for frame in 0..256 {
            let bytes = (frame as f32 / 120.0).to_le_bytes();
            let program = crate::CanvasRenderProgram::wgsl("shader source")
                .vertex_entry_point("vertex")
                .fragment_entry_point("fragment")
                .constant("QUALITY", 2.0)
                .uniform_bytes(bytes);
            let pass = embedded_canvas_render_pass(&program);
            assert_eq!(pass.uniforms.as_deref(), Some(bytes.as_slice()));
            assert_eq!(
                (pass.vertex_entry_point, pass.fragment_entry_point),
                ("vertex", "fragment")
            );
            keys.insert(WgpuCanvasPipelineKey::new(&pass, TextureFormat::Rgba8Unorm).into_owned());
            let padded = padded_uniform_bytes(pass.uniforms.as_deref().unwrap());
            assert_eq!(&padded[..4], bytes.as_slice());
            assert!(padded[4..].iter().all(|byte| *byte == 0));
            assert_eq!(padded.len() % 16, 0);
        }
        assert_eq!(
            keys.len(),
            1,
            "uniform contents must not specialize a pipeline"
        );
        let program = crate::CanvasRenderProgram::wgsl("shader source")
            .vertex_entry_point("vertex")
            .fragment_entry_point("fragment")
            .constant("QUALITY", 2.0)
            .uniform_bytes(Vec::new());
        let pass = embedded_canvas_render_pass(&program);
        assert_eq!(pass.uniforms.as_deref(), Some([].as_slice()));
        assert_eq!(
            padded_uniform_bytes(pass.uniforms.as_deref().unwrap()),
            vec![0; 16]
        );
        assert!(keys.contains(&WgpuCanvasPipelineKey::new(
            &pass,
            TextureFormat::Rgba8Unorm
        )));

        let mut no_uniforms = program.clone();
        no_uniforms.uniforms = None;
        let pass = embedded_canvas_render_pass(&no_uniforms);
        assert!(pass.uniforms.is_none());
        assert!(
            !keys.contains(&WgpuCanvasPipelineKey::new(
                &pass,
                TextureFormat::Rgba8Unorm
            )),
            "removing the uniform binding changes the pipeline layout"
        );
        let mut specialized = program;
        specialized.constants[0].value = 3.0;
        let pass = embedded_canvas_render_pass(&specialized);
        assert!(
            !keys.contains(&WgpuCanvasPipelineKey::new(
                &pass,
                TextureFormat::Rgba8Unorm
            )),
            "compile-time constants must still select a different pipeline"
        );
    }

    fn test_text_paint(text: impl Into<String>, rect: UiRect) -> TextPaint {
        TextPaint {
            rect,
            clip: UiRect::new(0.0, 0.0, 240.0, 96.0),
            text: text.into(),
            style: TextStyle {
                font_size: 14.0,
                line_height: 20.0,
                color: ColorRgba::WHITE,
                ..Default::default()
            },
            horizontal_align: TextHorizontalAlign::Start,
            vertical_align: TextVerticalAlign::Center,
            opacity: 1.0,
        }
    }

    #[test]
    fn default_glyph_font_system_includes_embedded_web_fonts() {
        let font_system = default_glyph_font_system();
        let families = font_system
            .db()
            .faces()
            .flat_map(|face| face.families.iter().map(|(name, _)| name.as_str()))
            .collect::<Vec<_>>();

        assert!(
            families.iter().any(|name| *name == "Ubuntu"),
            "embedded sans-serif font was not loaded: {families:?}"
        );
        assert!(
            families.iter().any(|name| *name == "Hack"),
            "embedded monospace font was not loaded: {families:?}"
        );
    }

    #[test]
    fn glyph_font_system_uses_injected_font_library_family_defaults() {
        let fonts = FontLibrary::new()
            .with_memory_font("hack-copy", epaint_default_fonts::HACK_REGULAR)
            .with_sans_serif_family("Hack")
            .with_serif_family("Hack")
            .with_monospace_family("Ubuntu");
        let font_system = glyph_font_system(&fonts);

        assert_eq!(
            font_system
                .db()
                .family_name(&glyphon::cosmic_text::fontdb::Family::SansSerif),
            "Hack"
        );
        assert_eq!(
            font_system
                .db()
                .family_name(&glyphon::cosmic_text::fontdb::Family::Serif),
            "Hack"
        );
        assert_eq!(
            font_system
                .db()
                .family_name(&glyphon::cosmic_text::fontdb::Family::Monospace),
            "Ubuntu"
        );
    }

    #[test]
    fn glyphon_text_chunks_reuse_unchanged_prepared_renderers() {
        let mut renderer = WgpuRenderer::default();
        renderer.warm_up().expect("wgpu renderer warm-up");

        let output = renderer
            .render_frame(chunked_text_request(0), &EmptyResourceResolver)
            .expect("initial chunked text frame");
        assert!(output.snapshot.is_none());
        let context = renderer.context.as_ref().expect("wgpu context");
        assert_eq!(context.glyph_chunk_order.len(), 8);
        assert_eq!(context.glyph_chunk_cache.len(), 8);
        assert!(context.glyph_chunks_active);

        let output = renderer
            .render_frame(chunked_text_request(1), &EmptyResourceResolver)
            .expect("single dirty row frame");
        assert!(output.snapshot.is_none());
        let context = renderer.context.as_ref().expect("wgpu context");
        assert_eq!(context.glyph_chunk_order.len(), 8);
        assert_eq!(
            context.glyph_chunk_cache.len(),
            9,
            "one changed label should replace one prepared text chunk, not the full scene"
        );
        assert!(context.glyph_chunks_active);
    }

    #[test]
    fn glyph_scratch_buffers_do_not_alias_scene_text_with_shared_node() {
        let mut renderer = WgpuRenderer::default();
        let size = PixelSize::new(220, 72);
        let context = renderer.ensure_context().expect("wgpu context");
        let health = test_text_paint("HP", UiRect::new(8.0, 8.0, 80.0, 20.0));
        let shield = test_text_paint("SHIELD", UiRect::new(8.0, 34.0, 100.0, 20.0));
        let health_key = TextBufferKey::new(&health);
        let shield_key = TextBufferKey::new(&shield);

        context
            .prepare_glyphon_text(size, OFFSCREEN_FORMAT, &[health, shield])
            .expect("prepare shared-node scene text");

        assert_ne!(health_key, shield_key);
        assert!(
            context.glyph_scratch_buffer_cache.contains_key(&health_key),
            "first scene label lost its scratch text buffer"
        );
        assert!(
            context.glyph_scratch_buffer_cache.contains_key(&shield_key),
            "second scene label lost its scratch text buffer"
        );
        assert_eq!(
            &context.glyph_scratch_buffer_cache[&health_key].key, &health_key,
            "first scene label points at the wrong scratch text buffer"
        );
        assert_eq!(
            &context.glyph_scratch_buffer_cache[&shield_key].key, &shield_key,
            "second scene label points at the wrong scratch text buffer"
        );
    }

    #[test]
    fn srgb_render_targets_linearize_clear_color() {
        let color = ColorRgba::new(18, 18, 18, 255);
        let gamma_clear = wgpu_color_for_format(color, TextureFormat::Rgba8Unorm);
        let srgb_clear = wgpu_color_for_format(color, TextureFormat::Rgba8UnormSrgb);

        assert!((gamma_clear.r - 18.0 / 255.0).abs() < 0.0001);
        assert!(srgb_clear.r < gamma_clear.r);
        assert!((srgb_clear.r - 0.006_049).abs() < 0.0001);
        assert_eq!(srgb_clear.a, 1.0);
    }

    #[test]
    fn srgb_formats_use_linearized_fragment_and_glyph_paths() {
        assert_eq!(
            main_fragment_entry_point(TextureFormat::Bgra8UnormSrgb),
            "fs_main_srgb"
        );
        assert_eq!(
            textured_fragment_entry_point(TextureFormat::Rgba8UnormSrgb),
            "fs_textured_srgb"
        );
        assert_eq!(
            glyph_color_mode(TextureFormat::Rgba8UnormSrgb),
            GlyphColorMode::Accurate
        );
        assert_eq!(
            glyph_color_mode(TextureFormat::Rgba8Unorm),
            GlyphColorMode::Web
        );
    }

    #[test]
    fn paint_occlusion_mask_culls_only_items_hidden_by_later_opaque_rects() {
        let mut items = Vec::new();
        items.push(test_rect_item(
            UiNodeId(1),
            UiRect::new(20.0, 20.0, 20.0, 20.0),
            ColorRgba::new(220, 40, 40, 255),
            0.0,
            1.0,
        ));
        for index in 0..127 {
            items.push(test_rect_item(
                UiNodeId(10 + index),
                UiRect::new(200.0 + index as f32, 200.0, 1.0, 1.0),
                ColorRgba::new(40, 40, 40, 255),
                0.0,
                1.0,
            ));
        }
        items.push(test_rect_item(
            UiNodeId(2),
            UiRect::new(10.0, 10.0, 72.0, 72.0),
            ColorRgba::new(20, 120, 80, 255),
            0.0,
            1.0,
        ));
        items.push(test_rect_item(
            UiNodeId(3),
            UiRect::new(72.0, 20.0, 20.0, 20.0),
            ColorRgba::new(220, 40, 40, 255),
            0.0,
            1.0,
        ));
        items.push(test_rect_item(
            UiNodeId(4),
            UiRect::new(70.0, 18.0, 72.0, 72.0),
            ColorRgba::new(20, 120, 80, 255),
            8.0,
            1.0,
        ));
        items.push(test_rect_item(
            UiNodeId(5),
            UiRect::new(160.0, 20.0, 20.0, 20.0),
            ColorRgba::new(220, 40, 40, 255),
            0.0,
            1.0,
        ));
        items.push(test_rect_item(
            UiNodeId(6),
            UiRect::new(130.0, 18.0, 72.0, 72.0),
            ColorRgba::new(20, 120, 80, 240),
            0.0,
            1.0,
        ));

        let mask = paint_occlusion_mask(&PaintList { items }, UiPoint::new(0.0, 0.0), 1.0);

        assert!(mask[0], "covered item behind opaque rect should be culled");
        assert!(
            !mask[128],
            "opaque covering rect must remain in the paint stream"
        );
        assert!(
            !mask[129],
            "rounded rects are not conservative full-coverage masks"
        );
        assert!(
            !mask[131],
            "translucent rects are not conservative full-coverage masks"
        );
    }

    #[test]
    fn srgb_pipelines_compile_on_wgpu_device() {
        let mut renderer = WgpuRenderer::default();
        renderer.warm_up().expect("wgpu renderer warm-up");
        let context = renderer.context.as_mut().expect("wgpu context");

        let _ = context.triangle_pipeline(TextureFormat::Rgba8UnormSrgb);
        let _ = context.textured_rect_pipeline(TextureFormat::Rgba8UnormSrgb);
        let _ = context.composited_rect_pipeline(TextureFormat::Rgba8UnormSrgb);
        for kind in SdfPipelineKind::ALL {
            let _ = context.sdf_pipeline(TextureFormat::Rgba8UnormSrgb, kind);
        }
    }

    #[test]
    fn snapshots_and_offscreen_targets_honor_explicit_clear_alpha() {
        let mut renderer = WgpuRenderer::default();
        let size = PixelSize::new(4, 4);
        for target in [RenderTarget::snapshot(size), RenderTarget::offscreen(size)] {
            for (clear, expected) in [
                (None, [18, 18, 18, 255]),
                (Some(ColorRgba::TRANSPARENT), [0, 0, 0, 0]),
                (Some(ColorRgba::new(30, 60, 90, 128)), [30, 60, 90, 128]),
            ] {
                let mut request = RenderFrameRequest::new(
                    target.clone(),
                    UiSize::new(4.0, 4.0),
                    PaintList {
                        items: vec![test_rect_item(
                            UiNodeId(1),
                            UiRect::new(1.0, 1.0, 2.0, 2.0),
                            ColorRgba::new(255, 0, 0, 255),
                            0.0,
                            1.0,
                        )],
                    },
                );
                if let Some(clear) = clear {
                    request.options.clear_color = clear;
                }
                let snapshot = renderer
                    .render_frame(request, &EmptyResourceResolver)
                    .expect("clear alpha frame")
                    .snapshot
                    .expect("snapshot");
                assert_eq!(
                    pixel_rgba(&snapshot.pixels, 4, 0, 0),
                    expected,
                    "clear {clear:?}, target {target:?}"
                );
                assert_eq!(
                    pixel_rgba(&snapshot.pixels, 4, 2, 2),
                    [255, 0, 0, 255],
                    "geometry must still render over clear {clear:?}"
                );
            }
        }
    }

    #[test]
    fn app_owned_view_encoder_honors_load_request_clear_and_override() {
        let mut renderer = WgpuRenderer::default();
        renderer.warm_up().expect("wgpu renderer warm-up");
        let context = renderer.context.as_ref().expect("wgpu context");
        let device = context.device.clone();
        let queue = context.queue.clone();
        let size = PixelSize::new(4, 4);
        let format = TextureFormat::Rgba8Unorm;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("operad-wgpu-app-owned-test-texture"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let padded_row_stride = upload_row_stride(size.width).expect("row stride");
        let readback_size = u64::from(padded_row_stride) * u64::from(size.height);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("operad-wgpu-app-owned-test-readback"),
            size: readback_size,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        for (name, clear_color, load_op, expected_background) in [
            (
                "load",
                ColorRgba::TRANSPARENT,
                Some(WgpuRenderLoadOp::Load),
                [0, 0, 255, 255],
            ),
            ("request clear", ColorRgba::TRANSPARENT, None, [0, 0, 0, 0]),
            (
                "transparent override",
                ColorRgba::WHITE,
                Some(WgpuRenderLoadOp::Clear(ColorRgba::TRANSPARENT)),
                [0, 0, 0, 0],
            ),
            (
                "translucent override",
                ColorRgba::TRANSPARENT,
                Some(WgpuRenderLoadOp::Clear(ColorRgba::new(0, 255, 0, 128))),
                [0, 255, 0, 128],
            ),
        ] {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("operad-wgpu-app-owned-test-encoder"),
            });
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("operad-wgpu-app-owned-test-background"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 1.0,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
            }

            let request = RenderFrameRequest::new(
                RenderTarget::app_owned("app-owned-test", UiSize::new(4.0, 4.0)),
                UiSize::new(4.0, 4.0),
                PaintList {
                    items: vec![test_rect_item(
                        UiNodeId(9),
                        UiRect::new(1.0, 1.0, 2.0, 2.0),
                        ColorRgba::new(240, 20, 40, 255),
                        0.0,
                        1.0,
                    )],
                },
            )
            .options(RenderOptions {
                collect_gpu_timing: true,
                clear_color,
                ..RenderOptions::default()
            });
            let timed_output = renderer
                .render_frame_into_view_with_encoder_timed(
                    request,
                    &EmptyResourceResolver,
                    &mut encoder,
                    WgpuRenderTargetView {
                        view: &view,
                        format,
                        load_op,
                    },
                )
                .expect("render into app-owned view");
            let gpu_timing_token = timed_output.gpu_timing_token;
            let output = timed_output.frame;
            assert_eq!(output.target.kind(), RenderTargetKind::AppOwned);
            assert_eq!(output.painted_items, 1);
            assert!(output.snapshot.is_none());

            encoder.copy_texture_to_buffer(
                TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded_row_stride),
                        rows_per_image: Some(size.height),
                    },
                },
                Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));
            if let Some(token) = gpu_timing_token {
                assert!(
                    renderer
                        .resolve_gpu_timing(token)
                        .expect("resolve caller-owned GPU timing")
                        .is_some(),
                    "timestamp-capable devices should resolve app-owned pass timing"
                );
            } else {
                assert!(
                    renderer
                        .context
                        .as_ref()
                        .expect("wgpu context")
                        .gpu_timer
                        .is_none(),
                    "timestamp-capable devices should return a timing token"
                );
            }
            let _ = device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("wgpu poll");
            let readback_slice = readback.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            readback_slice.map_async(wgpu::MapMode::Read, move |status| {
                tx.send(status).ok();
            });
            let _ = device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("wgpu poll");
            rx.recv().expect("map callback").expect("map readback");
            let mapped = readback_slice.get_mapped_range();
            let mut pixels = vec![0_u8; render_byte_len(size).expect("render byte len")];
            for y in 0..usize::try_from(size.height).expect("height") {
                let source = y * usize::try_from(padded_row_stride).expect("stride");
                let destination = y * usize::try_from(size.width).expect("width") * 4;
                pixels[destination..destination + usize::try_from(size.width).expect("width") * 4]
                    .copy_from_slice(
                        &mapped[source..source + usize::try_from(size.width).expect("width") * 4],
                    );
            }
            drop(mapped);
            readback.unmap();

            assert_eq!(pixel_rgba(&pixels, 4, 0, 0), expected_background, "{name}");
            assert_eq!(pixel_rgba(&pixels, 4, 2, 2), [240, 20, 40, 255]);
        }
    }

    fn test_rect_item(
        node: UiNodeId,
        rect: UiRect,
        fill: ColorRgba,
        corner_radius: f32,
        opacity: f32,
    ) -> PaintItem {
        PaintItem {
            node,
            rect,
            clip_rect: UiRect::new(0.0, 0.0, 320.0, 240.0),
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity,
            transform: PaintTransform::default(),
            shader: None,
            material: None,
            kind: PaintKind::Rect {
                fill,
                stroke: None,
                corner_radius,
            },
        }
    }

    #[test]
    fn unavailable_surface_does_not_apply_uploads_before_retrying_shape_changes() {
        use crate::platform::ImageHandle;
        use crate::renderer::ResourceDescriptor;

        let make_request = |updates: Vec<ResourceUpdate>| {
            let mut item = test_rect_item(
                UiNodeId(1),
                UiRect::new(0.0, 0.0, 8.0, 4.0),
                ColorRgba::WHITE,
                0.0,
                1.0,
            );
            item.kind = PaintKind::Image {
                key: "retry-image".to_owned(),
                tint: None,
            };
            RenderFrameRequest::new(
                RenderTarget::snapshot(PixelSize::new(8, 4)),
                UiSize::new(8.0, 4.0),
                PaintList { items: vec![item] },
            )
            .resource_updates(updates)
        };
        let base = ResourceDescriptor::new(
            ImageHandle::app("retry-image"),
            PixelSize::new(4, 4),
            ResourceFormat::Rgba8,
        );
        let replacement = ResourceDescriptor::new(
            base.handle.clone(),
            PixelSize::new(8, 4),
            ResourceFormat::Bgra8,
        );
        let updates = vec![
            ResourceUpdate::partial(
                base.clone(),
                PixelRect::new(0, 0, 1, 1),
                vec![0, 255, 0, 255],
            ),
            ResourceUpdate::full(replacement.clone(), [255, 0, 0, 255].repeat(32)),
            ResourceUpdate::partial(
                replacement,
                PixelRect::new(2, 1, 3, 2),
                [0, 255, 255, 255].repeat(6),
            ),
        ];
        let mut renderer = WgpuRenderer::default();
        renderer
            .render_frame(
                make_request(vec![ResourceUpdate::full(
                    base,
                    [255, 0, 0, 255].repeat(16),
                )]),
                &EmptyResourceResolver,
            )
            .expect("initial image");

        for reason in ["timeout", "outdated", "timeout"] {
            let error = renderer
                .prepare_frame(
                    make_request(updates.clone()),
                    &EmptyResourceResolver,
                    |_, _| Err::<(), _>(RenderError::SurfaceUnavailable(reason.to_owned())),
                )
                .err()
                .expect("injected surface failure");
            assert_eq!(
                error,
                RenderError::SurfaceUnavailable(reason.to_owned()),
                "retry must not fail because a previous failed attempt changed the image shape"
            );
        }

        let unchanged = renderer
            .render_frame(make_request(Vec::new()), &EmptyResourceResolver)
            .expect("read image after failed acquisition")
            .snapshot
            .expect("snapshot");
        assert!(unchanged
            .pixels
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 0, 0, 255]));

        let prepared = renderer
            .prepare_frame(make_request(updates), &EmptyResourceResolver, |_, _| Ok(()))
            .expect("acquisition recovered");
        let pixels = renderer
            .render_snapshot(prepared.size, ColorRgba::TRANSPARENT)
            .expect("render recovered frame");
        for y in 0..4 {
            for x in 0..8 {
                let expected = if (2..5).contains(&x) && (1..3).contains(&y) {
                    [255, 255, 0, 255]
                } else {
                    [0, 0, 255, 255]
                };
                assert_eq!(pixel_rgba(&pixels, 8, x, y), expected, "pixel ({x}, {y})");
            }
        }
    }

    #[test]
    fn partial_resource_uploads_preserve_the_base_and_reject_shape_changes() {
        use crate::platform::ImageHandle;
        use crate::renderer::ResourceDescriptor;

        let descriptor = ResourceDescriptor::new(
            ImageHandle::app("patched-image"),
            PixelSize::new(4, 4),
            ResourceFormat::Rgba8,
        );
        let render = |renderer: &mut WgpuRenderer, update: Option<ResourceUpdate>| {
            let mut item = test_rect_item(
                UiNodeId(1),
                UiRect::new(0.0, 0.0, 4.0, 4.0),
                ColorRgba::WHITE,
                0.0,
                1.0,
            );
            item.kind = PaintKind::Image {
                key: "patched-image".to_owned(),
                tint: None,
            };
            let mut request = RenderFrameRequest::new(
                RenderTarget::snapshot(PixelSize::new(4, 4)),
                UiSize::new(4.0, 4.0),
                PaintList { items: vec![item] },
            );
            request.resource_updates.extend(update);
            renderer
                .render_frame(request, &EmptyResourceResolver)
                .map(|output| output.snapshot.expect("snapshot"))
        };
        let mut renderer = WgpuRenderer::default();
        render(
            &mut renderer,
            Some(ResourceUpdate::full(
                descriptor.clone(),
                [255, 0, 0, 255].repeat(16),
            )),
        )
        .expect("full base upload");

        for (label, size, format) in [
            ("resize", PixelSize::new(6, 4), ResourceFormat::Rgba8),
            ("BGRA format change", descriptor.size, ResourceFormat::Bgra8),
            (
                "alpha format change",
                descriptor.size,
                ResourceFormat::Alpha8,
            ),
        ] {
            let mut changed = descriptor.clone();
            changed.size = size;
            changed.format = format;
            let result = render(
                &mut renderer,
                Some(ResourceUpdate::partial(
                    changed,
                    PixelRect::new(1, 1, 2, 2),
                    vec![255; 4 * format.bytes_per_pixel()],
                )),
            );
            assert!(
                matches!(result, Err(RenderError::InvalidResourceUpdate(_))),
                "{label} must require a full upload"
            );
            let image = render(&mut renderer, None).expect("base after rejected update");
            for pixel in image.pixels.chunks_exact(4) {
                assert_eq!(pixel, [255, 0, 0, 255], "{label} changed the base");
            }
        }

        // Default-version uploads are ordered writes; a patch changes only its rectangle.
        let image = render(
            &mut renderer,
            Some(ResourceUpdate::partial(
                descriptor.clone(),
                PixelRect::new(1, 1, 2, 2),
                [0, 255, 0, 255].repeat(4),
            )),
        )
        .expect("same-shape patch");
        for y in 0..4 {
            for x in 0..4 {
                let expected = if (1..3).contains(&x) && (1..3).contains(&y) {
                    [0, 255, 0, 255]
                } else {
                    [255, 0, 0, 255]
                };
                assert_eq!(pixel_rgba(&image.pixels, 4, x, y), expected);
            }
        }

        // A full replacement establishes the format for subsequent partial writes,
        // even when the GPU allocation can be reused at the same dimensions.
        let mut bgra = descriptor.clone();
        bgra.format = ResourceFormat::Bgra8;
        render(
            &mut renderer,
            Some(ResourceUpdate::full(
                bgra.clone(),
                [255, 0, 0, 255].repeat(16),
            )),
        )
        .expect("full format replacement");
        let image = render(
            &mut renderer,
            Some(ResourceUpdate::partial(
                bgra,
                PixelRect::new(0, 0, 1, 1),
                vec![0, 255, 255, 255],
            )),
        )
        .expect("patch after format replacement");
        assert_eq!(pixel_rgba(&image.pixels, 4, 0, 0), [255, 255, 0, 255]);
        assert_eq!(pixel_rgba(&image.pixels, 4, 3, 3), [0, 0, 255, 255]);

        render(
            &mut renderer,
            Some(ResourceUpdate::rgba8_image(
                ImageHandle::app("patched-image"),
                PixelSize::ZERO,
                Vec::new(),
            )),
        )
        .expect("release image");
        let result = render(
            &mut renderer,
            Some(ResourceUpdate::partial(
                descriptor,
                PixelRect::new(0, 0, 1, 1),
                vec![255; 4],
            )),
        );
        assert!(
            matches!(result, Err(RenderError::InvalidResourceUpdate(_))),
            "a patch cannot establish a missing base"
        );
    }

    #[test]
    fn compositor_cleanup_preserves_app_images_and_canvas_buffers() {
        let mut renderer = WgpuRenderer::default();
        // App names must not share the namespace or lifetime of generated layers.
        let image_key = "__operad_layer_1_0";
        let canvas = crate::CanvasContent::new("__operad_layer_1_1").gpu_context();
        renderer
            .get_gpu_context(&canvas, PixelSize::new(4, 4))
            .expect("persistent canvas")
            .clear(ColorRgba::new(0, 255, 0, 255));
        let image = ResourceUpdate::rgba8_image(
            crate::platform::ImageHandle::app(image_key),
            PixelSize::new(1, 1),
            vec![255, 0, 0, 255],
        );
        let item = |x, kind| {
            let mut item = test_rect_item(
                UiNodeId(1),
                UiRect::new(x, 0.0, 4.0, 4.0),
                ColorRgba::WHITE,
                0.0,
                1.0,
            );
            item.kind = kind;
            item
        };
        for (frame, with_layers) in [true, true, false, true, false].into_iter().enumerate() {
            let mut items = vec![
                item(
                    0.0,
                    PaintKind::Image {
                        key: image_key.to_owned(),
                        tint: None,
                    },
                ),
                item(4.0, PaintKind::Canvas(canvas.clone())),
            ];
            let blue = 40 + frame as u8 * 40;
            if with_layers {
                let bounds = UiRect::new(8.0, 0.0, 4.0, 4.0);
                let child = test_rect_item(
                    UiNodeId(2),
                    bounds,
                    ColorRgba::new(0, 0, blue, 255),
                    0.0,
                    1.0,
                );
                let inner = PaintCompositorLayer::new(bounds, PaintList { items: vec![child] });
                let outer = PaintCompositorLayer::new(
                    bounds,
                    PaintList {
                        items: vec![item(8.0, PaintKind::CompositedLayer(inner))],
                    },
                );
                items.push(item(8.0, PaintKind::CompositedLayer(outer)));
            }
            let mut request = RenderFrameRequest::new(
                RenderTarget::snapshot(PixelSize::new(12, 4)),
                UiSize::new(12.0, 4.0),
                PaintList { items },
            )
            .options(RenderOptions {
                clear_color: ColorRgba::new(3, 4, 5, 255),
                ..Default::default()
            });
            if frame == 0 {
                request.resource_updates.push(image.clone());
            }
            let snapshot = renderer
                .render_frame(request, &EmptyResourceResolver)
                .expect("compositor frame")
                .snapshot
                .expect("snapshot");
            assert_eq!(
                pixel_rgba(&snapshot.pixels, 12, 2, 2),
                [255, 0, 0, 255],
                "app image lost on frame {frame}"
            );
            assert_eq!(
                pixel_rgba(&snapshot.pixels, 12, 6, 2),
                [0, 255, 0, 255],
                "canvas buffer lost on frame {frame}"
            );
            assert_eq!(
                pixel_rgba(&snapshot.pixels, 12, 10, 2),
                if with_layers {
                    [0, 0, blue, 255]
                } else {
                    [3, 4, 5, 255]
                },
                "nested layer contents on frame {frame}"
            );
            let context = renderer.context.as_ref().expect("wgpu context");
            assert_eq!(
                context.textures.len(),
                2,
                "app resources changed on frame {frame}"
            );
            assert_eq!(
                context.layer_textures.len(),
                if with_layers { 2 } else { 0 },
                "retained compositor targets from an earlier frame {frame}"
            );
        }
    }

    #[test]
    fn canvas_context_render_pass_draws_shader_into_sampled_texture() {
        let mut renderer = WgpuRenderer::default();
        let canvas = crate::CanvasContent::new("attached.canvas").gpu_context();
        {
            let context = renderer
                .get_gpu_context(&canvas, PixelSize::new(4, 4))
                .expect("gpu canvas context");
            context
                .render_pass(WgpuCanvasRenderPass::wgsl(
                    r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.uv.x * 0.0 + 0.9098039, input.uv.y * 0.0 + 0.07843137, 0.17254902, 1.0);
}
"#,
                ))
                .expect("canvas shader pass");
        }

        let viewport = UiSize::new(4.0, 4.0);
        let output = renderer
            .render_frame(
                RenderFrameRequest::new(
                    RenderTarget::snapshot(PixelSize::new(4, 4)),
                    viewport,
                    PaintList {
                        items: vec![PaintItem {
                            node: UiNodeId(1),
                            rect: UiRect::new(0.0, 0.0, 4.0, 4.0),
                            clip_rect: UiRect::new(0.0, 0.0, 4.0, 4.0),
                            z_index: 0.0,
                            layer_order: LayerOrder::DEFAULT,
                            opacity: 1.0,
                            transform: Default::default(),
                            shader: None,
                            material: None,
                            kind: PaintKind::Canvas(canvas),
                        }],
                    },
                ),
                &EmptyResourceResolver,
            )
            .expect("canvas context render frame");
        let snapshot = output.snapshot.expect("snapshot");

        assert_eq!(snapshot.size, PixelSize::new(4, 4));
        assert_eq!(pixel_rgba(&snapshot.pixels, 4, 2, 2), [232, 20, 44, 255]);
    }

    #[test]
    fn canvas_pipeline_cache_bounds_shader_edits_and_recreates_evicted_variants() {
        const CACHE_LIMIT: usize = MAX_CACHED_CANVAS_PIPELINES;
        const SHADER: &str = r#"
override RED: f32 = 0.2;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(positions[index], 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(RED, 0.0, 0.0, 1.0);
}
"#;
        let mut renderer = WgpuRenderer::default();
        let canvas = crate::CanvasContent::new("constant.canvas").gpu_context();
        {
            let context = renderer
                .get_gpu_context(&canvas, PixelSize::new(4, 4))
                .expect("gpu canvas context");
            let cold = WgpuCanvasRenderPass::wgsl(SHADER).constant("RED", 0.25);
            let hot = WgpuCanvasRenderPass::wgsl(SHADER).constant("RED", 0.75);
            let cached_pipeline = |pass: &WgpuCanvasRenderPass<'_>| {
                context
                    .pipeline_cache
                    .borrow_mut()
                    .get(&WgpuCanvasPipelineKey::new(pass, context.format()))
            };
            context.render_pass(cold.clone()).expect("cold variant");
            let original_cold = cached_pipeline(&cold).expect("cached cold variant");
            context.render_pass(hot.clone()).expect("hot variant");
            let original_hot = cached_pipeline(&hot).expect("cached hot variant");
            // Direct canvas users need bounded retention even without UI frame boundaries.
            // Exercise both changing WGSL source and specialization-only changes.
            for revision in 0..CACHE_LIMIT * 3 {
                let shader = if revision % 2 == 0 {
                    Cow::Owned(format!("{SHADER}\n// revision {revision}"))
                } else {
                    Cow::Borrowed(SHADER)
                };
                context
                    .render_pass(
                        WgpuCanvasRenderPass::wgsl(shader)
                            .constant("RED", revision as f64 / (CACHE_LIMIT * 3) as f64),
                    )
                    .expect("edited shader");
                context
                    .render_pass(hot.clone())
                    .expect("reused hot variant");
            }
            let retained = context.pipeline_cache.borrow().len();
            assert!(
                retained <= CACHE_LIMIT,
                "shader edits retained {retained} pipelines; cache budget is {CACHE_LIMIT}"
            );
            assert_eq!(
                cached_pipeline(&hot),
                Some(original_hot),
                "hot pipeline recompiled"
            );
            assert!(
                cached_pipeline(&cold).is_none(),
                "unused variant never retired"
            );
            context
                .render_pass(cold.clone())
                .expect("recreated cold variant");
            assert_ne!(
                cached_pipeline(&cold),
                Some(original_cold),
                "cold variant was not recreated"
            );
        }

        let output = renderer
            .render_frame(
                RenderFrameRequest::new(
                    RenderTarget::snapshot(PixelSize::new(4, 4)),
                    UiSize::new(4.0, 4.0),
                    PaintList {
                        items: vec![PaintItem {
                            node: UiNodeId(1),
                            rect: UiRect::new(0.0, 0.0, 4.0, 4.0),
                            clip_rect: UiRect::new(0.0, 0.0, 4.0, 4.0),
                            z_index: 0.0,
                            layer_order: LayerOrder::DEFAULT,
                            opacity: 1.0,
                            transform: Default::default(),
                            shader: None,
                            material: None,
                            kind: PaintKind::Canvas(canvas),
                        }],
                    },
                ),
                &EmptyResourceResolver,
            )
            .expect("canvas context render frame");
        let snapshot = output.snapshot.expect("snapshot");
        let pixel = pixel_rgba(&snapshot.pixels, 4, 2, 2);

        assert_eq!(
            pixel,
            [64, 0, 0, 255],
            "recreated shader must preserve its specialization"
        );
    }

    #[test]
    fn embedded_canvas_program_draws_before_snapshot_composite() {
        let mut renderer = WgpuRenderer::default();
        let canvas = crate::CanvasContent::new("embedded.canvas").program(
            crate::CanvasRenderProgram::wgsl(
                r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return output;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 1.0, 1.0, 1.0);
}
"#,
            )
            .clear_color(Some(ColorRgba::new(0, 0, 0, 255))),
        );
        let output = renderer
            .render_frame(
                RenderFrameRequest::new(
                    RenderTarget::snapshot(PixelSize::new(4, 4)),
                    UiSize::new(4.0, 4.0),
                    PaintList {
                        items: vec![PaintItem {
                            node: UiNodeId(1),
                            rect: UiRect::new(0.0, 0.0, 4.0, 4.0),
                            clip_rect: UiRect::new(0.0, 0.0, 4.0, 4.0),
                            z_index: 0.0,
                            layer_order: LayerOrder::DEFAULT,
                            opacity: 1.0,
                            transform: Default::default(),
                            shader: None,
                            material: None,
                            kind: PaintKind::Canvas(canvas),
                        }],
                    },
                ),
                &EmptyResourceResolver,
            )
            .expect("embedded canvas program frame");
        let snapshot = output.snapshot.expect("snapshot");

        // Exact UNORM endpoints avoid implementation-dependent rounding of
        // decimal shader colors. This tests canvas/composite order.
        assert_eq!(pixel_rgba(&snapshot.pixels, 4, 2, 2), [0, 255, 255, 255]);
    }

    #[test]
    fn embedded_canvas_program_validation_error_keeps_frame_rendering() {
        let mut renderer = WgpuRenderer::default();
        let canvas = crate::CanvasContent::new("embedded.canvas.invalid").program(
            crate::CanvasRenderProgram::wgsl(
                r#"
@vertex
fn vs_main() -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return definitely_not_defined();
}
"#,
            )
            .clear_color(Some(ColorRgba::new(0, 0, 0, 255))),
        );
        let output = renderer
            .render_frame(
                RenderFrameRequest::new(
                    RenderTarget::snapshot(PixelSize::new(4, 4)),
                    UiSize::new(4.0, 4.0),
                    PaintList {
                        items: vec![PaintItem {
                            node: UiNodeId(1),
                            rect: UiRect::new(0.0, 0.0, 4.0, 4.0),
                            clip_rect: UiRect::new(0.0, 0.0, 4.0, 4.0),
                            z_index: 0.0,
                            layer_order: LayerOrder::DEFAULT,
                            opacity: 1.0,
                            transform: Default::default(),
                            shader: None,
                            material: None,
                            kind: PaintKind::Canvas(canvas),
                        }],
                    },
                ),
                &EmptyResourceResolver,
            )
            .expect("invalid embedded canvas shader should not abort the frame");
        let snapshot = output.snapshot.expect("snapshot");

        assert_eq!(pixel_rgba(&snapshot.pixels, 4, 2, 2), [88, 20, 34, 255]);
    }

    #[test]
    fn gpu_render_timing_is_reported_when_timestamp_queries_are_available() {
        let mut renderer = WgpuRenderer::default();
        renderer.warm_up().expect("wgpu renderer warm-up");

        let output = renderer
            .render_frame(
                chunked_text_request(0).options(RenderOptions {
                    collect_gpu_timing: true,
                    ..RenderOptions::default()
                }),
                &EmptyResourceResolver,
            )
            .expect("timestamped render frame");
        let context = renderer.context.as_ref().expect("wgpu context");
        if context.gpu_timer.is_some() {
            assert!(
                output.timings.duration("gpu-render").is_some(),
                "timestamp-capable devices must report GPU render pass timing"
            );
        } else {
            assert!(
                output.timings.duration("gpu-render").is_none(),
                "non-timestamp devices should not fake GPU timing"
            );
        }
    }

    #[test]
    fn text_render_key_preserves_fractional_position_without_rebuilding_layout_buffer() {
        let text = TextPaint {
            rect: UiRect::new(4.25, 6.5, 88.0, 28.0),
            clip: UiRect::new(0.0, 0.0, 96.0, 36.0),
            text: "Subpixel".to_string(),
            style: TextStyle {
                font_size: 20.0,
                line_height: 24.0,
                color: ColorRgba::WHITE,
                ..Default::default()
            },
            horizontal_align: TextHorizontalAlign::Start,
            vertical_align: TextVerticalAlign::Top,
            opacity: 1.0,
        };
        let moved = TextPaint {
            rect: UiRect::new(4.75, 6.5, 88.0, 28.0),
            ..text.clone()
        };
        let size = PixelSize::new(96, 36);
        let original_key = TextRenderKey::new(&text, size);
        let moved_key = TextRenderKey::new(&moved, size);

        assert_eq!(original_key.buffer, moved_key.buffer);
        assert!(original_key.buffer.has_same_layout_as(&moved_key.buffer));
        assert_ne!(original_key.rect_x, moved_key.rect_x);
    }

    #[test]
    fn text_render_keys_track_scene_text_alignment() {
        let text = TextPaint {
            rect: UiRect::new(4.0, 6.0, 88.0, 40.0),
            clip: UiRect::new(0.0, 0.0, 96.0, 64.0),
            text: "Centered".to_string(),
            style: TextStyle {
                font_size: 20.0,
                line_height: 24.0,
                color: ColorRgba::WHITE,
                ..Default::default()
            },
            horizontal_align: TextHorizontalAlign::Start,
            vertical_align: TextVerticalAlign::Top,
            opacity: 1.0,
        };
        let centered = TextPaint {
            horizontal_align: TextHorizontalAlign::Center,
            vertical_align: TextVerticalAlign::Center,
            ..text.clone()
        };
        let size = PixelSize::new(96, 64);
        let original_key = TextRenderKey::new(&text, size);
        let centered_key = TextRenderKey::new(&centered, size);

        assert_ne!(original_key.buffer, centered_key.buffer);
        assert!(!original_key.buffer.has_same_layout_as(&centered_key.buffer));
        assert_ne!(original_key.vertical_align, centered_key.vertical_align);
    }

    #[test]
    fn push_text_carries_scene_alignment_into_wgpu_text_geometry() {
        let mut geometry = RenderGeometry::default();
        push_text(
            &mut geometry,
            UiRect::new(4.0, 6.0, 88.0, 40.0),
            UiRect::new(0.0, 0.0, 96.0, 64.0),
            "Centered",
            &TextStyle {
                font_size: 20.0,
                line_height: 24.0,
                color: ColorRgba::WHITE,
                ..Default::default()
            },
            TextHorizontalAlign::Center,
            TextVerticalAlign::Center,
            1.0,
            PaintTransform::default(),
        );

        let text = geometry.texts.first().expect("text geometry");
        assert_eq!(text.horizontal_align, TextHorizontalAlign::Center);
        assert_eq!(text.vertical_align, TextVerticalAlign::Center);
    }

    #[cfg(feature = "text-cosmic")]
    #[test]
    fn glyphon_ellipsis_matches_layout_fitting_and_invalidates_cached_clip() {
        let mut fonts = default_glyph_font_system();
        let mut measurer = crate::CosmicTextMeasurer::new();
        let mut buffer = GlyphBuffer::new(&mut fonts, GlyphMetrics::new(14.0, 20.0));
        for source in [
            "WWW iii long label",
            "e\u{301}e\u{301} emoji 👩🏽‍💻 family",
            "אבג abc דהו",
        ] {
            for family in [FontFamily::SansSerif, FontFamily::Monospace] {
                let mut text = test_text_paint(source, UiRect::new(0.0, 0.0, 65.0, 24.0));
                text.style.family = family;
                text.style.wrap = TextWrap::None;
                let clipped_key = TextBufferKey::new(&text);
                sync_glyph_buffer(&mut buffer, &mut fonts, &text, None, &clipped_key);
                text.style.overflow = TextOverflow::Ellipsis;
                let ellipsis_key = TextBufferKey::new(&text);
                sync_glyph_buffer(
                    &mut buffer,
                    &mut fonts,
                    &text,
                    Some(&clipped_key),
                    &ellipsis_key,
                );
                let expected = measurer.fit_text(
                    &TextContent::new(source, text.style.clone()),
                    text.rect.width,
                );
                assert!(expected.truncated);
                assert_eq!(
                    buffer
                        .lines
                        .iter()
                        .map(|line| line.text())
                        .collect::<String>(),
                    expected.text
                );
                let runs: Vec<_> = buffer.layout_runs().collect();
                assert_eq!(runs.len(), 1);
                assert!(runs[0].line_w <= text.rect.width);
                assert!((runs[0].line_w - expected.size.width).abs() < 0.001);
                assert_ne!(
                    clipped_key, ellipsis_key,
                    "cache must observe overflow changes"
                );
                sync_glyph_buffer(
                    &mut buffer,
                    &mut fonts,
                    &TextPaint {
                        style: TextStyle {
                            overflow: TextOverflow::Clip,
                            ..text.style
                        },
                        ..text
                    },
                    Some(&ellipsis_key),
                    &clipped_key,
                );
                assert_eq!(buffer.lines[0].text(), source);
            }
        }
    }

    #[cfg(feature = "text-cosmic")]
    #[test]
    fn scene_text_ellipsis_renders_the_measured_presentation() {
        let mut renderer = WgpuRenderer::new();
        let rect = UiRect::new(2.0, 2.0, 70.0, 24.0);
        let style = TextStyle {
            wrap: TextWrap::None,
            ..Default::default()
        };
        let source = "A long label that needs fitting";
        let expected = crate::CosmicTextMeasurer::new()
            .fit_text(&TextContent::new(source, style.clone()), rect.width);
        assert!(expected.truncated && !expected.text.is_empty());
        let mut render = |value: &str, overflow| {
            renderer
                .render_frame(
                    RenderFrameRequest::new(
                        RenderTarget::snapshot(PixelSize::new(80, 30)),
                        UiSize::new(80.0, 30.0),
                        PaintList {
                            items: vec![PaintItem {
                                node: UiNodeId(1),
                                rect,
                                clip_rect: rect,
                                z_index: 0.0,
                                layer_order: LayerOrder::DEFAULT,
                                opacity: 1.0,
                                transform: Default::default(),
                                shader: None,
                                material: None,
                                kind: PaintKind::SceneText(
                                    crate::PaintText::new(value, rect, style.clone())
                                        .overflow(overflow),
                                ),
                            }],
                        },
                    ),
                    &EmptyResourceResolver,
                )
                .unwrap()
                .snapshot
                .unwrap()
                .pixels
        };
        let ellipsized = render(source, TextOverflow::Ellipsis);
        assert_eq!(ellipsized, render(&expected.text, TextOverflow::Clip));
        assert_ne!(ellipsized, render(source, TextOverflow::Clip));
    }

    #[test]
    fn missing_image_placeholder_uses_clear_checkerboard() {
        let mut geometry = RenderGeometry::default();
        let rect = UiRect::new(0.0, 0.0, 16.0, 16.0);
        push_image_placeholder(&mut geometry, rect, rect, "assets.missing", None, 1.0);

        assert_eq!(geometry.shapes.len(), 4);
        assert_eq!(geometry.vertices.len(), 0);
        assert_eq!(
            geometry.shapes[0].color,
            color_as_vertex(MISSING_IMAGE_DARK, 1.0)
        );
        assert_eq!(
            geometry.shapes[1].color,
            color_as_vertex(MISSING_IMAGE_PURPLE, 1.0)
        );
        assert_eq!(
            geometry.shapes[2].color,
            color_as_vertex(MISSING_IMAGE_PURPLE, 1.0)
        );
        assert_eq!(
            geometry.shapes[3].color,
            color_as_vertex(MISSING_IMAGE_DARK, 1.0)
        );
    }

    #[test]
    fn push_polygon_uses_concave_tessellation() {
        let mut geometry = RenderGeometry::default();
        // A U shape cannot be triangulated correctly as a fan from its first vertex.
        let points = [
            UiPoint::new(0.0, 0.0),
            UiPoint::new(16.0, 0.0),
            UiPoint::new(16.0, 16.0),
            UiPoint::new(12.0, 16.0),
            UiPoint::new(12.0, 4.0),
            UiPoint::new(4.0, 4.0),
            UiPoint::new(4.0, 16.0),
            UiPoint::new(0.0, 16.0),
        ];
        push_polygon(
            &mut geometry,
            &points,
            UiRect::new(0.0, 0.0, 32.0, 32.0),
            ColorRgba::WHITE,
            1.0,
        );

        let area = geometry
            .vertices
            .chunks_exact(3)
            .map(|triangle| {
                let [ax, ay] = triangle[0].position;
                let [bx, by] = triangle[1].position;
                let [cx, cy] = triangle[2].position;
                ((bx - ax) * (cy - ay) - (by - ay) * (cx - ax)).abs() * 0.5
            })
            .sum::<f32>();
        assert!((area - 160.0).abs() < 0.001, "concave polygon area: {area}");
    }

    #[test]
    fn built_in_icon_image_fallback_uses_vector_paths() {
        let mut geometry = RenderGeometry::default();
        let rect = UiRect::new(0.0, 0.0, 24.0, 24.0);
        let tint = ColorRgba::new(118, 183, 255, 255);
        push_image_placeholder(
            &mut geometry,
            rect,
            rect,
            BuiltInIcon::Operad.key(),
            Some(tint),
            1.0,
        );

        assert!(
            geometry
                .shapes
                .iter()
                .all(|s| s.color != color_as_vertex(MISSING_IMAGE_DARK, 1.0)
                    && s.color != color_as_vertex(MISSING_IMAGE_PURPLE, 1.0)),
            "built-in icon fell back to checkerboard"
        );
        assert!(
            !geometry.vertices.is_empty() || !geometry.shapes.is_empty(),
            "built-in icon should render vector fallback primitives"
        );
        assert!(
            geometry
                .vertices
                .iter()
                .any(|vertex| vertex.color == color_as_vertex(tint, 1.0))
                || geometry
                    .shapes
                    .iter()
                    .any(|s| s.color == color_as_vertex(tint, 1.0)),
            "built-in icon fallback did not carry the image tint"
        );
    }

    fn chunked_text_request(frame: usize) -> RenderFrameRequest {
        const ROWS: usize = 64;
        let viewport = UiSize::new(640.0, 480.0);
        let dirty_row = frame % ROWS;
        let mut items = Vec::with_capacity(ROWS + 1);
        items.push(PaintItem {
            node: UiNodeId(70_000),
            rect: UiRect::new(0.0, 0.0, viewport.width, viewport.height),
            clip_rect: UiRect::new(0.0, 0.0, viewport.width, viewport.height),
            z_index: 0.0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: Default::default(),
            shader: None,
            material: None,
            kind: PaintKind::Rect {
                fill: ColorRgba::new(6, 9, 14, 255),
                stroke: None,
                corner_radius: 0.0,
            },
        });
        for row in 0..ROWS {
            let text = if row == dirty_row {
                format!("Chunk row {row:02} dirty frame {frame}")
            } else {
                format!("Chunk row {row:02} stable cached text")
            };
            items.push(PaintItem {
                node: UiNodeId(70_001 + row),
                rect: UiRect::new(12.0, 12.0 + row as f32 * 7.0, 360.0, 12.0),
                clip_rect: UiRect::new(0.0, 0.0, viewport.width, viewport.height),
                z_index: 0.0,
                layer_order: LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: Default::default(),
                shader: None,
                material: None,
                kind: PaintKind::Text(TextContent::new(
                    text,
                    TextStyle {
                        font_size: 10.0,
                        line_height: 12.0,
                        color: ColorRgba::WHITE,
                        ..Default::default()
                    },
                )),
            });
        }
        RenderFrameRequest::new(
            RenderTarget::window("test.glyph-chunks", viewport),
            viewport,
            PaintList { items },
        )
    }

    fn pixel_rgba(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
        let start = (y * width + x) * 4;
        pixels[start..start + 4].try_into().expect("pixel range")
    }
}
