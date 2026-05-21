use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckboxState {
    Unchecked,
    Checked,
    Indeterminate,
}

impl CheckboxState {
    pub const fn from_checked(checked: bool) -> Self {
        if checked {
            Self::Checked
        } else {
            Self::Unchecked
        }
    }

    pub const fn is_checked(self) -> bool {
        matches!(self, Self::Checked)
    }

    pub const fn next(self, supports_indeterminate: bool) -> Self {
        if supports_indeterminate {
            match self {
                Self::Unchecked => Self::Checked,
                Self::Checked => Self::Indeterminate,
                Self::Indeterminate => Self::Unchecked,
            }
        } else {
            match self {
                Self::Checked => Self::Unchecked,
                Self::Unchecked | Self::Indeterminate => Self::Checked,
            }
        }
    }

    const fn accessibility_value(self) -> &'static str {
        match self {
            Self::Unchecked => "unchecked",
            Self::Checked => "checked",
            Self::Indeterminate => "mixed",
        }
    }

    const fn selection_value(self) -> Option<bool> {
        match self {
            Self::Unchecked => Some(false),
            Self::Checked => Some(true),
            Self::Indeterminate => None,
        }
    }

    const fn uses_checked_visual(self) -> bool {
        matches!(self, Self::Checked | Self::Indeterminate)
    }
}

impl From<bool> for CheckboxState {
    fn from(value: bool) -> Self {
        Self::from_checked(value)
    }
}

#[derive(Debug, Clone)]
pub struct CheckboxOptions {
    pub layout: LayoutStyle,
    pub box_visual: UiVisual,
    pub checked_box_visual: Option<UiVisual>,
    pub disabled_box_visual: Option<UiVisual>,
    pub box_size: UiSize,
    pub gap: f32,
    pub check_color: ColorRgba,
    pub check_image: Option<ImageContent>,
    pub check_shader: Option<ShaderEffect>,
    pub text_style: TextStyle,
    pub shader: Option<ShaderEffect>,
    pub animation: Option<AnimationMachine>,
    pub enabled: bool,
    pub supports_indeterminate: bool,
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
                    height: Dimension::auto(),
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
            box_size: UiSize::new(16.0, 16.0),
            gap: 8.0,
            check_color: ColorRgba::new(108, 180, 255, 255),
            check_image: None,
            check_shader: None,
            text_style: TextStyle::default(),
            shader: None,
            animation: None,
            enabled: true,
            supports_indeterminate: false,
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

    pub fn with_box_visual(mut self, visual: UiVisual) -> Self {
        self.box_visual = visual;
        self
    }

    pub fn with_checked_box_visual(mut self, visual: impl Into<Option<UiVisual>>) -> Self {
        self.checked_box_visual = visual.into();
        self
    }

    pub fn with_disabled_box_visual(mut self, visual: impl Into<Option<UiVisual>>) -> Self {
        self.disabled_box_visual = visual.into();
        self
    }

    pub const fn with_box_size(mut self, size: UiSize) -> Self {
        self.box_size = size;
        self
    }

    pub const fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub const fn with_check_color(mut self, color: ColorRgba) -> Self {
        self.check_color = color;
        self
    }

    /// Uses an image resource for the checked mark.
    ///
    /// The image is not limited to built-in icons. Use
    /// `ImageContent::from(ImageHandle::app("..."))` with a matching renderer
    /// resource update for arbitrary app-supplied PNG, JPEG, BMP, or raw pixel
    /// images.
    pub fn with_check_image(mut self, image: impl Into<Option<ImageContent>>) -> Self {
        self.check_image = image.into();
        self
    }

    pub fn with_check_shader(mut self, shader: impl Into<Option<ShaderEffect>>) -> Self {
        self.check_shader = shader.into();
        self
    }

    pub fn with_text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = text_style;
        self
    }

    pub fn with_shader(mut self, shader: impl Into<Option<ShaderEffect>>) -> Self {
        self.shader = shader.into();
        self
    }

    pub fn with_animation(mut self, animation: impl Into<Option<AnimationMachine>>) -> Self {
        self.animation = animation.into();
        self
    }

    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub const fn with_indeterminate_support(mut self, supports_indeterminate: bool) -> Self {
        self.supports_indeterminate = supports_indeterminate;
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn with_accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
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
    checkbox_with_state(
        document,
        parent,
        name,
        label_text,
        CheckboxState::from_checked(checked),
        options,
    )
}

