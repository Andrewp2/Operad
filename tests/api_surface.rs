use operad::prelude::*;
use operad::widgets::ext::{
    CommandPaletteItem, CommandPaletteState, SelectMenuState, SelectOption, TreeViewState,
};
use operad::widgets::TextInputState;

#[test]
fn prelude_fast_path_includes_layout_grid_and_auto_scroll_area() {
    let layout = layout::with_grid_template_columns(
        Layout::grid()
            .size(LayoutSize::points(240.0, 120.0))
            .gap(LayoutGap::points(8.0, 8.0))
            .to_layout_style(),
        [
            LayoutGridTrack::fraction(1.0),
            LayoutGridTrack::points(80.0),
        ],
    );
    assert_eq!(
        Layout::from_layout_style(&layout).map(|layout| layout.display),
        Some(LayoutDisplay::Grid)
    );

    let mut document = UiDocument::new(root_style(240.0, 120.0));
    let root = document.root();
    let viewport = scroll_area(
        &mut document,
        root,
        "items",
        ScrollAxes::BOTH,
        Layout::column()
            .size(LayoutSize::points(120.0, 80.0))
            .to_layout_style(),
    );

    let node = document.node(viewport);
    assert!(node.scroll().is_some());
    assert!(node.has_auto_scrollbar());

    let color = color_edit_button(
        &mut document,
        root,
        "accent",
        ColorRgba::new(74, 133, 198, 255),
        ColorButtonOptions::default().with_format(ColorValueFormat::Rgba),
    );
    assert!(document.node(color.root).accessibility().is_some());

    let rows = VirtualListSpec {
        row_count: 3,
        row_height: 20.0,
        viewport_height: 40.0,
        scroll_offset: 0.0,
        overscan: 1,
    };
    let list = virtual_list(
        &mut document,
        root,
        "rows",
        rows,
        |document, parent, index| {
            weak_label(
                document,
                parent,
                format!("row-{index}"),
                format!("Row {index}"),
                LayoutStyle::new(),
            );
        },
    );
    assert_eq!(
        document.node(list).accessibility().map(|meta| meta.role),
        Some(AccessibilityRole::List)
    );
}

#[test]
fn prelude_keeps_low_level_escape_hatches_out_of_the_fast_path() {
    let prelude = include_str!("../src/prelude.rs");
    let widget_exports_start = prelude
        .find("pub use crate::widgets::{")
        .expect("widgets prelude exports");
    let widget_exports = &prelude[widget_exports_start..];
    let widget_exports = &widget_exports[..widget_exports
        .find("};")
        .expect("end of widgets prelude exports")];
    let exported_names = widget_exports
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|name| !name.is_empty())
        .collect::<std::collections::HashSet<_>>();
    let low_level_exports = [
        "aligned_scrollbar_options",
        "scroll_area_with_bars",
        "scroll_container_shell",
        "raw_scroll_area",
        "scrollbar",
        "allocate_exact_size",
        "allocate_at_least",
        "allocate_painter",
        "area",
        "show_color_at",
        "resize_handle",
        "GridCellOptions",
        "GridOptions",
        "GridRowOptions",
        "grid_text_cell",
        "color_edit_button_rgb",
        "color_edit_button_rgba",
        "color_edit_button_rgba_premultiplied",
        "color_edit_button_rgba_unmultiplied",
        "color_edit_button_srgb",
        "color_edit_button_srgba",
        "color_edit_button_srgba_premultiplied",
        "color_edit_button_srgba_unmultiplied",
        "color_edit_button_hsva",
        "color_edit_button_oklch",
        "color_picker_color32",
        "color_picker_hsva_2d",
        "dnd_drag_source_actions_from_gesture_event",
        "dnd_drop_zone_actions_from_gesture_event",
        "dnd_drag_source_descriptor",
        "dnd_drop_target_descriptor",
        "dnd_drag_start_request",
        "dnd_apply_drop_zone_preview",
        "DragSourceDescriptor",
        "DragSourceId",
        "DropTargetDescriptor",
        "DropTargetHit",
        "DropTargetId",
        "DropZonePreviewState",
        "DialogDescriptor",
        "modal_dialog_close_actions_from_input_result",
        "modal_dialog_descriptor",
        "modal_dialog_dismiss_event_from_input_result",
        "modal_dialog_dismiss_event_from_key_event",
        "modal_dialog_dismiss_event_from_pointer_event",
        "modal_dialog_focus_trap",
        "modal_dialog_open_event",
        "OverlayFrameEvent",
        "OverlayFrameOutput",
        "OverlayFrameRequest",
        "OverlayFrameState",
        "process_overlay_frame",
        "apply_layout_animation_transitions_to_paint_list",
        "layout_animation_transitions",
        "LayoutAnimationOptions",
        "LayoutAnimationTransition",
        "AuditAxis",
        "AuditWarning",
        "ScrollbarAuditState",
        "ShaderEffect",
        "ShaderUniform",
        "ColorHsva2dNodes",
        "ColorHsva2dOptions",
        "MenuListNodes",
        "MenuListOptions",
        "tooltip_box_from_request",
        "tooltip_fade_slide_animation",
        "tooltip_rect",
        "tooltip_trigger_resolution",
        "TOOLTIP_HIDE_TRIGGER",
        "TOOLTIP_SHOW_TRIGGER",
    ];

    for name in low_level_exports {
        assert!(
            !exported_names.contains(name),
            "{name} should stay available from its module, not from the prelude fast path"
        );
    }
}

