use super::*;

/// Options for an accessibility grid composed from row and cell widgets.
///
/// This is a semantic data/grid widget helper. For CSS Grid-style layout
/// composition, prefer [`crate::layout::Layout::grid`] with grid track helpers.
#[derive(Debug, Clone)]
pub struct GridOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub accessibility_label: Option<String>,
}

impl Default for GridOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(6.0),
                    height: taffy::prelude::LengthPercentage::length(6.0),
                },
                ..Default::default()
            }),
            visual: UiVisual::TRANSPARENT,
            accessibility_label: None,
        }
    }
}

impl GridOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }
}

pub fn grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: GridOptions,
) -> UiNodeId {
    let name = name.into();
    document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Grid)
                .label(options.accessibility_label.unwrap_or(name)),
        ),
    )
}

#[derive(Debug, Clone)]
pub struct GridRowOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub selected: bool,
}

impl Default for GridRowOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(6.0),
                    height: taffy::prelude::LengthPercentage::length(0.0),
                },
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            visual: UiVisual::TRANSPARENT,
            selected: false,
        }
    }
}

pub fn grid_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: GridRowOptions,
) -> UiNodeId {
    let name = name.into();
    document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Row)
                .label(name)
                .selected(options.selected),
        ),
    )
}

#[derive(Debug, Clone)]
pub struct GridCellOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub text_style: TextStyle,
    pub selected: bool,
    pub enabled: bool,
}

impl Default for GridCellOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: length(120.0),
                    height: length(28.0),
                },
                padding: taffy::prelude::Rect::length(4.0_f32),
                ..Default::default()
            }),
            visual: UiVisual::TRANSPARENT,
            text_style: TextStyle::default(),
            selected: false,
            enabled: true,
        }
    }
}

pub fn grid_text_cell(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    options: GridCellOptions,
) -> UiNodeId {
    let name = name.into();
    let text = text.into();
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::GridCell)
        .label(text.clone())
        .selected(options.selected);
    if !options.enabled {
        accessibility = accessibility.disabled();
    }
    let cell = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(accessibility),
    );
    label(
        document,
        cell,
        format!("{name}.label"),
        text,
        options.text_style,
        LayoutStyle::new(),
    );
    cell
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_helpers_build_accessible_rows_and_cells() {
        let mut document = UiDocument::new(root_style(360.0, 160.0));
        let root = document.root;
        let grid = grid(&mut document, root, "settings", GridOptions::default());
        let row = grid_row(
            &mut document,
            grid,
            "settings.row.1",
            GridRowOptions::default(),
        );
        let cell = grid_text_cell(
            &mut document,
            row,
            "settings.row.1.name",
            "Theme",
            GridCellOptions::default(),
        );

        assert_eq!(
            document.node(grid).accessibility.as_ref().unwrap().role,
            AccessibilityRole::Grid
        );
        assert_eq!(
            document.node(row).accessibility.as_ref().unwrap().role,
            AccessibilityRole::Row
        );
        assert_eq!(
            document.node(cell).accessibility.as_ref().unwrap().role,
            AccessibilityRole::GridCell
        );
        assert_eq!(document.node(cell).children.len(), 1);
    }
}
