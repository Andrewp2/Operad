use taffy::prelude::{
    Dimension, Display, FlexDirection, LengthPercentage, LengthPercentageAuto, Rect,
    Size as TaffySize, Style,
};

use super::*;

#[derive(Debug, Clone)]
pub struct FrameOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub clip: ClipBehavior,
    pub scroll_axes: ScrollAxes,
    pub accessibility_label: Option<String>,
}

impl FrameOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }

    pub fn with_clip(mut self, clip: ClipBehavior) -> Self {
        self.clip = clip;
        self
    }

    pub fn with_scroll(mut self, axes: ScrollAxes) -> Self {
        self.scroll_axes = axes;
        self.clip = ClipBehavior::Clip;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl Default for FrameOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column()
                .with_width_percent(1.0)
                .with_padding(12.0)
                .with_gap(8.0),
            visual: UiVisual::panel(
                ColorRgba::new(24, 29, 36, 255),
                Some(StrokeStyle::new(ColorRgba::new(70, 82, 101, 255), 1.0)),
                4.0,
            ),
            clip: ClipBehavior::Clip,
            scroll_axes: ScrollAxes::NONE,
            accessibility_label: None,
        }
    }
}

pub fn frame(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: FrameOptions,
) -> UiNodeId {
    let name = name.into();
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: options.layout.style,
            clip: options.clip,
            ..Default::default()
        },
    )
    .with_visual(options.visual)
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Group).label(
            options
                .accessibility_label
                .unwrap_or_else(|| format!("{name} frame")),
        ),
    );
    if options.scroll_axes != ScrollAxes::NONE {
        node = node.with_scroll(options.scroll_axes);
    }
    document.add_child(parent, node)
}

pub fn group(document: &mut UiDocument, parent: UiNodeId, name: impl Into<String>) -> UiNodeId {
    frame(document, parent, name, FrameOptions::default())
}

#[derive(Debug, Clone)]
pub struct AreaOptions {
    pub rect: UiRect,
    pub visual: UiVisual,
    pub clip: ClipBehavior,
    pub z_index: i16,
    pub scroll_axes: ScrollAxes,
    pub action: Option<WidgetActionBinding>,
    pub action_mode: WidgetActionMode,
    pub accessibility_label: Option<String>,
}

impl AreaOptions {
    pub const fn new(rect: UiRect) -> Self {
        Self {
            rect,
            visual: UiVisual::TRANSPARENT,
            clip: ClipBehavior::Clip,
            z_index: 0,
            scroll_axes: ScrollAxes::NONE,
            action: None,
            action_mode: WidgetActionMode::Activate,
            accessibility_label: None,
        }
    }

    pub fn with_rect(mut self, rect: UiRect) -> Self {
        self.rect = rect;
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }

    pub fn with_clip(mut self, clip: ClipBehavior) -> Self {
        self.clip = clip;
        self
    }

    pub fn with_z_index(mut self, z_index: i16) -> Self {
        self.z_index = z_index;
        self
    }

    pub fn with_scroll(mut self, axes: ScrollAxes) -> Self {
        self.scroll_axes = axes;
        self.clip = ClipBehavior::Clip;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_action_mode(mut self, mode: WidgetActionMode) -> Self {
        self.action_mode = mode;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl Default for AreaOptions {
    fn default() -> Self {
        Self::new(UiRect::new(0.0, 0.0, 160.0, 120.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaNodes {
    pub root: UiNodeId,
}

pub fn area(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: AreaOptions,
    build_content: impl FnOnce(&mut UiDocument, UiNodeId),
) -> AreaNodes {
    let name = name.into();
    let mut layout = LayoutStyle::absolute_rect(options.rect);
    {
        let layout = layout.as_taffy_style_mut();
        layout.display = Display::Flex;
        layout.flex_direction = FlexDirection::Column;
    }
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: layout.style,
            clip: options.clip,
            z_index: options.z_index,
            ..Default::default()
        },
    )
    .with_visual(options.visual)
    .with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Group).label(
            options
                .accessibility_label
                .unwrap_or_else(|| format!("{name} area")),
        ),
    );
    if options.scroll_axes != ScrollAxes::NONE {
        node = node.with_scroll(options.scroll_axes);
    }
    if let Some(action) = options.action {
        node = node
            .with_input(InputBehavior::BUTTON)
            .with_action(action)
            .with_action_mode(options.action_mode);
    }
    let root = document.add_child(parent, node);
    build_content(document, root);
    AreaNodes { root }
}

#[derive(Debug, Clone)]
pub struct SceneOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub clip: ClipBehavior,
    pub action: Option<WidgetActionBinding>,
    pub action_mode: WidgetActionMode,
    pub accessibility_label: Option<String>,
}

impl SceneOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }

