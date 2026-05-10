//! Backend-facing accessibility contracts.
//!
//! Core widgets describe semantics through `AccessibilityMeta`; this module is
//! the bridge a backend uses to publish those semantics to a screen reader,
//! coordinate focus traps, and reflect host accessibility preferences.

use std::collections::{HashMap, HashSet};

use crate::{AccessibilityLiveRegion, AccessibilityNode, AccessibilityTree, UiNodeId};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccessibilityPreferences {
    pub screen_reader_active: bool,
    pub reduced_motion: bool,
    pub high_contrast: bool,
    pub forced_colors: bool,
    pub reduced_transparency: bool,
    pub text_scale: f32,
}

impl AccessibilityPreferences {
    pub const MIN_TEXT_SCALE: f32 = 0.75;
    pub const MAX_TEXT_SCALE: f32 = 2.0;

    pub const DEFAULT: Self = Self {
        screen_reader_active: false,
        reduced_motion: false,
        high_contrast: false,
        forced_colors: false,
        reduced_transparency: false,
        text_scale: 1.0,
    };

    pub const fn screen_reader_active(mut self, active: bool) -> Self {
        self.screen_reader_active = active;
        self
    }

    pub const fn reduced_motion(mut self, reduced: bool) -> Self {
        self.reduced_motion = reduced;
        self
    }

    pub const fn high_contrast(mut self, high_contrast: bool) -> Self {
        self.high_contrast = high_contrast;
        self
    }

    pub const fn forced_colors(mut self, forced_colors: bool) -> Self {
        self.forced_colors = forced_colors;
        self
    }

    pub const fn reduced_transparency(mut self, reduced: bool) -> Self {
        self.reduced_transparency = reduced;
        self
    }

    pub const fn text_scale(mut self, scale: f32) -> Self {
        self.text_scale = scale;
        self
    }

    pub const fn should_reduce_motion(self) -> bool {
        self.reduced_motion || self.screen_reader_active
    }

    pub const fn should_use_high_contrast(self) -> bool {
        self.high_contrast || self.forced_colors
    }

    pub const fn prefers_reduced_transparency(self) -> bool {
        self.reduced_transparency || self.forced_colors
    }

    pub fn normalized_text_scale(self) -> f32 {
        if self.text_scale.is_finite() {
            self.text_scale
                .clamp(Self::MIN_TEXT_SCALE, Self::MAX_TEXT_SCALE)
        } else {
            Self::DEFAULT.text_scale
        }
    }
}

impl Default for AccessibilityPreferences {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AccessibilityCapabilities {
    pub screen_reader_tree: bool,
    pub focus_restore: bool,
    pub focus_trap: bool,
    pub announcements: bool,
    pub live_regions: bool,
    pub preferences: bool,
    pub reduced_motion: bool,
    pub high_contrast: bool,
    pub clipboard: bool,
    pub text_ime: bool,
    pub drag_drop: bool,
    pub screenshots: bool,
}

impl AccessibilityCapabilities {
    pub const NONE: Self = Self {
        screen_reader_tree: false,
        focus_restore: false,
        focus_trap: false,
        announcements: false,
        live_regions: false,
        preferences: false,
        reduced_motion: false,
        high_contrast: false,
        clipboard: false,
        text_ime: false,
        drag_drop: false,
        screenshots: false,
    };

    pub const FULL: Self = Self {
        screen_reader_tree: true,
        focus_restore: true,
        focus_trap: true,
        announcements: true,
        live_regions: true,
        preferences: true,
        reduced_motion: true,
        high_contrast: true,
        clipboard: true,
        text_ime: true,
        drag_drop: true,
        screenshots: true,
    };

    pub const SCREEN_READER: Self = Self {
        screen_reader_tree: true,
        focus_restore: true,
        focus_trap: true,
        announcements: true,
        live_regions: true,
        preferences: true,
        reduced_motion: true,
        high_contrast: true,
        clipboard: false,
        text_ime: false,
        drag_drop: false,
        screenshots: false,
    };

