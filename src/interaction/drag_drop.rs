//! Renderer-neutral drag/drop surface metadata.
//!
//! Platform adapters own the OS drag loop through [`crate::platform`]. This
//! module gives widgets and editor surfaces stable source/target descriptions
//! they can expose to hit testing, accessibility, and platform-output routing.

use crate::{
    platform::{DragDropRequest, DragId, DragImage, DragOperation, DragPayload, LogicalPoint},
    AccessibilityAction, AccessibilityMeta, AccessibilityRole, AccessibilitySummary, UiPoint,
    UiRect,
};

const ALL_OPERATIONS: [DragOperation; 3] = [
    DragOperation::Copy,
    DragOperation::Move,
    DragOperation::Link,
];

/// Stable id for a draggable UI element.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DragSourceId(pub String);

impl DragSourceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl From<&str> for DragSourceId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for DragSourceId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable id for a drop target.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DropTargetId(pub String);

impl DropTargetId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl From<&str> for DropTargetId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for DropTargetId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Common drag/drop surface roles used by app shells and editor widgets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DragDropSurfaceKind {
    Asset,
    Button,
    DockPanel,
    DockTarget,
    ListRow,
    TableRow,
    TreeItem,
    EditorSurface,
    EditorLane,
    EditorRangeItem,
    Canvas,
    Custom(String),
}

impl DragDropSurfaceKind {
    pub const fn accessibility_role(&self) -> AccessibilityRole {
        match self {
            Self::Button => AccessibilityRole::Button,
            Self::ListRow | Self::Asset => AccessibilityRole::ListItem,
            Self::TableRow => AccessibilityRole::Row,
            Self::TreeItem => AccessibilityRole::TreeItem,
            Self::EditorSurface => AccessibilityRole::EditorSurface,
            Self::DockPanel
            | Self::DockTarget
            | Self::EditorLane
            | Self::EditorRangeItem
            | Self::Canvas
            | Self::Custom(_) => AccessibilityRole::Group,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Asset => "Asset",
            Self::Button => "Button",
            Self::DockPanel => "Dock panel",
            Self::DockTarget => "Dock target",
            Self::ListRow => "List row",
            Self::TableRow => "Table row",
            Self::TreeItem => "Tree item",
            Self::EditorSurface => "Editor surface",
            Self::EditorLane => "Editor lane",
            Self::EditorRangeItem => "Editor range item",
            Self::Canvas => "Canvas",
            Self::Custom(label) => label.as_str(),
        }
    }
}

/// Payload acceptance policy for a drop target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DropPayloadFilter {
    pub text: bool,
    pub files: bool,
    pub mime_types: Vec<String>,
}

impl DropPayloadFilter {
    pub const fn empty() -> Self {
        Self {
            text: false,
            files: false,
            mime_types: Vec::new(),
        }
    }

    pub fn any() -> Self {
        Self {
            text: true,
            files: true,
            mime_types: vec!["*/*".to_string()],
        }
    }

    pub const fn text(mut self) -> Self {
        self.text = true;
        self
    }

    pub const fn files(mut self) -> Self {
        self.files = true;
        self
    }

    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_types.push(mime_type.into());
        self
    }

    pub fn any_bytes(self) -> Self {
        self.mime_type("*/*")
    }

    pub fn accepts_payload(&self, payload: &DragPayload) -> bool {
        (self.text && payload.text.is_some())
            || (self.files && !payload.files.is_empty())
            || (!self.mime_types.is_empty()
                && payload.bytes.iter().any(|bytes| {
                    self.mime_types
                        .iter()
                        .any(|pattern| mime_type_matches(pattern, &bytes.mime_type))
                }))
    }

    pub fn is_empty(&self) -> bool {
        !self.text && !self.files && self.mime_types.is_empty()
    }
}

/// Metadata for a UI element that can start a platform drag operation.
#[derive(Debug, Clone, PartialEq)]
pub struct DragSourceDescriptor {
    pub id: DragSourceId,
    pub platform_id: DragId,
    pub kind: DragDropSurfaceKind,
    pub bounds: UiRect,
    pub payload: DragPayload,
    pub allowed_operations: Vec<DragOperation>,
    pub drag_image: Option<DragImage>,
    pub disabled: bool,
    pub label: Option<String>,
    pub hint: Option<String>,
}

