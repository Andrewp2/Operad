use super::*;

#[derive(Debug, Clone)]
pub struct CheckboxOptions {
    pub layout: LayoutStyle,
    pub box_visual: UiVisual,
    pub checked_box_visual: Option<UiVisual>,
    pub disabled_box_visual: Option<UiVisual>,
    pub check_color: ColorRgba,
    pub check_image: Option<ImageContent>,
    pub check_shader: Option<ShaderEffect>,
    pub text_style: TextStyle,
    pub shader: Option<ShaderEffect>,
    pub animation: Option<AnimationMachine>,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for CheckboxOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::auto(),
                    height: length(28.0),
                },
                ..Default::default()
            }),
            box_visual: UiVisual::panel(
                ColorRgba::new(29, 35, 43, 255),
                Some(StrokeStyle::new(ColorRgba::new(98, 113, 135, 255), 1.0)),
                3.0,
            ),
            checked_box_visual: Some(UiVisual::panel(
                ColorRgba::new(21, 58, 92, 255),
                Some(StrokeStyle::new(ColorRgba::new(108, 180, 255, 255), 1.0)),
                3.0,
            )),
            disabled_box_visual: Some(UiVisual::panel(
                ColorRgba::new(28, 32, 38, 160),
                Some(StrokeStyle::new(ColorRgba::new(67, 75, 88, 160), 1.0)),
                3.0,
            )),
            check_color: ColorRgba::new(108, 180, 255, 255),
            check_image: None,
            check_shader: None,
            text_style: TextStyle::default(),
            shader: None,
            animation: None,
            enabled: true,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl CheckboxOptions {
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

pub fn checkbox(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    checked: bool,
    options: CheckboxOptions,
) -> UiNodeId {
    let name = name.into();
    let label_text = label_text.into();
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Checkbox)
        .label(
            options
                .accessibility_label
                .clone()
                .unwrap_or_else(|| label_text.clone()),
        )
        .value(if checked { "checked" } else { "unchecked" })
        .checked(checked)
        .action(AccessibilityAction::new("toggle", "Toggle"));
    if let Some(hint) = options.accessibility_hint.clone() {
        accessibility = accessibility.hint(hint);
    }
    if options.enabled {
        accessibility = accessibility.focusable();
    } else {
        accessibility = accessibility.disabled();
    }
    let root_layout = options.layout.style.clone();
    let mut root_node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: root_layout.clone(),
            clip: ClipBehavior::Clip,
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
    if let Some(action) = options.action.clone() {
        root_node = root_node.with_action(action);
    }
    let root = document.add_child(parent, root_node);
    let box_visual = if !options.enabled {
        options.disabled_box_visual.unwrap_or(options.box_visual)
    } else if checked {
        options.checked_box_visual.unwrap_or(options.box_visual)
    } else {
        options.box_visual
    };
    let box_layout = Style {
        size: TaffySize {
            width: length(16.0),
            height: length(16.0),
        },
        margin: taffy::prelude::Rect {
            left: taffy::prelude::LengthPercentageAuto::length(0.0),
            right: taffy::prelude::LengthPercentageAuto::length(8.0),
            top: taffy::prelude::LengthPercentageAuto::length(0.0),
            bottom: taffy::prelude::LengthPercentageAuto::length(0.0),
        },
        ..Default::default()
    };
    let box_node = document.add_child(
        root,
        UiNode::container(
            format!("{name}.box"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(box_layout.clone()).style,
                ..Default::default()
            },
        )
        .with_visual(box_visual),
    );
    if checked {
        if let Some(image) = options.check_image {
            let mut check_node = UiNode::image(
                format!("{name}.check"),
                image,
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(16.0),
                        height: length(16.0),
                    },
                    ..Default::default()
                }),
            );
            if let Some(shader) = options.check_shader {
                check_node = check_node.with_shader(shader);
            }
            document.add_child(box_node, check_node);
        } else {
            let mut check_node = UiNode::scene(
                format!("{name}.check"),
                vec![
                    ScenePrimitive::Line {
                        from: UiPoint::new(3.0, 8.0),
                        to: UiPoint::new(6.5, 11.5),
                        stroke: StrokeStyle::new(options.check_color, 2.0),
                    },
                    ScenePrimitive::Line {
                        from: UiPoint::new(6.5, 11.5),
                        to: UiPoint::new(13.0, 4.0),
                        stroke: StrokeStyle::new(options.check_color, 2.0),
                    },
                ],
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(16.0),
                        height: length(16.0),
                    },
                    ..Default::default()
                }),
            );
            if let Some(shader) = options.check_shader {
                check_node = check_node.with_shader(shader);
            }
            document.add_child(box_node, check_node);
        }
    }
    let label_style = single_line_text_style(options.text_style);
    let label = label(
        document,
        root,
        format!("{name}.label"),
        label_text,
        label_style,
        LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: Dimension::auto(),
                height: Dimension::auto(),
            },
            ..Default::default()
        }),
    );
    publish_inline_intrinsic_size(
        document,
        root,
        vec![label],
        inline_intrinsic_base_size(&root_layout, &[&box_layout], 2),
    );
    root
}

pub fn checkbox_actions_from_input_result(
    document: &UiDocument,
    checkbox: UiNodeId,
    checked: bool,
    options: &CheckboxOptions,
    result: &UiInputResult,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    push_checkbox_input_result_actions(&mut queue, document, checkbox, checked, options, result);
    queue
}

pub fn push_checkbox_input_result_actions<'a>(
    queue: &'a mut WidgetActionQueue,
    document: &UiDocument,
    checkbox: UiNodeId,
    checked: bool,
    options: &CheckboxOptions,
    result: &UiInputResult,
) -> &'a mut WidgetActionQueue {
    if !result
        .clicked
        .is_some_and(|target| document.node_is_descendant_or_self(checkbox, target))
        || !action_target_enabled(document, checkbox)
    {
        return queue;
    }
    if let Some(binding) = options.action.clone() {
        queue.select(checkbox, binding, !checked);
    }
    queue
}

pub fn checkbox_actions_from_key_event(
    document: &UiDocument,
    checkbox: UiNodeId,
    checked: bool,
    options: &CheckboxOptions,
    event: &UiInputEvent,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    push_checkbox_key_event_actions(&mut queue, document, checkbox, checked, options, event);
    queue
}

pub fn push_checkbox_key_event_actions<'a>(
    queue: &'a mut WidgetActionQueue,
    document: &UiDocument,
    checkbox: UiNodeId,
    checked: bool,
    options: &CheckboxOptions,
    event: &UiInputEvent,
) -> &'a mut WidgetActionQueue {
    let UiInputEvent::Key { key, modifiers } = event else {
        return queue;
    };
    if document.focus.focused != Some(checkbox) || !action_target_enabled(document, checkbox) {
        return queue;
    }
    if let Some(binding) = options.action.clone() {
        if keyboard_activation_key(*key, *modifiers) {
            queue.select(checkbox, binding, !checked);
        }
    }
    queue
}
