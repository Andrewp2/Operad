//! Toggle control widget implementation.

use crate::widgets::{button, ButtonOptions};
use crate::{
    AccessibilityAction, AccessibilityMeta, AccessibilityRole, ColorRgba, EditPhase, LayoutStyle,
    StrokeStyle, TextStyle, UiDocument, UiNode, UiNodeId, UiVisual, WidgetActionBinding,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToggleValue {
    Off,
    On,
    Mixed,
}

impl ToggleValue {
    pub const fn from_bool(value: bool) -> Self {
        if value {
            Self::On
        } else {
            Self::Off
        }
    }

    pub const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }

    pub const fn is_mixed(self) -> bool {
        matches!(self, Self::Mixed)
    }

    pub const fn toggled(self) -> Self {
        match self {
            Self::Off | Self::Mixed => Self::On,
            Self::On => Self::Off,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::On => "on",
            Self::Mixed => "mixed",
        }
    }
}

impl From<bool> for ToggleValue {
    fn from(value: bool) -> Self {
        Self::from_bool(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToggleControlRole {
    Checkbox,
    Switch,
    ToggleButton,
}

impl ToggleControlRole {
    pub const fn accessibility_role(self) -> AccessibilityRole {
        match self {
            Self::Checkbox => AccessibilityRole::Checkbox,
            Self::Switch => AccessibilityRole::Switch,
            Self::ToggleButton => AccessibilityRole::ToggleButton,
        }
    }
}

impl Default for ToggleControlRole {
    fn default() -> Self {
        Self::Switch
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleControlState {
    pub value: ToggleValue,
    pub enabled: bool,
    pub phase: EditPhase,
}

impl ToggleControlState {
    pub const fn new(value: bool) -> Self {
        Self {
            value: ToggleValue::from_bool(value),
            enabled: true,
            phase: EditPhase::Preview,
        }
    }

    pub const fn mixed() -> Self {
        Self {
            value: ToggleValue::Mixed,
            enabled: true,
            phase: EditPhase::Preview,
        }
    }

    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn set_value(&mut self, value: ToggleValue, phase: EditPhase) -> ToggleControlOutcome {
        let previous = self.value;
        self.value = value;
        self.phase = phase;
        self.outcome(previous)
    }

    pub fn set_checked(&mut self, checked: bool, phase: EditPhase) -> ToggleControlOutcome {
        self.set_value(ToggleValue::from_bool(checked), phase)
    }

    pub fn toggle(&mut self) -> ToggleControlOutcome {
        if self.enabled {
            self.set_value(self.value.toggled(), EditPhase::UpdateEdit)
        } else {
            self.phase = EditPhase::Preview;
            self.outcome(self.value)
        }
    }

    pub fn commit(&mut self) -> ToggleControlOutcome {
        self.set_value(self.value, EditPhase::CommitEdit)
    }

    pub fn cancel_to(&mut self, value: ToggleValue) -> ToggleControlOutcome {
        self.set_value(value, EditPhase::CancelEdit)
    }

    pub fn accessibility_meta(
        &self,
        label: impl Into<String>,
        role: ToggleControlRole,
    ) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(role.accessibility_role())
            .label(label)
            .value(self.value.label())
            .action(AccessibilityAction::new("toggle", "Toggle"));
        match (role, self.value) {
            (ToggleControlRole::ToggleButton, ToggleValue::On) => {
                meta = meta.pressed(true);
            }
            (ToggleControlRole::ToggleButton, ToggleValue::Off) => {
                meta = meta.pressed(false);
            }
            (_, ToggleValue::Mixed) => {
                meta = meta.mixed();
            }
            (_, value) => {
                meta = meta.checked(value.is_on());
            }
        }
        if self.enabled {
            meta.focusable()
        } else {
            meta.disabled()
        }
    }

    fn outcome(&self, previous: ToggleValue) -> ToggleControlOutcome {
        ToggleControlOutcome {
            previous,
            value: self.value,
            phase: self.phase,
            changed: previous != self.value,
        }
    }
}

impl Default for ToggleControlState {
    fn default() -> Self {
        Self::new(false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToggleControlOutcome {
    pub previous: ToggleValue,
    pub value: ToggleValue,
    pub phase: EditPhase,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedControlItem {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
}

impl SegmentedControlItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            enabled: true,
            action: None,
            accessibility_label: None,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct SegmentedControlOptions {
    pub layout: LayoutStyle,
    pub item_layout: LayoutStyle,
    pub visual: UiVisual,
    pub selected_visual: UiVisual,
    pub hovered_visual: Option<UiVisual>,
    pub pressed_visual: Option<UiVisual>,
    pub disabled_visual: Option<UiVisual>,
    pub text_style: TextStyle,
    pub action_prefix: Option<String>,
    pub accessibility_label: Option<String>,
}

impl Default for SegmentedControlOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height(30.0)
                .with_gap(4.0),
            item_layout: LayoutStyle::new()
                .with_width(78.0)
                .with_height(28.0)
                .with_flex_shrink(0.0),
            visual: UiVisual::panel(
                ColorRgba::new(38, 46, 58, 255),
                Some(StrokeStyle::new(ColorRgba::new(74, 85, 104, 255), 1.0)),
                4.0,
            ),
            selected_visual: UiVisual::panel(
                ColorRgba::new(48, 112, 184, 255),
                Some(StrokeStyle::new(ColorRgba::new(120, 170, 230, 255), 1.0)),
                4.0,
            ),
            hovered_visual: Some(UiVisual::panel(
                ColorRgba::new(65, 86, 106, 255),
                Some(StrokeStyle::new(ColorRgba::new(120, 146, 174, 255), 1.0)),
                4.0,
            )),
            pressed_visual: Some(UiVisual::panel(
                ColorRgba::new(34, 54, 84, 255),
                Some(StrokeStyle::new(ColorRgba::new(88, 124, 164, 255), 1.0)),
                4.0,
            )),
            disabled_visual: Some(UiVisual::panel(
                ColorRgba::new(30, 34, 40, 180),
                Some(StrokeStyle::new(ColorRgba::new(64, 72, 84, 180), 1.0)),
                4.0,
            )),
            text_style: TextStyle {
                font_size: 12.0,
                line_height: 16.0,
                color: ColorRgba::new(238, 244, 252, 255),
                ..Default::default()
            },
            action_prefix: None,
            accessibility_label: None,
        }
    }
}

impl SegmentedControlOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_item_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.item_layout = layout.into();
        self
    }

    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedControlNodes {
    pub root: UiNodeId,
    pub items: Vec<UiNodeId>,
}

pub fn segmented_control(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    items: &[SegmentedControlItem],
    selected_id: Option<&str>,
    options: SegmentedControlOptions,
) -> SegmentedControlNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(name.clone(), options.layout).with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group)
                .label(options.accessibility_label.unwrap_or_else(|| name.clone()))
                .value(format!("{} choices", items.len())),
        ),
    );
    let mut nodes = Vec::with_capacity(items.len());
    for item in items {
        let selected = selected_id == Some(item.id.as_str());
        let mut button_options = ButtonOptions::new(options.item_layout.clone()).pressed(selected);
        button_options.visual = if selected {
            options.selected_visual
        } else {
            options.visual
        };
        button_options.hovered_visual = options.hovered_visual;
        button_options.pressed_visual = options.pressed_visual;
        button_options.pressed_hovered_visual = options.pressed_visual;
        button_options.disabled_visual = options.disabled_visual;
        button_options.text_style = options.text_style.clone();
        button_options.enabled = item.enabled;
        button_options.accessibility_label = item
            .accessibility_label
            .clone()
            .or_else(|| Some(item.label.clone()));
        if let Some(action) = item.action.clone().or_else(|| {
            options
                .action_prefix
                .as_ref()
                .map(|prefix| WidgetActionBinding::action(format!("{prefix}.{}", item.id)))
        }) {
            button_options.action = Some(action);
        }
        let node = button(
            document,
            root,
            format!("{name}.{}", item.id),
            item.label.clone(),
            button_options,
        );
        if let Some(accessibility) = document.node_mut(node).accessibility_mut() {
            accessibility.role = AccessibilityRole::RadioButton;
            accessibility.selected = Some(selected);
        }
        nodes.push(node);
    }
    SegmentedControlNodes { root, items: nodes }
}
