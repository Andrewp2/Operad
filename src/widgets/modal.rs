use super::*;

#[derive(Debug, Clone)]
pub struct ModalDialogOptions {
    pub overlay_layout: LayoutStyle,
    pub dialog_layout: LayoutStyle,
    pub body_layout: LayoutStyle,
    pub scrim_visual: UiVisual,
    pub dialog_visual: UiVisual,
    pub title_text_style: TextStyle,
    pub body_visual: UiVisual,
    pub z_index: i16,
    pub modal: bool,
    pub show_close_button: bool,
    pub close_action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for ModalDialogOptions {
    fn default() -> Self {
        Self {
            overlay_layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                position: taffy::prelude::Position::Absolute,
                inset: taffy::prelude::Rect::length(0.0),
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            }),
            dialog_layout: LayoutStyle::column()
                .with_width(420.0)
                .with_height(260.0)
                .with_padding(0.0),
            body_layout: LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0)
                .with_padding(12.0)
                .with_gap(8.0),
            scrim_visual: UiVisual::panel(ColorRgba::new(0, 0, 0, 140), None, 0.0),
            dialog_visual: UiVisual::panel(
                ColorRgba::new(22, 27, 34, 255),
                Some(StrokeStyle::new(ColorRgba::new(78, 91, 112, 255), 1.0)),
                6.0,
            ),
            title_text_style: TextStyle {
                font_size: 18.0,
                line_height: 24.0,
                weight: FontWeight::BOLD,
                ..Default::default()
            },
            body_visual: UiVisual::TRANSPARENT,
            z_index: 200,
            modal: true,
            show_close_button: true,
            close_action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl ModalDialogOptions {
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.dialog_layout = self.dialog_layout.with_width(width).with_height(height);
        self
    }

    pub fn with_close_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.close_action = Some(action.into());
        self
    }

    pub fn modeless(mut self) -> Self {
        self.modal = false;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModalDialogNodes {
    pub overlay: UiNodeId,
    pub scrim: UiNodeId,
    pub dialog: UiNodeId,
    pub header: UiNodeId,
    pub title: UiNodeId,
    pub close_button: Option<UiNodeId>,
    pub body: UiNodeId,
}

pub fn modal_dialog(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    title_text: impl Into<String>,
    options: ModalDialogOptions,
) -> ModalDialogNodes {
    let name = name.into();
    let title_text = title_text.into();
    let overlay = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.overlay_layout.style.clone(),
                clip: ClipBehavior::Clip,
                z_index: options.z_index,
                ..Default::default()
            },
        )
        .with_layer(crate::platform::UiLayer::AppOverlay)
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).hidden()),
    );

    let mut scrim = UiNode::container(
        format!("{name}.scrim"),
        UiNodeStyle {
            layout: LayoutStyle::absolute_rect(UiRect::new(0.0, 0.0, 0.0, 0.0)).style,
            clip: ClipBehavior::Clip,
            z_index: 0,
            ..Default::default()
        },
    )
    .with_visual(options.scrim_visual)
    .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).hidden());
    scrim.style.layout.inset = taffy::prelude::Rect::length(0.0);
    scrim.style.layout.size = TaffySize {
        width: Dimension::percent(1.0),
        height: Dimension::percent(1.0),
    };
    let scrim = document.add_child(overlay, scrim);

    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Dialog)
        .label(
            options
                .accessibility_label
                .clone()
                .unwrap_or_else(|| title_text.clone()),
        )
        .focusable();
    if options.modal {
        accessibility = accessibility.modal();
    }
    if let Some(hint) = options.accessibility_hint.clone() {
        accessibility = accessibility.hint(hint);
    }
    let dialog = document.add_child(
        overlay,
        UiNode::container(
            format!("{name}.dialog"),
            UiNodeStyle {
                layout: options.dialog_layout.style.clone(),
                clip: ClipBehavior::Clip,
                z_index: 1,
                ..Default::default()
            },
        )
        .with_visual(options.dialog_visual)
        .with_accessibility(accessibility),
    );

    let header = document.add_child(
        dialog,
        UiNode::container(
            format!("{name}.header"),
            UiNodeStyle {
                layout: LayoutStyle::row()
                    .with_width_percent(1.0)
                    .with_height(44.0)
                    .with_padding(10.0)
                    .with_gap(8.0)
                    .with_align_items(AlignItems::Center)
                    .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(18, 23, 31, 255),
            Some(StrokeStyle::new(ColorRgba::new(56, 68, 84, 255), 1.0)),
            0.0,
        )),
    );

    let title = label(
        document,
        header,
        format!("{name}.title"),
        title_text.clone(),
        options.title_text_style.clone(),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let close_button = options.show_close_button.then(|| {
        button(
            document,
            header,
            format!("{name}.close"),
            "x",
            ButtonOptions {
                layout: LayoutStyle::new().with_width(30.0).with_height(28.0),
                action: options.close_action.clone(),
                accessibility_label: Some(format!("Close {title_text}")),
                ..Default::default()
            },
        )
    });

    let body = document.add_child(
        dialog,
        UiNode::container(
            format!("{name}.body"),
            UiNodeStyle {
                layout: options.body_layout.style.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.body_visual)
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).label("Dialog body")),
    );

    ModalDialogNodes {
        overlay,
        scrim,
        dialog,
        header,
        title,
        close_button,
        body,
    }
}

