"""Original MIT synthetic canonical kana fixtures; fontTools 4.68.0.
No external font input. Unicode 17.0 canonical pairs, fixed timestamps.
Faces: 0 both forms, 1 composed only, 2 original only, 3 .notdef targets,
4 ccmp/calt/vert, 5 missing ccmp output, 6 mixed contextual rollback,
7 distinct fallback geometry, 8 incomplete original coverage,
9 mixed rollback with composed-only hih neighbor.
"""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection

# (composed, base, mark), canonical mappings from UnicodeData 17.0.0.
PAIRS = [
    (0x304C, 0x304B, 0x3099),
    (0x304E, 0x304D, 0x3099),
    (0x3050, 0x304F, 0x3099),
    (0x3052, 0x3051, 0x3099),
    (0x3054, 0x3053, 0x3099),
    (0x3056, 0x3055, 0x3099),
    (0x3058, 0x3057, 0x3099),
    (0x305A, 0x3059, 0x3099),
    (0x305C, 0x305B, 0x3099),
    (0x305E, 0x305D, 0x3099),
    (0x3060, 0x305F, 0x3099),
    (0x3062, 0x3061, 0x3099),
    (0x3065, 0x3064, 0x3099),
    (0x3067, 0x3066, 0x3099),
    (0x3069, 0x3068, 0x3099),
    (0x3070, 0x306F, 0x3099),
    (0x3071, 0x306F, 0x309A),
    (0x3073, 0x3072, 0x3099),
    (0x3074, 0x3072, 0x309A),
    (0x3076, 0x3075, 0x3099),
    (0x3077, 0x3075, 0x309A),
    (0x3079, 0x3078, 0x3099),
    (0x307A, 0x3078, 0x309A),
    (0x307C, 0x307B, 0x3099),
    (0x307D, 0x307B, 0x309A),
    (0x3094, 0x3046, 0x3099),
    (0x309E, 0x309D, 0x3099),
    (0x30AC, 0x30AB, 0x3099),
    (0x30AE, 0x30AD, 0x3099),
    (0x30B0, 0x30AF, 0x3099),
    (0x30B2, 0x30B1, 0x3099),
    (0x30B4, 0x30B3, 0x3099),
    (0x30B6, 0x30B5, 0x3099),
    (0x30B8, 0x30B7, 0x3099),
    (0x30BA, 0x30B9, 0x3099),
    (0x30BC, 0x30BB, 0x3099),
    (0x30BE, 0x30BD, 0x3099),
    (0x30C0, 0x30BF, 0x3099),
    (0x30C2, 0x30C1, 0x3099),
    (0x30C5, 0x30C4, 0x3099),
    (0x30C7, 0x30C6, 0x3099),
    (0x30C9, 0x30C8, 0x3099),
    (0x30D0, 0x30CF, 0x3099),
    (0x30D1, 0x30CF, 0x309A),
    (0x30D3, 0x30D2, 0x3099),
    (0x30D4, 0x30D2, 0x309A),
    (0x30D6, 0x30D5, 0x3099),
    (0x30D7, 0x30D5, 0x309A),
    (0x30D9, 0x30D8, 0x3099),
    (0x30DA, 0x30D8, 0x309A),
    (0x30DC, 0x30DB, 0x3099),
    (0x30DD, 0x30DB, 0x309A),
    (0x30F4, 0x30A6, 0x3099),
    (0x30F7, 0x30EF, 0x3099),
    (0x30F8, 0x30F0, 0x3099),
    (0x30F9, 0x30F1, 0x3099),
    (0x30FA, 0x30F2, 0x3099),
    (0x30FE, 0x30FD, 0x3099),
]

def name(cp):
    return f'u{cp:04X}'

SCALARS = sorted({cp for p in PAIRS for cp in p} | set(range(0x31F0, 0x3200)) |
                 {32, 65, 66, 0x301, 0x34F, 0x600, 0x200C, 0x200D, 0x30BB,
                  0x30C4, 0x30C8, 0xFF8A, 0xFF9E, 0xFF9F,
                  0x1100, 0x1161, 0x11A8, 0x1112, 0x1175, 0x11C2, 0xAC01, 0xD7A3})
GLYPHS = ['.notdef'] + [name(cp) for cp in SCALARS] + ['composed', 'contextual', 'vertical']


def build(index):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(GLYPHS)
    cmap = {cp: name(cp) for cp in SCALARS}
    targets = {p[0] for p in PAIRS}
    if index == 1:
        cmap = {cp: g for cp, g in cmap.items() if cp not in {p[1] for p in PAIRS} | {0x3099, 0x309A}}
    if index in (2, 8):
        for cp in targets: cmap.pop(cp)
    if index == 3:
        for cp in targets: cmap[cp] = '.notdef'
    if index == 8: cmap.pop(0x309A)
    if index == 9:
        for cp in (0x1112, 0x1175, 0x11C2): cmap.pop(cp)
    fb.setupCharacterMap(cmap)
    outlines = {}
    for i, glyph in enumerate(GLYPHS):
        pen = TTGlyphPen(None)
        if glyph != name(32):
            width = 90 + i * 2 + (75 if index == 7 and glyph not in [name(65), name(66), name(32)] else 0)
            pen.moveTo((0, 0)); pen.lineTo((width, 0))
            pen.lineTo((width, 400 + i)); pen.lineTo((0, 400)); pen.closePath()
        outlines[glyph] = pen.glyph()
    fb.setupGlyf(outlines)
    # Deliberately nonzero mark advance: composition must fix the canonical
    # pair without globally zeroing Ainu or other unligated marks.
    fb.setupHorizontalMetrics({g: (500 + i, 0) for i, g in enumerate(GLYPHS)})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupVerticalMetrics({g: (1000 + i, 80) for i, g in enumerate(GLYPHS)})
    fb.setupVerticalHeader(ascent=800, descent=-200)
    fb.setupNameTable({'familyName': 'MIT Kana Composition Test', 'styleName': 'Regular',
                      'uniqueFontIdentifier': 'MIT-Kana-Composition-Test',
                      'fullName': 'MIT Kana Composition Test', 'psName': 'MITKanaCompositionTest'})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    features = None
    if index == 4:
        features = """languagesystem DFLT dflt; languagesystem kana dflt;
        languagesystem kana JAN;
        feature ccmp { sub u30D1 by composed; } ccmp;
        feature calt { sub u0041 composed' u0042 by contextual; } calt;
        feature vert { sub composed by vertical; sub contextual by vertical; } vert;"""
    if index == 5:
        features = 'languagesystem DFLT dflt; feature ccmp { sub u30D1 by .notdef; } ccmp;'
    if index in (6, 9):
        features = """languagesystem DFLT dflt;
        feature calt {
          sub u0041 u30D1' u0042 by .notdef;
          sub uAC01 u30D1' by .notdef;
          sub u30D1 uAC01' by .notdef;
        } calt;"""
    if features: addOpenTypeFeaturesFromString(fb.font, features)
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font


if __name__ == '__main__':
    collection = TTCollection()
    collection.fonts = [build(i) for i in range(10)]
    collection.save(Path(__file__).with_name('kana-composition.ttc'))
