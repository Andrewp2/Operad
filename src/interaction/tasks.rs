//! Backend-neutral async task lifecycle contracts.
//!
//! Hosts own the executor and transport. This module keeps the retained UI
//! side deterministic: every task result is tied to a generation, stale results
//! are reported without mutating newer state, and visible task changes can be
//! summarized as runtime async invalidations.

use std::collections::BTreeMap;
use std::fmt;

use crate::{RuntimeInvalidation, RuntimeInvalidationReason};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for TaskId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for TaskId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskGeneration(pub u64);

impl TaskGeneration {
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskHandle {
    pub id: TaskId,
    pub generation: TaskGeneration,
}

impl TaskHandle {
    pub fn new(id: impl Into<TaskId>, generation: TaskGeneration) -> Self {
        Self {
            id: id.into(),
            generation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskProgress {
    pub completed: u64,
    pub total: Option<u64>,
    pub label: Option<String>,
}

impl TaskProgress {
    pub const fn indeterminate() -> Self {
        Self {
            completed: 0,
            total: None,
            label: None,
        }
    }

    pub const fn new(completed: u64, total: u64) -> Self {
        Self {
            completed,
            total: Some(total),
            label: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn fraction(&self) -> Option<f32> {
        let total = self.total?;
        if total == 0 {
            return Some(1.0);
        }
        Some((self.completed.min(total) as f32) / (total as f32))
    }
}

impl Default for TaskProgress {
    fn default() -> Self {
        Self::indeterminate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskError {
    pub message: String,
}

impl TaskError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskState {
    pub handle: TaskHandle,
    pub status: TaskStatus,
    pub progress: TaskProgress,
    pub completion: Option<String>,
    pub error: Option<TaskError>,
    pub cancellation_requested: bool,
}

impl TaskState {
    fn new(handle: TaskHandle) -> Self {
        Self {
            handle,
            status: TaskStatus::Pending,
            progress: TaskProgress::default(),
            completion: None,
            error: None,
            cancellation_requested: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskResultDisposition {
    Applied,
    Stale {
        current_generation: Option<TaskGeneration>,
    },
    AlreadyTerminal {
        status: TaskStatus,
    },
}

impl TaskResultDisposition {
    pub const fn applied(&self) -> bool {
        matches!(self, Self::Applied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInvalidationSummary {
    pub task_id: TaskId,
    pub generation: TaskGeneration,
    pub status: TaskStatus,
    pub detail: String,
}

impl TaskInvalidationSummary {
    pub fn invalidation(&self) -> RuntimeInvalidation {
        RuntimeInvalidation::new(RuntimeInvalidationReason::AsyncTask).detail(self.detail.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLifecycleReport {
    pub disposition: TaskResultDisposition,
    pub invalidation: Option<TaskInvalidationSummary>,
}

impl TaskLifecycleReport {
    fn applied(state: &TaskState, detail: impl Into<String>) -> Self {
        Self {
            disposition: TaskResultDisposition::Applied,
            invalidation: Some(TaskInvalidationSummary {
                task_id: state.handle.id.clone(),
                generation: state.handle.generation,
                status: state.status,
                detail: detail.into(),
            }),
        }
    }

    fn stale(current_generation: Option<TaskGeneration>) -> Self {
        Self {
            disposition: TaskResultDisposition::Stale { current_generation },
            invalidation: None,
        }
    }

    fn terminal(status: TaskStatus) -> Self {
        Self {
            disposition: TaskResultDisposition::AlreadyTerminal { status },
            invalidation: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskRegistry {
    tasks: BTreeMap<TaskId, TaskState>,
    generations: BTreeMap<TaskId, TaskGeneration>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self, id: impl Into<TaskId>) -> (TaskHandle, TaskInvalidationSummary) {
        let id = id.into();
        let generation = self
            .generations
            .get(&id)
            .copied()
            .unwrap_or(TaskGeneration(0))
            .next();
        self.generations.insert(id.clone(), generation);

        let handle = TaskHandle::new(id.clone(), generation);
        let state = TaskState::new(handle.clone());
        let summary = TaskInvalidationSummary {
            task_id: id.clone(),
            generation,
            status: state.status,
            detail: format!("async task `{id}` started"),
        };
        self.tasks.insert(id, state);
        (handle, summary)
    }

    pub fn state(&self, id: &TaskId) -> Option<&TaskState> {
        self.tasks.get(id)
    }

    pub fn progress(&mut self, handle: &TaskHandle, progress: TaskProgress) -> TaskLifecycleReport {
        let Some(state) = self.current_mut(handle) else {
            return TaskLifecycleReport::stale(self.current_generation(&handle.id));
        };
        if state.status.is_terminal() {
            return TaskLifecycleReport::terminal(state.status);
        }
        state.status = TaskStatus::Running;
        state.progress = progress;
        TaskLifecycleReport::applied(state, format!("async task `{}` progressed", handle.id))
    }

    pub fn complete(
        &mut self,
        handle: &TaskHandle,
        completion: impl Into<String>,
    ) -> TaskLifecycleReport {
        let Some(state) = self.current_mut(handle) else {
            return TaskLifecycleReport::stale(self.current_generation(&handle.id));
        };
        if state.status.is_terminal() {
            return TaskLifecycleReport::terminal(state.status);
        }
        state.status = TaskStatus::Succeeded;
        state.progress = TaskProgress::new(1, 1);
        state.completion = Some(completion.into());
        state.error = None;
        TaskLifecycleReport::applied(state, format!("async task `{}` completed", handle.id))
    }

    pub fn fail(&mut self, handle: &TaskHandle, error: TaskError) -> TaskLifecycleReport {
        let Some(state) = self.current_mut(handle) else {
            return TaskLifecycleReport::stale(self.current_generation(&handle.id));
        };
        if state.status.is_terminal() {
            return TaskLifecycleReport::terminal(state.status);
        }
        state.status = TaskStatus::Failed;
        state.error = Some(error);
        TaskLifecycleReport::applied(state, format!("async task `{}` failed", handle.id))
    }

    pub fn cancel(&mut self, handle: &TaskHandle) -> TaskLifecycleReport {
        let Some(state) = self.current_mut(handle) else {
            return TaskLifecycleReport::stale(self.current_generation(&handle.id));
        };
        if state.status.is_terminal() {
            return TaskLifecycleReport::terminal(state.status);
        }
        state.status = TaskStatus::Cancelled;
        state.cancellation_requested = true;
        TaskLifecycleReport::applied(state, format!("async task `{}` cancelled", handle.id))
    }

    fn current_generation(&self, id: &TaskId) -> Option<TaskGeneration> {
        self.tasks.get(id).map(|state| state.handle.generation)
    }

    fn current_mut(&mut self, handle: &TaskHandle) -> Option<&mut TaskState> {
        let state = self.tasks.get_mut(&handle.id)?;
        (state.handle.generation == handle.generation).then_some(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasks_track_progress_and_completion_with_async_invalidation() {
        let mut tasks = TaskRegistry::new();
        let (handle, started) = tasks.start("export");
        assert_eq!(started.status, TaskStatus::Pending);

        let progressed = tasks.progress(&handle, TaskProgress::new(2, 4).label("half"));
        assert!(progressed.disposition.applied());
        assert_eq!(
            progressed.invalidation.unwrap().invalidation().reason,
            RuntimeInvalidationReason::AsyncTask
        );

        let state = tasks.state(&TaskId::from("export")).unwrap();
        assert_eq!(state.status, TaskStatus::Running);
        assert_eq!(state.progress.fraction(), Some(0.5));

        let completed = tasks.complete(&handle, "ok");
        assert!(completed.disposition.applied());
        let state = tasks.state(&TaskId::from("export")).unwrap();
        assert_eq!(state.status, TaskStatus::Succeeded);
        assert_eq!(state.completion.as_deref(), Some("ok"));
    }

    #[test]
    fn tasks_cancel_and_ignore_late_completion() {
        let mut tasks = TaskRegistry::new();
        let (handle, _) = tasks.start("sync");

        let cancelled = tasks.cancel(&handle);
        assert!(cancelled.disposition.applied());
        let completed = tasks.complete(&handle, "late");
        assert_eq!(
            completed.disposition,
            TaskResultDisposition::AlreadyTerminal {
                status: TaskStatus::Cancelled
            }
        );

        let state = tasks.state(&TaskId::from("sync")).unwrap();
        assert_eq!(state.status, TaskStatus::Cancelled);
        assert!(state.completion.is_none());
        assert!(state.cancellation_requested);
    }

    #[test]
    fn tasks_reject_stale_results_without_replacing_newer_state() {
        let mut tasks = TaskRegistry::new();
        let (old_handle, _) = tasks.start("validate");
        let (new_handle, _) = tasks.start("validate");

        let stale = tasks.complete(&old_handle, "old");
        assert_eq!(
            stale.disposition,
            TaskResultDisposition::Stale {
                current_generation: Some(new_handle.generation)
            }
        );

        let state = tasks.state(&TaskId::from("validate")).unwrap();
        assert_eq!(state.handle.generation, new_handle.generation);
        assert_eq!(state.status, TaskStatus::Pending);
        assert!(state.completion.is_none());
    }
}
