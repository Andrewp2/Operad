use taffy::prelude::{Dimension, Display, Size as TaffySize, Style};

use crate::scrolling::{reveal_rect_into_view, RevealOptions, RevealScrollPlan};

use super::*;

#[derive(Debug, Clone)]
pub struct AllocationOptions {
    pub visual: UiVisual,
    pub clip: ClipBehavior,
    pub accessibility_label: Option<String>,
}

impl AllocationOptions {
    pub fn with_visual(mut self, visual: UiVisual) -> Self {
        self.visual = visual;
        self
    }

    pub fn with_clip(mut self, clip: ClipBehavior) -> Self {
        self.clip = clip;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl Default for AllocationOptions {
    fn default() -> Self {
        Self {
            visual: UiVisual::TRANSPARENT,
            clip: ClipBehavior::Clip,
            accessibility_label: None,
        }
    }
}

pub fn add_visible<T>(
    document: &mut UiDocument,
    parent: UiNodeId,
    visible: bool,
    build: impl FnOnce(&mut UiDocument, UiNodeId) -> T,
) -> Option<T> {
    visible.then(|| build(document, parent))
}

pub fn add_visible_ui(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    visible: bool,
    build: impl FnOnce(&mut UiDocument, UiNodeId),
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: LayoutStyle::column().with_width_percent(1.0).style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(format!("{name} visible block")),
        ),
    );
    build(document, root);
    if !visible {
        set_subtree_visible(document, root, false);
    }
    root
}

pub fn add_enabled<T>(
    document: &mut UiDocument,
    parent: UiNodeId,
    enabled: bool,
    build: impl FnOnce(&mut UiDocument, UiNodeId) -> T,
) -> T {
    let existing_len = document.node(parent).children.len();
    let result = build(document, parent);
    if !enabled {
        let added = document.node(parent).children[existing_len..].to_vec();
        for child in added {
            set_subtree_enabled(document, child, false);
        }
    }
    result
}

pub fn add_enabled_ui(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    enabled: bool,
    build: impl FnOnce(&mut UiDocument, UiNodeId),
) -> UiNodeId {
    let name = name.into();
    let root = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: LayoutStyle::column().with_width_percent(1.0).style,
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(format!("{name} enabled block")),
        ),
    );
    build(document, root);
    if !enabled {
        set_subtree_enabled(document, root, false);
    }
    root
}

pub fn set_subtree_enabled(document: &mut UiDocument, root: UiNodeId, enabled: bool) {
    if enabled {
        return;
    }
    let children = document.node(root).children.clone();
    {
        let node = document.node_mut(root);
        node.input = InputBehavior::NONE;
        if let Some(accessibility) = node.accessibility.take() {
            node.accessibility = Some(accessibility.disabled());
        }
        if let Some(visuals) = node.interaction_visuals {
            node.visual = visuals.resolve(false, false, false, false);
        }
    }
    for child in children {
        set_subtree_enabled(document, child, enabled);
    }
}

pub fn set_subtree_visible(document: &mut UiDocument, root: UiNodeId, visible: bool) {
    if visible {
        return;
    }
    let children = document.node(root).children.clone();
    {
        let node = document.node_mut(root);
        node.style.layout.display = Display::None;
        node.input = InputBehavior::NONE;
        if let Some(accessibility) = node.accessibility.take() {
            node.accessibility = Some(accessibility.hidden());
        }
    }
    for child in children {
        set_subtree_visible(document, child, visible);
    }
}

pub fn allocate_exact_size(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    size: UiSize,
    options: AllocationOptions,
) -> UiNodeId {
    allocate_node(
        document,
        parent,
        name,
        LayoutStyle::size(size.width.max(0.0), size.height.max(0.0)),
        options,
    )
}

pub fn allocate_at_least(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    min_size: UiSize,
    options: AllocationOptions,
) -> UiNodeId {
    allocate_node(
        document,
        parent,
        name,
        LayoutStyle::from_taffy_style(Style {
            size: TaffySize {
                width: Dimension::auto(),
                height: Dimension::auto(),
            },
            min_size: TaffySize {
                width: length(min_size.width.max(0.0)),
                height: length(min_size.height.max(0.0)),
            },
            ..Default::default()
        }),
        options,
    )
}

pub fn add_sized(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    size: UiSize,
    options: AllocationOptions,
    build: impl FnOnce(&mut UiDocument, UiNodeId),
) -> UiNodeId {
    let root = allocate_exact_size(document, parent, name, size, options);
    build(document, root);
    root
}

pub fn allocate_painter(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    size: UiSize,
    primitives: impl Into<Vec<ScenePrimitive>>,
    options: SceneOptions,
) -> UiNodeId {
    scene(
        document,
        parent,
        name,
        primitives,
        options.with_layout(LayoutStyle::size(size.width.max(0.0), size.height.max(0.0))),
    )
}

pub fn scroll_to_cursor(
    document: &mut UiDocument,
    scroll_node: UiNodeId,
    cursor_rect: UiRect,
) -> bool {
    scroll_to_rect(document, scroll_node, cursor_rect)
}

