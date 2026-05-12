//! Structured error classification and local fallback contracts.

use crate::platform::{PlatformErrorCode, PlatformServiceError, PlatformServiceKind, ResourceId};
use crate::renderer::RenderError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorDomain {
    Ui,
    Runtime,
    Renderer,
    Resource,
    Platform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorSeverity {
    Recoverable,
    Fatal,
}

impl ErrorSeverity {
    pub const fn is_recoverable(self) -> bool {
        matches!(self, Self::Recoverable)
    }

    pub const fn is_fatal(self) -> bool {
        matches!(self, Self::Fatal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiErrorKind {
    InvalidDocument,
    Layout,
    Focus,
    Input,
    Command,
    Accessibility,
    Theme,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeErrorKind {
    InvalidState,
    EventRouting,
    FrameScheduling,
    RepaintLoop,
    Adapter,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RendererErrorKind {
    UnsupportedTarget,
    UnsupportedResource,
    MissingResource,
    MissingCanvasRenderer,
    MissingImageRenderer,
    InvalidResourceUpdate,
    Backend,
    OutOfMemory,
    DeviceLost,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceErrorKind {
    EmptyId,
    MalformedId,
    InvalidDimensions,
    Missing,
    UnsupportedKind,
    OversizedTexture,
    OversizedImage,
    OversizedFont,
    ByteLengthMismatch,
    CacheBudgetExceeded,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformErrorKind {
    Unsupported,
    Denied,
    Cancelled,
    InvalidRequest,
    Failed,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Ui(UiErrorKind),
    Runtime(RuntimeErrorKind),
    Renderer(RendererErrorKind),
    Resource(ResourceErrorKind),
    Platform(PlatformErrorKind),
}

impl ErrorKind {
    pub const fn domain(self) -> ErrorDomain {
        match self {
            Self::Ui(_) => ErrorDomain::Ui,
            Self::Runtime(_) => ErrorDomain::Runtime,
            Self::Renderer(_) => ErrorDomain::Renderer,
            Self::Resource(_) => ErrorDomain::Resource,
            Self::Platform(_) => ErrorDomain::Platform,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorContext {
    pub key: String,
    pub value: String,
}

impl ErrorContext {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FallbackScope {
    Local,
    Frame,
    Application,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FallbackAction {
    None,
    RenderPlaceholder,
    SkipRenderItem,
    UseCachedFrame,
    ClampToLimit,
    TruncateInput,
    RejectInput,
    EvictCache,
    ClearCache,
    DisableFeature,
    AbortFrame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackDecision {
    pub action: FallbackAction,
    pub scope: FallbackScope,
    pub reason: String,
    pub user_visible: bool,
}

impl FallbackDecision {
    pub fn new(action: FallbackAction, scope: FallbackScope, reason: impl Into<String>) -> Self {
        Self {
            action,
            scope,
            reason: reason.into(),
            user_visible: false,
        }
    }

    pub fn none() -> Self {
        Self::new(FallbackAction::None, FallbackScope::Local, "")
    }

    pub fn render_placeholder(reason: impl Into<String>) -> Self {
        Self::new(
            FallbackAction::RenderPlaceholder,
            FallbackScope::Local,
            reason,
        )
        .user_visible(true)
    }

    pub fn skip_render_item(reason: impl Into<String>) -> Self {
        Self::new(FallbackAction::SkipRenderItem, FallbackScope::Local, reason)
    }

    pub fn use_cached_frame(reason: impl Into<String>) -> Self {
        Self::new(FallbackAction::UseCachedFrame, FallbackScope::Frame, reason).user_visible(true)
    }

    pub fn clamp_to_limit(reason: impl Into<String>) -> Self {
        Self::new(FallbackAction::ClampToLimit, FallbackScope::Local, reason)
    }

    pub fn truncate_input(reason: impl Into<String>) -> Self {
        Self::new(FallbackAction::TruncateInput, FallbackScope::Local, reason).user_visible(true)
    }

    pub fn reject_input(reason: impl Into<String>) -> Self {
        Self::new(FallbackAction::RejectInput, FallbackScope::Local, reason).user_visible(true)
    }

    pub fn evict_cache(reason: impl Into<String>) -> Self {
        Self::new(FallbackAction::EvictCache, FallbackScope::Local, reason)
    }

    pub fn abort_frame(reason: impl Into<String>) -> Self {
        Self::new(FallbackAction::AbortFrame, FallbackScope::Frame, reason).user_visible(true)
    }

    pub const fn is_local(&self) -> bool {
        matches!(self.scope, FallbackScope::Local)
    }

    pub const fn user_visible(mut self, user_visible: bool) -> Self {
        self.user_visible = user_visible;
        self
    }
}

impl Default for FallbackDecision {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorReport {
    pub kind: ErrorKind,
    pub domain: ErrorDomain,
    pub severity: ErrorSeverity,
    pub message: String,
    pub context: Vec<ErrorContext>,
    pub fallback: FallbackDecision,
}

impl ErrorReport {
    pub fn new(kind: ErrorKind, severity: ErrorSeverity, message: impl Into<String>) -> Self {
        Self {
            domain: kind.domain(),
            kind,
            severity,
            message: message.into(),
            context: Vec::new(),
            fallback: FallbackDecision::none(),
        }
    }

    pub fn recoverable(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::new(kind, ErrorSeverity::Recoverable, message)
    }

    pub fn fatal(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::new(kind, ErrorSeverity::Fatal, message)
    }

    pub fn context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.push(ErrorContext::new(key, value));
        self
    }

    pub fn resource_id(self, resource: &ResourceId) -> Self {
        self.context("resource_domain", format!("{:?}", resource.domain))
            .context("resource_key", resource.key.clone())
    }

    pub fn fallback(mut self, fallback: FallbackDecision) -> Self {
        self.fallback = fallback;
        self
    }

    pub const fn is_recoverable(&self) -> bool {
        self.severity.is_recoverable()
    }

    pub const fn is_fatal(&self) -> bool {
        self.severity.is_fatal()
    }
}

pub fn classify_render_error(error: &RenderError) -> ErrorReport {
    match error {
        RenderError::UnsupportedTarget(target) => ErrorReport::fatal(
            ErrorKind::Renderer(RendererErrorKind::UnsupportedTarget),
            format!("unsupported render target {target:?}"),
        )
        .fallback(FallbackDecision::abort_frame(
            "renderer cannot draw to the requested target",
        )),
        RenderError::UnsupportedResource(kind) => ErrorReport::recoverable(
            ErrorKind::Renderer(RendererErrorKind::UnsupportedResource),
            format!("unsupported render resource {kind:?}"),
        )
        .context("resource_kind", format!("{kind:?}"))
        .fallback(FallbackDecision::render_placeholder(
            "render unsupported resource placeholder",
        )),
        RenderError::MissingResource(resource) => ErrorReport::recoverable(
            ErrorKind::Renderer(RendererErrorKind::MissingResource),
            format!("missing render resource {:?}", resource.key),
        )
        .resource_id(resource)
        .fallback(FallbackDecision::render_placeholder(
            "render missing resource placeholder",
        )),
        RenderError::MissingCanvasRenderer(key) => ErrorReport::recoverable(
            ErrorKind::Renderer(RendererErrorKind::MissingCanvasRenderer),
            format!("missing canvas renderer for {key:?}"),
        )
        .context("canvas_key", key.clone())
        .fallback(FallbackDecision::skip_render_item(
            "skip canvas until a renderer is registered",
        )),
        RenderError::MissingImageRenderer(key) => ErrorReport::recoverable(
            ErrorKind::Renderer(RendererErrorKind::MissingImageRenderer),
            format!("missing image renderer for {key:?}"),
        )
        .context("image_key", key.clone())
        .fallback(FallbackDecision::render_placeholder(
            "render image placeholder until a renderer is registered",
        )),
        RenderError::InvalidResourceUpdate(reason) => ErrorReport::recoverable(
            ErrorKind::Renderer(RendererErrorKind::InvalidResourceUpdate),
            format!("invalid render resource update: {reason}"),
        )
        .context("reason", reason.clone())
        .fallback(FallbackDecision::skip_render_item(
            "skip invalid resource update",
        )),
        RenderError::Backend(reason) => ErrorReport::fatal(
            ErrorKind::Renderer(RendererErrorKind::Backend),
            reason.clone(),
        )
        .fallback(FallbackDecision::use_cached_frame(
            "keep the previous frame visible while the renderer recovers",
        )),
    }
}

pub fn classify_platform_error(
    service: PlatformServiceKind,
    error: &PlatformServiceError,
) -> ErrorReport {
    let kind = match error.code {
        PlatformErrorCode::Unsupported => PlatformErrorKind::Unsupported,
        PlatformErrorCode::Denied => PlatformErrorKind::Denied,
        PlatformErrorCode::Cancelled => PlatformErrorKind::Cancelled,
        PlatformErrorCode::InvalidRequest => PlatformErrorKind::InvalidRequest,
        PlatformErrorCode::Failed => PlatformErrorKind::Failed,
    };
    let severity = match error.code {
        PlatformErrorCode::Failed => ErrorSeverity::Fatal,
        PlatformErrorCode::Unsupported
        | PlatformErrorCode::Denied
        | PlatformErrorCode::Cancelled
        | PlatformErrorCode::InvalidRequest => ErrorSeverity::Recoverable,
    };
    let fallback = match error.code {
        PlatformErrorCode::Unsupported | PlatformErrorCode::Denied => {
            FallbackDecision::disable_feature("disable unavailable platform service")
        }
        PlatformErrorCode::Cancelled => FallbackDecision::none(),
        PlatformErrorCode::InvalidRequest => {
            FallbackDecision::reject_input("reject invalid platform request")
        }
        PlatformErrorCode::Failed => {
            FallbackDecision::abort_frame("platform service failure requires host recovery")
        }
    };

    ErrorReport::new(ErrorKind::Platform(kind), severity, error.message.clone())
        .context("service", format!("{service:?}"))
        .fallback(fallback)
}

impl FallbackDecision {
    pub fn disable_feature(reason: impl Into<String>) -> Self {
        Self::new(FallbackAction::DisableFeature, FallbackScope::Local, reason).user_visible(true)
    }
}

impl From<&RenderError> for ErrorReport {
    fn from(value: &RenderError) -> Self {
        classify_render_error(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{ResourceDomain, ResourceKind};

    #[test]
    fn render_error_classification_distinguishes_recoverable_and_fatal() {
        let missing = classify_render_error(&RenderError::MissingResource(ResourceId::new(
            ResourceDomain::App,
            "texture.meter",
        )));
        assert_eq!(missing.severity, ErrorSeverity::Recoverable);
        assert_eq!(missing.domain, ErrorDomain::Renderer);
        assert_eq!(missing.fallback.action, FallbackAction::RenderPlaceholder);
        assert!(missing.fallback.is_local());

        let backend = classify_render_error(&RenderError::Backend("device lost".to_string()));
        assert_eq!(backend.severity, ErrorSeverity::Fatal);
        assert_eq!(backend.fallback.action, FallbackAction::UseCachedFrame);
        assert_eq!(backend.fallback.scope, FallbackScope::Frame);
    }

    #[test]
    fn platform_error_classification_keeps_denials_recoverable() {
        let denied = PlatformServiceError::new(PlatformErrorCode::Denied, "clipboard denied");
        let report = classify_platform_error(PlatformServiceKind::Clipboard, &denied);

        assert!(report.is_recoverable());
        assert_eq!(report.kind, ErrorKind::Platform(PlatformErrorKind::Denied));
        assert_eq!(report.fallback.action, FallbackAction::DisableFeature);
        assert!(report.fallback.is_local());
    }

    #[test]
    fn fallback_rendering_decision_is_local_and_user_visible() {
        let report = ErrorReport::recoverable(
            ErrorKind::Resource(ResourceErrorKind::Missing),
            "image resource is missing",
        )
        .context("resource_kind", format!("{:?}", ResourceKind::Image))
        .fallback(FallbackDecision::render_placeholder(
            "draw a placeholder in the image slot",
        ));

        assert_eq!(report.fallback.action, FallbackAction::RenderPlaceholder);
        assert_eq!(report.fallback.scope, FallbackScope::Local);
        assert!(report.fallback.user_visible);
        assert!(report.is_recoverable());
    }
}
