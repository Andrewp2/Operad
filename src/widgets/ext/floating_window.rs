//! Floating window desktop widget.

use std::collections::{HashMap, HashSet};

use binpack2d::{
    dimension::Dimension as PackDimension,
    maxrects::{Heuristic as MaxRectsHeuristic, MaxRectsBin},
};
use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentage, Rect,
    Size as TaffySize, Style,
};

use crate::{
    layout, length, AccessibilityMeta, AccessibilityRole, ClipBehavior, ColorRgba, EditPhase,
    InputBehavior, InteractionVisuals, LayoutStyle, ScenePrimitive, StrokeStyle, TextStyle,
    UiDocument, UiNode, UiNodeId, UiNodeLayoutConstraint, UiNodeStyle, UiPoint, UiRect, UiSize,
    UiVisual, WidgetActionBinding, WidgetActionMode, WidgetPointerEdit,
};

use super::surfaces::{DEFAULT_SURFACE_BG, DEFAULT_SURFACE_STROKE};

const ORGANIZE_PACK_SCALE: f32 = 4.0;

#[derive(Debug, Clone, PartialEq)]
pub struct FloatingWindowDescriptor {
    pub id: String,
    pub title: String,
    pub preferred_size: UiSize,
    pub min_size: UiSize,
    pub content_min_size: Option<UiSize>,
    pub visible: bool,
    pub collapsed: bool,
    pub position: Option<UiPoint>,
    pub z_index: Option<i16>,
    pub activate_action: Option<WidgetActionBinding>,
    pub title_action: Option<WidgetActionBinding>,
    pub drag_action: Option<WidgetActionBinding>,
    pub collapse_action: Option<WidgetActionBinding>,
    pub close_action: Option<WidgetActionBinding>,
    pub resize_action: Option<WidgetActionBinding>,
    pub auto_size_to_content: bool,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl FloatingWindowDescriptor {
    pub fn new(id: impl Into<String>, title: impl Into<String>, preferred_size: UiSize) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            preferred_size,
            min_size: UiSize::new(160.0, 96.0),
            content_min_size: None,
            visible: true,
            collapsed: false,
            position: None,
            z_index: None,
            activate_action: None,
            title_action: None,
            drag_action: None,
            collapse_action: None,
            close_action: None,
            resize_action: None,
            auto_size_to_content: false,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }

    pub fn with_min_size(mut self, min_size: UiSize) -> Self {
        self.min_size = min_size;
        self
    }

    pub fn with_content_min_size(mut self, content_min_size: UiSize) -> Self {
        self.content_min_size = Some(content_min_size);
        self
    }

    pub fn with_position(mut self, position: UiPoint) -> Self {
        self.position = Some(position);
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub const fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn with_z_index(mut self, z_index: i16) -> Self {
        self.z_index = Some(z_index);
        self
    }

    pub fn with_activate_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.activate_action = Some(action.into());
        self
    }