#[test]
fn prelude_keeps_backend_runtime_and_planners_module_scoped() {
    let prelude = include_str!("../src/prelude.rs");

    for surface in [
        "pub use crate::platform",
        "BackendCapabilities",
        "PlatformServiceClient",
        "PlatformRequest",
        "PlatformResponse",
        "NativeWindow",
        "WebRuntime",
        "run_app",
        "run_ui_document",
        "CanvasHostCapture",
        "VirtualPlan",
        "VirtualPlanRequest",
        "VirtualizationDiagnostics",
        "ProgrammaticScrollBehavior",
        "RevealScrollPlan",
    ] {
        assert!(
            !prelude.contains(surface),
            "{surface} should stay available from its module, not from the prelude fast path"
        );
    }
    assert!(
        prelude.contains("VirtualListSpec"),
        "the high-level virtual_list widget should export its spec in the prelude"
    );
}

#[test]
fn layout_style_keeps_raw_taffy_escape_hatches_module_scoped() {
    let document = include_str!("../src/core/document.rs");
    let layout = include_str!("../src/core/layout.rs");

    for raw_surface in [
        "pub fn from_taffy_style",
        "pub fn as_taffy_style",
        "pub fn as_taffy_style_mut",
        "pub fn absolute_rect",
        "impl From<Style> for LayoutStyle",
        "impl From<Style> for UiNodeStyle",
    ] {
        assert!(
            !document.contains(raw_surface),
            "{raw_surface} should not be public on LayoutStyle/UiNodeStyle"
        );
    }
    assert!(
        layout.contains("pub fn absolute("),
        "advanced absolute positioning should stay available from operad::layout"
    );
    assert!(
        layout.contains("pub fn with_min_size("),
        "common constraint updates should use typed layout helpers instead of raw Taffy mutation"
    );
    assert!(
        !document.contains("pub layout: Style"),
        "UiNodeStyle should not expose raw Taffy layout as a public field"
    );
    let ui_node_style_start = document
        .find("pub struct UiNodeStyle")
        .expect("UiNodeStyle definition");
    let ui_node_style_block = &document[ui_node_style_start
        ..document[ui_node_style_start..]
            .find("}\n\nimpl UiNodeStyle")
            .expect("end of UiNodeStyle definition")
            + ui_node_style_start];
    for raw_field in [
        "pub clip: ClipBehavior",
        "pub opacity: f32",
        "pub z_index: f32",
    ] {
        assert!(
            !ui_node_style_block.contains(raw_field),
            "{raw_field} should use typed UiNodeStyle accessors/builders instead of public fields"
        );
    }
    for accessor in [
        "pub fn layout_style(&self)",
        "pub fn layout(&self)",
        "pub const fn clip(&self)",
        "pub const fn opacity(&self)",
        "pub const fn z_index(&self) -> f32",
        "pub fn with_clip",
        "pub const fn with_z_index(mut self, z_index: f32)",
        "pub fn set_z_index(&mut self, z_index: f32)",
    ] {
        assert!(
            document.contains(accessor),
            "{accessor} should provide a typed alternative to public raw UiNodeStyle fields"
        );
    }
}

#[test]
fn ui_node_keeps_retained_tree_storage_behind_accessors() {
    let document = include_str!("../src/core/document.rs");
    assert!(
        !document.contains("pub struct UiNodeId(pub usize)"),
        "UiNodeId should be an opaque handle, not a public tuple around the document storage index"
    );
    for accessor in [
        "pub const ROOT: Self",
        "pub const fn root()",
        "pub const fn from_index(index: usize)",
        "pub const fn index(self)",
    ] {
        assert!(
            document.contains(accessor),
            "{accessor} should make index interop explicit when diagnostics/tests need it"
        );
    }

    let ui_node_start = document
        .find("pub struct UiNode {")
        .expect("UiNode definition");
    let ui_node_block = &document[ui_node_start
        ..document[ui_node_start..]
            .find("}\n\nimpl UiNode")
            .expect("end of UiNode definition")
            + ui_node_start];

    for raw_field in [
        "pub name:",
        "pub parent:",
        "pub children:",
        "pub style:",
        "pub content:",
        "pub input:",
        "pub scroll:",
        "pub scrollbar:",
        "pub auto_scrollbar:",
        "pub layout:",
    ] {
        assert!(
            !ui_node_block.contains(raw_field),
            "{raw_field} should use UiNode accessors/builders instead of public storage fields"
        );
    }

    for accessor in [
        "pub fn name(&self)",
        "pub fn parent(&self)",
        "pub fn children(&self)",
        "pub fn style(&self)",
        "pub fn style_mut(&mut self)",
        "pub fn action(&self)",
        "pub fn content(&self)",
        "pub fn scroll(&self)",
        "pub fn has_auto_scrollbar(&self)",
        "pub fn layout(&self)",
    ] {
        assert!(
            document.contains(accessor),
            "{accessor} should provide a typed alternative to public UiNode storage"
        );
    }
}

