//! Locale, text-direction, and dynamic label policy primitives.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleId {
    tag: String,
}

impl LocaleId {
    pub fn new(tag: impl Into<String>) -> Result<Self, LocaleIdentifierError> {
        let tag = tag.into();
        let normalized = normalize_locale_tag(&tag)?;
        Ok(Self { tag: normalized })
    }

    pub fn unchecked(tag: impl Into<String>) -> Self {
        Self { tag: tag.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.tag
    }

    pub fn language(&self) -> &str {
        self.tag
            .split(['-', '_'])
            .next()
            .unwrap_or(self.tag.as_str())
    }

    pub fn direction_hint(&self) -> ResolvedTextDirection {
        if rtl_language(self.language()) {
            ResolvedTextDirection::Rtl
        } else {
            ResolvedTextDirection::Ltr
        }
    }
}

impl Default for LocaleId {
    fn default() -> Self {
        Self {
            tag: "en-US".to_string(),
        }
    }
}

impl TryFrom<&str> for LocaleId {
    type Error = LocaleIdentifierError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocaleIdentifierError {
    Empty,
    InvalidCharacter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTextDirection {
    Ltr,
    Rtl,
}

impl ResolvedTextDirection {
    pub const fn is_rtl(self) -> bool {
        matches!(self, Self::Rtl)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    Auto,
    Ltr,
    Rtl,
}

impl TextDirection {
    pub fn resolve(self, locale: Option<&LocaleId>) -> ResolvedTextDirection {
        match self {
            Self::Auto => locale
                .map(LocaleId::direction_hint)
                .unwrap_or(ResolvedTextDirection::Ltr),
            Self::Ltr => ResolvedTextDirection::Ltr,
            Self::Rtl => ResolvedTextDirection::Rtl,
        }
    }
}

impl Default for TextDirection {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidiPolicy {
    PlainText,
    Isolate,
    Embed,
}

impl Default for BidiPolicy {
    fn default() -> Self {
        Self::Isolate
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMirrorMode {
    Never,
    InlineStartEnd,
    AllHorizontal,
}

impl LayoutMirrorMode {
    pub const fn mirrors_for_direction(self, direction: ResolvedTextDirection) -> bool {
        direction.is_rtl() && !matches!(self, Self::Never)
    }
}

impl Default for LayoutMirrorMode {
    fn default() -> Self {
        Self::InlineStartEnd
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationPolicy {
    pub locale: LocaleId,
    pub text_direction: TextDirection,
    pub bidi: BidiPolicy,
    pub layout_mirroring: LayoutMirrorMode,
}

impl LocalizationPolicy {
    pub fn new(locale: LocaleId) -> Self {
        Self {
            locale,
            ..Default::default()
        }
    }

    pub fn with_text_direction(mut self, text_direction: TextDirection) -> Self {
        self.text_direction = text_direction;
        self
    }

    pub fn with_bidi(mut self, bidi: BidiPolicy) -> Self {
        self.bidi = bidi;
        self
    }

    pub fn with_layout_mirroring(mut self, layout_mirroring: LayoutMirrorMode) -> Self {
        self.layout_mirroring = layout_mirroring;
        self
    }

    pub fn resolved_direction(&self) -> ResolvedTextDirection {
        self.text_direction.resolve(Some(&self.locale))
    }

    pub fn should_mirror_layout(&self) -> bool {
        self.layout_mirroring
            .mirrors_for_direction(self.resolved_direction())
    }
}

impl Default for LocalizationPolicy {
    fn default() -> Self {
        Self {
            locale: LocaleId::default(),
            text_direction: TextDirection::Auto,
            bidi: BidiPolicy::default(),
            layout_mirroring: LayoutMirrorMode::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelUpdatePolicy {
    Static,
    Dynamic { revision: u64 },
}

impl LabelUpdatePolicy {
    pub const fn is_dynamic(self) -> bool {
        matches!(self, Self::Dynamic { .. })
    }
}

impl Default for LabelUpdatePolicy {
    fn default() -> Self {
        Self::Static
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicLabelMeta {
    pub key: Option<String>,
    pub fallback: String,
    pub locale: Option<LocaleId>,
    pub direction: TextDirection,
    pub bidi: BidiPolicy,
    pub update: LabelUpdatePolicy,
}

impl DynamicLabelMeta {
    pub fn literal(text: impl Into<String>) -> Self {
        Self {
            key: None,
            fallback: text.into(),
            locale: None,
            direction: TextDirection::Auto,
            bidi: BidiPolicy::default(),
            update: LabelUpdatePolicy::Static,
        }
    }

    pub fn keyed(key: impl Into<String>, fallback: impl Into<String>) -> Self {
        Self {
            key: Some(key.into()),
            fallback: fallback.into(),
            locale: None,
            direction: TextDirection::Auto,
            bidi: BidiPolicy::default(),
            update: LabelUpdatePolicy::Static,
        }
    }

    pub fn dynamic(key: impl Into<String>, fallback: impl Into<String>, revision: u64) -> Self {
        Self::keyed(key, fallback).with_update(LabelUpdatePolicy::Dynamic { revision })
    }

    pub fn with_locale(mut self, locale: LocaleId) -> Self {
        self.locale = Some(locale);
        self
    }

    pub fn with_direction(mut self, direction: TextDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn with_bidi(mut self, bidi: BidiPolicy) -> Self {
        self.bidi = bidi;
        self
    }

    pub fn with_update(mut self, update: LabelUpdatePolicy) -> Self {
        self.update = update;
        self
    }

    pub fn resolved_direction(&self, policy: Option<&LocalizationPolicy>) -> ResolvedTextDirection {
        match self.direction {
            TextDirection::Auto => self
                .locale
                .as_ref()
                .map(LocaleId::direction_hint)
                .or_else(|| policy.map(LocalizationPolicy::resolved_direction))
                .unwrap_or(ResolvedTextDirection::Ltr),
            direction => {
                direction.resolve(self.locale.as_ref().or_else(|| policy.map(|p| &p.locale)))
            }
        }
    }
}

fn normalize_locale_tag(tag: &str) -> Result<String, LocaleIdentifierError> {
    let tag = tag.trim();
    if tag.is_empty() {
        return Err(LocaleIdentifierError::Empty);
    }
    if !tag
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(LocaleIdentifierError::InvalidCharacter);
    }
    Ok(tag.replace('_', "-"))
}

fn rtl_language(language: &str) -> bool {
    matches!(
        language.to_ascii_lowercase().as_str(),
        "ar" | "dv" | "fa" | "he" | "iw" | "ks" | "ku" | "ps" | "sd" | "ug" | "ur" | "yi"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_hints_resolve_rtl_and_ltr() {
        let arabic = LocaleId::new("ar-EG").expect("valid locale");
        let english = LocaleId::new("en_US").expect("valid locale");

        assert_eq!(arabic.language(), "ar");
        assert_eq!(arabic.direction_hint(), ResolvedTextDirection::Rtl);
        assert_eq!(english.as_str(), "en-US");
        assert_eq!(
            TextDirection::Auto.resolve(Some(&english)),
            ResolvedTextDirection::Ltr
        );
    }

    #[test]
    fn mirroring_policy_follows_resolved_direction() {
        let policy = LocalizationPolicy::new(LocaleId::new("he-IL").expect("valid locale"));

        assert_eq!(policy.resolved_direction(), ResolvedTextDirection::Rtl);
        assert!(policy.should_mirror_layout());
        assert!(!policy
            .with_layout_mirroring(LayoutMirrorMode::Never)
            .should_mirror_layout());
    }

    #[test]
    fn dynamic_label_uses_label_locale_before_policy_locale() {
        let policy = LocalizationPolicy::new(LocaleId::new("en-US").expect("valid locale"));
        let label = DynamicLabelMeta::dynamic("nav.back", "Back", 7)
            .with_locale(LocaleId::new("fa-IR").expect("valid locale"));

        assert!(label.update.is_dynamic());
        assert_eq!(
            label.resolved_direction(Some(&policy)),
            ResolvedTextDirection::Rtl
        );
    }
}
