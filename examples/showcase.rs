use operad::platform::{DragPayload, PixelSize, UiLayer};
use operad::widgets::{CalendarDate, TextInputLayoutMetrics, TextInputOptions, TextInputState};
use operad::{
    root_style, widgets, AccessibilityMeta, AccessibilityRole, AlignedStroke, BuiltInIcon,
    CanvasContent, CanvasRenderOutput, ClipBehavior, ColorRgba, CornerRadii, DragDropSurfaceKind,
    DropPayloadFilter, DynamicLabelMeta, FocusRestoreTarget, FontFamily, FontWeight, FormState,
    FormValidationResult, ImageContent, LayoutFlexWrap, LayoutStyle, LocaleId, LocalizationPolicy,
    NativeWgpuCanvasRenderContext, NativeWgpuCanvasRenderRegistry, NativeWindowOptions,
    NativeWindowResult, PaintEffect, PaintRect, PaintText, RenderError, ScenePrimitive, ScrollAxes,
    StrokeStyle, TextHorizontalAlign, TextStyle, TextVerticalAlign, TextWrap, TooltipContent,
    UiDocument, UiNode, UiNodeId, UiNodeStyle, UiPoint, UiRect, UiSize, UiVisual,
    ValidationMessage, WgpuCanvasContext, WgpuCanvasRenderPass, WidgetAction, WidgetActionBinding,
    WidgetActionKind, WidgetDrag, WidgetDragPhase, WidgetTextEdit,
};

const RIGHT_PANEL_WIDTH: f32 = 300.0;
const SHOWCASE_WINDOW_Z_BASE: i16 = 64;
const SHOWCASE_WINDOW_Z_STRIDE: i16 = 32;
const SHOWCASE_WINDOW_Z_MAX: i16 = 960;
const SHOWCASE_TICK_RATE_HZ: f32 = 60.0;
const SHOWCASE_PROGRESS_RADIANS_PER_SECOND: f32 = 1.08;
const TEXT_CARET_BLINK_HZ: f32 = 1.1;
const CONTROLS_WIDGET_ROW_HEIGHT: f32 = 28.0;
const CONTROLS_WIDGET_ROW_GAP: f32 = 4.0;
const SHOWCASE_WIDGET_WINDOW_IDS: [&str; 28] = [
    "labels",
    "buttons",
    "checkbox",
    "toggles",
    "slider",
    "numeric",
    "text_input",
    "selection",
    "menus",
    "command_palette",
    "date_picker",
    "color_picker",
    "color_buttons",
    "progress",
    "lists_tables",
    "property_inspector",
    "trees",
    "layout_widgets",
    "containers",
    "forms",
    "overlays",
    "drag_drop",
    "media",
    "timeline",
    "toasts",
    "popup_panel",
    "canvas",
    "styling",
];

fn main() -> NativeWindowResult {
    let mut canvas_renderers = NativeWgpuCanvasRenderRegistry::new();
    canvas_renderers.register("canvas.shader", render_showcase_canvas);
    operad::run_app_with_canvas_renderers(
        NativeWindowOptions::new("showcase")
            .with_size(900.0, 760.0)
            .with_min_size(720.0, 560.0)
            .with_tick_action("runtime.tick")
            .with_tick_rate_hz(SHOWCASE_TICK_RATE_HZ),
        ShowcaseState::default(),
        ShowcaseState::update,
        ShowcaseState::view,
        canvas_renderers,
    )
}

struct ShowcaseState {
    checked: bool,
    slider: f32,
    slider_left: f32,
    slider_right: f32,
    slider_value_text: TextInputState,
    slider_left_text: TextInputState,
    slider_right_text: TextInputState,
    slider_step_value: f32,
    slider_step_text: TextInputState,
    slider_trailing_color: bool,
    slider_trailing_picker: widgets::ColorPickerState,
    slider_trailing_picker_open: bool,
    slider_thumb_shape: SliderThumbChoice,
    slider_use_steps: bool,
    slider_logarithmic: bool,
    slider_clamping: widgets::SliderClamping,
    slider_smart_aim: bool,
    label_locale: widgets::SelectMenuState,
    label_link_visited: bool,
    label_hyperlink_visited: bool,
    label_link_status: &'static str,
    color: widgets::ColorPickerState,
    date: widgets::DatePickerModel,
    radio_choice: &'static str,
    switch_enabled: bool,
    mixed_switch: widgets::ToggleValue,
    theme_preference: widgets::ThemePreference,
    numeric_value: f32,
    numeric_angle: f32,
    numeric_tau: f32,
    combo_open: bool,
    combo_label: String,
    dropdown: widgets::SelectMenuState,
    select_menu: widgets::SelectMenuState,
    text: TextInputState,
    selectable_text: TextInputState,
    singleline_text: TextInputState,
    multiline_text: TextInputState,
    text_area_text: TextInputState,
    code_editor_text: TextInputState,
    search_text: TextInputState,
    password_text: TextInputState,
    focused_text: Option<FocusedTextInput>,
    clipboard_text: String,
    system_clipboard: Option<arboard::Clipboard>,
    last_button: &'static str,
    toggle_button: bool,
    table_selection: widgets::DataTableSelection,
    tree: widgets::TreeViewState,
    outliner: widgets::TreeViewState,
    toast_visible: bool,
    toast_action_status: &'static str,
    popup_open: bool,
    progress_phase: f32,
    caret_phase: f32,
    command_palette: widgets::CommandPaletteState,
    last_command: String,
    list_scroll: f32,
    virtual_scroll: f32,
    table_scroll: f32,
    layout_preview_scroll: f32,
    layout_left_scroll: f32,
    layout_right_scroll: f32,
    layout_inspector_scroll: f32,
    layout_document_scroll: f32,
    layout_assets_scroll: f32,
    scrollbars: widgets::ScrollbarControllerState,
    layout_tab: usize,
    styling: StylingState,
    cube: CanvasCubeState,
    menu_bar: widgets::MenuBarState,
    menu_button: widgets::MenuButtonState,
    image_text_menu_button: widgets::MenuButtonState,
    image_menu_button: widgets::MenuButtonState,
    context_menu: widgets::ContextMenuState,
    menu_autosave: bool,
    menu_grid: bool,
    form: FormState,
    form_status: String,
    overlay_expanded: bool,
    overlay_popup_open: bool,
    overlay_modal_open: bool,
    color_button_status: &'static str,
    drag_drop_status: &'static str,
    layout_split: widgets::SplitPaneState,
    containers_scroll: operad::ScrollState,
    controls_scroll: f32,
    color_copied_hex: Option<String>,
    windows: ShowcaseWindows,
    desktop: widgets::FloatingDesktopState,
}

#[derive(Clone, Copy)]
struct StylingState {
    inner_same: bool,
    inner_margin: f32,
    inner_right: f32,
    inner_top: f32,
    inner_bottom: f32,
    outer_same: bool,
    outer_margin: f32,
    outer_right: f32,
    outer_top: f32,
    outer_bottom: f32,
    radius_same: bool,
    corner_radius: f32,
    corner_ne: f32,
    corner_sw: f32,
    corner_se: f32,
    shadow_x: f32,
    shadow_y: f32,
    shadow_blur: f32,
    shadow_spread: f32,
    shadow_alpha: f32,
    stroke_width: f32,
    stroke_tint: f32,
    fill_tint: f32,
    fill: ColorRgba,
}

impl Default for StylingState {
    fn default() -> Self {
        Self {
            inner_same: true,
            inner_margin: 12.0,
            inner_right: 12.0,
            inner_top: 12.0,
            inner_bottom: 12.0,
            outer_same: true,
            outer_margin: 24.0,
            outer_right: 24.0,
            outer_top: 24.0,
            outer_bottom: 24.0,
            radius_same: true,
            corner_radius: 12.0,
            corner_ne: 12.0,
            corner_sw: 12.0,
            corner_se: 12.0,
            shadow_x: 8.0,
            shadow_y: 12.0,
            shadow_blur: 16.0,
            shadow_spread: 0.0,
            shadow_alpha: 140.0,
            stroke_width: 1.0,
            stroke_tint: 0.68,
            fill_tint: 0.54,
            fill: ColorRgba::new(79, 45, 191, 255),
        }
    }
}

impl StylingState {
    fn inner_edges(self) -> [f32; 4] {
        if self.inner_same {
            [self.inner_margin; 4]
        } else {
            [
                self.inner_margin,
                self.inner_right,
                self.inner_top,
                self.inner_bottom,
            ]
        }
    }

    fn outer_edges(self) -> [f32; 4] {
        if self.outer_same {
            [self.outer_margin; 4]
        } else {
            [
                self.outer_margin,
                self.outer_right,
                self.outer_top,
                self.outer_bottom,
            ]
        }
    }

    fn radii(self) -> CornerRadii {
        if self.radius_same {
            CornerRadii::uniform(self.corner_radius)
        } else {
            CornerRadii::new(
                self.corner_radius,
                self.corner_ne,
                self.corner_se,
                self.corner_sw,
            )
        }
    }

    fn stroke_color(self) -> ColorRgba {
        let t = unit(self.stroke_tint);
        ColorRgba::new(
            (140.0 + t * 85.0) as u8,
            (140.0 + t * 85.0) as u8,
            (150.0 + t * 75.0) as u8,
            255,
        )
    }

    fn fill_color(self) -> ColorRgba {
        let t = unit(self.fill_tint);
        ColorRgba::new(
            (58.0 + t * 80.0) as u8,
            (30.0 + t * 44.0) as u8,
            (150.0 + t * 95.0) as u8,
            255,
        )
    }

