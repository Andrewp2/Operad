//! Editable form widget implementation.

use crate::{
    AccessibilityAction, AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole, CommandId,
    FocusDirection, ImageContent,
};

use super::{
    data::{PropertyRowStatus, PropertyValueKind},
    property_inspector::PropertyGridRow,
};

/// Renderer-neutral field kind for composed editable forms and inspectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditableFormFieldKind {
    Text,
    MultilineText,
    Number,
    Boolean,
    Select,
    Color,
    Date,
    Path,
    ReadOnly,
    Custom(String),
}

impl EditableFormFieldKind {
    pub fn label(&self) -> &str {
        match self {
            Self::Text => "text",
            Self::MultilineText => "multiline text",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Select => "select",
            Self::Color => "color",
            Self::Date => "date",
            Self::Path => "path",
            Self::ReadOnly => "read only",
            Self::Custom(label) => label.as_str(),
        }
    }

    pub const fn accessibility_role(&self) -> AccessibilityRole {
        match self {
            Self::Text | Self::MultilineText => AccessibilityRole::TextBox,
            Self::Number => AccessibilityRole::SpinButton,
            Self::Boolean => AccessibilityRole::Switch,
            Self::Select => AccessibilityRole::ComboBox,
            Self::Color | Self::Date | Self::Path | Self::ReadOnly | Self::Custom(_) => {
                AccessibilityRole::GridCell
            }
        }
    }

    pub const fn picker_backed(&self) -> bool {
        matches!(self, Self::Select | Self::Color | Self::Date | Self::Path)
    }
}

