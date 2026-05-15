use super::*;

#[derive(Debug, Clone)]
pub struct TooltipBoxOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub title_text_style: TextStyle,
    pub body_text_style: TextStyle,
    pub shortcut_text_style: TextStyle,
    pub animation: Option<AnimationMachine>,
    pub z_index: i16,
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
            z_index: 100,
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
        if initially_visible {
            "visible"
        } else {
            "hidden"
        },
    )
    .expect("tooltip animation preset should be internally valid")
}

pub fn tooltip_trigger_resolution(
    document: &UiDocument,
    target: UiNodeId,
    content: TooltipContent,
    input: &UiInputResult,
    now_ms: u64,
    options: TooltipTriggerOptions,
) -> TooltipResolution {
    let anchor = TooltipAnchor::new(target, document.node(target).layout.rect);
    let hover = (options.mode.allows_hover()
        && input
            .hovered
            .is_some_and(|node| document.node_is_descendant_or_self(target, node)))
    .then(|| {
        TooltipRequest::new(anchor, content.clone())
            .placement(options.placement)
            .invocation(TooltipInvocationKind::Hover)
    });
    let focus = (options.mode.allows_focus()
        && input
            .focused
            .is_some_and(|node| document.node_is_descendant_or_self(target, node)))
    .then(|| {
        TooltipRequest::new(anchor, content)
            .placement(options.placement)
            .invocation(TooltipInvocationKind::Focus)
    });
    resolve_tooltip_request(hover, focus, options.item_state, options.timing, now_ms)
}

pub fn tooltip_rect(
    anchor: UiRect,
    tooltip_size: UiSize,
    viewport: UiRect,
    placement: TooltipPlacement,
    offset: f32,
    cursor: Option<UiPoint>,
) -> UiRect {
    let offset = finite_or(offset, 0.0).max(0.0);
    let tooltip_size = UiSize::new(
        finite_or(tooltip_size.width, 0.0).max(0.0),
        finite_or(tooltip_size.height, 0.0).max(0.0),
    );
    let origin = match placement {
        TooltipPlacement::Above => UiPoint::new(anchor.x, anchor.y - tooltip_size.height - offset),
        TooltipPlacement::Below => UiPoint::new(anchor.x, anchor.bottom() + offset),
        TooltipPlacement::Left => UiPoint::new(anchor.x - tooltip_size.width - offset, anchor.y),
        TooltipPlacement::Right => UiPoint::new(anchor.right() + offset, anchor.y),
        TooltipPlacement::Cursor => cursor
            .map(|point| UiPoint::new(point.x + offset, point.y + offset))
            .unwrap_or_else(|| UiPoint::new(anchor.right() + offset, anchor.bottom() + offset)),
    };
    UiRect::new(
        clamp_tooltip_axis(origin.x, tooltip_size.width, viewport.x, viewport.right()),
        clamp_tooltip_axis(origin.y, tooltip_size.height, viewport.y, viewport.bottom()),
        tooltip_size.width,
        tooltip_size.height,
    )
}

fn clamp_tooltip_axis(value: f32, extent: f32, min: f32, max: f32) -> f32 {
    let min = finite_or(min, 0.0);
    let max = finite_or(max, min).max(min);
    let extent = finite_or(extent, 0.0).max(0.0);
    let upper = (max - extent).max(min);
    finite_or(value, min).clamp(min, upper)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
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
    let rect = tooltip_rect(
        request.anchor.rect,
        tooltip_size,
        viewport,
        request.placement,
        8.0,
        cursor,
    );
    tooltip_box(
        document,
        parent,
        name,
        request.content.clone(),
        options
            .at_rect(rect)
            .with_portal(UiPortalTarget::AppOverlay),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_rect_clamps_absolute_overlay_to_viewport() {
        let anchor = UiRect::new(260.0, 120.0, 40.0, 20.0);
        let rect = tooltip_rect(
            anchor,
            UiSize::new(120.0, 60.0),
            UiRect::new(0.0, 0.0, 300.0, 180.0),
            TooltipPlacement::Right,
            8.0,
            None,
        );

        assert_eq!(rect.x, 180.0);
        assert_eq!(rect.y, 120.0);
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
        assert_eq!(node.style.z_index, 100);
        assert_eq!(node.layer, Some(crate::platform::UiLayer::AppOverlay));
        assert_eq!(node.clip_scope, ClipScope::Viewport);
        assert_eq!(
            node.accessibility.as_ref().unwrap().role,
            AccessibilityRole::Tooltip
        );
        assert_eq!(node.children.len(), 3);
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
    }
}
