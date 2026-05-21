//! Tree view and outliner widget implementation.

use std::collections::HashSet;
use std::ops::Range;

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentage,
    LengthPercentageAuto, Rect as TaffyRect, Size as TaffySize, Style,
};

use crate::widgets::{button, ButtonOptions};
use crate::{
    drag_drop::{DragSourceDescriptor, DragSourceId, DropTargetDescriptor, DropTargetId},
    platform::{DragOperation, DragPayload},
    virtualization::{
        plan_virtualized_range, VirtualAxis, VirtualCollectionKind, VirtualFocusPreservation,
        VirtualItemKey, VirtualOverscan, VirtualPlan, VirtualPlanRequest,
    },
    AccessibilityAction, AccessibilityMeta, AccessibilityRole, ClipBehavior, ColorRgba, CommandId,
    DragDropSurfaceKind, DropPayloadFilter, ImageContent, InputBehavior, LayoutStyle, ScrollAxes,
    ShaderEffect, StrokeStyle, TextStyle, TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle,
    UiRect, UiSize, UiVisual,
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
    expanded_ids: Vec<String>,
    selected_index: Option<usize>,
}

impl TreeViewState {
    pub fn expanded(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut state = Self::default();
        for id in ids {
            state.set_expanded(id, true);
        }
        state
    }

    pub fn with_selected(mut self, selected_index: Option<usize>) -> Self {
        self.select(selected_index);
        self
    }

    pub fn expanded_ids(&self) -> &[String] {
        &self.expanded_ids
    }

