//! Platform-neutral composition and editable-text snapshots.

use std::ops::Range;

use crate::{TextStyle, UiNodeId, UiPoint, UiRect};

/// The authored content and font metrics of an editor in this document.
/// Publish these together so pointer edits use the style that was displayed,
/// even if application styling changes before the edit is applied.
#[derive(Debug, Clone, PartialEq)]
pub struct TextInputContent {
    pub node: UiNodeId,
    pub text_style: TextStyle,
    /// One displayed mask character per Unicode scalar in the editing model.
    /// Pointer selection still uses the original text's grapheme boundaries.
    pub mask: Option<char>,
}

/// A pointer resolved into an editor's authored content coordinates. Runtime
/// routing removes layout offsets, UI scaling, and the content paint transform.
#[derive(Debug, Clone, PartialEq)]
pub struct TextInputPointerGeometry {
    pub point: UiPoint,
    pub bounds: UiRect,
    /// The displayed style, when the owner publishes `TextInputContent`.
    /// Editors without document content metadata use their supplied options.
    pub text_style: Option<TextStyle>,
    /// The mask used by the displayed content, independent of IME sensitivity.
    pub mask: Option<char>,
}

/// An edit from an input method. All offsets are UTF-8 byte offsets, never
/// character counts or UTF-16 units. Platform adapters convert at their boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextCompositionEvent {
    /// Replace the draft, without changing committed text or undo history.
    /// `selection` is relative to `text`; None hides the composition caret.
    /// `replacement` addresses committed text. None retains the current marked
    /// range, or starts at the editor's selection when no composition exists.
    Preedit {
        text: String,
        selection: Option<Range<usize>>,
        replacement: Option<Range<usize>>,
    },
    /// Replace the marked range (or current selection) as one undoable edit.
    /// An explicit replacement range addresses committed text.
    Commit {
        text: String,
        replacement: Option<Range<usize>>,
    },
    /// Discard the draft and preserve the original text and selection.
    Cancel,
}

/// The text an input method sees for one editable node. Widgets publish this
/// alongside their visual content; custom editors can do the same.
///
/// Text includes the active draft. Selection offsets are UTF-8 bytes, with the
/// anchor at `start` and caret at `end` (a reversed selection is allowed).
/// `cursor_rect` is relative to the node's `text_input_content` node when one
/// is set, or to the owner otherwise, in authored UI units.
#[derive(Debug, Clone, PartialEq)]
pub struct TextInputSnapshot {
    pub text: String,
    pub selection: Range<usize>,
    pub composition: Option<Range<usize>>,
    pub cursor_rect: UiRect,
    pub multiline: bool,
    /// Passwords must not be exposed through text suggestions or learning.
    pub sensitive: bool,
}

impl TextInputSnapshot {
    pub fn new(text: impl Into<String>, selection: Range<usize>, cursor_rect: UiRect) -> Self {
        let text = text.into();
        let selection = clamp_range(&text, selection);
        Self {
            text,
            selection,
            composition: None,
            cursor_rect,
            multiline: false,
            sensitive: false,
        }
    }
}

pub(crate) fn clamp_offset(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

pub(crate) fn clamp_range(text: &str, range: Range<usize>) -> Range<usize> {
    clamp_offset(text, range.start)..clamp_offset(text, range.end)
}

#[cfg(feature = "widgets")]
pub(crate) fn ordered_range(text: &str, range: Range<usize>) -> Range<usize> {
    let range = clamp_range(text, range);
    range.start.min(range.end)..range.start.max(range.end)
}

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn byte_to_utf16(text: &str, offset: usize) -> usize {
    text[..clamp_offset(text, offset)].encode_utf16().count()
}

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn utf16_to_byte(text: &str, offset: usize) -> usize {
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        units += ch.len_utf16();
        if units > offset {
            return byte;
        }
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_offsets_round_trip_unicode_boundaries_and_clamp_partial_encodings() {
        for text in ["", "ab", "é候😀z", "a\u{301}👩‍👧\n好"] {
            for offset in 0..=text.len() + 3 {
                let byte = utf16_to_byte(text, byte_to_utf16(text, offset));
                assert_eq!(byte, clamp_offset(text, offset));
            }
            let mut utf16 = 0;
            for (byte, ch) in text.char_indices() {
                for unit in utf16..utf16 + ch.len_utf16() {
                    assert_eq!(utf16_to_byte(text, unit), byte);
                }
                utf16 += ch.len_utf16();
            }
            assert_eq!(utf16_to_byte(text, usize::MAX), text.len());
        }
    }
}
