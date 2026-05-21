use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct TableColumn {
    pub id: String,
    pub label: String,
    pub width: f32,
}

pub fn table_header(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    columns: &[TableColumn],
) -> UiNodeId {
    let name = name.into();
    let row = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(28.0),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(34, 41, 50, 255),
            Some(StrokeStyle::new(ColorRgba::new(67, 78, 95, 255), 1.0)),
            0.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Grid)
                .label(name.clone())
                .value(format!("{} columns", columns.len())),
        ),
    );
    for column in columns {
        let cell = label(
            document,
            row,
            format!("{name}.{}", column.id),
            &column.label,
            TextStyle::default(),
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(column.width),
                    height: Dimension::percent(1.0),
                },
                padding: taffy::prelude::Rect::length(4.0_f32),
                ..Default::default()
            }),
        );
        document.node_mut(cell).accessibility = Some(
            AccessibilityMeta::new(AccessibilityRole::GridCell)
                .label(column.label.clone())
                .value(column.id.clone()),
        );
    }
    row
}
