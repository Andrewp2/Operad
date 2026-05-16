//! Resource and input limit policies with non-panicking validation reports.

use crate::errors::{
    ErrorKind, ErrorReport, ErrorSeverity, FallbackDecision, ResourceErrorKind, UiErrorKind,
};
use crate::platform::{DragPayload, PixelSize, ResourceId, ResourceKind};
use crate::renderer::{ResourceDescriptor, ResourceUpdate};

const MIB: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DimensionLimit {
    pub max_width: u32,
    pub max_height: u32,
    pub max_pixels: u64,
}

impl DimensionLimit {
    pub const fn new(max_width: u32, max_height: u32, max_pixels: u64) -> Self {
        Self {
            max_width,
            max_height,
            max_pixels,
        }
    }

    pub const fn square(max_edge: u32) -> Self {
        Self {
            max_width: max_edge,
            max_height: max_edge,
            max_pixels: max_edge as u64 * max_edge as u64,
        }
    }

    pub fn accepts(self, size: PixelSize) -> bool {
        !size_is_empty(size)
            && size.width <= self.max_width
            && size.height <= self.max_height
            && pixel_count(size) <= self.max_pixels
    }

    pub const fn as_pixel_size(self) -> PixelSize {
        PixelSize::new(self.max_width, self.max_height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceLimitPolicy {
    pub texture_size: DimensionLimit,
    pub image_dimensions: DimensionLimit,
    pub max_font_bytes: usize,
    pub max_cache_bytes: usize,
}

impl ResourceLimitPolicy {
    pub const DEFAULT_TEXTURE_SIZE: DimensionLimit = DimensionLimit::square(8192);
    pub const DEFAULT_IMAGE_DIMENSIONS: DimensionLimit =
        DimensionLimit::new(16384, 16384, 128 * 1024 * 1024);
    pub const DEFAULT_MAX_FONT_BYTES: usize = 64 * MIB;
    pub const DEFAULT_MAX_CACHE_BYTES: usize = 256 * MIB;

    pub const fn new() -> Self {
        Self {
            texture_size: Self::DEFAULT_TEXTURE_SIZE,
            image_dimensions: Self::DEFAULT_IMAGE_DIMENSIONS,
            max_font_bytes: Self::DEFAULT_MAX_FONT_BYTES,
            max_cache_bytes: Self::DEFAULT_MAX_CACHE_BYTES,
        }
    }

    pub const fn texture_size(mut self, texture_size: DimensionLimit) -> Self {
        self.texture_size = texture_size;
        self
    }

    pub const fn image_dimensions(mut self, image_dimensions: DimensionLimit) -> Self {
        self.image_dimensions = image_dimensions;
        self
    }

    pub const fn max_font_bytes(mut self, max_font_bytes: usize) -> Self {
        self.max_font_bytes = max_font_bytes;
        self
    }

    pub const fn max_cache_bytes(mut self, max_cache_bytes: usize) -> Self {
        self.max_cache_bytes = max_cache_bytes;
        self
    }
}

impl Default for ResourceLimitPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputLimitPolicy {
    pub max_text_input_bytes: usize,
    pub max_paste_bytes: usize,
    pub max_virtualized_rows: usize,
    pub max_drag_drop_payload_bytes: usize,
}

impl InputLimitPolicy {
    pub const DEFAULT_MAX_TEXT_INPUT_BYTES: usize = 64 * 1024;
    pub const DEFAULT_MAX_PASTE_BYTES: usize = MIB;
    pub const DEFAULT_MAX_VIRTUALIZED_ROWS: usize = 1_000_000;
    pub const DEFAULT_MAX_DRAG_DROP_PAYLOAD_BYTES: usize = 64 * MIB;

    pub const fn new() -> Self {
        Self {
            max_text_input_bytes: Self::DEFAULT_MAX_TEXT_INPUT_BYTES,
            max_paste_bytes: Self::DEFAULT_MAX_PASTE_BYTES,
            max_virtualized_rows: Self::DEFAULT_MAX_VIRTUALIZED_ROWS,
            max_drag_drop_payload_bytes: Self::DEFAULT_MAX_DRAG_DROP_PAYLOAD_BYTES,
        }
    }

    pub const fn max_text_input_bytes(mut self, max_text_input_bytes: usize) -> Self {
        self.max_text_input_bytes = max_text_input_bytes;
        self
    }

    pub const fn max_paste_bytes(mut self, max_paste_bytes: usize) -> Self {
        self.max_paste_bytes = max_paste_bytes;
        self
    }

    pub const fn max_virtualized_rows(mut self, max_virtualized_rows: usize) -> Self {
        self.max_virtualized_rows = max_virtualized_rows;
        self
    }

    pub const fn max_drag_drop_payload_bytes(mut self, max_drag_drop_payload_bytes: usize) -> Self {
        self.max_drag_drop_payload_bytes = max_drag_drop_payload_bytes;
        self
    }
}

impl Default for InputLimitPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LimitPolicy {
    pub resources: ResourceLimitPolicy,
    pub input: InputLimitPolicy,
}

impl LimitPolicy {
    pub const fn new(resources: ResourceLimitPolicy, input: InputLimitPolicy) -> Self {
        Self { resources, input }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitKind {
    ResourceId,
    TextureSize,
    ImageDimensions,
    FontBytes,
    TextInputBytes,
    PasteBytes,
    VirtualizedRows,
    DragDropPayloadBytes,
    CacheMemoryBytes,
    ResourceUpdateBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitStatus {
    Accepted,
    Rejected,
    Clamped,
    Truncated,
    NeedsEviction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitValue {
    None,
    Bytes(usize),
    Count(usize),
    Dimensions { width: u32, height: u32 },
    Pixels(u64),
    ResourceId(String),
}

impl LimitValue {
    pub const fn bytes(value: usize) -> Self {
        Self::Bytes(value)
    }

    pub const fn count(value: usize) -> Self {
        Self::Count(value)
    }

    pub const fn dimensions(size: PixelSize) -> Self {
        Self::Dimensions {
            width: size.width,
            height: size.height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitReport {
    pub kind: LimitKind,
    pub status: LimitStatus,
    pub subject: Option<String>,
    pub observed: LimitValue,
    pub limit: LimitValue,
    pub error: Option<ErrorReport>,
    pub fallback: FallbackDecision,
}

impl LimitReport {
    pub fn accepted(kind: LimitKind, observed: LimitValue, limit: LimitValue) -> Self {
        Self {
            kind,
            status: LimitStatus::Accepted,
            subject: None,
            observed,
            limit,
            error: None,
            fallback: FallbackDecision::none(),
        }
    }

    pub fn rejected(
        kind: LimitKind,
        observed: LimitValue,
        limit: LimitValue,
        error: ErrorReport,
    ) -> Self {
        let fallback = error.fallback.clone();
        Self {
            kind,
            status: LimitStatus::Rejected,
            subject: None,
            observed,
            limit,
            error: Some(error),
            fallback,
        }
    }

    pub fn with_status(mut self, status: LimitStatus) -> Self {
        self.status = status;
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub const fn is_accepted(&self) -> bool {
        matches!(self.status, LimitStatus::Accepted)
    }

    pub const fn requires_fallback(&self) -> bool {
        !self.is_accepted()
    }

    pub fn clamped_count(&self) -> Option<usize> {
        match (self.status, &self.limit) {
            (LimitStatus::Clamped, LimitValue::Count(limit)) => Some(*limit),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheBudgetAction {
    Admit,
    EvictToFit { bytes_to_free: usize },
    RejectItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheBudgetDecision {
    pub action: CacheBudgetAction,
    pub current_bytes: usize,
    pub incoming_bytes: usize,
    pub max_bytes: usize,
    pub report: LimitReport,
}

impl CacheBudgetDecision {
    pub const fn is_admitted(&self) -> bool {
        matches!(self.action, CacheBudgetAction::Admit)
    }

    pub const fn bytes_to_free(&self) -> usize {
        match self.action {
            CacheBudgetAction::EvictToFit { bytes_to_free } => bytes_to_free,
            CacheBudgetAction::Admit | CacheBudgetAction::RejectItem => 0,
        }
    }
}

pub fn validate_resource_id(resource: &ResourceId, kind: ResourceKind) -> LimitReport {
    let observed = LimitValue::ResourceId(resource.key.clone());
    if resource.key.trim().is_empty() {
        return LimitReport::rejected(
            LimitKind::ResourceId,
            observed,
            LimitValue::None,
            ErrorReport::recoverable(
                ErrorKind::Resource(ResourceErrorKind::EmptyId),
                "resource id must not be empty",
            )
            .resource_id(resource)
            .context("resource_kind", format!("{kind:?}"))
            .fallback(FallbackDecision::skip_render_item(
                "skip resource with an empty id",
            )),
        )
        .subject(format!("{kind:?}"));
    }

    if resource.key.trim() != resource.key || resource.key.chars().any(char::is_control) {
        return LimitReport::rejected(
            LimitKind::ResourceId,
            observed,
            LimitValue::None,
            ErrorReport::recoverable(
                ErrorKind::Resource(ResourceErrorKind::MalformedId),
                "resource id contains leading, trailing, or control whitespace",
            )
            .resource_id(resource)
            .context("resource_kind", format!("{kind:?}"))
            .fallback(FallbackDecision::skip_render_item(
                "skip resource with a malformed id",
            )),
        )
        .subject(format!("{kind:?}"));
    }

    LimitReport::accepted(LimitKind::ResourceId, observed, LimitValue::None)
        .subject(format!("{kind:?}"))
}

pub fn validate_texture_size(
    resource: &ResourceId,
    size: PixelSize,
    policy: &ResourceLimitPolicy,
) -> LimitReport {
    validate_dimensions(
        LimitKind::TextureSize,
        resource,
        ResourceKind::Texture,
        size,
        policy.texture_size,
        ResourceErrorKind::OversizedTexture,
        "texture exceeds the configured size policy",
    )
}

pub fn validate_image_dimensions(
    resource: &ResourceId,
    size: PixelSize,
    policy: &ResourceLimitPolicy,
) -> LimitReport {
    validate_dimensions(
        LimitKind::ImageDimensions,
        resource,
        ResourceKind::Image,
        size,
        policy.image_dimensions,
        ResourceErrorKind::OversizedImage,
        "image exceeds the configured dimension policy",
    )
}

pub fn validate_font_bytes(
    resource: &ResourceId,
    byte_len: usize,
    policy: &ResourceLimitPolicy,
) -> LimitReport {
    let observed = LimitValue::Bytes(byte_len);
    let limit = LimitValue::Bytes(policy.max_font_bytes);
    if byte_len <= policy.max_font_bytes {
        return LimitReport::accepted(LimitKind::FontBytes, observed, limit)
            .subject(resource.key.clone());
    }

    LimitReport::rejected(
        LimitKind::FontBytes,
        observed,
        limit,
        ErrorReport::recoverable(
            ErrorKind::Resource(ResourceErrorKind::OversizedFont),
            "font resource exceeds the configured byte limit",
        )
        .resource_id(resource)
        .context("observed_bytes", byte_len.to_string())
        .context("limit_bytes", policy.max_font_bytes.to_string())
        .fallback(FallbackDecision::skip_render_item(
            "ignore oversized font resource",
        )),
    )
    .subject(resource.key.clone())
}

pub fn validate_resource_descriptor(
    descriptor: &ResourceDescriptor,
    policy: &ResourceLimitPolicy,
) -> LimitReport {
    let resource = descriptor.handle.id();
    let kind = descriptor.handle.kind();
    let id_report = validate_resource_id(resource, kind);
    if !id_report.is_accepted() {
        return id_report;
    }

    match kind {
        ResourceKind::Texture => validate_texture_size(resource, descriptor.size, policy),
        ResourceKind::Image | ResourceKind::Icon | ResourceKind::Thumbnail => validate_dimensions(
            LimitKind::ImageDimensions,
            resource,
            kind,
            descriptor.size,
            policy.image_dimensions,
            ResourceErrorKind::OversizedImage,
            "image resource exceeds the configured dimension policy",
        ),
    }
}

pub fn validate_resource_update(
    update: &ResourceUpdate,
    policy: &ResourceLimitPolicy,
) -> LimitReport {
    let descriptor_report = validate_resource_descriptor(&update.descriptor, policy);
    if !descriptor_report.is_accepted() {
        return descriptor_report;
    }

    if !update.dirty_rect_is_valid() {
        let resource = update.descriptor.handle.id();
        return LimitReport::rejected(
            LimitKind::ResourceUpdateBytes,
            LimitValue::dimensions(update.descriptor.size),
            LimitValue::dimensions(update.descriptor.size),
            ErrorReport::recoverable(
                ErrorKind::Resource(ResourceErrorKind::InvalidDimensions),
                "resource update dirty rect is empty or outside the descriptor size",
            )
            .resource_id(resource)
            .fallback(FallbackDecision::skip_render_item(
                "skip invalid resource update",
            )),
        )
        .subject(resource.key.clone());
    }

    let expected = match update.expected_byte_len() {
        Some(expected) => expected,
        None => {
            let resource = update.descriptor.handle.id();
            return LimitReport::rejected(
                LimitKind::ResourceUpdateBytes,
                LimitValue::Bytes(update.bytes.len()),
                LimitValue::None,
                ErrorReport::recoverable(
                    ErrorKind::Resource(ResourceErrorKind::ByteLengthMismatch),
                    "resource update byte length overflowed the expected size",
                )
                .resource_id(resource)
                .fallback(FallbackDecision::skip_render_item(
                    "skip invalid resource update",
                )),
            )
            .subject(resource.key.clone());
        }
    };

    if update.bytes.len() != expected {
        let resource = update.descriptor.handle.id();
        return LimitReport::rejected(
            LimitKind::ResourceUpdateBytes,
            LimitValue::Bytes(update.bytes.len()),
            LimitValue::Bytes(expected),
            ErrorReport::recoverable(
                ErrorKind::Resource(ResourceErrorKind::ByteLengthMismatch),
                "resource update byte length does not match the descriptor",
            )
            .resource_id(resource)
            .context("observed_bytes", update.bytes.len().to_string())
            .context("expected_bytes", expected.to_string())
            .fallback(FallbackDecision::skip_render_item(
                "skip invalid resource update",
            )),
        )
        .subject(resource.key.clone());
    }

    LimitReport::accepted(
        LimitKind::ResourceUpdateBytes,
        LimitValue::Bytes(update.bytes.len()),
        LimitValue::Bytes(expected),
    )
    .subject(update.descriptor.handle.id().key.clone())
}

pub fn validate_text_input(text: &str, policy: &InputLimitPolicy) -> LimitReport {
    validate_text_bytes(
        LimitKind::TextInputBytes,
        text.len(),
        policy.max_text_input_bytes,
        "text input exceeds the configured byte limit",
    )
}

pub fn validate_paste_payload(text: &str, policy: &InputLimitPolicy) -> LimitReport {
    validate_text_bytes(
        LimitKind::PasteBytes,
        text.len(),
        policy.max_paste_bytes,
        "paste payload exceeds the configured byte limit",
    )
}

pub fn truncate_str_to_byte_limit(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }

    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    &text[..end]
}

pub fn validate_virtualized_row_count(row_count: usize, policy: &InputLimitPolicy) -> LimitReport {
    let observed = LimitValue::Count(row_count);
    let limit = LimitValue::Count(policy.max_virtualized_rows);
    if row_count <= policy.max_virtualized_rows {
        return LimitReport::accepted(LimitKind::VirtualizedRows, observed, limit);
    }

    LimitReport::rejected(
        LimitKind::VirtualizedRows,
        observed,
        limit,
        ErrorReport::recoverable(
            ErrorKind::Ui(UiErrorKind::Input),
            "virtualized row count exceeds the configured limit",
        )
        .context("observed_rows", row_count.to_string())
        .context("limit_rows", policy.max_virtualized_rows.to_string())
        .fallback(FallbackDecision::clamp_to_limit(
            "clamp virtualized row count to the configured limit",
        )),
    )
    .with_status(LimitStatus::Clamped)
}

pub fn drag_payload_byte_len(payload: &DragPayload) -> usize {
    let mut total = 0usize;
    if let Some(text) = &payload.text {
        total = saturating_add(total, text.len());
    }
    for file in &payload.files {
        total = saturating_add(total, file.to_string_lossy().len());
    }
    for bytes in &payload.bytes {
        total = saturating_add(total, bytes.mime_type.len());
        if let Some(name) = &bytes.name {
            total = saturating_add(total, name.len());
        }
        total = saturating_add(total, bytes.bytes.len());
    }
    total
}

pub fn validate_drag_drop_payload(payload: &DragPayload, policy: &InputLimitPolicy) -> LimitReport {
    let byte_len = drag_payload_byte_len(payload);
    let observed = LimitValue::Bytes(byte_len);
    let limit = LimitValue::Bytes(policy.max_drag_drop_payload_bytes);
    if byte_len <= policy.max_drag_drop_payload_bytes {
        return LimitReport::accepted(LimitKind::DragDropPayloadBytes, observed, limit);
    }

    LimitReport::rejected(
        LimitKind::DragDropPayloadBytes,
        observed,
        limit,
        ErrorReport::recoverable(
            ErrorKind::Ui(UiErrorKind::Input),
            "drag/drop payload exceeds the configured byte limit",
        )
        .context("observed_bytes", byte_len.to_string())
        .context(
            "limit_bytes",
            policy.max_drag_drop_payload_bytes.to_string(),
        )
        .fallback(FallbackDecision::reject_input(
            "reject oversized drag/drop payload",
        )),
    )
}

pub fn validate_cache_budget(
    current_bytes: usize,
    incoming_bytes: usize,
    policy: &ResourceLimitPolicy,
) -> CacheBudgetDecision {
    let max_bytes = policy.max_cache_bytes;
    let total = saturating_add(current_bytes, incoming_bytes);

    if incoming_bytes > max_bytes {
        let report = LimitReport::rejected(
            LimitKind::CacheMemoryBytes,
            LimitValue::Bytes(incoming_bytes),
            LimitValue::Bytes(max_bytes),
            cache_budget_report(
                "cache item exceeds the configured memory budget",
                current_bytes,
                incoming_bytes,
                max_bytes,
                FallbackDecision::skip_render_item("skip cache insertion for oversized item"),
            ),
        );
        return CacheBudgetDecision {
            action: CacheBudgetAction::RejectItem,
            current_bytes,
            incoming_bytes,
            max_bytes,
            report,
        };
    }

    if total > max_bytes {
        let bytes_to_free = total.saturating_sub(max_bytes);
        let report = LimitReport::rejected(
            LimitKind::CacheMemoryBytes,
            LimitValue::Bytes(total),
            LimitValue::Bytes(max_bytes),
            cache_budget_report(
                "cache insertion requires eviction to stay within budget",
                current_bytes,
                incoming_bytes,
                max_bytes,
                FallbackDecision::evict_cache("evict least valuable cache entries"),
            ),
        )
        .with_status(LimitStatus::NeedsEviction);
        return CacheBudgetDecision {
            action: CacheBudgetAction::EvictToFit { bytes_to_free },
            current_bytes,
            incoming_bytes,
            max_bytes,
            report,
        };
    }

    CacheBudgetDecision {
        action: CacheBudgetAction::Admit,
        current_bytes,
        incoming_bytes,
        max_bytes,
        report: LimitReport::accepted(
            LimitKind::CacheMemoryBytes,
            LimitValue::Bytes(total),
            LimitValue::Bytes(max_bytes),
        ),
    }
}

fn validate_dimensions(
    limit_kind: LimitKind,
    resource: &ResourceId,
    resource_kind: ResourceKind,
    size: PixelSize,
    limit: DimensionLimit,
    oversized_kind: ResourceErrorKind,
    message: &'static str,
) -> LimitReport {
    let observed = LimitValue::dimensions(size);
    let allowed = LimitValue::Dimensions {
        width: limit.max_width,
        height: limit.max_height,
    };

    if size_is_empty(size) {
        return LimitReport::rejected(
            limit_kind,
            observed,
            allowed,
            ErrorReport::recoverable(
                ErrorKind::Resource(ResourceErrorKind::InvalidDimensions),
                "resource dimensions must be non-zero",
            )
            .resource_id(resource)
            .context("resource_kind", format!("{resource_kind:?}"))
            .fallback(FallbackDecision::skip_render_item(
                "skip resource with invalid dimensions",
            )),
        )
        .subject(resource.key.clone());
    }

    if limit.accepts(size) {
        return LimitReport::accepted(limit_kind, observed, allowed).subject(resource.key.clone());
    }

    LimitReport::rejected(
        limit_kind,
        observed,
        allowed,
        ErrorReport::recoverable(ErrorKind::Resource(oversized_kind), message)
            .resource_id(resource)
            .context("resource_kind", format!("{resource_kind:?}"))
            .context("width", size.width.to_string())
            .context("height", size.height.to_string())
            .context("pixels", pixel_count(size).to_string())
            .context("max_width", limit.max_width.to_string())
            .context("max_height", limit.max_height.to_string())
            .context("max_pixels", limit.max_pixels.to_string())
            .fallback(FallbackDecision::render_placeholder(
                "render a bounded placeholder instead of the oversized resource",
            )),
    )
    .subject(resource.key.clone())
}

fn validate_text_bytes(
    kind: LimitKind,
    byte_len: usize,
    max_bytes: usize,
    message: &'static str,
) -> LimitReport {
    let observed = LimitValue::Bytes(byte_len);
    let limit = LimitValue::Bytes(max_bytes);
    if byte_len <= max_bytes {
        return LimitReport::accepted(kind, observed, limit);
    }

    LimitReport::rejected(
        kind,
        observed,
        limit,
        ErrorReport::recoverable(ErrorKind::Ui(UiErrorKind::Input), message)
            .context("observed_bytes", byte_len.to_string())
            .context("limit_bytes", max_bytes.to_string())
            .fallback(FallbackDecision::truncate_input(
                "truncate text payload to the configured byte limit",
            )),
    )
    .with_status(LimitStatus::Truncated)
}

fn cache_budget_report(
    message: &'static str,
    current_bytes: usize,
    incoming_bytes: usize,
    max_bytes: usize,
    fallback: FallbackDecision,
) -> ErrorReport {
    ErrorReport::new(
        ErrorKind::Resource(ResourceErrorKind::CacheBudgetExceeded),
        ErrorSeverity::Recoverable,
        message,
    )
    .context("current_bytes", current_bytes.to_string())
    .context("incoming_bytes", incoming_bytes.to_string())
    .context("max_bytes", max_bytes.to_string())
    .fallback(fallback)
}

const fn size_is_empty(size: PixelSize) -> bool {
    size.width == 0 || size.height == 0
}

const fn pixel_count(size: PixelSize) -> u64 {
    size.width as u64 * size.height as u64
}

fn saturating_add(left: usize, right: usize) -> usize {
    left.saturating_add(right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::{ErrorDomain, FallbackAction};
    use crate::platform::{DragBytes, ResourceDomain, TextureHandle};
    use crate::renderer::{ResourceDescriptor, ResourceFormat};

    #[test]
    fn oversized_texture_is_rejected_with_placeholder_fallback() {
        let policy = ResourceLimitPolicy::new().texture_size(DimensionLimit::new(64, 64, 64 * 64));
        let id = ResourceId::new(ResourceDomain::App, "textures.waveform");
        let report = validate_texture_size(&id, PixelSize::new(65, 64), &policy);

        assert_eq!(report.status, LimitStatus::Rejected);
        assert_eq!(report.kind, LimitKind::TextureSize);
        assert_eq!(report.fallback.action, FallbackAction::RenderPlaceholder);
        assert_eq!(
            report.error.as_ref().map(|error| error.domain),
            Some(ErrorDomain::Resource)
        );
    }

    #[test]
    fn oversized_image_is_rejected_by_area_limit() {
        let policy =
            ResourceLimitPolicy::new().image_dimensions(DimensionLimit::new(128, 128, 1024));
        let id = ResourceId::new(ResourceDomain::App, "images.cover");
        let report = validate_image_dimensions(&id, PixelSize::new(64, 32), &policy);

        assert_eq!(report.status, LimitStatus::Rejected);
        assert_eq!(report.kind, LimitKind::ImageDimensions);
        assert_eq!(
            report.error.as_ref().map(|error| error.kind),
            Some(ErrorKind::Resource(ResourceErrorKind::OversizedImage))
        );
    }

    #[test]
    fn huge_paste_payload_is_truncated_without_panicking() {
        let policy = InputLimitPolicy::new().max_paste_bytes(8);
        let report = validate_paste_payload("abcdefghi", &policy);

        assert_eq!(report.status, LimitStatus::Truncated);
        assert_eq!(report.fallback.action, FallbackAction::TruncateInput);
        assert_eq!(truncate_str_to_byte_limit("abcdéfg", 5), "abcd");
    }

    #[test]
    fn empty_and_malformed_resource_ids_are_rejected() {
        let empty = validate_resource_id(&ResourceId::app(""), ResourceKind::Image);
        assert_eq!(empty.status, LimitStatus::Rejected);
        assert_eq!(
            empty.error.as_ref().map(|error| error.kind),
            Some(ErrorKind::Resource(ResourceErrorKind::EmptyId))
        );

        let malformed = validate_resource_id(&ResourceId::app("bad\nid"), ResourceKind::Texture);
        assert_eq!(malformed.status, LimitStatus::Rejected);
        assert_eq!(
            malformed.error.as_ref().map(|error| error.kind),
            Some(ErrorKind::Resource(ResourceErrorKind::MalformedId))
        );
    }

    #[test]
    fn cache_budget_reports_admit_evict_and_reject_decisions() {
        let policy = ResourceLimitPolicy::new().max_cache_bytes(100);

        let admit = validate_cache_budget(40, 50, &policy);
        assert_eq!(admit.action, CacheBudgetAction::Admit);
        assert!(admit.report.is_accepted());

        let evict = validate_cache_budget(90, 20, &policy);
        assert_eq!(
            evict.action,
            CacheBudgetAction::EvictToFit { bytes_to_free: 10 }
        );
        assert_eq!(evict.report.status, LimitStatus::NeedsEviction);
        assert_eq!(evict.report.fallback.action, FallbackAction::EvictCache);

        let reject = validate_cache_budget(0, 101, &policy);
        assert_eq!(reject.action, CacheBudgetAction::RejectItem);
        assert_eq!(reject.report.status, LimitStatus::Rejected);
    }

    #[test]
    fn resource_update_validation_rejects_byte_length_mismatch() {
        let descriptor = ResourceDescriptor::new(
            crate::platform::ResourceHandle::Texture(TextureHandle::app("textures.meter")),
            PixelSize::new(2, 2),
            ResourceFormat::Rgba8,
        );
        let update = ResourceUpdate::full(descriptor, vec![0; 15]);
        let report = validate_resource_update(&update, &ResourceLimitPolicy::new());

        assert_eq!(report.status, LimitStatus::Rejected);
        assert_eq!(
            report.error.as_ref().map(|error| error.kind),
            Some(ErrorKind::Resource(ResourceErrorKind::ByteLengthMismatch))
        );
    }

    #[test]
    fn drag_drop_payload_uses_total_payload_bytes() {
        let policy = InputLimitPolicy::new().max_drag_drop_payload_bytes(6);
        let payload = DragPayload::bytes(DragBytes::new("x", vec![1, 2, 3, 4, 5, 6]));
        let report = validate_drag_drop_payload(&payload, &policy);

        assert_eq!(report.status, LimitStatus::Rejected);
        assert_eq!(report.fallback.action, FallbackAction::RejectInput);
    }

    #[test]
    fn virtualized_rows_are_clamped_to_policy_limit() {
        let policy = InputLimitPolicy::new().max_virtualized_rows(100);
        let report = validate_virtualized_row_count(101, &policy);

        assert_eq!(report.status, LimitStatus::Clamped);
        assert_eq!(report.clamped_count(), Some(100));
        assert_eq!(report.fallback.action, FallbackAction::ClampToLimit);
    }
}
