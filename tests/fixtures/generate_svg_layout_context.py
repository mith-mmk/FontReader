#!/usr/bin/env python3
"""Generate original MIT fixtures for SVG text layout; requires fontTools."""
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import newTable
from fontTools.ttLib.tables.S_V_G_ import SVGDocument


def document(glyph, body):
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128">'
        f'<g id="glyph{glyph}">{body}</g></svg>'
    )


def build(ascent, cmap, name):
    names = [".notdef", "letterA", "letterB"]
    builder = FontBuilder(128, isTTF=True)
    builder.font.recalcTimestamp = False
    builder.font["head"].created = builder.font["head"].modified = 2082844800
    builder.setupGlyphOrder(names)
    builder.setupCharacterMap(cmap)
    builder.setupGlyf({glyph: TTGlyphPen(None).glyph() for glyph in names})
    builder.setupHorizontalMetrics({glyph: (64, 0) for glyph in names})
    builder.setupHorizontalHeader(ascent=ascent, descent=0)
    builder.setupNameTable({
        "familyName": name,
        "styleName": "Regular",
        "uniqueFontIdentifier": name,
        "fullName": name,
        "psName": name,
    })
    builder.setupOS2(sTypoAscender=ascent, sTypoDescender=0, usWinAscent=ascent, usWinDescent=0)
    builder.setupPost()
    table = newTable("SVG ")
    table.docList = [
        SVGDocument(document(1, '<g opacity=".5"><rect x="8" y="16" width="32" height="32" fill="#2277cc"/></g>'), 1, 1),
        SVGDocument(document(2, '<svg x="0" y="0" width="32" height="32" viewBox="0 0 32 32"><rect width="16" height="16"/></svg>'), 2, 2),
    ]
    builder.font["SVG "] = table
    return builder.font


root = Path(__file__).resolve().parent
build(128, {65: "letterA", 66: "letterB", 0x4E00: "letterA", 0x05D0: "letterA"}, "SvgLayoutPrimary").save(root / "svg-layout-primary.ttf")
build(256, {0x1F600: "letterA"}, "SvgLayoutFallback").save(root / "svg-layout-fallback.ttf")
