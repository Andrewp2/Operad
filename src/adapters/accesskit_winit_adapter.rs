//! Optional AccessKit bridge for winit hosts.
//!
//! This module is intentionally backend-specific. Core Operad widgets publish
//! `AccessibilityTree`; winit hosts can opt into this feature to expose that
//! tree through the platform AccessKit adapter.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
};

use accesskit::{
    Action as AccessKitAction, ActionHandler, ActionRequest, DeactivationHandler,
    Invalid as AccessKitInvalid, Live as AccessKitLive, Node as AccessKitNode,
    NodeId as AccessKitNodeId, Rect as AccessKitRect, Role as AccessKitRole,
    SortDirection as AccessKitSortDirection, Toggled as AccessKitToggled, Tree as AccessKitTree,
    TreeUpdate,
};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};

use crate::{
    AccessibilityAdapter, AccessibilityAdapterRequest, AccessibilityAdapterResponse,
    AccessibilityCapabilities, AccessibilityChecked, AccessibilityLiveRegion, AccessibilityNode,
    AccessibilityRole, AccessibilitySortDirection, AccessibilityTree, FocusRestoreTarget, UiNodeId,
    UiRect,
};

pub const ACCESSKIT_ROOT_NODE_ID: AccessKitNodeId = AccessKitNodeId(0);

