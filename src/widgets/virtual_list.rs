use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualListSpec {
    pub row_count: usize,
    pub row_height: f32,
    pub viewport_height: f32,
    pub scroll_offset: f32,
    pub overscan: usize,
}

impl VirtualListSpec {
    pub fn visible_range(self) -> Range<usize> {
        if self.row_count == 0 || self.row_height <= f32::EPSILON {
            return 0..0;
        }
        let first = (self.scroll_offset.max(0.0) / self.row_height).floor() as usize;
        let visible = (self.viewport_height / self.row_height).ceil() as usize + 1;
        let start = first.saturating_sub(self.overscan).min(self.row_count);
        let end = (first + visible + self.overscan).min(self.row_count);
        start..end
    }

    pub fn total_height(self) -> f32 {
        self.row_count as f32 * self.row_height
    }
}

pub fn virtual_list(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    spec: VirtualListSpec,
    mut build_row: impl FnMut(&mut UiDocument, UiNodeId, usize),
) -> UiNodeId {
    let name = name.into();
    let list = scroll_area(
        document,
        parent,
        name.clone(),
        ScrollAxes::VERTICAL,
        LayoutStyle::from_taffy_style(Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: TaffySize {
                width: Dimension::percent(1.0),
                height: length(spec.viewport_height),
            },
            ..Default::default()
        }),
    );
    document.node_mut(list).accessibility = Some(
        AccessibilityMeta::new(AccessibilityRole::List)
            .label(name.clone())
            .value(format!("{} items", spec.row_count)),
    );
    if let Some(scroll) = &mut document.nodes[list.0].scroll {
        scroll.offset.y = spec.scroll_offset.max(0.0);
    }
    let range = spec.visible_range();
    let top = range.start as f32 * spec.row_height;
    if top > 0.0 {
        document.add_child(list, spacer(format!("{name}.top_spacer"), top));
    }
    for row in range.clone() {
        build_row(document, list, row);
    }
    let bottom = (spec.row_count.saturating_sub(range.end)) as f32 * spec.row_height;
    if bottom > 0.0 {
        document.add_child(list, spacer(format!("{name}.bottom_spacer"), bottom));
    }
    list
}

fn spacer(name: impl Into<String>, height: f32) -> UiNode {
    UiNode::container(
        name,
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(height),
                },
                flex_shrink: 0.0,
                ..Default::default()
            })
            .style,
            ..Default::default()
        },
    )
}
