use super::*;

#[derive(Debug, Clone)]
pub struct ButtonOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub hovered_visual: Option<UiVisual>,
    pub pressed_visual: Option<UiVisual>,
    pub pressed_hovered_visual: Option<UiVisual>,
    pub focused_visual: Option<UiVisual>,
    pub disabled_visual: Option<UiVisual>,
    pub text_style: TextStyle,
    pub leading_image: Option<ImageContent>,
    pub image_size: UiSize,
    pub image_shader: Option<ShaderEffect>,
    pub shader: Option<ShaderEffect>,
    pub animation: Option<AnimationMachine>,
    pub enabled: bool,
    pub pressed: bool,
    pub focused: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl ButtonOptions {
    pub fn new(layout: impl Into<LayoutStyle>) -> Self {
        let layout = layout.into();
        Self {
            layout,
            ..Default::default()
        }
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_command(mut self, command: impl Into<CommandId>) -> Self {
        self.action = Some(WidgetActionBinding::command(command));
        self
    }
}

impl Default for ButtonOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            visual: UiVisual::panel(
                ColorRgba::new(36, 42, 52, 255),
                Some(StrokeStyle::new(ColorRgba::new(74, 85, 104, 255), 1.0)),
                4.0,
            ),
            hovered_visual: Some(UiVisual::panel(
                ColorRgba::new(56, 70, 86, 255),
                Some(StrokeStyle::new(ColorRgba::new(138, 164, 194, 255), 1.0)),
                4.0,
            )),
            pressed_visual: Some(UiVisual::panel(
                ColorRgba::new(22, 30, 40, 255),
                Some(StrokeStyle::new(ColorRgba::new(78, 104, 134, 255), 1.0)),
                4.0,
            )),
            pressed_hovered_visual: Some(UiVisual::panel(
                ColorRgba::new(42, 58, 74, 255),
                Some(StrokeStyle::new(ColorRgba::new(150, 184, 220, 255), 1.0)),
                4.0,
            )),
            focused_visual: Some(UiVisual::panel(
                ColorRgba::new(36, 42, 52, 255),
                Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.5)),
                4.0,
            )),
            disabled_visual: Some(UiVisual::panel(
                ColorRgba::new(30, 34, 40, 180),
                Some(StrokeStyle::new(ColorRgba::new(64, 72, 84, 180), 1.0)),
                4.0,
            )),
            text_style: TextStyle::default(),
            leading_image: None,
            image_size: UiSize::new(18.0, 18.0),
            image_shader: None,
            shader: None,
            animation: None,
            enabled: true,
            pressed: false,
            focused: false,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl ButtonOptions {
    fn resolved_visual(&self) -> UiVisual {
        if !self.enabled {
            self.disabled_visual.unwrap_or(self.visual)
        } else if self.pressed {
            self.pressed_visual.unwrap_or(self.visual)
        } else if self.focused {
            self.focused_visual.unwrap_or(self.visual)
        } else {
            self.visual
        }
    }

    fn interaction_visuals(&self) -> InteractionVisuals {
        InteractionVisuals::new(self.resolved_visual())
            .hovered(self.hovered_visual.unwrap_or(self.visual))
            .pressed(self.pressed_visual.unwrap_or(self.visual))
            .pressed_hovered(
                self.pressed_hovered_visual
                    .or(self.pressed_visual)
                    .unwrap_or(self.visual),
            )
            .focused(self.focused_visual.unwrap_or(self.visual))
            .disabled(self.disabled_visual.unwrap_or(self.visual))
    }

    fn resolved_visual_for_interaction(
        &self,
        hovered: bool,
        pressed: bool,
        focused: bool,
    ) -> UiVisual {
        self.interaction_visuals()
            .resolve(self.enabled, hovered, pressed, focused)
    }
}

impl ButtonOptions {
    fn update_document_interaction_visual(&self, document: &mut UiDocument, button: UiNodeId) {
        document.set_node_interaction_visuals(button, self.interaction_visuals());
        let hovered = document.focus.hovered == Some(button);
        let pressed = document.focus.pressed == Some(button) || self.pressed;
        let focused = document.focus.focused == Some(button) || self.focused;
        let visual = self.resolved_visual_for_interaction(hovered, pressed, focused);
        document.set_node_visual(button, visual);
    }
}

