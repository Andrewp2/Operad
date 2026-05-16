use super::*;

#[derive(Debug, Clone)]
pub struct FormSectionOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub title_style: TextStyle,
    pub accessibility_label: Option<String>,
}

impl Default for FormSectionOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column().with_padding(12.0).with_gap(10.0),
            visual: UiVisual::panel(
                ColorRgba::new(24, 29, 36, 255),
                Some(StrokeStyle::new(ColorRgba::new(70, 82, 101, 255), 1.0)),
                4.0,
            ),
            title_style: strong_text_style(),
            accessibility_label: None,
        }
    }
}

impl FormSectionOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormSectionNodes {
    pub root: UiNodeId,
    pub title: Option<UiNodeId>,
}

#[derive(Debug, Clone)]
pub struct FormRowOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub required: bool,
    pub invalid: Option<String>,
    pub accessibility_label: Option<String>,
}

impl Default for FormRowOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column().with_gap(6.0),
            visual: UiVisual::TRANSPARENT,
            required: false,
            invalid: None,
            accessibility_label: None,
        }
    }
}

impl FormRowOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn invalid(mut self, reason: impl Into<String>) -> Self {
        self.invalid = Some(reason.into());
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct FieldLabelOptions {
    pub layout: LayoutStyle,
    pub text_style: TextStyle,
    pub required: bool,
    pub accessibility_label: Option<String>,
}

impl Default for FieldLabelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::row().with_align_items(AlignItems::Center),
            text_style: strong_text_style(),
            required: false,
            accessibility_label: None,
        }
    }
}

