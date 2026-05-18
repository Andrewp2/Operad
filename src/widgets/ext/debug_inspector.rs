//! Debug inspector widgets.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, FlexWrap, JustifyContent, LengthPercentage,
    LengthPercentageAuto, Position, Rect as TaffyRect, Size as TaffySize, Style,
};

use crate::debug::{
    DebugAccessibilityOverlayNode, DebugAnimationGraphEdgeKind, DebugAnimationInspectorNode,
    DebugInspectorSnapshot, DebugLayoutInspectorNode,
};
use crate::{
    AccessibilityMeta, AccessibilityRole, AnimationInputValue, ColorRgba, InputBehavior,
    LayoutStyle, StrokeStyle, TextStyle, TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle,
    UiSize, UiVisual, WidgetActionMode,
};

use super::data::PropertyValueKind;
use super::property_inspector::{
    property_inspector_grid, PropertyGridRow, PropertyInspectorOptions,
};

#[derive(Debug, Clone)]
pub struct DebugInspectorPanelOptions {
    pub layout: LayoutStyle,
    pub selected_node: Option<String>,
    pub label_width: f32,
    pub row_height: f32,
    pub max_layout_rows: usize,
    pub max_animation_rows: usize,
    pub show_animation: bool,
    pub action_prefix: Option<String>,
    pub title_style: TextStyle,
}

impl Default for DebugInspectorPanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                gap: taffy::geometry::Size {
                    width: LengthPercentage::length(8.0),
                    height: LengthPercentage::length(8.0),
                },
                ..Default::default()
            }),
            selected_node: None,
            label_width: 128.0,
            row_height: 26.0,
            max_layout_rows: usize::MAX,
            max_animation_rows: 8,
            show_animation: true,
            action_prefix: None,
            title_style: TextStyle {
                font_size: 14.0,
                line_height: 20.0,
                color: ColorRgba::new(238, 243, 248, 255),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebugInspectorPanelNodes {
    pub root: UiNodeId,
    pub layout_grid: UiNodeId,
    pub animation_grid: UiNodeId,
}

#[derive(Debug, Clone)]
pub struct AnimationStateGraphPanelOptions {
    pub layout: LayoutStyle,
    pub state_width: f32,
    pub state_height: f32,
    pub edge_row_height: f32,
    pub max_edges: usize,
    pub action_prefix: Option<String>,
    pub state_visual: UiVisual,
    pub current_state_visual: UiVisual,
    pub target_state_visual: UiVisual,
    pub edge_visual: UiVisual,
    pub active_edge_visual: UiVisual,
    pub text_style: TextStyle,
    pub muted_text_style: TextStyle,
}

impl Default for AnimationStateGraphPanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                gap: taffy::geometry::Size {
                    width: LengthPercentage::length(6.0),
                    height: LengthPercentage::length(6.0),
                },
                ..Default::default()
            }),
            state_width: 92.0,
            state_height: 34.0,
            edge_row_height: 24.0,
            max_edges: 6,
            action_prefix: None,
            state_visual: UiVisual::panel(
                ColorRgba::new(25, 31, 40, 255),
                Some(StrokeStyle::new(ColorRgba::new(69, 83, 104, 255), 1.0)),
                4.0,
            ),
            current_state_visual: UiVisual::panel(
                ColorRgba::new(50, 87, 130, 255),
                Some(StrokeStyle::new(ColorRgba::new(116, 183, 255, 255), 1.0)),
                4.0,
            ),
            target_state_visual: UiVisual::panel(
                ColorRgba::new(65, 54, 110, 255),
                Some(StrokeStyle::new(ColorRgba::new(160, 130, 230, 255), 1.0)),
                4.0,
            ),
            edge_visual: UiVisual::panel(ColorRgba::new(18, 23, 31, 255), None, 3.0),
            active_edge_visual: UiVisual::panel(
                ColorRgba::new(54, 75, 46, 255),
                Some(StrokeStyle::new(ColorRgba::new(138, 214, 104, 230), 1.0)),
                3.0,
            ),
            text_style: TextStyle {
                font_size: 13.0,
                line_height: 18.0,
                wrap: TextWrap::None,
                color: ColorRgba::new(238, 243, 248, 255),
                ..Default::default()
            },
            muted_text_style: TextStyle {
                font_size: 12.0,
                line_height: 16.0,
                wrap: TextWrap::WordOrGlyph,
                color: ColorRgba::new(171, 183, 201, 255),
                ..Default::default()
            },
        }
    }
}

impl AnimationStateGraphPanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationStateGraphPanelNodes {
    pub root: UiNodeId,
    pub states: Vec<UiNodeId>,
    pub edges: Vec<UiNodeId>,
}

#[derive(Debug, Clone)]
pub struct AnimationInspectorControlsOptions {
    pub layout: LayoutStyle,
    pub row_height: f32,
    pub button_width: f32,
    pub scrub_width: f32,
    pub max_inputs: usize,
    pub paused: bool,
    pub scrub_progress: Option<f32>,
    pub action_prefix: Option<String>,
    pub button_visual: UiVisual,
    pub active_button_visual: UiVisual,
    pub track_visual: UiVisual,
    pub track_fill_visual: UiVisual,
    pub text_style: TextStyle,
    pub muted_text_style: TextStyle,
}

