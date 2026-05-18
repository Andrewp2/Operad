use crate::drag_drop::{DragSourceDescriptor, DragSourceId, DropTargetDescriptor, DropTargetId};
use crate::platform::{
    DragDropRequest, DragImage, DragOperation, DragPayload, LogicalPoint, LogicalSize,
};

use super::*;

fn all_drag_operations() -> Vec<DragOperation> {
    vec![
        DragOperation::Copy,
        DragOperation::Move,
        DragOperation::Link,
    ]
}

const fn logical_point(point: UiPoint) -> LogicalPoint {
    LogicalPoint::new(point.x, point.y)
}

const fn logical_size(size: UiSize) -> LogicalSize {
    LogicalSize::new(size.width, size.height)
}

#[derive(Debug, Clone, PartialEq)]
pub enum DragImagePolicy {
    None,
    Label {
        size: UiSize,
        hotspot: UiPoint,
    },
    ImageKey {
        key: String,
        size: UiSize,
        hotspot: UiPoint,
    },
    Custom(DragImage),
}

impl DragImagePolicy {
    pub const fn none() -> Self {
        Self::None
    }

    pub const fn label(size: UiSize, hotspot: UiPoint) -> Self {
        Self::Label { size, hotspot }
    }

    pub fn image_key(key: impl Into<String>, size: UiSize, hotspot: UiPoint) -> Self {
        Self::ImageKey {
            key: key.into(),
            size,
            hotspot,
        }
    }

    pub fn custom(image: DragImage) -> Self {
        Self::Custom(image)
    }

    pub fn resolve(&self, label: &str) -> Option<DragImage> {
        match self {
            Self::None => None,
            Self::Label { size, hotspot } => Some(
                DragImage::new(logical_size(*size))
                    .label(label.to_string())
                    .hotspot(logical_point(*hotspot)),
            ),
            Self::ImageKey { key, size, hotspot } => Some(
                DragImage::new(logical_size(*size))
                    .label(label.to_string())
                    .image_key(key.clone())
                    .hotspot(logical_point(*hotspot)),
            ),
            Self::Custom(image) => Some(image.clone()),
        }
    }
}

