//! Public API stability and feature-versioning markers.

use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiStability {
    Stable,
    Experimental,
    BackendSpecific,
    MigrationOnly,
}

impl ApiStability {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Experimental => "experimental",
            Self::BackendSpecific => "backend-specific",
            Self::MigrationOnly => "migration-only",
        }
    }

    pub const fn is_semver_protected(self) -> bool {
        matches!(self, Self::Stable)
    }

    pub const fn may_change_without_major(self) -> bool {
        !self.is_semver_protected()
    }
}

pub trait ApiStabilityMarker: Copy + Clone + Default + Eq + PartialEq {
    const STABILITY: ApiStability;

    fn stability(self) -> ApiStability {
        Self::STABILITY
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Stable;

impl ApiStabilityMarker for Stable {
    const STABILITY: ApiStability = ApiStability::Stable;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Experimental;

impl ApiStabilityMarker for Experimental {
    const STABILITY: ApiStability = ApiStability::Experimental;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BackendSpecific;

impl ApiStabilityMarker for BackendSpecific {
    const STABILITY: ApiStability = ApiStability::BackendSpecific;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MigrationOnly;

impl ApiStabilityMarker for MigrationOnly {
    const STABILITY: ApiStability = ApiStability::MigrationOnly;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StabilityNote {
    pub stability: ApiStability,
    pub since: Option<&'static str>,
    pub note: &'static str,
}

impl StabilityNote {
    pub const fn new(
        stability: ApiStability,
        since: Option<&'static str>,
        note: &'static str,
    ) -> Self {
        Self {
            stability,
            since,
            note,
        }
    }

    pub const fn stable(since: &'static str, note: &'static str) -> Self {
        Self::new(ApiStability::Stable, Some(since), note)
    }

    pub const fn experimental(since: &'static str, note: &'static str) -> Self {
        Self::new(ApiStability::Experimental, Some(since), note)
    }

    pub const fn backend_specific(feature: &'static str) -> Self {
        Self::new(ApiStability::BackendSpecific, None, feature)
    }

    pub const fn migration_only(note: &'static str) -> Self {
        Self::new(ApiStability::MigrationOnly, None, note)
    }

    pub const fn is_semver_protected(self) -> bool {
        self.stability.is_semver_protected()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureStability {
    pub feature: &'static str,
    pub stability: ApiStability,
    pub since: Option<&'static str>,
    pub note: &'static str,
}

impl FeatureStability {
    pub const fn new(
        feature: &'static str,
        stability: ApiStability,
        since: Option<&'static str>,
        note: &'static str,
    ) -> Self {
        Self {
            feature,
            stability,
            since,
            note,
        }
    }

    pub const fn stable(feature: &'static str, since: &'static str, note: &'static str) -> Self {
        Self::new(feature, ApiStability::Stable, Some(since), note)
    }

    pub const fn experimental(
        feature: &'static str,
        since: &'static str,
        note: &'static str,
    ) -> Self {
        Self::new(feature, ApiStability::Experimental, Some(since), note)
    }

    pub const fn backend_specific(feature: &'static str, note: &'static str) -> Self {
        Self::new(feature, ApiStability::BackendSpecific, None, note)
    }

    pub const fn migration_only(feature: &'static str, note: &'static str) -> Self {
        Self::new(feature, ApiStability::MigrationOnly, None, note)
    }

    pub const fn is_semver_protected(self) -> bool {
        self.stability.is_semver_protected()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApiStatus<S: ApiStabilityMarker> {
    pub since: Option<&'static str>,
    pub note: &'static str,
    marker: PhantomData<S>,
}

impl<S: ApiStabilityMarker> ApiStatus<S> {
    pub const fn new(since: Option<&'static str>, note: &'static str) -> Self {
        Self {
            since,
            note,
            marker: PhantomData,
        }
    }

    pub const fn stability(&self) -> ApiStability {
        S::STABILITY
    }

    pub const fn note(&self) -> StabilityNote {
        StabilityNote::new(S::STABILITY, self.since, self.note)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_types_classify_api_stability() {
        assert_eq!(Stable.stability(), ApiStability::Stable);
        assert_eq!(Experimental.stability(), ApiStability::Experimental);
        assert_eq!(BackendSpecific.stability(), ApiStability::BackendSpecific);
        assert_eq!(MigrationOnly.stability(), ApiStability::MigrationOnly);
    }

    #[test]
    fn stability_notes_encode_semver_expectations() {
        let stable = StabilityNote::stable("5.0.0", "Public layout primitives");
        let experimental = StabilityNote::experimental("5.0.0", "Early host runtime policy");

        assert!(stable.is_semver_protected());
        assert!(experimental.stability.may_change_without_major());
        assert_eq!(ApiStability::MigrationOnly.label(), "migration-only");
    }

    #[test]
    fn feature_stability_records_feature_scope() {
        let wgpu =
            FeatureStability::backend_specific("wgpu", "Renderer availability depends on backend");
        let status = ApiStatus::<Stable>::new(Some("5.0.0"), "Core document tree");

        assert_eq!(wgpu.feature, "wgpu");
        assert!(!wgpu.is_semver_protected());
        assert_eq!(status.stability(), ApiStability::Stable);
        assert_eq!(status.note().since, Some("5.0.0"));
    }
}
