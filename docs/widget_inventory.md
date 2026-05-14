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
- `color_edit_button_srgb`
- `color_edit_button_srgba`
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
- `button`
- `checkbox`
- `slider`
- `text_input`
- `selectable_text`
- `combo_box`
- `dropdown_select`
- `select_menu`
- `select_menu_popup`
- `menu_list`
- `menu_list_popup`
- `menu_bar`
- `context_menu`
- `command_palette`
- `color_picker`
- `date_picker`
- `canvas`
- `progress_indicator`
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
- `popup_panel`
- `floating_desktop`

Public widget-adjacent contracts and helpers exposed by `operad::widgets`, but not full visual widget builders:

- `floating_window_layout`
- `searchable_select_contract`
- `editable_form_contract`
- `drag_value`
- `scrollbar_thumb`
- `scrollbar_accessibility`
- `path_breadcrumbs`
- popover and overlay processing helpers
- menu hit-testing, placement, filtering, navigation, and selection helpers
- data-table hit-testing, sizing, filtering, and export helpers
- button, checkbox, slider, text-input, and selectable-text action/event helpers

## egui Features Missing In Operad

This section lists egui widget and widget-adjacent features from the sources above that do not currently have an equivalent public Operad visual widget builder. Some lower-level Operad contracts may exist, but they are counted as missing here if customers cannot add the feature with a normal `operad::widgets::*` builder.

Missing visual widgets:

- Link
- Hyperlink
- Radio button
- Radio group
- Selectable label / selectable value
- Toggle switch / toggle value
- Visual numeric drag value
- Compact color button
- Standalone image widget
- Image button, deprecated in egui
- Spinner
- Separator
- Collapsing header
- Generic scroll bar widget
- Reset button / reset-to-default button
- Angle drag controls

Missing color-editing conveniences:

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

Missing containers and layout widgets:

- Area
- Frame
- Group
- Modal
- Resize container
- Resize handle
- Scene
- Sides
- Tooltip
- Central panel
- Side panel
- Top/bottom panel
- Generic grid layout
- Columns layout
- Indented layout section

Missing menu conveniences:

- Menu button
- Image menu button
- Image-and-text menu button
- Submenu
- Submenu button

Missing text-style conveniences:

- Colored label
- Heading
- Code label
- Code editor
- Monospace label
- Strong label
- Weak label
- Small label

Missing interaction helpers:

- Generic drag source widget
- Generic drop zone widget
- Widget-level visible/enabled wrappers like `add_visible`, `add_enabled`, `add_visible_ui`, and `add_enabled_ui`
- Widget sizing helpers like `add_sized`, `allocate_exact_size`, `allocate_at_least`, and `allocate_painter`
- Programmatic scroll helpers like `scroll_to_cursor`, `scroll_to_rect`, and animated variants

Missing theme/demo helpers:

- Global theme preference buttons
- Global theme preference switch

Partially covered but not equivalent:

- Text editing: Operad has `text_input` and `selectable_text`, but no `code_editor` convenience widget.
- Images: Operad has `ImageContent` and image-bearing widgets, but no standalone `image` widget builder.
- Tooltips: Operad has tooltip contracts/resolution helpers, but no public tooltip visual builder.
- Dialogs/modals: Operad has dialog and overlay state contracts plus `floating_desktop` for window shells, but no public modal/dialog visual builder comparable to egui `Modal`.
- Drag and drop: Operad has drag/drop descriptors and metadata on some complex widgets, but no generic `dnd_drag_source` or `dnd_drop_zone` widget.
- Scroll bars: Operad exposes scrollbar geometry/accessibility helpers, but no complete visual scrollbar widget builder.
- Panels: Operad has `dock_workspace`, `split_pane`, `scroll_area`, and generic containers, but no direct `CentralPanel`, `SidePanel`, or `TopBottomPanel` equivalents.
- Generic grid: Operad has tables and property grids, but no general-purpose grid layout builder.
