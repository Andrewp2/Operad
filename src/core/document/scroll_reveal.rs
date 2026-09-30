//! Reveal a widget's active row once per selection or layout change.

use super::*;
use crate::core::identity::NodeIdentity;

#[derive(Debug)]
pub(super) struct ScrollReveal {
    pub(super) target: UiNodeId,
    applied: Option<ScrollRevealState>,
}

#[derive(Debug, Clone)]
pub(crate) struct ScrollRevealState {
    target: UiNodeId,
    identity: Option<NodeIdentity>,
    axes: ScrollAxes,
    viewport: UiSize,
    content_rect: UiRect,
}

impl ScrollRevealState {
    pub(crate) fn remap(&self, identities: &NodeIdentityIndex) -> Option<Self> {
        let target = *identities.by_identity.get(self.identity.as_ref()?)?;
        Some(Self {
            target,
            ..self.clone()
        })
    }

    fn matches(&self, next: &Self) -> bool {
        // Subtracting window origins and adding fractional wheel offsets can
        // introduce rounding noise. Scrolling alone must not reissue a reveal.
        let near = |a: f32, b: f32| (a - b).abs() < 0.01;
        self.target == next.target
            && self.identity == next.identity
            && self.axes == next.axes
            && near(self.viewport.width, next.viewport.width)
            && near(self.viewport.height, next.viewport.height)
            && near(self.content_rect.x, next.content_rect.x)
            && near(self.content_rect.y, next.content_rect.y)
            && near(self.content_rect.width, next.content_rect.width)
            && near(self.content_rect.height, next.content_rect.height)
    }
}

impl UiDocument {
    pub(crate) fn set_scroll_reveal_target(&mut self, owner: UiNodeId, target: Option<UiNodeId>) {
        if self.scroll_reveals.get(&owner).map(|reveal| reveal.target) == target {
            return;
        }
        match target {
            Some(target) => {
                self.scroll_reveals.insert(
                    owner,
                    ScrollReveal {
                        target,
                        applied: None,
                    },
                );
            }
            None => {
                self.scroll_reveals.remove(&owner);
            }
        }
        self.mark_layout_changed();
    }

    pub(crate) fn scroll_reveal_state(&self, owner: UiNodeId) -> Option<&ScrollRevealState> {
        self.scroll_reveals.get(&owner)?.applied.as_ref()
    }

    pub(crate) fn restore_scroll_reveal(&mut self, owner: UiNodeId, state: ScrollRevealState) {
        if let Some(reveal) = self.scroll_reveals.get_mut(&owner) {
            reveal.applied = Some(state);
        }
    }

    pub(super) fn apply_scroll_reveals(
        &mut self,
        sizing: &LayoutSizingPass,
        viewport: UiSize,
    ) -> Result<(), taffy::TaffyError> {
        if self.scroll_reveals.is_empty() {
            return Ok(());
        }
        let displayed = self.displayed_nodes();
        let mut owners: Vec<_> = self.scroll_reveals.keys().copied().collect();
        // Parents precede children in the document. Reveal inner scrollers
        // first so outer scrollers use the target's updated position.
        owners.sort_unstable_by_key(|id| std::cmp::Reverse(id.0));
        for owner in owners {
            let target = self.scroll_reveals[&owner].target;
            if !displayed.get(target.0).copied().unwrap_or(false)
                || !self.node_is_descendant_or_self(owner, target)
            {
                self.scroll_reveals.get_mut(&owner).unwrap().applied = None;
                continue;
            }
            let Some(scroll) = self.scroll_state(owner) else {
                continue;
            };
            if scroll.viewport_size.width <= 0.0 || scroll.viewport_size.height <= 0.0 {
                continue;
            }
            // A clipped row has layout.visible == false but still has valid
            // content geometry. Authored display, above, decides eligibility.
            let rect = self.nodes[target.0].layout.rect;
            let origin = self.nodes[owner.0].layout.rect;
            let next = ScrollRevealState {
                target,
                identity: self.identity_index().by_node[target.0].clone(),
                axes: scroll.axes,
                viewport: scroll.viewport_size,
                content_rect: UiRect::new(
                    rect.x - origin.x + scroll.offset.x,
                    rect.y - origin.y + scroll.offset.y,
                    rect.width,
                    rect.height,
                ),
            };
            if self
                .scroll_reveal_state(owner)
                .is_some_and(|old| old.matches(&next))
            {
                continue;
            }
            if !scroll.offset_is_authored() && self.scroll_to_node(owner, target) {
                self.apply_layout_position_pass(sizing, viewport)?;
            }
            self.scroll_reveals.get_mut(&owner).unwrap().applied = Some(next);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LayoutStyle;

    #[test]
    fn scroll_reveal_tracks_content_geometry_visibility_and_request_lifetime() {
        let viewport = UiSize::new(300.0, 250.0);
        let mut doc = UiDocument::new(LayoutStyle::size(viewport.width, viewport.height));
        let owner = doc.add_child(
            doc.root(),
            UiNode::container("scroll", LayoutStyle::size(100.0, 80.0))
                .with_scroll(ScrollAxes::BOTH),
        );
        let target = doc.add_child(
            owner,
            UiNode::container(
                "active",
                LayoutStyle::absolute_rect(UiRect::new(180.0, 180.0, 20.0, 20.0)),
            ),
        );
        let unrelated = doc.add_child(
            doc.root(),
            UiNode::container(
                "unrelated",
                LayoutStyle::absolute_rect(UiRect::new(180.0, 180.0, 20.0, 20.0)),
            ),
        );
        doc.set_scroll_reveal_target(owner, Some(target));
        for step in 0..8 {
            match step {
                1 => doc.set_node_style(
                    target,
                    LayoutStyle::absolute_rect(UiRect::new(130.0, 150.0, 20.0, 20.0)),
                ),
                2 => doc.node_mut(target).style.layout.display = Display::None,
                3 => doc.node_mut(target).style.layout.display = Display::Flex,
                4 => doc.set_scroll_reveal_target(owner, None),
                5 => doc.set_scroll_reveal_target(owner, Some(target)),
                6 => doc.node_mut(target).name = "replacement".to_owned(),
                7 => doc.set_scroll_reveal_target(owner, Some(unrelated)),
                _ => {}
            }
            doc.compute_layout(viewport, &mut ApproxTextMeasurer)
                .unwrap();
            if matches!(step, 2 | 4 | 7) {
                assert_eq!(
                    doc.scroll_state(owner).unwrap().offset(),
                    UiPoint::new(0.0, 0.0),
                    "step {step}"
                );
            } else {
                let layout = doc.node(target).layout();
                assert!(
                    layout.rect.x >= layout.clip_rect.x
                        && layout.rect.right() <= layout.clip_rect.right()
                        && layout.rect.y >= layout.clip_rect.y
                        && layout.rect.bottom() <= layout.clip_rect.bottom(),
                    "step {step}: {layout:?}"
                );
                assert!(doc.set_scroll_offset(owner, UiPoint::new(0.0, 0.0)));
                doc.compute_layout(viewport, &mut ApproxTextMeasurer)
                    .unwrap();
                assert_eq!(
                    doc.scroll_state(owner).unwrap().offset(),
                    UiPoint::new(0.0, 0.0),
                    "step {step}: manual scroll was overridden"
                );
            }
        }
    }
}
