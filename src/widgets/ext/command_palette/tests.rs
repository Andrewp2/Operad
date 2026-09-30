use super::*;
use crate::{ApproxTextMeasurer, KeyModifiers, TextCompositionEvent};

fn key(key: KeyCode, ctrl: bool) -> UiInputEvent {
    UiInputEvent::Key {
        key,
        modifiers: KeyModifiers {
            ctrl,
            ..Default::default()
        },
    }
}

fn composition(event: TextCompositionEvent) -> UiInputEvent {
    UiInputEvent::Composition {
        target: None,
        event,
    }
}

fn palette_document(
    state: &CommandPaletteState,
    items: &[CommandPaletteItem],
) -> (UiDocument, CommandPaletteNodes) {
    let mut document = UiDocument::new(LayoutStyle::column().with_size(600.0, 400.0));
    let root = document.root();
    let nodes = command_palette(
        &mut document,
        root,
        "palette",
        items,
        state,
        None,
        CommandPaletteOptions::default(),
    );
    let mut focus = document.focus_state().clone();
    focus.focused = Some(nodes.input);
    document.set_focus_state(focus);
    document
        .compute_layout(UiSize::new(600.0, 400.0), &mut ApproxTextMeasurer)
        .unwrap();
    (document, nodes)
}

#[test]
fn command_palette_edits_at_the_caret_and_replaces_selection() {
    let items = [
        CommandPaletteItem::new("abc", "abc"),
        CommandPaletteItem::new("ac", "ac"),
    ];
    let mut state = CommandPaletteState::new().with_query("abc");
    state.handle_event(&items, &key(KeyCode::Home, false));
    state.handle_event(&items, &key(KeyCode::ArrowRight, false));
    assert!(
        state
            .handle_event(&items, &key(KeyCode::Delete, false))
            .query_changed
    );
    assert_eq!(
        state.query(),
        "ac",
        "Delete must remove text after the caret"
    );
    state.handle_event(&items, &key(KeyCode::Character('a'), true));
    state.handle_event(&items, &UiInputEvent::TextInput("x".into()));
    assert_eq!(state.query(), "x", "typing replaces the selected query");
    state.handle_event(&items, &key(KeyCode::Character('z'), true));
    assert_eq!(state.query(), "ac", "query replacement is undoable");
    state.set_query("new", &items);
    state.handle_event(&items, &key(KeyCode::Character('z'), true));
    assert_eq!(
        state.query(),
        "new",
        "an external query starts a new history"
    );
    state.handle_event(&items, &key(KeyCode::Character('a'), true));
    state.handle_event(&items, &key(KeyCode::Delete, false));
    assert!(!state.clear_query(&items).query_changed);
    state.handle_event(&items, &key(KeyCode::Character('z'), true));
    assert_eq!(
        state.query(),
        "",
        "clearing an empty query also retires its history"
    );
    assert!(
        !state
            .apply_search_field(&SearchFieldState::from_query("\t\0"), &items)
            .query_changed
    );
}

#[test]
fn command_palette_composition_preserves_query_until_commit_and_owns_keys() {
    let items = [
        CommandPaletteItem::new("candidate", "候補"),
        CommandPaletteItem::new("other", "Other"),
    ];
    let mut state = CommandPaletteState::new().with_first_active_match(&items);
    let draft = composition(TextCompositionEvent::Preedit {
        text: "候補".into(),
        selection: Some(3..6),
        replacement: None,
    });
    assert!(!state.handle_event(&items, &draft).query_changed);
    assert_eq!(state.query(), "");
    let (document, nodes) = palette_document(&state, &items);
    let snapshot = document
        .node(nodes.input)
        .text_input()
        .expect("search exposes its editing snapshot");
    assert_eq!(snapshot.text, "候補");
    assert_eq!(snapshot.composition, Some(0..6));
    assert_eq!(snapshot.selection, 3..6);
    assert!(state
        .handle_event(&items, &key(KeyCode::Enter, false))
        .selected
        .is_none());
    assert!(
        !state
            .handle_event(&items, &key(KeyCode::Escape, false))
            .closed
    );
    assert_eq!(state.query(), "");
    state.handle_event(&items, &draft);
    assert!(
        state
            .handle_event(
                &items,
                &composition(TextCompositionEvent::Commit {
                    text: "候補".into(),
                    replacement: None
                })
            )
            .query_changed
    );
    assert_eq!(state.query(), "候補");
    assert_eq!(state.select_active(&items).unwrap().id, "candidate");
    state.handle_event(&items, &key(KeyCode::Character('z'), true));
    assert_eq!(
        state.query(),
        "",
        "the committed draft is one undoable edit"
    );
    assert!(
        state
            .handle_event(&items, &key(KeyCode::Escape, false))
            .closed
    );
}