    pub const fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn clear_expanded(&mut self) {
        self.expanded_ids.clear();
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

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualTreeViewSpec {
    pub row_height: f32,
    pub viewport_height: f32,
    pub scroll_offset: f32,
    pub overscan_rows: u64,
    pub focus: Option<VirtualFocusPreservation>,
}

impl VirtualTreeViewSpec {
    pub fn new(row_height: f32, viewport_height: f32) -> Self {
        Self {
            row_height,
            viewport_height,
            scroll_offset: 0.0,
            overscan_rows: 1,
            focus: None,
        }
    }

    pub fn scroll_offset(mut self, scroll_offset: f32) -> Self {
        self.scroll_offset = finite_nonnegative(scroll_offset);
        self
    }

    pub const fn overscan_rows(mut self, overscan_rows: u64) -> Self {
        self.overscan_rows = overscan_rows;
        self
    }

    pub fn focus(mut self, focus: VirtualFocusPreservation) -> Self {
        self.focus = Some(focus);
        self
    }

    pub fn content_height(&self, visible_count: usize) -> f32 {
        visible_count as f32 * finite_extent(self.row_height)
    }

    pub fn clamped_scroll_offset(&self, visible_count: usize) -> f32 {
        let content_height = self.content_height(visible_count);
        let viewport_height = finite_nonnegative(self.viewport_height);
        finite_nonnegative(self.scroll_offset).min((content_height - viewport_height).max(0.0))
    }

    pub fn plan(&self, visible_count: usize) -> VirtualPlan {
        let mut request = VirtualPlanRequest::new(
            visible_count as u64,
            finite_extent(self.row_height),
            finite_nonnegative(self.viewport_height),
        )
        .kind(VirtualCollectionKind::Tree)
        .axis(VirtualAxis::Vertical)
        .scroll_offset(self.scroll_offset)
        .overscan(VirtualOverscan::uniform(self.overscan_rows));
        if let Some(focus) = self.focus.clone() {
            request = request.focus(focus);
        }
        plan_virtualized_range(request)
    }
}

impl Default for VirtualTreeViewSpec {
    fn default() -> Self {
        Self::new(26.0, 240.0)
    }
}

pub fn tree_focus_preservation_by_id(
    previous_visible: &[TreeVisibleItem],
    next_visible: &[TreeVisibleItem],
    focused_id: impl AsRef<str>,
) -> VirtualFocusPreservation {
    let focused_id = focused_id.as_ref();
    VirtualFocusPreservation::new(
        VirtualItemKey::Stable(focused_id.to_owned()),
        previous_visible
            .iter()
            .find(|item| item.id == focused_id)
            .map(|item| item.index as u64),
        next_visible
            .iter()
            .find(|item| item.id == focused_id)
            .map(|item| item.index as u64),
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualTreeViewNodes {
    pub root: UiNodeId,
    pub body: UiNodeId,
    pub rows: Vec<UiNodeId>,
    pub top_spacer: Option<UiNodeId>,
    pub bottom_spacer: Option<UiNodeId>,
    pub plan: VirtualPlan,
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
    for item in &visible_items {
        add_tree_row(document, root, &name, item, visible_count, state, &options);
    }

    root
}

/// Build a tree view that only materializes rows inside the current viewport.
pub fn virtualized_tree_view(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    roots: &[TreeItem],
    state: &TreeViewState,
    spec: VirtualTreeViewSpec,
    mut options: TreeViewOptions,
) -> VirtualTreeViewNodes {
    let name = name.into();
    options.row_height = finite_extent(spec.row_height);
    let visible_items = state.visible_items(roots);
    let visible_count = visible_items.len();
    let plan = spec.plan(visible_count);
    let scroll_offset = spec.clamped_scroll_offset(visible_count);
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
                .value(virtual_tree_accessibility_value(
                    visible_count,
                    &plan.visible_range,
                ))
                .focusable(),
        ),
    );

    let body = document.add_child(
        root,
        UiNode::container(
            format!("{name}.body"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: px(spec.viewport_height),
                    },
                    min_size: TaffySize {
                        width: px(0.0),
                        height: px(0.0),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_scroll(ScrollAxes::VERTICAL),
    );
    if let Some(scroll) = &mut document.node_mut(body).scroll {
        scroll.offset.y = scroll_offset;
    }

    let mut top_spacer = None;
    let top = plan
        .items
        .first()
        .map(|item| item.start)
        .unwrap_or_default();
    if top > 0.0 {
        top_spacer = Some(document.add_child(body, tree_spacer(format!("{name}.top_spacer"), top)));
    }

    let mut rows = Vec::new();
    for item_plan in &plan.items {
        let Some(item) = visible_items.get(item_plan.index as usize) else {
            continue;
        };
        rows.push(add_tree_row(
            document,
            body,
            &name,
            item,
            visible_count,
            state,
            &options,
        ));
    }

    let bottom = visible_count.saturating_sub(plan.materialized_range.end as usize) as f32
        * finite_extent(spec.row_height);
    let mut bottom_spacer = None;
    if bottom > 0.0 {
        bottom_spacer =
            Some(document.add_child(body, tree_spacer(format!("{name}.bottom_spacer"), bottom)));
    }

    VirtualTreeViewNodes {
        root,
        body,
        rows,
        top_spacer,
        bottom_spacer,
        plan,
    }
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

fn add_tree_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    item: &TreeVisibleItem,
    visible_count: usize,
    state: &TreeViewState,
    options: &TreeViewOptions,
) -> UiNodeId {
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
                flex_shrink: 0.0,
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
        item,
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
    let row = document.add_child(parent, row);

    add_tree_indent_guides(document, row, name, item, options);

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
                flex_shrink: 1.0,
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::percent(1.0),
                },
                min_size: TaffySize {
                    width: px(0.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        ),
    );
    add_tree_row_actions(document, row, name, item, options);

    row
}

fn add_tree_indent_guides(
    document: &mut UiDocument,
    row: UiNodeId,
    name: &str,
    item: &TreeVisibleItem,
    options: &TreeViewOptions,
) {
    for depth in 0..item.depth {
        let slot = document.add_child(
            row,
            UiNode::container(
                format!("{name}.row.{}.indent.{depth}", item.id),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        display: Display::Flex,
                        align_items: Some(AlignItems::Center),
                        justify_content: Some(JustifyContent::Center),
                        size: TaffySize {
                            width: px(options.indent_width),
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
        document.add_child(
            slot,
            UiNode::container(
                format!("{name}.row.{}.indent.{depth}.guide", item.id),
                LayoutStyle::new()
                    .with_width(1.0)
                    .with_height_percent(1.0)
                    .with_flex_shrink(0.0),
            )
            .with_visual(UiVisual::panel(ColorRgba::new(63, 70, 82, 180), None, 0.0)),
        );
    }
}

fn add_tree_row_actions(
    document: &mut UiDocument,
    row: UiNodeId,
    name: &str,
    item: &TreeVisibleItem,
    options: &TreeViewOptions,
) {
    let Some(prefix) = options.row_action_prefix.as_deref() else {
        return;
    };
    for action in item.enabled_row_actions() {
        let action_name = action.id.as_str();
        let mut visual = UiVisual::panel(
            ColorRgba::new(50, 56, 66, 255),
            Some(StrokeStyle::new(ColorRgba::new(72, 82, 98, 255), 1.0)),
            4.0,
        );
        let text_color = if action.destructive {
            ColorRgba::new(255, 176, 64, 255)
        } else {
            ColorRgba::new(220, 228, 238, 255)
        };
        if action.destructive {
            visual.fill = ColorRgba::new(58, 48, 38, 255);
        }
        let mut button_options = ButtonOptions::new(
            LayoutStyle::new()
                .with_height((options.row_height - 6.0).max(20.0))
                .with_flex_shrink(0.0),
        )
        .with_action(format!("{prefix}.action.{}.{}", item.id, action_name));
        button_options.visual = visual;
        button_options.hovered_visual = Some(UiVisual::panel(
            ColorRgba::new(70, 78, 92, 255),
            Some(StrokeStyle::new(ColorRgba::new(96, 112, 132, 255), 1.0)),
            4.0,
        ));
        button_options.pressed_visual = Some(UiVisual::panel(
            ColorRgba::new(36, 42, 52, 255),
            Some(StrokeStyle::new(ColorRgba::new(92, 120, 156, 255), 1.0)),
            4.0,
        ));
        button_options.text_style = TextStyle {
            font_size: 12.0,
            line_height: 16.0,
            color: text_color,
            wrap: TextWrap::None,
            ..Default::default()
        };
        button_options.accessibility_label = Some(format!("{} {}", action.label, item.label));
        if let Some(image) = action.leading_image.clone() {
            button_options.leading_image = Some(image);
            button_options.image_size = UiSize::new(14.0, 14.0);
        }
        button(
            document,
            row,
            format!("{name}.row.{}.action.{action_name}", item.id),
            action.label.clone(),
            button_options,
        );
    }
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
                ..taffy::prelude::Rect::length(0.0_f32)
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

fn tree_spacer(name: impl Into<String>, height: f32) -> UiNode {
    UiNode::container(
        name,
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: px(height),
                },
                flex_shrink: 0.0,
                ..Default::default()
            })
            .style,
            ..Default::default()
        },
    )
}

fn virtual_tree_accessibility_value(visible_count: usize, visible_range: &Range<u64>) -> String {
    if visible_count == 0 {
        return "0 visible items".to_owned();
    }
    format!(
        "{} visible items; showing {}-{}",
        visible_count,
        visible_range.start.saturating_add(1),
        visible_range.end.min(visible_count as u64)
    )
}

fn px(value: f32) -> Dimension {
    Dimension::length(value.max(0.0))
}

fn finite_extent(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{root_style, ApproxTextMeasurer};

    fn node_id(document: &UiDocument, name: &str) -> UiNodeId {
        document
            .nodes()
            .iter()
            .enumerate()
            .find_map(|(index, node)| (node.name() == name).then_some(UiNodeId::from_index(index)))
            .unwrap_or_else(|| panic!("missing node {name:?}"))
    }

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
        assert_eq!(state.selected_index(), Some(item.index));
        assert!(state.is_expanded("src"));

        assert!(state.activate_visible_item_id(&roots, "target").is_none());
        assert_eq!(state.selected_index(), Some(item.index));
    }

    #[test]
    fn tree_view_renders_row_actions_and_indent_guides() {
        let mut document = UiDocument::new(root_style(360.0, 240.0));
        let roots = vec![
            TreeItem::new("root", "root").with_children(vec![TreeItem::new("child", "child #0")
                .with_row_action(TreeRowAction::new("delete", "delete").destructive())
                .with_row_action(TreeRowAction::new("add", "+"))]),
        ];
        let state = TreeViewState::expanded(["root"]);
        let root = document.root;
        tree_view(
            &mut document,
            root,
            "tree",
            &roots,
            &state,
            TreeViewOptions::default().with_row_action_prefix("tree"),
        );
        document
            .compute_layout(UiSize::new(360.0, 240.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let delete = document.node(node_id(&document, "tree.row.child.action.delete"));
        assert_eq!(
            delete
                .action()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("tree.action.child.delete")
        );
        let add = document.node(node_id(&document, "tree.row.child.action.add"));
        assert_eq!(
            add.action()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("tree.action.child.add")
        );
        assert!(document
            .nodes()
            .iter()
            .any(|node| node.name() == "tree.row.child.indent.0.guide"));
    }

    #[test]
    fn virtual_tree_focus_preservation_uses_stable_item_ids_across_filtering() {
        let previous_roots = vec![TreeItem::new("root", "Project").with_children(vec![
            TreeItem::new("alpha", "Alpha"),
            TreeItem::new("beta", "Beta"),
            TreeItem::new("gamma", "Gamma"),
            TreeItem::new("delta", "Delta"),
        ])];
        let next_roots = vec![TreeItem::new("root", "Project").with_children(vec![
            TreeItem::new("alpha", "Alpha"),
            TreeItem::new("gamma", "Gamma"),
            TreeItem::new("delta", "Delta"),
        ])];
        let state = TreeViewState::expanded(["root"]);
        let previous = state.visible_items(&previous_roots);
        let next = state.visible_items(&next_roots);

        let focus = tree_focus_preservation_by_id(&previous, &next, "gamma");
        assert_eq!(focus.key, VirtualItemKey::Stable("gamma".to_owned()));
        assert_eq!(focus.index_before, Some(3));
        assert_eq!(focus.index_after, Some(2));

        let plan = VirtualTreeViewSpec::new(20.0, 60.0)
            .scroll_offset(20.0)
            .focus(focus.clone())
            .plan(next.len());
        assert_eq!(plan.focus, Some(focus));
    }
}
