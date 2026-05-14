use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorOrientation {
    Horizontal,
    Vertical,
}

impl SeparatorOrientation {
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }
}

#[derive(Debug, Clone)]
pub struct SeparatorOptions {
    pub orientation: SeparatorOrientation,
    pub layout: Option<LayoutStyle>,
    pub visual: UiVisual,
    pub thickness: f32,
    pub margin: f32,
    pub accessibility_label: Option<String>,
}

impl SeparatorOptions {
    pub fn horizontal() -> Self {
        Self {
            orientation: SeparatorOrientation::Horizontal,
            ..Default::default()
        }
    }

    pub fn vertical() -> Self {
        Self {
            orientation: SeparatorOrientation::Vertical,
            ..Default::default()
        }
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = Some(layout.into());
        self
    }

    pub fn with_margin(mut self, margin: f32) -> Self {
        self.margin = margin;
        self
    }
}

impl Default for SeparatorOptions {
    fn default() -> Self {
        Self {
            orientation: SeparatorOrientation::Horizontal,
            layout: None,
            visual: UiVisual::panel(ColorRgba::new(67, 78, 95, 255), None, 0.0),
            thickness: 1.0,
            margin: 6.0,
            accessibility_label: None,
        }
    }
}

pub fn separator(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: SeparatorOptions,
) -> UiNodeId {
    let name = name.into();
    let layout = options.layout.unwrap_or_else(|| {
        if options.orientation.is_horizontal() {
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(options.thickness.max(0.0)),
                },
                margin: taffy::prelude::Rect {
                    left: taffy::prelude::LengthPercentageAuto::length(0.0),
                    right: taffy::prelude::LengthPercentageAuto::length(0.0),
                    top: taffy::prelude::LengthPercentageAuto::length(options.margin.max(0.0)),
                    bottom: taffy::prelude::LengthPercentageAuto::length(options.margin.max(0.0)),
                },
                flex_shrink: 0.0,
                ..Default::default()
            })
        } else {
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(options.thickness.max(0.0)),
                    height: Dimension::percent(1.0),
                },
                margin: taffy::prelude::Rect {
                    left: taffy::prelude::LengthPercentageAuto::length(options.margin.max(0.0)),
                    right: taffy::prelude::LengthPercentageAuto::length(options.margin.max(0.0)),
                    top: taffy::prelude::LengthPercentageAuto::length(0.0),
                    bottom: taffy::prelude::LengthPercentageAuto::length(0.0),
                },
                flex_shrink: 0.0,
                ..Default::default()
            })
        }
    });
    document.add_child(
        parent,
        UiNode::container(name.clone(), layout)
            .with_visual(options.visual)
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Separator)
                    .label(options.accessibility_label.unwrap_or(name)),
            ),
    )
}

pub fn spacer(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    let layout = layout.into();
    document.add_child(parent, UiNode::container(name, layout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separator_defaults_to_accessible_horizontal_rule() {
        let mut document = UiDocument::new(root_style(320.0, 120.0));
        let root = document.root;
        let node = separator(
            &mut document,
            root,
            "section.rule",
            SeparatorOptions::default(),
        );

        assert_eq!(
            document.node(node).accessibility.as_ref().unwrap().role,
            AccessibilityRole::Separator
        );
        assert_eq!(document.node(node).style.layout.size.height, length(1.0));
    }
}
