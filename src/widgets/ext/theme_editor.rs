//! Theme editor widgets built from renderer-neutral theme snapshots.

use std::collections::HashMap;

use taffy::prelude::{
    Dimension, Display, FlexDirection, LengthPercentage, Size as TaffySize, Style,
};

use crate::{
    AccessibilityPreferences, ColorRgba, ComponentRole, ComponentState, DebugThemeComponentState,
    DebugThemeSnapshot, DebugThemeToken, DebugThemeTokenKind, LayoutStyle, TextStyle, Theme,
    ThemePatch, UiDocument, UiNode, UiNodeId, UiNodeStyle,
};

use super::data::PropertyValueKind;
use super::property_inspector::{
    property_inspector_grid, PropertyGridRow, PropertyInspectorOptions,
};

#[derive(Debug, Clone)]
pub struct ThemeEditorPanelOptions {
    pub layout: LayoutStyle,
    pub token_filter: Option<DebugThemeTokenKind>,
    pub max_token_rows: usize,
    pub max_component_rows: usize,
    pub label_width: f32,
    pub row_height: f32,
    pub action_prefix: Option<String>,
    pub title_style: TextStyle,
}

impl Default for ThemeEditorPanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                gap: taffy::geometry::Size {
                    width: LengthPercentage::length(8.0),
                    height: LengthPercentage::length(8.0),
                },
                ..Default::default()
            }),
            token_filter: None,
            max_token_rows: 32,
            max_component_rows: 12,
            label_width: 190.0,
            row_height: 26.0,
            action_prefix: None,
            title_style: TextStyle {
                font_size: 14.0,
                line_height: 20.0,
                color: ColorRgba::new(238, 243, 248, 255),
                ..Default::default()
            },
        }
    }
}

