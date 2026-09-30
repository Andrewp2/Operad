//! Ordered application interception and stable canvas pointer ownership.

use std::collections::HashMap;

use crate::core::identity::NodeIdentity;
use crate::host::{
    process_document_input_with_filter, HostFrameOutput, HostFrameRequest, HostInteractionState,
};
use crate::input::{PointerButton, PointerEventKind, PointerId, RawInputEvent};
#[cfg(any(
    feature = "native-window",
    all(feature = "web-runtime", target_arch = "wasm32")
))]
use crate::renderer::CanvasHostCaptureId;
use crate::{
    CanvasContent, CanvasInteractionPolicy, HitTestBehavior, HitTestResult, TextMeasurer,
    UiContent, UiDocument, UiNodeId, UiPoint, UiRect,
};

use super::{CanvasInput, KeyboardInput, PointerInput, RuntimeHooks};

#[derive(Debug)]
struct CanvasPointerCapture {
    identity: Option<NodeIdentity>,
    node: UiNodeId,
    key: String,
    button: PointerButton,
    last_input: CanvasInput,
}

#[derive(Debug, Default)]
pub(crate) struct RuntimeHookInputState {
    captures: HashMap<PointerId, CanvasPointerCapture>,
}

impl RuntimeHookInputState {
    pub(crate) fn reconcile<State>(
        &mut self,
        document: &UiDocument,
        hooks: &mut RuntimeHooks<State>,
        state: &mut State,
    ) -> bool {
        if self.captures.is_empty() {
            return false;
        }
        let modal_scope = document.accessibility_modal_scope();
        let identities = document.identity_index();
        let mut invoked = false;
        self.captures.retain(|_, capture| {
            let node = capture
                .identity
                .as_ref()
                .and_then(|identity| identities.by_identity.get(identity).copied());
            if let Some(node) = node.filter(|&node| {
                canvas_target(document, node, modal_scope).is_some_and(|(owner, canvas, _)| {
                    owner == node
                        && canvas.key == capture.key
                        && (canvas.interaction.pointer_capture || canvas.interaction.pointer_lock)
                })
            }) {
                capture.node = node;
                return true;
            }
            if let Some(hook) = hooks.canvas_input.as_mut() {
                let mut input = capture.last_input.clone();
                input.node = None;
                if let RawInputEvent::Pointer(pointer) = &mut input.input {
                    pointer.kind = PointerEventKind::Cancel;
                    pointer.buttons = crate::input::PointerButtons::NONE;
                }
                hook(state, input);
                invoked = true;
            }
            false
        });
        invoked
    }

