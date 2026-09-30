//! Packed GPU descriptions of analytic UI shapes. No curve tessellation belongs here.

use std::mem;

use crate::{
    ColorRgba, CornerRadii, PaintEffect, PaintEffectKind, StrokeAlignment, StrokeStyle, UiPoint,
    UiRect,
};

use super::{color_as_vertex, finite_or, normalized_corner_radii_for_rect};

pub(super) const SHADER: &str = concat!(include_str!("shape.wgsl"), include_str!("sdf.wgsl"));
const AA_OUTSET: f32 = 1.0;

// Separate fragment programs keep ordinary fills from executing border,
// storage-buffer search, and Gaussian quadrature on software GPU backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum SdfPipelineKind {
    Fill,
    Border,
    Gradient,
    GradientBorder,
    MultistopGradient,
    MultistopGradientBorder,
    Segment,
    Shadow,
    InsetShadow,
}

impl SdfPipelineKind {
    pub const ALL: [Self; 9] = [
        Self::Fill,
        Self::Border,
        Self::Gradient,
        Self::GradientBorder,
        Self::MultistopGradient,
        Self::MultistopGradientBorder,
        Self::Segment,
        Self::Shadow,
        Self::InsetShadow,
    ];

    pub fn fragment_entry_point(self, srgb: bool) -> &'static str {
        match (self, srgb) {
            (Self::Fill, false) => "fs_sdf",
            (Self::Fill, true) => "fs_sdf_srgb",
            (Self::Border, false) => "fs_sdf_border",
            (Self::Border, true) => "fs_sdf_border_srgb",
            (Self::Gradient, false) => "fs_sdf_gradient",
            (Self::Gradient, true) => "fs_sdf_gradient_srgb",
            (Self::GradientBorder, false) => "fs_sdf_gradient_border",
            (Self::GradientBorder, true) => "fs_sdf_gradient_border_srgb",
            (Self::MultistopGradient, false) => "fs_sdf_multistop",
            (Self::MultistopGradient, true) => "fs_sdf_multistop_srgb",
            (Self::MultistopGradientBorder, false) => "fs_sdf_multistop_border",
            (Self::MultistopGradientBorder, true) => "fs_sdf_multistop_border_srgb",
            (Self::Segment, false) => "fs_sdf_segment",
            (Self::Segment, true) => "fs_sdf_segment_srgb",
            (Self::Shadow, false) => "fs_sdf_shadow",
            (Self::Shadow, true) => "fs_sdf_shadow_srgb",
            (Self::InsetShadow, false) => "fs_sdf_inset",
            (Self::InsetShadow, true) => "fs_sdf_inset_srgb",
        }
    }
}

/// All fields contain 32-bit scalars, without Rust padding. The shader's vertex
/// attributes use this order. `params` is kind, stroke width, stroke alignment,
/// and shadow blur extent; `effect` carries the inset offset and spread.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SdfInstance {
    pub draw_rect: [f32; 4],
    pub shape_rect: [f32; 4],
    pub color: [f32; 4],
    pub border_color: [f32; 4],
    pub radii: [f32; 4],
    pub params: [f32; 4],
    pub gradient_line: [f32; 4],
    pub effect: [f32; 4],
    pub gradient_range: [u32; 2],
}

impl SdfInstance {
    pub fn pipeline_kind(&self) -> SdfPipelineKind {
        match self.params[0] as u8 {
            2 => return SdfPipelineKind::Segment,
            3 => return SdfPipelineKind::Shadow,
            4 => return SdfPipelineKind::InsetShadow,
            _ => {}
        }
        let border = self.params[1] > 0.0 && self.border_color[3] > 0.0;
        match (self.gradient_range[1], border) {
            (0, false) => SdfPipelineKind::Fill,
            (0, true) => SdfPipelineKind::Border,
            (2, false) => SdfPipelineKind::Gradient,
            (2, true) => SdfPipelineKind::GradientBorder,
            (_, false) => SdfPipelineKind::MultistopGradient,
            (_, true) => SdfPipelineKind::MultistopGradientBorder,
        }
    }

    pub fn intersects_clip(&self, clip: UiRect) -> bool {
        let [x, y, width, height] = self.draw_rect;
        let bounds = UiRect::new(x, y, width, height);
        valid_rect(bounds) && bounds.intersection(clip).is_some()
    }

