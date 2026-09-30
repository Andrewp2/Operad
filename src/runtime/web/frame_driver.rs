//! Browser wakeup handles. Scheduling policy lives in the shared runtime;
//! this adapter translates its deadlines to animation frames and timeouts.

use super::{browser_window, log_web_runtime_reentry, stop_web_frame_error, WebRuntimeApp};
use crate::{UiDocument, UiSize, WidgetAction};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

/// Owns browser callback handles only. Repaint policy stays in RuntimeSession.
pub(super) struct WebFrameDriver {
    start: web_time::Instant,
    callback: RefCell<Option<Closure<dyn FnMut()>>>,
    pending: Cell<Option<WebWakeup>>,
    stopped: Cell<bool>,
    wake_callback: RefCell<Option<Closure<dyn FnMut()>>>,
    resize_observer: RefCell<Option<web_sys::ResizeObserver>>,
    resolution_observer: RefCell<Option<(f64, web_sys::MediaQueryList)>>,
}

#[derive(Clone, Copy)]
enum WebWakeup {
    Frame(i32),
    Timer { id: i32, deadline: Duration },
}

impl WebFrameDriver {
    pub(super) fn new() -> Self {
        Self {
            start: web_time::Instant::now(),
            callback: RefCell::new(None),
            pending: Cell::new(None),
            stopped: Cell::new(false),
            wake_callback: RefCell::new(None),
            resize_observer: RefCell::new(None),
            resolution_observer: RefCell::new(None),
        }
    }

    pub(super) fn now(&self) -> Duration {
        self.start.elapsed()
    }

    pub(super) fn is_stopped(&self) -> bool {
        self.stopped.get()
    }

    pub(super) fn observe_canvas(
        self: &Rc<Self>,
        canvas: &web_sys::HtmlCanvasElement,
    ) -> Result<(), JsValue> {
        let weak = Rc::downgrade(self);
        let callback = Closure::<dyn FnMut()>::wrap(Box::new(move || {
            if let Some(driver) = weak.upgrade() {
                driver.wake();
            }
        }));
        let window = browser_window()?;
        window.add_event_listener_with_callback("resize", callback.as_ref().unchecked_ref())?;
        let observer = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref())?;
        observer.observe(canvas);
        *self.resize_observer.borrow_mut() = Some(observer);
        *self.wake_callback.borrow_mut() = Some(callback);
        self.watch_resolution()
    }

    fn watch_resolution(&self) -> Result<(), JsValue> {
        let window = browser_window()?;
        let ratio = window.device_pixel_ratio();
        if self
            .resolution_observer
            .borrow()
            .as_ref()
            .is_some_and(|(previous, _)| *previous == ratio)
        {
            return Ok(());
        }
        let callback = self.wake_callback.borrow();
        let Some(callback) = callback.as_ref() else {
            return Ok(());
        };
        if let Some((_, previous)) = self.resolution_observer.borrow_mut().take() {
            previous
                .remove_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
        }
        if let Some(query) = window.match_media(&format!("(resolution: {ratio}dppx)"))? {
            query.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
            *self.resolution_observer.borrow_mut() = Some((ratio, query));
        }
        Ok(())
    }

    pub(super) fn wake(&self) {
        if let Err(error) = self.schedule(Some(Duration::ZERO)) {
            web_sys::console::error_1(&error);
        }
    }

    fn schedule(&self, delay: Option<Duration>) -> Result<(), JsValue> {
        if self.is_stopped() {
            return Ok(());
        }
        let Some(delay) = delay else {
            return Ok(());
        };
        let deadline = self.now().saturating_add(delay);
        match self.pending.get() {
            Some(WebWakeup::Frame(_)) => return Ok(()),
            Some(WebWakeup::Timer {
                deadline: pending, ..
            }) if pending <= deadline => return Ok(()),
            _ => {}
        }
        let window = browser_window()?;
        if let Some(WebWakeup::Timer { id, .. }) = self.pending.take() {
            window.clear_timeout_with_handle(id);
        }
        let callback = self.callback.borrow();
        let Some(callback) = callback.as_ref() else {
            return Ok(());
        };
        let next = if delay.is_zero() {
            WebWakeup::Frame(window.request_animation_frame(callback.as_ref().unchecked_ref())?)
        } else {
            // Round up so sub-millisecond remainders cannot produce a busy timer loop.
            let millis = (delay.as_secs_f64() * 1000.0).ceil().min(i32::MAX as f64) as i32;
            WebWakeup::Timer {
                id: window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    callback.as_ref().unchecked_ref(),
                    millis,
                )?,
                deadline,
            }
        };
        self.pending.set(Some(next));
        Ok(())
    }
    pub(super) fn stop(&self) {
        if self.stopped.replace(true) {
            return;
        }
        // Keep closures alive: a stop may be requested from a browser callback.
        if let Ok(window) = browser_window() {
            if let Some(observer) = self.resize_observer.borrow_mut().take() {
                observer.disconnect();
            }
            if let Some(callback) = self.wake_callback.borrow().as_ref() {
                let _ = window.remove_event_listener_with_callback(
                    "resize",
                    callback.as_ref().unchecked_ref(),
                );
                if let Some((_, query)) = self.resolution_observer.borrow_mut().take() {
                    let _ = query.remove_event_listener_with_callback(
                        "change",
                        callback.as_ref().unchecked_ref(),
                    );
                }
            }
            match self.pending.take() {
                Some(WebWakeup::Frame(id)) => {
                    let _ = window.cancel_animation_frame(id);
                }
                Some(WebWakeup::Timer { id, .. }) => window.clear_timeout_with_handle(id),
                None => {}
            }
        }
    }
}

impl Drop for WebFrameDriver {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(super) fn start_animation_loop<State, Update, View>(
    app: Rc<RefCell<WebRuntimeApp<State, Update, View>>>,
) -> Result<(), JsValue>
where
    State: 'static,
    Update: FnMut(&mut State, WidgetAction) + 'static,
    View: FnMut(&State, UiSize, &mut crate::runtime::ViewContext<'_>) -> UiDocument + 'static,
{
    let driver = app.borrow().frame_driver.clone();
    let callback_driver = Rc::downgrade(&driver);
    let callback_app = Rc::downgrade(&app);
    *driver.callback.borrow_mut() = Some(Closure::<dyn FnMut()>::wrap(Box::new(move || {
        let (Some(driver), Some(app)) = (callback_driver.upgrade(), callback_app.upgrade()) else {
            return;
        };
        if driver.is_stopped() {
            return;
        }
        if matches!(driver.pending.take(), Some(WebWakeup::Timer { .. })) {
            // Timers wake the host; presentation stays synchronized with the display.
            driver.wake();
            return;
        }
        let mut terminal_error = None;
        let delay = match app.try_borrow_mut() {
            Ok(mut app) => {
                let now = driver.now();
                // Input/resize wakeups can arrive before a surface retry is due.
                if app.session.frame_retry_delay(now).is_none() {
                    terminal_error = app.render().err();
                }
                if terminal_error.is_none() {
                    app.next_frame_delay()
                } else {
                    None
                }
            }
            Err(error) => {
                log_web_runtime_reentry("animation frame", &error);
                Some(Duration::ZERO)
            }
        };
        if let Some(error) = terminal_error {
            stop_web_frame_error(&app, &error);
            return;
        }
        if let Err(error) = driver.watch_resolution() {
            web_sys::console::error_1(&error);
        }
        if let Err(error) = driver.schedule(delay) {
            web_sys::console::error_1(&error);
        }
    })));
    driver.schedule(Some(Duration::ZERO))
}
