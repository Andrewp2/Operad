//! Retained display-list cache contracts for static and semi-static surfaces.
//!
//! This module is intentionally renderer-neutral. It lets applications and
//! backends retain expensive paint lists for editor backgrounds, ruler grids,
//! static panels, and snapshots, while invalidating them through explicit dirty
//! flags rather than backend-specific cache state.

use std::collections::HashMap;

use crate::testing::DirtyFlags;
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RetainedDisplayListCache {
    entries: HashMap<DisplayListKey, RetainedDisplayList>,
    max_entries: Option<usize>,
    frame: u64,
}

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

    pub fn insert(
        &mut self,
        key: DisplayListKey,
        kind: DisplayListKind,
        invalidation: DisplayListInvalidation,
        paint: PaintList,
    ) {
        let entry = RetainedDisplayList::new(key.clone(), kind, invalidation, paint, self.frame);
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

    pub fn invalidate(&mut self, request: DisplayListInvalidationRequest) -> usize {
        let before = self.entries.len();
        match request {
            DisplayListInvalidationRequest::All => self.entries.clear(),
            DisplayListInvalidationRequest::Scope(scope) => {
                self.entries.retain(|key, _| key.scope != scope);
            }
            DisplayListInvalidationRequest::Id(id) => {
                self.entries.retain(|key, _| key.id != id);
            }
            DisplayListInvalidationRequest::Dirty(dirty) => {
                self.entries
                    .retain(|_, entry| !entry.invalidation.invalidated_by(dirty));
            }
        }
        before - self.entries.len()
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
            self.entries.remove(&key);
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
                    z_index: 0,
                    layer_order: LayerOrder::DEFAULT,
                    opacity: 1.0,
                    transform: PaintTransform::default(),
                    shader: None,
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
        assert!(cache.get_reusable(&key, paint_dirty).is_none());
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

        assert_eq!(
            cache.invalidate(DisplayListInvalidationRequest::Scope(
                DisplayListScope::EditorSurface("timeline".into())
            )),
            1
        );
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
