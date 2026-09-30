//! Text fitting using the same measurements as layout and rendering.

use unicode_segmentation::UnicodeSegmentation;

use crate::{AvailableSize, KnownSize, TextContent, TextMeasurer, TextWrap, UiSize};

/// A measured, single-line presentation of a label. The source text is unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct FittedText {
    pub text: String,
    pub size: UiSize,
    /// Whether any source text was omitted, including additional lines.
    pub truncated: bool,
}

/// Fits a label to a width with a logical-end ellipsis, using `measurer`'s fonts.
///
/// Candidates are measured without width constraints and cut only at extended
/// grapheme boundaries. If even the ellipsis cannot fit, the result is empty.
/// Newlines terminate the first line and count as omitted content. Negative or
/// NaN widths are treated as zero. For mixed-direction text the ellipsis follows
/// the retained logical prefix; the shaper determines its visual position.
///
/// Use a font-backed measurer such as `CosmicTextMeasurer` for exact
/// font advances. Keep the original label for accessibility, tooltips, and edits.
pub fn fit_text(
    measurer: &mut impl TextMeasurer,
    text: &TextContent,
    max_width: f32,
) -> FittedText {
    let mut candidate = text.clone();
    candidate.style.wrap = TextWrap::None;
    candidate.style.overflow = crate::paint::TextOverflow::Clip;
    fit_single_line(&text.text, max_width, |value| {
        candidate.text.clear();
        candidate.text.push_str(value);
        measurer.measure(
            &candidate,
            KnownSize {
                width: None,
                height: None,
            },
            AvailableSize {
                width: None,
                height: None,
            },
        )
    })
}

#[cfg(feature = "text-cosmic")]
impl crate::CosmicTextMeasurer {
    /// Fits a label using this measurer's font library and shaping metrics.
    pub fn fit_text(&mut self, text: &TextContent, max_width: f32) -> FittedText {
        fit_text(self, text, max_width)
    }
}

