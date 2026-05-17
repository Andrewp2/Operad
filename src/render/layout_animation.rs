//! Layout animation planning.
//!
//! The layout pass remains authoritative. These helpers compare two completed
//! layout snapshots and produce visual interpolation records that renderers or
//! widgets can apply after layout, without feeding animated positions back into
//! size or placement.

use std::collections::HashMap;

use crate::{LayoutSnapshot, PaintList, PaintTransform, UiNodeId, UiPoint, UiRect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutAnimationOptions {
    pub progress: f32,
    pub include_root: bool,
    pub animate_scale: bool,
}

impl Default for LayoutAnimationOptions {
    fn default() -> Self {
        Self {
            progress: 1.0,
            include_root: false,
            animate_scale: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutAnimationTransition {
    pub node: UiNodeId,
    pub name: String,
    pub from_rect: UiRect,
    pub to_rect: UiRect,
    pub visual_rect: UiRect,
    pub progress: f32,
    pub transform: PaintTransform,
}

pub fn layout_animation_transitions(
    previous: &LayoutSnapshot,
    current: &LayoutSnapshot,
    options: LayoutAnimationOptions,
) -> Vec<LayoutAnimationTransition> {
    let progress = options.progress.clamp(0.0, 1.0);
    let mut previous_by_name = HashMap::<&str, &LayoutSnapshot>::new();
    collect_layout_snapshot_by_name(previous, options.include_root, &mut previous_by_name);

    let mut transitions = Vec::new();
    collect_layout_transitions(
        current,
        &previous_by_name,
        options.include_root,
        options.animate_scale,
        progress,
        &mut transitions,
    );
    transitions
}

pub fn apply_layout_animation_transitions_to_paint_list(
    paint: &mut PaintList,
    transitions: &[LayoutAnimationTransition],
) -> usize {
    if transitions.is_empty() {
        return 0;
    }

    let transforms = transitions
        .iter()
        .map(|transition| (transition.node, transition.transform))
        .collect::<HashMap<_, _>>();
    let mut applied = 0;
    for item in &mut paint.items {
        let Some(layout_transform) = transforms.get(&item.node).copied() else {
            continue;
        };
        item.transform = compose_layout_and_paint_transform(layout_transform, item.transform);
        applied += 1;
    }
    applied
}

fn compose_layout_and_paint_transform(
    layout_transform: PaintTransform,
    paint_transform: PaintTransform,
) -> PaintTransform {
    PaintTransform {
        translation: UiPoint::new(
            paint_transform.translation.x * layout_transform.scale + layout_transform.translation.x,
            paint_transform.translation.y * layout_transform.scale + layout_transform.translation.y,
        ),
        scale: paint_transform.scale * layout_transform.scale,
    }
}

fn collect_layout_snapshot_by_name<'a>(
    snapshot: &'a LayoutSnapshot,
    include: bool,
    out: &mut HashMap<&'a str, &'a LayoutSnapshot>,
) {
    if include {
        out.insert(snapshot.name.as_str(), snapshot);
    }
    for child in &snapshot.children {
        collect_layout_snapshot_by_name(child, true, out);
    }
}

fn collect_layout_transitions(
    current: &LayoutSnapshot,
    previous_by_name: &HashMap<&str, &LayoutSnapshot>,
    include: bool,
    animate_scale: bool,
    progress: f32,
    out: &mut Vec<LayoutAnimationTransition>,
) {
    if include {
        if let Some(previous) = previous_by_name.get(current.name.as_str()) {
            if previous.rect != current.rect {
                out.push(layout_transition(
                    previous,
                    current,
                    animate_scale,
                    progress,
                ));
            }
        }
    }
    for child in &current.children {
        collect_layout_transitions(child, previous_by_name, true, animate_scale, progress, out);
    }
}

fn layout_transition(
    previous: &LayoutSnapshot,
    current: &LayoutSnapshot,
    animate_scale: bool,
    progress: f32,
) -> LayoutAnimationTransition {
    let visual_rect = lerp_rect(previous.rect, current.rect, progress);
    let scale = if animate_scale {
        let width_scale = if current.rect.width.abs() > f32::EPSILON {
            visual_rect.width / current.rect.width
        } else {
            1.0
        };
        let height_scale = if current.rect.height.abs() > f32::EPSILON {
            visual_rect.height / current.rect.height
        } else {
            1.0
        };
        ((width_scale + height_scale) * 0.5).max(0.0)
    } else {
        1.0
    };
    LayoutAnimationTransition {
        node: current.id,
        name: current.name.clone(),
        from_rect: previous.rect,
        to_rect: current.rect,
        visual_rect,
        progress,
        transform: PaintTransform {
            translation: UiPoint::new(
                visual_rect.x - current.rect.x,
                visual_rect.y - current.rect.y,
            ),
            scale,
        },
    }
}

fn lerp_rect(from: UiRect, to: UiRect, progress: f32) -> UiRect {
    UiRect::new(
        lerp(from.x, to.x, progress),
        lerp(from.y, to.y, progress),
        lerp(from.width, to.width, progress),
        lerp(from.height, to.height, progress),
    )
}

fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        root_style, ApproxTextMeasurer, ColorRgba, LayoutStyle, PaintItem, PaintKind, PaintList,
        StrokeStyle, UiDocument, UiNode, UiSize,
    };

    #[test]
    fn layout_animation_transitions_interpolate_changed_rects_after_layout() {
        let mut previous = UiDocument::new(root_style(240.0, 120.0));
        previous.add_child(
            previous.root,
            UiNode::container("panel", LayoutStyle::size(80.0, 40.0)),
        );
        previous
            .compute_layout(UiSize::new(240.0, 120.0), &mut ApproxTextMeasurer)
            .expect("previous layout");

        let mut current = UiDocument::new(root_style(240.0, 120.0));
        current.add_child(
            current.root,
            UiNode::container("panel", LayoutStyle::size(160.0, 60.0)),
        );
        current
            .compute_layout(UiSize::new(240.0, 120.0), &mut ApproxTextMeasurer)
            .expect("current layout");

        let transitions = layout_animation_transitions(
            &previous.layout_snapshot(),
            &current.layout_snapshot(),
            LayoutAnimationOptions {
                progress: 0.5,
                ..Default::default()
            },
        );

        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].name, "panel");
        assert_eq!(transitions[0].visual_rect.width, 120.0);
        assert_eq!(transitions[0].visual_rect.height, 50.0);
        assert_eq!(transitions[0].to_rect.width, 160.0);
        assert!(transitions[0].transform.scale < 1.0);
    }

    #[test]
    fn layout_animation_transitions_apply_to_matching_paint_items_only() {
        let mut paint = PaintList {
            items: vec![
                PaintItem {
                    node: UiNodeId(1),
                    rect: UiRect::new(0.0, 0.0, 100.0, 40.0),
                    clip_rect: UiRect::new(0.0, 0.0, 200.0, 100.0),
                    z_index: 0,
                    layer_order: Default::default(),
                    opacity: 1.0,
                    transform: PaintTransform {
                        translation: UiPoint::new(2.0, 3.0),
                        scale: 1.0,
                    },
                    shader: None,
                    kind: PaintKind::Rect {
                        fill: ColorRgba::new(24, 30, 36, 255),
                        stroke: Some(StrokeStyle::new(ColorRgba::new(90, 100, 120, 255), 1.0)),
                        corner_radius: 4.0,
                    },
                },
                PaintItem {
                    node: UiNodeId(2),
                    rect: UiRect::new(0.0, 0.0, 40.0, 20.0),
                    clip_rect: UiRect::new(0.0, 0.0, 200.0, 100.0),
                    z_index: 0,
                    layer_order: Default::default(),
                    opacity: 1.0,
                    transform: PaintTransform::default(),
                    shader: None,
                    kind: PaintKind::Rect {
                        fill: ColorRgba::new(24, 30, 36, 255),
                        stroke: None,
                        corner_radius: 0.0,
                    },
                },
            ],
        };
        let transition = LayoutAnimationTransition {
            node: UiNodeId(1),
            name: "panel".to_owned(),
            from_rect: UiRect::new(0.0, 0.0, 80.0, 30.0),
            to_rect: UiRect::new(10.0, 10.0, 100.0, 40.0),
            visual_rect: UiRect::new(5.0, 5.0, 90.0, 35.0),
            progress: 0.5,
            transform: PaintTransform {
                translation: UiPoint::new(-5.0, -5.0),
                scale: 0.9,
            },
        };

        let applied = apply_layout_animation_transitions_to_paint_list(&mut paint, &[transition]);

        assert_eq!(applied, 1);
        assert_eq!(paint.items[0].transform.scale, 0.9);
        assert!((paint.items[0].transform.translation.x + 3.2).abs() < 0.001);
        assert!((paint.items[0].transform.translation.y + 2.3).abs() < 0.001);
        assert_eq!(paint.items[1].transform, PaintTransform::default());
    }
}
