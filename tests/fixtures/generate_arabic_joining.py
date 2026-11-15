"""Generate MIT synthetic Arabic joining fixture with FontTools 4.68.0."""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
b = FontBuilder(1000, isTTF=True)
names = ['.notdef', 'beh', 'part', 'dot', 'fatha', 'teh', 'composed', 'initial', 'final', 'composedInitial', 'tatweel', 'space', 'zwnj', 'zwj']
b.setupGlyphOrder(names)
b.setupCharacterMap({0x628:'beh',0x62A:'teh',0x64E:'fatha',0x640:'tatweel',32:'space',0x200C:'zwnj',0x200D:'zwj'})
outlines = {}
for name in names:
    p = TTGlyphPen(None)
    if name not in ['.notdef','space','zwj','zwnj']:
        p.moveTo((0,0)); p.lineTo((100,0)); p.lineTo((50,100)); p.closePath()
    outlines[name] = p.glyph()
b.setupGlyf(outlines)
widths = {'initial':200,'final':300,'composedInitial':250,'dot':0,'fatha':0,'tatweel':100,'zwj':0,'zwnj':0}
b.setupHorizontalMetrics({n:(widths.get(n,600),0) for n in names})
b.setupHorizontalHeader(ascent=800,descent=-200,lineGap=0)
b.setupNameTable({'familyName':'Fontcore Arabic Joining','styleName':'Regular','uniqueFontIdentifier':'FontcoreArabicJoining1','fullName':'Fontcore Arabic Joining','psName':'FontcoreArabicJoining'})
b.setupOS2(sTypoAscender=800,sTypoDescender=-200,usWinAscent=800,usWinDescent=200)
b.setupPost()
addOpenTypeFeaturesFromString(b.font, """
languagesystem DFLT dflt;
languagesystem arab dflt;
feature ccmp {
    sub beh by part dot;
    sub teh fatha by composed;
} ccmp;
feature init { sub part by initial; sub composed by composedInitial; } init;
feature fina { sub part by final; } fina;
""")
b.font['head'].created = b.font['head'].modified = 2082844800
b.font.recalcTimestamp = False
b.save(Path(__file__).with_name('arabic-joining.ttf'))
