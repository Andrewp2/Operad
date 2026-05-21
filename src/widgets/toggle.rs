use super::*;

#[derive(Debug, Clone)]
pub struct ToggleSwitchOptions {
    pub layout: LayoutStyle,
    pub track_visual: UiVisual,
    pub on_track_visual: UiVisual,
    pub disabled_track_visual: Option<UiVisual>,
    pub thumb_visual: UiVisual,
    pub disabled_thumb_visual: Option<UiVisual>,
    pub track_size: UiSize,
    pub thumb_size: UiSize,
    pub gap: f32,
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
            track_size: UiSize::new(44.0, 22.0),
            thumb_size: UiSize::new(18.0, 18.0),
            gap: 8.0,
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

    pub fn with_track_visual(mut self, visual: UiVisual) -> Self {
        self.track_visual = visual;
        self
    }

    pub fn with_on_track_visual(mut self, visual: UiVisual) -> Self {
        self.on_track_visual = visual;
        self
    }

    pub fn with_disabled_track_visual(mut self, visual: UiVisual) -> Self {
        self.disabled_track_visual = Some(visual);
        self
    }

    pub fn with_thumb_visual(mut self, visual: UiVisual) -> Self {
        self.thumb_visual = visual;
        self
    }

    pub fn with_disabled_thumb_visual(mut self, visual: UiVisual) -> Self {
        self.disabled_thumb_visual = Some(visual);
        self
    }

    pub fn with_track_size(mut self, size: UiSize) -> Self {
        self.track_size = UiSize::new(size.width.max(1.0), size.height.max(1.0));
        self
    }

    pub fn with_thumb_size(mut self, size: UiSize) -> Self {
        self.thumb_size = UiSize::new(size.width.max(1.0), size.height.max(1.0));
        self
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    pub fn with_text_style(mut self, style: TextStyle) -> Self {
        self.text_style = style;
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
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
    let has_label = !label_text.is_empty();
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Switch)
        .label(options.accessibility_label.clone().unwrap_or_else(|| {
            if has_label {
                label_text.clone()
            } else {
                name.clone()
            }
        }))
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
    let track_layout = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        align_items: Some(AlignItems::Center),
        size: TaffySize {
            width: length(options.track_size.width),
            height: length(options.track_size.height),
        },
        padding: taffy::prelude::Rect::length(
            ((options.track_size.height - options.thumb_size.height) * 0.5).max(0.0),
        ),
        margin: taffy::prelude::Rect {
            right: taffy::prelude::LengthPercentageAuto::length(if has_label {
                options.gap
            } else {
                0.0
            }),
            ..taffy::prelude::Rect::length(0.0_f32)
        },
        flex_shrink: 0.0,
        ..Default::default()
    };
    let track = document.add_child(
        root,
        UiNode::container(
            format!("{name}.track"),
            LayoutStyle::from_taffy_style(track_layout.clone()),
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
            LayoutStyle::size(options.thumb_size.width, options.thumb_size.height)
                .with_flex_shrink(0.0),
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
    let label = if has_label {
        let label_style = single_line_text_style(options.text_style);
        Some(label(
            document,
            root,
            format!("{name}.label"),
            label_text,
            label_style,
            LayoutStyle::new(),
        ))
    } else {
        None
    };
    publish_inline_intrinsic_size(
        document,
        root,
        label.into_iter().collect::<Vec<_>>(),
        inline_intrinsic_base_size(
            &root_layout,
            &[&track_layout],
            if has_label { 2 } else { 1 },
        ),
    );
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

    #[test]
    fn toggle_switch_supports_no_label_and_custom_track_thumb_geometry() {
        let mut document = UiDocument::new(root_style(240.0, 120.0));
        let root = document.root;
        let node = toggle_switch(
            &mut document,
            root,
            "mode.no_label",
            "",
            ToggleValue::On,
            ToggleSwitchOptions::default()
                .accessibility_label("No-label switch")
                .with_track_size(UiSize::new(72.0, 24.0))
                .with_thumb_size(UiSize::new(28.0, 18.0))
                .with_gap(0.0)
                .with_on_track_visual(UiVisual::panel(ColorRgba::new(91, 65, 158, 255), None, 4.0))
                .with_thumb_visual(UiVisual::panel(
                    ColorRgba::new(255, 205, 90, 255),
                    Some(StrokeStyle::new(ColorRgba::new(255, 236, 171, 255), 1.0)),
                    3.0,
                )),
        );

        assert_eq!(document.node(node).children.len(), 1);
        assert_eq!(
            document.node(node).accessibility.as_ref().unwrap().label,
            Some("No-label switch".to_string())
        );
        assert!(document.node(node).layout_constraint.is_some());

        document
            .compute_layout(UiSize::new(240.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let track = document.node(document.node(node).children[0]);
        assert_eq!(track.layout().rect.width, 72.0);
        assert_eq!(track.layout().rect.height, 24.0);
        let thumb = document.node(node_id_by_name(&document, "mode.no_label.thumb"));
        assert_eq!(thumb.layout().rect.width, 28.0);
        assert_eq!(thumb.layout().rect.height, 18.0);
    }

    fn node_id_by_name(document: &UiDocument, name: &str) -> UiNodeId {
        document
            .nodes()
            .iter()
            .enumerate()
            .find_map(|(index, node)| (node.name() == name).then_some(UiNodeId::from_index(index)))
            .unwrap_or_else(|| panic!("missing node {name:?}"))
    }
}