    fn shadow_color(self) -> ColorRgba {
        ColorRgba::new(0, 0, 0, self.shadow_alpha.clamp(0.0, 255.0) as u8)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusedTextInput {
    Editable,
    Selectable,
    Singleline,
    Multiline,
    TextArea,
    CodeEditor,
    Search,
    Password,
    SelectableHelper,
    SliderValue,
    SliderRangeLeft,
    SliderRangeRight,
    SliderStep,
}

impl FocusedTextInput {
    const fn is_read_only(self) -> bool {
        matches!(self, Self::Selectable | Self::SelectableHelper)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SliderThumbChoice {
    Circle,
    Square,
    Rectangle,
}

#[derive(Clone, Copy)]
struct CanvasCubeState {
    yaw: f32,
    pitch: f32,
    drag_origin_yaw: f32,
    drag_origin_pitch: f32,
    rendered: Option<CanvasCubeRenderKey>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CanvasCubeRenderKey {
    yaw_bits: u32,
    pitch_bits: u32,
    size: PixelSize,
}

impl Default for CanvasCubeState {
    fn default() -> Self {
        Self {
            yaw: 0.82,
            pitch: 0.52,
            drag_origin_yaw: 0.82,
            drag_origin_pitch: 0.52,
            rendered: None,
        }
    }
}

impl CanvasCubeState {
    fn apply_drag(&mut self, drag: WidgetDrag) {
        match drag.phase {
            WidgetDragPhase::Begin => {
                self.drag_origin_yaw = self.yaw;
                self.drag_origin_pitch = self.pitch;
                self.apply_drag_delta(drag.total_delta);
            }
            WidgetDragPhase::Update | WidgetDragPhase::Commit => {
                self.apply_drag_delta(drag.total_delta);
            }
            WidgetDragPhase::Cancel => {
                self.yaw = self.drag_origin_yaw;
                self.pitch = self.drag_origin_pitch;
            }
        }
    }

    fn apply_drag_delta(&mut self, total_delta: UiPoint) {
        self.yaw = self.drag_origin_yaw + total_delta.x * 0.012;
        self.pitch = (self.drag_origin_pitch + total_delta.y * 0.012).clamp(-1.25, 1.25);
    }

    fn render_key(self, size: PixelSize) -> CanvasCubeRenderKey {
        CanvasCubeRenderKey {
            yaw_bits: self.yaw.to_bits(),
            pitch_bits: self.pitch.to_bits(),
            size,
        }
    }

    fn needs_render(self, size: PixelSize) -> bool {
        self.rendered != Some(self.render_key(size))
    }

    fn mark_rendered(&mut self, size: PixelSize) {
        self.rendered = Some(self.render_key(size));
    }
}

impl Default for ShowcaseState {
    fn default() -> Self {
        let text = TextInputState::new("Editable text");
        let mut selectable_text = TextInputState::new("Selectable read-only text");
        selectable_text.selection_anchor = Some(0);
        selectable_text.caret = "Selectable".len();
        let windows = ShowcaseWindows::default();
        let desktop = widgets::FloatingDesktopState::with_visible_order(
            SHOWCASE_WIDGET_WINDOW_IDS
                .into_iter()
                .filter(|id| windows.is_visible(id))
                .map(str::to_string),
            showcase_window_z_policy(),
        );

        Self {
            checked: true,
            slider: 10.0,
            slider_left: 1.0,
            slider_right: 10000.0,
            slider_value_text: TextInputState::new("10"),
            slider_left_text: TextInputState::new("1"),
            slider_right_text: TextInputState::new("10000"),
            slider_step_value: 10.0,
            slider_step_text: TextInputState::new("10"),
            slider_trailing_color: true,
            slider_trailing_picker: widgets::ColorPickerState::new(color(120, 170, 230)),
            slider_trailing_picker_open: false,
            slider_thumb_shape: SliderThumbChoice::Circle,
            slider_use_steps: false,
            slider_logarithmic: true,
            slider_clamping: widgets::SliderClamping::Always,
            slider_smart_aim: true,
            label_locale: widgets::SelectMenuState::with_selected(1),
            label_link_visited: false,
            label_hyperlink_visited: false,
            label_link_status: "No link action yet",
            color: widgets::ColorPickerState::new(color(118, 183, 255)),
            date: widgets::DatePickerModel::builder()
                .selected(CalendarDate::new(2026, 5, 12))
                .today(CalendarDate::new(2026, 5, 12))
                .build(),
            radio_choice: "compact",
            switch_enabled: true,
            mixed_switch: widgets::ToggleValue::Mixed,
            theme_preference: widgets::ThemePreference::Dark,
            numeric_value: 42.0,
            numeric_angle: 0.75,
            numeric_tau: 0.75,
            combo_open: false,
            combo_label: "Compact".to_string(),
            dropdown: widgets::SelectMenuState::with_selected(1),
            select_menu: widgets::SelectMenuState {
                open: true,
                selected: Some(0),
                active: Some(2),
            },
            text,
            selectable_text,
            singleline_text: TextInputState::new("Single line"),
            multiline_text: TextInputState::new("First line\nSecond line"),
            text_area_text: TextInputState::new("Text area content"),
            code_editor_text: TextInputState::new("fn main() {\n    println!(\"showcase\");\n}"),
            search_text: TextInputState::new("widgets"),
            password_text: TextInputState::new("correct horse"),
            focused_text: None,
            clipboard_text: String::new(),
            system_clipboard: create_system_clipboard(),
            last_button: "None",
            toggle_button: false,
            table_selection: widgets::DataTableSelection::single_row(2)
                .with_active_cell(widgets::DataTableCellIndex::new(2, 1)),
            tree: widgets::TreeViewState::expanded(["root"]),
            outliner: widgets::TreeViewState::expanded(["root", "assets"]),
            toast_visible: false,
            toast_action_status: "No toast action",
            popup_open: false,
            progress_phase: 0.0,
            caret_phase: 0.0,
            command_palette: widgets::CommandPaletteState {
                query: String::new(),
                active_match: Some(0),
                max_results: 24,
            },
            last_command: "None".to_string(),
            list_scroll: 0.0,
            virtual_scroll: 0.0,
            table_scroll: 0.0,
            layout_preview_scroll: 0.0,
            layout_left_scroll: 0.0,
            layout_right_scroll: 0.0,
            layout_inspector_scroll: 0.0,
            layout_document_scroll: 0.0,
            layout_assets_scroll: 0.0,
            scrollbars: widgets::ScrollbarControllerState::new(),
            layout_tab: 0,
            styling: StylingState::default(),
            cube: CanvasCubeState::default(),
            menu_bar: widgets::MenuBarState {
                open_menu: Some(0),
                active_item: Some(0),
            },
            menu_button: widgets::MenuButtonState::new(),
            image_text_menu_button: widgets::MenuButtonState::new(),
            image_menu_button: widgets::MenuButtonState::new(),
            context_menu: widgets::ContextMenuState::closed(),
            menu_autosave: true,
            menu_grid: true,
            form: profile_form_state(),
            form_status: "Unsaved profile changes".to_string(),
            overlay_expanded: true,
            overlay_popup_open: false,
            overlay_modal_open: false,
            color_button_status: "None",
            drag_drop_status: "Idle",
            layout_split: widgets::SplitPaneState::new(0.44).with_min_sizes(80.0, 80.0),
            containers_scroll: operad::ScrollState {
                axes: ScrollAxes::BOTH,
                offset: UiPoint::new(24.0, 18.0),
                viewport_size: UiSize::new(260.0, 82.0),
                content_size: UiSize::new(440.0, 180.0),
            },
            controls_scroll: 0.0,
            color_copied_hex: None,
            windows,
            desktop,
        }
    }
}

struct ShowcaseWindows {
    labels: bool,
    buttons: bool,
    checkbox: bool,
    toggles: bool,
    slider: bool,
    numeric: bool,
    text_input: bool,
    selection: bool,
    menus: bool,
    command_palette: bool,
    date_picker: bool,
    color_picker: bool,
    color_buttons: bool,
    progress: bool,
    lists_tables: bool,
    property_inspector: bool,
    trees: bool,
    layout_widgets: bool,
    containers: bool,
    forms: bool,
    overlays: bool,
    drag_drop: bool,
    media: bool,
    timeline: bool,
    toasts: bool,
    popup_panel: bool,
    canvas: bool,
    styling: bool,
}

impl Default for ShowcaseWindows {
    fn default() -> Self {
        Self {
            labels: true,
            buttons: true,
            checkbox: false,
            toggles: false,
            slider: false,
            numeric: false,
            text_input: false,
            selection: false,
            menus: false,
            command_palette: false,
            date_picker: false,
            color_picker: true,
            color_buttons: false,
            progress: false,
            lists_tables: false,
            property_inspector: false,
            trees: false,
            layout_widgets: false,
            containers: false,
            forms: false,
            overlays: false,
            drag_drop: false,
            media: false,
            timeline: false,
            toasts: false,
            popup_panel: false,
            canvas: true,
            styling: false,
        }
    }
}

impl ShowcaseWindows {
    fn is_visible(&self, id: &str) -> bool {
        match id {
            "labels" => self.labels,
            "buttons" => self.buttons,
            "checkbox" => self.checkbox,
            "toggles" => self.toggles,
            "slider" => self.slider,
            "numeric" => self.numeric,
            "text_input" => self.text_input,
            "selection" => self.selection,
            "menus" => self.menus,
            "command_palette" => self.command_palette,
            "date_picker" => self.date_picker,
            "color_picker" => self.color_picker,
            "color_buttons" => self.color_buttons,
            "progress" => self.progress,
            "lists_tables" => self.lists_tables,
            "property_inspector" => self.property_inspector,
            "trees" => self.trees,
            "layout_widgets" => self.layout_widgets,
            "containers" => self.containers,
            "forms" => self.forms,
            "overlays" => self.overlays,
            "drag_drop" => self.drag_drop,
            "media" => self.media,
            "timeline" => self.timeline,
            "toasts" => self.toasts,
            "popup_panel" => self.popup_panel,
            "canvas" => self.canvas,
            "styling" => self.styling,
            _ => false,
        }
    }

    fn slot_mut(&mut self, id: &str) -> Option<&mut bool> {
        match id {
            "labels" => Some(&mut self.labels),
            "buttons" => Some(&mut self.buttons),
            "checkbox" => Some(&mut self.checkbox),
            "toggles" => Some(&mut self.toggles),
            "slider" => Some(&mut self.slider),
            "numeric" => Some(&mut self.numeric),
            "text_input" => Some(&mut self.text_input),
            "selection" => Some(&mut self.selection),
            "menus" => Some(&mut self.menus),
            "command_palette" => Some(&mut self.command_palette),
            "date_picker" => Some(&mut self.date_picker),
            "color_picker" => Some(&mut self.color_picker),
            "color_buttons" => Some(&mut self.color_buttons),
            "progress" => Some(&mut self.progress),
            "lists_tables" => Some(&mut self.lists_tables),
            "property_inspector" => Some(&mut self.property_inspector),
            "trees" => Some(&mut self.trees),
            "layout_widgets" => Some(&mut self.layout_widgets),
            "containers" => Some(&mut self.containers),
            "forms" => Some(&mut self.forms),
            "overlays" => Some(&mut self.overlays),
            "drag_drop" => Some(&mut self.drag_drop),
            "media" => Some(&mut self.media),
            "timeline" => Some(&mut self.timeline),
            "toasts" => Some(&mut self.toasts),
            "popup_panel" => Some(&mut self.popup_panel),
            "canvas" => Some(&mut self.canvas),
            "styling" => Some(&mut self.styling),
            _ => None,
        }
    }

    fn toggle(&mut self, id: &str) -> Option<bool> {
        if let Some(visible) = self.slot_mut(id) {
            *visible = !*visible;
            return Some(*visible);
        }
        None
    }

    fn close(&mut self, id: &str) {
        if let Some(visible) = self.slot_mut(id) {
            *visible = false;
        }
    }

    fn clear_all(&mut self) {
        for id in SHOWCASE_WIDGET_WINDOW_IDS {
            if let Some(visible) = self.slot_mut(id) {
                *visible = false;
            }
        }
    }
}

fn showcase_window_z_policy() -> widgets::FloatingDesktopZPolicy {
    widgets::FloatingDesktopZPolicy::new(
        SHOWCASE_WINDOW_Z_BASE,
        SHOWCASE_WINDOW_Z_STRIDE,
        SHOWCASE_WINDOW_Z_MAX,
    )
}

fn window_defaults(id: &str) -> widgets::FloatingWindowDefaults {
    widgets::FloatingWindowDefaults::new(
        default_window_position(id),
        default_window_size(id),
        default_window_state_min_size(id),
    )
}

impl ShowcaseState {
    fn update(&mut self, action: WidgetAction) {
        let WidgetAction { binding, kind, .. } = action;
        let WidgetActionBinding::Action(action_id) = binding else {
            return;
        };
        let action_id = action_id.as_str();

        let color_outcome = self.color.apply_action(
            action_id,
            kind.clone(),
            widgets::ColorPickerActionOptions::new("color").copy_hex("color.copy_hex"),
        );
        if color_outcome.update.is_some()
            || color_outcome.effect.is_some()
            || color_outcome.mode_changed
        {
            if let Some(widgets::ColorPickerEffect::CopyHex(hex)) = color_outcome.effect {
                self.copy_text_to_system_clipboard(&hex);
                self.clipboard_text = hex.clone();
                self.color_copied_hex = Some(hex);
            }
            return;
        }
        let color_buttons_outcome = self.color.apply_action(
            action_id,
            kind.clone(),
            widgets::ColorPickerActionOptions::new("color_buttons.hsva_2d"),
        );
        if color_buttons_outcome.update.is_some() || color_buttons_outcome.mode_changed {
            self.color_button_status = "HSVA field";
            return;
        }
        let slider_color_outcome = self.slider_trailing_picker.apply_action(
            action_id,
            kind.clone(),
            widgets::ColorPickerActionOptions::new("slider.trailing_picker"),
        );
        if slider_color_outcome.update.is_some() || slider_color_outcome.mode_changed {
            return;
        }

        if action_id == "window.clear_all" {
            self.windows.clear_all();
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.toggle.") {
            if self.windows.toggle(id).unwrap_or(false) {
                self.desktop.ensure_window(id, window_defaults(id));
                self.desktop.bring_to_front(id);
            }
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.close.") {
            self.windows.close(id);
            self.desktop.close(id);
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.activate.") {
            self.desktop.bring_to_front(id);
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.drag.") {
            if let WidgetActionKind::PointerEdit(edit) = kind {
                self.desktop
                    .apply_drag(id, edit, default_window_position(id));
            }
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.resize.") {
            if let WidgetActionKind::PointerEdit(edit) = kind {
                self.desktop.apply_resize(id, edit, window_defaults(id));
            }
            return;
        }
        if let Some(id) = action_id.strip_prefix("window.collapse.") {
            self.desktop.toggle_collapsed(id);
            return;
        }
        if let Some(id) = window_for_action(action_id) {
            self.desktop.bring_to_front(id);
        }
        if action_id == "runtime.tick" {
            self.progress_phase += SHOWCASE_PROGRESS_RADIANS_PER_SECOND / SHOWCASE_TICK_RATE_HZ;
            self.caret_phase = (self.caret_phase
                + std::f32::consts::TAU * TEXT_CARET_BLINK_HZ / SHOWCASE_TICK_RATE_HZ)
                % std::f32::consts::TAU;
            return;
        }
        if action_id == "command_palette.search" {
            if let WidgetActionKind::TextEdit(edit) = kind {
                self.apply_command_palette_event(edit.event);
            }
            return;
        }
        if let Some(id) = action_id.strip_prefix("command_palette.item.") {
            self.select_command_palette_item(id);
            return;
        }
        if let Some(input) = focused_text_for_action(action_id) {
            if let WidgetActionKind::TextEdit(edit) = kind {
                self.apply_text_edit(input, edit);
            }
            return;
        }

        match action_id {
            "labels.link" => {
                self.label_link_visited = true;
                self.label_link_status = "Internal link activated";
                return;
            }
            "labels.hyperlink" => {
                self.label_hyperlink_visited = true;
                self.label_link_status = "Opened docs.rs/operad";
                open_url("https://docs.rs/operad");
                return;
            }
            "button.default" => self.last_button = "Default",
            "button.primary" => self.last_button = "Primary",
            "button.secondary" => self.last_button = "Secondary",
            "button.destructive" => self.last_button = "Destructive",
            "button.small" => self.last_button = "Small",
            "button.icon" => self.last_button = "Settings",
            "button.image" => self.last_button = "Folder",
            "button.reset" => {
                self.toggle_button = false;
                self.last_button = "Reset";
            }
            "button.toggle" => {
                self.toggle_button = !self.toggle_button;
                self.last_button = "Toggle";
            }
            "checkbox.enabled" => self.checked = !self.checked,
            "labels.locale.toggle" => {
                self.label_locale.toggle(&label_locale_options());
                return;
            }
            "toggles.switch" => self.switch_enabled = !self.switch_enabled,
            "toggles.mixed" => self.mixed_switch = self.mixed_switch.toggled(),
            "toggles.radio.compact" => self.radio_choice = "compact",
            "toggles.radio.comfortable" => self.radio_choice = "comfortable",
            "toggles.radio.spacious" => self.radio_choice = "spacious",
            "toggles.theme.system" => {
                self.theme_preference = widgets::ThemePreference::System;
                return;
            }
            "toggles.theme.light" => {
                self.theme_preference = widgets::ThemePreference::Light;
                return;
            }
            "toggles.theme.dark" => {
                self.theme_preference = widgets::ThemePreference::Dark;
                return;
            }
            "theme.preference.dark" => {
                self.theme_preference = if self.theme_preference.is_dark() {
                    widgets::ThemePreference::Light
                } else {
                    widgets::ThemePreference::Dark
                };
                return;
            }
            "combo.toggle" => self.combo_open = !self.combo_open,
            "selection.dropdown.toggle" => {
                self.dropdown.toggle(&select_options());
                return;
            }
            "menus.menu_button" => {
                let button_items = menu_items(self.menu_autosave);
                let outcome = self.menu_button.toggle(&button_items);
                if outcome.opened {
                    self.image_text_menu_button.close();
                    self.image_menu_button.close();
                    self.context_menu.close();
                }
                return;
            }
            "menus.image_text_menu_button" => {
                let button_items = menu_items(self.menu_autosave);
                let outcome = self.image_text_menu_button.toggle(&button_items);
                if outcome.opened {
                    self.menu_button.close();
                    self.image_menu_button.close();
                    self.context_menu.close();
                }
                return;
            }
            "menus.image_menu_button" => {
                let button_items = menu_items(self.menu_autosave);
                let outcome = self.image_menu_button.toggle(&button_items);
                if outcome.opened {
                    self.menu_button.close();
                    self.image_text_menu_button.close();
                    self.context_menu.close();
                }
                return;
            }
            "menus.context.open" => {
                self.context_menu
                    .open_with_items(UiPoint::new(0.0, 0.0), &menu_items(self.menu_autosave));
                self.menu_button.close();
                self.image_text_menu_button.close();
                self.image_menu_button.close();
                return;
            }
            "menus.context.close" => {
                self.context_menu.close();
                return;
            }
            "menus.bar.file" => {
                self.menu_bar
                    .open(&menu_bar_menus(self.menu_autosave, self.menu_grid), 0);
                return;
            }
            "menus.bar.edit" => {
                self.menu_bar
                    .open(&menu_bar_menus(self.menu_autosave, self.menu_grid), 1);
                return;
            }
            "menus.bar.view" => {
                self.menu_bar
                    .open(&menu_bar_menus(self.menu_autosave, self.menu_grid), 2);
                return;
            }
            "date.previous" => self.date.show_previous_month(),
            "date.next" => self.date.show_next_month(),
            "date.week.sunday" => {
                self.date.first_weekday = widgets::Weekday::Sunday;
                return;
            }
            "date.week.monday" => {
                self.date.first_weekday = widgets::Weekday::Monday;
                return;
            }
            "date.range.toggle" => {
                if self.date.min.is_some() || self.date.max.is_some() {
                    self.date.min = None;
                    self.date.max = None;
                } else {
                    self.date.min = CalendarDate::new(2026, 5, 4);
                    self.date.max = CalendarDate::new(2026, 5, 29);
                }
                return;
            }
            "toast.show" => {
                self.toast_visible = true;
                return;
            }
            "toast.hide" => {
                self.toast_visible = false;
                return;
            }
            id if id.starts_with("toast.dismiss.") => {
                self.toast_visible = false;
                return;
            }
            "toast.action.1.undo" => {
                self.toast_action_status = "Undo requested";
                return;
            }
            "popup.toggle" => {
                self.popup_open = !self.popup_open;
                return;
            }
            "popup.close" => {
                self.popup_open = false;
                return;
            }
            "layout.tab.preview" => {
                self.layout_tab = 0;
                return;
            }
            "layout.tab.settings" => {
                self.layout_tab = 1;
                return;
            }
            "forms.profile.submit" => {
                self.form.submit();
                self.form_status = "Submit requested".to_string();
                return;
            }
            "forms.profile.apply" => {
                self.form.apply();
                self.form_status = "Applied".to_string();
                return;
            }
            "forms.profile.cancel" => {
                self.form.cancel();
                self.form_status = "Cancelled".to_string();
                return;
            }
            "forms.profile.reset" => {
                self.form = profile_form_state();
                self.form_status = "Reset".to_string();
                return;
            }
            "overlays.collapsing.toggle" => {
                self.overlay_expanded = !self.overlay_expanded;
                return;
            }
            "overlays.popup.toggle" => {
                self.overlay_popup_open = !self.overlay_popup_open;
                return;
            }
            "overlays.popup.close" => {
                self.overlay_popup_open = false;
                return;
            }
            "overlays.modal.open" => {
                self.overlay_modal_open = true;
                return;
            }
            "overlays.modal.close" => {
                self.overlay_modal_open = false;
                return;
            }
            "drag_drop.text_source" => {
                self.drag_drop_status = "Text drag started";
                return;
            }
            "drag_drop.accept_text" => {
                self.drag_drop_status = "Text payload accepted";
                return;
            }
            "drag_drop.files_only" => {
                self.drag_drop_status = "File payload rejected";
                return;
            }
            "slider.trailing" => {
                self.slider_trailing_color = !self.slider_trailing_color;
                return;
            }
            "slider.trailing_color_button" => {
                self.slider_trailing_picker_open = !self.slider_trailing_picker_open;
                return;
            }
            "slider.thumb.circle" => {
                self.slider_thumb_shape = SliderThumbChoice::Circle;
                return;
            }
            "slider.thumb.square" => {
                self.slider_thumb_shape = SliderThumbChoice::Square;
                return;
            }
            "slider.thumb.rectangle" => {
                self.slider_thumb_shape = SliderThumbChoice::Rectangle;
                return;
            }
            "slider.steps" => {
                self.slider_use_steps = !self.slider_use_steps;
                if self.slider_use_steps {
                    self.set_slider_value(widgets::round_slider_to_step(
                        self.slider,
                        self.slider_step(),
                    ));
                }
                return;
            }
            "slider.logarithmic" => {
                self.slider_logarithmic = !self.slider_logarithmic;
                return;
            }
            "slider.clamping.never" => {
                self.slider_clamping = widgets::SliderClamping::Never;
                return;
            }
            "slider.clamping.edits" => {
                self.slider_clamping = widgets::SliderClamping::Edits;
                return;
            }
            "slider.clamping.always" => {
                self.slider_clamping = widgets::SliderClamping::Always;
                self.clamp_slider_to_range();
                return;
            }
            "slider.smart_aim" => {
                self.slider_smart_aim = !self.slider_smart_aim;
                return;
            }
            "styling.inner_same" => {
                self.styling.inner_same = !self.styling.inner_same;
                return;
            }
            "styling.outer_same" => {
                self.styling.outer_same = !self.styling.outer_same;
                return;
            }
            "styling.radius_same" => {
                self.styling.radius_same = !self.styling.radius_same;
                return;
            }
            _ => {}
        }

        if action_id == "canvas.rotate" {
            if let WidgetActionKind::Drag(drag) = kind {
                self.cube.apply_drag(drag);
            }
            return;
        }
        if let WidgetActionKind::Scroll(scroll) = &kind {
            match action_id {
                "lists_tables.scroll_area.scroll" => self.list_scroll = scroll.offset.y,
                "lists_tables.virtual_list.scroll" => self.virtual_scroll = scroll.offset.y,
                "lists_tables.data_table.scroll" => self.table_scroll = scroll.offset.y,
                "layout.preview.scroll" => self.layout_preview_scroll = scroll.offset.y,
                "layout.left.scroll" => self.layout_left_scroll = scroll.offset.y,
                "layout.right.scroll" => self.layout_right_scroll = scroll.offset.y,
                "layout.inspector.scroll" => self.layout_inspector_scroll = scroll.offset.y,
                "layout.document.scroll" => self.layout_document_scroll = scroll.offset.y,
                "layout.assets.scroll" => self.layout_assets_scroll = scroll.offset.y,
                "containers.scroll_area_with_bars.scroll" => {
                    self.containers_scroll.offset =
                        self.containers_scroll.clamp_offset(scroll.offset);
                }
                "controls.widget_list.scroll" => {
                    self.controls_scroll =
                        controls_scroll_state(scroll.offset.y, scroll.viewport_size.height)
                            .offset
                            .y;
                }
                _ => {}
            }
            return;
        }

        if let Some(date) = action_id
            .strip_prefix("date.day.")
            .and_then(parse_calendar_date)
        {
            self.date.select(date);
            return;
        }

        if let Some(option_id) = action_id.strip_prefix("labels.locale.option.") {
            self.label_locale
                .select_id_and_close(&label_locale_options(), option_id);
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("selection.dropdown.option.") {
            self.dropdown
                .select_id_and_close(&select_options(), option_id);
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("selection.combo.option.") {
            if let Some(option) = select_options()
                .into_iter()
                .find(|option| option.id == option_id && option.enabled)
            {
                self.combo_label = option.label;
                self.combo_open = false;
            }
            return;
        }
        if let Some(option_id) = action_id.strip_prefix("selection.menu.option.") {
            self.select_menu.select_id(&select_options(), option_id);
            return;
        }
        if let Some(menu_id) = action_id.strip_prefix("menus.item.") {
            self.apply_menu_item(menu_id);
            return;
        }
        if let Some(menu_id) = action_id.strip_prefix("menus.context.") {
            self.apply_menu_item(menu_id);
            self.context_menu.close();
            return;
        }
        if let Some(kind) = action_id.strip_prefix("color_buttons.") {
            self.color_button_status = match kind {
                "compact" => "Compact",
                "swatch" => "Swatch",
                "rgb" => "RGB",
                "rgba" => "RGBA",
                "srgb" => "SRGB",
                "srgba" => "SRGBA",
                "hsva" => "HSVA",
                "oklch" => "OKLCH",
                "color32" => "Color32",
                "rgba_premultiplied" => "RGBA premultiplied",
                "rgba_unmultiplied" => "RGBA unmultiplied",
                "srgba_premultiplied" => "SRGBA premultiplied",
                "srgba_unmultiplied" => "SRGBA unmultiplied",
                _ => self.color_button_status,
            };
            return;
        }
        if let Some(row) = action_id
            .strip_prefix("lists_tables.data_table.row.")
            .and_then(|row| row.parse::<usize>().ok())
        {
            self.table_selection = widgets::DataTableSelection::single_row(row)
                .with_active_cell(widgets::DataTableCellIndex::new(row, 0));
            return;
        }
        if let Some(cell) = action_id
            .strip_prefix("lists_tables.data_table.cell.")
            .and_then(parse_table_cell)
        {
            self.table_selection =
                widgets::DataTableSelection::single_row(cell.row).with_active_cell(cell);
            return;
        }
        if let Some(id) = action_id.strip_prefix("trees.tree.row.") {
            self.apply_tree_row(id, false);
            return;
        }
        if let Some(id) = action_id.strip_prefix("trees.outliner.row.") {
            self.apply_tree_row(id, true);
            return;
        }

        let WidgetActionKind::PointerEdit(edit) = kind else {
            return;
        };
        match action_id {
            "numeric.drag_value" => {
                self.numeric_value = scaled_slider(edit.target_rect, edit.position, 0.0, 100.0);
            }
            "numeric.drag_angle" => {
                self.numeric_angle =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 360.0).to_radians();
            }
            "numeric.drag_angle_tau" => {
                self.numeric_tau = scaled_slider(edit.target_rect, edit.position, 0.0, 1.0)
                    * std::f32::consts::TAU;
            }
            "layout_widgets.split_pane.handle" => {
                let total_extent = self
                    .desktop
                    .size("layout_widgets", default_window_size("layout_widgets"))
                    .width
                    - 48.0;
                let total_extent = total_extent.max(1.0);
                let handle_center = edit.target_rect.x + edit.target_rect.width * 0.5;
                self.layout_split
                    .resize_by(edit.position.x - handle_center, total_extent, 6.0);
            }
            "slider.value" => {
                self.set_slider_value(
                    self.slider_value_spec()
                        .value_from_control_point(edit.target_rect, edit.position),
                );
            }
            "slider.range_left" => {
                let value = widgets::SliderValueSpec::new(0.0, self.slider_right.max(1.0))
                    .value_from_control_point(edit.target_rect, edit.position);
                self.set_slider_left(value.min(self.slider_right - 1.0));
            }
            "slider.range_right" => {
                let value = widgets::SliderValueSpec::new(self.slider_left + 1.0, 10000.0)
                    .value_from_control_point(edit.target_rect, edit.position);
                self.set_slider_right(value.max(self.slider_left + 1.0));
            }
            "lists_tables.scroll_area.scrollbar" => {
                let scroll = scroll_state(self.list_scroll, 92.0, 6.0 * 26.0);
                self.list_scroll = self
                    .scrollbars
                    .apply_drag_for_target_rect("list", scroll, widgets::ScrollAxis::Vertical, edit)
                    .y;
            }
            "lists_tables.virtual_list.scrollbar" => {
                let scroll = scroll_state(self.virtual_scroll, 112.0, 24.0 * 28.0);
                self.virtual_scroll = self
                    .scrollbars
                    .apply_drag_for_target_rect(
                        "virtual",
                        scroll,
                        widgets::ScrollAxis::Vertical,
                        edit,
                    )
                    .y;
            }
            "lists_tables.data_table.scrollbar" => {
                let scroll = scroll_state(self.table_scroll, 128.0, 16.0 * 28.0);
                self.table_scroll = self
                    .scrollbars
                    .apply_drag_for_target_rect(
                        "table",
                        scroll,
                        widgets::ScrollAxis::Vertical,
                        edit,
                    )
                    .y;
            }
            "containers.scroll_area_with_bars.vertical-scrollbar" => {
                self.containers_scroll.offset = self.scrollbars.apply_drag_for_target_rect(
                    "containers.vertical",
                    self.containers_scroll,
                    widgets::ScrollAxis::Vertical,
                    edit,
                );
            }
            "containers.scroll_area_with_bars.horizontal-scrollbar" => {
                self.containers_scroll.offset = self.scrollbars.apply_drag_for_target_rect(
                    "containers.horizontal",
                    self.containers_scroll,
                    widgets::ScrollAxis::Horizontal,
                    edit,
                );
            }
            "controls.widget_list.scrollbar" => {
                self.controls_scroll = self
                    .scrollbars
                    .apply_drag_for_target_rect(
                        "controls.widget_list",
                        controls_scroll_state(self.controls_scroll, edit.target_rect.height),
                        widgets::ScrollAxis::Vertical,
                        edit,
                    )
                    .y;
            }
            "styling.inner" => {
                self.styling.inner_margin =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 32.0);
                if self.styling.inner_same {
                    self.styling.inner_right = self.styling.inner_margin;
                    self.styling.inner_top = self.styling.inner_margin;
                    self.styling.inner_bottom = self.styling.inner_margin;
                }
            }
            "styling.inner_right" => {
                self.styling.inner_right =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 32.0);
            }
            "styling.inner_top" => {
                self.styling.inner_top = scaled_slider(edit.target_rect, edit.position, 0.0, 32.0);
            }
            "styling.inner_bottom" => {
                self.styling.inner_bottom =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 32.0);
            }
            "styling.outer" => {
                self.styling.outer_margin =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 40.0);
                if self.styling.outer_same {
                    self.styling.outer_right = self.styling.outer_margin;
                    self.styling.outer_top = self.styling.outer_margin;
                    self.styling.outer_bottom = self.styling.outer_margin;
                }
            }
            "styling.outer_right" => {
                self.styling.outer_right =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 40.0);
            }
            "styling.outer_top" => {
                self.styling.outer_top = scaled_slider(edit.target_rect, edit.position, 0.0, 40.0);
            }
            "styling.outer_bottom" => {
                self.styling.outer_bottom =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 40.0);
            }
            "styling.radius" => {
                self.styling.corner_radius =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 28.0);
                if self.styling.radius_same {
                    self.styling.corner_ne = self.styling.corner_radius;
                    self.styling.corner_sw = self.styling.corner_radius;
                    self.styling.corner_se = self.styling.corner_radius;
                }
            }
            "styling.radius_ne" => {
                self.styling.corner_ne = scaled_slider(edit.target_rect, edit.position, 0.0, 28.0);
            }
            "styling.radius_sw" => {
                self.styling.corner_sw = scaled_slider(edit.target_rect, edit.position, 0.0, 28.0);
            }
            "styling.radius_se" => {
                self.styling.corner_se = scaled_slider(edit.target_rect, edit.position, 0.0, 28.0);
            }
            "styling.shadow_x" => {
                self.styling.shadow_x = scaled_slider(edit.target_rect, edit.position, -24.0, 24.0);
            }
            "styling.shadow_y" => {
                self.styling.shadow_y = scaled_slider(edit.target_rect, edit.position, -24.0, 24.0);
            }
            "styling.shadow" => {
                self.styling.shadow_blur =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 32.0);
            }
            "styling.shadow_spread" => {
                self.styling.shadow_spread =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 16.0);
            }
            "styling.shadow_alpha" => {
                self.styling.shadow_alpha =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 220.0);
            }
            "styling.stroke" => {
                self.styling.stroke_width =
                    scaled_slider(edit.target_rect, edit.position, 0.0, 4.0);
            }
            "styling.fill" => {
                self.styling.fill_tint = widgets::slider_value_from_control_point(
                    edit.target_rect,
                    edit.position,
                    0.0..1.0,
                );
                self.styling.fill = self.styling.fill_color();
            }
            "styling.stroke_color" => {
                self.styling.stroke_tint = widgets::slider_value_from_control_point(
                    edit.target_rect,
                    edit.position,
                    0.0..1.0,
                );
            }
            _ => {}
        }
    }

    fn apply_command_palette_event(&mut self, event: operad::UiInputEvent) {
        let outcome = self
            .command_palette
            .handle_event(&command_palette_items(), &event);
        if let Some(selection) = outcome.selected {
            self.select_command_palette_item(&selection.id);
        }
    }

    fn select_command_palette_item(&mut self, id: &str) {
        if let Some(item) = command_palette_items()
            .into_iter()
            .find(|item| item.id == id && item.enabled)
        {
            self.last_command = item.title;
            self.command_palette.set_query("", &command_palette_items());
        }
    }

    fn apply_text_edit(&mut self, input: FocusedTextInput, edit: WidgetTextEdit) {
        self.focused_text = Some(input);
        if let Some(point) = edit.local_position {
            let style = text(13.0, color(230, 236, 246));
            let target_rect = edit
                .target_rect
                .unwrap_or_else(|| UiRect::new(0.0, 0.0, 320.0, 36.0));
            let metrics = TextInputLayoutMetrics::from_style(
                UiRect::new(
                    6.0,
                    6.0,
                    (target_rect.width - 12.0).max(1.0),
                    (target_rect.height - 12.0).max(1.0),
                ),
                &style,
            );
            if let Some(state) = self.text_state_mut(input) {
                state.move_caret_to_point(metrics, point, edit.selecting);
            }
            return;
        }

        let outcome = if input.is_read_only() {
            self.text_state_mut(input).map(|state| {
                state.handle_event_with_policy(
                    &edit.event,
                    widgets::TextInputInteractionPolicy::read_only(),
                )
            })
        } else {
            self.text_state_mut(input)
                .map(|state| state.handle_event(&edit.event))
        };
        if let Some(outcome) = outcome {
            self.apply_text_clipboard_outcome(input, outcome);
            self.sync_text_input_value(input);
        }
    }

    fn apply_text_clipboard_outcome(
        &mut self,
        input: FocusedTextInput,
        outcome: widgets::TextInputOutcome,
    ) {
        match outcome.clipboard {
            Some(widgets::TextInputClipboardAction::Copy(text))
            | Some(widgets::TextInputClipboardAction::Cut(text)) => {
                self.copy_text_to_system_clipboard(&text);
                self.clipboard_text = text;
            }
            Some(widgets::TextInputClipboardAction::Paste) => {
                let pasted = self
                    .read_text_from_system_clipboard()
                    .unwrap_or_else(|| self.clipboard_text.clone());
                if !input.is_read_only() {
                    if let Some(state) = self.text_state_mut(input) {
                        state.paste_text(&pasted);
                    }
                }
            }
            None => {}
        }
    }

    fn text_state_mut(&mut self, input: FocusedTextInput) -> Option<&mut TextInputState> {
        match input {
            FocusedTextInput::Editable => Some(&mut self.text),
            FocusedTextInput::Selectable | FocusedTextInput::SelectableHelper => {
                Some(&mut self.selectable_text)
            }
            FocusedTextInput::Singleline => Some(&mut self.singleline_text),
            FocusedTextInput::Multiline => Some(&mut self.multiline_text),
            FocusedTextInput::TextArea => Some(&mut self.text_area_text),
            FocusedTextInput::CodeEditor => Some(&mut self.code_editor_text),
            FocusedTextInput::Search => Some(&mut self.search_text),
            FocusedTextInput::Password => Some(&mut self.password_text),
            FocusedTextInput::SliderValue => Some(&mut self.slider_value_text),
            FocusedTextInput::SliderRangeLeft => Some(&mut self.slider_left_text),
            FocusedTextInput::SliderRangeRight => Some(&mut self.slider_right_text),
            FocusedTextInput::SliderStep => Some(&mut self.slider_step_text),
        }
    }

    fn sync_text_input_value(&mut self, input: FocusedTextInput) {
        match input {
            FocusedTextInput::SliderValue => {
                if let Ok(value) = self.slider_value_text.text.parse::<f32>() {
                    self.apply_slider_value_from_text(value);
                }
            }
            FocusedTextInput::SliderRangeLeft => {
                if let Ok(value) = self.slider_left_text.text.parse::<f32>() {
                    self.apply_slider_left_from_text(value);
                }
            }
            FocusedTextInput::SliderRangeRight => {
                if let Ok(value) = self.slider_right_text.text.parse::<f32>() {
                    self.apply_slider_right_from_text(value);
                }
            }
            FocusedTextInput::SliderStep => {
                if let Ok(value) = self.slider_step_text.text.parse::<f32>() {
                    self.slider_step_value = value.abs().max(0.0001);
                    if self.slider_use_steps {
                        self.set_slider_value(widgets::round_slider_to_step(
                            self.slider,
                            self.slider_step(),
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    fn copy_text_to_system_clipboard(&mut self, text: &str) {
        if self.system_clipboard.is_none() {
            self.system_clipboard = create_system_clipboard();
        }
        if let Some(clipboard) = self.system_clipboard.as_mut() {
            if clipboard.set_text(text.to_string()).is_err() {
                self.system_clipboard = None;
            }
        }
    }

    fn read_text_from_system_clipboard(&mut self) -> Option<String> {
        if self.system_clipboard.is_none() {
            self.system_clipboard = create_system_clipboard();
        }
        self.system_clipboard
            .as_mut()
            .and_then(|clipboard| clipboard.get_text().ok())
    }

    fn apply_menu_item(&mut self, id: &str) {
        let menus = menu_bar_menus(self.menu_autosave, self.menu_grid);
        self.menu_bar.set_active_item_by_id(&menus, id);
        if id == "autosave" {
            self.menu_autosave = !self.menu_autosave;
        } else if id == "grid" {
            self.menu_grid = !self.menu_grid;
        }
        self.menu_button.close();
        self.image_text_menu_button.close();
        self.image_menu_button.close();
    }

    fn apply_tree_row(&mut self, id: &str, outliner: bool) {
        let roots = tree_items();
        let state = if outliner {
            &mut self.outliner
        } else {
            &mut self.tree
        };
        state.activate_visible_item_id(&roots, id);
    }

    fn slider_value_spec(&self) -> widgets::SliderValueSpec {
        let mut spec = widgets::SliderValueSpec::new(self.slider_left, self.slider_right)
            .logarithmic(self.slider_logarithmic)
            .clamping(self.slider_clamping)
            .smart_aim(self.slider_smart_aim);
        if self.slider_use_steps {
            spec = spec.step(self.slider_step());
        }
        spec
    }

    fn set_slider_value(&mut self, value: f32) {
        let value = self.slider_value_spec().adjust_value(value);
        self.slider = value;
        self.slider_value_text.text = widgets::format_slider_value(value);
        self.slider_value_text.caret = self.slider_value_text.text.len();
        self.slider_value_text.selection_anchor = None;
    }

    fn apply_slider_value_from_text(&mut self, value: f32) {
        self.slider = if self.slider_clamping == widgets::SliderClamping::Always {
            self.slider_value_spec().clamp(value)
        } else {
            value
        };
    }

    fn set_slider_left(&mut self, value: f32) {
        self.slider_left = value.min(self.slider_right - 1.0).max(0.0);
        self.slider_left_text.text = widgets::format_slider_value(self.slider_left);
        self.slider_left_text.caret = self.slider_left_text.text.len();
        if self.slider_clamping == widgets::SliderClamping::Always {
            self.clamp_slider_to_range();
        }
    }

    fn apply_slider_left_from_text(&mut self, value: f32) {
        if value < self.slider_right {
            self.slider_left = value.max(0.0);
            if self.slider_clamping == widgets::SliderClamping::Always {
                self.slider = self.slider.clamp(self.slider_left, self.slider_right);
            }
        }
    }

    fn set_slider_right(&mut self, value: f32) {
        self.slider_right = value.max(self.slider_left + 1.0).min(10000.0);
        self.slider_right_text.text = widgets::format_slider_value(self.slider_right);
        self.slider_right_text.caret = self.slider_right_text.text.len();
        if self.slider_clamping == widgets::SliderClamping::Always {
            self.clamp_slider_to_range();
        }
    }

    fn apply_slider_right_from_text(&mut self, value: f32) {
        if value > self.slider_left {
            self.slider_right = value.min(10000.0);
            if self.slider_clamping == widgets::SliderClamping::Always {
                self.slider = self.slider.clamp(self.slider_left, self.slider_right);
            }
        }
    }

    fn clamp_slider_to_range(&mut self) {
        self.set_slider_value(self.slider.clamp(self.slider_left, self.slider_right));
    }

    fn slider_step(&self) -> f32 {
        self.slider_step_value.abs().max(0.0001)
    }

    fn view(&self, viewport: UiSize) -> UiDocument {
        let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
        ui.node_mut(ui.root).visual = UiVisual::panel(color(16, 20, 26), None, 0.0);

        let root = ui.root;
        let shell = ui.add_child(
            root,
            UiNode::container(
                "showcase.shell",
                LayoutStyle::row().with_size(viewport.width, viewport.height),
            ),
        );
        let desktop_width = (viewport.width - RIGHT_PANEL_WIDTH).max(360.0);
        let desktop = ui.add_child(
            shell,
            UiNode::container(
                "showcase.desktop",
                LayoutStyle::new()
                    .with_width(desktop_width)
                    .with_height(viewport.height)
                    .with_flex_shrink(1.0),
            )
            .with_visual(UiVisual::panel(color(15, 19, 25), None, 0.0)),
        );
        let controls = ui.add_child(
            shell,
            UiNode::container(
                "showcase.controls",
                LayoutStyle::column()
                    .with_width(RIGHT_PANEL_WIDTH)
                    .with_height(viewport.height)
                    .with_flex_shrink(0.0)
                    .padding(12.0)
                    .gap(4.0),
            )
            .with_visual(UiVisual::panel(
                color(21, 26, 33),
                Some(StrokeStyle::new(color(46, 56, 70), 1.0)),
                0.0,
            )),
        );

        showcase_windows(
            &mut ui,
            desktop,
            self,
            UiSize::new(desktop_width, viewport.height),
        );
        control_panel(&mut ui, controls, self, viewport.height);

        ui
    }
}

fn showcase_windows(
    ui: &mut UiDocument,
    desktop: UiNodeId,
    state: &ShowcaseState,
    desktop_size: UiSize,
) {
    let windows = showcase_window_descriptors(state, desktop_size);
    let mut options = widgets::FloatingDesktopOptions::new(desktop_size).with_layout(
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height_percent(1.0),
    );
    options.base_z_index = SHOWCASE_WINDOW_Z_BASE;
    options.window_z_stride = SHOWCASE_WINDOW_Z_STRIDE;
    options.margin = 18.0;
    options.gap = 14.0;
    widgets::floating_desktop(
        ui,
        desktop,
        "showcase.windows",
        &windows,
        options,
        |ui, window, descriptor| match descriptor.id.as_str() {
            "labels" => labels(ui, window, state),
            "buttons" => buttons(ui, window, state),
            "checkbox" => checkbox(ui, window, state),
            "toggles" => toggles(ui, window, state),
            "slider" => slider(ui, window, state),
            "numeric" => numeric_inputs(ui, window, state),
            "text_input" => text_input(ui, window, state),
            "selection" => selection_widgets(ui, window, state),
            "menus" => menu_widgets(ui, window, state),
            "command_palette" => command_palette(ui, window, state),
            "date_picker" => date_picker(ui, window, state),
            "color_picker" => color_picker(ui, window, state),
            "color_buttons" => color_buttons(ui, window, state),
            "progress" => progress_indicator(ui, window, state),
            "lists_tables" => list_and_table_widgets(ui, window, state),
            "property_inspector" => property_inspector(ui, window, state),
            "trees" => tree_widgets(ui, window, state),
            "layout_widgets" => tab_split_dock_widgets(ui, window, state),
            "containers" => container_widgets(ui, window, state),
            "forms" => form_widgets(ui, window, state),
            "overlays" => overlay_widgets(ui, window, state),
            "drag_drop" => drag_drop_widgets(ui, window, state),
            "media" => media_widgets(ui, window),
            "timeline" => timeline_ruler(ui, window),
            "toasts" => toast_controls(ui, window, state),
            "popup_panel" => popup_controls(ui, window, state),
            "canvas" => canvas(ui, window),
            "styling" => styling_widgets(ui, window, state),
            _ => {}
        },
    );
    showcase_overlays(ui, desktop, state, desktop_size);
}

fn showcase_overlays(
    ui: &mut UiDocument,
    desktop: UiNodeId,
    state: &ShowcaseState,
    desktop_size: UiSize,
) {
    if state.toast_visible {
        let overlay_width = 320.0;
        let overlay = ui.add_child(
            desktop,
            UiNode::container(
                "showcase.toast_overlay",
                UiNodeStyle {
                    layout: LayoutStyle::absolute_rect(UiRect::new(
                        (desktop_size.width - overlay_width - 18.0).max(18.0),
                        18.0,
                        overlay_width,
                        180.0,
                    ))
                    .as_taffy_style()
                    .clone(),
                    clip: ClipBehavior::None,
                    z_index: 6000,
                    ..Default::default()
                },
            ),
        );
        let mut stack = widgets::ToastStack::new(3);
        stack.push_toast(
            widgets::Toast::new(
                widgets::ToastId(1),
                widgets::ToastSeverity::Success,
                "Saved",
                Some("All changes are written".to_string()),
                None,
            )
            .with_action(widgets::ToastAction::new("undo", "Undo")),
        );
        stack.push(
            widgets::ToastSeverity::Warning,
            "Autosave paused",
            Some("Changes are kept locally".to_string()),
            None,
        );
        let mut options = widgets::ToastStackOptions::default();
        options.z_index = 6100;
        widgets::toast_stack(ui, overlay, "showcase.toast_overlay.stack", &stack, options);
    }

    if state.popup_open {
        let popup_width = 280.0;
        let popup_height = 110.0;
        let popup = widgets::popup_panel(
            ui,
            desktop,
            "showcase.popup_overlay",
            UiRect::new(
                (desktop_size.width - popup_width - 36.0).max(18.0),
                220.0_f32.min((desktop_size.height - popup_height - 18.0).max(18.0)),
                popup_width,
                popup_height,
            ),
            widgets::PopupOptions {
                z_index: 6100,
                accessibility: Some(
                    AccessibilityMeta::new(AccessibilityRole::Dialog).label("Popup panel"),
                ),
                ..Default::default()
            },
        );
        let body = ui.add_child(
            popup,
            UiNode::container(
                "showcase.popup_overlay.body",
                LayoutStyle::column()
                    .with_width_percent(1.0)
                    .with_height_percent(1.0)
                    .padding(12.0)
                    .gap(8.0),
            ),
        );
        let header = row(ui, body, "showcase.popup_overlay.header", 8.0);
        widgets::label(
            ui,
            header,
            "showcase.popup_overlay.title",
            "Popup panel",
            text(13.0, color(240, 244, 250)),
            LayoutStyle::new().with_width_percent(1.0),
        );
        let mut close =
            widgets::ButtonOptions::new(LayoutStyle::size(28.0, 24.0)).with_action("popup.close");
        close.visual = UiVisual::panel(color(28, 34, 43), None, 3.0);
        close.hovered_visual = Some(button_visual(54, 70, 92));
        close.text_style = text(13.0, color(220, 228, 238));
        widgets::button(ui, header, "showcase.popup_overlay.close", "x", close);
        widgets::label(
            ui,
            body,
            "showcase.popup_overlay.body_text",
            "This surface is rendered as an overlay.",
            text(12.0, color(196, 210, 230)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }
}

fn showcase_window_descriptors(
    state: &ShowcaseState,
    desktop_size: UiSize,
) -> Vec<widgets::FloatingWindowDescriptor> {
    let wide = (desktop_size.width - 36.0).min(720.0).max(320.0);
    let medium = (desktop_size.width - 36.0).min(604.0).max(300.0);
    let buttons_width = medium.min(620.0);
    let mut windows = Vec::new();
    push_window(
        &mut windows,
        state.windows.labels,
        "labels",
        "Labels",
        UiSize::new(380.0, 460.0),
    );
    push_window(
        &mut windows,
        state.windows.buttons,
        "buttons",
        "Buttons",
        UiSize::new(buttons_width, 220.0),
    );
    push_window(
        &mut windows,
        state.windows.checkbox,
        "checkbox",
        "Checkbox",
        UiSize::new(250.0, 72.0),
    );
    push_window(
        &mut windows,
        state.windows.toggles,
        "toggles",
        "Radio and toggles",
        UiSize::new(360.0, 320.0),
    );
    push_window(
        &mut windows,
        state.windows.slider,
        "slider",
        "Slider",
        UiSize::new(430.0, 560.0),
    );
    push_window(
        &mut windows,
        state.windows.numeric,
        "numeric",
        "Numeric input",
        UiSize::new(360.0, 180.0),
    );
    push_window(
        &mut windows,
        state.windows.text_input,
        "text_input",
        "Text input",
        UiSize::new(520.0, 560.0),
    );
    push_window(
        &mut windows,
        state.windows.selection,
        "selection",
        "Select controls",
        UiSize::new(360.0, 360.0),
    );
    push_window(
        &mut windows,
        state.windows.menus,
        "menus",
        "Menus",
        UiSize::new(wide, 520.0),
    );
    push_window(
        &mut windows,
        state.windows.command_palette,
        "command_palette",
        "Command palette",
        UiSize::new(520.0, 320.0),
    );
    push_window(
        &mut windows,
        state.windows.date_picker,
        "date_picker",
        "Date picker",
        UiSize::new(430.0, 390.0),
    );
    push_window(
        &mut windows,
        state.windows.color_picker,
        "color_picker",
        "Color picker",
        UiSize::new(340.0, 390.0),
    );
    push_window(
        &mut windows,
        state.windows.color_buttons,
        "color_buttons",
        "Color buttons",
        UiSize::new(430.0, 360.0),
    );
    push_window(
        &mut windows,
        state.windows.progress,
        "progress",
        "Progress indicator",
        UiSize::new(500.0, 168.0),
    );
    push_window(
        &mut windows,
        state.windows.lists_tables,
        "lists_tables",
        "Lists and tables",
        UiSize::new(wide, 620.0),
    );
    push_window(
        &mut windows,
        state.windows.property_inspector,
        "property_inspector",
        "Property inspector",
        UiSize::new(330.0, 250.0),
    );
    push_window(
        &mut windows,
        state.windows.trees,
        "trees",
        "Trees",
        UiSize::new(430.0, 390.0),
    );
    push_window(
        &mut windows,
        state.windows.layout_widgets,
        "layout_widgets",
        "Layout widgets",
        UiSize::new(wide.min(560.0), 400.0),
    );
    push_window(
        &mut windows,
        state.windows.containers,
        "containers",
        "Containers",
        UiSize::new(560.0, 640.0),
    );
    push_window(
        &mut windows,
        state.windows.forms,
        "forms",
        "Forms",
        UiSize::new(460.0, 520.0),
    );
    push_window(
        &mut windows,
        state.windows.overlays,
        "overlays",
        "Overlays",
        UiSize::new(560.0, 500.0),
    );
    push_window(
        &mut windows,
        state.windows.drag_drop,
        "drag_drop",
        "Drag and drop",
        UiSize::new(390.0, 340.0),
    );
    push_window(
        &mut windows,
        state.windows.media,
        "media",
        "Media",
        UiSize::new(360.0, 280.0),
    );
    push_window(
        &mut windows,
        state.windows.timeline,
        "timeline",
        "Timeline",
        UiSize::new(600.0, 120.0),
    );
    push_window(
        &mut windows,
        state.windows.toasts,
        "toasts",
        "Toasts",
        UiSize::new(320.0, 270.0),
    );
    push_window(
        &mut windows,
        state.windows.popup_panel,
        "popup_panel",
        "Popup panel",
        UiSize::new(360.0, 200.0),
    );
    push_window(
        &mut windows,
        state.windows.canvas,
        "canvas",
        "Canvas",
        UiSize::new(420.0, 292.0),
    );
    push_window(
        &mut windows,
        state.windows.styling,
        "styling",
        "Styling",
        UiSize::new(640.0, 560.0),
    );
    for window in &mut windows {
        window.drag_action = Some(WidgetActionBinding::action(format!(
            "window.drag.{}",
            window.id
        )));
        window.collapse_action = Some(WidgetActionBinding::action(format!(
            "window.collapse.{}",
            window.id
        )));
        window.resize_action = Some(WidgetActionBinding::action(format!(
            "window.resize.{}",
            window.id
        )));
        state
            .desktop
            .apply_to_descriptor(window, window_defaults(window.id.as_str()));
    }
    windows
}

fn push_window(
    windows: &mut Vec<widgets::FloatingWindowDescriptor>,
    visible: bool,
    id: &'static str,
    title: &'static str,
    preferred_size: UiSize,
) {
    if visible {
        windows.push(
            widgets::FloatingWindowDescriptor::new(id, title, preferred_size)
                .with_min_size(default_window_state_min_size(id))
                .with_auto_size_to_content(true)
                .with_activate_action(format!("window.activate.{id}"))
                .with_close_action(format!("window.close.{id}")),
        );
    }
}

fn default_window_size(id: &str) -> UiSize {
    match id {
        "labels" => UiSize::new(380.0, 460.0),
        "buttons" => UiSize::new(604.0, 220.0),
        "checkbox" => UiSize::new(250.0, 72.0),
        "toggles" => UiSize::new(360.0, 380.0),
        "slider" => UiSize::new(430.0, 560.0),
        "numeric" => UiSize::new(430.0, 180.0),
        "text_input" => UiSize::new(520.0, 640.0),
        "selection" => UiSize::new(360.0, 360.0),
        "menus" => UiSize::new(640.0, 640.0),
        "command_palette" => UiSize::new(520.0, 320.0),
        "date_picker" => UiSize::new(430.0, 390.0),
        "color_picker" => UiSize::new(340.0, 390.0),
        "color_buttons" => UiSize::new(430.0, 360.0),
        "progress" => UiSize::new(500.0, 168.0),
        "lists_tables" => UiSize::new(600.0, 700.0),
        "property_inspector" => UiSize::new(330.0, 250.0),
        "trees" => UiSize::new(430.0, 450.0),
        "layout_widgets" => UiSize::new(560.0, 400.0),
        "containers" => UiSize::new(560.0, 640.0),
        "forms" => UiSize::new(460.0, 520.0),
        "overlays" => UiSize::new(560.0, 500.0),
        "drag_drop" => UiSize::new(390.0, 340.0),
        "media" => UiSize::new(360.0, 280.0),
        "timeline" => UiSize::new(600.0, 120.0),
        "toasts" => UiSize::new(320.0, 270.0),
        "popup_panel" => UiSize::new(360.0, 200.0),
        "canvas" => UiSize::new(420.0, 292.0),
        "styling" => UiSize::new(640.0, 560.0),
        _ => UiSize::new(300.0, 180.0),
    }
}

fn default_window_state_min_size(_id: &str) -> UiSize {
    UiSize::new(160.0, 96.0)
}

fn default_window_position(id: &str) -> UiPoint {
    match id {
        "labels" => UiPoint::new(18.0, 18.0),
        "buttons" => UiPoint::new(420.0, 18.0),
        "checkbox" => UiPoint::new(360.0, 18.0),
        "toggles" => UiPoint::new(360.0, 110.0),
        "slider" => UiPoint::new(360.0, 110.0),
        "numeric" => UiPoint::new(360.0, 260.0),
        "text_input" => UiPoint::new(360.0, 18.0),
        "selection" => UiPoint::new(360.0, 404.0),
        "menus" => UiPoint::new(18.0, 18.0),
        "command_palette" => UiPoint::new(68.0, 88.0),
        "date_picker" => UiPoint::new(300.0, 170.0),
        "color_picker" => UiPoint::new(18.0, 560.0),
        "color_buttons" => UiPoint::new(380.0, 500.0),
        "progress" => UiPoint::new(72.0, 540.0),
        "lists_tables" => UiPoint::new(18.0, 90.0),
        "property_inspector" => UiPoint::new(300.0, 420.0),
        "trees" => UiPoint::new(36.0, 220.0),
        "layout_widgets" => UiPoint::new(18.0, 18.0),
        "containers" => UiPoint::new(48.0, 120.0),
        "forms" => UiPoint::new(120.0, 160.0),
        "overlays" => UiPoint::new(80.0, 110.0),
        "drag_drop" => UiPoint::new(210.0, 250.0),
        "media" => UiPoint::new(120.0, 360.0),
        "timeline" => UiPoint::new(18.0, 620.0),
        "toasts" => UiPoint::new(320.0, 70.0),
        "popup_panel" => UiPoint::new(320.0, 370.0),
        "canvas" => UiPoint::new(380.0, 560.0),
        "styling" => UiPoint::new(86.0, 118.0),
        _ => UiPoint::new(18.0, 18.0),
    }
}

fn window_for_action(action_id: &str) -> Option<&'static str> {
    match action_id {
        id if id.starts_with("labels.") => Some("labels"),
        id if id.starts_with("button.") => Some("buttons"),
        id if id.starts_with("checkbox.") => Some("checkbox"),
        id if id.starts_with("toggles.") => Some("toggles"),
        id if id.starts_with("theme.preference.") => Some("toggles"),
        id if id.starts_with("slider.") => Some("slider"),
        id if id.starts_with("numeric.") => Some("numeric"),
        id if id.starts_with("text.") => Some("text_input"),
        id if id.starts_with("combo.")
            || id.starts_with("selection.dropdown.")
            || id.starts_with("selection.menu.") =>
        {
            Some("selection")
        }
        id if id.starts_with("menus.") => Some("menus"),
        id if id.starts_with("command_palette.") => Some("command_palette"),
        id if id.starts_with("date.") => Some("date_picker"),
        id if id.starts_with("color.") => Some("color_picker"),
        id if id.starts_with("color_buttons.") => Some("color_buttons"),
        id if id.starts_with("progress.") => Some("progress"),
        id if id.starts_with("lists_tables.") => Some("lists_tables"),
        id if id.starts_with("property_inspector.") => Some("property_inspector"),
        id if id.starts_with("trees.") => Some("trees"),
        id if id.starts_with("layout.") || id.starts_with("layout_widgets.") => {
            Some("layout_widgets")
        }
        id if id.starts_with("containers.") => Some("containers"),
        id if id.starts_with("forms.") => Some("forms"),
        id if id.starts_with("overlays.") => Some("overlays"),
        id if id.starts_with("drag_drop.") => Some("drag_drop"),
        id if id.starts_with("media.") => Some("media"),
        id if id.starts_with("toast.") => Some("toasts"),
        id if id.starts_with("popup.") => Some("popup_panel"),
        id if id.starts_with("canvas.") => Some("canvas"),
        id if id.starts_with("styling.") => Some("styling"),
        _ => None,
    }
}

fn focused_text_for_action(action_id: &str) -> Option<FocusedTextInput> {
    Some(match action_id {
        "text.input.edit" => FocusedTextInput::Editable,
        "text.selectable.edit" => FocusedTextInput::Selectable,
        "text.singleline.edit" => FocusedTextInput::Singleline,
        "text.multiline.edit" => FocusedTextInput::Multiline,
        "text.area.edit" => FocusedTextInput::TextArea,
        "text.code_editor.edit" => FocusedTextInput::CodeEditor,
        "text.search.edit" => FocusedTextInput::Search,
        "text.password.edit" => FocusedTextInput::Password,
        "text.selectable_helper.edit" => FocusedTextInput::SelectableHelper,
        "slider.value_text.edit" => FocusedTextInput::SliderValue,
        "slider.left_text.edit" => FocusedTextInput::SliderRangeLeft,
        "slider.right_text.edit" => FocusedTextInput::SliderRangeRight,
        "slider.step_text.edit" => FocusedTextInput::SliderStep,
        _ => return None,
    })
}

fn control_panel(
    ui: &mut UiDocument,
    parent: UiNodeId,
    state: &ShowcaseState,
    viewport_height: f32,
) {
    widgets::label(
        ui,
        parent,
        "controls.title",
        "Widgets",
        text(16.0, color(244, 248, 252)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let list_viewport_height = controls_list_viewport_height(viewport_height);
    let list_row = ui.add_child(
        parent,
        UiNode::container(
            "controls.widget_list.row",
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height(list_viewport_height)
                .with_flex_grow(1.0)
                .with_flex_shrink(1.0)
                .gap(6.0),
        ),
    );
    let list = widgets::scroll_area(
        ui,
        list_row,
        "controls.widget_list",
        ScrollAxes::VERTICAL,
        LayoutStyle::column()
            .with_width(0.0)
            .with_height_percent(1.0)
            .with_flex_grow(1.0)
            .gap(CONTROLS_WIDGET_ROW_GAP),
    );
    ui.node_mut(list).action = Some("controls.widget_list.scroll".into());
    if let Some(scroll) = ui.node_mut(list).scroll.as_mut() {
        scroll.offset.y = controls_scroll_state(state.controls_scroll, list_viewport_height)
            .offset
            .y;
    }
    widgets::scrollbar(
        ui,
        list_row,
        "controls.widget_list.scrollbar",
        controls_scroll_state(state.controls_scroll, list_viewport_height),
        widgets::ScrollAxis::Vertical,
        widgets::ScrollbarOptions::default()
            .with_layout(LayoutStyle::new().with_width(8.0).with_height_percent(1.0))
            .with_track_size(UiSize::new(8.0, list_viewport_height))
            .with_action("controls.widget_list.scrollbar"),
    );

    window_toggle(ui, list, "labels", "Labels", state.windows.labels);
    window_toggle(ui, list, "buttons", "Buttons", state.windows.buttons);
    window_toggle(ui, list, "checkbox", "Checkbox", state.windows.checkbox);
    window_toggle(
        ui,
        list,
        "toggles",
        "Radio and toggles",
        state.windows.toggles,
    );
    window_toggle(ui, list, "slider", "Slider", state.windows.slider);
    window_toggle(ui, list, "numeric", "Numeric input", state.windows.numeric);
    window_toggle(
        ui,
        list,
        "text_input",
        "Text input",
        state.windows.text_input,
    );
    window_toggle(
        ui,
        list,
        "selection",
        "Select controls",
        state.windows.selection,
    );
    window_toggle(ui, list, "menus", "Menus", state.windows.menus);
    window_toggle(
        ui,
        list,
        "command_palette",
        "Command palette",
        state.windows.command_palette,
    );
    window_toggle(
        ui,
        list,
        "date_picker",
        "Date picker",
        state.windows.date_picker,
    );
    window_toggle(
        ui,
        list,
        "color_picker",
        "Color picker",
        state.windows.color_picker,
    );
    window_toggle(
        ui,
        list,
        "color_buttons",
        "Color buttons",
        state.windows.color_buttons,
    );
    window_toggle(
        ui,
        list,
        "progress",
        "Progress indicator",
        state.windows.progress,
    );
    window_toggle(
        ui,
        list,
        "lists_tables",
        "Lists and tables",
        state.windows.lists_tables,
    );
    window_toggle(
        ui,
        list,
        "property_inspector",
        "Property inspector",
        state.windows.property_inspector,
    );
    window_toggle(ui, list, "trees", "Trees", state.windows.trees);
    window_toggle(
        ui,
        list,
        "layout_widgets",
        "Layout widgets",
        state.windows.layout_widgets,
    );
    window_toggle(
        ui,
        list,
        "containers",
        "Containers",
        state.windows.containers,
    );
    window_toggle(ui, list, "forms", "Forms", state.windows.forms);
    window_toggle(ui, list, "overlays", "Overlays", state.windows.overlays);
    window_toggle(
        ui,
        list,
        "drag_drop",
        "Drag and drop",
        state.windows.drag_drop,
    );
    window_toggle(ui, list, "media", "Media", state.windows.media);
    window_toggle(ui, list, "timeline", "Timeline", state.windows.timeline);
    window_toggle(ui, list, "toasts", "Toasts", state.windows.toasts);
    window_toggle(
        ui,
        list,
        "popup_panel",
        "Popup panel",
        state.windows.popup_panel,
    );
    window_toggle(ui, list, "canvas", "Canvas", state.windows.canvas);
    window_toggle(ui, list, "styling", "Styling", state.windows.styling);

    ui.add_child(
        parent,
        UiNode::container(
            "controls.clear_all.spacer",
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(1.0)
                .with_flex_grow(1.0)
                .with_flex_shrink(1.0),
        ),
    );
    let mut clear =
        widgets::ButtonOptions::new(LayoutStyle::new().with_width_percent(1.0).with_height(30.0))
            .with_action("window.clear_all");
    clear.visual = UiVisual::panel(
        color(31, 38, 48),
        Some(StrokeStyle::new(color(76, 88, 106), 1.0)),
        4.0,
    );
    clear.hovered_visual = Some(UiVisual::panel(
        color(45, 56, 70),
        Some(StrokeStyle::new(color(118, 144, 174), 1.0)),
        4.0,
    ));
    clear.pressed_visual = Some(UiVisual::panel(
        color(20, 27, 36),
        Some(StrokeStyle::new(color(82, 104, 132), 1.0)),
        4.0,
    ));
    clear.pressed_hovered_visual = Some(UiVisual::panel(
        color(36, 48, 62),
        Some(StrokeStyle::new(color(138, 170, 206), 1.0)),
        4.0,
    ));
    clear.text_style = text(12.0, color(230, 236, 246));
    clear.accessibility_label = Some("Clear all widgets".to_string());
    widgets::button(ui, parent, "controls.clear_all", "Clear all", clear);
}

fn window_toggle(
    ui: &mut UiDocument,
    parent: UiNodeId,
    id: &'static str,
    label: &'static str,
    checked: bool,
) {
    let mut options =
        widgets::CheckboxOptions::default().with_action(format!("window.toggle.{id}"));
    options.layout = LayoutStyle::new()
        .with_width_percent(1.0)
        .with_height(CONTROLS_WIDGET_ROW_HEIGHT);
    options.text_style = text(12.0, color(220, 228, 238));
    widgets::checkbox(
        ui,
        parent,
        format!("controls.{id}"),
        label,
        checked,
        options,
    );
}

fn labels(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "labels", "Labels");
    ui.node_mut(body).style.layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_height_percent(1.0)
        .with_flex_grow(1.0)
        .gap(10.0)
        .as_taffy_style()
        .clone();
    widgets::label(
        ui,
        body,
        "labels.plain",
        "Plain label",
        text(13.0, color(226, 232, 242)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let locale_items = label_locale_options();
    let locale_id = state
        .label_locale
        .selected_id(&locale_items)
        .unwrap_or("es-MX");
    let localization = LocalizationPolicy::new(LocaleId::new(locale_id).expect("valid locale"));
    let locale_row = ui.add_child(
        body,
        UiNode::container(
            "labels.locale.row",
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_align_items(taffy::prelude::AlignItems::Center)
                .gap(10.0),
        ),
    );
    let locale_label_width = 270.0;
    let locale_dropdown_width = 148.0;
    let locale_gap = 10.0;
    widgets::localized_label(
        ui,
        locale_row,
        "labels.localized",
        DynamicLabelMeta::keyed("showcase.localized.greeting", localized_label(locale_id)),
        Some(&localization),
        text(13.0, color(170, 202, 255)),
        LayoutStyle::new().with_width(locale_label_width),
    );
    let mut locale_options = widgets::DropdownSelectOptions::default();
    locale_options.trigger_layout = LayoutStyle::row()
        .with_width(locale_dropdown_width)
        .with_height(30.0)
        .with_align_items(taffy::prelude::AlignItems::Center)
        .with_justify_content(taffy::prelude::JustifyContent::Center)
        .padding(6.0);
    locale_options.text_style = text(13.0, color(226, 232, 242));
    locale_options.accessibility_label = Some("Locale".to_string());
    locale_options.menu = widgets::SelectMenuOptions::default().with_action_prefix("labels.locale");
    locale_options.menu.width = locale_dropdown_width;
    locale_options.menu.row_height = 30.0;
    locale_options.menu.max_visible_rows = locale_items.len();
    locale_options.menu.text_style = text(13.0, color(226, 232, 242));
    let locale_nodes = widgets::dropdown_select(
        ui,
        locale_row,
        "labels.locale",
        &locale_items,
        &state.label_locale,
        Some(widgets::AnchoredPopup::new(
            UiRect::new(
                locale_label_width + locale_gap,
                0.0,
                locale_dropdown_width,
                30.0,
            ),
            UiRect::new(0.0, 0.0, 460.0, 260.0),
            widgets::PopupPlacement::default(),
        )),
        locale_options,
    );
    ui.node_mut(locale_nodes.trigger).action = Some("labels.locale.toggle".into());
    widgets::label(
        ui,
        body,
        "labels.muted",
        "Muted helper label",
        text(12.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let sizes = ui.add_child(
        body,
        UiNode::container(
            "labels.sizes",
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_align_items(taffy::prelude::AlignItems::FlexEnd)
                .gap(12.0),
        ),
    );
    widgets::label(
        ui,
        sizes,
        "labels.size.small",
        "12px",
        text(12.0, color(226, 232, 242)),
        LayoutStyle::new(),
    );
    widgets::label(
        ui,
        sizes,
        "labels.size.default",
        "13px",
        text(13.0, color(226, 232, 242)),
        LayoutStyle::new(),
    );
    widgets::label(
        ui,
        sizes,
        "labels.size.large",
        "18px",
        text(18.0, color(246, 249, 252)),
        LayoutStyle::new(),
    );
    widgets::label(
        ui,
        sizes,
        "labels.size.display",
        "24px",
        text(24.0, color(246, 249, 252)),
        LayoutStyle::new(),
    );

    let style_row = row(ui, body, "labels.styles", 12.0);
    let mut bold = text(13.0, color(246, 249, 252));
    bold.weight = FontWeight::BOLD;
    widgets::label(
        ui,
        style_row,
        "labels.style.bold",
        "Bold",
        bold,
        LayoutStyle::new(),
    );
    widgets::label(
        ui,
        style_row,
        "labels.style.weak",
        "Muted",
        text(13.0, color(154, 166, 184)),
        LayoutStyle::new(),
    );

    let font_row = row(ui, body, "labels.fonts", 12.0);
    let mut serif = text(13.0, color(226, 232, 242));
    serif.family = FontFamily::Serif;
    widgets::label(
        ui,
        font_row,
        "labels.font.serif",
        "Serif",
        serif,
        LayoutStyle::new(),
    );
    let mut mono = text(13.0, color(226, 232, 242));
    mono.family = FontFamily::Monospace;
    widgets::label(
        ui,
        font_row,
        "labels.font.mono",
        "Monospace",
        mono,
        LayoutStyle::new(),
    );

    let code_panel = ui.add_child(
        body,
        UiNode::container(
            "labels.code.panel",
            LayoutStyle::new()
                .with_width_percent(1.0)
                .padding(8.0)
                .with_height(36.0),
        )
        .with_visual(UiVisual::panel(
            color(10, 14, 20),
            Some(StrokeStyle::new(color(47, 59, 74), 1.0)),
            4.0,
        )),
    );
    widgets::code_label(
        ui,
        code_panel,
        "labels.code",
        "let label = widgets::label(...);",
        LayoutStyle::new().with_width_percent(1.0),
    );

    let colors = row(ui, body, "labels.colors", 14.0);
    widgets::colored_label(
        ui,
        colors,
        "labels.color.green",
        "Green",
        color(111, 203, 159),
        LayoutStyle::new(),
    );
    widgets::colored_label(
        ui,
        colors,
        "labels.color.yellow",
        "Yellow",
        color(232, 196, 101),
        LayoutStyle::new(),
    );
    widgets::colored_label(
        ui,
        colors,
        "labels.color.red",
        "Red",
        color(244, 118, 118),
        LayoutStyle::new(),
    );

    let wrap_row = wrapping_row(ui, body, "labels.wrap.row", 10.0);
    let wrap_word = ui.add_child(
        wrap_row,
        UiNode::container(
            "labels.wrap.word.panel",
            LayoutStyle::column().with_width(172.0).padding(8.0),
        )
        .with_visual(UiVisual::panel(
            color(18, 23, 31),
            Some(StrokeStyle::new(color(47, 59, 74), 1.0)),
            4.0,
        )),
    );
    widgets::wrapped_label(
        ui,
        wrap_word,
        "labels.wrap.word",
        "Word wrapping keeps this sentence readable in a narrow box.",
        TextWrap::Word,
        LayoutStyle::new().with_width_percent(1.0),
    );
    let wrap_glyph = ui.add_child(
        wrap_row,
        UiNode::container(
            "labels.wrap.glyph.panel",
            LayoutStyle::column().with_width(172.0).padding(8.0),
        )
        .with_visual(UiVisual::panel(
            color(18, 23, 31),
            Some(StrokeStyle::new(color(47, 59, 74), 1.0)),
            4.0,
        )),
    );
    widgets::wrapped_label(
        ui,
        wrap_glyph,
        "labels.wrap.glyph",
        "LongIdentifierWithoutSpaces",
        TextWrap::Glyph,
        LayoutStyle::new().with_width_percent(1.0),
    );

    let links = wrapping_row(ui, body, "labels.links", 12.0);
    widgets::link(
        ui,
        links,
        "labels.link",
        "Internal action",
        widgets::LinkOptions::default()
            .visited(state.label_link_visited)
            .with_action("labels.link"),
    );
    widgets::hyperlink(
        ui,
        links,
        "labels.hyperlink",
        "Open docs.rs",
        "https://docs.rs/operad",
        widgets::LinkOptions::default()
            .visited(state.label_hyperlink_visited)
            .with_action("labels.hyperlink"),
    );
    if state.label_link_status != "No link action yet" {
        widgets::label(
            ui,
            body,
            "labels.status",
            format!("Last action: {}", state.label_link_status),
            text(12.0, color(154, 166, 184)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }
}

fn buttons(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "buttons", "Buttons");
    let primary_row = wrapping_row(ui, body, "buttons.row", 10.0);
    button(
        ui,
        primary_row,
        "button.default",
        "Default",
        "button.default",
        button_visual(38, 46, 58),
    );
    button(
        ui,
        primary_row,
        "button.primary",
        "Primary",
        "button.primary",
        button_visual(48, 112, 184),
    );
    button(
        ui,
        primary_row,
        "button.secondary",
        "Secondary",
        "button.secondary",
        button_visual(58, 78, 96),
    );
    button(
        ui,
        primary_row,
        "button.destructive",
        "Destructive",
        "button.destructive",
        button_visual(157, 65, 73),
    );
    let mut disabled = widgets::ButtonOptions::new(LayoutStyle::size(92.0, 32.0));
    disabled.enabled = false;
    disabled.visual = button_visual(40, 44, 52);
    disabled.text_style = text(13.0, color(138, 146, 158));
    widgets::button(ui, primary_row, "button.disabled", "Disabled", disabled);
    let second_row = wrapping_row(ui, body, "buttons.row.options", 10.0);
    button(
        ui,
        second_row,
        "button.momentary",
        "Press only",
        "button.default",
        button_visual(42, 50, 62),
    );
    let mut toggle =
        widgets::ButtonOptions::new(LayoutStyle::size(112.0, 32.0)).with_action("button.toggle");
    toggle.pressed = state.toggle_button;
    toggle.visual = button_visual(42, 50, 62);
    toggle.hovered_visual = Some(button_visual(62, 74, 92));
    toggle.pressed_visual = Some(button_visual(86, 64, 156));
    toggle.pressed_hovered_visual = Some(button_visual(126, 94, 218));
    toggle.text_style = text(13.0, color(246, 249, 252));
    widgets::button(
        ui,
        second_row,
        "button.toggle",
        if state.toggle_button {
            "Toggle on"
        } else {
            "Toggle off"
        },
        toggle,
    );
    let mut forced_pressed = widgets::ButtonOptions::new(LayoutStyle::size(112.0, 32.0));
    forced_pressed.pressed = true;
    forced_pressed.visual = button_visual(42, 50, 62);
    forced_pressed.hovered_visual = Some(button_visual(62, 74, 92));
    forced_pressed.pressed_visual = Some(button_visual(38, 82, 136));
    forced_pressed.pressed_hovered_visual = Some(button_visual(62, 126, 196));
    forced_pressed.text_style = text(13.0, color(246, 249, 252));
    widgets::button(
        ui,
        second_row,
        "button.state.pressed",
        "Pressed",
        forced_pressed,
    );
    let helper_row = wrapping_row(ui, body, "buttons.row.helpers", 10.0);
    widgets::small_button(
        ui,
        helper_row,
        "button.small",
        "Small",
        widgets::ButtonOptions::default().with_action("button.small"),
    );
    widgets::icon_button(
        ui,
        helper_row,
        "button.icon",
        icon_image(BuiltInIcon::Settings),
        "Settings",
        widgets::ButtonOptions::default().with_action("button.icon"),
    );
    widgets::image_button(
        ui,
        helper_row,
        "button.image",
        icon_image(BuiltInIcon::Folder),
        "Folder",
        widgets::ButtonOptions::default().with_action("button.image"),
    );
    widgets::reset_button(
        ui,
        helper_row,
        "button.reset",
        state.toggle_button,
        widgets::ButtonOptions::default().with_action("button.reset"),
    );
    widgets::toggle_button(
        ui,
        helper_row,
        "button.toggle_helper",
        "Toggle helper",
        state.toggle_button,
        widgets::ButtonOptions::default().with_action("button.toggle"),
    );
    widgets::label(
        ui,
        body,
        "buttons.last",
        format!("Last pressed: {}", state.last_button),
        text(12.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn checkbox(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "checkbox", "Checkbox");
    let mut options = widgets::CheckboxOptions::default().with_action("checkbox.enabled");
    options.text_style = text(13.0, color(222, 228, 238));
    widgets::checkbox(
        ui,
        body,
        "checkbox.enabled",
        if state.checked { "Enabled" } else { "Disabled" },
        state.checked,
        options,
    );
}

fn toggles(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "toggles", "Radio and toggles");
    let radio_options = [
        widgets::RadioOption::new("compact", "Compact").with_action("toggles.radio.compact"),
        widgets::RadioOption::new("comfortable", "Comfortable")
            .with_action("toggles.radio.comfortable"),
        widgets::RadioOption::new("spacious", "Spacious").with_action("toggles.radio.spacious"),
        widgets::RadioOption::new("disabled", "Disabled").enabled(false),
    ];
    widgets::radio_group(
        ui,
        body,
        "toggles.radio_group",
        &radio_options,
        Some(state.radio_choice),
        widgets::RadioGroupOptions::default(),
    );
    widgets::radio_button(
        ui,
        body,
        "toggles.radio_single",
        "Standalone radio button",
        true,
        widgets::RadioButtonOptions::default().with_action("toggles.radio.compact"),
    );
    widgets::toggle_switch(
        ui,
        body,
        "toggles.switch",
        if state.switch_enabled {
            "Switch on"
        } else {
            "Switch off"
        },
        widgets::ToggleValue::from(state.switch_enabled),
        widgets::ToggleSwitchOptions::default().with_action("toggles.switch"),
    );
    widgets::toggle_switch(
        ui,
        body,
        "toggles.mixed",
        match state.mixed_switch {
            widgets::ToggleValue::Mixed => "Mixed switch",
            widgets::ToggleValue::On => "Mixed switch on",
            widgets::ToggleValue::Off => "Mixed switch off",
        },
        state.mixed_switch,
        widgets::ToggleSwitchOptions::default().with_action("toggles.mixed"),
    );
    widgets::theme_preference_buttons(
        ui,
        body,
        "toggles.theme_buttons",
        state.theme_preference,
        widgets::ThemePreferenceButtonsOptions::default().with_action_prefix("toggles.theme"),
    );
    widgets::theme_preference_switch(
        ui,
        body,
        "toggles.theme_switch",
        state.theme_preference,
        widgets::ThemePreferenceSwitchOptions::default().with_action("theme.preference.dark"),
    );
}

fn slider(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "slider", "Slider");
    widgets::label(
        ui,
        body,
        "slider.note",
        "Click a slider value to edit it with the keyboard.",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let value_row = row(ui, body, "slider.value.row", 10.0);
    let options = slider_options(state, 180.0).with_value_edit_action("slider.value");
    let slider_unit = state.slider_value_spec().normalize(state.slider);
    widgets::slider(
        ui,
        value_row,
        "slider.value",
        slider_unit,
        0.0..1.0,
        options.clone(),
    );
    slider_number_input(
        ui,
        value_row,
        "slider.value_text",
        &state.slider_value_text,
        FocusedTextInput::SliderValue,
        state,
        86.0,
    );
    widgets::label(
        ui,
        value_row,
        "slider.value.label",
        "f64 demo slider",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    widgets::label(
        ui,
        body,
        "slider.precision",
        format!(
            "Displayed value: {}    Full precision: {:.6}",
            widgets::format_slider_value(state.slider),
            state.slider
        ),
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    divider(ui, body, "slider.divider.range");
    widgets::label(
        ui,
        body,
        "slider.range.label",
        "Slider range",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let left_row = row(ui, body, "slider.range.left.row", 10.0);
    let left_options = widgets::SliderOptions::default()
        .with_layout(
            LayoutStyle::new()
                .with_width(180.0)
                .with_height(24.0)
                .with_flex_shrink(0.0),
        )
        .with_value_edit_action("slider.range_left");
    widgets::slider(
        ui,
        left_row,
        "slider.range_left",
        state.slider_left,
        0.0..state.slider_right.max(1.0),
        left_options,
    );
    slider_number_input(
        ui,
        left_row,
        "slider.left_text",
        &state.slider_left_text,
        FocusedTextInput::SliderRangeLeft,
        state,
        96.0,
    );
    widgets::label(
        ui,
        left_row,
        "slider.range.left.label",
        "left",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width(46.0),
    );
    let right_row = row(ui, body, "slider.range.right.row", 10.0);
    let right_options = widgets::SliderOptions::default()
        .with_layout(
            LayoutStyle::new()
                .with_width(180.0)
                .with_height(24.0)
                .with_flex_shrink(0.0),
        )
        .with_value_edit_action("slider.range_right");
    widgets::slider(
        ui,
        right_row,
        "slider.range_right",
        state.slider_right,
        (state.slider_left + 1.0)..10000.0,
        right_options,
    );
    slider_number_input(
        ui,
        right_row,
        "slider.right_text",
        &state.slider_right_text,
        FocusedTextInput::SliderRangeRight,
        state,
        96.0,
    );
    widgets::label(
        ui,
        right_row,
        "slider.range.right.label",
        "right",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width(46.0),
    );

    divider(ui, body, "slider.divider.trailing");
    let trailing_row = row(ui, body, "slider.trailing.row", 8.0);
    slider_checkbox_with_layout(
        ui,
        trailing_row,
        "slider.trailing",
        "Trailing color",
        state.slider_trailing_color,
        LayoutStyle::new()
            .with_width(142.0)
            .with_height(30.0)
            .with_flex_shrink(0.0),
    );
    widgets::color_edit_button_rgb(
        ui,
        trailing_row,
        "slider.trailing_color_button",
        state.slider_trailing_picker.value,
        color_square_button_options("slider.trailing_color_button")
            .accessibility_label("Pick trailing slider color"),
    );
    if state.slider_trailing_picker_open {
        widgets::color_picker(
            ui,
            body,
            "slider.trailing_picker",
            &state.slider_trailing_picker,
            widgets::ColorPickerOptions::default()
                .with_label("Trailing slider color")
                .with_action_prefix("slider.trailing_picker"),
        );
    }
    let thumb_row = row(ui, body, "slider.thumb.row", 8.0);
    widgets::label(
        ui,
        thumb_row,
        "slider.thumb.label",
        "Thumb",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width(64.0),
    );
    choice_button(
        ui,
        thumb_row,
        "slider.thumb.circle",
        "Circle",
        state.slider_thumb_shape == SliderThumbChoice::Circle,
    );
    choice_button(
        ui,
        thumb_row,
        "slider.thumb.square",
        "Square",
        state.slider_thumb_shape == SliderThumbChoice::Square,
    );
    choice_button(
        ui,
        thumb_row,
        "slider.thumb.rectangle",
        "Rectangle",
        state.slider_thumb_shape == SliderThumbChoice::Rectangle,
    );
    slider_checkbox(
        ui,
        body,
        "slider.steps",
        "Use steps",
        state.slider_use_steps,
    );
    let step_row = row(ui, body, "slider.step.row", 10.0);
    widgets::label(
        ui,
        step_row,
        "slider.step.label",
        "Step value",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width(74.0),
    );
    slider_number_input(
        ui,
        step_row,
        "slider.step_text",
        &state.slider_step_text,
        FocusedTextInput::SliderStep,
        state,
        86.0,
    );
    slider_checkbox(
        ui,
        body,
        "slider.logarithmic",
        "Logarithmic",
        state.slider_logarithmic,
    );
    let clamp_row = row(ui, body, "slider.clamping.row", 8.0);
    widgets::label(
        ui,
        clamp_row,
        "slider.clamping.label",
        "Clamping",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width(74.0),
    );
    choice_button(
        ui,
        clamp_row,
        "slider.clamping.never",
        "Never",
        state.slider_clamping == widgets::SliderClamping::Never,
    );
    choice_button(
        ui,
        clamp_row,
        "slider.clamping.edits",
        "Edits",
        state.slider_clamping == widgets::SliderClamping::Edits,
    );
    choice_button(
        ui,
        clamp_row,
        "slider.clamping.always",
        "Always",
        state.slider_clamping == widgets::SliderClamping::Always,
    );
    slider_checkbox(
        ui,
        body,
        "slider.smart_aim",
        "Smart aim",
        state.slider_smart_aim,
    );
}

fn numeric_inputs(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "numeric", "Numeric input");
    let row_one = row(ui, body, "numeric.row.values", 10.0);
    widgets::drag_value_input(
        ui,
        row_one,
        "numeric.drag_value",
        state.numeric_value as f64,
        widgets::DragValueOptions::default()
            .with_range(widgets::NumericRange::new(0.0, 100.0))
            .with_precision(widgets::NumericPrecision::decimals(1))
            .with_unit(widgets::NumericUnitFormat::default().suffix(" px"))
            .with_action("numeric.drag_value"),
    );
    widgets::drag_angle(
        ui,
        row_one,
        "numeric.drag_angle",
        state.numeric_angle as f64,
        widgets::DragValueOptions::default().with_action("numeric.drag_angle"),
    );
    widgets::drag_angle_tau(
        ui,
        row_one,
        "numeric.drag_angle_tau",
        state.numeric_tau as f64,
        widgets::DragValueOptions::default().with_action("numeric.drag_angle_tau"),
    );
    widgets::label(
        ui,
        body,
        "numeric.note",
        "Drag values expose spinbutton semantics and unit-aware formatting.",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn selection_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "selection", "Select controls");
    let select_width = 180.0;

    widgets::label(
        ui,
        body,
        "selection.combo.label",
        "Combo box",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let mut options = widgets::ComboBoxOptions::default();
    options.accessibility_label = Some("Display density".to_string());
    options.text_style = text(13.0, color(230, 236, 246));
    options.layout = LayoutStyle::new()
        .with_width(select_width)
        .with_height(30.0);
    let combo = widgets::combo_box(
        ui,
        body,
        "combo.toggle",
        state.combo_label.clone(),
        state.combo_open,
        options,
    );
    ui.node_mut(combo).action = Some("combo.toggle".into());
    let select_options = select_options();
    if state.combo_open {
        widgets::select_menu_popup(
            ui,
            body,
            "selection.combo_menu",
            widgets::AnchoredPopup::new(
                UiRect::new(0.0, 27.0, select_width, 30.0),
                UiRect::new(0.0, 0.0, 320.0, 308.0),
                widgets::PopupPlacement::default(),
            ),
            &select_options,
            &widgets::SelectMenuState {
                open: true,
                selected: select_options
                    .iter()
                    .position(|option| option.label == state.combo_label),
                active: select_options
                    .iter()
                    .position(|option| option.label == state.combo_label),
            },
            select_menu_options(select_width).with_action_prefix("selection.combo"),
        );
    }

    widgets::label(
        ui,
        body,
        "selection.menu.label",
        "Select menu",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::select_menu(
        ui,
        body,
        "selection.select_menu",
        &select_options,
        &state.select_menu,
        widgets::SelectMenuOptions::default().with_action_prefix("selection.menu"),
    );
    widgets::label(
        ui,
        body,
        "selection.dropdown.label",
        "Dropdown select",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let mut dropdown_options = widgets::DropdownSelectOptions::default();
    dropdown_options.menu =
        select_menu_options(select_width).with_action_prefix("selection.dropdown");
    let dropdown_nodes = widgets::dropdown_select(
        ui,
        body,
        "selection.dropdown",
        &select_options,
        &state.dropdown,
        Some(widgets::AnchoredPopup::new(
            UiRect::new(0.0, 0.0, select_width, 30.0),
            UiRect::new(0.0, 0.0, 320.0, 308.0),
            widgets::PopupPlacement::default(),
        )),
        dropdown_options,
    );
    ui.node_mut(dropdown_nodes.trigger).action = Some("selection.dropdown.toggle".into());
}

fn select_menu_options(width: f32) -> widgets::SelectMenuOptions {
    let mut options = widgets::SelectMenuOptions::default();
    options.width = width;
    options
}

fn text_input(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "text_input", "Text input");
    let mut options = TextInputOptions::default();
    options.placeholder = "Type here".to_string();
    options.layout = LayoutStyle::new().with_width(300.0).with_height(36.0);
    options.text_style = text(13.0, color(230, 236, 246));
    options.placeholder_style = text(13.0, color(144, 156, 174));
    options.edit_action = Some("text.input.edit".into());
    options.focused = state.focused_text == Some(FocusedTextInput::Editable);
    options.caret_visible = caret_visible(state.caret_phase);
    widgets::text_input(ui, body, "text.input", &state.text, options);

    let mut selectable_options = TextInputOptions::default();
    selectable_options.layout = LayoutStyle::new().with_width(360.0).with_height(36.0);
    selectable_options.text_style = text(13.0, color(196, 210, 230));
    selectable_options.read_only = true;
    selectable_options.selectable = true;
    selectable_options.focused = state.focused_text == Some(FocusedTextInput::Selectable);
    selectable_options.edit_action = Some("text.selectable.edit".into());
    selectable_options.caret_visible = caret_visible(state.caret_phase);
    widgets::text_input(
        ui,
        body,
        "text.selectable",
        &state.selectable_text,
        selectable_options,
    );

    let mut singleline = TextInputOptions::default();
    singleline.layout = LayoutStyle::new().with_width(300.0).with_height(34.0);
    singleline.text_style = text(13.0, color(230, 236, 246));
    singleline.placeholder = "Single line".to_string();
    singleline.edit_action = Some("text.singleline.edit".into());
    singleline.focused = state.focused_text == Some(FocusedTextInput::Singleline);
    singleline.caret_visible = caret_visible(state.caret_phase);
    widgets::singleline_text_input(
        ui,
        body,
        "text.singleline",
        &state.singleline_text,
        singleline,
    );

    let mut multiline = TextInputOptions::default();
    multiline.layout = LayoutStyle::new().with_width(360.0).with_height(72.0);
    multiline.text_style = text(13.0, color(230, 236, 246));
    multiline.edit_action = Some("text.multiline.edit".into());
    multiline.focused = state.focused_text == Some(FocusedTextInput::Multiline);
    multiline.caret_visible = caret_visible(state.caret_phase);
    widgets::multiline_text_input(ui, body, "text.multiline", &state.multiline_text, multiline);

    let mut area = TextInputOptions::default();
    area.layout = LayoutStyle::new().with_width(360.0).with_height(66.0);
    area.text_style = text(13.0, color(230, 236, 246));
    area.edit_action = Some("text.area.edit".into());
    area.focused = state.focused_text == Some(FocusedTextInput::TextArea);
    area.caret_visible = caret_visible(state.caret_phase);
    widgets::text_area(ui, body, "text.area", &state.text_area_text, area);

    let mut code = TextInputOptions::default();
    code.layout = LayoutStyle::new().with_width(360.0).with_height(88.0);
    code.edit_action = Some("text.code_editor.edit".into());
    code.focused = state.focused_text == Some(FocusedTextInput::CodeEditor);
    code.caret_visible = caret_visible(state.caret_phase);
    widgets::code_editor(ui, body, "text.code_editor", &state.code_editor_text, code);

    let mut search = TextInputOptions::default();
    search.layout = LayoutStyle::new().with_width(300.0).with_height(34.0);
    search.text_style = text(13.0, color(230, 236, 246));
    search.edit_action = Some("text.search.edit".into());
    search.focused = state.focused_text == Some(FocusedTextInput::Search);
    search.caret_visible = caret_visible(state.caret_phase);
    widgets::search_input(ui, body, "text.search", &state.search_text, search);

    let mut password = TextInputOptions::default();
    password.layout = LayoutStyle::new().with_width(300.0).with_height(34.0);
    password.text_style = text(13.0, color(230, 236, 246));
    password.edit_action = Some("text.password.edit".into());
    password.focused = state.focused_text == Some(FocusedTextInput::Password);
    password.caret_visible = caret_visible(state.caret_phase);
    widgets::password_input(ui, body, "text.password", &state.password_text, password);

    let mut helper = TextInputOptions::default();
    helper.read_only = true;
    helper.selectable = true;
    helper.layout = LayoutStyle::new().with_width(360.0).with_height(36.0);
    helper.text_style = text(13.0, color(196, 210, 230));
    helper.edit_action = Some("text.selectable_helper.edit".into());
    helper.focused = state.focused_text == Some(FocusedTextInput::SelectableHelper);
    helper.caret_visible = caret_visible(state.caret_phase);
    widgets::selectable_text(
        ui,
        body,
        "text.selectable_helper",
        &state.selectable_text,
        helper,
    );
}

fn date_picker(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "date", "Date picker");
    let controls = row(ui, body, "date.options", 8.0);
    choice_button(
        ui,
        controls,
        "date.week.sunday",
        "Sun first",
        state.date.first_weekday == widgets::Weekday::Sunday,
    );
    choice_button(
        ui,
        controls,
        "date.week.monday",
        "Mon first",
        state.date.first_weekday == widgets::Weekday::Monday,
    );
    let mut range_button =
        widgets::ButtonOptions::new(LayoutStyle::new().with_width(92.0).with_height(28.0))
            .with_action("date.range.toggle");
    range_button.visual = if state.date.min.is_some() || state.date.max.is_some() {
        button_visual(48, 112, 184)
    } else {
        button_visual(38, 46, 58)
    };
    range_button.hovered_visual = Some(button_visual(65, 86, 106));
    range_button.text_style = text(12.0, color(238, 244, 252));
    widgets::button(
        ui,
        controls,
        "date.range.toggle",
        "Limit range",
        range_button,
    );
    widgets::date_picker(
        ui,
        body,
        "date.picker",
        &state.date,
        widgets::DatePickerOptions::default().with_action_prefix("date"),
    );
    widgets::label(
        ui,
        body,
        "date.selected",
        format!(
            "Selected: {}",
            state
                .date
                .selected
                .map_or_else(|| "None".to_string(), CalendarDate::iso_string)
        ),
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn color_picker(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "color", "Color picker");
    widgets::color_picker(
        ui,
        body,
        "color.picker",
        &state.color,
        widgets::ColorPickerOptions::default()
            .with_action_prefix("color")
            .with_copy_hex_action("color.copy_hex")
            .with_copy_hex_label("Copy"),
    );
    if let Some(hex) = &state.color_copied_hex {
        widgets::label(
            ui,
            body,
            "color.copied",
            format!("Copied {hex}"),
            text(11.0, color(154, 166, 184)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }
}

fn color_buttons(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "color_buttons", "Color buttons");
    let current_color = state.color.value;

    widgets::label(
        ui,
        body,
        "color_buttons.edit_label",
        "Color edit button",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let edit_row = row(ui, body, "color_buttons.edit_row", 8.0);
    widgets::color_edit_button_rgb(
        ui,
        edit_row,
        "color_buttons.compact",
        current_color,
        color_square_button_options("color_buttons.compact").accessibility_label("Edit RGB color"),
    );
    widgets::label(
        ui,
        edit_row,
        "color_buttons.hex_value",
        widgets::format_hex_color(current_color, false),
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width(92.0),
    );

    widgets::label(
        ui,
        body,
        "color_buttons.format_label",
        "Value formats",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let rgb_row = row(ui, body, "color_buttons.rgb_row", 8.0);
    widgets::label(
        ui,
        rgb_row,
        "color_buttons.rgb_label",
        "RGB",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width(48.0),
    );
    widgets::color_edit_button_rgb(
        ui,
        rgb_row,
        "color_buttons.rgb",
        current_color,
        color_value_button_options("color_buttons.rgb", 180.0),
    );
    let rgba_row = row(ui, body, "color_buttons.rgba_row", 8.0);
    widgets::label(
        ui,
        rgba_row,
        "color_buttons.rgba_label",
        "RGBA",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width(48.0),
    );
    widgets::color_edit_button_rgba(
        ui,
        rgba_row,
        "color_buttons.rgba",
        current_color,
        color_value_button_options("color_buttons.rgba", 230.0),
    );
    let hsva_row = row(ui, body, "color_buttons.hsva_row", 8.0);
    widgets::label(
        ui,
        hsva_row,
        "color_buttons.hsva_label",
        "HSVA",
        text(12.0, color(186, 198, 216)),
        LayoutStyle::new().with_width(48.0),
    );
    widgets::color_edit_button_hsva(
        ui,
        hsva_row,
        "color_buttons.hsva",
        current_color,
        color_value_button_options("color_buttons.hsva", 260.0),
    );

    widgets::label(
        ui,
        body,
        "color_buttons.field_label",
        "2D color field",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::color_picker_hsva_2d(
        ui,
        body,
        "color_buttons.hsva_2d",
        state.color.hsv(),
        widgets::ColorHsva2dOptions::default()
            .with_layout(LayoutStyle::new().with_width(204.0).with_height(112.0))
            .with_action_prefix("color_buttons.hsva_2d"),
    );
    widgets::label(
        ui,
        body,
        "color_buttons.status",
        format!("Last activated: {}", state.color_button_status),
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn menu_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "menus", "Menus");
    let menus = menu_bar_menus(state.menu_autosave, state.menu_grid);
    let active_items = state
        .menu_bar
        .open_menu
        .and_then(|index| menus.get(index))
        .map(|menu| menu.items.clone())
        .unwrap_or_default();
    widgets::menu_bar(
        ui,
        body,
        "menus.menu_bar",
        &menus,
        &state.menu_bar,
        None,
        widgets::MenuBarOptions::default().with_action_prefix("menus.bar"),
    );

    if !active_items.is_empty() {
        widgets::menu_list(
            ui,
            body,
            "menus.menu_list",
            &active_items,
            state.menu_bar.active_item,
            widgets::MenuListOptions::default().with_action_prefix("menus.item"),
        );
        if let Some(active_item) = state.menu_bar.active_item {
            if let Some(children) = active_items
                .get(active_item)
                .and_then(|item| item.children())
            {
                widgets::menu_list_popup(
                    ui,
                    body,
                    "menus.submenu",
                    widgets::AnchoredPopup::new(
                        UiRect::new(
                            0.0,
                            40.0 + menu_item_top_offset(&active_items, active_item),
                            240.0,
                            menu_item_height(active_items.get(active_item)),
                        ),
                        UiRect::new(0.0, 0.0, 680.0, 468.0),
                        widgets::PopupPlacement::new(
                            widgets::PopupSide::Right,
                            widgets::PopupAlign::Start,
                        )
                        .with_offset(4.0),
                    ),
                    children,
                    Some(0),
                    widgets::MenuListOptions::default().with_action_prefix("menus.item"),
                );
            }
        }
    }
    divider(ui, body, "menus.divider.buttons");
    let button_row = row(ui, body, "menus.buttons", 8.0);
    let button_items = menu_items(state.menu_autosave);
    widgets::menu_button(
        ui,
        button_row,
        "menus.menu_button",
        "Menu button",
        &button_items,
        &state.menu_button,
        None,
        widgets::MenuButtonOptions::default().with_action("menus.menu_button"),
    );
    widgets::image_text_menu_button(
        ui,
        button_row,
        "menus.image_text_menu_button",
        "Image text",
        icon_image(BuiltInIcon::Folder),
        &button_items,
        &state.image_text_menu_button,
        None,
        widgets::MenuButtonOptions::default().with_action("menus.image_text_menu_button"),
    );
    widgets::image_menu_button(
        ui,
        button_row,
        "menus.image_menu_button",
        icon_image(BuiltInIcon::Settings),
        &button_items,
        &state.image_menu_button,
        None,
        widgets::MenuButtonOptions::default().with_action("menus.image_menu_button"),
    );
    if state.menu_button.open || state.image_text_menu_button.open || state.image_menu_button.open {
        let active = state
            .menu_button
            .navigation
            .active_path
            .first()
            .copied()
            .or_else(|| {
                state
                    .image_text_menu_button
                    .navigation
                    .active_path
                    .first()
                    .copied()
            })
            .or_else(|| {
                state
                    .image_menu_button
                    .navigation
                    .active_path
                    .first()
                    .copied()
            });
        widgets::menu_list(
            ui,
            body,
            "menus.button_menu",
            &button_items,
            active,
            widgets::MenuListOptions::default().with_action_prefix("menus.item"),
        );
    }

    let context_row = row(ui, body, "menus.context.controls", 8.0);
    button(
        ui,
        context_row,
        "menus.context.open",
        "Open context",
        "menus.context.open",
        button_visual(48, 112, 184),
    );
    button(
        ui,
        context_row,
        "menus.context.close",
        "Close",
        "menus.context.close",
        button_visual(58, 78, 96),
    );
    let mut context_options =
        widgets::MenuListOptions::default().with_action_prefix("menus.context");
    context_options.width = 180.0;
    context_options.max_visible_rows = 4;
    let _ = widgets::context_menu(
        ui,
        parent,
        "menus.context_menu",
        &button_items,
        &state.context_menu,
        UiRect::new(0.0, 0.0, 180.0, 120.0),
        widgets::PopupPlacement::default(),
        context_options,
    );
}

fn command_palette(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "command_palette", "Command palette");
    let items = command_palette_items();
    let mut options =
        widgets::CommandPaletteOptions::default().with_action_prefix("command_palette");
    options.width = 480.0;
    options.row_height = 44.0;
    options.max_visible_rows = 5;
    options.text_style = text(13.0, color(238, 244, 252));
    options.muted_text_style = text(11.0, color(166, 178, 196));
    widgets::command_palette(
        ui,
        body,
        "command_palette.panel",
        &items,
        &state.command_palette,
        None,
        options,
    );
    widgets::label(
        ui,
        body,
        "command_palette.last",
        format!("Last command: {}", state.last_command),
        text(12.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn progress_indicator(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "progress", "Progress indicator");
    let animated = smooth_loop(state.progress_phase * 0.85, 0.0) * 100.0;
    let mut progress = widgets::ProgressIndicatorOptions::default();
    progress.layout = LayoutStyle::new().with_width_percent(1.0).with_height(10.0);
    progress.accessibility_label = Some("Progress".to_string());
    widgets::progress_indicator(
        ui,
        body,
        "progress.primary",
        widgets::ProgressIndicatorValue::percent(animated),
        progress,
    );
    let compact_value = smooth_loop(state.progress_phase * 1.15, 0.7) * 100.0;
    let mut compact = widgets::ProgressIndicatorOptions::default();
    compact.layout = LayoutStyle::new().with_width_percent(1.0).with_height(6.0);
    compact.fill_visual = UiVisual::panel(color(111, 203, 159), None, 3.0);
    widgets::progress_indicator(
        ui,
        body,
        "progress.compact",
        widgets::ProgressIndicatorValue::percent(compact_value),
        compact,
    );
    let warning_value = smooth_loop(state.progress_phase * 0.65, 1.4) * 100.0;
    let mut warning = widgets::ProgressIndicatorOptions::default();
    warning.layout = LayoutStyle::new().with_width_percent(1.0).with_height(14.0);
    warning.fill_visual = UiVisual::panel(color(232, 186, 88), None, 4.0);
    widgets::progress_indicator(
        ui,
        body,
        "progress.warning",
        widgets::ProgressIndicatorValue::percent(warning_value),
        warning,
    );
    let spinner_row = row(ui, body, "progress.spinner.row", 8.0);
    widgets::spinner(
        ui,
        spinner_row,
        "progress.spinner",
        widgets::SpinnerOptions::default()
            .with_phase(state.progress_phase)
            .with_accessibility_label("Loading spinner"),
    );
    widgets::label(
        ui,
        spinner_row,
        "progress.spinner.label",
        "Spinner",
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn list_and_table_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "lists_tables", "Lists and tables");

    let scroll_shell = row(ui, body, "lists_tables.scroll_area.shell", 8.0);
    let nested_scroll = widgets::scroll_area(
        ui,
        scroll_shell,
        "lists_tables.scroll_area",
        ScrollAxes::VERTICAL,
        LayoutStyle::column()
            .with_width(0.0)
            .with_flex_grow(1.0)
            .with_height(92.0),
    );
    ui.node_mut(nested_scroll).action = Some("lists_tables.scroll_area.scroll".into());
    if let Some(scroll) = ui.node_mut(nested_scroll).scroll.as_mut() {
        scroll.offset.y = state.list_scroll;
    }
    for index in 0..6 {
        widgets::label(
            ui,
            nested_scroll,
            format!("lists_tables.scroll_area.row.{index}"),
            format!("Scroll row {}", index + 1),
            text(12.0, color(200, 212, 228)),
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(26.0)
                .with_flex_shrink(0.0),
        );
    }
    widgets::scrollbar(
        ui,
        scroll_shell,
        "lists_tables.scroll_area.scrollbar",
        scroll_state(state.list_scroll, 92.0, 6.0 * 26.0),
        widgets::ScrollAxis::Vertical,
        widgets::ScrollbarOptions::default()
            .with_layout(LayoutStyle::size(8.0, 92.0))
            .with_track_size(UiSize::new(8.0, 92.0))
            .with_action("lists_tables.scroll_area.scrollbar"),
    );

    widgets::table_header(ui, body, "lists_tables.table_header", &table_columns());

    let virtual_shell = row(ui, body, "lists_tables.virtual_list.shell", 8.0);
    let virtual_list = widgets::virtual_list(
        ui,
        virtual_shell,
        "lists_tables.virtual_list",
        widgets::VirtualListSpec {
            row_count: 24,
            row_height: 28.0,
            viewport_height: 112.0,
            scroll_offset: state.virtual_scroll,
            overscan: 1,
        },
        |ui, row_parent, row| {
            widgets::label(
                ui,
                row_parent,
                format!("lists_tables.virtual_list.row.{row}"),
                format!("Virtual row {}", row + 1),
                text(12.0, color(214, 224, 238)),
                LayoutStyle::new()
                    .with_width_percent(1.0)
                    .with_height(28.0)
                    .with_flex_shrink(0.0),
            );
        },
    );
    ui.node_mut(virtual_list).action = Some("lists_tables.virtual_list.scroll".into());
    widgets::scrollbar(
        ui,
        virtual_shell,
        "lists_tables.virtual_list.scrollbar",
        scroll_state(state.virtual_scroll, 112.0, 24.0 * 28.0),
        widgets::ScrollAxis::Vertical,
        widgets::ScrollbarOptions::default()
            .with_layout(LayoutStyle::size(8.0, 112.0))
            .with_track_size(UiSize::new(8.0, 112.0))
            .with_action("lists_tables.virtual_list.scrollbar"),
    );

    let table_shell = row(ui, body, "lists_tables.data_table.shell", 8.0);
    let table_scroll = widgets::scroll_area(
        ui,
        table_shell,
        "lists_tables.data_table",
        ScrollAxes::VERTICAL,
        LayoutStyle::column()
            .with_width(0.0)
            .with_flex_grow(1.0)
            .with_height(128.0),
    );
    ui.node_mut(table_scroll).action = Some("lists_tables.data_table.scroll".into());
    if let Some(scroll) = ui.node_mut(table_scroll).scroll.as_mut() {
        scroll.offset.y = state.table_scroll;
    }
    for row_index in 0..16 {
        data_table_row(ui, table_scroll, row_index, state);
    }
    widgets::scrollbar(
        ui,
        table_shell,
        "lists_tables.data_table.scrollbar",
        scroll_state(state.table_scroll, 128.0, 16.0 * 28.0),
        widgets::ScrollAxis::Vertical,
        widgets::ScrollbarOptions::default()
            .with_layout(LayoutStyle::size(8.0, 128.0))
            .with_track_size(UiSize::new(8.0, 128.0))
            .with_action("lists_tables.data_table.scrollbar"),
    );

    let columns = vec![
        widgets::DataTableColumn::new("name", "Virtualized", 160.0),
        widgets::DataTableColumn::new("status", "Status", 110.0),
        widgets::DataTableColumn::new("value", "Value", 70.0)
            .with_alignment(widgets::DataCellAlignment::End),
    ];
    let mut table_options = widgets::DataTableOptions::default()
        .with_row_action_prefix("lists_tables.virtualized_table.row")
        .with_cell_action_prefix("lists_tables.virtualized_table.cell");
    table_options.selection = state.table_selection.clone();
    widgets::virtualized_data_table(
        ui,
        body,
        "lists_tables.virtualized_table",
        &columns,
        widgets::VirtualDataTableSpec {
            row_count: 32,
            row_height: 28.0,
            viewport_width: 420.0,
            viewport_height: 128.0,
            scroll_offset: UiPoint::new(0.0, state.table_scroll),
            overscan_rows: 1,
        },
        table_options,
        |ui, cell_parent, cell| {
            let value = match cell.column {
                0 => format!("Virtual row {}", cell.row + 1),
                1 if cell.row % 2 == 0 => "Ready".to_string(),
                1 => "Pending".to_string(),
                _ => format!("{}%", 30 + cell.row * 2),
            };
            widgets::label(
                ui,
                cell_parent,
                format!(
                    "lists_tables.virtualized_table.cell.{}.{}.label",
                    cell.row, cell.column
                ),
                value,
                text(12.0, color(220, 228, 238)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
    );
}

fn data_table_row(ui: &mut UiDocument, parent: UiNodeId, row_index: usize, state: &ShowcaseState) {
    let selected = state.table_selection.contains_row(row_index);
    let row = ui.add_child(
        parent,
        UiNode::container(
            format!("lists_tables.data_table.row.{row_index}"),
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height(28.0)
                .with_flex_shrink(0.0),
        )
        .with_input(operad::InputBehavior::BUTTON)
        .with_action(format!("lists_tables.data_table.row.{row_index}"))
        .with_visual(if selected {
            UiVisual::panel(color(45, 73, 109), None, 0.0)
        } else {
            UiVisual::TRANSPARENT
        }),
    );
    let values = [
        format!("Item {}", row_index + 1),
        if row_index % 2 == 0 {
            "Ready".to_string()
        } else {
            "Pending".to_string()
        },
        format!("{}%", 40 + row_index * 3),
    ];
    let widths = [0.42, 0.33, 0.25];
    for (column, value) in values.into_iter().enumerate() {
        let cell = ui.add_child(
            row,
            UiNode::container(
                format!("lists_tables.data_table.cell.{row_index}.{column}"),
                LayoutStyle::new()
                    .with_width_percent(widths[column])
                    .with_height_percent(1.0)
                    .padding(6.0),
            )
            .with_input(operad::InputBehavior::BUTTON)
            .with_action(format!("lists_tables.data_table.cell.{row_index}.{column}")),
        );
        widgets::label(
            ui,
            cell,
            format!("lists_tables.data_table.cell.{row_index}.{column}.label"),
            value,
            text(12.0, color(222, 230, 240)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }
}

fn property_inspector(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "property_inspector", "Property inspector");
    widgets::label(
        ui,
        body,
        "property_inspector.target",
        "Inspecting: Styling preview",
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let mut options = widgets::PropertyInspectorOptions::default();
    options.selected_index = Some(0);
    options.label_width = 120.0;
    options.row_height = 30.0;
    widgets::property_inspector_grid(
        ui,
        body,
        "property_inspector.grid",
        &[
            widgets::PropertyGridRow::new("target", "Widget", "Button preview").read_only(),
            widgets::PropertyGridRow::new(
                "inner",
                "Inner margin",
                format!("{:.0}px", state.styling.inner_margin),
            )
            .with_kind(widgets::PropertyValueKind::Number),
            widgets::PropertyGridRow::new(
                "outer",
                "Outer margin",
                format!("{:.0}px", state.styling.outer_margin),
            )
            .with_kind(widgets::PropertyValueKind::Number),
            widgets::PropertyGridRow::new(
                "radius",
                "Corner radius",
                format!("{:.0}px", state.styling.corner_radius),
            )
            .with_kind(widgets::PropertyValueKind::Number),
            widgets::PropertyGridRow::new(
                "stroke",
                "Stroke",
                format!("{:.1}px", state.styling.stroke_width),
            )
            .with_kind(widgets::PropertyValueKind::Number)
            .changed(),
            widgets::PropertyGridRow::new("state", "Source", "Styling widget").read_only(),
        ],
        options,
    );
}

fn tree_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "trees", "Tree view");
    widgets::tree_view(
        ui,
        body,
        "trees.tree_view",
        &tree_items(),
        &state.tree,
        widgets::TreeViewOptions::default().with_row_action_prefix("trees.tree"),
    );
    widgets::outliner(
        ui,
        body,
        "trees.outliner",
        &tree_items(),
        &state.outliner,
        widgets::TreeViewOptions::default().with_row_action_prefix("trees.outliner"),
    );
}

fn tab_split_dock_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "layout_widgets", "Layout panels");
    let shell = ui.add_child(
        body,
        UiNode::container(
            "layout_widgets.egui_shell",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(340.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(13, 17, 23),
            Some(StrokeStyle::new(color(54, 65, 80), 1.0)),
            0.0,
        )),
    );
    let panels = row(ui, shell, "layout_widgets.egui_panels", 0.0);
    ui.node_mut(panels).style.layout.size.height = operad::length(340.0);
    let inspector = ui.add_child(
        panels,
        UiNode::container(
            "layout_widgets.inspector_panel",
            LayoutStyle::column()
                .with_width(230.0)
                .with_height_percent(1.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(18, 22, 29),
            Some(StrokeStyle::new(color(54, 65, 80), 1.0)),
            0.0,
        )),
    );
    egui_panel_contents(
        ui,
        inspector,
        "layout.inspector",
        "Inspector",
        state.layout_inspector_scroll,
    );

    let assets = ui.add_child(
        panels,
        UiNode::container(
            "layout_widgets.assets_panel",
            LayoutStyle::column()
                .with_width(0.0)
                .with_height_percent(1.0)
                .with_flex_grow(1.0),
        )
        .with_visual(UiVisual::panel(
            color(15, 19, 25),
            Some(StrokeStyle::new(color(54, 65, 80), 1.0)),
            0.0,
        )),
    );
    egui_panel_contents(
        ui,
        assets,
        "layout.assets",
        "Assets",
        state.layout_assets_scroll,
    );
}

fn container_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "containers", "Containers");

    let frame = widgets::frame(
        ui,
        body,
        "containers.frame",
        widgets::FrameOptions::default().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(64.0)
                .with_padding(8.0)
                .with_gap(6.0),
        ),
    );
    widgets::strong_label(
        ui,
        frame,
        "containers.frame.title",
        "Frame",
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::weak_label(
        ui,
        frame,
        "containers.frame.body",
        "Default framed surface with padding, stroke, and clipping.",
        LayoutStyle::new().with_width_percent(1.0),
    );

    let group = widgets::group(ui, body, "containers.group");
    widgets::label(
        ui,
        group,
        "containers.group.label",
        "Group helper",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let generic_panel = widgets::panel(
        ui,
        body,
        "containers.panel",
        widgets::PanelOptions::group().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(44.0)
                .with_padding(8.0),
        ),
    );
    widgets::label(
        ui,
        generic_panel,
        "containers.panel.label",
        "Generic panel",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let group_panel = widgets::group_panel(ui, body, "containers.group_panel");
    widgets::label(
        ui,
        group_panel,
        "containers.group_panel.label",
        "Group panel",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    widgets::separator(
        ui,
        body,
        "containers.separator",
        widgets::SeparatorOptions::default(),
    );
    widgets::spacer(
        ui,
        body,
        "containers.spacer",
        LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height(8.0)
            .with_flex_shrink(0.0),
    );

    let grid = widgets::grid(
        ui,
        body,
        "containers.grid",
        widgets::GridOptions::default().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(78.0)
                .with_gap(4.0),
        ),
    );
    for row_index in 0..2 {
        let row = widgets::grid_row(
            ui,
            grid,
            format!("containers.grid.row.{row_index}"),
            widgets::GridRowOptions::default(),
        );
        for column_index in 0..3 {
            widgets::grid_text_cell(
                ui,
                row,
                format!("containers.grid.row.{row_index}.cell.{column_index}"),
                format!("R{} C{}", row_index + 1, column_index + 1),
                widgets::GridCellOptions {
                    text_style: text(12.0, color(214, 224, 238)),
                    ..Default::default()
                },
            );
        }
    }

    widgets::sides(
        ui,
        body,
        "containers.sides",
        widgets::SidesOptions::default()
            .with_layout(LayoutStyle::row().with_width_percent(1.0).with_height(48.0))
            .with_gap(8.0)
            .with_visual(UiVisual::panel(
                color(20, 25, 32),
                Some(StrokeStyle::new(color(58, 68, 84), 1.0)),
                4.0,
            )),
        |ui, left| {
            widgets::label(
                ui,
                left,
                "containers.sides.left.label",
                "Left side",
                text(12.0, color(220, 228, 238)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
        |ui, right| {
            widgets::label(
                ui,
                right,
                "containers.sides.right.label",
                "Right side",
                text(12.0, color(220, 228, 238)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
    );

    widgets::columns(
        ui,
        body,
        "containers.columns",
        3,
        widgets::ColumnsOptions::default()
            .with_layout(LayoutStyle::row().with_width_percent(1.0).with_height(48.0))
            .with_gap(8.0),
        |ui, column, index| {
            widgets::label(
                ui,
                column,
                format!("containers.columns.{index}.label"),
                format!("Column {}", index + 1),
                text(12.0, color(220, 228, 238)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
    );

    let indented = widgets::indented_section(
        ui,
        body,
        "containers.indented",
        widgets::IndentOptions::default().with_amount(24.0),
    );
    widgets::label(
        ui,
        indented,
        "containers.indented.label",
        "Indented section",
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    widgets::resize_container(
        ui,
        body,
        "containers.resize_container",
        widgets::ResizeContainerOptions::default().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(92.0)
                .with_flex_shrink(0.0),
        ),
        |ui, content| {
            widgets::label(
                ui,
                content,
                "containers.resize_container.label",
                "Resize container",
                text(12.0, color(220, 228, 238)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
    );
    widgets::resize_handle(
        ui,
        body,
        "containers.resize_handle",
        widgets::ResizeHandleOptions::default()
            .with_layout(LayoutStyle::size(20.0, 20.0))
            .accessibility_label("Inline resize handle"),
    );

    widgets::scene(
        ui,
        body,
        "containers.scene",
        vec![
            ScenePrimitive::Rect(
                PaintRect::solid(UiRect::new(8.0, 12.0, 108.0, 46.0), color(48, 112, 184))
                    .stroke(AlignedStroke::inside(StrokeStyle::new(
                        color(132, 174, 222),
                        1.0,
                    )))
                    .corner_radii(CornerRadii::uniform(6.0)),
            ),
            ScenePrimitive::Circle {
                center: UiPoint::new(150.0, 35.0),
                radius: 22.0,
                fill: color(111, 203, 159),
                stroke: Some(StrokeStyle::new(color(176, 236, 206), 1.0)),
            },
            ScenePrimitive::Line {
                from: UiPoint::new(188.0, 18.0),
                to: UiPoint::new(238.0, 52.0),
                stroke: StrokeStyle::new(color(232, 186, 88), 3.0),
            },
        ],
        widgets::SceneOptions::default()
            .with_layout(LayoutStyle::new().with_width(260.0).with_height(70.0))
            .accessibility_label("Scene primitives"),
    );

    let panel_shell = widgets::frame(
        ui,
        body,
        "containers.panels",
        widgets::FrameOptions::default().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height(160.0)
                .with_padding(0.0)
                .with_gap(0.0),
        ),
    );
    let top = widgets::top_panel(ui, panel_shell, "containers.panels.top", 28.0);
    widgets::label(
        ui,
        top,
        "containers.panels.top.label",
        "Top panel",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let middle = row(ui, panel_shell, "containers.panels.middle", 0.0);
    let left = widgets::side_panel(
        ui,
        middle,
        "containers.panels.side",
        widgets::SidePanelSide::Left,
        90.0,
    );
    widgets::label(
        ui,
        left,
        "containers.panels.side.label",
        "Side",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let left = widgets::left_panel(ui, middle, "containers.panels.left", 90.0);
    widgets::label(
        ui,
        left,
        "containers.panels.left.label",
        "Left",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let center = widgets::central_panel(ui, middle, "containers.panels.center");
    widgets::label(
        ui,
        center,
        "containers.panels.center.label",
        "Central panel",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let right = widgets::right_panel(ui, middle, "containers.panels.right", 110.0);
    widgets::label(
        ui,
        right,
        "containers.panels.right.label",
        "Right",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let bottom = widgets::bottom_panel(ui, panel_shell, "containers.panels.bottom", 28.0);
    widgets::label(
        ui,
        bottom,
        "containers.panels.bottom.label",
        "Bottom panel",
        text(12.0, color(220, 228, 238)),
        LayoutStyle::new().with_width_percent(1.0),
    );

    let scroll = widgets::scroll_area_with_bars(
        ui,
        body,
        "containers.scroll_area_with_bars",
        state.containers_scroll,
        widgets::ScrollAreaWithBarsOptions::default()
            .with_axes(ScrollAxes::BOTH)
            .with_vertical_scrollbar(
                widgets::ScrollbarOptions::default()
                    .with_action("containers.scroll_area_with_bars.vertical-scrollbar"),
            )
            .with_horizontal_scrollbar(
                widgets::ScrollbarOptions::default()
                    .with_action("containers.scroll_area_with_bars.horizontal-scrollbar"),
            )
            .with_layout(LayoutStyle::column().with_width(300.0).with_height(116.0)),
    );
    ui.node_mut(scroll.viewport).action = Some("containers.scroll_area_with_bars.scroll".into());
    for index in 0..5 {
        widgets::label(
            ui,
            scroll.viewport,
            format!("containers.scroll_area_with_bars.row.{index}"),
            format!("Scrollable row {}", index + 1),
            text(12.0, color(200, 212, 228)),
            LayoutStyle::new()
                .with_width(420.0)
                .with_height(28.0)
                .with_flex_shrink(0.0),
        );
    }

    let area_host = ui.add_child(
        body,
        UiNode::container(
            "containers.area.host",
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(82.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(17, 20, 25),
            Some(StrokeStyle::new(color(58, 68, 84), 1.0)),
            4.0,
        )),
    );
    widgets::area(
        ui,
        area_host,
        "containers.area",
        widgets::AreaOptions::new(UiRect::new(14.0, 14.0, 180.0, 44.0))
            .with_visual(UiVisual::panel(color(39, 72, 109), None, 4.0))
            .accessibility_label("Absolute positioned area"),
        |ui, area| {
            widgets::label(
                ui,
                area,
                "containers.area.label",
                "Area",
                text(12.0, color(238, 244, 252)),
                LayoutStyle::new().with_width_percent(1.0),
            );
        },
    );
}

fn form_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "forms", "Forms");
    let section = widgets::form_section(
        ui,
        body,
        "forms.profile",
        Some("Profile".to_string()),
        widgets::FormSectionOptions::default().with_layout(
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_padding(12.0)
                .with_gap(10.0),
        ),
    );
    let name = widgets::form_row(
        ui,
        section.root,
        "forms.profile.name",
        widgets::FormRowOptions::default(),
    );
    widgets::field_label(
        ui,
        name,
        "forms.profile.name.label",
        "Name",
        widgets::FieldLabelOptions::default().required(),
    );
    widgets::field_help_text(
        ui,
        name,
        "forms.profile.name.help",
        "Shown in window titles and project lists.",
        widgets::FieldHelpOptions::default(),
    );
    form_text_field(ui, name, "forms.profile.name.input", "Ada Lovelace");

    let email = widgets::form_row(
        ui,
        section.root,
        "forms.profile.email",
        widgets::FormRowOptions::default()
            .required()
            .invalid("Use a complete email address"),
    );
    widgets::field_label(
        ui,
        email,
        "forms.profile.email.label",
        "Email",
        widgets::FieldLabelOptions::default().required(),
    );
    widgets::field_validation_message(
        ui,
        email,
        "forms.profile.email.validation",
        ValidationMessage::error("Use a complete email address"),
        widgets::ValidationMessageOptions::default(),
    );
    form_text_field(ui, email, "forms.profile.email.input", "ada@");

    let role = widgets::form_row(
        ui,
        section.root,
        "forms.profile.role",
        widgets::FormRowOptions::default(),
    );
    widgets::field_label(
        ui,
        role,
        "forms.profile.role.label",
        "Role",
        widgets::FieldLabelOptions::default(),
    );
    widgets::field_help_text(
        ui,
        role,
        "forms.profile.role.help",
        "Form rows compose labels, controls, help, and validation text.",
        widgets::FieldHelpOptions::default(),
    );
    form_text_field(ui, role, "forms.profile.role.input", "Maintainer");

    widgets::form_error_summary(
        ui,
        section.root,
        "forms.profile.errors",
        &state.form,
        widgets::FormErrorSummaryOptions::default(),
    );
    widgets::form_action_buttons(
        ui,
        section.root,
        "forms.profile.actions",
        &state.form,
        widgets::FormActionButtonsOptions::default()
            .include_reset(true)
            .with_action_prefix("forms.profile"),
    );
    widgets::label(
        ui,
        section.root,
        "forms.profile.status",
        format!("Status: {}", state.form_status),
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn overlay_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "overlays", "Overlays");
    let header = widgets::collapsing_header(
        ui,
        body,
        "overlays.collapsing",
        "Collapsing header",
        widgets::CollapsingHeaderOptions::default()
            .expanded(state.overlay_expanded)
            .with_toggle_action("overlays.collapsing.toggle"),
    );
    if let Some(panel) = header.body {
        widgets::label(
            ui,
            panel,
            "overlays.collapsing.body",
            "Expanded content lives under the header.",
            text(12.0, color(196, 210, 230)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }

    let controls = row(ui, body, "overlays.controls", 8.0);
    button(
        ui,
        controls,
        "overlays.popup.toggle",
        if state.overlay_popup_open {
            "Close popup"
        } else {
            "Open popup"
        },
        "overlays.popup.toggle",
        button_visual(48, 112, 184),
    );
    button(
        ui,
        controls,
        "overlays.modal.open",
        "Open modal",
        "overlays.modal.open",
        button_visual(58, 78, 96),
    );

    let tooltip = TooltipContent::new("Tooltip")
        .body("Tooltip boxes are overlay surfaces with title, body, and shortcut text.")
        .shortcut_label("Ctrl+K");
    let mut tooltip_options = widgets::TooltipBoxOptions::default()
        .with_layout(
            LayoutStyle::column()
                .with_width(280.0)
                .with_padding(8.0)
                .with_gap(4.0),
        )
        .with_animation(None);
    tooltip_options.layer = UiLayer::AppContent;
    tooltip_options.z_index = 0;
    widgets::tooltip_box(ui, body, "overlays.tooltip", tooltip, tooltip_options);

    if state.overlay_popup_open {
        let popup = widgets::popup_panel(
            ui,
            parent,
            "overlays.popup_panel",
            UiRect::new(0.0, 20.0, 160.0, 96.0),
            widgets::PopupOptions {
                z_index: 20,
                accessibility: Some(
                    AccessibilityMeta::new(AccessibilityRole::Dialog).label("Popup"),
                ),
                ..Default::default()
            },
        );
        let popup_body = ui.add_child(
            popup,
            UiNode::container(
                "overlays.popup_panel.body",
                LayoutStyle::column()
                    .with_width_percent(1.0)
                    .with_height_percent(1.0)
                    .with_padding(10.0)
                    .with_gap(6.0),
            ),
        );
        let popup_header = row(ui, popup_body, "overlays.popup_panel.header", 8.0);
        widgets::label(
            ui,
            popup_header,
            "overlays.popup_panel.label",
            "Popup panel",
            text(12.0, color(220, 228, 238)),
            LayoutStyle::new().with_width_percent(1.0),
        );
        let mut close = widgets::ButtonOptions::new(LayoutStyle::size(26.0, 22.0))
            .with_action("overlays.popup.close");
        close.visual = UiVisual::panel(color(28, 34, 43), None, 3.0);
        close.hovered_visual = Some(button_visual(54, 70, 92));
        close.text_style = text(12.0, color(220, 228, 238));
        widgets::button(ui, popup_header, "overlays.popup_panel.close", "x", close);
        widgets::label(
            ui,
            popup_body,
            "overlays.popup_panel.body_text",
            "Popup content is conditionally rendered.",
            text(11.0, color(196, 210, 230)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }

    if state.overlay_modal_open {
        let modal = widgets::modal_dialog(
            ui,
            body,
            "overlays.modal",
            "Modal dialog",
            widgets::ModalDialogOptions::default()
                .with_size(320.0, 180.0)
                .with_close_action("overlays.modal.close")
                .with_dismissal(widgets::DialogDismissal::STANDARD)
                .with_focus_restore(FocusRestoreTarget::Previous)
                .modeless(),
        );
        widgets::label(
            ui,
            modal.body,
            "overlays.modal.body.text",
            "Dialog body",
            text(12.0, color(220, 228, 238)),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }
}

fn drag_drop_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "drag_drop", "Drag and drop");
    widgets::dnd_drag_source(
        ui,
        body,
        "drag_drop.text_source",
        "Drag text payload",
        DragPayload::text("Operad payload"),
        widgets::DragSourceOptions::default()
            .with_kind(DragDropSurfaceKind::ListRow)
            .with_action("drag_drop.text_source")
            .with_accessibility_hint("Start a text drag operation"),
    );

    let accepted_options = widgets::DropZoneOptions::default()
        .with_kind(DragDropSurfaceKind::EditorSurface)
        .with_accepted_payload(DropPayloadFilter::empty().text())
        .with_action("drag_drop.accept_text")
        .with_accessibility_hint("Accepts text payloads");
    let accepted = widgets::dnd_drop_zone(
        ui,
        body,
        "drag_drop.accept_text",
        "Drop text here",
        accepted_options.clone(),
    );
    widgets::dnd_apply_drop_zone_preview(
        ui,
        accepted.root,
        &accepted_options,
        widgets::DropZonePreviewState::Accepted,
    );

    let rejected_options = widgets::DropZoneOptions::default()
        .with_layout(
            LayoutStyle::column()
                .with_width(240.0)
                .with_height(82.0)
                .with_padding(12.0),
        )
        .with_kind(DragDropSurfaceKind::Asset)
        .with_accepted_payload(DropPayloadFilter::empty().files())
        .with_action("drag_drop.files_only");
    let rejected = widgets::dnd_drop_zone(
        ui,
        body,
        "drag_drop.files_only",
        "Files only",
        rejected_options.clone(),
    );
    widgets::dnd_apply_drop_zone_preview(
        ui,
        rejected.root,
        &rejected_options,
        widgets::DropZonePreviewState::Rejected,
    );
    widgets::label(
        ui,
        body,
        "drag_drop.status",
        format!("Status: {}", state.drag_drop_status),
        text(11.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn media_widgets(ui: &mut UiDocument, parent: UiNodeId) {
    let body = section(ui, parent, "media", "Media");
    let icons = row(ui, body, "media.icons", 10.0);
    widgets::image(
        ui,
        icons,
        "media.image.play",
        icon_image(BuiltInIcon::Play),
        widgets::ImageOptions::default()
            .with_layout(LayoutStyle::size(42.0, 42.0))
            .with_accessibility_label("Play icon"),
    );
    widgets::image(
        ui,
        icons,
        "media.image.warning",
        ImageContent::new(BuiltInIcon::Warning.key()).tinted(color(232, 186, 88)),
        widgets::ImageOptions::default()
            .with_layout(LayoutStyle::size(42.0, 42.0))
            .with_accessibility_label("Warning icon"),
    );
    widgets::image(
        ui,
        icons,
        "media.image.info",
        ImageContent::new(BuiltInIcon::Info.key()).tinted(color(118, 183, 255)),
        widgets::ImageOptions::default()
            .with_layout(LayoutStyle::size(42.0, 42.0))
            .with_accessibility_label("Info icon"),
    );
    widgets::label(
        ui,
        body,
        "media.image.note",
        "Image widgets reference stable resource keys; the host resolves them to textures or vector assets.",
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn timeline_ruler(ui: &mut UiDocument, parent: UiNodeId) {
    let mut layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_height(40.0)
        .with_flex_shrink(0.0);
    layout.as_taffy_style_mut().min_size.width = operad::length(0.0);
    layout.as_taffy_style_mut().min_size.height = operad::length(0.0);
    let body = widgets::scroll_area(ui, parent, "timeline", ScrollAxes::BOTH, layout);
    widgets::timeline_ruler(
        ui,
        body,
        "timeline.ruler",
        widgets::RulerSpec {
            range: widgets::TimelineRange::new(0.0, 12.0),
            width: 600.0,
            major_step: 2.0,
            minor_step: 0.5,
            label_every: 1,
        },
        widgets::TimelineRulerOptions::default(),
    );
}

fn toast_controls(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "toasts", "Toasts");
    let controls = row(ui, body, "toasts.controls", 10.0);
    button(
        ui,
        controls,
        "toasts.show",
        "Show toast",
        "toast.show",
        button_visual(48, 112, 184),
    );
    button(
        ui,
        controls,
        "toasts.hide",
        "Hide",
        "toast.hide",
        button_visual(58, 78, 96),
    );
    widgets::label(
        ui,
        body,
        "toasts.status",
        if state.toast_visible {
            "Toast overlay is visible."
        } else {
            "Toast overlay is hidden."
        },
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    widgets::label(
        ui,
        body,
        "toasts.action_status",
        format!("Action: {}", state.toast_action_status),
        text(12.0, color(154, 166, 184)),
        LayoutStyle::new().with_width_percent(1.0),
    );
}

fn popup_controls(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "popup_panel", "Popup panel");
    let controls = row(ui, body, "popup_panel.controls", 8.0);
    button(
        ui,
        controls,
        "popup_panel.toggle",
        if state.popup_open {
            "Close popup"
        } else {
            "Open popup"
        },
        "popup.toggle",
        button_visual(48, 112, 184),
    );
    if state.popup_open {
        let mut close =
            widgets::ButtonOptions::new(LayoutStyle::size(30.0, 30.0)).with_action("popup.close");
        close.visual = UiVisual::panel(color(28, 34, 43), None, 3.0);
        close.hovered_visual = Some(button_visual(54, 70, 92));
        close.text_style = text(13.0, color(220, 228, 238));
        widgets::button(ui, controls, "popup_panel.inline_close", "x", close);
    }
    widgets::label(
        ui,
        body,
        "popup_panel.status",
        if state.popup_open {
            "Popup overlay is open."
        } else {
            "Popup overlay is closed."
        },
        text(12.0, color(196, 210, 230)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    if state.popup_open {
        let panel = widgets::popup_panel(
            ui,
            parent,
            "popup_panel.inline_preview",
            UiRect::new(0.0, 20.0, 160.0, 104.0),
            widgets::PopupOptions {
                z_index: 4,
                accessibility: Some(
                    AccessibilityMeta::new(AccessibilityRole::Dialog).label("Popup preview"),
                ),
                ..Default::default()
            },
        );
        let content = ui.add_child(
            panel,
            UiNode::container(
                "popup_panel.inline_preview.body",
                LayoutStyle::column()
                    .with_width_percent(1.0)
                    .with_height_percent(1.0)
                    .with_padding(10.0)
                    .with_gap(8.0),
            ),
        );
        let header = row(ui, content, "popup_panel.inline_preview.header", 8.0);
        widgets::label(
            ui,
            header,
            "popup_panel.inline_preview.title",
            "Popup panel",
            text(12.0, color(226, 234, 246)),
            LayoutStyle::new().with_width_percent(1.0),
        );
        let mut close =
            widgets::ButtonOptions::new(LayoutStyle::size(26.0, 22.0)).with_action("popup.close");
        close.visual = UiVisual::panel(color(28, 34, 43), None, 3.0);
        close.hovered_visual = Some(button_visual(54, 70, 92));
        close.text_style = text(12.0, color(220, 228, 238));
        widgets::button(ui, header, "popup_panel.inline_preview.close", "x", close);
        widgets::label(
            ui,
            content,
            "popup_panel.inline_preview.text",
            "Overlay content",
            text(11.0, color(196, 210, 230)),
            LayoutStyle::new().with_width_percent(1.0),
        );
        widgets::spacer(
            ui,
            body,
            "popup_panel.inline_preview.space",
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(112.0)
                .with_flex_shrink(0.0),
        );
    }
}

fn styling_widgets(ui: &mut UiDocument, parent: UiNodeId, state: &ShowcaseState) {
    let body = section(ui, parent, "styling", "Styling");
    let grid = ui.add_child(
        body,
        UiNode::container(
            "styling.grid",
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height_percent(1.0)
                .gap(16.0),
        ),
    );
    let controls = ui.add_child(
        grid,
        UiNode::container(
            "styling.controls",
            LayoutStyle::column()
                .with_width(330.0)
                .with_height_percent(1.0)
                .with_flex_shrink(0.0)
                .gap(6.0),
        ),
    );
    style_checkbox(
        ui,
        controls,
        "styling.inner_same",
        "Inner margin same",
        state.styling.inner_same,
    );
    style_slider(
        ui,
        controls,
        "styling.inner",
        "Inner left",
        state.styling.inner_margin,
        0.0..32.0,
    );
    if !state.styling.inner_same {
        style_slider(
            ui,
            controls,
            "styling.inner_right",
            "Inner right",
            state.styling.inner_right,
            0.0..32.0,
        );
        style_slider(
            ui,
            controls,
            "styling.inner_top",
            "Inner top",
            state.styling.inner_top,
            0.0..32.0,
        );
        style_slider(
            ui,
            controls,
            "styling.inner_bottom",
            "Inner bottom",
            state.styling.inner_bottom,
            0.0..32.0,
        );
    }
    style_checkbox(
        ui,
        controls,
        "styling.outer_same",
        "Outer margin same",
        state.styling.outer_same,
    );
    style_slider(
        ui,
        controls,
        "styling.outer",
        "Outer left",
        state.styling.outer_margin,
        0.0..40.0,
    );
    if !state.styling.outer_same {
        style_slider(
            ui,
            controls,
            "styling.outer_right",
            "Outer right",
            state.styling.outer_right,
            0.0..40.0,
        );
        style_slider(
            ui,
            controls,
            "styling.outer_top",
            "Outer top",
            state.styling.outer_top,
            0.0..40.0,
        );
        style_slider(
            ui,
            controls,
            "styling.outer_bottom",
            "Outer bottom",
            state.styling.outer_bottom,
            0.0..40.0,
        );
    }
    style_checkbox(
        ui,
        controls,
        "styling.radius_same",
        "Corner radius same",
        state.styling.radius_same,
    );
    style_slider(
        ui,
        controls,
        "styling.radius",
        "Radius NW",
        state.styling.corner_radius,
        0.0..28.0,
    );
    if !state.styling.radius_same {
        style_slider(
            ui,
            controls,
            "styling.radius_ne",
            "Radius NE",
            state.styling.corner_ne,
            0.0..28.0,
        );
        style_slider(
            ui,
            controls,
            "styling.radius_sw",
            "Radius SW",
            state.styling.corner_sw,
            0.0..28.0,
        );
        style_slider(
            ui,
            controls,
            "styling.radius_se",
            "Radius SE",
            state.styling.corner_se,
            0.0..28.0,
        );
    }
    style_slider(
        ui,
        controls,
        "styling.shadow_x",
        "Shadow x",
        state.styling.shadow_x,
        -24.0..24.0,
    );
    style_slider(
        ui,
        controls,
        "styling.shadow_y",
        "Shadow y",
        state.styling.shadow_y,
        -24.0..24.0,
    );
    style_slider(
        ui,
        controls,
        "styling.shadow",
        "Shadow blur",
        state.styling.shadow_blur,
        0.0..32.0,
    );
    style_slider(
        ui,
        controls,
        "styling.shadow_spread",
        "Shadow spread",
        state.styling.shadow_spread,
        0.0..16.0,
    );
    style_slider(
        ui,
        controls,
        "styling.shadow_alpha",
        "Shadow color",
        state.styling.shadow_alpha,
        0.0..220.0,
    );
    style_slider(
        ui,
        controls,
        "styling.fill",
        "Fill color",
        state.styling.fill_tint,
        0.0..1.0,
    );
    style_slider(
        ui,
        controls,
        "styling.stroke_color",
        "Stroke color",
        state.styling.stroke_tint,
        0.0..1.0,
    );
    style_slider(
        ui,
        controls,
        "styling.stroke",
        "Stroke",
        state.styling.stroke_width,
        0.0..4.0,
    );

    let preview = ui.add_child(
        grid,
        UiNode::container(
            "styling.preview",
            LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0)
                .padding(8.0),
        )
        .with_visual(UiVisual::panel(
            color(17, 20, 25),
            Some(StrokeStyle::new(color(56, 66, 82), 1.0)),
            4.0,
        )),
    );
    style_preview(ui, preview, state.styling);
}

fn style_slider(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    value: f32,
    range: std::ops::Range<f32>,
) {
    let row = row(ui, parent, format!("{name}.row"), 8.0);
    widgets::label(
        ui,
        row,
        format!("{name}.label"),
        label,
        text(12.0, color(166, 176, 190)),
        LayoutStyle::new().with_width(118.0),
    );
    widgets::label(
        ui,
        row,
        format!("{name}.value"),
        if range.end <= 1.0 {
            format!("{value:.2}")
        } else {
            format!("{value:.0}")
        },
        text(12.0, color(226, 232, 242)),
        LayoutStyle::new().with_width(42.0),
    );
    let mut options = widgets::SliderOptions::default()
        .with_layout(
            LayoutStyle::new()
                .with_width(112.0)
                .with_height(20.0)
                .with_flex_shrink(0.0),
        )
        .with_value_edit_action(name);
    options.fill_color = color(120, 170, 230);
    widgets::slider(
        ui,
        row,
        format!("{name}.slider"),
        ((value - range.start) / (range.end - range.start).max(f32::EPSILON)).clamp(0.0, 1.0),
        0.0..1.0,
        options,
    );
}

fn style_checkbox(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    checked: bool,
) {
    let mut options = widgets::CheckboxOptions::default().with_action(name);
    options.layout = LayoutStyle::new().with_width_percent(1.0).with_height(22.0);
    options.text_style = text(12.0, color(220, 228, 238));
    widgets::checkbox(ui, parent, name, label, checked, options);
}

fn style_preview(ui: &mut UiDocument, parent: UiNodeId, styling: StylingState) {
    let outer = styling.outer_edges();
    let inner = styling.inner_edges();
    let frame = UiRect::new(
        22.0 + outer[0],
        28.0 + outer[2],
        108.0 + inner[0] + inner[1],
        40.0 + inner[2] + inner[3],
    );
    let text_rect = UiRect::new(
        frame.x + inner[0],
        frame.y + inner[2],
        (frame.width - inner[0] - inner[1]).max(1.0),
        (frame.height - inner[2] - inner[3]).max(1.0),
    );
    ui.add_child(
        parent,
        UiNode::scene(
            "styling.preview.scene",
            vec![
                ScenePrimitive::Rect(
                    PaintRect::solid(frame, styling.fill_color())
                        .stroke(AlignedStroke::inside(StrokeStyle::new(
                            styling.stroke_color(),
                            styling.stroke_width,
                        )))
                        .corner_radii(styling.radii())
                        .effect(PaintEffect::shadow(
                            styling.shadow_color(),
                            UiPoint::new(styling.shadow_x, styling.shadow_y),
                            styling.shadow_blur,
                            styling.shadow_spread,
                        )),
                ),
                ScenePrimitive::Text(
                    PaintText::new("Content", text_rect, text(13.0, color(255, 255, 255)))
                        .horizontal_align(TextHorizontalAlign::Center)
                        .vertical_align(TextVerticalAlign::Center)
                        .multiline(false),
                ),
            ],
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(180.0)
                .with_flex_shrink(0.0),
        ),
    );
}

fn slider_options(state: &ShowcaseState, width: f32) -> widgets::SliderOptions {
    let mut options = widgets::SliderOptions::default().with_layout(
        LayoutStyle::new()
            .with_width(width)
            .with_height(24.0)
            .with_flex_shrink(0.0),
    );
    options.fill_color = if state.slider_trailing_color {
        state.slider_trailing_picker.value
    } else {
        color(42, 49, 58)
    };
    options.thumb_shape = match state.slider_thumb_shape {
        SliderThumbChoice::Circle => widgets::SliderThumbShape::Circle,
        SliderThumbChoice::Square => widgets::SliderThumbShape::Square,
        SliderThumbChoice::Rectangle => widgets::SliderThumbShape::Rectangle,
    };
    options
}

fn slider_number_input(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    input: &TextInputState,
    focused: FocusedTextInput,
    state: &ShowcaseState,
    width: f32,
) {
    let mut options = TextInputOptions::default();
    options.layout = LayoutStyle::new().with_width(width).with_height(28.0);
    options.text_style = text(12.0, color(230, 236, 246));
    options.placeholder_style = text(12.0, color(144, 156, 174));
    options.edit_action = Some(format!("{name}.edit").into());
    options.focused = state.focused_text == Some(focused);
    options.caret_visible = caret_visible(state.caret_phase);
    widgets::text_input(ui, parent, name, input, options);
}

fn form_text_field(ui: &mut UiDocument, parent: UiNodeId, name: &'static str, value: &'static str) {
    let mut options = TextInputOptions::default();
    options.layout = LayoutStyle::new().with_width_percent(1.0).with_height(30.0);
    options.text_style = text(12.0, color(230, 236, 246));
    options.read_only = true;
    widgets::text_input(ui, parent, name, &TextInputState::new(value), options);
}

fn slider_checkbox(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    checked: bool,
) {
    slider_checkbox_with_layout(
        ui,
        parent,
        name,
        label,
        checked,
        LayoutStyle::new().with_width_percent(1.0).with_height(30.0),
    );
}

fn slider_checkbox_with_layout(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    checked: bool,
    layout: LayoutStyle,
) {
    let mut options = widgets::CheckboxOptions::default().with_action(name);
    options.layout = layout;
    options.text_style = text(12.0, color(220, 228, 238));
    widgets::checkbox(ui, parent, name, label, checked, options);
}

fn choice_button(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    label: &'static str,
    selected: bool,
) {
    let mut options =
        widgets::ButtonOptions::new(LayoutStyle::new().with_width(78.0).with_height(28.0))
            .with_action(name);
    options.visual = if selected {
        button_visual(48, 112, 184)
    } else {
        button_visual(38, 46, 58)
    };
    options.hovered_visual = Some(button_visual(65, 86, 106));
    options.pressed_visual = Some(button_visual(34, 54, 84));
    options.text_style = text(12.0, color(238, 244, 252));
    widgets::button(ui, parent, name, label, options);
}

fn divider(ui: &mut UiDocument, parent: UiNodeId, name: &'static str) {
    ui.add_child(
        parent,
        UiNode::container(
            name,
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(1.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(color(48, 58, 72), None, 0.0)),
    );
}

fn canvas(ui: &mut UiDocument, parent: UiNodeId) {
    let body = section(ui, parent, "canvas", "Canvas");
    let mut options = widgets::CanvasOptions::default()
        .with_accessibility_label("Shader canvas")
        .with_action("canvas.rotate")
        .with_aspect_ratio(16.0 / 9.0);
    options.layout = LayoutStyle::new()
        .with_width_percent(1.0)
        .with_height_percent(1.0)
        .with_flex_grow(1.0)
        .with_flex_shrink(1.0);
    options.visual = UiVisual::panel(
        color(18, 22, 28),
        Some(StrokeStyle::new(color(58, 68, 84), 1.0)),
        4.0,
    );
    widgets::canvas(
        ui,
        body,
        "canvas.shader",
        CanvasContent::new("canvas.shader").gpu_context(),
        options,
    );
}

fn render_showcase_canvas(
    state: &mut ShowcaseState,
    context: NativeWgpuCanvasRenderContext<'_>,
) -> Result<CanvasRenderOutput, RenderError> {
    let size = context.surface_size();
    if state.cube.needs_render(size) {
        render_showcase_canvas_surface(state.cube, &context.surface)?;
        state.cube.mark_rendered(size);
    }
    Ok(CanvasRenderOutput::new())
}

fn render_showcase_canvas_surface(
    cube: CanvasCubeState,
    surface: &WgpuCanvasContext<'_>,
) -> Result<(), RenderError> {
    let uniforms = canvas_cube_uniform_bytes(cube);
    surface.render_pass(
        WgpuCanvasRenderPass::wgsl(include_str!("shaders/showcase_canvas.wgsl"))
            .label(Some("showcase.canvas"))
            .uniform_bytes(&uniforms[..])
            .clear_color(Some(color(18, 22, 28))),
    )
}

fn canvas_cube_uniform_bytes(cube: CanvasCubeState) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes[0..4].copy_from_slice(&cube.yaw.to_ne_bytes());
    bytes[4..8].copy_from_slice(&cube.pitch.to_ne_bytes());
    bytes
}

fn section(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    _title: impl Into<String>,
) -> UiNodeId {
    let name = name.into();
    let mut layout = LayoutStyle::column()
        .with_width_percent(1.0)
        .with_height_percent(1.0)
        .with_flex_grow(1.0)
        .gap(10.0);
    layout.as_taffy_style_mut().min_size.width = operad::length(0.0);
    layout.as_taffy_style_mut().min_size.height = operad::length(0.0);
    widgets::scroll_area(
        ui,
        parent,
        format!("{name}.section_scroll"),
        ScrollAxes::VERTICAL,
        layout,
    )
}

fn row(ui: &mut UiDocument, parent: UiNodeId, name: impl Into<String>, gap: f32) -> UiNodeId {
    ui.add_child(
        parent,
        UiNode::container(name, LayoutStyle::row().with_width_percent(1.0).gap(gap)),
    )
}

fn wrapping_row(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    gap: f32,
) -> UiNodeId {
    let mut layout = LayoutStyle::row().with_width_percent(1.0).gap(gap);
    layout.as_taffy_style_mut().flex_wrap = LayoutFlexWrap::Wrap.to_taffy();
    ui.add_child(parent, UiNode::container(name, layout))
}

fn egui_panel_contents(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: &'static str,
    title: &'static str,
    offset_y: f32,
) {
    let header = ui.add_child(
        parent,
        UiNode::container(
            format!("{name}.egui_header"),
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height(28.0)
                .with_padding(6.0)
                .with_flex_shrink(0.0),
        )
        .with_visual(UiVisual::panel(
            color(21, 26, 34),
            Some(StrokeStyle::new(color(54, 65, 80), 1.0)),
            0.0,
        )),
    );
    widgets::label(
        ui,
        header,
        format!("{name}.egui_title"),
        title,
        text(12.0, color(226, 234, 246)),
        LayoutStyle::new().with_width_percent(1.0),
    );
    let scroll = widgets::scroll_area(
        ui,
        parent,
        format!("{name}.scroll_area"),
        ScrollAxes::VERTICAL,
        LayoutStyle::column()
            .with_width_percent(1.0)
            .with_height(0.0)
            .with_flex_grow(1.0)
            .with_padding(8.0)
            .with_gap(6.0),
    );
    ui.node_mut(scroll).action = Some(format!("{name}.scroll").into());
    if let Some(scroll_state) = ui.node_mut(scroll).scroll.as_mut() {
        scroll_state.offset.y = offset_y;
    }
    for (index, line) in lorem_lines().iter().take(8).enumerate() {
        widgets::label(
            ui,
            scroll,
            format!("{name}.egui_line.{index}"),
            *line,
            TextStyle {
                wrap: TextWrap::None,
                ..text(11.0, color(190, 202, 218))
            },
            LayoutStyle::new()
                .with_width_percent(1.0)
                .with_height(22.0)
                .with_flex_shrink(0.0),
        );
    }
}

fn button(
    ui: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    action: impl Into<String>,
    visual: UiVisual,
) -> UiNodeId {
    let mut options = widgets::ButtonOptions::new(LayoutStyle::new().with_height(32.0))
        .with_action(action.into());
    options.visual = visual;
    options.hovered_visual = Some(adjusted_button_visual(visual, 58));
    options.pressed_visual = Some(adjusted_button_visual(visual, -62));
    options.pressed_hovered_visual = Some(adjusted_button_visual(visual, 8));
    options.text_style = text(13.0, color(246, 249, 252));
    widgets::button(ui, parent, name, label, options)
}

fn button_visual(r: u8, g: u8, b: u8) -> UiVisual {
    UiVisual::panel(
        color(r, g, b),
        Some(StrokeStyle::new(color(86, 102, 124), 1.0)),
        4.0,
    )
}

fn color_square_button_options(action: &'static str) -> widgets::ColorButtonOptions {
    widgets::ColorButtonOptions::default()
        .with_layout(LayoutStyle::size(30.0, 30.0).with_flex_shrink(0.0))
        .with_swatch_size(UiSize::new(30.0, 30.0))
        .with_action(action)
        .show_label(false)
}

fn color_value_button_options(action: &'static str, width: f32) -> widgets::ColorButtonOptions {
    widgets::ColorButtonOptions::default()
        .with_layout(
            LayoutStyle::new()
                .with_width(width)
                .with_height(30.0)
                .with_flex_shrink(0.0),
        )
        .with_action(action)
}

fn icon_image(icon: BuiltInIcon) -> ImageContent {
    ImageContent::new(icon.key()).tinted(color(220, 228, 238))
}

fn adjusted_button_visual(visual: UiVisual, delta: i16) -> UiVisual {
    UiVisual::panel(
        adjust_color(visual.fill, delta),
        visual.stroke.map(|stroke| StrokeStyle {
            color: adjust_color(stroke.color, delta / 2),
            width: stroke.width,
        }),
        visual.corner_radius,
    )
}

fn adjust_color(color: ColorRgba, delta: i16) -> ColorRgba {
    let channel = |value: u8| -> u8 { (i16::from(value) + delta).clamp(0, u8::MAX as i16) as u8 };
    ColorRgba::new(
        channel(color.r),
        channel(color.g),
        channel(color.b),
        color.a,
    )
}

fn select_options() -> Vec<widgets::SelectOption> {
    vec![
        widgets::SelectOption::new("compact", "Compact"),
        widgets::SelectOption::new("comfortable", "Comfortable"),
        widgets::SelectOption::new("spacious", "Spacious"),
        widgets::SelectOption::new("disabled", "Disabled").disabled(),
    ]
}

fn label_locale_options() -> Vec<widgets::SelectOption> {
    vec![
        widgets::SelectOption::new("en-US", "English"),
        widgets::SelectOption::new("es-MX", "Español"),
        widgets::SelectOption::new("fr-FR", "Français"),
        widgets::SelectOption::new("de-DE", "Deutsch"),
        widgets::SelectOption::new("it-IT", "Italiano"),
        widgets::SelectOption::new("pt-BR", "Português"),
        widgets::SelectOption::new("nl-NL", "Nederlands"),
    ]
}

fn localized_label(locale_id: &str) -> &'static str {
    match locale_id {
        "en-US" => "Interface language: English",
        "fr-FR" => "Langue de l'interface : français",
        "de-DE" => "Sprache der Oberfläche: Deutsch",
        "it-IT" => "Lingua dell'interfaccia: italiano",
        "pt-BR" => "Idioma da interface: português",
        "nl-NL" => "Interfacetaal: Nederlands",
        _ => "Idioma de interfaz: español de México",
    }
}

fn lorem_lines() -> [&'static str; 8] {
    [
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
        "Integer vitae arcu at neque feugiat posuere.",
        "Suspendisse potenti. Praesent eget sem non mauris luctus.",
        "Curabitur blandit, justo non gravida tristique, mi nunc.",
        "Donec at nibh vel sapien facilisis feugiat.",
        "Aliquam erat volutpat. Nam porttitor sem at ligula.",
        "Vivamus dictum eros vitae tortor aliquet, in tempor urna.",
        "Sed finibus velit non lectus efficitur, sed tempor orci.",
    ]
}

fn menu_bar_menus(autosave: bool, grid: bool) -> Vec<widgets::MenuBarMenu> {
    vec![
        widgets::MenuBarMenu::new("file", "File", menu_items(autosave)),
        widgets::MenuBarMenu::new(
            "edit",
            "Edit",
            vec![
                widgets::MenuItem::command("undo", "Undo").shortcut("Ctrl+Z"),
                widgets::MenuItem::command("redo", "Redo").shortcut("Ctrl+Shift+Z"),
            ],
        ),
        widgets::MenuBarMenu::new(
            "view",
            "View",
            vec![widgets::MenuItem::check("grid", "Grid", grid)],
        ),
    ]
}

fn menu_items(autosave: bool) -> Vec<widgets::MenuItem> {
    vec![
        widgets::MenuItem::command("new", "New").shortcut("Ctrl+N"),
        widgets::MenuItem::command("open", "Open").shortcut("Ctrl+O"),
        widgets::MenuItem::separator(),
        widgets::MenuItem::check("autosave", "Autosave", autosave),
        widgets::MenuItem::submenu(
            "recent",
            "Recent",
            vec![
                widgets::MenuItem::command("recent.one", "demo.rs"),
                widgets::MenuItem::command("recent.two", "notes.md"),
            ],
        ),
        widgets::MenuItem::command("delete", "Delete").destructive(),
        widgets::MenuItem::command("disabled", "Disabled").disabled(),
    ]
}

fn menu_item_top_offset(items: &[widgets::MenuItem], index: usize) -> f32 {
    items
        .iter()
        .take(index)
        .map(|item| menu_item_height(Some(item)))
        .sum()
}

fn menu_item_height(item: Option<&widgets::MenuItem>) -> f32 {
    if item.is_some_and(widgets::MenuItem::is_separator) {
        8.0
    } else {
        28.0
    }
}

fn command_palette_items() -> Vec<widgets::CommandPaletteItem> {
    vec![
        widgets::CommandPaletteItem::new("open", "Open")
            .subtitle("Open a document")
            .shortcut("Ctrl+O")
            .keyword("file"),
        widgets::CommandPaletteItem::new("save", "Save")
            .subtitle("Write current changes")
            .shortcut("Ctrl+S"),
        widgets::CommandPaletteItem::new("format", "Format document")
            .subtitle("Apply source formatting")
            .keyword("code"),
        widgets::CommandPaletteItem::new("rename", "Rename symbol")
            .subtitle("Change every reference")
            .shortcut("F2"),
        widgets::CommandPaletteItem::new("toggle_sidebar", "Toggle sidebar")
            .subtitle("Show or hide the widget panel")
            .shortcut("Ctrl+B"),
        widgets::CommandPaletteItem::new("run", "Run current example")
            .subtitle("Launch showcase")
            .shortcut("Ctrl+R"),
        widgets::CommandPaletteItem::new("focus_canvas", "Focus canvas")
            .subtitle("Move interaction to the canvas window"),
        widgets::CommandPaletteItem::new("reset_layout", "Reset window layout")
            .subtitle("Restore the default showcase positions"),
        widgets::CommandPaletteItem::new("disabled", "Disabled command").disabled(),
    ]
}

fn table_columns() -> Vec<widgets::TableColumn> {
    vec![
        widgets::TableColumn {
            id: "name".to_string(),
            label: "Name".to_string(),
            width: 160.0,
        },
        widgets::TableColumn {
            id: "status".to_string(),
            label: "Status".to_string(),
            width: 140.0,
        },
        widgets::TableColumn {
            id: "value".to_string(),
            label: "Value".to_string(),
            width: 100.0,
        },
    ]
}

fn tree_items() -> Vec<widgets::TreeItem> {
    vec![
        widgets::TreeItem::new("root", "Project").with_children(vec![
            widgets::TreeItem::new("src", "src").with_children(vec![
                widgets::TreeItem::new("lib", "lib.rs"),
                widgets::TreeItem::new("widgets", "widgets.rs"),
            ]),
            widgets::TreeItem::new("assets", "assets").with_children(vec![
                widgets::TreeItem::new("shader", "shader.wgsl"),
                widgets::TreeItem::new("logo", "logo.png"),
            ]),
            widgets::TreeItem::new("target", "target").disabled(),
        ]),
    ]
}

fn parse_calendar_date(value: &str) -> Option<CalendarDate> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    CalendarDate::new(year, month, day)
}

fn parse_table_cell(value: &str) -> Option<widgets::DataTableCellIndex> {
    let mut parts = value.split('.');
    let row = parts.next()?.parse().ok()?;
    let column = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(widgets::DataTableCellIndex::new(row, column))
}

fn unit(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn smooth_loop(phase: f32, offset: f32) -> f32 {
    0.5 - ((phase + offset).cos() * 0.5)
}

fn create_system_clipboard() -> Option<arboard::Clipboard> {
    arboard::Clipboard::new().ok()
}

fn profile_form_state() -> FormState {
    let mut form = FormState::new("profile")
        .with_field("name", "Operad")
        .with_field("email", "invalid@example")
        .with_field("role", "Designer");
    form.update_field("email", "invalid@example").unwrap();
    let request = form.begin_form_validation();
    let _ = form.apply_form_validation(
        FormValidationResult::new(request.generation)
            .with_field_messages(
                "email",
                vec![ValidationMessage::error("Use a complete email address")],
            )
            .with_form_message(ValidationMessage::warning("Unsaved profile changes")),
    );
    form
}

fn scaled_slider(rect: UiRect, point: UiPoint, min: f32, max: f32) -> f32 {
    min + unit(widgets::slider_value_from_control_point(
        rect,
        point,
        0.0..1.0,
    )) * (max - min)
}

fn scroll_state(offset_y: f32, viewport_height: f32, content_height: f32) -> operad::ScrollState {
    operad::ScrollState {
        axes: ScrollAxes::VERTICAL,
        offset: UiPoint::new(0.0, offset_y),
        viewport_size: UiSize::new(8.0, viewport_height),
        content_size: UiSize::new(8.0, content_height),
    }
}

fn controls_list_viewport_height(viewport_height: f32) -> f32 {
    (viewport_height - 110.0).max(120.0)
}

fn controls_scroll_state(offset_y: f32, viewport_height: f32) -> operad::ScrollState {
    let mut scroll = scroll_state(offset_y, viewport_height, controls_list_content_height());
    scroll.offset = scroll.clamp_offset(scroll.offset);
    scroll
}

fn controls_list_content_height() -> f32 {
    SHOWCASE_WIDGET_WINDOW_IDS.len() as f32 * CONTROLS_WIDGET_ROW_HEIGHT
        + (SHOWCASE_WIDGET_WINDOW_IDS.len().saturating_sub(1)) as f32 * CONTROLS_WIDGET_ROW_GAP
}

fn caret_visible(phase: f32) -> bool {
    phase.sin() >= 0.0
}

fn open_url(url: &str) {
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let _ = url;
}

fn text(size: f32, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size: size,
        line_height: size + 5.0,
        color,
        ..Default::default()
    }
}

fn color(r: u8, g: u8, b: u8) -> ColorRgba {
    ColorRgba::new(r, g, b, 255)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operad::{ApproxTextMeasurer, AuditWarning};

    fn state_with_window(id: &str) -> ShowcaseState {
        let mut state = ShowcaseState::default();
        state.windows.clear_all();
        *state.windows.slot_mut(id).expect("known showcase window") = true;
        if id == "popup_panel" {
            state.popup_open = true;
        }
        if id == "overlays" {
            state.overlay_popup_open = true;
            state.overlay_modal_open = true;
        }
        state
    }

    fn severe_layout_warning(warning: &AuditWarning) -> bool {
        matches!(
            warning,
            AuditWarning::NonFiniteRect { .. }
                | AuditWarning::EmptyInteractiveClip { .. }
                | AuditWarning::TextClipped { .. }
                | AuditWarning::NodeOutsideRoot { .. }
                | AuditWarning::PaintItemEmptyClip { .. }
        )
    }

    #[test]
    fn showcase_windows_avoid_hard_clipping_at_common_viewport_sizes() {
        let viewports = [
            UiSize::new(900.0, 760.0),
            UiSize::new(720.0, 560.0),
            UiSize::new(1180.0, 820.0),
        ];

        for viewport in viewports {
            for id in SHOWCASE_WIDGET_WINDOW_IDS {
                let state = state_with_window(id);
                let mut document = state.view(viewport);
                document
                    .compute_layout(viewport, &mut ApproxTextMeasurer)
                    .expect("showcase layout");
                let warnings = document
                    .audit_layout()
                    .into_iter()
                    .filter(severe_layout_warning)
                    .collect::<Vec<_>>();
                assert!(
                    warnings.is_empty(),
                    "window {id:?} at {viewport:?} emitted severe layout warnings: {warnings:#?}"
                );
            }
        }
    }

    #[test]
    fn showcase_windows_survive_small_user_resizes() {
        let viewport = UiSize::new(900.0, 760.0);
        for id in SHOWCASE_WIDGET_WINDOW_IDS {
            let mut state = state_with_window(id);
            state
                .desktop
                .sizes
                .insert(id.to_string(), UiSize::new(220.0, 140.0));
            state.desktop.user_sized.insert(id.to_string());
            if id == "selection" {
                state.combo_open = true;
                state.dropdown.open = true;
            }
            if id == "slider" {
                state.slider_trailing_picker_open = true;
            }
            if id == "menus" {
                state.menu_button.open(&menu_items(state.menu_autosave));
                state
                    .context_menu
                    .open_with_items(UiPoint::new(160.0, 160.0), &menu_items(state.menu_autosave));
            }
            if id == "overlays" {
                state.overlay_popup_open = true;
                state.overlay_modal_open = true;
            }
            if id == "popup_panel" {
                state.popup_open = true;
            }

            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            let warnings = document
                .audit_layout()
                .into_iter()
                .filter(severe_layout_warning)
                .collect::<Vec<_>>();
            assert!(
                warnings.is_empty(),
                "resized window {id:?} emitted severe layout warnings: {warnings:#?}"
            );
        }
    }

    #[test]
    fn showcase_canvas_aspect_fits_when_window_is_short() {
        let mut state = state_with_window("canvas");
        state
            .desktop
            .sizes
            .insert("canvas".to_string(), UiSize::new(420.0, 160.0));
        state.desktop.user_sized.insert("canvas".to_string());

        let viewport = UiSize::new(900.0, 760.0);
        let mut document = state.view(viewport);
        document
            .compute_layout(viewport, &mut ApproxTextMeasurer)
            .expect("showcase layout");

        let canvas = document
            .nodes()
            .iter()
            .find(|node| node.name == "canvas.shader")
            .expect("canvas shader node");
        let rect = canvas.layout.rect;
        assert!(rect.width > 0.0 && rect.height > 0.0, "{rect:?}");
        assert!(
            (rect.width / rect.height - 16.0 / 9.0).abs() < 0.01,
            "{rect:?}"
        );
        assert!(rect.width < 300.0, "{rect:?}");
    }

    #[test]
    fn showcase_slider_primary_track_width_is_stable_when_window_resizes() {
        let mut widths = Vec::new();
        for window_width in [430.0, 340.0] {
            let mut state = state_with_window("slider");
            state
                .desktop
                .sizes
                .insert("slider".to_string(), UiSize::new(window_width, 360.0));
            state.desktop.user_sized.insert("slider".to_string());

            let viewport = UiSize::new(900.0, 760.0);
            let mut document = state.view(viewport);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .expect("showcase layout");
            let slider = document
                .nodes()
                .iter()
                .find(|node| node.name == "slider.value")
                .expect("primary slider node");
            widths.push(slider.layout.rect.width);
        }

        assert!((widths[0] - 180.0).abs() < 0.01, "{widths:?}");
        assert!((widths[1] - 180.0).abs() < 0.01, "{widths:?}");
    }

    #[test]
    fn showcase_slider_color_button_opens_inline_picker() {
        let mut state = state_with_window("slider");
        state.update(WidgetAction::activate(
            UiNodeId(0),
            "slider.trailing_color_button",
        ));
        assert!(state.slider_trailing_picker_open);

        let viewport = UiSize::new(900.0, 760.0);
        let mut document = state.view(viewport);
        document
            .compute_layout(viewport, &mut ApproxTextMeasurer)
            .expect("showcase layout");
        assert!(document
            .nodes()
            .iter()
            .any(|node| node.name == "slider.trailing_picker"));
        assert!(!document
            .nodes()
            .iter()
            .any(|node| node.name == "slider.trailing_color_button.label"));
    }

    #[test]
    fn showcase_progress_phase_does_not_wrap_on_tick() {
        let mut state = ShowcaseState::default();
        state.progress_phase = std::f32::consts::TAU - 0.001;
        state.update(WidgetAction::activate(UiNodeId(0), "runtime.tick"));
        assert!(state.progress_phase > std::f32::consts::TAU);
    }
}
