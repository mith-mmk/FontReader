"""Original MIT lookup-specific predecessor fixtures (fontTools 4.68.0).

Only reads the original mark-coordinates.ttf; never touches installed fonts.
Run from any directory. Face order matches tests/mkmk_predecessor.rs.
"""
from pathlib import Path
from hashlib import sha256
from fontTools.ttLib import TTFont, TTCollection
from fontTools.ttLib.tables import otTables
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

ROOT = Path(__file__).parent
SOURCE = ROOT / 'mark-coordinates.ttf'
SOURCE_HASH = sha256(SOURCE.read_bytes()).hexdigest()
CASES = [
    'plain', 'set', 'class', 'later_filter', 'first_success',
    'eligible_no_coverage', 'eligible_null_anchor', 'zero_anchor',
    'ignore_marks', 'set_precedence', 'covered_base', 'covered_ligature',
    'covered_component', 'missing_gdef', 'unclassified', 'missing_classdef',
    'identity', 'single', 'multiple', 'multiple_one', 'ligature',
    'unclassified_base', 'ignore_base', 'ignore_ligature',
    'expansion_deletion', 'ligature_multiple',
]
fonts = []
for extension in [False, True]:
    for name in CASES:
        f = TTFont(SOURCE)
        f.recalcTimestamp = False
        for table in f['cmap'].tables:
            if table.isUnicode():
                # Mapped joiners are explicit GDEF Marks, not .notdef barriers.
                table.cmap.update({0x302: 'comma', 0x303: 'bracket',
                                   0x200C: 'comma', 0x200D: 'comma'})
        for glyph in ['comma', 'bracket']:
            f['hmtx'].metrics[glyph] = (0, f['hmtx'].metrics[glyph][1])
        for tag in ['GDEF', 'GPOS', 'GSUB']:
            if tag in f:
                del f[tag]
        flag = 'lookupflag UseMarkFilteringSet [acute bracket];'
        if name in ['plain', 'eligible_no_coverage', 'eligible_null_anchor',
                    'covered_base', 'covered_ligature', 'covered_component',
                    'missing_gdef', 'unclassified', 'missing_classdef']:
            flag = ''
        if name == 'class':
            flag = 'lookupflag MarkAttachmentType [acute bracket];'
        if name == 'ignore_marks':
            flag = 'lookupflag IgnoreMarks UseMarkFilteringSet [acute bracket];'
        if name == 'set_precedence':
            flag = 'lookupflag MarkAttachmentType [comma] UseMarkFilteringSet [acute bracket];'
        if name == 'ignore_base':
            flag = 'lookupflag IgnoreBaseGlyphs UseMarkFilteringSet [acute bracket];'
        if name == 'ignore_ligature':
            flag = 'lookupflag IgnoreLigatures UseMarkFilteringSet [acute bracket];'
        gsub = {
            'identity': 'sub A by A;', 'single': 'sub A by B;',
            'multiple': 'sub A by A A;', 'multiple_one': 'sub A by A;',
            'ligature': 'sub A A by B;',
            'expansion_deletion': 'lookup Expand { sub A by A B; } Expand; lookup Delete { sub B by NULL; } Delete;',
            'ligature_multiple': 'lookup Lig { sub A A by B; } Lig; lookup Multi { sub B by A; } Multi;',
        }.get(name, '')
        target = 'bracket' if name == 'zero_anchor' else 'acute'
        if name in ['eligible_null_anchor', 'covered_base', 'covered_ligature', 'covered_component']:
            target = '[acute comma]'
        first = f'lookup First {{ {flag} pos mark {target} <anchor 100 800> mark @TOP; }} First;'
        extra = ''
        if name == 'later_filter':
            first = 'lookup First { lookupflag UseMarkFilteringSet [acute comma]; pos mark bracket <anchor 100 900> mark @TOP; } First;'
            extra = 'lookup Second { lookupflag UseMarkFilteringSet [acute bracket]; pos mark acute <anchor 100 800> mark @TOP; } Second;'
        if name == 'first_success':
            extra = 'lookup Second { lookupflag UseMarkFilteringSet [acute bracket]; pos mark acute <anchor 100 1000> mark @TOP; } Second;'
        bracket_class = '@OTHER' if name == 'zero_anchor' else '@TOP'
        addOpenTypeFeaturesFromString(f, '''languagesystem DFLT dflt;
            markClass acute <anchor 100 700> @TOP;
            markClass bracket <anchor 100 700> ''' + bracket_class + ''';
            markClass comma <anchor 100 700> @OTHER;
            feature ccmp { ''' + gsub + ''' } ccmp;
            feature mark {
                pos base [A B] <anchor 300 700> mark @TOP;
                pos base [A B] <anchor 400 700> mark @OTHER;
            } mark;
            feature mkmk { ''' + first + extra + ' } mkmk;')
        gdef = f['GDEF'].table
        gdef.GlyphClassDef.classDefs.update({'A': 1, 'B': 1, 'acute': 3, 'comma': 3, 'bracket': 3})
        if name.startswith('covered_'):
            gdef.GlyphClassDef.classDefs['comma'] = {'covered_base': 1, 'covered_ligature': 2, 'covered_component': 4}[name]
        if name == 'missing_gdef':
            del f['GDEF']
        if name == 'unclassified':
            gdef.GlyphClassDef.classDefs.pop('acute')
        if name == 'unclassified_base':
            gdef.GlyphClassDef.classDefs.pop('A')
        if name == 'missing_classdef':
            gdef.GlyphClassDef = None
        if name in ['multiple_one', 'ligature_multiple']:
            # Force a true type-2 one-to-one substitution, not feaLib type 1.
            lookup = f['GSUB'].table.LookupList.Lookup[-1]
            sub = otTables.MultipleSubst(); sub.mapping = {'B': ['A']} if name == 'ligature_multiple' else {'A': ['A']}
            lookup.LookupType = 2; lookup.SubTable = [sub]
        for lookup in f['GPOS'].table.LookupList.Lookup:
            if lookup.LookupType == 6:
                for sub in lookup.SubTable:
                    if name == 'eligible_null_anchor':
                        i = sub.Mark2Coverage.glyphs.index('comma')
                        sub.Mark2Array.Mark2Record[i].Mark2Anchor[0] = None
                    if name == 'zero_anchor':
                        for record in sub.Mark1Array.MarkRecord:
                            record.MarkAnchor.XCoordinate = record.MarkAnchor.YCoordinate = 0
                        for record in sub.Mark2Array.Mark2Record:
                            for anchor in record.Mark2Anchor:
                                if anchor:
                                    anchor.XCoordinate = anchor.YCoordinate = 0
            if extension:
                wrapped = []
                for sub in lookup.SubTable:
                    e = otTables.ExtensionPos(); e.Format = 1
                    e.ExtensionLookupType = lookup.LookupType; e.ExtSubTable = sub
                    wrapped.append(e)
                lookup.LookupType = 9; lookup.SubTable = wrapped
        fonts.append(f)
collection = TTCollection(); collection.fonts = fonts
collection.save(ROOT / 'mkmk-predecessor.ttc')
assert sha256(SOURCE.read_bytes()).hexdigest() == SOURCE_HASH
print(f'{len(fonts)} faces; source SHA256 {SOURCE_HASH}')
