use super::*;

#[derive(Debug, Clone)]
pub struct RadioButtonOptions {
    pub layout: LayoutStyle,
    pub outer_visual: UiVisual,
    pub selected_outer_visual: Option<UiVisual>,
    pub disabled_outer_visual: Option<UiVisual>,
    pub dot_color: ColorRgba,
    pub text_style: TextStyle,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for RadioButtonOptions {
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
            outer_visual: UiVisual::panel(
                ColorRgba::new(29, 35, 43, 255),
                Some(StrokeStyle::new(ColorRgba::new(98, 113, 135, 255), 1.0)),
                8.0,
            ),
            selected_outer_visual: Some(UiVisual::panel(
                ColorRgba::new(21, 58, 92, 255),
                Some(StrokeStyle::new(ColorRgba::new(108, 180, 255, 255), 1.0)),
                8.0,
            )),
            disabled_outer_visual: Some(UiVisual::panel(
                ColorRgba::new(28, 32, 38, 160),
                Some(StrokeStyle::new(ColorRgba::new(67, 75, 88, 160), 1.0)),
                8.0,
            )),
            dot_color: ColorRgba::new(108, 180, 255, 255),
            text_style: TextStyle::default(),
            enabled: true,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl RadioButtonOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }
}

pub fn radio_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    selected: bool,
    options: RadioButtonOptions,
) -> UiNodeId {
    let name = name.into();
    let label_text = label_text.into();
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::RadioButton)
        .label(
            options
                .accessibility_label
                .clone()
                .unwrap_or_else(|| label_text.clone()),
        )
        .value(if selected { "selected" } else { "not selected" })
        .checked(selected)
        .action(AccessibilityAction::new("select", "Select"));
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
    .with_accessibility(accessibility);
    if let Some(action) = options.action.clone() {
        root_node = root_node.with_action(action);
    }
    let root = document.add_child(parent, root_node);
    let outer_visual = if !options.enabled {
        options
            .disabled_outer_visual
            .unwrap_or(options.outer_visual)
    } else if selected {
        options
            .selected_outer_visual
            .unwrap_or(options.outer_visual)
    } else {
        options.outer_visual
    };
    let outer = document.add_child(
        root,
        UiNode::container(
            format!("{name}.outer"),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                size: TaffySize {
                    width: length(16.0),
                    height: length(16.0),
                },
                margin: taffy::prelude::Rect {
                    right: taffy::prelude::LengthPercentageAuto::length(8.0),
                    ..taffy::prelude::Rect::length(0.0)
                },
                flex_shrink: 0.0,
                ..Default::default()
            }),
        )
        .with_visual(outer_visual),
    );
    if selected {
        document.add_child(
            outer,
            UiNode::scene(
                format!("{name}.dot"),
                vec![ScenePrimitive::Circle {
                    center: UiPoint::new(4.0, 4.0),
                    radius: 4.0,
                    fill: options.dot_color,
                    stroke: None,
                }],
                LayoutStyle::size(8.0, 8.0),
            )
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Image).label("Selected")),
        );
    }
    let mut label_style = options.text_style;
    label_style.wrap = TextWrap::None;
    let label = label(
        document,
        root,
        format!("{name}.label"),
        label_text,
        label_style,
        LayoutStyle::new(),
    );
    document.node_mut(root).layout_constraint = Some(UiNodeLayoutConstraint::InlineIntrinsicSize {
        sources: vec![label],
        min_size: UiSize::new(24.0, 28.0),
    });
    root
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadioOption {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
}

impl RadioOption {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            enabled: true,
            action: None,
        }
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct RadioGroupOptions {
    pub layout: LayoutStyle,
    pub button_options: RadioButtonOptions,
    pub accessibility_label: Option<String>,
}

impl Default for RadioGroupOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(4.0),
                    height: taffy::prelude::LengthPercentage::length(4.0),
                },
                ..Default::default()
            }),
            button_options: RadioButtonOptions::default(),
            accessibility_label: None,
        }
    }
}

pub fn radio_group(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: &[RadioOption],
    selected_id: Option<&str>,
    group_options: RadioGroupOptions,
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: group_options.layout.style,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(
                group_options
                    .accessibility_label
                    .unwrap_or_else(|| name.clone()),
            ),
        ),
    );
    for option in options {
        let mut button_options = group_options.button_options.clone();
        button_options.enabled = option.enabled;
        button_options.action = option.action.clone();
        radio_button(
            document,
            root,
            format!("{name}.{}", option.id),
            option.label.clone(),
            selected_id == Some(option.id.as_str()),
            button_options,
        );
    }
    root
}

pub fn radio_button_actions_from_input_result(
    document: &UiDocument,
    radio: UiNodeId,
    options: &RadioButtonOptions,
    result: &UiInputResult,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    if result
        .clicked
        .is_some_and(|target| document.node_is_descendant_or_self(radio, target))
        && action_target_enabled(document, radio)
    {
        if let Some(binding) = options.action.clone() {
            queue.select(radio, binding, true);
        }
    }
    queue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radio_button_builds_selected_accessible_control() {
        let mut document = UiDocument::new(root_style(240.0, 120.0));
        let root = document.root;
        let node = radio_button(
            &mut document,
            root,
            "choice.compact",
            "Compact",
            true,
            RadioButtonOptions::default().with_action("choice.compact"),
        );

        let accessibility = document.node(node).accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::RadioButton);
        assert_eq!(accessibility.checked, Some(AccessibilityChecked::True));
        assert_eq!(
            document.node(node).action.as_ref(),
            Some(&WidgetActionBinding::action("choice.compact"))
        );
        assert_eq!(document.node(node).children.len(), 2);
    }

    #[test]
    fn radio_group_adds_one_button_per_option() {
        let mut document = UiDocument::new(root_style(240.0, 120.0));
        let root = document.root;
        let group = radio_group(
            &mut document,
            root,
            "density",
            &[
                RadioOption::new("compact", "Compact"),
                RadioOption::new("comfortable", "Comfortable"),
            ],
            Some("comfortable"),
            RadioGroupOptions::default(),
        );

        assert_eq!(document.node(group).children.len(), 2);
        let selected = document.node(document.node(group).children[1]);
        assert_eq!(
            selected.accessibility.as_ref().unwrap().checked,
            Some(AccessibilityChecked::True)
        );
    }
}
