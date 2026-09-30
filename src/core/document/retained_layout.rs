//! Keep Taffy's dependency caches across authored document revisions.

use super::*;

impl UiDocument {
    pub(crate) fn inherit_frame_work(&mut self, previous: &mut Self) {
        if self.nodes.len() == previous.nodes.len()
            && self.nodes.iter().zip(&previous.nodes).all(|(node, old)| {
                node.name == old.name && node.logical_parent() == old.logical_parent()
            })
        {
            if let Some(identities) = previous.identity_cache.get() {
                self.identity_cache.get_or_init(|| identities.clone());
            }
        }
        // Size and position have different dependencies. A paint-only rebuild
        // can keep sizing even when host-restored scrolling needs a new position
        // pass. Constraints retain the ordinary reconciliation path.
        self.retained_layout = previous.retained_layout.take();
        if let Some(key) = previous.layout_cache_key {
            if self.retained_layout.is_some()
                && self.root == previous.root
                && self.ui_scale() == previous.ui_scale()
                && self.nodes.len() == previous.nodes.len()
                && self.nodes.iter().zip(&previous.nodes).all(|(node, old)| {
                    node.parent == old.parent
                        && node.children == old.children
                        && node.style.layout == old.style.layout
                        && auto_scrollbar_layout_gutter(node) == auto_scrollbar_layout_gutter(old)
                        && node.layout_constraint.is_none()
                        && old.layout_constraint.is_none()
                        && match (&node.content, &old.content) {
                            (UiContent::Text(text), UiContent::Text(old)) => text == old,
                            (UiContent::Text(_), _) | (_, UiContent::Text(_)) => false,
                            _ => true,
                        }
                })
            {
                let key = LayoutCacheKey {
                    revision: self.layout_revision,
                    ..key
                };
                self.layout_sizing_cache_key = Some(key);
                // Flat node indices and sizing are unchanged; names may have
                // changed, so future reconciliation must use current identity.
                let identities = self.identity_index().clone();
                self.retained_layout.as_mut().unwrap().identities = identities;
                if self.nodes.iter().zip(&previous.nodes).all(|(node, old)| {
                    node.style.clip == old.style.clip
                        && node.style.opacity == old.style.opacity
                        && node.clip_scope == old.clip_scope
                        && node.scroll.is_none()
                        && old.scroll.is_none()
                        && node.portal_owner.is_none()
                        && old.portal_owner.is_none()
                        && matches!(node.content, UiContent::Canvas(_))
                            == matches!(old.content, UiContent::Canvas(_))
                }) {
                    for (node, old) in self.nodes.iter_mut().zip(&previous.nodes) {
                        node.layout = old.layout;
                    }
                    self.layout_cache_key = Some(key);
                }
            }
        }
        let mut uploads = previous.take_resource_updates();
        uploads.append(self.take_resource_updates());
        self.resource_updates = uploads;
    }

