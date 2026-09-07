//! Structured error classification and local fallback contracts.

use std::fmt;

use crate::layout::{
    Layout, LayoutAlignment, LayoutDimension, LayoutGap, LayoutInset, LayoutInsets, LayoutPosition,
    LayoutSpacing,
};
use crate::platform::{
    AppLifecycleResponse, ClipboardResponse, CursorResponse, DragDropResponse, FileDialogResponse,
    NotificationResponse, OpenUrlResponse, PlatformErrorCode, PlatformResponse,
    PlatformServiceError, PlatformServiceKind, PlatformServiceResponse, RepaintResponse,
    ResourceId, ScreenshotResponse, TextImeResponse, UiLayer,
};
use crate::renderer::RenderError;
use crate::{
    AccessibilityLiveRegion, AccessibilityMeta, AccessibilityRole, ColorRgba, FontWeight,
    InputBehavior, LayoutStyle, StrokeStyle, TextStyle, TextWrap, UiDocument, UiNode, UiNodeId,
    UiNodeStyle, UiVisual,
};

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
    WindowCreation,
    SurfaceCreation,
    SurfaceConfiguration,
    AdapterRequest,
    DeviceRequest,
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

impl fmt::Display for ErrorReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} {:?} error: {}",
            self.severity, self.domain, self.message
        )?;
        for context in &self.context {
            write!(formatter, "\n{}: {}", context.key, context.value)?;
        }
        if self.fallback.action != FallbackAction::None {
            write!(
                formatter,
                "\nfallback: {:?} ({:?}) - {}",
                self.fallback.action, self.fallback.scope, self.fallback.reason
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for ErrorReport {}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeErrorOverlayOptions {
    pub layout: LayoutStyle,
    pub panel_visual: UiVisual,
    pub fatal_accent: UiVisual,
    pub recoverable_accent: UiVisual,
    pub title_style: TextStyle,
    pub message_style: TextStyle,
    pub context_label_style: TextStyle,
    pub context_value_style: TextStyle,
    pub context_label_width: f32,
    pub max_context_rows: usize,
    pub z_index: f32,
}

impl RuntimeErrorOverlayOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub const fn with_max_context_rows(mut self, rows: usize) -> Self {
        self.max_context_rows = rows;
        self
    }

    pub const fn with_z_index(mut self, z_index: f32) -> Self {
        self.z_index = z_index;
        self
    }
}

impl Default for RuntimeErrorOverlayOptions {
    fn default() -> Self {
        let title_style = TextStyle {
            font_size: 16.0,
            line_height: 22.0,
            weight: FontWeight::BOLD,
            wrap: TextWrap::WordOrGlyph,
            color: ColorRgba::WHITE,
            ..Default::default()
        };
        let message_style = TextStyle {
            font_size: 14.0,
            line_height: 20.0,
            wrap: TextWrap::WordOrGlyph,
            color: ColorRgba::new(230, 235, 245, 255),
            ..Default::default()
        };
        let context_label_style = TextStyle {
            font_size: 12.0,
            line_height: 18.0,
            wrap: TextWrap::WordOrGlyph,
            color: ColorRgba::new(166, 176, 194, 255),
            ..Default::default()
        };
        let context_value_style = TextStyle {
            font_size: 12.0,
            line_height: 18.0,
            wrap: TextWrap::WordOrGlyph,
            color: ColorRgba::new(221, 226, 236, 255),
            ..Default::default()
        };

        Self {
            layout: Layout::column()
                .position(LayoutPosition::Absolute)
                .inset(LayoutInsets::new(
                    LayoutInset::Points(16.0),
                    LayoutInset::Auto,
                    LayoutInset::Points(16.0),
                    LayoutInset::Auto,
                ))
                .width(LayoutDimension::Points(520.0))
                .padding(LayoutSpacing::points(14.0))
                .gap(LayoutGap::points(0.0, 8.0))
                .to_layout_style(),
            panel_visual: UiVisual::panel(
                ColorRgba::new(18, 23, 31, 242),
                Some(StrokeStyle::new(ColorRgba::new(92, 111, 136, 255), 1.0)),
                6.0,
            ),
            fatal_accent: UiVisual::panel(ColorRgba::new(224, 80, 80, 255), None, 2.0),
            recoverable_accent: UiVisual::panel(ColorRgba::new(82, 151, 237, 255), None, 2.0),
            title_style,
            message_style,
            context_label_style,
            context_value_style,
            context_label_width: 132.0,
            max_context_rows: 8,
            z_index: 10_000.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeErrorOverlayNodes {
    pub root: UiNodeId,
    pub accent: UiNodeId,
    pub title: UiNodeId,
    pub message: UiNodeId,
    pub context_rows: Vec<RuntimeErrorOverlayContextRow>,
    pub fallback: Option<UiNodeId>,
    pub truncated_context: Option<UiNodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeErrorOverlayContextRow {
    pub row: UiNodeId,
    pub key: UiNodeId,
    pub value: UiNodeId,
}

pub fn runtime_error_overlay(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    report: &ErrorReport,
    options: RuntimeErrorOverlayOptions,
) -> RuntimeErrorOverlayNodes {
    let name = name.into();
    let mut root = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style.clone(),
            z_index: options.z_index,
            ..Default::default()
        },
    )
    .with_layer(UiLayer::DebugOverlay)
    .with_clip_scope(crate::ClipScope::Viewport)
    .with_visual(options.panel_visual)
    .with_input(InputBehavior {
        pointer: true,
        focusable: false,
        keyboard: false,
    })
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Alert)
            .label(error_overlay_title(report))
            .value(report.message.clone())
            .hint(error_overlay_hint(report))
            .live_region(if report.is_fatal() {
                AccessibilityLiveRegion::Assertive
            } else {
                AccessibilityLiveRegion::Polite
            }),
    );
    root.style.z_index = options.z_index;
    let root = document.add_child(parent, root);

    let accent = document.add_child(
        root,
        UiNode::container(
            format!("{name}.accent"),
            Layout::fixed(4.0, 28.0).to_layout_style(),
        )
        .with_visual(if report.is_fatal() {
            options.fatal_accent
        } else {
            options.recoverable_accent
        }),
    );

    let title = document.add_child(
        root,
        UiNode::text(
            format!("{name}.title"),
            error_overlay_title(report),
            options.title_style.clone(),
            Layout::new()
                .width(LayoutDimension::Percent(1.0))
                .to_layout_style(),
        ),
    );

    let message = document.add_child(
        root,
        UiNode::text(
            format!("{name}.message"),
            report.message.clone(),
            options.message_style.clone(),
            Layout::new()
                .width(LayoutDimension::Percent(1.0))
                .to_layout_style(),
        ),
    );

    let mut context_rows = Vec::new();
    for context in report.context.iter().take(options.max_context_rows) {
        context_rows.push(error_overlay_context_row(
            document,
            root,
            &name,
            &context.key,
            &context.value,
            &options,
        ));
    }

    let truncated_context = if report.context.len() > options.max_context_rows {
        Some(
            document.add_child(
                root,
                UiNode::text(
                    format!("{name}.context.truncated"),
                    format!(
                        "{} more context row(s)",
                        report.context.len() - options.max_context_rows
                    ),
                    options.context_label_style.clone(),
                    Layout::new()
                        .width(LayoutDimension::Percent(1.0))
                        .to_layout_style(),
                ),
            ),
        )
    } else {
        None
    };

    let fallback = if report.fallback.action == FallbackAction::None {
        None
    } else {
        Some(
            document.add_child(
                root,
                UiNode::text(
                    format!("{name}.fallback"),
                    format!(
                        "Fallback: {:?} ({:?}) - {}",
                        report.fallback.action, report.fallback.scope, report.fallback.reason
                    ),
                    options.context_value_style,
                    Layout::new()
                        .width(LayoutDimension::Percent(1.0))
                        .to_layout_style(),
                ),
            ),
        )
    };

    RuntimeErrorOverlayNodes {
        root,
        accent,
        title,
        message,
        context_rows,
        fallback,
        truncated_context,
    }
}

fn error_overlay_context_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    key: &str,
    value: &str,
    options: &RuntimeErrorOverlayOptions,
) -> RuntimeErrorOverlayContextRow {
    let row_name = format!("{name}.context.{key}");
    let row = document.add_child(
        parent,
        UiNode::container(
            row_name.clone(),
            Layout::row()
                .align_items(LayoutAlignment::Start)
                .gap(LayoutGap::points(8.0, 0.0))
                .to_layout_style(),
        ),
    );
    let key = document.add_child(
        row,
        UiNode::text(
            format!("{row_name}.key"),
            key.to_string(),
            options.context_label_style.clone(),
            Layout::new()
                .width(LayoutDimension::Points(options.context_label_width))
                .to_layout_style(),
        ),
    );
    let value = document.add_child(
        row,
        UiNode::text(
            format!("{row_name}.value"),
            value.to_string(),
            options.context_value_style.clone(),
            Layout::new()
                .width(LayoutDimension::Percent(1.0))
                .to_layout_style(),
        ),
    );
    RuntimeErrorOverlayContextRow { row, key, value }
}

