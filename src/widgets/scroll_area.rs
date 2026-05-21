use super::*;
use crate::scrolling::ScrollbarVisibility;
use crate::widgets::scrollbar::{scrollbar, ScrollAxis, ScrollbarOptions};

#[derive(Debug, Clone)]
pub struct ScrollContainerOptions {
    pub layout: LayoutStyle,
    pub viewport_layout: LayoutStyle,
    pub axes: ScrollAxes,
    pub vertical_scrollbar: ScrollbarOptions,
    pub horizontal_scrollbar: ScrollbarOptions,
    pub scrollbar_thickness: f32,
    pub gap: f32,
    pub scrollbar_visibility: ScrollbarVisibility,
    pub auto_actions: bool,
    pub action_prefix: Option<String>,
    pub accessibility_label: Option<String>,
}

impl Default for ScrollContainerOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column()
                .with_width(240.0)
                .with_height(160.0)
                .with_gap(4.0),
            viewport_layout: LayoutStyle::new()
                .with_width(0.0)
                .with_height_percent(1.0)
                .with_flex_grow(1.0)
                .with_flex_shrink(1.0),
            axes: ScrollAxes::VERTICAL,
            vertical_scrollbar: ScrollbarOptions::default(),
            horizontal_scrollbar: ScrollbarOptions::default()
                .with_layout(LayoutStyle::size(120.0, 8.0))
                .with_track_size(UiSize::new(120.0, 8.0)),
            scrollbar_thickness: 8.0,
            gap: 4.0,
            scrollbar_visibility: ScrollbarVisibility::Auto,
            auto_actions: true,
            action_prefix: None,
            accessibility_label: None,
        }
    }
}

#[deprecated(
    since = "8.0.0",
    note = "scroll_area now provides automatic scrollbar affordances; use scroll_container for explicit scrollbar nodes"
)]
pub type ScrollAreaWithBarsOptions = ScrollContainerOptions;

impl ScrollContainerOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_viewport_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.viewport_layout = layout.into();
        self
    }

    pub const fn with_axes(mut self, axes: ScrollAxes) -> Self {
        self.axes = axes;
        self
    }

    pub fn with_vertical_scrollbar(mut self, options: ScrollbarOptions) -> Self {
        self.vertical_scrollbar = options;
        self
    }

    pub fn with_horizontal_scrollbar(mut self, options: ScrollbarOptions) -> Self {
        self.horizontal_scrollbar = options;
        self
    }

    pub const fn with_scrollbar_thickness(mut self, thickness: f32) -> Self {
        self.scrollbar_thickness = thickness;
        self
    }

    pub const fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub const fn with_scrollbar_visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scrollbar_visibility = visibility;
        self
    }

    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self.auto_actions = true;
        self
    }

    pub const fn without_actions(mut self) -> Self {
        self.auto_actions = false;
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollContainerNodes {
    pub root: UiNodeId,
    pub row: UiNodeId,
    pub viewport: UiNodeId,
    pub vertical_scrollbar: Option<UiNodeId>,
    pub horizontal_row: Option<UiNodeId>,
    pub horizontal_scrollbar: Option<UiNodeId>,
}

#[deprecated(
    since = "8.0.0",
    note = "scroll_area now provides automatic scrollbar affordances; use ScrollContainerNodes for explicit scrollbar nodes"
)]
pub type ScrollAreaWithBarsNodes = ScrollContainerNodes;

/// Create a clipped scroll viewport with automatic overlay scrollbars.
///
/// This is the default "just works" scroll primitive: when measured content
/// overflows the viewport, the renderer reveals hover-driven scrollbar
/// affordances on the overflowing axes. Use [`scroll_container`] when explicit
/// scrollbar nodes are needed.
pub fn scroll_area(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    axes: ScrollAxes,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    scroll_viewport(document, parent, name, axes, layout, true)
}

fn raw_scroll_area(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    axes: ScrollAxes,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    scroll_viewport(document, parent, name, axes, layout, false)
}

