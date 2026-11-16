"""Original MIT fixtures for bounded Old Hangul. fontTools 4.68.0; no external font data."""
from pathlib import Path
from copy import deepcopy
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection
from fontTools.ttLib.tables import otTables

names = ['.notdef','L','V','T','l','v','t','bad','A','ga','tone']
common = """languagesystem hang dflt;
lookup L1 { sub L by l; } L1;
lookup V1 { sub V by v; } V1;
lookup T1 { sub T by t; } T1;
"""
good = """
feature ljmo { sub L' lookup L1 V T; } ljmo;
feature vjmo { sub l V' lookup V1 T; } vjmo;
feature tjmo { sub l v T' lookup T1; } tjmo;
"""
features = [
'', common+good,
common+"""lookup Shared { sub L by l; sub V by v; sub l by bad; } Shared;
feature ljmo { lookup Shared; } ljmo;
feature vjmo { lookup Shared; } vjmo;
feature tjmo { lookup T1; } tjmo;""",
(common+good).replace('hang','latn'),
common+"""feature ljmo { sub L V' lookup V1 T; } ljmo;""",
common+"""feature ljmo { sub L' lookup L1 V'; } ljmo;""",
common+"""feature ccmp { sub L V T by bad; } ccmp;"""+good,
"""languagesystem hang dflt; lookup L1 useExtension { sub L by l; } L1;
feature ljmo { lookup L1; } ljmo;
feature vjmo { sub V by v; } vjmo;
feature tjmo { sub T by t; } tjmo;""",
"""languagesystem hang dflt; lookup L1 { lookupflag IgnoreMarks; sub L by l; } L1;
feature ljmo { sub L' lookup L1 V T; } ljmo;""",
common+"""feature ljmo { sub L by l bad; } ljmo;""",
common+"""feature ljmo { sub L by .notdef; } ljmo;""",
common+good,
common+good+"""feature rvrn { sub L by bad; } rvrn;""",
common+good+"""feature calt { sub L by .notdef; } calt;""",
common+good,
common+good+"""feature ccmp { sub L V T by bad; } ccmp;""",
common+good, common+good, common+good,
]
def build(index):
    fb=FontBuilder(1000,isTTF=True)
    fb.setupGlyphOrder(names)
    cmap={0x1100:'L',0x1161:'V',0x11eb:'T',0xa960:'L',0xd7b0:'V',0xd7cb:'T',
          0x119e:'V',0x115f:'L',0x1160:'V',0xac00:'ga',0x302e:'tone',65:'A',
          0xe000:'l',0xe001:'v',0xe002:'t',0xe003:'bad'}
    fb.setupCharacterMap(cmap)
    outlines={}
    for i,n in enumerate(names):
        p=TTGlyphPen(None);p.moveTo((i*10,0));p.lineTo((i*10+80,0));p.lineTo((i*10+80,400));p.closePath();outlines[n]=p.glyph()
    fb.setupGlyf(outlines)
    fb.setupHorizontalMetrics({n:(0 if n in ['v','t'] else 1000,0) for n in names})
    fb.setupHorizontalHeader(ascent=800,descent=-200)
    fb.setupNameTable({'familyName':'MIT Bounded Hangul '+str(index),'styleName':'Regular','uniqueFontIdentifier':'bounded-'+str(index),'fullName':'MIT Bounded Hangul','psName':'BoundedHangul'+str(index)})
    fb.setupOS2(sTypoAscender=800,sTypoDescender=-200,usWinAscent=800,usWinDescent=200)
    fb.setupPost();fb.setupMaxp()
    if features[index]: addOpenTypeFeaturesFromString(fb.font,features[index])
    if features[index]:
        # Force equivalent format3 so the fixture tests the admitted representation.
        for lookup in fb.font['GSUB'].table.LookupList.Lookup:
            if lookup.LookupType != 6: continue
            converted=[]
            for old in lookup.SubTable:
                if old.Format != 1: converted.append(old); continue
                for first, rules in zip(old.Coverage.glyphs, old.ChainSubRuleSet):
                    for rule in rules.ChainSubRule:
                        new=otTables.ChainContextSubst();new.Format=3
                        def covers(values):
                            result=[]
                            for value in values:
                                c=otTables.Coverage();c.glyphs=[value];result.append(c)
                            return result
                        new.BacktrackCoverage=covers(rule.Backtrack);new.BacktrackGlyphCount=len(new.BacktrackCoverage)
                        new.InputCoverage=covers([first]+rule.Input);new.InputGlyphCount=len(new.InputCoverage)
                        new.LookAheadCoverage=covers(rule.LookAhead);new.LookAheadGlyphCount=len(new.LookAheadCoverage)
                        new.SubstLookupRecord=rule.SubstLookupRecord;new.SubstCount=len(new.SubstLookupRecord)
                        converted.append(new)
            lookup.SubTable=converted;lookup.SubTableCount=len(converted)
    if index==11:
        lang=fb.font['GSUB'].table.ScriptList.ScriptRecord[0].Script.DefaultLangSys
        lang.ReqFeatureIndex=lang.FeatureIndex[0]
    if index==14:
        # Applicable contextual lookup with a nested out-of-range target.
        table=fb.font['GSUB'].table.LookupList.Lookup[3].SubTable[0]
        table.SubstLookupRecord[0].LookupListIndex=65535
    if index==16:
        gsub=fb.font['GSUB'].table
        replacement=deepcopy(gsub.LookupList.Lookup[0]);replacement.SubTable[0].mapping={'L':'bad'}
        li=len(gsub.LookupList.Lookup);gsub.LookupList.Lookup.append(replacement);gsub.LookupList.LookupCount+=1
        record=otTables.FeatureRecord();record.FeatureTag='ljmo';record.Feature=otTables.Feature()
        record.Feature.LookupListIndex=[li];record.Feature.LookupCount=1;record.Feature.FeatureParams=None
        fi=len(gsub.FeatureList.FeatureRecord);gsub.FeatureList.FeatureRecord.append(record);gsub.FeatureList.FeatureCount+=1
        lang=otTables.LangSysRecord();lang.LangSysTag='KOR ';lang.LangSys=otTables.LangSys()
        lang.LangSys.LookupOrder=None;lang.LangSys.ReqFeatureIndex=65535;lang.LangSys.FeatureIndex=[fi];lang.LangSys.FeatureCount=1
        script=gsub.ScriptList.ScriptRecord[0].Script;script.LangSysRecord=[lang];script.LangSysCount=1
    if index==17: fb.font['GSUB'].table.LookupList.Lookup[3].LookupFlag=8
    if index==18: fb.font['GSUB'].table.LookupList.Lookup[3].SubTable[0].SubstLookupRecord[0].LookupListIndex=3
    fb.font['head'].created=fb.font['head'].modified=2082844800
    return fb.font
if __name__=='__main__':
    collection=TTCollection();collection.fonts=[build(i) for i in range(len(features))]
    collection.save(Path(__file__).with_name('bounded-hangul.ttc'))
