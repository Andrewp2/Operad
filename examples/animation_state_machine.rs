use operad::native::{NativeWindowOptions, NativeWindowResult};
use operad::{
    layout, root_style, widgets, AccessibilityMeta, AccessibilityRole, AnimatedValues,
    AnimationCondition, AnimationMachine, AnimationNumberComparison, AnimationState,
    AnimationTransition, ColorRgba, InputBehavior, LayoutStyle, ScenePrimitive, StrokeStyle,
    TextStyle, UiDocument, UiNode, UiPoint, UiRect, UiSize, UiVisual, WidgetAction,
};

const ACTION_GOTO_PREFIX: &str = "animation.goto.";
const INPUT_TARGET: &str = "target";
const SHAPE_WIDTH: f32 = 128.0;
const SHAPE_HEIGHT: f32 = 108.0;
const MOTION_MARGIN: f32 = 24.0;

fn main() -> NativeWindowResult {
    operad::native::run_app_with(
        NativeWindowOptions::new("Animation state machine"),
        AnimationApp::default(),
        AnimationApp::update,
        AnimationApp::view,
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum DemoState {
    #[default]
    A,
    B,
    C,
}

impl DemoState {
    const ALL: [Self; 3] = [Self::A, Self::B, Self::C];

    fn position(self) -> UiPoint {
        match self {
            Self::A => UiPoint::new(76.0, 0.0),
            Self::B => UiPoint::new(0.0, 118.0),
            Self::C => UiPoint::new(152.0, 118.0),
        }
    }

    fn color(self) -> ColorRgba {
        match self {
            Self::A => ColorRgba::new(88, 214, 141, 255),
            Self::B => ColorRgba::new(235, 88, 88, 255),
            Self::C => ColorRgba::new(84, 156, 255, 255),
        }
    }

    fn targets(self) -> [Self; 2] {
        match self {
            Self::A => [Self::B, Self::C],
            Self::B => [Self::A, Self::C],
            Self::C => [Self::A, Self::B],
        }
    }

    fn target_input(self) -> f32 {
        match self {
            Self::A => 0.0,
            Self::B => 1.0,
            Self::C => 2.0,
        }
    }

    fn state_name(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => "b",
            Self::C => "c",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }

    fn action(self) -> String {
        format!("{ACTION_GOTO_PREFIX}{}", self.state_name())
    }

    fn from_action(action_id: &str) -> Option<Self> {
        let state = action_id.strip_prefix(ACTION_GOTO_PREFIX)?;
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.state_name() == state)
    }
}

#[derive(Default)]
struct AnimationApp {
    state: DemoState,
}

impl AnimationApp {
    fn update(&mut self, action: WidgetAction) {
        let Some(target) = action
            .binding
            .action_id()
            .and_then(|id| DemoState::from_action(id.as_str()))
        else {
            return;
        };
        if target != self.state {
            self.state = target;
        }
    }

    fn view(&self, viewport: UiSize) -> UiDocument {
        let mut ui = UiDocument::new(root_style(viewport.width, viewport.height));
        let panel = ui.add_child(
            ui.root(),
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

        let controls = ui.add_child(
            panel,
            UiNode::container(
                "animation.controls",
                LayoutStyle::column().with_width_percent(1.0).with_gap(8.0),
            ),
        );

        let states = ui.add_child(
            controls,
            UiNode::container(
                "animation.states",
                LayoutStyle::row().with_width_percent(1.0).with_gap(8.0),
            ),
        );
        for state in DemoState::ALL {
            state_chip(&mut ui, states, state, state == self.state);
        }

        let transitions = ui.add_child(
            controls,
            UiNode::container(
                "animation.transitions",
                LayoutStyle::row().with_width_percent(1.0).with_gap(8.0),
            ),
        );
        for target in self.state.targets() {
            widgets::button(
                &mut ui,
                transitions,
                target.action(),
                format!("To {}", target.label()),
                widgets::ButtonOptions::default().with_action(target.action()),
            );
        }

        let machine = shape_machine(self.state);
        let shape_bounds = machine.animation_bounds(shape_rect());
        let scene_width = shape_bounds.width + MOTION_MARGIN * 2.0;
        let scene_height = shape_bounds.height + MOTION_MARGIN * 2.0;
        let scene_origin = UiPoint::new(
            MOTION_MARGIN - shape_bounds.x,
            MOTION_MARGIN - shape_bounds.y,
        );
        let stage = ui.add_child(
            panel,
            UiNode::container(
                "animation.stage",
                layout::with_centered_children(
                    LayoutStyle::row()
                        .with_width_percent(1.0)
                        .with_flex_grow(1.0),
                ),
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
                shape_primitives(scene_origin),
                LayoutStyle::size(scene_width, scene_height),
            )
            .with_input(InputBehavior::BUTTON)
            .with_animation(machine)
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label("Animated state machine shape")
                    .focusable(),
            ),
        );
        ui
    }
}

fn state_chip(ui: &mut UiDocument, parent: operad::UiNodeId, state: DemoState, active: bool) {
    let chip = ui.add_child(
        parent,
        UiNode::container(
            format!("animation.state.{}", state.state_name()),
            LayoutStyle::row().with_padding(8.0),
        )
        .with_visual(UiVisual::panel(
            if active {
                ColorRgba::new(42, 72, 92, 255)
            } else {
                ColorRgba::new(25, 31, 39, 255)
            },
            Some(StrokeStyle::new(
                if active {
                    state.color()
                } else {
                    ColorRgba::new(58, 68, 84, 255)
                },
                1.0,
            )),
            4.0,
        )),
    );
    widgets::label(
        ui,
        chip,
        format!("animation.state.{}.label", state.state_name()),
        state.label(),
        state_text(active),
        LayoutStyle::new(),
    );
}

fn shape_machine(state: DemoState) -> AnimationMachine {
    let a = state_values(DemoState::A, 0.0, 0.86);
    let b = state_values(DemoState::B, 1.0, 0.94);
    let c = state_values(DemoState::C, 2.0, 1.0);
    AnimationMachine::new(
        vec![
            AnimationState::new("a", a),
            AnimationState::new("b", b),
            AnimationState::new("c", c),
        ],
        vec![
            target_transition(DemoState::A, DemoState::B, 0.66),
            target_transition(DemoState::A, DemoState::C, 0.96),
            target_transition(DemoState::B, DemoState::C, 0.66),
            target_transition(DemoState::B, DemoState::A, 0.72),
            target_transition(DemoState::C, DemoState::A, 0.84),
            target_transition(DemoState::C, DemoState::B, 0.66),
        ],
        "a",
    )
    .unwrap_or_else(|_| AnimationMachine::single_state("a", a))
    .with_number_input(INPUT_TARGET, state.target_input())
}

fn state_values(state: DemoState, morph: f32, opacity: f32) -> AnimatedValues {
    AnimatedValues::new(opacity, state.position(), 1.0)
        .with_morph(morph)
        .with_fill_color(state.color())
}

fn target_transition(from: DemoState, to: DemoState, duration: f32) -> AnimationTransition {
    AnimationTransition::when(
        from.state_name(),
        to.state_name(),
        AnimationCondition::number(
            INPUT_TARGET,
            AnimationNumberComparison::Equal,
            to.target_input(),
        ),
        duration,
    )
}

fn shape_rect() -> UiRect {
    UiRect::new(0.0, 0.0, SHAPE_WIDTH, SHAPE_HEIGHT)
}

fn shape_primitives(origin: UiPoint) -> Vec<ScenePrimitive> {
    vec![ScenePrimitive::MorphPolygonKeyframes {
        frames: vec![
            offset_points(clover_points(UiPoint::new(64.0, 56.0), 72), origin),
            offset_points(heart_points(UiPoint::new(64.0, 54.0), 2.5, 72), origin),
            offset_points(circle_points(UiPoint::new(64.0, 56.0), 40.0, 72), origin),
        ],
        amount: 0.0,
        fill: DemoState::A.color(),
        stroke: Some(StrokeStyle::new(ColorRgba::new(236, 244, 255, 255), 1.5)),
    }]
}

fn offset_points(points: impl IntoIterator<Item = UiPoint>, offset: UiPoint) -> Vec<UiPoint> {
    points
        .into_iter()
        .map(|point| offset_point(point, offset))
        .collect()
}

fn offset_point(point: UiPoint, offset: UiPoint) -> UiPoint {
    UiPoint::new(point.x + offset.x, point.y + offset.y)
}

fn clover_points(center: UiPoint, count: usize) -> Vec<UiPoint> {
    (0..count)
        .map(|index| {
            let angle =
                -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::TAU / count as f32;
            let radius = 24.0 + 19.0 * (4.0 * angle).cos();
            UiPoint::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )
        })
        .collect()
}

fn heart_points(center: UiPoint, scale: f32, count: usize) -> Vec<UiPoint> {
    (0..count)
        .map(|index| {
            let angle = index as f32 * std::f32::consts::TAU / count as f32;
            let sin = angle.sin();
            let x = 16.0 * sin * sin * sin;
            let y = -(13.0 * angle.cos()
                - 5.0 * (2.0 * angle).cos()
                - 2.0 * (3.0 * angle).cos()
                - (4.0 * angle).cos());
            UiPoint::new(center.x + x * scale, center.y + y * scale)
        })
        .collect()
}

fn circle_points(center: UiPoint, radius: f32, count: usize) -> Vec<UiPoint> {
    (0..count)
        .map(|index| {
            let angle =
                -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::TAU / count as f32;
            UiPoint::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )
        })
        .collect()
}

fn state_text(active: bool) -> TextStyle {
    TextStyle {
        color: if active {
            ColorRgba::new(236, 248, 250, 255)
        } else {
            ColorRgba::new(173, 188, 202, 255)
        },
        ..TextStyle::default()
    }
}