pub fn button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    options: ButtonOptions,
) -> UiNodeId {
    let name = name.into();
    let label = label.into();
    let mut layout = options.layout.style.clone();
    layout.display = Display::Flex;
    layout.flex_direction = FlexDirection::Row;
    layout.align_items = Some(AlignItems::Center);
    layout.justify_content = Some(JustifyContent::Center);
    let accessibility_label = options
        .accessibility_label
        .clone()
        .unwrap_or_else(|| label.clone());
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Button)
        .label(accessibility_label)
        .pressed(options.pressed)
        .action(AccessibilityAction::new("activate", "Activate"));
    if let Some(hint) = options.accessibility_hint.clone() {
        accessibility = accessibility.hint(hint);
    }
    if options.enabled {
        accessibility = accessibility.focusable();
    } else {
        accessibility = accessibility.disabled();
    }
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_input(if options.enabled {
        InputBehavior::BUTTON
    } else {
        InputBehavior::NONE
    })
    .with_interaction_visuals(options.interaction_visuals())
    .with_accessibility(accessibility);
    if let Some(shader) = options.shader.clone() {
        node = node.with_shader(shader);
    }
    if let Some(animation) = options.animation.clone() {
        node = node.with_animation(animation);
    }
    if let Some(action) = options.action.clone() {
        node = node.with_action(action);
    }
    let button = document.add_child(parent, node);
    options.update_document_interaction_visual(document, button);
    if let Some(image) = options.leading_image {
        let mut image_node = UiNode::image(
            format!("{name}.image"),
            image,
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(options.image_size.width),
                    height: length(options.image_size.height),
                },
                margin: taffy::prelude::Rect {
                    right: taffy::prelude::LengthPercentageAuto::length(6.0),
                    ..taffy::prelude::Rect::length(0.0)
                },
                ..Default::default()
            }),
        )
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Image).label(label.clone()));
        if let Some(shader) = options.image_shader {
            image_node = image_node.with_shader(shader);
        }
        document.add_child(button, image_node);
    }
    document.add_child(
        button,
        UiNode::text(
            format!("{name}.label"),
            label,
            options.text_style,
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::auto(),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        ),
    );
    button
}

pub fn button_actions_from_input_result(
    document: &UiDocument,
    button: UiNodeId,
    options: &ButtonOptions,
    result: &UiInputResult,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    push_button_input_result_actions(&mut queue, document, button, options, result);
    queue
}

pub fn push_button_input_result_actions<'a>(
    queue: &'a mut WidgetActionQueue,
    document: &UiDocument,
    button: UiNodeId,
    options: &ButtonOptions,
    result: &UiInputResult,
) -> &'a mut WidgetActionQueue {
    let Some(clicked) = result.clicked else {
        return queue;
    };
    if !document.node_is_descendant_or_self(button, clicked)
        || !action_target_enabled(document, button)
    {
        return queue;
    }
    if let Some(binding) = options.action.clone() {
        queue.push(WidgetAction::pointer_activate(button, binding, 1));
    }
    queue
}

pub fn button_actions_from_key_event(
    document: &UiDocument,
    button: UiNodeId,
    options: &ButtonOptions,
    event: &UiInputEvent,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    push_button_key_event_actions(&mut queue, document, button, options, event);
    queue
}

pub fn push_button_key_event_actions<'a>(
    queue: &'a mut WidgetActionQueue,
    document: &UiDocument,
    button: UiNodeId,
    options: &ButtonOptions,
    event: &UiInputEvent,
) -> &'a mut WidgetActionQueue {
    let UiInputEvent::Key { key, modifiers } = event else {
        return queue;
    };
    if document.focus.focused != Some(button) || !action_target_enabled(document, button) {
        return queue;
    }
    if let Some(binding) = options.action.clone() {
        queue.push_key_activation(button, binding, *key, *modifiers);
    }
    queue
}

pub fn button_actions_from_gesture_event(
    document: &UiDocument,
    button: UiNodeId,
    options: &ButtonOptions,
    event: &GestureEvent,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    push_button_gesture_event_actions(&mut queue, document, button, options, event);
    queue
}

pub fn push_button_gesture_event_actions<'a>(
    queue: &'a mut WidgetActionQueue,
    document: &UiDocument,
    button: UiNodeId,
    options: &ButtonOptions,
    event: &GestureEvent,
) -> &'a mut WidgetActionQueue {
    let GestureEvent::Click(click) = event else {
        return queue;
    };
    if click.button != PointerButton::Primary
        || !document.node_is_descendant_or_self(button, click.target)
    {
        return queue;
    }
    if !action_target_enabled(document, button) {
        return queue;
    }
    if let Some(binding) = options.action.clone() {
        queue.push(WidgetAction::pointer_activate(button, binding, click.count));
    }
    queue
}