impl Default for AnimationInspectorControlsOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                gap: taffy::geometry::Size {
                    width: LengthPercentage::length(0.0),
                    height: LengthPercentage::length(6.0),
                },
                ..Default::default()
            }),
            row_height: 28.0,
            button_width: 86.0,
            scrub_width: 132.0,
            max_inputs: 8,
            paused: false,
            scrub_progress: None,
            action_prefix: None,
            button_visual: UiVisual::panel(
                ColorRgba::new(34, 45, 60, 255),
                Some(StrokeStyle::new(ColorRgba::new(76, 92, 116, 255), 1.0)),
                4.0,
            ),
            active_button_visual: UiVisual::panel(
                ColorRgba::new(48, 92, 145, 255),
                Some(StrokeStyle::new(ColorRgba::new(116, 183, 255, 255), 1.0)),
                4.0,
            ),
            track_visual: UiVisual::panel(ColorRgba::new(35, 43, 54, 255), None, 4.0),
            track_fill_visual: UiVisual::panel(ColorRgba::new(105, 177, 255, 255), None, 4.0),
            text_style: TextStyle {
                font_size: 12.0,
                line_height: 16.0,
                wrap: TextWrap::None,
                color: ColorRgba::new(238, 243, 248, 255),
                ..Default::default()
            },
            muted_text_style: TextStyle {
                font_size: 12.0,
                line_height: 16.0,
                wrap: TextWrap::WordOrGlyph,
                color: ColorRgba::new(171, 183, 201, 255),
                ..Default::default()
            },
        }
    }
}

impl AnimationInspectorControlsOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }

    pub const fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    pub const fn scrub_progress(mut self, progress: f32) -> Self {
        self.scrub_progress = Some(progress);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationInspectorControlsNodes {
    pub root: UiNodeId,
    pub transport: Vec<UiNodeId>,
    pub inputs: Vec<UiNodeId>,
}

#[derive(Debug, Clone)]
pub struct AccessibilityOverlayPanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_rows: usize,
    pub action_prefix: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AccessibilityDebugOverlayOptions {
    pub layout: LayoutStyle,
    pub outline_visual: UiVisual,
    pub warning_visual: UiVisual,
    pub label_style: TextStyle,
    pub show_labels: bool,
    pub action_prefix: Option<String>,
    pub z_index: i16,
}

impl Default for AccessibilityDebugOverlayOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                position: Position::Absolute,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            }),
            outline_visual: UiVisual::panel(
                ColorRgba::TRANSPARENT,
                Some(StrokeStyle::new(ColorRgba::new(116, 183, 255, 210), 1.0)),
                2.0,
            ),
            warning_visual: UiVisual::panel(
                ColorRgba::TRANSPARENT,
                Some(StrokeStyle::new(ColorRgba::new(255, 203, 96, 230), 1.5)),
                2.0,
            ),
            label_style: TextStyle {
                font_size: 11.0,
                line_height: 14.0,
                color: ColorRgba::new(238, 243, 248, 255),
                ..Default::default()
            },
            show_labels: true,
            action_prefix: None,
            z_index: 900,
        }
    }
}

impl AccessibilityDebugOverlayOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityDebugOverlayNodes {
    pub root: UiNodeId,
    pub outlines: Vec<UiNodeId>,
    pub labels: Vec<UiNodeId>,
}

impl Default for AccessibilityOverlayPanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 180.0,
            row_height: 26.0,
            max_rows: 24,
            action_prefix: None,
        }
    }
}

impl DebugInspectorPanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

pub fn debug_inspector_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    snapshot: &DebugInspectorSnapshot,
    options: DebugInspectorPanelOptions,
) -> DebugInspectorPanelNodes {
    let name = name.into();
    let selected = selected_debug_node(snapshot, options.selected_node.as_deref());
    let animation = selected.and_then(|node| {
        snapshot
            .animations
            .iter()
            .find(|animation| animation.id == node.id)
    });

    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                ..Default::default()
            },
        ),
    );

    document.add_child(
        root,
        UiNode::text(
            format!("{name}.layout.title"),
            selected
                .map(|node| format!("Layout: {}", node.name))
                .unwrap_or_else(|| "Layout: no selected node".to_string()),
            options.title_style.clone(),
            LayoutStyle::new().with_width_percent(1.0).with_height(24.0),
        ),
    );
    let layout_grid = property_inspector_grid(
        document,
        root,
        format!("{name}.layout"),
        &layout_rows(selected, options.max_layout_rows),
        inspector_grid_options(
            options.label_width,
            options.row_height,
            options
                .action_prefix
                .as_deref()
                .map(|prefix| format!("{prefix}.layout")),
            "Layout inspector",
        ),
    );

    let animation_grid = if options.show_animation {
        document.add_child(
            root,
            UiNode::text(
                format!("{name}.animation.title"),
                animation
                    .map(|animation| format!("Animation: {}", animation.name))
                    .unwrap_or_else(|| "Animation: none".to_string()),
                TextStyle {
                    color: ColorRgba::new(171, 183, 201, 255),
                    ..options.title_style
                },
                LayoutStyle::new().with_width_percent(1.0).with_height(24.0),
            ),
        );
        property_inspector_grid(
            document,
            root,
            format!("{name}.animation"),
            &animation_rows(animation, options.max_animation_rows),
            inspector_grid_options(
                options.label_width,
                options.row_height,
                options
                    .action_prefix
                    .as_deref()
                    .map(|prefix| format!("{prefix}.animation")),
                "Animation inspector",
            ),
        )
    } else {
        layout_grid
    };

    DebugInspectorPanelNodes {
        root,
        layout_grid,
        animation_grid,
    }
}

