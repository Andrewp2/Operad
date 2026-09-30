//! Public custom-host acceptance for the same application used by the browser probe.

#[allow(dead_code)]
#[path = "../examples/runtime_editor.rs"]
mod editor;

use operad::host::collect_document_widget_actions;
use operad::input::{PointerButton, PointerEventKind, RawInputEvent, RawPointerEvent};
use operad::platform::{PixelSize, PlatformRequestIdAllocator};
use operad::renderer::RenderTarget;
use operad::runtime::session::RuntimeSession;
use operad::runtime::{RuntimeHooks, RuntimeMetrics, RuntimeObservation};
use operad::{
    ApproxTextMeasurer, AvailableSize, KnownSize, PaintKind, TextContent, TextMeasurer,
    UiDocumentScale, UiPoint, UiSize,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const VIEWPORT: UiSize = UiSize::new(700.0, 420.0);

struct CountingMeasurer(Rc<Cell<usize>>);
impl TextMeasurer for CountingMeasurer {
    fn measure(
        &mut self,
        text: &TextContent,
        known: KnownSize,
        available: AvailableSize,
    ) -> UiSize {
        self.0.set(self.0.get() + 1);
        ApproxTextMeasurer.measure(text, known, available)
    }
}

fn pointer(kind: PointerEventKind, x: f32, y: f32) -> RawInputEvent {
    RawInputEvent::Pointer(RawPointerEvent::new(kind, UiPoint::new(x, y), 1))
}

fn render(
    session: &mut RuntimeSession,
    state: &mut editor::Editor,
    hooks: &mut RuntimeHooks<editor::Editor>,
    measurer: &mut CountingMeasurer,
    events: &[RawInputEvent],
) {
    session.begin_frame(Duration::ZERO);
    let mut document = session
        .build_document(
            VIEWPORT,
            UiDocumentScale::DEFAULT,
            None,
            measurer,
            |size, views| editor::view(state, size, views),
        )
        .unwrap();
    let input = session
        .process_input_with_hooks(&mut document, VIEWPORT, events, &[], hooks, state, measurer)
        .unwrap();
    let target = RenderTarget::window("editor", VIEWPORT);
    let mut ids = PlatformRequestIdAllocator::default();
    let mut frame = session
        .finish_frame(
            &mut document,
            VIEWPORT,
            target.clone(),
            input,
            measurer,
            &mut ids,
        )
        .unwrap();
    for action in collect_document_widget_actions(&frame) {
        editor::update(state, action);
        session.invalidate_view();
    }
    while session.view_needs_rebuild() {
        session.retain_document(document);
        document = session
            .build_document(
                VIEWPORT,
                UiDocumentScale::DEFAULT,
                None,
                measurer,
                |size, views| editor::view(state, size, views),
            )
            .unwrap();
        session.reconcile_input_hooks(&document, hooks, state);
        let input = session
            .process_input(&mut document, VIEWPORT, Vec::new(), Vec::new(), measurer)
            .unwrap();
        frame = session
            .finish_frame(
                &mut document,
                VIEWPORT,
                target.clone(),
                input,
                measurer,
                &mut ids,
            )
            .unwrap();
    }
    let built = state.builds.get();
    let measured = measurer.0.get();
    hooks.observe(
        state,
        RuntimeObservation::new(
            RuntimeMetrics {
                physical_size: PixelSize::new(700, 420),
                viewport: VIEWPORT,
                scale_factor: 1.0,
                dpi_scale: 1.0,
                elapsed: Duration::ZERO,
            },
            &document,
            &frame,
            session.view_build_stats(),
        ),
    );
    assert_eq!(
        state.builds.get(),
        built,
        "observing rebuilt the application view"
    );
    assert_eq!(
        measurer.0.get(),
        measured,
        "observing measured layout again"
    );
    session.retain_document(document);
    session.frame_presented();
}

#[test]
fn shared_editor_hooks_and_observer_follow_the_final_frame_without_rebuilding_it() {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let captured = observations.clone();
    let mut hooks =
        editor::hooks(move |state, observation| {
            assert_eq!(editor::observed_status(observation), editor::status(state));
            assert!(observation.paint().items.iter().any(|item| {
            matches!(&item.kind, PaintKind::Text(text) if text.text == editor::status(state))
        }), "observation paint contains stale application state");
            let nodes = observation.document.nodes();
            let canvas_index = nodes
                .iter()
                .position(|node| node.name() == "timeline")
                .unwrap();
            let canvas = &nodes[canvas_index];
            assert_eq!(canvas.layout().rect.width, 320.0);
            assert!(canvas.layout().visible);
            let label = nodes
                .iter()
                .find(|node| node.name() == "arrangement.name")
                .unwrap();
            assert_eq!(
                label.accessibility().unwrap().label.as_deref(),
                Some(editor::LONG_LABEL)
            );
            captured.borrow_mut().push((state.revision, canvas_index));
        });
    let mut state = editor::Editor::default();
    let mut measurer = CountingMeasurer(Rc::new(Cell::new(0)));
    let mut session = RuntimeSession::new();
    render(&mut session, &mut state, &mut hooks, &mut measurer, &[]);
    render(
        &mut session,
        &mut state,
        &mut hooks,
        &mut measurer,
        &[
            pointer(PointerEventKind::Down(PointerButton::Primary), 300.0, 125.0),
            pointer(PointerEventKind::Up(PointerButton::Primary), 300.0, 125.0),
        ],
    );
    assert_eq!(
        state.downs, 0,
        "disabled overlay passed input into the canvas"
    );
    assert_eq!(state.blocked_activations, 0);
    render(
        &mut session,
        &mut state,
        &mut hooks,
        &mut measurer,
        &[pointer(
            PointerEventKind::Down(PointerButton::Primary),
            60.0,
            130.0,
        )],
    );
    assert!(state.dragging);
    assert_eq!(state.downs, 1);
    assert_ne!(
        observations.borrow()[0].1,
        observations.borrow().last().unwrap().1,
        "fixture must actually reorder document indices during capture"
    );
    render(
        &mut session,
        &mut state,
        &mut hooks,
        &mut measurer,
        &[
            pointer(PointerEventKind::Move, 740.0, 450.0),
            pointer(PointerEventKind::Up(PointerButton::Primary), 760.0, 470.0),
        ],
    );
    assert_eq!(
        (state.moves, state.releases, state.cancellations),
        (1, 1, 0)
    );
    assert!(!state.dragging);
    assert_eq!(state.last_local, Some(UiPoint::new(724.0, 370.0)));
    assert_eq!(observations.borrow().last().unwrap().0, state.revision);
    let built = state.builds.get();
    let measured = measurer.0.get();
    render(&mut session, &mut state, &mut hooks, &mut measurer, &[]);
    assert_eq!(
        state.builds.get(),
        built,
        "idle observation rebuilt the view"
    );
    assert_eq!(
        measurer.0.get(),
        measured,
        "idle observation invalidated layout"
    );
}
