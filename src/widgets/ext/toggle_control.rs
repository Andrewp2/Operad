//! Toggle control widget implementation.

use crate::{AccessibilityAction, AccessibilityMeta, AccessibilityRole, EditPhase};

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
