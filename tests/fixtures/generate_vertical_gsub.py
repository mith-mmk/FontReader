"""Generate the MIT GSUB Format 1 fixture with fontTools 4.65.0.

Run from the repository root. The source font is the committed synthetic
vertical-metrics.ttf; no installed fonts or network access are needed.
"""
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from struct import unpack_from

root = Path(__file__).parent
font = TTFont(root / "vertical-metrics.ttf", recalcTimestamp=False)
addOpenTypeFeaturesFromString(font, "feature vert { sub comma by B; sub bracket by comma; } vert;")
font.save(root / "vertical-gsub-format1.ttf")
# Verify the serialized fixture really exercises SingleSubst Format 1.
font = TTFont(root / "vertical-gsub-format1.ttf", recalcTimestamp=False)
lookup = font["GSUB"].table.LookupList.Lookup[0]
assert lookup.LookupType == 1
assert lookup.SubTable[0].mapping == {"comma": "B", "bracket": "comma"}
data = font["GSUB"].compile(font)
lookup_list = unpack_from(">H", data, 8)[0]
lookup_offset = lookup_list + unpack_from(">H", data, lookup_list + 2)[0]
subtable_offset = lookup_offset + unpack_from(">H", data, lookup_offset + 6)[0]
assert unpack_from(">H", data, subtable_offset)[0] == 1
