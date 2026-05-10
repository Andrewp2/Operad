//! Backend-neutral theme tokens for Operad v3.
//!
//! This module keeps product semantics out of Operad while giving consumers a
//! stable vocabulary for dense workstation-style UI surfaces. It intentionally
//! resolves to existing core primitives (`UiVisual`, `TextStyle`, `StrokeStyle`,
//! and `ColorRgba`) instead of renderer-specific paint objects.

use std::collections::{HashMap, HashSet};
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::{
    accessibility::AccessibilityPreferences, ColorRgba, FontFamily, FontStretch, FontStyle,
    FontWeight, StrokeStyle, TextStyle, TextWrap, UiVisual,
};

pub const OPERAD_DARK_THEME_NAME: &str = "operad.dark.v3";

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub colors: ColorTokens,
    pub spacing: SpacingTokens,
    pub typography: TypographyTokens,
    pub radius: RadiusTokens,
    pub stroke: StrokeTokens,
    pub effects: EffectTokens,
    pub opacity: OpacityTokens,
    pub motion: MotionTokens,
    pub components: ComponentTokens,
}

impl Theme {
    pub fn dark() -> Self {
        let colors = ColorTokens::dark();
        let spacing = SpacingTokens::dense();
        let typography = TypographyTokens::dark(&colors);
        let radius = RadiusTokens::default();
        let stroke = StrokeTokens::dark(&colors);
        let effects = EffectTokens::dark(&colors, &stroke);
        let opacity = OpacityTokens::default();
        let motion = MotionTokens::default();
        let components =
            ComponentTokens::dark(&colors, &spacing, &typography, &radius, &stroke, &opacity);

        Self {
            name: OPERAD_DARK_THEME_NAME,
            colors,
            spacing,
            typography,
            radius,
            stroke,
            effects,
            opacity,
            motion,
            components,
        }
    }

    pub fn component(&self, role: ComponentRole) -> &ComponentStyle {
        self.components.get(role)
    }

    pub fn resolve_visual(&self, role: ComponentRole, state: ComponentState) -> UiVisual {
        self.component(role).resolve_visual(state)
    }

    pub fn resolve_text(&self, role: ComponentRole, state: ComponentState) -> TextStyle {
        self.component(role).resolve_text(state)
    }

    pub fn resolve_icon(&self, role: ComponentRole, state: ComponentState) -> IconStyle {
        self.component(role).resolve_icon(state)
    }

    pub fn with_accessibility_preferences(&self, preferences: AccessibilityPreferences) -> Self {
        let mut theme = self.clone();
        theme.apply_accessibility_preferences(preferences);
        theme
    }

