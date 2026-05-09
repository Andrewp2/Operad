//! Backend-neutral platform contracts for Operad adapters.
//!
//! This module is intentionally data-only. Backends translate these contracts
//! to egui, wgpu, CPU snapshot rendering, host operating-system APIs, or
//! app-owned renderers without leaking backend types into application state.

use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::Duration;

use crate::accessibility::{AccessibilityCapabilities, AccessibilityRequestKind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalPoint {
    pub x: f32,
    pub y: f32,
}

impl LogicalPoint {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalSize {
    pub width: f32,
    pub height: f32,
}

impl LogicalSize {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalRect {
    pub origin: LogicalPoint,
    pub size: LogicalSize,
}

impl LogicalRect {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: LogicalPoint::new(x, y),
            size: LogicalSize::new(width, height),
        }
    }

    pub fn right(self) -> f32 {
        self.origin.x + self.size.width
    }

    pub fn bottom(self) -> f32 {
        self.origin.y + self.size.height
    }

    pub fn contains(self, point: LogicalPoint) -> bool {
        point.x >= self.origin.x
            && point.x <= self.right()
            && point.y >= self.origin.y
            && point.y <= self.bottom()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
}

impl PixelSize {
    pub const ZERO: Self = Self::new(0, 0);

    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceDomain {
    BuiltIn,
    App,
    Host,
}

impl Default for ResourceDomain {
    fn default() -> Self {
        Self::App
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Image,
    Icon,
    Texture,
    Thumbnail,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId {
    pub domain: ResourceDomain,
    pub key: String,
}

impl ResourceId {
    pub fn new(domain: ResourceDomain, key: impl Into<String>) -> Self {
        Self {
            domain,
            key: key.into(),
        }
    }

    pub fn app(key: impl Into<String>) -> Self {
        Self::new(ResourceDomain::App, key)
    }

    pub fn built_in(key: impl Into<String>) -> Self {
        Self::new(ResourceDomain::BuiltIn, key)
    }

    pub fn host(key: impl Into<String>) -> Self {
        Self::new(ResourceDomain::Host, key)
    }

    pub fn is_empty(&self) -> bool {
        self.key.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageHandle {
    pub id: ResourceId,
}

impl ImageHandle {
    pub fn app(key: impl Into<String>) -> Self {
        Self {
            id: ResourceId::app(key),
        }
    }

    pub fn from_id(id: ResourceId) -> Self {
        Self { id }
    }

    pub const fn kind(&self) -> ResourceKind {
        ResourceKind::Image
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IconHandle {
    pub id: ResourceId,
}

impl IconHandle {
    pub fn app(key: impl Into<String>) -> Self {
        Self {
            id: ResourceId::app(key),
        }
    }

    pub fn built_in(key: impl Into<String>) -> Self {
        Self {
            id: ResourceId::built_in(key),
        }
    }

    pub fn from_id(id: ResourceId) -> Self {
        Self { id }
    }

    pub const fn kind(&self) -> ResourceKind {
        ResourceKind::Icon
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureHandle {
    pub id: ResourceId,
}

impl TextureHandle {
    pub fn app(key: impl Into<String>) -> Self {
        Self {
            id: ResourceId::app(key),
        }
    }

    pub fn host(key: impl Into<String>) -> Self {
        Self {
            id: ResourceId::host(key),
        }
    }

    pub fn from_id(id: ResourceId) -> Self {
        Self { id }
    }

    pub const fn kind(&self) -> ResourceKind {
        ResourceKind::Texture
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThumbnailHandle {
    pub id: ResourceId,
}

impl ThumbnailHandle {
    pub fn app(key: impl Into<String>) -> Self {
        Self {
            id: ResourceId::app(key),
        }
    }

    pub fn from_id(id: ResourceId) -> Self {
        Self { id }
    }

    pub const fn kind(&self) -> ResourceKind {
        ResourceKind::Thumbnail
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceHandle {
    Image(ImageHandle),
    Icon(IconHandle),
    Texture(TextureHandle),
    Thumbnail(ThumbnailHandle),
}

impl ResourceHandle {
    pub const fn kind(&self) -> ResourceKind {
        match self {
            Self::Image(handle) => handle.kind(),
            Self::Icon(handle) => handle.kind(),
            Self::Texture(handle) => handle.kind(),
            Self::Thumbnail(handle) => handle.kind(),
        }
    }

    pub fn id(&self) -> &ResourceId {
        match self {
            Self::Image(handle) => &handle.id,
            Self::Icon(handle) => &handle.id,
            Self::Texture(handle) => &handle.id,
            Self::Thumbnail(handle) => &handle.id,
        }
    }
}

pub const LAYER_LOCAL_Z_MIN: i16 = -999;
pub const LAYER_LOCAL_Z_MAX: i16 = 999;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UiLayer {
    HostBackground,
    AppBackground,
    AppContent,
    AppOverlay,
    HostOverlay,
    DebugOverlay,
    SystemOverlay,
}

impl UiLayer {
    pub const fn base_z(self) -> i32 {
        match self {
            Self::HostBackground => -30_000,
            Self::AppBackground => -20_000,
            Self::AppContent => 0,
            Self::AppOverlay => 10_000,
            Self::HostOverlay => 20_000,
            Self::DebugOverlay => 30_000,
            Self::SystemOverlay => 40_000,
        }
    }

    pub const fn is_app(self) -> bool {
        matches!(
            self,
            Self::AppBackground | Self::AppContent | Self::AppOverlay
        )
    }

    pub const fn is_host(self) -> bool {
        matches!(
            self,
            Self::HostBackground | Self::HostOverlay | Self::SystemOverlay
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerOrder {
    pub layer: UiLayer,
    pub local_z: i16,
}

impl LayerOrder {
    pub const DEFAULT: Self = Self::new(UiLayer::AppContent, 0);

    pub const fn new(layer: UiLayer, local_z: i16) -> Self {
        Self {
            layer,
            local_z: clamp_local_z(local_z),
        }
    }

    pub const fn resolved_z(self) -> i32 {
        self.layer.base_z() + self.local_z as i32
    }
}

impl Default for LayerOrder {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Ord for LayerOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        self.resolved_z().cmp(&other.resolved_z())
    }
}

impl PartialOrd for LayerOrder {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub const fn clamp_local_z(local_z: i16) -> i16 {
    if local_z < LAYER_LOCAL_Z_MIN {
        LAYER_LOCAL_Z_MIN
    } else if local_z > LAYER_LOCAL_Z_MAX {
        LAYER_LOCAL_Z_MAX
    } else {
        local_z
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlatformRequestId(pub u64);

impl PlatformRequestId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformServiceKind {
    Clipboard,
    FileDialog,
    OpenUrl,
    Notification,
    Screenshot,
    TextIme,
    DragDrop,
    Cursor,
    Repaint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformErrorCode {
    Unsupported,
    Denied,
    Cancelled,
    InvalidRequest,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformServiceError {
    pub code: PlatformErrorCode,
    pub message: String,
}

impl PlatformServiceError {
    pub fn new(code: PlatformErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(PlatformErrorCode::Unsupported, message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardRequest {
    ReadText,
    WriteText(String),
    ReadFiles,
    WriteFiles(Vec<PathBuf>),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardResponse {
    Text(Option<String>),
    Files(Vec<PathBuf>),
    Completed,
    Unsupported,
    Error(PlatformServiceError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDialogMode {
    OpenFile,
    OpenFiles,
    SaveFile,
    PickFolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

impl FileDialogFilter {
    pub fn new(
        name: impl Into<String>,
        extensions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDialogRequest {
    pub mode: FileDialogMode,
    pub title: Option<String>,
    pub starting_directory: Option<PathBuf>,
    pub default_file_name: Option<String>,
    pub filters: Vec<FileDialogFilter>,
}

impl FileDialogRequest {
    pub fn new(mode: FileDialogMode) -> Self {
        Self {
            mode,
            title: None,
            starting_directory: None,
            default_file_name: None,
            filters: Vec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn starting_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.starting_directory = Some(path.into());
        self
    }

    pub fn default_file_name(mut self, name: impl Into<String>) -> Self {
        self.default_file_name = Some(name.into());
        self
    }

    pub fn filter(mut self, filter: FileDialogFilter) -> Self {
        self.filters.push(filter);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileDialogResponse {
    Selected(Vec<PathBuf>),
    Cancelled,
    Unsupported,
    Error(PlatformServiceError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenUrlRequest {
    pub url: String,
    pub new_window: bool,
}

impl OpenUrlRequest {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            new_window: false,
        }
    }

    pub const fn new_window(mut self, new_window: bool) -> Self {
        self.new_window = new_window;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenUrlResponse {
    Opened,
    Blocked,
    Unsupported,
    Error(PlatformServiceError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl Default for NotificationLevel {
    fn default() -> Self {
        Self::Info
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRequest {
    pub id: Option<String>,
    pub title: String,
    pub body: String,
    pub level: NotificationLevel,
    pub timeout: Option<Duration>,
}

impl NotificationRequest {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: None,
            title: title.into(),
            body: body.into(),
            level: NotificationLevel::Info,
            timeout: None,
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub const fn level(mut self, level: NotificationLevel) -> Self {
        self.level = level;
        self
    }

    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationResponse {
    Queued,
    Delivered,
    Clicked(Option<String>),
    Dismissed(Option<String>),
    Unsupported,
    Error(PlatformServiceError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotFormat {
    Rgba8,
    Bgra8,
    Png,
}

impl Default for ScreenshotFormat {
    fn default() -> Self {
        Self::Rgba8
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScreenshotTarget {
    Window,
    Viewport,
    Rect(LogicalRect),
    Layer(UiLayer),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenshotRequest {
    pub target: ScreenshotTarget,
    pub format: ScreenshotFormat,
    pub scale_factor: Option<f32>,
    pub include_debug_layers: bool,
}

impl ScreenshotRequest {
    pub const fn new(target: ScreenshotTarget) -> Self {
        Self {
            target,
            format: ScreenshotFormat::Rgba8,
            scale_factor: None,
            include_debug_layers: true,
        }
    }

    pub const fn format(mut self, format: ScreenshotFormat) -> Self {
        self.format = format;
        self
    }

    pub const fn scale_factor(mut self, scale_factor: f32) -> Self {
        self.scale_factor = Some(scale_factor);
        self
    }

    pub const fn include_debug_layers(mut self, include_debug_layers: bool) -> Self {
        self.include_debug_layers = include_debug_layers;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenshotImage {
    pub size: PixelSize,
    pub scale_factor: f32,
    pub format: ScreenshotFormat,
    pub bytes: Vec<u8>,
}

impl ScreenshotImage {
    pub fn new(
        size: PixelSize,
        scale_factor: f32,
        format: ScreenshotFormat,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            size,
            scale_factor,
            format,
            bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScreenshotResponse {
    Captured(ScreenshotImage),
    Unsupported,
    Error(PlatformServiceError),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextInputId(pub String);

impl TextInputId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub const EMPTY: Self = Self::new(0, 0);

    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn caret(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    pub const fn is_caret(self) -> bool {
        self.start == self.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextImeSession {
    pub input: TextInputId,
    pub cursor_rect: LogicalRect,
    pub surrounding_text: String,
    pub selection: TextRange,
    pub multiline: bool,
}

impl TextImeSession {
    pub fn new(input: TextInputId, cursor_rect: LogicalRect) -> Self {
        Self {
            input,
            cursor_rect,
            surrounding_text: String::new(),
            selection: TextRange::EMPTY,
            multiline: false,
        }
    }

    pub fn surrounding_text(mut self, text: impl Into<String>, selection: TextRange) -> Self {
        self.surrounding_text = text.into();
        self.selection = selection;
        self
    }

    pub const fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextImeRequest {
    Activate(TextImeSession),
    Update(TextImeSession),
    Deactivate { input: TextInputId },
    ShowKeyboard { input: TextInputId },
    HideKeyboard { input: TextInputId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextImeResponse {
    Activated {
        input: TextInputId,
    },
    Deactivated {
        input: TextInputId,
    },
    Commit {
        input: TextInputId,
        text: String,
    },
    Preedit {
        input: TextInputId,
        text: String,
        selection: Option<TextRange>,
    },
    DeleteSurrounding {
        input: TextInputId,
        before_chars: usize,
        after_chars: usize,
    },
    Unsupported,
    Error(PlatformServiceError),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DragId(pub String);

impl DragId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragOperation {
    Copy,
    Move,
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragBytes {
    pub mime_type: String,
    pub name: Option<String>,
    pub bytes: Vec<u8>,
}

impl DragBytes {
    pub fn new(mime_type: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            mime_type: mime_type.into(),
            name: None,
            bytes,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DragPayload {
    pub text: Option<String>,
    pub files: Vec<PathBuf>,
    pub bytes: Vec<DragBytes>,
}

impl DragPayload {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    pub fn files(files: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        Self {
            files: files.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }

    pub fn bytes(bytes: DragBytes) -> Self {
        Self {
            bytes: vec![bytes],
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DragDropRequest {
    Start {
        id: DragId,
        payload: DragPayload,
        origin: LogicalPoint,
        allowed_operations: Vec<DragOperation>,
    },
    SetAcceptedOperation {
        id: DragId,
        operation: Option<DragOperation>,
    },
    Cancel {
        id: DragId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DragDropResponse {
    Started {
        id: DragId,
    },
    Entered {
        id: DragId,
        position: LogicalPoint,
        payload: DragPayload,
    },
    Moved {
        id: DragId,
        position: LogicalPoint,
    },
    Dropped {
        id: DragId,
        position: LogicalPoint,
        payload: DragPayload,
        operation: DragOperation,
    },
    Left {
        id: DragId,
    },
    Cancelled {
        id: DragId,
    },
    Unsupported,
    Error(PlatformServiceError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorShape {
    Default,
    Pointer,
    Text,
    Crosshair,
    Grab,
    Grabbing,
    Move,
    NotAllowed,
    Wait,
    Progress,
    ResizeHorizontal,
    ResizeVertical,
    ResizeNorthEastSouthWest,
    ResizeNorthWestSouthEast,
    ZoomIn,
    ZoomOut,
}

impl Default for CursorShape {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CursorRequest {
    SetShape(CursorShape),
    SetVisible(bool),
    SetPosition(LogicalPoint),
    Confine(LogicalRect),
    ReleaseConfine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorResponse {
    Applied,
    Unsupported,
    Error(PlatformServiceError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepaintRequest {
    NextFrame,
    After(Duration),
    Area(LogicalRect),
    Continuous { active: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepaintResponse {
    Scheduled { delay: Duration },
    Repainted,
    Coalesced,
    Unsupported,
    Error(PlatformServiceError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlatformRequest {
    Clipboard(ClipboardRequest),
    FileDialog(FileDialogRequest),
    OpenUrl(OpenUrlRequest),
    Notification(NotificationRequest),
    Screenshot(ScreenshotRequest),
    TextIme(TextImeRequest),
    DragDrop(DragDropRequest),
    Cursor(CursorRequest),
    Repaint(RepaintRequest),
}

impl PlatformRequest {
    pub const fn kind(&self) -> PlatformServiceKind {
        match self {
            Self::Clipboard(_) => PlatformServiceKind::Clipboard,
            Self::FileDialog(_) => PlatformServiceKind::FileDialog,
            Self::OpenUrl(_) => PlatformServiceKind::OpenUrl,
            Self::Notification(_) => PlatformServiceKind::Notification,
            Self::Screenshot(_) => PlatformServiceKind::Screenshot,
            Self::TextIme(_) => PlatformServiceKind::TextIme,
            Self::DragDrop(_) => PlatformServiceKind::DragDrop,
            Self::Cursor(_) => PlatformServiceKind::Cursor,
            Self::Repaint(_) => PlatformServiceKind::Repaint,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlatformResponse {
    Clipboard(ClipboardResponse),
    FileDialog(FileDialogResponse),
    OpenUrl(OpenUrlResponse),
    Notification(NotificationResponse),
    Screenshot(ScreenshotResponse),
    TextIme(TextImeResponse),
    DragDrop(DragDropResponse),
    Cursor(CursorResponse),
    Repaint(RepaintResponse),
}

impl PlatformResponse {
    pub const fn kind(&self) -> PlatformServiceKind {
        match self {
            Self::Clipboard(_) => PlatformServiceKind::Clipboard,
            Self::FileDialog(_) => PlatformServiceKind::FileDialog,
            Self::OpenUrl(_) => PlatformServiceKind::OpenUrl,
            Self::Notification(_) => PlatformServiceKind::Notification,
            Self::Screenshot(_) => PlatformServiceKind::Screenshot,
            Self::TextIme(_) => PlatformServiceKind::TextIme,
            Self::DragDrop(_) => PlatformServiceKind::DragDrop,
            Self::Cursor(_) => PlatformServiceKind::Cursor,
            Self::Repaint(_) => PlatformServiceKind::Repaint,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlatformServiceRequest {
    pub id: PlatformRequestId,
    pub request: PlatformRequest,
}

impl PlatformServiceRequest {
    pub const fn new(id: PlatformRequestId, request: PlatformRequest) -> Self {
        Self { id, request }
    }

    pub const fn kind(&self) -> PlatformServiceKind {
        self.request.kind()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlatformServiceResponse {
    pub id: PlatformRequestId,
    pub response: PlatformResponse,
}

impl PlatformServiceResponse {
    pub const fn new(id: PlatformRequestId, response: PlatformResponse) -> Self {
        Self { id, response }
    }

    pub const fn kind(&self) -> PlatformServiceKind {
        self.response.kind()
    }

    pub const fn is_for(&self, request: &PlatformServiceRequest) -> bool {
        self.id.0 == request.id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendAdapterKind {
    Egui,
    Wgpu,
    CpuSnapshot,
    AppOwned,
    Test,
    Other,
}

impl Default for BackendAdapterKind {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceCapabilities {
    pub images: bool,
    pub icons: bool,
    pub textures: bool,
    pub thumbnails: bool,
    pub tinted_icons: bool,
    pub partial_texture_updates: bool,
}

impl ResourceCapabilities {
    pub const NONE: Self = Self {
        images: false,
        icons: false,
        textures: false,
        thumbnails: false,
        tinted_icons: false,
        partial_texture_updates: false,
    };

    pub const ALL: Self = Self {
        images: true,
        icons: true,
        textures: true,
        thumbnails: true,
        tinted_icons: true,
        partial_texture_updates: true,
    };

    pub const fn supports(self, kind: ResourceKind) -> bool {
        match kind {
            ResourceKind::Image => self.images,
            ResourceKind::Icon => self.icons,
            ResourceKind::Texture => self.textures,
            ResourceKind::Thumbnail => self.thumbnails,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerCapabilities {
    pub honors_layer_order: bool,
    pub supports_host_layers: bool,
    pub supports_debug_layers: bool,
    pub max_local_z: i16,
}

impl LayerCapabilities {
    pub const NONE: Self = Self {
        honors_layer_order: false,
        supports_host_layers: false,
        supports_debug_layers: false,
        max_local_z: 0,
    };

    pub const STANDARD: Self = Self {
        honors_layer_order: true,
        supports_host_layers: true,
        supports_debug_layers: true,
        max_local_z: LAYER_LOCAL_Z_MAX,
    };

    pub const fn supports(self, layer: UiLayer) -> bool {
        if layer.is_app() {
            return self.honors_layer_order;
        }
        if matches!(layer, UiLayer::DebugOverlay) {
            return self.supports_debug_layers;
        }
        self.supports_host_layers
    }
}

impl Default for LayerCapabilities {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlatformServiceCapabilities {
    pub clipboard_read: bool,
    pub clipboard_write: bool,
    pub file_open_dialog: bool,
    pub file_save_dialog: bool,
    pub folder_pick_dialog: bool,
    pub open_url: bool,
    pub notifications: bool,
    pub screenshots: bool,
    pub text_ime: bool,
    pub drag_drop: bool,
    pub cursor_shape: bool,
    pub cursor_position: bool,
    pub cursor_confine: bool,
    pub repaint: bool,
}

impl PlatformServiceCapabilities {
    pub const NONE: Self = Self {
        clipboard_read: false,
        clipboard_write: false,
        file_open_dialog: false,
        file_save_dialog: false,
        folder_pick_dialog: false,
        open_url: false,
        notifications: false,
        screenshots: false,
        text_ime: false,
        drag_drop: false,
        cursor_shape: false,
        cursor_position: false,
        cursor_confine: false,
        repaint: false,
    };

    pub const DESKTOP: Self = Self {
        clipboard_read: true,
        clipboard_write: true,
        file_open_dialog: true,
        file_save_dialog: true,
        folder_pick_dialog: true,
        open_url: true,
        notifications: true,
        screenshots: true,
        text_ime: true,
        drag_drop: true,
        cursor_shape: true,
        cursor_position: true,
        cursor_confine: true,
        repaint: true,
    };

    pub const fn supports(self, request: &PlatformRequest) -> bool {
        match request {
            PlatformRequest::Clipboard(request) => match request {
                ClipboardRequest::ReadText | ClipboardRequest::ReadFiles => self.clipboard_read,
                ClipboardRequest::WriteText(_)
                | ClipboardRequest::WriteFiles(_)
                | ClipboardRequest::Clear => self.clipboard_write,
            },
            PlatformRequest::FileDialog(request) => match request.mode {
                FileDialogMode::OpenFile | FileDialogMode::OpenFiles => self.file_open_dialog,
                FileDialogMode::SaveFile => self.file_save_dialog,
                FileDialogMode::PickFolder => self.folder_pick_dialog,
            },
            PlatformRequest::OpenUrl(_) => self.open_url,
            PlatformRequest::Notification(_) => self.notifications,
            PlatformRequest::Screenshot(_) => self.screenshots,
            PlatformRequest::TextIme(_) => self.text_ime,
            PlatformRequest::DragDrop(_) => self.drag_drop,
            PlatformRequest::Cursor(request) => match request {
                CursorRequest::SetShape(_) | CursorRequest::SetVisible(_) => self.cursor_shape,
                CursorRequest::SetPosition(_) => self.cursor_position,
                CursorRequest::Confine(_) | CursorRequest::ReleaseConfine => self.cursor_confine,
            },
            PlatformRequest::Repaint(_) => self.repaint,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderingCapabilities {
    pub high_dpi: bool,
    pub offscreen: bool,
    pub deterministic_snapshots: bool,
    pub partial_updates: bool,
}

impl RenderingCapabilities {
    pub const NONE: Self = Self {
        high_dpi: false,
        offscreen: false,
        deterministic_snapshots: false,
        partial_updates: false,
    };

    pub const STANDARD: Self = Self {
        high_dpi: true,
        offscreen: false,
        deterministic_snapshots: false,
        partial_updates: true,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendCapabilities {
    pub name: String,
    pub adapter: BackendAdapterKind,
    pub resources: ResourceCapabilities,
    pub layers: LayerCapabilities,
    pub services: PlatformServiceCapabilities,
    pub rendering: RenderingCapabilities,
    pub accessibility: AccessibilityCapabilities,
}

impl BackendCapabilities {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            adapter: BackendAdapterKind::Other,
            resources: ResourceCapabilities::NONE,
            layers: LayerCapabilities::NONE,
            services: PlatformServiceCapabilities::NONE,
            rendering: RenderingCapabilities::NONE,
            accessibility: AccessibilityCapabilities::NONE,
        }
    }

    pub const fn adapter(mut self, adapter: BackendAdapterKind) -> Self {
        self.adapter = adapter;
        self
    }

    pub const fn resources(mut self, resources: ResourceCapabilities) -> Self {
        self.resources = resources;
        self
    }

    pub const fn layers(mut self, layers: LayerCapabilities) -> Self {
        self.layers = layers;
        self
    }

    pub const fn services(mut self, services: PlatformServiceCapabilities) -> Self {
        self.services = services;
        self
    }

    pub const fn rendering(mut self, rendering: RenderingCapabilities) -> Self {
        self.rendering = rendering;
        self
    }

    pub const fn accessibility(mut self, accessibility: AccessibilityCapabilities) -> Self {
        self.accessibility = accessibility;
        self
    }

    pub const fn supports_resource(&self, kind: ResourceKind) -> bool {
        self.resources.supports(kind)
    }

    pub const fn supports_layer(&self, layer: UiLayer) -> bool {
        self.layers.supports(layer)
    }

    pub const fn supports_request(&self, request: &PlatformRequest) -> bool {
        self.services.supports(request)
    }

    pub const fn supports_accessibility(&self, request: AccessibilityRequestKind) -> bool {
        self.accessibility.supports(request)
    }
}

impl Default for BackendCapabilities {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_handles_keep_kind_and_domain() {
        let image = ResourceHandle::Image(ImageHandle::app("cover-art"));
        let icon = ResourceHandle::Icon(IconHandle::built_in("play"));
        let texture = ResourceHandle::Texture(TextureHandle::host("swapchain"));

        assert_eq!(image.kind(), ResourceKind::Image);
        assert_eq!(image.id().domain, ResourceDomain::App);
        assert_eq!(icon.kind(), ResourceKind::Icon);
        assert_eq!(icon.id().domain, ResourceDomain::BuiltIn);
        assert_eq!(texture.kind(), ResourceKind::Texture);
        assert_eq!(texture.id().domain, ResourceDomain::Host);
    }

    #[test]
    fn layer_order_keeps_debug_above_app_ui() {
        let app_high = LayerOrder::new(UiLayer::AppOverlay, LAYER_LOCAL_Z_MAX);
        let debug_low = LayerOrder::new(UiLayer::DebugOverlay, LAYER_LOCAL_Z_MIN);
        let host_background = LayerOrder::new(UiLayer::HostBackground, 0);

        assert!(debug_low > app_high);
        assert!(host_background < LayerOrder::new(UiLayer::AppBackground, 0));
        assert_eq!(
            LayerOrder::new(UiLayer::AppContent, i16::MAX).local_z,
            LAYER_LOCAL_Z_MAX
        );
    }

    #[test]
    fn platform_service_request_and_response_share_correlation_id() {
        let request = PlatformServiceRequest::new(
            PlatformRequestId::new(42),
            PlatformRequest::Clipboard(ClipboardRequest::ReadText),
        );
        let response = PlatformServiceResponse::new(
            PlatformRequestId::new(42),
            PlatformResponse::Clipboard(ClipboardResponse::Text(Some("copied".to_string()))),
        );

        assert_eq!(request.kind(), PlatformServiceKind::Clipboard);
        assert_eq!(response.kind(), PlatformServiceKind::Clipboard);
        assert!(response.is_for(&request));
    }

    #[test]
    fn capability_descriptor_matches_specific_service_requests() {
        let backend = BackendCapabilities::new("unit-test")
            .adapter(BackendAdapterKind::Test)
            .resources(ResourceCapabilities {
                images: true,
                icons: true,
                ..ResourceCapabilities::NONE
            })
            .layers(LayerCapabilities::STANDARD)
            .services(PlatformServiceCapabilities {
                clipboard_read: true,
                file_save_dialog: true,
                repaint: true,
                ..PlatformServiceCapabilities::NONE
            })
            .accessibility(AccessibilityCapabilities::SCREEN_READER);

        let read_clipboard = PlatformRequest::Clipboard(ClipboardRequest::ReadText);
        let write_clipboard = PlatformRequest::Clipboard(ClipboardRequest::WriteText("x".into()));
        let save_file =
            PlatformRequest::FileDialog(FileDialogRequest::new(FileDialogMode::SaveFile));
        let open_file =
            PlatformRequest::FileDialog(FileDialogRequest::new(FileDialogMode::OpenFile));

        assert!(backend.supports_resource(ResourceKind::Image));
        assert!(!backend.supports_resource(ResourceKind::Texture));
        assert!(backend.supports_layer(UiLayer::DebugOverlay));
        assert!(backend.supports_request(&read_clipboard));
        assert!(!backend.supports_request(&write_clipboard));
        assert!(backend.supports_request(&save_file));
        assert!(!backend.supports_request(&open_file));
        assert!(backend.supports_accessibility(AccessibilityRequestKind::PublishTree));
        assert!(!backend.accessibility.screenshots);
    }

    #[test]
    fn text_ime_and_drag_drop_payloads_are_plain_data() {
        let input = TextInputId::new("search");
        let ime = TextImeSession::new(input.clone(), LogicalRect::new(4.0, 8.0, 1.0, 18.0))
            .surrounding_text("abc", TextRange::caret(2));
        let payload = DragPayload::text("clip");

        assert_eq!(ime.input, input);
        assert!(ime.selection.is_caret());
        assert_eq!(payload.text.as_deref(), Some("clip"));
        assert!(LogicalRect::new(0.0, 0.0, 8.0, 8.0).contains(LogicalPoint::new(4.0, 4.0)));
    }
}
