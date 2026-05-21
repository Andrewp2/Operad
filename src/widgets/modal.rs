use super::ext::dialog::DialogDescriptor;
use super::ext::popover::OverlayFrameEvent;
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
    pub trap_focus: bool,
    pub focus_restore: FocusRestoreTarget,
    pub focus_wrap: bool,
    pub portal: UiPortalTarget,
    pub dismissal: DialogDismissal,
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
                inset: taffy::prelude::Rect::length(0.0_f32),
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
            trap_focus: true,
            focus_restore: FocusRestoreTarget::Previous,
            focus_wrap: true,
            portal: UiPortalTarget::GlobalAppOverlay,
            dismissal: DialogDismissal::MODAL,
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

    pub const fn with_dismissal(mut self, dismissal: DialogDismissal) -> Self {
        self.dismissal = dismissal;
        self
    }

    pub const fn with_focus_trap(mut self, trap_focus: bool) -> Self {
        self.trap_focus = trap_focus;
        self
    }

    pub const fn with_focus_restore(mut self, restore: FocusRestoreTarget) -> Self {
        self.focus_restore = restore;
        self
    }

    pub const fn with_focus_wrap(mut self, wrap: bool) -> Self {
        self.focus_wrap = wrap;
        self
    }

    pub fn with_portal(mut self, portal: UiPortalTarget) -> Self {
        self.portal = portal;
        self
    }

    pub const fn without_close_button(mut self) -> Self {
        self.show_close_button = false;
        self
    }

    pub fn modeless(mut self) -> Self {
        self.modal = false;
        self.trap_focus = false;
        self.dismissal = DialogDismissal::STANDARD;
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
    let mut accessibility = modal_dialog_descriptor(&name, &title_text, &options).accessibility();
    if let Some(label) = options.accessibility_label.clone() {
        accessibility.label = Some(label);
    }
    if let Some(hint) = options.accessibility_hint.clone() {
        accessibility.hint = Some(hint);
    }
    let overlay = document.add_portal_child(
        parent,
        options.portal.clone(),
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
        .with_clip_scope(ClipScope::Viewport)
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).hidden()),
    );

    let scrim_z_index = options.z_index.saturating_sub(1);
    let dialog_z_index = options.z_index;

    let mut scrim = UiNode::container(
        format!("{name}.scrim"),
        UiNodeStyle {
            layout: LayoutStyle::absolute_rect(UiRect::new(0.0, 0.0, 0.0, 0.0)).style,
            clip: ClipBehavior::Clip,
            z_index: scrim_z_index,
            ..Default::default()
        },
    )
    .with_visual(options.scrim_visual)
    .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).hidden());
    scrim.style.layout.inset = taffy::prelude::Rect::length(0.0_f32);
    scrim.style.layout.size = TaffySize {
        width: Dimension::percent(1.0),
        height: Dimension::percent(1.0),
    };
    let scrim = document.add_child(overlay, scrim);

    let dialog = document.add_child(
        overlay,
        UiNode::container(
            format!("{name}.dialog"),
            UiNodeStyle {
                layout: options.dialog_layout.style.clone(),
                clip: ClipBehavior::Clip,
                z_index: dialog_z_index,
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
        let close_enabled = options.dismissal.close_button;
        button(
            document,
            header,
            format!("{name}.close"),
            "x",
            ButtonOptions {
                layout: LayoutStyle::new().with_width(30.0).with_height(28.0),
                action: close_enabled
                    .then(|| options.close_action.clone())
                    .flatten(),
                enabled: close_enabled,
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

pub fn modal_dialog_descriptor(
    id: impl Into<String>,
    title: impl Into<String>,
    options: &ModalDialogOptions,
) -> DialogDescriptor {
    let mut descriptor = DialogDescriptor::new(id, title)
        .modal(options.modal)
        .trap_focus(options.trap_focus)
        .dismissal(options.dismissal);
    if let Some(hint) = options.accessibility_hint.clone() {
        descriptor = descriptor.accessibility_hint(hint);
    }
    descriptor
}

pub fn modal_dialog_focus_trap(
    nodes: ModalDialogNodes,
    options: &ModalDialogOptions,
) -> Option<FocusTrap> {
    options.trap_focus.then(|| {
        FocusTrap::new(nodes.dialog)
            .restore_focus(options.focus_restore)
            .wrap(options.focus_wrap)
    })
}

pub fn modal_dialog_open_event(
    id: impl Into<String>,
    title: impl Into<String>,
    nodes: ModalDialogNodes,
    options: &ModalDialogOptions,
) -> OverlayFrameEvent {
    let descriptor = modal_dialog_descriptor(id, title, options);
    match modal_dialog_focus_trap(nodes, options) {
        Some(focus_trap) => OverlayFrameEvent::open_dialog_with_focus_trap(descriptor, focus_trap),
        None => OverlayFrameEvent::open_dialog(descriptor),
    }
}

pub fn modal_dialog_dismiss_event_from_input_result(
    document: &UiDocument,
    nodes: ModalDialogNodes,
    options: &ModalDialogOptions,
    result: &UiInputResult,
) -> Option<OverlayFrameEvent> {
    let clicked = result.clicked?;
    if nodes
        .close_button
        .is_some_and(|close| document.node_is_descendant_or_self(close, clicked))
    {
        return options
            .dismissal
            .allows(DialogDismissReason::CloseButton)
            .then_some(OverlayFrameEvent::dismiss_dialog(
                DialogDismissReason::CloseButton,
            ));
    }
    (!document.node_is_descendant_or_self(nodes.dialog, clicked)
        && options
            .dismissal
            .allows(DialogDismissReason::OutsidePointer))
    .then_some(OverlayFrameEvent::dismiss_dialog(
        DialogDismissReason::OutsidePointer,
    ))
}

pub fn modal_dialog_dismiss_event_from_pointer_event(
    document: &UiDocument,
    nodes: ModalDialogNodes,
    options: &ModalDialogOptions,
    event: &UiInputEvent,
) -> Option<OverlayFrameEvent> {
    let point = match event {
        UiInputEvent::PointerDown(point) | UiInputEvent::PointerUp(point) => *point,
        _ => return None,
    };
    if !options
        .dismissal
        .allows(DialogDismissReason::OutsidePointer)
    {
        return None;
    }
    let hit = document.hit_test(point);
    hit.is_none_or(|node| !document.node_is_descendant_or_self(nodes.dialog, node))
        .then_some(OverlayFrameEvent::dismiss_dialog(
            DialogDismissReason::OutsidePointer,
        ))
}

pub fn modal_dialog_dismiss_event_from_key_event(
    options: &ModalDialogOptions,
    event: &UiInputEvent,
) -> Option<OverlayFrameEvent> {
    matches!(
        event,
        UiInputEvent::Key {
            key: KeyCode::Escape,
            modifiers: KeyModifiers::NONE,
        }
    )
    .then_some(DialogDismissReason::EscapeKey)
    .filter(|reason| options.dismissal.allows(*reason))
    .map(OverlayFrameEvent::dismiss_dialog)
}

pub fn modal_dialog_close_actions_from_input_result(
    document: &UiDocument,
    nodes: ModalDialogNodes,
    options: &ModalDialogOptions,
    result: &UiInputResult,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    if !options.dismissal.allows(DialogDismissReason::CloseButton) {
        return queue;
    }
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
        let options = ModalDialogOptions::default()
            .with_close_action("close.confirm")
            .with_focus_restore(FocusRestoreTarget::Node(root));
        let nodes = modal_dialog(
            &mut document,
            root,
            "confirm",
            "Confirm delete",
            options.clone(),
        );

        assert_eq!(
            document.node(nodes.overlay).layer,
            Some(crate::platform::UiLayer::AppOverlay)
        );
        let portal = document
            .portal_host(APP_OVERLAY_PORTAL)
            .expect("app overlay portal");
        assert_eq!(document.node(nodes.overlay).parent, Some(portal));
        assert_eq!(document.node(nodes.overlay).clip_scope, ClipScope::Viewport);
        assert_eq!(document.node(nodes.overlay).style.z_index, 200);
        assert_eq!(document.node(nodes.scrim).style.z_index, 199);
        assert_eq!(document.node(nodes.dialog).style.z_index, 200);
        let accessibility = document.node(nodes.dialog).accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::Dialog);
        assert!(accessibility.modal);
        assert!(nodes.close_button.is_some());
        let descriptor = modal_dialog_descriptor("confirm", "Confirm delete", &options);
        assert!(descriptor.modal);
        assert!(descriptor.trap_focus);
        assert_eq!(descriptor.dismissal, DialogDismissal::MODAL);
        let focus_trap = modal_dialog_focus_trap(nodes, &options).expect("focus trap");
        assert_eq!(focus_trap.root, nodes.dialog);
        assert_eq!(focus_trap.restore_focus, FocusRestoreTarget::Node(root));
        assert!(focus_trap.wrap);
        assert_eq!(
            modal_dialog_open_event("confirm", "Confirm delete", nodes, &options),
            OverlayFrameEvent::open_dialog_with_focus_trap(descriptor, focus_trap)
        );

        document
            .compute_layout(UiSize::new(640.0, 360.0), &mut ApproxTextMeasurer)
            .expect("modal layout");
        let paint = document.paint_list();
        let scrim_paint_index = paint
            .items
            .iter()
            .position(|item| {
                item.node == nodes.scrim && matches!(item.kind, PaintKind::Rect { .. })
            })
            .expect("scrim paint item");
        let dialog_paint_index = paint
            .items
            .iter()
            .position(|item| {
                item.node == nodes.dialog && matches!(item.kind, PaintKind::Rect { .. })
            })
            .expect("dialog paint item");
        assert!(
            scrim_paint_index < dialog_paint_index,
            "scrim must paint behind the modal dialog: {:#?}",
            paint.items
        );
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
        assert_eq!(
            modal_dialog_dismiss_event_from_input_result(&document, nodes, &options, &result),
            Some(OverlayFrameEvent::dismiss_dialog(
                DialogDismissReason::CloseButton
            ))
        );
    }

    #[test]
    fn modal_dismiss_helpers_respect_keyboard_pointer_and_policy() {
        let mut document = UiDocument::new(root_style(640.0, 360.0));
        let root = document.root;
        let modal_options = ModalDialogOptions::default();
        let modal_nodes =
            modal_dialog(&mut document, root, "modal", "Modal", modal_options.clone());
        document
            .compute_layout(UiSize::new(640.0, 360.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(
            modal_dialog_dismiss_event_from_key_event(
                &modal_options,
                &UiInputEvent::Key {
                    key: KeyCode::Escape,
                    modifiers: KeyModifiers::NONE,
                },
            ),
            Some(OverlayFrameEvent::dismiss_dialog(
                DialogDismissReason::EscapeKey
            ))
        );
        assert_eq!(
            modal_dialog_dismiss_event_from_pointer_event(
                &document,
                modal_nodes,
                &modal_options,
                &UiInputEvent::PointerUp(UiPoint::new(4.0, 4.0)),
            ),
            None
        );

        let modeless_options = ModalDialogOptions::default().modeless();
        let modeless_nodes = modal_dialog(
            &mut document,
            root,
            "modeless",
            "Modeless",
            modeless_options.clone(),
        );
        document
            .compute_layout(UiSize::new(640.0, 360.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert!(modal_dialog_focus_trap(modeless_nodes, &modeless_options).is_none());
        assert_eq!(
            modal_dialog_dismiss_event_from_pointer_event(
                &document,
                modeless_nodes,
                &modeless_options,
                &UiInputEvent::PointerUp(UiPoint::new(4.0, 4.0)),
            ),
            Some(OverlayFrameEvent::dismiss_dialog(
                DialogDismissReason::OutsidePointer
            ))
        );
    }

    #[test]
    fn modal_close_button_respects_dismissal_policy() {
        let mut document = UiDocument::new(root_style(640.0, 360.0));
        let root = document.root;
        let options = ModalDialogOptions::default()
            .with_close_action("close.confirm")
            .with_dismissal(DialogDismissal::NONE);
        let nodes = modal_dialog(&mut document, root, "locked", "Locked", options.clone());
        document
            .compute_layout(UiSize::new(640.0, 360.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let close = nodes.close_button.expect("close button");
        assert!(!document.node(close).input.pointer);
        assert!(!document.node(close).input.focusable);
        assert!(!document.node(close).input.keyboard);
        let close_rect = document.node(close).layout.rect;
        document.handle_input(UiInputEvent::PointerDown(UiPoint::new(
            close_rect.x + 2.0,
            close_rect.y + 2.0,
        )));
        let result = document.handle_input(UiInputEvent::PointerUp(UiPoint::new(
            close_rect.x + 2.0,
            close_rect.y + 2.0,
        )));

        assert!(
            modal_dialog_close_actions_from_input_result(&document, nodes, &options, &result)
                .as_slice()
                .is_empty()
        );
        assert_eq!(
            modal_dialog_dismiss_event_from_input_result(&document, nodes, &options, &result),
            None
        );
    }
}
