use super::*;
use crate::layout::tooltip_layout_rect;
pub use crate::layout::tooltip_rect;
use crate::tooltips::{
    resolve_tooltip_request, HelpItemState, HelpTimingPolicy, TooltipAnchor, TooltipContent,
    TooltipInvocationKind, TooltipPlacement, TooltipRequest, TooltipResolution,
};

#[derive(Debug, Clone)]
pub struct TooltipBoxOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub title_text_style: TextStyle,
    pub body_text_style: TextStyle,
    pub shortcut_text_style: TextStyle,
    pub animation: Option<AnimationMachine>,
    pub z_index: f32,
    pub layer: crate::platform::UiLayer,
    pub clip_scope: ClipScope,
    pub portal: UiPortalTarget,
    pub accessibility_label: Option<String>,
}

impl Default for TooltipBoxOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column()
                .with_width(240.0)
                .with_padding(8.0)
                .with_gap(4.0),
            visual: UiVisual::panel(
                ColorRgba::new(18, 23, 31, 245),
                Some(StrokeStyle::new(ColorRgba::new(92, 106, 128, 255), 1.0)),
                4.0,
            ),
            title_text_style: TextStyle {
                font_size: 14.0,
                line_height: 18.0,
                weight: FontWeight::BOLD,
                ..Default::default()
            },
            body_text_style: TextStyle {
                font_size: 13.0,
                line_height: 18.0,
                color: ColorRgba::new(198, 207, 219, 255),
                ..Default::default()
            },
            shortcut_text_style: TextStyle {
                font_size: 12.0,
                line_height: 16.0,
                color: ColorRgba::new(154, 168, 188, 255),
                ..Default::default()
            },
            animation: Some(tooltip_fade_slide_animation(true, false)),
            z_index: 100.0,
            layer: crate::platform::UiLayer::AppOverlay,
            clip_scope: ClipScope::Viewport,
            portal: UiPortalTarget::Parent,
            accessibility_label: None,
        }
    }
}

impl TooltipBoxOptions {
    pub fn at_rect(mut self, rect: UiRect) -> Self {
        self.layout = LayoutStyle::absolute_rect(rect);
        self
    }

    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_animation(mut self, animation: impl Into<Option<AnimationMachine>>) -> Self {
        self.animation = animation.into();
        self
    }

    pub const fn with_clip_scope(mut self, clip_scope: ClipScope) -> Self {
        self.clip_scope = clip_scope;
        self
    }

