//! Renderer-neutral testing helpers for Operad documents.
//!
//! These utilities are intended for consumers as well as Operad's own tests:
//! replay input without an egui harness, assert layout by stable node names,
//! inspect paint lists, diff rgba snapshots with tolerances, and track simple
//! frame timing sections.

use std::fmt;
use std::time::Duration;

use crate::platform::{
    ClipboardResponse, CursorResponse, DragDropResponse, FileDialogResponse, NotificationResponse,
    OpenUrlResponse, PlatformResponse, PlatformServiceError, PlatformServiceKind,
    PlatformServiceRequest, PlatformServiceResponse, RepaintResponse, ScreenshotResponse,
    TextImeResponse,
};
use crate::{
    HostFrameOutput, PaintItem, PaintKind, PaintList, RawInputEvent, UiDocument, UiInputEvent,
    UiInputResult, UiNode, UiNodeId, UiRect, UiSize,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestFailure {
    pub message: String,
}

impl TestFailure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TestFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TestFailure {}

pub type TestResult<T = ()> = Result<T, TestFailure>;

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayInput {
    Ui(UiInputEvent),
    Raw {
        event: RawInputEvent,
        line_size: f32,
        page_size: UiSize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventReplayStep {
    pub label: String,
    pub input: ReplayInput,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventReplayStepResult {
    pub label: String,
    pub input: ReplayInput,
    pub converted: Option<UiInputEvent>,
    pub result: Option<UiInputResult>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventReplay {
    pub steps: Vec<EventReplayStep>,
}

impl EventReplay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(mut self, label: impl Into<String>, event: UiInputEvent) -> Self {
        self.steps.push(EventReplayStep {
            label: label.into(),
            input: ReplayInput::Ui(event),
        });
        self
    }

    pub fn raw(mut self, label: impl Into<String>, event: RawInputEvent) -> Self {
        self.steps.push(EventReplayStep {
            label: label.into(),
            input: ReplayInput::Raw {
                event,
                line_size: 16.0,
                page_size: UiSize::new(800.0, 600.0),
            },
        });
        self
    }

    pub fn raw_scaled(
        mut self,
        label: impl Into<String>,
        event: RawInputEvent,
        line_size: f32,
        page_size: UiSize,
    ) -> Self {
        self.steps.push(EventReplayStep {
            label: label.into(),
            input: ReplayInput::Raw {
                event,
                line_size,
                page_size,
            },
        });
        self
    }

    pub fn run(&self, document: &mut UiDocument) -> EventReplayReport {
        let mut steps = Vec::with_capacity(self.steps.len());
        for step in &self.steps {
            let converted = match &step.input {
                ReplayInput::Ui(event) => Some(event.clone()),
                ReplayInput::Raw {
                    event,
                    line_size,
                    page_size,
                } => event.to_ui_input_event_with_wheel_scale(*line_size, *page_size),
            };
            let result = converted.clone().map(|event| document.handle_input(event));
            steps.push(EventReplayStepResult {
                label: step.label.clone(),
                input: step.input.clone(),
                converted,
                result,
            });
        }
        EventReplayReport { steps }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventReplayReport {
    pub steps: Vec<EventReplayStepResult>,
}

impl EventReplayReport {
    pub fn clicked_nodes(&self) -> Vec<UiNodeId> {
        self.steps
            .iter()
            .filter_map(|step| step.result.as_ref()?.clicked)
            .collect()
    }

    pub fn focused_nodes(&self) -> Vec<UiNodeId> {
        self.steps
            .iter()
            .filter_map(|step| step.result.as_ref()?.focused)
            .collect()
    }

    pub fn scrolled_nodes(&self) -> Vec<UiNodeId> {
        self.steps
            .iter()
            .filter_map(|step| step.result.as_ref()?.scrolled)
            .collect()
    }

    pub fn require_all_converted(&self) -> TestResult {
        if let Some(step) = self.steps.iter().find(|step| step.converted.is_none()) {
            Err(TestFailure::new(format!(
                "event replay step `{}` did not convert to UiInputEvent",
                step.label
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutAssertions<'a> {
    document: &'a UiDocument,
}

impl<'a> LayoutAssertions<'a> {
    pub const fn new(document: &'a UiDocument) -> Self {
        Self { document }
    }

    pub fn node(&self, name: &str) -> TestResult<(UiNodeId, &'a UiNode)> {
        self.document
            .nodes()
            .iter()
            .enumerate()
            .find(|(_, node)| node.name == name)
            .map(|(index, node)| (UiNodeId(index), node))
            .ok_or_else(|| TestFailure::new(format!("missing node `{name}`")))
    }

    pub fn rect(&self, name: &str) -> TestResult<UiRect> {
        Ok(self.node(name)?.1.layout.rect)
    }

    pub fn require_visible(&self, name: &str) -> TestResult {
        let (_, node) = self.node(name)?;
        if node.layout.visible {
            Ok(())
        } else {
            Err(TestFailure::new(format!("node `{name}` is not visible")))
        }
    }

    pub fn require_min_size(&self, name: &str, min_size: UiSize) -> TestResult {
        let rect = self.rect(name)?;
        if rect.width >= min_size.width && rect.height >= min_size.height {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{name}` expected at least {}x{}, got {}x{}",
                min_size.width, min_size.height, rect.width, rect.height
            )))
        }
    }

    pub fn require_contains(&self, outer: &str, inner: &str) -> TestResult {
        let outer_rect = self.rect(outer)?;
        let inner_rect = self.rect(inner)?;
        if outer_rect.contains_rect(inner_rect) {
            Ok(())
        } else {
            Err(TestFailure::new(format!(
                "node `{outer}` does not contain `{inner}`"
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaintKindSelector {
    Rect,
    Text,
    Canvas,
    Line,
    Circle,
    Polygon,
    Image,
    RichRect,
    SceneText,
    Path,
    ImagePlacement,
}

impl PaintKindSelector {
    pub const fn matches(self, kind: &PaintKind) -> bool {
        matches!(
            (self, kind),
            (Self::Rect, PaintKind::Rect { .. })
                | (Self::Text, PaintKind::Text(_))
                | (Self::Canvas, PaintKind::Canvas(_))
                | (Self::Line, PaintKind::Line { .. })
                | (Self::Circle, PaintKind::Circle { .. })
                | (Self::Polygon, PaintKind::Polygon { .. })
                | (Self::Image, PaintKind::Image { .. })
                | (Self::RichRect, PaintKind::RichRect(_))
                | (Self::SceneText, PaintKind::SceneText(_))
                | (Self::Path, PaintKind::Path(_))
                | (Self::ImagePlacement, PaintKind::ImagePlacement(_))
        )
    }
}

#[derive(Debug, Clone)]
pub struct PaintAssertions<'a> {
    document: &'a UiDocument,
    paint: PaintList,
}

impl<'a> PaintAssertions<'a> {
    pub fn new(document: &'a UiDocument) -> Self {
        Self {
            document,
            paint: document.paint_list(),
        }
    }

    pub fn paint(&self) -> &PaintList {
        &self.paint
    }

    pub fn count_kind(&self, selector: PaintKindSelector) -> usize {
        self.paint
            .items
            .iter()
            .filter(|item| selector.matches(&item.kind))
            .count()
    }

    pub fn require_node_kind(
        &self,
        node_name: &str,
        selector: PaintKindSelector,
    ) -> TestResult<&PaintItem> {
        let (id, _) = LayoutAssertions::new(self.document).node(node_name)?;
        self.paint
            .items
            .iter()
            .find(|item| item.node == id && selector.matches(&item.kind))
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "node `{node_name}` has no paint item of kind {selector:?}"
                ))
            })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlatformAssertions<'a> {
    requests: &'a [PlatformServiceRequest],
    responses: &'a [PlatformServiceResponse],
}

impl<'a> PlatformAssertions<'a> {
    pub const fn new(
        requests: &'a [PlatformServiceRequest],
        responses: &'a [PlatformServiceResponse],
    ) -> Self {
        Self {
            requests,
            responses,
        }
    }

    pub fn from_host_frame(output: &'a HostFrameOutput) -> Self {
        Self::new(&output.platform_requests, &output.platform_responses)
    }

    pub fn requests(&self) -> &'a [PlatformServiceRequest] {
        self.requests
    }

    pub fn responses(&self) -> &'a [PlatformServiceResponse] {
        self.responses
    }

    pub fn request_count(&self, kind: PlatformServiceKind) -> usize {
        self.requests
            .iter()
            .filter(|request| request.kind() == kind)
            .count()
    }

    pub fn response_count(&self, kind: PlatformServiceKind) -> usize {
        self.responses
            .iter()
            .filter(|response| response.kind() == kind)
            .count()
    }

    pub fn require_request_kind(
        &self,
        kind: PlatformServiceKind,
    ) -> TestResult<&'a PlatformServiceRequest> {
        self.requests
            .iter()
            .find(|request| request.kind() == kind)
            .ok_or_else(|| TestFailure::new(format!("missing platform request kind {kind:?}")))
    }

    pub fn require_response_kind(
        &self,
        kind: PlatformServiceKind,
    ) -> TestResult<&'a PlatformServiceResponse> {
        self.responses
            .iter()
            .find(|response| response.kind() == kind)
            .ok_or_else(|| TestFailure::new(format!("missing platform response kind {kind:?}")))
    }

    pub fn require_response_for(
        &self,
        request: &PlatformServiceRequest,
    ) -> TestResult<&'a PlatformServiceResponse> {
        self.responses
            .iter()
            .find(|response| response.is_for(request) && response.kind() == request.kind())
            .ok_or_else(|| {
                TestFailure::new(format!(
                    "missing {:?} response for platform request id {}",
                    request.kind(),
                    request.id.0
                ))
            })
    }

    pub fn require_all_responses_match_requests(&self) -> TestResult {
        for response in self.responses {
            if !self
                .requests
                .iter()
                .any(|request| response.is_for(request) && response.kind() == request.kind())
            {
                return Err(TestFailure::new(format!(
                    "platform response id {} kind {:?} has no matching request",
                    response.id.0,
                    response.kind()
                )));
            }
        }
        Ok(())
    }

    pub fn require_no_error_responses(&self) -> TestResult {
        if let Some((response, error)) = self.responses.iter().find_map(|response| {
            platform_response_error(&response.response).map(|error| (response, error))
        }) {
            Err(TestFailure::new(format!(
                "platform response id {} kind {:?} returned {:?}: {}",
                response.id.0,
                response.kind(),
                error.code,
                error.message
            )))
        } else {
            Ok(())
        }
    }
}

fn platform_response_error(response: &PlatformResponse) -> Option<&PlatformServiceError> {
    match response {
        PlatformResponse::Clipboard(ClipboardResponse::Error(error))
        | PlatformResponse::FileDialog(FileDialogResponse::Error(error))
        | PlatformResponse::OpenUrl(OpenUrlResponse::Error(error))
        | PlatformResponse::Notification(NotificationResponse::Error(error))
        | PlatformResponse::Screenshot(ScreenshotResponse::Error(error))
        | PlatformResponse::TextIme(TextImeResponse::Error(error))
        | PlatformResponse::DragDrop(DragDropResponse::Error(error))
        | PlatformResponse::Cursor(CursorResponse::Error(error))
        | PlatformResponse::Repaint(RepaintResponse::Error(error)) => Some(error),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbaImageView<'a> {
    pub width: usize,
    pub height: usize,
    pub pixels: &'a [u8],
}

impl<'a> RgbaImageView<'a> {
    pub fn new(width: usize, height: usize, pixels: &'a [u8]) -> TestResult<Self> {
        let expected_len = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| TestFailure::new("rgba image dimensions overflow"))?;
        if pixels.len() != expected_len {
            return Err(TestFailure::new(format!(
                "rgba image expected {expected_len} bytes, got {}",
                pixels.len()
            )));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelDiffTolerance {
    pub max_changed_pixels: usize,
    pub max_channel_delta: u8,
    pub max_total_channel_delta: u64,
}

impl PixelDiffTolerance {
    pub const EXACT: Self = Self {
        max_changed_pixels: 0,
        max_channel_delta: 0,
        max_total_channel_delta: 0,
    };
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PixelDiffReport {
    pub width: usize,
    pub height: usize,
    pub changed_pixels: usize,
    pub max_channel_delta: u8,
    pub total_channel_delta: u64,
}

impl PixelDiffReport {
    pub const fn is_within(self, tolerance: PixelDiffTolerance) -> bool {
        self.changed_pixels <= tolerance.max_changed_pixels
            && self.max_channel_delta <= tolerance.max_channel_delta
            && self.total_channel_delta <= tolerance.max_total_channel_delta
    }
}

pub fn diff_rgba8(
    expected: RgbaImageView<'_>,
    actual: RgbaImageView<'_>,
) -> TestResult<PixelDiffReport> {
    if expected.width != actual.width || expected.height != actual.height {
        return Err(TestFailure::new(format!(
            "rgba image dimensions differ: expected {}x{}, got {}x{}",
            expected.width, expected.height, actual.width, actual.height
        )));
    }

    let mut report = PixelDiffReport {
        width: expected.width,
        height: expected.height,
        ..Default::default()
    };

    for (expected, actual) in expected
        .pixels
        .chunks_exact(4)
        .zip(actual.pixels.chunks_exact(4))
    {
        let mut pixel_changed = false;
        for channel in 0..4 {
            let delta = expected[channel].abs_diff(actual[channel]);
            if delta > 0 {
                pixel_changed = true;
                report.max_channel_delta = report.max_channel_delta.max(delta);
                report.total_channel_delta += u64::from(delta);
            }
        }
        if pixel_changed {
            report.changed_pixels += 1;
        }
    }

    Ok(report)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirtyFlags {
    pub layout: bool,
    pub paint: bool,
    pub input: bool,
    pub theme: bool,
    pub text_measurement: bool,
}

impl DirtyFlags {
    pub const NONE: Self = Self {
        layout: false,
        paint: false,
        input: false,
        theme: false,
        text_measurement: false,
    };

    pub const ALL: Self = Self {
        layout: true,
        paint: true,
        input: true,
        theme: true,
        text_measurement: true,
    };

    pub const fn any(self) -> bool {
        self.layout || self.paint || self.input || self.theme || self.text_measurement
    }

    pub const fn union(self, other: Self) -> Self {
        Self {
            layout: self.layout || other.layout,
            paint: self.paint || other.paint,
            input: self.input || other.input,
            theme: self.theme || other.theme,
            text_measurement: self.text_measurement || other.text_measurement,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::NONE;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameTimingSection {
    pub name: String,
    pub duration: Duration,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameTiming {
    pub sections: Vec<FrameTimingSection>,
}

impl FrameTiming {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn section(mut self, name: impl Into<String>, duration: Duration) -> Self {
        self.sections.push(FrameTimingSection {
            name: name.into(),
            duration,
        });
        self
    }

    pub fn total(&self) -> Duration {
        self.sections.iter().map(|section| section.duration).sum()
    }

    pub fn duration(&self, name: &str) -> Option<Duration> {
        self.sections
            .iter()
            .find(|section| section.name == name)
            .map(|section| section.duration)
    }

    pub fn within_budget(&self, budget: Duration) -> bool {
        self.total() <= budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{
        ClipboardRequest, ClipboardResponse, PlatformErrorCode, PlatformRequest, PlatformRequestId,
        PlatformResponse, PlatformServiceError, PlatformServiceKind, RepaintRequest,
        RepaintResponse,
    };
    use crate::{
        length, root_style, ApproxTextMeasurer, ClipBehavior, ColorRgba, HostFrameOutput,
        HostInteractionState, ImageContent, InputBehavior, RawKeyboardEvent, RawPointerEvent,
        RawWheelEvent, StrokeStyle, TextStyle, UiNode, UiNodeStyle, UiPoint, UiVisual,
    };
    use taffy::prelude::{Dimension, Size as TaffySize, Style};

    fn fixed_style(width: f32, height: f32) -> UiNodeStyle {
        UiNodeStyle {
            layout: Style {
                size: TaffySize {
                    width: length(width),
                    height: length(height),
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn event_replay_runs_raw_and_document_events() {
        let mut document = UiDocument::new(root_style(180.0, 100.0));
        let root = document.root;
        let button = document.add_child(
            root,
            UiNode::container("play", fixed_style(80.0, 32.0)).with_input(InputBehavior::BUTTON),
        );
        document
            .compute_layout(UiSize::new(180.0, 100.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let report = EventReplay::new()
            .raw(
                "move",
                RawInputEvent::Pointer(RawPointerEvent::new(
                    crate::PointerEventKind::Move,
                    UiPoint::new(12.0, 12.0),
                    1,
                )),
            )
            .ui("down", UiInputEvent::PointerDown(UiPoint::new(12.0, 12.0)))
            .ui("up", UiInputEvent::PointerUp(UiPoint::new(12.0, 12.0)))
            .raw(
                "key",
                RawInputEvent::Keyboard(RawKeyboardEvent::press(
                    crate::KeyCode::Enter,
                    crate::KeyModifiers::NONE,
                    4,
                )),
            )
            .run(&mut document);

        assert_eq!(report.steps.len(), 4);
        assert_eq!(report.clicked_nodes(), vec![button]);
        assert_eq!(report.focused_nodes().last().copied(), Some(button));
        assert!(report.require_all_converted().is_ok());
    }

    #[test]
    fn layout_and_paint_assertions_use_stable_node_names() {
        let mut document = UiDocument::new(root_style(220.0, 120.0));
        let root = document.root;
        let panel = document.add_child(
            root,
            UiNode::container(
                "panel",
                UiNodeStyle {
                    clip: ClipBehavior::Clip,
                    ..fixed_style(140.0, 80.0)
                },
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(20, 24, 32, 255),
                Some(StrokeStyle::new(ColorRgba::new(80, 100, 120, 255), 1.0)),
                4.0,
            )),
        );
        document.add_child(
            panel,
            UiNode::image(
                "panel.icon",
                ImageContent::new("icons.play"),
                fixed_style(24.0, 24.0).layout,
            ),
        );
        document.add_child(
            panel,
            UiNode::text(
                "panel.label",
                "Play",
                TextStyle::default(),
                Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            ),
        );
        document
            .compute_layout(UiSize::new(220.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let layout = LayoutAssertions::new(&document);
        layout.require_visible("panel").expect("panel visible");
        layout
            .require_min_size("panel", UiSize::new(120.0, 60.0))
            .expect("panel minimum");
        layout
            .require_contains("panel", "panel.icon")
            .expect("panel contains icon");

        let paint = PaintAssertions::new(&document);
        assert!(paint.count_kind(PaintKindSelector::Rect) >= 1);
        paint
            .require_node_kind("panel.icon", PaintKindSelector::Image)
            .expect("icon paint");
        paint
            .require_node_kind("panel.label", PaintKindSelector::Text)
            .expect("text paint");
    }

    #[test]
    fn platform_assertions_match_requests_responses_and_errors() {
        let clipboard_id = PlatformRequestId::new(10);
        let repaint_id = PlatformRequestId::new(11);
        let output = HostFrameOutput::new(HostInteractionState::default())
            .request(
                clipboard_id,
                PlatformRequest::Clipboard(ClipboardRequest::ReadText),
            )
            .request(
                repaint_id,
                PlatformRequest::Repaint(RepaintRequest::NextFrame),
            )
            .response(
                clipboard_id,
                PlatformResponse::Clipboard(ClipboardResponse::Text(Some("copied".into()))),
            )
            .response(
                repaint_id,
                PlatformResponse::Repaint(RepaintResponse::Scheduled {
                    delay: Duration::from_millis(16),
                }),
            );

        let platform = PlatformAssertions::from_host_frame(&output);
        assert_eq!(platform.request_count(PlatformServiceKind::Clipboard), 1);
        assert_eq!(platform.response_count(PlatformServiceKind::Repaint), 1);
        let clipboard_request = platform
            .require_request_kind(PlatformServiceKind::Clipboard)
            .expect("clipboard request");
        let clipboard_response = platform
            .require_response_for(clipboard_request)
            .expect("clipboard response");
        assert!(matches!(
            clipboard_response.response,
            PlatformResponse::Clipboard(ClipboardResponse::Text(Some(_)))
        ));
        platform
            .require_all_responses_match_requests()
            .expect("matched responses");
        platform.require_no_error_responses().expect("no errors");

        let unmatched = HostFrameOutput::new(HostInteractionState::default()).response(
            PlatformRequestId::new(99),
            PlatformResponse::Repaint(RepaintResponse::Coalesced),
        );
        assert!(PlatformAssertions::from_host_frame(&unmatched)
            .require_all_responses_match_requests()
            .is_err());

        let error_output = HostFrameOutput::new(HostInteractionState::default())
            .request(
                clipboard_id,
                PlatformRequest::Clipboard(ClipboardRequest::ReadText),
            )
            .response(
                clipboard_id,
                PlatformResponse::Clipboard(ClipboardResponse::Error(PlatformServiceError::new(
                    PlatformErrorCode::Denied,
                    "clipboard blocked",
                ))),
            );
        assert!(PlatformAssertions::from_host_frame(&error_output)
            .require_no_error_responses()
            .is_err());
    }

    #[test]
    fn pixel_diff_reports_tolerance_compatible_changes() {
        let expected = [0, 0, 0, 255, 10, 20, 30, 255];
        let actual = [0, 1, 0, 255, 12, 18, 31, 255];
        let report = diff_rgba8(
            RgbaImageView::new(2, 1, &expected).expect("expected view"),
            RgbaImageView::new(2, 1, &actual).expect("actual view"),
        )
        .expect("diff");

        assert_eq!(report.changed_pixels, 2);
        assert_eq!(report.max_channel_delta, 2);
        assert_eq!(report.total_channel_delta, 6);
        assert!(report.is_within(PixelDiffTolerance {
            max_changed_pixels: 2,
            max_channel_delta: 2,
            max_total_channel_delta: 6,
        }));
        assert!(!report.is_within(PixelDiffTolerance::EXACT));
    }

    #[test]
    fn dirty_flags_and_frame_timing_track_test_budget_state() {
        let flags = DirtyFlags {
            layout: true,
            input: true,
            ..DirtyFlags::NONE
        }
        .union(DirtyFlags {
            paint: true,
            ..DirtyFlags::NONE
        });
        assert!(flags.any());
        assert!(flags.layout && flags.paint && flags.input);

        let timing = FrameTiming::new()
            .section("layout", Duration::from_millis(3))
            .section("paint", Duration::from_millis(4))
            .section("render", Duration::from_millis(8));
        assert_eq!(timing.duration("paint"), Some(Duration::from_millis(4)));
        assert_eq!(timing.total(), Duration::from_millis(15));
        assert!(timing.within_budget(Duration::from_millis(16)));
        assert!(!timing.within_budget(Duration::from_millis(10)));
    }

    #[test]
    fn replay_reports_unconverted_raw_events() {
        let mut document = UiDocument::new(root_style(100.0, 50.0));
        document
            .compute_layout(UiSize::new(100.0, 50.0), &mut ApproxTextMeasurer)
            .expect("layout");
        let report = EventReplay::new()
            .raw_scaled(
                "wheel",
                RawInputEvent::Wheel(RawWheelEvent::lines(
                    UiPoint::new(1.0, 1.0),
                    UiPoint::new(0.0, 1.0),
                    1,
                )),
                20.0,
                UiSize::new(100.0, 50.0),
            )
            .raw(
                "key-up",
                RawInputEvent::Keyboard(RawKeyboardEvent::release(
                    crate::KeyCode::Enter,
                    crate::KeyModifiers::NONE,
                    2,
                )),
            )
            .run(&mut document);

        assert_eq!(
            report.steps[0].converted,
            Some(UiInputEvent::Wheel {
                position: UiPoint::new(1.0, 1.0),
                delta: UiPoint::new(0.0, 20.0),
            })
        );
        assert!(report.steps[1].converted.is_none());
        assert!(report.require_all_converted().is_err());
    }
}