    pub const fn supports(self, request: AccessibilityRequestKind) -> bool {
        match request {
            AccessibilityRequestKind::PublishTree => self.screen_reader_tree,
            AccessibilityRequestKind::MoveFocus => self.focus_restore,
            AccessibilityRequestKind::SetFocusTrap => self.focus_trap,
            AccessibilityRequestKind::ClearFocusTrap => self.focus_trap,
            AccessibilityRequestKind::RestoreFocus => self.focus_restore,
            AccessibilityRequestKind::Announce => self.announcements,
            AccessibilityRequestKind::ApplyPreferences => self.preferences,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityRequestKind {
    PublishTree,
    MoveFocus,
    SetFocusTrap,
    ClearFocusTrap,
    RestoreFocus,
    Announce,
    ApplyPreferences,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusNavigationDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusRestoreTarget {
    None,
    Previous,
    Node(UiNodeId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusTrap {
    pub root: UiNodeId,
    pub restore_focus: FocusRestoreTarget,
    pub wrap: bool,
}

impl FocusTrap {
    pub const fn new(root: UiNodeId) -> Self {
        Self {
            root,
            restore_focus: FocusRestoreTarget::Previous,
            wrap: true,
        }
    }

    pub const fn restore_focus(mut self, target: FocusRestoreTarget) -> Self {
        self.restore_focus = target;
        self
    }

    pub const fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn contains(self, tree: &AccessibilityTree, node: UiNodeId) -> bool {
        tree.contains_node(self.root, node)
    }

    pub fn focus_order(self, tree: &AccessibilityTree) -> Vec<UiNodeId> {
        tree.focus_order
            .iter()
            .copied()
            .filter(|node| tree.is_focus_candidate(*node) && self.contains(tree, *node))
            .collect()
    }

    pub fn next_focus(
        self,
        tree: &AccessibilityTree,
        current: Option<UiNodeId>,
        direction: FocusNavigationDirection,
    ) -> Option<UiNodeId> {
        next_focus_in_order(&self.focus_order(tree), current, direction, self.wrap)
    }

    pub fn move_focus_request(
        self,
        tree: &AccessibilityTree,
        current: Option<UiNodeId>,
        direction: FocusNavigationDirection,
    ) -> Option<AccessibilityAdapterRequest> {
        self.next_focus(tree, current, direction).map(|target| {
            AccessibilityAdapterRequest::MoveFocus {
                target,
                restore: self.restore_focus,
            }
        })
    }

    pub const fn restore_focus_request(self) -> AccessibilityAdapterRequest {
        AccessibilityAdapterRequest::RestoreFocus(self.restore_focus)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityAnnouncement {
    pub source: Option<UiNodeId>,
    pub message: String,
    pub live_region: AccessibilityLiveRegion,
    pub interrupt: bool,
}

impl AccessibilityAnnouncement {
    pub fn new(message: impl Into<String>, live_region: AccessibilityLiveRegion) -> Self {
        Self {
            source: None,
            message: message.into(),
            live_region,
            interrupt: matches!(live_region, AccessibilityLiveRegion::Assertive),
        }
    }

    pub fn polite(message: impl Into<String>) -> Self {
        Self::new(message, AccessibilityLiveRegion::Polite)
    }

    pub fn assertive(message: impl Into<String>) -> Self {
        Self::new(message, AccessibilityLiveRegion::Assertive)
    }

    pub fn source(mut self, source: UiNodeId) -> Self {
        self.source = Some(source);
        self
    }

    pub const fn interrupt(mut self, interrupt: bool) -> Self {
        self.interrupt = interrupt;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityLiveRegionEntry {
    pub node: UiNodeId,
    pub message: String,
    pub live_region: AccessibilityLiveRegion,
}

impl AccessibilityLiveRegionEntry {
    pub fn from_node(node: &AccessibilityNode) -> Option<Self> {
        if node.live_region == AccessibilityLiveRegion::Off {
            return None;
        }
        let message = live_region_message(node);
        (!message.is_empty()).then_some(Self {
            node: node.id,
            message,
            live_region: node.live_region,
        })
    }

    pub fn announcement(&self) -> AccessibilityAnnouncement {
        AccessibilityAnnouncement::new(self.message.clone(), self.live_region).source(self.node)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccessibilityLiveRegionSnapshot {
    pub entries: Vec<AccessibilityLiveRegionEntry>,
}

impl AccessibilityLiveRegionSnapshot {
    pub fn from_tree(tree: &AccessibilityTree) -> Self {
        let mut entries = tree
            .live_region_nodes()
            .filter_map(AccessibilityLiveRegionEntry::from_node)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| (live_region_priority(entry.live_region), entry.node.0));
        Self { entries }
    }

    pub fn announcements_since(
        &self,
        previous: &AccessibilityLiveRegionSnapshot,
    ) -> Vec<AccessibilityAnnouncement> {
        let previous_by_node = previous
            .entries
            .iter()
            .map(|entry| (entry.node, entry))
            .collect::<HashMap<_, _>>();

        self.entries
            .iter()
            .filter(|entry| {
                previous_by_node.get(&entry.node).is_none_or(|previous| {
                    previous.message != entry.message || previous.live_region != entry.live_region
                })
            })
            .map(AccessibilityLiveRegionEntry::announcement)
            .collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccessibilityAnnouncementQueue {
    pub pending: Vec<AccessibilityAnnouncement>,
}

impl AccessibilityAnnouncementQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_live_region_diff(
        previous: &AccessibilityLiveRegionSnapshot,
        current: &AccessibilityLiveRegionSnapshot,
    ) -> Self {
        Self {
            pending: current.announcements_since(previous),
        }
    }

    pub fn push(&mut self, announcement: AccessibilityAnnouncement) {
        if !announcement.message.is_empty() {
            self.pending.push(announcement);
            self.prioritize();
        }
    }

    pub fn extend(&mut self, announcements: impl IntoIterator<Item = AccessibilityAnnouncement>) {
        for announcement in announcements {
            self.push(announcement);
        }
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn supported_requests(
        &self,
        capabilities: AccessibilityCapabilities,
    ) -> Vec<AccessibilityAdapterRequest> {
        if !capabilities.supports(AccessibilityRequestKind::Announce) {
            return Vec::new();
        }
        self.pending
            .iter()
            .cloned()
            .map(AccessibilityAdapterRequest::Announce)
            .collect()
    }

    pub fn drain(&mut self) -> Vec<AccessibilityAnnouncement> {
        self.pending.drain(..).collect()
    }

    fn prioritize(&mut self) {
        self.pending
            .sort_by_key(|announcement| live_region_priority(announcement.live_region));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccessibilityAdapterRequest {
    PublishTree {
        tree: AccessibilityTree,
        focused: Option<UiNodeId>,
        preferences: AccessibilityPreferences,
    },
    MoveFocus {
        target: UiNodeId,
        restore: FocusRestoreTarget,
    },
    SetFocusTrap(FocusTrap),
    ClearFocusTrap {
        restore: FocusRestoreTarget,
    },
    RestoreFocus(FocusRestoreTarget),
    Announce(AccessibilityAnnouncement),
    ApplyPreferences(AccessibilityPreferences),
}

impl AccessibilityAdapterRequest {
    pub const fn kind(&self) -> AccessibilityRequestKind {
        match self {
            Self::PublishTree { .. } => AccessibilityRequestKind::PublishTree,
            Self::MoveFocus { .. } => AccessibilityRequestKind::MoveFocus,
            Self::SetFocusTrap(_) => AccessibilityRequestKind::SetFocusTrap,
            Self::ClearFocusTrap { .. } => AccessibilityRequestKind::ClearFocusTrap,
            Self::RestoreFocus(_) => AccessibilityRequestKind::RestoreFocus,
            Self::Announce(_) => AccessibilityRequestKind::Announce,
            Self::ApplyPreferences(_) => AccessibilityRequestKind::ApplyPreferences,
        }
    }
}

pub fn push_supported_accessibility_request(
    requests: &mut Vec<AccessibilityAdapterRequest>,
    capabilities: AccessibilityCapabilities,
    request: AccessibilityAdapterRequest,
) -> bool {
    if capabilities.supports(request.kind()) {
        requests.push(request);
        true
    } else {
        false
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccessibilityAdapterRequestPlan {
    pub supported_requests: Vec<AccessibilityAdapterRequest>,
    pub unsupported_responses: Vec<AccessibilityAdapterResponse>,
}

impl AccessibilityAdapterRequestPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_requests<'a>(
        requests: impl IntoIterator<Item = &'a AccessibilityAdapterRequest>,
        capabilities: AccessibilityCapabilities,
    ) -> Self {
        let mut plan = Self::new();
        for request in requests {
            plan.push_request(request, capabilities);
        }
        plan
    }

    pub fn push_request(
        &mut self,
        request: &AccessibilityAdapterRequest,
        capabilities: AccessibilityCapabilities,
    ) -> bool {
        if capabilities.supports(request.kind()) {
            self.supported_requests.push(request.clone());
            true
        } else {
            self.unsupported_responses
                .push(AccessibilityAdapterResponse::Unsupported(request.kind()));
            false
        }
    }

    pub fn is_fully_supported(&self) -> bool {
        self.unsupported_responses.is_empty()
    }

    pub fn supported_count(&self, kind: AccessibilityRequestKind) -> usize {
        self.supported_requests
            .iter()
            .filter(|request| request.kind() == kind)
            .count()
    }

    pub fn unsupported_count(&self, kind: AccessibilityRequestKind) -> usize {
        self.unsupported_responses
            .iter()
            .filter(|response| {
                matches!(response, AccessibilityAdapterResponse::Unsupported(actual) if *actual == kind)
            })
            .count()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccessibilityAdapterResponse {
    Applied,
    Unsupported(AccessibilityRequestKind),
    FocusChanged(Option<UiNodeId>),
    PreferencesChanged(AccessibilityPreferences),
    Failed {
        request: AccessibilityRequestKind,
        reason: String,
    },
}

pub trait AccessibilityAdapter {
    fn accessibility_capabilities(&self) -> AccessibilityCapabilities;

    fn handle_accessibility_request(
        &mut self,
        request: AccessibilityAdapterRequest,
    ) -> AccessibilityAdapterResponse;
}

impl AccessibilityTree {
    pub fn node(&self, id: UiNodeId) -> Option<&AccessibilityNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn accessible_name(&self, id: UiNodeId) -> Option<String> {
        let node = self.node(id)?;
        let related = self.relation_names(&node.relations.labelled_by);
        if !related.is_empty() {
            return Some(related.join(" "));
        }
        node.direct_accessible_name()
    }

    pub fn accessible_description(&self, id: UiNodeId) -> Option<String> {
        let node = self.node(id)?;
        let related = self.relation_descriptions(&node.relations.described_by);
        if !related.is_empty() {
            return Some(related.join(" "));
        }
        node.direct_accessible_description()
    }

    pub fn screen_reader_text(&self, id: UiNodeId) -> Option<String> {
        let node = self.node(id)?;
        let mut parts = Vec::new();
        if let Some(name) = self.accessible_name(id) {
            parts.push(name);
        }
        if let Some(value) = non_empty_text(node.value.as_deref()) {
            parts.push(value);
        }
        if let Some(description) = self.accessible_description(id) {
            parts.push(description);
        }
        parts.extend(accessibility_state_text(node));
        if !node.key_shortcuts.is_empty() {
            parts.push(format!("Shortcut: {}", node.key_shortcuts.join(", ")));
        }
        (!parts.is_empty()).then(|| parts.join(". "))
    }

    pub fn focusable_nodes(&self) -> impl Iterator<Item = &AccessibilityNode> {
        self.nodes
            .iter()
            .filter(|node| node.enabled && node.focusable)
    }

    pub fn effective_focus_order(&self) -> Vec<UiNodeId> {
        match self.modal_scope {
            Some(scope) => self.focus_order_for_scope(scope),
            None => self
                .focus_order
                .iter()
                .copied()
                .filter(|node| self.is_focus_candidate(*node))
                .collect(),
        }
    }

    pub fn focus_order_for_scope(&self, scope: UiNodeId) -> Vec<UiNodeId> {
        self.focus_order
            .iter()
            .copied()
            .filter(|node| self.is_focus_candidate(*node) && self.contains_node(scope, *node))
            .collect()
    }

    pub fn next_focus(
        &self,
        current: Option<UiNodeId>,
        direction: FocusNavigationDirection,
        wrap: bool,
    ) -> Option<UiNodeId> {
        next_focus_in_order(&self.effective_focus_order(), current, direction, wrap)
    }

    pub fn move_focus_request(
        &self,
        current: Option<UiNodeId>,
        direction: FocusNavigationDirection,
        wrap: bool,
        restore: FocusRestoreTarget,
    ) -> Option<AccessibilityAdapterRequest> {
        self.next_focus(current, direction, wrap)
            .map(|target| AccessibilityAdapterRequest::MoveFocus { target, restore })
    }

    pub fn live_region_nodes(&self) -> impl Iterator<Item = &AccessibilityNode> {
        self.nodes
            .iter()
            .filter(|node| node.live_region != AccessibilityLiveRegion::Off)
    }

    pub fn summary_nodes(&self) -> impl Iterator<Item = &AccessibilityNode> {
        self.nodes.iter().filter(|node| node.summary.is_some())
    }

    pub fn screen_reader_summary(&self, node: UiNodeId) -> Option<String> {
        self.node(node)
            .and_then(|node| node.summary.as_ref())
            .map(|summary| summary.screen_reader_text())
    }

    pub fn contains_node(&self, ancestor: UiNodeId, node: UiNodeId) -> bool {
        if ancestor == node {
            return self.node(node).is_some();
        }

        let mut seen = HashSet::new();
        let mut current = self.node(node).and_then(|node| node.parent);
        while let Some(parent) = current {
            if parent == ancestor {
                return true;
            }
            if !seen.insert(parent) {
                return false;
            }
            current = self.node(parent).and_then(|node| node.parent);
        }

        false
    }

    fn is_focus_candidate(&self, id: UiNodeId) -> bool {
        self.node(id)
            .is_some_and(|node| node.enabled && node.focusable)
    }

    fn relation_names(&self, ids: &[UiNodeId]) -> Vec<String> {
        ids.iter()
            .filter_map(|id| self.node(*id))
            .filter_map(AccessibilityNode::direct_accessible_name)
            .collect()
    }

    fn relation_descriptions(&self, ids: &[UiNodeId]) -> Vec<String> {
        ids.iter()
            .filter_map(|id| self.node(*id))
            .filter_map(|node| {
                node.direct_accessible_name()
                    .or_else(|| node.direct_accessible_description())
            })
            .collect()
    }
}

fn next_focus_in_order(
    order: &[UiNodeId],
    current: Option<UiNodeId>,
    direction: FocusNavigationDirection,
    wrap: bool,
) -> Option<UiNodeId> {
    if order.is_empty() {
        return None;
    }

    let position = current.and_then(|current| order.iter().position(|node| *node == current));
    match (direction, position) {
        (FocusNavigationDirection::Forward, None) => order.first().copied(),
        (FocusNavigationDirection::Forward, Some(index)) if index + 1 < order.len() => {
            order.get(index + 1).copied()
        }
        (FocusNavigationDirection::Forward, Some(_)) if wrap => order.first().copied(),
        (FocusNavigationDirection::Backward, None) => order.last().copied(),
        (FocusNavigationDirection::Backward, Some(index)) if index > 0 => {
            order.get(index - 1).copied()
        }
        (FocusNavigationDirection::Backward, Some(_)) if wrap => order.last().copied(),
        _ => None,
    }
}

impl AccessibilityNode {
    pub fn direct_accessible_name(&self) -> Option<String> {
        non_empty_text(self.label.as_deref()).or_else(|| {
            self.summary.as_ref().and_then(|summary| {
                let text = summary.screen_reader_text();
                non_empty_text(Some(&text))
            })
        })
    }

    pub fn direct_accessible_description(&self) -> Option<String> {
        non_empty_text(self.hint.as_deref()).or_else(|| {
            self.invalid.as_ref().map(|reason| {
                if reason.trim().is_empty() {
                    "Invalid".to_string()
                } else {
                    format!("Invalid: {}", reason.trim())
                }
            })
        })
    }
}

fn live_region_message(node: &AccessibilityNode) -> String {
    if let Some(summary) = &node.summary {
        let text = summary.screen_reader_text();
        if !text.is_empty() {
            return text;
        }
    }

    let mut parts = Vec::new();
    if let Some(label) = &node.label {
        if !label.is_empty() {
            parts.push(label.clone());
        }
    }
    if let Some(value) = &node.value {
        if !value.is_empty() {
            parts.push(value.clone());
        }
    }
    if let Some(hint) = &node.hint {
        if !hint.is_empty() {
            parts.push(hint.clone());
        }
    }
    parts.join(": ")
}

fn live_region_priority(live_region: AccessibilityLiveRegion) -> u8 {
    match live_region {
        AccessibilityLiveRegion::Assertive => 0,
        AccessibilityLiveRegion::Polite => 1,
        AccessibilityLiveRegion::Off => 2,
    }
}

fn non_empty_text(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn accessibility_state_text(node: &AccessibilityNode) -> Vec<String> {
    let mut parts = Vec::new();
    if !node.enabled {
        parts.push("Disabled".to_string());
    }
    if node.selected == Some(true) {
        parts.push("Selected".to_string());
    }
    if let Some(checked) = node.checked {
        parts.push(
            match checked {
                crate::AccessibilityChecked::False => "Not checked",
                crate::AccessibilityChecked::True => "Checked",
                crate::AccessibilityChecked::Mixed => "Mixed",
            }
            .to_string(),
        );
    }
    if let Some(expanded) = node.expanded {
        parts.push(if expanded { "Expanded" } else { "Collapsed" }.to_string());
    }
    if node.pressed == Some(true) {
        parts.push("Pressed".to_string());
    }
    if node.read_only {
        parts.push("Read only".to_string());
    }
    if node.required {
        parts.push("Required".to_string());
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        length, AccessibilityMeta, AccessibilityRole, AccessibilitySummary, ApproxTextMeasurer,
        InputBehavior, UiDocument, UiNode, UiNodeStyle, UiSize,
    };
    use taffy::prelude::{Dimension, Size as TaffySize, Style};

    fn fixed_style(width: f32, height: f32) -> UiNodeStyle {
        UiNodeStyle {
            layout: Style {
                size: TaffySize {
                    width: length(width),
                    height: length(height),
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn accessible_node(
        id: UiNodeId,
        parent: Option<UiNodeId>,
        focusable: bool,
    ) -> AccessibilityNode {
        AccessibilityNode {
            id,
            parent,
            role: AccessibilityRole::Button,
            label: Some(format!("node-{}", id.0)),
            value: None,
            hint: None,
            rect: crate::UiRect::new(0.0, 0.0, 20.0, 20.0),
            enabled: true,
            focusable,
            modal: false,
            selected: None,
            checked: None,
            expanded: None,
            pressed: None,
            read_only: false,
            required: false,
            invalid: None,
            live_region: AccessibilityLiveRegion::Off,
            sort: crate::AccessibilitySortDirection::None,
            value_range: None,
            focus_order: None,
            key_shortcuts: Vec::new(),
            actions: Vec::new(),
            relations: Default::default(),
            summary: None,
        }
    }

    #[derive(Debug)]
    struct RecordingAdapter {
        capabilities: AccessibilityCapabilities,
        handled: Vec<AccessibilityRequestKind>,
    }

    impl AccessibilityAdapter for RecordingAdapter {
        fn accessibility_capabilities(&self) -> AccessibilityCapabilities {
            self.capabilities
        }

        fn handle_accessibility_request(
            &mut self,
            request: AccessibilityAdapterRequest,
        ) -> AccessibilityAdapterResponse {
            let kind = request.kind();
            if !self.capabilities.supports(kind) {
                return AccessibilityAdapterResponse::Unsupported(kind);
            }

            self.handled.push(kind);
            AccessibilityAdapterResponse::Applied
        }
    }

    #[test]
    fn accessibility_snapshot_links_to_nearest_accessible_parent() {
        let mut doc = UiDocument::new(fixed_style(240.0, 120.0));
        doc.node_mut(doc.root).accessibility = Some(
            AccessibilityMeta::new(AccessibilityRole::Application)
                .label("Host")
                .focusable(),
        );

        let panel = doc.add_child(
            doc.root,
            UiNode::container(
                "panel",
                UiNodeStyle {
                    layout: Style {
                        size: TaffySize {
                            width: Dimension::auto(),
                            height: Dimension::auto(),
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
        );
        let button = doc.add_child(
            panel,
            UiNode::container("play", fixed_style(80.0, 24.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Play")
                        .focusable(),
                ),
        );

        doc.compute_layout(UiSize::new(240.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let tree = doc.accessibility_snapshot();
        assert_eq!(tree.node(button).expect("button").parent, Some(doc.root));
        assert!(tree.contains_node(doc.root, button));
        assert_eq!(tree.focus_order, vec![doc.root, button]);
    }

    #[test]
    fn accessibility_summaries_round_trip_for_custom_editor_surfaces() {
        let mut doc = UiDocument::new(fixed_style(480.0, 240.0));
        let summary = AccessibilitySummary::new("Value grid")
            .description("Range editor with value lanes and magnitude lane")
            .item("Visible units", "1 through 8")
            .item("Selected items", "3")
            .instruction("Use arrow keys to move selected items");
        let editor = doc.add_child(
            doc.root,
            UiNode::container("value-grid", fixed_style(420.0, 180.0)).with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::EditorSurface)
                    .label("Value grid")
                    .focusable()
                    .summary(summary.clone()),
            ),
        );

        doc.compute_layout(UiSize::new(480.0, 240.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let tree = doc.accessibility_snapshot();
        let node = tree.node(editor).expect("editor node");

        assert_eq!(node.summary.as_ref(), Some(&summary));
        assert_eq!(
            tree.summary_nodes().map(|node| node.id).collect::<Vec<_>>(),
            vec![editor]
        );
        assert_eq!(
            tree.screen_reader_summary(editor).as_deref(),
            Some(
                "Value grid. Range editor with value lanes and magnitude lane. Visible units: 1 through 8. Selected items: 3. Use arrow keys to move selected items"
            )
        );
    }

    #[test]
    fn accessibility_tree_resolves_relation_names_and_screen_reader_text() {
        let label = UiNodeId(1);
        let hint = UiNodeId(2);
        let button = UiNodeId(3);
        let tree = AccessibilityTree {
            nodes: vec![
                AccessibilityNode {
                    role: AccessibilityRole::Label,
                    label: Some(" Play ".to_string()),
                    ..accessible_node(label, None, false)
                },
                AccessibilityNode {
                    role: AccessibilityRole::Tooltip,
                    label: Some("Starts transport".to_string()),
                    ..accessible_node(hint, None, false)
                },
                AccessibilityNode {
                    role: AccessibilityRole::ToggleButton,
                    label: Some("Transport play".to_string()),
                    value: Some("On".to_string()),
                    pressed: Some(true),
                    key_shortcuts: vec!["Space".to_string()],
                    relations: crate::AccessibilityRelations {
                        labelled_by: vec![label],
                        described_by: vec![hint],
                        ..Default::default()
                    },
                    ..accessible_node(button, None, true)
                },
            ],
            focus_order: vec![button],
            modal_scope: None,
        };

        assert_eq!(tree.accessible_name(button).as_deref(), Some("Play"));
        assert_eq!(
            tree.accessible_description(button).as_deref(),
            Some("Starts transport")
        );
        assert_eq!(
            tree.screen_reader_text(button).as_deref(),
            Some("Play. On. Starts transport. Pressed. Shortcut: Space")
        );
        assert_eq!(tree.screen_reader_text(UiNodeId(404)), None);
    }

    #[test]
    fn focus_trap_filters_and_wraps_focus_order() {
        let root = UiNodeId(1);
        let first = UiNodeId(2);
        let second = UiNodeId(3);
        let outside = UiNodeId(4);
        let tree = AccessibilityTree {
            nodes: vec![
                accessible_node(root, None, false),
                accessible_node(first, Some(root), true),
                accessible_node(second, Some(root), true),
                accessible_node(outside, None, true),
            ],
            focus_order: vec![first, second, outside],
            modal_scope: Some(root),
        };

        let trap = FocusTrap::new(root).restore_focus(FocusRestoreTarget::Node(outside));
        assert_eq!(trap.focus_order(&tree), vec![first, second]);
        assert_eq!(
            trap.next_focus(&tree, Some(first), FocusNavigationDirection::Forward),
            Some(second)
        );
        assert_eq!(
            trap.next_focus(&tree, Some(second), FocusNavigationDirection::Forward),
            Some(first)
        );
        assert_eq!(
            trap.wrap(false)
                .next_focus(&tree, Some(second), FocusNavigationDirection::Forward),
            None
        );
        assert_eq!(
            trap.next_focus(&tree, None, FocusNavigationDirection::Backward),
            Some(second)
        );
        assert_eq!(
            trap.move_focus_request(&tree, Some(first), FocusNavigationDirection::Forward),
            Some(AccessibilityAdapterRequest::MoveFocus {
                target: second,
                restore: FocusRestoreTarget::Node(outside),
            })
        );
        assert_eq!(
            trap.restore_focus_request(),
            AccessibilityAdapterRequest::RestoreFocus(FocusRestoreTarget::Node(outside))
        );
    }

    #[test]
    fn accessibility_tree_effective_focus_order_respects_modal_scope() {
        let root = UiNodeId(1);
        let first = UiNodeId(2);
        let second = UiNodeId(3);
        let disabled = UiNodeId(4);
        let outside = UiNodeId(5);
        let tree = AccessibilityTree {
            nodes: vec![
                accessible_node(root, None, false),
                accessible_node(first, Some(root), true),
                accessible_node(second, Some(root), true),
                AccessibilityNode {
                    enabled: false,
                    ..accessible_node(disabled, Some(root), true)
                },
                accessible_node(outside, None, true),
            ],
            focus_order: vec![outside, disabled, first, second],
            modal_scope: Some(root),
        };

        assert_eq!(tree.focus_order_for_scope(root), vec![first, second]);
        assert_eq!(tree.effective_focus_order(), vec![first, second]);
        assert_eq!(
            tree.next_focus(Some(first), FocusNavigationDirection::Forward, true),
            Some(second)
        );
        assert_eq!(
            tree.next_focus(Some(second), FocusNavigationDirection::Forward, true),
            Some(first)
        );
        assert_eq!(
            tree.next_focus(Some(second), FocusNavigationDirection::Forward, false),
            None
        );
        assert_eq!(
            tree.move_focus_request(
                Some(first),
                FocusNavigationDirection::Forward,
                true,
                FocusRestoreTarget::Previous,
            ),
            Some(AccessibilityAdapterRequest::MoveFocus {
                target: second,
                restore: FocusRestoreTarget::Previous,
            })
        );
    }

    #[test]
    fn capabilities_gate_adapter_request_kinds() {
        let caps = AccessibilityCapabilities::SCREEN_READER;
        assert!(caps.supports(AccessibilityRequestKind::PublishTree));
        assert!(caps.supports(AccessibilityRequestKind::SetFocusTrap));
        assert!(!caps.screenshots);

        let request =
            AccessibilityAdapterRequest::Announce(AccessibilityAnnouncement::assertive("Saved"));
        assert_eq!(request.kind(), AccessibilityRequestKind::Announce);
        assert!(caps.supports(request.kind()));
        assert!(!AccessibilityCapabilities::NONE.supports(request.kind()));
    }

    #[test]
    fn adapter_trait_routes_typed_requests_by_capability() {
        let mut adapter = RecordingAdapter {
            capabilities: AccessibilityCapabilities {
                announcements: true,
                ..AccessibilityCapabilities::NONE
            },
            handled: Vec::new(),
        };

        let announce =
            AccessibilityAdapterRequest::Announce(AccessibilityAnnouncement::polite("Ready"));
        let publish = AccessibilityAdapterRequest::PublishTree {
            tree: AccessibilityTree::default(),
            focused: None,
            preferences: AccessibilityPreferences::DEFAULT,
        };

        assert_eq!(
            adapter.handle_accessibility_request(announce),
            AccessibilityAdapterResponse::Applied
        );
        assert_eq!(
            adapter.handle_accessibility_request(publish),
            AccessibilityAdapterResponse::Unsupported(AccessibilityRequestKind::PublishTree)
        );
        assert_eq!(adapter.handled, vec![AccessibilityRequestKind::Announce]);
    }

    #[test]
    fn accessibility_request_plan_splits_supported_and_unsupported_requests() {
        let target = UiNodeId(7);
        let requests = vec![
            AccessibilityAdapterRequest::Announce(AccessibilityAnnouncement::polite("Ready")),
            AccessibilityAdapterRequest::MoveFocus {
                target,
                restore: FocusRestoreTarget::Previous,
            },
            AccessibilityAdapterRequest::PublishTree {
                tree: AccessibilityTree::default(),
                focused: Some(target),
                preferences: AccessibilityPreferences::DEFAULT,
            },
        ];
        let capabilities = AccessibilityCapabilities {
            announcements: true,
            focus_restore: true,
            ..AccessibilityCapabilities::NONE
        };

        let plan = AccessibilityAdapterRequestPlan::from_requests(&requests, capabilities);

        assert!(!plan.is_fully_supported());
        assert_eq!(
            plan.supported_requests,
            vec![requests[0].clone(), requests[1].clone()]
        );
        assert_eq!(
            plan.unsupported_responses,
            vec![AccessibilityAdapterResponse::Unsupported(
                AccessibilityRequestKind::PublishTree,
            )]
        );
        assert_eq!(plan.supported_count(AccessibilityRequestKind::Announce), 1);
        assert_eq!(plan.supported_count(AccessibilityRequestKind::MoveFocus), 1);
        assert_eq!(
            plan.unsupported_count(AccessibilityRequestKind::PublishTree),
            1
        );
    }

    #[test]
    fn live_region_snapshots_diff_into_supported_announcements() {
        let status = UiNodeId(1);
        let alert = UiNodeId(2);
        let ignored = UiNodeId(3);
        let previous = AccessibilityTree {
            nodes: vec![
                AccessibilityNode {
                    label: Some("Status".to_string()),
                    value: Some("Ready".to_string()),
                    live_region: AccessibilityLiveRegion::Polite,
                    ..accessible_node(status, None, false)
                },
                AccessibilityNode {
                    label: Some("Alert".to_string()),
                    summary: Some(
                        AccessibilitySummary::new("Warning").description("Pressure high"),
                    ),
                    live_region: AccessibilityLiveRegion::Assertive,
                    ..accessible_node(alert, None, false)
                },
                AccessibilityNode {
                    label: Some("Debug".to_string()),
                    value: Some("unchanged".to_string()),
                    live_region: AccessibilityLiveRegion::Off,
                    ..accessible_node(ignored, None, false)
                },
            ],
            focus_order: Vec::new(),
            modal_scope: None,
        };
        let current = AccessibilityTree {
            nodes: vec![
                AccessibilityNode {
                    label: Some("Status".to_string()),
                    value: Some("Running".to_string()),
                    live_region: AccessibilityLiveRegion::Polite,
                    ..accessible_node(status, None, false)
                },
                AccessibilityNode {
                    label: Some("Alert".to_string()),
                    summary: Some(
                        AccessibilitySummary::new("Warning").description("Pressure critical"),
                    ),
                    live_region: AccessibilityLiveRegion::Assertive,
                    ..accessible_node(alert, None, false)
                },
                AccessibilityNode {
                    label: Some("Debug".to_string()),
                    value: Some("changed".to_string()),
                    live_region: AccessibilityLiveRegion::Off,
                    ..accessible_node(ignored, None, false)
                },
            ],
            focus_order: Vec::new(),
            modal_scope: None,
        };

        let previous = AccessibilityLiveRegionSnapshot::from_tree(&previous);
        let current = AccessibilityLiveRegionSnapshot::from_tree(&current);
        let mut queue = AccessibilityAnnouncementQueue::from_live_region_diff(&previous, &current);

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pending[0].source, Some(alert));
        assert_eq!(queue.pending[0].message, "Warning. Pressure critical");
        assert_eq!(
            queue.pending[0].live_region,
            AccessibilityLiveRegion::Assertive
        );
        assert!(queue.pending[0].interrupt);
        assert_eq!(queue.pending[1].source, Some(status));
        assert_eq!(queue.pending[1].message, "Status: Running");
        assert_eq!(
            queue.pending[1].live_region,
            AccessibilityLiveRegion::Polite
        );
        assert!(!queue.pending[1].interrupt);
        assert_eq!(
            queue.supported_requests(AccessibilityCapabilities::NONE),
            Vec::<AccessibilityAdapterRequest>::new()
        );
        assert_eq!(
            queue
                .supported_requests(AccessibilityCapabilities::SCREEN_READER)
                .len(),
            2
        );
        assert_eq!(queue.drain().len(), 2);
        assert!(queue.is_empty());
    }

    #[test]
    fn announcement_queue_prioritizes_assertive_messages() {
        let mut queue = AccessibilityAnnouncementQueue::new();
        queue.push(AccessibilityAnnouncement::polite("Save complete").source(UiNodeId(1)));
        queue.push(AccessibilityAnnouncement::assertive("Connection lost").source(UiNodeId(2)));
        queue.push(AccessibilityAnnouncement::polite("Sync complete").source(UiNodeId(3)));

        assert_eq!(queue.pending[0].message, "Connection lost");
        assert_eq!(
            queue.pending[0].live_region,
            AccessibilityLiveRegion::Assertive
        );
        assert_eq!(queue.pending[1].message, "Save complete");
        assert_eq!(queue.pending[2].message, "Sync complete");
    }

    #[test]
    fn preferences_fold_host_flags_into_policy_helpers() {
        let preferences = AccessibilityPreferences::DEFAULT
            .screen_reader_active(true)
            .forced_colors(true)
            .reduced_transparency(true)
            .text_scale(4.0);

        assert!(preferences.should_reduce_motion());
        assert!(preferences.should_use_high_contrast());
        assert!(preferences.prefers_reduced_transparency());
        assert_eq!(
            preferences.normalized_text_scale(),
            AccessibilityPreferences::MAX_TEXT_SCALE
        );
        assert_eq!(
            AccessibilityPreferences::DEFAULT
                .text_scale(f32::NAN)
                .normalized_text_scale(),
            1.0
        );
    }
}
