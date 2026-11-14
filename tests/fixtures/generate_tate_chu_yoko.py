"""Generate MIT tate-chu-yoko fixtures using fontTools 4.65.0.

Reuses the committed synthetic vertical-metrics.ttf; no external fonts.
Faces: horizontal kerning/ligatures and vertical-feature sentinels, negative
total advance, nonzero horizontal line gap, and an em/line-height ratio of 2/3.
Source timestamps are retained.
"""
from pathlib import Path
from fontTools.ttLib import TTFont, TTCollection
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

ROOT = Path(__file__).parent


def build(kern=-100, gap=0):
    font = TTFont(ROOT / "vertical-metrics.ttf", recalcTimestamp=False)
    for table in font["cmap"].tables:
        if table.isUnicode():
            table.cmap.update({ord("1"): "A", ord("2"): "B"})
    font["hhea"].lineGap = gap
    addOpenTypeFeaturesFromString(font, f"""
        languagesystem DFLT dflt;
        table GDEF {{ GlyphClassDef [A B comma bracket], , [acute], ; }} GDEF;
        markClass acute <anchor 100 700> @TOP;
        feature mark {{ pos base A <anchor 200 500> mark @TOP; }} mark;
        feature kern {{ pos A B {kern}; }} kern;
        feature liga {{ sub A A by B; }} liga;
        feature vert {{ sub A by comma; }} vert;
        feature vrt2 {{ sub A by comma; }} vrt2;
        feature vrtr {{ sub A by bracket; }} vrtr;
    """)
    return font


collection = TTCollection()
meiryo_ratio = build()
meiryo_ratio["hhea"].ascent = 1100
meiryo_ratio["hhea"].descent = -400
collection.fonts = [build(), build(-2000), build(gap=200), meiryo_ratio]
collection.save(ROOT / "tate-chu-yoko.ttc")
