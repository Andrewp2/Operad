//! Shared surface primitives used by extended widgets.

use crate::{
    AnimatedValues, AnimationMachine, AnimationState, AnimationTransition, AnimationTrigger,
    ColorRgba, UiPoint,
};

pub(crate) const DEFAULT_SURFACE_BG: ColorRgba = ColorRgba::new(24, 29, 36, 255);
pub(crate) const DEFAULT_SURFACE_STROKE: ColorRgba = ColorRgba::new(70, 82, 101, 255);
pub(crate) const DEFAULT_ACCENT: ColorRgba = ColorRgba::new(108, 180, 255, 255);

pub const SURFACE_OPEN_TRIGGER: &str = "surface.open";
pub const SURFACE_CLOSE_TRIGGER: &str = "surface.close";
pub const TOAST_ENTER_TRIGGER: &str = "toast.enter";
pub const TOAST_EXIT_TRIGGER: &str = "toast.exit";

pub fn surface_open_close_animation(initially_open: bool) -> AnimationMachine {
    let initial = if initially_open { "open" } else { "closed" };
    let fallback_values = if initially_open {
        AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0)
    } else {
        AnimatedValues::new(0.0, UiPoint::new(0.0, 12.0), 0.98)
    };
    AnimationMachine::new(
        vec![
            AnimationState::new(
                "closed",
                AnimatedValues::new(0.0, UiPoint::new(0.0, 12.0), 0.98),
            ),
            AnimationState::new(
                "open",
                AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0),
            ),
        ],
        vec![
            AnimationTransition::new(
                "closed",
                "open",
                surface_animation_trigger(SURFACE_OPEN_TRIGGER),
                0.16,
            ),
            AnimationTransition::new(
                "open",
                "closed",
                surface_animation_trigger(SURFACE_CLOSE_TRIGGER),
                0.12,
            ),
        ],
        initial,
    )
    .unwrap_or_else(|_| AnimationMachine::single_state(initial, fallback_values))
}

pub fn toast_enter_exit_animation(initially_visible: bool) -> AnimationMachine {
    let initial = if initially_visible {
        "visible"
    } else {
        "hidden"
    };
    let fallback_values = if initially_visible {
        AnimatedValues::new(1.0, UiPoint::new(0.0, 0.0), 1.0)
    } else {
        AnimatedValues::new(0.0, UiPoint::new(18.0, 0.0), 0.98)
    };
    AnimationMachine::new(
        vec![
            AnimationState::new(
                "hidden",
                AnimatedValues::new(0.0, UiPoint::new(18.0, 0.0), 0.98),
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
                surface_animation_trigger(TOAST_ENTER_TRIGGER),
                0.18,
            ),
            AnimationTransition::new(
                "visible",
                "hidden",
                surface_animation_trigger(TOAST_EXIT_TRIGGER),
                0.12,
            ),
        ],
        initial,
    )
    .unwrap_or_else(|_| AnimationMachine::single_state(initial, fallback_values))
}

fn surface_animation_trigger(name: &str) -> AnimationTrigger {
    AnimationTrigger::Custom(name.to_string())
}

#[cfg(test)]
mod tests {
    use taffy::prelude::{Dimension, Size as TaffySize, Style};

    use crate::accessibility::{AccessibilityAdapterRequest, AccessibilityCapabilities};
    use crate::drag_drop::DropTargetDescriptor;
    use crate::platform::{DragOperation, DragPayload};
    use crate::*;

    use super::*;
    use super::{surface_animation_trigger, SURFACE_CLOSE_TRIGGER, SURFACE_OPEN_TRIGGER};
    use crate::widgets::ext::{
        dialog::*, dock_workspace::*, popover::*, progress_indicator::*, split_pane::*,
        timeline_ruler::*, toast::*,
    };

    #[test]
    fn progress_indicator_values_accessibility_and_fill_geometry() {
        let value = ProgressIndicatorValue::percent(42.0);
        assert_eq!(value.normalized(), Some(0.42));
        assert_eq!(
            value.fill_rect(UiRect::new(10.0, 20.0, 200.0, 12.0)),
            UiRect::new(10.0, 20.0, 84.0, 12.0)
        );
        let accessibility =
            value.accessibility_meta("Recipe load", ProgressIndicatorKind::Progress, None);
        assert_eq!(accessibility.role, AccessibilityRole::ProgressBar);
        assert_eq!(accessibility.label.as_deref(), Some("Recipe load"));
        assert_eq!(accessibility.value.as_deref(), Some("42%"));
        assert_eq!(
            accessibility.value_range,
            Some(AccessibilityValueRange::new(0.0, 100.0))
        );

        let meter = ProgressIndicatorValue::new(18.5, 0.0, 100.0).accessibility_meta(
            "CPU",
            ProgressIndicatorKind::Meter,
            Some("%"),
        );
        assert_eq!(meter.role, AccessibilityRole::Meter);
        assert_eq!(meter.value.as_deref(), Some("18.5 %"));

        let indeterminate = ProgressIndicatorValue::indeterminate(0.0, 1.0).accessibility_meta(
            "Sync",
            ProgressIndicatorKind::Progress,
            None,
        );
        assert_eq!(indeterminate.value.as_deref(), Some("Indeterminate"));
        assert_eq!(
            indeterminate.hint.as_deref(),
            Some("Value is not currently available")
        );
    }

