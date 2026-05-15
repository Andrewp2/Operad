//! Menu list widget implementations.

use taffy::prelude::{
    Dimension, Display, FlexDirection, JustifyContent, Rect as TaffyRect, Size as TaffySize, Style,
};

use crate::{
    length, AccessibilityMeta, AccessibilityRole, AnimationMachine, ClipBehavior, ColorRgba,
    InputBehavior, LayoutStyle, ScrollAxes, ShaderEffect, StrokeStyle, TextStyle, UiDocument,
    UiNode, UiNodeId, UiNodeStyle, UiSize, UiVisual, WidgetActionBinding,
};

use super::menu::{
    label, leading_image, length_percentage, menu_accessibility_label, place_popup, popup_panel,
    row_style, set_active_descendant, AnchoredPopup, MenuItem, MenuItemKind, PopupOptions,
};

#[derive(Debug, Clone)]
pub struct MenuListOptions {
    pub width: f32,
    pub row_height: f32,
    pub separator_height: f32,
    pub max_visible_rows: usize,
    pub menu_visual: UiVisual,
    pub item_visual: UiVisual,
    pub active_visual: UiVisual,
    pub disabled_visual: UiVisual,
    pub text_style: TextStyle,
    pub disabled_text_style: TextStyle,
    pub shortcut_text_style: TextStyle,
    pub destructive_text_style: TextStyle,
    pub image_size: UiSize,
    pub accessibility_label: Option<String>,
    pub menu_shader: Option<ShaderEffect>,
    pub active_shader: Option<ShaderEffect>,
    pub menu_animation: Option<AnimationMachine>,
    pub z_index: i16,
    pub action_prefix: Option<String>,
}

impl Default for MenuListOptions {
    fn default() -> Self {
        Self {
            width: 240.0,
            row_height: 28.0,
            separator_height: 8.0,
            max_visible_rows: 12,
            menu_visual: UiVisual::panel(
                ColorRgba::new(26, 31, 39, 255),
                Some(StrokeStyle::new(ColorRgba::new(77, 90, 111, 255), 1.0)),
                4.0,
            ),
            item_visual: UiVisual::TRANSPARENT,
            active_visual: UiVisual::panel(ColorRgba::new(58, 87, 126, 255), None, 2.0),
            disabled_visual: UiVisual::TRANSPARENT,
            text_style: TextStyle::default(),
            disabled_text_style: TextStyle {
                color: ColorRgba::new(138, 148, 164, 255),
                ..Default::default()
            },
            shortcut_text_style: TextStyle {
                color: ColorRgba::new(178, 188, 204, 255),
                ..Default::default()
            },
            destructive_text_style: TextStyle {
                color: ColorRgba::new(238, 116, 106, 255),
                ..Default::default()
            },
            image_size: UiSize::new(18.0, 18.0),
            accessibility_label: None,
            menu_shader: None,
            active_shader: None,
            menu_animation: None,
            z_index: 100,
            action_prefix: None,
        }
    }
}

impl MenuListOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuListNodes {
    pub root: UiNodeId,
    pub rows: Vec<UiNodeId>,
}

pub fn menu_list(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    items: &[MenuItem],
    active: Option<usize>,
    options: MenuListOptions,
) -> MenuListNodes {
    let name = name.into();
    let root = document.add_child(
        parent,
        menu_list_container_node(name.clone(), items, &options),
    );
    let rows = populate_menu_list(document, root, &name, items, active, &options);
    set_active_descendant(document, root, active_menu_row(items, &rows, active));
    MenuListNodes { root, rows }
}