fn scroll_viewport(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    axes: ScrollAxes,
    layout: impl Into<LayoutStyle>,
    auto_scrollbar: bool,
) -> UiNodeId {
    let name = name.into();
    let layout = layout.into();
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: layout.style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_scroll(axes)
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::List)
            .label(name)
            .value(scroll_axes_value(axes)),
    );
    if !auto_scrollbar {
        node = node.without_auto_scrollbar();
    }
    document.add_child(parent, node)
}

pub fn scroll_container(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    scroll: ScrollState,
    options: ScrollContainerOptions,
    build_content: impl FnOnce(&mut UiDocument, UiNodeId),
) -> ScrollContainerNodes {
    let nodes = scroll_container_shell(document, parent, name, scroll, options);
    build_content(document, nodes.viewport);
    nodes
}

pub fn scroll_container_shell(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    scroll: ScrollState,
    options: ScrollContainerOptions,
) -> ScrollContainerNodes {
    let name = name.into();
    let action_prefix = options
        .action_prefix
        .clone()
        .unwrap_or_else(|| name.clone());
    let mut root_layout = options.layout.style.clone();
    root_layout.display = Display::Flex;
    root_layout.flex_direction = FlexDirection::Column;
    root_layout.gap = LayoutStyle::new().with_gap(options.gap).style.gap;
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: root_layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(
                options
                    .accessibility_label
                    .clone()
                    .unwrap_or_else(|| name.clone()),
            ),
        ),
    );
    let mut row_layout = LayoutStyle::row()
        .with_width_percent(1.0)
        .with_height(0.0)
        .with_flex_grow(1.0)
        .with_flex_shrink(1.0)
        .with_gap(options.gap);
    row_layout.style.min_size.width = length(0.0);
    row_layout.style.min_size.height = length(0.0);
    let row = document.add_child(
        root,
        UiNode::container(
            format!("{name}.row"),
            UiNodeStyle {
                layout: row_layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    let mut viewport_layout = options.viewport_layout.clone();
    viewport_layout.style.min_size.width = length(0.0);
    viewport_layout.style.min_size.height = length(0.0);
    let viewport = raw_scroll_area(
        document,
        row,
        format!("{name}.viewport"),
        options.axes,
        viewport_layout,
    );
    document.node_mut(viewport).action = options
        .auto_actions
        .then(|| WidgetActionBinding::from(format!("{action_prefix}.scroll")));
    if let Some(viewport_scroll) = document.node_mut(viewport).scroll.as_mut() {
        *viewport_scroll = ScrollState {
            axes: options.axes,
            ..scroll
        };
    }
    let vertical_scrollbar = show_scrollbar(
        options.scrollbar_visibility,
        options.axes,
        scroll,
        ScrollAxis::Vertical,
    )
    .then(|| {
        let mut scrollbar_options = aligned_scrollbar_options(
            scroll,
            ScrollAxis::Vertical,
            options.scrollbar_thickness,
            options.vertical_scrollbar.clone(),
        );
        if options.auto_actions && scrollbar_options.action.is_none() {
            scrollbar_options.action = Some(format!("{action_prefix}.vertical-scrollbar").into());
        }
        scrollbar(
            document,
            row,
            format!("{name}.vertical-scrollbar"),
            scroll,
            ScrollAxis::Vertical,
            scrollbar_options,
        )
    });
    let (horizontal_row, horizontal_scrollbar) = if show_scrollbar(
        options.scrollbar_visibility,
        options.axes,
        scroll,
        ScrollAxis::Horizontal,
    ) {
        let horizontal_row = document.add_child(
            root,
            UiNode::container(
                format!("{name}.horizontal-row"),
                UiNodeStyle {
                    layout: LayoutStyle::row()
                        .with_width_percent(1.0)
                        .with_height(options.scrollbar_thickness)
                        .with_flex_shrink(0.0)
                        .with_gap(options.gap)
                        .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        let mut horizontal_options = aligned_scrollbar_options(
            scroll,
            ScrollAxis::Horizontal,
            options.scrollbar_thickness,
            options.horizontal_scrollbar.clone(),
        )
        .with_layout(
            LayoutStyle::new()
                .with_width(0.0)
                .with_height(options.scrollbar_thickness)
                .with_flex_grow(1.0)
                .with_flex_shrink(1.0),
        );
        if options.auto_actions && horizontal_options.action.is_none() {
            horizontal_options.action =
                Some(format!("{action_prefix}.horizontal-scrollbar").into());
        }
        let horizontal_scrollbar = scrollbar(
            document,
            horizontal_row,
            format!("{name}.horizontal-scrollbar"),
            scroll,
            ScrollAxis::Horizontal,
            horizontal_options,
        );
        if options.axes.vertical {
            document.add_child(
                horizontal_row,
                UiNode::container(
                    format!("{name}.scrollbar-corner"),
                    UiNodeStyle {
                        layout: LayoutStyle::size(
                            options.scrollbar_thickness,
                            options.scrollbar_thickness,
                        )
                        .with_flex_shrink(0.0)
                        .style,
                        clip: ClipBehavior::Clip,
                        ..Default::default()
                    },
                ),
            );
        }
        (Some(horizontal_row), Some(horizontal_scrollbar))
    } else {
        (None, None)
    };

    ScrollContainerNodes {
        root,
        row,
        viewport,
        vertical_scrollbar,
        horizontal_row,
        horizontal_scrollbar,
    }
}

#[deprecated(
    since = "8.0.0",
    note = "scroll_area now provides automatic scrollbar affordances; use scroll_container for explicit scrollbar nodes"
)]
#[allow(deprecated)]
pub fn scroll_area_with_bars(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    scroll: ScrollState,
    options: ScrollAreaWithBarsOptions,
) -> ScrollAreaWithBarsNodes {
    scroll_container_shell(document, parent, name, scroll, options)
}

fn show_scrollbar(
    visibility: ScrollbarVisibility,
    axes: ScrollAxes,
    scroll: ScrollState,
    axis: ScrollAxis,
) -> bool {
    if !scroll_axis_enabled(axis, axes) {
        return false;
    }
    if axis.value(scroll.max_offset()) <= f32::EPSILON {
        return false;
    }
    match visibility {
        ScrollbarVisibility::Always => true,
        ScrollbarVisibility::Auto => true,
        ScrollbarVisibility::Hidden => false,
    }
}

fn scroll_axis_enabled(axis: ScrollAxis, axes: ScrollAxes) -> bool {
    match axis {
        ScrollAxis::Vertical => axes.vertical,
        ScrollAxis::Horizontal => axes.horizontal,
    }
}

pub fn aligned_scrollbar_options(
    scroll: ScrollState,
    axis: ScrollAxis,
    thickness: f32,
    mut options: ScrollbarOptions,
) -> ScrollbarOptions {
    let thickness = thickness.max(1.0);
    let track_size = match axis {
        ScrollAxis::Vertical => UiSize::new(
            thickness,
            if scroll.viewport_size.height > f32::EPSILON {
                scroll.viewport_size.height.max(thickness)
            } else {
                options.track_size.height.max(thickness)
            },
        ),
        ScrollAxis::Horizontal => UiSize::new(
            if scroll.viewport_size.width > f32::EPSILON {
                scroll.viewport_size.width.max(thickness)
            } else {
                options.track_size.width.max(thickness)
            },
            thickness,
        ),
    };
    options.track_size = track_size;
    options.layout = match axis {
        ScrollAxis::Vertical => {
            LayoutStyle::size(thickness, track_size.height).with_flex_shrink(0.0)
        }
        ScrollAxis::Horizontal => {
            LayoutStyle::size(track_size.width, thickness).with_flex_shrink(0.0)
        }
    };
    options
}

fn scroll_axes_value(axes: ScrollAxes) -> &'static str {
    match axes {
        ScrollAxes {
            horizontal: false,
            vertical: false,
        } => "not scrollable",
        ScrollAxes {
            horizontal: true,
            vertical: false,
        } => "horizontal",
        ScrollAxes {
            horizontal: false,
            vertical: true,
        } => "vertical",
        ScrollAxes {
            horizontal: true,
            vertical: true,
        } => "horizontal and vertical",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scroll_state() -> ScrollState {
        ScrollState::new(ScrollAxes::BOTH)
            .with_sizes(UiSize::new(200.0, 100.0), UiSize::new(400.0, 300.0))
            .with_offset(UiPoint::new(40.0, 60.0))
    }

    #[test]
    fn aligned_scrollbar_options_match_scroll_viewport() {
        let scroll = scroll_state();
        let vertical = aligned_scrollbar_options(
            scroll,
            ScrollAxis::Vertical,
            10.0,
            ScrollbarOptions::default(),
        );
        assert_eq!(vertical.track_size, UiSize::new(10.0, 100.0));
        let horizontal = aligned_scrollbar_options(
            scroll,
            ScrollAxis::Horizontal,
            10.0,
            ScrollbarOptions::default(),
        );
        assert_eq!(horizontal.track_size, UiSize::new(200.0, 10.0));

        let fallback = aligned_scrollbar_options(
            ScrollState::new(ScrollAxes::VERTICAL),
            ScrollAxis::Vertical,
            10.0,
            ScrollbarOptions::default(),
        );
        assert_eq!(fallback.track_size, UiSize::new(10.0, 120.0));
    }

    #[test]
    fn scroll_area_enables_automatic_scrollbars_by_default() {
        let mut document = UiDocument::new(root_style(320.0, 220.0));
        let root = document.root;
        let viewport = scroll_area(
            &mut document,
            root,
            "content",
            ScrollAxes::VERTICAL,
            LayoutStyle::size(120.0, 80.0),
        );

        let node = document.node(viewport);
        assert!(node.scroll.is_some());
        assert!(node.auto_scrollbar);
    }

    #[test]
    fn raw_scroll_area_keeps_scrollbar_affordance_opt_in() {
        let mut document = UiDocument::new(root_style(320.0, 220.0));
        let root = document.root;
        let viewport = raw_scroll_area(
            &mut document,
            root,
            "custom.chrome",
            ScrollAxes::VERTICAL,
            LayoutStyle::size(120.0, 80.0),
        );

        let node = document.node(viewport);
        assert!(node.scroll.is_some());
        assert!(!node.auto_scrollbar);
    }

    #[allow(deprecated)]
    #[test]
    fn scroll_area_with_bars_builds_viewport_and_aligned_scrollbars() {
        let mut document = UiDocument::new(root_style(320.0, 220.0));
        let root = document.root;
        let scroll = scroll_state();
        let nodes = scroll_area_with_bars(
            &mut document,
            root,
            "results",
            scroll,
            ScrollAreaWithBarsOptions::default()
                .with_layout(LayoutStyle::size(260.0, 180.0))
                .with_axes(ScrollAxes::BOTH)
                .with_scrollbar_thickness(10.0)
                .with_accessibility_label("Results"),
        );

        assert_eq!(
            document.node(nodes.viewport).scroll.unwrap().axes,
            ScrollAxes::BOTH
        );
        assert_eq!(
            document.node(nodes.viewport).scroll.unwrap().offset,
            scroll.offset
        );
        assert_eq!(
            document
                .node(nodes.viewport)
                .action
                .as_ref()
                .and_then(WidgetActionBinding::action_id)
                .map(|id| id.as_str()),
            Some("results.scroll")
        );
        assert_eq!(
            document
                .node(nodes.root)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Results")
        );
        let vertical = nodes.vertical_scrollbar.expect("vertical scrollbar");
        let horizontal = nodes.horizontal_scrollbar.expect("horizontal scrollbar");
        assert_eq!(
            document
                .node(vertical)
                .action
                .as_ref()
                .and_then(WidgetActionBinding::action_id)
                .map(|id| id.as_str()),
            Some("results.vertical-scrollbar")
        );
        assert_eq!(
            document
                .node(horizontal)
                .action
                .as_ref()
                .and_then(WidgetActionBinding::action_id)
                .map(|id| id.as_str()),
            Some("results.horizontal-scrollbar")
        );
        assert_eq!(
            document
                .node(vertical)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("30%")
        );
        assert_eq!(
            document
                .node(horizontal)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("20%")
        );
        assert!(nodes.horizontal_row.is_some());
        assert_eq!(document.node(vertical).children.len(), 1);
        assert_eq!(document.node(horizontal).children.len(), 1);
    }

    #[test]
    fn scroll_container_builds_content_and_uses_one_action_prefix() {
        let mut document = UiDocument::new(root_style(320.0, 220.0));
        let root = document.root;
        let nodes = scroll_container(
            &mut document,
            root,
            "events",
            scroll_state(),
            ScrollContainerOptions::default()
                .with_axes(ScrollAxes::VERTICAL)
                .with_action_prefix("timeline.events"),
            |document, viewport| {
                label(
                    document,
                    viewport,
                    "events.row",
                    "Row",
                    TextStyle::default(),
                    LayoutStyle::new().with_width_percent(1.0),
                );
            },
        );

        assert_eq!(document.node(nodes.viewport).children.len(), 1);
        assert!(nodes.vertical_scrollbar.is_some());
        assert!(nodes.horizontal_scrollbar.is_none());
        assert_eq!(
            document
                .node(nodes.viewport)
                .action
                .as_ref()
                .and_then(WidgetActionBinding::action_id)
                .map(|id| id.as_str()),
            Some("timeline.events.scroll")
        );
        let vertical = nodes.vertical_scrollbar.expect("vertical scrollbar");
        assert_eq!(
            document
                .node(vertical)
                .action
                .as_ref()
                .and_then(WidgetActionBinding::action_id)
                .map(|id| id.as_str()),
            Some("timeline.events.vertical-scrollbar")
        );
    }

    #[test]
    fn scroll_container_can_auto_hide_unneeded_scrollbars() {
        let mut document = UiDocument::new(root_style(320.0, 220.0));
        let root = document.root;
        let scroll = ScrollState::new(ScrollAxes::BOTH)
            .with_sizes(UiSize::new(200.0, 100.0), UiSize::new(200.0, 260.0));
        let nodes = scroll_container_shell(
            &mut document,
            root,
            "auto",
            scroll,
            ScrollContainerOptions::default()
                .with_axes(ScrollAxes::BOTH)
                .with_scrollbar_visibility(ScrollbarVisibility::Auto),
        );

        assert!(nodes.vertical_scrollbar.is_some());
        assert!(nodes.horizontal_scrollbar.is_none());
        assert!(nodes.horizontal_row.is_none());
    }

    #[test]
    fn scroll_container_omits_unneeded_scrollbars_even_when_visibility_is_always() {
        let mut document = UiDocument::new(root_style(320.0, 220.0));
        let root = document.root;
        let scroll = ScrollState::new(ScrollAxes::BOTH)
            .with_sizes(UiSize::new(200.0, 100.0), UiSize::new(200.0, 100.0));
        let nodes = scroll_container_shell(
            &mut document,
            root,
            "empty",
            scroll,
            ScrollContainerOptions::default()
                .with_axes(ScrollAxes::BOTH)
                .with_scrollbar_visibility(ScrollbarVisibility::Always),
        );

        assert!(nodes.vertical_scrollbar.is_none());
        assert!(nodes.horizontal_scrollbar.is_none());
        assert!(nodes.horizontal_row.is_none());
    }
}
