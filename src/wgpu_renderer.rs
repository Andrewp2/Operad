#![cfg(feature = "wgpu")]

use std::cmp::max;
use std::collections::HashMap;
use std::mem;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use glyphon::{
    Attrs as GlyphAttrs, Buffer as GlyphBuffer, Cache as GlyphCache, Color as GlyphColor,
    Family as GlyphFamily, FontSystem as GlyphFontSystem, Metrics as GlyphMetrics,
    PrepareError as GlyphPrepareError, RenderError as GlyphRenderError,
    Resolution as GlyphResolution, Shaping as GlyphShaping, Stretch as GlyphStretch,
    Style as GlyphFontStyle, SwashCache as GlyphSwashCache, TextArea as GlyphTextArea,
    TextAtlas as GlyphTextAtlas, TextBounds as GlyphTextBounds, TextRenderer as GlyphTextRenderer,
    Viewport as GlyphViewport, Weight as GlyphWeight, Wrap as GlyphWrap,
};
use pollster::block_on;
use wgpu::{
    BufferUsages, Extent3d, Origin3d, TexelCopyBufferInfo, TexelCopyBufferLayout,
    TexelCopyTextureInfo, TextureFormat, COPY_BYTES_PER_ROW_ALIGNMENT,
};

use crate::accessibility::AccessibilityCapabilities;
use crate::platform::{
    BackendAdapterKind, BackendCapabilities, LayerCapabilities, PixelSize,
    PlatformServiceCapabilities, RenderingCapabilities, ResourceCapabilities,
};
use crate::{
    ColorRgba, FontFamily, FontStretch, FontStyle, FrameTiming, ImageAlignment, ImageFit,
    PaintKind, PathVerb, PixelRect, RenderError, RenderFrameOutput, RenderFrameRequest,
    RenderTarget, RenderTargetKind, RenderedImage, RendererAdapter, ResourceFormat,
    ResourceResolver, ResourceUpdate, StrokeStyle, TextStyle, TextWrap, UiNodeId, UiPoint, UiRect,
    UiSize, DEFAULT_CPU_SNAPSHOT_BACKGROUND,
};

const OFFSCREEN_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
const GLYPH_TEXT_CHUNK_SIZE: usize = 8;
const GPU_TIMESTAMP_QUERY_BYTES: u64 = 16;

