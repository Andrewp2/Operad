use super::*;

pub const SLIDER_THUMB_SIZE: f32 = 12.0;
pub const SLIDER_THUMB_RADIUS: f32 = SLIDER_THUMB_SIZE / 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderThumbShape {
    Circle,
    Square,
    Rectangle,
}

impl SliderThumbShape {
    const fn size(self) -> UiSize {
        match self {
            Self::Circle | Self::Square => UiSize::new(SLIDER_THUMB_SIZE, SLIDER_THUMB_SIZE),
            Self::Rectangle => UiSize::new(18.0, SLIDER_THUMB_SIZE),
        }
    }

    const fn radius(self) -> f32 {
        match self {
            Self::Circle => SLIDER_THUMB_RADIUS,
            Self::Square | Self::Rectangle => 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderClamping {
    Never,
    Edits,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderValueSpec {
    pub min: f32,
    pub max: f32,
    pub step: Option<f32>,
    pub logarithmic: bool,
    pub clamping: SliderClamping,
    pub smart_aim: bool,
}

impl SliderValueSpec {
    pub fn new(min: f32, max: f32) -> Self {
        let min = finite_or_f32(min, 0.0);
        let max = finite_or_f32(max, min + 1.0);
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        Self {
            min,
            max,
            step: None,
            logarithmic: false,
            clamping: SliderClamping::Always,
            smart_aim: false,
        }
    }

    pub fn step(mut self, step: f32) -> Self {
        if step.is_finite() && step > f32::EPSILON {
            self.step = Some(step);
        }
        self
    }

    pub const fn logarithmic(mut self, logarithmic: bool) -> Self {
        self.logarithmic = logarithmic;
        self
    }

    pub const fn clamping(mut self, clamping: SliderClamping) -> Self {
        self.clamping = clamping;
        self
    }

    pub const fn smart_aim(mut self, smart_aim: bool) -> Self {
        self.smart_aim = smart_aim;
        self
    }

    pub fn clamp(self, value: f32) -> f32 {
        finite_or_f32(value, self.min).clamp(self.min, self.max)
    }

    pub fn normalize(self, value: f32) -> f32 {
        let min = self.min.min(self.max - f32::EPSILON);
        if !self.logarithmic {
            return ((value - min) / (self.max - min).max(f32::EPSILON)).clamp(0.0, 1.0);
        }
        let positive_min = min.max(0.0001);
        let positive_max = self.max.max(positive_min + f32::EPSILON);
        let value = value.clamp(positive_min, positive_max);
        ((value.ln() - positive_min.ln()) / (positive_max.ln() - positive_min.ln())).clamp(0.0, 1.0)
    }

    pub fn value_at_unit(self, unit_value: f32) -> f32 {
        let unit_value = unit_value.clamp(0.0, 1.0);
        let min = self.min.min(self.max - f32::EPSILON);
        let value = if self.logarithmic {
            let positive_min = min.max(0.0001);
            let positive_max = self.max.max(positive_min + f32::EPSILON);
            (positive_min.ln() + (positive_max.ln() - positive_min.ln()) * unit_value).exp()
        } else {
            min + (self.max - min) * unit_value
        };
        self.adjust_value(value)
    }

    pub fn value_from_control_point(self, control: UiRect, point: UiPoint) -> f32 {
        self.value_at_unit(slider_value_from_control_point(control, point, 0.0..1.0))
    }

    pub fn adjust_value(self, value: f32) -> f32 {
        let mut value = if self.clamping == SliderClamping::Always {
            self.clamp(value)
        } else {
            finite_or_f32(value, self.min)
        };
        if let Some(step) = self.step {
            value = round_slider_to_step(value, step);
            if self.smart_aim && !self.logarithmic {
                value = smart_aim_slider_value(value, step);
            }
        }
        if self.clamping == SliderClamping::Always {
            value = self.clamp(value);
        }
        value
    }

    pub fn format_value(self, value: f32) -> String {
        format_slider_value(value)
    }
}

#[derive(Debug, Clone)]
pub struct SliderOptions {
    pub layout: LayoutStyle,
    pub track_visual: UiVisual,
    pub fill_color: ColorRgba,
    pub thumb_visual: UiVisual,
    pub disabled_track_visual: Option<UiVisual>,
    pub disabled_fill_color: Option<ColorRgba>,
    pub disabled_thumb_visual: Option<UiVisual>,
    pub track_shader: Option<ShaderEffect>,
    pub fill_shader: Option<ShaderEffect>,
    pub thumb_shader: Option<ShaderEffect>,
    pub shader: Option<ShaderEffect>,
    pub animation: Option<AnimationMachine>,
    pub enabled: bool,
    pub thumb_shape: SliderThumbShape,
    pub drag_action: Option<WidgetActionBinding>,
    pub value_edit_action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_value: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for SliderOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: length(160.0),
                    height: length(28.0),
                },
                flex_shrink: 0.0,
                ..Default::default()
            }),
            track_visual: UiVisual::panel(ColorRgba::new(42, 49, 58, 255), None, 3.0),
            fill_color: ColorRgba::new(108, 180, 255, 255),
            thumb_visual: UiVisual::panel(
                ColorRgba::new(235, 240, 247, 255),
                Some(StrokeStyle::new(ColorRgba::new(79, 93, 113, 255), 1.0)),
                6.0,
            ),
            disabled_track_visual: Some(UiVisual::panel(
                ColorRgba::new(35, 39, 45, 180),
                None,
                3.0,
            )),
            disabled_fill_color: Some(ColorRgba::new(92, 101, 114, 180)),
            disabled_thumb_visual: Some(UiVisual::panel(
                ColorRgba::new(150, 158, 170, 180),
                Some(StrokeStyle::new(ColorRgba::new(81, 90, 104, 180), 1.0)),
                6.0,
            )),
            track_shader: None,
            fill_shader: None,
            thumb_shader: None,
            shader: None,
            animation: None,
            enabled: true,
            thumb_shape: SliderThumbShape::Circle,
            drag_action: None,
            value_edit_action: None,
            accessibility_label: None,
            accessibility_value: None,
            accessibility_hint: None,
        }
    }
}

