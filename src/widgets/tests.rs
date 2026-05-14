use taffy::prelude::{Dimension, Size as TaffySize, Style};

use crate::widgets;
use crate::*;

fn button_style(width: f32, height: f32) -> UiNodeStyle {
    UiNodeStyle {
        layout: LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: length(width),
                height: length(height),
            },
            ..Default::default()
        })
        .style,
        ..Default::default()
    }
}

#[cfg(feature = "widgets")]
#[test]
fn widget_apis_accept_legacy_taffy_layout_inputs() {
    let root_style = root_style(280.0, 120.0);
    let mut doc = UiDocument::new(root_style);
    let root = doc.root;
    let legacy = Style {
        size: TaffySize {
            width: length(120.0),
            height: length(24.0),
        },
        ..Default::default()
    };
    let label = widgets::label(
        &mut doc,
        root,
        "label",
        "Legacy Label",
        TextStyle::default(),
        legacy.clone(),
    );
    let scroll = widgets::scroll_area(
        &mut doc,
        root,
        "scroll",
        ScrollAxes::HORIZONTAL,
        legacy.clone(),
    );

    let button_options = widgets::ButtonOptions::new(legacy.clone()).with_layout(legacy.clone());
    let checkbox_options = widgets::CheckboxOptions::default().with_layout(legacy.clone());
    let slider_options = widgets::SliderOptions::default().with_layout(legacy.clone());
    let text_input_options = widgets::TextInputOptions::default().with_layout(legacy.clone());
    let combo_box_options = widgets::ComboBoxOptions::default().with_layout(legacy.clone());

    assert_eq!(doc.node(label).style.layout.size, legacy.size);
    assert_eq!(doc.node(scroll).style.layout.size, legacy.size);
    assert_eq!(button_options.layout.as_taffy_style().size, legacy.size);
    assert_eq!(checkbox_options.layout.as_taffy_style().size, legacy.size);
    assert_eq!(slider_options.layout.as_taffy_style().size, legacy.size);
    assert_eq!(text_input_options.layout.as_taffy_style().size, legacy.size);
    assert_eq!(combo_box_options.layout.as_taffy_style().size, legacy.size);
}
#[cfg(feature = "widgets")]
#[test]
fn widget_localized_label_exports_direction_to_paint_and_accessibility() {
    let mut doc = UiDocument::new(root_style(300.0, 100.0));
    let root = doc.root;
    let policy = LocalizationPolicy::new(LocaleId::new("he-IL").expect("locale"));
    let label_meta = DynamicLabelMeta::dynamic("nav.back", "Back", 3).with_bidi(BidiPolicy::Embed);
    let label = widgets::localized_label(
        &mut doc,
        root,
        "back",
        label_meta,
        Some(&policy),
        TextStyle::default(),
        LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: Dimension::auto(),
                height: Dimension::auto(),
            },
            ..Default::default()
        }),
    );
    doc.compute_layout(UiSize::new(300.0, 100.0), &mut ApproxTextMeasurer)
        .expect("layout");

    let paint = doc.paint_list();
    let text_item = paint
        .items
        .iter()
        .find(|item| item.node == label)
        .expect("localized label paint");
    let PaintKind::Text(content) = &text_item.kind else {
        panic!("expected text paint");
    };
    assert_eq!(content.text, "Back");
    assert_eq!(content.locale.as_ref().map(LocaleId::as_str), Some("he-IL"));
    assert_eq!(content.direction, ResolvedTextDirection::Rtl);
    assert_eq!(content.bidi, BidiPolicy::Embed);
    assert_eq!(
        content
            .dynamic_label
            .as_ref()
            .and_then(|label| label.key.as_deref()),
        Some("nav.back")
    );

    let tree = doc.accessibility_tree();
    let accessible = tree.iter().find(|node| node.id == label).unwrap();
    assert_eq!(accessible.label.as_deref(), Some("Back"));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_button_builds_focusable_document_nodes() {
    let mut doc = UiDocument::new(root_style(200.0, 80.0));
    let root = doc.root;
    let button = widgets::button(
        &mut doc,
        root,
        "play",
        "Play",
        widgets::ButtonOptions::new(LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: length(80.0),
                height: length(32.0),
            },
            ..Default::default()
        })),
    );
    doc.compute_layout(UiSize::new(200.0, 80.0), &mut ApproxTextMeasurer)
        .expect("layout");

    assert!(doc.node(button).input.focusable);
    assert_eq!(doc.node(button).children.len(), 1);
    assert!(doc
        .paint_list()
        .items
        .iter()
        .any(|item| item.node == button));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_button_default_visuals_follow_hover_press_and_focus() {
    let mut doc = UiDocument::new(root_style(200.0, 80.0));
    let root = doc.root;
    let button = widgets::button(
        &mut doc,
        root,
        "play",
        "Play",
        widgets::ButtonOptions::new(LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: length(80.0),
                height: length(32.0),
            },
            ..Default::default()
        })),
    );
    doc.compute_layout(UiSize::new(200.0, 80.0), &mut ApproxTextMeasurer)
        .expect("layout");

    let normal = doc.node(button).visual;
    assert!(doc.node(button).interaction_visuals.is_some());

    let hovered = doc.handle_input(UiInputEvent::PointerMove(UiPoint::new(20.0, 16.0)));
    assert_eq!(hovered.hovered, Some(button));
    let hover_visual = doc.node(button).visual;
    assert_ne!(hover_visual, normal);

    let down = doc.handle_input(UiInputEvent::PointerDown(UiPoint::new(20.0, 16.0)));
    assert_eq!(down.pressed, Some(button));
    let pressed_visual = doc.node(button).visual;
    assert_ne!(pressed_visual, hover_visual);
    assert!(
        pressed_visual.fill.relative_luminance() < hover_visual.fill.relative_luminance(),
        "pressed button defaults should read as a sunken state"
    );
    assert_eq!(
        pressed_visual.corner_radius, hover_visual.corner_radius,
        "pressed button defaults should not change shape while the pointer is down"
    );

    let up = doc.handle_input(UiInputEvent::PointerUp(UiPoint::new(20.0, 16.0)));
    assert_eq!(up.clicked, Some(button));
    assert_eq!(
        doc.node(button).visual,
        hover_visual,
        "hover should remain visible on a focused button while the cursor is still over it"
    );

    let away = doc.handle_input(UiInputEvent::PointerMove(UiPoint::new(160.0, 16.0)));
    assert_eq!(away.hovered, None);
    let focused_visual = doc.node(button).visual;
    assert_ne!(focused_visual, hover_visual);
    assert_ne!(focused_visual, normal);

    let focused = doc.handle_input(UiInputEvent::Focus(FocusDirection::Next));
    assert_eq!(focused.focused, Some(button));
    assert_eq!(doc.node(button).visual, focused_visual);
}
#[cfg(feature = "widgets")]
#[test]
fn widget_button_options_apply_disabled_accessibility_and_media_hooks() {
    let mut doc = UiDocument::new(root_style(200.0, 80.0));
    let root = doc.root;
    let disabled_visual = UiVisual::panel(ColorRgba::new(10, 11, 12, 180), None, 2.0);
    let button = widgets::button(
        &mut doc,
        root,
        "render",
        "Render",
        widgets::ButtonOptions {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(96.0),
                    height: length(32.0),
                },
                ..Default::default()
            }),
            leading_image: Some(ImageContent::new("icons.render")),
            image_shader: Some(ShaderEffect::new("ui.icon_mask")),
            shader: Some(ShaderEffect::new("ui.disabled")),
            disabled_visual: Some(disabled_visual),
            enabled: false,
            accessibility_hint: Some("Unavailable while exporting".to_string()),
            ..Default::default()
        },
    );

    assert_eq!(doc.node(button).visual, disabled_visual);
    assert_eq!(doc.node(button).shader.as_ref().unwrap().key, "ui.disabled");
    assert!(!doc.node(button).input.pointer);
    assert!(!doc.node(button).input.focusable);

    let accessibility = doc.node(button).accessibility.as_ref().unwrap();
    assert_eq!(accessibility.role, AccessibilityRole::Button);
    assert_eq!(accessibility.label.as_deref(), Some("Render"));
    assert_eq!(
        accessibility.hint.as_deref(),
        Some("Unavailable while exporting")
    );
    assert!(!accessibility.enabled);
    assert!(!accessibility.focusable);

    let image = doc.node(button).children[0];
    assert!(matches!(doc.node(image).content, UiContent::Image(_)));
    assert_eq!(doc.node(image).shader.as_ref().unwrap().key, "ui.icon_mask");
}
#[cfg(feature = "widgets")]
#[test]
fn widget_button_convenience_builders_cover_common_button_modes() {
    let mut doc = UiDocument::new(root_style(360.0, 120.0));
    let root = doc.root;

    let small = widgets::small_button(
        &mut doc,
        root,
        "small",
        "Small",
        widgets::ButtonOptions::default(),
    );
    assert_eq!(
        doc.node(small).style.layout.size.height,
        Dimension::length(28.0)
    );

    let icon = widgets::icon_button(
        &mut doc,
        root,
        "icon",
        ImageContent::new("icons.save"),
        "Save",
        widgets::ButtonOptions::default(),
    );
    assert_eq!(
        doc.node(icon)
            .accessibility
            .as_ref()
            .unwrap()
            .label
            .as_deref(),
        Some("Save")
    );
    assert_eq!(doc.node(icon).children.len(), 1);
    assert!(matches!(
        doc.node(doc.node(icon).children[0]).content,
        UiContent::Image(_)
    ));

    let toggled = widgets::toggle_button(
        &mut doc,
        root,
        "toggle",
        "Pinned",
        true,
        widgets::ButtonOptions::default(),
    );
    assert_eq!(
        doc.node(toggled).accessibility.as_ref().unwrap().pressed,
        Some(true)
    );
    assert!(doc.node(toggled).visual.fill.relative_luminance() < 0.08);

    let reset = widgets::reset_button(
        &mut doc,
        root,
        "reset",
        false,
        widgets::ButtonOptions::default().with_action("style.reset"),
    );
    assert!(!doc.node(reset).input.pointer);
    assert!(!doc.node(reset).accessibility.as_ref().unwrap().enabled);
}
#[cfg(feature = "widgets")]
#[test]
fn widget_button_action_helpers_route_pointer_and_keyboard_activation() {
    let mut doc = UiDocument::new(root_style(200.0, 80.0));
    let root = doc.root;
    let options = widgets::ButtonOptions::new(LayoutStyle::from_taffy_style(Style {
        size: TaffySize {
            width: length(96.0),
            height: length(32.0),
        },
        ..Default::default()
    }))
    .with_action(WidgetActionBinding::action("transport.play"));
    let button = widgets::button(&mut doc, root, "play", "Play", options.clone());
    doc.compute_layout(UiSize::new(200.0, 80.0), &mut ApproxTextMeasurer)
        .expect("layout");

    doc.handle_input(UiInputEvent::PointerDown(UiPoint::new(12.0, 12.0)));
    let pointer_result = doc.handle_input(UiInputEvent::PointerUp(UiPoint::new(12.0, 12.0)));
    let pointer_actions =
        widgets::button_actions_from_input_result(&doc, button, &options, &pointer_result);

    assert_eq!(pointer_actions.len(), 1);
    assert_eq!(pointer_actions.as_slice()[0].target, button);
    assert_eq!(
        pointer_actions.as_slice()[0].kind,
        WidgetActionKind::Activate(WidgetActivation::pointer(1))
    );

    let label = doc.node(button).children[0];
    doc.node_mut(label).input = InputBehavior {
        pointer: true,
        focusable: false,
        keyboard: false,
    };
    let label_rect = doc.node(label).layout.rect;
    let label_point = UiPoint::new(
        label_rect.x + label_rect.width * 0.5,
        label_rect.y + label_rect.height * 0.5,
    );
    doc.handle_input(UiInputEvent::PointerDown(label_point));
    let label_result = doc.handle_input(UiInputEvent::PointerUp(label_point));
    assert_eq!(label_result.clicked, Some(label));
    let label_actions =
        widgets::button_actions_from_input_result(&doc, button, &options, &label_result);
    assert_eq!(label_actions.len(), 1);
    assert_eq!(label_actions.as_slice()[0].target, button);

    let gesture_actions = widgets::button_actions_from_gesture_event(
        &doc,
        button,
        &options,
        &GestureEvent::Click(PointerClick {
            pointer_id: PointerId::MOUSE,
            target: label,
            position: label_point,
            button: PointerButton::Primary,
            count: 2,
            modifiers: KeyModifiers::NONE,
            timestamp_millis: 16,
        }),
    );
    assert_eq!(gesture_actions.len(), 1);
    assert_eq!(gesture_actions.as_slice()[0].target, button);
    assert_eq!(
        gesture_actions.as_slice()[0].kind,
        WidgetActionKind::Activate(WidgetActivation::pointer(2))
    );

    let key_actions = widgets::button_actions_from_key_event(
        &doc,
        button,
        &options,
        &UiInputEvent::Key {
            key: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
        },
    );

    assert_eq!(key_actions.len(), 1);
    assert_eq!(
        key_actions.as_slice()[0].kind,
        WidgetActionKind::Activate(WidgetActivation::keyboard())
    );
}
#[cfg(feature = "widgets")]
#[test]
fn widget_button_action_helpers_suppress_disabled_and_preserve_command_binding() {
    let mut doc = UiDocument::new(root_style(240.0, 80.0));
    let root = doc.root;
    let disabled_options = widgets::ButtonOptions {
        enabled: false,
        action: Some(WidgetActionBinding::action("render.disabled")),
        ..Default::default()
    };
    let disabled = widgets::button(&mut doc, root, "render", "Render", disabled_options.clone());
    let disabled_result = UiInputResult {
        clicked: Some(disabled),
        ..Default::default()
    };

    assert!(widgets::button_actions_from_input_result(
        &doc,
        disabled,
        &disabled_options,
        &disabled_result
    )
    .is_empty());

    let command_options = widgets::ButtonOptions::default().with_command("file.save");
    let save = widgets::button(&mut doc, root, "save", "Save", command_options.clone());
    assert_eq!(
        doc.node(save)
            .action
            .as_ref()
            .and_then(WidgetActionBinding::command_id),
        Some(&CommandId::from("file.save"))
    );
    let save_result = UiInputResult {
        clicked: Some(save),
        ..Default::default()
    };
    let actions =
        widgets::button_actions_from_input_result(&doc, save, &command_options, &save_result);

    assert_eq!(
        actions.as_slice()[0].binding.command_id(),
        Some(&CommandId::from("file.save"))
    );
}
#[cfg(feature = "widgets")]
#[test]
fn widget_checkbox_action_helpers_toggle_selection_from_pointer_and_keyboard() {
    let mut doc = UiDocument::new(root_style(240.0, 80.0));
    let root = doc.root;
    let options =
        widgets::CheckboxOptions::default().with_action(WidgetActionBinding::action("sync"));
    let checkbox = widgets::checkbox(&mut doc, root, "sync", "Sync", false, options.clone());
    doc.focus.focused = Some(checkbox);

    let pointer = widgets::checkbox_actions_from_input_result(
        &doc,
        checkbox,
        false,
        &options,
        &UiInputResult {
            clicked: Some(checkbox),
            ..Default::default()
        },
    );
    assert_eq!(pointer.len(), 1);
    assert_eq!(
        pointer.as_slice()[0].kind,
        WidgetActionKind::Selection(WidgetSelection {
            selected: Some(true)
        })
    );

    let label = doc.node(checkbox).children[1];
    let label_pointer = widgets::checkbox_actions_from_input_result(
        &doc,
        checkbox,
        false,
        &options,
        &UiInputResult {
            clicked: Some(label),
            ..Default::default()
        },
    );
    assert_eq!(label_pointer.len(), 1);
    assert_eq!(label_pointer.as_slice()[0].target, checkbox);

    let keyboard = widgets::checkbox_actions_from_key_event(
        &doc,
        checkbox,
        true,
        &options,
        &UiInputEvent::Key {
            key: KeyCode::Character(' '),
            modifiers: KeyModifiers::NONE,
        },
    );
    assert_eq!(
        keyboard.as_slice()[0].kind,
        WidgetActionKind::Selection(WidgetSelection {
            selected: Some(false)
        })
    );
}
#[cfg(feature = "widgets")]
#[test]
fn widget_action_helpers_preserve_order_and_map_drag_value_edits() {
    let mut doc = UiDocument::new(root_style(320.0, 120.0));
    let root = doc.root;
    let apply_options =
        widgets::ButtonOptions::default().with_action(WidgetActionBinding::action("apply"));
    let apply = widgets::button(&mut doc, root, "apply", "Apply", apply_options.clone());
    let slider_options = widgets::SliderOptions::default()
        .with_drag_action(WidgetActionBinding::action("gain.drag"))
        .with_value_edit_action(WidgetActionBinding::action("gain.edit"));
    let slider = widgets::slider(
        &mut doc,
        root,
        "gain",
        0.5,
        0.0..1.0,
        slider_options.clone(),
    );
    let mut queue = WidgetActionQueue::new();
    let click = UiInputResult {
        clicked: Some(apply),
        ..Default::default()
    };
    let thumb = doc.node(slider).children[1];
    let drag = GestureEvent::Drag(DragGesture {
        pointer_id: PointerId::MOUSE,
        target: thumb,
        phase: GesturePhase::Update,
        origin: UiPoint::new(10.0, 10.0),
        current: UiPoint::new(60.0, 10.0),
        previous: UiPoint::new(40.0, 10.0),
        delta: UiPoint::new(20.0, 0.0),
        total_delta: UiPoint::new(50.0, 0.0),
        button: PointerButton::Primary,
        modifiers: KeyModifiers::NONE,
        captured: true,
        timestamp_millis: 12,
    });

    widgets::push_button_input_result_actions(&mut queue, &doc, apply, &apply_options, &click);
    widgets::push_slider_gesture_event_actions(&mut queue, &doc, slider, &slider_options, &drag);

    assert_eq!(queue.len(), 3);
    assert_eq!(queue.as_slice()[0].target, apply);
    assert_eq!(queue.as_slice()[1].target, slider);
    assert_eq!(queue.as_slice()[2].target, slider);
    assert_eq!(
        queue.as_slice()[1].kind,
        WidgetActionKind::Drag(WidgetDrag {
            phase: WidgetDragPhase::Update,
            origin: UiPoint::new(10.0, 10.0),
            current: UiPoint::new(60.0, 10.0),
            previous: UiPoint::new(40.0, 10.0),
            delta: UiPoint::new(20.0, 0.0),
            total_delta: UiPoint::new(50.0, 0.0),
        })
    );
    assert_eq!(
        queue.as_slice()[2].kind,
        WidgetActionKind::ValueEdit(WidgetValueEditPhase::Update)
    );
}
#[cfg(feature = "widgets")]
#[test]
fn widget_core_controls_export_accessibility_metadata() {
    let mut doc = UiDocument::new(root_style(360.0, 240.0));
    let root = doc.root;
    let title = widgets::label(
        &mut doc,
        root,
        "title",
        "Oscillator",
        TextStyle::default(),
        LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: Dimension::auto(),
                height: Dimension::auto(),
            },
            ..Default::default()
        }),
    );
    let scroll = widgets::scroll_area(
        &mut doc,
        root,
        "modulation_matrix",
        ScrollAxes::BOTH,
        LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: length(160.0),
                height: length(60.0),
            },
            ..Default::default()
        }),
    );
    let checkbox = widgets::checkbox(
        &mut doc,
        root,
        "sync",
        "Hard sync",
        true,
        widgets::CheckboxOptions::default(),
    );
    let slider = widgets::slider(
        &mut doc,
        root,
        "volume",
        0.25,
        0.0..1.0,
        widgets::SliderOptions {
            accessibility_label: Some("Volume".to_string()),
            ..Default::default()
        },
    );
    let input_state = widgets::TextInputState::new("");
    let input = widgets::text_input(
        &mut doc,
        root,
        "preset_name",
        &input_state,
        widgets::TextInputOptions {
            placeholder: "Preset name".to_string(),
            ..Default::default()
        },
    );
    let combo = widgets::combo_box(
        &mut doc,
        root,
        "waveform",
        "Sine",
        true,
        widgets::ComboBoxOptions::default(),
    );

    let tree = doc.accessibility_tree();
    let node = |id| tree.iter().find(|node| node.id == id).unwrap();

    assert_eq!(node(title).role, AccessibilityRole::Label);
    assert_eq!(node(title).label.as_deref(), Some("Oscillator"));
    assert_eq!(node(scroll).role, AccessibilityRole::List);
    assert_eq!(
        node(scroll).value.as_deref(),
        Some("horizontal and vertical")
    );
    assert_eq!(node(checkbox).role, AccessibilityRole::Checkbox);
    assert_eq!(node(checkbox).value.as_deref(), Some("checked"));
    assert_eq!(node(slider).role, AccessibilityRole::Slider);
    assert_eq!(node(slider).label.as_deref(), Some("Volume"));
    assert_eq!(node(slider).value.as_deref(), Some("0.25 (25%)"));
    assert_eq!(node(input).role, AccessibilityRole::TextBox);
    assert_eq!(node(input).hint.as_deref(), Some("Preset name"));
    assert_eq!(node(combo).role, AccessibilityRole::ComboBox);
    assert_eq!(node(combo).value.as_deref(), Some("Sine (open)"));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_edits_and_commits_state() {
    let mut state = widgets::TextInputState::new("gain");
    state.move_caret(widgets::CaretMovement::End, false);
    let outcome = state.handle_event(&UiInputEvent::TextInput("!".to_string()));
    assert!(outcome.changed);
    assert_eq!(state.history.undo_len(), 1);
    assert_eq!(
        outcome
            .transaction
            .as_ref()
            .map(|transaction| transaction.phase),
        Some(EditTransactionPhase::Commit)
    );
    assert_eq!(state.text, "gain!");
    let outcome = state.handle_event(&UiInputEvent::Key {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
    });
    assert!(outcome.committed);
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_records_history_and_keyboard_undo_redo() {
    let mut state = widgets::TextInputState::new("mix");
    let typed = state.handle_event_for_target(
        &UiInputEvent::TextInput("er".to_string()),
        TransactionTarget::widget("preset-name"),
    );

    assert_eq!(state.text, "mixer");
    assert_eq!(state.history.undo_len(), 1);
    let transaction = typed.transaction.as_ref().expect("text transaction");
    assert_eq!(transaction.payload.before, "mix");
    assert_eq!(transaction.payload.after, "mixer");
    assert_eq!(transaction.target, TransactionTarget::widget("preset-name"));

    let undo = state.handle_event(&UiInputEvent::Key {
        key: KeyCode::Character('z'),
        modifiers: KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        },
    });
    assert!(undo.changed);
    assert_eq!(state.text, "mix");
    assert_eq!(
        undo.history_apply.as_ref().map(|apply| apply.direction),
        Some(TextEditHistoryDirection::Undo)
    );
    assert_eq!(state.history.undo_len(), 0);
    assert_eq!(state.history.redo_len(), 1);

    let redo = state.handle_event(&UiInputEvent::Key {
        key: KeyCode::Character('z'),
        modifiers: KeyModifiers {
            ctrl: true,
            shift: true,
            ..KeyModifiers::NONE
        },
    });
    assert!(redo.changed);
    assert_eq!(state.text, "mixer");
    assert_eq!(
        redo.history_apply.as_ref().map(|apply| apply.direction),
        Some(TextEditHistoryDirection::Redo)
    );
    assert_eq!(state.history.undo_len(), 1);
    assert_eq!(state.history.redo_len(), 0);
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_supports_clipboard_edit_primitives() {
    let mut state = widgets::TextInputState::new("wet dry");
    state.move_caret(widgets::CaretMovement::Start, false);
    state.move_caret(widgets::CaretMovement::Right, true);
    state.move_caret(widgets::CaretMovement::Right, true);
    state.move_caret(widgets::CaretMovement::Right, true);

    assert_eq!(state.copy_selection().as_deref(), Some("wet"));
    assert_eq!(state.cut_selection().as_deref(), Some("wet"));
    assert_eq!(state.text, " dry");
    state.paste_text("very\nwet");
    assert_eq!(state.text, "very wet dry");
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_reports_clipboard_key_commands_and_sanitizes_paste() {
    let mut state = widgets::TextInputState::new("café");
    state.caret = 4;
    state.selection_anchor = Some(0);
    assert_eq!(state.copy_selection().as_deref(), Some("caf"));

    state.select_all();
    let copy = state.handle_event(&UiInputEvent::Key {
        key: KeyCode::Character('c'),
        modifiers: KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        },
    });
    assert_eq!(
        copy.clipboard,
        Some(widgets::TextInputClipboardAction::Copy("café".to_string()))
    );
    assert!(!copy.changed);

    let cut = state.handle_event(&UiInputEvent::Key {
        key: KeyCode::Character('x'),
        modifiers: KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        },
    });
    assert_eq!(
        cut.clipboard,
        Some(widgets::TextInputClipboardAction::Cut("café".to_string()))
    );
    assert!(cut.changed);
    assert_eq!(state.text, "");

    let paste_request = state.handle_event(&UiInputEvent::Key {
        key: KeyCode::Character('v'),
        modifiers: KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        },
    });
    assert_eq!(
        paste_request.clipboard,
        Some(widgets::TextInputClipboardAction::Paste)
    );
    assert!(!paste_request.changed);

    let paste = state.paste_text_with_outcome("dry\r\nwet\n");
    assert!(paste.changed);
    assert_eq!(state.text, "dry wet ");

    let mut multiline = widgets::TextInputState::new("").multiline(true);
    multiline.paste_text("a\r\nb\rc");
    assert_eq!(multiline.text, "a\nb\nc");
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_policy_supports_read_only_and_non_selectable_text() {
    let mut state = widgets::TextInputState::new("locked");
    state.select_all();
    let policy = widgets::TextInputInteractionPolicy::read_only();

    let copy = state.handle_event_with_policy(
        &UiInputEvent::Key {
            key: KeyCode::Character('c'),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
        },
        policy,
    );
    assert_eq!(
        copy.clipboard,
        Some(widgets::TextInputClipboardAction::Copy(
            "locked".to_string()
        ))
    );

    let cut = state.handle_event_with_policy(
        &UiInputEvent::Key {
            key: KeyCode::Character('x'),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
        },
        policy,
    );
    assert_eq!(cut.clipboard, None);
    assert_eq!(state.text, "locked");

    let typed = state.handle_event_with_policy(&UiInputEvent::TextInput("!".to_string()), policy);
    assert!(!typed.changed);
    assert_eq!(state.text, "locked");

    let ime = state.apply_ime_response_with_policy(
        &platform::TextImeResponse::Commit {
            input: platform::TextInputId::new("field"),
            text: "!".to_string(),
        },
        policy,
    );
    assert!(!ime.changed);
    assert_eq!(state.text, "locked");

    state.caret = 0;
    state.clear_selection();
    state.handle_event_with_policy(
        &UiInputEvent::Key {
            key: KeyCode::ArrowRight,
            modifiers: KeyModifiers {
                shift: true,
                ..KeyModifiers::NONE
            },
        },
        policy,
    );
    assert_eq!(state.selected_text(), Some("l"));

    let non_selectable = widgets::TextInputInteractionPolicy {
        selectable: false,
        ..policy
    };
    let copy = state.handle_event_with_policy(
        &UiInputEvent::Key {
            key: KeyCode::Character('c'),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
        },
        non_selectable,
    );
    assert_eq!(copy.clipboard, None);
    assert_eq!(state.selected_range(), None);
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_maps_clipboard_and_ime_platform_contracts() {
    let mut state = widgets::TextInputState::new("scale").multiline(true);
    state.caret = 2;
    state.selection_anchor = Some(0);
    let metrics =
        widgets::TextInputLayoutMetrics::new(UiRect::new(10.0, 20.0, 180.0, 40.0), 8.0, 18.0)
            .caret_width(2.0);
    let context =
        widgets::TextInputPlatformContext::for_node(UiNodeId(7), state.caret_rect(metrics));

    let session = state.ime_session(context.clone());
    assert_eq!(context.target, Some(UiNodeId(7)));
    assert_eq!(session.input, text_input_id_for_node(UiNodeId(7)));
    assert_eq!(
        session.cursor_rect,
        platform::LogicalRect::new(26.0, 20.0, 2.0, 18.0)
    );
    assert_eq!(session.surrounding_text, "scale");
    assert_eq!(session.selection, platform::TextRange::new(0, 2));
    assert!(session.multiline);
    assert_eq!(
        state.activate_ime_request(context.clone()),
        platform::TextImeRequest::Activate(session.clone())
    );
    assert_eq!(
        state.update_ime_request(context.clone()),
        platform::TextImeRequest::Update(session)
    );
    assert_eq!(
        widgets::TextInputState::deactivate_ime_request(context.input.clone()),
        platform::TextImeRequest::Deactivate {
            input: context.input.clone()
        }
    );
    assert_eq!(
        widgets::TextInputState::show_keyboard_request(context.input.clone()),
        platform::TextImeRequest::ShowKeyboard {
            input: context.input.clone()
        }
    );
    assert_eq!(
        widgets::TextInputState::hide_keyboard_request(context.input.clone()),
        platform::TextImeRequest::HideKeyboard {
            input: context.input.clone()
        }
    );

    let copy = state.handle_event(&UiInputEvent::Key {
        key: KeyCode::Character('c'),
        modifiers: KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        },
    });
    assert_eq!(
        copy.clipboard_request(),
        Some(platform::ClipboardRequest::WriteText("sc".to_string()))
    );
    let paste = state.handle_event(&UiInputEvent::Key {
        key: KeyCode::Character('v'),
        modifiers: KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        },
    });
    assert_eq!(
        paste.clipboard_request(),
        Some(platform::ClipboardRequest::ReadText)
    );
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_routes_focus_edits_clipboard_and_ime_requests() {
    let mut doc = UiDocument::new(root_style(320.0, 120.0));
    let root = doc.root;
    let mut state = widgets::TextInputState::new("");
    let input = widgets::text_input(
        &mut doc,
        root,
        "recipe",
        &state,
        widgets::TextInputOptions::default(),
    );
    doc.add_child(
        root,
        UiNode::container("apply", button_style(80.0, 32.0)).with_input(InputBehavior::BUTTON),
    );
    doc.compute_layout(UiSize::new(320.0, 120.0), &mut ApproxTextMeasurer)
        .expect("layout");
    let metrics =
        widgets::TextInputLayoutMetrics::new(UiRect::new(0.0, 0.0, 180.0, 30.0), 8.0, 18.0);
    let context = widgets::TextInputPlatformContext::for_node(input, state.caret_rect(metrics));

    let ignored = widgets::handle_text_input_event(
        &mut doc,
        input,
        &mut state,
        UiInputEvent::TextInput("A".to_string()),
        Some(context.clone()),
    );
    assert_eq!(state.text, "");
    assert!(ignored.edit.is_none());
    assert!(ignored.platform_requests.is_empty());

    let focus = widgets::handle_text_input_event(
        &mut doc,
        input,
        &mut state,
        UiInputEvent::PointerDown(UiPoint::new(10.0, 10.0)),
        Some(context.clone()),
    );
    assert!(focus.focused);
    assert_eq!(focus.input.focused, Some(input));
    assert!(matches!(
        &focus.platform_requests[..],
        [
            platform::PlatformRequest::TextIme(platform::TextImeRequest::Activate(session)),
            platform::PlatformRequest::TextIme(platform::TextImeRequest::ShowKeyboard { input: keyboard_input }),
        ] if session.input == context.input && *keyboard_input == context.input
    ));

    let typed = widgets::handle_text_input_event(
        &mut doc,
        input,
        &mut state,
        UiInputEvent::TextInput("AB".to_string()),
        Some(context.clone()),
    );
    assert_eq!(state.text, "AB");
    assert!(typed.did_edit());
    assert!(matches!(
        typed.platform_requests.last(),
        Some(platform::PlatformRequest::TextIme(platform::TextImeRequest::Update(session)))
            if session.surrounding_text == "AB"
    ));

    state.select_all();
    let copy = widgets::handle_text_input_event(
        &mut doc,
        input,
        &mut state,
        UiInputEvent::Key {
            key: KeyCode::Character('c'),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
        },
        Some(context.clone()),
    );
    assert_eq!(
        copy.platform_requests,
        vec![platform::PlatformRequest::Clipboard(
            platform::ClipboardRequest::WriteText("AB".to_string())
        )]
    );

    let commit = widgets::handle_text_input_event(
        &mut doc,
        input,
        &mut state,
        UiInputEvent::Key {
            key: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
        },
        Some(context.clone()),
    );
    assert!(commit.committed());
    assert!(matches!(
        &commit.platform_requests[..],
        [
            platform::PlatformRequest::TextIme(platform::TextImeRequest::HideKeyboard { input: keyboard_input }),
            platform::PlatformRequest::TextIme(platform::TextImeRequest::Deactivate { input: deactivated_input }),
        ] if *keyboard_input == context.input && *deactivated_input == context.input
    ));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_options_enforce_read_only_selection_and_clipboard() {
    let mut doc = UiDocument::new(root_style(320.0, 120.0));
    let root = doc.root;
    let mut state = widgets::TextInputState::new("AB");
    let options = widgets::TextInputOptions::default()
        .read_only()
        .with_edit_action(WidgetActionBinding::action("input.edit"));
    let input = widgets::text_input(&mut doc, root, "serial", &state, options.clone());
    doc.compute_layout(UiSize::new(320.0, 120.0), &mut ApproxTextMeasurer)
        .expect("layout");
    let context = widgets::TextInputPlatformContext::for_node(
        input,
        state.caret_rect(widgets::TextInputLayoutMetrics::new(
            UiRect::new(0.0, 0.0, 180.0, 30.0),
            8.0,
            18.0,
        )),
    );

    let accessibility = doc.node(input).accessibility.as_ref().unwrap();
    assert!(accessibility.read_only);
    assert!(accessibility.focusable);
    assert!(accessibility
        .actions
        .iter()
        .any(|action| action.id == "copy"));
    assert!(!accessibility
        .actions
        .iter()
        .any(|action| action.id == "cut"));
    assert!(!accessibility
        .actions
        .iter()
        .any(|action| action.id == "paste"));

    let focus = widgets::handle_text_input_event_with_options(
        &mut doc,
        input,
        &mut state,
        &options,
        UiInputEvent::PointerDown(UiPoint::new(10.0, 10.0)),
        Some(context.clone()),
    );
    assert!(focus.focused);
    assert!(focus.platform_requests.is_empty());

    let typed = widgets::handle_text_input_event_with_options(
        &mut doc,
        input,
        &mut state,
        &options,
        UiInputEvent::TextInput("!".to_string()),
        Some(context.clone()),
    );
    assert_eq!(state.text, "AB");
    assert!(!typed.did_edit());
    assert_eq!(
        widgets::text_input_actions_from_outcome(
            &doc,
            input,
            &options,
            typed.edit.as_ref().unwrap()
        )
        .len(),
        0
    );

    state.select_all();
    let copy = widgets::handle_text_input_event_with_options(
        &mut doc,
        input,
        &mut state,
        &options,
        UiInputEvent::Key {
            key: KeyCode::Character('c'),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
        },
        Some(context),
    );
    assert_eq!(
        copy.platform_requests,
        vec![platform::PlatformRequest::Clipboard(
            platform::ClipboardRequest::WriteText("AB".to_string())
        )]
    );

    let mut non_selectable_doc = UiDocument::new(root_style(320.0, 120.0));
    let non_selectable_root = non_selectable_doc.root;
    let locked = widgets::text_input(
        &mut non_selectable_doc,
        non_selectable_root,
        "locked",
        &state,
        options.clone().selectable(false),
    );
    let locked_node = non_selectable_doc.node(locked);
    assert!(!locked_node.input.pointer);
    assert!(!locked_node.input.focusable);
    let locked_accessibility = locked_node.accessibility.as_ref().unwrap();
    assert!(locked_accessibility.read_only);
    assert!(!locked_accessibility.focusable);
    assert!(!locked_accessibility
        .actions
        .iter()
        .any(|action| action.id == "copy"));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_selectable_text_wraps_read_only_copyable_input() {
    let mut doc = UiDocument::new(root_style(320.0, 120.0));
    let root = doc.root;
    let mut state = widgets::TextInputState::new("Reference");
    let options = widgets::TextInputOptions::default()
        .with_edit_action(WidgetActionBinding::action("should.not.emit"));

    let selectable = widgets::selectable_text(&mut doc, root, "reference", &state, options.clone());
    doc.compute_layout(UiSize::new(320.0, 120.0), &mut ApproxTextMeasurer)
        .expect("layout");
    let accessibility = doc.node(selectable).accessibility.as_ref().unwrap();
    assert!(accessibility.read_only);
    assert!(accessibility.focusable);
    assert!(accessibility
        .actions
        .iter()
        .any(|action| action.id == "copy"));
    assert!(!accessibility
        .actions
        .iter()
        .any(|action| action.id == "cut"));
    assert!(!accessibility
        .actions
        .iter()
        .any(|action| action.id == "paste"));

    let focus = widgets::handle_selectable_text_event(
        &mut doc,
        selectable,
        &mut state,
        &options,
        UiInputEvent::PointerDown(UiPoint::new(10.0, 10.0)),
        None,
    );
    assert!(focus.focused);
    assert!(focus.platform_requests.is_empty());

    let typed = widgets::handle_selectable_text_event(
        &mut doc,
        selectable,
        &mut state,
        &options,
        UiInputEvent::TextInput("!".to_string()),
        None,
    );
    assert_eq!(state.text, "Reference");
    assert!(!typed.did_edit());

    state.select_all();
    let copy = widgets::handle_selectable_text_event(
        &mut doc,
        selectable,
        &mut state,
        &options,
        UiInputEvent::Key {
            key: KeyCode::Character('c'),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::NONE
            },
        },
        None,
    );
    assert_eq!(
        copy.platform_requests,
        vec![platform::PlatformRequest::Clipboard(
            platform::ClipboardRequest::WriteText("Reference".to_string())
        )]
    );
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_applies_ime_commit_preedit_and_delete_responses() {
    let input = platform::TextInputId::new("field");
    let mut state = widgets::TextInputState::new("abcd");
    state.caret = 2;

    let preedit = state.apply_ime_response(&platform::TextImeResponse::Preedit {
        input: input.clone(),
        text: "候".to_string(),
        selection: Some(platform::TextRange::caret(1)),
    });
    assert!(!preedit.changed);
    assert_eq!(state.composing.as_deref(), Some("候"));

    let commit = state.apply_ime_response(&platform::TextImeResponse::Commit {
        input: input.clone(),
        text: "X".to_string(),
    });
    assert!(commit.changed);
    assert_eq!(state.text, "abXcd");
    assert_eq!(state.caret, 3);
    assert_eq!(state.composing, None);

    let delete = state.apply_ime_response(&platform::TextImeResponse::DeleteSurrounding {
        input: input.clone(),
        before_chars: 1,
        after_chars: 1,
    });
    assert!(delete.changed);
    assert_eq!(state.text, "abd");
    assert_eq!(state.caret, 2);

    state.composing = Some("x".to_string());
    state.apply_ime_response(&platform::TextImeResponse::Deactivated { input });
    assert_eq!(state.composing, None);
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_ignores_ime_responses_for_other_inputs() {
    let input = platform::TextInputId::new("field");
    let other = platform::TextInputId::new("other");
    let mut state = widgets::TextInputState::new("abcd");
    state.caret = 2;
    state.composing = Some("候".to_string());

    let ignored = state.apply_ime_response_for_input(
        &input,
        &platform::TextImeResponse::Commit {
            input: other.clone(),
            text: "X".to_string(),
        },
    );
    assert_eq!(ignored, None);
    assert_eq!(state.text, "abcd");
    assert_eq!(state.caret, 2);
    assert_eq!(state.composing.as_deref(), Some("候"));

    let applied = state
        .apply_ime_response_for_input(
            &input,
            &platform::TextImeResponse::Commit {
                input: input.clone(),
                text: "Y".to_string(),
            },
        )
        .expect("matching input response");
    assert!(applied.changed);
    assert_eq!(state.text, "abYcd");
    assert_eq!(state.composing, None);

    let deactivated = state.apply_ime_response_for_input(
        &input,
        &platform::TextImeResponse::Deactivated { input: other },
    );
    assert_eq!(deactivated, None);
    assert_eq!(state.text, "abYcd");
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_reports_selection_and_caret_line_metadata() {
    let mut state = widgets::TextInputState::new("alpha\nbéta\nomega").multiline(true);
    state.caret = "alpha\nbé".len();
    state.selection_anchor = Some("alpha\n".len());

    assert_eq!(state.selected_text(), Some("bé"));
    assert_eq!(
        state.selected_range(),
        Some("alpha\n".len().."alpha\nbé".len())
    );

    let info = state.caret_info();
    assert_eq!(
        info.position,
        widgets::TextInputPosition {
            byte_index: "alpha\nbé".len(),
            line: 1,
            column: 2,
        }
    );
    assert_eq!(info.line_range, "alpha\n".len().."alpha\nbéta".len());
    assert_eq!(info.selected_range, state.selected_range());
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_builds_caret_selection_and_scene_paint_plan() {
    let mut state = widgets::TextInputState::new("one\ntwo").multiline(true);
    state.selection_anchor = Some(1);
    state.caret = "one\nt".len();
    let style = TextStyle {
        font_size: 10.0,
        line_height: 14.0,
        ..Default::default()
    };
    let metrics =
        widgets::TextInputLayoutMetrics::from_style(UiRect::new(4.0, 6.0, 120.0, 40.0), &style)
            .caret_width(2.0);

    let caret = state.caret_rect(metrics);
    assert_eq!(caret.rect, UiRect::new(7.6000004, 6.0 + 14.0, 2.0, 14.0));
    assert_eq!(
        caret.position,
        widgets::TextInputPosition {
            byte_index: "one\nt".len(),
            line: 1,
            column: 1,
        }
    );

    let selection = state.selection_rects(metrics);
    assert_eq!(selection.len(), 2);
    assert_eq!(selection[0].byte_range, 1.."one".len());
    assert_eq!(selection[0].rect, UiRect::new(9.0, 6.0, 10.0, 14.0));
    assert_eq!(selection[1].byte_range, "one\n".len().."one\nt".len());
    assert_eq!(selection[1].rect, UiRect::new(4.0, 20.0, 3.6000001, 14.0));

    let plan = state.render_plan(metrics, style, widgets::TextInputPaintOptions::default());
    assert_eq!(plan.selection_rects, selection);
    assert_eq!(plan.caret, Some(caret));
    assert_eq!(plan.overlay_primitives().len(), 3);
    assert_eq!(plan.scene_primitives().len(), 4);
    assert!(matches!(
        &plan.scene_primitives()[2],
        ScenePrimitive::Text(text) if text.text == "one\ntwo"
    ));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_default_render_uses_scene_caret_at_text_end() {
    let mut doc = UiDocument::new(root_style(240.0, 80.0));
    let root = doc.root;
    let state = widgets::TextInputState::new("gain");
    let input = widgets::text_input(
        &mut doc,
        root,
        "gain",
        &state,
        widgets::TextInputOptions {
            focused: true,
            ..Default::default()
        },
    );

    let text_layer = doc.node(input).children[0];
    let UiContent::Scene(primitives) = &doc.node(text_layer).content else {
        panic!("text input should render text, selection, and caret through a scene");
    };

    assert!(matches!(
        &primitives[0],
        ScenePrimitive::Text(text) if text.text == "gain"
    ));
    assert!(matches!(
        primitives.last(),
        Some(ScenePrimitive::Rect(rect))
            if rect.rect.x == 6.0 + TextStyle::default().font_size * 1.78
                && rect.rect.width == 1.0
    ));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_convenience_builders_configure_common_modes() {
    let mut doc = UiDocument::new(root_style(720.0, 320.0));
    let root = doc.root;

    let multiline_state = widgets::TextInputState::new("one\ntwo");
    let area = widgets::text_area(
        &mut doc,
        root,
        "notes",
        &multiline_state,
        widgets::TextInputOptions::default(),
    );
    assert_eq!(
        doc.node(area).style.layout.size.height,
        Dimension::length(120.0)
    );

    let code = widgets::code_editor(
        &mut doc,
        root,
        "code",
        &widgets::TextInputState::new("let answer = 42;"),
        widgets::TextInputOptions::default(),
    );
    assert_eq!(
        doc.node(code).style.layout.size.width,
        Dimension::length(360.0)
    );
    let code_text = doc.node(code).children[0];
    let UiContent::Scene(primitives) = &doc.node(code_text).content else {
        panic!("code editor should render through a scene");
    };
    assert!(matches!(
        &primitives[0],
        ScenePrimitive::Text(text) if text.style.family == FontFamily::Monospace
    ));

    let search = widgets::search_input(
        &mut doc,
        root,
        "find",
        &widgets::TextInputState::new(""),
        widgets::TextInputOptions::default(),
    );
    assert_eq!(
        doc.node(search).accessibility.as_ref().unwrap().role,
        AccessibilityRole::SearchBox
    );

    let mut password_state = widgets::TextInputState::new("secret");
    password_state.caret = 3;
    let password = widgets::password_input(
        &mut doc,
        root,
        "password",
        &password_state,
        widgets::TextInputOptions {
            focused: true,
            ..Default::default()
        },
    );
    assert_eq!(
        doc.node(password)
            .accessibility
            .as_ref()
            .unwrap()
            .value
            .as_deref(),
        Some("******")
    );
    let password_text = doc.node(password).children[0];
    let UiContent::Scene(primitives) = &doc.node(password_text).content else {
        panic!("password input should render masked text through a scene");
    };
    assert!(matches!(
        &primitives[0],
        ScenePrimitive::Text(text) if text.text == "******"
    ));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_accessibility_summarizes_caret_and_selection() {
    let mut doc = UiDocument::new(root_style(240.0, 80.0));
    let root = doc.root;
    let mut state = widgets::TextInputState::new("alpha").multiline(false);
    state.caret = 3;
    state.selection_anchor = Some(1);

    let input = widgets::text_input(
        &mut doc,
        root,
        "name",
        &state,
        widgets::TextInputOptions::default(),
    );
    let summary = doc
        .node(input)
        .accessibility
        .as_ref()
        .and_then(|meta| meta.summary.as_ref())
        .expect("summary");
    let text = summary.screen_reader_text();
    assert!(text.contains("name caret"));
    assert!(text.contains("Line: 1"));
    assert!(text.contains("Column: 4"));
    assert!(text.contains("Selection: bytes 1 to 3"));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_supports_multiline_line_caret_movement() {
    let mut state = widgets::TextInputState::new("one\nfour\nsix").multiline(true);
    state.caret = "one\nfo".len();

    state.move_caret(widgets::CaretMovement::LineStart, false);
    assert_eq!(state.caret, "one\n".len());

    state.move_caret(widgets::CaretMovement::LineEnd, false);
    assert_eq!(state.caret, "one\nfour".len());

    state.move_caret(widgets::CaretMovement::Up, false);
    assert_eq!(state.caret, "one".len());

    state.move_caret(widgets::CaretMovement::Down, false);
    assert_eq!(state.caret, "one\nfou".len());

    let movement = state.handle_event(&UiInputEvent::Key {
        key: KeyCode::ArrowDown,
        modifiers: KeyModifiers {
            shift: true,
            ..KeyModifiers::NONE
        },
    });
    assert!(!movement.changed);
    assert_eq!(state.caret, "one\nfour\nsix".len());
    assert_eq!(state.selected_text(), Some("r\nsix"));

    state.handle_event(&UiInputEvent::Key {
        key: KeyCode::Home,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(state.caret, "one\nfour\n".len());
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_maps_pointer_points_to_caret_and_selection() {
    let metrics =
        widgets::TextInputLayoutMetrics::new(UiRect::new(10.0, 20.0, 120.0, 48.0), 8.0, 16.0);
    let mut state = widgets::TextInputState::new("alpha\nbeta").multiline(true);

    assert_eq!(
        state.position_at_point(metrics, UiPoint::new(10.0 + 2.6 * 8.0, 20.0 + 18.0)),
        widgets::TextInputPosition {
            byte_index: "alpha\nbet".len(),
            line: 1,
            column: 3,
        }
    );
    assert_eq!(
        state.byte_index_at_point(metrics, UiPoint::new(-20.0, -10.0)),
        0
    );
    assert_eq!(
        state.byte_index_at_point(metrics, UiPoint::new(240.0, 240.0)),
        "alpha\nbeta".len()
    );

    state.move_caret_to_point(metrics, UiPoint::new(10.0 + 2.0 * 8.0, 20.0), false);
    assert_eq!(state.caret, "al".len());
    assert_eq!(state.selection_anchor, None);

    state.move_caret_to_point(metrics, UiPoint::new(10.0 + 4.0 * 8.0, 20.0 + 16.0), true);
    assert_eq!(state.caret, "alpha\nbeta".len());
    assert_eq!(state.selected_text(), Some("pha\nbeta"));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_event_handler_places_caret_from_pointer_metrics() {
    let mut doc = UiDocument::new(root_style(320.0, 120.0));
    let root = doc.root;
    let mut state = widgets::TextInputState::new("abcdef");
    state.caret = 0;
    let input = widgets::text_input(
        &mut doc,
        root,
        "name",
        &state,
        widgets::TextInputOptions::default(),
    );
    doc.compute_layout(UiSize::new(320.0, 120.0), &mut ApproxTextMeasurer)
        .expect("layout");

    let metrics =
        widgets::TextInputLayoutMetrics::new(UiRect::new(0.0, 0.0, 180.0, 30.0), 8.0, 18.0);
    let context = widgets::TextInputPlatformContext::for_node(input, state.caret_rect(metrics));
    let focused = widgets::handle_text_input_event_with_metrics(
        &mut doc,
        input,
        &mut state,
        UiInputEvent::PointerDown(UiPoint::new(26.0, 8.0)),
        Some(context.clone()),
        Some(metrics),
    );

    assert!(focused.focused);
    assert_eq!(state.caret, 3);
    assert_eq!(state.selection_anchor, None);
    assert!(matches!(
        &focused.platform_requests[..],
        [
            platform::PlatformRequest::TextIme(platform::TextImeRequest::Activate(session)),
            platform::PlatformRequest::TextIme(platform::TextImeRequest::ShowKeyboard { input: keyboard_input }),
            platform::PlatformRequest::TextIme(platform::TextImeRequest::Update(update)),
        ] if session.selection == platform::TextRange::caret(3)
            && session.cursor_rect.origin.x == 24.0
            && *keyboard_input == context.input
            && update.selection == platform::TextRange::caret(3)
    ));

    let selected = widgets::handle_text_input_event_with_metrics(
        &mut doc,
        input,
        &mut state,
        UiInputEvent::PointerMove(UiPoint::new(50.0, 8.0)),
        Some(context),
        Some(metrics),
    );
    assert!(selected.focused);
    assert_eq!(state.caret, 6);
    assert_eq!(state.selected_text(), Some("def"));
    assert!(matches!(
        selected.platform_requests.last(),
        Some(platform::PlatformRequest::TextIme(platform::TextImeRequest::Update(update)))
            if update.selection == platform::TextRange::new(3, 6)
    ));
}
#[cfg(feature = "widgets")]
#[test]
fn widget_text_input_event_handler_derives_pointer_metrics_from_rendered_text() {
    let mut doc = UiDocument::new(root_style(320.0, 120.0));
    let root = doc.root;
    let mut state = widgets::TextInputState::new("abcdef");
    state.caret = 0;
    let input = widgets::text_input(
        &mut doc,
        root,
        "name",
        &state,
        widgets::TextInputOptions::default(),
    );
    doc.compute_layout(UiSize::new(320.0, 120.0), &mut ApproxTextMeasurer)
        .expect("layout");

    let text_rect = doc.node(doc.node(input).children[0]).layout.rect;
    let char_width = TextStyle::default().font_size * 0.50;
    let text_inset = 6.0;
    let context = widgets::TextInputPlatformContext::for_node(
        input,
        widgets::TextInputCaretRect {
            position: state.caret_position(),
            rect: UiRect::new(0.0, 0.0, 1.0, TextStyle::default().line_height),
        },
    );
    let focused = widgets::handle_text_input_event(
        &mut doc,
        input,
        &mut state,
        UiInputEvent::PointerDown(UiPoint::new(
            text_rect.x + text_inset + 3.1 * char_width,
            text_rect.y + 2.0,
        )),
        Some(context),
    );

    assert!(focused.focused);
    assert_eq!(state.caret, 3);
    assert!(matches!(
        focused.platform_requests.last(),
        Some(platform::PlatformRequest::TextIme(platform::TextImeRequest::Update(update)))
            if update.selection == platform::TextRange::caret(3)
                && update.cursor_rect.origin.x == text_rect.x + text_inset + 3.0 * char_width
    ));
}
#[cfg(feature = "widgets")]
#[test]
fn virtual_list_builds_only_visible_rows_with_spacers() {
    let mut doc = UiDocument::new(root_style(300.0, 200.0));
    let root = doc.root;
    let list = widgets::virtual_list(
        &mut doc,
        root,
        "events",
        widgets::VirtualListSpec {
            row_count: 100,
            row_height: 20.0,
            viewport_height: 60.0,
            scroll_offset: 200.0,
            overscan: 1,
        },
        |document, parent, row| {
            document.add_child(
                parent,
                UiNode::text(
                    format!("row.{row}"),
                    format!("Event {row}"),
                    TextStyle::default(),
                    LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: length(20.0),
                        },
                        ..Default::default()
                    }),
                )
                .with_input(InputBehavior::BUTTON),
            );
        },
    );
    doc.compute_layout(UiSize::new(300.0, 200.0), &mut ApproxTextMeasurer)
        .expect("layout");

    assert_eq!(doc.node(list).children.len(), 8);
    assert_eq!(doc.scroll_state(list).unwrap().content_size.height, 2000.0);
}
#[cfg(feature = "widgets")]
#[test]
fn widget_table_virtual_list_and_scrollbar_helpers_expose_metadata() {
    let mut doc = UiDocument::new(root_style(300.0, 200.0));
    let root = doc.root;
    let header = widgets::table_header(
        &mut doc,
        root,
        "events.header",
        &[
            widgets::TableColumn {
                id: "time".to_string(),
                label: "Time".to_string(),
                width: 80.0,
            },
            widgets::TableColumn {
                id: "name".to_string(),
                label: "Name".to_string(),
                width: 160.0,
            },
        ],
    );
    let list = widgets::virtual_list(
        &mut doc,
        root,
        "events",
        widgets::VirtualListSpec {
            row_count: 25,
            row_height: 20.0,
            viewport_height: 60.0,
            scroll_offset: 40.0,
            overscan: 0,
        },
        |document, parent, row| {
            document.add_child(
                parent,
                UiNode::text(
                    format!("row.{row}"),
                    format!("Event {row}"),
                    TextStyle::default(),
                    LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: length(20.0),
                        },
                        ..Default::default()
                    }),
                ),
            );
        },
    );

    let tree = doc.accessibility_tree();
    let header_node = tree.iter().find(|node| node.id == header).unwrap();
    let list_node = tree.iter().find(|node| node.id == list).unwrap();
    assert_eq!(header_node.role, AccessibilityRole::Grid);
    assert_eq!(header_node.value.as_deref(), Some("2 columns"));
    assert_eq!(list_node.role, AccessibilityRole::List);
    assert_eq!(list_node.value.as_deref(), Some("25 items"));
    assert!(tree.iter().any(|node| {
        node.role == AccessibilityRole::GridCell && node.label.as_deref() == Some("Time")
    }));

    let scroll = ScrollState {
        axes: ScrollAxes::VERTICAL,
        offset: UiPoint::new(0.0, 999.0),
        viewport_size: UiSize::new(10.0, 100.0),
        content_size: UiSize::new(10.0, 300.0),
    };
    let thumb = widgets::scrollbar_thumb(
        scroll,
        UiRect::new(0.0, 0.0, 10.0, 100.0),
        widgets::scrollbar::ScrollAxis::Vertical,
    );
    assert!((thumb.y - 66.66667).abs() < 0.01, "{thumb:?}");
    assert!((thumb.height - 33.33333).abs() < 0.01, "{thumb:?}");

    let accessibility = widgets::scrollbar_accessibility(
        "Events scrollbar",
        scroll,
        widgets::scrollbar::ScrollAxis::Vertical,
    );
    assert_eq!(accessibility.role, AccessibilityRole::Slider);
    assert_eq!(accessibility.value.as_deref(), Some("100%"));
    assert!(accessibility.focusable);

    let disabled_accessibility = widgets::scrollbar_accessibility(
        "Empty scrollbar",
        ScrollState {
            axes: ScrollAxes::VERTICAL,
            offset: UiPoint::new(0.0, 0.0),
            viewport_size: UiSize::new(10.0, 100.0),
            content_size: UiSize::new(10.0, 100.0),
        },
        widgets::scrollbar::ScrollAxis::Vertical,
    );
    assert!(!disabled_accessibility.enabled);
    assert!(!disabled_accessibility.focusable);
}
#[cfg(feature = "widgets")]
#[test]
fn widget_scrollbar_drag_state_maps_pointer_delta_to_scroll_offsets() {
    let vertical = ScrollState {
        axes: ScrollAxes::VERTICAL,
        offset: UiPoint::new(0.0, 60.0),
        viewport_size: UiSize::new(10.0, 100.0),
        content_size: UiSize::new(10.0, 400.0),
    };
    let track = UiRect::new(0.0, 0.0, 10.0, 100.0);
    let drag = widgets::ScrollbarDragState::new(
        vertical,
        track,
        widgets::scrollbar::ScrollAxis::Vertical,
        UiPoint::new(5.0, 20.0),
    )
    .expect("vertical drag");

    assert_eq!(drag.thumb, UiRect::new(0.0, 15.0, 10.0, 25.0));
    assert_eq!(
        drag.offset_for_pointer(UiPoint::new(5.0, 50.0)),
        UiPoint::new(0.0, 180.0)
    );
    assert_eq!(
        drag.scroll_state_for_pointer(vertical, UiPoint::new(5.0, 200.0))
            .offset,
        UiPoint::new(0.0, 300.0)
    );

    let horizontal = ScrollState {
        axes: ScrollAxes::HORIZONTAL,
        offset: UiPoint::new(30.0, 0.0),
        viewport_size: UiSize::new(50.0, 10.0),
        content_size: UiSize::new(200.0, 10.0),
    };
    let drag = widgets::ScrollbarDragState::new(
        horizontal,
        UiRect::new(0.0, 0.0, 100.0, 10.0),
        widgets::scrollbar::ScrollAxis::Horizontal,
        UiPoint::new(20.0, 5.0),
    )
    .expect("horizontal drag");
    let offset = drag.offset_for_pointer(UiPoint::new(60.0, 5.0));
    assert!((offset.x - 110.0).abs() < 0.01, "{offset:?}");
    assert_eq!(offset.y, 0.0);

    assert!(widgets::ScrollbarDragState::new(
        ScrollState {
            axes: ScrollAxes::VERTICAL,
            offset: UiPoint::new(0.0, 0.0),
            viewport_size: UiSize::new(10.0, 100.0),
            content_size: UiSize::new(10.0, 100.0),
        },
        track,
        widgets::scrollbar::ScrollAxis::Vertical,
        UiPoint::new(0.0, 0.0),
    )
    .is_none());
}
