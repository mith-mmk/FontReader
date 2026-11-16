"""Original MIT GDEF caret fixtures; no external font.

Faces: 0 coordinates; 1 contour points; 2 negative signed format-3;
3 ppem-20 device deltas; 4 real one-axis VariationIndex store; 5 invalid
VariationIndex (inner index 7); 6 static GPOS placement; 7/8/9 ordered/reversed/duplicate ffi carets. Coverage is [fi, fl], while physical LigGlyph
blocks are [fl, fi], so sorting offsets instead of following coverage is wrong.
"""
from pathlib import Path
import struct
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection, newTable
from fontTools.ttLib.tables.DefaultTable import DefaultTable
from fontTools.ttLib.tables._f_v_a_r import Axis


def u16(*values):
    return struct.pack('>' + 'H' * len(values), *values)


def caret(face, which):
    coordinate = [220, 730][which]
    if face == 0:
        return u16(1, coordinate)
    if face == 1:
        return u16(2, [1, 2][which])
    if face == 2:
        return u16(3) + struct.pack('>hH', [-90, -160][which], 0)
    if face == 3:
        # 4-bit signed delta: +1 for fi, +2 for fl, at 20 ppem only.
        device = u16(20, 20, 2, (which + 1) << 12)
    else:
        device = u16(0, 0 if face == 4 else 7, 0x8000)
    return u16(3, coordinate, 6) + device


def gdef(face, glyph_ids):
    if face >= 7:
        coordinates = {7: (200, 500), 8: (500, 200), 9: (200, 200)}[face]
        block = u16(2, 6, 10) + u16(1, coordinates[0], 1, coordinates[1])
        ligature = u16(6 + len(block), 1, 6) + block + u16(1, 1, glyph_ids['ffi'])
        return struct.pack('>IHHHH', 0x00010000, 0, 0, 12, 0) + ligature
    blocks = [u16(1, 4) + caret(face, i) for i in (0, 1)]
    # fi offset follows fl's block; do NOT sort this coverage-indexed array.
    ligature = u16(8 + len(blocks[1]) + len(blocks[0]), 2,
                   8 + len(blocks[1]), 8) + blocks[1] + blocks[0]
    ligature += u16(1, 2, glyph_ids['fi'], glyph_ids['fl'])
    if face < 4:
        return struct.pack('>IHHHH', 0x00010000, 0, 0, 12, 0) + ligature
    # ItemVariationStore: one positive-axis region, one item, delta +100.
    regions = u16(1, 1, 0, 0x4000, 0x4000)
    data = u16(1, 1, 1, 0, 100)
    store = u16(1) + struct.pack('>I', 12) + u16(1) + struct.pack('>I', 22) + regions + data
    return (struct.pack('>IHHHHHI', 0x00010003, 0, 0, 18, 0, 0, 18 + len(ligature))
            + ligature + store)


def build(index):
    order = ['.notdef', 'space', 'f', 'i', 'l', 'B', 'fi', 'fl']
    if index >= 7:
        order.append('ffi')
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap({32: 'space', 102: 'f', 105: 'i', 108: 'l', 66: 'B'})
    glyphs = {}
    for name in order:
        pen = TTGlyphPen(None)
        if name != 'space':
            # fi contour point 1 = (300,0), fl point 2 = (850,500).
            x = 300 if name == 'fi' else 850 if name == 'fl' else 400
            pen.moveTo((20, 0)); pen.lineTo((x, 0))
            pen.lineTo((x, 500)); pen.lineTo((20, 500)); pen.closePath()
        glyphs[name] = pen.glyph()
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics({g: (1200 if g == 'ffi' else 900 if g == 'fi' else 1100 if g == 'fl' else 300 if g == 'space' else 600, 0) for g in order})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    name = f'MITGdefCarets{index}'
    fb.setupNameTable(dict(familyName=name, styleName='Regular', uniqueFontIdentifier=name,
                          fullName=name, psName=name))
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    features = '''languagesystem DFLT dflt;
        feature liga { sub f i by fi; sub f l by fl; } liga;'''
    if index >= 7:
        features = features.replace('sub f i by fi;', 'sub f f i by ffi; sub f i by fi;')
    if index == 6:
        features += '''feature dist { pos fi <50 70 0 0>; pos fl <-30 -20 0 0>; } dist;'''
    addOpenTypeFeaturesFromString(fb.font, features)
    if index in (4, 5):
        fvar = newTable('fvar')
        axis = Axis(); axis.axisTag = 'wght'; axis.minValue = 100
        axis.defaultValue = 400; axis.maxValue = 900; axis.flags = 0; axis.axisNameID = 256
        fvar.axes = [axis]; fvar.instances = []
        fb.font['fvar'] = fvar
        fb.font['name'].setName('Weight', 256, 3, 1, 0x409)
        avar = newTable('avar'); avar.segments = {'wght': {-1.0: -1.0, 0.0: 0.0, 0.5: 0.25, 1.0: 1.0}}
        fb.font['avar'] = avar
        gvar = newTable('gvar'); gvar.variations = {}
        fb.font['gvar'] = gvar
    table = DefaultTable('GDEF'); table.data = gdef(0 if index == 6 else index, fb.font.getReverseGlyphMap())
    fb.font['GDEF'] = table
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font


if __name__ == '__main__':
    collection = TTCollection()
    collection.fonts = [build(i) for i in range(10)]
    collection.save(Path(__file__).with_name('gdef-carets.ttc'))
