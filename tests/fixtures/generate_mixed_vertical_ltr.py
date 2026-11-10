"""Generate MIT mixed-vertical-LTR cases with fontTools 4.65.0.

Read only vertical-metrics.ttf and vertical-variations.ttf. Fixed timestamps. Faces exercise orientation
feature exclusivity, ordinary GSUB ordering, marks, placement, and full advances.
"""
from pathlib import Path
from fontTools.ttLib import TTFont, TTCollection, newTable
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
from fontTools.ttLib.tables import otTables
from fontTools.varLib.builder import buildVarStore, buildVarRegionList, buildVarData
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
ROOT = Path(__file__).parent


def build(features='', pairs='pos A B -70;', gap=0):
    font = TTFont(ROOT / 'vertical-metrics.ttf', recalcTimestamp=False)
    for table in font['cmap'].tables:
        if table.isUnicode():
            table.cmap.update({0x6F22: 'comma', ord('V'): 'B', ord('f'): 'A',
                              ord('i'): 'B', 0x302A: 'acute', 0x20E3: 'acute',
                              ord('1'): 'A'})
    full = CmapSubtable.newSubtable(12)
    full.platformID, full.platEncID, full.language = 3, 10, 0
    full.cmap = dict(font.getBestCmap())
    full.cmap[0x1D127] = 'B'
    font['cmap'].tables.append(full)
    font['hhea'].lineGap = gap
    addOpenTypeFeaturesFromString(font, '''
        languagesystem DFLT dflt;
        table GDEF { GlyphClassDef [A B comma bracket], , [acute], ; } GDEF;
        markClass acute <anchor 100 700> @TOP;
        feature mark {
            pos base A <anchor 200 500> mark @TOP;
            pos base B <anchor 300 600> mark @TOP;
            pos base comma <anchor 650 900> mark @TOP;
            pos base bracket <anchor 500 800> mark @TOP;
        } mark;
    ''' + ('feature kern { ' + pairs + ' } kern;' if pairs else '') + features)
    return font


collection = TTCollection()
collection.fonts = [
    build(),
    build(pairs='pos A <20 100 -40 120> B <-30 -200 60 -80>;'),
    build('feature vert { sub bracket by B; } vert; feature vrt2 { sub bracket by A; } vrt2; feature vrtr { sub bracket by comma; } vrtr;'),
    build('feature vrt2 { sub bracket by A; } vrt2; feature vrtr { sub bracket by comma; } vrtr;'),
    build('feature vert { sub bracket by bracket; } vert; feature vrt2 { sub bracket by A; } vrt2; feature vrtr { sub bracket by comma; } vrtr;'),
    build('feature ccmp { sub A by B; } ccmp; feature vrtr { sub B by bracket; } vrtr; feature vert { sub B by comma; } vert; feature vrt2 { sub B by comma; } vrt2;'),
    build('feature ccmp { sub A by A B; } ccmp; feature vrtr { sub A by comma; sub B by bracket; } vrtr;'),
    build('feature liga { sub A B by bracket; } liga; feature vrtr { sub bracket by comma; } vrtr;'),
    build('feature ccmp { sub comma by A; } ccmp; feature vert { sub A by bracket; } vert; feature vrt2 { sub A by B; } vrt2; feature vrtr { sub A by B; } vrtr;'),
    build('feature liga { sub comma comma by A; } liga; feature vert { sub A by bracket; } vert;'),
    build(gap=200),
    build('feature ccmp { sub comma by A B; } ccmp; feature vert { sub A by comma; sub B by bracket; } vert;'),
    build('feature liga { sub comma A by bracket; sub A comma by bracket; } liga;'),
]
# Face13: the existing variable outline plus supported horizontal MVAR metrics.
variable = TTFont(ROOT / 'vertical-variations.ttf', recalcTimestamp=False)
for table in variable['cmap'].tables:
    if table.isUnicode():
        table.cmap.update({0x6F22: 'comma', ord('V'): 'B'})
mvar = newTable('MVAR')
mvar.table = t = otTables.MVAR()
t.Version, t.Reserved, t.ValueRecordSize = 0x10000, 0, 8
t.VarStore = buildVarStore(buildVarRegionList([{'wght': (0, 1, 1)}], ['wght']),
                          [buildVarData([0], [[200], [100], [200]], optimize=False)])
t.ValueRecord = []
for index, tag in enumerate(['hasc', 'hdsc', 'hlgp']):
    record = otTables.MetricsValueRecord()
    record.ValueTag, record.VarIdx = tag, index
    t.ValueRecord.append(record)
t.ValueRecordCount = len(t.ValueRecord)
variable['MVAR'] = mvar
collection.fonts.append(variable)
# Face14: upright cross-axis advances must also survive the next boundary.
collection.fonts.append(build('feature vkrn { pos comma <20 100 -40 120> bracket <-30 -200 60 -80>; } vkrn;'))
collection.save(ROOT / 'mixed-vertical-ltr.ttc')
