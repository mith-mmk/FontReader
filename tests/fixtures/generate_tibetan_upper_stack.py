"""Original MIT Tibetan upper-stack fixtures; fontTools 4.68.0, no external input.
Faces 0..23 and corresponding controls 24..47.
"""
from copy import deepcopy
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection

ROOT = Path(__file__).parent
GLYPHS = ['.notdef', 'base', 'sub', 'top', 'bottom', 'prep', 'stage', 'stack', 'deep', 'topalt', 'other', 'space']
SYSTEMS = 'languagesystem DFLT dflt; languagesystem tibt dflt; languagesystem tibt JAN; languagesystem latn dflt;'
GDEF = 'table GDEF { GlyphClassDef [base prep stage stack deep space], , [sub top bottom topalt other], ; } GDEF;'
ANCHORS = '''markClass top <anchor 100 700> @TOP;
markClass topalt <anchor 100 700> @TOP;
feature abvm { script tibt; pos base [base prep stage stack deep] <anchor 350 800> mark @TOP; } abvm;'''


def build(features, missing=None, no_gdef=False):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(GLYPHS)
    cmap = {32: 'space', 65: 'base', 66: 'sub', 0x301: 'top', 0x323: 'bottom'}
    cmap.update({cp: 'base' for cp in range(0xF40, 0xF70)})
    cmap.update({cp: 'sub' for cp in range(0xF90, 0xFBD)})
    cmap.update({cp: 'top' for cp in [0xF72, 0xF7A, 0xF7B, 0xF7C, 0xF7D, 0xF80]})
    cmap.update({cp: 'bottom' for cp in [0xF71, 0xF74]})
    cmap.update({cp: 'other' for cp in [0xF73, 0xF75, 0xF76, 0xF77, 0xF78, 0xF79, 0xF81, 0x200C, 0x200D, 0x34F]})
    if missing is not None:
        cmap.pop(missing)
    fb.setupCharacterMap(cmap)
    outlines = {}
    for i, name in enumerate(GLYPHS):
        pen = TTGlyphPen(None)
        if name != 'space':
            pen.moveTo((0, 0)); pen.lineTo((100 + i * 10, 0))
            pen.lineTo((100 + i * 10, 400)); pen.lineTo((0, 400)); pen.closePath()
        outlines[name] = pen.glyph()
    fb.setupGlyf(outlines)
    fb.setupHorizontalMetrics({g: (0 if g in ['sub', 'top', 'bottom', 'topalt', 'other'] else 600, 0) for g in GLYPHS})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({'familyName': 'MIT Tibetan Stack Test', 'styleName': 'Regular',
                      'uniqueFontIdentifier': 'MIT-Tibetan-Stack-Test',
                      'fullName': 'MIT Tibetan Stack Test', 'psName': 'MITTibetanStackTest'})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    addOpenTypeFeaturesFromString(fb.font, SYSTEMS + GDEF + ANCHORS + features)
    if no_gdef:
        del fb.font['GDEF']
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font

from fontTools.ttLib.tables import otTables
STACK = 'feature blws { script tibt; sub base sub sub by deep; sub base sub by stack; } blws; feature abvs { script tibt; sub top by topalt; } abvs;'
faces = [build(STACK)]
f = build(STACK)
for lookup in f['GPOS'].table.LookupList.Lookup:
    typ = lookup.LookupType
    wrapped = []
    for subtable in lookup.SubTable:
        ext = otTables.ExtensionPos(); ext.Format = 1; ext.ExtensionLookupType = typ; ext.ExtSubTable = subtable
        wrapped.append(ext)
    lookup.LookupType = 9; lookup.SubTable = wrapped
faces.append(f) # 1: extension anchors
faces.append(build(STACK + 'feature mark { script tibt; pos base stack <anchor 550 600> mark @TOP; } mark;')) # 2: mark priority
f = build(STACK); del f['GPOS']; faces.append(f) # 3
faces.append(build(STACK, no_gdef=True)) # 4
for glyph, classification in [('stack', 2), ('stack', None), ('topalt', None), ('topalt', 1)]:
    f = build(STACK)
    if classification is None: del f['GDEF'].table.GlyphClassDef.classDefs[glyph]
    else: f['GDEF'].table.GlyphClassDef.classDefs[glyph] = classification
    faces.append(f) # 5..8: no explicit Base+Mark proof
f = build(STACK)
s = f['GPOS'].table.LookupList.Lookup[0].SubTable[0]
i = s.BaseCoverage.glyphs.index('stack'); s.BaseCoverage.glyphs.pop(i); s.BaseArray.BaseRecord.pop(i); s.BaseArray.BaseCount -= 1
faces.append(f) # 9: no actual stack anchor
f = build(STACK); f['GPOS'].table.FeatureList.FeatureRecord[0].FeatureTag = 'blwm'; faces.append(f) # 10: wrong feature
f = build(STACK); f['GPOS'].table.LookupList.Lookup[0].LookupFlag = 8; faces.append(f) # 11: IgnoreMarks
faces.append(build(STACK, missing=0xF90)) # 12: missing original
faces.append(build('feature blws { script tibt; sub base sub by .notdef; } blws;')) # 13
faces.append(build(STACK + 'feature calt { script tibt; sub topalt by .notdef; } calt;')) # 14
faces.append(build('feature abvs { script tibt; sub top by topalt; } abvs;')) # 15: unresolved subjoined
faces.append(build('feature ccmp { script tibt; sub base by base base; } ccmp;' + STACK)) # 16: expansion
faces.append(build(STACK + 'feature liga { script tibt; sub stack topalt base by deep; } liga;')) # 17: cross cluster
faces.append(build('feature ccmp { script tibt; sub base sub by stack; } ccmp; feature blws { script tibt; sub other other by deep; } blws;')) # 18: no presentation activation
faces.append(build(STACK + 'feature calt { script tibt; sub topalt by topalt topalt; } calt;')) # 19: expanded upper
faces.append(build(STACK + 'feature calt { script tibt; sub stack topalt by deep; } calt;')) # 20: no remaining upper
f = build(STACK + 'feature mark { script tibt; pos ligature stack <anchor 900 900> mark @TOP ligComponent <anchor 950 950> mark @TOP; } mark;')
f['GDEF'].table.GlyphClassDef.classDefs['stack'] = 1
faces.append(f) # 21: type5 must not guess a component for explicit Base
faces.append(build(STACK, missing=0xF72)) # 22: missing original upper
faces.append(build('feature ccmp { script tibt; sub base by base base; } ccmp; feature blws { script tibt; sub base base sub by stack; } blws;')) # 23: expand then recombine
assert len(faces) == 24
controls = [deepcopy(f) for f in faces]
for f in controls:
    for feature in f['GSUB'].table.FeatureList.FeatureRecord:
        if feature.FeatureTag in ['abvs', 'blws']:
            feature.Feature.LookupListIndex = []; feature.Feature.LookupCount = 0
collection = TTCollection(); collection.fonts = faces + controls
collection.save(ROOT / 'tibetan-upper-stack.ttc')
