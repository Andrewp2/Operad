use operad::prelude::*;
use operad::widgets::ext::{
    CommandPaletteItem, CommandPaletteState, SelectMenuState, SelectOption, TreeViewState,
};
use operad::widgets::TextInputState;

#[test]
fn scroll_state_preserves_requested_offset_before_layout_sizes_exist() {
    let scroll = ScrollState::new(ScrollAxes::VERTICAL).with_offset(UiPoint::new(40.0, 80.0));

    assert_eq!(scroll.offset(), UiPoint::new(0.0, 80.0));
    assert_eq!(scroll.max_offset(), UiPoint::new(0.0, 0.0));
    assert_eq!(scroll.clamp_offset(scroll.offset()), UiPoint::new(0.0, 0.0));
}

#[test]
fn text_input_state_methods_clamp_selection_to_valid_text() {
    let mut state = TextInputState::new("café");
    state.set_caret(4);
    assert_eq!(state.caret(), "caf".len());

    state.set_selection(0, 4);
    assert_eq!(state.selected_range(), Some(0.."caf".len()));

    state.set_text("one\ntwo");
    assert_eq!(state.text(), "one two");
    assert_eq!(state.caret(), "one two".len());
    assert_eq!(state.selected_range(), None);

    state.set_multiline(true);
    state.set_text("one\r\ntwo");
    assert_eq!(state.text(), "one\ntwo");
}

#[test]
fn color_picker_edits_keep_color_spaces_and_bounded_history_in_sync() {
    let mut state = ColorPickerState::new(ColorRgba::new(255, 0, 0, 255))
        .with_max_recent(1)
        .with_recent([
            ColorRgba::new(0, 255, 0, 255),
            ColorRgba::new(0, 0, 255, 255),
        ]);
    assert_eq!(state.recent(), &[ColorRgba::new(0, 255, 0, 255)]);

    let update = state.set_hsv(ColorHsv::new(214.0, 0.68, 0.92, 0.5), EditPhase::CommitEdit);
    assert!(update.changed);
    assert_eq!(state.value(), state.hsv().to_rgba());
    assert_eq!(state.oklch(), ColorOklch::from_rgba(state.value()));
    assert_eq!(state.recent(), &[state.value()]);

    state.clear_recent();
    assert!(state.recent().is_empty());
}

#[test]
fn command_palette_query_change_clears_disabled_active_match() {
    let items = [
        CommandPaletteItem::new("open", "Open"),
        CommandPaletteItem::new("save", "Save").disabled(),
        CommandPaletteItem::new("close", "Close"),
    ];
    let mut state = CommandPaletteState::new()
        .with_query("o")
        .with_first_active_match(&items);
    assert_eq!(state.active_match(), Some(0));

    state.set_query("save", &items);
    assert_eq!(state.active_match(), None);
}

#[test]
fn select_menu_skips_disabled_options_and_closes_after_selection() {
    let options = [
        SelectOption::new("compact", "Compact"),
        SelectOption::new("disabled", "Disabled").disabled(),
        SelectOption::new("spacious", "Spacious"),
    ];
    let mut state = SelectMenuState::with_selected(0).with_open(&options);
    assert!(state.is_open());
    assert_eq!(state.selected_index(), Some(0));
    assert_eq!(state.active_index(), Some(0));

    assert_eq!(state.activate_index(&options, 1), None);
    assert_eq!(state.active_index(), Some(0));
    assert_eq!(state.activate_index(&options, 2), Some(2));

    let selection = state.select_active(&options).expect("select active");
    assert_eq!(selection.id, "spacious");
    assert!(!state.is_open());
    assert_eq!(state.selected_index(), Some(2));
}

#[test]
fn tree_view_expansion_deduplicates_ids_and_can_be_toggled() {
    let mut state = TreeViewState::expanded(["root", "root", "assets"]);
    assert_eq!(
        state.expanded_ids(),
        &["root".to_string(), "assets".to_string()]
    );
    assert!(!state.toggle_expanded("root"));
    assert_eq!(state.expanded_ids(), &["assets".to_string()]);
    state.clear_expanded();
    assert!(state.expanded_ids().is_empty());
}
