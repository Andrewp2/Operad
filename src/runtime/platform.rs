//! Backend-neutral platform contracts for Operad adapters.
//!
//! This module is intentionally data-only. Backends translate these contracts
//! to egui, wgpu, host operating-system APIs, test adapters, or
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

impl From<ImageHandle> for ResourceHandle {
    fn from(handle: ImageHandle) -> Self {
        Self::Image(handle)
    }
}

impl From<IconHandle> for ResourceHandle {
    fn from(handle: IconHandle) -> Self {
        Self::Icon(handle)
    }
}

impl From<TextureHandle> for ResourceHandle {
    fn from(handle: TextureHandle) -> Self {
        Self::Texture(handle)
    }
}

impl From<ThumbnailHandle> for ResourceHandle {
    fn from(handle: ThumbnailHandle) -> Self {
        Self::Thumbnail(handle)
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
pub struct PlatformRequestId(pub(crate) u64);

impl PlatformRequestId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformRequestIdAllocator {
    next: u64,
}

impl PlatformRequestIdAllocator {
    pub const fn new(first: u64) -> Self {
        Self { next: first }
    }

    pub const fn next_value(self) -> u64 {
        self.next
    }

    pub fn next_id(&mut self) -> PlatformRequestId {
        let id = PlatformRequestId::new(self.next);
        self.next = self.next.wrapping_add(1);
        id
    }

    pub fn allocate(&mut self, request: PlatformRequest) -> PlatformServiceRequest {
        PlatformServiceRequest::new(self.next_id(), request)
    }

    pub fn allocate_all(
        &mut self,
        requests: impl IntoIterator<Item = PlatformRequest>,
    ) -> Vec<PlatformServiceRequest> {
        requests
            .into_iter()
            .map(|request| self.allocate(request))
            .collect()
    }
}

impl Default for PlatformRequestIdAllocator {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformServiceKind {
    Clipboard,
    FileDialog,
    OpenUrl,
    Notification,
    Screenshot,
    AppLifecycle,
    TextIme,
    DragDrop,
    Cursor,
    Repaint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformServiceCapabilityKind {
    ClipboardRead,
    ClipboardWrite,
    FileOpenDialog,
    FileSaveDialog,
    FolderPickDialog,
    OpenUrl,
    Notification,
    Screenshot,
    AppQuit,
    WindowClose,
    TextIme,
    DragDrop,
    CursorShape,
    CursorVisibility,
    CursorPosition,
    CursorGrab,
    CursorConfine,
    Repaint,
}

impl PlatformServiceCapabilityKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ClipboardRead => "clipboard read",
            Self::ClipboardWrite => "clipboard write",
            Self::FileOpenDialog => "file open dialog",
            Self::FileSaveDialog => "file save dialog",
            Self::FolderPickDialog => "folder pick dialog",
            Self::OpenUrl => "open URL",
            Self::Notification => "notification",
            Self::Screenshot => "screenshot",
            Self::AppQuit => "app quit",
            Self::WindowClose => "window close",
            Self::TextIme => "text IME",
            Self::DragDrop => "drag and drop",
            Self::CursorShape => "cursor shape",
            Self::CursorVisibility => "cursor visibility",
            Self::CursorPosition => "cursor position",
            Self::CursorGrab => "cursor grab",
            Self::CursorConfine => "cursor confine",
            Self::Repaint => "repaint scheduling",
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppLifecycleRequest {
    Quit,
    CloseWindow { window_id: Option<String> },
}

impl AppLifecycleRequest {
    pub fn close_window(window_id: impl Into<String>) -> Self {
        Self::CloseWindow {
            window_id: Some(window_id.into()),
        }
    }

    pub const fn close_active_window() -> Self {
        Self::CloseWindow { window_id: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppLifecycleResponse {
    Applied,
    Cancelled,
    Unsupported,
    Error(PlatformServiceError),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextInputId(pub(crate) String);

impl TextInputId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
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

impl TextImeResponse {
    pub fn input(&self) -> Option<&TextInputId> {
        match self {
            Self::Activated { input }
            | Self::Deactivated { input }
            | Self::Commit { input, .. }
            | Self::Preedit { input, .. }
            | Self::DeleteSurrounding { input, .. } => Some(input),
            Self::Unsupported | Self::Error(_) => None,
        }
    }

    pub fn is_for_input(&self, input: &TextInputId) -> bool {
        match self.input() {
            Some(actual) => actual == input,
            None => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DragId(pub(crate) String);

impl DragId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DragImage {
    pub image_key: Option<String>,
    pub label: Option<String>,
    pub size: LogicalSize,
    pub hotspot: LogicalPoint,
}

impl DragImage {
    pub const fn new(size: LogicalSize) -> Self {
        Self {
            image_key: None,
            label: None,
            size,
            hotspot: LogicalPoint::ZERO,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn image_key(mut self, image_key: impl Into<String>) -> Self {
        self.image_key = Some(image_key.into());
        self
    }

    pub const fn hotspot(mut self, hotspot: LogicalPoint) -> Self {
        self.hotspot = hotspot;
        self
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
        drag_image: Option<DragImage>,
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
    SetGrab(CursorGrabMode),
    Confine(LogicalRect),
    ReleaseConfine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorGrabMode {
    None,
    Confined,
    Locked,
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
    AppLifecycle(AppLifecycleRequest),
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
            Self::AppLifecycle(_) => PlatformServiceKind::AppLifecycle,
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
    AppLifecycle(AppLifecycleResponse),
    TextIme(TextImeResponse),
    DragDrop(DragDropResponse),
    Cursor(CursorResponse),
    Repaint(RepaintResponse),
}

impl PlatformResponse {
    pub const fn unsupported(kind: PlatformServiceKind) -> Self {
        match kind {
            PlatformServiceKind::Clipboard => Self::Clipboard(ClipboardResponse::Unsupported),
            PlatformServiceKind::FileDialog => Self::FileDialog(FileDialogResponse::Unsupported),
            PlatformServiceKind::OpenUrl => Self::OpenUrl(OpenUrlResponse::Unsupported),
            PlatformServiceKind::Notification => {
                Self::Notification(NotificationResponse::Unsupported)
            }
            PlatformServiceKind::Screenshot => Self::Screenshot(ScreenshotResponse::Unsupported),
            PlatformServiceKind::AppLifecycle => {
                Self::AppLifecycle(AppLifecycleResponse::Unsupported)
            }
            PlatformServiceKind::TextIme => Self::TextIme(TextImeResponse::Unsupported),
            PlatformServiceKind::DragDrop => Self::DragDrop(DragDropResponse::Unsupported),
            PlatformServiceKind::Cursor => Self::Cursor(CursorResponse::Unsupported),
            PlatformServiceKind::Repaint => Self::Repaint(RepaintResponse::Unsupported),
        }
    }

    pub const fn kind(&self) -> PlatformServiceKind {
        match self {
            Self::Clipboard(_) => PlatformServiceKind::Clipboard,
            Self::FileDialog(_) => PlatformServiceKind::FileDialog,
            Self::OpenUrl(_) => PlatformServiceKind::OpenUrl,
            Self::Notification(_) => PlatformServiceKind::Notification,
            Self::Screenshot(_) => PlatformServiceKind::Screenshot,
            Self::AppLifecycle(_) => PlatformServiceKind::AppLifecycle,
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

    pub fn unsupported_response(&self) -> PlatformServiceResponse {
        PlatformServiceResponse::new(self.id, PlatformResponse::unsupported(self.kind()))
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
        self.id.value() == request.id.value()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendAdapterKind {
    Egui,
    Wgpu,
    AppOwned,
    Test,
    Other,
}

impl Default for BackendAdapterKind {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputCapabilityKind {
    PointerMove,
    PointerButton,
    PointerWheel,
    WheelPhase,
    HighResolutionWheel,
    KeyboardPress,
    KeyboardRelease,
    TextInput,
    TextIme,
    Modifiers,
    RawMouseMotion,
    PointerLock,
    Gamepad,
    CanvasLocalInput,
}

impl InputCapabilityKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::PointerMove => "pointer move",
            Self::PointerButton => "pointer button",
            Self::PointerWheel => "pointer wheel",
            Self::WheelPhase => "wheel phase",
            Self::HighResolutionWheel => "high resolution wheel",
            Self::KeyboardPress => "keyboard press",
            Self::KeyboardRelease => "keyboard release",
            Self::TextInput => "text input",
            Self::TextIme => "text IME input",
            Self::Modifiers => "keyboard modifiers",
            Self::RawMouseMotion => "raw mouse motion",
            Self::PointerLock => "pointer lock",
            Self::Gamepad => "gamepad input",
            Self::CanvasLocalInput => "canvas-local input",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputCapabilities {
    pub pointer_move: bool,
    pub pointer_button: bool,
    pub pointer_wheel: bool,
    pub wheel_phase: bool,
    pub high_resolution_wheel: bool,
    pub keyboard_press: bool,
    pub keyboard_release: bool,
    pub text_input: bool,
    pub text_ime: bool,
    pub modifiers: bool,
    pub raw_mouse_motion: bool,
    pub pointer_lock: bool,
    pub gamepad: bool,
    pub canvas_local_input: bool,
}

impl InputCapabilities {
    pub const NONE: Self = Self {
        pointer_move: false,
        pointer_button: false,
        pointer_wheel: false,
        wheel_phase: false,
        high_resolution_wheel: false,
        keyboard_press: false,
        keyboard_release: false,
        text_input: false,
        text_ime: false,
        modifiers: false,
        raw_mouse_motion: false,
        pointer_lock: false,
        gamepad: false,
        canvas_local_input: false,
    };

    pub const STANDARD: Self = Self {
        pointer_move: true,
        pointer_button: true,
        pointer_wheel: true,
        wheel_phase: false,
        high_resolution_wheel: false,
        keyboard_press: true,
        keyboard_release: true,
        text_input: true,
        text_ime: false,
        modifiers: true,
        raw_mouse_motion: false,
        pointer_lock: false,
        gamepad: false,
        canvas_local_input: true,
    };

    pub const DESKTOP: Self = Self {
        pointer_move: true,
        pointer_button: true,
        pointer_wheel: true,
        wheel_phase: true,
        high_resolution_wheel: true,
        keyboard_press: true,
        keyboard_release: true,
        text_input: true,
        text_ime: true,
        modifiers: true,
        raw_mouse_motion: true,
        pointer_lock: true,
        gamepad: false,
        canvas_local_input: true,
    };

    pub const fn supports(self, kind: InputCapabilityKind) -> bool {
        match kind {
            InputCapabilityKind::PointerMove => self.pointer_move,
            InputCapabilityKind::PointerButton => self.pointer_button,
            InputCapabilityKind::PointerWheel => self.pointer_wheel,
            InputCapabilityKind::WheelPhase => self.wheel_phase,
            InputCapabilityKind::HighResolutionWheel => self.high_resolution_wheel,
            InputCapabilityKind::KeyboardPress => self.keyboard_press,
            InputCapabilityKind::KeyboardRelease => self.keyboard_release,
            InputCapabilityKind::TextInput => self.text_input,
            InputCapabilityKind::TextIme => self.text_ime,
            InputCapabilityKind::Modifiers => self.modifiers,
            InputCapabilityKind::RawMouseMotion => self.raw_mouse_motion,
            InputCapabilityKind::PointerLock => self.pointer_lock,
            InputCapabilityKind::Gamepad => self.gamepad,
            InputCapabilityKind::CanvasLocalInput => self.canvas_local_input,
        }
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
    pub app_quit: bool,
    pub window_close: bool,
    pub text_ime: bool,
    pub drag_drop: bool,
    pub cursor_shape: bool,
    pub cursor_visible: bool,
    pub cursor_position: bool,
    pub cursor_grab: bool,
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
        app_quit: false,
        window_close: false,
        text_ime: false,
        drag_drop: false,
        cursor_shape: false,
        cursor_visible: false,
        cursor_position: false,
        cursor_grab: false,
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
        app_quit: true,
        window_close: true,
        text_ime: true,
        drag_drop: true,
        cursor_shape: true,
        cursor_visible: true,
        cursor_position: true,
        cursor_grab: true,
        cursor_confine: true,
        repaint: true,
    };

    pub const fn supports_kind(self, kind: PlatformServiceCapabilityKind) -> bool {
        match kind {
            PlatformServiceCapabilityKind::ClipboardRead => self.clipboard_read,
            PlatformServiceCapabilityKind::ClipboardWrite => self.clipboard_write,
            PlatformServiceCapabilityKind::FileOpenDialog => self.file_open_dialog,
            PlatformServiceCapabilityKind::FileSaveDialog => self.file_save_dialog,
            PlatformServiceCapabilityKind::FolderPickDialog => self.folder_pick_dialog,
            PlatformServiceCapabilityKind::OpenUrl => self.open_url,
            PlatformServiceCapabilityKind::Notification => self.notifications,
            PlatformServiceCapabilityKind::Screenshot => self.screenshots,
            PlatformServiceCapabilityKind::AppQuit => self.app_quit,
            PlatformServiceCapabilityKind::WindowClose => self.window_close,
            PlatformServiceCapabilityKind::TextIme => self.text_ime,
            PlatformServiceCapabilityKind::DragDrop => self.drag_drop,
            PlatformServiceCapabilityKind::CursorShape => self.cursor_shape,
            PlatformServiceCapabilityKind::CursorVisibility => self.cursor_visible,
            PlatformServiceCapabilityKind::CursorPosition => self.cursor_position,
            PlatformServiceCapabilityKind::CursorGrab => self.cursor_grab,
            PlatformServiceCapabilityKind::CursorConfine => self.cursor_confine,
            PlatformServiceCapabilityKind::Repaint => self.repaint,
        }
    }

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
            PlatformRequest::AppLifecycle(request) => match request {
                AppLifecycleRequest::Quit => self.app_quit,
                AppLifecycleRequest::CloseWindow { .. } => self.window_close,
            },
            PlatformRequest::TextIme(_) => self.text_ime,
            PlatformRequest::DragDrop(_) => self.drag_drop,
            PlatformRequest::Cursor(request) => match request {
                CursorRequest::SetShape(_) => self.cursor_shape,
                CursorRequest::SetVisible(_) => self.cursor_visible,
                CursorRequest::SetPosition(_) => self.cursor_position,
                CursorRequest::SetGrab(_) => self.cursor_grab,
                CursorRequest::Confine(_) | CursorRequest::ReleaseConfine => self.cursor_confine,
            },
            PlatformRequest::Repaint(_) => self.repaint,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderingCapabilityKind {
    HighDpi,
    Offscreen,
    DeterministicSnapshots,
    PartialUpdates,
    WebGpuSurface,
    AppOwnedViewRendering,
    NativeChildWindows,
    PlatformOverlays,
}

impl RenderingCapabilityKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::HighDpi => "high-DPI rendering",
            Self::Offscreen => "offscreen rendering",
            Self::DeterministicSnapshots => "deterministic snapshots",
            Self::PartialUpdates => "partial render updates",
            Self::WebGpuSurface => "WebGPU surface rendering",
            Self::AppOwnedViewRendering => "app-owned texture-view rendering",
            Self::NativeChildWindows => "native child windows",
            Self::PlatformOverlays => "platform overlays",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderingCapabilities {
    pub high_dpi: bool,
    pub offscreen: bool,
    pub deterministic_snapshots: bool,
    pub partial_updates: bool,
    pub webgpu_surface: bool,
    pub app_owned_view_rendering: bool,
    pub native_child_windows: bool,
    pub platform_overlays: bool,
}

impl RenderingCapabilities {
    pub const NONE: Self = Self {
        high_dpi: false,
        offscreen: false,
        deterministic_snapshots: false,
        partial_updates: false,
        webgpu_surface: false,
        app_owned_view_rendering: false,
        native_child_windows: false,
        platform_overlays: false,
    };

    pub const STANDARD: Self = Self {
        high_dpi: true,
        offscreen: false,
        deterministic_snapshots: false,
        partial_updates: true,
        webgpu_surface: false,
        app_owned_view_rendering: false,
        native_child_windows: false,
        platform_overlays: false,
    };

    pub const fn supports(self, kind: RenderingCapabilityKind) -> bool {
        match kind {
            RenderingCapabilityKind::HighDpi => self.high_dpi,
            RenderingCapabilityKind::Offscreen => self.offscreen,
            RenderingCapabilityKind::DeterministicSnapshots => self.deterministic_snapshots,
            RenderingCapabilityKind::PartialUpdates => self.partial_updates,
            RenderingCapabilityKind::WebGpuSurface => self.webgpu_surface,
            RenderingCapabilityKind::AppOwnedViewRendering => self.app_owned_view_rendering,
            RenderingCapabilityKind::NativeChildWindows => self.native_child_windows,
            RenderingCapabilityKind::PlatformOverlays => self.platform_overlays,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityFallback {
    UseFallback,
    DisableFeature,
    EmitDiagnostic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityDecision {
    UseFeature,
    UseFallback,
    DisableFeature,
    EmitDiagnostic,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackendCapabilityRequirement {
    Input(InputCapabilityKind),
    Resource(ResourceKind),
    Layer(UiLayer),
    PlatformService(PlatformServiceCapabilityKind),
    PlatformRequest(PlatformRequest),
    Rendering(RenderingCapabilityKind),
    Accessibility(AccessibilityRequestKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendCapabilityProfile {
    BasicUi,
    TextEditing,
    CommandHotkeys,
    CanvasPointerEditing,
    Flycam3d,
    DockWorkspace,
    PlatformDragDrop,
    AccessibleApp,
}

impl BackendCapabilityProfile {
    pub const fn label(self) -> &'static str {
        match self {
            Self::BasicUi => "basic UI",
            Self::TextEditing => "text editing",
            Self::CommandHotkeys => "command hotkeys",
            Self::CanvasPointerEditing => "canvas pointer editing",
            Self::Flycam3d => "3D flycam",
            Self::DockWorkspace => "docked workspace",
            Self::PlatformDragDrop => "platform drag and drop",
            Self::AccessibleApp => "accessible app",
        }
    }

    pub fn requirements(self) -> Vec<BackendCapabilityRequirement> {
        match self {
            Self::BasicUi => vec![
                BackendCapabilityRequirement::Input(InputCapabilityKind::PointerMove),
                BackendCapabilityRequirement::Input(InputCapabilityKind::PointerButton),
                BackendCapabilityRequirement::Input(InputCapabilityKind::PointerWheel),
                BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardPress),
                BackendCapabilityRequirement::Input(InputCapabilityKind::Modifiers),
                BackendCapabilityRequirement::Layer(UiLayer::AppContent),
                BackendCapabilityRequirement::Layer(UiLayer::AppOverlay),
                BackendCapabilityRequirement::PlatformService(
                    PlatformServiceCapabilityKind::Repaint,
                ),
            ],
            Self::TextEditing => vec![
                BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardPress),
                BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardRelease),
                BackendCapabilityRequirement::Input(InputCapabilityKind::TextInput),
                BackendCapabilityRequirement::PlatformService(
                    PlatformServiceCapabilityKind::ClipboardRead,
                ),
                BackendCapabilityRequirement::PlatformService(
                    PlatformServiceCapabilityKind::ClipboardWrite,
                ),
            ],
            Self::CommandHotkeys => vec![
                BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardPress),
                BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardRelease),
                BackendCapabilityRequirement::Input(InputCapabilityKind::Modifiers),
            ],
            Self::CanvasPointerEditing => vec![
                BackendCapabilityRequirement::Input(InputCapabilityKind::PointerMove),
                BackendCapabilityRequirement::Input(InputCapabilityKind::PointerButton),
                BackendCapabilityRequirement::Input(InputCapabilityKind::PointerWheel),
                BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardPress),
                BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardRelease),
                BackendCapabilityRequirement::Input(InputCapabilityKind::CanvasLocalInput),
                BackendCapabilityRequirement::Rendering(RenderingCapabilityKind::WebGpuSurface),
            ],
            Self::Flycam3d => vec![
                BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardPress),
                BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardRelease),
                BackendCapabilityRequirement::Input(InputCapabilityKind::RawMouseMotion),
                BackendCapabilityRequirement::Input(InputCapabilityKind::PointerLock),
                BackendCapabilityRequirement::Input(InputCapabilityKind::CanvasLocalInput),
                BackendCapabilityRequirement::PlatformService(
                    PlatformServiceCapabilityKind::CursorGrab,
                ),
                BackendCapabilityRequirement::PlatformService(
                    PlatformServiceCapabilityKind::CursorVisibility,
                ),
                BackendCapabilityRequirement::Rendering(RenderingCapabilityKind::WebGpuSurface),
            ],
            Self::DockWorkspace => vec![
                BackendCapabilityRequirement::Input(InputCapabilityKind::PointerMove),
                BackendCapabilityRequirement::Input(InputCapabilityKind::PointerButton),
                BackendCapabilityRequirement::Input(InputCapabilityKind::PointerWheel),
                BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardPress),
                BackendCapabilityRequirement::Layer(UiLayer::AppOverlay),
            ],
            Self::PlatformDragDrop => vec![BackendCapabilityRequirement::PlatformService(
                PlatformServiceCapabilityKind::DragDrop,
            )],
            Self::AccessibleApp => vec![
                BackendCapabilityRequirement::Accessibility(AccessibilityRequestKind::PublishTree),
                BackendCapabilityRequirement::Accessibility(AccessibilityRequestKind::MoveFocus),
                BackendCapabilityRequirement::Accessibility(AccessibilityRequestKind::SetFocusTrap),
                BackendCapabilityRequirement::Accessibility(AccessibilityRequestKind::Announce),
                BackendCapabilityRequirement::Accessibility(
                    AccessibilityRequestKind::ApplyPreferences,
                ),
            ],
        }
    }
}

impl BackendCapabilityRequirement {
    pub fn supported_by(&self, backend: &BackendCapabilities) -> bool {
        match self {
            Self::Input(kind) => backend.supports_input(*kind),
            Self::Resource(kind) => backend.supports_resource(*kind),
            Self::Layer(layer) => backend.supports_layer(*layer),
            Self::PlatformService(kind) => backend.supports_platform_service(*kind),
            Self::PlatformRequest(request) => backend.supports_request(request),
            Self::Rendering(kind) => backend.supports_rendering(*kind),
            Self::Accessibility(kind) => backend.supports_accessibility(*kind),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Input(kind) => format!("input:{}", kind.label()),
            Self::Resource(kind) => format!("resource:{kind:?}"),
            Self::Layer(layer) => format!("layer:{layer:?}"),
            Self::PlatformService(kind) => format!("platform:{}", kind.label()),
            Self::PlatformRequest(request) => platform_request_requirement_label(request),
            Self::Rendering(kind) => format!("rendering:{}", kind.label()),
            Self::Accessibility(kind) => format!("accessibility:{kind:?}"),
        }
    }

    pub fn remediation(&self) -> String {
        match self {
            Self::Input(kind) => format!(
                "Use a host that publishes {}, or provide a widget fallback that does not depend on that input stream.",
                kind.label()
            ),
            Self::Resource(kind) => format!(
                "Use a renderer/resource resolver that supports {kind:?} resources, or gate the feature behind a fallback asset."
            ),
            Self::Layer(layer) => format!(
                "Use a backend that supports {layer:?} layering, or move the content to a supported Operad layer."
            ),
            Self::PlatformService(kind) => format!(
                "Check {} before enabling the control, and choose a fallback, disabled state, or explicit diagnostic.",
                kind.label()
            ),
            Self::PlatformRequest(request) => format!(
                "Check support for {} before issuing the platform request, and surface an unsupported-request diagnostic if there is no fallback.",
                platform_request_requirement_label(request)
            ),
            Self::Rendering(kind) => format!(
                "Use a renderer that supports {}, or route this surface through a lower capability rendering path.",
                kind.label()
            ),
            Self::Accessibility(kind) => format!(
                "Use an accessibility adapter that supports {kind:?}, or expose an equivalent in-app fallback."
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackendCapabilityDiagnostic {
    pub backend: String,
    pub requirement: BackendCapabilityRequirement,
    pub supported: bool,
    pub decision: CapabilityDecision,
    pub summary: String,
    pub remediation: String,
}

impl BackendCapabilityDiagnostic {
    pub fn new(
        backend: &BackendCapabilities,
        requirement: BackendCapabilityRequirement,
        fallback: CapabilityFallback,
    ) -> Self {
        let supported = requirement.supported_by(backend);
        let decision = if supported {
            CapabilityDecision::UseFeature
        } else {
            match fallback {
                CapabilityFallback::UseFallback => CapabilityDecision::UseFallback,
                CapabilityFallback::DisableFeature => CapabilityDecision::DisableFeature,
                CapabilityFallback::EmitDiagnostic => CapabilityDecision::EmitDiagnostic,
            }
        };
        let label = requirement.label();
        let summary = if supported {
            format!("backend {:?} supports {label}", backend.name)
        } else {
            format!(
                "backend {:?} does not support {label}; decision: {decision:?}",
                backend.name
            )
        };
        let remediation = if supported {
            "No fallback is required.".to_string()
        } else {
            requirement.remediation()
        };

        Self {
            backend: backend.name.clone(),
            requirement,
            supported,
            decision,
            summary,
            remediation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendCapabilities {
    pub name: String,
    pub adapter: BackendAdapterKind,
    pub input: InputCapabilities,
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
            input: InputCapabilities::NONE,
            resources: ResourceCapabilities::NONE,
            layers: LayerCapabilities::NONE,
            services: PlatformServiceCapabilities::NONE,
            rendering: RenderingCapabilities::NONE,
            accessibility: AccessibilityCapabilities::NONE,
        }
    }

    pub fn native_window() -> Self {
        Self::new("native-window")
            .adapter(BackendAdapterKind::Wgpu)
            .input(InputCapabilities::DESKTOP)
            .resources(ResourceCapabilities {
                images: true,
                icons: true,
                textures: true,
                thumbnails: true,
                tinted_icons: true,
                partial_texture_updates: true,
            })
            .layers(LayerCapabilities::STANDARD)
            .services(PlatformServiceCapabilities {
                drag_drop: false,
                ..PlatformServiceCapabilities::DESKTOP
            })
            .rendering(RenderingCapabilities {
                high_dpi: true,
                offscreen: false,
                deterministic_snapshots: false,
                partial_updates: true,
                webgpu_surface: true,
                app_owned_view_rendering: true,
                native_child_windows: false,
                platform_overlays: false,
            })
            .accessibility(AccessibilityCapabilities::NONE)
    }

    pub fn web_runtime() -> Self {
        Self::new("web-runtime")
            .adapter(BackendAdapterKind::Wgpu)
            .input(InputCapabilities {
                pointer_move: true,
                pointer_button: true,
                pointer_wheel: true,
                wheel_phase: false,
                high_resolution_wheel: true,
                keyboard_press: true,
                keyboard_release: true,
                text_input: true,
                text_ime: false,
                modifiers: true,
                raw_mouse_motion: false,
                pointer_lock: false,
                gamepad: false,
                canvas_local_input: true,
            })
            .resources(ResourceCapabilities {
                images: true,
                icons: true,
                textures: true,
                thumbnails: true,
                tinted_icons: true,
                partial_texture_updates: true,
            })
            .layers(LayerCapabilities::STANDARD)
            .services(PlatformServiceCapabilities {
                clipboard_read: true,
                clipboard_write: true,
                open_url: true,
                cursor_shape: true,
                cursor_visible: true,
                repaint: true,
                ..PlatformServiceCapabilities::NONE
            })
            .rendering(RenderingCapabilities {
                high_dpi: true,
                offscreen: false,
                deterministic_snapshots: false,
                partial_updates: true,
                webgpu_surface: true,
                app_owned_view_rendering: true,
                native_child_windows: false,
                platform_overlays: false,
            })
            .accessibility(AccessibilityCapabilities::NONE)
    }

    pub fn test_host() -> Self {
        Self::new("operad-test-host")
            .adapter(BackendAdapterKind::Test)
            .input(InputCapabilities::STANDARD)
            .resources(ResourceCapabilities::ALL)
            .layers(LayerCapabilities::STANDARD)
            .services(PlatformServiceCapabilities {
                clipboard_read: true,
                clipboard_write: true,
                open_url: true,
                drag_drop: true,
                cursor_shape: true,
                cursor_visible: true,
                repaint: true,
                ..PlatformServiceCapabilities::NONE
            })
            .rendering(RenderingCapabilities {
                high_dpi: false,
                offscreen: true,
                deterministic_snapshots: true,
                partial_updates: true,
                webgpu_surface: false,
                app_owned_view_rendering: false,
                native_child_windows: false,
                platform_overlays: false,
            })
            .accessibility(AccessibilityCapabilities::SCREEN_READER)
    }

    pub const fn adapter(mut self, adapter: BackendAdapterKind) -> Self {
        self.adapter = adapter;
        self
    }

    pub const fn input(mut self, input: InputCapabilities) -> Self {
        self.input = input;
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

    pub const fn supports_input(&self, kind: InputCapabilityKind) -> bool {
        self.input.supports(kind)
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

    pub const fn supports_platform_service(&self, kind: PlatformServiceCapabilityKind) -> bool {
        self.services.supports_kind(kind)
    }

    pub const fn supports_rendering(&self, kind: RenderingCapabilityKind) -> bool {
        self.rendering.supports(kind)
    }

    pub const fn supports_accessibility(&self, request: AccessibilityRequestKind) -> bool {
        self.accessibility.supports(request)
    }

    pub fn supports_profile(&self, profile: BackendCapabilityProfile) -> bool {
        profile
            .requirements()
            .iter()
            .all(|requirement| requirement.supported_by(self))
    }

    pub fn diagnose_requirement(
        &self,
        requirement: BackendCapabilityRequirement,
        fallback: CapabilityFallback,
    ) -> BackendCapabilityDiagnostic {
        BackendCapabilityDiagnostic::new(self, requirement, fallback)
    }

    pub fn diagnose_requirements(
        &self,
        requirements: impl IntoIterator<Item = BackendCapabilityRequirement>,
        fallback: CapabilityFallback,
    ) -> Vec<BackendCapabilityDiagnostic> {
        requirements
            .into_iter()
            .map(|requirement| self.diagnose_requirement(requirement, fallback))
            .collect()
    }

    pub fn diagnose_profile(
        &self,
        profile: BackendCapabilityProfile,
        fallback: CapabilityFallback,
    ) -> Vec<BackendCapabilityDiagnostic> {
        self.diagnose_requirements(profile.requirements(), fallback)
    }

    pub fn diagnose_profiles(
        &self,
        profiles: impl IntoIterator<Item = BackendCapabilityProfile>,
        fallback: CapabilityFallback,
    ) -> Vec<BackendCapabilityDiagnostic> {
        profiles
            .into_iter()
            .flat_map(|profile| self.diagnose_profile(profile, fallback))
            .collect()
    }
}

impl Default for BackendCapabilities {
    fn default() -> Self {
        Self::new("")
    }
}

fn platform_request_requirement_label(request: &PlatformRequest) -> String {
    match request {
        PlatformRequest::Clipboard(request) => match request {
            ClipboardRequest::ReadText => "platform:clipboard read text".to_string(),
            ClipboardRequest::WriteText(_) => "platform:clipboard write text".to_string(),
            ClipboardRequest::ReadFiles => "platform:clipboard read files".to_string(),
            ClipboardRequest::WriteFiles(_) => "platform:clipboard write files".to_string(),
            ClipboardRequest::Clear => "platform:clipboard clear".to_string(),
        },
        PlatformRequest::FileDialog(request) => format!("platform:file dialog {:?}", request.mode),
        PlatformRequest::OpenUrl(_) => "platform:open URL".to_string(),
        PlatformRequest::Notification(_) => "platform:notification".to_string(),
        PlatformRequest::Screenshot(_) => "platform:screenshot".to_string(),
        PlatformRequest::AppLifecycle(request) => match request {
            AppLifecycleRequest::Quit => "platform:app quit".to_string(),
            AppLifecycleRequest::CloseWindow { .. } => "platform:window close".to_string(),
        },
        PlatformRequest::TextIme(_) => "platform:text IME".to_string(),
        PlatformRequest::DragDrop(_) => "platform:drag and drop".to_string(),
        PlatformRequest::Cursor(request) => match request {
            CursorRequest::SetShape(_) => "platform:cursor shape".to_string(),
            CursorRequest::SetVisible(_) => "platform:cursor visibility".to_string(),
            CursorRequest::SetPosition(_) => "platform:cursor position".to_string(),
            CursorRequest::SetGrab(mode) => format!("platform:cursor grab {mode:?}"),
            CursorRequest::Confine(_) => "platform:cursor confine".to_string(),
            CursorRequest::ReleaseConfine => "platform:cursor release confine".to_string(),
        },
        PlatformRequest::Repaint(_) => "platform:repaint scheduling".to_string(),
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
    fn platform_service_request_builds_correlated_unsupported_response() {
        let request = PlatformServiceRequest::new(
            PlatformRequestId::new(44),
            PlatformRequest::FileDialog(FileDialogRequest::new(FileDialogMode::OpenFile)),
        );

        let response = request.unsupported_response();

        assert_eq!(response.id, PlatformRequestId::new(44));
        assert_eq!(response.kind(), PlatformServiceKind::FileDialog);
        assert!(response.is_for(&request));
        assert_eq!(
            response.response,
            PlatformResponse::FileDialog(FileDialogResponse::Unsupported)
        );
        assert_eq!(
            PlatformResponse::unsupported(PlatformServiceKind::Cursor),
            PlatformResponse::Cursor(CursorResponse::Unsupported)
        );
    }

    #[test]
    fn platform_request_id_allocator_batches_service_requests_deterministically() {
        let mut allocator = PlatformRequestIdAllocator::new(10);
        let requests = allocator.allocate_all([
            PlatformRequest::Clipboard(ClipboardRequest::ReadText),
            PlatformRequest::OpenUrl(OpenUrlRequest::new("https://example.test")),
        ]);

        assert_eq!(
            requests
                .iter()
                .map(|request| request.id)
                .collect::<Vec<_>>(),
            vec![PlatformRequestId::new(10), PlatformRequestId::new(11)]
        );
        assert_eq!(
            requests
                .iter()
                .map(PlatformServiceRequest::kind)
                .collect::<Vec<_>>(),
            vec![PlatformServiceKind::Clipboard, PlatformServiceKind::OpenUrl]
        );
        assert_eq!(allocator.next_value(), 12);

        let mut wrapping_allocator = PlatformRequestIdAllocator::new(u64::MAX);
        assert_eq!(
            wrapping_allocator.next_id(),
            PlatformRequestId::new(u64::MAX)
        );
        assert_eq!(wrapping_allocator.next_id(), PlatformRequestId::new(0));
    }

    #[test]
    fn capability_descriptor_matches_specific_service_requests() {
        let backend = BackendCapabilities::new("unit-test")
            .adapter(BackendAdapterKind::Test)
            .input(InputCapabilities::STANDARD)
            .resources(ResourceCapabilities {
                images: true,
                icons: true,
                ..ResourceCapabilities::NONE
            })
            .layers(LayerCapabilities::STANDARD)
            .services(PlatformServiceCapabilities {
                clipboard_read: true,
                file_save_dialog: true,
                app_quit: true,
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
        let quit = PlatformRequest::AppLifecycle(AppLifecycleRequest::Quit);
        let close_window =
            PlatformRequest::AppLifecycle(AppLifecycleRequest::close_active_window());

        assert!(backend.supports_resource(ResourceKind::Image));
        assert!(backend.supports_input(InputCapabilityKind::KeyboardRelease));
        assert!(!backend.supports_input(InputCapabilityKind::RawMouseMotion));
        assert!(!backend.supports_resource(ResourceKind::Texture));
        assert!(backend.supports_layer(UiLayer::DebugOverlay));
        assert!(backend.supports_request(&read_clipboard));
        assert!(!backend.supports_request(&write_clipboard));
        assert!(backend.supports_request(&save_file));
        assert!(!backend.supports_request(&open_file));
        assert!(backend.supports_request(&quit));
        assert!(!backend.supports_request(&close_window));
        assert!(backend.supports_accessibility(AccessibilityRequestKind::PublishTree));
        assert!(!backend.accessibility.screenshots);
    }

    #[test]
    fn input_capabilities_distinguish_basic_input_from_flycam_input() {
        assert!(InputCapabilities::STANDARD.supports(InputCapabilityKind::KeyboardPress));
        assert!(InputCapabilities::STANDARD.supports(InputCapabilityKind::KeyboardRelease));
        assert!(InputCapabilities::STANDARD.supports(InputCapabilityKind::CanvasLocalInput));
        assert!(!InputCapabilities::STANDARD.supports(InputCapabilityKind::RawMouseMotion));
        assert!(!InputCapabilities::STANDARD.supports(InputCapabilityKind::PointerLock));

        assert!(InputCapabilities::DESKTOP.supports(InputCapabilityKind::RawMouseMotion));
        assert!(InputCapabilities::DESKTOP.supports(InputCapabilityKind::PointerLock));
        assert!(InputCapabilities::DESKTOP.supports(InputCapabilityKind::WheelPhase));
        assert!(!InputCapabilities::NONE.supports(InputCapabilityKind::KeyboardPress));
    }

    #[test]
    fn platform_service_capabilities_distinguish_cursor_subfeatures() {
        let capabilities = PlatformServiceCapabilities {
            cursor_shape: true,
            cursor_grab: true,
            ..PlatformServiceCapabilities::NONE
        };

        assert!(capabilities.supports_kind(PlatformServiceCapabilityKind::CursorShape));
        assert!(capabilities.supports_kind(PlatformServiceCapabilityKind::CursorGrab));
        assert!(!capabilities.supports_kind(PlatformServiceCapabilityKind::CursorVisibility));
        assert!(!capabilities.supports_kind(PlatformServiceCapabilityKind::CursorConfine));
        assert!(
            capabilities.supports(&PlatformRequest::Cursor(CursorRequest::SetShape(
                CursorShape::Pointer
            )))
        );
        assert!(
            capabilities.supports(&PlatformRequest::Cursor(CursorRequest::SetGrab(
                CursorGrabMode::Locked
            )))
        );
        assert!(!capabilities.supports(&PlatformRequest::Cursor(CursorRequest::SetVisible(false))));
        assert!(
            !capabilities.supports(&PlatformRequest::Cursor(CursorRequest::Confine(
                LogicalRect::new(0.0, 0.0, 20.0, 20.0)
            )))
        );
    }

    #[test]
    fn backend_capability_diagnostics_pick_declared_outcome() {
        let backend = BackendCapabilities::new("flycam-test")
            .adapter(BackendAdapterKind::Test)
            .input(InputCapabilities::STANDARD)
            .services(PlatformServiceCapabilities {
                cursor_visible: true,
                ..PlatformServiceCapabilities::NONE
            })
            .rendering(RenderingCapabilities {
                deterministic_snapshots: true,
                ..RenderingCapabilities::NONE
            });

        let supported = backend.diagnose_requirement(
            BackendCapabilityRequirement::Input(InputCapabilityKind::KeyboardRelease),
            CapabilityFallback::DisableFeature,
        );
        assert!(supported.supported);
        assert_eq!(supported.decision, CapabilityDecision::UseFeature);

        let unsupported = backend.diagnose_requirement(
            BackendCapabilityRequirement::Input(InputCapabilityKind::RawMouseMotion),
            CapabilityFallback::EmitDiagnostic,
        );
        assert!(!unsupported.supported);
        assert_eq!(unsupported.decision, CapabilityDecision::EmitDiagnostic);
        assert!(unsupported.summary.contains("raw mouse motion"));
        assert!(unsupported
            .remediation
            .contains("does not depend on that input stream"));

        let disabled = backend.diagnose_requirement(
            BackendCapabilityRequirement::PlatformRequest(PlatformRequest::Cursor(
                CursorRequest::SetGrab(CursorGrabMode::Locked),
            )),
            CapabilityFallback::DisableFeature,
        );
        assert!(!disabled.supported);
        assert_eq!(disabled.decision, CapabilityDecision::DisableFeature);
        assert!(disabled.summary.contains("cursor grab Locked"));

        let batch = backend.diagnose_requirements(
            [
                BackendCapabilityRequirement::Rendering(
                    RenderingCapabilityKind::DeterministicSnapshots,
                ),
                BackendCapabilityRequirement::PlatformService(
                    PlatformServiceCapabilityKind::CursorVisibility,
                ),
            ],
            CapabilityFallback::UseFallback,
        );
        assert_eq!(
            batch
                .iter()
                .map(|diagnostic| diagnostic.decision)
                .collect::<Vec<_>>(),
            vec![
                CapabilityDecision::UseFeature,
                CapabilityDecision::UseFeature
            ]
        );
    }

    #[test]
    fn capability_profiles_explain_product_level_interaction_needs() {
        let desktop = BackendCapabilities::new("desktop-test")
            .adapter(BackendAdapterKind::Test)
            .input(InputCapabilities::DESKTOP)
            .services(PlatformServiceCapabilities::DESKTOP)
            .layers(LayerCapabilities::STANDARD)
            .rendering(RenderingCapabilities {
                webgpu_surface: true,
                ..RenderingCapabilities::STANDARD
            })
            .accessibility(AccessibilityCapabilities::SCREEN_READER);

        assert!(desktop.supports_profile(BackendCapabilityProfile::CommandHotkeys));
        assert!(desktop.supports_profile(BackendCapabilityProfile::CanvasPointerEditing));
        assert!(desktop.supports_profile(BackendCapabilityProfile::Flycam3d));
        assert!(desktop.supports_profile(BackendCapabilityProfile::AccessibleApp));

        let browser_like = BackendCapabilities::new("browser-like")
            .adapter(BackendAdapterKind::Wgpu)
            .input(InputCapabilities {
                high_resolution_wheel: true,
                ..InputCapabilities::STANDARD
            })
            .services(PlatformServiceCapabilities {
                cursor_visible: true,
                repaint: true,
                ..PlatformServiceCapabilities::NONE
            })
            .layers(LayerCapabilities::STANDARD)
            .rendering(RenderingCapabilities {
                webgpu_surface: true,
                ..RenderingCapabilities::STANDARD
            });

        assert!(browser_like.supports_profile(BackendCapabilityProfile::CommandHotkeys));
        assert!(!browser_like.supports_profile(BackendCapabilityProfile::Flycam3d));

        let diagnostics = browser_like.diagnose_profile(
            BackendCapabilityProfile::Flycam3d,
            CapabilityFallback::EmitDiagnostic,
        );
        let missing_labels = diagnostics
            .iter()
            .filter(|diagnostic| !diagnostic.supported)
            .map(|diagnostic| diagnostic.requirement.label())
            .collect::<Vec<_>>();

        assert!(missing_labels.contains(&"input:raw mouse motion".to_string()));
        assert!(missing_labels.contains(&"input:pointer lock".to_string()));
        assert!(missing_labels.contains(&"platform:cursor grab".to_string()));
        assert!(diagnostics
            .iter()
            .filter(|diagnostic| !diagnostic.supported)
            .all(
                |diagnostic| diagnostic.decision == CapabilityDecision::EmitDiagnostic
                    && diagnostic.remediation.contains("fallback")
            ));
    }

    #[test]
    fn built_in_host_capability_matrix_covers_native_web_and_test_hosts() {
        let native = BackendCapabilities::native_window();
        assert_eq!(native.name, "native-window");
        assert_eq!(native.adapter, BackendAdapterKind::Wgpu);
        assert!(native.supports_profile(BackendCapabilityProfile::BasicUi));
        assert!(native.supports_profile(BackendCapabilityProfile::TextEditing));
        assert!(native.supports_profile(BackendCapabilityProfile::CommandHotkeys));
        assert!(native.supports_profile(BackendCapabilityProfile::CanvasPointerEditing));
        assert!(native.supports_profile(BackendCapabilityProfile::Flycam3d));
        assert!(native.supports_profile(BackendCapabilityProfile::DockWorkspace));
        assert!(!native.supports_profile(BackendCapabilityProfile::PlatformDragDrop));
        assert!(!native.supports_profile(BackendCapabilityProfile::AccessibleApp));

        let web = BackendCapabilities::web_runtime();
        assert_eq!(web.name, "web-runtime");
        assert_eq!(web.adapter, BackendAdapterKind::Wgpu);
        assert!(web.supports_input(InputCapabilityKind::KeyboardRelease));
        assert!(web.supports_profile(BackendCapabilityProfile::BasicUi));
        assert!(web.supports_profile(BackendCapabilityProfile::TextEditing));
        assert!(web.supports_profile(BackendCapabilityProfile::CommandHotkeys));
        assert!(web.supports_profile(BackendCapabilityProfile::CanvasPointerEditing));
        assert!(!web.supports_profile(BackendCapabilityProfile::Flycam3d));
        assert!(web.supports_profile(BackendCapabilityProfile::DockWorkspace));
        assert!(!web.supports_profile(BackendCapabilityProfile::PlatformDragDrop));

        let web_flycam_missing = unsupported_requirement_labels(
            &web,
            BackendCapabilityProfile::Flycam3d,
            CapabilityFallback::EmitDiagnostic,
        );
        assert_eq!(
            web_flycam_missing,
            vec![
                "input:raw mouse motion".to_string(),
                "input:pointer lock".to_string(),
                "platform:cursor grab".to_string(),
            ]
        );

        let web_platform_drag_drop_missing = unsupported_requirement_labels(
            &web,
            BackendCapabilityProfile::PlatformDragDrop,
            CapabilityFallback::UseFallback,
        );
        assert_eq!(
            web_platform_drag_drop_missing,
            vec!["platform:drag and drop".to_string()]
        );

        let test = BackendCapabilities::test_host();
        assert_eq!(test.name, "operad-test-host");
        assert_eq!(test.adapter, BackendAdapterKind::Test);
        assert!(test.rendering.deterministic_snapshots);
        assert!(test.supports_profile(BackendCapabilityProfile::BasicUi));
        assert!(test.supports_profile(BackendCapabilityProfile::TextEditing));
        assert!(test.supports_profile(BackendCapabilityProfile::CommandHotkeys));
        assert!(test.supports_profile(BackendCapabilityProfile::DockWorkspace));
        assert!(test.supports_profile(BackendCapabilityProfile::PlatformDragDrop));
        assert!(test.supports_profile(BackendCapabilityProfile::AccessibleApp));
        assert!(!test.supports_profile(BackendCapabilityProfile::CanvasPointerEditing));
        assert!(!test.supports_profile(BackendCapabilityProfile::Flycam3d));
    }

    fn unsupported_requirement_labels(
        backend: &BackendCapabilities,
        profile: BackendCapabilityProfile,
        fallback: CapabilityFallback,
    ) -> Vec<String> {
        backend
            .diagnose_profile(profile, fallback)
            .into_iter()
            .filter(|diagnostic| {
                !diagnostic.supported && diagnostic.decision != CapabilityDecision::UseFeature
            })
            .map(|diagnostic| diagnostic.requirement.label())
            .collect()
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

        let other = TextInputId::new("other");
        let commit = TextImeResponse::Commit {
            input: input.clone(),
            text: "x".to_string(),
        };
        assert_eq!(commit.input(), Some(&input));
        assert!(commit.is_for_input(&input));
        assert!(!commit.is_for_input(&other));
        assert!(TextImeResponse::Unsupported.is_for_input(&input));
    }
}
