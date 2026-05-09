//! Backend-neutral theme tokens for Operad v3.
//!
//! This module keeps product semantics out of Operad while giving consumers a
//! stable vocabulary for dense workstation-style UI surfaces. It intentionally
//! resolves to existing core primitives (`UiVisual`, `TextStyle`, `StrokeStyle`,
//! and `ColorRgba`) instead of renderer-specific paint objects.

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::{
    ColorRgba, FontFamily, FontStretch, FontStyle, FontWeight, StrokeStyle, TextStyle, TextWrap,
    UiVisual,
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
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentRole {
    Button,
    Tab,
    SearchField,
    TrackHeader,
    ClipBlock,
    PianoRollLane,
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
    pub track_header: ColorRgba,
    pub track_header_selected: ColorRgba,
    pub clip_audio: ColorRgba,
    pub clip_midi: ColorRgba,
    pub clip_automation: ColorRgba,
    pub piano_roll_lane: ColorRgba,
    pub piano_roll_lane_alt: ColorRgba,
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
            track_header: ColorRgba::new(25, 31, 41, 255),
            track_header_selected: ColorRgba::new(35, 55, 78, 255),
            clip_audio: ColorRgba::new(62, 157, 184, 255),
            clip_midi: ColorRgba::new(116, 176, 98, 255),
            clip_automation: ColorRgba::new(187, 126, 220, 255),
            piano_roll_lane: ColorRgba::new(17, 22, 30, 255),
            piano_roll_lane_alt: ColorRgba::new(14, 18, 25, 255),
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
    pub track_header: ComponentStyle,
    pub clip_block: ComponentStyle,
    pub piano_roll_lane: ComponentStyle,
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
            ComponentRole::TrackHeader => &self.track_header,
            ComponentRole::ClipBlock => &self.clip_block,
            ComponentRole::PianoRollLane => &self.piano_roll_lane,
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
            track_header: track_header_tokens(colors, spacing, typography, radius, stroke, opacity),
            clip_block: clip_block_tokens(colors, spacing, typography, radius, stroke, opacity),
            piano_roll_lane: piano_roll_lane_tokens(colors, spacing, typography, radius, stroke),
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

pub const fn color_with_alpha(color: ColorRgba, alpha: u8) -> ColorRgba {
    ColorRgba::new(color.r, color.g, color.b, alpha)
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

fn track_header_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
    opacity: &OpacityTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(colors.track_header, Some(stroke.surface), radius.none);

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(
                colors.surface_muted,
                Some(stroke.surface),
                radius.none,
            )),
            selected: Some(UiVisual::panel(
                colors.track_header_selected,
                Some(stroke.selected),
                radius.none,
            )),
            focused: Some(UiVisual::panel(
                colors.track_header_selected,
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
                color_with_alpha(colors.track_header, 150),
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

fn clip_block_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
    opacity: &OpacityTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(colors.clip_audio, Some(stroke.surface_strong), radius.sm);

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
                colors.clip_audio,
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
                color_with_alpha(colors.clip_audio, 135),
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

fn piano_roll_lane_tokens(
    colors: &ColorTokens,
    spacing: &SpacingTokens,
    typography: &TypographyTokens,
    radius: &RadiusTokens,
    stroke: &StrokeTokens,
) -> ComponentStyle {
    let base = UiVisual::panel(colors.piano_roll_lane, None, radius.none);

    ComponentStyle {
        visual: ComponentVisualStates {
            hovered: Some(UiVisual::panel(colors.surface_sunken, None, radius.none)),
            selected: Some(UiVisual::panel(colors.selected, None, radius.none)),
            active: Some(UiVisual::panel(colors.accent_muted, None, radius.none)),
            focused: Some(UiVisual::panel(
                colors.piano_roll_lane,
                Some(stroke.focus),
                radius.none,
            )),
            disabled: Some(UiVisual::panel(
                color_with_alpha(colors.piano_roll_lane_alt, 150),
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
}
