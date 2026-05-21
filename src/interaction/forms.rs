//! Form and validation lifecycle contracts.
//!
//! This module describes retained form state independently from any renderer or
//! widget toolkit. Async validation is generation-scoped so late results can be
//! observed and rejected without replacing newer field or form state.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormId(String);

impl FormId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FormId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for FormId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for FormId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldId(String);

impl FieldId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FieldId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for FieldId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for FieldId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValidationGeneration(pub(crate) u64);

impl ValidationGeneration {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationMessage {
    pub severity: ValidationSeverity,
    pub message: String,
}

impl ValidationMessage {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            message: message.into(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            message: message.into(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Info,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormPhase {
    Idle,
    Applying,
    Submitting,
    Cancelled,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldValidationRequest {
    pub form_id: FormId,
    pub field_id: FieldId,
    pub generation: ValidationGeneration,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldValidationResult {
    pub field_id: FieldId,
    pub generation: ValidationGeneration,
    pub messages: Vec<ValidationMessage>,
}

impl FieldValidationResult {
    pub fn new(
        field_id: impl Into<FieldId>,
        generation: ValidationGeneration,
        messages: Vec<ValidationMessage>,
    ) -> Self {
        Self {
            field_id: field_id.into(),
            generation,
            messages,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormValidationRequest {
    pub form_id: FormId,
    pub generation: ValidationGeneration,
    pub values: BTreeMap<FieldId, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormValidationResult {
    pub generation: ValidationGeneration,
    pub field_messages: BTreeMap<FieldId, Vec<ValidationMessage>>,
    pub form_messages: Vec<ValidationMessage>,
}

impl FormValidationResult {
    pub fn new(generation: ValidationGeneration) -> Self {
        Self {
            generation,
            field_messages: BTreeMap::new(),
            form_messages: Vec::new(),
        }
    }

    pub fn with_field_messages(
        mut self,
        field_id: impl Into<FieldId>,
        messages: Vec<ValidationMessage>,
    ) -> Self {
        self.field_messages.insert(field_id.into(), messages);
        self
    }

    pub fn with_form_message(mut self, message: ValidationMessage) -> Self {
        self.form_messages.push(message);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationApplyDisposition {
    Applied,
    Stale {
        expected: Option<ValidationGeneration>,
        received: ValidationGeneration,
    },
    UnknownField {
        field_id: FieldId,
    },
}

impl ValidationApplyDisposition {
    pub const fn applied(&self) -> bool {
        matches!(self, Self::Applied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldState {
    pub id: FieldId,
    pub value: String,
    pub baseline: String,
    pub dirty: bool,
    pub pending: bool,
    pub validating: bool,
    pub validation_generation: ValidationGeneration,
    pub pending_generation: Option<ValidationGeneration>,
    pub messages: Vec<ValidationMessage>,
}

impl FieldState {
    pub fn new(id: impl Into<FieldId>, value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            id: id.into(),
            value: value.clone(),
            baseline: value,
            dirty: false,
            pending: false,
            validating: false,
            validation_generation: ValidationGeneration::ZERO,
            pending_generation: None,
            messages: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        self.messages
            .iter()
            .any(|message| message.severity == ValidationSeverity::Error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleErrorSummaryRecord {
    pub form_id: FormId,
    pub field_id: Option<FieldId>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormState {
    pub id: FormId,
    pub fields: BTreeMap<FieldId, FieldState>,
    pub form_messages: Vec<ValidationMessage>,
    pub phase: FormPhase,
    pub dirty: bool,
    pub pending: bool,
    pub validating: bool,
    pub submitted: bool,
    pub validation_generation: ValidationGeneration,
    pub pending_generation: Option<ValidationGeneration>,
}

impl FormState {
    pub fn new(id: impl Into<FormId>) -> Self {
        Self {
            id: id.into(),
            fields: BTreeMap::new(),
            form_messages: Vec::new(),
            phase: FormPhase::Idle,
            dirty: false,
            pending: false,
            validating: false,
            submitted: false,
            validation_generation: ValidationGeneration::ZERO,
            pending_generation: None,
        }
    }

    pub fn with_field(mut self, id: impl Into<FieldId>, value: impl Into<String>) -> Self {
        self.add_field(id, value);
        self
    }

    pub fn add_field(&mut self, id: impl Into<FieldId>, value: impl Into<String>) {
        let field = FieldState::new(id, value);
        self.fields.insert(field.id.clone(), field);
        self.refresh_flags();
    }

    pub fn update_field(
        &mut self,
        id: impl Into<FieldId>,
        value: impl Into<String>,
    ) -> Option<&FieldState> {
        let id = id.into();
        let field = self.fields.get_mut(&id)?;
        field.value = value.into();
        field.dirty = field.value != field.baseline;
        field.pending = true;
        if field.validating {
            field.validation_generation = field.validation_generation.next();
            field.pending_generation = None;
            field.validating = false;
        }
        field.messages.clear();
        if self.pending_generation.is_some() {
            self.validation_generation = self.validation_generation.next();
            self.pending_generation = None;
            self.validating = false;
        }
        self.form_messages.clear();
        self.submitted = false;
        self.phase = FormPhase::Idle;
        self.refresh_flags();
        self.fields.get(&id)
    }

    pub fn apply(&mut self) {
        self.phase = FormPhase::Applying;
        for field in self.fields.values_mut() {
            field.baseline = field.value.clone();
            field.dirty = false;
            field.pending = false;
        }
        self.refresh_flags();
    }

    pub fn submit(&mut self) {
        self.phase = FormPhase::Submitting;
        self.submitted = true;
        self.pending = true;
    }

    pub fn cancel(&mut self) {
        self.phase = FormPhase::Cancelled;
        self.form_messages.clear();
        self.pending_generation = None;
        self.validating = false;
        for field in self.fields.values_mut() {
            field.value = field.baseline.clone();
            field.dirty = false;
            field.pending = false;
            field.validating = false;
            field.pending_generation = None;
            field.messages.clear();
        }
        self.refresh_flags();
    }

    pub fn reset(&mut self) {
        self.phase = FormPhase::Reset;
        self.form_messages.clear();
        self.submitted = false;
        self.validation_generation = ValidationGeneration::ZERO;
        self.pending_generation = None;
        for field in self.fields.values_mut() {
            field.value.clear();
            field.baseline.clear();
            field.dirty = false;
            field.pending = false;
            field.validating = false;
            field.validation_generation = ValidationGeneration::ZERO;
            field.pending_generation = None;
            field.messages.clear();
        }
        self.refresh_flags();
    }

    pub fn begin_field_validation(
        &mut self,
        id: impl Into<FieldId>,
    ) -> Option<FieldValidationRequest> {
        let id = id.into();
        let field = self.fields.get_mut(&id)?;
        field.validation_generation = field.validation_generation.next();
        field.pending_generation = Some(field.validation_generation);
        field.validating = true;
        let generation = field.validation_generation;
        let value = field.value.clone();
        self.refresh_flags();
        Some(FieldValidationRequest {
            form_id: self.id.clone(),
            field_id: id,
            generation,
            value,
        })
    }

    pub fn apply_field_validation(
        &mut self,
        result: FieldValidationResult,
    ) -> ValidationApplyDisposition {
        let Some(field) = self.fields.get_mut(&result.field_id) else {
            return ValidationApplyDisposition::UnknownField {
                field_id: result.field_id,
            };
        };
        if field.pending_generation != Some(result.generation) {
            return ValidationApplyDisposition::Stale {
                expected: field.pending_generation,
                received: result.generation,
            };
        }

        field.messages = result.messages;
        field.pending = false;
        field.pending_generation = None;
        field.validating = false;
        self.refresh_flags();
        ValidationApplyDisposition::Applied
    }

    pub fn begin_form_validation(&mut self) -> FormValidationRequest {
        self.validation_generation = self.validation_generation.next();
        self.pending_generation = Some(self.validation_generation);
        self.validating = true;
        self.pending = true;
        FormValidationRequest {
            form_id: self.id.clone(),
            generation: self.validation_generation,
            values: self.values(),
        }
    }

    pub fn apply_form_validation(
        &mut self,
        result: FormValidationResult,
    ) -> ValidationApplyDisposition {
        if self.pending_generation != Some(result.generation) {
            return ValidationApplyDisposition::Stale {
                expected: self.pending_generation,
                received: result.generation,
            };
        }
        if let Some(field_id) = result
            .field_messages
            .keys()
            .find(|field_id| !self.fields.contains_key(*field_id))
        {
            return ValidationApplyDisposition::UnknownField {
                field_id: field_id.clone(),
            };
        }

        for field in self.fields.values_mut() {
            field.messages.clear();
            field.pending = false;
        }
        for (field_id, messages) in result.field_messages {
            let Some(field) = self.fields.get_mut(&field_id) else {
                return ValidationApplyDisposition::UnknownField { field_id };
            };
            field.messages = messages;
        }
        self.form_messages = result.form_messages;
        self.pending_generation = None;
        self.validating = false;
        self.pending = false;
        self.refresh_flags();
        ValidationApplyDisposition::Applied
    }

    pub fn values(&self) -> BTreeMap<FieldId, String> {
        self.fields
            .iter()
            .map(|(id, field)| (id.clone(), field.value.clone()))
            .collect()
    }

    pub fn accessible_error_summary(&self) -> Vec<AccessibleErrorSummaryRecord> {
        let mut records = Vec::new();
        for message in &self.form_messages {
            if message.severity == ValidationSeverity::Error {
                records.push(AccessibleErrorSummaryRecord {
                    form_id: self.id.clone(),
                    field_id: None,
                    message: message.message.clone(),
                });
            }
        }
        for (field_id, field) in &self.fields {
            for message in &field.messages {
                if message.severity == ValidationSeverity::Error {
                    records.push(AccessibleErrorSummaryRecord {
                        form_id: self.id.clone(),
                        field_id: Some(field_id.clone()),
                        message: message.message.clone(),
                    });
                }
            }
        }
        records
    }

    fn refresh_flags(&mut self) {
        self.dirty = self.fields.values().any(|field| field.dirty);
        self.validating =
            self.pending_generation.is_some() || self.fields.values().any(|field| field.validating);
        self.pending =
            self.pending_generation.is_some() || self.fields.values().any(|field| field.pending);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forms_track_dirty_apply_cancel_and_reset() {
        let mut form = FormState::new("settings")
            .with_field("name", "Alpha")
            .with_field("mode", "Auto");

        form.update_field("name", "Beta").unwrap();
        assert!(form.dirty);
        assert!(form.pending);
        assert!(form.fields[&FieldId::from("name")].dirty);

        form.apply();
        assert_eq!(form.phase, FormPhase::Applying);
        assert!(!form.dirty);
        assert!(!form.pending);
        assert_eq!(form.fields[&FieldId::from("name")].baseline, "Beta");

        form.update_field("name", "Gamma").unwrap();
        form.cancel();
        assert_eq!(form.phase, FormPhase::Cancelled);
        assert_eq!(form.fields[&FieldId::from("name")].value, "Beta");
        assert!(!form.dirty);

        form.reset();
        assert_eq!(form.phase, FormPhase::Reset);
        assert_eq!(form.fields[&FieldId::from("name")].value, "");
        assert!(!form.submitted);
    }

    #[test]
    fn forms_reject_stale_async_field_validation_results() {
        let mut form = FormState::new("profile").with_field("email", "a@example.com");
        let first = form.begin_field_validation("email").unwrap();
        form.update_field("email", "b@example.com").unwrap();
        let second = form.begin_field_validation("email").unwrap();

        let stale = form.apply_field_validation(FieldValidationResult::new(
            "email",
            first.generation,
            vec![ValidationMessage::error("old error")],
        ));
        assert_eq!(
            stale,
            ValidationApplyDisposition::Stale {
                expected: Some(second.generation),
                received: first.generation
            }
        );
        assert!(form.fields[&FieldId::from("email")].messages.is_empty());

        let applied = form.apply_field_validation(FieldValidationResult::new(
            "email",
            second.generation,
            vec![ValidationMessage::warning("new warning")],
        ));
        assert!(applied.applied());
        let field = &form.fields[&FieldId::from("email")];
        assert!(!field.validating);
        assert_eq!(field.messages.len(), 1);
    }

    #[test]
    fn form_validation_clears_pending_while_preserving_dirty_changes() {
        let mut form = FormState::new("profile").with_field("email", "a@example.com");
        form.update_field("email", "b@example.com").unwrap();
        assert!(form.dirty);
        assert!(form.pending);

        let request = form.begin_form_validation();
        let applied = form.apply_form_validation(FormValidationResult::new(request.generation));

        assert!(applied.applied());
        assert!(form.dirty);
        assert!(!form.pending);
        assert!(!form.fields[&FieldId::from("email")].pending);
    }

    #[test]
    fn field_validation_clears_pending_while_preserving_dirty_changes() {
        let mut form = FormState::new("profile").with_field("email", "a@example.com");
        form.update_field("email", "b@example.com").unwrap();
        assert!(form.pending);

        let request = form.begin_field_validation("email").unwrap();
        let applied = form.apply_field_validation(FieldValidationResult::new(
            "email",
            request.generation,
            Vec::new(),
        ));

        assert!(applied.applied());
        assert!(form.dirty);
        assert!(!form.pending);
        assert!(!form.fields[&FieldId::from("email")].pending);
    }

    #[test]
    fn updating_field_invalidates_in_flight_form_validation() {
        let mut form = FormState::new("profile").with_field("email", "a@example.com");
        let request = form.begin_form_validation();
        form.update_field("email", "b@example.com").unwrap();

        let stale = form.apply_form_validation(FormValidationResult::new(request.generation));

        assert_eq!(
            stale,
            ValidationApplyDisposition::Stale {
                expected: None,
                received: request.generation
            }
        );
        assert!(form.form_messages.is_empty());
        assert!(form.fields[&FieldId::from("email")].messages.is_empty());
    }

    #[test]
    fn forms_reject_stale_async_form_validation_results() {
        let mut form = FormState::new("profile").with_field("email", "a@example.com");
        let first = form.begin_form_validation();
        let second = form.begin_form_validation();

        let stale = form.apply_form_validation(
            FormValidationResult::new(first.generation)
                .with_form_message(ValidationMessage::error("old form error")),
        );
        assert_eq!(
            stale,
            ValidationApplyDisposition::Stale {
                expected: Some(second.generation),
                received: first.generation
            }
        );
        assert!(form.form_messages.is_empty());

        let applied = form.apply_form_validation(
            FormValidationResult::new(second.generation)
                .with_field_messages("email", vec![ValidationMessage::error("required")]),
        );
        assert!(applied.applied());
        assert!(!form.validating);
        assert!(form.fields[&FieldId::from("email")].has_errors());
    }

    #[test]
    fn forms_generate_accessible_error_summary_for_form_and_fields() {
        let mut form = FormState::new("profile")
            .with_field("email", "")
            .with_field("name", "");
        let request = form.begin_form_validation();
        form.apply_form_validation(
            FormValidationResult::new(request.generation)
                .with_form_message(ValidationMessage::error("Fix the highlighted fields"))
                .with_field_messages(
                    "email",
                    vec![
                        ValidationMessage::error("Email is required"),
                        ValidationMessage::info("Used for receipts"),
                    ],
                )
                .with_field_messages("name", vec![ValidationMessage::warning("Short name")]),
        );

        let summary = form.accessible_error_summary();
        assert_eq!(summary.len(), 2);
        assert_eq!(summary[0].field_id, None);
        assert_eq!(summary[1].field_id, Some(FieldId::from("email")));
        assert_eq!(summary[1].message, "Email is required");
    }
}
