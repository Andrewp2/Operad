# Widget Inventory

This inventory compares the built-in widgets in egui with the public widget builders currently exposed by Operad.

Sources:

- egui 0.34.2 widgets: <https://docs.rs/egui/latest/egui/widgets/>
- egui 0.34.2 color picker widgets: <https://docs.rs/egui/latest/egui/widgets/color_picker/>
- egui 0.34.2 containers: <https://docs.rs/egui/latest/egui/containers/>
- egui 0.34.2 widget type list: <https://docs.rs/egui/latest/egui/enum.WidgetType.html>
- egui 0.34.2 `Ui` methods: <https://docs.rs/egui/latest/egui/struct.Ui.html>
- Operad exports: `src/lib.rs`, `src/widgets`, and `src/widgets/ext`

## egui Widgets

Core widgets from `egui::widgets` and `WidgetType`:

- `Label`
- `Link`
- `Hyperlink`
- `TextEdit`
- `Button`
- `Checkbox`
- `RadioButton`
- `RadioGroup`
- `SelectableLabel`
- `ComboBox`
- `Slider`
- `DragValue`
- `ColorButton`
- `Image`
- `ImageButton` deprecated
- `ProgressBar`
- `ProgressIndicator`
- `Spinner`
- `Separator`
- `CollapsingHeader`
- `Panel`
- `Window`
- `ResizeHandle`
- `ScrollBar`

Color widgets:

- `color_edit_button_hsva`
- `color_edit_button_rgb`
- `color_edit_button_rgba`
- `color_edit_button_rgba_premultiplied`
- `color_edit_button_rgba_unmultiplied`
- `color_edit_button_srgb`
- `color_edit_button_srgba`
- `color_edit_button_srgba_premultiplied`
- `color_edit_button_srgba_unmultiplied`
- `color_picker_color32`
- `color_picker_hsva_2d`
- `show_color`
- `show_color_at`

Container and layout widgets:

- `Area`
- `Frame`
- `Modal`
- `Popup`
- `Resize`
- `Scene`
- `Sides`
- `Tooltip`
- `ScrollArea`
- `CentralPanel`
- `SidePanel`
- `TopBottomPanel`
- `Grid`
- `MenuBar`
- `MenuButton`
- `SubMenu`
- `SubMenuButton`

Common `Ui` shorthand/widget methods:

- `label`
- `colored_label`
- `heading`
- `code`
- `code_editor`
- `monospace`
- `strong`
- `weak`
- `small`
- `button`
- `small_button`
- `checkbox`
- `radio`
- `radio_value`
- `selectable_label`
- `selectable_value`
- `toggle_value`
- `global_theme_preference_buttons`
- `global_theme_preference_switch`
- `text_edit_singleline`
- `text_edit_multiline`
- `image`
- `hyperlink`
- `hyperlink_to`
- `link`
- `separator`
- `spinner`
- `menu_button`
- `menu_image_button`
- `menu_image_text_button`
- `collapsing`
- `group`
- `dnd_drag_source`
- `dnd_drop_zone`
- `drag_angle`
- `drag_angle_tau`

## Operad Widgets

Public document-building widget functions currently exported under `operad::widgets`:

- `label`
- `localized_label`
- text-style label helpers: `heading_label`, `colored_label`, `code_label`,
  `monospace_label`, `strong_label`, `weak_label`, `small_label`,
  `wrapped_label`
