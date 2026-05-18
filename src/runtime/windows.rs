//! Backend-neutral multi-window routing state.
//!
//! The router keeps host-owned window, document, and render surface identity
//! separate from concrete backends. It validates ownership before host events
//! are handed to document processing and records the per-window routing state
//! needed by frame integration.

use std::collections::HashMap;

use crate::accessibility::AccessibilityAdapterRequest;
use crate::input::{PointerEventKind, RawKeyboardEvent, RawPointerEvent};
use crate::platform::{CursorRequest, TextImeRequest};
use crate::renderer::RenderTarget;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowId(pub(crate) String);

impl WindowId {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentId(pub(crate) String);

impl DocumentId {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub(crate) String);

impl SurfaceId {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OverlayId(pub(crate) String);

impl OverlayId {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowDocumentTarget {
    pub window_id: WindowId,
    pub document_id: DocumentId,
}

impl WindowDocumentTarget {
    pub fn new(window_id: WindowId, document_id: DocumentId) -> Self {
        Self {
            window_id,
            document_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayOwner {
    pub overlay_id: OverlayId,
    pub document_id: DocumentId,
}

impl OverlayOwner {
    pub fn new(overlay_id: OverlayId, document_id: DocumentId) -> Self {
        Self {
            overlay_id,
            document_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSurfaceOwner {
    pub surface_id: SurfaceId,
    pub document_id: DocumentId,
}

impl RenderSurfaceOwner {
    pub fn new(surface_id: SurfaceId, document_id: DocumentId) -> Self {
        Self {
            surface_id,
            document_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRoutingState {
    pub window_id: WindowId,
    pub documents: Vec<DocumentId>,
    pub focused: Option<DocumentId>,
    pub hovered: Option<DocumentId>,
    pub pressed: Option<DocumentId>,
    pub ime_target: Option<DocumentId>,
    pub cursor_target: Option<DocumentId>,
    pub overlay_owner: Option<OverlayOwner>,
    pub accessibility_target: Option<DocumentId>,
    pub render_surface_owner: Option<RenderSurfaceOwner>,
}

impl WindowRoutingState {
    pub fn new(window_id: WindowId) -> Self {
        Self {
            window_id,
            documents: Vec::new(),
            focused: None,
            hovered: None,
            pressed: None,
            ime_target: None,
            cursor_target: None,
            overlay_owner: None,
            accessibility_target: None,
            render_surface_owner: None,
        }
    }

    pub fn summary(&self) -> WindowRoutingSummary {
        WindowRoutingSummary {
            window_id: self.window_id.clone(),
            document_count: self.documents.len(),
            focused: self.focused.clone(),
            hovered: self.hovered.clone(),
            pressed: self.pressed.clone(),
            ime_target: self.ime_target.clone(),
            cursor_target: self.cursor_target.clone(),
            overlay_owner: self.overlay_owner.clone(),
            accessibility_target: self.accessibility_target.clone(),
            render_surface_owner: self.render_surface_owner.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRoutingSummary {
    pub window_id: WindowId,
    pub document_count: usize,
    pub focused: Option<DocumentId>,
    pub hovered: Option<DocumentId>,
    pub pressed: Option<DocumentId>,
    pub ime_target: Option<DocumentId>,
    pub cursor_target: Option<DocumentId>,
    pub overlay_owner: Option<OverlayOwner>,
    pub accessibility_target: Option<DocumentId>,
    pub render_surface_owner: Option<RenderSurfaceOwner>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowRouteEventKind {
    Pointer,
    Key,
    Ime,
    Cursor,
    Accessibility,
    Render,
    Focus,
    Overlay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowRouteRejection {
    UnknownWindow {
        window_id: WindowId,
        kind: WindowRouteEventKind,
    },
    UnknownDocument {
        window_id: WindowId,
        document_id: DocumentId,
        kind: WindowRouteEventKind,
    },
    DocumentInDifferentWindow {
        window_id: WindowId,
        document_id: DocumentId,
        actual_window_id: WindowId,
        kind: WindowRouteEventKind,
    },
    NoFocusedWindow {
        kind: WindowRouteEventKind,
    },
    NoFocusedDocument {
        window_id: WindowId,
        kind: WindowRouteEventKind,
    },
    UnknownSurface {
        window_id: WindowId,
        surface_id: SurfaceId,
        kind: WindowRouteEventKind,
    },
    SurfaceOwnedByDifferentDocument {
        window_id: WindowId,
        surface_id: SurfaceId,
        document_id: DocumentId,
        actual_document_id: DocumentId,
        kind: WindowRouteEventKind,
    },
    RenderTargetWindowMismatch {
        window_id: WindowId,
        target_window_id: String,
        kind: WindowRouteEventKind,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoutedWindowEvent {
    Pointer {
        target: WindowDocumentTarget,
        event: RawPointerEvent,
    },
    Key {
        target: WindowDocumentTarget,
        event: RawKeyboardEvent,
    },
    Ime {
        target: WindowDocumentTarget,
        request: TextImeRequest,
    },
    Cursor {
        target: WindowDocumentTarget,
        request: CursorRequest,
    },
    Accessibility {
        target: WindowDocumentTarget,
        request: AccessibilityAdapterRequest,
    },
}

impl RoutedWindowEvent {
    pub const fn target(&self) -> &WindowDocumentTarget {
        match self {
            Self::Pointer { target, .. }
            | Self::Key { target, .. }
            | Self::Ime { target, .. }
            | Self::Cursor { target, .. }
            | Self::Accessibility { target, .. } => target,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoutedRenderTarget {
    pub target: WindowDocumentTarget,
    pub surface_id: SurfaceId,
    pub render_target: RenderTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DocumentRegistration {
    window_id: WindowId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderSurfaceRegistration {
    window_id: WindowId,
    document_id: DocumentId,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WindowRouter {
    windows: HashMap<WindowId, WindowRoutingState>,
    documents: HashMap<DocumentId, DocumentRegistration>,
    render_surfaces: HashMap<SurfaceId, RenderSurfaceRegistration>,
    focused_window: Option<WindowId>,
    rejected: Vec<WindowRouteRejection>,
}

impl WindowRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_window(&mut self, window_id: impl Into<WindowId>) -> bool {
        let window_id = window_id.into();
        if self.windows.contains_key(&window_id) {
            return false;
        }
        self.windows
            .insert(window_id.clone(), WindowRoutingState::new(window_id));
        true
    }

    pub fn register_document(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
    ) -> Result<(), WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        let Some(window) = self.windows.get_mut(&window_id) else {
            let rejection = WindowRouteRejection::UnknownWindow {
                window_id,
                kind: WindowRouteEventKind::Focus,
            };
            self.rejected.push(rejection.clone());
            return Err(rejection);
        };
        if let Some(existing) = self.documents.get(&document_id) {
            if existing.window_id != window_id {
                let rejection = WindowRouteRejection::DocumentInDifferentWindow {
                    window_id,
                    document_id,
                    actual_window_id: existing.window_id.clone(),
                    kind: WindowRouteEventKind::Focus,
                };
                self.rejected.push(rejection.clone());
                return Err(rejection);
            }
            return Ok(());
        }
        window.documents.push(document_id.clone());
        self.documents
            .insert(document_id, DocumentRegistration { window_id });
        Ok(())
    }

    pub fn register_render_surface(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
        surface_id: impl Into<SurfaceId>,
    ) -> Result<(), WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        let surface_id = surface_id.into();
        self.validate_document(&window_id, &document_id, WindowRouteEventKind::Render)?;
        self.render_surfaces.insert(
            surface_id.clone(),
            RenderSurfaceRegistration {
                window_id: window_id.clone(),
                document_id: document_id.clone(),
            },
        );
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.render_surface_owner = Some(RenderSurfaceOwner::new(surface_id, document_id));
        }
        Ok(())
    }

    pub fn transfer_focus(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
    ) -> Result<WindowDocumentTarget, WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        self.validate_document(&window_id, &document_id, WindowRouteEventKind::Focus)?;

        if let Some(previous_window_id) = self.focused_window.as_ref() {
            if previous_window_id != &window_id {
                if let Some(previous_window) = self.windows.get_mut(previous_window_id) {
                    previous_window.focused = None;
                }
            }
        }
        self.focused_window = Some(window_id.clone());
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.focused = Some(document_id.clone());
        }
        Ok(WindowDocumentTarget::new(window_id, document_id))
    }

    pub fn route_pointer(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
        event: RawPointerEvent,
    ) -> Result<RoutedWindowEvent, WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        self.validate_document(&window_id, &document_id, WindowRouteEventKind::Pointer)?;
        if let Some(window) = self.windows.get_mut(&window_id) {
            match event.kind {
                PointerEventKind::Move => {
                    window.hovered = Some(document_id.clone());
                }
                PointerEventKind::Down(_) => {
                    window.hovered = Some(document_id.clone());
                    window.pressed = Some(document_id.clone());
                }
                PointerEventKind::Up(_) | PointerEventKind::Cancel => {
                    window.pressed = None;
                }
            }
        }
        Ok(RoutedWindowEvent::Pointer {
            target: WindowDocumentTarget::new(window_id, document_id),
            event,
        })
    }

    pub fn route_key(
        &mut self,
        event: RawKeyboardEvent,
    ) -> Result<RoutedWindowEvent, WindowRouteRejection> {
        let window_id = self.focused_window.clone().ok_or_else(|| {
            let rejection = WindowRouteRejection::NoFocusedWindow {
                kind: WindowRouteEventKind::Key,
            };
            self.rejected.push(rejection.clone());
            rejection
        })?;
        let document_id = self
            .windows
            .get(&window_id)
            .and_then(|window| window.focused.clone())
            .ok_or_else(|| {
                let rejection = WindowRouteRejection::NoFocusedDocument {
                    window_id: window_id.clone(),
                    kind: WindowRouteEventKind::Key,
                };
                self.rejected.push(rejection.clone());
                rejection
            })?;
        Ok(RoutedWindowEvent::Key {
            target: WindowDocumentTarget::new(window_id, document_id),
            event,
        })
    }

    pub fn route_key_to(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
        event: RawKeyboardEvent,
    ) -> Result<RoutedWindowEvent, WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        self.validate_document(&window_id, &document_id, WindowRouteEventKind::Key)?;
        Ok(RoutedWindowEvent::Key {
            target: WindowDocumentTarget::new(window_id, document_id),
            event,
        })
    }

    pub fn route_ime(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
        request: TextImeRequest,
    ) -> Result<RoutedWindowEvent, WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        self.validate_document(&window_id, &document_id, WindowRouteEventKind::Ime)?;
        if let Some(window) = self.windows.get_mut(&window_id) {
            match request {
                TextImeRequest::Deactivate { .. } | TextImeRequest::HideKeyboard { .. } => {
                    if window.ime_target.as_ref() == Some(&document_id) {
                        window.ime_target = None;
                    }
                }
                TextImeRequest::Activate(_)
                | TextImeRequest::Update(_)
                | TextImeRequest::ShowKeyboard { .. } => {
                    window.ime_target = Some(document_id.clone());
                }
            }
        }
        Ok(RoutedWindowEvent::Ime {
            target: WindowDocumentTarget::new(window_id, document_id),
            request,
        })
    }

    pub fn route_cursor(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
        request: CursorRequest,
    ) -> Result<RoutedWindowEvent, WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        self.validate_document(&window_id, &document_id, WindowRouteEventKind::Cursor)?;
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.cursor_target = Some(document_id.clone());
        }
        Ok(RoutedWindowEvent::Cursor {
            target: WindowDocumentTarget::new(window_id, document_id),
            request,
        })
    }

    pub fn route_accessibility(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
        request: AccessibilityAdapterRequest,
    ) -> Result<RoutedWindowEvent, WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        self.validate_document(
            &window_id,
            &document_id,
            WindowRouteEventKind::Accessibility,
        )?;
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.accessibility_target = Some(document_id.clone());
        }
        Ok(RoutedWindowEvent::Accessibility {
            target: WindowDocumentTarget::new(window_id, document_id),
            request,
        })
    }

    pub fn claim_overlay(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
        overlay_id: impl Into<OverlayId>,
    ) -> Result<OverlayOwner, WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        let overlay_id = overlay_id.into();
        self.validate_document(&window_id, &document_id, WindowRouteEventKind::Overlay)?;
        let owner = OverlayOwner::new(overlay_id, document_id);
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.overlay_owner = Some(owner.clone());
        }
        Ok(owner)
    }

    pub fn clear_overlay(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
    ) -> Result<Option<OverlayOwner>, WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        self.validate_document(&window_id, &document_id, WindowRouteEventKind::Overlay)?;
        let Some(window) = self.windows.get_mut(&window_id) else {
            return Ok(None);
        };
        if window
            .overlay_owner
            .as_ref()
            .is_some_and(|owner| owner.document_id == document_id)
        {
            return Ok(window.overlay_owner.take());
        }
        Ok(None)
    }

    pub fn route_render(
        &mut self,
        window_id: impl Into<WindowId>,
        document_id: impl Into<DocumentId>,
        surface_id: impl Into<SurfaceId>,
        render_target: RenderTarget,
    ) -> Result<RoutedRenderTarget, WindowRouteRejection> {
        let window_id = window_id.into();
        let document_id = document_id.into();
        let surface_id = surface_id.into();
        self.validate_document(&window_id, &document_id, WindowRouteEventKind::Render)?;
        self.validate_render_surface(
            &window_id,
            &document_id,
            &surface_id,
            WindowRouteEventKind::Render,
        )?;
        self.validate_render_target_window(&window_id, &render_target)?;
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.render_surface_owner = Some(RenderSurfaceOwner::new(
                surface_id.clone(),
                document_id.clone(),
            ));
        }
        Ok(RoutedRenderTarget {
            target: WindowDocumentTarget::new(window_id, document_id),
            surface_id,
            render_target,
        })
    }

    pub fn window_state(&self, window_id: &WindowId) -> Option<&WindowRoutingState> {
        self.windows.get(window_id)
    }

    pub fn focused_window(&self) -> Option<&WindowId> {
        self.focused_window.as_ref()
    }

    pub fn summaries(&self) -> Vec<WindowRoutingSummary> {
        self.windows
            .values()
            .map(WindowRoutingState::summary)
            .collect()
    }

    pub fn rejected(&self) -> &[WindowRouteRejection] {
        &self.rejected
    }

    pub fn take_rejected(&mut self) -> Vec<WindowRouteRejection> {
        std::mem::take(&mut self.rejected)
    }

    fn validate_document(
        &mut self,
        window_id: &WindowId,
        document_id: &DocumentId,
        kind: WindowRouteEventKind,
    ) -> Result<(), WindowRouteRejection> {
        if !self.windows.contains_key(window_id) {
            let rejection = WindowRouteRejection::UnknownWindow {
                window_id: window_id.clone(),
                kind,
            };
            self.rejected.push(rejection.clone());
            return Err(rejection);
        }
        let Some(registration) = self.documents.get(document_id) else {
            let rejection = WindowRouteRejection::UnknownDocument {
                window_id: window_id.clone(),
                document_id: document_id.clone(),
                kind,
            };
            self.rejected.push(rejection.clone());
            return Err(rejection);
        };
        if &registration.window_id != window_id {
            let rejection = WindowRouteRejection::DocumentInDifferentWindow {
                window_id: window_id.clone(),
                document_id: document_id.clone(),
                actual_window_id: registration.window_id.clone(),
                kind,
            };
            self.rejected.push(rejection.clone());
            return Err(rejection);
        }
        Ok(())
    }

    fn validate_render_surface(
        &mut self,
        window_id: &WindowId,
        document_id: &DocumentId,
        surface_id: &SurfaceId,
        kind: WindowRouteEventKind,
    ) -> Result<(), WindowRouteRejection> {
        let Some(registration) = self.render_surfaces.get(surface_id) else {
            let rejection = WindowRouteRejection::UnknownSurface {
                window_id: window_id.clone(),
                surface_id: surface_id.clone(),
                kind,
            };
            self.rejected.push(rejection.clone());
            return Err(rejection);
        };
        if &registration.window_id != window_id {
            let rejection = WindowRouteRejection::UnknownSurface {
                window_id: window_id.clone(),
                surface_id: surface_id.clone(),
                kind,
            };
            self.rejected.push(rejection.clone());
            return Err(rejection);
        }
        if &registration.document_id != document_id {
            let rejection = WindowRouteRejection::SurfaceOwnedByDifferentDocument {
                window_id: window_id.clone(),
                surface_id: surface_id.clone(),
                document_id: document_id.clone(),
                actual_document_id: registration.document_id.clone(),
                kind,
            };
            self.rejected.push(rejection.clone());
            return Err(rejection);
        }
        Ok(())
    }

    fn validate_render_target_window(
        &mut self,
        window_id: &WindowId,
        render_target: &RenderTarget,
    ) -> Result<(), WindowRouteRejection> {
        let target_window_id = match render_target {
            RenderTarget::Window { id, .. } | RenderTarget::AppOwned { id, .. } => id,
            RenderTarget::Offscreen { .. } | RenderTarget::Snapshot { .. } => return Ok(()),
        };
        if target_window_id == window_id.as_str() {
            return Ok(());
        }
        let rejection = WindowRouteRejection::RenderTargetWindowMismatch {
            window_id: window_id.clone(),
            target_window_id: target_window_id.clone(),
            kind: WindowRouteEventKind::Render,
        };
        self.rejected.push(rejection.clone());
        Err(rejection)
    }
}

impl From<&str> for WindowId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WindowId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for DocumentId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for DocumentId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SurfaceId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SurfaceId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for OverlayId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for OverlayId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::accessibility::AccessibilityAdapterRequest;
    use crate::input::RawKeyboardEvent;
    use crate::platform::{LogicalRect, TextImeSession, TextInputId};
    use crate::renderer::RenderTarget;
    use crate::{FocusRestoreTarget, KeyCode, KeyModifiers, UiNodeId, UiPoint, UiSize};

    use super::*;

    fn router_with_two_windows() -> WindowRouter {
        let mut router = WindowRouter::new();
        assert!(router.register_window("main"));
        assert!(router.register_window("tools"));
        router.register_document("main", "main-doc").unwrap();
        router.register_document("tools", "tools-doc").unwrap();
        router
    }

    #[test]
    fn focus_transfer_between_windows_routes_keys_to_active_window_document() {
        let mut router = router_with_two_windows();
        router.transfer_focus("main", "main-doc").unwrap();
        router.transfer_focus("tools", "tools-doc").unwrap();

        let routed = router
            .route_key(RawKeyboardEvent::press(
                KeyCode::Character('x'),
                KeyModifiers::NONE,
                1,
            ))
            .unwrap();

        assert_eq!(
            routed.target(),
            &WindowDocumentTarget::new(WindowId::new("tools"), DocumentId::new("tools-doc"))
        );
        assert_eq!(router.focused_window(), Some(&WindowId::new("tools")));
        assert_eq!(
            router.window_state(&WindowId::new("main")).unwrap().focused,
            None
        );
        assert_eq!(
            router
                .window_state(&WindowId::new("tools"))
                .unwrap()
                .focused,
            Some(DocumentId::new("tools-doc"))
        );
    }

    #[test]
    fn ime_state_is_isolated_per_window() {
        let mut router = router_with_two_windows();
        let request = TextImeRequest::Activate(TextImeSession::new(
            TextInputId::new("search"),
            LogicalRect::new(0.0, 0.0, 1.0, 16.0),
        ));

        router.route_ime("main", "main-doc", request).unwrap();

        assert_eq!(
            router
                .window_state(&WindowId::new("main"))
                .unwrap()
                .ime_target,
            Some(DocumentId::new("main-doc"))
        );
        assert_eq!(
            router
                .window_state(&WindowId::new("tools"))
                .unwrap()
                .ime_target,
            None
        );
    }

    #[test]
    fn overlay_ownership_is_window_local() {
        let mut router = router_with_two_windows();

        router
            .claim_overlay("main", "main-doc", "main-menu")
            .unwrap();
        router
            .claim_overlay("tools", "tools-doc", "tools-menu")
            .unwrap();
        let cleared = router.clear_overlay("main", "main-doc").unwrap();

        assert_eq!(
            cleared,
            Some(OverlayOwner::new(
                OverlayId::new("main-menu"),
                DocumentId::new("main-doc")
            ))
        );
        assert_eq!(
            router
                .window_state(&WindowId::new("tools"))
                .unwrap()
                .overlay_owner,
            Some(OverlayOwner::new(
                OverlayId::new("tools-menu"),
                DocumentId::new("tools-doc")
            ))
        );
    }

    #[test]
    fn render_target_routes_through_registered_surface_owner() {
        let mut router = router_with_two_windows();
        router
            .register_render_surface("main", "main-doc", "main-surface")
            .unwrap();

        let routed = router
            .route_render(
                "main",
                "main-doc",
                "main-surface",
                RenderTarget::window("main", UiSize::new(640.0, 480.0)),
            )
            .unwrap();

        assert_eq!(routed.surface_id, SurfaceId::new("main-surface"));
        assert_eq!(
            routed.target,
            WindowDocumentTarget::new(WindowId::new("main"), DocumentId::new("main-doc"))
        );
        assert_eq!(
            router
                .window_state(&WindowId::new("main"))
                .unwrap()
                .render_surface_owner,
            Some(RenderSurfaceOwner::new(
                SurfaceId::new("main-surface"),
                DocumentId::new("main-doc")
            ))
        );
    }

    #[test]
    fn unroutable_events_are_reported_and_retained() {
        let mut router = router_with_two_windows();

        let rejection = router
            .route_key(RawKeyboardEvent::press(
                KeyCode::Enter,
                KeyModifiers::NONE,
                1,
            ))
            .unwrap_err();
        assert_eq!(
            rejection,
            WindowRouteRejection::NoFocusedWindow {
                kind: WindowRouteEventKind::Key
            }
        );

        let pointer_rejection = router
            .route_pointer(
                "main",
                "missing-doc",
                RawPointerEvent::new(PointerEventKind::Move, UiPoint::new(1.0, 1.0), 2),
            )
            .unwrap_err();
        assert_eq!(
            pointer_rejection,
            WindowRouteRejection::UnknownDocument {
                window_id: WindowId::new("main"),
                document_id: DocumentId::new("missing-doc"),
                kind: WindowRouteEventKind::Pointer
            }
        );
        assert_eq!(router.rejected().len(), 2);
        assert_eq!(router.take_rejected().len(), 2);
        assert!(router.rejected().is_empty());
    }

    #[test]
    fn accessibility_routing_updates_window_target_only() {
        let mut router = router_with_two_windows();
        let request =
            AccessibilityAdapterRequest::RestoreFocus(FocusRestoreTarget::Node(UiNodeId(9)));

        router
            .route_accessibility("tools", "tools-doc", request)
            .unwrap();

        assert_eq!(
            router
                .window_state(&WindowId::new("tools"))
                .unwrap()
                .accessibility_target,
            Some(DocumentId::new("tools-doc"))
        );
        assert_eq!(
            router
                .window_state(&WindowId::new("main"))
                .unwrap()
                .accessibility_target,
            None
        );
    }
}
