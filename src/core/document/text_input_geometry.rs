use super::*;
use crate::effective_geometry::clipped_visible_rect;

impl UiDocument {
    /// Transforms authored content after it has been translated to the node's
    /// layout origin. Layout bounds already include UI scaling; local scene
    /// content does not. Scale around that origin before applying animation.
    pub(crate) fn node_content_transform(&self, id: UiNodeId) -> EffectiveTransform {
        let rect = self.node(id).layout.rect;
        let animation = self.node_effective_transform(id);
        let scale = self.ui_scale();
        EffectiveTransform::new(
            UiPoint::new(
                animation.translation.x + rect.x * (1.0 - scale) * animation.scale,
                animation.translation.y + rect.y * (1.0 - scale) * animation.scale,
            ),
            animation.scale * scale,
        )
    }

    pub(crate) fn text_input_content_node(&self, owner: UiNodeId) -> UiNodeId {
        self.node(owner)
            .text_input_content()
            .map_or(owner, |content| content.node)
    }

    pub(crate) fn text_input_content_bounds(&self, owner: UiNodeId) -> UiRect {
        let rect = self.node(self.text_input_content_node(owner)).layout.rect;
        let scale = self.ui_scale();
        UiRect::new(0.0, 0.0, rect.width / scale, rect.height / scale)
    }

    pub(crate) fn text_input_pointer_geometry(
        &self,
        owner: UiNodeId,
        position: UiPoint,
    ) -> Option<crate::TextInputPointerGeometry> {
        let content = self.text_input_content_node(owner);
        let rect = self.node(content).layout.rect;
        let position = self
            .node_content_transform(content)
            .inverse_transform_point(position)?;
        Some(crate::TextInputPointerGeometry {
            point: UiPoint::new(position.x - rect.x, position.y - rect.y),
            bounds: self.text_input_content_bounds(owner),
            text_style: self
                .node(owner)
                .text_input_content()
                .map(|content| content.text_style.clone()),
            mask: self
                .node(owner)
                .text_input_content()
                .and_then(|content| content.mask),
        })
    }

    pub(crate) fn text_input_cursor_rect(&self, owner: UiNodeId, cursor: UiRect) -> UiRect {
        let content = self.text_input_content_node(owner);
        let origin = self.node(content).layout.rect;
        let mut cursor = self
            .node_content_transform(content)
            .transform_rect_bounds(UiRect::new(
                origin.x + cursor.x,
                origin.y + cursor.y,
                cursor.width,
                cursor.height,
            ));
        let content_node = self.node(content);
        let rect = content_node.layout().rect;
        let transform = self.node_effective_transform(content);
        let content_clip = self
            .node(owner)
            .text_input_content()
            .map(|_| content_node.layout().clip_rect);
        let visible = content_clip.or_else(|| {
            clipped_visible_rect(
                transform.transform_rect_bounds(rect),
                &[EffectiveClip::new(content_node.layout().clip_rect)],
            )
        });
        if let Some(visible) = visible {
            // Authored scene content can extend beyond its allocated box.
            // Match its actual clip, including a partially visible caret.
            if let Some(clipped) = content_clip.and_then(|clip| cursor.intersection(clip)) {
                cursor = clipped;
            } else {
                cursor.width = cursor.width.min(visible.width).max(1.0);
                cursor.height = cursor.height.min(visible.height).max(1.0);
                cursor.x = cursor
                    .x
                    .clamp(visible.x, (visible.right() - cursor.width).max(visible.x));
                cursor.y = cursor
                    .y
                    .clamp(visible.y, (visible.bottom() - cursor.height).max(visible.y));
            }
        }
        cursor
    }
}
