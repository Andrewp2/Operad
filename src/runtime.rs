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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeTimerId(pub u64);

impl RuntimeTimerId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeIdleWorkId(pub u64);

impl RuntimeIdleWorkId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTimerDeadline {
    pub id: RuntimeTimerId,
    pub deadline: Duration,
    pub invalidation: RuntimeInvalidation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTimerCompletion {
    pub id: RuntimeTimerId,
    pub deadline: Duration,
    pub fired_at: Duration,
    pub invalidation: RuntimeInvalidation,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTimerCancellation {
    pub id: RuntimeTimerId,
    pub deadline: Option<Duration>,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeIdleBudgetRequest {
    pub id: RuntimeIdleWorkId,
    pub min_budget: Duration,
    pub requested_at: Duration,
    pub detail: Option<String>,
}

impl RuntimeIdleBudgetRequest {
    pub fn new(
        id: RuntimeIdleWorkId,
        min_budget: Duration,
        requested_at: Duration,
        detail: Option<String>,
    ) -> Self {
        Self {
            id,
            min_budget,
            requested_at,
            detail,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeIdleCompletion {
    pub id: RuntimeIdleWorkId,
    pub requested_at: Option<Duration>,
    pub budget_used: Duration,
    pub completed_at: Duration,
    pub invalidation: Option<RuntimeInvalidation>,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeIdleCancellation {
    pub id: RuntimeIdleWorkId,
    pub requested_at: Option<Duration>,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeTimerScheduler {
    next_id: u64,
    deadlines: Vec<RuntimeTimerDeadline>,
}

impl RuntimeTimerScheduler {
    fn new() -> Self {
        Self {
            next_id: 1,
            deadlines: Vec::new(),
        }
    }

    fn schedule(
        &mut self,
        now: Duration,
        delay: Duration,
        invalidation: RuntimeInvalidation,
    ) -> RuntimeTimerDeadline {
        let deadline = RuntimeTimerDeadline {
            id: RuntimeTimerId::new(self.next_id),
            deadline: now.saturating_add(delay),
            invalidation,
        };
        self.next_id = self.next_id.wrapping_add(1);
        self.deadlines.push(deadline.clone());
        self.deadlines.sort_by_key(|deadline| deadline.deadline);
        deadline
    }

    fn cancel(&mut self, id: RuntimeTimerId) -> RuntimeTimerCancellation {
        if let Some(index) = self.deadlines.iter().position(|deadline| deadline.id == id) {
            let deadline = self.deadlines.remove(index);
            RuntimeTimerCancellation {
                id,
                deadline: Some(deadline.deadline),
                stale: false,
            }
        } else {
            RuntimeTimerCancellation {
                id,
                deadline: None,
                stale: true,
            }
        }
    }

    fn complete(&mut self, id: RuntimeTimerId, fired_at: Duration) -> RuntimeTimerCompletion {
        if let Some(index) = self.deadlines.iter().position(|deadline| deadline.id == id) {
            let deadline = self.deadlines.remove(index);
            RuntimeTimerCompletion {
                id,
                deadline: deadline.deadline,
                fired_at,
                invalidation: deadline.invalidation,
                stale: false,
            }
        } else {
            RuntimeTimerCompletion {
                id,
                deadline: fired_at,
                fired_at,
                invalidation: RuntimeInvalidation::new(RuntimeInvalidationReason::Explicit)
                    .detail("stale timer"),
                stale: true,
            }
        }
    }
}

impl Default for RuntimeTimerScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeIdleScheduler {
    next_id: u64,
    requests: Vec<RuntimeIdleBudgetRequest>,
}

impl RuntimeIdleScheduler {
    fn new() -> Self {
        Self {
            next_id: 1,
            requests: Vec::new(),
        }
    }

    fn request(
        &mut self,
        requested_at: Duration,
        min_budget: Duration,
        detail: Option<String>,
    ) -> RuntimeIdleBudgetRequest {
        let request = RuntimeIdleBudgetRequest::new(
            RuntimeIdleWorkId::new(self.next_id),
            min_budget,
            requested_at,
            detail,
        );
        self.next_id = self.next_id.wrapping_add(1);
        self.requests.push(request.clone());
        request
    }

    fn cancel(&mut self, id: RuntimeIdleWorkId) -> RuntimeIdleCancellation {
        if let Some(index) = self.requests.iter().position(|request| request.id == id) {
            let request = self.requests.remove(index);
            RuntimeIdleCancellation {
                id,
                requested_at: Some(request.requested_at),
                stale: false,
            }
        } else {
            RuntimeIdleCancellation {
                id,
                requested_at: None,
                stale: true,
            }
        }
    }

    fn complete(
        &mut self,
        id: RuntimeIdleWorkId,
        budget_used: Duration,
        completed_at: Duration,
        invalidation: Option<RuntimeInvalidation>,
    ) -> RuntimeIdleCompletion {
        if let Some(index) = self.requests.iter().position(|request| request.id == id) {
            let request = self.requests.remove(index);
            RuntimeIdleCompletion {
                id,
                requested_at: Some(request.requested_at),
                budget_used,
                completed_at,
                invalidation,
                stale: false,
            }
        } else {
            RuntimeIdleCompletion {
                id,
                requested_at: None,
                budget_used,
                completed_at,
                invalidation: None,
                stale: true,
            }
        }
    }
}

impl Default for RuntimeIdleScheduler {
    fn default() -> Self {
        Self::new()
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
    TimerFired {
        id: RuntimeTimerId,
        fired_at: Duration,
    },
    IdleWorkCompleted {
        id: RuntimeIdleWorkId,
        budget_used: Duration,
        invalidation: Option<RuntimeInvalidation>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeFramePlan {
    pub clock: RuntimeFrameClock,
    pub viewport: UiSize,
    pub raw_input: Vec<RawInputEvent>,
    pub platform_responses: Vec<PlatformServiceResponse>,
    pub platform_requests: Vec<PlatformServiceRequest>,
    pub timer_deadlines: Vec<RuntimeTimerDeadline>,
    pub timer_completions: Vec<RuntimeTimerCompletion>,
    pub timer_cancellations: Vec<RuntimeTimerCancellation>,
    pub idle_budget_requests: Vec<RuntimeIdleBudgetRequest>,
    pub idle_completions: Vec<RuntimeIdleCompletion>,
    pub idle_cancellations: Vec<RuntimeIdleCancellation>,
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
    pending_timer_completions: Vec<RuntimeTimerCompletion>,
    pending_timer_cancellations: Vec<RuntimeTimerCancellation>,
    pending_idle_completions: Vec<RuntimeIdleCompletion>,
    pending_idle_cancellations: Vec<RuntimeIdleCancellation>,
    timers: RuntimeTimerScheduler,
    idle: RuntimeIdleScheduler,
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
            pending_timer_completions: Vec::new(),
            pending_timer_cancellations: Vec::new(),
            pending_idle_completions: Vec::new(),
            pending_idle_cancellations: Vec::new(),
            timers: RuntimeTimerScheduler::default(),
            idle: RuntimeIdleScheduler::default(),
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

    pub fn timer_deadlines(&self) -> &[RuntimeTimerDeadline] {
        &self.timers.deadlines
    }

    pub fn idle_budget_requests(&self) -> &[RuntimeIdleBudgetRequest] {
        &self.idle.requests
    }

    pub fn schedule_timer(
        &mut self,
        delay: Duration,
        invalidation: RuntimeInvalidation,
    ) -> RuntimeTimerDeadline {
        let deadline = self
            .timers
            .schedule(self.clock.elapsed, delay, invalidation);
        self.repaint.request(RepaintRequest::After(delay));
        deadline
    }

    pub fn cancel_timer(&mut self, id: RuntimeTimerId) -> RuntimeTimerCancellation {
        let cancellation = self.timers.cancel(id);
        self.pending_timer_cancellations.push(cancellation.clone());
        cancellation
    }

    pub fn request_idle_work(
        &mut self,
        min_budget: Duration,
        detail: Option<String>,
    ) -> RuntimeIdleBudgetRequest {
        self.idle.request(self.clock.elapsed, min_budget, detail)
    }

    pub fn cancel_idle_work(&mut self, id: RuntimeIdleWorkId) -> RuntimeIdleCancellation {
        let cancellation = self.idle.cancel(id);
        self.pending_idle_cancellations.push(cancellation.clone());
        cancellation
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
            RuntimeWindowEvent::TimerFired { id, fired_at } => {
                let completion = self.timers.complete(id, fired_at);
                if !completion.stale {
                    self.repaint.invalidate(completion.invalidation.clone());
                }
                self.pending_timer_completions.push(completion);
            }
            RuntimeWindowEvent::IdleWorkCompleted {
                id,
                budget_used,
                invalidation,
            } => {
                let completion =
                    self.idle
                        .complete(id, budget_used, self.clock.elapsed, invalidation);
                if let Some(invalidation) = completion.invalidation.clone() {
                    self.repaint.invalidate(invalidation);
                }
                self.pending_idle_completions.push(completion);
            }
        }
    }

    pub fn next_frame_plan(&mut self, delta: Duration) -> RuntimeFramePlan {
        self.clock = self.clock.next(delta);
        let mut loop_guard_tripped = self.repaint.tripped_guard();
        let should_render = self.repaint.frame_due() && !loop_guard_tripped;
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
        if !self.pending_platform_requests.is_empty()
            || !self.pending_platform_responses.is_empty()
            || !self.pending_timer_completions.is_empty()
            || !self.pending_timer_cancellations.is_empty()
            || !self.pending_idle_completions.is_empty()
            || !self.pending_idle_cancellations.is_empty()
        {
            trace.push(RuntimeFramePhase::ServicePlatformRequests);
        }
        if !should_render {
            trace.push(RuntimeFramePhase::Idle);
            self.repaint.mark_idle();
            loop_guard_tripped = loop_guard_tripped || self.repaint.tripped_guard();
        }

        let plan = RuntimeFramePlan {
            clock: self.clock,
            viewport: self.viewport,
            raw_input: std::mem::take(&mut self.pending_input),
            platform_responses: std::mem::take(&mut self.pending_platform_responses),
            platform_requests: std::mem::take(&mut self.pending_platform_requests),
            timer_deadlines: self.timers.deadlines.clone(),
            timer_completions: std::mem::take(&mut self.pending_timer_completions),
            timer_cancellations: std::mem::take(&mut self.pending_timer_cancellations),
            idle_budget_requests: if should_render {
                Vec::new()
            } else {
                self.idle.requests.clone()
            },
            idle_completions: std::mem::take(&mut self.pending_idle_completions),
            idle_cancellations: std::mem::take(&mut self.pending_idle_cancellations),
            dirty_flags: self.repaint.dirty_flags(),
            invalidations: self.repaint.invalidations().to_vec(),
            trace,
            should_render,
            loop_guard_tripped,
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

        let first = runtime.next_frame_plan(Duration::from_millis(1));
        let second = runtime.next_frame_plan(Duration::from_millis(1));
        let third = runtime.next_frame_plan(Duration::from_millis(1));
        let guarded = runtime.next_frame_plan(Duration::from_millis(1));

        assert!(first.should_render);
        assert!(second.should_render);
        assert!(third.should_render);
        assert!(!first.loop_guard_tripped);
        assert!(!second.loop_guard_tripped);
        assert!(!third.loop_guard_tripped);
        assert!(guarded.loop_guard_tripped);
        assert!(!guarded.should_render);
        assert!(guarded.trace.phases().contains(&RuntimeFramePhase::Idle));
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

    #[test]
    fn timers_expose_deadlines_and_fire_repaint_invalidations() {
        let mut runtime = runtime();
        let deadline = runtime.schedule_timer(
            Duration::from_millis(50),
            RuntimeInvalidation::new(RuntimeInvalidationReason::Animation).detail("blink"),
        );

        assert_eq!(deadline.id, RuntimeTimerId::new(1));
        assert_eq!(deadline.deadline, Duration::from_millis(50));
        assert_eq!(runtime.timer_deadlines(), &[deadline.clone()]);
        assert_eq!(
            runtime.repaint_scheduler().delay(),
            Some(Duration::from_millis(50))
        );

        let idle_plan = runtime.next_frame_plan(Duration::from_millis(10));
        assert!(!idle_plan.should_render);
        assert_eq!(idle_plan.timer_deadlines, vec![deadline.clone()]);

        runtime.handle_event(RuntimeWindowEvent::TimerFired {
            id: deadline.id,
            fired_at: Duration::from_millis(50),
        });
        let render_plan = runtime.next_frame_plan(Duration::from_millis(40));

        assert!(render_plan.should_render);
        assert_eq!(render_plan.timer_deadlines, Vec::new());
        assert_eq!(render_plan.timer_completions.len(), 1);
        assert!(!render_plan.timer_completions[0].stale);
        assert_eq!(
            render_plan.timer_completions[0].deadline,
            Duration::from_millis(50)
        );
        assert_eq!(
            render_plan.invalidations[0].detail.as_deref(),
            Some("blink")
        );
        assert!(render_plan.dirty_flags.paint);
    }

    #[test]
    fn timer_cancellation_and_stale_completion_are_recorded() {
        let mut runtime = runtime();
        let deadline = runtime.schedule_timer(
            Duration::from_millis(25),
            RuntimeInvalidation::new(RuntimeInvalidationReason::Explicit),
        );

        let cancellation = runtime.cancel_timer(deadline.id);
        assert!(!cancellation.stale);
        assert_eq!(cancellation.deadline, Some(Duration::from_millis(25)));

        runtime.handle_event(RuntimeWindowEvent::TimerFired {
            id: deadline.id,
            fired_at: Duration::from_millis(25),
        });
        let plan = runtime.next_frame_plan(Duration::from_millis(25));

        assert!(!plan.should_render);
        assert_eq!(plan.timer_deadlines, Vec::new());
        assert_eq!(plan.timer_cancellations, vec![cancellation]);
        assert_eq!(plan.timer_completions.len(), 1);
        assert!(plan.timer_completions[0].stale);
        assert_eq!(plan.invalidations, Vec::new());
    }

    #[test]
    fn timer_deadlines_coalesce_with_existing_repaint_delay() {
        let mut runtime = runtime();
        runtime.handle_event(RuntimeWindowEvent::RequestRepaint(RepaintRequest::After(
            Duration::from_millis(80),
        )));
        runtime.schedule_timer(
            Duration::from_millis(30),
            RuntimeInvalidation::new(RuntimeInvalidationReason::Animation),
        );
        runtime.handle_event(RuntimeWindowEvent::RequestRepaint(RepaintRequest::After(
            Duration::from_millis(10),
        )));

        assert_eq!(
            runtime.repaint_scheduler().delay(),
            Some(Duration::from_millis(10))
        );
        let plan = runtime.next_frame_plan(Duration::ZERO);
        assert_eq!(plan.timer_deadlines.len(), 1);
        assert_eq!(plan.timer_deadlines[0].deadline, Duration::from_millis(30));
    }

    #[test]
    fn idle_work_requests_complete_with_optional_invalidation() {
        let mut runtime = runtime();
        let request =
            runtime.request_idle_work(Duration::from_millis(4), Some("precompute".to_string()));

        let idle_plan = runtime.next_frame_plan(Duration::from_millis(1));
        assert!(!idle_plan.should_render);
        assert_eq!(idle_plan.idle_budget_requests, vec![request.clone()]);
        assert!(idle_plan.trace.phases().contains(&RuntimeFramePhase::Idle));

        runtime.handle_event(RuntimeWindowEvent::IdleWorkCompleted {
            id: request.id,
            budget_used: Duration::from_millis(3),
            invalidation: Some(
                RuntimeInvalidation::new(RuntimeInvalidationReason::AsyncTask).detail("prefetch"),
            ),
        });
        let render_plan = runtime.next_frame_plan(Duration::from_millis(3));

        assert!(render_plan.should_render);
        assert_eq!(render_plan.idle_completions.len(), 1);
        assert!(!render_plan.idle_completions[0].stale);
        assert_eq!(
            render_plan.idle_completions[0].budget_used,
            Duration::from_millis(3)
        );
        assert_eq!(
            render_plan.invalidations[0].detail.as_deref(),
            Some("prefetch")
        );
        assert!(render_plan.dirty_flags.layout);
    }

    #[test]
    fn idle_cancellation_and_stale_ids_are_recorded_without_repaint() {
        let mut runtime = runtime();
        let request = runtime.request_idle_work(Duration::from_millis(8), None);

        let cancellation = runtime.cancel_idle_work(request.id);
        let stale_cancellation = runtime.cancel_idle_work(RuntimeIdleWorkId::new(99));
        runtime.handle_event(RuntimeWindowEvent::IdleWorkCompleted {
            id: request.id,
            budget_used: Duration::from_millis(1),
            invalidation: Some(RuntimeInvalidation::new(
                RuntimeInvalidationReason::Explicit,
            )),
        });
        let plan = runtime.next_frame_plan(Duration::from_millis(1));

        assert!(!plan.should_render);
        assert_eq!(plan.idle_budget_requests, Vec::new());
        assert_eq!(
            plan.idle_cancellations,
            vec![cancellation, stale_cancellation]
        );
        assert!(!plan.idle_cancellations[0].stale);
        assert!(plan.idle_cancellations[1].stale);
        assert_eq!(plan.idle_completions.len(), 1);
        assert!(plan.idle_completions[0].stale);
        assert_eq!(plan.invalidations, Vec::new());
    }

    #[test]
    fn loop_guard_prevents_unbounded_repaint_loops() {
        let mut runtime = runtime().with_loop_guard(RuntimeLoopGuard::new(0));
        runtime.handle_event(RuntimeWindowEvent::RequestRepaint(
            RepaintRequest::Continuous { active: true },
        ));

        let rendered = runtime.next_frame_plan(Duration::from_millis(1));
        let guarded = runtime.next_frame_plan(Duration::from_millis(1));

        assert!(rendered.should_render);
        assert!(!rendered.loop_guard_tripped);
        assert!(!guarded.should_render);
        assert!(guarded.loop_guard_tripped);
        assert_eq!(
            guarded.trace.phases(),
            &[
                RuntimeFramePhase::CollectPlatformEvents,
                RuntimeFramePhase::Idle,
            ]
        );
    }
}
