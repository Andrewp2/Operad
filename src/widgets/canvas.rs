use super::*;

#[derive(Debug, Clone)]
pub struct CanvasOptions {
    pub layout: LayoutStyle,
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
    content: CanvasContent,
    options: CanvasOptions,
) -> UiNodeId {
    let name = name.into();
    let label = options
        .accessibility_label
        .clone()
        .unwrap_or_else(|| name.clone());
    let mut node = UiNode::canvas(name, content.key.clone(), options.layout)
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
