use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Central,
    Top,
    Bottom,
    Left,
    Right,
    Group,
}

impl PanelKind {
    pub const fn accessibility_label(self) -> &'static str {
        match self {
            Self::Central => "Central panel",
            Self::Top => "Top panel",
            Self::Bottom => "Bottom panel",
            Self::Left => "Left panel",
            Self::Right => "Right panel",
            Self::Group => "Group",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PanelOptions {
    pub kind: PanelKind,
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub clip: ClipBehavior,
    pub scroll_axes: ScrollAxes,
    pub accessibility_label: Option<String>,
}

impl PanelOptions {
    pub fn central() -> Self {
        Self {
            kind: PanelKind::Central,
            layout: LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0)
                .with_flex_grow(1.0),
            ..Default::default()
        }
    }

    pub fn top(height: f32) -> Self {
        Self {
            kind: PanelKind::Top,
            layout: LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height(height)
                .with_flex_shrink(0.0),
            ..Default::default()
        }
    }

    pub fn bottom(height: f32) -> Self {
        Self {
            kind: PanelKind::Bottom,
            layout: LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height(height)
                .with_flex_shrink(0.0),
            ..Default::default()
        }
    }

    pub fn left(width: f32) -> Self {
        Self {
            kind: PanelKind::Left,
            layout: LayoutStyle::column()
                .with_width(width)
                .with_height_percent(1.0)
                .with_flex_shrink(0.0),
            ..Default::default()
        }
    }

    pub fn right(width: f32) -> Self {
        Self {
            kind: PanelKind::Right,
            layout: LayoutStyle::column()
                .with_width(width)
                .with_height_percent(1.0)
                .with_flex_shrink(0.0),
            ..Default::default()
        }
    }

    pub fn group() -> Self {
        Self {
            kind: PanelKind::Group,
            ..Default::default()
        }
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }

    pub fn with_scroll(mut self, axes: ScrollAxes) -> Self {
        self.scroll_axes = axes;
        self.clip = ClipBehavior::Clip;
        self
    }
}

impl Default for PanelOptions {
    fn default() -> Self {
        Self {
            kind: PanelKind::Group,
            layout: LayoutStyle::column().with_padding(8.0).with_gap(8.0),
            visual: UiVisual::panel(
                ColorRgba::new(24, 29, 36, 255),
                Some(StrokeStyle::new(ColorRgba::new(70, 82, 101, 255), 1.0)),
                0.0,
            ),
            clip: ClipBehavior::Clip,
            scroll_axes: ScrollAxes::NONE,
            accessibility_label: None,
        }
    }
}

pub fn panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: PanelOptions,
) -> UiNodeId {
    let name = name.into();
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style,
            clip: options.clip,
            ..Default::default()
        },
    )
    .with_visual(options.visual)
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Group).label(
            options
                .accessibility_label
                .unwrap_or_else(|| options.kind.accessibility_label().to_string()),
        ),
    );
    if options.scroll_axes != ScrollAxes::NONE {
        node = node.with_scroll(options.scroll_axes);
    }
    document.add_child(parent, node)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_helpers_create_scrollable_group_panels() {
        let mut document = UiDocument::new(root_style(320.0, 180.0));
        let root = document.root;
        let node = panel(
            &mut document,
            root,
            "main",
            PanelOptions::central().with_scroll(ScrollAxes::VERTICAL),
        );

        let panel_node = document.node(node);
        assert_eq!(
            panel_node.accessibility.as_ref().unwrap().label.as_deref(),
            Some("Central panel")
        );
        assert_eq!(panel_node.scroll.unwrap().axes, ScrollAxes::VERTICAL);
    }
}
