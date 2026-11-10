"""Generate MIT sideways-LTR cases with fontTools 4.65.0.

Read only vertical-metrics.ttf. Faces: chained vrtr, ccmp, smcp, liga,
locl+ccmp, upright-only sentinel, cross-axis GPOS, filtered vrtr, sbix,
raw SVG fallback, and CurrentColor SVG paths. Fixed source timestamps are retained.
"""
from pathlib import Path
from fontTools.ttLib import TTFont, TTCollection, newTable
from fontTools.ttLib.tables.sbixStrike import Strike
from fontTools.ttLib.tables.sbixGlyph import Glyph
import base64
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
ROOT = Path(__file__).parent

def build(ordinary, sideways):
    font = TTFont(ROOT / 'vertical-metrics.ttf', recalcTimestamp=False)
    for table in font['cmap'].tables:
        if table.isUnicode():
            table.cmap.update({ord('f'): 'A', ord('i'): 'B'})
    uvs = CmapSubtable.newSubtable(14)
    uvs.platformID, uvs.platEncID, uvs.language = 0, 5, 0
    uvs.cmap = {}
    uvs.uvsDict = {0xFE00: [(65, 'B')]}
    font['cmap'].tables.append(uvs)
    font['hmtx'].metrics['bracket'] = (900, 100)
    font['vmtx'].metrics['bracket'] = (1700, 300)
    addOpenTypeFeaturesFromString(font, '''
        languagesystem DFLT dflt;
        table GDEF { GlyphClassDef [A B comma bracket], , [acute], ; } GDEF;
        markClass acute <anchor 100 700> @TOP;
        feature mark {
            pos base A <anchor 200 500> mark @TOP;
            pos base B <anchor 300 600> mark @TOP;
            pos base bracket <anchor 650 900> mark @TOP;
        } mark;
        feature kern { pos bracket bracket -70; } kern;
        feature vert { sub A by comma; sub bracket by comma; } vert;
        feature vrt2 { sub A by comma; sub bracket by comma; } vrt2;
    ''' + ordinary + sideways)
    return font

chain = '''lookup first { sub B by comma; } first;
lookup second { sub comma by bracket; } second;
feature vrtr { lookup first; lookup second; } vrtr;'''
collection = TTCollection()
collection.fonts = [
    build('', 'lookup one { sub A by B; } one; lookup two { sub B by bracket; } two; feature vrtr { lookup one; lookup two; } vrtr;'),
    build('feature ccmp { sub A by B; } ccmp;', chain),
    build('feature smcp { sub A by B; } smcp;', chain),
    build('feature liga { sub A B by comma; } liga;', 'feature vrtr { sub comma by bracket; } vrtr;'),
    build('feature locl { sub A by B; } locl; feature ccmp { sub B by comma; } ccmp;', 'feature vrtr { sub comma by bracket; } vrtr;'),
    build('', ''),
]
cross_axis = build('', '')
addOpenTypeFeaturesFromString(cross_axis, '''languagesystem DFLT dflt;
feature kern { pos A <0 0 0 0> B <0 0 0 400>; } kern;''')
collection.fonts.append(cross_axis)
# Face 7: a filtered second lookup must not erase an earlier successful step.
collection.fonts.append(build('', """lookup first { sub A by B; } first;
lookup filtered { lookupflag IgnoreBaseGlyphs; sub B by bracket; } filtered;
feature vrtr { lookup first; lookup filtered; } vrtr;"""))
# Face 8: one valid transparent PNG, selected explicitly by emoji presentation.
raster = build('', '')
raster['sbix'] = newTable('sbix')
strike = Strike(ppem=100, resolution=72)
strike.glyphs['A'] = Glyph(glyphName='A', graphicType='png ', imageData=base64.b64decode(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII='))
raster['sbix'].strikes = {100: strike}
collection.fonts.append(raster)
# Faces 9/10: raw SVG fallback and a supported CurrentColor pathified SVG.
for body in [
    '<defs><mask id="m"><rect width="400" height="600" fill="white"/></mask></defs><path mask="url(#m)" d="M 0 0 L 400 0 L 400 600 Z"/>',
    '<path fill="currentColor" d="M 0 0 L 400 0 L 400 600 Z"/>',
]:
    svg = build('', '')
    svg['SVG '] = newTable('SVG ')
    svg['SVG '].docList = [('<svg id="glyph1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 1000">' + body + '</svg>', 1, 1)]
    collection.fonts.append(svg)
collection.save(ROOT / 'sideways-ltr.ttc')