    pub fn with_portal(mut self, portal: UiPortalTarget) -> Self {
        self.portal = portal;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TooltipTriggerMode {
    Hover,
    Focus,
    HoverOrFocus,
}

impl TooltipTriggerMode {
    pub const fn allows_hover(self) -> bool {
        matches!(self, Self::Hover | Self::HoverOrFocus)
    }

    pub const fn allows_focus(self) -> bool {
        matches!(self, Self::Focus | Self::HoverOrFocus)
    }
}

impl Default for TooltipTriggerMode {
    fn default() -> Self {
        Self::HoverOrFocus
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TooltipTriggerOptions {
    pub mode: TooltipTriggerMode,
    pub placement: TooltipPlacement,
    pub timing: HelpTimingPolicy,
    pub item_state: HelpItemState,
}

impl TooltipTriggerOptions {
    pub const fn hover_only(mut self) -> Self {
        self.mode = TooltipTriggerMode::Hover;
        self
    }

    pub const fn focus_only(mut self) -> Self {
        self.mode = TooltipTriggerMode::Focus;
        self
    }

    pub const fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub const fn timing(mut self, timing: HelpTimingPolicy) -> Self {
        self.timing = timing;
        self
    }

    pub const fn immediate(mut self) -> Self {
        self.timing = HelpTimingPolicy::immediate();
        self
    }

    pub const fn item_state(mut self, item_state: HelpItemState) -> Self {
        self.item_state = item_state;
        self
    }
}

impl Default for TooltipTriggerOptions {
    fn default() -> Self {
        Self {
            mode: TooltipTriggerMode::HoverOrFocus,
            placement: TooltipPlacement::default(),
            timing: HelpTimingPolicy::default(),
            item_state: HelpItemState::ENABLED,
        }
    }
}

pub const TOOLTIP_SHOW_TRIGGER: &str = "tooltip.show";
pub const TOOLTIP_HIDE_TRIGGER: &str = "tooltip.hide";

pub fn tooltip_fade_slide_animation(
    initially_visible: bool,
    reduced_motion: bool,
) -> AnimationMachine {
    let show_duration = if reduced_motion { 0.0 } else { 0.12 };
    let hide_duration = if reduced_motion { 0.0 } else { 0.08 };
    let initial = if initially_visible {
        "visible"
    } else {
        "hidden"
    };
    let fallback_values = if initially_visible {
        AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0)
    } else {
        AnimatedValues::new(0.0, UiPoint::new(0.0, 4.0), 0.99)
    };
    AnimationMachine::new(
        vec![
            AnimationState::new(
                "hidden",
                AnimatedValues::new(0.0, UiPoint::new(0.0, 4.0), 0.99),
            ),
            AnimationState::new(
                "visible",
                AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0),
            ),
        ],
        vec![
            AnimationTransition::new(
                "hidden",
                "visible",
                AnimationTrigger::Custom(TOOLTIP_SHOW_TRIGGER.to_owned()),
                show_duration,
            ),
            AnimationTransition::new(
                "visible",
                "hidden",
                AnimationTrigger::Custom(TOOLTIP_HIDE_TRIGGER.to_owned()),
                hide_duration,
            ),
        ],
        initial,
    )
    .unwrap_or_else(|_| AnimationMachine::single_state(initial, fallback_values))
}

/// Resolve hover or focus help using the target's painted bounds.
pub fn tooltip_trigger_resolution(
    document: &UiDocument,
    target: UiNodeId,
    content: TooltipContent,
    input: &UiInputResult,
    now_ms: u64,
    options: TooltipTriggerOptions,
) -> TooltipResolution {
    let modal_scope = document.accessibility_modal_scope();
    if !document.node_in_modal_scope(target, modal_scope) {
        return TooltipResolution::hidden();
    }
    // Disabled controls may still explain why they are unavailable. HelpItemState
    // governs that policy; action eligibility is deliberately not used here.
    let matches_target = |node| {
        document.node_in_modal_scope(node, modal_scope)
            && document.node_is_logical_descendant_or_self(target, node)
    };
    let anchor = tooltip_anchor(document, target);
    let hover =
        (options.mode.allows_hover() && input.hovered.is_some_and(matches_target)).then(|| {
            TooltipRequest::new(anchor, content.clone())
                .placement(options.placement)
                .invocation(TooltipInvocationKind::Hover)
        });
    let focus =
        (options.mode.allows_focus() && input.focused.is_some_and(matches_target)).then(|| {
            TooltipRequest::new(anchor, content)
                .placement(options.placement)
                .invocation(TooltipInvocationKind::Focus)
        });
    resolve_tooltip_request(hover, focus, options.item_state, options.timing, now_ms)
}

fn tooltip_anchor(document: &UiDocument, target: UiNodeId) -> TooltipAnchor {
    TooltipAnchor::new(
        target,
        document
            .node_effective_transform(target)
            .transform_rect_bounds(document.node(target).layout.rect),
    )
}

pub fn tooltip_box(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    content: TooltipContent,
    options: TooltipBoxOptions,
) -> UiNodeId {
    let name = name.into();
    let text = content.text();
    let mut tooltip_node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style.clone(),
            clip: ClipBehavior::Clip,
            z_index: options.z_index,
            ..Default::default()
        },
    )
    .with_layer(options.layer)
    .with_clip_scope(options.clip_scope)
    .with_visual(options.visual)
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Tooltip)
            .label(options.accessibility_label.unwrap_or(content.title.clone()))
            .hint(text),
    );
    if let Some(animation) = options.animation {
        tooltip_node = tooltip_node.with_animation(animation);
    }
    let tooltip = document.add_portal_child(parent, options.portal.clone(), tooltip_node);

    label(
        document,
        tooltip,
        format!("{name}.title"),
        content.title,
        options.title_text_style,
        LayoutStyle::new().with_width_percent(1.0),
    );

    if let Some(body) = content.body {
        label(
            document,
            tooltip,
            format!("{name}.body"),
            body,
            options.body_text_style.clone(),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }

    if let Some(shortcut) = content.shortcut_label {
        label(
            document,
            tooltip,
            format!("{name}.shortcut"),
            shortcut,
            options.shortcut_text_style,
            LayoutStyle::new().with_width_percent(1.0),
        );
    }

    if let Some(reason) = content.disabled_reason {
        label(
            document,
            tooltip,
            format!("{name}.disabled_reason"),
            reason,
            options.body_text_style.clone(),
            LayoutStyle::new().with_width_percent(1.0),
        );
    }

    tooltip
}

