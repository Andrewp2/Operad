use super::*;

#[derive(Debug, Clone)]
pub struct ComboBoxOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub open_visual: Option<UiVisual>,
    pub disabled_visual: Option<UiVisual>,
    pub text_style: TextStyle,
    pub leading_image: Option<ImageContent>,
    pub image_size: UiSize,
    pub shader: Option<ShaderEffect>,
    pub animation: Option<AnimationMachine>,
    pub enabled: bool,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl Default for ComboBoxOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: length(180.0),
                    height: length(30.0),
                },
                padding: taffy::prelude::Rect::length(6.0_f32),
                ..Default::default()
            }),
            visual: UiVisual::panel(
                ColorRgba::new(31, 37, 46, 255),
                Some(StrokeStyle::new(ColorRgba::new(84, 98, 121, 255), 1.0)),
                4.0,
            ),
            open_visual: Some(UiVisual::panel(
                ColorRgba::new(38, 48, 62, 255),
                Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.5)),
                4.0,
            )),
            disabled_visual: Some(UiVisual::panel(
                ColorRgba::new(29, 33, 40, 170),
                Some(StrokeStyle::new(ColorRgba::new(65, 73, 87, 170), 1.0)),
                4.0,
            )),
            text_style: TextStyle::default(),
            leading_image: None,
            image_size: UiSize::new(18.0, 18.0),
            shader: None,
            animation: None,
            enabled: true,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }
}

impl ComboBoxOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }
}

pub fn combo_box(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    selected_label: impl Into<String>,
    open: bool,
    options: ComboBoxOptions,
) -> UiNodeId {
    let name = name.into();
    let selected_label = selected_label.into();
    let accessibility_label = options
        .accessibility_label
        .clone()
        .unwrap_or_else(|| name.clone());
    let accessibility_hint = options.accessibility_hint.clone();
    let root = button(
        document,
        parent,
        name.clone(),
        selected_label.clone(),
        ButtonOptions {
            layout: options.layout,
            visual: options.visual,
            hovered_visual: None,
            pressed_visual: options.open_visual,
            pressed_hovered_visual: options.open_visual,
            focused_visual: None,
            disabled_visual: options.disabled_visual,
            text_style: options.text_style,
            leading_image: options.leading_image,
            image_size: options.image_size,
            image_shader: None,
            shader: options.shader,
            animation: options.animation,
            enabled: options.enabled,
            pressed: open,
            focused: false,
            action: None,
            accessibility_label: Some(accessibility_label.clone()),
            accessibility_hint: accessibility_hint.clone(),
        },
    );
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::ComboBox)
        .label(accessibility_label)
        .value(if open {
            format!("{selected_label} (open)")
        } else {
            selected_label
        })
        .expanded(open)
        .action(if open {
            AccessibilityAction::new("close", "Close")
        } else {
            AccessibilityAction::new("open", "Open")
        });
    if let Some(hint) = accessibility_hint {
        accessibility = accessibility.hint(hint);
    }
    if options.enabled {
        accessibility = accessibility.focusable();
    } else {
        accessibility = accessibility.disabled();
    }
    document.node_mut(root).accessibility = Some(accessibility);
    if open {
        document.node_mut(root).style.z_index = 20;
    }
    root
}
