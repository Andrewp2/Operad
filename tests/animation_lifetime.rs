use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use operad::{
    root_style, AnimatedValues, AnimationMachine, AnimationState, AnimationTransition,
    AnimationTrigger, ApproxTextMeasurer, ColorRgba, LayoutStyle, PaintKind, ScenePrimitive,
    UiDocument, UiNode, UiPoint, UiSize,
};

#[derive(Clone, Copy, Default)]
struct Allocations {
    live: isize,
    peak: usize,
}

thread_local! {
    static ALLOCATIONS: Cell<Option<Allocations>> = const { Cell::new(None) };
}

struct CountingAllocator;

fn record(delta: isize) {
    let _ = ALLOCATIONS.try_with(|slot| {
        if let Some(mut counts) = slot.get() {
            counts.live += delta;
            counts.peak = counts.peak.max(counts.live.max(0) as usize);
            slot.set(Some(counts));
        }
    });
}

// All allocation ownership stays with System. Only this test thread is measured.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size() as isize);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size() as isize);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size as isize - layout.size() as isize);
        unsafe { System.realloc(pointer, layout, size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record(-(layout.size() as isize));
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn with_allocation_tracking(run: impl FnOnce()) -> Allocations {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ALLOCATIONS.with(|slot| slot.set(None));
        }
    }
    ALLOCATIONS.with(|slot| assert!(slot.replace(Some(Allocations::default())).is_none()));
    let _reset = Reset;
    run();
    ALLOCATIONS.with(|slot| slot.get().unwrap())
}

const NAMES: [&str; 3] = ["a", "b", "c"];
const MORPHS: [f32; 3] = [0.25, 1.25, 3.0];

fn machine(morphs: [f32; 3]) -> AnimationMachine {
    let states = NAMES
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            AnimationState::new(
                name,
                AnimatedValues::new(
                    0.4 + i as f32 * 0.25,
                    UiPoint::new(i as f32 * 10.0, i as f32 * 5.0),
                    1.0 + i as f32 * 0.1,
                )
                .with_morph(morphs[i]),
            )
        })
        .collect();
    let mut transitions = Vec::new();
    for from in NAMES {
        for to in NAMES {
            if from != to {
                transitions.push(AnimationTransition::new(
                    from,
                    to,
                    AnimationTrigger::Custom(to.into()),
                    1.0,
                ));
            }
        }
    }
    AnimationMachine::new(states, transitions, "a").unwrap()
}

fn check_interruption_storage(morphs: [f32; 3]) {
    let counts = with_allocation_tracking(|| {
        let mut current = machine(morphs);
        let mut advanced = false;
        for step in 0..4096 {
            let target = (step + 1) % NAMES.len();
            let before = current.values();
            assert!(current.trigger(AnimationTrigger::Custom(NAMES[target].into())));
            assert_eq!(
                current.values(),
                before,
                "retargeting changed current values"
            );
            advanced |= current.tick([0.0, 0.0125, 0.125, 0.375][step % 4]).advanced;
            if step % 17 == 0 {
                let mut rebuilt = machine(morphs);
                assert!(rebuilt.retain_runtime_from(&current));
                assert_eq!(rebuilt.values(), current.values());
                current = rebuilt;
            }
            let peak = ALLOCATIONS.with(|slot| slot.get().unwrap().peak);
            assert!(
                peak < 32 * 1024,
                "three-state animation retained interruption history: step={step}, peak={peak}"
            );
        }
        assert!(advanced);
        assert!(current.tick(1.0).completed);
        assert!(!current.is_animating());
        assert_eq!(
            current.values(),
            current.states()[4096 % NAMES.len()].values
        );
    });
    assert_eq!(counts.live, 0, "animation allocations outlived their owner");
    eprintln!(
        "three-state animation peak live allocation: {} bytes",
        counts.peak
    );
}

#[test]
fn ordinary_animation_interruptions_have_bounded_storage() {
    check_interruption_storage([0.0; 3]);
}

#[test]
fn morph_animation_interruptions_have_bounded_storage() {
    check_interruption_storage(MORPHS);
}

