//! Repaint deadlines shared by native, browser, and headless hosts.
//!
//! Delayed requests coalesce to the earliest absolute deadline. An earlier
//! input frame does not consume that deadline. Continuous rendering is a
//! separate policy, so disabling it never cancels one-shot requests.

use super::{RuntimeInvalidation, RuntimeLoopGuard};
use crate::{platform::RepaintRequest, DirtyFlags};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeRepaintScheduler {
    next_frame: bool,
    now: Duration,
    deadline: Option<Duration>,
    retry_deadline: Option<Duration>,
    retry_backoff: Duration,
    in_flight: Option<(DirtyFlags, Vec<RuntimeInvalidation>)>,
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
            now: Duration::ZERO,
            deadline: None,
            retry_deadline: None,
            retry_backoff: Duration::ZERO,
            in_flight: None,
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
                let deadline = self.now.saturating_add(delay);
                self.deadline = Some(
                    self.deadline
                        .map_or(deadline, |current| current.min(deadline)),
                );
            }
            RepaintRequest::Area(_) => self.next_frame = true,
            RepaintRequest::Continuous { active } => {
                self.continuous = active;
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

    /// Advance using a monotonic, host-relative clock. Regressing timestamps
    /// cannot postpone work that has already become due.
    pub fn advance_to(&mut self, now: Duration) {
        self.now = self.now.max(now);
    }

    pub fn delay(&self) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_sub(self.now))
    }

    pub fn frame_due(&self) -> bool {
        self.retry_delay(self.now).is_none()
            && (self.next_frame || self.continuous || self.delay() == Some(Duration::ZERO))
    }

    /// Minimum wait before another presentation attempt. External input and
    /// host tick/idle policies must not bypass this delay. Use the same monotonic
    /// clock as `advance_to`; the scheduler never moves its clock backward.
    pub fn retry_delay(&self, now: Duration) -> Option<Duration> {
        self.retry_deadline
            .map(|deadline| deadline.saturating_sub(now.max(self.now)))
            .filter(|delay| !delay.is_zero())
    }

    /// None means the host can sleep until an external event arrives.
    pub fn next_frame_delay(&self) -> Option<Duration> {
        if let Some(delay) = self.retry_delay(self.now) {
            return Some(delay);
        }
        if self.frame_due() {
            Some(Duration::ZERO)
        } else {
            self.delay()
        }
    }

    /// Consume only work due at frame start. Requests made while processing or
    /// presenting this frame belong to a subsequent frame.
    pub fn begin_frame(&mut self) {
        assert!(
            self.in_flight.is_none(),
            "a repaint frame is already in flight"
        );
        self.next_frame = false;
        if self.delay() == Some(Duration::ZERO) {
            self.deadline = None;
        }
        self.in_flight = Some((
            std::mem::take(&mut self.dirty_flags),
            std::mem::take(&mut self.invalidations),
        ));
    }

    /// Advance the clock to completion before reporting a failed attempt.
    /// Retries back off from 16 ms to 250 ms; success resets the failure streak.
    pub fn finish_frame(&mut self, rendered: bool) {
        if self.in_flight.is_none() {
            return;
        }
        if rendered {
            self.in_flight = None;
            self.retry_deadline = None;
            self.retry_backoff = Duration::ZERO;
            self.frames_without_idle = self.frames_without_idle.saturating_add(1);
        } else {
            self.defer_frame();
            self.retry_backoff = if self.retry_backoff.is_zero() {
                Duration::from_millis(16)
            } else {
                self.retry_backoff
                    .saturating_mul(2)
                    .min(Duration::from_millis(250))
            };
            self.retry_deadline = Some(self.now.saturating_add(self.retry_backoff));
        }
    }

    /// Finish preparing a frame without attempting presentation. Retain its
    /// work without extending an existing retry deadline or failure streak.
    pub fn defer_frame(&mut self) {
        let Some((dirty, mut invalidations)) = self.in_flight.take() else {
            return;
        };
        self.next_frame = true;
        self.dirty_flags = dirty.union(self.dirty_flags);
        invalidations.append(&mut self.invalidations);
        self.invalidations = invalidations;
        self.frames_without_idle = 0;
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

/// Coalesce a batch without losing independent one-shot deadlines or the final
/// continuous-rendering policy. Requests in a batch share the same clock time.
pub fn coalesce_repaint_requests(
    requests: impl IntoIterator<Item = RepaintRequest>,
) -> Vec<RepaintRequest> {
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

    let mut requests = Vec::new();
    if let Some(active) = continuous {
        requests.push(RepaintRequest::Continuous { active });
    }
    if next_frame {
        requests.push(RepaintRequest::NextFrame);
    }
    if let Some(delay) = delay {
        requests.push(RepaintRequest::After(delay));
    }
    requests
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::RuntimeInvalidationReason;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn persistent_failures_back_off_from_completion_and_preserve_work() {
        let mut scheduler = RuntimeRepaintScheduler::default();
        let mut now = Duration::ZERO;
        let mut previous_delay = Duration::ZERO;
        for _ in 0..12 {
            scheduler.advance_to(now);
            scheduler.invalidate(RuntimeInvalidation::new(RuntimeInvalidationReason::Resize));
            scheduler.begin_frame();
            // Include a slow unsuccessful attempt: delaying from frame start
            // would make the retry immediately due again.
            now += ms(300);
            scheduler.advance_to(now);
            scheduler.finish_frame(false);
            let delay = scheduler.next_frame_delay().expect("retry pending");
            assert!(delay > Duration::ZERO, "failed presentation must not spin");
            assert!(delay >= previous_delay && delay <= ms(250));
            assert!(scheduler.dirty_flags().layout);
            // Ordinary work cannot shorten the surface's recovery interval.
            scheduler.request(RepaintRequest::Continuous { active: true });
            scheduler.request(RepaintRequest::NextFrame);
            scheduler.request(RepaintRequest::After(ms(1)));
            assert_eq!(scheduler.next_frame_delay(), Some(delay));
            scheduler.advance_to(now + delay - Duration::from_nanos(1));
            assert!(!scheduler.frame_due());
            now += delay;
            scheduler.advance_to(now);
            assert!(scheduler.frame_due());
            previous_delay = delay;
        }
        assert_eq!(previous_delay, ms(250), "persistent failures reach the cap");
        scheduler.begin_frame();
        scheduler.request(RepaintRequest::Continuous { active: false });
        scheduler.request(RepaintRequest::After(ms(500)));
        scheduler.finish_frame(true);
        assert_eq!(scheduler.next_frame_delay(), Some(ms(500)));
        assert!(scheduler.invalidations().is_empty());
        scheduler.begin_frame();
        scheduler.finish_frame(false);
        assert!(
            scheduler.next_frame_delay().unwrap() < previous_delay,
            "a successful frame must reset the failure streak"
        );
    }

    #[test]
    fn coalescing_preserves_scheduling_for_all_short_request_sequences() {
        let requests = [
            RepaintRequest::NextFrame,
            RepaintRequest::After(Duration::ZERO),
            RepaintRequest::After(ms(5)),
            RepaintRequest::After(ms(20)),
            RepaintRequest::Continuous { active: true },
            RepaintRequest::Continuous { active: false },
            RepaintRequest::Area(crate::platform::LogicalRect::new(0.0, 0.0, 1.0, 1.0)),
        ];
        for mut sequence in 0..requests.len().pow(4) {
            let mut batch = Vec::new();
            for _ in 0..4 {
                batch.push(requests[sequence % requests.len()].clone());
                sequence /= requests.len();
            }
            for initially_continuous in [false, true] {
                let mut direct = RuntimeRepaintScheduler::default();
                let mut coalesced = RuntimeRepaintScheduler::default();
                for scheduler in [&mut direct, &mut coalesced] {
                    scheduler.request(RepaintRequest::Continuous {
                        active: initially_continuous,
                    });
                }
                for request in batch.clone() {
                    direct.request(request);
                }
                for request in coalesce_repaint_requests(batch.clone()) {
                    coalesced.request(request);
                }
                for now in [0, 3, 5, 20, 30] {
                    direct.advance_to(ms(now));
                    coalesced.advance_to(ms(now));
                    assert_eq!(
                        direct.next_frame_delay(),
                        coalesced.next_frame_delay(),
                        "{batch:?} at {now}"
                    );
                    assert_eq!(direct.continuous(), coalesced.continuous());
                    // Include an unrelated early frame, so dropping a future
                    // deadline cannot hide behind an immediate repaint.
                    for scheduler in [&mut direct, &mut coalesced] {
                        scheduler.begin_frame();
                        scheduler.finish_frame(true);
                    }
                }
            }
        }
    }

    #[test]
    fn relative_delays_compare_absolute_deadlines_and_survive_early_frames() {
        let mut scheduler = RuntimeRepaintScheduler::default();
        scheduler.request(RepaintRequest::After(ms(100)));
        scheduler.advance_to(ms(90));
        scheduler.request(RepaintRequest::After(ms(20)));
        assert_eq!(scheduler.next_frame_delay(), Some(ms(10)));
        scheduler.begin_frame();
        scheduler.finish_frame(true);
        assert_eq!(scheduler.next_frame_delay(), Some(ms(10)));
        scheduler.advance_to(ms(100));
        assert!(scheduler.frame_due());
        scheduler.begin_frame();
        scheduler.finish_frame(true);
        assert_eq!(scheduler.next_frame_delay(), None);
    }

    #[test]
    fn requests_during_a_frame_survive_presentation_or_failure() {
        for succeeds in [true, false] {
            let mut scheduler = RuntimeRepaintScheduler::default();
            scheduler.invalidate(RuntimeInvalidation::new(RuntimeInvalidationReason::Resize));
            scheduler.begin_frame();
            scheduler.request(RepaintRequest::NextFrame);
            scheduler.request(RepaintRequest::After(ms(50)));
            scheduler.finish_frame(succeeds);
            let delay = if succeeds { Duration::ZERO } else { ms(16) };
            assert_eq!(scheduler.next_frame_delay(), Some(delay));
            assert_eq!(scheduler.dirty_flags().layout, !succeeds);
            scheduler.advance_to(delay);
            assert!(scheduler.frame_due());
            scheduler.begin_frame();
            scheduler.finish_frame(true);
            assert!(!scheduler.frame_due());
            assert_eq!(scheduler.next_frame_delay(), Some(ms(50) - delay));
        }
    }

    #[test]
    fn continuous_toggle_never_discards_independent_work() {
        for one_shot in [
            None,
            Some(RepaintRequest::NextFrame),
            Some(RepaintRequest::After(ms(9))),
        ] {
            let mut scheduler = RuntimeRepaintScheduler::default();
            scheduler.request(RepaintRequest::Continuous { active: true });
            scheduler.begin_frame();
            if let Some(request) = one_shot.clone() {
                scheduler.request(request);
            }
            scheduler.request(RepaintRequest::Continuous { active: false });
            scheduler.finish_frame(true);
            let expected = match one_shot {
                None => None,
                Some(RepaintRequest::NextFrame) => Some(Duration::ZERO),
                _ => Some(ms(9)),
            };
            assert_eq!(scheduler.next_frame_delay(), expected);
        }
    }

    #[test]
    fn advancing_clock_cannot_postpone_a_deadline_or_overflow() {
        let mut scheduler = RuntimeRepaintScheduler::default();
        scheduler.advance_to(ms(10));
        scheduler.request(RepaintRequest::After(ms(10)));
        scheduler.advance_to(ms(20));
        scheduler.advance_to(ms(5));
        assert!(scheduler.frame_due());
        scheduler.begin_frame();
        scheduler.request(RepaintRequest::After(Duration::MAX));
        scheduler.finish_frame(true);
        scheduler.advance_to(Duration::MAX);
        assert!(scheduler.frame_due());
    }
}