pub fn menu_list_popup(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    popup: AnchoredPopup,
    items: &[MenuItem],
    active: Option<usize>,
    options: MenuListOptions,
) -> MenuListNodes {
    let name = name.into();
    let height = visible_menu_height(items, &options);
    let layout = place_popup(
        popup.anchor,
        UiSize::new(options.width.max(0.0), height.max(0.0)),
        popup.viewport,
        popup.placement,
    );
    let root = popup_panel(
        document,
        parent,
        name.clone(),
        layout.rect,
        PopupOptions {
            visual: options.menu_visual,
            z_index: options.z_index,
            scroll_axes: if menu_row_count_for_scroll(items) > options.max_visible_rows {
                ScrollAxes::VERTICAL
            } else {
                ScrollAxes::NONE
            },
            accessibility: Some(AccessibilityMeta::new(AccessibilityRole::Menu).label(
                menu_accessibility_label(&name, options.accessibility_label.as_ref()),
            )),
            shader: options.menu_shader.clone(),
            animation: options.menu_animation.clone(),
            ..Default::default()
        },
    );
    {
        let layout = &mut document.node_mut(root).style.layout;
        layout.display = Display::Flex;
        layout.flex_direction = FlexDirection::Column;
    }
    let rows = populate_menu_list(document, root, &name, items, active, &options);
    set_active_descendant(document, root, active_menu_row(items, &rows, active));
    MenuListNodes { root, rows }
}

fn menu_list_container_node(
    name: impl Into<String>,
    items: &[MenuItem],
    options: &MenuListOptions,
) -> UiNode {
    let name = name.into();
    let scroll = menu_row_count_for_scroll(items) > options.max_visible_rows;
    let height = visible_menu_height(items, options);
    let mut node = UiNode::container(
        name.clone(),
        UiNodeStyle {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: length(options.width.max(0.0)),
                    height: length(height.max(0.0)),
                },
                ..Default::default()
            })
            .style,
            clip: ClipBehavior::Clip,
            z_index: options.z_index,
            ..Default::default()
        },
    )
    .with_visual(options.menu_visual)
    .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Menu).label(
        menu_accessibility_label(&name, options.accessibility_label.as_ref()),
    ));
    if scroll {
        node = node.with_scroll(ScrollAxes::VERTICAL);
    }
    if let Some(shader) = options.menu_shader.clone() {
        node = node.with_shader(shader);
    }
    if let Some(animation) = options.menu_animation.clone() {
        node = node.with_animation(animation);
    }
    node
}

