//! Shared document lifecycle for native, web, and application-owned hosts.
//!
//! Node indices are valid within one document only. A session reconciles state
//! using the sequence of node names from root to node. Sibling names must be
//! unique: ambiguous paths (including descendants of an ambiguous parent) never
//! inherit runtime state. Moving a node to a different parent starts a new
//! lifetime. Applications own text/editing models and may explicitly override
//! focus, scrolling, and animation inputs in each document description.

use crate::core::identity::{NodeIdentity, NodeIdentityIndex};
use std::collections::HashMap;

use crate::accessibility::AccessibilityCapabilities;
use crate::host::{
    process_document_frame, process_host_frame_input_with_target_resolver, text_input_id_for_node,
    HostDocumentFrameOutput, HostDocumentFrameState, HostFrameOutput, HostInteractionState,
};
use crate::input::{PointerEventKind, RawInputEvent};
use crate::layout_animation::LayoutAnimationOptions;
use crate::platform::{
    PlatformRequest, PlatformRequestIdAllocator, PlatformServiceResponse, TextImeRequest,
};
use crate::renderer::{RenderOptions, RenderTarget};
use crate::{
    AnimationMachine, TextMeasurer, UiDocument, UiDocumentScale, UiFocusState, UiNodeId, UiPoint,
    UiSize,
};

#[derive(Debug, Default)]
struct NodeRuntimeState {
    scroll: Option<UiPoint>,
    animation: Option<AnimationMachine>,
}

/// Host capabilities and frame policies, independent of the windowing backend.
#[derive(Debug, Clone, Default)]
pub struct RuntimeSessionOptions {
    pub accessibility_capabilities: AccessibilityCapabilities,
    pub layout_animation: Option<LayoutAnimationOptions>,
    /// Accessibility preferences here govern both rendering and host output.
    pub render: RenderOptions,
}

/// One window's persistent UI lifecycle. Keep one session per independent UI.
#[derive(Debug, Default)]
pub struct RuntimeSession {
    options: RuntimeSessionOptions,
    frame: HostDocumentFrameState,
    identities: NodeIdentityIndex,
    retained: HashMap<NodeIdentity, NodeRuntimeState>,
    pending_requests: Vec<PlatformRequest>,
    document: Option<UiDocument>,
    document_viewport: Option<UiSize>,
    authored_node_count: usize,
    view_invalidated: bool,
}