// Reference state is one resolved polygon per primitive, never animation history.
// Arc-length sampling preserves the existing unequal-vertex interpolation contract.
fn resample(points: &[UiPoint], count: usize) -> Vec<UiPoint> {
    if points.is_empty() {
        return Vec::new();
    }
    if points.len() == count {
        return points.to_vec();
    }
    let mut lengths = Vec::new();
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        lengths.push(((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt());
    }
    let perimeter: f32 = lengths.iter().sum();
    (0..count)
        .map(|sample| {
            let mut remaining = perimeter * sample as f32 / count as f32;
            let mut edge = 0;
            while edge + 1 < lengths.len() && remaining > lengths[edge] {
                remaining -= lengths[edge];
                edge += 1;
            }
            let a = points[edge];
            let b = points[(edge + 1) % points.len()];
            let t = (remaining / lengths[edge].max(f32::EPSILON)).clamp(0.0, 1.0);
            UiPoint::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
        })
        .collect()
}

fn blend(from: &[UiPoint], to: &[UiPoint], progress: f32) -> Vec<UiPoint> {
    if from.is_empty() || to.is_empty() {
        return Vec::new();
    }
    let count = from.len().max(to.len()).max(3);
    resample(from, count)
        .into_iter()
        .zip(resample(to, count))
        .map(|(a, b)| UiPoint::new(a.x + (b.x - a.x) * progress, a.y + (b.y - a.y) * progress))
        .collect()
}

fn keyframe(frames: &[Vec<UiPoint>], amount: f32) -> Vec<UiPoint> {
    let amount = amount.clamp(0.0, (frames.len() - 1) as f32);
    let from = amount.floor() as usize;
    blend(
        &frames[from],
        &frames[(from + 1).min(frames.len() - 1)],
        amount.fract(),
    )
}

fn assert_polygons(document: &UiDocument, node: operad::UiNodeId, expected: &[Vec<UiPoint>]) {
    let paint = document.paint_list();
    let polygons: Vec<_> = paint
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            PaintKind::Polygon { points, .. } if item.node == node => Some(points),
            _ => None,
        })
        .collect();
    assert_eq!(polygons.len(), expected.len());
    let origin = document.node(node).layout().rect;
    for (actual, expected) in polygons.into_iter().zip(expected) {
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - origin.x - expected.x).abs() < 0.001
                    && (actual.y - origin.y - expected.y).abs() < 0.001,
                "painted {actual:?}, expected local {expected:?}, origin={origin:?}"
            );
        }
    }
}

#[test]
fn interrupted_morph_paint_matches_resolved_geometry_through_rebuilds() {
    for counts in [
        [5, 5, 5, 5],
        [3, 4, 6, 9],
        [9, 6, 4, 3],
        [1, 1, 1, 1],
        [0, 4, 6, 9],
    ] {
        let frames: Vec<Vec<_>> = counts
            .into_iter()
            .enumerate()
            .map(|(frame, count)| {
                (0..count)
                    .map(|i| {
                        let angle = std::f32::consts::TAU * i as f32 / count as f32;
                        let radius = if i % 2 == 0 { 18.0 } else { 11.0 } + frame as f32 * 7.0;
                        UiPoint::new(60.0 + radius * angle.cos(), 65.0 + radius * angle.sin())
                    })
                    .collect()
            })
            .collect();
        let offsets = [-0.5, 0.0, 0.4];
        let mut document = UiDocument::new(root_style(240.0, 220.0));
        let node = document.add_child(
            document.root(),
            UiNode::scene(
                "morphs",
                offsets
                    .into_iter()
                    .map(|amount| ScenePrimitive::MorphPolygonKeyframes {
                        frames: frames.clone(),
                        amount,
                        fill: ColorRgba::WHITE,
                        stroke: None,
                    })
                    .collect(),
                LayoutStyle::size(180.0, 180.0),
            )
            .with_animation(machine(MORPHS)),
        );
        document
            .compute_layout(UiSize::new(240.0, 220.0), &mut ApproxTextMeasurer)
            .unwrap();
        let mut resolved: Vec<_> = offsets
            .into_iter()
            .map(|offset| keyframe(&frames, offset + MORPHS[0]))
            .collect();
        assert_polygons(&document, node, &resolved);

        for step in 0..4096 {
            let target = (step + 1) % NAMES.len();
            let targets: Vec<_> = offsets
                .into_iter()
                .map(|offset| keyframe(&frames, offset + MORPHS[target]))
                .collect();
            assert!(
                document.trigger_animation(node, AnimationTrigger::Custom(NAMES[target].into()))
            );
            let start: Vec<_> = resolved
                .iter()
                .zip(&targets)
                .map(|(a, b)| blend(a, b, 0.0))
                .collect();
            assert_polygons(&document, node, &start);
            let dt = [0.0, 0.0125, 0.125, 0.375][step % 4];
            document.tick_animations(dt);
            resolved = resolved
                .iter()
                .zip(&targets)
                .map(|(a, b)| blend(a, b, dt))
                .collect();
            assert_polygons(&document, node, &resolved);
            if step % 17 == 0 {
                let mut rebuilt = machine(MORPHS);
                assert!(rebuilt.retain_runtime_from(document.node(node).animation().unwrap()));
                // Rebuild the scene node through its public authored-state API.
                let mut replacement = document.node(node).clone().with_animation(rebuilt);
                std::mem::swap(document.node_mut(node), &mut replacement);
                assert_polygons(&document, node, &resolved);
            }
        }
        assert_eq!(document.tick_animations(1.0).active, 0);
        let final_shapes: Vec<_> = offsets
            .into_iter()
            .map(|offset| keyframe(&frames, offset + MORPHS[4096 % 3]))
            .collect();
        assert_polygons(&document, node, &final_shapes);
    }
}