    pub fn with_clip(mut self, clip: ClipBehavior) -> Self {
        self.clip = clip;
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_action_mode(mut self, mode: WidgetActionMode) -> Self {
        self.action_mode = mode;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::size(160.0, 120.0),
            visual: UiVisual::TRANSPARENT,
            clip: ClipBehavior::Clip,
            action: None,
            action_mode: WidgetActionMode::Activate,
            accessibility_label: None,
        }
    }
}

pub fn scene(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    primitives: impl Into<Vec<ScenePrimitive>>,
    options: SceneOptions,
) -> UiNodeId {
    let name = name.into();
    let mut node =
        UiNode::scene(name.clone(), primitives.into(), options.layout).with_visual(options.visual);
    node.style.clip = options.clip;
    node = node.with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::Image).label(
            options
                .accessibility_label
                .unwrap_or_else(|| format!("{name} scene")),
        ),
    );
    if let Some(action) = options.action {
        node = node
            .with_input(InputBehavior::BUTTON)
            .with_action(action)
            .with_action_mode(options.action_mode);
    }
    document.add_child(parent, node)
}

#[derive(Debug, Clone)]
pub struct SidesOptions {
    pub layout: LayoutStyle,
    pub gap: f32,
    pub left_min_width: f32,
    pub right_min_width: f32,
    pub visual: UiVisual,
}

impl SidesOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_min_widths(mut self, left: f32, right: f32) -> Self {
        self.left_min_width = left.max(0.0);
        self.right_min_width = right.max(0.0);
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }
}

impl Default for SidesOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::row().with_width_percent(1.0),
            gap: 8.0,
            left_min_width: 0.0,
            right_min_width: 0.0,
            visual: UiVisual::TRANSPARENT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidesNodes {
    pub root: UiNodeId,
    pub left: UiNodeId,
    pub right: UiNodeId,
}

pub fn sides(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: SidesOptions,
    build_left: impl FnOnce(&mut UiDocument, UiNodeId),
    build_right: impl FnOnce(&mut UiDocument, UiNodeId),
) -> SidesNodes {
    let name = name.into();
    let mut layout = options.layout;
    {
        let layout = layout.as_taffy_style_mut();
        layout.display = Display::Flex;
        layout.flex_direction = FlexDirection::Row;
        layout.gap = TaffySize {
            width: LengthPercentage::length(options.gap.max(0.0)),
            height: LengthPercentage::length(options.gap.max(0.0)),
        };
    }

    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(format!("{name} sides")),
        ),
    );
    let left = document.add_child(
        root,
        side_child_style(&name, "left", options.left_min_width),
    );
    build_left(document, left);
    let right = document.add_child(
        root,
        side_child_style(&name, "right", options.right_min_width),
    );
    build_right(document, right);
    SidesNodes { root, left, right }
}

fn side_child_style(name: &str, side: &str, min_width: f32) -> UiNode {
    UiNode::container(
        format!("{name}.{side}"),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_basis: length(0.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_size: TaffySize {
                    width: length(min_width.max(0.0)),
                    height: Dimension::auto(),
                },
                ..Default::default()
            })
            .style,
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
}

#[derive(Debug, Clone)]
pub struct ColumnsOptions {
    pub layout: LayoutStyle,
    pub gap: f32,
    pub min_column_width: f32,
    pub visual: UiVisual,
}

impl ColumnsOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_min_column_width(mut self, width: f32) -> Self {
        self.min_column_width = width.max(0.0);
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }
}

impl Default for ColumnsOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::row().with_width_percent(1.0),
            gap: 8.0,
            min_column_width: 0.0,
            visual: UiVisual::TRANSPARENT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnsNodes {
    pub root: UiNodeId,
    pub columns: Vec<UiNodeId>,
}

