//! Dock workspace widget.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, LengthPercentageAuto, Position, Rect,
    Size as TaffySize, Style,
};

use crate::{
    length, AccessibilityMeta, AccessibilityRole, ClipBehavior, ImageContent, InputBehavior,
    LayoutStyle, ShaderEffect, StrokeStyle, TextStyle, UiDocument, UiNode, UiNodeId, UiNodeStyle,
    UiSize, UiVisual,
};

use super::surfaces::{DEFAULT_ACCENT, DEFAULT_SURFACE_BG, DEFAULT_SURFACE_STROKE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockSide {
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

impl DockSide {
    pub const fn is_horizontal_edge(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    pub const fn is_vertical_edge(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

fn dock_side_name(side: DockSide) -> &'static str {
    match side {
        DockSide::Top => "Top",
        DockSide::Bottom => "Bottom",
        DockSide::Left => "Left",
        DockSide::Right => "Right",
        DockSide::Center => "Center",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockPanelDescriptor {
    pub id: String,
    pub title: String,
    pub side: DockSide,
    pub size: f32,
    pub min_size: f32,
    pub visible: bool,
    pub resizable: bool,
    pub title_image: Option<ImageContent>,
    pub shader: Option<ShaderEffect>,
    pub accessibility_label: Option<String>,
    pub accessibility_hint: Option<String>,
}

impl DockPanelDescriptor {
    pub fn new(id: impl Into<String>, title: impl Into<String>, side: DockSide, size: f32) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            side,
            size: size.max(0.0),
            min_size: 48.0,
            visible: true,
            resizable: false,
            title_image: None,
            shader: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }

    pub fn center(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            side: DockSide::Center,
            size: 1.0,
            min_size: 0.0,
            visible: true,
            resizable: false,
            title_image: None,
            shader: None,
            accessibility_label: None,
            accessibility_hint: None,
        }
    }

    pub fn with_min_size(mut self, min_size: f32) -> Self {
        self.min_size = min_size.max(0.0);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn title_image(mut self, image: ImageContent) -> Self {
        self.title_image = Some(image);
        self
    }

    pub fn shader(mut self, shader: ShaderEffect) -> Self {
        self.shader = Some(shader);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    pub fn accessibility(&self) -> AccessibilityMeta {
        let label = self
            .accessibility_label
            .clone()
            .or_else(|| (!self.title.is_empty()).then(|| self.title.clone()))
            .unwrap_or_else(|| self.id.clone());
        let hint = self.accessibility_hint.clone().unwrap_or_else(|| {
            if self.resizable {
                format!("{} dock panel, resizable", dock_side_name(self.side))
            } else {
                format!("{} dock panel", dock_side_name(self.side))
            }
        });
        AccessibilityMeta::new(AccessibilityRole::TabPanel)
            .label(label)
            .hint(hint)
    }
}

#[derive(Debug, Clone)]
pub struct DockWorkspaceOptions {
    pub layout: LayoutStyle,
    pub panel_visual: UiVisual,
    pub center_visual: UiVisual,
    pub resize_handle_visual: UiVisual,
    pub title_style: TextStyle,
    pub show_titles: bool,
    pub handle_thickness: f32,
    pub title_image_size: UiSize,
}

impl Default for DockWorkspaceOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            }),
            panel_visual: UiVisual::panel(
                DEFAULT_SURFACE_BG,
                Some(StrokeStyle::new(DEFAULT_SURFACE_STROKE, 1.0)),
                0.0,
            ),
            center_visual: UiVisual::TRANSPARENT,
            resize_handle_visual: UiVisual::panel(DEFAULT_ACCENT, None, 0.0),
            title_style: TextStyle {
                font_size: 13.0,
                line_height: 18.0,
                ..Default::default()
            },
            show_titles: true,
            handle_thickness: 5.0,
            title_image_size: UiSize::new(16.0, 16.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockPanelNode {
    pub id: String,
    pub side: DockSide,
    pub root: UiNodeId,
    pub title: Option<UiNodeId>,
    pub content: UiNodeId,
    pub resize_handle: Option<UiNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockWorkspaceNodes {
    pub root: UiNodeId,
    pub body: UiNodeId,
    pub center: Option<UiNodeId>,
    pub panels: Vec<DockPanelNode>,
}

pub fn dock_workspace(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    panels: &[DockPanelDescriptor],
    options: DockWorkspaceOptions,
    mut build_panel: impl FnMut(&mut UiDocument, UiNodeId, &DockPanelDescriptor),
) -> DockWorkspaceNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.style.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Application)
                .label(format!("{name} docking workspace"))
                .hint("Contains docked panels and a center workspace"),
        ),
    );
    let mut panel_nodes = Vec::new();

    for panel in panels_for_side(panels, DockSide::Top) {
        panel_nodes.push(add_dock_panel(document, root, &name, panel, &options));
        if let Some(node) = panel_nodes.last() {
            build_panel(document, node.content, panel);
        }
    }

    let body = document.add_child(
        root,
        UiNode::container(
            format!("{name}.body"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: length(0.0),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );

    for side in [DockSide::Left, DockSide::Center, DockSide::Right] {
        for panel in panels_for_side(panels, side) {
            panel_nodes.push(add_dock_panel(document, body, &name, panel, &options));
            if let Some(node) = panel_nodes.last() {
                build_panel(document, node.content, panel);
            }
        }
    }

    let center = panel_nodes
        .iter()
        .find(|panel| panel.side == DockSide::Center)
        .map(|panel| panel.root)
        .or_else(|| {
            let fallback = DockPanelDescriptor::center("center", "");
            let node = add_dock_panel(document, body, &name, &fallback, &options);
            let root = node.root;
            panel_nodes.push(node);
            Some(root)
        });

    for panel in panels_for_side(panels, DockSide::Bottom) {
        panel_nodes.push(add_dock_panel(document, root, &name, panel, &options));
        if let Some(node) = panel_nodes.last() {
            build_panel(document, node.content, panel);
        }
    }

    DockWorkspaceNodes {
        root,
        body,
        center,
        panels: panel_nodes,
    }
}

fn panels_for_side(
    panels: &[DockPanelDescriptor],
    side: DockSide,
) -> impl Iterator<Item = &DockPanelDescriptor> {
    panels
        .iter()
        .filter(move |panel| panel.visible && panel.side == side)
}

fn add_dock_panel(
    document: &mut UiDocument,
    parent: UiNodeId,
    workspace_name: &str,
    panel: &DockPanelDescriptor,
    options: &DockWorkspaceOptions,
) -> DockPanelNode {
    let mut root_node = UiNode::container(
        format!("{workspace_name}.panel.{}", panel.id),
        dock_panel_style(panel),
    )
    .with_visual(if panel.side == DockSide::Center {
        options.center_visual
    } else {
        options.panel_visual
    })
    .with_accessibility(panel.accessibility());
    if let Some(shader) = &panel.shader {
        root_node = root_node.with_shader(shader.clone());
    }
    let root = document.add_child(parent, root_node);

    let title = if options.show_titles && (!panel.title.is_empty() || panel.title_image.is_some()) {
        let title_bar = document.add_child(
            root,
            UiNode::container(
                format!("{workspace_name}.panel.{}.title", panel.id),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: length(24.0),
                        },
                        padding: Rect::length(4.0),
                        flex_shrink: 0.0,
                        ..Default::default()
                    })
                    .style,
                    ..Default::default()
                },
            ),
        );
        if let Some(image) = &panel.title_image {
            document.add_child(
                title_bar,
                UiNode::image(
                    format!("{workspace_name}.panel.{}.title.image", panel.id),
                    image.clone(),
                    LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: length(options.title_image_size.width),
                            height: length(options.title_image_size.height),
                        },
                        margin: Rect {
                            right: LengthPercentageAuto::length(6.0),
                            ..Rect::length(0.0)
                        },
                        flex_shrink: 0.0,
                        ..Default::default()
                    }),
                )
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Image).label(panel.title.clone()),
                ),
            );
        }
        if !panel.title.is_empty() {
            document.add_child(
                title_bar,
                UiNode::text(
                    format!("{workspace_name}.panel.{}.title.label", panel.id),
                    panel.title.clone(),
                    options.title_style.clone(),
                    LayoutStyle::from_taffy_style(Style {
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: Dimension::auto(),
                        },
                        ..Default::default()
                    }),
                )
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Label).label(panel.title.clone()),
                ),
            );
        }
        Some(title_bar)
    } else {
        None
    };

    let content = document.add_child(
        root,
        UiNode::container(
            format!("{workspace_name}.panel.{}.content", panel.id),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: length(0.0),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );

    let resize_handle = panel
        .resizable
        .then(|| add_dock_resize_handle(document, root, workspace_name, panel, options));

    DockPanelNode {
        id: panel.id.clone(),
        side: panel.side,
        root,
        title,
        content,
        resize_handle,
    }
}

