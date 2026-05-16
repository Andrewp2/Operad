//! Tab group widget implementation.

use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentageAuto,
    Size as TaffySize, Style,
};

use crate::widgets::{
    inline_intrinsic_base_size, publish_inline_intrinsic_size, single_line_text_style,
};
use crate::{
    AccessibilityMeta, AccessibilityRole, ClipBehavior, ColorRgba, ImageContent, InputBehavior,
    LayoutStyle, ShaderEffect, StrokeStyle, TextStyle, TextWrap, UiDocument, UiNode, UiNodeId,
    UiNodeStyle, UiVisual,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub disabled: bool,
    pub closable: bool,
    pub dirty: bool,
    pub leading_image: Option<ImageContent>,
}

impl TabItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            disabled: false,
            closable: false,
            dirty: false,
            leading_image: None,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }

    pub fn dirty(mut self) -> Self {
        self.dirty = true;
        self
    }

    pub fn with_leading_image(mut self, image: ImageContent) -> Self {
        self.leading_image = Some(image);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TabGroupState {
    pub selected_index: Option<usize>,
    pub focused_index: Option<usize>,
}

impl TabGroupState {
    pub const fn selected(selected_index: usize) -> Self {
        Self {
            selected_index: Some(selected_index),
            focused_index: Some(selected_index),
        }
    }

    pub fn clamped_selected_index(self, tabs: &[TabItem]) -> Option<usize> {
        let selected = self.selected_index?;
        (selected < tabs.len()).then_some(selected)
    }

    pub fn selected_tab(self, tabs: &[TabItem]) -> Option<&TabItem> {
        tabs.get(self.clamped_selected_index(tabs)?)
    }

    pub fn selected_tab_id(self, tabs: &[TabItem]) -> Option<&str> {
        Some(self.selected_tab(tabs)?.id.as_str())
    }

    pub fn clamped_focused_index(self, tabs: &[TabItem]) -> Option<usize> {
        let focused = self.focused_index?;
        (focused < tabs.len()).then_some(focused)
    }

    pub fn focus_next(&mut self, tabs: &[TabItem]) -> Option<usize> {
        let index = next_enabled_tab_index(tabs, self.focused_index.or(self.selected_index))?;
        self.focused_index = Some(index);
        Some(index)
    }

    pub fn focus_previous(&mut self, tabs: &[TabItem]) -> Option<usize> {
        let index = previous_enabled_tab_index(tabs, self.focused_index.or(self.selected_index))?;
        self.focused_index = Some(index);
        Some(index)
    }

    pub fn select_focused(&mut self, tabs: &[TabItem]) -> Option<usize> {
        let focused = self.clamped_focused_index(tabs)?;
        if tabs[focused].disabled {
            return None;
        }
        self.selected_index = Some(focused);
        Some(focused)
    }

    pub fn select_next(&mut self, tabs: &[TabItem]) -> Option<usize> {
        if tabs.is_empty() {
            self.selected_index = None;
            self.focused_index = None;
            return None;
        }
        let index = self.focus_next(tabs)?;
        self.selected_index = Some(index);
        Some(index)
    }

    pub fn select_previous(&mut self, tabs: &[TabItem]) -> Option<usize> {
        if tabs.is_empty() {
            self.selected_index = None;
            self.focused_index = None;
            return None;
        }
        let index = self.focus_previous(tabs)?;
        self.selected_index = Some(index);
        Some(index)
    }
}

#[derive(Debug, Clone)]
pub struct TabGroupOptions {
    pub layout: LayoutStyle,
    pub tab_strip_height: f32,
    pub min_tab_width: f32,
    pub background_visual: UiVisual,
    pub tab_visual: UiVisual,
    pub selected_tab_visual: UiVisual,
    pub panel_visual: UiVisual,
    pub selected_tab_shader: Option<ShaderEffect>,
    pub focused_tab_shader: Option<ShaderEffect>,
    pub panel_shader: Option<ShaderEffect>,
    pub text_style: TextStyle,
    pub muted_text_style: TextStyle,
    pub leading_image_size: f32,
    pub accessibility_label: Option<String>,
}

impl Default for TabGroupOptions {
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
            tab_strip_height: 32.0,
            min_tab_width: 96.0,
            background_visual: UiVisual::panel(
                ColorRgba::new(16, 20, 26, 255),
                Some(StrokeStyle::new(ColorRgba::new(58, 69, 84, 255), 1.0)),
                4.0,
            ),
            tab_visual: UiVisual::panel(ColorRgba::new(28, 34, 43, 255), None, 0.0),
            selected_tab_visual: UiVisual::panel(ColorRgba::new(43, 52, 65, 255), None, 0.0),
            panel_visual: UiVisual::TRANSPARENT,
            selected_tab_shader: None,
            focused_tab_shader: None,
            panel_shader: None,
            text_style: TextStyle::default(),
            muted_text_style: muted_text_style(),
            leading_image_size: 16.0,
            accessibility_label: None,
        }
    }
}

