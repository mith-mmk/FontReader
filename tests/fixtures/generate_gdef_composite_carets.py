"""MIT fixture: bounded static translated composite GDEF points (fontTools)."""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection
from fontTools.ttLib.tables._g_l_y_f import GlyphComponent


def build(index):
    order=['.notdef','f','i','left','right','pair','n0','n1','n2','n3','n4','fi']
    glyphs={}
    for name in order[:5]:
        pen=TTGlyphPen(None)
        x=-30 if name=='right' else 10
        pen.moveTo((x,0)); pen.lineTo((x+100,0)); pen.lineTo((x+100,500)); pen.lineTo((x,500)); pen.closePath()
        glyphs[name]=pen.glyph()
    def composite(name,parts):
        pen=TTGlyphPen(glyphs)
        for part,transform in parts: pen.addComponent(part,transform)
        glyphs[name]=pen.glyph()
    identity=(1,0,0,1,0,0)
    composite('pair',[('left',(1,0,0,1,100,0)),('right',(1,0,0,1,400,50))])
    for n in range(5): composite('n'+str(n),[('pair' if n==0 else 'n'+str(n-1),identity)])
    if index==1: composite('fi',[('pair',(1,0,0,1,20,30))])
    elif index==6: composite('fi',[('n4',identity)])
    elif index==4: composite('fi',[('left',(0.5,0,0,0.5,100,0)),('right',(1,0,0,1,400,50))])
    else: composite('fi',[('left',(1,0,0,1,100,0)),('right',(1,0,0,1,400,50))])
    if index==3: glyphs['fi'].components[0].flags |= 0x200
    if index==5:
        attachment=GlyphComponent(); attachment.glyphName='right'; attachment.firstPt=1; attachment.secondPt=0; attachment.flags=0
        glyphs['fi'].components[1]=attachment
    fb=FontBuilder(1000,isTTF=True);fb.setupGlyphOrder(order)
    fb.setupCharacterMap({102:'f',105:'i'});fb.setupGlyf(glyphs)
    metrics={name:(900,getattr(glyph,'xMin',0)) for name,glyph in glyphs.items()}
    metrics['left']=(600,-90);metrics['right']=(600,200)
    if index==2: metrics['fi']=(900,0)
    fb.setupHorizontalMetrics(metrics);fb.setupHorizontalHeader(ascent=800,descent=-200)
    name=f'MITCompositeCaret{index}'
    fb.setupNameTable(dict(familyName=name,styleName='Regular',uniqueFontIdentifier=name,fullName=name,psName=name))
    fb.setupOS2(sTypoAscender=800,sTypoDescender=-200,usWinAscent=800,usWinDescent=200);fb.setupPost()
    point=8 if index==7 else 5
    addOpenTypeFeaturesFromString(fb.font,f'''languagesystem DFLT dflt;
        feature liga {{ sub f i by fi; }} liga;
        table GDEF {{ LigatureCaretByIndex fi {point}; }} GDEF;''')
    fb.font['head'].created=fb.font['head'].modified=2082844800;fb.font.recalcTimestamp=False
    return fb.font

if __name__=='__main__':
    collection=TTCollection();collection.fonts=[build(i) for i in range(8)]
    collection.save(Path(__file__).with_name('gdef-composite-carets.ttc'))
