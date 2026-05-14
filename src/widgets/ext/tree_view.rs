//! Tree view and outliner widget implementation.

use std::collections::HashSet;

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, LengthPercentage, LengthPercentageAuto,
    Rect as TaffyRect, Size as TaffySize, Style,
};

use crate::{
    platform::{DragOperation, DragPayload},
    AccessibilityAction, AccessibilityMeta, AccessibilityRole, ClipBehavior, ColorRgba, CommandId,
    DragDropSurfaceKind, DragSourceDescriptor, DragSourceId, DropPayloadFilter,
    DropTargetDescriptor, DropTargetId, ImageContent, InputBehavior, LayoutStyle, ShaderEffect,
    StrokeStyle, TextStyle, TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiRect, UiVisual,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeItem {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeItem>,
    pub disabled: bool,
    pub leading_image: Option<ImageContent>,
    pub row_actions: Vec<TreeRowAction>,
    pub context_menu_commands: Vec<CommandId>,
    pub draggable: bool,
    pub drop_policy: Option<TreeItemDropPolicy>,
}

impl TreeItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            disabled: false,
            leading_image: None,
            row_actions: Vec::new(),
            context_menu_commands: Vec::new(),
            draggable: false,
            drop_policy: None,
        }
    }

    pub fn with_children(mut self, children: Vec<TreeItem>) -> Self {
        self.children = children;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn with_leading_image(mut self, image: ImageContent) -> Self {
        self.leading_image = Some(image);
        self
    }

    pub fn with_row_action(mut self, action: TreeRowAction) -> Self {
        self.row_actions.push(action);
        self
    }

    pub fn with_row_actions(mut self, actions: impl IntoIterator<Item = TreeRowAction>) -> Self {
        self.row_actions.extend(actions);
        self
    }

    pub fn with_context_menu_command(mut self, command: impl Into<CommandId>) -> Self {
        self.context_menu_commands.push(command.into());
        self
    }

    pub fn with_context_menu_commands(
        mut self,
        commands: impl IntoIterator<Item = impl Into<CommandId>>,
    ) -> Self {
        self.context_menu_commands
            .extend(commands.into_iter().map(Into::into));
        self
    }

    pub const fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    pub fn with_drop_policy(mut self, policy: TreeItemDropPolicy) -> Self {
        self.drop_policy = Some(policy);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRowAction {
    pub id: CommandId,
    pub label: String,
    pub disabled: bool,
    pub destructive: bool,
    pub leading_image: Option<ImageContent>,
}

impl TreeRowAction {
    pub fn new(id: impl Into<CommandId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            disabled: false,
            destructive: false,
            leading_image: None,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    pub fn with_leading_image(mut self, image: ImageContent) -> Self {
        self.leading_image = Some(image);
        self
    }

    pub fn accessibility_action(&self) -> AccessibilityAction {
        AccessibilityAction::new(self.id.as_str(), self.label.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TreeDropPlacement {
    Before,
    On,
    Inside,
    After,
}

impl TreeDropPlacement {
    pub const ALL: [Self; 4] = [Self::Before, Self::On, Self::Inside, Self::After];

    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::On => "on",
            Self::Inside => "inside",
            Self::After => "after",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::On => "on",
            Self::Inside => "inside",
            Self::After => "after",
        }
    }

    pub fn bounds(self, row_bounds: UiRect) -> UiRect {
        let edge_height = (row_bounds.height * 0.25).max(1.0).min(row_bounds.height);
        match self {
            Self::Before => UiRect::new(row_bounds.x, row_bounds.y, row_bounds.width, edge_height),
            Self::After => UiRect::new(
                row_bounds.x,
                row_bounds.bottom() - edge_height,
                row_bounds.width,
                edge_height,
            ),
            Self::On | Self::Inside => row_bounds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeItemDropPolicy {
    pub accepted_payload: DropPayloadFilter,
    pub accepted_operations: Vec<DragOperation>,
    pub placements: Vec<TreeDropPlacement>,
    pub disabled: bool,
}

impl TreeItemDropPolicy {
    pub fn new(accepted_payload: DropPayloadFilter) -> Self {
        Self {
            accepted_payload,
            accepted_operations: vec![
                DragOperation::Copy,
                DragOperation::Move,
                DragOperation::Link,
            ],
            placements: vec![TreeDropPlacement::On],
            disabled: false,
        }
    }

    pub fn any_payload() -> Self {
        Self::new(DropPayloadFilter::any())
    }

    pub fn accepted_operations(
        mut self,
        operations: impl IntoIterator<Item = DragOperation>,
    ) -> Self {
        self.accepted_operations = operations.into_iter().collect();
        self
    }

    pub fn placements(mut self, placements: impl IntoIterator<Item = TreeDropPlacement>) -> Self {
        self.placements = placements.into_iter().collect();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn allows_placement(&self, placement: TreeDropPlacement) -> bool {
        !self.disabled
            && self.placements.contains(&placement)
            && !self.accepted_operations.is_empty()
            && !self.accepted_payload.is_empty()
    }

    pub fn enabled(&self) -> bool {
        !self.disabled
            && self
                .placements
                .iter()
                .any(|placement| self.allows_placement(*placement))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeVisibleItem {
    pub index: usize,
    pub id: String,
    pub label: String,
    pub leading_image: Option<ImageContent>,
    pub depth: usize,
    pub parent_id: Option<String>,
    pub child_count: usize,
    pub expanded: bool,
    pub disabled: bool,
    pub row_actions: Vec<TreeRowAction>,
    pub context_menu_commands: Vec<CommandId>,
    pub draggable: bool,
    pub drop_policy: Option<TreeItemDropPolicy>,
}

impl TreeVisibleItem {
    pub fn has_children(&self) -> bool {
        self.child_count > 0
    }

    pub fn enabled_row_actions(&self) -> Vec<&TreeRowAction> {
        self.row_actions
            .iter()
            .filter(|action| !action.disabled)
            .collect()
    }

    pub fn has_context_menu(&self) -> bool {
        !self.context_menu_commands.is_empty()
    }

    pub fn drag_source(
        &self,
        bounds: UiRect,
        payload: DragPayload,
        allowed_operations: impl IntoIterator<Item = DragOperation>,
    ) -> Option<DragSourceDescriptor> {
        (!self.disabled && self.draggable).then(|| {
            DragSourceDescriptor::new(
                DragSourceId::new(format!("tree.item.{}", self.id)),
                DragDropSurfaceKind::TreeItem,
                bounds,
                payload,
            )
            .allowed_operations(allowed_operations)
            .label(self.label.clone())
        })
    }

    pub fn drop_target(
        &self,
        bounds: UiRect,
        placement: TreeDropPlacement,
    ) -> Option<DropTargetDescriptor> {
        let policy = self.drop_policy.as_ref()?;
        policy.allows_placement(placement).then(|| {
            DropTargetDescriptor::new(
                DropTargetId::new(format!("tree.item.{}.{}", self.id, placement.suffix())),
                DragDropSurfaceKind::TreeItem,
                placement.bounds(bounds),
            )
            .accepted_payload(policy.accepted_payload.clone())
            .accepted_operations(policy.accepted_operations.clone())
            .label(format!("{} {}", self.label, placement.label()))
        })
    }

    pub fn drop_targets(&self, bounds: UiRect) -> Vec<DropTargetDescriptor> {
        self.drop_policy
            .as_ref()
            .map(|policy| {
                policy
                    .placements
                    .iter()
                    .filter_map(|placement| self.drop_target(bounds, *placement))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TreeViewState {
    pub expanded_ids: Vec<String>,
    pub selected_index: Option<usize>,
}

impl TreeViewState {
    pub fn expanded(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            expanded_ids: ids.into_iter().map(Into::into).collect(),
            selected_index: None,
        }
    }

    pub fn is_expanded(&self, id: &str) -> bool {
        self.expanded_ids.iter().any(|expanded| expanded == id)
    }

    pub fn set_expanded(&mut self, id: impl Into<String>, expanded: bool) {
        let id = id.into();
        if expanded {
            if !self.expanded_ids.iter().any(|existing| existing == &id) {
                self.expanded_ids.push(id);
            }
        } else {
            self.expanded_ids.retain(|existing| existing != &id);
        }
    }

    pub fn toggle_expanded(&mut self, id: impl Into<String>) -> bool {
        let id = id.into();
        let expanded = !self.is_expanded(&id);
        self.set_expanded(id, expanded);
        expanded
    }

    pub fn select(&mut self, selected_index: Option<usize>) {
        self.selected_index = selected_index;
    }

    pub fn visible_items(&self, roots: &[TreeItem]) -> Vec<TreeVisibleItem> {
        let expanded: HashSet<&str> = self.expanded_ids.iter().map(String::as_str).collect();
        let mut visible = Vec::new();
        flatten_tree_items(roots, &expanded, 0, None, &mut visible);
        visible
    }

    pub fn selected_visible_item(&self, roots: &[TreeItem]) -> Option<TreeVisibleItem> {
        let selected_index = self.selected_index?;
        self.visible_items(roots)
            .into_iter()
            .find(|item| item.index == selected_index)
    }

    pub fn activate_visible_item_id(
        &mut self,
        roots: &[TreeItem],
        id: &str,
    ) -> Option<TreeVisibleItem> {
        let item = self
            .visible_items(roots)
            .into_iter()
            .find(|item| item.id == id && !item.disabled)?;
        self.select(Some(item.index));
        if item.has_children() {
            self.toggle_expanded(item.id.clone());
        }
        Some(item)
    }

    pub fn select_next_visible(&mut self, roots: &[TreeItem]) -> Option<usize> {
        let visible = self.visible_items(roots);
        let current = self.selected_index;
        let index = next_enabled_visible_index(&visible, current)?;
        self.select(Some(index));
        Some(index)
    }

    pub fn select_previous_visible(&mut self, roots: &[TreeItem]) -> Option<usize> {
        let visible = self.visible_items(roots);
        let current = self.selected_index;
        let index = previous_enabled_visible_index(&visible, current)?;
        self.select(Some(index));
        Some(index)
    }

    pub fn toggle_selected_expansion(&mut self, roots: &[TreeItem]) -> Option<bool> {
        let selected = self.selected_visible_item(roots)?;
        selected
            .has_children()
            .then(|| self.toggle_expanded(selected.id))
    }
}

#[derive(Debug, Clone)]
pub struct TreeViewOptions {
    pub layout: LayoutStyle,
    pub row_height: f32,
    pub indent_width: f32,
    pub disclosure_width: f32,
    pub focused_index: Option<usize>,
    pub background_visual: UiVisual,
    pub row_visual: UiVisual,
    pub selected_row_visual: UiVisual,
    pub selected_row_shader: Option<ShaderEffect>,
    pub focused_row_shader: Option<ShaderEffect>,
    pub text_style: TextStyle,
    pub muted_text_style: TextStyle,
    pub leading_image_size: f32,
    pub accessibility_label: Option<String>,
    pub row_action_prefix: Option<String>,
}

impl Default for TreeViewOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            row_height: 26.0,
            indent_width: 16.0,
            disclosure_width: 18.0,
            focused_index: None,
            background_visual: UiVisual::panel(
                ColorRgba::new(18, 22, 28, 255),
                Some(StrokeStyle::new(ColorRgba::new(58, 69, 84, 255), 1.0)),
                4.0,
            ),
            row_visual: UiVisual::TRANSPARENT,
            selected_row_visual: UiVisual::panel(ColorRgba::new(41, 59, 82, 255), None, 0.0),
            selected_row_shader: None,
            focused_row_shader: None,
            text_style: TextStyle::default(),
            muted_text_style: muted_text_style(),
            leading_image_size: 16.0,
            accessibility_label: None,
            row_action_prefix: None,
        }
    }
}

impl TreeViewOptions {
    pub fn with_row_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.row_action_prefix = Some(prefix.into());
        self
    }
}

/// Build a tree view/outliner from nested IDs and a visible selection index.
pub fn tree_view(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    roots: &[TreeItem],
    state: &TreeViewState,
    options: TreeViewOptions,
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.background_visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Tree)
                .label(accessibility_label_or_name(
                    &options.accessibility_label,
                    &name,
                ))
                .value(format!(
                    "{} visible items",
                    state.visible_items(roots).len()
                ))
                .focusable(),
        ),
    );

    let visible_items = state.visible_items(roots);
    let visible_count = visible_items.len();
    for item in visible_items {
        let selected = state.selected_index == Some(item.index);
        let focused = options.focused_index == Some(item.index);
        let visual = if selected {
            options.selected_row_visual
        } else {
            options.row_visual
        };
        let mut row_node = UiNode::container(
            format!("{name}.row.{}", item.id),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: px(options.row_height),
                    },
                    padding: TaffyRect {
                        left: LengthPercentage::length(8.0),
                        right: LengthPercentage::length(8.0),
                        top: LengthPercentage::length(0.0),
                        bottom: LengthPercentage::length(0.0),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(if item.disabled {
            InputBehavior::NONE
        } else {
            InputBehavior::BUTTON
        })
        .with_visual(visual)
        .with_accessibility(tree_item_accessibility(
            &item,
            visible_count,
            selected,
            focused,
        ));
        if !item.disabled {
            if let Some(prefix) = options.row_action_prefix.as_deref() {
                row_node = row_node.with_action(format!("{prefix}.row.{}", item.id));
            }
        }
        let row = with_optional_shader(
            row_node,
            if selected {
                options.selected_row_shader.as_ref()
            } else if focused {
                options.focused_row_shader.as_ref()
            } else {
                None
            },
        );
        let row = document.add_child(root, row);

        if item.depth > 0 {
            document.add_child(
                row,
                UiNode::container(
                    format!("{name}.row.{}.indent", item.id),
                    UiNodeStyle {
                        layout: LayoutStyle::from_taffy_style(Style {
                            size: TaffySize {
                                width: px(item.depth as f32 * options.indent_width),
                                height: Dimension::percent(1.0),
                            },
                            flex_shrink: 0.0,
                            ..Default::default()
                        })
                        .style,
                        ..Default::default()
                    },
                ),
            );
        }

        let disclosure = if item.has_children() {
            if item.expanded {
                "v"
            } else {
                ">"
            }
        } else {
            ""
        };
        document.add_child(
            row,
            UiNode::text(
                format!("{name}.row.{}.disclosure", item.id),
                disclosure,
                options.muted_text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: px(options.disclosure_width),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                }),
            ),
        );

        if let Some(image) = item.leading_image.clone() {
            document.add_child(
                row,
                leading_image_node(
                    format!("{name}.row.{}.image", item.id),
                    image,
                    options.leading_image_size,
                    Some(item.label.clone()),
                ),
            );
        }

        let style = if item.disabled {
            options.muted_text_style.clone()
        } else {
            options.text_style.clone()
        };
        document.add_child(
            row,
            UiNode::text(
                format!("{name}.row.{}.label", item.id),
                &item.label,
                style,
                LayoutStyle::from_taffy_style(Style {
                    flex_grow: 1.0,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                }),
            ),
        );
    }

    root
}

pub fn outliner(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    roots: &[TreeItem],
    state: &TreeViewState,
    options: TreeViewOptions,
) -> UiNodeId {
    tree_view(document, parent, name, roots, state, options)
}

fn flatten_tree_items(
    items: &[TreeItem],
    expanded: &HashSet<&str>,
    depth: usize,
    parent_id: Option<&str>,
    visible: &mut Vec<TreeVisibleItem>,
) {
    for item in items {
        let has_children = !item.children.is_empty();
        let is_expanded = has_children && expanded.contains(item.id.as_str());
        let index = visible.len();
        visible.push(TreeVisibleItem {
            index,
            id: item.id.clone(),
            label: item.label.clone(),
            leading_image: item.leading_image.clone(),
            depth,
            parent_id: parent_id.map(str::to_owned),
            child_count: item.children.len(),
            expanded: is_expanded,
            disabled: item.disabled,
            row_actions: item.row_actions.clone(),
            context_menu_commands: item.context_menu_commands.clone(),
            draggable: item.draggable,
            drop_policy: item.drop_policy.clone(),
        });
        if is_expanded {
            flatten_tree_items(&item.children, expanded, depth + 1, Some(&item.id), visible);
        }
    }
}

fn tree_item_accessibility(
    item: &TreeVisibleItem,
    visible_count: usize,
    selected: bool,
    focused: bool,
) -> AccessibilityMeta {
    let mut value = vec![
        format!("item {} of {}", item.index + 1, visible_count),
        format!("level {}", item.depth + 1),
        if item.has_children() {
            format!(
                "{}; {} children",
                if item.expanded {
                    "expanded"
                } else {
                    "collapsed"
                },
                item.child_count
            )
        } else {
            "leaf".to_owned()
        },
    ];
    push_state(&mut value, "selected", selected);
    push_state(&mut value, "focused", focused);
    push_state(&mut value, "disabled", item.disabled);
    push_state(&mut value, "draggable", item.draggable);
    push_state(
        &mut value,
        "drop target",
        item.drop_policy
            .as_ref()
            .is_some_and(TreeItemDropPolicy::enabled),
    );
    if !item.row_actions.is_empty() {
        value.push(format!("{} actions", item.enabled_row_actions().len()));
    }
    push_state(&mut value, "context menu", item.has_context_menu());

    let mut meta = AccessibilityMeta::new(AccessibilityRole::TreeItem)
        .label(item.label.clone())
        .value(value.join("; "))
        .selected(selected)
        .expanded(item.expanded)
        .focusable();
    for action in item.enabled_row_actions() {
        meta = meta.action(action.accessibility_action());
    }
    if item.has_context_menu() {
        meta = meta.action(AccessibilityAction::new(
            "context_menu.open",
            "Open context menu",
        ));
    }
    if item.draggable && !item.disabled {
        meta = meta.action(AccessibilityAction::new("drag.start", "Start drag"));
    }
    if item
        .drop_policy
        .as_ref()
        .is_some_and(TreeItemDropPolicy::enabled)
    {
        meta = meta.action(AccessibilityAction::new("drop.accept", "Accept drop"));
    }
    apply_enabled(meta, !item.disabled)
}

fn next_enabled_visible_index(
    visible: &[TreeVisibleItem],
    current: Option<usize>,
) -> Option<usize> {
    let start = current.and_then(|index| index.checked_add(1)).unwrap_or(0);
    visible
        .iter()
        .find(|item| item.index >= start && !item.disabled)
        .or_else(|| visible.iter().rev().find(|item| !item.disabled))
        .map(|item| item.index)
}

fn previous_enabled_visible_index(
    visible: &[TreeVisibleItem],
    current: Option<usize>,
) -> Option<usize> {
    match current {
        Some(current) => visible
            .iter()
            .rev()
            .find(|item| item.index < current && !item.disabled)
            .or_else(|| visible.iter().find(|item| !item.disabled))
            .map(|item| item.index),
        None => visible
            .iter()
            .rev()
            .find(|item| !item.disabled)
            .map(|item| item.index),
    }
}

fn leading_image_node(
    name: impl Into<String>,
    image: ImageContent,
    size: f32,
    label: Option<String>,
) -> UiNode {
    let node = UiNode::image(
        name,
        image,
        LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: px(size),
                height: px(size),
            },
            margin: taffy::prelude::Rect {
                right: LengthPercentageAuto::length(6.0),
                ..taffy::prelude::Rect::length(0.0)
            },
            flex_shrink: 0.0,
            ..Default::default()
        }),
    );
    if let Some(label) = label {
        node.with_accessibility(AccessibilityMeta::new(AccessibilityRole::Image).label(label))
    } else {
        node
    }
}

fn with_optional_shader(mut node: UiNode, shader: Option<&ShaderEffect>) -> UiNode {
    if let Some(shader) = shader {
        node = node.with_shader(shader.clone());
    }
    node
}

fn accessibility_label_or_name(label: &Option<String>, name: &str) -> String {
    label.clone().unwrap_or_else(|| name.to_owned())
}

fn apply_enabled(meta: AccessibilityMeta, enabled: bool) -> AccessibilityMeta {
    if enabled {
        meta
    } else {
        meta.disabled()
    }
}

fn push_state(values: &mut Vec<String>, label: &str, active: bool) {
    if active {
        values.push(label.to_owned());
    }
}

fn muted_text_style() -> TextStyle {
    TextStyle {
        color: ColorRgba::new(151, 162, 178, 255),
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn px(value: f32) -> Dimension {
    Dimension::length(value.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_view_state_activates_visible_item_by_id() {
        let roots = vec![TreeItem::new("root", "Project").with_children(vec![
            TreeItem::new("src", "src").with_children(vec![TreeItem::new("lib", "lib.rs")]),
            TreeItem::new("target", "target").disabled(),
        ])];
        let mut state = TreeViewState::expanded(["root"]);

        let item = state
            .activate_visible_item_id(&roots, "src")
            .expect("src item");
        assert_eq!(item.id, "src");
        assert_eq!(state.selected_index, Some(item.index));
        assert!(state.is_expanded("src"));

        assert!(state.activate_visible_item_id(&roots, "target").is_none());
        assert_eq!(state.selected_index, Some(item.index));
    }
}
