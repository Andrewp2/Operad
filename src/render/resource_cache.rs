//! Backend-neutral resource lifecycle and cache accounting.
//!
//! This module tracks texture-like resources without owning backend objects.
//! Backends can use it to validate uploads, reject stale versions, account for
//! uploaded bytes, and plan evictions before touching GPU or host renderer APIs.

use std::collections::{HashMap, HashSet};

use crate::platform::{ResourceDomain, ResourceHandle, ResourceId};
use crate::renderer::{PixelRect, ResourceDescriptor, ResourceResolver, ResourceUpdate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceLifecycleOutcome {
    Full,
    Partial,
    Stale,
    Rejected,
    Retained,
    Evicted,
}

impl ResourceLifecycleOutcome {
    pub const fn uploaded(self) -> bool {
        matches!(self, Self::Full | Self::Partial)
    }

    pub const fn retained(self) -> bool {
        matches!(self, Self::Retained)
    }

    pub const fn rejected(self) -> bool {
        matches!(self, Self::Rejected | Self::Stale)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceUpdateKind {
    Full,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceUpdateIssue {
    MissingResource,
    DescriptorMismatch {
        expected: ResourceDescriptor,
        actual: ResourceDescriptor,
    },
    StaleVersion {
        current_version: u64,
        update_version: u64,
    },
    InvalidByteLength {
        expected: Option<usize>,
        actual: usize,
    },
    InvalidDirtyRect {
        rect: PixelRect,
        size: crate::platform::PixelSize,
    },
    PartialUpdateMissingBase,
    PartialDescriptorChange {
        current: ResourceDescriptor,
        update: ResourceDescriptor,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUpdateValidation {
    pub handle: ResourceHandle,
    pub outcome: ResourceLifecycleOutcome,
    pub update_kind: Option<ResourceUpdateKind>,
    pub issue: Option<ResourceUpdateIssue>,
    pub expected_upload_bytes: Option<usize>,
    pub actual_upload_bytes: usize,
    pub resource_bytes: Option<usize>,
    pub dirty_rect: Option<PixelRect>,
    pub update_version: u64,
    pub current_version: Option<u64>,
}

impl ResourceUpdateValidation {
    pub const fn is_uploadable(&self) -> bool {
        matches!(
            self.outcome,
            ResourceLifecycleOutcome::Full | ResourceLifecycleOutcome::Partial
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedResource {
    pub descriptor: ResourceDescriptor,
    pub bytes: usize,
    pub created_frame: u64,
    pub last_used_frame: u64,
    pub last_update_frame: u64,
    pub upload_count: u64,
    pub full_upload_count: u64,
    pub partial_upload_count: u64,
    pub retained_count: u64,
    pub uploaded_bytes: usize,
    pub full_uploaded_bytes: usize,
    pub partial_uploaded_bytes: usize,
}

impl CachedResource {
    fn new(
        descriptor: ResourceDescriptor,
        bytes: usize,
        uploaded_bytes: usize,
        frame: u64,
    ) -> Self {
        Self {
            descriptor,
            bytes,
            created_frame: frame,
            last_used_frame: frame,
            last_update_frame: frame,
            upload_count: 1,
            full_upload_count: 1,
            partial_upload_count: 0,
            retained_count: 0,
            uploaded_bytes,
            full_uploaded_bytes: uploaded_bytes,
            partial_uploaded_bytes: 0,
        }
    }

    fn apply_full_update(
        &mut self,
        descriptor: ResourceDescriptor,
        bytes: usize,
        upload_bytes: usize,
        frame: u64,
    ) {
        self.descriptor = descriptor;
        self.bytes = bytes;
        self.last_used_frame = frame;
        self.last_update_frame = frame;
        self.upload_count = self.upload_count.saturating_add(1);
        self.full_upload_count = self.full_upload_count.saturating_add(1);
        self.uploaded_bytes = self.uploaded_bytes.saturating_add(upload_bytes);
        self.full_uploaded_bytes = self.full_uploaded_bytes.saturating_add(upload_bytes);
    }

    fn apply_partial_update(
        &mut self,
        descriptor: ResourceDescriptor,
        upload_bytes: usize,
        frame: u64,
    ) {
        self.descriptor = descriptor;
        self.last_used_frame = frame;
        self.last_update_frame = frame;
        self.upload_count = self.upload_count.saturating_add(1);
        self.partial_upload_count = self.partial_upload_count.saturating_add(1);
        self.uploaded_bytes = self.uploaded_bytes.saturating_add(upload_bytes);
        self.partial_uploaded_bytes = self.partial_uploaded_bytes.saturating_add(upload_bytes);
    }

    fn retain(&mut self, frame: u64) {
        self.last_used_frame = frame;
        self.retained_count = self.retained_count.saturating_add(1);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUpdateReport {
    pub handle: ResourceHandle,
    pub outcome: ResourceLifecycleOutcome,
    pub update_kind: Option<ResourceUpdateKind>,
    pub issue: Option<ResourceUpdateIssue>,
    pub frame: u64,
    pub update_version: Option<u64>,
    pub current_version: Option<u64>,
    pub bytes_uploaded: usize,
    pub resource_bytes: Option<usize>,
    pub total_bytes_before: usize,
    pub total_bytes_after: usize,
    pub created_frame: Option<u64>,
    pub last_used_frame: Option<u64>,
    pub dirty_rect: Option<PixelRect>,
}

impl ResourceUpdateReport {
    pub const fn uploaded(&self) -> bool {
        self.outcome.uploaded()
    }

    pub const fn rejected(&self) -> bool {
        self.outcome.rejected()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceCachePolicy {
    pub max_bytes: Option<usize>,
    pub max_unused_frames: Option<u64>,
}

impl ResourceCachePolicy {
    pub const UNBOUNDED: Self = Self {
        max_bytes: None,
        max_unused_frames: None,
    };

    pub const fn unbounded() -> Self {
        Self::UNBOUNDED
    }

    pub const fn max_bytes(max_bytes: usize) -> Self {
        Self {
            max_bytes: Some(max_bytes),
            max_unused_frames: None,
        }
    }

    pub const fn max_unused_frames(max_unused_frames: u64) -> Self {
        Self {
            max_bytes: None,
            max_unused_frames: Some(max_unused_frames),
        }
    }

    pub const fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = Some(max_bytes);
        self
    }

    pub const fn with_max_unused_frames(mut self, max_unused_frames: u64) -> Self {
        self.max_unused_frames = Some(max_unused_frames);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceEvictionReason {
    ByteBudget,
    Age,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEvictionCandidate {
    pub handle: ResourceHandle,
    pub descriptor: ResourceDescriptor,
    pub bytes: usize,
    pub created_frame: u64,
    pub last_used_frame: u64,
    pub age_frames: u64,
    pub reasons: Vec<ResourceEvictionReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEvictionPlan {
    pub policy: ResourceCachePolicy,
    pub frame: u64,
    pub total_bytes_before: usize,
    pub bytes_to_free: usize,
    pub total_bytes_after: usize,
    pub candidates: Vec<ResourceEvictionCandidate>,
}

impl ResourceEvictionPlan {
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    pub fn evicted_count(&self) -> usize {
        self.candidates.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEvictionReport {
    pub policy: ResourceCachePolicy,
    pub frame: u64,
    pub total_bytes_before: usize,
    pub total_bytes_after: usize,
    pub evicted: Vec<ResourceUpdateReport>,
}

impl ResourceEvictionReport {
    pub fn evicted_count(&self) -> usize {
        self.evicted.len()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceCache {
    entries: HashMap<ResourceHandle, CachedResource>,
    frame: u64,
    total_bytes: usize,
}

impl ResourceCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn frame(&self) -> u64 {
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

    pub const fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn contains(&self, handle: &ResourceHandle) -> bool {
        self.entries.contains_key(handle)
    }

    pub fn entry(&self, handle: &ResourceHandle) -> Option<&CachedResource> {
        self.entries.get(handle)
    }

    pub fn apply_update(&mut self, update: &ResourceUpdate) -> ResourceUpdateReport {
        self.apply_update_with_descriptor(update, Some(&update.descriptor))
    }

    pub fn apply_resolved_update(
        &mut self,
        update: &ResourceUpdate,
        resolver: &dyn ResourceResolver,
    ) -> ResourceUpdateReport {
        let resolved = resolver.resolve_resource(update.descriptor.handle.id());
        self.apply_update_with_descriptor(update, resolved.as_ref())
    }

    pub fn apply_update_with_descriptor(
        &mut self,
        update: &ResourceUpdate,
        expected: Option<&ResourceDescriptor>,
    ) -> ResourceUpdateReport {
        let before = self.total_bytes;
        let validation = validate_resource_update_with_descriptor(update, expected);
        if !validation.is_uploadable() {
            return self.report_from_validation(validation, before, self.total_bytes);
        }

        let handle = update.descriptor.handle.clone();
        if let Some(entry) = self.entries.get(&handle) {
            if update.descriptor.version < entry.descriptor.version {
                return self.stale_report(
                    update,
                    before,
                    entry.descriptor.version,
                    ResourceUpdateIssue::StaleVersion {
                        current_version: entry.descriptor.version,
                        update_version: update.descriptor.version,
                    },
                );
            }

            if update.descriptor.version == entry.descriptor.version {
                if !same_resource_shape(&entry.descriptor, &update.descriptor) {
                    return self.rejected_report(
                        update,
                        before,
                        ResourceUpdateIssue::DescriptorMismatch {
                            expected: entry.descriptor.clone(),
                            actual: update.descriptor.clone(),
                        },
                    );
                }
                return self.retain_existing(&handle, before);
            }

            if update.is_partial() && !same_resource_shape(&entry.descriptor, &update.descriptor) {
                return self.rejected_report(
                    update,
                    before,
                    ResourceUpdateIssue::PartialDescriptorChange {
                        current: entry.descriptor.clone(),
                        update: update.descriptor.clone(),
                    },
                );
            }
        } else if update.is_partial() {
            return self.rejected_report(
                update,
                before,
                ResourceUpdateIssue::PartialUpdateMissingBase,
            );
        }

        let upload_bytes = validation.expected_upload_bytes.unwrap_or(0);
        let resource_bytes = validation.resource_bytes.unwrap_or(0);
        match validation.update_kind {
            Some(ResourceUpdateKind::Full) => {
                let (previous_bytes, entry) = match self.entries.remove(&handle) {
                    Some(mut entry) => {
                        let previous_bytes = entry.bytes;
                        entry.apply_full_update(
                            update.descriptor.clone(),
                            resource_bytes,
                            upload_bytes,
                            self.frame,
                        );
                        (previous_bytes, entry)
                    }
                    None => (
                        0,
                        CachedResource::new(
                            update.descriptor.clone(),
                            resource_bytes,
                            upload_bytes,
                            self.frame,
                        ),
                    ),
                };
                self.total_bytes = self
                    .total_bytes
                    .saturating_sub(previous_bytes)
                    .saturating_add(resource_bytes);
                let report = report_from_entry(
                    &entry,
                    ResourceLifecycleOutcome::Full,
                    Some(ResourceUpdateKind::Full),
                    None,
                    self.frame,
                    Some(update.descriptor.version),
                    upload_bytes,
                    Some(resource_bytes),
                    before,
                    self.total_bytes,
                    update.dirty_rect,
                );
                self.entries.insert(handle, entry);
                report
            }
            Some(ResourceUpdateKind::Partial) => {
                let entry = self
                    .entries
                    .get_mut(&handle)
                    .expect("partial uploads require an existing cache entry");
                entry.apply_partial_update(update.descriptor.clone(), upload_bytes, self.frame);
                report_from_entry(
                    entry,
                    ResourceLifecycleOutcome::Partial,
                    Some(ResourceUpdateKind::Partial),
                    None,
                    self.frame,
                    Some(update.descriptor.version),
                    upload_bytes,
                    Some(entry.bytes),
                    before,
                    self.total_bytes,
                    update.dirty_rect,
                )
            }
            None => self.report_from_validation(validation, before, self.total_bytes),
        }
    }

    pub fn retain(&mut self, handle: &ResourceHandle) -> ResourceUpdateReport {
        let before = self.total_bytes;
        self.retain_existing(handle, before)
    }

    pub fn plan_eviction_by_byte_budget(&self, max_bytes: usize) -> ResourceEvictionPlan {
        self.plan_eviction(ResourceCachePolicy::max_bytes(max_bytes))
    }

    pub fn plan_eviction_by_age(&self, max_unused_frames: u64) -> ResourceEvictionPlan {
        self.plan_eviction(ResourceCachePolicy::max_unused_frames(max_unused_frames))
    }

    pub fn plan_eviction(&self, policy: ResourceCachePolicy) -> ResourceEvictionPlan {
        let mut selected = HashSet::<ResourceHandle>::new();
        let mut candidates = Vec::<ResourceEvictionCandidate>::new();

        if let Some(max_unused_frames) = policy.max_unused_frames {
            for (handle, entry) in sorted_entries(&self.entries) {
                let age = self.frame.saturating_sub(entry.last_used_frame);
                if age > max_unused_frames {
                    selected.insert(handle.clone());
                    candidates.push(eviction_candidate(
                        handle,
                        entry,
                        self.frame,
                        vec![ResourceEvictionReason::Age],
                    ));
                }
            }
        }

        if let Some(max_bytes) = policy.max_bytes {
            let age_selected_bytes = candidates
                .iter()
                .map(|candidate| candidate.bytes)
                .sum::<usize>();
            let mut projected_bytes = self.total_bytes.saturating_sub(age_selected_bytes);
            if projected_bytes > max_bytes {
                for (handle, entry) in sorted_entries(&self.entries) {
                    if selected.contains(handle) {
                        continue;
                    }
                    selected.insert(handle.clone());
                    projected_bytes = projected_bytes.saturating_sub(entry.bytes);
                    candidates.push(eviction_candidate(
                        handle,
                        entry,
                        self.frame,
                        vec![ResourceEvictionReason::ByteBudget],
                    ));
                    if projected_bytes <= max_bytes {
                        break;
                    }
                }
            }
        }

        candidates.sort_by(eviction_candidate_order);
        let bytes_to_free = candidates
            .iter()
            .map(|candidate| candidate.bytes)
            .sum::<usize>();
        ResourceEvictionPlan {
            policy,
            frame: self.frame,
            total_bytes_before: self.total_bytes,
            bytes_to_free,
            total_bytes_after: self.total_bytes.saturating_sub(bytes_to_free),
            candidates,
        }
    }

    pub fn evict_planned(&mut self, plan: &ResourceEvictionPlan) -> ResourceEvictionReport {
        let before = self.total_bytes;
        let mut evicted = Vec::new();
        for candidate in &plan.candidates {
            let Some(entry) = self.entries.remove(&candidate.handle) else {
                continue;
            };
            let entry_bytes = entry.bytes;
            self.total_bytes = self.total_bytes.saturating_sub(entry_bytes);
            evicted.push(report_from_entry(
                &entry,
                ResourceLifecycleOutcome::Evicted,
                None,
                None,
                self.frame,
                None,
                0,
                Some(entry_bytes),
                before,
                self.total_bytes,
                None,
            ));
        }
        ResourceEvictionReport {
            policy: plan.policy,
            frame: self.frame,
            total_bytes_before: before,
            total_bytes_after: self.total_bytes,
            evicted,
        }
    }

    fn retain_existing(&mut self, handle: &ResourceHandle, before: usize) -> ResourceUpdateReport {
        match self.entries.get_mut(handle) {
            Some(entry) => {
                entry.retain(self.frame);
                report_from_entry(
                    entry,
                    ResourceLifecycleOutcome::Retained,
                    None,
                    None,
                    self.frame,
                    Some(entry.descriptor.version),
                    0,
                    Some(entry.bytes),
                    before,
                    self.total_bytes,
                    None,
                )
            }
            None => ResourceUpdateReport {
                handle: handle.clone(),
                outcome: ResourceLifecycleOutcome::Rejected,
                update_kind: None,
                issue: Some(ResourceUpdateIssue::MissingResource),
                frame: self.frame,
                update_version: None,
                current_version: None,
                bytes_uploaded: 0,
                resource_bytes: None,
                total_bytes_before: before,
                total_bytes_after: self.total_bytes,
                created_frame: None,
                last_used_frame: None,
                dirty_rect: None,
            },
        }
    }

    fn report_from_validation(
        &self,
        validation: ResourceUpdateValidation,
        before: usize,
        after: usize,
    ) -> ResourceUpdateReport {
        ResourceUpdateReport {
            handle: validation.handle,
            outcome: validation.outcome,
            update_kind: validation.update_kind,
            issue: validation.issue,
            frame: self.frame,
            update_version: Some(validation.update_version),
            current_version: validation.current_version,
            bytes_uploaded: 0,
            resource_bytes: validation.resource_bytes,
            total_bytes_before: before,
            total_bytes_after: after,
            created_frame: None,
            last_used_frame: None,
            dirty_rect: validation.dirty_rect,
        }
    }

    fn stale_report(
        &self,
        update: &ResourceUpdate,
        before: usize,
        current_version: u64,
        issue: ResourceUpdateIssue,
    ) -> ResourceUpdateReport {
        ResourceUpdateReport {
            handle: update.descriptor.handle.clone(),
            outcome: ResourceLifecycleOutcome::Stale,
            update_kind: update_kind(update),
            issue: Some(issue),
            frame: self.frame,
            update_version: Some(update.descriptor.version),
            current_version: Some(current_version),
            bytes_uploaded: 0,
            resource_bytes: resource_descriptor_byte_len(&update.descriptor),
            total_bytes_before: before,
            total_bytes_after: self.total_bytes,
            created_frame: None,
            last_used_frame: None,
            dirty_rect: update.dirty_rect,
        }
    }

    fn rejected_report(
        &self,
        update: &ResourceUpdate,
        before: usize,
        issue: ResourceUpdateIssue,
    ) -> ResourceUpdateReport {
        ResourceUpdateReport {
            handle: update.descriptor.handle.clone(),
            outcome: ResourceLifecycleOutcome::Rejected,
            update_kind: update_kind(update),
            issue: Some(issue),
            frame: self.frame,
            update_version: Some(update.descriptor.version),
            current_version: self
                .entries
                .get(&update.descriptor.handle)
                .map(|entry| entry.descriptor.version),
            bytes_uploaded: 0,
            resource_bytes: resource_descriptor_byte_len(&update.descriptor),
            total_bytes_before: before,
            total_bytes_after: self.total_bytes,
            created_frame: None,
            last_used_frame: None,
            dirty_rect: update.dirty_rect,
        }
    }
}

pub fn validate_resource_update(update: &ResourceUpdate) -> ResourceUpdateValidation {
    validate_resource_update_with_descriptor(update, Some(&update.descriptor))
}

pub fn validate_resource_update_with_descriptor(
    update: &ResourceUpdate,
    expected: Option<&ResourceDescriptor>,
) -> ResourceUpdateValidation {
    let handle = update.descriptor.handle.clone();
    let update_version = update.descriptor.version;
    let update_kind = update_kind(update);
    let expected_upload_bytes = update.expected_byte_len();
    let resource_bytes = resource_descriptor_byte_len(&update.descriptor);

    let rejected = |issue: ResourceUpdateIssue| ResourceUpdateValidation {
        handle: handle.clone(),
        outcome: ResourceLifecycleOutcome::Rejected,
        update_kind,
        issue: Some(issue),
        expected_upload_bytes,
        actual_upload_bytes: update.bytes.len(),
        resource_bytes,
        dirty_rect: update.dirty_rect,
        update_version,
        current_version: None,
    };

    let Some(expected) = expected else {
        return rejected(ResourceUpdateIssue::MissingResource);
    };

    if !same_resource_shape(expected, &update.descriptor) {
        return rejected(ResourceUpdateIssue::DescriptorMismatch {
            expected: expected.clone(),
            actual: update.descriptor.clone(),
        });
    }

    if update.descriptor.version < expected.version {
        return ResourceUpdateValidation {
            handle,
            outcome: ResourceLifecycleOutcome::Stale,
            update_kind,
            issue: Some(ResourceUpdateIssue::StaleVersion {
                current_version: expected.version,
                update_version,
            }),
            expected_upload_bytes,
            actual_upload_bytes: update.bytes.len(),
            resource_bytes,
            dirty_rect: update.dirty_rect,
            update_version,
            current_version: Some(expected.version),
        };
    }

    if let Some(rect) = update.dirty_rect {
        if rect.is_empty() || !rect.contains(update.descriptor.size) {
            return rejected(ResourceUpdateIssue::InvalidDirtyRect {
                rect,
                size: update.descriptor.size,
            });
        }
    }

    if expected_upload_bytes != Some(update.bytes.len()) {
        return rejected(ResourceUpdateIssue::InvalidByteLength {
            expected: expected_upload_bytes,
            actual: update.bytes.len(),
        });
    }

    ResourceUpdateValidation {
        handle,
        outcome: match update_kind {
            Some(ResourceUpdateKind::Full) => ResourceLifecycleOutcome::Full,
            Some(ResourceUpdateKind::Partial) => ResourceLifecycleOutcome::Partial,
            None => ResourceLifecycleOutcome::Rejected,
        },
        update_kind,
        issue: None,
        expected_upload_bytes,
        actual_upload_bytes: update.bytes.len(),
        resource_bytes,
        dirty_rect: update.dirty_rect,
        update_version,
        current_version: Some(expected.version),
    }
}

pub fn resource_descriptor_byte_len(descriptor: &ResourceDescriptor) -> Option<usize> {
    let pixels = usize::try_from(descriptor.size.width)
        .ok()?
        .checked_mul(usize::try_from(descriptor.size.height).ok()?)?;
    pixels.checked_mul(descriptor.format.bytes_per_pixel())
}

fn update_kind(update: &ResourceUpdate) -> Option<ResourceUpdateKind> {
    Some(if update.is_partial() {
        ResourceUpdateKind::Partial
    } else {
        ResourceUpdateKind::Full
    })
}

fn same_resource_shape(left: &ResourceDescriptor, right: &ResourceDescriptor) -> bool {
    left.handle == right.handle && left.size == right.size && left.format == right.format
}

fn report_from_entry(
    entry: &CachedResource,
    outcome: ResourceLifecycleOutcome,
    update_kind: Option<ResourceUpdateKind>,
    issue: Option<ResourceUpdateIssue>,
    frame: u64,
    update_version: Option<u64>,
    bytes_uploaded: usize,
    resource_bytes: Option<usize>,
    total_bytes_before: usize,
    total_bytes_after: usize,
    dirty_rect: Option<PixelRect>,
) -> ResourceUpdateReport {
    ResourceUpdateReport {
        handle: entry.descriptor.handle.clone(),
        outcome,
        update_kind,
        issue,
        frame,
        update_version,
        current_version: Some(entry.descriptor.version),
        bytes_uploaded,
        resource_bytes,
        total_bytes_before,
        total_bytes_after,
        created_frame: Some(entry.created_frame),
        last_used_frame: Some(entry.last_used_frame),
        dirty_rect,
    }
}

fn eviction_candidate(
    handle: &ResourceHandle,
    entry: &CachedResource,
    frame: u64,
    reasons: Vec<ResourceEvictionReason>,
) -> ResourceEvictionCandidate {
    ResourceEvictionCandidate {
        handle: handle.clone(),
        descriptor: entry.descriptor.clone(),
        bytes: entry.bytes,
        created_frame: entry.created_frame,
        last_used_frame: entry.last_used_frame,
        age_frames: frame.saturating_sub(entry.last_used_frame),
        reasons,
    }
}

fn sorted_entries(
    entries: &HashMap<ResourceHandle, CachedResource>,
) -> Vec<(&ResourceHandle, &CachedResource)> {
    let mut entries = entries.iter().collect::<Vec<_>>();
    entries.sort_by(|(left_handle, left), (right_handle, right)| {
        (left.last_used_frame, left.created_frame)
            .cmp(&(right.last_used_frame, right.created_frame))
            .then_with(|| {
                resource_handle_order(left_handle).cmp(&resource_handle_order(right_handle))
            })
    });
    entries
}

fn eviction_candidate_order(
    left: &ResourceEvictionCandidate,
    right: &ResourceEvictionCandidate,
) -> std::cmp::Ordering {
    (left.last_used_frame, left.created_frame)
        .cmp(&(right.last_used_frame, right.created_frame))
        .then_with(|| {
            resource_handle_order(&left.handle).cmp(&resource_handle_order(&right.handle))
        })
}

fn resource_handle_order(handle: &ResourceHandle) -> (u8, u8, &str) {
    let kind = match handle {
        ResourceHandle::Image(_) => 0,
        ResourceHandle::Icon(_) => 1,
        ResourceHandle::Texture(_) => 2,
        ResourceHandle::Thumbnail(_) => 3,
    };
    let id = handle.id();
    (kind, resource_domain_order(id), id.key.as_str())
}

fn resource_domain_order(id: &ResourceId) -> u8 {
    match id.domain {
        ResourceDomain::BuiltIn => 0,
        ResourceDomain::App => 1,
        ResourceDomain::Host => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{ImageHandle, PixelSize, ResourceHandle, TextureHandle};
    use crate::renderer::ResourceFormat;

    #[derive(Debug, Clone)]
    struct SingleResolver {
        descriptor: Option<ResourceDescriptor>,
    }

    impl ResourceResolver for SingleResolver {
        fn resolve_resource(&self, id: &ResourceId) -> Option<ResourceDescriptor> {
            self.descriptor
                .clone()
                .filter(|descriptor| descriptor.handle.id() == id)
        }
    }

    fn image_descriptor(key: &str, size: PixelSize, version: u64) -> ResourceDescriptor {
        ResourceDescriptor::new(
            ResourceHandle::Image(ImageHandle::app(key)),
            size,
            ResourceFormat::Rgba8,
        )
        .version(version)
    }

    fn texture_descriptor(key: &str, size: PixelSize, version: u64) -> ResourceDescriptor {
        ResourceDescriptor::new(
            ResourceHandle::Texture(TextureHandle::app(key)),
            size,
            ResourceFormat::Rgba8,
        )
        .version(version)
    }

    fn full_update(descriptor: ResourceDescriptor) -> ResourceUpdate {
        let bytes = resource_descriptor_byte_len(&descriptor).expect("descriptor bytes");
        ResourceUpdate::full(descriptor, vec![7; bytes])
    }

    #[test]
    fn validation_helpers_classify_uploads_and_reject_descriptor_or_dirty_rect_mismatches() {
        let descriptor = image_descriptor("validate", PixelSize::new(4, 4), 2);

        let full = validate_resource_update(&full_update(descriptor.clone()));
        assert_eq!(full.outcome, ResourceLifecycleOutcome::Full);
        assert_eq!(full.update_kind, Some(ResourceUpdateKind::Full));
        assert_eq!(full.expected_upload_bytes, Some(64));

        let partial_update = ResourceUpdate::partial(
            descriptor.clone().version(3),
            PixelRect::new(1, 1, 2, 2),
            vec![3; 2 * 2 * 4],
        );
        let partial = validate_resource_update_with_descriptor(&partial_update, Some(&descriptor));
        assert_eq!(partial.outcome, ResourceLifecycleOutcome::Partial);
        assert_eq!(partial.update_kind, Some(ResourceUpdateKind::Partial));
        assert_eq!(partial.expected_upload_bytes, Some(16));

        let wrong_format = ResourceUpdate::full(
            ResourceDescriptor::new(
                descriptor.handle.clone(),
                descriptor.size,
                ResourceFormat::Bgra8,
            )
            .version(2),
            vec![0; 64],
        );
        let mismatch = validate_resource_update_with_descriptor(&wrong_format, Some(&descriptor));
        assert_eq!(mismatch.outcome, ResourceLifecycleOutcome::Rejected);
        assert!(matches!(
            mismatch.issue,
            Some(ResourceUpdateIssue::DescriptorMismatch { .. })
        ));

        let invalid_rect = ResourceUpdate::partial(
            descriptor.clone(),
            PixelRect::new(3, 3, 2, 2),
            vec![0; 2 * 2 * 4],
        );
        let invalid = validate_resource_update(&invalid_rect);
        assert_eq!(invalid.outcome, ResourceLifecycleOutcome::Rejected);
        assert_eq!(
            invalid.issue,
            Some(ResourceUpdateIssue::InvalidDirtyRect {
                rect: PixelRect::new(3, 3, 2, 2),
                size: PixelSize::new(4, 4)
            })
        );

        let stale_descriptor = descriptor.clone().version(4);
        let stale = validate_resource_update_with_descriptor(
            &full_update(descriptor.clone()),
            Some(&stale_descriptor),
        );
        assert_eq!(stale.outcome, ResourceLifecycleOutcome::Stale);
    }

    #[test]
    fn resolved_update_reports_missing_resource_without_inserting() {
        let descriptor = image_descriptor("missing", PixelSize::new(2, 2), 1);
        let update = full_update(descriptor.clone());
        let mut cache = ResourceCache::new();

        let report = cache.apply_resolved_update(&update, &SingleResolver { descriptor: None });

        assert_eq!(report.outcome, ResourceLifecycleOutcome::Rejected);
        assert_eq!(report.issue, Some(ResourceUpdateIssue::MissingResource));
        assert_eq!(report.bytes_uploaded, 0);
        assert!(!cache.contains(&descriptor.handle));
        assert_eq!(cache.total_bytes(), 0);
    }

    #[test]
    fn stale_versions_are_reported_and_do_not_replace_current_entry() {
        let current = image_descriptor("cover", PixelSize::new(2, 2), 3);
        let stale = image_descriptor("cover", PixelSize::new(2, 2), 2);
        let mut cache = ResourceCache::new();
        let first = cache.apply_update(&full_update(current.clone()));
        assert_eq!(first.outcome, ResourceLifecycleOutcome::Full);

        cache.advance_frame();
        let stale_report = cache.apply_update(&full_update(stale));

        assert_eq!(stale_report.outcome, ResourceLifecycleOutcome::Stale);
        assert_eq!(
            stale_report.issue,
            Some(ResourceUpdateIssue::StaleVersion {
                current_version: 3,
                update_version: 2
            })
        );
        assert_eq!(stale_report.bytes_uploaded, 0);
        let entry = cache.entry(&current.handle).expect("current resource");
        assert_eq!(entry.descriptor.version, 3);
        assert_eq!(entry.last_used_frame, 0);
    }

    #[test]
    fn partial_update_accounting_tracks_uploaded_bytes_without_changing_retained_size() {
        let first_descriptor = texture_descriptor("atlas", PixelSize::new(4, 4), 1);
        let next_descriptor = texture_descriptor("atlas", PixelSize::new(4, 4), 2);
        let mut cache = ResourceCache::new();

        let full = cache.apply_update(&full_update(first_descriptor.clone()));
        assert_eq!(full.outcome, ResourceLifecycleOutcome::Full);
        assert_eq!(full.bytes_uploaded, 64);
        assert_eq!(cache.total_bytes(), 64);

        cache.advance_frame();
        let partial = ResourceUpdate::partial(
            next_descriptor.clone(),
            PixelRect::new(1, 1, 2, 3),
            vec![11; 2 * 3 * 4],
        );
        let partial_report = cache.apply_update(&partial);

        assert_eq!(partial_report.outcome, ResourceLifecycleOutcome::Partial);
        assert_eq!(
            partial_report.update_kind,
            Some(ResourceUpdateKind::Partial)
        );
        assert_eq!(partial_report.bytes_uploaded, 24);
        assert_eq!(partial_report.resource_bytes, Some(64));
        assert_eq!(cache.total_bytes(), 64);

        let entry = cache.entry(&next_descriptor.handle).expect("atlas entry");
        assert_eq!(entry.bytes, 64);
        assert_eq!(entry.upload_count, 2);
        assert_eq!(entry.full_upload_count, 1);
        assert_eq!(entry.partial_upload_count, 1);
        assert_eq!(entry.full_uploaded_bytes, 64);
        assert_eq!(entry.partial_uploaded_bytes, 24);
        assert_eq!(entry.uploaded_bytes, 88);
    }

    #[test]
    fn eviction_plans_by_byte_budget_are_lru_and_deterministic() {
        let mut cache = ResourceCache::new();
        let alpha = texture_descriptor("alpha", PixelSize::new(4, 4), 1);
        let beta = texture_descriptor("beta", PixelSize::new(2, 2), 1);
        let gamma = texture_descriptor("gamma", PixelSize::new(4, 2), 1);

        cache.apply_update(&full_update(alpha.clone()));
        cache.advance_frame();
        cache.apply_update(&full_update(beta.clone()));
        cache.advance_frame();
        cache.apply_update(&full_update(gamma.clone()));
        cache.advance_frame();
        cache.retain(&alpha.handle);

        let plan = cache.plan_eviction_by_byte_budget(96);

        assert_eq!(plan.total_bytes_before, 112);
        assert_eq!(plan.bytes_to_free, 16);
        assert_eq!(plan.total_bytes_after, 96);
        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].handle, beta.handle);
        assert_eq!(
            plan.candidates[0].reasons,
            vec![ResourceEvictionReason::ByteBudget]
        );

        let report = cache.evict_planned(&plan);
        assert_eq!(report.evicted_count(), 1);
        assert_eq!(report.evicted[0].outcome, ResourceLifecycleOutcome::Evicted);
        assert!(!cache.contains(&beta.handle));
        assert_eq!(cache.total_bytes(), 96);
    }

    #[test]
    fn eviction_plans_by_age_keep_recently_retained_resources() {
        let mut cache = ResourceCache::new();
        let old = texture_descriptor("old", PixelSize::new(2, 2), 1);
        let recent = texture_descriptor("recent", PixelSize::new(2, 2), 1);

        cache.apply_update(&full_update(old.clone()));
        cache.advance_frame();
        cache.apply_update(&full_update(recent.clone()));
        cache.advance_frame();
        cache.advance_frame();
        cache.retain(&recent.handle);
        cache.advance_frame();

        let plan = cache.plan_eviction_by_age(2);

        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].handle, old.handle);
        assert_eq!(plan.candidates[0].age_frames, 4);
        assert_eq!(
            plan.candidates[0].reasons,
            vec![ResourceEvictionReason::Age]
        );
    }

    #[test]
    fn retained_reports_update_last_used_frames_without_uploading() {
        let descriptor = image_descriptor("stable", PixelSize::new(2, 2), 5);
        let update = full_update(descriptor.clone());
        let mut cache = ResourceCache::new();

        let first = cache.apply_update(&update);
        assert_eq!(first.frame, 0);
        assert_eq!(first.created_frame, Some(0));
        assert_eq!(first.last_used_frame, Some(0));

        cache.advance_frame();
        let retained = cache.apply_update(&update);
        assert_eq!(retained.outcome, ResourceLifecycleOutcome::Retained);
        assert_eq!(retained.bytes_uploaded, 0);
        assert_eq!(retained.frame, 1);
        assert_eq!(retained.created_frame, Some(0));
        assert_eq!(retained.last_used_frame, Some(1));

        cache.advance_frame();
        let touched = cache.retain(&descriptor.handle);
        assert_eq!(touched.outcome, ResourceLifecycleOutcome::Retained);
        assert_eq!(touched.frame, 2);
        assert_eq!(cache.entry(&descriptor.handle).unwrap().last_used_frame, 2);
        assert_eq!(cache.entry(&descriptor.handle).unwrap().retained_count, 2);
    }
}
