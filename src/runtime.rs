//! Backend-neutral runtime and frame scheduling contracts.
//!
//! This module models the reusable host-loop pieces Operad needs without
//! depending on a concrete windowing crate. Native hosts can translate their
//! events into these contracts, while tests can drive the same lifecycle
//! deterministically.

use std::time::Duration;

use crate::input::RawInputEvent;
use crate::platform::{
    PlatformRequest, PlatformRequestId, PlatformRequestIdAllocator, PlatformResponse,
    PlatformServiceRequest, PlatformServiceResponse, RepaintRequest,
};
use crate::{DirtyFlags, UiSize};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeWindowId(pub String);

impl RuntimeWindowId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeSurfaceId(pub String);

impl RuntimeSurfaceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeFramePhase {
    CollectPlatformEvents,
    ConvertInput,
    ProcessHostFrame,
    BuildDocumentFrame,
    Layout,
    BuildPaint,
    Render,
    Present,
    ServicePlatformRequests,
    Idle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePhaseTrace {
    phases: Vec<RuntimeFramePhase>,
}

impl RuntimePhaseTrace {
    pub fn new() -> Self {
        Self { phases: Vec::new() }
    }

    pub fn push(&mut self, phase: RuntimeFramePhase) {
        if self.phases.last() != Some(&phase) {
            self.phases.push(phase);
        }
    }

