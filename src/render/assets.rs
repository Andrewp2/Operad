//! Renderer-neutral icon and image asset registry contracts.
//!
//! Operad widgets can refer to stable handles and metadata without embedding
//! backend image types or hand-rolled icon paths. Backends remain responsible
//! for resolving the handles into textures, vector glyphs, or application-owned
//! image resources.

use std::collections::HashMap;

use crate::platform::{IconHandle, ImageHandle, ResourceDomain, ResourceHandle, ResourceId};
use crate::{
    ColorRgba, ImageAlignment, ImageContent, PaintPath, ScenePrimitive, StrokeStyle, UiPoint,
    UiRect, UiSize,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltInIcon {
    Play,
    Pause,
    Stop,
    Record,
    Loop,
    Metronome,
    Rewind,
    FastForward,
    Settings,
    Add,
    Search,
    Folder,
    Collapse,
    Expand,
    Mute,
    Solo,
    Lock,
    Unlock,
    Snap,
    ZoomIn,
    ZoomOut,
    Link,
    Unlink,
    Scissors,
    Pencil,
    Pointer,
    Grid,
    Check,
    Close,
    Warning,
    Info,
}

impl BuiltInIcon {
    pub const COMMON: [Self; 31] = [
        Self::Play,
        Self::Pause,
        Self::Stop,
        Self::Record,
        Self::Loop,
        Self::Metronome,
        Self::Rewind,
        Self::FastForward,
        Self::Settings,
        Self::Add,
        Self::Search,
        Self::Folder,
        Self::Collapse,
        Self::Expand,
        Self::Mute,
        Self::Solo,
        Self::Lock,
        Self::Unlock,
        Self::Snap,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::Link,
        Self::Unlink,
        Self::Scissors,
        Self::Pencil,
        Self::Pointer,
        Self::Grid,
        Self::Check,
        Self::Close,
        Self::Warning,
        Self::Info,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Play => "icons.play",
            Self::Pause => "icons.pause",
            Self::Stop => "icons.stop",
            Self::Record => "icons.record",
            Self::Loop => "icons.loop",
            Self::Metronome => "icons.metronome",
            Self::Rewind => "icons.rewind",
            Self::FastForward => "icons.fast-forward",
            Self::Settings => "icons.settings",
            Self::Add => "icons.add",
            Self::Search => "icons.search",
            Self::Folder => "icons.folder",
            Self::Collapse => "icons.collapse",
            Self::Expand => "icons.expand",
            Self::Mute => "icons.mute",
            Self::Solo => "icons.solo",
            Self::Lock => "icons.lock",
            Self::Unlock => "icons.unlock",
            Self::Snap => "icons.snap",
            Self::ZoomIn => "icons.zoom-in",
            Self::ZoomOut => "icons.zoom-out",
            Self::Link => "icons.link",
            Self::Unlink => "icons.unlink",
            Self::Scissors => "icons.scissors",
            Self::Pencil => "icons.pencil",
            Self::Pointer => "icons.pointer",
            Self::Grid => "icons.grid",
            Self::Check => "icons.check",
            Self::Close => "icons.close",
            Self::Warning => "icons.warning",
            Self::Info => "icons.info",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Pause => "Pause",
            Self::Stop => "Stop",
            Self::Record => "Record",
            Self::Loop => "Loop",
            Self::Metronome => "Metronome",
            Self::Rewind => "Rewind",
            Self::FastForward => "Fast forward",
            Self::Settings => "Settings",
            Self::Add => "Add",
            Self::Search => "Search",
            Self::Folder => "Folder",
            Self::Collapse => "Collapse",
            Self::Expand => "Expand",
            Self::Mute => "Mute",
            Self::Solo => "Solo",
            Self::Lock => "Lock",
            Self::Unlock => "Unlock",
            Self::Snap => "Snap",
            Self::ZoomIn => "Zoom in",
            Self::ZoomOut => "Zoom out",
            Self::Link => "Link",
            Self::Unlink => "Unlink",
            Self::Scissors => "Scissors",
            Self::Pencil => "Pencil",
            Self::Pointer => "Pointer",
            Self::Grid => "Grid",
            Self::Check => "Check",
            Self::Close => "Close",
            Self::Warning => "Warning",
            Self::Info => "Info",
        }
    }

    pub const fn keywords(self) -> &'static [&'static str] {
        match self {
            Self::Play => &["transport", "start"],
            Self::Pause => &["transport"],
            Self::Stop => &["transport"],
            Self::Record => &["transport", "capture"],
            Self::Loop => &["transport", "repeat"],
            Self::Metronome => &["transport", "tempo"],
            Self::Rewind => &["transport"],
            Self::FastForward => &["transport"],
            Self::Settings => &["preferences", "configuration"],
            Self::Add => &["new", "plus"],
            Self::Search => &["find", "filter"],
            Self::Folder => &["browser", "directory"],
            Self::Collapse => &["disclosure", "tree"],
            Self::Expand => &["disclosure", "tree"],
            Self::Mute => &["audio", "track"],
            Self::Solo => &["audio", "track"],
            Self::Lock => &["state"],
            Self::Unlock => &["state"],
            Self::Snap => &["grid", "quantize"],
            Self::ZoomIn => &["view"],
            Self::ZoomOut => &["view"],
            Self::Link => &["connection"],
            Self::Unlink => &["connection"],
            Self::Scissors => &["split", "cut"],
            Self::Pencil => &["draw", "edit"],
            Self::Pointer => &["select", "tool"],
            Self::Grid => &["editor", "snap"],
            Self::Check => &["confirm"],
            Self::Close => &["dismiss", "remove"],
            Self::Warning => &["alert"],
            Self::Info => &["help"],
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "icons.play" => Some(Self::Play),
            "icons.pause" => Some(Self::Pause),
            "icons.stop" => Some(Self::Stop),
            "icons.record" => Some(Self::Record),
            "icons.loop" => Some(Self::Loop),
            "icons.metronome" => Some(Self::Metronome),
            "icons.rewind" => Some(Self::Rewind),
            "icons.fast-forward" => Some(Self::FastForward),
            "icons.settings" => Some(Self::Settings),
            "icons.add" => Some(Self::Add),
            "icons.search" => Some(Self::Search),
            "icons.folder" => Some(Self::Folder),
            "icons.collapse" => Some(Self::Collapse),
            "icons.expand" => Some(Self::Expand),
            "icons.mute" => Some(Self::Mute),
            "icons.solo" => Some(Self::Solo),
            "icons.lock" => Some(Self::Lock),
            "icons.unlock" => Some(Self::Unlock),
            "icons.snap" => Some(Self::Snap),
            "icons.zoom-in" => Some(Self::ZoomIn),
            "icons.zoom-out" => Some(Self::ZoomOut),
            "icons.link" => Some(Self::Link),
            "icons.unlink" => Some(Self::Unlink),
            "icons.scissors" => Some(Self::Scissors),
            "icons.pencil" => Some(Self::Pencil),
            "icons.pointer" => Some(Self::Pointer),
            "icons.grid" => Some(Self::Grid),
            "icons.check" => Some(Self::Check),
            "icons.close" => Some(Self::Close),
            "icons.warning" => Some(Self::Warning),
            "icons.info" => Some(Self::Info),
            _ => None,
        }
    }

    pub fn from_handle(handle: &IconHandle) -> Option<Self> {
        if handle.id.domain == ResourceDomain::BuiltIn {
            Self::from_key(&handle.id.key)
        } else {
            None
        }
    }

    pub fn handle(self) -> IconHandle {
        IconHandle::built_in(self.key())
    }

    pub fn descriptor(self) -> IconDescriptor {
        let mut descriptor = IconDescriptor::new(self.handle(), self.label())
            .default_size(UiSize::new(16.0, 16.0))
            .alignment(ImageAlignment::Center);
        for keyword in self.keywords() {
            descriptor = descriptor.keyword(*keyword);
        }
        descriptor
    }

    pub fn fallback_paths(self, rect: UiRect, tint: ColorRgba) -> Vec<PaintPath> {
        let icon = IconPathBuilder::new(rect, tint);
        match self {
            Self::Play => vec![icon.filled(&[(8.0, 5.0), (18.0, 12.0), (8.0, 19.0)])],
            Self::Pause => vec![
                icon.filled_rect(7.0, 5.0, 3.5, 14.0),
                icon.filled_rect(13.5, 5.0, 3.5, 14.0),
            ],
            Self::Stop => vec![icon.filled_rect(6.0, 6.0, 12.0, 12.0)],
            Self::Record => vec![icon.circle(12.0, 12.0, 7.0, true)],
            Self::Loop => vec![
                icon.polyline(&[(6.0, 10.0), (6.0, 7.0), (17.0, 7.0), (19.0, 10.0)]),
                icon.filled(&[(19.0, 10.0), (16.0, 9.0), (18.0, 13.0)]),
                icon.polyline(&[(18.0, 14.0), (18.0, 17.0), (7.0, 17.0), (5.0, 14.0)]),
                icon.filled(&[(5.0, 14.0), (8.0, 15.0), (6.0, 11.0)]),
            ],
            Self::Metronome => vec![
                icon.filled(&[(8.0, 20.0), (11.0, 4.0), (13.0, 4.0), (16.0, 20.0)]),
                icon.polyline(&[(12.0, 5.0), (15.0, 16.0)]),
                icon.circle(15.0, 16.0, 1.5, true),
            ],
            Self::Rewind => vec![
                icon.filled(&[(6.0, 12.0), (13.0, 6.0), (13.0, 18.0)]),
                icon.filled(&[(12.0, 12.0), (19.0, 6.0), (19.0, 18.0)]),
            ],
            Self::FastForward => vec![
                icon.filled(&[(5.0, 6.0), (12.0, 12.0), (5.0, 18.0)]),
                icon.filled(&[(11.0, 6.0), (18.0, 12.0), (11.0, 18.0)]),
            ],
            Self::Settings => {
                let mut paths = vec![icon.circle(12.0, 12.0, 4.5, false)];
                for (from, to) in [
                    ((12.0, 3.5), (12.0, 6.0)),
                    ((12.0, 18.0), (12.0, 20.5)),
                    ((3.5, 12.0), (6.0, 12.0)),
                    ((18.0, 12.0), (20.5, 12.0)),
                    ((6.0, 6.0), (7.8, 7.8)),
                    ((16.2, 16.2), (18.0, 18.0)),
                    ((18.0, 6.0), (16.2, 7.8)),
                    ((7.8, 16.2), (6.0, 18.0)),
                ] {
                    paths.push(icon.line(from, to));
                }
                paths
            }
            Self::Add => vec![
                icon.line((12.0, 5.0), (12.0, 19.0)),
                icon.line((5.0, 12.0), (19.0, 12.0)),
            ],
            Self::Search => vec![
                icon.circle(10.5, 10.5, 5.5, false),
                icon.line((15.0, 15.0), (20.0, 20.0)),
            ],
            Self::Folder => vec![icon.filled(&[
                (3.0, 8.0),
                (9.0, 8.0),
                (11.0, 10.0),
                (21.0, 10.0),
                (21.0, 19.0),
                (3.0, 19.0),
            ])],
            Self::Collapse => vec![icon.polyline(&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)])],
            Self::Expand => vec![icon.polyline(&[(9.0, 7.0), (14.0, 12.0), (9.0, 17.0)])],
            Self::Mute => vec![
                icon.filled(&[
                    (4.0, 10.0),
                    (8.0, 10.0),
                    (13.0, 6.0),
                    (13.0, 18.0),
                    (8.0, 14.0),
                    (4.0, 14.0),
                ]),
                icon.line((16.0, 9.0), (21.0, 15.0)),
                icon.line((21.0, 9.0), (16.0, 15.0)),
            ],
            Self::Solo => vec![
                icon.circle(12.0, 12.0, 8.0, false),
                icon.polyline(&[
                    (15.0, 8.0),
                    (10.0, 8.0),
                    (9.0, 12.0),
                    (14.0, 12.0),
                    (13.0, 16.0),
                    (8.0, 16.0),
                ]),
            ],
            Self::Lock => vec![
                icon.lock_body(),
                icon.polyline(&[
                    (8.0, 10.0),
                    (8.0, 7.0),
                    (12.0, 4.5),
                    (16.0, 7.0),
                    (16.0, 10.0),
                ]),
            ],
            Self::Unlock => vec![
                icon.lock_body(),
                icon.polyline(&[(8.0, 10.0), (8.0, 7.0), (12.0, 4.5), (16.5, 6.5)]),
            ],
            Self::Snap => vec![
                icon.polyline(&[
                    (6.0, 5.0),
                    (6.0, 14.0),
                    (9.0, 18.0),
                    (15.0, 18.0),
                    (18.0, 14.0),
                    (18.0, 5.0),
                ]),
                icon.line((6.0, 9.0), (10.0, 9.0)),
                icon.line((14.0, 9.0), (18.0, 9.0)),
            ],
            Self::ZoomIn => vec![
                icon.circle(10.5, 10.5, 5.5, false),
                icon.line((10.5, 7.5), (10.5, 13.5)),
                icon.line((7.5, 10.5), (13.5, 10.5)),
                icon.line((15.0, 15.0), (20.0, 20.0)),
            ],
            Self::ZoomOut => vec![
                icon.circle(10.5, 10.5, 5.5, false),
                icon.line((7.5, 10.5), (13.5, 10.5)),
                icon.line((15.0, 15.0), (20.0, 20.0)),
            ],
            Self::Link => vec![
                icon.polyline(&[
                    (9.0, 8.0),
                    (6.0, 8.0),
                    (4.0, 10.0),
                    (4.0, 14.0),
                    (6.0, 16.0),
                    (10.0, 16.0),
                    (13.0, 13.0),
                ]),
                icon.polyline(&[
                    (15.0, 8.0),
                    (18.0, 8.0),
                    (20.0, 10.0),
                    (20.0, 14.0),
                    (18.0, 16.0),
                    (14.0, 16.0),
                    (11.0, 13.0),
                ]),
                icon.line((8.0, 12.0), (16.0, 12.0)),
            ],
            Self::Unlink => vec![
                icon.polyline(&[
                    (9.0, 8.0),
                    (6.0, 8.0),
                    (4.0, 10.0),
                    (4.0, 14.0),
                    (6.0, 16.0),
                    (9.0, 16.0),
                ]),
                icon.polyline(&[
                    (15.0, 8.0),
                    (18.0, 8.0),
                    (20.0, 10.0),
                    (20.0, 14.0),
                    (18.0, 16.0),
                    (15.0, 16.0),
                ]),
                icon.line((8.0, 20.0), (16.0, 4.0)),
            ],
            Self::Scissors => vec![
                icon.circle(7.0, 7.0, 2.5, false),
                icon.circle(7.0, 17.0, 2.5, false),
                icon.line((9.0, 9.0), (19.0, 19.0)),
                icon.line((9.0, 15.0), (19.0, 5.0)),
            ],
            Self::Pencil => vec![
                icon.filled(&[
                    (5.0, 18.0),
                    (7.0, 13.0),
                    (16.0, 4.0),
                    (20.0, 8.0),
                    (11.0, 17.0),
                ]),
                icon.line((14.5, 5.5), (18.5, 9.5)),
            ],
            Self::Pointer => vec![icon.filled(&[
                (6.0, 4.0),
                (18.0, 13.0),
                (12.0, 14.0),
                (15.0, 20.0),
                (12.0, 21.0),
                (9.0, 15.0),
                (6.0, 18.0),
            ])],
            Self::Grid => vec![
                icon.rect_outline(5.0, 5.0, 14.0, 14.0),
                icon.line((12.0, 5.0), (12.0, 19.0)),
                icon.line((5.0, 12.0), (19.0, 12.0)),
            ],
            Self::Check => vec![icon.polyline(&[(5.0, 12.0), (10.0, 17.0), (19.0, 7.0)])],
            Self::Close => vec![
                icon.line((6.0, 6.0), (18.0, 18.0)),
                icon.line((18.0, 6.0), (6.0, 18.0)),
            ],
            Self::Warning => vec![
                icon.filled(&[(12.0, 4.0), (21.0, 20.0), (3.0, 20.0)]),
                icon.warning_cutout_line(),
                icon.warning_cutout_dot(),
            ],
            Self::Info => vec![
                icon.circle(12.0, 12.0, 8.0, false),
                icon.line((12.0, 10.0), (12.0, 17.0)),
                icon.circle(12.0, 7.5, 1.0, true),
            ],
        }
    }

    pub fn fallback_scene(self, rect: UiRect, tint: ColorRgba) -> Vec<ScenePrimitive> {
        self.fallback_paths(rect, tint)
            .into_iter()
            .map(ScenePrimitive::Path)
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
struct IconPathBuilder {
    rect: UiRect,
    tint: ColorRgba,
    stroke_width: f32,
}

impl IconPathBuilder {
    fn new(rect: UiRect, tint: ColorRgba) -> Self {
        let size = rect.width.min(rect.height).max(0.0);
        let rect = UiRect::new(
            rect.x + (rect.width - size) * 0.5,
            rect.y + (rect.height - size) * 0.5,
            size,
            size,
        );
        Self {
            rect,
            tint,
            stroke_width: (size / 12.0).clamp(1.0, 2.5),
        }
    }

    fn point(self, x: f32, y: f32) -> UiPoint {
        UiPoint::new(
            self.rect.x + self.rect.width * (x / 24.0),
            self.rect.y + self.rect.height * (y / 24.0),
        )
    }

    fn stroke(self) -> StrokeStyle {
        StrokeStyle::new(self.tint, self.stroke_width)
    }

    fn line(self, from: (f32, f32), to: (f32, f32)) -> PaintPath {
        PaintPath::new()
            .move_to(self.point(from.0, from.1))
            .line_to(self.point(to.0, to.1))
            .stroke(self.stroke())
    }

    fn polyline(self, points: &[(f32, f32)]) -> PaintPath {
        let mut path = PaintPath::new();
        for (index, point) in points.iter().copied().enumerate() {
            if index == 0 {
                path = path.move_to(self.point(point.0, point.1));
            } else {
                path = path.line_to(self.point(point.0, point.1));
            }
        }
        path.stroke(self.stroke())
    }

    fn filled(self, points: &[(f32, f32)]) -> PaintPath {
        let mut path = PaintPath::new();
        for (index, point) in points.iter().copied().enumerate() {
            if index == 0 {
                path = path.move_to(self.point(point.0, point.1));
            } else {
                path = path.line_to(self.point(point.0, point.1));
            }
        }
        path.close().fill(self.tint)
    }

    fn filled_rect(self, x: f32, y: f32, width: f32, height: f32) -> PaintPath {
        self.filled(&[
            (x, y),
            (x + width, y),
            (x + width, y + height),
            (x, y + height),
        ])
    }

    fn rect_outline(self, x: f32, y: f32, width: f32, height: f32) -> PaintPath {
        self.polyline(&[
            (x, y),
            (x + width, y),
            (x + width, y + height),
            (x, y + height),
            (x, y),
        ])
    }

    fn circle(self, cx: f32, cy: f32, radius: f32, filled: bool) -> PaintPath {
        let segments = 16;
        let mut points = Vec::with_capacity(segments + 1);
        for index in 0..segments {
            let angle = (index as f32 / segments as f32) * std::f32::consts::TAU;
            points.push((cx + radius * angle.cos(), cy + radius * angle.sin()));
        }
        if filled {
            self.filled(&points)
        } else {
            points.push(points[0]);
            self.polyline(&points)
        }
    }

    fn lock_body(self) -> PaintPath {
        self.filled_rect(6.0, 10.0, 12.0, 9.0)
    }

    fn warning_cutout_line(self) -> PaintPath {
        PaintPath::new()
            .move_to(self.point(12.0, 9.0))
            .line_to(self.point(12.0, 14.5))
            .stroke(StrokeStyle::new(
                ColorRgba::new(0, 0, 0, self.tint.a),
                self.stroke_width,
            ))
    }

    fn warning_cutout_dot(self) -> PaintPath {
        IconPathBuilder {
            tint: ColorRgba::new(0, 0, 0, self.tint.a),
            ..self
        }
        .circle(12.0, 17.0, 1.0, true)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IconDescriptor {
    pub handle: IconHandle,
    pub label: String,
    pub default_size: UiSize,
    pub alignment: ImageAlignment,
    pub keywords: Vec<String>,
    pub tintable: bool,
}

impl IconDescriptor {
    pub fn new(handle: IconHandle, label: impl Into<String>) -> Self {
        Self {
            handle,
            label: label.into(),
            default_size: UiSize::new(16.0, 16.0),
            alignment: ImageAlignment::Center,
            keywords: Vec::new(),
            tintable: true,
        }
    }

    pub fn default_size(mut self, size: UiSize) -> Self {
        if size.width.is_finite()
            && size.height.is_finite()
            && size.width > 0.0
            && size.height > 0.0
        {
            self.default_size = size;
        }
        self
    }

    pub const fn alignment(mut self, alignment: ImageAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn keyword(mut self, keyword: impl Into<String>) -> Self {
        let keyword = keyword.into();
        if !keyword.is_empty() && !self.keywords.contains(&keyword) {
            self.keywords.push(keyword);
        }
        self
    }

    pub const fn tintable(mut self, tintable: bool) -> Self {
        self.tintable = tintable;
        self
    }

    pub fn resource_handle(&self) -> ResourceHandle {
        ResourceHandle::Icon(self.handle.clone())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageDescriptor {
    pub handle: ImageHandle,
    pub label: String,
    pub default_size: UiSize,
    pub alignment: ImageAlignment,
    pub keywords: Vec<String>,
    pub tintable: bool,
}

impl ImageDescriptor {
    pub fn new(handle: ImageHandle, label: impl Into<String>, default_size: UiSize) -> Self {
        Self {
            handle,
            label: label.into(),
            default_size,
            alignment: ImageAlignment::Center,
            keywords: Vec::new(),
            tintable: false,
        }
    }

    pub const fn alignment(mut self, alignment: ImageAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn keyword(mut self, keyword: impl Into<String>) -> Self {
        let keyword = keyword.into();
        if !keyword.is_empty() && !self.keywords.contains(&keyword) {
            self.keywords.push(keyword);
        }
        self
    }

    pub const fn tintable(mut self, tintable: bool) -> Self {
        self.tintable = tintable;
        self
    }

    pub fn resource_handle(&self) -> ResourceHandle {
        ResourceHandle::Image(self.handle.clone())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IconAsset {
    pub handle: IconHandle,
    pub label: String,
    pub size: UiSize,
    pub alignment: ImageAlignment,
    pub tint: Option<ColorRgba>,
}

impl IconAsset {
    pub fn from_descriptor(descriptor: &IconDescriptor) -> Self {
        Self {
            handle: descriptor.handle.clone(),
            label: descriptor.label.clone(),
            size: descriptor.default_size,
            alignment: descriptor.alignment,
            tint: None,
        }
    }

    pub fn tint(mut self, tint: ColorRgba) -> Self {
        self.tint = Some(tint);
        self
    }

    pub fn size(mut self, size: UiSize) -> Self {
        if size.width.is_finite()
            && size.height.is_finite()
            && size.width > 0.0
            && size.height > 0.0
        {
            self.size = size;
        }
        self
    }

    pub const fn alignment(mut self, alignment: ImageAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn image_content(&self) -> ImageContent {
        let content = ImageContent::new(self.handle.id.key.clone());
        if let Some(tint) = self.tint {
            content.tinted(tint)
        } else {
            content
        }
    }

    pub fn fallback_paths(&self) -> Option<Vec<PaintPath>> {
        let tint = self.tint.unwrap_or(ColorRgba::WHITE);
        let rect = UiRect::new(0.0, 0.0, self.size.width, self.size.height);
        BuiltInIcon::from_handle(&self.handle).map(|icon| icon.fallback_paths(rect, tint))
    }

    pub fn fallback_scene(&self) -> Option<Vec<ScenePrimitive>> {
        self.fallback_paths().map(|paths| {
            paths
                .into_iter()
                .map(ScenePrimitive::Path)
                .collect::<Vec<_>>()
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IconButtonAsset {
    pub icon: IconAsset,
    pub accessibility_label: String,
    pub tooltip: Option<String>,
    pub shortcut: Option<String>,
}

impl IconButtonAsset {
    pub fn new(icon: IconAsset, accessibility_label: impl Into<String>) -> Self {
        Self {
            icon,
            accessibility_label: accessibility_label.into(),
            tooltip: None,
            shortcut: None,
        }
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn tooltip_text(&self) -> Option<String> {
        match (&self.tooltip, &self.shortcut) {
            (Some(tooltip), Some(shortcut)) => Some(format!("{tooltip} ({shortcut})")),
            (Some(tooltip), None) => Some(tooltip.clone()),
            (None, Some(shortcut)) => Some(format!("{} ({shortcut})", self.accessibility_label)),
            (None, None) => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssetRegistry {
    icons: HashMap<ResourceId, IconDescriptor>,
    images: HashMap<ResourceId, ImageDescriptor>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_common_icons() -> Self {
        let mut registry = Self::new();
        registry.register_common_icons();
        registry
    }

    pub fn register_common_icons(&mut self) {
        for icon in BuiltInIcon::COMMON {
            self.register_icon(icon.descriptor());
        }
    }

    pub fn register_icon(&mut self, descriptor: IconDescriptor) -> Option<IconDescriptor> {
        self.icons.insert(descriptor.handle.id.clone(), descriptor)
    }

    pub fn register_image(&mut self, descriptor: ImageDescriptor) -> Option<ImageDescriptor> {
        self.images.insert(descriptor.handle.id.clone(), descriptor)
    }

    pub fn resolve_icon(&self, handle: &IconHandle) -> Option<&IconDescriptor> {
        self.icons.get(&handle.id)
    }

    pub fn resolve_built_in_icon(&self, icon: BuiltInIcon) -> Option<&IconDescriptor> {
        self.resolve_icon(&icon.handle())
    }

    pub fn resolve_image(&self, handle: &ImageHandle) -> Option<&ImageDescriptor> {
        self.images.get(&handle.id)
    }

    pub fn icon_asset(&self, handle: &IconHandle) -> Option<IconAsset> {
        self.resolve_icon(handle).map(IconAsset::from_descriptor)
    }

    pub fn built_in_icon_asset(&self, icon: BuiltInIcon) -> Option<IconAsset> {
        self.icon_asset(&icon.handle())
    }

    pub fn icon_button(
        &self,
        icon: BuiltInIcon,
        accessibility_label: impl Into<String>,
    ) -> Option<IconButtonAsset> {
        self.built_in_icon_asset(icon)
            .map(|asset| IconButtonAsset::new(asset, accessibility_label))
    }

    pub fn icon_count(&self) -> usize {
        self.icons.len()
    }

    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    pub fn search_icons(&self, query: &str) -> Vec<&IconDescriptor> {
        let query = query.trim().to_ascii_lowercase();
        let mut matches = self
            .icons
            .values()
            .filter(|descriptor| {
                if query.is_empty() {
                    return true;
                }
                descriptor.label.to_ascii_lowercase().contains(&query)
                    || descriptor
                        .handle
                        .id
                        .key
                        .to_ascii_lowercase()
                        .contains(&query)
                    || descriptor
                        .keywords
                        .iter()
                        .any(|keyword| keyword.to_ascii_lowercase().contains(&query))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            left.label
                .cmp(&right.label)
                .then_with(|| left.handle.id.key.cmp(&right.handle.id.key))
        });
        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ResourceDomain;

    #[test]
    fn common_icon_registry_resolves_built_in_action_handles() {
        let registry = AssetRegistry::with_common_icons();
        let play = registry
            .resolve_built_in_icon(BuiltInIcon::Play)
            .expect("play icon");
        let grid = registry
            .resolve_built_in_icon(BuiltInIcon::Grid)
            .expect("grid icon");

        assert_eq!(registry.icon_count(), BuiltInIcon::COMMON.len());
        assert_eq!(play.handle.id.domain, ResourceDomain::BuiltIn);
        assert_eq!(play.handle.id.key, "icons.play");
        assert_eq!(play.label, "Play");
        assert!(play.keywords.iter().any(|keyword| keyword == "transport"));
        assert_eq!(grid.handle.id.key, "icons.grid");
        assert!(grid.keywords.iter().any(|keyword| keyword == "snap"));
    }

    #[test]
    fn app_icons_and_images_preserve_resource_handles_metadata_and_replacements() {
        let mut registry = AssetRegistry::new();
        let icon = IconDescriptor::new(IconHandle::app("orbifold.track.synth"), "Synth track")
            .default_size(UiSize::new(20.0, 20.0))
            .alignment(ImageAlignment::Start)
            .keyword("track")
            .keyword("instrument");
        let replacement =
            IconDescriptor::new(IconHandle::app("orbifold.track.synth"), "Instrument track");
        let image = ImageDescriptor::new(
            ImageHandle::app("covers.scale"),
            "Scale cover",
            UiSize::new(64.0, 48.0),
        )
        .alignment(ImageAlignment::End)
        .tintable(true)
        .keyword("scale");

        assert!(registry.register_icon(icon).is_none());
        let previous = registry
            .register_icon(replacement)
            .expect("previous descriptor");
        assert_eq!(previous.label, "Synth track");
        assert!(registry.register_image(image).is_none());

        let resolved_icon = registry
            .resolve_icon(&IconHandle::app("orbifold.track.synth"))
            .expect("app icon");
        let resolved_image = registry
            .resolve_image(&ImageHandle::app("covers.scale"))
            .expect("app image");

        assert_eq!(resolved_icon.label, "Instrument track");
        assert_eq!(
            resolved_icon.resource_handle().kind(),
            crate::platform::ResourceKind::Icon
        );
        assert_eq!(resolved_image.default_size, UiSize::new(64.0, 48.0));
        assert!(resolved_image.tintable);
        assert_eq!(
            resolved_image.resource_handle().kind(),
            crate::platform::ResourceKind::Image
        );
    }

    #[test]
    fn icon_assets_convert_to_existing_image_content_with_tint_size_and_alignment() {
        let registry = AssetRegistry::with_common_icons();
        let icon = registry
            .built_in_icon_asset(BuiltInIcon::Record)
            .expect("record icon")
            .tint(ColorRgba::new(220, 42, 58, 255))
            .size(UiSize::new(18.0, 18.0))
            .alignment(ImageAlignment::End);
        let content = icon.image_content();

        assert_eq!(icon.handle.id.domain, ResourceDomain::BuiltIn);
        assert_eq!(icon.size, UiSize::new(18.0, 18.0));
        assert_eq!(icon.alignment, ImageAlignment::End);
        assert_eq!(content.key, "icons.record");
        assert_eq!(content.tint, Some(ColorRgba::new(220, 42, 58, 255)));
    }

    #[test]
    fn built_in_icons_provide_vector_fallback_paths() {
        for icon in BuiltInIcon::COMMON {
            let paths = icon.fallback_paths(
                UiRect::new(0.0, 0.0, 24.0, 24.0),
                ColorRgba::new(120, 180, 255, 255),
            );
            assert!(!paths.is_empty(), "{icon:?} has no fallback paths");
            assert!(
                paths
                    .iter()
                    .any(|path| path.fill.is_some() || path.stroke.is_some()),
                "{icon:?} fallback paths are not paintable"
            );
            assert!(
                paths
                    .iter()
                    .any(|path| path.bounds().width > 0.0 || path.bounds().height > 0.0),
                "{icon:?} fallback paths have empty bounds"
            );
        }
    }

    #[test]
    fn icon_asset_fallback_scene_uses_tint_size_and_built_in_domain() {
        let registry = AssetRegistry::with_common_icons();
        let tint = ColorRgba::new(220, 42, 58, 255);
        let icon = registry
            .built_in_icon_asset(BuiltInIcon::Record)
            .expect("record icon")
            .tint(tint)
            .size(UiSize::new(18.0, 20.0));

        let paths = icon.fallback_paths().expect("built-in fallback paths");
        let scene = icon.fallback_scene().expect("built-in fallback scene");

        assert!(paths.iter().any(|path| {
            path.fill
                .as_ref()
                .is_some_and(|fill| fill.fallback_color() == tint)
                || path.stroke.is_some_and(|stroke| stroke.style.color == tint)
        }));
        assert_eq!(scene.len(), paths.len());
        assert!(matches!(scene[0], ScenePrimitive::Path(_)));

        let app_icon = IconAsset {
            handle: IconHandle::app("icons.record"),
            label: "Record".to_string(),
            size: UiSize::new(18.0, 18.0),
            alignment: ImageAlignment::Center,
            tint: Some(tint),
        };
        assert!(app_icon.fallback_paths().is_none());
    }

    #[test]
    fn icon_button_asset_carries_accessibility_tooltip_and_shortcut_text() {
        let registry = AssetRegistry::with_common_icons();
        let button = registry
            .icon_button(BuiltInIcon::Play, "Play transport")
            .expect("play button")
            .tooltip("Start playback")
            .shortcut("Space");

        assert_eq!(button.accessibility_label, "Play transport");
        assert_eq!(
            button.tooltip_text().as_deref(),
            Some("Start playback (Space)")
        );
        assert_eq!(button.icon.image_content().key, "icons.play");

        let shortcut_only = IconButtonAsset::new(button.icon, "Stop transport").shortcut("Esc");
        assert_eq!(
            shortcut_only.tooltip_text().as_deref(),
            Some("Stop transport (Esc)")
        );
    }

    #[test]
    fn icon_search_matches_labels_keys_and_keywords_in_stable_order() {
        let registry = AssetRegistry::with_common_icons();

        let transport = registry.search_icons("transport");
        let labels = transport
            .iter()
            .map(|descriptor| descriptor.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                "Fast forward",
                "Loop",
                "Metronome",
                "Pause",
                "Play",
                "Record",
                "Rewind",
                "Stop"
            ]
        );

        let scissors = registry.search_icons("icons.scissors");
        assert_eq!(scissors.len(), 1);
        assert_eq!(scissors[0].label, "Scissors");
    }
}
