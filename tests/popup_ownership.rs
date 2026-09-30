#![cfg(feature = "widgets")]

use operad::input::{GestureEvent, PointerButton, PointerClick, PointerId};
use operad::layout::LayoutDisplay;
use operad::widgets::ext::{
    dropdown_select, menu_bar, menu_button, AnchoredPopup, DropdownSelectOptions, MenuBarAnchors,
    MenuBarMenu, MenuBarOptions, MenuBarState, MenuButtonAnchors, MenuButtonOptions,
    MenuButtonState, MenuItem, MenuNavigationState, PopupAlign, PopupPlacement, PopupSide,
    SelectMenuState, SelectOption,
};
use operad::{
    root_style, ApproxTextMeasurer, KeyModifiers, LayoutStyle, UiDocument, UiInputEvent, UiNode,
    UiNodeId, UiPoint, UiPortalTarget, UiRect, UiSize, WidgetAction,
};

const VIEWPORT: UiSize = UiSize::new(1600.0, 900.0);

#[derive(Clone, Copy, Debug)]
enum Control {
    Button,
    Bar,
    Dropdown,
    Submenu,
}

struct Fixture {
    document: UiDocument,
    trigger: UiNodeId,
    owner: UiNodeId,
    popup: UiNodeId,
    row: UiNodeId,
    sibling: Option<UiNodeId>,
    host: UiNodeId,
    expected_origin: UiPoint,
}

fn portals() -> Vec<UiPortalTarget> {
    vec![
        UiPortalTarget::Parent,
        UiPortalTarget::AppOverlay,
        UiPortalTarget::named("host"),
        UiPortalTarget::named("missing"),
        UiPortalTarget::GlobalAppOverlay,
        UiPortalTarget::global_named("host"),
        UiPortalTarget::global_named("missing"),
    ]
}

fn independent(portal: &UiPortalTarget) -> bool {
    matches!(
        portal,
        UiPortalTarget::GlobalAppOverlay | UiPortalTarget::GlobalNamed(_)
    )
}