fn populate_menu_list(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    items: &[MenuItem],
    active: Option<usize>,
    options: &MenuListOptions,
) -> Vec<UiNodeId> {
    let mut rows = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        if item.is_separator() {
            rows.push(separator_row(document, parent, name, index, options));
            continue;
        }

        let text_style = menu_item_text_style(item, options);
        let row = document.add_child(
            parent,
            menu_item_row_node(
                format!("{name}.item.{index}"),
                item,
                active == Some(index),
                options,
            ),
        );
        if item.enabled {
            if let (Some(prefix), Some(id)) = (&options.action_prefix, &item.id) {
                document.node_mut(row).action =
                    Some(WidgetActionBinding::action(format!("{prefix}.{id}")));
            }
        }
        if let Some(image) = &item.image {
            leading_image(
                document,
                row,
                format!("{name}.item.{index}.image"),
                image.clone(),
                &menu_item_accessibility_label(item),
                options.image_size,
            );
        }
        label(
            document,
            row,
            format!("{name}.item.{index}.label"),
            menu_item_label(item),
            text_style,
            LayoutStyle::from_taffy_style(Style {
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
        );
        if let MenuItemKind::Check { checked } = &item.kind {
            label(
                document,
                row,
                format!("{name}.item.{index}.check"),
                if *checked { "x" } else { "" },
                options.shortcut_text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: length(18.0),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            );
        } else if let Some(shortcut) = &item.shortcut {
            label(
                document,
                row,
                format!("{name}.item.{index}.shortcut"),
                shortcut,
                options.shortcut_text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            );
        } else if item.children().is_some() {
            label(
                document,
                row,
                format!("{name}.item.{index}.submenu"),
                ">",
                options.shortcut_text_style.clone(),
                LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::auto(),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            );
        }
        rows.push(row);
    }
    rows
}

fn active_menu_row(
    items: &[MenuItem],
    rows: &[UiNodeId],
    active: Option<usize>,
) -> Option<UiNodeId> {
    active
        .filter(|index| items.get(*index).is_some_and(MenuItem::is_navigable))
        .and_then(|index| rows.get(index).copied())
}

fn menu_item_row_node(
    name: impl Into<String>,
    item: &MenuItem,
    active: bool,
    options: &MenuListOptions,
) -> UiNode {
    let visual = if item.enabled {
        if active {
            options.active_visual
        } else {
            options.item_visual
        }
    } else {
        options.disabled_visual
    };
    let mut node = UiNode::container(name, row_style(options.row_height))
        .with_input(if item.enabled {
            InputBehavior::BUTTON
        } else {
            InputBehavior::NONE
        })
        .with_visual(visual)
        .with_accessibility(menu_item_accessibility(item, active));
    if active {
        if let Some(shader) = options.active_shader.clone() {
            node = node.with_shader(shader);
        }
    }
    node
}

fn menu_item_text_style(item: &MenuItem, options: &MenuListOptions) -> TextStyle {
    if !item.enabled {
        options.disabled_text_style.clone()
    } else if item.destructive {
        options.destructive_text_style.clone()
    } else {
        options.text_style.clone()
    }
}

fn separator_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    index: usize,
    options: &MenuListOptions,
) -> UiNodeId {
    let row = document.add_child(
        parent,
        UiNode::container(
            format!("{name}.separator.{index}"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    justify_content: Some(JustifyContent::Center),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(options.separator_height.max(0.0)),
                    },
                    flex_shrink: 0.0,
                    padding: TaffyRect {
                        left: length_percentage(8.0),
                        right: length_percentage(8.0),
                        top: length_percentage(0.0),
                        bottom: length_percentage(0.0),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    document.add_child(
        row,
        UiNode::container(
            format!("{name}.separator.{index}.line"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: length(1.0),
                    },
                    ..Default::default()
                })
                .style,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(ColorRgba::new(77, 90, 111, 255), None, 0.0)),
    );
    row
}

fn menu_item_label(item: &MenuItem) -> String {
    match &item.kind {
        MenuItemKind::Command | MenuItemKind::Submenu { .. } => item.label.clone(),
        MenuItemKind::Check { .. } | MenuItemKind::Separator => item.label.clone(),
    }
}

fn menu_row_count_for_scroll(items: &[MenuItem]) -> usize {
    items.len()
}

fn visible_menu_height(items: &[MenuItem], options: &MenuListOptions) -> f32 {
    let mut height = 0.0;
    for item in items.iter().take(options.max_visible_rows) {
        height += if item.is_separator() {
            options.separator_height
        } else {
            options.row_height
        };
    }
    height
}

fn menu_item_accessibility_label(item: &MenuItem) -> String {
    item.accessibility_label
        .clone()
        .unwrap_or_else(|| item.label.clone())
}

fn menu_item_accessibility(item: &MenuItem, active: bool) -> AccessibilityMeta {
    let mut accessibility = AccessibilityMeta::new(AccessibilityRole::MenuItem)
        .label(menu_item_accessibility_label(item))
        .selected(active);
    if let MenuItemKind::Check { checked } = &item.kind {
        accessibility = accessibility
            .value(if *checked { "checked" } else { "unchecked" })
            .checked(*checked);
    }
    if item.children().is_some() {
        accessibility = accessibility.hint("Opens submenu").expanded(active);
    } else if item.destructive {
        accessibility = accessibility.hint("Destructive action");
    } else if let Some(shortcut) = &item.shortcut {
        accessibility = accessibility
            .hint(format!("Shortcut {shortcut}"))
            .shortcut(shortcut.clone());
    } else if active {
        accessibility = accessibility.hint("Active menu item");
    }
    if item.enabled {
        accessibility.focusable()
    } else {
        accessibility.disabled()
    }
}