    pub fn phases(&self) -> &[RuntimeFramePhase] {
        &self.phases
    }
}

impl Default for RuntimePhaseTrace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeInvalidationReason {
    Input,
    Resize,
    ScaleFactor,
    Animation,
    AsyncTask,
    PlatformResponse,
    Accessibility,
    Resource,
    Explicit,
}

impl RuntimeInvalidationReason {
    pub const fn dirty_flags(self) -> DirtyFlags {
        match self {
            Self::Input => DirtyFlags {
                input: true,
                layout: false,
                paint: true,
                theme: false,
                text_measurement: false,
            },
            Self::Resize | Self::ScaleFactor => DirtyFlags {
                input: false,
                layout: true,
                paint: true,
                theme: false,
                text_measurement: true,
            },
            Self::Animation => DirtyFlags {
                input: false,
                layout: false,
                paint: true,
                theme: false,
                text_measurement: false,
            },
            Self::AsyncTask | Self::PlatformResponse | Self::Explicit => DirtyFlags {
                input: false,
                layout: true,
                paint: true,
                theme: false,
                text_measurement: false,
            },
            Self::Accessibility => DirtyFlags {
                input: true,
                layout: false,
                paint: false,
                theme: false,
                text_measurement: false,
            },
            Self::Resource => DirtyFlags {
                input: false,
                layout: false,
                paint: true,
                theme: false,
                text_measurement: false,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInvalidation {
    pub reason: RuntimeInvalidationReason,
    pub detail: Option<String>,
}

impl RuntimeInvalidation {
    pub const fn new(reason: RuntimeInvalidationReason) -> Self {
        Self {
            reason,
            detail: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeFrameClock {
    pub frame_index: u64,
    pub elapsed: Duration,
}

impl RuntimeFrameClock {
    pub const fn new(frame_index: u64, elapsed: Duration) -> Self {
        Self {
            frame_index,
            elapsed,
        }
    }

    pub const fn next(self, delta: Duration) -> Self {
        Self {
            frame_index: self.frame_index.wrapping_add(1),
            elapsed: self.elapsed.saturating_add(delta),
        }
    }
}

impl Default for RuntimeFrameClock {
    fn default() -> Self {
        Self::new(0, Duration::ZERO)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeLoopGuard {
    pub max_frames_without_idle: u32,
}

impl RuntimeLoopGuard {
    pub const fn new(max_frames_without_idle: u32) -> Self {
        Self {
            max_frames_without_idle,
        }
    }
}

impl Default for RuntimeLoopGuard {
    fn default() -> Self {
        Self::new(120)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeRepaintScheduler {
    next_frame: bool,
    delay: Option<Duration>,
    continuous: bool,
    dirty_flags: DirtyFlags,
    invalidations: Vec<RuntimeInvalidation>,
    frames_without_idle: u32,
    guard: RuntimeLoopGuard,
}

impl RuntimeRepaintScheduler {
    pub fn new(guard: RuntimeLoopGuard) -> Self {
        Self {
            next_frame: false,
            delay: None,
            continuous: false,
            dirty_flags: DirtyFlags::NONE,
            invalidations: Vec::new(),
            frames_without_idle: 0,
            guard,
        }
    }

    pub fn request(&mut self, request: RepaintRequest) {
        match request {
            RepaintRequest::NextFrame => self.next_frame = true,
            RepaintRequest::After(delay) => {
                self.delay = Some(self.delay.map_or(delay, |current| current.min(delay)));
            }
            RepaintRequest::Area(_) => self.next_frame = true,
            RepaintRequest::Continuous { active } => {
                self.continuous = active;
                if active {
                    self.next_frame = true;
                }
            }
        }
    }

    pub fn invalidate(&mut self, invalidation: RuntimeInvalidation) {
        self.dirty_flags = self.dirty_flags.union(invalidation.reason.dirty_flags());
        self.invalidations.push(invalidation);
        self.next_frame = true;
    }

    pub const fn dirty_flags(&self) -> DirtyFlags {
        self.dirty_flags
    }

    pub fn invalidations(&self) -> &[RuntimeInvalidation] {
        &self.invalidations
    }

    pub const fn continuous(&self) -> bool {
        self.continuous
    }

    pub const fn delay(&self) -> Option<Duration> {
        self.delay
    }

    pub const fn frame_due(&self) -> bool {
        self.next_frame || self.continuous
    }

    pub fn finish_frame(&mut self, rendered: bool) {
        if rendered {
            self.next_frame = self.continuous;
            self.delay = None;
            self.dirty_flags = DirtyFlags::NONE;
            self.invalidations.clear();
            self.frames_without_idle = self.frames_without_idle.saturating_add(1);
        } else {
            self.frames_without_idle = 0;
        }
    }

    pub fn mark_idle(&mut self) {
        self.frames_without_idle = 0;
    }

    pub const fn tripped_guard(&self) -> bool {
        self.frames_without_idle > self.guard.max_frames_without_idle
    }
}

impl Default for RuntimeRepaintScheduler {
    fn default() -> Self {
        Self::new(RuntimeLoopGuard::default())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeWindowEvent {
    Resized {
        window: RuntimeWindowId,
        surface: RuntimeSurfaceId,
        size: UiSize,
    },
    ScaleFactorChanged {
        window: RuntimeWindowId,
        scale_factor: f32,
        size: UiSize,
    },
    Focused {
        window: RuntimeWindowId,
        focused: bool,
    },
    CloseRequested {
        window: RuntimeWindowId,
    },
    RawInput(RawInputEvent),
    PlatformResponse(PlatformServiceResponse),
    RequestRepaint(RepaintRequest),
    Invalidate(RuntimeInvalidation),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeFramePlan {
    pub clock: RuntimeFrameClock,
    pub viewport: UiSize,
    pub raw_input: Vec<RawInputEvent>,
    pub platform_responses: Vec<PlatformServiceResponse>,
    pub platform_requests: Vec<PlatformServiceRequest>,
    pub dirty_flags: DirtyFlags,
    pub invalidations: Vec<RuntimeInvalidation>,
    pub trace: RuntimePhaseTrace,
    pub should_render: bool,
    pub loop_guard_tripped: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeLoopState {
    pub window: RuntimeWindowId,
    pub surface: RuntimeSurfaceId,
    pub viewport: UiSize,
    pub scale_factor: f32,
    pub focused: bool,
    pub close_requested: bool,
    clock: RuntimeFrameClock,
    pending_input: Vec<RawInputEvent>,
    pending_platform_responses: Vec<PlatformServiceResponse>,
    pending_platform_requests: Vec<PlatformServiceRequest>,
    repaint: RuntimeRepaintScheduler,
    request_ids: PlatformRequestIdAllocator,
}

impl RuntimeLoopState {
    pub fn new(window: RuntimeWindowId, surface: RuntimeSurfaceId, viewport: UiSize) -> Self {
        Self {
            window,
            surface,
            viewport,
            scale_factor: 1.0,
            focused: true,
            close_requested: false,
            clock: RuntimeFrameClock::default(),
            pending_input: Vec::new(),
            pending_platform_responses: Vec::new(),
            pending_platform_requests: Vec::new(),
            repaint: RuntimeRepaintScheduler::default(),
            request_ids: PlatformRequestIdAllocator::default(),
        }
    }

    pub fn with_loop_guard(mut self, guard: RuntimeLoopGuard) -> Self {
        self.repaint = RuntimeRepaintScheduler::new(guard);
        self
    }

    pub const fn clock(&self) -> RuntimeFrameClock {
        self.clock
    }

    pub const fn repaint_scheduler(&self) -> &RuntimeRepaintScheduler {
        &self.repaint
    }

    pub fn push_platform_request(&mut self, request: PlatformRequest) -> PlatformRequestId {
        let service = self.request_ids.allocate(request);
        let id = service.id;
        if matches!(service.request, PlatformRequest::Repaint(_)) {
            if let PlatformRequest::Repaint(request) = &service.request {
                self.repaint.request(request.clone());
            }
        }
        self.pending_platform_requests.push(service);
        id
    }

    pub fn push_platform_requests(
        &mut self,
        requests: impl IntoIterator<Item = PlatformRequest>,
    ) -> Vec<PlatformRequestId> {
        requests
            .into_iter()
            .map(|request| self.push_platform_request(request))
            .collect()
    }

    pub fn handle_event(&mut self, event: RuntimeWindowEvent) {
        match event {
            RuntimeWindowEvent::Resized {
                window,
                surface,
                size,
            } => {
                if window == self.window && surface == self.surface && self.viewport != size {
                    self.viewport = size;
                    self.repaint
                        .invalidate(RuntimeInvalidation::new(RuntimeInvalidationReason::Resize));
                }
            }
            RuntimeWindowEvent::ScaleFactorChanged {
                window,
                scale_factor,
                size,
            } => {
                if window == self.window {
                    self.scale_factor = scale_factor.max(0.0);
                    self.viewport = size;
                    self.repaint.invalidate(RuntimeInvalidation::new(
                        RuntimeInvalidationReason::ScaleFactor,
                    ));
                }
            }
            RuntimeWindowEvent::Focused { window, focused } => {
                if window == self.window {
                    self.focused = focused;
                    self.repaint
                        .invalidate(RuntimeInvalidation::new(RuntimeInvalidationReason::Input));
                }
            }
            RuntimeWindowEvent::CloseRequested { window } => {
                if window == self.window {
                    self.close_requested = true;
                }
            }
            RuntimeWindowEvent::RawInput(input) => {
                self.pending_input.push(input);
                self.repaint
                    .invalidate(RuntimeInvalidation::new(RuntimeInvalidationReason::Input));
            }
            RuntimeWindowEvent::PlatformResponse(response) => {
                self.pending_platform_responses.push(response);
                self.repaint.invalidate(RuntimeInvalidation::new(
                    RuntimeInvalidationReason::PlatformResponse,
                ));
            }
            RuntimeWindowEvent::RequestRepaint(request) => {
                self.repaint.request(request);
            }
            RuntimeWindowEvent::Invalidate(invalidation) => {
                self.repaint.invalidate(invalidation);
            }
        }
    }

    pub fn next_frame_plan(&mut self, delta: Duration) -> RuntimeFramePlan {
        self.clock = self.clock.next(delta);
        let should_render = self.repaint.frame_due();
        let mut trace = RuntimePhaseTrace::new();
        trace.push(RuntimeFramePhase::CollectPlatformEvents);
        if !self.pending_input.is_empty() {
            trace.push(RuntimeFramePhase::ConvertInput);
        }
        if should_render {
            trace.push(RuntimeFramePhase::ProcessHostFrame);
            trace.push(RuntimeFramePhase::BuildDocumentFrame);
            trace.push(RuntimeFramePhase::Layout);
            trace.push(RuntimeFramePhase::BuildPaint);
            trace.push(RuntimeFramePhase::Render);
            trace.push(RuntimeFramePhase::Present);
        }
        if !self.pending_platform_requests.is_empty() || !self.pending_platform_responses.is_empty()
        {
            trace.push(RuntimeFramePhase::ServicePlatformRequests);
        }
        if !should_render {
            trace.push(RuntimeFramePhase::Idle);
            self.repaint.mark_idle();
        }

        let plan = RuntimeFramePlan {
            clock: self.clock,
            viewport: self.viewport,
            raw_input: std::mem::take(&mut self.pending_input),
            platform_responses: std::mem::take(&mut self.pending_platform_responses),
            platform_requests: std::mem::take(&mut self.pending_platform_requests),
            dirty_flags: self.repaint.dirty_flags(),
            invalidations: self.repaint.invalidations().to_vec(),
            trace,
            should_render,
            loop_guard_tripped: self.repaint.tripped_guard(),
        };
        self.repaint.finish_frame(should_render);
        plan
    }
}

pub fn coalesce_repaint_requests(
    requests: impl IntoIterator<Item = RepaintRequest>,
) -> Option<RepaintRequest> {
    let mut next_frame = false;
    let mut delay: Option<Duration> = None;
    let mut continuous = None;

    for request in requests {
        match request {
            RepaintRequest::NextFrame | RepaintRequest::Area(_) => next_frame = true,
            RepaintRequest::After(next_delay) => {
                delay = Some(delay.map_or(next_delay, |current| current.min(next_delay)));
            }
            RepaintRequest::Continuous { active } => continuous = Some(active),
        }
    }

    if let Some(active) = continuous {
        Some(RepaintRequest::Continuous { active })
    } else if next_frame {
        Some(RepaintRequest::NextFrame)
    } else {
        delay.map(RepaintRequest::After)
    }
}

pub fn collect_repaint_requests<'a>(
    requests: impl IntoIterator<Item = &'a PlatformServiceRequest>,
) -> Vec<RepaintRequest> {
    requests
        .into_iter()
        .filter_map(|request| match &request.request {
            PlatformRequest::Repaint(repaint) => Some(repaint.clone()),
            _ => None,
        })
        .collect()
}

pub fn completed_platform_response(
    id: PlatformRequestId,
    response: PlatformResponse,
) -> PlatformServiceResponse {
    PlatformServiceResponse::new(id, response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{PointerEventKind, RawPointerEvent};
    use crate::platform::RepaintResponse;
    use crate::{UiPoint, UiRect};

    fn runtime() -> RuntimeLoopState {
        RuntimeLoopState::new(
            RuntimeWindowId::new("main"),
            RuntimeSurfaceId::new("surface"),
            UiSize::new(640.0, 480.0),
        )
    }

    #[test]
    fn raw_input_schedules_frame_and_preserves_phase_order() {
        let mut runtime = runtime();
        runtime.handle_event(RuntimeWindowEvent::RawInput(RawInputEvent::Pointer(
            RawPointerEvent::new(PointerEventKind::Move, UiPoint::new(12.0, 14.0), 1),
        )));

        let plan = runtime.next_frame_plan(Duration::from_millis(16));
        assert!(plan.should_render);
        assert_eq!(plan.raw_input.len(), 1);
        assert!(plan.dirty_flags.input);
        assert!(plan.dirty_flags.paint);
        assert_eq!(
            plan.trace.phases(),
            &[
                RuntimeFramePhase::CollectPlatformEvents,
                RuntimeFramePhase::ConvertInput,
                RuntimeFramePhase::ProcessHostFrame,
                RuntimeFramePhase::BuildDocumentFrame,
                RuntimeFramePhase::Layout,
                RuntimeFramePhase::BuildPaint,
                RuntimeFramePhase::Render,
                RuntimeFramePhase::Present,
            ]
        );
    }

    #[test]
    fn repaint_requests_coalesce_to_strongest_schedule() {
        assert_eq!(
            coalesce_repaint_requests([
                RepaintRequest::After(Duration::from_millis(40)),
                RepaintRequest::After(Duration::from_millis(16)),
            ]),
            Some(RepaintRequest::After(Duration::from_millis(16)))
        );
        assert_eq!(
            coalesce_repaint_requests([
                RepaintRequest::After(Duration::from_millis(16)),
                RepaintRequest::NextFrame,
            ]),
            Some(RepaintRequest::NextFrame)
        );
        assert_eq!(
            coalesce_repaint_requests([
                RepaintRequest::NextFrame,
                RepaintRequest::Continuous { active: true },
            ]),
            Some(RepaintRequest::Continuous { active: true })
        );
    }

    #[test]
    fn resize_updates_viewport_and_layout_dirty_flags() {
        let mut runtime = runtime();
        runtime.handle_event(RuntimeWindowEvent::Resized {
            window: RuntimeWindowId::new("main"),
            surface: RuntimeSurfaceId::new("surface"),
            size: UiSize::new(800.0, 600.0),
        });

        let plan = runtime.next_frame_plan(Duration::from_millis(1));
        assert_eq!(plan.viewport, UiSize::new(800.0, 600.0));
        assert!(plan.dirty_flags.layout);
        assert!(plan.dirty_flags.paint);
        assert!(plan.dirty_flags.text_measurement);
    }

    #[test]
    fn platform_requests_are_collected_and_repaint_is_scheduled() {
        let mut runtime = runtime();
        let id = runtime.push_platform_request(PlatformRequest::Repaint(RepaintRequest::Area(
            crate::platform::LogicalRect::new(0.0, 0.0, 10.0, 10.0),
        )));
        runtime.handle_event(RuntimeWindowEvent::PlatformResponse(
            completed_platform_response(
                id,
                PlatformResponse::Repaint(RepaintResponse::Scheduled {
                    delay: Duration::ZERO,
                }),
            ),
        ));

        let plan = runtime.next_frame_plan(Duration::from_millis(8));
        assert!(plan.should_render);
        assert_eq!(plan.platform_requests.len(), 1);
        assert_eq!(collect_repaint_requests(&plan.platform_requests).len(), 1);
        assert_eq!(plan.platform_responses.len(), 1);
        assert!(plan
            .trace
            .phases()
            .contains(&RuntimeFramePhase::ServicePlatformRequests));
    }

    #[test]
    fn headless_tick_goes_idle_when_no_work_is_pending() {
        let mut runtime = runtime();
        let plan = runtime.next_frame_plan(Duration::from_millis(16));
        assert!(!plan.should_render);
        assert_eq!(plan.raw_input, Vec::new());
        assert_eq!(
            plan.trace.phases(),
            &[
                RuntimeFramePhase::CollectPlatformEvents,
                RuntimeFramePhase::Idle,
            ]
        );
    }

    #[test]
    fn continuous_repaint_trips_loop_guard() {
        let mut runtime = runtime().with_loop_guard(RuntimeLoopGuard::new(2));
        runtime.handle_event(RuntimeWindowEvent::RequestRepaint(
            RepaintRequest::Continuous { active: true },
        ));

        assert!(
            !runtime
                .next_frame_plan(Duration::from_millis(1))
                .loop_guard_tripped
        );
        assert!(
            !runtime
                .next_frame_plan(Duration::from_millis(1))
                .loop_guard_tripped
        );
        assert!(
            !runtime
                .next_frame_plan(Duration::from_millis(1))
                .loop_guard_tripped
        );
        assert!(
            runtime
                .next_frame_plan(Duration::from_millis(1))
                .loop_guard_tripped
        );
    }

    #[test]
    fn explicit_invalidations_union_dirty_flags_and_keep_details() {
        let mut runtime = runtime();
        runtime.handle_event(RuntimeWindowEvent::Invalidate(
            RuntimeInvalidation::new(RuntimeInvalidationReason::Resource).detail("atlas"),
        ));
        runtime.handle_event(RuntimeWindowEvent::Invalidate(RuntimeInvalidation::new(
            RuntimeInvalidationReason::Accessibility,
        )));

        let plan = runtime.next_frame_plan(Duration::ZERO);
        assert!(plan.dirty_flags.paint);
        assert!(plan.dirty_flags.input);
        assert_eq!(plan.invalidations.len(), 2);
        assert_eq!(plan.invalidations[0].detail.as_deref(), Some("atlas"));
    }

    #[test]
    fn ignored_window_events_do_not_schedule_work() {
        let mut runtime = runtime();
        runtime.handle_event(RuntimeWindowEvent::Resized {
            window: RuntimeWindowId::new("other"),
            surface: RuntimeSurfaceId::new("surface"),
            size: UiSize::new(800.0, 600.0),
        });
        runtime.handle_event(RuntimeWindowEvent::RequestRepaint(RepaintRequest::After(
            Duration::from_millis(30),
        )));

        let plan = runtime.next_frame_plan(Duration::ZERO);
        assert!(!plan.should_render);
        assert_eq!(
            runtime.repaint_scheduler().delay(),
            Some(Duration::from_millis(30))
        );
    }

    #[test]
    fn area_repaint_uses_next_frame_render_semantics() {
        let mut scheduler = RuntimeRepaintScheduler::default();
        scheduler.request(RepaintRequest::Area(crate::platform::LogicalRect::new(
            1.0, 2.0, 3.0, 4.0,
        )));
        assert!(scheduler.frame_due());

        let mut trace = RuntimePhaseTrace::new();
        trace.push(RuntimeFramePhase::Render);
        trace.push(RuntimeFramePhase::Render);
        assert_eq!(trace.phases(), &[RuntimeFramePhase::Render]);

        let _ = UiRect::new(0.0, 0.0, 1.0, 1.0);
    }
}
