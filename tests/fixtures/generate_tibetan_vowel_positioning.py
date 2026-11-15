"""MIT original synthetic Tibetan GPOS fixtures; fontTools 4.68.0.

Run from any directory. No external font inputs or other generated outputs.
Faces 0..25 are independent cases; faces 26..51 are their GPOS-free controls.
All timestamps are fixed, so repeated runs produce byte-identical TTC bytes.
"""
from copy import deepcopy
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection

ROOT = Path(__file__).parent
GLYPHS = ['.notdef', 'base', 'top', 'bottom', 'other', 'space']
GDEF = 'table GDEF { GlyphClassDef [base space], , [top bottom other], ; } GDEF;'
CLASSES = '''markClass top <anchor 100 700> @TOP;
markClass bottom <anchor 50 -100> @BOTTOM;
markClass other <anchor 0 0> @OTHER;'''
SYSTEMS = 'languagesystem DFLT dflt; languagesystem tibt dflt;'


def fallback(flag='', extension=False):
    return f'''lookup above {'useExtension' if extension else ''} {{
        {flag} pos base base <anchor 350 800> mark @TOP;
    }} above;
    lookup below {'useExtension' if extension else ''} {{
        {flag} pos base base <anchor 250 -200> mark @BOTTOM;
    }} below;
    feature abvm {{ script tibt; lookup above; }} abvm;
    feature blwm {{ script tibt; lookup below; }} blwm;'''


def build(features, no_gdef=False):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(GLYPHS)
    cmap = {32: 'space', 65: 'base', 0x301: 'top', 0x323: 'bottom'}
    cmap.update({cp: 'base' for cp in range(0xF40, 0xF70)})
    cmap.update({cp: 'top' for cp in [0xF72, 0xF7A, 0xF7B, 0xF7C, 0xF7D, 0xF80]})
    cmap.update({cp: 'bottom' for cp in [0xF71, 0xF74]})
    # Unsupported vowels deliberately alias an attachable glyph to test Unicode gating.
    cmap.update({cp: 'top' for cp in [0xF73, 0xF75, 0xF76, 0xF77, 0xF78, 0xF79, 0xF81]})
    cmap[0xF90] = 'other'
    fb.setupCharacterMap(cmap)
    outlines = {}
    for name in GLYPHS:
        pen = TTGlyphPen(None)
        if name != 'space':
            pen.moveTo((50, 0)); pen.lineTo((350, 0))
            pen.lineTo((350, 400)); pen.lineTo((50, 400)); pen.closePath()
        outlines[name] = pen.glyph()
    fb.setupGlyf(outlines)
    fb.setupHorizontalMetrics({g: (0 if g in ['top', 'bottom', 'other'] else 600, 50) for g in GLYPHS})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({'familyName': 'MIT Tibetan Vowel Test', 'styleName': 'Regular',
                      'uniqueFontIdentifier': 'MIT-Tibetan-Vowel-Test',
                      'fullName': 'MIT Tibetan Vowel Test', 'psName': 'MITTibetanVowelTest'})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    addOpenTypeFeaturesFromString(fb.font, SYSTEMS + GDEF + CLASSES + features)
    if no_gdef:
        del fb.font['GDEF']
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font

faces = [
    build(fallback()),  # 0: abvm and blwm
    build(fallback() + 'feature mark { pos base base <anchor 550 600> mark @TOP; } mark;'),
    build(fallback() + 'feature mkmk { pos mark top <anchor 180 900> mark @TOP; } mkmk;'),
    build('feature abvm { script DFLT; pos base base <anchor 950 1000> mark @TOP; } abvm;'),
    build('''feature abvm {
        script DFLT; pos base base <anchor 950 1000> mark @TOP;
        script tibt; language dflt; pos base base <anchor 350 800> mark @TOP;
        language JAN exclude_dflt; pos base base <anchor 750 1000> mark @TOP;
    } abvm;'''),
    build(fallback('lookupflag IgnoreMarks;')),
    build(fallback('lookupflag IgnoreBaseGlyphs;')),
    build(fallback('lookupflag IgnoreLigatures;')),
    build(fallback('lookupflag UseMarkFilteringSet [top bottom];')),
    build(fallback('lookupflag UseMarkFilteringSet [other];')),
    build(fallback('lookupflag MarkAttachmentType [top bottom];')),
    build(fallback('lookupflag MarkAttachmentType [other];')),
    build(fallback('lookupflag IgnoreLigatures;'), no_gdef=True),
    build(fallback(), no_gdef=True),
    build(fallback(extension=True)),
    build(fallback() + 'feature liga { sub base base by base; } liga;'),
]
# 16,17: GDEF exists but one participant is unclassified.
for glyph in ['base', 'top']:
    font = build(fallback('lookupflag IgnoreLigatures;'))
    del font['GDEF'].table.GlyphClassDef.classDefs[glyph]
    faces.append(font)
# 18: GDEF itself is present but the glyph class definition is absent.
font = build(fallback('lookupflag IgnoreLigatures;'))
font['GDEF'].table.GlyphClassDef = None
faces.append(font)
# 19: a mark glyph created from a ligature must not enter the simple-vowel path.
faces.append(build(fallback() + 'feature liga { sub top bottom by top; } liga;'))
# 20,21: filtering metadata absent despite valid glyph classes.
font = build(fallback('lookupflag UseMarkFilteringSet [top bottom];'))
font['GDEF'].table.MarkGlyphSetsDef = None
faces.append(font)
font = build(fallback('lookupflag MarkAttachmentType [top bottom];'))
font['GDEF'].table.MarkAttachClassDef = None
faces.append(font)
# 22,23: extension-wrapped filtered lookups, rejected and accepted respectively.
faces.append(build(fallback('lookupflag IgnoreMarks;', extension=True)))
faces.append(build(fallback('lookupflag UseMarkFilteringSet [top bottom];', extension=True)))
# 24: an unrelated script cannot supply Tibetan fallback.
faces.append(build('feature abvm { script latn; pos base base <anchor 950 1000> mark @TOP; } abvm;'))
# 25: the first subtable misses; the second must still be tried.
font = build(fallback())
lookup = font['GPOS'].table.LookupList.Lookup[0]
miss = deepcopy(lookup.SubTable[0])
miss.BaseCoverage.glyphs = ['space']
lookup.SubTable.insert(0, miss)
lookup.SubTableCount = len(lookup.SubTable)
faces.append(font)
controls = [deepcopy(font) for font in faces]
for font in controls:
    del font['GPOS']
collection = TTCollection()
collection.fonts = faces + controls
collection.save(ROOT / 'tibetan-vowel-positioning.ttc')
