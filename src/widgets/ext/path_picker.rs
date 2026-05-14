//! Path picker widget model.

use std::path::{Component, Path, PathBuf};

use crate::{AccessibilityMeta, AccessibilityRole, ColorRgba, EditPhase, ImageContent};

use super::pickers::{PickerAnimationMeta, PickerElementStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPickerMode {
    OpenFile,
    SaveFile,
    Directory,
    Any,
}

impl PathPickerMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenFile => "Open file",
            Self::SaveFile => "Save file",
            Self::Directory => "Choose directory",
            Self::Any => "Choose path",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathBreadcrumb {
    pub label: String,
    pub path: PathBuf,
    pub is_root: bool,
}

impl PathBreadcrumb {
    pub fn accessibility_meta(&self) -> AccessibilityMeta {
        AccessibilityMeta::new(AccessibilityRole::Button)
            .label(format!("Go to {}", self.label))
            .value(path_to_text(&self.path))
            .focusable()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPickerState {
    pub mode: PathPickerMode,
    pub current_path: PathBuf,
    pub selected_path: Option<PathBuf>,
    pub text: String,
    pub recent_paths: Vec<PathBuf>,
    pub max_recent: usize,
}

impl PathPickerState {
    pub fn new(mode: PathPickerMode, current_path: impl Into<PathBuf>) -> Self {
        Self {
            mode,
            current_path: current_path.into(),
            selected_path: None,
            text: String::new(),
            recent_paths: Vec::new(),
            max_recent: 8,
        }
    }

    pub fn with_selected_path(mut self, selected_path: impl Into<PathBuf>) -> Self {
        let selected_path = selected_path.into();
        self.text = path_to_text(&selected_path);
        self.selected_path = Some(selected_path);
        self
    }

    pub fn with_recent_paths(mut self, recent_paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let paths: Vec<_> = recent_paths.into_iter().collect();
        for path in paths.into_iter().rev() {
            self.remember_recent(path);
        }
        self
    }

    pub fn breadcrumbs(&self) -> Vec<PathBreadcrumb> {
        path_breadcrumbs(&self.current_path)
    }

    pub fn validation(&self) -> PathTextValidation {
        validate_path_text(&self.text)
    }

    pub fn field_accessibility_meta(&self, label: impl Into<String>) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(AccessibilityRole::TextBox)
            .label(label)
            .value(self.text.clone())
            .hint(format!(
                "{}. Current folder {}",
                self.mode.label(),
                path_to_text(&self.current_path)
            ))
            .focusable();
        if !self.validation().is_valid() && !self.text.is_empty() {
            meta = meta.hint("Enter a path");
        }
        meta
    }

    pub fn control_accessibility_meta(&self, control: PathPickerControl) -> AccessibilityMeta {
        let mut meta = AccessibilityMeta::new(AccessibilityRole::Button)
            .label(control.label(self))
            .focusable();
        if let Some(value) = control.value(self) {
            meta = meta.value(value);
        }
        if !control.enabled(self) {
            meta = meta.disabled();
        }
        meta
    }

    pub fn copy_current_path(&self) -> String {
        path_to_text(&self.current_path)
    }

    pub fn copy_selected_path(&self) -> Option<String> {
        self.selected_path.as_deref().map(path_to_text)
    }

    pub fn copy_text(&self) -> String {
        if self.text.is_empty() {
            self.copy_selected_path()
                .unwrap_or_else(|| self.copy_current_path())
        } else {
            self.text.clone()
        }
    }

    pub fn update_text(&mut self, text: impl Into<String>) -> PathPickerUpdate {
        let previous = self.selected_path.clone();
        let text = text.into();
        let changed = self.text != text;
        self.text = text;
        PathPickerUpdate {
            previous,
            selected_path: self.selected_path.clone(),
            current_path: self.current_path.clone(),
            text: self.text.clone(),
            phase: EditPhase::UpdateEdit,
            changed,
        }
    }

    pub fn paste_path_text(&mut self, text: &str) -> Option<PathPickerUpdate> {
        parse_path_text(text).map(|path| self.select_path(path))
    }

    pub fn commit_text(&mut self) -> PathPickerUpdate {
        let previous = self.selected_path.clone();
        let Some(path) = parse_path_text(&self.text) else {
            return PathPickerUpdate {
                previous,
                selected_path: self.selected_path.clone(),
                current_path: self.current_path.clone(),
                text: self.text.clone(),
                phase: EditPhase::CancelEdit,
                changed: false,
            };
        };
        self.select_path(path)
    }

    pub fn navigate_to(&mut self, path: impl Into<PathBuf>) -> PathPickerUpdate {
        let previous = self.selected_path.clone();
        let path = path.into();
        let changed = self.current_path != path;
        self.current_path = path;
        PathPickerUpdate {
            previous,
            selected_path: self.selected_path.clone(),
            current_path: self.current_path.clone(),
            text: self.text.clone(),
            phase: EditPhase::UpdateEdit,
            changed,
        }
    }

    pub fn select_path(&mut self, path: impl Into<PathBuf>) -> PathPickerUpdate {
        let previous = self.selected_path.clone();
        let path = path.into();
        self.text = path_to_text(&path);
        self.selected_path = Some(path.clone());
        self.remember_recent(path);
        let changed = previous != self.selected_path;
        PathPickerUpdate {
            previous,
            selected_path: self.selected_path.clone(),
            current_path: self.current_path.clone(),
            text: self.text.clone(),
            phase: EditPhase::CommitEdit,
            changed,
        }
    }

    pub fn clear_selection(&mut self) -> PathPickerUpdate {
        let previous = self.selected_path.take();
        self.text.clear();
        PathPickerUpdate {
            previous: previous.clone(),
            selected_path: None,
            current_path: self.current_path.clone(),
            text: self.text.clone(),
            phase: EditPhase::CancelEdit,
            changed: previous.is_some(),
        }
    }

    pub fn remember_recent(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        self.recent_paths.retain(|recent| recent != &path);
        self.recent_paths.insert(0, path);
        self.recent_paths.truncate(self.max_recent);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPickerUpdate {
    pub previous: Option<PathBuf>,
    pub selected_path: Option<PathBuf>,
    pub current_path: PathBuf,
    pub text: String,
    pub phase: EditPhase,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathTextValidationStatus {
    Valid,
    Empty,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathTextValidation {
    pub status: PathTextValidationStatus,
    pub path: Option<PathBuf>,
    pub message: Option<String>,
}

impl PathTextValidation {
    pub fn is_valid(&self) -> bool {
        self.status == PathTextValidationStatus::Valid
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPickerControl {
    Browse,
    Clear,
    Confirm,
}

impl PathPickerControl {
    fn label(self, picker: &PathPickerState) -> String {
        match self {
            Self::Browse => picker.mode.label().to_string(),
            Self::Clear => "Clear selected path".to_string(),
            Self::Confirm => match picker.mode {
                PathPickerMode::OpenFile => "Open selected path".to_string(),
                PathPickerMode::SaveFile => "Save to selected path".to_string(),
                PathPickerMode::Directory => "Choose selected directory".to_string(),
                PathPickerMode::Any => "Choose selected path".to_string(),
            },
        }
    }

    fn value(self, picker: &PathPickerState) -> Option<String> {
        match self {
            Self::Browse => Some(path_to_text(&picker.current_path)),
            Self::Clear | Self::Confirm => picker.selected_path.as_deref().map(path_to_text),
        }
    }

    fn enabled(self, picker: &PathPickerState) -> bool {
        match self {
            Self::Browse => true,
            Self::Clear | Self::Confirm => picker.selected_path.is_some(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathPickerStyle {
    pub text_field: PickerElementStyle,
    pub invalid_text_field: PickerElementStyle,
    pub browse_button: PickerElementStyle,
    pub breadcrumb_button: PickerElementStyle,
    pub selected_path: PickerElementStyle,
}

impl PathPickerStyle {
    pub fn style_for_validation(&self, validation: &PathTextValidation) -> &PickerElementStyle {
        if validation.is_valid() || validation.status == PathTextValidationStatus::Empty {
            &self.text_field
        } else {
            &self.invalid_text_field
        }
    }
}

impl Default for PathPickerStyle {
    fn default() -> Self {
        Self {
            text_field: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(235, 240, 247, 255))
                .with_background(ColorRgba::new(18, 22, 28, 255)),
            invalid_text_field: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(255, 238, 240, 255))
                .with_border(ColorRgba::new(201, 74, 91, 255)),
            browse_button: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(238, 242, 248, 255))
                .with_background(ColorRgba::new(35, 42, 53, 255))
                .with_image(ImageContent::new("icons.folder")),
            breadcrumb_button: PickerElementStyle::default()
                .with_foreground(ColorRgba::new(199, 209, 223, 255)),
            selected_path: PickerElementStyle::default()
                .with_background(ColorRgba::new(47, 72, 103, 255))
                .with_animation(PickerAnimationMeta::new("path.selected", 0.10)),
        }
    }
}

pub fn path_breadcrumbs(path: impl AsRef<Path>) -> Vec<PathBreadcrumb> {
    let mut crumbs = Vec::new();
    let mut current = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            Component::Prefix(prefix) => {
                current.push(prefix.as_os_str());
                crumbs.push(PathBreadcrumb {
                    label: prefix.as_os_str().to_string_lossy().into_owned(),
                    path: current.clone(),
                    is_root: true,
                });
            }
            Component::RootDir => {
                current.push(component.as_os_str());
                crumbs.push(PathBreadcrumb {
                    label: std::path::MAIN_SEPARATOR.to_string(),
                    path: current.clone(),
                    is_root: true,
                });
            }
            Component::Normal(part) => {
                current.push(part);
                crumbs.push(PathBreadcrumb {
                    label: part.to_string_lossy().into_owned(),
                    path: current.clone(),
                    is_root: false,
                });
            }
            Component::ParentDir => {
                current.push("..");
                crumbs.push(PathBreadcrumb {
                    label: "..".to_string(),
                    path: current.clone(),
                    is_root: false,
                });
            }
            Component::CurDir => {}
        }
    }

    if crumbs.is_empty() {
        crumbs.push(PathBreadcrumb {
            label: ".".to_string(),
            path: PathBuf::from("."),
            is_root: false,
        });
    }

    crumbs
}

fn path_to_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn parse_path_text(text: &str) -> Option<PathBuf> {
    let text = normalize_path_clipboard_text(text);
    (!text.is_empty() && !text.contains('\0')).then(|| PathBuf::from(text))
}

fn validate_path_text(text: &str) -> PathTextValidation {
    let text = normalize_path_clipboard_text(text);
    if text.is_empty() {
        return PathTextValidation {
            status: PathTextValidationStatus::Empty,
            path: None,
            message: Some("Enter a path".to_string()),
        };
    }
    if text.contains('\0') {
        return PathTextValidation {
            status: PathTextValidationStatus::Invalid,
            path: None,
            message: Some("Path contains an invalid null byte".to_string()),
        };
    }
    PathTextValidation {
        status: PathTextValidationStatus::Valid,
        path: Some(PathBuf::from(text)),
        message: None,
    }
}

fn normalize_path_clipboard_text(text: &str) -> String {
    let text = text.trim();
    let text = text
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .or_else(|| {
            text.strip_prefix('\'')
                .and_then(|text| text.strip_suffix('\''))
        })
        .unwrap_or(text);
    text.trim().to_string()
}
