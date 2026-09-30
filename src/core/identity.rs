//! Shared scoped identity for retained runtime state and frame history.

use std::collections::HashMap;

use crate::{LayoutSnapshot, UiDocument, UiNodeId};

// Segments keep names containing '/' distinct from actual parent boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct NodeIdentity(Vec<String>);

#[derive(Debug, Default)]
pub(crate) struct NodeIdentityIndex {
    pub by_node: Vec<Option<NodeIdentity>>,
    pub by_identity: HashMap<NodeIdentity, UiNodeId>,
}

impl NodeIdentityIndex {
    pub fn from_document(document: &UiDocument) -> Self {
        Self::from_nodes(
            document
                .nodes()
                .iter()
                .enumerate()
                .map(|(index, node)| {
                    (
                        UiNodeId(index),
                        node.logical_parent(),
                        node.name().to_owned(),
                    )
                })
                .collect(),
        )
    }

    pub fn from_layout(layout: &LayoutSnapshot) -> Self {
        fn collect(
            layout: &LayoutSnapshot,
            parent: Option<UiNodeId>,
            nodes: &mut Vec<(UiNodeId, Option<UiNodeId>, String)>,
            owners: &mut Vec<(UiNodeId, UiNodeId)>,
        ) {
            nodes.push((layout.id, parent, layout.name.clone()));
            if let Some(owner) = layout.portal_owner {
                owners.push((layout.id, owner));
            }
            for child in &layout.children {
                collect(child, Some(layout.id), nodes, owners);
            }
        }
        let mut nodes = Vec::new();
        let mut owners = Vec::new();
        collect(layout, None, &mut nodes, &mut owners);
        // Physical tree traversal can visit a detached popup before its source
        // owner. Authored node IDs order both layout parents and owners first.
        if !owners.is_empty() {
            nodes.sort_unstable_by_key(|(id, _, _)| id.index());
        }
        for (id, owner) in owners {
            // A snapshot subtree can omit the source owner. Its local physical
            // root remains a valid identity boundary in that case.
            if nodes
                .binary_search_by_key(&owner.index(), |(id, _, _)| id.index())
                .is_ok()
            {
                let index = nodes
                    .binary_search_by_key(&id.index(), |(id, _, _)| id.index())
                    .unwrap();
                nodes[index].1 = Some(owner);
            }
        }
        Self::from_nodes(nodes)
    }

    fn from_nodes(nodes: Vec<(UiNodeId, Option<UiNodeId>, String)>) -> Self {
        let length = nodes
            .iter()
            .map(|(id, _, _)| id.index() + 1)
            .max()
            .unwrap_or(0);
        let mut paths: Vec<Option<NodeIdentity>> = vec![None; length];
        let mut counts = HashMap::<NodeIdentity, usize>::new();
        for (id, parent, name) in &nodes {
            let mut path = parent.map_or_else(Vec::new, |parent| {
                paths[parent.index()].as_ref().unwrap().0.clone()
            });
            path.push(name.clone());
            let identity = NodeIdentity(path);
            *counts.entry(identity.clone()).or_default() += 1;
            paths[id.index()] = Some(identity);
        }
        let mut identities = Self {
            by_node: vec![None; length],
            by_identity: HashMap::new(),
        };
        for (id, parent, _) in &nodes {
            let identity = paths[id.index()].take().unwrap();
            let parent_valid =
                parent.is_none_or(|parent| identities.by_node[parent.index()].is_some());
            if parent_valid && counts[&identity] == 1 {
                identities.by_identity.insert(identity.clone(), *id);
                identities.by_node[id.index()] = Some(identity);
            }
        }
        identities
    }

    pub fn remap(&self, node: UiNodeId, next: &Self) -> Option<UiNodeId> {
        let identity = self.by_node.get(node.index())?.as_ref()?;
        next.by_identity.get(identity).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::document::view_fragment::ViewFragment;
    use crate::{LayoutStyle, UiNode};

    #[test]
    fn portal_identities_match_layout_snapshots_and_follow_source_owners() {
        let mut doc = UiDocument::new(LayoutStyle::size(300.0, 200.0));
        let host = doc.ensure_app_overlay_portal();
        let owners = ["first", "second"].map(|name| {
            doc.add_child(
                doc.root(),
                UiNode::container(name, LayoutStyle::size(100.0, 30.0)),
            )
        });
        let popups = owners.map(|owner| {
            doc.add_portal_child(
                owner,
                crate::UiPortalTarget::AppOverlay,
                UiNode::container("popup", LayoutStyle::size(100.0, 30.0)),
            )
        });
        let before = doc.identity_index().clone();
        assert!(popups.iter().all(|id| before.by_node[id.index()].is_some()));
        assert_ne!(
            before.by_node[popups[0].index()],
            before.by_node[popups[1].index()]
        );
        let layout = doc.layout_snapshot();
        assert_eq!(
            NodeIdentityIndex::from_layout(&layout).by_node,
            before.by_node
        );
        let detached = layout
            .children
            .iter()
            .find(|child| child.id == host)
            .unwrap()
            .children
            .first()
            .unwrap();
        assert!(NodeIdentityIndex::from_layout(detached).by_node[popups[0].index()].is_some());
        doc.node_mut(owners[0]).name = "replacement".into();
        let after = doc.identity_index();
        assert_eq!(before.remap(popups[0], after), None);
        assert_eq!(before.remap(popups[1], after), Some(popups[1]));
        assert_eq!(
            NodeIdentityIndex::from_layout(&doc.layout_snapshot()).by_node,
            after.by_node
        );
    }

    #[test]
    fn cached_identities_follow_mutations_without_changing_retained_snapshots() {
        let mut doc = UiDocument::new(LayoutStyle::size(200.0, 100.0));
        let group = doc.add_child(
            doc.root(),
            UiNode::container("group", LayoutStyle::size(100.0, 80.0)),
        );
        let child = doc.add_child(
            group,
            UiNode::container("editor", LayoutStyle::size(80.0, 60.0)),
        );
        let authored_count = doc.node_count();
        let original = doc.identity_index().clone();
        let original_paths = original.by_node.clone();
        let check = |doc: &UiDocument, survives: bool| {
            let current = doc.identity_index();
            let fresh = NodeIdentityIndex::from_document(doc);
            assert_eq!(current.by_node, fresh.by_node);
            assert_eq!(current.by_identity, fresh.by_identity);
            assert_eq!(original.remap(child, current), survives.then_some(child));
            assert_eq!(
                original.by_node, original_paths,
                "retained snapshot must stay immutable"
            );
        };
        check(&doc, true);
        doc.set_node_style(child, LayoutStyle::size(60.0, 40.0));
        check(&doc, true);
        doc.add_child(
            doc.root(),
            UiNode::container("group", LayoutStyle::size(50.0, 30.0)),
        );
        check(&doc, false);
        doc.truncate_runtime_nodes(authored_count);
        check(&doc, true);
        doc.node_mut(group).name = "renamed".into();
        check(&doc, false);
        doc.edit_node(group, |node| node.name = "group".into());
        check(&doc, true);
        let fragment = ViewFragment::from_document(UiDocument::new(LayoutStyle::size(100.0, 80.0)));
        doc.append_view_fragment(doc.root(), "group".into(), &fragment, true);
        check(&doc, false);
        doc.truncate_runtime_nodes(authored_count);
        check(&doc, true);
    }
}