fn fixture(control: Control, portal: UiPortalTarget, name: &str) -> Fixture {
    let mut document = UiDocument::new(root_style(VIEWPORT.width, VIEWPORT.height));
    let root = document.root();
    let parent = document.add_child(
        root,
        UiNode::container(
            "parent",
            operad::layout::with_absolute_position(LayoutStyle::size(1000.0, 650.0), 140.0, 90.0),
        ),
    );
    let named = document.add_child(
        root,
        UiNode::container(
            "host",
            operad::layout::with_absolute_position(LayoutStyle::size(1000.0, 700.0), 250.0, 50.0),
        ),
    );
    document.register_portal_host("host", named);
    let host = match &portal {
        UiPortalTarget::AppOverlay | UiPortalTarget::GlobalAppOverlay => {
            document.ensure_app_overlay_portal()
        }
        UiPortalTarget::Named(id) | UiPortalTarget::GlobalNamed(id) => {
            document.portal_host(id.clone()).unwrap_or(parent)
        }
        UiPortalTarget::Parent => parent,
    };
    let anchor = UiRect::new(30.0, 40.0, 100.0, 30.0);
    let viewport = UiRect::new(0.0, 0.0, 1000.0, 650.0);
    let placement = PopupPlacement::new(PopupSide::Bottom, PopupAlign::Start).with_offset(0.0);
    let anchors = MenuButtonAnchors::new(anchor, viewport)
        .with_submenu_anchor(vec![0], UiRect::new(30.0, 70.0, 240.0, 28.0))
        .with_submenu_anchor(vec![0, 0], UiRect::new(280.0, 70.0, 240.0, 28.0));
    let items = vec![MenuItem::command("leaf", "Leaf")];
    let (trigger, owner, popup, row, sibling) = match control {
        Control::Button | Control::Submenu => {
            let nested = vec![
                MenuItem::submenu(
                    "first",
                    "First",
                    vec![
                        MenuItem::submenu("second", "Second", items.clone()),
                        MenuItem::command("sibling", "Sibling"),
                    ],
                ),
                MenuItem::command("other", "Other"),
            ];
            let submenu = matches!(control, Control::Submenu);
            let mut options = MenuButtonOptions::default().with_action("trigger");
            options.popup_placement = placement;
            options.submenu_placement =
                PopupPlacement::new(PopupSide::Right, PopupAlign::Start).with_offset(0.0);
            options.popup_menu.portal = portal;
            options.popup_menu.action_prefix = Some("menu".into());
            let nodes = menu_button(
                &mut document,
                parent,
                name,
                "Menu",
                if submenu { &nested } else { &items },
                &MenuButtonState {
                    open: true,
                    navigation: MenuNavigationState::with_active_path(if submenu {
                        vec![0, 0, 0]
                    } else {
                        vec![0]
                    }),
                },
                Some(&anchors),
                options,
            );
            if submenu {
                assert_eq!(nodes.submenus.len(), 2);
                (
                    nodes.button,
                    nodes.submenus[0].rows[0],
                    nodes.submenus[1].root,
                    nodes.submenus[1].rows[0],
                    Some(nodes.submenus[0].rows[1]),
                )
            } else {
                let popup = nodes.popup.unwrap();
                (nodes.button, nodes.button, popup.root, popup.rows[0], None)
            }
        }
        Control::Bar => {
            let mut options = MenuBarOptions::default();
            options.popup_placement = placement;
            options.popup_menu.portal = portal;
            options.popup_menu.action_prefix = Some("menu".into());
            let nodes = menu_bar(
                &mut document,
                parent,
                name,
                &[MenuBarMenu::new("file", "File", items)],
                &MenuBarState {
                    open_menu: Some(0),
                    active_item: Some(0),
                },
                Some(&MenuBarAnchors {
                    anchors: vec![anchor],
                    viewport,
                }),
                options,
            );
            let popup = nodes.popup.unwrap();
            (
                nodes.buttons[0],
                nodes.buttons[0],
                popup.root,
                popup.rows[0],
                None,
            )
        }
        Control::Dropdown => {
            let items = [SelectOption::new("leaf", "Leaf")];
            let state = SelectMenuState::new().with_open(&items);
            let mut options = DropdownSelectOptions::default();
            options.menu.portal = portal;
            options.menu.action_prefix = Some("menu".into());
            let nodes = dropdown_select(
                &mut document,
                parent,
                name,
                &items,
                &state,
                Some(AnchoredPopup::new(anchor, viewport, placement)),
                options,
            );
            let popup = nodes.popup.unwrap();
            (
                nodes.trigger,
                nodes.trigger,
                popup.root,
                popup.rows[0],
                None,
            )
        }
    };
    layout(&mut document);
    let host_rect = document.node(host).layout().rect;
    let x = if matches!(control, Control::Submenu) {
        520.0
    } else {
        30.0
    };
    Fixture {
        document,
        trigger,
        owner,
        popup,
        row,
        sibling,
        host,
        expected_origin: UiPoint::new(host_rect.x + x, host_rect.y + 70.0),
    }
}

fn layout(document: &mut UiDocument) {
    document
        .compute_layout(VIEWPORT, &mut ApproxTextMeasurer)
        .unwrap();
}

