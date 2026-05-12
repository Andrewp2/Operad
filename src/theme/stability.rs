//! Stability records for v5 theme and design-token APIs.

use crate::{ApiStability, FeatureStability, StabilityNote};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeStabilityScope {
    ThemeModel,
    TokenCategories,
    ComponentStates,
    ScopedThemes,
    AccessibilityAdjustments,
    BackendRendering,
    MigrationCompatibility,
    DiagnosticInspection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeTokenCategory {
    Color,
    Spacing,
    Typography,
    Radius,
    Stroke,
    Effect,
    Opacity,
    Motion,
    Component,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThemeScopeStability {
    pub scope: ThemeStabilityScope,
    pub status: StabilityNote,
}

impl ThemeScopeStability {
    pub const fn new(scope: ThemeStabilityScope, status: StabilityNote) -> Self {
        Self { scope, status }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThemeTokenStability {
    pub category: ThemeTokenCategory,
    pub status: StabilityNote,
}

impl ThemeTokenStability {
    pub const fn new(category: ThemeTokenCategory, status: StabilityNote) -> Self {
        Self { category, status }
    }
}

pub const THEME_SCOPE_STABILITY: &[ThemeScopeStability] = &[
    ThemeScopeStability::new(
        ThemeStabilityScope::ThemeModel,
        StabilityNote::stable(
            "5.0.0",
            "Theme and ThemePatch record shapes are v5 public API.",
        ),
    ),
    ThemeScopeStability::new(
        ThemeStabilityScope::TokenCategories,
        StabilityNote::stable("5.0.0", "Core token categories are semver protected."),
    ),
    ThemeScopeStability::new(
        ThemeStabilityScope::ComponentStates,
        StabilityNote::stable(
            "5.0.0",
            "Component role and state resolution order is semver protected.",
        ),
    ),
    ThemeScopeStability::new(
        ThemeStabilityScope::ScopedThemes,
        StabilityNote::stable("5.0.0", "Scoped theme inheritance and fallback are stable."),
    ),
    ThemeScopeStability::new(
        ThemeStabilityScope::AccessibilityAdjustments,
        StabilityNote::stable(
            "5.0.0",
            "Reduced motion, contrast, and transparency adjustments are stable policy.",
        ),
    ),
    ThemeScopeStability::new(
        ThemeStabilityScope::BackendRendering,
        StabilityNote::backend_specific("Renderer output can vary by backend feature."),
    ),
    ThemeScopeStability::new(
        ThemeStabilityScope::MigrationCompatibility,
        StabilityNote::migration_only("Compatibility paths are not new product API."),
    ),
    ThemeScopeStability::new(
        ThemeStabilityScope::DiagnosticInspection,
        StabilityNote::experimental("5.0.0", "Debug theme inspection may expand before v5.1."),
    ),
];

pub const THEME_TOKEN_STABILITY: &[ThemeTokenStability] = &[
    ThemeTokenStability::new(
        ThemeTokenCategory::Color,
        StabilityNote::stable("5.0.0", "Semantic color fields are stable public tokens."),
    ),
    ThemeTokenStability::new(
        ThemeTokenCategory::Spacing,
        StabilityNote::stable("5.0.0", "Spacing scale fields are stable public tokens."),
    ),
    ThemeTokenStability::new(
        ThemeTokenCategory::Typography,
        StabilityNote::stable("5.0.0", "Typography role fields are stable public tokens."),
    ),
    ThemeTokenStability::new(
        ThemeTokenCategory::Radius,
        StabilityNote::stable("5.0.0", "Radius scale fields are stable public tokens."),
    ),
    ThemeTokenStability::new(
        ThemeTokenCategory::Stroke,
        StabilityNote::stable("5.0.0", "Stroke role fields are stable public tokens."),
    ),
    ThemeTokenStability::new(
        ThemeTokenCategory::Effect,
        StabilityNote::stable("5.0.0", "Effect records are stable with backend fallbacks."),
    ),
    ThemeTokenStability::new(
        ThemeTokenCategory::Opacity,
        StabilityNote::stable("5.0.0", "Opacity role fields are stable public tokens."),
    ),
    ThemeTokenStability::new(
        ThemeTokenCategory::Motion,
        StabilityNote::stable(
            "5.0.0",
            "Motion durations and curves are stable public tokens.",
        ),
    ),
    ThemeTokenStability::new(
        ThemeTokenCategory::Component,
        StabilityNote::stable(
            "5.0.0",
            "Component token records and state slots are stable.",
        ),
    ),
];

pub const THEME_FEATURE_STABILITY: &[FeatureStability] = &[
    FeatureStability::stable(
        "theme",
        "5.0.0",
        "Backend-neutral Theme, token records, scoped registry, and component state resolution.",
    ),
    FeatureStability::stable(
        "accessibility-theme-adjustments",
        "5.0.0",
        "Reduced motion, high contrast, forced colors, and reduced transparency token policies.",
    ),
    FeatureStability::backend_specific(
        "wgpu",
        "GPU rendering may differ in antialiasing, effect approximation, and text integration.",
    ),
    FeatureStability::migration_only(
        "egui-renderer-compat",
        "Legacy renderer compatibility exists to aid migration, not as the preferred v5 path.",
    ),
    FeatureStability::experimental(
        "debug-theme-inspection",
        "5.0.0",
        "Debug snapshots and token inspection are diagnostics and may add fields.",
    ),
];

pub fn theme_scope_stability(scope: ThemeStabilityScope) -> Option<ThemeScopeStability> {
    THEME_SCOPE_STABILITY
        .iter()
        .copied()
        .find(|record| record.scope == scope)
}

pub fn theme_token_stability(category: ThemeTokenCategory) -> Option<ThemeTokenStability> {
    THEME_TOKEN_STABILITY
        .iter()
        .copied()
        .find(|record| record.category == category)
}

pub fn theme_feature_stability(feature: &str) -> Option<FeatureStability> {
    THEME_FEATURE_STABILITY
        .iter()
        .copied()
        .find(|record| record.feature == feature)
}

pub fn stable_theme_token_categories() -> impl Iterator<Item = ThemeTokenCategory> {
    THEME_TOKEN_STABILITY
        .iter()
        .filter(|record| record.status.stability == ApiStability::Stable)
        .map(|record| record.category)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_core_token_categories_are_stable() {
        let stable = stable_theme_token_categories().collect::<Vec<_>>();

        assert_eq!(stable.len(), THEME_TOKEN_STABILITY.len());
        assert!(stable.contains(&ThemeTokenCategory::Color));
        assert!(stable.contains(&ThemeTokenCategory::Motion));
        assert!(theme_token_stability(ThemeTokenCategory::Component)
            .unwrap()
            .status
            .is_semver_protected());
    }

    #[test]
    fn scopes_classify_backend_migration_and_diagnostic_surfaces() {
        let backend = theme_scope_stability(ThemeStabilityScope::BackendRendering).unwrap();
        let migration = theme_scope_stability(ThemeStabilityScope::MigrationCompatibility).unwrap();
        let diagnostics = theme_scope_stability(ThemeStabilityScope::DiagnosticInspection).unwrap();

        assert_eq!(backend.status.stability, ApiStability::BackendSpecific);
        assert_eq!(migration.status.stability, ApiStability::MigrationOnly);
        assert_eq!(diagnostics.status.stability, ApiStability::Experimental);
    }

    #[test]
    fn feature_records_link_to_versioning_categories() {
        let theme = theme_feature_stability("theme").unwrap();
        let wgpu = theme_feature_stability("wgpu").unwrap();
        let compat = theme_feature_stability("egui-renderer-compat").unwrap();

        assert!(theme.is_semver_protected());
        assert_eq!(wgpu.stability, ApiStability::BackendSpecific);
        assert_eq!(compat.stability, ApiStability::MigrationOnly);
        assert!(theme_feature_stability("unknown").is_none());
    }
}