impl FieldLabelOptions {
    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct FieldHelpOptions {
    pub layout: LayoutStyle,
    pub text_style: TextStyle,
}

impl Default for FieldHelpOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::new(),
            text_style: TextStyle {
                color: ColorRgba::new(166, 178, 196, 255),
                font_size: 13.0,
                line_height: 17.0,
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ValidationMessageOptions {
    pub layout: LayoutStyle,
    pub text_style: Option<TextStyle>,
}

#[derive(Debug, Clone)]
pub struct FormErrorSummaryOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub title: String,
    pub title_style: TextStyle,
    pub message_style: TextStyle,
}

impl Default for FormErrorSummaryOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column().with_padding(10.0).with_gap(6.0),
            visual: UiVisual::panel(
                ColorRgba::new(48, 25, 28, 255),
                Some(StrokeStyle::new(ColorRgba::new(194, 96, 105, 255), 1.0)),
                4.0,
            ),
            title: "Fix the highlighted fields".to_string(),
            title_style: strong_text_style(),
            message_style: validation_text_style(ValidationSeverity::Error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormErrorSummaryNodes {
    pub root: UiNodeId,
    pub title: UiNodeId,
    pub messages: Vec<UiNodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormActionKind {
    Submit,
    Apply,
    Cancel,
    Reset,
}

impl FormActionKind {
    pub const ALL: [Self; 4] = [Self::Submit, Self::Apply, Self::Cancel, Self::Reset];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Submit => "submit",
            Self::Apply => "apply",
            Self::Cancel => "cancel",
            Self::Reset => "reset",
        }
    }

    pub const fn default_label(self) -> &'static str {
        match self {
            Self::Submit => "Submit",
            Self::Apply => "Apply",
            Self::Cancel => "Cancel",
            Self::Reset => "Reset",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormActionLabels {
    pub submit: String,
    pub apply: String,
    pub cancel: String,
    pub reset: String,
}

impl FormActionLabels {
    pub fn label(&self, action: FormActionKind) -> &str {
        match action {
            FormActionKind::Submit => &self.submit,
            FormActionKind::Apply => &self.apply,
            FormActionKind::Cancel => &self.cancel,
            FormActionKind::Reset => &self.reset,
        }
    }
}

impl Default for FormActionLabels {
    fn default() -> Self {
        Self {
            submit: FormActionKind::Submit.default_label().to_owned(),
            apply: FormActionKind::Apply.default_label().to_owned(),
            cancel: FormActionKind::Cancel.default_label().to_owned(),
            reset: FormActionKind::Reset.default_label().to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormActionAvailability {
    pub submit: bool,
    pub apply: bool,
    pub cancel: bool,
    pub reset: bool,
}

impl FormActionAvailability {
    pub fn from_form(form: &FormState) -> Self {
        let has_errors = form_has_errors(form);
        let busy = form.validating || form.pending;
        Self {
            submit: !has_errors && !busy,
            apply: form.dirty && !has_errors && !busy,
            cancel: form.dirty || form.pending || form.validating || form.submitted,
            reset: !form.fields.is_empty()
                && form
                    .fields
                    .values()
                    .any(|field| !field.value.is_empty() || field.dirty || field.pending),
        }
    }

    pub const fn is_available(self, action: FormActionKind) -> bool {
        match action {
            FormActionKind::Submit => self.submit,
            FormActionKind::Apply => self.apply,
            FormActionKind::Cancel => self.cancel,
            FormActionKind::Reset => self.reset,
        }
    }

    pub const fn enabled(self, enabled: bool) -> Self {
        if enabled {
            self
        } else {
            Self {
                submit: false,
                apply: false,
                cancel: false,
                reset: false,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormActionButtonsOptions {
    pub layout: LayoutStyle,
    pub button_options: ButtonOptions,
    pub labels: FormActionLabels,
    pub include_reset: bool,
    pub enabled: bool,
    pub action_prefix: Option<String>,
    pub accessibility_label: Option<String>,
}

impl FormActionButtonsOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_button_options(mut self, options: ButtonOptions) -> Self {
        self.button_options = options;
        self
    }

    pub fn with_labels(mut self, labels: FormActionLabels) -> Self {
        self.labels = labels;
        self
    }

    pub const fn include_reset(mut self, include_reset: bool) -> Self {
        self.include_reset = include_reset;
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }

    pub fn without_actions(mut self) -> Self {
        self.action_prefix = None;
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    fn action_for(&self, action: FormActionKind) -> Option<WidgetActionBinding> {
        self.action_prefix
            .as_ref()
            .map(|prefix| WidgetActionBinding::action(format!("{prefix}.{}", action.as_str())))
    }
}

impl Default for FormActionButtonsOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::FlexEnd),
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(8.0),
                    height: taffy::prelude::LengthPercentage::length(8.0),
                },
                ..Default::default()
            }),
            button_options: ButtonOptions {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    align_items: Some(AlignItems::Center),
                    justify_content: Some(JustifyContent::Center),
                    size: TaffySize {
                        width: length(88.0),
                        height: length(32.0),
                    },
                    padding: taffy::prelude::Rect::length(8.0),
                    ..Default::default()
                }),
                ..Default::default()
            },
            labels: FormActionLabels::default(),
            include_reset: false,
            enabled: true,
            action_prefix: Some("form".to_owned()),
            accessibility_label: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormActionButtonNodes {
    pub root: UiNodeId,
    pub submit: UiNodeId,
    pub apply: UiNodeId,
    pub cancel: UiNodeId,
    pub reset: Option<UiNodeId>,
}

impl FormActionButtonNodes {
    pub fn node_for(self, action: FormActionKind) -> Option<UiNodeId> {
        match action {
            FormActionKind::Submit => Some(self.submit),
            FormActionKind::Apply => Some(self.apply),
            FormActionKind::Cancel => Some(self.cancel),
            FormActionKind::Reset => self.reset,
        }
    }
}

pub fn form_action_buttons(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    form: &FormState,
    options: FormActionButtonsOptions,
) -> FormActionButtonNodes {
    let name = name.into();
    let availability = FormActionAvailability::from_form(form).enabled(options.enabled);
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(
                options
                    .accessibility_label
                    .clone()
                    .unwrap_or_else(|| "Form actions".to_owned()),
            ),
        ),
    );
    let submit = form_action_button(
        document,
        root,
        &name,
        FormActionKind::Submit,
        availability,
        &options,
    );
    let apply = form_action_button(
        document,
        root,
        &name,
        FormActionKind::Apply,
        availability,
        &options,
    );
    let cancel = form_action_button(
        document,
        root,
        &name,
        FormActionKind::Cancel,
        availability,
        &options,
    );
    let reset = options.include_reset.then(|| {
        form_action_button(
            document,
            root,
            &name,
            FormActionKind::Reset,
            availability,
            &options,
        )
    });
    FormActionButtonNodes {
        root,
        submit,
        apply,
        cancel,
        reset,
    }
}

fn form_action_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    action: FormActionKind,
    availability: FormActionAvailability,
    options: &FormActionButtonsOptions,
) -> UiNodeId {
    let mut button_options = options.button_options.clone();
    button_options.enabled = availability.is_available(action);
    button_options.action = options.action_for(action);
    button_options.accessibility_label = Some(options.labels.label(action).to_owned());
    button(
        document,
        parent,
        format!("{name}.{}", action.as_str()),
        options.labels.label(action).to_owned(),
        button_options,
    )
}

pub fn form_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    title: impl Into<Option<String>>,
    options: FormSectionOptions,
) -> FormSectionNodes {
    let name = name.into();
    let title = title.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(
                options
                    .accessibility_label
                    .clone()
                    .or_else(|| title.clone())
                    .unwrap_or_else(|| name.clone()),
            ),
        ),
    );
    let title = title.map(|title| {
        document.add_child(
            root,
            UiNode::text(
                format!("{name}.title"),
                title.clone(),
                options.title_style,
                LayoutStyle::new(),
            )
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label(title)),
        )
    });
    FormSectionNodes { root, title }
}