pub fn columns(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    column_count: usize,
    options: ColumnsOptions,
    mut build_column: impl FnMut(&mut UiDocument, UiNodeId, usize),
) -> ColumnsNodes {
    let name = name.into();
    let count = column_count.max(1);
    let mut layout = options.layout;
    {
        let layout = layout.as_taffy_style_mut();
        layout.display = Display::Flex;
        layout.flex_direction = FlexDirection::Row;
        layout.gap = TaffySize {
            width: LengthPercentage::length(options.gap.max(0.0)),
            height: LengthPercentage::length(options.gap.max(0.0)),
        };
    }
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(format!("{name} columns")),
        ),
    );
    let mut columns = Vec::with_capacity(count);
    for index in 0..count {
        let column = document.add_child(
            root,
            UiNode::container(
                format!("{name}.column.{index}"),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        flex_basis: length(0.0),
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        min_size: TaffySize {
                            width: length(options.min_column_width.max(0.0)),
                            height: Dimension::auto(),
                        },
                        ..Default::default()
                    })
                    .style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        build_column(document, column, index);
        columns.push(column);
    }
    ColumnsNodes { root, columns }
}

#[derive(Debug, Clone)]
pub struct IndentOptions {
    pub layout: LayoutStyle,
    pub amount: f32,
    pub visual: UiVisual,
    pub accessibility_label: Option<String>,
}

impl IndentOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_amount(mut self, amount: f32) -> Self {
        self.amount = amount.max(0.0);
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl Default for IndentOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column().with_width_percent(1.0),
            amount: 18.0,
            visual: UiVisual::TRANSPARENT,
            accessibility_label: None,
        }
    }
}

pub fn indented_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: IndentOptions,
) -> UiNodeId {
    let name = name.into();
    let mut layout = options.layout;
    layout.as_taffy_style_mut().padding.left = LengthPercentage::length(options.amount.max(0.0));
    document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(
                options
                    .accessibility_label
                    .unwrap_or_else(|| format!("{name} indented section")),
            ),
        ),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandlePlacement {
    BottomRight,
    Right,
    Bottom,
    Inline,
}

#[derive(Debug, Clone)]
pub struct ResizeHandleOptions {
    pub layout: LayoutStyle,
    pub placement: ResizeHandlePlacement,
    pub size: f32,
    pub visual: UiVisual,
    pub grip_color: ColorRgba,
    pub action: Option<WidgetActionBinding>,
    pub accessibility_label: Option<String>,
}

impl ResizeHandleOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self.placement = ResizeHandlePlacement::Inline;
        self
    }

    pub fn with_placement(mut self, placement: ResizeHandlePlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size.max(8.0);
        self
    }

    pub fn with_action(mut self, action: impl Into<WidgetActionBinding>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl Default for ResizeHandleOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::size(16.0, 16.0),
            placement: ResizeHandlePlacement::BottomRight,
            size: 16.0,
            visual: UiVisual::panel(ColorRgba::TRANSPARENT, None, 0.0),
            grip_color: ColorRgba::new(120, 134, 156, 210),
            action: None,
            accessibility_label: None,
        }
    }
}

pub fn resize_handle(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: ResizeHandleOptions,
) -> UiNodeId {
    let name = name.into();
    let size = options.size.max(8.0);
    let layout = resize_handle_layout(options.placement, size, options.layout);
    let mut handle = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: layout.style,
            clip: ClipBehavior::Clip,
            z_index: 2,
            ..Default::default()
        },
    )
    .with_visual(options.visual)
    .with_accessibility(
        AccessibilityMeta::new(if options.action.is_some() {
            AccessibilityRole::Button
        } else {
            AccessibilityRole::Image
        })
        .label(
            options
                .accessibility_label
                .clone()
                .unwrap_or_else(|| format!("{name} resize handle")),
        ),
    );
    if let Some(action) = options.action {
        handle = handle
            .with_input(InputBehavior::BUTTON)
            .with_action(action)
            .with_action_mode(WidgetActionMode::PointerEdit);
    }
    let handle = document.add_child(parent, handle);
    document.add_child(
        handle,
        UiNode::scene(
            format!("{name}.grip"),
            resize_grip_lines(size, options.grip_color),
            LayoutStyle::size(size, size),
        )
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Image).label("Resize grip")),
    );
    handle
}

