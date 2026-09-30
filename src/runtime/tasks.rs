//! Typed completion delivery from application-owned jobs to the UI thread.
//!
//! Operad owns delivery, wakeup, and view invalidation, not the application's
//! executor. Native senders are thread-safe when `Message: Send`; browser
//! senders can be used from local futures and JavaScript callbacks. A Web Worker
//! can forward its messages through a callback on the application's browser thread.

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) type TaskWaker = Arc<dyn Fn() + Send + Sync>;
#[cfg(target_arch = "wasm32")]
pub(crate) type TaskWaker = std::rc::Rc<dyn Fn()>;

struct Queue<Message> {
    messages: VecDeque<Message>,
    capacity: usize,
    closed: bool,
    wake: Option<TaskWaker>,
}

/// Create a bounded channel for [`super::RuntimeHooks::with_task_completions`].
///
/// Sending never waits for the UI to drain the queue. A full or closed channel
/// returns the message to its caller. Results queued before the host starts are
/// delivered once the host installs its wakeup callback. Each channel has one
/// receiver; dropping it closes the channel and discards pending results.
///
/// # Panics
/// Panics if `capacity` is zero.
pub fn task_channel<Message>(capacity: usize) -> (TaskSender<Message>, TaskReceiver<Message>) {
    assert!(capacity > 0, "task completion capacity must be positive");
    let queue = Arc::new(Mutex::new(Queue {
        messages: VecDeque::new(),
        capacity,
        closed: false,
        wake: None,
    }));
    (
        TaskSender {
            queue: queue.clone(),
        },
        TaskReceiver { queue },
    )
}

/// A clonable completion endpoint. It never owns or accesses application state.
///
/// Native applications may move it into a worker thread or an async task when
/// its message type is `Send`. The handler still runs only on the host thread.
pub struct TaskSender<Message> {
    queue: Arc<Mutex<Queue<Message>>>,
}

impl<Message> Clone for TaskSender<Message> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
        }
    }
}

impl<Message> TaskSender<Message> {
    /// Queue a result and wake the host if the queue was empty.
    ///
    /// Accepted messages are delivered once, in channel order, unless the
    /// receiver closes first. Wakeups coalesce while results are pending. The
    /// host drains a finite batch, so a handler may enqueue more work without
    /// causing recursive dispatch or starving input in the current frame.
    pub fn try_send(&self, message: Message) -> Result<(), TaskSendError<Message>> {
        let wake = {
            let mut queue = self.queue.lock().expect("task completion queue poisoned");
            if queue.closed {
                return Err(TaskSendError::Closed(message));
            }
            if queue.messages.len() == queue.capacity {
                return Err(TaskSendError::Full(message));
            }
            queue.messages.push_back(message);
            if queue.messages.len() == 1 {
                queue.wake.clone()
            } else {
                None
            }
        };
        // Never invoke host or application code while holding the queue lock.
        if let Some(wake) = wake {
            wake();
        }
        Ok(())
    }

    /// Allows a producer to stop cooperatively after the application shuts down.
    /// A receiver can close immediately after this check; always handle send errors.
    pub fn is_closed(&self) -> bool {
        self.queue
            .lock()
            .expect("task completion queue poisoned")
            .closed
    }
}

impl<Message> fmt::Debug for TaskSender<Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskSender")
            .field("closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

/// The sole consumer of a task channel. Attach it to runtime hooks; it need not
/// be polled by the application. Closing the receiver does not abort external work.
pub struct TaskReceiver<Message> {
    queue: Arc<Mutex<Queue<Message>>>,
}

impl<Message> TaskReceiver<Message> {
    pub(crate) fn set_waker(&mut self, wake: TaskWaker) {
        let (previous, pending) = {
            let mut queue = self.queue.lock().expect("task completion queue poisoned");
            let previous = queue.wake.replace(wake.clone());
            (previous, !queue.messages.is_empty())
        };
        drop(previous);
        // Installing after an early send must not lose the only wakeup.
        if pending {
            wake();
        }
    }

    fn drain(&mut self) -> VecDeque<Message> {
        std::mem::take(
            &mut self
                .queue
                .lock()
                .expect("task completion queue poisoned")
                .messages,
        )
    }
}

impl<Message> Drop for TaskReceiver<Message> {
    fn drop(&mut self) {
        let (messages, wake) = {
            let mut queue = self.queue.lock().expect("task completion queue poisoned");
            queue.closed = true;
            (std::mem::take(&mut queue.messages), queue.wake.take())
        };
        // Message and callback destructors may themselves use the sender.
        drop(messages);
        drop(wake);
    }
}

/// Sending failed without consuming the result. The caller chooses whether to
/// retry, cancel, or report overload; the runtime never silently drops it.
#[derive(Debug, PartialEq, Eq)]
pub enum TaskSendError<Message> {
    Full(Message),
    Closed(Message),
}

impl<Message> TaskSendError<Message> {
    pub fn into_inner(self) -> Message {
        match self {
            Self::Full(message) | Self::Closed(message) => message,
        }
    }
}

impl<Message> fmt::Display for TaskSendError<Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Full(_) => "task completion queue is full",
            Self::Closed(_) => "task completion receiver is closed",
        })
    }
}
impl<Message: fmt::Debug> std::error::Error for TaskSendError<Message> {}

pub(crate) trait CompletionSource<State> {
    fn set_waker(&mut self, wake: TaskWaker);
    fn take_batch(&mut self) -> Option<Box<dyn FnOnce(&mut State) -> usize>>;
}

pub(crate) struct CompletionHandler<State, Message, Handler> {
    pub receiver: TaskReceiver<Message>,
    // Keeping the callback outside the thread-safe queue permits ordinary !Send UI state.
    pub handler: std::rc::Rc<std::cell::RefCell<Handler>>,
    pub marker: std::marker::PhantomData<fn(&mut State)>,
}

impl<State: 'static, Message: 'static, Handler: FnMut(&mut State, Message) + 'static>
    CompletionSource<State> for CompletionHandler<State, Message, Handler>
{
    fn set_waker(&mut self, wake: TaskWaker) {
        self.receiver.set_waker(wake);
    }

    fn take_batch(&mut self) -> Option<Box<dyn FnOnce(&mut State) -> usize>> {
        let messages = self.receiver.drain();
        if messages.is_empty() {
            return None;
        }
        let handler = self.handler.clone();
        Some(Box::new(move |state| {
            let count = messages.len();
            for message in messages {
                (handler.borrow_mut())(state, message);
            }
            count
        }))
    }
}

#[cfg(test)]
mod tests;
