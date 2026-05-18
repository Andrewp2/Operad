use operad::native::{NativeWindowOptions, NativeWindowResult};
use operad::widgets::{self, TextInputOptions, TextInputState};
use operad::{
    root_style, ColorRgba, LayoutStyle, TextStyle, UiDocument, UiNode, UiNodeId, UiSize, UiVisual,
    WidgetAction, WidgetActionKind,
};

fn main() -> NativeWindowResult {
    operad::native::run_app_with(
        NativeWindowOptions::new("Simple form").with_min_size(420.0, 300.0),
        FormApp::default(),
        FormApp::update,
        FormApp::view,
    )
}

struct FormApp {
    name: TextInputState,
    email: TextInputState,
    submitted: String,
}

impl Default for FormApp {
    fn default() -> Self {
        Self {
            name: TextInputState::new(""),
            email: TextInputState::new(""),
            submitted: String::new(),
        }
    }
}

impl FormApp {
    fn update(&mut self, action: WidgetAction) {
        let Some(action_id) = action.binding.action_id().map(|id| id.as_str()) else {
            return;
        };
        match action_id {
            "form.name.edit" => apply_text_edit(&mut self.name, &action.kind),
            "form.email.edit" => apply_text_edit(&mut self.email, &action.kind),
            "form.submit" => {
                self.submitted = format!("Submitted {} <{}>", self.name.text(), self.email.text());
            }
            _ => {}
        }
    }

    fn view(&self, viewport: UiSize) -> UiDocument {
        let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
        let panel = app_panel(&mut ui, "form.panel", viewport, 420.0, 260.0);
        widgets::label(
            &mut ui,
            panel,
            "form.title",
            "Simple form",
            heading(),
            LayoutStyle::new().with_width_percent(1.0).with_height(32.0),
        );
        form_field(&mut ui, panel, "Name", "form.name", &self.name);
        form_field(&mut ui, panel, "Email", "form.email", &self.email);
        widgets::button(
            &mut ui,
            panel,
            "form.submit",
            "Submit",
            widgets::ButtonOptions::default().with_action("form.submit"),
        );
        widgets::label(
            &mut ui,
            panel,
            "form.status",
            if self.submitted.is_empty() {
                "Enter values and submit."
            } else {
                &self.submitted
            },
            muted(),
            LayoutStyle::new().with_width_percent(1.0).with_height(28.0),
        );
        ui
    }
}

fn form_field(
    ui: &mut UiDocument,
    parent: UiNodeId,
    label: &str,
    name: &str,
    state: &TextInputState,
) {
    let row = ui.add_child(
        parent,
        UiNode::container(
            format!("{name}.row"),
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height(38.0)
                .with_gap(8.0),
        ),
    );
    widgets::label(
        ui,
        row,
        format!("{name}.label"),
        label,
        TextStyle::default(),
        LayoutStyle::size(76.0, 32.0),
    );
    widgets::text_input(
        ui,
        row,
        name,
        state,
        TextInputOptions::default()
            .with_layout(LayoutStyle::new().with_width(260.0).with_height(32.0))
            .with_edit_action(format!("{name}.edit")),
    );
}

fn apply_text_edit(state: &mut TextInputState, kind: &WidgetActionKind) {
    if let WidgetActionKind::TextEdit(edit) = kind {
        state.apply_widget_text_edit(edit, &TextInputOptions::default());
    }
}

fn app_panel(
    ui: &mut UiDocument,
    name: &str,
    viewport: UiSize,
    width: f32,
    height: f32,
) -> UiNodeId {
    ui.add_child(
        ui.root(),
        UiNode::container(
            name,
            LayoutStyle::column()
                .with_size(width.min(viewport.width.max(1.0)), height)
                .with_padding(16.0)
                .with_gap(10.0),
        )
        .with_visual(UiVisual::panel(ColorRgba::new(24, 29, 36, 255), None, 6.0)),
    )
}

fn heading() -> TextStyle {
    TextStyle {
        font_size: 22.0,
        line_height: 30.0,
        color: ColorRgba::WHITE,
        ..TextStyle::default()
    }
}

fn muted() -> TextStyle {
    TextStyle {
        color: ColorRgba::new(166, 178, 196, 255),
        ..TextStyle::default()
    }
}
