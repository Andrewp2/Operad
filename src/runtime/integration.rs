//! Application callbacks and observations shared by native and browser hosts.

pub(crate) mod input;

use std::time::Duration;

use crate::host::HostDocumentFrameOutput;
use crate::input::{RawInputEvent, RawKeyboardEvent, RawPointerEvent};
use crate::platform::{
    PixelSize, PlatformRequest, PlatformServiceRequest, PlatformServiceResponse,
};
use crate::renderer::CanvasHostCaptureId;
use crate::{PaintList, UiDocument, UiNode, UiNodeId, UiPoint, UiRect, UiSize, WidgetAction};

use super::session::RuntimeInteractionCancellation;

/// Dimensions and elapsed host time for one frame on either platform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeMetrics {
    pub physical_size: PixelSize,
    pub viewport: UiSize,
    pub scale_factor: f32,
    pub dpi_scale: f32,
    /// Monotonic elapsed time since this runtime started, not wall-clock time.
    pub elapsed: Duration,
}

/// Normalized input for an application-owned canvas or one of its descendants.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasInput {
    /// Current document-local node. None when an invalidated owner is cancelled.
    pub node: Option<UiNodeId>,
    pub key: String,
    pub rect: UiRect,
    /// Coordinates within the canvas, after undoing its effective paint transform.
    pub local_position: Option<UiPoint>,
    pub input: RawInputEvent,
}

/// A keyboard event and its focus context at this point in the input sequence.
///
/// Earlier pointer or navigation events in the same batch are already reflected
/// in `focused`. Inspect its action or text-input metadata to decide whether an
/// application shortcut should yield to the control. The node is borrowed only
/// for this callback; application state need not retain a document-local ID.
#[derive(Debug, Clone)]
pub struct KeyboardInput<'a> {
    pub event: RawKeyboardEvent,
    pub focused: Option<&'a UiNode>,
}

/// A pointer event and its current document ownership, before widget dispatch.
/// Nodes with action bindings resolve to the same logical owner as widget
/// actions. The references are valid only during the observer callback.
#[derive(Debug, Clone, Copy)]
pub struct PointerInput<'a> {
    pub event: RawPointerEvent,
    /// Enabled hit target at the pointer, respecting blocking and modal scope.
    pub hit: Option<&'a UiNode>,
    /// Owner of this pointer's active widget or canvas capture, even outside
    /// its bounds. A press establishes capture after this observation.
    pub captured: Option<&'a UiNode>,
}

#[derive(Debug, Clone)]
pub struct RawMouseMotion {
    pub delta: (f64, f64),
    pub timestamp_millis: u64,
    pub captured_canvas: Option<CanvasHostCaptureId>,
}

/// A read-only view of the final computed frame, after application actions and
/// any resulting rebuild, immediately before rendering submission.
///
/// Observing does not call the application's view or perform layout again.
/// Submission can still fail after this callback; this is not a presentation
/// acknowledgement. Borrowed node IDs are valid only for this document.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeObservation<'a> {
    pub metrics: RuntimeMetrics,
    pub document: &'a UiDocument,
    pub frame: &'a HostDocumentFrameOutput,
    pub view_build_stats: super::ViewBuildStats,
}

impl<'a> RuntimeObservation<'a> {
    /// Application-owned hosts can observe the same frame they submit.
    pub fn new(
        metrics: RuntimeMetrics,
        document: &'a UiDocument,
        frame: &'a HostDocumentFrameOutput,
        view_build_stats: super::ViewBuildStats,
    ) -> Self {
        Self {
            metrics,
            document,
            frame,
            view_build_stats,
        }
    }

    pub fn paint(&self) -> &'a PaintList {
        &self.frame.render_request.paint
    }
}

/// A frame hook's output and whether it changed the application description.
///
/// Plain callback outputs convert to `changed`, preserving conservative view
/// invalidation. Return `unchanged` only when every value read by the view is
/// unchanged. Draining a request queue or updating a cursor need not change the
/// view; playback, asynchronous edits, and status updates often do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeHookResult<T = ()> {
    pub value: T,
    pub view_changed: bool,
}

impl<T> RuntimeHookResult<T> {
    pub fn changed(value: T) -> Self {
        Self {
            value,
            view_changed: true,
        }
    }