/// Build a tab group and call `build_panel` for the selected tab index.
pub fn tab_group(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    tabs: &[TabItem],
    state: TabGroupState,
    options: TabGroupOptions,
    mut build_panel: impl FnMut(&mut UiDocument, UiNodeId, usize),
) -> UiNodeId {
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
        .with_visual(options.background_visual),
    );

    let strip = document.add_child(
        root,
        UiNode::container(
            format!("{name}.strip"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: px(options.tab_strip_height),
                    },
                    ..Default::default()
                })
                .style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::TabList)
                .label(accessibility_label_or_name(
                    &options.accessibility_label,
                    &name,
                ))
                .value(format!("{} tabs", tabs.len()))
                .focusable(),
        ),
    );

    let selected_index = state.clamped_selected_index(tabs);
    let focused_index = state.clamped_focused_index(tabs);
    for (index, tab) in tabs.iter().enumerate() {
        let selected = selected_index == Some(index);
        let focused = focused_index == Some(index);
        let style = single_line_text_style(if tab.disabled {
            options.muted_text_style.clone()
        } else {
            options.text_style.clone()
        });
        let tab_layout = Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            justify_content: Some(JustifyContent::Center),
            size: TaffySize {
                width: px(options.min_tab_width),
                height: Dimension::percent(1.0),
            },
            padding: taffy::prelude::Rect::length(6.0),
            flex_shrink: 0.0,
            ..Default::default()
        };
        let tab_node = with_optional_shader(
            UiNode::container(
                format!("{name}.tab.{}", tab.id),
                UiNodeStyle {
                    layout: LayoutStyle::from_taffy_style(tab_layout.clone()).style,
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_input(if tab.disabled {
                InputBehavior::NONE
            } else {
                InputBehavior::BUTTON
            })
            .with_visual(if selected {
                options.selected_tab_visual
            } else {
                options.tab_visual
            })
            .with_accessibility(tab_accessibility(
                tab,
                index,
                tabs.len(),
                selected,
                focused,
            )),
            if selected {
                options.selected_tab_shader.as_ref()
            } else if focused {
                options.focused_tab_shader.as_ref()
            } else {
                None
            },
        );
        let tab_node = document.add_child(strip, tab_node);

        let label = if tab.dirty {
            format!("{} *", tab.label)
        } else {
            tab.label.clone()
        };
        let mut fixed_items = Vec::<Style>::new();
        if let Some(image) = tab.leading_image.clone() {
            let image_layout = leading_image_layout(options.leading_image_size);
            fixed_items.push(image_layout.clone());
            document.add_child(
                tab_node,
                leading_image_node(
                    format!("{name}.tab.{}.image", tab.id),
                    image,
                    image_layout,
                    Some(tab.label.clone()),
                ),
            );
        }
        let label_node = document.add_child(
            tab_node,
            UiNode::text(
                format!("{name}.tab.{}.label", tab.id),
                label,
                style,
                LayoutStyle::from_taffy_style(Style {
                    flex_grow: 1.0,
                    size: TaffySize {
                        width: Dimension::percent(1.0),
                        height: Dimension::auto(),
                    },
                    ..Default::default()
                }),
            ),
        );

        if tab.closable {
            let close_layout = Style {
                size: TaffySize {
                    width: px(16.0),
                    height: Dimension::percent(1.0),
                },
                ..Default::default()
            };
            fixed_items.push(close_layout.clone());
            document.add_child(
                tab_node,
                UiNode::text(
                    format!("{name}.tab.{}.close", tab.id),
                    "x",
                    single_line_text_style(options.muted_text_style.clone()),
                    LayoutStyle::from_taffy_style(close_layout),
                )
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::Button)
                        .label(format!("Close {}", tab.label))
                        .focusable(),
                ),
            );
        }
        let fixed_items = fixed_items.iter().collect::<Vec<_>>();
        publish_inline_intrinsic_size(
            document,
            tab_node,
            vec![label_node],
            inline_intrinsic_base_size(&tab_layout, &fixed_items, fixed_items.len() + 1),
        );
    }

    let panel = with_optional_shader(
        UiNode::container(
            format!("{name}.panel"),
            UiNodeStyle {
                layout: LayoutStyle::from_taffy_style(Style {
                    flex_grow: 1.0,
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
        )
        .with_visual(options.panel_visual)
        .with_accessibility(tab_panel_accessibility(tabs, selected_index, &name)),
        options.panel_shader.as_ref(),
    );
    let panel = document.add_child(root, panel);

    if let Some(index) = selected_index {
        build_panel(document, panel, index);
    }

    root
}

fn tab_accessibility(
    tab: &TabItem,
    index: usize,
    tab_count: usize,
    selected: bool,
    focused: bool,
) -> AccessibilityMeta {
    let mut value = vec![format!("tab {} of {}", index + 1, tab_count)];
    push_state(&mut value, "selected", selected);
    push_state(&mut value, "focused", focused);
    push_state(&mut value, "dirty", tab.dirty);
    push_state(&mut value, "closable", tab.closable);
    push_state(&mut value, "disabled", tab.disabled);

    apply_enabled(
        AccessibilityMeta::new(AccessibilityRole::Tab)
            .label(tab.label.clone())
            .value(value.join("; "))
            .selected(selected)
            .focusable(),
        !tab.disabled,
    )
}

fn tab_panel_accessibility(
    tabs: &[TabItem],
    selected_index: Option<usize>,
    group_name: &str,
) -> AccessibilityMeta {
    let selected = selected_index.and_then(|index| tabs.get(index));
    let label = selected
        .map(|tab| format!("{} panel", tab.label))
        .unwrap_or_else(|| format!("{group_name} panel"));
    let value = selected
        .map(|tab| format!("selected tab {}", tab.id))
        .unwrap_or_else(|| "no selected tab".to_owned());
    AccessibilityMeta::new(AccessibilityRole::TabPanel)
        .label(label)
        .value(value)
}

fn next_enabled_tab_index(tabs: &[TabItem], current: Option<usize>) -> Option<usize> {
    if tabs.is_empty() {
        return None;
    }
    let start = current
        .map(|index| (index.min(tabs.len() - 1) + 1) % tabs.len())
        .unwrap_or(0);
    for offset in 0..tabs.len() {
        let index = (start + offset) % tabs.len();
        if !tabs[index].disabled {
            return Some(index);
        }
    }
    None
}

fn previous_enabled_tab_index(tabs: &[TabItem], current: Option<usize>) -> Option<usize> {
    if tabs.is_empty() {
        return None;
    }
    let start = current
        .map(|index| (index.min(tabs.len() - 1) + tabs.len() - 1) % tabs.len())
        .unwrap_or(tabs.len() - 1);
    for offset in 0..tabs.len() {
        let index = (start + tabs.len() - offset) % tabs.len();
        if !tabs[index].disabled {
            return Some(index);
        }
    }
    None
}

fn leading_image_node(
    name: impl Into<String>,
    image: ImageContent,
    layout: Style,
    label: Option<String>,
) -> UiNode {
    let node = UiNode::image(name, image, LayoutStyle::from_taffy_style(layout));
    if let Some(label) = label {
        node.with_accessibility(AccessibilityMeta::new(AccessibilityRole::Image).label(label))
    } else {
        node
    }
}

fn leading_image_layout(size: f32) -> Style {
    Style {
        size: TaffySize {
            width: px(size),
            height: px(size),
        },
        margin: taffy::prelude::Rect {
            right: LengthPercentageAuto::length(6.0),
            ..taffy::prelude::Rect::length(0.0)
        },
        flex_shrink: 0.0,
        ..Default::default()
    }
}

fn with_optional_shader(mut node: UiNode, shader: Option<&ShaderEffect>) -> UiNode {
    if let Some(shader) = shader {
        node = node.with_shader(shader.clone());
    }
    node
}

fn accessibility_label_or_name(label: &Option<String>, name: &str) -> String {
    label.clone().unwrap_or_else(|| name.to_owned())
}

fn apply_enabled(meta: AccessibilityMeta, enabled: bool) -> AccessibilityMeta {
    if enabled {
        meta
    } else {
        meta.disabled()
    }
}

fn push_state(values: &mut Vec<String>, label: &str, active: bool) {
    if active {
        values.push(label.to_owned());
    }
}

fn muted_text_style() -> TextStyle {
    TextStyle {
        color: ColorRgba::new(151, 162, 178, 255),
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn px(value: f32) -> Dimension {
    Dimension::length(value.max(0.0))
}