    pub(super) fn reconcile_layout_tree(&mut self) -> Result<LayoutSizingPass, taffy::TaffyError> {
        let identities = self.identity_index().clone();
        let previous = self.retained_layout.take();
        let (mut taffy, old_mapping, old_identities, old_content) = match previous {
            Some(previous) => (
                previous.taffy,
                previous.mapping,
                previous.identities,
                previous.measured_content,
            ),
            None => (TaffyTree::new(), Vec::new(), Arc::default(), Vec::new()),
        };
        let mut mapping = Vec::with_capacity(self.nodes.len());
        let mut measured_content = Vec::with_capacity(self.nodes.len());
        let mut retained = HashSet::new();
        for (index, node) in self.nodes.iter().enumerate() {
            let previous = identities.by_node[index]
                .as_ref()
                .and_then(|identity| old_identities.by_identity.get(identity))
                .copied();
            let existing = previous.and_then(|id| old_mapping[id.0]);
            let mut style = scaled_taffy_style(&node.style.layout, self.ui_scale());
            apply_auto_scrollbar_layout_gutter(node, &mut style, self.ui_scale());
            let text = match &node.content {
                UiContent::Text(text) if node.children.is_empty() => {
                    Some(scaled_text_content(text, self.ui_scale()))
                }
                _ => None,
            };
            let context = text.as_ref().map(|text| MeasureContext::Text {
                node: UiNodeId(index),
                text: text.clone(),
            });
            let mut content = None;
            let id = if let Some(id) = existing {
                retained.insert(id);
                let style_changed = taffy.style(id)? != &style;
                if style_changed {
                    taffy.set_style(id, style)?;
                }
                let text_changed = match (taffy.get_node_context(id), &text) {
                    (Some(MeasureContext::Text { text: old, .. }), Some(text)) => old != text,
                    (None, None) => false,
                    _ => true,
                };
                if text_changed {
                    taffy.set_node_context(id, context)?;
                } else {
                    if let Some(MeasureContext::Text { node, .. }) = taffy.get_node_context_mut(id)
                    {
                        *node = UiNodeId(index);
                    }
                    if !style_changed {
                        content = previous.and_then(|id| old_content[id.0]);
                    }
                }
                id
            } else if let Some(context) = context {
                taffy.new_leaf_with_context(style, context)?
            } else {
                taffy.new_leaf(style)?
            };
            mapping.push(Some(id));
            measured_content.push(content);
        }
        // Update parents before removing old nodes. Taffy's remove() unlinks
        // children without invalidating the parent's cached size.
        for (index, node) in self.nodes.iter().enumerate() {
            let id = mapping[index].unwrap();
            let children: Vec<_> = node
                .children
                .iter()
                .map(|child| mapping[child.0].unwrap())
                .collect();
            if taffy.children(id)? != children {
                taffy.set_children(id, &children)?;
            }
        }
        for id in old_mapping.into_iter().flatten() {
            if !retained.contains(&id) {
                taffy.set_node_context(id, None)?;
                taffy.remove(id)?;
            }
        }
        Ok(LayoutSizingPass {
            root: mapping[self.root.0].unwrap(),
            taffy,
            mapping,
            measured_content,
            sizes: vec![UiSize::ZERO; self.nodes.len()],
            identities,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Measurements(usize);

    impl TextMeasurer for Measurements {
        fn measure(
            &mut self,
            text: &TextContent,
            known: KnownSize,
            available: AvailableSize,
        ) -> UiSize {
            self.0 += 1;
            ApproxTextMeasurer.measure(text, known, available)
        }
    }

    fn describe(change: usize) -> UiDocument {
        let mut doc = UiDocument::new(LayoutStyle::column().with_size(400.0, 300.0));
        let root = doc.root();
        let group = doc.add_child(
            root,
            UiNode::container("group", LayoutStyle::size(120.0, 70.0)),
        );
        doc.node_mut(group).style.clip = ClipBehavior::Clip;
        let text = doc.add_child(
            if change == 3 { root } else { group },
            UiNode::text(
                "label",
                if change == 2 {
                    "a longer label"
                } else {
                    "label"
                },
                TextStyle::default(),
                LayoutStyle::new(),
            ),
        );
        let canvas = doc.add_child(
            group,
            UiNode::canvas(
                "canvas",
                "canvas",
                LayoutStyle::absolute_rect(UiRect::new(100.0, 30.0, 180.0, 80.0)),
            ),
        );
        doc.node_mut(canvas).style.layout.aspect_ratio = Some(1.0);
        doc.add_child(
            root,
            UiNode::container("footer", LayoutStyle::size(30.0, 20.0)),
        );
        doc.set_node_visual(
            group,
            UiVisual::panel(
                if change == 0 {
                    ColorRgba::WHITE
                } else {
                    ColorRgba::BLACK
                },
                None,
                0.0,
            ),
        );
        match change {
            1 => doc.set_node_style(group, LayoutStyle::size(80.0, 40.0)),
            4 => {
                if let UiContent::Text(text) = &mut doc.node_mut(text).content {
                    text.style.font_size *= 2.0;
                }
            }
            5 => doc.node_mut(group).style.clip = ClipBehavior::None,
            6 => doc.node_mut(group).style.opacity = 0.4,
            7 => doc.node_mut(canvas).style.layout.aspect_ratio = Some(2.0),
            8 => doc.set_node_content(canvas, UiContent::Empty),
            9 => {
                doc.add_child(
                    root,
                    UiNode::container("extra", LayoutStyle::size(30.0, 20.0)),
                );
            }
            10 => {
                doc.add_portal_child(
                    group,
                    UiPortalTarget::AppOverlay,
                    UiNode::container("popup", LayoutStyle::size(50.0, 40.0)),
                );
            }
            11 => {
                doc.node_mut(group).layout_constraint =
                    Some(UiNodeLayoutConstraint::InlineIntrinsicSize {
                        sources: vec![text],
                        min_size: UiSize::ZERO,
                    })
            }
            12 => {
                let node = doc.node_mut(group);
                node.scroll = Some(ScrollState::new(ScrollAxes::BOTH));
                node.style.clip = ClipBehavior::Clip;
            }
            15 => doc.node_mut(root).children.reverse(),
            16 => doc.node_mut(canvas).clip_scope = ClipScope::Viewport,
            17 => doc.node_mut(group).name = "renamed".into(),
            _ => {}
        }
        doc
    }

    #[test]
    fn paint_only_rebuild_preserves_measurements_and_geometry_but_refreshes_paint() {
        let viewport = UiSize::new(400.0, 300.0);
        let mut measurements = Measurements::default();
        let mut old = describe(0);
        old.compute_layout(viewport, &mut measurements).unwrap();
        let measured = measurements.0;
        let previous_paint = old.paint_list();
        let geometry: Vec<_> = old.nodes.iter().map(|node| node.layout).collect();
        let mut next = describe(18);
        next.inherit_frame_work(&mut old);
        next.compute_layout(viewport, &mut measurements).unwrap();
        assert_eq!(
            measurements.0, measured,
            "paint-only rebuilding must not measure text again"
        );
        assert_eq!(
            next.nodes
                .iter()
                .map(|node| node.layout)
                .collect::<Vec<_>>(),
            geometry
        );
        assert_ne!(
            next.paint_list(),
            previous_paint,
            "reuse must not retain the old paint content"
        );
        let retained = next.identity_index().clone();
        next.node_mut(UiNodeId(1)).name = "replacement".into();
        assert_eq!(retained.remap(UiNodeId(1), next.identity_index()), None);
        assert_eq!(
            next.identity_index().by_node,
            NodeIdentityIndex::from_document(&next).by_node
        );
    }

    #[test]
    fn inherited_layout_matches_fresh_layout_after_dependency_changes() {
        let mut old = describe(0);
        old.compute_layout(UiSize::new(400.0, 300.0), &mut ApproxTextMeasurer)
            .unwrap();
        for change in (0..18).chain((0..18).rev()) {
            let viewport = if change == 14 {
                UiSize::new(100.0, 70.0)
            } else {
                UiSize::new(400.0, 300.0)
            };
            let scale = if change == 13 {
                UiDocumentScale::new(1.5, 2.0)
            } else {
                UiDocumentScale::DEFAULT
            };
            // Prime identity history, just as RuntimeSession does.
            old.identity_index();
            let mut next = describe(change);
            next.set_scale(scale);
            next.inherit_frame_work(&mut old);
            next.compute_layout(viewport, &mut ApproxTextMeasurer)
                .unwrap();
            let mut fresh = describe(change);
            fresh.set_scale(scale);
            fresh
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .unwrap();
            assert_eq!(
                next.nodes
                    .iter()
                    .map(|node| node.layout)
                    .collect::<Vec<_>>(),
                fresh
                    .nodes
                    .iter()
                    .map(|node| node.layout)
                    .collect::<Vec<_>>(),
                "dependency change {change}"
            );
            assert_eq!(
                next.identity_index().by_node,
                fresh.identity_index().by_node,
                "identity change {change}"
            );
            old = next;
        }
    }
}