    pub fn unchanged(value: T) -> Self {
        Self {
            value,
            view_changed: false,
        }
    }
}

impl<T> From<T> for RuntimeHookResult<T> {
    fn from(value: T) -> Self {
        Self::changed(value)
    }
}

/// The application integration surface on both native and web.
///
/// Input hooks run in input order before ordinary widget dispatch. Returning
/// true consumes that event. Canvas pointer ownership persists across document
/// rebuilds and ends on release, cancellation, removal, or disabling.
/// Mutating hooks conservatively invalidate the view; observers borrow state
/// immutably and never invalidate or rebuild the document. Frame and platform
/// hooks can explicitly report unchanged view inputs with [`RuntimeHookResult`].
pub struct RuntimeHooks<State> {
    task_completions: Vec<Box<dyn super::tasks::CompletionSource<State>>>,
    task_waker: Option<super::tasks::TaskWaker>,
    pub(crate) title: Option<Box<dyn Fn(&State) -> String>>,
    pub(crate) scale_factor: Option<Box<dyn Fn(&State, RuntimeMetrics) -> f32>>,
    pub(crate) close_requested: Option<Box<dyn FnMut(&mut State) -> bool>>,
    pub(crate) keyboard_input:
        Option<Box<dyn for<'a> FnMut(&mut State, KeyboardInput<'a>) -> bool>>,
    pub(crate) pointer_observer: Option<Box<dyn for<'a> FnMut(&mut State, PointerInput<'a>)>>,
    pub(crate) raw_mouse_motion: Option<Box<dyn FnMut(&mut State, RawMouseMotion) -> bool>>,
    pub(crate) canvas_input: Option<Box<dyn FnMut(&mut State, CanvasInput) -> bool>>,
    pub(crate) platform_requests: Option<
        Box<dyn FnMut(&mut State, RuntimeMetrics) -> RuntimeHookResult<Vec<PlatformRequest>>>,
    >,
    pub(crate) platform_service_requests: Option<
        Box<
            dyn FnMut(&mut State, RuntimeMetrics) -> RuntimeHookResult<Vec<PlatformServiceRequest>>,
        >,
    >,
    pub(crate) platform_responses:
        Option<Box<dyn FnMut(&mut State, &[PlatformServiceResponse]) -> RuntimeHookResult>>,
    pub(crate) before_render:
        Option<Box<dyn FnMut(&mut State, RuntimeMetrics) -> RuntimeHookResult>>,
    pub(crate) idle_redraw: Option<Box<dyn Fn(&State) -> bool>>,
    pub(crate) frame_observer: Option<Box<dyn for<'a> FnMut(&State, RuntimeObservation<'a>)>>,
    pub(crate) interaction_cancelled:
        Option<Box<dyn FnMut(&mut State, RuntimeInteractionCancellation)>>,
}

impl<State> Default for RuntimeHooks<State> {
    fn default() -> Self {
        Self {
            task_completions: Vec::new(),
            task_waker: None,
            title: None,
            scale_factor: None,
            close_requested: None,
            keyboard_input: None,
            pointer_observer: None,
            raw_mouse_motion: None,
            canvas_input: None,
            platform_requests: None,
            platform_service_requests: None,
            platform_responses: None,
            before_render: None,
            idle_redraw: None,
            frame_observer: None,
            interaction_cancelled: None,
        }
    }
}