    pub(crate) fn process<State>(
        &mut self,
        document: &mut UiDocument,
        measurer: &mut impl TextMeasurer,
        request: HostFrameRequest,
        hooks: &mut RuntimeHooks<State>,
        state: &mut State,
    ) -> (Result<HostFrameOutput, taffy::TaffyError>, bool) {
        let mut invoked = false;
        let wheel_scale = (16.0, request.viewport);
        let output = process_document_input_with_filter(
            document,
            measurer,
            request,
            wheel_scale,
            |document, event, interaction| {
                if let RawInputEvent::Pointer(pointer) = event {
                    if let Some(observer) = hooks.pointer_observer.as_mut() {
                        invoked = true;
                        let hit = document
                            .pointer_input_hit(pointer.position)
                            .0
                            .and_then(HitTestResult::target);
                        let captured = self
                            .captures
                            .get(&pointer.pointer_id)
                            .filter(|_| hooks.canvas_input.is_some())
                            .and_then(|capture| {
                                canvas_target(
                                    document,
                                    capture.node,
                                    document.accessibility_modal_scope(),
                                )
                            })
                            .map(|(node, _, _)| node)
                            .or_else(|| {
                                interaction
                                    .drag_capture
                                    .filter(|capture| capture.pointer_id == pointer.pointer_id)
                                    .map(|capture| capture.target)
                            });
                        observer(
                            state,
                            PointerInput {
                                event: *pointer,
                                hit: hit.and_then(|id| pointer_owner(document, id)),
                                captured: captured.and_then(|id| pointer_owner(document, id)),
                            },
                        );
                    }
                }
                if let RawInputEvent::Keyboard(key) = event {
                    if let Some(hook) = hooks.keyboard_input.as_mut() {
                        invoked = true;
                        let focused = document
                            .focus_state()
                            .focused
                            .and_then(|id| document.nodes().get(id.index()));
                        if hook(
                            state,
                            KeyboardInput {
                                event: key.clone(),
                                focused,
                            },
                        ) {
                            return false;
                        }
                    }
                }
                let Some(hook) = hooks.canvas_input.as_mut() else {
                    self.captures.clear();
                    return true;
                };
                let modal_scope = document.accessibility_modal_scope();
                let canvas_input = match event {
                    RawInputEvent::Pointer(pointer) => self
                        .captures
                        .get(&pointer.pointer_id)
                        .and_then(|capture| canvas_target(document, capture.node, modal_scope))
                        .map(|(node, canvas, rect)| {
                            canvas_input(
                                document,
                                node,
                                canvas,
                                rect,
                                Some(pointer.position),
                                event.clone(),
                            )
                        })
                        .or_else(|| {
                            (pointer.kind != PointerEventKind::Cancel)
                                .then(|| {
                                    canvas_input_for_raw_event_in_scope(
                                        document,
                                        interaction,
                                        event,
                                        modal_scope,
                                    )
                                })
                                .flatten()
                        }),
                    _ => canvas_input_for_raw_event_in_scope(
                        document,
                        interaction,
                        event,
                        modal_scope,
                    ),
                };
                let Some(canvas_input) = canvas_input else {
                    return true;
                };
                if let RawInputEvent::Pointer(pointer) = event {
                    match pointer.kind {
                        PointerEventKind::Down(button) => {
                            if let Some(node) = canvas_input.node {
                                self.captures.entry(pointer.pointer_id).or_insert_with(|| {
                                    CanvasPointerCapture {
                                        identity: document
                                            .identity_index()
                                            .by_node
                                            .get(node.index())
                                            .cloned()
                                            .flatten(),
                                        node,
                                        key: canvas_input.key.clone(),
                                        button,
                                        last_input: canvas_input.clone(),
                                    }
                                });
                            }
                        }
                        PointerEventKind::Move => {
                            if let Some(capture) = self.captures.get_mut(&pointer.pointer_id) {
                                capture.last_input = canvas_input.clone();
                            }
                        }
                        PointerEventKind::Up(button) => {
                            if self
                                .captures
                                .get(&pointer.pointer_id)
                                .is_some_and(|capture| capture.button == button)
                            {
                                self.captures.remove(&pointer.pointer_id);
                            }
                        }
                        PointerEventKind::Cancel => {
                            self.captures.remove(&pointer.pointer_id);
                        }
                    }
                }
                invoked = true;
                !hook(state, canvas_input)
            },
        );
        (output, invoked)
    }
}

fn pointer_owner(document: &UiDocument, target: UiNodeId) -> Option<&crate::UiNode> {
    if !document.node_is_enabled(target)
        || !document.node_in_modal_scope(target, document.accessibility_modal_scope())
    {
        return None;
    }
    let owner =
        crate::actions::resolve_action_target(document, target, |id| document.node(id).action())
            .map_or(target, |(id, _, _)| id);
    document.nodes().get(owner.index())
}

#[cfg(test)]
pub(crate) fn canvas_input_for_raw_event(
    document: &UiDocument,
    state: &HostInteractionState,
    event: &RawInputEvent,
) -> Option<CanvasInput> {
    canvas_input_for_raw_event_in_scope(
        document,
        state,
        event,
        document.accessibility_modal_scope(),
    )
}

