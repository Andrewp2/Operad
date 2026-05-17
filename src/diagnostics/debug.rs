//! Renderer-neutral debug snapshots and dumps.
//!
//! These helpers collect layout, paint, input, command-scope, repaint, and
//! timing state into plain data. Backends can render the data as overlays,
//! logs, or inspector panels without Operad depending on a debug UI renderer.

use std::collections::{BTreeMap, HashMap};

use crate::platform::UiLayer;
use crate::{
    AccessibilityNode, AnimatedValues, AnimationActiveTransitionSnapshot, AnimationBlendBinding,
    AnimationCondition, AnimationInputValue, AnimationNumberComparison, AnimationState,
    AnimationTransition, AnimationTrigger, AuditWarning, ColorRgba, CommandId, CommandScope,
    ComponentRole, ComponentState, ComponentStateSlot, DirtyFlags, FrameTiming, GestureEvent,
    GesturePhase, HostInteractionState, HostNodeInteraction, IconStyle, InputBehavior,
    IntrinsicSize, LayerEffect, LayoutSnapshot, MotionCurve, PaintKind, PaintList,
    ScopedThemeRegistry, ScrollState, StrokeStyle, TextMeasurer, TextStyle, Theme, ThemeScope,
    ThemeScopeError, ThemeScopeId, ThemeScopeKind, UiContent, UiDocument, UiNode, UiNodeId,
    UiPoint, UiRect, UiSize, UiVisual,
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
    pub min_resolved_z: Option<i32>,
    pub max_resolved_z: Option<i32>,
}

