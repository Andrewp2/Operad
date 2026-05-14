use std::f32::consts::TAU;

use super::*;

#[derive(Debug, Clone)]
pub struct SpinnerOptions {
    pub layout: LayoutStyle,
    pub color: ColorRgba,
    pub radius: f32,
    pub stroke_width: f32,
    pub spoke_count: usize,
    pub animation: Option<AnimationMachine>,
    pub accessibility_label: Option<String>,
}

impl Default for SpinnerOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::size(24.0, 24.0),
            color: ColorRgba::new(108, 180, 255, 255),
            radius: 8.0,
            stroke_width: 2.0,
            spoke_count: 12,
            animation: None,
            accessibility_label: None,
        }
    }
}

impl SpinnerOptions {
    pub fn with_layout(mut self, layout: impl Into<LayoutStyle>) -> Self {
        self.layout = layout.into();
        self
    }

    pub fn with_accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

pub fn spinner(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
    options: SpinnerOptions,
) -> UiNodeId {
    let name = name.into();
    let primitives = spinner_primitives(
        options.color,
        options.radius,
        options.stroke_width,
        options.spoke_count,
    );
    let mut node = UiNode::scene(name.clone(), primitives, options.layout).with_accessibility(
        AccessibilityMeta::new(AccessibilityRole::ProgressBar)
            .label(options.accessibility_label.unwrap_or(name))
            .value("Indeterminate")
            .hint("Loading"),
    );
    if let Some(animation) = options.animation {
        node = node.with_animation(animation);
    }
    document.add_child(parent, node)
}

fn spinner_primitives(
    color: ColorRgba,
    radius: f32,
    stroke_width: f32,
    spoke_count: usize,
) -> Vec<ScenePrimitive> {
    let spoke_count = spoke_count.clamp(4, 24);
    let center = UiPoint::new(radius + stroke_width, radius + stroke_width);
    let inner = radius * 0.45;
    let outer = radius;
    (0..spoke_count)
        .map(|index| {
            let t = index as f32 / spoke_count as f32;
            let angle = t * TAU;
            let alpha = (64.0 + 191.0 * t).round().clamp(0.0, 255.0) as u8;
            let spoke_color = ColorRgba::new(color.r, color.g, color.b, alpha);
            ScenePrimitive::Line {
                from: UiPoint::new(
                    center.x + angle.cos() * inner,
                    center.y + angle.sin() * inner,
                ),
                to: UiPoint::new(
                    center.x + angle.cos() * outer,
                    center.y + angle.sin() * outer,
                ),
                stroke: StrokeStyle::new(spoke_color, stroke_width.max(0.5)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_builds_indeterminate_progress_scene() {
        let mut document = UiDocument::new(root_style(120.0, 80.0));
        let root = document.root;
        let node = spinner(
            &mut document,
            root,
            "loading",
            SpinnerOptions {
                spoke_count: 8,
                ..Default::default()
            },
        );

        assert!(matches!(
            &document.node(node).content,
            UiContent::Scene(primitives) if primitives.len() == 8
        ));
        let accessibility = document.node(node).accessibility.as_ref().unwrap();
        assert_eq!(accessibility.role, AccessibilityRole::ProgressBar);
        assert_eq!(accessibility.value.as_deref(), Some("Indeterminate"));
    }
}
