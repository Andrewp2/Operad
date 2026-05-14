use super::*;

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
