use super::*;

#[derive(Debug, Clone)]
pub struct CanvasOptions {
    pub layout: LayoutStyle,
    pub aspect_ratio: Option<f32>,
    pub intrinsic_size: Option<UiSize>,
    pub visual: UiVisual,
    pub input: InputBehavior,
    pub action: Option<WidgetActionBinding>,
    pub action_mode: WidgetActionMode,
    pub accessibility_label: Option<String>,
}

impl Default for CanvasOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height_percent(1.0)
                .with_flex_grow(1.0),
            aspect_ratio: None,
            intrinsic_size: Some(UiSize::new(320.0, 180.0)),
            visual: UiVisual::TRANSPARENT,
            input: InputBehavior {
                pointer: true,
                focusable: true,
                keyboard: true,
            },
            action: None,
            action_mode: WidgetActionMode::Activate,
            accessibility_label: None,
        }
    }
}

impl CanvasOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub const fn with_aspect_ratio(mut self, aspect_ratio: f32) -> Self {
        self.aspect_ratio = Some(aspect_ratio);
        self
    }

    pub const fn with_intrinsic_size(mut self, size: UiSize) -> Self {
        self.intrinsic_size = Some(size);
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }
}

pub fn canvas(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    mut content: CanvasContent,
    options: CanvasOptions,
) -> UiNodeId {
    let name = name.into();
    let label = options
        .accessibility_label
        .clone()
        .unwrap_or_else(|| name.clone());
    let mut layout = options.layout;
    if let Some(aspect_ratio) = options.aspect_ratio {
        if aspect_ratio.is_finite() && aspect_ratio > 0.0 {
            layout.as_taffy_style_mut().aspect_ratio = Some(aspect_ratio);
        }
    }
    if let Some(intrinsic_size) = options.intrinsic_size {
        content = content.intrinsic_size(intrinsic_size);
    }
    let mut node = UiNode::canvas(name, content.key.clone(), layout)
        .with_input(options.input)
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group)
                .label(label)
                .focusable(),
        );
    node.action = options.action;
    node.action_mode = options.action_mode;
    node.content = UiContent::Canvas(content);
    document.add_child(parent, node)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_options_can_preserve_aspect_ratio() {
        let mut document = UiDocument::new(root_style(240.0, 160.0));
        let canvas = canvas(
            &mut document,
            UiNodeId(0),
            "canvas.aspect",
            CanvasContent::new("canvas.aspect"),
            CanvasOptions::default()
                .with_layout(LayoutStyle::new().with_width(160.0))
                .with_aspect_ratio(2.0),
        );
        document
            .compute_layout(UiSize::new(240.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let rect = document.node(canvas).layout.rect;
        assert!((rect.width - 160.0).abs() < 0.01, "{rect:?}");
        assert!((rect.height - 80.0).abs() < 0.01, "{rect:?}");
    }

    #[test]
    fn canvas_aspect_ratio_fits_short_available_height() {
        let mut document = UiDocument::new(root_style(300.0, 120.0));
        let canvas = canvas(
            &mut document,
            UiNodeId(0),
            "canvas.short",
            CanvasContent::new("canvas.short"),
            CanvasOptions::default().with_aspect_ratio(16.0 / 9.0),
        );
        document
            .compute_layout(UiSize::new(300.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let rect = document.node(canvas).layout.rect;
        assert!((rect.width - 213.33).abs() < 0.02, "{rect:?}");
        assert!((rect.height - 120.0).abs() < 0.01, "{rect:?}");
        assert!((rect.x - 43.33).abs() < 0.02, "{rect:?}");
        assert!((rect.y - 0.0).abs() < 0.01, "{rect:?}");
    }

    #[test]
    fn canvas_aspect_ratio_fits_narrow_available_width() {
        let mut document = UiDocument::new(root_style(120.0, 300.0));
        let canvas = canvas(
            &mut document,
            UiNodeId(0),
            "canvas.narrow",
            CanvasContent::new("canvas.narrow"),
            CanvasOptions::default().with_aspect_ratio(16.0 / 9.0),
        );
        document
            .compute_layout(UiSize::new(120.0, 300.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let rect = document.node(canvas).layout.rect;
        assert!((rect.width - 120.0).abs() < 0.01, "{rect:?}");
        assert!((rect.height - 67.5).abs() < 0.01, "{rect:?}");
        assert!((rect.x - 0.0).abs() < 0.01, "{rect:?}");
        assert!((rect.y - 116.25).abs() < 0.01, "{rect:?}");
    }
}