impl DragSourceDescriptor {
    pub fn new(
        id: impl Into<DragSourceId>,
        kind: DragDropSurfaceKind,
        bounds: UiRect,
        payload: DragPayload,
    ) -> Self {
        let id = id.into();
        Self {
            platform_id: DragId::new(id.0.clone()),
            id,
            kind,
            bounds,
            payload,
            allowed_operations: ALL_OPERATIONS.to_vec(),
            drag_image: None,
            disabled: false,
            label: None,
            hint: None,
        }
    }

    pub fn platform_id(mut self, platform_id: impl Into<String>) -> Self {
        self.platform_id = DragId::new(platform_id);
        self
    }

    pub fn allowed_operations(
        mut self,
        operations: impl IntoIterator<Item = DragOperation>,
    ) -> Self {
        self.allowed_operations = operations.into_iter().collect();
        self
    }

    pub fn drag_image(mut self, drag_image: impl Into<Option<DragImage>>) -> Self {
        self.drag_image = drag_image.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn can_start(&self) -> bool {
        !self.disabled && payload_has_content(&self.payload) && !self.allowed_operations.is_empty()
    }

    pub fn contains_point(&self, point: UiPoint) -> bool {
        self.bounds.contains_point(point)
    }

    pub fn start_request(&self, origin: UiPoint) -> Option<DragDropRequest> {
        self.can_start().then(|| DragDropRequest::Start {
            id: self.platform_id.clone(),
            payload: self.payload.clone(),
            origin: LogicalPoint::new(origin.x, origin.y),
            allowed_operations: self.allowed_operations.clone(),
            drag_image: self.drag_image.clone(),
        })
    }

    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        let label = self
            .label
            .clone()
            .unwrap_or_else(|| self.kind.label().to_string());
        let mut meta = AccessibilityMeta::new(self.kind.accessibility_role())
            .label(label.clone())
            .hint(
                self.hint
                    .clone()
                    .unwrap_or_else(|| "Draggable item".to_string()),
            )
            .summary(
                AccessibilitySummary::new(label)
                    .item("Kind", self.kind.label())
                    .item("Operations", operation_list_text(&self.allowed_operations)),
            )
            .action(AccessibilityAction::new("drag.start", "Start drag"));
        if self.disabled {
            meta = meta.disabled();
        }
        meta
    }
}

/// Metadata for a UI element that can accept a dragged payload.
#[derive(Debug, Clone, PartialEq)]
pub struct DropTargetDescriptor {
    pub id: DropTargetId,
    pub kind: DragDropSurfaceKind,
    pub bounds: UiRect,
    pub z_index: i16,
    pub accepted_payload: DropPayloadFilter,
    pub accepted_operations: Vec<DragOperation>,
    pub disabled: bool,
    pub label: Option<String>,
    pub hint: Option<String>,
}

impl DropTargetDescriptor {
    pub fn new(id: impl Into<DropTargetId>, kind: DragDropSurfaceKind, bounds: UiRect) -> Self {
        Self {
            id: id.into(),
            kind,
            bounds,
            z_index: 0,
            accepted_payload: DropPayloadFilter::empty(),
            accepted_operations: ALL_OPERATIONS.to_vec(),
            disabled: false,
            label: None,
            hint: None,
        }
    }

    pub const fn z_index(mut self, z_index: i16) -> Self {
        self.z_index = z_index;
        self
    }

    pub fn accepted_payload(mut self, accepted_payload: DropPayloadFilter) -> Self {
        self.accepted_payload = accepted_payload;
        self
    }

    pub fn accept_text(mut self) -> Self {
        self.accepted_payload = self.accepted_payload.text();
        self
    }

    pub fn accept_files(mut self) -> Self {
        self.accepted_payload = self.accepted_payload.files();
        self
    }

