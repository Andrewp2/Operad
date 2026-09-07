//! Retained display-list cache contracts for static and semi-static surfaces.
//!
//! This module is intentionally renderer-neutral. It lets applications and
//! backends retain expensive paint lists for editor backgrounds, ruler grids,
//! static panels, and snapshots, while invalidating them through explicit dirty
//! flags rather than backend-specific cache state.

use std::collections::HashMap;

use crate::core::invalidation::DirtyFlags;
use crate::{PaintList, UiNodeId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DisplayListId(String);

impl DisplayListId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for DisplayListId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for DisplayListId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DisplayListScope {
    Document,
    Node(UiNodeId),
    EditorSurface(String),
    Custom(String),
}

impl DisplayListScope {
    pub fn editor_surface(id: impl Into<String>) -> Self {
        Self::EditorSurface(id.into())
    }

    pub fn custom(id: impl Into<String>) -> Self {
        Self::Custom(id.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayListKey {
    pub scope: DisplayListScope,
    pub id: DisplayListId,
    pub revision: u64,
}

impl DisplayListKey {
    pub fn new(scope: DisplayListScope, id: impl Into<DisplayListId>, revision: u64) -> Self {
        Self {
            scope,
            id: id.into(),
            revision,
        }
    }

    pub fn editor_background(surface: impl Into<String>, revision: u64) -> Self {
        Self::new(
            DisplayListScope::EditorSurface(surface.into()),
            DisplayListId::new("background"),
            revision,
        )
    }

    pub fn node(node: UiNodeId, id: impl Into<DisplayListId>, revision: u64) -> Self {
        Self::new(DisplayListScope::Node(node), id, revision)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayListKind {
    StaticBackground,
    StaticPanel,
    DynamicOverlay,
    Snapshot,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayListInvalidation {
    pub dirty_flags: DirtyFlags,
}

impl DisplayListInvalidation {
    pub const NONE: Self = Self {
        dirty_flags: DirtyFlags::NONE,
    };

    pub const ANY: Self = Self {
        dirty_flags: DirtyFlags::ALL,
    };

    pub const STATIC_EDITOR_BACKGROUND: Self = Self {
        dirty_flags: DirtyFlags {
            layout: true,
            paint: true,
            input: false,
            theme: true,
            text_measurement: true,
        },
    };

    pub const STATIC_PANEL: Self = Self {
        dirty_flags: DirtyFlags {
            layout: true,
            paint: true,
            input: false,
            theme: true,
            text_measurement: true,
        },
    };

    pub const INPUT_OVERLAY: Self = Self {
        dirty_flags: DirtyFlags {
            layout: false,
            paint: true,
            input: true,
            theme: true,
            text_measurement: false,
        },
    };

    pub const fn new(dirty_flags: DirtyFlags) -> Self {
        Self { dirty_flags }
    }

    pub const fn invalidated_by(self, dirty: DirtyFlags) -> bool {
        dirty_flags_intersect(self.dirty_flags, dirty)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetainedDisplayList {
    pub key: DisplayListKey,
    pub kind: DisplayListKind,
    pub invalidation: DisplayListInvalidation,
    pub paint: PaintList,
    pub item_count: usize,
    pub created_frame: u64,
    pub last_used_frame: u64,
}

impl RetainedDisplayList {
    pub fn new(
        key: DisplayListKey,
        kind: DisplayListKind,
        invalidation: DisplayListInvalidation,
        paint: PaintList,
        frame: u64,
    ) -> Self {
        let item_count = paint.items.len();
        Self {
            key,
            kind,
            invalidation,
            paint,
            item_count,
            created_frame: frame,
            last_used_frame: frame,
        }
    }

    pub fn reusable_for(&self, dirty: DirtyFlags) -> bool {
        !self.invalidation.invalidated_by(dirty)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayListInvalidationRequest {
    All,
    Scope(DisplayListScope),
    Id(DisplayListId),
    Dirty(DirtyFlags),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayListReuseOutcome {
    Reused,
    MissAbsent,
    MissDirty,
    MissEvicted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayListReuseReport {
    pub key: DisplayListKey,
    pub outcome: DisplayListReuseOutcome,
    pub dirty_flags: DirtyFlags,
    pub frame: u64,
    pub kind: Option<DisplayListKind>,
    pub invalidation: Option<DisplayListInvalidation>,
    pub item_count: Option<usize>,
    pub created_frame: Option<u64>,
    pub last_used_frame: Option<u64>,
}

impl DisplayListReuseReport {
    fn from_entry(
        entry: &RetainedDisplayList,
        outcome: DisplayListReuseOutcome,
        dirty_flags: DirtyFlags,
        frame: u64,
    ) -> Self {
        Self {
            key: entry.key.clone(),
            outcome,
            dirty_flags,
            frame,
            kind: Some(entry.kind),
            invalidation: Some(entry.invalidation),
            item_count: Some(entry.item_count),
            created_frame: Some(entry.created_frame),
            last_used_frame: Some(entry.last_used_frame),
        }
    }

    fn missing(
        key: DisplayListKey,
        outcome: DisplayListReuseOutcome,
        dirty_flags: DirtyFlags,
        frame: u64,
    ) -> Self {
        Self {
            key,
            outcome,
            dirty_flags,
            frame,
            kind: None,
            invalidation: None,
            item_count: None,
            created_frame: None,
            last_used_frame: None,
        }
    }

    pub const fn reused(&self) -> bool {
        matches!(self.outcome, DisplayListReuseOutcome::Reused)
    }

    pub const fn missed(&self) -> bool {
        !self.reused()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayListInvalidationReport {
    pub request: DisplayListInvalidationRequest,
    pub removed_keys: Vec<DisplayListKey>,
    pub before_len: usize,
    pub after_len: usize,
    pub frame: u64,
}

impl DisplayListInvalidationReport {
    pub fn removed_count(&self) -> usize {
        self.removed_keys.len()
    }

    pub fn removed_key(&self, key: &DisplayListKey) -> bool {
        self.removed_keys.iter().any(|removed| removed == key)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RetainedDisplayListCache {
    entries: HashMap<DisplayListKey, RetainedDisplayList>,
    evicted_keys: Vec<DisplayListKey>,
    max_entries: Option<usize>,
    frame: u64,
}

const MAX_EVICTION_HISTORY: usize = 64;

impl RetainedDisplayListCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity_limit(max_entries: usize) -> Self {
        Self {
            max_entries: Some(max_entries.max(1)),
            ..Self::default()
        }
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn advance_frame(&mut self) -> u64 {
        self.frame = self.frame.wrapping_add(1);
        self.frame
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains_key(&self, key: &DisplayListKey) -> bool {
        self.entries.contains_key(key)
    }

    pub fn was_evicted(&self, key: &DisplayListKey) -> bool {
        self.evicted_keys.iter().any(|evicted| evicted == key)
    }

    pub fn insert(
        &mut self,
        key: DisplayListKey,
        kind: DisplayListKind,
        invalidation: DisplayListInvalidation,
        paint: PaintList,
    ) {
        let entry = RetainedDisplayList::new(key.clone(), kind, invalidation, paint, self.frame);
        self.evicted_keys.retain(|evicted| evicted != &key);
        self.entries.insert(key, entry);
        self.evict_to_limit();
    }

    pub fn insert_static_editor_background(
        &mut self,
        surface: impl Into<String>,
        revision: u64,
        paint: PaintList,
    ) -> DisplayListKey {
        let key = DisplayListKey::editor_background(surface, revision);
        self.insert(
            key.clone(),
            DisplayListKind::StaticBackground,
            DisplayListInvalidation::STATIC_EDITOR_BACKGROUND,
            paint,
        );
        key
    }

    pub fn entry(&self, key: &DisplayListKey) -> Option<&RetainedDisplayList> {
        self.entries.get(key)
    }

    pub fn get_reusable(&mut self, key: &DisplayListKey, dirty: DirtyFlags) -> Option<&PaintList> {
        let entry = self.entries.get_mut(key)?;
        if !entry.reusable_for(dirty) {
            return None;
        }
        entry.last_used_frame = self.frame;
        Some(&entry.paint)
    }

    pub fn reuse_report(
        &mut self,
        key: &DisplayListKey,
        dirty: DirtyFlags,
    ) -> DisplayListReuseReport {
        if let Some(entry) = self.entries.get_mut(key) {
            if entry.reusable_for(dirty) {
                entry.last_used_frame = self.frame;
                return DisplayListReuseReport::from_entry(
                    entry,
                    DisplayListReuseOutcome::Reused,
                    dirty,
                    self.frame,
                );
            }
            return DisplayListReuseReport::from_entry(
                entry,
                DisplayListReuseOutcome::MissDirty,
                dirty,
                self.frame,
            );
        }

        let outcome = if self.was_evicted(key) {
            DisplayListReuseOutcome::MissEvicted
        } else {
            DisplayListReuseOutcome::MissAbsent
        };
        DisplayListReuseReport::missing(key.clone(), outcome, dirty, self.frame)
    }

    pub fn invalidate(&mut self, request: DisplayListInvalidationRequest) -> usize {
        self.invalidate_with_report(request).removed_count()
    }

    pub fn invalidate_with_report(
        &mut self,
        request: DisplayListInvalidationRequest,
    ) -> DisplayListInvalidationReport {
        let before_len = self.entries.len();
        let mut removed_keys = Vec::new();
        match request {
            DisplayListInvalidationRequest::All => {
                removed_keys.extend(self.entries.keys().cloned());
                self.entries.clear();
            }
            DisplayListInvalidationRequest::Scope(ref scope) => {
                self.entries.retain(|key, _| {
                    if key.scope == *scope {
                        removed_keys.push(key.clone());
                        false
                    } else {
                        true
                    }
                });
            }
            DisplayListInvalidationRequest::Id(ref id) => {
                self.entries.retain(|key, _| {
                    if key.id == *id {
                        removed_keys.push(key.clone());
                        false
                    } else {
                        true
                    }
                });
            }
            DisplayListInvalidationRequest::Dirty(dirty) => {
                self.entries.retain(|key, entry| {
                    if entry.invalidation.invalidated_by(dirty) {
                        removed_keys.push(key.clone());
                        false
                    } else {
                        true
                    }
                });
            }
        }
        DisplayListInvalidationReport {
            request,
            removed_keys,
            before_len,
            after_len: self.entries.len(),
            frame: self.frame,
        }
    }

    fn evict_to_limit(&mut self) {
        let Some(max_entries) = self.max_entries else {
            return;
        };
        while self.entries.len() > max_entries {
            let Some(key) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| (entry.last_used_frame, entry.created_frame))
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            if self.entries.remove(&key).is_some() {
                self.record_eviction(key);
            }
        }
    }

    fn record_eviction(&mut self, key: DisplayListKey) {
        self.evicted_keys.retain(|evicted| evicted != &key);
        self.evicted_keys.push(key);
        if self.evicted_keys.len() > MAX_EVICTION_HISTORY {
            let excess = self.evicted_keys.len() - MAX_EVICTION_HISTORY;
            self.evicted_keys.drain(0..excess);
        }
    }
}

const fn dirty_flags_intersect(left: DirtyFlags, right: DirtyFlags) -> bool {
    (left.layout && right.layout)
        || (left.paint && right.paint)
        || (left.input && right.input)
        || (left.theme && right.theme)
        || (left.text_measurement && right.text_measurement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{platform::LayerOrder, ColorRgba, PaintItem, PaintKind, PaintTransform, UiRect};

    fn paint_list(items: usize) -> PaintList {
        PaintList {
            items: (0..items)
                .map(|index| PaintItem {
                    node: UiNodeId(index),
                    rect: UiRect::new(index as f32, 0.0, 1.0, 1.0),
                    clip_rect: UiRect::new(0.0, 0.0, 100.0, 100.0),
                    z_index: 0.0,
                    layer_order: LayerOrder::DEFAULT,
                    opacity: 1.0,
                    transform: PaintTransform::default(),
                    shader: None,
                    material: None,
                    kind: PaintKind::Rect {
                        fill: ColorRgba::new(10, 20, 30, 255),
                        stroke: None,
                        corner_radius: 0.0,
                    },
                })
                .collect(),
        }
    }

    #[test]
    fn static_editor_background_reuses_across_input_dirty_and_blocks_paint_dirty() {
        let mut cache = RetainedDisplayListCache::new();
        let key = cache.insert_static_editor_background("value-grid", 4, paint_list(3));

        let input_dirty = DirtyFlags {
            input: true,
            ..DirtyFlags::NONE
        };
        let paint_dirty = DirtyFlags {
            paint: true,
            ..DirtyFlags::NONE
        };

        assert_eq!(
            cache.get_reusable(&key, input_dirty).unwrap().items.len(),
            3
        );
        let reused = cache.reuse_report(&key, input_dirty);
        assert_eq!(reused.outcome, DisplayListReuseOutcome::Reused);
        assert_eq!(reused.item_count, Some(3));
        assert_eq!(reused.kind, Some(DisplayListKind::StaticBackground));
        assert!(cache.get_reusable(&key, paint_dirty).is_none());
        let dirty_miss = cache.reuse_report(&key, paint_dirty);
        assert_eq!(dirty_miss.outcome, DisplayListReuseOutcome::MissDirty);
        assert_eq!(
            dirty_miss.invalidation,
            Some(DisplayListInvalidation::STATIC_EDITOR_BACKGROUND)
        );
        assert!(cache.contains_key(&key));

        assert_eq!(
            cache.invalidate(DisplayListInvalidationRequest::Dirty(paint_dirty)),
            1
        );
        assert!(!cache.contains_key(&key));
    }

    #[test]
    fn cache_capacity_evicts_least_recently_used_display_list() {
        let mut cache = RetainedDisplayListCache::with_capacity_limit(2);
        let first = DisplayListKey::node(UiNodeId(1), "first", 0);
        let second = DisplayListKey::node(UiNodeId(2), "second", 0);
        let third = DisplayListKey::node(UiNodeId(3), "third", 0);

        cache.insert(
            first.clone(),
            DisplayListKind::StaticPanel,
            DisplayListInvalidation::STATIC_PANEL,
            paint_list(1),
        );
        cache.advance_frame();
        cache.insert(
            second.clone(),
            DisplayListKind::StaticPanel,
            DisplayListInvalidation::STATIC_PANEL,
            paint_list(2),
        );
        cache.advance_frame();
        assert!(cache.get_reusable(&first, DirtyFlags::NONE).is_some());
        cache.advance_frame();
        cache.insert(
            third.clone(),
            DisplayListKind::StaticPanel,
            DisplayListInvalidation::STATIC_PANEL,
            paint_list(3),
        );

        assert!(cache.contains_key(&first));
        assert!(!cache.contains_key(&second));
        assert!(cache.contains_key(&third));
        assert!(cache.was_evicted(&second));
        let evicted = cache.reuse_report(&second, DirtyFlags::NONE);
        assert_eq!(evicted.outcome, DisplayListReuseOutcome::MissEvicted);

        cache.insert(
            second.clone(),
            DisplayListKind::StaticPanel,
            DisplayListInvalidation::STATIC_PANEL,
            paint_list(4),
        );
        assert!(!cache.was_evicted(&second));
    }

    #[test]
    fn invalidation_requests_remove_matching_scope_id_and_all_entries() {
        let mut cache = RetainedDisplayListCache::new();
        let node_key = DisplayListKey::node(UiNodeId(4), "grid", 1);
        let editor_key = DisplayListKey::editor_background("timeline", 1);
        let custom_key = DisplayListKey::new(DisplayListScope::custom("meter"), "peak", 0);

        cache.insert(
            node_key.clone(),
            DisplayListKind::StaticPanel,
            DisplayListInvalidation::STATIC_PANEL,
            paint_list(1),
        );
        cache.insert(
            editor_key.clone(),
            DisplayListKind::StaticBackground,
            DisplayListInvalidation::STATIC_EDITOR_BACKGROUND,
            paint_list(2),
        );
        cache.insert(
            custom_key.clone(),
            DisplayListKind::DynamicOverlay,
            DisplayListInvalidation::INPUT_OVERLAY,
            paint_list(3),
        );

        let report = cache.invalidate_with_report(DisplayListInvalidationRequest::Scope(
            DisplayListScope::EditorSurface("timeline".into()),
        ));
        assert_eq!(report.removed_count(), 1);
        assert!(report.removed_key(&editor_key));
        assert_eq!(report.before_len, 3);
        assert_eq!(report.after_len, 2);
        assert!(!cache.contains_key(&editor_key));
        assert_eq!(
            cache.invalidate(DisplayListInvalidationRequest::Id(DisplayListId::new(
                "peak"
            ))),
            1
        );
        assert!(!cache.contains_key(&custom_key));
        assert_eq!(cache.invalidate(DisplayListInvalidationRequest::All), 1);
        assert!(cache.is_empty());
    }

    #[test]
    fn display_list_entries_record_metadata_and_usage_frames() {
        let mut cache = RetainedDisplayListCache::new();
        cache.advance_frame();
        let key = DisplayListKey::new(DisplayListScope::Document, "snapshot", 9);
        cache.insert(
            key.clone(),
            DisplayListKind::Snapshot,
            DisplayListInvalidation::ANY,
            paint_list(5),
        );

        let entry = cache.entry(&key).unwrap();
        assert_eq!(entry.item_count, 5);
        assert_eq!(entry.created_frame, 1);
        assert_eq!(entry.last_used_frame, 1);

        cache.advance_frame();
        assert!(cache.get_reusable(&key, DirtyFlags::NONE).is_some());
        assert_eq!(cache.entry(&key).unwrap().last_used_frame, 2);
        assert!(cache.get_reusable(&key, DirtyFlags::ALL).is_none());
    }
}
