"""Generate MIT synthetic composite gvar data with fontTools 4.68.0.

Run from repository root. No external fonts. Existing fixtures are untouched.
The Rust oracle records independent fontTools instancing of supported XY cases.
Nonzero-left-phantom and conflicting-table cases are diagnostic, not conforming
variable-font authoring. U/P exercise documented fallback boundaries.
"""
from copy import deepcopy
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import newTable, TTCollection
from fontTools.ttLib.tables import otTables
from fontTools.ttLib.tables.TupleVariation import TupleVariation
from fontTools.ttLib.tables._g_l_y_f import GlyphComponent
from fontTools.varLib.builder import buildVarRegionList, buildVarData, buildVarStore, buildDeltaSetIndexMap
from fontTools.varLib.instancer import instantiateVariableFont

ROOT = Path(__file__).parent
names = ['.notdef', 'A', 'B', 'C', 'D', 'E', 'S', 'T', 'U', 'P']
fb = FontBuilder(1000, isTTF=True)
fb.setupGlyphOrder(names)
fb.setupCharacterMap({ord(n): n for n in names[1:]})
p = TTGlyphPen(None)
p.moveTo((50, 0)); p.lineTo((450, 0)); p.lineTo((450, 700)); p.lineTo((50, 700)); p.closePath()
glyphs = {'.notdef': TTGlyphPen(None).glyph(), 'A': p.glyph()}

def composite(name, components):
    p = TTGlyphPen(glyphs)
    for base, transform in components:
        p.addComponent(base, transform)
    glyphs[name] = p.glyph()

identity = (1, 0, 0, 1, 100, 0)
composite('B', [('A', identity)])
composite('C', [('B', (1, 0, 0, 1, 50, 20))])
for n in ['D', 'E', 'U']:
    composite(n, [('A', identity)])
composite('S', [('A', (0.5, 0, 0, 0.5, 100, 0))])
glyphs['S'].components[0].flags = 0x0800  # SCALED_COMPONENT_OFFSET
composite('T', [('A', identity), ('A', (1, 0, 0, 1, 800, 0))])
glyphs['U'].components[0].flags |= 0x0200  # USE_MY_METRICS fallback diagnostic
composite('P', [('A', identity)])
attachment = GlyphComponent()
attachment.glyphName = 'A'; attachment.firstPt = 1; attachment.secondPt = 0
attachment.flags = 0
glyphs['P'].components.append(attachment)
fb.setupGlyf(glyphs)
advances = {n: 700 for n in names}
advances.update(A=600, C=900, S=400, T=1400, P=1100)
fb.setupHorizontalMetrics({n: (advances[n], getattr(g, 'xMin', 0)) for n, g in glyphs.items()})
fb.setupHorizontalHeader(ascent=800, descent=-200)
fb.setupVerticalMetrics({n: (1200, 100) for n in names})
fb.setupVerticalHeader(ascent=800, descent=-200)
fb.setupNameTable({'familyName': 'Fontcore Composite Gvar', 'styleName': 'Regular',
    'uniqueFontIdentifier': 'Fontcore-Composite-Gvar-1', 'fullName': 'Fontcore Composite Gvar',
    'psName': 'FontcoreCompositeGvar'})
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
def variation(name, deltas):
    f['gvar'].variations[name] = [TupleVariation(region, deltas)]
variation('A', [(40, 80)] * 4 + [(0, 0), (200, 0), (0, 100), (0, -100)])
variation('B', [(20, 30), (-10, 0), (190, 0), (0, 140), (0, -60)])
variation('C', [(-12, 8), (0, 0), (100, 0), (0, 80), (0, -20)])
# Sparse tuples: no IUP between component offsets or phantom points.
variation('D', [None, (-9, 0), (-210, 0), None, None])
# E has no tuple, but its varied child changes its bounds and bearings.
variation('S', [(20, 40), (0, 0), (100, 0), (0, 80), (0, -20)])
variation('T', [None, (200, 20), None, (100, 0), None, None])
variation('U', [(20, 30), (-10, 0), (190, 0), (0, 140), (0, -60)])
variation('P', [None, (200, 20), None, (100, 0), None, None])

selected = deepcopy(f)
selected['hmtx'].metrics['B'] = (750, 135)
selected['vmtx'].metrics['B'] = (1300, 120)
selected['head'].flags &= ~2  # Diagnostic nonzero default left phantom.

def with_metric_tables(source, mapped_zero=False):
    result = deepcopy(source)
    for tag, advance_name, bearing_name, trailing_name, delta in [
        ('HVAR', 'AdvWidthMap', 'LsbMap', 'RsbMap', 300),
        ('VVAR', 'AdvHeightMap', 'TsbMap', 'BsbMap', 400),
    ]:
        t = getattr(otTables, tag)(); t.Version = 0x10000
        t.VarStore = buildVarStore(buildVarRegionList([region], ['wght']),
            [buildVarData([0], [[0 if mapped_zero else delta]], optimize=False)])
        setattr(t, advance_name, buildDeltaSetIndexMap([0] * len(names)))
        setattr(t, bearing_name, buildDeltaSetIndexMap([0] * len(names)) if mapped_zero else None)
        setattr(t, trailing_name, None)
        if tag == 'VVAR':
            t.VOrgMap = None
        result[tag] = newTable(tag); result[tag].table = t
    return result

collection = TTCollection()
# Faces 2/3 deliberately conflict to test source selection, not font authoring.
collection.fonts = [f, selected, with_metric_tables(f, True), with_metric_tables(f)]
collection.save(ROOT / 'composite-gvar.ttc')

oracle = ["// Generated by generate_composite_gvar.py with fontTools 4.68.0.",
    "// (face, weight, glyph, [advance, LSB, vertical advance, TSB, xMin, yMin, xMax, yMax])",
    "&["]
for index, face in enumerate(collection.fonts[:2]):
    for weight in [400, 413, 525, 650, 900]:
        instance = instantiateVariableFont(face, {'wght': weight}, inplace=False)
        for name in ['B', 'C', 'D', 'E', 'S', 'T']:
            g = instance['glyf'][name]
            values = [*instance['hmtx'].metrics[name], *instance['vmtx'].metrics[name],
                g.xMin, g.yMin, g.xMax, g.yMax]
            oracle.append(f"    ({index}, {weight}, '{name}', {values}),")
oracle.append("]")
(ROOT / 'composite-gvar-oracle.rs').write_text('\n'.join(oracle) + '\n')