    const ATTRIBUTES: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
        0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4,
        4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4,
        8 => Uint32x2
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }

    pub fn rectangle(
        rect: UiRect,
        color: [f32; 4],
        radii: CornerRadii,
        stroke: Option<(StrokeStyle, StrokeAlignment)>,
        opacity: f32,
    ) -> Option<Self> {
        if !valid_rect(rect) {
            return None;
        }
        let radii = normalized_corner_radii_for_rect(radii, rect.width, rect.height);
        let mut instance = Self::base(rect, color, radii);
        if let Some((stroke, alignment)) = stroke.filter(|(s, _)| s.is_visible()) {
            let width = finite_or(stroke.width, 0.0).max(0.0);
            instance.border_color = color_as_vertex(stroke.color, opacity);
            instance.params[1] = width;
            instance.params[2] = match alignment {
                StrokeAlignment::Inside => 0.0,
                StrokeAlignment::Center => 1.0,
                StrokeAlignment::Outside => 2.0,
            };
            let outset = width * instance.params[2] * 0.5;
            instance.draw_rect = rect_array(expand(rect, outset + AA_OUTSET));
        }
        Some(instance)
    }

    pub fn circle(
        center: UiPoint,
        radius: f32,
        color: ColorRgba,
        stroke: Option<StrokeStyle>,
        opacity: f32,
    ) -> Option<Self> {
        if !radius.is_finite() || radius <= 0.0 {
            return None;
        }
        let rect = UiRect::new(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        );
        let mut instance = Self::rectangle(
            rect,
            color_as_vertex(color, opacity),
            CornerRadii::ZERO,
            stroke.map(|s| (s, StrokeAlignment::Center)),
            opacity,
        )?;
        instance.params[0] = 1.0;
        Some(instance)
    }

    pub fn segment(from: UiPoint, to: UiPoint, stroke: StrokeStyle, opacity: f32) -> Option<Self> {
        let width = finite_or(stroke.width, 0.0).max(0.0);
        if !stroke.is_visible() || width == 0.0 {
            return None;
        }
        let rect = UiRect::new(
            from.x.min(to.x) - width * 0.5,
            from.y.min(to.y) - width * 0.5,
            (to.x - from.x).abs() + width,
            (to.y - from.y).abs() + width,
        );
        if !valid_rect(rect) {
            return None;
        }
        let mut instance = Self::base(
            rect,
            color_as_vertex(stroke.color, opacity),
            CornerRadii::ZERO,
        );
        // Segment endpoints replace the box bounds for this shape kind.
        instance.shape_rect = [from.x, from.y, to.x, to.y];
        instance.params = [2.0, width, 0.0, 0.0];
        Some(instance)
    }

    pub fn shadow(
        rect: UiRect,
        radii: CornerRadii,
        effect: PaintEffect,
        opacity: f32,
    ) -> Option<Self> {
        if !valid_rect(rect) || effect.color.a == 0 || opacity <= 0.0 {
            return None;
        }
        let offset = UiPoint::new(
            finite_or(effect.offset.x, 0.0),
            finite_or(effect.offset.y, 0.0),
        );
        let spread = finite_or(effect.spread, 0.0);
        let blur = finite_or(effect.blur_radius, 0.0).max(0.0);
        let inset = effect.kind == PaintEffectKind::InsetShadow;
        let shape = if inset {
            rect
        } else {
            UiRect::new(
                rect.x + offset.x - spread,
                rect.y + offset.y - spread,
                rect.width + 2.0 * spread,
                rect.height + 2.0 * spread,
            )
        };
        if !valid_rect(shape) {
            return None;
        }
        let radii = if inset {
            radii
        } else {
            CornerRadii::new(
                (radii.top_left + spread).max(0.0),
                (radii.top_right + spread).max(0.0),
                (radii.bottom_right + spread).max(0.0),
                (radii.bottom_left + spread).max(0.0),
            )
        };
        let mut instance = Self::base(
            shape,
            color_as_vertex(effect.color, opacity),
            normalized_corner_radii_for_rect(radii, shape.width, shape.height),
        );
        instance.params = [if inset { 4.0 } else { 3.0 }, 0.0, 0.0, blur];
        instance.effect = [offset.x, offset.y, spread, 0.0];
        if !inset {
            instance.draw_rect = rect_array(expand(shape, blur + AA_OUTSET));
        }
        Some(instance)
    }

    fn base(rect: UiRect, color: [f32; 4], radii: CornerRadii) -> Self {
        Self {
            draw_rect: rect_array(expand(rect, AA_OUTSET)),
            shape_rect: rect_array(rect),
            color,
            border_color: [0.0; 4],
            radii: [
                radii.top_left,
                radii.top_right,
                radii.bottom_right,
                radii.bottom_left,
            ],
            params: [0.0; 4],
            gradient_line: [0.0; 4],
            effect: [0.0; 4],
            gradient_range: [0; 2],
        }
    }
}

/// Storage-buffer array entries are two vec4s (32 bytes, WGSL alignment 16).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct SdfGradientStop {
    pub position: [f32; 4],
    pub color: [f32; 4],
}

pub(super) fn instance_bytes(instances: &[SdfInstance]) -> &[u8] {
    // repr(C) with only contiguous f32/u32 arrays; no padding or invalid bit patterns.
    unsafe { std::slice::from_raw_parts(instances.as_ptr().cast(), mem::size_of_val(instances)) }
}

pub(super) fn gradient_bytes(stops: &[SdfGradientStop]) -> &[u8] {
    // The Rust offsets and array stride match the two vec4s in the shader.
    unsafe { std::slice::from_raw_parts(stops.as_ptr().cast(), mem::size_of_val(stops)) }
}

fn rect_array(rect: UiRect) -> [f32; 4] {
    [rect.x, rect.y, rect.width, rect.height]
}

fn expand(rect: UiRect, outset: f32) -> UiRect {
    UiRect::new(
        rect.x - outset,
        rect.y - outset,
        rect.width + outset * 2.0,
        rect.height + outset * 2.0,
    )
}

fn valid_rect(rect: UiRect) -> bool {
    [
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        rect.right(),
        rect.bottom(),
    ]
    .iter()
    .all(|v| v.is_finite())
        && rect.width > 0.0
        && rect.height > 0.0
}

pub(super) fn buffer_capacity(
    required: u64,
    limit: u64,
) -> Result<u64, crate::renderer::RenderError> {
    let limit = limit & !3;
    if required > limit || required == 0 {
        return Err(crate::renderer::RenderError::Backend(format!(
            "SDF buffer requires {required} bytes; device limit is {limit}"
        )));
    }
    Ok(required
        .checked_next_power_of_two()
        .unwrap_or(limit)
        .min(limit))
}