impl SliderOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_drag_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.drag_action = Some(action.into());
        self
    }

    pub fn with_value_edit_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.value_edit_action = Some(action.into());
        self
    }
}

pub fn slider(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    value: f32,
    range: Range<f32>,
    options: SliderOptions,
) -> UiNodeId {
    let name = name.into();
    let t = normalized_value(value, range.clone());
    let thumb_size = options.thumb_shape.size();
    let thumb_radius = options.thumb_shape.radius();
    let mut layout = options.layout.style.clone();
    layout.display = Display::Flex;
    layout.flex_direction = FlexDirection::Row;
    layout.align_items = Some(AlignItems::Center);
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Slider)
        .label(
            options
                .accessibility_label
                .clone()
                .unwrap_or_else(|| name.clone()),
        )
        .value(
            options
                .accessibility_value
                .clone()
                .unwrap_or_else(|| slider_accessibility_value(value, range.clone())),
        )
        .value_range(AccessibilityValueRange::new(
            range.start as f64,
            range.end as f64,
        ))
        .action(AccessibilityAction::new("increase", "Increase"))
        .action(AccessibilityAction::new("decrease", "Decrease"));
    if let Some(hint) = options.accessibility_hint.clone() {
        accessibility = accessibility.hint(hint);
    }
    if options.enabled {
        accessibility = accessibility.focusable();
    } else {
        accessibility = accessibility.disabled();
    }
    let mut root_node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout,
            clip: ClipBehavior::None,
            ..Default::default()
        },
    )
    .with_input(if options.enabled {
        InputBehavior::BUTTON
    } else {
        InputBehavior::NONE
    })
    .with_accessibility(accessibility);
    if let Some(shader) = options.shader {
        root_node = root_node.with_shader(shader);
    }
    if let Some(animation) = options.animation {
        root_node = root_node.with_animation(animation);
    }
    if let Some(action) = options.value_edit_action.clone() {
        root_node = root_node.with_pointer_edit_action(action);
    } else if let Some(action) = options.drag_action.clone() {
        root_node = root_node.with_action(action);
    }
    let root = document.add_child(parent, root_node);
    let track_visual = if options.enabled {
        options.track_visual
    } else {
        options
            .disabled_track_visual
            .unwrap_or(options.track_visual)
    };
    let fill_color = if options.enabled {
        options.fill_color
    } else {
        options.disabled_fill_color.unwrap_or(options.fill_color)
    };
    let thumb_visual = if options.enabled {
        options.thumb_visual
    } else {
        options
            .disabled_thumb_visual
            .unwrap_or(options.thumb_visual)
    };
    let mut track_node = UiNode::container(
        format!("{name}.track"),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                position: taffy::prelude::Position::Absolute,
                inset: taffy::prelude::Rect {
                    left: taffy::prelude::LengthPercentageAuto::length(0.0),
                    right: taffy::prelude::LengthPercentageAuto::auto(),
                    top: taffy::prelude::LengthPercentageAuto::percent(0.5),
                    bottom: taffy::prelude::LengthPercentageAuto::auto(),
                },
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(6.0),
                },
                margin: taffy::prelude::Rect {
                    left: taffy::prelude::LengthPercentageAuto::length(0.0),
                    right: taffy::prelude::LengthPercentageAuto::length(0.0),
                    top: taffy::prelude::LengthPercentageAuto::length(-3.0),
                    bottom: taffy::prelude::LengthPercentageAuto::length(0.0),
                },
                ..Default::default()
            })
            .style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_visual(track_visual);
    if let Some(shader) = options.track_shader {
        track_node = track_node.with_shader(shader);
    }
    let track = document.add_child(root, track_node);
    let mut fill_node = UiNode::container(
        format!("{name}.fill"),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(t),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            })
            .style,
            ..Default::default()
        },
    )
    .with_visual(UiVisual::panel(fill_color, None, 3.0));
    if let Some(shader) = options.fill_shader {
        fill_node = fill_node.with_shader(shader);
    }
    document.add_child(track, fill_node);
    document.add_child(
        root,
        UiNode::container(
            format!("{name}.before_thumb"),
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(0.0),
                    height: length(1.0),
                },
                flex_grow: t,
                flex_shrink: 1.0,
                ..Default::default()
            }),
        ),
    );
    let mut thumb_node = UiNode::container(
        format!("{name}.thumb"),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(thumb_size.width),
                    height: length(thumb_size.height),
                },
                flex_shrink: 0.0,
                ..Default::default()
            })
            .style,
            z_index: 3.0,
            ..Default::default()
        },
    )
    .with_visual(UiVisual::panel(
        thumb_visual.fill,
        thumb_visual.stroke,
        thumb_radius,
    ));
    if let Some(shader) = options.thumb_shader {
        thumb_node = thumb_node.with_shader(shader);
    }
    document.add_child(root, thumb_node);
    document.add_child(
        root,
        UiNode::container(
            format!("{name}.after_thumb"),
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(0.0),
                    height: length(1.0),
                },
                flex_grow: 1.0 - t,
                flex_shrink: 1.0,
                ..Default::default()
            }),
        ),
    );
    root
}

