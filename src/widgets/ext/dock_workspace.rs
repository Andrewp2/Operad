//! Dock workspace widget.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentage,
    LengthPercentageAuto, Position, Rect, Size as TaffySize, Style,
};

use crate::{
    length,
    platform::{DragOperation, DragPayload},
    widgets::publish_inline_intrinsic_size,
    AccessibilityMeta, AccessibilityRole, ClipBehavior, DragDropSurfaceKind, DragSourceDescriptor,
    DragSourceId, DropPayloadFilter, DropTargetDescriptor, DropTargetHit, DropTargetId,
    ImageContent, InputBehavior, LayoutStyle, ShaderEffect, StrokeStyle, TextStyle, UiDocument,
    UiNode, UiNodeId, UiNodeStyle, UiRect, UiSize, UiVisual, WidgetActionBinding,
};

use super::surfaces::{DEFAULT_ACCENT, DEFAULT_SURFACE_BG, DEFAULT_SURFACE_STROKE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockSide {
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

impl DockSide {
    pub const fn is_horizontal_edge(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    pub const fn is_vertical_edge(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

fn dock_side_name(side: DockSide) -> &'static str {
    match side {
        DockSide::Top => "Top",
        DockSide::Bottom => "Bottom",
        DockSide::Left => "Left",
        DockSide::Right => "Right",
        DockSide::Center => "Center",
    }
}

const DOCK_PANEL_PAYLOAD_PREFIX: &str = "operad:dock-panel:";

pub fn dock_panel_drag_payload(panel_id: impl AsRef<str>) -> DragPayload {
    DragPayload::text(format!("{DOCK_PANEL_PAYLOAD_PREFIX}{}", panel_id.as_ref()))
}

pub fn dock_panel_id_from_payload(payload: &DragPayload) -> Option<&str> {
    payload
        .text
        .as_deref()
        .and_then(|text| text.strip_prefix(DOCK_PANEL_PAYLOAD_PREFIX))
        .filter(|id| !id.is_empty())
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockPanelDescriptor {
    pub id: String,
    pub title: String,
    pub side: DockSide,
    pub size: f32,
    pub min_size: f32,
    pub visible: bool,
    pub resizable: bool,
    pub title_image: Option<ImageContent>,
    pub shader: Option<ShaderEffect>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl DockPanelDescriptor {
    pub fn new(id: impl Into<String>, title: impl Into<String>, side: DockSide, size: f32) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            side,
            size: size.max(0.0),
            min_size: 48.0,
            visible: true,
            resizable: false,
            title_image: None,
            shader: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }

    pub fn center(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            side: DockSide::Center,
            size: 1.0,
            min_size: 0.0,
            visible: true,
            resizable: false,
            title_image: None,
            shader: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }

    pub fn with_min_size(mut self, min_size: f32) -> Self {
        self.min_size = min_size.max(0.0);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn title_image(mut self, image: ImageContent) -> Self {
        self.title_image = Some(image);
        self
    }

    pub fn shader(mut self, shader: ShaderEffect) -> Self {
        self.shader = Some(shader);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    pub fn drag_source(&self, bounds: UiRect) -> Option<DragSourceDescriptor> {
        if !self.visible || self.id.is_empty() {
            return None;
        }

        let label = if self.title.is_empty() {
            self.id.clone()
        } else {
            self.title.clone()
        };

        Some(
            DragSourceDescriptor::new(
                DragSourceId::new(format!("dock.panel.{}", self.id)),
                DragDropSurfaceKind::DockPanel,
                bounds,
                dock_panel_drag_payload(&self.id),
            )
            .allowed_operations([DragOperation::Move])
            .label(format!("{label} panel"))
            .hint("Drag to dock, undock, float, or reorder this panel"),
        )
    }

    pub fn accessibility(&self) -> AccessibilityMeta {
        let label = self
            .accessibility_label
            .clone()
            .or_else(|| (!self.title.is_empty()).then(|| self.title.clone()))
            .unwrap_or_else(|| self.id.clone());
        let hint = self.accessibility_hint.clone().unwrap_or_else(|| {
            if self.resizable {
                format!("{} dock panel, resizable", dock_side_name(self.side))
            } else {
                format!("{} dock panel", dock_side_name(self.side))
            }
        });
        AccessibilityMeta::new(AccessibilityRole::TabPanel)
            .label(label)
            .hint(hint)
    }
}

#[derive(Debug, Clone)]
pub struct DockWorkspaceOptions {
    pub layout: LayoutStyle,
    pub panel_visual: UiVisual,
    pub center_visual: UiVisual,
    pub resize_handle_visual: UiVisual,
    pub title_style: TextStyle,
    pub show_titles: bool,
    pub handle_thickness: f32,
    pub title_image_size: UiSize,
}

impl Default for DockWorkspaceOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            }),
            panel_visual: UiVisual::panel(
                DEFAULT_SURFACE_BG,
                Some(StrokeStyle::new(DEFAULT_SURFACE_STROKE, 1.0)),
                0.0,
            ),
            center_visual: UiVisual::TRANSPARENT,
            resize_handle_visual: UiVisual::panel(DEFAULT_ACCENT, None, 0.0),
            title_style: TextStyle {
                font_size: 13.0,
                line_height: 18.0,
                ..Default::default()
            },
            show_titles: true,
            handle_thickness: 5.0,
            title_image_size: UiSize::new(16.0, 16.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockPanelNode {
    pub id: String,
    pub side: DockSide,
    pub root: UiNodeId,
    pub title: Option<UiNodeId>,
    pub content: UiNodeId,
    pub resize_handle: Option<UiNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockWorkspaceNodes {
    pub root: UiNodeId,
    pub body: UiNodeId,
    pub center: Option<UiNodeId>,
    pub panels: Vec<DockPanelNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockPanelLayoutSnapshot {
    pub id: String,
    pub side: DockSide,
    pub size: f32,
    pub visible: bool,
}

impl DockPanelLayoutSnapshot {
    pub fn from_descriptor(panel: &DockPanelDescriptor) -> Self {
        Self {
            id: panel.id.clone(),
            side: panel.side,
            size: panel.size,
            visible: panel.visible,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DockWorkspaceLayoutSnapshot {
    pub panels: Vec<DockPanelLayoutSnapshot>,
}

impl DockWorkspaceLayoutSnapshot {
    pub fn from_descriptors(panels: &[DockPanelDescriptor]) -> Self {
        Self {
            panels: panels
                .iter()
                .map(DockPanelLayoutSnapshot::from_descriptor)
                .collect(),
        }
    }

    pub fn panel(&self, id: &str) -> Option<&DockPanelLayoutSnapshot> {
        self.panels.iter().find(|panel| panel.id == id)
    }

    pub fn apply_to(&self, panels: &mut [DockPanelDescriptor]) -> DockWorkspaceLayoutApplyReport {
        let mut report = DockWorkspaceLayoutApplyReport::default();
        for saved in &self.panels {
            if let Some(panel) = panels.iter_mut().find(|panel| panel.id == saved.id) {
                panel.side = saved.side;
                panel.size = saved.size.max(panel.min_size);
                panel.visible = saved.visible;
                report.updated.push(saved.id.clone());
            } else {
                report.missing.push(saved.id.clone());
            }
        }
        report
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DockWorkspaceLayoutApplyReport {
    pub updated: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DockPanelPlacement {
    Docked(DockSide),
    Floating(UiRect),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockFloatingPanel {
    pub id: String,
    pub rect: UiRect,
}

impl DockFloatingPanel {
    pub fn new(id: impl Into<String>, rect: UiRect) -> Self {
        Self {
            id: id.into(),
            rect,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockWorkspaceStateChange {
    pub panel_id: String,
    pub previous: DockPanelPlacement,
    pub next: DockPanelPlacement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockWorkspaceVisibilityChange {
    pub panel_id: String,
    pub previous_visible: bool,
    pub next_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockPanelReorderPlacement {
    Before,
    After,
}

impl DockPanelReorderPlacement {
    pub const fn id_suffix(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Before => "Move before",
            Self::After => "Move after",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockWorkspaceReorderChange {
    pub panel_id: String,
    pub target_panel_id: String,
    pub previous_index: usize,
    pub next_index: usize,
    pub previous_side: DockSide,
    pub next_side: DockSide,
    pub placement: DockPanelReorderPlacement,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockWorkspaceState {
    floating_panels: Vec<DockFloatingPanel>,
    hidden_panels: Vec<String>,
    panel_order: Vec<String>,
    pub focused_panel: Option<String>,
}

impl Default for DockWorkspaceState {
    fn default() -> Self {
        Self {
            floating_panels: Vec::new(),
            hidden_panels: Vec::new(),
            panel_order: Vec::new(),
            focused_panel: None,
        }
    }
}

impl DockWorkspaceState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn floating_panels(&self) -> &[DockFloatingPanel] {
        &self.floating_panels
    }

    pub fn floating_panel(&self, id: &str) -> Option<&DockFloatingPanel> {
        self.floating_panels.iter().find(|panel| panel.id == id)
    }

    pub fn is_floating(&self, id: &str) -> bool {
        self.floating_panel(id).is_some()
    }

    pub fn hidden_panels(&self) -> &[String] {
        &self.hidden_panels
    }

    pub fn panel_order(&self) -> &[String] {
        &self.panel_order
    }

    pub fn set_panel_order(&mut self, order: impl IntoIterator<Item = impl Into<String>>) {
        self.panel_order = order.into_iter().map(Into::into).collect();
    }

    pub fn apply_order_to_panels(&self, panels: &mut Vec<DockPanelDescriptor>) {
        if self.panel_order.is_empty() {
            return;
        }

        panels.sort_by_key(|panel| {
            self.panel_order
                .iter()
                .position(|id| id == &panel.id)
                .unwrap_or(usize::MAX)
        });
    }

    pub fn is_hidden(&self, id: &str) -> bool {
        self.hidden_panels.iter().any(|panel| panel == id)
    }

    pub fn set_panel_hidden(
        &mut self,
        id: impl Into<String>,
        hidden: bool,
    ) -> DockWorkspaceVisibilityChange {
        let id = id.into();
        let was_hidden = self.is_hidden(&id);
        if hidden {
            if !was_hidden {
                self.hidden_panels.push(id.clone());
            }
            self.floating_panels
                .retain(|floating| floating.id != id.as_str());
            if self.focused_panel.as_deref() == Some(id.as_str()) {
                self.focused_panel = None;
            }
        } else {
            self.hidden_panels.retain(|panel| panel != &id);
            self.focus_panel(id.clone());
        }
        DockWorkspaceVisibilityChange {
            panel_id: id,
            previous_visible: !was_hidden,
            next_visible: !hidden,
        }
    }

    pub fn hide_panel(&mut self, id: impl Into<String>) -> DockWorkspaceVisibilityChange {
        self.set_panel_hidden(id, true)
    }

    pub fn show_panel(&mut self, id: impl Into<String>) -> DockWorkspaceVisibilityChange {
        self.set_panel_hidden(id, false)
    }

    pub fn toggle_panel_hidden(&mut self, id: impl Into<String>) -> DockWorkspaceVisibilityChange {
        let id = id.into();
        let hidden = !self.is_hidden(&id);
        self.set_panel_hidden(id, hidden)
    }

    pub fn focus_panel(&mut self, id: impl Into<String>) {
        self.focused_panel = Some(id.into());
    }

    pub fn float_panel(
        &mut self,
        panel: &DockPanelDescriptor,
        rect: UiRect,
    ) -> DockWorkspaceStateChange {
        let previous = self
            .floating_panel(&panel.id)
            .map(|floating| DockPanelPlacement::Floating(floating.rect))
            .unwrap_or(DockPanelPlacement::Docked(panel.side));
        if let Some(floating) = self
            .floating_panels
            .iter_mut()
            .find(|floating| floating.id == panel.id)
        {
            floating.rect = rect;
        } else {
            self.floating_panels
                .push(DockFloatingPanel::new(panel.id.clone(), rect));
        }
        self.focus_panel(panel.id.clone());
        DockWorkspaceStateChange {
            panel_id: panel.id.clone(),
            previous,
            next: DockPanelPlacement::Floating(rect),
        }
    }

    pub fn dock_panel(
        &mut self,
        panel: &DockPanelDescriptor,
        side: DockSide,
    ) -> DockWorkspaceStateChange {
        let previous = self
            .floating_panel(&panel.id)
            .map(|floating| DockPanelPlacement::Floating(floating.rect))
            .unwrap_or(DockPanelPlacement::Docked(panel.side));
        self.floating_panels
            .retain(|floating| floating.id != panel.id);
        self.focus_panel(panel.id.clone());
        DockWorkspaceStateChange {
            panel_id: panel.id.clone(),
            previous,
            next: DockPanelPlacement::Docked(side),
        }
    }

    pub fn apply_visibility_to_panels(&self, panels: &mut [DockPanelDescriptor]) {
        for panel in panels {
            if self.is_floating(&panel.id) || self.is_hidden(&panel.id) {
                panel.visible = false;
            }
        }
    }

    pub fn apply_drop_to_panels(
        &mut self,
        panels: &mut [DockPanelDescriptor],
        payload: &DragPayload,
        placement: DockDropPlacement,
        floating_rect: UiRect,
    ) -> Option<DockWorkspaceStateChange> {
        let panel_id = dock_panel_id_from_payload(payload)?;
        let panel = panels.iter_mut().find(|panel| panel.id == panel_id)?;
        match placement {
            DockDropPlacement::Dock(side) => {
                let change = self.dock_panel(panel, side);
                panel.side = side;
                panel.visible = true;
                Some(change)
            }
            DockDropPlacement::Floating => {
                let change = self.float_panel(panel, floating_rect);
                panel.visible = false;
                Some(change)
            }
        }
    }

    pub fn apply_drop_hit_to_panels(
        &mut self,
        panels: &mut [DockPanelDescriptor],
        payload: &DragPayload,
        hit: &DropTargetHit,
        floating_rect: UiRect,
    ) -> Option<DockWorkspaceStateChange> {
        let placement = dock_drop_placement_from_target_id(hit.target_id.0.as_str())?;
        self.apply_drop_to_panels(panels, payload, placement, floating_rect)
    }

    pub fn reorder_panel(
        &mut self,
        panels: &mut Vec<DockPanelDescriptor>,
        panel_id: &str,
        target_panel_id: &str,
        placement: DockPanelReorderPlacement,
    ) -> Option<DockWorkspaceReorderChange> {
        if panel_id == target_panel_id {
            return None;
        }

        let previous_index = panels.iter().position(|panel| panel.id == panel_id)?;
        let target_index = panels
            .iter()
            .position(|panel| panel.id == target_panel_id)?;
        let previous_side = panels[previous_index].side;
        let next_side = panels[target_index].side;

        let mut panel = panels.remove(previous_index);
        let adjusted_target_index = if previous_index < target_index {
            target_index.saturating_sub(1)
        } else {
            target_index
        };
        let mut next_index = match placement {
            DockPanelReorderPlacement::Before => adjusted_target_index,
            DockPanelReorderPlacement::After => adjusted_target_index.saturating_add(1),
        };
        next_index = next_index.min(panels.len());

        panel.side = next_side;
        panel.visible = true;
        panels.insert(next_index, panel);
        self.panel_order = panels.iter().map(|panel| panel.id.clone()).collect();
        self.floating_panels
            .retain(|floating| floating.id != panel_id);
        self.hidden_panels.retain(|hidden| hidden != panel_id);
        self.focus_panel(panel_id.to_owned());

        Some(DockWorkspaceReorderChange {
            panel_id: panel_id.to_owned(),
            target_panel_id: target_panel_id.to_owned(),
            previous_index,
            next_index,
            previous_side,
            next_side,
            placement,
        })
    }

    pub fn apply_reorder_to_panels(
        &mut self,
        panels: &mut Vec<DockPanelDescriptor>,
        payload: &DragPayload,
        target_panel_id: &str,
        placement: DockPanelReorderPlacement,
    ) -> Option<DockWorkspaceReorderChange> {
        let panel_id = dock_panel_id_from_payload(payload)?;
        self.reorder_panel(panels, panel_id, target_panel_id, placement)
    }

    pub fn apply_reorder_hit_to_panels(
        &mut self,
        panels: &mut Vec<DockPanelDescriptor>,
        payload: &DragPayload,
        hit: &DropTargetHit,
    ) -> Option<DockWorkspaceReorderChange> {
        let target = dock_panel_reorder_target_from_id(hit.target_id.0.as_str())?;
        self.apply_reorder_to_panels(panels, payload, &target.panel_id, target.placement)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockDropPlacement {
    Dock(DockSide),
    Floating,
}

impl DockDropPlacement {
    pub const fn id_suffix(self) -> &'static str {
        match self {
            Self::Dock(DockSide::Top) => "top",
            Self::Dock(DockSide::Bottom) => "bottom",
            Self::Dock(DockSide::Left) => "left",
            Self::Dock(DockSide::Right) => "right",
            Self::Dock(DockSide::Center) => "center",
            Self::Floating => "floating",
        }
    }

    pub const fn z_index(self) -> i16 {
        match self {
            Self::Dock(DockSide::Top)
            | Self::Dock(DockSide::Bottom)
            | Self::Dock(DockSide::Left)
            | Self::Dock(DockSide::Right) => 30,
            Self::Dock(DockSide::Center) => 20,
            Self::Floating => 10,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Dock(side) => format!("Dock {}", dock_side_name(side).to_lowercase()),
            Self::Floating => "Float panel".to_owned(),
        }
    }
}

pub fn dock_drop_placement_from_target_id(id: impl AsRef<str>) -> Option<DockDropPlacement> {
    match id.as_ref().rsplit_once(".drop.")?.1 {
        "top" => Some(DockDropPlacement::Dock(DockSide::Top)),
        "bottom" => Some(DockDropPlacement::Dock(DockSide::Bottom)),
        "left" => Some(DockDropPlacement::Dock(DockSide::Left)),
        "right" => Some(DockDropPlacement::Dock(DockSide::Right)),
        "center" => Some(DockDropPlacement::Dock(DockSide::Center)),
        "floating" => Some(DockDropPlacement::Floating),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockPanelReorderTargetId {
    pub side: DockSide,
    pub panel_id: String,
    pub placement: DockPanelReorderPlacement,
}

pub fn dock_panel_reorder_target_from_id(id: impl AsRef<str>) -> Option<DockPanelReorderTargetId> {
    let suffix = id.as_ref().rsplit_once(".reorder.")?.1;
    let (side, rest) = suffix.split_once('.')?;
    let (panel_id, placement) = rest.rsplit_once('.')?;
    let side = match side {
        "top" => DockSide::Top,
        "bottom" => DockSide::Bottom,
        "left" => DockSide::Left,
        "right" => DockSide::Right,
        "center" => DockSide::Center,
        _ => return None,
    };
    let placement = match placement {
        "before" => DockPanelReorderPlacement::Before,
        "after" => DockPanelReorderPlacement::After,
        _ => return None,
    };
    Some(DockPanelReorderTargetId {
        side,
        panel_id: panel_id.to_owned(),
        placement,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockWorkspaceDropZone {
    pub placement: DockDropPlacement,
    pub target: DropTargetDescriptor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockPanelReorderTarget {
    pub panel_id: String,
    pub side: DockSide,
    pub placement: DockPanelReorderPlacement,
    pub target: DropTargetDescriptor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockWorkspaceDragOptions {
    pub allowed_sides: Vec<DockSide>,
    pub allow_floating: bool,
    pub edge_thickness: f32,
    pub accepted_operations: Vec<DragOperation>,
}

impl Default for DockWorkspaceDragOptions {
    fn default() -> Self {
        Self {
            allowed_sides: vec![
                DockSide::Top,
                DockSide::Bottom,
                DockSide::Left,
                DockSide::Right,
                DockSide::Center,
            ],
            allow_floating: true,
            edge_thickness: 48.0,
            accepted_operations: vec![DragOperation::Move],
        }
    }
}

impl DockWorkspaceDragOptions {
    pub fn allowed_sides(mut self, sides: impl IntoIterator<Item = DockSide>) -> Self {
        self.allowed_sides = sides.into_iter().collect();
        self
    }

    pub const fn allow_floating(mut self, allow_floating: bool) -> Self {
        self.allow_floating = allow_floating;
        self
    }

    pub fn edge_thickness(mut self, edge_thickness: f32) -> Self {
        self.edge_thickness = edge_thickness.max(1.0);
        self
    }

    pub fn accepted_operations(
        mut self,
        operations: impl IntoIterator<Item = DragOperation>,
    ) -> Self {
        self.accepted_operations = operations.into_iter().collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockWorkspaceReorderOptions {
    pub target_thickness: f32,
    pub accepted_operations: Vec<DragOperation>,
}

impl Default for DockWorkspaceReorderOptions {
    fn default() -> Self {
        Self {
            target_thickness: 32.0,
            accepted_operations: vec![DragOperation::Move],
        }
    }
}

impl DockWorkspaceReorderOptions {
    pub fn target_thickness(mut self, target_thickness: f32) -> Self {
        self.target_thickness = target_thickness.max(1.0);
        self
    }

    pub fn accepted_operations(
        mut self,
        operations: impl IntoIterator<Item = DragOperation>,
    ) -> Self {
        self.accepted_operations = operations.into_iter().collect();
        self
    }
}

pub fn dock_workspace_drop_zones(
    name: impl AsRef<str>,
    bounds: UiRect,
    options: DockWorkspaceDragOptions,
) -> Vec<DockWorkspaceDropZone> {
    let name = name.as_ref();
    let mut zones = Vec::new();
    for side in &options.allowed_sides {
        zones.push(dock_workspace_drop_zone(
            name,
            DockDropPlacement::Dock(*side),
            dock_drop_zone_rect(bounds, *side, options.edge_thickness),
            &options.accepted_operations,
        ));
    }
    if options.allow_floating {
        zones.push(dock_workspace_drop_zone(
            name,
            DockDropPlacement::Floating,
            bounds,
            &options.accepted_operations,
        ));
    }
    zones
}

pub fn dock_panel_reorder_drop_targets(
    name: impl AsRef<str>,
    panels: &[DockPanelDescriptor],
    side: DockSide,
    bounds: UiRect,
    options: DockWorkspaceReorderOptions,
) -> Vec<DockPanelReorderTarget> {
    let side_panels: Vec<_> = panels_for_side(panels, side).collect();
    let count = side_panels.len();
    if count == 0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
        return Vec::new();
    }

    let mut targets = Vec::with_capacity(count * 2);
    let primary_extent = if side.is_vertical_edge() {
        bounds.height
    } else {
        bounds.width
    };
    let cell_extent = primary_extent / count as f32;
    let target_extent = options.target_thickness.min(cell_extent * 0.5).max(1.0);

    for (index, panel) in side_panels.iter().enumerate() {
        let start = index as f32 * cell_extent;
        let before_bounds = dock_reorder_rect(bounds, side, start, target_extent);
        let after_bounds = dock_reorder_rect(
            bounds,
            side,
            start + cell_extent - target_extent,
            target_extent,
        );
        targets.push(dock_panel_reorder_target(
            name.as_ref(),
            panel,
            side,
            DockPanelReorderPlacement::Before,
            before_bounds,
            &options.accepted_operations,
        ));
        targets.push(dock_panel_reorder_target(
            name.as_ref(),
            panel,
            side,
            DockPanelReorderPlacement::After,
            after_bounds,
            &options.accepted_operations,
        ));
    }

    targets
}

fn dock_panel_reorder_target(
    workspace_name: &str,
    panel: &DockPanelDescriptor,
    side: DockSide,
    placement: DockPanelReorderPlacement,
    bounds: UiRect,
    operations: &[DragOperation],
) -> DockPanelReorderTarget {
    let target = DropTargetDescriptor::new(
        DropTargetId::new(format!(
            "{}.reorder.{}.{}.{}",
            workspace_name,
            dock_side_name(side).to_lowercase(),
            panel.id,
            placement.id_suffix()
        )),
        DragDropSurfaceKind::DockTarget,
        bounds,
    )
    .z_index(40)
    .accepted_payload(DropPayloadFilter::empty().text())
    .accepted_operations(operations.iter().copied())
    .label(format!("{} {}", placement.label(), panel.title))
    .hint("Drop a dock panel here to reorder this workspace side");
    DockPanelReorderTarget {
        panel_id: panel.id.clone(),
        side,
        placement,
        target,
    }
}

fn dock_reorder_rect(bounds: UiRect, side: DockSide, offset: f32, extent: f32) -> UiRect {
    if side.is_vertical_edge() {
        UiRect::new(bounds.x, bounds.y + offset, bounds.width, extent)
    } else {
        UiRect::new(bounds.x + offset, bounds.y, extent, bounds.height)
    }
}

fn dock_workspace_drop_zone(
    name: &str,
    placement: DockDropPlacement,
    bounds: UiRect,
    operations: &[DragOperation],
) -> DockWorkspaceDropZone {
    let target = DropTargetDescriptor::new(
        DropTargetId::new(format!("{}.drop.{}", name, placement.id_suffix())),
        DragDropSurfaceKind::DockTarget,
        bounds,
    )
    .z_index(placement.z_index())
    .accepted_payload(DropPayloadFilter::empty().text())
    .accepted_operations(operations.iter().copied())
    .label(placement.label())
    .hint("Drop a dock panel here");
    DockWorkspaceDropZone { placement, target }
}

fn dock_drop_zone_rect(bounds: UiRect, side: DockSide, edge_thickness: f32) -> UiRect {
    let edge = edge_thickness
        .max(1.0)
        .min(bounds.width.max(0.0))
        .min(bounds.height.max(0.0));
    match side {
        DockSide::Top => UiRect::new(bounds.x, bounds.y, bounds.width, edge),
        DockSide::Bottom => UiRect::new(bounds.x, bounds.bottom() - edge, bounds.width, edge),
        DockSide::Left => UiRect::new(bounds.x, bounds.y, edge, bounds.height),
        DockSide::Right => UiRect::new(bounds.right() - edge, bounds.y, edge, bounds.height),
        DockSide::Center => {
            let inset_x = edge.min(bounds.width / 2.0);
            let inset_y = edge.min(bounds.height / 2.0);
            UiRect::new(
                bounds.x + inset_x,
                bounds.y + inset_y,
                (bounds.width - inset_x * 2.0).max(0.0),
                (bounds.height - inset_y * 2.0).max(0.0),
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockDrawerDescriptor {
    pub id: String,
    pub title: String,
    pub panel_id: String,
    pub side: DockSide,
    pub open: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl DockDrawerDescriptor {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        panel_id: impl Into<String>,
        side: DockSide,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            panel_id: panel_id.into(),
            side,
            open: false,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }

    pub fn for_panel(panel: &DockPanelDescriptor, open: bool) -> Self {
        Self {
            id: panel.id.clone(),
            title: panel.title.clone(),
            panel_id: panel.id.clone(),
            side: panel.side,
            open,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }

    pub const fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    pub fn accessibility(&self) -> AccessibilityMeta {
        AccessibilityMeta::new(AccessibilityRole::Button)
            .label(
                self.accessibility_label
                    .clone()
                    .unwrap_or_else(|| self.title.clone()),
            )
            .hint(self.accessibility_hint.clone().unwrap_or_else(|| {
                if self.open {
                    "Close dock drawer".to_owned()
                } else {
                    "Open dock drawer".to_owned()
                }
            }))
            .pressed(self.open)
            .focusable()
    }
}

#[derive(Debug, Clone)]
pub struct DockDrawerRailOptions {
    pub layout: LayoutStyle,
    pub item_size: UiSize,
    pub visual: UiVisual,
    pub hovered_visual: Option<UiVisual>,
    pub active_visual: Option<UiVisual>,
    pub text_style: TextStyle,
    pub gap: f32,
}

impl Default for DockDrawerRailOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height(34.0)
                .with_padding(4.0)
                .with_gap(4.0),
            item_size: UiSize::new(92.0, 26.0),
            visual: UiVisual::panel(
                DEFAULT_SURFACE_BG,
                Some(StrokeStyle::new(DEFAULT_SURFACE_STROKE, 1.0)),
                4.0,
            ),
            hovered_visual: Some(UiVisual::panel(
                DEFAULT_SURFACE_BG,
                Some(StrokeStyle::new(DEFAULT_ACCENT, 1.0)),
                4.0,
            )),
            active_visual: Some(UiVisual::panel(
                DEFAULT_ACCENT,
                Some(StrokeStyle::new(DEFAULT_ACCENT, 1.0)),
                4.0,
            )),
            text_style: TextStyle {
                font_size: 12.0,
                line_height: 16.0,
                ..Default::default()
            },
            gap: 4.0,
        }
    }
}

impl DockDrawerRailOptions {
    pub fn for_side(side: DockSide) -> Self {
        let mut options = Self::default();
        let direction = if side.is_vertical_edge() {
            FlexDirection::Column
        } else {
            FlexDirection::Row
        };
        options.layout = LayoutStyle::from_taffy_style(Style {
            display: Display::Flex,
            flex_direction: direction,
            size: TaffySize {
                width: Dimension::percent(1.0),
                height: Dimension::auto(),
            },
            padding: Rect::length(4.0),
            gap: TaffySize {
                width: LengthPercentage::length(options.gap),
                height: LengthPercentage::length(options.gap),
            },
            ..Default::default()
        });
        options
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockDrawerItemNode {
    pub root: UiNodeId,
    pub label: UiNodeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockDrawerRailNodes {
    pub root: UiNodeId,
    pub items: Vec<DockDrawerItemNode>,
}

pub fn dock_drawer_rail(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    drawers: &[DockDrawerDescriptor],
    options: DockDrawerRailOptions,
) -> DockDrawerRailNodes {
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
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Toolbar)
                .label(format!("{name} dock drawers"))
                .hint("Shows drawer buttons for dock panels"),
        ),
    );

    let mut items = Vec::with_capacity(drawers.len());
    for drawer in drawers {
        let mut node = UiNode::container(
            format!("{name}.drawer.{}", drawer.id),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    align_items: Some(AlignItems::Center),
                    justify_content: Some(JustifyContent::Center),
                    size: TaffySize {
                        width: length(options.item_size.width),
                        height: length(options.item_size.height),
                    },
                    flex_shrink: 0.0,
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(if drawer.open {
            options.active_visual.unwrap_or(options.visual)
        } else {
            options.visual
        })
        .with_accessibility(drawer.accessibility());
        if let Some(action) = drawer.action.clone() {
            node = node.with_action(action);
        }
        let root = document.add_child(root, node);
        if let Some(hovered) = options.hovered_visual {
            let current_visual = document.node(root).visual;
            document.node_mut(root).interaction_visuals =
                Some(crate::InteractionVisuals::new(current_visual).hovered(hovered));
        }
        let label = document.add_child(
            root,
            UiNode::text(
                format!("{name}.drawer.{}.label", drawer.id),
                drawer.title.clone(),
                options.text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Label).label(drawer.title.clone()),
            ),
        );
        items.push(DockDrawerItemNode { root, label });
    }

    DockDrawerRailNodes { root, items }
}

pub fn dock_workspace(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    panels: &[DockPanelDescriptor],
    options: DockWorkspaceOptions,
    mut build_panel: impl FnMut(&mut UiDocument, UiNodeId, &DockPanelDescriptor),
) -> DockWorkspaceNodes {
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
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Application)
                .label(format!("{name} docking workspace"))
                .hint("Contains docked panels and a center workspace"),
        ),
    );
    let mut panel_nodes = Vec::new();

    for panel in panels_for_side(panels, DockSide::Top) {
        panel_nodes.push(add_dock_panel(document, root, &name, panel, &options));
        if let Some(node) = panel_nodes.last() {
            build_panel(document, node.content, panel);
        }
    }

    let body = document.add_child(
        root,
        UiNode::container(
            format!("{name}.body"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: length(0.0),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );

    for side in [DockSide::Left, DockSide::Center, DockSide::Right] {
        for panel in panels_for_side(panels, side) {
            panel_nodes.push(add_dock_panel(document, body, &name, panel, &options));
            if let Some(node) = panel_nodes.last() {
                build_panel(document, node.content, panel);
                if side == DockSide::Center {
                    publish_inline_intrinsic_size(
                        document,
                        node.root,
                        vec![node.content],
                        UiSize::ZERO,
                    );
                }
            }
        }
    }

    let center = panel_nodes
        .iter()
        .find(|panel| panel.side == DockSide::Center)
        .map(|panel| panel.root)
        .or_else(|| {
            let fallback = DockPanelDescriptor::center("center", "");
            let node = add_dock_panel(document, body, &name, &fallback, &options);
            let root = node.root;
            panel_nodes.push(node);
            Some(root)
        });

    for panel in panels_for_side(panels, DockSide::Bottom) {
        panel_nodes.push(add_dock_panel(document, root, &name, panel, &options));
        if let Some(node) = panel_nodes.last() {
            build_panel(document, node.content, panel);
        }
    }

    DockWorkspaceNodes {
        root,
        body,
        center,
        panels: panel_nodes,
    }
}

fn panels_for_side(
    panels: &[DockPanelDescriptor],
    side: DockSide,
) -> impl Iterator<Item = &DockPanelDescriptor> {
    panels
        .iter()
        .filter(move |panel| panel.visible && panel.side == side)
}

fn add_dock_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    workspace_name: &str,
    panel: &DockPanelDescriptor,
    options: &DockWorkspaceOptions,
) -> DockPanelNode {
    let mut root_node = UiNode::container(
        format!("{workspace_name}.panel.{}", panel.id),
        dock_panel_style(panel),
    )
    .with_visual(if panel.side == DockSide::Center {
        options.center_visual
    } else {
        options.panel_visual
    })
    .with_accessibility(panel.accessibility());
    if let Some(shader) = &panel.shader {
        root_node = root_node.with_shader(shader.clone());
    }
    let root = document.add_child(parent, root_node);

    let title = if options.show_titles && (!panel.title.is_empty() || panel.title_image.is_some()) {
        let title_bar = document.add_child(
            root,
            UiNode::container(
                format!("{workspace_name}.panel.{}.title", panel.id),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: length(24.0),
                        },
                        padding: Rect::length(4.0),
                        flex_shrink: 0.0,
                        ..Default::default()
                    })
                    .style,
                    ..Default::default()
                },
            ),
        );
        if let Some(image) = &panel.title_image {
            document.add_child(
                title_bar,
                UiNode::image(
                    format!("{workspace_name}.panel.{}.title.image", panel.id),
                    image.clone(),
                    LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(options.title_image_size.width),
                            height: length(options.title_image_size.height),
                        },
                        margin: Rect {
                            right: LengthPercentageAuto::length(6.0),
                            ..Rect::length(0.0)
                        },
                        flex_shrink: 0.0,
                        ..Default::default()
                    }),
                )
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Image).label(panel.title.clone()),
                ),
            );
        }
        if !panel.title.is_empty() {
            document.add_child(
                title_bar,
                UiNode::text(
                    format!("{workspace_name}.panel.{}.title.label", panel.id),
                    panel.title.clone(),
                    options.title_style.clone(),
                    LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: Dimension::auto(),
                        },
                        ..Default::default()
                    }),
                )
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Label).label(panel.title.clone()),
                ),
            );
        }
        Some(title_bar)
    } else {
        None
    };

    let content = document.add_child(
        root,
        UiNode::container(
            format!("{workspace_name}.panel.{}.content", panel.id),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: length(0.0),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );

    let resize_handle = panel
        .resizable
        .then(|| add_dock_resize_handle(document, root, workspace_name, panel, options));

    DockPanelNode {
        id: panel.id.clone(),
        side: panel.side,
        root,
        title,
        content,
        resize_handle,
    }
}

fn dock_panel_style(panel: &DockPanelDescriptor) -> UiNodeStyle {
    let mut layout = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        flex_shrink: 0.0,
        ..Default::default()
    };
    match panel.side {
        DockSide::Top | DockSide::Bottom => {
            layout.flex_shrink = 1.0;
            layout.size = TaffySize {
                width: Dimension::percent(1.0),
                height: length(panel.size),
            };
            layout.min_size.height = length(panel.min_size);
        }
        DockSide::Left | DockSide::Right => {
            layout.flex_shrink = 1.0;
            layout.size = TaffySize {
                width: length(panel.size),
                height: Dimension::percent(1.0),
            };
            layout.min_size.width = length(panel.min_size);
        }
        DockSide::Center => {
            layout.flex_grow = panel.size.max(0.0);
            layout.flex_shrink = 1.0;
            layout.flex_basis = length(0.0);
            layout.size = TaffySize {
                width: Dimension::percent(1.0),
                height: Dimension::percent(1.0),
            };
            layout.min_size.width = length(panel.min_size);
        }
    }
    UiNodeStyle {
        layout: LayoutStyle::from_taffy_style(layout).style,
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

fn add_dock_resize_handle(
    document: &mut UiDocument,
    parent: UiNodeId,
    workspace_name: &str,
    panel: &DockPanelDescriptor,
    options: &DockWorkspaceOptions,
) -> UiNodeId {
    let mut inset = Rect::length(0.0);
    let size = match panel.side {
        DockSide::Top => {
            inset.top = LengthPercentageAuto::auto();
            TaffySize {
                width: Dimension::percent(1.0),
                height: length(options.handle_thickness),
            }
        }
        DockSide::Bottom => {
            inset.bottom = LengthPercentageAuto::auto();
            TaffySize {
                width: Dimension::percent(1.0),
                height: length(options.handle_thickness),
            }
        }
        DockSide::Left => {
            inset.left = LengthPercentageAuto::auto();
            TaffySize {
                width: length(options.handle_thickness),
                height: Dimension::percent(1.0),
            }
        }
        DockSide::Right => {
            inset.right = LengthPercentageAuto::auto();
            TaffySize {
                width: length(options.handle_thickness),
                height: Dimension::percent(1.0),
            }
        }
        DockSide::Center => TaffySize {
            width: length(0.0),
            height: length(0.0),
        },
    };
    document.add_child(
        parent,
        UiNode::container(
            format!("{workspace_name}.panel.{}.resize", panel.id),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    position: Position::Absolute,
                    inset,
                    size,
                    ..Default::default()
                })
                .style,
                z_index: 1,
                ..Default::default()
            },
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(options.resize_handle_visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Slider)
                .label(format!("Resize {} panel", panel.title))
                .hint(format!(
                    "Drag to resize the {} dock panel",
                    dock_side_name(panel.side).to_lowercase()
                ))
                .focusable(),
        ),
    )
}