/// Place help using an anchor, viewport, and cursor in logical window coordinates.
/// `tooltip_size` uses document UI units, like other authored widget dimensions.
#[allow(clippy::too_many_arguments)]
pub fn tooltip_box_from_request(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    request: &TooltipRequest,
    viewport: UiRect,
    tooltip_size: UiSize,
    cursor: Option<UiPoint>,
    options: TooltipBoxOptions,
) -> UiNodeId {
    let rect = tooltip_layout_rect(
        document.ui_scale(),
        request.anchor.rect,
        tooltip_size,
        viewport,
        request.placement,
        8.0,
        cursor,
    );
    let tooltip = tooltip_box(
        document,
        parent,
        name,
        request.content.clone(),
        options
            .at_rect(rect)
            .with_portal(UiPortalTarget::AppOverlay),
    );
    document.node_mut(tooltip).layout_constraint = Some(UiNodeLayoutConstraint::Tooltip {
        anchor: request.anchor.rect,
        viewport,
        size: tooltip_size,
        placement: request.placement,
        offset: 8.0,
        cursor,
    });
    tooltip
}

pub(crate) fn add_active_node_tooltip(
    document: &mut UiDocument,
    viewport: UiSize,
    cursor: Option<UiPoint>,
) -> Option<UiNodeId> {
    let focus = document.focus_state();
    let (target, tooltip) = focus
        .focused
        .and_then(|active| node_tooltip_for(document, active))
        .or_else(|| {
            focus
                .hovered
                .and_then(|active| node_tooltip_for(document, active))
        })?;
    let anchor = tooltip_anchor(document, target).rect;
    let bounds = UiRect::new(0.0, 0.0, viewport.width, viewport.height);
    let rect = tooltip_layout_rect(
        document.ui_scale(),
        anchor,
        tooltip.size,
        bounds,
        tooltip.placement,
        tooltip.offset,
        cursor,
    );
    let tooltip_name = format!("{}.tooltip", document.node(target).name());
    if document
        .nodes()
        .iter()
        .any(|node| node.name() == tooltip_name && node.logical_parent() == Some(target))
    {
        return None;
    }
    let help = tooltip_box(
        document,
        target,
        tooltip_name,
        tooltip.content,
        TooltipBoxOptions::default()
            .at_rect(rect)
            .with_portal(UiPortalTarget::AppOverlay),
    );
    document.node_mut(help).layout_constraint = Some(UiNodeLayoutConstraint::Tooltip {
        anchor,
        viewport: bounds,
        size: tooltip.size,
        placement: tooltip.placement,
        offset: tooltip.offset,
        cursor,
    });
    Some(help)
}