fn error_overlay_title(report: &ErrorReport) -> String {
    format!("{:?} {:?} error", report.severity, report.domain)
}

fn error_overlay_hint(report: &ErrorReport) -> String {
    let mut parts = Vec::new();
    if let Some(consequence) = error_context_value(report, "user_visible_consequence") {
        parts.push(format!("Consequence: {consequence}"));
    }
    if let Some(next_step) = error_context_value(report, "next_step") {
        parts.push(format!("Next step: {next_step}"));
    }
    if report.fallback.action != FallbackAction::None {
        parts.push(format!(
            "Fallback: {:?}, {:?}",
            report.fallback.action, report.fallback.scope
        ));
    }
    if parts.is_empty() {
        "Runtime error details are visible in the overlay.".to_string()
    } else {
        parts.join(". ")
    }
}

fn error_context_value<'a>(report: &'a ErrorReport, key: &str) -> Option<&'a str> {
    report
        .context
        .iter()
        .find(|context| context.key == key)
        .map(|context| context.value.as_str())
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

pub fn classify_platform_service_response(
    response: &PlatformServiceResponse,
) -> Option<ErrorReport> {
    let service = response.kind();
    let report = match platform_response_failure(&response.response)? {
        PlatformResponseFailure::Unsupported => ErrorReport::recoverable(
            ErrorKind::Platform(PlatformErrorKind::Unsupported),
            format!("host does not support the {service:?} platform service"),
        )
        .context("service", format!("{service:?}"))
        .context(
            "next_step",
            "Check BackendCapabilities before issuing this request, or provide a disabled state, fallback behavior, or explicit diagnostic.",
        )
        .fallback(FallbackDecision::disable_feature(
            "disable unavailable platform service",
        )),
        PlatformResponseFailure::Denied(message) => ErrorReport::recoverable(
            ErrorKind::Platform(PlatformErrorKind::Denied),
            message,
        )
        .context("service", format!("{service:?}"))
        .context(
            "next_step",
            "Keep the control visible only if it can explain the denial; otherwise disable the dependent feature or offer a fallback.",
        )
        .fallback(FallbackDecision::disable_feature(
            "disable denied platform service",
        )),
        PlatformResponseFailure::Error(error) => classify_platform_error(service, error).context(
            "next_step",
            "Surface the platform error near the dependent control and choose a fallback that matches the error severity.",
        ),
    };

    Some(
        report
            .context("platform_request_id", response.id.value().to_string())
            .context("operation", "processing platform service response")
            .context("host_subsystem", "platform service")
            .context(
                "user_visible_consequence",
                "the requested platform feature did not complete",
            ),
    )
}

