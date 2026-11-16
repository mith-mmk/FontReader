"""Original MIT synthetic Latin composition fixtures; requires fontTools only."""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection

CODES = {0x61:'a',0x42:'B',0x303:'tilde',0x301:'acute',0x325:'ringbelow',0x306:'breve',0xE3:'atilde',0x1E01:'aringbelow',0x103:'abreve',0x1EAF:'abreveacute'}
ORDER = ['.notdef'] + list(CODES.values())

def build(index):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(ORDER)
    cmap = dict(CODES)
    if index in [1, 6]:
        cmap = {c:g for c,g in cmap.items() if c < 0x80 or 0x300 <= c <= 0x36f}
    if index == 2:
        cmap.pop(0x61)
        cmap.pop(0x306)
    if index == 4:
        cmap.pop(0x1EAF)
    fb.setupCharacterMap(cmap)
    glyphs = {}
    for i,g in enumerate(ORDER):
        p = TTGlyphPen(None)
        x,y = i*7, (-200 if g == 'ringbelow' else 0)
        p.moveTo((x,y)); p.lineTo((x+100,y)); p.lineTo((x+100,y+300+i*20)); p.lineTo((x,y+300+i*20)); p.closePath()
        glyphs[g] = p.glyph()
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics({g:(0 if g in ['tilde','acute','ringbelow','breve'] else 600,0) for g in ORDER})
    fb.setupHorizontalHeader(ascent=1000,descent=-300)
    fb.setupNameTable({'familyName':'MIT Latin Composition','styleName':'Regular','uniqueFontIdentifier':f'MITLatinComposition{index}','fullName':'MIT Latin Composition','psName':f'MITLatinComposition{index}'})
    fb.setupOS2(sTypoAscender=1000,sTypoDescender=-300,usWinAscent=1000,usWinDescent=300)
    fb.setupPost()
    fea = '''languagesystem DFLT dflt;
markClass [tilde acute breve] <anchor 0 0> @TOP;
markClass ringbelow <anchor 0 0> @BOTTOM;
feature mark {
pos base a <anchor 250 500> mark @TOP <anchor 250 -100> mark @BOTTOM;
pos base atilde <anchor 275 800> mark @TOP;
pos base aringbelow <anchor 300 550> mark @TOP;
pos base abreve <anchor 325 750> mark @TOP;
} mark;
'''
    if index in [3, 5]:
        fea += 'feature ccmp { sub atilde by .notdef; sub aringbelow by .notdef; sub abreveacute by .notdef; } ccmp;'
    if index in [5, 6]:
        fea += 'feature mkmk { pos mark tilde <anchor 50 300> mark @TOP; } mkmk;'
    addOpenTypeFeaturesFromString(fb.font,fea)
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font

if __name__ == '__main__':
    collection = TTCollection()
    collection.fonts = [build(i) for i in range(7)]
    collection.save(Path(__file__).with_name('latin-composition.ttc'))