impl<State> RuntimeHooks<State> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply messages from application-owned jobs on the UI thread. Each nonempty batch
    /// invalidates the view. Multiple channels are allowed; FIFO is guaranteed
    /// within a channel, not between channels. Messages queued during delivery
    /// run in a later batch, keeping input and rendering responsive.
    ///
    /// Native and web runners install the wakeup and drain before building the
    /// view. Application-owned hosts use `set_task_waker` and
    /// `RuntimeSession::apply_task_completions` at the same boundary.
    pub fn with_task_completions<Message: 'static>(
        mut self,
        mut receiver: super::TaskReceiver<Message>,
        handler: impl FnMut(&mut State, Message) + 'static,
    ) -> Self
    where
        State: 'static,
    {
        if let Some(wake) = &self.task_waker {
            receiver.set_waker(wake.clone());
        }
        self.task_completions
            .push(Box::new(super::tasks::CompletionHandler {
                receiver,
                handler: std::rc::Rc::new(std::cell::RefCell::new(handler)),
                marker: std::marker::PhantomData,
            }));
        self
    }

    /// Install the host wakeup. It may run on a producer thread and must only
    /// schedule host work; it must not access application state or drain hooks.
    /// Queued startup results wake the newly installed host immediately.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_task_waker(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        self.install_task_waker(std::sync::Arc::new(wake));
    }

    /// Install a browser-thread wakeup. Queued startup results wake immediately.
    #[cfg(target_arch = "wasm32")]
    pub fn set_task_waker(&mut self, wake: impl Fn() + 'static) {
        self.install_task_waker(std::rc::Rc::new(wake));
    }

    fn install_task_waker(&mut self, wake: super::tasks::TaskWaker) {
        self.task_waker = Some(wake.clone());
        for source in &mut self.task_completions {
            source.set_waker(wake.clone());
        }
    }

    pub(crate) fn apply_task_completions(&mut self, state: &mut State) -> usize {
        // Snapshot every channel before invoking any handler: one channel cannot
        // extend this batch by enqueueing into another channel's callback.
        let batches: Vec<_> = self
            .task_completions
            .iter_mut()
            .filter_map(|source| source.take_batch())
            .collect();
        batches.into_iter().map(|apply| apply(state)).sum()
    }

    pub fn with_title(mut self, hook: impl Fn(&State) -> String + 'static) -> Self {
        self.title = Some(Box::new(hook));
        self
    }
    pub fn with_scale_factor(
        mut self,
        hook: impl Fn(&State, RuntimeMetrics) -> f32 + 'static,
    ) -> Self {
        self.scale_factor = Some(Box::new(hook));
        self
    }
    pub fn with_close_requested(mut self, hook: impl FnMut(&mut State) -> bool + 'static) -> Self {
        self.close_requested = Some(Box::new(hook));
        self
    }
    /// Observe a key, its generated text, and the current focused control.
    /// Returning true consumes the key and generated text; independent text and
    /// IME composition remain separate. Focus includes earlier events in this batch.
    pub fn with_keyboard_input(
        mut self,
        hook: impl for<'a> FnMut(&mut State, KeyboardInput<'a>) -> bool + 'static,
    ) -> Self {
        self.keyboard_input = Some(Box::new(hook));
        self
    }
    pub fn with_raw_mouse_motion(
        mut self,
        hook: impl FnMut(&mut State, RawMouseMotion) -> bool + 'static,
    ) -> Self {
        self.raw_mouse_motion = Some(Box::new(hook));
        self
    }

    /// Update hover previews or application cursor state using authoritative
    /// hit/capture ownership. This observes every raw pointer event in order;
    /// it cannot consume input or replace runtime capture. Use widget actions
    /// for clicks and drags, or `with_canvas_input` for canvas interception.
    pub fn with_pointer_observer(
        mut self,
        observer: impl for<'a> FnMut(&mut State, PointerInput<'a>) + 'static,
    ) -> Self {
        self.pointer_observer = Some(Box::new(observer));
        self
    }
    pub fn with_canvas_input(
        mut self,
        hook: impl FnMut(&mut State, CanvasInput) -> bool + 'static,
    ) -> Self {
        self.canvas_input = Some(Box::new(hook));
        self
    }
    /// Return plain requests to invalidate conservatively, or a
    /// [`RuntimeHookResult`] to report whether the view changed.
    pub fn with_platform_requests<R: Into<RuntimeHookResult<Vec<PlatformRequest>>>>(
        mut self,
        mut hook: impl FnMut(&mut State, RuntimeMetrics) -> R + 'static,
    ) -> Self {
        self.platform_requests = Some(Box::new(move |state, metrics| hook(state, metrics).into()));
        self
    }
    /// Like [`Self::with_platform_requests`], with application-assigned IDs.
    pub fn with_platform_service_requests<
        R: Into<RuntimeHookResult<Vec<PlatformServiceRequest>>>,
    >(
        mut self,
        mut hook: impl FnMut(&mut State, RuntimeMetrics) -> R + 'static,
    ) -> Self {
        self.platform_service_requests =
            Some(Box::new(move |state, metrics| hook(state, metrics).into()));
        self
    }
    /// Returning `()` invalidates conservatively, even when a response is only
    /// an acknowledgement. Return [`RuntimeHookResult::unchanged`] to retain it.
    pub fn with_platform_responses<R: Into<RuntimeHookResult>>(
        mut self,
        mut hook: impl FnMut(&mut State, &[PlatformServiceResponse]) -> R + 'static,
    ) -> Self {
        self.platform_responses = Some(Box::new(move |state, responses| {
            hook(state, responses).into()
        }));
        self
    }
    /// Run before view construction on each frame. Returning `()` invalidates
    /// conservatively. An explicit unchanged result retains the document while
    /// still allowing input, animation, layout refreshes, and painting to run.
    pub fn with_before_render<R: Into<RuntimeHookResult>>(
        mut self,
        mut hook: impl FnMut(&mut State, RuntimeMetrics) -> R + 'static,
    ) -> Self {
        self.before_render = Some(Box::new(move |state, metrics| hook(state, metrics).into()));
        self
    }
    pub fn with_idle_redraw(mut self, hook: impl Fn(&State) -> bool + 'static) -> Self {
        self.idle_redraw = Some(Box::new(hook));
        self
    }
    pub fn with_frame_observer(
        mut self,
        hook: impl for<'a> FnMut(&State, RuntimeObservation<'a>) + 'static,
    ) -> Self {
        self.frame_observer = Some(Box::new(hook));
        self
    }
    pub fn with_interaction_cancelled(
        mut self,
        hook: impl FnMut(&mut State, RuntimeInteractionCancellation) + 'static,
    ) -> Self {
        self.interaction_cancelled = Some(Box::new(hook));
        self
    }

    /// Invoke the observer from an application-owned host after final layout,
    /// immediately before submitting the frame. This performs no runtime work.
    pub fn observe(&mut self, state: &State, observation: RuntimeObservation<'_>) {
        if let Some(observer) = self.frame_observer.as_mut() {
            observer(state, observation);
        }
    }
}

