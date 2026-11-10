"""MIT static CFF vertical fixtures. fontTools 4.65.0; run from repo root."""
from generate_vertical_flow import build, ROOT
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.ttLib import TTCollection, newTable
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

def cff(vertical=True, vorg=True):
    old = build(vertical).font
    fb = FontBuilder(1000, isTTF=False)
    fb.font = old
    fb.font.sfntVersion = 'OTTO'
    chars = {}
    glyphs = old.getGlyphSet()
    for name in old.getGlyphOrder():
        pen = T2CharStringPen(None, glyphs)
        if name == 'B':
            # Cubic top = 600, although the control points reach 800.
            pen.moveTo((100, 0)); pen.curveTo((100, 800), (800, 800), (800, 0)); pen.closePath()
        else:
            glyphs[name].draw(pen)
        chars[name] = pen.getCharString()
    del old['glyf']; del old['loca']
    fb.setupCFF('FontcoreSyntheticCFF', {}, chars, {})
    fb.setupMaxp()
    if vorg:
        old['VORG'] = newTable('VORG')
        old['VORG'].majorVersion, old['VORG'].minorVersion = 1, 0
        old['VORG'].defaultVertOriginY = 900
        old['VORG'].VOriginRecords = {'B': -100, 'acute': 1200}
        if vertical:
            old['vmtx'].metrics['A'] = (1200, 200)
            old['vmtx'].metrics['B'] = (800, -700)
            old['vmtx'].metrics['acute'] = (1000, 400)
    addOpenTypeFeaturesFromString(old, '''languagesystem DFLT dflt;
    table GDEF { GlyphClassDef [A B], , [acute], ; } GDEF;
    markClass acute <anchor 100 700> @TOP;
    feature mark { pos base A <anchor 300 700> mark @TOP; } mark;
    feature mkmk { pos mark acute <anchor 100 800> mark @TOP; } mkmk;
    feature vert { sub comma by B; } vert;
    ''')
    return old
collection = TTCollection()
malformed = cff()
from fontTools.ttLib.tables.DefaultTable import DefaultTable
malformed['VORG'] = DefaultTable('VORG'); malformed['VORG'].data = b'bad'
truetype = build(True).font
truetype['VORG'] = DefaultTable('VORG'); truetype['VORG'].data = b'bad'
collection.fonts = [cff(), cff(vorg=False), cff(False, False), malformed, truetype]
collection.save(ROOT / 'cff-vertical-origin.ttc')
f = cff(); f.flavor = 'woff'; f.save(ROOT / 'cff-vertical-origin.woff')
