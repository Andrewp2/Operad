//! Pending ordered writes and replaceable resource snapshots.

use super::*;

#[derive(Debug, Default)]
pub(crate) struct PendingResourceUploads {
    updates: Vec<renderer::ResourceUpdate>,
    snapshots: HashSet<usize>,
    dirty: bool,
}

impl PendingResourceUploads {
    pub fn push_update(&mut self, update: renderer::ResourceUpdate) {
        self.updates.push(update);
    }

    pub fn push_snapshot(&mut self, update: renderer::ResourceUpdate) {
        debug_assert!(!update.is_partial());
        self.snapshots.insert(self.updates.len());
        self.updates.push(update);
        self.dirty = true;
    }

    pub fn append(&mut self, mut newer: Self) {
        self.dirty |= !newer.snapshots.is_empty();
        let offset = self.updates.len();
        self.snapshots
            .extend(newer.snapshots.into_iter().map(|index| index + offset));
        self.updates.append(&mut newer.updates);
    }

    pub fn as_slice(&self) -> &[renderer::ResourceUpdate] {
        &self.updates
    }

    pub fn clear(&mut self) {
        self.updates.clear();
        self.snapshots.clear();
        self.dirty = false;
    }

    fn compact(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        let mut keep = vec![true; self.updates.len()];
        {
            // Some backends resolve different handle kinds/domains through the
            // same textual key. Preserve intervening aliases and their bases.
            let mut newer = HashMap::<&str, (&platform::ResourceHandle, bool)>::new();
            for index in (0..self.updates.len()).rev() {
                let handle = &self.updates[index].descriptor.handle;
                let state = newer.entry(&handle.id().key).or_insert((handle, false));
                if state.0 != handle {
                    *state = (handle, false);
                }
                if state.1 {
                    keep[index] = false;
                } else if self.snapshots.contains(&index) {
                    state.1 = true;
                }
            }
        }
        let previous_snapshots = std::mem::take(&mut self.snapshots);
        let mut snapshots = HashSet::with_capacity(previous_snapshots.len());
        let mut original_index = 0;
        let mut retained_index = 0;
        self.updates.retain(|_| {
            let retain = keep[original_index];
            if retain {
                if previous_snapshots.contains(&original_index) {
                    snapshots.insert(retained_index);
                }
                retained_index += 1;
            }
            original_index += 1;
            retain
        });
        self.snapshots = snapshots;
    }
}

impl UiDocument {
    pub(crate) fn take_resource_updates(&mut self) -> PendingResourceUploads {
        std::mem::take(&mut self.resource_updates)
    }

    pub(crate) fn append_resource_updates(&mut self, updates: PendingResourceUploads) {
        self.resource_updates.append(updates);
    }

