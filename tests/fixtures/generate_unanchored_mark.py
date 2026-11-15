"""Original MIT fixtures derived only from mark-coordinates.ttf.
fontTools 4.68.0. Order: mode, nominal advance, GDEF evidence.
"""
from pathlib import Path
from fontTools.ttLib import TTFont, TTCollection
from fontTools.ttLib.tables import otTables
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
ROOT = Path(__file__).parent
MODES = ['anchor', 'filtered', 'coverage_absent', 'single', 'pair', 'no_gpos']
fonts = []
for mode in MODES:
    for advance in [0, 150]:
        for evidence in ['none', 'class', 'points']:
            f = TTFont(ROOT / 'mark-coordinates.ttf', recalcTimestamp=False)
            del f['GPOS']; del f['GSUB']; del f['GDEF']
            features = 'languagesystem DFLT dflt;\n'
            if mode in ['anchor', 'filtered', 'coverage_absent']:
                base = 'B' if mode == 'coverage_absent' else 'A'
                flag = 'lookupflag IgnoreMarks;' if mode == 'filtered' else ''
                features += f'markClass acute <anchor 100 700> @TOP; feature mark {{ {flag} pos base {base} <anchor 300 700> mark @TOP; }} mark;'
            elif mode == 'single':
                features += 'feature kern { pos acute <17 23 31 43>; } kern;'
            elif mode == 'pair':
                features += 'feature kern { pos A <5 7 -11 13> acute <17 23 31 43>; } kern;'
            features += 'table GDEF { GlyphClassDef [A B], , [acute], ; } GDEF;'
            addOpenTypeFeaturesFromString(f, features)
            f['hmtx'].metrics['acute'] = (advance, f['hmtx'].metrics['acute'][1])
            g = f['GDEF'].table
            if evidence == 'class':
                d = otTables.ClassDef(); d.classDefs = {'acute': 1}; g.MarkAttachClassDef = d
            elif evidence == 'points':
                a = otTables.AttachList(); a.Coverage = otTables.Coverage(); a.Coverage.glyphs = ['acute']
                p = otTables.AttachPoint(); p.PointIndex = [0]; p.PointCount = 1
                a.AttachPoint = [p]; a.GlyphCount = 1; g.AttachList = a
            fonts.append(f)
import copy
vertical = TTFont(ROOT / 'vertical-metrics.ttf', recalcTimestamp=False)
for f in list(fonts):
    f = copy.deepcopy(f)
    f['vhea'] = copy.deepcopy(vertical['vhea'])
    f['vmtx'] = copy.deepcopy(vertical['vmtx'])
    f['vmtx'].metrics['acute'] = (f['hmtx'].metrics['acute'][0], 200)
    fonts.append(f)
c = TTCollection(); c.fonts = fonts; c.save(ROOT / 'unanchored-mark.ttc')