fn slider_accessibility_value(value: f32, range: Range<f32>) -> String {
    let percent = normalized_value(value, range) * 100.0;
    format!("{value} ({percent:.0}%)")
}

pub fn normalized_value(value: f32, range: Range<f32>) -> f32 {
    let span = range.end - range.start;
    if span.abs() <= f32::EPSILON {
        return 0.0;
    }
    ((value - range.start) / span).clamp(0.0, 1.0)
}

pub fn slider_value_from_point(track: UiRect, point: UiPoint, range: Range<f32>) -> f32 {
    let t = ((point.x - track.x) / track.width.max(1.0)).clamp(0.0, 1.0);
    range.start + (range.end - range.start) * t
}

pub fn slider_value_from_control_point(control: UiRect, point: UiPoint, range: Range<f32>) -> f32 {
    let usable_width = (control.width - SLIDER_THUMB_SIZE).max(1.0);
    let t = ((point.x - control.x - SLIDER_THUMB_RADIUS) / usable_width).clamp(0.0, 1.0);
    range.start + (range.end - range.start) * t
}

pub fn format_slider_value(value: f32) -> String {
    if (value - value.round()).abs() < 0.000_5 {
        format!("{value:.0}")
    } else if value.abs() < 10.0 {
        format!("{value:.3}")
    } else if value.abs() < 1000.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.1}")
    }
}

