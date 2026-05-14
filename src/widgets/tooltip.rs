use super::*;

#[derive(Debug, Clone)]
pub struct TooltipBoxOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub title_text_style: TextStyle,
    pub body_text_style: TextStyle,
    pub shortcut_text_style: TextStyle,
    pub z_index: i16,
    pub layer: crate::platform::UiLayer,
    pub accessibility_label: Option<String>,
}

impl Default for TooltipBoxOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column()
                .with_width(240.0)
                .with_padding(8.0)
                .with_gap(4.0),
            visual: UiVisual::panel(
                ColorRgba::new(18, 23, 31, 245),
                Some(StrokeStyle::new(ColorRgba::new(92, 106, 128, 255), 1.0)),
                4.0,
            ),
            title_text_style: TextStyle {
                font_size: 14.0,
                line_height: 18.0,
                weight: FontWeight::BOLD,
                ..Default::default()
            },
            body_text_style: TextStyle {
                font_size: 13.0,
                line_height: 18.0,
                color: ColorRgba::new(198, 207, 219, 255),
                ..Default::default()
            },
            shortcut_text_style: TextStyle {
                font_size: 12.0,
                line_height: 16.0,
                color: ColorRgba::new(154, 168, 188, 255),
                ..Default::default()
            },
            z_index: 100,
            layer: crate::platform::UiLayer::AppOverlay,
            accessibility_label: None,
        }
    }
}

impl TooltipBoxOptions {
    pub fn at_rect(mut self, rect: UiRect) -> Self {
        self.layout = LayoutStyle::absolute_rect(rect);
        self
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }
}

pub fn tooltip_rect(
    anchor: UiRect,
    tooltip_size: UiSize,
    viewport: UiRect,
    placement: TooltipPlacement,
    offset: f32,
    cursor: Option<UiPoint>,
) -> UiRect {
    let offset = finite_or(offset, 0.0).max(0.0);
    let tooltip_size = UiSize::new(
        finite_or(tooltip_size.width, 0.0).max(0.0),
        finite_or(tooltip_size.height, 0.0).max(0.0),
    );
    let origin = match placement {
        TooltipPlacement::Above => UiPoint::new(anchor.x, anchor.y - tooltip_size.height - offset),
        TooltipPlacement::Below => UiPoint::new(anchor.x, anchor.bottom() + offset),
        TooltipPlacement::Left => UiPoint::new(anchor.x - tooltip_size.width - offset, anchor.y),
        TooltipPlacement::Right => UiPoint::new(anchor.right() + offset, anchor.y),
        TooltipPlacement::Cursor => cursor
            .map(|point| UiPoint::new(point.x + offset, point.y + offset))
            .unwrap_or_else(|| UiPoint::new(anchor.right() + offset, anchor.bottom() + offset)),
    };
    UiRect::new(
        clamp_tooltip_axis(origin.x, tooltip_size.width, viewport.x, viewport.right()),
        clamp_tooltip_axis(origin.y, tooltip_size.height, viewport.y, viewport.bottom()),
        tooltip_size.width,
        tooltip_size.height,
    )
}

fn clamp_tooltip_axis(value: f32, extent: f32, min: f32, max: f32) -> f32 {
    let min = finite_or(min, 0.0);
    let max = finite_or(max, min).max(min);
    let extent = finite_or(extent, 0.0).max(0.0);
    let upper = (max - extent).max(min);
    finite_or(value, min).clamp(min, upper)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

pub fn tooltip_box(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    content: TooltipContent,
    options: TooltipBoxOptions,
) -> UiNodeId {
    let name = name.into();
    let text = content.text();
    let tooltip = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                clip: ClipBehavior::Clip,
                z_index: options.z_index,
                ..Default::default()
            },
        )
        .with_layer(options.layer)
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Tooltip)
                .label(options.accessibility_label.unwrap_or(content.title.clone()))
                .hint(text),
        ),
    );

    label(
        document,
        tooltip,
        format!("{name}.title"),
        content.title,
        options.title_text_style,
        LayoutStyle::new().with_width_percent(1.0),
    );

    if let Some(body) = content.body {
        label(
            document,
            tooltip,
            format!("{name}.body"),
            body,
            options.body_text_style.clone(),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }

    if let Some(shortcut) = content.shortcut_label {
        label(
            document,
            tooltip,
            format!("{name}.shortcut"),
            shortcut,
            options.shortcut_text_style,
            LayoutStyle::new().with_width_percent(1.0),
        );
    }

    if let Some(reason) = content.disabled_reason {
        label(
            document,
            tooltip,
            format!("{name}.disabled_reason"),
            reason,
            options.body_text_style.clone(),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }

    tooltip
}

pub fn tooltip_box_from_request(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    request: &TooltipRequest,
    viewport: UiRect,
    tooltip_size: UiSize,
    cursor: Option<UiPoint>,
    options: TooltipBoxOptions,
) -> UiNodeId {
    let rect = tooltip_rect(
        request.anchor.rect,
        tooltip_size,
        viewport,
        request.placement,
        8.0,
        cursor,
    );
    tooltip_box(
        document,
        parent,
        name,
        request.content.clone(),
        options.at_rect(rect),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_rect_clamps_absolute_overlay_to_viewport() {
        let anchor = UiRect::new(260.0, 120.0, 40.0, 20.0);
        let rect = tooltip_rect(
            anchor,
            UiSize::new(120.0, 60.0),
            UiRect::new(0.0, 0.0, 300.0, 180.0),
            TooltipPlacement::Right,
            8.0,
            None,
        );

        assert_eq!(rect.x, 180.0);
        assert_eq!(rect.y, 120.0);
    }

    #[test]
    fn tooltip_box_builds_accessible_overlay_content() {
        let mut document = UiDocument::new(root_style(300.0, 180.0));
        let root = document.root;
        let tooltip = tooltip_box(
            &mut document,
            root,
            "save.tooltip",
            TooltipContent::new("Save")
                .body("Write changes to disk")
                .shortcut_label("Ctrl+S"),
            TooltipBoxOptions::default().at_rect(UiRect::new(16.0, 24.0, 180.0, 72.0)),
        );

        let node = document.node(tooltip);
        assert_eq!(node.style.z_index, 100);
        assert_eq!(node.layer, Some(crate::platform::UiLayer::AppOverlay));
        assert_eq!(
            node.accessibility.as_ref().unwrap().role,
            AccessibilityRole::Tooltip
        );
        assert_eq!(node.children.len(), 3);
    }
}