impl RuntimeSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_options(options: RuntimeSessionOptions) -> Self {
        Self {
            options,
            ..Self::default()
        }
    }

    pub fn set_options(&mut self, options: RuntimeSessionOptions) {
        self.options = options;
        self.invalidate_view();
    }

    pub fn interaction(&self) -> &HostInteractionState {
        &self.frame.interaction
    }

    /// Call after changing application state used by the view. Input-only
    /// redraws and animation ticks do not require rebuilding the description.
    pub fn invalidate_view(&mut self) {
        self.view_invalidated = true;
    }

    /// Obtain the current document, rebuilding only after an application change
    /// or viewport resize. Return it with `retain_document` after the frame.
    pub fn build_document(
        &mut self,
        viewport: UiSize,
        scale: UiDocumentScale,
        cursor: Option<UiPoint>,
        measurer: &mut impl TextMeasurer,
        view: impl FnOnce(UiSize) -> UiDocument,
    ) -> Result<UiDocument, taffy::TaffyError> {
        let cached = if !self.view_invalidated && self.document_viewport == Some(viewport) {
            self.document.take()
        } else {
            self.document = None;
            None
        };
        let mut document = cached.unwrap_or_else(|| view(viewport));
        self.authored_node_count = document.node_count();
        self.view_invalidated = false;
        self.document_viewport = Some(viewport);
        self.prepare_document(&mut document, viewport, scale, cursor, measurer)?;
        Ok(document)
    }

    pub fn retain_document(&mut self, mut document: UiDocument) {
        document.truncate_runtime_nodes(self.authored_node_count);
        self.document = Some(document);
    }

    /// Acknowledge successful rendering. Failed frames keep their resource
    /// uploads available for the next attempt.
    pub fn frame_presented(&mut self) {
        if let Some(document) = &mut self.document {
            document.clear_resource_updates();
        }
    }

    /// Restore runtime state into a newly authored document before routing input.
    pub fn prepare_document(
        &mut self,
        document: &mut UiDocument,
        viewport: UiSize,
        scale: UiDocumentScale,
        cursor: Option<UiPoint>,
        measurer: &mut impl TextMeasurer,
    ) -> Result<(), taffy::TaffyError> {
        self.authored_node_count = document.node_count();
        let identities = NodeIdentityIndex::from_document(document);
        let remap = |node| self.identities.remap(node, &identities);
        let state = &mut self.frame.interaction;
        state.hovered = state.hovered.and_then(remap);
        state.pressed = state.pressed.and_then(remap);
        state.focused = state.focused.and_then(remap);
        state.drag_capture = state.drag_capture.and_then(|mut capture| {
            capture.target = remap(capture.target)?;
            Some(capture)
        });
        state.gesture_tracker.remap_targets(remap);
        let old_text_target = state.text_target;
        state.text_target = state.text_target.and_then(remap);
        if let Some(mut ime) = state.text_ime.take() {
            if let Some(target) = state.text_target {
                let input = text_input_id_for_node(target);
                if old_text_target.is_some_and(|old| ime.input == text_input_id_for_node(old))
                    && ime.input != input
                {
                    self.pending_requests.push(PlatformRequest::TextIme(
                        TextImeRequest::Deactivate {
                            input: ime.input.clone(),
                        },
                    ));
                    ime.input = input;
                    self.pending_requests
                        .push(PlatformRequest::TextIme(TextImeRequest::Activate(
                            ime.clone(),
                        )));
                }
                state.text_ime = Some(ime);
            } else {
                self.pending_requests
                    .push(PlatformRequest::TextIme(TextImeRequest::Deactivate {
                        input: ime.input,
                    }));
            }
        }
        state.wheel_target = state.wheel_target.and_then(remap);
        state.input_consumed_by = state.input_consumed_by.and_then(remap);
        state.input_consumed &= state.input_consumed_by.is_some();
        if let Some(route) = &mut state.shortcut_route {
            route.target = route.target.and_then(remap);
        }
        self.pending_requests
            .extend(state.canvas_host_capture.remap_targets(remap));

        // Published platform trees still use the old IDs and must be republished.
        // Internal history follows identity so unchanged live regions are not
        // re-announced and surviving nodes retain their animation origins.
        if self.identities.by_node != identities.by_node {
            self.frame.layout = self
                .frame
                .layout
                .take()
                .and_then(|layout| remap_layout(layout, &remap));
            self.frame.accessibility.tree = None;
            if let Some(regions) = &mut self.frame.accessibility.live_regions {
                regions.entries.retain_mut(|entry| {
                    if let Some(node) = remap(entry.node) {
                        entry.node = node;
                        true
                    } else {
                        false
                    }
                });
            }
        }
        self.frame.accessibility.focused = self
            .frame
            .accessibility
            .focused
            .map(|id| id.and_then(remap));
        self.identities = identities;
        self.retained
            .retain(|key, _| self.identities.by_identity.contains_key(key));

        document.set_scale(scale);
        document.set_pointer_position(cursor);
        for (identity, runtime) in &self.retained {
            let id = self.identities.by_identity[identity];
            let node = &mut document.nodes[id.index()];
            if let (Some(scroll), Some(offset)) = (node.scroll.as_mut(), runtime.scroll) {
                if !scroll.offset_is_authored() {
                    scroll.set_host_offset(offset);
                }
            }
            if let (Some(animation), Some(previous)) =
                (node.animation.as_mut(), runtime.animation.as_ref())
            {
                if animation.has_same_definition(previous) {
                    animation.retain_runtime_from(previous);
                }
            }
        }
        let previous = UiFocusState {
            hovered: state.hovered,
            pressed: state.pressed,
            focused: state.focused,
        };
        let mut focus = previous.clone();
        if std::mem::take(&mut document.focus_authored) {
            focus.focused = document.focus.focused;
        }
        // Button state describes the latest queued event, not necessarily the
        // last processed one. A queued release still needs its press owner.
        document.set_runtime_focus_state(focus);
        document.compute_layout(viewport, measurer)?;
        let mut focus = document.focus.clone();
        focus.hovered = cursor.and_then(|point| document.hit_test(point));
        focus.focused = focus.focused.filter(|id| {
            let node = document.node(*id);
            node.layout.visible
                && node.input.focusable
                && node.accessibility.as_ref().is_none_or(|meta| meta.enabled)
        });
        focus.pressed = focus.pressed.filter(|id| {
            let node = document.node(*id);
            node.layout.visible
                && node.input.pointer
                && node.accessibility.as_ref().is_none_or(|meta| meta.enabled)
        });
        document.set_runtime_focus_state(focus);
        document.refresh_interaction_animation_inputs(previous, cursor);
        // Interaction styles can change text metrics. Hit testing for queued
        // input must see the resulting geometry, even on a cached document.
        document.compute_layout(viewport, measurer)?;
        state.hovered = document.focus.hovered;
        state.pressed = document.focus.pressed;
        state.focused = document.focus.focused;
        let pointer_target = |id: UiNodeId| {
            let node = document.node(id);
            (node.layout.visible
                && node.input.pointer
                && node.accessibility.as_ref().is_none_or(|meta| meta.enabled))
            .then_some(id)
        };
        state.drag_capture = state
            .drag_capture
            .filter(|capture| pointer_target(capture.target).is_some());
        state.gesture_tracker.remap_targets(pointer_target);
        self.pending_requests.extend(
            state
                .canvas_host_capture
                .remap_targets(|id| document.node(id).layout.visible.then_some(id)),
        );
        if state.text_target.is_some() && state.text_target != state.focused {
            state.text_target = None;
            if let Some(ime) = state.text_ime.take() {
                self.pending_requests
                    .push(PlatformRequest::TextIme(TextImeRequest::Deactivate {
                        input: ime.input,
                    }));
            }
        }
        Ok(())
    }

    pub fn process_input(
        &self,
        document: &UiDocument,
        viewport: UiSize,
        input: Vec<RawInputEvent>,
        responses: Vec<PlatformServiceResponse>,
    ) -> HostFrameOutput {
        let mut request = self.frame.host_frame_request(viewport);
        request.raw_input = input;
        request.platform_responses = responses;
        process_host_frame_input_with_target_resolver(request, |event, state| {
            resolve_target(event, state, document)
        })
    }

    /// Layout, paint, and capture the authoritative state of a processed frame.
    pub fn finish_frame(
        &mut self,
        document: &mut UiDocument,
        viewport: UiSize,
        target: RenderTarget,
        mut input: HostFrameOutput,
        measurer: &mut impl TextMeasurer,
        request_ids: &mut PlatformRequestIdAllocator,
    ) -> Result<HostDocumentFrameOutput, taffy::TaffyError> {
        input
            .platform_requests
            .extend(request_ids.allocate_all(self.pending_requests.drain(..)));
        let mut request = self
            .frame
            .document_frame_request(viewport, target, input)
            .accessibility_capabilities(self.options.accessibility_capabilities)
            .accessibility_preferences(self.options.render.accessibility_preferences)
            .render_options(self.options.render);
        if let Some(options) = self.options.layout_animation {
            request = request.layout_animation_options(options);
        }
        let frame = process_document_frame(document, measurer, request)?;
        self.frame.apply_document_frame_output(&frame);
        self.capture(document);
        Ok(frame)
    }

    fn capture(&mut self, document: &UiDocument) {
        self.identities = NodeIdentityIndex::from_document(document);
        self.retained.clear();
        for (key, id) in &self.identities.by_identity {
            let node = document.node(*id);
            if node.scroll.is_some() || node.animation.is_some() {
                self.retained.insert(
                    key.clone(),
                    NodeRuntimeState {
                        scroll: node.scroll.as_ref().map(|scroll| scroll.offset),
                        animation: node.animation.clone(),
                    },
                );
            }
        }
    }
}

fn remap_layout(
    mut layout: crate::LayoutSnapshot,
    remap: &impl Fn(UiNodeId) -> Option<UiNodeId>,
) -> Option<crate::LayoutSnapshot> {
    layout.id = remap(layout.id)?;
    layout.children = layout
        .children
        .into_iter()
        .filter_map(|child| remap_layout(child, remap))
        .collect();
    Some(layout)
}

#[cfg(test)]
mod tests;

pub(crate) fn resolve_target(
    event: &RawInputEvent,
    state: &HostInteractionState,
    document: &UiDocument,
) -> Option<UiNodeId> {
    match event {
        RawInputEvent::Pointer(pointer) => state
            .drag_capture
            .filter(|capture| {
                capture.pointer_id == pointer.pointer_id
                    && matches!(
                        pointer.kind,
                        PointerEventKind::Move | PointerEventKind::Up(_) | PointerEventKind::Cancel
                    )
            })
            .map(|capture| capture.target)
            .or_else(|| document.hit_test(pointer.position)),
        RawInputEvent::Wheel(wheel) => document.hit_test(wheel.position),
        RawInputEvent::Keyboard(_) | RawInputEvent::Text(_) | RawInputEvent::Focus(_) => None,
    }
}