    pub fn with_title_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.title_action = Some(action.into());
        self
    }

    pub fn with_drag_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.drag_action = Some(action.into());
        self
    }

    pub fn with_collapse_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.collapse_action = Some(action.into());
        self
    }

    pub fn with_close_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.close_action = Some(action.into());
        self
    }

    pub fn with_resize_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.resize_action = Some(action.into());
        self
    }

    pub const fn with_auto_size_to_content(mut self, auto_size_to_content: bool) -> Self {
        self.auto_size_to_content = auto_size_to_content;
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

    fn accessibility(&self) -> AccessibilityMeta {
        let label = self
            .accessibility_label
            .clone()
            .or_else(|| (!self.title.is_empty()).then(|| self.title.clone()))
            .unwrap_or_else(|| self.id.clone());
        let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Window).label(label);
        if let Some(hint) = self.accessibility_hint.clone() {
            accessibility = accessibility.hint(hint);
        }
        accessibility
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingWindowDefaults {
    pub position: UiPoint,
    pub size: UiSize,
    pub min_size: UiSize,
}

impl FloatingWindowDefaults {
    pub const fn new(position: UiPoint, size: UiSize, min_size: UiSize) -> Self {
        Self {
            position,
            size,
            min_size,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingWindowOrganizeSpec<'a> {
    pub id: &'a str,
    pub defaults: FloatingWindowDefaults,
    pub collapsed_size: Option<UiSize>,
}

impl<'a> FloatingWindowOrganizeSpec<'a> {
    pub const fn new(id: &'a str, defaults: FloatingWindowDefaults) -> Self {
        Self {
            id,
            defaults,
            collapsed_size: None,
        }
    }

    pub const fn with_collapsed_size(mut self, size: UiSize) -> Self {
        self.collapsed_size = Some(size);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatingWindowOrganizeMode {
    /// Every window fit at its current or preferred size.
    Preferred,
    /// At least one window was reduced to its minimum size so the layout could fit.
    Minimum,
    /// Windows were collapsed to their title bars so the layout could fit.
    Collapsed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatingWindowOrganizeOutcome {
    /// The windows were repositioned inside the requested bounds.
    Organized { mode: FloatingWindowOrganizeMode },
    /// The windows cannot fit inside the requested bounds even after the compact
    /// collapsed-title fallback, so the existing desktop state was left unchanged.
    NoFit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingDesktopZPolicy {
    pub base_z_index: i16,
    pub window_z_stride: i16,
    pub max_z_index: i16,
}

impl FloatingDesktopZPolicy {
    pub const fn new(base_z_index: i16, window_z_stride: i16, max_z_index: i16) -> Self {
        Self {
            base_z_index,
            window_z_stride,
            max_z_index,
        }
    }
}

impl Default for FloatingDesktopZPolicy {
    fn default() -> Self {
        Self::new(1, 32, i16::MAX - 32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingWindowDragState {
    pub origin: UiPoint,
    pub pointer_offset: UiPoint,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingWindowResizeState {
    pub origin_size: UiSize,
    pub origin_pointer: UiPoint,
    pub was_user_sized: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FloatingDesktopState {
    pub positions: HashMap<String, UiPoint>,
    pub sizes: HashMap<String, UiSize>,
    pub user_sized: HashSet<String>,
    pub drag: HashMap<String, FloatingWindowDragState>,
    pub resize: HashMap<String, FloatingWindowResizeState>,
    pub collapsed: HashSet<String>,
    pub z_order: HashMap<String, i16>,
    pub next_z_index: i16,
    pub z_policy: FloatingDesktopZPolicy,
}

impl FloatingDesktopState {
    pub fn new(z_policy: FloatingDesktopZPolicy) -> Self {
        Self {
            positions: HashMap::new(),
            sizes: HashMap::new(),
            user_sized: HashSet::new(),
            drag: HashMap::new(),
            resize: HashMap::new(),
            collapsed: HashSet::new(),
            z_order: HashMap::new(),
            next_z_index: z_policy.base_z_index,
            z_policy,
        }
    }

    pub fn with_visible_order(
        visible_ids: impl IntoIterator<Item = impl Into<String>>,
        z_policy: FloatingDesktopZPolicy,
    ) -> Self {
        let mut state = Self::new(z_policy);
        let stride = z_policy.window_z_stride.max(1);
        let max_before_stride = z_policy.max_z_index.saturating_sub(stride);
        for id in visible_ids {
            state.z_order.insert(id.into(), state.next_z_index);
            state.next_z_index = state
                .next_z_index
                .saturating_add(stride)
                .min(max_before_stride);
        }
        state
    }

    pub fn ensure_window(&mut self, id: &str, defaults: FloatingWindowDefaults) {
        self.positions
            .entry(id.to_string())
            .or_insert(defaults.position);
    }

    pub fn position(&self, id: &str, fallback: UiPoint) -> UiPoint {
        self.positions.get(id).copied().unwrap_or(fallback)
    }

    pub fn size(&self, id: &str, fallback: UiSize) -> UiSize {
        self.sizes.get(id).copied().unwrap_or(fallback)
    }

    pub fn z_index(&self, id: &str) -> Option<i16> {
        self.z_order.get(id).copied()
    }

    pub fn is_collapsed(&self, id: &str) -> bool {
        self.collapsed.contains(id)
    }

    pub fn toggle_collapsed(&mut self, id: &str) {
        if !self.collapsed.insert(id.to_string()) {
            self.collapsed.remove(id);
        }
        self.bring_to_front(id);
    }

    pub fn close(&mut self, id: &str) {
        self.drag.remove(id);
        self.resize.remove(id);
        self.collapsed.remove(id);
    }

    pub fn bring_to_front(&mut self, id: &str) {
        let stride = self.z_policy.window_z_stride.max(1);
        if self.next_z_index > self.z_policy.max_z_index.saturating_sub(stride) {
            self.compact_z_order();
        }
        let current_max = self
            .z_order
            .values()
            .copied()
            .max()
            .unwrap_or(self.next_z_index)
            .max(self.next_z_index);
        if current_max > self.z_policy.max_z_index.saturating_sub(stride) {
            self.compact_z_order();
        }
        let current_max = self
            .z_order
            .values()
            .copied()
            .max()
            .unwrap_or(self.next_z_index)
            .max(self.next_z_index);
        self.next_z_index = current_max
            .saturating_add(stride)
            .min(self.z_policy.max_z_index);
        self.z_order.insert(id.to_string(), self.next_z_index);
    }

    pub fn compact_z_order(&mut self) {
        let mut entries = self
            .z_order
            .iter()
            .map(|(id, z)| (id.clone(), *z))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
        self.z_order.clear();
        let stride = self.z_policy.window_z_stride.max(1);
        for (index, (id, _)) in entries.into_iter().enumerate() {
            let z = self
                .z_policy
                .base_z_index
                .saturating_add((index as i16).saturating_mul(stride))
                .min(self.z_policy.max_z_index.saturating_sub(stride));
            self.z_order.insert(id, z);
        }
        self.next_z_index = self
            .z_order
            .values()
            .copied()
            .max()
            .unwrap_or(self.z_policy.base_z_index);
    }

    pub fn organize_windows<'a>(
        &mut self,
        windows: impl IntoIterator<Item = (&'a str, FloatingWindowDefaults)>,
        bounds: UiSize,
        options: &FloatingDesktopOptions,
    ) -> FloatingWindowOrganizeOutcome {
        let bounds_rect = UiRect::new(
            0.0,
            0.0,
            finite_or(bounds.width, 0.0).max(0.0),
            finite_or(bounds.height, 0.0).max(0.0),
        );
        self.organize_windows_in_rect(windows, bounds_rect, options)
    }

    pub fn organize_windows_in_rect<'a>(
        &mut self,
        windows: impl IntoIterator<Item = (&'a str, FloatingWindowDefaults)>,
        bounds_rect: UiRect,
        options: &FloatingDesktopOptions,
    ) -> FloatingWindowOrganizeOutcome {
        self.organize_window_specs_in_rect(
            windows
                .into_iter()
                .map(|(id, defaults)| FloatingWindowOrganizeSpec::new(id, defaults)),
            bounds_rect,
            options,
        )
    }

    pub fn organize_window_specs_in_rect<'a>(
        &mut self,
        windows: impl IntoIterator<Item = FloatingWindowOrganizeSpec<'a>>,
        bounds_rect: UiRect,
        options: &FloatingDesktopOptions,
    ) -> FloatingWindowOrganizeOutcome {
        let margin = finite_or(options.margin, 0.0).max(0.0);
        let gap = finite_or(options.gap, 0.0).max(0.0);
        let bounds_rect = UiRect::new(
            finite_or(bounds_rect.x, 0.0),
            finite_or(bounds_rect.y, 0.0),
            finite_or(bounds_rect.width, 0.0).max(0.0),
            finite_or(bounds_rect.height, 0.0).max(0.0),
        );
        let inner_bounds = layout::inset_rect(bounds_rect, margin);
        let stride = self.z_policy.window_z_stride.max(1);
        let max_before_stride = self.z_policy.max_z_index.saturating_sub(stride);
        let mut last_z = self.z_policy.base_z_index;
        let mut items = Vec::new();

        for spec in windows.into_iter() {
            let id = spec.id;
            let defaults = spec.defaults;
            let mut size = self.size(id, defaults.size);
            let mut min_size = defaults.min_size;
            let collapsed_size = normalize_organize_collapsed_size(
                spec.collapsed_size.unwrap_or_else(|| {
                    UiSize::new(defaults.min_size.width, options.title_bar_height)
                }),
                options.title_bar_height,
            );
            if self.is_collapsed(id) {
                size = collapsed_size;
                min_size = collapsed_size;
            }
            items.push(OrganizeWindowItem {
                id: id.to_string(),
                size,
                min_size,
                collapsed_size,
                collapsed: self.is_collapsed(id),
            });
        }

        let Some((rects, mode)) =
            organize_window_rects(&items, inner_bounds, gap, options.title_bar_height)
        else {
            return FloatingWindowOrganizeOutcome::NoFit;
        };
        for (index, (item, rect)) in items.iter().zip(rects.into_iter()).enumerate() {
            self.ensure_window(
                &item.id,
                FloatingWindowDefaults::new(
                    UiPoint::new(rect.x, rect.y),
                    UiSize::new(rect.width, rect.height),
                    item.min_size,
                ),
            );
            self.drag.remove(&item.id);
            self.resize.remove(&item.id);
            self.positions
                .insert(item.id.clone(), UiPoint::new(rect.x, rect.y));
            self.sizes
                .insert(item.id.clone(), UiSize::new(rect.width, rect.height));
            self.user_sized.insert(item.id.clone());
            let z = self
                .z_policy
                .base_z_index
                .saturating_add((index as i16).saturating_mul(stride))
                .min(max_before_stride);
            self.z_order.insert(item.id.clone(), z);
            last_z = z;
            if mode == FloatingWindowOrganizeMode::Collapsed {
                self.collapsed.insert(item.id.clone());
            }
        }
        self.next_z_index = last_z;
        FloatingWindowOrganizeOutcome::Organized { mode }
    }

    pub fn apply_drag(&mut self, id: &str, edit: WidgetPointerEdit, fallback_position: UiPoint) {
        let origin = self.position(id, UiPoint::new(edit.target_rect.x, edit.target_rect.y));
        let origin = if self.positions.contains_key(id) {
            origin
        } else {
            fallback_position
        };
        match edit.phase.edit_phase() {
            EditPhase::Preview => {}
            EditPhase::BeginEdit => {
                self.bring_to_front(id);
                self.drag.insert(
                    id.to_string(),
                    FloatingWindowDragState {
                        origin,
                        pointer_offset: edit.local_position,
                    },
                );
                self.positions.insert(id.to_string(), origin);
            }
            EditPhase::UpdateEdit | EditPhase::CommitEdit => {
                let drag = self
                    .drag
                    .get(id)
                    .copied()
                    .unwrap_or(FloatingWindowDragState {
                        origin,
                        pointer_offset: edit.local_position,
                    });
                let position = UiPoint::new(
                    edit.position.x - drag.pointer_offset.x,
                    edit.position.y - drag.pointer_offset.y,
                );
                self.positions.insert(id.to_string(), position);
                if edit.phase.edit_phase() == EditPhase::CommitEdit {
                    self.drag.remove(id);
                }
            }
            EditPhase::CancelEdit => {
                if let Some(drag) = self.drag.remove(id) {
                    self.positions.insert(id.to_string(), drag.origin);
                }
            }
        }
    }

    pub fn apply_resize(
        &mut self,
        id: &str,
        edit: WidgetPointerEdit,
        defaults: FloatingWindowDefaults,
    ) {
        let stored_size = self.size(id, defaults.size);
        let target_width = finite_or(edit.target_rect.width, 0.0);
        let target_height = finite_or(edit.target_rect.height, 0.0);
        let rendered_size = UiSize::new(
            if target_width > 0.0 {
                target_width
            } else {
                stored_size.width
            },
            if target_height > 0.0 {
                target_height
            } else {
                stored_size.height
            },
        );
        let origin_size = UiSize::new(
            rendered_size.width.max(defaults.min_size.width).max(1.0),
            rendered_size.height.max(defaults.min_size.height).max(1.0),
        );
        match edit.phase.edit_phase() {
            EditPhase::Preview => {}
            EditPhase::BeginEdit => {
                self.bring_to_front(id);
                self.resize.insert(
                    id.to_string(),
                    FloatingWindowResizeState {
                        origin_size,
                        origin_pointer: edit.position,
                        was_user_sized: self.user_sized.contains(id),
                    },
                );
                self.user_sized.insert(id.to_string());
                self.sizes.insert(id.to_string(), origin_size);
            }
            EditPhase::UpdateEdit | EditPhase::CommitEdit => {
                let resize = self
                    .resize
                    .get(id)
                    .copied()
                    .unwrap_or(FloatingWindowResizeState {
                        origin_size,
                        origin_pointer: edit.position,
                        was_user_sized: self.user_sized.contains(id),
                    });
                self.user_sized.insert(id.to_string());
                let size = UiSize::new(
                    (resize.origin_size.width + edit.position.x - resize.origin_pointer.x)
                        .max(defaults.min_size.width),
                    (resize.origin_size.height + edit.position.y - resize.origin_pointer.y)
                        .max(defaults.min_size.height),
                );
                self.sizes.insert(id.to_string(), size);
                if edit.phase.edit_phase() == EditPhase::CommitEdit {
                    self.resize.remove(id);
                }
            }
            EditPhase::CancelEdit => {
                if let Some(resize) = self.resize.remove(id) {
                    if !resize.was_user_sized {
                        self.user_sized.remove(id);
                    }
                    self.sizes.insert(id.to_string(), resize.origin_size);
                }
            }
        }
    }

    pub fn apply_to_descriptor(
        &self,
        descriptor: &mut FloatingWindowDescriptor,
        defaults: FloatingWindowDefaults,
    ) {
        descriptor.position = Some(self.position(&descriptor.id, defaults.position));
        descriptor.preferred_size = self.size(&descriptor.id, defaults.size);
        if self.user_sized.contains(&descriptor.id) || self.sizes.contains_key(&descriptor.id) {
            descriptor.auto_size_to_content = false;
        }
        descriptor.collapsed = self.is_collapsed(&descriptor.id);
        if let Some(z_index) = self.z_index(&descriptor.id) {
            descriptor.z_index = Some(z_index);
        }
    }
}

impl Default for FloatingDesktopState {
    fn default() -> Self {
        Self::new(FloatingDesktopZPolicy::default())
    }
}

#[derive(Debug, Clone)]
pub struct FloatingDesktopOptions {
    pub layout: LayoutStyle,
    pub bounds: UiSize,
    pub bounds_rect: UiRect,
    pub margin: f32,
    pub gap: f32,
    pub cascade_offset: f32,
    pub base_z_index: i16,
    pub window_z_stride: i16,
    pub window_visual: UiVisual,
    pub title_bar_visual: UiVisual,
    pub content_visual: UiVisual,
    pub title_style: TextStyle,
    pub title_bar_height: f32,
    pub content_padding: f32,
    pub content_gap: f32,
    pub close_button_size: f32,
    pub resize_handle_size: f32,
    pub close_button_visual: UiVisual,
    pub close_button_hovered_visual: UiVisual,
    pub close_button_pressed_visual: UiVisual,
    pub close_button_text_style: TextStyle,
    pub resize_handle_visual: UiVisual,
}

impl FloatingDesktopOptions {
    pub fn new(bounds: UiSize) -> Self {
        Self {
            bounds,
            bounds_rect: UiRect::new(0.0, 0.0, bounds.width, bounds.height),
            ..Default::default()
        }
    }

    pub fn with_bounds_rect(mut self, bounds_rect: UiRect) -> Self {
        self.bounds_rect = UiRect::new(
            finite_or(bounds_rect.x, 0.0),
            finite_or(bounds_rect.y, 0.0),
            finite_or(bounds_rect.width, 0.0).max(0.0),
            finite_or(bounds_rect.height, 0.0).max(0.0),
        );
        self.bounds = UiSize::new(self.bounds_rect.width, self.bounds_rect.height);
        self
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_margin(mut self, margin: f32) -> Self {
        self.margin = margin;
        self
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_cascade_offset(mut self, offset: f32) -> Self {
        self.cascade_offset = offset;
        self
    }

    pub fn with_content_padding(mut self, padding: f32) -> Self {
        self.content_padding = padding;
        self
    }

    pub fn with_content_gap(mut self, gap: f32) -> Self {
        self.content_gap = gap;
        self
    }
}

impl Default for FloatingDesktopOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height_percent(1.0),
            bounds: UiSize::new(800.0, 600.0),
            bounds_rect: UiRect::new(0.0, 0.0, 800.0, 600.0),
            margin: 18.0,
            gap: 14.0,
            cascade_offset: 28.0,
            base_z_index: 1,
            window_z_stride: 32,
            window_visual: UiVisual::panel(
                DEFAULT_SURFACE_BG,
                Some(StrokeStyle::new(DEFAULT_SURFACE_STROKE, 1.0)),
                0.0,
            ),
            title_bar_visual: UiVisual::panel(
                ColorRgba::new(21, 26, 33, 255),
                Some(StrokeStyle::new(DEFAULT_SURFACE_STROKE, 1.0)),
                0.0,
            ),
            content_visual: UiVisual::TRANSPARENT,
            title_style: TextStyle {
                font_size: 13.0,
                line_height: 18.0,
                color: ColorRgba::new(186, 198, 216, 255),
                ..Default::default()
            },
            title_bar_height: 32.0,
            content_padding: 10.0,
            content_gap: 8.0,
            close_button_size: 22.0,
            resize_handle_size: 16.0,
            close_button_visual: UiVisual::panel(ColorRgba::TRANSPARENT, None, 3.0),
            close_button_hovered_visual: UiVisual::panel(
                ColorRgba::new(48, 58, 72, 255),
                None,
                3.0,
            ),
            close_button_pressed_visual: UiVisual::panel(
                ColorRgba::new(35, 42, 52, 255),
                None,
                3.0,
            ),
            close_button_text_style: TextStyle {
                font_size: 14.0,
                line_height: 16.0,
                color: ColorRgba::new(178, 190, 206, 255),
                ..Default::default()
            },
            resize_handle_visual: UiVisual::panel(ColorRgba::TRANSPARENT, None, 0.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FloatingWindowPlacement {
    pub source_index: usize,
    pub id: String,
    pub rect: UiRect,
    pub z_index: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloatingWindowNode {
    pub id: String,
    pub root: UiNodeId,
    pub title_bar: UiNodeId,
    pub collapse_button: Option<UiNodeId>,
    pub title: Option<UiNodeId>,
    pub close_button: Option<UiNodeId>,
    pub content: UiNodeId,
    pub resize_handle: Option<UiNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloatingDesktopNodes {
    pub root: UiNodeId,
    pub windows: Vec<FloatingWindowNode>,
}

#[derive(Debug, Clone)]
struct OrganizeWindowItem {
    id: String,
    size: UiSize,
    min_size: UiSize,
    collapsed_size: UiSize,
    collapsed: bool,
}

pub fn floating_window_layout(
    windows: &[FloatingWindowDescriptor],
    bounds: UiSize,
    options: &FloatingDesktopOptions,
) -> Vec<FloatingWindowPlacement> {
    let margin = finite_or(options.margin, 0.0).max(0.0);
    let gap = finite_or(options.gap, 0.0).max(0.0);
    let cascade_offset = finite_or(options.cascade_offset, 0.0).max(0.0);
    let mut placements = Vec::new();
    let default_bounds_rect = UiRect::new(
        0.0,
        0.0,
        finite_or(bounds.width, 0.0).max(0.0),
        finite_or(bounds.height, 0.0).max(0.0),
    );
    let bounds_rect = if options.bounds_rect.width > 0.0 || options.bounds_rect.height > 0.0 {
        options.bounds_rect
    } else {
        default_bounds_rect
    };
    let usable_bounds = UiSize::new(bounds_rect.width, bounds_rect.height);
    let inner_bounds = layout::inset_rect(bounds_rect, margin);
    let mut flow = layout::ContainedFlowLayout::new(inner_bounds)
        .with_gap(gap)
        .with_cascade_offset(cascade_offset);

    for (source_index, window) in windows
        .iter()
        .enumerate()
        .filter(|(_, window)| window.visible)
    {
        let (size, min_size) = if window.collapsed {
            let collapsed_size = resolved_collapsed_min_size(window, usable_bounds, options);
            (collapsed_size, collapsed_size)
        } else {
            (
                resolved_size(window, usable_bounds, options),
                resolved_min_size(window, usable_bounds, options),
            )
        };
        let z_index = window.z_index.unwrap_or_else(|| {
            options.base_z_index.saturating_add(
                (placements.len().min(i16::MAX as usize) as i16)
                    .saturating_mul(options.window_z_stride.max(1)),
            )
        });
        let rect = if let Some(position) = window.position {
            layout::contain_rect_from_origin(
                UiRect::new(position.x, position.y, size.width, size.height),
                inner_bounds,
                min_size,
            )
        } else {
            flow.next_rect(size, min_size)
        };

        placements.push(FloatingWindowPlacement {
            source_index,
            id: window.id.clone(),
            rect,
            z_index,
        });
    }

    placements
}

fn organize_window_rects(
    items: &[OrganizeWindowItem],
    bounds: UiRect,
    gap: f32,
    title_bar_height: f32,
) -> Option<(Vec<UiRect>, FloatingWindowOrganizeMode)> {
    let (preferred, preferred_fits) = organize_window_rects_for_mode(
        items,
        bounds,
        gap,
        title_bar_height,
        FloatingWindowOrganizeMode::Preferred,
    );
    if preferred_fits {
        return Some((preferred, FloatingWindowOrganizeMode::Preferred));
    }

    let (minimum, minimum_fits) = organize_window_rects_for_mode(
        items,
        bounds,
        gap,
        title_bar_height,
        FloatingWindowOrganizeMode::Minimum,
    );
    if minimum_fits {
        return Some((minimum, FloatingWindowOrganizeMode::Minimum));
    }

    let (collapsed, collapsed_fits) = organize_window_rects_for_mode(
        items,
        bounds,
        gap,
        title_bar_height,
        FloatingWindowOrganizeMode::Collapsed,
    );
    if collapsed_fits {
        return Some((collapsed, FloatingWindowOrganizeMode::Collapsed));
    }

    None
}

fn organize_window_rects_for_mode(
    items: &[OrganizeWindowItem],
    bounds: UiRect,
    gap: f32,
    title_bar_height: f32,
    mode: FloatingWindowOrganizeMode,
) -> (Vec<UiRect>, bool) {
    pack_organize_window_rects(items, bounds, gap, title_bar_height, mode)
        .map_or_else(|| (Vec::new(), false), |rects| (rects, true))
}

fn pack_organize_window_rects(
    items: &[OrganizeWindowItem],
    bounds: UiRect,
    gap: f32,
    title_bar_height: f32,
    mode: FloatingWindowOrganizeMode,
) -> Option<Vec<UiRect>> {
    let bin_width = organize_pack_bound(bounds.width)?;
    let bin_height = organize_pack_bound(bounds.height)?;
    let pack_gap = finite_or(gap, 0.0).max(0.0);
    let sizes = items
        .iter()
        .map(|item| organize_window_item_size(item, title_bar_height, mode))
        .collect::<Vec<_>>();
    let dimensions = sizes
        .iter()
        .enumerate()
        .map(|(index, size)| {
            Some(PackDimension::with_id(
                index as isize,
                organize_pack_extent(size.width + pack_gap)?,
                organize_pack_extent(size.height + pack_gap)?,
                0,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    let mut bin = MaxRectsBin::with_capacity(bin_width, bin_height, dimensions.len());
    let (packed, rejected) = bin.insert_list(&dimensions, MaxRectsHeuristic::BestShortSideFit);
    if !rejected.is_empty() || packed.len() != dimensions.len() {
        return None;
    }

    let mut rects = vec![UiRect::new(0.0, 0.0, 0.0, 0.0); items.len()];
    for packed_rect in packed {
        let index = usize::try_from(packed_rect.id()).ok()?;
        let size = *sizes.get(index)?;
        let rect = UiRect::new(
            bounds.x + packed_rect.x() as f32 / ORGANIZE_PACK_SCALE,
            bounds.y + packed_rect.y() as f32 / ORGANIZE_PACK_SCALE,
            size.width,
            size.height,
        );
        if rect.right() > bounds.right() + f32::EPSILON
            || rect.bottom() > bounds.bottom() + f32::EPSILON
        {
            return None;
        }
        rects[index] = rect;
    }
    Some(rects)
}

fn organize_window_item_size(
    item: &OrganizeWindowItem,
    title_bar_height: f32,
    mode: FloatingWindowOrganizeMode,
) -> UiSize {
    if item.collapsed || mode == FloatingWindowOrganizeMode::Collapsed {
        return normalize_organize_collapsed_size(item.collapsed_size, title_bar_height);
    }
    let min_width = finite_or(item.min_size.width, 1.0).max(1.0);
    let min_height = finite_or(item.min_size.height, 1.0).max(1.0);
    let (width, height) = match mode {
        FloatingWindowOrganizeMode::Preferred => (
            finite_or(item.size.width, min_width).max(min_width),
            finite_or(item.size.height, min_height).max(min_height),
        ),
        FloatingWindowOrganizeMode::Minimum => (min_width, min_height),
        FloatingWindowOrganizeMode::Collapsed => unreachable!("collapsed mode returns early"),
    };
    UiSize::new(width, height)
}

fn normalize_organize_collapsed_size(size: UiSize, title_bar_height: f32) -> UiSize {
    UiSize::new(
        finite_or(size.width, 1.0).max(1.0),
        finite_or(size.height, title_bar_height)
            .max(finite_or(title_bar_height, 32.0))
            .max(1.0),
    )
}

fn organize_pack_extent(value: f32) -> Option<i32> {
    let scaled = (finite_or(value, 1.0).max(0.0) * ORGANIZE_PACK_SCALE)
        .ceil()
        .max(1.0);
    (scaled <= i32::MAX as f32).then_some(scaled as i32)
}

fn organize_pack_bound(value: f32) -> Option<i32> {
    let scaled = (finite_or(value, 0.0).max(0.0) * ORGANIZE_PACK_SCALE).floor();
    (scaled >= 1.0 && scaled <= i32::MAX as f32).then_some(scaled as i32)
}

pub fn floating_desktop(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    windows: &[FloatingWindowDescriptor],
    options: FloatingDesktopOptions,
    mut build_window: impl FnMut(&mut UiDocument, UiNodeId, &FloatingWindowDescriptor),
) -> FloatingDesktopNodes {
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
                .label(format!("{name} floating workspace"))
                .hint("Contains floating windows"),
        ),
    );
    let placements = floating_window_layout(windows, options.bounds, &options);
    let mut nodes = Vec::new();
    for placement in placements {
        let descriptor = &windows[placement.source_index];
        let node = add_floating_window(document, root, &name, descriptor, &placement, &options);
        if !descriptor.collapsed {
            build_window(document, node.content, descriptor);
        }
        normalize_window_subtree_z(
            document,
            node.root,
            placement.z_index,
            options.window_z_stride,
        );
        nodes.push(node);
    }

    FloatingDesktopNodes {
        root,
        windows: nodes,
    }
}

fn add_floating_window(
    document: &mut UiDocument,
    parent: UiNodeId,
    desktop_name: &str,
    descriptor: &FloatingWindowDescriptor,
    placement: &FloatingWindowPlacement,
    options: &FloatingDesktopOptions,
) -> FloatingWindowNode {
    let mut root_layout = LayoutStyle::absolute_rect(placement.rect).style;
    root_layout.display = Display::Flex;
    root_layout.flex_direction = FlexDirection::Column;
    let mut root_node = UiNode::container(
        format!("{desktop_name}.window.{}", descriptor.id),
        UiNodeStyle {
            layout: root_layout,
            clip: ClipBehavior::Clip,
            z_index: placement.z_index,
            ..Default::default()
        },
    )
    .with_input(InputBehavior {
        pointer: true,
        focusable: false,
        keyboard: false,
    })
    .with_visual(options.window_visual)
    .with_accessibility(descriptor.accessibility());
    if let Some(action) = descriptor.activate_action.clone() {
        root_node = root_node.with_action(action);
    }
    let root = document.add_child(parent, root_node);

    let mut title_bar_node = UiNode::container(
        format!("{desktop_name}.window.{}.title_bar", descriptor.id),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(options.title_bar_height.max(0.0)),
                },
                padding: Rect {
                    left: LengthPercentage::length(options.content_padding.max(0.0)),
                    right: LengthPercentage::length(options.content_padding.max(0.0)),
                    top: LengthPercentage::length(0.0),
                    bottom: LengthPercentage::length(0.0),
                },
                flex_shrink: 0.0,
                ..Default::default()
            })
            .style,
            ..Default::default()
        },
    )
    .with_visual(options.title_bar_visual);
    if let Some(action) = descriptor.drag_action.clone() {
        title_bar_node = title_bar_node
            .with_input(InputBehavior::BUTTON)
            .with_action(action)
            .with_action_mode(WidgetActionMode::PointerEdit)
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label(format!("Move {}", descriptor.title))
                    .focusable(),
            );
    } else if let Some(action) = descriptor.title_action.clone() {
        title_bar_node = title_bar_node
            .with_input(InputBehavior::BUTTON)
            .with_action(action)
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label(format!("Activate {}", descriptor.title))
                    .focusable(),
            );
    }
    let title_bar = document.add_child(root, title_bar_node);

    let collapse_button = descriptor.collapse_action.as_ref().map(|action| {
        add_collapse_button(
            document,
            title_bar,
            desktop_name,
            descriptor,
            action,
            options,
        )
    });

    let title = (!descriptor.title.is_empty()).then(|| {
        document.add_child(
            title_bar,
            UiNode::text(
                format!("{desktop_name}.window.{}.title", descriptor.id),
                descriptor.title.clone(),
                options.title_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    flex_grow: 1.0,
                    flex_shrink: 0.0,
                    ..Default::default()
                }),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Label).label(descriptor.title.clone()),
            ),
        )
    });

    let close_button = descriptor.close_action.as_ref().map(|action| {
        add_close_button(
            document,
            title_bar,
            desktop_name,
            descriptor,
            action,
            options,
        )
    });

    let content_height = if descriptor.collapsed {
        length(0.0)
    } else {
        Dimension::auto()
    };
    let content_flex_grow = if descriptor.collapsed { 0.0 } else { 1.0 };
    let content = document.add_child(
        root,
        UiNode::container(
            format!("{desktop_name}.window.{}.content", descriptor.id),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_grow: content_flex_grow,
                    flex_shrink: 1.0,
                    flex_basis: length(0.0),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: content_height,
                    },
                    min_size: TaffySize {
                        width: length(0.0),
                        height: length(0.0),
                    },
                    padding: Rect {
                        left: LengthPercentage::length(options.content_padding.max(0.0)),
                        right: LengthPercentage::length(options.content_padding.max(0.0)),
                        top: LengthPercentage::length(options.content_padding.max(0.0)),
                        bottom: LengthPercentage::length(options.content_padding.max(0.0)),
                    },
                    gap: TaffySize {
                        width: LengthPercentage::length(options.content_gap.max(0.0)),
                        height: LengthPercentage::length(options.content_gap.max(0.0)),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.content_visual),
    );

    let resize_handle = (!descriptor.collapsed)
        .then(|| {
            descriptor.resize_action.as_ref().map(|action| {
                add_resize_handle(document, root, desktop_name, descriptor, action, options)
            })
        })
        .flatten();

    let mut constraint_sources = vec![title_bar];
    if !descriptor.collapsed {
        constraint_sources.push(content);
    }
    let bounds = layout::inset_rect(options.bounds_rect, options.margin);
    let constraint_min_size = if descriptor.collapsed {
        resolved_collapsed_min_size(descriptor, options.bounds, options)
    } else {
        resolved_min_size(descriptor, options.bounds, options)
    };
    document.node_mut(root).layout_constraint =
        Some(UiNodeLayoutConstraint::StackedIntrinsicSize {
            sources: constraint_sources,
            min_size: constraint_min_size,
            bounds,
            fit_to_preferred: descriptor.auto_size_to_content,
        });

    FloatingWindowNode {
        id: descriptor.id.clone(),
        root,
        title_bar,
        collapse_button,
        title,
        close_button,
        content,
        resize_handle,
    }
}

fn add_collapse_button(
    document: &mut UiDocument,
    title_bar: UiNodeId,
    desktop_name: &str,
    descriptor: &FloatingWindowDescriptor,
    action: &WidgetActionBinding,
    options: &FloatingDesktopOptions,
) -> UiNodeId {
    let size = options.close_button_size.max(1.0);
    let button = document.add_child(
        title_bar,
        UiNode::container(
            format!("{desktop_name}.window.{}.collapse", descriptor.id),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                flex_shrink: 0.0,
                size: TaffySize {
                    width: length(size),
                    height: length(size),
                },
                margin: Rect {
                    left: taffy::prelude::LengthPercentageAuto::length(0.0),
                    right: taffy::prelude::LengthPercentageAuto::length(8.0),
                    top: taffy::prelude::LengthPercentageAuto::length(0.0),
                    bottom: taffy::prelude::LengthPercentageAuto::length(0.0),
                },
                ..Default::default()
            }),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action(action.clone())
        .with_interaction_visuals(
            InteractionVisuals::new(options.close_button_visual)
                .hovered(options.close_button_hovered_visual)
                .pressed(options.close_button_pressed_visual),
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Button)
                .label(if descriptor.collapsed {
                    format!("Expand {}", descriptor.title)
                } else {
                    format!("Collapse {}", descriptor.title)
                })
                .focusable(),
        ),
    );
    document.add_child(
        button,
        UiNode::text(
            format!("{desktop_name}.window.{}.collapse.label", descriptor.id),
            if descriptor.collapsed { ">" } else { "v" },
            options.close_button_text_style.clone(),
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        )
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label(
            if descriptor.collapsed {
                "Expand"
            } else {
                "Collapse"
            },
        )),
    );
    button
}

fn add_close_button(
    document: &mut UiDocument,
    title_bar: UiNodeId,
    desktop_name: &str,
    descriptor: &FloatingWindowDescriptor,
    action: &WidgetActionBinding,
    options: &FloatingDesktopOptions,
) -> UiNodeId {
    let size = options.close_button_size.max(1.0);
    let close = document.add_child(
        title_bar,
        UiNode::container(
            format!("{desktop_name}.window.{}.close", descriptor.id),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                flex_shrink: 0.0,
                size: TaffySize {
                    width: length(size),
                    height: length(size),
                },
                margin: Rect {
                    left: taffy::prelude::LengthPercentageAuto::length(8.0),
                    right: taffy::prelude::LengthPercentageAuto::length(0.0),
                    top: taffy::prelude::LengthPercentageAuto::length(0.0),
                    bottom: taffy::prelude::LengthPercentageAuto::length(0.0),
                },
                ..Default::default()
            }),
        )
        .with_input(InputBehavior::BUTTON)
        .with_action(action.clone())
        .with_interaction_visuals(
            InteractionVisuals::new(options.close_button_visual)
                .hovered(options.close_button_hovered_visual)
                .pressed(options.close_button_pressed_visual),
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Button)
                .label(format!("Close {}", descriptor.title))
                .focusable(),
        ),
    );
    document.add_child(
        close,
        UiNode::text(
            format!("{desktop_name}.window.{}.close.label", descriptor.id),
            "x",
            options.close_button_text_style.clone(),
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        )
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label("Close")),
    );
    close
}

fn add_resize_handle(
    document: &mut UiDocument,
    root: UiNodeId,
    desktop_name: &str,
    descriptor: &FloatingWindowDescriptor,
    action: &WidgetActionBinding,
    options: &FloatingDesktopOptions,
) -> UiNodeId {
    let size = options.resize_handle_size.max(8.0);
    let handle = document.add_child(
        root,
        UiNode::container(
            format!("{desktop_name}.window.{}.resize", descriptor.id),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    position: taffy::prelude::Position::Absolute,
                    inset: Rect {
                        left: taffy::prelude::LengthPercentageAuto::auto(),
                        right: taffy::prelude::LengthPercentageAuto::length(4.0),
                        top: taffy::prelude::LengthPercentageAuto::auto(),
                        bottom: taffy::prelude::LengthPercentageAuto::length(4.0),
                    },
                    size: TaffySize {
                        width: length(size),
                        height: length(size),
                    },
                    ..Default::default()
                })
                .style,
                z_index: 2,
                ..Default::default()
            },
        )
        .with_input(InputBehavior::BUTTON)
        .with_action(action.clone())
        .with_action_mode(WidgetActionMode::PointerEditParentRect)
        .with_visual(options.resize_handle_visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Button)
                .label(format!("Resize {}", descriptor.title))
                .focusable(),
        ),
    );
    let grip_color = ColorRgba::new(120, 134, 156, 210);
    document.add_child(
        handle,
        UiNode::scene(
            format!("{desktop_name}.window.{}.resize.grip", descriptor.id),
            vec![
                ScenePrimitive::Line {
                    from: UiPoint::new(size - 5.0, size - 13.0),
                    to: UiPoint::new(size - 13.0, size - 5.0),
                    stroke: StrokeStyle::new(grip_color, 1.0),
                },
                ScenePrimitive::Line {
                    from: UiPoint::new(size - 4.0, size - 9.0),
                    to: UiPoint::new(size - 9.0, size - 4.0),
                    stroke: StrokeStyle::new(grip_color, 1.0),
                },
                ScenePrimitive::Line {
                    from: UiPoint::new(size - 3.0, size - 5.0),
                    to: UiPoint::new(size - 5.0, size - 3.0),
                    stroke: StrokeStyle::new(grip_color, 1.0),
                },
            ],
            LayoutStyle::size(size, size),
        )
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Image).label("Resize grip")),
    );
    handle
}

