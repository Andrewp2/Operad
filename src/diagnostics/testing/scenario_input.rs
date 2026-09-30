//! Scenario input uses the same gesture and document routing as runtime hosts.

use super::*;
use crate::host::{
    process_document_input_event, process_document_input_with_filter, HostFrameRequest,
    HostInputEvent,
};
use crate::input::{
    PointerButton, PointerEventKind, RawKeyboardEvent, RawPointerEvent, RawTextInputEvent,
    RawWheelEvent,
};

impl ScenarioHarness {
    pub(super) fn process_replay(
        &mut self,
        document: &mut UiDocument,
        replay: &EventReplay,
        measurer: &mut impl TextMeasurer,
    ) -> Result<(HostFrameOutput, EventReplayReport), taffy::TaffyError> {
        let mut output = HostFrameOutput::new(self.state.interaction.clone());
        let mut steps = Vec::with_capacity(replay.steps.len());
        for step in &replay.steps {
            self.replay_time_millis = self.replay_time_millis.saturating_add(1);
            let event_start = output.events.len();
            let raw = match &step.input {
                ReplayInput::Raw {
                    event,
                    line_size,
                    page_size,
                } => {
                    if let Some(timestamp) = raw_timestamp(event) {
                        self.replay_time_millis = self.replay_time_millis.max(timestamp);
                    }
                    Some((event.clone(), (*line_size, *page_size)))
                }
                ReplayInput::Ui(event) => {
                    if let Some(raw) =
                        raw_from_ui(event, document.pointer_position, self.replay_time_millis)
                    {
                        // UiWheelEvent already carries pixel deltas. Keep its
                        // original unit metadata without converting a second time.
                        Some((raw, (1.0, UiSize::new(1.0, 1.0))))
                    } else {
                        // A normalized composition already names its document
                        // owner; raw composition instead resolves an IME session.
                        document.compute_layout(self.viewport, measurer)?;
                        let previous_focus = document.focus_state().clone();
                        let mut event = HostInputEvent::from(event.clone());
                        process_document_input_event(
                            document,
                            &mut event,
                            &mut output.state,
                            previous_focus,
                        );
                        output.events.push(event);
                        None
                    }
                }
                ReplayInput::WindowResize(viewport) => {
                    self.viewport = *viewport;
                    self.target = render_target_with_viewport(&self.target, *viewport);
                    None
                }
                ReplayInput::PlatformResponse(response) => {
                    output.platform_responses.push(response.clone());
                    None
                }
                ReplayInput::Command(_) => None,
            };
            if let Some((raw, scale)) = raw {
                let request =
                    HostFrameRequest::new(self.viewport, std::mem::take(&mut output.state))
                        .raw_event(raw);
                let next = process_document_input_with_filter(
                    document,
                    measurer,
                    request,
                    scale,
                    |_, _, _| true,
                )?;
                output.state = next.state;
                output.events.extend(next.events);
                output.commands.extend(next.commands);
                output.platform_requests.extend(next.platform_requests);
                output.platform_responses.extend(next.platform_responses);
            }
            let applied = &output.events[event_start..];
            steps.push(EventReplayStepResult {
                label: step.label.clone(),
                input: step.input.clone(),
                converted: applied
                    .iter()
                    .filter_map(|event| event.ui_event.clone())
                    .collect(),
                platform_response: replay_input_to_platform_response(&step.input),
                viewport_resize: replay_input_to_viewport_resize(&step.input),
                results: applied
                    .iter()
                    .filter_map(|event| event.document_result.as_ref()?.input.clone())
                    .collect(),
            });
        }
        Ok((output, EventReplayReport { steps }))
    }
}

fn raw_timestamp(event: &RawInputEvent) -> Option<u64> {
    match event {
        RawInputEvent::Pointer(event) => Some(event.timestamp_millis),
        RawInputEvent::Wheel(event) => Some(event.timestamp_millis),
        RawInputEvent::Keyboard(event) => Some(event.timestamp_millis),
        RawInputEvent::Text(event) => Some(event.timestamp_millis),
        RawInputEvent::Composition(event) => Some(event.timestamp_millis),
        RawInputEvent::Focus(_) => None,
    }
}