pub const ACCESSKIT_WINIT_CAPABILITIES: AccessibilityCapabilities = AccessibilityCapabilities {
    screen_reader_tree: true,
    focus_restore: true,
    focus_trap: false,
    announcements: false,
    live_regions: true,
    preferences: false,
    reduced_motion: false,
    high_contrast: false,
    clipboard: false,
    text_ime: false,
    drag_drop: false,
    screenshots: false,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessKitTreeOptions {
    pub toolkit_name: Option<String>,
    pub toolkit_version: Option<String>,
    pub root_label: Option<String>,
}

impl AccessKitTreeOptions {
    pub fn operad() -> Self {
        Self {
            toolkit_name: Some(env!("CARGO_PKG_NAME").to_string()),
            toolkit_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            root_label: Some("Operad application".to_string()),
        }
    }

    pub fn toolkit_name(mut self, name: impl Into<String>) -> Self {
        self.toolkit_name = Some(name.into());
        self
    }

    pub fn without_toolkit_name(mut self) -> Self {
        self.toolkit_name = None;
        self
    }

    pub fn toolkit_version(mut self, version: impl Into<String>) -> Self {
        self.toolkit_version = Some(version.into());
        self
    }

    pub fn without_toolkit_version(mut self) -> Self {
        self.toolkit_version = None;
        self
    }

    pub fn root_label(mut self, label: impl Into<String>) -> Self {
        self.root_label = Some(label.into());
        self
    }

    pub fn without_root_label(mut self) -> Self {
        self.root_label = None;
        self
    }
}

impl Default for AccessKitTreeOptions {
    fn default() -> Self {
        Self::operad()
    }
}

pub const fn accesskit_node_id(node: UiNodeId) -> AccessKitNodeId {
    AccessKitNodeId(node.0 as u64 + 1)
}

pub fn operad_node_id(node: AccessKitNodeId) -> Option<UiNodeId> {
    if node.0 == 0 {
        return None;
    }
    usize::try_from(node.0 - 1).ok().map(UiNodeId)
}

pub fn accesskit_tree_update(
    tree: &AccessibilityTree,
    focused: Option<UiNodeId>,
    options: AccessKitTreeOptions,
) -> TreeUpdate {
    let node_ids = tree
        .nodes
        .iter()
        .map(|node| node.id)
        .collect::<HashSet<_>>();
    let mut children_by_parent = HashMap::<UiNodeId, Vec<AccessKitNodeId>>::new();
    let mut root_children = Vec::new();

    for node in &tree.nodes {
        let child = accesskit_node_id(node.id);
        match node.parent.filter(|parent| node_ids.contains(parent)) {
            Some(parent) => children_by_parent.entry(parent).or_default().push(child),
            None => root_children.push(child),
        }
    }

    let mut root = AccessKitNode::new(AccessKitRole::Window);
    root.set_children(root_children);
    if let Some(label) = non_empty_owned(options.root_label) {
        root.set_label(label);
    }

    let mut nodes = Vec::with_capacity(tree.nodes.len() + 1);
    nodes.push((ACCESSKIT_ROOT_NODE_ID, root));
    for node in &tree.nodes {
        let children = children_by_parent.remove(&node.id).unwrap_or_default();
        nodes.push((
            accesskit_node_id(node.id),
            accesskit_node(tree, node, &node_ids, children),
        ));
    }

    let mut accesskit_tree = AccessKitTree::new(ACCESSKIT_ROOT_NODE_ID);
    accesskit_tree.toolkit_name = non_empty_owned(options.toolkit_name);
    accesskit_tree.toolkit_version = non_empty_owned(options.toolkit_version);

    TreeUpdate {
        nodes,
        tree: Some(accesskit_tree),
        focus: accesskit_focus_id(tree, focused),
    }
}

pub struct AccessKitWinitAdapter {
    adapter: accesskit_winit::Adapter,
    shared: Arc<Mutex<AccessKitWinitShared>>,
    tree: AccessibilityTree,
    options: AccessKitTreeOptions,
    focused: Option<UiNodeId>,
    previous_focus: Option<UiNodeId>,
}

impl AccessKitWinitAdapter {
    pub fn new(
        event_loop: &ActiveEventLoop,
        window: &Window,
        tree: AccessibilityTree,
        focused: Option<UiNodeId>,
        options: AccessKitTreeOptions,
    ) -> Self {
        let focused = normalize_focus(&tree, focused);
        let initial_update = accesskit_tree_update(&tree, focused, options.clone());
        let shared = Arc::new(Mutex::new(AccessKitWinitShared {
            latest_full_update: Some(initial_update),
            ..AccessKitWinitShared::default()
        }));
        let adapter = accesskit_winit::Adapter::with_direct_handlers(
            event_loop,
            window,
            AccessKitActivationHandler {
                shared: shared.clone(),
            },
            AccessKitActionHandler {
                shared: shared.clone(),
            },
            AccessKitDeactivationHandler {
                shared: shared.clone(),
            },
        );
        Self {
            adapter,
            shared,
            tree,
            options,
            focused,
            previous_focus: None,
        }
    }

    pub fn process_event(&mut self, window: &Window, event: &WindowEvent) {
        self.adapter.process_event(window, event);
    }

    pub fn publish_tree(
        &mut self,
        tree: AccessibilityTree,
        focused: Option<UiNodeId>,
    ) -> AccessibilityAdapterResponse {
        self.previous_focus = self.focused;
        self.focused = normalize_focus(&tree, focused);
        self.tree = tree;
        let update = self.full_tree_update();
        lock_shared(&self.shared).latest_full_update = Some(update.clone());
        self.adapter.update_if_active(|| update);
        AccessibilityAdapterResponse::Applied
    }

    pub fn publish_focus(&mut self, focused: Option<UiNodeId>) -> AccessibilityAdapterResponse {
        self.previous_focus = self.focused;
        self.focused = normalize_focus(&self.tree, focused);
        let full_update = self.full_tree_update();
        let focus_update = TreeUpdate {
            nodes: Vec::new(),
            tree: None,
            focus: full_update.focus,
        };
        lock_shared(&self.shared).latest_full_update = Some(full_update);
        self.adapter.update_if_active(|| focus_update);
        AccessibilityAdapterResponse::FocusChanged(self.focused)
    }

    pub fn set_options(&mut self, options: AccessKitTreeOptions) {
        self.options = options;
        let update = self.full_tree_update();
        lock_shared(&self.shared).latest_full_update = Some(update.clone());
        self.adapter.update_if_active(|| update);
    }

    pub fn take_action_requests(&mut self) -> Vec<ActionRequest> {
        std::mem::take(&mut lock_shared(&self.shared).action_requests)
    }

    pub fn deactivation_count(&self) -> usize {
        lock_shared(&self.shared).deactivation_count
    }

    fn full_tree_update(&self) -> TreeUpdate {
        accesskit_tree_update(&self.tree, self.focused, self.options.clone())
    }

    fn restore_focus_target(&self, restore: FocusRestoreTarget) -> Option<UiNodeId> {
        match restore {
            FocusRestoreTarget::None => None,
            FocusRestoreTarget::Previous => self.previous_focus.or(self.focused),
            FocusRestoreTarget::Node(node) => Some(node),
        }
    }
}

impl AccessibilityAdapter for AccessKitWinitAdapter {
    fn accessibility_capabilities(&self) -> AccessibilityCapabilities {
        ACCESSKIT_WINIT_CAPABILITIES
    }

    fn handle_accessibility_request(
        &mut self,
        request: AccessibilityAdapterRequest,
    ) -> AccessibilityAdapterResponse {
        let kind = request.kind();
        if !self.accessibility_capabilities().supports(kind) {
            return AccessibilityAdapterResponse::Unsupported(kind);
        }

        match request {
            AccessibilityAdapterRequest::PublishTree {
                tree,
                focused,
                preferences: _,
            } => self.publish_tree(tree, focused),
            AccessibilityAdapterRequest::MoveFocus { target, .. } => {
                self.publish_focus(Some(target))
            }
            AccessibilityAdapterRequest::RestoreFocus(restore) => {
                self.publish_focus(self.restore_focus_target(restore))
            }
            AccessibilityAdapterRequest::SetFocusTrap(_)
            | AccessibilityAdapterRequest::ClearFocusTrap { .. }
            | AccessibilityAdapterRequest::Announce(_)
            | AccessibilityAdapterRequest::ApplyPreferences(_) => {
                AccessibilityAdapterResponse::Unsupported(kind)
            }
        }
    }
}

#[derive(Debug, Default)]
struct AccessKitWinitShared {
    latest_full_update: Option<TreeUpdate>,
    action_requests: Vec<ActionRequest>,
    deactivation_count: usize,
}

#[derive(Clone)]
struct AccessKitActivationHandler {
    shared: Arc<Mutex<AccessKitWinitShared>>,
}

impl accesskit::ActivationHandler for AccessKitActivationHandler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        lock_shared(&self.shared).latest_full_update.clone()
    }
}