pub fn round_slider_to_step(value: f32, step: f32) -> f32 {
    if step <= f32::EPSILON {
        return value;
    }
    (value / step).round() * step
}

pub fn smart_aim_slider_value(value: f32, step: f32) -> f32 {
    let coarse = (step * 5.0).max(1.0);
    let aimed = round_slider_to_step(value, coarse);
    if (value - aimed).abs() <= coarse * 0.08 {
        aimed
    } else {
        value
    }
}

fn finite_or_f32(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

pub fn slider_actions_from_gesture_event(
    document: &UiDocument,
    slider: UiNodeId,
    options: &SliderOptions,
    event: &GestureEvent,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    push_slider_gesture_event_actions(&mut queue, document, slider, options, event);
    queue
}

pub fn push_slider_gesture_event_actions<'a>(
    queue: &'a mut WidgetActionQueue,
    document: &UiDocument,
    slider: UiNodeId,
    options: &SliderOptions,
    event: &GestureEvent,
) -> &'a mut WidgetActionQueue {
    let GestureEvent::Drag(gesture) = event else {
        return queue;
    };
    if !document.node_is_descendant_or_self(slider, gesture.target)
        || !action_target_enabled(document, slider)
    {
        return queue;
    }
    let mut gesture = *gesture;
    gesture.target = slider;
    if let Some(binding) = options.drag_action.clone() {
        if let Some(action) = WidgetAction::drag_from_gesture(&gesture, binding) {
            queue.push(action);
        }
    }
    if let Some(binding) = options.value_edit_action.clone() {
        queue.push(WidgetAction::value_edit_from_drag(&gesture, binding));
    }
    queue
}

#[cfg(test)]
mod tests {
    use crate::{root_style, ApproxTextMeasurer};

    use super::*;

    #[test]
    fn slider_thumb_is_centered_on_track() {
        let mut document = UiDocument::new(root_style(240.0, 80.0));
        let root = document.root;
        let slider = slider(
            &mut document,
            root,
            "gain",
            0.58,
            0.0..1.0,
            SliderOptions::default()
                .with_layout(LayoutStyle::new().with_width(200.0).with_height(30.0)),
        );
        document
            .compute_layout(UiSize::new(240.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let track = document.node(document.node(slider).children[0]).layout.rect;
        let thumb = document.node(document.node(slider).children[2]).layout.rect;
        let track_center_y = track.y + track.height / 2.0;
        let thumb_center_y = thumb.y + thumb.height / 2.0;

        assert!(
            (track_center_y - thumb_center_y).abs() < 0.5,
            "track={track:?}, thumb={thumb:?}"
        );
    }

    #[test]
    fn slider_thumb_paints_above_track_and_fill() {
        let mut document = UiDocument::new(root_style(240.0, 80.0));
        let root = document.root;
        slider(
            &mut document,
            root,
            "gain",
            0.58,
            0.0..1.0,
            SliderOptions::default()
                .with_layout(LayoutStyle::new().with_width(200.0).with_height(30.0)),
        );
        document
            .compute_layout(UiSize::new(240.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let paint = document.paint_list();
        let fill = paint
            .items
            .iter()
            .find(|item| document.node(item.node).name == "gain.fill")
            .expect("fill paint item");
        let thumb = paint
            .items
            .iter()
            .find(|item| document.node(item.node).name == "gain.thumb")
            .expect("thumb paint item");

        assert!(thumb.layer_order > fill.layer_order);
    }

    #[test]
    fn slider_value_spec_maps_logarithmic_steps_and_clamping() {
        let spec = SliderValueSpec::new(1.0, 10_000.0)
            .logarithmic(true)
            .clamping(SliderClamping::Always);

        assert!((spec.value_at_unit(0.5) - 100.0).abs() < 0.01);
        assert!((spec.normalize(100.0) - 0.5).abs() < 0.001);

        let stepped = SliderValueSpec::new(0.0, 100.0)
            .step(5.0)
            .clamping(SliderClamping::Always);
        assert_eq!(stepped.adjust_value(42.0), 40.0);
        assert_eq!(stepped.adjust_value(102.0), 100.0);
    }
}
