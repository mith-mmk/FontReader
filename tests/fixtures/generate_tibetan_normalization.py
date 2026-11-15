"""Original MIT Tibetan normalization fixtures; fontTools 4.68.0.

No external input. Fixed timestamps produce byte-identical TTC bytes.
Faces 0..7 are candidates; faces 8..15 omit Tibetan presentation as controls.
"""
from copy import deepcopy
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection

GLYPHS = ['.notdef', 'base', 'sub', 'aa', 'u', 'top', 'compound', 'prep', 'stack', 'triple', 'long', 'space']
COMPOUNDS = [0xF43, 0xF4D, 0xF52, 0xF57, 0xF5C, 0xF69, 0xF73, 0xF75, 0xF76, 0xF77, 0xF78, 0xF79, 0xF81, 0xF93, 0xF9D, 0xFA2, 0xFA7, 0xFAC, 0xFB9]
SYSTEMS = 'languagesystem DFLT dflt; languagesystem tibt dflt; languagesystem tibt JAN;'
GDEF = 'table GDEF { GlyphClassDef [base compound prep stack triple long space], , [sub aa u top], ; } GDEF;'
PREP = 'feature ccmp { script tibt; sub base by prep; } ccmp;'
PRESENT = 'feature blws { script tibt; sub prep aa u by long; sub prep sub sub by triple; sub prep sub by stack; sub prep aa by stack; sub prep u by stack; } blws;'


def build(index):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(GLYPHS)
    cmap = {32: 'space', 65: 'base', 0xF0B: 'space', 0xF0D: 'space', 0xF71: 'aa', 0xF74: 'u', 0xF72: 'top', 0xF80: 'top'}
    cmap.update({cp: 'base' for cp in range(0xF40, 0xF6D)})
    cmap.update({cp: 'sub' for cp in range(0xF90, 0xFBD)})
    cmap.update({cp: 'compound' for cp in COMPOUNDS})
    if index == 1: cmap.pop(0xF71)
    if index == 2: cmap.pop(0xF43)
    if index == 6:
        cmap.pop(0xF75); cmap.pop(0xF71)
    fb.setupCharacterMap(cmap)
    outlines = {}
    for i, name in enumerate(GLYPHS):
        pen = TTGlyphPen(None)
        if name != 'space':
            width = 100 + i * 10 + (index * 100 if index in [1, 2, 6] else 0)
            pen.moveTo((0, 0)); pen.lineTo((width, 0))
            pen.lineTo((width, 400)); pen.lineTo((0, 400)); pen.closePath()
        outlines[name] = pen.glyph()
    fb.setupGlyf(outlines)
    fb.setupHorizontalMetrics({g: (0 if g in ['sub', 'aa', 'u', 'top'] else 600, 0) for g in GLYPHS})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({'familyName': 'MIT Tibetan Normalization Test', 'styleName': 'Regular',
                      'uniqueFontIdentifier': 'MIT-Tibetan-Normalization-Test',
                      'fullName': 'MIT Tibetan Normalization Test', 'psName': 'MITTibetanNormalizationTest'})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    prep, present = PREP, PRESENT
    if index == 3:
        present = 'feature blws { script tibt; sub prep sub by .notdef; sub prep aa u by .notdef; } blws;'
    if index == 4:
        prep = 'feature ccmp { script tibt; sub base by prep prep; } ccmp;'
    if index == 5:
        present = 'feature abvs { script tibt; sub sub by sub; } abvs;'
    if index == 7:
        present = 'feature blws { script tibt; lookupflag IgnoreMarks; sub prep sub by stack; sub prep aa u by long; } blws;'
    addOpenTypeFeaturesFromString(fb.font, SYSTEMS + GDEF + prep + present)
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font


faces = [build(i) for i in range(8)]
controls = []
for font in faces:
    control = deepcopy(font)
    gsub = control['GSUB'].table
    for record in gsub.FeatureList.FeatureRecord:
        if record.FeatureTag in ['blws', 'abvs']:
            record.Feature.LookupListIndex = []
            record.Feature.LookupCount = 0
    controls.append(control)
collection = TTCollection()
collection.fonts = faces + controls
collection.save(Path(__file__).with_name('tibetan-normalization.ttc'))
