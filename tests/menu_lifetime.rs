#![cfg(feature = "widgets")]

use operad::widgets::ext::menu::{
    menu_command_selection_at_path, menu_item_at_path, menu_selection_at_path,
};
use operad::widgets::ext::{
    menu_bar, menu_button, MenuBarAnchors, MenuBarMenu, MenuBarOptions, MenuBarState,
    MenuButtonAnchors, MenuButtonOptions, MenuButtonState, MenuItem, MenuItemKind,
    MenuNavigationState, NavigationDirection,
};
use operad::{
    root_style, ApproxTextMeasurer, KeyCode, KeyModifiers, UiDocument, UiInputEvent, UiNodeId,
    UiPoint, UiRect, UiSize,
};

fn key(key: KeyCode) -> UiInputEvent {
    UiInputEvent::Key {
        key,
        modifiers: KeyModifiers::NONE,
    }
}

fn items() -> Vec<MenuItem> {
    vec![
        MenuItem::submenu(
            "root",
            "Root",
            vec![
                MenuItem::command("alpha", "Alpha"),
                MenuItem::submenu(
                    "nested",
                    "Nested",
                    vec![
                        MenuItem::command("leaf", "Leaf"),
                        MenuItem::check("check", "Check", true),
                    ],
                ),
                MenuItem::command("zebra", "Zebra"),
            ],
        ),
        MenuItem::command("other", "Other"),
    ]
}

fn children(item: &mut MenuItem) -> &mut Vec<MenuItem> {
    let MenuItemKind::Submenu { items } = &mut item.kind else {
        panic!("submenu fixture")
    };
    items
}

fn set_enabled(items: &mut [MenuItem], mask: u8) {
    items[0].enabled = mask & 1 != 0;
    let nested = &mut children(&mut items[0])[1];
    nested.enabled = mask & 2 != 0;
    children(nested)[0].enabled = mask & 4 != 0;
    children(nested)[1].enabled = mask & 8 != 0;
}

#[test]
fn menu_selection_requires_every_ancestor_to_be_enabled() {
    for mask in (0..16).rev() {
        let mut items = items();
        set_enabled(&mut items, mask);
        for leaf in 0..2 {
            let path = vec![0, 1, leaf];
            let id = if leaf == 0 { "leaf" } else { "check" };
            let enabled = mask & 3 == 3 && mask & (4 << leaf) != 0;
            // Read-only lookup still exposes disabled data to editors/inspectors.
            assert_eq!(
                menu_item_at_path(&items, &path).unwrap().id.as_deref(),
                Some(id)
            );
            assert_eq!(
                menu_selection_at_path(&items, &path).is_some(),
                enabled,
                "mask={mask}"
            );
            assert_eq!(
                menu_command_selection_at_path(&items, &path).is_some(),
                enabled
            );
            let navigation = MenuNavigationState::with_active_path(path.clone());
            assert_eq!(navigation.select_active(&items).is_some(), enabled);
            assert_eq!(navigation.active_item(&items).is_some(), enabled);
            for event in [key(KeyCode::Enter), key(KeyCode::Character(' '))] {
                let mut state = MenuButtonState {
                    open: true,
                    navigation: navigation.clone(),
                };
                let outcome = state.handle_event(&items, &event);
                assert_eq!(outcome.selected.is_some(), enabled);
                assert_eq!(outcome.closed, enabled);
                if let Some(selection) = outcome.selected {
                    assert_eq!(selection.id.as_deref(), Some(id));
                    assert_eq!(selection.index_path, path);
                } else {
                    assert!(!outcome.opened_submenu);
                }
            }
        }
        assert!(
            menu_selection_at_path(&items, &[1]).is_some(),
            "unrelated sibling remains enabled"
        );
    }
}

#[test]
fn submenu_opening_rejects_disabled_paths() {
    for mask in 0..16 {
        let mut items = items();
        set_enabled(&mut items, mask);
        for path in [vec![0], vec![0, 1]] {
            let child = if path.len() == 1 {
                (mask & 1 != 0).then_some(0)
            } else if mask & 3 == 3 && mask & 12 != 0 {
                Some(if mask & 4 != 0 { 0 } else { 1 })
            } else {
                None
            };
            let expected = child.map(|child| {
                let mut path = path.clone();
                path.push(child);
                path
            });
            let mut direct = MenuNavigationState::with_active_path(path.clone());
            assert_eq!(
                direct.open_submenu(&items),
                expected,
                "mask={mask}, path={path:?}"
            );
            for event in [
                key(KeyCode::ArrowRight),
                key(KeyCode::Enter),
                key(KeyCode::Character(' ')),
            ] {
                let mut navigation = MenuNavigationState::with_active_path(path.clone());
                let outcome = navigation.handle_event(&items, &event);
                assert_eq!(outcome.active_path, expected);
                assert_eq!(outcome.opened_submenu, expected.is_some());
                assert!(outcome.selected.is_none());
                assert_eq!(
                    navigation.active_path,
                    expected.clone().unwrap_or_else(|| path.clone())
                );
            }
        }
    }
}