fn dock_panel_style(panel: &DockPanelDescriptor) -> UiNodeStyle {
    let mut layout = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        flex_shrink: 0.0,
        ..Default::default()
    };
    match panel.side {
        DockSide::Top | DockSide::Bottom => {
            layout.size = TaffySize {
                width: Dimension::percent(1.0),
                height: length(panel.size),
            };
            layout.min_size.height = length(panel.min_size);
        }
        DockSide::Left | DockSide::Right => {
            layout.size = TaffySize {
                width: length(panel.size),
                height: Dimension::percent(1.0),
            };
            layout.min_size.width = length(panel.min_size);
        }
        DockSide::Center => {
            layout.flex_grow = panel.size.max(0.0);
            layout.flex_shrink = 1.0;
            layout.flex_basis = length(0.0);
            layout.size = TaffySize {
                width: Dimension::percent(1.0),
                height: Dimension::percent(1.0),
            };
            layout.min_size.width = length(panel.min_size);
        }
    }
    UiNodeStyle {
        layout: LayoutStyle::from_taffy_style(layout).style,
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

fn add_dock_resize_handle(
    document: &mut UiDocument,
    parent: UiNodeId,
    workspace_name: &str,
    panel: &DockPanelDescriptor,
    options: &DockWorkspaceOptions,
) -> UiNodeId {
    let mut inset = Rect::length(0.0);
    let size = match panel.side {
        DockSide::Top => {
            inset.top = LengthPercentageAuto::auto();
            TaffySize {
                width: Dimension::percent(1.0),
                height: length(options.handle_thickness),
            }
        }
        DockSide::Bottom => {
            inset.bottom = LengthPercentageAuto::auto();
            TaffySize {
                width: Dimension::percent(1.0),
                height: length(options.handle_thickness),
            }
        }
        DockSide::Left => {
            inset.left = LengthPercentageAuto::auto();
            TaffySize {
                width: length(options.handle_thickness),
                height: Dimension::percent(1.0),
            }
        }
        DockSide::Right => {
            inset.right = LengthPercentageAuto::auto();
            TaffySize {
                width: length(options.handle_thickness),
                height: Dimension::percent(1.0),
            }
        }
        DockSide::Center => TaffySize {
            width: length(0.0),
            height: length(0.0),
        },
    };
    document.add_child(
        parent,
        UiNode::container(
            format!("{workspace_name}.panel.{}.resize", panel.id),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    position: Position::Absolute,
                    inset,
                    size,
                    ..Default::default()
                })
                .style,
                z_index: 1,
                ..Default::default()
            },
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(options.resize_handle_visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Slider)
                .label(format!("Resize {} panel", panel.title))
                .hint(format!(
                    "Drag to resize the {} dock panel",
                    dock_side_name(panel.side).to_lowercase()
                ))
                .focusable(),
        ),
    )
}
