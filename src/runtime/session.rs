//! Shared document lifecycle for native, web, and application-owned hosts.
//!
//! Node indices are valid within one document only. A session reconciles state
//! using the sequence of node names from root to node. Sibling names must be
//! unique: ambiguous paths (including descendants of an ambiguous parent) never
//! inherit runtime state. Moving a node to a different parent starts a new
//! lifetime. Applications own text/editing models and may explicitly override
//! focus, scrolling, and animation inputs in each document description.

use crate::core::document::scroll_reveal::ScrollRevealState;
use crate::core::identity::{NodeIdentity, NodeIdentityIndex};
mod focus;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::accessibility::AccessibilityCapabilities;
use crate::host::{
    process_document_frame, process_document_input_with_filter, text_input_id_for_node,
    HostDocumentFrameOutput, HostDocumentFrameState, HostFrameOutput, HostInteractionState,
};
use crate::input::{GestureEvent, GesturePhase, PointerId, RawInputEvent};
use crate::layout_animation::LayoutAnimationOptions;
use crate::platform::{
    PlatformRequest, PlatformRequestIdAllocator, PlatformServiceRequest, PlatformServiceResponse,
    RepaintRequest, RepaintResponse, TextImeRequest,
};
use crate::renderer::{RenderOptions, RenderTarget};
use crate::{
    AnimationMachine, TextMeasurer, UiDocument, UiDocumentScale, UiFocusState, UiNodeId, UiPoint,
    UiSize, WidgetActionBinding, WidgetActionKind, WidgetActionMode, WidgetDragPhase,
    WidgetValueEditPhase,
};

/// Application cleanup for an edit whose owner disappeared or stopped accepting input.
///
/// The binding identifies the original application operation. `kind` is a
/// `PointerEdit` or `Drag` in its cancel phase, or a `TextEdit` composition cancel.
/// There is intentionally no node index: the original document may no longer
/// exist, and an index from it could refer to an unrelated node in the new view.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeInteractionCancellation {
    pub binding: WidgetActionBinding,
    pub kind: WidgetActionKind,
}

#[derive(Debug)]
struct ActiveWidgetEdit {
    capture: UiNodeId,
    owner: UiNodeId,
    mode: WidgetActionMode,
    cancellation: RuntimeInteractionCancellation,
}

#[derive(Debug, Default)]
struct NodeRuntimeState {
    scroll: Option<UiPoint>,
    scroll_reveal: Option<ScrollRevealState>,
    scrollbar_drag: Option<crate::core::document::AutoScrollbarDrag>,
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
    repaint: super::RuntimeRepaintScheduler,
    animations_active: bool,
    options: RuntimeSessionOptions,
    frame: HostDocumentFrameState,
    identities: Arc<NodeIdentityIndex>,
    retained: HashMap<NodeIdentity, NodeRuntimeState>,
    pending_requests: Vec<PlatformRequest>,
    document: Option<UiDocument>,
    views: super::views::ViewCache,
    document_viewport: Option<UiSize>,
    authored_node_count: usize,
    view_invalidated: bool,
    layout_refresh_requested: bool,
    widget_edits: HashMap<PointerId, ActiveWidgetEdit>,
    interaction_cancellations: Vec<RuntimeInteractionCancellation>,
    hook_input: super::integration::input::RuntimeHookInputState,
    text_input: super::ime::RuntimeTextInput,
    focus_lifecycle: focus::FocusLifecycle,
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

    /// Run frame preparation before building the application view. An unchanged
    /// hook never clears invalidation from input, tasks, or another hook.
    pub fn apply_before_render<State>(
        &mut self,
        hooks: &mut super::RuntimeHooks<State>,
        state: &mut State,
        metrics: super::RuntimeMetrics,
    ) {
        if let Some(hook) = &mut hooks.before_render {
            self.apply_hook_result(hook(state, metrics));
        }
    }

