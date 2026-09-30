use super::*;
use crate::runtime::session::RuntimeSession;
use crate::runtime::RuntimeHooks;
use crate::{
    ApproxTextMeasurer, LayoutStyle, TextStyle, UiContent, UiDocument, UiDocumentScale, UiNode,
    UiSize,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(5);

fn rendered_value(session: &mut RuntimeSession, value: &str) -> String {
    let document = session
        .build_document(
            UiSize::new(200.0, 100.0),
            UiDocumentScale::DEFAULT,
            None,
            &mut ApproxTextMeasurer,
            |_, _views| {
                let mut doc = UiDocument::new(LayoutStyle::size(200.0, 100.0));
                doc.add_child(
                    doc.root(),
                    UiNode::text(
                        "result",
                        value,
                        TextStyle::default(),
                        LayoutStyle::size(200.0, 30.0),
                    ),
                );
                doc
            },
        )
        .unwrap();
    let text = document
        .nodes()
        .iter()
        .find_map(|node| match node.content() {
            UiContent::Text(text) => Some(text.text.clone()),
            _ => None,
        })
        .unwrap();
    session.retain_document(document);
    text
}

#[test]
fn worker_completion_updates_non_send_state_on_ui_thread_and_invalidates_retained_view() {
    let (sender, receiver) = task_channel(4);
    let (wake, wakes) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    let (sent, next_sent) = mpsc::channel();
    let ui_thread = std::thread::current().id();
    // The callback and state are deliberately !Send. Only messages cross threads.
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed_calls = calls.clone();
    let mut hooks = RuntimeHooks::new().with_task_completions(
        receiver,
        move |state: &mut Rc<RefCell<String>>, message: Result<String, String>| {
            assert_eq!(std::thread::current().id(), ui_thread);
            observed_calls.borrow_mut().push(message.clone());
            if observed_calls.borrow().len() == 1 {
                // The queue has drained, but its handler has not finished: a
                // concurrent result must schedule another delivery.
                resume.send(()).unwrap();
                next_sent.recv_timeout(TIMEOUT).unwrap();
            }
            *state.borrow_mut() = message.unwrap_or_else(|error| error);
        },
    );
    hooks.set_task_waker(move || wake.send(()).unwrap());
    let mut session = RuntimeSession::new();
    let mut state = Rc::new(RefCell::new("waiting".to_owned()));
    assert_eq!(rendered_value(&mut session, &state.borrow()), "waiting");

    let worker = std::thread::spawn(move || {
        sender.try_send(Ok("loaded".to_owned())).unwrap();
        resumed.recv_timeout(TIMEOUT).unwrap();
        sender.try_send(Err("read failed".to_owned())).unwrap();
        sent.send(()).unwrap();
    });
    wakes.recv_timeout(TIMEOUT).unwrap();
    assert_eq!(&*state.borrow(), "waiting");
    assert_eq!(session.apply_task_completions(&mut hooks, &mut state), 1);
    assert_eq!(rendered_value(&mut session, &state.borrow()), "loaded");
    wakes.recv_timeout(TIMEOUT).unwrap();
    assert_eq!(session.apply_task_completions(&mut hooks, &mut state), 1);
    assert_eq!(rendered_value(&mut session, &state.borrow()), "read failed");
    assert_eq!(calls.borrow().len(), 2);
    assert_eq!(session.apply_task_completions(&mut hooks, &mut state), 0);
    assert!(!session.view_needs_rebuild());
    worker.join().unwrap();
}

#[test]
fn startup_and_replacement_wakers_deliver_pending_results_and_coalesce_bursts() {
    for install_before_registering in [false, true] {
        let (sender, receiver) = task_channel(4);
        sender.try_send(1).unwrap();
        let wakes = Arc::new(AtomicUsize::new(0));
        let count = wakes.clone();
        let mut hooks = RuntimeHooks::new();
        let handler = |state: &mut Vec<u32>, value| state.push(value);
        if install_before_registering {
            hooks.set_task_waker(move || {
                count.fetch_add(1, Ordering::SeqCst);
            });
            hooks = hooks.with_task_completions(receiver, handler);
        } else {
            hooks = hooks.with_task_completions(receiver, handler);
            hooks.set_task_waker(move || {
                count.fetch_add(1, Ordering::SeqCst);
            });
        }
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        let replacement_wakes = Arc::new(AtomicUsize::new(0));
        let count = replacement_wakes.clone();
        hooks.set_task_waker(move || {
            count.fetch_add(1, Ordering::SeqCst);
        });
        sender.try_send(2).unwrap();
        sender.clone().try_send(3).unwrap();
        assert_eq!(replacement_wakes.load(Ordering::SeqCst), 1);
        let mut session = RuntimeSession::new();
        let mut state = Vec::new();
        assert_eq!(session.apply_task_completions(&mut hooks, &mut state), 3);
        sender.try_send(4).unwrap();
        assert_eq!(replacement_wakes.load(Ordering::SeqCst), 2);
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        assert_eq!(session.apply_task_completions(&mut hooks, &mut state), 1);
        assert_eq!(state, [1, 2, 3, 4]);
    }
}

#[test]
fn full_queue_returns_ownership_for_retry_and_shutdown_rejects_remaining_producers() {
    let (sender, receiver) = task_channel(1);
    let mut hooks = RuntimeHooks::new()
        .with_task_completions(receiver, |state: &mut Vec<String>, message| {
            state.push(message)
        });
    sender.try_send("first".to_owned()).unwrap();
    let retry = match sender.try_send("second".to_owned()) {
        Err(TaskSendError::Full(message)) => message,
        result => panic!("expected backpressure, got {result:?}"),
    };
    let mut session = RuntimeSession::new();
    let mut state = Vec::new();
    assert_eq!(session.apply_task_completions(&mut hooks, &mut state), 1);
    sender.try_send(retry).unwrap();
    assert_eq!(session.apply_task_completions(&mut hooks, &mut state), 1);
    sender.try_send("pending at shutdown".to_owned()).unwrap();
    drop(hooks);
    assert!(sender.is_closed());
    assert_eq!(
        sender.clone().try_send("late".to_owned()),
        Err(TaskSendError::Closed("late".to_owned()))
    );
    assert_eq!(state, ["first", "second"]);
}

#[test]
fn shutdown_drops_pending_messages_outside_the_queue_lock() {
    struct ReentrantDrop {
        sender: TaskSender<ReentrantDrop>,
        dropped: mpsc::Sender<bool>,
    }
    impl Drop for ReentrantDrop {
        fn drop(&mut self) {
            self.dropped.send(self.sender.is_closed()).unwrap();
        }
    }
    let (sender, receiver) = task_channel(1);
    let (dropped, did_drop) = mpsc::channel();
    assert!(sender
        .try_send(ReentrantDrop {
            sender: sender.clone(),
            dropped
        })
        .is_ok());
    let closer = std::thread::spawn(move || drop(receiver));
    assert!(did_drop
        .recv_timeout(TIMEOUT)
        .expect("dropping a message deadlocked the queue"));
    closer.join().unwrap();
}

#[test]
fn callbacks_enqueue_for_a_later_batch_even_across_channels() {
    let (first, first_receiver) = task_channel(4);
    let (second, second_receiver) = task_channel(4);
    let again = first.clone();
    let mut hooks = RuntimeHooks::new()
        .with_task_completions(first_receiver, move |state: &mut Vec<u32>, value| {
            state.push(value);
            if value == 1 {
                again.try_send(2).unwrap();
                second.try_send(3).unwrap();
            }
        })
        .with_task_completions(second_receiver, |state: &mut Vec<u32>, value| {
            state.push(value)
        });
    let (wake, wakes) = mpsc::channel();
    hooks.set_task_waker(move || wake.send(()).unwrap());
    first.try_send(1).unwrap();
    wakes.recv_timeout(TIMEOUT).unwrap();
    let mut session = RuntimeSession::new();
    let mut state = Vec::new();
    assert_eq!(session.apply_task_completions(&mut hooks, &mut state), 1);
    assert_eq!(state, [1]);
    wakes.recv_timeout(TIMEOUT).unwrap();
    assert_eq!(session.apply_task_completions(&mut hooks, &mut state), 2);
    state[1..].sort(); // Ordering between independent channels is unspecified.
    assert_eq!(state, [1, 2, 3]);
    assert_eq!(session.apply_task_completions(&mut hooks, &mut state), 0);
}

#[test]
fn task_registry_keeps_late_and_cancelled_results_from_overwriting_current_state() {
    use crate::tasks::{TaskHandle, TaskRegistry};
    #[derive(Default)]
    struct State {
        tasks: TaskRegistry,
        value: String,
    }
    let (sender, receiver) = task_channel(4);
    let mut hooks = RuntimeHooks::new().with_task_completions(
        receiver,
        |state: &mut State, (handle, result): (TaskHandle, String)| {
            if state
                .tasks
                .complete(&handle, "loaded")
                .disposition
                .applied()
            {
                state.value = result;
            }
        },
    );
    let mut state = State::default();
    let (old, _) = state.tasks.start("load");
    let (current, _) = state.tasks.start("load");
    let (cancelled, _) = state.tasks.start("cancelled");
    state.tasks.cancel(&cancelled);
    sender
        .try_send((current.clone(), "current".into()))
        .unwrap();
    sender.try_send((old, "stale".into())).unwrap();
    sender.try_send((cancelled, "cancelled".into())).unwrap();
    sender.try_send((current, "duplicate".into())).unwrap();
    assert_eq!(
        RuntimeSession::new().apply_task_completions(&mut hooks, &mut state),
        4
    );
    assert_eq!(state.value, "current");
}