    pub(crate) fn compact_resource_updates(&mut self) {
        self.resource_updates.compact();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{ImageHandle, PixelSize, ResourceHandle, ResourceId, TextureHandle};
    use crate::renderer::{PixelRect, ResourceDescriptor, ResourceFormat, ResourceUpdate};
    use std::hash::Hash;

    fn full(handle: &ResourceHandle, value: u8) -> ResourceUpdate {
        ResourceUpdate::full(
            ResourceDescriptor::new(handle.clone(), PixelSize::new(4, 3), ResourceFormat::Alpha8),
            vec![value; 12],
        )
    }

    fn pixels<K: Eq + Hash>(
        updates: &[ResourceUpdate],
        key: impl Fn(&ResourceHandle) -> K,
    ) -> HashMap<K, (PixelSize, Vec<u8>)> {
        let mut images = HashMap::<K, (PixelSize, Vec<u8>)>::new();
        for update in updates {
            assert!(update.has_expected_byte_len() && update.dirty_rect_is_valid());
            let key = key(&update.descriptor.handle);
            if let Some(rect) = update.dirty_rect {
                let (size, bytes) = images
                    .get_mut(&key)
                    .expect("partial update requires a base");
                assert_eq!(*size, update.descriptor.size);
                for y in 0..rect.height as usize {
                    let destination = (rect.y as usize + y) * size.width as usize + rect.x as usize;
                    let source = y * rect.width as usize;
                    bytes[destination..destination + rect.width as usize]
                        .copy_from_slice(&update.bytes[source..source + rect.width as usize]);
                }
            } else {
                images.insert(key, (update.descriptor.size, update.bytes.to_vec()));
            }
        }
        images
    }

    #[test]
    fn compaction_preserves_pixels_across_interleaved_snapshots_patches_and_aliases() {
        let handles = [
            ResourceHandle::from(ImageHandle::app("a")),
            ResourceHandle::from(TextureHandle::host("a")),
            ResourceHandle::from(ImageHandle::app("b")),
            ResourceHandle::from(ImageHandle::from_id(ResourceId::built_in("b"))),
        ];
        let mut removed = 0;
        for seed in 0..64_u64 {
            let mut state = seed;
            let mut next = || {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (state >> 32) as usize
            };
            let mut original = Vec::new();
            let mut pending = PendingResourceUploads::default();
            for handle in &handles {
                let update = full(handle, 0);
                pending.push_update(update.clone());
                original.push(update);
            }
            for step in 0..128 {
                let mut fragment = PendingResourceUploads::default();
                let handle = &handles[next() % handles.len()];
                let value = next() as u8;
                let mut update = full(handle, value);
                match next() % 3 {
                    0 => fragment.push_snapshot(update.clone()),
                    1 => fragment.push_update(update.clone()),
                    _ => {
                        update.dirty_rect = Some(PixelRect::new(
                            (next() % 3) as u32,
                            (next() % 2) as u32,
                            2,
                            2,
                        ));
                        update.bytes = vec![value; 4].into();
                        fragment.push_update(update.clone());
                    }
                }
                original.push(update);
                pending.append(fragment);
                if step % 7 == 0 {
                    pending.compact();
                }
            }
            pending.compact();
            assert_eq!(
                pixels(pending.as_slice(), ResourceHandle::clone),
                pixels(&original, ResourceHandle::clone),
                "typed resource identities, seed {seed}"
            );
            assert_eq!(
                pixels(pending.as_slice(), |handle| handle.id().key.clone()),
                pixels(&original, |handle| handle.id().key.clone()),
                "textual-key aliases, seed {seed}"
            );
            removed += original.len() - pending.as_slice().len();
        }
        assert!(removed > 0, "generated workloads must exercise compaction");
    }

    #[test]
    fn aliased_patch_keeps_its_base_before_a_later_snapshot_changes_shape() {
        let image = ResourceHandle::from(ImageHandle::app("shared"));
        let alias = ResourceHandle::from(TextureHandle::host("shared"));
        let base = full(&image, 1);
        let patch = ResourceUpdate::partial(
            ResourceDescriptor::new(alias, PixelSize::new(4, 3), ResourceFormat::Alpha8),
            PixelRect::new(1, 1, 2, 1),
            vec![2; 2],
        );
        let replacement = ResourceUpdate::full(
            ResourceDescriptor::new(image, PixelSize::new(2, 2), ResourceFormat::Alpha8),
            vec![3; 4],
        );
        let mut pending = PendingResourceUploads::default();
        pending.push_snapshot(base.clone());
        pending.push_update(patch.clone());
        pending.push_snapshot(replacement.clone());
        pending.compact();
        let expected = pixels(&[base, patch, replacement], |handle| {
            handle.id().key.clone()
        });
        assert_eq!(
            pixels(pending.as_slice(), |handle| handle.id().key.clone()),
            expected
        );
    }

    #[test]
    fn ordered_writes_and_current_validation_survive_snapshot_compaction() {
        let image = ResourceHandle::from(ImageHandle::app("ordered"));
        let other = ResourceHandle::from(ImageHandle::app("snapshot"));
        let mut newer = full(&image, 1);
        newer.descriptor.version = 8;
        let mut stale = full(&image, 2);
        stale.descriptor.version = 7;
        let mut invalid = full(&image, 3);
        invalid.bytes = vec![3; 1].into();
        let original = vec![newer, stale, invalid];
        let mut pending = PendingResourceUploads::default();
        for update in &original {
            pending.push_update(update.clone());
        }
        pending.push_snapshot(full(&other, 1));
        pending.push_snapshot(full(&other, 2));
        pending.compact();
        let ordered: Vec<_> = pending
            .as_slice()
            .iter()
            .filter(|update| update.descriptor.handle == image)
            .cloned()
            .collect();
        assert_eq!(
            ordered, original,
            "explicit writes must retain their versions and validation errors"
        );

        // An explicit complete replacement makes even invalid, unsubmitted
        // previous proposals obsolete. Its own validation must still run.
        pending.push_snapshot(full(&image, 4));
        pending.compact();
        assert!(pending
            .as_slice()
            .iter()
            .all(ResourceUpdate::has_expected_byte_len));
        let mut invalid_snapshot = full(&image, 5);
        invalid_snapshot.bytes = vec![5; 1].into();
        pending.push_snapshot(invalid_snapshot);
        pending.compact();
        let current = pending
            .as_slice()
            .iter()
            .find(|update| update.descriptor.handle == image)
            .unwrap();
        assert!(
            !current.has_expected_byte_len(),
            "compaction cannot hide the current invalid snapshot"
        );
    }
}