pub fn form_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: FormRowOptions,
) -> UiNodeId {
    let name = name.into();
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Group).label(
        options
            .accessibility_label
            .clone()
            .unwrap_or_else(|| name.clone()),
    );
    if options.required {
        accessibility = accessibility.required();
    }
    if let Some(reason) = options.invalid.clone() {
        accessibility = accessibility.invalid(reason);
    }
    document.add_child(
        parent,
        UiNode::container(
            name,
            UiNodeStyle {
                layout: options.layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(accessibility),
    )
}

pub fn field_label(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    options: FieldLabelOptions,
) -> UiNodeId {
    let name = name.into();
    let text = text.into();
    let rendered = if options.required {
        format!("{text} *")
    } else {
        text.clone()
    };
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::Label).label(
        options
            .accessibility_label
            .clone()
            .unwrap_or_else(|| text.clone()),
    );
    if options.required {
        accessibility = accessibility.required();
    }
    document.add_child(
        parent,
        UiNode::text(name, rendered, options.text_style, options.layout)
            .with_accessibility(accessibility),
    )
}

pub fn field_help_text(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    options: FieldHelpOptions,
) -> UiNodeId {
    let text = text.into();
    document.add_child(
        parent,
        UiNode::text(name, text.clone(), options.text_style, options.layout).with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Status)
                .label("Help")
                .value(text)
                .live_region(AccessibilityLiveRegion::Polite),
        ),
    )
}

pub fn field_validation_message(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    message: ValidationMessage,
    options: ValidationMessageOptions,
) -> UiNodeId {
    let role = match message.severity {
        ValidationSeverity::Error => AccessibilityRole::Alert,
        ValidationSeverity::Warning | ValidationSeverity::Info => AccessibilityRole::Status,
    };
    let live_region = match message.severity {
        ValidationSeverity::Error => AccessibilityLiveRegion::Assertive,
        ValidationSeverity::Warning | ValidationSeverity::Info => AccessibilityLiveRegion::Polite,
    };
    let mut accessibility = AccessibilityMeta::new(role)
        .label(validation_severity_label(message.severity))
        .value(message.message.clone())
        .live_region(live_region);
    if message.severity == ValidationSeverity::Error {
        accessibility = accessibility.invalid(message.message.clone());
    }
    document.add_child(
        parent,
        UiNode::text(
            name,
            message.message.clone(),
            options
                .text_style
                .unwrap_or_else(|| validation_text_style(message.severity)),
            options.layout,
        )
        .with_accessibility(accessibility),
    )
}

pub fn form_error_summary(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    form: &FormState,
    options: FormErrorSummaryOptions,
) -> Option<FormErrorSummaryNodes> {
    let records = form.accessible_error_summary();
    if records.is_empty() {
        return None;
    }
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Alert)
                .label(options.title.clone())
                .value(format!("{} errors", records.len()))
                .live_region(AccessibilityLiveRegion::Assertive),
        ),
    );
    let title = document.add_child(
        root,
        UiNode::text(
            format!("{name}.title"),
            options.title.clone(),
            options.title_style,
            LayoutStyle::new(),
        )
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label(options.title)),
    );
    let mut messages = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        let label = record
            .field_id
            .as_ref()
            .map(|field_id| format!("{}: {}", field_id.as_str(), record.message))
            .unwrap_or_else(|| record.message.clone());
        messages.push(
            document.add_child(
                root,
                UiNode::text(
                    format!("{name}.message.{index}"),
                    label.clone(),
                    options.message_style.clone(),
                    LayoutStyle::new(),
                )
                .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Alert).label(label)),
            ),
        );
    }
    Some(FormErrorSummaryNodes {
        root,
        title,
        messages,
    })
}

pub fn validation_text_style(severity: ValidationSeverity) -> TextStyle {
    TextStyle {
        color: match severity {
            ValidationSeverity::Info => ColorRgba::new(126, 183, 255, 255),
            ValidationSeverity::Warning => ColorRgba::new(239, 199, 93, 255),
            ValidationSeverity::Error => ColorRgba::new(255, 135, 142, 255),
        },
        font_size: 13.0,
        line_height: 17.0,
        ..Default::default()
    }
}

pub fn form_has_errors(form: &FormState) -> bool {
    form.form_messages
        .iter()
        .any(|message| message.severity == ValidationSeverity::Error)
        || form.fields.values().any(FieldState::has_errors)
}

pub fn form_field_order(form: &FormState) -> Vec<FieldId> {
    form.fields.keys().cloned().collect()
}

