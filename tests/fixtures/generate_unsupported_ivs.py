"""Original MIT synthetic IVS fixtures, using fontTools 4.68.0.

Reuses original MIT vertical-metrics outlines. No vendor font data.
Faces: nominal, unsupported pair, nondefault, default, gid0 contextual
repair, unrepaired gid0, nondefault without combining-mark coverage, invalid
mark cmap, distinct nominal continuity face, invalid post-GSUB mark.
"""
from pathlib import Path
from fontTools.ttLib import TTFont, TTCollection
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
ROOT = Path(__file__).parent
fonts = []
for index in range(10):
    font = TTFont(ROOT / 'vertical-metrics.ttf', recalcTimestamp=False)
    for tag in ['GSUB', 'GPOS']:
        if tag in font:
            del font[tag]
    cmap = {0x3042: 'A', 0x3044: 'B', 0x20000: 'A', 0x3001: 'comma',
            65: 'A', 66: 'B', 0x301: 'acute'}
    if index == 4:
        cmap[0x301] = 'B'  # EGC-local context repairs the explicit zero
    if index == 8:
        cmap[0x3046] = 'B'
        cmap[0x3042] = 'B'
    if index == 6:
        del cmap[0x301]
    nominal = CmapSubtable.newSubtable(12)
    nominal.platformID, nominal.platEncID, nominal.language = 3, 10, 0
    nominal.cmap = cmap
    font['cmap'].tables = [nominal]
    if index and index != 8:
        uvs = CmapSubtable.newSubtable(14)
        uvs.platformID, uvs.platEncID, uvs.language = 0, 5, 0
        uvs.cmap = {}
        value = None if index == 3 else ('.notdef' if index in (4, 5) else 'B')
        entries = [(0x3044, None)] if index == 1 else [(65, value), (0x3042, value), (0x20000, value)]
        uvs.uvsDict = {0xE0100: entries, 0xFE00: entries}
        font['cmap'].tables.append(uvs)
    if index == 4:
        addOpenTypeFeaturesFromString(font, '''languagesystem DFLT dflt;
            feature ccmp { sub .notdef' B by comma; } ccmp;''')
    if index == 9:
        addOpenTypeFeaturesFromString(font, 'languagesystem DFLT dflt; feature ccmp { sub acute by B; } ccmp;')
        font['GSUB'].table.LookupList.Lookup[0].SubTable[0].mapping['acute'] = 'glyph65534'
    if index == 7:
        nominal.cmap[0x301] = 'glyph65534'  # deliberately invalid nonzero mapping
    fonts.append(font)
collection = TTCollection()
collection.fonts = fonts
collection.save(ROOT / 'unsupported-ivs.ttc')

# Both CFF formats share the same coverage probe. Invalid cmap glyph IDs must
# not win family preference merely because a CFF table exists.
from copy import deepcopy
for kind in ['cff', 'cff2']:
    source = TTCollection(ROOT / f'{kind}-vertical-origin.ttc', recalcTimestamp=False).fonts[0]
    pair = []
    for invalid in [0, 1, 2]:
        font = deepcopy(source)
        for tag in ['GSUB', 'GPOS']:
            if tag in font:
                del font[tag]
        uvs = CmapSubtable.newSubtable(14)
        uvs.platformID, uvs.platEncID, uvs.language = 0, 5, 0
        uvs.cmap = {}
        uvs.uvsDict = {0xE0100: [(65, 'glyph65534'), (0x301, 'glyph65534')]}
        if invalid == 2:
            uvs.uvsDict[0xE0100][0] = (65, 'B')
            from fontTools.colorLib.builder import buildCPAL
            from fontTools.ttLib.tables.DefaultTable import DefaultTable
            import struct
            font['COLR'] = DefaultTable('COLR')
            font['COLR'].data = struct.pack('>HHIIHHHHHH', 0, 1, 14, 20, 1, 65534, 0, 1, font.getGlyphID('A'), 0)
            font['CPAL'] = buildCPAL([[(0, 0, 0, 1)]])
        if invalid:
            font['cmap'].tables.append(uvs)
        pair.append(font)
    collection = TTCollection()
    collection.fonts = pair
    collection.save(ROOT / f'unsupported-ivs-{kind}.ttc')
