use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePreferenceLabels {
    pub system: String,
    pub light: String,
    pub dark: String,
    pub switch: String,
}

impl ThemePreferenceLabels {
    pub fn label(&self, preference: ThemePreference) -> &str {
        match preference {
            ThemePreference::System => &self.system,
            ThemePreference::Light => &self.light,
            ThemePreference::Dark => &self.dark,
        }
    }
}

impl Default for ThemePreferenceLabels {
    fn default() -> Self {
        Self {
            system: "System".to_owned(),
            light: "Light".to_owned(),
            dark: "Dark".to_owned(),
            switch: "Dark theme".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThemePreferenceButtonsOptions {
    pub layout: LayoutStyle,
    pub button_options: ButtonOptions,
    pub labels: ThemePreferenceLabels,
    pub include_system: bool,
    pub enabled: bool,
    pub action_prefix: Option<String>,
    pub accessibility_label: Option<String>,
}

impl ThemePreferenceButtonsOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_button_options(mut self, options: ButtonOptions) -> Self {
        self.button_options = options;
        self
    }

    pub fn with_labels(mut self, labels: ThemePreferenceLabels) -> Self {
        self.labels = labels;
        self
    }

    pub const fn include_system(mut self, include_system: bool) -> Self {
        self.include_system = include_system;
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

    fn action_for(&self, preference: ThemePreference) -> Option<WidgetActionBinding> {
        self.action_prefix
            .as_ref()
            .map(|prefix| WidgetActionBinding::action(format!("{prefix}.{}", preference.as_str())))
    }
}

impl Default for ThemePreferenceButtonsOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                gap: TaffySize {
                    width: taffy::prelude::LengthPercentage::length(4.0),
                    height: taffy::prelude::LengthPercentage::length(4.0),
                },
                ..Default::default()
            }),
            button_options: ButtonOptions {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    align_items: Some(AlignItems::Center),
                    justify_content: Some(JustifyContent::Center),
                    size: TaffySize {
                        width: length(80.0),
                        height: length(30.0),
                    },
                    padding: taffy::prelude::Rect::length(6.0),
                    ..Default::default()
                }),
                ..Default::default()
            },
            labels: ThemePreferenceLabels::default(),
            include_system: true,
            enabled: true,
            action_prefix: Some("theme.preference".to_owned()),
            accessibility_label: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemePreferenceButtonNodes {
    pub root: UiNodeId,
    pub system: Option<UiNodeId>,
    pub light: UiNodeId,
    pub dark: UiNodeId,
}

impl ThemePreferenceButtonNodes {
    pub fn node_for(self, preference: ThemePreference) -> Option<UiNodeId> {
        match preference {
            ThemePreference::System => self.system,
            ThemePreference::Light => Some(self.light),
            ThemePreference::Dark => Some(self.dark),
        }
    }
}

pub fn theme_preference_buttons(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    current: ThemePreference,
    options: ThemePreferenceButtonsOptions,
) -> ThemePreferenceButtonNodes {
    let name = name.into();
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
                    .unwrap_or_else(|| "Theme preference".to_owned()),
            ),
        ),
    );

    let system = options.include_system.then(|| {
        add_theme_preference_button(
            document,
            root,
            &name,
            ThemePreference::System,
            current,
            &options,
        )
    });
    let light = add_theme_preference_button(
        document,
        root,
        &name,
        ThemePreference::Light,
        current,
        &options,
    );
    let dark = add_theme_preference_button(
        document,
        root,
        &name,
        ThemePreference::Dark,
        current,
        &options,
    );

    ThemePreferenceButtonNodes {
        root,
        system,
        light,
        dark,
    }
}

fn add_theme_preference_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    preference: ThemePreference,
    current: ThemePreference,
    options: &ThemePreferenceButtonsOptions,
) -> UiNodeId {
    let label = options.labels.label(preference).to_owned();
    let mut button_options = options.button_options.clone();
    button_options.enabled = options.enabled;
    button_options.action = options.action_for(preference);
    button_options.accessibility_label = Some(format!("{label} theme"));
    toggle_button(
        document,
        parent,
        format!("{name}.{}", preference.as_str()),
        label,
        current == preference,
        button_options,
    )
}

#[derive(Debug, Clone)]
pub struct ThemePreferenceSwitchOptions {
    pub switch_options: ToggleSwitchOptions,
    pub labels: ThemePreferenceLabels,
    pub enabled: bool,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
}

impl ThemePreferenceSwitchOptions {
    pub fn with_switch_options(mut self, options: ToggleSwitchOptions) -> Self {
        self.switch_options = options;
        self
    }

    pub fn with_labels(mut self, labels: ThemePreferenceLabels) -> Self {
        self.labels = labels;
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
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

impl Default for ThemePreferenceSwitchOptions {
    fn default() -> Self {
        Self {
            switch_options: ToggleSwitchOptions::default(),
            labels: ThemePreferenceLabels::default(),
            enabled: true,
            action: Some(WidgetActionBinding::action("theme.preference.dark")),
            accessibility_label: None,
        }
    }
}

pub fn theme_preference_switch(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    current: ThemePreference,
    options: ThemePreferenceSwitchOptions,
) -> UiNodeId {
    let name = name.into();
    let mut switch_options = options.switch_options;
    switch_options.enabled = options.enabled;
    switch_options.action = options.action;
    switch_options.accessibility_label = Some(
        options
            .accessibility_label
            .unwrap_or_else(|| options.labels.switch.clone()),
    );
    toggle_switch(
        document,
        parent,
        name,
        options.labels.switch,
        if current.is_dark() {
            ToggleValue::On
        } else {
            ToggleValue::Off
        },
        switch_options,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_preference_buttons_build_segmented_choices() {
        let mut document = UiDocument::new(root_style(320.0, 120.0));
        let root = document.root;
        let nodes = theme_preference_buttons(
            &mut document,
            root,
            "theme",
            ThemePreference::Dark,
            ThemePreferenceButtonsOptions::default(),
        );

        assert!(nodes.system.is_some());
        assert_eq!(document.node(nodes.root).children.len(), 3);
        assert_eq!(
            document.node(nodes.dark).action.as_ref(),
            Some(&WidgetActionBinding::action("theme.preference.dark"))
        );
        assert!(document
            .node(nodes.dark)
            .accessibility
            .as_ref()
            .is_some_and(|accessibility| accessibility.pressed == Some(true)));
    }

    #[test]
    fn theme_preference_switch_maps_dark_to_checked_switch() {
        let mut document = UiDocument::new(root_style(320.0, 120.0));
        let root = document.root;
        let switch = theme_preference_switch(
            &mut document,
            root,
            "dark-theme",
            ThemePreference::Dark,
            ThemePreferenceSwitchOptions::default(),
        );

        assert_eq!(
            document.node(switch).action.as_ref(),
            Some(&WidgetActionBinding::action("theme.preference.dark"))
        );
        assert!(
            document
                .node(switch)
                .accessibility
                .as_ref()
                .is_some_and(
                    |accessibility| accessibility.checked == Some(AccessibilityChecked::True)
                )
        );
    }
}