/// Shared by the layout measurer and renderer, which supply their own font
/// system. Bisection bounds shaping work for long labels. Every returned
/// candidate is independently shaped; kerning and ligatures cannot cause it to
/// exceed the width. Non-monotonic advances can produce a conservative prefix.
pub(crate) fn fit_single_line(
    text: &str,
    max_width: f32,
    mut measure: impl FnMut(&str) -> UiSize,
) -> FittedText {
    let max_width = if max_width.is_nan() {
        0.0
    } else {
        max_width.max(0.0)
    };
    let line_end = text
        .find(['\n', '\r', '\u{0085}', '\u{2028}', '\u{2029}'])
        .unwrap_or(text.len());
    let line = &text[..line_end];
    let mut full_size = measure(line);
    if line.is_empty() {
        full_size.width = 0.0;
    }
    if line_end == text.len() && full_size.width <= max_width {
        return FittedText {
            text: line.to_owned(),
            size: full_size,
            truncated: false,
        };
    }
    let marker = "…";
    let marker_size = measure(marker);
    if max_width <= 0.0 || marker_size.width > max_width {
        let mut size = measure("");
        size.width = 0.0;
        return FittedText {
            text: String::new(),
            size,
            truncated: !text.is_empty(),
        };
    }
    let boundaries: Vec<_> = line
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .chain(std::iter::once(line.len()))
        .collect();
    let mut fitted = FittedText {
        text: marker.to_owned(),
        size: marker_size,
        truncated: true,
    };
    let mut low = 1;
    // A width-truncated label must omit at least one grapheme. With extra source
    // lines, retaining the complete first line still represents omitted content.
    let mut high = boundaries.len() - usize::from(line_end == text.len());
    while low < high {
        let middle = low + (high - low) / 2;
        let mut candidate = String::with_capacity(boundaries[middle] + marker.len());
        candidate.push_str(&line[..boundaries[middle]]);
        candidate.push_str(marker);
        let size = measure(&candidate);
        if size.width <= max_width {
            fitted.text = candidate;
            fitted.size = size;
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    fitted
}

#[cfg(all(test, feature = "text-cosmic"))]
mod tests {
    use super::*;
    use crate::{CosmicTextMeasurer, TextStyle};

    fn unbounded(measurer: &mut CosmicTextMeasurer, text: &str) -> UiSize {
        measurer.measure(
            &TextContent::new(
                text,
                TextStyle {
                    wrap: TextWrap::None,
                    ..Default::default()
                },
            ),
            KnownSize {
                width: None,
                height: None,
            },
            AvailableSize {
                width: None,
                height: None,
            },
        )
    }

    #[test]
    fn fitting_uses_font_advances_and_keeps_complete_graphemes() {
        let mut measurer = CosmicTextMeasurer::new();
        assert!(unbounded(&mut measurer, "WWW").width > unbounded(&mut measurer, "iii").width);
        let narrow_width = unbounded(&mut measurer, "iiiiii").width;
        assert!(
            !measurer
                .fit_text(
                    &TextContent::new("iiiiii", TextStyle::default()),
                    narrow_width
                )
                .truncated
        );
        assert!(
            measurer
                .fit_text(
                    &TextContent::new("WWWWWW", TextStyle::default()),
                    narrow_width
                )
                .truncated
        );
        for source in [
            "Wide WWW and narrow iii",
            "e\u{301}e\u{301}e\u{301}e\u{301}",
            "👩🏽‍💻👨‍👩‍👧‍👦🇫🇷abcdef",
            "مرحبا بالعالم",
            "אבג abc דהו",
        ] {
            let original = TextContent::new(source, TextStyle::default());
            let width = unbounded(&mut measurer, source).width;
            let boundaries: Vec<_> = source
                .grapheme_indices(true)
                .map(|(i, _)| i)
                .chain(std::iter::once(source.len()))
                .collect();
            for fraction in [0.0, 0.15, 0.4, 0.8, 1.0, 2.0] {
                let available = width * fraction;
                let fitted = measurer.fit_text(&original, available);
                assert!(
                    fitted.size.width <= available + 0.001,
                    "{source:?}: {fitted:?}"
                );
                assert_eq!(fitted.size, unbounded(&mut measurer, &fitted.text));
                if fitted.truncated && !fitted.text.is_empty() {
                    let prefix = fitted.text.strip_suffix('…').unwrap();
                    assert!(source.starts_with(prefix));
                    assert!(
                        boundaries.contains(&prefix.len()),
                        "split grapheme: {fitted:?}"
                    );
                } else if !fitted.truncated {
                    assert_eq!(fitted.text, source);
                }
                assert_eq!(original.text, source);
            }
        }
    }

    #[test]
    fn fitting_handles_tiny_invalid_and_multiline_bounds() {
        let mut measurer = CosmicTextMeasurer::new();
        let source = TextContent::new("first\nsecond", TextStyle::default());
        for width in [0.0, -1.0, f32::NAN] {
            let fitted = measurer.fit_text(&source, width);
            assert!(fitted.text.is_empty());
            assert!(fitted.truncated);
        }
        let marker_width = unbounded(&mut measurer, "…").width;
        assert!(measurer
            .fit_text(&source, marker_width / 2.0)
            .text
            .is_empty());
        assert_eq!(measurer.fit_text(&source, marker_width).text, "…");
        let multiline = measurer.fit_text(&source, f32::INFINITY);
        assert_eq!(multiline.text, "first…");
        assert!(multiline.truncated);
        let empty = measurer.fit_text(&TextContent::new("", TextStyle::default()), 0.0);
        assert!(empty.text.is_empty());
        assert!(!empty.truncated);
    }

    #[test]
    fn ellipsized_flex_label_shrinks_without_losing_source_or_accessibility() {
        use crate::{
            AccessibilityMeta, AccessibilityRole, LayoutStyle, PaintKind, UiContent, UiDocument,
            UiNode,
        };
        let source = "A very long editor object name with combining e\u{301} characters";
        let mut document = UiDocument::new(LayoutStyle::row().with_width(100.0).with_height(24.0));
        let root = document.root;
        let label = document.add_child(
            root,
            UiNode::text(
                "name",
                source,
                TextStyle::default().ellipsis(),
                LayoutStyle::default(),
            )
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Label).label(source)),
        );
        let mut measurer = CosmicTextMeasurer::new();
        document
            .compute_layout(UiSize::new(100.0, 24.0), &mut measurer)
            .unwrap();
        let node = document.node(label);
        assert!(node.layout().rect.width <= 100.0);
        let UiContent::Text(content) = node.content() else {
            panic!("missing text")
        };
        assert_eq!(content.text, source);
        assert!(
            measurer
                .fit_text(content, node.layout().rect.width)
                .truncated
        );
        assert_eq!(
            document
                .accessibility_tree()
                .iter()
                .find(|node| node.id == label)
                .unwrap()
                .label
                .as_deref(),
            Some(source)
        );
        let paint = document.paint_list();
        let rendered = paint
            .items
            .iter()
            .find_map(|item| match &item.kind {
                PaintKind::Text(text) if item.node == label => Some(text),
                _ => None,
            })
            .unwrap();
        assert_eq!(rendered.text, source);
        assert_eq!(rendered.style.overflow, crate::TextOverflow::Ellipsis);
        let trace = crate::debug::DebugTextLayoutTrace::from_document(
            &document,
            &mut measurer,
            Some("name"),
        )
        .unwrap();
        assert!(
            !trace.horizontal_overflow,
            "intentional ellipsis is not a text overflow defect"
        );
    }

    #[test]
    fn ellipsized_underlines_stay_inside_the_content_box() {
        use crate::{LayoutStyle, PaintKind, UiDocument, UiNode};
        let mut measurer = CosmicTextMeasurer::new();
        for width in [2.0, 32.0, 90.0] {
            let mut document = UiDocument::new(LayoutStyle::size(100.0, 24.0));
            let node = document.add_child(
                document.root,
                UiNode::text(
                    "underlined",
                    "A much longer underlined label",
                    TextStyle {
                        underline: true,
                        ..TextStyle::default().ellipsis()
                    },
                    LayoutStyle::size(width, 24.0),
                ),
            );
            document
                .compute_layout(UiSize::new(100.0, 24.0), &mut measurer)
                .unwrap();
            let bounds = document.node(node).layout().rect;
            let paint = document.paint_list();
            let (from, to) = paint
                .items
                .iter()
                .find_map(|item| match item.kind {
                    PaintKind::Line { from, to, .. } if item.node == node => Some((from, to)),
                    _ => None,
                })
                .expect("underline paint");
            assert!(from.x >= bounds.x && to.x <= bounds.right());
        }
    }
}
