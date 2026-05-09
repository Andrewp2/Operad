//! Workspace surface widgets: split panes, docking, toasts, and timelines.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, LengthPercentageAuto, Position, Rect,
    Size as TaffySize, Style,
};

use crate::{
    length, ClipBehavior, ColorRgba, InputBehavior, ScenePrimitive, StrokeStyle, TextStyle,
    UiDocument, UiNode, UiNodeId, UiNodeStyle, UiPoint, UiRect, UiSize, UiVisual,
};

const DEFAULT_SURFACE_BG: ColorRgba = ColorRgba::new(24, 29, 36, 255);
const DEFAULT_SURFACE_STROKE: ColorRgba = ColorRgba::new(70, 82, 101, 255);
const DEFAULT_ACCENT: ColorRgba = ColorRgba::new(108, 180, 255, 255);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

impl SplitAxis {
    pub const fn flex_direction(self) -> FlexDirection {
        match self {
            Self::Horizontal => FlexDirection::Row,
            Self::Vertical => FlexDirection::Column,
        }
    }

    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitPaneSizes {
    pub first: f32,
    pub handle: f32,
    pub second: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitPaneState {
    pub fraction: f32,
    pub min_first: f32,
    pub min_second: f32,
}

impl SplitPaneState {
    pub fn new(fraction: f32) -> Self {
        Self {
            fraction: fraction.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    pub fn with_min_sizes(mut self, first: f32, second: f32) -> Self {
        self.min_first = first.max(0.0);
        self.min_second = second.max(0.0);
        self
    }

    pub fn set_fraction(&mut self, fraction: f32) -> bool {
        if !fraction.is_finite() {
            return false;
        }
        let fraction = fraction.clamp(0.0, 1.0);
        if (self.fraction - fraction).abs() <= f32::EPSILON {
            return false;
        }
        self.fraction = fraction;
        true
    }

    pub fn resolved_sizes(self, total_extent: f32, handle_thickness: f32) -> SplitPaneSizes {
        let total = total_extent.max(0.0);
        let handle = handle_thickness.max(0.0).min(total);
        let available = (total - handle).max(0.0);
        if available <= f32::EPSILON {
            return SplitPaneSizes {
                first: 0.0,
                handle,
                second: 0.0,
            };
        }

        let mut min_first = self.min_first.max(0.0);
        let mut min_second = self.min_second.max(0.0);
        let min_total = min_first + min_second;
        if min_total > available && min_total > f32::EPSILON {
            let scale = available / min_total;
            min_first *= scale;
            min_second *= scale;
        }

        let lower = min_first.min(available);
        let upper = (available - min_second).max(lower);
        let desired = available * self.fraction.clamp(0.0, 1.0);
        let first = desired.clamp(lower, upper);
        SplitPaneSizes {
            first,
            handle,
            second: (available - first).max(0.0),
        }
    }

    pub fn resize_by(&mut self, delta: f32, total_extent: f32, handle_thickness: f32) -> bool {
        if !delta.is_finite() || !total_extent.is_finite() || !handle_thickness.is_finite() {
            return false;
        }
        let available = (total_extent.max(0.0) - handle_thickness.max(0.0)).max(0.0);
        if available <= f32::EPSILON {
            return false;
        }
        let sizes = self.resolved_sizes(total_extent, handle_thickness);
        let next_first = (sizes.first + delta).clamp(0.0, available);
        self.set_fraction(next_first / available)
    }
}

impl Default for SplitPaneState {
    fn default() -> Self {
        Self {
            fraction: 0.5,
            min_first: 48.0,
            min_second: 48.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SplitPaneOptions {
    pub layout: Style,
    pub handle_thickness: f32,
    pub root_visual: UiVisual,
    pub pane_visual: UiVisual,
    pub handle_visual: UiVisual,
}

impl Default for SplitPaneOptions {
    fn default() -> Self {
        Self {
            layout: Style {
                display: Display::Flex,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            },
            handle_thickness: 6.0,
            root_visual: UiVisual::TRANSPARENT,
            pane_visual: UiVisual::TRANSPARENT,
            handle_visual: UiVisual::panel(DEFAULT_SURFACE_STROKE, None, 2.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitPaneNodes {
    pub root: UiNodeId,
    pub first: UiNodeId,
    pub handle: UiNodeId,
    pub second: UiNodeId,
}

pub fn split_pane(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    axis: SplitAxis,
    state: SplitPaneState,
    options: SplitPaneOptions,
    build_first: impl FnOnce(&mut UiDocument, UiNodeId),
    build_second: impl FnOnce(&mut UiDocument, UiNodeId),
) -> SplitPaneNodes {
    let name = name.into();
    let mut layout = options.layout;
    layout.display = Display::Flex;
    layout.flex_direction = axis.flex_direction();

    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.root_visual),
    );
    let first = document.add_child(
        root,
        UiNode::container(
            format!("{name}.first"),
            split_pane_child_style(axis, state.fraction, state.min_first),
        )
        .with_visual(options.pane_visual),
    );
    build_first(document, first);

    let handle = document.add_child(
        root,
        UiNode::container(
            format!("{name}.handle"),
            split_pane_handle_style(axis, options.handle_thickness),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(options.handle_visual),
    );

    let second = document.add_child(
        root,
        UiNode::container(
            format!("{name}.second"),
            split_pane_child_style(axis, 1.0 - state.fraction, state.min_second),
        )
        .with_visual(options.pane_visual),
    );
    build_second(document, second);

    SplitPaneNodes {
        root,
        first,
        handle,
        second,
    }
}

fn split_pane_child_style(axis: SplitAxis, grow: f32, min_extent: f32) -> UiNodeStyle {
    let mut layout = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        flex_basis: length(0.0),
        flex_grow: grow.max(0.0),
        flex_shrink: 1.0,
        ..Default::default()
    };
    if axis.is_horizontal() {
        layout.size.height = Dimension::percent(1.0);
        layout.min_size.width = length(min_extent.max(0.0));
    } else {
        layout.size.width = Dimension::percent(1.0);
        layout.min_size.height = length(min_extent.max(0.0));
    }
    UiNodeStyle {
        layout,
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

fn split_pane_handle_style(axis: SplitAxis, thickness: f32) -> UiNodeStyle {
    let thickness = thickness.max(0.0);
    let size = if axis.is_horizontal() {
        TaffySize {
            width: length(thickness),
            height: Dimension::percent(1.0),
        }
    } else {
        TaffySize {
            width: Dimension::percent(1.0),
            height: length(thickness),
        }
    };
    UiNodeStyle {
        layout: Style {
            flex_shrink: 0.0,
            size,
            ..Default::default()
        },
        clip: ClipBehavior::Clip,
        ..Default::default()
    }
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct DockPanelDescriptor {
    pub id: String,
    pub title: String,
    pub side: DockSide,
    pub size: f32,
    pub min_size: f32,
    pub visible: bool,
    pub resizable: bool,
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
}

#[derive(Debug, Clone)]
pub struct DockWorkspaceOptions {
    pub layout: Style,
    pub panel_visual: UiVisual,
    pub center_visual: UiVisual,
    pub resize_handle_visual: UiVisual,
    pub title_style: TextStyle,
    pub show_titles: bool,
    pub handle_thickness: f32,
}

impl Default for DockWorkspaceOptions {
    fn default() -> Self {
        Self {
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            },
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
                layout: options.layout.clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
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
                layout: Style {
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
                },
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
    let root = document.add_child(
        parent,
        UiNode::container(
            format!("{workspace_name}.panel.{}", panel.id),
            dock_panel_style(panel),
        )
        .with_visual(if panel.side == DockSide::Center {
            options.center_visual
        } else {
            options.panel_visual
        }),
    );

    let title = if options.show_titles && !panel.title.is_empty() {
        Some(document.add_child(
            root,
            UiNode::text(
                format!("{workspace_name}.panel.{}.title", panel.id),
                panel.title.clone(),
                options.title_style.clone(),
                Style {
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(24.0),
                    },
                    padding: Rect::length(4.0),
                    flex_shrink: 0.0,
                    ..Default::default()
                },
            ),
        ))
    } else {
        None
    };

    let content = document.add_child(
        root,
        UiNode::container(
            format!("{workspace_name}.panel.{}.content", panel.id),
            UiNodeStyle {
                layout: Style {
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
                },
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
        layout,
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
                layout: Style {
                    position: Position::Absolute,
                    inset,
                    size,
                    ..Default::default()
                },
                z_index: 1,
                ..Default::default()
            },
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(options.resize_handle_visual),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogDismissReason {
    EscapeKey,
    OutsidePointer,
    CloseButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogDismissal {
    pub escape_key: bool,
    pub outside_pointer: bool,
    pub close_button: bool,
}

impl DialogDismissal {
    pub const NONE: Self = Self {
        escape_key: false,
        outside_pointer: false,
        close_button: false,
    };

    pub const STANDARD: Self = Self {
        escape_key: true,
        outside_pointer: true,
        close_button: true,
    };

    pub const MODAL: Self = Self {
        escape_key: true,
        outside_pointer: false,
        close_button: true,
    };

    pub const fn allows(self, reason: DialogDismissReason) -> bool {
        match reason {
            DialogDismissReason::EscapeKey => self.escape_key,
            DialogDismissReason::OutsidePointer => self.outside_pointer,
            DialogDismissReason::CloseButton => self.close_button,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogDescriptor {
    pub id: String,
    pub title: String,
    pub modal: bool,
    pub trap_focus: bool,
    pub dismissal: DialogDismissal,
}

impl DialogDescriptor {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            modal: false,
            trap_focus: false,
            dismissal: DialogDismissal::STANDARD,
        }
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self.trap_focus = modal;
        if modal {
            self.dismissal = DialogDismissal::MODAL;
        }
        self
    }

    pub fn trap_focus(mut self, trap_focus: bool) -> Self {
        self.trap_focus = trap_focus;
        self
    }

    pub fn dismissal(mut self, dismissal: DialogDismissal) -> Self {
        self.dismissal = dismissal;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DialogStack {
    pub dialogs: Vec<DialogDescriptor>,
}

impl DialogStack {
    pub fn open(&mut self, dialog: DialogDescriptor) {
        self.close(&dialog.id);
        self.dialogs.push(dialog);
    }

    pub fn close(&mut self, id: &str) -> Option<DialogDescriptor> {
        let index = self.dialogs.iter().position(|dialog| dialog.id == id)?;
        Some(self.dialogs.remove(index))
    }

    pub fn dismiss_top(&mut self, reason: DialogDismissReason) -> Option<DialogDescriptor> {
        let top = self.dialogs.last()?;
        if !top.dismissal.allows(reason) {
            return None;
        }
        self.dialogs.pop()
    }

    pub fn top(&self) -> Option<&DialogDescriptor> {
        self.dialogs.last()
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.dialogs.iter().any(|dialog| dialog.id == id)
    }

    pub fn traps_focus(&self) -> bool {
        self.dialogs
            .iter()
            .any(|dialog| dialog.modal || dialog.trap_focus)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopoverAnchor {
    Node(UiNodeId),
    Rect(UiRect),
    Point(UiPoint),
}

impl PopoverAnchor {
    pub fn resolve(self, document: &UiDocument) -> Option<UiRect> {
        match self {
            Self::Node(id) => document.nodes().get(id.0).map(|node| node.layout.rect),
            Self::Rect(rect) => Some(rect),
            Self::Point(point) => Some(UiRect::new(point.x, point.y, 0.0, 0.0)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverPlacement {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopoverDescriptor {
    pub id: String,
    pub anchor: PopoverAnchor,
    pub placement: PopoverPlacement,
    pub modal: bool,
    pub close_on_outside: bool,
}

impl PopoverDescriptor {
    pub fn new(id: impl Into<String>, anchor: PopoverAnchor, placement: PopoverPlacement) -> Self {
        Self {
            id: id.into(),
            anchor,
            placement,
            modal: false,
            close_on_outside: true,
        }
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    pub fn close_on_outside(mut self, close_on_outside: bool) -> Self {
        self.close_on_outside = close_on_outside;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PopoverState {
    pub current: Option<PopoverDescriptor>,
}

impl PopoverState {
    pub fn open(&mut self, popover: PopoverDescriptor) {
        self.current = Some(popover);
    }

    pub fn close(&mut self) -> Option<PopoverDescriptor> {
        self.current.take()
    }

    pub fn toggle(&mut self, popover: PopoverDescriptor) {
        if self.is_open(&popover.id) {
            self.close();
        } else {
            self.open(popover);
        }
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.current
            .as_ref()
            .is_some_and(|popover| popover.id == id)
    }

    pub fn dismiss_for_outside_pointer(&mut self) -> Option<PopoverDescriptor> {
        if self
            .current
            .as_ref()
            .is_some_and(|popover| popover.close_on_outside)
        {
            return self.close();
        }
        None
    }
}

pub fn resolve_popover_rect(
    anchor: UiRect,
    popover_size: UiSize,
    viewport: UiRect,
    placement: PopoverPlacement,
    offset: f32,
) -> UiRect {
    let offset = offset.max(0.0);
    let mut rect = match placement {
        PopoverPlacement::Top => UiRect::new(
            anchor.x,
            anchor.y - popover_size.height - offset,
            popover_size.width,
            popover_size.height,
        ),
        PopoverPlacement::Bottom => UiRect::new(
            anchor.x,
            anchor.bottom() + offset,
            popover_size.width,
            popover_size.height,
        ),
        PopoverPlacement::Left => UiRect::new(
            anchor.x - popover_size.width - offset,
            anchor.y,
            popover_size.width,
            popover_size.height,
        ),
        PopoverPlacement::Right => UiRect::new(
            anchor.right() + offset,
            anchor.y,
            popover_size.width,
            popover_size.height,
        ),
    };
    rect.x = clamp_to_viewport(rect.x, rect.width, viewport.x, viewport.right());
    rect.y = clamp_to_viewport(rect.y, rect.height, viewport.y, viewport.bottom());
    rect
}

fn clamp_to_viewport(value: f32, extent: f32, min: f32, max: f32) -> f32 {
    let upper = (max - extent).max(min);
    value.clamp(min, upper)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToastId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastSeverity {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastAction {
    pub id: String,
    pub label: String,
}

impl ToastAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub id: ToastId,
    pub severity: ToastSeverity,
    pub title: String,
    pub body: Option<String>,
    pub timeout_seconds: Option<f32>,
    pub age_seconds: f32,
    pub actions: Vec<ToastAction>,
}

impl Toast {
    pub fn new(
        id: ToastId,
        severity: ToastSeverity,
        title: impl Into<String>,
        body: Option<String>,
        timeout_seconds: Option<f32>,
    ) -> Self {
        Self {
            id,
            severity,
            title: title.into(),
            body,
            timeout_seconds,
            age_seconds: 0.0,
            actions: Vec::new(),
        }
    }

    pub fn with_action(mut self, action: ToastAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn expired(&self) -> bool {
        self.timeout_seconds
            .is_some_and(|timeout| self.age_seconds >= timeout)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToastStack {
    pub toasts: Vec<Toast>,
    pub max_visible: usize,
    next_id: u64,
}

impl ToastStack {
    pub fn new(max_visible: usize) -> Self {
        Self {
            toasts: Vec::new(),
            max_visible,
            next_id: 1,
        }
    }

    pub fn push(
        &mut self,
        severity: ToastSeverity,
        title: impl Into<String>,
        body: Option<String>,
        timeout_seconds: Option<f32>,
    ) -> ToastId {
        let id = ToastId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.toasts
            .push(Toast::new(id, severity, title, body, timeout_seconds));
        id
    }

    pub fn push_toast(&mut self, mut toast: Toast) -> ToastId {
        if toast.id.0 == 0 {
            toast.id = ToastId(self.next_id);
            self.next_id = self.next_id.saturating_add(1);
        } else {
            self.next_id = self.next_id.max(toast.id.0.saturating_add(1));
        }
        let id = toast.id;
        self.toasts.push(toast);
        id
    }

    pub fn dismiss(&mut self, id: ToastId) -> Option<Toast> {
        let index = self.toasts.iter().position(|toast| toast.id == id)?;
        Some(self.toasts.remove(index))
    }

    pub fn tick(&mut self, dt_seconds: f32) {
        let dt = dt_seconds.max(0.0);
        for toast in &mut self.toasts {
            toast.age_seconds += dt;
        }
        self.toasts.retain(|toast| !toast.expired());
    }

    pub fn visible(&self) -> &[Toast] {
        let start = self.toasts.len().saturating_sub(self.max_visible);
        &self.toasts[start..]
    }
}

impl Default for ToastStack {
    fn default() -> Self {
        Self::new(4)
    }
}

#[derive(Debug, Clone)]
pub struct ToastStackOptions {
    pub layout: Style,
    pub info_visual: UiVisual,
    pub success_visual: UiVisual,
    pub warning_visual: UiVisual,
    pub error_visual: UiVisual,
    pub action_visual: UiVisual,
    pub title_style: TextStyle,
    pub body_style: TextStyle,
}

impl Default for ToastStackOptions {
    fn default() -> Self {
        Self {
            layout: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: Some(AlignItems::FlexEnd),
                size: TaffySize {
                    width: length(320.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
            info_visual: UiVisual::panel(
                ColorRgba::new(31, 39, 50, 245),
                Some(StrokeStyle::new(DEFAULT_SURFACE_STROKE, 1.0)),
                4.0,
            ),
            success_visual: UiVisual::panel(
                ColorRgba::new(22, 58, 44, 245),
                Some(StrokeStyle::new(ColorRgba::new(74, 160, 118, 255), 1.0)),
                4.0,
            ),
            warning_visual: UiVisual::panel(
                ColorRgba::new(70, 54, 24, 245),
                Some(StrokeStyle::new(ColorRgba::new(190, 148, 62, 255), 1.0)),
                4.0,
            ),
            error_visual: UiVisual::panel(
                ColorRgba::new(73, 31, 35, 245),
                Some(StrokeStyle::new(ColorRgba::new(205, 91, 102, 255), 1.0)),
                4.0,
            ),
            action_visual: UiVisual::panel(
                ColorRgba::new(48, 58, 72, 255),
                Some(StrokeStyle::new(DEFAULT_ACCENT, 1.0)),
                3.0,
            ),
            title_style: TextStyle {
                font_size: 14.0,
                line_height: 18.0,
                ..Default::default()
            },
            body_style: TextStyle {
                font_size: 13.0,
                line_height: 17.0,
                color: ColorRgba::new(218, 226, 238, 255),
                ..Default::default()
            },
        }
    }
}

pub fn toast_stack(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    stack: &ToastStack,
    options: ToastStackOptions,
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: options.layout.clone(),
                z_index: 60,
                ..Default::default()
            },
        ),
    );

    for toast in stack.visible() {
        add_toast_node(document, root, &name, toast, &options);
    }

    root
}

fn add_toast_node(
    document: &mut UiDocument,
    parent: UiNodeId,
    stack_name: &str,
    toast: &Toast,
    options: &ToastStackOptions,
) -> UiNodeId {
    let visual = match toast.severity {
        ToastSeverity::Info => options.info_visual,
        ToastSeverity::Success => options.success_visual,
        ToastSeverity::Warning => options.warning_visual,
        ToastSeverity::Error => options.error_visual,
    };
    let root = document.add_child(
        parent,
        UiNode::container(
            format!("{stack_name}.toast.{}", toast.id.0),
            UiNodeStyle {
                layout: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    padding: Rect::length(8.0),
                    margin: Rect {
                        bottom: LengthPercentageAuto::length(8.0),
                        ..Rect::length(0.0)
                    },
                    ..Default::default()
                },
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(visual),
    );
    document.add_child(
        root,
        UiNode::text(
            format!("{stack_name}.toast.{}.title", toast.id.0),
            toast.title.clone(),
            options.title_style.clone(),
            Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            },
        ),
    );
    if let Some(body) = &toast.body {
        document.add_child(
            root,
            UiNode::text(
                format!("{stack_name}.toast.{}.body", toast.id.0),
                body.clone(),
                options.body_style.clone(),
                Style {
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            ),
        );
    }
    if !toast.actions.is_empty() {
        let actions = document.add_child(
            root,
            UiNode::container(
                format!("{stack_name}.toast.{}.actions", toast.id.0),
                UiNodeStyle {
                    layout: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        size: TaffySize {
                            width: Dimension::percent(1.0),
                            height: Dimension::auto(),
                        },
                        margin: Rect {
                            top: LengthPercentageAuto::length(6.0),
                            ..Rect::length(0.0)
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
        );
        for action in &toast.actions {
            let button = document.add_child(
                actions,
                UiNode::container(
                    format!("{stack_name}.toast.{}.action.{}", toast.id.0, action.id),
                    UiNodeStyle {
                        layout: Style {
                            display: Display::Flex,
                            size: TaffySize {
                                width: Dimension::auto(),
                                height: length(24.0),
                            },
                            padding: Rect::length(6.0),
                            margin: Rect {
                                right: LengthPercentageAuto::length(6.0),
                                ..Rect::length(0.0)
                            },
                            ..Default::default()
                        },
                        clip: ClipBehavior::Clip,
                        ..Default::default()
                    },
                )
                .with_input(InputBehavior::BUTTON)
                .with_visual(options.action_visual),
            );
            document.add_child(
                button,
                UiNode::text(
                    format!(
                        "{stack_name}.toast.{}.action.{}.label",
                        toast.id.0, action.id
                    ),
                    action.label.clone(),
                    options.body_style.clone(),
                    Style {
                        size: TaffySize {
                            width: Dimension::auto(),
                            height: Dimension::auto(),
                        },
                        ..Default::default()
                    },
                ),
            );
        }
    }
    root
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRange {
    pub start: f64,
    pub end: f64,
}

impl TimelineRange {
    pub const fn new(start: f64, end: f64) -> Self {
        Self { start, end }
    }

    pub fn duration(self) -> f64 {
        (self.end - self.start).max(0.0)
    }

    pub fn contains(self, value: f64) -> bool {
        value >= self.start && value <= self.end
    }

    pub fn normalized(self, value: f64) -> f32 {
        let duration = self.duration();
        if duration <= f64::EPSILON {
            return 0.0;
        }
        ((value - self.start) / duration).clamp(0.0, 1.0) as f32
    }

    pub fn value_to_x(self, value: f64, width: f32) -> f32 {
        self.normalized(value) * width.max(0.0)
    }

    pub fn x_to_value(self, x: f32, width: f32) -> f64 {
        let width = width.max(1.0);
        self.start + self.duration() * (x.max(0.0).min(width) as f64 / width as f64)
    }

    pub fn pan(self, delta: f64) -> Self {
        Self {
            start: self.start + delta,
            end: self.end + delta,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulerTickKind {
    Major,
    Minor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RulerTick {
    pub value: f64,
    pub x: f32,
    pub kind: RulerTickKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RulerSpec {
    pub range: TimelineRange,
    pub width: f32,
    pub major_step: f64,
    pub minor_step: f64,
    pub label_every: usize,
}

impl RulerSpec {
    pub fn ticks(self) -> Vec<RulerTick> {
        if self.range.duration() <= f64::EPSILON
            || self.width <= f32::EPSILON
            || self.major_step <= f64::EPSILON
            || self.minor_step <= f64::EPSILON
        {
            return Vec::new();
        }
        let start_index = (self.range.start / self.minor_step).ceil() as i64;
        let end_index = (self.range.end / self.minor_step).floor() as i64;
        let label_every = self.label_every.max(1);
        let mut major_count = 0_usize;
        let mut ticks = Vec::new();
        for index in start_index..=end_index {
            if ticks.len() >= 10_000 {
                break;
            }
            let value = index as f64 * self.minor_step;
            let major_ratio = value / self.major_step;
            let is_major = (major_ratio - major_ratio.round()).abs() < 0.000_001;
            let label = if is_major {
                let should_label = major_count % label_every == 0;
                major_count += 1;
                should_label.then(|| format_ruler_label(value))
            } else {
                None
            };
            ticks.push(RulerTick {
                value,
                x: self.range.value_to_x(value, self.width),
                kind: if is_major {
                    RulerTickKind::Major
                } else {
                    RulerTickKind::Minor
                },
                label,
            });
        }
        ticks
    }
}

fn format_ruler_label(value: f64) -> String {
    if value.fract().abs() < 0.000_001 {
        return format!("{}", value.round() as i64);
    }
    let mut label = format!("{value:.3}");
    while label.contains('.') && label.ends_with('0') {
        label.pop();
    }
    if label.ends_with('.') {
        label.pop();
    }
    label
}

#[derive(Debug, Clone)]
pub struct TimelineRulerOptions {
    pub layout: Style,
    pub height: f32,
    pub background_visual: UiVisual,
    pub major_stroke: StrokeStyle,
    pub minor_stroke: StrokeStyle,
    pub label_style: TextStyle,
}

impl Default for TimelineRulerOptions {
    fn default() -> Self {
        Self {
            layout: Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: length(32.0),
                },
                ..Default::default()
            },
            height: 32.0,
            background_visual: UiVisual::panel(
                ColorRgba::new(20, 24, 30, 255),
                Some(StrokeStyle::new(DEFAULT_SURFACE_STROKE, 1.0)),
                0.0,
            ),
            major_stroke: StrokeStyle::new(ColorRgba::new(180, 190, 205, 255), 1.0),
            minor_stroke: StrokeStyle::new(ColorRgba::new(86, 98, 116, 255), 1.0),
            label_style: TextStyle {
                font_size: 11.0,
                line_height: 14.0,
                color: ColorRgba::new(218, 226, 238, 255),
                ..Default::default()
            },
        }
    }
}

pub fn timeline_ruler(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    spec: RulerSpec,
    options: TimelineRulerOptions,
) -> UiNodeId {
    let name = name.into();
    let mut layout = options.layout;
    layout.size.height = length(options.height);
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(options.background_visual),
    );

    let ticks = spec.ticks();
    let primitives = ticks
        .iter()
        .map(|tick| {
            let height = match tick.kind {
                RulerTickKind::Major => options.height,
                RulerTickKind::Minor => options.height * 0.5,
            };
            ScenePrimitive::Line {
                from: UiPoint::new(tick.x, options.height),
                to: UiPoint::new(tick.x, options.height - height),
                stroke: match tick.kind {
                    RulerTickKind::Major => options.major_stroke,
                    RulerTickKind::Minor => options.minor_stroke,
                },
            }
        })
        .collect::<Vec<_>>();
    document.add_child(
        root,
        UiNode::scene(
            format!("{name}.ticks"),
            primitives,
            Style {
                position: Position::Absolute,
                size: TaffySize {
                    width: length(spec.width),
                    height: length(options.height),
                },
                ..Default::default()
            },
        ),
    );

    for tick in ticks.iter().filter(|tick| tick.label.is_some()) {
        let mut inset = Rect::length(0.0);
        inset.left = LengthPercentageAuto::length(tick.x + 3.0);
        inset.top = LengthPercentageAuto::length(2.0);
        document.add_child(
            root,
            UiNode::text(
                format!("{name}.label.{}", tick.value),
                tick.label.clone().unwrap_or_default(),
                options.label_style.clone(),
                Style {
                    position: Position::Absolute,
                    inset,
                    size: TaffySize {
                        width: length(64.0),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                },
            ),
        );
    }

    root
}

#[cfg(test)]
mod tests {
    use crate::{root_style, ApproxTextMeasurer, TextContent, UiContent};

    use super::*;

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
                        Style {
                            size: TaffySize {
                                width: Dimension::auto(),
                                height: Dimension::auto(),
                            },
                            ..Default::default()
                        },
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
                        Style {
                            size: TaffySize {
                                width: Dimension::auto(),
                                height: Dimension::auto(),
                            },
                            ..Default::default()
                        },
                    ),
                );
            },
        );
        doc.compute_layout(UiSize::new(400.0, 200.0), &mut ApproxTextMeasurer)
            .expect("layout");

        assert!(doc.node(nodes.handle).input.focusable);
        assert!(doc.node(nodes.first).layout.rect.width >= state.min_first);
        assert_eq!(doc.node(nodes.root).children.len(), 3);
    }

    #[test]
    fn dock_workspace_builds_visible_panels_and_center() {
        let panels = vec![
            DockPanelDescriptor::new("top", "Toolbar", DockSide::Top, 40.0),
            DockPanelDescriptor::new("left", "Browser", DockSide::Left, 120.0).resizable(true),
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
                        Style {
                            size: TaffySize {
                                width: Dimension::auto(),
                                height: Dimension::auto(),
                            },
                            ..Default::default()
                        },
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
    }

    #[test]
    fn dialog_and_popover_state_track_dismissal_rules() {
        let mut dialogs = DialogStack::default();
        dialogs.open(DialogDescriptor::new("settings", "Settings").modal(true));
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
        );
        popovers.toggle(popover.clone());
        assert!(popovers.is_open("tools"));
        popovers.toggle(popover);
        assert!(!popovers.is_open("tools"));

        let rect = resolve_popover_rect(
            UiRect::new(180.0, 180.0, 20.0, 20.0),
            UiSize::new(80.0, 50.0),
            UiRect::new(0.0, 0.0, 220.0, 220.0),
            PopoverPlacement::Bottom,
            6.0,
        );
        assert_eq!(rect.x, 140.0);
        assert_eq!(rect.y, 170.0);
    }

    #[test]
    fn toast_stack_expires_limits_and_builds_action_nodes() {
        let mut stack = ToastStack::new(2);
        stack.push(ToastSeverity::Info, "One", None, Some(1.0));
        stack.push(ToastSeverity::Success, "Two", None, None);
        let action_toast = Toast::new(
            ToastId(99),
            ToastSeverity::Warning,
            "Three",
            Some("Body".to_string()),
            None,
        )
        .with_action(ToastAction::new("retry", "Retry"));
        stack.push_toast(action_toast);

        assert_eq!(
            stack
                .visible()
                .iter()
                .map(|toast| toast.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Two", "Three"]
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
    }

    #[test]
    fn timeline_range_and_ruler_ticks_are_renderer_neutral() {
        let range = TimelineRange::new(10.0, 14.0);
        assert_eq!(range.value_to_x(12.0, 400.0), 200.0);
        assert_eq!(range.x_to_value(100.0, 400.0), 11.0);

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

        let mut doc = UiDocument::new(root_style(400.0, 80.0));
        let root = doc.root;
        let ruler = timeline_ruler(
            &mut doc,
            root,
            "ruler",
            spec,
            TimelineRulerOptions::default(),
        );
        doc.compute_layout(UiSize::new(400.0, 80.0), &mut ApproxTextMeasurer)
            .expect("layout");
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
    }
}
