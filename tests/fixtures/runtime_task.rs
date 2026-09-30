//! Native event-loop regression probe. A worker waits for stdin while the UI
//! sleeps. Run with scripts/check-native-task.py after building this example.

use operad::runtime::{task_channel, Application, RuntimeHooks};
use operad::{LayoutStyle, TextStyle, UiContent, UiDocument, UiNode, UiSize};
use std::io::{BufRead, Write};

fn view(
    state: &String,
    viewport: UiSize,
    views: &mut operad::runtime::ViewContext<'_>,
) -> UiDocument {
    let mut doc = UiDocument::new(LayoutStyle::size(viewport.width, viewport.height));
    let root = doc.root();
    views.section(&mut doc, root, "result-panel", state, |state, _| {
        let mut doc = UiDocument::new(LayoutStyle::size(300.0, 40.0));
        doc.add_child(
            doc.root(),
            UiNode::text(
                "result",
                state,
                TextStyle::default(),
                LayoutStyle::default(),
            ),
        );
        doc
    });
    views.section(&mut doc, root, "unchanged-panel", &(), |_, _| {
        let mut doc = UiDocument::new(LayoutStyle::size(300.0, 40.0));
        doc.add_child(
            doc.root(),
            UiNode::text(
                "unchanged",
                "Background jobs update only their result panel",
                TextStyle::default(),
                LayoutStyle::default(),
            ),
        );
        doc
    });
    doc
}

fn main() -> operad::native::NativeWindowResult {
    let (sender, receiver) = task_channel(1);
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let result = match line.unwrap().as_str() {
                "complete" => Ok("loaded".to_owned()),
                "fail" => Err("read failed".to_owned()),
                command => panic!("unknown probe command: {command}"),
            };
            if sender.try_send(result).is_err() {
                break;
            }
        }
    });
    let ui_thread = std::thread::current().id();
    let mut frames = 0;
    let mut previous = None;
    let hooks = RuntimeHooks::new()
        .with_task_completions(
            receiver,
            move |state: &mut String, result: Result<String, String>| {
                assert_eq!(std::thread::current().id(), ui_thread);
                *state = result.unwrap_or_else(|error| error);
            },
        )
        .with_frame_observer(move |state: &String, observation| {
            let rendered = observation
                .document
                .nodes()
                .iter()
                .find_map(|node| match node.content() {
                    UiContent::Text(text) => Some(text.text.as_str()),
                    _ => None,
                })
                .unwrap();
            assert_eq!(rendered, state);
            if previous.as_ref() != Some(state) {
                let stats = observation.view_build_stats;
                assert_eq!(stats.rebuilt, if previous.is_none() { 2 } else { 1 });
                assert_eq!(stats.reused, usize::from(previous.is_some()));
                previous = Some(state.clone());
            }
            frames += 1;
            println!("FRAME {frames} {rendered}");
            std::io::stdout().flush().unwrap();
        });
    Application::new("waiting".to_owned(), |_, _| {}, view)
        .with_hooks(hooks)
        .run_native(operad::native::NativeWindowOptions::new(
            "Background completion probe",
        ))
}