#[derive(Clone)]
struct AccessKitActionHandler {
    shared: Arc<Mutex<AccessKitWinitShared>>,
}

impl ActionHandler for AccessKitActionHandler {
    fn do_action(&mut self, request: ActionRequest) {
        lock_shared(&self.shared).action_requests.push(request);
    }
}

#[derive(Clone)]
struct AccessKitDeactivationHandler {
    shared: Arc<Mutex<AccessKitWinitShared>>,
}

impl DeactivationHandler for AccessKitDeactivationHandler {
    fn deactivate_accessibility(&mut self) {
        lock_shared(&self.shared).deactivation_count += 1;
    }
}

fn accesskit_node(
    tree: &AccessibilityTree,
    node: &AccessibilityNode,
    node_ids: &HashSet<UiNodeId>,
    children: Vec<AccessKitNodeId>,
) -> AccessKitNode {
    let mut accesskit_node = AccessKitNode::new(accesskit_role(node.role));
    accesskit_node.set_bounds(accesskit_rect(node.rect));
    accesskit_node.set_children(children);

    if let Some(name) = tree.accessible_name(node.id) {
        if node.role == AccessibilityRole::Label {
            accesskit_node.set_value(name);
        } else {
            accesskit_node.set_label(name);
        }
    }
    if let Some(description) = tree.accessible_description(node.id) {
        accesskit_node.set_description(description);
    }
    if let Some(value) = non_empty_ref(node.value.as_deref()) {
        if node.role != AccessibilityRole::Label || accesskit_node.value().is_none() {
            accesskit_node.set_value(value.to_string());
        }
    }

    if !node.enabled {
        accesskit_node.set_disabled();
    }
    if node.modal {
        accesskit_node.set_modal();
    }
    if node.read_only {
        accesskit_node.set_read_only();
    }
    if node.required {
        accesskit_node.set_required();
    }
    if node.invalid.is_some() {
        accesskit_node.set_invalid(AccessKitInvalid::True);
    }
    if node.focusable {
        accesskit_node.add_action(AccessKitAction::Focus);
    }
    if supports_click(node) {
        accesskit_node.add_action(AccessKitAction::Click);
    }
    if let Some(selected) = node.selected {
        accesskit_node.set_selected(selected);
    }
    if let Some(expanded) = node.expanded {
        accesskit_node.set_expanded(expanded);
    }
    if let Some(checked) = node.checked {
        accesskit_node.set_toggled(accesskit_toggled(checked));
    } else if let Some(pressed) = node.pressed {
        accesskit_node.set_toggled(AccessKitToggled::from(pressed));
    }
    if let Some(live) = accesskit_live(node.live_region) {
        accesskit_node.set_live(live);
    }
    if let Some(sort) = accesskit_sort(node.sort) {
        accesskit_node.set_sort_direction(sort);
    }
    if let Some(range) = node.value_range {
        accesskit_node.set_min_numeric_value(range.min);
        accesskit_node.set_max_numeric_value(range.max);
        if let Some(step) = range.step {
            accesskit_node.set_numeric_value_step(step);
        }
        if let Some(value) = node.value.as_deref().and_then(parse_numeric_value) {
            accesskit_node.set_numeric_value(value);
        }
    }
    if !node.key_shortcuts.is_empty() {
        accesskit_node.set_keyboard_shortcut(node.key_shortcuts.join(", "));
    }

    let labelled_by = relation_nodes(&node.relations.labelled_by, node_ids);
    if !labelled_by.is_empty() {
        accesskit_node.set_labelled_by(labelled_by);
    }
    let described_by = relation_nodes(&node.relations.described_by, node_ids);
    if !described_by.is_empty() {
        accesskit_node.set_described_by(described_by);
    }
    let controls = relation_nodes(&node.relations.controls, node_ids);
    if !controls.is_empty() {
        accesskit_node.set_controls(controls);
    }
    let owns = relation_nodes(&node.relations.owns, node_ids);
    if !owns.is_empty() {
        accesskit_node.set_owns(owns);
    }
    if let Some(active_descendant) = node
        .relations
        .active_descendant
        .filter(|id| node_ids.contains(id))
    {
        accesskit_node.set_active_descendant(accesskit_node_id(active_descendant));
    }

    accesskit_node
}