fn center(document: &UiDocument, node: UiNodeId) -> UiPoint {
    let rect = document.node(node).layout().rect;
    assert!(rect.width > 0.0 && rect.height > 0.0);
    UiPoint::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

fn click(document: &mut UiDocument, point: UiPoint) -> Option<UiNodeId> {
    document.handle_input(UiInputEvent::PointerDown(point));
    document
        .handle_input(UiInputEvent::PointerUp(point))
        .clicked
}

fn queued_action(document: &UiDocument, row: UiNodeId, position: UiPoint) -> Option<WidgetAction> {
    WidgetAction::from_gesture_event_for_document(
        document,
        &GestureEvent::Click(PointerClick {
            pointer_id: PointerId::MOUSE,
            target: row,
            position,
            button: PointerButton::Primary,
            count: 1,
            modifiers: KeyModifiers::NONE,
            timestamp_millis: 1,
        }),
        |id| document.node(id).action().cloned(),
    )
}

fn exercise(control: Control) {
    for portal in portals() {
        let global = independent(&portal);
        let Fixture {
            mut document,
            trigger,
            owner,
            popup,
            row,
            sibling,
            host,
            expected_origin,
        } = fixture(control, portal.clone(), "control");
        let context = format!("{control:?} {portal:?}");
        let rect = document.node(popup).layout().rect;
        let clip = document.node(row).layout().clip_rect;
        assert_eq!(
            document.node(popup).parent(),
            Some(host),
            "{context}: placement host"
        );
        assert_eq!(
            UiPoint::new(rect.x, rect.y),
            expected_origin,
            "{context}: anchor coordinates"
        );
        let point = center(&document, row);
        assert_eq!(
            click(&mut document, point),
            Some(row),
            "{context}: positive control"
        );
        assert!(queued_action(&document, row, point).is_some());
        document.handle_input(UiInputEvent::PointerDown(point));
        assert_eq!(document.focus_state().pressed, Some(row));
        document.set_node_enabled(owner, false);
        assert_eq!(
            document.node_is_enabled(row),
            global,
            "{context}: source disable"
        );
        assert_eq!(document.focus_state().pressed, global.then_some(row));
        assert_eq!(document.focus_state().focused, global.then_some(row));
        let tree = document.accessibility_snapshot();
        assert_eq!(tree.node(row).unwrap().enabled, global);
        assert_eq!(tree.effective_focus_order().contains(&row), global);
        assert_eq!(tree.contains_node(owner, row), !global);
        assert_eq!(queued_action(&document, row, point).is_some(), global);
        assert_eq!(
            document
                .handle_input(UiInputEvent::PointerUp(point))
                .clicked,
            global.then_some(row)
        );
        assert_eq!(click(&mut document, point), global.then_some(row));
        if let Some(sibling) = sibling {
            let sibling_point = center(&document, sibling);
            assert_eq!(click(&mut document, sibling_point), Some(sibling));
        }
        document.set_node_enabled(owner, true);
        layout(&mut document);
        assert_eq!(document.node(popup).layout().rect, rect);
        assert_eq!(document.node(row).layout().clip_rect, clip);
        assert_eq!(click(&mut document, point), Some(row));

        // Root trigger ownership must also reach the deepest submenu.
        document.set_node_enabled(trigger, false);
        assert_eq!(document.node_is_enabled(row), global);
        document.set_node_enabled(trigger, true);
        document.set_node_enabled(row, false);
        document.set_node_enabled(trigger, false);
        document.set_node_enabled(trigger, true);
        assert!(
            !document.node_is_enabled(row),
            "a source toggle must preserve a row's own disabled state"
        );
        document.set_node_enabled(row, true);

        for source in [owner, trigger] {
            let style = document.node(source).style().clone();
            let hidden = style.layout().unwrap().display(LayoutDisplay::None);
            document.set_node_style(
                source,
                operad::UiNodeStyle::new(hidden)
                    .with_clip(style.clip())
                    .with_z_index(style.z_index())
                    .with_opacity(style.opacity()),
            );
            layout(&mut document);
            assert_eq!(
                document.node(row).layout().visible,
                global,
                "{context}: hidden source"
            );
            assert_eq!(
                document.accessibility_snapshot().node(row).is_some(),
                global
            );
            assert_eq!(
                document
                    .paint_list()
                    .items
                    .iter()
                    .any(|item| item.node == popup),
                global
            );
            assert_eq!(click(&mut document, point), global.then_some(row));
            document.set_node_style(source, style);
            layout(&mut document);
            assert_eq!(document.node(popup).layout().rect, rect);
            assert_eq!(document.node(row).layout().clip_rect, clip);
            assert_eq!(click(&mut document, point), Some(row));
        }
    }
}

#[test]
fn menu_button_popup_follows_its_trigger_lifetime() {
    exercise(Control::Button);
}

#[test]
fn menu_bar_popup_follows_its_trigger_lifetime() {
    exercise(Control::Bar);
}

#[test]
fn dropdown_popup_follows_its_trigger_lifetime() {
    exercise(Control::Dropdown);
}

#[test]
fn nested_popup_follows_each_trigger_in_its_ownership_chain() {
    exercise(Control::Submenu);
}

#[test]
fn cached_popups_remap_ownership_and_cancel_disabled_captures() {
    use operad::host::collect_document_widget_actions;
    use operad::input::{PointerEventKind, RawInputEvent, RawPointerEvent};
    use operad::platform::PlatformRequestIdAllocator;
    use operad::renderer::RenderTarget;
    use operad::runtime::session::RuntimeSession;
    use operad::UiDocumentScale;
    use std::cell::Cell;

    fn find(document: &UiDocument, name: &str) -> UiNodeId {
        document
            .nodes()
            .iter()
            .enumerate()
            .find_map(|(index, node)| (node.name() == name).then_some(UiNodeId::from_index(index)))
            .expect("cached node")
    }

    fn frame(
        session: &mut RuntimeSession,
        document: &mut UiDocument,
        kinds: &[PointerEventKind],
        row: UiNodeId,
    ) -> Vec<WidgetAction> {
        let point = center(document, row);
        let input = session
            .process_input(
                document,
                VIEWPORT,
                kinds
                    .iter()
                    .copied()
                    .map(|kind| RawInputEvent::Pointer(RawPointerEvent::new(kind, point, 1)))
                    .collect(),
                Vec::new(),
                &mut ApproxTextMeasurer,
            )
            .unwrap();
        let output = session
            .finish_frame(
                document,
                VIEWPORT,
                RenderTarget::window("popup", VIEWPORT),
                input,
                &mut ApproxTextMeasurer,
                &mut PlatformRequestIdAllocator::default(),
            )
            .unwrap();
        collect_document_widget_actions(&output)
    }

    let down = PointerEventKind::Down(PointerButton::Primary);
    let up = PointerEventKind::Up(PointerButton::Primary);
    for control in [
        Control::Button,
        Control::Bar,
        Control::Dropdown,
        Control::Submenu,
    ] {
        for portal in portals() {
            let global = independent(&portal);
            let prototype = fixture(control, portal.clone(), "control");
            let owner_name = prototype.document.node(prototype.owner).name().to_owned();
            let row_name = prototype.document.node(prototype.row).name().to_owned();
            let builds = Cell::new(0);
            let mut session = RuntimeSession::new();
            let mut previous_row: Option<UiNodeId> = None;
            for step in 0..4 {
                session.invalidate_view();
                // Keep active click coordinates stable; change scale after release.
                let scale = UiDocumentScale::new(if step == 3 { 1.5 } else { 1.0 }, 1.0);
                let mut document = session
                    .build_document(
                        VIEWPORT,
                        scale,
                        None,
                        &mut ApproxTextMeasurer,
                        |_, views| {
                            let mut document =
                                UiDocument::new(root_style(VIEWPORT.width, VIEWPORT.height));
                            let root = document.root();
                            if step % 2 == 1 {
                                document.add_child(
                                    root,
                                    UiNode::container("padding", LayoutStyle::size(1.0, 1.0)),
                                );
                            }
                            views.section(&mut document, root, "panel", &(), |_, _| {
                                builds.set(builds.get() + 1);
                                fixture(control, portal.clone(), "control").document
                            });
                            document
                        },
                    )
                    .unwrap();
                // Scale changes invalidate sections; index-only changes reuse them.
                assert_eq!(builds.get(), if step == 3 { 2 } else { 1 });
                if step == 1 || step == 2 {
                    assert_eq!(session.view_build_stats().reused, 1);
                }
                let row = find(&document, &row_name);
                let owner = find(&document, &owner_name);
                assert_eq!(
                    document.accessibility_snapshot().contains_node(owner, row),
                    !global
                );
                if let Some(previous) = previous_row {
                    assert_ne!(row, previous, "cached section must exercise node remapping");
                }
                previous_row = Some(row);
                match step {
                    0 => {
                        assert!(frame(&mut session, &mut document, &[down], row).is_empty());
                        assert_eq!(session.interaction().pressed, Some(row));
                    }
                    1 => {
                        assert_eq!(session.interaction().pressed, Some(row));
                        assert_eq!(session.interaction().focused, Some(row));
                        let actions = frame(&mut session, &mut document, &[up], row);
                        assert_eq!(actions.len(), 1, "{control:?} {portal:?} step={step}");
                        assert_eq!(actions[0].target, row);
                        assert!(frame(&mut session, &mut document, &[down], row).is_empty());
                    }
                    2 => {
                        document.set_node_enabled(owner, false);
                        session
                            .prepare_document(
                                &mut document,
                                VIEWPORT,
                                scale,
                                None,
                                &mut ApproxTextMeasurer,
                            )
                            .unwrap();
                        assert_eq!(session.interaction().pressed, global.then_some(row));
                        assert_eq!(session.interaction().focused, global.then_some(row));
                        let actions = frame(&mut session, &mut document, &[up], row);
                        assert_eq!(actions.len(), usize::from(global));
                    }
                    3 => {
                        assert!(document.node_is_enabled(row));
                        assert_eq!(session.interaction().pressed, None);
                        assert!(
                            frame(&mut session, &mut document, &[up], row).is_empty(),
                            "restoring the trigger must not revive a stale release"
                        );
                        let actions = frame(&mut session, &mut document, &[down, up], row);
                        assert_eq!(actions.len(), 1, "{control:?} {portal:?} step={step}");
                        assert_eq!(actions[0].target, row);
                    }
                    _ => unreachable!(),
                }
                session.retain_document(document);
            }
            assert_eq!(
                builds.get(),
                2,
                "only the scale change rebuilds the section"
            );
        }
    }
}
