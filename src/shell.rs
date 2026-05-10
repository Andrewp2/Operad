//! Persistable app-shell and workspace state contracts.
//!
//! This module is intentionally data-first. Widget builders can consume these
//! records, but applications can also save and restore them without depending on
//! a renderer or on the concrete widget tree that happened to produce them.

use std::collections::HashMap;

use crate::{accessibility::FocusRestoreTarget, UiPoint, UiRect, UiSize};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShellRegion {
    MenuBar,
    TransportBar,
    Toolbar,
    LeftPanel,
    RightPanel,
    BottomPanel,
    StatusBar,
    TrackList,
    Arrangement,
    Editor,
    CenterWorkspace,
    Custom(String),
}

impl ShellRegion {
    pub fn custom(id: impl Into<String>) -> Self {
        Self::Custom(id.into())
    }

    pub fn is_edge(&self) -> bool {
        matches!(
            self,
            Self::MenuBar
                | Self::TransportBar
                | Self::Toolbar
                | Self::LeftPanel
                | Self::RightPanel
                | Self::BottomPanel
                | Self::StatusBar
        )
    }

    pub fn is_editor_surface(&self) -> bool {
        matches!(self, Self::TrackList | Self::Arrangement | Self::Editor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellExtent {
    pub current: f32,
    pub min: f32,
    pub max: Option<f32>,
}

impl ShellExtent {
    pub fn new(current: f32) -> Self {
        Self {
            current: current.max(0.0),
            min: 0.0,
            max: None,
        }
    }

    pub fn with_limits(mut self, min: f32, max: Option<f32>) -> Self {
        self.min = min.max(0.0);
        self.max = max.map(|value| value.max(self.min));
        self.current = self.clamp(self.current);
        self
    }

    pub fn clamp(self, value: f32) -> f32 {
        if !value.is_finite() {
            return self.current;
        }

        let upper = self.max.unwrap_or(f32::MAX);
        value.clamp(self.min, upper)
    }

    pub fn set(&mut self, value: f32) -> bool {
        let value = self.clamp(value);
        if (self.current - value).abs() <= f32::EPSILON {
            return false;
        }
        self.current = value;
        true
    }

    pub fn resize_by(&mut self, delta: f32) -> bool {
        if !delta.is_finite() {
            return false;
        }
        self.set(self.current + delta)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DockPlacement {
    Docked(ShellRegion),
    Floating,
    Hidden,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellPanelState {
    pub id: String,
    pub title: String,
    pub placement: DockPlacement,
    pub extent: ShellExtent,
    pub visible: bool,
    pub resizable: bool,
    pub collapsed: bool,
    pub collapsed_extent: f32,
    pub restore_extent: f32,
    pub scroll_offset: UiPoint,
    pub active_tab: Option<String>,
    pub focus_restore: FocusRestoreTarget,
}

impl ShellPanelState {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        region: ShellRegion,
        extent: f32,
    ) -> Self {
        let extent = ShellExtent::new(extent);
        Self {
            id: id.into(),
            title: title.into(),
            placement: DockPlacement::Docked(region),
            extent,
            visible: true,
            resizable: false,
            collapsed: false,
            collapsed_extent: 0.0,
            restore_extent: extent.current,
            scroll_offset: UiPoint::new(0.0, 0.0),
            active_tab: None,
            focus_restore: FocusRestoreTarget::Previous,
        }
    }

    pub fn floating(id: impl Into<String>, title: impl Into<String>, extent: f32) -> Self {
        let mut panel = Self::new(id, title, ShellRegion::CenterWorkspace, extent);
        panel.placement = DockPlacement::Floating;
        panel
    }

    pub fn with_limits(mut self, min: f32, max: Option<f32>) -> Self {
        self.extent = self.extent.with_limits(min, max);
        self.restore_extent = self.extent.current;
        self
    }

    pub const fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        if !visible {
            self.placement = DockPlacement::Hidden;
        }
        self
    }

    pub fn active_tab(mut self, tab_id: impl Into<String>) -> Self {
        self.active_tab = Some(tab_id.into());
        self
    }

    pub const fn focus_restore(mut self, target: FocusRestoreTarget) -> Self {
        self.focus_restore = target;
        self
    }

    pub fn set_extent(&mut self, extent: f32) -> bool {
        if self.collapsed {
            self.restore_extent = self.extent.clamp(extent);
            return false;
        }
        let changed = self.extent.set(extent);
        if changed {
            self.restore_extent = self.extent.current;
        }
        changed
    }

    pub fn resize_by(&mut self, delta: f32) -> bool {
        if !self.resizable || self.collapsed {
            return false;
        }
        let changed = self.extent.resize_by(delta);
        if changed {
            self.restore_extent = self.extent.current;
        }
        changed
    }

    pub fn collapse(&mut self) -> bool {
        if self.collapsed {
            return false;
        }
        self.restore_extent = self.extent.current;
        self.extent.current = self.extent.clamp(self.collapsed_extent);
        self.collapsed = true;
        true
    }

    pub fn restore(&mut self) -> bool {
        if !self.collapsed {
            return false;
        }
        self.collapsed = false;
        self.extent.current = self.extent.clamp(self.restore_extent);
        true
    }

    pub fn set_scroll_offset(&mut self, offset: UiPoint) -> bool {
        if self.scroll_offset == offset {
            return false;
        }
        self.scroll_offset = offset;
        true
    }

    pub fn dock(&mut self, region: ShellRegion) {
        self.placement = DockPlacement::Docked(region);
        self.visible = true;
    }

    pub fn effective_extent(&self) -> f32 {
        if self.visible {
            self.extent.current.max(0.0)
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SplitPaneSide {
    First,
    Second,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PersistentSplitState {
    pub fraction: f32,
    pub min_first: f32,
    pub min_second: f32,
    pub collapsed: Option<SplitPaneSide>,
    restore_fraction: f32,
}

impl PersistentSplitState {
    pub fn new(fraction: f32) -> Self {
        let fraction = fraction.clamp(0.0, 1.0);
        Self {
            fraction,
            min_first: 48.0,
            min_second: 48.0,
            collapsed: None,
            restore_fraction: fraction,
        }
    }

    pub fn with_min_sizes(mut self, first: f32, second: f32) -> Self {
        self.min_first = first.max(0.0);
        self.min_second = second.max(0.0);
        self
    }

    pub fn set_fraction(&mut self, fraction: f32) -> bool {
        if !fraction.is_finite() || self.collapsed.is_some() {
            return false;
        }
        let fraction = fraction.clamp(0.0, 1.0);
        if (self.fraction - fraction).abs() <= f32::EPSILON {
            return false;
        }
        self.fraction = fraction;
        self.restore_fraction = fraction;
        true
    }

    pub fn keyboard_resize(&mut self, side: SplitPaneSide, step_fraction: f32) -> bool {
        let direction = match side {
            SplitPaneSide::First => 1.0,
            SplitPaneSide::Second => -1.0,
        };
        self.set_fraction(self.fraction + step_fraction * direction)
    }

    pub fn collapse(&mut self, side: SplitPaneSide) -> bool {
        if self.collapsed == Some(side) {
            return false;
        }
        if self.collapsed.is_none() {
            self.restore_fraction = self.fraction;
        }
        self.collapsed = Some(side);
        self.fraction = match side {
            SplitPaneSide::First => 0.0,
            SplitPaneSide::Second => 1.0,
        };
        true
    }

    pub fn restore(&mut self) -> bool {
        if self.collapsed.is_none() {
            return false;
        }
        self.collapsed = None;
        self.fraction = self.restore_fraction.clamp(0.0, 1.0);
        true
    }
}

impl Default for PersistentSplitState {
    fn default() -> Self {
        Self::new(0.5)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScrollSyncAxes {
    pub horizontal: bool,
    pub vertical: bool,
}

impl ScrollSyncAxes {
    pub const BOTH: Self = Self {
        horizontal: true,
        vertical: true,
    };
    pub const HORIZONTAL: Self = Self {
        horizontal: true,
        vertical: false,
    };
    pub const VERTICAL: Self = Self {
        horizontal: false,
        vertical: true,
    };
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScrollSyncMember {
    pub id: String,
    pub offset: UiPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScrollSyncGroup {
    pub id: String,
    pub axes: ScrollSyncAxes,
    pub members: Vec<ScrollSyncMember>,
}

impl ScrollSyncGroup {
    pub fn new(id: impl Into<String>, axes: ScrollSyncAxes) -> Self {
        Self {
            id: id.into(),
            axes,
            members: Vec::new(),
        }
    }

    pub fn add_member(mut self, id: impl Into<String>, offset: UiPoint) -> Self {
        self.members.push(ScrollSyncMember {
            id: id.into(),
            offset,
        });
        self
    }

    pub fn member_offset(&self, id: &str) -> Option<UiPoint> {
        self.members
            .iter()
            .find(|member| member.id == id)
            .map(|member| member.offset)
    }

    pub fn set_offset(&mut self, source: &str, offset: UiPoint) -> Vec<String> {
        let mut changed = Vec::new();
        for member in &mut self.members {
            let next = UiPoint::new(
                if self.axes.horizontal {
                    offset.x
                } else {
                    member.offset.x
                },
                if self.axes.vertical {
                    offset.y
                } else {
                    member.offset.y
                },
            );
            if member.offset != next {
                member.offset = next;
                if member.id != source {
                    changed.push(member.id.clone());
                }
            }
        }
        changed
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShellWorkspaceState {
    pub panels: Vec<ShellPanelState>,
    pub splits: HashMap<String, PersistentSplitState>,
    pub scroll_groups: Vec<ScrollSyncGroup>,
    pub focused_panel: Option<String>,
    pub restored_focus: Option<FocusRestoreTarget>,
}

impl ShellWorkspaceState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn panel(&self, id: &str) -> Option<&ShellPanelState> {
        self.panels.iter().find(|panel| panel.id == id)
    }

    pub fn panel_mut(&mut self, id: &str) -> Option<&mut ShellPanelState> {
        self.panels.iter_mut().find(|panel| panel.id == id)
    }

    pub fn upsert_panel(&mut self, panel: ShellPanelState) {
        if let Some(existing) = self.panel_mut(&panel.id) {
            *existing = panel;
        } else {
            self.panels.push(panel);
        }
    }

    pub fn visible_panels_in_region<'a>(
        &'a self,
        region: &'a ShellRegion,
    ) -> impl Iterator<Item = &'a ShellPanelState> + 'a {
        self.panels.iter().filter(move |panel| {
            panel.visible
                && matches!(&panel.placement, DockPlacement::Docked(docked) if docked == region)
        })
    }

    pub fn set_focused_panel(&mut self, id: impl Into<String>, restore: FocusRestoreTarget) {
        self.focused_panel = Some(id.into());
        self.restored_focus = Some(restore);
    }

    pub fn apply_scroll(&mut self, group_id: &str, source: &str, offset: UiPoint) -> Vec<String> {
        self.scroll_groups
            .iter_mut()
            .find(|group| group.id == group_id)
            .map(|group| group.set_offset(source, offset))
            .unwrap_or_default()
    }

    pub fn layout_for_size(&self, size: UiSize) -> ShellLayoutPlan {
        self.layout(UiRect::new(0.0, 0.0, size.width, size.height))
    }

    pub fn layout(&self, viewport: UiRect) -> ShellLayoutPlan {
        ShellLayoutPlan::from_workspace(self, viewport)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellPanelLayout {
    pub id: String,
    pub title: String,
    pub region: ShellRegion,
    pub rect: UiRect,
    pub scroll_offset: UiPoint,
    pub visible: bool,
    pub collapsed: bool,
    pub resizable: bool,
    pub active_tab: Option<String>,
}

impl ShellPanelLayout {
    fn from_panel(panel: &ShellPanelState, region: ShellRegion, rect: UiRect) -> Self {
        Self {
            id: panel.id.clone(),
            title: panel.title.clone(),
            region,
            rect,
            scroll_offset: panel.scroll_offset,
            visible: panel.visible,
            collapsed: panel.collapsed,
            resizable: panel.resizable,
            active_tab: panel.active_tab.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellRegionLayout {
    pub region: ShellRegion,
    pub rect: UiRect,
    pub panel_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellLayoutPlan {
    pub viewport: UiRect,
    pub workspace_rect: UiRect,
    pub regions: Vec<ShellRegionLayout>,
    pub panels: Vec<ShellPanelLayout>,
    pub floating_panels: Vec<ShellPanelLayout>,
    pub hidden_panel_ids: Vec<String>,
}

impl ShellLayoutPlan {
    pub fn from_workspace(workspace: &ShellWorkspaceState, viewport: UiRect) -> Self {
        let viewport = sanitize_rect(viewport);
        let planner = ShellLayoutPlanner::new(workspace, viewport);
        planner.plan()
    }

    pub fn region_rect(&self, region: &ShellRegion) -> Option<UiRect> {
        self.regions
            .iter()
            .find(|layout| &layout.region == region)
            .map(|layout| layout.rect)
    }

    pub fn panel_rect(&self, id: &str) -> Option<UiRect> {
        self.panels
            .iter()
            .chain(self.floating_panels.iter())
            .find(|panel| panel.id == id)
            .map(|panel| panel.rect)
    }

    pub fn region_panels<'a>(
        &'a self,
        region: &'a ShellRegion,
    ) -> impl Iterator<Item = &'a ShellPanelLayout> + 'a {
        self.panels
            .iter()
            .filter(move |panel| &panel.region == region)
    }
}

struct ShellLayoutPlanner<'a> {
    workspace: &'a ShellWorkspaceState,
    viewport: UiRect,
    remaining: UiRect,
    regions: Vec<ShellRegionLayout>,
    panels: Vec<ShellPanelLayout>,
    floating_panels: Vec<ShellPanelLayout>,
    hidden_panel_ids: Vec<String>,
}

impl<'a> ShellLayoutPlanner<'a> {
    fn new(workspace: &'a ShellWorkspaceState, viewport: UiRect) -> Self {
        Self {
            workspace,
            viewport,
            remaining: viewport,
            regions: Vec::new(),
            panels: Vec::new(),
            floating_panels: Vec::new(),
            hidden_panel_ids: Vec::new(),
        }
    }

    fn plan(mut self) -> ShellLayoutPlan {
        self.collect_floating_and_hidden();
        self.consume_top_region(ShellRegion::MenuBar);
        self.consume_top_region(ShellRegion::TransportBar);
        self.consume_top_region(ShellRegion::Toolbar);
        self.consume_bottom_region(ShellRegion::StatusBar);
        self.consume_bottom_region(ShellRegion::BottomPanel);
        self.consume_left_region(ShellRegion::LeftPanel);
        self.consume_right_region(ShellRegion::RightPanel);

        let workspace_rect = self.remaining;
        self.push_region(ShellRegion::CenterWorkspace, workspace_rect, Vec::new());
        self.plan_center_workspace(workspace_rect);

        ShellLayoutPlan {
            viewport: self.viewport,
            workspace_rect,
            regions: self.regions,
            panels: self.panels,
            floating_panels: self.floating_panels,
            hidden_panel_ids: self.hidden_panel_ids,
        }
    }

    fn collect_floating_and_hidden(&mut self) {
        for panel in &self.workspace.panels {
            match &panel.placement {
                DockPlacement::Hidden => self.hidden_panel_ids.push(panel.id.clone()),
                DockPlacement::Floating if panel.visible => {
                    let extent = panel.effective_extent();
                    let rect = UiRect::new(self.viewport.x, self.viewport.y, extent, extent);
                    self.floating_panels.push(ShellPanelLayout::from_panel(
                        panel,
                        ShellRegion::CenterWorkspace,
                        rect,
                    ));
                }
                DockPlacement::Floating => self.hidden_panel_ids.push(panel.id.clone()),
                DockPlacement::Docked(_) if !panel.visible => {
                    self.hidden_panel_ids.push(panel.id.clone())
                }
                DockPlacement::Docked(_) => {}
            }
        }
    }

    fn consume_top_region(&mut self, region: ShellRegion) {
        let panels = self.visible_docked_panels(region.clone());
        let extent = region_extent(&panels);
        if extent <= f32::EPSILON {
            return;
        }
        let height = extent.min(self.remaining.height);
        let rect = UiRect::new(
            self.remaining.x,
            self.remaining.y,
            self.remaining.width,
            height,
        );
        self.remaining.y += height;
        self.remaining.height = (self.remaining.height - height).max(0.0);
        self.push_panel_region(region, rect, panels);
    }

    fn consume_bottom_region(&mut self, region: ShellRegion) {
        let panels = self.visible_docked_panels(region.clone());
        let extent = region_extent(&panels);
        if extent <= f32::EPSILON {
            return;
        }
        let height = extent.min(self.remaining.height);
        let rect = UiRect::new(
            self.remaining.x,
            self.remaining.bottom() - height,
            self.remaining.width,
            height,
        );
        self.remaining.height = (self.remaining.height - height).max(0.0);
        self.push_panel_region(region, rect, panels);
    }

    fn consume_left_region(&mut self, region: ShellRegion) {
        let panels = self.visible_docked_panels(region.clone());
        let extent = region_extent(&panels);
        if extent <= f32::EPSILON {
            return;
        }
        let width = extent.min(self.remaining.width);
        let rect = UiRect::new(
            self.remaining.x,
            self.remaining.y,
            width,
            self.remaining.height,
        );
        self.remaining.x += width;
        self.remaining.width = (self.remaining.width - width).max(0.0);
        self.push_panel_region(region, rect, panels);
    }

    fn consume_right_region(&mut self, region: ShellRegion) {
        let panels = self.visible_docked_panels(region.clone());
        let extent = region_extent(&panels);
        if extent <= f32::EPSILON {
            return;
        }
        let width = extent.min(self.remaining.width);
        let rect = UiRect::new(
            self.remaining.right() - width,
            self.remaining.y,
            width,
            self.remaining.height,
        );
        self.remaining.width = (self.remaining.width - width).max(0.0);
        self.push_panel_region(region, rect, panels);
    }

    fn plan_center_workspace(&mut self, workspace_rect: UiRect) {
        let mut center = workspace_rect;
        let editor_panels = self.visible_docked_panels(ShellRegion::Editor);
        let editor_extent = region_extent(&editor_panels);
        if editor_extent > f32::EPSILON {
            let height = editor_extent.min(center.height);
            let rect = UiRect::new(center.x, center.bottom() - height, center.width, height);
            center.height = (center.height - height).max(0.0);
            self.push_panel_region(ShellRegion::Editor, rect, editor_panels);
        }

        let track_panels = self.visible_docked_panels(ShellRegion::TrackList);
        let track_extent = region_extent(&track_panels);
        if track_extent > f32::EPSILON {
            let width = track_extent.min(center.width);
            let rect = UiRect::new(center.x, center.y, width, center.height);
            center.x += width;
            center.width = (center.width - width).max(0.0);
            self.push_panel_region(ShellRegion::TrackList, rect, track_panels);
        }

        let arrangement_panels = self.visible_docked_panels(ShellRegion::Arrangement);
        if !arrangement_panels.is_empty() || center.width > 0.0 || center.height > 0.0 {
            self.push_panel_region(ShellRegion::Arrangement, center, arrangement_panels);
        }
    }

    fn visible_docked_panels(&self, region: ShellRegion) -> Vec<&'a ShellPanelState> {
        self.workspace
            .panels
            .iter()
            .filter(move |panel| {
                panel.visible
                    && matches!(&panel.placement, DockPlacement::Docked(docked) if *docked == region)
            })
            .collect()
    }

    fn push_panel_region(
        &mut self,
        region: ShellRegion,
        rect: UiRect,
        panels: Vec<&'a ShellPanelState>,
    ) {
        let panel_ids = panels
            .iter()
            .map(|panel| panel.id.clone())
            .collect::<Vec<_>>();
        for panel in panels {
            self.panels
                .push(ShellPanelLayout::from_panel(panel, region.clone(), rect));
        }
        self.push_region(region, rect, panel_ids);
    }

    fn push_region(&mut self, region: ShellRegion, rect: UiRect, panel_ids: Vec<String>) {
        self.regions.push(ShellRegionLayout {
            region,
            rect,
            panel_ids,
        });
    }
}

fn region_extent(panels: &[&ShellPanelState]) -> f32 {
    panels
        .iter()
        .map(|panel| panel.effective_extent())
        .fold(0.0_f32, f32::max)
}

fn sanitize_rect(rect: UiRect) -> UiRect {
    UiRect::new(
        finite_or_zero(rect.x),
        finite_or_zero(rect.y),
        finite_nonnegative(rect.width),
        finite_nonnegative(rect.height),
    )
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
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
    use crate::UiNodeId;

    #[test]
    fn panel_state_clamps_resizes_collapses_and_restores() {
        let mut panel = ShellPanelState::new("browser", "Browser", ShellRegion::LeftPanel, 180.0)
            .with_limits(96.0, Some(280.0))
            .resizable(true)
            .active_tab("files")
            .focus_restore(FocusRestoreTarget::Node(UiNodeId(42)));

        assert!(panel.resize_by(200.0));
        assert_eq!(panel.extent.current, 280.0);
        assert_eq!(panel.restore_extent, 280.0);
        assert_eq!(panel.active_tab.as_deref(), Some("files"));

        panel.collapsed_extent = 24.0;
        assert!(panel.collapse());
        assert!(panel.collapsed);
        assert_eq!(panel.extent.current, 96.0);
        assert!(!panel.resize_by(-40.0));
        assert!(panel.restore());
        assert_eq!(panel.extent.current, 280.0);

        panel.dock(ShellRegion::RightPanel);
        assert_eq!(
            panel.placement,
            DockPlacement::Docked(ShellRegion::RightPanel)
        );
        assert!(panel.visible);
    }

    #[test]
    fn split_state_supports_keyboard_resize_and_collapse_restore() {
        let mut split = PersistentSplitState::new(0.5).with_min_sizes(120.0, 80.0);

        assert!(split.keyboard_resize(SplitPaneSide::First, 0.1));
        assert_eq!(split.fraction, 0.6);
        assert!(split.collapse(SplitPaneSide::Second));
        assert_eq!(split.fraction, 1.0);
        assert!(!split.keyboard_resize(SplitPaneSide::Second, 0.1));
        assert!(split.restore());
        assert_eq!(split.fraction, 0.6);
        assert_eq!(split.collapsed, None);
    }

    #[test]
    fn scroll_sync_group_mirrors_configured_axes() {
        let mut group = ScrollSyncGroup::new("arrangement", ScrollSyncAxes::VERTICAL)
            .add_member("track-list", UiPoint::new(12.0, 0.0))
            .add_member("timeline", UiPoint::new(40.0, 0.0));

        let changed = group.set_offset("timeline", UiPoint::new(90.0, 320.0));
        assert_eq!(changed, vec!["track-list"]);
        assert_eq!(
            group.member_offset("track-list"),
            Some(UiPoint::new(12.0, 320.0))
        );
        assert_eq!(
            group.member_offset("timeline"),
            Some(UiPoint::new(40.0, 320.0))
        );
    }

    #[test]
    fn workspace_state_persists_panels_splits_focus_and_scroll_groups() {
        let mut workspace = ShellWorkspaceState::new();
        workspace.upsert_panel(
            ShellPanelState::new("transport", "Transport", ShellRegion::TransportBar, 44.0)
                .visible(true),
        );
        workspace.upsert_panel(
            ShellPanelState::new("inspector", "Inspector", ShellRegion::RightPanel, 260.0)
                .with_limits(160.0, Some(420.0))
                .resizable(true),
        );
        workspace
            .splits
            .insert("main".to_string(), PersistentSplitState::new(0.72));
        workspace.scroll_groups.push(
            ScrollSyncGroup::new("tracks", ScrollSyncAxes::VERTICAL)
                .add_member("track-list", UiPoint::new(0.0, 0.0))
                .add_member("arrangement", UiPoint::new(0.0, 0.0)),
        );
        workspace.set_focused_panel("inspector", FocusRestoreTarget::Previous);

        assert_eq!(
            workspace
                .visible_panels_in_region(&ShellRegion::RightPanel)
                .map(|panel| panel.id.as_str())
                .collect::<Vec<_>>(),
            vec!["inspector"]
        );
        assert_eq!(
            workspace.apply_scroll("tracks", "arrangement", UiPoint::new(0.0, 128.0)),
            vec!["track-list"]
        );
        assert_eq!(
            workspace.panel("inspector").map(|panel| panel.extent.max),
            Some(Some(420.0))
        );
        assert_eq!(workspace.focused_panel.as_deref(), Some("inspector"));
        assert_eq!(workspace.splits["main"].fraction, 0.72);
    }

    #[test]
    fn workspace_layout_plan_consumes_shell_edges_and_editor_regions() {
        let mut workspace = ShellWorkspaceState::new();
        workspace.upsert_panel(ShellPanelState::new(
            "menu",
            "Menu",
            ShellRegion::MenuBar,
            24.0,
        ));
        workspace.upsert_panel(ShellPanelState::new(
            "transport",
            "Transport",
            ShellRegion::TransportBar,
            40.0,
        ));
        workspace.upsert_panel(ShellPanelState::new(
            "status",
            "Status",
            ShellRegion::StatusBar,
            20.0,
        ));
        workspace.upsert_panel(
            ShellPanelState::new("browser", "Browser", ShellRegion::LeftPanel, 180.0)
                .resizable(true),
        );
        workspace.upsert_panel(
            ShellPanelState::new("inspector", "Inspector", ShellRegion::RightPanel, 220.0)
                .resizable(true)
                .active_tab("scale-lab"),
        );
        workspace.upsert_panel(ShellPanelState::new(
            "tracks",
            "Tracks",
            ShellRegion::TrackList,
            140.0,
        ));
        workspace.upsert_panel(ShellPanelState::new(
            "arrangement",
            "Arrangement",
            ShellRegion::Arrangement,
            1.0,
        ));
        workspace.upsert_panel(ShellPanelState::new(
            "piano-roll",
            "Piano Roll",
            ShellRegion::Editor,
            160.0,
        ));

        let plan = workspace.layout_for_size(UiSize::new(1000.0, 700.0));

        assert_eq!(
            plan.region_rect(&ShellRegion::MenuBar),
            Some(UiRect::new(0.0, 0.0, 1000.0, 24.0))
        );
        assert_eq!(
            plan.region_rect(&ShellRegion::TransportBar),
            Some(UiRect::new(0.0, 24.0, 1000.0, 40.0))
        );
        assert_eq!(
            plan.region_rect(&ShellRegion::StatusBar),
            Some(UiRect::new(0.0, 680.0, 1000.0, 20.0))
        );
        assert_eq!(
            plan.region_rect(&ShellRegion::LeftPanel),
            Some(UiRect::new(0.0, 64.0, 180.0, 616.0))
        );
        assert_eq!(
            plan.region_rect(&ShellRegion::RightPanel),
            Some(UiRect::new(780.0, 64.0, 220.0, 616.0))
        );
        assert_eq!(plan.workspace_rect, UiRect::new(180.0, 64.0, 600.0, 616.0));
        assert_eq!(
            plan.region_rect(&ShellRegion::Editor),
            Some(UiRect::new(180.0, 520.0, 600.0, 160.0))
        );
        assert_eq!(
            plan.region_rect(&ShellRegion::TrackList),
            Some(UiRect::new(180.0, 64.0, 140.0, 456.0))
        );
        assert_eq!(
            plan.region_rect(&ShellRegion::Arrangement),
            Some(UiRect::new(320.0, 64.0, 460.0, 456.0))
        );
        assert_eq!(
            plan.panel_rect("inspector"),
            Some(UiRect::new(780.0, 64.0, 220.0, 616.0))
        );
        assert_eq!(
            plan.region_panels(&ShellRegion::RightPanel)
                .next()
                .and_then(|panel| panel.active_tab.as_deref()),
            Some("scale-lab")
        );
    }

    #[test]
    fn workspace_layout_tracks_hidden_floating_and_collapsed_panels() {
        let mut workspace = ShellWorkspaceState::new();
        let mut left = ShellPanelState::new("left", "Left", ShellRegion::LeftPanel, 240.0);
        left.collapsed_extent = 32.0;
        assert!(left.collapse());
        workspace.upsert_panel(left);
        workspace.upsert_panel(
            ShellPanelState::new("hidden", "Hidden", ShellRegion::RightPanel, 180.0).visible(false),
        );
        workspace.upsert_panel(ShellPanelState::floating("floating", "Floating", 320.0));

        let plan = workspace.layout(UiRect::new(10.0, 20.0, 800.0, 600.0));

        assert_eq!(
            plan.region_rect(&ShellRegion::LeftPanel),
            Some(UiRect::new(10.0, 20.0, 32.0, 600.0))
        );
        assert_eq!(plan.panel_rect("hidden"), None);
        assert_eq!(plan.hidden_panel_ids, vec!["hidden"]);
        assert_eq!(
            plan.panel_rect("floating"),
            Some(UiRect::new(10.0, 20.0, 320.0, 320.0))
        );
        assert!(plan
            .region_panels(&ShellRegion::LeftPanel)
            .next()
            .is_some_and(|panel| panel.collapsed));
    }
}