- `link`
- `hyperlink`
- `selectable_label`
- `button`
- `small_button`
- `icon_button`
- `image_button`
- `toggle_button`
- `reset_button`
- `checkbox`
- `radio_button`
- `radio_group`
- `toggle_switch`
- `theme_preference_buttons`
- `theme_preference_switch`
- `slider`
- `drag_value_input`
- `drag_angle`
- `drag_angle_tau`
- `text_input`
- `singleline_text_input`
- `multiline_text_input`
- `text_area`
- `code_editor`
- `search_input`
- `password_input`
- `selectable_text`
- `selectable_value`
- `combo_box`
- `dropdown_select`
- `select_menu`
- `select_menu_popup`
- `menu_list`
- `menu_list_popup`
- `menu_bar`
- `menu_button`
- `image_menu_button`
- `image_text_menu_button`
- `submenu`
- `submenu_button`
- `context_menu`
- `command_palette`
- `color_picker`
- `compact_color_button`
- `color_swatch_button`
- `color_edit_button_rgb`
- `color_edit_button_rgba`
- `color_edit_button_rgba_premultiplied`
- `color_edit_button_rgba_unmultiplied`
- `color_edit_button_srgb`
- `color_edit_button_srgba`
- `color_edit_button_srgba_premultiplied`
- `color_edit_button_srgba_unmultiplied`
- `color_picker_color32`
- `color_picker_hsva_2d`
- `color_edit_button_hsva`
- `color_edit_button_oklch`
- `show_color`
- `show_color_at`
- `date_picker`
- `canvas`
- `image`
- `separator`
- `spacer`
- `spinner`
- `progress_indicator`
- `grid`
- `grid_row`
- `grid_text_cell`
- `panel`
- `central_panel`
- `top_panel`
- `bottom_panel`
- `side_panel`
- `left_panel`
- `right_panel`
- `group_panel`
- `area`
- `frame`
- `group`
- `scene`
- `sides`
- `columns`
- `indented_section`
- `resize_handle`
- `resize_container`
- `collapsing_header`
- `tooltip_box`
- `modal_dialog`
- `dnd_drag_source`
- `dnd_drop_zone`
- `form_section`
- `form_row`
- `field_label`
- `field_help_text`
- `field_validation_message`
- `form_error_summary`
- `table_header`
- `virtual_list`
- `virtualized_data_table`
- `property_inspector_grid`
- `tree_view`
- `outliner`
- `tab_group`
- `split_pane`
- `dock_workspace`
- `timeline_ruler`
- `toast_stack`
- `scroll_area`
- `scrollbar`
- `popup_panel`
- `floating_desktop`

Public widget-adjacent contracts and helpers exposed by `operad::widgets`, but not full visual widget builders:

- `floating_window_layout`
- `searchable_select_contract`
- `editable_form_contract`
- numeric drag model helper `drag_value`
- `scrollbar_thumb`
- `scrollbar_accessibility`
- `path_breadcrumbs`
- `add_visible`
- `add_visible_ui`
- `add_enabled`
- `add_enabled_ui`
- `set_subtree_visible`
- `set_subtree_enabled`
- `add_sized`
- `allocate_exact_size`
- `allocate_at_least`
- `allocate_painter`
- `scroll_to_cursor`
- `scroll_to_rect`
- `scroll_to_rect_with_options`
- drag/drop descriptor, hit-testing, and platform drag-start helpers
- popover and overlay processing helpers
- menu hit-testing, placement, filtering, navigation, and selection helpers
- data-table hit-testing, sizing, filtering, and export helpers
- button, checkbox, slider, text-input, and selectable-text action/event helpers

## egui Features Missing In Operad

This section lists egui widget and widget-adjacent features from the sources above that do not currently have an equivalent public Operad visual widget builder. Some lower-level Operad contracts may exist, but they are counted as missing here if customers cannot add the feature with a normal `operad::widgets::*` builder.

Partially covered but not equivalent:

- Text editing: Operad has single-line, multiline, text-area, code-editor, search, password, and selectable-text builders, but still needs richer editor-specific features.
- Forms: Operad now has visual section, row, label, help, validation, and error-summary helpers plus form state contracts. It still needs higher-level submit/cancel/apply form composition and keyboard traversal helpers.
- Tooltips: Operad now has `tooltip_box` plus tooltip contracts/resolution helpers, but still needs richer trigger wiring and animation policy.
- Dialogs/modals: Operad now has `modal_dialog` plus dialog and overlay state contracts, but still needs focus-trap and dismissal helpers wired into the visual builder.
- Drag and drop: Operad now has generic `dnd_drag_source` and `dnd_drop_zone` builders, plus descriptors and metadata on some complex widgets. It still needs fuller drop preview styling, drag-image policy, and platform adapter coverage.
- Scroll bars: Operad exposes a scrollbar widget and geometry/accessibility helpers, but still needs fuller integration coverage across every scrollable widget.
- Panels: Operad has `central_panel`, `side_panel`, `top_panel`, `bottom_panel`,
  `left_panel`, `right_panel`, `dock_workspace`, `split_pane`, `scroll_area`,
  and generic containers. It still needs richer pane-grid docking and animated
  panel collapse behavior.