pub fn scroll_to_rect(
    document: &mut UiDocument,
    scroll_node: UiNodeId,
    target_rect: UiRect,
) -> bool {
    document.scroll_rect_into_view(scroll_node, target_rect)
}

pub fn scroll_to_rect_with_options(
    document: &mut UiDocument,
    scroll_node: UiNodeId,
    target_rect: UiRect,
    options: RevealOptions,
) -> Option<RevealScrollPlan> {
    let scroll = document.scroll_state(scroll_node)?;
    let plan = reveal_rect_into_view(scroll, target_rect, options);
    if plan.changed() {
        document.set_scroll_offset(scroll_node, plan.offset_after);
    }
    Some(plan)
}

fn allocate_node(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    layout: LayoutStyle,
    options: AllocationOptions,
) -> UiNodeId {
    let name = name.into();
    document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            UiNodeStyle {
                layout: layout.style,
                clip: options.clip,
                ..Default::default()
            },
        )
        .with_visual(options.visual)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group).label(
                options
                    .accessibility_label
                    .unwrap_or_else(|| format!("{name} allocation")),
            ),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_interaction_helpers_gate_visibility_and_enabled_state() {
        let mut document = UiDocument::new(root_style(320.0, 200.0));
        let root = document.root;

        let skipped = add_visible(&mut document, root, false, |document, parent| {
            button(
                document,
                parent,
                "hidden.button",
                "Hidden",
                Default::default(),
            )
        });
        assert!(skipped.is_none());

        let hidden = add_visible_ui(
            &mut document,
            root,
            "hidden.ui",
            false,
            |document, parent| {
                button(
                    document,
                    parent,
                    "hidden.ui.button",
                    "Hidden",
                    Default::default(),
                );
            },
        );
        assert_eq!(document.node(hidden).style.layout.display, Display::None);
        assert!(document
            .node(hidden)
            .accessibility
            .as_ref()
            .is_some_and(|meta| meta.hidden));

        let disabled = add_enabled(&mut document, root, false, |document, parent| {
            button(
                document,
                parent,
                "disabled.button",
                "Disabled",
                Default::default(),
            )
        });
        assert!(!document.node(disabled).input.pointer);
        assert!(
            !document
                .node(disabled)
                .accessibility
                .as_ref()
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn allocation_and_scroll_helpers_wrap_core_primitives() {
        let mut document = UiDocument::new(root_style(400.0, 260.0));
        let root = document.root;

        let exact = allocate_exact_size(
            &mut document,
            root,
            "exact",
            UiSize::new(64.0, 32.0),
            AllocationOptions::default(),
        );
        let at_least = allocate_at_least(
            &mut document,
            root,
            "min",
            UiSize::new(80.0, 40.0),
            AllocationOptions::default(),
        );
        let sized = add_sized(
            &mut document,
            root,
            "sized",
            UiSize::new(96.0, 48.0),
            AllocationOptions::default(),
            |document, parent| {
                label(
                    document,
                    parent,
                    "sized.label",
                    "Sized",
                    TextStyle::default(),
                    LayoutStyle::new(),
                );
            },
        );
        let painter = allocate_painter(
            &mut document,
            root,
            "painter",
            UiSize::new(72.0, 36.0),
            vec![ScenePrimitive::Line {
                from: UiPoint::new(0.0, 0.0),
                to: UiPoint::new(72.0, 36.0),
                stroke: StrokeStyle::new(ColorRgba::WHITE, 1.0),
            }],
            SceneOptions::default(),
        );

        assert_eq!(document.node(exact).style.layout.size.width, length(64.0));
        assert_eq!(
            document.node(at_least).style.layout.min_size.width,
            length(80.0)
        );
        assert_eq!(document.node(sized).children.len(), 1);
        assert!(matches!(
            document.node(painter).content,
            UiContent::Scene(_)
        ));

        let scroll = document.add_child(
            root,
            UiNode::container("scroll", LayoutStyle::size(100.0, 100.0))
                .with_scroll(ScrollAxes::BOTH),
        );
        document.node_mut(scroll).scroll = Some(ScrollState {
            axes: ScrollAxes::BOTH,
            offset: UiPoint::new(0.0, 0.0),
            viewport_size: UiSize::new(100.0, 100.0),
            content_size: UiSize::new(400.0, 400.0),
        });
        document.node_mut(scroll).layout.rect = UiRect::new(0.0, 0.0, 100.0, 100.0);

        let plan = scroll_to_rect_with_options(
            &mut document,
            scroll,
            UiRect::new(140.0, 150.0, 20.0, 20.0),
            RevealOptions::nearest(),
        )
        .unwrap();

        assert!(plan.changed());
        assert_eq!(
            document.scroll_state(scroll).unwrap().offset,
            UiPoint::new(60.0, 70.0)
        );
        assert!(!scroll_to_cursor(
            &mut document,
            scroll,
            UiRect::new(60.0, 70.0, 10.0, 10.0)
        ));
    }
}