    /// Collect application platform work in hook order. The host executes these
    /// requests and delivers responses through `apply_platform_responses`.
    /// Sending a request and invalidating the view are independent decisions.
    pub fn take_platform_requests<State>(
        &mut self,
        hooks: &mut super::RuntimeHooks<State>,
        state: &mut State,
        metrics: super::RuntimeMetrics,
        ids: &mut PlatformRequestIdAllocator,
    ) -> Vec<PlatformServiceRequest> {
        let mut requests = Vec::new();
        if let Some(hook) = &mut hooks.platform_requests {
            requests.extend(ids.allocate_all(self.apply_hook_result(hook(state, metrics))));
        }
        if let Some(hook) = &mut hooks.platform_service_requests {
            requests.extend(self.apply_hook_result(hook(state, metrics)));
        }
        requests
    }

    /// Deliver nonempty platform response batches before the next view build.
    pub fn apply_platform_responses<State>(
        &mut self,
        hooks: &mut super::RuntimeHooks<State>,
        state: &mut State,
        responses: &[PlatformServiceResponse],
    ) {
        if !responses.is_empty() {
            if let Some(hook) = &mut hooks.platform_responses {
                self.apply_hook_result(hook(state, responses));
            }
        }
    }

    fn apply_hook_result<T>(&mut self, result: super::RuntimeHookResult<T>) -> T {
        if result.view_changed {
            self.invalidate_view();
        }
        result.value
    }

    pub fn interaction(&self) -> &HostInteractionState {
        &self.frame.interaction
    }

    /// Apply an explicit application IME request to composition routing state.
    /// Built-in hosts call this before executing the platform request. Automatic
    /// requests from `finish_frame` have already applied their state and retain
    /// runtime ownership; explicit sessions retain application-owned snapshots.
    pub fn apply_text_ime_request(&mut self, request: &TextImeRequest) {
        let state = &mut self.frame.interaction;
        match request {
            TextImeRequest::Activate(session) | TextImeRequest::Update(session) => {
                if state.text_ime.as_ref() == Some(session) {
                    return;
                }
                self.text_input.application_owned();
                let target = state.text_target.or(state.focused);
                if let Some(target) = target {
                    state.activate_text_ime_for(target, session.clone());
                } else {
                    state.activate_text_ime(session.clone());
                }
            }
            TextImeRequest::Deactivate { input } | TextImeRequest::HideKeyboard { input }
                if state
                    .text_ime
                    .as_ref()
                    .is_some_and(|session| session.input == *input) =>
            {
                self.text_input.application_owned();
                state.deactivate_text_ime(input.clone());
            }
            _ => {}
        }
    }

    /// The retained document, available after `retain_document` until the next build.
    pub fn document(&self) -> Option<&UiDocument> {
        self.document.as_ref()
    }

    /// Drain edit cancellations after preparing a document and before applying
    /// further input. Native and web runners deliver these through runtime hooks.
    pub fn take_interaction_cancellations(&mut self) -> Vec<RuntimeInteractionCancellation> {
        std::mem::take(&mut self.interaction_cancellations)
    }

    /// Start one host frame using elapsed time from a monotonic clock.
    /// Call once even if actions cause multiple document passes in this frame.
    pub fn begin_frame(&mut self, now: Duration) {
        self.repaint.advance_to(now);
        self.repaint.begin_frame();
    }

    /// Schedule presentation independently from invalidating the authored view.
    pub fn request_repaint(&mut self, now: Duration, request: RepaintRequest) -> RepaintResponse {
        self.repaint.advance_to(now);
        let response = match &request {
            RepaintRequest::Continuous { active: false } => RepaintResponse::Coalesced,
            RepaintRequest::After(delay) => RepaintResponse::Scheduled { delay: *delay },
            _ => RepaintResponse::Scheduled {
                delay: Duration::ZERO,
            },
        };
        self.repaint.request(request);
        response
    }