fn accesskit_role(role: AccessibilityRole) -> AccessKitRole {
    match role {
        AccessibilityRole::Alert => AccessKitRole::Alert,
        AccessibilityRole::Application => AccessKitRole::Application,
        AccessibilityRole::Button => AccessKitRole::Button,
        AccessibilityRole::Checkbox => AccessKitRole::CheckBox,
        AccessibilityRole::ColumnHeader => AccessKitRole::ColumnHeader,
        AccessibilityRole::ComboBox => AccessKitRole::ComboBox,
        AccessibilityRole::Dialog => AccessKitRole::Dialog,
        AccessibilityRole::EditorSurface => AccessKitRole::Document,
        AccessibilityRole::Group => AccessKitRole::Group,
        AccessibilityRole::Grid => AccessKitRole::Grid,
        AccessibilityRole::GridCell => AccessKitRole::Cell,
        AccessibilityRole::Image => AccessKitRole::Image,
        AccessibilityRole::Label => AccessKitRole::Label,
        AccessibilityRole::Link => AccessKitRole::Link,
        AccessibilityRole::List => AccessKitRole::List,
        AccessibilityRole::ListItem => AccessKitRole::ListItem,
        AccessibilityRole::Meter => AccessKitRole::Meter,
        AccessibilityRole::Menu => AccessKitRole::Menu,
        AccessibilityRole::MenuBar => AccessKitRole::MenuBar,
        AccessibilityRole::MenuItem => AccessKitRole::MenuItem,
        AccessibilityRole::ProgressBar => AccessKitRole::ProgressIndicator,
        AccessibilityRole::RadioButton => AccessKitRole::RadioButton,
        AccessibilityRole::Row => AccessKitRole::Row,
        AccessibilityRole::RowHeader => AccessKitRole::RowHeader,
        AccessibilityRole::Ruler => AccessKitRole::GenericContainer,
        AccessibilityRole::SearchBox => AccessKitRole::SearchInput,
        AccessibilityRole::Separator => AccessKitRole::GenericContainer,
        AccessibilityRole::Slider => AccessKitRole::Slider,
        AccessibilityRole::SpinButton => AccessKitRole::SpinButton,
        AccessibilityRole::Splitter => AccessKitRole::Splitter,
        AccessibilityRole::Status => AccessKitRole::Status,
        AccessibilityRole::Switch => AccessKitRole::Switch,
        AccessibilityRole::Tab => AccessKitRole::Tab,
        AccessibilityRole::TabList => AccessKitRole::TabList,
        AccessibilityRole::TabPanel => AccessKitRole::TabPanel,
        AccessibilityRole::TextBox => AccessKitRole::TextInput,
        AccessibilityRole::ToggleButton => AccessKitRole::Button,
        AccessibilityRole::Toolbar => AccessKitRole::Toolbar,
        AccessibilityRole::Tooltip => AccessKitRole::Tooltip,
        AccessibilityRole::Tree => AccessKitRole::Tree,
        AccessibilityRole::TreeItem => AccessKitRole::TreeItem,
        AccessibilityRole::Window => AccessKitRole::Window,
    }
}