#[test]
fn ui_document_keeps_focus_and_scale_mutation_behind_methods() {
    let document = include_str!("../src/core/document.rs");
    let ui_document_start = document
        .find("pub struct UiDocument")
        .expect("UiDocument definition");
    let ui_document_block = &document[ui_document_start
        ..document[ui_document_start..]
            .find("}\n\nimpl UiDocument")
            .expect("end of UiDocument definition")
            + ui_document_start];

    for raw_field in [
        "pub root: UiNodeId",
        "pub focus: UiFocusState",
        "pub scale: UiDocumentScale",
    ] {
        assert!(
            !ui_document_block.contains(raw_field),
            "{raw_field} should use UiDocument accessors/mutators so invalidation and focus cleanup run"
        );
    }

    for accessor in [
        "pub const fn root(&self)",
        "pub fn scale(&self)",
        "pub fn set_scale(&mut self",
        "pub fn set_ui_scale(&mut self",
        "pub fn set_dpi_scale(&mut self",
        "pub fn focus_state(&self)",
        "pub fn set_focus_state(&mut self",
    ] {
        assert!(
            document.contains(accessor),
            "{accessor} should provide the typed UiDocument public path"
        );
    }
}

#[test]
fn scroll_state_keeps_raw_storage_behind_value_api() {
    let document = include_str!("../src/core/document.rs");
    let scroll_state_start = document
        .find("pub struct ScrollState")
        .expect("ScrollState definition");
    let scroll_state_block = &document[scroll_state_start
        ..document[scroll_state_start..]
            .find("}\n\nimpl ScrollState")
            .expect("end of ScrollState definition")
            + scroll_state_start];

    for raw_field in [
        "pub axes:",
        "pub offset:",
        "pub viewport_size:",
        "pub content_size:",
    ] {
        assert!(
            !scroll_state_block.contains(raw_field),
            "{raw_field} should use ScrollState accessors/builders instead of public storage fields"
        );
    }

    for accessor in [
        "pub const fn axes(self)",
        "pub const fn offset(self)",
        "pub const fn viewport_size(self)",
        "pub const fn content_size(self)",
        "pub fn with_offset",
        "pub const fn with_sizes",
        "pub fn set_offset",
        "pub fn sanitize_offset",
        "pub fn clamp_offset",
    ] {
        assert!(
            document.contains(accessor),
            "{accessor} should provide the typed ScrollState public path"
        );
    }
}

#[test]
fn scroll_state_preserves_requested_offset_before_layout_sizes_exist() {
    let scroll = ScrollState::new(ScrollAxes::VERTICAL).with_offset(UiPoint::new(40.0, 80.0));

    assert_eq!(scroll.offset(), UiPoint::new(0.0, 80.0));
    assert_eq!(scroll.max_offset(), UiPoint::new(0.0, 0.0));
    assert_eq!(scroll.clamp_offset(scroll.offset()), UiPoint::new(0.0, 0.0));
}