#[test]
fn command_palette_runtime_establishes_an_ime_session_for_search() {
    use crate::host::collect_document_widget_actions;
    use crate::input::{RawInputEvent, RawTextCompositionEvent};
    use crate::platform::PlatformRequestIdAllocator;
    use crate::renderer::RenderTarget;
    use crate::runtime::session::RuntimeSession;
    let mut state = CommandPaletteState::new().with_query("a😀z");
    let (mut document, _) = palette_document(&state, &[]);
    let viewport = UiSize::new(600.0, 400.0);
    let mut session = RuntimeSession::new();
    session
        .prepare_document(
            &mut document,
            viewport,
            crate::UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    let input = session
        .process_input(
            &mut document,
            viewport,
            Vec::new(),
            Vec::new(),
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    session
        .finish_frame(
            &mut document,
            viewport,
            RenderTarget::window("palette", viewport),
            input,
            &mut ApproxTextMeasurer,
            &mut PlatformRequestIdAllocator::default(),
        )
        .unwrap();
    let ime = session
        .interaction()
        .text_ime
        .as_ref()
        .expect("focused palette must activate the platform input method");
    assert_eq!(ime.surrounding_text, "a😀z");
    let owner = ime.input.clone();
    let input = session
        .process_input(
            &mut document,
            viewport,
            vec![RawInputEvent::Composition(RawTextCompositionEvent {
                input: owner.clone(),
                event: TextCompositionEvent::Preedit {
                    text: "候補".into(),
                    selection: Some(3..6),
                    replacement: Some(1..5),
                },
                timestamp_millis: 1,
            })],
            Vec::new(),
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    let frame = session
        .finish_frame(
            &mut document,
            viewport,
            RenderTarget::window("palette", viewport),
            input,
            &mut ApproxTextMeasurer,
            &mut PlatformRequestIdAllocator::default(),
        )
        .unwrap();
    let mut delivered = 0;
    for action in collect_document_widget_actions(&frame) {
        if let crate::WidgetActionKind::TextEdit(edit) = action.kind {
            assert_eq!(
                action.binding.action_id().unwrap().as_str(),
                "palette.search"
            );
            state.apply_widget_text_edit(&[], &edit, &CommandPaletteOptions::default());
            delivered += 1;
        }
    }
    assert_eq!(
        delivered, 1,
        "platform composition reaches the search model once"
    );
    assert_eq!(state.query(), "a😀z");
    let (mut rebuilt, nodes) = palette_document(&state, &[]);
    session
        .prepare_document(
            &mut rebuilt,
            viewport,
            crate::UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    let input = session
        .process_input(
            &mut rebuilt,
            viewport,
            Vec::new(),
            Vec::new(),
            &mut ApproxTextMeasurer,
        )
        .unwrap();
    session
        .finish_frame(
            &mut rebuilt,
            viewport,
            RenderTarget::window("palette", viewport),
            input,
            &mut ApproxTextMeasurer,
            &mut PlatformRequestIdAllocator::default(),
        )
        .unwrap();
    let ime = session.interaction().text_ime.as_ref().unwrap();
    assert_eq!(ime.input, owner);
    assert_eq!(ime.surrounding_text, "a候補z");
    assert_eq!(ime.selection, crate::platform::TextRange::new(4, 7));
    assert_eq!(
        rebuilt.node(nodes.input).text_input().unwrap().composition,
        Some(1..7)
    );
}

#[test]
fn command_palette_pointer_selection_and_clipboard_use_shared_editing() {
    use crate::widgets::text_input::TextInputClipboardAction;
    let items = [
        CommandPaletteItem::new("one", "One"),
        CommandPaletteItem::new("two", "Two"),
    ];
    let mut state = CommandPaletteState::new()
        .with_query("a😀z")
        .with_first_active_match(&items);
    let options = CommandPaletteOptions::default();
    let mut edit = WidgetTextEdit::new(UiInputEvent::PointerDown(crate::UiPoint::new(0.0, 8.0)));
    edit.local_position = Some(crate::UiPoint::new(-100.0, 8.0));
    edit.target_rect = Some(crate::UiRect::new(40.0, 4.0, 300.0, 34.0));
    state.apply_widget_text_edit(&items, &edit, &options);
    edit.local_position = None;
    edit.geometry = Some(crate::TextInputPointerGeometry {
        point: crate::UiPoint::new(1000.0, 8.0),
        bounds: crate::UiRect::new(0.0, 0.0, 300.0, 34.0),
        text_style: None,
        mask: None,
    });
    edit.event = UiInputEvent::PointerMove(crate::UiPoint::new(1040.0, 12.0));
    edit.selecting = true;
    state.apply_widget_text_edit(&items, &edit, &options);
    let copy = state.handle_event(&items, &key(KeyCode::Character('c'), true));
    assert_eq!(
        copy.edit.unwrap().clipboard,
        Some(TextInputClipboardAction::Copy("a😀z".into()))
    );
    let cut = state.handle_event(&items, &key(KeyCode::Character('x'), true));
    assert!(cut.query_changed);
    assert_eq!(
        cut.edit.unwrap().clipboard,
        Some(TextInputClipboardAction::Cut("a😀z".into()))
    );
    assert_eq!(state.query(), "");
    state.handle_event(&items, &key(KeyCode::ArrowDown, false));
    assert_eq!(state.select_active(&items).unwrap().id, "two");
    state.handle_event(&items, &key(KeyCode::Home, false));
    assert_eq!(
        state.select_active(&items).unwrap().id,
        "two",
        "caret navigation preserves the active result"
    );
    assert_eq!(
        state
            .handle_event(&items, &key(KeyCode::Character('v'), true))
            .edit
            .unwrap()
            .clipboard,
        Some(TextInputClipboardAction::Paste)
    );
}

#[test]
fn command_palette_painted_caret_matches_platform_geometry() {
    for width in [220.0, 440.0] {
        for icon in [false, true] {
            for composing in [false, true] {
                let mut state = CommandPaletteState::new();
                if composing {
                    state.handle_event(
                        &[],
                        &composition(TextCompositionEvent::Preedit {
                            text: "a😀".into(),
                            selection: Some(1..5),
                            replacement: None,
                        }),
                    );
                }
                let mut document = UiDocument::new(
                    LayoutStyle::column()
                        .with_size(480.0, 220.0)
                        .with_padding(16.0),
                );
                let root = document.root();
                let mut options = CommandPaletteOptions {
                    width,
                    focused: true,
                    ..Default::default()
                };
                if !icon {
                    options.input_image = None;
                }
                let nodes =
                    command_palette(&mut document, root, "palette", &[], &state, None, options);
                document
                    .compute_layout(UiSize::new(480.0, 220.0), &mut ApproxTextMeasurer)
                    .unwrap();
                let field = document.node(nodes.input);
                let local = field.text_input().unwrap().cursor_rect;
                let expected = crate::UiRect::new(
                    field.layout().rect.x + local.x,
                    field.layout().rect.y + local.y,
                    local.width,
                    local.height,
                );
                let paint = document.paint_list();
                let actual = paint
                    .items
                    .iter()
                    .find_map(|item| match &item.kind {
                        crate::PaintKind::RichRect(rect)
                            if rect.rect.width == local.width
                                && rect.rect.height == local.height =>
                        {
                            Some(rect.rect)
                        }
                        _ => None,
                    })
                    .expect("focused query paints a caret");
                assert!((actual.x - expected.x).abs() < 0.01 && (actual.y - expected.y).abs() < 0.01,
                    "painted caret {actual:?} differs from platform caret {expected:?}; width={width}, icon={icon}, composing={composing}");
                assert!(actual.bottom() <= field.layout().rect.bottom());
            }
        }
    }
}
