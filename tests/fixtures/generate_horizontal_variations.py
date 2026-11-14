"""Generate deterministic MIT synthetic HVAR/gvar regression data, fontTools 4.65.
Run from repository root. No external fonts. Conflicting faces are diagnostic only.
"""
from copy import deepcopy
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import newTable, TTCollection
from fontTools.ttLib.tables import otTables
from fontTools.ttLib.tables.TupleVariation import TupleVariation
from fontTools.varLib.builder import buildVarRegionList, buildVarData, buildVarStore, buildDeltaSetIndexMap
ROOT = Path(__file__).parent
fb = FontBuilder(1000, isTTF=True)
names = ['.notdef', 'A', 'B']
fb.setupGlyphOrder(names)
fb.setupCharacterMap({65: 'A', 66: 'B'})
p = TTGlyphPen(None)
p.moveTo((50, 0)); p.lineTo((450, 0)); p.lineTo((450, 700)); p.lineTo((50, 700)); p.closePath()
glyphs = {'.notdef': TTGlyphPen(None).glyph(), 'A': p.glyph()}
p = TTGlyphPen(glyphs)
p.addComponent('A', (1, 0, 0, 1, 100, 0))
glyphs['B'] = p.glyph()
fb.setupGlyf(glyphs)
fb.setupHorizontalMetrics({'.notdef': (600, 0), 'A': (600, 50), 'B': (700, 150)})
fb.setupHorizontalHeader(ascent=800, descent=-200)
fb.setupNameTable({'familyName': 'Fontcore Synthetic Horizontal', 'styleName': 'Regular', 'uniqueFontIdentifier': 'Fontcore-Horizontal-1', 'fullName': 'Fontcore Synthetic Horizontal', 'psName': 'FontcoreSyntheticHorizontal'})
fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
fb.setupPost()
fb.setupFvar([('wght', 100, 400, 900, 'Weight')], [])
fb.setupStat([{'tag': 'wght', 'name': 'Weight', 'ordering': 0}])
f = fb.font
f['head'].created = f['head'].modified = 2082844800
f['head'].flags = (f['head'].flags | 2) & ~32
f.recalcTimestamp = False
f['gvar'] = newTable('gvar')
f['gvar'].variations = {n: [] for n in names}
region = {'wght': (0, 1, 1)}
f['gvar'].variations['A'] = [TupleVariation(region, [(40, 0)] * 4 + [(0, 0), (200, 0), (0, 0), (0, 0)])]
f['gvar'].variations['B'] = [TupleVariation(region, [(0, 0), (0, 0), (200, 0), (0, 0), (0, 0)])]
def hvar(source, advance=200, lsb=40, mapped=True, explicit=False):
    result = deepcopy(source)
    t = otTables.HVAR(); t.Version = 0x10000
    t.VarStore = buildVarStore(buildVarRegionList([region], ['wght']), [buildVarData([0], [[0], [advance], [advance], [lsb], [advance-lsb]], optimize=False)])
    t.AdvWidthMap = buildDeltaSetIndexMap([0, 1, 2]) if explicit else None
    t.LsbMap = buildDeltaSetIndexMap([0, 3, 3]) if mapped else None
    t.RsbMap = buildDeltaSetIndexMap([0, 4, 4]) if mapped else None
    result['HVAR'] = newTable('HVAR'); result['HVAR'].table = t
    return result
phantom = deepcopy(f)
phantom['gvar'].variations['A'] = [TupleVariation(region, [None] * 4 + [(0, 0), (200, 0), None, None])]
# Faces 4/5/7 deliberately conflict to test precedence, not font conformance.
table_only = hvar(f)
table_only['gvar'].variations = {n: [] for n in names}
negative = deepcopy(phantom)
negative['gvar'].variations['A'] = [TupleVariation(region, [None] * 4 + [(0, 0), (-200, 0), None, None])]
# Faces 9-12 are saturation diagnostics, not conforming metric ranges.
under = deepcopy(phantom)
under['gvar'].variations['A'] = [TupleVariation(region, [None] * 4 + [(0, 0), (-800, 0), None, None])]
over = deepcopy(phantom)
over['hmtx'].metrics['A'] = (65500, 50)
hvar_under = hvar(under, advance=-800, lsb=0)
hvar_over = hvar(over, advance=200, lsb=0)
# Signed, fractional interpolation away from half-way rounding boundaries.
signed = deepcopy(f)
signed['gvar'].variations['A'] = [TupleVariation(region, [(-41, 0)] * 4 + [(0, 0), (-201, 0), (0, 0), (0, 0)])]
collection = TTCollection()
collection.fonts = [f, hvar(f), hvar(f, mapped=False), hvar(f, explicit=True), hvar(f, advance=300, lsb=80), hvar(f, advance=0, lsb=0), phantom, table_only, negative, under, over, hvar_under, hvar_over, signed, hvar(signed, advance=-201, lsb=-41)]
collection.save(ROOT / 'horizontal-variations.ttc')

hvar(f).save(ROOT / 'horizontal-variations.ttf')