fn resize_handle_layout(
    placement: ResizeHandlePlacement,
    size: f32,
    inline_layout: LayoutStyle,
) -> LayoutStyle {
    match placement {
        ResizeHandlePlacement::Inline => inline_layout.with_size(size, size),
        ResizeHandlePlacement::BottomRight => LayoutStyle::from_taffy_style(Style {
            position: taffy::prelude::Position::Absolute,
            inset: Rect {
                left: LengthPercentageAuto::auto(),
                right: LengthPercentageAuto::length(4.0),
                top: LengthPercentageAuto::auto(),
                bottom: LengthPercentageAuto::length(4.0),
            },
            size: TaffySize {
                width: length(size),
                height: length(size),
            },
            ..Default::default()
        }),
        ResizeHandlePlacement::Right => LayoutStyle::from_taffy_style(Style {
            position: taffy::prelude::Position::Absolute,
            inset: Rect {
                left: LengthPercentageAuto::auto(),
                right: LengthPercentageAuto::length(4.0),
                top: LengthPercentageAuto::length(4.0),
                bottom: LengthPercentageAuto::length(4.0),
            },
            size: TaffySize {
                width: length(size),
                height: Dimension::auto(),
            },
            ..Default::default()
        }),
        ResizeHandlePlacement::Bottom => LayoutStyle::from_taffy_style(Style {
            position: taffy::prelude::Position::Absolute,
            inset: Rect {
                left: LengthPercentageAuto::length(4.0),
                right: LengthPercentageAuto::length(4.0),
                top: LengthPercentageAuto::auto(),
                bottom: LengthPercentageAuto::length(4.0),
            },
            size: TaffySize {
                width: Dimension::auto(),
                height: length(size),
            },
            ..Default::default()
        }),
    }
}

fn resize_grip_lines(size: f32, color: ColorRgba) -> Vec<ScenePrimitive> {
    vec![
        ScenePrimitive::Line {
            from: UiPoint::new(size - 5.0, size - 13.0),
            to: UiPoint::new(size - 13.0, size - 5.0),
            stroke: StrokeStyle::new(color, 1.0),
        },
        ScenePrimitive::Line {
            from: UiPoint::new(size - 4.0, size - 9.0),
            to: UiPoint::new(size - 9.0, size - 4.0),
            stroke: StrokeStyle::new(color, 1.0),
        },
        ScenePrimitive::Line {
            from: UiPoint::new(size - 3.0, size - 5.0),
            to: UiPoint::new(size - 5.0, size - 3.0),
            stroke: StrokeStyle::new(color, 1.0),
        },
    ]
}

#[derive(Debug, Clone)]
pub struct ResizeContainerOptions {
    pub layout: LayoutStyle,
    pub visual: UiVisual,
    pub content_padding: f32,
    pub handle: ResizeHandleOptions,
    pub accessibility_label: Option<String>,
}

