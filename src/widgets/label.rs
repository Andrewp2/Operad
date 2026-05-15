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

#[derive(Debug, Clone)]
pub struct LinkOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub hovered_visual: UiVisual,
    pub focused_visual: UiVisual,
    pub disabled_visual: UiVisual,
    pub text_style: TextStyle,
    pub visited_text_style: TextStyle,
    pub enabled: bool,
    pub visited: bool,
    pub url: Option<String>,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for LinkOptions {
    fn default() -> Self {
        let mut layout = LayoutStyle::column()
            .with_align_items(AlignItems::FlexStart)
            .with_padding(0.0);
        layout.as_taffy_style_mut().align_self = Some(AlignItems::FlexStart);
        let text_style = TextStyle {
            color: ColorRgba::new(118, 183, 255, 255),
            wrap: TextWrap::None,
            ..Default::default()
        };
        let visited_text_style = TextStyle {
            color: ColorRgba::new(176, 133, 255, 255),
            wrap: TextWrap::None,
            ..Default::default()
        };
        Self {
            layout,
            visual: UiVisual::TRANSPARENT,
            hovered_visual: UiVisual::TRANSPARENT,
            focused_visual: UiVisual::panel(
                ColorRgba::TRANSPARENT,
                Some(StrokeStyle::new(ColorRgba::new(112, 170, 230, 255), 1.0)),
                3.0,
            ),
            disabled_visual: UiVisual::TRANSPARENT,
            text_style,
            visited_text_style,
            enabled: true,
            visited: false,
            url: None,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl LinkOptions {
    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_command(mut self, command: impl Into<CommandId>) -> Self {
        self.action = Some(WidgetActionBinding::command(command));
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub const fn visited(mut self, visited: bool) -> Self {
        self.visited = visited;
        self
    }
}

pub fn link(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    options: LinkOptions,
) -> UiNodeId {
    let name = name.into();
    let text = text.into();
    let normal_text_style = if options.visited {
        options.visited_text_style.clone()
    } else {
        options.text_style.clone()
    };
    let mut hovered_text_style = normal_text_style.clone();
    hovered_text_style.underline = true;
    let hint = options
        .accessibility_hint
        .clone()
        .or_else(|| options.url.as_ref().map(|url| format!("Opens {url}")));
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Link)
        .label(
            options
                .accessibility_label
                .clone()
                .unwrap_or_else(|| text.clone()),
        )
        .action(AccessibilityAction::new("activate", "Open"));
    if let Some(url) = options.url.clone() {
        accessibility = accessibility.value(url);
    }
    if let Some(hint) = hint {
        accessibility = accessibility.hint(hint);
    }
    if options.enabled {
        accessibility = accessibility.focusable();
    } else {
        accessibility = accessibility.disabled();
    }

    let interaction_visuals = InteractionVisuals::new(options.visual)
        .hovered(options.hovered_visual)
        .focused(options.focused_visual)
        .pressed(options.hovered_visual)
        .disabled(options.disabled_visual);
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style.clone(),
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_interaction_visuals(interaction_visuals)
    .with_accessibility(accessibility);
    if options.enabled {
        node = node.with_input(InputBehavior::BUTTON);
    }
    if let Some(action) = options.action.clone() {
        node = node.with_action(action);
    }
    let link = document.add_child(parent, node);
    let mut label_node = UiNode::text(
        format!("{name}.label"),
        text,
        normal_text_style.clone(),
        LayoutStyle::new(),
    )
    .with_interaction_text_styles(
        TextInteractionStyles::new(normal_text_style)
            .hovered(hovered_text_style.clone())
            .pressed(hovered_text_style.clone())
            .pressed_hovered(hovered_text_style),
    );
    if options.enabled {
        label_node = label_node.with_input(InputBehavior::BUTTON);
        if let Some(action) = options.action.clone() {
            label_node = label_node.with_action(action);
        }
    }
    document.add_child(link, label_node);
    link
}

pub fn hyperlink(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    url: impl Into<String>,
    options: LinkOptions,
) -> UiNodeId {
    link(document, parent, name, text, options.with_url(url))
}

pub fn link_actions_from_input_result(
    document: &UiDocument,
    link: UiNodeId,
    options: &LinkOptions,
    result: &UiInputResult,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    let Some(clicked) = result.clicked else {
        return queue;
    };
    if !document.node_is_descendant_or_self(link, clicked) || !action_target_enabled(document, link)
    {
        return queue;
    }
    if let Some(binding) = options.action.clone() {
        queue.push(WidgetAction::pointer_activate(link, binding, 1));
    }
    queue
}

#[derive(Debug, Clone)]
pub struct SelectableLabelOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub hovered_visual: UiVisual,
    pub selected_visual: UiVisual,
    pub selected_hovered_visual: UiVisual,
    pub focused_visual: UiVisual,
    pub disabled_visual: UiVisual,
    pub text_style: TextStyle,
    pub selected: bool,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for SelectableLabelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::row()
                .with_align_items(AlignItems::Center)
                .with_padding(6.0),
            visual: UiVisual::TRANSPARENT,
            hovered_visual: UiVisual::panel(ColorRgba::new(39, 51, 66, 255), None, 4.0),
            selected_visual: UiVisual::panel(ColorRgba::new(54, 84, 123, 255), None, 4.0),
            selected_hovered_visual: UiVisual::panel(ColorRgba::new(68, 102, 148, 255), None, 4.0),
            focused_visual: UiVisual::panel(
                ColorRgba::TRANSPARENT,
                Some(StrokeStyle::new(ColorRgba::new(112, 170, 230, 255), 1.0)),
                4.0,
            ),
            disabled_visual: UiVisual::panel(ColorRgba::TRANSPARENT, None, 4.0),
            text_style: TextStyle::default(),
            selected: false,
            enabled: true,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl SelectableLabelOptions {
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }
}

pub fn selectable_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    options: SelectableLabelOptions,
) -> UiNodeId {
    let name = name.into();
    let text = text.into();
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::ToggleButton)
        .label(
            options
                .accessibility_label
                .clone()
                .unwrap_or_else(|| text.clone()),
        )
        .pressed(options.selected)
        .selected(options.selected)
        .action(AccessibilityAction::new("select", "Select"));
    if options.enabled {
        accessibility = accessibility.focusable();
    } else {
        accessibility = accessibility.disabled();
    }
    if let Some(hint) = options.accessibility_hint.clone() {
        accessibility = accessibility.hint(hint);
    }
    let base_visual = if options.selected {
        options.selected_visual
    } else {
        options.visual
    };
    let interaction_visuals = InteractionVisuals::new(base_visual)
        .hovered(if options.selected {
            options.selected_hovered_visual
        } else {
            options.hovered_visual
        })
        .pressed(options.selected_hovered_visual)
        .pressed_hovered(options.selected_hovered_visual)
        .focused(options.focused_visual)
        .disabled(options.disabled_visual);
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style.clone(),
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_interaction_visuals(interaction_visuals)
    .with_accessibility(accessibility);
    if options.enabled {
        node = node.with_input(InputBehavior::BUTTON);
    }
    if let Some(action) = options.action.clone() {
        node = node.with_action(action);
    }
    let label_node = document.add_child(parent, node);
    document.add_child(
        label_node,
        UiNode::text(
            format!("{name}.label"),
            text,
            options.text_style,
            LayoutStyle::new(),
        ),
    );
    label_node
}

