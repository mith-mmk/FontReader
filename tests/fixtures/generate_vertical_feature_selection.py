"""Generate MIT vertical feature-selection fixtures with fontTools 4.65.0.

Run from repository root; only the committed synthetic base is required.
Faces: 0 all tags, 1 reversed records, 2 vert, 3 vrtr, 4 uncovered vrt2,
5 kana-only, 6 empty vrt2, 7 invalid lookup index, 8 unreferenced vrt2,
9 named LangSys only, 10 required vrt2, 11 empty-subtable vrt2 lookup.
The base's comma/bracket are distinct stand-ins for C/D.
"""
from pathlib import Path
from copy import deepcopy
from fontTools.ttLib import TTFont, TTCollection
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

root = Path(__file__).parent

def build(features, script="DFLT dflt"):
    font = TTFont(root / "vertical-metrics.ttf", recalcTimestamp=False)
    font["vmtx"].metrics["bracket"] = (1400, 300)
    addOpenTypeFeaturesFromString(font, "languagesystem " + script + ";" + features)
    return font

vert = "feature vert { sub A by B; } vert;"
vrt2 = "feature vrt2 { sub A by comma; } vrt2;"
vrtr = "feature vrtr { sub A by bracket; } vrtr;"
all_tags = build(vert + vrt2 + vrtr)
reverse = deepcopy(all_tags)
table = reverse["GSUB"].table
count = len(table.FeatureList.FeatureRecord)
table.FeatureList.FeatureRecord.reverse()
lang = table.ScriptList.ScriptRecord[0].Script.DefaultLangSys
lang.FeatureIndex = list(range(count))
# Reversed records deliberately vary traversal order, not lookup semantics.
fonts = [all_tags, reverse, build(vert), build(vrtr),
         build(vert + "feature vrt2 { sub B by comma; } vrt2;"),
         build(vert + vrt2 + vrtr, "kana dflt")]
for mode in ["empty", "invalid", "unreferenced", "named", "required", "empty_lookup"]:
    font = build(vert + vrt2)
    table = font["GSUB"].table
    index = next(i for i, rec in enumerate(table.FeatureList.FeatureRecord) if rec.FeatureTag == "vrt2")
    feature = table.FeatureList.FeatureRecord[index].Feature
    lang = table.ScriptList.ScriptRecord[0].Script.DefaultLangSys
    if mode == "empty":
        feature.LookupListIndex = []; feature.LookupCount = 0
    elif mode == "invalid":
        feature.LookupListIndex = [65534]
    elif mode == "unreferenced":
        lang.FeatureIndex.remove(index); lang.FeatureCount -= 1
    elif mode == "named":
        from fontTools.ttLib.tables import otTables
        script = table.ScriptList.ScriptRecord[0].Script
        rec = otTables.LangSysRecord(); rec.LangSysTag = "JAN "; rec.LangSys = lang
        script.DefaultLangSys = None; script.LangSysRecord = [rec]; script.LangSysCount = 1
    elif mode == "required":
        lang.FeatureIndex.remove(index); lang.FeatureCount -= 1; lang.ReqFeatureIndex = index
    else:
        lookup = table.LookupList.Lookup[feature.LookupListIndex[0]]
        lookup.SubTable = []; lookup.SubTableCount = 0
    fonts.append(font)
collection = TTCollection(); collection.fonts = fonts
collection.save(root / "vertical-feature-selection.ttc")
