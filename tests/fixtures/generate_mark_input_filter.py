"""Original MIT lookup-input filtering variants, fontTools 4.68.0.
Uses the committed original MIT mark-coordinates.ttf; never installed fonts.
Run from repository root. Face order is recorded in mark_input_filter.rs.
"""
from pathlib import Path
from fontTools.ttLib import TTFont, TTCollection
from fontTools.ttLib.tables import otTables
ROOT = Path(__file__).parent
CASES = [
    ('plain',0,None,None), ('ignore_base',2,None,None),
    ('ignore_ligature',4,None,None), ('ignore_mark',8,None,None),
    ('missing_attach_class',256,None,None), ('attach_match',256,1,None),
    ('attach_mismatch',512,1,None), ('set_include',16,None,True),
    ('set_exclude',16,None,False), ('set_precedence',272,None,True),
    ('ignore_precedence',24,None,True), ('missing_set',16,None,None),
    ('invalid_set_index',16,None,True), ('missing_gdef',8,None,None),
    ('unclassified',8,None,None), ('zero_anchor',0,None,None),
    ('later_lookup',8,None,None),
]
fonts=[]
for extension in [False,True]:
    for name,flag,attach,markset in CASES:
        f=TTFont(ROOT/'mark-coordinates.ttf'); f.recalcTimestamp=False
        g=f['GDEF'].table
        if attach is not None:
            d=otTables.ClassDef();d.classDefs={'acute':attach};g.MarkAttachClassDef=d
        if markset is not None:
            g.Version=0x00010002;s=otTables.MarkGlyphSetsDef();s.MarkSetTableFormat=1
            c=otTables.Coverage();c.glyphs=['acute'] if markset else ['A']
            s.Coverage=[c];s.MarkSetCount=1;g.MarkGlyphSetsDef=s
        if name=='missing_gdef':del f['GDEF']
        if name=='unclassified':g.GlyphClassDef.classDefs.pop('acute')
        if name=='later_lookup':
            import copy
            table=f['GPOS'].table
            originals=list(table.LookupList.Lookup)
            for feature in table.FeatureList.FeatureRecord:
                feature.Feature.LookupListIndex += [len(originals)+i for i in feature.Feature.LookupListIndex]
                feature.Feature.LookupCount=len(feature.Feature.LookupListIndex)
            table.LookupList.Lookup += copy.deepcopy(originals)
            table.LookupList.LookupCount=len(table.LookupList.Lookup)
        for i,l in enumerate(f['GPOS'].table.LookupList.Lookup):
            l.LookupFlag=0 if name=='later_lookup' and i>=3 else flag
            if flag&16:l.MarkFilteringSet=9 if name=='invalid_set_index' else 0
            if name=='zero_anchor':
                for sub in l.SubTable:
                    for record in sub.MarkArray.MarkRecord if l.LookupType in [4,5] else sub.Mark1Array.MarkRecord:
                        record.MarkAnchor.XCoordinate=0;record.MarkAnchor.YCoordinate=0
                    if l.LookupType==4:
                        anchors=[a for r in sub.BaseArray.BaseRecord for a in r.BaseAnchor]
                    elif l.LookupType==5:
                        anchors=[a for r in sub.LigatureArray.LigatureAttach for c in r.ComponentRecord for a in c.LigatureAnchor]
                    else:anchors=[a for r in sub.Mark2Array.Mark2Record for a in r.Mark2Anchor]
                    for a in anchors:
                        if a:a.XCoordinate=0;a.YCoordinate=0
            if extension:
                kind=l.LookupType;wrapped=[]
                for sub in l.SubTable:
                    e=otTables.ExtensionPos();e.Format=1;e.ExtensionLookupType=kind;e.ExtSubTable=sub;wrapped.append(e)
                l.LookupType=9;l.SubTable=wrapped
        fonts.append(f)
c=TTCollection();c.fonts=fonts;c.save(ROOT/'mark-input-filter.ttc')
