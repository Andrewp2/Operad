//! Renderer-neutral debug snapshots and dumps.
//!
//! These helpers collect layout, paint, input, command-scope, repaint, and
//! timing state into plain data. Backends can render the data as overlays,
//! logs, or inspector panels without Operad depending on a debug UI renderer.

use std::collections::{BTreeMap, HashMap};

use crate::{
    CommandId, CommandScope, DirtyFlags, FrameTiming, GestureEvent, GesturePhase,
    HostInteractionState, HostNodeInteraction, LayoutSnapshot, PaintKind, PaintList, UiDocument,
    UiNodeId, UiPoint, UiRect,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugOverlayOptions {
    pub include_invisible: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DebugOverlayContext {
    pub host: HostInteractionState,
    pub active_gesture: Option<DebugGestureState>,
    pub dirty_flags: DirtyFlags,
    pub repaint_reason: Option<String>,
    pub timings: FrameTiming,
}

impl DebugOverlayContext {
    pub fn new(host: HostInteractionState) -> Self {
        Self {
            host,
            ..Default::default()
        }
    }

    pub fn active_gesture(mut self, gesture: &GestureEvent) -> Self {
        self.active_gesture = Some(DebugGestureState::from(gesture));
        self
    }

    pub fn dirty_flags(mut self, dirty_flags: DirtyFlags) -> Self {
        self.dirty_flags = dirty_flags;
        self
    }

    pub fn repaint_reason(mut self, reason: impl Into<String>) -> Self {
        self.repaint_reason = Some(reason.into());
        self
    }

    pub fn timings(mut self, timings: FrameTiming) -> Self {
        self.timings = timings;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugGestureKind {
    Hover,
    Press,
    Drag(GesturePhase),
    Click { count: u8 },
    Wheel,
    Cancel,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugGestureState {
    pub kind: DebugGestureKind,
    pub target: Option<UiNodeId>,
    pub position: UiPoint,
}

impl From<&GestureEvent> for DebugGestureState {
    fn from(event: &GestureEvent) -> Self {
        match event {
            GestureEvent::Hover {
                target, position, ..
            } => Self {
                kind: DebugGestureKind::Hover,
                target: *target,
                position: *position,
            },
            GestureEvent::Press {
                target, position, ..
            } => Self {
                kind: DebugGestureKind::Press,
                target: *target,
                position: *position,
            },
            GestureEvent::Drag(gesture) => Self {
                kind: DebugGestureKind::Drag(gesture.phase),
                target: Some(gesture.target),
                position: gesture.current,
            },
            GestureEvent::Click(click) => Self {
                kind: DebugGestureKind::Click { count: click.count },
                target: Some(click.target),
                position: click.position,
            },
            GestureEvent::WheelTargeted { target, event } => Self {
                kind: DebugGestureKind::Wheel,
                target: *target,
                position: event.position,
            },
            GestureEvent::Cancel {
                target, position, ..
            } => Self {
                kind: DebugGestureKind::Cancel,
                target: Some(*target),
                position: *position,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugPaintStats {
    pub count: usize,
    pub min_z: Option<i16>,
    pub max_z: Option<i16>,
}

impl DebugPaintStats {
    fn record(&mut self, z_index: i16) {
        self.count += 1;
        self.min_z = Some(self.min_z.map_or(z_index, |z| z.min(z_index)));
        self.max_z = Some(self.max_z.map_or(z_index, |z| z.max(z_index)));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugOverlayNode {
    pub id: UiNodeId,
    pub name: String,
    pub rect: UiRect,
    pub clip_rect: UiRect,
    pub visible: bool,
    pub pointer: bool,
    pub focusable: bool,
    pub interaction: HostNodeInteraction,
    pub paint: DebugPaintStats,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugOverlaySnapshot {
    pub nodes: Vec<DebugOverlayNode>,
    pub active_gesture: Option<DebugGestureState>,
    pub active_shortcut_scopes: Vec<CommandScope>,
    pub routed_command: Option<CommandId>,
    pub dirty_flags: DirtyFlags,
    pub repaint_reason: Option<String>,
    pub timings: FrameTiming,
}

impl DebugOverlaySnapshot {
    pub fn from_document(
        document: &UiDocument,
        context: DebugOverlayContext,
        options: DebugOverlayOptions,
    ) -> Self {
        let layout = document.layout_snapshot();
        let paint = document.paint_list();
        Self::from_parts(&layout, &paint, context, options)
    }

    pub fn from_parts(
        layout: &LayoutSnapshot,
        paint: &PaintList,
        context: DebugOverlayContext,
        options: DebugOverlayOptions,
    ) -> Self {
        let paint_stats = paint_stats_by_node(paint);
        let mut nodes = Vec::new();
        collect_debug_nodes(
            layout,
            &paint_stats,
            &context.host,
            options.include_invisible,
            &mut nodes,
        );
        Self {
            nodes,
            active_gesture: context.active_gesture,
            active_shortcut_scopes: context.host.active_shortcut_scopes,
            routed_command: context.host.shortcut_route.and_then(|route| route.command),
            dirty_flags: context.dirty_flags,
            repaint_reason: context.repaint_reason,
            timings: context.timings,
        }
    }

    pub fn active_nodes(&self) -> impl Iterator<Item = &DebugOverlayNode> {
        self.nodes
            .iter()
            .filter(|node| node.interaction.any() || node.paint.count > 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugPaintKindCount {
    pub kind: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugPaintItem {
    pub node: UiNodeId,
    pub node_name: Option<String>,
    pub kind: String,
    pub rect: UiRect,
    pub clip_rect: UiRect,
    pub z_index: i16,
    pub opacity: f32,
    pub shader_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugPaintDump {
    pub item_count: usize,
    pub kind_counts: Vec<DebugPaintKindCount>,
    pub items: Vec<DebugPaintItem>,
}

impl DebugPaintDump {
    pub fn from_document(document: &UiDocument) -> Self {
        let layout = document.layout_snapshot();
        let paint = document.paint_list();
        Self::from_parts(&layout, &paint)
    }

    pub fn from_parts(layout: &LayoutSnapshot, paint: &PaintList) -> Self {
        let names = layout_names_by_node(layout);
        let mut counts = BTreeMap::<String, usize>::new();
        let items = paint
            .items
            .iter()
            .map(|item| {
                let kind = paint_kind_label(&item.kind).to_owned();
                *counts.entry(kind.clone()).or_default() += 1;
                DebugPaintItem {
                    node: item.node,
                    node_name: names.get(&item.node).cloned(),
                    kind,
                    rect: item.rect,
                    clip_rect: item.clip_rect,
                    z_index: item.z_index,
                    opacity: item.opacity,
                    shader_key: item.shader.as_ref().map(|shader| shader.key.clone()),
                }
            })
            .collect::<Vec<_>>();
        Self {
            item_count: items.len(),
            kind_counts: counts
                .into_iter()
                .map(|(kind, count)| DebugPaintKindCount { kind, count })
                .collect(),
            items,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugHitCandidate {
    pub id: UiNodeId,
    pub name: String,
    pub rect: UiRect,
    pub clip_rect: UiRect,
    pub pointer: bool,
    pub visible: bool,
    pub contains_rect: bool,
    pub contains_clip: bool,
    pub paint: DebugPaintStats,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugHitTrace {
    pub point: UiPoint,
    pub hit: Option<UiNodeId>,
    pub candidates: Vec<DebugHitCandidate>,
}

impl DebugHitTrace {
    pub fn from_document(document: &UiDocument, point: UiPoint) -> Self {
        let layout = document.layout_snapshot();
        let paint = document.paint_list();
        Self::from_parts(&layout, &paint, document.hit_test(point), point)
    }

    pub fn from_parts(
        layout: &LayoutSnapshot,
        paint: &PaintList,
        hit: Option<UiNodeId>,
        point: UiPoint,
    ) -> Self {
        let paint_stats = paint_stats_by_node(paint);
        let mut candidates = Vec::new();
        collect_hit_candidates(layout, &paint_stats, point, &mut candidates);
        candidates.sort_by(|left, right| {
            right
                .contains_rect
                .cmp(&left.contains_rect)
                .then_with(|| right.contains_clip.cmp(&left.contains_clip))
                .then_with(|| right.paint.max_z.cmp(&left.paint.max_z))
                .then_with(|| left.id.0.cmp(&right.id.0))
        });
        Self {
            point,
            hit,
            candidates,
        }
    }
}

pub fn layout_snapshot_dump(snapshot: &LayoutSnapshot) -> String {
    let mut lines = Vec::new();
    push_layout_dump_line(snapshot, 0, &mut lines);
    lines.join("\n")
}

fn collect_debug_nodes(
    snapshot: &LayoutSnapshot,
    paint_stats: &HashMap<UiNodeId, DebugPaintStats>,
    host: &HostInteractionState,
    include_invisible: bool,
    out: &mut Vec<DebugOverlayNode>,
) {
    if include_invisible || snapshot.visible {
        out.push(DebugOverlayNode {
            id: snapshot.id,
            name: snapshot.name.clone(),
            rect: snapshot.rect,
            clip_rect: snapshot.clip_rect,
            visible: snapshot.visible,
            pointer: snapshot.pointer,
            focusable: snapshot.focusable,
            interaction: host.node_state(snapshot.id),
            paint: paint_stats.get(&snapshot.id).copied().unwrap_or_default(),
        });
    }

    for child in &snapshot.children {
        collect_debug_nodes(child, paint_stats, host, include_invisible, out);
    }
}

fn collect_hit_candidates(
    snapshot: &LayoutSnapshot,
    paint_stats: &HashMap<UiNodeId, DebugPaintStats>,
    point: UiPoint,
    out: &mut Vec<DebugHitCandidate>,
) {
    let contains_rect = snapshot.rect.contains_point(point);
    let contains_clip = snapshot.clip_rect.contains_point(point);
    if contains_rect || contains_clip {
        out.push(DebugHitCandidate {
            id: snapshot.id,
            name: snapshot.name.clone(),
            rect: snapshot.rect,
            clip_rect: snapshot.clip_rect,
            pointer: snapshot.pointer,
            visible: snapshot.visible,
            contains_rect,
            contains_clip,
            paint: paint_stats.get(&snapshot.id).copied().unwrap_or_default(),
        });
    }

    for child in &snapshot.children {
        collect_hit_candidates(child, paint_stats, point, out);
    }
}

fn paint_stats_by_node(paint: &PaintList) -> HashMap<UiNodeId, DebugPaintStats> {
    let mut stats = HashMap::new();
    for item in &paint.items {
        stats
            .entry(item.node)
            .or_insert_with(DebugPaintStats::default)
            .record(item.z_index);
    }
    stats
}

fn layout_names_by_node(snapshot: &LayoutSnapshot) -> HashMap<UiNodeId, String> {
    let mut names = HashMap::new();
    collect_layout_names(snapshot, &mut names);
    names
}

fn collect_layout_names(snapshot: &LayoutSnapshot, names: &mut HashMap<UiNodeId, String>) {
    names.insert(snapshot.id, snapshot.name.clone());
    for child in &snapshot.children {
        collect_layout_names(child, names);
    }
}

fn push_layout_dump_line(snapshot: &LayoutSnapshot, depth: usize, lines: &mut Vec<String>) {
    lines.push(format!(
        "{}{}#{} rect={:.1},{:.1},{:.1},{:.1} clip={:.1},{:.1},{:.1},{:.1} visible={} pointer={} focusable={}",
        "  ".repeat(depth),
        snapshot.name,
        snapshot.id.0,
        snapshot.rect.x,
        snapshot.rect.y,
        snapshot.rect.width,
        snapshot.rect.height,
        snapshot.clip_rect.x,
        snapshot.clip_rect.y,
        snapshot.clip_rect.width,
        snapshot.clip_rect.height,
        snapshot.visible,
        snapshot.pointer,
        snapshot.focusable,
    ));
    for child in &snapshot.children {
        push_layout_dump_line(child, depth + 1, lines);
    }
}

fn paint_kind_label(kind: &PaintKind) -> &'static str {
    match kind {
        PaintKind::Rect { .. } => "rect",
        PaintKind::Text(_) => "text",
        PaintKind::Canvas(_) => "canvas",
        PaintKind::Line { .. } => "line",
        PaintKind::Circle { .. } => "circle",
        PaintKind::Polygon { .. } => "polygon",
        PaintKind::Image { .. } => "image",
        PaintKind::RichRect(_) => "rich_rect",
        PaintKind::SceneText(_) => "scene_text",
        PaintKind::Path(_) => "path",
        PaintKind::ImagePlacement(_) => "image_placement",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        length, ApproxTextMeasurer, ColorRgba, InputBehavior, RawWheelEvent, StrokeStyle,
        TextStyle, UiNode, UiNodeStyle, UiSize, UiVisual, WheelDeltaUnit, WheelPhase,
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
    fn debug_overlay_snapshot_combines_layout_paint_and_host_state() {
        let mut doc = UiDocument::new(fixed_style(240.0, 160.0));
        let button = doc.add_child(
            doc.root,
            UiNode::container("play", fixed_style(80.0, 32.0))
                .with_input(InputBehavior::BUTTON)
                .with_visual(UiVisual::panel(
                    ColorRgba::new(20, 30, 40, 255),
                    Some(StrokeStyle::new(ColorRgba::new(80, 90, 100, 255), 1.0)),
                    4.0,
                )),
        );
        doc.compute_layout(UiSize::new(240.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let mut host = HostInteractionState {
            hovered: Some(button),
            focused: Some(button),
            active_shortcut_scopes: vec![CommandScope::Workspace],
            ..Default::default()
        };
        host.shortcut_route = Some(crate::HostShortcutRoute {
            shortcut: crate::Shortcut::ctrl('p'),
            active_scopes: vec![CommandScope::Workspace],
            target: Some(button),
            command: Some(CommandId::new("transport.play")),
        });
        let wheel = GestureEvent::WheelTargeted {
            target: Some(button),
            event: RawWheelEvent {
                position: UiPoint::new(10.0, 10.0),
                delta: UiPoint::new(0.0, -1.0),
                unit: WheelDeltaUnit::Line,
                phase: WheelPhase::Moved,
                modifiers: crate::KeyModifiers::NONE,
                timestamp_millis: 10,
            },
        };

        let snapshot = DebugOverlaySnapshot::from_document(
            &doc,
            DebugOverlayContext::new(host)
                .active_gesture(&wheel)
                .dirty_flags(DirtyFlags {
                    paint: true,
                    ..DirtyFlags::NONE
                })
                .repaint_reason("hover changed")
                .timings(FrameTiming::new().section("layout", std::time::Duration::from_millis(2))),
            DebugOverlayOptions::default(),
        );

        let button_debug = snapshot
            .nodes
            .iter()
            .find(|node| node.id == button)
            .unwrap();
        assert!(button_debug.interaction.hovered);
        assert!(button_debug.interaction.focused);
        assert_eq!(button_debug.paint.count, 1);
        assert_eq!(
            snapshot.routed_command,
            Some(CommandId::new("transport.play"))
        );
        assert_eq!(
            snapshot.active_shortcut_scopes,
            vec![CommandScope::Workspace]
        );
        assert_eq!(snapshot.repaint_reason.as_deref(), Some("hover changed"));
        assert_eq!(
            snapshot.active_gesture.as_ref().unwrap().kind,
            DebugGestureKind::Wheel
        );
        assert_eq!(
            snapshot.timings.duration("layout"),
            Some(std::time::Duration::from_millis(2))
        );
    }

    #[test]
    fn debug_paint_dump_counts_primitives_and_preserves_node_names() {
        let mut doc = UiDocument::new(fixed_style(200.0, 120.0));
        let label = doc.add_child(
            doc.root,
            UiNode::text(
                "status",
                "Ready",
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
        doc.compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let dump = DebugPaintDump::from_document(&doc);

        assert_eq!(dump.item_count, 1);
        assert_eq!(
            dump.kind_counts,
            vec![DebugPaintKindCount {
                kind: "text".to_string(),
                count: 1,
            }]
        );
        assert_eq!(dump.items[0].node, label);
        assert_eq!(dump.items[0].node_name.as_deref(), Some("status"));
    }

    #[test]
    fn debug_hit_trace_lists_candidates_and_layout_dump_lines() {
        let mut doc = UiDocument::new(fixed_style(200.0, 120.0));
        let child = doc.add_child(
            doc.root,
            UiNode::container("target", fixed_style(60.0, 30.0)).with_input(InputBehavior::BUTTON),
        );
        doc.compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let trace = DebugHitTrace::from_document(&doc, UiPoint::new(10.0, 10.0));
        let dump = layout_snapshot_dump(&doc.layout_snapshot());

        assert_eq!(trace.hit, Some(child));
        assert!(trace
            .candidates
            .iter()
            .any(|candidate| candidate.id == child && candidate.pointer));
        assert!(dump.contains("root#0 rect="));
        assert!(dump.contains("  target#1 rect="));
    }
}
