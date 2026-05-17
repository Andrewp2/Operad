use operad::{
    root_style, widgets, CanvasContent, CanvasRenderProgram, ColorRgba, LayoutStyle,
    NativeWindowOptions, StrokeStyle, TextStyle, UiDocument, UiNode, UiSize, UiVisual,
    WidgetAction,
};

fn main() -> operad::NativeWindowResult {
    operad::run_app_with(
        NativeWindowOptions::new("Canvas app")
            .with_min_size(520.0, 380.0)
            .with_tick_action("runtime.tick")
            .with_tick_rate_hz(60.0),
        CanvasApp::default(),
        CanvasApp::update,
        CanvasApp::view,
    )
}

#[derive(Default)]
struct CanvasApp {
    phase: f32,
}

impl CanvasApp {
    fn update(&mut self, action: WidgetAction) {
        if action
            .binding
            .action_id()
            .is_some_and(|id| id.as_str() == "runtime.tick")
        {
            self.phase = (self.phase + 0.01) % 1.0;
        }
    }

    fn view(&self, viewport: UiSize) -> UiDocument {
        let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
        let panel = ui.add_child(
            ui.root,
            UiNode::container(
                "canvas.app",
                LayoutStyle::column()
                    .with_width_percent(1.0)
                    .with_height_percent(1.0)
                    .with_padding(16.0)
                    .with_gap(10.0),
            )
            .with_visual(UiVisual::panel(ColorRgba::new(13, 17, 23, 255), None, 0.0)),
        );
        widgets::label(
            &mut ui,
            panel,
            "canvas.title",
            "WGPU canvas",
            heading(),
            LayoutStyle::new().with_width_percent(1.0).with_height(32.0),
        );
        let mut options = widgets::CanvasOptions::default()
            .with_accessibility_label("Animated shader canvas")
            .with_aspect_ratio(16.0 / 9.0);
        options.layout = LayoutStyle::new()
            .with_width_percent(1.0)
            .with_height(0.0)
            .with_flex_grow(1.0);
        options.visual = UiVisual::panel(
            ColorRgba::new(18, 22, 28, 255),
            Some(StrokeStyle::new(ColorRgba::new(58, 68, 84, 255), 1.0)),
            4.0,
        );
        widgets::canvas(
            &mut ui,
            panel,
            "canvas.preview",
            CanvasContent::new("canvas.preview").program(shader(self.phase)),
            options,
        );
        ui
    }
}

fn shader(phase: f32) -> CanvasRenderProgram {
    CanvasRenderProgram::wgsl(
        r#"
override PHASE: f32 = 0.0;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5, 0.5);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let p = input.uv * 2.0 - vec2<f32>(1.0, 1.0);
    let wave = 0.5 + 0.5 * sin((p.x + p.y + PHASE * 6.28318) * 5.0);
    let glow = 1.0 / (1.0 + dot(p, p) * 3.0);
    let color = vec3<f32>(0.10, 0.32, 0.72) * glow + vec3<f32>(0.22, 0.70, 0.56) * wave * 0.35;
    return vec4<f32>(color, 1.0);
}
"#,
    )
    .label("template.canvas")
    .constant("PHASE", phase as f64)
    .clear_color(Some(ColorRgba::new(18, 22, 28, 255)))
}

fn heading() -> TextStyle {
    TextStyle {
        font_size: 22.0,
        line_height: 30.0,
        color: ColorRgba::WHITE,
        ..TextStyle::default()
    }
}