fn normalize_window_subtree_z(
    document: &mut UiDocument,
    root: UiNodeId,
    window_z: i16,
    window_z_stride: i16,
) {
    let band_max = window_z.saturating_add(window_z_stride.max(1).saturating_sub(1));
    document.node_mut(root).style.z_index = window_z;
    let children = document.node(root).children.clone();
    for child in children {
        normalize_window_child_z(document, child, window_z, window_z, band_max);
    }
}

fn normalize_window_child_z(
    document: &mut UiDocument,
    node: UiNodeId,
    window_z: i16,
    parent_z: i16,
    band_max: i16,
) {
    let local_z = document.node(node).style.z_index;
    let child_z = normalized_child_z(local_z, window_z, parent_z, band_max);
    document.node_mut(node).style.z_index = child_z;
    let children = document.node(node).children.clone();
    for child in children {
        normalize_window_child_z(document, child, window_z, child_z, band_max);
    }
}

fn normalized_child_z(local_z: i16, window_z: i16, parent_z: i16, band_max: i16) -> i16 {
    let relative_z = local_z.max(1);
    parent_z
        .saturating_add(relative_z)
        .max(window_z)
        .min(band_max)
}

fn resolved_size(
    window: &FloatingWindowDescriptor,
    bounds: UiSize,
    options: &FloatingDesktopOptions,
) -> UiSize {
    let margin = finite_or(options.margin, 0.0).max(0.0);
    let min_size = resolved_min_size(window, bounds, options);
    let available_width =
        (finite_or(bounds.width, window.preferred_size.width) - margin * 2.0).max(1.0);
    let available_height =
        (finite_or(bounds.height, window.preferred_size.height) - margin * 2.0).max(1.0);
    UiSize::new(
        finite_or(window.preferred_size.width, min_size.width)
            .clamp(min_size.width, available_width)
            .max(1.0),
        finite_or(window.preferred_size.height, min_size.height)
            .clamp(min_size.height, available_height)
            .max(1.0),
    )
}