    #[test]
    fn progress_indicator_builds_accessible_fill_node() {
        let mut doc = UiDocument::new(root_style(240.0, 40.0));
        let root = doc.root;
        let nodes = progress_indicator(
            &mut doc,
            root,
            "cpu",
            ProgressIndicatorValue::new(18.5, 0.0, 100.0),
            ProgressIndicatorOptions {
                kind: ProgressIndicatorKind::Meter,
                accessibility_label: Some("CPU load".to_string()),
                accessibility_unit: Some("%".to_string()),
                fill_shader: Some(ShaderEffect::new("meter.fill")),
                ..Default::default()
            },
        );
        doc.compute_layout(UiSize::new(240.0, 40.0), &mut ApproxTextMeasurer)
            .expect("layout");

        let root_accessibility = doc.node(nodes.root).accessibility.as_ref().unwrap();
        assert_eq!(root_accessibility.role, AccessibilityRole::Meter);
        assert_eq!(root_accessibility.label.as_deref(), Some("CPU load"));
        assert_eq!(root_accessibility.value.as_deref(), Some("18.5 %"));
        assert_eq!(
            doc.node(nodes.fill)
                .shader
                .as_ref()
                .map(|shader| shader.key.as_str()),
            Some("meter.fill")
        );
        assert!(doc.node(nodes.fill).layout.rect.width > 40.0);
        assert!(doc.node(nodes.fill).layout.rect.width < 50.0);
    }