#[test]
fn navigation_recovers_at_the_first_unavailable_ancestor() {
    for unavailable in 0..4 {
        let mut items = items();
        match unavailable {
            0 => items[0].enabled = false,
            1 => children(&mut items[0])[1].enabled = false,
            2 => items[0].kind = MenuItemKind::Command,
            3 => children(&mut items[0])[1].kind = MenuItemKind::Command,
            _ => unreachable!(),
        }
        let at_root = unavailable % 2 == 0;
        let expected_next = if at_root { vec![1] } else { vec![0, 2] };
        let expected_previous = if at_root { vec![1] } else { vec![0, 0] };
        for (direction, expected) in [
            (NavigationDirection::Next, expected_next.clone()),
            (NavigationDirection::Previous, expected_previous.clone()),
        ] {
            let mut navigation = MenuNavigationState::with_active_path(vec![0, 1, 0]);
            assert_eq!(navigation.move_active(&items, direction), Some(expected));
        }
        let events = [
            (key(KeyCode::ArrowDown), expected_next.clone()),
            (key(KeyCode::ArrowUp), expected_previous),
            (key(KeyCode::End), expected_next.clone()),
            (
                UiInputEvent::TextInput(if at_root { "o" } else { "z" }.into()),
                expected_next,
            ),
        ];
        for (event, expected) in events {
            let mut navigation = MenuNavigationState::with_active_path(vec![0, 1, 0]);
            let outcome = navigation.handle_event(&items, &event);
            assert_eq!(
                outcome.active_path,
                Some(expected.clone()),
                "case={unavailable}"
            );
            assert_eq!(navigation.active_path, expected);
            assert!(
                outcome.selected.is_none(),
                "recovery must not activate another command"
            );
        }
    }
    for remove_root in [false, true] {
        let mut items = items();
        let expected = if remove_root {
            items.remove(0);
            vec![0]
        } else {
            children(&mut items[0]).remove(1);
            vec![0, 0]
        };
        let mut navigation = MenuNavigationState::with_active_path(vec![0, 1, 0]);
        assert!(navigation.select_active(&items).is_none());
        assert_eq!(
            navigation.move_active(&items, NavigationDirection::Next),
            Some(expected)
        );
    }
    for (missing_path, expected) in [(vec![99, 0], vec![0]), (vec![0, 99, 0], vec![0, 0])] {
        let mut navigation = MenuNavigationState::with_active_path(missing_path);
        assert_eq!(
            navigation.move_active(&items(), NavigationDirection::Next),
            Some(expected)
        );
    }
}

fn action_node(document: &UiDocument, action: &str) -> Option<UiNodeId> {
    document
        .nodes()
        .iter()
        .enumerate()
        .find_map(|(index, node)| {
            (node
                .action()
                .and_then(|binding| binding.action_id())
                .map(AsRef::as_ref)
                == Some(action))
            .then_some(UiNodeId::from_index(index))
        })
}

fn assert_clickable(document: &mut UiDocument, node: UiNodeId) {
    document
        .compute_layout(UiSize::new(1200.0, 700.0), &mut ApproxTextMeasurer)
        .unwrap();
    let rect = document.node(node).layout().rect;
    assert!(rect.width > 0.0 && rect.height > 0.0);
    let point = UiPoint::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    document.handle_input(UiInputEvent::PointerDown(point));
    assert_eq!(
        document
            .handle_input(UiInputEvent::PointerUp(point))
            .clicked,
        Some(node)
    );
}