impl ThemeEditorPanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }

    pub fn with_token_filter(mut self, kind: DebugThemeTokenKind) -> Self {
        self.token_filter = Some(kind);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeEditorPanelNodes {
    pub root: UiNodeId,
    pub token_grid: UiNodeId,
    pub component_grid: UiNodeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemePatchGroup {
    Colors,
    Spacing,
    Typography,
    Radius,
    Stroke,
    Effects,
    Opacity,
    Motion,
    Components,
}

impl ThemePatchGroup {
    pub const fn token_prefix(self) -> &'static str {
        match self {
            Self::Colors => "colors",
            Self::Spacing => "spacing",
            Self::Typography => "typography",
            Self::Radius => "radius",
            Self::Stroke => "stroke",
            Self::Effects => "effects",
            Self::Opacity => "opacity",
            Self::Motion => "motion",
            Self::Components => "components",
        }
    }

    pub const fn patch_method(self) -> &'static str {
        match self {
            Self::Colors => "colors",
            Self::Spacing => "spacing",
            Self::Typography => "typography",
            Self::Radius => "radius",
            Self::Stroke => "stroke",
            Self::Effects => "effects",
            Self::Opacity => "opacity",
            Self::Motion => "motion",
            Self::Components => "components",
        }
    }

    const fn requires_clone(self) -> bool {
        matches!(self, Self::Typography | Self::Components)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePatchTokenChange {
    pub path: String,
    pub kind: DebugThemeTokenKind,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePatchSnippetOptions {
    pub base_theme_expr: String,
    pub patch_variable: String,
}

impl Default for ThemePatchSnippetOptions {
    fn default() -> Self {
        Self {
            base_theme_expr: "base_theme".to_owned(),
            patch_variable: "patch".to_owned(),
        }
    }
}

impl ThemePatchSnippetOptions {
    pub fn with_base_theme_expr(mut self, expression: impl Into<String>) -> Self {
        self.base_theme_expr = expression.into();
        self
    }

    pub fn with_patch_variable(mut self, variable: impl Into<String>) -> Self {
        self.patch_variable = variable.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemePatchExport {
    pub patch: ThemePatch,
    pub changed_groups: Vec<ThemePatchGroup>,
    pub changed_tokens: Vec<ThemePatchTokenChange>,
    pub rust_snippet: String,
}

impl ThemePatchExport {
    pub fn from_themes(base: &Theme, edited: &Theme) -> Self {
        let patch = theme_patch_from_themes(base, edited);
        let changed_groups = theme_patch_changed_groups(base, edited);
        let changed_tokens = theme_patch_token_changes(base, edited);
        let rust_snippet = theme_patch_rust_snippet_parts(
            &changed_groups,
            &changed_tokens,
            ThemePatchSnippetOptions::default(),
        );
        Self {
            patch,
            changed_groups,
            changed_tokens,
            rust_snippet,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.patch.is_empty() && self.changed_tokens.is_empty()
    }

    pub fn rust_snippet_with_options(&self, options: ThemePatchSnippetOptions) -> String {
        theme_patch_rust_snippet_parts(&self.changed_groups, &self.changed_tokens, options)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeAccessibilityIssueKind {
    TextContrast,
    IconContrast,
    ReducedMotionPolicy,
    HighContrastPolicy,
    ForcedColorsPolicy,
    ReducedTransparencyPolicy,
    TextScalePolicy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeAccessibilityIssue {
    pub kind: ThemeAccessibilityIssueKind,
    pub role: Option<ComponentRole>,
    pub state: Option<ComponentState>,
    pub role_label: Option<String>,
    pub state_label: Option<String>,
    pub token_path: Option<String>,
    pub message: String,
    pub contrast_ratio: Option<f32>,
    pub required_ratio: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeAccessibilityAuditOptions {
    pub fallback_background: ColorRgba,
    pub minimum_text_contrast: f32,
    pub minimum_icon_contrast: f32,
    pub include_disabled_states: bool,
}

impl Default for ThemeAccessibilityAuditOptions {
    fn default() -> Self {
        Self {
            fallback_background: ColorRgba::BLACK,
            minimum_text_contrast: 4.5,
            minimum_icon_contrast: 3.0,
            include_disabled_states: false,
        }
    }
}

impl ThemeAccessibilityAuditOptions {
    pub fn fallback_background(mut self, color: ColorRgba) -> Self {
        self.fallback_background = color;
        self
    }

    pub fn include_disabled_states(mut self, include: bool) -> Self {
        self.include_disabled_states = include;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThemeAccessibilityAudit {
    pub issues: Vec<ThemeAccessibilityIssue>,
}

impl ThemeAccessibilityAudit {
    pub fn from_theme(theme: &Theme, options: ThemeAccessibilityAuditOptions) -> Self {
        let snapshot = DebugThemeSnapshot::from_theme(theme);
        let mut audit = Self::default();
        audit.collect_component_contrast(&snapshot, options);
        audit
    }

    pub fn from_theme_with_preferences(
        baseline: &Theme,
        current: &Theme,
        preferences: AccessibilityPreferences,
        options: ThemeAccessibilityAuditOptions,
    ) -> Self {
        let mut audit = Self::from_theme(current, options);
        audit.collect_preference_policy(baseline, current, preferences);
        audit
    }

    pub fn passed(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn has_issue(&self, kind: ThemeAccessibilityIssueKind) -> bool {
        self.issues.iter().any(|issue| issue.kind == kind)
    }

    pub fn issues_of_kind(
        &self,
        kind: ThemeAccessibilityIssueKind,
    ) -> impl Iterator<Item = &ThemeAccessibilityIssue> {
        self.issues.iter().filter(move |issue| issue.kind == kind)
    }

    fn collect_component_contrast(
        &mut self,
        snapshot: &DebugThemeSnapshot,
        options: ThemeAccessibilityAuditOptions,
    ) {
        for component in &snapshot.component_states {
            if component.state.disabled() && !options.include_disabled_states {
                continue;
            }

            let background = component.fill.composite_over(options.fallback_background);
            let text_color = component.text_color.composite_over(background);
            let text_ratio = text_color.contrast_ratio(background);
            if text_ratio + f32::EPSILON < options.minimum_text_contrast {
                self.issues.push(component_contrast_issue(
                    ThemeAccessibilityIssueKind::TextContrast,
                    component,
                    "text",
                    text_ratio,
                    options.minimum_text_contrast,
                ));
            }

            if component.icon_opacity > f32::EPSILON {
                let icon_color = color_with_opacity(component.icon_tint, component.icon_opacity)
                    .composite_over(background);
                let icon_ratio = icon_color.contrast_ratio(background);
                if icon_ratio + f32::EPSILON < options.minimum_icon_contrast {
                    self.issues.push(component_contrast_issue(
                        ThemeAccessibilityIssueKind::IconContrast,
                        component,
                        "icon",
                        icon_ratio,
                        options.minimum_icon_contrast,
                    ));
                }
            }
        }
    }

    fn collect_preference_policy(
        &mut self,
        baseline: &Theme,
        current: &Theme,
        preferences: AccessibilityPreferences,
    ) {
        let expected = baseline.with_accessibility_preferences(preferences);

        if preferences.should_reduce_motion() && current.motion != expected.motion {
            self.issues.push(policy_issue(
                ThemeAccessibilityIssueKind::ReducedMotionPolicy,
                "motion",
                "Reduced-motion preferences are active, but motion tokens do not match the reduced-motion theme policy.",
            ));
        }

        if preferences.forced_colors
            && (current.colors != expected.colors
                || current.stroke != expected.stroke
                || current.typography != expected.typography
                || current.components != expected.components)
        {
            self.issues.push(policy_issue(
                ThemeAccessibilityIssueKind::ForcedColorsPolicy,
                "colors",
                "Forced-colors preferences are active, but color, stroke, typography, or component tokens do not match the forced-colors policy.",
            ));
        } else if preferences.high_contrast
            && (current.colors != expected.colors
                || current.stroke != expected.stroke
                || current.typography != expected.typography
                || current.components != expected.components)
        {
            self.issues.push(policy_issue(
                ThemeAccessibilityIssueKind::HighContrastPolicy,
                "colors",
                "High-contrast preferences are active, but color, stroke, typography, or component tokens do not match the high-contrast policy.",
            ));
        }

        if preferences.reduced_transparency
            && (current.colors != expected.colors
                || current.opacity != expected.opacity
                || current.effects != expected.effects
                || current.typography != expected.typography
                || current.components != expected.components)
        {
            self.issues.push(policy_issue(
                ThemeAccessibilityIssueKind::ReducedTransparencyPolicy,
                "opacity",
                "Reduced-transparency preferences are active, but color, opacity, effect, typography, or component tokens do not match the reduced-transparency policy.",
            ));
        }

        if (preferences.normalized_text_scale() - AccessibilityPreferences::DEFAULT.text_scale)
            .abs()
            > f32::EPSILON
            && (current.typography != expected.typography
                || current.components != expected.components)
        {
            self.issues.push(policy_issue(
                ThemeAccessibilityIssueKind::TextScalePolicy,
                "typography",
                "Text-scale preferences are active, but typography or component tokens do not match the scaled theme policy.",
            ));
        }
    }
}

pub fn theme_patch_export(base: &Theme, edited: &Theme) -> ThemePatchExport {
    ThemePatchExport::from_themes(base, edited)
}

pub fn theme_patch_from_themes(base: &Theme, edited: &Theme) -> ThemePatch {
    ThemePatch {
        colors: (base.colors != edited.colors).then_some(edited.colors),
        spacing: (base.spacing != edited.spacing).then_some(edited.spacing),
        typography: (base.typography != edited.typography).then_some(edited.typography.clone()),
        radius: (base.radius != edited.radius).then_some(edited.radius),
        stroke: (base.stroke != edited.stroke).then_some(edited.stroke),
        effects: (base.effects != edited.effects).then_some(edited.effects),
        opacity: (base.opacity != edited.opacity).then_some(edited.opacity),
        motion: (base.motion != edited.motion).then_some(edited.motion),
        components: (base.components != edited.components).then_some(edited.components.clone()),
    }
}

pub fn theme_editor_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    snapshot: &DebugThemeSnapshot,
    options: ThemeEditorPanelOptions,
) -> ThemeEditorPanelNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                ..Default::default()
            },
        ),
    );

    document.add_child(
        root,
        UiNode::text(
            format!("{name}.tokens.title"),
            format!("Theme: {}", snapshot.name),
            options.title_style.clone(),
            LayoutStyle::new().with_width_percent(1.0).with_height(24.0),
        ),
    );

    let token_grid = property_inspector_grid(
        document,
        root,
        format!("{name}.tokens"),
        &theme_token_rows(snapshot, options.token_filter, options.max_token_rows),
        grid_options(
            options.label_width,
            options.row_height,
            options
                .action_prefix
                .as_deref()
                .map(|prefix| format!("{prefix}.token")),
            "Theme tokens",
        ),
    );

    document.add_child(
        root,
        UiNode::text(
            format!("{name}.components.title"),
            "Component states",
            TextStyle {
                color: ColorRgba::new(171, 183, 201, 255),
                ..options.title_style
            },
            LayoutStyle::new().with_width_percent(1.0).with_height(24.0),
        ),
    );

    let component_grid = property_inspector_grid(
        document,
        root,
        format!("{name}.components"),
        &theme_component_rows(snapshot, options.max_component_rows),
        grid_options(
            options.label_width,
            options.row_height,
            options
                .action_prefix
                .as_deref()
                .map(|prefix| format!("{prefix}.component")),
            "Theme component states",
        ),
    );

    ThemeEditorPanelNodes {
        root,
        token_grid,
        component_grid,
    }
}

fn grid_options(
    label_width: f32,
    row_height: f32,
    action_prefix: Option<String>,
    accessibility_label: impl Into<String>,
) -> PropertyInspectorOptions {
    PropertyInspectorOptions {
        label_width,
        row_height,
        action_prefix,
        accessibility_label: Some(accessibility_label.into()),
        ..Default::default()
    }
}

fn theme_token_rows(
    snapshot: &DebugThemeSnapshot,
    filter: Option<DebugThemeTokenKind>,
    limit: usize,
) -> Vec<PropertyGridRow> {
    let rows = snapshot
        .tokens
        .iter()
        .filter(|token| filter.is_none_or(|kind| token.kind == kind))
        .take(limit)
        .map(theme_token_row)
        .collect::<Vec<_>>();

    if rows.is_empty() {
        vec![PropertyGridRow::new("empty", "Tokens", "No matching tokens").read_only()]
    } else {
        rows
    }
}

fn theme_token_row(token: &DebugThemeToken) -> PropertyGridRow {
    PropertyGridRow::new(
        sanitize_row_id(&token.path),
        token.path.clone(),
        compact_value(token.value.clone(), 40),
    )
    .with_kind(theme_token_value_kind(token.kind))
    .read_only()
}

fn theme_token_value_kind(kind: DebugThemeTokenKind) -> PropertyValueKind {
    match kind {
        DebugThemeTokenKind::Color => PropertyValueKind::Color,
        DebugThemeTokenKind::Spacing
        | DebugThemeTokenKind::Radius
        | DebugThemeTokenKind::Stroke
        | DebugThemeTokenKind::Opacity
        | DebugThemeTokenKind::Motion
        | DebugThemeTokenKind::ComponentLayout => PropertyValueKind::Number,
        DebugThemeTokenKind::Typography
        | DebugThemeTokenKind::Theme
        | DebugThemeTokenKind::Effect => PropertyValueKind::Text,
    }
}

fn theme_component_rows(snapshot: &DebugThemeSnapshot, limit: usize) -> Vec<PropertyGridRow> {
    let rows = snapshot
        .component_states
        .iter()
        .take(limit)
        .map(theme_component_row)
        .collect::<Vec<_>>();

    if rows.is_empty() {
        vec![PropertyGridRow::new("empty", "Components", "No component states").read_only()]
    } else {
        rows
    }
}

fn theme_component_row(component: &DebugThemeComponentState) -> PropertyGridRow {
    let id = sanitize_row_id(format!(
        "{}.{}",
        component.role_label, component.state_label
    ));
    let value = compact_value(
        format!(
            "visual={:?}; text={:?}; min={:.0}x{:.0}; pad={:.0},{:.0}",
            component.visual_slot,
            component.text_slot,
            component.min_width,
            component.min_height,
            component.padding_x,
            component.padding_y
        ),
        40,
    );
    PropertyGridRow::new(
        id,
        format!("{}.{}", component.role_label, component.state_label),
        value,
    )
    .with_kind(PropertyValueKind::Custom)
    .read_only()
}

fn sanitize_row_id(value: impl AsRef<str>) -> String {
    let mut out = String::new();
    for character in value.as_ref().chars() {
        if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
            out.push(character);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "row".to_owned()
    } else {
        out
    }
}

fn compact_value(value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }
    let mut out = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

fn theme_patch_changed_groups(base: &Theme, edited: &Theme) -> Vec<ThemePatchGroup> {
    let mut groups = Vec::new();
    if base.colors != edited.colors {
        groups.push(ThemePatchGroup::Colors);
    }
    if base.spacing != edited.spacing {
        groups.push(ThemePatchGroup::Spacing);
    }
    if base.typography != edited.typography {
        groups.push(ThemePatchGroup::Typography);
    }
    if base.radius != edited.radius {
        groups.push(ThemePatchGroup::Radius);
    }
    if base.stroke != edited.stroke {
        groups.push(ThemePatchGroup::Stroke);
    }
    if base.effects != edited.effects {
        groups.push(ThemePatchGroup::Effects);
    }
    if base.opacity != edited.opacity {
        groups.push(ThemePatchGroup::Opacity);
    }
    if base.motion != edited.motion {
        groups.push(ThemePatchGroup::Motion);
    }
    if base.components != edited.components {
        groups.push(ThemePatchGroup::Components);
    }
    groups
}

fn theme_patch_token_changes(base: &Theme, edited: &Theme) -> Vec<ThemePatchTokenChange> {
    let before = DebugThemeSnapshot::from_theme(base);
    let after = DebugThemeSnapshot::from_theme(edited);
    let after_by_path = after
        .tokens
        .iter()
        .map(|token| (token.path.as_str(), token))
        .collect::<HashMap<_, _>>();

    let mut changes = Vec::new();
    for before_token in &before.tokens {
        let Some(after_token) = after_by_path.get(before_token.path.as_str()) else {
            continue;
        };
        if before_token.value == after_token.value {
            continue;
        }
        changes.push(ThemePatchTokenChange {
            path: before_token.path.clone(),
            kind: after_token.kind,
            before: before_token.value.clone(),
            after: after_token.value.clone(),
        });
    }
    changes
}

fn theme_patch_rust_snippet_parts(
    groups: &[ThemePatchGroup],
    changes: &[ThemePatchTokenChange],
    options: ThemePatchSnippetOptions,
) -> String {
    if groups.is_empty() {
        return "ThemePatch::new()".to_owned();
    }

    let mut lines = vec![
        "{".to_owned(),
        format!(
            "    let mut {} = ThemePatch::new();",
            options.patch_variable
        ),
    ];

    for &group in groups {
        let variable = group.token_prefix();
        let clone_suffix = if group.requires_clone() {
            ".clone()"
        } else {
            ""
        };
        lines.push(format!(
            "    let mut {variable} = {}.{}{clone_suffix};",
            options.base_theme_expr,
            group.token_prefix()
        ));
        let mut supported_assignments = 0;
        for change in changes.iter().filter(|change| {
            change
                .path
                .strip_prefix(group.token_prefix())
                .is_some_and(|suffix| suffix.starts_with('.'))
        }) {
            if let Some(assignment) = theme_patch_rust_assignment(group, change) {
                lines.push(format!("    {assignment}"));
                supported_assignments += 1;
            }
        }
        if supported_assignments == 0 {
            lines.push(format!(
                "    // {} changed outside exported primitive token rows; keep the typed patch value or set fields manually.",
                group.token_prefix()
            ));
        }
        lines.push(format!(
            "    {} = {}.{}({});",
            options.patch_variable,
            options.patch_variable,
            group.patch_method(),
            variable
        ));
    }

    lines.push(format!("    {}", options.patch_variable));
    lines.push("}".to_owned());
    lines.join("\n")
}

fn theme_patch_rust_assignment(
    group: ThemePatchGroup,
    change: &ThemePatchTokenChange,
) -> Option<String> {
    let field = change
        .path
        .strip_prefix(group.token_prefix())?
        .strip_prefix('.')?;
    if !is_safe_theme_field_path(field) {
        return None;
    }

    let value = match group {
        ThemePatchGroup::Colors => color_literal(&change.after)?,
        ThemePatchGroup::Spacing | ThemePatchGroup::Radius | ThemePatchGroup::Opacity => {
            f32_literal(change.after.parse::<f32>().ok()?)
        }
        ThemePatchGroup::Stroke => {
            if let Some(width_field) = field.strip_suffix("_width") {
                let _ = width_field;
                f32_literal(change.after.parse::<f32>().ok()?)
            } else {
                stroke_literal(&change.after)?
            }
        }
        ThemePatchGroup::Motion => motion_literal(field, &change.after)?,
        ThemePatchGroup::Components => component_layout_literal(&change.after)?,
        ThemePatchGroup::Typography | ThemePatchGroup::Effects => return None,
    };
    Some(format!("{}.{field} = {value};", group.token_prefix()))
}

fn is_safe_theme_field_path(path: &str) -> bool {
    path.split('.').all(|segment| {
        !segment.is_empty()
            && segment
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
    })
}

fn color_literal(value: &str) -> Option<String> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 8 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
    Some(format!("ColorRgba::new({r}, {g}, {b}, {a})"))
}

fn stroke_literal(value: &str) -> Option<String> {
    let width = value
        .strip_prefix("width=")?
        .split_once(" color=")?
        .0
        .parse::<f32>()
        .ok()?;
    let color = value.split_once(" color=")?.1;
    Some(format!(
        "StrokeStyle::new({}, {})",
        color_literal(color)?,
        f32_literal(width)
    ))
}

fn motion_literal(field: &str, value: &str) -> Option<String> {
    if field.ends_with("_ms") {
        return value
            .parse::<u16>()
            .ok()
            .map(|duration| duration.to_string());
    }
    if field == "reduced_motion_scale" {
        return value.parse::<f32>().ok().map(f32_literal);
    }
    if matches!(value, "Linear" | "EaseOut" | "EaseInOut") {
        return Some(format!("MotionCurve::{value}"));
    }
    if value.starts_with("CubicBezier(") && value.ends_with(')') {
        return Some(format!("MotionCurve::{value}"));
    }
    None
}

fn component_layout_literal(value: &str) -> Option<String> {
    let min = value.strip_prefix("min=")?.split_once(" padding=")?;
    let (min_width, min_height) = parse_pair(min.0, 'x')?;
    let padding = min.1.split_once(" gap=")?;
    let (padding_x, padding_y) = parse_pair(padding.0, 'x')?;
    let gap = padding.1.split_once(" icon=")?;
    let gap_value = gap.0.parse::<f32>().ok()?;
    let icon_size = gap.1.parse::<f32>().ok()?;
    Some(format!(
        "ComponentLayoutTokens {{ min_width: {}, min_height: {}, padding_x: {}, padding_y: {}, gap: {}, icon_size: {} }}",
        f32_literal(min_width),
        f32_literal(min_height),
        f32_literal(padding_x),
        f32_literal(padding_y),
        f32_literal(gap_value),
        f32_literal(icon_size)
    ))
}

fn parse_pair(value: &str, delimiter: char) -> Option<(f32, f32)> {
    let (first, second) = value.split_once(delimiter)?;
    Some((first.parse::<f32>().ok()?, second.parse::<f32>().ok()?))
}

fn f32_literal(value: f32) -> String {
    if value.is_nan() {
        return "f32::NAN".to_owned();
    }
    if value == f32::INFINITY {
        return "f32::INFINITY".to_owned();
    }
    if value == f32::NEG_INFINITY {
        return "f32::NEG_INFINITY".to_owned();
    }

    let mut out = format!("{value:.3}");
    while out.contains('.') && out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.push('0');
    }
    out
}

fn component_contrast_issue(
    kind: ThemeAccessibilityIssueKind,
    component: &DebugThemeComponentState,
    target: &str,
    contrast_ratio: f32,
    required_ratio: f32,
) -> ThemeAccessibilityIssue {
    ThemeAccessibilityIssue {
        kind,
        role: Some(component.role),
        state: Some(component.state),
        role_label: Some(component.role_label.clone()),
        state_label: Some(component.state_label.clone()),
        token_path: Some(format!(
            "components.{}.{}",
            component.role_label, component.state_label
        )),
        message: format!(
            "{} {} {target} contrast {:.2}:1 is below required {:.2}:1",
            component.role_label, component.state_label, contrast_ratio, required_ratio
        ),
        contrast_ratio: Some(contrast_ratio),
        required_ratio: Some(required_ratio),
    }
}

fn policy_issue(
    kind: ThemeAccessibilityIssueKind,
    token_path: &str,
    message: &str,
) -> ThemeAccessibilityIssue {
    ThemeAccessibilityIssue {
        kind,
        role: None,
        state: None,
        role_label: None,
        state_label: None,
        token_path: Some(token_path.to_owned()),
        message: message.to_owned(),
        contrast_ratio: None,
        required_ratio: None,
    }
}

fn color_with_opacity(color: ColorRgba, opacity: f32) -> ColorRgba {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    ColorRgba::new(
        color.r,
        color.g,
        color.b,
        ((color.a as f32) * opacity).round().clamp(0.0, 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        root_style, ApproxTextMeasurer, ComponentRole, ComponentState, MotionCurve, StrokeStyle,
        Theme, UiSize, WidgetActionBinding,
    };

    #[test]
    fn theme_editor_panel_builds_token_and_component_rows() {
        let snapshot = DebugThemeSnapshot::from_theme(&Theme::dark());
        let mut doc = UiDocument::new(root_style(480.0, 520.0));
        let root = doc.root;

        let nodes = theme_editor_panel(
            &mut doc,
            root,
            "theme.editor",
            &snapshot,
            ThemeEditorPanelOptions {
                token_filter: Some(DebugThemeTokenKind::Color),
                max_token_rows: 3,
                max_component_rows: 2,
                action_prefix: Some("theme.edit".to_owned()),
                ..Default::default()
            },
        );
        doc.compute_layout(UiSize::new(480.0, 520.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert_eq!(doc.node(nodes.root).name, "theme.editor");
        assert_eq!(doc.node(nodes.token_grid).children.len(), 3);
        assert_eq!(doc.node(nodes.component_grid).children.len(), 2);

        let first_row = doc.node(doc.node(nodes.token_grid).children[0]);
        assert_eq!(
            first_row.action.as_ref(),
            Some(&WidgetActionBinding::action(
                "theme.edit.token.row.colors_canvas"
            ))
        );
    }

    #[test]
    fn theme_patch_export_collects_patch_groups_and_changed_tokens() {
        let base = Theme::dark();
        let mut edited = base.clone();
        edited.colors.accent = ColorRgba::new(1, 2, 3, 255);
        edited.spacing.grid = 12.5;
        edited.stroke.focus = StrokeStyle::new(ColorRgba::new(8, 9, 10, 255), 2.5);
        edited.motion.standard = MotionCurve::EaseOut;
        edited.components.button.layout.min_width = 96.0;

        let export = ThemePatchExport::from_themes(&base, &edited);

        assert_eq!(
            export.changed_groups,
            vec![
                ThemePatchGroup::Colors,
                ThemePatchGroup::Spacing,
                ThemePatchGroup::Stroke,
                ThemePatchGroup::Motion,
                ThemePatchGroup::Components
            ]
        );
        assert_eq!(export.patch.colors.unwrap().accent, edited.colors.accent);
        assert_eq!(export.patch.spacing.unwrap().grid, 12.5);
        assert_eq!(export.patch.stroke.unwrap().focus, edited.stroke.focus);
        assert_eq!(export.patch.motion.unwrap().standard, MotionCurve::EaseOut);
        assert_eq!(
            export.patch.components.unwrap().button.layout.min_width,
            96.0
        );

        let paths = export
            .changed_tokens
            .iter()
            .map(|change| change.path.as_str())
            .collect::<Vec<_>>();
        assert!(paths.contains(&"colors.accent"));
        assert!(paths.contains(&"spacing.grid"));
        assert!(paths.contains(&"stroke.focus"));
        assert!(paths.contains(&"motion.standard"));
        assert!(paths.contains(&"components.button.layout"));

        assert!(export
            .rust_snippet
            .contains("colors.accent = ColorRgba::new(1, 2, 3, 255);"));
        assert!(export.rust_snippet.contains("spacing.grid = 12.5;"));
        assert!(export
            .rust_snippet
            .contains("stroke.focus = StrokeStyle::new(ColorRgba::new(8, 9, 10, 255), 2.5);"));
        assert!(export
            .rust_snippet
            .contains("motion.standard = MotionCurve::EaseOut;"));
        assert!(export
            .rust_snippet
            .contains("components.button.layout = ComponentLayoutTokens"));
    }

    #[test]
    fn theme_patch_export_supports_custom_snippet_names_and_empty_exports() {
        let base = Theme::dark();
        let empty = theme_patch_export(&base, &base);
        assert!(empty.is_empty());
        assert_eq!(empty.rust_snippet, "ThemePatch::new()");

        let mut edited = base.clone();
        edited.opacity.disabled = 0.31;
        let export = theme_patch_export(&base, &edited);
        let snippet = export.rust_snippet_with_options(
            ThemePatchSnippetOptions::default()
                .with_base_theme_expr("Theme::dark()")
                .with_patch_variable("theme_patch"),
        );

        assert!(snippet.contains("let mut theme_patch = ThemePatch::new();"));
        assert!(snippet.contains("let mut opacity = Theme::dark().opacity;"));
        assert!(snippet.contains("opacity.disabled = 0.31;"));
        assert!(snippet.contains("theme_patch = theme_patch.opacity(opacity);"));
    }

    #[test]
    fn theme_patch_export_ignores_resolved_component_state_rows() {
        let base = Theme::dark();
        let mut edited = base.clone();
        edited.colors.accent = ColorRgba::new(4, 5, 6, 255);

        let export = theme_patch_export(&base, &edited);

        assert!(export
            .changed_tokens
            .iter()
            .any(|change| change.path == "colors.accent"));
        assert!(DebugThemeSnapshot::from_theme(&edited)
            .component_state(ComponentRole::Button, ComponentState::NORMAL)
            .is_some());
    }

    #[test]
    fn theme_accessibility_audit_reports_component_text_and_icon_contrast() {
        let mut theme = Theme::dark();
        let fill = theme.components.button.visual.base.fill;
        theme.components.button.text.base.color = fill;
        theme.components.button.icon.base.tint = fill;
        theme.components.button.icon.base.opacity = 1.0;

        let audit = ThemeAccessibilityAudit::from_theme(
            &theme,
            ThemeAccessibilityAuditOptions::default().fallback_background(theme.colors.canvas),
        );

        let text_issue = audit
            .issues_of_kind(ThemeAccessibilityIssueKind::TextContrast)
            .find(|issue| {
                issue.role == Some(ComponentRole::Button)
                    && issue.state == Some(ComponentState::NORMAL)
            })
            .expect("button text contrast issue");
        assert!(text_issue.contrast_ratio.unwrap() < text_issue.required_ratio.unwrap());

        assert!(audit
            .issues_of_kind(ThemeAccessibilityIssueKind::IconContrast)
            .any(|issue| issue.role == Some(ComponentRole::Button)
                && issue.state == Some(ComponentState::NORMAL)));
    }

    #[test]
    fn theme_accessibility_audit_checks_preference_policy_against_baseline() {
        let baseline = Theme::dark();
        let preferences = AccessibilityPreferences::DEFAULT
            .reduced_motion(true)
            .forced_colors(true)
            .reduced_transparency(true)
            .text_scale(1.35);

        let stale = ThemeAccessibilityAudit::from_theme_with_preferences(
            &baseline,
            &baseline,
            preferences,
            ThemeAccessibilityAuditOptions::default(),
        );

        assert!(stale.has_issue(ThemeAccessibilityIssueKind::ReducedMotionPolicy));
        assert!(stale.has_issue(ThemeAccessibilityIssueKind::ForcedColorsPolicy));
        assert!(stale.has_issue(ThemeAccessibilityIssueKind::ReducedTransparencyPolicy));
        assert!(stale.has_issue(ThemeAccessibilityIssueKind::TextScalePolicy));
        assert!(!stale.has_issue(ThemeAccessibilityIssueKind::HighContrastPolicy));

        let adjusted = baseline.with_accessibility_preferences(preferences);
        let applied = ThemeAccessibilityAudit::from_theme_with_preferences(
            &baseline,
            &adjusted,
            preferences,
            ThemeAccessibilityAuditOptions::default(),
        );

        assert!(!applied.has_issue(ThemeAccessibilityIssueKind::ReducedMotionPolicy));
        assert!(!applied.has_issue(ThemeAccessibilityIssueKind::ForcedColorsPolicy));
        assert!(!applied.has_issue(ThemeAccessibilityIssueKind::ReducedTransparencyPolicy));
        assert!(!applied.has_issue(ThemeAccessibilityIssueKind::TextScalePolicy));
    }
}
