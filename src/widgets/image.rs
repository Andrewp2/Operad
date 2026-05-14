use super::*;

#[derive(Debug, Clone)]
pub struct ImageOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub shader: Option<ShaderEffect>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::size(64.0, 64.0),
            visual: UiVisual::TRANSPARENT,
            shader: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl ImageOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn with_accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    pub fn with_shader(mut self, shader: impl Into<ShaderEffect>) -> Self {
        self.shader = Some(shader.into());
        self
    }
}

pub fn image(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    content: ImageContent,
    options: ImageOptions,
) -> UiNodeId {
    let name = name.into();
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Image).label(
        options
            .accessibility_label
            .clone()
            .unwrap_or_else(|| name.clone()),
    );
    if let Some(hint) = options.accessibility_hint {
        accessibility = accessibility.hint(hint);
    }
    let mut node = UiNode::image(name, content, options.layout)
        .with_visual(options.visual)
        .with_accessibility(accessibility);
    if let Some(shader) = options.shader {
        node = node.with_shader(shader);
    }
    document.add_child(parent, node)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_builder_wraps_image_content_with_accessibility() {
        let mut document = UiDocument::new(root_style(160.0, 120.0));
        let root = document.root;
        let node = image(
            &mut document,
            root,
            "logo",
            ImageContent::new("assets.logo"),
            ImageOptions::default().with_accessibility_label("Logo"),
        );

        assert!(matches!(
            &document.node(node).content,
            UiContent::Image(image) if image.key == "assets.logo"
        ));
        let accessibility = document.node(node).accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::Image);
        assert_eq!(accessibility.label.as_deref(), Some("Logo"));
    }
}
