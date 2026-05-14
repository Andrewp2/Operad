use super::*;

pub fn colored_text_style(color: ColorRgba) -> TextStyle {
    TextStyle {
        color,
        ..Default::default()
    }
}

pub fn heading_text_style() -> TextStyle {
    TextStyle {
        font_size: 24.0,
        line_height: 30.0,
        weight: FontWeight::BOLD,
        ..Default::default()
    }
}

pub fn strong_text_style() -> TextStyle {
    TextStyle {
        weight: FontWeight::BOLD,
        ..Default::default()
    }
}

pub fn weak_text_style() -> TextStyle {
    TextStyle {
        color: ColorRgba::new(166, 178, 196, 255),
        ..Default::default()
    }
}

pub fn small_text_style() -> TextStyle {
    TextStyle {
        font_size: 13.0,
        line_height: 17.0,
        ..Default::default()
    }
}

pub fn monospace_text_style() -> TextStyle {
    TextStyle {
        family: FontFamily::Monospace,
        ..Default::default()
    }
}

pub fn code_text_style() -> TextStyle {
    TextStyle {
        family: FontFamily::Monospace,
        font_size: 14.0,
        line_height: 18.0,
        color: ColorRgba::new(214, 225, 240, 255),
        wrap: TextWrap::None,
        ..Default::default()
    }
}

pub fn label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    style: TextStyle,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    let layout = layout.into();
    let text = text.into();
    document.add_child(
        parent,
        UiNode::text(name, text.clone(), style, layout)
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label(text)),
    )
}

pub fn heading_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    label(document, parent, name, text, heading_text_style(), layout)
}

pub fn strong_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    label(document, parent, name, text, strong_text_style(), layout)
}

pub fn weak_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    label(document, parent, name, text, weak_text_style(), layout)
}

pub fn small_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    label(document, parent, name, text, small_text_style(), layout)
}

pub fn monospace_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    label(document, parent, name, text, monospace_text_style(), layout)
}

pub fn code_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    label(document, parent, name, text, code_text_style(), layout)
}

pub fn colored_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    color: ColorRgba,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    label(
        document,
        parent,
        name,
        text,
        colored_text_style(color),
        layout,
    )
}

pub fn wrapped_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    wrap: TextWrap,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    let mut style = TextStyle::default();
    style.wrap = wrap;
    label(document, parent, name, text, style, layout)
}

pub fn localized_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: DynamicLabelMeta,
    policy: Option<&LocalizationPolicy>,
    style: TextStyle,
    layout: impl Into<LayoutStyle>,
) -> UiNodeId {
    let text = label.fallback.clone();
    document.add_child(
        parent,
        UiNode::localized_text(name, label, policy, style, layout)
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label(text)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_style_helpers_set_expected_text_metadata() {
        let mut document = UiDocument::new(root_style(320.0, 120.0));
        let root = document.root;
        let heading = heading_label(
            &mut document,
            root,
            "heading",
            "Heading",
            LayoutStyle::new(),
        );
        let code = code_label(
            &mut document,
            root,
            "code",
            "let x = 1;",
            LayoutStyle::new(),
        );

        let UiContent::Text(heading_text) = &document.node(heading).content else {
            panic!("heading should be text");
        };
        assert_eq!(heading_text.style.weight, FontWeight::BOLD);
        assert_eq!(heading_text.style.font_size, 24.0);

        let UiContent::Text(code_text) = &document.node(code).content else {
            panic!("code should be text");
        };
        assert_eq!(code_text.style.family, FontFamily::Monospace);
        assert_eq!(code_text.style.wrap, TextWrap::None);
    }
}