pub fn next_form_field(fields: &[FieldId], current: Option<&FieldId>) -> Option<FieldId> {
    if fields.is_empty() {
        return None;
    }
    let start = current
        .and_then(|id| fields.iter().position(|field| field == id))
        .map(|index| (index + 1) % fields.len())
        .unwrap_or(0);
    fields.get(start).cloned()
}

pub fn previous_form_field(fields: &[FieldId], current: Option<&FieldId>) -> Option<FieldId> {
    if fields.is_empty() {
        return None;
    }
    let index = current
        .and_then(|id| fields.iter().position(|field| field == id))
        .map(|index| {
            if index == 0 {
                fields.len() - 1
            } else {
                index - 1
            }
        })
        .unwrap_or(fields.len() - 1);
    fields.get(index).cloned()
}

fn validation_severity_label(severity: ValidationSeverity) -> &'static str {
    match severity {
        ValidationSeverity::Info => "Info",
        ValidationSeverity::Warning => "Warning",
        ValidationSeverity::Error => "Error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_widgets_build_required_labels_validation_and_summary() {
        let mut document = UiDocument::new(root_style(420.0, 260.0));
        let root = document.root;
        let section = form_section(
            &mut document,
            root,
            "account",
            Some("Account".to_string()),
            FormSectionOptions::default(),
        );
        let row = form_row(
            &mut document,
            section.root,
            "email-row",
            FormRowOptions::default()
                .required()
                .invalid("Email is required"),
        );
        let field = field_label(
            &mut document,
            row,
            "email-label",
            "Email",
            FieldLabelOptions::default().required(),
        );
        let validation = field_validation_message(
            &mut document,
            row,
            "email-error",
            ValidationMessage::error("Email is required"),
            ValidationMessageOptions::default(),
        );
        field_help_text(
            &mut document,
            row,
            "email-help",
            "Used for receipts",
            FieldHelpOptions::default(),
        );

        assert!(matches!(
            &document.node(field).content,
            UiContent::Text(text) if text.text == "Email *"
        ));
        assert!(
            document
                .node(field)
                .accessibility
                .as_ref()
                .unwrap()
                .required
        );
        assert_eq!(
            document
                .node(validation)
                .accessibility
                .as_ref()
                .unwrap()
                .role,
            AccessibilityRole::Alert
        );
        assert_eq!(
            document
                .node(row)
                .accessibility
                .as_ref()
                .unwrap()
                .invalid
                .as_deref(),
            Some("Email is required")
        );

        let mut form = FormState::new("account").with_field("email", "");
        let request = form.begin_form_validation();
        form.apply_form_validation(
            FormValidationResult::new(request.generation)
                .with_field_messages("email", vec![ValidationMessage::error("Email is required")]),
        );
        let summary = form_error_summary(
            &mut document,
            root,
            "account-summary",
            &form,
            FormErrorSummaryOptions::default(),
        )
        .expect("summary");
        assert_eq!(summary.messages.len(), 1);
        assert_eq!(
            document
                .node(summary.root)
                .accessibility
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("1 errors")
        );
    }

    #[test]
    fn form_action_buttons_reflect_form_state() {
        let mut document = UiDocument::new(root_style(420.0, 160.0));
        let root = document.root;
        let mut form = FormState::new("account").with_field("email", "old@example.com");
        form.fields.get_mut(&FieldId::from("email")).unwrap().value = "new@example.com".to_owned();
        form.fields.get_mut(&FieldId::from("email")).unwrap().dirty = true;
        form.dirty = true;

        let nodes = form_action_buttons(
            &mut document,
            root,
            "account-actions",
            &form,
            FormActionButtonsOptions::default().include_reset(true),
        );

        assert_eq!(document.node(nodes.root).children.len(), 4);
        assert_eq!(
            document.node(nodes.submit).action.as_ref(),
            Some(&WidgetActionBinding::action("form.submit"))
        );
        assert!(document
            .node(nodes.apply)
            .accessibility
            .as_ref()
            .is_some_and(|accessibility| accessibility.enabled));
        assert!(document
            .node(nodes.reset.unwrap())
            .accessibility
            .as_ref()
            .is_some_and(|accessibility| accessibility.enabled));
    }

    #[test]
    fn form_field_traversal_wraps_stable_field_order() {
        let form = FormState::new("account")
            .with_field("email", "")
            .with_field("name", "")
            .with_field("timezone", "");
        let order = form_field_order(&form);

        assert_eq!(
            next_form_field(&order, Some(&FieldId::from("name"))),
            Some(FieldId::from("timezone"))
        );
        assert_eq!(
            next_form_field(&order, Some(&FieldId::from("timezone"))),
            Some(FieldId::from("email"))
        );
        assert_eq!(
            previous_form_field(&order, Some(&FieldId::from("email"))),
            Some(FieldId::from("timezone"))
        );
    }
}