impl ResizeContainerOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }

    pub fn with_content_padding(mut self, padding: f32) -> Self {
        self.content_padding = padding.max(0.0);
        self
    }

    pub fn with_handle(mut self, handle: ResizeHandleOptions) -> Self {
        self.handle = handle;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl Default for ResizeContainerOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::column()
                .with_width_percent(1.0)
                .with_height_percent(1.0),
            visual: UiVisual::panel(
                ColorRgba::new(24, 29, 36, 255),
                Some(StrokeStyle::new(ColorRgba::new(70, 82, 101, 255), 1.0)),
                4.0,
            ),
            content_padding: 12.0,
            handle: ResizeHandleOptions::default(),
            accessibility_label: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeContainerNodes {
    pub root: UiNodeId,
    pub content: UiNodeId,
    pub handle: UiNodeId,
}

pub fn resize_container(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: ResizeContainerOptions,
    build_content: impl FnOnce(&mut UiDocument, UiNodeId),
) -> ResizeContainerNodes {
    let name = name.into();
    let mut root_layout = options.layout;
    {
        let layout = root_layout.as_taffy_style_mut();
        layout.display = Display::Flex;
        layout.flex_direction = FlexDirection::Column;
    }
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: root_layout.style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(
                options
                    .accessibility_label
                    .unwrap_or_else(|| format!("{name} resize container")),
            ),
        ),
    );
    let content = document.add_child(
        root,
        UiNode::container(
            format!("{name}.content"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: length(0.0),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    padding: Rect {
                        left: LengthPercentage::length(options.content_padding),
                        right: LengthPercentage::length(options.content_padding),
                        top: LengthPercentage::length(options.content_padding),
                        bottom: LengthPercentage::length(options.content_padding),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    build_content(document, content);
    let handle = resize_handle(
        document,
        root,
        format!("{name}.resize"),
        options
            .handle
            .with_placement(ResizeHandlePlacement::BottomRight),
    );
    ResizeContainerNodes {
        root,
        content,
        handle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_helpers_build_frames_columns_sides_and_resize_handles() {
        let mut document = UiDocument::new(root_style(640.0, 360.0));
        let root = document.root;

        let frame_id = frame(
            &mut document,
            root,
            "card",
            FrameOptions::default()
                .accessibility_label("Card")
                .with_scroll(ScrollAxes::VERTICAL),
        );
        let group_id = group(&mut document, root, "group");
        let area_nodes = area(
            &mut document,
            root,
            "floating_area",
            AreaOptions::new(UiRect::new(20.0, 24.0, 180.0, 96.0))
                .with_z_index(7)
                .with_action("area.activate")
                .with_action_mode(WidgetActionMode::PointerEdit),
            |document, parent| {
                label(
                    document,
                    parent,
                    "floating_area.label",
                    "Area",
                    TextStyle::default(),
                    LayoutStyle::new(),
                );
            },
        );
        let scene_id = scene(
            &mut document,
            root,
            "scene",
            vec![ScenePrimitive::Line {
                from: UiPoint::new(0.0, 0.0),
                to: UiPoint::new(24.0, 24.0),
                stroke: StrokeStyle::new(ColorRgba::WHITE, 1.0),
            }],
            SceneOptions::default().with_action("scene.activate"),
        );
        let sides_nodes = sides(
            &mut document,
            root,
            "properties",
            SidesOptions::default().with_min_widths(120.0, 160.0),
            |document, parent| {
                label(
                    document,
                    parent,
                    "left.label",
                    "Left",
                    TextStyle::default(),
                    LayoutStyle::new(),
                );
            },
            |document, parent| {
                label(
                    document,
                    parent,
                    "right.label",
                    "Right",
                    TextStyle::default(),
                    LayoutStyle::new(),
                );
            },
        );
        let columns_nodes = columns(
            &mut document,
            root,
            "columns",
            3,
            ColumnsOptions::default().with_min_column_width(40.0),
            |document, parent, index| {
                label(
                    document,
                    parent,
                    format!("column.{index}.label"),
                    format!("Column {index}"),
                    TextStyle::default(),
                    LayoutStyle::new(),
                );
            },
        );
        let indent = indented_section(&mut document, root, "indent", IndentOptions::default());
        let resize_nodes = resize_container(
            &mut document,
            root,
            "resizable",
            ResizeContainerOptions::default()
                .with_handle(ResizeHandleOptions::default().with_action("panel.resize")),
            |document, parent| {
                label(
                    document,
                    parent,
                    "resizable.label",
                    "Resizable",
                    TextStyle::default(),
                    LayoutStyle::new(),
                );
            },
        );

        assert_eq!(
            document
                .node(frame_id)
                .accessibility
                .as_ref()
                .unwrap()
                .label
                .as_deref(),
            Some("Card")
        );
        assert_eq!(
            document.node(frame_id).scroll.unwrap().axes,
            ScrollAxes::VERTICAL
        );
        assert_eq!(
            document.node(group_id).accessibility.as_ref().unwrap().role,
            AccessibilityRole::Group
        );
        assert_eq!(
            document.node(area_nodes.root).style.layout.position,
            taffy::prelude::Position::Absolute
        );
        assert_eq!(document.node(area_nodes.root).style.z_index, 7);
        assert_eq!(
            document.node(area_nodes.root).action.as_ref(),
            Some(&WidgetActionBinding::action("area.activate"))
        );
        assert!(matches!(
            document.node(scene_id).content,
            UiContent::Scene(ref primitives) if primitives.len() == 1
        ));
        assert_eq!(
            document.node(scene_id).action.as_ref(),
            Some(&WidgetActionBinding::action("scene.activate"))
        );
        assert_eq!(document.node(sides_nodes.root).children.len(), 2);
        assert_eq!(columns_nodes.columns.len(), 3);
        assert_eq!(
            document.node(indent).style.layout.padding.left,
            LengthPercentage::length(18.0)
        );
        assert_eq!(
            document.node(resize_nodes.handle).action.as_ref(),
            Some(&WidgetActionBinding::action("panel.resize"))
        );
        assert_eq!(
            document.node(resize_nodes.handle).action_mode,
            WidgetActionMode::PointerEdit
        );
    }
}