    pub fn accept_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.accepted_payload = self.accepted_payload.mime_type(mime_type);
        self
    }

    pub fn accepted_operations(
        mut self,
        operations: impl IntoIterator<Item = DragOperation>,
    ) -> Self {
        self.accepted_operations = operations.into_iter().collect();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn contains_point(&self, point: UiPoint) -> bool {
        self.bounds.contains_point(point)
    }

    pub fn accepts_payload(&self, payload: &DragPayload) -> bool {
        self.accepted_payload.accepts_payload(payload)
    }

    pub fn resolve_operation(
        &self,
        payload: &DragPayload,
        source_operations: &[DragOperation],
    ) -> Option<DragOperation> {
        if self.disabled || !self.accepts_payload(payload) {
            return None;
        }
        self.accepted_operations
            .iter()
            .copied()
            .find(|operation| source_operations.contains(operation))
    }

    pub fn hit_test(
        targets: &[Self],
        point: UiPoint,
        payload: &DragPayload,
        source_operations: &[DragOperation],
    ) -> Option<DropTargetHit> {
        let mut best: Option<(usize, DropTargetHit)> = None;
        for (index, target) in targets.iter().enumerate() {
            if !target.contains_point(point) {
                continue;
            }
            let Some(operation) = target.resolve_operation(payload, source_operations) else {
                continue;
            };
            let hit = DropTargetHit {
                target_id: target.id.clone(),
                kind: target.kind.clone(),
                local_position: UiPoint::new(point.x - target.bounds.x, point.y - target.bounds.y),
                operation,
                z_index: target.z_index,
            };
            let replaces_best = best.as_ref().is_none_or(|(best_index, best_hit)| {
                hit.z_index > best_hit.z_index
                    || (hit.z_index == best_hit.z_index && index > *best_index)
            });
            if replaces_best {
                best = Some((index, hit));
            }
        }
        best.map(|(_, hit)| hit)
    }

    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        let label = self
            .label
            .clone()
            .unwrap_or_else(|| self.kind.label().to_string());
        let mut meta = AccessibilityMeta::new(self.kind.accessibility_role())
            .label(label.clone())
            .hint(
                self.hint
                    .clone()
                    .unwrap_or_else(|| "Drop target".to_string()),
            )
            .summary(
                AccessibilitySummary::new(label)
                    .item("Kind", self.kind.label())
                    .item("Accepts", payload_filter_text(&self.accepted_payload))
                    .item("Operations", operation_list_text(&self.accepted_operations)),
            )
            .action(AccessibilityAction::new("drop.accept", "Accept drop"));
        if self.disabled {
            meta = meta.disabled();
        }
        meta
    }
}

/// Result of resolving a pointer position against a drop-target set.
#[derive(Debug, Clone, PartialEq)]
pub struct DropTargetHit {
    pub target_id: DropTargetId,
    pub kind: DragDropSurfaceKind,
    pub local_position: UiPoint,
    pub operation: DragOperation,
    pub z_index: i16,
}

pub fn payload_has_content(payload: &DragPayload) -> bool {
    payload.text.as_ref().is_some_and(|text| !text.is_empty())
        || !payload.files.is_empty()
        || !payload.bytes.is_empty()
}

fn payload_filter_text(filter: &DropPayloadFilter) -> String {
    let mut parts = Vec::new();
    if filter.text {
        parts.push("text".to_string());
    }
    if filter.files {
        parts.push("files".to_string());
    }
    parts.extend(filter.mime_types.iter().cloned());
    if parts.is_empty() {
        "nothing".to_string()
    } else {
        parts.join(", ")
    }
}