impl DebugPaintStats {
    fn record(&mut self, item: &crate::PaintItem) {
        self.count += 1;
        let z_index = item.z_index;
        let resolved_z = item.layer_order.resolved_z();
        self.min_z = Some(self.min_z.map_or(z_index, |z| z.min(z_index)));
        self.max_z = Some(self.max_z.map_or(z_index, |z| z.max(z_index)));
        self.min_resolved_z = Some(
            self.min_resolved_z
                .map_or(resolved_z, |z| z.min(resolved_z)),
        );
        self.max_resolved_z = Some(
            self.max_resolved_z
                .map_or(resolved_z, |z| z.max(resolved_z)),
        );
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
pub enum DebugNodeContentKind {
    Empty,
    Text,
    Canvas,
    Image,
    PaintRect,
    Scene,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugLayoutStyleSummary {
    pub display: String,
    pub flex_direction: String,
    pub flex_wrap: String,
    pub size: String,
    pub min_size: String,
    pub max_size: String,
    pub margin: String,
    pub padding: String,
    pub gap: String,
    pub position: String,
    pub inset: String,
    pub align_items: String,
    pub justify_content: String,
    pub clip: String,
    pub clip_scope: String,
    pub layer: Option<String>,
    pub z_index: i16,
    pub opacity: String,
    pub layout_constraint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugLayoutInspectorNode {
    pub id: UiNodeId,
    pub name: String,
    pub parent: Option<UiNodeId>,
    pub children: Vec<UiNodeId>,
    pub content_kind: DebugNodeContentKind,
    pub text: Option<String>,
    pub rect: UiRect,
    pub clip_rect: UiRect,
    pub final_size: UiSize,
    pub intrinsic_size: Option<IntrinsicSize>,
    pub layout_content_size: Option<UiSize>,
    pub visible: bool,
    pub input: InputBehavior,
    pub scroll: Option<ScrollState>,
    pub style: DebugLayoutStyleSummary,
    pub paint: DebugPaintStats,
    pub accessibility: Option<AccessibilityNode>,
    pub audit_warnings: Vec<AuditWarning>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugAnimationInspectorNode {
    pub id: UiNodeId,
    pub name: String,
    pub current_state: String,
    pub values: AnimatedValues,
    pub active_transition: Option<AnimationActiveTransitionSnapshot>,
    pub inputs: Vec<(String, AnimationInputValue)>,
    pub states: Vec<AnimationState>,
    pub transitions: Vec<AnimationTransition>,
    pub blend_bindings: Vec<AnimationBlendBinding>,
}

impl DebugAnimationInspectorNode {
    pub fn state_graph(&self) -> DebugAnimationGraph {
        DebugAnimationGraph::from_animation(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugAnimationGraph {
    pub states: Vec<DebugAnimationGraphState>,
    pub edges: Vec<DebugAnimationGraphEdge>,
}

impl DebugAnimationGraph {
    pub fn from_animation(animation: &DebugAnimationInspectorNode) -> Self {
        let target_state = animation
            .active_transition
            .as_ref()
            .map(|active| active.to_state_name.as_str());
        let states = animation
            .states
            .iter()
            .enumerate()
            .map(|(index, state)| DebugAnimationGraphState {
                index,
                name: state.name.clone(),
                current: state.name == animation.current_state,
                target: target_state == Some(state.name.as_str()),
            })
            .collect::<Vec<_>>();

        let mut edges = animation
            .transitions
            .iter()
            .map(|transition| {
                let active = animation.active_transition.as_ref().is_some_and(|active| {
                    active.from_state_name == transition.from
                        && active.to_state_name == transition.to
                });
                DebugAnimationGraphEdge {
                    from: transition.from.clone(),
                    to: transition.to.clone(),
                    label: animation_transition_label(transition),
                    kind: if transition.trigger.is_some() {
                        DebugAnimationGraphEdgeKind::Trigger
                    } else {
                        DebugAnimationGraphEdgeKind::Condition
                    },
                    active,
                    progress: active
                        .then(|| {
                            animation
                                .active_transition
                                .as_ref()
                                .map(|active| active.progress)
                        })
                        .flatten(),
                }
            })
            .collect::<Vec<_>>();

        edges.extend(animation.blend_bindings.iter().map(|binding| {
            let progress = animation.inputs.iter().find_map(|(name, value)| {
                if name == &binding.input {
                    value.as_number().map(|number| {
                        ((number - binding.min) / (binding.max - binding.min).max(f32::EPSILON))
                            .clamp(0.0, 1.0)
                    })
                } else {
                    None
                }
            });
            DebugAnimationGraphEdge {
                from: binding.from.clone(),
                to: binding.to.clone(),
                label: format!(
                    "blend {} [{:.2}..{:.2}]",
                    binding.input, binding.min, binding.max
                ),
                kind: DebugAnimationGraphEdgeKind::Blend,
                active: progress.is_some(),
                progress,
            }
        }));

        Self { states, edges }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugAnimationGraphState {
    pub index: usize,
    pub name: String,
    pub current: bool,
    pub target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugAnimationGraphEdgeKind {
    Trigger,
    Condition,
    Blend,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugAnimationGraphEdge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub kind: DebugAnimationGraphEdgeKind,
    pub active: bool,
    pub progress: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugAccessibilityOverlayNode {
    pub id: UiNodeId,
    pub name: String,
    pub rect: UiRect,
    pub focus_index: Option<usize>,
    pub accessibility: Option<AccessibilityNode>,
    pub warnings: Vec<AuditWarning>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugInspectorSnapshot {
    pub nodes: Vec<DebugLayoutInspectorNode>,
    pub animations: Vec<DebugAnimationInspectorNode>,
    pub accessibility_overlay: Vec<DebugAccessibilityOverlayNode>,
    pub audits: Vec<AuditWarning>,
}

impl DebugInspectorSnapshot {
    pub fn from_document(document: &UiDocument, text_measurer: &mut impl TextMeasurer) -> Self {
        let layout = document.layout_snapshot();
        let paint = document.paint_list();
        let paint_stats = paint_stats_by_node(&paint);
        let accessibility = document.accessibility_snapshot();
        let accessibility_by_node = accessibility
            .nodes
            .iter()
            .cloned()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let focus_order = accessibility
            .focus_order
            .iter()
            .copied()
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect::<HashMap<_, _>>();
        let audits = document.audit_layout();
        let audits_by_node = audits_by_node(&audits);

        let mut nodes = Vec::new();
        collect_layout_inspector_nodes(
            document,
            &layout,
            &paint_stats,
            &accessibility_by_node,
            &audits_by_node,
            text_measurer,
            &mut nodes,
        );

        let animations = document
            .nodes()
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let animation = node.animation.as_ref()?;
                Some(DebugAnimationInspectorNode {
                    id: UiNodeId(index),
                    name: node.name.clone(),
                    current_state: animation.current_state_name().to_owned(),
                    values: animation.values(),
                    active_transition: animation.active_transition(),
                    inputs: sorted_animation_inputs(animation.inputs()),
                    states: animation.states().to_vec(),
                    transitions: animation.transitions().to_vec(),
                    blend_bindings: animation.blend_bindings().to_vec(),
                })
            })
            .collect::<Vec<_>>();

        let accessibility_overlay = nodes
            .iter()
            .filter(|node| {
                node.accessibility.is_some()
                    || !node.audit_warnings.is_empty()
                    || focus_order.contains_key(&node.id)
            })
            .map(|node| DebugAccessibilityOverlayNode {
                id: node.id,
                name: node.name.clone(),
                rect: node.rect,
                focus_index: focus_order.get(&node.id).copied(),
                accessibility: node.accessibility.clone(),
                warnings: node.audit_warnings.clone(),
            })
            .collect();

        Self {
            nodes,
            animations,
            accessibility_overlay,
            audits,
        }
    }

    pub fn node(&self, name: &str) -> Option<&DebugLayoutInspectorNode> {
        self.nodes.iter().find(|node| node.name == name)
    }

    pub fn animation(&self, name: &str) -> Option<&DebugAnimationInspectorNode> {
        self.animations
            .iter()
            .find(|animation| animation.name == name)
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
    pub layer: UiLayer,
    pub resolved_z: i32,
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
                    layer: item.layer_order.layer,
                    resolved_z: item.layer_order.resolved_z(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebugThemeTokenKind {
    Theme,
    Color,
    Spacing,
    Typography,
    Radius,
    Stroke,
    Effect,
    Opacity,
    Motion,
    ComponentLayout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugThemeToken {
    pub path: String,
    pub kind: DebugThemeTokenKind,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugThemeScopeInfo {
    pub id: ThemeScopeId,
    pub kind: ThemeScopeKind,
    pub parent: Option<ThemeScopeId>,
}

impl DebugThemeScopeInfo {
    pub fn from_scope(scope: &ThemeScope) -> Self {
        Self {
            id: scope.id.clone(),
            kind: scope.kind.clone(),
            parent: scope.parent.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugThemeComponentState {
    pub role: ComponentRole,
    pub role_label: String,
    pub state: ComponentState,
    pub state_label: String,
    pub visual_slot: ComponentStateSlot,
    pub text_slot: ComponentStateSlot,
    pub icon_slot: ComponentStateSlot,
    pub fill: ColorRgba,
    pub stroke: Option<StrokeStyle>,
    pub corner_radius: f32,
    pub text_color: ColorRgba,
    pub icon_tint: ColorRgba,
    pub icon_opacity: f32,
    pub min_width: f32,
    pub min_height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub gap: f32,
    pub icon_size: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugThemeSnapshot {
    pub name: String,
    pub scope: Option<DebugThemeScopeInfo>,
    pub tokens: Vec<DebugThemeToken>,
    pub component_states: Vec<DebugThemeComponentState>,
}

impl DebugThemeSnapshot {
    pub fn from_theme(theme: &Theme) -> Self {
        let mut tokens = Vec::new();
        let mut component_states = Vec::new();
        collect_theme_tokens(theme, &mut tokens);
        collect_theme_component_states(theme, &mut tokens, &mut component_states);
        Self {
            name: theme.name.to_owned(),
            scope: None,
            tokens,
            component_states,
        }
    }

    pub fn from_registry_scope(
        registry: &ScopedThemeRegistry,
        scope_id: &ThemeScopeId,
    ) -> Result<Self, ThemeScopeError> {
        let scope = registry
            .scope(scope_id)
            .ok_or_else(|| ThemeScopeError::MissingScope(scope_id.clone()))?;
        let mut snapshot = Self::from_theme(&registry.resolve(scope_id)?);
        snapshot.scope = Some(DebugThemeScopeInfo::from_scope(scope));
        Ok(snapshot)
    }

    pub fn token(&self, path: &str) -> Option<&DebugThemeToken> {
        self.tokens.iter().find(|token| token.path == path)
    }

    pub fn tokens_with_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = &'a DebugThemeToken> + 'a {
        self.tokens
            .iter()
            .filter(move |token| token.path.starts_with(prefix))
    }

    pub fn component_state(
        &self,
        role: ComponentRole,
        state: ComponentState,
    ) -> Option<&DebugThemeComponentState> {
        self.component_states
            .iter()
            .find(|component| component.role == role && component.state == state)
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
                .then_with(|| right.paint.max_resolved_z.cmp(&left.paint.max_resolved_z))
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

fn collect_layout_inspector_nodes(
    document: &UiDocument,
    snapshot: &LayoutSnapshot,
    paint_stats: &HashMap<UiNodeId, DebugPaintStats>,
    accessibility_by_node: &HashMap<UiNodeId, AccessibilityNode>,
    audits_by_node: &HashMap<UiNodeId, Vec<AuditWarning>>,
    text_measurer: &mut impl TextMeasurer,
    out: &mut Vec<DebugLayoutInspectorNode>,
) {
    let node = document.node(snapshot.id);
    out.push(DebugLayoutInspectorNode {
        id: snapshot.id,
        name: node.name.clone(),
        parent: node.parent,
        children: node.children.clone(),
        content_kind: debug_content_kind(node),
        text: debug_node_text(node),
        rect: snapshot.rect,
        clip_rect: snapshot.clip_rect,
        final_size: UiSize::new(snapshot.rect.width, snapshot.rect.height),
        intrinsic_size: document.intrinsic_size(snapshot.id, text_measurer).ok(),
        layout_content_size: node.layout.content_size,
        visible: snapshot.visible,
        input: node.input,
        scroll: snapshot.scroll,
        style: debug_layout_style_summary(node),
        paint: paint_stats.get(&snapshot.id).copied().unwrap_or_default(),
        accessibility: accessibility_by_node.get(&snapshot.id).cloned(),
        audit_warnings: audits_by_node
            .get(&snapshot.id)
            .cloned()
            .unwrap_or_default(),
    });

    for child in &snapshot.children {
        collect_layout_inspector_nodes(
            document,
            child,
            paint_stats,
            accessibility_by_node,
            audits_by_node,
            text_measurer,
            out,
        );
    }
}

fn debug_content_kind(node: &UiNode) -> DebugNodeContentKind {
    match &node.content {
        UiContent::Empty => DebugNodeContentKind::Empty,
        UiContent::Text(_) => DebugNodeContentKind::Text,
        UiContent::Canvas(_) => DebugNodeContentKind::Canvas,
        UiContent::Image(_) => DebugNodeContentKind::Image,
        UiContent::PaintRect(_) => DebugNodeContentKind::PaintRect,
        UiContent::Scene(_) => DebugNodeContentKind::Scene,
    }
}

fn debug_node_text(node: &UiNode) -> Option<String> {
    match &node.content {
        UiContent::Text(text) => Some(text.text.clone()),
        _ => None,
    }
}

fn debug_layout_style_summary(node: &UiNode) -> DebugLayoutStyleSummary {
    let style = &node.style.layout;
    DebugLayoutStyleSummary {
        display: format!("{:?}", style.display),
        flex_direction: format!("{:?}", style.flex_direction),
        flex_wrap: format!("{:?}", style.flex_wrap),
        size: format!("{:?}", style.size),
        min_size: format!("{:?}", style.min_size),
        max_size: format!("{:?}", style.max_size),
        margin: format!("{:?}", style.margin),
        padding: format!("{:?}", style.padding),
        gap: format!("{:?}", style.gap),
        position: format!("{:?}", style.position),
        inset: format!("{:?}", style.inset),
        align_items: format!("{:?}", style.align_items),
        justify_content: format!("{:?}", style.justify_content),
        clip: format!("{:?}", node.style.clip),
        clip_scope: format!("{:?}", node.clip_scope),
        layer: node.layer.map(|layer| format!("{layer:?}")),
        z_index: node.style.z_index,
        opacity: format!("{:.3}", node.style.opacity),
        layout_constraint: node
            .layout_constraint
            .as_ref()
            .map(|constraint| format!("{constraint:?}")),
    }
}

fn sorted_animation_inputs(
    inputs: &HashMap<String, AnimationInputValue>,
) -> Vec<(String, AnimationInputValue)> {
    let mut inputs = inputs
        .iter()
        .map(|(name, value)| (name.clone(), *value))
        .collect::<Vec<_>>();
    inputs.sort_by(|left, right| left.0.cmp(&right.0));
    inputs
}

fn animation_transition_label(transition: &AnimationTransition) -> String {
    let mut parts = Vec::new();
    if let Some(trigger) = &transition.trigger {
        parts.push(format!("on {}", animation_trigger_label(trigger)));
    }
    if !transition.conditions.is_empty() {
        parts.push(
            transition
                .conditions
                .iter()
                .map(animation_condition_label)
                .collect::<Vec<_>>()
                .join(" && "),
        );
    }
    if parts.is_empty() {
        parts.push("always".to_owned());
    }
    parts.push(format!("{:.2}s", transition.duration_seconds));
    parts.join("; ")
}

fn animation_trigger_label(trigger: &AnimationTrigger) -> String {
    match trigger {
        AnimationTrigger::PointerEnter => "pointer enter".to_owned(),
        AnimationTrigger::PointerLeave => "pointer leave".to_owned(),
        AnimationTrigger::FocusGained => "focus gained".to_owned(),
        AnimationTrigger::FocusLost => "focus lost".to_owned(),
        AnimationTrigger::Pressed => "pressed".to_owned(),
        AnimationTrigger::Released => "released".to_owned(),
        AnimationTrigger::Custom(name) => name.clone(),
    }
}

fn animation_condition_label(condition: &AnimationCondition) -> String {
    match condition {
        AnimationCondition::Bool { input, value } => format!("{input} == {value}"),
        AnimationCondition::Number {
            input,
            comparison,
            value,
        } => format!(
            "{input} {} {:.3}",
            animation_number_comparison_label(*comparison),
            value
        ),
        AnimationCondition::Trigger { input } => format!("{input} fired"),
    }
}

fn animation_number_comparison_label(comparison: AnimationNumberComparison) -> &'static str {
    match comparison {
        AnimationNumberComparison::LessThan => "<",
        AnimationNumberComparison::LessThanOrEqual => "<=",
        AnimationNumberComparison::GreaterThan => ">",
        AnimationNumberComparison::GreaterThanOrEqual => ">=",
        AnimationNumberComparison::Equal => "==",
        AnimationNumberComparison::NotEqual => "!=",
    }
}

fn audits_by_node(audits: &[AuditWarning]) -> HashMap<UiNodeId, Vec<AuditWarning>> {
    let mut by_node = HashMap::<UiNodeId, Vec<AuditWarning>>::new();
    for audit in audits {
        let Some(node) = audit_node_id(audit) else {
            continue;
        };
        by_node.entry(node).or_default().push(audit.clone());
    }
    by_node
}

fn audit_node_id(audit: &AuditWarning) -> Option<UiNodeId> {
    match audit {
        AuditWarning::NonFiniteRect { node, .. }
        | AuditWarning::InvisibleInteractiveNode { node, .. }
        | AuditWarning::EmptyInteractiveClip { node, .. }
        | AuditWarning::InteractiveTooSmall { node, .. }
        | AuditWarning::FocusableMissingFromAccessibilityTree { node, .. }
        | AuditWarning::InteractiveAccessibilityMissing { node, .. }
        | AuditWarning::AccessibleNameMissing { node, .. }
        | AuditWarning::AccessibilityActionMissing { node, .. }
        | AuditWarning::AccessibilityActionIdMissing { node, .. }
        | AuditWarning::AccessibilityActionLabelMissing { node, .. }
        | AuditWarning::AccessibilityActionDuplicate { node, .. }
        | AuditWarning::AccessibilityStateMissing { node, .. }
        | AuditWarning::AccessibilityValueMissing { node, .. }
        | AuditWarning::AccessibilityValueRangeMissing { node, .. }
        | AuditWarning::AccessibilityValueRangeInvalid { node, .. }
        | AuditWarning::AccessibilityRelationTargetMissing { node, .. }
        | AuditWarning::TextClipped { node, .. }
        | AuditWarning::TextContrastTooLow { node, .. }
        | AuditWarning::NodeOutsideRoot { node, .. }
        | AuditWarning::PaintItemEmptyClip { node } => Some(*node),
        AuditWarning::DuplicateNodeName { .. } => None,
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
            .record(item);
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
        PaintKind::CompositedLayer(_) => "composited_layer",
        PaintKind::RichRect(_) => "rich_rect",
        PaintKind::SceneText(_) => "scene_text",
        PaintKind::Path(_) => "path",
        PaintKind::ImagePlacement(_) => "image_placement",
    }
}

fn collect_theme_tokens(theme: &Theme, tokens: &mut Vec<DebugThemeToken>) {
    push_token(
        tokens,
        DebugThemeTokenKind::Theme,
        "theme.name",
        theme.name.to_owned(),
    );

    push_color(tokens, "colors.canvas", theme.colors.canvas);
    push_color(tokens, "colors.canvas_subtle", theme.colors.canvas_subtle);
    push_color(tokens, "colors.surface", theme.colors.surface);
    push_color(tokens, "colors.surface_muted", theme.colors.surface_muted);
    push_color(
        tokens,
        "colors.surface_elevated",
        theme.colors.surface_elevated,
    );
    push_color(
        tokens,
        "colors.surface_overlay",
        theme.colors.surface_overlay,
    );
    push_color(tokens, "colors.surface_sunken", theme.colors.surface_sunken);
    push_color(tokens, "colors.border", theme.colors.border);
    push_color(tokens, "colors.border_muted", theme.colors.border_muted);
    push_color(tokens, "colors.border_strong", theme.colors.border_strong);
    push_color(tokens, "colors.divider", theme.colors.divider);
    push_color(tokens, "colors.text", theme.colors.text);
    push_color(tokens, "colors.text_muted", theme.colors.text_muted);
    push_color(tokens, "colors.text_subtle", theme.colors.text_subtle);
    push_color(tokens, "colors.text_disabled", theme.colors.text_disabled);
    push_color(tokens, "colors.text_inverse", theme.colors.text_inverse);
    push_color(tokens, "colors.accent", theme.colors.accent);
    push_color(tokens, "colors.accent_hover", theme.colors.accent_hover);
    push_color(tokens, "colors.accent_pressed", theme.colors.accent_pressed);
    push_color(tokens, "colors.accent_muted", theme.colors.accent_muted);
    push_color(tokens, "colors.accent_strong", theme.colors.accent_strong);
    push_color(tokens, "colors.accent_text", theme.colors.accent_text);
    push_color(tokens, "colors.success", theme.colors.success);
    push_color(tokens, "colors.warning", theme.colors.warning);
    push_color(tokens, "colors.danger", theme.colors.danger);
    push_color(tokens, "colors.info", theme.colors.info);
    push_color(tokens, "colors.selected", theme.colors.selected);
    push_color(tokens, "colors.selected_hover", theme.colors.selected_hover);
    push_color(tokens, "colors.selected_text", theme.colors.selected_text);
    push_color(tokens, "colors.focus_ring", theme.colors.focus_ring);
    push_color(tokens, "colors.overlay_scrim", theme.colors.overlay_scrim);
    push_color(
        tokens,
        "colors.editor_background",
        theme.colors.editor_background,
    );
    push_color(
        tokens,
        "colors.editor_grid_major",
        theme.colors.editor_grid_major,
    );
    push_color(
        tokens,
        "colors.editor_grid_minor",
        theme.colors.editor_grid_minor,
    );
    push_color(tokens, "colors.lane_header", theme.colors.lane_header);
    push_color(
        tokens,
        "colors.lane_header_selected",
        theme.colors.lane_header_selected,
    );
    push_color(
        tokens,
        "colors.range_item_primary",
        theme.colors.range_item_primary,
    );
    push_color(
        tokens,
        "colors.range_item_secondary",
        theme.colors.range_item_secondary,
    );
    push_color(
        tokens,
        "colors.range_item_accent",
        theme.colors.range_item_accent,
    );
    push_color(tokens, "colors.editor_lane", theme.colors.editor_lane);
    push_color(
        tokens,
        "colors.editor_lane_alternate",
        theme.colors.editor_lane_alternate,
    );
    push_color(
        tokens,
        "colors.transport_active",
        theme.colors.transport_active,
    );

    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.none",
        theme.spacing.none,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.xxxs",
        theme.spacing.xxxs,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.xxs",
        theme.spacing.xxs,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.xs",
        theme.spacing.xs,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.sm",
        theme.spacing.sm,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.md",
        theme.spacing.md,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.lg",
        theme.spacing.lg,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.xl",
        theme.spacing.xl,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.xxl",
        theme.spacing.xxl,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.control_x",
        theme.spacing.control_x,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.control_y",
        theme.spacing.control_y,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.panel",
        theme.spacing.panel,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.toolbar_gap",
        theme.spacing.toolbar_gap,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.row_gap",
        theme.spacing.row_gap,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Spacing,
        "spacing.grid",
        theme.spacing.grid,
    );

    push_text_style(tokens, "typography.caption", &theme.typography.caption);
    push_text_style(
        tokens,
        "typography.caption_strong",
        &theme.typography.caption_strong,
    );
    push_text_style(tokens, "typography.body", &theme.typography.body);
    push_text_style(
        tokens,
        "typography.body_strong",
        &theme.typography.body_strong,
    );
    push_text_style(tokens, "typography.label", &theme.typography.label);
    push_text_style(
        tokens,
        "typography.label_strong",
        &theme.typography.label_strong,
    );
    push_text_style(tokens, "typography.heading", &theme.typography.heading);
    push_text_style(tokens, "typography.title", &theme.typography.title);
    push_text_style(tokens, "typography.mono", &theme.typography.mono);
    push_text_style(tokens, "typography.numeric", &theme.typography.numeric);
    push_text_style(tokens, "typography.disabled", &theme.typography.disabled);

    push_f32(
        tokens,
        DebugThemeTokenKind::Radius,
        "radius.none",
        theme.radius.none,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Radius,
        "radius.xs",
        theme.radius.xs,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Radius,
        "radius.sm",
        theme.radius.sm,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Radius,
        "radius.md",
        theme.radius.md,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Radius,
        "radius.lg",
        theme.radius.lg,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Radius,
        "radius.xl",
        theme.radius.xl,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Radius,
        "radius.pill",
        theme.radius.pill,
    );

    push_f32(
        tokens,
        DebugThemeTokenKind::Stroke,
        "stroke.hairline_width",
        theme.stroke.hairline_width,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Stroke,
        "stroke.thin_width",
        theme.stroke.thin_width,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Stroke,
        "stroke.medium_width",
        theme.stroke.medium_width,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Stroke,
        "stroke.strong_width",
        theme.stroke.strong_width,
    );
    push_stroke(tokens, "stroke.divider", theme.stroke.divider);
    push_stroke(tokens, "stroke.surface", theme.stroke.surface);
    push_stroke(tokens, "stroke.surface_strong", theme.stroke.surface_strong);
    push_stroke(tokens, "stroke.control", theme.stroke.control);
    push_stroke(tokens, "stroke.control_hover", theme.stroke.control_hover);
    push_stroke(tokens, "stroke.focus", theme.stroke.focus);
    push_stroke(tokens, "stroke.selected", theme.stroke.selected);
    push_stroke(tokens, "stroke.invalid", theme.stroke.invalid);
    push_stroke(tokens, "stroke.warning", theme.stroke.warning);

    push_effect(tokens, "effects.panel_shadow", theme.effects.panel_shadow);
    push_effect(
        tokens,
        "effects.floating_shadow",
        theme.effects.floating_shadow,
    );
    push_effect(
        tokens,
        "effects.popover_shadow",
        theme.effects.popover_shadow,
    );
    push_effect(tokens, "effects.focus_glow", theme.effects.focus_glow);
    push_effect(tokens, "effects.accent_glow", theme.effects.accent_glow);
    push_effect(tokens, "effects.danger_glow", theme.effects.danger_glow);
    push_effect(
        tokens,
        "effects.inset_hairline",
        theme.effects.inset_hairline,
    );

    push_f32(
        tokens,
        DebugThemeTokenKind::Opacity,
        "opacity.opaque",
        theme.opacity.opaque,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Opacity,
        "opacity.hover_overlay",
        theme.opacity.hover_overlay,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Opacity,
        "opacity.pressed_overlay",
        theme.opacity.pressed_overlay,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Opacity,
        "opacity.selected_overlay",
        theme.opacity.selected_overlay,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Opacity,
        "opacity.disabled",
        theme.opacity.disabled,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Opacity,
        "opacity.muted",
        theme.opacity.muted,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Opacity,
        "opacity.scrim",
        theme.opacity.scrim,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Opacity,
        "opacity.drag_preview",
        theme.opacity.drag_preview,
    );
    push_f32(
        tokens,
        DebugThemeTokenKind::Opacity,
        "opacity.focus_glow",
        theme.opacity.focus_glow,
    );

    push_token(
        tokens,
        DebugThemeTokenKind::Motion,
        "motion.instant_ms",
        theme.motion.instant_ms.to_string(),
    );
    push_token(
        tokens,
        DebugThemeTokenKind::Motion,
        "motion.micro_ms",
        theme.motion.micro_ms.to_string(),
    );
    push_token(
        tokens,
        DebugThemeTokenKind::Motion,
        "motion.fast_ms",
        theme.motion.fast_ms.to_string(),
    );
    push_token(
        tokens,
        DebugThemeTokenKind::Motion,
        "motion.normal_ms",
        theme.motion.normal_ms.to_string(),
    );
    push_token(
        tokens,
        DebugThemeTokenKind::Motion,
        "motion.slow_ms",
        theme.motion.slow_ms.to_string(),
    );
    push_token(
        tokens,
        DebugThemeTokenKind::Motion,
        "motion.tooltip_delay_ms",
        theme.motion.tooltip_delay_ms.to_string(),
    );
    push_motion_curve(tokens, "motion.standard", theme.motion.standard);
    push_motion_curve(tokens, "motion.emphasized", theme.motion.emphasized);
    push_motion_curve(tokens, "motion.exit", theme.motion.exit);
    push_f32(
        tokens,
        DebugThemeTokenKind::Motion,
        "motion.reduced_motion_scale",
        theme.motion.reduced_motion_scale,
    );
}

fn collect_theme_component_states(
    theme: &Theme,
    tokens: &mut Vec<DebugThemeToken>,
    out: &mut Vec<DebugThemeComponentState>,
) {
    for (role, role_label) in component_roles() {
        let component = theme.component(role);
        let layout = component.layout;
        push_token(
            tokens,
            DebugThemeTokenKind::ComponentLayout,
            &format!("components.{role_label}.layout"),
            format!(
                "min={:.1}x{:.1} padding={:.1}x{:.1} gap={:.1} icon={:.1}",
                layout.min_width,
                layout.min_height,
                layout.padding_x,
                layout.padding_y,
                layout.gap,
                layout.icon_size
            ),
        );

        for (state, state_label) in component_states() {
            let (visual_slot, visual) = component.visual.resolve_slot(state);
            let (text_slot, text) = component.text.resolve_slot(state);
            let (icon_slot, icon) = component.icon.resolve_slot(state);
            out.push(component_state_snapshot(
                role,
                role_label,
                state,
                state_label,
                visual_slot,
                visual,
                text_slot,
                &text,
                icon_slot,
                icon,
                layout,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn component_state_snapshot(
    role: ComponentRole,
    role_label: &str,
    state: ComponentState,
    state_label: &str,
    visual_slot: ComponentStateSlot,
    visual: UiVisual,
    text_slot: ComponentStateSlot,
    text: &TextStyle,
    icon_slot: ComponentStateSlot,
    icon: IconStyle,
    layout: crate::ComponentLayoutTokens,
) -> DebugThemeComponentState {
    DebugThemeComponentState {
        role,
        role_label: role_label.to_owned(),
        state,
        state_label: state_label.to_owned(),
        visual_slot,
        text_slot,
        icon_slot,
        fill: visual.fill,
        stroke: visual.stroke,
        corner_radius: visual.corner_radius,
        text_color: text.color,
        icon_tint: icon.tint,
        icon_opacity: icon.opacity,
        min_width: layout.min_width,
        min_height: layout.min_height,
        padding_x: layout.padding_x,
        padding_y: layout.padding_y,
        gap: layout.gap,
        icon_size: layout.icon_size,
    }
}

fn component_roles() -> [(ComponentRole, &'static str); 9] {
    [
        (ComponentRole::Button, "button"),
        (ComponentRole::Tab, "tab"),
        (ComponentRole::SearchField, "search_field"),
        (ComponentRole::LaneHeader, "lane_header"),
        (ComponentRole::RangeItem, "range_item"),
        (ComponentRole::EditorLane, "editor_lane"),
        (ComponentRole::PropertyRow, "property_row"),
        (ComponentRole::MenuRow, "menu_row"),
        (ComponentRole::TransportControl, "transport_control"),
    ]
}

fn component_states() -> [(ComponentState, &'static str); 13] {
    [
        (ComponentState::NORMAL, "normal"),
        (ComponentState::HOVERED, "hovered"),
        (ComponentState::PRESSED, "pressed"),
        (ComponentState::FOCUSED, "focused"),
        (ComponentState::SELECTED, "selected"),
        (ComponentState::ACTIVE, "active"),
        (ComponentState::INVALID, "invalid"),
        (ComponentState::WARNING, "warning"),
        (ComponentState::CHANGED, "changed"),
        (ComponentState::PENDING, "pending"),
        (ComponentState::OPEN, "open"),
        (ComponentState::CHECKED, "checked"),
        (ComponentState::DISABLED, "disabled"),
    ]
}

fn push_color(tokens: &mut Vec<DebugThemeToken>, path: &str, color: ColorRgba) {
    push_token(tokens, DebugThemeTokenKind::Color, path, color_value(color));
}

fn push_f32(tokens: &mut Vec<DebugThemeToken>, kind: DebugThemeTokenKind, path: &str, value: f32) {
    push_token(tokens, kind, path, format!("{value:.3}"));
}

fn push_text_style(tokens: &mut Vec<DebugThemeToken>, path: &str, style: &TextStyle) {
    push_token(
        tokens,
        DebugThemeTokenKind::Typography,
        path,
        format!(
            "size={:.1} line={:.1} family={:?} weight={} style={:?} stretch={:?} wrap={:?} color={}",
            style.font_size,
            style.line_height,
            style.family,
            style.weight.0,
            style.style,
            style.stretch,
            style.wrap,
            color_value(style.color)
        ),
    );
}

fn push_stroke(tokens: &mut Vec<DebugThemeToken>, path: &str, stroke: StrokeStyle) {
    push_token(
        tokens,
        DebugThemeTokenKind::Stroke,
        path,
        stroke_value(stroke),
    );
}

fn push_effect(tokens: &mut Vec<DebugThemeToken>, path: &str, effect: LayerEffect) {
    push_token(
        tokens,
        DebugThemeTokenKind::Effect,
        path,
        format!(
            "kind={:?} color={} offset={:.1},{:.1} blur={:.1} spread={:.1} opacity={:.3} fallback={}",
            effect.kind,
            color_value(effect.color),
            effect.offset_x,
            effect.offset_y,
            effect.blur_radius,
            effect.spread,
            effect.opacity,
            effect
                .fallback_stroke
                .map(stroke_value)
                .unwrap_or_else(|| "none".to_owned())
        ),
    );
}

fn push_motion_curve(tokens: &mut Vec<DebugThemeToken>, path: &str, curve: MotionCurve) {
    push_token(
        tokens,
        DebugThemeTokenKind::Motion,
        path,
        format!("{curve:?}"),
    );
}

fn push_token(
    tokens: &mut Vec<DebugThemeToken>,
    kind: DebugThemeTokenKind,
    path: &str,
    value: String,
) {
    tokens.push(DebugThemeToken {
        path: path.to_owned(),
        kind,
        value,
    });
}

fn color_value(color: ColorRgba) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        color.r, color.g, color.b, color.a
    )
}

fn stroke_value(stroke: StrokeStyle) -> String {
    format!(
        "width={:.3} color={}",
        stroke.width,
        color_value(stroke.color)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        length, ApproxTextMeasurer, ColorRgba, ComponentRole, ComponentState, ComponentStateSlot,
        InputBehavior, LayoutStyle, RawWheelEvent, ScopedThemeRegistry, StrokeStyle, TextStyle,
        Theme, ThemePatch, ThemeScope, ThemeScopeId, ThemeScopeKind, UiNode, UiNodeStyle, UiSize,
        UiVisual, WheelDeltaUnit, WheelPhase,
    };
    use taffy::prelude::{
        Dimension, LengthPercentageAuto, Position, Rect, Size as TaffySize, Style,
    };

    fn fixed_style(width: f32, height: f32) -> UiNodeStyle {
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: length(width),
                    height: length(height),
                },
                ..Default::default()
            })
            .style,
            ..Default::default()
        }
    }

    fn layered_absolute_style(z_index: i16, width: f32, height: f32) -> UiNodeStyle {
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                position: Position::Absolute,
                inset: Rect {
                    left: LengthPercentageAuto::length(0.0),
                    top: LengthPercentageAuto::length(0.0),
                    ..Rect::length(0.0)
                },
                size: TaffySize {
                    width: length(width),
                    height: length(height),
                },
                ..Default::default()
            })
            .style,
            z_index,
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
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
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
        assert_eq!(dump.items[0].layer, UiLayer::AppContent);
        assert_eq!(dump.items[0].resolved_z, UiLayer::AppContent.base_z());
    }

    #[test]
    fn debug_inspector_snapshot_combines_layout_animation_and_accessibility() {
        let mut doc = UiDocument::new(fixed_style(240.0, 160.0));
        let button = doc.add_child(
            doc.root,
            UiNode::container("play", fixed_style(96.0, 36.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    crate::AccessibilityMeta::new(crate::AccessibilityRole::Button)
                        .label("Play")
                        .focusable()
                        .action(crate::AccessibilityAction::new("activate", "Activate")),
                )
                .with_animation(
                    crate::AnimationMachine::new(
                        vec![
                            crate::AnimationState::new(
                                "idle",
                                crate::AnimatedValues::new(1.0, crate::UiPoint::new(0.0, 0.0), 1.0),
                            ),
                            crate::AnimationState::new(
                                "armed",
                                crate::AnimatedValues::new(1.0, crate::UiPoint::new(4.0, 0.0), 1.0),
                            ),
                        ],
                        Vec::new(),
                        "idle",
                    )
                    .expect("animation")
                    .with_number_input("hover", 0.25)
                    .with_blend_binding(crate::AnimationBlendBinding::new(
                        "hover", "idle", "armed",
                    )),
                )
                .with_visual(UiVisual::panel(ColorRgba::new(20, 80, 140, 255), None, 4.0)),
        );
        doc.compute_layout(UiSize::new(240.0, 160.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let snapshot = DebugInspectorSnapshot::from_document(&doc, &mut ApproxTextMeasurer);
        let node = snapshot.node("play").expect("play inspector node");

        assert_eq!(node.id, button);
        assert_eq!(node.final_size, UiSize::new(96.0, 36.0));
        assert!(node.intrinsic_size.is_some());
        assert_eq!(
            node.accessibility
                .as_ref()
                .and_then(|meta| meta.label.as_deref()),
            Some("Play")
        );
        assert!(snapshot
            .accessibility_overlay
            .iter()
            .any(|overlay| overlay.id == button && overlay.focus_index == Some(0)));

        let animation = snapshot.animation("play").expect("animation inspector");
        assert_eq!(animation.current_state, "idle");
        assert_eq!(animation.inputs[0].0, "hover");
        assert_eq!(animation.states.len(), 2);
        assert_eq!(animation.blend_bindings.len(), 1);
    }

    #[test]
    fn debug_paint_dump_exposes_platform_layer_order() {
        let mut doc = UiDocument::new(fixed_style(200.0, 120.0));
        let overlay = doc.add_child(
            doc.root,
            UiNode::container(
                "debug_overlay",
                UiNodeStyle {
                    z_index: -10,
                    ..fixed_style(80.0, 40.0)
                },
            )
            .with_layer(UiLayer::DebugOverlay)
            .with_visual(UiVisual::panel(ColorRgba::new(20, 30, 40, 255), None, 4.0)),
        );
        doc.compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let dump = DebugPaintDump::from_document(&doc);
        let item = dump
            .items
            .iter()
            .find(|item| item.node == overlay)
            .expect("overlay paint item");

        assert_eq!(item.layer, UiLayer::DebugOverlay);
        assert_eq!(item.z_index, -10);
        assert_eq!(item.resolved_z, UiLayer::DebugOverlay.base_z() - 10);
    }

    #[test]
    fn debug_theme_snapshot_exposes_tokens_and_component_states() {
        let snapshot = DebugThemeSnapshot::from_theme(&Theme::dark());

        assert_eq!(
            snapshot.token("colors.transport_active").unwrap().value,
            "#5CD4A5FF"
        );
        assert_eq!(
            snapshot.token("spacing.grid").unwrap().kind,
            DebugThemeTokenKind::Spacing
        );
        assert!(snapshot.tokens_with_prefix("typography.").count() >= 8);

        let button = snapshot
            .component_state(ComponentRole::Button, ComponentState::NORMAL)
            .unwrap();
        assert_eq!(button.role_label, "button");
        assert_eq!(button.state_label, "normal");
        assert_eq!(button.visual_slot, ComponentStateSlot::Base);
        assert!(snapshot.component_states.len() >= 100);
    }

    #[test]
    fn debug_theme_snapshot_resolves_scoped_theme_tokens() {
        let base = Theme::dark();
        let mut editor_colors = base.colors;
        editor_colors.editor_background = ColorRgba::new(1, 2, 3, 255);
        let registry = ScopedThemeRegistry::new(base).with_scope(
            ThemeScope::editor_surface("editor-grid")
                .with_patch(ThemePatch::new().colors(editor_colors)),
        );

        let snapshot =
            DebugThemeSnapshot::from_registry_scope(&registry, &ThemeScopeId::new("editor-grid"))
                .unwrap();

        assert_eq!(
            snapshot.scope.as_ref().unwrap().kind,
            ThemeScopeKind::EditorSurface
        );
        assert_eq!(
            snapshot.token("colors.editor_background").unwrap().value,
            "#010203FF"
        );
        assert!(
            DebugThemeSnapshot::from_registry_scope(&registry, &ThemeScopeId::new("missing"))
                .is_err()
        );
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

    #[test]
    fn debug_hit_trace_sorts_candidates_by_resolved_layer_order() {
        let mut doc = UiDocument::new(fixed_style(200.0, 120.0));
        let app_overlay = doc.add_child(
            doc.root,
            UiNode::container(
                "app_overlay",
                layered_absolute_style(crate::platform::LAYER_LOCAL_Z_MAX, 80.0, 40.0),
            )
            .with_layer(UiLayer::AppOverlay)
            .with_input(InputBehavior::BUTTON)
            .with_visual(UiVisual::panel(ColorRgba::new(20, 80, 140, 255), None, 0.0)),
        );
        let debug_overlay = doc.add_child(
            doc.root,
            UiNode::container(
                "debug_overlay",
                layered_absolute_style(crate::platform::LAYER_LOCAL_Z_MIN, 80.0, 40.0),
            )
            .with_layer(UiLayer::DebugOverlay)
            .with_input(InputBehavior::BUTTON)
            .with_visual(UiVisual::panel(ColorRgba::new(180, 40, 40, 255), None, 0.0)),
        );
        doc.compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let trace = DebugHitTrace::from_document(&doc, UiPoint::new(10.0, 10.0));

        assert_eq!(trace.hit, Some(debug_overlay));
        assert_eq!(trace.candidates[0].id, debug_overlay);
        assert_eq!(trace.candidates[1].id, app_overlay);
        assert!(
            trace.candidates[0].paint.max_resolved_z > trace.candidates[1].paint.max_resolved_z
        );
    }
}