fn resolved_min_size(
    window: &FloatingWindowDescriptor,
    bounds: UiSize,
    options: &FloatingDesktopOptions,
) -> UiSize {
    let margin = finite_or(options.margin, 0.0).max(0.0);
    let available_width =
        (finite_or(bounds.width, window.preferred_size.width) - margin * 2.0).max(1.0);
    let available_height =
        (finite_or(bounds.height, window.preferred_size.height) - margin * 2.0).max(1.0);
    let padding = finite_or(options.content_padding, 0.0).max(0.0);
    let title_bar_height = finite_or(options.title_bar_height, 0.0).max(0.0);
    let title_control_width = title_bar_control_width(window, options);
    let title_text_width = approximate_title_width(&window.title, &options.title_style);
    let title_min_width = padding * 2.0 + title_control_width + title_text_width;
    let chrome_min = window
        .content_min_size
        .map(|content| {
            UiSize::new(
                (finite_or(content.width, 1.0).max(1.0) + padding * 2.0).max(title_min_width),
                finite_or(content.height, 1.0).max(1.0) + title_bar_height + padding * 2.0,
            )
        })
        .unwrap_or_else(|| UiSize::new(title_min_width, title_bar_height));
    UiSize::new(
        finite_or(window.min_size.width, 1.0)
            .max(chrome_min.width)
            .max(1.0)
            .min(available_width),
        finite_or(window.min_size.height, 1.0)
            .max(chrome_min.height)
            .max(1.0)
            .min(available_height),
    )
}

