These synthetic TrueType fixtures contain one triangular `A` glyph. They share
the same family, metrics, character mapping, and glyph index but have different
outlines, making stale rasterized glyphs observable when a font library changes.
They contain no third-party font data and use the repository's license.

Regenerate them with `python3 tests/fixtures/fonts/generate.py` and fontTools.
Rust tests embed the generated files and do not require Python or fontTools.
