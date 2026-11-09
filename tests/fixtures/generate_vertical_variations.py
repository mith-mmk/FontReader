"""Generate MIT vertical-metric variation fixtures with fontTools 4.65.0.

Run from repository root. Shared builder also regenerates identical flow files.
All tests consume committed bytes; no installed fonts or Python are needed.
"""
from copy import deepcopy
from generate_vertical_flow import build, ROOT
from fontTools.ttLib import newTable, TTCollection
from fontTools.ttLib.tables import otTables
from fontTools.ttLib.tables.TupleVariation import TupleVariation
from fontTools.varLib.builder import (
    buildVarRegionList, buildVarData, buildVarStore, buildDeltaSetIndexMap,
)

fb = build(True)
fb.setupFvar([('wght', 100, 400, 900, 'Weight')], [])
fb.setupStat([{'tag': 'wght', 'name': 'Weight', 'ordering': 0}])
font = fb.font
font['head'].flags = (font['head'].flags | 2) & ~32
font['gvar'] = newTable('gvar')
font['gvar'].variations = {name: [] for name in font.getGlyphOrder()}
# A: yMax 700 -> 800; top phantom 800 -> 940; bottom -400 -> -460.
# Advance delta +200, TSB delta +40, BSB delta +60; horizontal metrics unchanged.
font['gvar'].variations['A'] = [TupleVariation({'wght': (0, 1, 1)},
    [(0, 0), (0, 0), (0, 100), (0, 100), (0, 0), (0, 0), (0, 140), (0, -60)])]


def with_vvar(source, advance=200, tsb=40, mappings=True, origin=False):
    result = deepcopy(source)
    vvar = newTable('VVAR')
    vvar.table = t = otTables.VVAR()
    t.Version = 0x10000
    t.VarStore = buildVarStore(
        buildVarRegionList([{'wght': (0, 1, 1)}], ['wght']),
        [buildVarData([0], [[0], [advance]] + [[0]] * 6 + [[tsb], [60], [2000]],
                      optimize=False)])
    t.AdvHeightMap = None  # Required advance deltas use implicit glyph indices.
    t.TsbMap = buildDeltaSetIndexMap([0, 8, 0, 0, 0, 0, 0, 0]) if mappings else None
    t.BsbMap = buildDeltaSetIndexMap([0, 9, 0, 0, 0, 0, 0, 0]) if mappings else None
    t.VOrgMap = buildDeltaSetIndexMap([0, 10, 0, 0, 0, 0, 0, 0]) if origin else None
    result['VVAR'] = vvar
    return result


both = with_vvar(font)
both.save(ROOT / 'vertical-variations.ttf')
# The mismatched face is deliberately inconsistent: it tests source selection,
# not a claim that differing gvar and VVAR metrics are valid font authoring.
mismatch = with_vvar(font, advance=300, tsb=80)
# VVAR origin maps are not used for TrueType, even when present.
ignored_origin = with_vvar(font, origin=True)
# Isolate VVAR when no gvar tuple applies; diagnostic rather than authoring model.
table_only = with_vvar(font)
table_only['gvar'].variations['A'] = []
# Distinct mapped zero must override nonzero gvar, not be treated as absent.
zero = with_vvar(font, advance=0, tsb=0)
collection = TTCollection()
collection.fonts = [font, both, with_vvar(font, mappings=False), mismatch,
                    ignored_origin, table_only, zero]
collection.save(ROOT / 'vertical-variations.ttc')