fn node_tooltip_for(
    document: &UiDocument,
    mut active: UiNodeId,
) -> Option<(UiNodeId, crate::core::document::UiNodeTooltip)> {
    loop {
        let node = document.nodes().get(active.0)?;
        if let Some(tooltip) = node.tooltip() {
            // Resolve the modal only when there is help to show. A candidate
            // above that boundary cannot provide help for the active modal.
            let modal_scope = document.accessibility_modal_scope();
            return document
                .node_in_modal_scope(active, modal_scope)
                .then(|| (active, tooltip.clone()));
        }
        active = node.logical_parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tooltips::TooltipVisibility;

    #[test]
    fn tooltip_anchors_and_boxes_match_painted_controls_at_every_scale() {
        let viewport = UiSize::new(1200.0, 900.0);
        let size = UiSize::new(96.0, 48.0);
        for ui_scale in [0.5, 1.0, 1.5, 2.0] {
            for dpi_scale in [1.0, 2.0] {
                for paint_scale in [0.5, 1.0, 2.0, -1.0] {
                    for placement in [TooltipPlacement::Right, TooltipPlacement::Below] {
                        let mut document = UiDocument::new(
                            LayoutStyle::new()
                                .with_width_percent(1.0)
                                .with_height_percent(1.0),
                        )
                        .with_scale(UiDocumentScale::new(ui_scale, dpi_scale));
                        let translation = if paint_scale < 0.0 {
                            UiPoint::new(600.0, 500.0)
                        } else {
                            UiPoint::new(80.0, 90.0)
                        };
                        let control = document.add_child(
                            document.root(),
                            UiNode::container(
                                "control",
                                LayoutStyle::absolute_rect(UiRect::new(60.0, 80.0, 80.0, 40.0)),
                            )
                            .with_input(InputBehavior::BUTTON)
                            .with_visual(UiVisual::panel(ColorRgba::WHITE, None, 0.0))
                            .with_animation(AnimationMachine::single_state(
                                "pose",
                                AnimatedValues::new(1.0, translation, paint_scale),
                            ))
                            .with_tooltip(TooltipContent::new("Help"))
                            .with_tooltip_size(size)
                            .with_tooltip_placement(placement),
                        );
                        document
                            .compute_layout(viewport, &mut ApproxTextMeasurer)
                            .unwrap();
                        let paint = document.paint_list();
                        let item = paint
                            .items
                            .iter()
                            .find(|item| item.node == control)
                            .unwrap();
                        let bounds =
                            crate::effective_geometry::EffectiveGeometry::from_paint_item(item, 0)
                                .transformed_bounds();
                        let input = document.handle_input(UiInputEvent::PointerMove(UiPoint::new(
                            bounds.x + bounds.width / 2.0,
                            bounds.y + bounds.height / 2.0,
                        )));
                        assert_eq!(input.hovered, Some(control));
                        let request = tooltip_trigger_resolution(
                            &document,
                            control,
                            TooltipContent::new("Help"),
                            &input,
                            0,
                            TooltipTriggerOptions::default()
                                .immediate()
                                .placement(placement),
                        )
                        .request
                        .unwrap();
                        assert_eq!(request.anchor.rect, bounds,
                            "anchor disagrees with paint: ui={ui_scale}, dpi={dpi_scale}, paint={paint_scale}");
                        let manual = tooltip_box_from_request(
                            &mut document,
                            control,
                            "manual.help",
                            &request,
                            UiRect::new(0.0, 0.0, viewport.width, viewport.height),
                            size,
                            None,
                            TooltipBoxOptions::default(),
                        );
                        let automatic =
                            add_active_node_tooltip(&mut document, viewport, None).unwrap();
                        document
                            .compute_layout(viewport, &mut ApproxTextMeasurer)
                            .unwrap();
                        let expected = match placement {
                            TooltipPlacement::Right => UiRect::new(
                                bounds.right() + 8.0 * ui_scale,
                                bounds.y,
                                size.width * ui_scale,
                                size.height * ui_scale,
                            ),
                            TooltipPlacement::Below => UiRect::new(
                                bounds.x,
                                bounds.bottom() + 8.0 * ui_scale,
                                size.width * ui_scale,
                                size.height * ui_scale,
                            ),
                            _ => unreachable!(),
                        };
                        for tooltip in [manual, automatic] {
                            assert_eq!(document.node(tooltip).layout().rect, expected,
                                "tooltip detached from paint: ui={ui_scale}, dpi={dpi_scale}, paint={paint_scale}, placement={placement:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn tooltip_placement_clamps_scaled_boxes_in_window_coordinates() {
        let viewport = UiRect::new(0.0, 0.0, 320.0, 240.0);
        let size = UiSize::new(96.0, 48.0);
        for ui_scale in [0.5, 1.0, 1.5, 2.0] {
            for placement in [
                TooltipPlacement::Above,
                TooltipPlacement::Below,
                TooltipPlacement::Left,
                TooltipPlacement::Right,
                TooltipPlacement::Cursor,
            ] {
                for apply_scale_after_build in [false, true] {
                    for origin in [
                        UiPoint::new(20.0, 20.0),
                        UiPoint::new(300.0, 20.0),
                        UiPoint::new(20.0, 190.0),
                        UiPoint::new(300.0, 190.0),
                    ] {
                        let mut document = UiDocument::new(
                            LayoutStyle::new()
                                .with_width_percent(1.0)
                                .with_height_percent(1.0),
                        )
                        .with_scale(UiDocumentScale::new(
                            if apply_scale_after_build {
                                1.0
                            } else {
                                ui_scale
                            },
                            1.0,
                        ));
                        let root = document.root();
                        let anchor = UiRect::new(origin.x, origin.y, 20.0, 20.0);
                        let cursor = UiPoint::new(origin.x + 10.0, origin.y + 10.0);
                        let request = TooltipRequest::new(
                            TooltipAnchor::new(root, anchor),
                            TooltipContent::new("Help"),
                        )
                        .placement(placement);
                        let tooltip = tooltip_box_from_request(
                            &mut document,
                            root,
                            "help",
                            &request,
                            viewport,
                            size,
                            Some(cursor),
                            TooltipBoxOptions::default(),
                        );
                        document.set_ui_scale(ui_scale);
                        document
                            .compute_layout(
                                UiSize::new(viewport.width, viewport.height),
                                &mut ApproxTextMeasurer,
                            )
                            .unwrap();
                        let rect = document.node(tooltip).layout().rect;
                        assert_eq!(
                            (rect.width, rect.height),
                            (size.width * ui_scale, size.height * ui_scale)
                        );
                        assert!(viewport.contains_rect(rect),
                        "tooltip escaped viewport: ui={ui_scale}, placement={placement:?}, anchor={anchor:?}, rect={rect:?}");
                        if placement == TooltipPlacement::Cursor {
                            assert_eq!(
                                (rect.x, rect.y),
                                (
                                    (cursor.x + 8.0 * ui_scale)
                                        .clamp(viewport.x, viewport.right() - rect.width),
                                    (cursor.y + 8.0 * ui_scale)
                                        .clamp(viewport.y, viewport.bottom() - rect.height)
                                )
                            );
                        } else {
                            assert!(!rect.intersects(anchor), "side tooltip covers its anchor: ui={ui_scale}, placement={placement:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn tooltips_follow_portal_ownership_and_stop_at_modal_boundaries() {
        let viewport = UiSize::new(400.0, 300.0);
        for portal in [
            UiPortalTarget::Parent,
            UiPortalTarget::AppOverlay,
            UiPortalTarget::named("host"),
            UiPortalTarget::GlobalAppOverlay,
            UiPortalTarget::global_named("host"),
        ] {
            for modal in [false, true] {
                for owner_help in [true, false] {
                    for focus in [false, true] {
                        let mut document = UiDocument::new(root_style(400.0, 300.0));
                        let root = document.root();
                        document
                            .node_mut(root)
                            .set_tooltip(TooltipContent::new("Root help"));
                        let mut metadata = AccessibilityMeta::new(AccessibilityRole::Group);
                        if modal {
                            metadata = metadata.modal();
                        }
                        let owner = document.add_child(
                            root,
                            UiNode::container(
                                "owner",
                                LayoutStyle::absolute_rect(UiRect::new(8.0, 8.0, 100.0, 80.0)),
                            )
                            .with_accessibility(metadata),
                        );
                        if owner_help {
                            document
                                .node_mut(owner)
                                .set_tooltip(TooltipContent::new("Owner help"));
                        }
                        let host = document.add_child(
                            root,
                            UiNode::container("host", LayoutStyle::size(400.0, 300.0))
                                .with_tooltip(TooltipContent::new("Host help")),
                        );
                        document.register_portal_host("host", host);
                        let popup = document.add_portal_child(
                            owner,
                            portal.clone(),
                            UiNode::container("popup", LayoutStyle::size(80.0, 32.0)),
                        );
                        let active = document.add_child(
                            popup,
                            UiNode::container("active", LayoutStyle::size(40.0, 24.0))
                                .with_input(InputBehavior::BUTTON),
                        );
                        document
                            .compute_layout(viewport, &mut ApproxTextMeasurer)
                            .unwrap();
                        let input = UiInputResult {
                            hovered: (!focus).then_some(active),
                            focused: focus.then_some(active),
                            ..Default::default()
                        };
                        let owned = matches!(
                            portal,
                            UiPortalTarget::Parent
                                | UiPortalTarget::AppOverlay
                                | UiPortalTarget::Named(_)
                        );
                        for target in [owner, root] {
                            let resolution = tooltip_trigger_resolution(
                                &document,
                                target,
                                TooltipContent::new("Explicit help"),
                                &input,
                                0,
                                TooltipTriggerOptions::default().immediate(),
                            );
                            let expected = if target == owner { owned } else { !modal };
                            assert_eq!(resolution.request.is_some(), expected,
                                "manual target={target:?}, portal={portal:?}, modal={modal}, focus={focus}");
                        }
                        document.set_focus_state(UiFocusState {
                            hovered: input.hovered,
                            focused: input.focused,
                            ..Default::default()
                        });
                        let expected = if modal && !owned {
                            None
                        } else if owned && owner_help {
                            Some(owner)
                        } else if owned && modal {
                            None
                        } else if matches!(portal, UiPortalTarget::GlobalNamed(_)) {
                            Some(host)
                        } else {
                            Some(root)
                        };
                        let tooltip = add_active_node_tooltip(&mut document, viewport, None);
                        assert_eq!(tooltip.is_some(), expected.is_some(),
                            "automatic portal={portal:?}, modal={modal}, owner_help={owner_help}, focus={focus}");
                        if let (Some(tooltip), Some(expected)) = (tooltip, expected) {
                            assert_eq!(
                                document.node(tooltip).logical_parent(),
                                Some(expected),
                                "tooltip lost its owner: portal={portal:?}, modal={modal}"
                            );
                            assert_eq!(
                                document
                                    .node(tooltip)
                                    .accessibility()
                                    .unwrap()
                                    .label
                                    .as_deref(),
                                Some(
                                    document
                                        .node(expected)
                                        .tooltip()
                                        .unwrap()
                                        .content
                                        .title
                                        .as_str()
                                )
                            );
                        }
                        if portal == UiPortalTarget::Parent && !modal && owner_help && !focus {
                            document.set_node_enabled(owner, false);
                            for allow_disabled_help in [true, false] {
                                let resolution = tooltip_trigger_resolution(
                                    &document,
                                    owner,
                                    TooltipContent::new("Disabled help"),
                                    &input,
                                    0,
                                    TooltipTriggerOptions::default().immediate().item_state(
                                        HelpItemState {
                                            allow_disabled_help,
                                            ..HelpItemState::disabled()
                                        },
                                    ),
                                );
                                assert_eq!(resolution.request.is_some(), allow_disabled_help);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn automatic_tooltip_names_are_scoped_to_their_owner() {
        let viewport = UiSize::new(400.0, 300.0);
        for foreign_owner in [true, false] {
            let mut document = UiDocument::new(root_style(400.0, 300.0));
            let first_panel = document.add_child(
                document.root(),
                UiNode::container("first", LayoutStyle::size(160.0, 80.0)),
            );
            let second_panel = document.add_child(
                document.root(),
                UiNode::container("second", LayoutStyle::size(160.0, 80.0)),
            );
            let first = document.add_child(
                first_panel,
                UiNode::container("control", LayoutStyle::size(80.0, 32.0)),
            );
            let active = document.add_child(
                second_panel,
                UiNode::container("control", LayoutStyle::size(80.0, 32.0))
                    .with_input(InputBehavior::BUTTON)
                    .with_tooltip(TooltipContent::new("Active help")),
            );
            tooltip_box(
                &mut document,
                if foreign_owner { first } else { active },
                "control.tooltip",
                TooltipContent::new("Authored help"),
                TooltipBoxOptions::default().with_portal(UiPortalTarget::AppOverlay),
            );
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .unwrap();
            document.set_focus_state(UiFocusState {
                focused: Some(active),
                ..Default::default()
            });
            let tooltip = add_active_node_tooltip(&mut document, viewport, None);
            assert_eq!(
                tooltip.is_some(),
                foreign_owner,
                "only a tooltip for the same owner should suppress automatic help"
            );
            if let Some(tooltip) = tooltip {
                assert_eq!(document.node(tooltip).logical_parent(), Some(active));
            }
            let count = document.node_count();
            assert!(add_active_node_tooltip(&mut document, viewport, None).is_none());
            assert_eq!(
                document.node_count(),
                count,
                "repeated resolution must not add a duplicate"
            );
        }
    }

    #[test]
    fn automatic_tooltips_prefer_focus_and_fall_back_to_available_hover_help() {
        for focus_help in [true, false] {
            for hover_help in [false, true] {
                let viewport = UiSize::new(400.0, 300.0);
                let mut document = UiDocument::new(root_style(400.0, 300.0));
                let root = document.root();
                let focused = document.add_child(
                    root,
                    UiNode::container("focused", LayoutStyle::size(80.0, 32.0))
                        .with_input(InputBehavior::BUTTON),
                );
                let hovered = document.add_child(
                    root,
                    UiNode::container("hovered", LayoutStyle::size(80.0, 32.0))
                        .with_input(InputBehavior::BUTTON),
                );
                if focus_help {
                    document
                        .node_mut(focused)
                        .set_tooltip(TooltipContent::new("Focus help"));
                }
                if hover_help {
                    document
                        .node_mut(hovered)
                        .set_tooltip(TooltipContent::new("Hover help"));
                }
                document
                    .compute_layout(viewport, &mut ApproxTextMeasurer)
                    .unwrap();
                document.set_focus_state(UiFocusState {
                    focused: Some(focused),
                    hovered: Some(hovered),
                    ..Default::default()
                });
                let tooltip = add_active_node_tooltip(&mut document, viewport, None);
                let expected = focus_help
                    .then_some(focused)
                    .or_else(|| hover_help.then_some(hovered));
                assert_eq!(
                    tooltip.is_some(),
                    expected.is_some(),
                    "focus_help={focus_help}, hover_help={hover_help}"
                );
                if let (Some(tooltip), Some(expected)) = (tooltip, expected) {
                    assert_eq!(
                        document
                            .node(tooltip)
                            .accessibility()
                            .unwrap()
                            .label
                            .as_deref(),
                        Some(
                            document
                                .node(expected)
                                .tooltip()
                                .unwrap()
                                .content
                                .title
                                .as_str()
                        )
                    );
                }
            }
        }
    }

    #[test]
    fn tooltip_rect_falls_back_before_clamping_to_viewport() {
        let anchor = UiRect::new(260.0, 120.0, 40.0, 20.0);
        let rect = tooltip_rect(
            anchor,
            UiSize::new(120.0, 60.0),
            UiRect::new(0.0, 0.0, 300.0, 180.0),
            TooltipPlacement::Right,
            8.0,
            None,
        );

        assert_eq!(rect.x, 132.0);
        assert_eq!(rect.y, 120.0);
        assert!(rect.right() <= anchor.x);
    }

    #[test]
    fn tooltip_rect_clamps_when_neither_side_has_room() {
        let anchor = UiRect::new(78.0, 44.0, 24.0, 20.0);
        let rect = tooltip_rect(
            anchor,
            UiSize::new(220.0, 60.0),
            UiRect::new(0.0, 0.0, 160.0, 120.0),
            TooltipPlacement::Right,
            8.0,
            None,
        );

        assert_eq!(rect.x, 0.0);
        assert_eq!(rect.y, 44.0);
    }

    #[test]
    fn tooltip_box_builds_accessible_overlay_content() {
        let mut document = UiDocument::new(root_style(300.0, 180.0));
        let root = document.root;
        let tooltip = tooltip_box(
            &mut document,
            root,
            "save.tooltip",
            TooltipContent::new("Save")
                .body("Write changes to disk")
                .shortcut_label("Ctrl+S"),
            TooltipBoxOptions::default().at_rect(UiRect::new(16.0, 24.0, 180.0, 72.0)),
        );

        let node = document.node(tooltip);
        assert_eq!(node.layer, Some(crate::platform::UiLayer::AppOverlay));
        assert_eq!(node.clip_scope, ClipScope::Viewport);
        assert_eq!(
            node.accessibility.as_ref().unwrap().role,
            AccessibilityRole::Tooltip
        );
        for expected in ["Save", "Write changes to disk", "Ctrl+S"] {
            assert!(document.nodes().iter().any(|child| {
                matches!(&child.content, UiContent::Text(text) if text.text == expected)
            }));
        }
        assert_eq!(
            node.animation.as_ref().unwrap().current_state_name(),
            "visible"
        );
    }

    #[test]
    fn tooltip_box_from_request_routes_through_overlay_portal() {
        let mut document = UiDocument::new(root_style(300.0, 180.0));
        let root = document.root;
        let request = TooltipRequest::new(
            TooltipAnchor::new(root, UiRect::new(24.0, 24.0, 40.0, 20.0)),
            TooltipContent::new("Save"),
        );
        let tooltip = tooltip_box_from_request(
            &mut document,
            root,
            "save.tooltip",
            &request,
            UiRect::new(0.0, 0.0, 300.0, 180.0),
            UiSize::new(120.0, 48.0),
            None,
            TooltipBoxOptions::default(),
        );

        let portal = document
            .portal_host(APP_OVERLAY_PORTAL)
            .expect("app overlay portal");
        assert_eq!(document.node(tooltip).parent, Some(portal));
    }

    #[test]
    fn tooltip_trigger_resolution_prefers_focus_and_respects_timing() {
        let mut document = UiDocument::new(root_style(300.0, 180.0));
        let root = document.root;
        let trigger = button(
            &mut document,
            root,
            "save",
            "Save",
            ButtonOptions::default(),
        );
        let input = UiInputResult {
            hovered: Some(trigger),
            focused: Some(trigger),
            ..Default::default()
        };

        let resolution = tooltip_trigger_resolution(
            &document,
            trigger,
            TooltipContent::new("Save").body("Write changes"),
            &input,
            100,
            TooltipTriggerOptions::default().placement(TooltipPlacement::Below),
        );

        assert_eq!(resolution.visibility, TooltipVisibility::Visible);
        let request = resolution.request.expect("request");
        assert_eq!(request.invocation, TooltipInvocationKind::Focus);
        assert_eq!(request.placement, TooltipPlacement::Below);
        assert_eq!(resolution.show_at_ms, Some(100));
    }

    #[test]
    fn tooltip_animation_policy_can_disable_motion() {
        let mut animation = tooltip_fade_slide_animation(false, true);
        assert_eq!(animation.current_state_name(), "hidden");
        assert!(animation.trigger(AnimationTrigger::Custom(TOOLTIP_SHOW_TRIGGER.to_owned())));
        animation.tick(0.0);
        assert_eq!(animation.current_state_name(), "visible");
        assert!(
            !animation.is_animating(),
            "reduced motion must finish immediately"
        );
        assert_eq!(animation.values().opacity, 1.0);
        assert_eq!(animation.values().translate, UiPoint::new(0.0, 0.0));
    }
}
