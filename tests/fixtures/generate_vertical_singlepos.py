"""Generate MIT-licensed synthetic SinglePos fixtures (fontTools 4.68.0).

Run from the repository root. FontTools is only needed to regenerate the files;
the Rust tests use the committed bytes and do not need an external font corpus.
"""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

root = Path(__file__).parent
builder = FontBuilder(1000, isTTF=True)
glyphs = ['.notdef', 'A', 'B', 'C', 'acute']
builder.setupGlyphOrder(glyphs)
builder.setupCharacterMap({65: 'A', 66: 'B', 67: 'C', 0x31F7: 'A', 0x3099: 'acute', 0x309A: 'acute'})
outlines = {}
for name in glyphs:
    pen = TTGlyphPen(None)
    if name != '.notdef':
        pen.moveTo((50, 0)); pen.lineTo((450, 0)); pen.lineTo((250, 500)); pen.closePath()
    outlines[name] = pen.glyph()
builder.setupGlyf(outlines)
builder.setupHorizontalMetrics({name: (0 if name == 'acute' else 600, 50) for name in glyphs})
builder.setupHorizontalHeader(ascent=800, descent=-200, lineGap=0)
builder.setupVerticalMetrics({name: (1000, 100) for name in glyphs})
builder.setupVerticalHeader(ascent=800, descent=-200, lineGap=0)
builder.setupNameTable({'familyName': 'Fontcore Synthetic SinglePos', 'styleName': 'Regular',
                       'uniqueFontIdentifier': 'Fontcore-SinglePos-1', 'fullName': 'Fontcore Synthetic SinglePos',
                       'psName': 'FontcoreSyntheticSinglePos'})
builder.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
builder.setupPost()
addOpenTypeFeaturesFromString(builder.font, """
languagesystem DFLT dflt;
feature vert { pos acute <500 1000 0 -1000>; pos B <-20 -30 40 50>; } vert;
""")
builder.font['head'].created = builder.font['head'].modified = 2082844800
builder.font.recalcTimestamp = False
builder.save(root / 'vertical-singlepos.ttf')
del builder.font['GPOS']
builder.save(root / 'vertical-singlepos-control.ttf')
