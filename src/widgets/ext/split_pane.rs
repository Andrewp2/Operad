//! Split pane widget.

use taffy::prelude::{Dimension, Display, FlexDirection, Size as TaffySize, Style};

use crate::{
    length, AccessibilityMeta, AccessibilityRole, ClipBehavior, InputBehavior, InteractionVisuals,
    LayoutStyle, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiVisual, WidgetActionBinding,
    WidgetActionMode,
};

use super::surfaces::{DEFAULT_ACCENT, DEFAULT_SURFACE_STROKE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

impl SplitAxis {
    pub const fn flex_direction(self) -> FlexDirection {
        match self {
            Self::Horizontal => FlexDirection::Row,
            Self::Vertical => FlexDirection::Column,
        }
    }

    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitPaneSizes {
    pub first: f32,
    pub handle: f32,
    pub second: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitPaneState {
    pub fraction: f32,
    pub min_first: f32,
    pub min_second: f32,
}

impl SplitPaneState {
    pub fn new(fraction: f32) -> Self {
        Self {
            fraction: fraction.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    pub fn with_min_sizes(mut self, first: f32, second: f32) -> Self {
        self.min_first = first.max(0.0);
        self.min_second = second.max(0.0);
        self
    }

    pub fn set_fraction(&mut self, fraction: f32) -> bool {
        if !fraction.is_finite() {
            return false;
        }
        let fraction = fraction.clamp(0.0, 1.0);
        if (self.fraction - fraction).abs() <= f32::EPSILON {
            return false;
        }
        self.fraction = fraction;
        true
    }

    pub fn resolved_sizes(self, total_extent: f32, handle_thickness: f32) -> SplitPaneSizes {
        let total = total_extent.max(0.0);
        let handle = handle_thickness.max(0.0).min(total);
        let available = (total - handle).max(0.0);
        if available <= f32::EPSILON {
            return SplitPaneSizes {
                first: 0.0,
                handle,
                second: 0.0,
            };
        }

        let mut min_first = self.min_first.max(0.0);
        let mut min_second = self.min_second.max(0.0);
        let min_total = min_first + min_second;
        if min_total > available && min_total > f32::EPSILON {
            let scale = available / min_total;
            min_first *= scale;
            min_second *= scale;
        }

        let lower = min_first.min(available);
        let upper = (available - min_second).max(lower);
        let desired = available * self.fraction.clamp(0.0, 1.0);
        let first = desired.clamp(lower, upper);
        SplitPaneSizes {
            first,
            handle,
            second: (available - first).max(0.0),
        }
    }

    pub fn resize_by(&mut self, delta: f32, total_extent: f32, handle_thickness: f32) -> bool {
        if !delta.is_finite() || !total_extent.is_finite() || !handle_thickness.is_finite() {
            return false;
        }
        let available = (total_extent.max(0.0) - handle_thickness.max(0.0)).max(0.0);
        if available <= f32::EPSILON {
            return false;
        }
        let sizes = self.resolved_sizes(total_extent, handle_thickness);
        let next_first = (sizes.first + delta).clamp(0.0, available);
        self.set_fraction(next_first / available)
    }
}

impl Default for SplitPaneState {
    fn default() -> Self {
        Self {
            fraction: 0.5,
            min_first: 48.0,
            min_second: 48.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SplitPaneOptions {
    pub layout: LayoutStyle,
    pub handle_thickness: f32,
    pub root_visual: UiVisual,
    pub pane_visual: UiVisual,
    pub handle_visual: UiVisual,
    pub handle_hover_visual: Option<UiVisual>,
    pub handle_action: Option<WidgetActionBinding>,
}

impl Default for SplitPaneOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            }),
            handle_thickness: 2.0,
            root_visual: UiVisual::TRANSPARENT,
            pane_visual: UiVisual::TRANSPARENT,
            handle_visual: UiVisual::panel(DEFAULT_SURFACE_STROKE, None, 2.0),
            handle_hover_visual: Some(UiVisual::panel(DEFAULT_ACCENT, None, 2.0)),
            handle_action: None,
        }
    }
}

impl SplitPaneOptions {
    pub fn with_handle_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.handle_action = Some(action.into());
        self
    }

    pub fn with_handle_hover_visual(mut self, visual: UiVisual) -> Self {
        self.handle_hover_visual = Some(visual);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitPaneNodes {
    pub root: UiNodeId,
    pub first: UiNodeId,
    pub handle: UiNodeId,
    pub second: UiNodeId,
}

#[allow(clippy::too_many_arguments)]
pub fn split_pane(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    axis: SplitAxis,
    state: SplitPaneState,
    options: SplitPaneOptions,
    build_first: impl FnOnce(&mut UiDocument, UiNodeId),
    build_second: impl FnOnce(&mut UiDocument, UiNodeId),
) -> SplitPaneNodes {
    let name = name.into();
    let mut layout = options.layout;
    {
        let layout = layout.as_taffy_style_mut();
        layout.display = Display::Flex;
        layout.flex_direction = axis.flex_direction();
    }

    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.root_visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Application)
                .label(format!("{name} split pane"))
                .hint("Contains two resizable panes"),
        ),
    );
    let first = document.add_child(
        root,
        UiNode::container(
            format!("{name}.first"),
            split_pane_child_style(axis, state.fraction, state.min_first),
        )
        .with_visual(options.pane_visual),
    );
    build_first(document, first);

    let handle_visuals = InteractionVisuals::new(options.handle_visual)
        .hovered(options.handle_hover_visual.unwrap_or(options.handle_visual))
        .pressed(options.handle_hover_visual.unwrap_or(options.handle_visual))
        .pressed_hovered(options.handle_hover_visual.unwrap_or(options.handle_visual));
    let mut handle_node = UiNode::container(
        format!("{name}.handle"),
        split_pane_handle_style(axis, options.handle_thickness),
    )
    .with_input(InputBehavior::BUTTON)
    .with_interaction_visuals(handle_visuals)
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Slider)
            .label(format!("{name} splitter"))
            .value(format!("{:.0}%", state.fraction.clamp(0.0, 1.0) * 100.0))
            .hint(match axis {
                SplitAxis::Horizontal => "Resize the left and right panes",
                SplitAxis::Vertical => "Resize the upper and lower panes",
            })
            .focusable(),
    );
    if let Some(action) = options.handle_action {
        handle_node = handle_node
            .with_action(action)
            .with_action_mode(WidgetActionMode::PointerEditParentRect);
    }
    let handle = document.add_child(root, handle_node);

    let second = document.add_child(
        root,
        UiNode::container(
            format!("{name}.second"),
            split_pane_child_style(axis, 1.0 - state.fraction, state.min_second),
        )
        .with_visual(options.pane_visual),
    );
    build_second(document, second);

    SplitPaneNodes {
        root,
        first,
        handle,
        second,
    }
}

fn split_pane_child_style(axis: SplitAxis, grow: f32, min_extent: f32) -> UiNodeStyle {
    let mut layout = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        flex_basis: length(0.0),
        flex_grow: grow.max(0.0),
        flex_shrink: 1.0,
        ..Default::default()
    };
    if axis.is_horizontal() {
        layout.size.height = Dimension::percent(1.0);
        layout.min_size.width = length(min_extent.max(0.0));
    } else {
        layout.size.width = Dimension::percent(1.0);
        layout.min_size.height = length(min_extent.max(0.0));
    }
    UiNodeStyle {
        layout: LayoutStyle::from_taffy_style(layout).style,
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

fn split_pane_handle_style(axis: SplitAxis, thickness: f32) -> UiNodeStyle {
    let thickness = thickness.max(0.0);
    let size = if axis.is_horizontal() {
        TaffySize {
            width: length(thickness),
            height: Dimension::percent(1.0),
        }
    } else {
        TaffySize {
            width: Dimension::percent(1.0),
            height: length(thickness),
        }
    };
    UiNodeStyle {
        layout: LayoutStyle::from_taffy_style(Style {
            flex_shrink: 0.0,
            size,
            ..Default::default()
        })
        .style,
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}