impl Default for DragImagePolicy {
    fn default() -> Self {
        Self::Label {
            size: UiSize::new(160.0, 36.0),
            hotspot: UiPoint::new(12.0, 12.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DragSourceOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub hovered_visual: Option<UiVisual>,
    pub focused_visual: Option<UiVisual>,
    pub disabled_visual: Option<UiVisual>,
    pub text_style: TextStyle,
    pub kind: DragDropSurfaceKind,
    pub allowed_operations: Vec<DragOperation>,
    pub drag_image: DragImagePolicy,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for DragSourceOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::row()
                .with_height(40.0)
                .with_padding(8.0)
                .with_gap(8.0)
                .with_flex_shrink(0.0),
            visual: UiVisual::panel(
                ColorRgba::new(36, 42, 52, 255),
                Some(StrokeStyle::new(ColorRgba::new(86, 104, 126, 255), 1.0)),
                4.0,
            ),
            hovered_visual: Some(UiVisual::panel(
                ColorRgba::new(48, 61, 76, 255),
                Some(StrokeStyle::new(ColorRgba::new(132, 166, 204, 255), 1.0)),
                4.0,
            )),
            focused_visual: Some(UiVisual::panel(
                ColorRgba::new(36, 42, 52, 255),
                Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.5)),
                4.0,
            )),
            disabled_visual: Some(UiVisual::panel(
                ColorRgba::new(30, 34, 40, 180),
                Some(StrokeStyle::new(ColorRgba::new(64, 72, 84, 180), 1.0)),
                4.0,
            )),
            text_style: TextStyle::default(),
            kind: DragDropSurfaceKind::Custom("Drag source".to_string()),
            allowed_operations: all_drag_operations(),
            drag_image: DragImagePolicy::default(),
            enabled: true,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl DragSourceOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_kind(mut self, kind: DragDropSurfaceKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_allowed_operations(
        mut self,
        operations: impl IntoIterator<Item = DragOperation>,
    ) -> Self {
        self.allowed_operations = operations.into_iter().collect();
        self
    }

    pub fn with_drag_image_policy(mut self, policy: DragImagePolicy) -> Self {
        self.drag_image = policy;
        self
    }

    pub fn without_drag_image(mut self) -> Self {
        self.drag_image = DragImagePolicy::None;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn with_accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    fn resolved_visual(&self, document: &UiDocument, node: UiNodeId) -> UiVisual {
        if !self.enabled {
            return self.disabled_visual.unwrap_or(self.visual);
        }
        if document.focus.focused == Some(node) {
            return self.focused_visual.unwrap_or(self.visual);
        }
        if document.focus.hovered == Some(node) {
            return self.hovered_visual.unwrap_or(self.visual);
        }
        self.visual
    }

    fn interaction_visuals(&self) -> InteractionVisuals {
        InteractionVisuals::new(self.visual)
            .hovered(self.hovered_visual.unwrap_or(self.visual))
            .focused(self.focused_visual.unwrap_or(self.visual))
            .disabled(self.disabled_visual.unwrap_or(self.visual))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragSourceNodes {
    pub root: UiNodeId,
    pub label: UiNodeId,
}

#[derive(Debug, Clone)]
pub struct DropZoneOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub hovered_visual: Option<UiVisual>,
    pub active_visual: Option<UiVisual>,
    pub rejected_visual: Option<UiVisual>,
    pub disabled_visual: Option<UiVisual>,
    pub text_style: TextStyle,
    pub kind: DragDropSurfaceKind,
    pub accepted_payload: DropPayloadFilter,
    pub accepted_operations: Vec<DragOperation>,
    pub z_index: i16,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for DropZoneOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column()
                .with_width(240.0)
                .with_height(120.0)
                .with_padding(12.0)
                .with_gap(8.0),
            visual: UiVisual::panel(
                ColorRgba::new(20, 25, 32, 255),
                Some(StrokeStyle::new(ColorRgba::new(82, 98, 122, 255), 1.0)),
                4.0,
            ),
            hovered_visual: Some(UiVisual::panel(
                ColorRgba::new(28, 38, 50, 255),
                Some(StrokeStyle::new(ColorRgba::new(118, 154, 196, 255), 1.0)),
                4.0,
            )),
            active_visual: Some(UiVisual::panel(
                ColorRgba::new(27, 50, 40, 255),
                Some(StrokeStyle::new(ColorRgba::new(108, 184, 142, 255), 1.0)),
                4.0,
            )),
            rejected_visual: Some(UiVisual::panel(
                ColorRgba::new(58, 32, 36, 255),
                Some(StrokeStyle::new(ColorRgba::new(210, 96, 106, 255), 1.0)),
                4.0,
            )),
            disabled_visual: Some(UiVisual::panel(
                ColorRgba::new(18, 21, 26, 180),
                Some(StrokeStyle::new(ColorRgba::new(56, 64, 76, 180), 1.0)),
                4.0,
            )),
            text_style: TextStyle::default(),
            kind: DragDropSurfaceKind::Custom("Drop zone".to_string()),
            accepted_payload: DropPayloadFilter::any(),
            accepted_operations: all_drag_operations(),
            z_index: 0,
            enabled: true,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl DropZoneOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_kind(mut self, kind: DragDropSurfaceKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_accepted_payload(mut self, filter: DropPayloadFilter) -> Self {
        self.accepted_payload = filter;
        self
    }

    pub fn with_accepted_operations(
        mut self,
        operations: impl IntoIterator<Item = DragOperation>,
    ) -> Self {
        self.accepted_operations = operations.into_iter().collect();
        self
    }

    pub fn with_rejected_visual(mut self, visual: UiVisual) -> Self {
        self.rejected_visual = Some(visual);
        self
    }

    pub const fn with_z_index(mut self, z_index: i16) -> Self {
        self.z_index = z_index;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn with_accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    fn resolved_visual(&self, document: &UiDocument, node: UiNodeId) -> UiVisual {
        if !self.enabled {
            return self.disabled_visual.unwrap_or(self.visual);
        }
        if document.focus.pressed == Some(node) {
            return self.active_visual.unwrap_or(self.visual);
        }
        if document.focus.hovered == Some(node) {
            return self.hovered_visual.unwrap_or(self.visual);
        }
        self.visual
    }

    fn interaction_visuals(&self) -> InteractionVisuals {
        InteractionVisuals::new(self.visual)
            .hovered(self.hovered_visual.unwrap_or(self.visual))
            .pressed(self.active_visual.unwrap_or(self.visual))
            .disabled(self.disabled_visual.unwrap_or(self.visual))
    }

    pub fn preview_visual(&self, state: DropZonePreviewState) -> UiVisual {
        match state {
            DropZonePreviewState::Idle => self.visual,
            DropZonePreviewState::Hovered => self.hovered_visual.unwrap_or(self.visual),
            DropZonePreviewState::Accepted => self.active_visual.unwrap_or(self.visual),
            DropZonePreviewState::Rejected => self
                .rejected_visual
                .or(self.hovered_visual)
                .unwrap_or(self.visual),
            DropZonePreviewState::Disabled => self.disabled_visual.unwrap_or(self.visual),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropZonePreviewState {
    Idle,
    Hovered,
    Accepted,
    Rejected,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropZoneNodes {
    pub root: UiNodeId,
    pub label: UiNodeId,
}

pub fn dnd_drag_source(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    payload: DragPayload,
    options: DragSourceOptions,
) -> DragSourceNodes {
    let name = name.into();
    let label = label.into();
    let mut accessibility = DragSourceDescriptor::new(
        DragSourceId::new(name.clone()),
        options.kind.clone(),
        UiRect::new(0.0, 0.0, 0.0, 0.0),
        payload,
    )
    .allowed_operations(options.allowed_operations.clone())
    .disabled(!options.enabled)
    .label(
        options
            .accessibility_label
            .clone()
            .unwrap_or_else(|| label.clone()),
    )
    .accessibility_meta();
    if let Some(hint) = options.accessibility_hint.clone() {
        accessibility = accessibility.hint(hint);
    }

    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style.clone(),
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_interaction_visuals(options.interaction_visuals())
    .with_visual(options.visual)
    .with_input(if options.enabled {
        InputBehavior::BUTTON
    } else {
        InputBehavior::NONE
    })
    .with_accessibility(accessibility);
    if let Some(action) = options.action.clone() {
        node = node.with_action(action);
    }

    let root = document.add_child(parent, node);
    document.set_node_visual(root, options.resolved_visual(document, root));
    let mut text_style = options.text_style;
    text_style.wrap = TextWrap::None;
    let label_node = document.add_child(
        root,
        UiNode::text(
            format!("{name}.label"),
            label,
            text_style,
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        ),
    );
    DragSourceNodes {
        root,
        label: label_node,
    }
}

pub fn dnd_drop_zone(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    options: DropZoneOptions,
) -> DropZoneNodes {
    let name = name.into();
    let label = label.into();
    let mut descriptor = DropTargetDescriptor::new(
        DropTargetId::new(name.clone()),
        options.kind.clone(),
        UiRect::new(0.0, 0.0, 0.0, 0.0),
    )
    .accepted_payload(options.accepted_payload.clone())
    .accepted_operations(options.accepted_operations.clone())
    .z_index(options.z_index)
    .disabled(!options.enabled)
    .label(
        options
            .accessibility_label
            .clone()
            .unwrap_or_else(|| label.clone()),
    );
    if let Some(hint) = options.accessibility_hint.clone() {
        descriptor = descriptor.hint(hint);
    }

    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style.clone(),
            clip: ClipBehavior::Clip,
            z_index: options.z_index,
            ..Default::default()
        },
    )
    .with_interaction_visuals(options.interaction_visuals())
    .with_visual(options.visual)
    .with_input(if options.enabled {
        InputBehavior::BUTTON
    } else {
        InputBehavior::NONE
    })
    .with_accessibility(descriptor.accessibility_meta());
    if let Some(action) = options.action.clone() {
        node = node.with_action(action);
    }

    let root = document.add_child(parent, node);
    document.set_node_visual(root, options.resolved_visual(document, root));
    let mut text_style = options.text_style;
    text_style.wrap = TextWrap::None;
    let label_node = document.add_child(
        root,
        UiNode::text(
            format!("{name}.label"),
            label,
            text_style,
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        ),
    );
    DropZoneNodes {
        root,
        label: label_node,
    }
}

pub fn dnd_drag_source_descriptor(
    document: &UiDocument,
    source: UiNodeId,
    payload: DragPayload,
    options: &DragSourceOptions,
) -> DragSourceDescriptor {
    let node = document.node(source);
    let label = options
        .accessibility_label
        .clone()
        .or_else(|| {
            node.accessibility
                .as_ref()
                .and_then(|meta| meta.label.clone())
        })
        .unwrap_or_else(|| node.name.clone());
    let mut descriptor = DragSourceDescriptor::new(
        DragSourceId::new(node.name.clone()),
        options.kind.clone(),
        node.layout.rect,
        payload,
    )
    .allowed_operations(options.allowed_operations.clone())
    .drag_image(options.drag_image.resolve(&label))
    .disabled(!options.enabled)
    .label(label);
    if let Some(hint) = options.accessibility_hint.clone() {
        descriptor = descriptor.hint(hint);
    }
    descriptor
}

pub fn dnd_drop_zone_preview_state(
    document: &UiDocument,
    target: UiNodeId,
    options: &DropZoneOptions,
    payload: Option<&DragPayload>,
    source_operations: &[DragOperation],
    point: UiPoint,
) -> DropZonePreviewState {
    if !options.enabled || !action_target_enabled(document, target) {
        return DropZonePreviewState::Disabled;
    }
    let descriptor = dnd_drop_target_descriptor(document, target, options);
    if !descriptor.contains_point(point) {
        return DropZonePreviewState::Idle;
    }
    let Some(payload) = payload else {
        return DropZonePreviewState::Hovered;
    };
    if descriptor
        .resolve_operation(payload, source_operations)
        .is_some()
    {
        DropZonePreviewState::Accepted
    } else {
        DropZonePreviewState::Rejected
    }
}

pub fn dnd_apply_drop_zone_preview(
    document: &mut UiDocument,
    target: UiNodeId,
    options: &DropZoneOptions,
    state: DropZonePreviewState,
) {
    document.set_node_visual(target, options.preview_visual(state));
}

pub fn dnd_drag_start_request(
    document: &UiDocument,
    source: UiNodeId,
    payload: DragPayload,
    options: &DragSourceOptions,
    origin: UiPoint,
) -> Option<DragDropRequest> {
    dnd_drag_source_descriptor(document, source, payload, options).start_request(origin)
}

pub fn dnd_drop_target_descriptor(
    document: &UiDocument,
    target: UiNodeId,
    options: &DropZoneOptions,
) -> DropTargetDescriptor {
    let node = document.node(target);
    let label = options
        .accessibility_label
        .clone()
        .or_else(|| {
            node.accessibility
                .as_ref()
                .and_then(|meta| meta.label.clone())
        })
        .unwrap_or_else(|| node.name.clone());
    let mut descriptor = DropTargetDescriptor::new(
        DropTargetId::new(node.name.clone()),
        options.kind.clone(),
        node.layout.rect,
    )
    .accepted_payload(options.accepted_payload.clone())
    .accepted_operations(options.accepted_operations.clone())
    .z_index(options.z_index)
    .disabled(!options.enabled)
    .label(label);
    if let Some(hint) = options.accessibility_hint.clone() {
        descriptor = descriptor.hint(hint);
    }
    descriptor
}

pub fn dnd_drag_source_actions_from_gesture_event(
    document: &UiDocument,
    source: UiNodeId,
    options: &DragSourceOptions,
    event: &GestureEvent,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    let Some(binding) = options.action.clone() else {
        return queue;
    };
    let GestureEvent::Drag(gesture) = event else {
        return queue;
    };
    if gesture.target != source && !document.node_is_descendant_or_self(source, gesture.target)
        || !action_target_enabled(document, source)
    {
        return queue;
    }
    if let Some(mut action) = WidgetAction::drag_from_gesture(gesture, binding) {
        action.target = source;
        queue.push(action);
    }
    queue
}

pub fn dnd_drop_zone_actions_from_gesture_event(
    document: &UiDocument,
    target: UiNodeId,
    options: &DropZoneOptions,
    event: &GestureEvent,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    let Some(binding) = options.action.clone() else {
        return queue;
    };
    let GestureEvent::Drag(gesture) = event else {
        return queue;
    };
    if gesture.target != target && !document.node_is_descendant_or_self(target, gesture.target)
        || !action_target_enabled(document, target)
    {
        return queue;
    }
    if let Some(mut action) = WidgetAction::drag_from_gesture(gesture, binding) {
        action.target = target;
        queue.push(action);
    }
    queue
}

#[cfg(test)]
mod tests {
    use crate::input::{DragGesture, GesturePhase};

    use super::*;

    #[test]
    fn dnd_builders_create_accessible_surfaces_and_descriptors() {
        let mut document = UiDocument::new(root_style(420.0, 240.0));
        let root = document.root;
        let source_options = DragSourceOptions::default()
            .with_kind(DragDropSurfaceKind::Asset)
            .with_allowed_operations([DragOperation::Move])
            .with_action("asset.drag")
            .with_accessibility_label("Asset clip");
        let payload = DragPayload::text("clip-a");
        let source = dnd_drag_source(
            &mut document,
            root,
            "clip",
            "Clip A",
            payload.clone(),
            source_options.clone(),
        );
        document.node_mut(source.root).layout.rect = UiRect::new(10.0, 20.0, 120.0, 40.0);

        let drop_options = DropZoneOptions::default()
            .with_kind(DragDropSurfaceKind::ListRow)
            .with_accepted_payload(DropPayloadFilter::empty().text())
            .with_accepted_operations([DragOperation::Move])
            .with_z_index(7)
            .with_accessibility_label("Timeline lane");
        let target = dnd_drop_zone(
            &mut document,
            root,
            "lane",
            "Drop here",
            drop_options.clone(),
        );
        document.node_mut(target.root).layout.rect = UiRect::new(0.0, 80.0, 320.0, 100.0);

        let source_descriptor =
            dnd_drag_source_descriptor(&document, source.root, payload.clone(), &source_options);
        assert_eq!(source_descriptor.id, DragSourceId::new("clip"));
        assert_eq!(
            source_descriptor.bounds,
            UiRect::new(10.0, 20.0, 120.0, 40.0)
        );
        assert_eq!(
            source_descriptor.allowed_operations,
            vec![DragOperation::Move]
        );
        assert_eq!(
            source_descriptor.accessibility_meta().label.as_deref(),
            Some("Asset clip")
        );
        let drag_image = source_descriptor.drag_image.as_ref().expect("drag image");
        assert_eq!(drag_image.label.as_deref(), Some("Asset clip"));
        assert_eq!(drag_image.hotspot, LogicalPoint::new(12.0, 12.0));

        let start_request = dnd_drag_start_request(
            &document,
            source.root,
            payload.clone(),
            &source_options,
            UiPoint::new(24.0, 30.0),
        )
        .expect("drag request");
        assert!(matches!(
            start_request,
            DragDropRequest::Start {
                drag_image: Some(_),
                ..
            }
        ));

        let target_descriptor = dnd_drop_target_descriptor(&document, target.root, &drop_options);
        assert_eq!(target_descriptor.id, DropTargetId::new("lane"));
        assert_eq!(target_descriptor.z_index, 7);
        assert_eq!(
            target_descriptor.resolve_operation(&payload, &[DragOperation::Move]),
            Some(DragOperation::Move)
        );
        let accessibility = document.node(target.root).accessibility.as_ref().unwrap();
        assert_eq!(accessibility.label.as_deref(), Some("Timeline lane"));
    }

    #[test]
    fn dnd_drop_preview_state_applies_accepted_and_rejected_visuals() {
        let mut document = UiDocument::new(root_style(420.0, 240.0));
        let root = document.root;
        let options = DropZoneOptions::default()
            .with_accepted_payload(DropPayloadFilter::empty().text())
            .with_accepted_operations([DragOperation::Move]);
        let target = dnd_drop_zone(&mut document, root, "lane", "Drop here", options.clone());
        document.node_mut(target.root).layout.rect = UiRect::new(0.0, 20.0, 320.0, 120.0);

        let accepted = dnd_drop_zone_preview_state(
            &document,
            target.root,
            &options,
            Some(&DragPayload::text("clip")),
            &[DragOperation::Move],
            UiPoint::new(24.0, 40.0),
        );
        assert_eq!(accepted, DropZonePreviewState::Accepted);
        dnd_apply_drop_zone_preview(&mut document, target.root, &options, accepted);
        assert_eq!(
            document.node(target.root).visual,
            options.active_visual.unwrap()
        );

        let rejected = dnd_drop_zone_preview_state(
            &document,
            target.root,
            &options,
            Some(&DragPayload::files(["clip.wav"])),
            &[DragOperation::Move],
            UiPoint::new(24.0, 40.0),
        );
        assert_eq!(rejected, DropZonePreviewState::Rejected);
        dnd_apply_drop_zone_preview(&mut document, target.root, &options, rejected);
        assert_eq!(
            document.node(target.root).visual,
            options.rejected_visual.unwrap()
        );

        assert_eq!(
            dnd_drop_zone_preview_state(
                &document,
                target.root,
                &options,
                None,
                &[],
                UiPoint::new(24.0, 40.0),
            ),
            DropZonePreviewState::Hovered
        );
    }

    #[test]
    fn dnd_drag_source_actions_follow_descendant_gestures() {
        let mut document = UiDocument::new(root_style(240.0, 120.0));
        let root = document.root;
        let options = DragSourceOptions::default().with_action("clip.drag");
        let source = dnd_drag_source(
            &mut document,
            root,
            "clip",
            "Clip",
            DragPayload::text("clip"),
            options.clone(),
        );
        let event = GestureEvent::Drag(DragGesture {
            target: source.label,
            pointer_id: crate::input::PointerId::new(1),
            button: PointerButton::Primary,
            phase: GesturePhase::Update,
            origin: UiPoint::new(10.0, 10.0),
            current: UiPoint::new(40.0, 12.0),
            previous: UiPoint::new(30.0, 10.0),
            delta: UiPoint::new(10.0, 2.0),
            total_delta: UiPoint::new(30.0, 2.0),
            modifiers: KeyModifiers::NONE,
            captured: true,
            timestamp_millis: 16,
        });

        let queue =
            dnd_drag_source_actions_from_gesture_event(&document, source.root, &options, &event);
        let actions = queue.into_vec();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].target, source.root);
        assert!(matches!(actions[0].kind, WidgetActionKind::Drag(_)));
    }
}
