//! Toast widget.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, LengthPercentageAuto, Rect, Size as TaffySize,
    Style,
};

use crate::{
    length, AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole, AnimationMachine,
    ClipBehavior, ColorRgba, ImageContent, InputBehavior, InteractionVisuals, LayoutStyle,
    ShaderEffect, StrokeStyle, TextStyle, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiVisual,
    WidgetActionBinding,
};

use super::surfaces::{toast_enter_exit_animation, DEFAULT_ACCENT, DEFAULT_SURFACE_STROKE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToastId(pub(crate) u64);

impl ToastId {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastSeverity {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastAction {
    pub id: String,
    pub label: String,
}

impl ToastAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub id: ToastId,
    pub severity: ToastSeverity,
    pub title: String,
    pub body: Option<String>,
    pub timeout_seconds: Option<f32>,
    pub age_seconds: f32,
    pub actions: Vec<ToastAction>,
    pub icon: Option<ImageContent>,
    pub shader: Option<ShaderEffect>,
    pub accessibility_hint: Option<String>,
}

impl Toast {
    pub fn new(
        id: ToastId,
        severity: ToastSeverity,
        title: impl Into<String>,
        body: Option<String>,
        timeout_seconds: Option<f32>,
    ) -> Self {
        Self {
            id,
            severity,
            title: title.into(),
            body,
            timeout_seconds: timeout_seconds
                .filter(|timeout| timeout.is_finite())
                .map(|timeout| timeout.max(0.0)),
            age_seconds: 0.0,
            actions: Vec::new(),
            icon: None,
            shader: None,
            accessibility_hint: None,
        }
    }

    pub fn with_action(mut self, action: ToastAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_icon(mut self, icon: ImageContent) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn with_shader(mut self, shader: ShaderEffect) -> Self {
        self.shader = Some(shader);
        self
    }

    pub fn accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    pub fn expired(&self) -> bool {
        self.timeout_seconds
            .is_some_and(|timeout| self.age_seconds >= timeout)
    }

    pub fn remaining_seconds(&self) -> Option<f32> {
        self.timeout_seconds
            .map(|timeout| (timeout - self.age_seconds).max(0.0))
    }

    pub fn accessibility(&self) -> AccessibilityMeta {
        let mut label = format!("{}: {}", self.severity.as_str(), self.title);
        if let Some(body) = &self.body {
            if !body.is_empty() {
                label.push_str(". ");
                label.push_str(body);
            }
        }
        let hint = self.accessibility_hint.clone().unwrap_or_else(|| {
            if self.actions.is_empty() {
                "Notification".to_string()
            } else {
                format!("Notification with {} action(s)", self.actions.len())
            }
        });
        AccessibilityMeta::new(AccessibilityRole::ListItem)
            .label(label)
            .hint(hint)
            .live_region(match self.severity {
                ToastSeverity::Error | ToastSeverity::Warning => AccessibilityLiveRegion::Assertive,
                ToastSeverity::Info | ToastSeverity::Success => AccessibilityLiveRegion::Polite,
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToastStack {
    pub toasts: Vec<Toast>,
    pub max_visible: usize,
    next_id: u64,
}

impl ToastStack {
    pub fn new(max_visible: usize) -> Self {
        Self {
            toasts: Vec::new(),
            max_visible,
            next_id: 1,
        }
    }

    pub fn push(
        &mut self,
        severity: ToastSeverity,
        title: impl Into<String>,
        body: Option<String>,
        timeout_seconds: Option<f32>,
    ) -> ToastId {
        let id = ToastId::new(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.toasts
            .push(Toast::new(id, severity, title, body, timeout_seconds));
        id
    }

    pub fn push_toast(&mut self, mut toast: Toast) -> ToastId {
        if toast.id.value() == 0 {
            toast.id = ToastId::new(self.next_id);
            self.next_id = self.next_id.saturating_add(1);
        } else {
            self.next_id = self.next_id.max(toast.id.value().saturating_add(1));
        }
        let id = toast.id;
        self.toasts.push(toast);
        id
    }

    pub fn dismiss(&mut self, id: ToastId) -> Option<Toast> {
        let index = self.toasts.iter().position(|toast| toast.id == id)?;
        Some(self.toasts.remove(index))
    }

    pub fn tick(&mut self, dt_seconds: f32) {
        if !dt_seconds.is_finite() {
            return;
        }
        let dt = dt_seconds.max(0.0);
        for toast in &mut self.toasts {
            toast.age_seconds += dt;
        }
        self.toasts.retain(|toast| !toast.expired());
    }

    pub fn visible(&self) -> &[Toast] {
        let start = self.toasts.len().saturating_sub(self.max_visible);
        &self.toasts[start..]
    }
}

impl Default for ToastStack {
    fn default() -> Self {
        Self::new(4)
    }
}

#[derive(Debug, Clone)]
pub struct ToastStackOptions {
    pub layout: LayoutStyle,
    pub z_index: i16,
    pub info_visual: UiVisual,
    pub success_visual: UiVisual,
    pub warning_visual: UiVisual,
    pub error_visual: UiVisual,
    pub action_visual: UiVisual,
    pub title_style: TextStyle,
    pub body_style: TextStyle,
    pub toast_animation: Option<AnimationMachine>,
    pub show_close_button: bool,
    pub action_prefix: Option<String>,
    pub close_action_prefix: Option<String>,
    pub close_button_visual: UiVisual,
    pub close_button_hovered_visual: UiVisual,
    pub close_button_pressed_visual: UiVisual,
    pub close_button_text_style: TextStyle,
}

impl Default for ToastStackOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: Some(AlignItems::FlexEnd),
                size: TaffySize {
                    width: length(320.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            z_index: 60,
            info_visual: UiVisual::panel(
                ColorRgba::new(31, 39, 50, 245),
                Some(StrokeStyle::new(DEFAULT_SURFACE_STROKE, 1.0)),
                4.0,
            ),
            success_visual: UiVisual::panel(
                ColorRgba::new(22, 58, 44, 245),
                Some(StrokeStyle::new(ColorRgba::new(74, 160, 118, 255), 1.0)),
                4.0,
            ),
            warning_visual: UiVisual::panel(
                ColorRgba::new(70, 54, 24, 245),
                Some(StrokeStyle::new(ColorRgba::new(190, 148, 62, 255), 1.0)),
                4.0,
            ),
            error_visual: UiVisual::panel(
                ColorRgba::new(73, 31, 35, 245),
                Some(StrokeStyle::new(ColorRgba::new(205, 91, 102, 255), 1.0)),
                4.0,
            ),
            action_visual: UiVisual::panel(
                ColorRgba::new(48, 58, 72, 255),
                Some(StrokeStyle::new(DEFAULT_ACCENT, 1.0)),
                3.0,
            ),
            title_style: TextStyle {
                font_size: 14.0,
                line_height: 18.0,
                ..Default::default()
            },
            body_style: TextStyle {
                font_size: 13.0,
                line_height: 17.0,
                color: ColorRgba::new(218, 226, 238, 255),
                ..Default::default()
            },
            toast_animation: Some(toast_enter_exit_animation(true)),
            show_close_button: true,
            action_prefix: Some("toast.action".to_string()),
            close_action_prefix: Some("toast.dismiss".to_string()),
            close_button_visual: UiVisual::panel(ColorRgba::TRANSPARENT, None, 3.0),
            close_button_hovered_visual: UiVisual::panel(
                ColorRgba::new(62, 74, 90, 210),
                None,
                3.0,
            ),
            close_button_pressed_visual: UiVisual::panel(
                ColorRgba::new(42, 50, 62, 230),
                None,
                3.0,
            ),
            close_button_text_style: TextStyle {
                font_size: 13.0,
                line_height: 16.0,
                color: ColorRgba::new(220, 228, 238, 255),
                ..Default::default()
            },
        }
    }
}

pub fn toast_stack(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    stack: &ToastStack,
    options: ToastStackOptions,
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                z_index: options.z_index,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::List)
                .label("Notifications")
                .hint("Most recent notifications"),
        ),
    );

    for toast in stack.visible() {
        add_toast_node(document, root, &name, toast, &options);
    }

    root
}

fn add_toast_node(
    document: &mut UiDocument,
    parent: UiNodeId,
    stack_name: &str,
    toast: &Toast,
    options: &ToastStackOptions,
) -> UiNodeId {
    let visual = match toast.severity {
        ToastSeverity::Info => options.info_visual,
        ToastSeverity::Success => options.success_visual,
        ToastSeverity::Warning => options.warning_visual,
        ToastSeverity::Error => options.error_visual,
    };
    let mut root_node = UiNode::container(
        format!("{stack_name}.toast.{}", toast.id.value()),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                padding: Rect::length(8.0_f32),
                margin: Rect {
                    bottom: LengthPercentageAuto::length(8.0),
                    ..Rect::length(0.0_f32)
                },
                ..Default::default()
            })
            .style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_visual(visual)
    .with_accessibility(toast.accessibility());
    if let Some(shader) = &toast.shader {
        root_node = root_node.with_shader(shader.clone());
    }
    if let Some(animation) = &options.toast_animation {
        root_node = root_node.with_animation(animation.clone());
    }
    let root = document.add_child(parent, root_node);

    let header = document.add_child(
        root,
        UiNode::container(
            format!("{stack_name}.toast.{}.header", toast.id.value()),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                })
                .style,
                ..Default::default()
            },
        ),
    );
    if let Some(icon) = &toast.icon {
        document.add_child(
            header,
            UiNode::image(
                format!("{stack_name}.toast.{}.icon", toast.id.value()),
                icon.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(18.0),
                        height: length(18.0),
                    },
                    margin: Rect {
                        right: LengthPercentageAuto::length(6.0),
                        ..Rect::length(0.0_f32)
                    },
                    flex_shrink: 0.0,
                    ..Default::default()
                }),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Image)
                    .label(format!("{} notification icon", toast.severity.as_str())),
            ),
        );
    }
    document.add_child(
        header,
        UiNode::text(
            format!("{stack_name}.toast.{}.title", toast.id.value()),
            toast.title.clone(),
            options.title_style.clone(),
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                flex_grow: 1.0,
                flex_shrink: 1.0,
                ..Default::default()
            }),
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Label).label(toast.title.clone()),
        ),
    );
    if options.show_close_button {
        let close = document.add_child(
            header,
            UiNode::container(
                format!("{stack_name}.toast.{}.close", toast.id.value()),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        justify_content: Some(taffy::prelude::JustifyContent::Center),
                        size: TaffySize {
                            width: length(24.0),
                            height: length(24.0),
                        },
                        flex_shrink: 0.0,
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON)
            .with_action(
                options
                    .close_action_prefix
                    .as_deref()
                    .map(|prefix| format!("{prefix}.{}", toast.id.value()))
                    .map(WidgetActionBinding::action)
                    .unwrap_or_else(|| {
                        WidgetActionBinding::action(format!("toast.close.{}", toast.id.value()))
                    }),
            )
            .with_visual(options.close_button_visual)
            .with_interaction_visuals(
                InteractionVisuals::new(options.close_button_visual)
                    .hovered(options.close_button_hovered_visual)
                    .pressed(options.close_button_pressed_visual),
            )
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label(format!("Dismiss {}", toast.title))
                    .focusable(),
            ),
        );
        document.add_child(
            close,
            UiNode::text(
                format!("{stack_name}.toast.{}.close.label", toast.id.value()),
                "x",
                options.close_button_text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            ),
        );
    }
    if let Some(body) = &toast.body {
        document.add_child(
            root,
            UiNode::text(
                format!("{stack_name}.toast.{}.body", toast.id.value()),
                body.clone(),
                options.body_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            ),
        );
    }
    if !toast.actions.is_empty() {
        let actions = document.add_child(
            root,
            UiNode::container(
                format!("{stack_name}.toast.{}.actions", toast.id.value()),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: Dimension::auto(),
                        },
                        margin: Rect {
                            top: LengthPercentageAuto::length(6.0),
                            ..Rect::length(0.0_f32)
                        },
                        ..Default::default()
                    })
                    .style,
                    ..Default::default()
                },
            ),
        );
        for action in &toast.actions {
            let mut button_node = UiNode::container(
                format!(
                    "{stack_name}.toast.{}.action.{}",
                    toast.id.value(),
                    action.id
                ),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        size: TaffySize {
                            width: Dimension::auto(),
                            height: length(28.0),
                        },
                        padding: Rect::length(8.0_f32),
                        margin: Rect {
                            right: LengthPercentageAuto::length(6.0),
                            ..Rect::length(0.0_f32)
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(InputBehavior::BUTTON)
            .with_visual(options.action_visual)
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label(action.label.clone())
                    .hint(format!("Activate {} notification action", toast.title))
                    .focusable(),
            );
            if let Some(prefix) = &options.action_prefix {
                button_node = button_node.with_action(WidgetActionBinding::action(format!(
                    "{prefix}.{}.{}",
                    toast.id.value(),
                    action.id
                )));
            }
            let button = document.add_child(actions, button_node);
            document.add_child(
                button,
                UiNode::text(
                    format!(
                        "{stack_name}.toast.{}.action.{}.label",
                        toast.id.value(),
                        action.id
                    ),
                    action.label.clone(),
                    options.body_style.clone(),
                    LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: Dimension::auto(),
                            height: Dimension::auto(),
                        },
                        ..Default::default()
                    }),
                ),
            );
        }
    }
    root
}