fn resolved_collapsed_min_size(
    window: &FloatingWindowDescriptor,
    bounds: UiSize,
    options: &FloatingDesktopOptions,
) -> UiSize {
    let margin = finite_or(options.margin, 0.0).max(0.0);
    let available_width =
        (finite_or(bounds.width, window.preferred_size.width) - margin * 2.0).max(1.0);
    let padding = finite_or(options.content_padding, 0.0).max(0.0);
    let title_bar_height = finite_or(options.title_bar_height, 32.0).max(1.0);
    let title_control_width = title_bar_control_width(window, options);
    let title_text_width = approximate_title_width(&window.title, &options.title_style);
    let title_min_width = padding * 2.0 + title_control_width + title_text_width;
    UiSize::new(
        finite_or(window.min_size.width, 1.0)
            .max(title_min_width)
            .max(1.0)
            .min(available_width),
        title_bar_height,
    )
}

fn title_bar_control_width(
    window: &FloatingWindowDescriptor,
    options: &FloatingDesktopOptions,
) -> f32 {
    let button = finite_or(options.close_button_size, 0.0).max(1.0);
    let mut width = 0.0;
    if window.collapse_action.is_some() {
        width += button + 8.0;
    }
    if window.close_action.is_some() {
        width += button + 8.0;
    }
    width
}