enum PlatformResponseFailure<'a> {
    Unsupported,
    Denied(&'static str),
    Error(&'a PlatformServiceError),
}

fn platform_response_failure(response: &PlatformResponse) -> Option<PlatformResponseFailure<'_>> {
    match response {
        PlatformResponse::Clipboard(ClipboardResponse::Unsupported)
        | PlatformResponse::FileDialog(FileDialogResponse::Unsupported)
        | PlatformResponse::OpenUrl(OpenUrlResponse::Unsupported)
        | PlatformResponse::Notification(NotificationResponse::Unsupported)
        | PlatformResponse::Screenshot(ScreenshotResponse::Unsupported)
        | PlatformResponse::AppLifecycle(AppLifecycleResponse::Unsupported)
        | PlatformResponse::TextIme(TextImeResponse::Unsupported)
        | PlatformResponse::DragDrop(DragDropResponse::Unsupported)
        | PlatformResponse::Cursor(CursorResponse::Unsupported)
        | PlatformResponse::Repaint(RepaintResponse::Unsupported) => {
            Some(PlatformResponseFailure::Unsupported)
        }
        PlatformResponse::OpenUrl(OpenUrlResponse::Blocked) => Some(
            PlatformResponseFailure::Denied("open URL request was blocked by the host"),
        ),
        PlatformResponse::Clipboard(ClipboardResponse::Error(error))
        | PlatformResponse::FileDialog(FileDialogResponse::Error(error))
        | PlatformResponse::OpenUrl(OpenUrlResponse::Error(error))
        | PlatformResponse::Notification(NotificationResponse::Error(error))
        | PlatformResponse::Screenshot(ScreenshotResponse::Error(error))
        | PlatformResponse::AppLifecycle(AppLifecycleResponse::Error(error))
        | PlatformResponse::TextIme(TextImeResponse::Error(error))
        | PlatformResponse::DragDrop(DragDropResponse::Error(error))
        | PlatformResponse::Cursor(CursorResponse::Error(error))
        | PlatformResponse::Repaint(RepaintResponse::Error(error)) => {
            Some(PlatformResponseFailure::Error(error))
        }
        _ => None,
    }
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
    use crate::platform::{PlatformRequestId, ResourceDomain, ResourceKind};
    use crate::{root_style, ApproxTextMeasurer, UiContent, UiSize};

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
    fn platform_service_response_classification_reports_unsupported_and_denied_capabilities() {
        let unsupported = PlatformServiceResponse::new(
            PlatformRequestId::new(42),
            PlatformResponse::Cursor(CursorResponse::Unsupported),
        );
        let report = classify_platform_service_response(&unsupported)
            .expect("unsupported platform response should produce report");

        assert_eq!(
            report.kind,
            ErrorKind::Platform(PlatformErrorKind::Unsupported)
        );
        assert_eq!(report.fallback.action, FallbackAction::DisableFeature);
        assert!(report.fallback.user_visible);
        assert!(report
            .context
            .iter()
            .any(|context| context.key == "platform_request_id" && context.value == "42"));
        assert!(report
            .context
            .iter()
            .any(|context| context.key == "next_step"
                && context.value.contains("BackendCapabilities")));

        let denied = PlatformServiceResponse::new(
            PlatformRequestId::new(43),
            PlatformResponse::OpenUrl(OpenUrlResponse::Blocked),
        );
        let report = classify_platform_service_response(&denied)
            .expect("denied platform response should produce report");

        assert_eq!(report.kind, ErrorKind::Platform(PlatformErrorKind::Denied));
        assert!(report.message.contains("blocked"));
        assert_eq!(report.fallback.action, FallbackAction::DisableFeature);

        let applied = PlatformServiceResponse::new(
            PlatformRequestId::new(44),
            PlatformResponse::Cursor(CursorResponse::Applied),
        );
        assert!(classify_platform_service_response(&applied).is_none());
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

    #[test]
    fn runtime_error_overlay_builds_accessible_actionable_surface() {
        let report = ErrorReport::fatal(
            ErrorKind::Runtime(RuntimeErrorKind::SurfaceCreation),
            "failed to create a WGPU surface",
        )
        .context("backend", "wgpu")
        .context("operation", "creating the WGPU surface")
        .context("user_visible_consequence", "the native window cannot draw")
        .context("next_step", "check GPU drivers and window-system support")
        .fallback(FallbackDecision::abort_frame(
            "surface creation must complete before the first frame",
        ));
        let mut document = UiDocument::new(root_style(640.0, 360.0));
        let root = document.root;

        let nodes = runtime_error_overlay(
            &mut document,
            root,
            "runtime.error",
            &report,
            RuntimeErrorOverlayOptions::default(),
        );

        document
            .compute_layout(UiSize::new(640.0, 360.0), &mut ApproxTextMeasurer)
            .expect("runtime error overlay layout");

        let root = document.node(nodes.root);
        assert_eq!(root.layer, Some(UiLayer::DebugOverlay));
        assert!(root.input.pointer);
        let accessibility = root.accessibility.as_ref().expect("overlay accessibility");
        assert_eq!(accessibility.role, AccessibilityRole::Alert);
        assert_eq!(
            accessibility.live_region,
            AccessibilityLiveRegion::Assertive
        );
        assert!(accessibility
            .hint
            .as_deref()
            .unwrap_or_default()
            .contains("Next step: check GPU drivers"));
        assert_eq!(nodes.context_rows.len(), 4);
        assert!(nodes.fallback.is_some());

        assert_eq!(
            text_content(document.node(nodes.title)),
            "Fatal Runtime error"
        );
        assert_eq!(
            text_content(document.node(nodes.message)),
            "failed to create a WGPU surface"
        );
        assert!(text_content(document.node(nodes.fallback.unwrap())).contains("AbortFrame"));
    }

    #[test]
    fn runtime_error_overlay_truncates_large_context_lists() {
        let mut report = ErrorReport::recoverable(
            ErrorKind::Renderer(RendererErrorKind::MissingResource),
            "missing atlas",
        );
        for index in 0..6 {
            report = report.context(format!("context_{index}"), index.to_string());
        }
        let mut document = UiDocument::new(root_style(480.0, 240.0));
        let root = document.root;

        let nodes = runtime_error_overlay(
            &mut document,
            root,
            "runtime.warning",
            &report,
            RuntimeErrorOverlayOptions::default().with_max_context_rows(3),
        );

        assert_eq!(nodes.context_rows.len(), 3);
        assert!(nodes.truncated_context.is_some());
        assert_eq!(
            document
                .node(nodes.root)
                .accessibility
                .as_ref()
                .expect("overlay accessibility")
                .live_region,
            AccessibilityLiveRegion::Polite
        );
        assert!(text_content(document.node(nodes.truncated_context.unwrap())).contains("3 more"));
    }

    fn text_content(node: &UiNode) -> &str {
        match &node.content {
            UiContent::Text(text) => &text.text,
            _ => panic!("expected text node"),
        }
    }
}
