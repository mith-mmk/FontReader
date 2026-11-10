"""Generate the MIT grapheme fallback fixture with fontTools 4.65.0.

Self-contained: run from any directory; tests consume committed TTC bytes.
Faces 0/1/2 cover cluster prefixes/all scalars/suffixes respectively, with
400/800/600 unit advances. There are no shaping substitutions or positioning.
"""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTCollection
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable

PREFIX = {0x41, 0x4E00, 0x1100, 0x0915, 0x0600, 0x1F469, 0x1F1EF}
SUFFIX = {0x42, 0x1161, 0x11A8, 0x093E, 0x094D, 0x0937, 0x0627,
          0x0301, 0x1F3FD, 0x200D, 0x1F4BB, 0x1F1F5}
ALL = PREFIX | SUFFIX


def build(index, coverage, advance):
    fb = FontBuilder(1000, isTTF=True)
    names = {cp: f'u{cp:05X}' for cp in sorted(ALL)}
    order = ['.notdef'] + list(names.values()) + ['variant']
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap({cp: names[cp] for cp in sorted(coverage)})
    outlines = {}
    for name in order:
        pen = TTGlyphPen(None)
        if name != '.notdef':
            pen.moveTo((50, 0)); pen.lineTo((advance - 50, 0))
            pen.lineTo((advance - 50, 700)); pen.lineTo((50, 700)); pen.closePath()
        outlines[name] = pen.glyph()
    fb.setupGlyf(outlines)
    fb.setupHorizontalMetrics({name: (advance + 100 if name == 'variant' else advance, 50 if name != '.notdef' else 0) for name in order})
    fb.setupHorizontalHeader(ascent=800, descent=-200, lineGap=0)
    fb.setupNameTable({'familyName': f'Fontcore Grapheme {index}', 'styleName': 'Regular',
                       'uniqueFontIdentifier': f'Fontcore-Grapheme-{index}-1',
                       'fullName': f'Fontcore Grapheme {index}', 'psName': f'FontcoreGrapheme{index}'})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    if index == 1:
        uvs = CmapSubtable.newSubtable(14)
        uvs.platformID, uvs.platEncID, uvs.language = 0, 5, 0
        uvs.cmap = {}
        uvs.uvsDict = {0xFE00: [(0x41, 'variant')], 0xE0100: [(0x4E00, 'variant')]}
        fb.font['cmap'].tables.append(uvs)
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font


if __name__ == '__main__':
    collection = TTCollection()
    collection.fonts = [build(0, PREFIX, 400), build(1, ALL, 800), build(2, SUFFIX, 600)]
    collection.save(Path(__file__).with_name('unicode-grapheme-fallback.ttc'))
