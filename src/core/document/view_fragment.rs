//! Authored section snapshots; these never contain live interaction or layout caches.

use super::*;

pub(crate) struct ViewFragment {
    nodes: Vec<UiNode>,
    focus: UiFocusState,
    focus_authored: bool,
    overlays: Vec<UiNodeId>,
    reused_nodes: HashSet<UiNodeId>,
    scroll_reveals: Vec<(UiNodeId, UiNodeId)>,
}

impl ViewFragment {
    pub fn from_document(document: UiDocument) -> Self {
        let mut overlays = document.view_overlays;
        if let Some(id) = document
            .portal_hosts
            .get(&UiPortalId::from(APP_OVERLAY_PORTAL))
        {
            if !overlays.contains(id) {
                overlays.push(*id);
            }
        }
        Self {
            nodes: document.nodes,
            focus: document.focus,
            focus_authored: document.focus_authored,
            overlays,
            reused_nodes: document.reused_view_nodes,
            scroll_reveals: document
                .scroll_reveals
                .into_iter()
                .map(|(owner, reveal)| (owner, reveal.target))
                .collect(),
        }
    }
}

impl UiDocument {
    pub(crate) fn append_view_fragment(
        &mut self,
        parent: UiNodeId,
        name: String,
        fragment: &ViewFragment,
        reused: bool,
    ) -> UiNodeId {
        self.identity_cache.take();
        self.visual_order_cache.take();
        let offset = self.nodes.len();
        let root = UiNodeId(offset);
        let remap = |id: UiNodeId| {
            assert!(
                id.0 < fragment.nodes.len(),
                "view contains a foreign node ID"
            );
            UiNodeId(id.0 + offset)
        };
        let mut path = vec![name.clone()];
        let mut ancestor = Some(parent);
        while let Some(id) = ancestor {
            path.push(self.node(id).name.clone());
            ancestor = self.node(id).parent;
        }
        path.reverse();
        self.nodes.reserve(fragment.nodes.len());
        for (index, original) in fragment.nodes.iter().enumerate() {
            let mut node = original.clone();
            node.parent = original.parent.map(remap).or(Some(parent));
            node.stack_parent = original.stack_parent.map(remap);
            node.portal_owner = original.portal_owner.map(remap);
            if let Some(content) = &mut node.text_input_content {
                content.node = remap(content.node);
            }
            for child in &mut node.children {
                *child = remap(*child);
            }
            node.layout = ComputedLayout::default();
            if reused || fragment.reused_nodes.contains(&UiNodeId(index)) {
                // Only stateful nodes need this marker during reconciliation.
                // Large cached hit-target trees otherwise pay for a hash entry
                // per decorative or stateless node on every rebuild.
                if node.animation.is_some() || node.scroll.is_some() {
                    self.reused_view_nodes.insert(UiNodeId(offset + index));
                }
                if let Some(scroll) = &mut node.scroll {
                    scroll.offset_source = ScrollOffsetSource::Host;
                }
            }
            if let Some(constraint) = &mut node.layout_constraint {
                match constraint {
                    UiNodeLayoutConstraint::AnchoredPopup { .. }
                    | UiNodeLayoutConstraint::Tooltip { .. } => {}
                    UiNodeLayoutConstraint::InlineIntrinsicSize { sources, .. }
                    | UiNodeLayoutConstraint::StackedIntrinsicSize { sources, .. } => {
                        for source in sources {
                            *source = remap(*source);
                        }
                    }
                }
            }
            if let Some(meta) = &mut node.accessibility {
                if let crate::accessibility::FocusRestoreTarget::Node(target) =
                    &mut meta.modal_focus_restore
                {
                    *target = remap(*target);
                }
                let relations = &mut meta.relations;
                for ids in [
                    &mut relations.labelled_by,
                    &mut relations.described_by,
                    &mut relations.controls,
                    &mut relations.owns,
                ] {
                    for id in ids {
                        *id = remap(*id);
                    }
                }
                relations.active_descendant = relations.active_descendant.map(remap);
            }
            if index == 0 {
                node.name = name.clone();
                node.children
                    .retain(|id| !fragment.overlays.contains(&UiNodeId(id.0 - offset)));
            }
            if fragment.overlays.contains(&UiNodeId(index)) {
                // Each section owns its viewport host. A scoped host avoids name
                // collisions between menus from independently reusable sections.
                node.name = format!("view-overlay:{path:?}:{}", original.name);
                node.parent = Some(self.root);
                self.nodes[self.root.0]
                    .children
                    .push(UiNodeId(offset + index));
                self.view_overlays.push(UiNodeId(offset + index));
            }
            self.nodes.push(node);
        }
        self.nodes[parent.0].children.push(root);
        for &(owner, target) in &fragment.scroll_reveals {
            self.set_scroll_reveal_target(remap(owner), Some(remap(target)));
        }
        if fragment.focus_authored && !reused {
            self.focus = UiFocusState {
                hovered: fragment.focus.hovered.map(remap),
                focused: fragment.focus.focused.map(remap),
                pressed: fragment.focus.pressed.map(remap),
            };
            self.focus_authored = true;
        }
        self.mark_layout_changed();
        root
    }
}