fn raw_from_ui(
    event: &UiInputEvent,
    pointer: Option<UiPoint>,
    timestamp: u64,
) -> Option<RawInputEvent> {
    let pointer_event =
        |kind, point| RawInputEvent::Pointer(RawPointerEvent::new(kind, point, timestamp));
    Some(match event {
        UiInputEvent::PointerMove(point) => pointer_event(PointerEventKind::Move, *point),
        UiInputEvent::PointerDown(point) => {
            pointer_event(PointerEventKind::Down(PointerButton::Primary), *point)
        }
        UiInputEvent::PointerUp(point) => {
            pointer_event(PointerEventKind::Up(PointerButton::Primary), *point)
        }
        UiInputEvent::PointerCancel => pointer_event(
            PointerEventKind::Cancel,
            pointer.unwrap_or(UiPoint::new(0.0, 0.0)),
        ),
        UiInputEvent::Wheel(event) => RawInputEvent::Wheel(RawWheelEvent {
            position: event.position,
            delta: event.delta,
            unit: event.unit,
            phase: event.phase,
            modifiers: event.modifiers,
            timestamp_millis: timestamp,
        }),
        UiInputEvent::Key { key, modifiers } => {
            RawInputEvent::Keyboard(RawKeyboardEvent::press(*key, *modifiers, timestamp))
        }
        UiInputEvent::TextInput(text) => {
            RawInputEvent::Text(RawTextInputEvent::new(text.clone(), timestamp))
        }
        UiInputEvent::Focus(direction) => RawInputEvent::Focus(*direction),
        UiInputEvent::Composition { .. } => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{GestureEvent, WheelDeltaUnit};
    use crate::{InputBehavior, LayoutStyle, WidgetActionKind};

    fn button_document() -> (UiDocument, UiNodeId) {
        let mut document = UiDocument::new(LayoutStyle::size(180.0, 100.0));
        let button = document.add_child(
            document.root(),
            UiNode::container("button", LayoutStyle::size(96.0, 32.0))
                .with_input(InputBehavior::BUTTON)
                .with_action("activate"),
        );
        (document, button)
    }

    #[test]
    fn scenario_pointer_gestures_and_step_results_survive_every_frame_partition() {
        for raw in [false, true] {
            for moves in [false, true] {
                for distance in [0.0, 20.0] {
                    let start = UiPoint::new(8.0, 8.0);
                    let end = UiPoint::new(start.x + distance, start.y);
                    let mut replay = EventReplay::new().pointer_down("down", start).raw(
                        "unmatched release",
                        RawInputEvent::Pointer(RawPointerEvent::new(
                            PointerEventKind::Up(PointerButton::Secondary),
                            start,
                            2,
                        )),
                    );
                    if moves {
                        replay = replay.pointer_move("move", end);
                    }
                    replay = replay.pointer_up("up", end);
                    if raw {
                        for (i, step) in replay.steps.iter_mut().enumerate() {
                            if let ReplayInput::Ui(event) = &step.input {
                                step.input = ReplayInput::Raw {
                                    event: raw_from_ui(event, None, i as u64 + 1).unwrap(),
                                    line_size: 16.0,
                                    page_size: UiSize::new(180.0, 100.0),
                                };
                            }
                        }
                    }
                    for partition in 0..(1 << (replay.steps.len() - 1)) {
                        let (mut document, button) = button_document();
                        let mut harness = ScenarioHarness::new(UiSize::new(180.0, 100.0));
                        let mut pending = EventReplay::new();
                        let mut clicked = Vec::new();
                        let mut activations = Vec::new();
                        for (i, step) in replay.steps.iter().enumerate() {
                            pending.steps.push(step.clone());
                            if i + 1 == replay.steps.len() || partition & (1 << i) != 0 {
                                let report = harness
                                    .run_frame(
                                        "gesture",
                                        &mut document,
                                        std::mem::take(&mut pending),
                                    )
                                    .unwrap();
                                for step in &report.events.steps {
                                    if step.label == "unmatched release" {
                                        assert!(
                                            step.converted.is_empty() && step.results.is_empty(),
                                            "ignored input borrowed another step's result"
                                        );
                                    }
                                    if step.label == "up" {
                                        assert_eq!(step.results.len(), 1);
                                        assert_eq!(
                                            step.results[0].clicked,
                                            (distance == 0.0).then_some(button)
                                        );
                                    }
                                }
                                clicked.extend(report.events.clicked_nodes());
                                activations.extend(
                                    crate::host::collect_document_widget_actions(&report.document)
                                        .into_iter()
                                        .filter(|action| {
                                            matches!(action.kind, WidgetActionKind::Activate(_))
                                        })
                                        .map(|action| action.target),
                                );
                            }
                        }
                        let expected: Vec<_> =
                            (distance == 0.0).then_some(button).into_iter().collect();
                        assert_eq!(
                            clicked, expected,
                            "raw={raw}, moves={moves}, distance={distance}, partition={partition}"
                        );
                        assert_eq!(activations, expected);
                        assert_eq!(harness.current_state().interaction.pressed, None);
                        assert_eq!(harness.current_state().interaction.drag_capture, None);
                    }
                }
            }
        }
    }

    #[test]
    fn scenario_resize_applies_between_events_and_preserves_step_attribution() {
        let mut document = UiDocument::new(
            LayoutStyle::row()
                .with_width_percent(1.0)
                .with_height_percent(1.0),
        );
        let buttons = ["left", "right"].map(|name| {
            document.add_child(
                document.root(),
                UiNode::container(
                    name,
                    LayoutStyle::new().with_width_percent(0.5).with_height(32.0),
                )
                .with_input(InputBehavior::BUTTON),
            )
        });
        let point = UiPoint::new(150.0, 16.0);
        let mut harness = ScenarioHarness::new(UiSize::new(200.0, 100.0));
        let report = harness
            .run_frame(
                "resize",
                &mut document,
                EventReplay::new()
                    .pointer_click("before", point)
                    .window_resize("resize", UiSize::new(400.0, 100.0))
                    .pointer_click("after", point),
            )
            .unwrap();
        assert_eq!(
            report.events.step("before.up").unwrap().results[0].clicked,
            Some(buttons[1])
        );
        assert!(report.events.step("resize").unwrap().results.is_empty());
        assert_eq!(
            report.events.step("after.up").unwrap().results[0].clicked,
            Some(buttons[0])
        );
        assert_eq!(report.viewport(), UiSize::new(400.0, 100.0));
    }

    #[test]
    fn scenario_scaled_wheel_keeps_units_and_updates_the_next_click_geometry() {
        let point = UiPoint::new(10.0, 10.0);
        let replays = [
            EventReplay::new().raw_scaled(
                "wheel",
                RawInputEvent::Wheel(RawWheelEvent::lines(point, UiPoint::new(0.0, 2.0), 100)),
                10.0,
                UiSize::new(100.0, 40.0),
            ),
            EventReplay::new().raw_scaled(
                "wheel",
                RawInputEvent::Wheel(RawWheelEvent::pages(point, UiPoint::new(0.0, 1.0), 100)),
                16.0,
                UiSize::new(100.0, 20.0),
            ),
            EventReplay::new().ui(
                "wheel",
                UiInputEvent::Wheel(
                    crate::UiWheelEvent::pixels(point, UiPoint::new(0.0, 20.0))
                        .unit(WheelDeltaUnit::Page),
                ),
            ),
        ];
        for (replay, unit) in replays.into_iter().zip([
            WheelDeltaUnit::Line,
            WheelDeltaUnit::Page,
            WheelDeltaUnit::Page,
        ]) {
            let mut document = UiDocument::new(LayoutStyle::size(180.0, 100.0));
            let scroll = document.add_child(
                document.root(),
                UiNode::container("scroll", LayoutStyle::column().with_size(100.0, 40.0))
                    .with_scroll(crate::ScrollAxes::VERTICAL),
            );
            let rows: Vec<_> = (0..8)
                .map(|i| {
                    document.add_child(
                        scroll,
                        UiNode::container(
                            format!("row.{i}"),
                            LayoutStyle::size(50.0, 20.0).with_flex_shrink(0.0),
                        )
                        .with_input(InputBehavior::BUTTON),
                    )
                })
                .collect();
            let mut harness = ScenarioHarness::new(UiSize::new(180.0, 100.0));
            let report = harness
                .run_frame(
                    "wheel then click",
                    &mut document,
                    replay.pointer_click("click", point),
                )
                .unwrap();
            let step = report.events.step("wheel").unwrap();
            let UiInputEvent::Wheel(wheel) = &step.converted[0] else {
                panic!("missing wheel")
            };
            assert_eq!(wheel.delta, UiPoint::new(0.0, 20.0));
            assert_eq!(wheel.unit, unit);
            assert_eq!(step.results[0].scrolled, Some(scroll));
            assert_eq!(report.events.clicked_nodes(), vec![rows[1]]);
        }
    }

    #[test]
    fn scenario_synthetic_clock_continues_after_raw_events_and_between_frames() {
        let (mut document, _) = button_document();
        let point = UiPoint::new(8.0, 8.0);
        let mut harness = ScenarioHarness::new(UiSize::new(180.0, 100.0));
        for (step, expected_count) in [1, 2, 1].into_iter().enumerate() {
            if step == 2 {
                harness.replay_time_millis += 1000;
            }
            let replay = if step == 0 {
                EventReplay::new()
                    .raw(
                        "down",
                        RawInputEvent::Pointer(RawPointerEvent::new(
                            PointerEventKind::Down(PointerButton::Primary),
                            point,
                            10000,
                        )),
                    )
                    .raw(
                        "up",
                        RawInputEvent::Pointer(RawPointerEvent::new(
                            PointerEventKind::Up(PointerButton::Primary),
                            point,
                            10010,
                        )),
                    )
            } else {
                EventReplay::new().pointer_click("click", point)
            };
            let report = harness.run_frame("clock", &mut document, replay).unwrap();
            let counts: Vec<_> = report
                .document
                .host_output
                .gestures()
                .filter_map(|gesture| match gesture {
                    GestureEvent::Click(click) => Some(click.count),
                    _ => None,
                })
                .collect();
            assert_eq!(counts, vec![expected_count]);
        }
    }
}
