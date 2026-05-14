use super::*;

#[derive(Debug, Clone)]
pub struct ScrollAreaWithBarsOptions {
    pub layout: LayoutStyle,
    pub axes: ScrollAxes,
    pub vertical_scrollbar: ScrollbarOptions,
    pub horizontal_scrollbar: ScrollbarOptions,
    pub scrollbar_thickness: f32,
    pub gap: f32,
    pub accessibility_label: Option<String>,
}

impl Default for ScrollAreaWithBarsOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column()
                .with_width(240.0)
                .with_height(160.0)
                .with_gap(4.0),
            axes: ScrollAxes::VERTICAL,
            vertical_scrollbar: ScrollbarOptions::default(),
            horizontal_scrollbar: ScrollbarOptions::default()
                .with_layout(LayoutStyle::size(120.0, 8.0))
                .with_track_size(UiSize::new(120.0, 8.0)),
            scrollbar_thickness: 8.0,
            gap: 4.0,
            accessibility_label: None,
        }
    }
}

impl ScrollAreaWithBarsOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
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

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollAreaWithBarsNodes {
    pub root: UiNodeId,
    pub row: UiNodeId,
    pub viewport: UiNodeId,
    pub vertical_scrollbar: Option<UiNodeId>,
    pub horizontal_row: Option<UiNodeId>,
    pub horizontal_scrollbar: Option<UiNodeId>,
}

pub fn scroll_area(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    axes: ScrollAxes,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    let name = name.into();
    let layout = layout.into();
    document.add_child(
        parent,
        UiNode::container(
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
        ),
    )
}

pub fn scroll_area_with_bars(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    scroll: ScrollState,
    options: ScrollAreaWithBarsOptions,
) -> ScrollAreaWithBarsNodes {
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
            AccessibilityMeta::new(AccessibilityRole::Group).label(
                options
                    .accessibility_label
                    .clone()
                    .unwrap_or_else(|| name.clone()),
            ),
        ),
    );
    let row = document.add_child(
        root,
        UiNode::container(
            format!("{name}.row"),
            UiNodeStyle {
                layout: LayoutStyle::row()
                    .with_width_percent(1.0)
                    .with_flex_grow(1.0)
                    .with_gap(options.gap)
                    .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    let viewport = scroll_area(
        document,
        row,
        format!("{name}.viewport"),
        options.axes,
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height_percent(1.0)
            .with_flex_grow(1.0),
    );
    let vertical_scrollbar = options.axes.vertical.then(|| {
        scrollbar(
            document,
            row,
            format!("{name}.vertical-scrollbar"),
            scroll,
            ScrollAxis::Vertical,
            aligned_scrollbar_options(
                scroll,
                ScrollAxis::Vertical,
                options.scrollbar_thickness,
                options.vertical_scrollbar.clone(),
            ),
        )
    });
    let (horizontal_row, horizontal_scrollbar) = if options.axes.horizontal {
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
        let horizontal_scrollbar = scrollbar(
            document,
            horizontal_row,
            format!("{name}.horizontal-scrollbar"),
            scroll,
            ScrollAxis::Horizontal,
            aligned_scrollbar_options(
                scroll,
                ScrollAxis::Horizontal,
                options.scrollbar_thickness,
                options.horizontal_scrollbar.clone(),
            )
            .with_layout(
                LayoutStyle::new()
                    .with_width_percent(1.0)
                    .with_height(options.scrollbar_thickness)
                    .with_flex_grow(1.0),
            ),
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

    ScrollAreaWithBarsNodes {
        root,
        row,
        viewport,
        vertical_scrollbar,
        horizontal_row,
        horizontal_scrollbar,
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
        ScrollState {
            axes: ScrollAxes::BOTH,
            offset: UiPoint::new(40.0, 60.0),
            viewport_size: UiSize::new(200.0, 100.0),
            content_size: UiSize::new(400.0, 300.0),
        }
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
}