fn approximate_title_width(title: &str, style: &TextStyle) -> f32 {
    if title.is_empty() {
        return 0.0;
    }
    let font_size = finite_or(style.font_size, 13.0).max(1.0);
    (title.chars().count() as f32 * font_size * 0.55).max(font_size)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use crate::{root_style, ApproxTextMeasurer, ScrollAxes};

    use super::*;

    #[test]
    fn floating_desktop_state_applies_window_defaults_and_order() {
        let defaults = FloatingWindowDefaults::new(
            UiPoint::new(20.0, 30.0),
            UiSize::new(240.0, 180.0),
            UiSize::new(120.0, 90.0),
        );
        let mut state = FloatingDesktopState::new(FloatingDesktopZPolicy::new(10, 5, 100));

        state.ensure_window("inspector", defaults);
        state.bring_to_front("inspector");
        state.toggle_collapsed("inspector");

        let mut descriptor =
            FloatingWindowDescriptor::new("inspector", "Inspector", UiSize::new(1.0, 1.0));
        state.apply_to_descriptor(&mut descriptor, defaults);

        assert_eq!(descriptor.position, Some(defaults.position));
        assert_eq!(descriptor.preferred_size, defaults.size);
        assert!(descriptor.collapsed);
        assert_eq!(descriptor.z_index, Some(20));
    }

    #[test]
    fn floating_desktop_state_organizes_windows_into_contained_flow() {
        let defaults_a = FloatingWindowDefaults::new(
            UiPoint::new(300.0, 200.0),
            UiSize::new(140.0, 80.0),
            UiSize::new(80.0, 50.0),
        );
        let defaults_b = FloatingWindowDefaults::new(
            UiPoint::new(280.0, 180.0),
            UiSize::new(140.0, 80.0),
            UiSize::new(80.0, 50.0),
        );
        let defaults_c = FloatingWindowDefaults::new(
            UiPoint::new(260.0, 160.0),
            UiSize::new(140.0, 80.0),
            UiSize::new(80.0, 50.0),
        );
        let options = FloatingDesktopOptions::new(UiSize::new(330.0, 220.0))
            .with_margin(10.0)
            .with_gap(8.0);
        let mut state = FloatingDesktopState::new(FloatingDesktopZPolicy::new(10, 5, 100));

        let outcome = state.organize_windows(
            [("a", defaults_a), ("b", defaults_b), ("c", defaults_c)],
            options.bounds,
            &options,
        );

        assert_eq!(
            outcome,
            FloatingWindowOrganizeOutcome::Organized {
                mode: FloatingWindowOrganizeMode::Preferred,
            }
        );
        let rects = ["a", "b", "c"]
            .into_iter()
            .map(|id| {
                let position = state.position(id, UiPoint::new(0.0, 0.0));
                let size = state.size(id, UiSize::ZERO);
                UiRect::new(position.x, position.y, size.width, size.height)
            })
            .collect::<Vec<_>>();
        for (index, rect) in rects.iter().enumerate() {
            assert!(
                rect.x >= 10.0
                    && rect.y >= 10.0
                    && rect.right() <= 320.0 + f32::EPSILON
                    && rect.bottom() <= 210.0 + f32::EPSILON,
                "{index}: {rect:?}"
            );
            for other in rects.iter().skip(index + 1) {
                assert!(!overlaps(*rect, *other), "{rect:?} overlapped {other:?}");
            }
        }
        assert_eq!(state.size("a", UiSize::ZERO), defaults_a.size);
        assert_eq!(state.size("b", UiSize::ZERO), defaults_b.size);
        assert_eq!(state.size("c", UiSize::ZERO), defaults_c.size);
        assert_eq!(state.z_index("a"), Some(10));
        assert_eq!(state.z_index("b"), Some(15));
        assert_eq!(state.z_index("c"), Some(20));
        assert!(state.user_sized.contains("a"));
    }

    #[test]
    fn floating_desktop_state_organizes_windows_with_rectangle_packing() {
        let tall = FloatingWindowDefaults::new(
            UiPoint::new(300.0, 200.0),
            UiSize::new(100.0, 160.0),
            UiSize::new(100.0, 160.0),
        );
        let wide = FloatingWindowDefaults::new(
            UiPoint::new(280.0, 180.0),
            UiSize::new(160.0, 80.0),
            UiSize::new(160.0, 80.0),
        );
        let options = FloatingDesktopOptions::new(UiSize::new(260.0, 160.0))
            .with_margin(0.0)
            .with_gap(0.0);
        let mut state = FloatingDesktopState::new(FloatingDesktopZPolicy::new(10, 5, 100));

        let outcome = state.organize_windows(
            [("tall", tall), ("wide-a", wide), ("wide-b", wide)],
            options.bounds,
            &options,
        );

        assert_eq!(
            outcome,
            FloatingWindowOrganizeOutcome::Organized {
                mode: FloatingWindowOrganizeMode::Preferred,
            }
        );
        let rects = ["tall", "wide-a", "wide-b"]
            .into_iter()
            .map(|id| {
                let size = state.size(id, UiSize::new(0.0, 0.0));
                let position = state.position(id, UiPoint::new(0.0, 0.0));
                UiRect::new(position.x, position.y, size.width, size.height)
            })
            .collect::<Vec<_>>();
        for (index, rect) in rects.iter().enumerate() {
            assert!(rect.x >= 0.0 && rect.y >= 0.0, "{index}: {rect:?}");
            assert!(rect.right() <= 260.0 + f32::EPSILON, "{index}: {rect:?}");
            assert!(rect.bottom() <= 160.0 + f32::EPSILON, "{index}: {rect:?}");
            for other in rects.iter().skip(index + 1) {
                assert!(!overlaps(*rect, *other), "{rect:?} overlapped {other:?}");
            }
        }
    }

    #[test]
    fn floating_desktop_state_reports_no_fit_without_mutating_windows() {
        let defaults_a = FloatingWindowDefaults::new(
            UiPoint::new(24.0, 30.0),
            UiSize::new(180.0, 110.0),
            UiSize::new(120.0, 70.0),
        );
        let defaults_b = FloatingWindowDefaults::new(
            UiPoint::new(48.0, 60.0),
            UiSize::new(180.0, 110.0),
            UiSize::new(120.0, 70.0),
        );
        let options = FloatingDesktopOptions::new(UiSize::new(100.0, 120.0))
            .with_margin(10.0)
            .with_gap(8.0);
        let mut state = FloatingDesktopState::new(FloatingDesktopZPolicy::new(10, 5, 100));
        state.ensure_window("a", defaults_a);
        state.ensure_window("b", defaults_b);
        state
            .sizes
            .insert("a".to_string(), UiSize::new(190.0, 120.0));
        state.user_sized.insert("a".to_string());
        state.collapsed.insert("b".to_string());
        state.z_order.insert("a".to_string(), 35);
        state.z_order.insert("b".to_string(), 20);
        state.next_z_index = 35;
        let before = state.clone();

        let outcome = state.organize_windows(
            [("a", defaults_a), ("b", defaults_b)],
            options.bounds,
            &options,
        );

        assert_eq!(outcome, FloatingWindowOrganizeOutcome::NoFit);
        assert_eq!(state, before);
    }

    #[test]
    fn floating_desktop_state_collapses_windows_when_only_title_bars_fit() {
        let defaults = FloatingWindowDefaults::new(
            UiPoint::new(24.0, 30.0),
            UiSize::new(180.0, 100.0),
            UiSize::new(120.0, 70.0),
        );
        let options = FloatingDesktopOptions::new(UiSize::new(340.0, 160.0))
            .with_margin(10.0)
            .with_gap(8.0);
        let mut state = FloatingDesktopState::new(FloatingDesktopZPolicy::new(10, 5, 100));

        let outcome = state.organize_windows(
            [
                ("a", defaults),
                ("b", defaults),
                ("c", defaults),
                ("d", defaults),
            ],
            options.bounds,
            &options,
        );

        assert_eq!(
            outcome,
            FloatingWindowOrganizeOutcome::Organized {
                mode: FloatingWindowOrganizeMode::Collapsed,
            }
        );
        for id in ["a", "b", "c", "d"] {
            assert!(state.is_collapsed(id), "{id} should be collapsed");
            assert_eq!(
                state.size(id, UiSize::ZERO).height,
                options.title_bar_height
            );
        }
    }

    #[test]
    fn floating_desktop_state_organizer_preserves_minimum_width() {
        let defaults = FloatingWindowDefaults::new(
            UiPoint::new(24.0, 30.0),
            UiSize::new(260.0, 80.0),
            UiSize::new(220.0, 60.0),
        );
        let options = FloatingDesktopOptions::new(UiSize::new(200.0, 180.0))
            .with_margin(10.0)
            .with_gap(8.0);
        let mut state = FloatingDesktopState::new(FloatingDesktopZPolicy::new(10, 5, 100));
        state.ensure_window("wide", defaults);
        let before = state.clone();

        let outcome = state.organize_windows([("wide", defaults)], options.bounds, &options);

        assert_eq!(outcome, FloatingWindowOrganizeOutcome::NoFit);
        assert_eq!(state, before);
    }

    #[test]
    fn floating_window_layout_wraps_auto_windows_without_overlap() {
        let windows = vec![
            FloatingWindowDescriptor::new("a", "A", UiSize::new(180.0, 80.0))
                .with_min_size(UiSize::new(120.0, 60.0)),
            FloatingWindowDescriptor::new("b", "B", UiSize::new(180.0, 80.0))
                .with_min_size(UiSize::new(120.0, 60.0)),
            FloatingWindowDescriptor::new("c", "C", UiSize::new(180.0, 80.0))
                .with_min_size(UiSize::new(120.0, 60.0)),
            FloatingWindowDescriptor::new("d", "D", UiSize::new(180.0, 80.0))
                .with_min_size(UiSize::new(120.0, 60.0)),
        ];
        let options = FloatingDesktopOptions::new(UiSize::new(420.0, 260.0))
            .with_margin(10.0)
            .with_gap(10.0);
        let placements = floating_window_layout(&windows, options.bounds, &options);

        assert_eq!(placements.len(), 4);
        assert_eq!(placements[0].rect.x, 10.0);
        assert_eq!(placements[1].rect.x, 200.0);
        assert_eq!(placements[2].rect.x, 10.0);
        assert_eq!(placements[2].rect.y, 100.0);
        for left in 0..placements.len() {
            for right in left + 1..placements.len() {
                assert!(!overlaps(placements[left].rect, placements[right].rect));
            }
        }
    }

    #[test]
    fn floating_window_layout_clamps_explicit_positions_to_bounds() {
        let windows = vec![FloatingWindowDescriptor::new(
            "inspector",
            "Inspector",
            UiSize::new(160.0, 120.0),
        )
        .with_position(UiPoint::new(380.0, 280.0))];
        let options = FloatingDesktopOptions::new(UiSize::new(400.0, 300.0))
            .with_margin(12.0)
            .with_gap(8.0);
        let placements = floating_window_layout(&windows, options.bounds, &options);

        assert_eq!(placements[0].rect.x, 228.0);
        assert_eq!(placements[0].rect.y, 192.0);
    }

    #[test]
    fn floating_window_layout_keeps_explicit_origin_when_resized_wide() {
        let windows =
            vec![
                FloatingWindowDescriptor::new("wide", "Wide", UiSize::new(600.0, 120.0))
                    .with_position(UiPoint::new(100.0, 40.0)),
            ];
        let options = FloatingDesktopOptions::new(UiSize::new(400.0, 300.0))
            .with_margin(12.0)
            .with_gap(8.0);
        let placements = floating_window_layout(&windows, options.bounds, &options);

        assert_eq!(placements[0].rect.x, 100.0);
        assert_eq!(placements[0].rect.width, 288.0);
        assert_eq!(placements[0].rect.y, 40.0);
    }

    #[test]
    fn floating_desktop_builds_window_shells_with_taffy_content() {
        let windows = vec![
            FloatingWindowDescriptor::new("one", "One", UiSize::new(180.0, 120.0)),
            FloatingWindowDescriptor::new("two", "Two", UiSize::new(180.0, 120.0))
                .with_title_action("window.two.activate")
                .with_close_action("window.two.close"),
            FloatingWindowDescriptor::new("hidden", "Hidden", UiSize::new(180.0, 120.0))
                .visible(false),
        ];
        let mut document = UiDocument::new(root_style(420.0, 260.0));
        let root = document.root;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            FloatingDesktopOptions::new(UiSize::new(420.0, 260.0)),
            |document, parent, window| {
                document.add_child(
                    parent,
                    UiNode::text(
                        format!("{}.body", window.id),
                        window.id.clone(),
                        TextStyle::default(),
                        LayoutStyle::new().with_width_percent(1.0),
                    ),
                );
            },
        );
        document
            .compute_layout(UiSize::new(420.0, 260.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(nodes.windows.len(), 2);
        assert_eq!(document.node(nodes.windows[0].root).style.z_index, 1);
        assert_eq!(document.node(nodes.windows[1].root).style.z_index, 33);
        assert_eq!(
            document.node(nodes.windows[1].title_bar).action.as_ref(),
            Some(&WidgetActionBinding::action("window.two.activate"))
        );
        assert_eq!(
            document
                .node(nodes.windows[1].close_button.expect("close button"))
                .action
                .as_ref(),
            Some(&WidgetActionBinding::action("window.two.close"))
        );
        assert_eq!(document.node(nodes.windows[0].content).children.len(), 1);
        assert!(document.accessibility_tree().iter().any(|node| {
            node.id == nodes.windows[0].root
                && node.role == AccessibilityRole::Window
                && node.label.as_deref() == Some("One")
        }));
    }

    #[test]
    fn floating_desktop_title_bar_can_emit_drag_edits() {
        let windows =
            vec![
                FloatingWindowDescriptor::new("movable", "Movable", UiSize::new(180.0, 120.0))
                    .with_drag_action("window.movable.drag"),
            ];
        let mut document = UiDocument::new(root_style(420.0, 260.0));
        let root = document.root;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            FloatingDesktopOptions::new(UiSize::new(420.0, 260.0)),
            |_document, _parent, _window| {},
        );

        let title_bar = document.node(nodes.windows[0].title_bar);
        assert_eq!(
            title_bar.action.as_ref(),
            Some(&WidgetActionBinding::action("window.movable.drag"))
        );
        assert_eq!(title_bar.action_mode, WidgetActionMode::PointerEdit);
    }

    #[test]
    fn floating_desktop_window_shell_occludes_lower_window_hits() {
        let windows = vec![
            FloatingWindowDescriptor::new("back", "Back", UiSize::new(180.0, 120.0))
                .with_position(UiPoint::new(20.0, 20.0))
                .with_activate_action("window.back.activate"),
            FloatingWindowDescriptor::new("front", "Front", UiSize::new(180.0, 120.0))
                .with_position(UiPoint::new(40.0, 40.0))
                .with_activate_action("window.front.activate"),
        ];
        let mut document = UiDocument::new(root_style(260.0, 200.0));
        let root = document.root;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            FloatingDesktopOptions::new(UiSize::new(260.0, 200.0)).with_margin(0.0),
            |_document, _parent, _window| {},
        );
        document
            .compute_layout(UiSize::new(260.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let hit = document.hit_test(UiPoint::new(80.0, 100.0));
        assert_eq!(hit, Some(nodes.windows[1].root));
        assert_eq!(
            document.node(nodes.windows[1].root).action.as_ref(),
            Some(&WidgetActionBinding::action("window.front.activate"))
        );
    }

    #[test]
    fn floating_desktop_keeps_descendants_inside_parent_z_band() {
        let windows = vec![
            FloatingWindowDescriptor::new("back", "Back", UiSize::new(180.0, 120.0))
                .with_position(UiPoint::new(18.0, 18.0)),
            FloatingWindowDescriptor::new("front", "Front", UiSize::new(180.0, 120.0))
                .with_position(UiPoint::new(36.0, 36.0)),
        ];
        let mut document = UiDocument::new(root_style(420.0, 260.0));
        let root = document.root;
        let mut back_high_child = None;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            FloatingDesktopOptions::new(UiSize::new(420.0, 260.0)),
            |document, parent, window| {
                let style = LayoutStyle::new().with_width_percent(1.0).with_height(24.0);
                let child = document.add_child(
                    parent,
                    UiNode::container(
                        format!("{}.high_child", window.id),
                        UiNodeStyle {
                            layout: style.style,
                            z_index: 120,
                            ..Default::default()
                        },
                    )
                    .with_visual(UiVisual::panel(
                        ColorRgba::new(255, 0, 0, 255),
                        None,
                        0.0,
                    )),
                );
                if window.id == "back" {
                    back_high_child = Some(child);
                }
            },
        );

        let back_root_z = document.node(nodes.windows[0].root).style.z_index;
        let front_root_z = document.node(nodes.windows[1].root).style.z_index;
        let back_child_z = document
            .node(back_high_child.expect("back child was added"))
            .style
            .z_index;

        assert_eq!(back_root_z, 1);
        assert_eq!(front_root_z, 33);
        assert_eq!(back_child_z, 32);
        assert!(back_child_z < front_root_z);
    }

    #[test]
    fn floating_desktop_resolves_child_z_relative_to_parent() {
        let windows = vec![FloatingWindowDescriptor::new(
            "slider",
            "Slider",
            UiSize::new(220.0, 120.0),
        )];
        let mut document = UiDocument::new(root_style(300.0, 180.0));
        let root = document.root;
        let mut fill = None;
        let mut thumb = None;
        floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            FloatingDesktopOptions::new(UiSize::new(300.0, 180.0)).with_margin(0.0),
            |document, parent, _window| {
                let slider_root = document.add_child(
                    parent,
                    UiNode::container(
                        "slider.root",
                        UiNodeStyle {
                            layout: LayoutStyle::new().with_width(120.0).with_height(30.0).style,
                            ..Default::default()
                        },
                    ),
                );
                let track = document.add_child(
                    slider_root,
                    UiNode::container(
                        "slider.track",
                        UiNodeStyle {
                            layout: LayoutStyle::new().with_width(120.0).with_height(6.0).style,
                            ..Default::default()
                        },
                    ),
                );
                fill = Some(document.add_child(
                    track,
                    UiNode::container(
                        "slider.fill",
                        UiNodeStyle {
                            layout: LayoutStyle::new().with_width(80.0).with_height(6.0).style,
                            ..Default::default()
                        },
                    ),
                ));
                thumb = Some(document.add_child(
                    slider_root,
                    UiNode::container(
                        "slider.thumb",
                        UiNodeStyle {
                            layout: LayoutStyle::new().with_width(12.0).with_height(12.0).style,
                            z_index: 3,
                            ..Default::default()
                        },
                    ),
                ));
            },
        );

        let fill_z = document.node(fill.expect("fill")).style.z_index;
        let thumb_z = document.node(thumb.expect("thumb")).style.z_index;
        assert!(thumb_z > fill_z, "thumb_z={thumb_z}, fill_z={fill_z}");
    }

    #[test]
    fn floating_desktop_lays_out_and_paints_window_content() {
        let windows = vec![FloatingWindowDescriptor::new(
            "labels",
            "Labels",
            UiSize::new(260.0, 130.0),
        )];
        let mut document = UiDocument::new(root_style(420.0, 260.0));
        let mut body_label = None;
        let options = FloatingDesktopOptions::new(UiSize::new(420.0, 260.0));
        let content_padding = options.content_padding;
        let nodes = floating_desktop(
            &mut document,
            UiNodeId(0),
            "desk",
            &windows,
            options,
            |document, parent, _window| {
                body_label = Some(document.add_child(
                    parent,
                    UiNode::text(
                        "labels.body.text",
                        "Plain label",
                        TextStyle::default(),
                        LayoutStyle::new().with_width_percent(1.0),
                    ),
                ));
            },
        );
        document
            .compute_layout(UiSize::new(420.0, 260.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let window = &nodes.windows[0];
        let content_rect = document.node(window.content).layout.rect;
        assert!(content_rect.width > 0.0);
        assert!(content_rect.height > 0.0);

        let label = body_label.expect("body label was added");
        let label_layout = document.node(label).layout;
        assert!(label_layout.visible);
        assert!((label_layout.rect.x - (content_rect.x + content_padding)).abs() < 0.01);
        assert!(label_layout.rect.width > 0.0);
        assert!(label_layout.rect.height > 0.0);
        assert!(label_layout.clip_rect.width > 0.0);
        assert!(label_layout.clip_rect.height > 0.0);

        let paint = document.paint_list();
        assert!(paint.items.iter().any(|item| {
            item.node == label
                && matches!(item.kind, crate::PaintKind::Text(_))
                && item.rect.width > 0.0
                && item.rect.height > 0.0
                && item.clip_rect.width > 0.0
                && item.clip_rect.height > 0.0
        }));
    }

    #[test]
    fn floating_desktop_grows_window_to_measured_content_minimum() {
        let windows = vec![FloatingWindowDescriptor::new(
            "wide_content",
            "Wide content",
            UiSize::new(120.0, 90.0),
        )
        .with_min_size(UiSize::new(80.0, 70.0))
        .with_position(UiPoint::new(20.0, 20.0))];
        let mut document = UiDocument::new(root_style(420.0, 260.0));
        let root = document.root;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            FloatingDesktopOptions::new(UiSize::new(420.0, 260.0)).with_margin(10.0),
            |document, parent, _window| {
                document.add_child(
                    parent,
                    UiNode::container("wide.fixed", LayoutStyle::size(260.0, 42.0)),
                );
            },
        );
        document
            .compute_layout(UiSize::new(420.0, 260.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let window = document.node(nodes.windows[0].root).layout.rect;
        let content = document.node(nodes.windows[0].content).layout.rect;
        assert!(window.width >= 280.0, "{window:?}");
        assert!(window.height >= 94.0, "{window:?}");
        assert!(
            content.right() <= window.right(),
            "content={content:?} window={window:?}"
        );
    }

    #[test]
    fn floating_desktop_applies_published_content_minimum_to_window_constraint() {
        let windows = vec![FloatingWindowDescriptor::new(
            "published_min",
            "Published minimum",
            UiSize::new(140.0, 90.0),
        )
        .with_min_size(UiSize::new(80.0, 60.0))
        .with_content_min_size(UiSize::new(260.0, 120.0))
        .with_auto_size_to_content(true)
        .with_position(UiPoint::new(20.0, 20.0))];
        let options = FloatingDesktopOptions::new(UiSize::new(420.0, 260.0)).with_margin(10.0);
        let mut document = UiDocument::new(root_style(420.0, 260.0));
        let root = document.root;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            options.clone(),
            |document, parent, _window| {
                document.add_child(
                    parent,
                    UiNode::container("published_min.fill", LayoutStyle::size(24.0, 24.0)),
                );
            },
        );
        document
            .compute_layout(UiSize::new(420.0, 260.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let window = document.node(nodes.windows[0].root).layout.rect;
        let expected_width = 260.0 + options.content_padding * 2.0;
        let expected_height = 120.0 + options.title_bar_height + options.content_padding * 2.0;
        assert!(window.width >= expected_width, "{window:?}");
        assert!(window.height >= expected_height, "{window:?}");
    }

    #[test]
    fn floating_desktop_collapsed_window_uses_title_minimum_not_hidden_content_minimum() {
        let windows = vec![FloatingWindowDescriptor::new(
            "published_min",
            "Published minimum",
            UiSize::new(140.0, 90.0),
        )
        .with_min_size(UiSize::new(160.0, 60.0))
        .with_content_min_size(UiSize::new(620.0, 360.0))
        .collapsed(true)
        .with_auto_size_to_content(true)
        .with_position(UiPoint::new(20.0, 20.0))];
        let options = FloatingDesktopOptions::new(UiSize::new(720.0, 420.0)).with_margin(10.0);
        let mut document = UiDocument::new(root_style(720.0, 420.0));
        let root = document.root;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            options.clone(),
            |document, parent, _window| {
                document.add_child(
                    parent,
                    UiNode::container("published_min.fill", LayoutStyle::size(620.0, 360.0)),
                );
            },
        );
        document
            .compute_layout(UiSize::new(720.0, 420.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let window = document.node(nodes.windows[0].root).layout.rect;
        assert!(window.width < 300.0, "{window:?}");
        assert_eq!(window.height, options.title_bar_height);
        assert!(
            document.node(nodes.windows[0].content).children.is_empty(),
            "collapsed windows should not publish hidden body children to layout/audit"
        );
    }

    #[test]
    fn floating_desktop_scroll_content_min_height_is_bounded_by_desktop() {
        let windows = vec![FloatingWindowDescriptor::new(
            "scroll_content",
            "Scroll content",
            UiSize::new(240.0, 110.0),
        )
        .with_min_size(UiSize::new(120.0, 80.0))
        .with_position(UiPoint::new(20.0, 20.0))];
        let mut document = UiDocument::new(root_style(420.0, 260.0));
        let root = document.root;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            FloatingDesktopOptions::new(UiSize::new(420.0, 260.0)).with_margin(10.0),
            |document, parent, _window| {
                let mut layout = LayoutStyle::column()
                    .with_width_percent(1.0)
                    .with_height_percent(1.0)
                    .with_flex_grow(1.0);
                layout.as_taffy_style_mut().min_size.height = length(48.0);
                let scroll = document.add_child(
                    parent,
                    UiNode::container("scroll_content.body", layout)
                        .with_scroll(ScrollAxes::VERTICAL),
                );
                for index in 0..12 {
                    document.add_child(
                        scroll,
                        UiNode::container(
                            format!("scroll_content.row.{index}"),
                            LayoutStyle::new()
                                .with_width_percent(1.0)
                                .with_height(24.0)
                                .with_flex_shrink(0.0),
                        ),
                    );
                }
            },
        );
        document
            .compute_layout(UiSize::new(420.0, 260.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let window = document.node(nodes.windows[0].root).layout.rect;
        assert!(
            window.height >= 230.0 && window.height <= 240.0,
            "{window:?}"
        );
    }

    #[test]
    fn floating_desktop_unbounded_scroll_content_contributes_to_min_window_height() {
        let windows = vec![FloatingWindowDescriptor::new(
            "scroll_content",
            "Scroll content",
            UiSize::new(240.0, 110.0),
        )
        .with_min_size(UiSize::new(120.0, 80.0))
        .with_position(UiPoint::new(20.0, 20.0))];
        let mut document = UiDocument::new(root_style(420.0, 380.0));
        let root = document.root;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            FloatingDesktopOptions::new(UiSize::new(420.0, 380.0)).with_margin(10.0),
            |document, parent, _window| {
                let scroll = document.add_child(
                    parent,
                    UiNode::container(
                        "scroll_content.body",
                        LayoutStyle::column()
                            .with_width_percent(1.0)
                            .with_height_percent(1.0)
                            .with_flex_grow(1.0),
                    )
                    .with_scroll(ScrollAxes::VERTICAL),
                );
                for index in 0..8 {
                    document.add_child(
                        scroll,
                        UiNode::container(
                            format!("scroll_content.row.{index}"),
                            LayoutStyle::new()
                                .with_width_percent(1.0)
                                .with_height(24.0)
                                .with_flex_shrink(0.0),
                        ),
                    );
                }
            },
        );
        document
            .compute_layout(UiSize::new(420.0, 380.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let window = document.node(nodes.windows[0].root).layout.rect;
        assert!(window.height >= 240.0, "{window:?}");
    }

    #[test]
    fn floating_desktop_scroll_viewport_minimum_contributes_to_window_min_height() {
        let windows = vec![FloatingWindowDescriptor::new(
            "scroll_viewport",
            "Scroll viewport",
            UiSize::new(240.0, 90.0),
        )
        .with_min_size(UiSize::new(120.0, 70.0))
        .with_position(UiPoint::new(20.0, 20.0))];
        let mut document = UiDocument::new(root_style(420.0, 280.0));
        let root = document.root;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            FloatingDesktopOptions::new(UiSize::new(420.0, 280.0)).with_margin(10.0),
            |document, parent, _window| {
                let mut layout = LayoutStyle::column()
                    .with_width_percent(1.0)
                    .with_height_percent(1.0)
                    .with_flex_grow(1.0);
                layout.as_taffy_style_mut().min_size.height = length(140.0);
                let scroll = document.add_child(
                    parent,
                    UiNode::container("scroll_viewport.body", layout)
                        .with_scroll(ScrollAxes::VERTICAL),
                );
                for index in 0..12 {
                    document.add_child(
                        scroll,
                        UiNode::container(
                            format!("scroll_viewport.row.{index}"),
                            LayoutStyle::new()
                                .with_width_percent(1.0)
                                .with_height(24.0)
                                .with_flex_shrink(0.0),
                        ),
                    );
                }
            },
        );
        document
            .compute_layout(UiSize::new(420.0, 280.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let window = document.node(nodes.windows[0].root).layout.rect;
        let viewport = document
            .nodes()
            .iter()
            .find(|node| node.name == "scroll_viewport.body")
            .expect("scroll viewport")
            .layout
            .rect;
        assert!(window.height >= 190.0, "{window:?}");
        assert!(viewport.height >= 140.0, "{viewport:?}");
    }

    #[test]
    fn floating_desktop_auto_sizes_spawn_to_preferred_content() {
        let windows =
            vec![
                FloatingWindowDescriptor::new("compact", "Compact", UiSize::new(360.0, 220.0))
                    .with_min_size(UiSize::new(80.0, 70.0))
                    .with_auto_size_to_content(true)
                    .with_position(UiPoint::new(20.0, 20.0)),
            ];
        let mut document = UiDocument::new(root_style(420.0, 260.0));
        let root = document.root;
        let nodes = floating_desktop(
            &mut document,
            root,
            "desk",
            &windows,
            FloatingDesktopOptions::new(UiSize::new(420.0, 260.0)).with_margin(10.0),
            |document, parent, _window| {
                document.add_child(
                    parent,
                    UiNode::container("compact.fixed", LayoutStyle::size(120.0, 42.0)),
                );
            },
        );
        document
            .compute_layout(UiSize::new(420.0, 260.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let window = document.node(nodes.windows[0].root).layout.rect;
        assert!(window.width < 220.0, "{window:?}");
        assert!(window.height < 150.0, "{window:?}");
        assert!(window.width >= 140.0, "{window:?}");
        assert!(window.height >= 94.0, "{window:?}");
    }

    #[test]
    fn floating_desktop_state_disables_auto_size_after_user_resize() {
        let defaults = FloatingWindowDefaults::new(
            UiPoint::new(20.0, 20.0),
            UiSize::new(240.0, 140.0),
            UiSize::new(80.0, 70.0),
        );
        let mut state = FloatingDesktopState::default();
        state.ensure_window("compact", defaults);
        state.apply_resize(
            "compact",
            WidgetPointerEdit::new(
                crate::WidgetValueEditPhase::Begin,
                UiPoint::new(240.0, 140.0),
                UiPoint::new(0.0, 0.0),
                UiRect::new(20.0, 20.0, 240.0, 140.0),
            ),
            defaults,
        );
        state.apply_resize(
            "compact",
            WidgetPointerEdit::new(
                crate::WidgetValueEditPhase::Commit,
                UiPoint::new(300.0, 180.0),
                UiPoint::new(0.0, 0.0),
                UiRect::new(20.0, 20.0, 240.0, 140.0),
            ),
            defaults,
        );

        let mut descriptor =
            FloatingWindowDescriptor::new("compact", "Compact", UiSize::new(1.0, 1.0))
                .with_auto_size_to_content(true);
        state.apply_to_descriptor(&mut descriptor, defaults);

        assert!(!descriptor.auto_size_to_content);
        assert_eq!(descriptor.preferred_size, UiSize::new(300.0, 180.0));
    }

    #[test]
    fn floating_desktop_state_disables_auto_size_for_known_window_sizes() {
        let defaults = FloatingWindowDefaults::new(
            UiPoint::new(20.0, 20.0),
            UiSize::new(240.0, 140.0),
            UiSize::new(80.0, 70.0),
        );
        let mut state = FloatingDesktopState::default();
        state.ensure_window("compact", defaults);
        state
            .sizes
            .insert("compact".to_string(), UiSize::new(260.0, 150.0));

        let mut descriptor =
            FloatingWindowDescriptor::new("compact", "Compact", UiSize::new(1.0, 1.0))
                .with_auto_size_to_content(true);
        state.apply_to_descriptor(&mut descriptor, defaults);

        assert!(!descriptor.auto_size_to_content);
        assert_eq!(descriptor.preferred_size, UiSize::new(260.0, 150.0));
    }

    #[test]
    fn floating_desktop_state_resizes_from_rendered_window_rect() {
        let defaults = FloatingWindowDefaults::new(
            UiPoint::new(20.0, 20.0),
            UiSize::new(240.0, 140.0),
            UiSize::new(80.0, 70.0),
        );
        let mut state = FloatingDesktopState::default();
        state.ensure_window("compact", defaults);
        state
            .sizes
            .insert("compact".to_string(), UiSize::new(80.0, 70.0));
        state.user_sized.insert("compact".to_string());

        state.apply_resize(
            "compact",
            WidgetPointerEdit::new(
                crate::WidgetValueEditPhase::Begin,
                UiPoint::new(260.0, 160.0),
                UiPoint::new(240.0, 140.0),
                UiRect::new(20.0, 20.0, 260.0, 140.0),
            ),
            defaults,
        );
        state.apply_resize(
            "compact",
            WidgetPointerEdit::new(
                crate::WidgetValueEditPhase::Commit,
                UiPoint::new(300.0, 190.0),
                UiPoint::new(280.0, 170.0),
                UiRect::new(20.0, 20.0, 260.0, 140.0),
            ),
            defaults,
        );

        assert_eq!(
            state.size("compact", defaults.size),
            UiSize::new(300.0, 170.0)
        );
    }

    fn overlaps(left: UiRect, right: UiRect) -> bool {
        left.x < right.x + right.width
            && left.x + left.width > right.x
            && left.y < right.y + right.height
            && left.y + left.height > right.y
    }
}
