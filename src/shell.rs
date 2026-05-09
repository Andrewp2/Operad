//! Persistable app-shell and workspace state contracts.
//!
//! This module is intentionally data-first. Widget builders can consume these
//! records, but applications can also save and restore them without depending on
//! a renderer or on the concrete widget tree that happened to produce them.

use std::collections::HashMap;

use crate::{accessibility::FocusRestoreTarget, UiPoint};

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
}
