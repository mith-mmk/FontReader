"""Original MIT synthetic modern Hangul fixtures; fontTools 4.68.0.

No external font input. Fixed timestamps give reproducible TTC bytes.
Faces: 0 both forms, 1 composed only, 2 jamo only, 3 LV only, 4 LVT only,
5 incomplete original coverage, 6 GSUB stages, 7 .notdef syllable mapping,
8 distinct fallback geometry, 9 post-GSUB missing glyph,
10 contextual missing glyph plus a composed-only neighboring syllable.
"""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection

GLYPHS = ['.notdef', 'L', 'V', 'T', 'ga', 'gag', 'hih', 'other', 'A', 'B',
          'space', 'mark', 'composed', 'contextual', 'vertical', 'variant']
SYLLABLES = {0xAC00: 'ga', 0xAC01: 'gag', 0xD7A3: 'hih'}


def build(index):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(GLYPHS)
    cmap = {32: 'space', 65: 'A', 66: 'B', 0x301: 'mark', 0x600: 'other',
            0x200C: 'other', 0x200D: 'other', 0x34F: 'mark',
            0x302E: 'mark', 0x302F: 'mark', 0x3131: 'L', 0x314F: 'V'}
    cmap.update({cp: 'L' for cp in range(0x1100, 0x1160)})
    cmap.update({cp: 'V' for cp in range(0x1160, 0x11A8)})
    cmap.update({cp: 'T' for cp in range(0x11A8, 0x1200)})
    cmap.update({0xA960: 'L', 0xD7B0: 'V', 0xD7CB: 'T'})
    cmap.update(SYLLABLES)
    if index == 1:
        cmap = {cp: g for cp, g in cmap.items() if not 0x1100 <= cp <= 0x11FF}
    if index == 2:
        for cp in SYLLABLES: cmap.pop(cp)
    if index == 3:
        cmap.pop(0xAC01); cmap.pop(0xD7A3)
    if index == 4:
        cmap.pop(0xAC00)
    if index == 5:
        cmap.pop(0x1161); cmap.pop(0x11A8); cmap.pop(0xAC01)
    if index == 10:
        cmap.pop(0x1112); cmap.pop(0x1175); cmap.pop(0x11C2)
    if index == 7:
        cmap[0xAC01] = '.notdef'
    fb.setupCharacterMap(cmap)
    outlines = {}
    for i, name in enumerate(GLYPHS):
        pen = TTGlyphPen(None)
        if name != 'space':
            width = 90 + i * 13 + (75 if index == 8 and name not in ['A', 'B', 'space'] else 0)
            pen.moveTo((0, 0)); pen.lineTo((width, 0))
            pen.lineTo((width, 400 + i * 7)); pen.lineTo((0, 400)); pen.closePath()
        outlines[name] = pen.glyph()
    fb.setupGlyf(outlines)
    fb.setupHorizontalMetrics({g: (0 if g == 'mark' else 500 + i * 10, 0)
                               for i, g in enumerate(GLYPHS)})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupVerticalMetrics({g: (1000 + i * 5, 80) for i, g in enumerate(GLYPHS)})
    fb.setupVerticalHeader(ascent=800, descent=-200)
    fb.setupNameTable({'familyName': 'MIT Hangul Composition Test', 'styleName': 'Regular',
                      'uniqueFontIdentifier': 'MIT-Hangul-Composition-Test',
                      'fullName': 'MIT Hangul Composition Test', 'psName': 'MITHangulCompositionTest'})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    if index in (6, 9, 10):
        features = """languagesystem DFLT dflt; languagesystem hang dflt;
        languagesystem hang KOR;
        feature ccmp { sub gag by composed; } ccmp;
        feature calt { sub A composed' B by contextual; } calt;
        feature smcp { sub ga by variant; } smcp;
        feature vert { sub composed by vertical; sub contextual by vertical; } vert;"""
        if index == 9:
            features = 'languagesystem DFLT dflt; feature ccmp { sub gag by .notdef; } ccmp;'
        if index == 10:
            features = "languagesystem DFLT dflt; feature calt { sub A gag' B by .notdef; } calt;"
        addOpenTypeFeaturesFromString(fb.font, features)
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font


collection = TTCollection()
collection.fonts = [build(i) for i in range(11)]
collection.save(Path(__file__).with_name('hangul-composition.ttc'))