fn operation_list_text(operations: &[DragOperation]) -> String {
    if operations.is_empty() {
        return "none".to_string();
    }
    operations
        .iter()
        .map(|operation| match operation {
            DragOperation::Copy => "copy",
            DragOperation::Move => "move",
            DragOperation::Link => "link",
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn mime_type_matches(pattern: &str, mime_type: &str) -> bool {
    let pattern = pattern.trim();
    if pattern == "*" || pattern == "*/*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return mime_type
            .split_once('/')
            .is_some_and(|(family, _)| prefix.eq_ignore_ascii_case(family));
    }
    pattern.eq_ignore_ascii_case(mime_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::DragBytes;

    #[test]
    fn payload_filters_match_text_files_and_mime_wildcards() {
        let text = DragPayload::text("clip");
        let files = DragPayload::files(["clip.wav"]);
        let image = DragPayload::bytes(DragBytes::new("image/png", vec![1, 2, 3]));
        let wav = DragPayload::bytes(DragBytes::new("audio/wav", vec![4, 5]));

        assert!(DropPayloadFilter::empty().text().accepts_payload(&text));
        assert!(DropPayloadFilter::empty().files().accepts_payload(&files));
        assert!(DropPayloadFilter::empty()
            .mime_type("image/*")
            .accepts_payload(&image));
        assert!(!DropPayloadFilter::empty()
            .mime_type("image/*")
            .accepts_payload(&wav));
        assert!(DropPayloadFilter::any().accepts_payload(&wav));
    }

    #[test]
    fn drag_source_builds_platform_start_request_when_available() {
        let source = DragSourceDescriptor::new(
            "clip.1",
            DragDropSurfaceKind::EditorRangeItem,
            UiRect::new(10.0, 20.0, 100.0, 30.0),
            DragPayload::text("clip"),
        )
        .allowed_operations([DragOperation::Move])
        .label("Verse clip");

        let request = source
            .start_request(UiPoint::new(12.0, 24.0))
            .expect("source can start");
        assert_eq!(
            request,
            DragDropRequest::Start {
                id: DragId::new("clip.1"),
                payload: DragPayload::text("clip"),
                origin: LogicalPoint::new(12.0, 24.0),
                allowed_operations: vec![DragOperation::Move],
                drag_image: None,
            }
        );
        assert_eq!(
            source.accessibility_meta().label.as_deref(),
            Some("Verse clip")
        );

        let disabled = source.clone().disabled(true);
        assert!(disabled.start_request(UiPoint::new(0.0, 0.0)).is_none());
    }

    #[test]
    fn drop_target_hit_test_prefers_topmost_matching_target() {
        let payload = DragPayload::bytes(DragBytes::new("audio/wav", vec![1]));
        let targets = vec![
            DropTargetDescriptor::new(
                "track.1",
                DragDropSurfaceKind::EditorLane,
                UiRect::new(0.0, 0.0, 300.0, 60.0),
            )
            .accept_mime_type("audio/*")
            .accepted_operations([DragOperation::Copy, DragOperation::Move])
            .z_index(1),
            DropTargetDescriptor::new(
                "clip.1",
                DragDropSurfaceKind::EditorRangeItem,
                UiRect::new(50.0, 10.0, 100.0, 30.0),
            )
            .accept_mime_type("audio/wav")
            .accepted_operations([DragOperation::Move])
            .z_index(4),
        ];

        let hit = DropTargetDescriptor::hit_test(
            &targets,
            UiPoint::new(80.0, 20.0),
            &payload,
            &[DragOperation::Move],
        )
        .expect("hit");
        assert_eq!(hit.target_id, DropTargetId::new("clip.1"));
        assert_eq!(hit.local_position, UiPoint::new(30.0, 10.0));
        assert_eq!(hit.operation, DragOperation::Move);
    }

    #[test]
    fn drop_target_rejects_payload_operations_and_disabled_targets() {
        let payload = DragPayload::text("row");
        let target = DropTargetDescriptor::new(
            "row.1",
            DragDropSurfaceKind::TableRow,
            UiRect::new(0.0, 0.0, 100.0, 20.0),
        )
        .accept_files()
        .accepted_operations([DragOperation::Copy]);

        assert_eq!(
            target.resolve_operation(&payload, &[DragOperation::Copy]),
            None
        );

        let target = target.accept_text();
        assert_eq!(
            target.resolve_operation(&payload, &[DragOperation::Move]),
            None
        );
        assert_eq!(
            target.resolve_operation(&payload, &[DragOperation::Copy]),
            Some(DragOperation::Copy)
        );
        assert_eq!(
            target
                .disabled(true)
                .resolve_operation(&payload, &[DragOperation::Copy]),
            None
        );
    }

    #[test]
    fn drop_target_accessibility_summarizes_acceptance_policy() {
        let target = DropTargetDescriptor::new(
            "assets",
            DragDropSurfaceKind::Asset,
            UiRect::new(0.0, 0.0, 200.0, 80.0),
        )
        .accept_files()
        .accept_mime_type("image/*")
        .label("Asset bin");

        let meta = target.accessibility_meta();
        assert_eq!(meta.role, AccessibilityRole::ListItem);
        assert_eq!(meta.label.as_deref(), Some("Asset bin"));
        let text = meta.summary.unwrap().screen_reader_text();
        assert!(text.contains("Accepts: files, image/*"));
        assert!(text.contains("Operations: copy, move, link"));
    }
}