/// Define state, update, view and hooks once, then select a platform host.
/// Platform options configure the window or canvas; application logic remains
/// the same on either host.
// A portable application may be defined in a crate without enabling a host.
#[cfg_attr(
    not(any(
        feature = "native-window",
        all(feature = "web-runtime", target_arch = "wasm32")
    )),
    allow(dead_code)
)]
pub struct Application<State> {
    state: State,
    update: Box<dyn FnMut(&mut State, WidgetAction)>,
    view: Box<dyn FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument>,
    hooks: RuntimeHooks<State>,
}

impl<State: 'static> Application<State> {
    pub fn new(
        state: State,
        update: impl FnMut(&mut State, WidgetAction) + 'static,
        view: impl FnMut(&State, UiSize, &mut super::ViewContext<'_>) -> UiDocument + 'static,
    ) -> Self {
        Self {
            state,
            update: Box::new(update),
            view: Box::new(view),
            hooks: RuntimeHooks::new(),
        }
    }

    pub fn with_hooks(mut self, hooks: RuntimeHooks<State>) -> Self {
        self.hooks = hooks;
        self
    }

    #[cfg(feature = "native-window")]
    pub fn run_native(
        self,
        options: super::native::NativeWindowOptions,
    ) -> super::native::NativeWindowResult {
        self.run_native_with_canvas_renderers(
            options,
            super::native::NativeWgpuCanvasRenderRegistry::new(),
        )
    }

    #[cfg(feature = "native-window")]
    pub fn run_native_with_canvas_renderers(
        self,
        options: super::native::NativeWindowOptions,
        renderers: super::native::NativeWgpuCanvasRenderRegistry<State>,
    ) -> super::native::NativeWindowResult {
        super::native::run_app_with_canvas_renderers_and_hooks(
            options,
            self.state,
            self.update,
            self.view,
            renderers,
            self.hooks,
        )
    }

    #[cfg(all(feature = "web-runtime", target_arch = "wasm32"))]
    pub async fn run_web(
        self,
        options: super::web::WebRuntimeOptions,
    ) -> Result<(), wasm_bindgen::JsValue> {
        super::web::run_app_with_hooks(options, self.state, self.update, self.view, self.hooks)
            .await
    }
}
