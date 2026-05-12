//! Higher-level input device policy contracts.
//!
//! This module describes how backends should normalize touch, stylus, and
//! gamepad input before handing it to document/navigation code. It deliberately
//! stays data-oriented so hosts can adopt the contracts without coupling this
//! crate to any specific windowing backend.

use crate::{PointerId, UiNodeId, UiPoint};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchGesturePolicy {
    pub tap_max_distance: f32,
    pub tap_max_millis: u64,
    pub long_press_min_millis: u64,
    pub pan_min_distance: f32,
    pub pinch_min_scale_delta: f32,
    pub rotation_min_radians: f32,
    pub kinetic_min_velocity: f32,
}

impl Default for TouchGesturePolicy {
    fn default() -> Self {
        Self {
            tap_max_distance: 8.0,
            tap_max_millis: 250,
            long_press_min_millis: 500,
            pan_min_distance: 10.0,
            pinch_min_scale_delta: 0.08,
            rotation_min_radians: 0.12,
            kinetic_min_velocity: 0.35,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchGestureKind {
    Tap,
    LongPress,
    Pan,
    Pinch,
    Rotation,
    KineticHandoff,
    Cancellation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchGestureSample {
    pub contact_count: u8,
    pub origin: UiPoint,
    pub current: UiPoint,
    pub elapsed_millis: u64,
    pub scale: f32,
    pub rotation_radians: f32,
    pub velocity: UiPoint,
    pub cancelled: bool,
}

impl TouchGestureSample {
    pub const fn single(origin: UiPoint, current: UiPoint, elapsed_millis: u64) -> Self {
        Self {
            contact_count: 1,
            origin,
            current,
            elapsed_millis,
            scale: 1.0,
            rotation_radians: 0.0,
            velocity: UiPoint::new(0.0, 0.0),
            cancelled: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerCaptureInteraction {
    None,
    Acquire,
    RouteToCapture,
    Release,
    CancelAndRelease,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchGestureClassification {
    pub kind: TouchGestureKind,
    pub capture: PointerCaptureInteraction,
    pub target: Option<UiNodeId>,
}

pub fn classify_touch_gesture(
    sample: TouchGestureSample,
    captured_target: Option<UiNodeId>,
    hit_target: Option<UiNodeId>,
    policy: TouchGesturePolicy,
) -> TouchGestureClassification {
    let target = captured_target.or(hit_target);
    if sample.cancelled {
        return TouchGestureClassification {
            kind: TouchGestureKind::Cancellation,
            capture: if captured_target.is_some() {
                PointerCaptureInteraction::CancelAndRelease
            } else {
                PointerCaptureInteraction::None
            },
            target,
        };
    }

    let distance = point_distance(delta(sample.origin, sample.current));
    if sample.contact_count >= 2 {
        let scale_delta = (sample.scale - 1.0).abs();
        if scale_delta >= policy.pinch_min_scale_delta {
            return TouchGestureClassification {
                kind: TouchGestureKind::Pinch,
                capture: capture_for_active_gesture(captured_target),
                target,
            };
        }
        if sample.rotation_radians.abs() >= policy.rotation_min_radians {
            return TouchGestureClassification {
                kind: TouchGestureKind::Rotation,
                capture: capture_for_active_gesture(captured_target),
                target,
            };
        }
    }

    if distance >= policy.pan_min_distance {
        let speed = point_distance(sample.velocity);
        return TouchGestureClassification {
            kind: if speed >= policy.kinetic_min_velocity {
                TouchGestureKind::KineticHandoff
            } else {
                TouchGestureKind::Pan
            },
            capture: capture_for_active_gesture(captured_target),
            target,
        };
    }

    TouchGestureClassification {
        kind: if sample.elapsed_millis >= policy.long_press_min_millis {
            TouchGestureKind::LongPress
        } else {
            TouchGestureKind::Tap
        },
        capture: if captured_target.is_some() {
            PointerCaptureInteraction::RouteToCapture
        } else if sample.elapsed_millis <= policy.tap_max_millis
            && distance <= policy.tap_max_distance
        {
            PointerCaptureInteraction::Release
        } else {
            PointerCaptureInteraction::Acquire
        },
        target,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StylusContactPhase {
    Hover,
    Contact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StylusButton {
    Barrel,
    Eraser,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StylusMetadata {
    pub pointer_id: PointerId,
    pub phase: StylusContactPhase,
    pub pressure: Option<f32>,
    pub tilt_x_degrees: Option<f32>,
    pub tilt_y_degrees: Option<f32>,
    pub twist_degrees: Option<f32>,
    pub barrel_button: bool,
    pub eraser_button: bool,
}

impl StylusMetadata {
    pub const fn new(pointer_id: PointerId, phase: StylusContactPhase) -> Self {
        Self {
            pointer_id,
            phase,
            pressure: None,
            tilt_x_degrees: None,
            tilt_y_degrees: None,
            twist_degrees: None,
            barrel_button: false,
            eraser_button: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StylusMetadataSupport {
    pub pressure: bool,
    pub tilt: bool,
    pub twist: bool,
    pub barrel_button: bool,
    pub eraser_button: bool,
    pub hover: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StylusMetadataField {
    Pressure,
    Tilt,
    Twist,
    BarrelButton,
    EraserButton,
    Hover,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StylusRouteReport {
    pub metadata: StylusMetadata,
    pub rejected: bool,
    pub fallback_to_pointer: bool,
    pub unsupported: Vec<StylusMetadataField>,
}

pub fn route_stylus_metadata(
    metadata: StylusMetadata,
    support: StylusMetadataSupport,
) -> StylusRouteReport {
    let mut routed = metadata;
    let mut unsupported = Vec::new();

    if routed.pressure.is_some() && !support.pressure {
        routed.pressure = None;
        unsupported.push(StylusMetadataField::Pressure);
    }
    if (routed.tilt_x_degrees.is_some() || routed.tilt_y_degrees.is_some()) && !support.tilt {
        routed.tilt_x_degrees = None;
        routed.tilt_y_degrees = None;
        unsupported.push(StylusMetadataField::Tilt);
    }
    if routed.twist_degrees.is_some() && !support.twist {
        routed.twist_degrees = None;
        unsupported.push(StylusMetadataField::Twist);
    }
    if routed.barrel_button && !support.barrel_button {
        routed.barrel_button = false;
        unsupported.push(StylusMetadataField::BarrelButton);
    }
    if routed.eraser_button && !support.eraser_button {
        routed.eraser_button = false;
        unsupported.push(StylusMetadataField::EraserButton);
    }
    if routed.phase == StylusContactPhase::Hover && !support.hover {
        routed.phase = StylusContactPhase::Contact;
        unsupported.push(StylusMetadataField::Hover);
    }

    StylusRouteReport {
        metadata: routed,
        rejected: metadata.phase == StylusContactPhase::Hover && !support.hover,
        fallback_to_pointer: !unsupported.is_empty(),
        unsupported,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GamepadDeviceId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South,
    East,
    West,
    North,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Start,
    Select,
    Other(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    TriggerLeft,
    TriggerRight,
    Other(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadNavigationAction {
    NavigateUp,
    NavigateDown,
    NavigateLeft,
    NavigateRight,
    Activate,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GamepadNavigationPolicy {
    pub axis_deadzone: f32,
    pub repeat_initial_delay_millis: u64,
    pub repeat_interval_millis: u64,
}

impl Default for GamepadNavigationPolicy {
    fn default() -> Self {
        Self {
            axis_deadzone: 0.25,
            repeat_initial_delay_millis: 350,
            repeat_interval_millis: 90,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GamepadInput {
    Button {
        button: GamepadButton,
        pressed: bool,
        held_millis: u64,
        repeat_count: u32,
    },
    Axis {
        axis: GamepadAxis,
        value: f32,
        held_millis: u64,
        repeat_count: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GamepadNavigationReport {
    pub device_id: GamepadDeviceId,
    pub action: Option<GamepadNavigationAction>,
    pub focus_scope: Option<UiNodeId>,
    pub repeated: bool,
    pub ignored_by_deadzone: bool,
    pub suppressed_by_repeat: bool,
}

pub fn map_gamepad_navigation(
    device_id: GamepadDeviceId,
    input: GamepadInput,
    focus_scope: Option<UiNodeId>,
    policy: GamepadNavigationPolicy,
) -> GamepadNavigationReport {
    let (candidate_action, repeated, ignored_by_deadzone, suppressed_by_repeat) = match input {
        GamepadInput::Button {
            button,
            pressed,
            held_millis,
            repeat_count,
        } if pressed => (
            action_for_button(button),
            repeat_count > 0 && repeat_allowed(held_millis, repeat_count, policy),
            false,
            repeat_count > 0 && !repeat_allowed(held_millis, repeat_count, policy),
        ),
        GamepadInput::Button { .. } => (None, false, false, false),
        GamepadInput::Axis {
            axis,
            value,
            held_millis,
            repeat_count,
        } => {
            if value.abs() < policy.axis_deadzone {
                (None, false, true, false)
            } else {
                (
                    action_for_axis(axis, value),
                    repeat_count > 0 && repeat_allowed(held_millis, repeat_count, policy),
                    false,
                    repeat_count > 0 && !repeat_allowed(held_millis, repeat_count, policy),
                )
            }
        }
    };
    let action = if suppressed_by_repeat {
        None
    } else {
        candidate_action
    };

    GamepadNavigationReport {
        device_id,
        action,
        focus_scope,
        repeated,
        ignored_by_deadzone,
        suppressed_by_repeat,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CancelCaptureRouteReport {
    pub pointer_id: PointerId,
    pub target: Option<UiNodeId>,
    pub capture: PointerCaptureInteraction,
    pub delivered_cancel: bool,
}

pub fn route_cancel_for_capture(
    pointer_id: PointerId,
    captured_target: Option<UiNodeId>,
    hit_target: Option<UiNodeId>,
) -> CancelCaptureRouteReport {
    CancelCaptureRouteReport {
        pointer_id,
        target: captured_target.or(hit_target),
        capture: if captured_target.is_some() {
            PointerCaptureInteraction::CancelAndRelease
        } else {
            PointerCaptureInteraction::None
        },
        delivered_cancel: captured_target.or(hit_target).is_some(),
    }
}

fn capture_for_active_gesture(captured_target: Option<UiNodeId>) -> PointerCaptureInteraction {
    if captured_target.is_some() {
        PointerCaptureInteraction::RouteToCapture
    } else {
        PointerCaptureInteraction::Acquire
    }
}

const fn delta(from: UiPoint, to: UiPoint) -> UiPoint {
    UiPoint::new(to.x - from.x, to.y - from.y)
}

fn point_distance(delta: UiPoint) -> f32 {
    (delta.x * delta.x + delta.y * delta.y).sqrt()
}

fn action_for_button(button: GamepadButton) -> Option<GamepadNavigationAction> {
    match button {
        GamepadButton::DPadUp => Some(GamepadNavigationAction::NavigateUp),
        GamepadButton::DPadDown => Some(GamepadNavigationAction::NavigateDown),
        GamepadButton::DPadLeft => Some(GamepadNavigationAction::NavigateLeft),
        GamepadButton::DPadRight => Some(GamepadNavigationAction::NavigateRight),
        GamepadButton::South => Some(GamepadNavigationAction::Activate),
        GamepadButton::East => Some(GamepadNavigationAction::Cancel),
        _ => None,
    }
}

fn action_for_axis(axis: GamepadAxis, value: f32) -> Option<GamepadNavigationAction> {
    match (axis, value.is_sign_positive()) {
        (GamepadAxis::LeftX, true) => Some(GamepadNavigationAction::NavigateRight),
        (GamepadAxis::LeftX, false) => Some(GamepadNavigationAction::NavigateLeft),
        (GamepadAxis::LeftY, true) => Some(GamepadNavigationAction::NavigateDown),
        (GamepadAxis::LeftY, false) => Some(GamepadNavigationAction::NavigateUp),
        _ => None,
    }
}

const fn repeat_allowed(
    held_millis: u64,
    repeat_count: u32,
    policy: GamepadNavigationPolicy,
) -> bool {
    if repeat_count == 0 {
        return true;
    }
    held_millis >= policy.repeat_initial_delay_millis
        && (held_millis - policy.repeat_initial_delay_millis)
            >= policy.repeat_interval_millis * (repeat_count.saturating_sub(1) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET: UiNodeId = UiNodeId(7);
    const CAPTURED: UiNodeId = UiNodeId(9);

    #[test]
    fn touch_gesture_classification_covers_tap_pan_pinch_rotation_and_kinetic() {
        let policy = TouchGesturePolicy::default();

        let tap = classify_touch_gesture(
            TouchGestureSample::single(UiPoint::new(0.0, 0.0), UiPoint::new(2.0, 1.0), 100),
            None,
            Some(TARGET),
            policy,
        );
        assert_eq!(tap.kind, TouchGestureKind::Tap);
        assert_eq!(tap.capture, PointerCaptureInteraction::Release);

        let long_press = classify_touch_gesture(
            TouchGestureSample::single(UiPoint::new(0.0, 0.0), UiPoint::new(1.0, 0.0), 600),
            None,
            Some(TARGET),
            policy,
        );
        assert_eq!(long_press.kind, TouchGestureKind::LongPress);
        assert_eq!(long_press.capture, PointerCaptureInteraction::Acquire);

        let pan = classify_touch_gesture(
            TouchGestureSample::single(UiPoint::new(0.0, 0.0), UiPoint::new(20.0, 0.0), 180),
            None,
            Some(TARGET),
            policy,
        );
        assert_eq!(pan.kind, TouchGestureKind::Pan);
        assert_eq!(pan.capture, PointerCaptureInteraction::Acquire);

        let pinch = classify_touch_gesture(
            TouchGestureSample {
                contact_count: 2,
                scale: 1.2,
                ..TouchGestureSample::single(UiPoint::new(0.0, 0.0), UiPoint::new(1.0, 1.0), 120)
            },
            Some(CAPTURED),
            Some(TARGET),
            policy,
        );
        assert_eq!(pinch.kind, TouchGestureKind::Pinch);
        assert_eq!(pinch.capture, PointerCaptureInteraction::RouteToCapture);
        assert_eq!(pinch.target, Some(CAPTURED));

        let rotation = classify_touch_gesture(
            TouchGestureSample {
                contact_count: 2,
                rotation_radians: 0.2,
                ..TouchGestureSample::single(UiPoint::new(0.0, 0.0), UiPoint::new(1.0, 1.0), 120)
            },
            None,
            Some(TARGET),
            policy,
        );
        assert_eq!(rotation.kind, TouchGestureKind::Rotation);

        let kinetic = classify_touch_gesture(
            TouchGestureSample {
                velocity: UiPoint::new(0.5, 0.0),
                ..TouchGestureSample::single(UiPoint::new(0.0, 0.0), UiPoint::new(30.0, 0.0), 120)
            },
            None,
            Some(TARGET),
            policy,
        );
        assert_eq!(kinetic.kind, TouchGestureKind::KineticHandoff);
    }

    #[test]
    fn stylus_metadata_preserves_supported_fields_and_falls_back_for_unsupported() {
        let metadata = StylusMetadata {
            pressure: Some(0.7),
            tilt_x_degrees: Some(12.0),
            tilt_y_degrees: Some(-4.0),
            twist_degrees: Some(30.0),
            barrel_button: true,
            eraser_button: true,
            ..StylusMetadata::new(PointerId(42), StylusContactPhase::Hover)
        };

        let full = route_stylus_metadata(
            metadata,
            StylusMetadataSupport {
                pressure: true,
                tilt: true,
                twist: true,
                barrel_button: true,
                eraser_button: true,
                hover: true,
            },
        );
        assert_eq!(full.metadata, metadata);
        assert!(!full.fallback_to_pointer);
        assert!(full.unsupported.is_empty());

        let fallback = route_stylus_metadata(
            metadata,
            StylusMetadataSupport {
                pressure: true,
                barrel_button: true,
                ..Default::default()
            },
        );
        assert_eq!(fallback.metadata.pressure, Some(0.7));
        assert_eq!(fallback.metadata.tilt_x_degrees, None);
        assert_eq!(fallback.metadata.twist_degrees, None);
        assert!(fallback.metadata.barrel_button);
        assert!(!fallback.metadata.eraser_button);
        assert_eq!(fallback.metadata.phase, StylusContactPhase::Contact);
        assert!(fallback.rejected);
        assert!(fallback.fallback_to_pointer);
        assert_eq!(
            fallback.unsupported,
            vec![
                StylusMetadataField::Tilt,
                StylusMetadataField::Twist,
                StylusMetadataField::EraserButton,
                StylusMetadataField::Hover
            ]
        );
    }

    #[test]
    fn gamepad_navigation_applies_deadzone_repeat_and_focus_scope() {
        let policy = GamepadNavigationPolicy {
            axis_deadzone: 0.3,
            repeat_initial_delay_millis: 300,
            repeat_interval_millis: 100,
        };
        let device = GamepadDeviceId(3);

        let deadzone = map_gamepad_navigation(
            device,
            GamepadInput::Axis {
                axis: GamepadAxis::LeftX,
                value: 0.2,
                held_millis: 0,
                repeat_count: 0,
            },
            Some(TARGET),
            policy,
        );
        assert_eq!(deadzone.action, None);
        assert!(deadzone.ignored_by_deadzone);

        let initial = map_gamepad_navigation(
            device,
            GamepadInput::Axis {
                axis: GamepadAxis::LeftY,
                value: -0.8,
                held_millis: 0,
                repeat_count: 0,
            },
            Some(TARGET),
            policy,
        );
        assert_eq!(initial.action, Some(GamepadNavigationAction::NavigateUp));
        assert_eq!(initial.focus_scope, Some(TARGET));
        assert!(!initial.repeated);
        assert!(!initial.suppressed_by_repeat);

        let too_early = map_gamepad_navigation(
            device,
            GamepadInput::Button {
                button: GamepadButton::DPadRight,
                pressed: true,
                held_millis: 350,
                repeat_count: 2,
            },
            Some(TARGET),
            policy,
        );
        assert_eq!(too_early.action, None);
        assert!(!too_early.repeated);
        assert!(too_early.suppressed_by_repeat);

        let repeat = map_gamepad_navigation(
            device,
            GamepadInput::Button {
                button: GamepadButton::South,
                pressed: true,
                held_millis: 400,
                repeat_count: 2,
            },
            Some(TARGET),
            policy,
        );
        assert_eq!(repeat.action, Some(GamepadNavigationAction::Activate));
        assert!(repeat.repeated);
        assert!(!repeat.suppressed_by_repeat);
    }

    #[test]
    fn cancel_and_capture_routing_reports_release_target() {
        let policy = TouchGesturePolicy::default();
        let cancelled = classify_touch_gesture(
            TouchGestureSample {
                cancelled: true,
                ..TouchGestureSample::single(UiPoint::new(0.0, 0.0), UiPoint::new(4.0, 0.0), 20)
            },
            Some(CAPTURED),
            Some(TARGET),
            policy,
        );
        assert_eq!(cancelled.kind, TouchGestureKind::Cancellation);
        assert_eq!(cancelled.target, Some(CAPTURED));
        assert_eq!(
            cancelled.capture,
            PointerCaptureInteraction::CancelAndRelease
        );

        let report = route_cancel_for_capture(PointerId(11), Some(CAPTURED), Some(TARGET));
        assert_eq!(report.pointer_id, PointerId(11));
        assert_eq!(report.target, Some(CAPTURED));
        assert_eq!(report.capture, PointerCaptureInteraction::CancelAndRelease);
        assert!(report.delivered_cancel);

        let uncaptured = route_cancel_for_capture(PointerId(12), None, None);
        assert_eq!(uncaptured.capture, PointerCaptureInteraction::None);
        assert!(!uncaptured.delivered_cancel);
    }
}
