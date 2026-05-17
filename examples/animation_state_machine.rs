use operad::{
    root_style, widgets, AccessibilityMeta, AccessibilityRole, AnimatedValues,
    AnimationBlendBinding, AnimationCondition, AnimationMachine, AnimationState,
    AnimationTransition, ColorRgba, InputBehavior, LayoutStyle, NativeWindowOptions,
    ScenePrimitive, StrokeStyle, TextStyle, UiDocument, UiNode, UiPoint, UiSize, UiVisual,
    WidgetAction,
};

const INPUT_OPEN: &str = "open";
const INPUT_MORPH: &str = "morph";

fn main() -> operad::NativeWindowResult {
    operad::run_app_with(
        NativeWindowOptions::new("Animation state machine").with_min_size(540.0, 360.0),
        AnimationApp::default(),
        AnimationApp::update,
        AnimationApp::view,
    )
}

#[derive(Default)]
struct AnimationApp {
    open: bool,
}

impl AnimationApp {
    fn update(&mut self, action: WidgetAction) {
        if action
            .binding
            .action_id()
            .is_some_and(|id| id.as_str() == "animation.toggle")
        {
            self.open = !self.open;
        }
    }

    fn view(&self, viewport: UiSize) -> UiDocument {
        let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
        let panel = ui.add_child(
            ui.root,
            UiNode::container(
                "animation.panel",
                LayoutStyle::column()
                    .with_width_percent(1.0)
                    .with_height_percent(1.0)
                    .with_padding(16.0)
                    .with_gap(12.0),
            )
            .with_visual(UiVisual::panel(ColorRgba::new(13, 17, 23, 255), None, 0.0)),
        );

        widgets::label(
            &mut ui,
            panel,
            "animation.title",
            "Animation state machine",
            heading(),
            LayoutStyle::new().with_width_percent(1.0).with_height(34.0),
        );

        let row = ui.add_child(
            panel,
            UiNode::container(
                "animation.controls",
                LayoutStyle::row()
                    .with_width_percent(1.0)
                    .with_height(40.0)
                    .with_gap(10.0),
            ),
        );
        widgets::button(
            &mut ui,
            row,
            "animation.toggle",
            if self.open { "Close" } else { "Open" },
            widgets::ButtonOptions::default().with_action("animation.toggle"),
        );
        widgets::label(
            &mut ui,
            row,
            "animation.state",
            if self.open {
                "State input: open"
            } else {
                "State input: closed"
            },
            muted(),
            LayoutStyle::new().with_width(220.0).with_height(32.0),
        );

        let stage = ui.add_child(
            panel,
            UiNode::container(
                "animation.stage",
                LayoutStyle::row()
                    .with_width_percent(1.0)
                    .with_height(0.0)
                    .with_flex_grow(1.0),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(16, 21, 28, 255),
                Some(StrokeStyle::new(ColorRgba::new(58, 68, 84, 255), 1.0)),
                6.0,
            )),
        );

        ui.add_child(
            stage,
            UiNode::scene(
                "animation.shape",
                shape_primitives(),
                LayoutStyle::new()
                    .with_width_percent(1.0)
                    .with_height_percent(1.0),
            )
            .with_input(InputBehavior::BUTTON)
            .with_animation(shape_machine(self.open))
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label("Animated morphing shape")
                    .focusable(),
            ),
        );
        ui
    }
}

fn shape_machine(open: bool) -> AnimationMachine {
    let closed = AnimatedValues::new(0.82, UiPoint::new(0.0, 0.0), 1.0).with_morph(0.0);
    let open_values = AnimatedValues::new(1.0, UiPoint::new(160.0, 0.0), 1.08).with_morph(1.0);
    AnimationMachine::new(
        vec![
            AnimationState::new("closed", closed),
            AnimationState::new("open", open_values),
        ],
        vec![
            AnimationTransition::when(
                "closed",
                "open",
                AnimationCondition::bool(INPUT_OPEN, true),
                0.24,
            ),
            AnimationTransition::when(
                "open",
                "closed",
                AnimationCondition::bool(INPUT_OPEN, false),
                0.18,
            ),
        ],
        "closed",
    )
    .unwrap_or_else(|_| AnimationMachine::single_state("closed", closed))
    .with_bool_input(INPUT_OPEN, open)
    .with_number_input(INPUT_MORPH, if open { 1.0 } else { 0.0 })
    .with_blend_binding(AnimationBlendBinding::new(INPUT_MORPH, "closed", "open"))
}

fn shape_primitives() -> Vec<ScenePrimitive> {
    vec![
        ScenePrimitive::MorphPolygon {
            from_points: vec![
                UiPoint::new(76.0, 54.0),
                UiPoint::new(156.0, 54.0),
                UiPoint::new(156.0, 134.0),
                UiPoint::new(76.0, 134.0),
            ],
            to_points: pentagon_points(UiPoint::new(116.0, 94.0), 48.0),
            amount: 0.0,
            fill: ColorRgba::new(120, 210, 180, 255),
            stroke: Some(StrokeStyle::new(ColorRgba::new(236, 244, 255, 255), 1.5)),
        },
        ScenePrimitive::Circle {
            center: UiPoint::new(102.0, 78.0),
            radius: 8.0,
            fill: ColorRgba::new(244, 248, 255, 255),
            stroke: None,
        },
    ]
}

fn pentagon_points(center: UiPoint, radius: f32) -> Vec<UiPoint> {
    (0..5)
        .map(|index| {
            let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::TAU / 5.0;
            UiPoint::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )
        })
        .collect()
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
