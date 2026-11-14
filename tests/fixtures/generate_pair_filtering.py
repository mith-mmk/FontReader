"""MIT synthetic PairPos lookup filtering fixtures, fontTools 4.65.0.
No external font input. Run from any directory. Face order is asserted in tests.
"""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection

ROOT = Path(__file__).parent
GLYPHS = [".notdef", "A", "B", "C", "lig", "acute", "grave"]
FIRST = "<10 20 -30 40>"
SECOND = "<-50 -60 70 -80>"
PAIR = f"pos A {FIRST} B {SECOND};"
GDEF = "table GDEF { GlyphClassDef [A B C], [lig], [acute grave], ; } GDEF;"

def build(body, flag="", classes=False, extension=False, gdef=True):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(GLYPHS)
    fb.setupCharacterMap({65:"A", 66:"B", 67:"C", 0xE000:"lig", 0x301:"acute", 0x300:"grave"})
    outlines = {}
    for name in GLYPHS:
        pen = TTGlyphPen(None)
        pen.moveTo((50, 0)); pen.lineTo((350, 0)); pen.lineTo((350, 400)); pen.lineTo((50, 400)); pen.closePath()
        outlines[name] = pen.glyph()
    fb.setupGlyf(outlines)
    fb.setupHorizontalMetrics({g:(0 if g in ["acute", "grave"] else 500, 50) for g in GLYPHS})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupVerticalMetrics({g:(0 if g in ["acute", "grave"] else 1000, 400) for g in GLYPHS})
    fb.setupVerticalHeader(ascent=800, descent=-200)
    fb.setupNameTable({"familyName":"Pair Filtering", "styleName":"Regular", "uniqueFontIdentifier":"MIT Pair Filtering", "fullName":"Pair Filtering", "psName":"PairFiltering"})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    if classes:
        body = body.replace("pos A ", "pos [A C] ").replace(" B ", " [B lig] ")
    feature = "" if not body else f"lookup pairs {'useExtension' if extension else ''} {{ {flag} {body} }} pairs; feature kern {{ lookup pairs; }} kern; feature vkrn {{ lookup pairs; }} vkrn;"
    addOpenTypeFeaturesFromString(fb.font, "languagesystem DFLT dflt; " + (GDEF if gdef else "") + feature)
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font

fonts = [build(""), build(PAIR), build(PAIR, "lookupflag IgnoreMarks;"),
         build(f"pos acute {FIRST} grave {SECOND};", "lookupflag IgnoreBaseGlyphs;"),
         build(PAIR, "lookupflag IgnoreLigatures;"),
         build(PAIR + f" pos A {FIRST} acute {SECOND};", "lookupflag MarkAttachmentType [acute];"),
         build(PAIR + f" pos A {FIRST} acute {SECOND};", "lookupflag UseMarkFilteringSet [acute];"),
         build(PAIR + f" pos A {FIRST} acute {SECOND};", "lookupflag MarkAttachmentType [grave] UseMarkFilteringSet [acute];"),
         build(PAIR, "lookupflag IgnoreMarks MarkAttachmentType [acute] UseMarkFilteringSet [acute];"),
         build(PAIR, "lookupflag IgnoreMarks;", gdef=False),
         build(PAIR, "lookupflag IgnoreMarks;", extension=True),
         build(PAIR, "lookupflag IgnoreMarks;", classes=True)]
# Independent lookups have different neighbors; both must contribute to A.
f = build("")
addOpenTypeFeaturesFromString(f, GDEF + f"lookup skip {{ lookupflag IgnoreMarks; {PAIR} }} skip; lookup keep {{ pos A <1 2 3 4> acute <5 6 7 8>; }} keep; feature kern {{ lookup skip; lookup keep; }} kern; feature vkrn {{ lookup skip; lookup keep; }} vkrn;")
fonts.append(f)
# A non-empty, all-zero second ValueRecord still consumes the second glyph.
fonts.append(build("pos A <0 0 -10 0> B <0 0 0 0>; pos B <0 0 -20 0> C <0 0 0 0>;"))
for lookup in fonts[-1]['GPOS'].table.LookupList.Lookup:
    for sub in lookup.SubTable:
        sub.ValueFormat2 = 4
        for pair_set in sub.PairSet:
            for pair in pair_set.PairValueRecord:
                from fontTools.ttLib.tables.otBase import ValueRecord
                pair.Value2 = ValueRecord(4); pair.Value2.XAdvance = 0
fonts.append(build("pos A B -10; pos B C -20;"))
# Matching baselines preserve GDEF attachment metadata while removing GPOS.
from copy import deepcopy
baselines = [deepcopy(font) for font in fonts]
for font in baselines:
    if 'GPOS' in font:
        del font['GPOS']
collection = TTCollection(); collection.fonts = fonts + baselines
# Face 30 combines SinglePos, PairPos, and real mark-to-base attachment.
f = build("")
addOpenTypeFeaturesFromString(f, GDEF + """
languagesystem DFLT dflt;
lookup pairs { pos A <10 20 -30 40> acute <-50 -60 70 -80>; } pairs;
feature kern { lookup pairs; pos A <1 2 3 4>; } kern;
feature dist { pos acute <5 6 7 8>; } dist;
markClass acute <anchor 0 0> @TOP;
feature mark { pos base A <anchor 300 500> mark @TOP; } mark;
""")
collection.fonts.append(f)
# Face 31 exercises the unchanged locale/script collector with filtered pairs.
f = build("")
addOpenTypeFeaturesFromString(f, GDEF + """
languagesystem DFLT dflt;
languagesystem arab dflt;
feature kern {
    script DFLT; lookupflag IgnoreMarks; pos A B -10;
    script arab; lookupflag IgnoreMarks; pos A B -90;
} kern;
""")
collection.fonts.append(f)
collection.fonts.append(build(PAIR, "lookupflag IgnoreMarks;", classes=True, extension=True))
collection.save(ROOT / "pair-filtering.ttc")