fn accesskit_rect(rect: UiRect) -> AccessKitRect {
    AccessKitRect::new(
        rect.x as f64,
        rect.y as f64,
        rect.right() as f64,
        rect.bottom() as f64,
    )
}

fn accesskit_focus_id(tree: &AccessibilityTree, focused: Option<UiNodeId>) -> AccessKitNodeId {
    normalize_focus(tree, focused)
        .map(accesskit_node_id)
        .unwrap_or(ACCESSKIT_ROOT_NODE_ID)
}

fn normalize_focus(tree: &AccessibilityTree, focused: Option<UiNodeId>) -> Option<UiNodeId> {
    focused.filter(|id| tree.node(*id).is_some())
}

fn accesskit_toggled(checked: AccessibilityChecked) -> AccessKitToggled {
    match checked {
        AccessibilityChecked::False => AccessKitToggled::False,
        AccessibilityChecked::True => AccessKitToggled::True,
        AccessibilityChecked::Mixed => AccessKitToggled::Mixed,
    }
}

fn accesskit_live(live: AccessibilityLiveRegion) -> Option<AccessKitLive> {
    match live {
        AccessibilityLiveRegion::Off => None,
        AccessibilityLiveRegion::Polite => Some(AccessKitLive::Polite),
        AccessibilityLiveRegion::Assertive => Some(AccessKitLive::Assertive),
    }
}

fn accesskit_sort(sort: AccessibilitySortDirection) -> Option<AccessKitSortDirection> {
    match sort {
        AccessibilitySortDirection::None => None,
        AccessibilitySortDirection::Ascending => Some(AccessKitSortDirection::Ascending),
        AccessibilitySortDirection::Descending => Some(AccessKitSortDirection::Descending),
        AccessibilitySortDirection::Other => Some(AccessKitSortDirection::Other),
    }
}

fn relation_nodes(ids: &[UiNodeId], node_ids: &HashSet<UiNodeId>) -> Vec<AccessKitNodeId> {
    ids.iter()
        .copied()
        .filter(|id| node_ids.contains(id))
        .map(accesskit_node_id)
        .collect()
}

fn supports_click(node: &AccessibilityNode) -> bool {
    !node.actions.is_empty()
        || matches!(
            node.role,
            AccessibilityRole::Button
                | AccessibilityRole::Checkbox
                | AccessibilityRole::Link
                | AccessibilityRole::MenuItem
                | AccessibilityRole::RadioButton
                | AccessibilityRole::Switch
                | AccessibilityRole::ToggleButton
        )
}

