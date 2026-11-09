"""Generate MIT vertical lookup-chain fixtures with fontTools 4.65.0.

Run from the repository root. Uses only the committed synthetic base.
Faces: direct, reversed indices, extension, first subtable, missed subtable,
identity, cycle, duplicate LangSys references, multiple, ligature, alternate,
contextual, reverse contextual, invalid index, empty lookup, extension multiple.
Then vert chain, unsupported vrt2 with vert, direct/extension Single format 2.
"""
from copy import deepcopy
from pathlib import Path
from fontTools.ttLib import TTFont, TTCollection
from fontTools.ttLib.tables import otTables
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

root = Path(__file__).parent


def build(body):
    font = TTFont(root / "vertical-metrics.ttf", recalcTimestamp=False)
    addOpenTypeFeaturesFromString(font, "languagesystem DFLT dflt;" + body)
    return font


def chain(extension=""):
    return build(f"""
        lookup L0 {extension} {{ sub A by B; }} L0;
        lookup L1 {extension} {{ sub B by comma; }} L1;
        feature vrt2 {{ lookup L0; lookup L1; }} vrt2;
    """)


fonts = [chain(), chain(), chain("useExtension")]
fonts[1]["GSUB"].table.FeatureList.FeatureRecord[0].Feature.LookupListIndex.reverse()
for first in ["A", "bracket"]:
    font = chain()
    table = font["GSUB"].table
    lookup = table.LookupList.Lookup[0]
    lookup.SubTable[0].mapping = {first: "B"}
    second = deepcopy(lookup.SubTable[0]); second.mapping = {"A": "comma", "B": "comma"}
    lookup.SubTable.append(second); lookup.SubTableCount = 2
    table.FeatureList.FeatureRecord[0].Feature.LookupListIndex = [0]
    fonts.append(font)
fonts.append(build("feature vrt2 { sub A by A; } vrt2;"))
fonts.append(build("""
    lookup L0 { sub A by B; } L0;
    lookup L1 { sub B by A; } L1;
    feature vrt2 { lookup L0; lookup L1; } vrt2;
"""))
font = build("feature vrt2 { sub A by B; sub B by comma; } vrt2;")
script = font["GSUB"].table.ScriptList.ScriptRecord[0].Script
record = otTables.LangSysRecord(); record.LangSysTag = "JAN "
record.LangSys = deepcopy(script.DefaultLangSys)
script.LangSysRecord = [record]; script.LangSysCount = 1
feature = font["GSUB"].table.FeatureList.FeatureRecord[0].Feature
feature.LookupListIndex = [0, 0]; feature.LookupCount = 2
fonts.append(font)
for rule in ["sub A by B comma;", "sub A B by comma;", "sub A from [B comma];",
             "sub A' B by comma;", "rsub A' B by comma;"]:
    fonts.append(build("feature vrt2 { " + rule + " } vrt2;"))
font = chain()
feature = font["GSUB"].table.FeatureList.FeatureRecord[0].Feature
feature.LookupListIndex = [65534, 1, 0]; feature.LookupCount = 3
fonts.append(font)
font = chain()
font["GSUB"].table.LookupList.Lookup[0].SubTable = []
font["GSUB"].table.LookupList.Lookup[0].SubTableCount = 0
fonts.append(font)
fonts.append(build("""
    lookup L0 useExtension { sub A by B comma; } L0;
    feature vrt2 { lookup L0; } vrt2;
"""))
font = chain()
font["GSUB"].table.FeatureList.FeatureRecord[0].FeatureTag = "vert"
fonts.append(font)
fonts.append(build("feature vert { sub A by B; } vert; feature vrt2 { sub A by B comma; } vrt2;"))
for extension in ["", "useExtension"]:
    fonts.append(build(f"""
        lookup L0 {extension} {{ sub A by B; sub bracket by A; }} L0;
        lookup L1 {extension} {{ sub B by comma; sub bracket by A; }} L1;
        feature vrt2 {{ lookup L0; lookup L1; }} vrt2;
    """))
collection = TTCollection(); collection.fonts = fonts
collection.save(root / "vertical-lookup-chain.ttc")
