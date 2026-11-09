"""Generate MIT vertical-flow fixtures with fontTools 4.65.0.

Run from the repository root. Tests use committed bytes and need no fontTools
or installed font. Every variant has identical outlines and horizontal metrics.
"""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTCollection

ROOT = Path(__file__).parent
GLYPHS = [".notdef", "A", "B", "comma", "bracket", "space", "acute", "zerowidth"]

def build(vertical):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(GLYPHS)
    fb.setupCharacterMap({65: "A", 66: "B", 0x3001: "comma", 0x300C: "bracket", 32: "space", 0x301: "acute", 0x200B: "zerowidth"})
    rectangles = {"A": (50, 0, 450, 700), "B": (100, -100, 800, 500),
                  "comma": (600, 600, 800, 800), "bracket": (100, 100, 300, 900), "acute": (100, 700, 200, 800)}
    outlines = {}
    for name in GLYPHS:
        pen = TTGlyphPen(None)
        if name in rectangles:
            x0, y0, x1, y1 = rectangles[name]
            pen.moveTo((x0, y0)); pen.lineTo((x1, y0))
            pen.lineTo((x1, y1)); pen.lineTo((x0, y1)); pen.closePath()
        outlines[name] = pen.glyph()
    fb.setupGlyf(outlines)
    fb.setupHorizontalMetrics({name: (0 if name in ["acute", "zerowidth"] else 600 if name in ["A", "space"] else 1000, 0 if name not in rectangles else rectangles[name][0]) for name in GLYPHS})
    fb.setupHorizontalHeader(ascent=800, descent=-200, lineGap=0)
    if vertical:
        fb.setupVerticalMetrics({name: (1200, 100) if name == "A" else (800, 50) if name == "B" else (1000, 200) for name in GLYPHS})
        fb.setupVerticalHeader(ascent=500, descent=-500, lineGap=0)
    fb.setupNameTable({"familyName": "Fontcore Synthetic Vertical", "styleName": "Regular",
                       "uniqueFontIdentifier": "Fontcore-Vertical-1", "fullName": "Fontcore Synthetic Vertical",
                       "psName": "FontcoreSyntheticVertical"})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    fb.font["head"].created = fb.font["head"].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb

build(False).save(ROOT / "vertical-no-metrics.ttf")
with_metrics = build(True)
with_metrics.save(ROOT / "vertical-metrics.ttf")
del with_metrics.font["vmtx"]
with_metrics.save(ROOT / "vertical-header-only.ttf")

gap = build(False).font
gap["hhea"].lineGap = 200
zero_extent = build(False).font
zero_extent["hhea"].ascent = zero_extent["hhea"].descent = 0
zero_extent["hhea"].lineGap = 1000
negative_extent = build(False).font
negative_extent["hhea"].ascent = -100
negative_extent["hhea"].descent = 100
negative_extent["hhea"].lineGap = 1200
zero_advance = build(True).font
zero_advance["vmtx"].metrics["A"] = (0, 100)
collection = TTCollection()
collection.fonts = [build(False).font, build(True).font, gap, zero_extent,
                    negative_extent, zero_advance]
collection.save(ROOT / "vertical-mixed.ttc")