    pub fn apply_accessibility_preferences(&mut self, preferences: AccessibilityPreferences) {
        let text_scale = preferences.normalized_text_scale();
        if text_scale != AccessibilityPreferences::DEFAULT.text_scale {
            self.typography = typography_with_scale(&self.typography, text_scale);
            self.components = component_tokens_with_text_scale(&self.components, text_scale);
        }

        if preferences.should_reduce_motion() {
            self.motion = motion_tokens_with_reduced_motion(self.motion);
        }

        if preferences.should_use_high_contrast() {
            let source_colors = self.colors;
            self.colors = color_tokens_with_high_contrast(self.colors);
            self.stroke = stroke_tokens_with_high_contrast(self.stroke, &self.colors);
            self.typography =
                typography_with_high_contrast(&self.typography, &source_colors, &self.colors);
            self.components = component_tokens_with_high_contrast(
                &self.components,
                &source_colors,
                &self.colors,
                &self.stroke,
            );
        }

        if preferences.prefers_reduced_transparency() {
            self.colors = color_tokens_without_transparency(self.colors);
            self.opacity = opacity_tokens_without_transparency(self.opacity);
            self.effects = effect_tokens_without_transparency(self.effects);
            self.typography = typography_without_transparency(&self.typography);
            self.components = component_tokens_without_transparency(&self.components);
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThemeScopeId(String);

impl ThemeScopeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ThemeScopeId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for ThemeScopeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ThemeScopeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&ThemeScopeId> for ThemeScopeId {
    fn from(value: &ThemeScopeId) -> Self {
        value.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ThemeScopeKind {
    Shell,
    Panel,
    EditorSurface,
    Overlay,
    Menu,
    Tooltip,
    Custom(String),
}

impl ThemeScopeKind {
    pub fn custom(id: impl Into<String>) -> Self {
        Self::Custom(id.into())
    }

    pub const fn is_editor_surface(&self) -> bool {
        matches!(self, Self::EditorSurface)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThemePatch {
    pub colors: Option<ColorTokens>,
    pub spacing: Option<SpacingTokens>,
    pub typography: Option<TypographyTokens>,
    pub radius: Option<RadiusTokens>,
    pub stroke: Option<StrokeTokens>,
    pub effects: Option<EffectTokens>,
    pub opacity: Option<OpacityTokens>,
    pub motion: Option<MotionTokens>,
    pub components: Option<ComponentTokens>,
}

impl ThemePatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn colors(mut self, colors: ColorTokens) -> Self {
        self.colors = Some(colors);
        self
    }

    pub fn spacing(mut self, spacing: SpacingTokens) -> Self {
        self.spacing = Some(spacing);
        self
    }

    pub fn typography(mut self, typography: TypographyTokens) -> Self {
        self.typography = Some(typography);
        self
    }

    pub fn radius(mut self, radius: RadiusTokens) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn stroke(mut self, stroke: StrokeTokens) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub fn effects(mut self, effects: EffectTokens) -> Self {
        self.effects = Some(effects);
        self
    }

    pub fn opacity(mut self, opacity: OpacityTokens) -> Self {
        self.opacity = Some(opacity);
        self
    }

    pub fn motion(mut self, motion: MotionTokens) -> Self {
        self.motion = Some(motion);
        self
    }

    pub fn components(mut self, components: ComponentTokens) -> Self {
        self.components = Some(components);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.colors.is_none()
            && self.spacing.is_none()
            && self.typography.is_none()
            && self.radius.is_none()
            && self.stroke.is_none()
            && self.effects.is_none()
            && self.opacity.is_none()
            && self.motion.is_none()
            && self.components.is_none()
    }

    pub fn apply_to(&self, theme: &mut Theme) {
        let colors_changed = self.colors.is_some();
        let spacing_changed = self.spacing.is_some();
        let typography_changed = self.typography.is_some();
        let radius_changed = self.radius.is_some();
        let stroke_changed = self.stroke.is_some();
        let opacity_changed = self.opacity.is_some();

        if let Some(colors) = self.colors {
            theme.colors = colors;
        }

        if colors_changed && self.typography.is_none() {
            theme.typography = TypographyTokens::dark(&theme.colors);
        }

        if colors_changed && self.stroke.is_none() {
            theme.stroke = StrokeTokens::dark(&theme.colors);
        }

        if let Some(spacing) = self.spacing {
            theme.spacing = spacing;
        }

        if let Some(typography) = &self.typography {
            theme.typography = typography.clone();
        }

        if let Some(radius) = self.radius {
            theme.radius = radius;
        }

        if let Some(stroke) = self.stroke {
            theme.stroke = stroke;
        }

        if (colors_changed || stroke_changed) && self.effects.is_none() {
            theme.effects = EffectTokens::dark(&theme.colors, &theme.stroke);
        }

        if let Some(effects) = self.effects {
            theme.effects = effects;
        }

        if let Some(opacity) = self.opacity {
            theme.opacity = opacity;
        }

        if let Some(motion) = self.motion {
            theme.motion = motion;
        }

        let component_inputs_changed = colors_changed
            || spacing_changed
            || typography_changed
            || radius_changed
            || stroke_changed
            || opacity_changed;

        if component_inputs_changed && self.components.is_none() {
            theme.components = ComponentTokens::dark(
                &theme.colors,
                &theme.spacing,
                &theme.typography,
                &theme.radius,
                &theme.stroke,
                &theme.opacity,
            );
        }

        if let Some(components) = &self.components {
            theme.components = components.clone();
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeScope {
    pub id: ThemeScopeId,
    pub kind: ThemeScopeKind,
    pub parent: Option<ThemeScopeId>,
    pub patch: ThemePatch,
}

impl ThemeScope {
    pub fn new(id: impl Into<ThemeScopeId>, kind: ThemeScopeKind) -> Self {
        Self {
            id: id.into(),
            kind,
            parent: None,
            patch: ThemePatch::default(),
        }
    }

    pub fn shell(id: impl Into<ThemeScopeId>) -> Self {
        Self::new(id, ThemeScopeKind::Shell)
    }

    pub fn panel(id: impl Into<ThemeScopeId>) -> Self {
        Self::new(id, ThemeScopeKind::Panel)
    }

    pub fn editor_surface(id: impl Into<ThemeScopeId>) -> Self {
        Self::new(id, ThemeScopeKind::EditorSurface)
    }

    pub fn with_parent(mut self, parent: impl Into<ThemeScopeId>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    pub fn with_patch(mut self, patch: ThemePatch) -> Self {
        self.patch = patch;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeScopeError {
    MissingScope(ThemeScopeId),
    Cycle(Vec<ThemeScopeId>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScopedThemeRegistry {
    base: Theme,
    scopes: HashMap<ThemeScopeId, ThemeScope>,
}

impl ScopedThemeRegistry {
    pub fn new(base: Theme) -> Self {
        Self {
            base,
            scopes: HashMap::new(),
        }
    }

    pub fn base(&self) -> &Theme {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut Theme {
        &mut self.base
    }

    pub fn set_base(&mut self, base: Theme) {
        self.base = base;
    }

    pub fn insert(&mut self, scope: ThemeScope) -> Option<ThemeScope> {
        self.scopes.insert(scope.id.clone(), scope)
    }

    pub fn with_scope(mut self, scope: ThemeScope) -> Self {
        self.insert(scope);
        self
    }

    pub fn remove(&mut self, id: &ThemeScopeId) -> Option<ThemeScope> {
        self.scopes.remove(id)
    }

    pub fn scope(&self, id: &ThemeScopeId) -> Option<&ThemeScope> {
        self.scopes.get(id)
    }

    pub fn scopes(&self) -> impl Iterator<Item = &ThemeScope> {
        self.scopes.values()
    }

    pub fn resolve(&self, id: &ThemeScopeId) -> Result<Theme, ThemeScopeError> {
        let mut chain = Vec::new();
        let mut path = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(id.clone());

        while let Some(scope_id) = current {
            if !seen.insert(scope_id.clone()) {
                path.push(scope_id);
                return Err(ThemeScopeError::Cycle(path));
            }

            let scope = self
                .scopes
                .get(&scope_id)
                .ok_or_else(|| ThemeScopeError::MissingScope(scope_id.clone()))?;
            current = scope.parent.clone();
            path.push(scope_id);
            chain.push(scope);
        }

        let mut theme = self.base.clone();
        for scope in chain.iter().rev() {
            scope.patch.apply_to(&mut theme);
        }

        Ok(theme)
    }

    pub fn resolve_or_base(&self, id: &ThemeScopeId) -> Theme {
        self.resolve(id).unwrap_or_else(|_| self.base.clone())
    }
}

impl Default for ScopedThemeRegistry {
    fn default() -> Self {
        Self::new(Theme::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentRole {
    Button,
    Tab,
    SearchField,
    LaneHeader,
    RangeItem,
    EditorLane,
    PropertyRow,
    MenuRow,
    TransportControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ComponentState(u16);

impl ComponentState {
    const ALL_BITS: u16 = Self::HOVERED.0
        | Self::PRESSED.0
        | Self::FOCUSED.0
        | Self::SELECTED.0
        | Self::ACTIVE.0
        | Self::INVALID.0
        | Self::WARNING.0
        | Self::CHANGED.0
        | Self::PENDING.0
        | Self::OPEN.0
        | Self::CHECKED.0
        | Self::DISABLED.0;

    pub const NORMAL: Self = Self(0);
    pub const HOVERED: Self = Self(1 << 0);
    pub const PRESSED: Self = Self(1 << 1);
    pub const FOCUSED: Self = Self(1 << 2);
    pub const SELECTED: Self = Self(1 << 3);
    pub const ACTIVE: Self = Self(1 << 4);
    pub const INVALID: Self = Self(1 << 5);
    pub const WARNING: Self = Self(1 << 6);
    pub const CHANGED: Self = Self(1 << 7);
    pub const PENDING: Self = Self(1 << 8);
    pub const OPEN: Self = Self(1 << 9);
    pub const CHECKED: Self = Self(1 << 10);
    pub const DISABLED: Self = Self(1 << 11);

    pub const fn empty() -> Self {
        Self::NORMAL
    }

    pub const fn from_bits(bits: u16) -> Self {
        Self(bits & Self::ALL_BITS)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }

    pub const fn intersects(self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    pub const fn with(self, flag: Self) -> Self {
        Self::from_bits(self.0 | flag.0)
    }

    pub const fn without(self, flag: Self) -> Self {
        Self::from_bits(self.0 & !flag.0)
    }

    pub const fn hovered(self) -> bool {
        self.contains(Self::HOVERED)
    }

    pub const fn pressed(self) -> bool {
        self.contains(Self::PRESSED)
    }

    pub const fn focused(self) -> bool {
        self.contains(Self::FOCUSED)
    }

    pub const fn selected(self) -> bool {
        self.contains(Self::SELECTED)
    }

    pub const fn active(self) -> bool {
        self.contains(Self::ACTIVE)
    }

    pub const fn invalid(self) -> bool {
        self.contains(Self::INVALID)
    }

    pub const fn warning(self) -> bool {
        self.contains(Self::WARNING)
    }

    pub const fn changed(self) -> bool {
        self.contains(Self::CHANGED)
    }

    pub const fn pending(self) -> bool {
        self.contains(Self::PENDING)
    }

    pub const fn open(self) -> bool {
        self.contains(Self::OPEN)
    }

    pub const fn checked(self) -> bool {
        self.contains(Self::CHECKED)
    }

    pub const fn disabled(self) -> bool {
        self.contains(Self::DISABLED)
    }
}

impl BitOr for ComponentState {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::from_bits(self.0 | rhs.0)
    }
}

impl BitOrAssign for ComponentState {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitAnd for ComponentState {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::from_bits(self.0 & rhs.0)
    }
}

impl BitAndAssign for ComponentState {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl Not for ComponentState {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::from_bits(!self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentStateSlot {
    Base,
    Hovered,
    Pressed,
    Focused,
    Selected,
    Active,
    Invalid,
    Warning,
    Changed,
    Pending,
    Open,
    Checked,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorTokens {
    pub canvas: ColorRgba,
    pub canvas_subtle: ColorRgba,
    pub surface: ColorRgba,
    pub surface_muted: ColorRgba,
    pub surface_elevated: ColorRgba,
    pub surface_overlay: ColorRgba,
    pub surface_sunken: ColorRgba,
    pub border: ColorRgba,
    pub border_muted: ColorRgba,
    pub border_strong: ColorRgba,
    pub divider: ColorRgba,
    pub text: ColorRgba,
    pub text_muted: ColorRgba,
    pub text_subtle: ColorRgba,
    pub text_disabled: ColorRgba,
    pub text_inverse: ColorRgba,
    pub accent: ColorRgba,
    pub accent_hover: ColorRgba,
    pub accent_pressed: ColorRgba,
    pub accent_muted: ColorRgba,
    pub accent_strong: ColorRgba,
    pub accent_text: ColorRgba,
    pub success: ColorRgba,
    pub warning: ColorRgba,
    pub danger: ColorRgba,
    pub info: ColorRgba,
    pub selected: ColorRgba,
    pub selected_hover: ColorRgba,
    pub selected_text: ColorRgba,
    pub focus_ring: ColorRgba,
    pub overlay_scrim: ColorRgba,
    pub editor_background: ColorRgba,
    pub editor_grid_major: ColorRgba,
    pub editor_grid_minor: ColorRgba,
    pub lane_header: ColorRgba,
    pub lane_header_selected: ColorRgba,
    pub range_item_primary: ColorRgba,
    pub range_item_secondary: ColorRgba,
    pub range_item_accent: ColorRgba,
    pub editor_lane: ColorRgba,
    pub editor_lane_alternate: ColorRgba,
    pub transport_active: ColorRgba,
}

impl ColorTokens {
    pub const fn dark() -> Self {
        Self {
            canvas: ColorRgba::new(9, 12, 16, 255),
            canvas_subtle: ColorRgba::new(12, 16, 22, 255),
            surface: ColorRgba::new(18, 23, 31, 255),
            surface_muted: ColorRgba::new(23, 29, 39, 255),
            surface_elevated: ColorRgba::new(29, 36, 47, 255),
            surface_overlay: ColorRgba::new(38, 47, 61, 255),
            surface_sunken: ColorRgba::new(13, 17, 24, 255),
            border: ColorRgba::new(63, 75, 92, 255),
            border_muted: ColorRgba::new(43, 52, 65, 255),
            border_strong: ColorRgba::new(92, 108, 130, 255),
            divider: ColorRgba::new(35, 42, 53, 255),
            text: ColorRgba::new(232, 238, 246, 255),
            text_muted: ColorRgba::new(180, 190, 203, 255),
            text_subtle: ColorRgba::new(134, 148, 166, 255),
            text_disabled: ColorRgba::new(104, 116, 132, 185),
            text_inverse: ColorRgba::new(8, 12, 16, 255),
            accent: ColorRgba::new(99, 190, 255, 255),
            accent_hover: ColorRgba::new(132, 207, 255, 255),
            accent_pressed: ColorRgba::new(56, 146, 220, 255),
            accent_muted: ColorRgba::new(28, 67, 95, 255),
            accent_strong: ColorRgba::new(32, 131, 224, 255),
            accent_text: ColorRgba::new(225, 246, 255, 255),
            success: ColorRgba::new(83, 201, 147, 255),
            warning: ColorRgba::new(238, 183, 87, 255),
            danger: ColorRgba::new(240, 102, 124, 255),
            info: ColorRgba::new(127, 166, 255, 255),
            selected: ColorRgba::new(34, 70, 104, 255),
            selected_hover: ColorRgba::new(42, 84, 122, 255),
            selected_text: ColorRgba::new(233, 247, 255, 255),
            focus_ring: ColorRgba::new(124, 213, 255, 255),
            overlay_scrim: ColorRgba::new(3, 6, 10, 180),
            editor_background: ColorRgba::new(10, 13, 18, 255),
            editor_grid_major: ColorRgba::new(55, 66, 82, 255),
            editor_grid_minor: ColorRgba::new(31, 38, 49, 255),
            lane_header: ColorRgba::new(25, 31, 41, 255),
            lane_header_selected: ColorRgba::new(35, 55, 78, 255),
            range_item_primary: ColorRgba::new(62, 157, 184, 255),
            range_item_secondary: ColorRgba::new(116, 176, 98, 255),
            range_item_accent: ColorRgba::new(187, 126, 220, 255),
            editor_lane: ColorRgba::new(17, 22, 30, 255),
            editor_lane_alternate: ColorRgba::new(14, 18, 25, 255),
            transport_active: ColorRgba::new(92, 212, 165, 255),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingTokens {
    pub none: f32,
    pub xxxs: f32,
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
    pub control_x: f32,
    pub control_y: f32,
    pub panel: f32,
    pub toolbar_gap: f32,
    pub row_gap: f32,
    pub grid: f32,
}

impl SpacingTokens {
    pub const fn dense() -> Self {
        Self {
            none: 0.0,
            xxxs: 1.0,
            xxs: 2.0,
            xs: 4.0,
            sm: 6.0,
            md: 8.0,
            lg: 12.0,
            xl: 16.0,
            xxl: 24.0,
            control_x: 10.0,
            control_y: 6.0,
            panel: 12.0,
            toolbar_gap: 4.0,
            row_gap: 2.0,
            grid: 8.0,
        }
    }
}

impl Default for SpacingTokens {
    fn default() -> Self {
        Self::dense()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypographyTokens {
    pub caption: TextStyle,
    pub caption_strong: TextStyle,
    pub body: TextStyle,
    pub body_strong: TextStyle,
    pub label: TextStyle,
    pub label_strong: TextStyle,
    pub heading: TextStyle,
    pub title: TextStyle,
    pub mono: TextStyle,
    pub numeric: TextStyle,
    pub disabled: TextStyle,
}

impl TypographyTokens {
    pub fn dark(colors: &ColorTokens) -> Self {
        Self {
            caption: text_style(11.0, 14.0, FontWeight::NORMAL, colors.text_subtle),
            caption_strong: text_style(11.0, 14.0, FontWeight::BOLD, colors.text_muted),
            body: text_style(14.0, 20.0, FontWeight::NORMAL, colors.text),
            body_strong: text_style(14.0, 20.0, FontWeight::BOLD, colors.text),
            label: text_style(13.0, 18.0, FontWeight::NORMAL, colors.text_muted),
            label_strong: text_style(13.0, 18.0, FontWeight::BOLD, colors.text),
            heading: text_style(18.0, 24.0, FontWeight::BOLD, colors.text),
            title: text_style(24.0, 30.0, FontWeight::BOLD, colors.text),
            mono: mono_style(13.0, 18.0, FontWeight::NORMAL, colors.text_muted),
            numeric: mono_style(12.0, 16.0, FontWeight::NORMAL, colors.text),
            disabled: text_style(13.0, 18.0, FontWeight::NORMAL, colors.text_disabled),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusTokens {
    pub none: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub pill: f32,
}

impl Default for RadiusTokens {
    fn default() -> Self {
        Self {
            none: 0.0,
            xs: 2.0,
            sm: 4.0,
            md: 6.0,
            lg: 8.0,
            xl: 12.0,
            pill: 999.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeTokens {
    pub hairline_width: f32,
    pub thin_width: f32,
    pub medium_width: f32,
    pub strong_width: f32,
    pub divider: StrokeStyle,
    pub surface: StrokeStyle,
    pub surface_strong: StrokeStyle,
    pub control: StrokeStyle,
    pub control_hover: StrokeStyle,
    pub focus: StrokeStyle,
    pub selected: StrokeStyle,
    pub invalid: StrokeStyle,
    pub warning: StrokeStyle,
}

impl StrokeTokens {
    pub const fn dark(colors: &ColorTokens) -> Self {
        Self {
            hairline_width: 1.0,
            thin_width: 1.0,
            medium_width: 1.5,
            strong_width: 2.0,
            divider: StrokeStyle::new(colors.divider, 1.0),
            surface: StrokeStyle::new(colors.border_muted, 1.0),
            surface_strong: StrokeStyle::new(colors.border, 1.0),
            control: StrokeStyle::new(colors.border, 1.0),
            control_hover: StrokeStyle::new(colors.border_strong, 1.0),
            focus: StrokeStyle::new(colors.focus_ring, 1.5),
            selected: StrokeStyle::new(colors.accent, 1.0),
            invalid: StrokeStyle::new(colors.danger, 1.0),
            warning: StrokeStyle::new(colors.warning, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectTokens {
    pub panel_shadow: LayerEffect,
    pub floating_shadow: LayerEffect,
    pub popover_shadow: LayerEffect,
    pub focus_glow: LayerEffect,
    pub accent_glow: LayerEffect,
    pub danger_glow: LayerEffect,
    pub inset_hairline: LayerEffect,
}

impl EffectTokens {
    pub const fn dark(colors: &ColorTokens, stroke: &StrokeTokens) -> Self {
        Self {
            panel_shadow: LayerEffect {
                kind: LayerEffectKind::Shadow,
                color: ColorRgba::new(0, 0, 0, 255),
                offset_x: 0.0,
                offset_y: 6.0,
                blur_radius: 18.0,
                spread: -8.0,
                opacity: 0.38,
                fallback_stroke: Some(stroke.surface),
            },
            floating_shadow: LayerEffect {
                kind: LayerEffectKind::Shadow,
                color: ColorRgba::new(0, 0, 0, 255),
                offset_x: 0.0,
                offset_y: 14.0,
                blur_radius: 32.0,
                spread: -12.0,
                opacity: 0.5,
                fallback_stroke: Some(stroke.surface_strong),
            },
            popover_shadow: LayerEffect {
                kind: LayerEffectKind::Shadow,
                color: ColorRgba::new(0, 0, 0, 255),
                offset_x: 0.0,
                offset_y: 20.0,
                blur_radius: 48.0,
                spread: -16.0,
                opacity: 0.62,
                fallback_stroke: Some(stroke.surface_strong),
            },
            focus_glow: LayerEffect {
                kind: LayerEffectKind::Glow,
                color: colors.focus_ring,
                offset_x: 0.0,
                offset_y: 0.0,
                blur_radius: 10.0,
                spread: 0.0,
                opacity: 0.34,
                fallback_stroke: Some(stroke.focus),
            },
            accent_glow: LayerEffect {
                kind: LayerEffectKind::Glow,
                color: colors.accent,
                offset_x: 0.0,
                offset_y: 0.0,
                blur_radius: 16.0,
                spread: 0.0,
                opacity: 0.26,
                fallback_stroke: Some(stroke.selected),
            },
            danger_glow: LayerEffect {
                kind: LayerEffectKind::Glow,
                color: colors.danger,
                offset_x: 0.0,
                offset_y: 0.0,
                blur_radius: 14.0,
                spread: 0.0,
                opacity: 0.28,
                fallback_stroke: Some(stroke.invalid),
            },
            inset_hairline: LayerEffect {
                kind: LayerEffectKind::Inset,
                color: colors.border_muted,
                offset_x: 0.0,
                offset_y: 0.0,
                blur_radius: 0.0,
                spread: 1.0,
                opacity: 1.0,
                fallback_stroke: Some(stroke.surface),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerEffectKind {
    Shadow,
    Glow,
    Inset,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerEffect {
    pub kind: LayerEffectKind,
    pub color: ColorRgba,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread: f32,
    pub opacity: f32,
    pub fallback_stroke: Option<StrokeStyle>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpacityTokens {
    pub opaque: f32,
    pub hover_overlay: f32,
    pub pressed_overlay: f32,
    pub selected_overlay: f32,
    pub disabled: f32,
    pub muted: f32,
    pub scrim: f32,
    pub drag_preview: f32,
    pub focus_glow: f32,
}

impl Default for OpacityTokens {
    fn default() -> Self {
        Self {
            opaque: 1.0,
            hover_overlay: 0.1,
            pressed_overlay: 0.18,
            selected_overlay: 0.22,
            disabled: 0.46,
            muted: 0.68,
            scrim: 0.72,
            drag_preview: 0.82,
            focus_glow: 0.34,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionTokens {
    pub instant_ms: u16,
    pub micro_ms: u16,
    pub fast_ms: u16,
    pub normal_ms: u16,
    pub slow_ms: u16,
    pub tooltip_delay_ms: u16,
    pub standard: MotionCurve,
    pub emphasized: MotionCurve,
    pub exit: MotionCurve,
    pub reduced_motion_scale: f32,
}

impl Default for MotionTokens {
    fn default() -> Self {
        Self {
            instant_ms: 0,
            micro_ms: 70,
            fast_ms: 120,
            normal_ms: 180,
            slow_ms: 260,
            tooltip_delay_ms: 450,
            standard: MotionCurve::CubicBezier(0.2, 0.0, 0.0, 1.0),
            emphasized: MotionCurve::CubicBezier(0.2, 0.0, 0.0, 1.0),
            exit: MotionCurve::CubicBezier(0.4, 0.0, 1.0, 1.0),
            reduced_motion_scale: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionCurve {
    Linear,
    EaseOut,
    EaseInOut,
    CubicBezier(f32, f32, f32, f32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentTokens {
    pub button: ComponentStyle,
    pub tab: ComponentStyle,
    pub search_field: ComponentStyle,
    pub lane_header: ComponentStyle,
    pub range_item: ComponentStyle,
    pub editor_lane: ComponentStyle,
    pub property_row: ComponentStyle,
    pub menu_row: ComponentStyle,
    pub transport_control: ComponentStyle,
}

impl ComponentTokens {
    pub fn get(&self, role: ComponentRole) -> &ComponentStyle {
        match role {
            ComponentRole::Button => &self.button,
            ComponentRole::Tab => &self.tab,
            ComponentRole::SearchField => &self.search_field,
            ComponentRole::LaneHeader => &self.lane_header,
            ComponentRole::RangeItem => &self.range_item,
            ComponentRole::EditorLane => &self.editor_lane,
            ComponentRole::PropertyRow => &self.property_row,
            ComponentRole::MenuRow => &self.menu_row,
            ComponentRole::TransportControl => &self.transport_control,
        }
    }

    fn dark(
        colors: &ColorTokens,
        spacing: &SpacingTokens,
        typography: &TypographyTokens,
        radius: &RadiusTokens,
        stroke: &StrokeTokens,
        opacity: &OpacityTokens,
    ) -> Self {
        Self {
            button: button_tokens(colors, spacing, typography, radius, stroke, opacity),
            tab: tab_tokens(colors, spacing, typography, radius, stroke, opacity),
            search_field: search_field_tokens(colors, spacing, typography, radius, stroke, opacity),
            lane_header: lane_header_tokens(colors, spacing, typography, radius, stroke, opacity),
            range_item: range_item_tokens(colors, spacing, typography, radius, stroke, opacity),
            editor_lane: editor_lane_tokens(colors, spacing, typography, radius, stroke),
            property_row: property_row_tokens(colors, spacing, typography, radius, stroke, opacity),
            menu_row: menu_row_tokens(colors, spacing, typography, radius, stroke, opacity),
            transport_control: transport_tokens(
                colors, spacing, typography, radius, stroke, opacity,
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentStyle {
    pub visual: ComponentVisualStates,
    pub text: ComponentTextStates,
    pub icon: ComponentIconStates,
    pub layout: ComponentLayoutTokens,
}

impl ComponentStyle {
    pub fn resolve_visual(&self, state: ComponentState) -> UiVisual {
        self.visual.resolve(state)
    }

    pub fn resolve_text(&self, state: ComponentState) -> TextStyle {
        self.text.resolve(state)
    }

    pub fn resolve_icon(&self, state: ComponentState) -> IconStyle {
        self.icon.resolve(state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentLayoutTokens {
    pub min_width: f32,
    pub min_height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub gap: f32,
    pub icon_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentVisualStates {
    pub base: UiVisual,
    pub hovered: Option<UiVisual>,
    pub pressed: Option<UiVisual>,
    pub focused: Option<UiVisual>,
    pub selected: Option<UiVisual>,
    pub active: Option<UiVisual>,
    pub invalid: Option<UiVisual>,
    pub warning: Option<UiVisual>,
    pub changed: Option<UiVisual>,
    pub pending: Option<UiVisual>,
    pub open: Option<UiVisual>,
    pub checked: Option<UiVisual>,
    pub disabled: Option<UiVisual>,
}

impl ComponentVisualStates {
    pub const fn from_base(base: UiVisual) -> Self {
        Self {
            base,
            hovered: None,
            pressed: None,
            focused: None,
            selected: None,
            active: None,
            invalid: None,
            warning: None,
            changed: None,
            pending: None,
            open: None,
            checked: None,
            disabled: None,
        }
    }

    pub fn resolve(&self, state: ComponentState) -> UiVisual {
        self.resolve_slot(state).1
    }

    pub fn resolve_slot(&self, state: ComponentState) -> (ComponentStateSlot, UiVisual) {
        if state.disabled() {
            return (
                self.disabled
                    .map(|_| ComponentStateSlot::Disabled)
                    .unwrap_or(ComponentStateSlot::Base),
                self.disabled.unwrap_or(self.base),
            );
        }

        for (flag, slot, value) in self.state_values() {
            if state.contains(flag) {
                if let Some(visual) = value {
                    return (slot, visual);
                }
            }
        }

        (ComponentStateSlot::Base, self.base)
    }

    fn state_values(&self) -> [(ComponentState, ComponentStateSlot, Option<UiVisual>); 11] {
        [
            (
                ComponentState::INVALID,
                ComponentStateSlot::Invalid,
                self.invalid,
            ),
            (
                ComponentState::WARNING,
                ComponentStateSlot::Warning,
                self.warning,
            ),
            (
                ComponentState::PENDING,
                ComponentStateSlot::Pending,
                self.pending,
            ),
            (
                ComponentState::PRESSED,
                ComponentStateSlot::Pressed,
                self.pressed,
            ),
            (
                ComponentState::FOCUSED,
                ComponentStateSlot::Focused,
                self.focused,
            ),
            (
                ComponentState::ACTIVE,
                ComponentStateSlot::Active,
                self.active,
            ),
            (ComponentState::OPEN, ComponentStateSlot::Open, self.open),
            (
                ComponentState::CHECKED,
                ComponentStateSlot::Checked,
                self.checked,
            ),
            (
                ComponentState::SELECTED,
                ComponentStateSlot::Selected,
                self.selected,
            ),
            (
                ComponentState::CHANGED,
                ComponentStateSlot::Changed,
                self.changed,
            ),
            (
                ComponentState::HOVERED,
                ComponentStateSlot::Hovered,
                self.hovered,
            ),
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentTextStates {
    pub base: TextStyle,
    pub hovered: Option<TextStyle>,
    pub pressed: Option<TextStyle>,
    pub focused: Option<TextStyle>,
    pub selected: Option<TextStyle>,
    pub active: Option<TextStyle>,
    pub invalid: Option<TextStyle>,
    pub warning: Option<TextStyle>,
    pub changed: Option<TextStyle>,
    pub pending: Option<TextStyle>,
    pub open: Option<TextStyle>,
    pub checked: Option<TextStyle>,
    pub disabled: Option<TextStyle>,
}

impl ComponentTextStates {
    pub fn from_base(base: TextStyle) -> Self {
        Self {
            base,
            hovered: None,
            pressed: None,
            focused: None,
            selected: None,
            active: None,
            invalid: None,
            warning: None,
            changed: None,
            pending: None,
            open: None,
            checked: None,
            disabled: None,
        }
    }

    pub fn resolve(&self, state: ComponentState) -> TextStyle {
        self.resolve_slot(state).1
    }

    pub fn resolve_slot(&self, state: ComponentState) -> (ComponentStateSlot, TextStyle) {
        if state.disabled() {
            return (
                self.disabled
                    .as_ref()
                    .map(|_| ComponentStateSlot::Disabled)
                    .unwrap_or(ComponentStateSlot::Base),
                self.disabled.clone().unwrap_or_else(|| self.base.clone()),
            );
        }

        for (flag, slot, value) in self.state_values() {
            if state.contains(flag) {
                if let Some(style) = value {
                    return (slot, style);
                }
            }
        }

        (ComponentStateSlot::Base, self.base.clone())
    }

    fn state_values(&self) -> [(ComponentState, ComponentStateSlot, Option<TextStyle>); 11] {
        [
            (
                ComponentState::INVALID,
                ComponentStateSlot::Invalid,
                self.invalid.clone(),
            ),
            (
                ComponentState::WARNING,
                ComponentStateSlot::Warning,
                self.warning.clone(),
            ),
            (
                ComponentState::PENDING,
                ComponentStateSlot::Pending,
                self.pending.clone(),
            ),
            (
                ComponentState::PRESSED,
                ComponentStateSlot::Pressed,
                self.pressed.clone(),
            ),
            (
                ComponentState::FOCUSED,
                ComponentStateSlot::Focused,
                self.focused.clone(),
            ),
            (
                ComponentState::ACTIVE,
                ComponentStateSlot::Active,
                self.active.clone(),
            ),
            (
                ComponentState::OPEN,
                ComponentStateSlot::Open,
                self.open.clone(),
            ),
            (
                ComponentState::CHECKED,
                ComponentStateSlot::Checked,
                self.checked.clone(),
            ),
            (
                ComponentState::SELECTED,
                ComponentStateSlot::Selected,
                self.selected.clone(),
            ),
            (
                ComponentState::CHANGED,
                ComponentStateSlot::Changed,
                self.changed.clone(),
            ),
            (
                ComponentState::HOVERED,
                ComponentStateSlot::Hovered,
                self.hovered.clone(),
            ),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconStyle {
    pub tint: ColorRgba,
    pub opacity: f32,
}

impl IconStyle {
    pub const fn new(tint: ColorRgba, opacity: f32) -> Self {
        Self { tint, opacity }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentIconStates {
    pub base: IconStyle,
    pub hovered: Option<IconStyle>,
    pub pressed: Option<IconStyle>,
    pub focused: Option<IconStyle>,
    pub selected: Option<IconStyle>,
    pub active: Option<IconStyle>,
    pub invalid: Option<IconStyle>,
    pub warning: Option<IconStyle>,
    pub changed: Option<IconStyle>,
    pub pending: Option<IconStyle>,
    pub open: Option<IconStyle>,
    pub checked: Option<IconStyle>,
    pub disabled: Option<IconStyle>,
}

impl ComponentIconStates {
    pub const fn from_base(base: IconStyle) -> Self {
        Self {
            base,
            hovered: None,
            pressed: None,
            focused: None,
            selected: None,
            active: None,
            invalid: None,
            warning: None,
            changed: None,
            pending: None,
            open: None,
            checked: None,
            disabled: None,
        }
    }

    pub fn resolve(&self, state: ComponentState) -> IconStyle {
        self.resolve_slot(state).1
    }

    pub fn resolve_slot(&self, state: ComponentState) -> (ComponentStateSlot, IconStyle) {
        if state.disabled() {
            return (
                self.disabled
                    .map(|_| ComponentStateSlot::Disabled)
                    .unwrap_or(ComponentStateSlot::Base),
                self.disabled.unwrap_or(self.base),
            );
        }

        for (flag, slot, value) in self.state_values() {
            if state.contains(flag) {
                if let Some(style) = value {
                    return (slot, style);
                }
            }
        }

        (ComponentStateSlot::Base, self.base)
    }

    fn state_values(&self) -> [(ComponentState, ComponentStateSlot, Option<IconStyle>); 11] {
        [
            (
                ComponentState::INVALID,
                ComponentStateSlot::Invalid,
                self.invalid,
            ),
            (
                ComponentState::WARNING,
                ComponentStateSlot::Warning,
                self.warning,
            ),
            (
                ComponentState::PENDING,
                ComponentStateSlot::Pending,
                self.pending,
            ),
            (
                ComponentState::PRESSED,
                ComponentStateSlot::Pressed,
                self.pressed,
            ),
            (
                ComponentState::FOCUSED,
                ComponentStateSlot::Focused,
                self.focused,
            ),
            (
                ComponentState::ACTIVE,
                ComponentStateSlot::Active,
                self.active,
            ),
            (ComponentState::OPEN, ComponentStateSlot::Open, self.open),
            (
                ComponentState::CHECKED,
                ComponentStateSlot::Checked,
                self.checked,
            ),
            (
                ComponentState::SELECTED,
                ComponentStateSlot::Selected,
                self.selected,
            ),
            (
                ComponentState::CHANGED,
                ComponentStateSlot::Changed,
                self.changed,
            ),
            (
                ComponentState::HOVERED,
                ComponentStateSlot::Hovered,
                self.hovered,
            ),
        ]
    }
}

pub fn text_style_with_color(style: &TextStyle, color: ColorRgba) -> TextStyle {
    let mut next = style.clone();
    next.color = color;
    next
}

pub fn text_style_with_scale(style: &TextStyle, scale: f32) -> TextStyle {
    let scale = normalized_text_scale(scale);
    let mut next = style.clone();
    next.font_size *= scale;
    next.line_height *= scale;
    next
}

pub const fn color_with_alpha(color: ColorRgba, alpha: u8) -> ColorRgba {
    ColorRgba::new(color.r, color.g, color.b, alpha)
}

fn normalized_text_scale(scale: f32) -> f32 {
    if scale.is_finite() {
        scale.clamp(
            AccessibilityPreferences::MIN_TEXT_SCALE,
            AccessibilityPreferences::MAX_TEXT_SCALE,
        )
    } else {
        AccessibilityPreferences::DEFAULT.text_scale
    }
}

fn color_without_transparency(color: ColorRgba) -> ColorRgba {
    if color.a == 0 || color.a == 255 {
        color
    } else {
        color_with_alpha(color, 255)
    }
}

fn opacity_without_transparency(opacity: f32) -> f32 {
    if !opacity.is_finite() {
        1.0
    } else if opacity <= 0.0 {
        0.0
    } else {
        1.0
    }
}

fn width_at_least(width: f32, minimum: f32) -> f32 {
    if width.is_finite() {
        width.max(minimum)
    } else {
        minimum
    }
}

fn stroke_with_min_width(stroke: StrokeStyle, minimum: f32) -> StrokeStyle {
    StrokeStyle::new(
        color_without_transparency(stroke.color),
        width_at_least(stroke.width, minimum),
    )
}

fn visual_without_transparency(visual: UiVisual) -> UiVisual {
    UiVisual {
        fill: color_without_transparency(visual.fill),
        stroke: visual
            .stroke
            .map(|stroke| stroke_with_min_width(stroke, stroke.width)),
        corner_radius: visual.corner_radius,
    }
}

fn visual_with_high_contrast(visual: UiVisual, stroke: &StrokeTokens) -> UiVisual {
    UiVisual {
        fill: color_without_transparency(visual.fill),
        stroke: visual
            .stroke
            .map(|value| stroke_with_min_width(value, stroke.thin_width)),
        corner_radius: visual.corner_radius,
    }
}

fn focused_visual_with_high_contrast(visual: UiVisual, stroke: &StrokeTokens) -> UiVisual {
    UiVisual {
        stroke: Some(stroke.focus),
        ..visual_with_high_contrast(visual, stroke)
    }
}

fn text_color_with_high_contrast(
    color: ColorRgba,
    source: &ColorTokens,
    contrast: &ColorTokens,
) -> ColorRgba {
    if color == source.text_muted
        || color == source.text_subtle
        || color == source.text_disabled
        || (color.a > 0 && color.a < 220)
    {
        contrast.text
    } else {
        color_without_transparency(color)
    }
}

fn text_style_without_transparency(style: &TextStyle) -> TextStyle {
    text_style_with_color(style, color_without_transparency(style.color))
}

fn text_style_with_high_contrast(
    style: &TextStyle,
    source: &ColorTokens,
    contrast: &ColorTokens,
) -> TextStyle {
    text_style_with_color(
        style,
        text_color_with_high_contrast(style.color, source, contrast),
    )
}

fn typography_with_scale(tokens: &TypographyTokens, scale: f32) -> TypographyTokens {
    TypographyTokens {
        caption: text_style_with_scale(&tokens.caption, scale),
        caption_strong: text_style_with_scale(&tokens.caption_strong, scale),
        body: text_style_with_scale(&tokens.body, scale),
        body_strong: text_style_with_scale(&tokens.body_strong, scale),
        label: text_style_with_scale(&tokens.label, scale),
        label_strong: text_style_with_scale(&tokens.label_strong, scale),
        heading: text_style_with_scale(&tokens.heading, scale),
        title: text_style_with_scale(&tokens.title, scale),
        mono: text_style_with_scale(&tokens.mono, scale),
        numeric: text_style_with_scale(&tokens.numeric, scale),
        disabled: text_style_with_scale(&tokens.disabled, scale),
    }
}

fn typography_without_transparency(tokens: &TypographyTokens) -> TypographyTokens {
    TypographyTokens {
        caption: text_style_without_transparency(&tokens.caption),
        caption_strong: text_style_without_transparency(&tokens.caption_strong),
        body: text_style_without_transparency(&tokens.body),
        body_strong: text_style_without_transparency(&tokens.body_strong),
        label: text_style_without_transparency(&tokens.label),
        label_strong: text_style_without_transparency(&tokens.label_strong),
        heading: text_style_without_transparency(&tokens.heading),
        title: text_style_without_transparency(&tokens.title),
        mono: text_style_without_transparency(&tokens.mono),
        numeric: text_style_without_transparency(&tokens.numeric),
        disabled: text_style_without_transparency(&tokens.disabled),
    }
}

fn typography_with_high_contrast(
    tokens: &TypographyTokens,
    source: &ColorTokens,
    contrast: &ColorTokens,
) -> TypographyTokens {
    TypographyTokens {
        caption: text_style_with_high_contrast(&tokens.caption, source, contrast),
        caption_strong: text_style_with_high_contrast(&tokens.caption_strong, source, contrast),
        body: text_style_with_high_contrast(&tokens.body, source, contrast),
        body_strong: text_style_with_high_contrast(&tokens.body_strong, source, contrast),
        label: text_style_with_high_contrast(&tokens.label, source, contrast),
        label_strong: text_style_with_high_contrast(&tokens.label_strong, source, contrast),
        heading: text_style_with_high_contrast(&tokens.heading, source, contrast),
        title: text_style_with_high_contrast(&tokens.title, source, contrast),
        mono: text_style_with_high_contrast(&tokens.mono, source, contrast),
        numeric: text_style_with_high_contrast(&tokens.numeric, source, contrast),
        disabled: text_style_with_high_contrast(&tokens.disabled, source, contrast),
    }
}

fn motion_tokens_with_reduced_motion(mut motion: MotionTokens) -> MotionTokens {
    motion.micro_ms = scaled_motion_duration(motion.micro_ms, motion);
    motion.fast_ms = scaled_motion_duration(motion.fast_ms, motion);
    motion.normal_ms = scaled_motion_duration(motion.normal_ms, motion);
    motion.slow_ms = scaled_motion_duration(motion.slow_ms, motion);
    motion.standard = MotionCurve::Linear;
    motion.emphasized = MotionCurve::Linear;
    motion.exit = MotionCurve::Linear;
    motion
}

fn scaled_motion_duration(duration: u16, motion: MotionTokens) -> u16 {
    let scale = motion.reduced_motion_scale;
    if !scale.is_finite() || scale <= 0.0 {
        return motion.instant_ms;
    }

    ((duration as f32) * scale)
        .round()
        .clamp(motion.instant_ms as f32, u16::MAX as f32) as u16
}

fn color_tokens_without_transparency(mut colors: ColorTokens) -> ColorTokens {
    colors.canvas = color_without_transparency(colors.canvas);
    colors.canvas_subtle = color_without_transparency(colors.canvas_subtle);
    colors.surface = color_without_transparency(colors.surface);
    colors.surface_muted = color_without_transparency(colors.surface_muted);
    colors.surface_elevated = color_without_transparency(colors.surface_elevated);
    colors.surface_overlay = color_without_transparency(colors.surface_overlay);
    colors.surface_sunken = color_without_transparency(colors.surface_sunken);
    colors.border = color_without_transparency(colors.border);
    colors.border_muted = color_without_transparency(colors.border_muted);
    colors.border_strong = color_without_transparency(colors.border_strong);
    colors.divider = color_without_transparency(colors.divider);
    colors.text = color_without_transparency(colors.text);
    colors.text_muted = color_without_transparency(colors.text_muted);
    colors.text_subtle = color_without_transparency(colors.text_subtle);
    colors.text_disabled = color_without_transparency(colors.text_disabled);
    colors.text_inverse = color_without_transparency(colors.text_inverse);
    colors.accent = color_without_transparency(colors.accent);
    colors.accent_hover = color_without_transparency(colors.accent_hover);
    colors.accent_pressed = color_without_transparency(colors.accent_pressed);
    colors.accent_muted = color_without_transparency(colors.accent_muted);
    colors.accent_strong = color_without_transparency(colors.accent_strong);
    colors.accent_text = color_without_transparency(colors.accent_text);
    colors.success = color_without_transparency(colors.success);
    colors.warning = color_without_transparency(colors.warning);
    colors.danger = color_without_transparency(colors.danger);
    colors.info = color_without_transparency(colors.info);
    colors.selected = color_without_transparency(colors.selected);
    colors.selected_hover = color_without_transparency(colors.selected_hover);
    colors.selected_text = color_without_transparency(colors.selected_text);
    colors.focus_ring = color_without_transparency(colors.focus_ring);
    colors.overlay_scrim = color_without_transparency(colors.overlay_scrim);
    colors.editor_background = color_without_transparency(colors.editor_background);
    colors.editor_grid_major = color_without_transparency(colors.editor_grid_major);
    colors.editor_grid_minor = color_without_transparency(colors.editor_grid_minor);
    colors.lane_header = color_without_transparency(colors.lane_header);
    colors.lane_header_selected = color_without_transparency(colors.lane_header_selected);
    colors.range_item_primary = color_without_transparency(colors.range_item_primary);
    colors.range_item_secondary = color_without_transparency(colors.range_item_secondary);
    colors.range_item_accent = color_without_transparency(colors.range_item_accent);
    colors.editor_lane = color_without_transparency(colors.editor_lane);
    colors.editor_lane_alternate = color_without_transparency(colors.editor_lane_alternate);
    colors.transport_active = color_without_transparency(colors.transport_active);
    colors
}

fn color_tokens_with_high_contrast(mut colors: ColorTokens) -> ColorTokens {
    colors.canvas_subtle = colors.canvas;
    colors.surface_muted = colors.surface_elevated;
    colors.surface_overlay = colors.surface_elevated;
    colors.border_muted = colors.border;
    colors.divider = colors.border_strong;
    colors.text_muted = colors.text;
    colors.text_subtle = colors.text;
    colors.text_disabled = colors.text;
    colors.focus_ring = color_without_transparency(colors.focus_ring);
    colors.overlay_scrim = color_with_alpha(colors.overlay_scrim, colors.overlay_scrim.a.max(220));
    colors.editor_grid_minor = colors.border_muted;
    colors
}

fn stroke_tokens_with_high_contrast(
    mut stroke: StrokeTokens,
    colors: &ColorTokens,
) -> StrokeTokens {
    stroke.hairline_width = width_at_least(stroke.hairline_width, 1.0);
    stroke.thin_width = width_at_least(stroke.thin_width, 1.5);
    stroke.medium_width = width_at_least(stroke.medium_width, 2.0);
    stroke.strong_width = width_at_least(stroke.strong_width, 2.5);
    stroke.divider = StrokeStyle::new(colors.border_strong, stroke.thin_width);
    stroke.surface = StrokeStyle::new(colors.border, stroke.thin_width);
    stroke.surface_strong = StrokeStyle::new(colors.border_strong, stroke.medium_width);
    stroke.control = StrokeStyle::new(colors.border_strong, stroke.thin_width);
    stroke.control_hover = StrokeStyle::new(colors.focus_ring, stroke.medium_width);
    stroke.focus = StrokeStyle::new(colors.focus_ring, stroke.strong_width);
    stroke.selected = StrokeStyle::new(colors.selected_text, stroke.medium_width);
    stroke.invalid = StrokeStyle::new(colors.danger, stroke.medium_width);
    stroke.warning = StrokeStyle::new(colors.warning, stroke.medium_width);
    stroke
}

fn opacity_tokens_without_transparency(mut opacity: OpacityTokens) -> OpacityTokens {
    opacity.opaque = 1.0;
    opacity.hover_overlay = opacity_without_transparency(opacity.hover_overlay);
    opacity.pressed_overlay = opacity_without_transparency(opacity.pressed_overlay);
    opacity.selected_overlay = opacity_without_transparency(opacity.selected_overlay);
    opacity.disabled = opacity_without_transparency(opacity.disabled);
    opacity.muted = opacity_without_transparency(opacity.muted);
    opacity.scrim = opacity_without_transparency(opacity.scrim);
    opacity.drag_preview = opacity_without_transparency(opacity.drag_preview);
    opacity.focus_glow = opacity_without_transparency(opacity.focus_glow);
    opacity
}

fn effect_without_transparency(mut effect: LayerEffect) -> LayerEffect {
    effect.color = color_without_transparency(effect.color);
    effect.opacity = opacity_without_transparency(effect.opacity);
    effect.fallback_stroke = effect
        .fallback_stroke
        .map(|stroke| stroke_with_min_width(stroke, stroke.width));
    effect
}

fn effect_tokens_without_transparency(effects: EffectTokens) -> EffectTokens {
    EffectTokens {
        panel_shadow: effect_without_transparency(effects.panel_shadow),
        floating_shadow: effect_without_transparency(effects.floating_shadow),
        popover_shadow: effect_without_transparency(effects.popover_shadow),
        focus_glow: effect_without_transparency(effects.focus_glow),
        accent_glow: effect_without_transparency(effects.accent_glow),
        danger_glow: effect_without_transparency(effects.danger_glow),
        inset_hairline: effect_without_transparency(effects.inset_hairline),
    }
}

fn map_component_tokens(
    tokens: &ComponentTokens,
    mut map: impl FnMut(&ComponentStyle) -> ComponentStyle,
) -> ComponentTokens {
    ComponentTokens {
        button: map(&tokens.button),
        tab: map(&tokens.tab),
        search_field: map(&tokens.search_field),
        lane_header: map(&tokens.lane_header),
        range_item: map(&tokens.range_item),
        editor_lane: map(&tokens.editor_lane),
        property_row: map(&tokens.property_row),
        menu_row: map(&tokens.menu_row),
        transport_control: map(&tokens.transport_control),
    }
}

fn component_tokens_with_text_scale(tokens: &ComponentTokens, scale: f32) -> ComponentTokens {
    map_component_tokens(tokens, |style| ComponentStyle {
        text: text_states_with_scale(&style.text, scale),
        ..style.clone()
    })
}

fn component_tokens_without_transparency(tokens: &ComponentTokens) -> ComponentTokens {
    map_component_tokens(tokens, component_style_without_transparency)
}

fn component_tokens_with_high_contrast(
    tokens: &ComponentTokens,
    source: &ColorTokens,
    contrast: &ColorTokens,
    stroke: &StrokeTokens,
) -> ComponentTokens {
    map_component_tokens(tokens, |style| {
        component_style_with_high_contrast(style, source, contrast, stroke)
    })
}

fn text_states_with_scale(states: &ComponentTextStates, scale: f32) -> ComponentTextStates {
    ComponentTextStates {
        base: text_style_with_scale(&states.base, scale),
        hovered: states
            .hovered
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        pressed: states
            .pressed
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        focused: states
            .focused
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        selected: states
            .selected
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        active: states
            .active
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        invalid: states
            .invalid
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        warning: states
            .warning
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        changed: states
            .changed
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        pending: states
            .pending
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        open: states
            .open
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        checked: states
            .checked
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
        disabled: states
            .disabled
            .as_ref()
            .map(|style| text_style_with_scale(style, scale)),
    }
}

fn text_states_without_transparency(states: &ComponentTextStates) -> ComponentTextStates {
    ComponentTextStates {
        base: text_style_without_transparency(&states.base),
        hovered: states.hovered.as_ref().map(text_style_without_transparency),
        pressed: states.pressed.as_ref().map(text_style_without_transparency),
        focused: states.focused.as_ref().map(text_style_without_transparency),
        selected: states
            .selected
            .as_ref()
            .map(text_style_without_transparency),
        active: states.active.as_ref().map(text_style_without_transparency),
        invalid: states.invalid.as_ref().map(text_style_without_transparency),
        warning: states.warning.as_ref().map(text_style_without_transparency),
        changed: states.changed.as_ref().map(text_style_without_transparency),
        pending: states.pending.as_ref().map(text_style_without_transparency),
        open: states.open.as_ref().map(text_style_without_transparency),
        checked: states.checked.as_ref().map(text_style_without_transparency),
        disabled: states
            .disabled
            .as_ref()
            .map(text_style_without_transparency),
    }
}

fn text_states_with_high_contrast(
    states: &ComponentTextStates,
    source: &ColorTokens,
    contrast: &ColorTokens,
) -> ComponentTextStates {
    ComponentTextStates {
        base: text_style_with_high_contrast(&states.base, source, contrast),
        hovered: states
            .hovered
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        pressed: states
            .pressed
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        focused: states
            .focused
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        selected: states
            .selected
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        active: states
            .active
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        invalid: states
            .invalid
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        warning: states
            .warning
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        changed: states
            .changed
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        pending: states
            .pending
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        open: states
            .open
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        checked: states
            .checked
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
        disabled: states
            .disabled
            .as_ref()
            .map(|style| text_style_with_high_contrast(style, source, contrast)),
    }
}

fn visual_states_without_transparency(states: ComponentVisualStates) -> ComponentVisualStates {
    ComponentVisualStates {
        base: visual_without_transparency(states.base),
        hovered: states.hovered.map(visual_without_transparency),
        pressed: states.pressed.map(visual_without_transparency),
        focused: states.focused.map(visual_without_transparency),
        selected: states.selected.map(visual_without_transparency),
        active: states.active.map(visual_without_transparency),
        invalid: states.invalid.map(visual_without_transparency),
        warning: states.warning.map(visual_without_transparency),
        changed: states.changed.map(visual_without_transparency),
        pending: states.pending.map(visual_without_transparency),
        open: states.open.map(visual_without_transparency),
        checked: states.checked.map(visual_without_transparency),
        disabled: states.disabled.map(visual_without_transparency),
    }
}

fn visual_states_with_high_contrast(
    states: ComponentVisualStates,
    stroke: &StrokeTokens,
) -> ComponentVisualStates {
    ComponentVisualStates {
        base: visual_with_high_contrast(states.base, stroke),
        hovered: states
            .hovered
            .map(|visual| visual_with_high_contrast(visual, stroke)),
        pressed: states
            .pressed
            .map(|visual| visual_with_high_contrast(visual, stroke)),
        focused: states
            .focused
            .map(|visual| focused_visual_with_high_contrast(visual, stroke)),
        selected: states
            .selected
            .map(|visual| visual_with_high_contrast(visual, stroke)),
        active: states
            .active
            .map(|visual| visual_with_high_contrast(visual, stroke)),
        invalid: states
            .invalid
            .map(|visual| visual_with_high_contrast(visual, stroke)),
        warning: states
            .warning
            .map(|visual| visual_with_high_contrast(visual, stroke)),
        changed: states
            .changed
            .map(|visual| visual_with_high_contrast(visual, stroke)),
        pending: states
            .pending
            .map(|visual| visual_with_high_contrast(visual, stroke)),
        open: states
            .open
            .map(|visual| visual_with_high_contrast(visual, stroke)),
        checked: states
            .checked
            .map(|visual| visual_with_high_contrast(visual, stroke)),
        disabled: states
            .disabled
            .map(|visual| visual_with_high_contrast(visual, stroke)),
    }
}

fn icon_without_transparency(icon: IconStyle) -> IconStyle {
    IconStyle {
        tint: color_without_transparency(icon.tint),
        opacity: opacity_without_transparency(icon.opacity),
    }
}

fn icon_with_high_contrast(
    icon: IconStyle,
    source: &ColorTokens,
    contrast: &ColorTokens,
) -> IconStyle {
    IconStyle {
        tint: text_color_with_high_contrast(icon.tint, source, contrast),
        opacity: opacity_without_transparency(icon.opacity),
    }
}

fn icon_states_without_transparency(states: ComponentIconStates) -> ComponentIconStates {
    ComponentIconStates {
        base: icon_without_transparency(states.base),
        hovered: states.hovered.map(icon_without_transparency),
        pressed: states.pressed.map(icon_without_transparency),
        focused: states.focused.map(icon_without_transparency),
        selected: states.selected.map(icon_without_transparency),
        active: states.active.map(icon_without_transparency),
        invalid: states.invalid.map(icon_without_transparency),
        warning: states.warning.map(icon_without_transparency),
        changed: states.changed.map(icon_without_transparency),
        pending: states.pending.map(icon_without_transparency),
        open: states.open.map(icon_without_transparency),
        checked: states.checked.map(icon_without_transparency),
        disabled: states.disabled.map(icon_without_transparency),
    }
}

fn icon_states_with_high_contrast(
    states: ComponentIconStates,
    source: &ColorTokens,
    contrast: &ColorTokens,
) -> ComponentIconStates {
    ComponentIconStates {
        base: icon_with_high_contrast(states.base, source, contrast),
        hovered: states
            .hovered
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        pressed: states
            .pressed
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        focused: states
            .focused
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        selected: states
            .selected
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        active: states
            .active
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        invalid: states
            .invalid
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        warning: states
            .warning
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        changed: states
            .changed
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        pending: states
            .pending
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        open: states
            .open
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        checked: states
            .checked
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
        disabled: states
            .disabled
            .map(|icon| icon_with_high_contrast(icon, source, contrast)),
    }
}

fn component_style_without_transparency(style: &ComponentStyle) -> ComponentStyle {
    ComponentStyle {
        visual: visual_states_without_transparency(style.visual),
        text: text_states_without_transparency(&style.text),
        icon: icon_states_without_transparency(style.icon),
        layout: style.layout,
    }
}

fn component_style_with_high_contrast(
    style: &ComponentStyle,
    source: &ColorTokens,
    contrast: &ColorTokens,
    stroke: &StrokeTokens,
) -> ComponentStyle {
    ComponentStyle {
        visual: visual_states_with_high_contrast(style.visual, stroke),
        text: text_states_with_high_contrast(&style.text, source, contrast),
        icon: icon_states_with_high_contrast(style.icon, source, contrast),
        layout: style.layout,
    }
}

fn button_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
    opacity: &OpacityTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(colors.surface_elevated, Some(stroke.control), radius.sm);

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(
                colors.surface_overlay,
                Some(stroke.control_hover),
                radius.sm,
            )),
            pressed: Some(UiVisual::panel(
                colors.accent_muted,
                Some(stroke.selected),
                radius.sm,
            )),
            focused: Some(UiVisual::panel(
                colors.surface_overlay,
                Some(stroke.focus),
                radius.sm,
            )),
            active: Some(UiVisual::panel(
                colors.accent_strong,
                Some(stroke.selected),
                radius.sm,
            )),
            selected: Some(UiVisual::panel(
                colors.selected,
                Some(stroke.selected),
                radius.sm,
            )),
            invalid: Some(UiVisual::panel(
                colors.surface_elevated,
                Some(stroke.invalid),
                radius.sm,
            )),
            warning: Some(UiVisual::panel(
                colors.surface_elevated,
                Some(stroke.warning),
                radius.sm,
            )),
            pending: Some(UiVisual::panel(
                colors.surface_overlay,
                Some(stroke.selected),
                radius.sm,
            )),
            disabled: Some(UiVisual::panel(
                color_with_alpha(colors.surface_muted, 170),
                Some(StrokeStyle::new(
                    color_with_alpha(colors.border_muted, 150),
                    stroke.thin_width,
                )),
                radius.sm,
            )),
            ..ComponentVisualStates::from_base(base)
        },
        text: ComponentTextStates {
            selected: Some(text_style_with_color(
                &typography.label_strong,
                colors.selected_text,
            )),
            active: Some(text_style_with_color(
                &typography.label_strong,
                colors.accent_text,
            )),
            invalid: Some(text_style_with_color(
                &typography.label_strong,
                colors.danger,
            )),
            warning: Some(text_style_with_color(
                &typography.label_strong,
                colors.warning,
            )),
            disabled: Some(text_style_with_color(
                &typography.label,
                color_with_alpha(colors.text_disabled, 190),
            )),
            ..ComponentTextStates::from_base(typography.label_strong.clone())
        },
        icon: ComponentIconStates {
            hovered: Some(IconStyle::new(colors.text, opacity.opaque)),
            pressed: Some(IconStyle::new(colors.accent_text, opacity.opaque)),
            selected: Some(IconStyle::new(colors.selected_text, opacity.opaque)),
            active: Some(IconStyle::new(colors.accent_text, opacity.opaque)),
            invalid: Some(IconStyle::new(colors.danger, opacity.opaque)),
            warning: Some(IconStyle::new(colors.warning, opacity.opaque)),
            disabled: Some(IconStyle::new(colors.text_disabled, opacity.disabled)),
            ..ComponentIconStates::from_base(IconStyle::new(colors.text_muted, opacity.opaque))
        },
        layout: ComponentLayoutTokens {
            min_width: 28.0,
            min_height: 28.0,
            padding_x: spacing.control_x,
            padding_y: spacing.control_y,
            gap: spacing.xs,
            icon_size: 16.0,
        },
    }
}

fn tab_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
    opacity: &OpacityTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(colors.surface_sunken, Some(stroke.divider), radius.sm);

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(
                colors.surface_muted,
                Some(stroke.surface),
                radius.sm,
            )),
            selected: Some(UiVisual::panel(
                colors.surface_elevated,
                Some(stroke.selected),
                radius.sm,
            )),
            focused: Some(UiVisual::panel(
                colors.surface_elevated,
                Some(stroke.focus),
                radius.sm,
            )),
            active: Some(UiVisual::panel(
                colors.selected,
                Some(stroke.selected),
                radius.sm,
            )),
            disabled: Some(UiVisual::panel(
                color_with_alpha(colors.surface_sunken, 160),
                None,
                radius.sm,
            )),
            ..ComponentVisualStates::from_base(base)
        },
        text: ComponentTextStates {
            selected: Some(text_style_with_color(
                &typography.label_strong,
                colors.selected_text,
            )),
            active: Some(text_style_with_color(
                &typography.label_strong,
                colors.accent_text,
            )),
            disabled: Some(typography.disabled.clone()),
            ..ComponentTextStates::from_base(typography.label.clone())
        },
        icon: ComponentIconStates {
            selected: Some(IconStyle::new(colors.selected_text, opacity.opaque)),
            active: Some(IconStyle::new(colors.accent_text, opacity.opaque)),
            disabled: Some(IconStyle::new(colors.text_disabled, opacity.disabled)),
            ..ComponentIconStates::from_base(IconStyle::new(colors.text_subtle, opacity.muted))
        },
        layout: ComponentLayoutTokens {
            min_width: 32.0,
            min_height: 26.0,
            padding_x: spacing.md,
            padding_y: spacing.xs,
            gap: spacing.xs,
            icon_size: 14.0,
        },
    }
}

fn search_field_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
    opacity: &OpacityTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(colors.surface_sunken, Some(stroke.control), radius.md);

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(
                colors.surface,
                Some(stroke.control_hover),
                radius.md,
            )),
            focused: Some(UiVisual::panel(
                colors.surface,
                Some(stroke.focus),
                radius.md,
            )),
            invalid: Some(UiVisual::panel(
                colors.surface_sunken,
                Some(stroke.invalid),
                radius.md,
            )),
            warning: Some(UiVisual::panel(
                colors.surface_sunken,
                Some(stroke.warning),
                radius.md,
            )),
            disabled: Some(UiVisual::panel(
                color_with_alpha(colors.surface_sunken, 150),
                Some(StrokeStyle::new(
                    color_with_alpha(colors.border_muted, 120),
                    stroke.thin_width,
                )),
                radius.md,
            )),
            ..ComponentVisualStates::from_base(base)
        },
        text: ComponentTextStates {
            invalid: Some(text_style_with_color(&typography.body, colors.danger)),
            warning: Some(text_style_with_color(&typography.body, colors.warning)),
            disabled: Some(typography.disabled.clone()),
            ..ComponentTextStates::from_base(typography.body.clone())
        },
        icon: ComponentIconStates {
            focused: Some(IconStyle::new(colors.accent, opacity.opaque)),
            invalid: Some(IconStyle::new(colors.danger, opacity.opaque)),
            warning: Some(IconStyle::new(colors.warning, opacity.opaque)),
            disabled: Some(IconStyle::new(colors.text_disabled, opacity.disabled)),
            ..ComponentIconStates::from_base(IconStyle::new(colors.text_subtle, opacity.muted))
        },
        layout: ComponentLayoutTokens {
            min_width: 160.0,
            min_height: 30.0,
            padding_x: spacing.md,
            padding_y: spacing.xs,
            gap: spacing.sm,
            icon_size: 16.0,
        },
    }
}

fn lane_header_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
    opacity: &OpacityTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(colors.lane_header, Some(stroke.surface), radius.none);

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(
                colors.surface_muted,
                Some(stroke.surface),
                radius.none,
            )),
            selected: Some(UiVisual::panel(
                colors.lane_header_selected,
                Some(stroke.selected),
                radius.none,
            )),
            focused: Some(UiVisual::panel(
                colors.lane_header_selected,
                Some(stroke.focus),
                radius.none,
            )),
            active: Some(UiVisual::panel(
                colors.selected,
                Some(stroke.selected),
                radius.none,
            )),
            changed: Some(UiVisual::panel(
                colors.surface_muted,
                Some(stroke.warning),
                radius.none,
            )),
            disabled: Some(UiVisual::panel(
                color_with_alpha(colors.lane_header, 150),
                Some(StrokeStyle::new(
                    color_with_alpha(colors.border_muted, 120),
                    stroke.thin_width,
                )),
                radius.none,
            )),
            ..ComponentVisualStates::from_base(base)
        },
        text: ComponentTextStates {
            selected: Some(text_style_with_color(
                &typography.label_strong,
                colors.selected_text,
            )),
            active: Some(text_style_with_color(
                &typography.label_strong,
                colors.accent_text,
            )),
            changed: Some(text_style_with_color(
                &typography.label_strong,
                colors.warning,
            )),
            disabled: Some(typography.disabled.clone()),
            ..ComponentTextStates::from_base(typography.label.clone())
        },
        icon: ComponentIconStates {
            selected: Some(IconStyle::new(colors.selected_text, opacity.opaque)),
            active: Some(IconStyle::new(colors.transport_active, opacity.opaque)),
            changed: Some(IconStyle::new(colors.warning, opacity.opaque)),
            disabled: Some(IconStyle::new(colors.text_disabled, opacity.disabled)),
            ..ComponentIconStates::from_base(IconStyle::new(colors.text_subtle, opacity.muted))
        },
        layout: ComponentLayoutTokens {
            min_width: 120.0,
            min_height: 32.0,
            padding_x: spacing.md,
            padding_y: spacing.xs,
            gap: spacing.sm,
            icon_size: 15.0,
        },
    }
}

fn range_item_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
    opacity: &OpacityTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(
        colors.range_item_primary,
        Some(stroke.surface_strong),
        radius.sm,
    );

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(
                ColorRgba::new(72, 178, 207, 255),
                Some(stroke.control_hover),
                radius.sm,
            )),
            selected: Some(UiVisual::panel(
                colors.accent_strong,
                Some(stroke.focus),
                radius.sm,
            )),
            focused: Some(UiVisual::panel(
                colors.range_item_primary,
                Some(stroke.focus),
                radius.sm,
            )),
            active: Some(UiVisual::panel(
                colors.transport_active,
                Some(stroke.selected),
                radius.sm,
            )),
            invalid: Some(UiVisual::panel(
                colors.danger,
                Some(stroke.invalid),
                radius.sm,
            )),
            warning: Some(UiVisual::panel(
                colors.warning,
                Some(stroke.warning),
                radius.sm,
            )),
            pending: Some(UiVisual::panel(
                colors.accent_muted,
                Some(stroke.selected),
                radius.sm,
            )),
            disabled: Some(UiVisual::panel(
                color_with_alpha(colors.range_item_primary, 135),
                Some(StrokeStyle::new(
                    color_with_alpha(colors.border_muted, 120),
                    stroke.thin_width,
                )),
                radius.sm,
            )),
            ..ComponentVisualStates::from_base(base)
        },
        text: ComponentTextStates {
            selected: Some(text_style_with_color(
                &typography.caption_strong,
                colors.accent_text,
            )),
            active: Some(text_style_with_color(
                &typography.caption_strong,
                colors.text_inverse,
            )),
            invalid: Some(text_style_with_color(
                &typography.caption_strong,
                colors.text_inverse,
            )),
            warning: Some(text_style_with_color(
                &typography.caption_strong,
                colors.text_inverse,
            )),
            disabled: Some(text_style_with_color(
                &typography.caption,
                color_with_alpha(colors.text_inverse, 150),
            )),
            ..ComponentTextStates::from_base(text_style_with_color(
                &typography.caption_strong,
                ColorRgba::new(236, 250, 255, 255),
            ))
        },
        icon: ComponentIconStates {
            selected: Some(IconStyle::new(colors.accent_text, opacity.opaque)),
            active: Some(IconStyle::new(colors.text_inverse, opacity.opaque)),
            invalid: Some(IconStyle::new(colors.text_inverse, opacity.opaque)),
            warning: Some(IconStyle::new(colors.text_inverse, opacity.opaque)),
            disabled: Some(IconStyle::new(colors.text_inverse, opacity.disabled)),
            ..ComponentIconStates::from_base(IconStyle::new(
                ColorRgba::new(236, 250, 255, 255),
                opacity.opaque,
            ))
        },
        layout: ComponentLayoutTokens {
            min_width: 48.0,
            min_height: 24.0,
            padding_x: spacing.sm,
            padding_y: spacing.xxs,
            gap: spacing.xs,
            icon_size: 12.0,
        },
    }
}

fn editor_lane_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(colors.editor_lane, None, radius.none);

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(colors.surface_sunken, None, radius.none)),
            selected: Some(UiVisual::panel(colors.selected, None, radius.none)),
            active: Some(UiVisual::panel(colors.accent_muted, None, radius.none)),
            focused: Some(UiVisual::panel(
                colors.editor_lane,
                Some(stroke.focus),
                radius.none,
            )),
            disabled: Some(UiVisual::panel(
                color_with_alpha(colors.editor_lane_alternate, 150),
                None,
                radius.none,
            )),
            ..ComponentVisualStates::from_base(base)
        },
        text: ComponentTextStates {
            selected: Some(text_style_with_color(
                &typography.caption,
                colors.selected_text,
            )),
            disabled: Some(typography.disabled.clone()),
            ..ComponentTextStates::from_base(typography.caption.clone())
        },
        icon: ComponentIconStates::from_base(IconStyle::new(colors.text_subtle, 0.68)),
        layout: ComponentLayoutTokens {
            min_width: 24.0,
            min_height: 18.0,
            padding_x: spacing.xs,
            padding_y: spacing.xxxs,
            gap: spacing.xxs,
            icon_size: 10.0,
        },
    }
}

fn property_row_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
    opacity: &OpacityTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(ColorRgba::TRANSPARENT, None, radius.none);

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(colors.surface_muted, None, radius.xs)),
            selected: Some(UiVisual::panel(colors.selected, None, radius.xs)),
            focused: Some(UiVisual::panel(
                colors.surface_muted,
                Some(stroke.focus),
                radius.xs,
            )),
            changed: Some(UiVisual::panel(
                colors.surface_muted,
                Some(stroke.warning),
                radius.xs,
            )),
            invalid: Some(UiVisual::panel(
                colors.surface_muted,
                Some(stroke.invalid),
                radius.xs,
            )),
            disabled: Some(UiVisual::panel(
                color_with_alpha(colors.surface_sunken, 100),
                None,
                radius.xs,
            )),
            ..ComponentVisualStates::from_base(base)
        },
        text: ComponentTextStates {
            selected: Some(text_style_with_color(
                &typography.label,
                colors.selected_text,
            )),
            changed: Some(text_style_with_color(&typography.label, colors.warning)),
            invalid: Some(text_style_with_color(&typography.label, colors.danger)),
            disabled: Some(typography.disabled.clone()),
            ..ComponentTextStates::from_base(typography.label.clone())
        },
        icon: ComponentIconStates {
            selected: Some(IconStyle::new(colors.selected_text, opacity.opaque)),
            changed: Some(IconStyle::new(colors.warning, opacity.opaque)),
            invalid: Some(IconStyle::new(colors.danger, opacity.opaque)),
            disabled: Some(IconStyle::new(colors.text_disabled, opacity.disabled)),
            ..ComponentIconStates::from_base(IconStyle::new(colors.text_subtle, opacity.muted))
        },
        layout: ComponentLayoutTokens {
            min_width: 80.0,
            min_height: 26.0,
            padding_x: spacing.sm,
            padding_y: spacing.xxs,
            gap: spacing.sm,
            icon_size: 14.0,
        },
    }
}

fn menu_row_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
    opacity: &OpacityTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(ColorRgba::TRANSPARENT, None, radius.xs);

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(colors.surface_muted, None, radius.xs)),
            selected: Some(UiVisual::panel(colors.selected, None, radius.xs)),
            focused: Some(UiVisual::panel(
                colors.surface_muted,
                Some(stroke.focus),
                radius.xs,
            )),
            active: Some(UiVisual::panel(colors.selected_hover, None, radius.xs)),
            checked: Some(UiVisual::panel(colors.selected, None, radius.xs)),
            disabled: Some(UiVisual::panel(ColorRgba::TRANSPARENT, None, radius.xs)),
            ..ComponentVisualStates::from_base(base)
        },
        text: ComponentTextStates {
            selected: Some(text_style_with_color(
                &typography.label,
                colors.selected_text,
            )),
            active: Some(text_style_with_color(
                &typography.label,
                colors.selected_text,
            )),
            checked: Some(text_style_with_color(
                &typography.label_strong,
                colors.selected_text,
            )),
            disabled: Some(typography.disabled.clone()),
            ..ComponentTextStates::from_base(typography.label.clone())
        },
        icon: ComponentIconStates {
            selected: Some(IconStyle::new(colors.selected_text, opacity.opaque)),
            active: Some(IconStyle::new(colors.selected_text, opacity.opaque)),
            checked: Some(IconStyle::new(colors.accent, opacity.opaque)),
            disabled: Some(IconStyle::new(colors.text_disabled, opacity.disabled)),
            ..ComponentIconStates::from_base(IconStyle::new(colors.text_subtle, opacity.muted))
        },
        layout: ComponentLayoutTokens {
            min_width: 120.0,
            min_height: 26.0,
            padding_x: spacing.md,
            padding_y: spacing.xs,
            gap: spacing.sm,
            icon_size: 14.0,
        },
    }
}

fn transport_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
    opacity: &OpacityTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(colors.surface_elevated, Some(stroke.control), radius.md);

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(
                colors.surface_overlay,
                Some(stroke.control_hover),
                radius.md,
            )),
            pressed: Some(UiVisual::panel(
                colors.surface_sunken,
                Some(stroke.selected),
                radius.md,
            )),
            focused: Some(UiVisual::panel(
                colors.surface_overlay,
                Some(stroke.focus),
                radius.md,
            )),
            active: Some(UiVisual::panel(
                colors.transport_active,
                Some(StrokeStyle::new(colors.transport_active, stroke.thin_width)),
                radius.md,
            )),
            checked: Some(UiVisual::panel(
                colors.transport_active,
                Some(StrokeStyle::new(colors.transport_active, stroke.thin_width)),
                radius.md,
            )),
            disabled: Some(UiVisual::panel(
                color_with_alpha(colors.surface_muted, 150),
                Some(StrokeStyle::new(
                    color_with_alpha(colors.border_muted, 120),
                    stroke.thin_width,
                )),
                radius.md,
            )),
            ..ComponentVisualStates::from_base(base)
        },
        text: ComponentTextStates {
            active: Some(text_style_with_color(
                &typography.numeric,
                colors.text_inverse,
            )),
            checked: Some(text_style_with_color(
                &typography.numeric,
                colors.text_inverse,
            )),
            disabled: Some(typography.disabled.clone()),
            ..ComponentTextStates::from_base(typography.numeric.clone())
        },
        icon: ComponentIconStates {
            hovered: Some(IconStyle::new(colors.text, opacity.opaque)),
            active: Some(IconStyle::new(colors.text_inverse, opacity.opaque)),
            checked: Some(IconStyle::new(colors.text_inverse, opacity.opaque)),
            disabled: Some(IconStyle::new(colors.text_disabled, opacity.disabled)),
            ..ComponentIconStates::from_base(IconStyle::new(colors.text_muted, opacity.opaque))
        },
        layout: ComponentLayoutTokens {
            min_width: 30.0,
            min_height: 30.0,
            padding_x: spacing.sm,
            padding_y: spacing.xs,
            gap: spacing.xs,
            icon_size: 16.0,
        },
    }
}

fn text_style(font_size: f32, line_height: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height,
        family: FontFamily::SansSerif,
        weight,
        style: FontStyle::Normal,
        stretch: FontStretch::Normal,
        wrap: TextWrap::Word,
        color,
    }
}

fn mono_style(font_size: f32, line_height: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        family: FontFamily::Monospace,
        ..text_style(font_size, line_height, weight, color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_state_flags_are_compact_and_composable() {
        let state = ComponentState::HOVERED | ComponentState::FOCUSED | ComponentState::SELECTED;

        assert_eq!(
            state.bits(),
            ComponentState::HOVERED.bits()
                | ComponentState::FOCUSED.bits()
                | ComponentState::SELECTED.bits()
        );
        assert!(state.hovered());
        assert!(state.focused());
        assert!(state.selected());
        assert!(!state.disabled());
        assert_eq!(
            state.without(ComponentState::FOCUSED),
            ComponentState::HOVERED | ComponentState::SELECTED
        );
    }

    #[test]
    fn disabled_state_suppresses_interactive_resolution() {
        let theme = Theme::dark();
        let state = ComponentState::DISABLED | ComponentState::PRESSED | ComponentState::FOCUSED;
        let button = theme.component(ComponentRole::Button);

        assert_eq!(
            button.visual.resolve_slot(state).0,
            ComponentStateSlot::Disabled
        );
        assert_eq!(
            button.text.resolve_slot(state).0,
            ComponentStateSlot::Disabled
        );
        assert_eq!(
            button.icon.resolve_slot(state).0,
            ComponentStateSlot::Disabled
        );
    }

    #[test]
    fn focused_visual_wins_over_hover_when_both_are_present() {
        let theme = Theme::dark();
        let visual = theme.resolve_visual(
            ComponentRole::Button,
            ComponentState::FOCUSED | ComponentState::HOVERED,
        );

        assert_eq!(visual.stroke, Some(theme.stroke.focus));
    }

    #[test]
    fn missing_state_specific_visual_falls_back_to_base() {
        let base = UiVisual::panel(ColorRgba::new(1, 2, 3, 255), None, 0.0);
        let states = ComponentVisualStates::from_base(base);

        assert_eq!(states.resolve(ComponentState::HOVERED), base);
        assert_eq!(
            states.resolve_slot(ComponentState::DISABLED),
            (ComponentStateSlot::Base, base)
        );
    }

    #[test]
    fn dark_theme_exposes_dense_semantic_tokens() {
        let theme = Theme::dark();

        assert_eq!(theme.name, OPERAD_DARK_THEME_NAME);
        assert!(theme.spacing.xxs < theme.spacing.md);
        assert!(theme.spacing.md < theme.spacing.xxl);
        assert!(theme.radius.sm < theme.radius.pill);
        assert!(theme.motion.fast_ms < theme.motion.slow_ms);
        assert_eq!(theme.colors.canvas.a, 255);
        assert_eq!(theme.colors.focus_ring.a, 255);
        assert_ne!(theme.colors.success, theme.colors.warning);
        assert_ne!(theme.colors.warning, theme.colors.danger);
    }

    #[test]
    fn component_helpers_resolve_visual_text_and_icon() {
        let theme = Theme::dark();
        let state = ComponentState::ACTIVE;

        assert_eq!(
            theme.resolve_visual(ComponentRole::TransportControl, state),
            theme.components.transport_control.visual.active.unwrap()
        );
        assert_eq!(
            theme
                .resolve_text(ComponentRole::TransportControl, state)
                .color,
            theme.colors.text_inverse
        );
        assert_eq!(
            theme
                .resolve_icon(ComponentRole::TransportControl, state)
                .tint,
            theme.colors.text_inverse
        );
    }

    #[test]
    fn theme_preferences_scale_typography_and_component_text() {
        let theme = Theme::dark();
        let adjusted =
            theme.with_accessibility_preferences(AccessibilityPreferences::DEFAULT.text_scale(1.5));

        assert_eq!(adjusted.typography.body.font_size, 21.0);
        assert_eq!(adjusted.typography.body.line_height, 30.0);
        assert_eq!(
            adjusted
                .resolve_text(ComponentRole::Button, ComponentState::NORMAL)
                .font_size,
            theme
                .resolve_text(ComponentRole::Button, ComponentState::NORMAL)
                .font_size
                * 1.5
        );
    }

    #[test]
    fn theme_preferences_reduce_motion_tokens() {
        let theme = Theme::dark();
        let adjusted = theme
            .with_accessibility_preferences(AccessibilityPreferences::DEFAULT.reduced_motion(true));

        assert_eq!(adjusted.motion.micro_ms, adjusted.motion.instant_ms);
        assert_eq!(adjusted.motion.fast_ms, adjusted.motion.instant_ms);
        assert_eq!(adjusted.motion.normal_ms, adjusted.motion.instant_ms);
        assert_eq!(adjusted.motion.slow_ms, adjusted.motion.instant_ms);
        assert_eq!(adjusted.motion.standard, MotionCurve::Linear);
        assert_eq!(adjusted.motion.emphasized, MotionCurve::Linear);
        assert_eq!(adjusted.motion.exit, MotionCurve::Linear);
    }

    #[test]
    fn theme_preferences_remove_translucent_component_colors() {
        let theme = Theme::dark();
        let adjusted = theme.with_accessibility_preferences(
            AccessibilityPreferences::DEFAULT.reduced_transparency(true),
        );
        let disabled_button =
            adjusted.resolve_visual(ComponentRole::Button, ComponentState::DISABLED);

        assert_eq!(adjusted.colors.overlay_scrim.a, 255);
        assert_eq!(adjusted.typography.disabled.color.a, 255);
        assert_eq!(disabled_button.fill.a, 255);
        assert_eq!(
            adjusted
                .resolve_icon(ComponentRole::Button, ComponentState::DISABLED)
                .opacity,
            1.0
        );
        assert_eq!(adjusted.opacity.scrim, 1.0);
    }

    #[test]
    fn forced_colors_implies_high_contrast_theme_policy() {
        let theme = Theme::dark();
        let adjusted = theme
            .with_accessibility_preferences(AccessibilityPreferences::DEFAULT.forced_colors(true));

        assert_eq!(adjusted.colors.text_muted, adjusted.colors.text);
        assert_eq!(adjusted.colors.text_subtle, adjusted.colors.text);
        assert_eq!(adjusted.colors.text_disabled, adjusted.colors.text);
        assert_eq!(adjusted.colors.overlay_scrim.a, 255);
        assert!(adjusted.stroke.focus.width > theme.stroke.focus.width);
        assert_eq!(
            adjusted
                .resolve_text(ComponentRole::Button, ComponentState::DISABLED)
                .color,
            adjusted.colors.text
        );
        assert_eq!(
            adjusted
                .resolve_visual(ComponentRole::Button, ComponentState::FOCUSED)
                .stroke,
            Some(adjusted.stroke.focus)
        );
    }

    #[test]
    fn scoped_theme_resolves_inherited_editor_overrides() {
        let base = Theme::dark();
        let mut editor_colors = base.colors;
        editor_colors.editor_background = ColorRgba::new(4, 8, 12, 255);
        editor_colors.editor_lane = ColorRgba::new(18, 24, 36, 255);
        editor_colors.range_item_secondary = ColorRgba::new(36, 180, 118, 255);

        let shell_id = ThemeScopeId::new("shell");
        let editor_id = ThemeScopeId::new("value-grid");
        let registry = ScopedThemeRegistry::new(base.clone())
            .with_scope(ThemeScope::shell(shell_id.clone()))
            .with_scope(
                ThemeScope::editor_surface(editor_id.clone())
                    .with_parent(shell_id.clone())
                    .with_patch(ThemePatch::new().colors(editor_colors)),
            );

        let shell = registry.resolve(&shell_id).unwrap();
        let editor = registry.resolve(&editor_id).unwrap();

        assert_eq!(
            shell.colors.editor_background,
            base.colors.editor_background
        );
        assert_eq!(
            shell.resolve_visual(ComponentRole::Button, ComponentState::NORMAL),
            base.resolve_visual(ComponentRole::Button, ComponentState::NORMAL)
        );
        assert_eq!(
            editor.colors.editor_background,
            editor_colors.editor_background
        );
        assert_eq!(
            editor
                .resolve_visual(ComponentRole::EditorLane, ComponentState::NORMAL)
                .fill,
            editor_colors.editor_lane
        );
    }

    #[test]
    fn scoped_theme_parent_patch_feeds_child_resolution() {
        let mut compact_spacing = SpacingTokens::dense();
        compact_spacing.control_x = 3.0;
        compact_spacing.control_y = 2.0;

        let shell_id = ThemeScopeId::new("shell");
        let panel_id = ThemeScopeId::new("inspector");
        let registry = ScopedThemeRegistry::default()
            .with_scope(
                ThemeScope::shell(shell_id.clone())
                    .with_patch(ThemePatch::new().spacing(compact_spacing)),
            )
            .with_scope(ThemeScope::panel(panel_id.clone()).with_parent(shell_id));

        let panel = registry.resolve(&panel_id).unwrap();

        assert_eq!(panel.spacing.control_x, compact_spacing.control_x);
        assert_eq!(
            panel.component(ComponentRole::Button).layout.padding_x,
            compact_spacing.control_x
        );
    }

    #[test]
    fn scoped_theme_reports_missing_scope_and_cycles() {
        let missing = ScopedThemeRegistry::default()
            .resolve(&ThemeScopeId::new("missing"))
            .unwrap_err();
        assert_eq!(
            missing,
            ThemeScopeError::MissingScope(ThemeScopeId::new("missing"))
        );

        let first = ThemeScopeId::new("first");
        let second = ThemeScopeId::new("second");
        let registry = ScopedThemeRegistry::default()
            .with_scope(ThemeScope::panel(first.clone()).with_parent(second.clone()))
            .with_scope(ThemeScope::panel(second.clone()).with_parent(first.clone()));

        let err = registry.resolve(&first).unwrap_err();
        assert!(
            matches!(err, ThemeScopeError::Cycle(path) if path == vec![first, second, ThemeScopeId::new("first")])
        );
    }
}
