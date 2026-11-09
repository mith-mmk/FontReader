"""Generate MIT synthetic vertical GSUB order cases with fontTools 4.65.0.

Run from repository root. Only vertical-metrics.ttf is read; no other fixture
is rewritten. Faces: ccmp, smcp, liga, locl-before-ccmp, ccmp+vert fallback.
comma/bracket are distinct stand-ins for intermediate/final forms; f/i map
onto A/B. Fixed base timestamps make collection bytes reproducible.
"""
from pathlib import Path
from fontTools.ttLib import TTFont, TTCollection
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

ROOT = Path(__file__).parent

def build(features):
    font = TTFont(ROOT / "vertical-metrics.ttf", recalcTimestamp=False)
    for table in font['cmap'].tables:
        if table.isUnicode():
            table.cmap.update({ord('f'): 'A', ord('i'): 'B'})
    uvs = CmapSubtable.newSubtable(14)
    uvs.platformID, uvs.platEncID, uvs.language = 0, 5, 0
    uvs.cmap = {}
    uvs.uvsDict = {0xFE00: [(65, 'B')]}
    font['cmap'].tables.append(uvs)
    font['vmtx'].metrics['bracket'] = (1400, 300)
    addOpenTypeFeaturesFromString(font, '''
        languagesystem DFLT dflt;
        table GDEF { GlyphClassDef [A B comma bracket], , [acute], ; } GDEF;
        markClass acute <anchor 100 700> @TOP;
        feature mark {
            pos base B <anchor 200 500> mark @TOP;
            pos base comma <anchor 650 800> mark @TOP;
            pos base bracket <anchor 150 900> mark @TOP;
        } mark;
    ''' + features)
    return font

fonts = [
    build('feature ccmp { sub A by B; } ccmp; feature vrt2 { sub B by comma; } vrt2;'),
    build('feature smcp { sub A by B; } smcp; feature vrt2 { sub B by comma; } vrt2;'),
    build('feature liga { sub A B by comma; } liga; feature vrt2 { sub comma by bracket; } vrt2;'),
    build('feature locl { sub A by B; } locl; feature ccmp { sub B by comma; } ccmp; feature vrt2 { sub comma by bracket; } vrt2;'),
    build('feature ccmp { sub A by B; } ccmp; feature vert { sub B by comma; } vert;'),
]
collection = TTCollection()
collection.fonts = fonts
collection.save(ROOT / 'vertical-gsub-order.ttc')
