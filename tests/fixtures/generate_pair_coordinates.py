"""Generate MIT PairPos coordinate cases with fontTools 4.65.0.

Reads only vertical-metrics.ttf. Face 0 uses glyph pairs (format 1), face 1
uses class pairs (format 2). Positive, negative, and zero Y values occur in
both value records. X adjustments are distinct sentinels. kern and vkrn
reference the same lookup; hmtx/vmtx metrics deliberately differ.
"""
from pathlib import Path
from fontTools.ttLib import TTFont, TTCollection
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

ROOT = Path(__file__).parent


def build(classes):
    font = TTFont(ROOT / "vertical-metrics.ttf", recalcTimestamp=False)
    left, right = ("[A comma]", "[B bracket]") if classes else ("A", "B")
    addOpenTypeFeaturesFromString(font, f"""
        languagesystem DFLT dflt;
        lookup pairs {{
            pos {left} <20 100 -40 120> {right} <-30 -200 60 -80>;
            pos {right} <-20 -100 40 -120> {left} <30 200 -60 80>;
            pos {left} <10 0 20 0> {left} <-10 0 -20 0>;
        }} pairs;
        feature kern {{ lookup pairs; }} kern;
        feature vkrn {{ lookup pairs; }} vkrn;
    """)
    formats = [sub.Format for lookup in font["GPOS"].table.LookupList.Lookup
               for sub in lookup.SubTable]
    assert formats and all(value == (2 if classes else 1) for value in formats), formats
    return font


collection = TTCollection()
collection.fonts = [build(False), build(True)]
collection.save(ROOT / "pair-coordinates.ttc")