pub fn selectable_value<T: PartialEq>(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    current: &T,
    value: &T,
    text: impl Into<String>,
    options: SelectableLabelOptions,
) -> UiNodeId {
    selectable_label(
        document,
        parent,
        name,
        text,
        SelectableLabelOptions {
            selected: current == value,
            ..options
        },
    )
}

pub fn selectable_label_actions_from_input_result(
    document: &UiDocument,
    label: UiNodeId,
    options: &SelectableLabelOptions,
    result: &UiInputResult,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    let Some(clicked) = result.clicked else {
        return queue;
    };
    if !document.node_is_descendant_or_self(label, clicked)
        || !action_target_enabled(document, label)
    {
        return queue;
    }
    if let Some(binding) = options.action.clone() {
        queue.select(label, binding, !options.selected);
    }
    queue
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

    #[test]
    fn link_builds_focusable_accessible_link() {
        let mut document = UiDocument::new(root_style(320.0, 120.0));
        let root = document.root;
        let options = LinkOptions::default()
            .with_layout(
                LayoutStyle::column()
                    .with_width(160.0)
                    .with_align_items(AlignItems::FlexStart),
            )
            .with_url("https://example.test")
            .with_action("open.example");
        let link = link(&mut document, root, "docs", "Docs", options.clone());
        document
            .compute_layout(UiSize::new(320.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let node = document.node(link);
        let accessibility = node.accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::Link);
        assert_eq!(accessibility.value.as_deref(), Some("https://example.test"));
        assert!(node.input.pointer);
        let children = document.node(link).children.clone();
        assert_eq!(children.len(), 1);
        let label_rect = document.node(children[0]).layout.rect;
        assert!(label_rect.width < document.node(link).layout.rect.width);
        let UiContent::Text(label_text) = &document.node(children[0]).content else {
            panic!("link label should be text");
        };
        assert!(!label_text.style.underline);

        document.handle_input(UiInputEvent::PointerMove(UiPoint::new(3.0, 3.0)));
        let UiContent::Text(label_text) = &document.node(children[0]).content else {
            panic!("link label should be text");
        };
        assert!(label_text.style.underline);
        let underlined_text = document
            .paint_list()
            .items
            .into_iter()
            .find(|item| item.node == children[0] && matches!(item.kind, PaintKind::Text(_)))
            .expect("underlined text should paint");
        assert_eq!(underlined_text.rect.width, label_rect.width);
        assert!(underlined_text.rect.width < document.node(link).layout.rect.width);
        let underline = document
            .paint_list()
            .items
            .into_iter()
            .find(|item| item.node == children[0] && matches!(item.kind, PaintKind::Line { .. }))
            .expect("hovered link should paint a text-width underline");
        let PaintKind::Line { from, to, .. } = underline.kind else {
            panic!("underline should be a line");
        };
        assert_eq!(from.x, label_rect.x);
        assert!(((to.x - from.x) - label_rect.width).abs() < 0.25);
        assert!(to.x - from.x < document.node(link).layout.rect.width);

        document.handle_input(UiInputEvent::PointerDown(UiPoint::new(3.0, 3.0)));
        let result = document.handle_input(UiInputEvent::PointerUp(UiPoint::new(3.0, 3.0)));
        let actions = link_actions_from_input_result(&document, link, &options, &result);
        assert!(matches!(
            actions.as_slice().first().map(|action| &action.kind),
            Some(WidgetActionKind::Activate(_))
        ));
    }

    #[test]
    fn selectable_label_reports_selected_state_and_toggle_action() {
        let mut document = UiDocument::new(root_style(320.0, 120.0));
        let root = document.root;
        let options = SelectableLabelOptions::default()
            .selected(true)
            .with_action("select.preview");
        let label = selectable_label(&mut document, root, "preview", "Preview", options.clone());
        document
            .compute_layout(UiSize::new(320.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let node = document.node(label);
        let accessibility = node.accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::ToggleButton);
        assert_eq!(accessibility.selected, Some(true));
        assert_eq!(accessibility.pressed, Some(true));
        assert_eq!(node.visual, options.selected_visual);

        document.handle_input(UiInputEvent::PointerDown(UiPoint::new(3.0, 3.0)));
        let result = document.handle_input(UiInputEvent::PointerUp(UiPoint::new(3.0, 3.0)));
        let actions =
            selectable_label_actions_from_input_result(&document, label, &options, &result);
        assert!(matches!(
            actions.as_slice().first().map(|action| &action.kind),
            Some(WidgetActionKind::Selection(WidgetSelection {
                selected: Some(false)
            }))
        ));
    }

    #[test]
    fn selectable_value_sets_selected_state_from_current_value() {
        let mut document = UiDocument::new(root_style(320.0, 120.0));
        let root = document.root;
        let compact = selectable_value(
            &mut document,
            root,
            "density.compact",
            &"compact",
            &"compact",
            "Compact",
            SelectableLabelOptions::default().with_action("density.compact"),
        );
        let comfortable = selectable_value(
            &mut document,
            root,
            "density.comfortable",
            &"compact",
            &"comfortable",
            "Comfortable",
            SelectableLabelOptions::default().with_action("density.comfortable"),
        );

        assert_eq!(
            document
                .node(compact)
                .accessibility
                .as_ref()
                .unwrap()
                .selected,
            Some(true)
        );
        assert_eq!(
            document
                .node(comfortable)
                .accessibility
                .as_ref()
                .unwrap()
                .selected,
            Some(false)
        );
        assert_eq!(
            document.node(compact).action.as_ref(),
            Some(&WidgetActionBinding::action("density.compact"))
        );
    }
}