pub fn animation_state_graph_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    animation: Option<&DebugAnimationInspectorNode>,
    options: AnimationStateGraphPanelOptions,
) -> AnimationStateGraphPanelNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(format!("{name} graph")),
        ),
    );

    let Some(animation) = animation else {
        document.add_child(
            root,
            UiNode::text(
                format!("{name}.empty"),
                "No animation state machine",
                options.muted_text_style,
                LayoutStyle::new().with_width_percent(1.0).with_height(24.0),
            ),
        );
        return AnimationStateGraphPanelNodes {
            root,
            states: Vec::new(),
            edges: Vec::new(),
        };
    };

    let graph = animation.state_graph();
    let state_row = document.add_child(
        root,
        UiNode::container(
            format!("{name}.states"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    gap: taffy::geometry::Size {
                        width: LengthPercentage::length(6.0),
                        height: LengthPercentage::length(6.0),
                    },
                    ..Default::default()
                })
                .style,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::List).label("Animation states"),
        ),
    );

    let mut states = Vec::with_capacity(graph.states.len());
    for state in &graph.states {
        let visual = if state.current {
            options.current_state_visual
        } else if state.target {
            options.target_state_visual
        } else {
            options.state_visual
        };
        let state_name = animation_graph_action_id_part(&state.name);
        let mut state_node = UiNode::container(
            format!("{name}.state.{state_name}"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    justify_content: Some(JustifyContent::Center),
                    size: TaffySize {
                        width: Dimension::length(options.state_width.max(1.0)),
                        height: Dimension::length(options.state_height.max(1.0)),
                    },
                    padding: TaffyRect::length(4.0),
                    flex_shrink: 0.0,
                    ..Default::default()
                })
                .style,
                ..Default::default()
            },
        )
        .with_visual(visual)
        .with_input(InputBehavior::BUTTON)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::ListItem)
                .label(state.name.clone())
                .value(animation_graph_state_value(state.current, state.target))
                .focusable(),
        );
        if let Some(prefix) = options.action_prefix.as_deref() {
            state_node = state_node.with_action(format!("{prefix}.state.{state_name}"));
        }
        let state_node = document.add_child(state_row, state_node);
        document.add_child(
            state_node,
            UiNode::text(
                format!("{name}.state.{state_name}.label"),
                state.name.clone(),
                options.text_style.clone(),
                LayoutStyle::new().with_width_percent(1.0).with_height(20.0),
            ),
        );
        states.push(state_node);
    }

    let edge_list = document.add_child(
        root,
        UiNode::container(
            format!("{name}.edges"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    gap: taffy::geometry::Size {
                        width: LengthPercentage::length(0.0),
                        height: LengthPercentage::length(4.0),
                    },
                    ..Default::default()
                })
                .style,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::List).label("Animation transitions"),
        ),
    );

    let mut edges = Vec::new();
    for (index, edge) in graph.edges.iter().take(options.max_edges).enumerate() {
        let edge_value = animation_graph_edge_value(edge.kind, edge.active, edge.progress);
        let edge_node = document.add_child(
            edge_list,
            UiNode::container(
                format!("{name}.edge.{index}"),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: Dimension::length(options.edge_row_height.max(1.0)),
                        },
                        padding: TaffyRect {
                            left: LengthPercentage::length(6.0),
                            right: LengthPercentage::length(6.0),
                            top: LengthPercentage::length(0.0),
                            bottom: LengthPercentage::length(0.0),
                        },
                        gap: taffy::geometry::Size {
                            width: LengthPercentage::length(6.0),
                            height: LengthPercentage::length(0.0),
                        },
                        ..Default::default()
                    })
                    .style,
                    ..Default::default()
                },
            )
            .with_visual(if edge.active {
                options.active_edge_visual
            } else {
                options.edge_visual
            })
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::ListItem)
                    .label(format!("{} to {}", edge.from, edge.to))
                    .value(edge_value.clone()),
            ),
        );
        document.add_child(
            edge_node,
            UiNode::text(
                format!("{name}.edge.{index}.route"),
                format!("{} -> {}", edge.from, edge.to),
                options.text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::length(128.0),
                        height: Dimension::auto(),
                    },
                    flex_shrink: 0.0,
                    ..Default::default()
                }),
            ),
        );
        document.add_child(
            edge_node,
            UiNode::text(
                format!("{name}.edge.{index}.label"),
                if edge.label.is_empty() {
                    edge_value
                } else {
                    edge.label.clone()
                },
                options.muted_text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    ..Default::default()
                }),
            ),
        );
        edges.push(edge_node);
    }

    AnimationStateGraphPanelNodes {
        root,
        states,
        edges,
    }
}

