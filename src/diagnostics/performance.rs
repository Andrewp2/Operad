//! Renderer-neutral performance diagnostics.
//!
//! These types turn low-level timing sections and cache reuse reports into the
//! named pipeline stages v7 needs for repeatable showcase and stress-probe
//! measurements.

use std::time::Duration;

use crate::{DisplayListReuseOutcome, DisplayListReuseReport, FrameTiming};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FramePipelineStage {
    TreeBuild,
    Diff,
    Layout,
    TextShaping,
    HitTesting,
    PaintList,
    Batching,
    Uploads,
    BackendDraw,
    Canvas,
    Accessibility,
    Custom(String),
}

impl FramePipelineStage {
    pub fn label(&self) -> &str {
        match self {
            Self::TreeBuild => "tree-build",
            Self::Diff => "diff",
            Self::Layout => "layout",
            Self::TextShaping => "text-shaping",
            Self::HitTesting => "hit-testing",
            Self::PaintList => "paint-list",
            Self::Batching => "batching",
            Self::Uploads => "uploads",
            Self::BackendDraw => "backend-draw",
            Self::Canvas => "canvas",
            Self::Accessibility => "accessibility",
            Self::Custom(label) => label.as_str(),
        }
    }

    pub fn from_label(label: impl Into<String>) -> Self {
        match label.into().as_str() {
            "tree-build" | "tree_rebuild" | "tree-rebuild" => Self::TreeBuild,
            "diff" => Self::Diff,
            "layout" => Self::Layout,
            "text-shaping" | "text_shaping" | "text" => Self::TextShaping,
            "hit-testing" | "hit_testing" | "hit-test" => Self::HitTesting,
            "paint-list" | "paint_list" | "paint" => Self::PaintList,
            "batching" | "batch" => Self::Batching,
            "uploads" | "upload" => Self::Uploads,
            "backend-draw" | "backend_draw" | "render" | "draw" | "gpu-render" => Self::BackendDraw,
            "canvas" => Self::Canvas,
            "accessibility" | "a11y" => Self::Accessibility,
            other => Self::Custom(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramePipelineSection {
    pub stage: FramePipelineStage,
    pub duration: Duration,
}

impl FramePipelineSection {
    pub const fn new(stage: FramePipelineStage, duration: Duration) -> Self {
        Self { stage, duration }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FramePipelineTiming {
    pub sections: Vec<FramePipelineSection>,
}

impl FramePipelineTiming {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stage(mut self, stage: FramePipelineStage, duration: Duration) -> Self {
        self.push(stage, duration);
        self
    }

    pub fn push(&mut self, stage: FramePipelineStage, duration: Duration) {
        self.sections
            .push(FramePipelineSection::new(stage, duration));
    }

    pub fn from_frame_timing(timing: &FrameTiming) -> Self {
        let mut pipeline = Self::new();
        for section in &timing.sections {
            pipeline.push(
                FramePipelineStage::from_label(section.name.clone()),
                section.duration,
            );
        }
        pipeline
    }

    pub fn total(&self) -> Duration {
        self.sections.iter().map(|section| section.duration).sum()
    }

    pub fn duration(&self, stage: &FramePipelineStage) -> Option<Duration> {
        self.sections
            .iter()
            .filter(|section| &section.stage == stage)
            .map(|section| section.duration)
            .reduce(|left, right| left + right)
    }

    pub fn missing_required_stages(&self) -> Vec<FramePipelineStage> {
        required_pipeline_stages()
            .into_iter()
            .filter(|stage| self.duration(stage).is_none())
            .collect()
    }

    pub fn slowest_stage(&self) -> Option<&FramePipelineSection> {
        self.sections.iter().max_by_key(|section| section.duration)
    }

    pub fn to_frame_timing(&self) -> FrameTiming {
        self.sections
            .iter()
            .fold(FrameTiming::new(), |timing, section| {
                timing.section(section.stage.label(), section.duration)
            })
    }
}

pub fn required_pipeline_stages() -> Vec<FramePipelineStage> {
    vec![
        FramePipelineStage::TreeBuild,
        FramePipelineStage::Diff,
        FramePipelineStage::Layout,
        FramePipelineStage::TextShaping,
        FramePipelineStage::HitTesting,
        FramePipelineStage::PaintList,
        FramePipelineStage::Batching,
        FramePipelineStage::Uploads,
        FramePipelineStage::BackendDraw,
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheDiagnosticKind {
    Layout,
    ShapedText,
    Image,
    CanvasTexture,
    DisplayList,
    Custom(String),
}

impl CacheDiagnosticKind {
    pub fn label(&self) -> &str {
        match self {
            Self::Layout => "layout",
            Self::ShapedText => "shaped-text",
            Self::Image => "image",
            Self::CanvasTexture => "canvas-texture",
            Self::DisplayList => "display-list",
            Self::Custom(label) => label.as_str(),
        }
    }

    pub fn from_label(label: impl Into<String>) -> Self {
        match label.into().as_str() {
            "layout" => Self::Layout,
            "text" | "text-shaping" | "shaped-text" | "glyph" | "glyphon" => Self::ShapedText,
            "image" | "images" | "texture" => Self::Image,
            "canvas" | "canvas-texture" | "canvas_texture" => Self::CanvasTexture,
            "display" | "display-list" | "display_list" | "retained-display-list" => {
                Self::DisplayList
            }
            other => Self::Custom(other.to_string()),
        }
    }
}

pub fn required_cache_diagnostic_kinds() -> Vec<CacheDiagnosticKind> {
    vec![
        CacheDiagnosticKind::Layout,
        CacheDiagnosticKind::ShapedText,
        CacheDiagnosticKind::Image,
        CacheDiagnosticKind::CanvasTexture,
        CacheDiagnosticKind::DisplayList,
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheDiagnostic {
    pub kind: CacheDiagnosticKind,
    pub name: String,
    pub lookups: usize,
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub retained_bytes: Option<usize>,
}

impl CacheDiagnostic {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            kind: CacheDiagnosticKind::from_label(name.clone()),
            name,
            lookups: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            retained_bytes: None,
        }
    }

    pub fn with_kind(mut self, kind: CacheDiagnosticKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn lookup(mut self, hit: bool) -> Self {
        self.lookups += 1;
        if hit {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        self
    }

    pub fn eviction(mut self) -> Self {
        self.evictions += 1;
        self
    }

    pub const fn retained_bytes(mut self, retained_bytes: usize) -> Self {
        self.retained_bytes = Some(retained_bytes);
        self
    }

    pub fn hit_rate(&self) -> Option<f32> {
        (self.lookups > 0).then(|| self.hits as f32 / self.lookups as f32)
    }

    pub fn from_display_list_reports<'a>(
        name: impl Into<String>,
        reports: impl IntoIterator<Item = &'a DisplayListReuseReport>,
    ) -> Self {
        let mut diagnostic = Self::new(name);
        for report in reports {
            diagnostic.lookups += 1;
            match report.outcome {
                DisplayListReuseOutcome::Reused => diagnostic.hits += 1,
                DisplayListReuseOutcome::MissEvicted => {
                    diagnostic.misses += 1;
                    diagnostic.evictions += 1;
                }
                DisplayListReuseOutcome::MissAbsent | DisplayListReuseOutcome::MissDirty => {
                    diagnostic.misses += 1;
                }
            }
        }
        diagnostic
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PerformanceSnapshot {
    pub frame: u64,
    pub pipeline: FramePipelineTiming,
    pub caches: Vec<CacheDiagnostic>,
}

impl PerformanceSnapshot {
    pub fn new(frame: u64) -> Self {
        Self {
            frame,
            ..Default::default()
        }
    }

    pub fn pipeline(mut self, pipeline: FramePipelineTiming) -> Self {
        self.pipeline = pipeline;
        self
    }

    pub fn cache(mut self, cache: CacheDiagnostic) -> Self {
        self.caches.push(cache);
        self
    }

    pub fn cache_hit_rate(&self, name: &str) -> Option<f32> {
        self.caches
            .iter()
            .find(|cache| cache.name == name)
            .and_then(CacheDiagnostic::hit_rate)
    }

    pub fn cache_by_kind(&self, kind: &CacheDiagnosticKind) -> Option<&CacheDiagnostic> {
        self.caches.iter().find(|cache| &cache.kind == kind)
    }

    pub fn missing_required_stages(&self) -> Vec<FramePipelineStage> {
        self.pipeline.missing_required_stages()
    }

    pub fn missing_required_cache_kinds(&self) -> Vec<CacheDiagnosticKind> {
        required_cache_diagnostic_kinds()
            .into_iter()
            .filter(|kind| self.cache_by_kind(kind).is_none())
            .collect()
    }

    pub fn has_required_diagnostics(&self) -> bool {
        self.missing_required_stages().is_empty() && self.missing_required_cache_kinds().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        DirtyFlags, DisplayListId, DisplayListKey, DisplayListKind, DisplayListScope, UiNodeId,
    };

    use super::*;

    #[test]
    fn frame_pipeline_maps_timing_sections_to_required_stages() {
        let timing = FrameTiming::new()
            .section("layout", Duration::from_micros(300))
            .section("paint", Duration::from_micros(200))
            .section("render", Duration::from_micros(500));
        let pipeline = FramePipelineTiming::from_frame_timing(&timing);

        assert_eq!(
            pipeline.duration(&FramePipelineStage::PaintList),
            Some(Duration::from_micros(200))
        );
        assert_eq!(
            pipeline
                .slowest_stage()
                .map(|section| section.stage.label()),
            Some("backend-draw")
        );
        assert_eq!(
            FramePipelineStage::from_label("gpu-render"),
            FramePipelineStage::BackendDraw
        );
        assert!(pipeline
            .missing_required_stages()
            .contains(&FramePipelineStage::TreeBuild));
    }

    #[test]
    fn cache_diagnostic_aggregates_display_list_reuse_reports() {
        let key = DisplayListKey::new(DisplayListScope::Node(UiNodeId(1)), "panel", 0);
        let reports = vec![
            DisplayListReuseReport {
                key: key.clone(),
                outcome: DisplayListReuseOutcome::Reused,
                dirty_flags: DirtyFlags::NONE,
                frame: 1,
                kind: Some(DisplayListKind::StaticPanel),
                invalidation: None,
                item_count: Some(4),
                created_frame: Some(0),
                last_used_frame: Some(1),
            },
            DisplayListReuseReport {
                key: DisplayListKey::new(DisplayListScope::Document, DisplayListId::new("old"), 0),
                outcome: DisplayListReuseOutcome::MissEvicted,
                dirty_flags: DirtyFlags::NONE,
                frame: 2,
                kind: None,
                invalidation: None,
                item_count: None,
                created_frame: None,
                last_used_frame: None,
            },
        ];

        let diagnostic = CacheDiagnostic::from_display_list_reports("display-list", &reports);

        assert_eq!(diagnostic.lookups, 2);
        assert_eq!(diagnostic.hits, 1);
        assert_eq!(diagnostic.misses, 1);
        assert_eq!(diagnostic.evictions, 1);
        assert_eq!(diagnostic.hit_rate(), Some(0.5));
        assert_eq!(diagnostic.kind, CacheDiagnosticKind::DisplayList);
    }

    #[test]
    fn performance_snapshot_reports_required_cache_coverage() {
        let pipeline = required_pipeline_stages()
            .into_iter()
            .fold(FramePipelineTiming::new(), |pipeline, stage| {
                pipeline.stage(stage, Duration::from_micros(1))
            });
        let complete = PerformanceSnapshot::new(4)
            .pipeline(pipeline)
            .cache(CacheDiagnostic::new("layout").lookup(true))
            .cache(CacheDiagnostic::new("shaped-text").lookup(true))
            .cache(CacheDiagnostic::new("image").lookup(false))
            .cache(CacheDiagnostic::new("canvas-texture").lookup(true))
            .cache(CacheDiagnostic::new("display-list").lookup(true));

        assert!(complete.has_required_diagnostics());
        assert_eq!(
            complete
                .cache_by_kind(&CacheDiagnosticKind::CanvasTexture)
                .unwrap()
                .name,
            "canvas-texture"
        );

        let incomplete = PerformanceSnapshot::new(5).cache(CacheDiagnostic::new("display-list"));
        assert_eq!(
            incomplete.missing_required_cache_kinds(),
            vec![
                CacheDiagnosticKind::Layout,
                CacheDiagnosticKind::ShapedText,
                CacheDiagnosticKind::Image,
                CacheDiagnosticKind::CanvasTexture,
            ]
        );
    }
}