pub fn modal_dialog_close_actions_from_input_result(
    document: &UiDocument,
    nodes: ModalDialogNodes,
    options: &ModalDialogOptions,
    result: &UiInputResult,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    let Some(close_button) = nodes.close_button else {
        return queue;
    };
    let Some(clicked) = result.clicked else {
        return queue;
    };
    if !document.node_is_descendant_or_self(close_button, clicked)
        || !action_target_enabled(document, close_button)
    {
        return queue;
    }
    if let Some(binding) = options.close_action.clone() {
        queue.close(nodes.dialog, binding);
    }
    queue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_dialog_builds_layered_modal_surface() {
        let mut document = UiDocument::new(root_style(640.0, 360.0));
        let root = document.root;
        let nodes = modal_dialog(
            &mut document,
            root,
            "confirm",
            "Confirm delete",
            ModalDialogOptions::default().with_close_action("close.confirm"),
        );

        assert_eq!(
            document.node(nodes.overlay).layer,
            Some(crate::platform::UiLayer::AppOverlay)
        );
        assert_eq!(document.node(nodes.overlay).style.z_index, 200);
        assert_eq!(document.node(nodes.scrim).style.z_index, 0);
        assert_eq!(document.node(nodes.dialog).style.z_index, 1);
        let accessibility = document.node(nodes.dialog).accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::Dialog);
        assert!(accessibility.modal);
        assert!(nodes.close_button.is_some());
    }

    #[test]
    fn modal_close_helper_queues_close_for_close_button() {
        let mut document = UiDocument::new(root_style(640.0, 360.0));
        let root = document.root;
        let options = ModalDialogOptions::default().with_close_action("close.confirm");
        let nodes = modal_dialog(&mut document, root, "confirm", "Confirm", options.clone());
        document
            .compute_layout(UiSize::new(640.0, 360.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let close_rect = document.node(nodes.close_button.unwrap()).layout.rect;
        document.handle_input(UiInputEvent::PointerDown(UiPoint::new(
            close_rect.x + 2.0,
            close_rect.y + 2.0,
        )));
        let result = document.handle_input(UiInputEvent::PointerUp(UiPoint::new(
            close_rect.x + 2.0,
            close_rect.y + 2.0,
        )));

        let actions =
            modal_dialog_close_actions_from_input_result(&document, nodes, &options, &result);
        assert!(matches!(
            actions.as_slice().first().map(|action| &action.kind),
            Some(WidgetActionKind::Close)
        ));
    }
}
