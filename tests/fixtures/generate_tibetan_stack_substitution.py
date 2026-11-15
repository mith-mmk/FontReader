"""Original MIT bounded Tibetan stack fixtures; fontTools 4.68.0.

No external input. Fixed timestamps make regeneration byte-identical.
Faces 0..29 exercise stacks; 30..59 remove abvs/blws for legacy controls.
"""
from copy import deepcopy
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTCollection

ROOT = Path(__file__).parent
GLYPHS = ['.notdef', 'base', 'sub', 'top', 'bottom', 'prep', 'stage', 'stack', 'deep', 'topalt', 'other', 'space']
SYSTEMS = 'languagesystem DFLT dflt; languagesystem tibt dflt; languagesystem tibt JAN; languagesystem latn dflt;'
GDEF = 'table GDEF { GlyphClassDef [base prep stage stack deep space], , [sub top bottom topalt other], ; } GDEF;'
ANCHORS = '''markClass top <anchor 100 700> @TOP;
markClass topalt <anchor 100 700> @TOP;
feature abvm { script tibt; pos base [base prep stage stack deep] <anchor 350 800> mark @TOP; } abvm;'''


def build(features, missing=None, no_gdef=False):
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(GLYPHS)
    cmap = {32: 'space', 65: 'base', 66: 'sub', 0x301: 'top', 0x323: 'bottom'}
    cmap.update({cp: 'base' for cp in range(0xF40, 0xF70)})
    cmap.update({cp: 'sub' for cp in range(0xF90, 0xFBD)})
    cmap.update({cp: 'top' for cp in [0xF72, 0xF7A, 0xF7B, 0xF7C, 0xF7D, 0xF80]})
    cmap.update({cp: 'bottom' for cp in [0xF71, 0xF74]})
    cmap.update({cp: 'other' for cp in [0xF73, 0xF75, 0xF76, 0xF77, 0xF78, 0xF79, 0xF81, 0x200C, 0x200D, 0x34F]})
    if missing is not None:
        cmap.pop(missing)
    fb.setupCharacterMap(cmap)
    outlines = {}
    for i, name in enumerate(GLYPHS):
        pen = TTGlyphPen(None)
        if name != 'space':
            pen.moveTo((0, 0)); pen.lineTo((100 + i * 10, 0))
            pen.lineTo((100 + i * 10, 400)); pen.lineTo((0, 400)); pen.closePath()
        outlines[name] = pen.glyph()
    fb.setupGlyf(outlines)
    fb.setupHorizontalMetrics({g: (0 if g in ['sub', 'top', 'bottom', 'topalt', 'other'] else 600, 0) for g in GLYPHS})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({'familyName': 'MIT Tibetan Stack Test', 'styleName': 'Regular',
                      'uniqueFontIdentifier': 'MIT-Tibetan-Stack-Test',
                      'fullName': 'MIT Tibetan Stack Test', 'psName': 'MITTibetanStackTest'})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    addOpenTypeFeaturesFromString(fb.font, SYSTEMS + GDEF + ANCHORS + features)
    if no_gdef:
        del fb.font['GDEF']
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font