    /// Earliest required repaint. Hosts merge this with their application tick
    /// deadline and wake for input, resizing, and asynchronous service results.
    pub fn next_frame_delay(&mut self, now: Duration) -> Option<Duration> {
        self.repaint.advance_to(now);
        if let Some(delay) = self.frame_retry_delay(now) {
            Some(delay)
        } else if self.animations_active {
            Some(Duration::ZERO)
        } else {
            self.repaint.next_frame_delay()
        }
    }

    /// Minimum wait before another presentation attempt. Hosts must honor this
    /// before rendering, even for input, animation, application ticks or idle work.
    pub fn frame_retry_delay(&self, now: Duration) -> Option<Duration> {
        self.repaint.retry_delay(now)
    }

    /// Preserve uploads and back off after a failed presentation. Pass the
    /// monotonic time at failure, not frame start, so slow failures cannot spin.
    pub fn frame_failed(&mut self, now: Duration) {
        self.repaint.advance_to(now);
        self.repaint.finish_frame(false);
    }

    /// Finish input/layout work while presentation is deferred. This preserves
    /// uploads and repaint work without extending the surface retry deadline.
    pub fn frame_deferred(&mut self) {
        self.repaint.defer_frame();
    }

    /// Call after changing application state used by the view. Input-only
    /// redraws and animation ticks do not require rebuilding the description.
    pub fn invalidate_view(&mut self) {
        self.view_invalidated = true;
    }

    /// Apply a finite batch of completed application jobs on the host thread,
    /// before computing metrics or building the next document. The host's task
    /// waker schedules this work; no tick action or continuous redraw is needed.
    /// An empty batch leaves a retained view valid.
    pub fn apply_task_completions<State>(
        &mut self,
        hooks: &mut super::RuntimeHooks<State>,
        state: &mut State,
    ) -> usize {
        let count = hooks.apply_task_completions(state);
        if count > 0 {
            self.invalidate_view();
        }
        count
    }

    /// Whether an application callback changed the view since its last build.
    pub fn view_needs_rebuild(&self) -> bool {
        self.view_invalidated
    }

    pub fn view_build_stats(&self) -> super::ViewBuildStats {
        self.views.stats
    }

    /// Discard authored sections and text measurements after changing fonts or
    /// another external rendering dependency that is not represented in inputs.
    pub fn refresh_view(&mut self) {
        self.views.clear();
        self.layout_refresh_requested = true;
        self.invalidate_view();
    }

