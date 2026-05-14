use super::*;

#[derive(Debug, Clone)]
pub struct CollapsingHeaderOptions {
    pub layout: LayoutStyle,
    pub header_layout: LayoutStyle,
    pub body_layout: LayoutStyle,
    pub header_visual: UiVisual,
    pub hovered_visual: UiVisual,
    pub pressed_visual: UiVisual,
    pub body_visual: UiVisual,
    pub text_style: TextStyle,
    pub indicator_text_style: TextStyle,
    pub expanded: bool,
    pub enabled: bool,
    pub toggle_action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for CollapsingHeaderOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column().with_width_percent(1.0),
            header_layout: LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height(32.0)
                .with_padding(6.0)
                .with_gap(6.0)
                .with_align_items(AlignItems::Center),
            body_layout: LayoutStyle::column()
                .with_width_percent(1.0)
                .with_padding(8.0)
                .with_gap(8.0),
            header_visual: UiVisual::panel(ColorRgba::TRANSPARENT, None, 0.0),
            hovered_visual: UiVisual::panel(ColorRgba::new(38, 48, 61, 255), None, 4.0),
            pressed_visual: UiVisual::panel(ColorRgba::new(27, 36, 48, 255), None, 4.0),
            body_visual: UiVisual::TRANSPARENT,
            text_style: TextStyle::default(),
            indicator_text_style: TextStyle::default(),
            expanded: true,
            enabled: true,
            toggle_action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl CollapsingHeaderOptions {
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn with_toggle_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.toggle_action = Some(action.into());
        self
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollapsingHeaderNodes {
    pub root: UiNodeId,
    pub header: UiNodeId,
    pub indicator: UiNodeId,
    pub label: UiNodeId,
    pub body: Option<UiNodeId>,
}

pub fn collapsing_header(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    options: CollapsingHeaderOptions,
) -> CollapsingHeaderNodes {
    let name = name.into();
    let label_text = label_text.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::TRANSPARENT)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(
                options
                    .accessibility_label
                    .clone()
                    .unwrap_or_else(|| label_text.clone()),
            ),
        ),
    );

    let mut header_accessibility = AccessibilityMeta::new(AccessibilityRole::Button)
        .label(
            options
                .accessibility_label
                .clone()
                .unwrap_or_else(|| label_text.clone()),
        )
        .expanded(options.expanded)
        .action(AccessibilityAction::new("toggle", "Toggle"));
    if options.enabled {
        header_accessibility = header_accessibility.focusable();
    } else {
        header_accessibility = header_accessibility.disabled();
    }
    if let Some(hint) = options.accessibility_hint.clone() {
        header_accessibility = header_accessibility.hint(hint);
    }

    let header_visuals = InteractionVisuals::new(options.header_visual)
        .hovered(options.hovered_visual)
        .pressed(options.pressed_visual)
        .pressed_hovered(options.pressed_visual)
        .disabled(options.header_visual);
    let mut header_node = UiNode::container(
        format!("{name}.header"),
        UiNodeStyle {
            layout: options.header_layout.style.clone(),
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_interaction_visuals(header_visuals)
    .with_accessibility(header_accessibility);
    if options.enabled {
        header_node = header_node.with_input(InputBehavior::BUTTON);
    }
    if let Some(action) = options.toggle_action.clone() {
        header_node = header_node.with_action(action);
    }
    let header = document.add_child(root, header_node);

    let indicator = document.add_child(
        header,
        UiNode::text(
            format!("{name}.indicator"),
            if options.expanded { "v" } else { ">" },
            options.indicator_text_style.clone(),
            LayoutStyle::new().with_width(18.0).with_flex_shrink(0.0),
        )
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).hidden()),
    );

    let label = document.add_child(
        header,
        UiNode::text(
            format!("{name}.label"),
            label_text,
            options.text_style.clone(),
            LayoutStyle::new().with_width_percent(1.0),
        ),
    );

    let body = options.expanded.then(|| {
        document.add_child(
            root,
            UiNode::container(
                format!("{name}.body"),
                UiNodeStyle {
                    layout: options.body_layout.style.clone(),
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_visual(options.body_visual)
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).label("Content")),
        )
    });

    if let Some(body) = body {
        if let Some(accessibility) = document.node_mut(header).accessibility.as_mut() {
            accessibility.relations.controls.push(body);
        }
    }

    CollapsingHeaderNodes {
        root,
        header,
        indicator,
        label,
        body,
    }
}

pub fn collapsing_header_actions_from_input_result(
    document: &UiDocument,
    nodes: CollapsingHeaderNodes,
    options: &CollapsingHeaderOptions,
    result: &UiInputResult,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    let Some(clicked) = result.clicked else {
        return queue;
    };
    if !document.node_is_descendant_or_self(nodes.header, clicked)
        || !action_target_enabled(document, nodes.header)
    {
        return queue;
    }
    if let Some(binding) = options.toggle_action.clone() {
        if options.expanded {
            queue.close(nodes.header, binding);
        } else {
            queue.open(nodes.header, binding);
        }
    }
    queue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsing_header_expanded_creates_controlled_body() {
        let mut document = UiDocument::new(root_style(320.0, 180.0));
        let root = document.root;
        let nodes = collapsing_header(
            &mut document,
            root,
            "advanced",
            "Advanced",
            CollapsingHeaderOptions::default().with_toggle_action("toggle.advanced"),
        );

        let body = nodes.body.expect("expanded header body");
        let header_accessibility = document.node(nodes.header).accessibility.as_ref().unwrap();
        assert_eq!(header_accessibility.expanded, Some(true));
        assert_eq!(header_accessibility.relations.controls, vec![body]);
        assert!(document.node(nodes.header).input.pointer);
    }

    #[test]
    fn collapsing_header_collapsed_omits_body_and_queues_open() {
        let mut document = UiDocument::new(root_style(320.0, 180.0));
        let root = document.root;
        let options = CollapsingHeaderOptions::default()
            .expanded(false)
            .with_toggle_action("toggle.advanced");
        let nodes = collapsing_header(&mut document, root, "advanced", "Advanced", options.clone());
        document
            .compute_layout(UiSize::new(320.0, 180.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert!(nodes.body.is_none());
        let result = document.handle_input(UiInputEvent::PointerDown(UiPoint::new(8.0, 8.0)));
        assert_eq!(result.pressed, Some(nodes.header));
        let result = document.handle_input(UiInputEvent::PointerUp(UiPoint::new(8.0, 8.0)));
        let actions =
            collapsing_header_actions_from_input_result(&document, nodes, &options, &result);
        assert!(matches!(
            actions.as_slice().first().map(|action| &action.kind),
            Some(WidgetActionKind::Open)
        ));
    }
}