# The presentation calt precedes blws in LookupList, while ccmp is last.
# Preprocessing must execute first, then one ordered presentation group.
STAGES = '''lookup context { sub prep by stage; } context;
lookup below { sub stage sub sub bottom by deep; sub stage sub sub by deep;
               sub stage sub bottom by deep; sub stage sub by stack; sub stage bottom by stack; } below;
lookup above { sub top by topalt; } above;
lookup compose { sub base by prep; } compose;
feature calt { script tibt; lookup context; } calt;
feature blws { script tibt; lookup below; } blws;
feature abvs { script tibt; lookup above; } abvs;
feature ccmp { script tibt; lookup compose; } ccmp;'''
faces = [build(STAGES)]
# 1: unrelated scripts must not activate the path.
faces.append(build('feature blws { script DFLT; sub base sub by deep; script latn; sub base sub by stack; } blws;'))
# 2: no fallback to a DFLT ccmp while explicitly selecting tibt.
faces.append(build('feature ccmp { script DFLT; sub base by prep; } ccmp; feature blws { script tibt; sub base sub by stack; } blws;'))
# 3: retain locale LangSys inside the fixed script.
faces.append(build('feature blws { script tibt; language dflt; sub base sub by stack; language JAN exclude_dflt; sub base sub by deep; } blws;'))
# 4: a lookup shared across presentation features executes only once.
faces.append(build('lookup shared { sub base by prep; sub prep by stage; } shared; feature abvs { script tibt; lookup shared; } abvs; feature blws { script tibt; lookup shared; } blws;'))
# 5/6: conservative GSUB flags with marks, including missing GDEF.
flagged = 'feature blws { script tibt; lookupflag IgnoreMarks; sub base sub by stack; } blws;'
faces.extend([build(flagged), build(flagged, no_gdef=True)])
# 7: the complete cluster lacks a cmap component. Never hide .notdef by GSUB.
faces.append(build('feature blws { script tibt; sub base .notdef by stack; } blws;', missing=0xF90))
# 8: substitution-produced .notdef still makes the whole grapheme fall back.
faces.append(build('feature blws { script tibt; sub base sub by .notdef; } blws;'))
# 9: an unresolved subjoined mark must not grant a retained top mark an anchor.
faces.append(build('feature abvs { script tibt; sub top by topalt; } abvs;'))
# 10: ordinary mark anchors retain priority after a bounded stack ligature.
faces.append(build(STAGES + 'feature mark { pos base stack <anchor 550 600> mark @TOP; } mark;'))
# 11: whole-run presentation sees both lookbehind and lookahead clusters.
faces.append(build("""feature blws { script tibt; sub base sub by stack; } blws;
feature calt { script tibt; sub stack base' by deep; sub base' stack by prep; } calt;"""))
# 12..17: exact no-excluded-glyph proof with filtering metadata.
filtering = 'feature blws { script tibt; lookupflag UseMarkFilteringSet [sub bottom]; sub base sub by stack; sub base bottom by stack; } blws;'
faces.append(build(filtering))
faces.append(build(filtering.replace('[sub bottom]', '[other]')))
f = build(filtering); f['GDEF'].table.MarkGlyphSetsDef = None; faces.append(f)
attachment = filtering.replace('UseMarkFilteringSet', 'MarkAttachmentType')
faces.append(build(attachment))
f = build(attachment); f['GDEF'].table.MarkAttachClassDef = None; faces.append(f)
f = build(filtering); del f['GDEF'].table.GlyphClassDef.classDefs['sub']; faces.append(f)
# 18/19: expansion/recombination and deletion cannot establish source ownership.
faces.append(build('feature ccmp { script tibt; sub base by base base; } ccmp; feature blws { script tibt; sub base base sub by stack; } blws;'))
faces.append(build('feature ccmp { script tibt; sub sub by NULL; } ccmp; feature abvs { script tibt; sub base by prep; } abvs;'))
# 20: no matching new feature must not activate different preprocessing order.
faces.append(build('lookup early { sub prep by stage; } early; feature calt { script tibt; lookup early; } calt; feature ccmp { script tibt; sub base by prep; } ccmp; feature blws { script tibt; sub other other by deep; } blws;'))
# 21: actual orientation alternatives, preserving one selected orientation tag.
faces.append(build('feature blws { script tibt; sub base sub by stack; } blws; feature vert { sub stack by prep; } vert; feature vrt2 { sub stack by deep; } vrt2; feature vrtr { sub stack by stage; } vrtr;'))
# 22: required abvs participates even without an optional feature index.
f = build('feature abvs { script tibt; sub top by topalt; } abvs;')
t = f['GSUB'].table; script = next(x.Script for x in t.ScriptList.ScriptRecord if x.ScriptTag == 'tibt')
script.DefaultLangSys.ReqFeatureIndex = next(i for i,x in enumerate(t.FeatureList.FeatureRecord) if x.FeatureTag == 'abvs')
script.DefaultLangSys.FeatureIndex = []; script.DefaultLangSys.FeatureCount = 0; faces.append(f)
# 23: fonts without GSUB remain entirely on the old path.
f = build(''); faces.append(f)
# 24: Extension preserves its own filter set.
faces.append(build('lookup filtered useExtension { lookupflag UseMarkFilteringSet [sub]; sub base sub by stack; } filtered; feature blws { script tibt; lookup filtered; } blws;'))
# 25: nested filtered lookup must recheck after introducing an excluded mark.
faces.append(build("""lookup change { sub base by other; } change;
lookup filtered { lookupflag UseMarkFilteringSet [sub]; sub other sub by stack; } filtered;
feature blws { script tibt; sub base' lookup change lookup filtered sub; } blws;"""))
# 26/27: flag precedence (IgnoreMarks > filter set > attachment class).
f = build(filtering); f['GSUB'].table.LookupList.Lookup[0].LookupFlag |= 8; faces.append(f)
f = build(filtering); f['GSUB'].table.LookupList.Lookup[0].LookupFlag |= 0x100; faces.append(f)
# 28: a presentation ligature crossing original clusters rolls back.
faces.append(build('feature blws { script tibt; sub base sub by stack; } blws; feature liga { script tibt; sub stack base by deep; } liga;'))
# 29: isolated enhancement repairs a ccmp .notdef, but grouped legacy does not.
faces.append(build('feature ccmp { script tibt; sub base by .notdef; } ccmp; feature blws { script tibt; sub .notdef sub by stack; } blws;'))
controls = [deepcopy(font) for font in faces]
for font in controls:
    if 'GSUB' not in font: continue
    table = font['GSUB'].table
    for feature in table.FeatureList.FeatureRecord:
        if feature.FeatureTag in ['abvs', 'blws']:
            feature.Feature.LookupListIndex = []
            feature.Feature.LookupCount = 0
collection = TTCollection()
collection.fonts = faces + controls
collection.save(ROOT / 'tibetan-stack-substitution.ttc')
