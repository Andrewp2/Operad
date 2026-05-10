//! Persistable app-shell and workspace state contracts.
//!
//! This module is intentionally data-first. Widget builders can consume these
//! records, but applications can also save and restore them without depending on
//! a renderer or on the concrete widget tree that happened to produce them.

use std::collections::HashMap;

use taffy::prelude::{
    Dimension, LengthPercentageAuto, Position, Rect as TaffyRect, Size as TaffySize, Style,
};

use crate::{
    accessibility::FocusRestoreTarget, AccessibilityAction, AccessibilityMeta, AccessibilityRole,
    AccessibilityValueRange, ClipBehavior, InputBehavior, LayoutStyle, UiDocument, UiNode,
    UiNodeId, UiNodeStyle, UiPoint, UiRect, UiSize,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShellRegion {
    MenuBar,
    TransportBar,
    Toolbar,
    LeftPanel,
    RightPanel,
    BottomPanel,
    StatusBar,
    LaneList,
    Timeline,
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
        matches!(self, Self::LaneList | Self::Timeline | Self::Editor)
    }

    pub fn stable_key(&self) -> String {
        match self {
            Self::MenuBar => "menu-bar".to_string(),
            Self::TransportBar => "transport-bar".to_string(),
            Self::Toolbar => "toolbar".to_string(),
            Self::LeftPanel => "left-panel".to_string(),
            Self::RightPanel => "right-panel".to_string(),
            Self::BottomPanel => "bottom-panel".to_string(),
            Self::StatusBar => "status-bar".to_string(),
            Self::LaneList => "lane-list".to_string(),
            Self::Timeline => "timeline".to_string(),
            Self::Editor => "editor".to_string(),
            Self::CenterWorkspace => "center-workspace".to_string(),
            Self::Custom(id) => format!("custom.{id}"),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::MenuBar => "Menu bar".to_string(),
            Self::TransportBar => "Transport bar".to_string(),
            Self::Toolbar => "Toolbar".to_string(),
            Self::LeftPanel => "Left panel".to_string(),
            Self::RightPanel => "Right panel".to_string(),
            Self::BottomPanel => "Bottom panel".to_string(),
            Self::StatusBar => "Status bar".to_string(),
            Self::LaneList => "Lane list".to_string(),
            Self::Timeline => "Timeline".to_string(),
            Self::Editor => "Editor".to_string(),
            Self::CenterWorkspace => "Center workspace".to_string(),
            Self::Custom(id) => id.clone(),
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellBarItemRole {
    Command,
    Toggle,
    Readout,
    Separator,
    Spacer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellBarOverflowPolicy {
    Never,
    WhenNeeded,
    Always,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellNumericReadout {
    pub value: String,
    pub unit: Option<String>,
    pub precision: Option<u8>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}

impl ShellNumericReadout {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            unit: None,
            precision: None,
            min_value: None,
            max_value: None,
        }
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub const fn precision(mut self, precision: u8) -> Self {
        self.precision = Some(precision);
        self
    }

    pub const fn range(mut self, min_value: f64, max_value: f64) -> Self {
        self.min_value = Some(min_value);
        self.max_value = Some(max_value);
        self
    }

    pub fn accessibility_value(&self) -> String {
        if let Some(unit) = &self.unit {
            if self.value.is_empty() {
                unit.clone()
            } else {
                format!("{} {}", self.value, unit)
            }
        } else {
            self.value.clone()
        }
    }

    pub fn accessibility_range(&self) -> Option<AccessibilityValueRange> {
        let (Some(min), Some(max)) = (self.min_value, self.max_value) else {
            return None;
        };
        if !min.is_finite() || !max.is_finite() {
            return None;
        }

        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        let range = AccessibilityValueRange::new(min, max);
        Some(if let Some(precision) = self.precision {
            range.with_step(10_f64.powi(-(precision.min(12) as i32)))
        } else {
            range
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellBarItem {
    pub id: String,
    pub label: String,
    pub command_id: Option<String>,
    pub role: ShellBarItemRole,
    pub enabled: bool,
    pub active: bool,
    pub pressed: bool,
    pub priority: i32,
    pub min_width: f32,
    pub preferred_width: f32,
    pub overflow_policy: ShellBarOverflowPolicy,
    pub readout: Option<ShellNumericReadout>,
}

impl ShellBarItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>, role: ShellBarItemRole) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            command_id: None,
            role,
            enabled: true,
            active: false,
            pressed: false,
            priority: 0,
            min_width: 32.0,
            preferred_width: 40.0,
            overflow_policy: ShellBarOverflowPolicy::WhenNeeded,
            readout: None,
        }
    }

    pub fn command(
        id: impl Into<String>,
        label: impl Into<String>,
        command_id: impl Into<String>,
    ) -> Self {
        Self::new(id, label, ShellBarItemRole::Command).command_id(command_id)
    }

    pub fn toggle(
        id: impl Into<String>,
        label: impl Into<String>,
        command_id: impl Into<String>,
    ) -> Self {
        Self::new(id, label, ShellBarItemRole::Toggle).command_id(command_id)
    }

    pub fn readout(
        id: impl Into<String>,
        label: impl Into<String>,
        readout: ShellNumericReadout,
    ) -> Self {
        Self::new(id, label, ShellBarItemRole::Readout)
            .with_readout(readout)
            .overflow_policy(ShellBarOverflowPolicy::Never)
            .widths(56.0, 72.0)
    }

    pub fn command_id(mut self, command_id: impl Into<String>) -> Self {
        self.command_id = Some(command_id.into());
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub const fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub const fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub const fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn widths(mut self, min_width: f32, preferred_width: f32) -> Self {
        self.min_width = finite_nonnegative(min_width);
        self.preferred_width = finite_nonnegative(preferred_width).max(self.min_width);
        self
    }

    pub const fn overflow_policy(mut self, overflow_policy: ShellBarOverflowPolicy) -> Self {
        self.overflow_policy = overflow_policy;
        self
    }

    pub fn with_readout(mut self, readout: ShellNumericReadout) -> Self {
        self.readout = Some(readout);
        self
    }

    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        shell_bar_accessibility_meta(ShellBarAccessibility {
            label: &self.label,
            command_id: self.command_id.as_deref(),
            role: self.role,
            enabled: self.enabled,
            active: self.active,
            pressed: self.pressed,
            readout: self.readout.as_ref(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellBarCluster {
    pub id: String,
    pub label: String,
    pub items: Vec<ShellBarItem>,
}

impl ShellBarCluster {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            items: Vec::new(),
        }
    }

    pub fn add_item(mut self, item: ShellBarItem) -> Self {
        self.items.push(item);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellBarLayoutSpacing {
    pub item_gap: f32,
    pub cluster_gap: f32,
}

impl ShellBarLayoutSpacing {
    pub fn new(item_gap: f32, cluster_gap: f32) -> Self {
        Self {
            item_gap: finite_nonnegative(item_gap),
            cluster_gap: finite_nonnegative(cluster_gap),
        }
    }
}

impl Default for ShellBarLayoutSpacing {
    fn default() -> Self {
        Self::new(4.0, 8.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellBarItemLayout {
    pub id: String,
    pub cluster_id: Option<String>,
    pub label: String,
    pub command_id: Option<String>,
    pub role: ShellBarItemRole,
    pub enabled: bool,
    pub active: bool,
    pub pressed: bool,
    pub x: f32,
    pub width: f32,
    pub min_width: f32,
    pub preferred_width: f32,
    pub readout: Option<ShellNumericReadout>,
}

impl ShellBarItemLayout {
    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        shell_bar_accessibility_meta(ShellBarAccessibility {
            label: &self.label,
            command_id: self.command_id.as_deref(),
            role: self.role,
            enabled: self.enabled,
            active: self.active,
            pressed: self.pressed,
            readout: self.readout.as_ref(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellBarOverflowItem {
    pub id: String,
    pub cluster_id: Option<String>,
    pub label: String,
    pub command_id: Option<String>,
    pub role: ShellBarItemRole,
    pub enabled: bool,
    pub active: bool,
    pub pressed: bool,
    pub priority: i32,
    pub readout: Option<ShellNumericReadout>,
}

impl ShellBarOverflowItem {
    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        shell_bar_accessibility_meta(ShellBarAccessibility {
            label: &self.label,
            command_id: self.command_id.as_deref(),
            role: self.role,
            enabled: self.enabled,
            active: self.active,
            pressed: self.pressed,
            readout: self.readout.as_ref(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellBarLayoutPlan {
    pub available_width: f32,
    pub used_width: f32,
    pub visible_items: Vec<ShellBarItemLayout>,
    pub overflow_items: Vec<ShellBarOverflowItem>,
    pub clipped: bool,
}

impl ShellBarLayoutPlan {
    pub fn from_items(items: &[ShellBarItem], available_width: f32) -> Self {
        Self::from_items_with_spacing(items, available_width, ShellBarLayoutSpacing::default())
    }

    pub fn from_items_with_spacing(
        items: &[ShellBarItem],
        available_width: f32,
        spacing: ShellBarLayoutSpacing,
    ) -> Self {
        let entries = items
            .iter()
            .map(|item| ShellBarPlanEntry {
                cluster_id: None,
                item,
            })
            .collect::<Vec<_>>();
        Self::from_entries(&entries, available_width, spacing)
    }

    pub fn from_clusters(clusters: &[ShellBarCluster], available_width: f32) -> Self {
        Self::from_clusters_with_spacing(
            clusters,
            available_width,
            ShellBarLayoutSpacing::default(),
        )
    }

    pub fn from_clusters_with_spacing(
        clusters: &[ShellBarCluster],
        available_width: f32,
        spacing: ShellBarLayoutSpacing,
    ) -> Self {
        let entries = clusters
            .iter()
            .flat_map(|cluster| {
                cluster.items.iter().map(|item| ShellBarPlanEntry {
                    cluster_id: Some(cluster.id.as_str()),
                    item,
                })
            })
            .collect::<Vec<_>>();
        Self::from_entries(&entries, available_width, spacing)
    }

    fn from_entries(
        entries: &[ShellBarPlanEntry<'_>],
        available_width: f32,
        spacing: ShellBarLayoutSpacing,
    ) -> Self {
        let available_width = finite_nonnegative(available_width);
        let mut visible = entries
            .iter()
            .map(|entry| entry.item.overflow_policy != ShellBarOverflowPolicy::Always)
            .collect::<Vec<_>>();

        loop {
            if bar_width(entries, &visible, spacing, BarWidthMode::Min) <= available_width {
                break;
            }

            let Some(index) = visible
                .iter()
                .enumerate()
                .filter(|(index, is_visible)| {
                    **is_visible
                        && entries[*index].item.overflow_policy
                            == ShellBarOverflowPolicy::WhenNeeded
                })
                .min_by_key(|(index, _)| (entries[*index].item.priority, usize::MAX - *index))
                .map(|(index, _)| index)
            else {
                break;
            };

            visible[index] = false;
        }

        let min_width = bar_width(entries, &visible, spacing, BarWidthMode::Min);
        let preferred_width = bar_width(entries, &visible, spacing, BarWidthMode::Preferred);
        let clipped = min_width > available_width;
        let target_width = if preferred_width <= available_width {
            preferred_width
        } else {
            min_width.max(available_width)
        };
        let extra_width = (target_width - min_width).max(0.0);
        let flexible_width = entries
            .iter()
            .enumerate()
            .filter(|(index, _)| visible[*index])
            .map(|(_, entry)| {
                (item_preferred_width(entry.item) - item_min_width(entry.item)).max(0.0)
            })
            .sum::<f32>();

        let mut visible_items = Vec::new();
        let mut overflow_items = Vec::new();
        let mut x = 0.0;
        let mut previous_cluster_id: Option<&str> = None;

        for (index, entry) in entries.iter().enumerate() {
            if visible[index] {
                if !visible_items.is_empty() {
                    x += gap_between(previous_cluster_id, entry.cluster_id, spacing);
                }

                let min_width = item_min_width(entry.item);
                let preferred_width = item_preferred_width(entry.item);
                let flex = (preferred_width - min_width).max(0.0);
                let width = if flexible_width > f32::EPSILON {
                    min_width + extra_width * (flex / flexible_width)
                } else {
                    min_width
                };

                visible_items.push(entry.layout(x, width));
                x += width;
                previous_cluster_id = entry.cluster_id;
            } else {
                overflow_items.push(entry.overflow_item());
            }
        }

        Self {
            available_width,
            used_width: x,
            visible_items,
            overflow_items,
            clipped,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ShellBarPlanEntry<'a> {
    cluster_id: Option<&'a str>,
    item: &'a ShellBarItem,
}

impl ShellBarPlanEntry<'_> {
    fn layout(self, x: f32, width: f32) -> ShellBarItemLayout {
        ShellBarItemLayout {
            id: self.item.id.clone(),
            cluster_id: self.cluster_id.map(str::to_string),
            label: self.item.label.clone(),
            command_id: self.item.command_id.clone(),
            role: self.item.role,
            enabled: self.item.enabled,
            active: self.item.active,
            pressed: self.item.pressed,
            x,
            width,
            min_width: item_min_width(self.item),
            preferred_width: item_preferred_width(self.item),
            readout: self.item.readout.clone(),
        }
    }

    fn overflow_item(self) -> ShellBarOverflowItem {
        ShellBarOverflowItem {
            id: self.item.id.clone(),
            cluster_id: self.cluster_id.map(str::to_string),
            label: self.item.label.clone(),
            command_id: self.item.command_id.clone(),
            role: self.item.role,
            enabled: self.item.enabled,
            active: self.item.active,
            pressed: self.item.pressed,
            priority: self.item.priority,
            readout: self.item.readout.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BarWidthMode {
    Min,
    Preferred,
}

fn bar_width(
    entries: &[ShellBarPlanEntry<'_>],
    visible: &[bool],
    spacing: ShellBarLayoutSpacing,
    mode: BarWidthMode,
) -> f32 {
    let mut width = 0.0;
    let mut previous_cluster_id = None;
    let mut has_visible_item = false;

    for (index, entry) in entries.iter().enumerate() {
        if !visible[index] {
            continue;
        }

        if has_visible_item {
            width += gap_between(previous_cluster_id, entry.cluster_id, spacing);
        }

        width += match mode {
            BarWidthMode::Min => item_min_width(entry.item),
            BarWidthMode::Preferred => item_preferred_width(entry.item),
        };
        previous_cluster_id = entry.cluster_id;
        has_visible_item = true;
    }

    width
}

fn gap_between(
    previous_cluster_id: Option<&str>,
    cluster_id: Option<&str>,
    spacing: ShellBarLayoutSpacing,
) -> f32 {
    if previous_cluster_id == cluster_id {
        spacing.item_gap
    } else {
        spacing.cluster_gap
    }
}

fn item_min_width(item: &ShellBarItem) -> f32 {
    finite_nonnegative(item.min_width)
}

fn item_preferred_width(item: &ShellBarItem) -> f32 {
    finite_nonnegative(item.preferred_width).max(item_min_width(item))
}

#[derive(Debug, Clone, Copy)]
struct ShellBarAccessibility<'a> {
    label: &'a str,
    command_id: Option<&'a str>,
    role: ShellBarItemRole,
    enabled: bool,
    active: bool,
    pressed: bool,
    readout: Option<&'a ShellNumericReadout>,
}

fn shell_bar_accessibility_meta(item: ShellBarAccessibility<'_>) -> AccessibilityMeta {
    let mut meta = AccessibilityMeta::new(shell_bar_accessibility_role(item.role, item.readout))
        .label(item.label.to_owned());

    match item.role {
        ShellBarItemRole::Command => {
            meta = meta.focusable().pressed(item.pressed);
        }
        ShellBarItemRole::Toggle => {
            meta = meta.focusable().checked(item.active).pressed(item.pressed);
        }
        ShellBarItemRole::Readout => {
            meta = meta.read_only();
            if let Some(readout) = item.readout {
                meta = meta.value(readout.accessibility_value());
                if let Some(range) = readout.accessibility_range() {
                    meta = meta.value_range(range);
                }
            }
        }
        ShellBarItemRole::Separator => {}
        ShellBarItemRole::Spacer => {
            meta = meta.hidden();
        }
    }

    if let Some(command_id) = item.command_id {
        meta = meta.action(AccessibilityAction::new(
            command_id.to_owned(),
            item.label.to_owned(),
        ));
    }
    if item.enabled {
        meta
    } else {
        meta.disabled()
    }
}

fn shell_bar_accessibility_role(
    role: ShellBarItemRole,
    readout: Option<&ShellNumericReadout>,
) -> AccessibilityRole {
    match role {
        ShellBarItemRole::Command => AccessibilityRole::Button,
        ShellBarItemRole::Toggle => AccessibilityRole::ToggleButton,
        ShellBarItemRole::Readout
            if readout
                .and_then(ShellNumericReadout::accessibility_range)
                .is_some() =>
        {
            AccessibilityRole::Meter
        }
        ShellBarItemRole::Readout => AccessibilityRole::Status,
        ShellBarItemRole::Separator => AccessibilityRole::Separator,
        ShellBarItemRole::Spacer => AccessibilityRole::Group,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellDocumentOptions {
    pub include_empty_regions: bool,
    pub include_hidden_panels: bool,
    pub include_resize_handles: bool,
    pub resize_handle_thickness: f32,
}

impl Default for ShellDocumentOptions {
    fn default() -> Self {
        Self {
            include_empty_regions: true,
            include_hidden_panels: true,
            include_resize_handles: true,
            resize_handle_thickness: 4.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellRegionDocumentNode {
    pub region: ShellRegion,
    pub node: UiNodeId,
    pub rect: UiRect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellPanelDocumentNode {
    pub id: String,
    pub title: String,
    pub node: UiNodeId,
    pub content: Option<UiNodeId>,
    pub resize_handle: Option<UiNodeId>,
    pub rect: UiRect,
    pub region: Option<ShellRegion>,
    pub floating: bool,
    pub hidden: bool,
    pub collapsed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellDocumentNodes {
    pub root: UiNodeId,
    pub regions: Vec<ShellRegionDocumentNode>,
    pub panels: Vec<ShellPanelDocumentNode>,
}

impl ShellDocumentNodes {
    pub fn region(&self, region: &ShellRegion) -> Option<&ShellRegionDocumentNode> {
        self.regions.iter().find(|node| &node.region == region)
    }

    pub fn region_node(&self, region: &ShellRegion) -> Option<UiNodeId> {
        self.region(region).map(|node| node.node)
    }

    pub fn panel(&self, id: &str) -> Option<&ShellPanelDocumentNode> {
        self.panels.iter().find(|panel| panel.id == id)
    }

    pub fn panel_node(&self, id: &str) -> Option<UiNodeId> {
        self.panel(id).map(|panel| panel.node)
    }
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

    pub fn build_document(
        &self,
        document: &mut UiDocument,
        parent: UiNodeId,
        name: impl Into<String>,
        options: ShellDocumentOptions,
        build_panel: impl FnMut(&mut UiDocument, UiNodeId, &ShellPanelLayout),
    ) -> ShellDocumentNodes {
        build_shell_document(document, parent, name, self, options, build_panel)
    }
}

pub fn build_shell_document(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    plan: &ShellLayoutPlan,
    options: ShellDocumentOptions,
    mut build_panel: impl FnMut(&mut UiDocument, UiNodeId, &ShellPanelLayout),
) -> ShellDocumentNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            shell_node_style(plan.viewport, UiPoint::new(0.0, 0.0)),
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Application)
                .label(format!("{name} shell workspace"))
                .hint("Contains shell regions and panels"),
        ),
    );
    let origin = UiPoint::new(plan.viewport.x, plan.viewport.y);
    let mut regions = Vec::new();
    let mut panels = Vec::new();

    for region in &plan.regions {
        if !options.include_empty_regions && region.panel_ids.is_empty() {
            continue;
        }
        let node = document.add_child(
            root,
            UiNode::container(
                format!("{name}.region.{}", region.region.stable_key()),
                shell_node_style(region.rect, origin),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Group).label(region.region.label()),
            ),
        );
        regions.push(ShellRegionDocumentNode {
            region: region.region.clone(),
            node,
            rect: region.rect,
        });
    }

    for panel in &plan.panels {
        let region_node = regions
            .iter()
            .find(|region| region.region == panel.region)
            .map(|region| (region.node, UiPoint::new(region.rect.x, region.rect.y)));
        let (parent, parent_origin) =
            region_node.unwrap_or((root, UiPoint::new(plan.viewport.x, plan.viewport.y)));
        panels.push(add_shell_panel_document_node(
            document,
            &name,
            panel,
            ShellPanelDocumentTarget {
                parent,
                origin: parent_origin,
                region: Some(panel.region.clone()),
                floating: false,
            },
            options,
            &mut build_panel,
        ));
    }

    for panel in &plan.floating_panels {
        panels.push(add_shell_panel_document_node(
            document,
            &name,
            panel,
            ShellPanelDocumentTarget {
                parent: root,
                origin: UiPoint::new(plan.viewport.x, plan.viewport.y),
                region: Some(panel.region.clone()),
                floating: true,
            },
            options,
            &mut build_panel,
        ));
    }

    if options.include_hidden_panels {
        for id in &plan.hidden_panel_ids {
            let node = document.add_child(
                root,
                UiNode::container(
                    format!("{name}.panel.{id}.hidden"),
                    shell_node_style(
                        UiRect::new(plan.viewport.x, plan.viewport.y, 0.0, 0.0),
                        origin,
                    ),
                )
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Group)
                        .label(id.clone())
                        .hidden(),
                ),
            );
            panels.push(ShellPanelDocumentNode {
                id: id.clone(),
                title: id.clone(),
                node,
                content: None,
                resize_handle: None,
                rect: UiRect::new(0.0, 0.0, 0.0, 0.0),
                region: None,
                floating: false,
                hidden: true,
                collapsed: false,
            });
        }
    }

    ShellDocumentNodes {
        root,
        regions,
        panels,
    }
}

fn add_shell_panel_document_node(
    document: &mut UiDocument,
    workspace_name: &str,
    panel: &ShellPanelLayout,
    target: ShellPanelDocumentTarget,
    options: ShellDocumentOptions,
    build_panel: &mut impl FnMut(&mut UiDocument, UiNodeId, &ShellPanelLayout),
) -> ShellPanelDocumentNode {
    let node = document.add_child(
        target.parent,
        UiNode::container(
            format!("{workspace_name}.panel.{}", panel.id),
            shell_node_style(panel.rect, target.origin),
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::TabPanel)
                .label(panel.title.clone())
                .value(panel_state_value(panel)),
        ),
    );
    let content = document.add_child(
        node,
        UiNode::container(
            format!("{workspace_name}.panel.{}.content", panel.id),
            shell_node_style(
                UiRect::new(0.0, 0.0, panel.rect.width, panel.rect.height),
                UiPoint::new(0.0, 0.0),
            ),
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group)
                .label(format!("{} content", panel.title)),
        ),
    );
    build_panel(document, content, panel);

    let resize_handle = if options.include_resize_handles && panel.resizable {
        let rect = resize_handle_rect(panel, options.resize_handle_thickness);
        Some(
            document.add_child(
                node,
                UiNode::container(
                    format!("{workspace_name}.panel.{}.resize", panel.id),
                    shell_node_style(rect, UiPoint::new(0.0, 0.0)),
                )
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Slider)
                        .label(format!("{} resize handle", panel.title))
                        .value(format!("{:.0}px", panel_extent(panel)))
                        .focusable(),
                ),
            ),
        )
    } else {
        None
    };

    ShellPanelDocumentNode {
        id: panel.id.clone(),
        title: panel.title.clone(),
        node,
        content: Some(content),
        resize_handle,
        rect: panel.rect,
        region: target.region,
        floating: target.floating,
        hidden: false,
        collapsed: panel.collapsed,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ShellPanelDocumentTarget {
    parent: UiNodeId,
    origin: UiPoint,
    region: Option<ShellRegion>,
    floating: bool,
}

fn shell_node_style(rect: UiRect, origin: UiPoint) -> UiNodeStyle {
    let x = finite_or_zero(rect.x - origin.x);
    let y = finite_or_zero(rect.y - origin.y);
    let width = finite_nonnegative(rect.width);
    let height = finite_nonnegative(rect.height);

    UiNodeStyle {
        layout: LayoutStyle::from_taffy_style(Style {
            position: Position::Absolute,
            inset: TaffyRect {
                left: LengthPercentageAuto::length(x),
                top: LengthPercentageAuto::length(y),
                right: LengthPercentageAuto::auto(),
                bottom: LengthPercentageAuto::auto(),
            },
            size: TaffySize {
                width: Dimension::length(width),
                height: Dimension::length(height),
            },
            ..Default::default()
        }),
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

fn panel_state_value(panel: &ShellPanelLayout) -> String {
    let mut parts = Vec::new();
    parts.push(if panel.collapsed {
        "collapsed".to_string()
    } else {
        "expanded".to_string()
    });
    if let Some(tab) = &panel.active_tab {
        parts.push(format!("active tab {tab}"));
    }
    parts.join(", ")
}

fn panel_extent(panel: &ShellPanelLayout) -> f32 {
    match panel.region {
        ShellRegion::LeftPanel | ShellRegion::RightPanel | ShellRegion::LaneList => {
            panel.rect.width
        }
        ShellRegion::MenuBar
        | ShellRegion::TransportBar
        | ShellRegion::Toolbar
        | ShellRegion::BottomPanel
        | ShellRegion::StatusBar
        | ShellRegion::Editor => panel.rect.height,
        ShellRegion::Timeline | ShellRegion::CenterWorkspace | ShellRegion::Custom(_) => {
            panel.rect.width.max(panel.rect.height)
        }
    }
}

fn resize_handle_rect(panel: &ShellPanelLayout, thickness: f32) -> UiRect {
    let thickness = finite_nonnegative(thickness).min(panel.rect.width.max(panel.rect.height));
    match panel.region {
        ShellRegion::RightPanel => UiRect::new(0.0, 0.0, thickness, panel.rect.height),
        ShellRegion::BottomPanel | ShellRegion::Editor | ShellRegion::StatusBar => {
            UiRect::new(0.0, 0.0, panel.rect.width, thickness)
        }
        ShellRegion::MenuBar | ShellRegion::TransportBar | ShellRegion::Toolbar => UiRect::new(
            0.0,
            (panel.rect.height - thickness).max(0.0),
            panel.rect.width,
            thickness,
        ),
        ShellRegion::LeftPanel
        | ShellRegion::LaneList
        | ShellRegion::Timeline
        | ShellRegion::CenterWorkspace
        | ShellRegion::Custom(_) => UiRect::new(
            (panel.rect.width - thickness).max(0.0),
            0.0,
            thickness,
            panel.rect.height,
        ),
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

        let lane_panels = self.visible_docked_panels(ShellRegion::LaneList);
        let lane_extent = region_extent(&lane_panels);
        if lane_extent > f32::EPSILON {
            let width = lane_extent.min(center.width);
            let rect = UiRect::new(center.x, center.y, width, center.height);
            center.x += width;
            center.width = (center.width - width).max(0.0);
            self.push_panel_region(ShellRegion::LaneList, rect, lane_panels);
        }

        let timeline_panels = self.visible_docked_panels(ShellRegion::Timeline);
        if !timeline_panels.is_empty() || center.width > 0.0 || center.height > 0.0 {
            self.push_panel_region(ShellRegion::Timeline, center, timeline_panels);
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
    use crate::{root_style, ApproxTextMeasurer, TextStyle, UiDocument, UiNodeId};

    #[test]
    fn shell_bar_layout_keeps_transport_readouts_and_overflows_low_priority_items() {
        let clusters = vec![
            ShellBarCluster::new("transport", "Transport")
                .add_item(
                    ShellBarItem::command("stop", "Stop", "transport.stop")
                        .overflow_policy(ShellBarOverflowPolicy::Never)
                        .widths(28.0, 32.0),
                )
                .add_item(
                    ShellBarItem::toggle("play", "Play", "transport.play")
                        .priority(100)
                        .active(true)
                        .pressed(true)
                        .widths(32.0, 40.0),
                )
                .add_item(ShellBarItem::readout(
                    "tempo",
                    "Tempo",
                    ShellNumericReadout::new("120.0")
                        .unit("bpm")
                        .precision(1)
                        .range(20.0, 300.0),
                )),
            ShellBarCluster::new("tools", "Tools")
                .add_item(
                    ShellBarItem::toggle("quantize", "Quantize", "edit.quantize")
                        .priority(10)
                        .widths(44.0, 64.0),
                )
                .add_item(
                    ShellBarItem::toggle("metro", "Metronome", "transport.metronome")
                        .priority(50)
                        .enabled(false)
                        .active(true)
                        .widths(36.0, 44.0),
                ),
        ];

        let plan = ShellBarLayoutPlan::from_clusters(&clusters, 160.0);

        assert_eq!(
            plan.visible_items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["stop", "play", "tempo"]
        );
        assert_eq!(
            plan.overflow_items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["quantize", "metro"]
        );
        assert_eq!(
            plan.visible_items[1].command_id.as_deref(),
            Some("transport.play")
        );
        assert!(plan.visible_items[1].active);
        assert!(plan.visible_items[1].pressed);
        assert_eq!(
            plan.visible_items[2].cluster_id.as_deref(),
            Some("transport")
        );
        assert_eq!(
            plan.visible_items[2]
                .readout
                .as_ref()
                .and_then(|readout| readout.unit.as_deref()),
            Some("bpm")
        );
        assert!(!plan.clipped);
    }

    #[test]
    fn shell_bar_layout_shrinks_visible_items_before_overflowing_them() {
        let items = vec![
            ShellBarItem::command("select", "Select", "tool.select").widths(40.0, 80.0),
            ShellBarItem::command("draw", "Draw", "tool.draw").widths(40.0, 80.0),
        ];

        let plan = ShellBarLayoutPlan::from_items_with_spacing(
            &items,
            100.0,
            ShellBarLayoutSpacing::new(4.0, 4.0),
        );

        assert_eq!(plan.overflow_items, Vec::new());
        assert_eq!(
            plan.visible_items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["select", "draw"]
        );
        assert_eq!(plan.visible_items[0].width, 48.0);
        assert_eq!(plan.visible_items[1].x, 52.0);
        assert_eq!(plan.visible_items[1].width, 48.0);
        assert_eq!(plan.used_width, 100.0);
    }

    #[test]
    fn shell_bar_items_export_accessibility_metadata() {
        let toggle = ShellBarItem::toggle("loop", "Loop", "transport.loop")
            .active(true)
            .pressed(true);
        let toggle_meta = toggle.accessibility_meta();
        assert_eq!(toggle_meta.role, AccessibilityRole::ToggleButton);
        assert_eq!(toggle_meta.label.as_deref(), Some("Loop"));
        assert_eq!(toggle_meta.checked, Some(crate::AccessibilityChecked::True));
        assert_eq!(toggle_meta.pressed, Some(true));
        assert!(toggle_meta.focusable);
        assert_eq!(toggle_meta.actions[0].id, "transport.loop");

        let disabled = ShellBarItem::command("record", "Record", "transport.record")
            .enabled(false)
            .pressed(true);
        let disabled_meta = disabled.accessibility_meta();
        assert_eq!(disabled_meta.role, AccessibilityRole::Button);
        assert!(!disabled_meta.enabled);
        assert_eq!(disabled_meta.pressed, Some(true));

        let readout = ShellBarItem::readout(
            "cpu",
            "CPU",
            ShellNumericReadout::new("18")
                .unit("%")
                .precision(0)
                .range(0.0, 100.0),
        );
        let readout_meta = readout.accessibility_meta();
        assert_eq!(readout_meta.role, AccessibilityRole::Meter);
        assert_eq!(readout_meta.value.as_deref(), Some("18 %"));
        assert_eq!(
            readout_meta.value_range,
            Some(AccessibilityValueRange::new(0.0, 100.0).with_step(1.0))
        );
        assert!(readout_meta.read_only);

        let spacer = ShellBarItem::new("fill", "", ShellBarItemRole::Spacer);
        assert!(spacer.accessibility_meta().hidden);
    }

    #[test]
    fn shell_bar_layout_items_preserve_accessibility_state() {
        let items = vec![
            ShellBarItem::toggle("snap", "Snap", "edit.snap")
                .active(true)
                .widths(40.0, 56.0),
            ShellBarItem::command("grid", "Grid", "view.grid")
                .overflow_policy(ShellBarOverflowPolicy::Always),
        ];
        let plan = ShellBarLayoutPlan::from_items(&items, 80.0);

        let visible_meta = plan.visible_items[0].accessibility_meta();
        assert_eq!(visible_meta.role, AccessibilityRole::ToggleButton);
        assert_eq!(
            visible_meta.checked,
            Some(crate::AccessibilityChecked::True)
        );
        assert_eq!(visible_meta.actions[0].id, "edit.snap");

        let overflow_meta = plan.overflow_items[0].accessibility_meta();
        assert_eq!(overflow_meta.role, AccessibilityRole::Button);
        assert_eq!(overflow_meta.label.as_deref(), Some("Grid"));
        assert_eq!(overflow_meta.actions[0].id, "view.grid");
    }

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
        let mut group = ScrollSyncGroup::new("timeline", ScrollSyncAxes::VERTICAL)
            .add_member("lane-list", UiPoint::new(12.0, 0.0))
            .add_member("timeline", UiPoint::new(40.0, 0.0));

        let changed = group.set_offset("timeline", UiPoint::new(90.0, 320.0));
        assert_eq!(changed, vec!["lane-list"]);
        assert_eq!(
            group.member_offset("lane-list"),
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
            ScrollSyncGroup::new("lanes", ScrollSyncAxes::VERTICAL)
                .add_member("lane-list", UiPoint::new(0.0, 0.0))
                .add_member("timeline", UiPoint::new(0.0, 0.0)),
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
            workspace.apply_scroll("lanes", "timeline", UiPoint::new(0.0, 128.0)),
            vec!["lane-list"]
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
            "lanes",
            "Lanes",
            ShellRegion::LaneList,
            140.0,
        ));
        workspace.upsert_panel(ShellPanelState::new(
            "timeline",
            "Timeline",
            ShellRegion::Timeline,
            1.0,
        ));
        workspace.upsert_panel(ShellPanelState::new(
            "value-grid",
            "Value Grid",
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
            plan.region_rect(&ShellRegion::LaneList),
            Some(UiRect::new(180.0, 64.0, 140.0, 456.0))
        );
        assert_eq!(
            plan.region_rect(&ShellRegion::Timeline),
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
    fn workspace_layout_lanes_hidden_floating_and_collapsed_panels() {
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

    #[test]
    fn shell_layout_plan_builds_stable_document_nodes() {
        let mut workspace = ShellWorkspaceState::new();
        workspace.upsert_panel(ShellPanelState::new(
            "menu",
            "Menu",
            ShellRegion::MenuBar,
            24.0,
        ));
        let mut browser = ShellPanelState::new("browser", "Browser", ShellRegion::LeftPanel, 180.0)
            .resizable(true)
            .active_tab("assets");
        browser.collapsed_extent = 32.0;
        assert!(browser.collapse());
        workspace.upsert_panel(browser);
        workspace.upsert_panel(ShellPanelState::new(
            "timeline",
            "Timeline",
            ShellRegion::Timeline,
            1.0,
        ));
        workspace.upsert_panel(ShellPanelState::new(
            "value-grid",
            "Value Grid",
            ShellRegion::Editor,
            160.0,
        ));
        workspace.upsert_panel(
            ShellPanelState::new("inspector", "Inspector", ShellRegion::RightPanel, 220.0)
                .visible(false),
        );

        let plan = workspace.layout_for_size(UiSize::new(800.0, 500.0));
        let mut document = UiDocument::new(root_style(800.0, 500.0));
        let root = document.root;
        let nodes = build_shell_document(
            &mut document,
            root,
            "shell",
            &plan,
            ShellDocumentOptions::default(),
            |document, parent, panel| {
                document.add_child(
                    parent,
                    UiNode::text(
                        format!("shell.panel.{}.label", panel.id),
                        panel.title.clone(),
                        TextStyle::default(),
                        LayoutStyle::from_taffy_style(Style {
                            size: TaffySize {
                                width: Dimension::auto(),
                                height: Dimension::auto(),
                            },
                            ..Default::default()
                        }),
                    ),
                );
            },
        );

        document
            .compute_layout(UiSize::new(800.0, 500.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(document.node(nodes.root).name, "shell");
        assert!(nodes.region_node(&ShellRegion::MenuBar).is_some());
        assert!(nodes.region_node(&ShellRegion::LeftPanel).is_some());
        assert!(nodes.panel("timeline").is_some());

        let browser = nodes.panel("browser").expect("browser node");
        assert!(browser.collapsed);
        assert_eq!(browser.rect, plan.panel_rect("browser").unwrap());
        assert_eq!(
            document.node(browser.node).layout.rect,
            plan.panel_rect("browser").unwrap()
        );
        assert_eq!(
            document.node(browser.content.unwrap()).layout.rect,
            plan.panel_rect("browser").unwrap()
        );
        assert_eq!(
            document
                .node(browser.node)
                .accessibility
                .as_ref()
                .unwrap()
                .value,
            Some("collapsed, active tab assets".to_string())
        );

        let handle = browser.resize_handle.expect("resize handle");
        let browser_rect = plan.panel_rect("browser").unwrap();
        assert_eq!(
            document.node(handle).layout.rect,
            UiRect::new(
                browser_rect.right() - 4.0,
                browser_rect.y,
                4.0,
                browser_rect.height
            )
        );
        let handle_accessibility = document.node(handle).accessibility.as_ref().unwrap();
        assert_eq!(handle_accessibility.role, AccessibilityRole::Slider);
        assert!(handle_accessibility.focusable);
        assert!(document.node(handle).input.focusable);

        let hidden = nodes.panel("inspector").expect("hidden node");
        assert!(hidden.hidden);
        assert!(hidden.content.is_none());
        let hidden_node = document.node(hidden.node);
        assert!(hidden_node.accessibility.as_ref().unwrap().hidden);
        assert_eq!(hidden_node.layout.rect.width, 0.0);
        assert_eq!(hidden_node.layout.rect.height, 0.0);
    }
}
