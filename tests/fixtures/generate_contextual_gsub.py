"""MIT synthetic contextual GSUB regression faces. Reproduce with fontTools 4.68.0.

Run from repository root; tests use committed bytes, without Python or fonts.
Shared builder regenerates byte-identical preexisting vertical flow fixtures.
"""
from generate_vertical_flow import build, ROOT
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable


def face(features):
    f = build(True).font
    for table in f['cmap'].tables:
        if table.isUnicode():
            table.cmap.update({0x65e5: 'A', 0x672c: 'B'})
    uvs = CmapSubtable.newSubtable(14)
    uvs.platformID, uvs.platEncID, uvs.language = 0, 5, 0
    uvs.cmap = {}
    uvs.uvsDict = {0xE01E1: [(0x65e5, 'bracket')]}
    f['cmap'].tables.append(uvs)
    addOpenTypeFeaturesFromString(f, 'languagesystem DFLT dflt;\n' + features)
    return f

faces = []
# 0..3: contextual substitutions are script/direction independent.
for tag in ['rclt', 'calt', 'clig', 'rlig']:
    faces.append(face(f'feature {tag} {{ sub A B\' by comma; }} {tag};'))
# 4: context consumes an IVS placeholder before ccmp cleans unused placeholders.
faces.append(face('''
feature rclt { sub bracket B by comma; } rclt;
feature ccmp { sub bracket by A; } ccmp;
'''))
# 5: same cleanup order in selected vertical feature; horizontal remains plain.
faces.append(face('''
feature vert { sub bracket B by comma; } vert;
feature ccmp { sub bracket by A; } ccmp;
'''))
# 6: vrt2 selected instead of vert; mixed upright selects only vert.
faces.append(face('''
feature vert { sub A B' by comma; } vert;
feature vrt2 { sub A B' by bracket; } vrt2;
'''))
# 7: feature shared by rclt and liga is applied once, not A->B->comma.
faces.append(face('''
lookup shared { sub A by B; sub B by comma; } shared;
feature rclt { lookup shared; } rclt;
feature liga { lookup shared; } liga;
'''))
# 8: nested multiple substitution changes the input length.
faces.append(face('''
lookup expand { sub B by comma bracket; } expand;
feature calt { sub A B' lookup expand; } calt;
'''))
# 9: sideways context selected only by its explicit route.
faces.append(face("feature vrtr { sub A B' by comma; } vrtr;"))
# 10: contextual ligature must retain its two components for mark-to-ligature.
faces.append(face('\nmarkClass acute <anchor 100 700> @TOP;\nfeature rclt { sub A B by B; } rclt;\nfeature mark {\n    pos base B <anchor 300 500> mark @TOP;\n    pos ligature B <anchor 100 500> mark @TOP\n        ligComponent <anchor 800 500> mark @TOP;\n} mark;\n'))
# 11: IgnoreMarks is safe when GDEF proves the run has no excluded glyphs.
faces.append(face('\ntable GDEF { GlyphClassDef [A B comma bracket], , [acute], ; } GDEF;\nfeature liga { lookupflag IgnoreMarks; sub A B by comma; } liga;\n'))
# 12: orientation retains all-script scope even when ordinary features use locale.
faces.append(face("""
feature vrt2 {
    script DFLT; language dflt; sub A by B;
    script kana; language JAN exclude_dflt; sub B by bracket;
} vrt2;
"""))
collection = TTCollection()
collection.fonts = faces
collection.save(ROOT / 'contextual-gsub.ttc')