fn parse_numeric_value(value: &str) -> Option<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn non_empty_owned(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn non_empty_ref(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn lock_shared(shared: &Arc<Mutex<AccessKitWinitShared>>) -> MutexGuard<'_, AccessKitWinitShared> {
    shared.lock().unwrap_or_else(|error| error.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessibilityAction, AccessibilityRelations, AccessibilityValueRange};

    fn node(id: usize, role: AccessibilityRole, label: &str) -> AccessibilityNode {
        AccessibilityNode {
            id: UiNodeId(id),
            parent: None,
            role,
            label: Some(label.to_string()),
            value: None,
            hint: None,
            rect: UiRect::new(id as f32, 2.0, 10.0, 12.0),
            enabled: true,
            focusable: false,
            modal: false,
            selected: None,
            checked: None,
            expanded: None,
            pressed: None,
            read_only: false,
            required: false,
            invalid: None,
            live_region: AccessibilityLiveRegion::Off,
            sort: AccessibilitySortDirection::None,
            value_range: None,
            focus_order: None,
            key_shortcuts: Vec::new(),
            actions: Vec::new(),
            relations: AccessibilityRelations::default(),
            summary: None,
        }
    }

    fn update_node(update: &TreeUpdate, id: AccessKitNodeId) -> &AccessKitNode {
        update
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == id).then_some(node))
            .expect("accesskit node exists")
    }

    #[test]
    fn accesskit_tree_update_adds_synthetic_root_and_focus() {
        let mut label = node(1, AccessibilityRole::Label, "Primary action");
        let mut button = node(2, AccessibilityRole::Button, "Save");
        button.parent = Some(label.id);
        button.focusable = true;
        button.hint = Some("Writes the file".to_string());
        button
            .actions
            .push(AccessibilityAction::new("save", "Save"));
        button.relations.labelled_by.push(label.id);
        label.relations.controls.push(button.id);

        let tree = AccessibilityTree {
            nodes: vec![label, button],
            focus_order: vec![UiNodeId(2)],
            modal_scope: None,
        };
        let update =
            accesskit_tree_update(&tree, Some(UiNodeId(2)), AccessKitTreeOptions::default());

        assert_eq!(update.tree.as_ref().unwrap().root, ACCESSKIT_ROOT_NODE_ID);
        assert_eq!(update.focus, accesskit_node_id(UiNodeId(2)));
        assert_eq!(
            update_node(&update, ACCESSKIT_ROOT_NODE_ID).children(),
            &[accesskit_node_id(UiNodeId(1))]
        );
        assert_eq!(
            update_node(&update, accesskit_node_id(UiNodeId(1))).children(),
            &[accesskit_node_id(UiNodeId(2))]
        );

        let button = update_node(&update, accesskit_node_id(UiNodeId(2)));
        assert_eq!(button.role(), AccessKitRole::Button);
        assert_eq!(button.label(), Some("Primary action"));
        assert_eq!(button.description(), Some("Writes the file"));
        assert_eq!(
            button.bounds(),
            Some(AccessKitRect::new(2.0, 2.0, 12.0, 14.0))
        );
        assert!(button.supports_action(AccessKitAction::Focus));
        assert!(button.supports_action(AccessKitAction::Click));
        assert_eq!(button.labelled_by(), &[accesskit_node_id(UiNodeId(1))]);
    }

    #[test]
    fn accesskit_tree_update_maps_state_value_and_relations() {
        let mut slider = node(4, AccessibilityRole::Slider, "Volume");
        slider.focusable = true;
        slider.enabled = false;
        slider.value = Some("42".to_string());
        slider.checked = Some(AccessibilityChecked::Mixed);
        slider.selected = Some(true);
        slider.expanded = Some(false);
        slider.modal = true;
        slider.read_only = true;
        slider.required = true;
        slider.invalid = Some("Out of range".to_string());
        slider.live_region = AccessibilityLiveRegion::Assertive;
        slider.sort = AccessibilitySortDirection::Descending;
        slider.value_range = Some(AccessibilityValueRange::new(0.0, 100.0).with_step(5.0));
        slider.key_shortcuts.push("Ctrl+V".to_string());

        let tree = AccessibilityTree {
            nodes: vec![slider],
            focus_order: vec![UiNodeId(4)],
            modal_scope: None,
        };
        let update =
            accesskit_tree_update(&tree, Some(UiNodeId(404)), AccessKitTreeOptions::default());
        let slider = update_node(&update, accesskit_node_id(UiNodeId(4)));

        assert_eq!(update.focus, ACCESSKIT_ROOT_NODE_ID);
        assert!(slider.is_disabled());
        assert!(slider.is_modal());
        assert!(slider.is_read_only());
        assert!(slider.is_required());
        assert_eq!(slider.invalid(), Some(AccessKitInvalid::True));
        assert_eq!(slider.toggled(), Some(AccessKitToggled::Mixed));
        assert_eq!(slider.is_selected(), Some(true));
        assert_eq!(slider.is_expanded(), Some(false));
        assert_eq!(slider.live(), Some(AccessKitLive::Assertive));
        assert_eq!(
            slider.sort_direction(),
            Some(AccessKitSortDirection::Descending)
        );
        assert_eq!(slider.numeric_value(), Some(42.0));
        assert_eq!(slider.min_numeric_value(), Some(0.0));
        assert_eq!(slider.max_numeric_value(), Some(100.0));
        assert_eq!(slider.numeric_value_step(), Some(5.0));
        assert_eq!(slider.keyboard_shortcut(), Some("Ctrl+V"));
    }
}
