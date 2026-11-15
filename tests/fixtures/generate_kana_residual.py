"""Original MIT residual-kana fixtures. No external font data; fontTools 4.68.0."""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection, newTable
from fontTools.ttLib.tables import otTables

CODES = [65, 66, 0x301, 0x3042, 0x304b, 0x304c, 0x3099, 0x309a, 0x309b, 0x309c, 0x30a2, 0x31f7]
def name(c): return 'u%04X' % c
ORDER = ['.notdef'] + [name(c) for c in CODES] + ['alt', 'lig']

def build(index):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(ORDER)
    cmap = {c: name(c) for c in CODES}
    if index == 13: cmap[0x3099] = '.notdef'
    if index == 17: cmap[0x3042] = '.notdef'
    fb.setupCharacterMap(cmap)
    glyphs = {}
    for n, g in enumerate(ORDER):
        pen = TTGlyphPen(None)
        pen.moveTo((n*5, 0)); pen.lineTo((n*5+100, 0)); pen.lineTo((n*5+100, 700)); pen.lineTo((n*5, 700)); pen.closePath()
        glyphs[g] = pen.glyph()
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics({g:(1000, 0) for g in ORDER})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupVerticalMetrics({g:(1000, 180) for g in ORDER})
    fb.setupVerticalHeader(ascent=800, descent=-200)
    fb.setupNameTable({'familyName':'MIT Kana Residual', 'styleName':'Regular', 'uniqueFontIdentifier':'MITKanaResidual', 'fullName':'MIT Kana Residual', 'psName':'MITKanaResidual'})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    # An unrelated GPOS lookup retains nominal mark offsets, as in Hiragino.
    features = 'languagesystem DFLT dflt; feature kern { pos u0041 10; } kern;'
    extras = {
        3: 'feature dist { pos u3099 <25 35 -1000 0>; } dist; feature vert { pos u3099 <25 35 0 -1000>; } vert;',
        4: 'feature dist { pos u3099 <0 0 0 0>; } dist; feature vert { pos u3099 <0 0 0 0>; } vert;',
        5: 'feature ccmp { sub u3099 by alt; } ccmp;',
        6: 'feature ccmp { sub u3099 by u3099; } ccmp;',
        7: 'feature ccmp { sub u3099 by u3099 alt; } ccmp;',
        8: 'feature liga { sub u3042 u3099 by lig; } liga;',
        9: 'feature vert { sub u3042 by alt; } vert;',
        10: 'feature kern { pos u3042 <0 0 0 0> u3099 <0 0 -1000 0>; } kern; feature vkrn { pos u3042 <0 0 0 0> u3099 <0 0 0 -1000>; } vkrn;',
        11: 'feature kern { pos u3042 <0 0 0 0> u3099 <0 0 0 0>; } kern; feature vkrn { pos u3042 <0 0 0 0> u3099 <0 0 0 0>; } vkrn;',
        14: 'markClass u3099 <anchor 0 700> @M; feature mark { pos base u3042 <anchor 300 700> mark @M; } mark;',
        15: 'feature ccmp { sub u3042 by u3042 alt; } ccmp;',
        16: 'feature ccmp { sub u304C by .notdef; } ccmp;',
        18: 'feature kern { pos u3099 u0042 -1000; } kern; feature vkrn { pos u3099 u0042 <0 0 0 -1000>; } vkrn;',
        19: 'feature kern { pos u3042 u3099 0; } kern; feature vkrn { pos u3042 u3099 <0 0 0 0>; } vkrn;',
        20: 'feature locl { sub u3099 by u3099; } locl;',
        21: 'feature ccmp { sub u3042 by NULL; } ccmp;',
    }
    features += extras.get(index, '')
    addOpenTypeFeaturesFromString(fb.font, features)
    if index == 1:
        table = newTable('GDEF'); table.table = otTables.GDEF(); table.table.Version=0x10000
        for key in ['GlyphClassDef','AttachList','LigCaretList','MarkAttachClassDef']: setattr(table.table,key,None)
        fb.font['GDEF']=table
    if index == 2:
        addOpenTypeFeaturesFromString(fb.font, features+' table GDEF { GlyphClassDef [u3042], , , ; } GDEF;')
    if index == 12: del fb.font['GPOS']
    # No implicit author classes except the deliberately authored fixture.
    if index not in [1,2,14] and 'GDEF' in fb.font: del fb.font['GDEF']
    fb.font['head'].created=fb.font['head'].modified=2082844800
    fb.font.recalcTimestamp=False
    return fb.font

if __name__ == '__main__':
    out=TTCollection(); out.fonts=[build(i) for i in range(22)]
    out.save(Path(__file__).with_name('kana-residual.ttc'))
