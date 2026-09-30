#![cfg(feature = "widgets")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use operad::widgets::text_input::{
    handle_text_input_event_with_metrics, TextInputLayoutMetrics, TextInputState,
};
use operad::{
    InputBehavior, KeyCode, KeyModifiers, LayoutStyle, TextCompositionEvent, UiDocument,
    UiFocusState, UiInputEvent, UiNode, UiRect,
};

thread_local! {
    // A const, allocation-free TLS slot excludes allocations from other test threads.
    static ALLOCATED: Cell<Option<usize>> = const { Cell::new(None) };
}

struct CountingAllocator;

fn record_allocation(bytes: usize) {
    let _ = ALLOCATED.try_with(|allocated| {
        if let Some(total) = allocated.get() {
            allocated.set(Some(total.saturating_add(bytes)));
        }
    });
}

// Allocation and deallocation remain paired through System; only byte counts change.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation(size);
        unsafe { System.realloc(pointer, layout, size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn allocated_bytes(run: impl FnOnce()) -> usize {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ALLOCATED.with(|allocated| allocated.set(None));
        }
    }
    ALLOCATED.with(|allocated| assert!(allocated.replace(Some(0)).is_none()));
    let _reset = Reset;
    run();
    ALLOCATED.with(|allocated| allocated.get().unwrap())
}

#[test]
fn text_input_non_editing_operations_have_bounded_allocations() {
    let key = |key, ctrl| UiInputEvent::Key {
        key,
        modifiers: KeyModifiers {
            ctrl,
            ..KeyModifiers::NONE
        },
    };
    let preedit = UiInputEvent::Composition {
        target: None,
        event: TextCompositionEvent::Preedit {
            text: "候補".into(),
            selection: Some(3..6),
            replacement: None,
        },
    };
    let cancel = UiInputEvent::Composition {
        target: None,
        event: TextCompositionEvent::Cancel,
    };
    let cases = [
        (
            "arrows",
            [
                key(KeyCode::ArrowLeft, false),
                key(KeyCode::ArrowRight, false),
            ],
        ),
        (
            "select all",
            std::array::from_fn(|_| key(KeyCode::Character('a'), true)),
        ),
        (
            "copy",
            std::array::from_fn(|_| key(KeyCode::Character('c'), true)),
        ),
        ("preedit", [preedit.clone(), preedit.clone()]),
        ("cancel", [cancel.clone(), cancel]),
    ];
    let mut document = UiDocument::new(LayoutStyle::size(320.0, 80.0));
    let node = document.add_child(
        document.root(),
        UiNode::container("editor", LayoutStyle::size(320.0, 80.0))
            .with_input(InputBehavior::BUTTON),
    );
    document.set_focus_state(UiFocusState {
        focused: Some(node),
        ..Default::default()
    });
    let metrics = TextInputLayoutMetrics::new(UiRect::new(0.0, 0.0, 320.0, 80.0), 8.0, 16.0);

    for size in [4 * 1024, 1024 * 1024] {
        for pattern in ["x", "e\u{301}👩‍🚀"] {
            let text = pattern.repeat(size / pattern.len());
            for through_document in [false, true] {
                for (name, events) in &cases {
                    let mut state = TextInputState::new(text.clone());
                    state.set_caret(size / 2);
                    if *name == "copy" {
                        state.set_selection(size / 2, size / 2 + 8);
                    } else if *name == "cancel" {
                        state.handle_event(&preedit);
                    }
                    let iterations = 16;
                    let bytes = allocated_bytes(|| {
                        for iteration in 0..iterations {
                            let event = &events[iteration % events.len()];
                            let outcome = if through_document {
                                let outcome = handle_text_input_event_with_metrics(
                                    &mut document,
                                    node,
                                    &mut state,
                                    event.clone(),
                                    None,
                                    Some(metrics),
                                );
                                assert!(outcome.input.consumed);
                                outcome.edit.expect("focused text input")
                            } else {
                                state.handle_event(event)
                            };
                            assert!(!outcome.changed && outcome.transaction.is_none());
                            std::hint::black_box(outcome);
                        }
                    });
                    // Allow small draft, clipboard, and request allocations, independent
                    // of the committed text length. Timing is not part of this contract.
                    assert!(
                    bytes <= iterations * 1024,
                    "{name}: size={size}, pattern={pattern:?}, document={through_document}, allocated={bytes}"
                );
                    assert_eq!(state.text(), text);
                    assert!(!state.history().can_undo());
                }
            }
        }
    }
}

