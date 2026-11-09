"""Generate MIT TrueType vertical-origin cases with fontTools 4.65.0.

Run from the repository root. Importing the shared vertical-flow builder also
regenerates those identical base fixtures. Rust tests use committed bytes only.
"""
from generate_vertical_flow import build, ROOT
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTCollection, newTable
from fontTools.colorLib.builder import buildCOLR, buildCPAL
from fontTools.ttLib.tables.sbixGlyph import Glyph as SbixGlyph
from fontTools.ttLib.tables.sbixStrike import Strike
from fontTools.ttLib.tables.TupleVariation import TupleVariation
import base64

marks = build(True).font
addOpenTypeFeaturesFromString(marks, '''
languagesystem DFLT dflt;
table GDEF { GlyphClassDef [A B comma bracket], , [acute], ; } GDEF;
markClass acute <anchor 100 700> @TOP;
feature mark { pos base A <anchor 300 700> mark @TOP; } mark;
feature mkmk { pos mark acute <anchor 100 800> mark @TOP; } mkmk;
feature vert { sub comma by B; } vert;
''')
negative = build(True).font
negative['vmtx'].metrics['B'] = (800, -150)
composite = build(True).font
pen = TTGlyphPen(composite.getGlyphSet())
pen.addComponent('A', (1, 0, 0, 1, 100, 200))
composite['glyf']['B'] = pen.glyph()
composite['hmtx'].metrics['B'] = (1000, 150)
# Incorrect VORG must be ignored for TrueType outlines (OpenType requirement).
composite['VORG'] = newTable('VORG')
composite['VORG'].majorVersion, composite['VORG'].minorVersion = 1, 0
composite['VORG'].defaultVertOriginY = 3000
composite['VORG'].VOriginRecords = {'B': 4000}
color = build(True).font
# Empty base and both colored outlines share width/height/vertical origin.
color['glyf']['comma'] = TTGlyphPen(None).glyph()
color['hmtx'].metrics['comma'] = (600, 0)
color['hmtx'].metrics['B'] = (600, 100)
color['vmtx'].metrics['comma'] = (1200, 800)
color['vmtx'].metrics['B'] = (1200, 300)
color['COLR'] = buildCOLR({'comma': [('A', 0), ('B', 1)]}, version=0, glyphMap=color.getReverseGlyphMap())
color['CPAL'] = buildCPAL([[(1.0, 0.0, 0.0, 1.0), (0.0, 0.0, 1.0, 1.0)]])
svg = build(True).font
svg['SVG '] = newTable('SVG ')
svg['SVG '].docList = [('<svg xmlns="http://www.w3.org/2000/svg"><g id="glyph1"><rect x="10" y="20" width="20" height="20" fill="#ff0000"/></g></svg>', 1, 1)]
raster = build(True).font
raster['sbix'] = newTable('sbix')
strike = Strike(ppem=100, resolution=72)
png = base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==')
strike.glyphs = {'A': SbixGlyph(glyphName='A', originOffsetX=3, originOffsetY=4,
                               graphicType='png ', imageData=png)}
raster['sbix'].strikes = {100: strike}
variable_builder = build(True)
variable_builder.setupFvar([('wght', 100, 400, 900, 'Weight')], [])
variable = variable_builder.font
variable['gvar'] = newTable('gvar')
variable['gvar'].variations = {name: [] for name in variable.getGlyphOrder()}
variable['gvar'].variations['A'] = [TupleVariation({'wght': (0, 1, 1)},
    [(0, 100)] * 4 + [(0, 0)] * 4)]
collection = TTCollection()
collection.fonts = [marks, negative, composite, color, svg, raster, variable]
collection.save(ROOT / 'vertical-origin-cases.ttc')
# Exercise the independent WOFF table-loading path with a static origin font.
woff = build(True).font
woff.flavor = 'woff'
woff.save(ROOT / 'vertical-origin.woff')
