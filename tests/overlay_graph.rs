use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use operad::{
    OverlayDismissPolicy, OverlayDismissReason, OverlayEntry, OverlayFocusRestoreTarget, OverlayId,
    OverlayKind, OverlayStack, UiPoint, UiRect,
};

fn id(index: usize) -> OverlayId {
    OverlayId::new(index as u64 + 1)
}

fn entry(index: usize) -> OverlayEntry {
    OverlayEntry::new(
        id(index),
        OverlayKind::Menu,
        UiRect::new(index as f32 * 20.0, 0.0, 10.0, 10.0),
    )
    .focus_restore(OverlayFocusRestoreTarget::Logical(format!(
        "trigger {index}"
    )))
}

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[test]
fn overlay_graph_routing_terminates_and_matches_reachability() {
    const CHILD: &str = "OPERAD_TEST_OVERLAY_GRAPH_CHILD";
    if std::env::var(CHILD).as_deref() == Ok("1") {
        compare_small_graphs();
        check_replacement_cycle();
        return;
    }

    // A reintroduced ancestry loop must fail this test instead of hanging CI.
    // The timeout bounds the subprocess, not the performance of an individual query.
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "overlay_graph_routing_terminates_and_matches_reachability",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .stdin(Stdio::null())
            .spawn()
            .expect("spawn isolated routing regression"),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "overlay routing child failed: {status}");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "overlay routing did not terminate within 10 seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn compare_small_graphs() {
    const N: usize = 4;
    // Each entry has no parent, one of four live parents, or a missing parent.
    for encoded in 0..6_usize.pow(N as u32) {
        let mut remaining = encoded;
        let parents: [Option<usize>; N] = std::array::from_fn(|_| {
            let choice = remaining % 6;
            remaining /= 6;
            (choice != 0).then(|| choice - 1)
        });

        // Independent transitive closure: production walks one parent chain.
        let mut reachable = [[false; N]; N];
        for (child, parent) in parents.iter().enumerate() {
            if let Some(parent) = parent.filter(|parent| *parent < N) {
                reachable[child][parent] = true;
            }
        }
        for via in 0..N {
            for from in 0..N {
                for to in 0..N {
                    reachable[from][to] |= reachable[from][via] && reachable[via][to];
                }
            }
        }

        // No modal, a graph member as modal, and a separate unrelated modal.
        for modal in [None, Some(0), Some(N)] {
            let mut stack = OverlayStack::new();
            for (index, parent) in parents.iter().enumerate() {
                let mut overlay =
                    entry(index)
                        .modal(modal == Some(index))
                        .dismiss_policy(OverlayDismissPolicy {
                            outside_pointer: index % 2 == 0,
                            ..OverlayDismissPolicy::dismissible()
                        });
                if let Some(parent) = parent {
                    overlay = overlay.parent(if *parent < N { id(*parent) } else { id(99) });
                }
                stack.push(overlay);
            }
            if modal == Some(N) {
                stack.push(entry(N).modal(true).layer(10));
            }
            let before = stack.clone();
            let count = stack.entries().len();
            for hit in (0..count).map(Some).chain([None]) {
                let point = UiPoint::new(hit.map_or(1000.0, |hit| hit as f32 * 20.0 + 5.0), 5.0);
                let decision = stack.route_pointer_down(point);
                assert_eq!(decision.hit, hit.map(id));
                let allowed = modal.is_none_or(|modal| {
                    hit.is_some_and(|hit| {
                        hit == modal || (hit < N && modal < N && reachable[hit][modal])
                    })
                });
                assert_eq!(
                    decision.blocked_by_modal,
                    if allowed { None } else { modal.map(id) },
                    "parents={parents:?}, modal={modal:?}, hit={hit:?}"
                );
                let mut dismiss = Vec::new();
                if allowed {
                    for candidate in (0..count).rev() {
                        if hit.is_some_and(|hit| {
                            hit == candidate
                                || (hit < N && candidate < N && reachable[hit][candidate])
                        }) {
                            break;
                        }
                        if candidate == N || candidate % 2 == 0 {
                            dismiss.push(id(candidate));
                        }
                    }
                }
                assert_eq!(
                    decision.dismiss, dismiss,
                    "parents={parents:?}, hit={hit:?}"
                );
            }
            assert_eq!(stack, before, "routing mutated the parent graph");

            for root in 0..N {
                let mut dismissed_stack = stack.clone();
                let outcome = dismissed_stack.dismiss(id(root), OverlayDismissReason::Programmatic);
                let expected: Vec<_> = (0..N)
                    .rev()
                    .filter(|child| *child == root || reachable[*child][root])
                    .map(id)
                    .collect();
                assert_eq!(
                    outcome.dismissed, expected,
                    "parents={parents:?}, root={root}"
                );
                assert_eq!(outcome.focus_restore.len(), expected.len());
                for record in &outcome.focus_restore {
                    assert_eq!(
                        record.target,
                        OverlayFocusRestoreTarget::Logical(format!(
                            "trigger {}",
                            record.overlay.value() - 1
                        ))
                    );
                    assert_eq!(record.reason, OverlayDismissReason::Programmatic);
                }
                for index in 0..count {
                    assert_eq!(
                        dismissed_stack.get(id(index)).is_none(),
                        expected.contains(&id(index))
                    );
                }
            }
        }
    }
}

fn check_replacement_cycle() {
    let mut stack = OverlayStack::new();
    stack.push(entry(0));
    stack.push(entry(1).parent(id(0)));
    stack.push(entry(0).parent(id(1)));
    stack.push(entry(2).modal(true));
    let decision = stack.route_pointer_down(UiPoint::new(5.0, 5.0));
    assert_eq!(decision.hit, Some(id(0)));
    assert_eq!(decision.blocked_by_modal, Some(id(2)));
    assert!(decision.dismiss.is_empty());
    assert_eq!(stack.entries().len(), 3);
    stack.dismiss(id(2), OverlayDismissReason::Programmatic);
    assert_eq!(
        stack
            .dismiss(id(0), OverlayDismissReason::Programmatic)
            .dismissed,
        [id(0), id(1)]
    );
}

#[test]
fn overlay_dismissal_handles_deep_parent_chains() {
    let mut stack = OverlayStack::new();
    for index in 0..1024 {
        let mut overlay = entry(index);
        if index > 0 {
            overlay = overlay.parent(id(index - 1));
        }
        stack.push(overlay);
    }
    let outcome = stack.dismiss(id(0), OverlayDismissReason::Programmatic);
    assert_eq!(
        outcome.dismissed,
        (0..1024).rev().map(id).collect::<Vec<_>>()
    );
    assert_eq!(outcome.focus_restore.len(), 1024);
    assert!(stack.entries().is_empty());
}
