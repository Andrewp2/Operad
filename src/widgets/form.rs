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

#[derive(Debug, Clone)]
pub struct ValidationMessageOptions {
    pub layout: LayoutStyle,
    pub text_style: Option<TextStyle>,
}

impl Default for ValidationMessageOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::new(),
            text_style: None,
        }
    }
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
}