    /// Obtain the current document, rebuilding only after an application change
    /// or viewport resize. Return it with `retain_document` after the frame.
    pub fn build_document(
        &mut self,
        viewport: UiSize,
        scale: UiDocumentScale,
        cursor: Option<UiPoint>,
        measurer: &mut impl TextMeasurer,
        view: impl FnOnce(UiSize, &mut super::ViewContext<'_>) -> UiDocument,
    ) -> Result<UiDocument, taffy::TaffyError> {
        let mut previous = self.document.take();
        if let Some(document) = &mut previous {
            document.truncate_runtime_nodes(self.authored_node_count);
            if self.layout_refresh_requested {
                document.invalidate_layout();
            }
        }
        self.layout_refresh_requested = false;
        // Keep the submitted document intact for observation between frames.
        // Runtime overlays are regenerated when the next frame is prepared.
        let mut document = if !self.view_invalidated
            && self.document_viewport == Some(viewport)
            && previous
                .as_ref()
                .is_some_and(|document| document.scale() == scale)
        {
            self.views.stats = super::ViewBuildStats::default();
            previous.take().unwrap()
        } else {
            let mut document = self.views.build(viewport, scale, view);
            document.set_scale(scale);
            if let Some(previous) = &mut previous {
                document.inherit_frame_work(previous);
            }
            document
        };
        self.authored_node_count = document.node_count();
        self.view_invalidated = false;
        self.document_viewport = Some(viewport);
        self.prepare_document(&mut document, viewport, scale, cursor, measurer)?;
        Ok(document)
    }

    pub fn retain_document(&mut self, document: UiDocument) {
        self.document = Some(document);
    }

    /// Acknowledge successful rendering. Failed frames keep their resource
    /// uploads available for the next attempt.
    pub fn frame_presented(&mut self) {
        self.repaint.finish_frame(true);
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
        document.compact_resource_updates();
        let had_text_input = self.frame.interaction.text_ime.is_some();
        self.authored_node_count = document.node_count();
        let identities = document.identity_index().clone();
        let remap = |node| self.identities.remap(node, &identities);
        let mut cancelled_widget_targets = Vec::new();
        let cancellations = &mut self.interaction_cancellations;
        self.widget_edits.retain(|_, edit| {
            let capture = remap(edit.capture);
            if let (Some(capture), Some(owner)) = (capture, remap(edit.owner)) {
                edit.capture = capture;
                edit.owner = owner;
                true
            } else {
                cancelled_widget_targets.extend(capture);
                cancellations.push(edit.cancellation.clone());
                false
            }
        });
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
        state.canvas_host_capture.remap_targets(remap);

        // Published platform trees still use the old IDs and must be republished.
        // Internal history follows identity so unchanged live regions are not
        // re-announced and surviving nodes retain their animation origins.
        if !Arc::ptr_eq(&self.identities, &identities)
            && self.identities.by_node != identities.by_node
        {
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
        document.auto_scrollbar_drag = None;
        for (identity, runtime) in &self.retained {
            let id = self.identities.by_identity[identity];
            let node = &mut document.nodes[id.index()];
            if let Some(mut drag) = runtime.scrollbar_drag {
                drag.node = id;
                document.auto_scrollbar_drag = Some(drag);
            }
            if let (Some(scroll), Some(offset)) = (node.scroll.as_mut(), runtime.scroll) {
                if !scroll.offset_is_authored() {
                    scroll.set_host_offset(offset);
                }
            }
            if let (Some(animation), Some(previous)) =
                (node.animation.as_mut(), runtime.animation.as_ref())
            {
                if animation.has_same_definition(previous) {
                    if document.reused_view_nodes.contains(&id) {
                        *animation = previous.clone();
                    } else {
                        animation.retain_runtime_from(previous);
                    }
                }
            }
            if let Some(reveal) = runtime
                .scroll_reveal
                .as_ref()
                .and_then(|state| state.remap(&self.identities))
            {
                document.restore_scroll_reveal(id, reveal);
            }
        }
        let previous = UiFocusState {
            hovered: state.hovered,
            pressed: state.pressed,
            focused: state.focused,
        };
        let mut focus = previous.clone();
        let focus_authored = std::mem::take(&mut document.focus_authored);
        if focus_authored {
            focus.focused = document.focus.focused;
        }
        // Button state describes the latest queued event, not necessarily the
        // last processed one. A queued release still needs its press owner.
        document.set_runtime_focus_state(focus);
        document.compute_layout(viewport, measurer)?;
        let modal_scope = document.accessibility_modal_scope();
        let scrollbar_target = document.auto_scrollbar_drag.map(|drag| drag.node);
        document.sanitize_auto_scrollbar_drag();
        if document.auto_scrollbar_drag.is_none() {
            cancelled_widget_targets.extend(scrollbar_target);
        }
        let scrollbar_target = document.auto_scrollbar_drag.map(|drag| drag.node);
        let cancellations = &mut self.interaction_cancellations;
        self.widget_edits.retain(|_, edit| {
            let capture = document.node(edit.capture);
            let owner = document.node(edit.owner);
            let valid = capture.layout.visible
                && document.node_in_modal_scope(edit.capture, modal_scope)
                && document.node_in_modal_scope(edit.owner, modal_scope)
                && capture.input.pointer
                && capture.hit_test_behavior == crate::HitTestBehavior::Auto
                && document.node_is_enabled(edit.capture)
                && owner.layout.visible
                && owner.hit_test_behavior == crate::HitTestBehavior::Auto
                && document.node_is_enabled(edit.owner)
                && owner.action.as_ref() == Some(&edit.cancellation.binding)
                && owner.action_mode == edit.mode;
            if !valid {
                cancelled_widget_targets.push(edit.capture);
                cancellations.push(edit.cancellation.clone());
            }
            valid
        });
        let mut focus = document.focus.clone();
        focus.hovered = cursor.and_then(|point| {
            document
                .pointer_input_hit(point)
                .0
                .and_then(crate::HitTestResult::target)
        });
        focus.focused = focus.focused.filter(|id| document.node_accepts_focus(*id));
        focus.focused = self.focus_lifecycle.reconcile(
            document,
            &self.identities,
            &previous,
            focus.focused,
            focus_authored,
        );
        focus.pressed = focus.pressed.filter(|id| {
            let node = document.node(*id);
            node.layout.visible
                && document.node_in_modal_scope(*id, modal_scope)
                && (node.input.pointer || scrollbar_target == Some(*id))
                && node.hit_test_behavior == crate::HitTestBehavior::Auto
                && document.node_is_enabled(*id)
                && !cancelled_widget_targets.contains(id)
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
                && document.node_in_modal_scope(id, modal_scope)
                && (node.input.pointer || scrollbar_target == Some(id))
                && node.hit_test_behavior == crate::HitTestBehavior::Auto
                && document.node_is_enabled(id))
            .then_some(id)
            .filter(|id| !cancelled_widget_targets.contains(id))
        };
        state.drag_capture = state
            .drag_capture
            .filter(|capture| pointer_target(capture.target).is_some());
        state.gesture_tracker.remap_targets(pointer_target);
        state.canvas_host_capture.remap_targets(|id| {
            let node = document.node(id);
            (node.layout.visible
                && document.node_in_modal_scope(id, modal_scope)
                && node.hit_test_behavior == crate::HitTestBehavior::Auto
                && document.node_is_enabled(id))
            .then_some(id)
        });
        let previous_text_target = state.text_target;
        if state.text_target.is_some()
            && (state.text_target != state.focused
                || state.text_target.is_some_and(|target| {
                    !self.text_input.can_retain_for(document.node(target), state)
                }))
        {
            state.text_target = None;
            if let Some(ime) = state.text_ime.take() {
                self.pending_requests
                    .push(PlatformRequest::TextIme(TextImeRequest::Deactivate {
                        input: ime.input,
                    }));
            }
        }
        if had_text_input && state.text_ime.is_none() {
            state.text_composition = crate::host::HostTextCompositionState::Inactive;
            if let Some(cancellation) = self.text_input.cancellation() {
                let notified = previous_text_target.is_some_and(|target| {
                    self.focus_lifecycle.will_cancel_composition(
                        document,
                        &self.identities,
                        target,
                        &cancellation.binding,
                    )
                });
                if !notified {
                    self.interaction_cancellations.push(cancellation);
                }
            }
        }
        Ok(())
    }

    /// Apply input in order, updating document geometry before the next event.
    pub fn process_input(
        &self,
        document: &mut UiDocument,
        viewport: UiSize,
        input: Vec<RawInputEvent>,
        responses: Vec<PlatformServiceResponse>,
        measurer: &mut impl TextMeasurer,
    ) -> Result<HostFrameOutput, taffy::TaffyError> {
        let mut request = self.frame.host_frame_request(viewport);
        request.raw_input = input;
        request.platform_responses = responses;
        process_document_input_with_filter(
            document,
            measurer,
            request,
            (16.0, viewport),
            |_, _, _| true,
        )
    }

    /// Deliver owner-removal cancellations after preparing a document. Hooks may
    /// mutate application state, so hosts must honor view invalidation afterward.
    pub fn reconcile_input_hooks<State>(
        &mut self,
        document: &UiDocument,
        hooks: &mut super::integration::RuntimeHooks<State>,
        state: &mut State,
    ) {
        let mut invoked = false;
        for cancellation in self.take_interaction_cancellations() {
            if let Some(hook) = hooks.interaction_cancelled.as_mut() {
                hook(state, cancellation);
                invoked = true;
            }
        }
        invoked |= self.hook_input.reconcile(document, hooks, state);
        if invoked {
            self.invalidate_view();
        }
    }

    /// Route normalized input through application hooks and ordinary widgets in
    /// event order, preserving canvas ownership across document rebuilds.
    pub fn process_input_with_hooks<State>(
        &mut self,
        document: &mut UiDocument,
        viewport: UiSize,
        input: &[RawInputEvent],
        responses: &[PlatformServiceResponse],
        hooks: &mut super::integration::RuntimeHooks<State>,
        state: &mut State,
        measurer: &mut impl TextMeasurer,
    ) -> Result<HostFrameOutput, taffy::TaffyError> {
        self.reconcile_input_hooks(document, hooks, state);
        let mut request = self.frame.host_frame_request(viewport);
        request.raw_input = input.to_vec();
        request.platform_responses = responses.to_vec();
        let (output, invoked) = self
            .hook_input
            .process(document, measurer, request, hooks, state);
        if invoked {
            self.invalidate_view();
        }
        output
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
        let focus_events = self.focus_lifecycle.take_events(document, &self.identities);
        input.events.splice(0..0, focus_events);
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
        let mut frame = process_document_frame(document, measurer, request)?;
        self.frame.apply_document_frame_output(&frame);
        let ime_requests = self.text_input.sync(&mut self.frame.interaction, document);
        frame.host_output.state = self.frame.interaction.clone();
        frame
            .host_output
            .platform_requests
            .extend(request_ids.allocate_all(ime_requests));
        self.animations_active = document.animations_active();
        self.capture(document);
        self.capture_widget_edits(document, &frame);
        Ok(frame)
    }

    fn capture_widget_edits(&mut self, document: &UiDocument, frame: &HostDocumentFrameOutput) {
        for (event, input) in frame.input_events() {
            if input.is_some_and(|input| input.scrollbar_target.is_some()) {
                continue;
            }
            let Some(gesture) = &event.gesture else {
                continue;
            };
            let GestureEvent::Drag(drag) = gesture else {
                continue;
            };
            if matches!(drag.phase, GesturePhase::Commit | GesturePhase::Cancel) {
                self.widget_edits.remove(&drag.pointer_id);
                continue;
            }
            if !matches!(drag.phase, GesturePhase::Begin | GesturePhase::Update) {
                continue;
            }
            let Some(action) = event
                .document_result
                .as_ref()
                .and_then(|result| result.gesture_action())
                .cloned()
            else {
                continue;
            };
            let kind = match action.kind {
                WidgetActionKind::PointerEdit(mut edit) => {
                    edit.phase = WidgetValueEditPhase::Cancel;
                    WidgetActionKind::PointerEdit(edit)
                }
                WidgetActionKind::Drag(mut edit) => {
                    edit.phase = WidgetDragPhase::Cancel;
                    edit.previous = edit.current;
                    edit.delta = UiPoint::new(0.0, 0.0);
                    WidgetActionKind::Drag(edit)
                }
                _ => continue,
            };
            self.widget_edits.insert(
                drag.pointer_id,
                ActiveWidgetEdit {
                    capture: drag.target,
                    owner: action.target,
                    mode: document.node(action.target).action_mode,
                    cancellation: RuntimeInteractionCancellation {
                        binding: action.binding,
                        kind,
                    },
                },
            );
        }
    }

    fn capture(&mut self, document: &UiDocument) {
        self.identities = document.identity_index().clone();
        self.retained.clear();
        for (key, id) in &self.identities.by_identity {
            let node = document.node(*id);
            if node.scroll.is_some() || node.animation.is_some() {
                self.retained.insert(
                    key.clone(),
                    NodeRuntimeState {
                        scroll: node.scroll.as_ref().map(|scroll| scroll.offset),
                        scroll_reveal: document.scroll_reveal_state(*id).cloned(),
                        scrollbar_drag: document
                            .auto_scrollbar_drag
                            .filter(|drag| drag.node == *id),
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