pub fn animation_inspector_controls_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    animation: Option<&DebugAnimationInspectorNode>,
    options: AnimationInspectorControlsOptions,
) -> AnimationInspectorControlsNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group)
                .label(format!("{name} animation controls")),
        ),
    );

    let Some(animation) = animation else {
        document.add_child(
            root,
            UiNode::text(
                format!("{name}.empty"),
                "No animation controls",
                options.muted_text_style,
                LayoutStyle::new().with_width_percent(1.0).with_height(24.0),
            ),
        );
        return AnimationInspectorControlsNodes {
            root,
            transport: Vec::new(),
            inputs: Vec::new(),
        };
    };

    let transport_row = debug_control_row(document, root, format!("{name}.transport"), &options);
    let mut transport = Vec::new();
    transport.push(animation_control_button(
        document,
        transport_row,
        format!("{name}.transport.pause_toggle"),
        if options.paused { "Resume" } else { "Pause" },
        options
            .action_prefix
            .as_deref()
            .map(|prefix| format!("{prefix}.transport.pause_toggle")),
        options.paused,
        &options,
    ));
    transport.push(animation_control_button(
        document,
        transport_row,
        format!("{name}.transport.step"),
        "Step",
        options
            .action_prefix
            .as_deref()
            .map(|prefix| format!("{prefix}.transport.step")),
        false,
        &options,
    ));
    transport.push(animation_scrub_control(
        document,
        transport_row,
        format!("{name}.transport.scrub"),
        options
            .scrub_progress
            .or_else(|| {
                animation
                    .active_transition
                    .as_ref()
                    .map(|active| active.progress)
            })
            .unwrap_or(0.0),
        options
            .action_prefix
            .as_deref()
            .map(|prefix| format!("{prefix}.transport.scrub")),
        &options,
    ));

    let inputs_root = document.add_child(
        root,
        UiNode::container(
            format!("{name}.inputs"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    gap: taffy::geometry::Size {
                        width: LengthPercentage::length(0.0),
                        height: LengthPercentage::length(4.0),
                    },
                    ..Default::default()
                })
                .style,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::List).label("Animation inputs"),
        ),
    );

    let mut inputs = Vec::new();
    for (input_name, value) in animation.inputs.iter().take(options.max_inputs) {
        let id = animation_graph_action_id_part(input_name);
        let action_base = options
            .action_prefix
            .as_deref()
            .map(|prefix| format!("{prefix}.input.{id}"));
        let node = match value {
            AnimationInputValue::Bool(active) => animation_control_button(
                document,
                inputs_root,
                format!("{name}.input.{id}.toggle"),
                format!("{input_name}: {active}"),
                action_base.map(|base| format!("{base}.toggle")),
                *active,
                &options,
            ),
            AnimationInputValue::Number(number) => animation_number_input_control(
                document,
                inputs_root,
                format!("{name}.input.{id}.set"),
                input_name,
                *number,
                action_base.map(|base| format!("{base}.set")),
                &options,
            ),
            AnimationInputValue::Trigger(fired) => animation_control_button(
                document,
                inputs_root,
                format!("{name}.input.{id}.fire"),
                if *fired {
                    format!("{input_name}: fired")
                } else {
                    format!("Fire {input_name}")
                },
                action_base.map(|base| format!("{base}.fire")),
                *fired,
                &options,
            ),
        };
        inputs.push(node);
    }

    AnimationInspectorControlsNodes {
        root,
        transport,
        inputs,
    }
}

pub fn accessibility_overlay_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    snapshot: &DebugInspectorSnapshot,
    options: AccessibilityOverlayPanelOptions,
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                ..Default::default()
            },
        ),
    );

    property_inspector_grid(
        document,
        root,
        format!("{name}.rows"),
        &accessibility_overlay_rows(snapshot, options.max_rows),
        PropertyInspectorOptions {
            label_width: options.label_width,
            row_height: options.row_height,
            action_prefix: options.action_prefix,
            accessibility_label: Some("Accessibility overlay".to_owned()),
            ..Default::default()
        },
    );

    root
}

pub fn accessibility_debug_overlay(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    snapshot: &DebugInspectorSnapshot,
    options: AccessibilityDebugOverlayOptions,
) -> AccessibilityDebugOverlayNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                z_index: options.z_index,
                ..Default::default()
            },
        ),
    );

    let mut outlines = Vec::new();
    let mut labels = Vec::new();
    for node in &snapshot.accessibility_overlay {
        let outline_name = format!("{name}.outline.{}", node.id.0);
        let mut outline = UiNode::container(
            outline_name.clone(),
            UiNodeStyle {
                layout: absolute_rect_style(node.rect).style,
                z_index: options.z_index,
                ..Default::default()
            },
        )
        .with_visual(if node.warnings.is_empty() {
            options.outline_visual
        } else {
            options.warning_visual
        });
        if let Some(prefix) = &options.action_prefix {
            outline = outline.with_action(format!("{prefix}.node.{}", node.id.0));
        }
        let outline = document.add_child(root, outline);
        outlines.push(outline);

        if options.show_labels {
            let label = document.add_child(
                outline,
                UiNode::text(
                    format!("{outline_name}.label"),
                    accessibility_overlay_label(node),
                    options.label_style.clone(),
                    LayoutStyle::new()
                        .with_width_percent(1.0)
                        .with_height(16.0)
                        .padding(2.0),
                ),
            );
            labels.push(label);
        }
    }

    AccessibilityDebugOverlayNodes {
        root,
        outlines,
        labels,
    }
}

fn inspector_grid_options(
    label_width: f32,
    row_height: f32,
    action_prefix: Option<String>,
    accessibility_label: impl Into<String>,
) -> PropertyInspectorOptions {
    PropertyInspectorOptions {
        label_width,
        row_height,
        action_prefix,
        accessibility_label: Some(accessibility_label.into()),
        ..Default::default()
    }
}

fn debug_control_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    _options: &AnimationInspectorControlsOptions,
) -> UiNodeId {
    document.add_child(
        parent,
        UiNode::container(
            name,
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    gap: taffy::geometry::Size {
                        width: LengthPercentage::length(6.0),
                        height: LengthPercentage::length(6.0),
                    },
                    ..Default::default()
                })
                .style,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Toolbar).label("Animation transport"),
        ),
    )
}

