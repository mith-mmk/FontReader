"""MIT CFF2 vertical-origin fixtures; fontTools 4.65.0, run from repo root.

Synthetic diagnostic faces isolate VORG, optional VVAR maps and curve extrema.
All bytes and checksums are reproducible; tests do not require Python.
"""
from generate_vertical_flow import build, ROOT
from fontTools.misc.psCharStrings import T2CharString
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.ttLib import TTCollection, newTable
from fontTools.ttLib.tables import otTables
from fontTools.varLib.builder import (buildVarRegionList, buildVarData,
                                     buildVarStore, buildDeltaSetIndexMap)
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString


def font(vertical=True, vorg=True, origin='signed'):
    fb = build(vertical)
    old = fb.font
    glyphs = old.getGlyphSet()
    chars = {}
    for name in old.getGlyphOrder():
        pen = T2CharStringPen(None, glyphs, CFF2=True)
        glyphs[name].draw(pen)
        chars[name] = pen.getCharString()
    # Relative cubic operands yield a true maximum at t=1/2, not a control point.
    chars['B'] = T2CharString(program=[100, 0, 'rmoveto',
        0, 800, 160, 1, 'blend', 700, 0, 0, -800, -160, 1, 'blend', 'rrcurveto'])
    chars['A'] = T2CharString(program=[50, 0, 'rmoveto', 400, 0, 'rlineto',
        0, 700, 100, 1, 'blend', 'rlineto', -400, 0, 'rlineto'])
    del old['glyf']; del old['loca']
    fb.isTTF = False
    fb.setupFvar([('wght', 100, 400, 900, 'Weight')], [])
    fb.setupStat([{'tag': 'wght', 'name': 'Weight', 'ordering': 0}])
    fb.setupCFF2(chars, regions=[{'wght': (0, 1, 1)}])
    fb.setupMaxp()
    if vorg:
        old['VORG'] = newTable('VORG')
        old['VORG'].majorVersion, old['VORG'].minorVersion = 1, 0
        old['VORG'].defaultVertOriginY = 900
        old['VORG'].VOriginRecords = {'B': -100, 'acute': 1200}
    if vertical:
        vvar = newTable('VVAR'); vvar.table = t = otTables.VVAR()
        t.Version = 0x10000
        # Distinct indices deliberately distinguish each optional map from advances.
        t.VarStore = buildVarStore(
            buildVarRegionList([{'wght': (0, 1, 1)}], ['wght']),
            [buildVarData([0], [[0], [200], [100]] + [[0]] * 5
                          + [[40], [20], [80], [-60]], optimize=False)])
        t.AdvHeightMap = None
        t.TsbMap = buildDeltaSetIndexMap([0, 8, 9, 0, 0, 0, 0, 0])
        t.BsbMap = None
        t.VOrgMap = (None if origin == 'absent' else
                    buildDeltaSetIndexMap([0] * 8 if origin == 'zero' else
                                          [0, 10, 11, 0, 0, 0, 0, 0]))
        old['VVAR'] = vvar
    addOpenTypeFeaturesFromString(old, '''languagesystem DFLT dflt;
    table GDEF { GlyphClassDef [A B], , [acute], ; } GDEF;
    markClass acute <anchor 100 700> @TOP;
    feature mark { pos base A <anchor 300 700> mark @TOP; } mark;
    feature mkmk { pos mark acute <anchor 100 800> mark @TOP; } mkmk;
    feature vert { sub comma by B; } vert;
    ''')
    return old


collection = TTCollection()
collection.fonts = [font(), font(vorg=False, origin='absent'),
                    font(origin='absent'), font(origin='zero'),
                    font(vertical=False, vorg=False), font(vorg=False)]
collection.save(ROOT / 'cff2-vertical-origin.ttc')
f = font(); f.flavor = 'woff'; f.save(ROOT / 'cff2-vertical-origin.woff')