const WGPU_UI_SHADER: &str = r#"
struct Scene {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

struct TriangleInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct RectInput {
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct TexturedRectInput {
    @location(0) rect: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) tint: vec4<f32>,
};

struct SdfRectInput {
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) radius: f32,
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

struct SdfRectOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_position: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) radius: f32,
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
fn vs_rect(@builtin(vertex_index) vertex_index: u32, input: RectInput) -> VertexOutput {
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
    var output: VertexOutput;
    let x = position.x / scene.viewport.x * 2.0 - 1.0;
    let y = 1.0 - position.y / scene.viewport.y * 2.0;
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
fn vs_sdf_rect(@builtin(vertex_index) vertex_index: u32, input: SdfRectInput) -> SdfRectOutput {
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
    var output: SdfRectOutput;
    let x = position.x / scene.viewport.x * 2.0 - 1.0;
    let y = 1.0 - position.y / scene.viewport.y * 2.0;
    output.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    output.color = input.color;
    output.local_position = unit * input.rect.zw;
    output.size = input.rect.zw;
    output.radius = input.radius;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}

@fragment
fn fs_textured(input: TexturedVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(image_texture, image_sampler, input.uv) * input.tint;
}

@fragment
fn fs_sdf_rect(input: SdfRectOutput) -> @location(0) vec4<f32> {
    let radius = clamp(input.radius, 0.0, min(input.size.x, input.size.y) * 0.5);
    let half_size = input.size * 0.5;
    let centered = input.local_position - half_size;
    let q = abs(centered) - half_size + vec2<f32>(radius, radius);
    let distance = length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
    let alpha = 1.0 - smoothstep(-0.75, 0.75, distance);
    return vec4<f32>(input.color.rgb, input.color.a * alpha);
}
"#;

#[derive(Debug)]
pub struct WgpuRenderer {
    context: Option<WgpuContext>,
    geometry: RenderGeometry,
}

struct WgpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline_layout: wgpu::PipelineLayout,
    texture_pipeline_layout: wgpu::PipelineLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_sampler: wgpu::Sampler,
    shader: wgpu::ShaderModule,
    triangle_pipelines: HashMap<TextureFormat, wgpu::RenderPipeline>,
    rect_pipelines: HashMap<TextureFormat, wgpu::RenderPipeline>,
    textured_rect_pipelines: HashMap<TextureFormat, wgpu::RenderPipeline>,
    sdf_rect_pipelines: HashMap<TextureFormat, wgpu::RenderPipeline>,
    scene_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_capacity: u64,
    rect_buffer: Option<wgpu::Buffer>,
    rect_capacity: u64,
    textured_rect_buffer: Option<wgpu::Buffer>,
    textured_rect_capacity: u64,
    sdf_rect_buffer: Option<wgpu::Buffer>,
    sdf_rect_capacity: u64,
    textures: HashMap<String, WgpuTextureResource>,
    font_system: GlyphFontSystem,
    swash_cache: GlyphSwashCache,
    glyph_cache: GlyphCache,
    glyph_viewport: GlyphViewport,
    glyph_atlas: Option<GlyphTextAtlas>,
    glyph_format: Option<TextureFormat>,
    glyph_buffer_cache: HashMap<TextBufferKey, CachedGlyphBuffer>,
    glyph_scratch_buffer_cache: HashMap<UiNodeId, CachedGlyphBuffer>,
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
            .field("rect_pipelines", &self.rect_pipelines.len())
            .field(
                "textured_rect_pipelines",
                &self.textured_rect_pipelines.len(),
            )
            .field("sdf_rect_pipelines", &self.sdf_rect_pipelines.len())
            .field("textures", &self.textures.len())
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
    fn new(device: wgpu::Device, queue: wgpu::Queue) -> Result<Self, RenderError> {
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
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
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
                bind_group_layouts: &[&bind_group_layout, &texture_bind_group_layout],
                push_constant_ranges: &[],
            });
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("operad-wgpu-texture-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
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
            pipeline_layout,
            texture_pipeline_layout,
            texture_bind_group_layout,
            texture_sampler,
            shader,
            triangle_pipelines: HashMap::new(),
            rect_pipelines: HashMap::new(),
            textured_rect_pipelines: HashMap::new(),
            sdf_rect_pipelines: HashMap::new(),
            scene_buffer,
            scene_bind_group,
            vertex_buffer: None,
            vertex_capacity: 0,
            rect_buffer: None,
            rect_capacity: 0,
            textured_rect_buffer: None,
            textured_rect_capacity: 0,
            sdf_rect_buffer: None,
            sdf_rect_capacity: 0,
            textures,
            font_system: GlyphFontSystem::new(),
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

    fn triangle_pipeline(&mut self, format: TextureFormat) -> &wgpu::RenderPipeline {
        if !self.triangle_pipelines.contains_key(&format) {
            let pipeline = self.create_pipeline(
                format,
                "vs_triangle",
                "fs_main",
                &[GpuVertex::layout()],
                &self.pipeline_layout,
            );
            self.triangle_pipelines.insert(format, pipeline);
        }
        self.triangle_pipelines
            .get(&format)
            .expect("pipeline was inserted before lookup")
    }

    fn rect_pipeline(&mut self, format: TextureFormat) -> &wgpu::RenderPipeline {
        if !self.rect_pipelines.contains_key(&format) {
            let pipeline = self.create_pipeline(
                format,
                "vs_rect",
                "fs_main",
                &[GpuRectInstance::layout()],
                &self.pipeline_layout,
            );
            self.rect_pipelines.insert(format, pipeline);
        }
        self.rect_pipelines
            .get(&format)
            .expect("pipeline was inserted before lookup")
    }

    fn textured_rect_pipeline(&mut self, format: TextureFormat) -> &wgpu::RenderPipeline {
        if !self.textured_rect_pipelines.contains_key(&format) {
            let pipeline = self.create_pipeline(
                format,
                "vs_textured_rect",
                "fs_textured",
                &[GpuTexturedRectInstance::layout()],
                &self.texture_pipeline_layout,
            );
            self.textured_rect_pipelines.insert(format, pipeline);
        }
        self.textured_rect_pipelines
            .get(&format)
            .expect("pipeline was inserted before lookup")
    }

    fn sdf_rect_pipeline(&mut self, format: TextureFormat) -> &wgpu::RenderPipeline {
        if !self.sdf_rect_pipelines.contains_key(&format) {
            let pipeline = self.create_pipeline(
                format,
                "vs_sdf_rect",
                "fs_sdf_rect",
                &[GpuSdfRectInstance::layout()],
                &self.pipeline_layout,
            );
            self.sdf_rect_pipelines.insert(format, pipeline);
        }
        self.sdf_rect_pipelines
            .get(&format)
            .expect("pipeline was inserted before lookup")
    }

    fn create_pipeline(
        &self,
        format: TextureFormat,
        vertex_entry_point: &'static str,
        fragment_entry_point: &'static str,
        vertex_buffers: &[wgpu::VertexBufferLayout<'static>],
        layout: &wgpu::PipelineLayout,
    ) -> wgpu::RenderPipeline {
        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("operad-wgpu-ui-pipeline"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some(vertex_entry_point),
                    buffers: vertex_buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
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
                multiview: None,
                cache: None,
            })
    }

    fn write_scene_uniform(&self, size: PixelSize) {
        self.queue
            .write_buffer(&self.scene_buffer, 0, &pack_scene_uniform(size));
    }

    fn vertex_buffer_for(&mut self, vertices: &[GpuVertex]) -> Option<wgpu::Buffer> {
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

        let buffer = self
            .vertex_buffer
            .as_ref()
            .expect("vertex buffer is allocated before upload");
        self.queue.write_buffer(buffer, 0, vertex_bytes);
        Some(buffer.clone())
    }

    fn rect_buffer_for(&mut self, rects: &[GpuRectInstance]) -> Option<wgpu::Buffer> {
        if rects.is_empty() {
            return None;
        }

        let rect_bytes = rect_instance_bytes(rects);
        let required = u64::try_from(rect_bytes.len()).ok()?;
        if self.rect_capacity < required {
            let capacity = required.next_power_of_two();
            self.rect_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("operad-wgpu-ui-rect-instances"),
                size: capacity,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.rect_capacity = capacity;
        }

        let buffer = self
            .rect_buffer
            .as_ref()
            .expect("rect instance buffer is allocated before upload");
        self.queue.write_buffer(buffer, 0, rect_bytes);
        Some(buffer.clone())
    }

    fn textured_rect_buffer_for(
        &mut self,
        rects: &[GpuTexturedRectInstance],
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

        let buffer = self
            .textured_rect_buffer
            .as_ref()
            .expect("textured rect instance buffer is allocated before upload");
        self.queue.write_buffer(buffer, 0, rect_bytes);
        Some(buffer.clone())
    }

    fn sdf_rect_buffer_for(&mut self, rects: &[GpuSdfRectInstance]) -> Option<wgpu::Buffer> {
        if rects.is_empty() {
            return None;
        }

        let rect_bytes = sdf_rect_instance_bytes(rects);
        let required = u64::try_from(rect_bytes.len()).ok()?;
        if self.sdf_rect_capacity < required {
            let capacity = required.next_power_of_two();
            self.sdf_rect_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("operad-wgpu-ui-sdf-rect-instances"),
                size: capacity,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.sdf_rect_capacity = capacity;
        }

        let buffer = self
            .sdf_rect_buffer
            .as_ref()
            .expect("sdf rect instance buffer is allocated before upload");
        self.queue.write_buffer(buffer, 0, rect_bytes);
        Some(buffer.clone())
    }

    fn upload_resource_updates(&mut self, updates: &[ResourceUpdate]) -> Result<(), RenderError> {
        for update in updates {
            self.upload_resource_update(update)?;
        }
        Ok(())
    }

    fn upload_resource_update(&mut self, update: &ResourceUpdate) -> Result<(), RenderError> {
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

        let recreate = self
            .textures
            .get(&key)
            .is_none_or(|texture| texture.size != size);
        if recreate {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("operad-wgpu-resource-texture"),
                size: Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: OFFSCREEN_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
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
                    texture,
                    _view: view,
                    bind_group,
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
        let texture = self.textures.get(&key).ok_or_else(|| {
            RenderError::Backend("wgpu texture upload target missing".to_string())
        })?;
        self.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture.texture,
                mip_level: 0,
                origin: Origin3d {
                    x: dirty_rect.x,
                    y: dirty_rect.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
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
        Ok(())
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

        self.ensure_glyphon_text(format);
        let render_keys = visible_texts
            .iter()
            .map(|text| TextRenderKey::new(text, size))
            .collect::<Vec<_>>();
        self.glyph_scene_active = false;
        self.glyph_chunks_active = false;
        self.glyph_generation = self.glyph_generation.wrapping_add(1);
        let generation = self.glyph_generation;
        self.glyph_viewport.update(
            &self.queue,
            GlyphResolution {
                width: size.width,
                height: size.height,
            },
        );

        for (text, render_key) in visible_texts.iter().zip(render_keys.iter()) {
            self.ensure_glyph_buffer(text, &render_key.buffer, generation);
        }

        if visible_texts.len() > GLYPH_TEXT_CHUNK_SIZE {
            self.glyph_chunk_order =
                self.prepare_glyphon_text_chunks(size, &visible_texts, &render_keys, generation)?;
            self.glyph_chunks_active = true;
        } else if self.glyph_scene_renderer.is_some() && self.glyph_scene_key == render_keys {
            self.glyph_scene_active = true;
        } else {
            self.prepare_glyphon_text_scene(size, &visible_texts, &render_keys)?;
            self.glyph_scene_active = true;
        }
        self.prune_glyphon_text_cache();

        Ok(true)
    }

    fn prepare_glyphon_text_batches(
        &mut self,
        size: PixelSize,
        format: TextureFormat,
        geometry: &RenderGeometry,
    ) -> Result<Vec<Vec<TextChunkKey>>, RenderError> {
        let mut batch_chunks = vec![Vec::new(); geometry.batches.len()];
        if geometry.texts.is_empty() || size.width == 0 || size.height == 0 {
            return Ok(batch_chunks);
        }

        self.ensure_glyphon_text(format);
        self.glyph_scene_active = false;
        self.glyph_chunks_active = false;
        self.glyph_chunk_order.clear();
        self.glyph_generation = self.glyph_generation.wrapping_add(1);
        let generation = self.glyph_generation;
        self.glyph_viewport.update(
            &self.queue,
            GlyphResolution {
                width: size.width,
                height: size.height,
            },
        );

        let target_rect = UiRect::new(0.0, 0.0, size.width as f32, size.height as f32);
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
                self.ensure_glyph_buffer(text, &render_key.buffer, generation);
            }
            let chunks =
                self.prepare_glyphon_text_chunks(size, &visible_texts, &render_keys, generation)?;
            self.glyph_chunk_order.extend(chunks.iter().cloned());
            batch_chunks[batch_index] = chunks;
            self.glyph_chunks_active = true;
        }
        self.prune_glyphon_text_cache();

        Ok(batch_chunks)
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

        if let Some(cached) = self.glyph_scratch_buffer_cache.get_mut(&text.node) {
            if cached.key != *buffer_key {
                sync_glyph_buffer(
                    &mut cached.buffer,
                    &mut self.font_system,
                    text,
                    Some(&cached.key),
                    buffer_key,
                );
                cached.key = buffer_key.clone();
                cached.stable_hits = 0;
            } else {
                cached.stable_hits = cached.stable_hits.saturating_add(1);
            }
            cached.last_used_generation = generation;
            if cached.stable_hits >= 1 {
                let cached = self
                    .glyph_scratch_buffer_cache
                    .remove(&text.node)
                    .expect("scratch glyph buffer exists before promotion");
                self.glyph_buffer_cache.insert(buffer_key.clone(), cached);
            }
            return;
        }

        let mut buffer = GlyphBuffer::new_empty(GlyphMetrics::new(1.0, 1.0));
        sync_glyph_buffer(&mut buffer, &mut self.font_system, text, None, buffer_key);
        self.glyph_scratch_buffer_cache.insert(
            text.node,
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
    ) -> Result<(), RenderError> {
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
    ) -> Result<Vec<TextChunkKey>, RenderError> {
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
    ) -> Result<GlyphTextRenderer, RenderError> {
        let mut text_areas = Vec::with_capacity(texts.len());
        for (text, render_key) in texts.iter().zip(render_keys.iter()) {
            let buffer = self
                .glyph_buffer_cache
                .get(&render_key.buffer)
                .or_else(|| self.glyph_scratch_buffer_cache.get(&text.node))
                .ok_or_else(|| {
                    RenderError::Backend("glyph text buffer missing from cache".to_string())
                })?;
            text_areas.push(GlyphTextArea {
                buffer: &buffer.buffer,
                left: text.rect.x,
                top: text.rect.y,
                scale: 1.0,
                bounds: glyph_text_bounds(text.clip, size),
                default_color: glyph_color(text.style.color, text.opacity),
                custom_glyphs: &[],
            });
        }
        let atlas = self
            .glyph_atlas
            .as_mut()
            .expect("glyph atlas is initialized before scene text renderer");
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
            .map_err(glyph_prepare_error)?;
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

        let atlas = GlyphTextAtlas::new(&self.device, &self.queue, &self.glyph_cache, format);
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
            .poll(wgpu::PollType::Wait)
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

    fn discard_view(&mut self, size: PixelSize, format: TextureFormat) -> wgpu::TextureView {
        let recreate = self
            .discard_target
            .as_ref()
            .is_none_or(|target| target.size != size || target.format != format);
        if recreate {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("operad-wgpu-discard-texture"),
                size: Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
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
            .expect("discard target is cached before use")
            .view
            .clone()
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
    texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
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
struct GpuRectInstance {
    rect: [f32; 4],
    color: [f32; 4],
}

impl GpuRectInstance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];

    fn new(rect: UiRect, color: [f32; 4]) -> Self {
        Self {
            rect: [rect.x, rect.y, rect.width, rect.height],
            color,
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
struct GpuSdfRectInstance {
    rect: [f32; 4],
    color: [f32; 4],
    radius: f32,
    _pad: [f32; 3],
}

impl GpuSdfRectInstance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32];

    fn new(rect: UiRect, color: [f32; 4], radius: f32) -> Self {
        Self {
            rect: [rect.x, rect.y, rect.width, rect.height],
            color,
            radius,
            _pad: [0.0; 3],
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
    Rect,
    Triangle,
    TexturedRect,
    SdfRect,
    Text,
}

#[derive(Debug, Clone)]
struct GeometryBatch {
    kind: GeometryBatchKind,
    clip: UiRect,
    texture_key: Option<String>,
    start: u32,
    count: u32,
}

#[derive(Debug, Clone)]
struct TextPaint {
    node: UiNodeId,
    rect: UiRect,
    clip: UiRect,
    text: String,
    style: TextStyle,
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
        })
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
        }
    }
}

#[derive(Debug, Clone, Default)]
struct RenderGeometry {
    rects: Vec<GpuRectInstance>,
    textured_rects: Vec<GpuTexturedRectInstance>,
    sdf_rects: Vec<GpuSdfRectInstance>,
    vertices: Vec<GpuVertex>,
    texts: Vec<TextPaint>,
    batches: Vec<GeometryBatch>,
}

impl RenderGeometry {
    fn clear(&mut self) {
        self.rects.clear();
        self.textured_rects.clear();
        self.sdf_rects.clear();
        self.vertices.clear();
        self.texts.clear();
        self.batches.clear();
    }

    fn push_batch(
        &mut self,
        kind: GeometryBatchKind,
        clip: UiRect,
        texture_key: Option<&str>,
        start: u32,
        count: u32,
    ) {
        if let Some(batch) = self.batches.last_mut() {
            if batch.kind == kind
                && batch.clip == clip
                && batch.texture_key.as_deref() == texture_key
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
            texture_key: texture_key.map(str::to_owned),
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

    fn push_quad(&mut self, clip: UiRect, points: [UiPoint; 4], color: [f32; 4]) {
        let Ok(vertex_start) = u32::try_from(self.vertices.len()) else {
            return;
        };
        self.vertices.reserve(6);
        for point in [
            points[0], points[1], points[2], points[0], points[2], points[3],
        ] {
            self.vertices.push(GpuVertex::new(point, color));
        }
        self.push_batch(GeometryBatchKind::Triangle, clip, None, vertex_start, 6);
    }

    fn push_rect(&mut self, clip: UiRect, rect: UiRect, color: [f32; 4]) {
        let Ok(start) = u32::try_from(self.rects.len()) else {
            return;
        };
        self.rects.push(GpuRectInstance::new(rect, color));
        self.push_batch(GeometryBatchKind::Rect, clip, None, start, 1);
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
            Some(texture_key),
            start,
            1,
        );
    }

    fn push_sdf_rect(&mut self, clip: UiRect, rect: UiRect, color: [f32; 4], radius: f32) {
        let Ok(start) = u32::try_from(self.sdf_rects.len()) else {
            return;
        };
        self.sdf_rects
            .push(GpuSdfRectInstance::new(rect, color, radius));
        self.push_batch(GeometryBatchKind::SdfRect, clip, None, start, 1);
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
        }
    }

    pub fn with_device_queue(
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Result<Self, RenderError> {
        Ok(Self {
            context: Some(WgpuContext::new(device, queue)?),
            geometry: RenderGeometry::default(),
        })
    }

    pub fn warm_up(&mut self) -> Result<(), RenderError> {
        let context = self.ensure_context()?;
        let _ = context.triangle_pipeline(OFFSCREEN_FORMAT);
        let _ = context.rect_pipeline(OFFSCREEN_FORMAT);
        let _ = context.textured_rect_pipeline(OFFSCREEN_FORMAT);
        let _ = context.sdf_rect_pipeline(OFFSCREEN_FORMAT);
        context.prepare_glyphon_text(
            PixelSize::new(512, 128),
            OFFSCREEN_FORMAT,
            &[TextPaint {
                node: UiNodeId(0),
                rect: UiRect::new(0.0, 0.0, 512.0, 128.0),
                clip: UiRect::new(0.0, 0.0, 512.0, 128.0),
                text: "Operad glyphon warmup 0123456789 reusable toolkit surface".to_string(),
                style: TextStyle {
                    font_size: 12.0,
                    line_height: 16.0,
                    color: ColorRgba::WHITE,
                    ..Default::default()
                },
                opacity: 1.0,
            }],
        )?;
        Ok(())
    }

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

        let size = render_target_pixel_size(&request.target, request.viewport)?;
        let target_kind = request.target.kind();
        let clear_color = clear_color_for_request(&request);
        let mut context = WgpuContext::new(device.clone(), queue.clone())?;
        context.upload_resource_updates(&request.resource_updates)?;
        self.geometry.clear();
        build_geometry_into(&mut self.geometry, &request.paint, &mut context);
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

    fn validate_resource_updates(
        &self,
        request: &RenderFrameRequest,
        resolver: &dyn ResourceResolver,
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
            let id = update.descriptor.handle.id();
            if resolver.resolve_resource(id).is_none() {
                return Err(RenderError::MissingResource(id.clone()));
            }
        }
        Ok(())
    }

    fn ensure_context(&mut self) -> Result<&mut WgpuContext, RenderError> {
        if self.context.is_none() {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
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

            self.context = Some(WgpuContext::new(device, queue)?);
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

#[derive(Debug)]
pub struct WgpuSurfaceRenderer<'window> {
    renderer: WgpuRenderer,
    surface: wgpu::Surface<'window>,
    surface_config: wgpu::SurfaceConfiguration,
}

impl<'window> WgpuSurfaceRenderer<'window> {
    pub fn new(
        surface: wgpu::Surface<'window>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        mut surface_config: wgpu::SurfaceConfiguration,
    ) -> Result<Self, RenderError> {
        surface_config
            .usage
            .insert(wgpu::TextureUsages::RENDER_ATTACHMENT);
        let renderer = WgpuRenderer::with_device_queue(device, queue)?;
        if surface_config.width > 0 && surface_config.height > 0 {
            let context = renderer.context.as_ref().ok_or_else(|| {
                RenderError::Backend("wgpu surface renderer failed to initialize".to_string())
            })?;
            surface.configure(&context.device, &surface_config);
        }
        Ok(Self {
            renderer,
            surface,
            surface_config,
        })
    }

    fn render_to_surface(
        &mut self,
        size: PixelSize,
        clear_color: ColorRgba,
        collect_gpu_timing: bool,
    ) -> Result<Option<Duration>, RenderError> {
        if size.width == 0 || size.height == 0 {
            return Ok(None);
        }

        self.configure_surface(size)?;
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.configure_surface(size)?;
                self.surface.get_current_texture().map_err(|error| {
                    RenderError::Backend(format!("surface reacquire failed: {error}"))
                })?
            }
            Err(wgpu::SurfaceError::Timeout) => {
                return Err(RenderError::Backend(
                    "surface acquire timed out".to_string(),
                ));
            }
            Err(other) => {
                return Err(RenderError::Backend(format!(
                    "surface acquire failed: {other}"
                )));
            }
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
            self.surface_config.format,
            size,
            &self.renderer.geometry,
            clear_color,
            true,
            collect_gpu_timing,
        )?;
        context.queue.submit(Some(encoder.finish()));
        frame.present();
        if gpu_timer_used {
            context.read_gpu_render_duration()
        } else {
            Ok(None)
        }
    }

    fn configure_surface(&mut self, size: PixelSize) -> Result<(), RenderError> {
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let context = self.renderer.context.as_ref().ok_or_else(|| {
            RenderError::Backend("wgpu surface renderer missing context".to_string())
        })?;

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
        if needs_resize {
            self.surface
                .configure(&context.device, &self.surface_config);
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

        let batch_started = Instant::now();
        let batches = request.batches();
        let batch_duration = batch_started.elapsed();

        self.renderer
            .validate_resource_updates(&request, resolver)?;

        let size = render_target_pixel_size(&request.target, request.viewport)?;
        let clear_color = clear_color_for_request(&request);
        self.renderer.geometry.clear();
        {
            let context = self.renderer.context.as_mut().ok_or_else(|| {
                RenderError::Backend("wgpu surface renderer missing context".to_string())
            })?;
            context.upload_resource_updates(&request.resource_updates)?;
            build_geometry_into(&mut self.renderer.geometry, &request.paint, context);
        }
        let mut output = RenderFrameOutput::new(request.target.clone());
        output.painted_items = request.paint.items.len();
        output.batches = batches;
        output.dirty_regions = request.dirty_regions.clone();

        let render_started = Instant::now();
        let gpu_render_duration =
            self.render_to_surface(size, clear_color, request.options.collect_gpu_timing)?;

        let mut timings = FrameTiming::new()
            .section("batch", batch_duration)
            .section("render", render_started.elapsed());
        if let Some(duration) = gpu_render_duration {
            timings = timings.section("gpu-render", duration);
        }
        output.timings = timings;
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
            })
            .accessibility(AccessibilityCapabilities::NONE)
    }

    fn render_frame(
        &mut self,
        request: RenderFrameRequest,
        resolver: &dyn ResourceResolver,
    ) -> Result<RenderFrameOutput, RenderError> {
        let batch_started = Instant::now();
        let batches = request.batches();
        let batch_duration = batch_started.elapsed();

        self.validate_resource_updates(&request, resolver)?;

        let size = render_target_pixel_size(&request.target, request.viewport)?;
        let target_kind = request.target.kind();
        let clear_color = clear_color_for_request(&request);
        {
            let context = self.ensure_context()?;
            context.upload_resource_updates(&request.resource_updates)?;
        }
        let mut geometry = mem::take(&mut self.geometry);
        geometry.clear();
        {
            let context = self.context.as_mut().ok_or_else(|| {
                RenderError::Backend("wgpu backend failed to initialize".to_string())
            })?;
            build_geometry_into(&mut geometry, &request.paint, context);
        }
        self.geometry = geometry;
        let mut output = RenderFrameOutput::new(request.target);
        output.painted_items = request.paint.items.len();
        output.batches = batches;
        output.dirty_regions = request.dirty_regions;

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
                self.render_discard_frame(size, clear_color, request.options.collect_gpu_timing)?;
        }

        let mut timings = FrameTiming::new()
            .section("batch", batch_duration)
            .section("render", render_started.elapsed());
        if let Some(duration) = gpu_render_duration {
            timings = timings.section("gpu-render", duration);
        }
        output.timings = timings;
        Ok(output)
    }
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
    let view = context.discard_view(size, OFFSCREEN_FORMAT);
    record_render_pass(
        context,
        encoder,
        &view,
        OFFSCREEN_FORMAT,
        size,
        geometry,
        clear_color,
        true,
        collect_gpu_timing,
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

    let texture = context.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("operad-wgpu-snapshot-texture"),
        size: Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: OFFSCREEN_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
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
        clear_color,
        true,
        false,
    )?;

    let padded_row_stride = upload_row_stride(size.width)?;
    let padded_size = u64::from(padded_row_stride)
        .checked_mul(u64::from(size.height))
        .ok_or_else(|| RenderError::Backend("wgpu readback buffer too large".to_string()))?;
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
        .poll(wgpu::PollType::Wait)
        .map_err(|error| RenderError::Backend(format!("wgpu poll failed: {error}")))?;

    let readback_slice = readback_buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    readback_slice.map_async(wgpu::MapMode::Read, move |status| {
        let _ = tx.send(status);
    });
    let _ = context
        .device
        .poll(wgpu::PollType::Wait)
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

fn record_render_pass(
    context: &mut WgpuContext,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    format: TextureFormat,
    size: PixelSize,
    geometry: &RenderGeometry,
    clear_color: ColorRgba,
    render_text: bool,
    collect_gpu_timing: bool,
) -> Result<bool, RenderError> {
    if size.width == 0 || size.height == 0 {
        return Ok(false);
    }

    context.write_scene_uniform(size);
    let text_batch_chunks = if render_text {
        context.prepare_glyphon_text_batches(size, format, geometry)?
    } else {
        vec![Vec::new(); geometry.batches.len()]
    };
    let bind_group = context.scene_bind_group.clone();
    let rect_buffer = context.rect_buffer_for(&geometry.rects);
    let textured_rect_buffer = context.textured_rect_buffer_for(&geometry.textured_rects);
    let sdf_rect_buffer = context.sdf_rect_buffer_for(&geometry.sdf_rects);
    let vertex_buffer = context.vertex_buffer_for(&geometry.vertices);
    let texture_bind_groups = geometry
        .batches
        .iter()
        .filter_map(|batch| batch.texture_key.as_ref())
        .filter_map(|key| {
            context
                .textures
                .get(key)
                .map(|texture| (key.clone(), texture.bind_group.clone()))
        })
        .collect::<HashMap<_, _>>();

    let clear = wgpu_color(clear_color);
    let rect_pipeline = context.rect_pipeline(format).clone();
    let triangle_pipeline = context.triangle_pipeline(format).clone();
    let textured_rect_pipeline = context.textured_rect_pipeline(format).clone();
    let sdf_rect_pipeline = context.sdf_rect_pipeline(format).clone();
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
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes,
    });
    for (batch_index, batch) in geometry.batches.iter().enumerate() {
        let Some(scissor) = scissor_rect(batch.clip, size) else {
            continue;
        };
        pass.set_scissor_rect(scissor.x, scissor.y, scissor.width, scissor.height);
        pass.set_bind_group(0, &bind_group, &[]);
        match batch.kind {
            GeometryBatchKind::Rect => {
                let Some(rect_buffer) = &rect_buffer else {
                    continue;
                };
                pass.set_pipeline(&rect_pipeline);
                pass.set_vertex_buffer(0, rect_buffer.slice(..));
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
            GeometryBatchKind::SdfRect => {
                let Some(sdf_rect_buffer) = &sdf_rect_buffer else {
                    continue;
                };
                pass.set_pipeline(&sdf_rect_pipeline);
                pass.set_vertex_buffer(0, sdf_rect_buffer.slice(..));
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
) {
    for item in &paint.items {
        let clip = item.clip_rect;
        match &item.kind {
            PaintKind::Rect {
                fill,
                stroke,
                corner_radius,
            } => {
                let rect = item.transform.transform_rect(item.rect);
                push_fill_rect_with_radius(
                    geometry,
                    rect,
                    clip,
                    *fill,
                    item.opacity,
                    corner_radius * item.transform.scale.max(0.0),
                );
                if let Some(stroke) = stroke {
                    push_stroke_rect(geometry, rect, clip, *stroke, item.opacity);
                }
            }
            PaintKind::Text(text) => push_text(
                geometry,
                item.node,
                item.rect,
                clip,
                &text.text,
                &text.style,
                item.opacity,
                item.transform,
            ),
            PaintKind::SceneText(text) => {
                push_text(
                    geometry,
                    item.node,
                    text.rect,
                    clip,
                    &text.text,
                    &text.style,
                    item.opacity,
                    item.transform,
                );
            }
            PaintKind::Canvas(canvas) => {
                push_canvas(
                    geometry,
                    item.transform.transform_rect(item.rect),
                    clip,
                    &canvas.key,
                    item.opacity,
                );
            }
            PaintKind::Line { from, to, stroke } => {
                push_line(
                    geometry,
                    item.transform.transform_point(*from),
                    item.transform.transform_point(*to),
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
                let center = item.transform.transform_point(*center);
                let radius = radius * item.transform.scale.max(0.0);
                push_fill_circle(geometry, center, radius, clip, *fill, item.opacity);
                if let Some(stroke) = stroke {
                    push_stroke_circle(geometry, center, radius, clip, *stroke, item.opacity);
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
                    .map(|point| item.transform.transform_point(point))
                    .collect::<Vec<_>>();
                push_polygon(geometry, &points, clip, *fill, item.opacity);
                if let Some(stroke) = stroke {
                    push_polyline(geometry, &points, clip, *stroke, item.opacity, true);
                }
            }
            PaintKind::Image { key, tint } => {
                push_image(
                    geometry,
                    item.transform.transform_rect(item.rect),
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
            PaintKind::RichRect(rect_primitive) => {
                let rect = item.transform.transform_rect(rect_primitive.rect);
                for effect in &rect_primitive.effects {
                    let spread = effect.spread.max(0.0) + effect.blur_radius.max(0.0) * 0.25;
                    let effect_rect = UiRect::new(
                        rect.x + effect.offset.x - spread,
                        rect.y + effect.offset.y - spread,
                        rect.width + spread * 2.0,
                        rect.height + spread * 2.0,
                    );
                    push_fill_rect(geometry, effect_rect, clip, effect.color, item.opacity);
                }
                push_fill_rect_with_radius(
                    geometry,
                    rect,
                    clip,
                    rect_primitive.fill.fallback_color(),
                    item.opacity,
                    rect_primitive.corner_radii.max_radius() * item.transform.scale.max(0.0),
                );
                if let Some(stroke) = rect_primitive.stroke {
                    push_stroke_rect(geometry, rect, clip, stroke.style, item.opacity);
                }
            }
            PaintKind::Path(path) => {
                let path_points = path
                    .verbs
                    .iter()
                    .filter_map(|verb| path_verb_end_point(*verb))
                    .map(|point| item.transform.transform_point(point))
                    .collect::<Vec<_>>();
                if let Some(fill) = &path.fill {
                    push_polygon(
                        geometry,
                        &path_points,
                        clip,
                        fill.fallback_color(),
                        item.opacity,
                    );
                }
                if let Some(stroke) = path.stroke {
                    push_polyline(
                        geometry,
                        &path_points,
                        clip,
                        stroke.style,
                        item.opacity,
                        false,
                    );
                }
            }
            PaintKind::ImagePlacement(image) => {
                push_image(
                    geometry,
                    item.transform.transform_rect(image.rect),
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
}

fn push_fill_rect(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
) {
    if color.a == 0 || opacity <= 0.0 {
        return;
    }
    push_fill_rect_with_color(geometry, rect, clip, color_as_vertex(color, opacity));
}

fn push_fill_rect_with_radius(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
    radius: f32,
) {
    if color.a == 0 || opacity <= 0.0 {
        return;
    }
    let radius = radius.max(0.0);
    if radius <= f32::EPSILON {
        push_fill_rect_with_color(geometry, rect, clip, color_as_vertex(color, opacity));
        return;
    }
    if rect.width <= 0.0 || rect.height <= 0.0 || rect.intersection(clip).is_none() {
        return;
    }
    geometry.push_sdf_rect(clip, rect, color_as_vertex(color, opacity), radius);
}

fn push_fill_rect_with_color(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    color: [f32; 4],
) {
    let Some(rect) = snapped_intersection(rect, clip) else {
        return;
    };
    geometry.push_rect(clip, rect, color);
}

fn push_stroke_rect(
    geometry: &mut RenderGeometry,
    rect: UiRect,
    clip: UiRect,
    stroke: StrokeStyle,
    opacity: f32,
) {
    let width = stroke.width.max(1.0);
    push_fill_rect(
        geometry,
        UiRect::new(rect.x, rect.y, rect.width, width),
        clip,
        stroke.color,
        opacity,
    );
    push_fill_rect(
        geometry,
        UiRect::new(rect.x, rect.bottom() - width, rect.width, width),
        clip,
        stroke.color,
        opacity,
    );
    push_fill_rect(
        geometry,
        UiRect::new(rect.x, rect.y, width, rect.height),
        clip,
        stroke.color,
        opacity,
    );
    push_fill_rect(
        geometry,
        UiRect::new(rect.right() - width, rect.y, width, rect.height),
        clip,
        stroke.color,
        opacity,
    );
}

fn push_text(
    geometry: &mut RenderGeometry,
    node: UiNodeId,
    rect: UiRect,
    clip: UiRect,
    text: &str,
    style: &TextStyle,
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
    geometry.push_text(TextPaint {
        node,
        rect,
        clip,
        text: text.to_owned(),
        style,
        opacity,
    });
}

fn push_canvas(geometry: &mut RenderGeometry, rect: UiRect, clip: UiRect, key: &str, opacity: f32) {
    let base = snapshot_color_from_key(key, 210);
    push_fill_rect(geometry, rect, clip, base, opacity);
    let accent = ColorRgba::new(
        base.r.saturating_add(34),
        base.g.saturating_add(24),
        base.b.saturating_add(18),
        255,
    );
    let mut x = rect.x;
    while x < rect.right() {
        push_line(
            geometry,
            UiPoint::new(x, rect.y),
            UiPoint::new(x + rect.height, rect.bottom()),
            clip,
            StrokeStyle::new(accent, 1.0),
            opacity,
        );
        x += 12.0;
    }
}

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
    let base = tint.unwrap_or_else(|| snapshot_color_from_key(key, 235));
    push_fill_rect(geometry, rect, clip, base, opacity);
    let hash = snapshot_hash_str(key);
    let stripe = ColorRgba::new(
        base.r.saturating_sub(((hash >> 8) & 31) as u8),
        base.g.saturating_sub(((hash >> 16) & 31) as u8),
        base.b.saturating_sub(((hash >> 24) & 31) as u8),
        base.a,
    );
    let mut x = rect.x;
    while x < rect.right() {
        push_fill_rect(
            geometry,
            UiRect::new(x, rect.y, 2.0, rect.height),
            clip,
            stripe,
            0.8 * opacity,
        );
        x += 6.0;
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
    if stroke.color.a == 0 || opacity <= 0.0 {
        return;
    }

    let width = stroke.width.max(1.0);
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f32::EPSILON {
        push_fill_rect(
            geometry,
            UiRect::new(from.x, from.y, width, width),
            clip,
            stroke.color,
            opacity,
        );
        return;
    }

    let half = width * 0.5 + 0.75;
    let nx = -dy / length * half;
    let ny = dx / length * half;
    let points = [
        UiPoint::new(from.x + nx, from.y + ny),
        UiPoint::new(to.x + nx, to.y + ny),
        UiPoint::new(to.x - nx, to.y - ny),
        UiPoint::new(from.x - nx, from.y - ny),
    ];
    push_quad(
        geometry,
        points,
        clip,
        color_as_vertex(stroke.color, opacity),
    );
}

fn push_fill_circle(
    geometry: &mut RenderGeometry,
    center: UiPoint,
    radius: f32,
    clip: UiRect,
    color: ColorRgba,
    opacity: f32,
) {
    if radius <= 0.0 || color.a == 0 || opacity <= 0.0 {
        return;
    }
    let color = color_as_vertex(color, opacity);
    let segments = circle_segments(radius);
    let mut vertices = Vec::with_capacity(segments.saturating_mul(3));
    for index in 0..segments {
        let a0 = std::f32::consts::TAU * index as f32 / segments as f32;
        let a1 = std::f32::consts::TAU * (index + 1) as f32 / segments as f32;
        vertices.push(GpuVertex::new(center, color));
        vertices.push(GpuVertex::new(
            UiPoint::new(center.x + radius * a0.cos(), center.y + radius * a0.sin()),
            color,
        ));
        vertices.push(GpuVertex::new(
            UiPoint::new(center.x + radius * a1.cos(), center.y + radius * a1.sin()),
            color,
        ));
    }
    geometry.push_triangle_vertices(clip, &vertices);
}

fn push_stroke_circle(
    geometry: &mut RenderGeometry,
    center: UiPoint,
    radius: f32,
    clip: UiRect,
    stroke: StrokeStyle,
    opacity: f32,
) {
    if radius <= 0.0 || stroke.color.a == 0 || opacity <= 0.0 {
        return;
    }
    let half = stroke.width.max(1.0) * 0.5;
    let inner = (radius - half).max(0.0);
    let outer = radius + half;
    let color = color_as_vertex(stroke.color, opacity);
    let segments = circle_segments(outer);
    let mut vertices = Vec::with_capacity(segments.saturating_mul(6));
    for index in 0..segments {
        let a0 = std::f32::consts::TAU * index as f32 / segments as f32;
        let a1 = std::f32::consts::TAU * (index + 1) as f32 / segments as f32;
        let outer0 = UiPoint::new(center.x + outer * a0.cos(), center.y + outer * a0.sin());
        let outer1 = UiPoint::new(center.x + outer * a1.cos(), center.y + outer * a1.sin());
        let inner0 = UiPoint::new(center.x + inner * a0.cos(), center.y + inner * a0.sin());
        let inner1 = UiPoint::new(center.x + inner * a1.cos(), center.y + inner * a1.sin());
        vertices.extend_from_slice(&[
            GpuVertex::new(outer0, color),
            GpuVertex::new(outer1, color),
            GpuVertex::new(inner1, color),
            GpuVertex::new(outer0, color),
            GpuVertex::new(inner1, color),
            GpuVertex::new(inner0, color),
        ]);
    }
    geometry.push_triangle_vertices(clip, &vertices);
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
    let color = color_as_vertex(color, opacity);
    let mut vertices = Vec::with_capacity((points.len() - 2).saturating_mul(3));
    for index in 1..points.len() - 1 {
        vertices.extend_from_slice(&[
            GpuVertex::new(points[0], color),
            GpuVertex::new(points[index], color),
            GpuVertex::new(points[index + 1], color),
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
            *points
                .last()
                .expect("closed polyline has at least one point"),
            points[0],
            clip,
            stroke,
            opacity,
        );
    }
}

fn push_quad(geometry: &mut RenderGeometry, points: [UiPoint; 4], clip: UiRect, color: [f32; 4]) {
    geometry.push_quad(clip, points, color);
}

fn snapped_intersection(rect: UiRect, clip: UiRect) -> Option<UiRect> {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return None;
    }
    let rect = rect.intersection(clip)?;
    let left = rect.x.floor();
    let top = rect.y.floor();
    let right = rect.right().ceil();
    let bottom = rect.bottom().ceil();
    if !left.is_finite() || !top.is_finite() || !right.is_finite() || !bottom.is_finite() {
        return None;
    }
    if left >= right || top >= bottom {
        return None;
    }
    Some(UiRect::new(left, top, right - left, bottom - top))
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

fn circle_segments(radius: f32) -> usize {
    max(12, (radius.abs().sqrt() * 8.0).ceil() as usize).min(96)
}

fn path_verb_end_point(verb: PathVerb) -> Option<UiPoint> {
    match verb {
        PathVerb::MoveTo(point)
        | PathVerb::LineTo(point)
        | PathVerb::QuadraticTo { to: point, .. }
        | PathVerb::CubicTo { to: point, .. } => Some(point),
        PathVerb::Close => None,
    }
}

fn color_as_vertex(color: ColorRgba, opacity: f32) -> [f32; 4] {
    [
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        (f32::from(color.a) / 255.0 * opacity.clamp(0.0, 1.0)).clamp(0.0, 1.0),
    ]
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
        buffer.set_wrap(font_system, glyph_wrap(text.style.wrap));
    }
    let attrs = glyph_attrs(&text.style);
    buffer.set_text(font_system, &text.text, &attrs, glyph_shaping(&text.text));
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
    GlyphWeight(weight.0)
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

fn glyph_prepare_error(error: GlyphPrepareError) -> RenderError {
    RenderError::Backend(format!("glyphon text prepare failed: {error}"))
}

fn glyph_render_error(error: GlyphRenderError) -> RenderError {
    RenderError::Backend(format!("glyphon text render failed: {error}"))
}

fn clear_color_for_request(request: &RenderFrameRequest) -> ColorRgba {
    if request.options.clear_color == ColorRgba::TRANSPARENT {
        DEFAULT_CPU_SNAPSHOT_BACKGROUND
    } else {
        request.options.clear_color
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

fn render_target_pixel_size(
    target: &RenderTarget,
    viewport: UiSize,
) -> Result<PixelSize, RenderError> {
    match target {
        RenderTarget::Offscreen { size, .. } | RenderTarget::Snapshot { size, .. } => Ok(*size),
        RenderTarget::Window { .. } | RenderTarget::AppOwned { .. } => {
            pixel_size_from_viewport(viewport)
        }
    }
}

fn pixel_size_from_viewport(viewport: UiSize) -> Result<PixelSize, RenderError> {
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
    if viewport.width.round() > u32::MAX as f32 || viewport.height.round() > u32::MAX as f32 {
        return Err(RenderError::Backend(
            "snapshot viewport exceeds u32 pixel dimensions".to_string(),
        ));
    }
    Ok(PixelSize::new(
        viewport.width.round() as u32,
        viewport.height.round() as u32,
    ))
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

fn vertex_bytes(vertices: &[GpuVertex]) -> &[u8] {
    let byte_len = vertices.len().saturating_mul(mem::size_of::<GpuVertex>());
    // GpuVertex is #[repr(C)] and contains only f32 arrays, so its in-memory
    // layout is exactly the vertex buffer layout described to wgpu.
    unsafe { std::slice::from_raw_parts(vertices.as_ptr().cast::<u8>(), byte_len) }
}

fn rect_instance_bytes(rects: &[GpuRectInstance]) -> &[u8] {
    let byte_len = rects
        .len()
        .saturating_mul(mem::size_of::<GpuRectInstance>());
    // GpuRectInstance is #[repr(C)] and contains only f32 arrays matching the
    // instance buffer layout described to wgpu.
    unsafe { std::slice::from_raw_parts(rects.as_ptr().cast::<u8>(), byte_len) }
}

fn textured_rect_instance_bytes(rects: &[GpuTexturedRectInstance]) -> &[u8] {
    let byte_len = rects
        .len()
        .saturating_mul(mem::size_of::<GpuTexturedRectInstance>());
    // GpuTexturedRectInstance is #[repr(C)] and contains only f32 arrays
    // matching the textured instance buffer layout described to wgpu.
    unsafe { std::slice::from_raw_parts(rects.as_ptr().cast::<u8>(), byte_len) }
}

fn sdf_rect_instance_bytes(rects: &[GpuSdfRectInstance]) -> &[u8] {
    let byte_len = rects
        .len()
        .saturating_mul(mem::size_of::<GpuSdfRectInstance>());
    // GpuSdfRectInstance is #[repr(C)] and contains only f32 values matching
    // the SDF instance buffer layout described to wgpu.
    unsafe { std::slice::from_raw_parts(rects.as_ptr().cast::<u8>(), byte_len) }
}

fn rgba_bytes_for_update(update: &ResourceUpdate) -> Result<Vec<u8>, RenderError> {
    if !update.has_expected_byte_len() {
        return Err(RenderError::InvalidResourceUpdate(
            update.descriptor.handle.id().key.clone(),
        ));
    }

    match update.descriptor.format {
        ResourceFormat::Rgba8 => Ok(update.bytes.clone()),
        ResourceFormat::Bgra8 => {
            let mut rgba = Vec::with_capacity(update.bytes.len());
            for bgra in update.bytes.chunks_exact(4) {
                rgba.extend_from_slice(&[bgra[2], bgra[1], bgra[0], bgra[3]]);
            }
            Ok(rgba)
        }
        ResourceFormat::Alpha8 => {
            let mut rgba = Vec::with_capacity(update.bytes.len().saturating_mul(4));
            for alpha in &update.bytes {
                rgba.extend_from_slice(&[255, 255, 255, *alpha]);
            }
            Ok(rgba)
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

fn snapshot_color_from_key(key: &str, alpha: u8) -> ColorRgba {
    let hash = snapshot_hash_str(key);
    ColorRgba::new(
        48 + (hash & 127) as u8,
        58 + ((hash >> 8) & 127) as u8,
        68 + ((hash >> 16) & 127) as u8,
        alpha,
    )
}

fn snapshot_hash_str(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::LayerOrder;
    use crate::{EmptyResourceResolver, PaintItem, PaintList, RenderOptions, TextContent};

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

    fn chunked_text_request(frame: usize) -> RenderFrameRequest {
        const ROWS: usize = 64;
        let viewport = UiSize::new(640.0, 480.0);
        let dirty_row = frame % ROWS;
        let mut items = Vec::with_capacity(ROWS + 1);
        items.push(PaintItem {
            node: UiNodeId(70_000),
            rect: UiRect::new(0.0, 0.0, viewport.width, viewport.height),
            clip_rect: UiRect::new(0.0, 0.0, viewport.width, viewport.height),
            z_index: 0,
            layer_order: LayerOrder::DEFAULT,
            opacity: 1.0,
            transform: Default::default(),
            shader: None,
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
                z_index: 0,
                layer_order: LayerOrder::DEFAULT,
                opacity: 1.0,
                transform: Default::default(),
                shader: None,
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
}