fn animation_control_button(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    action: Option<String>,
    active: bool,
    options: &AnimationInspectorControlsOptions,
) -> UiNodeId {
    let label = label.into();
    let mut node = UiNode::container(
        name.into(),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                size: TaffySize {
                    width: Dimension::length(options.button_width.max(1.0)),
                    height: Dimension::length(options.row_height.max(1.0)),
                },
                padding: TaffyRect::length(4.0),
                flex_shrink: 0.0,
                ..Default::default()
            })
            .style,
            ..Default::default()
        },
    )
    .with_visual(if active {
        options.active_button_visual
    } else {
        options.button_visual
    })
    .with_input(InputBehavior::BUTTON)
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Button)
            .label(label.clone())
            .focusable(),
    );
    if let Some(action) = action {
        node = node.with_action(action);
    }
    let button = document.add_child(parent, node);
    document.add_child(
        button,
        UiNode::text(
            format!("{}.label", document.node(button).name),
            label,
            options.text_style.clone(),
            LayoutStyle::new().with_width_percent(1.0),
        ),
    );
    button
}

fn animation_scrub_control(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    progress: f32,
    action: Option<String>,
    options: &AnimationInspectorControlsOptions,
) -> UiNodeId {
    let name = name.into();
    let progress = progress.clamp(0.0, 1.0);
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                size: TaffySize {
                    width: Dimension::length(options.scrub_width.max(1.0)),
                    height: Dimension::length(options.row_height.max(1.0)),
                },
                padding: TaffyRect::length(4.0),
                flex_shrink: 0.0,
                ..Default::default()
            })
            .style,
            ..Default::default()
        },
    )
    .with_visual(options.track_visual)
    .with_input(InputBehavior::BUTTON)
    .with_action_mode(WidgetActionMode::PointerEdit)
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Slider)
            .label("Scrub animation")
            .value(format!("{:.0}%", progress * 100.0))
            .focusable(),
    );
    if let Some(action) = action {
        node = node.with_action(action);
    }
    let scrub = document.add_child(parent, node);
    document.add_child(
        scrub,
        UiNode::container(
            format!("{name}.fill"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::length((options.scrub_width - 8.0).max(1.0) * progress),
                        height: Dimension::percent(1.0),
                    },
                    flex_shrink: 0.0,
                    ..Default::default()
                })
                .style,
                ..Default::default()
            },
        )
        .with_visual(options.track_fill_visual),
    );
    scrub
}

fn animation_number_input_control(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    input_name: &str,
    value: f32,
    action: Option<String>,
    options: &AnimationInspectorControlsOptions,
) -> UiNodeId {
    let name = name.into();
    let row = debug_control_row(document, parent, name.clone(), options);
    document.add_child(
        row,
        UiNode::text(
            format!("{name}.label"),
            format!("{input_name}: {value:.2}"),
            options.text_style.clone(),
            LayoutStyle::new()
                .with_width(options.button_width.max(1.0))
                .with_height(options.row_height.max(1.0))
                .with_flex_shrink(0.0),
        ),
    );
    animation_scrub_control(
        document,
        row,
        format!("{name}.slider"),
        value.clamp(0.0, 1.0),
        action,
        options,
    );
    row
}

fn selected_debug_node<'a>(
    snapshot: &'a DebugInspectorSnapshot,
    selected_name: Option<&str>,
) -> Option<&'a DebugLayoutInspectorNode> {
    selected_name
        .and_then(|name| snapshot.node(name))
        .or_else(|| snapshot.nodes.first())
}

fn layout_rows(node: Option<&DebugLayoutInspectorNode>, max_rows: usize) -> Vec<PropertyGridRow> {
    let Some(node) = node else {
        return vec![row("empty", "Node", "No nodes in snapshot")];
    };
    let mut rows = vec![
        row("id", "ID", format!("#{}", node.id.0)),
        row("content", "Content", format!("{:?}", node.content_kind)),
        row("rect", "Rect", format_rect(node.rect)),
        row("clip", "Clip", format_rect(node.clip_rect)),
        row("final", "Final size", format_size(node.final_size)),
        row("children", "Children", node.children.len().to_string()),
        row("paint", "Paint items", node.paint.count.to_string()),
        row(
            "input",
            "Input",
            format!(
                "pointer={} focus={} keyboard={}",
                node.input.pointer, node.input.focusable, node.input.keyboard
            ),
        ),
        row("display", "Display", node.style.display.clone()),
        row("direction", "Direction", node.style.flex_direction.clone()),
        row("size", "Style size", node.style.size.clone()),
        row("min_size", "Min size", node.style.min_size.clone()),
        row("margin", "Margin", node.style.margin.clone()),
        row("padding", "Padding", node.style.padding.clone()),
        row("gap", "Gap", node.style.gap.clone()),
    ];
    if let Some(intrinsic) = node.intrinsic_size {
        rows.push(row(
            "intrinsic.min",
            "Intrinsic min",
            format_size(intrinsic.min),
        ));
        rows.push(row(
            "intrinsic.preferred",
            "Intrinsic pref",
            format_size(intrinsic.preferred),
        ));
    }
    if let Some(scroll) = node.scroll {
        rows.push(row(
            "scroll.offset",
            "Scroll offset",
            format_point(scroll.offset),
        ));
        rows.push(row(
            "scroll.viewport",
            "Scroll viewport",
            format_size(scroll.viewport_size),
        ));
        rows.push(row(
            "scroll.content",
            "Scroll content",
            format_size(scroll.content_size),
        ));
    }
    if let Some(scrollbar) = node.scrollbar {
        rows.push(row(
            "scrollbar.axis",
            "Scrollbar axis",
            format!("{:?}", scrollbar.axis),
        ));
        rows.push(row(
            "scrollbar.range",
            "Scrollbar range",
            format!(
                "viewport {:.1}, content {:.1}, max {:.1}",
                scrollbar.viewport(),
                scrollbar.content(),
                scrollbar.max_offset()
            ),
        ));
    }
    if let Some(accessibility) = &node.accessibility {
        rows.push(row(
            "accessibility",
            "Accessibility",
            format!(
                "{:?} {}",
                accessibility.role,
                accessibility.label.as_deref().unwrap_or("")
            ),
        ));
    }
    if !node.audit_warnings.is_empty() {
        rows.push(row(
            "audits",
            "Audit warnings",
            node.audit_warnings.len().to_string(),
        ));
    }
    rows.truncate(max_rows);
    rows
}

