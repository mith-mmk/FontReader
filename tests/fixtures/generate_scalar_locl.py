"""Original MIT fixtures, fontTools 4.68.0; no third-party font data.
Face 0 nominal; 1..10 direct modes below; 11..20 Extension equivalents.
"""
from pathlib import Path
from copy import deepcopy
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection
from fontTools.ttLib.tables import otTables

b = FontBuilder(1000, isTTF=True)
names = ['.notdef', 'A', 'B', 'C', 'D']
b.setupGlyphOrder(names)
b.setupCharacterMap({65:'A',66:'B',67:'C',68:'D',0x628:'A',0x62A:'B',0x915:'A'})
glyphs = {}
for i, n in enumerate(names):
    p = TTGlyphPen(None)
    p.moveTo((0,0)); p.lineTo((100+i*30,0)); p.lineTo((50,100+i*20)); p.closePath()
    glyphs[n] = p.glyph()
b.setupGlyf(glyphs)
b.setupHorizontalMetrics({n:(500+i*50,0) for i,n in enumerate(names)})
b.setupHorizontalHeader(ascent=800,descent=-200,lineGap=0)
b.setupNameTable({'familyName':'Fontcore Scalar Locl','styleName':'Regular','uniqueFontIdentifier':'FontcoreScalarLocl','fullName':'Fontcore Scalar Locl','psName':'FontcoreScalarLocl'})
b.setupOS2(sTypoAscender=800,sTypoDescender=-200,usWinAscent=800,usWinDescent=200)
b.setupPost()
b.font['head'].created = b.font['head'].modified = 2082844800
b.font.recalcTimestamp = False
# blocked, one-input, ordered records, identity, next lookup, Single,
# Alternate, legacy Multiple, empty Multiple, real liga after blocked locl.
modes = ['sub A B by C;', 'sub A B by C;', 'sub A B by C; sub A D by D;',
         'sub A B by A;', 'sub A B by C; sub A by D;', 'sub A by C;',
         'sub A from [C D];', 'sub A by C D;', 'sub A by NULL;', 'sub A B by C;']
fonts = [deepcopy(b.font)]
for extended in (False, True):
    for mode, rules in enumerate(modes):
        f = deepcopy(b.font)
        features = 'languagesystem DFLT dflt; languagesystem arab dflt; languagesystem deva dflt; '
        for tag in ['locl','isol','init','medi','fina']:
            features += f'feature {tag} {{ {rules} }} {tag}; '
        if mode == 9:
            features += 'feature liga { sub A B by D; } liga;'
        addOpenTypeFeaturesFromString(f, features)
        for lookup in f['GSUB'].table.LookupList.Lookup:
            if lookup.LookupType == 4 and mode in (1,2,3):
                for st in lookup.SubTable:
                    record = st.ligatures['A'][1 if mode == 2 else 0]
                    record.Component = []; record.CompCount = 1
            if extended:
                subs = []
                for sub in lookup.SubTable:
                    ext = otTables.ExtensionSubst()
                    ext.Format = 1; ext.ExtensionLookupType = lookup.LookupType; ext.ExtSubTable = sub
                    subs.append(ext)
                lookup.LookupType = 7; lookup.SubTable = subs
        fonts.append(f)
collection = TTCollection(); collection.fonts = fonts
collection.save(Path(__file__).with_name('scalar-locl.ttc'))
