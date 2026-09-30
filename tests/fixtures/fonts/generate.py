"""Regenerate the synthetic font-replacement fixtures (requires fontTools)."""

from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen


for name, points in (
    ("lower", [(0, 0), (800, 0), (0, 800)]),
    ("upper", [(0, 0), (800, 800), (0, 800)]),
):
    builder = FontBuilder(1000, isTTF=True)
    builder.setupGlyphOrder([".notdef", "A"])
    builder.setupCharacterMap({ord("A"): "A"})
    pen = TTGlyphPen(None)
    pen.moveTo(points[0])
    for point in points[1:]:
        pen.lineTo(point)
    pen.closePath()
    builder.setupGlyf({".notdef": TTGlyphPen(None).glyph(), "A": pen.glyph()})
    builder.setupHorizontalMetrics({".notdef": (1000, 0), "A": (1000, 0)})
    builder.setupHorizontalHeader(ascent=1000, descent=0)
    builder.setupNameTable({
        "familyName": "ReplacementFixture",
        "styleName": "Regular",
        "uniqueFontIdentifier": "ReplacementFixture Regular",
        "fullName": "ReplacementFixture Regular",
        "psName": "ReplacementFixture-Regular",
        "version": "Version 1.0",
    })
    builder.setupOS2(sTypoAscender=1000, sTypoDescender=0, usWinAscent=1000, usWinDescent=0)
    builder.setupPost()
    builder.setupMaxp()
    # Fixed timestamps keep regeneration deterministic.
    builder.font["head"].created = builder.font["head"].modified = 2082844800
    builder.font.recalcTimestamp = False
    builder.save(Path(__file__).with_name(f"replacement-{name}.ttf"))