    #[test]
    fn split_pane_state_clamps_resizes_and_builds_nodes() {
        let mut state = SplitPaneState::new(0.25).with_min_sizes(120.0, 80.0);
        let sizes = state.resolved_sizes(300.0, 10.0);
        assert_eq!(sizes.handle, 10.0);
        assert_eq!(sizes.first, 120.0);
        assert_eq!(sizes.second, 170.0);

        assert!(state.resize_by(80.0, 300.0, 10.0));
        assert!(state.fraction > 0.6 && state.fraction < 0.7);

        let mut doc = UiDocument::new(root_style(400.0, 200.0));
        let root = doc.root;
        let nodes = split_pane(
            &mut doc,
            root,
            "workspace",
            SplitAxis::Horizontal,
            state,
            SplitPaneOptions::default(),
            |document, parent| {
                document.add_child(
                    parent,
                    UiNode::text(
                        "left.label",
                        "Left",
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
            },
            |document, parent| {
                document.add_child(
                    parent,
                    UiNode::text(
                        "right.label",
                        "Right",
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
            },
        );
        doc.compute_layout(UiSize::new(400.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert!(doc.node(nodes.handle).input.focusable);
        assert!(doc.node(nodes.first).layout.rect.width >= state.min_first);
        assert_eq!(doc.node(nodes.root).children.len(), 3);

        let accessibility = doc.accessibility_tree();
        let splitter = accessibility
            .iter()
            .find(|node| node.id == nodes.handle)
            .expect("splitter accessibility");
        assert_eq!(splitter.role, AccessibilityRole::Slider);
        assert_eq!(splitter.label.as_deref(), Some("workspace splitter"));
        assert_eq!(splitter.value.as_deref(), Some("69%"));
        assert!(splitter.focusable);
    }

    #[test]
    fn dock_workspace_builds_visible_panels_and_center() {
        let panels = vec![
            DockPanelDescriptor::new("top", "Toolbar", DockSide::Top, 40.0),
            DockPanelDescriptor::new("left", "Browser", DockSide::Left, 120.0)
                .resizable(true)
                .title_image(ImageContent::new("icons.browser"))
                .shader(ShaderEffect::new("dock.panel.blur").uniform("radius", 12.0))
                .accessibility_hint("Project browser panel"),
            DockPanelDescriptor::center("editor", "Editor"),
            DockPanelDescriptor::new("right", "Inspector", DockSide::Right, 90.0).visible(false),
        ];
        let mut doc = UiDocument::new(root_style(500.0, 320.0));
        let root = doc.root;
        let nodes = dock_workspace(
            &mut doc,
            root,
            "dock",
            &panels,
            DockWorkspaceOptions::default(),
            |document, parent, panel| {
                document.add_child(
                    parent,
                    UiNode::text(
                        format!("{}.body", panel.id),
                        panel.id.clone(),
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
            },
        );
        doc.compute_layout(UiSize::new(500.0, 320.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert!(nodes.center.is_some());
        assert_eq!(nodes.panels.len(), 3);
        assert!(nodes
            .panels
            .iter()
            .any(|panel| panel.id == "left" && panel.resize_handle.is_some()));
        let left = nodes
            .panels
            .iter()
            .find(|panel| panel.id == "left")
            .expect("left panel");
        assert_eq!(doc.node(left.root).layout.rect.width, 120.0);
        assert_eq!(
            doc.node(left.root)
                .shader
                .as_ref()
                .map(|shader| shader.key.as_str()),
            Some("dock.panel.blur")
        );
        let title = left.title.expect("left title");
        assert!(doc.node(title).children.iter().any(|child| {
            matches!(
                doc.node(*child).content,
                UiContent::Image(ref image) if image.key == "icons.browser"
            )
        }));

        let accessibility = doc.accessibility_tree();
        let panel_accessibility = accessibility
            .iter()
            .find(|node| node.id == left.root)
            .expect("left panel accessibility");
        assert_eq!(panel_accessibility.role, AccessibilityRole::TabPanel);
        assert_eq!(panel_accessibility.label.as_deref(), Some("Browser"));
        assert_eq!(
            panel_accessibility.hint.as_deref(),
            Some("Project browser panel")
        );
        let resize_handle = left.resize_handle.expect("left resize handle");
        let resize_accessibility = accessibility
            .iter()
            .find(|node| node.id == resize_handle)
            .expect("resize accessibility");
        assert_eq!(resize_accessibility.role, AccessibilityRole::Slider);
        assert!(resize_accessibility.focusable);
    }

    #[test]
    fn dock_workspace_layout_snapshot_restores_panel_size_side_and_visibility() {
        let original = vec![
            DockPanelDescriptor::new("assets", "Assets", DockSide::Left, 180.0)
                .with_min_size(120.0),
            DockPanelDescriptor::new("inspector", "Inspector", DockSide::Right, 240.0),
            DockPanelDescriptor::center("canvas", "Canvas"),
        ];
        let mut snapshot = DockWorkspaceLayoutSnapshot::from_descriptors(&original);
        snapshot.panels[0].side = DockSide::Bottom;
        snapshot.panels[0].size = 40.0;
        snapshot.panels[1].visible = false;
        snapshot.panels.push(DockPanelLayoutSnapshot {
            id: "missing".to_owned(),
            side: DockSide::Left,
            size: 100.0,
            visible: true,
        });

        let mut restored = original.clone();
        let report = snapshot.apply_to(&mut restored);

        assert_eq!(report.updated, vec!["assets", "inspector", "canvas"]);
        assert_eq!(report.missing, vec!["missing"]);
        assert_eq!(restored[0].side, DockSide::Bottom);
        assert_eq!(restored[0].size, 120.0);
        assert!(!restored[1].visible);
        assert_eq!(snapshot.panel("canvas").unwrap().side, DockSide::Center);
    }

    #[test]
    fn dock_workspace_drag_sources_and_drop_zones_cover_dock_float_targets() {
        let panel = DockPanelDescriptor::new("assets", "Assets", DockSide::Left, 180.0);
        let source = panel
            .drag_source(UiRect::new(10.0, 20.0, 120.0, 24.0))
            .expect("panel drag source");

        assert_eq!(source.kind, DragDropSurfaceKind::DockPanel);
        assert_eq!(dock_panel_id_from_payload(&source.payload), Some("assets"));
        assert_eq!(source.allowed_operations, vec![DragOperation::Move]);
        assert!(source.can_start());
        assert_eq!(
            source.accessibility_meta().label.as_deref(),
            Some("Assets panel")
        );

        let zones = dock_workspace_drop_zones(
            "dock",
            UiRect::new(0.0, 0.0, 400.0, 300.0),
            DockWorkspaceDragOptions::default().edge_thickness(40.0),
        );
        assert_eq!(zones.len(), 6);
        assert!(zones
            .iter()
            .any(|zone| zone.placement == DockDropPlacement::Floating));
        assert!(zones
            .iter()
            .any(|zone| zone.placement == DockDropPlacement::Dock(DockSide::Center)));

        let targets: Vec<_> = zones.iter().map(|zone| zone.target.clone()).collect();
        let top_hit = DropTargetDescriptor::hit_test(
            &targets,
            UiPoint::new(200.0, 12.0),
            &source.payload,
            &source.allowed_operations,
        )
        .expect("top hit");
        assert_eq!(top_hit.target_id.as_str(), "dock.drop.top");
        assert_eq!(top_hit.kind, DragDropSurfaceKind::DockTarget);

        let center_hit = DropTargetDescriptor::hit_test(
            &targets,
            UiPoint::new(200.0, 150.0),
            &source.payload,
            &source.allowed_operations,
        )
        .expect("center hit");
        assert_eq!(center_hit.target_id.as_str(), "dock.drop.center");

        let platform_payload = DragPayload::text("not a dock panel");
        assert_eq!(dock_panel_id_from_payload(&platform_payload), None);
        let bridged_hit = DropTargetDescriptor::hit_test(
            &targets,
            UiPoint::new(12.0, 12.0),
            &platform_payload,
            &source.allowed_operations,
        );
        assert!(
            bridged_hit.is_some(),
            "dock zones accept text payloads so hosts can bridge platform drags"
        );
    }

    #[test]
    fn dock_workspace_state_applies_dock_and_float_drops_to_descriptors() {
        let mut panels = vec![
            DockPanelDescriptor::new("assets", "Assets", DockSide::Left, 180.0),
            DockPanelDescriptor::new("inspect", "Inspector", DockSide::Right, 220.0),
        ];
        let mut state = DockWorkspaceState::new();
        let payload = dock_panel_drag_payload("inspect");
        let floating_rect = UiRect::new(40.0, 48.0, 220.0, 180.0);

        let change = state
            .apply_drop_to_panels(
                &mut panels,
                &payload,
                DockDropPlacement::Floating,
                floating_rect,
            )
            .expect("float update");
        assert_eq!(change.panel_id, "inspect");
        assert_eq!(change.previous, DockPanelPlacement::Docked(DockSide::Right));
        assert_eq!(change.next, DockPanelPlacement::Floating(floating_rect));
        assert!(state.is_floating("inspect"));
        assert!(!panels[1].visible);

        let change = state
            .apply_drop_to_panels(
                &mut panels,
                &payload,
                DockDropPlacement::Dock(DockSide::Left),
                floating_rect,
            )
            .expect("dock update");
        assert_eq!(change.previous, DockPanelPlacement::Floating(floating_rect));
        assert_eq!(change.next, DockPanelPlacement::Docked(DockSide::Left));
        assert!(!state.is_floating("inspect"));
        assert_eq!(panels[1].side, DockSide::Left);
        assert!(panels[1].visible);
    }

    #[test]
    fn dock_workspace_state_tracks_drawer_visibility_and_reorder() {
        let mut panels = vec![
            DockPanelDescriptor::new("inspector", "Inspector", DockSide::Left, 120.0),
            DockPanelDescriptor::new("assets", "Assets", DockSide::Left, 104.0),
            DockPanelDescriptor::center("document", "Document"),
        ];
        let mut state = DockWorkspaceState::new();

        let visibility = state.hide_panel("assets");
        assert_eq!(visibility.previous_visible, true);
        assert_eq!(visibility.next_visible, false);
        state.apply_visibility_to_panels(&mut panels);
        assert!(!panels[1].visible);
        assert!(state.is_hidden("assets"));

        let payload = dock_panel_drag_payload("assets");
        let reorder = state
            .apply_reorder_to_panels(
                &mut panels,
                &payload,
                "inspector",
                DockPanelReorderPlacement::Before,
            )
            .expect("reorder hidden drawer panel");

        assert_eq!(reorder.panel_id, "assets");
        assert_eq!(reorder.target_panel_id, "inspector");
        assert_eq!(reorder.previous_side, DockSide::Left);
        assert_eq!(reorder.next_side, DockSide::Left);
        assert_eq!(panels[0].id, "assets");
        assert!(panels[0].visible);
        assert!(!state.is_hidden("assets"));
        let order: Vec<_> = state.panel_order().iter().map(String::as_str).collect();
        assert_eq!(order, vec!["assets", "inspector", "document"]);
        assert_eq!(state.focused_panel.as_deref(), Some("assets"));
    }

    #[test]
    fn dock_workspace_reorder_targets_and_drawer_rail_are_public_widgets() {
        let panels = vec![
            DockPanelDescriptor::new("inspector", "Inspector", DockSide::Left, 120.0),
            DockPanelDescriptor::new("assets", "Assets", DockSide::Left, 104.0),
            DockPanelDescriptor::center("document", "Document"),
        ];
        let targets = dock_panel_reorder_drop_targets(
            "dock",
            &panels,
            DockSide::Left,
            UiRect::new(10.0, 20.0, 100.0, 240.0),
            DockWorkspaceReorderOptions::default().target_thickness(20.0),
        );
        assert_eq!(targets.len(), 4);
        assert_eq!(
            targets[0].target.id.as_str(),
            "dock.reorder.left.inspector.before"
        );
        assert_eq!(
            targets[1].target.id.as_str(),
            "dock.reorder.left.inspector.after"
        );
        assert_eq!(
            targets[0].target.bounds,
            UiRect::new(10.0, 20.0, 100.0, 20.0)
        );
        assert_eq!(
            targets[1].target.bounds,
            UiRect::new(10.0, 120.0, 100.0, 20.0)
        );

        let mut doc = UiDocument::new(root_style(320.0, 160.0));
        let drawers = [
            DockDrawerDescriptor::for_panel(&panels[0], true).with_action("dock.drawer.inspector"),
            DockDrawerDescriptor::for_panel(&panels[1], false).with_action("dock.drawer.assets"),
        ];
        let root = doc.root;
        let nodes = dock_drawer_rail(
            &mut doc,
            root,
            "dock.drawers",
            &drawers,
            DockDrawerRailOptions::default(),
        );
        doc.compute_layout(UiSize::new(320.0, 160.0), &mut ApproxTextMeasurer)
            .expect("drawer rail layout");

        assert_eq!(nodes.items.len(), 2);
        assert_eq!(
            doc.node(nodes.items[0].root)
                .action
                .as_ref()
                .and_then(WidgetActionBinding::action_id)
                .map(WidgetActionId::as_str),
            Some("dock.drawer.inspector")
        );
        let accessibility = doc.accessibility_tree();
        let drawer_accessibility = accessibility
            .iter()
            .find(|node| node.id == nodes.items[0].root)
            .expect("drawer accessibility");
        assert_eq!(drawer_accessibility.role, AccessibilityRole::Button);
        assert_eq!(drawer_accessibility.label.as_deref(), Some("Inspector"));
    }

    #[test]
    fn dock_workspace_commits_host_drop_hits_to_dock_and_reorder_state() {
        let mut panels = vec![
            DockPanelDescriptor::new("inspector", "Inspector", DockSide::Left, 120.0),
            DockPanelDescriptor::new("assets", "Assets", DockSide::Right, 104.0),
            DockPanelDescriptor::center("document", "Document"),
        ];
        let mut state = DockWorkspaceState::new();
        let source = panels[0]
            .drag_source(UiRect::new(0.0, 0.0, 120.0, 28.0))
            .expect("drag source");
        assert!(source.start_request(UiPoint::new(12.0, 12.0)).is_some());

        let zones = dock_workspace_drop_zones(
            "dock",
            UiRect::new(0.0, 0.0, 400.0, 240.0),
            DockWorkspaceDragOptions::default().allowed_sides([DockSide::Right]),
        );
        let zone_targets = zones
            .iter()
            .map(|zone| zone.target.clone())
            .collect::<Vec<_>>();
        let dock_hit = DropTargetDescriptor::hit_test(
            &zone_targets,
            UiPoint::new(390.0, 100.0),
            &source.payload,
            &source.allowed_operations,
        )
        .expect("right dock hit");
        let dock_change = state
            .apply_drop_hit_to_panels(
                &mut panels,
                &source.payload,
                &dock_hit,
                UiRect::new(40.0, 48.0, 220.0, 180.0),
            )
            .expect("dock commit");

        assert_eq!(
            dock_change.next,
            DockPanelPlacement::Docked(DockSide::Right)
        );
        assert_eq!(panels[0].side, DockSide::Right);

        let reorder_targets = dock_panel_reorder_drop_targets(
            "dock",
            &panels,
            DockSide::Right,
            UiRect::new(0.0, 0.0, 180.0, 220.0),
            DockWorkspaceReorderOptions::default().target_thickness(24.0),
        );
        let target_descriptors = reorder_targets
            .iter()
            .map(|target| target.target.clone())
            .collect::<Vec<_>>();
        let reorder_hit = DropTargetDescriptor::hit_test(
            &target_descriptors,
            UiPoint::new(12.0, 206.0),
            &source.payload,
            &source.allowed_operations,
        )
        .expect("reorder hit");
        let reorder_change = state
            .apply_reorder_hit_to_panels(&mut panels, &source.payload, &reorder_hit)
            .expect("reorder commit");

        assert_eq!(reorder_change.panel_id, "inspector");
        assert_eq!(reorder_change.next_side, DockSide::Right);
        assert_eq!(panels[1].id, "inspector");
        let order: Vec<_> = state.panel_order().iter().map(String::as_str).collect();
        assert_eq!(order, vec!["assets", "inspector", "document"]);
    }

    #[test]
    fn dialog_and_popover_state_track_dismissal_rules() {
        let mut dialogs = DialogStack::default();
        let settings = DialogDescriptor::new("settings", "Settings")
            .modal(true)
            .accessibility_hint("Configure workspace settings");
        let settings_accessibility = settings.accessibility();
        assert_eq!(settings_accessibility.role, AccessibilityRole::Dialog);
        assert_eq!(settings_accessibility.label.as_deref(), Some("Settings"));
        assert_eq!(
            settings_accessibility.hint.as_deref(),
            Some("Configure workspace settings")
        );
        assert!(settings_accessibility.focusable);

        dialogs.open(settings);
        dialogs.open(DialogDescriptor::new("confirm", "Confirm").dismissal(DialogDismissal::NONE));
        assert!(dialogs.traps_focus());
        assert_eq!(dialogs.top().unwrap().id, "confirm");
        assert!(dialogs
            .dismiss_top(DialogDismissReason::EscapeKey)
            .is_none());
        assert!(dialogs.close("confirm").is_some());
        assert_eq!(
            dialogs
                .dismiss_top(DialogDismissReason::EscapeKey)
                .unwrap()
                .id,
            "settings"
        );

        let mut popovers = PopoverState::default();
        let popover = PopoverDescriptor::new(
            "tools",
            PopoverAnchor::Rect(UiRect::new(90.0, 90.0, 20.0, 20.0)),
            PopoverPlacement::Bottom,
        )
        .accessibility_label("Tool menu")
        .close_on_escape(false);
        let popover_accessibility = popover.accessibility();
        assert_eq!(popover_accessibility.role, AccessibilityRole::Menu);
        assert_eq!(popover_accessibility.label.as_deref(), Some("Tool menu"));
        popovers.toggle(popover.clone());
        assert!(popovers.is_open("tools"));
        popovers.toggle(popover);
        assert!(!popovers.is_open("tools"));

        popovers.open(
            PopoverDescriptor::new(
                "sticky",
                PopoverAnchor::Point(UiPoint::new(2.0, 3.0)),
                PopoverPlacement::Right,
            )
            .close_on_escape(false),
        );
        assert!(popovers.dismiss(PopoverDismissReason::EscapeKey).is_none());
        assert_eq!(
            popovers
                .dismiss(PopoverDismissReason::OutsidePointer)
                .unwrap()
                .id,
            "sticky"
        );

        let rect = resolve_popover_rect(
            UiRect::new(180.0, 180.0, 20.0, 20.0),
            UiSize::new(80.0, 50.0),
            UiRect::new(0.0, 0.0, 220.0, 220.0),
            PopoverPlacement::Bottom,
            6.0,
        );
        assert_eq!(rect.x, 140.0);
        assert_eq!(rect.y, 170.0);

        let guarded = resolve_popover_rect(
            UiRect::new(f32::NAN, 0.0, 10.0, 10.0),
            UiSize::new(f32::NAN, 24.0),
            UiRect::new(0.0, 0.0, 100.0, 100.0),
            PopoverPlacement::Right,
            f32::NAN,
        );
        assert!(guarded.x.is_finite());
        assert_eq!(guarded.width, 0.0);
    }

    #[test]
    fn overlay_frame_opens_dialog_with_focus_trap_request() {
        let focus_trap =
            FocusTrap::new(UiNodeId(9)).restore_focus(FocusRestoreTarget::Node(UiNodeId(2)));

        let output = process_overlay_frame(
            OverlayFrameRequest::new(OverlayFrameState::new())
                .accessibility_capabilities(AccessibilityCapabilities::FULL)
                .event(OverlayFrameEvent::open_dialog_with_focus_trap(
                    DialogDescriptor::new("settings", "Settings").modal(true),
                    focus_trap,
                )),
        );

        assert!(output.changed);
        assert!(output.state.dialogs.is_open("settings"));
        assert_eq!(output.state.focus_trap, Some(focus_trap));
        assert!(output.state.traps_focus());
        assert_eq!(
            output.accessibility_requests,
            vec![AccessibilityAdapterRequest::SetFocusTrap(focus_trap)]
        );
    }

    #[test]
    fn overlay_frame_dismisses_dialog_and_popover_by_policy() {
        let mut state = OverlayFrameState::new();
        state
            .dialogs
            .open(DialogDescriptor::new("confirm", "Confirm"));
        state.popover.open(PopoverDescriptor::new(
            "tools",
            PopoverAnchor::Point(UiPoint::new(2.0, 3.0)),
            PopoverPlacement::Right,
        ));

        let output = process_overlay_frame(OverlayFrameRequest::new(state).events([
            OverlayFrameEvent::dismiss_dialog(DialogDismissReason::EscapeKey),
            OverlayFrameEvent::dismiss_popover(PopoverDismissReason::OutsidePointer),
        ]));

        assert!(output.changed);
        assert_eq!(output.dismissed_dialogs.len(), 1);
        assert_eq!(output.dismissed_dialogs[0].id, "confirm");
        assert_eq!(
            output
                .dismissed_popover
                .as_ref()
                .map(|popover| popover.id.as_str()),
            Some("tools")
        );
        assert!(!output.state.has_overlay());
    }

    #[test]
    fn overlay_frame_respects_focus_trap_capabilities_and_clear_restore() {
        let focus_trap = FocusTrap::new(UiNodeId(7));
        let unsupported = process_overlay_frame(
            OverlayFrameRequest::new(OverlayFrameState::new())
                .event(OverlayFrameEvent::set_focus_trap(focus_trap)),
        );
        assert_eq!(unsupported.state.focus_trap, Some(focus_trap));
        assert!(unsupported.accessibility_requests.is_empty());

        let output = process_overlay_frame(
            OverlayFrameRequest::new(unsupported.state)
                .accessibility_capabilities(AccessibilityCapabilities::FULL)
                .event(OverlayFrameEvent::clear_focus_trap(
                    FocusRestoreTarget::Previous,
                )),
        );

        assert!(output.changed);
        assert_eq!(output.state.focus_trap, None);
        assert_eq!(
            output.accessibility_requests,
            vec![AccessibilityAdapterRequest::ClearFocusTrap {
                restore: FocusRestoreTarget::Previous
            }]
        );
    }

    #[test]
    fn overlay_frame_toggle_popover_reports_dismissed_popover() {
        let popover = PopoverDescriptor::new(
            "toolbox",
            PopoverAnchor::Rect(UiRect::new(10.0, 10.0, 20.0, 20.0)),
            PopoverPlacement::Bottom,
        );
        let output = process_overlay_frame(
            OverlayFrameRequest::new(OverlayFrameState::new())
                .event(OverlayFrameEvent::toggle_popover(popover.clone())),
        );
        assert!(output.state.popover.is_open("toolbox"));
        assert!(output.dismissed_popover.is_none());

        let output = process_overlay_frame(
            OverlayFrameRequest::new(output.state)
                .event(OverlayFrameEvent::toggle_popover(popover)),
        );
        assert!(!output.state.popover.is_open("toolbox"));
        assert_eq!(
            output
                .dismissed_popover
                .as_ref()
                .map(|popover| popover.id.as_str()),
            Some("toolbox")
        );
    }

    #[test]
    fn toast_stack_expires_limits_and_builds_action_nodes() {
        let mut stack = ToastStack::new(2);
        stack.push(ToastSeverity::Info, "One", None, Some(1.0));
        stack.push(ToastSeverity::Success, "Two", None, None);
        let action_toast = Toast::new(
            ToastId::new(99),
            ToastSeverity::Warning,
            "Three",
            Some("Body".to_string()),
            None,
        )
        .with_action(ToastAction::new("retry", "Retry"))
        .with_icon(ImageContent::new("icons.warning"))
        .with_shader(ShaderEffect::new("toast.warning.glow").uniform("strength", 0.8))
        .accessibility_hint("Requires attention");
        stack.push_toast(action_toast);

        assert_eq!(
            stack
                .visible()
                .iter()
                .map(|toast| toast.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Two", "Three"]
        );
        stack.tick(f32::NAN);
        assert_eq!(
            stack
                .toasts
                .iter()
                .find(|toast| toast.title == "One")
                .unwrap()
                .remaining_seconds(),
            Some(1.0)
        );
        stack.tick(1.1);
        assert!(!stack.toasts.iter().any(|toast| toast.title == "One"));

        let mut doc = UiDocument::new(root_style(400.0, 240.0));
        let root = doc.root;
        let stack_node = toast_stack(
            &mut doc,
            root,
            "toasts",
            &stack,
            ToastStackOptions::default(),
        );
        doc.compute_layout(UiSize::new(400.0, 240.0), &mut ApproxTextMeasurer)
            .expect("layout");
        assert_eq!(doc.node(stack_node).children.len(), 2);
        assert!(doc.nodes().iter().any(|node| node.input.focusable));
        let warning_node = doc
            .node(stack_node)
            .children
            .last()
            .copied()
            .expect("warning toast");
        assert_eq!(
            doc.node(warning_node)
                .shader
                .as_ref()
                .map(|shader| shader.key.as_str()),
            Some("toast.warning.glow")
        );
        assert_eq!(
            doc.node(warning_node)
                .animation
                .as_ref()
                .map(AnimationMachine::current_state_name),
            Some("visible")
        );
        assert!(doc.nodes().iter().any(|node| {
            matches!(
                node.content,
                UiContent::Image(ref image) if image.key == "icons.warning"
            )
        }));

        let accessibility = doc.accessibility_tree();
        assert!(accessibility.iter().any(|node| {
            node.id == stack_node
                && node.role == AccessibilityRole::List
                && node.label.as_deref() == Some("Notifications")
        }));
        assert!(accessibility.iter().any(|node| {
            node.role == AccessibilityRole::ListItem
                && node.label.as_deref() == Some("Warning: Three. Body")
                && node.hint.as_deref() == Some("Requires attention")
        }));
        assert!(accessibility.iter().any(|node| {
            node.role == AccessibilityRole::Button && node.label.as_deref() == Some("Retry")
        }));
        let retry_action = doc
            .nodes()
            .iter()
            .find(|node| node.name == "toasts.toast.99.action.retry")
            .and_then(|node| node.action.as_ref());
        assert_eq!(
            retry_action,
            Some(&WidgetActionBinding::action("toast.action.99.retry"))
        );
    }

    #[test]
    fn timeline_range_and_ruler_ticks_are_renderer_neutral() {
        let range = TimelineRange::new(10.0, 14.0);
        assert_eq!(range.value_to_x(12.0, 400.0), 200.0);
        assert_eq!(range.x_to_value(100.0, 400.0), 11.0);
        let reversed = TimelineRange::new(14.0, 10.0);
        assert_eq!(reversed.ordered(), range);
        assert!(reversed.contains(12.0));
        assert_eq!(reversed.x_to_value(100.0, 400.0), 11.0);

        let spec = RulerSpec {
            range,
            width: 400.0,
            major_step: 1.0,
            minor_step: 0.25,
            label_every: 2,
        };
        let ticks = spec.ticks();
        assert_eq!(ticks.first().unwrap().value, 10.0);
        assert!(ticks
            .iter()
            .any(|tick| tick.kind == RulerTickKind::Minor && tick.label.is_none()));
        assert_eq!(
            ticks
                .iter()
                .filter_map(|tick| tick.label.as_deref())
                .collect::<Vec<_>>(),
            vec!["10", "12", "14"]
        );
        assert!(RulerSpec {
            range,
            width: f32::NAN,
            major_step: 1.0,
            minor_step: 0.25,
            label_every: 1,
        }
        .ticks()
        .is_empty());

        let mut doc = UiDocument::new(root_style(400.0, 80.0));
        let root = doc.root;
        let options = TimelineRulerOptions {
            shader: Some(ShaderEffect::new("timeline.scanline")),
            accessibility_label: Some("Transport timeline".to_string()),
            ..TimelineRulerOptions::default()
        };
        let ruler = timeline_ruler(&mut doc, root, "ruler", spec, options);
        doc.compute_layout(UiSize::new(400.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");
        assert_eq!(
            doc.node(ruler)
                .shader
                .as_ref()
                .map(|shader| shader.key.as_str()),
            Some("timeline.scanline")
        );
        let has_scene = doc.node(ruler).children.iter().any(|child| {
            matches!(
                doc.node(*child).content,
                UiContent::Scene(ref primitives) if !primitives.is_empty()
            )
        });
        let has_label_text = doc.node(ruler).children.iter().any(|child| {
            matches!(
                doc.node(*child).content,
                UiContent::Text(TextContent { .. })
            )
        });
        assert!(has_scene);
        assert!(has_label_text);

        let ruler_accessibility = doc
            .accessibility_tree()
            .into_iter()
            .find(|node| node.id == ruler)
            .expect("ruler accessibility");
        assert_eq!(ruler_accessibility.role, AccessibilityRole::Slider);
        assert_eq!(
            ruler_accessibility.label.as_deref(),
            Some("Transport timeline")
        );
        assert_eq!(ruler_accessibility.value.as_deref(), Some("10 to 14"));
    }

    #[test]
    fn surface_animation_presets_transition_between_states() {
        let mut open_close = surface_open_close_animation(false);
        assert_eq!(open_close.current_state_name(), "closed");
        assert!(open_close.trigger(surface_animation_trigger(SURFACE_OPEN_TRIGGER)));
        open_close.tick(0.16);
        assert_eq!(open_close.current_state_name(), "open");
        assert!(open_close.trigger(surface_animation_trigger(SURFACE_CLOSE_TRIGGER)));
        open_close.tick(0.12);
        assert_eq!(open_close.current_state_name(), "closed");

        let mut toast = toast_enter_exit_animation(false);
        assert_eq!(toast.current_state_name(), "hidden");
        assert!(toast.trigger(surface_animation_trigger(TOAST_ENTER_TRIGGER)));
        toast.tick(0.18);
        assert_eq!(toast.current_state_name(), "visible");
        assert!(toast.trigger(surface_animation_trigger(TOAST_EXIT_TRIGGER)));
        toast.tick(0.12);
        assert_eq!(toast.current_state_name(), "hidden");
    }
}
