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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidePanelSide {
    Left,
    Right,
}

impl SidePanelSide {
    pub const fn panel_kind(self) -> PanelKind {
        match self {
            Self::Left => PanelKind::Left,
            Self::Right => PanelKind::Right,
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

    pub fn with_clip(mut self, clip: ClipBehavior) -> Self {
        self.clip = clip;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
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

pub fn central_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
) -> UiNodeId {
    panel(document, parent, name, PanelOptions::central())
}

pub fn top_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    height: f32,
) -> UiNodeId {
    panel(document, parent, name, PanelOptions::top(height))
}

pub fn bottom_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    height: f32,
) -> UiNodeId {
    panel(document, parent, name, PanelOptions::bottom(height))
}

pub fn side_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    side: SidePanelSide,
    width: f32,
) -> UiNodeId {
    let options = match side {
        SidePanelSide::Left => PanelOptions::left(width),
        SidePanelSide::Right => PanelOptions::right(width),
    };
    panel(document, parent, name, options)
}

pub fn left_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    width: f32,
) -> UiNodeId {
    side_panel(document, parent, name, SidePanelSide::Left, width)
}

pub fn right_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    width: f32,
) -> UiNodeId {
    side_panel(document, parent, name, SidePanelSide::Right, width)
}

pub fn group_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
) -> UiNodeId {
    panel(document, parent, name, PanelOptions::group())
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

    #[test]
    fn panel_convenience_helpers_create_expected_panel_kinds() {
        let mut document = UiDocument::new(root_style(480.0, 320.0));
        let root = document.root;
        let top = top_panel(&mut document, root, "top", 32.0);
        let left = left_panel(&mut document, root, "left", 120.0);
        let central = central_panel(&mut document, root, "central");
        let right = right_panel(&mut document, root, "right", 96.0);
        let bottom = bottom_panel(&mut document, root, "bottom", 28.0);
        let group = group_panel(&mut document, root, "group");

        assert_eq!(
            document
                .node(top)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Top panel")
        );
        assert_eq!(
            document
                .node(left)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Left panel")
        );
        assert_eq!(
            document
                .node(central)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Central panel")
        );
        assert_eq!(
            document
                .node(right)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Right panel")
        );
        assert_eq!(
            document
                .node(bottom)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Bottom panel")
        );
        assert_eq!(
            document
                .node(group)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Group")
        );
    }
}
