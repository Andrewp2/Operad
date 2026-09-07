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
                .map(|(index, node)| (UiNodeId(index), node.parent(), node.name().to_owned()))
                .collect(),
        )
    }

    pub fn from_layout(layout: &LayoutSnapshot) -> Self {
        fn collect(
            layout: &LayoutSnapshot,
            parent: Option<UiNodeId>,
            nodes: &mut Vec<(UiNodeId, Option<UiNodeId>, String)>,
        ) {
            nodes.push((layout.id, parent, layout.name.clone()));
            for child in &layout.children {
                collect(child, Some(layout.id), nodes);
            }
        }
        let mut nodes = Vec::new();
        collect(layout, None, &mut nodes);
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
