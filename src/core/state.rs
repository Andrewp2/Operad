//! Backend-neutral retained widget state and lifecycle reconciliation.
//!
//! The store in this module is intentionally app-agnostic. Applications provide
//! stable widget IDs and scopes, while widgets describe typed state slots that
//! can survive document rebuilds without coupling to a renderer or host.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::{
    AnimationState, DirtyFlags, RuntimeInvalidation, RuntimeInvalidationReason, RuntimeWindowId,
    ScrollState,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(String);

impl WidgetId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for WidgetId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for WidgetId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WidgetId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&WidgetId> for WidgetId {
    fn from(value: &WidgetId) -> Self {
        value.clone()
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetStateSlotId(String);

impl WidgetStateSlotId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for WidgetStateSlotId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WidgetStateSlotId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for WidgetStateSlotId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidgetStateScope {
    Document(String),
    Window(RuntimeWindowId),
    Custom(String),
}

impl WidgetStateScope {
    pub fn document(id: impl Into<String>) -> Self {
        Self::Document(id.into())
    }

    pub fn window(id: impl Into<RuntimeWindowId>) -> Self {
        Self::Window(id.into())
    }

    pub fn custom(id: impl Into<String>) -> Self {
        Self::Custom(id.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetKey {
    pub scope: WidgetStateScope,
    pub id: WidgetId,
}

impl WidgetKey {
    pub fn new(scope: WidgetStateScope, id: impl Into<WidgetId>) -> Self {
        Self {
            scope,
            id: id.into(),
        }
    }

    pub fn document(document: impl Into<String>, id: impl Into<WidgetId>) -> Self {
        Self::new(WidgetStateScope::document(document), id)
    }

    pub fn window(window: impl Into<RuntimeWindowId>, id: impl Into<WidgetId>) -> Self {
        Self::new(WidgetStateScope::window(window), id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetStateKey {
    pub widget: WidgetKey,
    pub slot: WidgetStateSlotId,
}

impl WidgetStateKey {
    pub fn new(widget: WidgetKey, slot: impl Into<WidgetStateSlotId>) -> Self {
        Self {
            widget,
            slot: slot.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidgetStateSlotKind {
    Focus,
    Hover,
    Pressed,
    Overlay,
    Scroll,
    Animation,
    Edit,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetStateSlotDescriptor {
    pub id: WidgetStateSlotId,
    pub kind: WidgetStateSlotKind,
    pub dirty_flags: DirtyFlags,
    pub invalidation_reason: RuntimeInvalidationReason,
}

impl WidgetStateSlotDescriptor {
    pub fn new(id: impl Into<WidgetStateSlotId>, kind: WidgetStateSlotKind) -> Self {
        let kind_for_flags = kind.clone();
        Self {
            id: id.into(),
            kind,
            dirty_flags: dirty_flags_for_slot_kind(&kind_for_flags),
            invalidation_reason: invalidation_reason_for_slot_kind(&kind_for_flags),
        }
    }

    pub fn focus() -> Self {
        Self::new("focus", WidgetStateSlotKind::Focus)
    }

    pub fn hover() -> Self {
        Self::new("hover", WidgetStateSlotKind::Hover)
    }

    pub fn pressed() -> Self {
        Self::new("pressed", WidgetStateSlotKind::Pressed)
    }

    pub fn overlay() -> Self {
        Self::new("overlay", WidgetStateSlotKind::Overlay)
    }

    pub fn scroll() -> Self {
        Self::new("scroll", WidgetStateSlotKind::Scroll)
    }

    pub fn animation() -> Self {
        Self::new("animation", WidgetStateSlotKind::Animation)
    }

    pub fn edit() -> Self {
        Self::new("edit", WidgetStateSlotKind::Edit)
    }

    pub fn custom(id: impl Into<WidgetStateSlotId>, kind: impl Into<String>) -> Self {
        Self::new(id, WidgetStateSlotKind::Custom(kind.into()))
    }

    pub const fn dirty_flags(mut self, dirty_flags: DirtyFlags) -> Self {
        self.dirty_flags = dirty_flags;
        self
    }

    pub const fn invalidation_reason(mut self, reason: RuntimeInvalidationReason) -> Self {
        self.invalidation_reason = reason;
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WidgetFocusState {
    pub focused: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WidgetHoverState {
    pub hovered: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WidgetPressedState {
    pub pressed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WidgetOverlayState {
    pub open: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WidgetEditState {
    pub text: String,
    pub cursor_byte_index: usize,
    pub selection_anchor_byte_index: Option<usize>,
    pub composing: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetStateValue {
    Focus(WidgetFocusState),
    Hover(WidgetHoverState),
    Pressed(WidgetPressedState),
    Overlay(WidgetOverlayState),
    Scroll(ScrollState),
    Animation(AnimationState),
    Edit(WidgetEditState),
    Custom { kind: String, revision: u64 },
}

impl WidgetStateValue {
    pub const fn focus(focused: bool) -> Self {
        Self::Focus(WidgetFocusState { focused })
    }

    pub const fn hover(hovered: bool) -> Self {
        Self::Hover(WidgetHoverState { hovered })
    }

    pub const fn pressed(pressed: bool) -> Self {
        Self::Pressed(WidgetPressedState { pressed })
    }

    pub const fn overlay(open: bool) -> Self {
        Self::Overlay(WidgetOverlayState { open })
    }

    pub fn edit(text: impl Into<String>, cursor_byte_index: usize) -> Self {
        Self::Edit(WidgetEditState {
            text: text.into(),
            cursor_byte_index,
            selection_anchor_byte_index: None,
            composing: false,
        })
    }

    pub fn custom(kind: impl Into<String>, revision: u64) -> Self {
        Self::Custom {
            kind: kind.into(),
            revision,
        }
    }

    pub fn kind(&self) -> WidgetStateSlotKind {
        match self {
            Self::Focus(_) => WidgetStateSlotKind::Focus,
            Self::Hover(_) => WidgetStateSlotKind::Hover,
            Self::Pressed(_) => WidgetStateSlotKind::Pressed,
            Self::Overlay(_) => WidgetStateSlotKind::Overlay,
            Self::Scroll(_) => WidgetStateSlotKind::Scroll,
            Self::Animation(_) => WidgetStateSlotKind::Animation,
            Self::Edit(_) => WidgetStateSlotKind::Edit,
            Self::Custom { kind, .. } => WidgetStateSlotKind::Custom(kind.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetainedWidgetStateEntry {
    pub key: WidgetStateKey,
    pub descriptor: WidgetStateSlotDescriptor,
    pub value: WidgetStateValue,
    pub created_generation: u64,
    pub updated_generation: u64,
    pub last_seen_generation: u64,
    pub stale_since_generation: Option<u64>,
    pub keepalive: bool,
    pub revision: u64,
}

impl RetainedWidgetStateEntry {
    fn new(
        key: WidgetStateKey,
        descriptor: WidgetStateSlotDescriptor,
        value: WidgetStateValue,
        generation: u64,
    ) -> Self {
        Self {
            key,
            descriptor,
            value,
            created_generation: generation,
            updated_generation: generation,
            last_seen_generation: generation,
            stale_since_generation: None,
            keepalive: false,
            revision: 0,
        }
    }

    pub const fn is_stale(&self) -> bool {
        self.stale_since_generation.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetStateRetention {
    pub max_stale_generations: u64,
    pub max_entry_age_generations: Option<u64>,
}

impl WidgetStateRetention {
    pub const fn new(max_stale_generations: u64) -> Self {
        Self {
            max_stale_generations,
            max_entry_age_generations: None,
        }
    }

    pub const fn with_max_entry_age_generations(mut self, max_age: u64) -> Self {
        self.max_entry_age_generations = Some(max_age);
        self
    }
}

impl Default for WidgetStateRetention {
    fn default() -> Self {
        Self::new(2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetStateLifecycleOutcome {
    Created,
    Reused,
    Expired,
    Orphaned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetStateLifecycleItem {
    pub key: WidgetStateKey,
    pub outcome: WidgetStateLifecycleOutcome,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WidgetStateLifecycleReport {
    pub generation: u64,
    pub created: Vec<WidgetStateKey>,
    pub reused: Vec<WidgetStateKey>,
    pub expired: Vec<WidgetStateKey>,
    pub orphaned: Vec<WidgetStateKey>,
}

impl WidgetStateLifecycleReport {
    fn push(&mut self, key: WidgetStateKey, outcome: WidgetStateLifecycleOutcome) {
        match outcome {
            WidgetStateLifecycleOutcome::Created => self.created.push(key),
            WidgetStateLifecycleOutcome::Reused => self.reused.push(key),
            WidgetStateLifecycleOutcome::Expired => self.expired.push(key),
            WidgetStateLifecycleOutcome::Orphaned => self.orphaned.push(key),
        }
    }

    pub fn items(&self) -> Vec<WidgetStateLifecycleItem> {
        self.created
            .iter()
            .cloned()
            .map(|key| WidgetStateLifecycleItem {
                key,
                outcome: WidgetStateLifecycleOutcome::Created,
            })
            .chain(
                self.reused
                    .iter()
                    .cloned()
                    .map(|key| WidgetStateLifecycleItem {
                        key,
                        outcome: WidgetStateLifecycleOutcome::Reused,
                    }),
            )
            .chain(
                self.expired
                    .iter()
                    .cloned()
                    .map(|key| WidgetStateLifecycleItem {
                        key,
                        outcome: WidgetStateLifecycleOutcome::Expired,
                    }),
            )
            .chain(
                self.orphaned
                    .iter()
                    .cloned()
                    .map(|key| WidgetStateLifecycleItem {
                        key,
                        outcome: WidgetStateLifecycleOutcome::Orphaned,
                    }),
            )
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetStateError {
    SlotKindMismatch {
        key: Box<WidgetStateKey>,
        expected: WidgetStateSlotKind,
        actual: WidgetStateSlotKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetStateUpdateReport {
    pub key: WidgetStateKey,
    pub changed: bool,
    pub created: bool,
    pub dirty_flags: DirtyFlags,
    pub invalidations: Vec<RuntimeInvalidation>,
}

impl WidgetStateUpdateReport {
    fn unchanged(key: WidgetStateKey, created: bool) -> Self {
        Self {
            key,
            changed: false,
            created,
            dirty_flags: DirtyFlags::NONE,
            invalidations: Vec::new(),
        }
    }

    fn changed(key: WidgetStateKey, created: bool, descriptor: &WidgetStateSlotDescriptor) -> Self {
        let detail = format!(
            "widget state changed: widget={} slot={}",
            key.widget.id, key.slot
        );
        Self {
            key,
            changed: true,
            created,
            dirty_flags: descriptor.dirty_flags,
            invalidations: vec![
                RuntimeInvalidation::new(descriptor.invalidation_reason).detail(detail)
            ],
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WidgetStateInvalidationSummary {
    pub dirty_flags: DirtyFlags,
    pub invalidations: Vec<RuntimeInvalidation>,
}

impl WidgetStateInvalidationSummary {
    pub fn push(&mut self, report: &WidgetStateUpdateReport) {
        self.dirty_flags = self.dirty_flags.union(report.dirty_flags);
        self.invalidations.extend(report.invalidations.clone());
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetainedWidgetStateStore {
    generation: u64,
    retention: WidgetStateRetention,
    entries: HashMap<WidgetStateKey, RetainedWidgetStateEntry>,
    last_lifecycle_report: WidgetStateLifecycleReport,
}

impl RetainedWidgetStateStore {
    pub fn new(retention: WidgetStateRetention) -> Self {
        Self {
            generation: 0,
            retention,
            entries: HashMap::new(),
            last_lifecycle_report: WidgetStateLifecycleReport::default(),
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn retention(&self) -> WidgetStateRetention {
        self.retention
    }

    pub fn set_retention(&mut self, retention: WidgetStateRetention) {
        self.retention = retention;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> impl Iterator<Item = &RetainedWidgetStateEntry> {
        self.entries.values()
    }

    pub fn last_lifecycle_report(&self) -> &WidgetStateLifecycleReport {
        &self.last_lifecycle_report
    }

    pub fn begin_frame(
        &mut self,
        seen_widgets: impl IntoIterator<Item = WidgetKey>,
    ) -> WidgetStateLifecycleReport {
        self.generation = self.generation.wrapping_add(1);
        let seen_widgets: HashSet<WidgetKey> = seen_widgets.into_iter().collect();
        let mut report = WidgetStateLifecycleReport {
            generation: self.generation,
            ..WidgetStateLifecycleReport::default()
        };
        let mut expired = Vec::new();

        for (key, entry) in &mut self.entries {
            if seen_widgets.contains(&key.widget) {
                entry.last_seen_generation = self.generation;
                entry.stale_since_generation = None;
                report.push(key.clone(), WidgetStateLifecycleOutcome::Reused);
                continue;
            }

            if entry.stale_since_generation.is_none() {
                entry.stale_since_generation = Some(self.generation);
                report.push(key.clone(), WidgetStateLifecycleOutcome::Orphaned);
            }

            if !entry.keepalive && entry_expired(entry, self.generation, self.retention) {
                expired.push(key.clone());
            }
        }

        for key in expired {
            self.entries.remove(&key);
            report.push(key, WidgetStateLifecycleOutcome::Expired);
        }

        self.last_lifecycle_report = report.clone();
        report
    }

    pub fn get(&self, key: &WidgetStateKey) -> Option<&RetainedWidgetStateEntry> {
        self.entries.get(key)
    }

    pub fn value(
        &self,
        key: &WidgetStateKey,
        descriptor: &WidgetStateSlotDescriptor,
    ) -> Result<Option<&WidgetStateValue>, WidgetStateError> {
        let Some(entry) = self.entries.get(key) else {
            return Ok(None);
        };
        validate_slot_kind(key, &entry.descriptor.kind, &descriptor.kind)?;
        Ok(Some(&entry.value))
    }

    pub fn set_keepalive(&mut self, key: &WidgetStateKey, keepalive: bool) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.keepalive = keepalive;
            true
        } else {
            false
        }
    }

    pub fn ensure(
        &mut self,
        widget: WidgetKey,
        descriptor: WidgetStateSlotDescriptor,
        default_value: WidgetStateValue,
    ) -> Result<WidgetStateUpdateReport, WidgetStateError> {
        validate_slot_kind_for_value(
            &WidgetStateKey::new(widget.clone(), descriptor.id.clone()),
            &descriptor.kind,
            &default_value,
        )?;

        let key = WidgetStateKey::new(widget, descriptor.id.clone());
        let Some(entry) = self.entries.get_mut(&key) else {
            let entry = RetainedWidgetStateEntry::new(
                key.clone(),
                descriptor.clone(),
                default_value,
                self.generation,
            );
            self.entries.insert(key.clone(), entry);
            self.last_lifecycle_report
                .push(key.clone(), WidgetStateLifecycleOutcome::Created);
            return Ok(WidgetStateUpdateReport::changed(key, true, &descriptor));
        };

        validate_slot_kind(&key, &entry.descriptor.kind, &descriptor.kind)?;
        entry.last_seen_generation = self.generation;
        entry.stale_since_generation = None;
        Ok(WidgetStateUpdateReport::unchanged(key, false))
    }

    pub fn set(
        &mut self,
        widget: WidgetKey,
        descriptor: WidgetStateSlotDescriptor,
        value: WidgetStateValue,
    ) -> Result<WidgetStateUpdateReport, WidgetStateError> {
        validate_slot_kind_for_value(
            &WidgetStateKey::new(widget.clone(), descriptor.id.clone()),
            &descriptor.kind,
            &value,
        )?;

        let key = WidgetStateKey::new(widget, descriptor.id.clone());
        let Some(entry) = self.entries.get_mut(&key) else {
            let entry = RetainedWidgetStateEntry::new(
                key.clone(),
                descriptor.clone(),
                value,
                self.generation,
            );
            self.entries.insert(key.clone(), entry);
            self.last_lifecycle_report
                .push(key.clone(), WidgetStateLifecycleOutcome::Created);
            return Ok(WidgetStateUpdateReport::changed(key, true, &descriptor));
        };

        validate_slot_kind(&key, &entry.descriptor.kind, &descriptor.kind)?;

        entry.last_seen_generation = self.generation;
        entry.stale_since_generation = None;
        if entry.value == value {
            return Ok(WidgetStateUpdateReport::unchanged(key, false));
        }

        entry.value = value;
        entry.descriptor = descriptor.clone();
        entry.updated_generation = self.generation;
        entry.revision = entry.revision.saturating_add(1);
        Ok(WidgetStateUpdateReport::changed(key, false, &descriptor))
    }

    pub fn invalidation_summary<'a>(
        reports: impl IntoIterator<Item = &'a WidgetStateUpdateReport>,
    ) -> WidgetStateInvalidationSummary {
        let mut summary = WidgetStateInvalidationSummary::default();
        for report in reports {
            summary.push(report);
        }
        summary
    }
}

impl Default for RetainedWidgetStateStore {
    fn default() -> Self {
        Self::new(WidgetStateRetention::default())
    }
}

fn dirty_flags_for_slot_kind(kind: &WidgetStateSlotKind) -> DirtyFlags {
    match kind {
        WidgetStateSlotKind::Focus
        | WidgetStateSlotKind::Hover
        | WidgetStateSlotKind::Pressed
        | WidgetStateSlotKind::Overlay
        | WidgetStateSlotKind::Scroll
        | WidgetStateSlotKind::Edit => DirtyFlags {
            input: true,
            paint: true,
            ..DirtyFlags::NONE
        },
        WidgetStateSlotKind::Animation => DirtyFlags {
            paint: true,
            ..DirtyFlags::NONE
        },
        WidgetStateSlotKind::Custom(_) => DirtyFlags {
            layout: true,
            paint: true,
            ..DirtyFlags::NONE
        },
    }
}

fn invalidation_reason_for_slot_kind(kind: &WidgetStateSlotKind) -> RuntimeInvalidationReason {
    match kind {
        WidgetStateSlotKind::Animation => RuntimeInvalidationReason::Animation,
        WidgetStateSlotKind::Custom(_) => RuntimeInvalidationReason::Explicit,
        _ => RuntimeInvalidationReason::Input,
    }
}

fn entry_expired(
    entry: &RetainedWidgetStateEntry,
    generation: u64,
    retention: WidgetStateRetention,
) -> bool {
    let stale_expired = entry.stale_since_generation.is_some_and(|stale_since| {
        generation.saturating_sub(stale_since) > retention.max_stale_generations
    });
    let age_expired = retention
        .max_entry_age_generations
        .is_some_and(|max_age| generation.saturating_sub(entry.created_generation) > max_age);
    stale_expired || age_expired
}

fn validate_slot_kind(
    key: &WidgetStateKey,
    expected: &WidgetStateSlotKind,
    actual: &WidgetStateSlotKind,
) -> Result<(), WidgetStateError> {
    if expected == actual {
        Ok(())
    } else {
        Err(WidgetStateError::SlotKindMismatch {
            key: Box::new(key.clone()),
            expected: expected.clone(),
            actual: actual.clone(),
        })
    }
}

fn validate_slot_kind_for_value(
    key: &WidgetStateKey,
    expected: &WidgetStateSlotKind,
    value: &WidgetStateValue,
) -> Result<(), WidgetStateError> {
    validate_slot_kind(key, expected, &value.kind())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RuntimeWindowId, ScrollAxes, UiPoint};

    fn widget(id: &str) -> WidgetKey {
        WidgetKey::document("doc-a", id)
    }

    fn focus_key(id: &str) -> WidgetStateKey {
        WidgetStateKey::new(widget(id), "focus")
    }

    #[test]
    fn state_preserves_value_across_document_rebuilds() {
        let mut store = RetainedWidgetStateStore::default();
        let field = widget("field");

        store.begin_frame([field.clone()]);
        store
            .set(
                field.clone(),
                WidgetStateSlotDescriptor::focus(),
                WidgetStateValue::focus(true),
            )
            .unwrap();
        let ensured = store
            .ensure(
                field.clone(),
                WidgetStateSlotDescriptor::focus(),
                WidgetStateValue::focus(false),
            )
            .unwrap();
        assert!(!ensured.changed);

        let report = store.begin_frame([field]);
        assert_eq!(report.reused, vec![focus_key("field")]);
        assert_eq!(
            store.get(&focus_key("field")).unwrap().value,
            WidgetStateValue::focus(true)
        );
    }

    #[test]
    fn removed_widgets_are_orphaned_then_expired() {
        let mut store = RetainedWidgetStateStore::new(WidgetStateRetention::new(1));
        let field = widget("field");

        store.begin_frame([field.clone()]);
        store
            .set(
                field,
                WidgetStateSlotDescriptor::hover(),
                WidgetStateValue::hover(true),
            )
            .unwrap();

        let hover_key = WidgetStateKey::new(widget("field"), "hover");
        let orphaned = store.begin_frame([]);
        assert_eq!(orphaned.orphaned, vec![hover_key.clone()]);
        assert!(store.get(&hover_key).unwrap().is_stale());

        store.begin_frame([]);
        let expired = store.begin_frame([]);
        assert_eq!(expired.expired, vec![hover_key.clone()]);
        assert!(store.get(&hover_key).is_none());
    }

    #[test]
    fn keepalive_entries_survive_missing_frames() {
        let mut store = RetainedWidgetStateStore::new(WidgetStateRetention::new(0));
        let popover = widget("popover");
        let overlay_key = WidgetStateKey::new(popover.clone(), "overlay");

        store.begin_frame([popover.clone()]);
        store
            .set(
                popover,
                WidgetStateSlotDescriptor::overlay(),
                WidgetStateValue::overlay(true),
            )
            .unwrap();
        assert!(store.set_keepalive(&overlay_key, true));

        let orphaned = store.begin_frame([]);
        assert_eq!(orphaned.orphaned, vec![overlay_key.clone()]);
        let expired = store.begin_frame([]);
        assert!(expired.expired.is_empty());
        assert_eq!(
            store.get(&overlay_key).unwrap().value,
            WidgetStateValue::overlay(true)
        );
    }

    #[test]
    fn scopes_isolate_matching_widget_ids() {
        let mut store = RetainedWidgetStateStore::default();
        let document_widget = WidgetKey::document("doc-a", "shared");
        let window_widget = WidgetKey::window(RuntimeWindowId::new("main"), "shared");

        store.begin_frame([document_widget.clone(), window_widget.clone()]);
        store
            .set(
                document_widget.clone(),
                WidgetStateSlotDescriptor::scroll(),
                WidgetStateValue::Scroll(ScrollState {
                    axes: ScrollAxes::VERTICAL,
                    offset: UiPoint::new(0.0, 24.0),
                    viewport_size: crate::UiSize::new(100.0, 100.0),
                    content_size: crate::UiSize::new(100.0, 500.0),
                }),
            )
            .unwrap();
        store
            .set(
                window_widget.clone(),
                WidgetStateSlotDescriptor::scroll(),
                WidgetStateValue::Scroll(ScrollState {
                    axes: ScrollAxes::VERTICAL,
                    offset: UiPoint::new(0.0, 88.0),
                    viewport_size: crate::UiSize::new(100.0, 100.0),
                    content_size: crate::UiSize::new(100.0, 500.0),
                }),
            )
            .unwrap();

        let document_key = WidgetStateKey::new(document_widget, "scroll");
        let window_key = WidgetStateKey::new(window_widget, "scroll");
        assert_ne!(
            store.get(&document_key).unwrap().value,
            store.get(&window_key).unwrap().value
        );
    }

    #[test]
    fn typed_slot_mismatch_is_rejected() {
        let mut store = RetainedWidgetStateStore::default();
        let field = widget("field");
        let key = focus_key("field");

        store.begin_frame([field.clone()]);
        store
            .set(
                field.clone(),
                WidgetStateSlotDescriptor::focus(),
                WidgetStateValue::focus(true),
            )
            .unwrap();

        let err = store
            .set(
                field,
                WidgetStateSlotDescriptor::focus(),
                WidgetStateValue::hover(true),
            )
            .unwrap_err();
        assert_eq!(
            err,
            WidgetStateError::SlotKindMismatch {
                key: Box::new(key.clone()),
                expected: WidgetStateSlotKind::Focus,
                actual: WidgetStateSlotKind::Hover,
            }
        );
        assert_eq!(
            store.get(&key).unwrap().value,
            WidgetStateValue::focus(true)
        );
    }

    #[test]
    fn changed_state_reports_dirty_flags_and_runtime_invalidation() {
        let mut store = RetainedWidgetStateStore::default();
        let button = widget("button");

        store.begin_frame([button.clone()]);
        let first = store
            .set(
                button.clone(),
                WidgetStateSlotDescriptor::pressed(),
                WidgetStateValue::pressed(false),
            )
            .unwrap();
        let second = store
            .set(
                button.clone(),
                WidgetStateSlotDescriptor::pressed(),
                WidgetStateValue::pressed(true),
            )
            .unwrap();
        let unchanged = store
            .set(
                button,
                WidgetStateSlotDescriptor::pressed(),
                WidgetStateValue::pressed(true),
            )
            .unwrap();

        assert!(first.created);
        assert!(second.changed);
        assert!(second.dirty_flags.input);
        assert!(second.dirty_flags.paint);
        assert_eq!(
            second.invalidations[0].reason,
            RuntimeInvalidationReason::Input
        );
        assert!(!unchanged.changed);
        assert!(unchanged.invalidations.is_empty());

        let summary = RetainedWidgetStateStore::invalidation_summary([&second, &unchanged]);
        assert!(summary.dirty_flags.input);
        assert_eq!(summary.invalidations.len(), 1);
    }
}