fn animation_rows(
    animation: Option<&DebugAnimationInspectorNode>,
    max_rows: usize,
) -> Vec<PropertyGridRow> {
    let Some(animation) = animation else {
        return vec![row("empty", "Animation", "No animation on selected node")];
    };
    let mut rows = vec![
        row("state", "Current state", animation.current_state.clone()),
        row(
            "opacity",
            "Opacity",
            format!("{:.3}", animation.values.opacity),
        ),
        row(
            "translate",
            "Translate",
            format_point(animation.values.translate),
        ),
        row("scale", "Scale", format!("{:.3}", animation.values.scale)),
        row("morph", "Morph", format!("{:.3}", animation.values.morph)),
        row("states", "States", animation.states.len().to_string()),
        row(
            "transitions",
            "Transitions",
            animation.transitions.len().to_string(),
        ),
    ];
    if let Some(active) = &animation.active_transition {
        rows.push(row("active", "Active", active.to_state_name.clone()));
        rows.push(row(
            "progress",
            "Progress",
            format!("{:.1}%", active.progress * 100.0),
        ));
    }
    for (index, (name, value)) in animation.inputs.iter().take(max_rows).enumerate() {
        rows.push(
            PropertyGridRow::new(
                format!("input.{index}"),
                format!("Input {name}"),
                format_animation_input_value(*value),
            )
            .with_kind(animation_input_value_kind(*value)),
        );
    }
    rows
}

fn format_animation_input_value(value: AnimationInputValue) -> String {
    match value {
        AnimationInputValue::Bool(value) => value.to_string(),
        AnimationInputValue::Number(value) => format!("{value:.3}"),
        AnimationInputValue::Trigger(fired) => {
            if fired {
                "fired".to_owned()
            } else {
                "ready".to_owned()
            }
        }
    }
}

fn animation_input_value_kind(value: AnimationInputValue) -> PropertyValueKind {
    match value {
        AnimationInputValue::Bool(_) | AnimationInputValue::Trigger(_) => {
            PropertyValueKind::Boolean
        }
        AnimationInputValue::Number(_) => PropertyValueKind::Number,
    }
}

fn accessibility_overlay_rows(
    snapshot: &DebugInspectorSnapshot,
    max_rows: usize,
) -> Vec<PropertyGridRow> {
    let rows = snapshot
        .accessibility_overlay
        .iter()
        .take(max_rows)
        .map(accessibility_overlay_row)
        .collect::<Vec<_>>();
    if rows.is_empty() {
        vec![row("empty", "Accessibility", "No accessibility nodes")]
    } else {
        rows
    }
}

fn accessibility_overlay_row(node: &DebugAccessibilityOverlayNode) -> PropertyGridRow {
    let value = if let Some(accessibility) = &node.accessibility {
        let mut parts = vec![format!("{:?}", accessibility.role)];
        if let Some(label) = &accessibility.label {
            parts.push(format!("label={label}"));
        }
        if let Some(value) = &accessibility.value {
            parts.push(format!("value={value}"));
        }
        if let Some(index) = node.focus_index {
            parts.push(format!("tab={}", index + 1));
        }
        if !node.warnings.is_empty() {
            parts.push(format!("{} warnings", node.warnings.len()));
        }
        parts.join("; ")
    } else {
        format!("{} warnings", node.warnings.len())
    };

    let mut row = row(format!("node.{}", node.id.0), node.name.clone(), value);
    if !node.warnings.is_empty() {
        row = row.warning(format!("{} audit warnings", node.warnings.len()));
    }
    row
}

fn accessibility_overlay_label(node: &DebugAccessibilityOverlayNode) -> String {
    if let Some(accessibility) = &node.accessibility {
        let name = accessibility
            .label
            .as_deref()
            .filter(|label| !label.is_empty())
            .unwrap_or(node.name.as_str());
        format!("{:?}: {name}", accessibility.role)
    } else {
        node.name.clone()
    }
}

fn animation_graph_state_value(current: bool, target: bool) -> String {
    match (current, target) {
        (true, true) => "current target".to_owned(),
        (true, false) => "current".to_owned(),
        (false, true) => "target".to_owned(),
        (false, false) => "state".to_owned(),
    }
}

fn animation_graph_edge_value(
    kind: DebugAnimationGraphEdgeKind,
    active: bool,
    progress: Option<f32>,
) -> String {
    let mut parts = vec![match kind {
        DebugAnimationGraphEdgeKind::Trigger => "trigger".to_owned(),
        DebugAnimationGraphEdgeKind::Condition => "condition".to_owned(),
        DebugAnimationGraphEdgeKind::Blend => "blend".to_owned(),
    }];
    if active {
        parts.push("active".to_owned());
    }
    if let Some(progress) = progress {
        parts.push(format!("{:.0}%", progress * 100.0));
    }
    parts.join("; ")
}

fn animation_graph_action_id_part(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
            out.push(character);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "state".to_owned()
    } else {
        out
    }
}

