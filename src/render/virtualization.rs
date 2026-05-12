//! Backend-neutral virtualization contracts for large retained collections.
//!
//! The planner models the shared row/axis work for lists, tables, trees, and
//! grids without coupling to a renderer. It accepts sparse measurements so
//! large collections do not require per-row allocation.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VirtualCollectionKind {
    List,
    Table,
    Tree,
    Grid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VirtualAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VirtualItemKey {
    Index(u64),
    Cell { row: u64, column: u64 },
    Stable(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualExtent {
    pub estimated: f32,
    pub measured: Option<f32>,
}

impl VirtualExtent {
    pub const fn estimated(estimated: f32) -> Self {
        Self {
            estimated,
            measured: None,
        }
    }

    pub const fn measured(estimated: f32, measured: f32) -> Self {
        Self {
            estimated,
            measured: Some(measured),
        }
    }

    pub fn resolved(self) -> f32 {
        finite_extent(self.measured.unwrap_or(self.estimated))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualMeasuredExtent {
    pub index: u64,
    pub extent: f32,
}

impl VirtualMeasuredExtent {
    pub const fn new(index: u64, extent: f32) -> Self {
        Self { index, extent }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualOverscan {
    pub before: u64,
    pub after: u64,
}

impl VirtualOverscan {
    pub const NONE: Self = Self {
        before: 0,
        after: 0,
    };

    pub const fn uniform(count: u64) -> Self {
        Self {
            before: count,
            after: count,
        }
    }

    pub const fn new(before: u64, after: u64) -> Self {
        Self { before, after }
    }
}

impl Default for VirtualOverscan {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VirtualStickyEdge {
    Header,
    Footer,
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualStickyRegion {
    pub key: VirtualItemKey,
    pub index: u64,
    pub edge: VirtualStickyEdge,
    pub extent: f32,
    pub z_order: i32,
}

impl VirtualStickyRegion {
    pub fn new(index: u64, edge: VirtualStickyEdge, extent: f32) -> Self {
        Self {
            key: VirtualItemKey::Index(index),
            index,
            edge,
            extent: finite_extent(extent),
            z_order: 0,
        }
    }

    pub fn key(mut self, key: VirtualItemKey) -> Self {
        self.key = key;
        self
    }

    pub const fn z_order(mut self, z_order: i32) -> Self {
        self.z_order = z_order;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualFocusPreservation {
    pub key: VirtualItemKey,
    pub index_before: Option<u64>,
    pub index_after: Option<u64>,
}

impl VirtualFocusPreservation {
    pub const fn new(
        key: VirtualItemKey,
        index_before: Option<u64>,
        index_after: Option<u64>,
    ) -> Self {
        Self {
            key,
            index_before,
            index_after,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualSelectionPreservation {
    pub keys: Vec<VirtualItemKey>,
    pub anchor: Option<VirtualItemKey>,
}

impl VirtualSelectionPreservation {
    pub fn new(keys: Vec<VirtualItemKey>) -> Self {
        Self { keys, anchor: None }
    }

    pub fn anchor(mut self, anchor: VirtualItemKey) -> Self {
        self.anchor = Some(anchor);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualAccessibilityRecord {
    pub visible_start_index: u64,
    pub visible_count: u64,
    pub total_count: u64,
}

impl VirtualAccessibilityRecord {
    pub const fn new(visible_start_index: u64, visible_count: u64, total_count: u64) -> Self {
        Self {
            visible_start_index,
            visible_count,
            total_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualScrollAnchor {
    pub key: VirtualItemKey,
    pub index: u64,
}

impl VirtualScrollAnchor {
    pub const fn new(key: VirtualItemKey, index: u64) -> Self {
        Self { key, index }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualScrollAnchorAdjustment {
    pub anchor: VirtualScrollAnchor,
    pub offset_before: f32,
    pub offset_after: f32,
    pub delta: f32,
    pub scroll_offset_after: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualItemPlan {
    pub key: VirtualItemKey,
    pub index: u64,
    pub start: f32,
    pub extent: f32,
    pub sticky: Option<VirtualStickyEdge>,
    pub z_order: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualPlan {
    pub kind: VirtualCollectionKind,
    pub axis: VirtualAxis,
    pub item_count: u64,
    pub content_extent: f32,
    pub visible_range: Range<u64>,
    pub materialized_range: Range<u64>,
    pub items: Vec<VirtualItemPlan>,
    pub sticky_items: Vec<VirtualItemPlan>,
    pub accessibility: VirtualAccessibilityRecord,
    pub focus: Option<VirtualFocusPreservation>,
    pub selection: Option<VirtualSelectionPreservation>,
}

impl VirtualPlan {
    pub fn visible_count(&self) -> u64 {
        self.visible_range
            .end
            .saturating_sub(self.visible_range.start)
    }

    pub fn materialized_count(&self) -> u64 {
        self.materialized_range
            .end
            .saturating_sub(self.materialized_range.start)
    }
}

#[derive(Debug, Clone)]
pub struct VirtualPlanRequest<'a> {
    pub kind: VirtualCollectionKind,
    pub axis: VirtualAxis,
    pub item_count: u64,
    pub estimated_extent: f32,
    pub viewport_extent: f32,
    pub scroll_offset: f32,
    pub measured_extents: &'a [VirtualMeasuredExtent],
    pub overscan: VirtualOverscan,
    pub sticky_regions: &'a [VirtualStickyRegion],
    pub focus: Option<VirtualFocusPreservation>,
    pub selection: Option<VirtualSelectionPreservation>,
}

impl<'a> VirtualPlanRequest<'a> {
    pub const fn new(item_count: u64, estimated_extent: f32, viewport_extent: f32) -> Self {
        Self {
            kind: VirtualCollectionKind::List,
            axis: VirtualAxis::Vertical,
            item_count,
            estimated_extent,
            viewport_extent,
            scroll_offset: 0.0,
            measured_extents: &[],
            overscan: VirtualOverscan::NONE,
            sticky_regions: &[],
            focus: None,
            selection: None,
        }
    }

    pub const fn kind(mut self, kind: VirtualCollectionKind) -> Self {
        self.kind = kind;
        self
    }

    pub const fn axis(mut self, axis: VirtualAxis) -> Self {
        self.axis = axis;
        self
    }

    pub fn scroll_offset(mut self, scroll_offset: f32) -> Self {
        self.scroll_offset = finite_non_negative(scroll_offset);
        self
    }

    pub const fn measured_extents(mut self, measured_extents: &'a [VirtualMeasuredExtent]) -> Self {
        self.measured_extents = measured_extents;
        self
    }

    pub const fn overscan(mut self, overscan: VirtualOverscan) -> Self {
        self.overscan = overscan;
        self
    }

    pub const fn sticky_regions(mut self, sticky_regions: &'a [VirtualStickyRegion]) -> Self {
        self.sticky_regions = sticky_regions;
        self
    }

    pub fn focus(mut self, focus: VirtualFocusPreservation) -> Self {
        self.focus = Some(focus);
        self
    }

    pub fn selection(mut self, selection: VirtualSelectionPreservation) -> Self {
        self.selection = Some(selection);
        self
    }
}

pub fn plan_virtualized_range(request: VirtualPlanRequest<'_>) -> VirtualPlan {
    let estimated_extent = finite_extent(request.estimated_extent);
    let viewport_extent = finite_non_negative(request.viewport_extent);
    let content_extent = virtual_offset_for_index(
        request.item_count,
        estimated_extent,
        request.measured_extents,
    );
    let max_scroll = (content_extent - viewport_extent).max(0.0);
    let scroll_offset = finite_non_negative(request.scroll_offset).min(max_scroll);

    let visible_start = first_index_at_or_after(
        scroll_offset,
        request.item_count,
        estimated_extent,
        request.measured_extents,
    );
    let visible_end = first_index_with_start_at_or_after(
        scroll_offset + viewport_extent,
        request.item_count,
        estimated_extent,
        request.measured_extents,
    )
    .min(request.item_count);
    let visible_end = if visible_start < request.item_count {
        visible_end.max(visible_start + 1)
    } else {
        visible_end
    };
    let materialized_start = visible_start.saturating_sub(request.overscan.before);
    let materialized_end = visible_end
        .saturating_add(request.overscan.after)
        .min(request.item_count);

    let mut items = Vec::with_capacity(range_len(materialized_start, materialized_end));
    for index in materialized_start..materialized_end {
        items.push(item_plan(
            index,
            estimated_extent,
            request.measured_extents,
            None,
        ));
    }

    let mut sticky_items = Vec::new();
    for region in request.sticky_regions {
        if region.index >= request.item_count {
            continue;
        }
        let mut item = item_plan(
            region.index,
            estimated_extent,
            request.measured_extents,
            Some(region.edge),
        );
        item.key = region.key.clone();
        item.extent = finite_extent(region.extent);
        item.z_order = region.z_order;
        sticky_items.push(item);
    }
    sticky_items.sort_by_key(|item| item.z_order);

    VirtualPlan {
        kind: request.kind,
        axis: request.axis,
        item_count: request.item_count,
        content_extent,
        visible_range: visible_start..visible_end,
        materialized_range: materialized_start..materialized_end,
        items,
        sticky_items,
        accessibility: VirtualAccessibilityRecord::new(
            visible_start,
            visible_end.saturating_sub(visible_start),
            request.item_count,
        ),
        focus: request.focus,
        selection: request.selection,
    }
}

pub fn virtual_scroll_anchor_adjustment(
    anchor: VirtualScrollAnchor,
    scroll_offset_before: f32,
    estimated_extent: f32,
    previous_measured_extents: &[VirtualMeasuredExtent],
    next_measured_extents: &[VirtualMeasuredExtent],
) -> VirtualScrollAnchorAdjustment {
    let estimated_extent = finite_extent(estimated_extent);
    let offset_before =
        virtual_offset_for_index(anchor.index, estimated_extent, previous_measured_extents);
    let offset_after =
        virtual_offset_for_index(anchor.index, estimated_extent, next_measured_extents);
    let delta = offset_after - offset_before;
    VirtualScrollAnchorAdjustment {
        anchor,
        offset_before,
        offset_after,
        delta,
        scroll_offset_after: finite_non_negative(scroll_offset_before + delta),
    }
}

pub fn virtual_offset_for_index(
    index: u64,
    estimated_extent: f32,
    measured_extents: &[VirtualMeasuredExtent],
) -> f32 {
    let estimated_extent = finite_extent(estimated_extent);
    let base = (index as f32) * estimated_extent;
    measured_extents
        .iter()
        .filter(|measurement| measurement.index < index)
        .map(|measurement| finite_extent(measurement.extent) - estimated_extent)
        .sum::<f32>()
        + base
}

fn item_plan(
    index: u64,
    estimated_extent: f32,
    measured_extents: &[VirtualMeasuredExtent],
    sticky: Option<VirtualStickyEdge>,
) -> VirtualItemPlan {
    VirtualItemPlan {
        key: VirtualItemKey::Index(index),
        index,
        start: virtual_offset_for_index(index, estimated_extent, measured_extents),
        extent: measured_extent_for(index, estimated_extent, measured_extents),
        sticky,
        z_order: 0,
    }
}

fn first_index_at_or_after(
    offset: f32,
    item_count: u64,
    estimated_extent: f32,
    measured_extents: &[VirtualMeasuredExtent],
) -> u64 {
    if item_count == 0 {
        return 0;
    }
    let offset = finite_non_negative(offset);
    let mut low = 0;
    let mut high = item_count;
    while low < high {
        let mid = low + (high - low) / 2;
        let mid_end = virtual_offset_for_index(mid + 1, estimated_extent, measured_extents);
        if mid_end <= offset {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low.min(item_count)
}

fn first_index_with_start_at_or_after(
    offset: f32,
    item_count: u64,
    estimated_extent: f32,
    measured_extents: &[VirtualMeasuredExtent],
) -> u64 {
    let offset = finite_non_negative(offset);
    let mut low = 0;
    let mut high = item_count;
    while low < high {
        let mid = low + (high - low) / 2;
        let mid_start = virtual_offset_for_index(mid, estimated_extent, measured_extents);
        if mid_start < offset {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low.min(item_count)
}

fn measured_extent_for(
    index: u64,
    estimated_extent: f32,
    measured_extents: &[VirtualMeasuredExtent],
) -> f32 {
    measured_extents
        .iter()
        .find(|measurement| measurement.index == index)
        .map(|measurement| finite_extent(measurement.extent))
        .unwrap_or_else(|| finite_extent(estimated_extent))
}

fn range_len(start: u64, end: u64) -> usize {
    end.saturating_sub(start).min(usize::MAX as u64) as usize
}

fn finite_extent(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_row_count_materializes_only_visible_window() {
        let plan = plan_virtualized_range(
            VirtualPlanRequest::new(1_000_000_000, 20.0, 100.0)
                .scroll_offset(2_000_000.0)
                .overscan(VirtualOverscan::new(2, 3)),
        );

        assert_eq!(plan.visible_range, 100_000..100_005);
        assert_eq!(plan.materialized_range, 99_998..100_008);
        assert_eq!(plan.items.len(), 10);
        assert_eq!(plan.content_extent, 20_000_000_000.0);
        assert_eq!(
            plan.items.first().map(|item| item.key.clone()),
            Some(VirtualItemKey::Index(99_998))
        );
    }

    #[test]
    fn measured_row_heights_adjust_offsets_and_visible_range() {
        let measured = [
            VirtualMeasuredExtent::new(1, 40.0),
            VirtualMeasuredExtent::new(3, 10.0),
        ];
        let plan = plan_virtualized_range(
            VirtualPlanRequest::new(6, 20.0, 45.0)
                .scroll_offset(35.0)
                .measured_extents(&measured),
        );

        assert_eq!(plan.visible_range, 1..3);
        assert_eq!(plan.items.len(), 2);
        assert_eq!(plan.items[0].start, 20.0);
        assert_eq!(plan.items[0].extent, 40.0);
        assert_eq!(plan.items[1].start, 60.0);
        assert_eq!(plan.content_extent, 130.0);
    }

    #[test]
    fn sticky_regions_are_reported_apart_from_materialized_rows() {
        let sticky = [
            VirtualStickyRegion::new(0, VirtualStickyEdge::Header, 32.0)
                .key(VirtualItemKey::Stable("header".to_string()))
                .z_order(10),
            VirtualStickyRegion::new(99, VirtualStickyEdge::Footer, 24.0)
                .key(VirtualItemKey::Stable("summary".to_string())),
        ];
        let plan = plan_virtualized_range(
            VirtualPlanRequest::new(100, 20.0, 60.0)
                .scroll_offset(500.0)
                .sticky_regions(&sticky),
        );

        assert_eq!(plan.visible_range, 25..28);
        assert_eq!(plan.sticky_items.len(), 2);
        assert_eq!(
            plan.sticky_items[0].key,
            VirtualItemKey::Stable("summary".to_string())
        );
        assert_eq!(
            plan.sticky_items[1].key,
            VirtualItemKey::Stable("header".to_string())
        );
        assert_eq!(plan.sticky_items[1].sticky, Some(VirtualStickyEdge::Header));
        assert_eq!(plan.sticky_items[1].z_order, 10);
        assert!(!plan.items.iter().any(|item| item.index == 0));
    }

    #[test]
    fn grid_contracts_preserve_collection_axis_and_cell_keys() {
        let sticky = [VirtualStickyRegion::new(0, VirtualStickyEdge::Start, 18.0)
            .key(VirtualItemKey::Cell { row: 0, column: 0 })];
        let plan = plan_virtualized_range(
            VirtualPlanRequest::new(10, 12.0, 24.0)
                .kind(VirtualCollectionKind::Grid)
                .axis(VirtualAxis::Horizontal)
                .sticky_regions(&sticky),
        );

        assert_eq!(plan.kind, VirtualCollectionKind::Grid);
        assert_eq!(plan.axis, VirtualAxis::Horizontal);
        assert_eq!(
            plan.sticky_items[0].key,
            VirtualItemKey::Cell { row: 0, column: 0 }
        );
    }

    #[test]
    fn overscan_expands_materialized_range_without_changing_visible_range() {
        let plan = plan_virtualized_range(
            VirtualPlanRequest::new(20, 10.0, 30.0)
                .scroll_offset(50.0)
                .overscan(VirtualOverscan::new(2, 4)),
        );

        assert_eq!(plan.visible_range, 5..8);
        assert_eq!(plan.materialized_range, 3..12);
        assert_eq!(plan.visible_count(), 3);
        assert_eq!(plan.materialized_count(), 9);
    }

    #[test]
    fn focus_and_selection_preservation_records_round_trip() {
        let focus =
            VirtualFocusPreservation::new(VirtualItemKey::Stable("row-9".into()), Some(9), Some(8));
        let selection = VirtualSelectionPreservation::new(vec![
            VirtualItemKey::Stable("row-1".into()),
            VirtualItemKey::Stable("row-9".into()),
        ])
        .anchor(VirtualItemKey::Stable("row-1".into()));

        let plan = plan_virtualized_range(
            VirtualPlanRequest::new(100, 20.0, 80.0)
                .focus(focus.clone())
                .selection(selection.clone()),
        );

        assert_eq!(plan.focus, Some(focus));
        assert_eq!(plan.selection, Some(selection));
    }

    #[test]
    fn accessibility_reports_visible_indices_and_total_count() {
        let plan =
            plan_virtualized_range(VirtualPlanRequest::new(50, 10.0, 25.0).scroll_offset(120.0));

        assert_eq!(
            plan.accessibility,
            VirtualAccessibilityRecord::new(12, 3, 50)
        );
    }

    #[test]
    fn scroll_anchor_adjusts_after_measured_size_changes_before_anchor() {
        let previous = [VirtualMeasuredExtent::new(2, 20.0)];
        let next = [
            VirtualMeasuredExtent::new(2, 35.0),
            VirtualMeasuredExtent::new(4, 30.0),
        ];
        let adjustment = virtual_scroll_anchor_adjustment(
            VirtualScrollAnchor::new(VirtualItemKey::Stable("row-5".into()), 5),
            70.0,
            20.0,
            &previous,
            &next,
        );

        assert_eq!(adjustment.offset_before, 100.0);
        assert_eq!(adjustment.offset_after, 125.0);
        assert_eq!(adjustment.delta, 25.0);
        assert_eq!(adjustment.scroll_offset_after, 95.0);
    }
}