#[test]
fn text_input_state_keeps_editor_internals_behind_methods() {
    let text_input = include_str!("../src/widgets/text_input.rs");
    let state_start = text_input
        .find("pub struct TextInputState")
        .expect("TextInputState definition");
    let state_block = &text_input[state_start
        ..text_input[state_start..]
            .find("}\n\nimpl TextInputState")
            .expect("end of TextInputState definition")
            + state_start];

    for raw_field in [
        "pub text:",
        "pub caret:",
        "pub selection_anchor:",
        "pub multiline:",
        "pub composing:",
        "pub history:",
        "pub history_sequence:",
    ] {
        assert!(
            !state_block.contains(raw_field),
            "{raw_field} should use TextInputState methods so caret, selection, composition, and history stay coherent"
        );
    }

    for method in [
        "pub fn text(&self)",
        "pub fn set_text",
        "pub const fn is_multiline",
        "pub fn set_multiline",
        "pub fn caret(&self)",
        "pub fn set_caret",
        "pub fn selection_anchor(&self)",
        "pub fn set_selection",
        "pub fn composing(&self)",
        "pub fn history(&self)",
    ] {
        assert!(
            text_input.contains(method),
            "{method} should be the typed TextInputState public path"
        );
    }
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
fn color_picker_state_keeps_color_spaces_and_recent_storage_behind_methods() {
    let color_picker = include_str!("../src/widgets/ext/color_picker.rs");
    let state_start = color_picker
        .find("pub struct ColorPickerState")
        .expect("ColorPickerState definition");
    let state_block = &color_picker[state_start
        ..color_picker[state_start..]
            .find("}\n\nimpl ColorPickerState")
            .expect("end of ColorPickerState definition")
            + state_start];

    for raw_field in [
        "pub value:",
        "pub hsv:",
        "pub oklch:",
        "pub mode:",
        "pub palette:",
        "pub recent:",
        "pub max_recent:",
    ] {
        assert!(
            !state_block.contains(raw_field),
            "{raw_field} should use ColorPickerState methods so cached color spaces and recent colors stay coherent"
        );
    }

    for method in [
        "pub fn value(&self)",
        "pub fn hsv(&self)",
        "pub fn oklch(&self)",
        "pub fn mode(&self)",
        "pub fn palette(&self)",
        "pub fn palette_mut(&mut self)",
        "pub fn recent(&self)",
        "pub fn max_recent(&self)",
        "pub fn set_palette",
        "pub fn set_max_recent",
        "pub fn clear_recent",
        "pub fn set_rgba",
        "pub fn set_hsv",
        "pub fn set_oklch",
    ] {
        assert!(
            color_picker.contains(method),
            "{method} should provide the typed ColorPickerState public path"
        );
    }

    let mut state = ColorPickerState::new(ColorRgba::new(255, 0, 0, 255))
        .with_max_recent(1)
        .with_recent([
            ColorRgba::new(0, 255, 0, 255),
            ColorRgba::new(0, 0, 255, 255),
        ]);
    assert_eq!(state.value(), ColorRgba::new(255, 0, 0, 255));
    assert_eq!(state.max_recent(), 1);
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
fn command_palette_state_keeps_search_storage_behind_methods() {
    let command_palette = include_str!("../src/widgets/ext/command_palette.rs");
    let state_start = command_palette
        .find("pub struct CommandPaletteState")
        .expect("CommandPaletteState definition");
    let state_block = &command_palette[state_start
        ..command_palette[state_start..]
            .find("}\n\nimpl CommandPaletteState")
            .expect("end of CommandPaletteState definition")
            + state_start];

    for raw_field in ["pub query:", "pub active_match:", "pub max_results:"] {
        assert!(
            !state_block.contains(raw_field),
            "{raw_field} should use CommandPaletteState methods so query changes and active matches stay coherent"
        );
    }

    for method in [
        "pub fn query(&self)",
        "pub fn active_match(&self)",
        "pub fn max_results(&self)",
        "pub fn with_max_results",
        "pub fn set_max_results",
        "pub fn refresh_active_match",
        "pub fn set_query",
        "pub fn move_active",
        "pub fn select_active",
    ] {
        assert!(
            command_palette.contains(method),
            "{method} should provide the typed CommandPaletteState public path"
        );
    }

    let items = [
        CommandPaletteItem::new("open", "Open"),
        CommandPaletteItem::new("save", "Save").disabled(),
        CommandPaletteItem::new("close", "Close"),
    ];
    let mut state = CommandPaletteState::new()
        .with_query("o")
        .with_max_results(2)
        .with_first_active_match(&items);
    assert_eq!(state.query(), "o");
    assert_eq!(state.max_results(), 2);
    assert_eq!(state.active_match(), Some(0));

    state.set_query("save", &items);
    assert_eq!(state.query(), "save");
    assert_eq!(state.active_match(), None);

    state.set_max_results(1);
    assert_eq!(state.max_results(), 1);
}

#[test]
fn select_menu_state_keeps_open_selection_and_active_indices_behind_methods() {
    let dropdown = include_str!("../src/widgets/ext/dropdown.rs");
    let state_start = dropdown
        .find("pub struct SelectMenuState")
        .expect("SelectMenuState definition");
    let state_block = &dropdown[state_start
        ..dropdown[state_start..]
            .find("}\n\nimpl SelectMenuState")
            .expect("end of SelectMenuState definition")
            + state_start];

    for raw_field in ["pub open:", "pub selected:", "pub active:"] {
        assert!(
            !state_block.contains(raw_field),
            "{raw_field} should use SelectMenuState methods so active and selected options stay valid"
        );
    }

    for method in [
        "pub fn with_open",
        "pub fn with_active",
        "pub const fn is_open",
        "pub const fn selected_index",
        "pub const fn active_index",
        "pub fn open(&mut self",
        "pub fn close(&mut self)",
        "pub fn toggle",
        "pub fn activate_index",
        "pub fn select_id",
    ] {
        assert!(
            dropdown.contains(method),
            "{method} should provide the typed SelectMenuState public path"
        );
    }

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
fn tree_view_state_keeps_expansion_and_selection_storage_behind_methods() {
    let tree_view = include_str!("../src/widgets/ext/tree_view.rs");
    let state_start = tree_view
        .find("pub struct TreeViewState")
        .expect("TreeViewState definition");
    let state_block = &tree_view[state_start
        ..tree_view[state_start..]
            .find("}\n\nimpl TreeViewState")
            .expect("end of TreeViewState definition")
            + state_start];

    for raw_field in ["pub expanded_ids:", "pub selected_index:"] {
        assert!(
            !state_block.contains(raw_field),
            "{raw_field} should use TreeViewState methods so expansion ids remain deduplicated"
        );
    }

    for method in [
        "pub fn expanded_ids(&self)",
        "pub const fn selected_index(&self)",
        "pub fn clear_expanded",
        "pub fn set_expanded",
        "pub fn toggle_expanded",
        "pub fn select(&mut self",
        "pub fn visible_items",
    ] {
        assert!(
            tree_view.contains(method),
            "{method} should provide the typed TreeViewState public path"
        );
    }

    let mut state = TreeViewState::expanded(["root", "root", "assets"]).with_selected(Some(3));
    assert_eq!(
        state.expanded_ids(),
        &["root".to_string(), "assets".to_string()]
    );
    assert_eq!(state.selected_index(), Some(3));
    assert!(!state.toggle_expanded("root"));
    assert_eq!(state.expanded_ids(), &["assets".to_string()]);
    state.clear_expanded();
    assert!(state.expanded_ids().is_empty());
}

#[test]
fn identity_newtypes_keep_storage_behind_named_accessors() {
    let runtime = include_str!("../src/runtime/mod.rs");
    let platform = include_str!("../src/runtime/platform.rs");
    let windows = include_str!("../src/runtime/windows.rs");
    let drag_drop = include_str!("../src/interaction/drag_drop.rs");
    let input = include_str!("../src/interaction/input.rs");
    let overlays = include_str!("../src/interaction/overlays.rs");
    let navigation = include_str!("../src/interaction/navigation.rs");
    let forms = include_str!("../src/interaction/forms.rs");
    let tasks = include_str!("../src/interaction/tasks.rs");
    let input_devices = include_str!("../src/interaction/input_devices.rs");
    let compositor = include_str!("../src/render/compositor.rs");
    let fonts = include_str!("../src/render/fonts.rs");
    let toast = include_str!("../src/widgets/ext/toast.rs");

    for raw_tuple in [
        "pub struct RuntimeWindowId(pub String)",
        "pub struct RuntimeSurfaceId(pub String)",
        "pub struct RuntimeTimerId(pub u64)",
        "pub struct RuntimeIdleWorkId(pub u64)",
        "pub struct PlatformRequestId(pub u64)",
        "pub struct TextInputId(pub String)",
        "pub struct DragId(pub String)",
        "pub struct WindowId(pub String)",
        "pub struct DocumentId(pub String)",
        "pub struct SurfaceId(pub String)",
        "pub struct OverlayId(pub String)",
        "pub struct DragSourceId(pub String)",
        "pub struct DropTargetId(pub String)",
        "pub struct PointerId(pub u64)",
        "pub struct OverlayId(pub u64)",
        "pub struct NavigationItemId(pub u64)",
        "pub struct ValidationGeneration(pub u64)",
        "pub struct TaskGeneration(pub u64)",
        "pub struct GamepadDeviceId(pub u64)",
        "pub struct StackingContextId(pub u64)",
        "pub struct CompositorLayerId(pub u64)",
        "pub struct FontGeneration(pub u64)",
        "pub struct ToastId(pub u64)",
    ] {
        assert!(
            !runtime.contains(raw_tuple)
                && !platform.contains(raw_tuple)
                && !windows.contains(raw_tuple)
                && !drag_drop.contains(raw_tuple)
                && !input.contains(raw_tuple)
                && !overlays.contains(raw_tuple)
                && !navigation.contains(raw_tuple)
                && !forms.contains(raw_tuple)
                && !tasks.contains(raw_tuple)
                && !input_devices.contains(raw_tuple)
                && !compositor.contains(raw_tuple)
                && !fonts.contains(raw_tuple)
                && !toast.contains(raw_tuple),
            "{raw_tuple} should not expose raw tuple storage publicly"
        );
    }

    for method in [
        "pub fn as_str(&self)",
        "pub fn into_string(self)",
        "pub const fn value(self)",
    ] {
        assert!(
            runtime.contains(method)
                || platform.contains(method)
                || windows.contains(method)
                || drag_drop.contains(method)
                || input.contains(method)
                || overlays.contains(method)
                || navigation.contains(method)
                || forms.contains(method)
                || tasks.contains(method)
                || input_devices.contains(method)
                || compositor.contains(method)
                || fonts.contains(method)
                || toast.contains(method),
            "{method} should be available on the appropriate opaque id types"
        );
    }
}

#[test]
fn identity_newtypes_expose_clear_raw_value_methods() {
    assert_eq!(
        operad::runtime::RuntimeWindowId::new("main").as_str(),
        "main"
    );
    assert_eq!(
        operad::runtime::RuntimeSurfaceId::new("surface").into_string(),
        "surface"
    );
    assert_eq!(operad::runtime::RuntimeTimerId::new(9).value(), 9);
    assert_eq!(operad::runtime::RuntimeIdleWorkId::new(11).value(), 11);
    assert_eq!(operad::platform::PlatformRequestId::new(12).value(), 12);
    assert_eq!(
        operad::platform::TextInputId::new("node:1").as_str(),
        "node:1"
    );
    assert_eq!(operad::platform::DragId::new("drag").into_string(), "drag");
    assert_eq!(operad::windows::WindowId::new("window").as_str(), "window");
    assert_eq!(
        operad::windows::DocumentId::new("document").into_string(),
        "document"
    );
    assert_eq!(
        operad::windows::SurfaceId::new("surface").as_str(),
        "surface"
    );
    assert_eq!(
        operad::windows::OverlayId::new("overlay").into_string(),
        "overlay"
    );
    assert_eq!(
        operad::drag_drop::DragSourceId::new("source").as_str(),
        "source"
    );
    assert_eq!(
        operad::drag_drop::DropTargetId::new("target").into_string(),
        "target"
    );
    assert_eq!(operad::input::PointerId::new(1).value(), 1);
    assert_eq!(operad::overlays::OverlayId::new(2).value(), 2);
    assert_eq!(operad::navigation::NavigationItemId::new(3).value(), 3);
    assert_eq!(operad::forms::ValidationGeneration::new(4).value(), 4);
    assert_eq!(operad::tasks::TaskGeneration::new(5).value(), 5);
    assert_eq!(operad::input_devices::GamepadDeviceId::new(6).value(), 6);
    assert_eq!(operad::compositor::StackingContextId::new(7).value(), 7);
    assert_eq!(operad::compositor::CompositorLayerId::new(8).value(), 8);
    assert_eq!(operad::fonts::FontGeneration::new(9).value(), 9);
    assert_eq!(operad::widgets::ext::ToastId::new(10).value(), 10);
}

#[test]
fn font_weight_keeps_numeric_storage_behind_value_api() {
    let document = include_str!("../src/core/document.rs");

    assert!(
        !document.contains("pub struct FontWeight(pub u16)"),
        "FontWeight should expose FontWeight::new/value rather than a public tuple field"
    );
    assert!(document.contains("pub const fn new(value: u16)"));
    assert!(document.contains("pub const fn value(self) -> u16"));
    assert_eq!(FontWeight::new(550).value(), 550);
    assert_eq!(FontWeight::BOLD.value(), 700);
}

#[test]
fn font_library_carries_shared_renderer_and_measurer_inputs() {
    let fonts = operad::fonts::FontLibrary::new()
        .with_memory_font("ui", vec![1_u8, 2, 3])
        .with_sans_serif_family("Inter")
        .with_serif_family("Source Serif")
        .with_monospace_family("JetBrains Mono");

    assert_eq!(fonts.fonts()[0].key(), "ui");
    assert_eq!(fonts.fonts()[0].bytes(), &[1, 2, 3]);
    assert_eq!(fonts.sans_serif_family(), Some("Inter"));
    assert_eq!(fonts.serif_family(), Some("Source Serif"));
    assert_eq!(fonts.monospace_family(), Some("JetBrains Mono"));
}

#[test]
fn renderer_contract_exposes_empty_resource_resolver() {
    use operad::renderer::ResourceResolver;

    let resolver = operad::renderer::EmptyResourceResolver;
    assert_eq!(
        resolver.resolve_resource(&operad::platform::ResourceId::app("missing")),
        None
    );
}

#[test]
fn rendering_capabilities_include_app_owned_view_composition() {
    assert!(operad::platform::BackendCapabilities::native_window()
        .rendering
        .supports(operad::platform::RenderingCapabilityKind::AppOwnedViewRendering));
    assert!(!operad::platform::BackendCapabilities::test_host()
        .rendering
        .supports(operad::platform::RenderingCapabilityKind::AppOwnedViewRendering));
}

#[test]
fn flat_widgets_keep_legacy_color_helpers_out_of_the_fast_path() {
    let ext_mod = include_str!("../src/widgets/ext/mod.rs");
    let pickers = include_str!("../src/widgets/ext/pickers.rs");

    assert!(
        !ext_mod.contains("pub use color_picker::*"),
        "flat widget exports should list color picker APIs explicitly instead of glob-exporting legacy helpers"
    );

    for legacy_helper in [
        "color_edit_button_rgb",
        "color_edit_button_rgba",
        "color_edit_button_rgba_premultiplied",
        "color_edit_button_rgba_unmultiplied",
        "color_edit_button_srgb",
        "color_edit_button_srgba",
        "color_edit_button_srgba_premultiplied",
        "color_edit_button_srgba_unmultiplied",
        "color_edit_button_hsva",
        "color_edit_button_oklch",
        "color_picker_color32",
    ] {
        assert!(
            !pickers.contains(legacy_helper),
            "{legacy_helper} should stay out of widgets::pickers; use color_edit_button with ColorButtonOptions::with_format instead"
        );
    }
}

#[test]
fn extension_widgets_have_a_public_module_namespace() {
    let root = include_str!("../src/lib.rs");
    let widgets = include_str!("../src/widgets/mod.rs");

    assert!(
        !root.contains("mod widget_ext"),
        "extension widgets should not live in a hidden root module"
    );
    assert!(
        widgets.contains("pub mod ext;"),
        "extension widgets should be reachable as operad::widgets::ext"
    );
    assert!(
        !widgets.contains("pub use ext::*;"),
        "widgets::* should not flatten every extension widget; use operad::widgets::ext for advanced widgets"
    );
    for required_bridge in [
        "pub use ext::dialog::{DialogDismissReason, DialogDismissal};",
        "pub use ext::numeric_input::{NumericPrecision, NumericRange, NumericUnitFormat};",
        "pub use ext::toggle_control::ToggleValue;",
    ] {
        assert!(
            widgets.contains(required_bridge),
            "{required_bridge} should stay flat because core widgets expose it in their public options"
        );
    }
}

#[test]
fn extension_widget_namespace_uses_explicit_reexports() {
    let ext_mod = include_str!("../src/widgets/ext/mod.rs");

    assert!(
        !ext_mod.contains("pub use color_picker::*"),
        "color picker exports should stay curated so legacy format-specific helpers remain module-scoped"
    );
    assert!(
        !ext_mod.contains("::*"),
        "widgets::ext should list re-exported APIs explicitly so the advanced namespace stays reviewable"
    );
    for expected in [
        "pub use command_palette::{",
        "pub use data_table::{",
        "pub use dock_workspace::{",
        "pub use dropdown::{",
        "pub use floating_window::{",
        "pub use menu::{",
        "pub use theme_editor::{",
    ] {
        assert!(
            ext_mod.contains(expected),
            "{expected} should be an explicit extension-widget export block"
        );
    }
}

#[test]
fn extension_widget_namespace_keeps_helper_algorithms_module_scoped() {
    let ext_mod = include_str!("../src/widgets/ext/mod.rs");
    let mut export_text = String::new();
    let mut collecting_export = false;
    for line in ext_mod.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("pub use ") {
            collecting_export = true;
        }
        if collecting_export {
            export_text.push_str(line);
            export_text.push('\n');
        }
        if collecting_export && trimmed.ends_with(';') {
            collecting_export = false;
        }
    }
    let exported_names = export_text
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|name| !name.is_empty())
        .collect::<std::collections::HashSet<_>>();

    for helper in [
        "format_color_value",
        "format_hex_color",
        "parse_hex_color",
        "show_color_at",
        "color_picker_hsva_2d",
        "command_palette_item_from_command",
        "command_palette_items_from_registry",
        "command_palette_items_from_registry_with_history",
        "filter_command_palette",
        "data_table_cell_at_point",
        "data_table_column_at_x",
        "data_table_width",
        "export_data_table_text",
        "dock_drop_placement_from_target_id",
        "dock_panel_drag_payload",
        "dock_panel_id_from_payload",
        "dock_panel_reorder_drop_targets",
        "dock_panel_reorder_target_from_id",
        "dock_workspace_drop_zones",
        "filter_select_option_indices",
        "filter_select_options",
        "centered_popup_rect",
        "first_navigable_index",
        "last_navigable_index",
        "menu_command_selection_at_path",
        "menu_item_at_path",
        "menu_item_from_command",
        "menu_items_at_path",
        "menu_selection_at_path",
        "next_navigable_index",
        "place_popup",
        "process_overlay_frame",
        "resolve_popover_rect",
        "theme_patch_export",
        "theme_patch_from_themes",
        "tree_focus_preservation_by_id",
    ] {
        assert!(
            !exported_names.contains(helper),
            "{helper} should stay in its extension submodule instead of the flat widgets::ext catalogue"
        );
    }
}

#[test]
fn flat_widgets_keep_advanced_extension_widgets_module_scoped() {
    let widgets = include_str!("../src/widgets/mod.rs");

    for surface in [
        "command_palette",
        "command_diagnostics_panel",
        "animation_activity_panel",
        "animation_activity_timeline_panel",
        "bounds_autopsy_overlay",
        "bounds_autopsy_panel",
        "cache_reuse_panel",
        "clip_chain_panel",
        "clip_chain_timeline_panel",
        "clip_scroll_panel",
        "scroll_range_panel",
        "scroll_timeline_panel",
        "shortcut_route_panel",
        "constraint_issues_panel",
        "constraint_timeline_panel",
        "accessibility_tree_panel",
        "accessibility_timeline_panel",
        "action_dispatch_panel",
        "action_map_panel",
        "action_map_timeline_panel",
        "drag_affordance_panel",
        "interaction_affordance_panel",
        "interaction_affordance_timeline_panel",
        "interaction_state_timeline_panel",
        "debug_capture_report_panel",
        "debug_contract_panel",
        "debug_finding_inbox_panel",
        "debug_health_score_panel",
        "debug_invariant_panel",
        "debug_issue_panel",
        "debug_inspector_panel",
        "debug_layout_tree_panel",
        "node_search_panel",
        "debug_panel_recommendation_panel",
        "debug_session_narrative_panel",
        "diagnostics_coverage_panel",
        "dirty_state_panel",
        "dock_workspace",
        "dropdown_select",
        "event_route_panel",
        "focus_navigation_panel",
        "focus_navigation_timeline_panel",
        "frame_autopsy_panel",
        "frame_bottleneck_panel",
        "frame_budget_panel",
        "frame_diff_panel",
        "frame_recorder_panel",
        "frame_regression_panel",
        "frame_timeline_panel",
        "frame_timing_waterfall_panel",
        "fix_verification_panel",
        "hit_target_panel",
        "interaction_state_panel",
        "investigation_plan_panel",
        "inspect_point_overlay",
        "inspect_point_panel",
        "invariant_timeline_panel",
        "layout_jank_timeline_panel",
        "layout_movement_panel",
        "frame_timing_panel",
        "frame_trace_panel",
        "invalidation_blame_panel",
        "invalidation_blast_panel",
        "invalidation_timeline_panel",
        "hitbox_map_panel",
        "hitbox_occlusion_panel",
        "hitbox_occlusion_timeline_panel",
        "hitbox_timeline_panel",
        "inspect_node_panel",
        "issue_timeline_panel",
        "layout_autopsy_panel",
        "layout_cause_panel",
        "layout_cost_autopsy_panel",
        "layout_cost_panel",
        "layout_cost_timeline_panel",
        "layout_pressure_panel",
        "layout_pressure_timeline_panel",
        "node_change_panel",
        "node_frame_history_panel",
        "node_explanation_panel",
        "node_hotspots_panel",
        "node_provenance_panel",
        "node_recommendation_panel",
        "node_style_compare_panel",
        "overlay_recommendation_panel",
        "overlap_autopsy_panel",
        "overlap_report_panel",
        "overlap_timeline_panel",
        "paint_batch_timeline_panel",
        "paint_batches_panel",
        "paint_hit_mismatch_panel",
        "paint_hit_mismatch_timeline_panel",
        "paint_order_panel",
        "paint_overdraw_panel",
        "paint_overdraw_timeline_panel",
        "performance_timeline_panel",
        "point_autopsy_overlay",
        "point_autopsy_panel",
        "pointer_autopsy_panel",
        "pointer_probe_panel",
        "pointer_session_panel",
        "question_guide_panel",
        "render_layer_timeline_panel",
        "render_layers_panel",
        "property_inspector_grid",
        "resource_diagnostics_panel",
        "resource_timeline_panel",
        "responsive_layout_panel",
        "root_cause_cluster_panel",
        "root_cause_timeline_panel",
        "slow_frame_panel",
        "slow_node_timeline_panel",
        "slow_nodes_panel",
        "stacking_order_panel",
        "stacking_order_timeline_panel",
        "text_contrast_panel",
        "text_fit_panel",
        "text_fit_timeline_panel",
        "text_input_event_panel",
        "text_input_state_panel",
        "text_layout_panel",
        "text_localization_panel",
        "text_style_panel",
        "visibility_panel",
        "visibility_timeline_panel",
        "visual_effect_timeline_panel",
        "visual_effects_panel",
        "wheel_route_panel",
        "widget_state_retention_panel",
        "why_timeline_panel",
        "why_trace_panel",
        "theme_editor_panel",
        "timeline_ruler",
        "toast_stack",
        "tree_view",
        "virtualized_data_table",
    ] {
        assert!(
            !widgets.contains(&format!("pub use ext::{surface}")),
            "{surface} should be imported from operad::widgets::ext, not flattened into widgets::*"
        );
        assert!(
            !widgets.contains("pub use ext::*"),
            "blanket extension exports would put {surface} back in widgets::*"
        );
    }
    assert!(
        widgets.contains("pub mod ext;"),
        "advanced widgets should still have a public namespace"
    );
}

#[test]
fn flat_widgets_use_explicit_core_reexports() {
    let widgets = include_str!("../src/widgets/mod.rs");

    assert!(
        !widgets.contains("pub use button::*")
            && !widgets.contains("pub use helpers::*")
            && !widgets.contains("pub use text_input::*")
            && !widgets.contains("pub use tooltip::*"),
        "operad::widgets should list flat core-widget exports explicitly"
    );
    for line in widgets.lines() {
        assert!(
            !(line.trim_start().starts_with("pub use ") && line.contains("::*")),
            "operad::widgets should not use wildcard re-exports: {line}"
        );
    }
    for expected in [
        "pub use button::{",
        "pub use container::{",
        "pub use form::{",
        "pub use helpers::{",
        "pub use text_input::{",
        "pub use tooltip::{",
    ] {
        assert!(
            widgets.contains(expected),
            "{expected} should be an explicit core-widget export block"
        );
    }

    let mut export_text = String::new();
    let mut collecting_export = false;
    for line in widgets.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("pub use ") {
            collecting_export = true;
        }
        if collecting_export {
            export_text.push_str(line);
            export_text.push('\n');
        }
        if collecting_export && trimmed.ends_with(';') {
            collecting_export = false;
        }
    }
    let exported_names = export_text
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|name| !name.is_empty())
        .collect::<std::collections::HashSet<_>>();
    for low_level in [
        "button_actions_from_input_result",
        "push_button_input_result_actions",
        "checkbox_actions_from_input_result",
        "collapsing_header_actions_from_input_result",
        "area",
        "AreaOptions",
        "resize_handle",
        "ResizeHandleOptions",
        "add_sized",
        "AllocationOptions",
        "allocate_at_least",
        "allocate_exact_size",
        "allocate_painter",
        "grid",
        "grid_row",
        "grid_text_cell",
        "GridOptions",
        "GridRowOptions",
        "GridCellOptions",
        "dnd_drag_source_descriptor",
        "dnd_apply_drop_zone_preview",
        "DropZonePreviewState",
        "drag_value_input_actions_from_gesture_event",
        "link_actions_from_input_result",
        "selectable_label_actions_from_input_result",
        "modal_dialog_descriptor",
        "modal_dialog_open_event",
        "modal_dialog_dismiss_event_from_input_result",
        "radio_button_actions_from_input_result",
        "slider_actions_from_gesture_event",
        "round_slider_to_step",
        "slider_value_from_control_point",
        "handle_text_input_event",
        "text_input_actions_from_outcome",
        "CaretMovement",
        "TextInputCaretInfo",
        "TextInputCaretRect",
        "TextInputClipboardAction",
        "TextInputEventOutcome",
        "TextInputLayoutMetrics",
        "TextInputOutcome",
        "TextInputPaintOptions",
        "TextInputPlatformContext",
        "TextInputPosition",
        "TextInputRenderPlan",
        "TextInputSelectionRect",
        "toggle_switch_actions_from_input_result",
        "tooltip_rect",
        "tooltip_box_from_request",
        "TOOLTIP_SHOW_TRIGGER",
    ] {
        assert!(
            !exported_names.contains(low_level),
            "{low_level} should stay available from its widget module, not from widgets::*"
        );
    }
}

