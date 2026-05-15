use super::*;

#[derive(Debug, Clone)]
pub struct ToggleSwitchOptions {
    pub layout: LayoutStyle,
    pub track_visual: UiVisual,
    pub on_track_visual: UiVisual,
    pub disabled_track_visual: Option<UiVisual>,
    pub thumb_visual: UiVisual,
    pub disabled_thumb_visual: Option<UiVisual>,
    pub text_style: TextStyle,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for ToggleSwitchOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::auto(),
                    height: length(30.0),
                },
                ..Default::default()
            }),
            track_visual: UiVisual::panel(ColorRgba::new(42, 49, 58, 255), None, 11.0),
            on_track_visual: UiVisual::panel(ColorRgba::new(21, 92, 78, 255), None, 11.0),
            disabled_track_visual: Some(UiVisual::panel(
                ColorRgba::new(35, 39, 45, 160),
                None,
                11.0,
            )),
            thumb_visual: UiVisual::panel(
                ColorRgba::new(235, 240, 247, 255),
                Some(StrokeStyle::new(ColorRgba::new(79, 93, 113, 255), 1.0)),
                9.0,
            ),
            disabled_thumb_visual: Some(UiVisual::panel(
                ColorRgba::new(150, 158, 170, 180),
                Some(StrokeStyle::new(ColorRgba::new(81, 90, 104, 180), 1.0)),
                9.0,
            )),
            text_style: TextStyle::default(),
            enabled: true,
            action: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl ToggleSwitchOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }
}

pub fn toggle_switch(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label_text: impl Into<String>,
    value: ToggleValue,
    options: ToggleSwitchOptions,
) -> UiNodeId {
    let name = name.into();
    let label_text = label_text.into();
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Switch)
        .label(
            options
                .accessibility_label
                .clone()
                .unwrap_or_else(|| label_text.clone()),
        )
        .value(value.label())
        .action(AccessibilityAction::new("toggle", "Toggle"));
    accessibility = if value.is_mixed() {
        accessibility.mixed()
    } else {
        accessibility.checked(value.is_on())
    };
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

    let track_visual = if !options.enabled {
        options
            .disabled_track_visual
            .unwrap_or(options.track_visual)
    } else if value.is_on() {
        options.on_track_visual
    } else {
        options.track_visual
    };
    let thumb_visual = if options.enabled {
        options.thumb_visual
    } else {
        options
            .disabled_thumb_visual
            .unwrap_or(options.thumb_visual)
    };
    let track = document.add_child(
        root,
        UiNode::container(
            format!("{name}.track"),
            LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: length(44.0),
                    height: length(22.0),
                },
                padding: taffy::prelude::Rect::length(2.0),
                margin: taffy::prelude::Rect {
                    right: taffy::prelude::LengthPercentageAuto::length(8.0),
                    ..taffy::prelude::Rect::length(0.0)
                },
                flex_shrink: 0.0,
                ..Default::default()
            }),
        )
        .with_visual(track_visual),
    );
    if value.is_on() {
        document.add_child(
            track,
            UiNode::container(
                format!("{name}.track.before_thumb"),
                LayoutStyle::new().with_flex_grow(1.0).with_height(1.0),
            ),
        );
    }
    document.add_child(
        track,
        UiNode::container(
            format!("{name}.thumb"),
            LayoutStyle::size(18.0, 18.0).with_flex_shrink(0.0),
        )
        .with_visual(thumb_visual),
    );
    if !value.is_on() {
        document.add_child(
            track,
            UiNode::container(
                format!("{name}.track.after_thumb"),
                LayoutStyle::new().with_flex_grow(1.0).with_height(1.0),
            ),
        );
    }
    if !label_text.is_empty() {
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
        document.node_mut(root).layout_constraint =
            Some(UiNodeLayoutConstraint::InlineIntrinsicSize {
                sources: vec![label],
                min_size: UiSize::new(52.0, 30.0),
            });
    }
    root
}

pub fn toggle_switch_actions_from_input_result(
    document: &UiDocument,
    toggle: UiNodeId,
    value: ToggleValue,
    options: &ToggleSwitchOptions,
    result: &UiInputResult,
) -> WidgetActionQueue {
    let mut queue = WidgetActionQueue::new();
    if result
        .clicked
        .is_some_and(|target| document.node_is_descendant_or_self(toggle, target))
        && action_target_enabled(document, toggle)
    {
        if let Some(binding) = options.action.clone() {
            queue.select(toggle, binding, value.toggled().is_on());
        }
    }
    queue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_switch_builds_switch_with_track_thumb_and_label() {
        let mut document = UiDocument::new(root_style(240.0, 120.0));
        let root = document.root;
        let node = toggle_switch(
            &mut document,
            root,
            "autosave",
            "Autosave",
            ToggleValue::On,
            ToggleSwitchOptions::default().with_action("autosave.toggle"),
        );

        let accessibility = document.node(node).accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::Switch);
        assert_eq!(accessibility.checked, Some(AccessibilityChecked::True));
        assert_eq!(document.node(node).children.len(), 2);
        assert_eq!(
            document.node(node).action.as_ref(),
            Some(&WidgetActionBinding::action("autosave.toggle"))
        );
    }
}