impl From<PropertyValueKind> for EditableFormFieldKind {
    fn from(value: PropertyValueKind) -> Self {
        match value {
            PropertyValueKind::Text => Self::Text,
            PropertyValueKind::Number => Self::Number,
            PropertyValueKind::Boolean => Self::Boolean,
            PropertyValueKind::Choice => Self::Select,
            PropertyValueKind::Color => Self::Color,
            PropertyValueKind::Custom => Self::Custom("custom".to_owned()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditableFormCommitMode {
    Immediate,
    OnBlur,
    Explicit,
}

impl Default for EditableFormCommitMode {
    fn default() -> Self {
        Self::Explicit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditableFormCommandKind {
    BeginEdit,
    Commit,
    Cancel,
    OpenPicker,
    Clear,
    Reset,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditableFormCommand {
    pub id: CommandId,
    pub label: String,
    pub kind: EditableFormCommandKind,
    pub enabled: bool,
}

impl EditableFormCommand {
    pub fn new(id: impl Into<CommandId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: EditableFormCommandKind::Custom,
            enabled: true,
        }
    }

    pub fn commit(id: impl Into<CommandId>) -> Self {
        Self::new(id, "Commit").kind(EditableFormCommandKind::Commit)
    }

    pub fn cancel(id: impl Into<CommandId>) -> Self {
        Self::new(id, "Cancel").kind(EditableFormCommandKind::Cancel)
    }

    pub fn open_picker(id: impl Into<CommandId>) -> Self {
        Self::new(id, "Open picker").kind(EditableFormCommandKind::OpenPicker)
    }

    pub const fn kind(mut self, kind: EditableFormCommandKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn accessibility_action(&self) -> AccessibilityAction {
        AccessibilityAction::new(self.id.as_str(), self.label.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditableFormField {
    pub id: String,
    pub label: String,
    pub value: String,
    pub kind: EditableFormFieldKind,
    pub status: PropertyRowStatus,
    pub required: bool,
    pub read_only: bool,
    pub disabled: bool,
    pub commit_mode: EditableFormCommitMode,
    pub commands: Vec<EditableFormCommand>,
    pub leading_image: Option<ImageContent>,
}

impl EditableFormField {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        value: impl Into<String>,
        kind: EditableFormFieldKind,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: value.into(),
            kind,
            status: PropertyRowStatus::default(),
            required: false,
            read_only: false,
            disabled: false,
            commit_mode: EditableFormCommitMode::Explicit,
            commands: Vec::new(),
            leading_image: None,
        }
    }

    pub fn from_property(row: &PropertyGridRow) -> Self {
        Self {
            id: row.id.clone(),
            label: row.label.clone(),
            value: row.value.clone(),
            kind: EditableFormFieldKind::from(row.value_kind),
            status: row.status.clone(),
            required: false,
            read_only: !row.editable,
            disabled: row.disabled,
            commit_mode: EditableFormCommitMode::Explicit,
            commands: Vec::new(),
            leading_image: row.leading_image.clone(),
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub const fn commit_mode(mut self, mode: EditableFormCommitMode) -> Self {
        self.commit_mode = mode;
        self
    }

    pub fn with_status(mut self, status: PropertyRowStatus) -> Self {
        self.status = status;
        self
    }

    pub fn changed(mut self) -> Self {
        self.status = self.status.changed();
        self
    }

    pub fn pending(mut self) -> Self {
        self.status = self.status.pending();
        self
    }

    pub fn invalid(mut self, reason: impl Into<String>) -> Self {
        self.status = self.status.invalid(reason);
        self
    }

    pub fn with_command(mut self, command: EditableFormCommand) -> Self {
        self.commands.push(command);
        self
    }

    pub fn with_commands(
        mut self,
        commands: impl IntoIterator<Item = EditableFormCommand>,
    ) -> Self {
        self.commands.extend(commands);
        self
    }

    pub fn with_leading_image(mut self, image: ImageContent) -> Self {
        self.leading_image = Some(image);
        self
    }

    pub fn can_focus(&self) -> bool {
        !self.disabled && !self.read_only
    }

    pub fn can_edit(&self) -> bool {
        self.can_focus() && !matches!(self.kind, EditableFormFieldKind::ReadOnly)
    }

    pub fn accessibility(&self, index: usize, count: usize, focused: bool) -> AccessibilityMeta {
        let mut value = vec![
            self.value.clone(),
            self.kind.label().to_owned(),
            format!("field {} of {}", index + 1, count),
        ];
        push_state(&mut value, "focused", focused);
        push_state(&mut value, "required", self.required);
        push_state(&mut value, "read only", self.read_only);
        push_state(&mut value, "disabled", self.disabled);
        push_property_status_value(&mut value, &self.status);

        let mut meta = AccessibilityMeta::new(self.kind.accessibility_role())
            .label(self.label.clone())
            .value(value.join("; "))
            .selected(focused);
        if self.required {
            meta = meta.required();
        }
        if self.can_focus() {
            meta = meta.focusable();
        }
        if self.read_only {
            meta = meta.read_only();
        }
        for command in self.commands.iter().filter(|command| command.enabled) {
            meta = meta.action(command.accessibility_action());
        }
        meta = apply_property_status_accessibility(meta, &self.status);
        apply_enabled(meta, !self.disabled)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditableFormState {
    pub focused_field: Option<String>,
    pub editing_field: Option<String>,
}

impl EditableFormState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn focused(mut self, field: impl Into<String>) -> Self {
        self.focused_field = Some(field.into());
        self
    }

    pub fn editing(mut self, field: impl Into<String>) -> Self {
        let field = field.into();
        self.focused_field = Some(field.clone());
        self.editing_field = Some(field);
        self
    }

    pub fn focus_field(
        &mut self,
        fields: &[EditableFormField],
        field_id: &str,
    ) -> EditableFormOutcome {
        let Some(field) = fields
            .iter()
            .find(|field| field.id == field_id && field.can_focus())
        else {
            return EditableFormOutcome::default();
        };
        self.focused_field = Some(field.id.clone());
        if self.editing_field.as_deref() != Some(field.id.as_str()) {
            self.editing_field = None;
        }
        EditableFormOutcome {
            focused_field: Some(field.id.clone()),
            ..Default::default()
        }
    }

    pub fn move_focus(
        &mut self,
        fields: &[EditableFormField],
        direction: FocusDirection,
    ) -> EditableFormOutcome {
        let Some(next) = next_form_focus_field(fields, self.focused_field.as_deref(), direction)
        else {
            return EditableFormOutcome::default();
        };
        self.focused_field = Some(next.id.clone());
        self.editing_field = None;
        EditableFormOutcome {
            focused_field: Some(next.id.clone()),
            ..Default::default()
        }
    }

    pub fn begin_edit(&mut self, fields: &[EditableFormField]) -> EditableFormOutcome {
        let Some(field) = self.focused_editable_field(fields) else {
            return EditableFormOutcome::default();
        };
        self.editing_field = Some(field.id.clone());
        EditableFormOutcome {
            began_editing: Some(field.id.clone()),
            ..Default::default()
        }
    }

    pub fn commit(&mut self) -> EditableFormOutcome {
        let committed = self.editing_field.take();
        EditableFormOutcome {
            committed_field: committed,
            ..Default::default()
        }
    }

    pub fn cancel(&mut self) -> EditableFormOutcome {
        let canceled = self.editing_field.take();
        EditableFormOutcome {
            canceled_field: canceled,
            ..Default::default()
        }
    }

    pub fn open_picker(&self, fields: &[EditableFormField]) -> EditableFormOutcome {
        let Some(field) = self.focused_editable_field(fields) else {
            return EditableFormOutcome::default();
        };
        if !field.kind.picker_backed() {
            return EditableFormOutcome::default();
        }
        EditableFormOutcome {
            opened_picker: Some(field.id.clone()),
            ..Default::default()
        }
    }

    fn focused_editable_field<'a>(
        &self,
        fields: &'a [EditableFormField],
    ) -> Option<&'a EditableFormField> {
        let focused = self.focused_field.as_deref()?;
        fields
            .iter()
            .find(|field| field.id == focused && field.can_edit())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditableFormOutcome {
    pub focused_field: Option<String>,
    pub began_editing: Option<String>,
    pub committed_field: Option<String>,
    pub canceled_field: Option<String>,
    pub opened_picker: Option<String>,
}

impl EditableFormOutcome {
    pub fn is_empty(&self) -> bool {
        self.focused_field.is_none()
            && self.began_editing.is_none()
            && self.committed_field.is_none()
            && self.canceled_field.is_none()
            && self.opened_picker.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditableFormFieldContract {
    pub id: String,
    pub label: String,
    pub value: String,
    pub kind: EditableFormFieldKind,
    pub index: usize,
    pub focused: bool,
    pub editing: bool,
    pub accessibility: AccessibilityMeta,
    pub commands: Vec<EditableFormCommand>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditableFormContract {
    pub id: String,
    pub label: String,
    pub field_count: usize,
    pub editable_count: usize,
    pub invalid_count: usize,
    pub changed_count: usize,
    pub pending_count: usize,
    pub focused_field: Option<String>,
    pub editing_field: Option<String>,
    pub accessibility: AccessibilityMeta,
    pub fields: Vec<EditableFormFieldContract>,
}

pub fn editable_form_contract(
    id: impl Into<String>,
    label: impl Into<String>,
    fields: &[EditableFormField],
    state: &EditableFormState,
) -> EditableFormContract {
    let id = id.into();
    let label = label.into();
    let editable_count = fields.iter().filter(|field| field.can_edit()).count();
    let invalid_count = fields
        .iter()
        .filter(|field| field.status.invalid.is_some() || field.status.error.is_some())
        .count();
    let changed_count = fields.iter().filter(|field| field.status.changed).count();
    let pending_count = fields.iter().filter(|field| field.status.pending).count();

    let mut value = vec![format!("{} fields", fields.len())];
    if editable_count > 0 {
        value.push(format!("{editable_count} editable"));
    }
    if invalid_count > 0 {
        value.push(format!("{invalid_count} invalid"));
    }
    if changed_count > 0 {
        value.push(format!("{changed_count} changed"));
    }
    if pending_count > 0 {
        value.push(format!("{pending_count} pending"));
    }

    let accessibility = AccessibilityMeta::new(AccessibilityRole::Group)
        .label(label.clone())
        .value(value.join("; "));
    let field_count = fields.len();
    let fields = fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let focused = state.focused_field.as_deref() == Some(field.id.as_str());
            let editing = state.editing_field.as_deref() == Some(field.id.as_str());
            EditableFormFieldContract {
                id: field.id.clone(),
                label: field.label.clone(),
                value: field.value.clone(),
                kind: field.kind.clone(),
                index,
                focused,
                editing,
                accessibility: field.accessibility(index, field_count, focused),
                commands: field.commands.clone(),
            }
        })
        .collect();

    EditableFormContract {
        id,
        label,
        field_count,
        editable_count,
        invalid_count,
        changed_count,
        pending_count,
        focused_field: state.focused_field.clone(),
        editing_field: state.editing_field.clone(),
        accessibility,
        fields,
    }
}

fn push_property_status_value(values: &mut Vec<String>, status: &PropertyRowStatus) {
    push_state(values, "changed", status.changed);
    push_state(values, "pending", status.pending);
    if status.invalid.is_some() {
        values.push("invalid".to_owned());
    }
    if status.error.is_some() {
        values.push("error".to_owned());
    }
    if status.warning.is_some() {
        values.push("warning".to_owned());
    }
    if status.help.is_some() {
        values.push("help available".to_owned());
    }
}

fn apply_property_status_accessibility(
    mut meta: AccessibilityMeta,
    status: &PropertyRowStatus,
) -> AccessibilityMeta {
    if let Some(message) = property_status_invalid_message(status) {
        meta = meta.invalid(message);
    }
    if let Some(hint) = property_status_hint(status) {
        meta = meta.hint(hint);
    }
    if status.error.is_some() {
        meta = meta.live_region(AccessibilityLiveRegion::Assertive);
    } else if status.pending {
        meta = meta.live_region(AccessibilityLiveRegion::Polite);
    }
    meta
}

fn property_status_invalid_message(status: &PropertyRowStatus) -> Option<String> {
    status.error.clone().or_else(|| status.invalid.clone())
}

fn property_status_hint(status: &PropertyRowStatus) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(error) = &status.error {
        parts.push(format!("Error: {error}"));
    }
    if let Some(invalid) = &status.invalid {
        parts.push(format!("Invalid: {invalid}"));
    }
    if let Some(warning) = &status.warning {
        parts.push(format!("Warning: {warning}"));
    }
    if let Some(help) = &status.help {
        parts.push(format!("Help: {help}"));
    }
    if status.pending {
        parts.push("Pending".to_owned());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

fn apply_enabled(meta: AccessibilityMeta, enabled: bool) -> AccessibilityMeta {
    if enabled {
        meta
    } else {
        meta.disabled()
    }
}

fn push_state(values: &mut Vec<String>, label: &str, active: bool) {
    if active {
        values.push(label.to_owned());
    }
}

fn next_form_focus_field<'a>(
    fields: &'a [EditableFormField],
    current: Option<&str>,
    direction: FocusDirection,
) -> Option<&'a EditableFormField> {
    let focusable = fields
        .iter()
        .filter(|field| field.can_focus())
        .collect::<Vec<_>>();
    if focusable.is_empty() {
        return None;
    }

    let current_index =
        current.and_then(|current| focusable.iter().position(|field| field.id == current));
    let next_index = match (direction, current_index) {
        (FocusDirection::Next, Some(index)) => (index + 1) % focusable.len(),
        (FocusDirection::Previous, Some(0)) => focusable.len() - 1,
        (FocusDirection::Previous, Some(index)) => index - 1,
        (FocusDirection::Previous, None) => focusable.len() - 1,
        (FocusDirection::Next, None) => 0,
    };
    focusable.get(next_index).copied()
}