fn canvas_input_for_raw_event_in_scope(
    document: &UiDocument,
    state: &HostInteractionState,
    event: &RawInputEvent,
    modal_scope: Option<UiNodeId>,
) -> Option<CanvasInput> {
    match event {
        RawInputEvent::Pointer(pointer) => {
            if document.auto_scrollbar_drag.is_some_and(|drag| {
                state.drag_capture.is_some_and(|capture| {
                    capture.pointer_id == pointer.pointer_id && capture.target == drag.node
                })
            }) || document
                .auto_scrollbar_hit_target(pointer.position)
                .is_some()
            {
                return None;
            }
            let target = crate::host::document_input_target(event, state, document)?;
            let (node, canvas, rect) = canvas_target(document, target, modal_scope)?;
            (canvas.interaction.pointer_capture || canvas.interaction.pointer_lock).then(|| {
                canvas_input(
                    document,
                    node,
                    canvas,
                    rect,
                    Some(pointer.position),
                    event.clone(),
                )
            })
        }
        RawInputEvent::Wheel(wheel) => {
            if matches!(
                document.hit_test_result(wheel.position),
                Some(HitTestResult::Blocked(_))
            ) {
                return None;
            }
            crate::host::document_input_target(event, state, document)
                .and_then(|target| canvas_target(document, target, modal_scope))
                .filter(|(_, canvas, _)| canvas.interaction.wheel_capture)
                .or_else(|| {
                    active_canvas_capture(document, state, modal_scope, |policy| {
                        policy.wheel_capture
                    })
                })
                .map(|(node, canvas, rect)| {
                    canvas_input(
                        document,
                        node,
                        canvas,
                        rect,
                        Some(wheel.position),
                        event.clone(),
                    )
                })
        }
        RawInputEvent::Keyboard(_) | RawInputEvent::Text(_) | RawInputEvent::Composition(_) => {
            state
                .focused
                .and_then(|focused| canvas_target(document, focused, modal_scope))
                .filter(|(_, canvas, _)| canvas.interaction.keyboard_capture)
                .or_else(|| {
                    active_canvas_capture(document, state, modal_scope, |policy| {
                        policy.keyboard_capture
                    })
                })
                .map(|(node, canvas, rect)| {
                    canvas_input(document, node, canvas, rect, None, event.clone())
                })
        }
        RawInputEvent::Focus(_) => None,
    }
}

fn canvas_target(
    document: &UiDocument,
    target: UiNodeId,
    modal_scope: Option<UiNodeId>,
) -> Option<(UiNodeId, &CanvasContent, UiRect)> {
    let mut current = Some(target);
    while let Some(id) = current {
        let node = document.nodes().get(id.0)?;
        if !node.layout.visible
            || !document.node_is_enabled(id)
            || !document.node_in_modal_scope(id, modal_scope)
        {
            return None;
        }
        if let UiContent::Canvas(canvas) = &node.content {
            return (node.hit_test_behavior() == HitTestBehavior::Auto).then_some((
                id,
                canvas,
                node.layout.rect,
            ));
        }
        current = node.parent;
    }
    None
}

fn active_canvas_capture<'a>(
    document: &'a UiDocument,
    state: &HostInteractionState,
    modal_scope: Option<UiNodeId>,
    accepts: impl Fn(CanvasInteractionPolicy) -> bool,
) -> Option<(UiNodeId, &'a CanvasContent, UiRect)> {
    state
        .canvas_host_capture
        .active_plans()
        .iter()
        .find_map(|plan| {
            // A retained plan identifies the owner, not its current policy.
            // Changing content must neither preserve revoked input channels nor
            // transfer this ownership to an ancestor with the same surface key.
            canvas_target(document, plan.node, modal_scope).filter(|(owner, canvas, _)| {
                *owner == plan.node && canvas.key == plan.key && accepts(canvas.interaction)
            })
        })
}

#[cfg(any(
    feature = "native-window",
    all(feature = "web-runtime", target_arch = "wasm32")
))]
pub(crate) fn captured_raw_mouse_canvas(
    state: &HostInteractionState,
) -> Option<CanvasHostCaptureId> {
    state
        .canvas_host_capture
        .active_plans()
        .iter()
        .find(|plan| plan.pointer_lock)
        .map(CanvasHostCaptureId::from_plan)
}

fn canvas_input(
    document: &UiDocument,
    node: UiNodeId,
    canvas: &CanvasContent,
    rect: UiRect,
    position: Option<UiPoint>,
    input: RawInputEvent,
) -> CanvasInput {
    let local_position = position.and_then(|position| {
        document
            .node_effective_transform(node)
            .inverse_transform_point(position)
            .map(|position| UiPoint::new(position.x - rect.x, position.y - rect.y))
    });
    CanvasInput {
        node: Some(node),
        key: canvas.key.clone(),
        rect,
        local_position,
        input,
    }
}

#[cfg(test)]
mod tests;