#[test]
fn menu_button_rebuilds_remove_actions_beneath_disabled_branches() {
    let state = MenuButtonState {
        open: true,
        navigation: MenuNavigationState::with_active_path(vec![0, 1, 0]),
    };
    let anchors = MenuButtonAnchors::new(
        UiRect::new(10.0, 10.0, 100.0, 30.0),
        UiRect::new(0.0, 0.0, 1200.0, 700.0),
    )
    .with_submenu_anchor(vec![0], UiRect::new(10.0, 44.0, 240.0, 28.0))
    .with_submenu_anchor(vec![0, 1], UiRect::new(260.0, 74.0, 240.0, 28.0));
    for trigger_enabled in [true, false] {
        for mask in [15, 14, 13, 15] {
            // Open, disable each ancestor, restore.
            let mut items = items();
            set_enabled(&mut items, mask);
            let mut document = UiDocument::new(root_style(1200.0, 700.0));
            let root = document.root();
            let mut options = MenuButtonOptions::default().with_action_prefix("menu");
            options.enabled = trigger_enabled;
            let nodes = menu_button(
                &mut document,
                root,
                "menu",
                "Menu",
                &items,
                &state,
                Some(&anchors),
                options,
            );
            let leaf = action_node(&document, "menu.leaf");
            assert_eq!(
                leaf.is_some(),
                trigger_enabled && mask & 3 == 3,
                "mask={mask}"
            );
            assert_eq!(
                action_node(&document, "menu.other").is_some(),
                trigger_enabled
            );
            let metadata = document.node(nodes.button).accessibility().unwrap();
            assert_eq!(metadata.expanded, Some(trigger_enabled));
            for node in document.nodes() {
                if let Some(meta) = node.accessibility().filter(|meta| !meta.enabled) {
                    assert_ne!(meta.selected, Some(true));
                    assert_ne!(meta.expanded, Some(true));
                }
            }
            if let Some(leaf) = leaf {
                assert_clickable(&mut document, leaf);
            }
        }
    }
}

fn menus() -> Vec<MenuBarMenu> {
    vec![
        MenuBarMenu::new(
            "file",
            "File",
            vec![
                MenuItem::command("save", "Save"),
                MenuItem::command("close", "Close"),
            ],
        ),
        MenuBarMenu::new("view", "View", vec![MenuItem::check("grid", "Grid", true)]),
    ]
}

#[test]
fn menu_bar_state_rejects_commands_after_its_open_menu_is_disabled() {
    let mut menus = menus();
    let mut state = MenuBarState::default();
    assert!(state.open(&menus, 0));
    assert!(state.select_active(&menus).is_some());
    menus[0].enabled = false;
    assert!(state.select_active(&menus).is_none());
    assert!(state.set_active_item_by_id(&menus, "close").is_none());
    assert!(state.move_item(&menus, NavigationDirection::Next).is_none());
    assert_eq!(state.move_menu(&menus, NavigationDirection::Next), Some(1));
    assert_eq!(
        state.select_active(&menus).unwrap().id.as_deref(),
        Some("grid")
    );
    menus[0].enabled = true;
    assert!(state.open(&menus, 0));
    assert_eq!(state.set_active_item_by_id(&menus, "close"), Some(1));
    assert_eq!(
        state.select_active(&menus).unwrap().id.as_deref(),
        Some("close")
    );
}

#[test]
fn menu_bar_rebuilds_remove_a_disabled_menus_popup() {
    let state = MenuBarState {
        open_menu: Some(0),
        active_item: Some(0),
    };
    let anchors = MenuBarAnchors {
        anchors: vec![
            UiRect::new(10.0, 10.0, 80.0, 30.0),
            UiRect::new(100.0, 10.0, 80.0, 30.0),
        ],
        viewport: UiRect::new(0.0, 0.0, 1200.0, 700.0),
    };
    for enabled in [true, false, true] {
        let mut menus = menus();
        menus[0].enabled = enabled;
        let mut document = UiDocument::new(root_style(1200.0, 700.0));
        let root = document.root();
        let mut options = MenuBarOptions::default().with_action_prefix("bar");
        options.popup_menu.action_prefix = Some("item".into());
        let nodes = menu_bar(
            &mut document,
            root,
            "bar",
            &menus,
            &state,
            Some(&anchors),
            options,
        );
        let save = action_node(&document, "item.save");
        assert_eq!(save.is_some(), enabled);
        assert_eq!(
            document
                .node(nodes.buttons[0])
                .accessibility()
                .unwrap()
                .expanded,
            Some(enabled)
        );
        assert!(action_node(&document, "bar.view").is_some());
        if let Some(save) = save {
            assert_clickable(&mut document, save);
        }
    }
}

#[test]
fn reopening_menus_discards_unavailable_paths() {
    let mut items = items();
    let path = vec![0, 1, 0];
    let mut state = MenuButtonState {
        open: false,
        navigation: MenuNavigationState::with_active_path(path.clone()),
    };
    assert_eq!(state.open(&items), Some(path));
    items[0].enabled = false;
    assert_eq!(state.open(&items), Some(vec![1]));
    items[1].enabled = false;
    assert_eq!(state.open(&items), None);
    assert!(state.navigation.active_path.is_empty());
    state.navigation.active_path = vec![0, 1, 0];
    assert_eq!(state.navigation.open_root(&[]), None);
    assert!(state.navigation.active_path.is_empty());
}
