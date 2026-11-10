"""Generate MIT static COLR v0 origin cases with fontTools 4.65.0.

Run from repository root; imports regenerate identical shared base fixtures.
Rust tests use committed bytes, without Python or external fonts.
"""
from copy import deepcopy
from generate_vertical_flow import build, ROOT
from fontTools.colorLib.builder import buildCOLR, buildCPAL
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTCollection

fb = build(True)
font = fb.font
font['glyf']['comma'] = TTGlyphPen(None).glyph()
font['hmtx'].metrics['comma'] = (600, 0)
font['hmtx'].metrics['B'] = (600, 100)
font['vmtx'].metrics['comma'] = (1200, 800)
font['vmtx'].metrics['B'] = (1200, 300)
font['COLR'] = buildCOLR({'comma': [('A', 0), ('B', 1)], 'acute': [('acute', 0xFFFF)]},
                         version=0, glyphMap=font.getReverseGlyphMap())
font['CPAL'] = buildCPAL([[(1., 0., 0., 1.), (0., 0., 1., 1.)]])
addOpenTypeFeaturesFromString(font, '''
languagesystem DFLT dflt;
table GDEF { GlyphClassDef [A B comma bracket], , [acute], ; } GDEF;
markClass acute <anchor 100 700> @TOP;
feature mark { pos base comma <anchor 300 700> mark @TOP; } mark;
feature mkmk { pos mark acute <anchor 100 800> mark @TOP; } mkmk;
''')
negative = deepcopy(font)
for name, tsb in [('comma', 400), ('A', -300), ('B', -100)]:
    negative['vmtx'].metrics[name] = (1200, tsb)
missing = deepcopy(font)
del missing['vhea']; del missing['vmtx']
no_cpal = deepcopy(font)
del no_cpal['CPAL']
# A visible monochrome fallback makes accidental COLR translations observable.
no_cpal['glyf']['comma'] = deepcopy(font['glyf']['A'])
invalid_palette = deepcopy(no_cpal)
invalid_palette['CPAL'] = buildCPAL([[(1., 0., 0., 1.)]])
for layer in invalid_palette['COLR'].ColorLayers['comma']:
    layer.colorID = 7
variable_builder = build(True)
variable_builder.setupFvar([('wght', 100, 400, 900, 'Weight')], [])
variable = deepcopy(font)
variable['fvar'] = variable_builder.font['fvar']
variable['name'] = variable_builder.font['name']
mono_mark = deepcopy(font)
del mono_mark['COLR'].ColorLayers['acute']
nonempty = deepcopy(negative)
nonempty['glyf']['comma'] = deepcopy(nonempty['glyf']['A'])
nonempty['hmtx'].metrics['comma'] = (600, 50)
nonempty['vmtx'].metrics['comma'] = (1200, -300)
collection = TTCollection()
collection.fonts = [font, negative, missing, no_cpal, invalid_palette, variable, mono_mark, nonempty]
collection.save(ROOT / 'colr-vertical-origin.ttc')