pub fn checkbox_with_state(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    state: CheckboxState,
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
        .value(state.accessibility_value())
        .action(AccessibilityAction::new("toggle", "Toggle"));
    accessibility = match state {
        CheckboxState::Unchecked => accessibility.checked(false),
        CheckboxState::Checked => accessibility.checked(true),
        CheckboxState::Indeterminate => accessibility.mixed(),
    };
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
    } else if state.uses_checked_visual() {
        options.checked_box_visual.unwrap_or(options.box_visual)
    } else {
        options.box_visual
    };
    let box_size = checkbox_box_size(options.box_size);
    let gap = checkbox_gap(options.gap);
    let box_layout = Style {
        display: Display::Flex,
        align_items: Some(AlignItems::Center),
        justify_content: Some(JustifyContent::Center),
        size: TaffySize {
            width: length(box_size.width),
            height: length(box_size.height),
        },
        margin: taffy::prelude::Rect {
            left: taffy::prelude::LengthPercentageAuto::length(0.0),
            right: taffy::prelude::LengthPercentageAuto::length(gap),
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
    match state {
        CheckboxState::Unchecked => {}
        CheckboxState::Checked => {
            add_checkbox_check_mark(
                document,
                box_node,
                &name,
                box_size,
                options.check_color,
                options.check_image,
                options.check_shader,
            );
        }
        CheckboxState::Indeterminate => {
            add_checkbox_indeterminate_mark(
                document,
                box_node,
                &name,
                box_size,
                options.check_color,
                options.check_shader,
            );
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

fn add_checkbox_check_mark(
    document: &mut UiDocument,
    box_node: UiNodeId,
    name: &str,
    box_size: UiSize,
    check_color: ColorRgba,
    check_image: Option<ImageContent>,
    check_shader: Option<ShaderEffect>,
) {
    if let Some(image) = check_image {
        let mut check_node = UiNode::image(
            format!("{name}.check"),
            image,
            checkbox_image_mark_layout(box_size),
        );
        if let Some(shader) = check_shader {
            check_node = check_node.with_shader(shader);
        }
        document.add_child(box_node, check_node);
    } else {
        let mut check_node = UiNode::scene(
            format!("{name}.check"),
            vec![
                ScenePrimitive::Line {
                    from: UiPoint::new(box_size.width * 0.1875, box_size.height * 0.5),
                    to: UiPoint::new(box_size.width * 0.40625, box_size.height * 0.71875),
                    stroke: StrokeStyle::new(check_color, checkbox_check_stroke_width(box_size)),
                },
                ScenePrimitive::Line {
                    from: UiPoint::new(box_size.width * 0.40625, box_size.height * 0.71875),
                    to: UiPoint::new(box_size.width * 0.8125, box_size.height * 0.25),
                    stroke: StrokeStyle::new(check_color, checkbox_check_stroke_width(box_size)),
                },
            ],
            checkbox_mark_layout(box_size),
        );
        if let Some(shader) = check_shader {
            check_node = check_node.with_shader(shader);
        }
        document.add_child(box_node, check_node);
    }
}

fn add_checkbox_indeterminate_mark(
    document: &mut UiDocument,
    box_node: UiNodeId,
    name: &str,
    box_size: UiSize,
    check_color: ColorRgba,
    check_shader: Option<ShaderEffect>,
) {
    let mut mark_node = UiNode::scene(
        format!("{name}.indeterminate"),
        vec![ScenePrimitive::Line {
            from: UiPoint::new(box_size.width * 0.25, box_size.height * 0.5),
            to: UiPoint::new(box_size.width * 0.75, box_size.height * 0.5),
            stroke: StrokeStyle::new(check_color, checkbox_check_stroke_width(box_size)),
        }],
        checkbox_mark_layout(box_size),
    );
    if let Some(shader) = check_shader {
        mark_node = mark_node.with_shader(shader);
    }
    document.add_child(box_node, mark_node);
}

fn checkbox_mark_layout(box_size: UiSize) -> LayoutStyle {
    LayoutStyle::from_taffy_style(Style {
        size: TaffySize {
            width: length(box_size.width),
            height: length(box_size.height),
        },
        ..Default::default()
    })
}

fn checkbox_image_mark_layout(box_size: UiSize) -> LayoutStyle {
    let inset = (box_size.width.min(box_size.height) * 0.0625).clamp(2.0, 4.0);
    LayoutStyle::from_taffy_style(Style {
        size: TaffySize {
            width: length((box_size.width - inset * 2.0).max(1.0)),
            height: length((box_size.height - inset * 2.0).max(1.0)),
        },
        flex_shrink: 0.0,
        ..Default::default()
    })
}

fn checkbox_box_size(size: UiSize) -> UiSize {
    UiSize::new(
        finite_or(size.width, 16.0).max(1.0),
        finite_or(size.height, 16.0).max(1.0),
    )
}

fn checkbox_gap(gap: f32) -> f32 {
    finite_or(gap, 8.0).max(0.0)
}

fn checkbox_check_stroke_width(size: UiSize) -> f32 {
    (size.width.min(size.height) / 8.0).clamp(1.0, 4.0)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
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

pub fn checkbox_state_actions_from_input_result(
    document: &UiDocument,
    checkbox: UiNodeId,
    state: CheckboxState,
    options: &CheckboxOptions,
    result: &UiInputResult,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    push_checkbox_state_input_result_actions(
        &mut queue, document, checkbox, state, options, result,
    );
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
    push_checkbox_state_input_result_actions(
        queue,
        document,
        checkbox,
        CheckboxState::from_checked(checked),
        options,
        result,
    )
}

pub fn push_checkbox_state_input_result_actions<'a>(
    queue: &'a mut WidgetActionQueue,
    document: &UiDocument,
    checkbox: UiNodeId,
    state: CheckboxState,
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
        let next = state.next(options.supports_indeterminate);
        queue.push(WidgetAction::selection(
            checkbox,
            binding,
            next.selection_value(),
        ));
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

pub fn checkbox_state_actions_from_key_event(
    document: &UiDocument,
    checkbox: UiNodeId,
    state: CheckboxState,
    options: &CheckboxOptions,
    event: &UiInputEvent,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    push_checkbox_state_key_event_actions(&mut queue, document, checkbox, state, options, event);
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
    push_checkbox_state_key_event_actions(
        queue,
        document,
        checkbox,
        CheckboxState::from_checked(checked),
        options,
        event,
    )
}

pub fn push_checkbox_state_key_event_actions<'a>(
    queue: &'a mut WidgetActionQueue,
    document: &UiDocument,
    checkbox: UiNodeId,
    state: CheckboxState,
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
            let next = state.next(options.supports_indeterminate);
            queue.push(WidgetAction::selection(
                checkbox,
                binding,
                next.selection_value(),
            ));
        }
    }
    queue
}