#[test]
fn flat_widgets_keep_raw_scroll_and_manual_scrollbars_module_scoped() {
    let widgets = include_str!("../src/widgets/mod.rs");
    let scroll_area = include_str!("../src/widgets/scroll_area.rs");
    let document = include_str!("../src/core/document.rs");

    assert!(
        !widgets.contains("pub use scroll_area::*"),
        "widgets::* should expose the automatic scroll fast path explicitly, not every raw scroll helper"
    );
    assert!(
        !scroll_area.contains("pub fn raw_scroll_area"),
        "raw_scroll_area should stay private so public app code gets automatic scrollbars by default"
    );
    assert!(
        !document.contains("pub fn without_auto_scrollbar"),
        "turning off automatic scrollbars should not be a public UiNode fast path"
    );
    assert!(
        !widgets.contains("pub use scrollbar::{"),
        "manual scrollbar primitives should be imported from operad::widgets::scrollbar"
    );

    for required in [
        "scroll_area",
        "scroll_container",
        "ScrollContainerNodes",
        "ScrollContainerOptions",
    ] {
        assert!(
            widgets.contains(required),
            "{required} should remain in widgets::* as the normal scroll path"
        );
    }
}

#[test]
fn public_modules_expose_their_owning_subsystems() {
    fn same_type<T>(_: Option<T>, _: Option<T>) {}
    same_type(
        None::<operad::host::HostInteractionState>,
        None::<operad::runtime::host::HostInteractionState>,
    );
    same_type(
        None::<operad::renderer::RenderFrameRequest>,
        None::<operad::render::renderer::RenderFrameRequest>,
    );
    same_type(
        None::<operad::actions::WidgetAction>,
        None::<operad::interaction::actions::WidgetAction>,
    );
    same_type(
        None::<operad::DirtyFlags>,
        None::<operad::core::invalidation::DirtyFlags>,
    );
    same_type(
        None::<operad::FrameTiming>,
        None::<operad::core::timing::FrameTiming>,
    );
    same_type(
        None::<operad::debug::DebugInspectorSnapshot>,
        None::<operad::diagnostics::debug::DebugInspectorSnapshot>,
    );
    same_type(
        None::<operad::testing::EventReplay>,
        None::<operad::diagnostics::testing::EventReplay>,
    );
}
