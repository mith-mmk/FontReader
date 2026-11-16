"""Original MIT European fallback fixture. fontTools required only to regenerate."""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection

CODES = {ord(c): 'u%04X' % ord(c) for c in 'ao uAOUäöüÄÖÜßẞfiB'.replace(' ', '')}
CODES[0x308] = 'dieresis'
ORDER = ['.notdef', *CODES.values(), 'fi']


def build(index):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(ORDER)
    cmap = dict(CODES)
    if index == 0:
        cmap = {c: g for c, g in cmap.items() if c < 128}
    elif index == 2:
        cmap = {0x308: 'dieresis'}
    fb.setupCharacterMap(cmap)
    glyphs = {}
    for i, g in enumerate(ORDER):
        pen = TTGlyphPen(None)
        x = i * 5 + index * 100
        pen.moveTo((x, 0)); pen.lineTo((x + 80, 0))
        pen.lineTo((x + 80, 300)); pen.lineTo((x, 300)); pen.closePath()
        glyphs[g] = pen.glyph()
    fb.setupGlyf(glyphs)
    advance = 600 if index == 0 else 800
    fb.setupHorizontalMetrics({g: (0 if g == 'dieresis' else advance, 0) for g in ORDER})
    fb.setupHorizontalHeader(ascent=1000, descent=0)
    name = f'MITEuropeanFallback{index}'
    fb.setupNameTable(dict(familyName=name, styleName='Regular', uniqueFontIdentifier=name,
                           fullName=name, psName=name))
    fb.setupOS2(sTypoAscender=1000, sTypoDescender=0, usWinAscent=1000, usWinDescent=0)
    fb.setupPost()
    addOpenTypeFeaturesFromString(fb.font, '''languagesystem DFLT dflt;
        feature liga { sub u0066 u0069 by fi; } liga;''')
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font


if __name__ == '__main__':
    collection = TTCollection()
    collection.fonts = [build(i) for i in range(3)]
    collection.save(Path(__file__).with_name('european-fallback.ttc'))
