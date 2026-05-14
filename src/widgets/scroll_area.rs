use super::*;

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
