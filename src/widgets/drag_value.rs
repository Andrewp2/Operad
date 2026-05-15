use super::*;

const TAU: f64 = std::f64::consts::TAU;

#[derive(Debug, Clone)]
pub struct DragValueOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub hovered_visual: Option<UiVisual>,
    pub pressed_visual: Option<UiVisual>,
    pub disabled_visual: Option<UiVisual>,
    pub text_style: TextStyle,
    pub precision: NumericPrecision,
    pub range: Option<NumericRange>,
    pub unit: NumericUnitFormat,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for DragValueOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                size: TaffySize {
                    width: length(92.0),
                    height: length(30.0),
                },
                padding: taffy::prelude::Rect {
                    left: taffy::prelude::LengthPercentage::length(8.0),
                    right: taffy::prelude::LengthPercentage::length(8.0),
                    top: taffy::prelude::LengthPercentage::length(4.0),
                    bottom: taffy::prelude::LengthPercentage::length(4.0),
                },
                ..Default::default()
            }),
            visual: UiVisual::panel(
                ColorRgba::new(36, 42, 52, 255),
                Some(StrokeStyle::new(ColorRgba::new(74, 85, 104, 255), 1.0)),
                4.0,
            ),
            hovered_visual: Some(UiVisual::panel(
                ColorRgba::new(48, 58, 72, 255),
                Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.0)),
                4.0,
            )),
            pressed_visual: Some(UiVisual::panel(
                ColorRgba::new(28, 36, 48, 255),
                Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.0)),
                4.0,
            )),
            disabled_visual: Some(UiVisual::panel(
                ColorRgba::new(30, 34, 40, 180),
                Some(StrokeStyle::new(ColorRgba::new(64, 72, 84, 180), 1.0)),
                4.0,
            )),
            text_style: TextStyle::default(),
            precision: NumericPrecision::default(),
            range: None,
            unit: NumericUnitFormat::default(),
            enabled: true,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl DragValueOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_precision(mut self, precision: NumericPrecision) -> Self {
        self.precision = precision;
        self
    }

    pub fn with_range(mut self, range: NumericRange) -> Self {
        self.range = Some(range);
        self
    }

    pub fn with_unit(mut self, unit: NumericUnitFormat) -> Self {
        self.unit = unit;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }
}

pub fn drag_value_input(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    value: f64,
    options: DragValueOptions,
) -> UiNodeId {
    let name = name.into();
    let value = options.range.map_or(value, |range| range.clamp(value));
    let text = options.unit.format(options.precision.format(value));
    let label_text = options
        .accessibility_label
        .clone()
        .unwrap_or_else(|| name.clone());
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::SpinButton)
        .label(label_text)
        .value(text.clone())
        .focusable()
        .action(AccessibilityAction::new("adjust", "Adjust value"));
    if let Some(range) = options.range {
        accessibility =
            accessibility.value_range(AccessibilityValueRange::new(range.min, range.max));
    }
    if let Some(hint) = options.accessibility_hint.clone() {
        accessibility = accessibility.hint(hint);
    }
    if !options.enabled {
        accessibility = accessibility.disabled();
    }

    let interaction_visuals = InteractionVisuals::new(options.visual)
        .hovered(options.hovered_visual.unwrap_or(options.visual))
        .pressed(options.pressed_visual.unwrap_or(options.visual))
        .disabled(options.disabled_visual.unwrap_or(options.visual));
    let mut root_node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_input(if options.enabled {
        InputBehavior::BUTTON
    } else {
        InputBehavior::NONE
    })
    .with_interaction_visuals(interaction_visuals)
    .with_action_mode(WidgetActionMode::PointerEdit)
    .with_accessibility(accessibility);
    if let Some(action) = options.action.clone() {
        root_node = root_node.with_action(action);
    }
    let root = document.add_child(parent, root_node);
    label(
        document,
        root,
        format!("{name}.value"),
        text,
        options.text_style,
        LayoutStyle::new(),
    );
    root
}

pub fn drag_angle(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    radians: f64,
    options: DragValueOptions,
) -> UiNodeId {
    drag_value_input(
        document,
        parent,
        name,
        radians.to_degrees(),
        angle_drag_options(options, NumericRange::new(0.0, 360.0), " deg", 1),
    )
}

pub fn drag_angle_tau(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    radians: f64,
    options: DragValueOptions,
) -> UiNodeId {
    drag_value_input(
        document,
        parent,
        name,
        radians / TAU,
        angle_drag_options(options, NumericRange::new(0.0, 1.0), " tau", 3),
    )
}

fn angle_drag_options(
    mut options: DragValueOptions,
    default_range: NumericRange,
    default_suffix: &str,
    default_decimals: u8,
) -> DragValueOptions {
    if options.range.is_none() {
        options.range = Some(default_range);
    }
    if options.unit.is_empty() {
        options.unit = NumericUnitFormat::default().suffix(default_suffix);
    }
    if options.precision == NumericPrecision::default() {
        options.precision = NumericPrecision::decimals(default_decimals);
    }
    options
}

pub fn drag_value_input_actions_from_gesture_event(
    document: &UiDocument,
    node: UiNodeId,
    options: &DragValueOptions,
    event: &GestureEvent,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    let GestureEvent::Drag(gesture) = event else {
        return queue;
    };
    if !document.node_is_descendant_or_self(node, gesture.target)
        || !action_target_enabled(document, node)
    {
        return queue;
    }
    let Some(binding) = options.action.clone() else {
        return queue;
    };
    let rect = document.node(node).layout.rect;
    queue.push(WidgetAction::pointer_edit(
        node,
        binding,
        WidgetPointerEdit::new(
            WidgetValueEditPhase::from(gesture.phase),
            gesture.current,
            UiPoint::new(gesture.current.x - rect.x, gesture.current.y - rect.y),
            rect,
        ),
    ));
    queue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_value_input_builds_spinbutton_with_formatted_value() {
        let mut document = UiDocument::new(root_style(240.0, 120.0));
        let root = document.root;
        let node = drag_value_input(
            &mut document,
            root,
            "angle",
            42.25,
            DragValueOptions::default()
                .with_precision(NumericPrecision::decimals(1))
                .with_unit(NumericUnitFormat::default().suffix(" deg"))
                .with_range(NumericRange::new(0.0, 360.0))
                .with_action("angle.drag"),
        );

        let accessibility = document.node(node).accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::SpinButton);
        assert_eq!(accessibility.value.as_deref(), Some("42.3 deg"));
        assert_eq!(
            document.node(node).action_mode,
            WidgetActionMode::PointerEdit
        );
        assert_eq!(
            document.node(node).action.as_ref(),
            Some(&WidgetActionBinding::action("angle.drag"))
        );
    }

    #[test]
    fn drag_angle_helpers_format_degrees_and_tau_fraction() {
        let mut document = UiDocument::new(root_style(240.0, 120.0));
        let root = document.root;
        let degrees = drag_angle(
            &mut document,
            root,
            "angle.degrees",
            std::f64::consts::FRAC_PI_2,
            DragValueOptions::default().with_action("angle.degrees.drag"),
        );
        let tau = drag_angle_tau(
            &mut document,
            root,
            "angle.tau",
            std::f64::consts::FRAC_PI_2,
            DragValueOptions::default().with_action("angle.tau.drag"),
        );

        assert_eq!(
            document
                .node(degrees)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("90.0 deg")
        );
        assert_eq!(
            document
                .node(tau)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("0.250 tau")
        );
    }
}
