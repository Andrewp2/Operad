//! Backend-neutral font lifecycle and cache accounting.
//!
//! This module tracks font faces without owning backend font objects. Backends
//! can use it to resolve fallback stacks, reject stale load completions, report
//! missing or failed faces, account for loaded bytes, and plan LRU evictions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::{FontStretch, FontStyle, FontWeight};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontFamilyId(String);

impl FontFamilyId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FontFamilyId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for FontFamilyId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontFaceId {
    pub family: FontFamilyId,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub stretch: FontStretch,
}

impl FontFaceId {
    pub fn new(
        family: impl Into<FontFamilyId>,
        weight: FontWeight,
        style: FontStyle,
        stretch: FontStretch,
    ) -> Self {
        Self {
            family: family.into(),
            weight,
            style,
            stretch,
        }
    }

    pub fn regular(family: impl Into<FontFamilyId>) -> Self {
        Self::new(
            family,
            FontWeight::NORMAL,
            FontStyle::Normal,
            FontStretch::Normal,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFallbackStack {
    pub families: Vec<FontFamilyId>,
}

impl FontFallbackStack {
    pub fn new(families: impl IntoIterator<Item = impl Into<FontFamilyId>>) -> Self {
        Self {
            families: families.into_iter().map(Into::into).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.families.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontSourceDescriptor {
    BuiltIn { key: String },
    System { family_name: String },
    File { path: String },
    Memory { key: String },
}

impl FontSourceDescriptor {
    pub fn built_in(key: impl Into<String>) -> Self {
        Self::BuiltIn { key: key.into() }
    }

    pub fn system(family_name: impl Into<String>) -> Self {
        Self::System {
            family_name: family_name.into(),
        }
    }

    pub fn file(path: impl Into<String>) -> Self {
        Self::File { path: path.into() }
    }

    pub fn memory(key: impl Into<String>) -> Self {
        Self::Memory { key: key.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontBytes {
    key: String,
    bytes: Arc<[u8]>,
}

impl FontBytes {
    pub fn new(key: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            key: key.into(),
            bytes: Arc::from(bytes.into().into_boxed_slice()),
        }
    }

    pub fn from_static(key: impl Into<String>, bytes: &'static [u8]) -> Self {
        Self {
            key: key.into(),
            bytes: Arc::from(bytes),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontLibrary {
    fonts: Vec<FontBytes>,
    sans_serif_family: Option<String>,
    serif_family: Option<String>,
    monospace_family: Option<String>,
}

impl FontLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_font(mut self, font: FontBytes) -> Self {
        self.add_font(font);
        self
    }

    pub fn with_memory_font(mut self, key: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.add_memory_font(key, bytes);
        self
    }

    pub fn add_font(&mut self, font: FontBytes) -> &mut Self {
        self.fonts.push(font);
        self
    }

    pub fn add_memory_font(
        &mut self,
        key: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> &mut Self {
        self.add_font(FontBytes::new(key, bytes))
    }

    pub fn fonts(&self) -> &[FontBytes] {
        &self.fonts
    }

    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
            && self.sans_serif_family.is_none()
            && self.serif_family.is_none()
            && self.monospace_family.is_none()
    }

    pub fn with_sans_serif_family(mut self, family: impl Into<String>) -> Self {
        self.set_sans_serif_family(family);
        self
    }

    pub fn with_serif_family(mut self, family: impl Into<String>) -> Self {
        self.set_serif_family(family);
        self
    }

    pub fn with_monospace_family(mut self, family: impl Into<String>) -> Self {
        self.set_monospace_family(family);
        self
    }

    pub fn set_sans_serif_family(&mut self, family: impl Into<String>) -> &mut Self {
        self.sans_serif_family = Some(family.into());
        self
    }

    pub fn set_serif_family(&mut self, family: impl Into<String>) -> &mut Self {
        self.serif_family = Some(family.into());
        self
    }

    pub fn set_monospace_family(&mut self, family: impl Into<String>) -> &mut Self {
        self.monospace_family = Some(family.into());
        self
    }

    pub fn sans_serif_family(&self) -> Option<&str> {
        self.sans_serif_family.as_deref()
    }

    pub fn serif_family(&self) -> Option<&str> {
        self.serif_family.as_deref()
    }

    pub fn monospace_family(&self) -> Option<&str> {
        self.monospace_family.as_deref()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontGeneration(pub(crate) u64);

impl FontGeneration {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub const fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFaceDescriptor {
    pub id: FontFaceId,
    pub source: FontSourceDescriptor,
    pub generation: FontGeneration,
}

impl FontFaceDescriptor {
    pub fn new(id: FontFaceId, source: FontSourceDescriptor, generation: FontGeneration) -> Self {
        Self {
            id,
            source,
            generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontLoadStatus {
    Pending,
    Loaded,
    Missing,
    Failed { reason: String },
}

impl FontLoadStatus {
    pub const fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded)
    }

    pub const fn is_terminal_failure(&self) -> bool {
        matches!(self, Self::Missing | Self::Failed { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontLifecycleOutcome {
    Registered,
    Loaded,
    Missing,
    Failed,
    Retained,
    Stale,
    Evicted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontLifecycleIssue {
    MissingFace,
    InvalidLoadedByteLength,
    StaleGeneration {
        current_generation: FontGeneration,
        update_generation: FontGeneration,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedFontFace {
    pub descriptor: FontFaceDescriptor,
    pub status: FontLoadStatus,
    pub bytes: usize,
    pub created_frame: u64,
    pub last_used_frame: u64,
    pub last_update_frame: u64,
}

impl CachedFontFace {
    fn new(descriptor: FontFaceDescriptor, frame: u64) -> Self {
        Self {
            descriptor,
            status: FontLoadStatus::Pending,
            bytes: 0,
            created_frame: frame,
            last_used_frame: frame,
            last_update_frame: frame,
        }
    }

    fn apply_status(&mut self, status: FontLoadStatus, bytes: usize, frame: u64) -> usize {
        let previous = self.bytes;
        self.status = status;
        self.bytes = bytes;
        self.last_used_frame = frame;
        self.last_update_frame = frame;
        previous
    }

    fn retain(&mut self, frame: u64) {
        self.last_used_frame = frame;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontLifecycleReport {
    pub face_id: FontFaceId,
    pub outcome: FontLifecycleOutcome,
    pub issue: Option<FontLifecycleIssue>,
    pub generation: FontGeneration,
    pub current_generation: Option<FontGeneration>,
    pub frame: u64,
    pub bytes_before: usize,
    pub bytes_after: usize,
    pub total_bytes_before: usize,
    pub total_bytes_after: usize,
    pub created_frame: Option<u64>,
    pub last_used_frame: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFallbackAttempt {
    pub face_id: FontFaceId,
    pub status: Option<FontLoadStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFallbackResolution {
    pub resolved: Option<FontFaceId>,
    pub attempts: Vec<FontFallbackAttempt>,
    pub frame: u64,
}

impl FontFallbackResolution {
    pub fn missing_faces(&self) -> Vec<&FontFaceId> {
        self.attempts
            .iter()
            .filter(|attempt| matches!(attempt.status, Some(FontLoadStatus::Missing) | None))
            .map(|attempt| &attempt.face_id)
            .collect()
    }

    pub fn failed_faces(&self) -> Vec<&FontFaceId> {
        self.attempts
            .iter()
            .filter(|attempt| matches!(attempt.status, Some(FontLoadStatus::Failed { .. })))
            .map(|attempt| &attempt.face_id)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FontCachePolicy {
    pub max_bytes: Option<usize>,
}

impl FontCachePolicy {
    pub const UNBOUNDED: Self = Self { max_bytes: None };

    pub const fn max_bytes(max_bytes: usize) -> Self {
        Self {
            max_bytes: Some(max_bytes),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontEvictionCandidate {
    pub face_id: FontFaceId,
    pub bytes: usize,
    pub created_frame: u64,
    pub last_used_frame: u64,
    pub age_frames: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontEvictionPlan {
    pub policy: FontCachePolicy,
    pub frame: u64,
    pub total_bytes_before: usize,
    pub bytes_to_free: usize,
    pub total_bytes_after: usize,
    pub candidates: Vec<FontEvictionCandidate>,
}

impl FontEvictionPlan {
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    pub fn evicted_count(&self) -> usize {
        self.candidates.len()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontRegistry {
    faces: HashMap<FontFaceId, CachedFontFace>,
    generation: FontGeneration,
    frame: u64,
    total_bytes: usize,
}

impl FontRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn generation(&self) -> FontGeneration {
        self.generation
    }

    pub fn advance_generation(&mut self) -> FontGeneration {
        self.generation = self.generation.next();
        self.generation
    }

    pub const fn frame(&self) -> u64 {
        self.frame
    }

    pub fn advance_frame(&mut self) -> u64 {
        self.frame = self.frame.wrapping_add(1);
        self.frame
    }

    pub const fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn len(&self) -> usize {
        self.faces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    pub fn face(&self, face_id: &FontFaceId) -> Option<&CachedFontFace> {
        self.faces.get(face_id)
    }

    pub fn register_face(&mut self, descriptor: FontFaceDescriptor) -> FontLifecycleReport {
        let before = self.total_bytes;
        self.generation = self.generation.max(descriptor.generation);
        let face_id = descriptor.id.clone();

        if let Some(face) = self.faces.get_mut(&face_id) {
            if descriptor.generation < face.descriptor.generation {
                return stale_report(
                    face_id,
                    descriptor.generation,
                    face.descriptor.generation,
                    self.frame,
                    before,
                    self.total_bytes,
                );
            }
            face.descriptor = descriptor;
            face.status = FontLoadStatus::Pending;
            let previous = face.bytes;
            face.bytes = 0;
            face.last_update_frame = self.frame;
            face.last_used_frame = self.frame;
            self.total_bytes = self.total_bytes.saturating_sub(previous);
            return report_from_face(
                face,
                FontLifecycleOutcome::Registered,
                None,
                self.frame,
                before,
                self.total_bytes,
            );
        }

        let face = CachedFontFace::new(descriptor, self.frame);
        let report = report_from_face(
            &face,
            FontLifecycleOutcome::Registered,
            None,
            self.frame,
            before,
            self.total_bytes,
        );
        self.faces.insert(face_id, face);
        report
    }

    pub fn mark_loaded(
        &mut self,
        face_id: &FontFaceId,
        generation: FontGeneration,
        bytes: usize,
    ) -> FontLifecycleReport {
        if bytes == 0 {
            return self.apply_status(
                face_id,
                generation,
                FontLoadStatus::Failed {
                    reason: "loaded font face reported zero bytes".to_string(),
                },
                0,
                Some(FontLifecycleIssue::InvalidLoadedByteLength),
            );
        }
        self.apply_status(face_id, generation, FontLoadStatus::Loaded, bytes, None)
    }

    pub fn mark_missing(
        &mut self,
        face_id: &FontFaceId,
        generation: FontGeneration,
    ) -> FontLifecycleReport {
        self.apply_status(face_id, generation, FontLoadStatus::Missing, 0, None)
    }

    pub fn mark_failed(
        &mut self,
        face_id: &FontFaceId,
        generation: FontGeneration,
        reason: impl Into<String>,
    ) -> FontLifecycleReport {
        self.apply_status(
            face_id,
            generation,
            FontLoadStatus::Failed {
                reason: reason.into(),
            },
            0,
            None,
        )
    }

    pub fn retain(&mut self, face_id: &FontFaceId) -> FontLifecycleReport {
        let before = self.total_bytes;
        match self.faces.get_mut(face_id) {
            Some(face) => {
                face.retain(self.frame);
                report_from_face(
                    face,
                    FontLifecycleOutcome::Retained,
                    None,
                    self.frame,
                    before,
                    self.total_bytes,
                )
            }
            None => FontLifecycleReport {
                face_id: face_id.clone(),
                outcome: FontLifecycleOutcome::Failed,
                issue: Some(FontLifecycleIssue::MissingFace),
                generation: self.generation,
                current_generation: None,
                frame: self.frame,
                bytes_before: 0,
                bytes_after: 0,
                total_bytes_before: before,
                total_bytes_after: self.total_bytes,
                created_frame: None,
                last_used_frame: None,
            },
        }
    }

    pub fn resolve_fallback_stack(
        &mut self,
        stack: &FontFallbackStack,
        weight: FontWeight,
        style: FontStyle,
        stretch: FontStretch,
    ) -> FontFallbackResolution {
        let mut attempts = Vec::new();
        let mut resolved = None;

        for family in &stack.families {
            let face_id = FontFaceId::new(family.clone(), weight, style, stretch);
            let status = self.faces.get(&face_id).map(|face| face.status.clone());
            attempts.push(FontFallbackAttempt {
                face_id: face_id.clone(),
                status: status.clone(),
            });
            if matches!(status, Some(FontLoadStatus::Loaded)) {
                self.retain(&face_id);
                resolved = Some(face_id);
                break;
            }
        }

        FontFallbackResolution {
            resolved,
            attempts,
            frame: self.frame,
        }
    }

    pub fn missing_faces(&self) -> Vec<&CachedFontFace> {
        let mut faces = self
            .faces
            .values()
            .filter(|face| matches!(face.status, FontLoadStatus::Missing))
            .collect::<Vec<_>>();
        faces.sort_by(|left, right| {
            face_id_order(&left.descriptor.id).cmp(&face_id_order(&right.descriptor.id))
        });
        faces
    }

    pub fn failed_faces(&self) -> Vec<&CachedFontFace> {
        let mut faces = self
            .faces
            .values()
            .filter(|face| matches!(face.status, FontLoadStatus::Failed { .. }))
            .collect::<Vec<_>>();
        faces.sort_by(|left, right| {
            face_id_order(&left.descriptor.id).cmp(&face_id_order(&right.descriptor.id))
        });
        faces
    }

    pub fn plan_eviction_by_byte_budget(&self, max_bytes: usize) -> FontEvictionPlan {
        self.plan_eviction(FontCachePolicy::max_bytes(max_bytes))
    }

    pub fn plan_eviction(&self, policy: FontCachePolicy) -> FontEvictionPlan {
        let mut candidates = Vec::new();
        if let Some(max_bytes) = policy.max_bytes {
            let mut projected_bytes = self.total_bytes;
            if projected_bytes > max_bytes {
                for face in sorted_loaded_faces(&self.faces) {
                    projected_bytes = projected_bytes.saturating_sub(face.bytes);
                    candidates.push(FontEvictionCandidate {
                        face_id: face.descriptor.id.clone(),
                        bytes: face.bytes,
                        created_frame: face.created_frame,
                        last_used_frame: face.last_used_frame,
                        age_frames: self.frame.saturating_sub(face.last_used_frame),
                    });
                    if projected_bytes <= max_bytes {
                        break;
                    }
                }
            }
        }

        let bytes_to_free = candidates
            .iter()
            .map(|candidate| candidate.bytes)
            .sum::<usize>();
        FontEvictionPlan {
            policy,
            frame: self.frame,
            total_bytes_before: self.total_bytes,
            bytes_to_free,
            total_bytes_after: self.total_bytes.saturating_sub(bytes_to_free),
            candidates,
        }
    }

    pub fn evict_planned(&mut self, plan: &FontEvictionPlan) -> Vec<FontLifecycleReport> {
        let mut reports = Vec::new();
        for candidate in &plan.candidates {
            let before = self.total_bytes;
            let Some(mut face) = self.faces.remove(&candidate.face_id) else {
                continue;
            };
            let previous = face.apply_status(FontLoadStatus::Pending, 0, self.frame);
            self.total_bytes = self.total_bytes.saturating_sub(previous);
            reports.push(report_from_face(
                &face,
                FontLifecycleOutcome::Evicted,
                None,
                self.frame,
                before,
                self.total_bytes,
            ));
        }
        reports
    }

    fn apply_status(
        &mut self,
        face_id: &FontFaceId,
        generation: FontGeneration,
        status: FontLoadStatus,
        bytes: usize,
        issue: Option<FontLifecycleIssue>,
    ) -> FontLifecycleReport {
        let before = self.total_bytes;
        let Some(face) = self.faces.get_mut(face_id) else {
            return FontLifecycleReport {
                face_id: face_id.clone(),
                outcome: FontLifecycleOutcome::Failed,
                issue: Some(FontLifecycleIssue::MissingFace),
                generation,
                current_generation: None,
                frame: self.frame,
                bytes_before: 0,
                bytes_after: 0,
                total_bytes_before: before,
                total_bytes_after: self.total_bytes,
                created_frame: None,
                last_used_frame: None,
            };
        };

        if generation < face.descriptor.generation {
            return stale_report(
                face_id.clone(),
                generation,
                face.descriptor.generation,
                self.frame,
                before,
                self.total_bytes,
            );
        }

        face.descriptor.generation = generation;
        self.generation = self.generation.max(generation);
        let previous = face.apply_status(status, bytes, self.frame);
        self.total_bytes = self
            .total_bytes
            .saturating_sub(previous)
            .saturating_add(bytes);
        let outcome = match &face.status {
            FontLoadStatus::Pending => FontLifecycleOutcome::Registered,
            FontLoadStatus::Loaded => FontLifecycleOutcome::Loaded,
            FontLoadStatus::Missing => FontLifecycleOutcome::Missing,
            FontLoadStatus::Failed { .. } => FontLifecycleOutcome::Failed,
        };
        report_from_face(face, outcome, issue, self.frame, before, self.total_bytes)
    }
}

fn stale_report(
    face_id: FontFaceId,
    generation: FontGeneration,
    current_generation: FontGeneration,
    frame: u64,
    total_bytes_before: usize,
    total_bytes_after: usize,
) -> FontLifecycleReport {
    FontLifecycleReport {
        face_id,
        outcome: FontLifecycleOutcome::Stale,
        issue: Some(FontLifecycleIssue::StaleGeneration {
            current_generation,
            update_generation: generation,
        }),
        generation,
        current_generation: Some(current_generation),
        frame,
        bytes_before: 0,
        bytes_after: 0,
        total_bytes_before,
        total_bytes_after,
        created_frame: None,
        last_used_frame: None,
    }
}

fn report_from_face(
    face: &CachedFontFace,
    outcome: FontLifecycleOutcome,
    issue: Option<FontLifecycleIssue>,
    frame: u64,
    total_bytes_before: usize,
    total_bytes_after: usize,
) -> FontLifecycleReport {
    FontLifecycleReport {
        face_id: face.descriptor.id.clone(),
        outcome,
        issue,
        generation: face.descriptor.generation,
        current_generation: Some(face.descriptor.generation),
        frame,
        bytes_before: face.bytes,
        bytes_after: face.bytes,
        total_bytes_before,
        total_bytes_after,
        created_frame: Some(face.created_frame),
        last_used_frame: Some(face.last_used_frame),
    }
}

fn sorted_loaded_faces(faces: &HashMap<FontFaceId, CachedFontFace>) -> Vec<&CachedFontFace> {
    let mut faces = faces
        .values()
        .filter(|face| face.status.is_loaded() && face.bytes > 0)
        .collect::<Vec<_>>();
    faces.sort_by(|left, right| {
        (left.last_used_frame, left.created_frame)
            .cmp(&(right.last_used_frame, right.created_frame))
            .then_with(|| {
                face_id_order(&left.descriptor.id).cmp(&face_id_order(&right.descriptor.id))
            })
    });
    faces
}

fn face_id_order(face_id: &FontFaceId) -> (&str, u16, u8, u8) {
    (
        face_id.family.as_str(),
        face_id.weight.value(),
        font_style_order(face_id.style),
        font_stretch_order(face_id.stretch),
    )
}

fn font_style_order(style: FontStyle) -> u8 {
    match style {
        FontStyle::Normal => 0,
        FontStyle::Italic => 1,
        FontStyle::Oblique => 2,
    }
}

fn font_stretch_order(stretch: FontStretch) -> u8 {
    match stretch {
        FontStretch::Condensed => 0,
        FontStretch::Normal => 1,
        FontStretch::Expanded => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(family: &str) -> FontFaceId {
        FontFaceId::regular(family)
    }

    fn descriptor(family: &str, generation: u64) -> FontFaceDescriptor {
        FontFaceDescriptor::new(
            face(family),
            FontSourceDescriptor::memory(format!("{family}-regular")),
            FontGeneration::new(generation),
        )
    }

    fn register_loaded(
        registry: &mut FontRegistry,
        family: &str,
        generation: u64,
        bytes: usize,
    ) -> FontFaceId {
        let id = face(family);
        registry.register_face(descriptor(family, generation));
        registry.mark_loaded(&id, FontGeneration::new(generation), bytes);
        id
    }

    #[test]
    fn fallback_stack_resolves_first_loaded_face_and_records_attempts() {
        let mut registry = FontRegistry::new();
        let missing = face("Missing Sans");
        let loaded = register_loaded(&mut registry, "App Sans", 1, 128);
        registry.register_face(descriptor("Missing Sans", 1));
        registry.mark_missing(&missing, FontGeneration::new(1));

        let resolution = registry.resolve_fallback_stack(
            &FontFallbackStack::new(["Missing Sans", "App Sans", "System Sans"]),
            FontWeight::NORMAL,
            FontStyle::Normal,
            FontStretch::Normal,
        );

        assert_eq!(resolution.resolved, Some(loaded));
        assert_eq!(resolution.attempts.len(), 2);
        assert_eq!(resolution.missing_faces(), vec![&missing]);
    }

    #[test]
    fn missing_and_failed_faces_are_reported() {
        let mut registry = FontRegistry::new();
        let missing = face("Missing Sans");
        let failed = face("Broken Serif");
        registry.register_face(descriptor("Missing Sans", 1));
        registry.register_face(descriptor("Broken Serif", 1));

        let missing_report = registry.mark_missing(&missing, FontGeneration::new(1));
        let failed_report = registry.mark_failed(&failed, FontGeneration::new(1), "parse error");

        assert_eq!(missing_report.outcome, FontLifecycleOutcome::Missing);
        assert_eq!(failed_report.outcome, FontLifecycleOutcome::Failed);
        assert_eq!(
            registry
                .missing_faces()
                .iter()
                .map(|face| &face.descriptor.id)
                .collect::<Vec<_>>(),
            vec![&missing]
        );
        assert_eq!(
            registry
                .failed_faces()
                .iter()
                .map(|face| &face.descriptor.id)
                .collect::<Vec<_>>(),
            vec![&failed]
        );
    }

    #[test]
    fn stale_generation_load_results_are_rejected() {
        let mut registry = FontRegistry::new();
        let id = face("App Sans");
        registry.register_face(descriptor("App Sans", 2));

        let report = registry.mark_loaded(&id, FontGeneration::new(1), 128);

        assert_eq!(report.outcome, FontLifecycleOutcome::Stale);
        assert_eq!(
            report.issue,
            Some(FontLifecycleIssue::StaleGeneration {
                current_generation: FontGeneration::new(2),
                update_generation: FontGeneration::new(1),
            })
        );
        assert_eq!(registry.total_bytes(), 0);
        assert_eq!(
            registry.face(&id).map(|face| &face.status),
            Some(&FontLoadStatus::Pending)
        );
    }

    #[test]
    fn byte_accounting_tracks_loaded_missing_failed_and_replaced_faces() {
        let mut registry = FontRegistry::new();
        let loaded = face("App Sans");
        let missing = face("Missing Sans");
        let failed = face("Broken Serif");
        registry.register_face(descriptor("App Sans", 1));
        registry.register_face(descriptor("Missing Sans", 1));
        registry.register_face(descriptor("Broken Serif", 1));

        registry.mark_loaded(&loaded, FontGeneration::new(1), 200);
        registry.mark_missing(&missing, FontGeneration::new(1));
        registry.mark_failed(&failed, FontGeneration::new(1), "parse error");
        assert_eq!(registry.total_bytes(), 200);

        registry.mark_loaded(&loaded, FontGeneration::new(2), 80);

        assert_eq!(registry.total_bytes(), 80);
        assert_eq!(registry.face(&loaded).map(|face| face.bytes), Some(80));
        assert_eq!(registry.face(&missing).map(|face| face.bytes), Some(0));
        assert_eq!(registry.face(&failed).map(|face| face.bytes), Some(0));
    }

    #[test]
    fn fallback_resolution_updates_last_used_frame() {
        let mut registry = FontRegistry::new();
        let id = register_loaded(&mut registry, "App Sans", 1, 128);
        registry.advance_frame();
        registry.advance_frame();

        let resolution = registry.resolve_fallback_stack(
            &FontFallbackStack::new(["App Sans"]),
            FontWeight::NORMAL,
            FontStyle::Normal,
            FontStretch::Normal,
        );

        assert_eq!(resolution.resolved, Some(id.clone()));
        assert_eq!(registry.face(&id).map(|face| face.last_used_frame), Some(2));
    }

    #[test]
    fn eviction_planning_selects_least_recently_used_loaded_faces_under_budget() {
        let mut registry = FontRegistry::new();
        let old = register_loaded(&mut registry, "Old Sans", 1, 100);
        registry.advance_frame();
        let middle = register_loaded(&mut registry, "Middle Sans", 1, 90);
        registry.advance_frame();
        let recent = register_loaded(&mut registry, "Recent Sans", 1, 80);
        registry.advance_frame();
        registry.retain(&middle);

        let plan = registry.plan_eviction_by_byte_budget(120);

        assert_eq!(plan.total_bytes_before, 270);
        assert_eq!(plan.bytes_to_free, 180);
        assert_eq!(plan.total_bytes_after, 90);
        assert_eq!(
            plan.candidates
                .iter()
                .map(|candidate| &candidate.face_id)
                .collect::<Vec<_>>(),
            vec![&old, &recent]
        );
        assert_eq!(
            registry.face(&middle).map(|face| face.last_used_frame),
            Some(3)
        );
    }
}