fn absolute_rect_style(rect: crate::UiRect) -> LayoutStyle {
    LayoutStyle::from_taffy_style(Style {
        position: Position::Absolute,
        inset: TaffyRect {
            left: LengthPercentageAuto::length(rect.x),
            top: LengthPercentageAuto::length(rect.y),
            right: LengthPercentageAuto::auto(),
            bottom: LengthPercentageAuto::auto(),
        },
        size: TaffySize {
            width: Dimension::length(rect.width.max(0.0)),
            height: Dimension::length(rect.height.max(0.0)),
        },
        ..Default::default()
    })
}

fn row(
    id: impl Into<String>,
    label: impl Into<String>,
    value: impl Into<String>,
) -> PropertyGridRow {
    PropertyGridRow::new(id, label, compact_value(value.into(), 40)).read_only()
}

fn format_size(size: UiSize) -> String {
    format!("{:.0} x {:.0}", size.width, size.height)
}

fn format_point(point: crate::UiPoint) -> String {
    format!("{:.0}, {:.0}", point.x, point.y)
}

fn format_rect(rect: crate::UiRect) -> String {
    format!(
        "{:.0}, {:.0}, {:.0} x {:.0}",
        rect.x, rect.y, rect.width, rect.height
    )
}

fn compact_value(value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }
    let mut out = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        root_style, AccessibilityAction, AccessibilityMeta, AccessibilityRole, AnimatedValues,
        AnimationBlendBinding, AnimationCondition, AnimationMachine, AnimationState,
        AnimationTransition, ApproxTextMeasurer, ColorRgba, InputBehavior, StrokeStyle, UiVisual,
    };

    fn node_named(doc: &UiDocument, name: &str) -> UiNodeId {
        doc.nodes()
            .iter()
            .position(|node| node.name == name)
            .map(UiNodeId)
            .unwrap_or_else(|| panic!("missing node {name}"))
    }

    #[test]
    fn debug_inspector_panel_builds_layout_and_animation_grids() {
        let mut source = UiDocument::new(root_style(200.0, 120.0));
        source.add_child(
            source.root,
            UiNode::container("target", LayoutStyle::size(96.0, 36.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Target")
                        .focusable()
                        .action(AccessibilityAction::new("activate", "Activate")),
                )
                .with_animation(
                    AnimationMachine::new(
                        vec![
                            AnimationState::new(
                                "idle",
                                AnimatedValues::new(1.0, crate::UiPoint::new(0.0, 0.0), 1.0),
                            ),
                            AnimationState::new(
                                "hot",
                                AnimatedValues::new(1.0, crate::UiPoint::new(8.0, 0.0), 1.0),
                            ),
                        ],
                        Vec::new(),
                        "idle",
                    )
                    .expect("animation")
                    .with_number_input("hover", 0.5)
                    .with_blend_binding(AnimationBlendBinding::new("hover", "idle", "hot")),
                )
                .with_visual(UiVisual::panel(
                    ColorRgba::new(20, 80, 140, 255),
                    Some(StrokeStyle::new(ColorRgba::new(80, 120, 180, 255), 1.0)),
                    4.0,
                )),
        );
        source
            .compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("source layout");
        let snapshot = DebugInspectorSnapshot::from_document(&source, &mut ApproxTextMeasurer);

        let mut doc = UiDocument::new(root_style(360.0, 320.0));
        let root = doc.root;
        let nodes = debug_inspector_panel(
            &mut doc,
            root,
            "debug.inspector",
            &snapshot,
            DebugInspectorPanelOptions {
                selected_node: Some("target".to_string()),
                action_prefix: Some("debug.inspect".to_owned()),
                ..Default::default()
            },
        );
        doc.compute_layout(UiSize::new(360.0, 320.0), &mut ApproxTextMeasurer)
            .expect("inspector layout");

        assert_eq!(doc.node(nodes.root).name, "debug.inspector");
        assert!(!doc.node(nodes.layout_grid).children.is_empty());
        assert!(!doc.node(nodes.animation_grid).children.is_empty());
        let first_animation_input = doc.node(doc.node(nodes.animation_grid).children[7]);
        assert_eq!(
            first_animation_input
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("debug.inspect.animation.row.input.0")
        );
    }

    #[test]
    fn animation_state_graph_panel_builds_state_and_transition_nodes() {
        let mut source = UiDocument::new(root_style(240.0, 140.0));
        source.add_child(
            source.root,
            UiNode::container("animated", LayoutStyle::size(80.0, 32.0)).with_animation(
                AnimationMachine::new(
                    vec![
                        AnimationState::new(
                            "idle",
                            AnimatedValues::new(1.0, crate::UiPoint::new(0.0, 0.0), 1.0),
                        ),
                        AnimationState::new(
                            "open",
                            AnimatedValues::new(1.0, crate::UiPoint::new(20.0, 0.0), 1.1),
                        ),
                    ],
                    vec![AnimationTransition::when(
                        "idle",
                        "open",
                        AnimationCondition::bool("expanded", true),
                        0.25,
                    )],
                    "idle",
                )
                .expect("animation")
                .with_number_input("hover", 0.5)
                .with_blend_binding(AnimationBlendBinding::new("hover", "idle", "open"))
                .with_bool_input("expanded", true),
            ),
        );
        source
            .compute_layout(UiSize::new(240.0, 140.0), &mut ApproxTextMeasurer)
            .expect("source layout");
        let snapshot = DebugInspectorSnapshot::from_document(&source, &mut ApproxTextMeasurer);
        let animation = snapshot.animation("animated").expect("animation snapshot");
        let graph = animation.state_graph();

        assert_eq!(graph.states.len(), 2);
        assert!(graph
            .states
            .iter()
            .any(|state| state.name == "open" && state.target));
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == DebugAnimationGraphEdgeKind::Condition
                && edge.active
                && edge.progress == Some(0.0)
        }));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.kind == DebugAnimationGraphEdgeKind::Blend));

        let mut doc = UiDocument::new(root_style(360.0, 180.0));
        let root = doc.root;
        let nodes = animation_state_graph_panel(
            &mut doc,
            root,
            "animation.graph",
            Some(animation),
            AnimationStateGraphPanelOptions::default().with_action_prefix("anim.inspect"),
        );
        doc.compute_layout(UiSize::new(360.0, 180.0), &mut ApproxTextMeasurer)
            .expect("graph layout");

        assert_eq!(nodes.states.len(), 2);
        assert_eq!(nodes.edges.len(), 2);
        assert_eq!(
            doc.node(nodes.states[0])
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("anim.inspect.state.idle")
        );
    }

    #[test]
    fn animation_inspector_controls_panel_builds_transport_and_input_actions() {
        let mut source = UiDocument::new(root_style(240.0, 140.0));
        source.add_child(
            source.root,
            UiNode::container("animated", LayoutStyle::size(80.0, 32.0)).with_animation(
                AnimationMachine::new(
                    vec![
                        AnimationState::new(
                            "idle",
                            AnimatedValues::new(1.0, crate::UiPoint::new(0.0, 0.0), 1.0),
                        ),
                        AnimationState::new(
                            "open",
                            AnimatedValues::new(1.0, crate::UiPoint::new(20.0, 0.0), 1.1),
                        ),
                    ],
                    vec![AnimationTransition::when(
                        "idle",
                        "open",
                        AnimationCondition::trigger("pulse"),
                        0.25,
                    )],
                    "idle",
                )
                .expect("animation")
                .with_number_input("scrub", 0.5)
                .with_bool_input("active", true)
                .with_trigger_input("pulse"),
            ),
        );
        source
            .compute_layout(UiSize::new(240.0, 140.0), &mut ApproxTextMeasurer)
            .expect("source layout");
        let snapshot = DebugInspectorSnapshot::from_document(&source, &mut ApproxTextMeasurer);
        let animation = snapshot.animation("animated").expect("animation snapshot");

        let mut doc = UiDocument::new(root_style(420.0, 220.0));
        let root = doc.root;
        let nodes = animation_inspector_controls_panel(
            &mut doc,
            root,
            "animation.controls",
            Some(animation),
            AnimationInspectorControlsOptions::default()
                .with_action_prefix("anim.controls")
                .paused(true)
                .scrub_progress(0.25),
        );
        doc.compute_layout(UiSize::new(420.0, 220.0), &mut ApproxTextMeasurer)
            .expect("controls layout");

        assert_eq!(nodes.transport.len(), 3);
        assert_eq!(nodes.inputs.len(), 3);
        assert_eq!(
            doc.node(nodes.transport[0])
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("anim.controls.transport.pause_toggle")
        );
        assert_eq!(
            doc.node(nodes.transport[2]).action_mode,
            WidgetActionMode::PointerEdit
        );
        assert_eq!(
            doc.node(node_named(&doc, "animation.controls.input.active.toggle"))
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("anim.controls.input.active.toggle")
        );
        assert_eq!(
            doc.node(node_named(
                &doc,
                "animation.controls.input.scrub.set.slider"
            ))
            .action
            .as_ref()
            .and_then(|action| action.action_id())
            .map(|id| id.as_str()),
            Some("anim.controls.input.scrub.set")
        );
        assert_eq!(
            doc.node(node_named(&doc, "animation.controls.input.pulse.fire"))
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("anim.controls.input.pulse.fire")
        );
    }

    #[test]
    fn accessibility_overlay_panel_lists_focusable_and_warned_nodes() {
        let mut source = UiDocument::new(root_style(200.0, 120.0));
        source.add_child(
            source.root,
            UiNode::container("button", LayoutStyle::size(96.0, 36.0))
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label("Run")
                        .focusable(),
                ),
        );
        source
            .compute_layout(UiSize::new(200.0, 120.0), &mut ApproxTextMeasurer)
            .expect("source layout");
        let snapshot = DebugInspectorSnapshot::from_document(&source, &mut ApproxTextMeasurer);

        let mut doc = UiDocument::new(root_style(360.0, 160.0));
        let root = doc.root;
        let panel = accessibility_overlay_panel(
            &mut doc,
            root,
            "a11y.overlay",
            &snapshot,
            AccessibilityOverlayPanelOptions {
                action_prefix: Some("a11y.inspect".to_owned()),
                ..Default::default()
            },
        );
        doc.compute_layout(UiSize::new(360.0, 160.0), &mut ApproxTextMeasurer)
            .expect("overlay layout");

        assert_eq!(doc.node(panel).name, "a11y.overlay");
        let grid = doc.node(panel).children[0];
        assert!(!doc.node(grid).children.is_empty());

        let overlay = accessibility_debug_overlay(
            &mut doc,
            root,
            "a11y.visual",
            &snapshot,
            AccessibilityDebugOverlayOptions::default().with_action_prefix("a11y.visual"),
        );
        doc.compute_layout(UiSize::new(360.0, 160.0), &mut ApproxTextMeasurer)
            .expect("visual overlay layout");

        assert!(!overlay.outlines.is_empty());
        assert_eq!(overlay.outlines.len(), overlay.labels.len());
        assert_eq!(
            doc.node(overlay.outlines[0])
                .action
                .as_ref()
                .and_then(|action| action.action_id())
                .map(|id| id.as_str()),
            Some("a11y.visual.node.1")
        );
    }
}