#[test]
fn text_input_rendering_allocations_do_not_grow_with_edit_history() {
    use operad::widgets::{self, TextInputOptions};

    fn build(kind: &str, state: &TextInputState) -> UiDocument {
        let mut document = UiDocument::new(LayoutStyle::size(320.0, 160.0));
        let parent = document.root();
        let options = TextInputOptions {
            focused: true,
            ..Default::default()
        };
        match kind {
            "base" => widgets::text_input(&mut document, parent, "field", state, options),
            "singleline" => {
                widgets::singleline_text_input(&mut document, parent, "field", state, options)
            }
            "multiline" => {
                widgets::multiline_text_input(&mut document, parent, "field", state, options)
            }
            "area" => widgets::text_area(&mut document, parent, "field", state, options),
            "code" => widgets::code_editor(&mut document, parent, "field", state, options),
            "search" => widgets::search_input(&mut document, parent, "field", state, options),
            "password" => widgets::password_input(&mut document, parent, "field", state, options),
            _ => unreachable!(),
        };
        document
    }

    let large_edit = UiInputEvent::TextInput("x".repeat(32 * 1024));
    let small_edit = UiInputEvent::TextInput("Ae\u{301}👩‍🚀\nZ".into());
    let mut regressions = Vec::new();
    for multiline in [false, true] {
        for composing in [false, true] {
            let mut history = TextInputState::new("").multiline(multiline);
            for _ in 0..16 {
                for edit in [&large_edit, &small_edit] {
                    history.select_all();
                    assert!(history.handle_event(edit).changed);
                }
            }
            history.undo_text_edit().unwrap();
            history.undo_text_edit().unwrap();
            assert!(history.history().can_undo() && history.history().can_redo());
            history.set_selection(1, 4);
            if composing {
                history.apply_composition(&TextCompositionEvent::Preedit {
                    text: "候補".into(),
                    selection: Some(3..6),
                    replacement: None,
                });
            }
            let before = history.clone();
            let mut pristine = history.clone();
            pristine.clear_history();
            for kind in [
                "base",
                "singleline",
                "multiline",
                "area",
                "code",
                "search",
                "password",
            ] {
                // Warm font/shaping caches for the same visible content before
                // measuring; history is the only difference between these models.
                std::hint::black_box(build(kind, &pristine));
                std::hint::black_box(build(kind, &history));
                let clean_bytes = allocated_bytes(|| {
                    std::hint::black_box(build(kind, &pristine));
                });
                let history_bytes = allocated_bytes(|| {
                    std::hint::black_box(build(kind, &history));
                });
                let case = format!(
                    "{kind}, multiline={multiline}, composing={composing}: clean={clean_bytes}, history={history_bytes}"
                );
                eprintln!("{case}");
                // Small cache bookkeeping is allowed; retained edit snapshots
                // must not be copied to build the current field's presentation.
                if history_bytes > clean_bytes + 4096 {
                    regressions.push(case);
                }
            }
            assert_eq!(history, before, "rendering changed the editing model");
        }
    }
    assert!(regressions.is_empty(), "{}", regressions.join("\n"));
}

#[cfg(feature = "text-cosmic")]
#[test]
fn text_input_widget_construction_stays_near_one_render_plan_allocation_cost() {
    use operad::widgets::text_input::TextInputPaintOptions;
    use operad::widgets::{self, TextInputOptions};
    use operad::{FontFamily, TextStyle};

    let mut regressions = Vec::new();
    for family in [FontFamily::SansSerif, FontFamily::Monospace] {
        for composition in [0, 1, 2] {
            for read_only in [false, true] {
                let style = TextStyle {
                    family: family.clone(),
                    ..Default::default()
                };
                let mut state = TextInputState::new("Ae\u{301}👩‍🚀\nZ").multiline(true);
                state.set_selection(1, 4);
                if composition != 0 {
                    state.apply_composition(&TextCompositionEvent::Preedit {
                        text: "候補".into(),
                        selection: (composition == 1).then_some(3..6),
                        replacement: None,
                    });
                }
                let render_plan = || {
                    state.render_plan(
                        TextInputLayoutMetrics::from_style(
                            UiRect::new(6.0, 6.0, 300.0, 120.0),
                            &style,
                        ),
                        style.clone(),
                        TextInputPaintOptions::default(),
                    )
                };
                let widget = || {
                    let mut document = UiDocument::new(LayoutStyle::size(320.0, 160.0));
                    let parent = document.root();
                    widgets::text_input(
                        &mut document,
                        parent,
                        "field",
                        &state,
                        TextInputOptions {
                            focused: true,
                            read_only,
                            text_style: style.clone(),
                            ..Default::default()
                        },
                    );
                    document
                };
                std::hint::black_box(render_plan());
                std::hint::black_box(widget());
                let plan_bytes = allocated_bytes(|| {
                    std::hint::black_box(render_plan());
                });
                let widget_bytes = allocated_bytes(|| {
                    std::hint::black_box(widget());
                });
                let case = format!(
                    "{family:?}, composition={composition}, read_only={read_only}: plan={plan_bytes}, widget={widget_bytes}"
                );
                eprintln!("{case}");
                // Account for the document, scene, accessibility, and IME data
                // without multiplying the font-dependent cost of text layout.
                if widget_bytes > plan_bytes + 64 * 1024 {
                    regressions.push(case);
                }
            }
        }
    }
    assert!(regressions.is_empty(), "{}", regressions.join("\n"));
}
