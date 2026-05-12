//! Button widget entry point.
//!
//! The button implementation currently lives in the core `widgets` module for
//! API compatibility. This module provides the per-widget path while preserving
//! the existing `widgets::button` function and related helpers.

pub use super::{
    button, button_actions_from_gesture_event, button_actions_from_input_result,
    button_actions_from_key_event, push_button_gesture_event_actions,
    push_button_input_result_actions, push_button_key_event_actions, ButtonOptions,
};
